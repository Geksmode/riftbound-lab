"""Legal discretionary actions (rule 410) and the process of playing cards and abilities (rules 353-359, 398-406).

Actions are tuples that only reference objects by uid, so the same action can be applied to a cloned game:
  ('end',) ('pass',)
  ('play', uid, src, choice)      src in hand|champ|facedown|trash(flow)
  ('hide', uid, bf)
  ('act', src_uid, ability_index, choice)   src_uid = 'legend' for the player's legend
  ('move', (uids...), dest)
"""
from game import Item, ANY, SPEC, EQUIP_BONUS
from itertools import count

_ids = count(1)
MAX_CHOICES = 8


def card_obj(g, pid, uid, src):
    pl = g.p[pid]
    zone = {"hand": pl.hand, "champ": pl.champ, "trash": pl.trash}.get(src)
    if src == "facedown":
        for b in g.bfs:
            if b.facedown is not None and b.facedown.uid == uid:
                return b.facedown
        return None
    for c in zone or []:
        if c.uid == uid:
            return c
    return None


# ---------------------------------------------------------------- timing
def has_timing(g, pid, im, src, closed, loc_ok_ambush=False):
    """Can this card be played now? closed=True: Closed state (needs Reaction).
    closed=False and showdown: needs Action or Reaction. Neutral Open on own turn: anything."""
    if src == "facedown":
        return True                                  # facedown Hidden cards have Reaction (rule 811.6)
    t = im.timing
    if im.quickdraw:
        t = "reaction"
    if closed:
        return t == "reaction" or loc_ok_ambush
    return t in ("action", "reaction") or loc_ok_ambush


def state_neutral_open_main(g, pid):
    return not g.chain and g.sd is None and g.stage == "main" and g.tp == pid


# ---------------------------------------------------------------- cost
def total_cost(g, pid, card, choice, src):
    """Rule 356: base cost (modified), additional costs, increases, discounts. Returns (energy, reqs)."""
    im = g.impl(card)
    sp = card.spec
    e, p = sp["e"], sp["p"]
    doms = sp["domains"]
    if src == "facedown" or choice.get("free"):
        e, p = 0, 0                                    # Hidden: ignore its base cost
    elif choice.get("flow"):
        e, p = im.flow[0], im.flow[1]
        doms = im.flow[2] if len(im.flow) > 2 else doms
    if choice.get("ignore_energy"):
        e = 0
    if im.cost_mod and src != "facedown" and not choice.get("free"):
        de, dp = im.cost_mod(g, pid, card, choice)
        e, p = max(0, e - de), max(0, p - dp)
    reqs = g.power_reqs(doms, p)
    # optional additional costs
    if choice.get("acc"):
        e += 1
        reqs.append(frozenset(sp["domains"]) or ANY)
    if choice.get("rep"):
        e += im.repeat[0]
        reqs += [frozenset(im.repeat[1]) or ANY] * im.repeat[2]
    if im.extra_cost_fn is not None:
        de, dreqs = im.extra_cost_fn(g, pid, card, choice)
        e += de
        reqs += list(dreqs)
    # Deflect: enemy objects with Deflect chosen by this spell (rule 809)
    if not im.ignore_deflect:
        reqs += deflect_reqs(g, pid, choice)
    # discounts: Astral Heron "your next card costs [2] and [A][A] less"
    disc = [ef for ef in g.effects if ef.get("kind") == "heron" and ef["pid"] == pid]
    for ef in disc:
        e = max(0, e - 2)
        for _ in range(2):
            if reqs:
                # drop the most restrictive requirement first
                reqs.sort(key=len)
                reqs.pop(0)
    return e, reqs


def deflect_reqs(g, pid, choice):
    """Deflect (rule 809): [A] per Deflect value for each enemy object chosen in choice['tg']."""
    out = []
    for uid in choice.get("tg", ()):
        o = g.obj(uid) if isinstance(uid, int) else None
        if o is not None and o.ctrl != pid:
            out += [ANY] * g.kw_value(o, "Deflect")
    return out


def pay_ctx(card):
    return dict(kind=card.spec["type"].lower(), card=card)


