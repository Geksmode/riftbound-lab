"""Batch chaos: the cards of batches/chaos.txt. See GUIDE.md.

Cards that need an engine hook are not registered here: see NEEDS_chaos.md.
"""
from itertools import combinations

from cards import *  # noqa: F401,F403
from actions import affordable, total_cost


# ====================================================================== private helpers
def _q(g, o, name, fn, data=None, **kw):
    """Queue a triggered ability of permanent o (its incarnation is remembered, see _me)."""
    d = dict(data or {})
    d["_oid"] = o.oid
    g.queue_trigger(o.ctrl, name, fn, d, src=o.uid, **kw)


def _me(g, it):
    """The source permanent of a trigger if it is still the same object on the board (rule 359.3.e.4)."""
    o = g.obj(it.src)
    if o is None or o.oid != it.data.get("_oid", o.oid):
        return None
    return o


def _same(g, uid, oid):
    o = g.obj(uid)
    return o if o is not None and o.oid == oid else None


def _cval(c):
    return c.spec["e"] + 2 * c.spec["p"] + (c.spec["might"] or 0) / 2


def _discard(g, pid, n):
    """Discard n (rule 422): the player chooses; as many as possible (rule 359.3.e.11)."""
    for _ in range(n):
        h = g.p[pid].hand
        if not h:
            return
        g.discard(pid, g.ask(pid, "discard", list(h)))


def _to_hand(g, o):
    if o is not None and o in g.board:
        g.to_zone(o, "hand")
        return True
    return False


def _draw_card(g, pid, c):
    """Draw a specific card of the Main Deck ("Draw one of them")."""
    g.draw_card(pid, c)


def _token(g, pid, name, ready=False):
    """'Play a ... token': a unit token is played like a unit (rules 185.2.a, 355.2.a), gear to base."""
    if SPEC[name]["type"] == "Gear":
        return make_token(g, name, pid, "base", ready)
    ls = token_locations(g, pid, name)
    if not ls:
        return None
    loc = g.ask(pid, "token_location", ls, token=name)
    return make_token(g, name, pid, loc, ready)


def _gold(g, pid):
    return make_token(g, "Gold", pid, "base", ready=False)       # "play a Gold gear token exhausted"


def _ensure_effect(g, key, on, fn):
    """Install a game-long listener once (used for an ability that triggers on its own death)."""
    if not any(e.get("key") == key for e in g.effects):
        g.effects.append(dict(on=on, fn=fn, key=key))


def _from_facedown(info):
    """'played' event: was the card played from face down (Hidden)?"""
    it = info.get("item")
    if it is not None:
        return it.data.get("from") == "facedown"
    return getattr(info["card"], "played_from", None) == "facedown"


def _dests(u):
    return [d for d in ("base", 0, 1) if d != u.loc]


def _deflect_ok(g, pid, u):
    return g.can_pay(pid, 0, deflect_reqs(g, pid, dict(tg=(u.uid,))))


def _pay_options(g, pid, c, src):
    """Play choices of a unit played by an effect that still pays its costs (Cursed Sarcophagus, Last Rites)."""
    return [ch for ch in unit_play_choices(g, pid, c, src) if affordable(g, pid, c, ch, src)]


def _play_paying(g, pid, c, src):
    opts = _pay_options(g, pid, c, src)
    if not opts:
        return None
    ch = g.ask(pid, "play_choice", opts, card=c)
    return play_card(g, pid, c, src, dict(ch))           # normal process of play: its costs are paid (rule 353)


def _power_ok(g, pid, c):
    return g.can_pay(pid, 0, g.power_reqs(c.spec["domains"], c.spec["p"]))


def _trash_units(g, pid, fn=None):
    return sorted([c for c in g.p[pid].trash if c.spec["type"] == "Unit" and g.impl(c) is not None
                   and (fn is None or fn(c))],
                  key=lambda c: -_cval(c))


def _return_from_trash(g, pid, typ, kind):
    cs = sorted([c for c in g.p[pid].trash if c.spec["type"] == typ], key=lambda c: -_cval(c))
    c = g.ask(pid, kind, cs)
    if c is not None:
        g.to_zone(c, "hand")


def _buff_me(n):
    return lambda g, it: _me(g, it) is not None and g.mod(_me(g, it), n)


def _won_combat(g, o, info):
    """'combat_won' event: o was in that combat and its controller won it."""
    return info["pid"] == o.ctrl and g.sd is not None and o.uid in g.sd.members


