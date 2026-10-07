"""Batch calm: the Calm cards of batches/calm.txt (units, spells, gear and equipment). See GUIDE.md.

Cards that need an engine hook are not registered here: they are listed in NEEDS_calm.md with the hook they need.
Conventions of this module:
- triggered abilities never keep Obj references in closures (the AI clones the game with deepcopy): they store
  uids / oids in the trigger data or in the g.effects entry;
- damage dealt by a unit because of a spell ("It deals damage equal to its Might ...") has the unit as its
  source, not the spell (rule 417.6.b.3), so it is dealt with kind "unit" (no spell/ability Bonus Damage);
- tokens played without a location ("Play a 2 might Sand Soldier unit token") are played like units, to base or
  to a battlefield their controller controls (rules 184.2, 439.2.c): the location is asked ("play_location").
"""
from itertools import combinations
from cards import *  # noqa: F401,F403
from actions import total_cost, pay_ctx

_PET_TAGS = ("Bird", "Cat", "Dog", "Poro")


# ====================================================================== helpers
def _alive(g, uid, oid):
    """The permanent uid if it is still the same incarnation (oid), else None."""
    o = g.obj(uid)
    return o if o is not None and o.oid == oid else None


def _pet_tags(g, pid):
    """Tags among Bird, Cat, Dog and Poro found among the units pid controls."""
    found = set()
    for u in g.units(pid):
        found |= set(t for t in _PET_TAGS if t in u.spec["tags"])
    return found


def _near_victory(g, pid):
    """'If an opponent's score is within 3 points of the Victory Score'."""
    return any(g.victory - g.p[q].points <= 3 for q in (0, 1) if q != pid)


def _token_locs(g, pid):
    return ["base"] + [b.idx for b in g.bfs if b.ctrl == pid]


def _play_token(g, name, pid, loc=None, ready=False):
    """Play a unit token; loc None = a location the token could be played to (asked)."""
    if loc is None:
        loc = g.ask(pid, "play_location", _token_locs(g, pid))
    return make_token(g, name, pid, loc, ready=ready)


def _draw_card(g, pid, c):
    """'Draw it': a card looked at on top of the Main Deck goes to the hand (a draw)."""
    pl = g.p[pid]
    if c in pl.deck:
        pl.deck.remove(c)
    c.zone = "hand"
    pl.hand.append(c)
    g.emit("draw", pid=pid, card=c)


def _look_pick(g, pid, n, ok, kind):
    """Look at the top n cards of pid's Main Deck, may reveal one for which ok(card) and draw it, recycle the rest.
    Returns the drawn card or None."""
    pl = g.p[pid]
    top = pl.deck[:n]
    if not top:
        return None
    cands = sorted([c for c in top if ok(c)], key=lambda c: -(c.spec["e"] + 2 * c.spec["p"]))
    pick = g.ask(pid, kind, cands + [None])
    if pick is not None:
        g.log(f"  P{pid} reveals and draws {pick.cname}")
        _draw_card(g, pid, pick)
    rest = [c for c in top if c is not pick]
    for c in rest:
        if c in pl.deck:
            pl.deck.remove(c)
    g.recycle_cards(pid, rest)
    return pick


def _units_first(g, pid, hb=None):
    """All units, friendly ones first (AI ordering for positive effects)."""
    return friends(g, pid, False, hb) + enemies(g, pid, False, hb)


def _mod_trigger(name, n):
    """'When you play me, give a unit +n might this turn.'"""
    def on_play(g, o, ctx):
        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.mod(u, n)
        g.queue_trigger(o.ctrl, name, res, src=o.uid,
                        choose=trig_target(lambda g_, it: _units_first(g_, it.ctrl), P_unit,
                                           kind="friendly_target", deflect=True))
    return on_play


def _hold(o, info):
    return info["pid"] == o.ctrl and o in info["units"]


def _spell_items(g, pid):
    """Spells on the chain that can be countered, enemy spells first (most recent first)."""
    sp = [i for i in reversed(g.chain) if i.kind == "spell" and not i.uncounterable]
    return [i for i in sp if i.ctrl != pid] + [i for i in sp if i.ctrl == pid]


