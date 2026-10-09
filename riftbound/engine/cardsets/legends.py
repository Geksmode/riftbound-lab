"""Cartes du paquet legends (liste : batches/legends.txt). Voir GUIDE.md.

Legends are not on the board: their triggered abilities are `legend_event(g, pid, ev, info)`, their activated
abilities are `abilities` (item.src is the legend's uid, `g.p[pid].legend` is the legend Obj, which can be
exhausted / empowered), their passive abilities over units are `aura_kw` / `aura_might` (src = the legend Obj,
src.owner = its player) and their [Add] abilities are `add` (used while paying, rule 429.3).
Cards of the batch that need an engine hook are listed in NEEDS_legends.md and are not registered here.
"""
from cards import *  # noqa: F401,F403
from actions import card_choices, keyword_play_triggers


# ---------------------------------------------------------------------- helpers
def _leg(g, pid):
    return g.p[pid].legend


def _exhaust_me(g, it):
    """Cost within instructions "exhaust me" of a legend trigger (rule 383.3.b): False if already exhausted."""
    leg = _leg(g, it.ctrl)
    if leg.exhausted:
        return False
    leg.exhausted = True
    return True


def _same(g, uid, oid):
    """The board object uid if it is still the same incarnation (rule 359.3.e.2)."""
    o = g.obj(uid)
    return o if o is not None and o.oid == oid else None


def _is_equipment(o):
    return o.spec["type"] == "Gear" and "Equipment" in o.spec["tags"]


def _friends_first(g, pid, us, key=None):
    """Units ordered for the AI: friendly ones first (by key or value), then enemies."""
    key = key or (lambda u: -value(g, u))
    return sorted([u for u in us if u.ctrl == pid], key=key) + sorted([u for u in us if u.ctrl != pid],
                                                                       key=lambda u: value(g, u))


# ---------------------------------------------------------------------- Ahri, Nine-Tailed Fox
# "When an enemy unit attacks a battlefield you control, give it -1 might this turn, to a minimum of 1 might."
def _ahri(g, pid, ev, info):
    if ev != "attack":
        return
    u = info["obj"]
    if u.ctrl == pid or g.bfs[info["bf"]].ctrl != pid:
        return

    def res(g_, it):
        o = _same(g_, it.data["u"], it.data["oid"])
        if o is not None:
            g_.mod(o, -1, minimum=1)            # snapshot at resolution (rule 477.3.b)
    g.queue_trigger(pid, "Ahri, Nine-Tailed Fox", res, dict(u=u.uid, oid=u.oid), src=_leg(g, pid).uid)


card("Ahri, Nine-Tailed Fox", legend_event=_ahri)


# ---------------------------------------------------------------------- Annie, Dark Child
# "At the end of your turn, ready 2 runes."
def _annie_res(g, it):
    for _ in range(2):
        ex = sorted([r for r in g.p[it.ctrl].runes if r.exhausted], key=lambda r: (r.domain, r.uid))
        if not ex:
            return
        r = g.ask(it.ctrl, "ready_rune", ex)
        r.exhausted = False


def _annie(g, pid, ev, info):
    if ev == "end_turn" and info["pid"] == pid:
        g.queue_trigger(pid, "Annie, Dark Child", _annie_res, src=_leg(g, pid).uid)


card("Annie, Dark Child", legend_event=_annie)


# ---------------------------------------------------------------------- Azir, Emperor of the Sands
# "Your Sand Soldiers have [Weaponmaster]. 1 energy, [E]: Play a 2 might Sand Soldier unit token to your base.
# Use only if you've played an Equipment this turn."
def _azir_aura(g, src, o):
    if o.ctrl == src.owner and o.spec["type"] == "Unit" and o.cname == "Sand Soldier":
        return {"Weaponmaster": 1}
    return None


def _azir_play(g, it):
    # Tokens are played (rule 185.2.a): make_token runs its play triggers, incl. [Weaponmaster] from the aura (rule 821).
    make_token(g, "Sand Soldier", it.ctrl, "base")


card("Azir, Emperor of the Sands", aura_kw=_azir_aura, abilities=[
    ability("Sand Soldier", "1 energy", exhaust=True, resolve=_azir_play,
            can=lambda g, pid, o: any("Equipment" in SPEC[n]["tags"] for n in g.played[pid]))])


# ---------------------------------------------------------------------- [Add] legends
# Darius, Hand of Noxus — "[E]: [Reaction], [Legion] — [Add] 1 energy." (Legion: you've played a card this
# turn, rule 812; while paying for a card, that card is not yet finalized)
card("Darius, Hand of Noxus", add=[add_ability("1 energy", can=lambda g, pid, o, ctx: g.legion(pid))])

# Diana, Scorn of the Moon — "[Reaction][>] [E]: [Add] 1 energy. Spend this Energy only during showdowns."
# The Add is only used while paying (rule 429.3), so the energy is spent at once: only during a showdown.
card("Diana, Scorn of the Moon", add=[add_ability("1 energy", can=lambda g, pid, o, ctx: g.sd is not None)])

# Kai'Sa, Daughter of the Void — "[E]: [Reaction] — [Add] 1 rune of any type. Use only to play spells."
card("Kai'Sa, Daughter of the Void",
     add=[add_ability("1 rune of any type", can=lambda g, pid, o, ctx: ctx is not None and ctx.get("kind") == "spell")])