# ====================================================================== A
# Abandon — "[Reaction] Counter a spell. Return it to its owner's hand instead of putting it in their trash.
# [Predict]."
def _abandon(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is not None and tgt.kind == "spell":
        c = tgt.card
        if g.counter(tgt) and c.zone == "trash":         # a countered Flow spell is banished, not trashed
            g.to_zone(c, "hand")
    g.predict(it.ctrl, 1)


def _counter_choices(g, pid, ctx):
    its = [i for i in reversed(g.chain) if i.kind == "spell" and g.counterable(i)]
    its.sort(key=lambda i: i.ctrl == pid)
    return [dict(item=i.id) for i in its]


card("Abandon", timing="reaction", resolve=_abandon, choices=_counter_choices)


# Acceptable Losses — "[Action] Each player kills one of their gear."
def _acceptable(g, it):
    victims = []
    for pid in (g.tp, 1 - g.tp):
        gs = g.gear(pid)
        if gs:
            victims.append(g.ask(pid, "sacrifice_gear", sorted(gs, key=lambda x: (not x.token, x.spec["e"]))))
    g.kill(victims, it.ctrl)


card("Acceptable Losses", timing="action", resolve=_acceptable)


# Ancient Warmonger — "[Accelerate] I have [Assault] equal to the number of enemy units here."
card("Ancient Warmonger", accelerate=True,
     aura_kw=lambda g, src, o: ({"Assault": len(g.units(1 - o.ctrl, o.loc))}
                                if o is src and o.loc in (0, 1) else None))


# Angler Beast — "When you play me, return all units with 2 might or less to their owners' hands."
def _angler(g, o, ctx):
    def res(g_, it):
        for u in [u for u in g_.units() if g_.might(u) <= 2]:
            _to_hand(g_, u)
    _q(g, o, "Angler Beast", res)


card("Angler Beast", on_play=_angler)


# Annie, Stubborn — "When you play me, return a spell from your trash to your hand."
card("Annie, Stubborn", on_play=lambda g, o, ctx: _q(
    g, o, "Annie, Stubborn", lambda g_, it: _return_from_trash(g_, it.ctrl, "Spell", "trash_spell")))


# ====================================================================== B
# Beast Below — "When you play me, return another friendly unit and an enemy unit to their owners' hands."
def _beast_choose(g, it):
    fr = sorted([u for u in g.units(it.ctrl) if u.uid != it.src], key=lambda u: value(g, u))
    en = [u for u in enemies(g, it.ctrl) if _deflect_ok(g, it.ctrl, u)]
    if not fr and not en:
        return False
    if fr:
        f = g.ask(it.ctrl, "friend_target", fr, item=it)
        g.add_target(it, f, lambda g_, i, x: x.ctrl == i.ctrl and x.uid != i.src)
    else:
        it.targets.append((None, -1, None))
    if en:
        e = g.ask(it.ctrl, "target", en, item=it)
        if not pay_deflect(g, it, e):
            return False
        g.add_target(it, e, lambda g_, i, x: x.ctrl != i.ctrl)
    return True


def _beast(g, o, ctx):
    def res(g_, it):
        for u in [g_.legal(it, i) for i in range(len(it.targets))]:
            _to_hand(g_, u)
    _q(g, o, "Beast Below", res, choose=_beast_choose)


card("Beast Below", on_play=_beast)


# Bewitching Spirit — "When you play me, choose a player. They discard 1."
def _bewitching(g, it):
    _discard(g, g.ask(it.ctrl, "choose_player", [1 - it.ctrl, it.ctrl]), 1)


card("Bewitching Spirit", on_play=lambda g, o, ctx: _q(g, o, "Bewitching Spirit", _bewitching))


# Black Market Broker — "When you play a card from face down, play a Gold gear token exhausted."
card("Black Market Broker", on_event=lambda g, o, ev, info: (
    ev == "played" and info["pid"] == o.ctrl and _from_facedown(info)
    and _q(g, o, "Black Market Broker", lambda g_, it: _gold(g_, it.ctrl))))


# Blast Cone — "When you play this, you may move an enemy unit. When you move an enemy unit, you may exhaust this
# to [Stun] it."
def _cone_play(g, o, ctx):
    def choose(g_, it):
        opts = [(u, d) for u in enemies(g_, it.ctrl) if _deflect_ok(g_, it.ctrl, u) for d in _dests(u)]
        if not opts:
            return False
        opts.sort(key=lambda x: (x[1] != "base", -value(g_, x[0])))
        u, d = g_.ask(it.ctrl, "move_enemy", opts, item=it)
        if not pay_deflect(g_, it, u):
            return False
        g_.add_target(it, u, P_enemy)
        it.data["dest"] = d
        return True

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None and u.loc != it.data["dest"]:
            g_.move([u], it.data["dest"], it.ctrl)
    _q(g, o, "Blast Cone", res, may=True, choose=choose)


def _cone_event(g, o, ev, info):
    if ev == "move" and info["by"] == o.ctrl and info["obj"].ctrl != o.ctrl and not o.exhausted:
        u = info["obj"]

        def cost(g_, it):
            me = _me(g_, it)
            if me is None or me.exhausted:
                return False
            me.exhausted = True
            return True

        def res(g_, it):
            x = _same(g_, it.data["uid"], it.data["oid"])
            if x is not None:
                g_.stun(x, it.ctrl)
        _q(g, o, "Blast Cone (stun)", res, dict(uid=u.uid, oid=u.oid), may=True, cost=cost)


card("Blast Cone", on_play=_cone_play, on_event=_cone_event)


# Bone Skewer — "[Hidden] Choose a battlefield. An opponent reveals their hand. You may choose a unit from it. They
# play that unit to that battlefield, ignoring any and all costs. When they do, [Stun] it."
def _skewer(g, it):
    bf = it.data["bf"]
    opp = 1 - it.ctrl
    us = sorted([c for c in g.p[opp].hand if c.spec["type"] == "Unit" and g.impl(c) is not None],
                key=lambda c: (c.spec["might"] or 0))
    c = g.ask(it.ctrl, "skewer_pick", us + [None]) if us else None
    if c is None:
        return
    u = play_card(g, opp, c, "hand", dict(loc=bf, free=True), limited=True)
    if u is not None and u in g.board:
        def stun(g_, i):
            x = _same(g_, i.data["uid"], i.data["oid"])
            if x is not None:
                g_.stun(x, i.ctrl)
        g.queue_trigger(it.ctrl, "Bone Skewer (stun)", stun, dict(uid=u.uid, oid=u.oid))


def _skewer_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    bfs = [hb] if hb is not None else [0, 1]          # rule 811.1.d.2-3: from Hidden, that battlefield
    bfs = sorted(bfs, key=lambda i: -sum(g.might(u) for u in g.units(pid, i)))
    return [dict(bf=i) for i in bfs]


card("Bone Skewer", hidden=True, resolve=_skewer, choices=_skewer_choices)


# Boots of Swiftness — "[Equip] [Chaos]" ; card image: Might Bonus +2, Effect Text "[Ganking]".
card("Boots of Swiftness", equip="1 chaos rune", bonus=2, equip_kw={"Ganking": 1})


# ====================================================================== C
# Called Shot — "[Action] [Repeat] [Chaos] Look at the top 2 cards of your Main Deck. Draw one and recycle the
# other."
def _called_shot(g, it):
    pid = it.ctrl
    top = g.look(pid, g.p[pid].deck[:2])    # looking never burns out (rule 431.1.c)
    if not top:
        return
    c = g.ask(pid, "draw_pick", sorted(top, key=lambda x: -_cval(x)))
    _draw_card(g, pid, c)
    g.recycle_cards(pid, [x for x in top if x is not c])


card("Called Shot", timing="action", repeat=repeat_cost("1 chaos rune"), resolve=repeatable(_called_shot))


# Cemetery Attendant — "When you play me, return a unit from your trash to your hand."
card("Cemetery Attendant", on_play=lambda g, o, ctx: _q(
    g, o, "Cemetery Attendant", lambda g_, it: _return_from_trash(g_, it.ctrl, "Unit", "trash_unit")))


# Conscription — "You may spend 5 XP as an additional cost to play this. Choose an enemy unit at a battlefield with
# 3 might or less. If you paid the additional cost, choose any enemy unit at a battlefield instead. Take control of
# it, exhaust it, and recall it."
def _consc_pred(g, it, o):
    return P_enemy_bf(g, it, o) and (bool(it.data.get("xp")) or g.might(o) <= 3)


def _consc_choices(g, pid, ctx):
    es = enemies(g, pid, True)
    out = [dict(tg=(u.uid,)) for u in es if g.might(u) <= 3]
    if g.p[pid].xp >= 5:
        out = [dict(tg=(u.uid,), xp=5) for u in es if g.might(u) > 3] + out
    return out


def _take_control(g, pid, u):
    """Gain control of a unit (rule 188) and recall it (rule 455: not a move)."""
    u.ctrl = pid
    g.recall(u)
    g.need_cleanup = True


def _conscription(g, it):
    u = g.legal(it, 0)
    if u is not None:
        u.exhausted = True
        _take_control(g, it.ctrl, u)


card("Conscription", preds=[_consc_pred], resolve=_conscription, choices=_consc_choices)


# Corrupt Enforcer — "When I move to a battlefield, discard 1. When I win a combat, draw 1."
def _enforcer(g, o, ev, info):
    if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
        _q(g, o, "Corrupt Enforcer", lambda g_, it: _discard(g_, it.ctrl, 1))
    elif ev == "combat_won" and _won_combat(g, o, info):
        _q(g, o, "Corrupt Enforcer (win)", lambda g_, it: g_.draw(it.ctrl, 1))


card("Corrupt Enforcer", on_event=_enforcer)


# Crescent Guardian — "If you've played a spell this turn, you may pay [Chaos] as an additional cost to play me. If
# you do, I enter ready."  ("played" in a non-triggered check = finalized, rule 419.4.b)
card("Crescent Guardian",
     as_played=lambda g, pid, c: [dict()] + ([dict(cg=True)] if g.spells_played[pid] else []),
     extra_cost_fn=lambda g, pid, c, ch: (0, [frozenset({"Chaos"})]) if ch.get("cg") else (0, []),
     enter_ready=lambda g, pid, c, ch: bool(ch.get("cg")))


# Cull — "[Equip] [Chaos]" ; card image: Might Bonus +1, Effect Text "When I conquer, play a Gold gear token
# exhausted."
def _cull_effect(g, gear, unit, ev, info):
    if ev == "conquer" and info["pid"] == unit.ctrl and unit in info["units"]:
        g.queue_trigger(unit.ctrl, "Cull", lambda g_, it: _gold(g_, it.ctrl), src=unit.uid)


card("Cull", equip="1 chaos rune", bonus=1, effect_event=_cull_effect)


# Cursed Sarcophagus — "When you play this, banish all units from your trash. [E]: Play a unit banished with this.
# (You must pay its costs.)"  (rule 427.3: cards banished by the same object)
def _sarc_play(g, o, ctx):
    def res(g_, it):
        us = [c for c in g_.p[it.ctrl].trash if c.spec["type"] == "Unit"]
        for c in us:
            g_.to_zone(c, "banish")
        me = _me(g_, it)
        if me is not None:
            me.sarc = (me.oid, [c.uid for c in us])
    _q(g, o, "Cursed Sarcophagus", res)


def _sarc_cards(g, o):
    s = getattr(o, "sarc", None)
    if not s or s[0] != o.oid:
        return []
    return [c for pl in g.p for c in pl.banish if c.uid in s[1]]


def _sarc_choices(g, pid, o):
    return [dict(c=c.uid) for c in sorted(_sarc_cards(g, o), key=lambda c: -_cval(c)) if _pay_options(g, pid, c, "banish")]


def _sarc_res(g, it):
    o = g.obj(it.src)
    c = next((x for pl in g.p for x in pl.banish if x.uid == it.data.get("c")), None)
    if c is not None and (o is None or c in _sarc_cards(g, o)):
        _play_paying(g, it.ctrl, c, "banish")


card("Cursed Sarcophagus", on_play=_sarc_play,
     abilities=[ability("Unearth", exhaust=True, choices=_sarc_choices, resolve=_sarc_res)])


# ====================================================================== D
# Decree of Discord — "Return any number of enemy Order units with total Might 5 or less to their owners' hands."
def _order_enemy(g, it, o):
    return P_enemy(g, it, o) and "Order" in o.spec["domains"]


def _dod_choices(g, pid, ctx):
    es = cap(g, [u for u in enemies(g, pid) if "Order" in u.spec["domains"]], 6, pid)
    out = []
    for k in range(len(es), 0, -1):
        for comb in combinations(es, k):
            if sum(g.might(u) for u in comb) <= 5:
                out.append(dict(tg=tuple(u.uid for u in comb)))
    out.sort(key=lambda c: -sum(value(g, g.obj(x)) for x in c["tg"]))
    return cap(g, out, 7, pid) + [dict(tg=())]


def _discord(g, it):
    us = [u for u in (g.legal(it, i) for i in range(len(it.targets))) if u is not None]
    if sum(g.might(u) for u in us) > 5:
        # rule 355.11.b: the group no longer fulfils the restriction: choose a subset of the original targets
        subs = [list(c) for k in range(len(us), 0, -1) for c in combinations(us, k) if sum(g.might(u) for u in c) <= 5]
        subs.sort(key=lambda s: -sum(value(g, u) for u in s))
        us = g.ask(it.ctrl, "discord_subset", subs) or []
    for u in us:
        _to_hand(g, u)


card("Decree of Discord", preds=[_order_enemy], resolve=_discord, choices=_dod_choices)


# Diana, No Longer Human — "[Ambush] When you play a spell, give me +2 might this turn."
card("Diana, No Longer Human", ambush=True, on_event=lambda g, o, ev, info: (
    ev == "played" and info["pid"] == o.ctrl and info["card"].spec["type"] == "Spell"
    and _q(g, o, "Diana, No Longer Human", _buff_me(2))))


# Doran's Ring — "[Equip] [Chaos]" ; card image: Might Bonus +1, Effect Text "When I conquer, discard 1, then draw
# 1."
def _doran_effect(g, gear, unit, ev, info):
    if ev == "conquer" and info["pid"] == unit.ctrl and unit in info["units"]:
        g.queue_trigger(unit.ctrl, "Doran's Ring", lambda g_, it: (_discard(g_, it.ctrl, 1), g_.draw(it.ctrl, 1)),
                        src=unit.uid)


card("Doran's Ring", equip="1 chaos rune", bonus=1, effect_event=_doran_effect)


# Downwell — "Return all units and gear to their owners' hands."
def _downwell(g, it):
    for o in [x for x in g.board if x.spec["type"] in ("Unit", "Gear")]:
        _to_hand(g, o)


card("Downwell", resolve=_downwell)


# Draven, Audacious — "[Deflect] The first time I win a combat each turn, you score 1 point. When I die in combat,
# choose an opponent. They score 1 point."  (points not from Conquer ignore the Final Point rule, 471.1.a.1)
def _draven_die(g, eff, info):
    inf = info["info"]
    if inf["name"] == "Draven, Audacious" and inf["in_combat"]:
        # triggers on its own death (rule 428.1.a.1.b), like a Deathknell but it isn't one (Karthus doesn't double)
        g.queue_trigger(inf["ctrl"], "Draven, Audacious (death)",
                        lambda g_, it: g_.gain_point(1 - it.ctrl, "Draven, Audacious"))


def _draven(g, o, ev, info):
    _ensure_effect(g, "draven_die", "die", _draven_die)
    if ev == "combat_won" and _won_combat(g, o, info):
        key = ("draven_win", o.uid, o.oid, g.turn_no)
        if g.stats[key]:
            return
        g.stats[key] = 1
        _q(g, o, "Draven, Audacious", lambda g_, it: g_.gain_point(it.ctrl, "Draven, Audacious"))


card("Draven, Audacious", kw={"Deflect": 1}, on_event=_draven,
     on_play=lambda g, o, ctx: _ensure_effect(g, "draven_die", "die", _draven_die))


# ====================================================================== E
# Edge of Night — "[Hidden] When you play this from face down, attach it to a unit you control here. [Equip]
# [Chaos]" ; card image: Might Bonus +2.
def _edge_play(g, o, ctx):
    if ctx.get("src") != "facedown":
        return
    here = ctx.get("hidden_bf")

    def res(g_, it):
        u, me = g_.legal(it, 0), _me(g_, it)
        if u is not None and me is not None:
            attach(g_, me, u)
    _q(g, o, "Edge of Night", res,
       choose=trig_target(lambda g_, it: [u for u in friends(g_, it.ctrl) if u.loc == here],
                          lambda g_, it, u: u.ctrl == it.ctrl and u.loc == here, kind="equip_target"))


card("Edge of Night", hidden=True, equip="1 chaos rune", bonus=2, on_play=_edge_play)


# Ember Monk — "When you play a card from [Hidden], give me +2 might this turn." (pas le mot-clé Hidden : il ne se
# cache pas, règle 811.1 ; le CSV range « Hidden » dans keywords parce que le texte le mentionne, image OGN 167 vérifiée)
card("Ember Monk", on_event=lambda g, o, ev, info: (
    ev == "played" and info["pid"] == o.ctrl and _from_facedown(info) and _q(g, o, "Ember Monk", _buff_me(2))))


# Evelynn, Entrancing — "[Hidden] [Backline] When you play me from face down on your turn, you may move an enemy
# unit at a different location to my battlefield."  (target chosen freely: rule 811.1.d.2)
def _evelynn(g, o, ctx):
    if ctx.get("src") != "facedown" or g.tp != o.ctrl:
        return

    def res(g_, it):
        u, me = g_.legal(it, 0), _me(g_, it)
        if u is not None and me is not None and me.loc in (0, 1) and u.loc != me.loc:
            g_.move([u], me.loc, it.ctrl)
    _q(g, o, "Evelynn, Entrancing", res, may=True,
       choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc != o.loc],
                          lambda g_, it, u: u.ctrl != it.ctrl, deflect=True))


