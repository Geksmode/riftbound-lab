"""Batch body: Body cards (list: batches/body.txt). See GUIDE.md.

Cards that need an engine hook are not registered here: see NEEDS_body.md.
Conventions of this module:
- "X and Y deal damage equal to their Mights to each other" (Challenge...): the damage is dealt by the units, not by
  the spell (rule 417.6.b.3), so it is dealt with kind "unit" (no Void Gate bonus, which is for spells/abilities)
  and each unit's controller is responsible for its damage (417.6.b.4). Both Mights are read before any damage
  is dealt (simultaneous damage, 417.1.d).
- Choices of triggered abilities are made at finalization (rule 383.3), paying [Deflect] for enemy units (809).
- Costs within instructions of triggered abilities are paid on finalization, of spells on resolution (740.4.a).
"""
from cards import *  # noqa: F401,F403
from game import ANY
from actions import total_cost as _total_cost

_BODY = frozenset({"Body"})


# ====================================================================== helpers
def _src(g, it):
    """The permanent that created a triggered/activated item, if it is still the same object on the board."""
    o = g.obj(it.src) if it.src is not None else None
    if o is None:
        return None
    oid = it.data.get("oid")
    if oid is not None and o.oid != oid:
        return None
    return o


def _trig(g, o, name, fn, data=None, **kw):
    d = dict(data or {})
    d["oid"] = o.oid
    g.queue_trigger(o.ctrl, name, fn, d, src=o.uid, **kw)
    return True


def _payable(g, it, o):
    """Deflect of an enemy object can be paid by the controller of the item (rule 809)."""
    return g.can_pay(it.ctrl, 0, deflect_reqs(g, it.ctrl, dict(tg=(o.uid,))))


def _pick(options_fn, pred=None, n=1, kind="choose", deflect=True):
    """Finalization chooser for a triggered ability choosing up to n objects (n=1: exactly one).
    Enemy objects with [Deflect] must be paid for (rule 809). Returns False (trigger removed) when nothing was
    chosen."""
    def choose(g, it):
        opts = list(options_fn(g, it))
        chosen = []
        for _ in range(n):
            cand = [o for o in opts if o not in chosen and (not deflect or _payable(g, it, o))]
            if not cand:
                break
            if chosen or n > 1:
                o = g.ask(it.ctrl, kind, cand + [None], item=it)
            else:
                o = g.ask(it.ctrl, kind, cand, item=it)
            if o is None:
                break
            if deflect and not pay_deflect(g, it, o):
                break
            g.add_target(it, o, pred)
            chosen.append(o)
        return bool(chosen)
    return choose


def _targets(g, it):
    out = []
    for i in range(len(it.targets)):
        o = g.legal(it, i)
        if o is not None:
            out.append(o)
    return out


def _fight(g, a, b):
    """a and b deal damage equal to their Mights to each other (simultaneously, dealt by the units)."""
    if a is None or b is None or a is b or a not in g.board or b not in g.board:
        return
    ma, mb = max(0, g.might(a)), max(0, g.might(b))
    g.deal(b, ma, "unit", a.ctrl)
    g.deal(a, mb, "unit", b.ctrl)


def _fight_score(g, f, e, bonus=0):
    """AI ordering of (friendly, enemy) fights: kill the enemy, keep the friendly unit."""
    mf, me = g.might(f) + bonus, g.might(e)
    s = 0
    if mf > 0 and mf >= me - e.damage:
        s += 10 + value(g, e)
    if me > 0 and me >= mf - f.damage:
        s -= 5 + value(g, f)
    return -s


def _duel_choices(g, pid, frs, ens, bonus=0, extra=None):
    out = []
    for f in frs[:4]:
        for e in ens[:4]:
            d = dict(tg=(f.uid, e.uid))
            if extra:
                d.update(extra)
            out.append((_fight_score(g, f, e, bonus), len(out), d))
    out.sort(key=lambda x: (x[0], x[1]))
    return [d for _, _, d in out]


def _spend_buff(g, o):
    """Spend a buff (rule 745): the buff counter of a unit the player controls is removed."""
    if o is not None and o.buff > 0:
        o.buff = 0
        return True
    return False


def _dests(u):
    return [d for d in ("base", 0, 1) if d != u.loc]


def _unit_play_choices(g, pid, c, locs):
    """Play choices of a unit played by an effect (locations x optional additional costs)."""
    im = g.impl(c)
    extra = im.as_played(g, pid, c) if im.as_played else [{}]
    out = []
    for loc in locs:
        for ex in extra:
            for acc in ([False, True] if im.accelerate else [False]):
                out.append(dict(ex, loc=loc, acc=acc))
    return out


def _reduced_cost(g, pid, c, ch, src, de):
    e, reqs = _total_cost(g, pid, c, ch, src)
    return max(0, e - de), reqs


def _play_reduced(g, pid, c, src, ch, de):
    """Play a card from src paying its cost reduced by de energy (Here to Help, Wild Claw)."""
    e, reqs = _reduced_cost(g, pid, c, ch, src, de)
    if not g.pay(pid, e, reqs, dict(kind=c.spec["type"].lower(), card=c)):
        return None
    g.effects = [ef for ef in g.effects if not (ef.get("kind") == "heron" and ef["pid"] == pid)]
    return play_card(g, pid, c, src, ch, limited=True)


def _ctrl_locs(g, pid):
    return ["base"] + [b.idx for b in g.bfs if b.ctrl == pid]


def _first_each_turn(o, key, g):
    """True the first time this is called for (object incarnation, key) in a turn."""
    attr = "_body_" + key
    mark = (g.turn_no, o.oid)
    if getattr(o, attr, None) == mark:
        return False
    setattr(o, attr, mark)
    return True


def _ready_runes(g, pid, n):
    ex = [r for r in g.p[pid].runes if r.exhausted][:n]
    for r in ex:
        r.exhausted = False
    return len(ex)


def _won(g, o, info):
    """o wins the combat that just had a result (it remains at the battlefield, rule 466.3)."""
    return info["pid"] == o.ctrl and o.loc == info["bf"] and o.desig is not None