def _counter_spell(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is not None and tgt.kind == "spell":
        g.counter(tgt)


def _ready_runes(g, pid, n):
    ex = [r for r in g.p[pid].runes if r.exhausted]
    for _ in range(min(n, len(ex))):
        r = g.ask(pid, "ready_rune", list(ex)) if len(ex) > 1 else ex[0]
        ex.remove(r)
        r.exhausted = False


def _compositions(total, k, cap=400):
    """Ways to split total into k positive integers (ordered), at most cap of them."""
    out = []

    def rec(left, parts, acc):
        if len(out) >= cap:
            return
        if parts == 1:
            if left >= 1:
                out.append(tuple(acc + [left]))
            return
        for x in range(1, left - parts + 2):
            rec(left - x, parts - 1, acc + [x])
    if k >= 1 and total >= k:
        rec(total, k, [])
    return out


def _split_damage(g, pid, amount, targets, source_kind, by_pid):
    """Rule 355.14: split amount among targets, each getting at least 1; if there are more targets than damage,
    the controller chooses which ones stop being targets (keeping exactly amount of them). Returns the targets."""
    if amount <= 0 or not targets:
        return []
    if len(targets) > amount:
        keeps = list(combinations(targets, amount))[:200]
        keeps.sort(key=lambda ts: -sum(value(g, t) for t in ts if g.might(t) - t.damage <= 1))
        targets = list(g.ask(pid, "split_keep", keeps))
    splits = _compositions(amount, len(targets))

    def score(sp):
        return -sum(value(g, t) for t, n in zip(targets, sp) if t.damage + n >= g.might(t))
    splits.sort(key=score)
    sp = g.ask(pid, "split_damage", splits)
    for t, n in zip(targets, sp):
        g.deal(t, n, source_kind, by_pid)
    return targets


def _kill_lethal(g, units, by_pid):
    """Units of units with lethal damage die now (the kills a spell is responsible for); returns those killed."""
    dead = [u for u in units if u in g.board and g.lethal(u)]
    return g.kill(dead, by_pid) if dead else []


# ====================================================================== units
# Affectionate Poro — "When a combat that I was in ends, if I haven't been dealt damage this turn, draw 1."
def _aff_poro(g, o, ev, info):
    if ev == "damaged" and info["obj"] is o:
        g.stats[("calm_dmg", o.uid, o.oid, g.turn_no)] += 1
    elif ev == "combat_end" and o in info["members"] and not g.stats[("calm_dmg", o.uid, o.oid, g.turn_no)]:
        def res(g_, it):
            if not g_.stats[("calm_dmg", it.src, it.data["oid"], g_.turn_no)]:
                g_.draw(it.ctrl, 1)
        g.queue_trigger(o.ctrl, "Affectionate Poro", res, dict(oid=o.oid), src=o.uid)


card("Affectionate Poro", on_event=_aff_poro)


# Ahri, Alluring — "When I hold, you score 1 point."
def _ahri(g, o, ev, info):
    if ev == "hold" and _hold(o, info):
        g.queue_trigger(o.ctrl, "Ahri, Alluring", lambda g_, it: g_.gain_point(it.ctrl, "Ahri, Alluring"), src=o.uid)


card("Ahri, Alluring", on_event=_ahri)


# Allay, Eager Admirer — "[Deflect] While I'm at a battlefield, your other units here have [Deflect]."
card("Allay, Eager Admirer", kw={"Deflect": 1},
     aura_kw=lambda g, src, o: ({"Deflect": 1} if src in g.board and src.loc in (0, 1) and o is not src
                                and o.ctrl == src.ctrl and o.loc == src.loc and o.spec["type"] == "Unit" else None))


# Aphelios, Exalted — "When you attach an Equipment to me, choose one that hasn't been chosen this turn —
# Ready 2 runes. / Channel 1 rune exhausted. / Buff a friendly unit."
def _aphelios(g, o, ev, info):
    if ev != "attached" or info["unit"] is not o or "Equipment" not in info["gear"].spec["tags"] \
            or info["gear"].ctrl != o.ctrl:
        return
    oid = o.oid

    def choose(g_, it):
        used = [m for m in ("ready", "channel", "buff") if g_.stats[("calm_aph", it.src, oid, g_.turn_no, m)]]
        modes = [m for m in ("ready", "channel", "buff") if m not in used]
        if "ready" in modes and not any(r.exhausted for r in g_.p[it.ctrl].runes):
            modes.remove("ready")
            modes.append("ready")                         # AI order only: still a legal mode
        if not modes:
            return False
        m = g_.ask(it.ctrl, "aphelios_mode", modes)
        g_.stats[("calm_aph", it.src, oid, g_.turn_no, m)] += 1
        it.data["mode"] = m
        if m == "buff":
            us = sorted(friends(g_, it.ctrl), key=lambda u: (u.buff > 0, -value(g_, u)))
            if us:
                g_.add_target(it, g_.ask(it.ctrl, "friendly_target", us, item=it), P_friend)
        return True

    def res(g_, it):
        m = it.data.get("mode")
        if m == "ready":
            _ready_runes(g_, it.ctrl, 2)
        elif m == "channel":
            g_.channel(it.ctrl, 1, exhausted=True)
        elif m == "buff":
            u = g_.legal(it, 0)
            if u is not None:
                g_.buff(u)
    g.queue_trigger(o.ctrl, "Aphelios, Exalted", res, src=o.uid, choose=choose)


card("Aphelios, Exalted", on_event=_aphelios)


# Apprentice Smith — "When I move, reveal the top card of your Main Deck. If it's a gear, draw it. Otherwise,
# recycle it."
def _smith_res(g, it):
    pl = g.p[it.ctrl]
    if not pl.deck:
        return
    c = pl.deck[0]
    g.log(f"  P{it.ctrl} reveals {c.cname}")
    if c.spec["type"] == "Gear":
        _draw_card(g, it.ctrl, c)
    else:
        pl.deck.remove(c)
        g.recycle_cards(it.ctrl, [c])


card("Apprentice Smith", on_event=lambda g, o, ev, info: ev == "move" and info["obj"] is o and
     g.queue_trigger(o.ctrl, "Apprentice Smith", _smith_res, src=o.uid))


# Azir, Ascendant — "[Calm]: [Action] — Choose a unit you control. Move me to its location and it to my original
# location. If it's equipped, you may attach one of its Equipment to me. Use only once per turn."
def _azir_res(g, it):
    az = g.obj(it.src)
    u = g.legal(it, 0)
    if az is None or u is None:
        return
    a_loc, u_loc = az.loc, u.loc
    g.move([az], u_loc, it.ctrl)
    g.move([u], a_loc, it.ctrl)
    eq = [g.obj(x) for x in u.attached]
    eq = [x for x in eq if x is not None and "Equipment" in x.spec["tags"]]
    if eq:
        eq.sort(key=lambda x: -EQUIP_BONUS.get(x.cname, 0))
        x = g.ask(it.ctrl, "azir_attach", eq + [None])
        if x is not None and az in g.board:
            attach(g, x, az)


def _azir_choices(g, pid, o):
    us = [u for u in friends(g, pid) if u is not o and (u.loc != o.loc or u.attached)]
    us.sort(key=lambda u: (u.loc in (0, 1), len(u.attached)), reverse=True)
    return [dict(tg=(u.uid,)) for u in us]


def _azir_mark(g, pid, o, ch):
    g.stats[("calm_azir", o.uid, o.oid, g.turn_no)] += 1


card("Azir, Ascendant", abilities=[ability(
    "Swap", cost="1 calm rune", timing="action", preds=[P_friend], choices=_azir_choices, resolve=_azir_res,
    can=lambda g, pid, o: not g.stats[("calm_azir", o.uid, o.oid, g.turn_no)], extra_cost=_azir_mark)])


# Caitlyn, Patrolling — "I must be assigned combat damage last. [E]: Deal damage equal to my Might to a unit at a
# battlefield. Use this ability only while I'm at a battlefield."
# The first ability is Backline (rule 465.2.c.8 speaks of "Caitlyn, Patrolling with the Backline ability").
def _cait_res(g, it):
    me = g.obj(it.src)
    u = g.legal(it, 0)
    if me is None or u is None:
        return                                          # null Might (rule 359.3.e.12)
    g.deal(u, g.might(me), "ability", it.ctrl)


card("Caitlyn, Patrolling", kw={"Backline": 1}, abilities=[ability(
    "Shoot", exhaust=True, preds=[P_unit_bf], resolve=_cait_res,
    can=lambda g, pid, o: o.loc in (0, 1),
    choices=lambda g, pid, o: tg_choices(all_units(g, pid, True)))])


# Clockwork Keeper — "As you play me, you may pay [Calm] as an additional cost. If you do, draw 1."
card("Clockwork Keeper",
     as_played=lambda g, pid, c: [dict(calm_paid=True), dict()],
     extra_cost_fn=lambda g, pid, c, ch: (0, [frozenset({"Calm"})]) if ch.get("calm_paid") else (0, []),
     on_play=lambda g, o, ctx: ctx.get("calm_paid") and g.draw(o.ctrl, 1))


# Daisy! — "I enter ready. Reduce my cost by 1 energy for each of the following tags among your units — Bird,
# Cat, Dog, and Poro. When I attack while your units have all 4 tags, [Stun] an enemy unit here."
def _daisy(g, o, ev, info):
    if ev == "attack" and info["obj"] is o and len(_pet_tags(g, o.ctrl)) == 4:
        here = o.loc
        g.queue_trigger(o.ctrl, "Daisy!", lambda g_, it: g_.stun(g_.legal(it, 0), it.ctrl), src=o.uid,
                        choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                                           lambda g_, it, u: u.ctrl != it.ctrl and u.loc == here, deflect=True))


card("Daisy!", enter_ready=True, on_event=_daisy,
     cost_mod=lambda g, pid, c, ch: (len(_pet_tags(g, pid)), 0))


# Eclipse Herald — "When you stun an enemy unit, ready me and give me +1 might this turn."
def _eclipse(g, o, ev, info):
    if ev == "stun" and info["by"] == o.ctrl and info["obj"].ctrl != o.ctrl:
        def res(g_, it):
            me = _alive(g_, it.src, it.data["oid"])
            if me is not None:
                g_.ready_obj(me)
                g_.mod(me, 1)
        g.queue_trigger(o.ctrl, "Eclipse Herald", res, dict(oid=o.oid), src=o.uid)


card("Eclipse Herald", on_event=_eclipse)


# Enthusiastic Promoter — "[Backline] When I hold, [Buff] all units here."
def _promoter(g, o, ev, info):
    if ev == "hold" and _hold(o, info):
        bf = info["bf"]
        g.queue_trigger(o.ctrl, "Enthusiastic Promoter",
                        lambda g_, it: [g_.buff(u) for u in g_.units(loc=bf)], src=o.uid)


card("Enthusiastic Promoter", kw={"Backline": 1}, on_event=_promoter)

# Field Musicians — "When you play me, give a unit +3 might this turn."
card("Field Musicians", on_play=_mod_trigger("Field Musicians", 3))

# Whiteflame Protector — "When you play me, give a unit +8 might this turn."
card("Whiteflame Protector", on_play=_mod_trigger("Whiteflame Protector", 8))


# Frisky Hunter — "When you play me, play a 1 might Bird unit token with [Deflect] here."
def _frisky(g, o, ctx):
    def res(g_, it):
        me = _alive(g_, it.src, it.data["oid"])
        make_token(g_, "Bird", it.ctrl, me.loc if me is not None else it.data["loc"])
    g.queue_trigger(o.ctrl, "Frisky Hunter", res, dict(oid=o.oid, loc=o.loc), src=o.uid)


card("Frisky Hunter", on_play=_frisky)

# Frostcoat Mother — "[Empower] [12]. This ability costs [1] less for each rune you control. [Empowered] I have
# +3 might."
card("Frostcoat Mother", empower=lambda g, pid, o, ch: (max(0, 12 - len(g.p[pid].runes)), []),
     might_if=[(when_empowered, 3)])

# Steel Paws — "[Deflect] [Empower] [7] [Empowered] I have +7 might."
card("Steel Paws", kw={"Deflect": 1}, empower="7 energy", might_if=[(when_empowered, 7)])

# Serene Ascetic — "[Empower] [3] [Empowered] I have [Deflect] and [Shield 3]."
card("Serene Ascetic", empower="3 energy", kw_if=[(when_empowered, {"Deflect": 1, "Shield": 3})])


# Nasus, Ascended — "[Deflect 2] [Empower] [8] [Empowered] When I conquer, you score 1 point."
def _nasus(g, o, ev, info):
    if ev == "conquer" and _hold(o, info) and o.empowered:
        g.queue_trigger(o.ctrl, "Nasus, Ascended", lambda g_, it: g_.gain_point(it.ctrl, "Nasus, Ascended"),
                        src=o.uid)


card("Nasus, Ascended", kw={"Deflect": 2}, empower="8 energy", on_event=_nasus)