card("Evelynn, Entrancing", hidden=True, kw={"Backline": 1}, on_play=_evelynn)


# Evershade Stalker — "When you play me, discard 1, then draw 1."
card("Evershade Stalker", on_play=lambda g, o, ctx: _q(
    g, o, "Evershade Stalker", lambda g_, it: (_discard(g_, it.ctrl, 1), g_.draw(it.ctrl, 1))))


# Existential Dread — "[Action] [Repeat] [2] [Stun] an attacking enemy unit. If it's already stunned, return it to
# its owner's hand instead."
def _dread(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    if u.stunned:
        _to_hand(g, u)
    else:
        g.stun(u, it.ctrl)


def _attacking_enemy(g, it, o):
    return P_enemy(g, it, o) and o.desig == "att"


card("Existential Dread", timing="action", repeat=repeat_cost("2 energy"), preds=[_attacking_enemy],
     resolve=repeatable(_dread),
     choices=lambda g, pid, ctx: tg_choices([u for u in enemies(g, pid) if u.desig == "att"]))


# ====================================================================== F
# Factory Recall — "[Action] Return a gear to its owner's hand."
card("Factory Recall", timing="action", preds=[P_gear], resolve=lambda g, it: _to_hand(g, g.legal(it, 0)),
     choices=lambda g, pid, ctx: tg_choices(sorted(g.gear(), key=lambda x: (x.ctrl == pid, -x.spec["e"]))))


# Fading Memories — "Give a unit at a battlefield or a gear [Temporary]."
def _fading_pred(g, it, o):
    return (o.spec["type"] == "Unit" and o.loc in (0, 1)) or o.spec["type"] == "Gear"


card("Fading Memories", preds=[_fading_pred],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.grant(g.legal(it, 0), "Temporary", 1, None),
     choices=lambda g, pid, ctx: tg_choices(
         enemies(g, pid, True) + sorted([x for x in g.gear() if x.ctrl != pid], key=lambda x: -x.spec["e"])
         + g.gear(pid) + friends(g, pid, True)))


# Fae Porter — "When I move to a battlefield, you may pay [Chaos] to move a unit you control to the same
# battlefield."
def _porter(g, o, ev, info):
    if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
        dest = info["to"]

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None and u.loc != dest:
                g_.move([u], dest, it.ctrl)
        _q(g, o, "Fae Porter", res, may=True, cost=may_pay(0, lambda g_, it: [frozenset({"Chaos"})]),
           choose=trig_target(lambda g_, it: [u for u in friends(g_, it.ctrl) if u.loc != dest],
                              P_friend, kind="friend_target"))


card("Fae Porter", on_event=_porter)


# Fight or Flight — "[Hidden] [Action] Move a unit from a battlefield to its base."
card("Fight or Flight", hidden=True, timing="action", preds=[P_unit_bf],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.move([g.legal(it, 0)], "base", it.ctrl),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])
                                            + friends(g, pid, True, ctx["hidden_bf"])))


# Fizz, Trickster — "When you play me, you may play a spell from your trash with Energy cost no more than [3],
# ignoring its Energy cost. Recycle that spell after you play it. (You must still pay its Power cost.)"
# A spell countered after being played this way was not played (rule 425.1.b): it goes to the trash (425.1.a.1).
def _spell_play_options(g, pid, c, src):
    im = g.impl(c)
    if im is None:
        return []
    if full_choices(g, pid) and im.all_choices is not None:
        base = im.all_choices(g, pid, dict(hidden_bf=None, card=c)) or [dict()]
    else:
        base = cap(g, (im.choices(g, pid, dict(hidden_bf=None, card=c)) if im.choices else [dict()]), 8, pid)
    out = []
    for ch in base:
        vs = [ch]
        if im.repeat:
            vs += [dict(ch, rep=True, tg2=ch2.get("tg", ())) for ch2 in cap(g, base, 3, pid)]
        for v in vs:
            v = dict(v, ignore_energy=True)
            if affordable(g, pid, c, v, src):
                out.append((c, v))
    return out


def _fizz(g, it):
    pid = it.ctrl
    opts = []
    for c in sorted([c for c in g.p[pid].trash if c.spec["type"] == "Spell" and c.spec["e"] <= 3],
                    key=lambda c: -_cval(c)):
        opts += _spell_play_options(g, pid, c, "trash")
    pick = g.ask(pid, "fizz_pick", opts + [None]) if opts else None
    if pick is None:
        return
    c, ch = pick
    e, reqs = total_cost(g, pid, c, ch, "trash")
    if not g.pay(pid, e, reqs, dict(kind="spell", card=c)):
        return
    sp = play_card(g, pid, c, "trash", dict(ch), limited=True)
    if sp is None:
        return
    iid = sp.id

    def recycle(g_, eff, info):
        if info.get("item") is not None and info["item"].id == iid:
            g_.effects.remove(eff)
            if c in g_.p[c.owner].trash:
                g_.recycle_cards(c.owner, [c])      # "Recycle that spell after you play it"
    g.effects.append(dict(on="played", fn=recycle, dur="turn"))


card("Fizz, Trickster", on_play=lambda g, o, ctx: _q(g, o, "Fizz, Trickster", _fizz))


# Flash — "[Reaction] Move up to 2 friendly units to base."
def _flash_choices(g, pid, ctx):
    fr = cap(g, friends(g, pid, True), 4, pid)
    out = [dict(tg=(a.uid, b.uid)) for a, b in combinations(fr, 2)] + [dict(tg=(u.uid,)) for u in fr]
    return cap(g, out, 7, pid) + [dict(tg=())]


def _flash(g, it):
    us = [u for u in (g.legal(it, i) for i in range(len(it.targets))) if u is not None and u.loc != "base"]
    if us:
        g.move(us, "base", it.ctrl)


card("Flash", timing="reaction", preds=[P_friend], resolve=_flash, choices=_flash_choices)


# Forgotten Relic — "When you play this or at the start of your Beginning Phase, [Burn 1]. When you burn a unit this
# way, do this: Give a friendly unit +might equal to the burned card's Might this turn."
def _relic_trigger(g, o):
    def res(g_, it):
        for c in g_.burn(it.ctrl, 1):
            if c.spec["type"] == "Unit":
                def give(g2, it2):
                    u = g2.legal(it2, 0)
                    if u is not None:
                        g2.mod(u, it2.data["m"])
                g_.queue_trigger(it.ctrl, "Forgotten Relic (unit burned)", give, dict(m=c.spec["might"] or 0),
                                 choose=trig_target(lambda g2, it2: friends(g2, it2.ctrl), P_friend,
                                                    kind="friend_target"))
    _q(g, o, "Forgotten Relic", res)


card("Forgotten Relic", on_play=lambda g, o, ctx: _relic_trigger(g, o),
     on_event=lambda g, o, ev, info: ev == "beginning_start" and info["pid"] == o.ctrl and _relic_trigger(g, o))


# ====================================================================== G
# Gust — "[Reaction] Return a unit at a battlefield with 3 might or less to its owner's hand."
def _small_bf(g, it, o):
    return P_unit_bf(g, it, o) and g.might(o) <= 3


card("Gust", timing="reaction", preds=[_small_bf], resolve=lambda g, it: _to_hand(g, g.legal(it, 0)),
     choices=lambda g, pid, ctx: tg_choices([u for u in enemies(g, pid, True) if g.might(u) <= 3]
                                            + [u for u in friends(g, pid, True) if g.might(u) <= 3]))


# Gust Monk — "You may pay [1] as an additional cost to play me. When you play me, if you paid the additional
# cost, banish a card from any trash to give a unit [Assault 2] this turn."
def _gust_monk(g, o, ctx):
    if not ctx.get("gm"):
        return

    def cost(g_, it):                         # "[banish X] to [give Y]": a cost within the instruction (355.10.c.1)
        cs = [c for p in (1 - it.ctrl, it.ctrl) for c in g_.p[p].trash]
        if not cs:
            return False
        c = g_.ask(it.ctrl, "banish_from_trash", sorted(cs, key=lambda c: (c.owner == it.ctrl, -_cval(c))))
        g_.to_zone(c, "banish")
        return True

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.grant(u, "Assault", 2, "turn")
    _q(g, o, "Gust Monk", res, cost=cost,
       choose=trig_target(lambda g_, it: friends(g_, it.ctrl) + enemies(g_, it.ctrl), P_unit, kind="friend_target",
                          deflect=True))


card("Gust Monk", on_play=_gust_monk, as_played=lambda g, pid, c: [dict(), dict(gm=True)],
     extra_cost_fn=lambda g, pid, c, ch: (1, []) if ch.get("gm") else (0, []))


# ====================================================================== H
# Hard Bargain — "[Reaction] [Repeat] [2] Counter a spell unless its controller pays [2]."
# The repetition may choose another spell (rule 820.2.a), so it is modelled with its own choice (rep2/item2) priced
# by extra_cost_fn instead of the generic Impl.repeat (which only re-chooses unit targets).
def _bargain_once(g, it, iid):
    tgt = item_by_id(g, iid)
    if tgt is None or tgt.kind != "spell" or not g.counterable(tgt):
        return
    p = tgt.ctrl
    if g.can_pay(p, 2, []) and g.ask(p, "pay_or_countered", [True, False], item=it):
        g.pay(p, 2, [])
        return
    g.counter(tgt)


def _bargain(g, it):
    _bargain_once(g, it, it.data.get("item"))
    if it.data.get("rep2"):
        _bargain_once(g, it, it.data.get("item2"))


def _bargain_choices(g, pid, ctx):
    base = _counter_choices(g, pid, ctx)
    reps = [dict(a, rep2=True, item2=b["item"]) for a in cap(g, base, 2, pid) for b in cap(g, base, 2, pid)]
    return base[:3] + reps + base[3:]                 # repeat variants within the first 8 choices