# Ornn, Fire Below the Mountain — "[E]: [Reaction] — [Add] 1 rune of any type. Use only to play gear or use gear
# abilities."
def _ornn_can(g, pid, o, ctx):
    if ctx is None:
        return False
    if ctx.get("kind") == "gear":
        return True
    ob = ctx.get("obj")
    return ctx.get("kind") == "ability" and ob is not None and ob.zone == "board" and ob.spec["type"] == "Gear"


card("Ornn, Fire Below the Mountain", add=[add_ability("1 rune of any type", can=_ornn_can)])


# ---------------------------------------------------------------------- Draven, Glorious Executioner
# "When you win a combat, draw 1."
def _draven(g, pid, ev, info):
    if ev == "combat_won" and info["pid"] == pid:
        g.queue_trigger(pid, "Draven, Glorious Executioner", lambda g_, it: g_.draw(it.ctrl, 1), src=_leg(g, pid).uid)


card("Draven, Glorious Executioner", legend_event=_draven)


# ---------------------------------------------------------------------- Ezreal, Prodigal Explorer
# "[E]: [Reaction] — Draw 1. Use only if you've chosen enemy units and/or gear twice this turn with spells or unit
# abilities."  Counted once per spell / unit ability that chose at least one enemy unit or gear (a targeting
# event, rule 383.4.b, happens when the item is finalized).
def _unit_item(g, item):
    if item.kind == "spell":
        return True
    if item.kind == "trigger" and item.name.startswith("Deathknell "):
        return True                                     # Deathknell of a unit (its source has left the board)
    src = g.obj(item.src) if item.src is not None else None
    return src is not None and src.spec["type"] == "Unit"


def _ezreal(g, pid, ev, info):
    if ev != "chosen":
        return
    it, o = info["item"], info["obj"]
    if it.ctrl != pid or o.ctrl == pid or o.spec["type"] not in ("Unit", "Gear") or not _unit_item(g, it):
        return
    if g.stats[("ezreal_last", pid)] == it.id:
        return
    g.stats[("ezreal_last", pid)] = it.id
    g.stats[("ezreal", pid, g.turn_no)] += 1


card("Ezreal, Prodigal Explorer", legend_event=_ezreal, abilities=[
    ability("Draw", timing="reaction", exhaust=True, resolve=lambda g, it: g.draw(it.ctrl, 1),
            can=lambda g, pid, o: g.stats[("ezreal", pid, g.turn_no)] >= 2)])


# ---------------------------------------------------------------------- Fiora, Grand Duelist
# "When one of your units becomes [Mighty], you may exhaust me to channel 1 rune exhausted."
def _fiora(g, pid, ev, info):
    if ev == "becomes_mighty" and info["obj"].ctrl == pid:
        g.queue_trigger(pid, "Fiora, Grand Duelist", lambda g_, it: g_.channel(it.ctrl, 1, exhausted=True),
                        src=_leg(g, pid).uid, may=True, cost=_exhaust_me)


card("Fiora, Grand Duelist", legend_event=_fiora, track_mighty=True)


# ---------------------------------------------------------------------- Garen, Might of Demacia
# "When you conquer, if you have 4+ units at that battlefield, draw 2."  The "if" is part of the trigger
# condition (rule 383.2.a.1): checked when the conquer happens only.
def _garen(g, pid, ev, info):
    if ev == "conquer" and info["pid"] == pid and len(info["units"]) >= 4:
        g.queue_trigger(pid, "Garen, Might of Demacia", lambda g_, it: g_.draw(it.ctrl, 2), src=_leg(g, pid).uid)


card("Garen, Might of Demacia", legend_event=_garen)


# ---------------------------------------------------------------------- Irelia, Blade Dancer
# "When you choose a friendly unit, you may exhaust me and pay 1 rune of any type to ready it. When you conquer,
# you may pay 1 energy to ready me."
def _irelia_cost(g, it):
    leg = _leg(g, it.ctrl)
    if leg.exhausted or not g.can_pay(it.ctrl, 0, [ANY]):
        return False
    leg.exhausted = True
    return g.pay(it.ctrl, 0, [ANY])


def _irelia_can(g, it):
    # Pas de question « utiliser l'effet ? » (retour utilisateur) quand la légende est déjà inclinée, qu'aucune rune ne
    # peut être payée, ou que l'unité choisie est déjà prête (« ready it » ne ferait rien).
    o = _same(g, it.data["u"], it.data["oid"])
    return (not _leg(g, it.ctrl).exhausted and g.can_pay(it.ctrl, 0, [ANY])
            and o is not None and o.exhausted)


_irelia_cost.can = _irelia_can
_irelia_cost.label = "légende prête et 1 rune, pour une unité inclinée"


def _irelia_ready(g, it):
    o = _same(g, it.data["u"], it.data["oid"])
    if o is not None:
        g.ready_obj(o)


def _irelia(g, pid, ev, info):
    if ev == "chosen":
        it, o = info["item"], info["obj"]
        if it.ctrl == pid and o.ctrl == pid and o.spec["type"] == "Unit":
            g.queue_trigger(pid, "Irelia, Blade Dancer", _irelia_ready, dict(u=o.uid, oid=o.oid),
                            src=_leg(g, pid).uid, may=True, cost=_irelia_cost)
    elif ev == "conquer" and info["pid"] == pid:
        g.queue_trigger(pid, "Irelia, Blade Dancer (conquer)", lambda g_, it: g_.ready_obj(_leg(g_, it.ctrl)),
                        src=_leg(g, pid).uid, may=True, cost=may_pay(1))


card("Irelia, Blade Dancer", legend_event=_irelia)


