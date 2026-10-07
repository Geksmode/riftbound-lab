"""Batch fury: the Fury cards listed in batches/fury.txt (plus the Shadow Clone token, rule 187.11).
Cards that need an engine hook are not registered: see NEEDS_fury.md. Rule numbers: core rules 2026-07-16."""
from cards import *  # noqa: F401,F403
from actions import total_cost, pay_ctx

FURY = frozenset({"Fury"})


# ====================================================================== private helpers
def _hand_others(g, pid, card):
    return [c for c in g.p[pid].hand if c is not card]


def _discard_one(g, pid, kind="discard", exclude=None, order=None):
    """Discard 1 card of pid's choice (rule 422). Returns the discarded card or None."""
    cs = [c for c in g.p[pid].hand if c is not exclude]
    if not cs:
        return None
    if order is not None:
        cs = sorted(cs, key=order)
    c = g.ask(pid, kind, cs)
    g.discard(pid, c)
    return c


def _discard_n(n):
    def res(g, it):
        for _ in range(n):
            _discard_one(g, it.ctrl)
    return res


def _killable(g, u, n):
    return g.might(u) - u.damage + u.prevent <= n


def _dmg_targets(g, pid, n, at_bf=True, hb=None, enemies_only=False):
    """Units a 'deal n' effect may choose, best first for the AI: enemies it kills, other enemies, then friends."""
    us = [u for u in g.units() if (not at_bf or u.loc in (0, 1)) and g.targetable(u, pid)
          and (hb is None or u.loc == hb) and (not enemies_only or u.ctrl != pid)]
    return sorted(us, key=lambda u: (u.ctrl == pid, not _killable(g, u, n), -value(g, u), u.uid))


def _buff_targets(g, pid, at_bf=False, hb=None):
    """Units a beneficial effect may choose, best first for the AI: friends (at battlefields first), then enemies."""
    us = [u for u in g.units() if (not at_bf or u.loc in (0, 1)) and g.targetable(u, pid)
          and (hb is None or u.loc == hb)]
    return sorted(us, key=lambda u: (u.ctrl != pid, u.loc not in (0, 1), -value(g, u), u.uid))


def _me(g, it):
    return g.obj(it.src)


def _unit_locs(g, pid):
    """Where a unit (or unit token) can be played by default: base or a battlefield its controller controls."""
    return ["base"] + [b.idx for b in g.bfs if b.ctrl == pid]


def _token(g, pid, name, ready=False, kind="token_location"):
    """'Play a ... unit token' without a location: the player picks a legal location (base first)."""
    loc = g.ask(pid, kind, _unit_locs(g, pid))
    return make_token(g, name, pid, loc, ready=ready)


def _gold(g, pid):
    """'Play a Gold gear token exhausted.' (gear is played to base, rule 149.2)"""
    return make_token(g, "Gold", pid, "base", ready=False)


def _detach(g, gear):
    """Detach an Equipment (rule 434): it stays where it is; cleanup recalls unattached gear at battlefields."""
    u = g.obj(gear.attached_to) if gear.attached_to is not None else None
    if u is not None and gear.uid in u.attached:
        u.attached.remove(gear.uid)
    gear.attached_to = None
    g.need_cleanup = True


def _play_choices(g, pid, card, src, free=False, discount_e=0, extra_reqs=(), locs=None, extra_ch=None):
    """Legal (choice, energy, reqs) to play card as a limited play (rule 419.3): ignoring its cost (free) or for its
    cost reduced by discount_e energy, plus extra_reqs (alternative cost such as 'for 1 rune of any type').
    Mandatory additional costs (Deflect...) still apply."""
    im = g.impl(card)
    if im is None:
        return []
    typ = card.spec["type"]
    if typ == "Unit":
        base = [dict(loc=l) for l in (locs if locs is not None else _unit_locs(g, pid))]
    elif typ == "Gear":
        base = [dict(loc="base")]
    elif typ == "Spell":
        base = im.choices(g, pid, dict(hidden_bf=None, card=card)) if im.choices else [dict()]
        base = list(base)[:8]
    else:
        return []
    out = []
    for ch in base:
        ch = dict(ch, **(extra_ch or {}))
        if free:
            ch["free"] = True
        e, reqs = total_cost(g, pid, card, ch, src)
        e = max(0, e - discount_e)
        reqs = list(reqs) + list(extra_reqs)
        if g.can_pay(pid, e, reqs, pay_ctx(card)):
            out.append((ch, e, reqs))
    return out


def _play_from(g, pid, card, src, cands):
    """Pay and play one of the candidates of _play_choices (asked to pid)."""
    if not cands:
        return None
    ch = g.ask(pid, "play_choice", [c[0] for c in cands], card=card)
    _, e, reqs = next(c for c in cands if c[0] is ch)
    if not g.pay(pid, e, reqs, pay_ctx(card)):
        return None
    return play_card(g, pid, card, src, dict(ch), limited=True)


def _replay_offer(g, it, pid, kind, extra_ch=None):
    """Offer made by a resolving spell: once its card has left the chain (its 'played' event, rule 419.4.a), pid may
    play it again from the trash for 1 rune of any type (Dancing Grenade, Death from Below)."""
    g.effects.append(dict(on="played", fn=_replay_fire, item=it.id, card=it.card.uid, pid=pid, kind=kind,
                          extra=dict(extra_ch or {}), dur="turn"))


def _replay_fire(g, eff, info):
    item = info.get("item")
    if item is None or item.id != eff["item"]:
        return
    g.effects.remove(eff)
    c = info["card"]
    if c.uid != eff["card"] or c.zone != "trash":
        return
    pid = eff["pid"]
    cands = _play_choices(g, pid, c, "trash", free=True, extra_reqs=[ANY], extra_ch=eff["extra"])
    if not cands or not g.ask(pid, "may", [True, False], reason=eff["kind"]):
        return
    _play_from(g, pid, c, "trash", cands)


