"""Batch order: the Order cards listed in batches/order.txt (units, spells, gear). See GUIDE.md.

Cards that need an engine hook are not registered here; they are listed in NEEDS_order.md.
"""
from cards import *  # noqa: F401,F403
from actions import total_cost, card_choices, pay_ctx

ORDER = frozenset({"Order"})
_PETS = ("Bird", "Cat", "Dog", "Poro")


# ====================================================================== private helpers
def _locs(g, pid):
    """Locations where pid can play a unit by default (rule 355.2.a): base, then battlefields they control."""
    return ["base"] + [b.idx for b in g.bfs if b.ctrl == pid]


def _tokens(g, pid, name, n, loc=None, ready=False):
    """Play n unit/gear tokens. loc=None: the location of each token is chosen as it is played (rule 355.2,
    the tokens may go to different locations); gear tokens go to the base."""
    out = []
    for _ in range(n):
        l = loc
        if l is None:
            l = "base" if SPEC[name]["type"] == "Gear" else g.ask(pid, "token_location", _locs(g, pid), token=name)
        out.append(make_token(g, name, pid, l, ready=ready))
    return out


def _here(g, it, default):
    """Location of the source of a trigger if it is still on the board, else the remembered location."""
    o = g.obj(it.src) if it.src is not None else None
    return o.loc if o is not None else default


def _is_pet(spec):
    return any(t in spec["tags"] for t in _PETS)


def _me(g, it):
    return g.obj(it.src) if it.src is not None else None


def _trash_card(g, pid, uid, oid):
    for c in g.p[pid].trash:
        if c.uid == uid and c.oid == oid:
            return c
    return None


def _pairs(us, n=4):
    us = us[:n]
    return [dict(tg=(a.uid, b.uid)) for i, a in enumerate(us) for b in us[i + 1:]]


def _choose_more(g, pid, kind, cands, n, key):
    """pid picks up to n objects among cands one by one (best first for the AI). Returns the picked ones."""
    picked = []
    cands = sorted(cands, key=key)
    while cands and len(picked) < n:
        c = g.ask(pid, kind, list(cands), n=n - len(picked)) if len(cands) > 1 else cands[0]
        picked.append(c)
        cands.remove(c)
    return picked


# ====================================================================== cards
# Albus Ferros — "When you play me, spend any number of buffs. For each buff spent, channel 1 rune exhausted."
# (rule 702.2.b: a player spends buffs only from units they control, one buff per unit at most.)
def _albus_res(g, it):
    pid = it.ctrl
    n = 0
    while True:
        us = sorted([u for u in g.units(pid) if u.buff > 0], key=lambda u: value(g, u))
        if not us:
            break
        u = g.ask(pid, "spend_buff", us + [None], reason="Albus Ferros")
        if u is None:
            break
        u.buff = 0
        n += 1
    if n:
        g.channel(pid, n, exhausted=True)


card("Albus Ferros", on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Albus Ferros", _albus_res, src=o.uid))


# Altar of Memories — "When a friendly unit dies, you may exhaust me to draw 1, then put a card from your hand on
# the top or bottom of your Main Deck."
def _altar_event(g, o, ev, info):
    if ev == "die" and info["info"]["ctrl"] == o.ctrl and info["info"]["spec"]["type"] == "Unit":
        def cost(g_, it):                        # cost within the instruction (rule 383.3.b)
            me = _me(g_, it)
            if me is None or me.exhausted:
                return False
            me.exhausted = True
            return True

        def res(g_, it):
            pid = it.ctrl
            g_.draw(pid, 1)
            hand = g_.p[pid].hand
            if not hand:
                return
            c = g_.ask(pid, "altar_card", sorted(hand, key=lambda x: x.spec["e"] + 2 * x.spec["p"]))
            where = g_.ask(pid, "altar_where", ["bottom", "top"], card=c)
            g_.to_zone(c, "deck", bottom=(where == "bottom"))
        g.queue_trigger(o.ctrl, "Altar of Memories", res, src=o.uid, may=True, cost=cost)


card("Altar of Memories", on_event=_altar_event)


# Ambessa, Respected and Feared — "[Empower] [1][Order][Order]. [Empowered] I have [Assault 2]. [Empowered] When I
# attack, kill an enemy unit here with less Might than me."
def _ambessa_event(g, o, ev, info):
    if ev == "attack" and info["obj"] is o and o.empowered:
        here = o.loc

        def ok(g_, it, u):
            me = _me(g_, it)
            m = g_.might(me) if me is not None else it.data["m"]
            return u.spec["type"] == "Unit" and u.ctrl != it.ctrl and u.loc == here and g_.might(u) < m

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.kill([u], it.ctrl)
        g.queue_trigger(o.ctrl, "Ambessa, Respected and Feared", res, dict(m=g.might(o)), src=o.uid,
                        choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if ok(g_, it, u)], ok,
                                           deflect=True))


card("Ambessa, Respected and Feared", empower="1 energy and 2 order runes",
     kw_if=[(when_empowered, {"Assault": 2})], on_event=_ambessa_event)


# Aurok General — "[Empower] [3][Order]. [Empowered] Your units that are [Empowered] have +2 might (including me)."
card("Aurok General", empower="3 energy and 1 order rune",
     aura_might=lambda g, src, o: 2 if (src.empowered and src.spec["type"] == "Unit" and o.ctrl == src.ctrl
                                        and o.spec["type"] == "Unit" and o.empowered) else 0)