# ---------------------------------------------------------------------- Jax, Grandmaster At Arms
# "1 energy, [E]: Attach a detached Equipment you control to a unit you control. [E]: Attach an attached
# Equipment you control to a unit you control."  (Attach, rule 434: the Equipment's location follows the unit.)
def _jax_choices(attached):
    def choices(g, pid, o):
        gears = [x for x in g.gear(pid) if _is_equipment(x) and (x.attached_to is not None) == attached]
        gears.sort(key=lambda x: (-EQUIP_BONUS.get(x.cname, 0), x.uid))
        out = []
        for u in sorted(friends(g, pid), key=lambda u: (u.loc not in (0, 1), -value(g, u))):
            for x in gears:
                if x.attached_to != u.uid:
                    out.append(dict(tg=(x.uid, u.uid)))
        return out
    return choices


def _jax_pred(attached):
    return lambda g, it, x: x.ctrl == it.ctrl and _is_equipment(x) and (x.attached_to is not None) == attached


def _jax_res(g, it):
    x, u = g.legal(it, 0), g.legal(it, 1)
    if x is not None and u is not None and x.attached_to != u.uid:
        attach(g, x, u)


card("Jax, Grandmaster At Arms", abilities=[
    ability("Attach detached", "1 energy", exhaust=True, choices=_jax_choices(False),
            preds=[_jax_pred(False), P_friend], resolve=_jax_res),
    ability("Attach attached", exhaust=True, choices=_jax_choices(True),
            preds=[_jax_pred(True), P_friend], resolve=_jax_res)])


# ---------------------------------------------------------------------- Jayce, Defender of Tomorrow
# "[Empower] 2 energy and 2 runes of any type. 1 energy, [E]: Ready a gear. [Empowered][>] 1 energy, [E]: Ready
# 2 gear."
def _gear_order(g, pid):
    return sorted(g.gear(), key=lambda x: (not x.exhausted, x.ctrl != pid, -x.spec["e"], x.uid))


def _jayce_one(g, pid, o):
    return [dict(tg=(x.uid,)) for x in _gear_order(g, pid) if x.exhausted]


def _jayce_two(g, pid, o):
    gs = _gear_order(g, pid)
    out = []
    for i, a in enumerate(gs):
        if not a.exhausted or i >= 4:
            break
        for b in gs[i + 1:]:
            out.append(dict(tg=(a.uid, b.uid)))
    return out


def _jayce_res(g, it):
    for i in range(len(it.targets)):
        x = g.legal(it, i)
        if x is not None:
            g.ready_obj(x)


card("Jayce, Defender of Tomorrow", empower="2 energy and 2 runes of any type", abilities=[
    ability("Ready a gear", "1 energy", exhaust=True, choices=_jayce_one, preds=[P_gear], resolve=_jayce_res),
    ability("Ready 2 gear", "1 energy", exhaust=True, choices=_jayce_two, preds=[P_gear, P_gear],
            resolve=_jayce_res, can=lambda g, pid, o: o.empowered)])


# ---------------------------------------------------------------------- Jinx, Loose Cannon
# "At start of your Beginning Phase, draw 1 if you have one or fewer cards in your hand."  (The "if" is part of
# the effect, checked on resolution: rule 383.2.a.1, Loose Cannon example.)
def _jinx(g, pid, ev, info):
    if ev == "beginning_start" and info["pid"] == pid:
        g.queue_trigger(pid, "Jinx, Loose Cannon",
                        lambda g_, it: len(g_.p[it.ctrl].hand) <= 1 and g_.draw(it.ctrl, 1), src=_leg(g, pid).uid)


card("Jinx, Loose Cannon", legend_event=_jinx)


# ---------------------------------------------------------------------- Kennen, Heart of the Tempest
# "When you play a card from anywhere other than your hand, empower me. [Action][>] Disempower me, [E]: Give a
# unit [Assault 2] this turn."  (Tokens are not cards, rule 185.)
def _played_from(info):
    c = info["card"]
    if c.spec["type"] == "Spell":
        it = info.get("item")
        return it.data.get("from") if it is not None else None
    return getattr(c, "played_from", None)


def _kennen(g, pid, ev, info):
    if ev == "played" and info["pid"] == pid and not info["card"].token and _played_from(info) != "hand":
        g.queue_trigger(pid, "Kennen, Heart of the Tempest", lambda g_, it: g_.empower(_leg(g_, it.ctrl)),
                        src=_leg(g, pid).uid)


def _kennen_choices(g, pid, o):
    us = [u for u in g.units() if g.targetable(u, pid)]
    return tg_choices(_friends_first(g, pid, us, key=lambda u: (u.loc not in (0, 1), -value(g, u))))


card("Kennen, Heart of the Tempest", legend_event=_kennen, abilities=[
    ability("Assault 2", timing="action", exhaust=True, choices=_kennen_choices, preds=[P_unit],
            can=lambda g, pid, o: o.empowered, extra_cost=lambda g, pid, o, ch: g.disempower(o),
            resolve=lambda g, it: g.legal(it, 0) is not None and g.grant(g.legal(it, 0), "Assault", 2, "turn"))])


# ---------------------------------------------------------------------- Kha'Zix, Voidreaver
# "When you win a combat, gain 1 XP. Spend 1 XP, [E]: [Buff] a unit. Spend 2 XP, [E]: Move an exhausted friendly
# unit from a battlefield to its base."
def _khazix(g, pid, ev, info):
    if ev == "combat_won" and info["pid"] == pid:
        g.queue_trigger(pid, "Kha'Zix, Voidreaver", lambda g_, it: g_.gain_xp(it.ctrl, 1), src=_leg(g, pid).uid)