# ====================================================================== tokens
# Shadow Clone — "When I attack, you may banish a unit from your trash. If you do, give me [Assault 4] this turn."
# (rule 187.11)
def _clone_event(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        def res(g_, it):
            me = _me(g_, it)
            us = [c for c in g_.p[it.ctrl].trash if c.spec["type"] == "Unit"]
            if me is None or not us:
                return
            c = g_.ask(it.ctrl, "clone_banish", us + [None])
            if c is None:
                return
            g_.to_zone(c, "banish")
            g_.grant(me, "Assault", 4, "turn")
        g.queue_trigger(o.ctrl, "Shadow Clone", res, src=o.uid)


card("Shadow Clone", on_event=_clone_event)


# ====================================================================== spells
# Angle Shot — "[Reaction] Choose a unit and an Equipment with the same controller. Attach that Equipment to that
# unit or detach that Equipment from that unit. Draw 1."  (attach if it isn't attached to that unit, else detach)
def _angle_choices(g, pid, ctx):
    good, bad = [], []
    for side in (pid, 1 - pid):
        eqs = sorted([x for x in g.gear(side) if "Equipment" in x.spec["tags"]],
                     key=lambda x: (-EQUIP_BONUS.get(x.cname, 0), x.uid))
        us = sorted([u for u in g.units(side) if g.targetable(u, pid)],
                    key=lambda u: (u.loc not in (0, 1), -value(g, u), u.uid))
        for x in eqs:
            for u in us[:3]:
                if x.attached_to == u.uid:
                    (good if side != pid else bad).append(dict(tg=(u.uid, x.uid)))
                else:
                    (good if side == pid else bad).append(dict(tg=(u.uid, x.uid)))
    return good + bad


def _angle(g, it):
    u, x = g.legal(it, 0), g.legal(it, 1)
    if u is not None and x is not None and u.ctrl == x.ctrl and "Equipment" in x.spec["tags"]:
        if x.attached_to == u.uid:
            _detach(g, x)
        else:
            attach(g, x, u)
    g.draw(it.ctrl, 1)


card("Angle Shot", timing="reaction", preds=[P_unit, P_gear], resolve=_angle, choices=_angle_choices)


# Blind Fury — "[Action] Each opponent reveals the top card of their Main Deck. Choose one and banish it, then play
# it, ignoring its cost. Then recycle the rest."  (1v1: one revealed card, so nothing is left to recycle; rule
# 419.3.c: if it can't be played, nothing more happens and it stays banished)
def _blind_fury(g, it):
    pid = it.ctrl
    opp = 1 - pid
    if not g.p[opp].deck:
        return                                       # revealing more than the deck: no burn out (rule 431.1.c)
    c = g.p[opp].deck[0]
    g.log(f"  P{opp} reveals {c}")
    g.to_zone(c, "banish")
    _play_from(g, pid, c, "banish", _play_choices(g, pid, c, "banish", free=True))


card("Blind Fury", timing="action", resolve=_blind_fury)


# Blood Rush — "[Action] [Repeat] 1 energy. Give a unit [Assault 2]." (no duration is printed: the grant lasts while
# the unit stays on the board)
def _blood_rush(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.grant(u, "Assault", 2, None)


card("Blood Rush", timing="action", repeat=repeat_cost("1 energy"), preds=[P_unit], resolve=repeatable(_blood_rush),
     choices=lambda g, pid, ctx: tg_choices(_buff_targets(g, pid, False, ctx["hidden_bf"])))


# Cleave — "[Action] Give a unit [Assault 3] this turn."
card("Cleave", timing="action", preds=[P_unit],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.grant(g.legal(it, 0), "Assault", 3, "turn"),
     choices=lambda g, pid, ctx: tg_choices(_buff_targets(g, pid, False, ctx["hidden_bf"])))


# Consuming Curse — "[Action] Deal 2 to a unit at a battlefield. This deals 1 Bonus Damage for each card with this
# name in your trash." (Bonus Damage, rules 712-715: counted as it resolves; this card is still on the chain)
def _curse_n(g, pid):
    return 2 + sum(1 for c in g.p[pid].trash if c.cname == "Consuming Curse")


def _curse(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.deal(u, _curse_n(g, it.ctrl), "spell", it.ctrl)


card("Consuming Curse", timing="action", preds=[P_unit_bf], resolve=_curse,
     choices=lambda g, pid, ctx: tg_choices(_dmg_targets(g, pid, _curse_n(g, pid), True, ctx["hidden_bf"])))


# Curtain Call — "[Repeat] — 1 energy / 1 rune of any type / 1 energy and 1 rune of any type. Choose one you haven't
# already chosen — Draw 1. / Deal 2 to a unit at a battlefield. / Deal 3 to a unit at a base. / Give a unit at a
# battlefield -4 might this turn."  Choice: cc = the modes in execution order, tg = their targets, rc = the Repeat
# costs paid (one per extra mode, each cost at most once, rule 820).
_CC_REP = [(1, []), (0, [ANY]), (1, [ANY])]
_CC_TARGETED = ("d2", "d3", "m4")


def _cc_extra(g, pid, card, ch):
    e, reqs = 0, []
    for i in ch.get("rc", ()):
        e += _CC_REP[i][0]
        reqs += _CC_REP[i][1]
    return e, reqs


def _cc_choices(g, pid, ctx):
    from itertools import combinations
    card = ctx["card"]
    es_bf = enemies(g, pid, True)
    es_base = [u for u in enemies(g, pid) if u.loc == "base"]
    best = {}
    if es_bf:
        best["d2"] = sorted(es_bf, key=lambda u: (not _killable(g, u, 2), -value(g, u), u.uid))[0]
        best["m4"] = sorted(es_bf, key=lambda u: (-value(g, u), u.uid))[0]
    if es_base:
        best["d3"] = sorted(es_base, key=lambda u: (not _killable(g, u, 3), -value(g, u), u.uid))[0]
    modes = [m for m in ("m4", "d2", "d3", "draw") if m == "draw" or m in best]   # -4 might first: helps damage
    out = []
    for k in range(len(modes), 0, -1):
        for combo in combinations(modes, k):
            tg = tuple(best[m].uid for m in combo if m in _CC_TARGETED)
            for rc in ([tuple(range(k - 1))] + ([(1,)] if k == 2 else [])):
                ch = dict(cc=combo, tg=tg, rc=rc)
                e, reqs = total_cost(g, pid, card, ch, "hand")
                if not g.can_pay(pid, e, reqs, pay_ctx(card)):
                    continue
                sc = 0
                for m in combo:
                    if m == "draw":
                        sc += 1.5
                    elif m == "m4":
                        sc += 1 + (2 if "d2" in combo and best["d2"] is best["m4"] else 0)
                    else:
                        u = best[m]
                        sc += value(g, u) if _killable(g, u, 2 if m == "d2" else 3) else 1
                out.append((-(sc - 0.7 * len(rc)), len(out), ch))
                break
    out.sort(key=lambda x: (x[0], x[1]))
    return [c for _, _, c in out]


def _curtain_call(g, it):
    i = 0
    for m in it.data.get("cc", ()):
        if m == "draw":
            g.draw(it.ctrl, 1)
            continue
        u = g.legal(it, i)
        i += 1
        if u is None:
            continue
        if m == "d2" and u.loc in (0, 1):
            g.deal(u, 2, "spell", it.ctrl)
        elif m == "d3" and u.loc == "base":
            g.deal(u, 3, "spell", it.ctrl)
        elif m == "m4" and u.loc in (0, 1):
            g.mod(u, -4)


card("Curtain Call", preds=[P_unit], resolve=_curtain_call, choices=_cc_choices, extra_cost_fn=_cc_extra)


# Dancing Grenade — "Deal 2 to a unit. Its controller may play this spell again for 1 rune of any type. If they do,
# this deals 1 additional Bonus Damage for each time this spell has dealt damage this turn."
# The new play happens right after this one resolves (the card must be in the trash to be played again).
def _grenade(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    key = ("dancing_grenade", it.card.uid, g.turn_no)
    n = 2 + (g.stats[key] if it.data.get("dg_again") else 0)
    if g.deal(u, n, "spell", it.ctrl) > 0:
        g.stats[key] += 1
    if u in g.board:
        _replay_offer(g, it, u.ctrl, "dancing_grenade", dict(dg_again=True))


card("Dancing Grenade", preds=[P_unit], resolve=_grenade,
     choices=lambda g, pid, ctx: tg_choices(_dmg_targets(g, pid, 2, False, ctx["hidden_bf"], enemies_only=True)))


# Danger Zone — "[Reaction] [Repeat] 1 energy and 1 rune of any type. Give your Mechs +1 might this turn."
def _danger_zone(g, it):
    for u in g.units(it.ctrl):
        if "Mech" in u.spec["tags"]:
            g.mod(u, 1)


card("Danger Zone", timing="reaction", repeat=repeat_cost("1 energy and 1 rune of any type"),
     resolve=repeatable(_danger_zone))


# Death Mark — "[Burn 3]. Play a 0 might Shadow Clone unit token. [Flow] 1 energy and 2 runes of any type"
def _death_mark(g, it):
    g.burn(it.ctrl, 3)
    if g.winner is None:
        _token(g, it.ctrl, "Shadow Clone")


card("Death Mark", resolve=_death_mark, flow=flow_cost("1 energy and 2 runes of any type"))


# Death from Below — "Kill a unit at a battlefield. Then, if it had 3 might or less, you may play this from your
# trash for 1 rune of any type."  Decision: the offer is made once, right after this play resolves and the card is
# in the trash (same mechanism as Dancing Grenade); it is not a lasting permission.
def _dfb(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    m = g.might(u)
    g.kill([u], it.ctrl)
    if m <= 3:
        _replay_offer(g, it, it.ctrl, "death_from_below")


card("Death from Below", preds=[P_unit_bf], resolve=_dfb,
     choices=lambda g, pid, ctx: tg_choices(sorted(enemies(g, pid, True, ctx["hidden_bf"]),
                                                   key=lambda u: (g.might(u) > 3, -value(g, u), u.uid))
                                            + friends(g, pid, True, ctx["hidden_bf"])))


# Detonate — "Kill a gear. Its controller draws 2."
def _detonate(g, it):
    x = g.legal(it, 0)
    if x is None:
        return
    c = x.ctrl
    g.kill([x], it.ctrl)
    g.draw(c, 2)


card("Detonate", preds=[P_gear], resolve=_detonate,
     choices=lambda g, pid, ctx: tg_choices(sorted(g.gear(), key=lambda x: (x.ctrl == pid, -x.spec["e"], x.uid))))


# Disintegrate — "[Action] Deal 3 to a unit at a battlefield. If this kills it, draw 1."  The unit dies in the
# cleanup that follows (rule 428.5.c attributes that kill to this spell): the draw happens then.
def _disintegrate(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    g.deal(u, 3, "spell", it.ctrl)
    if u in g.board and g.lethal(u):
        g.effects.append(dict(on="die", fn=_disintegrate_die, uid=u.uid, pid=it.ctrl, dur="turn"))


def _disintegrate_die(g, eff, info):
    if info["info"]["obj"].uid == eff["uid"]:
        g.effects.remove(eff)
        g.draw(eff["pid"], 1)


card("Disintegrate", timing="action", preds=[P_unit_bf], resolve=_disintegrate,
     choices=lambda g, pid, ctx: tg_choices(_dmg_targets(g, pid, 3, True, ctx["hidden_bf"])))


# Firestorm — "Deal 3 to all enemy units at a battlefield."
def _firestorm(g, it):
    for u in list(g.units(1 - it.ctrl, it.data["bf"])):
        g.deal(u, 3, "spell", it.ctrl)


card("Firestorm", resolve=_firestorm,
     choices=lambda g, pid, ctx: sorted([dict(bf=b.idx) for b in g.bfs if g.units(1 - pid, b.idx)],
                                        key=lambda c: (-sum(value(g, u) for u in g.units(1 - pid, c["bf"])), c["bf"])))


# Get Excited! — "[Action] Discard 1. Deal its Energy cost as damage to a unit at a battlefield."
def _excited(g, it):
    c = _discard_one(g, it.ctrl, "excited_discard", order=lambda c: (-c.spec["e"], c.uid))
    u = g.legal(it, 0)
    if c is not None and u is not None and c.spec["e"] > 0:
        g.deal(u, c.spec["e"], "spell", it.ctrl)


card("Get Excited!", timing="action", preds=[P_unit_bf], resolve=_excited,
     choices=lambda g, pid, ctx: tg_choices(_dmg_targets(
         g, pid, max([c.spec["e"] for c in g.p[pid].hand if c is not ctx["card"]] + [0]), True, ctx["hidden_bf"])))


# Hextech Ray / Incinerate / Void Seeker — "[Action] Deal N to a unit at a battlefield. (Draw 1.)"
def _deal_bf(n, draw=0):
    def res(g, it):
        u = g.legal(it, 0)
        if u is not None:
            g.deal(u, n, "spell", it.ctrl)
        if draw:
            g.draw(it.ctrl, draw)
    return res


card("Hextech Ray", timing="action", preds=[P_unit_bf], resolve=_deal_bf(3),
     choices=lambda g, pid, ctx: tg_choices(_dmg_targets(g, pid, 3, True, ctx["hidden_bf"])))
card("Incinerate", timing="action", preds=[P_unit_bf], resolve=_deal_bf(2),
     choices=lambda g, pid, ctx: tg_choices(_dmg_targets(g, pid, 2, True, ctx["hidden_bf"])))
card("Void Seeker", timing="action", preds=[P_unit_bf], resolve=_deal_bf(4, draw=1),
     choices=lambda g, pid, ctx: tg_choices(_dmg_targets(g, pid, 4, True, ctx["hidden_bf"])))


# Icathian Rain — "Deal 2 to a unit." six times (six targets; the same unit may be chosen several times)
def _rain_choices(g, pid, ctx):
    es = enemies(g, pid)
    if not es:
        us = _dmg_targets(g, pid, 2, False)
        return [dict(tg=(us[0].uid,) * 6)] if us else []

    def greedy(cands):
        tg = []
        for u in cands:
            need = max(1, -(-(g.might(u) - u.damage + u.prevent) // 2))
            if len(tg) + need > 6:
                continue
            tg += [u.uid] * need
        while len(tg) < 6:
            tg.append(tg[-1] if tg else cands[0].uid)
        return tuple(tg[:6])
    out = [greedy(es), greedy(sorted(es, key=lambda u: (g.might(u) - u.damage, u.uid))), (es[0].uid,) * 6]
    res = []
    for t in out:
        if dict(tg=t) not in res:
            res.append(dict(tg=t))
    return res


def _rain(g, it):
    for i in range(6):
        u = g.legal(it, i)
        if u is not None:
            g.deal(u, 2, "spell", it.ctrl)


card("Icathian Rain", preds=[P_unit], resolve=_rain, choices=_rain_choices)


# Monster Harpoon — "[Action] Deal 2 to a unit at a battlefield. If you control a facedown card, deal 4 to it
# instead."
def _facedown(g, pid):
    return any(b.facedown is not None and b.facedown.owner == pid for b in g.bfs)


def _harpoon(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.deal(u, 4 if _facedown(g, it.ctrl) else 2, "spell", it.ctrl)


card("Monster Harpoon", timing="action", preds=[P_unit_bf], resolve=_harpoon,
     choices=lambda g, pid, ctx: tg_choices(_dmg_targets(g, pid, 4 if _facedown(g, pid) else 2, True,
                                                         ctx["hidden_bf"])))


# Noxian Guillotine — "[Action] Choose a unit. Kill it the next time it takes damage this turn. [Legion] — Kill it
# now instead."  (rule 391 delayed triggered ability; rule 158.2: with Legion the delayed part is ignored)
def _guillotine(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    if g.legion(it.ctrl, it.card):
        g.kill([u], it.ctrl)
    else:
        g.effects.append(dict(on="damaged", fn=_guillotine_fire, uid=u.uid, oid=u.oid, pid=it.ctrl, dur="turn"))


def _guillotine_kill(g, it):
    x = g.obj(it.data["uid"])
    if x is not None and x.oid == it.data["oid"]:
        g.kill([x], it.ctrl)


def _guillotine_fire(g, eff, info):
    o = info["obj"]
    if o.uid == eff["uid"] and o.oid == eff["oid"]:
        g.effects.remove(eff)
        g.queue_trigger(eff["pid"], "Noxian Guillotine", _guillotine_kill, dict(uid=o.uid, oid=o.oid))


card("Noxian Guillotine", timing="action", preds=[P_unit], resolve=_guillotine,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, False, ctx["hidden_bf"])))


# Perfect Execution — "Ready a unit and give it [Assault 3] this turn. [Flow] 3 energy and 1 fury rune"
def _perfect(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.ready_obj(u)
        g.grant(u, "Assault", 3, "turn")


card("Perfect Execution", preds=[P_unit], resolve=_perfect, flow=flow_cost("3 energy and 1 fury rune"),
     choices=lambda g, pid, ctx: tg_choices(sorted(friends(g, pid), key=lambda u: (not u.exhausted, -value(g, u), u.uid))
                                            + enemies(g, pid)))


# Piercing Light — "[Repeat] 2 energy and 1 fury rune. Deal 2 to a unit at a battlefield, then deal 2 to up to one
# other unit."
def _piercing(g, it):
    a = g.legal(it, 0)
    if a is not None:
        g.deal(a, 2, "spell", it.ctrl)
    b = g.legal(it, 1)
    if b is not None and it.targets[1][0] != it.targets[0][0]:
        g.deal(b, 2, "spell", it.ctrl)


def _piercing_choices(g, pid, ctx):
    firsts = _dmg_targets(g, pid, 2, True, ctx["hidden_bf"])[:3]
    seconds = [u for u in _dmg_targets(g, pid, 2, False) if u.ctrl != pid][:3]
    out = []
    for a in firsts:
        for b in seconds:
            if b is not a:
                out.append(dict(tg=(a.uid, b.uid)))
        out.append(dict(tg=(a.uid,)))
    return out


card("Piercing Light", repeat=repeat_cost("2 energy and 1 fury rune"), preds=[P_unit_bf, P_unit],
     resolve=repeatable(_piercing), choices=_piercing_choices)


# Relentless Pursuit — "[Action] Move a friendly unit. You may attach an Equipment with the same controller to it.
# This turn, that unit has 'When I conquer, you may move me to my base.'"  (a spell's move is not a Standard Move:
# any destination, rule 144.4 restricts only the Standard Move)
def _pursuit(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    if u.loc != it.data["dest"]:
        g.move([u], it.data["dest"], it.ctrl)
    eqs = [x for x in g.gear(u.ctrl) if "Equipment" in x.spec["tags"] and x.attached_to != u.uid]
    free = sorted([x for x in eqs if x.attached_to is None], key=lambda x: (-EQUIP_BONUS.get(x.cname, 0), x.uid))
    x = g.ask(it.ctrl, "pursuit_attach", free + [None] + [x for x in eqs if x not in free])
    if x is not None and u in g.board:
        attach(g, x, u)
    g.effects.append(dict(on="conquer", fn=_pursuit_fire, uid=u.uid, oid=u.oid, dur="turn"))


def _pursuit_back(g, it):
    u = g.obj(it.data["uid"])
    if u is not None and u.oid == it.data["oid"] and u.loc != "base":
        g.move([u], "base", it.ctrl)


def _pursuit_fire(g, eff, info):
    for u in info["units"]:
        if u.uid == eff["uid"] and u.oid == eff["oid"]:
            g.queue_trigger(u.ctrl, "Relentless Pursuit", _pursuit_back, dict(uid=u.uid, oid=u.oid), src=u.uid,
                            may=True)


def _pursuit_choices(g, pid, ctx):
    out = []
    for u in friends(g, pid, False, ctx["hidden_bf"]):
        for d in ("base", 0, 1):
            if d == u.loc:
                continue
            if d == "base":
                sc = 2
            else:
                b = g.bfs[d]
                sc = 0 if b.ctrl != pid and not g.units(1 - pid, d) else 1
            out.append(((sc, -value(g, u), u.uid, str(d)), dict(tg=(u.uid,), dest=d)))
    out.sort(key=lambda x: x[0])
    return [x[1] for x in out]


card("Relentless Pursuit", timing="action", preds=[P_friend], resolve=_pursuit, choices=_pursuit_choices)


# Right of Conquest — "Draw 1, then draw 1 for each battlefield you or allies control." (1v1: no allies)
card("Right of Conquest",
     resolve=lambda g, it: (g.draw(it.ctrl, 1), g.draw(it.ctrl, sum(1 for b in g.bfs if b.ctrl == it.ctrl))))


# Ruthless Strike — "[Action] As an additional cost to play this, you may discard 1. Deal 3 to a unit at a
# battlefield. If you paid the additional cost, deal 5 to it instead."
def _ruthless_choices(g, pid, ctx):
    out = []
    can_disc = bool(_hand_others(g, pid, ctx["card"]))
    for u in _dmg_targets(g, pid, 5, True, ctx["hidden_bf"])[:4]:
        if can_disc and not _killable(g, u, 3):
            out.append(dict(tg=(u.uid,), disc=True))
        out.append(dict(tg=(u.uid,)))
        if can_disc and _killable(g, u, 3):
            out.append(dict(tg=(u.uid,), disc=True))
    return out


card("Ruthless Strike", timing="action", preds=[P_unit_bf], choices=_ruthless_choices,
     pay_extra=lambda g, pid, card, ch: ch.get("disc") and _discard_one(g, pid, exclude=card),
     resolve=lambda g, it: g.legal(it, 0) is not None and g.deal(g.legal(it, 0), 5 if it.data.get("disc") else 3,
                                                                  "spell", it.ctrl))


# Shakedown — "[Reaction] Choose an enemy unit. Deal 6 to it unless its controller has you draw 2."
def _shakedown(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    opts = [True, False] if _killable(g, u, 6) and value(g, u) >= 4 else [False, True]
    if g.ask(u.ctrl, "shakedown_let_draw", opts, unit=u):
        g.draw(it.ctrl, 2)
    else:
        g.deal(u, 6, "spell", it.ctrl)


card("Shakedown", timing="reaction", preds=[P_enemy], resolve=_shakedown,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, False, ctx["hidden_bf"])))


# Square Up — "[Repeat] — Discard 1. Give a unit [Assault 4] this turn."  The Repeat cost is not a resource cost:
# choice 'sq' (repeat paid, rule 820) with the repetition's target in 'tg2'.
def _square_choices(g, pid, ctx):
    us = _buff_targets(g, pid, False, ctx["hidden_bf"])
    out = tg_choices(us[:4])
    if _hand_others(g, pid, ctx["card"]):
        rep = [dict(tg=(u.uid,), sq=True, tg2=(v.uid,)) for u in us[:2] for v in us[:2]]
        out = rep[:2] + out[:3] + rep[2:]
    return out


def _square(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.grant(u, "Assault", 4, "turn")
    if it.data.get("sq"):
        for uid, oid in it.data.get("t2", []):
            v = g.obj(uid)
            if v is not None and v.oid == oid and P_unit(g, it, v) and g.targetable(v, it.ctrl):
                g.grant(v, "Assault", 4, "turn")


card("Square Up", preds=[P_unit], resolve=_square, choices=_square_choices,
     pay_extra=lambda g, pid, card, ch: ch.get("sq") and _discard_one(g, pid, exclude=card))


# Stormbringer — "Choose a friendly unit in your base. Deal damage equal to its Might to all enemy units at a
# battlefield, then move your unit there."
def _P_friend_base(g, it, o):
    return P_friend(g, it, o) and o.loc == "base"


def _storm(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    bf = it.data["bf"]
    m = max(0, g.might(u))
    for e in list(g.units(1 - it.ctrl, bf)):
        g.deal(e, m, "spell", it.ctrl)
    if u in g.board:
        g.move([u], bf, it.ctrl)


def _storm_choices(g, pid, ctx):
    out = []
    for u in friends(g, pid):
        if u.loc != "base":
            continue
        m = g.might(u)
        for b in g.bfs:
            sc = sum(value(g, e) for e in g.units(1 - pid, b.idx) if _killable(g, e, m))
            out.append(((-sc, -m, b.idx, u.uid), dict(tg=(u.uid,), bf=b.idx)))
    out.sort(key=lambda x: x[0])
    return [x[1] for x in out]


card("Stormbringer", preds=[_P_friend_base], resolve=_storm, choices=_storm_choices)


# Sudden Storm — "[Hidden] [Action] Deal 2 to a unit at a battlefield. If it's attacking, deal 4 to it instead."
def _sudden(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.deal(u, 4 if u.desig == "att" else 2, "spell", it.ctrl)


card("Sudden Storm", timing="action", hidden=True, preds=[P_unit_bf], resolve=_sudden,
     choices=lambda g, pid, ctx: tg_choices(sorted(_dmg_targets(g, pid, 2, True, ctx["hidden_bf"]),
                                                   key=lambda u: (u.ctrl == pid, u.desig != "att"))))


# Thermo Beam — "[Action] Kill all gear."
card("Thermo Beam", timing="action", resolve=lambda g, it: g.kill(list(g.gear()), it.ctrl))


# Thrill of the Hunt — "[Reaction] Banish a friendly unit, then its owner plays it to any battlefield, ignoring its
# cost."  (a token ceases to exist in the banishment, rule 186.1)
def _thrill(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    owner = u.owner
    tok = u.token
    g.to_zone(u, "banish")
    if tok or u not in g.p[owner].banish:
        return
    locs = sorted([0, 1], key=lambda i: (g.bfs[i].ctrl == owner, i))
    _play_from(g, owner, u, "banish", _play_choices(g, owner, u, "banish", free=True, locs=locs))


card("Thrill of the Hunt", timing="reaction", preds=[P_friend], resolve=_thrill,
     choices=lambda g, pid, ctx: tg_choices(sorted([u for u in friends(g, pid, False, ctx["hidden_bf"])],
                                                   key=lambda u: (u.token, u.loc in (0, 1), -value(g, u), u.uid))))


# Upstage Comedy — "[Repeat] 2 energy. Ready a unit."
card("Upstage Comedy", repeat=repeat_cost("2 energy"), preds=[P_unit],
     resolve=repeatable(lambda g, it: g.legal(it, 0) is not None and g.ready_obj(g.legal(it, 0))),
     choices=lambda g, pid, ctx: tg_choices(sorted(friends(g, pid), key=lambda u: (not u.exhausted, -value(g, u), u.uid))
                                            + enemies(g, pid)))


# Vault Breaker — "[Action] Give a unit [Assault 2] and [Ganking] this turn."
def _vault(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.grant(u, "Assault", 2, "turn")
        g.grant(u, "Ganking", 1, "turn")


card("Vault Breaker", timing="action", preds=[P_unit], resolve=_vault,
     choices=lambda g, pid, ctx: tg_choices(_buff_targets(g, pid, False, ctx["hidden_bf"])))


# Void Rush — "Reveal the top 2 cards of your Main Deck. You may play one of them, reducing its cost by 2 energy.
# Draw any you did not play this way."
def _void_rush(g, it):
    pid = it.ctrl
    pl = g.p[pid]
    top = pl.deck[:2]
    if not top:
        return
    g.log(f"  P{pid} reveals {top}")
    cands = {c.uid: _play_choices(g, pid, c, "deck", discount_e=2) for c in top}
    opts = sorted([c for c in top if cands[c.uid]], key=lambda c: (-(c.spec["e"] + 2 * c.spec["p"]), c.uid)) + [None]
    pick = g.ask(pid, "void_rush_pick", opts)
    if pick is not None:
        _play_from(g, pid, pick, "deck", cands[pick.uid])
    for c in top:
        if c is not pick and c in pl.deck:
            pl.deck.remove(c)
            c.zone = "hand"
            pl.hand.append(c)
            g.emit("draw", pid=pid, card=c)


card("Void Rush", resolve=_void_rush)


# ====================================================================== units
# Arena Kingpin — "I enter ready. [E]: Give a unit +3 might this turn."
card("Arena Kingpin", enter_ready=True, abilities=[ability(
    "Might", exhaust=True, preds=[P_unit],
    choices=lambda g, pid, o: tg_choices(_buff_targets(g, pid)),
    resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 3))])


# Baccai Reaper — "When I attack, you may pay 1 fury rune to give me [Assault 2] this turn."
def _reaper(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        g.queue_trigger(o.ctrl, "Baccai Reaper", lambda g_, it: _me(g_, it) is not None and
                        g_.grant(_me(g_, it), "Assault", 2, "turn"), src=o.uid, may=True,
                        cost=may_pay(0, lambda g_, it: [FURY]))


card("Baccai Reaper", on_event=_reaper)


# Baccai Sandspinner — "[Empower] 5 energy. This ability costs 3 energy less if you control 4 or fewer runes.
# [Empowered] I have [Deflect] and [Assault 2]."
card("Baccai Sandspinner", empower=lambda g, pid, o, ch: (2 if len(g.p[pid].runes) <= 4 else 5, []),
     kw_if=[(when_empowered, {"Deflect": 1, "Assault": 2})])


# Battering Ram — "I cost 1 energy less for each card you've played this turn, to a minimum of 1 energy."
# (rule 419.4.b: cards finalized this turn, countered ones included)
card("Battering Ram",
     cost_mod=lambda g, pid, c, ch: (max(0, min(len(g.finalized[pid]), c.spec["e"] - 1)), 0))


# Blade Twirler — "The first time I move each turn, choose a player. They [Burn 1]."
def _twirler_res(g, it):
    p = g.ask(it.ctrl, "burn_player", [1 - it.ctrl, it.ctrl])
    g.burn(p, 1)


def _twirler(g, o, ev, info):
    if ev == "move" and info["obj"] is o:
        key = ("blade_twirler", o.uid, o.oid, g.turn_no)
        if key in g.stats:
            return
        g.stats[key] = 1
        g.queue_trigger(o.ctrl, "Blade Twirler", _twirler_res, src=o.uid)


card("Blade Twirler", on_event=_twirler)


# Blast Corps Cadet — "You may pay 1 energy and 1 fury rune as an additional cost to play me. When you play me, if
# you paid the additional cost, deal 2 to a unit at a battlefield."
def _cadet_res(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.deal(u, 2, "ability", it.ctrl)


def _cadet_play(g, o, ctx):
    if ctx.get("bcc"):
        g.queue_trigger(o.ctrl, "Blast Corps Cadet", _cadet_res, dict(dmg=2), src=o.uid,
                        choose=trig_target(lambda g_, it: _dmg_targets(g_, it.ctrl, 2, True), P_unit_bf, deflect=True))


card("Blast Corps Cadet", on_play=_cadet_play,
     as_played=lambda g, pid, c: [dict(bcc=True), dict()],
     extra_cost_fn=lambda g, pid, c, ch: (1, [FURY]) if ch.get("bcc") else (0, []))


# Brazen Buccaneer — "As you play me, you may discard 1 as an additional cost. If you do, reduce my cost by 2
# energy."
card("Brazen Buccaneer",
     as_played=lambda g, pid, c: ([dict(bb=True)] if _hand_others(g, pid, c) else []) + [dict()],
     cost_mod=lambda g, pid, c, ch: (2, 0) if ch.get("bb") else (0, 0),
     pay_extra=lambda g, pid, c, ch: ch.get("bb") and _discard_one(g, pid, exclude=c))


# Captain Farron — "Other friendly units here have [Assault]."
card("Captain Farron", aura_kw=lambda g, src, o: {"Assault": 1} if (
    o is not src and src in g.board and o.spec["type"] == "Unit" and o.ctrl == src.ctrl and o.loc == src.loc) else None)


# Chemtech Enforcer — "[Assault 2] When you play me, discard 1."
card("Chemtech Enforcer", kw={"Assault": 2},
     on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Chemtech Enforcer", _discard_n(1), src=o.uid))


# Dangerous Duo — "[Legion] — When you play me, give a unit +2 might this turn."
def _duo(g, o, ctx):
    if g.legion(o.ctrl, o):
        g.queue_trigger(o.ctrl, "Dangerous Duo", lambda g_, it: g_.legal(it, 0) is not None and
                        g_.mod(g_.legal(it, 0), 2), src=o.uid,
                        choose=trig_target(lambda g_, it: _buff_targets(g_, it.ctrl), P_unit, kind="friendly_target",
                                           deflect=True))


card("Dangerous Duo", on_play=_duo)


# Draven, Showboat — "My Might is increased by your points."
card("Draven, Showboat", might_mod=lambda g, o: g.p[o.ctrl].points)


# Draven, Vanquisher — "When I win a combat, play a Gold gear token exhausted. When I attack or defend, you may pay
# 1 fury rune. If you do, give me +2 might this turn."
def _vanquisher(g, o, ev, info):
    if ev == "combat_won" and info["pid"] == o.ctrl and o.loc == info["bf"] and g.sd is not None \
            and o.uid in g.sd.members:
        g.queue_trigger(o.ctrl, "Draven, Vanquisher (Gold)", lambda g_, it: _gold(g_, it.ctrl), src=o.uid)
    if ev in ("attack", "defend") and info["obj"] is o:
        g.queue_trigger(o.ctrl, "Draven, Vanquisher", lambda g_, it: _me(g_, it) is not None and
                        g_.mod(_me(g_, it), 2), src=o.uid, may=True, cost=may_pay(0, lambda g_, it: [FURY]))


card("Draven, Vanquisher", on_event=_vanquisher)


# Dunebreaker — "If you have two or fewer cards in your hand, I enter ready. When I hold, draw 2."
def _dunebreaker(g, o, ev, info):
    if ev == "hold" and info["pid"] == o.ctrl and o in info["units"]:
        g.queue_trigger(o.ctrl, "Dunebreaker", lambda g_, it: g_.draw(it.ctrl, 2), src=o.uid)


card("Dunebreaker", enter_ready=lambda g, pid, c, ch: len(g.p[pid].hand) <= 2, on_event=_dunebreaker)

# Eager Drakehound — "I enter ready."
card("Eager Drakehound", enter_ready=True)


# Eclipse Dragon — "[Accelerate] When I move, if you control 4 or fewer runes, draw 1."
def _eclipse(g, o, ev, info):
    if ev == "move" and info["obj"] is o and len(g.p[o.ctrl].runes) <= 4:
        g.queue_trigger(o.ctrl, "Eclipse Dragon", lambda g_, it: len(g_.p[it.ctrl].runes) <= 4 and
                        g_.draw(it.ctrl, 1), src=o.uid)


card("Eclipse Dragon", accelerate=True, on_event=_eclipse)


# Forsaken Baccai — "If you control fewer runes than an opponent at the start of your Beginning Phase, give me +1
# might this turn."  /  Oasis Raider — "[same], give me +2 might and [Ganking] this turn."
def _fewer_runes_start(might, ganking):
    def res(g, it):
        me = _me(g, it)
        if me is not None:
            g.mod(me, might)
            if ganking:
                g.grant(me, "Ganking", 1, "turn")

    def ev_fn(g, o, ev, info):
        if ev == "beginning_start" and info["pid"] == o.ctrl and \
                len(g.p[o.ctrl].runes) < len(g.p[1 - o.ctrl].runes):
            g.queue_trigger(o.ctrl, o.cname, res, src=o.uid)
    return ev_fn


card("Forsaken Baccai", on_event=_fewer_runes_start(1, False))
card("Oasis Raider", on_event=_fewer_runes_start(2, True))


# Gem Jammer — "When you play me, give a unit [Ganking] this turn." (the Ganking keyword is only in that effect)
card("Gem Jammer", on_play=lambda g, o, ctx: g.queue_trigger(
    o.ctrl, "Gem Jammer", lambda g_, it: g_.legal(it, 0) is not None and g_.grant(g_.legal(it, 0), "Ganking", 1, "turn"),
    src=o.uid, choose=trig_target(lambda g_, it: sorted(friends(g_, it.ctrl), key=lambda u: (
        g_.has_kw(u, "Ganking"), u.loc not in (0, 1), -value(g_, u), u.uid)) + enemies(g_, it.ctrl), P_unit,
        kind="friendly_target", deflect=True)))


# Grim Apothecary — "[Ambush] When you play me, you may return a friendly unit at a battlefield to its owner's
# hand."
def _apothecary_choose(g, it):
    us = friends(g, it.ctrl, True)
    if not us:
        return False
    hurt = [u for u in us if u.damage > 0 or u.stunned]
    u = g.ask(it.ctrl, "apothecary_pick", hurt + [None] + [u for u in us if u not in hurt])
    if u is None:
        return False
    g.add_target(it, u, P_friend_bf)
    return True


def _apothecary_res(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.to_zone(u, "hand")


card("Grim Apothecary", ambush=True, on_play=lambda g, o, ctx: g.queue_trigger(
    o.ctrl, "Grim Apothecary", _apothecary_res, src=o.uid, choose=_apothecary_choose))


# Inviolus Vox — "When I conquer, give a friendly unit +8 might this turn."
def _vox(g, o, ev, info):
    if ev == "conquer" and info["pid"] == o.ctrl and o in info["units"]:
        g.queue_trigger(o.ctrl, "Inviolus Vox", lambda g_, it: g_.legal(it, 0) is not None and
                        g_.mod(g_.legal(it, 0), 8), src=o.uid,
                        choose=trig_target(lambda g_, it: friends(g_, it.ctrl), P_friend, kind="friendly_target"))


card("Inviolus Vox", on_event=_vox)


# Jhin, Murderous Artist — "[Deflect] [Ganking] When I move, [Add] 1 energy and 1 rune of any type."
# Abilities that add resources resolve immediately and can't be reacted to (rule 429.3.a).
def _jhin(g, o, ev, info):
    if ev == "move" and info["obj"] is o:
        g.add_pool(o.ctrl, 1, ["A"])


card("Jhin, Murderous Artist", kw={"Deflect": 1, "Ganking": 1}, on_event=_jhin)

# Jinx, Demolitionist — "[Accelerate] [Assault 2] When you play me, discard 2."
card("Jinx, Demolitionist", accelerate=True, kw={"Assault": 2},
     on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Jinx, Demolitionist", _discard_n(2), src=o.uid))

# Kadregrin the Infernal — "When you play me, draw 1 for each of your [Mighty] units."
card("Kadregrin the Infernal", on_play=lambda g, o, ctx: g.queue_trigger(
    o.ctrl, "Kadregrin the Infernal",
    lambda g_, it: g_.draw(it.ctrl, sum(1 for u in g_.units(it.ctrl) if g_.mighty(u))), src=o.uid))


# Katarina, Reckless — "When you hide a card, ready me. When you play a card from face down, deal 2 to an enemy
# unit."
def _kat_res(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.deal(u, 2, "ability", it.ctrl)


def _katarina(g, o, ev, info):
    if ev == "hide" and info["pid"] == o.ctrl:
        g.queue_trigger(o.ctrl, "Katarina (ready)", lambda g_, it: _me(g_, it) is not None and
                        g_.ready_obj(_me(g_, it)), src=o.uid)
    if ev == "played" and info["pid"] == o.ctrl:
        it0 = info.get("item")
        src = it0.data.get("from") if it0 is not None else getattr(info["card"], "played_from", None)
        if src == "facedown":
            g.queue_trigger(o.ctrl, "Katarina, Reckless", _kat_res, dict(dmg=2), src=o.uid,
                            choose=trig_target(lambda g_, it: enemies(g_, it.ctrl), P_enemy, deflect=True))


card("Katarina, Reckless", on_event=_katarina)


# Lord Broadmane — "[Ambush] When you play me, give your other units here [Assault] this turn."
def _broadmane(g, o, ctx):
    here = o.loc

    def res(g_, it):
        for u in g_.units(it.ctrl, here):
            if u.uid != it.src:
                g_.grant(u, "Assault", 1, "turn")
    g.queue_trigger(o.ctrl, "Lord Broadmane", res, src=o.uid)


card("Lord Broadmane", ambush=True, on_play=_broadmane)


# Lucian, Gunslinger — "[Assault] When I attack, deal damage equal to my [Assault] to an enemy unit here."
def _lucian_res(g, it):
    me, u = _me(g, it), g.legal(it, 0)
    if me is not None and u is not None:
        g.deal(u, g.kw_value(me, "Assault"), "ability", it.ctrl)


def _lucian(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc
        g.queue_trigger(o.ctrl, "Lucian, Gunslinger", _lucian_res, dict(dmg=g.kw_value(o, "Assault")), src=o.uid,
                        choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                                           lambda g_, it, u: P_enemy(g_, it, u) and u.loc == here, deflect=True))


card("Lucian, Gunslinger", kw={"Assault": 1}, on_event=_lucian)


# Morgana, Vindictive — "[Ambush] When you play me, deal damage to a unit equal to the damage marked on it."
def _morgana_res(g, it):
    u = g.legal(it, 0)
    if u is not None and u.damage > 0:
        g.deal(u, u.damage, "ability", it.ctrl)


card("Morgana, Vindictive", ambush=True, on_play=lambda g, o, ctx: g.queue_trigger(
    o.ctrl, "Morgana, Vindictive", _morgana_res, src=o.uid,
    choose=trig_target(lambda g_, it: sorted(all_units(g_, it.ctrl), key=lambda u: (
        u.ctrl == it.ctrl, u.damage == 0, not _killable(g_, u, u.damage), -value(g_, u), u.uid)),
        P_unit, kind="morgana_target", deflect=True)))


# Punching Poro — "[Empower] — Discard 1. [Empowered] I have +1 might."
card("Punching Poro", might_if=[(when_empowered, 1)], abilities=[ability(
    "Empower", can=lambda g, pid, o: not o.empowered and bool(g.p[pid].hand),
    extra_cost=lambda g, pid, o, ch: _discard_one(g, pid),
    resolve=lambda g, it: _me(g, it) is not None and g.empower(_me(g, it)))])


# Pyke, Dockside Butcher — "[Hidden] [Ganking] You may pay 1 fury rune as an additional cost to play me. When you
# play me, if you paid the additional cost, ready me and give me +2 might this turn."
def _pyke_res(g, it):
    me = _me(g, it)
    if me is not None:
        g.ready_obj(me)
        g.mod(me, 2)


card("Pyke, Dockside Butcher", hidden=True, kw={"Ganking": 1},
     on_play=lambda g, o, ctx: ctx.get("pyke") and g.queue_trigger(o.ctrl, "Pyke, Dockside Butcher", _pyke_res,
                                                                    src=o.uid),
     as_played=lambda g, pid, c: [dict(pyke=True), dict()],
     extra_cost_fn=lambda g, pid, c, ch: (0, [FURY]) if ch.get("pyke") else (0, []))


# Rell, Magnetic — "[Tank] When I attack, you may play an Equipment with Energy cost no more than 2 energy, ignoring
# its cost, and attach it to me."  (played from the hand, rule 419.1.a)
def _rell_res(g, it):
    pid = it.ctrl
    cs = [c for c in g.p[pid].hand if c.spec["type"] == "Gear" and "Equipment" in c.spec["tags"] and c.spec["e"] <= 2]
    cands = {c.uid: _play_choices(g, pid, c, "hand", free=True) for c in cs}
    cs = sorted([c for c in cs if cands[c.uid]], key=lambda c: (-EQUIP_BONUS.get(c.cname, 0), c.uid))
    pick = g.ask(pid, "rell_pick", cs + [None])
    if pick is None:
        return
    x = _play_from(g, pid, pick, "hand", cands[pick.uid])
    me = _me(g, it)
    if x is not None and x in g.board and me is not None:
        attach(g, x, me)


card("Rell, Magnetic", kw={"Tank": 1}, on_event=lambda g, o, ev, info: ev == "attack" and info["obj"] is o and
     g.queue_trigger(o.ctrl, "Rell, Magnetic", _rell_res, src=o.uid))


# Renekton, Rage Fueled — "[Accelerate] When I attack, if you control 4 or fewer runes, deal 2 to all enemy units
# here."
def _renekton_res(g, it):
    if len(g.p[it.ctrl].runes) > 4:
        return
    for u in list(g.units(1 - it.ctrl, it.data["here"])):
        g.deal(u, 2, "ability", it.ctrl)


def _renekton(g, o, ev, info):
    if ev == "attack" and info["obj"] is o and len(g.p[o.ctrl].runes) <= 4:
        g.queue_trigger(o.ctrl, "Renekton, Rage Fueled", _renekton_res, dict(here=o.loc), src=o.uid)


card("Renekton, Rage Fueled", accelerate=True, on_event=_renekton)


# Rumble, Hotheaded — "Your Mechs each have [Assault]. When I conquer, you may recycle another friendly unit to play
# a Mech from your trash. Reduce its Energy cost by the Might of the unit you recycled."
def _rumble_res(g, it):
    pid = it.ctrl
    others = [u for u in g.units(pid) if u.uid != it.src]
    mechs = [c for c in g.p[pid].trash if c.spec["type"] == "Unit" and "Mech" in c.spec["tags"]]
    opts = []
    for u in sorted(others, key=lambda u: (value(g, u), u.uid))[:4]:
        m = max(0, g.might(u))
        for c in sorted(mechs, key=lambda c: (-(c.spec["might"] or 0), c.uid))[:4]:
            if _play_choices(g, pid, c, "trash", discount_e=m):
                opts.append((u.uid, c.uid))
    pick = g.ask(pid, "rumble_pick", opts + [None])
    if pick is None:
        return
    u = g.obj(pick[0])
    c = next(x for x in g.p[pid].trash if x.uid == pick[1])
    m = max(0, g.might(u))
    g.recycle_cards(pid, [u])
    _play_from(g, pid, c, "trash", _play_choices(g, pid, c, "trash", discount_e=m))


card("Rumble, Hotheaded",
     on_event=lambda g, o, ev, info: ev == "conquer" and info["pid"] == o.ctrl and o in info["units"] and
     g.queue_trigger(o.ctrl, "Rumble, Hotheaded", _rumble_res, src=o.uid),
     aura_kw=lambda g, src, o: {"Assault": 1} if (src in g.board and o.spec["type"] == "Unit" and o.ctrl == src.ctrl
                                                  and "Mech" in o.spec["tags"]) else None)

# Scorchclaw — "[Hunt 2] [Level 3] I have +1 might and enter ready."
card("Scorchclaw", kw={"Hunt": 2}, levels=[(3, dict(might=1, ready=True))])


# Scrapyard Champion — "[Legion] — When you play me, discard 2, then draw 2."
def _scrapyard_res(g, it):
    _discard_n(2)(g, it)
    g.draw(it.ctrl, 2)


card("Scrapyard Champion", on_play=lambda g, o, ctx: g.legion(o.ctrl, o) and g.queue_trigger(
    o.ctrl, "Scrapyard Champion", _scrapyard_res, src=o.uid))

# Shadow Assassin — "I enter ready if you have a card with my name in your trash."
card("Shadow Assassin", enter_ready=lambda g, pid, c, ch: any(x.cname == "Shadow Assassin" for x in g.p[pid].trash))

# Shadow Fiend — "[Empower] 2 energy and 1 fury rune. [Empowered] I have [Assault 3]."
card("Shadow Fiend", empower="2 energy and 1 fury rune", kw_if=[(when_empowered, {"Assault": 3})])


# Tibbers — "When you play me, deal 3 to all units at battlefields."
def _tibbers(g, it):
    for u in [u for u in g.units() if u.loc in (0, 1)]:
        g.deal(u, 3, "ability", it.ctrl)


card("Tibbers", on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Tibbers", _tibbers, src=o.uid))


# Twilight Reveler — "When I attack, ready another friendly unit."
def _reveler(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        g.queue_trigger(o.ctrl, "Twilight Reveler", lambda g_, it: g_.legal(it, 0) is not None and
                        g_.ready_obj(g_.legal(it, 0)), src=o.uid,
                        choose=trig_target(lambda g_, it: sorted([u for u in friends(g_, it.ctrl) if u.uid != it.src],
                                                                 key=lambda u: (not u.exhausted, -value(g_, u), u.uid)),
                                           lambda g_, it, u: P_friend(g_, it, u) and u.uid != it.src,
                                           kind="friendly_target"))


card("Twilight Reveler", on_event=_reveler)


# Vayne, Hunter — "[Assault 3] If an opponent controls a battlefield, I enter ready. When I conquer, you may pay
# 1 energy to return me to my owner's hand."
def _vayne(g, o, ev, info):
    if ev == "conquer" and info["pid"] == o.ctrl and o in info["units"]:
        g.queue_trigger(o.ctrl, "Vayne, Hunter", lambda g_, it: _me(g_, it) is not None and
                        g_.to_zone(_me(g_, it), "hand"), src=o.uid, may=True, cost=may_pay(1))


card("Vayne, Hunter", kw={"Assault": 3}, on_event=_vayne,
     enter_ready=lambda g, pid, c, ch: any(b.ctrl == 1 - pid for b in g.bfs))


# Vi, Destructive — "[Ganking] Recycle 1 from your trash: Give me +1 might this turn." (rule 416.3)
def _recycle_one(g, pid, o, ch):
    tr = g.p[pid].trash
    if tr:
        c = g.ask(pid, "recycle_trash", sorted(tr, key=lambda c: (c.spec["e"] + 2 * c.spec["p"], c.uid)))
        g.recycle_cards(pid, [c])


card("Vi, Destructive", kw={"Ganking": 1}, abilities=[ability(
    "Might", can=lambda g, pid, o: bool(g.p[pid].trash), extra_cost=_recycle_one,
    resolve=lambda g, it: _me(g, it) is not None and g.mod(_me(g, it), 1))])

# Vi, Hotheaded — "[Deflect] 2 energy and 1 fury rune: Double my Might this turn." (rules 432, 477.3.c)
card("Vi, Hotheaded", kw={"Deflect": 1}, abilities=[ability(
    "Double", "2 energy and 1 fury rune",
    resolve=lambda g, it: _me(g, it) is not None and g.mod(_me(g, it), max(0, g.might(_me(g, it)))))])

# Void Drone — "I cost 2 energy less to play from anywhere other than your hand."
card("Void Drone", cost_mod=lambda g, pid, c, ch: (2, 0) if c.zone != "hand" else (0, 0))


# Volibear, Furious — "[Deflect 2] When I attack, deal 5 damage split among any number of enemy units here."
def _split_options(g, pid, us, total):
    """Allocations of total damage among units (tuples of (uid, n)), best first."""
    us = sorted(us, key=lambda u: (-value(g, u), u.uid))

    def need(u):
        return max(1, g.might(u) - u.damage + u.prevent)
    out = []
    for order in (us, sorted(us, key=lambda u: (need(u), -value(g, u), u.uid))):
        left, al = total, []
        for u in order:
            n = need(u)
            if n <= left:
                al.append((u.uid, n))
                left -= n
        if al and left:
            al[0] = (al[0][0], al[0][1] + left)
        elif not al and order:
            al = [(order[0].uid, total)]
        out.append(tuple(al))
    for u in us[:2]:
        out.append(((u.uid, total),))
    res = []
    for a in out:
        if a and a not in res:
            res.append(a)
    return res


def _voli_choose(g, it):
    here = it.data["here"]
    us = [u for u in enemies(g, it.ctrl) if u.loc == here]
    opts = [a for a in _split_options(g, it.ctrl, us, 5)
            if g.can_pay(it.ctrl, 0, deflect_reqs(g, it.ctrl, dict(tg=tuple(x for x, _ in a))))]
    if not opts:
        return False
    a = g.ask(it.ctrl, "split_damage", opts, item=it)
    reqs = deflect_reqs(g, it.ctrl, dict(tg=tuple(x for x, _ in a)))
    if reqs and not g.pay(it.ctrl, 0, reqs, dict(kind="ability")):
        return False
    for uid, _ in a:
        g.add_target(it, g.obj(uid), lambda g_, it_, u: P_enemy(g_, it_, u) and u.loc == it_.data["here"])
    it.data["split"] = [n for _, n in a]
    return True


def _voli_res(g, it):
    for i, n in enumerate(it.data.get("split", [])):
        u = g.legal(it, i)
        if u is not None:
            g.deal(u, n, "ability", it.ctrl)


card("Volibear, Furious", kw={"Deflect": 2},
     on_event=lambda g, o, ev, info: ev == "attack" and info["obj"] is o and g.queue_trigger(
         o.ctrl, "Volibear, Furious", _voli_res, dict(here=o.loc), src=o.uid, choose=_voli_choose))


# Xerath, Freed — "1 fury rune, [E]: Deal 3 to a unit. Use this ability only while I'm at a battlefield."
card("Xerath, Freed", abilities=[ability(
    "Deal 3", "1 fury rune", exhaust=True, preds=[P_unit], can=lambda g, pid, o: o.loc in (0, 1),
    choices=lambda g, pid, o: tg_choices(_dmg_targets(g, pid, 3, False)),
    resolve=lambda g, it: g.legal(it, 0) is not None and g.deal(g.legal(it, 0), 3, "ability", it.ctrl))])

# Zed, From the Shadows — "You may discard 1 as an additional cost to play me. When you play me, if you paid the
# additional cost, play a 0 might Shadow Clone unit token."
card("Zed, From the Shadows",
     on_play=lambda g, o, ctx: ctx.get("zed") and g.queue_trigger(
         o.ctrl, "Zed, From the Shadows", lambda g_, it: _token(g_, it.ctrl, "Shadow Clone"), src=o.uid),
     as_played=lambda g, pid, c: ([dict(zed=True)] if _hand_others(g, pid, c) else []) + [dict()],
     pay_extra=lambda g, pid, c, ch: ch.get("zed") and _discard_one(g, pid, exclude=c))


# ====================================================================== gear
# Assembly Rig — "1 energy and 1 fury rune, Recycle a unit from your trash, [E]: Play a 3 might Mech unit token to
# your base."
def _rig_choices(g, pid, o):
    seen, out = set(), []
    for c in sorted(g.p[pid].trash, key=lambda c: (c.spec["e"] + 2 * c.spec["p"], c.uid)):
        if c.spec["type"] == "Unit" and c.cname not in seen:
            seen.add(c.cname)
            out.append(dict(rec=c.uid))
    return out[:3]


def _rig_pay(g, pid, o, ch):
    c = next((x for x in g.p[pid].trash if x.uid == ch.get("rec")), None)
    if c is not None:
        g.recycle_cards(pid, [c])


card("Assembly Rig", abilities=[ability(
    "Mech", "1 energy and 1 fury rune", exhaust=True, choices=_rig_choices, extra_cost=_rig_pay,
    resolve=lambda g, it: make_token(g, "Mech", it.ctrl, "base"))])


# Fresh Beans — "When you play a unit during a showdown, you may exhaust this to draw 1."
def _beans_cost(g, it):
    me = _me(g, it)
    if me is None or me.exhausted:
        return False
    me.exhausted = True
    return True


def _beans(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and info["card"].spec["type"] == "Unit" and g.sd is not None \
            and not o.exhausted:
        g.queue_trigger(o.ctrl, "Fresh Beans", lambda g_, it: g_.draw(it.ctrl, 1), src=o.uid, may=True,
                        cost=_beans_cost)


card("Fresh Beans", on_event=_beans)

# Iron Ballista — "This enters exhausted. [E]: Deal 2 to a unit at a battlefield."
card("Iron Ballista", enter_exhausted=True, abilities=[ability(
    "Deal 2", exhaust=True, preds=[P_unit_bf],
    choices=lambda g, pid, o: tg_choices(_dmg_targets(g, pid, 2, True)),
    resolve=lambda g, it: g.legal(it, 0) is not None and g.deal(g.legal(it, 0), 2, "ability", it.ctrl))])

# Rage Amplifier — "[Empower] 6 energy and 1 fury rune. Your units have +1 might. If I'm [Empowered], they have +2
# might instead."
card("Rage Amplifier", empower="6 energy and 1 fury rune",
     aura_might=lambda g, src, o: (2 if src.empowered else 1) if (src in g.board and o.spec["type"] == "Unit"
                                                                  and o.ctrl == src.ctrl) else 0)

# Seal of Rage — "[E]: [Reaction] — [Add] 1 fury rune."
card("Seal of Rage", add=[add_ability("1 fury rune")])


# Sun Disc — "[E]: [Legion] — The next unit you play this turn enters ready."
def _sun_disc(g, it):
    if g.legion(it.ctrl):
        g.effects.append(dict(on="played", fn=_sun_disc_fire, pid=it.ctrl, dur="turn"))


def _sun_disc_fire(g, eff, info):
    if info["pid"] == eff["pid"] and info["card"].spec["type"] == "Unit" and info["card"] in g.board:
        g.effects.remove(eff)
        info["card"].exhausted = False                 # enters ready: no 'ready' event


card("Sun Disc", abilities=[ability("Legion", exhaust=True, can=lambda g, pid, o: g.legion(pid), resolve=_sun_disc)])


# Equipment (Might Bonus read on the card images, rule 718.4)
# Recurve Bow — "[Equip] 1 fury rune", +0; Effect Text: "When I attack or defend, deal 2 to an enemy unit here."
def _bow_res(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.deal(u, 2, "ability", it.ctrl)


def _bow(g, gear, unit, ev, info):
    if ev in ("attack", "defend") and info["obj"] is unit:
        here = unit.loc
        g.queue_trigger(unit.ctrl, "Recurve Bow", _bow_res, dict(dmg=2), src=gear.uid,
                        choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                                           lambda g_, it, u: P_enemy(g_, it, u) and u.loc == here, deflect=True))


card("Recurve Bow", equip="1 fury rune", bonus=0, effect_event=_bow)

# Serrated Dirk — "[Equip] 1 fury rune", +0; Effect Text: "[Assault 2]"
card("Serrated Dirk", equip="1 fury rune", bonus=0, equip_kw={"Assault": 2})

# Spinning Axe — "[Quick-Draw] [Equip] 1 rune of any type [Temporary] (If this is unattached, kill it at the start of
# its controller's Beginning Phase, before scoring.)", +3
card("Spinning Axe", timing="reaction", quickdraw=True, equip="1 rune of any type", bonus=3,
     kw_if=[(lambda g, o: o.attached_to is None, {"Temporary": 1})])