card("Hard Bargain", timing="reaction", resolve=_bargain, choices=_bargain_choices,
     opt_parts=lambda g, pid, c, ch: [("rep", 2, [])] if ch.get("rep2") else [])


# Harpoon Squad — "When I move from a battlefield, give me +2 might this turn."
card("Harpoon Squad", on_event=lambda g, o, ev, info: (
    ev == "move" and info["obj"] is o and info["frm"] in (0, 1) and _q(g, o, "Harpoon Squad", _buff_me(2))))


# Heedless Resurrection — "[Reaction] As an additional cost to play this, kill a friendly unit. Play a unit from
# your trash that costs no more Energy and no more Power than the killed unit, ignoring its cost."
# (look-back at the killed unit's cost, rule 359.3.e.13; a token costs 0, rule 185.3.a.1)
def _heedless(g, it):
    ki = it.data.get("killed_info")
    if not ki:
        return
    cands = _trash_units(g, it.ctrl, lambda c: c.spec["e"] <= ki["e"] and c.spec["p"] <= ki["p"])
    pick = g.ask(it.ctrl, "resurrect_pick", cands)
    if pick is not None:
        play_unit_free(g, it.ctrl, pick, "trash")


card("Heedless Resurrection", timing="reaction", resolve=_heedless,
     choices=lambda g, pid, ctx: [dict(kill=u.uid) for u in sorted(g.units(pid), key=lambda u: value(g, u))])


# ====================================================================== I
# Illaoi, Prophet of the Great Kraken — "When you play me or when I score, play a 1 might Tentacle unit token.
# I have +1 might for each token unit you control."  ("I score": I'm among the units of a conquer or hold, 471.2)
def _illaoi_token(g, o):
    _q(g, o, "Illaoi, Prophet of the Great Kraken", lambda g_, it: _token(g_, it.ctrl, "Tentacle"))


card("Illaoi, Prophet of the Great Kraken", on_play=lambda g, o, ctx: _illaoi_token(g, o),
     on_event=lambda g, o, ev, info: (ev in ("conquer", "hold") and info["pid"] == o.ctrl and o in info["units"]
                                      and _illaoi_token(g, o)),
     might_mod=lambda g, o: len([u for u in g.units(o.ctrl) if u.token]))


# Insightful Investigator — "When you play me, choose an opponent. They reveal their hand. You may pay 2 XP to
# choose a card from their hand. If you do, they discard that card and draw 1."
def _investigator(g, it):
    opp = 1 - it.ctrl
    hand_ = g.p[opp].hand
    if not hand_ or g.p[it.ctrl].xp < 2:
        return
    c = g.ask(it.ctrl, "investigator_pick", sorted(hand_, key=lambda x: -_cval(x)) + [None])
    if c is None or not g.spend_xp(it.ctrl, 2):
        return
    g.discard(opp, c)
    g.draw(opp, 1)


card("Insightful Investigator", on_play=lambda g, o, ctx: _q(g, o, "Insightful Investigator", _investigator))


# Invert Timelines — "Each player discards their hand, then draws 4."
def _invert(g, it):
    for pid in (g.tp, 1 - g.tp):
        for c in list(g.p[pid].hand):
            g.discard(pid, c)
    for pid in (g.tp, 1 - g.tp):
        g.draw(pid, 4)


card("Invert Timelines", resolve=_invert)


# Isolate — "Move an enemy unit from a battlefield to its base. Then, if there's an enemy unit alone at that
# battlefield, draw 1."
def _isolate(g, it):
    u = g.legal(it, 0)
    if u is None:
        return                                        # "that battlefield" refers to the target (359.3.e.14.a)
    bf = u.loc
    g.move([u], "base", it.ctrl)
    if any(g.alone(e) for e in g.units(1 - it.ctrl, bf)):
        g.draw(it.ctrl, 1)


card("Isolate", preds=[P_enemy_bf], resolve=_isolate,
     choices=lambda g, pid, ctx: tg_choices(sorted(enemies(g, pid, True),
                                                   key=lambda u: (len(g.units(u.ctrl, u.loc)) != 2, -value(g, u)))))


# ====================================================================== J
# Jae Medarda — "When you choose me with a spell, draw 1."
card("Jae Medarda", on_event=lambda g, o, ev, info: (
    ev == "chosen" and info["obj"] is o and info["item"].kind == "spell" and info["item"].ctrl == o.ctrl
    and _q(g, o, "Jae Medarda", lambda g_, it: g_.draw(it.ctrl, 1))))


# Jinx, Rebel — "When you discard one or more cards, ready me and give me +1 might this turn."
# The discards of one action (Invert Timelines) trigger once: a pending trigger of this Jinx is not doubled.
def _jinx(g, o, ev, info):
    if ev == "discard" and info["pid"] == o.ctrl:
        if any(t.src == o.uid and t.name == "Jinx, Rebel" for t in g.trigq):
            return

        def res(g_, it):
            me = _me(g_, it)
            if me is not None:
                g_.ready_obj(me)
                g_.mod(me, 1)
        _q(g, o, "Jinx, Rebel", res)


card("Jinx, Rebel", on_event=_jinx)


# ====================================================================== K
# Kayn, Unleashed — "[Ganking] If I have moved twice this turn, I don't take damage."
# From his second move this turn all damage to him is prevented (Game.deal and combat damage use Obj.prevent,
# which is cleared at the end of the turn).
def _kayn(g, o, ev, info):
    if ev == "move" and info["obj"] is o:
        key = ("kayn_moves", o.uid, o.oid, g.turn_no)
        g.stats[key] += 1
        if g.stats[key] >= 2 and o.prevent < 10 ** 6:
            o.prevent += 10 ** 6


card("Kayn, Unleashed", kw={"Ganking": 1}, on_event=_kayn)


# Kha'Zix, Mutating Horror — "[Ambush] When I attack or defend, if an enemy unit is alone here, give me +2 might
# this turn and gain 2 XP."  (intervening if: checked again on resolution)
def _enemy_alone_here(g, o):
    return o.loc in (0, 1) and any(g.alone(e) for e in g.units(1 - o.ctrl, o.loc))


def _khazix(g, o, ev, info):
    if ev in ("attack", "defend") and info["obj"] is o and _enemy_alone_here(g, o):
        def res(g_, it):
            me = _me(g_, it)
            if me is not None and _enemy_alone_here(g_, me):
                g_.mod(me, 2)
                g_.gain_xp(it.ctrl, 2)
        _q(g, o, "Kha'Zix, Mutating Horror", res)


card("Kha'Zix, Mutating Horror", ambush=True, on_event=_khazix)


# Kharox — "[Empower] [6][Chaos][Chaos] When I become [Empowered], choose an opponent. They [Burn 3]. Then you may
# do this: Choose a unit in their trash and play it, ignoring its cost."
def _kharox_res(g, it):
    opp = 1 - it.ctrl
    g.burn(opp, 3)
    cands = _trash_units(g, opp)
    pick = g.ask(it.ctrl, "kharox_pick", cands + [None]) if cands else None
    if pick is not None:
        play_unit_free(g, it.ctrl, pick, "trash")


card("Kharox", empower="6 energy and 2 chaos runes",
     on_event=lambda g, o, ev, info: ev == "empowered" and info["obj"] is o and _q(g, o, "Kharox", _kharox_res))


# Kinkou Lifeblade — "[Empower] [2] [Empowered] I have +1 might and [Ganking]."
card("Kinkou Lifeblade", empower="2 energy", might_if=[(when_empowered, 1)], kw_if=[(when_empowered, {"Ganking": 1})])


# Kog'Maw, Caustic — "[Deathknell] Deal 4 to all units at my battlefield."
def _kogmaw(g, it):
    loc = it.data["info"]["loc"]
    if loc in (0, 1):
        for u in g.units(loc=loc):
            g.deal(u, 4, "ability", it.ctrl)


card("Kog'Maw, Caustic", deathknell=_kogmaw)


# ====================================================================== L
# Last Rites — "[Equip] [Chaos], Recycle 2 cards from your trash" ; card image: Might Bonus +2, Effect Text "When I
# conquer or hold, you may play a unit from your trash. (You still pay its costs.)"
def _rites_extra(g, pid, o, ch):
    picks = []
    for _ in range(2):
        rest = [c for c in g.p[pid].trash if c not in picks]
        if not rest:
            break
        picks.append(g.ask(pid, "recycle_from_trash", sorted(rest, key=_cval)))
    g.recycle_cards(pid, picks)


def _rites_equip():
    ab = equip_ability_cost("1 chaos rune")
    ab["can"] = lambda g, pid, o: len(g.p[pid].trash) >= 2
    ab["can_extra"] = lambda g, pid, o: len(g.p[pid].trash) >= 2
    ab["extra_cost"] = _rites_extra
    return ab


def _rites_effect(g, gear, unit, ev, info):
    if ev in ("conquer", "hold") and info["pid"] == unit.ctrl and unit in info["units"]:
        def res(g_, it):
            cands = [c for c in _trash_units(g_, it.ctrl) if _pay_options(g_, it.ctrl, c, "trash")]
            pick = g_.ask(it.ctrl, "rites_pick", cands + [None]) if cands else None
            if pick is not None:
                _play_paying(g_, it.ctrl, pick, "trash")
        g.queue_trigger(unit.ctrl, "Last Rites", res, src=unit.uid)


card("Last Rites", abilities=[_rites_equip()], bonus=2, effect_event=_rites_effect)


# Loyal Pup — "When you defend at a battlefield, you may move me there."
def _pup(g, o, ev, info):
    if ev == "combat_start" and info["sd"].defender == o.ctrl and o.loc != info["bf"]:
        bf = info["bf"]

        def res(g_, it):
            me = _me(g_, it)
            if me is not None and me.loc != bf:
                g_.move([me], bf, it.ctrl)
        _q(g, o, "Loyal Pup", res, may=True)


card("Loyal Pup", on_event=_pup)


# Lunar Boon — "[Reaction] Discard 1, then draw 2."
card("Lunar Boon", timing="reaction", resolve=lambda g, it: (_discard(g, it.ctrl, 1), g.draw(it.ctrl, 2)))


# ====================================================================== M
# Maddened Marauder — "[Tank] When you play me, move a unit from a battlefield to its base."
def _marauder(g, o, ctx):
    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.move([u], "base", it.ctrl)
    _q(g, o, "Maddened Marauder", res,
       choose=trig_target(lambda g_, it: enemies(g_, it.ctrl, True) + friends(g_, it.ctrl, True), P_unit_bf,
                          deflect=True))


card("Maddened Marauder", kw={"Tank": 1}, on_play=_marauder)


# Megatusk — "[Ganking] Spend 3 XP: Give your units here [Ganking] this turn."
def _megatusk(g, it):
    me = g.obj(it.src)
    if me is None:
        return
    for u in g.units(it.ctrl, me.loc):
        g.grant(u, "Ganking", 1, "turn")