# ====================================================================== units
# Anivia, Primal — "When I attack, deal 3 to all enemy units here."
def _anivia(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        def res(g_, it):
            me = _src(g_, it)
            here = me.loc if me is not None else it.data["loc"]
            for u in list(g_.units(1 - it.ctrl, here)):
                g_.deal(u, 3, "ability", it.ctrl)
        _trig(g, o, "Anivia, Primal", res, dict(loc=o.loc))


card("Anivia, Primal", on_event=_anivia)


# Baccai Witherclaw — "[Empower] [1][A][A] [Empowered] I have +2 might. [Empowered][Deathknell] Channel 2 runes
# exhausted." The Deathknell is a dependent ability: it triggers only if I died while Empowered (look-back, 808).
card("Baccai Witherclaw", empower="1 energy and 2 runes of any type", might_if=[(when_empowered, 2)],
     deathknell=lambda g, it: g.channel(it.ctrl, 2, exhausted=True),
     dk_choose=lambda g, it: bool(it.data["info"]["empowered"]))


# Bilgewater Bully — "While I'm buffed, I have [Ganking]."
card("Bilgewater Bully", kw_if=[(lambda g, o: o.buff > 0, {"Ganking": 1})])


# Brutal Hunter — "[Empower] [3] [Empowered] I have +2 might and [Ganking]."
card("Brutal Hunter", empower="3 energy", might_if=[(when_empowered, 2)], kw_if=[(when_empowered, {"Ganking": 1})])


# Buhru Captain — "When you play me, you may draw 1 or buff me."
def _buhru(g, o, ctx):
    def res(g_, it):
        me = _src(g_, it)
        opts = (["buff", "draw"] if me is not None and me.buff == 0 else ["draw", "buff"]) + [None]
        c = g_.ask(it.ctrl, "buhru_mode", opts, item=it)
        if c == "draw":
            g_.draw(it.ctrl, 1)
        elif c == "buff" and me is not None:
            g_.buff(me)
    _trig(g, o, "Buhru Captain", res)


card("Buhru Captain", on_play=_buhru)


# Carnivorous Snapvine — "When you play me, choose an enemy unit at a battlefield. We deal damage equal to our
# Mights to each other."
def _snapvine(g, o, ctx):
    hb = ctx.get("hidden_bf")
    _trig(g, o, "Carnivorous Snapvine", lambda g_, it: _fight(g_, _src(g_, it), g_.legal(it, 0)),
          choose=_pick(lambda g_, it: enemies(g_, it.ctrl, True, hb), P_enemy_bf, kind="target"))


card("Carnivorous Snapvine", on_play=_snapvine)


# Cithria of Cloudfield — "When you play another unit, buff me."
def _cithria(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and info["card"] is not o and info["card"].spec["type"] == "Unit":
        _trig(g, o, "Cithria of Cloudfield", lambda g_, it: g_.buff(_src(g_, it)))


card("Cithria of Cloudfield", on_event=_cithria)


# Corrupted Dragon — "If your score is not within 3 points of the Victory Score, I enter ready. When I attack, you
# may move any number of enemy units here each with 5 might or less to their base."
def _cdragon(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc

        def pred(g_, it, u):
            return u.ctrl != it.ctrl and u.loc == here and g_.might(u) <= 5

        def opts(g_, it):
            return [u for u in enemies(g_, it.ctrl) if u.loc == here and g_.might(u) <= 5]

        def res(g_, it):
            for u in _targets(g_, it):
                g_.move([u], "base", it.ctrl)
        _trig(g, o, "Corrupted Dragon", res, choose=_pick(opts, pred, n=99, kind="corrupted_dragon"))


card("Corrupted Dragon", on_event=_cdragon,
     enter_ready=lambda g, pid, c, ch: g.victory - g.p[pid].points > 3)


# Crackshot Corsair — "When I attack, deal 1 to an enemy unit here."
def _corsair(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc
        _trig(g, o, "Crackshot Corsair", lambda g_, it: g_.deal(g_.legal(it, 0), 1, "ability", it.ctrl),
              dict(dmg=1),
              choose=_pick(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                           lambda g_, it, u: u.ctrl != it.ctrl and u.loc == here, kind="target"))


card("Crackshot Corsair", on_event=_corsair)


# Crowd Favorite — "[Hunt] Spend 2 XP: [Buff] me."
card("Crowd Favorite", kw={"Hunt": 1},
     abilities=[ability("Buff", xp=2, resolve=lambda g, it: g.buff(g.obj(it.src)))])


# Dame the Despoiler — "[Empower] [5][Body] [Empowered] When I attack or defend, choose a unit here. Increase my
# Might to its Might this turn, then give me +1 might this turn."
def _dame(g, o, ev, info):
    if ev in ("attack", "defend") and info["obj"] is o and o.empowered:
        here = o.loc

        def res(g_, it):
            me = _src(g_, it)
            if me is None:
                return
            t = g_.legal(it, 0)
            if t is not None:
                diff = g_.might(t) - g_.might(me)
                if diff > 0:
                    g_.mod(me, diff)
            g_.mod(me, 1)

        def opts(g_, it):
            return sorted([u for u in g_.units(loc=here) if u.uid != it.src and
                           (u.ctrl == it.ctrl or g_.targetable(u, it.ctrl))], key=lambda u: -g_.might(u))
        _trig(g, o, "Dame the Despoiler", res,
              choose=_pick(opts, lambda g_, it, u: u.loc == here, kind="dame_unit"))


card("Dame the Despoiler", empower="5 energy and 1 body rune", on_event=_dame)


# Dazzling Aurora — "At the end of your turn, reveal cards from the top of your Main Deck until you reveal a unit.
# Play it, ignoring its cost, and recycle the rest." (Revealing never burns out, rule 431.1.c.)
def _aurora(g, o, ev, info):
    if ev == "end_turn" and info["pid"] == o.ctrl:
        def res(g_, it):
            pl = g_.p[it.ctrl]
            revealed, unit = [], None
            for c in list(pl.deck):
                if c.spec["type"] == "Unit" and g_.impl(c) is not None:
                    unit = c
                    break
                revealed.append(c)
            g_.log(f"  Dazzling Aurora reveals {revealed + ([unit] if unit else [])}")
            if unit is not None:
                play_unit_free(g_, it.ctrl, unit, "deck")
            for c in revealed:
                if c in pl.deck:
                    pl.deck.remove(c)
            g_.recycle_cards(it.ctrl, revealed)
        _trig(g, o, "Dazzling Aurora", res)


card("Dazzling Aurora", on_event=_aurora)


# Demacian Diplomat — "When you play me, gain 1 XP."
card("Demacian Diplomat", on_play=lambda g, o, ctx: _trig(g, o, "Demacian Diplomat",
                                                          lambda g_, it: g_.gain_xp(it.ctrl, 1)))


# Direwing — "I enter ready if you control another Dragon."
card("Direwing", enter_ready=lambda g, pid, c, ch: any("Dragon" in u.spec["tags"] and u is not c
                                                       for u in g.units(pid)))


# Dragonsoul Sage — "[Reaction][>] [E]: [Add] [1]." (used while paying, rule 429.3)
card("Dragonsoul Sage", add=[add_ability("1 energy")])


# Dune Drake — "When I attack, give me +2 might this turn if there is a ready enemy unit here."
def _drake(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        def res(g_, it):
            me = _src(g_, it)
            if me is not None and any(not u.exhausted for u in g_.units(1 - me.ctrl, me.loc)):
                g_.mod(me, 2)
        _trig(g, o, "Dune Drake", res)


card("Dune Drake", on_event=_drake)


# Fiora, Peerless — "When I attack or defend one on one, double my Might this combat." (one on one: she and the
# enemy unit there are both alone, rule 740.2.b; "this combat" effects expire when the combat ends, 466.7.c)
def _fiora(g, o, ev, info):
    if ev in ("attack", "defend") and info["obj"] is o:
        if not (g.alone(o) and len(g.units(1 - o.ctrl, o.loc)) == 1):
            return

        def res(g_, it):
            me = _src(g_, it)
            if me is None:
                return
            m = g_.might(me)
            if m <= 0:
                return
            entry = [m, "turn"]
            me.mods.append(entry)

            def end(g2, eff, inf):
                if inf["bf"] == eff["bf"]:
                    u = g2.obj(eff["uid"])
                    if u is not None:
                        u.mods = [x for x in u.mods if x is not eff["entry"]]
                    g2.effects.remove(eff)
            g_.effects.append(dict(on="combat_end", fn=end, dur="turn", bf=me.loc, uid=me.uid, entry=entry))
        _trig(g, o, "Fiora, Peerless", res)


card("Fiora, Peerless", on_event=_fiora)


# First Mate — "When you play me, ready another unit."
def _first_mate(g, o, ctx):
    def opts(g_, it):
        us = [u for u in g_.units() if u.uid != it.src and (u.ctrl == it.ctrl or g_.targetable(u, it.ctrl))]
        return sorted(us, key=lambda u: (u.ctrl != it.ctrl, not u.exhausted, -value(g_, u)))
    _trig(g, o, "First Mate", lambda g_, it: g_.legal(it, 0) is not None and g_.ready_obj(g_.legal(it, 0)),
          choose=_pick(opts, P_unit, kind="ready_target"))


card("First Mate", on_play=_first_mate)


# Fretful Feline — "When I become ready, give me +2 might this turn."
card("Fretful Feline", on_event=lambda g, o, ev, info: ev == "ready" and info["obj"] is o and _trig(
    g, o, "Fretful Feline", lambda g_, it: _src(g_, it) is not None and g_.mod(_src(g_, it), 2)))


# Garen, Rugged — "[Assault 2], [Shield 2]"
card("Garen, Rugged", kw={"Assault": 2, "Shield": 2})


# Gemhand Hunter — "[Hunt] [Level 6] I have +1 might." (the trailing "ambush" of the database is not on the card)
card("Gemhand Hunter", kw={"Hunt": 1}, levels=[(6, dict(might=1))])


# Gentle Gemdragon — "When you play me or another Dragon, ready up to 2 runes."
def _gemdragon(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and (info["card"] is o or "Dragon" in info["card"].spec["tags"]):
        _trig(g, o, "Gentle Gemdragon", lambda g_, it: _ready_runes(g_, it.ctrl, 2))


card("Gentle Gemdragon", on_event=_gemdragon)


# Imposing Challenger — "When I move, you may move an enemy unit here with less Might than me to a different
# battlefield." ("here" and "me" are checked again on resolution, rule 359.3.f)
def _challenger(g, o, ev, info):
    if ev == "move" and info["obj"] is o and o.loc in (0, 1):
        here = o.loc

        def ok(g_, it, u):
            me = _src(g_, it)
            return me is not None and u.ctrl != it.ctrl and u.loc == here and me.loc == here and \
                g_.might(u) < g_.might(me)

        def opts(g_, it):
            me = _src(g_, it)
            if me is None:
                return []
            return [u for u in enemies(g_, it.ctrl) if u.loc == here and g_.might(u) < g_.might(me)]

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.move([u], 1 - here, it.ctrl)
        _trig(g, o, "Imposing Challenger", res, may=True, choose=_pick(opts, ok, kind="target"))


card("Imposing Challenger", on_event=_challenger)


# Irresistible Faefolk — "When I move to a battlefield, you may move an enemy unit to that battlefield."
def _faefolk(g, o, ev, info):
    if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
        dest = info["to"]

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None and u.loc != dest:
                g_.move([u], dest, it.ctrl)
        _trig(g, o, "Irresistible Faefolk", res, may=True,
              choose=_pick(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc != dest], P_enemy,
                           kind="faefolk_target"))


card("Irresistible Faefolk", on_event=_faefolk)


# Jaull-Fish — "[Accelerate] I cost 2 energy less for each of your [Mighty] units."
card("Jaull-Fish", accelerate=True,
     cost_mod=lambda g, pid, c, ch: (2 * sum(1 for u in g.units(pid) if g.mighty(u)), 0))


# Jax, Unrelenting — "[Weaponmaster] When you attach an Equipment to me, you may pay [1] to draw 1."
def _jax(g, o, ev, info):
    if ev == "attached" and info["unit"] is o and "Equipment" in info["gear"].spec["tags"]:
        _trig(g, o, "Jax, Unrelenting", lambda g_, it: g_.draw(it.ctrl, 1), may=True, cost=may_pay(1))


card("Jax, Unrelenting", kw={"Weaponmaster": 1}, on_event=_jax)


# Jayce, Hammer in Hand — "When I become ready, choose one to give me this turn — [Assault 2]; [Deflect 2];
# [Ganking]."
def _jayce(g, o, ev, info):
    if ev == "ready" and info["obj"] is o:
        def res(g_, it):
            me = _src(g_, it)
            if me is None:
                return
            opts = ["Assault", "Ganking", "Deflect"] if me.loc == "base" else ["Ganking", "Assault", "Deflect"]
            m = g_.ask(it.ctrl, "jayce_mode", opts, item=it)
            g_.grant(me, m, 1 if m == "Ganking" else 2)
        _trig(g, o, "Jayce, Hammer in Hand", res)


card("Jayce, Hammer in Hand", on_event=_jayce)


# Kato the Arm — "[Deflect] When I move to a battlefield, give a friendly unit my keywords and +might equal to my
# Might this turn." (keywords and Might read on resolution, rule 359.3.f)
_KWS = ("Assault", "Backline", "Deflect", "Ganking", "Hunt", "Shield", "Tank", "Temporary", "Vision", "Weaponmaster")


def _kato(g, o, ev, info):
    if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
        def res(g_, it):
            me = _src(g_, it)
            u = g_.legal(it, 0)
            if me is None or u is None:
                return
            for k in _KWS:
                v = g_.kw_value(me, k)
                if v > 0:
                    g_.grant(u, k, v)
            g_.mod(u, max(0, g_.might(me)))

        def opts(g_, it):
            return sorted(g_.units(it.ctrl), key=lambda u: (u.uid == it.src, u.loc not in (0, 1), -value(g_, u)))
        _trig(g, o, "Kato the Arm", res, choose=_pick(opts, P_friend, kind="kato_target", deflect=False))


card("Kato the Arm", kw={"Deflect": 1}, on_event=_kato)


# Kha'Zix, Evolving Hunter — "[Hunt] When I attack, you may spend 3 XP to deal damage equal to my Might to an enemy
# unit here." (cost within instructions of a trigger: paid on finalization, rule 740.4.a.2)
def _khazix(g, o, ev, info):
    if ev == "attack" and info["obj"] is o and g.p[o.ctrl].xp >= 3:
        here = o.loc

        def res(g_, it):
            me = _src(g_, it)
            u = g_.legal(it, 0)
            if me is not None and u is not None:
                g_.deal(u, max(0, g_.might(me)), "ability", it.ctrl)
        _trig(g, o, "Kha'Zix, Evolving Hunter", res, dict(dmg=g.might(o)), may=True,
              cost=lambda g_, it: g_.spend_xp(it.ctrl, 3),
              choose=_pick(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                           lambda g_, it, u: u.ctrl != it.ctrl and u.loc == here, kind="target"))


card("Kha'Zix, Evolving Hunter", kw={"Hunt": 1}, on_event=_khazix)


# Kinkou Initiate — "When you play me, draw 1 if your other units have total Might 5 or more."
def _initiate(g, o, ctx):
    def res(g_, it):
        if sum(max(0, g_.might(u)) for u in g_.units(it.ctrl) if u.uid != it.src) >= 5:
            g_.draw(it.ctrl, 1)
    _trig(g, o, "Kinkou Initiate", res)


card("Kinkou Initiate", on_play=_initiate)


# Kinkou Monk — "When you play me, buff up to two other friendly units."
def _monk(g, o, ctx):
    def opts(g_, it):
        return sorted([u for u in g_.units(it.ctrl) if u.uid != it.src], key=lambda u: (u.buff > 0, -value(g_, u)))
    _trig(g, o, "Kinkou Monk", lambda g_, it: [g_.buff(u) for u in _targets(g_, it)],
          choose=_pick(opts, P_friend, n=2, kind="buff_target", deflect=False))


card("Kinkou Monk", on_play=_monk)


# Kraken Hunter — "[Accelerate] [Assault] As you play me, you may spend any number of buffs as an additional cost.
# Reduce my cost by [Body] for each buff you spend."
def _kraken_extra(g, pid, c):
    bs = sorted([u for u in g.units(pid) if u.buff > 0], key=lambda u: value(g, u))
    out = [dict()]
    for u in bs[:2]:
        out.append(dict(spend=(u.uid,)))
    if len(bs) >= 2:
        out.append(dict(spend=(bs[0].uid, bs[1].uid)))
    return out


def _kraken_pay(g, pid, c, ch):
    for uid in ch.get("spend", ()):
        u = g.obj(uid)
        if u is not None and u.ctrl == pid:
            _spend_buff(g, u)


card("Kraken Hunter", accelerate=True, kw={"Assault": 1}, as_played=_kraken_extra, pay_extra=_kraken_pay,
     cost_mod=lambda g, pid, c, ch: (0, len([u for u in ch.get("spend", ()) if g.obj(u) is not None
                                              and g.obj(u).buff > 0])))


# Laurent Bladekeeper — "Ganking"
card("Laurent Bladekeeper", kw={"Ganking": 1})


# Lee Sin, Centered — "[Accelerate] Other buffed friendly units at my battlefield have +2 might."
def _lee_aura(g, src, o):
    if src.loc in (0, 1) and o is not src and o.loc == src.loc and o.ctrl == src.ctrl and o.buff > 0 \
            and o.spec["type"] == "Unit":
        return 2
    return 0


card("Lee Sin, Centered", accelerate=True, aura_might=_lee_aura)


# Legion Marauder — "[Empower] — [1] or [Body] [Empowered] I have +1 might."
card("Legion Marauder", abilities=[empower_ability("1 energy"), empower_ability("1 body rune")],
     might_if=[(when_empowered, 1)])


# Lucian, Merciless — "[Weaponmaster] The first time I conquer each turn, ready me."
def _lucian(g, o, ev, info):
    if ev == "conquer" and o in info["units"] and _first_each_turn(o, "lucian", g):
        _trig(g, o, "Lucian, Merciless", lambda g_, it: _src(g_, it) is not None and g_.ready_obj(_src(g_, it)))


card("Lucian, Merciless", kw={"Weaponmaster": 1}, on_event=_lucian)


# Master Yi, Honed — "[Ganking] I enter ready."
card("Master Yi, Honed", kw={"Ganking": 1}, enter_ready=True)

# Master Yi, Tempered — "[Hunt 2] [Level 6] I have [Deflect] and [Ganking]."
card("Master Yi, Tempered", kw={"Hunt": 2}, levels=[(6, dict(kw={"Deflect": 1, "Ganking": 1}))])


# Miss Fortune, Captain — "[Accelerate] [Ganking] The first time I move each turn, you may ready something else
# that's exhausted." (anything on the board: units, gear, runes, legends; rules 107, 415.1)
def _mf(g, o, ev, info):
    if ev == "move" and info["obj"] is o and _first_each_turn(o, "mf", g):
        def choose(g_, it):
            pid = it.ctrl
            mine = sorted([u for u in g_.units(pid) if u.exhausted and u.uid != it.src], key=lambda u: -value(g_, u))
            rs = [r for r in g_.p[pid].runes if r.exhausted][:1]
            other = [x for x in g_.board if x.exhausted and x.uid != it.src and x not in mine
                     and (x.ctrl == pid or (g_.targetable(x, pid) and _payable(g_, it, x)))]
            other.sort(key=lambda x: (x.ctrl != pid, x.spec["type"] != "Gear", x.uid))
            legs = sorted([pl.legend for pl in g_.p if pl.legend.exhausted], key=lambda x: x.owner != pid)
            erune = [r for r in g_.p[1 - pid].runes if r.exhausted][:1]
            opts = [("obj", x) for x in mine] + [("rune", r) for r in rs] + [("legend", x) for x in legs] + \
                [("obj", x) for x in other] + [("rune", r) for r in erune]
            if not opts:
                return False
            k, x = g_.ask(pid, "mf_ready", opts, item=it)
            if k == "obj":
                if not pay_deflect(g_, it, x):
                    return False
                g_.add_target(it, x)
            else:
                it.data["thing"] = (k, x)
            return True

        def res(g_, it):
            if it.targets:
                x = g_.legal(it, 0)
                if x is not None:
                    g_.ready_obj(x)
            elif it.data.get("thing"):
                it.data["thing"][1].exhausted = False
        _trig(g, o, "Miss Fortune, Captain", res, may=True, choose=choose)


card("Miss Fortune, Captain", accelerate=True, kw={"Ganking": 1}, on_event=_mf)


# Nidalee, Cat Form — "[Ambush] When I win a combat, draw 1." (I win if I remain after combat)
card("Nidalee, Cat Form", ambush=True,
     on_event=lambda g, o, ev, info: ev == "combat_won" and _won(g, o, info) and _trig(
         g, o, "Nidalee, Cat Form", lambda g_, it: g_.draw(it.ctrl, 1)))


# Nilah, Joyful Ascetic — "[Accelerate] [Ganking] When I move, gain 1 XP."
card("Nilah, Joyful Ascetic", accelerate=True, kw={"Ganking": 1},
     on_event=lambda g, o, ev, info: ev == "move" and info["obj"] is o and _trig(
         g, o, "Nilah, Joyful Ascetic", lambda g_, it: g_.gain_xp(it.ctrl, 1)))


# Noxian Demolitionist — "When I conquer, you may kill a gear with Energy cost no more than my Might."
def _demolitionist(g, o, ev, info):
    if ev == "conquer" and o in info["units"]:
        def ok(g_, it, x):
            me = _src(g_, it)
            return x.spec["type"] == "Gear" and me is not None and x.spec["e"] <= g_.might(me)

        def opts(g_, it):
            me = _src(g_, it)
            if me is None:
                return []
            return sorted([x for x in g_.gear() if x.spec["e"] <= g_.might(me)],
                          key=lambda x: (x.ctrl == it.ctrl, -x.spec["e"], x.uid))
        _trig(g, o, "Noxian Demolitionist", lambda g_, it: g_.kill([g_.legal(it, 0)], it.ctrl), may=True,
              choose=_pick(opts, ok, kind="gear_target"))


card("Noxian Demolitionist", on_event=_demolitionist)


# Pit Rookie — "When you play me, buff another friendly unit."
card("Pit Rookie", on_play=lambda g, o, ctx: _trig(
    g, o, "Pit Rookie", lambda g_, it: g_.buff(g_.legal(it, 0)),
    choose=_pick(lambda g_, it: sorted([u for u in g_.units(it.ctrl) if u.uid != it.src],
                                       key=lambda u: (u.buff > 0, -value(g_, u))), P_friend, kind="buff_target",
                 deflect=False)))


# Poppy, Paragon — "[Deflect] When you play me, if an opponent's score is within 3 points of the Victory Score,
# ready me and gain 3 XP." (intervening condition checked on trigger and on resolution)
def _poppy_cond(g, pid):
    return g.victory - g.p[1 - pid].points <= 3


def _poppy(g, o, ctx):
    if not _poppy_cond(g, o.ctrl):
        return

    def res(g_, it):
        if not _poppy_cond(g_, it.ctrl):
            return
        me = _src(g_, it)
        if me is not None:
            g_.ready_obj(me)
        g_.gain_xp(it.ctrl, 3)
    _trig(g, o, "Poppy, Paragon", res)


card("Poppy, Paragon", kw={"Deflect": 1}, on_play=_poppy)


# Profiteer — "When you play me, you may disempower something you control to empower a legend, unit, or gear."
# (disempowering is the cost within instructions, paid on finalization, rule 740.4.a.2)
def _profiteer(g, o, ctx):
    def choose(g_, it):
        pid = it.ctrl
        srcs = [x for x in g_.board if x.ctrl == pid and x.empowered]
        if g_.p[pid].legend.empowered:
            srcs.append(g_.p[pid].legend)
        if not srcs:
            return False
        dsts = [x for x in g_.board if not x.empowered and x.spec["type"] in ("Unit", "Gear")
                and (x.ctrl == pid or (g_.targetable(x, pid) and _payable(g_, it, x)))]
        dsts.sort(key=lambda x: (x.ctrl != pid, -value(g_, x)))
        legs = sorted([pl.legend for pl in g_.p if not pl.legend.empowered], key=lambda x: x.owner != pid)
        dsts = [x for x in dsts if x.ctrl == pid] + [x for x in legs if x.owner == pid] + \
            [x for x in dsts if x.ctrl != pid] + [x for x in legs if x.owner != pid]
        if not dsts:
            return False
        pairs = [(s_, d) for d in dsts for s_ in srcs]
        s_, d = g_.ask(pid, "profiteer", pairs, item=it)
        if d.zone == "board":
            if not pay_deflect(g_, it, d):
                return False
            g_.add_target(it, d)
        else:
            it.data["legend"] = d.owner
        g_.disempower(s_)
        return True

    def res(g_, it):
        if it.targets:
            d = g_.legal(it, 0)
        elif it.data.get("legend") is not None:
            d = g_.p[it.data["legend"]].legend
        else:
            d = None
        if d is not None:
            g_.empower(d)
    _trig(g, o, "Profiteer", res, may=True, choose=choose)


card("Profiteer", on_play=_profiteer)


# Qiyana, Victorious — "[Deflect] When I conquer, draw 1 or channel 1 rune exhausted."
def _qiyana(g, o, ev, info):
    if ev == "conquer" and o in info["units"]:
        def res(g_, it):
            opts = ["draw", "channel"]
            if len(g_.p[it.ctrl].runes) < 6 and g_.p[it.ctrl].rune_deck:
                opts = ["channel", "draw"]
            if g_.ask(it.ctrl, "qiyana_mode", opts, item=it) == "draw":
                g_.draw(it.ctrl, 1)
            else:
                g_.channel(it.ctrl, 1, exhausted=True)
        _trig(g, o, "Qiyana, Victorious", res)


card("Qiyana, Victorious", kw={"Deflect": 1}, on_event=_qiyana)


# Repair Specialist — "I have [Assault] equal to the number of gear you control."
card("Repair Specialist",
     aura_kw=lambda g, src, o: {"Assault": len(g.gear(src.ctrl))} if o is src else None)


# Ruin Runner — "I can't be chosen by enemy spells and abilities."
card("Ruin Runner", untargetable=lambda g, o, by: by != o.ctrl)


# Sea Monkey — "You may pay [1] as an additional cost to play me. When you play me, if you paid the additional
# cost, buff me."
card("Sea Monkey", as_played=lambda g, pid, c: [dict(), dict(paid=True)],
     extra_cost_fn=lambda g, pid, c, ch: (1, []) if ch.get("paid") else (0, []),
     on_play=lambda g, o, ctx: ctx.get("paid") and _trig(g, o, "Sea Monkey", lambda g_, it: g_.buff(_src(g_, it))))


# Sett, Brawler — "When I'm played and when I conquer, buff me. Spend my buff: Give me +4 might this turn."
card("Sett, Brawler",
     on_play=lambda g, o, ctx: _trig(g, o, "Sett, Brawler", lambda g_, it: g_.buff(_src(g_, it))),
     on_event=lambda g, o, ev, info: ev == "conquer" and o in info["units"] and _trig(
         g, o, "Sett, Brawler (conquer)", lambda g_, it: g_.buff(_src(g_, it))),
     abilities=[ability("Spend buff", can=lambda g, pid, o: o.buff > 0,
                        extra_cost=lambda g, pid, o, ch: _spend_buff(g, o),
                        resolve=lambda g, it: g.obj(it.src) is not None and g.mod(g.obj(it.src), 4))])


# Sivir, Ambitious — "[Deflect 2] When I conquer after an attack, if you assigned 5 or more excess damage to enemy
# units, you may deal that much to an enemy unit." Excess damage on a unit = damage assigned to it beyond the lethal
# damage it needed when damage was assigned (rule 142.4.b), from the attacker's assignment (Showdown.assigned).
def _sivir(g, o, ev, info):
    if ev == "combat_start":
        o._body_sivir = (g.turn_no, info["bf"], {})
    elif ev == "damaged" and info["kind"] == "combat":
        sd = g.sd
        u = info["obj"]
        rec = getattr(o, "_body_sivir", None)
        if sd is None or sd.assigned is None or u.ctrl == o.ctrl or sd.attacker != o.ctrl or rec is None \
                or rec[:2] != (g.turn_no, sd.bf):
            return
        a = sd.assigned[0].get(u, 0)
        n = info["amount"]
        d0 = u.damage - n                       # damage marked before this combat damage
        p0 = (a - n) + u.prevent                # prevention it had when damage was assigned
        need = max(1, g.might(u) - d0) + p0
        rec[2][u.uid] = max(0, a - need)
    elif ev == "conquer" and o in info["units"]:
        sd = g.sd
        rec = getattr(o, "_body_sivir", None)
        if sd is None or not sd.combat or sd.attacker != o.ctrl or sd.bf != info["bf"] or rec is None \
                or rec[:2] != (g.turn_no, sd.bf):
            return
        ex = sum(rec[2][k] for k in sorted(rec[2]))
        if ex < 5:
            return
        _trig(g, o, "Sivir, Ambitious", lambda g_, it: g_.deal(g_.legal(it, 0), it.data["dmg"], "ability", it.ctrl),
              dict(dmg=ex), may=True,
              choose=_pick(lambda g_, it: enemies(g_, it.ctrl), P_enemy, kind="target"))


card("Sivir, Ambitious", kw={"Deflect": 2}, on_event=_sivir)


# Stormclaw Ursine — "[Tank] When you play me, channel 1 rune exhausted."
card("Stormclaw Ursine", kw={"Tank": 1}, on_play=lambda g, o, ctx: _trig(
    g, o, "Stormclaw Ursine", lambda g_, it: g_.channel(it.ctrl, 1, exhausted=True)))

# Targonian Visionary — "[Level 11] I have +4 might."
card("Targonian Visionary", levels=[(11, dict(might=4))])


# Udyr, Wildman — "Spend my buff: Choose one you've not chosen this turn — Deal 2 to a unit at a battlefield.
# Stun a unit at a battlefield. Ready me. Give me [Ganking] this turn."
def _udyr_used(g, o):
    rec = getattr(o, "_body_udyr", None)
    if rec is None or rec[:2] != (g.turn_no, o.oid):
        return ()
    return rec[2]


def _udyr_choices(g, pid, o):
    used = _udyr_used(g, o)
    out = []
    if "ready" not in used and o.exhausted:
        out.append(dict(mode="ready"))
    if "dmg" not in used:
        es = [u for u in enemies(g, pid, True) if g.might(u) - u.damage <= 2] + \
             [u for u in enemies(g, pid, True) if g.might(u) - u.damage > 2]
        out += [dict(mode="dmg", tg=(u.uid,)) for u in es[:3]]
    if "stun" not in used:
        out += [dict(mode="stun", tg=(u.uid,)) for u in enemies(g, pid, True) if not u.stunned][:2]
    if "gank" not in used and not g.has_kw(o, "Ganking"):
        out.append(dict(mode="gank"))
    if "ready" not in used and not o.exhausted:
        out.append(dict(mode="ready"))
    return out


def _udyr_pay(g, pid, o, ch):
    _spend_buff(g, o)
    o._body_udyr = (g.turn_no, o.oid, tuple(_udyr_used(g, o)) + (ch["mode"],))


def _udyr_res(g, it):
    m = it.data["mode"]
    me = g.obj(it.src)
    if m in ("dmg", "stun"):
        u = g.legal(it, 0)
        if u is not None:
            if m == "dmg":
                g.deal(u, 2, "ability", it.ctrl)
            else:
                g.stun(u, it.ctrl)
    elif me is not None:
        if m == "ready":
            g.ready_obj(me)
        else:
            g.grant(me, "Ganking", 1)


card("Udyr, Wildman",
     abilities=[ability("Spend buff", can=lambda g, pid, o: o.buff > 0, choices=_udyr_choices, preds=[P_unit_bf],
                        extra_cost=_udyr_pay, resolve=_udyr_res)])


# Volibear, Imposing — "[Shield 3] [Tank] When an opponent moves to a battlefield other than mine, draw 1."
# Rules decision: one trigger per move performed by an opponent (units moved together count once).
def _volibear(g, o, ev, info):
    if ev == "move" and info["by"] == 1 - o.ctrl and info["to"] in (0, 1) and info["to"] != o.loc:
        for t in g.trigq:
            if t.src == o.uid and t.name == "Volibear, Imposing" and t.data.get("to") == info["to"]:
                return
        _trig(g, o, "Volibear, Imposing", lambda g_, it: g_.draw(it.ctrl, 1), dict(to=info["to"]))


card("Volibear, Imposing", kw={"Shield": 3, "Tank": 1}, on_event=_volibear)


# Warwick, Hunter — "I enter ready. When I attack, kill all damaged enemy units here."
def _warwick(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        def res(g_, it):
            me = _src(g_, it)
            here = me.loc if me is not None else it.data["loc"]
            g_.kill([u for u in g_.units(1 - it.ctrl, here) if u.damage > 0], it.ctrl)
        _trig(g, o, "Warwick, Hunter", res, dict(loc=o.loc))


card("Warwick, Hunter", enter_ready=True, on_event=_warwick)


# Wildclaw Shaman — "When you play me, you may spend a buff to buff me and ready me."
def _shaman(g, o, ctx):
    def cost(g_, it):
        bs = sorted([u for u in g_.units(it.ctrl) if u.buff > 0], key=lambda u: (u.uid == it.src, value(g_, u)))
        if not bs:
            return False
        return _spend_buff(g_, g_.ask(it.ctrl, "spend_buff", bs, item=it))

    def res(g_, it):
        me = _src(g_, it)
        if me is not None:
            g_.buff(me)
            g_.ready_obj(me)
    _trig(g, o, "Wildclaw Shaman", res, may=True, cost=cost)


card("Wildclaw Shaman", on_play=_shaman)


# Yone, Blademaster — "[Weaponmaster] When I conquer an open battlefield, deal damage equal to my Might to an enemy
# unit in a base." Open = unoccupied and uncontrolled (rule 170.11.c): in 1v1 that is a conquer that does not
# follow a combat (an empty battlefield loses its controller at the next cleanup, rule 323).
def _yone(g, o, ev, info):
    if ev == "conquer" and o in info["units"] and (g.sd is None or not g.sd.combat):
        def res(g_, it):
            me = _src(g_, it)
            u = g_.legal(it, 0)
            if me is not None and u is not None:
                g_.deal(u, max(0, g_.might(me)), "ability", it.ctrl)
        _trig(g, o, "Yone, Blademaster", res, dict(dmg=g.might(o)),
              choose=_pick(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == "base"],
                           lambda g_, it, u: u.ctrl != it.ctrl and u.loc == "base", kind="target"))


card("Yone, Blademaster", kw={"Weaponmaster": 1}, on_event=_yone)


# Yordle Explorer — "When you play a card with Power cost [A][A] or more, draw 1." (printed cost, rule 206)
card("Yordle Explorer", on_event=lambda g, o, ev, info: ev == "played" and info["pid"] == o.ctrl and
     info["card"].spec["p"] >= 2 and _trig(g, o, "Yordle Explorer", lambda g_, it: g_.draw(it.ctrl, 1)))


# ====================================================================== gear
# Arena Bar — "[E]: Buff an exhausted friendly unit."
card("Arena Bar", abilities=[ability(
    "Buff", exhaust=True, preds=[lambda g, it, o: P_friend(g, it, o) and o.exhausted],
    choices=lambda g, pid, o: tg_choices(sorted([u for u in g.units(pid) if u.exhausted],
                                                key=lambda u: (u.buff > 0, -value(g, u)))),
    resolve=lambda g, it: g.buff(g.legal(it, 0)))])


# Blood Rose — "When you play a unit, you may pay [1] to gain 1 XP. Spend 3 XP, [E]: Ready a unit."
def _blood_rose(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and info["card"].spec["type"] == "Unit":
        _trig(g, o, "Blood Rose", lambda g_, it: g_.gain_xp(it.ctrl, 1), may=True, cost=may_pay(1))


card("Blood Rose", on_event=_blood_rose, abilities=[ability(
    "Ready", xp=3, exhaust=True, preds=[P_unit],
    choices=lambda g, pid, o: tg_choices(sorted([u for u in g.units() if u.ctrl == pid or g.targetable(u, pid)],
                                                key=lambda u: (u.ctrl != pid, not u.exhausted, -value(g, u)))),
    resolve=lambda g, it: g.legal(it, 0) is not None and g.ready_obj(g.legal(it, 0)))])


# Equipment: Might Bonus and Effect Text read on the card images (not in the card database).
# Boneshiver — "[Equip] [1][Body]", +2, "When I conquer, channel 1 rune exhausted."
def _boneshiver(g, gear, unit, ev, info):
    if ev == "conquer" and unit in info["units"]:
        g.queue_trigger(unit.ctrl, "Boneshiver", lambda g_, it: g_.channel(it.ctrl, 1, exhausted=True), src=gear.uid)


card("Boneshiver", equip="1 energy and 1 body rune", bonus=2, effect_event=_boneshiver)

# Doran's Blade — "[Equip] [Body]", +2
card("Doran's Blade", equip="1 body rune", bonus=2)

# Hexdrinker — "[Equip] [Body]", +1, "[Deflect]"
card("Hexdrinker", equip="1 body rune", bonus=1, equip_kw={"Deflect": 1})

# Hunter's Machete — "[Equip] [Body]", +2, "[Hunt]"
card("Hunter's Machete", equip="1 body rune", bonus=2, equip_kw={"Hunt": 1})


# Trinity Force — "[Equip] [Body]", +2, "When I hold, score 1 point." (a point not gained through Conquer: no
# Final Point restriction, rule 471.1.a.1)
def _trinity(g, gear, unit, ev, info):
    if ev == "hold" and unit in info["units"]:
        g.queue_trigger(unit.ctrl, "Trinity Force", lambda g_, it: g_.gain_point(it.ctrl, "Trinity Force"),
                        src=gear.uid)


card("Trinity Force", equip="1 body rune", bonus=2, effect_event=_trinity)


# Warmog's Armor — "[Equip] [Body]", +1, "When I conquer, buff me."
def _warmog_res(g, it):
    u = g.obj(it.data["u"])
    if u is not None and u.oid == it.data["uoid"]:
        g.buff(u)


def _warmog(g, gear, unit, ev, info):
    if ev == "conquer" and unit in info["units"]:
        g.queue_trigger(unit.ctrl, "Warmog's Armor", _warmog_res, dict(u=unit.uid, uoid=unit.oid), src=gear.uid)


card("Warmog's Armor", equip="1 body rune", bonus=1, effect_event=_warmog)


# Hextech Disc — "[Empower] — [E] Disempower this, [1], [E]: Play a 3 might Mech unit token to your base."
card("Hextech Disc", abilities=[
    empower_ability("", exhaust=True),
    ability("Mech", cost="1 energy", exhaust=True, can=lambda g, pid, o: o.empowered,
            extra_cost=lambda g, pid, o, ch: g.disempower(o),
            resolve=lambda g, it: make_token(g, "Mech", it.ctrl, "base"))])


# Mistfall — "When you buff a friendly unit, you may pay [Body] and exhaust this to ready it."
# (No card buffs an enemy unit, so a buff of a friendly unit is a buff by its controller.)
def _mistfall(g, o, ev, info):
    if ev == "buff" and info["obj"].ctrl == o.ctrl and not o.exhausted:
        u = info["obj"]

        def cost(g_, it):
            me = _src(g_, it)
            if me is None or me.exhausted or not g_.can_pay(it.ctrl, 0, [_BODY]):
                return False
            g_.pay(it.ctrl, 0, [_BODY])
            me.exhausted = True
            return True

        def res(g_, it):
            x = g_.obj(it.data["u"])
            if x is not None and x.oid == it.data["uoid"]:
                g_.ready_obj(x)
        _trig(g, o, "Mistfall", res, dict(u=u.uid, uoid=u.oid), may=True, cost=cost)


card("Mistfall", on_event=_mistfall)


# Petricite Monument — "[Temporary] Friendly units have [Deflect]."
card("Petricite Monument", kw={"Temporary": 1},
     aura_kw=lambda g, src, o: {"Deflect": 1} if o.ctrl == src.ctrl and o.spec["type"] == "Unit" else None)


# Platewyrm Egg — "This enters exhausted. [Empower] — [1], [E] [Reaction][>] [E]: [Add] [1]. If this is
# [Empowered], [Add] [2] instead."
card("Platewyrm Egg", enter_exhausted=True, abilities=[empower_ability("1 energy", exhaust=True)],
     add=[add_ability("1 energy", can=lambda g, pid, o, ctx: not o.empowered),
          add_ability("2 energy", can=lambda g, pid, o, ctx: o.empowered)])

# Seal of Strength — "[E]: [Reaction] — [Add] [Body]."
card("Seal of Strength", add=[add_ability("1 body rune")])


# Tools of Empire — "[Empower] [2] [E]: Give a unit +2 might this turn. If this is [Empowered], give that unit +4
# might this turn instead."
def _tools(g, it):
    u = g.legal(it, 0)
    me = g.obj(it.src)
    if u is not None:
        g.mod(u, 4 if (me is not None and me.empowered) else 2)


card("Tools of Empire", empower="2 energy", abilities=[ability(
    "Might", exhaust=True, preds=[P_unit], resolve=_tools,
    choices=lambda g, pid, o: tg_choices(friends(g, pid, True) + [u for u in friends(g, pid) if u.loc == "base"]))])


# ====================================================================== spells
# Bullet Time — "[Action] Pay any amount of [A] to deal that much damage to all enemy units at a battlefield."
# (cost within instructions of a spell: paid on resolution, rule 740.4.a.1)
def _bullet(g, it):
    pid, bf = it.ctrl, it.data["bf"]
    mx = 0
    while mx < 12 and g.can_pay(pid, 0, [ANY] * (mx + 1)):
        mx += 1
    es = g.units(1 - pid, bf)
    need = max([g.might(u) - u.damage for u in es] + [0])
    if g.bfs[bf].name == "Void Gate":
        need -= 1
    opts = list(range(mx, -1, -1))
    if 0 < need <= mx:
        opts.remove(need)
        opts.insert(0, need)
    x = g.ask(pid, "bullet_time_x", opts, item=it)
    if not x or not g.pay(pid, 0, [ANY] * x):
        return
    for u in list(g.units(1 - pid, bf)):
        g.deal(u, x, "spell", pid)


card("Bullet Time", timing="action", resolve=_bullet,
     choices=lambda g, pid, ctx: sorted([dict(bf=b.idx) for b in g.bfs if ctx["hidden_bf"] in (None, b.idx)],
                                        key=lambda c: -len(g.units(1 - pid, c["bf"]))))


# Call to Battle — "Move a unit you control to a battlefield you control. Then, choose an opponent. They move a
# unit they control to the same battlefield."
def _call(g, it):
    dest = it.data["dest"]
    u = g.legal(it, 0)
    if u is not None and g.bfs[dest].ctrl == it.ctrl and u.loc != dest:
        g.move([u], dest, it.ctrl)
    opp = 1 - it.ctrl
    us = sorted([x for x in g.units(opp) if x.loc != dest], key=lambda x: -value(g, x))
    if us:
        x = g.ask(opp, "call_to_battle", us, item=it)
        g.move([x], dest, opp)


card("Call to Battle", preds=[P_friend], resolve=_call,
     choices=lambda g, pid, ctx: [dict(tg=(u.uid,), dest=b.idx) for b in g.bfs if b.ctrl == pid
                                  for u in friends(g, pid, False, ctx["hidden_bf"]) if u.loc != b.idx])


# Cannon Barrage — "[Reaction] Deal 2 to all enemy units in combat."
card("Cannon Barrage", timing="reaction",
     resolve=lambda g, it: [g.deal(u, 2, "spell", it.ctrl) for u in g.units(1 - it.ctrl) if g.in_combat(u)])


# Cataclysmic Duel — "Each player chooses a unit they control. Kill the rest."
def _cataclysm(g, it):
    keep = []
    for pid in (g.tp, 1 - g.tp):
        us = sorted(g.units(pid), key=lambda u: -value(g, u))
        if us:
            keep.append(g.ask(pid, "duel_keep", us, item=it))
    g.kill([u for u in g.units() if u not in keep], it.ctrl)


card("Cataclysmic Duel", resolve=_cataclysm)


# Catalyst of Aeons — "Channel 2 runes exhausted. If you couldn't channel 2 runes this way, draw 1."
card("Catalyst of Aeons",
     resolve=lambda g, it: len(g.channel(it.ctrl, 2, exhausted=True)) < 2 and g.draw(it.ctrl, 1))


# Challenge — "[Action] Choose a friendly unit and an enemy unit. They deal damage equal to their Mights to each
# other."
card("Challenge", timing="action", preds=[P_friend, P_enemy],
     resolve=lambda g, it: _fight(g, g.legal(it, 0), g.legal(it, 1)),
     choices=lambda g, pid, ctx: _duel_choices(g, pid, friends(g, pid, False, ctx["hidden_bf"]),
                                               enemies(g, pid, False, ctx["hidden_bf"])))


# Clash of Giants — "Choose two units. They deal damage equal to their Mights to each other."
def _clash_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    es = enemies(g, pid, False, hb)[:4]
    fs = friends(g, pid, False, hb)[:3]
    out = []
    for i, a in enumerate(es):
        for b in es[i + 1:]:
            out.append(dict(tg=(a.uid, b.uid)))
    return out + _duel_choices(g, pid, fs, es)


card("Clash of Giants", preds=[P_unit, P_unit], resolve=lambda g, it: _fight(g, g.legal(it, 0), g.legal(it, 1)),
     choices=_clash_choices)


# Concentrate — "Draw 2. [Level 6] This costs [2] less. [Level 11] This costs [4] less instead."
card("Concentrate", resolve=lambda g, it: g.draw(it.ctrl, 2),
     cost_mod=lambda g, pid, c, ch: (4 if g.p[pid].xp >= 11 else 2 if g.p[pid].xp >= 6 else 0, 0))


# Confront — "[Action] Units you play this turn enter ready. Draw 1." (the unit is readied as it enters, before
# anything can happen: it enters ready, so no "ready" event)
def _confront(g, it):
    def enter(g_, eff, info):
        c = info["card"]
        if info["pid"] == eff["pid"] and c.spec["type"] == "Unit" and c in g_.board:
            c.exhausted = False
    g.effects.append(dict(on="played", fn=enter, dur="turn", pid=it.ctrl))
    g.draw(it.ctrl, 1)


card("Confront", timing="action", resolve=_confront)


# Decisive Strike — "[Action] Give friendly units +2 might this turn."
card("Decisive Strike", timing="action", resolve=lambda g, it: [g.mod(u, 2) for u in g.units(it.ctrl)])


# Decree of Strength — "Choose an opponent. They reveal their hand and you choose a Mind card from it. They
# recycle that card."
def _reveal_recycle(g, it, ok, kind):
    opp = 1 - it.ctrl
    g.log(f"  P{opp} reveals {g.p[opp].hand}")
    cs = sorted([c for c in g.p[opp].hand if ok(c)], key=lambda c: (-(c.spec["e"] + 2 * c.spec["p"]), c.uid))
    if cs:
        c = g.ask(it.ctrl, kind, cs, item=it)
        g.recycle_cards(opp, [c])


card("Decree of Strength", resolve=lambda g, it: _reveal_recycle(g, it, lambda c: "Mind" in c.spec["domains"],
                                                                 "decree_strength"))


# Disposal Order — "[Reaction] Choose one — Choose up to 3 cards from opponents' trashes. Their owners recycle
# them. / Draw 1."
def _disposal(g, it):
    if it.data.get("mode") == "draw":
        g.draw(it.ctrl, 1)
        return
    opp = 1 - it.ctrl
    cs = [c for c in g.p[opp].trash if c.uid in it.data.get("cards", ())]
    if cs:
        g.recycle_cards(opp, cs)


def _disposal_choices(g, pid, ctx):
    out = [dict(mode="draw")]
    tr = sorted(g.p[1 - pid].trash, key=lambda c: (g.impl(c) is None or not g.impl(c).flow,
                                                   -(c.spec["e"] + 2 * c.spec["p"]), c.uid))
    if tr:
        out.append(dict(mode="recycle", cards=tuple(c.uid for c in tr[:3])))
        if len(tr) > 1:
            out.append(dict(mode="recycle", cards=(tr[0].uid,)))
    return out


card("Disposal Order", timing="reaction", resolve=_disposal, choices=_disposal_choices)


# Flurry of Blades — "[Reaction] Deal 1 to all units at battlefields."
card("Flurry of Blades", timing="reaction",
     resolve=lambda g, it: [g.deal(u, 1, "spell", it.ctrl) for u in g.units() if u.loc in (0, 1)])


# Gentlemen's Duel — "[Action] Give a friendly unit +3 might this turn. Then choose an enemy unit. They deal damage
# equal to their Mights to each other." (both chosen as the spell is played, rule 355)
def _gduel(g, it):
    f = g.legal(it, 0)
    if f is not None:
        g.mod(f, 3)
    _fight(g, f, g.legal(it, 1))


card("Gentlemen's Duel", timing="action", preds=[P_friend, P_enemy], resolve=_gduel,
     choices=lambda g, pid, ctx: _duel_choices(g, pid, friends(g, pid, False, ctx["hidden_bf"]),
                                               enemies(g, pid, False, ctx["hidden_bf"]), bonus=3))


# Grim Resolve — "[Action] Give a friendly unit +3 might this turn. When it wins a combat this turn, gain 2 XP."
def _grim(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    g.mod(u, 3)

    def won(g_, eff, info):
        x = g_.obj(eff["uid"])
        if x is not None and x.oid == eff["oid"] and _won(g_, x, info):
            g_.queue_trigger(x.ctrl, "Grim Resolve", lambda g2, it2: g2.gain_xp(it2.ctrl, 2), src=x.uid)
    g.effects.append(dict(on="combat_won", fn=won, dur="turn", uid=u.uid, oid=u.oid))


card("Grim Resolve", timing="action", preds=[P_friend], resolve=_grim,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid, False, ctx["hidden_bf"])))


# Guttural Roar — "[Action] Give a unit +2 might this turn. If it's [Empowered], give it +4 might this turn instead."
def _roar(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.mod(u, 4 if u.empowered else 2)


card("Guttural Roar", timing="action", preds=[P_unit], resolve=_roar,
     choices=lambda g, pid, ctx: tg_choices(sorted(friends(g, pid, False, ctx["hidden_bf"]),
                                                   key=lambda u: (not u.empowered, -value(g, u)))))


# Here to Help — "[Hidden] [Action] You may play a unit from hand to a battlefield you control, reducing its cost
# by [3]."
def _here_to_help(g, it):
    pid = it.ctrl
    locs = [b.idx for b in g.bfs if b.ctrl == pid]
    if not locs:
        return
    opts = []
    seen = set()
    for c in list(g.p[pid].hand):
        if c.spec["type"] != "Unit" or c.cname in seen or g.impl(c) is None:
            continue
        seen.add(c.cname)
        for ch in _unit_play_choices(g, pid, c, locs):
            e, reqs = _reduced_cost(g, pid, c, ch, "hand", 3)
            if g.can_pay(pid, e, reqs, dict(kind="unit", card=c)):
                opts.append((c, ch))
    if not opts:
        return
    opts.sort(key=lambda x: (-(x[0].spec["e"] + 2 * x[0].spec["p"]), x[1].get("acc", False), x[0].uid))
    pick = g.ask(pid, "here_to_help", opts + [None], item=it)
    if pick is not None:
        _play_reduced(g, pid, pick[0], "hand", dict(pick[1]), 3)


card("Here to Help", hidden=True, timing="action", resolve=_here_to_help)


# Keeper's Verdict — "[Action] Choose an enemy unit at a battlefield. Its owner places it on the top or bottom of
# their Main Deck."
def _verdict(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    pos = g.ask(u.owner, "verdict_place", ["top", "bottom"], item=it, unit=u)
    g.log(f"  {u} goes to the {pos} of its owner's deck")
    g.to_zone(u, "deck", bottom=(pos == "bottom"))


card("Keeper's Verdict", timing="action", preds=[P_enemy_bf], resolve=_verdict,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# Marching Orders — "[Action] [Repeat] [3] Choose a friendly unit anywhere and an enemy unit at a battlefield. They
# deal damage equal to their Mights to each other."
card("Marching Orders", timing="action", repeat=repeat_cost("3 energy"), preds=[P_friend, P_enemy_bf],
     resolve=repeatable(lambda g, it: _fight(g, g.legal(it, 0), g.legal(it, 1))),
     choices=lambda g, pid, ctx: _duel_choices(g, pid, friends(g, pid, False, ctx["hidden_bf"]),
                                               enemies(g, pid, True, ctx["hidden_bf"])))


# Mobilize — "Channel 1 rune exhausted. If you can't, draw 1."
card("Mobilize", resolve=lambda g, it: not g.channel(it.ctrl, 1, exhausted=True) and g.draw(it.ctrl, 1))

# On the Hunt — "Ready your units."
card("On the Hunt", resolve=lambda g, it: [g.ready_obj(u) for u in g.units(it.ctrl)])

# Onslaught — "Give a unit +6 might this turn. [Flow] [4]"
card("Onslaught", preds=[P_unit], flow=flow_cost("4 energy"),
     resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 6),
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid, False, ctx["hidden_bf"])))


# Overt Operation — "[Action] For each friendly unit, you may spend its buff to ready it. Then buff all friendly
# units."
def _overt(g, it):
    pid = it.ctrl
    for u in list(g.units(pid)):
        if u.buff > 0 and u in g.board:
            if g.ask(pid, "overt_spend", [True, False] if u.exhausted else [False, True], item=it, unit=u):
                _spend_buff(g, u)
                g.ready_obj(u)
    for u in list(g.units(pid)):
        g.buff(u)


card("Overt Operation", timing="action", resolve=_overt)

# Primal Strength — "[Action] Give a unit +7 might this turn."
card("Primal Strength", timing="action", preds=[P_unit],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 7),
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid, False, ctx["hidden_bf"])))


# Public Execution — "Choose a friendly unit. Kill an enemy unit with less Might than it. [Flow] [5][A][A]"
def _execution(g, it):
    f, e = g.legal(it, 0), g.legal(it, 1)
    if f is not None and e is not None and g.might(e) < g.might(f):
        g.kill([e], it.ctrl)


def _execution_choices(g, pid, ctx):
    out = []
    fs = sorted(friends(g, pid, False, ctx["hidden_bf"]), key=lambda x: g.might(x))
    for e in enemies(g, pid, False, ctx["hidden_bf"]):
        f = next((f for f in fs if g.might(f) > g.might(e)), None)
        if f is not None:
            out.append(dict(tg=(f.uid, e.uid)))
    return out


card("Public Execution", preds=[P_friend, P_enemy], flow=flow_cost("5 energy and 2 runes of any type"),
     resolve=_execution, choices=_execution_choices)

# Punch First — "[Action] Give a unit +5 might this turn."
card("Punch First", timing="action", preds=[P_unit],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 5),
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid, False, ctx["hidden_bf"])))