# Guardian of the Passage — "When I hold, you may return a unit or gear from your trash to your hand."
def _passage(g, o, ev, info):
    if ev != "hold" or not _hold(o, info):
        return

    def choose(g_, it):
        cs = [c for c in g_.p[it.ctrl].trash if c.spec["type"] in ("Unit", "Gear")]
        if not cs:
            return False
        c = g_.ask(it.ctrl, "return_pick", sorted(cs, key=lambda c: -(c.spec["e"] + 2 * c.spec["p"])), item=it)
        it.data["card"] = c.uid
        return True

    def res(g_, it):
        c = next((x for x in g_.p[it.ctrl].trash if x.uid == it.data.get("card")), None)
        if c is not None:
            g_.to_zone(c, "hand")
    g.queue_trigger(o.ctrl, "Guardian of the Passage", res, src=o.uid, may=True, choose=choose)


card("Guardian of the Passage", on_event=_passage)

# Herald of Spring — "[Hunt] When you play me, gain 2 XP."
card("Herald of Spring", kw={"Hunt": 1},
     on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Herald of Spring", lambda g_, it: g_.gain_xp(it.ctrl, 2),
                                               src=o.uid))


# Iascylla — "When I hold, at the start of your next Main Phase, you may move an enemy unit to this battlefield."
def _iascylla_main(g, eff, info):
    if info["pid"] != eff["pid"]:
        return
    g.effects.remove(eff)
    bf = eff["bf"]

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None and u.loc != bf:
            g_.move([u], bf, it.ctrl)
    g.queue_trigger(eff["pid"], "Iascylla (Main Phase)", res, may=True,
                    choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc != bf],
                                       lambda g_, it, u: u.ctrl != it.ctrl, deflect=True))


def _iascylla(g, o, ev, info):
    if ev == "hold" and _hold(o, info):
        pid, bf = o.ctrl, info["bf"]
        g.queue_trigger(pid, "Iascylla", lambda g_, it: g_.effects.append(
            dict(on="main_start", fn=_iascylla_main, pid=pid, bf=bf)), src=o.uid)


card("Iascylla", on_event=_iascylla)


# Ivern, Nurturer — "When you play me or when I hold, look at the top 3 cards of your Main Deck. You may reveal a
# unit from among them and draw it. Recycle the rest. Then if you revealed a Bird, Cat, Dog, or Poro, do this:
# [Buff] a friendly unit."
def _ivern_res(g, it):
    pick = _look_pick(g, it.ctrl, 3, lambda c: c.spec["type"] == "Unit", "ivern_pick")
    if pick is not None and any(t in pick.spec["tags"] for t in _PET_TAGS):
        us = sorted(friends(g, it.ctrl), key=lambda u: (u.buff > 0, -value(g, u)))
        if us:
            g.buff(g.ask(it.ctrl, "friendly_target", us))


card("Ivern, Nurturer",
     on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Ivern, Nurturer", _ivern_res, src=o.uid),
     on_event=lambda g, o, ev, info: ev == "hold" and _hold(o, info) and
     g.queue_trigger(o.ctrl, "Ivern, Nurturer", _ivern_res, src=o.uid))


# Ornn, Blacksmith — "When you play me or when I hold, look at the top 4 cards of your Main Deck. You may reveal a
# gear from among them and draw it. Then recycle the rest."
def _ornn_res(g, it):
    _look_pick(g, it.ctrl, 4, lambda c: c.spec["type"] == "Gear", "ornn_pick")


card("Ornn, Blacksmith",
     on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Ornn, Blacksmith", _ornn_res, src=o.uid),
     on_event=lambda g, o, ev, info: ev == "hold" and _hold(o, info) and
     g.queue_trigger(o.ctrl, "Ornn, Blacksmith", _ornn_res, src=o.uid))


# Janna, Savior — "[Reaction] When you play me, heal your units here, then move an enemy unit from here to its base."
def _janna(g, o, ctx):
    here = o.loc

    def res(g_, it):
        for u in g_.units(it.ctrl, here):
            u.damage = 0
        u = g_.legal(it, 0)
        if u is not None and u.loc == here:
            g_.move([u], "base", it.ctrl)

    def choose(g_, it):
        es = [u for u in enemies(g_, it.ctrl) if u.loc == here and here in (0, 1)]
        es = [u for u in es if g_.can_pay(it.ctrl, 0, deflect_reqs(g_, it.ctrl, dict(tg=(u.uid,))))]
        if es:
            u = g_.ask(it.ctrl, "target", es, item=it)
            if pay_deflect(g_, it, u):
                g_.add_target(it, u, lambda g2, it2, x: x.ctrl != it2.ctrl and x.loc == here)
        return True                                      # the heal happens even without an enemy unit
    g.queue_trigger(o.ctrl, "Janna, Savior", res, src=o.uid, choose=choose)


card("Janna, Savior", timing="reaction", on_play=_janna)


# Leona, Zealot — "If an opponent's score is within 3 points of the Victory Score, I enter ready. Stunned enemy units
# here have -8 might, to a minimum of 1 might."
_LEONA_GUARD = set()


def _leona_aura(g, src, o):
    if src.uid in _LEONA_GUARD or src not in g.board or o.spec["type"] != "Unit" or not o.stunned \
            or o.ctrl == src.ctrl or o.loc != src.loc:
        return 0
    _LEONA_GUARD.add(src.uid)
    try:
        m = g.might(o)                                   # Might without this effect
    finally:
        _LEONA_GUARD.discard(src.uid)
    return -min(8, max(0, m - 1))


card("Leona, Zealot", enter_ready=lambda g, pid, c, ch: _near_victory(g, pid), aura_might=_leona_aura)

# Master Yi, Meditative — "While you have 8+ runes, I have +4 might."
card("Master Yi, Meditative", might_if=[(lambda g, o: len(g.p[o.ctrl].runes) >= 8, 4)])


# Master Yi, Unstoppable — "[Level 3] I cost [2][Calm] less. [Level 6] I cost [4][Calm][Calm] less instead.
# [Level 11] I cost [6][Calm][Calm][Calm] less instead. [Level 16] I can't be chosen by enemy spells and abilities."
def _yi_cost(g, pid, c, ch):
    xp = g.p[pid].xp
    for n, d in ((11, (6, 3)), (6, (4, 2)), (3, (2, 1))):
        if xp >= n:
            return d
    return (0, 0)


card("Master Yi, Unstoppable", cost_mod=_yi_cost,
     untargetable=lambda g, o, by: by != o.ctrl and g.p[o.ctrl].xp >= 16)


# Monch — "If an opponent controls a stunned unit, I cost [2] less and enter ready."
def _opp_stunned(g, pid):
    return any(u.stunned for u in g.units(1 - pid))


card("Monch", cost_mod=lambda g, pid, c, ch: (2, 0) if _opp_stunned(g, pid) else (0, 0),
     enter_ready=lambda g, pid, c, ch: _opp_stunned(g, pid))

# Mosstomper — "[Hunt 2] [Level 3] I have +1 might and [Deflect]."
card("Mosstomper", kw={"Hunt": 2}, levels=[(3, dict(might=1, kw={"Deflect": 1}))])


# Nami, Headstrong — "You may pay [Calm] as an additional cost to play me. When you play me, if you paid the
# additional cost, [Stun] an enemy unit. When I hold, the next time you play a unit this turn, ready it and [Buff]
# it."
def _nami_play(g, o, ctx):
    if not ctx.get("calm_paid"):
        return
    hb = ctx.get("hidden_bf")
    g.queue_trigger(o.ctrl, "Nami, Headstrong", lambda g_, it: g_.stun(g_.legal(it, 0), it.ctrl), src=o.uid,
                    choose=trig_target(lambda g_, it: enemies(g_, it.ctrl, False, hb), P_enemy, deflect=True))


def _nami_next(g, eff, info):
    if info["pid"] != eff["pid"] or info["card"].spec["type"] != "Unit":
        return
    g.effects.remove(eff)
    c = info["card"]

    def res(g_, it):
        u = _alive(g_, it.data["u"], it.data["oid"])
        if u is not None:
            g_.ready_obj(u)
            g_.buff(u)
    g.queue_trigger(eff["pid"], "Nami, Headstrong (next unit)", res, dict(u=c.uid, oid=c.oid))


def _nami_event(g, o, ev, info):
    if ev == "hold" and _hold(o, info):
        pid = o.ctrl
        g.queue_trigger(pid, "Nami, Headstrong", lambda g_, it: g_.effects.append(
            dict(on="played", fn=_nami_next, pid=pid, dur="turn")), src=o.uid)


card("Nami, Headstrong", on_play=_nami_play, on_event=_nami_event,
     as_played=lambda g, pid, c: [dict(calm_paid=True), dict()],
     extra_cost_fn=lambda g, pid, c, ch: (0, [frozenset({"Calm"})]) if ch.get("calm_paid") else (0, []))

