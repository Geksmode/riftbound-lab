"""Card behaviours: one Impl per card of the Akali / LeBlanc pool (85 cards: the 68 main/sideboard cards of the
12 tournament lists, legends, runes, battlefields), plus tokens. Each entry follows the card text quoted above it.

Hooks
  timing        None | 'action' | 'reaction'      (spells and abilities)
  hidden, ambush, accelerate, quickdraw, flow=(e, p, domains), repeat=(e, domains, p)
  kw            static keywords {'Tank':1, 'Assault':1, ...}
  choices(g, pid, ctx) -> [choice]   spells: targets as uids in choice['tg'] (ordered best first for the AI)
  preds         target predicates checked again on resolution (rule 359.3.e)
  resolve(g, item)                   spells
  on_play(g, obj, ctx)               "When you play me/this"
  deathknell(g, item)                item.data['info'] holds the look-back information
  on_event(g, obj, ev, info)         triggered abilities of a permanent
  might_mod(g, obj) -> int           passive Might modifiers
  untargetable(g, obj, by_pid)
  abilities                          activated abilities
  cost_mod(g, pid, card, choice) -> (energy reduction, power reduction)
  as_played(g, pid, card) -> [extra choice dicts]   optional additional costs
  bf_event / legend_event / effect_event / no_score
"""
from game import ANY, SPEC
from actions import play_card, attach


class Impl:
    def __init__(s, name, **kw):
        s.name = name
        s.timing = None
        s.hidden = s.ambush = s.accelerate = s.quickdraw = False
        s.flow = s.repeat = None
        s.kw = {}
        s.choices = s.resolve = s.on_play = s.deathknell = s.on_event = s.might_mod = None
        s.untargetable = s.cost_mod = s.as_played = None
        s.bf_event = s.legend_event = s.effect_event = s.no_score = None
        s.preds = None
        s.dk_choose = None
        s.abilities = []
        s.uncounterable = False
        s.banish_after = False
        s.max_choices = 8
        for k, v in kw.items():
            setattr(s, k, v)


IMPL = {}


def card(name, **kw):
    IMPL[name] = Impl(name, **kw)
    return IMPL[name]


# ====================================================================== helpers
def value(g, o):
    """Rough value of a unit, used only to order choices for the AI."""
    if o.spec["type"] != "Unit":
        return 2 + o.spec["e"] / 2
    v = g.might(o) + (o.spec["e"] + 2 * o.spec["p"]) / 3
    if o.cname in ("Karthus, Eternal",):
        v += 4
    if o.token and o.cname == "Reflection":
        v -= 1
    return v


def enemies(g, pid, at_bf=False, hidden_bf=None, targetable=True):
    us = [u for u in g.units(1 - pid) if (not at_bf or u.loc in (0, 1))]
    if hidden_bf is not None:
        us = [u for u in us if u.loc == hidden_bf]
    if targetable:
        us = [u for u in us if g.targetable(u, pid)]
    return sorted(us, key=lambda u: -value(g, u))


def friends(g, pid, at_bf=False, hidden_bf=None):
    us = [u for u in g.units(pid) if (not at_bf or u.loc in (0, 1))]
    if hidden_bf is not None:
        us = [u for u in us if u.loc == hidden_bf]
    return sorted(us, key=lambda u: -value(g, u))


def all_units(g, pid, at_bf=False, hidden_bf=None):
    us = [u for u in g.units() if (not at_bf or u.loc in (0, 1))]
    if hidden_bf is not None:
        us = [u for u in us if u.loc == hidden_bf]
    us = [u for u in us if g.targetable(u, pid)]
    return sorted(us, key=lambda u: (u.ctrl == pid, -value(g, u)))


def hb_ok(it, o):
    hb = it.data.get("hidden_bf")
    return hb is None or o.loc == hb


def P_unit(g, it, o):
    return o.spec["type"] == "Unit" and hb_ok(it, o)


def P_unit_bf(g, it, o):
    return o.spec["type"] == "Unit" and o.loc in (0, 1) and hb_ok(it, o)


def P_enemy(g, it, o):
    return o.spec["type"] == "Unit" and o.ctrl != it.ctrl and hb_ok(it, o)


def P_enemy_bf(g, it, o):
    return P_enemy(g, it, o) and o.loc in (0, 1)


def P_friend(g, it, o):
    return o.spec["type"] == "Unit" and o.ctrl == it.ctrl and hb_ok(it, o)


def P_friend_bf(g, it, o):
    return P_friend(g, it, o) and o.loc in (0, 1)


def P_gear(g, it, o):
    return o.spec["type"] == "Gear"


def tg_choices(units, n=1):
    return [dict(tg=(u.uid,)) for u in units]


def item_by_id(g, iid):
    for it in g.chain:
        if it.id == iid:
            return it
    return None


def trig_target(options_fn, pred=None, kind="target", optional=False):
    """Build a finalization-time chooser for a triggered ability with one target."""
    def choose(g, it):
        opts = options_fn(g, it)
        if not opts:
            return False
        o = g.ask(it.ctrl, kind, opts, item=it)
        if o is None:
            return False
        g.add_target(it, o, pred)
        return True
    return choose


def may_pay(e, reqs_fn=None):
    def cost(g, it):
        reqs = reqs_fn(g, it) if reqs_fn else []
        if not g.can_pay(it.ctrl, e, reqs):
            return False
        return g.pay(it.ctrl, e, reqs)
    return cost


def play_unit_free(g, pid, c, src, loc_choices=None, ignore_energy=False):
    """Play a unit as a limited action, ignoring its cost (or only its Energy cost)."""
    locs = loc_choices if loc_choices is not None else (["base"] + [b.idx for b in g.bfs if b.ctrl == pid])
    loc = g.ask(pid, "play_location", locs, card=c)
    ch = dict(loc=loc, free=not ignore_energy, ignore_energy=ignore_energy, pay_power=ignore_energy)
    return play_card(g, pid, c, src, ch, limited=True)


# ====================================================================== runes, legends
card("Fury Rune"); card("Calm Rune"); card("Mind Rune"); card("Order Rune")


# Akali, Rogue Assassin — "[Empower] [3][A]. [Action][>] [E]: If it's your turn, move a friendly unit in a
# showdown to base and if I'm [Empowered], ready it."
def _akali_retreat(g, it):
    u = g.legal(it, 0)
    if u is None or g.tp != it.ctrl:
        return
    g.move([u], "base", it.ctrl)
    leg = g.p[it.ctrl].legend
    if leg.empowered:
        g.ready_obj(u)