card("Megatusk", kw={"Ganking": 1}, abilities=[ability("Stampede", xp=3, resolve=_megatusk)])


# Mel, Defiant Soul — "[Empower] — Discard a spell. When I become [Empowered], banish an enemy unit at a
# battlefield with 3 might or less."
def _mel_spells(g, pid):
    seen, out = set(), []
    for c in sorted(g.p[pid].hand, key=_cval):
        if c.spec["type"] == "Spell" and c.cname not in seen:
            seen.add(c.cname)
            out.append(c)
    return out


def _mel_discard(g, pid, o, ch):
    c = next((x for x in g.p[pid].hand if x.uid == ch.get("disc")), None)
    if c is not None:
        g.discard(pid, c)


def _mel_event(g, o, ev, info):
    if ev == "empowered" and info["obj"] is o:
        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.to_zone(u, "banish")
        _q(g, o, "Mel, Defiant Soul", res,
           choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl, True) if g_.might(u) <= 3],
                              lambda g_, it, u: P_enemy_bf(g_, it, u) and g_.might(u) <= 3, deflect=True))


card("Mel, Defiant Soul", on_event=_mel_event, abilities=[ability(
    "Empower", can=lambda g, pid, o: not o.empowered and bool(_mel_spells(g, pid)),
    choices=lambda g, pid, o: [dict(disc=c.uid) for c in _mel_spells(g, pid)], extra_cost=_mel_discard,
    resolve=lambda g, it: g.obj(it.src) is not None and g.empower(g.obj(it.src)))])


# Minah Swiftfoot — "When I move to a battlefield, choose one — Each player discards 1. / Each player draws 1."
def _minah(g, o, ev, info):
    if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
        def choose(g_, it):
            it.data["mode"] = g_.ask(it.ctrl, "minah_mode", ["draw", "discard"], item=it)
            return True

        def res(g_, it):
            for p in (g_.tp, 1 - g_.tp):
                if it.data["mode"] == "draw":
                    g_.draw(p, 1)
                else:
                    _discard(g_, p, 1)
        _q(g, o, "Minah Swiftfoot", res, choose=choose)


card("Minah Swiftfoot", on_event=_minah)


# Mindsplitter — "When you play me, choose an opponent. They reveal their hand. Choose a card from it, and they
# discard that card."
def _mindsplitter(g, it):
    opp = 1 - it.ctrl
    if g.p[opp].hand:
        g.discard(opp, g.ask(it.ctrl, "mindsplitter_pick", sorted(g.p[opp].hand, key=lambda x: -_cval(x))))


card("Mindsplitter", on_play=lambda g, o, ctx: _q(g, o, "Mindsplitter", _mindsplitter))


# Mister Root — "[Accelerate] When I move to a battlefield, gain 2 XP."
card("Mister Root", accelerate=True, on_event=lambda g, o, ev, info: (
    ev == "move" and info["obj"] is o and info["to"] in (0, 1)
    and _q(g, o, "Mister Root", lambda g_, it: g_.gain_xp(it.ctrl, 2))))


# Morbid Return — "[Action] Return a unit from your trash to your hand."  (targets a card in the trash, 355.9.a)
def _morbid(g, it):
    c = next((x for x in g.p[it.ctrl].trash if x.uid == it.data.get("c")), None)
    if c is not None:
        g.to_zone(c, "hand")


card("Morbid Return", timing="action", resolve=_morbid,
     choices=lambda g, pid, ctx: [dict(c=c.uid) for c in _trash_units(g, pid)])


# ====================================================================== O / P
# Overzealous Fan — "When I defend, you may kill me to move an attacking unit to its base."
def _fan(g, o, ev, info):
    if ev == "defend" and info["obj"] is o:
        def cost(g_, it):
            me = _me(g_, it)
            if me is None:
                return False
            g_.kill([me], it.ctrl, cost=True)
            return True

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.move([u], "base", it.ctrl)
        _q(g, o, "Overzealous Fan", res, may=True, cost=cost,
           choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.desig == "att"],
                              lambda g_, it, u: u.desig == "att", deflect=True))


card("Overzealous Fan", on_event=_fan)


# Pack of Wonders — "[Hidden] [E]: Return another friendly gear, unit, or [Hidden] card to its owner's hand."
def _pack_choices(g, pid, o):
    out = [dict(tg=(x.uid,)) for x in sorted(g.board, key=lambda x: -value(g, x))
           if x.ctrl == pid and x is not o and x.spec["type"] in ("Unit", "Gear")]
    out += [dict(fd=b.idx, fdu=c.uid) for b in g.bfs for c in b.facedowns if c.owner == pid]
    return out


def _pack(g, it):
    if it.data.get("fd") is not None:
        b = g.bfs[it.data["fd"]]
        for c in list(b.facedowns):
            if c.uid == it.data.get("fdu", c.uid) and c.owner == it.ctrl:
                g.to_zone(c, "hand")
                break
        return
    _to_hand(g, g.legal(it, 0))


# Pack of Wonders n'a pas le mot-clé Hidden (il le mentionne seulement, image OGN 181 vérifiée) : ne se cache pas (811.1).
card("Pack of Wonders", abilities=[ability(
    "Return", exhaust=True, choices=_pack_choices, resolve=_pack,
    preds=[lambda g, it, x: x.ctrl == it.ctrl and x.uid != it.src])])


# Possession — "[Action] Choose an enemy unit at a battlefield. Take control of it and recall it."
card("Possession", timing="action", preds=[P_enemy_bf],
     resolve=lambda g, it: g.legal(it, 0) is not None and _take_control(g, it.ctrl, g.legal(it, 0)),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True)))


# Pyke, Returned — "[Hidden] [Backline] Once each turn, when an enemy unit dies while I'm at a battlefield, play a
# Gold gear token exhausted."
def _pyke(g, o, ev, info):
    if ev == "die" and o.loc in (0, 1):
        inf = info["info"]
        if inf["ctrl"] != o.ctrl and inf["spec"]["type"] == "Unit":
            key = ("pyke", o.uid, o.oid, g.turn_no)
            if g.stats[key]:
                return
            g.stats[key] = 1
            _q(g, o, "Pyke, Returned", lambda g_, it: _gold(g_, it.ctrl))


card("Pyke, Returned", hidden=True, kw={"Backline": 1}, on_event=_pyke)


# ====================================================================== R
# Ravenbloom Prefect — "When an opponent plays a gear, you may banish me to banish it."
def _ravenbloom(g, o, ev, info):
    if ev == "played" and info["pid"] != o.ctrl and info["card"].spec["type"] == "Gear":
        gear = info["card"]

        def cost(g_, it):
            me = _me(g_, it)
            if me is None:
                return False
            g_.to_zone(me, "banish")
            return True

        def res(g_, it):
            x = _same(g_, it.data["uid"], it.data["oid"])
            if x is not None:
                g_.to_zone(x, "banish")
        _q(g, o, "Ravenbloom Prefect", res, dict(uid=gear.uid, oid=gear.oid), may=True, cost=cost)


card("Ravenbloom Prefect", on_event=_ravenbloom)


# Rebuke — "[Action] Return a unit at a battlefield to its owner's hand."
card("Rebuke", timing="action", preds=[P_unit_bf], resolve=lambda g, it: _to_hand(g, g.legal(it, 0)),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True) + friends(g, pid, True)))


# Rhasa the Sunderer — "I cost [1] less for each card in your trash."
card("Rhasa the Sunderer", cost_mod=lambda g, pid, c, ch: (len(g.p[pid].trash), 0))