def _buff_choices(g, pid, o, only_friends=False):
    us = [u for u in g.units() if g.targetable(u, pid) and (not only_friends or u.ctrl == pid)]
    return tg_choices(_friends_first(g, pid, us, key=lambda u: (u.buff > 0, -value(g, u))))


def _p_exh_friend_bf(g, it, o):
    return P_friend_bf(g, it, o) and o.exhausted


def _khazix_move(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.move([u], "base", it.ctrl)


card("Kha'Zix, Voidreaver", legend_event=_khazix, abilities=[
    ability("Buff", xp=1, exhaust=True, choices=_buff_choices, preds=[P_unit],
            resolve=lambda g, it: g.buff(g.legal(it, 0))),
    ability("Move to base", xp=2, exhaust=True, preds=[_p_exh_friend_bf], resolve=_khazix_move,
            choices=lambda g, pid, o: tg_choices([u for u in friends(g, pid, True) if u.exhausted]))])


# ---------------------------------------------------------------------- Lee Sin, Blind Monk
# "1 energy, [E]: Buff a friendly unit."
card("Lee Sin, Blind Monk", abilities=[
    ability("Buff", "1 energy", exhaust=True, preds=[P_friend], resolve=lambda g, it: g.buff(g.legal(it, 0)),
            choices=lambda g, pid, o: _buff_choices(g, pid, o, only_friends=True))])


# ---------------------------------------------------------------------- Leona, Radiant Dawn
# "When you stun one or more enemy units, buff a friendly unit."  Units stunned by the same action trigger it once:
# their 'stun' events are emitted before the triggers are put on the chain, so a pending Leona trigger is reused.
def _leona(g, pid, ev, info):
    if ev != "stun" or info.get("by") != pid or info["obj"].ctrl == pid:
        return
    if any(t.ctrl == pid and t.name == "Leona, Radiant Dawn" for t in g.trigq):
        return
    g.queue_trigger(pid, "Leona, Radiant Dawn", lambda g_, it: g_.buff(g_.legal(it, 0)), src=_leg(g, pid).uid,
                    choose=trig_target(lambda g_, it: sorted(g_.units(it.ctrl), key=lambda u: (u.buff > 0, -value(g_, u))),
                                       P_friend, kind="buff_target"))


card("Leona, Radiant Dawn", legend_event=_leona)


# ---------------------------------------------------------------------- Lillia, Bashful Bloom
# "4 energy, [E]: Play a ready 3 might Sprite unit token with [Temporary]. This ability costs 1 energy less for
# each friendly unit with [Temporary]."
def _lillia_cost(g, pid, o, ch):
    n = sum(1 for u in g.units(pid) if g.has_kw(u, "Temporary"))
    return max(0, 4 - n), []


def _lillia_res(g, it):
    make_token(g, "Sprite", it.ctrl, "base", ready=True)


card("Lillia, Bashful Bloom", abilities=[ability("Sprite", _lillia_cost, exhaust=True, resolve=_lillia_res)])


# ---------------------------------------------------------------------- Lucian, Purifier
# "Your Equipment each give [Assault]."  Each Equipment you control attached to a unit gives it [Assault]
# (Assault values add up, rule 807.2).
def _lucian_aura(g, src, o):
    if o.spec["type"] != "Unit" or not o.attached:
        return None
    n = 0
    for gu in o.attached:
        x = g.obj(gu)
        if x is not None and x.ctrl == src.owner and _is_equipment(x):
            n += 1
    return {"Assault": n} if n else None


card("Lucian, Purifier", aura_kw=_lucian_aura)


# ---------------------------------------------------------------------- Lux, Lady of Luminosity
# "When you play a spell that costs 5 energy or more, draw 1."  Cost = printed cost (rule 206).
def _lux(g, pid, ev, info):
    if ev == "played" and info["pid"] == pid:
        c = info["card"]
        if c.spec["type"] == "Spell" and c.spec["e"] >= 5:
            g.queue_trigger(pid, "Lux, Lady of Luminosity", lambda g_, it: g_.draw(it.ctrl, 1), src=_leg(g, pid).uid)


card("Lux, Lady of Luminosity", legend_event=_lux)


# ---------------------------------------------------------------------- Master Yi, Wuju Bladesman
# "While a friendly unit defends alone, it gets +2 might."  (Alone: no other friendly unit there, rule 740.2.a.)
def _yi_aura(g, src, o):
    if o.ctrl == src.owner and o.spec["type"] == "Unit" and o.desig == "def" and g.alone(o):
        return 2
    return 0


card("Master Yi, Wuju Bladesman", aura_might=_yi_aura)


# ---------------------------------------------------------------------- Miss Fortune, Bounty Hunter
# "[E]: Give a unit [Ganking] this turn."
def _mf_choices(g, pid, o):
    us = [u for u in g.units() if g.targetable(u, pid)]
    return tg_choices(_friends_first(g, pid, us, key=lambda u: (g.has_kw(u, "Ganking"), u.loc not in (0, 1),
                                                                u.exhausted, -value(g, u))))


card("Miss Fortune, Bounty Hunter", abilities=[
    ability("Ganking", exhaust=True, choices=_mf_choices, preds=[P_unit],
            resolve=lambda g, it: g.legal(it, 0) is not None and g.grant(g.legal(it, 0), "Ganking", 1, "turn"))])


# ---------------------------------------------------------------------- Poppy, Keeper of the Hammer
# "When you hold, gain 1 XP. Spend 3 XP, [E]: Draw 1."
def _poppy(g, pid, ev, info):
    if ev == "hold" and info["pid"] == pid:
        g.queue_trigger(pid, "Poppy, Keeper of the Hammer", lambda g_, it: g_.gain_xp(it.ctrl, 1), src=_leg(g, pid).uid)


card("Poppy, Keeper of the Hammer", legend_event=_poppy, abilities=[
    ability("Draw", xp=3, exhaust=True, resolve=lambda g, it: g.draw(it.ctrl, 1))])


# ---------------------------------------------------------------------- Pyke, Bloodharbor Ripper
# "1 energy, [E]: Return a friendly unit at a battlefield to its owner's hand. Play a Gold gear token exhausted."
# The ability resolves even if its target became illegal (rule 359.3.e.1): the Gold is still played.
def _pyke(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.to_zone(u, "hand")                       # to its owner's hand; a token ceases to exist (rule 186.1)
    make_token(g, "Gold", it.ctrl, "base", ready=False)


card("Pyke, Bloodharbor Ripper", abilities=[
    ability("Return", "1 energy", exhaust=True, preds=[P_friend_bf], resolve=_pyke,
            choices=lambda g, pid, o: tg_choices(sorted(friends(g, pid, True),
                                                        key=lambda u: (not u.exhausted, u.token, -u.spec["e"]))))])


# ---------------------------------------------------------------------- Rek'Sai, Void Burrower
# "When you conquer, you may exhaust me to reveal the top 2 cards of your Main Deck. You may play one. Then
# recycle the rest."  Playing it is a Limited Play (rule 419.3): its costs are paid as normal; it is played during
# the resolution of the ability, so it is not restricted to the Main Phase or to [Action]/[Reaction] cards.
def _reksai_res(g, it):
    pid = it.ctrl
    pl = g.p[pid]
    top = g.reveal(pid, 2)                         # fewer cards: reveal as many as possible (rule 431.1.c)
    if not top:
        return
    opts = []
    for c in top:
        chs = card_choices(g, pid, c, "deck", False, False)
        if chs:
            opts.append((c, chs))
    opts.sort(key=lambda x: -(x[0].spec["e"] + 2 * x[0].spec["p"]))
    pick = g.ask(pid, "reksai_pick", [c for c, _ in opts] + [None])
    if pick is not None:
        chs = [x for c, x in opts if c is pick][0]
        ch = g.ask(pid, "reksai_choice", chs, card=pick)
        play_card(g, pid, pick, "deck", dict(ch))
    rest = [c for c in top if c is not pick and c in pl.deck]
    g.recycle_cards(pid, rest)


def _reksai(g, pid, ev, info):
    if ev == "conquer" and info["pid"] == pid:
        g.queue_trigger(pid, "Rek'sai, Void Burrower", _reksai_res, src=_leg(g, pid).uid, may=True, cost=_exhaust_me)


card("Rek'sai, Void Burrower", legend_event=_reksai)


# ---------------------------------------------------------------------- Rengar, Pridestalker
# "When you play a unit, give a unit +1 might this turn."
def _rengar(g, pid, ev, info):
    if ev == "played" and info["pid"] == pid and info["card"].spec["type"] == "Unit":
        def opts(g_, it):
            us = [u for u in g_.units() if g_.targetable(u, it.ctrl)]
            return _friends_first(g_, it.ctrl, us, key=lambda u: (u.loc not in (0, 1), -value(g_, u)))
        g.queue_trigger(pid, "Rengar, Pridestalker", lambda g_, it: g_.legal(it, 0) is not None and
                        g_.mod(g_.legal(it, 0), 1), src=_leg(g, pid).uid,
                        choose=trig_target(opts, P_unit, kind="rengar_target", deflect=True))


card("Rengar, Pridestalker", legend_event=_rengar)


# ---------------------------------------------------------------------- Rumble, Mechanized Menace
# "Your Mechs have [Shield]."
card("Rumble, Mechanized Menace",
     aura_kw=lambda g, src, o: {"Shield": 1} if (o.ctrl == src.owner and o.spec["type"] == "Unit"
                                                 and "Mech" in g.tags(o)) else None)


# ---------------------------------------------------------------------- Shen, Eye of Twilight
# "[Action][>] [E]: Give a friendly unit [Tank] this turn."
card("Shen, Eye of Twilight", abilities=[
    ability("Tank", timing="action", exhaust=True, preds=[P_friend],
            choices=lambda g, pid, o: tg_choices(sorted(friends(g, pid), key=lambda u: (not g.in_combat(u),
                                                                                       g.has_kw(u, "Tank"),
                                                                                       -g.might(u)))),
            resolve=lambda g, it: g.legal(it, 0) is not None and g.grant(g.legal(it, 0), "Tank", 1, "turn"))])


# ---------------------------------------------------------------------- Vex, Gloomist
# "When you or an ally hold, you may exhaust me to draw 1."  (1v1: no ally.)
def _vex(g, pid, ev, info):
    if ev == "hold" and info["pid"] == pid:
        g.queue_trigger(pid, "Vex, Gloomist", lambda g_, it: g_.draw(it.ctrl, 1), src=_leg(g, pid).uid,
                        may=True, cost=_exhaust_me)


card("Vex, Gloomist", legend_event=_vex)


# ---------------------------------------------------------------------- Vi, Piltover Enforcer
# "When you conquer, if you assigned 3 or more excess damage, you may exhaust me to ready a unit."
# Excess damage = combat damage assigned to a unit beyond the lethal damage it needed (rules 142.4, 437.5.a,
# 465.2.c.3). It is measured when the combat damage is dealt (the 'damaged' events of the combat, before the
# units die) from the assignment kept in g.sd.assigned, and stored on the showdown. The "if" is part of the trigger
# condition (rule 383.2.a.1).
def _vi_record(g, pid, info):
    sd = g.sd
    if sd is None or not sd.combat or sd.assigned is None:
        return
    mine = sd.assigned[0] if pid == sd.attacker else sd.assigned[1]
    u = info["obj"]
    if u not in mine:
        return
    n, dealt = mine[u], info["amount"]
    prevent_before = u.prevent + (n - dealt)            # the rest of the assignment was prevented
    need = max(1, g.might(u) - (u.damage - dealt)) + prevent_before
    if n > need:
        ex = getattr(sd, "legends_excess", None)
        if ex is None:
            ex = sd.legends_excess = [0, 0]
        ex[pid] += n - need


def _vi(g, pid, ev, info):
    if ev == "damaged" and info["kind"] == "combat":
        _vi_record(g, pid, info)
    elif ev == "conquer" and info["pid"] == pid:
        sd = g.sd
        if sd is None or not sd.combat or sd.bf != info["bf"]:
            return
        if (getattr(sd, "legends_excess", None) or [0, 0])[pid] < 3:
            return
        g.queue_trigger(pid, "Vi, Piltover Enforcer", lambda g_, it: g_.legal(it, 0) is not None and
                        g_.ready_obj(g_.legal(it, 0)), src=_leg(g, pid).uid, may=True,
                        choose=trig_target(lambda g_, it: _friends_first(
                            g_, it.ctrl, [u for u in g_.units() if g_.targetable(u, it.ctrl)],
                            key=lambda u: (not u.exhausted, -value(g_, u))), P_unit, kind="ready_target",
                            deflect=True),
                        cost=_exhaust_me)


card("Vi, Piltover Enforcer", legend_event=_vi)


# ---------------------------------------------------------------------- Viktor, Herald of the Arcane
# "1 energy, [E]: Play a 1 might Recruit unit token."  (To its controller's base.)
def _viktor(g, it):
    make_token(g, "Recruit", it.ctrl, "base")


card("Viktor, Herald of the Arcane", abilities=[ability("Recruit", "1 energy", exhaust=True, resolve=_viktor)])


# ---------------------------------------------------------------------- Volibear, Relentless Storm
# "When you play a [Mighty] unit, you may exhaust me to channel 1 rune exhausted."
def _volibear(g, pid, ev, info):
    if ev == "played" and info["pid"] == pid:
        c = info["card"]
        if c.spec["type"] == "Unit" and g.mighty(c):
            g.queue_trigger(pid, "Volibear, Relentless Storm", lambda g_, it: g_.channel(it.ctrl, 1, exhausted=True),
                            src=_leg(g, pid).uid, may=True, cost=_exhaust_me)


card("Volibear, Relentless Storm", legend_event=_volibear)


# ---------------------------------------------------------------------- Yasuo, Unforgiven
# "2 energy, [E]: Move a friendly unit to or from its base."
def _yasuo_choices(g, pid, o):
    out = []
    for u in friends(g, pid):
        if u.loc == "base":
            for b in (0, 1):
                out.append(dict(tg=(u.uid,), dest=b))
        else:
            out.append(dict(tg=(u.uid,), dest="base"))

    def score(c):
        u = g.obj(c["tg"][0])
        if c["dest"] == "base":
            return (1, not u.exhausted, -value(g, u))
        return (0, g.bfs[c["dest"]].ctrl == pid, -value(g, u))
    out.sort(key=score)
    return out


def _yasuo(g, it):
    u = g.legal(it, 0)
    d = it.data.get("dest")
    if u is None or d is None:
        return
    if (d == "base" and u.loc in (0, 1)) or (d in (0, 1) and u.loc == "base"):
        g.move([u], d, it.ctrl)


card("Yasuo, Unforgiven", abilities=[
    ability("Move", "2 energy", exhaust=True, choices=_yasuo_choices, preds=[P_friend], resolve=_yasuo)])


# ====================================================================== legends using the engine hooks (integration)
ask_text(nasus_ready_rune="Nasus : quelle rune redresser ?", zed_discard="Zed : quelle carte défausser ?")


def _disempower_me(g, pid, o, ch):
    g.disempower(o)                     # the source of the ability (Heimerdinger, Inventor may borrow it)


def _empowered(g, pid, o):
    return o.empowered


def _empower_echo(name):
    """'When you empower something else, empower me.' (the player who empowers: event 'empowered' by=, rule 441)"""
    def ev(g, pid, ev_, info):
        if ev_ == "empowered" and info.get("by") == pid and info["obj"] is not _leg(g, pid):
            g.queue_trigger(pid, name, lambda g_, it: g_.empower(_leg(g_, it.ctrl), by=it.ctrl), src=_leg(g, pid).uid)
    return ev


# ---------------------------------------------------------------------- Ambessa, Matriarch of War
# "When you empower something else, empower me. Disempower me, 1 rune of any type, exhaust: Ready a unit."
card("Ambessa, Matriarch of War", legend_event=_empower_echo("Ambessa, Matriarch of War"), abilities=[ability(
    "Ready", "1 rune of any type", exhaust=True, preds=[P_unit], can=_empowered, extra_cost=_disempower_me,
    choices=lambda g, pid, o: tg_choices(sorted(
        [u for u in g.units() if g.targetable(u, pid)],
        key=lambda u: (u.ctrl != pid, not u.exhausted, -value(g, u), u.uid))),
    resolve=lambda g, it: g.legal(it, 0) is not None and g.ready_obj(g.legal(it, 0)))])


# ---------------------------------------------------------------------- Mel, Soul's Reflection
# "When you empower something else, empower me. Disempower me, exhaust: Give a unit at a battlefield -2 might this
# turn."
card("Mel, Soul's Reflection", legend_event=_empower_echo("Mel, Soul's Reflection"), abilities=[ability(
    "-2 might", None, exhaust=True, preds=[P_unit_bf], can=_empowered, extra_cost=_disempower_me,
    choices=lambda g, pid, o: tg_choices(sorted(
        [u for u in g.units() if u.loc in (0, 1) and g.targetable(u, pid)],
        key=lambda u: (u.ctrl == pid, -value(g, u), u.uid))),
    resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), -2))])