# Ol' Poro — "I can't be played on your first, second, or third turns."
card("Ol' Poro", as_played=lambda g, pid, c: [dict()] if g.p[pid].turns > 3 else [])


# Pakaa Protector — "When I move, reveal the top card of your Main Deck. If it's a unit, draw it. Otherwise, put it
# in your trash and give me +2 might this turn."
def _pakaa_res(g, it):
    pl = g.p[it.ctrl]
    if not pl.deck:
        return
    c = pl.deck[0]
    g.log(f"  P{it.ctrl} reveals {c.cname}")
    if c.spec["type"] == "Unit":
        _draw_card(g, it.ctrl, c)
    else:
        g.to_zone(c, "trash")
        me = _alive(g, it.src, it.data["oid"])
        if me is not None:
            g.mod(me, 2)


card("Pakaa Protector", on_event=lambda g, o, ev, info: ev == "move" and info["obj"] is o and
     g.queue_trigger(o.ctrl, "Pakaa Protector", _pakaa_res, dict(oid=o.oid), src=o.uid))


# Poro Herder — "When you play me, if you control a Poro, buff me and draw 1."
def _has_poro(g, pid):
    return any("Poro" in u.spec["tags"] for u in g.units(pid))


def _herder_res(g, it):
    if not _has_poro(g, it.ctrl):
        return
    me = _alive(g, it.src, it.data["oid"])
    if me is not None:
        g.buff(me)
    g.draw(it.ctrl, 1)


card("Poro Herder", on_play=lambda g, o, ctx: _has_poro(g, o.ctrl) and
     g.queue_trigger(o.ctrl, "Poro Herder", _herder_res, dict(oid=o.oid), src=o.uid))


# Legion Quartermaster — "As an additional cost to play me, return a friendly gear to its owner's hand."
def _qm_pay(g, pid, c, ch):
    if "ret_gear" in ch:
        x = g.obj(ch["ret_gear"])
    else:                                   # played by an effect (play_unit_free): the cost is still paid
        gs = sorted(g.gear(pid), key=lambda x: (not x.token, x.spec["e"] + 2 * x.spec["p"], x.uid))
        x = g.ask(pid, "return_gear", gs) if gs else None
    if x is not None:
        g.to_zone(x, "hand")


card("Legion Quartermaster", pay_extra=_qm_pay,
     as_played=lambda g, pid, c: [dict(ret_gear=x.uid) for x in sorted(
         g.gear(pid), key=lambda x: (not x.token, x.spec["e"] + 2 * x.spec["p"], x.uid))])


# Ribbon Dancer — "When I move to a battlefield, give another friendly unit +1 might this turn."
def _ribbon(g, o, ev, info):
    if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
        g.queue_trigger(o.ctrl, "Ribbon Dancer", lambda g_, it: g_.legal(it, 0) is not None and
                        g_.mod(g_.legal(it, 0), 1), src=o.uid,
                        choose=trig_target(lambda g_, it: [u for u in friends(g_, it.ctrl) if u.uid != it.src],
                                           lambda g_, it, u: u.ctrl == it.ctrl and u.uid != it.src,
                                           kind="friendly_target"))


card("Ribbon Dancer", on_event=_ribbon)


# Riven, Shattered — "[Weaponmaster] When I attack, choose an enemy unit here. Deal 2 to it for each Equipment
# attached to me."
def _equipment_on(g, u):
    return sum(1 for x in u.attached if g.obj(x) is not None and "Equipment" in g.obj(x).spec["tags"])


def _riven(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc

        def res(g_, it):
            me = _alive(g_, it.src, it.data["oid"])
            u = g_.legal(it, 0)
            if me is not None and u is not None:
                g_.deal(u, 2 * _equipment_on(g_, me), "ability", it.ctrl)
        g.queue_trigger(o.ctrl, "Riven, Shattered", res, dict(oid=o.oid, dmg=2 * _equipment_on(g, o)), src=o.uid,
                        choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                                           lambda g_, it, u: u.ctrl != it.ctrl and u.loc == here, deflect=True))


card("Riven, Shattered", kw={"Weaponmaster": 1}, on_event=_riven)


# Royal Entourage — "When you play me, ready or exhaust a legend."
def _entourage(g, o, ctx):
    def choose(g_, it):
        opts = []
        for q in (it.ctrl, 1 - it.ctrl):
            leg = g_.p[q].legend
            good, bad = ("ready", "exhaust") if q == it.ctrl else ("exhaust", "ready")
            opts.append((0 if (good == "ready") == leg.exhausted else 1, q, good))
            opts.append((2, q, bad))
        opts.sort()
        it.data["leg"], it.data["act"] = g_.ask(it.ctrl, "entourage", [(q, a) for _, q, a in opts])
        return True

    def res(g_, it):
        leg = g_.p[it.data["leg"]].legend
        if it.data["act"] == "ready":
            g_.ready_obj(leg)
        else:
            leg.exhausted = True
    g.queue_trigger(o.ctrl, "Royal Entourage", res, src=o.uid, choose=choose)


card("Royal Entourage", on_play=_entourage)


# Shadow — "If you play me to a battlefield, I enter ready. [Action][>] [1][A], [E]: [Stun] an enemy unit attacking
# here."
def _shadow_pred(g, it, o):
    me = g.obj(it.src)
    return me is not None and o.spec["type"] == "Unit" and o.ctrl != it.ctrl and o.desig == "att" and o.loc == me.loc


card("Shadow", enter_ready=lambda g, pid, c, ch: ch.get("loc") in (0, 1), abilities=[ability(
    "Stun", cost="1 energy and 1 rune of any type", timing="action", exhaust=True, preds=[_shadow_pred],
    resolve=lambda g, it: g.stun(g.legal(it, 0), it.ctrl),
    choices=lambda g, pid, o: tg_choices([u for u in enemies(g, pid) if u.desig == "att" and u.loc == o.loc
                                          and not u.stunned]))])


# Shen, Scourge of Shadows — "When I hold, if there is exactly one other unit you control here, draw 1."
def _shen_ok(g, uid, pid, bf):
    return len([u for u in g.units(pid, bf) if u.uid != uid]) == 1


def _shen(g, o, ev, info):
    if ev == "hold" and _hold(o, info) and _shen_ok(g, o.uid, o.ctrl, info["bf"]):
        bf = info["bf"]
        g.queue_trigger(o.ctrl, "Shen, Scourge of Shadows",
                        lambda g_, it: _shen_ok(g_, it.src, it.ctrl, bf) and g_.draw(it.ctrl, 1), src=o.uid)


card("Shen, Scourge of Shadows", on_event=_shen)


# Simian Ancestor — "When you buff me, ready me." (the 'buff' event has no source player; in the card pool a unit is
# only buffed by its controller's effects, see NEEDS_calm.md)
def _simian(g, o, ev, info):
    if ev == "buff" and info["obj"] is o:
        def res(g_, it):
            me = _alive(g_, it.src, it.data["oid"])
            if me is not None:
                g_.ready_obj(me)
        g.queue_trigger(o.ctrl, "Simian Ancestor", res, dict(oid=o.oid), src=o.uid)


card("Simian Ancestor", on_event=_simian)


# Solari Shieldbearer — "When you play me, stun a unit."
def _shieldbearer(g, o, ctx):
    def opts(g_, it):
        es = enemies(g_, it.ctrl)
        return [u for u in es if not u.stunned] + [u for u in es if u.stunned] + friends(g_, it.ctrl)
    g.queue_trigger(o.ctrl, "Solari Shieldbearer", lambda g_, it: g_.stun(g_.legal(it, 0), it.ctrl), src=o.uid,
                    choose=trig_target(opts, P_unit, deflect=True))


card("Solari Shieldbearer", on_play=_shieldbearer)


# Sona, Harmonious — "While I'm at a battlefield, ready 4 friendly runes at the end of your turn."
def _sona(g, o, ev, info):
    if ev == "end_turn" and info["pid"] == o.ctrl and o.loc in (0, 1):
        g.queue_trigger(o.ctrl, "Sona, Harmonious", lambda g_, it: _ready_runes(g_, it.ctrl, 4), src=o.uid)


card("Sona, Harmonious", on_event=_sona)

# Taric, Protector — "[Shield] [Tank] Other friendly units here have [Shield]."
card("Taric, Protector", kw={"Shield": 1, "Tank": 1},
     aura_kw=lambda g, src, o: ({"Shield": 1} if src in g.board and o is not src and o.ctrl == src.ctrl
                                and o.loc == src.loc and o.spec["type"] == "Unit" else None))