# Rampage — "As you play this, you may pay [Body] as an additional cost. Choose a friendly unit and an enemy unit.
# If you paid the additional cost, give the friendly unit +2 might this turn. They deal damage equal to their Mights
# to each other."
def _rampage(g, it):
    f = g.legal(it, 0)
    if f is not None and it.data.get("paid"):
        g.mod(f, 2)
    _fight(g, f, g.legal(it, 1))


def _rampage_choices(g, pid, ctx):
    fs, es = friends(g, pid, False, ctx["hidden_bf"]), enemies(g, pid, False, ctx["hidden_bf"])
    a = _duel_choices(g, pid, fs, es)[:4]
    b = _duel_choices(g, pid, fs, es, bonus=2, extra=dict(paid=True))[:4]
    return [x for pair in zip(b, a) for x in pair]


card("Rampage", preds=[P_friend, P_enemy], resolve=_rampage, choices=_rampage_choices,
     extra_cost_fn=lambda g, pid, c, ch: (0, [_BODY]) if ch.get("paid") else (0, []))


# Repulse — "[Reaction] Choose a friendly unit at a battlefield. Counter an enemy spell or ability that chooses it
# and no other friendly unit."
def _repulse_ok(g, pid, i, u):
    fr = set(x.uid for x in g.units(pid))
    return i.ctrl != pid and u.uid in i.chosen and len(i.chosen & fr) == 1


