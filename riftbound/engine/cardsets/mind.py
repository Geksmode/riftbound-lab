"""Cartes du paquet mind (liste : batches/mind.txt). Voir GUIDE.md.

Cards whose behaviour needs an engine hook are not registered here: see NEEDS_mind.md.
Closures never capture game objects (AI clones deep-copy the game, not closures): objects travel through item.data,
effect dicts or uids.
"""
from cards import *  # noqa: F401,F403
from game import ANY
from actions import keyword_play_triggers, total_cost

MIND = frozenset({"Mind"})


# ====================================================================== private helpers
def _src(g, it):
    """The source permanent of a triggered/activated ability, if still on the board."""
    return g.obj(it.src)


def _here(g, it):
    """'here' of a source permanent, checked on execution (rule 359.3.f.2): its battlefield/base, or None."""
    me = g.obj(it.src)
    return None if me is None else me.loc


def _token_locs(g, pid, hb=None):
    """Valid locations to play a unit token whose effect names none (rule 355.2: base or a battlefield you
    control); a hidden spell/play effect must play it at its battlefield (rule 811.1.d.3)."""
    if hb is not None:
        return [hb]
    return ["base"] + [b.idx for b in g.bfs if b.ctrl == pid]


def _play_token(g, name, pid, loc="base", ready=False):
    """Play a token (rules 185.2.a, 350.2): it enters the board, its keyword play triggers ([Vision]) trigger and
    'played' is emitted. Tokens are not cards: n=0 and nothing is added to g.played (cards played this turn)."""
    return make_token(g, name, pid, loc, ready)       # the engine's single token-play path


def _play_unit_token(g, name, pid, ready=False, hb=None, loc=None):
    if loc is None:
        ls = token_locations(g, pid, name, _token_locs(g, pid, hb))
        if not ls:
            return None
        loc = g.ask(pid, "play_location", ls)
    return _play_token(g, name, pid, loc, ready)


def _gold(g, pid):
    """'Play a Gold gear token exhausted.' (gear are played to base)"""
    return _play_token(g, "Gold", pid, "base", ready=False)


def _is_mech(g, o):
    return "Mech" in g.tags(o)


def _minus(g, u, n, minimum=None):
    if u is not None and u in g.board:
        g.mod(u, -n, minimum=minimum)


def _rune(g, uid):
    for pl in g.p:
        for r in pl.runes:
            if r.uid == uid:
                return r
    return None


def _card_in(zone, uid, oid):
    for c in zone:
        if c.uid == uid and c.oid == oid:
            return c
    return None


def _trig(g, o, name, fn, data=None, **kw):
    g.queue_trigger(o.ctrl, name, fn, data, src=o.uid, **kw)


def _is_here(u, loc, pid):
    """u is at loc: a battlefield, or pid's own base (bases are per player)."""
    return u.loc == loc and (loc in (0, 1) or u.ctrl == pid)


def _target_here(any_unit=False):
    """trig_target for 'an enemy unit here' ('here' = the source's location, checked on execution)."""
    def opts(g, it):
        me = g.obj(it.src)
        if me is None:
            return []
        pool = all_units(g, it.ctrl) if any_unit else enemies(g, it.ctrl)
        return [u for u in pool if _is_here(u, me.loc, me.ctrl)]

    def pred(g, it, u):
        me = g.obj(it.src)
        return me is not None and _is_here(u, me.loc, me.ctrl) and (any_unit or u.ctrl != it.ctrl)
    return trig_target(opts, pred, deflect=True)


# ====================================================================== units
# Ahri, Inquisitive — "When I attack or defend, give an enemy unit here -2 might this turn, to a minimum of 1
# might."
def _ahri(g, o, ev, info):
    if ev in ("attack", "defend") and info["obj"] is o:
        def res(g_, it):
            _minus(g_, g_.legal(it, 0), 2, minimum=1)
        _trig(g, o, "Ahri, Inquisitive", res, choose=_target_here())


card("Ahri, Inquisitive", on_event=_ahri)


# Apprentice Mage — "[Empower] 2 energy. When I become [Empowered], [Predict 2]. [Empowered] I have +1 might."
def _mage(g, o, ev, info):
    if ev == "empowered" and info["obj"] is o:
        _trig(g, o, "Apprentice Mage", lambda g_, it: g_.predict(it.ctrl, 2))


card("Apprentice Mage", empower="2 energy", might_if=[(when_empowered, 1)], on_event=_mage)


# Aspiring Engineer — "When you play me, return a gear from your trash to your hand."
# The trash is public: the gear card is a target (rule 355.10.a), chosen as the trigger is finalized.
def _engineer(g, o, ctx):
    def choose(g_, it):
        gs = sorted([c for c in g_.p[it.ctrl].trash if c.spec["type"] == "Gear"],
                    key=lambda c: (-c.spec["e"] - 2 * c.spec["p"], c.cname))
        if not gs:
            return False
        c = g_.ask(it.ctrl, "trash_gear", gs, item=it)
        it.data["card"] = (c.uid, c.oid)
        return True

    def res(g_, it):
        c = _card_in(g_.p[it.ctrl].trash, *it.data["card"])
        if c is not None:
            g_.to_zone(c, "hand")
    _trig(g, o, "Aspiring Engineer", res, choose=choose)


card("Aspiring Engineer", on_play=_engineer)


# Ava Achiever — "[Hidden] When I attack, you may pay 1 mind rune to play a card with [Hidden] from your hand
# here, ignoring its cost."
# Ruling chosen: a spell played "here" makes its choices like a spell played from Hidden at this battlefield
# (rule 811.1.d.2); a unit is played to this battlefield; a gear is played to base (gear location, rule 355.2).
def _hidden_card(g, c):
    im = g.impl(c)
    return im is not None and im.hidden


def _ava_plays(g, pid, here):
    """(card, choice) pairs for Ava: base costs ignored (rule 356.1.b.1), additional costs (Deflect...) still paid."""
    out = []
    for c in sorted([c for c in g.p[pid].hand if _hidden_card(g, c)], key=lambda c: (-c.spec["e"], c.cname)):
        t = c.spec["type"]
        im = g.impl(c)
        if t == "Unit":
            chs = [dict(loc=here)]
        elif t == "Gear":
            chs = [dict(loc="base")]
        elif t == "Spell":
            chs = cap(g, (im.choices(g, pid, dict(hidden_bf=here, card=c)) if im.choices else [dict()]), 3, pid)
        else:
            chs = []
        for ch in chs:
            ch = dict(ch, free=True)
            e, reqs = total_cost(g, pid, c, ch, "hand")
            if g.can_pay(pid, e, reqs):
                out.append((c, ch))
    return out