def affordable(g, pid, card, choice, src):
    e, reqs = total_cost(g, pid, card, choice, src)
    return g.can_pay(pid, e, reqs, pay_ctx(card))


def unit_timing_ok(im, closed):
    """A unit or gear with [Reaction] (or [Action] outside Closed states) can be played at that timing,
    still only to its normal locations (rules 806.3, 813.3.a)."""
    return im.timing == "reaction" or (im.timing == "action" and not closed)


# ---------------------------------------------------------------- enumeration
def unit_locations(g, pid, card, src, closed_or_focus):
    """Rule 355.2: base or a battlefield you control; Ambush adds battlefields where you have units."""
    im = g.impl(card)
    if src == "facedown":
        return [card.hidden_bf]
    locs = []
    if not closed_or_focus:
        locs.append("base")
        locs += [b.idx for b in g.bfs if b.ctrl == pid]
    if im.ambush:
        for b in g.bfs:
            if b.idx not in locs and any(u.ctrl == pid for u in g.units(loc=b.idx)):
                locs.append(b.idx)
    return locs


def card_choices(g, pid, card, src, timing_closed, timing_showdown):
    """All the play choices for a card (rule 355), already filtered for affordability."""
    im = g.impl(card)
    if im is None:
        return []
    typ = card.spec["type"]
    restricted = timing_closed or timing_showdown
    out = []
    hidden_bf = card.hidden_bf if src == "facedown" else None
    if typ == "Unit":
        timed = unit_timing_ok(im, timing_closed)
        if restricted and src != "facedown" and not im.ambush and not timed:
            return []
        locs = unit_locations(g, pid, card, src, restricted and src != "facedown" and not timed)
        extra = im.as_played(g, pid, card) if im.as_played else [{}]
        for loc in locs:
            for ex in extra:
                for acc in ([False, True] if im.accelerate and src != "facedown" else [False]):
                    ch = dict(ex, loc=loc, acc=acc)
                    if src == "trash":
                        continue
                    out.append(ch)
    elif typ == "Gear":
        if restricted and not (im.quickdraw or src == "facedown" or unit_timing_ok(im, timing_closed)):
            return []
        out.append(dict(loc=hidden_bf if hidden_bf is not None else "base"))
    elif typ == "Spell":
        if restricted and src != "facedown" and im.timing not in ("action", "reaction"):
            return []
        if timing_closed and src != "facedown" and im.timing != "reaction":
            return []
        base = im.choices(g, pid, dict(hidden_bf=hidden_bf, card=card)) if im.choices else [dict()]
        base = base[:MAX_CHOICES]
        for ch in base:
            if src == "trash":
                ch = dict(ch, flow=True)
            out.append(ch)
            if im.repeat and src != "trash":
                for ch2 in ((im.choices(g, pid, dict(hidden_bf=hidden_bf, card=card)) if im.choices else None)
                            or [{}])[:3]:
                    out.append(dict(ch, rep=True, tg2=ch2.get("tg", ())))
    res = []
    for ch in out:
        if affordable(g, pid, card, ch, src):
            res.append(ch)
    return res


def playable_cards(g, pid, closed, showdown):
    pl = g.p[pid]
    cands = [(c, "hand") for c in pl.hand] + [(c, "champ") for c in pl.champ]
    for b in g.bfs:
        c = b.facedown
        if c is not None and c.owner == pid and c.hidden_turn is not None and c.hidden_turn < g.turn_no:
            cands.append((c, "facedown"))
    for c in pl.trash:
        im = g.impl(c)
        if im is not None and im.flow:
            cands.append((c, "trash"))
    return cands


def play_options(g, pid, closed, showdown):
    opts = []
    seen = set()
    for c, src in playable_cards(g, pid, closed, showdown):
        key = (c.cname, src)
        if key in seen and src != "facedown":
            continue                                    # identical copies give identical options
        seen.add(key)
        for ch in card_choices(g, pid, c, src, closed, showdown):
            opts.append(("play", c.uid, src, ch))
    return opts