def _akali_retreat_choices(g, pid, leg):
    if g.sd is None or g.tp != pid:
        return []
    return [dict(tg=(u.uid,)) for u in friends(g, pid) if u.loc == g.sd.bf]


card("Akali, Rogue Assassin", abilities=[
    dict(name="Empower", timing="main", can=lambda g, pid, o: not o.empowered,
         cost=lambda g, pid, o, ch: (3, [ANY]), resolve=lambda g, it: setattr(g.p[it.ctrl].legend, "empowered", True)),
    dict(name="Retreat", timing="action", exhaust=True, can=lambda g, pid, o: g.sd is not None and g.tp == pid,
         choices=_akali_retreat_choices, cost=lambda g, pid, o, ch: (0, []),
         preds=[lambda g, it, o: o.ctrl == it.ctrl and g.sd is not None and o.loc == g.sd.bf],
         resolve=_akali_retreat),
])


# LeBlanc, Deceiver — "When you conquer or hold, you may discard 1 and exhaust me to play a ready Reflection unit
# token there. It becomes a copy of another unit there. Give it [Temporary]."
def _reflection(g, pid, loc, model):
    t = g.new_token("Reflection", pid)
    g.enter_board(t, pid, loc, ready=True)
    if model is not None:
        t.copy = model.cname                     # copyable traits incl. stats (Reflection ruling)
    g.grant(t, "Temporary", 1, None)
    g.log(f"  Reflection of {model.cname if model else 'nothing'} at {loc}")
    g.stats[f"reflection_P{pid}"] += 1
    return t


def _leblanc_legend(g, pid, ev, info):
    if ev not in ("conquer", "hold") or info["pid"] != pid:
        return
    bf = info["bf"]

    def cost(g_, it):
        pl = g_.p[it.ctrl]
        if pl.legend.exhausted or not pl.hand:
            return False
        c = g_.ask(it.ctrl, "discard", list(pl.hand), reason="leblanc")
        g_.to_zone(c, "trash")
        pl.legend.exhausted = True
        return True

    def choose(g_, it):
        us = [u for u in g_.units(loc=bf)]
        if not us:
            return False
        o = g_.ask(it.ctrl, "copy_target", sorted(us, key=lambda u: -value(g_, u)), item=it)
        g_.add_target(it, o, lambda g2, it2, o2: o2.loc == bf)
        return True

    def res(g_, it):
        model = g_.legal(it, 0)
        _reflection(g_, it.ctrl, bf, model)

    g.queue_trigger(pid, "LeBlanc, Deceiver", res, dict(bf=bf), may=True, choose=choose, cost=cost)


card("LeBlanc, Deceiver", legend_event=_leblanc_legend)


# ====================================================================== battlefields
def _bar(g, b, ev, info):
    # "When a unit moves from here, give it +1 might this turn."
    if ev == "move" and info["frm"] == b.idx:
        o = info["obj"]
        g.queue_trigger(o.ctrl, "Back-Alley Bar", lambda g_, it: (g_.obj(it.data["u"]) and
                        g_.mod(g_.obj(it.data["u"]), 1)), dict(u=o.uid))


card("Back-Alley Bar", bf_event=_bar)


def _dusk(g, b, ev, info):
    # "At the start of your Beginning Phase, you may kill a unit you control here to draw 1."
    if ev == "beginning_start":
        pid = info["pid"]
        if not g.units(pid, b.idx):
            return

        def res(g_, it):
            us = g_.units(it.ctrl, b.idx)
            if not us:
                return
            u = g_.ask(it.ctrl, "dusk_kill", [None] + us)
            if u is None:
                return
            g_.kill([u], it.ctrl, cost=True)
            g_.draw(it.ctrl, 1)
        g.queue_trigger(pid, "Dusk Rose Lab", res, may=True)


card("Dusk Rose Lab", bf_event=_dusk)
card("Forbidding Waste")                                # passive in Game.might
card("Forgotten Monument", no_score=lambda g, pid, b: g.p[pid].turns < 3)
card("Void Gate")                                       # Bonus Damage in Game.deal
card("Windswept Hillock")                               # Ganking in Game.kw_value
card("Aspirant's Climb")                                # victory score +1 in Game.__init__


def _sigil(g, b, ev, info):
    # "When you conquer here, recycle one of your runes."
    if ev == "conquer" and info["bf"] == b.idx:
        def res(g_, it):
            rs = g_.p[it.ctrl].runes
            if rs:
                r = g_.ask(it.ctrl, "recycle_rune", list(rs))
                g_.recycle_rune(it.ctrl, r)
        g.queue_trigger(info["pid"], "Sigil of the Storm", res)


card("Sigil of the Storm", bf_event=_sigil)


def _star_spring(g, b, ev, info):
    # "The first time a player plays a non-token unit here each turn, they may move another unit they control
    # here to its base."
    if ev == "played" and info["card"].spec["type"] == "Unit" and not info["card"].token and info["card"].loc == b.idx:
        key = ("star", b.idx, info["pid"], g.turn_no)
        if key in g.stats:
            return
        g.stats[key] = 1
        played = info["card"]

        def choose(g_, it):
            us = [u for u in g_.units(it.ctrl, b.idx) if u is not played]
            if not us:
                return False
            u = g_.ask(it.ctrl, "star_spring", us)
            g_.add_target(it, u, lambda g2, it2, o: o.loc == b.idx)
            return True

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.move([u], "base", it.ctrl)
        g.queue_trigger(info["pid"], "Star Spring", res, may=True, choose=choose)


card("Star Spring", bf_event=_star_spring)


def _targon(g, b, ev, info):
    # "When you conquer here, ready 2 runes at the end of this turn."
    if ev == "conquer" and info["bf"] == b.idx:
        pid = info["pid"]

        def res(g_, it):
            def at_end(g2, eff, inf):
                g2.effects.remove(eff)

                def ready2(g3, it3):
                    ex = [r for r in g3.p[it3.ctrl].runes if r.exhausted]
                    for r in ex[:2]:
                        r.exhausted = False
                g2.queue_trigger(pid, "Targon's Peak (end of turn)", ready2)
            if g_.stage in ("expire",):
                return
            g_.effects.append(dict(on="end_turn", fn=at_end, dur="turn", pid=pid))
        g.queue_trigger(pid, "Targon's Peak", res)


card("Targon's Peak", bf_event=_targon)


