"""Batch radiance: the cards of the Radiance set (RAD, released 2026-10-23). See GUIDE.md.

New keywords of the Core Rules dated 2026-10-09 (rules/source/core_rules_2026-10-09.txt), handled by the engine:
[Deploy] (830: actions.gear_locations / has_deploy, Game.hold, cleanup step 5), [Disarm] (831: Game.combat_trigger_check
-> cards.disarm_trigger), [Show Off] (832: Impl.show_off, actions.show_options / shown_info) and the Bomb gear token
(187.12). "When I become exhausted" is the 'exhausted' event (Obj.exhausted, Game._advance).
Cards that can't be modelled faithfully stay out of IMPL: see NEEDS_radiance.md.
"""
from itertools import combinations

from cards import *  # noqa: F401,F403
from actions import affordable, total_cost, gear_locations, has_deploy, show_options, spell_variants
from lang import tr


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


def _cval(c):
    return c.spec["e"] + 2 * c.spec["p"] + (c.spec["might"] or 0) / 2


def _discard(g, pid, n):
    """Discard n (rule 422): the player chooses; as many as possible (rule 359.3.e.11)."""
    for _ in range(n):
        h = g.p[pid].hand
        if not h:
            return
        g.discard(pid, g.ask(pid, "discard", sorted(h, key=lambda c: (_cval(c), c.uid))))


def _bf_ask(g, pid, kind, locs=(0, 1)):
    locs = list(locs)
    if not locs:
        return None
    return g.ask(pid, kind, locs) if len(locs) > 1 else locs[0]


def _bomb(g, pid, loc=None):
    """'Play a Bomb gear token to a battlefield' ([Deploy]: any battlefield, rule 830.1.b)."""
    if loc is None:
        loc = _bf_ask(g, pid, "bomb_location")
    return make_token(g, "Bomb", pid, loc, ready=True)


def _gold(g, pid, ready=False):
    return make_token(g, "Gold", pid, "base", ready=ready)      # "play a Gold gear token exhausted"


def _unit_token(g, pid, name, locs=None, ready=False):
    """'Play a ... unit token': base or a battlefield you control (rules 184.2, 355.2), unless a location is given."""
    ls = token_locations(g, pid, name, locs)
    if not ls:
        return None
    loc = g.ask(pid, "token_location", ls, token=name) if len(ls) > 1 else ls[0]
    return make_token(g, name, pid, loc, ready)


def _from_facedown(info):
    """'played' event: was the card played from face down (Hidden)?"""
    it = info.get("item")
    if it is not None and it.data.get("from") is not None:
        return it.data.get("from") == "facedown"
    return getattr(info["card"], "played_from", None) == "facedown"


def _is_deploy_gear(g, o):
    return g.is_gear(o) and has_deploy(g, o)


def _units_here(g, loc, pid=None):
    return [u for u in g.units(pid) if u.loc == loc]


def _deflect_ok(g, pid, u):
    return g.can_pay(pid, 0, deflect_reqs(g, pid, dict(tg=(u.uid,))))


def _buff_targets(g, pid, hb=None):
    """'Give a unit +N': friendly units first (AI order), then the others."""
    fr = friends(g, pid, False, hb)
    return fr + [u for u in all_units(g, pid, False, hb) if u.ctrl != pid]


def _minus_targets(g, pid, hb=None, at_bf=False):
    es = enemies(g, pid, at_bf, hb)
    return es + [u for u in friends(g, pid, at_bf, hb)]


def _banish_playable(g, pid):
    """'Banish the top card of your Main Deck. Until your next Ending Phase, you may play that card. (You still pay
    its costs.)' (Runeblade Anvil, Akali, Brash): a play permission from Banishment (actions.playable_cards)."""
    pl = g.p[pid]
    if not pl.deck:
        return None                                     # not a draw: no burn out (rule 431)
    c = pl.deck[0]
    g.to_zone(c, "banish", by=pid)
    key = ("banished", c.uid, c.oid)
    g.effects.append(dict(kind="play_banished", pid=pid, uid=c.uid, oid=c.oid, key=key))

    def end(g_, eff, info):
        if info["pid"] == eff["pid"]:                   # "until your next Ending Phase"
            g_.effects = [e for e in g_.effects if e.get("key") != eff["key"] and e is not eff]
    g.effects.append(dict(on="end_turn", fn=end, pid=pid, key=key))
    g.log(f"  P{pid} banishes {c.cname} (may play it until their next Ending Phase)")
    return c


def _free_choices(g, pid, card, src, ignore_energy=False, extra_reqs=(), locs=None, free=True):
    """Legal (choice, energy, reqs) to play card as a limited play (rule 419.3) ignoring its cost (or only its Energy
    cost), plus extra_reqs; mandatory additional costs (Deflect...) still apply. Gear with [Deploy] go to a
    battlefield; [Show Off] choices are offered."""
    im = g.impl(card)
    if im is None or play_forbidden_(g, pid, card, src):
        return []
    typ = card.spec["type"]
    shows = show_options(g, pid, card, im)
    if typ == "Unit":
        base = [dict(ch, **sh) for ch in unit_play_choices(g, pid, card, src, locs) for sh in shows]
    elif typ == "Gear":
        base = [dict(loc=l, **sh) for l in gear_locations(g, pid, card) for sh in shows]
    elif typ == "Spell":
        ctx = dict(hidden_bf=None, card=card, src=src)
        if full_choices(g, pid) and im.all_choices is not None:
            base = im.all_choices(g, pid, ctx) or [dict()]
        else:
            base = im.choices(g, pid, ctx) if im.choices else [dict()]
        base = [dict(ch, **sh) for ch in cap(g, base, 8, pid) for sh in shows]
        base = spell_variants(g, pid, card, im, base, ctx)
    else:
        return []
    out = []
    for ch in base:
        ch = dict(ch)
        if ignore_energy:
            ch["ignore_energy"] = True
        elif free:
            ch["free"] = True
        e, reqs = total_cost(g, pid, card, ch, src)
        reqs = list(reqs) + list(extra_reqs)
        if g.can_pay(pid, e, reqs, pay_ctx(card)):
            out.append((ch, e, reqs))
    return out


def play_forbidden_(g, pid, card, src):
    from actions import play_forbidden
    return play_forbidden(g, pid, card, src)


def _play_from(g, pid, card, src, cands, may=False):
    """Pay and play one of the candidates of _free_choices (asked to pid)."""
    if not cands:
        return None
    opts = [c[0] for c in cands]
    if may:
        opts = opts + [None]
    ch = g.ask(pid, "play_choice", opts, card=card)
    if ch is None:
        return None
    _, e, reqs = next(c for c in cands if c[0] is ch)
    if (e or reqs) and not g.pay(pid, e, reqs, pay_ctx(card)):
        return None
    return play_card(g, pid, card, src, dict(ch), limited=True)


def _played_card(info):
    """'played' event of a card (not a token)."""
    return not info.get("token") and info["card"] is not None


def _exh_cost_choices(g, pid, exclude=None):
    """'As an additional cost, exhaust a friendly unit': the ready friendly units (least valuable first)."""
    return [u for u in sorted(g.units(pid), key=lambda u: (value(g, u), u.uid)) if not u.exhausted and u is not exclude]


def _pay_exh(g, pid, card, ch):
    u = g.obj(ch.get("exh")) if ch.get("exh") is not None else None
    if u is not None:
        u.exhausted = True


def _pay_ret(g, pid, card, ch):
    """'As an additional cost, return a friendly unit/gear to its owner's hand' (ret / ret_gear)."""
    for k in ("ret", "ret_gear"):
        x = g.obj(ch.get(k)) if ch.get(k) is not None else None
        if x is not None:
            ch[k + "_e"] = x.spec["e"] if not x.token else 0          # its Energy cost (rule 206)
            g.to_zone(x, "hand")


def _choose_up_to(n, cands_fn, pred, kind, enemy_first=True):
    """Finalization-time chooser for 'up to n' targets (rule 355.13: zero allowed); Deflect is paid (rule 809)."""
    def choose(g, it):
        cs = [u for u in cands_fn(g, it) if _deflect_ok(g, it.ctrl, u)]
        k = min(n, len(cs))
        opts = []
        pool = cap(g, cs, 4, it.ctrl)
        for m in range(k, 0, -1):
            for grp in combinations(pool if not full_choices(g, it.ctrl) else cs, m):
                opts.append(Opt(", ".join(g.label(u) for u in grp), tuple(u.uid for u in grp)))
        opts.append(Opt(tr("aucune", "none"), ()))
        pick = g.ask(it.ctrl, kind, opts, item=it)
        for uid in pick.value:
            u = g.obj(uid)
            if u is None or not pay_deflect(g, it, u):
                continue
            g.add_target(it, u, pred)
        return True
    return choose


def _mod_target(g, it, n, i=0):
    u = g.legal(it, i)
    if u is not None:
        g.mod(u, n)
    return u


def _gear_ctrl_of_item(g, item):
    """Controller of the gear whose ability this item is (activated, triggered or Deathknell), else None."""
    if item is None or item.kind == "spell":
        return None
    o = g.obj(item.src) if item.src is not None else None
    if o is None:
        o = item.data.get("_src_obj")
    if o is not None and getattr(o, "spec", None) is not None and g.is_gear(o):
        return o.ctrl
    info = item.data.get("info")
    if isinstance(info, dict) and info.get("spec", {}).get("type") == "Gear":
        return info["ctrl"]
    return None


# ====================================================================== tokens
# Bomb (rule 187.12) — "[Deploy] [Deathknell][>] Deal 2 to an enemy unit here and kill all other Bombs you control
# here." The enemy unit is chosen as the Deathknell is finalized; without one, the Bombs still die.
def _bomb_choose(g, it):
    info = it.data["info"]
    cs = [u for u in enemies(g, it.ctrl) if u.loc == info["loc"] and _deflect_ok(g, it.ctrl, u)]
    if full_choices(g, it.ctrl):
        cs = sorted(cs, key=lambda u: u.uid)
    if cs:
        u = g.ask(it.ctrl, "bomb_target", cs, item=it)
        if pay_deflect(g, it, u):
            g.add_target(it, u, lambda g_, it_, x: x.ctrl != it_.ctrl and x.loc == it_.data["info"]["loc"])
    return True


def _bomb_dk(g, it):
    info = it.data["info"]
    u = g.legal(it, 0)
    if u is not None:
        g.deal(u, 2, "ability", it.ctrl)
    others = [o for o in g.board if o.cname == "Bomb" and o.ctrl == it.ctrl and o.loc == info["loc"]
              and o.uid != info["uid"]]
    if others:
        g.kill(others, it.ctrl)


card("Bomb", kw={"Deploy": 1}, deathknell=_bomb_dk, dk_choose=_bomb_choose)


# ====================================================================== Fury
# Amateur Demolitionist — "When you play me, play a Bomb gear token to a battlefield."
card("Amateur Demolitionist", on_play=lambda g, o, ctx: _q(g, o, "Amateur Demolitionist", lambda g_, it: _bomb(g_, it.ctrl)))


# Banner of Conquest — "[Deploy] Friendly units here have [Assault]."
def _here_friend(g, src, o):
    return o is not src and o.spec["type"] == "Unit" and o.ctrl == src.ctrl and src.loc in (0, 1) and o.loc == src.loc


card("Banner of Conquest", kw={"Deploy": 1},
     aura_kw=lambda g, src, o: {"Assault": 1} if src.zone == "board" and _here_friend(g, src, o) else None)


# Bilgewater Tracker — "When you play me, deal 1 to each of up to 2 units."
def _tracker_res(g, it):
    for i in range(len(it.targets)):
        u = g.legal(it, i)
        if u is not None:
            g.deal(u, 1, "ability", it.ctrl)


card("Bilgewater Tracker", on_play=lambda g, o, ctx: _q(
    g, o, "Bilgewater Tracker", _tracker_res,
    choose=_choose_up_to(2, lambda g_, it: all_units(g_, it.ctrl, False, ctx.get("hidden_bf")), P_unit, "tracker_targets")))


# Brawl — "[Action] Give a unit [Assault] this turn equal to its Might." (its Might as Brawl resolves)
def _brawl(g, it):
    u = g.legal(it, 0)
    if u is not None and g.might(u) > 0:
        g.grant(u, "Assault", g.might(u), "turn")


card("Brawl", timing="action", preds=[P_unit], resolve=_brawl,
     choices=lambda g, pid, ctx: tg_choices(_buff_targets(g, pid, ctx["hidden_bf"])))


# Lingering Animus — "I enter ready if a friendly unit or gear died this turn."
card("Lingering Animus", enter_ready=lambda g, pid, c, ch: any(
    inf["ctrl"] == pid and inf["spec"]["type"] in ("Unit", "Gear") for inf in g.hist["died"]))


# Scorched Earth — "[Action] Deal 5 damage split among units at a single battlefield." (rule 355.14: each chosen
# unit gets at least 1)
def _split_opts(g, pid, us, total):
    """Damage splits among units us: (uid, n) tuples, the AI's few sensible ones first; a human gets every split."""
    out = []
    order = sorted(us, key=lambda u: (u.ctrl == pid, -value(g, u), u.uid))
    left, al = total, []
    for u in order:
        if left <= 0 or u.ctrl == pid:
            break
        n = min(left, max(1, g.might(u) - u.damage))
        al.append((u.uid, n))
        left -= n
    if al and left:
        al[-1] = (al[-1][0], al[-1][1] + left)
    if al:
        out.append(tuple(al))
    for u in order[:2]:
        if u.ctrl != pid:
            out.append(((u.uid, total),))
    if full_choices(g, pid):
        def parts(k, n):
            if k == 1:
                yield (n,)
                return
            for x in range(1, n - k + 2):
                for rest in parts(k - 1, n - x):
                    yield (x,) + rest
        for k in range(1, min(total, len(us)) + 1):
            for grp in combinations(sorted(us, key=lambda u: u.uid), k):
                for p in parts(k, total):
                    out.append(tuple((u.uid, n) for u, n in zip(grp, p)))
    res = []
    for a in out:
        if a and a not in res:
            res.append(a)
    return res


def _scorched_choices(g, pid, ctx):
    out = []
    hb = ctx["hidden_bf"]
    for b in (0, 1):
        if hb is not None and b != hb:
            continue
        us = [u for u in g.units() if u.loc == b and g.targetable(u, pid)]
        for a in _split_opts(g, pid, us, 5):
            out.append(dict(tg=tuple(x for x, _ in a), split=tuple(n for _, n in a)))
    return out


def _scorched(g, it):
    for i, n in enumerate(it.data.get("split", ())):
        u = g.legal(it, i)
        if u is not None:
            g.deal(u, n, "spell", it.ctrl)


card("Scorched Earth", timing="action", preds=[P_unit_bf], resolve=_scorched, choices=_scorched_choices,
     max_choices=12)


# Rally for War — "Choose an occupied enemy battlefield. Play three 1 might Recruit unit tokens there."
def _rally(g, it):
    b = g.bfs[it.data["bf"]]
    if b.ctrl is None or b.ctrl == it.ctrl or not g.units(loc=b.idx):
        return                                      # no longer an occupied enemy battlefield (rule 359.3.e)
    for _ in range(3):
        make_token(g, "Recruit", it.ctrl, b.idx)


card("Rally for War", resolve=_rally,
     choices=lambda g, pid, ctx: [dict(bf=b.idx) for b in g.bfs if b.ctrl == 1 - pid and g.units(loc=b.idx)])


# War Scout — "[Accelerate] When you play me, give a unit +2 might this turn."
def _plus_trigger(name, n, opts_fn=None, pred=P_unit, may=False):
    """'When you play me, give a unit +n might this turn' (n < 0: '-n'): the unit is chosen at finalization."""
    def on_play(g, o, ctx):
        hb = ctx.get("hidden_bf")
        fn = opts_fn or ((lambda g_, it: _buff_targets(g_, it.ctrl, hb)) if n > 0
                         else (lambda g_, it: _minus_targets(g_, it.ctrl, hb)))
        _q(g, o, name, lambda g_, it: _mod_target(g_, it, n), may=may, choose=trig_target(fn, pred))
    return on_play


card("War Scout", accelerate=True, on_play=_plus_trigger("War Scout", 2))


# Antagonistic Apparatus — "I enter ready. When you play me, play a ready 3 might Mech unit token here."
def _mech_here(ready):
    def res(g, it):
        me = g.obj(it.src)
        loc = me.loc if me is not None else it.data["loc"]
        make_token(g, "Mech", it.ctrl, loc, ready=ready)
    return res