def ability_options(g, pid, closed, showdown, main):
    opts = []
    srcs = [o for o in g.board if o.ctrl == pid] + ["legend"]
    for o in srcs:
        im = g.impl(g.p[pid].legend_name) if o == "legend" else g.impl(o)
        if im is None:
            continue
        obj = g.p[pid].legend if o == "legend" else o
        for i, ab in enumerate(im.abilities):
            t = ab.get("timing", "main")
            if closed and t != "reaction":
                continue
            if showdown and not closed and t not in ("action", "reaction"):
                continue
            if not closed and not showdown and not main:
                continue
            if g.tp != pid and t == "main":
                continue
            if ab.get("can") and not ab["can"](g, pid, obj):
                continue
            if ab.get("exhaust") and obj.exhausted:
                continue
            chs = ab["choices"](g, pid, obj) if ab.get("choices") else [dict()]
            for ch in chs[:MAX_CHOICES]:
                e, reqs = ability_cost(g, pid, obj, ab, ch)
                if g.can_pay(pid, e, reqs, dict(kind="ability", obj=obj)):
                    opts.append(("act", ("legend", pid) if o == "legend" else o.uid, i, ch))
    return opts


def ability_cost(g, pid, obj, ab, ch):
    """Cost of an activated ability: its own cost plus Deflect for enemy objects it chooses (rule 809)."""
    e, reqs = ab["cost"](g, pid, obj, ch)
    if not ab.get("ignore_deflect"):
        reqs = list(reqs) + deflect_reqs(g, pid, ch)
    return e, reqs


def move_options(g, pid):
    """Standard Move (rule 144/420.3): exhaust ready units to move them to one destination.
    base -> battlefield, battlefield -> base, battlefield -> battlefield only with Ganking."""
    ready = [u for u in g.units(pid) if not u.exhausted]
    opts = []
    dests = ["base", 0, 1]
    for d in dests:
        movers = [u for u in ready if u.loc != d and (d == "base" or u.loc == "base" or g.has_kw(u, "Ganking"))]
        if d == "base":
            movers = [u for u in movers if u.loc in (0, 1)]
        if not movers:
            continue
        groups = set()
        for u in movers:
            groups.add((u.uid,))
        by_loc = {}
        for u in movers:
            by_loc.setdefault(u.loc, []).append(u.uid)
        for l, us in by_loc.items():
            if len(us) > 1:
                groups.add(tuple(sorted(us)))
                if len(us) > 2:
                    # all but the weakest
                    us2 = sorted(us, key=lambda x: g.might(g.obj(x)))
                    groups.add(tuple(sorted(us2[1:])))
        if len(by_loc) > 1:
            groups.add(tuple(sorted(u.uid for u in movers)))
        for grp in sorted(groups):   # ordre fixe : un set de tuples n'itère pas pareil en 32 et 64 bits (Pyodide)
            opts.append(("move", grp, d))
    return opts


def hide_options(g, pid):
    pl = g.p[pid]
    opts = []
    if not g.can_pay(pid, 0, [ANY]):
        return opts
    seen = set()
    for c in pl.hand + pl.champ:
        im = g.impl(c)
        if im is None or not im.hidden or c.cname in seen:
            continue
        seen.add(c.cname)
        for b in g.bfs:
            if b.ctrl == pid and b.facedown is None:
                opts.append(("hide", c.uid, b.idx))
    return opts


def main_options(g, pid):
    opts = [("end",)]
    opts += play_options(g, pid, closed=False, showdown=False)
    opts += ability_options(g, pid, closed=False, showdown=False, main=True)
    opts += hide_options(g, pid)
    opts += move_options(g, pid)
    return opts


def timed_options(g, pid, closed):
    showdown = g.sd is not None
    opts = play_options(g, pid, closed=closed, showdown=showdown or closed)
    opts += ability_options(g, pid, closed=closed, showdown=showdown, main=False)
    return opts