def _threshold(g, b, ev, info):
    # "When combat starts here, the attacker and defender each [Add] 1 energy." (Add resolves immediately)
    if ev == "combat_start" and info["bf"] == b.idx:
        sd = info["sd"]
        g.p[sd.attacker].pool_e += 1
        g.p[sd.defender].pool_e += 1
        g.log("  Threshold of the Gray: both players add 1 energy")


card("Threshold of the Gray", bf_event=_threshold)


# ====================================================================== tokens
card("Mech")
card("Reflection")
card("Gold")                                            # Add ability used while paying (Game.pay)


# ====================================================================== Akali deck
def _dw_event(g, o, ev, info):
    # Akali, Deadly Weapon — "When I move, you may deal 1 to a unit at a battlefield I moved to or from.
    # If I'm [Empowered], deal 2 instead."
    if ev == "move" and info["obj"] is o:
        locs = [l for l in (info["frm"], info["to"]) if l in (0, 1)]
        if not locs:
            return
        dmg = 2 if o.empowered else 1

        def opts(g_, it):
            return [u for u in all_units(g_, it.ctrl) if u.loc in locs]

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.deal(u, it.data["dmg"], "ability", it.ctrl)
        g.queue_trigger(o.ctrl, "Akali, Deadly Weapon", res, dict(dmg=dmg), src=o.uid, may=True,
                        choose=trig_target(opts, lambda g_, it, u: u.loc in locs))


card("Akali, Deadly Weapon", on_event=_dw_event,
     might_mod=lambda g, o: 1 if o.empowered else 0,
     abilities=[dict(name="Empower", timing="main", can=lambda g, pid, o: not o.empowered,
                     cost=lambda g, pid, o, ch: (2, [frozenset({"Fury"})]),
                     resolve=lambda g, it: g.obj(it.src) is not None and setattr(g.obj(it.src), "empowered", True))])


def _silent_event(g, o, ev, info):
    # Akali, Silent — "When I move to a battlefield, give me +2 might this turn."
    if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
        g.queue_trigger(o.ctrl, "Akali, Silent", lambda g_, it: g_.obj(it.src) is not None and g_.mod(g_.obj(it.src), 2),
                        src=o.uid)


card("Akali, Silent", on_event=_silent_event,
     untargetable=lambda g, o, by: by != o.ctrl and not g.in_combat(o))


def _adaptatron(g, o, ev, info):
    # "When I conquer, you may kill a gear. If you do, buff me."
    if ev == "conquer" and info["pid"] == o.ctrl and o in info["units"]:
        def res(g_, it):
            gg = g_.legal(it, 0)
            if gg is not None and g_.kill([gg], it.ctrl):
                me = g_.obj(it.src)
                if me is not None:
                    g_.buff(me)
        g.queue_trigger(o.ctrl, "Adaptatron", res, src=o.uid, may=True,
                        choose=trig_target(lambda g_, it: sorted(g_.gear(), key=lambda x: x.ctrl == it.ctrl), P_gear))


card("Adaptatron", on_event=_adaptatron)


# Against the Odds — "[Reaction] Give a friendly unit at a battlefield +2 might this turn for each enemy unit there."
def _ato(g, it):
    u = g.legal(it, 0)
    if u is not None:
        n = len([e for e in g.units(1 - it.ctrl, u.loc)])
        g.mod(u, 2 * n)


card("Against the Odds", timing="reaction", preds=[P_friend_bf], resolve=_ato,
     choices=lambda g, pid, ctx: tg_choices([u for u in friends(g, pid, True, ctx["hidden_bf"])
                                             if g.units(1 - pid, u.loc)]))


def _heron_event(g, o, ev, info):
    # Astral Heron — "When you play your first card each turn, if I'm at a battlefield, your next card costs
    # [2] and [A][A] less."
    if ev == "played" and info["pid"] == o.ctrl and info["n"] == 1 and o.loc in (0, 1) and not info["card"].token:
        pid = o.ctrl
        g.queue_trigger(pid, "Astral Heron", lambda g_, it: g_.effects.append(dict(kind="heron", pid=pid)), src=o.uid)


card("Astral Heron", on_event=_heron_event)


# Back Off — "[Hidden] [Action] [Stun] a unit. If you played this from your hand, draw 1."
def _back_off(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.stun(u, it.ctrl)
    if it.data.get("from") == "hand":
        g.draw(it.ctrl, 1)


card("Back Off", timing="action", hidden=True, preds=[P_unit], resolve=_back_off,
     choices=lambda g, pid, ctx: tg_choices([u for u in enemies(g, pid, False, ctx["hidden_bf"]) if not u.stunned]))


# Blitzcrank, Impassive — "[Tank] When you play me to a battlefield, you may move an enemy unit to here.
# When I hold, return me to my owner's hand."
def _blitz_play(g, o, ctx):
    if o.loc not in (0, 1):
        return
    here = o.loc

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.move([u], here, it.ctrl)
    g.queue_trigger(o.ctrl, "Blitzcrank", res, src=o.uid, may=True,
                    choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc != here],
                                       lambda g_, it, u: u.ctrl != it.ctrl))


def _blitz_event(g, o, ev, info):
    if ev == "hold" and info["pid"] == o.ctrl and o in info["units"]:
        g.queue_trigger(o.ctrl, "Blitzcrank (hold)", lambda g_, it: g_.obj(it.src) is not None and
                        g_.to_zone(g_.obj(it.src), "hand"), src=o.uid)


card("Blitzcrank, Impassive", kw={"Tank": 1}, on_play=_blitz_play, on_event=_blitz_event)


# Block — "[Hidden] [Action] Give a unit [Shield 3] and [Tank] this turn."
def _block(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.grant(u, "Shield", 3)
        g.grant(u, "Tank", 1)


card("Block", timing="action", hidden=True, preds=[P_unit], resolve=_block,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid, False, ctx["hidden_bf"])))


# Brittle Steel — "Kill a gear. [Flow] [4][Fury]"
def _kill_gear(g, it):
    gg = g.legal(it, 0)
    if gg is not None:
        g.kill([gg], it.ctrl)


card("Brittle Steel", preds=[P_gear], resolve=_kill_gear, flow=(4, 1, ("Fury",)),
     choices=lambda g, pid, ctx: tg_choices(sorted(g.gear(), key=lambda x: (x.ctrl == pid, -x.spec["e"]))))


# Charm — "Move an enemy unit."
def _charm(g, it):
    u = g.legal(it, 0)
    if u is not None and u.loc != it.data["dest"]:
        g.move([u], it.data["dest"], it.ctrl)