# Tasty Faefolk — "[Accelerate] [Deathknell] Channel 2 runes exhausted and draw 1."
card("Tasty Faefolk", accelerate=True,
     deathknell=lambda g, it: (g.channel(it.ctrl, 2, exhausted=True), g.draw(it.ctrl, 1)))


# Trevor Snoozebottom — "[Shield] When I hold, play a ready 3 might Sprite unit token with [Temporary] here."
def _trevor(g, o, ev, info):
    if ev == "hold" and _hold(o, info):
        bf = info["bf"]
        g.queue_trigger(o.ctrl, "Trevor Snoozebottom",
                        lambda g_, it: make_token(g_, "Sprite", it.ctrl, bf, ready=True), src=o.uid)


card("Trevor Snoozebottom", kw={"Shield": 1}, on_event=_trevor)


# Vex, Mocking — "[Shield] [Tank] When you [Stun] an enemy unit at a battlefield, you may move me to that
# battlefield."
def _vex(g, o, ev, info):
    u = info.get("obj") if ev == "stun" else None
    if u is not None and info["by"] == o.ctrl and u.ctrl != o.ctrl and u.loc in (0, 1) and o.loc != u.loc:
        bf = u.loc

        def res(g_, it):
            me = _alive(g_, it.src, it.data["oid"])
            if me is not None and me.loc != bf:
                g_.move([me], bf, it.ctrl)
        g.queue_trigger(o.ctrl, "Vex, Mocking", res, dict(oid=o.oid), src=o.uid, may=True)


card("Vex, Mocking", kw={"Shield": 1, "Tank": 1}, on_event=_vex)

# Wielder of Water — "While I'm attacking or defending alone, I have +2 might."
card("Wielder of Water", might_mod=lambda g, o: 2 if o.desig in ("att", "def") and g.alone(o) else 0)

# Wizened Elder — "While I'm buffed, I have an additional +1 might."
card("Wizened Elder", might_mod=lambda g, o: 1 if o.buff > 0 else 0)

# Wuju Apprentice — "[Hunt] [Level 6] When you play me, draw 1."
card("Wuju Apprentice", kw={"Hunt": 1},
     on_play=lambda g, o, ctx: g.level(o.ctrl, 6) and
     g.queue_trigger(o.ctrl, "Wuju Apprentice", lambda g_, it: g_.draw(it.ctrl, 1), src=o.uid))


# Yasuo, Remorseful — "When I attack, deal damage equal to my Might to an enemy unit here."
def _yasuo(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc

        def res(g_, it):
            me = _alive(g_, it.src, it.data["oid"])
            u = g_.legal(it, 0)
            if me is not None and u is not None:            # Might checked on execution (rule 359.3.f.2)
                g_.deal(u, g_.might(me), "ability", it.ctrl)
        g.queue_trigger(o.ctrl, "Yasuo, Remorseful", res, dict(oid=o.oid, dmg=g.might(o)), src=o.uid,
                        choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                                           lambda g_, it, u: u.ctrl != it.ctrl and u.loc == here, deflect=True))


card("Yasuo, Remorseful", on_event=_yasuo)


# Yuumi, Magical Cat — "[Tank] When I attack or defend, give one of your other units here +3 might and [Tank] this
# turn."
def _yuumi(g, o, ev, info):
    if ev in ("attack", "defend") and info["obj"] is o:
        here = o.loc

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.mod(u, 3)
                g_.grant(u, "Tank", 1)
        g.queue_trigger(o.ctrl, "Yuumi, Magical Cat", res, src=o.uid,
                        choose=trig_target(lambda g_, it: [u for u in friends(g_, it.ctrl)
                                                           if u.loc == here and u.uid != it.src],
                                           lambda g_, it, u: u.ctrl == it.ctrl and u.loc == here,
                                           kind="friendly_target"))


card("Yuumi, Magical Cat", kw={"Tank": 1}, on_event=_yuumi)


# ====================================================================== gear and equipment
# (Might Bonus and Effect Text of Equipment read on the card images, see GUIDE.md section 2)
# Brutalizer — "[Equip] [Calm]" ; Might Bonus +1 ; Effect Text: "If this was attached to me this turn, I have an
# additional +2 might."
def _brut_event(g, o, ev, info):
    if ev == "attached" and info["gear"] is o:
        u = info["unit"]
        g.stats[("calm_brut", o.uid, o.oid, u.uid, u.oid, g.turn_no)] = 1


def _brut_aura(g, src, o):
    if src.attached_to != o.uid or src not in g.board:
        return 0
    return 2 if g.stats[("calm_brut", src.uid, src.oid, o.uid, o.oid, g.turn_no)] else 0


card("Brutalizer", equip="1 calm rune", bonus=1, on_event=_brut_event, aura_might=_brut_aura)

# Doran's Shield — "[Equip] [Calm]" ; Might Bonus +1 ; Effect Text: "[Tank]".
card("Doran's Shield", equip="1 calm rune", bonus=1, equip_kw={"Tank": 1})


# Hand Hammer — "[Equip] [Calm]" ; Might Bonus +1 ; Effect Text: "I have +2 might while I'm at a battlefield with
# exactly one other unit you control."
def _hammer_aura(g, src, o):
    if src.attached_to != o.uid or src not in g.board or o.loc not in (0, 1):
        return 0
    return 2 if len([u for u in g.units(src.ctrl, o.loc) if u is not o]) == 1 else 0


card("Hand Hammer", equip="1 calm rune", bonus=1, aura_might=_hammer_aura)

# Soul Sword — "[Equip] [Calm]" ; Might Bonus +1 ; Effect Text: "[Level 3] I have an additional +1 might."
card("Soul Sword", equip="1 calm rune", bonus=1,
     aura_might=lambda g, src, o: 1 if src.attached_to == o.uid and src in g.board and g.p[src.ctrl].xp >= 3 else 0)


# Forgefire Cape — "[Unique] [Equip] [A]" ; Might Bonus +3 ; Effect Text: "When I attack or defend, deal 2 to all
# enemy units here."
def _cape_effect(g, gear, unit, ev, info):
    if ev in ("attack", "defend") and info["obj"] is unit:
        here = unit.loc

        def res(g_, it):
            for u in g_.units(1 - it.ctrl, here):
                g_.deal(u, 2, "ability", it.ctrl)
        g.queue_trigger(unit.ctrl, "Forgefire Cape", res, src=unit.uid)


card("Forgefire Cape", equip="1 rune of any type", bonus=3, effect_event=_cape_effect)


# Shurelya's Requiem — "[Unique] [Equip] [A] When you play this, ready your units." ; Might Bonus +2 ;
# Effect Text: "Your units here have [Ganking]."
def _shurelya_aura(g, src, o):
    u = g.obj(src.attached_to) if src.attached_to is not None else None
    if u is None or src not in g.board or o.spec["type"] != "Unit" or o.ctrl != src.ctrl or o.loc != u.loc:
        return None
    return {"Ganking": 1}


card("Shurelya's Requiem", equip="1 rune of any type", bonus=2, aura_kw=_shurelya_aura,
     on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Shurelya's Requiem", lambda g_, it: [
         g_.ready_obj(u) for u in g_.units(it.ctrl)], src=o.uid))

# Heart of Dark Ice — "[E]: Give a unit +3 might this turn."
card("Heart of Dark Ice", abilities=[ability(
    "Chill", exhaust=True, preds=[P_unit], resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 3),
    choices=lambda g, pid, o: tg_choices(_units_first(g, pid)))])

# Honeyfruit — "This enters exhausted. [Reaction][>] [E]: [Add] [A]. [Level 6][>] [Reaction][>] [E]: [Add] [1][A]."
# ([Level 6] replaces the first ability: only one of them exists at a time)
card("Honeyfruit", enter_exhausted=True, add=[
    add_ability("1 rune of any type", can=lambda g, pid, o, ctx: g.p[pid].xp < 6),
    add_ability("1 energy and 1 rune of any type", can=lambda g, pid, o, ctx: g.p[pid].xp >= 6)])


# Mask of Foresight — "When a friendly unit attacks or defends alone, give it +1 might this turn."
def _mask(g, o, ev, info):
    if ev in ("attack", "defend") and info["obj"].ctrl == o.ctrl and g.alone(info["obj"]):
        u = info["obj"]

        def res(g_, it):
            x = _alive(g_, it.data["u"], it.data["oid"])
            if x is not None:
                g_.mod(x, 1)
        g.queue_trigger(o.ctrl, "Mask of Foresight", res, dict(u=u.uid, oid=u.oid), src=o.uid)


card("Mask of Foresight", on_event=_mask)