# ---------------------------------------------------------------- execution
def do_action(g, a):
    kind = a[0]
    pid = g.chain[-1].ctrl if False else None
    if kind == "play":
        _, uid, src, ch = a
        pid = owner_of(g, uid, src)
        card = card_obj(g, pid, uid, src)
        play_card(g, pid, card, src, dict(ch))
    elif kind == "hide":
        _, uid, bf = a
        pid = owner_of(g, uid, "hand")
        hide(g, pid, uid, bf)
    elif kind == "act":
        _, src, i, ch = a
        activate(g, src, i, dict(ch))
    elif kind == "move":
        _, uids, dest = a
        units = [g.obj(u) for u in uids]
        pid = units[0].ctrl
        for u in units:
            u.exhausted = True                       # cost of the Standard Move
        g.log(f"P{pid} moves {units} -> {dest}")
        g.move(units, dest, pid, standard=True)
    else:
        raise ValueError(a)


def owner_of(g, uid, src):
    for pl in g.p:
        for z in (pl.hand, pl.champ, pl.trash):
            for c in z:
                if c.uid == uid:
                    return pl.pid
    for b in g.bfs:
        if b.facedown is not None and b.facedown.uid == uid:
            return b.facedown.owner
    raise ValueError(f"card {uid} not found")


def hide(g, pid, uid, bf):
    """Hide (rule 421, 811): pay [A], place facedown at a battlefield you control."""
    c = card_obj(g, pid, uid, "hand") or card_obj(g, pid, uid, "champ")
    assert g.pay(pid, 0, [ANY])
    g.remove_from_zone(c)
    c.reset()
    c.zone = "facedown"
    c.hidden_turn = g.turn_no
    c.hidden_bf = bf
    g.bfs[bf].facedown = c
    g.log(f"P{pid} hides a card at {g.bfs[bf].name}")
    g.stats[f"hide_P{pid}"] += 1
    g.need_cleanup = True
    g.emit("hide", pid=pid, card=c, bf=bf)


def play_card(g, pid, card, src, ch, limited=False):
    """The process of play (rule 353). limited=True: played by an effect (Baited Hook, Mixologist...)."""
    im = g.impl(card)
    sp = card.spec
    if not limited:
        e, reqs = total_cost(g, pid, card, ch, src)
        if not g.pay(pid, e, reqs, pay_ctx(card)):
            raise RuntimeError(f"cannot pay for {card} {ch}")
        # consume Heron discount
        g.effects = [ef for ef in g.effects if not (ef.get("kind") == "heron" and ef["pid"] == pid)]
    elif ch.get("pay_power"):
        _, reqs = total_cost(g, pid, card, dict(ch, ignore_energy=True, free=False), "hand")
        if not g.pay(pid, 0, reqs):
            return None
    # additional non-resource costs (paid as part of costs, rule 357.2)
    if ch.get("kill") is not None:
        k = g.obj(ch["kill"])
        if k is not None:
            ch["killed_info"] = dict(e=k.spec["e"], p=k.spec["p"], might=g.might(k), name=k.cname)
            g.kill([k], pid, cost=True)
    if ch.get("xp"):
        g.spend_xp(pid, ch["xp"] if not isinstance(ch["xp"], bool) else 3)
    if im.pay_extra is not None:
        im.pay_extra(g, pid, card, ch)                 # non-resource additional costs of the card (discard...)
    g.remove_from_zone(card)
    from_facedown = src == "facedown"
    hidden_bf = card.hidden_bf if from_facedown else None
    card.zone = "chain"
    g.finalized[pid].append(card.cname)
    g.log(f"P{pid} plays {card.cname} from {src} {fmt_choice(g, ch)}")
    g.stats[f"play_{card.cname}"] += 1
    if not g.chain and g.chain_origin is None:
        g.chain_origin = "play"
    if sp["type"] in ("Unit", "Gear"):
        loc = ch.get("loc", "base")
        if sp["type"] == "Gear" and not from_facedown:
            loc = "base"
        ready = (sp["type"] == "Gear" and not im.enter_exhausted) or bool(ch.get("acc")) or ch.get("ready", False)
        if not ready and enters_ready(g, pid, card, ch):
            ready = True
        g.enter_board(card, pid, loc, ready=ready)
        card.played_from = src
        card.play_choice = ch
        g.played_event(pid, card)
        keyword_play_triggers(g, pid, card, hidden_bf)
        if im.on_play:
            im.on_play(g, card, dict(ch, hidden_bf=hidden_bf, src=src))
        if im.quickdraw:
            # Quick-Draw: "When you play this, attach it to a unit you control."
            def qd_choose(g_, it):
                us = [u for u in g_.units(it.ctrl)]
                if hidden_bf is not None:
                    us = [u for u in us if u.loc == hidden_bf] or us
                if not us:
                    return False
                u = g_.ask(it.ctrl, "equip_target", us, gear=card)
                g_.add_target(it, u)
                return True

            def qd_resolve(g_, it):
                u = g_.legal(it, 0)
                gobj = g_.obj(it.src)
                if u is not None and gobj is not None:
                    attach(g_, gobj, u)
            g.queue_trigger(pid, f"Quick-Draw {card.cname}", qd_resolve, src=card.uid, choose=qd_choose)
        g.need_cleanup = True
        return card
    # spell
    g.spells_played[pid] += 1
    it = Item("spell", pid, card.cname, card=card, src=card.uid, data=dict(ch))
    it.data["from"] = src
    it.data["hidden_bf"] = hidden_bf
    if ch.get("flow") or im.banish_after:
        it.data["after"] = "banish"
    if im.uncounterable:
        it.uncounterable = True
    preds = im.preds or []
    tg = list(ch.get("tg", ()))
    for i, uid in enumerate(tg):
        if isinstance(uid, int):
            o = g.obj(uid)
            if o is not None:
                pred = preds[min(i, len(preds) - 1)] if preds else None
                g.add_target(it, o, pred)
            else:
                it.targets.append((uid, -1, None))
        else:
            it.targets.append((None, -1, None))
    for uid in ch.get("tg2", ()):
        if isinstance(uid, int) and g.obj(uid) is not None:
            o = g.obj(uid)
            it.data.setdefault("t2", []).append((o.uid, o.oid))
            it.chosen.add(o.uid)
    if ch.get("item") is not None:
        it.data["target_item"] = ch["item"]
    g.chain.append(it)
    g.priority = pid
    g.passes = 0
    g.on_finalize(it)
    g.need_cleanup = True
    return it