def _repulse(g, it):
    u = g.legal(it, 0)
    tgt = item_by_id(g, it.data.get("item"))
    if u is not None and tgt is not None and _repulse_ok(g, it.ctrl, tgt, u):
        g.counter(tgt)


def _repulse_choices(g, pid, ctx):
    out = []
    for i in reversed(g.chain):
        if i.ctrl == pid or i.uncounterable:
            continue
        for u in friends(g, pid, True, ctx["hidden_bf"]):
            if _repulse_ok(g, pid, i, u):
                out.append(dict(tg=(u.uid,), item=i.id))
    return out


card("Repulse", timing="reaction", preds=[P_friend_bf], resolve=_repulse, choices=_repulse_choices)


# Riposte — "[Reaction] Choose a friendly unit and a spell. Counter that spell and give that unit +might equal to
# that spell's Energy cost this turn." (printed cost, rule 206; if the spell can't be countered or has left the
# chain, the unit still gets the Might: do as much as you can, rule 055)
def _riposte(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is not None and tgt.kind == "spell":
        g.counter(tgt)
    u = g.legal(it, 0)
    if u is not None:
        g.mod(u, it.data.get("cost", 0))


def _riposte_choices(g, pid, ctx):
    out = []
    spells = sorted([i for i in reversed(g.chain) if i.kind == "spell"], key=lambda i: (i.ctrl == pid, i.uncounterable))
    for i in spells[:3]:
        for u in friends(g, pid, False, ctx["hidden_bf"])[:3]:
            out.append(dict(tg=(u.uid,), item=i.id, cost=i.card.spec["e"]))
    return out


card("Riposte", timing="reaction", preds=[P_friend], resolve=_riposte, choices=_riposte_choices)

# Sabotage — "Choose an opponent. They reveal their hand. Choose a non-unit card from it, and recycle that card."
card("Sabotage", resolve=lambda g, it: _reveal_recycle(g, it, lambda c: c.spec["type"] != "Unit", "sabotage"))

# Show of Strength — "[Reaction] Draw 1 for each of your [Mighty] units."
card("Show of Strength", timing="reaction",
     resolve=lambda g, it: g.draw(it.ctrl, sum(1 for u in g.units(it.ctrl) if g.mighty(u))))


# Showstopper — "Buff a friendly unit in your base, then move it to a battlefield."
def _showstopper(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    g.buff(u)
    if u.loc != it.data["dest"]:
        g.move([u], it.data["dest"], it.ctrl)


card("Showstopper", preds=[lambda g, it, o: P_friend(g, it, o) and o.loc == "base"], resolve=_showstopper,
     choices=lambda g, pid, ctx: sorted([dict(tg=(u.uid,), dest=b.idx) for u in friends(g, pid) if u.loc == "base"
                                         for b in g.bfs], key=lambda c: g.bfs[c["dest"]].ctrl == pid))


# Stare Down — "Choose a friendly unit and a battlefield. Move all enemy units at that battlefield with less Might
# than the chosen unit to their base. Gain 1 XP."
def _stare(g, it):
    u = g.legal(it, 0)
    if u is not None:
        m = g.might(u)
        es = [e for e in g.units(1 - it.ctrl, it.data["bf"]) if g.might(e) < m]
        if es:
            g.move(es, "base", it.ctrl)
    g.gain_xp(it.ctrl, 1)


def _stare_choices(g, pid, ctx):
    out = [dict(tg=(u.uid,), bf=b.idx) for u in friends(g, pid, False, ctx["hidden_bf"])[:3] for b in g.bfs]
    return sorted(out, key=lambda c: -sum(1 for e in g.units(1 - pid, c["bf"])
                                          if g.might(e) < g.might(g.obj(c["tg"][0]))))


card("Stare Down", preds=[P_friend], resolve=_stare, choices=_stare_choices)


# Strike Down — "Choose an equipped friendly unit. It deals damage equal to its Might to an enemy unit. Then detach
# an Equipment from it." (the detached gear stays where it is and is recalled at the next cleanup, rule 323)
def _equipped(g, o):
    return any(g.obj(x) is not None and "Equipment" in g.obj(x).spec["tags"] for x in o.attached)


def _strike_down(g, it):
    f, e = g.legal(it, 0), g.legal(it, 1)
    if f is None:
        return
    if e is not None:
        g.deal(e, max(0, g.might(f)), "unit", it.ctrl)
    eq = [g.obj(x) for x in f.attached if g.obj(x) is not None and "Equipment" in g.obj(x).spec["tags"]]
    if eq:
        x = g.ask(it.ctrl, "detach", sorted(eq, key=lambda x: (EQUIP_BONUS.get(x.cname, 0), x.uid)), item=it)
        f.attached.remove(x.uid)
        x.attached_to = None
        g.need_cleanup = True
        g.log(f"  {x} detached from {f}")


def _strike_choices(g, pid, ctx):
    out = [dict(tg=(f.uid, e.uid)) for f in friends(g, pid, False, ctx["hidden_bf"]) if _equipped(g, f)
           for e in enemies(g, pid, False, ctx["hidden_bf"])[:4]]

    def score(c):
        f, e = g.obj(c["tg"][0]), g.obj(c["tg"][1])
        return -(value(g, e) if g.might(f) >= g.might(e) - e.damage else 0)
    return sorted(out, key=score)


card("Strike Down", preds=[lambda g, it, o: P_friend(g, it, o) and _equipped(g, o), P_enemy], resolve=_strike_down,
     choices=_strike_choices)


# Void Assault — "Move a friendly unit, then move an enemy unit."
def _void_assault(g, it):
    f = g.legal(it, 0)
    if f is not None and f.loc != it.data["d1"]:
        g.move([f], it.data["d1"], it.ctrl)
    e = g.legal(it, 1)
    if e is not None and e.loc != it.data["d2"]:
        g.move([e], it.data["d2"], it.ctrl)


def _void_assault_choices(g, pid, ctx):
    out = []
    for f in friends(g, pid, False, ctx["hidden_bf"])[:2]:
        for d1 in _dests(f):
            for e in enemies(g, pid, False, ctx["hidden_bf"])[:2]:
                for d2 in _dests(e):
                    sc = (d1 not in (0, 1)) + (d2 != "base") - (d1 in (0, 1) and d1 == d2)
                    out.append((sc, len(out), dict(tg=(f.uid, e.uid), d1=d1, d2=d2)))
    out.sort(key=lambda x: (x[0], x[1]))
    return [c for _, _, c in out]


card("Void Assault", preds=[P_friend, P_enemy], resolve=_void_assault, choices=_void_assault_choices)


# Wallop — "[Action] As you play this, you may spend a buff as an additional cost. If you do, ignore this spell's
# cost. Ready a unit."
def _wallop_choices(g, pid, ctx):
    us = sorted([u for u in g.units() if (u.ctrl == pid or g.targetable(u, pid))
                 and (ctx["hidden_bf"] is None or u.loc == ctx["hidden_bf"])],
                key=lambda u: (u.ctrl != pid, not u.exhausted, -value(g, u)))[:4]
    bs = sorted([u for u in g.units(pid) if u.buff > 0], key=lambda u: value(g, u))
    out = []
    for u in us:
        out.append(dict(tg=(u.uid,)))
        if bs:
            out.append(dict(tg=(u.uid,), free=True, spend=bs[0].uid))
    return out


def _wallop_pay(g, pid, c, ch):
    if ch.get("spend"):
        _spend_buff(g, g.obj(ch["spend"]))


card("Wallop", timing="action", preds=[P_unit], choices=_wallop_choices, pay_extra=_wallop_pay,
     resolve=lambda g, it: g.legal(it, 0) is not None and g.ready_obj(g.legal(it, 0)))


# Wild Claw — "Look at the top 5 cards of your Main Deck. You may banish a unit or gear from among them and play it,
# reducing its Energy cost by [5]. Recycle the rest. Then you may do this: Empower it."
def _wild_claw(g, it):
    pid = it.ctrl
    pl = g.p[pid]
    top = pl.deck[:5]
    opts = []
    for c in top:
        if c.spec["type"] not in ("Unit", "Gear") or g.impl(c) is None:
            continue
        chs = _unit_play_choices(g, pid, c, _ctrl_locs(g, pid)) if c.spec["type"] == "Unit" else [dict(loc="base")]
        for ch in chs:
            e, reqs = _reduced_cost(g, pid, c, ch, "banish", 5)
            if g.can_pay(pid, e, reqs, dict(kind=c.spec["type"].lower(), card=c)):
                opts.append((c, ch))
    opts.sort(key=lambda x: (-(x[0].spec["e"] + 2 * x[0].spec["p"]), x[1].get("loc") != "base",
                             x[1].get("acc", False), x[0].uid))
    pick = g.ask(pid, "wild_claw", opts + [None], item=it) if opts else None
    played = None
    if pick is not None:
        c, ch = pick
        pl.deck.remove(c)
        c.zone = "banish"
        pl.banish.append(c)
        played = _play_reduced(g, pid, c, "banish", dict(ch), 5)
    rest = [c for c in top if c in pl.deck]
    for c in rest:
        pl.deck.remove(c)
    g.recycle_cards(pid, rest)
    if played is not None and played in g.board and not played.empowered:
        if g.ask(pid, "may", [True, False], item=it):
            g.empower(played)


card("Wild Claw", resolve=_wild_claw)