# Poro Snax — "When you play this, draw 1. [1][Calm], [E], Kill this: Draw 1."
card("Poro Snax",
     on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Poro Snax", lambda g_, it: g_.draw(it.ctrl, 1), src=o.uid),
     abilities=[ability("Snack", cost="1 energy and 1 calm rune", exhaust=True, kill_self=True,
                        resolve=lambda g, it: g.draw(it.ctrl, 1))])

# Seal of Focus — "[E]: [Reaction] — [Add] [Calm]."
card("Seal of Focus", add=[add_ability("1 calm rune")])

# Spirit's Refuge — "When you play this, buff a friendly unit. Friendly buffed units have [Deflect] if they didn't
# already."
_REFUGE_GUARD = [False]


def _refuge_aura(g, src, o):
    if _REFUGE_GUARD[0] or src not in g.board or o.spec["type"] != "Unit" or o.ctrl != src.ctrl or o.buff <= 0:
        return None
    first = min((x for x in g.board if x.cname == "Spirit's Refuge" and x.ctrl == o.ctrl), key=lambda x: x.uid)
    if first is not src:
        return None                                       # "if they didn't already": one Deflect at most
    _REFUGE_GUARD[0] = True
    try:
        has = g.kw_value(o, "Deflect") > 0
    finally:
        _REFUGE_GUARD[0] = False
    return None if has else {"Deflect": 1}


def _refuge_play(g, o, ctx):
    g.queue_trigger(o.ctrl, "Spirit's Refuge", lambda g_, it: g_.buff(g_.legal(it, 0)), src=o.uid,
                    choose=trig_target(lambda g_, it: sorted(friends(g_, it.ctrl), key=lambda u: u.buff > 0),
                                       P_friend, kind="friendly_target"))


card("Spirit's Refuge", on_play=_refuge_play, aura_kw=_refuge_aura)


# Forgotten Signpost — "[Action][>] Exhaust a unit you control, [E]: Move a different unit you control to the
# location of the unit you exhausted to pay for this ability."
def _signpost_choices(g, pid, o):
    out = []
    for e in friends(g, pid):
        if e.exhausted:
            continue
        for m in friends(g, pid):
            if m is not e and m.loc != e.loc:
                out.append(dict(exh=e.uid, exh_oid=e.oid, tg=(m.uid,)))
    out.sort(key=lambda c: (g.obj(c["exh"]).loc not in (0, 1), value(g, g.obj(c["exh"]))))
    return out


def _signpost_pay(g, pid, o, ch):
    e = g.obj(ch["exh"])
    if e is not None:
        e.exhausted = True


def _signpost_res(g, it):
    e = _alive(g, it.data["exh"], it.data["exh_oid"])
    m = g.legal(it, 0)
    if e is None or m is None or m.loc == e.loc:
        return                                            # null location (rule 359.3.e.12)
    g.move([m], e.loc, it.ctrl)


card("Forgotten Signpost", abilities=[ability(
    "Signpost", timing="action", exhaust=True, preds=[P_friend], choices=_signpost_choices,
    extra_cost=_signpost_pay, resolve=_signpost_res)])


# ====================================================================== spells
# Alpha Strike — "[Action] Choose a friendly unit. It deals damage equal to its Might split among enemy units at
# battlefields. Then for each unit this kills, do this: Gain 1 XP."  (splitting: rule 355.14)
def _alpha_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    out = []
    es_all = enemies(g, pid, True, hb)
    for f in sorted(friends(g, pid), key=lambda u: -g.might(u))[:3]:
        m = g.might(f)
        if m <= 0 or not es_all:
            continue
        es = sorted(es_all, key=lambda u: (max(1, g.might(u) - u.damage), -value(g, u)))
        pick, left = [], m
        for e in es:
            need = max(1, g.might(e) - e.damage)
            if need <= left and len(pick) < m:
                pick.append(e)
                left -= need
        if pick:
            out.append(dict(tg=(f.uid,) + tuple(e.uid for e in pick)))
        for e in es_all[:2]:
            c = dict(tg=(f.uid, e.uid))
            if c not in out:
                out.append(c)
    return out


def _alpha(g, it):
    f = g.legal(it, 0)
    if f is None:
        return
    ts = [g.legal(it, i) for i in range(1, len(it.targets))]
    ts = [t for t in ts if t is not None]
    hit = _split_damage(g, it.ctrl, g.might(f), ts, "unit", it.ctrl)
    killed = _kill_lethal(g, hit, it.ctrl)
    g.gain_xp(it.ctrl, len(killed))


card("Alpha Strike", timing="action", preds=[P_friend, P_enemy_bf], resolve=_alpha, choices=_alpha_choices)


# Arise! — "Play a 2 might Sand Soldier unit token for each Equipment you control. Then ready two of them."
def _arise(g, it):
    n = len([x for x in g.gear(it.ctrl) if "Equipment" in x.spec["tags"]])
    toks = [_play_token(g, "Sand Soldier", it.ctrl) for _ in range(n)]
    for t in toks[:2]:
        g.ready_obj(t)


card("Arise!", resolve=_arise)

# Combat Experience — "[Reaction] Give a unit +1 might this turn. [Level 6][>] Give it +3 might this turn instead."
card("Combat Experience", timing="reaction", preds=[P_unit],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 3 if g.level(it.ctrl, 6) else 1),
     choices=lambda g, pid, ctx: tg_choices(_units_first(g, pid, ctx["hidden_bf"])))


# Defiant Dance — "[Reaction] Give a unit +2 might this turn and another unit -2 might this turn."
def _dance(g, it):
    a, b = g.legal(it, 0), g.legal(it, 1)
    if a is not None:
        g.mod(a, 2)
    if b is not None:
        g.mod(b, -2)


def _dance_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    fr, en = friends(g, pid, False, hb), enemies(g, pid, False, hb)
    out = [dict(tg=(a.uid, b.uid)) for a in fr[:3] for b in en[:3]]
    us = fr + en
    out += [dict(tg=(a.uid, b.uid)) for a in us for b in us if a is not b and dict(tg=(a.uid, b.uid)) not in out]
    return out


card("Defiant Dance", timing="reaction", preds=[P_unit, P_unit], resolve=_dance, choices=_dance_choices)

# Desert's Call — "[Repeat] [2] Play a 2 might Sand Soldier unit token."
card("Desert's Call", repeat=repeat_cost("2 energy"),
     resolve=repeatable(lambda g, it: _play_token(g, "Sand Soldier", it.ctrl)))

# Double Trouble — "[Repeat] [2] Look at the top 3 cards of your Main Deck. You may reveal a unit from among them and
# draw it. Recycle the rest."
card("Double Trouble", repeat=repeat_cost("2 energy"),
     resolve=repeatable(lambda g, it: _look_pick(g, it.ctrl, 3, lambda c: c.spec["type"] == "Unit", "double_pick")))


# Dragon's Rage — "Move an enemy unit. Then choose another enemy unit at its destination. They deal damage equal to
# their Mights to each other."  (both enemy units are chosen as it is played, rule 355.5)
def _rage(g, it):
    a = g.legal(it, 0)
    if a is None:
        return                                            # linked instructions (rule 359.3.e.14.a)
    if a.loc != it.data["dest"]:
        g.move([a], it.data["dest"], it.ctrl)
    b = g.legal(it, 1)
    if b is None or a not in g.board or b.loc != a.loc:
        return
    ma, mb = g.might(a), g.might(b)
    g.deal(b, ma, "unit", a.ctrl)                     # rule 417.6.b.3-4: the units are the sources
    g.deal(a, mb, "unit", b.ctrl)


def _rage_choices(g, pid, ctx):
    es = enemies(g, pid)
    out = []
    for a in es[:4]:
        for d in ("base", 0, 1):
            if d == a.loc:
                continue
            for b in es:
                if b is not a and b.loc == d:
                    out.append(dict(tg=(a.uid, b.uid), dest=d))

    def score(c):
        a, b = g.obj(c["tg"][0]), g.obj(c["tg"][1])
        s = 0
        if g.might(a) >= g.might(b) - b.damage:
            s -= value(g, b)
        if g.might(b) >= g.might(a) - a.damage:
            s -= value(g, a)
        return s
    out.sort(key=score)
    return out


card("Dragon's Rage", preds=[P_enemy, P_enemy], resolve=_rage, choices=_rage_choices)


# Emperor's Divide — "[Hidden] [Action] Move any number of friendly units at a battlefield to their base."
def _divide(g, it):
    us = [g.legal(it, i) for i in range(len(it.targets))]
    us = [u for u in us if u is not None]
    if us:
        g.move(us, "base", it.ctrl)


def _divide_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    out = []
    for b in (0, 1):
        if hb is not None and b != hb:
            continue
        us = friends(g, pid, True, b)
        if not us:
            continue
        out.append(dict(tg=tuple(u.uid for u in us)))
        if len(us) > 1:
            out += [dict(tg=(u.uid,)) for u in us]
    return out + [dict(tg=())]