def _charm_choices(g, pid, ctx):
    out = []
    for u in enemies(g, pid):
        for d in ("base", 0, 1):
            if d != u.loc:
                out.append(dict(tg=(u.uid,), dest=d))
    out.sort(key=lambda c: (c["dest"] != "base",))
    return out


card("Charm", preds=[P_enemy], resolve=_charm, choices=_charm_choices, max_choices=10)


# Crumbling Sands — "[Reaction] Counter a spell if an opponent has played another spell this turn."
def _crumbling(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is None:
        return
    opp = 1 - it.ctrl
    n = g.spells_played[opp] - (1 if tgt.ctrl == opp else 0)
    if n >= 1:
        g.counter(tgt)


card("Crumbling Sands", timing="reaction", resolve=_crumbling,
     choices=lambda g, pid, ctx: [dict(item=i.id) for i in g.chain if i.kind == "spell" and i.ctrl != pid])


# Darius, Trifarian — "When you play your second card in a turn, give me +2 might this turn and ready me."
def _darius(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and info["n"] == 2:
        def res(g_, it):
            me = g_.obj(it.src)
            if me is not None:
                g_.mod(me, 2)
                g_.ready_obj(me)
        g.queue_trigger(o.ctrl, "Darius", res, src=o.uid)


card("Darius, Trifarian", on_event=_darius)


# Decree of Focus — "[Reaction] Choose a friendly unit that's in combat with an enemy Fury unit or that's being
# chosen by an enemy Fury spell. Give it +4 might this turn."
def _dof_ok(g, pid, u):
    if g.in_combat(u) and any("Fury" in e.spec["domains"] for e in g.units(1 - pid, u.loc) if e.desig):
        return True
    return any(i.kind == "spell" and i.ctrl != pid and "Fury" in i.card.spec["domains"] and u.uid in i.chosen for i in g.chain)


card("Decree of Focus", timing="reaction", preds=[P_friend],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 4),
     choices=lambda g, pid, ctx: tg_choices([u for u in friends(g, pid) if _dof_ok(g, pid, u)]))


# Decree of Insight — "[Reaction] Ignore [Deflect] while paying this spell's cost. Give an enemy Body unit -5
# might this turn."
card("Decree of Insight", timing="reaction",
     preds=[lambda g, it, o: P_enemy(g, it, o) and "Body" in o.spec["domains"]],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), -5),
     choices=lambda g, pid, ctx: tg_choices([u for u in enemies(g, pid) if "Body" in u.spec["domains"]]))


# Decree of Rage — "[Action] This can't be countered. Deal 4 to an enemy Calm unit."
card("Decree of Rage", timing="action", uncounterable=True,
     preds=[lambda g, it, o: P_enemy(g, it, o) and "Calm" in o.spec["domains"]],
     resolve=lambda g, it: g.deal(g.legal(it, 0), 4, "spell", it.ctrl),
     choices=lambda g, pid, ctx: tg_choices([u for u in enemies(g, pid) if "Calm" in u.spec["domains"]]))


# Decree of Unity — "Kill an enemy Chaos unit or gear."
card("Decree of Unity",
     preds=[lambda g, it, o: o.ctrl != it.ctrl and "Chaos" in o.spec["domains"]],
     resolve=lambda g, it: g.kill([g.legal(it, 0)], it.ctrl),
     choices=lambda g, pid, ctx: tg_choices([o for o in g.board if o.ctrl != pid and "Chaos" in o.spec["domains"]]))


# Defy — "[Reaction] Counter a spell that costs no more than 4 energy and no more than 1 rune of any type."
def _defy(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is not None and tgt.kind == "spell" and tgt.card.spec["e"] <= 4 and tgt.card.spec["p"] <= 1:
        g.counter(tgt)


card("Defy", timing="reaction", resolve=_defy,
     choices=lambda g, pid, ctx: [dict(item=i.id) for i in reversed(g.chain) if i.kind == "spell" and i.ctrl != pid
                                  and i.card.spec["e"] <= 4 and i.card.spec["p"] <= 1 and not i.uncounterable])


# Disarming Rake — "When you play me, you may kill a gear."
def _rake(g, o, ctx):
    g.queue_trigger(o.ctrl, "Disarming Rake", _kill_gear, src=o.uid, may=True,
                    choose=trig_target(lambda g_, it: sorted(g_.gear(), key=lambda x: x.ctrl == it.ctrl), P_gear))


card("Disarming Rake", on_play=_rake)


# Discipline — "[Reaction] Give a unit +2 might this turn. Draw 1."
def _discipline(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.mod(u, 2)
    g.draw(it.ctrl, 1)


card("Discipline", timing="reaction", preds=[P_unit], resolve=_discipline,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid)))


# En Garde — "[Reaction] Give a friendly unit +1 might this turn, then an additional +1 might this turn if it is
# the only unit you control there."
def _en_garde(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.mod(u, 1)
        if g.alone(u):
            g.mod(u, 1)


card("En Garde", timing="reaction", preds=[P_friend], resolve=_en_garde,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid)))


# Falling Star — "Deal 3 to a unit. Deal 3 to a unit."
def _falling_star(g, it):
    for i in range(2):
        u = g.legal(it, i)
        if u is not None:
            g.deal(u, 3, "spell", it.ctrl)


def _fs_choices(g, pid, ctx):
    es = enemies(g, pid)[:4]
    out = []
    for i, a in enumerate(es):
        for b in es[i:]:
            out.append(dict(tg=(a.uid, b.uid)))
    def score(c):
        a, b = g.obj(c["tg"][0]), g.obj(c["tg"][1])
        if a is b:
            return -(value(g, a) if g.might(a) - a.damage > 3 else value(g, a) - 3)
        return -(value(g, a) * (g.might(a) - a.damage <= 3) + value(g, b) * (g.might(b) - b.damage <= 3))
    out.sort(key=score)
    return out


card("Falling Star", preds=[P_unit, P_unit], resolve=_falling_star, choices=_fs_choices)


# Ferrous Forerunner — "[Deathknell] Play two 3 might Mech unit tokens to your base."
def _forerunner_dk(g, it):
    for _ in range(2):
        t = g.new_token("Mech", it.ctrl)
        g.enter_board(t, it.ctrl, "base")


card("Ferrous Forerunner", deathknell=_forerunner_dk)