# ---------------------------------------------------------------------- Ivern, Green Father
# "When you conquer or hold, you may exhaust me to replace that battlefield with a Brush battlefield token."
# (Game.replace_battlefield, rule 438; the Brush token is in battlefields.py)
def _ivern(g, pid, ev, info):
    if ev in ("conquer", "hold") and info["pid"] == pid:
        g.queue_trigger(pid, "Ivern, Green Father",
                        lambda g_, it: g_.replace_battlefield(g_.bfs[it.data["bf"]], "Brush"),
                        dict(bf=info["bf"]), src=_leg(g, pid).uid, may=True, cost=_exhaust_me)


card("Ivern, Green Father", legend_event=_ivern)


# ---------------------------------------------------------------------- Jhin, Virtuoso
# "When you play a spell, if you spent 4 energy or more, you may banish it. Then, if there are four spells banished
# with me, put each in its trash, channel 4 runes, and draw 1." (the spells banished with Jhin: legend.jhin)
def _jhin_res(g, it):
    leg = _leg(g, it.ctrl)
    c = it.data["card"]
    if c.zone == "trash" and c.oid == it.data["oid"]:
        g.to_zone(c, "banish", by=it.ctrl)
        leg.jhin = getattr(leg, "jhin", []) + [(c, c.oid)]
    with_me = [(x, oid) for x, oid in getattr(leg, "jhin", []) if x.zone == "banish" and x.oid == oid]
    leg.jhin = with_me
    if len(with_me) >= 4:
        leg.jhin = []
        for x, _ in with_me:
            g.to_zone(x, "trash", by=it.ctrl)
        g.channel(it.ctrl, 4)
        g.draw(it.ctrl, 1)