card("Emperor's Divide", timing="action", hidden=True, preds=[P_friend_bf], resolve=_divide, choices=_divide_choices)

# Feral Strength — "[Reaction] [Repeat] [2] Give a unit +2 might this turn."
card("Feral Strength", timing="reaction", repeat=repeat_cost("2 energy"), preds=[P_unit],
     resolve=repeatable(lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 2)),
     choices=lambda g, pid, ctx: tg_choices(_units_first(g, pid, ctx["hidden_bf"])))

# Find Your Center — "[Action] If an opponent's score is within 3 points of the Victory Score, this costs [2] less.
# Draw 1 and channel 1 rune exhausted."
card("Find Your Center", timing="action",
     cost_mod=lambda g, pid, c, ch: (2, 0) if _near_victory(g, pid) else (0, 0),
     resolve=lambda g, it: (g.draw(it.ctrl, 1), g.channel(it.ctrl, 1, exhausted=True)))


# Flurry of Feathers — "[Reaction] Choose one — Counter a spell. / Play four 1 might Bird unit tokens with
# [Deflect]."
def _flurry(g, it):
    if it.data.get("mode") == "counter":
        _counter_spell(g, it)
    else:
        for _ in range(4):
            _play_token(g, "Bird", it.ctrl)


card("Flurry of Feathers", timing="reaction", resolve=_flurry,
     choices=lambda g, pid, ctx: [dict(mode="counter", item=i.id) for i in _spell_items(g, pid)][:6]
     + [dict(mode="birds")])


# Fox-Fire — "[Hidden] [Action] Kill any number of units at a battlefield with total Might 4 or less."
# Group targeting (rule 355.11.b): if the targets no longer qualify as it resolves, the controller chooses a subset
# of them that does.
def _foxfire_ok(g, grp):
    return grp and len(set(u.loc for u in grp)) == 1 and grp[0].loc in (0, 1) and sum(g.might(u) for u in grp) <= 4


def _foxfire(g, it):
    us = [g.obj(uid) for uid, oid, _ in it.targets]
    us = [u for u, (_, oid, _) in zip(us, it.targets) if u is not None and u.oid == oid and g.targetable(u, it.ctrl)
          and u.spec["type"] == "Unit"]
    if not us:
        return
    if not _foxfire_ok(g, us):
        subs = []
        for k in range(len(us), 0, -1):
            for grp in combinations(us, k):
                if _foxfire_ok(g, list(grp)):
                    subs.append(grp)
        if not subs:
            return
        subs.sort(key=lambda grp: -sum(value(g, u) * (1 if u.ctrl != it.ctrl else -1) for u in grp))
        us = list(g.ask(it.ctrl, "foxfire_subset", subs))
    g.kill(us, it.ctrl)


def _foxfire_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    out = []
    for b in (0, 1):
        if hb is not None and b != hb:
            continue
        es = enemies(g, pid, True, b)[:6]
        cands = []
        for k in range(1, len(es) + 1):
            for grp in combinations(es, k):
                if sum(g.might(u) for u in grp) <= 4:
                    cands.append(grp)
        cands.sort(key=lambda grp: -sum(value(g, u) for u in grp))
        out += [dict(tg=tuple(u.uid for u in grp)) for grp in cands[:4]]
    return out


card("Fox-Fire", timing="action", hidden=True, preds=[P_unit], resolve=_foxfire, choices=_foxfire_choices)

# Friendship — "[Reaction] Choose a unit. Give it +1 might this turn for each of the following tags among your
# units — Bird, Cat, Dog, and Poro."
card("Friendship", timing="reaction", preds=[P_unit],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), len(_pet_tags(g, it.ctrl))),
     choices=lambda g, pid, ctx: tg_choices(_units_first(g, pid, ctx["hidden_bf"])))


# Last Breath — "[Action] Ready a friendly unit. It deals damage equal to its Might to an enemy unit at a
# battlefield."
def _last_breath(g, it):
    f, e = g.legal(it, 0), g.legal(it, 1)
    if f is None:
        return
    g.ready_obj(f)
    if e is not None:
        g.deal(e, g.might(f), "unit", it.ctrl)


def _lb_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    out = [dict(tg=(f.uid, e.uid)) for f in friends(g, pid, False, hb)[:3] for e in enemies(g, pid, True, hb)[:3]]

    def score(c):
        f, e = g.obj(c["tg"][0]), g.obj(c["tg"][1])
        return -(value(g, e) if g.might(f) >= g.might(e) - e.damage else 0)
    out.sort(key=score)
    return out


card("Last Breath", timing="action", preds=[P_friend, P_enemy_bf], resolve=_last_breath, choices=_lb_choices)


# Last Stand — "[Action] Double a friendly unit's Might this turn. Give it [Temporary]."
def _last_stand(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.mod(u, g.might(u))
        g.grant(u, "Temporary", 1, None)


card("Last Stand", timing="action", preds=[P_friend], resolve=_last_stand,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid, False, ctx["hidden_bf"])))


# Meditation — "[Reaction] As an additional cost to play this, you may exhaust a friendly unit. If you do, draw 2.
# Otherwise, draw 1."
def _med_pay(g, pid, c, ch):
    u = g.obj(ch["exh"]) if ch.get("exh") else None
    if u is not None:
        u.exhausted = True


card("Meditation", timing="reaction", pay_extra=_med_pay,
     resolve=lambda g, it: g.draw(it.ctrl, 2 if it.data.get("exh") else 1),
     choices=lambda g, pid, ctx: [dict(exh=u.uid) for u in sorted(g.units(pid), key=lambda u: value(g, u))
                                  if not u.exhausted][:4] + [dict()])


# Party Favors — "Each other player chooses Cards or Runes. For each player that chooses Cards, you and that player
# each draw 1. For each player that chooses Runes, you and that player each channel 1 rune exhausted."
def _favors(g, it):
    opp = 1 - it.ctrl
    pick = g.ask(opp, "party_favors", ["cards", "runes"])
    for q in (it.ctrl, opp):
        if pick == "cards":
            g.draw(q, 1)
        else:
            g.channel(q, 1, exhausted=True)


card("Party Favors", resolve=_favors)


# Reinforce — "Look at the top 5 cards of your Main Deck. You may banish a unit from among them, then play it,
# reducing its cost by [5]. Recycle the remaining cards."
def _reinforce(g, it):
    pid = it.ctrl
    pl = g.p[pid]
    top = pl.deck[:5]
    opts = []
    for c in top:
        im = g.impl(c)
        if c.spec["type"] != "Unit" or im is None:
            continue
        extras = im.as_played(g, pid, c) if im.as_played else [{}]
        for loc in _token_locs(g, pid):
            for ex in extras:
                for acc in ([False, True] if im.accelerate else [False]):
                    ch = dict(ex, loc=loc, acc=acc)
                    e, reqs = total_cost(g, pid, c, ch, "banish")
                    if g.can_pay(pid, max(0, e - 5), reqs, pay_ctx(c)):
                        opts.append((c, ch))
    opts.sort(key=lambda o: (-(o[0].spec["e"] + 2 * o[0].spec["p"]), o[1]["loc"] != "base", o[1]["acc"]))
    pick = g.ask(pid, "reinforce_pick", opts[:12] + [None])
    rest = [c for c in top if pick is None or c is not pick[0]]
    if pick is not None:
        c, ch = pick
        e, reqs = total_cost(g, pid, c, ch, "banish")
        g.to_zone(c, "banish")
        g.log(f"  Reinforce banishes and plays {c.cname}")
        if g.pay(pid, max(0, e - 5), reqs, pay_ctx(c)):
            play_card(g, pid, c, "banish", dict(ch), limited=True)
    for c in rest:
        if c in pl.deck:
            pl.deck.remove(c)
    g.recycle_cards(pid, rest)


card("Reinforce", resolve=_reinforce)


# Resonating Strike — "[Hidden] [Reaction] Choose a battlefield you control and a unit you control at a different
# location. Move that unit to that battlefield and give it +2 might this turn."
def _resonating(g, it):
    u = g.legal(it, 0)
    b = it.data["bf"]
    if u is None:
        return
    if g.bfs[b].ctrl == it.ctrl and u.loc != b:
        g.move([u], b, it.ctrl)
    g.mod(u, 2)


def _resonating_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    out = []
    for b in g.bfs:
        if b.ctrl != pid or (hb is not None and b.idx != hb):
            continue
        for u in friends(g, pid):
            if u.loc != b.idx:
                out.append(dict(bf=b.idx, tg=(u.uid,)))
    return out