# Azir, Sovereign — "[Accelerate] When I attack, you may move any number of your token units to this battlefield."
def _azir_event(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc

        def res(g_, it):
            pid = it.ctrl
            movers = [u for u in g_.units(pid) if u.token and u.loc != here]
            pick = [u for u in movers if g_.ask(pid, "azir_move", [True, False], unit=u)]
            if pick:
                g_.move(pick, here, pid)
        g.queue_trigger(o.ctrl, "Azir, Sovereign", res, src=o.uid)


card("Azir, Sovereign", accelerate=True, on_event=_azir_event)


# B.F. Sword — "[Equip] [Order]" ; Might Bonus +3 (card image).
card("B.F. Sword", equip="1 order rune", bonus=3)


# Back to Back — "[Reaction] Give two friendly units each +2 might this turn." (two targets, rule 355.8)
def _two_mod(n):
    def res(g, it):
        for i in range(2):
            u = g.legal(it, i)
            if u is not None:
                g.mod(u, n)
    return res


card("Back to Back", timing="reaction", preds=[P_friend, P_friend], resolve=_two_mod(2),
     choices=lambda g, pid, ctx: _pairs(friends(g, pid, False, ctx["hidden_bf"])))


# Bandle Soldier — "[Level 3] I enter ready."
card("Bandle Soldier", levels=[(3, dict(ready=True))])


# Blade of the Ruined King — "[Equip] — [Order], Kill a friendly unit" ; Might Bonus +4 (card image).
def _blade_choices(g, pid, o):
    us = [u for u in sorted(friends(g, pid), key=lambda u: -g.might(u)) if u.uid != o.attached_to]
    vs = sorted(g.units(pid), key=lambda x: value(g, x))
    # killing the unit to equip would make the choice illegal (rule 355.16); each target with its cheapest victim
    # first (only the first MAX_CHOICES options are offered)
    pairs = [(u, v) for u in us for v in vs if v is not u]
    first = []
    for u in us:
        p = next((pv for pv in pairs if pv[0] is u), None)
        if p is not None:
            first.append(p)
    rest = [pv for pv in pairs if pv not in first]
    return [dict(tg=(u.uid,), victim=v.uid) for u, v in first + rest]


def _blade_extra(g, pid, o, ch):
    v = g.obj(ch["victim"]) if ch.get("victim") else None
    if v is None:                               # Weaponmaster: the cost is paid on resolution
        tg = ch.get("tg", (None,))[0]
        vs = sorted([u for u in g.units(pid) if u.uid != tg], key=lambda x: value(g, x))
        v = g.ask(pid, "sacrifice", vs) if vs else None
    if v is not None:
        g.kill([v], pid, cost=True)


card("Blade of the Ruined King", bonus=4, abilities=[dict(
    name="Equip", timing="main", choices=_blade_choices, cost=lambda g, pid, o, ch: (0, [ORDER]), preds=[P_friend],
    can=lambda g, pid, o: len(g.units(pid)) >= 2, extra_cost=_blade_extra,
    can_extra=lambda g, pid, o: len(g.units(pid)) >= 2,
    resolve=lambda g, it: (g.legal(it, 0) is not None and g.obj(it.src) is not None
                           and attach(g, g.obj(it.src), g.legal(it, 0))))])


# Blast of Power — "[Action] Kill a unit at a battlefield."
def _kill_tg(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.kill([u], it.ctrl)


card("Blast of Power", timing="action", preds=[P_unit_bf], resolve=_kill_tg,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# Blood Money — "[Action] Kill a unit at a battlefield with 2 might or less. If it was an enemy unit, play a Gold
# gear token exhausted. If it was a friendly unit, play two Gold gear tokens exhausted."
def _p_small(n, base=P_unit_bf):
    return lambda g, it, o: base(g, it, o) and g.might(o) <= n


def _blood_money(g, it):
    u = g.legal(it, 0)
    if u is None:
        return                                   # linked instructions need the target (rule 359.3.e)
    friendly = u.ctrl == it.ctrl
    g.kill([u], it.ctrl)
    _tokens(g, it.ctrl, "Gold", 2 if friendly else 1)


card("Blood Money", timing="action", preds=[_p_small(2)], resolve=_blood_money,
     choices=lambda g, pid, ctx: tg_choices([u for u in all_units(g, pid, True, ctx["hidden_bf"])
                                             if g.might(u) <= 2]))


# Bonds of Strength — "[Reaction] [Repeat] [2] Give two friendly units each +1 might this turn."
card("Bonds of Strength", timing="reaction", repeat=repeat_cost("2 energy"), preds=[P_friend, P_friend],
     resolve=repeatable(_two_mod(1)),
     choices=lambda g, pid, ctx: _pairs(friends(g, pid, False, ctx["hidden_bf"])))


# Call to Glory — "[Reaction] As you play this, you may spend a buff as an additional cost. If you do, ignore this
# spell's cost. Give a unit +3 might this turn."
def _ctg_choices(g, pid, ctx):
    us = friends(g, pid, False, ctx["hidden_bf"]) + enemies(g, pid, False, ctx["hidden_bf"])
    buffed = sorted([u for u in g.units(pid) if u.buff > 0], key=lambda u: value(g, u))
    out = []
    for u in us:
        if buffed:
            out.append(dict(tg=(u.uid,), free=True, spend=buffed[0].uid))
    out += tg_choices(us)
    return out


def _ctg_pay(g, pid, card, ch):
    if ch.get("spend"):
        b = g.obj(ch["spend"])
        if b is not None and b.ctrl == pid and b.buff > 0:
            b.buff = 0


card("Call to Glory", timing="reaction", preds=[P_unit], pay_extra=_ctg_pay, choices=_ctg_choices,
     resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 3))


# Carrion Dredger — "[Deathknell] Play a 1 might Bird unit token with [Deflect] to your base."
card("Carrion Dredger", deathknell=lambda g, it: make_token(g, "Bird", it.ctrl, "base"))


# Commander Ledros — "As you play me, you may kill any number of friendly units as an additional cost. Reduce my
# cost by [Order] for each killed this way. [Deflect] [Ganking]"
def _ledros_as_played(g, pid, c):
    us = sorted(g.units(pid), key=lambda u: value(g, u))
    out = [dict()] + [dict(kills=tuple(u.uid for u in us[:k])) for k in range(1, min(4, len(us)) + 1)]
    out += [dict(kills=(u.uid,)) for u in us[1:6]]          # any single unit (prefixes: the cheapest ones)
    return out


def _ledros_pay(g, pid, c, ch):
    vs = [g.obj(u) for u in ch.get("kills", ())]
    vs = [v for v in vs if v is not None and v.ctrl == pid]
    if vs:
        g.kill(vs, pid, cost=True)


card("Commander Ledros", kw={"Deflect": 1, "Ganking": 1}, as_played=_ledros_as_played, pay_extra=_ledros_pay,
     cost_mod=lambda g, pid, c, ch: (0, len(ch.get("kills", ()))))


# Corina Veraza — "[Accelerate] When I move to a battlefield, play three 1 might Recruit unit tokens here."
def _move_tokens(name, n, token="Recruit"):
    def ev_fn(g, o, ev, info):
        if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
            to = info["to"]
            g.queue_trigger(o.ctrl, name, lambda g_, it: _tokens(g_, it.ctrl, token, n, _here(g_, it, to)), src=o.uid)
    return ev_fn


card("Corina Veraza", accelerate=True, on_event=_move_tokens("Corina Veraza", 3))


# Crimson Pigeons — "I have +2 might while I'm attacking with another unit."
card("Crimson Pigeons",
     might_mod=lambda g, o: 2 if o.desig == "att" and any(u is not o and u.desig == "att"
                                                          for u in g.units(o.ctrl, o.loc)) else 0)


# Cruel Patron — "As an additional cost to play me, kill a friendly unit."
card("Cruel Patron", as_played=lambda g, pid, c: [dict(kill=u.uid) for u in sorted(g.units(pid),
                                                                                     key=lambda u: value(g, u))])


# Darius, Executioner — "[Legion] — When you play me, ready me. Other friendly units have +1 might here."
def _other_here_aura(g, src, o):
    return 1 if (src.spec["type"] == "Unit" and o is not src and o.ctrl == src.ctrl and o.spec["type"] == "Unit"
                 and o.loc == src.loc) else 0


def _darius_play(g, o, ctx):
    if g.legion(o.ctrl, o):
        g.queue_trigger(o.ctrl, "Darius, Executioner", lambda g_, it: _me(g_, it) is not None and
                        g_.ready_obj(_me(g_, it)), src=o.uid)


card("Darius, Executioner", on_play=_darius_play, aura_might=_other_here_aura)


# Disciple of Shen — "[Hidden] I have [Shield 3] while I'm at a battlefield with exactly one other unit you control."
card("Disciple of Shen", hidden=True,
     kw_if=[(lambda g, o: o.loc in (0, 1) and len([u for u in g.units(o.ctrl, o.loc) if u is not o]) == 1,
             {"Shield": 3})])


# Divine Judgment — "Each player chooses 2 units, 2 gear, 2 runes, and 2 cards in their hands. Recycle the rest."
# Not targeted (rule 355.10.e): the choices are made on resolution, starting with the turn player.
def _divine(g, it):
    keep = {}
    for pid in (g.tp, 1 - g.tp):
        pl = g.p[pid]
        ku = _choose_more(g, pid, "judgment_unit", g.units(pid), 2, lambda u: -value(g, u))
        kg = _choose_more(g, pid, "judgment_gear", g.gear(pid), 2, lambda x: -(x.spec["e"] + 2 * x.spec["p"]))
        kr = _choose_more(g, pid, "judgment_rune", list(pl.runes), 2, lambda r: (r.exhausted, r.domain))
        kh = _choose_more(g, pid, "judgment_hand", list(pl.hand), 2, lambda c: -(c.spec["e"] + 2 * c.spec["p"]))
        keep[pid] = (ku, kg, kr, kh)
    for pid in (g.tp, 1 - g.tp):
        ku, kg, kr, kh = keep[pid]
        pl = g.p[pid]
        perms = [o for o in g.units(pid) if o not in ku] + [o for o in g.gear(pid) if o not in kg]
        cards_ = perms + [c for c in pl.hand if c not in kh]
        for r in [r for r in pl.runes if r not in kr]:
            g.recycle_rune(pid, r)
        if cards_:
            g.recycle_cards(pid, cards_)


card("Divine Judgment", resolve=_divine)


# Divining Shells — "[Vision] [Action] Kill this, [E]: Give a unit +2 might this turn."
card("Divining Shells", kw={"Vision": 1}, abilities=[ability(
    "Shells", timing="action", exhaust=True, kill_self=True, preds=[P_unit],
    choices=lambda g, pid, o: tg_choices(friends(g, pid)),
    resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 2))])


# Drag Under — "[Action] I cost [2] less to play from anywhere other than your hand. Kill a unit at a battlefield."
card("Drag Under", timing="action", preds=[P_unit_bf], resolve=_kill_tg,
     cost_mod=lambda g, pid, c, ch: (2, 0) if c.zone != "hand" else (0, 0),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# Dragon Form — "Choose a unit. Its base Might becomes 5 this turn. [Flow] [3]"
# The new base Might is applied as a 'this turn' modifier equal to the difference with the current base Might
# (printed Might plus earlier Dragon Forms on the same object this turn), so the total Might is the same as with
# a base of 5 (rule 477.3: Might = base + increases + decreases).
def _dragon_base(g, o):
    b = o.spec["might"] or 0
    for ef in g.effects:
        if ef.get("kind") == "order_base_might" and ef["uid"] == o.uid and ef["oid"] == o.oid:
            b += ef["delta"]
    return b


def _dragon_form(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    d = 5 - _dragon_base(g, u)
    g.effects.append(dict(kind="order_base_might", pid=it.ctrl, uid=u.uid, oid=u.oid, delta=d, dur="turn"))
    if d:
        u.mods.append([d, "turn"])


def _dragon_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    fr = sorted([u for u in friends(g, pid, False, hb) if _dragon_base(g, u) < 5], key=lambda u: _dragon_base(g, u))
    en = sorted([u for u in enemies(g, pid, False, hb) if _dragon_base(g, u) > 5], key=lambda u: -_dragon_base(g, u))
    rest = [u for u in all_units(g, pid, False, hb) if u not in fr and u not in en]
    return tg_choices(fr + en + rest)


card("Dragon Form", flow=flow_cost("3 energy"), preds=[P_unit], resolve=_dragon_form, choices=_dragon_choices)


# Eminent Benefactor — "When I hold, play two Gold gear tokens exhausted."
card("Eminent Benefactor", on_event=lambda g, o, ev, info: ev == "hold" and o in info["units"] and g.queue_trigger(
    o.ctrl, "Eminent Benefactor", lambda g_, it: _tokens(g_, it.ctrl, "Gold", 2), src=o.uid))


# Enthralling Protector — "[Hunt] Spend 2 XP: [Buff] me."
card("Enthralling Protector", kw={"Hunt": 1}, abilities=[ability(
    "Buff", xp=2, resolve=lambda g, it: _me(g, it) is not None and g.buff(_me(g, it)))])


# Escaped Grayback — "[Empower] — Kill a friendly unit. [Empowered] I have +2 might."
def _grayback_empower(g, it):
    me = _me(g, it)
    if me is not None:
        g.empower(me)


card("Escaped Grayback", might_if=[(when_empowered, 2)], abilities=[ability(
    "Empower", can=lambda g, pid, o: not o.empowered and any(u is not o for u in g.units(pid)),
    choices=lambda g, pid, o: [dict(victim=u.uid) for u in sorted(g.units(pid), key=lambda u: value(g, u))
                               if u is not o],
    extra_cost=lambda g, pid, o, ch: g.kill([g.obj(ch["victim"])], pid, cost=True),
    resolve=_grayback_empower)])


# Eye of the Herald — "[Equip] [Order]" ; Might Bonus +0 and Effect Text (card image): "When I move, play a 1 might
# Recruit unit token here."
def _eye_effect(g, gear, unit, ev, info):
    if ev == "move" and info["obj"] is unit:
        to = info["to"]
        g.queue_trigger(unit.ctrl, "Eye of the Herald", lambda g_, it: _tokens(g_, it.ctrl, "Recruit", 1,
                                                                               _here(g_, it, to)), src=unit.uid)


card("Eye of the Herald", equip="1 order rune", bonus=0, effect_event=_eye_effect)


# Facebreaker — "[Hidden] [Action] Stun a friendly unit and an enemy unit at the same battlefield."
def _facebreaker_choices(g, pid, ctx):
    out = []
    for e in enemies(g, pid, True, ctx["hidden_bf"]):
        for f in sorted(g.units(pid, e.loc), key=lambda u: value(g, u)):
            out.append(dict(tg=(f.uid, e.uid)))
    return out


def _facebreaker(g, it):
    for i in range(2):
        u = g.legal(it, i)
        if u is not None:
            g.stun(u, it.ctrl)


card("Facebreaker", timing="action", hidden=True, preds=[P_friend_bf, P_enemy_bf], resolve=_facebreaker,
     choices=_facebreaker_choices)


# Faithful Manufactor — "When you play me, play a 1 might Recruit unit token here."
def _play_tokens_here(name, n, token):
    def on_play(g, o, ctx):
        here = o.loc
        g.queue_trigger(o.ctrl, name, lambda g_, it: _tokens(g_, it.ctrl, token, n, _here(g_, it, here)), src=o.uid)
    return on_play


card("Faithful Manufactor", on_play=_play_tokens_here("Faithful Manufactor", 1, "Recruit"))


# Fiora, Victorious — "While I'm [Mighty], I have [Deflect], [Ganking], and [Shield]."
card("Fiora, Victorious", kw_if=[(when_mighty, {"Deflect": 1, "Ganking": 1, "Shield": 1})])


# Fiora, Worthy — "When a unit you control becomes [Mighty], you may pay [Order] to ready it."
def _fiora_worthy(g, o, ev, info):
    if ev == "becomes_mighty" and info["obj"].ctrl == o.ctrl:
        u = info["obj"]

        def res(g_, it):
            x = g_.obj(it.data["uid"])
            if x is not None and x.oid == it.data["oid"]:
                g_.ready_obj(x)
        g.queue_trigger(o.ctrl, "Fiora, Worthy", res, dict(uid=u.uid, oid=u.oid), src=o.uid, may=True,
                        cost=may_pay(0, lambda g_, it: [ORDER]))


card("Fiora, Worthy", track_mighty=True, on_event=_fiora_worthy)


# Forge of the Future — "When you play this, play a 1 might Recruit unit token at your base. Kill this: Recycle up
# to 4 cards from trashes."
def _forge_res(g, it):
    pid = it.ctrl
    picked = []
    while len(picked) < 4:
        cands = [c for p in (1 - pid, pid) for c in g.p[p].trash if c not in picked]
        if not cands:
            break
        c = g.ask(pid, "forge_recycle", cands + [None], n=4 - len(picked))
        if c is None:
            break
        picked.append(c)
    if picked:
        g.recycle_cards(pid, picked)              # each card goes to the bottom of its owner's Main Deck


card("Forge of the Future",
     on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Forge of the Future",
                                               lambda g_, it: _tokens(g_, it.ctrl, "Recruit", 1, "base"), src=o.uid),
     abilities=[ability("Recycle", kill_self=True, resolve=_forge_res)])


# Garen, Commander — "Other friendly units have +1 might here."
card("Garen, Commander", aura_might=_other_here_aura)


# Glowstone — "[Empower] [A][A]. Disempower this, [E]: Choose a player. They gain control of this and recall it.
# At the end of your turn, kill this and deal 5 to all units you control."
def _glow_give(g, it):
    me = _me(g, it)
    if me is None:
        return
    p = it.data["player"]
    me.ctrl = p
    g.recall(me)
    g.log(f"  P{p} gains control of {me}")


def _glow_event(g, o, ev, info):
    if ev == "end_turn" and info["pid"] == o.ctrl:
        def res(g_, it):
            me = g_.obj(it.data["uid"])
            if me is not None and me.oid == it.data["oid"]:
                g_.kill([me], it.ctrl)
            for u in list(g_.units(it.ctrl)):
                g_.deal(u, 5, "ability", it.ctrl)
        g.queue_trigger(o.ctrl, "Glowstone", res, dict(uid=o.uid, oid=o.oid), src=o.uid)


card("Glowstone", empower="2 runes of any type", on_event=_glow_event, abilities=[ability(
    "Give", exhaust=True, can=lambda g, pid, o: o.empowered,
    choices=lambda g, pid, o: [dict(player=1 - pid), dict(player=pid)],
    extra_cost=lambda g, pid, o, ch: g.disempower(o), resolve=_glow_give)])


# Grand Strategem — "[Action] Give friendly units +5 might this turn."
def _strategem(g, it):
    for u in g.units(it.ctrl):
        g.mod(u, 5)


card("Grand Strategem", timing="action", resolve=_strategem)


# Guards! — "[Hidden] Play a 2 might Sand Soldier unit token. You may pay [Order] to ready it."
# From Hidden, the token is played at that battlefield (rule 811.1.d.3).
def _guards(g, it):
    pid = it.ctrl
    hb = it.data.get("hidden_bf")
    t = _tokens(g, pid, "Sand Soldier", 1, hb)[0]
    if g.can_pay(pid, 0, [ORDER]) and g.ask(pid, "may", [True, False], item=it, reason="Guards! ready"):
        if g.pay(pid, 0, [ORDER]):
            g.ready_obj(t)


card("Guards!", hidden=True, resolve=_guards)


# Heroic Charge — "[Action] Give a friendly unit +1 might this turn and [Stun] an enemy unit at its location."
def _hc_enemy(g, it, o):
    f = g.obj(it.targets[0][0]) if it.targets else None
    return P_enemy(g, it, o) and f is not None and o.loc == f.loc


def _heroic(g, it):
    f = g.legal(it, 0)
    e = g.legal(it, 1)
    if f is not None:
        g.mod(f, 1)
    if e is not None:
        g.stun(e, it.ctrl)


def _hc_choices(g, pid, ctx):
    out = []
    for e in enemies(g, pid, False, ctx["hidden_bf"]):
        for f in friends(g, pid, False, ctx["hidden_bf"]):
            if f.loc == e.loc:
                out.append(dict(tg=(f.uid, e.uid)))
    out.sort(key=lambda c: g.obj(c["tg"][1]).stunned)
    return out


card("Heroic Charge", timing="action", preds=[P_friend, _hc_enemy], resolve=_heroic, choices=_hc_choices)


# Imperial Decree — "[Action] When any unit takes damage this turn, kill it."
def _decree(g, it):
    pid = it.ctrl

    def on_damage(g_, eff, info):
        u = info["obj"]
        if u.spec["type"] != "Unit":
            return

        def res(g2, it2):
            x = g2.obj(it2.data["uid"])
            if x is not None and x.oid == it2.data["oid"]:
                g2.kill([x], it2.ctrl)
        g_.queue_trigger(eff["pid"], "Imperial Decree", res, dict(uid=u.uid, oid=u.oid))
    g.effects.append(dict(on="damaged", fn=on_damage, dur="turn", pid=pid))


card("Imperial Decree", timing="action", resolve=_decree)


# Karma, Channeler — "[Vision] When you recycle one or more cards, buff a friendly unit." (runes aren't cards:
# recycling a rune emits no 'recycle' event)
def _karma_event(g, o, ev, info):
    if ev == "recycle" and info["pid"] == o.ctrl and info["cards"]:
        g.queue_trigger(o.ctrl, "Karma, Channeler", lambda g_, it: g_.legal(it, 0) is not None and
                        g_.buff(g_.legal(it, 0)), src=o.uid,
                        choose=trig_target(lambda g_, it: sorted(friends(g_, it.ctrl), key=lambda u: u.buff > 0),
                                           P_friend))


card("Karma, Channeler", kw={"Vision": 1}, on_event=_karma_event)


# Kayle, Justified — "[Empower] [3]. I can be [Empowered] up to three times. I have +2 might for each time I'm
# [Empowered]. While I'm [Empowered] three times, I have [Deflect 3] and [Ganking]." (rule 441.1.c.1)
# The number of times is kept in g.effects; it counts only while the Empowered status is on (a Disempower removes
# the status, rule 442.1).
def _kayle_n(g, o):
    if not o.empowered:
        return 0
    for ef in g.effects:
        if ef.get("kind") == "order_kayle" and ef["uid"] == o.uid and ef["oid"] == o.oid:
            return ef["n"]
    return 1


def _kayle_empower(g, it):
    o = _me(g, it)
    if o is None:
        return
    n = _kayle_n(g, o)
    if n >= 3:
        return
    g.effects = [ef for ef in g.effects if not (ef.get("kind") == "order_kayle" and ef["uid"] == o.uid)]
    g.effects.append(dict(kind="order_kayle", pid=it.ctrl, uid=o.uid, oid=o.oid, n=n + 1))
    if n == 0:
        g.empower(o)
    else:
        g.log(f"  {o} is empowered again ({n + 1})")
        g.emit("empowered", obj=o)


card("Kayle, Justified", might_mod=lambda g, o: 2 * _kayle_n(g, o),
     kw_if=[(lambda g, o: _kayle_n(g, o) >= 3, {"Deflect": 3, "Ganking": 1})],
     abilities=[ability("Empower", "3 energy", can=lambda g, pid, o: _kayle_n(g, o) < 3, resolve=_kayle_empower)])


# Keeper of Law — "I cost [2] and [Order] less if you control a battlefield with exactly two units there."
card("Keeper of Law", cost_mod=lambda g, pid, c, ch: (2, 1) if any(
    b.ctrl == pid and len(g.units(loc=b.idx)) == 2 for b in g.bfs) else (0, 0))


# King's Edict — "Starting with the next player, each other player chooses a unit you don't control that hasn't
# been chosen for this spell. Kill those units." (1v1: the opponent chooses; not targeted, rule 355.10.e)
def _edict(g, it):
    pid = it.ctrl
    chosen = []
    for p in (1 - pid,):
        us = [u for u in g.units() if u.ctrl != pid and u not in chosen]
        if us:
            chosen.append(g.ask(p, "sacrifice", sorted(us, key=lambda u: (u.ctrl != p, value(g, u)))))
    g.kill(chosen, pid)


card("King's Edict", resolve=_edict)


# Lacerate — "Choose a unit. If it's [Empowered], disempower it. Then kill it if it has 3 might or less.
# [Flow] [4][Order][Order]"
def _lacerate(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    g.disempower(u)
    if g.might(u) <= 3:
        g.kill([u], it.ctrl)


def _lacerate_choices(g, pid, ctx):
    es = enemies(g, pid, False, ctx["hidden_bf"])
    return tg_choices(sorted(es, key=lambda u: (g.might(u) - (2 if u.empowered else 0) > 3, -value(g, u)))
                      + friends(g, pid, False, ctx["hidden_bf"]))


card("Lacerate", flow=flow_cost("4 energy and 2 order runes"), preds=[P_unit], resolve=_lacerate,
     choices=_lacerate_choices)


# Leona, Determined — "[Shield] When I attack, stun an enemy unit here."
def _leona_event(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc
        ok = lambda g_, it, u: P_enemy(g_, it, u) and u.loc == here
        g.queue_trigger(o.ctrl, "Leona, Determined", lambda g_, it: g_.stun(g_.legal(it, 0), it.ctrl), src=o.uid,
                        choose=trig_target(lambda g_, it: sorted([u for u in enemies(g_, it.ctrl) if ok(g_, it, u)],
                                                                 key=lambda u: u.stunned), ok, deflect=True))


card("Leona, Determined", kw={"Shield": 1}, on_event=_leona_event)


# Lightning Rush — "Look at the top 3 cards of your Main Deck. You may choose a card from among them and draw it.
# Put the rest into your trash. [Flow] [2][A]"
def _rush(g, it):
    pid = it.ctrl
    pl = g.p[pid]
    top = pl.deck[:3]
    if not top:
        return
    pick = g.ask(pid, "rush_pick", sorted(top, key=lambda c: -(c.spec["e"] + 2 * c.spec["p"])) + [None])
    if pick is not None:
        pl.deck.remove(pick)
        pick.zone = "hand"
        pl.hand.append(pick)
        g.emit("draw", pid=pid, card=pick)
    for c in top:
        if c is not pick:
            g.to_zone(c, "trash")


card("Lightning Rush", flow=flow_cost("2 energy and 1 rune of any type"), resolve=_rush)


# Loyal Poro — "[Deathknell] If I didn't die alone, draw 1."
card("Loyal Poro", deathknell=lambda g, it: (not it.data["info"]["alone"]) and g.draw(it.ctrl, 1))


# Lux, Crownguard — "[E]: [Reaction] — [Add] [2]. Use only to play spells." ([Add] abilities are used while paying)
card("Lux, Crownguard", add=[add_ability("2 energy", can=lambda g, pid, o, ctx: ctx is not None
                                         and ctx.get("kind") == "spell")])


# Machine Evangel — "[Deathknell] Play three 1 might Recruit unit tokens into your base."
card("Machine Evangel", deathknell=lambda g, it: _tokens(g, it.ctrl, "Recruit", 3, "base"))


# Masa, Crashing Thunder — "You may pay [Order] as an additional cost to play me. When you play me, if you paid the
# additional cost, [Stun] an enemy unit at a battlefield."
def _masa_play(g, o, ctx):
    if not ctx.get("masa"):
        return
    hb = ctx.get("hidden_bf")
    g.queue_trigger(o.ctrl, "Masa, Crashing Thunder", lambda g_, it: g_.stun(g_.legal(it, 0), it.ctrl), src=o.uid,
                    choose=trig_target(lambda g_, it: sorted(enemies(g_, it.ctrl, True, hb), key=lambda u: u.stunned),
                                       P_enemy_bf, deflect=True))


card("Masa, Crashing Thunder", on_play=_masa_play, as_played=lambda g, pid, c: [dict(masa=True), dict()],
     extra_cost_fn=lambda g, pid, c, ch: (0, [ORDER]) if ch.get("masa") else (0, []))


# Noxian Drummer — "When I move to a battlefield, play a 1 might Recruit unit token here."
card("Noxian Drummer", on_event=_move_tokens("Noxian Drummer", 1))


# Noxian Emissary — "[Empower] [1][Order]. [Empowered] [Deathknell] Play two 1 might Recruit unit tokens to your
# base." (look-back: Empowered when it died, rule 808.1.d.3)
card("Noxian Emissary", empower="1 energy and 1 order rune",
     deathknell=lambda g, it: it.data["info"]["empowered"] and _tokens(g, it.ctrl, "Recruit", 2, "base"))


# Peak Guardian — "When you play me, buff me. Then, if I am at a battlefield, buff all other friendly units there."
def _peak(g, it):
    me = _me(g, it)
    if me is None:
        return
    g.buff(me)
    if me.loc in (0, 1):
        for u in g.units(it.ctrl, me.loc):
            if u is not me:
                g.buff(u)


card("Peak Guardian", on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Peak Guardian", _peak, src=o.uid))


# Poppy, Defender of the Meek — "You may spend 3 XP as an additional cost to play me. If you do, I cost [3] less.
# [Ambush] [Tank]"
card("Poppy, Defender of the Meek", ambush=True, kw={"Tank": 1},
     as_played=lambda g, pid, c: ([dict(xp=3)] if g.p[pid].xp >= 3 else []) + [dict()],
     cost_mod=lambda g, pid, c, ch: (3, 0) if ch.get("xp") else (0, 0))


# Recruit the Vanguard — "[Action] Play four 1 might Recruit unit tokens." (each to your base or a battlefield you
# control; from Hidden — it has no Hidden — n/a)
card("Recruit the Vanguard", timing="action", resolve=lambda g, it: _tokens(g, it.ctrl, "Recruit", 4))


# Rek'Sai, Swarm Queen — "When I attack, you may reveal the top 2 cards of your Main Deck. You may play one. Then
# recycle the rest. If the played card is a unit, you may play it here."
# Playing it is a limited play (rule 419.3): costs are paid, timing is ignored.
def _reksai_options(g, pid, c, here):
    im = g.impl(c)
    if im is None:
        return []
    out = []
    if c.spec["type"] == "Unit":
        locs = [here] + [l for l in _locs(g, pid) if l != here]
        if im.ambush:
            locs += [b.idx for b in g.bfs if b.idx not in locs and g.units(pid, b.idx)]
        extras = im.as_played(g, pid, c) if im.as_played else [{}]
        for loc in locs:
            for ex in extras:
                for acc in ([False, True] if im.accelerate else [False]):
                    out.append(dict(ex, loc=loc, acc=acc))
        out = [ch for ch in out if g.can_pay(pid, *total_cost(g, pid, c, ch, "deck"), pay_ctx(c))]
    else:
        out = card_choices(g, pid, c, "deck", False, False)
    return [(c, ch) for ch in out[:6]]


def _reksai_res(g, it):
    pid = it.ctrl
    here = it.data["here"]
    pl = g.p[pid]
    top = pl.deck[:2]
    if not top:
        return
    g.log(f"  P{pid} reveals {top}")
    opts = []
    for c in top:
        opts += _reksai_options(g, pid, c, here)
    pick = g.ask(pid, "reksai_play", opts + [None]) if opts else None
    played = None
    if pick is not None:
        c, ch = pick
        e, reqs = total_cost(g, pid, c, ch, "deck")
        if g.pay(pid, e, reqs, pay_ctx(c)):
            played = c
            play_card(g, pid, c, "deck", dict(ch), limited=True)
    rest = [c for c in top if c is not played and c in pl.deck]
    for c in rest:
        pl.deck.remove(c)
    if rest:
        g.recycle_cards(pid, rest)


def _reksai_event(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        g.queue_trigger(o.ctrl, "Rek'Sai, Swarm Queen", _reksai_res, dict(here=o.loc), src=o.uid, may=True)


card("Rek'Sai, Swarm Queen", on_event=_reksai_event)


# Royal Guard — "When you play me, play a 2 might Sand Soldier unit token here."
card("Royal Guard", on_play=_play_tokens_here("Royal Guard", 1, "Sand Soldier"))


# Sacred Shears — "[Equip] [Order]" ; Might Bonus +1 and Effect Text (card image): "[Deathknell] — Draw 1."
# The equipped unit has the Deathknell: when it dies, its controller draws 1 (look-back 'attached', rule 808.1.d.3;
# Karthus, Eternal doubles it as in Game.kill).
def _shears_event(g, o, ev, info):
    if ev == "die" and o.uid in info["info"]["attached"]:
        inf = info["info"]
        k = 1 + sum(1 for u in g.units(inf["ctrl"]) if u.cname == "Karthus, Eternal")
        for _ in range(k):
            g.queue_trigger(inf["ctrl"], f"Deathknell {inf['name']} (Sacred Shears)",
                            lambda g_, it: g_.draw(it.ctrl, 1))


card("Sacred Shears", equip="1 order rune", bonus=1, on_event=_shears_event)


# Sandshifter — "When you play me, kill an enemy unit with 3 might or less."
def _sandshifter(g, o, ctx):
    ok = lambda g_, it, u: P_enemy(g_, it, u) and g_.might(u) <= 3
    g.queue_trigger(o.ctrl, "Sandshifter", _kill_tg, src=o.uid,
                    choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if ok(g_, it, u)], ok,
                                       deflect=True))


card("Sandshifter", on_play=_sandshifter)


# Scrutinizing Sergeant — "When you play me, gain 1 XP for each friendly unit."
card("Scrutinizing Sergeant", on_play=lambda g, o, ctx: g.queue_trigger(
    o.ctrl, "Scrutinizing Sergeant", lambda g_, it: g_.gain_xp(it.ctrl, len(g_.units(it.ctrl))), src=o.uid))


# Seal of Unity — "[E]: [Reaction] — [Add] [Order]."
card("Seal of Unity", add=[add_ability("1 order rune")])


# Sett, Kingpin — "[Tank] I get +1 might for each buffed friendly unit at my battlefield."
card("Sett, Kingpin", kw={"Tank": 1},
     might_mod=lambda g, o: len([u for u in g.units(o.ctrl, o.loc) if u.buff > 0]) if o.loc in (0, 1) else 0)


# Shadow's Call — "Choose a friendly unit without [Temporary]. Give it [Temporary]. Draw 2."
def _shadows_call(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.grant(u, "Temporary", 1, None)
    g.draw(it.ctrl, 2)


_P_no_temp = lambda g, it, o: P_friend(g, it, o) and not g.has_kw(o, "Temporary")
card("Shadow's Call", preds=[_P_no_temp], resolve=_shadows_call,
     choices=lambda g, pid, ctx: tg_choices(sorted([u for u in friends(g, pid, False, ctx["hidden_bf"])
                                                    if not g.has_kw(u, "Temporary")], key=lambda u: value(g, u))))


# Shard of Undoing — "The first time a friendly unit dies during your Beginning Phase each turn, each opponent must
# kill one of their units." (Beginning Phase: stages beginning/scoring, and 'channel' before the Channel Phase runs,
# when the hold triggers resolve, rule 315.2)
def _shard_event(g, o, ev, info):
    inf = info.get("info") if ev == "die" else None
    if inf is None or inf["ctrl"] != o.ctrl or inf["spec"]["type"] != "Unit":
        return
    if g.tp != o.ctrl or g.stage not in ("beginning", "scoring", "channel"):
        return
    key = ("order_shard", o.uid, g.turn_no)
    if key in g.stats:
        return
    g.stats[key] = 1

    def res(g_, it):
        for p in (1 - it.ctrl,):
            us = g_.units(p)
            if us:
                g_.kill([g_.ask(p, "sacrifice", sorted(us, key=lambda u: value(g_, u)))], p)
    g.queue_trigger(o.ctrl, "Shard of Undoing", res, src=o.uid)


card("Shard of Undoing", on_event=_shard_event)


# Shen, Leader of the Kinkou Order — "[Shield] When I hold, if there is exactly one other unit you control here, you
# score 1 point." (intervening condition checked when it triggers and on resolution, rule 383.2.a.1)
def _shen_ok(g, o):
    return o is not None and len([u for u in g.units(o.ctrl, o.loc) if u is not o]) == 1


def _shen_event(g, o, ev, info):
    if ev == "hold" and o in info["units"] and _shen_ok(g, o):
        g.queue_trigger(o.ctrl, "Shen, Leader of the Kinkou Order", lambda g_, it: _shen_ok(g_, _me(g_, it)) and
                        g_.gain_point(it.ctrl, "Shen, Leader of the Kinkou Order"), src=o.uid)


card("Shen, Leader of the Kinkou Order", kw={"Shield": 1}, on_event=_shen_event)


# Shepherd's Heirloom — "When you play this, gain 1 XP. [Equip] — Spend 1 XP" ; Might Bonus +2 (card image).
card("Shepherd's Heirloom", bonus=2,
     on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Shepherd's Heirloom", lambda g_, it: g_.gain_xp(it.ctrl, 1),
                                               src=o.uid),
     abilities=[dict(
         name="Equip", timing="main",
         choices=lambda g, pid, o: [dict(tg=(u.uid,)) for u in friends(g, pid) if u.uid != o.attached_to],
         cost=lambda g, pid, o, ch: (0, []), preds=[P_friend],
         can=lambda g, pid, o: g.p[pid].xp >= 1, can_extra=lambda g, pid, o: g.p[pid].xp >= 1,
         extra_cost=lambda g, pid, o, ch: g.spend_xp(pid, 1),
         resolve=lambda g, it: (g.legal(it, 0) is not None and g.obj(it.src) is not None
                                and attach(g, g.obj(it.src), g.legal(it, 0))))])


# Solari Chief — "When you play me, choose an enemy unit. If it is stunned, kill it. Otherwise, stun it."
def _chief_res(g, it):
    u = g.legal(it, 0)
    if u is None:
        return
    if u.stunned:
        g.kill([u], it.ctrl)
    else:
        g.stun(u, it.ctrl)


card("Solari Chief", on_play=lambda g, o, ctx: g.queue_trigger(
    o.ctrl, "Solari Chief", _chief_res, src=o.uid,
    choose=trig_target(lambda g_, it: sorted(enemies(g_, it.ctrl), key=lambda u: not u.stunned), P_enemy,
                       deflect=True)))


# Solari Sunhawk — "[Empower] [2]. [Empowered] I have +1 might and [Deflect 2]."
card("Solari Sunhawk", empower="2 energy", might_if=[(when_empowered, 1)], kw_if=[(when_empowered, {"Deflect": 2})])


# Soul Harvest — "Kill a unit at a battlefield with 3 might or less."
card("Soul Harvest", preds=[_p_small(3)], resolve=_kill_tg,
     choices=lambda g, pid, ctx: tg_choices([u for u in all_units(g, pid, True, ctx["hidden_bf"]) if g.might(u) <= 3]))


# Spectral Matron — "When you play me, you may play a unit costing no more than [3] and no more than [A] from your
# trash, ignoring its cost." (the card in the trash is chosen as the trigger is finalized, rule 355.10.a)
def _small_trash_units(g, pid, e, p):
    return sorted([c for c in g.p[pid].trash if c.spec["type"] == "Unit" and c.spec["e"] <= e and c.spec["p"] <= p],
                  key=lambda c: (-(c.spec["might"] or 0), -c.spec["e"], c.cname))


def _matron_choose(g, it):
    cands = _small_trash_units(g, it.ctrl, 3, 1)
    if not cands:
        return False
    c = g.ask(it.ctrl, "mixologist_pick", cands)
    it.data["pick"] = (c.uid, c.oid)
    return True


def _matron_res(g, it):
    c = _trash_card(g, it.ctrl, *it.data["pick"])
    if c is not None:
        play_unit_free(g, it.ctrl, c, "trash")


card("Spectral Matron", on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Spectral Matron", _matron_res, src=o.uid,
                                                                  may=True, choose=_matron_choose))


# Stalking Wolf — "[Ambush] As an additional cost to play me, kill a Bird, Cat, Dog, or Poro you control. You may
# play me to its battlefield (even if you don't have other units there)."
# The killed unit is still at its battlefield when the locations are listed, so Ambush already offers it.
card("Stalking Wolf", ambush=True,
     as_played=lambda g, pid, c: [dict(kill=u.uid) for u in sorted(g.units(pid), key=lambda u: value(g, u))
                                  if _is_pet(u.spec)])


# Starhound — "When you play me, return a Bird, Cat, Dog, or Poro from your trash to your hand."
def _starhound_choose(g, it):
    cands = sorted([c for c in g.p[it.ctrl].trash if _is_pet(c.spec)],
                   key=lambda c: (-(c.spec["e"] + 2 * c.spec["p"]), c.cname))
    if not cands:
        return False
    c = g.ask(it.ctrl, "starhound_pick", cands)
    it.data["pick"] = (c.uid, c.oid)
    return True


def _starhound_res(g, it):
    c = _trash_card(g, it.ctrl, *it.data["pick"])
    if c is not None:
        g.to_zone(c, "hand")


card("Starhound", on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Starhound", _starhound_res, src=o.uid,
                                                            choose=_starhound_choose))


# The Ruination — "Kill all units."
card("The Ruination", resolve=lambda g, it: g.kill(g.units(), it.ctrl))


# Trifarian Gloryseeker — "[Legion] — When you play me, buff me."
def _gloryseeker(g, o, ctx):
    if g.legion(o.ctrl, o):
        g.queue_trigger(o.ctrl, "Trifarian Gloryseeker", lambda g_, it: g_.buff(_me(g_, it)), src=o.uid)


card("Trifarian Gloryseeker", on_play=_gloryseeker)


# Trove Golem — "When you play me, play four Gold gear tokens exhausted."
card("Trove Golem", on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Trove Golem",
                                                              lambda g_, it: _tokens(g_, it.ctrl, "Gold", 4),
                                                              src=o.uid))


# Trusty Ramhound — "While you have another unit here, I have +1 might."
card("Trusty Ramhound", might_mod=lambda g, o: 0 if g.alone(o) else 1)


# Ultrasoft Poro — "[Deflect] [E]: Play two 1 might Bird unit tokens with [Deflect]. Use this ability only while I'm
# at a battlefield."
card("Ultrasoft Poro", kw={"Deflect": 1}, abilities=[ability(
    "Birds", exhaust=True, can=lambda g, pid, o: o.loc in (0, 1),
    resolve=lambda g, it: _tokens(g, it.ctrl, "Bird", 2))])


# Undying Loyalty — "This costs [2] less if you choose a Bird, Cat, Dog, or Poro. Play a unit with cost no more than
# [2] and no more than [A] from your trash, ignoring its cost." (the unit card in the trash is a target, 355.10.a)
def _loyalty_choices(g, pid, ctx):
    out, seen = [], set()
    for c in sorted(_small_trash_units(g, pid, 2, 1), key=lambda c: (not _is_pet(c.spec), -(c.spec["might"] or 0))):
        if c.cname not in seen:
            seen.add(c.cname)
            out.append(dict(pick=(c.uid, c.oid)))
    return out


def _loyalty_res(g, it):
    c = _trash_card(g, it.ctrl, *it.data["pick"])
    if c is not None:
        play_unit_free(g, it.ctrl, c, "trash")


def _loyalty_cost(g, pid, card, ch):
    c = _trash_card(g, pid, *ch["pick"]) if ch.get("pick") else None
    return (2, 0) if c is not None and _is_pet(c.spec) else (0, 0)


card("Undying Loyalty", choices=_loyalty_choices, resolve=_loyalty_res, cost_mod=_loyalty_cost)


# Unsung Hero — "[Deathknell] — If I was [Mighty], draw 2." (look-back Might, rule 808.1.d.3)
card("Unsung Hero", deathknell=lambda g, it: it.data["info"]["might"] >= 5 and g.draw(it.ctrl, 2))


# Vanguard Armory — "[E]: Play three 1 might Recruit unit tokens. (You may play them to different locations.)"
card("Vanguard Armory", abilities=[ability("Recruits", exhaust=True,
                                           resolve=lambda g, it: _tokens(g, it.ctrl, "Recruit", 3))])


# Vanguard Attendant — "I enter ready."
card("Vanguard Attendant", enter_ready=True)


# Vanguard Captain — "[Legion] — When you play me, play two 1 might Recruit unit tokens here."
def _captain(g, o, ctx):
    if g.legion(o.ctrl, o):
        _play_tokens_here("Vanguard Captain", 2, "Recruit")(g, o, ctx)


card("Vanguard Captain", on_play=_captain)


# Vengeance — "Kill a unit."
card("Vengeance", preds=[P_unit], resolve=_kill_tg,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, False, ctx["hidden_bf"])))