def _jhin(g, pid, ev, info):
    it = info.get("item")
    if ev == "played" and info["pid"] == pid and info["card"].spec["type"] == "Spell" and it is not None \
            and it.data.get("paid_e", 0) >= 4:
        c = info["card"]
        g.queue_trigger(pid, "Jhin, Virtuoso", _jhin_res, dict(card=c, oid=c.oid), src=_leg(g, pid).uid, may=True)


card("Jhin, Virtuoso", legend_event=_jhin)


# ---------------------------------------------------------------------- Master Yi, Wuju Master
# "[Level 6][>] Your units have +1 might. [Level 11][>] Your units enter ready."
card("Master Yi, Wuju Master",
     aura_might=lambda g, src, o: 1 if o.ctrl == src.owner and o.spec["type"] == "Unit" and g.p[src.owner].xp >= 6
     else 0,
     units_enter_ready=lambda g, src, pid, c: pid == src.owner and c.spec["type"] == "Unit" and g.p[pid].xp >= 11)


# ---------------------------------------------------------------------- Nasus, Curator of the Sands
# "When you play a unit, gear, or activated ability with Energy cost 7 energy or more, you may exhaust me to ready
# up to 2 runes." (the printed Energy cost, rule 206)
def _nasus_res(g, it):
    for _ in range(2):
        ex = sorted([r for r in g.p[it.ctrl].runes if r.exhausted], key=lambda r: (r.domain, r.uid))
        if not ex:
            return
        r = g.ask(it.ctrl, "nasus_ready_rune", ex + [None])
        if r is None:
            return
        r.exhausted = False