# Ride The Wind — "[Action] Move a friendly unit and ready it."
def _ride(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    if u.loc != it.data["dest"]:
        g.move([u], it.data["dest"], it.ctrl)
    g.ready_obj(u)


def _ride_choices(g, pid, ctx):
    out = [dict(tg=(u.uid,), dest=d) for u in friends(g, pid) for d in _dests(u)]
    out.sort(key=lambda c: (not (c["dest"] in (0, 1) and g.bfs[c["dest"]].ctrl != pid), -value(g, g.obj(c["tg"][0]))))
    return out


card("Ride The Wind", timing="action", preds=[P_friend], resolve=_ride, choices=_ride_choices, max_choices=10)


# ====================================================================== S
# Scryer's Bloom — "This enters exhausted. Kill this, [1], [E]: [Predict 2], then draw 1. Gain 1 XP."
card("Scryer's Bloom", enter_exhausted=True, abilities=[ability(
    "Scry", cost="1 energy", exhaust=True, kill_self=True,
    resolve=lambda g, it: (g.predict(it.ctrl, 2), g.draw(it.ctrl, 1), g.gain_xp(it.ctrl, 1)))])


# Seal of Discord — "[E]: [Reaction] — [Add] [Chaos]."
card("Seal of Discord", add=[add_ability("1 chaos rune")])


# Shadow Order Disciple — "When I move, you may [Burn 1] to give me +1 might this turn."
def _disciple(g, o, ev, info):
    if ev == "move" and info["obj"] is o:
        def cost(g_, it):
            g_.burn(it.ctrl, 1)
            return True
        _q(g, o, "Shadow Order Disciple", _buff_me(1), may=True, cost=cost)


card("Shadow Order Disciple", on_event=_disciple)


# Shadowblade Lurker — "I cost [2] less for each card with my name in your trash."
card("Shadowblade Lurker",
     cost_mod=lambda g, pid, c, ch: (2 * sum(1 for x in g.p[pid].trash if x.cname == "Shadowblade Lurker"), 0))


# Shadows of the Past — "Return up to 2 units from trashes to their owners' hands."
def _shadows_choices(g, pid, ctx):
    cs = cap(g, _trash_units(g, pid) + _trash_units(g, 1 - pid), 4, pid)
    out = [dict(cs=(a.uid, b.uid)) for a, b in combinations(cs, 2)] + [dict(cs=(c.uid,)) for c in cs]
    return cap(g, out, 7, pid) + [dict(cs=())]


def _shadows(g, it):
    for uid in it.data.get("cs", ()):
        c = next((x for pl in g.p for x in pl.trash if x.uid == uid), None)
        if c is not None:
            g.to_zone(c, "hand")


card("Shadows of the Past", resolve=_shadows, choices=_shadows_choices)


# Sinister Poro — "When I attack, you may pay [1] to move an enemy unit here to its base."
def _sinister(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.move([u], "base", it.ctrl)
        _q(g, o, "Sinister Poro", res, may=True, cost=may_pay(1),
           choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                              lambda g_, it, u: u.ctrl != it.ctrl and u.loc == here, deflect=True))


card("Sinister Poro", on_event=_sinister)


# Soulgorger — "When you play me, you may play a unit from your trash, ignoring its Energy cost. (You must still
# pay its Power cost.)"
def _from_trash_ignore_energy(g, pid, kind, may):
    cands = [c for c in _trash_units(g, pid) if _power_ok(g, pid, c)]
    if not cands:
        return
    pick = g.ask(pid, kind, cands + ([None] if may else []))
    if pick is not None:
        play_unit_free(g, pid, pick, "trash", ignore_energy=True)


card("Soulgorger", on_play=lambda g, o, ctx: _q(
    g, o, "Soulgorger", lambda g_, it: _from_trash_ignore_energy(g_, it.ctrl, "soulgorger_pick", True)))


# Spiderling — "[Hidden] I have +1 might for each other unit you control here with my name. Your deck can have any
# number of cards named Spiderling."
card("Spiderling", hidden=True,
     might_mod=lambda g, o: sum(1 for u in g.units(o.ctrl, o.loc) if u is not o and u.cname == "Spiderling"))


# Spirit Wheel — "When you choose a friendly unit, you may pay [1] and exhaust this to draw 1."
def _wheel(g, o, ev, info):
    u = info.get("obj") if ev == "chosen" else None
    if u is not None and info["item"].ctrl == o.ctrl and u.ctrl == o.ctrl and u.spec["type"] == "Unit" \
            and not o.exhausted:
        def cost(g_, it):
            me = _me(g_, it)
            if me is None or me.exhausted or not g_.can_pay(it.ctrl, 1, []):
                return False
            g_.pay(it.ctrl, 1, [])
            me.exhausted = True
            return True
        _q(g, o, "Spirit Wheel", lambda g_, it: g_.draw(it.ctrl, 1), may=True, cost=cost)


card("Spirit Wheel", on_event=_wheel)


# Stacked Deck — "[Action] Look at the top 3 cards of your Main Deck. Put 1 into your hand and recycle the rest."
def _stacked(g, it):
    pl = g.p[it.ctrl]
    top = g.look(it.ctrl, pl.deck[:3])
    if not top:
        return
    c = g.ask(it.ctrl, "stacked_pick", sorted(top, key=lambda x: -_cval(x)))
    pl.deck.remove(c)
    c.zone = "hand"
    pl.hand.append(c)
    g.recycle_cards(it.ctrl, [x for x in top if x is not c])


card("Stacked Deck", timing="action", resolve=_stacked)


# Star-Crossed — "[Reaction] Return a friendly unit and an enemy unit to their owners' hands."
card("Star-Crossed", timing="reaction", preds=[P_friend, P_enemy],
     resolve=lambda g, it: [_to_hand(g, u) for u in (g.legal(it, 0), g.legal(it, 1))],
     choices=lambda g, pid, ctx: [dict(tg=(f.uid, e.uid)) for e in cap(g, enemies(g, pid), 4, pid)
                                  for f in cap(g, sorted(g.units(pid), key=lambda u: value(g, u)), 2, pid)])


# Stealthy Pursuer — "When a friendly unit moves from my location, I may be moved with it."
def _pursuer(g, o, ev, info):
    u = info.get("obj") if ev == "move" else None
    if u is not None and u is not o and u.ctrl == o.ctrl and info["frm"] == o.loc:
        dest = info["to"]

        def res(g_, it):
            me = _me(g_, it)
            if me is not None and me.loc != dest:
                g_.move([me], dest, it.ctrl)
        _q(g, o, "Stealthy Pursuer", res, may=True)


card("Stealthy Pursuer", on_event=_pursuer)


# Switcheroo — "[Hidden] [Action] Swap the Might of two units at the same battlefield this turn." (rule 433)
def _swap(g, it):
    a, b = g.legal(it, 0), g.legal(it, 1)
    if a is None or b is None or a.loc != b.loc or a.loc not in (0, 1):
        return
    ma, mb = g.might(a), g.might(b)
    if ma == mb:
        return
    lo, hi = (a, b) if ma < mb else (b, a)
    d = abs(ma - mb)
    g.mod(lo, d)
    g.mod(hi, -d)


def _swap_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    out = []
    for bf in ([hb] if hb is not None else [0, 1]):
        us = [u for u in g.units(loc=bf) if g.targetable(u, pid)]
        for a, b in combinations(us, 2):
            # deux unités de même Might sont des cibles légales (l'échange ne change rien) : proposées au joueur humain,
            # pas à l'IA pour qui ce choix ne sert à rien
            if g.might(a) != g.might(b) or full_choices(g, pid):
                out.append(dict(tg=(a.uid, b.uid)))

    def gain(c):
        a, b = g.obj(c["tg"][0]), g.obj(c["tg"][1])
        if a.ctrl == b.ctrl:
            return 0
        f, e = (a, b) if a.ctrl == pid else (b, a)
        return -(g.might(e) - g.might(f))
    out.sort(key=gain)
    return out


card("Switcheroo", hidden=True, timing="action", preds=[P_unit_bf, P_unit_bf], resolve=_swap, choices=_swap_choices)


# ====================================================================== T
# Tail-Cloaked Matriarch — "[Empower] [2][Chaos] When I become [Empowered], you may choose a unit in your trash with
# Energy cost no more than [3] and Power cost no more than [A]. Play it to your base, ignoring its cost."
def _matriarch_res(g, it):
    cands = _trash_units(g, it.ctrl, lambda c: c.spec["e"] <= 3 and c.spec["p"] <= 1)
    pick = g.ask(it.ctrl, "matriarch_pick", cands + [None]) if cands else None
    if pick is not None:
        play_unit_free(g, it.ctrl, pick, "trash", loc_choices=["base"])


card("Tail-Cloaked Matriarch", empower="2 energy and 1 chaos rune",
     on_event=lambda g, o, ev, info: (ev == "empowered" and info["obj"] is o
                                      and _q(g, o, "Tail-Cloaked Matriarch", _matriarch_res)))


# Teemo, Scout — "[Hidden] When you play me, give me +3 might this turn."
card("Teemo, Scout", hidden=True, on_play=lambda g, o, ctx: _q(g, o, "Teemo, Scout", _buff_me(3)))


# Temptation — "[Repeat] [2] Move an enemy unit to a location where there's a unit with the same controller."
# The repetition chooses its own unit and destination (rule 820.2.a): choice keys rep2 / tg2 / dest2, priced by
# extra_cost_fn (with the Deflect of the second target).
def _tempt_dests(g, u):
    return [d for d in ("base", 0, 1) if d != u.loc and any(x is not u for x in g.units(u.ctrl, d))]


def _tempt_once(g, it, u, dest):
    if u is not None and u.ctrl != it.ctrl and dest in _tempt_dests(g, u):
        g.move([u], dest, it.ctrl)


def _temptation(g, it):
    _tempt_once(g, it, g.legal(it, 0), it.data.get("dest"))
    if it.data.get("rep2") and it.data.get("t2"):
        u2 = _same(g, *it.data["t2"][0])
        if u2 is not None and not g.targetable(u2, it.ctrl):
            u2 = None
        _tempt_once(g, it, u2, it.data.get("dest2"))


def _tempt_choices(g, pid, ctx):
    base = []
    for u in enemies(g, pid):
        for d in _tempt_dests(g, u):
            base.append(dict(tg=(u.uid,), dest=d))
    base.sort(key=lambda c: (c["dest"] == "base", -value(g, g.obj(c["tg"][0]))))
    reps = [dict(a, rep2=True, tg2=b["tg"], dest2=b["dest"]) for a in cap(g, base, 2, pid) for b in cap(g, base, 2, pid)]
    return base[:3] + reps + base[3:]                 # repeat variants within the first 8 choices


def _tempt_parts(g, pid, c, ch):
    """The Repeat cost (each cost a separate optional additional cost) and the Deflect of the repetition's target."""
    if not ch.get("rep2"):
        return []
    return [("rep", 2, []), ("must", 0, deflect_reqs(g, pid, dict(tg=ch.get("tg2", ()))))]


card("Temptation", preds=[P_enemy], resolve=_temptation, choices=_tempt_choices, opt_parts=_tempt_parts)


# The Harrowing — "Play a unit from your trash, ignoring its Energy cost. (You must still pay its Power cost.)"
card("The Harrowing", resolve=lambda g, it: _from_trash_ignore_energy(g, it.ctrl, "harrowing_pick", False))


# The List — "As you play this, name a tag. [E]: Give a unit with the named tag -2 might this turn."
_TAGS = []


def _all_tags():
    if not _TAGS:
        _TAGS.extend(sorted(set(t for s in SPEC.values() for t in s["tags"])))
    return _TAGS


def _list_play(g, o, ctx):
    seen = []
    for u in enemies(g, o.ctrl, targetable=False):
        for t in sorted(g.tags(u)):
            if t not in seen:
                seen.append(t)
    tags = seen + [t for t in _all_tags() if t not in seen]
    o.named_tag = (o.oid, g.ask(o.ctrl, "name_tag", tags))     # "As you play this" (rule 355.1)


def _list_tag(o):
    t = getattr(o, "named_tag", None)
    return t[1] if t and t[0] == o.oid else None


card("The List", on_play=_list_play, abilities=[ability(
    "Mark", exhaust=True,
    choices=lambda g, pid, o: tg_choices([u for u in enemies(g, pid) + friends(g, pid)
                                          if _list_tag(o) in g.tags(u)]),
    preds=[lambda g, it, u: g.obj(it.src) is not None and _list_tag(g.obj(it.src)) in g.tags(u)],
    resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), -2))])


# The Syren — "[1], [E]: Move a friendly unit at a battlefield to your base."
card("The Syren", abilities=[ability(
    "Recall", cost="1 energy", exhaust=True, preds=[P_friend_bf],
    choices=lambda g, pid, o: tg_choices(friends(g, pid, True)),
    resolve=lambda g, it: g.legal(it, 0) is not None and g.move([g.legal(it, 0)], "base", it.ctrl))])


# Tideturner — "[Hidden] When you play me, you may choose a friendly unit. Move me to its location and it to my
# original location."  (its target may be chosen freely when played from Hidden, rule 811.1.d.2 example)
def _tideturner(g, o, ctx):
    def res(g_, it):
        u, me = g_.legal(it, 0), _me(g_, it)
        if u is None or me is None or u.loc == me.loc:
            return
        orig, there = me.loc, u.loc
        g_.move([me], there, it.ctrl)
        g_.move([u], orig, it.ctrl)
    _q(g, o, "Tideturner", res, may=True,
       choose=trig_target(lambda g_, it: [u for u in friends(g_, it.ctrl) if u.uid != it.src and u.loc != o.loc],
                          lambda g_, it, u: u.ctrl == it.ctrl and u.uid != it.src, kind="friend_target"))


card("Tideturner", hidden=True, on_play=_tideturner)