# Irelia, Fervent — "[Deflect] When you choose or ready me, give me +1 might this turn."
def _irelia(g, o, ev, info):
    if (ev == "chosen" and info["obj"] is o and info["item"].ctrl == o.ctrl) or (ev == "ready" and info["obj"] is o):
        g.queue_trigger(o.ctrl, "Irelia", lambda g_, it: g_.obj(it.src) is not None and g_.mod(g_.obj(it.src), 1),
                        src=o.uid)


card("Irelia, Fervent", kw={"Deflect": 1}, on_event=_irelia)


# Kai'Sa, Survivor — "[Accelerate] When I conquer, draw 1."
def _kaisa(g, o, ev, info):
    if ev == "conquer" and info["pid"] == o.ctrl and o in info["units"]:
        g.queue_trigger(o.ctrl, "Kai'Sa", lambda g_, it: g_.draw(it.ctrl, 1), src=o.uid)


card("Kai'Sa, Survivor", accelerate=True, on_event=_kaisa)


# Lonely Poro — "[Deathknell] If I died alone, draw 1."
card("Lonely Poro", deathknell=lambda g, it: it.data["info"]["alone"] and g.draw(it.ctrl, 1))


# Long Sword / Sterak's Gage — "[Quick-Draw] [Equip] [Fury]/[Calm]" ; Pendulum Blade — "[Equip] [Fury]"
def equip_ability(domain):
    return dict(name="Equip", timing="main",
                choices=lambda g, pid, o: [dict(tg=(u.uid,)) for u in friends(g, pid) if u.uid != o.attached_to],
                cost=lambda g, pid, o, ch: (0, [frozenset({domain})]),
                preds=[P_friend],
                resolve=lambda g, it: (g.legal(it, 0) is not None and g.obj(it.src) is not None
                                       and attach(g, g.obj(it.src), g.legal(it, 0))))


card("Long Sword", timing="reaction", quickdraw=True, abilities=[equip_ability("Fury")])
card("Sterak's Gage", timing="reaction", quickdraw=True, abilities=[equip_ability("Calm")])


def _pendulum_effect(g, gear, unit, ev, info):
    # Effect text (attached): the equipped unit gets +2 might this turn when it moves to a battlefield.
    if ev == "move" and info["obj"] is unit and info["to"] in (0, 1):
        g.queue_trigger(unit.ctrl, "Pendulum Blade", lambda g_, it: g_.obj(it.src) is not None and
                        g_.mod(g_.obj(it.src), 2), src=unit.uid)


card("Pendulum Blade", abilities=[equip_ability("Fury")], effect_event=_pendulum_effect)


# Mischievous Marai — "[Hidden] When you play me to a battlefield, deal 2 to an enemy unit here."
def _marai(g, o, ctx):
    if o.loc not in (0, 1):
        return
    here = o.loc

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.deal(u, 2, "ability", it.ctrl)
    g.queue_trigger(o.ctrl, "Mischievous Marai", res, src=o.uid,
                    choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                                       lambda g_, it, u: u.loc == here and u.ctrl != it.ctrl))


card("Mischievous Marai", hidden=True, on_play=_marai)


# Mournful Witness — "When a combat that I was in ends, empower me. [Empowered] I have +2 might."
def _witness(g, o, ev, info):
    if ev == "combat_end" and o in info["members"]:
        g.queue_trigger(o.ctrl, "Mournful Witness", lambda g_, it: g_.obj(it.src) is not None and
                        setattr(g_.obj(it.src), "empowered", True), src=o.uid)


card("Mournful Witness", on_event=_witness, might_mod=lambda g, o: 2 if o.empowered else 0)


# Not So Fast — "[Reaction] Counter an enemy spell or ability that chooses a friendly unit or gear."
def _nsf_items(g, pid):
    mine = set(o.uid for o in g.board if o.ctrl == pid)
    return [i for i in reversed(g.chain) if i.ctrl != pid and (i.chosen & mine) and not i.uncounterable]


def _nsf(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is not None:
        g.counter(tgt)


card("Not So Fast", timing="reaction", resolve=_nsf,
     choices=lambda g, pid, ctx: [dict(item=i.id) for i in _nsf_items(g, pid)])


# Noxus Hopeful — "[Legion] I cost 2 energy less."
card("Noxus Hopeful", cost_mod=lambda g, pid, c, ch: (2, 0) if g.finalized[pid] else (0, 0))


# Scuttle Crab — "When you play me, draw 1. [Deathknell] Choose an opponent. They reveal their hand. You can look
# at their facedown cards this turn. Gain 1 XP."
def _crab_dk(g, it):
    g.p[1 - it.ctrl].revealed_turn = g.turn_no
    g.p[it.ctrl].xp += 1


card("Scuttle Crab", on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Scuttle Crab", lambda g_, it: g_.draw(it.ctrl, 1),
                                                                src=o.uid),
     deathknell=_crab_dk)


# Shuriken Flip — "Deal 2 to up to one enemy unit at a battlefield, then move a friendly unit.
# [Flow] [3][A]"
def _flip(g, it):
    if it.data.get("tg") and it.data["tg"][0] is not None:
        u = g.legal(it, 0)
        if u is not None:
            g.deal(u, 2, "spell", it.ctrl)
    m = g.obj(it.data.get("mover")) if it.data.get("mover") else None
    if m is not None and m.ctrl == it.ctrl and m.oid == it.data.get("mover_oid") and m.loc != it.data["dest"]:
        if it.data.get("hidden_bf") is None or True:
            g.move([m], it.data["dest"], it.ctrl)


def _flip_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    es = enemies(g, pid, True, hb)[:3]
    tgs = [(e.uid,) for e in es] + [()]
    fr = friends(g, pid)
    if hb is not None:
        fr = [u for u in fr if u.loc == hb] or fr
    moves = []
    for u in fr:
        for d in ("base", 0, 1):
            if d != u.loc:
                moves.append((u.uid, u.oid, d))
    out = []
    for t in tgs:
        for (m, oid, d) in moves:
            out.append(dict(tg=t, mover=m, mover_oid=oid, dest=d))
        if not moves:
            out.append(dict(tg=t, mover=None, dest=None))

    def score(c):
        s = 0
        if c["tg"]:
            e = g.obj(c["tg"][0])
            s -= value(g, e) * (g.might(e) - e.damage <= 2) + 1
        if c.get("mover"):
            m = g.obj(c["mover"])
            if c["dest"] in (0, 1) and g.bfs[c["dest"]].ctrl != pid:
                s -= 2 + g.might(m) / 2
            if m.cname in ("Akali, Deadly Weapon", "Stellacorn Herder"):
                s -= 1
        return s
    out.sort(key=score)
    return out