def _ava(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        def cost(g_, it):
            if not any(_hidden_card(g_, c) for c in g_.p[it.ctrl].hand):
                return False                      # nothing to play: paying would do nothing
            return g_.pay(it.ctrl, 0, [MIND])

        def res(g_, it):
            here = _here(g_, it)
            if here not in (0, 1):
                return
            plays = _ava_plays(g_, it.ctrl, here)
            if not plays:
                return
            k = g_.ask(it.ctrl, "ava_play", list(range(len(plays))), plays=plays)
            c, ch = plays[k]
            e, reqs = total_cost(g_, it.ctrl, c, ch, "hand")
            if (e or reqs) and not g_.pay(it.ctrl, e, reqs):
                return
            res_ = play_card(g_, it.ctrl, c, "hand", ch, limited=True)
            if c.spec["type"] == "Spell" and res_ is not None:
                res_.data["hidden_bf"] = here
        _trig(g, o, "Ava Achiever", res, may=True, cost=cost)


card("Ava Achiever", hidden=True, on_event=_ava)


# Bard, Mercurial — "You may exhaust your legend as an additional cost to play me. When you play me, if you paid
# the additional cost, move any number of your units to an open battlefield."
# Open battlefield: unoccupied and uncontrolled (rule 170.11.c).
def _bard_res(g, it):
    pid = it.ctrl
    opn = [b.idx for b in g.bfs if b.ctrl is None and not g.units(loc=b.idx)]
    if not opn:
        return
    us = friends(g, pid)
    opts = []
    for bf in opn:
        base = tuple(u.uid for u in us if u.loc == "base")
        everyone = tuple(u.uid for u in us)
        for grp in (base, everyone):
            if grp and (bf, grp) not in opts:
                opts.append((bf, grp))
        for u in us[:3]:
            if (bf, (u.uid,)) not in opts:
                opts.append((bf, (u.uid,)))
        if full_choices(g, pid):                  # a human: any set of units
            from itertools import combinations
            for k in range(1, len(us) + 1):
                for grp in combinations(us, k):
                    if (bf, tuple(u.uid for u in grp)) not in opts:
                        opts.append((bf, tuple(u.uid for u in grp)))
        opts.append((bf, ()))
    bf, grp = g.ask(pid, "bard_move", opts)
    movers = [g.obj(x) for x in grp if g.obj(x) is not None and g.obj(x).ctrl == pid]
    if movers:
        g.move(movers, bf, pid)


card("Bard, Mercurial",
     as_played=lambda g, pid, c: [dict()] + ([dict(bard=True)] if not g.p[pid].legend.exhausted else []),
     pay_extra=lambda g, pid, c, ch: ch.get("bard") and setattr(g.p[pid].legend, "exhausted", True),
     on_play=lambda g, o, ctx: ctx.get("bard") and _trig(g, o, "Bard, Mercurial", _bard_res))


# Blastcone Fae — "[Hidden] When you play me, give a unit -2 might this turn, to a minimum of 1 might."
def _blastcone(g, o, ctx):
    hb = ctx.get("hidden_bf")
    _trig(g, o, "Blastcone Fae", lambda g_, it: _minus(g_, g_.legal(it, 0), 2, minimum=1),
          choose=trig_target(lambda g_, it: all_units(g_, it.ctrl, False, hb), P_unit, deflect=True))


card("Blastcone Fae", hidden=True, on_play=_blastcone)


# Breakneck Mech — "Your Mechs have [Deflect] and [Ganking]. I enter ready if you control another Mech."
card("Breakneck Mech",
     aura_kw=lambda g, src, o: {"Deflect": 1, "Ganking": 1} if (o.ctrl == src.ctrl and o.spec["type"] == "Unit"
                                                                and _is_mech(g, o)) else None,
     enter_ready=lambda g, pid, c, ch: any(_is_mech(g, u) for u in g.units(pid)))


# Bubble Bot — "When you play me, ready another friendly Mech."
def _bubble(g, o, ctx):
    me = o.uid
    _trig(g, o, "Bubble Bot", lambda g_, it: g_.legal(it, 0) is not None and g_.ready_obj(g_.legal(it, 0)),
          choose=trig_target(lambda g_, it: sorted([u for u in g_.units(it.ctrl) if _is_mech(g_, u) and u.uid != me],
                                                   key=lambda u: (not u.exhausted, -value(g_, u))),
                             lambda g_, it, u: u.ctrl == it.ctrl and _is_mech(g_, u) and u.uid != me))


card("Bubble Bot", on_play=_bubble)


# Card Sharp — "When you play me, you and each opponent may play a Gold gear token exhausted. For each opponent
# who did, you play a Gold gear token exhausted."
def _sharp(g, it):
    pid = it.ctrl
    if g.ask(pid, "may", [True, False], item=it):
        _gold(g, pid)
    opp = 1 - pid
    if g.ask(opp, "may", [True, False], item=it):
        _gold(g, opp)
        _gold(g, pid)


card("Card Sharp", on_play=lambda g, o, ctx: _trig(g, o, "Card Sharp", _sharp))


# Cloud Drake — "When you play me, draw 1." ; Lecturing Yordle — "[Tank] When you play me, draw 1."
card("Cloud Drake", on_play=lambda g, o, ctx: _trig(g, o, "Cloud Drake", lambda g_, it: g_.draw(it.ctrl, 1)))
card("Lecturing Yordle", kw={"Tank": 1},
     on_play=lambda g, o, ctx: _trig(g, o, "Lecturing Yordle", lambda g_, it: g_.draw(it.ctrl, 1)))


# Covert Informant — "[Empower] 3 energy. [Empowered] When I move, draw 1."
def _informant(g, o, ev, info):
    if ev == "move" and info["obj"] is o and o.empowered:
        _trig(g, o, "Covert Informant", lambda g_, it: g_.draw(it.ctrl, 1))


card("Covert Informant", empower="3 energy", on_event=_informant)


# Diana, Lunari — "When a showdown begins here, you may pay 1 energy. If you do, [Predict], then reveal the top
# card of your Main Deck. If it's a spell, draw it."  (cost within instructions paid at finalization, 383.3.b)
def _diana_res(g, it):
    pid = it.ctrl
    g.predict(pid, 1)
    rv = g.reveal(pid, 1)
    if rv:
        if rv[0].spec["type"] == "Spell" and g.p[pid].deck[:1] == rv:
            g.draw(pid, 1)


def _diana(g, o, ev, info):
    if ev == "showdown_start" and info["bf"] == o.loc:
        _trig(g, o, "Diana, Lunari", _diana_res, may=True, cost=may_pay(1))


card("Diana, Lunari", on_event=_diana)


# Dr. Mundo, Expert — "My Might is increased by the number of cards in your trash. At the start of your Beginning
# Phase, recycle 3 from your trash."  (recycle X from a zone: not targeted, as many as possible, rule 416.6)
def _recycle_from_trash(g, pid, n, kind):
    picked = []
    for _ in range(n):
        tr = [c for c in g.p[pid].trash if c not in picked]
        if not tr:
            break
        picked.append(g.ask(pid, kind, sorted(tr, key=lambda c: (c.spec["e"] + 2 * c.spec["p"], c.cname))))
    if picked:
        g.recycle_cards(pid, picked)
    return picked


def _mundo(g, o, ev, info):
    if ev == "beginning_start" and info["pid"] == o.ctrl:
        _trig(g, o, "Dr. Mundo, Expert", lambda g_, it: _recycle_from_trash(g_, it.ctrl, 3, "recycle_pick"))


card("Dr. Mundo, Expert", might_mod=lambda g, o: len(g.p[o.ctrl].trash), on_event=_mundo)


# Dramatic Visionary — "[Deathknell] [Predict 2]."
card("Dramatic Visionary", deathknell=lambda g, it: g.predict(it.ctrl, 2))


# Dropboarder — "When you play me, if you control two or more gear, ready me."
def _dropboarder(g, o, ctx):
    if len(g.gear(o.ctrl)) >= 2:
        def res(g_, it):
            me = _src(g_, it)
            if me is not None and len(g_.gear(it.ctrl)) >= 2:
                g_.ready_obj(me)
        _trig(g, o, "Dropboarder", res)


card("Dropboarder", on_play=_dropboarder)


# Ekko, Recurrent — "[Accelerate] [Deathknell] — Recycle me to ready your runes."
# "Recycle me" is a cost within instructions, paid as the trigger is finalized (rule 383.3.b, Ekko example).
def _ekko_cost(g, it):
    c = it.data["info"]["obj"]
    pl = g.p[c.owner]
    if c.token or c not in pl.trash:
        return False
    g.recycle_cards(c.owner, [c])
    return True


def _ekko_dk(g, it):
    for r in g.p[it.ctrl].runes:
        r.exhausted = False


card("Ekko, Recurrent", accelerate=True, deathknell=_ekko_dk, dk_choose=_ekko_cost)


# Fate Weaver — "When you play me, look at the top 4 cards of your Main Deck. You may reveal a spell with Energy
# cost 4 energy or more from among them and draw it. Recycle the rest."
def _weaver_res(g, it):
    pl = g.p[it.ctrl]
    top = g.look(it.ctrl, pl.deck[:4])
    sp = sorted([c for c in top if c.spec["type"] == "Spell" and c.spec["e"] >= 4], key=lambda c: -c.spec["e"])
    pick = g.ask(it.ctrl, "fate_pick", sp + [None]) if sp else None
    rest = [c for c in top if c is not pick]
    if pick is not None:
        g.log(f"  P{it.ctrl} reveals and draws {pick}")
        g.draw_card(it.ctrl, pick)
    for c in rest:
        pl.deck.remove(c)
    g.recycle_cards(it.ctrl, rest)


card("Fate Weaver", on_play=lambda g, o, ctx: _trig(g, o, "Fate Weaver", _weaver_res))


# Forecaster — "Your Mechs have [Vision]."
card("Forecaster",
     aura_kw=lambda g, src, o: {"Vision": 1} if (o.ctrl == src.ctrl and o.spec["type"] == "Unit" and _is_mech(g, o))
     else None)


# Frostcoat Cub — "You may pay 1 mind rune as an additional cost to play me. When you play me, if you paid the
# additional cost, give a unit -2 might this turn."
def _cub(g, o, ctx):
    if not ctx.get("cub"):
        return
    hb = ctx.get("hidden_bf")
    _trig(g, o, "Frostcoat Cub", lambda g_, it: _minus(g_, g_.legal(it, 0), 2),
          choose=trig_target(lambda g_, it: all_units(g_, it.ctrl, False, hb), P_unit, deflect=True))


card("Frostcoat Cub", as_played=lambda g, pid, c: [dict(), dict(cub=True)],
     extra_cost_fn=lambda g, pid, c, ch: (0, [MIND]) if ch.get("cub") else (0, []), on_play=_cub)


# Gearhead — "[Accelerate] Each Equipment attached to me gives double its base Might bonus."
card("Gearhead", accelerate=True,
     might_mod=lambda g, o: sum(EQUIP_BONUS.get(g.obj(x).cname, 0) for x in o.attached if g.obj(x) is not None))


# Gemcraft Seer — "[Vision] Other friendly units have [Vision]."
card("Gemcraft Seer", kw={"Vision": 1},
     aura_kw=lambda g, src, o: {"Vision": 1} if (o is not src and o.ctrl == src.ctrl and o.spec["type"] == "Unit")
     else None)


# Grumpy Rockbear — "[Empower] 12 energy. This ability costs 1 energy less for each rune you control.
# [Empowered] I have [Deflect] and [Shield 3]."
card("Grumpy Rockbear", empower=lambda g, pid, o, ch: (max(0, 12 - len(g.p[pid].runes)), []),
     kw_if=[(when_empowered, {"Deflect": 1, "Shield": 3})])


# Gustwalker — "[Hunt 2] [Level 3] I have +1 might and [Ganking]."
card("Gustwalker", kw={"Hunt": 2}, levels=[(3, dict(might=1, kw={"Ganking": 1}))])


# Hwei, Brooding Painter — "When I move, draw 1, then discard 1. Then, do the following based on the discarded
# card's type: Spell — Draw 1. Gear — Ready up to 2 runes. Unit — Give me +3 might this turn."
def _hwei_res(g, it):
    pid = it.ctrl
    g.draw(pid, 1)
    hand_ = g.p[pid].hand
    if not hand_:
        return
    c = g.ask(pid, "discard", list(hand_), reason="hwei")
    if not g.discard(pid, c):
        return
    t = c.spec["type"]
    if t == "Spell":
        g.draw(pid, 1)
    elif t == "Gear":
        ex = [r for r in g.p[pid].runes if r.exhausted]
        if full_choices(g, pid) and len(ex) > 2 and len({r.domain for r in ex}) > 1:
            for _ in range(2):                    # a human chooses which runes (by domain)
                r = g.ask(pid, "ready_rune", [r for r in g.p[pid].runes if r.exhausted])
                r.exhausted = False
        else:
            for r in ex[:2]:
                r.exhausted = False
    elif t == "Unit":
        me = _src(g, it)
        if me is not None:
            g.mod(me, 3)


def _hwei(g, o, ev, info):
    if ev == "move" and info["obj"] is o:
        _trig(g, o, "Hwei, Brooding Painter", _hwei_res)


card("Hwei, Brooding Painter", on_event=_hwei)


# Icevale Archer — "When I attack, you may pay 1 energy to give a unit here -1 might this turn."
def _archer(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        _trig(g, o, "Icevale Archer", lambda g_, it: _minus(g_, g_.legal(it, 0), 1), may=True,
              choose=_target_here(any_unit=True), cost=may_pay(1))


card("Icevale Archer", on_event=_archer)


# Jayce, Brilliant Inventor — "When you play me or the first time you play a non-token gear each turn, you may
# ready something besides me that's exhausted."
def _jayce_choose(g, it):
    pid = it.ctrl
    opts = []
    us = sorted([u for u in g.units(pid) if u.exhausted and u.uid != it.src], key=lambda u: -value(g, u))
    opts += [("obj", u.uid) for u in us]
    opts += [("obj", x.uid) for x in g.gear(pid) if x.exhausted and x.uid != it.src]
    if g.p[pid].legend.exhausted:
        opts.append(("legend", pid))
    opts += [("rune", r.uid) for r in g.p[pid].runes if r.exhausted]
    if not opts:
        return False
    pick = g.ask(pid, "ready_pick", opts, item=it)
    it.data["pick"] = pick
    if pick[0] == "obj":
        g.add_target(it, g.obj(pick[1]), lambda g_, it_, o: o.exhausted and o.uid != it_.src)
    return True


def _jayce_res(g, it):
    kind, x = it.data["pick"]
    if kind == "obj":
        o = g.legal(it, 0)
        if o is not None:
            g.ready_obj(o)
    elif kind == "legend":
        g.p[x].legend.exhausted = False
    else:
        r = _rune(g, x)
        if r is not None and r.exhausted:
            r.exhausted = False


def _jayce_event(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and info["card"].spec["type"] == "Gear" and not info["card"].token:
        if sum(1 for n in g.played[o.ctrl] if SPEC[n]["type"] == "Gear") == 1:        # first non-token gear
            _trig(g, o, "Jayce, Brilliant Inventor", _jayce_res, may=True, choose=_jayce_choose)


card("Jayce, Brilliant Inventor", on_event=_jayce_event,
     on_play=lambda g, o, ctx: _trig(g, o, "Jayce, Brilliant Inventor", _jayce_res, may=True, choose=_jayce_choose))


# Keeper of Masks — "[Hidden] [Temporary] When you play me, play two Reflection unit tokens here. They become
# copies of me."
def _keeper_res(g, it):
    here = _here(g, it)
    if here is None:
        return
    for _ in range(2):
        t = _play_token(g, "Reflection", it.ctrl, here, ready=False)
        if t is not None:
            t.copy = "Keeper of Masks"             # copyable traits incl. Might and keywords


card("Keeper of Masks", hidden=True, kw={"Temporary": 1},
     on_play=lambda g, o, ctx: _trig(g, o, "Keeper of Masks", _keeper_res))


# Lillia, Fae Fawn — "[Accelerate] When I move from a location, play a 3 might Sprite unit token with [Temporary]
# there."  ("there" is referenced from the trigger condition, rule 359.3.f.3 Lillia example)
def _lillia(g, o, ev, info):
    if ev == "move" and info["obj"] is o:
        _trig(g, o, "Lillia, Fae Fawn", lambda g_, it: _play_token(g_, "Sprite", it.ctrl, it.data["frm"]),
              dict(frm=info["frm"]))


card("Lillia, Fae Fawn", accelerate=True, on_event=_lillia)


# Lux, Illuminated — "When you play a spell that costs 5 energy or more, give me +3 might this turn."
# (printed cost, rule 206 Lux example)
def _lux(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and info["card"].spec["type"] == "Spell" and \
            not info["card"].token and info["card"].spec["e"] >= 5:
        _trig(g, o, "Lux, Illuminated", lambda g_, it: _src(g_, it) is not None and g_.mod(_src(g_, it), 3))


card("Lux, Illuminated", on_event=_lux)


# Nasus, Guardian of Knowledge — "Once each turn, when an enemy unit here dies, channel 1 rune exhausted."
def _nasus(g, o, ev, info):
    if ev == "die":
        inf = info["info"]
        if inf["spec"]["type"] == "Unit" and inf["ctrl"] != o.ctrl and inf["loc"] == o.loc and o.loc in (0, 1):
            key = ("nasus", o.uid, o.oid, g.turn_no)
            if key in g.stats:                       # rule 383.3.e
                return
            g.stats[key] = 1
            _trig(g, o, "Nasus, Guardian of Knowledge", lambda g_, it: g_.channel(it.ctrl, 1, exhausted=True))


card("Nasus, Guardian of Knowledge", on_event=_nasus)


# Ornn, Forge God — "[Deflect 2] [Weaponmaster] I have +1 might for each friendly gear."
card("Ornn, Forge God", kw={"Deflect": 2, "Weaponmaster": 1}, might_mod=lambda g, o: len(g.gear(o.ctrl)))


# Patched Porobot — "(I enter exhausted.) When you play me, if you control 3 or more other gear, draw 1."
def _porobot(g, o, ctx):
    if len(g.gear(o.ctrl)) >= 3:
        _trig(g, o, "Patched Porobot", lambda g_, it: len(g_.gear(it.ctrl)) >= 3 and g_.draw(it.ctrl, 1))


card("Patched Porobot", on_play=_porobot)


# Petal Pixie — "I have +1 might for each of your units with [Temporary] at my battlefield."
card("Petal Pixie", might_mod=lambda g, o: 0 if o.loc not in (0, 1) else
     sum(1 for u in g.units(o.ctrl, o.loc) if g.has_kw(u, "Temporary")))


# Pickpocket — "When you play me, you may kill a gear with Energy cost no more than 1 energy. If you do, play a
# Gold gear token exhausted."  (tokens cost 0, rule 185.3.a.1)
def _cheap_gear(g, it, x):
    return x.spec["type"] == "Gear" and (0 if x.token and not x.copy else x.spec["e"]) <= 1


def _pickpocket_res(g, it):
    x = g.legal(it, 0)
    if x is not None and g.kill([x], it.ctrl):
        _gold(g, it.ctrl)


def _pickpocket(g, o, ctx):
    hb = ctx.get("hidden_bf")
    _trig(g, o, "Pickpocket", _pickpocket_res, may=True,
          choose=trig_target(lambda g_, it: sorted([x for x in g_.gear() if _cheap_gear(g_, it, x)],
                                                   key=lambda x: (x.ctrl == it.ctrl, -x.spec["e"])),
                             _cheap_gear, deflect=True))


card("Pickpocket", on_play=_pickpocket)


# Pit Crew — "When you play a gear, ready me."
def _pit_crew(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and info["card"].spec["type"] == "Gear":
        _trig(g, o, "Pit Crew", lambda g_, it: _src(g_, it) is not None and g_.ready_obj(_src(g_, it)))


card("Pit Crew", on_event=_pit_crew)


# Plaza Guardian — "I cost 1 energy less for each gear you control. [Deflect]"
card("Plaza Guardian", kw={"Deflect": 1}, cost_mod=lambda g, pid, c, ch: (len(g.gear(pid)), 0))


# Plundering Poro — "When I conquer, play a Gold gear token exhausted."
def _plunder(g, o, ev, info):
    if ev == "conquer" and info["pid"] == o.ctrl and o in info["units"]:
        _trig(g, o, "Plundering Poro", lambda g_, it: _gold(g_, it.ctrl))


card("Plundering Poro", on_event=_plunder)


# Ravenbloom Student — "When you play a spell, give me +1 might this turn."
def _ravenbloom(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and info["card"].spec["type"] == "Spell":
        _trig(g, o, "Ravenbloom Student", lambda g_, it: _src(g_, it) is not None and g_.mod(_src(g_, it), 1))


card("Ravenbloom Student", on_event=_ravenbloom)


# Renata Glasc, Mastermind — "1 energy and 1 mind rune: Draw 1. 4 energy and 4 mind runes, exhaust: Score 1 point.
# Use my abilities only while I'm at a battlefield."  (a point scored by an effect, not a Conquer: no Final
# Point restriction, rule 471.1.b)
card("Renata Glasc, Mastermind", abilities=[
    ability("Draw", "1 energy and 1 mind rune", can=lambda g, pid, o: o.loc in (0, 1),
            resolve=lambda g, it: g.draw(it.ctrl, 1)),
    ability("Score", "4 energy and 4 mind runes", exhaust=True, can=lambda g, pid, o: o.loc in (0, 1),
            resolve=lambda g, it: g.gain_point(it.ctrl, "Renata Glasc, Mastermind")),
])


# Riptide Rex — "When you play me, deal 6 to an enemy unit at a battlefield."
def _rex(g, o, ctx):
    hb = ctx.get("hidden_bf")
    _trig(g, o, "Riptide Rex", lambda g_, it: g_.deal(g_.legal(it, 0), 6, "ability", it.ctrl), dict(dmg=6),
          choose=trig_target(lambda g_, it: enemies(g_, it.ctrl, True, hb), P_enemy_bf, deflect=True))


card("Riptide Rex", on_play=_rex)


# Rumble, Scrapper — "Your Mechs have +1 might (including me). When I hold, play a 3 might Mech unit token to
# your base."
def _rumble(g, o, ev, info):
    if ev == "hold" and info["pid"] == o.ctrl and o in info["units"]:
        _trig(g, o, "Rumble, Scrapper", lambda g_, it: _play_token(g_, "Mech", it.ctrl, "base"))


card("Rumble, Scrapper", on_event=_rumble,
     aura_might=lambda g, src, o: 1 if (o.ctrl == src.ctrl and o.spec["type"] == "Unit" and _is_mech(g, o)) else 0)


# Sky Cruiser — "Discard a gear, 1 energy, exhaust: Deal 4 to a unit at a battlefield."
def _cruiser_choices(g, pid, o):
    gs = sorted([c for c in g.p[pid].hand if c.spec["type"] == "Gear"], key=lambda c: (c.spec["e"], c.cname))
    names, picks = set(), []
    for c in gs:
        if c.cname not in names:
            names.add(c.cname)
            picks.append(c)
    out = []
    for u in all_units(g, pid, True):
        for c in cap(g, picks, 2, pid):
            out.append(dict(tg=(u.uid,), disc=c.uid))
    return out


def _cruiser_discard(g, pid, o, ch):
    c = next((x for x in g.p[pid].hand if x.uid == ch.get("disc")), None)
    if c is not None:
        g.discard(pid, c)


card("Sky Cruiser", abilities=[
    ability("Strike", "1 energy", exhaust=True, choices=_cruiser_choices, preds=[P_unit_bf],
            can=lambda g, pid, o: any(c.spec["type"] == "Gear" for c in g.p[pid].hand),
            extra_cost=_cruiser_discard,
            resolve=lambda g, it: g.deal(g.legal(it, 0), 4, "ability", it.ctrl))])


# Soul Shepherd — "Your token units have +1 might."
card("Soul Shepherd",
     aura_might=lambda g, src, o: 1 if (o.ctrl == src.ctrl and o.token and o.spec["type"] == "Unit") else 0)


# Spectral Centaur — "When another friendly unit dies, give me +2 might this turn."
def _centaur(g, o, ev, info):
    if ev == "die":
        inf = info["info"]
        if inf["ctrl"] == o.ctrl and inf["spec"]["type"] == "Unit" and inf["obj"] is not o:
            _trig(g, o, "Spectral Centaur", lambda g_, it: _src(g_, it) is not None and g_.mod(_src(g_, it), 2))


card("Spectral Centaur", on_event=_centaur)


# Sprite Mother — "When you play me, play a ready 3 might Sprite unit token with [Temporary] here."
def _mother_res(g, it):
    here = _here(g, it)
    if here is not None:
        _play_token(g, "Sprite", it.ctrl, here, ready=True)


card("Sprite Mother", on_play=lambda g, o, ctx: _trig(g, o, "Sprite Mother", _mother_res))


# Sprite Queen — "When you play me or at the start of your Beginning Phase, play a ready 3 might Sprite unit token
# with [Temporary] to your base."
def _queen_res(g, it):
    _play_token(g, "Sprite", it.ctrl, "base", ready=True)


card("Sprite Queen", on_play=lambda g, o, ctx: _trig(g, o, "Sprite Queen", _queen_res),
     on_event=lambda g, o, ev, info: ev == "beginning_start" and info["pid"] == o.ctrl and
     _trig(g, o, "Sprite Queen", _queen_res))


# Swain, Visionary — "[Vision] When I conquer, if you've played a non-token unit, a non-token gear, and a spell this
# turn, you score 1 point."
def _swain_ok(g, pid):
    ts = set(SPEC[n]["type"] for n in g.played[pid] if n in SPEC)
    return {"Unit", "Gear", "Spell"} <= ts


def _swain(g, o, ev, info):
    if ev == "conquer" and info["pid"] == o.ctrl and o in info["units"] and _swain_ok(g, o.ctrl):
        _trig(g, o, "Swain, Visionary",
              lambda g_, it: _swain_ok(g_, it.ctrl) and g_.gain_point(it.ctrl, "Swain, Visionary"))


card("Swain, Visionary", kw={"Vision": 1}, on_event=_swain)


# Teemo, Strategist — "[Hidden] When I defend or I'm played from [Hidden], reveal the top 5 cards of your Main Deck.
# Deal 1 to an enemy unit here for each card with [Hidden], then recycle them."
# Read as one chosen enemy unit here taking 1 per revealed Hidden card ("+1 might for each" pattern).
def _teemo_res(g, it):
    pl = g.p[it.ctrl]
    top = g.reveal(it.ctrl, 5)
    n = sum(1 for c in top if _hidden_card(g, c) or "Hidden" in keywords_of(c.cname))
    u = g.legal(it, 0)
    if u is not None and n:
        g.deal(u, n, "ability", it.ctrl)
    for c in top:
        if c in pl.deck:
            pl.deck.remove(c)
    g.recycle_cards(it.ctrl, top)


def _teemo_trigger(g, o):
    _trig(g, o, "Teemo, Strategist", _teemo_res, choose=_target_here())


card("Teemo, Strategist", hidden=True,
     on_play=lambda g, o, ctx: ctx.get("src") == "facedown" and _teemo_trigger(g, o),
     on_event=lambda g, o, ev, info: ev == "defend" and info["obj"] is o and _teemo_trigger(g, o))


# Viktor, Innovator — "When you play a card on an opponent's turn, play a 1 might Recruit unit token in your base."
def _viktor(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and g.tp != o.ctrl and not info["card"].token:
        _trig(g, o, "Viktor, Innovator", lambda g_, it: _play_token(g_, "Recruit", it.ctrl, "base"))


card("Viktor, Innovator", on_event=_viktor)


# ====================================================================== gear
# Bottled Constellation — "At the start of your Main Phase, you may kill 3 other friendly units and/or gear to
# score 1 point."  ("kill 3 ... to" is a cost within instructions, paid at finalization: rule 383.3.b)
def _constellation_cost(g, it):
    pid = it.ctrl
    cands = [o for o in g.board if o.ctrl == pid and o.uid != it.src and o.spec["type"] in ("Unit", "Gear")]
    if len(cands) < 3:
        return False
    picked = []
    for _ in range(3):
        rest = sorted([o for o in cands if o not in picked], key=lambda o: (value(g, o), o.uid))
        picked.append(g.ask(pid, "constellation_kill", rest, item=it))
    g.kill(picked, pid, cost=True)
    return True


def _constellation(g, o, ev, info):
    if ev == "main_start" and info["pid"] == o.ctrl:
        _trig(g, o, "Bottled Constellation", lambda g_, it: g_.gain_point(it.ctrl, "Bottled Constellation"),
              may=True, cost=_constellation_cost)


card("Bottled Constellation", on_event=_constellation)


# Chemtech Cask — "When you play a spell on an opponent's turn, you may exhaust me to play a Gold gear token
# exhausted."
def _cask_cost(g, it):
    me = _src(g, it)
    if me is None or me.exhausted:
        return False
    me.exhausted = True
    return True


def _cask(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and g.tp != o.ctrl and info["card"].spec["type"] == "Spell" \
            and not o.exhausted:
        _trig(g, o, "Chemtech Cask", lambda g_, it: _gold(g_, it.ctrl), may=True, cost=_cask_cost)


card("Chemtech Cask", on_event=_cask)


# Cloth Armor — "[Quick-Draw] [Equip] 1 mind rune"; Might Bonus +0 and Effect Text "[Shield 2]" (card image).
card("Cloth Armor", timing="reaction", quickdraw=True, equip="1 mind rune", bonus=0, equip_kw={"Shield": 2})


# Energy Conduit — "exhaust: [Reaction] — [Add] 1 energy."
card("Energy Conduit", add=[add_ability("1 energy")])


# Garbage Grabber — "Recycle 3 from your trash, 1 energy, exhaust: Draw 1."
card("Garbage Grabber", abilities=[
    ability("Draw", "1 energy", exhaust=True, can=lambda g, pid, o: len(g.p[pid].trash) >= 3,
            extra_cost=lambda g, pid, o, ch: _recycle_from_trash(g, pid, 3, "recycle_pick"),
            resolve=lambda g, it: g.draw(it.ctrl, 1))])


# Gutter Palace — "At the start of your Beginning Phase, if you have exactly 4 cards in hand and exactly 4 units at
# battlefields, you win the game. Discard 1, exhaust: Play a 1 might Bird unit token with [Deflect]."
def _palace_ok(g, pid):
    return len(g.p[pid].hand) == 4 and sum(1 for u in g.units(pid) if u.loc in (0, 1)) == 4


def _palace(g, o, ev, info):
    if ev == "beginning_start" and info["pid"] == o.ctrl and _palace_ok(g, o.ctrl):
        _trig(g, o, "Gutter Palace", lambda g_, it: g_.win(it.ctrl))


def _palace_discard(g, pid, o, ch):
    c = g.ask(pid, "discard", list(g.p[pid].hand), reason="gutter_palace")
    if c is not None:
        g.discard(pid, c)


card("Gutter Palace", on_event=_palace, abilities=[
    ability("Bird", None, exhaust=True, can=lambda g, pid, o: bool(g.p[pid].hand), extra_cost=_palace_discard,
            resolve=lambda g, it: _play_unit_token(g, "Bird", it.ctrl))])


# Hextech Formula — "This enters exhausted. exhaust: Empower another gear."
card("Hextech Formula", enter_exhausted=True, abilities=[
    ability("Empower", None, exhaust=True,
            choices=lambda g, pid, o: [dict(tg=(x.uid,)) for x in sorted(g.gear(pid), key=lambda x: (x.empowered,
                                                                                                        -x.spec["e"]))
                                       if x.uid != o.uid],
            preds=[lambda g, it, x: x.spec["type"] == "Gear" and x.uid != it.src],
            resolve=lambda g, it: g.empower(g.legal(it, 0)))])


# Mushroom Pouch — "At the start of your Beginning Phase, if you control a facedown card at a battlefield, draw 1."
def _pouch(g, o, ev, info):
    if ev == "beginning_start" and info["pid"] == o.ctrl and \
            any(c.owner == o.ctrl for b in g.bfs for c in b.facedowns):
        _trig(g, o, "Mushroom Pouch", lambda g_, it: g_.draw(it.ctrl, 1))


card("Mushroom Pouch", on_event=_pouch)


# Orb of Regret — "exhaust: Give a unit -1 might this turn, to a minimum of 1 might."
card("Orb of Regret", abilities=[
    ability("Regret", None, exhaust=True, choices=lambda g, pid, o: tg_choices(all_units(g, pid)), preds=[P_unit],
            resolve=lambda g, it: _minus(g, g.legal(it, 0), 1, minimum=1))])


# Questionable Tome — "[Empower] — exhaust. Disempower this, 1 energy, exhaust: Draw 1."
card("Questionable Tome", abilities=[
    empower_ability((0, []), exhaust=True),
    ability("Draw", "1 energy", exhaust=True, can=lambda g, pid, o: o.empowered,
            extra_cost=lambda g, pid, o, ch: g.disempower(o), resolve=lambda g, it: g.draw(it.ctrl, 1))])


# Seal of Insight — "exhaust: [Reaction] — [Add] 1 mind rune."
card("Seal of Insight", add=[add_ability("1 mind rune")])


# Sprite Fountain — "[Temporary] When you play this, play a ready 3 might Sprite unit token with [Temporary] to your
# base. [Deathknell] Repeat this gear's play effect."
def _fountain_res(g, it):
    _play_token(g, "Sprite", it.ctrl, "base", ready=True)


card("Sprite Fountain", kw={"Temporary": 1}, deathknell=_fountain_res,
     on_play=lambda g, o, ctx: _trig(g, o, "Sprite Fountain", _fountain_res))


# Sumpworks Map — "[Reaction] [Temporary] When an opponent scores, draw 1."
# Scoring (rule 469): a Conquer or Hold that scores, or a point scored by a card effect ("score 1 point"); gaining
# a point from an opponent's Burn Out is not scoring.
def _map(g, o, ev, info):
    opp = 1 - o.ctrl
    hit = False
    if ev in ("conquer", "hold") and info["pid"] == opp and g.can_score(opp, g.bfs[info["bf"]]):
        hit = True
    elif ev == "point" and info["pid"] == opp and info["why"] != "burn out" and \
            not str(info["why"]).startswith(("conquer", "hold")):
        hit = True
    if hit:
        _trig(g, o, "Sumpworks Map", lambda g_, it: g_.draw(it.ctrl, 1))


card("Sumpworks Map", timing="reaction", kw={"Temporary": 1}, on_event=_map)


# The Zero Drive — "[Equip] 1 energy and 1 mind rune. 3 energy and 1 mind rune, Banish this: Play all units
# banished with this, ignoring their costs. (Use only if unattached.)"; Might Bonus +2 and Effect Text
# "[Deathknell] — Banish me." (card image). The granted Deathknell is watched by an effect created when the Drive
# is attached (the unit's look-back info lists its attached gear, so it works even if the Drive left the board).
def _zero_dk_watch(g, eff, info):
    inf = info["info"]
    if eff["gear"] not in inf.get("attached", ()):
        return
    g.queue_trigger(inf["ctrl"], "Deathknell (The Zero Drive)", _zero_dk_res, dict(card=inf["obj"], gear=eff["gear"]))


def _zero_dk_res(g, it):
    c = it.data["card"]
    if c.token or c not in g.p[c.owner].trash:
        return
    g.to_zone(c, "banish")
    g.log(f"  {c} is banished with The Zero Drive")
    x = g.obj(it.data["gear"])
    if x is not None:
        x.__dict__.setdefault("zero_banished", []).append((x.oid, c))


def _zero_event(g, o, ev, info):
    if ev == "attached" and info["gear"] is o:
        key = ("zero_watch", o.uid)
        if key not in g.stats:
            g.stats[key] = 1
            g.effects.append(dict(on="die", fn=_zero_dk_watch, gear=o.uid))


def _zero_pay(g, pid, o, ch):
    ch["cards"] = [c for oid, c in getattr(o, "zero_banished", []) if oid == o.oid]
    g.to_zone(o, "banish")


def _zero_res(g, it):
    for c in it.data.get("cards", []):
        if c in g.p[c.owner].banish:
            play_unit_free(g, it.ctrl, c, "banish")


card("The Zero Drive", equip="1 energy and 1 mind rune", bonus=2, equip_kw={"Deathknell": 1}, on_event=_zero_event,
     abilities=[ability("Release", "3 energy and 1 mind rune", can=lambda g, pid, o: o.attached_to is None,
                        extra_cost=_zero_pay, resolve=_zero_res)])


# World Atlas — "[Equip] 1 mind rune"; Might Bonus +2 and Effect Text "When I hold, play two Gold gear tokens
# exhausted." (card image)
def _atlas(g, gear, unit, ev, info):
    if ev == "hold" and info["pid"] == unit.ctrl and unit in info["units"]:
        g.queue_trigger(unit.ctrl, "World Atlas", lambda g_, it: (_gold(g_, it.ctrl), _gold(g_, it.ctrl)),
                        src=unit.uid)


card("World Atlas", equip="1 mind rune", bonus=2, effect_event=_atlas)


# ====================================================================== spells
def _dmg_spell(n):
    return lambda g, it: g.deal(g.legal(it, 0), n, "spell", it.ctrl)


def _minus_spell(n, minimum=None):
    return lambda g, it: _minus(g, g.legal(it, 0), n, minimum)


def _P_perm(g, it, o):
    return o.spec["type"] in ("Unit", "Gear") and hb_ok(it, o)


# Acceleration Gate — "Ready up to 4 units, gear, and/or runes."
# Runes are chosen as targets too (rule 355.9.a): they are kept as uids in choice['runes']. Readying something
# that is not exhausted does nothing, so runes about to be exhausted to pay for the spell can be chosen.
def _gate_choices(g, pid, ctx):
    perms = sorted([u for u in g.units(pid) if u.exhausted], key=lambda u: -value(g, u)) + \
        sorted([x for x in g.gear(pid) if x.exhausted], key=lambda x: -x.spec["e"])
    rs = sorted(g.p[pid].runes, key=lambda r: (not r.exhausted, r.domain, r.uid))
    out = []
    for k in sorted({min(len(perms), n) for n in (4, 2, 0)}, reverse=True):
        ch = dict(tg=tuple(o.uid for o in perms[:k]), runes=tuple(r.uid for r in rs[:4 - k]))
        if ch not in out:
            out.append(ch)
    return out


def _gate(g, it):
    for i in range(len(it.targets)):
        o = g.legal(it, i)
        if o is not None:
            g.ready_obj(o)
    for uid in it.data.get("runes", ()):
        r = _rune(g, uid)
        if r is not None:
            r.exhausted = False


card("Acceleration Gate", choices=_gate_choices, preds=[_P_perm], resolve=_gate)


# Arcane Shift — "[Action] Banish a friendly unit, then its owner plays it, ignoring its cost. Deal 3 to an enemy
# unit at a battlefield. Banish this."
def _shift(g, it):
    u = g.legal(it, 0)
    if u is not None:
        owner, tok = u.owner, u.token
        g.to_zone(u, "banish")                       # a token ceases to exist (rule 186.1)
        if not tok and u in g.p[owner].banish:
            play_unit_free(g, owner, u, "banish")
    e = g.legal(it, 1)
    if e is not None:
        g.deal(e, 3, "spell", it.ctrl)


def _shift_choices(g, pid, ctx):
    fr = cap(g, sorted(friends(g, pid), key=lambda u: (not u.exhausted, -u.damage, -value(g, u))), 2, pid)
    es = cap(g, enemies(g, pid, True), 4, pid)
    return [dict(tg=(f.uid, e.uid)) for e in es for f in fr]


card("Arcane Shift", timing="action", banish_after=True, preds=[P_friend, P_enemy_bf], resolve=_shift,
     choices=_shift_choices)


# Clairvoyance — "[Reaction] [Predict 5]. Draw 2."
card("Clairvoyance", timing="reaction", resolve=lambda g, it: (g.predict(it.ctrl, 5), g.draw(it.ctrl, 2)))

# Consult the Past — "[Hidden] [Reaction] Draw 2."
card("Consult the Past", hidden=True, timing="reaction", resolve=lambda g, it: g.draw(it.ctrl, 2))


# Convergent Mutation — "[Reaction] Choose a friendly unit. This turn, increase its Might to the Might of another
# friendly unit."
def _mutation(g, it):
    a, b = g.legal(it, 0), g.legal(it, 1)
    if a is None or b is None or a is b:
        return
    diff = g.might(b) - g.might(a)
    if diff > 0:
        g.mod(a, diff)


def _mutation_choices(g, pid, ctx):
    fr = friends(g, pid, False, ctx["hidden_bf"])
    out = []
    for a in sorted(fr, key=lambda u: g.might(u)):
        for b in sorted(friends(g, pid), key=lambda u: -g.might(u)):
            if b is not a and g.might(b) > g.might(a):
                out.append(dict(tg=(a.uid, b.uid)))
    out.sort(key=lambda c: (not g.in_combat(g.obj(c["tg"][0])), g.might(g.obj(c["tg"][0])) - g.might(g.obj(c["tg"][1]))))
    return out


card("Convergent Mutation", timing="reaction", preds=[P_friend, lambda g, it, o: P_friend(g, it, o)],
     resolve=_mutation, choices=_mutation_choices)


# Crescent Strike — "[Action] Choose a battlefield and an enemy unit there. Deal 4 to that unit and 1 to each other
# enemy unit there."
def _crescent(g, it):
    bf = it.data["bf"]
    u = g.legal(it, 0)
    if u is not None:
        g.deal(u, 4, "spell", it.ctrl)
    for e in [x for x in g.units(1 - it.ctrl) if x.loc == bf and x.uid != it.data["tg"][0]]:
        g.deal(e, 1, "spell", it.ctrl)


def _crescent_choices(g, pid, ctx):
    out = []
    for u in enemies(g, pid, True, ctx["hidden_bf"]):
        out.append(dict(bf=u.loc, tg=(u.uid,)))
    out.sort(key=lambda c: -(len(g.units(1 - pid, c["bf"])) + value(g, g.obj(c["tg"][0])) / 4))
    return out


card("Crescent Strike", timing="action", resolve=_crescent, choices=_crescent_choices,
     preds=[lambda g, it, o: P_enemy(g, it, o) and o.loc == it.data["bf"]])


# Deadly Flourish — "Deal 3 to an enemy unit. When it dies this turn, play a Gold gear token exhausted."
def _flourish_die(g, eff, info):
    o = info["info"]["obj"]
    if o.uid == eff["uid"] and o.oid == eff["oid"] + 1:      # the same incarnation (to_zone adds 1 to oid)
        g.effects.remove(eff)
        g.queue_trigger(eff["pid"], "Deadly Flourish", lambda g_, it: _gold(g_, it.ctrl))


def _flourish(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    g.deal(u, 3, "spell", it.ctrl)
    g.effects.append(dict(on="die", fn=_flourish_die, dur="turn", uid=u.uid, oid=u.oid, pid=it.ctrl))


card("Deadly Flourish", preds=[P_enemy], resolve=_flourish,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, False, ctx["hidden_bf"])))


# Downstage Dramatics — "[Reaction] [Repeat] 2 energy. Draw 1."
card("Downstage Dramatics", timing="reaction", repeat=repeat_cost("2 energy"),
     resolve=repeatable(lambda g, it: g.draw(it.ctrl, 1)))

# Dredge Up — "Draw 1. [Flow] 2 energy"
card("Dredge Up", flow=flow_cost("2 energy"), resolve=lambda g, it: g.draw(it.ctrl, 1))


# Eclipse — "[Reaction] Give a unit -4 might this turn. [Predict]."
def _eclipse(g, it):
    _minus(g, g.legal(it, 0), 4)
    g.predict(it.ctrl, 1)


def _units_choices(g, pid, ctx, at_bf=False):
    return tg_choices(all_units(g, pid, at_bf, ctx["hidden_bf"]))


card("Eclipse", timing="reaction", preds=[P_unit], resolve=_eclipse, choices=_units_choices)

# Falling Comet — "[Action] Deal 6 to a unit at a battlefield." ; Final Spark — "[Action] Deal 8 to a unit."
card("Falling Comet", timing="action", preds=[P_unit_bf], resolve=_dmg_spell(6),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))
card("Final Spark", timing="action", preds=[P_unit], resolve=_dmg_spell(8),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, False, ctx["hidden_bf"])))

# Frigid Touch — "[Reaction] [Repeat] 2 energy. Give a unit -2 might this turn."
card("Frigid Touch", timing="reaction", repeat=repeat_cost("2 energy"), preds=[P_unit],
     resolve=repeatable(_minus_spell(2)), choices=_units_choices)


# Hostile Takeover — "[Hidden] Take control of an enemy unit at a battlefield. Ready it. Lose control of that unit
# and recall it at end of turn."  (a combat or a conquest follows from the cleanup, rule 323)
def _takeover_end(g, eff, info):
    g.effects.remove(eff)
    g.queue_trigger(eff["pid"], "Hostile Takeover (end of turn)", _takeover_back, dict(eff))


def _takeover_back(g, it):
    u = g.obj(it.data["uid"])
    if u is None or u.oid != it.data["oid"]:
        return
    if u.ctrl == it.data["pid"]:
        u.ctrl = it.data["prev"]
    g.recall(u)


def _takeover(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    prev = u.ctrl
    u.ctrl = it.ctrl
    g.log(f"  P{it.ctrl} takes control of {u}")
    g.need_cleanup = True
    g.ready_obj(u)
    g.effects.append(dict(on="end_turn", fn=_takeover_end, dur="turn", uid=u.uid, oid=u.oid, pid=it.ctrl,
                          prev=prev))


card("Hostile Takeover", hidden=True, preds=[P_enemy_bf], resolve=_takeover,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# Iterative Design — "Play a 3 might Mech unit token. [Flow] 2 energy and 1 mind rune"
card("Iterative Design", flow=flow_cost("2 energy and 1 mind rune"),
     resolve=lambda g, it: _play_unit_token(g, "Mech", it.ctrl, hb=it.data.get("hidden_bf")))


# Mesmerize — "[Reaction] Choose one — Return a friendly unit to its owner's hand. / Give an enemy unit -2 might
# this turn."
def _mesmerize(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    if it.data["mode"] == "return":
        g.to_zone(u, "hand")
    else:
        g.mod(u, -2)


def _mesmerize_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    out = [dict(mode="minus", tg=(u.uid,)) for u in enemies(g, pid, False, hb)]
    out += [dict(mode="return", tg=(u.uid,)) for u in sorted(friends(g, pid, False, hb),
                                                              key=lambda u: (-u.damage, -value(g, u)))]
    return out


card("Mesmerize", timing="reaction", resolve=_mesmerize, choices=_mesmerize_choices,
     preds=[lambda g, it, o: (P_friend(g, it, o) if it.data["mode"] == "return" else P_enemy(g, it, o))])


# Moonfall — "[Action] Choose a battlefield where you have units. You may move up to one enemy unit to that
# battlefield. Then give enemy units there -2 might this turn."
def _moonfall(g, it):
    bf = it.data["bf"]
    if not any(u.loc == bf for u in g.units(it.ctrl)):
        return                                       # the chosen battlefield is no longer a legal choice
    if it.targets:
        u = g.legal(it, 0)
        if u is not None and u.loc != bf:
            g.move([u], bf, it.ctrl)
    for e in [x for x in g.units(1 - it.ctrl) if x.loc == bf]:
        g.mod(e, -2)


def _moonfall_choices(g, pid, ctx):
    out = []
    for bf in (0, 1):
        if not any(u.loc == bf for u in g.units(pid)):
            continue
        out += cap(g, [dict(bf=bf, tg=(u.uid,)) for u in enemies(g, pid) if u.loc != bf], 3, pid)
        out.append(dict(bf=bf, tg=()))
    out.sort(key=lambda c: -(len(g.units(1 - pid, c["bf"])) + len(c["tg"])))
    return out


card("Moonfall", timing="action", preds=[P_enemy], resolve=_moonfall, choices=_moonfall_choices)

# Moonlight Affliction — "[Reaction] Give a unit -10 might this turn."
card("Moonlight Affliction", timing="reaction", preds=[P_unit], resolve=_minus_spell(10), choices=_units_choices)


# Portal Rescue — "[Action] Banish a friendly unit, then play it to base, ignoring its cost."
def _portal(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    tok = u.token
    g.to_zone(u, "banish")
    if not tok and u in g.p[u.owner].banish:
        play_card(g, it.ctrl, u, "banish", dict(loc="base", free=True), limited=True)


card("Portal Rescue", timing="action", preds=[P_friend], resolve=_portal,
     choices=lambda g, pid, ctx: tg_choices(sorted(friends(g, pid, False, ctx["hidden_bf"]),
                                                   key=lambda u: (-u.damage, not u.exhausted, -value(g, u)))))

# Premonition — "[Reaction] Draw 3." ; Progress Day — "Draw 4."
card("Premonition", timing="reaction", resolve=lambda g, it: g.draw(it.ctrl, 3))
card("Progress Day", resolve=lambda g, it: g.draw(it.ctrl, 4))


# Production Surge — "This costs 2 energy less if you control a Mech. Play a 3 might Mech unit token to your base.
# Draw 1."
card("Production Surge", cost_mod=lambda g, pid, c, ch: (2, 0) if any(_is_mech(g, u) for u in g.units(pid)) else (0, 0),
     resolve=lambda g, it: (_play_token(g, "Mech", it.ctrl, "base"), g.draw(it.ctrl, 1)))


# Promising Future — "Each player looks at the top 5 cards of their Main Deck, chooses one, then recycles the rest.
# Starting with the next player, each player plays those cards, ignoring Energy costs. (They must still pay Power
# costs.)"  A chosen card that can't be played (no legal choice, Power unpayable) stays on top of its deck.
def _pf_plays(g, pid, c):
    im = g.impl(c)
    if im is None:
        return []
    t = c.spec["type"]
    if t == "Unit":
        chs = [dict(loc=l) for l in ["base"] + [b.idx for b in g.bfs if b.ctrl == pid]]
    elif t == "Gear":
        chs = [dict(loc="base")]
    else:
        chs = cap(g, (im.choices(g, pid, dict(hidden_bf=None, card=c)) if im.choices else [dict()]), 8, pid)
    out = []
    for ch in chs:
        ch = dict(ch, ignore_energy=True, pay_power=True)
        _, reqs = total_cost(g, pid, c, dict(ch, free=False), "hand")
        if g.can_pay(pid, 0, reqs):
            out.append(ch)
    return out


def _promising(g, it):
    picks = {}
    for pid in (it.ctrl, 1 - it.ctrl):
        deck = g.p[pid].deck
        top = g.look(pid, deck[:5])
        if not top:
            continue
        c = g.ask(pid, "promising_pick",
                  sorted(top, key=lambda x: (not _pf_plays(g, pid, x), -x.spec["e"], x.cname)))
        rest = [x for x in top if x is not c]
        for x in rest:
            deck.remove(x)
        g.recycle_cards(pid, rest)
        picks[pid] = c
    for pid in (1 - it.ctrl, it.ctrl):               # starting with the next player
        c = picks.get(pid)
        if c is None or c not in g.p[pid].deck:
            continue
        plays = _pf_plays(g, pid, c)
        if not plays:
            continue
        ch = g.ask(pid, "promising_play", plays, card=c)
        play_card(g, pid, c, "deck", ch, limited=True)


card("Promising Future", resolve=_promising)


# Retreat — "[Reaction] Return a friendly unit to its owner's hand. Its owner channels 1 rune exhausted."
def _retreat(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    owner = u.owner
    g.to_zone(u, "hand")
    g.channel(owner, 1, exhausted=True)


card("Retreat", timing="reaction", preds=[P_friend], resolve=_retreat,
     choices=lambda g, pid, ctx: tg_choices(sorted(friends(g, pid, False, ctx["hidden_bf"]),
                                                   key=lambda u: (-u.damage, not g.in_combat(u), -value(g, u)))))


# Rocket Barrage — "[Repeat] 4 energy and 1 mind rune. Choose one — Deal 4 to a unit in a base. / Kill a gear."
# The mode follows from the chosen object (a unit or a gear), so each repetition can choose another mode.
def _barrage_pred(g, it, o):
    return (o.spec["type"] == "Unit" and o.loc == "base") or o.spec["type"] == "Gear"


def _barrage_one(g, it):
    o = g.legal(it, 0)
    if o is None:
        return
    if o.spec["type"] == "Unit":
        g.deal(o, 4, "spell", it.ctrl)
    else:
        g.kill([o], it.ctrl)


def _barrage_choices(g, pid, ctx):
    us = [u for u in enemies(g, pid) if u.loc == "base"]
    gs = sorted([x for x in g.gear() if x.ctrl != pid], key=lambda x: -x.spec["e"])
    return tg_choices(cap(g, us, 5, pid) + cap(g, gs, 5, pid))


card("Rocket Barrage", repeat=repeat_cost("4 energy and 1 mind rune"), preds=[_barrage_pred],
     resolve=repeatable(_barrage_one), choices=_barrage_choices)


# Shock Blast — "[Action] This costs 2 energy less if you control something that's [Empowered]. Deal 4 to a unit at
# a battlefield."
def _empowered_any(g, pid):
    return g.p[pid].legend.empowered or any(o.empowered for o in g.board if o.ctrl == pid)


card("Shock Blast", timing="action", preds=[P_unit_bf], resolve=_dmg_spell(4),
     cost_mod=lambda g, pid, c, ch: (2, 0) if _empowered_any(g, pid) else (0, 0),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# Singularity — "Deal 6 to each of up to two units."
def _singularity(g, it):
    for i in range(len(it.targets)):
        u = g.legal(it, i)
        if u is not None:
            g.deal(u, 6, "spell", it.ctrl)


def _singularity_choices(g, pid, ctx):
    # "each of up to two units": any units (a human may choose a friendly one), and zero targets (rule 355.13)
    hb = ctx["hidden_bf"]
    es = cap(g, all_units(g, pid, False, hb) if full_choices(g, pid) else enemies(g, pid, False, hb), 4, pid)
    out = [dict(tg=(a.uid, b.uid)) for i, a in enumerate(es) for b in es[i + 1:]] + [dict(tg=(a.uid,)) for a in es]

    def score(c):
        return -sum(value(g, g.obj(u)) * (g.obj(u).ctrl != pid) * (g.might(g.obj(u)) - g.obj(u).damage <= 6)
                    for u in c["tg"])
    out.sort(key=score)
    return out + [dict(tg=())]


card("Singularity", preds=[P_unit], resolve=_singularity, choices=_singularity_choices, max_choices=10)


# Siphon Power — "[Reaction] Choose a battlefield. Give friendly units there +1 might this turn and enemy units
# there -1 might this turn, to a minimum of 1 might."
def _siphon(g, it):
    bf = it.data["bf"]
    for u in [x for x in g.units() if x.loc == bf]:
        if u.ctrl == it.ctrl:
            g.mod(u, 1)
        else:
            g.mod(u, -1, minimum=1)


card("Siphon Power", timing="reaction", resolve=_siphon,
     choices=lambda g, pid, ctx: sorted([dict(bf=b.idx) for b in g.bfs if g.units(loc=b.idx)],
                                        key=lambda c: (g.sd is None or g.sd.bf != c["bf"], -len(g.units(loc=c["bf"])))))

# Smoke Screen — "[Reaction] Give a unit -4 might this turn, to a minimum of 1 might."
card("Smoke Screen", timing="reaction", preds=[P_unit], resolve=_minus_spell(4, 1), choices=_units_choices)


# Smoke and Mirrors — "[Hidden] [Action] Choose a unit you control and another unit you control at a different
# location. If at least one of them has [Temporary], move each to the other's location. Draw 1."
# Played from Hidden, only the first unit must be at that battlefield (rule 811.1.d.2.a example).
def _smoke_mirrors(g, it):
    a, b = g.legal(it, 0), g.legal(it, 1)
    if a is not None and b is not None and a.loc != b.loc and (g.has_kw(a, "Temporary") or g.has_kw(b, "Temporary")):
        la, lb = a.loc, b.loc
        g.move([a], lb, it.ctrl)
        g.move([b], la, it.ctrl)
    g.draw(it.ctrl, 1)


def _smoke_mirrors_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    out = []
    for a in friends(g, pid, False, hb):
        for b in friends(g, pid):
            if b is not a and b.loc != a.loc:
                out.append(dict(tg=(a.uid, b.uid)))
    out.sort(key=lambda c: not any(g.has_kw(g.obj(u), "Temporary") for u in c["tg"]))
    return out


card("Smoke and Mirrors", hidden=True, timing="action", resolve=_smoke_mirrors, choices=_smoke_mirrors_choices,
     preds=[P_friend, lambda g, it, o: o.spec["type"] == "Unit" and o.ctrl == it.ctrl])


# Sprite Burst — "Play two ready 3 might Sprite unit tokens with [Temporary]."
card("Sprite Burst", resolve=lambda g, it: [_play_unit_token(g, "Sprite", it.ctrl, True, it.data.get("hidden_bf"))
                                            for _ in range(2)])

# Sprite Call — "[Hidden] [Action] Play a ready 3 might Sprite unit token with [Temporary]."
card("Sprite Call", hidden=True, timing="action",
     resolve=lambda g, it: _play_unit_token(g, "Sprite", it.ctrl, True, it.data.get("hidden_bf")))


# Temporal Breach — "[Hidden] Banish a unit, then its owner plays it to the same location, ignoring its cost."
def _breach(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    loc, owner, tok = u.loc, u.owner, u.token
    g.to_zone(u, "banish")
    if not tok and u in g.p[owner].banish:
        play_card(g, owner, u, "banish", dict(loc=loc, free=True), limited=True)


card("Temporal Breach", hidden=True, preds=[P_unit], resolve=_breach,
     choices=lambda g, pid, ctx: tg_choices(sorted(friends(g, pid, False, ctx["hidden_bf"]),
                                                   key=lambda u: (-u.damage, not u.exhausted, -value(g, u)))
                                            + enemies(g, pid, False, ctx["hidden_bf"])))


# Unchecked Power — "Exhaust all friendly units, then deal 12 to ALL units at battlefields."
def _unchecked(g, it):
    for u in g.units(it.ctrl):
        u.exhausted = True
    for u in [x for x in g.units() if x.loc in (0, 1)]:
        g.deal(u, 12, "spell", it.ctrl)


card("Unchecked Power", resolve=_unchecked)


# Wages of Pain — "[Hidden] [Action] Deal 3 to a unit at a battlefield. Play a Gold gear token exhausted."
card("Wages of Pain", hidden=True, timing="action", preds=[P_unit_bf],
     resolve=lambda g, it: (g.deal(g.legal(it, 0), 3, "spell", it.ctrl), _gold(g, it.ctrl)),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# ====================================================================== cards using the engine hooks (integration)
from actions import spell_variants, pay_ctx, abilities_of     # noqa: E402

ask_text(jayce_kill_gear="Jayce : quelle gear alliée tuer ?",
         guerilla_return="Guerilla Warfare : quelle carte [Hidden] reprendre ?",
         zilean_copy="Zilean : jouer une copie supplémentaire du jeton ?",
         kaisa_spell="Kai'Sa : quel sort jouer depuis ta défausse ?",
         rebuttal_pay="Rebuttal : payer 1 rune pour prendre le contrôle du sort (sinon il est contré) ?")


def _play_spell_limited(g, pid, c, src, flags, extra=None):
    """Play the spell c from src as a limited play (rule 419.3) with cost flags (ignore_energy...): its choices and
    optional additional costs (Repeat) are offered; the cost left (Power, additional costs) is paid."""
    from actions import play_forbidden
    im = g.impl(c)
    if im is None or play_forbidden(g, pid, c, src):
        return None
    ctx = dict(hidden_bf=None, card=c, src=src)
    if full_choices(g, pid) and im.all_choices is not None:
        base = im.all_choices(g, pid, ctx) or [dict()]
    else:
        base = im.choices(g, pid, ctx) if im.choices else [dict()]
    out = []
    for ch in spell_variants(g, pid, c, im, cap(g, base, 8, pid), ctx):
        ch = dict(ch, **flags)
        e, reqs = total_cost(g, pid, c, ch, src)
        if g.can_pay(pid, e, reqs, pay_ctx(c)):
            out.append((ch, e, reqs))
    if not out:
        return None
    ch = g.ask(pid, "play_choice", [x[0] for x in out], card=c) if len(out) > 1 else out[0][0]
    _, e, reqs = next(x for x in out if x[0] is ch)
    ch = dict(ch, **(extra or {}))
    if e or reqs:
        ch["pay_cost"] = (e, tuple(reqs))
    return play_card(g, pid, c, src, ch, limited=True)


# Applied Researchers — "[Empower] 3 energy. [Empowered][>] Your spells cost 1 energy and 1 rune of any type less, to
# a minimum of 1 energy." (rule 356.4.e: the minimum applies to this discount)
card("Applied Researchers", abilities=[empower_ability("3 energy")],
     cost_aura=lambda g, src, pid, what: [dict(e=-1, rm=1, min_e=1)]
     if src.empowered and pid == src.ctrl and what["kind"] == "spell" and what["card"] is not None else [])


# Eager Apprentice — "While I'm at a battlefield, the Energy costs for spells you play is reduced by 1 energy, to a
# minimum of 1 energy."
card("Eager Apprentice",
     cost_aura=lambda g, src, pid, what: [dict(e=-1, min_e=1)]
     if src.loc in (0, 1) and pid == src.ctrl and what["kind"] == "spell" and what["card"] is not None else [])


# Jayce, Man of Progress — "When you play me, you may kill a friendly gear. If you do, you may play a gear with Energy
# cost no more than 7 energy from hand this turn, ignoring its Energy cost." (a play permission, consumed when used)
def _jayce_perm(g, eff, card, src):
    return src == "hand" and card.spec["type"] == "Gear" and card.spec["e"] <= 7


def _mop_res(g, it):
    gs = sorted(g.gear(it.ctrl), key=lambda x: (x.spec["e"] + 2 * x.spec["p"], x.uid))
    if not gs:
        return
    x = g.ask(it.ctrl, "jayce_kill_gear", gs) if len(gs) > 1 else gs[0]
    if g.kill([x], it.ctrl):
        g.effects.append(dict(kind="play_perm", pid=it.ctrl, key=f"jayce-{it.id}", fn=_jayce_perm,
                              choice=dict(ignore_energy=True), dur="turn"))


card("Jayce, Man of Progress", on_play=lambda g, o, ctx: g.gear(o.ctrl) and _trig(
    g, o, "Jayce, Man of Progress", _mop_res, may=True))


# Temporal Portal — "1 rune of any type, [E]: Give the next spell you play this turn [Repeat] equal to its cost."
# (its cost: the spell's printed Energy and Power cost)
def _portal_cost(g, card):
    return card.spec["e"], g.power_reqs(card.spec["domains"], card.spec["p"])


card("Temporal Portal", abilities=[ability(
    "Repeat", "1 rune of any type", exhaust=True,
    resolve=lambda g, it: g.effects.append(dict(kind="grant_repeat", pid=it.ctrl, cost=_portal_cost, next=True,
                                                key=f"portal-{it.id}", dur="turn")))])


# Jhin, Meticulous Killer — "[Vision] If you've spent 4 energy or more to play a spell this turn, you may play me for
# 1 mind rune." (an alternative cost, rule 356.1.a)
def _mk_alt(g, pid, c, src):
    if src == "trash" and not g.effects_of("play_from_trash", pid):
        return []
    return [dict(key="jhin", e=0, reqs=[MIND])] if any(e >= 4 for e in g.hist["spell_e"][pid]) else []


card("Jhin, Meticulous Killer", kw={"Vision": 1}, alt_costs=_mk_alt)


# Guerilla Warfare — "Return up to two cards with [Hidden] from your trash to your hand. You can hide cards ignoring
# costs this turn."
def _guerilla(g, it):
    pid = it.ctrl
    for _ in range(2):
        cs = sorted([c for c in g.p[pid].trash if "Hidden" in keywords_of(c.cname)],
                    key=lambda c: (-(c.spec["e"] + 2 * c.spec["p"]), c.uid))
        if not cs:
            break
        c = g.ask(pid, "guerilla_return", cs + [None])
        if c is None:
            break
        g.to_zone(c, "hand")
    g.effects.append(dict(kind="hide_free", pid=pid, dur="turn"))


card("Guerilla Warfare", resolve=_guerilla)


# Frigid Jewel — "When you draw your second card each turn, give a friendly unit +2 might this turn." (draw event n=
# the number of cards that player drew this turn)
card("Frigid Jewel", on_event=lambda g, o, ev, info: ev == "draw" and info["pid"] == o.ctrl and info["n"] == 2 and _trig(
    g, o, "Frigid Jewel", lambda g_, it: g_.legal(it, 0) is not None and g_.mod(g_.legal(it, 0), 2),
    choose=trig_target(lambda g_, it: sorted(g_.units(it.ctrl), key=lambda u: (u.loc not in (0, 1), -value(g_, u))),
                       P_friend, kind="buff_target")))


# Wraith of Echoes — "The first time a friendly unit dies each turn, draw 1." (Game.hist['died'] counts the deaths
# made before it entered the board; units dying together: the first one)
def _wraith(g, o, ev, info):
    if ev != "die":
        return
    d = info["info"]
    if d["ctrl"] != o.ctrl or d["spec"]["type"] != "Unit" or d["obj"] is o:
        return
    if sum(1 for x in g.hist["died"] if x["ctrl"] == o.ctrl and x["spec"]["type"] == "Unit") == 1:
        _trig(g, o, "Wraith of Echoes", lambda g_, it: g_.draw(it.ctrl, 1))


card("Wraith of Echoes", on_event=_wraith)


# Ezreal, Dashing — "When I attack or defend, deal damage equal to my Might to an enemy unit here. I don't deal
# combat damage. 1 mind rune: [Action] — Move me to your base."
def _dashing_res(g, it):
    me, u = _src(g, it), g.legal(it, 0)
    if me is not None and u is not None:
        g.deal(u, max(0, g.might(me)), "ability", it.ctrl)


def _dashing_back(g, it):
    me = _src(g, it)
    if me is not None and me.loc != "base":
        g.move([me], "base", it.ctrl)


card("Ezreal, Dashing", no_combat_damage=lambda g, o: True,
     on_event=lambda g, o, ev, info: ev in ("attack", "defend") and info["obj"] is o and _trig(
         g, o, "Ezreal, Dashing", _dashing_res, choose=_target_here()),
     abilities=[ability("Move to base", "1 mind rune", timing="action", resolve=_dashing_back,
                        can=lambda g, pid, o: o.loc != "base")])


# Otterpus — "If a player would score 1 point from conquering or holding during their first or second turn, they draw
# 1 instead." (a replacement of the point, rule 370)
def _otterpus(g, src, pid, b, why):
    if why in ("conquer", "hold") and g.tp == pid and g.p[pid].turns <= 2:
        g.log(f"  Otterpus: P{pid} draws instead of scoring")
        g.draw(pid, 1)
        return True
    return False


card("Otterpus", point_rep=_otterpus)


# Blue Sentinel — "[Shield 2] Your hold effects for holding here trigger an additional time. When I hold, [Add] 1
# rune of any type at the start of your next Main Phase."
def _sentinel_main(g, eff, info):
    if info["pid"] != eff["pid"]:
        return
    g.effects.remove(eff)
    g.p[eff["pid"]].pool_p["A"] += 1
    g.log(f"  Blue Sentinel: P{eff['pid']} adds 1 rune of any type")


card("Blue Sentinel", kw={"Shield": 2},
     scoring_extra=lambda g, src, pid, bf, ev: 1 if ev == "hold" and pid == src.ctrl and src.loc == bf else 0,
     on_event=lambda g, o, ev, info: ev == "hold" and o in info["units"] and _trig(
         g, o, "Blue Sentinel", lambda g_, it: g_.effects.append(dict(on="main_start", fn=_sentinel_main,
                                                                      pid=it.ctrl))))


# Zilean, Time Mage — "Once each turn, if you would play a token unit while I'm at a battlefield, you may play that
# token and an additional copy of it instead." (make_token)
def _zilean(g, src, pid, name):
    if pid != src.ctrl or src.loc not in (0, 1):
        return False
    if any(e.get("kind") == "zilean_used" and e["uid"] == src.uid and e["oid"] == src.oid for e in g.effects):
        return False
    if not g.ask(pid, "zilean_copy", [True, False], card=name):
        return False
    g.effects.append(dict(kind="zilean_used", uid=src.uid, oid=src.oid, dur="turn"))
    return True


card("Zilean, Time Mage", token_rep=_zilean)


# Prize of Progress — "When you use an activated ability of a gear, give me +1 might this turn." ([Add] abilities
# used while paying included: event 'activated')
card("Prize of Progress", on_event=lambda g, o, ev, info: ev == "activated" and info["pid"] == o.ctrl
     and info.get("obj") is not None and getattr(info["obj"], "spec", None) is not None
     and info["obj"].spec["type"] == "Gear" and _trig(
         g, o, "Prize of Progress", lambda g_, it: _src(g_, it) is not None and g_.mod(_src(g_, it), 1)))


# Rebuttal — "[Reaction] Choose a spell with Energy cost no more than 4 energy. You may pay 1 rune of any type. If you
# do, gain control of it and you may make new choices for it. Otherwise, counter it."
def _rebuttal(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is None or tgt.kind != "spell":
        return
    if g.can_pay(it.ctrl, 0, [ANY]) and g.ask(it.ctrl, "rebuttal_pay", [True, False], item=tgt) \
            and g.pay(it.ctrl, 0, [ANY]):
        gain_control_item(g, tgt, it.ctrl)
        remake_choices(g, tgt, it.ctrl)
    else:
        g.counter(tgt)


card("Rebuttal", timing="reaction", resolve=_rebuttal,
     choices=lambda g, pid, ctx: [dict(item=i.id) for i in sorted(
         [i for i in reversed(g.chain) if i.kind == "spell" and i.card.spec["e"] <= 4],
         key=lambda i: i.ctrl == pid)])


# Kai'Sa, Evolutionary — "[Ganking] When I conquer, you may play a spell from your trash with Energy cost less than
# your points without paying its Energy cost. Then recycle it." ("Then recycle it": wherever it leaves the chain,
# resolved or countered: item.data['after'] = 'recycle')
def _kaisa_res(g, it):
    pid = it.ctrl
    cs = sorted([c for c in g.p[pid].trash if c.spec["type"] == "Spell" and c.spec["e"] < g.p[pid].points],
                key=lambda c: (-c.spec["e"], c.uid))
    while cs:
        c = g.ask(pid, "kaisa_spell", cs + [None])
        if c is None:
            return
        if _play_spell_limited(g, pid, c, "trash", dict(ignore_energy=True), dict(after="recycle")) is not None:
            return
        cs.remove(c)


card("Kai'Sa, Evolutionary", kw={"Ganking": 1},
     on_event=lambda g, o, ev, info: ev == "conquer" and o in info["units"] and _trig(
         g, o, "Kai'Sa, Evolutionary", _kaisa_res))


# Mel, Newly Awakened — "When you play me, draw 1. [Empower] 3 energy. [Empowered][>] Your spells and abilities can't
# be countered. If a spell or ability you control would give -might to a unit it chooses, it gives an additional -1
# might."
card("Mel, Newly Awakened", abilities=[empower_ability("3 energy")],
     on_play=lambda g, o, ctx: _trig(g, o, "Mel, Newly Awakened", lambda g_, it: g_.draw(it.ctrl, 1)),
     no_counter=lambda g, src, item: src.empowered and item.ctrl == src.ctrl,
     minus_extra=lambda g, src, item, o: 1 if src.empowered and item is not None and item.ctrl == src.ctrl else 0)


# Heimerdinger, Inventor — "I have all exhaust abilities of all friendly legends, units, and gear." (activated
# abilities with [E] in their cost, and [Add] abilities with [E]: Gold's "Kill this, [E]: [Add] [A]" included; the
# ability's "me"/"this" is Heimerdinger, rule 376)
def _heimer_others(g, src):
    pid = src.ctrl
    out = [(g.p[pid].legend, g.impl(g.p[pid].legend_name))]
    for x in g.board:
        if x is not src and x.ctrl == pid and x.spec["type"] in ("Unit", "Gear") and x.attached_to is None:
            out.append((x, g.impl(x)))
    return out


def _heimer_abilities(g, src, obj):
    if obj is not src:
        return ()
    out, seen = [], set()
    for x, im in _heimer_others(g, src):
        abs_ = list(im.abilities) if im is not None else []
        abs_ += [a for a, _ in x.abs]
        if x.zone == "board" and x.attached:
            for n in g.copied_texts(x):
                ic = g.impl(n)
                if ic is not None:
                    abs_ += list(ic.abilities)
        for a in abs_:
            if a.get("exhaust") and id(a) not in seen:
                seen.add(id(a))
                out.append(a)
    return out


_HEIMER_GOLD = dict(p=["A"], exhaust=True, kill=True)


def _heimer_adds(g, src):
    out, seen = [], set()
    for x, im in _heimer_others(g, src):
        if x.cname in ("Gold", "Gold // Buff") and id(_HEIMER_GOLD) not in seen:
            seen.add(id(_HEIMER_GOLD))
            out.append(_HEIMER_GOLD)
        for ad in (im.add if im is not None else ()):
            if ad.get("exhaust", True) and id(ad) not in seen:
                seen.add(id(ad))
                out.append(ad)
    return out


card("Heimerdinger, Inventor", grant_abilities=_heimer_abilities, grant_add=_heimer_adds)


# Hextech Anomaly — "[E]: [Reaction] — Pay any amount of 1 rune of any type to [Add] that much Energy."
card("Hextech Anomaly", add=[dict(conv="p2e", var=True, exhaust=True)])


# Malzahar, Fanatic — "Kill a friendly unit or gear, [E]: [Action] — [Add] 2 runes of any type."
def _malz_victims(g, pid, o):
    return sorted([x for x in g.board if x.ctrl == pid and x is not o and x.spec["type"] in ("Unit", "Gear")],
                  key=lambda x: (x.spec["e"] + 2 * x.spec["p"], x.uid))


card("Malzahar, Fanatic", add=[dict(conv="kill", p=["A", "A"], victims=_malz_victims, exhaust=True,
                                    timing="action")])


# Experimental Hexplate — "[Equip] 1 mind rune". Might Bonus +1, Effect Text (card image): "I am a Mech."
card("Experimental Hexplate", equip="1 mind rune", bonus=1, equip_tags={"Mech"})