def enters_ready(g, pid, card, ch):
    """'I enter ready' (Impl.enter_ready: True or fn(g, pid, card, choice)) and [Level N] 'I enter ready'
    (Impl.levels ... ready=True). Checked on the card as it is played."""
    im = g.impl(card)
    er = im.enter_ready
    if er is True or (callable(er) and er(g, pid, card, ch)):
        return True
    if im.levels:
        xp = g.p[pid].xp
        if any(xp >= n and d.get("ready") for n, d in im.levels):
            return True
    return False


def keyword_play_triggers(g, pid, card, hidden_bf=None):
    """Play triggers of printed or granted keywords of a permanent that was just played (rule 803: keywords
    first): [Vision] (rule 817, one trigger per instance) and [Weaponmaster] (rule 821)."""
    for _ in range(g.kw_value(card, "Vision")):
        g.queue_trigger(pid, f"Vision {card.cname}", lambda g_, it: g_.predict(it.ctrl, 1), src=card.uid)
    if card.spec["type"] == "Unit":
        for _ in range(g.kw_value(card, "Weaponmaster")):
            weaponmaster_trigger(g, pid, card)


def equip_cost(g, pid, gear, unit, discount_any=0):
    """Cost of the Equip ability of gear for unit, as (energy, reqs, ability) or None if it has none.
    discount_any: number of [A] removed (Weaponmaster, rule 821.1.c.3: only [A] is reduced)."""
    im = g.impl(gear)
    if im is None:
        return None
    for ab in im.abilities:
        if ab.get("name") == "Equip":
            e, reqs = ab["cost"](g, pid, gear, dict(tg=(unit.uid,)))
            reqs = list(reqs)
            for _ in range(discount_any):
                if ANY in reqs:
                    reqs.remove(ANY)
            return e, reqs, ab
    return None