card("Shuriken Flip", flow=(3, 1, tuple(sorted(("Fury", "Calm", "Mind", "Body", "Chaos", "Order")))),
     preds=[P_enemy_bf], resolve=_flip, choices=_flip_choices, max_choices=12)


# Sky Splitter — "[Action] This spell's Energy cost is reduced by the highest Might among units you control.
# Deal 5 to a unit at a battlefield."
card("Sky Splitter", timing="action", preds=[P_unit_bf],
     cost_mod=lambda g, pid, c, ch: (max([g.might(u) for u in g.units(pid)] + [0]), 0),
     resolve=lambda g, it: g.deal(g.legal(it, 0), 5, "spell", it.ctrl),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# Stellacorn Herder — "When I move, draw 1."
def _herder(g, o, ev, info):
    if ev == "move" and info["obj"] is o:
        g.queue_trigger(o.ctrl, "Stellacorn Herder", lambda g_, it: g_.draw(it.ctrl, 1), src=o.uid)


card("Stellacorn Herder", on_event=_herder)


# Tomb-Raider Barbara — "When you play me, if you control 7 or more runes, choose an enemy gear. If it's
# [Empowered], disempower it. Otherwise, kill it."
def _barbara(g, o, ctx):
    if len(g.p[o.ctrl].runes) < 7:
        return

    def res(g_, it):
        gg = g_.legal(it, 0)
        if gg is None:
            return
        if gg.empowered:
            gg.empowered = False
        else:
            g_.kill([gg], it.ctrl)
    g.queue_trigger(o.ctrl, "Tomb-Raider Barbara", res, src=o.uid,
                    choose=trig_target(lambda g_, it: [x for x in g_.gear() if x.ctrl != it.ctrl],
                                       lambda g_, it, x: x.ctrl != it.ctrl))


card("Tomb-Raider Barbara", on_play=_barbara)


# Zhonya's Hourglass — "[Hidden] If a friendly unit would die, kill this instead. Heal that unit, exhaust it,
# and recall it." (errata; the replacement effect is in Game.kill)
card("Zhonya's Hourglass", hidden=True)


# ====================================================================== LeBlanc deck
# Baited Hook — "[1][Order], [E]: Kill a friendly unit. Look at the top 5 cards of your Main Deck. You may banish
# a unit from among them that has Might up to 1 more than the killed unit and play it, ignoring its cost.
# Then recycle the rest."
def _hook(g, it):
    pid = it.ctrl
    u = g.legal(it, 0)
    killed_might = None
    if u is not None:
        killed_might = g.might(u)
        g.kill([u], pid)
    pl = g.p[pid]
    top = pl.deck[:5]
    if killed_might is not None:
        cands = [c for c in top if c.spec["type"] == "Unit" and (c.spec["might"] or 0) <= killed_might + 1]
    else:
        cands = []
    pick = g.ask(pid, "hook_pick", [None] + sorted(cands, key=lambda c: -(c.spec["might"] or 0) - c.spec["e"] / 10))
    rest = [c for c in top if c is not pick]
    if pick is not None:
        g.log(f"  Baited Hook finds {pick.cname}")
        g.stats["hook_hit"] += 1
        pl.deck.remove(pick)
        pick.zone = "banish"
        pl.banish.append(pick)
        play_unit_free(g, pid, pick, "banish")
    for c in rest:
        if c in pl.deck:
            pl.deck.remove(c)
    g.recycle_cards(pid, rest)


card("Baited Hook", abilities=[dict(
    name="Hook", timing="main", exhaust=True, preds=[P_friend],
    choices=lambda g, pid, o: [dict(tg=(u.uid,)) for u in sorted(g.units(pid), key=lambda u: value(g, u))],
    cost=lambda g, pid, o, ch: (1, [frozenset({"Order"})]), resolve=_hook)])


# Bellows Breath — "[Action] [Repeat] [1][Mind] Deal 1 to up to three units at the same location."
def _bellows(g, it):
    groups = [[g.legal(it, i) for i in range(len(it.targets))]]
    if it.data.get("t2"):
        groups.append([g.obj(u) if g.obj(u) is not None and g.obj(u).oid == oid else None for u, oid in it.data["t2"]])
    for grp in groups:
        grp = [u for u in grp if u is not None]
        if not grp:
            continue
        loc = max(set(u.loc for u in grp), key=lambda l: sum(1 for u in grp if u.loc == l))
        for u in grp:
            if u.loc == loc:
                g.deal(u, 1, "spell", it.ctrl)


def _bellows_choices(g, pid, ctx):
    out = []
    locs = set(u.loc for u in enemies(g, pid, False, ctx["hidden_bf"]))
    for l in sorted(locs, key=str):
        es = [u for u in enemies(g, pid) if u.loc == l][:3]
        out.append(dict(tg=tuple(u.uid for u in es)))
    out.sort(key=lambda c: -sum(1 for u in c["tg"] if g.might(g.obj(u)) - g.obj(u).damage <= 1))
    return out


card("Bellows Breath", timing="action", repeat=(1, ("Mind",), 1), preds=[P_unit], resolve=_bellows,
     choices=_bellows_choices)


# Black Rose Dignitary — "[Assault] [Deathknell] Channel 1 rune exhausted."
card("Black Rose Dignitary", kw={"Assault": 1}, deathknell=lambda g, it: g.channel(it.ctrl, 1, exhausted=True))


# Chakram Dancer — "[Ambush] When you play me, give your other units here [Shield] this turn."
def _chakram(g, o, ctx):
    here = o.loc

    def res(g_, it):
        for u in g_.units(it.ctrl, here):
            if u.uid != it.src:
                g_.grant(u, "Shield", 1)
    g.queue_trigger(o.ctrl, "Chakram Dancer", res, src=o.uid)


card("Chakram Dancer", ambush=True, on_play=_chakram)


# Cull the Weak — "Each player kills one of their units."
def _cull(g, it):
    victims = []
    for pid in (g.tp, 1 - g.tp):
        us = g.units(pid)
        if us:
            victims.append(g.ask(pid, "sacrifice", sorted(us, key=lambda u: value(g, u))))
    g.kill(victims)


card("Cull the Weak", resolve=_cull)


# Deathgrip — "[Reaction] Kill a friendly unit to give +might equal to its Might to another friendly unit this
# turn. Draw 1."
def _deathgrip(g, it):
    tgt = g.legal(it, 0)
    victims = [u for u in g.units(it.ctrl) if u is not tgt]
    if victims:
        v = g.ask(it.ctrl, "deathgrip_victim", [None] + sorted(victims, key=lambda u: value(g, u)), target=tgt)
        if v is not None:
            m = g.might(v)
            killed = g.kill([v], it.ctrl, cost=True)
            if killed and tgt is not None and tgt in g.board:
                g.mod(tgt, max(0, m))
    g.draw(it.ctrl, 1)


card("Deathgrip", timing="reaction", preds=[P_friend], resolve=_deathgrip,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid)) + [dict(tg=())])