# Tornado Warrior — "[Hidden] When you play me from face down, you may empower something here. Disempower it at end
# of turn."
def _tornado(g, o, ctx):
    if ctx.get("src") != "facedown":
        return
    here = ctx.get("hidden_bf")

    def res(g_, it):
        x = g_.legal(it, 0)
        if x is None or not g_.empower(x):
            return
        uid, oid, pid = x.uid, x.oid, it.ctrl

        def at_end(g2, eff, info):
            g2.effects.remove(eff)

            def dis(g3, it3):
                y = _same(g3, uid, oid)
                if y is not None:
                    g3.disempower(y)
            g2.queue_trigger(pid, "Tornado Warrior (end of turn)", dis)
        g_.effects.append(dict(on="end_turn", fn=at_end, dur="turn"))
    _q(g, o, "Tornado Warrior", res, may=True,
       choose=trig_target(lambda g_, it: sorted([x for x in g_.board if x.loc == here and not x.empowered],
                                                key=lambda x: (x.ctrl != it.ctrl, -value(g_, x))),
                          lambda g_, it, x: x.loc == here, kind="empower_target"))


card("Tornado Warrior", hidden=True, on_play=_tornado)


# Traveling Merchant — "When I move, discard 1, then draw 1."
card("Traveling Merchant", on_event=lambda g, o, ev, info: (
    ev == "move" and info["obj"] is o
    and _q(g, o, "Traveling Merchant", lambda g_, it: (_discard(g_, it.ctrl, 1), g_.draw(it.ctrl, 1)))))


# Treasure Hunter — "When I move, play a Gold gear token exhausted."
card("Treasure Hunter", on_event=lambda g, o, ev, info: (
    ev == "move" and info["obj"] is o and _q(g, o, "Treasure Hunter", lambda g_, it: _gold(g_, it.ctrl))))


# Twilight Step — "Move a unit with 3 might or less. [Flow] [4][Chaos]"
def _twilight_choices(g, pid, ctx):
    out = [dict(tg=(u.uid,), dest=d) for u in friends(g, pid) + enemies(g, pid) if g.might(u) <= 3 for d in _dests(u)]

    def good(c):
        u = g.obj(c["tg"][0])
        return (u.ctrl == pid and c["dest"] in (0, 1)) or (u.ctrl != pid and c["dest"] == "base")
    out.sort(key=lambda c: not good(c))
    return out


card("Twilight Step", flow=flow_cost("4 energy and 1 chaos rune"),
     preds=[lambda g, it, o: P_unit(g, it, o) and g.might(o) <= 3], choices=_twilight_choices, max_choices=10,
     resolve=lambda g, it: (g.legal(it, 0) is not None and g.legal(it, 0).loc != it.data["dest"]
                            and g.move([g.legal(it, 0)], it.data["dest"], it.ctrl)))


# Twisted Fate, Gambler — "When I attack, reveal the top rune of your rune deck, then recycle it. Do one of the
# following based on its domain: [Fury] — Deal 2 to an enemy unit here and 1 to all other enemy units here.
# [Mind] — Draw 1. [Order] — Stun an enemy unit."  (other domains: nothing)
def _tf_pick(g, it, opts, kind):
    opts = [u for u in opts if _deflect_ok(g, it.ctrl, u)]
    if not opts:
        return None
    u = g.ask(it.ctrl, kind, opts, item=it)
    if u is None or not pay_deflect(g, it, u):
        return None
    return u


def _tf_res(g, it):
    pl = g.p[it.ctrl]
    if not pl.rune_deck:
        return
    r = pl.rune_deck.pop(0)
    pl.rune_deck.append(r)                                # recycle: bottom of the rune deck
    g.log(f"  Twisted Fate reveals a {r.domain} rune")
    here = it.data["here"]
    if r.domain == "Fury":
        u = _tf_pick(g, it, [u for u in enemies(g, it.ctrl) if u.loc == here], "target")
        if u is not None:
            others = [x for x in g.units(1 - it.ctrl, here) if x is not u]
            g.deal(u, 2, "ability", it.ctrl)
            for x in others:
                g.deal(x, 1, "ability", it.ctrl)
    elif r.domain == "Mind":
        g.draw(it.ctrl, 1)
    elif r.domain == "Order":
        u = _tf_pick(g, it, sorted(enemies(g, it.ctrl), key=lambda x: x.stunned), "target")
        if u is not None:
            g.stun(u, it.ctrl)


card("Twisted Fate, Gambler", on_event=lambda g, o, ev, info: (
    ev == "attack" and info["obj"] is o and _q(g, o, "Twisted Fate, Gambler", _tf_res, dict(here=o.loc))))


# ====================================================================== U / V / W
# Undercover Agent — "[Deathknell] Discard 2, then draw 2."
card("Undercover Agent", deathknell=lambda g, it: (_discard(g, it.ctrl, 2), g.draw(it.ctrl, 2)))


# Up from the Deep — "Play two 1 might Tentacle unit tokens. [Flow] [3]"
card("Up from the Deep", flow=flow_cost("3 energy"),
     resolve=lambda g, it: (_token(g, it.ctrl, "Tentacle"), _token(g, it.ctrl, "Tentacle")))


# Vicious Snapjaws — "When another friendly unit dies, gain 1 XP."
card("Vicious Snapjaws", on_event=lambda g, o, ev, info: (
    ev == "die" and info["info"]["ctrl"] == o.ctrl and info["info"]["obj"] is not o
    and info["info"]["spec"]["type"] == "Unit"
    and _q(g, o, "Vicious Snapjaws", lambda g_, it: g_.gain_xp(it.ctrl, 1))))


# Walking Roost — "[Deflect] When you play me, choose an opponent. They play a 1 might Bird unit token with
# [Deflect]."  (the opponent controls the token, rule 182, and plays it to one of their locations)
card("Walking Roost", kw={"Deflect": 1}, on_play=lambda g, o, ctx: _q(
    g, o, "Walking Roost", lambda g_, it: _token(g_, 1 - it.ctrl, "Bird")))


# Whirlwind — "Starting with the next player, each player may return a unit to its owner's hand."
def _whirlwind(g, it):
    for pid in (1 - it.ctrl, it.ctrl):
        en = sorted(g.units(1 - pid), key=lambda u: -value(g, u))
        own = sorted(g.units(pid), key=lambda u: value(g, u))
        if not en and not own:
            continue
        _to_hand(g, g.ask(pid, "whirlwind_pick", en + [None] + own))


card("Whirlwind", resolve=_whirlwind)


# Wind and Ghosts — "[Action] Choose a unit at a battlefield. If it has 3 might or less, banish it. Otherwise,
# return it to its owner's hand."
def _wind_ghosts(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    if g.might(u) <= 3:
        g.to_zone(u, "banish")
    else:
        _to_hand(g, u)


card("Wind and Ghosts", timing="action", preds=[P_unit_bf], resolve=_wind_ghosts,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True) + friends(g, pid, True)))


# Windsinger — "Hidden When you play me, you may return another unit at a battlefield with 3 might or less to its
# owner's hand."  (the database text lost the brackets of [Hidden]; the card has the keyword)
def _windsinger(g, o, ctx):
    hb = ctx.get("hidden_bf")

    def ok(g_, it, u):
        return u.uid != it.src and u.loc in (0, 1) and g_.might(u) <= 3 and (hb is None or u.loc == hb)

    _q(g, o, "Windsinger", lambda g_, it: _to_hand(g_, g_.legal(it, 0)), may=True,
       choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl, True) + friends(g_, it.ctrl, True)
                                          if ok(g_, it, u)], ok, deflect=True))


card("Windsinger", hidden=True, on_play=_windsinger)


# ====================================================================== Y / Z
# Yasuo, Windrider — "[Ganking] The third time I move in a turn, you score 1 point."
def _yasuo(g, o, ev, info):
    if ev == "move" and info["obj"] is o:
        key = ("yasuo_moves", o.uid, o.oid, g.turn_no)
        g.stats[key] += 1
        if g.stats[key] == 3:
            _q(g, o, "Yasuo, Windrider", lambda g_, it: g_.gain_point(it.ctrl, "Yasuo, Windrider"))


card("Yasuo, Windrider", kw={"Ganking": 1}, on_event=_yasuo)


# Zaunite Bouncer — "When you play me, return another unit at a battlefield to its owner's hand."
card("Zaunite Bouncer", on_play=lambda g, o, ctx: _q(
    g, o, "Zaunite Bouncer", lambda g_, it: _to_hand(g_, g_.legal(it, 0)),
    choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl, True) + friends(g_, it.ctrl, True)
                                       if u.uid != it.src],
                       lambda g_, it, u: u.uid != it.src and u.loc in (0, 1), deflect=True)))


# Zed, Without a Sound — "When I conquer, play a 0 might Shadow Clone unit token to your base. [Action] [1][Chaos]:
# Move me and a Shadow Clone you control to each other's locations."  (Shadow Clone is registered by cardsets/fury)
def _zed_event(g, o, ev, info):
    if ev == "conquer" and info["pid"] == o.ctrl and o in info["units"]:
        _q(g, o, "Zed, Without a Sound", lambda g_, it: make_token(g_, "Shadow Clone", it.ctrl, "base"))


def _zed_swap(g, it):
    me, c = g.obj(it.src), g.legal(it, 0)
    if me is None or c is None or me.loc == c.loc:
        return
    a, b = me.loc, c.loc
    g.move([me], b, it.ctrl)
    g.move([c], a, it.ctrl)


card("Zed, Without a Sound", on_event=_zed_event, abilities=[ability(
    "Shadow Swap", cost="1 energy and 1 chaos rune", timing="action",
    preds=[lambda g, it, u: u.ctrl == it.ctrl and u.cname == "Shadow Clone"],
    choices=lambda g, pid, o: tg_choices([u for u in g.units(pid) if u.cname == "Shadow Clone" and u.loc != o.loc]),
    resolve=_zed_swap)])


# ====================================================================== cards using the engine hooks (integration)
ask_text(ocean_drake="Ocean Drake : quelle unité non-Dragon renvoyer dans la main de son propriétaire ?",
         kennen_flow="Kennen, Storm of Shuriken : à quel sort de ta défausse donner [Flow] ?",
         mask_mother="Mask Mother : quelle unité alliée gagne +2 ?",
         maduli_dest="Maduli the Gatekeeper : vers quel champ de bataille ?",
         nocturne_play="Nocturne, Horrifying : le jouer pour 1 rune ?")


def _open_bfs(g, closed):
    return [] if closed else [b.idx for b in g.bfs if open_bf(g, b)]


# Miss Fortune, Buccaneer — "You may play me to an open battlefield. Friendly units may be played to open
# battlefields."
card("Miss Fortune, Buccaneer", extra_locs=lambda g, pid, c, closed: _open_bfs(g, closed),
     aura_locs=lambda g, src, pid, c, closed: _open_bfs(g, closed) if pid == src.ctrl and c.spec["type"] == "Unit"
     else [])