def _nasus(g, pid, ev, info):
    if info.get("pid") != pid:
        return
    if (ev == "played" and info["card"].spec["type"] in ("Unit", "Gear") and info["card"].spec["e"] >= 7) or \
            (ev == "activated" and (info.get("cost_e") or 0) >= 7):
        g.queue_trigger(pid, "Nasus, Curator of the Sands", _nasus_res, src=_leg(g, pid).uid, may=True,
                        cost=_exhaust_me)


card("Nasus, Curator of the Sands", legend_event=_nasus)


# ---------------------------------------------------------------------- Renata Glasc, Chem-Baroness
# "When you or an ally hold, you may exhaust me to play a Gold gear token exhausted. While your score is within 3
# points of the Victory Score, your Gold [ADD] an additional 1 energy." (1v1: no ally)
def _renata(g, pid, ev, info):
    if ev == "hold" and info["pid"] == pid:
        g.queue_trigger(pid, "Renata Glasc, Chem-Baroness",
                        lambda g_, it: make_token(g_, "Gold", it.ctrl, "base", ready=False),
                        src=_leg(g, pid).uid, may=True, cost=_exhaust_me)


card("Renata Glasc, Chem-Baroness", legend_event=_renata,
     gold_extra=lambda g, src, pid: 1 if pid == src.owner and g.victory - g.p[pid].points <= 3 else 0)