def weaponmaster_trigger(g, pid, unit):
    """[Weaponmaster] "When you play me, you may choose an Equipment you control and pay its Equip cost, reduced
    by [A], to attach it to me" (rule 821). The choice is made at finalization, the cost paid on resolution."""
    def options(g_, it):
        u = g_.obj(it.src)
        if u is None:
            return []
        out = []
        for x in g_.gear(it.ctrl):
            if "Equipment" not in x.spec["tags"] or x.attached_to == u.uid:
                continue
            c = equip_cost(g_, it.ctrl, x, u, 1)
            if c is not None and g_.can_pay(it.ctrl, c[0], c[1], dict(kind="ability", obj=x)) and \
                    (c[2].get("can_extra") is None or c[2]["can_extra"](g_, it.ctrl, x)):
                out.append(x)
        return sorted(out, key=lambda x: -(EQUIP_BONUS.get(x.cname, 0) * 10 + x.spec["e"]))

    def choose(g_, it):
        opts = options(g_, it)
        if not opts:
            return False
        x = g_.ask(it.ctrl, "weaponmaster_pick", opts, unit=g_.obj(it.src))
        if x is None:
            return False
        g_.add_target(it, x, lambda g2, it2, o: o.ctrl == it2.ctrl)
        return True

    def resolve(g_, it):
        u = g_.obj(it.src)
        x = g_.legal(it, 0)
        if u is None or x is None or u.oid != it.data["oid"]:
            return
        c = equip_cost(g_, it.ctrl, x, u, 1)
        if c is None or not g_.pay(it.ctrl, c[0], c[1], dict(kind="ability", obj=x)):
            return
        if c[2].get("extra_cost"):
            c[2]["extra_cost"](g_, it.ctrl, x, dict(tg=(u.uid,)))
        attach(g_, x, u)

    g.queue_trigger(pid, f"Weaponmaster {unit.cname}", resolve, dict(oid=unit.oid), src=unit.uid, may=True,
                    choose=choose)


def attach(g, gear, unit):
    """Attach (rule 434): detach from previous unit, location follows the unit (not a move)."""
    if gear.attached_to is not None:
        old = g.obj(gear.attached_to)
        if old is not None and gear.uid in old.attached:
            old.attached.remove(gear.uid)
    gear.attached_to = unit.uid
    unit.attached.append(gear.uid)
    gear.loc = unit.loc
    g.log(f"  {gear} attached to {unit}")
    g.emit("attached", gear=gear, unit=unit)


def activate(g, src, i, ch):
    """Activated abilities (rule 376). Add abilities and Equip resolve through the chain like others."""
    if isinstance(src, tuple):
        pid = src[1]
        obj = g.p[pid].legend
        im = g.impl(g.p[pid].legend_name)
    else:
        obj = g.obj(src)
        pid = obj.ctrl
        im = g.impl(obj)
    ab = im.abilities[i]
    e, reqs = ability_cost(g, pid, obj, ab, ch)
    if not g.pay(pid, e, reqs, dict(kind="ability", obj=obj)):
        raise RuntimeError("cannot pay ability")
    if ab.get("exhaust"):
        obj.exhausted = True
    if ab.get("extra_cost"):
        ab["extra_cost"](g, pid, obj, ch)
    it = Item("ability", pid, f"{obj.cname}: {ab['name']}", fn=ab["resolve"], src=obj.uid, data=dict(ch))
    for j, uid in enumerate(ch.get("tg", ())):
        o = g.obj(uid)
        if o is not None:
            preds = ab.get("preds") or []
            g.add_target(it, o, preds[min(j, len(preds) - 1)] if preds else None)
    g.log(f"P{pid} activates {it.name} {fmt_choice(g, ch)}")
    g.stats[f"act_{obj.cname}:{ab['name']}"] += 1
    if not g.chain and g.chain_origin is None:
        g.chain_origin = "play"
    g.chain.append(it)
    g.priority = pid
    g.passes = 0
    g.on_finalize(it)
    g.need_cleanup = True


def fmt_choice(g, ch):
    parts = []
    for k, v in ch.items():
        if k in ("tg", "tg2"):
            parts.append(f"{k}=" + ",".join(str(g.obj(u) or u) for u in v))
        elif k in ("killed_info",):
            continue
        elif v is not False and v is not None and v != {} and v != ():
            parts.append(f"{k}={v}")
    return "(" + " ".join(parts) + ")" if parts else ""