# Ocean Drake — "You may play me to an open battlefield. When you play me, you may return a non-Dragon unit to its
# owner's hand."
def _drake_res(g, it):
    us = sorted([u for u in g.units() if "Dragon" not in g.tags(u)],
                key=lambda u: (u.ctrl == it.ctrl, -value(g, u), u.uid))
    if us:
        u = g.ask(it.ctrl, "ocean_drake", us + [None], item=it)
        if u is not None:
            _to_hand(g, u)


card("Ocean Drake", extra_locs=lambda g, pid, c, closed: _open_bfs(g, closed),
     on_play=lambda g, o, ctx: _q(g, o, "Ocean Drake", _drake_res, may=True))


# Sai Scout — "[Vision] You may play me to an open battlefield."
card("Sai Scout", kw={"Vision": 1}, extra_locs=lambda g, pid, c, closed: _open_bfs(g, closed))


# Sneaky Deckhand — "You may play me to an open battlefield."
card("Sneaky Deckhand", extra_locs=lambda g, pid, c, closed: _open_bfs(g, closed))


# Irelia, Graceful — "Your spells that choose me cost 1 energy or 1 rune of any type less."
def _irelia_aura(g, src, pid, what):
    ch = what.get("choice") or {}
    chosen = list(ch.get("tg", ())) + list(ch.get("tg2", ()))
    for rc in ch.get("reps", ()):
        chosen += list(rc.get("tg", ()))
    if pid == src.ctrl and what["kind"] == "spell" and src.uid in chosen:
        return [dict(flex=1)]
    return ()


card("Irelia, Graceful", cost_aura=_irelia_aura)


# Stargazer — "Spells with [Flow] you play from your trash cost 2 energy less, to a minimum of 1 energy."
card("Stargazer",
     cost_aura=lambda g, src, pid, what: [dict(e=-2, min_e=1)] if pid == src.ctrl and what["kind"] == "spell"
     and what["src"] == "trash" and flow_of(g, pid, what["card"]) else ())


# Vex, Cheerless — "While I'm in combat, friendly spells cost 1 energy and 1 rune of any type less to a minimum of 1
# energy, and enemy spells cost 1 energy and 1 rune of any type more."
def _vex_aura(g, src, pid, what):
    if what["kind"] != "spell" or not g.in_combat(src):
        return ()
    if pid == src.ctrl:
        return [dict(e=-1, rm=1, min_e=1)]
    return [dict(e=1, p=[ANY])]


card("Vex, Cheerless", cost_aura=_vex_aura)


# Ezreal, Prodigy — "When you play me, discard 1, then draw 2. Optional additional costs you pay cost 1 energy or 1
# rune of any type less." (each optional additional cost: Accelerate, each Repeat cost, "you may pay ... as an
# additional cost")
def _ezreal_res(g, it):
    _discard(g, it.ctrl, 1)
    g.draw(it.ctrl, 2)


card("Ezreal, Prodigy", on_play=lambda g, o, ctx: _q(g, o, "Ezreal, Prodigy", _ezreal_res),
     cost_aura=lambda g, src, pid, what: [dict(flex=1, on="opt")] if pid == src.ctrl else ())


# Syndra, Transcendent — "While I'm in a showdown, your spells have [Repeat] 2 energy and 1 chaos rune." (a granted
# Repeat instance, rule 820.1.c.2: actions.repeat_instances; its repetition makes its own choices)
def _syndra_kw(g, src, pid, c, kw, zone):
    if kw == "Repeat" and pid == src.ctrl and c.spec["type"] == "Spell" and g.sd is not None and src.loc == g.sd.bf:
        return (2, [frozenset({"Chaos"})])
    return False


card("Syndra, Transcendent", card_kw=_syndra_kw)


# Kennen, Storm of Shuriken — "When you play me, [Burn 2]. When I conquer, give a spell in your trash [Flow] equal to
# its cost this turn." (a 'grant_flow' effect on that card: its Energy and Power costs, of its domains)
def _kennen_conquer(g, it):
    sp = sorted([c for c in g.p[it.ctrl].trash if c.spec["type"] == "Spell" and g.impl(c) is not None],
                key=lambda c: (-_cval(c), c.uid))
    if not sp:
        return
    c = g.ask(it.ctrl, "kennen_flow", sp, item=it)
    g.effects.append(dict(kind="grant_flow", uid=c.uid, oid=c.oid, dur="turn",
                          flow=(c.spec["e"], c.spec["p"], tuple(c.spec["domains"]))))
    g.log(f"  {c} has [Flow] this turn")


def _kennen_event(g, o, ev, info):
    if ev == "conquer" and o in info["units"]:
        _q(g, o, "Kennen, Storm of Shuriken", _kennen_conquer)


card("Kennen, Storm of Shuriken", on_event=_kennen_event,
     on_play=lambda g, o, ctx: _q(g, o, "Kennen, Storm of Shuriken", lambda g_, it: g_.burn(it.ctrl, 2)))


# Mask Mother — "When you discard me, you may pay 1 energy to give a friendly unit +2 might this turn."
def _mask_res(g, it):
    us = sorted(g.units(it.ctrl), key=lambda u: (-value(g, u), u.uid))
    if us:
        g.mod(g.ask(it.ctrl, "mask_mother", us, item=it) if len(us) > 1 else us[0], 2)


card("Mask Mother", on_discard=lambda g, c, pid: g.queue_trigger(pid, "Mask Mother", _mask_res, may=True,
                                                                 cost=may_pay(1)))


# Scrapheap — "When this is played, discarded, or killed, draw 1." (killed: its own 'leave' event from a kill; this is
# not a Deathknell)
card("Scrapheap",
     on_play=lambda g, o, ctx: _q(g, o, "Scrapheap", lambda g_, it: g_.draw(it.ctrl, 1)),
     on_discard=lambda g, c, pid: g.queue_trigger(pid, "Scrapheap", lambda g_, it: g_.draw(it.ctrl, 1)),
     on_leave=lambda g, o, info: info.get("killed") and g.queue_trigger(
         info["info"]["ctrl"], "Scrapheap", lambda g_, it: g_.draw(it.ctrl, 1)))


# Treasure Trove — "When this leaves the board, draw 1 and channel 1 rune exhausted. [Chaos], [E]: Kill this."
card("Treasure Trove",
     on_leave=lambda g, o, info: g.queue_trigger(
         info["info"]["ctrl"], "Treasure Trove",
         lambda g_, it: (g_.draw(it.ctrl, 1), g_.channel(it.ctrl, 1, exhausted=True))),
     abilities=[ability("Kill this", "1 chaos rune", exhaust=True,
                        resolve=lambda g, it: _me(g, it) is not None and g.kill([_me(g, it)], it.ctrl))])


# Maduli the Gatekeeper — "I can't be readied. [Chaos]: Move me to an occupied enemy battlefield if my Might is greater
# than the total Might of enemy units there."
def _maduli_ok(g, o, b):
    es = g.units(1 - o.ctrl, b)
    return bool(es) and g.bfs[b].ctrl == 1 - o.ctrl and g.might(o) > sum(g.might(u) for u in es)


def _maduli_res(g, it):
    o = g.obj(it.src)
    b = it.data.get("bf")
    if o is not None and b is not None and o.loc != b and _maduli_ok(g, o, b):
        g.move([o], b, it.ctrl)


card("Maduli the Gatekeeper", no_ready=lambda g, o: True,
     abilities=[ability("Move", "1 chaos rune", resolve=_maduli_res,
                        choices=lambda g, pid, o: [dict(bf=b.idx) for b in g.bfs
                                                   if b.idx != o.loc and _maduli_ok(g, o, b.idx)])])


# Vex, Apathetic — "[Deflect] When an opponent plays a unit while I'm at a battlefield, [Stun] it. They can't move it
# this turn."
def _vex_no_move(g, eff, o, dest, by_pid, standard):
    return o.uid == eff["uid"] and o.oid == eff["oid"] and by_pid == eff["pid"]


def _vex_ap_res(g, it):
    u = _same(g, it.data["u"], it.data["uoid"])
    if u is None:
        return
    g.stun(u, it.ctrl)
    g.effects.append(dict(kind="cant_move", fn=_vex_no_move, uid=u.uid, oid=u.oid, pid=u.ctrl, dur="turn"))


def _vex_ap_event(g, o, ev, info):
    c = info.get("card") if ev == "played" else None
    if c is not None and info["pid"] != o.ctrl and c.spec["type"] == "Unit" and o.loc in (0, 1) and c in g.board:
        _q(g, o, "Vex, Apathetic", _vex_ap_res, dict(u=c.uid, uoid=c.oid))


card("Vex, Apathetic", kw={"Deflect": 1}, on_event=_vex_ap_event)


# Sivir, Mercenary — "[Accelerate] If you've spent at least 2 runes of any type this turn, I have +2 might and
# [Ganking]." (Game.hist['power_spent'])
def _sivir_merc(g, o):
    return g.hist["power_spent"][o.ctrl] >= 2


card("Sivir, Mercenary", accelerate=True, might_if=[(_sivir_merc, 2)], kw_if=[(_sivir_merc, {"Ganking": 1})])


# Nocturne, Horrifying — "[Ganking] When you look at cards from the top of your deck (and don't draw them) and see me,
# you may play me for 1 rune of any type." (Impl.on_seen, called by Game.look; when the trigger resolves he must
# still be in the Main Deck, not drawn since)
def _nocturne_res(g, it):
    c = None
    for x in g.p[it.ctrl].deck:
        if x.uid == it.data["c"]:
            c = x
    if c is None or c.oid != it.data["oid"] or c.drawn_at != it.data["drawn"]:
        return
    chs = [ch for ch in unit_play_choices(g, it.ctrl, c, "deck")
           if g.can_pay(it.ctrl, *total_cost(g, it.ctrl, c, dict(ch, alt="nocturne", alt_e=0, alt_reqs=(ANY,)),
                                             "deck"), dict(kind="unit", card=c))]
    if not chs:
        return
    ch = g.ask(it.ctrl, "play_choice", chs, card=c) if len(chs) > 1 else chs[0]
    ch = dict(ch, alt="nocturne", alt_e=0, alt_reqs=(ANY,))
    e, reqs = total_cost(g, it.ctrl, c, ch, "deck")
    if g.pay(it.ctrl, e, reqs, dict(kind="unit", card=c)):
        play_card(g, it.ctrl, c, "deck", ch, limited=True)


def _nocturne_seen(g, c, pid):
    if pid == c.owner:
        g.queue_trigger(pid, "Nocturne, Horrifying", _nocturne_res,
                        dict(c=c.uid, oid=c.oid, drawn=c.drawn_at), may=True)


card("Nocturne, Horrifying", kw={"Ganking": 1}, on_seen=_nocturne_seen)