card("Resonating Strike", timing="reaction", hidden=True, preds=[P_friend], resolve=_resonating,
     choices=_resonating_choices)

# Rune Prison — "[Action] Stun a unit."
card("Rune Prison", timing="action", preds=[P_unit], resolve=lambda g, it: g.stun(g.legal(it, 0), it.ctrl),
     choices=lambda g, pid, ctx: tg_choices([u for u in enemies(g, pid, False, ctx["hidden_bf"]) if not u.stunned]
                                            + [u for u in enemies(g, pid, False, ctx["hidden_bf"]) if u.stunned]
                                            + friends(g, pid, False, ctx["hidden_bf"])))


# Sanction — "[Reaction] Choose one — Empower a unit. Disempower it at end of turn. / Disempower a unit that's
# [Empowered]. Empower it at end of turn."
def _sanction_end(g, eff, info):
    g.effects.remove(eff)

    def res(g_, it):
        u = _alive(g_, it.data["u"], it.data["oid"])
        if u is None:
            return
        if it.data["mode"] == "emp":
            g_.disempower(u)
        else:
            g_.empower(u)
    g.queue_trigger(eff["pid"], "Sanction (end of turn)", res, dict(u=eff["u"], oid=eff["oid"], mode=eff["mode"]))


def _sanction(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    mode = it.data["mode"]
    if mode == "emp":
        g.empower(u)
    else:
        g.disempower(u)
    g.effects.append(dict(on="end_turn", fn=_sanction_end, pid=it.ctrl, u=u.uid, oid=u.oid, mode=mode, dur="turn"))


def _sanction_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    fr, en = friends(g, pid, False, hb), enemies(g, pid, False, hb)
    return ([dict(tg=(u.uid,), mode="dis") for u in en if u.empowered]
            + [dict(tg=(u.uid,), mode="emp") for u in fr if not u.empowered]
            + [dict(tg=(u.uid,), mode="emp") for u in en if not u.empowered]
            + [dict(tg=(u.uid,), mode="dis") for u in fr if u.empowered])


card("Sanction", timing="reaction", resolve=_sanction, choices=_sanction_choices,
     preds=[lambda g, it, o: P_unit(g, it, o) and (it.data.get("mode") == "emp" or o.empowered)])


# Shadow Dash — "Move an enemy unit to a battlefield where you have units. If you have exactly two units there, they
# each get +1 might this turn. [Flow] [5][A][A]"
def _dash(g, it):
    u = g.legal(it, 0)
    d = it.data["dest"]
    if u is not None and u.loc != d and g.units(it.ctrl, d):
        g.move([u], d, it.ctrl)
    mine = g.units(it.ctrl, d)
    if len(mine) == 2:
        for x in mine:
            g.mod(x, 1)


def _dash_choices(g, pid, ctx):
    out = []
    for u in enemies(g, pid):
        for b in (0, 1):
            if u.loc != b and g.units(pid, b):
                out.append(dict(tg=(u.uid,), dest=b))
    out.sort(key=lambda c: (len(g.units(pid, c["dest"])) != 2, -value(g, g.obj(c["tg"][0]))))
    return out


card("Shadow Dash", preds=[P_enemy], resolve=_dash, choices=_dash_choices,
     flow=flow_cost("5 energy and 2 runes of any type"))


# Siphoning Strike — "Deal 4 to a unit at a battlefield. If you control 7 or more runes, deal 7 to it instead.
# When it dies this turn, channel 1 rune exhausted."
def _siphon_die(g, eff, info):
    o = info["info"]["obj"]
    if o.uid != eff["u"] or o.oid != eff["oid"] + 1:
        return
    g.effects.remove(eff)
    g.queue_trigger(eff["pid"], "Siphoning Strike (dies)", lambda g_, it: g_.channel(it.ctrl, 1, exhausted=True))


def _siphon(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    g.effects.append(dict(on="die", fn=_siphon_die, pid=it.ctrl, u=u.uid, oid=u.oid, dur="turn"))
    g.deal(u, 7 if len(g.p[it.ctrl].runes) >= 7 else 4, "spell", it.ctrl)


card("Siphoning Strike", preds=[P_unit_bf], resolve=_siphon,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# Skyward Strike — "Move an enemy unit. [Level 6][>] [Stun] an enemy unit."
def _skyward(g, it):
    u = g.legal(it, 0)
    if u is not None and u.loc != it.data["dest"]:
        g.move([u], it.data["dest"], it.ctrl)
    if len(it.targets) > 1 and g.level(it.ctrl, 6):
        g.stun(g.legal(it, 1), it.ctrl)


def _skyward_choices(g, pid, ctx):
    es = enemies(g, pid)
    lv = g.level(pid, 6)
    out = []
    for u in es[:4]:
        for d in ("base", 0, 1):
            if d == u.loc:
                continue
            if lv:
                for s in ([x for x in es if not x.stunned][:2] or es[:1]):
                    out.append(dict(tg=(u.uid, s.uid), dest=d))
            else:
                out.append(dict(tg=(u.uid,), dest=d))
    out.sort(key=lambda c: (c["dest"] != "base", -value(g, g.obj(c["tg"][0]))))
    return out


card("Skyward Strike", preds=[P_enemy, P_enemy], resolve=_skyward, choices=_skyward_choices)

# Thwonk! — "[Action] [Repeat] [2] Stun an attacking unit."
card("Thwonk!", timing="action", repeat=repeat_cost("2 energy"),
     preds=[lambda g, it, o: o.spec["type"] == "Unit" and o.desig == "att" and hb_ok(it, o)],
     resolve=repeatable(lambda g, it: g.stun(g.legal(it, 0), it.ctrl)),
     choices=lambda g, pid, ctx: tg_choices([u for u in enemies(g, pid, False, ctx["hidden_bf"]) + friends(
         g, pid, False, ctx["hidden_bf"]) if u.desig == "att"]))


# Tricksy Tentacles — "Move any number of enemy units with the same controller and a total Might of 8 or less to a
# single location."  (group targeting, rule 355.11)
def _tricksy(g, it):
    us = [g.legal(it, i) for i in range(len(it.targets))]
    us = [u for u in us if u is not None]
    if not us:
        return
    if len(set(u.ctrl for u in us)) > 1 or sum(g.might(u) for u in us) > 8:
        subs = [grp for k in range(len(us), 0, -1) for grp in combinations(us, k)
                if len(set(u.ctrl for u in grp)) == 1 and sum(g.might(u) for u in grp) <= 8]
        if not subs:
            return
        subs.sort(key=lambda grp: -sum(value(g, u) for u in grp))
        us = list(g.ask(it.ctrl, "tricksy_subset", subs))
    movers = [u for u in us if u.loc != it.data["dest"]]
    if movers:
        g.move(movers, it.data["dest"], it.ctrl)


def _tricksy_choices(g, pid, ctx):
    es = enemies(g, pid)[:6]
    grps = []
    for k in range(len(es), 0, -1):
        for grp in combinations(es, k):
            if sum(g.might(u) for u in grp) <= 8:
                grps.append(grp)
    grps.sort(key=lambda grp: -sum(value(g, u) for u in grp if u.loc in (0, 1)))
    out = []
    for grp in grps[:3]:
        for d in ("base", 0, 1):
            if any(u.loc != d for u in grp):
                out.append(dict(tg=tuple(u.uid for u in grp), dest=d))
    out.sort(key=lambda c: c["dest"] != "base")
    return out


card("Tricksy Tentacles", preds=[P_enemy], resolve=_tricksy, choices=_tricksy_choices)

# Wind Wall — "[Reaction] Counter a spell."
card("Wind Wall", timing="reaction", resolve=_counter_spell,
     choices=lambda g, pid, ctx: [dict(item=i.id) for i in _spell_items(g, pid)])


# Zenith Blade — "[Action] Stun an enemy unit at a battlefield. You may move a friendly unit to that enemy unit's
# battlefield."
def _zenith(g, it):
    e = g.legal(it, 0)
    if e is None:
        return
    g.stun(e, it.ctrl)
    m = g.legal(it, 1)
    if m is not None and e.loc in (0, 1) and m.loc != e.loc:
        g.move([m], e.loc, it.ctrl)


def _zenith_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    es = enemies(g, pid, True, hb)
    out = []
    for e in [u for u in es if not u.stunned] + [u for u in es if u.stunned]:
        out.append(dict(tg=(e.uid,)))
        for m in friends(g, pid)[:2]:
            if m.loc != e.loc:
                out.append(dict(tg=(e.uid, m.uid)))
    return out


card("Zenith Blade", timing="action", preds=[P_enemy_bf, P_friend], resolve=_zenith, choices=_zenith_choices)