# ---------------------------------------------------------------------- Renekton, Butcher of the Sands
# "[Reaction][>] 2 runes of any type, exhaust: [Add] 2 energy. Spend this Energy only to play units or activated
# abilities of units." (an [Add] with a resource cost: Game.plan_convert 'p2e'; the rest floats restricted)
def _renekton_only(g, pid, ctx):
    if not ctx:
        return False
    if ctx.get("kind") == "unit":
        return True
    o = ctx.get("obj")
    return ctx.get("kind") == "ability" and o is not None and getattr(o, "zone", None) == "board" \
        and o.spec["type"] == "Unit"


card("Renekton, Butcher of the Sands",
     add=[dict(conv="p2e", n=2, exhaust=True, only=_renekton_only,
               can=lambda g, pid, o, ctx: _renekton_only(g, pid, ctx))])


# ---------------------------------------------------------------------- Sett, The Boss
# "When a buffed unit you control would die, you may pay 1 rune of any type and exhaust me to spend its buff and
# recall it exhausted instead. When you conquer, ready me." (recall doesn't heal: rule 455)
def _sett_rep(g, src, u):
    pid = src.owner
    if u.ctrl != pid or u.buff <= 0 or src.exhausted or not g.can_pay(pid, 0, [ANY]):
        return None

    def cost(g_):
        leg = _leg(g_, pid)
        if leg.exhausted or not g_.pay(pid, 0, [ANY]):
            return False
        leg.exhausted = True
        return True

    def apply(g_, x):
        g_.spend_buff(x, pid)
        g_.save_unit(x, heal=False)
    return dict(name="Sett, The Boss", apply=apply, may=True, cost=cost)


def _sett(g, pid, ev, info):
    if ev == "conquer" and info["pid"] == pid:
        g.queue_trigger(pid, "Sett, The Boss", lambda g_, it: g_.ready_obj(_leg(g_, it.ctrl)), src=_leg(g, pid).uid)


card("Sett, The Boss", death_rep=_sett_rep, legend_event=_sett)


# ---------------------------------------------------------------------- Sivir, Battle Mistress
# "When you recycle a rune, you may exhaust me to play a Gold gear token exhausted. When one or more enemy units
# die, ready me." (the deaths of one kill trigger it once, like Leona)
def _sivir(g, pid, ev, info):
    if ev == "rune_recycle" and info["pid"] == pid:
        g.queue_trigger(pid, "Sivir, Battle Mistress", lambda g_, it: make_token(g_, "Gold", it.ctrl, "base",
                                                                                   ready=False),
                        src=_leg(g, pid).uid, may=True, cost=_exhaust_me)
    elif ev == "die" and info["info"]["spec"]["type"] == "Unit" and info["info"]["ctrl"] != pid:
        if any(t.ctrl == pid and t.name == "Sivir, Battle Mistress (ready)" for t in g.trigq):
            return
        g.queue_trigger(pid, "Sivir, Battle Mistress (ready)", lambda g_, it: g_.ready_obj(_leg(g_, it.ctrl)),
                        src=_leg(g, pid).uid)


card("Sivir, Battle Mistress", legend_event=_sivir)


# ---------------------------------------------------------------------- Teemo, Swift Scout
# "You may pay 1 energy to hide a card with [Hidden] instead of 1 rune of any type. 1 energy, exhaust: Put a Teemo
# unit you own into your hand from your Champion Zone or the board."
def _teemo_cands(g, pid):
    out = [c for c in g.p[pid].champ if c.spec["type"] == "Unit" and "Teemo" in c.spec["tags"]]
    out += sorted([u for u in g.board if u.owner == pid and u.spec["type"] == "Unit" and "Teemo" in g.tags(u)],
                  key=lambda u: (u.exhausted is False, u.uid))
    return out


def _teemo_res(g, it):
    c = g.obj(it.data["c"]) if it.data["zone"] == "board" else next(
        (x for x in g.p[it.ctrl].champ if x.uid == it.data["c"]), None)
    if c is None or c.oid != it.data["oid"]:
        return
    if it.data["zone"] == "board":
        g.to_zone(c, "hand")
    else:
        g.p[it.ctrl].champ.remove(c)
        c.zone = "hand"
        g.p[it.ctrl].hand.append(c)


card("Teemo, Swift Scout", hide_cost=lambda g, src, pid: [(1, [])] if pid == src.owner else [],
     abilities=[ability(
         "Teemo to hand", "1 energy", exhaust=True, can=lambda g, pid, o: bool(_teemo_cands(g, pid)),
         choices=lambda g, pid, o: [dict(c=c.uid, oid=c.oid, zone="board" if c in g.board else "champ")
                                    for c in _teemo_cands(g, pid)],
         resolve=_teemo_res)])


# ---------------------------------------------------------------------- Zed, Master of Shadows
# "When you banish a card you own, empower me. [Action][>] Disempower me, exhaust: Discard 1, then draw 1."
def _zed(g, pid, ev, info):
    if ev == "banish" and info.get("pid") == pid and info["card"].owner == pid:
        g.queue_trigger(pid, "Zed, Master of Shadows", lambda g_, it: g_.empower(_leg(g_, it.ctrl), by=it.ctrl),
                        src=_leg(g, pid).uid)


def _zed_res(g, it):
    h = sorted(g.p[it.ctrl].hand, key=lambda c: (c.spec["e"] + 2 * c.spec["p"], c.uid))
    if h:
        g.discard(it.ctrl, g.ask(it.ctrl, "zed_discard", h))
    g.draw(it.ctrl, 1)


card("Zed, Master of Shadows", legend_event=_zed, abilities=[ability(
    "Discard/draw", None, timing="action", exhaust=True, can=_empowered, extra_cost=_disempower_me,
    resolve=_zed_res)])