# Viktor, Leader — "When another non-Recruit unit you control dies, play a 1 might Recruit unit token into your base."
# (Viktor dying at the same time doesn't see it, rule 383.2.c.2)
def _viktor_event(g, o, ev, info):
    if ev != "die":
        return
    inf = info["info"]
    if inf["obj"] is not o and inf["ctrl"] == o.ctrl and inf["spec"]["type"] == "Unit" \
            and "Recruit" not in inf["spec"]["tags"]:
        g.queue_trigger(o.ctrl, "Viktor, Leader", lambda g_, it: _tokens(g_, it.ctrl, "Recruit", 1, "base"), src=o.uid)


card("Viktor, Leader", on_event=_viktor_event)


# Xin Zhao, Vigilant — "[Tank] I enter ready if you have two or more other units in your base."
card("Xin Zhao, Vigilant", kw={"Tank": 1},
     enter_ready=lambda g, pid, c, ch: len([u for u in g.units(pid, "base") if u is not c]) >= 2)


# Zaun Punk — "You may kill a friendly gear as an additional cost to play me. When you play me, if you paid the
# additional cost, kill a gear."
def _punk_play(g, o, ctx):
    if ctx.get("kill") is None:
        return
    g.queue_trigger(o.ctrl, "Zaun Punk", _kill_tg, src=o.uid,
                    choose=trig_target(lambda g_, it: sorted(g_.gear(), key=lambda x: (x.ctrl == it.ctrl,
                                                                                     -x.spec["e"])), P_gear,
                                       deflect=True))


card("Zaun Punk", on_play=_punk_play,
     as_played=lambda g, pid, c: [dict()] + [dict(kill=x.uid) for x in sorted(g.gear(pid),
                                                                               key=lambda x: (x.spec["e"], x.uid))])