# Glasc Mixologist — "[Deathknell] You may play a unit with cost no more than 3 energy and no more than 1 rune of
# any type from your trash, ignoring its cost."
def _mixo(g, it):
    pid = it.ctrl
    cands = [c for c in g.p[pid].trash if c.spec["type"] == "Unit" and c.spec["e"] <= 3 and c.spec["p"] <= 1]
    pick = g.ask(pid, "mixologist_pick", [None] + sorted(cands, key=lambda c: -(c.spec["might"] or 0) - c.spec["e"]))
    if pick is not None:
        play_unit_free(g, pid, pick, "trash")


card("Glasc Mixologist", deathknell=_mixo)


# Harnessed Dragon — "When you play me, kill an enemy unit."
def _dragon(g, o, ctx):
    hb = ctx.get("hidden_bf")
    g.queue_trigger(o.ctrl, "Harnessed Dragon", lambda g_, it: g_.kill([g_.legal(it, 0)], it.ctrl), src=o.uid,
                    choose=trig_target(lambda g_, it: enemies(g_, it.ctrl, False, hb), P_enemy))


card("Harnessed Dragon", on_play=_dragon)


# Hidden Blade — "[Hidden] [Action] Kill a unit at a battlefield. Its controller draws 2."
def _blade(g, it):
    u = g.legal(it, 0)
    if u is None:
        return                                      # linked instruction ignored too (rule 359.3.e.14.a)
    c = u.ctrl
    g.kill([u], it.ctrl)
    g.draw(c, 2)


card("Hidden Blade", timing="action", hidden=True, preds=[P_unit_bf], resolve=_blade,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# Honest Broker — "[Deathknell] Play a Gold gear token exhausted."
def _broker(g, it):
    t = g.new_token("Gold", it.ctrl)
    g.enter_board(t, it.ctrl, "base", ready=False)


card("Honest Broker", deathknell=_broker)


# Karthus, Eternal — "Your [Deathknell] effects trigger an additional time." (in Game.kill)
card("Karthus, Eternal")


# Kennen, Keeper of Balance — "[Hidden] When you play me or I attack, you may pay [2] to [Stun] a unit.
# While there's a stunned enemy unit here, I have +2 might."
def _kennen_trigger(g, o, hb=None):
    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.stun(u, it.ctrl)
    g.queue_trigger(o.ctrl, "Kennen", res, src=o.uid, may=True,
                    choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl, False, hb) if not u.stunned]
                                       + [u for u in friends(g_, it.ctrl) if hb is None or u.loc == hb][:0], P_unit),
                    cost=may_pay(2))


card("Kennen, Keeper of Balance", hidden=True,
     on_play=lambda g, o, ctx: _kennen_trigger(g, o, ctx.get("hidden_bf")),
     on_event=lambda g, o, ev, info: ev == "attack" and info["obj"] is o and _kennen_trigger(g, o),
     might_mod=lambda g, o: 2 if any(u.stunned for u in g.units(1 - o.ctrl, o.loc)) else 0)


# Ki Barrier — "[Reaction] Choose a unit. Prevent the next 7 damage that would be dealt to it this turn."
def _ki(g, it):
    u = g.legal(it, 0)
    if u is not None:
        u.prevent += 7


card("Ki Barrier", timing="reaction", preds=[P_unit], resolve=_ki,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid)))


# LeBlanc, Everywhere At Once — "[Backline] Your [Temporary] effects at my battlefield don't trigger."
card("LeBlanc, Everywhere At Once", kw={"Backline": 1})


# LeBlanc, Fragmented — "[Assault] [Deathknell] Draw 1. If it's your Beginning Phase, draw 2 instead."
card("LeBlanc, Fragmented", kw={"Assault": 1},
     deathknell=lambda g, it: g.draw(it.ctrl, 2 if (g.tp == it.ctrl and g.stage in ("beginning", "scoring")) else 1))


# Mirror Image — "Choose a unit. Play a ready Reflection unit token to your base. It becomes a copy of that unit.
# Give it [Temporary]."
def _mirror(g, it):
    u = g.legal(it, 0)
    _reflection(g, it.ctrl, "base", u)


card("Mirror Image", preds=[P_unit], resolve=_mirror,
     choices=lambda g, pid, ctx: tg_choices(sorted([u for u in g.units() if g.targetable(u, pid)],
                                                   key=lambda u: -value(g, u))))


# Rift Herald — "When I move to a battlefield, look at the top 3 cards of your Main Deck. You may reveal a unit
# from among them and draw it. Recycle the rest. [Deathknell] Play a unit from your hand to your base, ignoring
# its Energy cost. (You must still pay its Power cost.)"
def _herald_move(g, o, ev, info):
    if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
        def res(g_, it):
            pl = g_.p[it.ctrl]
            top = pl.deck[:3]
            us = [c for c in top if c.spec["type"] == "Unit"]
            pick = g_.ask(it.ctrl, "herald_pick", [None] + us)
            rest = [c for c in top if c is not pick]
            if pick is not None:
                pl.deck.remove(pick); pick.zone = "hand"; pl.hand.append(pick)
            for c in rest:
                pl.deck.remove(c)
            g_.recycle_cards(it.ctrl, rest)
        g.queue_trigger(o.ctrl, "Rift Herald", res, src=o.uid)


def _herald_dk(g, it):
    pid = it.ctrl
    cands = [c for c in g.p[pid].hand if c.spec["type"] == "Unit"
             and g.can_pay(pid, 0, g.power_reqs(c.spec["domains"], c.spec["p"]))]
    pick = g.ask(pid, "herald_dk_pick", sorted(cands, key=lambda c: -c.spec["e"]))
    if pick is not None:
        play_unit_free(g, pid, pick, "hand", loc_choices=["base"], ignore_energy=True)


card("Rift Herald", on_event=_herald_move, deathknell=_herald_dk)