card("Antagonistic Apparatus", enter_ready=True,
     on_play=lambda g, o, ctx: _q(g, o, "Antagonistic Apparatus", _mech_here(True), dict(loc=o.loc)))


# Forbidden Technique — "Ready a unit. When you play a card with Energy cost 7 or more, you may play this from your
# trash for [1][Fury][Fury]. Banish this." ("Banish this" is an instruction of the spell: it is banished as it
# resolves, wherever it was played from.)
def _ft_zone(g, c, ev, info):
    if ev == "played" and c.zone == "trash" and info["pid"] == c.owner and _played_card(info) \
            and info["card"].spec["e"] >= 7:
        def res(g_, it):
            x = next((y for y in g_.p[it.ctrl].trash if y.uid == it.data["uid"]), None)
            if x is None:
                return
            cands = _free_choices(g_, it.ctrl, x, "trash", extra_reqs=[frozenset({"Fury"})] * 2)
            cands = [(ch, e + 1, r) for ch, e, r in cands if g_.can_pay(it.ctrl, e + 1, r, pay_ctx(x))]
            _play_from(g_, it.ctrl, x, "trash", cands, may=True)
        g.queue_trigger(c.owner, "Forbidden Technique (trash)", res, dict(uid=c.uid))


card("Forbidden Technique", preds=[P_unit], banish_after=True, zone_event=_ft_zone,
     resolve=lambda g, it: g.legal(it, 0) is not None and g.ready_obj(g.legal(it, 0)),
     choices=lambda g, pid, ctx: tg_choices([u for u in friends(g, pid, False, ctx["hidden_bf"]) if u.exhausted]
                                            + [u for u in all_units(g, pid, False, ctx["hidden_bf"])
                                               if u.ctrl == pid and not u.exhausted]))


# Incorrigible Construct — "[Show Off] a Mech. I enter ready if you showed off."
card("Incorrigible Construct", show_off=lambda g, pid, c, x: "Mech" in (g.tags(x) if x in g.board else x.spec["tags"]),
     enter_ready=lambda g, pid, c, ch: ch.get("show") is not None)


# Runeblade Anvil — "When you play this or when this leaves the board, banish the top card of your Main Deck. Until
# your next Ending Phase, you may play that card. [2][Fury], [E]: Kill this."
def _anvil_leave(g, o, info):
    g.queue_trigger(info["info"]["ctrl"], "Runeblade Anvil (leaves)", lambda g_, it: _banish_playable(g_, it.ctrl))


card("Runeblade Anvil", on_leave=_anvil_leave,
     on_play=lambda g, o, ctx: _q(g, o, "Runeblade Anvil", lambda g_, it: _banish_playable(g_, it.ctrl)),
     abilities=[ability("Kill this", "2 energy and 1 fury rune", exhaust=True,
                        resolve=lambda g, it: g.obj(it.src) is not None and g.kill([g.obj(it.src)], it.ctrl))])


# Lost to the Sands — "[Action] Choose one — Deal 4 to a unit at a battlefield. Kill a gear."
def _lts_pred(g, it, o):
    return P_unit_bf(g, it, o) if it.data.get("mode") == "deal" else g.is_gear(o)


def _lts(g, it):
    o = g.legal(it, 0)
    if o is None:
        return
    if it.data.get("mode") == "deal":
        g.deal(o, 4, "spell", it.ctrl)
    else:
        g.kill([o], it.ctrl)


def _lts_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    out = [dict(mode="deal", tg=(u.uid,)) for u in _minus_targets(g, pid, hb, True)]
    gs = sorted([x for x in g.gear() if hb is None or x.loc == hb], key=lambda x: (x.ctrl == pid, -x.spec["e"], x.uid))
    out += [dict(mode="gear", tg=(x.uid,)) for x in gs if g.targetable(x, pid)]
    return out


card("Lost to the Sands", timing="action", preds=[_lts_pred], resolve=_lts, choices=_lts_choices, max_choices=10)


# Shimmering Swimmer — "You may kill a friendly unit or gear as an additional cost to play me. When you play me, if
# you paid the additional cost, deal 2 to a unit."
def _kill_cost_opts(g, pid, gear=True, units=True):
    xs = [o for o in g.board if o.ctrl == pid and ((units and o.spec["type"] == "Unit") or (gear and g.is_gear(o)))]
    return cap(g, sorted(xs, key=lambda o: (value(g, o), o.uid)), 3, pid)


def _deal_res(n):
    def res(g, it):
        u = g.legal(it, 0)
        if u is not None:
            g.deal(u, n, "ability", it.ctrl)
    return res


card("Shimmering Swimmer",
     as_played=lambda g, pid, c: [dict()] + [dict(kill=o.uid) for o in _kill_cost_opts(g, pid)],
     on_play=lambda g, o, ctx: ctx.get("kill") is not None and _q(
         g, o, "Shimmering Swimmer", _deal_res(2),
         choose=trig_target(lambda g_, it: _minus_targets(g_, it.ctrl, ctx.get("hidden_bf")), P_unit)))


# Akali, Brash — "The first time I move each turn, [Add] [1][A]. The second time I move each turn, banish the top
# card of your Main Deck. Until your next Ending Phase, you may play that card." ([Add] resolves at once, 430.2)
def _akali_brash(g, o, ev, info):
    if ev == "move" and info["obj"] is o:
        key = ("akali_brash", o.uid, o.oid, g.turn_no)
        g.stats[key] += 1
        if g.stats[key] == 1:
            g.add_pool(o.ctrl, 1, ["A"])
        elif g.stats[key] == 2:
            _q(g, o, "Akali, Brash", lambda g_, it: _banish_playable(g_, it.ctrl))


card("Akali, Brash", on_event=_akali_brash)


# Coalesced Energy — "At the end of your turn, recycle all your runes, then channel 4 runes exhausted."
def _coalesced_res(g, it):
    for r in list(g.p[it.ctrl].runes):
        g.recycle_rune(it.ctrl, r)
    g.channel(it.ctrl, 4, exhausted=True)


card("Coalesced Energy", on_event=lambda g, o, ev, info: ev == "end_turn" and info["pid"] == o.ctrl and _q(
    g, o, "Coalesced Energy", _coalesced_res))


# Detonation Amplifier — "[Deploy] When you play this, you may kill another friendly unit or gear to draw 1.
# Friendly [Deathknell] effects here trigger an additional time."
def _amp_play(g, o, ctx):
    def res(g_, it):
        xs = [x for x in g_.board if x.ctrl == it.ctrl and x.uid != it.src
              and (x.spec["type"] == "Unit" or g_.is_gear(x))]
        if not xs:
            return
        x = g_.ask(it.ctrl, "amp_kill", sorted(xs, key=lambda x: (value(g_, x), x.uid)) + [None])
        if x is not None and g_.kill([x], it.ctrl, cost=True):
            g_.draw(it.ctrl, 1)
    _q(g, o, "Detonation Amplifier", res)


card("Detonation Amplifier", kw={"Deploy": 1}, on_play=_amp_play,
     dk_extra=lambda g, src, inf: 1 if (src.zone == "board" and src.loc in (0, 1) and inf["ctrl"] == src.ctrl
                                         and inf["loc"] == src.loc and inf["uid"] != src.uid) else 0)


# Drakehound Packmaster — "When you play me, reveal the top card of each opponent's Main Deck. Choose one and banish it,
# then play it, ignoring its cost. Recycle the rest." (1v1: one revealed card; rule 419.3.c: if it can't be played,
# it stays banished)
def _packmaster(g, it):
    opp = 1 - it.ctrl
    top = g.reveal(it.ctrl, 1, owner=opp)
    if not top:
        return
    c = top[0]
    g.to_zone(c, "banish", by=it.ctrl)
    _play_from(g, it.ctrl, c, "banish", _free_choices(g, it.ctrl, c, "banish"))


card("Drakehound Packmaster", on_play=lambda g, o, ctx: _q(g, o, "Drakehound Packmaster", _packmaster))


# Firebrand Raider — "I enter ready. When you play me, ready all enemy units. Other units enter ready. Units don't
# ready during the Awaken Phase."
def _raider_res(g, it):
    for u in g.units(1 - it.ctrl):
        g.ready_obj(u)


card("Firebrand Raider", enter_ready=True,
     on_play=lambda g, o, ctx: _q(g, o, "Firebrand Raider", _raider_res),
     units_enter_ready=lambda g, src, pid, c: c is not src and c.spec["type"] == "Unit" and src.zone == "board",
     aura_no_ready=lambda g, src, o, by, kind: kind == "awaken" and o.spec["type"] == "Unit")


# Mordekaiser, Twice Slain — "[Accelerate] [Assault 3] Once each turn, if I would die, you may kill a friendly unit
# to heal me, exhaust me, and recall me instead."
def _morde_rep(g, src, u):
    if u is not src or g.stats[("morde_twice", src.uid, src.oid, g.turn_no)]:
        return None
    others = [x for x in g.units(src.ctrl) if x is not src and x not in g._dying]
    if not others:
        return None
    pid = src.ctrl

    def cost(g_):
        xs = [x for x in g_.units(pid) if x.uid != src.uid and x not in g_._dying]
        if not xs:
            return False
        x = g_.ask(pid, "morde_kill", sorted(xs, key=lambda x: (value(g_, x), x.uid)))
        g_.stats[("morde_twice", src.uid, src.oid, g_.turn_no)] = 1
        return bool(g_.kill([x], pid, cost=True))
    return dict(name="Mordekaiser, Twice Slain", may=True, cost=cost, apply=lambda g_, x: g_.save_unit(x))


card("Mordekaiser, Twice Slain", accelerate=True, kw={"Assault": 3}, death_rep=_morde_rep)


# Ekko, Revolutionary — "I enter ready. [Assault] [Action][>] Discard 1, [1][Fury]: Banish me, then play me to your
# base, ignoring my cost."
def _ekko_rev(g, it):
    me = g.obj(it.src)
    if me is None:
        return
    g.to_zone(me, "banish", by=it.ctrl)
    c = next((x for x in g.p[me.owner].banish if x.uid == me.uid), None)
    if c is not None:
        play_unit_free(g, it.ctrl, c, "banish", loc_choices=["base"])


card("Ekko, Revolutionary", enter_ready=True, kw={"Assault": 1}, abilities=[ability(
    "Banish and replay", "1 energy and 1 fury rune", timing="action", resolve=_ekko_rev,
    can=lambda g, pid, o: bool(g.p[pid].hand), extra_cost=lambda g, pid, o, ch: _discard(g, pid, 1))])


# Whirling Twirligig — "[E]: Play a Gold gear token exhausted. Kill a friendly gear, [E]: Play a 1 might Recruit unit
# token. Kill a friendly unit, [E]: Play a 3 might Mech unit token."
def _kill_ch(g, pid, kind):
    xs = [x for x in g.board if x.ctrl == pid and (g.is_gear(x) if kind == "gear" else x.spec["type"] == "Unit")]
    return [dict(kill=x.uid) for x in cap(g, sorted(xs, key=lambda x: (value(g, x), x.uid)), 3, pid)]


def _kill_extra(g, pid, o, ch):
    x = g.obj(ch.get("kill"))
    if x is not None:
        g.kill([x], pid, cost=True)


card("Whirling Twirligig", abilities=[
    ability("Gold", None, exhaust=True, resolve=lambda g, it: _gold(g, it.ctrl)),
    ability("Recruit", None, exhaust=True, choices=lambda g, pid, o: _kill_ch(g, pid, "gear"),
            extra_cost=_kill_extra, resolve=lambda g, it: _unit_token(g, it.ctrl, "Recruit")),
    ability("Mech", None, exhaust=True, choices=lambda g, pid, o: _kill_ch(g, pid, "unit"),
            extra_cost=_kill_extra, resolve=lambda g, it: _unit_token(g, it.ctrl, "Mech"))])


# Ziggs, Short-Fused — "When you play me, play a Bomb gear token to a battlefield. Abilities of friendly gear deal 1
# Bonus Damage." (Bonus Damage, rules 712-715: abilities whose source is a gear of Ziggs's controller)
card("Ziggs, Short-Fused",
     on_play=lambda g, o, ctx: _q(g, o, "Ziggs, Short-Fused", lambda g_, it: _bomb(g_, it.ctrl)),
     dmg_bonus=lambda g, src, ctx: 1 if (src.zone == "board" and ctx["kind"] == "ability"
                                         and _gear_ctrl_of_item(g, ctx["item"]) == src.ctrl) else 0)


# ====================================================================== Calm
# Blossoming Dreamroot — "[Deploy] When you win a combat here, draw 1."
card("Blossoming Dreamroot", kw={"Deploy": 1}, on_event=lambda g, o, ev, info: (
    ev == "combat_won" and info["pid"] == o.ctrl and info["bf"] == o.loc
    and _q(g, o, "Blossoming Dreamroot", lambda g_, it: g_.draw(it.ctrl, 1))))