# Ruined Rex — "[Deathknell] Deal 4 to an enemy unit."
def _rex_dk(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.deal(u, 4, "ability", it.ctrl)


IMPL_REX = card("Ruined Rex", deathknell=_rex_dk,
                dk_choose=trig_target(lambda g_, it: enemies(g_, it.ctrl), P_enemy))

# Sacrifice — "[Reaction] As an additional cost to play this, kill a friendly [Mighty] unit. Draw 2 and channel 1
# rune exhausted."
card("Sacrifice", timing="reaction",
     choices=lambda g, pid, ctx: [dict(kill=u.uid) for u in sorted(g.units(pid), key=lambda u: value(g, u)) if g.mighty(u)],
     resolve=lambda g, it: (g.draw(it.ctrl, 2), g.channel(it.ctrl, 1, exhausted=True)))


# Safety Inspector — "You may spend 3 XP as an additional cost to play me. When you play me, each player must kill
# one of their units. If you paid my additional cost, you don't kill a unit this way."
def _inspector(g, o, ctx):
    paid = ctx.get("xp")

    def res(g_, it):
        victims = []
        for pid in (g_.tp, 1 - g_.tp):
            if pid == it.ctrl and paid:
                continue
            us = g_.units(pid)
            if us:
                victims.append(g_.ask(pid, "sacrifice", sorted(us, key=lambda u: value(g_, u))))
        g_.kill(victims)
    g.queue_trigger(o.ctrl, "Safety Inspector", res, src=o.uid)


card("Safety Inspector", on_play=_inspector,
     as_played=lambda g, pid, c: [dict()] + ([dict(xp=True)] if g.p[pid].xp >= 3 else []))


# Salvage — "[Action] You may kill a gear. Draw 1."
def _salvage(g, it):
    if it.targets:
        gg = g.legal(it, 0)
        if gg is not None:
            g.kill([gg], it.ctrl)
    g.draw(it.ctrl, 1)


card("Salvage", timing="action", preds=[P_gear], resolve=_salvage,
     choices=lambda g, pid, ctx: tg_choices([x for x in g.gear() if x.ctrl != pid]) + [dict(tg=())])


# Soaring Scout — "[Deathknell] Channel 1 rune exhausted."
card("Soaring Scout", deathknell=lambda g, it: g.channel(it.ctrl, 1, exhausted=True))

# Stupefy — "[Reaction] Give a unit -1 might this turn, to a minimum of 1 might. Draw 1."
card("Stupefy", timing="reaction", preds=[P_unit],
     resolve=lambda g, it: (g.legal(it, 0) is not None and g.mod(g.legal(it, 0), -1, minimum=1), g.draw(it.ctrl, 1)),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid)))


# Thousand-Tailed Watcher — "[Accelerate] When you play me, give enemy units -3 might this turn, to a minimum of 1."
def _watcher(g, o, ctx):
    def res(g_, it):
        for u in g_.units(1 - it.ctrl):
            g_.mod(u, -3, minimum=1)
    g.queue_trigger(o.ctrl, "Thousand-Tailed Watcher", res, src=o.uid)


card("Thousand-Tailed Watcher", accelerate=True, on_play=_watcher)


# Time Warp — "Take a turn after this one. Banish this."
card("Time Warp", banish_after=True, resolve=lambda g, it: g.extra_turns.insert(0, it.ctrl))


# Turn to Dust — "Give a gear [Temporary]."
card("Turn to Dust", preds=[P_gear], resolve=lambda g, it: g.legal(it, 0) is not None and g.grant(g.legal(it, 0), "Temporary", 1, None),
     choices=lambda g, pid, ctx: tg_choices([x for x in g.gear() if x.ctrl != pid]))


# Vi, Peacekeeper — "[Ambush] When I attack, [Stun] an enemy unit here."
def _vi(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc
        g.queue_trigger(o.ctrl, "Vi", lambda g_, it: g_.stun(g_.legal(it, 0), it.ctrl), src=o.uid,
                        choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                                           lambda g_, it, u: u.loc == here and u.ctrl != it.ctrl))


card("Vi, Peacekeeper", ambush=True, on_event=_vi)

# Watchful Sentry — "[Deathknell] Draw 1."
card("Watchful Sentry", deathknell=lambda g, it: g.draw(it.ctrl, 1))


# Ashe, Focused — "When you play me, choose an opponent. They reveal their hand. Choose a card revealed this way
# and banish it. When they hold, return it to their hand (even if I'm no longer on the board)."
def _ashe(g, o, ctx):
    def res(g_, it):
        opp = 1 - it.ctrl
        hand = g_.p[opp].hand
        if not hand:
            return
        c = g_.ask(it.ctrl, "ashe_pick", sorted(hand, key=lambda x: -(x.spec["e"] + 2 * x.spec["p"])))
        g_.to_zone(c, "banish")

        def back(g2, eff, info):
            if info["pid"] == opp and c in g2.p[opp].banish:
                g2.effects.remove(eff)
                g2.p[opp].banish.remove(c)
                c.zone = "hand"; g2.p[opp].hand.append(c)
        g_.effects.append(dict(on="hold", fn=back))
    g.queue_trigger(o.ctrl, "Ashe", res, src=o.uid)


card("Ashe, Focused", on_play=_ashe)


# Atakhan — "You may kill a friendly unit as an additional cost to play me. If you do, I cost [1] less for each
# Energy it costs and [Order] less for each Power it costs. [Ganking] When I attack, the defender must kill one of
# their units here."
def _atakhan_cost(g, pid, c, ch):
    k = g.obj(ch["kill"]) if ch.get("kill") else None
    if k is None:
        return (0, 0)
    return (k.spec["e"] if not k.token or k.copy else 0, k.spec["p"] if not k.token or k.copy else 0)


def _atakhan_event(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc

        def res(g_, it):
            d = 1 - it.ctrl
            us = g_.units(d, here)
            if us:
                g_.kill([g_.ask(d, "sacrifice", sorted(us, key=lambda u: value(g_, u)))], d)
        g.queue_trigger(o.ctrl, "Atakhan", res, src=o.uid)


card("Atakhan", kw={"Ganking": 1}, cost_mod=_atakhan_cost, on_event=_atakhan_event,
     as_played=lambda g, pid, c: [dict()] + [dict(kill=u.uid) for u in sorted(g.units(pid), key=lambda u: value(g, u))[:3]])


def check():
    """Every card name of the pool must have an implementation."""
    import re
    names = [l.split(" | ")[0][3:] for l in open(__file__.replace("cards.py", "pool.txt"), encoding="utf-8") if l.startswith("## ")]
    return [n for n in names if n not in IMPL]