# Consult the Heavens — "[Reaction] Give a unit [Shield 2] this turn. [Predict]."
def _consult(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.grant(u, "Shield", 2, "turn")
    g.predict(it.ctrl, 1)


card("Consult the Heavens", timing="reaction", preds=[P_unit], resolve=_consult,
     choices=lambda g, pid, ctx: tg_choices(_buff_targets(g, pid, ctx["hidden_bf"])))


# Entrenched Boar — "You may have me enter ready. If you do, I can't move this turn."
def _boar_play(g, o, ctx):
    if ctx.get("boar"):
        g.effects.append(dict(kind="cant_move", uid=o.uid, oid=o.oid, dur="turn",
                              fn=lambda g_, eff, x, dest, by, std: x.uid == eff["uid"] and x.oid == eff["oid"]))


card("Entrenched Boar", as_played=lambda g, pid, c: [dict(), dict(boar=True)],
     enter_ready=lambda g, pid, c, ch: bool(ch.get("boar")), on_play=_boar_play)


# Iridescent Avian — "[E]: [Buff] a unit. If it's already buffed, give it +2 might this turn instead."
def _avian(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    if u.buff > 0:
        g.mod(u, 2)
    else:
        g.buff(u)


card("Iridescent Avian", abilities=[ability(
    "Buff", None, exhaust=True, preds=[P_unit], resolve=_avian,
    choices=lambda g, pid, o: tg_choices([u for u in friends(g, pid) if u.buff == 0] + [u for u in friends(g, pid) if u.buff]
                                         + ([u for u in all_units(g, pid) if u.ctrl != pid] if full_choices(g, pid) else [])))])


# Ki Burst — "[Action] As an additional cost to play this, exhaust a friendly unit. [Stun] a unit."
def _exh_spell_choices(targets_fn):
    """Spell choices with the additional cost 'exhaust a friendly unit' (exh) and a target (tg)."""
    def choices(g, pid, ctx):
        out = []
        tgs = targets_fn(g, pid, ctx)
        for x in cap(g, _exh_cost_choices(g, pid), 2, pid):
            for t in tgs:
                out.append(dict(tg=t, exh=x.uid))
        return out
    return choices


card("Ki Burst", timing="action", preds=[P_unit], pay_extra=_pay_exh,
     resolve=lambda g, it: g.legal(it, 0) is not None and g.stun(g.legal(it, 0), it.ctrl),
     choices=_exh_spell_choices(lambda g, pid, ctx: [(u.uid,) for u in enemies(g, pid, False, ctx["hidden_bf"])
                                                     if not u.stunned or full_choices(g, pid)]
                                + ([(u.uid,) for u in friends(g, pid, False, ctx["hidden_bf"])] if full_choices(g, pid) else [])))


# Life from Stone — "[Reaction] As an additional cost to play this, exhaust a friendly unit. Draw 1 and channel 1 rune
# exhausted."
card("Life from Stone", timing="reaction", pay_extra=_pay_exh,
     resolve=lambda g, it: (g.draw(it.ctrl, 1), g.channel(it.ctrl, 1, exhausted=True)),
     choices=lambda g, pid, ctx: [dict(exh=x.uid) for x in cap(g, _exh_cost_choices(g, pid), 2, pid)])


# Sanctum Conservator — "When you play a card with Energy cost 7 or more, ready me." (printed cost, rule 206)
card("Sanctum Conservator", on_event=lambda g, o, ev, info: (
    ev == "played" and info["pid"] == o.ctrl and _played_card(info) and info["card"].spec["e"] >= 7
    and _q(g, o, "Sanctum Conservator", lambda g_, it: _me(g_, it) is not None and g_.ready_obj(_me(g_, it)))))


# Stoic Ferryman — "[Shield 3] When you play me to a battlefield, you may move another unit you control here to your
# base."
def _ferryman(g, o, ctx):
    if o.loc not in (0, 1):
        return
    here = o.loc

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.move([u], "base", it.ctrl)
    _q(g, o, "Stoic Ferryman", res, may=True, choose=trig_target(
        lambda g_, it: [u for u in friends(g_, it.ctrl) if u.loc == here and u.uid != it.src],
        lambda g_, it, u: u.ctrl == it.ctrl and u.loc == here and u.uid != it.src))


card("Stoic Ferryman", kw={"Shield": 3}, on_play=_ferryman)


# Battle Preparations — "[Reaction] [Show Off] a gear. Give a unit +2 might this turn. If you showed off, ready up to
# 2 runes."
def _ready_runes(g, pid, n):
    for _ in range(n):
        ex = [r for r in g.p[pid].runes if r.exhausted]
        if not ex:
            return
        doms = sorted({r.domain for r in ex})
        r = g.ask(pid, "ready_rune", ex) if len(doms) > 1 else ex[0]
        r.exhausted = False


def _battle_prep(g, it):
    _mod_target(g, it, 2)
    if it.data.get("shown"):
        _ready_runes(g, it.ctrl, 2)


card("Battle Preparations", timing="reaction", preds=[P_unit], resolve=_battle_prep,
     show_off=lambda g, pid, c, x: g.is_gear(x) if x in g.board else x.spec["type"] == "Gear",
     choices=lambda g, pid, ctx: tg_choices(_buff_targets(g, pid, ctx["hidden_bf"])))


# Command: Attack — "Choose one — Move a friendly gear with [Deploy] to a battlefield. Look at the top 5 cards of your
# Main Deck. You may reveal a gear from among them and draw it. Recycle the rest."
def _command_attack(g, it):
    if it.data.get("mode") == "move":
        x = g.legal(it, 0)
        if x is not None and x.loc != it.data["dest"]:
            g.move([x], it.data["dest"], it.ctrl)
        return
    pl = g.p[it.ctrl]
    top = g.look(it.ctrl, pl.deck[:5])
    gs = [c for c in top if c.spec["type"] == "Gear"]
    pick = g.ask(it.ctrl, "gear_pick", sorted(gs, key=lambda c: (-_cval(c), c.uid)) + [None]) if gs else None
    if pick is not None:
        g.log(f"  P{it.ctrl} reveals {pick.cname}")
        g.draw_card(it.ctrl, pick)
    rest = [c for c in top if c is not pick and c in pl.deck]
    for c in rest:
        pl.deck.remove(c)
    g.recycle_cards(it.ctrl, rest)


def _ca_choices(g, pid, ctx):
    out = [dict(mode="look")]
    for x in g.gear(pid):
        if has_deploy(g, x):
            out += [dict(mode="move", tg=(x.uid,), dest=b) for b in (0, 1) if b != x.loc]
    return out


card("Command: Attack", resolve=_command_attack, choices=_ca_choices,
     preds=[lambda g, it, o: o.ctrl == it.ctrl and _is_deploy_gear(g, o)])


# Hunker Down — "[Reaction] Ready a friendly unit. It can't move this turn. Draw 1."
def _hunker(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.ready_obj(u)
        g.effects.append(dict(kind="cant_move", uid=u.uid, oid=u.oid, dur="turn",
                              fn=lambda g_, eff, x, dest, by, std: x.uid == eff["uid"] and x.oid == eff["oid"]))
    g.draw(it.ctrl, 1)


card("Hunker Down", timing="reaction", preds=[P_friend], resolve=_hunker,
     choices=lambda g, pid, ctx: tg_choices([u for u in friends(g, pid, False, ctx["hidden_bf"]) if u.exhausted]
                                            + [u for u in friends(g, pid, False, ctx["hidden_bf"]) if not u.exhausted]))


# Invigorating Bloom — "When you play this, give a friendly unit +3 might this turn. Exhaust a friendly [Mighty] unit,
# [E]: Draw 1."
def _bloom_extra(g, pid, o, ch):
    u = g.obj(ch.get("exh"))
    if u is not None:
        u.exhausted = True


card("Invigorating Bloom",
     on_play=_plus_trigger("Invigorating Bloom", 3, lambda g_, it: friends(g_, it.ctrl), P_friend),
     abilities=[ability("Draw", None, exhaust=True, extra_cost=_bloom_extra,
                        choices=lambda g, pid, o: [dict(exh=u.uid) for u in _exh_cost_choices(g, pid) if g.mighty(u)],
                        resolve=lambda g, it: g.draw(it.ctrl, 1))])


# Storm Raptor — "[Deflect] When a stunned enemy unit dies here, draw 1."
card("Storm Raptor", kw={"Deflect": 1}, on_event=lambda g, o, ev, info: (
    ev == "die" and info["info"]["ctrl"] != o.ctrl and info["info"]["stunned"] and o.loc in (0, 1)
    and info["info"]["loc"] == o.loc and info["info"]["spec"]["type"] == "Unit"
    and _q(g, o, "Storm Raptor", lambda g_, it: g_.draw(it.ctrl, 1))))


# Watchful Curator — "Your gear have [Deflect]."
card("Watchful Curator", aura_kw=lambda g, src, o: {"Deflect": 1} if (
    src.zone == "board" and src.spec["type"] == "Unit" and o.ctrl == src.ctrl and g.is_gear(o)) else None)


# Ahri, Confident — "[Disarm] When you reduce the Might of an enemy unit here by 1 or more, give me +1 might this
# turn." ('might_reduced' event: a -might given by a spell or ability you control)
card("Ahri, Confident", kw={"Disarm": 1}, on_event=lambda g, o, ev, info: (
    ev == "might_reduced" and info["by"] == o.ctrl and info["obj"].ctrl != o.ctrl and o.loc in (0, 1)
    and info["obj"].loc == o.loc and info["amount"] >= 1
    and _q(g, o, "Ahri, Confident", lambda g_, it: _me(g_, it) is not None and g_.mod(_me(g_, it), 1))))


# Ephemeral Ward — "Your units have +2 might. When you play a unit, kill this."
card("Ephemeral Ward",
     aura_might=lambda g, src, o: 2 if src.zone == "board" and o.ctrl == src.ctrl and o.spec["type"] == "Unit"
     and o is not src else 0,
     on_event=lambda g, o, ev, info: (
         ev == "played" and info["pid"] == o.ctrl and info["card"].spec["type"] == "Unit"
         and _q(g, o, "Ephemeral Ward", lambda g_, it: _me(g_, it) is not None and g_.kill([_me(g_, it)], it.ctrl))))


# Heartening Soul — "When you play me, ready a friendly legend. Friendly legends have "[E]: Give a unit +3 might this
# turn.""
_SOUL_AB = ability("Give +3", None, exhaust=True, preds=[P_unit], resolve=lambda g, it: _mod_target(g, it, 3),
                   choices=lambda g, pid, o: tg_choices(_buff_targets(g, pid)))


def _soul_ready(g, it):
    leg = g.p[it.ctrl].legend
    if leg.exhausted:
        leg.exhausted = False
        g.emit("ready", obj=leg, by=it.ctrl)


card("Heartening Soul", on_play=lambda g, o, ctx: _q(g, o, "Heartening Soul", _soul_ready),
     grant_abilities=lambda g, src, o: [_SOUL_AB] if (src.zone == "board" and getattr(o, "zone", None) == "legend"
                                                       and o.owner == src.ctrl) else [])


# Hush — "[Reaction] Counter a spell with Power cost [A][A] or more." (printed Power cost, rule 206)
def _hush(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is not None and tgt.kind == "spell" and tgt.card.spec["p"] >= 2:
        g.counter(tgt)


card("Hush", timing="reaction", resolve=_hush,
     choices=lambda g, pid, ctx: [dict(item=i.id) for i in reversed(g.chain) if i.kind == "spell"
                                  and i.card.spec["p"] >= 2 and g.counterable(i) and (i.ctrl != pid or full_choices(g, pid))])


# Orianna, Exquisite — "[Shield] I don't deal combat damage unless there's a friendly gear here."
card("Orianna, Exquisite", kw={"Shield": 1}, no_combat_damage=lambda g, o: not any(
    x.ctrl == o.ctrl and x.loc == o.loc for x in g.gear()))


# Riven, Broken Blade — "When I become exhausted, choose one that hasn't been chosen this turn — [Buff] me. Spend my
# buff to [Stun] an enemy unit. Spend my buff to ready me."
_RIVEN_MODES = ("buff", "stun", "ready")


def _riven_bb(g, o, ev, info):
    if ev != "exhausted" or o not in info["objs"]:
        return
    key = ("riven_bb", o.uid, o.oid, g.turn_no)

    def choose(g_, it):
        me = _me(g_, it)
        done = {e["mode"] for e in g_.effects if e.get("key") == key}
        modes = [m for m in _RIVEN_MODES if m not in done]
        if not modes:
            return False
        has = me is not None and me.buff > 0
        order = (["stun", "ready", "buff"] if has else ["buff", "stun", "ready"])
        modes.sort(key=order.index)
        labels = dict(buff=tr("me renforcer (buff)", "buff me"), stun=tr("dépenser mon buff : étourdir un ennemi",
                      "spend my buff: stun an enemy"), ready=tr("dépenser mon buff : me préparer", "spend my buff: ready me"))
        m = g_.ask(it.ctrl, "riven_mode", [Opt(labels[x], x) for x in modes], item=it).value
        g_.effects.append(dict(key=key, mode=m, dur="turn"))
        it.data["mode"] = m
        if m == "stun":
            cs = [u for u in enemies(g_, it.ctrl) if _deflect_ok(g_, it.ctrl, u)]
            if cs:
                u = g_.ask(it.ctrl, "target", cs, item=it)
                if pay_deflect(g_, it, u):
                    g_.add_target(it, u, P_enemy)
        return True

    def res(g_, it):
        me = _me(g_, it)
        m = it.data.get("mode")
        if me is None:
            return
        if m == "buff":
            g_.buff(me)
        elif m == "stun":
            u = g_.legal(it, 0)
            if u is not None and g_.spend_buff(me, it.ctrl):
                g_.stun(u, it.ctrl)
        elif m == "ready" and g_.spend_buff(me, it.ctrl):
            g_.ready_obj(me)
    _q(g, o, "Riven, Broken Blade", res, choose=choose)


card("Riven, Broken Blade", on_event=_riven_bb)


# Hunting Blind — "[Deploy] When you play this, you may move an enemy unit to this battlefield. Friendly units here
# have [Shield 2]."
def _blind_play(g, o, ctx):
    def res(g_, it):
        u, me = g_.legal(it, 0), g_.obj(it.src)
        if u is not None and me is not None and me.loc in (0, 1) and u.loc != me.loc:
            g_.move([u], me.loc, it.ctrl)
    _q(g, o, "Hunting Blind", res, may=True, choose=trig_target(
        lambda g_, it: [u for u in enemies(g_, it.ctrl) if g_.obj(it.src) is not None and u.loc != g_.obj(it.src).loc],
        P_enemy))


card("Hunting Blind", kw={"Deploy": 1}, on_play=_blind_play,
     aura_kw=lambda g, src, o: {"Shield": 2} if src.zone == "board" and _here_friend(g, src, o) else None)


# K'Sante, Dauntless — "[Shield] When I attack or defend, double the [Shield] of a unit you control here this turn."
def _ksante_d(g, o, ev, info):
    if ev in ("attack", "defend") and info["obj"] is o:
        here = o.loc

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                n = g_.kw_value(u, "Shield")
                if n > 0:
                    g_.grant(u, "Shield", n, "turn")
        _q(g, o, "K'Sante, Dauntless", res, choose=trig_target(
            lambda g_, it: sorted([u for u in g_.units(it.ctrl) if u.loc == here],
                                  key=lambda u: (-g_.kw_value(u, "Shield"), u.uid)),
            lambda g_, it, u: u.ctrl == it.ctrl and u.loc == here))


card("K'Sante, Dauntless", kw={"Shield": 1}, on_event=_ksante_d)


# Resplendent Rostrum — "[E]: Draw 1 and channel 1 rune exhausted. When this becomes exhausted, give a friendly unit
# +3 might this turn."
card("Resplendent Rostrum",
     abilities=[ability("Draw", None, exhaust=True,
                        resolve=lambda g, it: (g.draw(it.ctrl, 1), g.channel(it.ctrl, 1, exhausted=True)))],
     on_event=lambda g, o, ev, info: ev == "exhausted" and o in info["objs"] and _q(
         g, o, "Resplendent Rostrum", lambda g_, it: _mod_target(g_, it, 3),
         choose=trig_target(lambda g_, it: friends(g_, it.ctrl), P_friend)))


# ====================================================================== Mind
# Fanfare — "The next three times you play a unit this turn, draw 1."
def _fanfare(g, it):
    pid = it.ctrl

    def fire(g_, eff, info):
        if info["pid"] != eff["pid"] or info["card"].spec["type"] != "Unit":
            return
        eff["left"] -= 1
        if eff["left"] <= 0:
            g_.effects.remove(eff)
        g_.queue_trigger(eff["pid"], "Fanfare", lambda g2, it2: g2.draw(it2.ctrl, 1))
    g.effects.append(dict(on="played", fn=fire, pid=pid, left=3, dur="turn"))


card("Fanfare", resolve=_fanfare)


# High Note — "[Reaction] Give a unit -might this turn equal to the number of exhausted units you control."
def _high_note(g, it):
    u = g.legal(it, 0)
    n = sum(1 for x in g.units(it.ctrl) if x.exhausted)
    if u is not None and n:
        g.mod(u, -n)


card("High Note", timing="reaction", preds=[P_unit], resolve=_high_note,
     choices=lambda g, pid, ctx: tg_choices(_minus_targets(g, pid, ctx["hidden_bf"])))


# Industrious Mechsmith — "When you play me, play a 3 might Mech unit token here."
card("Industrious Mechsmith",
     on_play=lambda g, o, ctx: _q(g, o, "Industrious Mechsmith", _mech_here(False), dict(loc=o.loc)))


# Mysterious Chronomancer — "When you play me, [Predict 2]. [Disarm 2]"
card("Mysterious Chronomancer", kw={"Disarm": 2},
     on_play=lambda g, o, ctx: _q(g, o, "Mysterious Chronomancer", lambda g_, it: g_.predict(it.ctrl, 2)))


# Parallel Convergence — "As an additional cost to play this, return a friendly unit to its owner's hand. Deal 5 to a
# unit at a battlefield."
def _ret_spell_choices(targets_fn):
    def choices(g, pid, ctx):
        out = []
        tgs = targets_fn(g, pid, ctx)
        rets = cap(g, sorted(g.units(pid), key=lambda u: (u.token, -u.spec["e"], u.uid)), 2, pid)
        for r in rets:
            for t in tgs:
                if r.uid not in t:
                    out.append(dict(tg=t, ret=r.uid))
        return out
    return choices


card("Parallel Convergence", preds=[P_unit_bf], pay_extra=_pay_ret,
     resolve=lambda g, it: g.deal(g.legal(it, 0), 5, "spell", it.ctrl),
     choices=_ret_spell_choices(lambda g, pid, ctx: [(u.uid,) for u in _minus_targets(g, pid, ctx["hidden_bf"], True)]))


# Resource Extractor — "[Deploy] At the end of your turn, if you control this battlefield, play a Gold gear token
# exhausted."
card("Resource Extractor", kw={"Deploy": 1}, on_event=lambda g, o, ev, info: (
    ev == "end_turn" and info["pid"] == o.ctrl and o.loc in (0, 1) and g.bfs[o.loc].ctrl == o.ctrl
    and _q(g, o, "Resource Extractor", lambda g_, it: _gold(g_, it.ctrl))))


# Strigian Oracle — "When I become exhausted, [Predict]."
card("Strigian Oracle", on_event=lambda g, o, ev, info: ev == "exhausted" and o in info["objs"] and _q(
    g, o, "Strigian Oracle", lambda g_, it: g_.predict(it.ctrl, 1)))


# Clanging Sentry — "Your Mechs have [Disarm]." (values add up, rule 831.2)
card("Clanging Sentry", aura_kw=lambda g, src, o: {"Disarm": 1} if (
    src.zone == "board" and o.ctrl == src.ctrl and o.spec["type"] == "Unit" and "Mech" in g.tags(o)) else None)


# Improvised Projectile — "[Action] Choose a friendly gear. Deal damage equal to its Energy cost to a unit at a
# battlefield." (its printed Energy cost, rule 206; an illegal gear gives no information: nothing is dealt, 359.3.e.12)
def _projectile(g, it):
    x, u = g.legal(it, 0), g.legal(it, 1)
    if x is not None and u is not None:
        n = x.spec["e"] if not x.token or x.copy else 0
        g.deal(u, n, "spell", it.ctrl)


def _projectile_choices(g, pid, ctx):
    gs = sorted(g.gear(pid), key=lambda x: (-x.spec["e"], x.uid))
    us = _minus_targets(g, pid, ctx["hidden_bf"], True)
    return [dict(tg=(x.uid, u.uid)) for x in cap(g, gs, 2, pid) for u in us if u.uid != x.uid]


card("Improvised Projectile", timing="action",
     preds=[lambda g, it, o: g.is_gear(o) and o.ctrl == it.ctrl, P_unit_bf], resolve=_projectile,
     choices=_projectile_choices)


# Piltovan Guidebot — "When you play a card, you may have my base Might become equal to that card's Energy cost this
# turn." (its printed Energy cost, rule 206)
def _guidebot(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and _played_card(info):
        n = info["card"].spec["e"]

        def res(g_, it):
            me = _me(g_, it)
            if me is not None:
                g_.effects.append(dict(kind="base_might", uid=me.uid, oid=me.oid, value=it.data["n"], dur="turn"))
        _q(g, o, "Piltovan Guidebot", res, dict(n=n), may=True)


card("Piltovan Guidebot", on_event=_guidebot)


# Riptide Mage — "[Accelerate] When you play me, give an enemy unit -3 might this turn."
card("Riptide Mage", accelerate=True, on_play=lambda g, o, ctx: _q(
    g, o, "Riptide Mage", lambda g_, it: _mod_target(g_, it, -3),
    choose=trig_target(lambda g_, it: enemies(g_, it.ctrl, False, ctx.get("hidden_bf")), P_enemy)))


# Supernova — "Deal 10 to a unit. Draw 2."
card("Supernova", preds=[P_unit],
     resolve=lambda g, it: (g.deal(g.legal(it, 0), 10, "spell", it.ctrl), g.draw(it.ctrl, 2)),
     choices=lambda g, pid, ctx: tg_choices(_minus_targets(g, pid, ctx["hidden_bf"])))


# The Empyrean — "[Show Off] a card. When you play me, if you showed off a card, [Predict] equal to its Power cost,
# then draw 2." (its printed Power cost as it was shown, rule 832.1.d.1; a token isn't a card, rule 052)
def _empyrean(g, o, ctx):
    sh = ctx.get("shown")
    if sh is None:
        return

    def res(g_, it):
        if it.data["p"] > 0:
            g_.predict(it.ctrl, it.data["p"])
        g_.draw(it.ctrl, 2)
    _q(g, o, "The Empyrean", res, dict(p=sh["p"]))


card("The Empyrean", on_play=_empyrean, show_off=lambda g, pid, c, x: not x.token,
     show_key=lambda g, x: -x.spec["p"])


# Ekko, Ingenious — "As an additional cost to play me, return a friendly unit to its owner's hand. [Ganking]"
card("Ekko, Ingenious", kw={"Ganking": 1}, pay_extra=_pay_ret,
     as_played=lambda g, pid, c: [dict(ret=u.uid) for u in cap(g, sorted(
         g.units(pid), key=lambda u: (not u.token, -u.spec["e"], u.uid)), 3, pid)])


# Gravity Field — "[Deploy] Opponents must pay [1][A] to move one or more units to this battlefield. [Deathknell]
# Draw 1."
card("Gravity Field", kw={"Deploy": 1}, deathknell=lambda g, it: g.draw(it.ctrl, 1),
     move_cost=lambda g, src, pid, units, dest: (1, [ANY]) if (
         src.zone == "board" and src.loc in (0, 1) and dest == src.loc and pid != src.ctrl) else None)


# Kai'Sa, Rebel — "[Disarm] While I'm in a showdown, your spells have [Reaction]."
card("Kai'Sa, Rebel", kw={"Disarm": 1},
     card_kw=lambda g, src, pid, c, kw, zone: (
         kw == "Reaction" and src.zone == "board" and pid == src.ctrl and c.spec["type"] == "Spell"
         and g.sd is not None and src.loc == g.sd.bf))


# Scrutinizing Scope — "[1], [E]: Choose an opponent. They guess a card type. Reveal the top card of your Main Deck.
# If it has that type, recycle it. Otherwise, draw it."
def _scope(g, it):
    opp = 1 - it.ctrl
    guess = g.ask(opp, "scope_guess", [Opt(tr("unité", "unit"), "Unit"), Opt(tr("sort", "spell"), "Spell"),
                                       Opt(tr("équipement", "gear"), "Gear")]).value
    top = g.reveal(it.ctrl, 1)
    if not top:
        return
    c = top[0]
    if c.spec["type"] == guess:
        g.p[it.ctrl].deck.remove(c)
        g.recycle_cards(it.ctrl, [c])
    else:
        g.draw_card(it.ctrl, c)


card("Scrutinizing Scope", abilities=[ability("Guess", "1 energy", exhaust=True, resolve=_scope)])


# Seraphine, Inspiring — "[Disarm 2] Once each turn, when you exhaust three or more friendly units at the same time,
# you may draw 1." (objects exhausted by the same action: one 'exhausted' event)
def _sera_insp(g, o, ev, info):
    if ev == "exhausted":
        mine = [x for x in info["objs"] if x.ctrl == o.ctrl and x.spec["type"] == "Unit"]
        key = ("sera_insp", o.uid, o.oid, g.turn_no)
        if len(mine) >= 3 and not g.stats[key]:
            g.stats[key] = 1
            _q(g, o, "Seraphine, Inspiring", lambda g_, it: g_.draw(it.ctrl, 1), may=True)


card("Seraphine, Inspiring", kw={"Disarm": 2}, on_event=_sera_insp)


# Spirit of the Forge — "You may return a friendly gear to its owner's hand as an additional cost to play me. When you
# play me, if you paid the additional cost, ready me and give me +might this turn equal to the Energy cost of the
# gear you returned."
def _forge_play(g, o, ctx):
    if ctx.get("ret_gear") is None:
        return

    def res(g_, it):
        me = _me(g_, it)
        if me is not None:
            g_.ready_obj(me)
            if it.data["n"]:
                g_.mod(me, it.data["n"])
    _q(g, o, "Spirit of the Forge", res, dict(n=ctx.get("ret_gear_e", 0)))


card("Spirit of the Forge", pay_extra=_pay_ret, on_play=_forge_play,
     as_played=lambda g, pid, c: [dict()] + [dict(ret_gear=x.uid) for x in cap(g, sorted(
         g.gear(pid), key=lambda x: (-x.spec["e"], x.uid)), 3, pid)])


# Clocktower Guardian — "When you play me or when I hold, [Predict 3]. [Deathknell] Banish the top card of your Main
# Deck. You may play it, ignoring its Energy cost."
def _clock_dk(g, it):
    pl = g.p[it.ctrl]
    if not pl.deck:
        return
    c = pl.deck[0]
    g.to_zone(c, "banish", by=it.ctrl)
    _play_from(g, it.ctrl, c, "banish", _free_choices(g, it.ctrl, c, "banish", ignore_energy=True), may=True)


card("Clocktower Guardian", deathknell=_clock_dk,
     on_play=lambda g, o, ctx: _q(g, o, "Clocktower Guardian", lambda g_, it: g_.predict(it.ctrl, 3)),
     on_event=lambda g, o, ev, info: ev == "hold" and info["pid"] == o.ctrl and o in info["units"] and _q(
         g, o, "Clocktower Guardian", lambda g_, it: g_.predict(it.ctrl, 3)))


# Orianna, Mechanical Marvel — "Once each turn, while I'm at a battlefield, you may pay [1][A] less to play a gear."
# (a discount: always taken, used up by the first gear played with it)
def _om_key(g, src):
    return ("orianna_mm", src.uid, src.oid, g.turn_no)


def _om_aura(g, src, pid, what):
    if src.zone == "board" and src.loc in (0, 1) and pid == src.ctrl and what["kind"] == "gear" \
            and what["card"] is not None and not g.stats[_om_key(g, src)]:
        return [dict(e=-1, rm=1)]
    return []


def _om_event(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and _played_card(info) and info["card"].spec["type"] == "Gear" \
            and o.loc in (0, 1) and not getattr(info["card"], "limited_play", True):
        g.stats[_om_key(g, o)] = 1


card("Orianna, Mechanical Marvel", cost_aura=_om_aura, on_event=_om_event)


# Zero Point Inflection — "When you play a card, draw 2. When you draw 1 or more cards with 9 or more other cards in
# your hand, discard your hand, kill this, and choose an opponent. They score 1 point."
def _zpi_boom(g, it):
    me = _me(g, it)
    pl = g.p[it.ctrl]
    for c in list(pl.hand):
        g.discard(it.ctrl, c)
    if me is not None:
        g.kill([me], it.ctrl)
    g.gain_point(1 - it.ctrl, "Zero Point Inflection")


def _zpi(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and _played_card(info):
        _q(g, o, "Zero Point Inflection", lambda g_, it: g_.draw(it.ctrl, 2))
    elif ev == "draw" and info["pid"] == o.ctrl and len(g.p[o.ctrl].hand) - 1 >= 9:
        key = ("zpi", o.uid, o.oid)
        if not any(t.data.get("_zpi") == key for t in g.trigq + g.chain if t.kind == "trigger"):
            _q(g, o, "Zero Point Inflection (hand)", _zpi_boom, dict(_zpi=key))


card("Zero Point Inflection", on_event=_zpi)


# ====================================================================== Body
# Hold 'Em — "[Action] This costs [2] less if you've played a unit or gear to a battlefield this turn. Give a unit +3
# might this turn."
card("Hold 'Em", timing="action", preds=[P_unit], resolve=lambda g, it: _mod_target(g, it, 3),
     cost_mod=lambda g, pid, c, ch: (2, 0) if g.hist["to_bf"][pid] else (0, 0),
     choices=lambda g, pid, ctx: tg_choices(_buff_targets(g, pid, ctx["hidden_bf"])))


# Nazuman Ntofo — "[Equip] [1]" ; Effect Text and Might Bonus read on the card image: [Shield 2], +0 might.
card("Nazuman Ntofo", equip="1 energy", bonus=0, equip_kw={"Shield": 2})


# Overloaded Traveler — "While there's a friendly gear with [Deploy] at my battlefield, I have +1 might."
card("Overloaded Traveler", might_mod=lambda g, o: 1 if o.loc in (0, 1) and any(
    x.ctrl == o.ctrl and x.loc == o.loc and has_deploy(g, x) for x in g.gear()) else 0)


# Pillaged Armory — "[Deploy] When you play this or when you conquer here, [Buff] a friendly unit."
def _armory_trig(g, o):
    _q(g, o, "Pillaged Armory", lambda g_, it: g_.legal(it, 0) is not None and g_.buff(g_.legal(it, 0)),
       choose=trig_target(lambda g_, it: [u for u in friends(g_, it.ctrl) if u.buff == 0]
                          + [u for u in friends(g_, it.ctrl) if u.buff], P_friend))


card("Pillaged Armory", kw={"Deploy": 1}, on_play=lambda g, o, ctx: _armory_trig(g, o),
     on_event=lambda g, o, ev, info: ev == "conquer" and info["pid"] == o.ctrl and info["bf"] == o.loc
     and _armory_trig(g, o))


# Raving Witness — "When I move, reveal the top card of your Main Deck. If it's [Mighty], draw it. Otherwise, recycle
# it." (a card outside the board is Mighty with 5+ printed Might, rule 706)
def _witness_res(g, it):
    top = g.reveal(it.ctrl, 1)
    if not top:
        return
    c = top[0]
    if c.spec["type"] == "Unit" and g.mighty(c):
        g.draw_card(it.ctrl, c)
    elif c in g.p[it.ctrl].deck:
        g.p[it.ctrl].deck.remove(c)
        g.recycle_cards(it.ctrl, [c])


card("Raving Witness", on_event=lambda g, o, ev, info: ev == "move" and info["obj"] is o and _q(
    g, o, "Raving Witness", _witness_res))


# Skystrike Squadron — "You may return a friendly gear to its owner's hand as an additional cost to play me. If you
# do, I cost [2] less. [Deflect]"
card("Skystrike Squadron", kw={"Deflect": 1}, pay_extra=_pay_ret,
     cost_mod=lambda g, pid, c, ch: (2, 0) if ch.get("ret_gear") is not None else (0, 0),
     as_played=lambda g, pid, c: [dict()] + [dict(ret_gear=x.uid) for x in cap(g, sorted(
         g.gear(pid), key=lambda x: (x.spec["e"], x.uid)), 3, pid)])


# Sneaky Zeebles — "When I conquer, [Add] [A]." (an [Add] resolves at once, rule 430.2)
card("Sneaky Zeebles", on_event=lambda g, o, ev, info: (
    ev == "conquer" and info["pid"] == o.ctrl and o in info["units"] and g.add_pool(o.ctrl, 0, ["A"])))


# Wrath of the Freljord — "I enter ready. [Shield 3]"
card("Wrath of the Freljord", enter_ready=True, kw={"Shield": 3})


# Armored Tusker — "[Shield 3] [Tank] When I move, you may deal damage equal to my [Shield] to a unit at a
# battlefield." (its Shield value as the ability resolves)
def _tusker_res(g, it):
    u, me = g.legal(it, 0), g.obj(it.src)
    n = g.kw_value(me, "Shield") if me is not None else it.data["n"]
    if u is not None and n > 0:
        g.deal(u, n, "ability", it.ctrl)


card("Armored Tusker", kw={"Shield": 3, "Tank": 1}, on_event=lambda g, o, ev, info: (
    ev == "move" and info["obj"] is o and _q(
        g, o, "Armored Tusker", _tusker_res, dict(n=g.kw_value(o, "Shield")), may=True,
        choose=trig_target(lambda g_, it: _minus_targets(g_, it.ctrl, None, True), P_unit_bf))))


# Avalanche — "Deal 3 to all units in bases."
def _avalanche(g, it):
    for u in [u for u in g.units() if u.loc == "base"]:
        g.deal(u, 3, "spell", it.ctrl)


card("Avalanche", resolve=_avalanche)


# Bloodscent Swarm — "I cost [2] less for each battlefield you've conquered this turn."
card("Bloodscent Swarm", cost_mod=lambda g, pid, c, ch: (2 * len(set(g.hist["conq_bf"][pid])), 0))


# Elnuk Herd — "[Hidden] When you play me, give another friendly unit +3 might this turn, then recycle me if I was
# played from face down."
def _elnuk_res(g, it):
    _mod_target(g, it, 3)
    me = _me(g, it)
    if it.data["fd"] and me is not None:
        g.recycle_cards(it.ctrl, [me])


def _elnuk_play(g, o, ctx):
    fd = ctx.get("src") == "facedown"
    _q(g, o, "Elnuk Herd", _elnuk_res, dict(fd=fd), choose=trig_target(
        lambda g_, it: [u for u in friends(g_, it.ctrl) if u.uid != it.src],
        lambda g_, it, u: u.ctrl == it.ctrl and u.uid != it.src))
    if fd and not friends(g, o.ctrl)[1:]:
        _q(g, o, "Elnuk Herd (recycle)", lambda g_, it: _me(g_, it) is not None and g_.recycle_cards(it.ctrl, [_me(g_, it)]))


card("Elnuk Herd", hidden=True, on_play=_elnuk_play)


# Primordial Roar — "[Show Off] a unit. If you showed off a unit, deal damage equal to that unit's Might to a unit."
# (the Might as it was shown, rule 832.1.d.1)
def _roar(g, it):
    sh = it.data.get("shown")
    u = g.legal(it, 0)
    if sh is not None and u is not None and sh["might"] > 0:
        g.deal(u, sh["might"], "spell", it.ctrl)


card("Primordial Roar", preds=[P_unit], resolve=_roar,
     show_off=lambda g, pid, c, x: x.spec["type"] == "Unit",
     show_key=lambda g, x: -(g.might(x) if x in g.board else (x.spec["might"] or 0)),
     choices=lambda g, pid, ctx: tg_choices(_minus_targets(g, pid, ctx["hidden_bf"])))


# Stalwart Goatherd — "[Shield 2] When you play me, you may kill a gear with Energy cost [4] or less."
card("Stalwart Goatherd", kw={"Shield": 2}, on_play=lambda g, o, ctx: _q(
    g, o, "Stalwart Goatherd", lambda g_, it: g_.legal(it, 0) is not None and g_.kill([g_.legal(it, 0)], it.ctrl),
    may=True, choose=trig_target(
        lambda g_, it: sorted([x for x in g_.gear() if x.spec["e"] <= 4 and x.ctrl != it.ctrl],
                              key=lambda x: (-x.spec["e"], x.uid)),
        lambda g_, it, x: g_.is_gear(x) and x.spec["e"] <= 4)))


# Cataclysm — "Opponents can't play spells this turn. Move a friendly unit to an occupied enemy battlefield."
def _cataclysm(g, it):
    g.effects.append(dict(kind="no_play", pid=it.ctrl, dur="turn",
                          fn=lambda g_, eff, pid, c, zone: pid != eff["pid"] and c.spec["type"] == "Spell"))
    u = g.legal(it, 0)
    d = it.data.get("dest")
    if u is not None and d is not None and u.loc != d and g.bfs[d].ctrl == 1 - it.ctrl and g.units(loc=d):
        g.move([u], d, it.ctrl)


def _cataclysm_choices(g, pid, ctx):
    ds = [b.idx for b in g.bfs if b.ctrl == 1 - pid and g.units(loc=b.idx)]
    out = [dict(tg=(u.uid,), dest=d) for u in friends(g, pid, False, ctx["hidden_bf"]) for d in ds if u.loc != d]
    return out


card("Cataclysm", preds=[P_friend], resolve=_cataclysm, choices=_cataclysm_choices)


# Graves, Blasting Through — "When I attack or defend, deal damage equal to my Might to all enemy units here. I don't
# deal combat damage."
def _graves_res(g, it):
    me = _me(g, it)
    if me is None:
        return
    n = g.might(me)
    for u in [u for u in g.units(1 - it.ctrl) if u.loc == me.loc]:
        g.deal(u, n, "ability", it.ctrl)


card("Graves, Blasting Through", no_combat_damage=lambda g, o: True,
     on_event=lambda g, o, ev, info: ev in ("attack", "defend") and info["obj"] is o and _q(
         g, o, "Graves, Blasting Through", _graves_res))


# K'Sante, Courageous — "[Accelerate] [Shield 2] When I hold, channel 1 rune."
card("K'Sante, Courageous", accelerate=True, kw={"Shield": 2}, on_event=lambda g, o, ev, info: (
    ev == "hold" and info["pid"] == o.ctrl and o in info["units"]
    and _q(g, o, "K'Sante, Courageous", lambda g_, it: g_.channel(it.ctrl, 1))))


# Portable Portal — "[Deploy] When you play this, you may pay [Body] to ready a unit here. Friendly units here have
# [Ganking]."
def _portal_play(g, o, ctx):
    here = o.loc

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.ready_obj(u)
    _q(g, o, "Portable Portal", res, may=True, cost=may_pay(0, lambda g_, it: [frozenset({"Body"})]),
       choose=trig_target(lambda g_, it: [u for u in friends(g_, it.ctrl) if u.loc == here and u.exhausted]
                          + [u for u in all_units(g_, it.ctrl) if u.loc == here and u.ctrl != it.ctrl],
                          lambda g_, it, u: u.loc == here))


card("Portable Portal", kw={"Deploy": 1}, on_play=_portal_play,
     aura_kw=lambda g, src, o: {"Ganking": 1} if src.zone == "board" and _here_friend(g, src, o) else None)


# Staunch Survivor is keyword-only ([Assault 2], [Shield 2]): cards.auto_keywords.

# Titanic Shrine — "When you play a unit with Might greater than each other unit on the board, exhaust me to draw 1."
def _shrine(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and info["card"].spec["type"] == "Unit" and info["card"] in g.board:
        u = info["card"]
        if all(g.might(u) > g.might(x) for x in g.units() if x is not u):
            def cost(g_, it):
                me = _me(g_, it)
                if me is None or me.exhausted:
                    return False
                me.exhausted = True
                return True
            _q(g, o, "Titanic Shrine", lambda g_, it: g_.draw(it.ctrl, 1), cost=cost)


card("Titanic Shrine", on_event=_shrine)


# Evelynn, Consuming — "[Ambush] When you play a card from face down, ready me and give me +1 might this turn."
def _eve_consuming(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and _played_card(info) and _from_facedown(info):
        def res(g_, it):
            me = _me(g_, it)
            if me is not None:
                g_.ready_obj(me)
                g_.mod(me, 1)
        _q(g, o, "Evelynn, Consuming", res)


card("Evelynn, Consuming", ambush=True, on_event=_eve_consuming)


# Jarvan IV, Martial Paragon — "Your gear with [Deploy] have [Action]. When I conquer, if there is a friendly gear
# with [Deploy] here, channel 1 rune exhausted."
card("Jarvan IV, Martial Paragon",
     card_kw=lambda g, src, pid, c, kw, zone: (kw == "Action" and src.zone == "board" and pid == src.ctrl
                                               and c.spec["type"] == "Gear" and has_deploy(g, c)),
     on_event=lambda g, o, ev, info: (
         ev == "conquer" and info["pid"] == o.ctrl and o in info["units"]
         and any(x.ctrl == o.ctrl and x.loc == o.loc and has_deploy(g, x) for x in g.gear())
         and _q(g, o, "Jarvan IV, Martial Paragon", lambda g_, it: g_.channel(it.ctrl, 1, exhausted=True))))


# Seraphic Wyvern — "If you would channel runes, channel that many runes plus 1 exhausted rune instead."
card("Seraphic Wyvern", channel_rep=lambda g, src, pid: 1 if src.zone == "board" and pid == src.ctrl else 0)


# ====================================================================== Chaos
# Disposal Expert — "I enter ready if you control no other units. [Disarm 2]"
card("Disposal Expert", kw={"Disarm": 2}, enter_ready=lambda g, pid, c, ch: not any(u is not c for u in g.units(pid)))


# Explosimian — "When I hold, play three Bomb gear tokens to a single battlefield."
def _three_bombs(g, it):
    loc = _bf_ask(g, it.ctrl, "bomb_location")
    for _ in range(3):
        _bomb(g, it.ctrl, loc)


card("Explosimian", on_event=lambda g, o, ev, info: (
    ev == "hold" and info["pid"] == o.ctrl and o in info["units"] and _q(g, o, "Explosimian", _three_bombs)))


# Momentary Companion — "[Hidden] When you play me, if it's not your turn, give me [Temporary]."
card("Momentary Companion", hidden=True, on_play=lambda g, o, ctx: g.tp != o.ctrl and _q(
    g, o, "Momentary Companion", lambda g_, it: _me(g_, it) is not None and g_.grant(_me(g_, it), "Temporary", 1, None)))


# Satchel Charge — "[Action] Move a friendly unit, then play a Bomb gear token to a battlefield."
def _satchel(g, it):
    u = g.legal(it, 0)
    if u is not None and it.data.get("dest") is not None and u.loc != it.data["dest"]:
        g.move([u], it.data["dest"], it.ctrl)
    _bomb(g, it.ctrl)


def _move_choices(us):
    return [dict(tg=(u.uid,), dest=d) for u in us for d in ("base", 0, 1) if d != u.loc]


card("Satchel Charge", timing="action", preds=[P_friend], resolve=_satchel,
     choices=lambda g, pid, ctx: _move_choices(friends(g, pid, False, ctx["hidden_bf"])),
     max_choices=12)


# Siren of the Shallows is keyword-only ([Disarm] [Ganking]): cards.auto_keywords.

# Siren's Song — "[Reaction] Move an enemy unit that moved this turn."
def _moved(g, u):
    return (u.uid, u.oid) in g.hist["moved"]


card("Siren's Song", timing="reaction", preds=[lambda g, it, o: P_enemy(g, it, o) and _moved(g, o)],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.legal(it, 0).loc != it.data["dest"] and g.move(
         [g.legal(it, 0)], it.data["dest"], it.ctrl),
     choices=lambda g, pid, ctx: sorted(_move_choices([u for u in enemies(g, pid, False, ctx["hidden_bf"]) if _moved(g, u)]),
                                        key=lambda ch: ch["dest"] != "base"), max_choices=10)


# Suave Negotiator — "When you play me, choose an opponent. You and that player each draw 1."
card("Suave Negotiator", on_play=lambda g, o, ctx: _q(
    g, o, "Suave Negotiator", lambda g_, it: (g_.draw(it.ctrl, 1), g_.draw(1 - it.ctrl, 1))))


# The Waters' Wage — "[Hidden] Play a Gold gear token exhausted. If it's your turn and you played this from face down,
# play three ready Gold gear tokens instead."
def _wage(g, it):
    if g.tp == it.ctrl and it.data.get("from") == "facedown":
        for _ in range(3):
            _gold(g, it.ctrl, ready=True)
    else:
        _gold(g, it.ctrl)


card("The Waters' Wage", hidden=True, resolve=_wage)


# An Offer You Can't Refuse — "[Action] Kill a gear. Its controller plays a Gold gear token exhausted."
def _offer(g, it):
    x = g.legal(it, 0)
    if x is None:
        return
    c = x.ctrl
    g.kill([x], it.ctrl)
    _gold(g, c)


card("An Offer You Can't Refuse", timing="action", preds=[P_gear], resolve=_offer,
     choices=lambda g, pid, ctx: tg_choices(sorted([x for x in g.gear() if g.targetable(x, pid)
                                                     and (ctx["hidden_bf"] is None or x.loc == ctx["hidden_bf"])],
                                                    key=lambda x: (x.ctrl == pid, -x.spec["e"], x.uid))))


# Demon Shade — "[Hidden] Choose a friendly unit, then choose one — Move it to an open battlefield. Give it [Ganking]
# this turn if there are no open battlefields."
def _shade(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    if it.data["mode"] == "move":
        d = it.data["dest"]
        if open_bf(g, g.bfs[d]) and u.loc != d:
            g.move([u], d, it.ctrl)
    elif not any(open_bf(g, b) for b in g.bfs):
        g.grant(u, "Ganking", 1, "turn")


def _shade_choices(g, pid, ctx):
    out = []
    for u in friends(g, pid, False, ctx["hidden_bf"]):
        out += [dict(tg=(u.uid,), mode="move", dest=b.idx) for b in g.bfs if open_bf(g, b) and u.loc != b.idx]
        out.append(dict(tg=(u.uid,), mode="ganking"))
    return out


card("Demon Shade", hidden=True, preds=[P_friend], resolve=_shade, choices=_shade_choices, max_choices=10)


# Gutter Guardian — "[Hidden] [Show Off] a card with [Hidden]. When you play me, if you showed off, [Add] [A]."
def _has_hidden(g, x):
    im = g.impl(x)
    return im is not None and (im.hidden or g.has_kw(x, "Hidden")) and not x.token


card("Gutter Guardian", hidden=True, show_off=lambda g, pid, c, x: _has_hidden(g, x),
     on_play=lambda g, o, ctx: ctx.get("show") is not None and g.add_pool(o.ctrl, 0, ["A"]))


# Pitiless Stalker — "[E]: [Stun] an enemy unit."
card("Pitiless Stalker", abilities=[ability(
    "Stun", None, exhaust=True, preds=[P_enemy],
    resolve=lambda g, it: g.legal(it, 0) is not None and g.stun(g.legal(it, 0), it.ctrl),
    choices=lambda g, pid, o: tg_choices([u for u in enemies(g, pid) if not u.stunned or full_choices(g, pid)]))])


# Reckless Innovator — "[Accelerate] When I become exhausted, choose one — Play a Gold gear token exhausted. Pay
# [Chaos] to draw 1."
def _innovator(g, o, ev, info):
    if ev != "exhausted" or o not in info["objs"]:
        return

    def choose(g_, it):
        opts = [Opt(tr("jouer un jeton Gold épuisé", "play an exhausted Gold token"), "gold")]
        if g_.can_pay(it.ctrl, 0, [frozenset({"Chaos"})]):
            opts.insert(0, Opt(tr("payer [Chaos] pour piocher 1", "pay [Chaos] to draw 1"), "draw"))
        it.data["mode"] = g_.ask(it.ctrl, "innovator_mode", opts, item=it).value
        return True

    def res(g_, it):
        if it.data["mode"] == "gold":
            _gold(g_, it.ctrl)
        elif g_.pay(it.ctrl, 0, [frozenset({"Chaos"})]):
            g_.draw(it.ctrl, 1)
    _q(g, o, "Reckless Innovator", res, choose=choose)


card("Reckless Innovator", accelerate=True, on_event=_innovator)


# The Good Stuff — "[Deploy] When you play this, discard 1, then draw 3. When you hold here, return this to its
# owner's hand."
card("The Good Stuff", kw={"Deploy": 1},
     on_play=lambda g, o, ctx: _q(g, o, "The Good Stuff", lambda g_, it: (_discard(g_, it.ctrl, 1), g_.draw(it.ctrl, 3))),
     on_event=lambda g, o, ev, info: ev == "hold" and info["pid"] == o.ctrl and info["bf"] == o.loc and _q(
         g, o, "The Good Stuff (hold)", lambda g_, it: _me(g_, it) is not None and g_.to_zone(_me(g_, it), "hand")))


# Bedeviling Stranger — "[Hidden] When you play me, choose a battlefield. If there are 4 or more units there, you may
# return an enemy unit there to its owner's hand."
def _stranger(g, o, ctx):
    def choose(g_, it):
        bfs = sorted((0, 1), key=lambda b: -len(g_.units(loc=b)))
        it.data["bf"] = g_.ask(it.ctrl, "stranger_bf", bfs, item=it)
        return True

    def res(g_, it):
        b = it.data["bf"]
        if len(g_.units(loc=b)) < 4:
            return
        cs = [u for u in enemies(g_, it.ctrl) if u.loc == b and _deflect_ok(g_, it.ctrl, u)]
        if not cs:
            return
        u = g_.ask(it.ctrl, "stranger_return", cs + [None], item=it)
        if u is not None and pay_deflect(g_, it, u):
            g_.to_zone(u, "hand")
    _q(g, o, "Bedeviling Stranger", res, choose=choose)


card("Bedeviling Stranger", hidden=True, on_play=_stranger)


# Bushwhack Trap — "[Deploy] While you or an ally control this battlefield, that player may hide an additional card
# here." (1v1: no allies)
card("Bushwhack Trap", kw={"Deploy": 1}, extra_hide_slots=lambda g, src, b: 1 if (
    src.zone == "board" and src.loc == b.idx and b.ctrl == src.ctrl) else 0)


# Evelynn, In Control — "[Disarm] When I hold, if you control a facedown card here, score 1 point."
card("Evelynn, In Control", kw={"Disarm": 1}, on_event=lambda g, o, ev, info: (
    ev == "hold" and info["pid"] == o.ctrl and o in info["units"] and _q(
        g, o, "Evelynn, In Control", lambda g_, it: _me(g_, it) is not None and any(
            c.owner == it.ctrl for c in g_.bfs[_me(g_, it).loc].facedowns if _me(g_, it).loc in (0, 1))
        and g_.gain_point(it.ctrl, "Evelynn, In Control"))))


# Lulu, Whimsical — "When I move, you may have another unit's base Might become 4 this turn."
def _lulu(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.effects.append(dict(kind="base_might", uid=u.uid, oid=u.oid, value=4, dur="turn"))


card("Lulu, Whimsical", on_event=lambda g, o, ev, info: ev == "move" and info["obj"] is o and _q(
    g, o, "Lulu, Whimsical", _lulu, may=True, choose=trig_target(
        lambda g_, it: sorted([u for u in g_.units() if u.uid != it.src and g_.targetable(u, it.ctrl)],
                              key=lambda u: ((u.spec["might"] or 0) - 4 if u.ctrl == it.ctrl else 4 - (u.spec["might"] or 0), u.uid)),
        lambda g_, it, u: u.uid != it.src)))


# Mechanical Bandit — "At the start of your Beginning Phase, each player reveals the top card of their Main Deck. If
# you revealed a card with the lowest Energy cost, draw it. Recycle the rest." (a tie for lowest: yours is a card
# with the lowest Energy cost)
def _bandit(g, it):
    shown = {}
    for pid in (it.ctrl, 1 - it.ctrl):
        top = g.reveal(pid, 1)
        if top:
            shown[pid] = top[0]
    mine = shown.get(it.ctrl)
    if mine is not None and all(mine.spec["e"] <= c.spec["e"] for c in shown.values()):
        g.draw_card(it.ctrl, mine)
        shown.pop(it.ctrl)
    for pid, c in shown.items():
        if c in g.p[pid].deck:
            g.p[pid].deck.remove(c)
            g.recycle_cards(pid, [c])


card("Mechanical Bandit", on_event=lambda g, o, ev, info: ev == "beginning_start" and info["pid"] == o.ctrl and _q(
    g, o, "Mechanical Bandit", _bandit))


# Ziggs, Explosive — "[1], [E]: Play a Bomb gear token to a battlefield. When I defend, you may kill a friendly gear to
# give me +2 might this turn."
def _ziggs_def(g, o, ev, info):
    if ev == "defend" and info["obj"] is o and g.gear(o.ctrl):
        def cost(g_, it):
            xs = sorted(g_.gear(it.ctrl), key=lambda x: (value(g_, x), x.uid))
            if not xs:
                return False
            x = g_.ask(it.ctrl, "sacrifice_gear", xs)
            return bool(g_.kill([x], it.ctrl, cost=True))
        _q(g, o, "Ziggs, Explosive", lambda g_, it: _me(g_, it) is not None and g_.mod(_me(g_, it), 2), may=True,
           cost=cost)


card("Ziggs, Explosive", on_event=_ziggs_def,
     abilities=[ability("Bomb", "1 energy", exhaust=True, resolve=lambda g, it: _bomb(g, it.ctrl))])


# Angler Wyrm — "[Deflect] [E]: Choose an opponent. They must move a unit they control to my battlefield."
def _wyrm(g, it):
    me = g.obj(it.src)
    if me is None or me.loc not in (0, 1):
        return
    opp = 1 - it.ctrl
    us = [u for u in g.units(opp) if u.loc != me.loc and g.can_move(u, me.loc, opp)]
    if us:
        u = g.ask(opp, "wyrm_move", sorted(us, key=lambda u: (value(g, u), u.uid)))
        g.move([u], me.loc, opp)


card("Angler Wyrm", kw={"Deflect": 1}, abilities=[ability(
    "Lure", None, exhaust=True, resolve=_wyrm, can=lambda g, pid, o: o.loc in (0, 1))])


# Riven, Deserter — "When another friendly unit becomes exhausted, give me +1 might this turn. Exhaust a friendly unit:
# Ready a unit. Use this ability only once each turn."
def _deserter(g, o, ev, info):
    if ev == "exhausted":
        for x in info["objs"]:
            if x is not o and x.ctrl == o.ctrl and x.spec["type"] == "Unit":
                _q(g, o, "Riven, Deserter", lambda g_, it: _me(g_, it) is not None and g_.mod(_me(g_, it), 1))


def _deserter_choices(g, pid, o):
    ex = [u for u in g.units(pid) if u.exhausted]
    out = []
    for x in cap(g, _exh_cost_choices(g, pid), 2, pid):
        for t in ex + ([u for u in g.units() if not u.exhausted] if full_choices(g, pid) else []):
            if t is not x:
                out.append(dict(exh=x.uid, tg=(t.uid,)))
    return out


def _deserter_extra(g, pid, o, ch):
    g.stats[("deserter", o.uid, o.oid, g.turn_no)] = 1
    _pay_exh(g, pid, None, ch)


card("Riven, Deserter", on_event=_deserter, abilities=[ability(
    "Ready", None, preds=[P_unit], choices=_deserter_choices, extra_cost=_deserter_extra,
    can=lambda g, pid, o: not g.stats[("deserter", o.uid, o.oid, g.turn_no)],
    resolve=lambda g, it: g.legal(it, 0) is not None and g.ready_obj(g.legal(it, 0)))])


# Transpositionist — "[Chaos], [E]: Return a friendly unit to its owner's hand. You may play a unit with less Might
# than that unit had, ignoring its cost." (the unit is played from your hand; its Might as it left the board)
def _transpo(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    m = g.might(u)
    g.to_zone(u, "hand")
    cs = [c for c in g.p[it.ctrl].hand if c.spec["type"] == "Unit" and (c.spec["might"] or 0) < m]
    if not cs:
        return
    c = g.ask(it.ctrl, "transpo_pick", sorted(cs, key=lambda c: (-_cval(c), c.uid)) + [None])
    if c is not None:
        _play_from(g, it.ctrl, c, "hand", _free_choices(g, it.ctrl, c, "hand"))


card("Transpositionist", abilities=[ability(
    "Swap", "1 chaos rune", exhaust=True, preds=[P_friend], resolve=_transpo,
    choices=lambda g, pid, o: tg_choices(sorted(g.units(pid), key=lambda u: (-g.might(u), u.uid))))])


# ====================================================================== Order
# Concert Poster — "[Deploy] When you play this or when you conquer here, you may pay [1] to play a 1 might Recruit
# unit token to your base."
def _poster(g, o):
    _q(g, o, "Concert Poster", lambda g_, it: make_token(g_, "Recruit", it.ctrl, "base"), may=True, cost=may_pay(1))


card("Concert Poster", kw={"Deploy": 1}, on_play=lambda g, o, ctx: _poster(g, o),
     on_event=lambda g, o, ev, info: ev == "conquer" and info["pid"] == o.ctrl and info["bf"] == o.loc and _poster(g, o))


# For Demacia! — "[Reaction] Choose a battlefield. Give friendly units there [Shield 2] this turn."
def _for_demacia(g, it):
    for u in g.units(it.ctrl):
        if u.loc == it.data["bf"]:
            g.grant(u, "Shield", 2, "turn")


card("For Demacia!", timing="reaction", resolve=_for_demacia,
     choices=lambda g, pid, ctx: sorted([dict(bf=b) for b in (0, 1) if ctx["hidden_bf"] is None or b == ctx["hidden_bf"]],
                                        key=lambda ch: -len([u for u in g.units(pid) if u.loc == ch["bf"]])))


# Herald of the Emperor — "When you play me, choose a friendly unit. Its base Might becomes 5 this turn."
def _herald_emp(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.effects.append(dict(kind="base_might", uid=u.uid, oid=u.oid, value=5, dur="turn"))


card("Herald of the Emperor", on_play=lambda g, o, ctx: _q(g, o, "Herald of the Emperor", _herald_emp, choose=trig_target(
    lambda g_, it: sorted(friends(g_, it.ctrl), key=lambda u: ((u.spec["might"] or 0), u.uid)), P_friend)))


# Hunting Dog — "[Deathknell] Play a 1 might Bird unit token with [Deflect] to your base."
card("Hunting Dog", deathknell=lambda g, it: make_token(g, "Bird", it.ctrl, "base"))


# Meticulous Restorationist — "When you play a card with Energy cost [7] or more, you may play me from your trash for
# [Order]." (an alternative cost, rule 356.1.a, as a triggered ability of the card in the trash)
def _resto_zone(g, c, ev, info):
    if ev == "played" and c.zone == "trash" and info["pid"] == c.owner and _played_card(info) \
            and info["card"].spec["e"] >= 7:
        def res(g_, it):
            x = next((y for y in g_.p[it.ctrl].trash if y.uid == it.data["uid"]), None)
            if x is not None:
                play_unit_free(g_, it.ctrl, x, "trash", alt_cost=(0, [frozenset({"Order"})]))
        g.queue_trigger(c.owner, "Meticulous Restorationist (trash)", res, dict(uid=c.uid), may=True,
                        cost=None)


card("Meticulous Restorationist", zone_event=_resto_zone)


# One-Two Punch — "[Action] [Buff] a friendly unit and [Stun] an enemy unit."
def _one_two(g, it):
    f, e = g.legal(it, 0), g.legal(it, 1)
    if f is not None:
        g.buff(f)
    if e is not None:
        g.stun(e, it.ctrl)


card("One-Two Punch", timing="action", preds=[P_friend, P_enemy], resolve=_one_two,
     choices=lambda g, pid, ctx: [dict(tg=(f.uid, e.uid))
                                  for f in cap(g, [u for u in friends(g, pid, False, ctx["hidden_bf"]) if u.buff == 0]
                                               + [u for u in friends(g, pid, False, ctx["hidden_bf"]) if u.buff], 3, pid)
                                  for e in cap(g, enemies(g, pid, False, ctx["hidden_bf"]), 3, pid)])


# Party Crasher — "I cost [1] less for each exhausted unit you control. [Tank]"
card("Party Crasher", kw={"Tank": 1}, cost_mod=lambda g, pid, c, ch: (sum(1 for u in g.units(pid) if u.exhausted), 0))


# Protector's Shield — "[Quick-Draw] [Equip] [1]" ; Effect Text and Might Bonus read on the card image: [Tank], +1.
card("Protector's Shield", timing="reaction", quickdraw=True, equip="1 energy", bonus=1, equip_kw={"Tank": 1})


# Burn Away — "Banish a unit with 3 might or less or a gear. [Burn 2]."
def _burn_away_pred(g, it, o):
    return g.is_gear(o) or (o.spec["type"] == "Unit" and g.might(o) <= 3)


def _burn_away(g, it):
    o = g.legal(it, 0)
    if o is not None:
        g.to_zone(o, "banish", by=it.ctrl)
    g.burn(it.ctrl, 2)


card("Burn Away", preds=[_burn_away_pred], resolve=_burn_away,
     choices=lambda g, pid, ctx: tg_choices(sorted([o for o in g.board if g.targetable(o, pid) and _burn_away_pred(g, None, o)
                                                    and (ctx["hidden_bf"] is None or o.loc == ctx["hidden_bf"])],
                                                   key=lambda o: (o.ctrl == pid, -value(g, o), o.uid))))


# Dune Devourer — "You may kill a friendly unit as an additional cost to play me. If you do, I cost [2] less.
# [Deflect]"
card("Dune Devourer", kw={"Deflect": 1},
     cost_mod=lambda g, pid, c, ch: (2, 0) if ch.get("kill") is not None else (0, 0),
     as_played=lambda g, pid, c: [dict()] + [dict(kill=o.uid) for o in _kill_cost_opts(g, pid, gear=False)])


# Mournful Spirit — "[Deathknell] You may kill a unit or gear here." (where it died; chosen as the Deathknell is
# finalized)
def _spirit_choose(g, it):
    loc = it.data["info"]["loc"]
    cs = [o for o in g.board if o.loc == loc and (o.spec["type"] == "Unit" or g.is_gear(o))
          and g.targetable(o, it.ctrl) and _deflect_ok(g, it.ctrl, o)]
    if not cs:
        return False
    cs.sort(key=lambda o: (o.ctrl == it.ctrl, -value(g, o), o.uid))
    o = g.ask(it.ctrl, "spirit_kill", cs + [None], item=it)
    if o is None or not pay_deflect(g, it, o):
        return False
    g.add_target(it, o, lambda g_, it_, x: x.loc == it_.data["info"]["loc"])
    return True


card("Mournful Spirit", deathknell=lambda g, it: g.legal(it, 0) is not None and g.kill([g.legal(it, 0)], it.ctrl),
     dk_choose=_spirit_choose)


# Rousing Display — "[Show Off] a card with Energy cost [7] or more. Play two 1 might Recruit unit tokens to your base.
# If you showed off, play four to your base instead."
def _rousing(g, it):
    for _ in range(4 if it.data.get("shown") else 2):
        make_token(g, "Recruit", it.ctrl, "base")


card("Rousing Display", resolve=_rousing, show_off=lambda g, pid, c, x: not x.token and x.spec["e"] >= 7)


# Steadfast Outrider — "[Tank] While there are 2 or more other friendly units and/or gear at my battlefield, I have +2
# might."
card("Steadfast Outrider", kw={"Tank": 1}, might_mod=lambda g, o: 2 if o.loc in (0, 1) and sum(
    1 for x in g.board if x is not o and x.ctrl == o.ctrl and x.loc == o.loc
    and (x.spec["type"] == "Unit" or g.is_gear(x))) >= 2 else 0)


# Trifarian Captain — "When you play me, look at the top 4 cards of your Main Deck. You may reveal from among them a
# unit or gear with Energy cost no more than [3] and with Power cost no more than [A] and draw it. Recycle the rest."
def _captain(g, it):
    pl = g.p[it.ctrl]
    top = g.look(it.ctrl, pl.deck[:4])
    cs = [c for c in top if c.spec["type"] in ("Unit", "Gear") and c.spec["e"] <= 3 and c.spec["p"] <= 1]
    pick = g.ask(it.ctrl, "captain_pick", sorted(cs, key=lambda c: (-_cval(c), c.uid)) + [None]) if cs else None
    if pick is not None:
        g.log(f"  P{it.ctrl} reveals {pick.cname}")
        g.draw_card(it.ctrl, pick)
    rest = [c for c in top if c is not pick and c in pl.deck]
    for c in rest:
        pl.deck.remove(c)
    g.recycle_cards(it.ctrl, rest)


card("Trifarian Captain", on_play=lambda g, o, ctx: _q(g, o, "Trifarian Captain", _captain))


# Battlefield Armorer — "When a friendly gear at a battlefield dies, you may pay [1] to return it to its owner's hand."
# (a token stops existing, rule 186.1: nothing to return)
def _armorer(g, o, ev, info):
    if ev == "die":
        inf = info["info"]
        if inf["ctrl"] == o.ctrl and inf["spec"]["type"] == "Gear" and inf["loc"] in (0, 1) and not inf["token"]:
            c = inf["obj"]

            def res(g_, it):
                x = next((y for pl in g_.p for y in pl.trash if y.uid == it.data["uid"] and y.oid == it.data["oid"]), None)
                if x is not None:
                    g_.to_zone(x, "hand")
            _q(g, o, "Battlefield Armorer", res, dict(uid=c.uid, oid=c.oid), may=True, cost=may_pay(1))


card("Battlefield Armorer", on_event=_armorer)


# Demacian Illuminators — "When one or more friendly units die, [Buff] a friendly unit." (once per kill event)
def _illum(g, o, ev, info):
    if ev == "die":
        inf = info["info"]
        key = ("illum", o.uid, o.oid, inf.get("batch"))
        if inf["ctrl"] == o.ctrl and inf["spec"]["type"] == "Unit" and o in g.board and not g.stats[key]:
            g.stats[key] = 1
            _q(g, o, "Demacian Illuminators", lambda g_, it: g_.legal(it, 0) is not None and g_.buff(g_.legal(it, 0)),
               choose=trig_target(lambda g_, it: [u for u in friends(g_, it.ctrl) if u.buff == 0]
                                  + [u for u in friends(g_, it.ctrl) if u.buff], P_friend))


card("Demacian Illuminators", on_event=_illum)


# Heimerdinger, Academy Instructor — "If you would play a unit token to a location while I'm at a battlefield, you may
# play a 3 might Mech unit token to that same location instead."
card("Heimerdinger, Academy Instructor", token_swap=lambda g, src, pid, name, loc: "Mech" if (
    src.zone == "board" and src.loc in (0, 1) and pid == src.ctrl and name != "Mech") else None)


# Jarvan IV, Royal Heir — "[Assault] [Tank] When I attack, you may move a unit you control to my battlefield."
def _jarvan_rh(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None and u.loc != here:
                g_.move([u], here, it.ctrl)
        _q(g, o, "Jarvan IV, Royal Heir", res, may=True, choose=trig_target(
            lambda g_, it: [u for u in friends(g_, it.ctrl) if u.loc != here and g_.can_move(u, here, it.ctrl)],
            lambda g_, it, u: u.ctrl == it.ctrl))


card("Jarvan IV, Royal Heir", kw={"Assault": 1, "Tank": 1}, on_event=_jarvan_rh)


# Raucous Anthem — "Ready your units and give them +3 might this turn."
def _anthem(g, it):
    for u in g.units(it.ctrl):
        g.ready_obj(u)
        g.mod(u, 3)


card("Raucous Anthem", resolve=_anthem)


# Remember the Fallen — "Play up to 3 units and/or gear from your trash, each with Energy cost no more than [3] and
# Power cost no more than [A], ignoring their costs."
def _remember(g, it):
    pid = it.ctrl
    for _ in range(3):
        cs = [c for c in g.p[pid].trash if c.spec["type"] in ("Unit", "Gear") and c.spec["e"] <= 3
              and c.spec["p"] <= 1 and g.impl(c) is not None and _free_choices(g, pid, c, "trash")]
        if not cs:
            return
        c = g.ask(pid, "remember_pick", sorted(cs, key=lambda c: (-_cval(c), c.uid)) + [None])
        if c is None:
            return
        _play_from(g, pid, c, "trash", _free_choices(g, pid, c, "trash"))


card("Remember the Fallen", resolve=_remember)


# Dockside Lock-Up — "[Deploy] When you play this, banish a unit here. When you hold here, kill this to play the
# banished card, ignoring its cost. (Tokens aren't cards.)"
def _lockup_play(g, o, ctx):
    here = o.loc

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is None:
            return
        tok = u.token
        g_.to_zone(u, "banish", by=it.ctrl)
        me = _me(g_, it)
        if me is not None and not tok:
            g_.effects.append(dict(kind="lockup", src=(me.uid, me.oid), card=u.uid, coid=u.oid))
    _q(g, o, "Dockside Lock-Up", res, choose=trig_target(
        lambda g_, it: sorted([u for u in g_.units() if u.loc == here], key=lambda u: (u.ctrl == it.ctrl, -value(g_, u), u.uid)),
        lambda g_, it, u: u.loc == here))


def _lockup_hold(g, o, ev, info):
    if ev == "hold" and info["pid"] == o.ctrl and info["bf"] == o.loc:
        def cost(g_, it):
            me = _me(g_, it)
            return me is not None and bool(g_.kill([me], it.ctrl, cost=True))

        def res(g_, it):
            ef = it.data.get("lock")
            if ef is None:
                return
            c = next((y for pl in g_.p for y in pl.banish if y.uid == ef["card"] and y.oid == ef["coid"]), None)
            if c is not None:
                _play_from(g_, it.ctrl, c, "banish", _free_choices(g_, it.ctrl, c, "banish"))
        lock = next((dict(e) for e in g.effects if e.get("kind") == "lockup" and e["src"] == (o.uid, o.oid)), None)
        _q(g, o, "Dockside Lock-Up (hold)", res, dict(lock=lock), cost=cost)


card("Dockside Lock-Up", kw={"Deploy": 1}, on_play=_lockup_play, on_event=_lockup_hold)


# Mordekaiser, Dominating — "When you play me or when I attack, you may discard 1 to kill a unit or gear."
def _morde_dom(g, o):
    def cost(g_, it):
        if not g_.p[it.ctrl].hand:
            return False
        _discard(g_, it.ctrl, 1)
        return True
    cost.can = lambda g_, it: bool(g_.p[it.ctrl].hand)
    _q(g, o, "Mordekaiser, Dominating", lambda g_, it: g_.legal(it, 0) is not None and g_.kill([g_.legal(it, 0)], it.ctrl),
       may=True, cost=cost, choose=trig_target(
           lambda g_, it: sorted([x for x in g_.board if x.ctrl != it.ctrl and (x.spec["type"] == "Unit" or g_.is_gear(x))],
                                 key=lambda x: (-value(g_, x), x.uid)),
           lambda g_, it, x: x.spec["type"] == "Unit" or g_.is_gear(x)))


card("Mordekaiser, Dominating", on_play=lambda g, o, ctx: _morde_dom(g, o),
     on_event=lambda g, o, ev, info: ev == "attack" and info["obj"] is o and _morde_dom(g, o))


# Seraphine, Not Alone — "When you play me or when I become exhausted, play a 1 might Recruit unit token to your base.
# I have +1 might for each other exhausted unit you control."
def _sna_trig(g, o):
    _q(g, o, "Seraphine, Not Alone", lambda g_, it: make_token(g_, "Recruit", it.ctrl, "base"))


card("Seraphine, Not Alone", on_play=lambda g, o, ctx: _sna_trig(g, o),
     on_event=lambda g, o, ev, info: ev == "exhausted" and o in info["objs"] and _sna_trig(g, o),
     might_mod=lambda g, o: sum(1 for u in g.units(o.ctrl) if u is not o and u.exhausted))


# ====================================================================== legends and signature cards
def _legend(g, it):
    return g.p[it.ctrl].legend


# Ekko, Boy Who Shattered Time — "[Empower] — [1], [E] (Use only if not Empowered.) Disempower me, [E]: Reduce the cost
# of the next unit you play this turn by [2][A] and give it "Kill me at end of turn.""
def _ekko_kill_end(g, eff, info):
    u = g.obj(eff["uid"])
    if u is not None and u.oid == eff["oid"]:
        g.queue_trigger(u.ctrl, "Kill me at end of turn", lambda g_, it: (
            g_.obj(it.data["uid"]) is not None and g_.obj(it.data["uid"]).oid == it.data["oid"]
            and g_.kill([g_.obj(it.data["uid"])], it.ctrl)), dict(uid=u.uid, oid=u.oid), src=u.uid)
    g.effects.remove(eff)


def _ekko_boy(g, it):
    pid = it.ctrl
    key = ("ekko_boy", it.id)

    def mod(g_, eff, p, what):
        return [dict(e=-2, rm=1)] if what["kind"] == "unit" and what["card"] is not None else []

    def mark(g_, eff, info):
        if info["pid"] == pid and not info.get("token") and info["card"].spec["type"] == "Unit":
            g_.effects = [e for e in g_.effects if e.get("key") != key]
            c = info["card"]
            g_.effects.append(dict(on="end_turn", fn=_ekko_kill_end, uid=c.uid, oid=c.oid, dur="turn"))
    g.effects.append(dict(kind="cost_mod", pid=pid, fn=mod, once=True, key=key, dur="turn"))
    g.effects.append(dict(on="played", fn=mark, key=key, dur="turn"))


card("Ekko, Boy Who Shattered Time", abilities=[
    empower_ability("1 energy", exhaust=True),
    dict(name="Next unit", timing="main", exhaust=True, can=lambda g, pid, o: o.empowered,
         cost=lambda g, pid, o, ch: (0, []), extra_cost=lambda g, pid, o, ch: g.disempower(o), resolve=_ekko_boy)])


# Chronobreak — "As an additional cost to play this, pay any amount of Power. Banish that many units you control, then
# for each, its owner plays that unit back to the same location or to their base, ignoring its cost. (You choose the
# order.)" (the units are chosen as it resolves; a token stops existing once banished, rule 186.1)
def _chronobreak(g, it):
    pid = it.ctrl
    n = it.data.get("x", 0)
    us = list(g.units(pid))
    chosen = []
    for _ in range(min(n, len(us))):
        u = g.ask(pid, "chrono_pick", sorted([u for u in us if u not in chosen], key=lambda u: (u.exhausted is False, u.uid)))
        chosen.append(u)
    back = []
    for u in chosen:
        loc, owner, tok = u.loc, u.owner, u.token
        g.to_zone(u, "banish", by=pid)
        if not tok:
            back.append((u, loc, owner))
    while back:
        u, loc, owner = back.pop(0) if len(back) == 1 else g.ask(pid, "chrono_order", back)
        if (u, loc, owner) in back:
            back.remove((u, loc, owner))
        c = next((x for x in g.p[owner].banish if x.uid == u.uid), None)
        if c is not None:
            locs = ["base"] + ([loc] if loc in (0, 1) else [])
            play_unit_free(g, owner, c, "banish", loc_choices=locs)


card("Chronobreak", resolve=_chronobreak,
     choices=lambda g, pid, ctx: [dict(x=n) for n in range(len(g.units(pid)), -1, -1)],
     extra_cost_fn=lambda g, pid, c, ch: (0, [ANY] * ch.get("x", 0)))


# Ziggs, Hexplosives Expert — "[1][A], [E]: Play a Bomb gear token to a battlefield."
card("Ziggs, Hexplosives Expert", abilities=[ability(
    "Bomb", "1 energy and 1 rune of any type", exhaust=True, resolve=lambda g, it: _bomb(g, it.ctrl))])


# Hexplosive Minefield — "[Action] Kill any number of friendly Bomb tokens. At end of turn, play that many Bomb tokens
# to a single battlefield."
def _minefield(g, it):
    bombs = [g.obj(u) for u in it.data.get("bombs", ())]
    bombs = [b for b in bombs if b is not None and b.ctrl == it.ctrl and b.cname == "Bomb"]
    killed = g.kill(bombs, it.ctrl) if bombs else []
    n = len(killed)
    if n:
        pid = it.ctrl

        def at_end(g_, eff, info):
            g_.effects.remove(eff)

            def res(g2, it2):
                loc = _bf_ask(g2, it2.ctrl, "bomb_location")
                for _ in range(it2.data["n"]):
                    _bomb(g2, it2.ctrl, loc)
            g_.queue_trigger(pid, "Hexplosive Minefield (end of turn)", res, dict(n=eff["n"]))
        g.effects.append(dict(on="end_turn", fn=at_end, n=n, dur="turn"))


def _minefield_choices(g, pid, ctx):
    bs = sorted([o.uid for o in g.board if o.ctrl == pid and o.cname == "Bomb"])
    out = []
    for k in range(len(bs), -1, -1):
        for grp in combinations(bs, k):
            out.append(dict(bombs=grp))
            if not full_choices(g, pid):
                break
    return out


card("Hexplosive Minefield", timing="action", resolve=_minefield, choices=_minefield_choices)


# Mordekaiser, Iron Revenant — "[Reaction][>] Banish a unit or gear from your trash, [E]: [Add] [A]. Spend this Power
# only to play units or gear." (an [Add] used while paying a unit or a gear, rule 429; its other cost on use)
def _iron_can(g, pid, o, ctx):
    return ctx is not None and ctx.get("kind") in ("unit", "gear") and any(
        c.spec["type"] in ("Unit", "Gear") for c in g.p[pid].trash)


def _iron_use(g, pid, o):
    cs = sorted([c for c in g.p[pid].trash if c.spec["type"] in ("Unit", "Gear")], key=lambda c: (_cval(c), c.uid))
    if cs:
        g.to_zone(g.ask(pid, "banish_from_trash", cs) if len(cs) > 1 else cs[0], "banish", by=pid)


card("Mordekaiser, Iron Revenant", add=[dict(e=0, p=["A"], exhaust=True, kill=False, can=_iron_can, on_use=_iron_use)])


# Realm of Death — "Give an enemy unit -2 might this turn and a friendly unit +2 might this turn. They deal damage
# equal to their Mights to each other." (simultaneously, as spell damage of its controller)
def _realm(g, it):
    e, f = g.legal(it, 0), g.legal(it, 1)
    if e is not None:
        g.mod(e, -2)
    if f is not None:
        g.mod(f, 2)
    if e is not None and f is not None:
        me, mf = max(0, g.might(e)), max(0, g.might(f))
        g.deal(e, mf, "spell", it.ctrl)
        g.deal(f, me, "spell", it.ctrl)


card("Realm of Death", preds=[P_enemy, P_friend], resolve=_realm,
     choices=lambda g, pid, ctx: [dict(tg=(e.uid, f.uid)) for e in cap(g, enemies(g, pid), 3, pid)
                                  for f in cap(g, sorted(friends(g, pid), key=lambda u: (-g.might(u), u.uid)), 3, pid)])


# Orianna, Lady of Clockwork — "At the start of your Main Phase, you may pay [1] to choose a friendly gear. It becomes
# a unit gear with Might equal to its Energy cost until you choose another gear with me." (Obj.ugear: a Unit whose
# Might is its printed Energy cost; an attached Equipment detaches as it becomes a unit, rule 143.5.b)
def _ori_legend(g, pid, ev, info):
    if ev != "main_start" or info["pid"] != pid or not g.gear(pid):
        return

    def choose(g_, it):
        gs = [x for x in g_.gear(it.ctrl) if not x.ugear]
        if not gs:
            return False
        x = g_.ask(it.ctrl, "orianna_gear", sorted(gs, key=lambda x: (-x.spec["e"], x.uid)), item=it)
        g_.add_target(it, x, lambda g2, it2, o: o.ctrl == it2.ctrl and g2.is_gear(o))
        return True

    def res(g_, it):
        x = g_.legal(it, 0)
        if x is None:
            return
        for y in g_.board:
            if y.ugear and y.ctrl == it.ctrl and y.__dict__.get("ugear_by") == it.ctrl:
                y.ugear = False
        if x.attached_to is not None:
            u = g_.obj(x.attached_to)
            if u is not None and x.uid in u.attached:
                u.attached.remove(x.uid)
            x.attached_to = None
        x.ugear = True
        x.ugear_by = it.ctrl
        g_.need_cleanup = True
        g_.log(f"  {x.cname} becomes a unit gear")
    g.queue_trigger(pid, "Orianna, Lady of Clockwork", res, may=True, cost=may_pay(1), choose=choose)


card("Orianna, Lady of Clockwork", legend_event=_ori_legend)


# The Ball — "[Deploy] [Ganking] When you play or move this, choose one — Draw 1. Deal 2 to enemy units here."
def _ball_trig(g, o):
    def choose(g_, it):
        me = g_.obj(it.src)
        n = len([u for u in g_.units(1 - it.ctrl) if me is not None and u.loc == me.loc and u.loc in (0, 1)])
        opts = [Opt(tr("piocher 1", "draw 1"), "draw"), Opt(tr("2 dégâts aux ennemis ici", "deal 2 to enemy units here"), "deal")]
        if n >= 2:
            opts.reverse()
        it.data["mode"] = g_.ask(it.ctrl, "ball_mode", opts, item=it).value
        return True

    def res(g_, it):
        me = g_.obj(it.src)
        if it.data["mode"] == "draw":
            g_.draw(it.ctrl, 1)
        elif me is not None and me.loc in (0, 1):
            for u in [u for u in g_.units(1 - it.ctrl) if u.loc == me.loc]:
                g_.deal(u, 2, "ability", it.ctrl)
    _q(g, o, "The Ball", res, choose=choose)


card("The Ball", kw={"Deploy": 1, "Ganking": 1}, on_play=lambda g, o, ctx: _ball_trig(g, o),
     on_event=lambda g, o, ev, info: ev == "move" and info["obj"] is o and _ball_trig(g, o))


# K'Sante, Pride of Nazumah — "[1], [E]: Give a friendly unit "I have [Assault] equal to my [Shield]" this turn."
# (a granted text: kept as a marker grant read by the legend's aura)
_KS_GUARD = set()


def _ksante_aura(g, src, o):
    if not o.grants or o.uid in _KS_GUARD or not any(k == "KSanteAssault" for k, _, _ in o.grants):
        return None
    _KS_GUARD.add(o.uid)
    try:
        n = g.kw_value(o, "Shield")
    finally:
        _KS_GUARD.discard(o.uid)
    return {"Assault": n} if n else None


card("K'Sante, Pride of Nazumah", aura_kw=_ksante_aura, abilities=[ability(
    "Assault = Shield", "1 energy", exhaust=True, preds=[P_friend],
    resolve=lambda g, it: g.legal(it, 0) is not None and g.grant(g.legal(it, 0), "KSanteAssault", 1, "turn"),
    choices=lambda g, pid, o: tg_choices(sorted(friends(g, pid), key=lambda u: (-g.kw_value(u, "Shield"), u.uid))))])


# Ntofo Strikes — "[Action] Give a friendly unit [Shield 2] this turn, then deal damage equal to its [Shield] to an
# enemy unit." (its Shield value after the grant)
def _ntofo(g, it):
    f, e = g.legal(it, 0), g.legal(it, 1)
    if f is None:
        return
    g.grant(f, "Shield", 2, "turn")
    if e is not None:
        g.deal(e, g.kw_value(f, "Shield"), "spell", it.ctrl)


card("Ntofo Strikes", timing="action", preds=[P_friend, P_enemy], resolve=_ntofo,
     choices=lambda g, pid, ctx: [dict(tg=(f.uid, e.uid))
                                  for f in cap(g, sorted(friends(g, pid, False, ctx["hidden_bf"]),
                                                         key=lambda u: (-g.kw_value(u, "Shield"), u.uid)), 3, pid)
                                  for e in cap(g, enemies(g, pid, False, ctx["hidden_bf"]), 3, pid)])


# Riven, The Exile — "Units you play to battlefields you control enter ready." (card.played_to: where it is played)
card("Riven, The Exile", units_enter_ready=lambda g, src, pid, c: (
    pid == src.owner and getattr(src, "zone", None) == "legend" and c.spec["type"] == "Unit"
    and getattr(c, "played_to", None) in (0, 1) and g.bfs[c.played_to].ctrl == pid))


# Blade of the Exile — "[Equip] — [A] or exhaust a friendly unit" ; Effect Text and Might Bonus read on the card:
# [Ganking], +3 might.
def _blade_choices(g, pid, o):
    out = []
    for u in friends(g, pid):
        if u.uid == o.attached_to:
            continue
        out.append(dict(tg=(u.uid,)))
        for x in cap(g, _exh_cost_choices(g, pid), 2, pid):
            out.append(dict(tg=(u.uid,), exh=x.uid))
    return out


_BLADE_EQUIP = dict(name="Equip", timing="main", choices=_blade_choices, preds=[P_friend],
                    cost=lambda g, pid, o, ch: (0, []) if ch.get("exh") is not None else (0, [ANY]),
                    extra_cost=lambda g, pid, o, ch: _pay_exh(g, pid, None, ch),
                    resolve=lambda g, it: (g.legal(it, 0) is not None and g.obj(it.src) is not None
                                           and attach(g, g.obj(it.src), g.legal(it, 0))))
card("Blade of the Exile", abilities=[_BLADE_EQUIP], bonus=3, equip_kw={"Ganking": 1})


# Seraphine, Starry-Eyed Songstress — "As an additional cost to play a spell, you may exhaust me and three friendly
# units to reduce the cost of that spell by [2]."
def _sera_opts(g, src, pid, card):
    if getattr(src, "zone", None) != "legend" or src.owner != pid or src.exhausted or card.spec["type"] != "Spell":
        return []
    us = _exh_cost_choices(g, pid)
    if len(us) < 3:
        return []
    groups = list(combinations(us, 3)) if full_choices(g, pid) else [tuple(us[:3])]
    return [dict(sera=tuple(u.uid for u in grp)) for grp in groups]


def _sera_pay(g, src, pid, card, ch):
    if ch.get("sera") and getattr(src, "zone", None) == "legend" and src.owner == pid:
        objs = [src] + [g.obj(u) for u in ch["sera"] if g.obj(u) is not None]
        for o in objs:
            o.exhausted = True                        # at the same time (one 'exhausted' event)


card("Seraphine, Starry-Eyed Songstress", spell_opts=_sera_opts, opt_pay=_sera_pay,
     cost_aura=lambda g, src, pid, what: [dict(e=-2)] if (
         getattr(src, "zone", None) == "legend" and src.owner == pid and what["kind"] == "spell"
         and (what.get("choice") or {}).get("sera")) else [])


# Encore — "Choose two — Deal 2 to a unit. Play two 1 might Recruit unit tokens. Give up to two units each +1 might
# this turn. Look at the top 2 cards of your Main Deck, draw 1 and recycle the other."
def _encore(g, it):
    modes = it.data.get("modes", ())
    i = 0
    if "deal" in modes:
        u = g.legal(it, i)
        i += 1
        if u is not None:
            g.deal(u, 2, "spell", it.ctrl)
    if "recruits" in modes:
        for _ in range(2):
            _unit_token(g, it.ctrl, "Recruit")
    if "plus" in modes:
        for j in range(i, len(it.targets)):
            u = g.legal(it, j)
            if u is not None:
                g.mod(u, 1)
    if "look" in modes:
        pl = g.p[it.ctrl]
        top = g.look(it.ctrl, pl.deck[:2])
        if top:
            c = g.ask(it.ctrl, "draw_pick", sorted(top, key=lambda c: (-_cval(c), c.uid))) if len(top) > 1 else top[0]
            g.draw_card(it.ctrl, c)
            rest = [x for x in top if x is not c and x in pl.deck]
            for x in rest:
                pl.deck.remove(x)
            g.recycle_cards(it.ctrl, rest)


def _encore_choices(g, pid, ctx):
    deal = [(u.uid,) for u in cap(g, _minus_targets(g, pid), 3, pid)]
    fr = cap(g, friends(g, pid), 3, pid)
    plus = [tuple(u.uid for u in grp) for k in (2, 1) for grp in combinations(fr, k)][:(None if full_choices(g, pid) else 2)]
    plus = plus + [()]
    if full_choices(g, pid):
        alls = sorted(g.units(), key=lambda u: u.uid)
        plus = [tuple(u.uid for u in grp) for k in (2, 1, 0) for grp in combinations(alls, k)]
    out = []
    for a, b in combinations(("deal", "recruits", "plus", "look"), 2):
        tgs_a = deal if a == "deal" else [()]
        tgs_b = plus if b == "plus" else [()]
        if a == "plus":
            tgs_a, tgs_b = plus, [()]
        for ta in tgs_a:
            for tb in tgs_b:
                tg = ta + tb if a == "deal" else (ta if a == "plus" else tb)
                if a == "deal" and b == "plus" and set(ta) & set(tb) and not full_choices(g, pid):
                    continue
                out.append(dict(modes=(a, b), tg=tg))
    return out


card("Encore", preds=[P_unit], resolve=_encore, choices=_encore_choices, max_choices=16)


# Evelynn, Agony's Embrace — "Once each turn, when you play a card from face down, draw 1. Then, if it's not your
# turn, discard 1."
def _eve_legend(g, pid, ev, info):
    if ev == "played" and info["pid"] == pid and _played_card(info) and _from_facedown(info):
        key = ("eve_agony", pid, g.turn_no)
        if g.stats[key]:
            return
        g.stats[key] = 1

        def res(g_, it):
            g_.draw(it.ctrl, 1)
            if g_.tp != it.ctrl:
                _discard(g_, it.ctrl, 1)
        g.queue_trigger(pid, "Evelynn, Agony's Embrace", res)


card("Evelynn, Agony's Embrace", legend_event=_eve_legend)


# Last Caress — "[Reaction] The next unit you play this turn (even from face down) can be played to any battlefield.
# When you play it, [Buff] it."
def _caress(g, it):
    pid = it.ctrl
    key = ("caress", it.id)

    def locs(g_, eff, card, closed):
        return [0, 1] if card.spec["type"] == "Unit" else []

    def fire(g_, eff, info):
        if info["pid"] == pid and not info.get("token") and info["card"].spec["type"] == "Unit":
            g_.effects = [e for e in g_.effects if e.get("key") != key]
            c = info["card"]
            g_.queue_trigger(pid, "Last Caress", lambda g2, it2: (
                g2.obj(it2.data["uid"]) is not None and g2.buff(g2.obj(it2.data["uid"]))), dict(uid=c.uid), src=c.uid)
    g.effects.append(dict(kind="aura_locs", pid=pid, fn=locs, key=key, dur="turn"))
    g.effects.append(dict(on="played", fn=fire, key=key, dur="turn"))


card("Last Caress", timing="reaction", resolve=_caress)


# Jarvan IV, Exemplar of Demacia — "When you play a gear with [Deploy], you may exhaust me and pay [A] to move a
# friendly unit to that battlefield."
def _jarvan_ex(g, pid, ev, info):
    if ev != "played" or info["pid"] != pid or not g.is_gear(info["card"]) or not has_deploy(g, info["card"]):
        return
    x = info["card"]
    if x.loc not in (0, 1):
        return
    dest = x.loc

    def cost(g_, it):
        leg = g_.p[it.ctrl].legend
        if leg.exhausted or not g_.can_pay(it.ctrl, 0, [ANY]):
            return False
        leg.exhausted = True
        return g_.pay(it.ctrl, 0, [ANY])
    cost.can = lambda g_, it: not g_.p[it.ctrl].legend.exhausted and g_.can_pay(it.ctrl, 0, [ANY])

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None and u.loc != dest:
            g_.move([u], dest, it.ctrl)
    g.queue_trigger(pid, "Jarvan IV, Exemplar of Demacia", res, may=True, cost=cost, choose=trig_target(
        lambda g_, it: [u for u in friends(g_, it.ctrl) if u.loc != dest], lambda g_, it, u: u.ctrl == it.ctrl))


card("Jarvan IV, Exemplar of Demacia", legend_event=_jarvan_ex)


# Demacian Standard — "[Deploy] When one or more friendly units attack here, give one of them +2 might this turn. When
# you hold here, you may return this to its owner's hand."
def _standard(g, o, ev, info):
    if ev == "attack" and info["obj"].ctrl == o.ctrl and info["bf"] == o.loc and g.sd is not None:
        key = ("standard", o.uid, o.oid, g.turn_no, g.stats["combats"])
        if g.stats[key]:
            return
        g.stats[key] = 1
        here = o.loc
        _q(g, o, "Demacian Standard", lambda g_, it: _mod_target(g_, it, 2), choose=trig_target(
            lambda g_, it: sorted([u for u in g_.units(it.ctrl) if u.loc == here and u.desig == "att"],
                                  key=lambda u: (-g_.might(u), u.uid)),
            lambda g_, it, u: u.ctrl == it.ctrl and u.loc == here))
    elif ev == "hold" and info["pid"] == o.ctrl and info["bf"] == o.loc:
        _q(g, o, "Demacian Standard (hold)", lambda g_, it: _me(g_, it) is not None and g_.to_zone(_me(g_, it), "hand"),
           may=True)


card("Demacian Standard", kw={"Deploy": 1}, on_event=_standard)


# ====================================================================== battlefields
# Bandle Scouts' Academy — "When you hide a card here, play a Gold gear token exhausted."
card("Bandle Scouts' Academy", bf_event=lambda g, b, ev, info: ev == "hide" and info["bf"] == b.idx and g.queue_trigger(
    info["pid"], "Bandle Scouts' Academy", lambda g_, it: _gold(g_, it.ctrl)))


# Black Market — "[A][A]: Draw 1. Each player may use this ability once on their turn (even if they don't control this
# battlefield)." (an ability of the battlefield each player can activate: given to each player's legend)
def _market_ab(b):
    key = lambda g, pid: ("black_market", b.idx, pid, g.turn_no)

    def extra(g, pid, o, ch):
        g.stats[key(g, pid)] = 1
    return ability("Black Market: draw 1", "2 runes of any type", resolve=lambda g, it: g.draw(it.ctrl, 1),
                   can=lambda g, pid, o: g.tp == pid and not g.stats[key(g, pid)], extra_cost=extra)


_MARKET_ABS = {}


def _market_grant(g, b, o):
    if getattr(o, "zone", None) != "legend":
        return []
    if b.idx not in _MARKET_ABS:
        _MARKET_ABS[b.idx] = _market_ab(b)
    return [_MARKET_ABS[b.idx]]


card("Black Market", grant_abilities=_market_grant)


# Black Rose Sanctum — "For each player, the first time each turn a non-token unit they control dies here, they play a
# 1 might Recruit unit token to their base."
def _sanctum(g, b, ev, info):
    if ev == "die":
        inf = info["info"]
        if inf["loc"] == b.idx and inf["spec"]["type"] == "Unit" and not inf["token"]:
            key = ("rose_sanctum", b.idx, inf["ctrl"], g.turn_no)
            if not g.stats[key]:
                g.stats[key] = 1
                g.queue_trigger(inf["ctrl"], "Black Rose Sanctum", lambda g_, it: make_token(g_, "Recruit", it.ctrl, "base"))


card("Black Rose Sanctum", bf_event=_sanctum)


# Cosmic Vista — "When you hold here, if your Rune Deck is empty, you score 1 point."
card("Cosmic Vista", bf_event=lambda g, b, ev, info: ev == "hold" and info["bf"] == b.idx and g.queue_trigger(
    info["pid"], "Cosmic Vista", lambda g_, it: not g_.p[it.ctrl].rune_deck and g_.gain_point(it.ctrl, "Cosmic Vista")))


# Durand Memorial — "When you defend here, choose a unit you control here. Give it [Tank] this combat."
def _durand(g, b, ev, info):
    if ev == "defend" and info["bf"] == b.idx and g.sd is not None:
        pid = info["obj"].ctrl
        key = ("durand", b.idx, pid, g.stats["combats"], g.turn_no)
        if g.stats[key]:
            return
        g.stats[key] = 1

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is None:
                return
            ent = ["Tank", 1, "combat"]
            u.grants.append(ent)

            def end(g2, eff, inf):
                g2.effects.remove(eff)
                x = g2.obj(eff["uid"])
                if x is not None and ent in x.grants:
                    x.grants.remove(ent)
            g_.effects.append(dict(on="combat_end", fn=end, uid=u.uid))
        g.queue_trigger(pid, "Durand Memorial", res, choose=trig_target(
            lambda g_, it: sorted([u for u in g_.units(it.ctrl) if u.loc == b.idx], key=lambda u: (-g_.might(u), u.uid)),
            lambda g_, it, u: u.ctrl == it.ctrl and u.loc == b.idx))


card("Durand Memorial", bf_event=_durand)


# Firelights' Hideout — "When you conquer here, the next unit you play this turn has [Accelerate]."
def _hideout(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx:
        pid = info["pid"]

        def res(g_, it):
            key = ("hideout", it.id)

            def fire(g2, eff, inf):
                if inf["pid"] == pid and not inf.get("token") and inf["card"].spec["type"] == "Unit":
                    g2.effects = [e for e in g2.effects if e.get("key") != key]
            g_.effects.append(dict(kind="card_kw", pid=pid, key=key, dur="turn",
                                   fn=lambda g2, eff, c, kw, src: kw == "Accelerate" and c.spec["type"] == "Unit"))
            g_.effects.append(dict(on="played", fn=fire, key=key, dur="turn"))
        g.queue_trigger(pid, "Firelights' Hideout", res)


card("Firelights' Hideout", bf_event=_hideout)


# Hunters' Circle — "For each player, the first unit or gear they play here on their turn costs [1] less."
def _circle_key(g, b, pid):
    return ("hunters_circle", b.idx, pid, g.turn_no)


def _circle_aura(g, b, pid, what):
    ch = what.get("choice") or {}
    if what["kind"] in ("unit", "gear") and what["card"] is not None and g.tp == pid and ch.get("loc") == b.idx \
            and what.get("src") != "facedown" and not g.stats[_circle_key(g, b, pid)]:
        return [dict(e=-1)]
    return []


def _circle_event(g, b, ev, info):
    if ev == "played" and info["pid"] == g.tp and info["card"].spec["type"] in ("Unit", "Gear") \
            and info["card"].zone == "board" and info["card"].loc == b.idx:
        g.stats[_circle_key(g, b, info["pid"])] = 1


card("Hunters' Circle", cost_aura=_circle_aura, bf_event=_circle_event)


# Packed Amphitheater — "Champion units here have +1 might."
card("Packed Amphitheater", aura_might=lambda g, b, o: 1 if (
    o.spec["type"] == "Unit" and o.loc == b.idx and o.spec["super"] == "Champion") else 0)


# Rakelstake — "For each player, the first time they play a unit or gear here each turn, they [Predict]."
def _rakel(g, b, ev, info):
    if ev == "played" and info["card"].spec["type"] in ("Unit", "Gear") and info["card"].zone == "board" \
            and info["card"].loc == b.idx:
        key = ("rakelstake", b.idx, info["pid"], g.turn_no)
        if not g.stats[key]:
            g.stats[key] = 1
            g.queue_trigger(info["pid"], "Rakelstake", lambda g_, it: g_.predict(it.ctrl, 1))


card("Rakelstake", bf_event=_rakel)


# Ruined Monastery — "When you conquer here, you may discard 1 and channel 1 rune exhausted."
card("Ruined Monastery", bf_event=lambda g, b, ev, info: ev == "conquer" and info["bf"] == b.idx and g.queue_trigger(
    info["pid"], "Ruined Monastery", lambda g_, it: (_discard(g_, it.ctrl, 1), g_.channel(it.ctrl, 1, exhausted=True)),
    may=True))


# Neeko, Blending In: not modelled (several legends in the Legend Zone, choices at deck building): NEEDS_radiance.md.

# French titles of this module's questions (train.py); English ones in lang.ASK_EN.
ask_text(
    bomb_location="Sur quel champ de bataille jouer le jeton Bomb ?", bomb_target="Bomb : quelle unité ennemie touche-t-elle ?",
    tracker_targets="Quelles unités (jusqu'à 2) subissent 1 dégât ?", amp_kill="Quelle autre carte alliée détruire pour piocher 1 ?",
    morde_kill="Quelle unité alliée détruire pour sauver Mordekaiser ?", riven_mode="Riven : quel effet choisir ?",
    gear_pick="Quel équipement révéler et piocher ?", scope_guess="Devine le type de la carte du dessus :",
    innovator_mode="Reckless Innovator : quel effet ?", stranger_bf="Quel champ de bataille choisir ?",
    stranger_return="Quelle unité ennemie renvoyer dans la main de son propriétaire ?",
    wyrm_move="Quelle unité déplacer vers le champ de bataille d'Angler Wyrm ?",
    transpo_pick="Quelle unité de ta main jouer gratuitement ?", spirit_kill="Quelle unité ou quel équipement détruire ?",
    captain_pick="Quelle carte révéler et piocher ?", remember_pick="Quelle carte de ta défausse jouer ?",
    chrono_pick="Quelle unité bannir ?", chrono_order="Quelle unité rejouer maintenant ?",
    orianna_gear="Quel équipement devient une unité ?", ball_mode="The Ball : quel effet ?")


# Staunch Survivor — "[Assault 2], [Shield 2]" (keyword-only, but the comma keeps it out of cards.auto_keywords)
card("Staunch Survivor", kw={"Assault": 2, "Shield": 2})
