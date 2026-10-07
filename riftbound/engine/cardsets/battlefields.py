"""Batch battlefields: the battlefields of batches/battlefields.txt. See GUIDE.md.

Conventions used here
- A battlefield's abilities are controlled by its controller, or by the turn player while it is uncontrolled
  (rule 190.6.a/b). Abilities that name a player ("When you conquer here", "they may", "that player") are
  controlled by / act for that player.
- "When you defend here" is a player-level defend trigger (rule 383.4.f): it triggers once per combat, when the
  first unit of that player gains the Defender designation at this battlefield.
- Cards that need an engine hook are not registered: see NEEDS_battlefields.md.
"""
from cards import *  # noqa: F401,F403
from game import ANY as _ANY


def _bf_ctrl(g, b):
    """Controller of a battlefield ability that names no player (rule 190.6)."""
    return b.ctrl if b.ctrl is not None else g.tp


def _here(b):
    return lambda g, it, o: o.loc == b.idx


def _first_defend(g, b, info):
    """True the first time a unit of a player defends at b in the current combat (rule 383.4.f.2.a)."""
    sd = g.sd
    o = info["obj"]
    if info["bf"] != b.idx or sd is None:
        return False
    seen = getattr(sd, "_bf_defend_seen", None)
    if seen is None:
        seen = set()
        sd._bf_defend_seen = seen
    key = (b.idx, o.ctrl)
    if key in seen:
        return False
    seen.add(key)
    return True


# ---------------------------------------------------------------------------------------------- Abandoned Hall
# "When a player plays a spell, they may give a unit they control here +1 might this turn."
# (a spell is played when it resolves, rule 419.4.a: engine event 'played')
def _abandoned_hall(g, b, ev, info):
    if ev != "played" or info["card"].spec["type"] != "Spell":
        return
    pid = info["pid"]
    if not g.units(pid, b.idx):
        return

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.mod(u, 1)
    g.queue_trigger(pid, "Abandoned Hall", res, may=True,
                    choose=trig_target(lambda g_, it: friends(g_, it.ctrl, False, b.idx),
                                       lambda g_, it, o: o.ctrl == it.ctrl and o.loc == b.idx))


card("Abandoned Hall", bf_event=_abandoned_hall)


# ---------------------------------------------------------------------------------------------- Altar to Unity
# "When you hold here, play a 1 might Recruit unit token in your base."
def _altar_unity(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "Altar to Unity",
                        lambda g_, it: make_token(g_, "Recruit", it.ctrl, "base"))


card("Altar to Unity", bf_event=_altar_unity)


# ---------------------------------------------------------------------------------------------- Amateur Recital
# "When you hold here, you may move a unit at a battlefield to its base."
def _recital(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.move([u], "base", it.ctrl)
        g.queue_trigger(info["pid"], "Amateur Recital", res, may=True,
                        choose=trig_target(lambda g_, it: all_units(g_, it.ctrl, True),
                                           lambda g_, it, o: o.loc in (0, 1), deflect=True))


card("Amateur Recital", bf_event=_recital)


# ---------------------------------------------------------------------------------------------- Black Flame Altar
# "Units here with [Temporary] have [Shield]."
_BFA_GUARD = set()


def _bfa_aura(g, b, o):
    if o.loc != b.idx or o.spec["type"] != "Unit" or o.uid in _BFA_GUARD:
        return None
    _BFA_GUARD.add(o.uid)            # this aura gives no Temporary: don't evaluate it while checking Temporary
    try:
        t = g.has_kw(o, "Temporary")
    finally:
        _BFA_GUARD.discard(o.uid)
    return {"Shield": 1} if t else None


card("Black Flame Altar", aura_kw=_bfa_aura)


# ---------------------------------------------------------------------------------------------- Emperor's Dais
# "When you conquer here, you may pay 1 energy and return a unit you control here to its owner's hand. If you do,
# play a 2 might Sand Soldier unit token here."  (costs within instructions, rule 355.10.c.1: not a target)
def _dais(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx:
        here = b.idx

        def res(g_, it):
            us = sorted(g_.units(it.ctrl, here), key=lambda u: value(g_, u))
            if not us or not g_.can_pay(it.ctrl, 1, []):
                return
            if not g_.ask(it.ctrl, "may", [True, False], item=it):
                return
            u = g_.ask(it.ctrl, "dais_return", us)
            if u is None or not g_.pay(it.ctrl, 1, []):
                return
            g_.to_zone(u, "hand")
            make_token(g_, "Sand Soldier", it.ctrl, here)
        g.queue_trigger(info["pid"], "Emperor's Dais", res)


card("Emperor's Dais", bf_event=_dais)


# ---------------------------------------------------------------------------------------------- Fortified Position
# "When you defend here, choose a unit. It gains [Shield 2] this combat."
def _fortified(g, b, ev, info):
    if ev == "defend" and _first_defend(g, b, info):
        pid = info["obj"].ctrl

        def opts(g_, it):
            mine = [u for u in friends(g_, it.ctrl) if u.loc == b.idx]
            rest = [u for u in all_units(g_, it.ctrl) if u not in mine]
            return mine + sorted(rest, key=lambda u: u.ctrl != it.ctrl)

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is None:
                return
            gr = ["Shield", 2, "turn"]
            u.grants.append(gr)

            def end(g2, eff, inf):        # "this combat": the grant ends with the combat
                if eff in g2.effects:
                    g2.effects.remove(eff)
                x = g2.obj(eff["uid"])
                if x is not None:
                    for k in list(x.grants):
                        if k is eff["grant"]:
                            x.grants.remove(k)
            g_.effects.append(dict(on="combat_end", fn=end, dur="turn", uid=u.uid, grant=gr))
        g.queue_trigger(pid, "Fortified Position", res,
                        choose=trig_target(opts, None, kind="fortified_shield", deflect=True))


card("Fortified Position", bf_event=_fortified)


# ---------------------------------------------------------------------------------------------- Frozen Fortress
# "At the start of each player's Beginning Phase, deal 1 to each unit here. (This happens before scoring.)"
def _frozen(g, b, ev, info):
    if ev == "beginning_start":
        def res(g_, it):
            for u in list(g_.units(loc=b.idx)):
                g_.deal(u, 1, "ability", it.ctrl)
        g.queue_trigger(_bf_ctrl(g, b), "Frozen Fortress", res)


card("Frozen Fortress", bf_event=_frozen)


# ---------------------------------------------------------------------------------------------- Grove of the God-Willow
# "When you hold here, draw 1."
def _grove(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "Grove of the God-Willow", lambda g_, it: g_.draw(it.ctrl, 1))


card("Grove of the God-Willow", bf_event=_grove)


# ---------------------------------------------------------------------------------------------- Hall of Legends
# "When you conquer here, you may pay 1 energy to ready your legend."
def _hall(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "Hall of Legends", lambda g_, it: g_.ready_obj(g_.p[it.ctrl].legend),
                        may=True, cost=may_pay(1))


card("Hall of Legends", bf_event=_hall)


# ---------------------------------------------------------------------------------------------- Hallowed Tomb
# "When you hold here, you may return your Chosen Champion from your trash to your Champion Zone if it is empty."
def _champ_uid(g, pid):
    """uid of pid's Chosen Champion card. It is noted the first time this battlefield sees an event: the first
    event of a game (Beginning Phase of turn 1) precedes every play, so the champion is still in its zone."""
    return getattr(g.p[pid], "bf_chosen_champion", None)


def _hallowed(g, b, ev, info):
    for pl in g.p:
        if not hasattr(pl, "bf_chosen_champion"):
            pl.bf_chosen_champion = pl.champ[0].uid if pl.champ else None
    if ev == "hold" and info["bf"] == b.idx:
        pid = info["pid"]

        def find(g_, p):
            uid = _champ_uid(g_, p)
            return next((c for c in g_.p[p].trash if c.uid == uid), None) if uid is not None else None

        def res(g_, it):
            pl = g_.p[it.ctrl]
            c = find(g_, it.ctrl)
            if c is None or pl.champ:
                return
            pl.trash.remove(c)
            c.reset()
            c.oid += 1
            c.zone = "champ"
            pl.champ.append(c)
            g_.log(f"  Hallowed Tomb returns {c} to the Champion Zone")
        if find(g, pid) is not None and not g.p[pid].champ:
            g.queue_trigger(pid, "Hallowed Tomb", res, may=True)


card("Hallowed Tomb", bf_event=_hallowed)


# ---------------------------------------------------------------------------------------------- Kinkou Temple
# "Units here with [Tank] have +1 might."
card("Kinkou Temple",
     aura_might=lambda g, b, o: 1 if o.loc == b.idx and o.spec["type"] == "Unit" and g.has_kw(o, "Tank") else 0)


# ---------------------------------------------------------------------------------------------- Minefield
# "When you conquer here, put the top 2 cards of your Main Deck into your trash."  (= burning, rules 440.1, 431.1.b)
def _minefield(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "Minefield", lambda g_, it: g_.burn(it.ctrl, 2))


card("Minefield", bf_event=_minefield)


# ---------------------------------------------------------------------------------------------- Monastery of Hirana
# "When you conquer here, you may spend a buff to draw 1."  (spend a buff: rule 702.2.b, a unit you control;
# cost within instructions, rule 355.10.c.1)
def _monastery(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx:
        def cost(g_, it):
            us = [u for u in g_.units(it.ctrl) if u.buff > 0]
            if not us:
                return False
            u = g_.ask(it.ctrl, "spend_buff", sorted(us, key=lambda x: value(g_, x)))
            if u is None:
                return False
            g_.spend_buff(u, it.ctrl)
            return True
        g.queue_trigger(info["pid"], "Monastery of Hirana", lambda g_, it: g_.draw(it.ctrl, 1), may=True, cost=cost)


card("Monastery of Hirana", bf_event=_monastery)


# ---------------------------------------------------------------------------------------------- Navori Fighting Pit
# "When you hold here, buff a unit here."
def _navori(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        def opts(g_, it):
            us = [u for u in g_.units(loc=b.idx) if g_.targetable(u, it.ctrl)]
            return sorted(us, key=lambda u: (u.ctrl != it.ctrl, u.buff > 0, -value(g_, u)))

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.buff(u)
        g.queue_trigger(info["pid"], "Navori Fighting Pit", res,
                        choose=trig_target(opts, _here(b), kind="buff_target", deflect=True))


card("Navori Fighting Pit", bf_event=_navori)


# ---------------------------------------------------------------------------------------------- Obelisk of Power
# "At the start of each player's first Beginning Phase, that player channels 1 rune."
def _obelisk(g, b, ev, info):
    if ev == "beginning_start" and g.p[info["pid"]].turns == 1:
        g.queue_trigger(_bf_ctrl(g, b), "Obelisk of Power", lambda g_, it: g_.channel(it.data["who"], 1),
                        dict(who=info["pid"]))


card("Obelisk of Power", bf_event=_obelisk)


# ---------------------------------------------------------------------------------------------- Power Nexus
# "When you hold here, you may pay 4 runes of any type to score 1 point."  (not a conquer point: no Final Point
# restriction, rule 471.1.a.1)
def _nexus(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "Power Nexus", lambda g_, it: g_.gain_point(it.ctrl, "Power Nexus"),
                        may=True, cost=may_pay(0, lambda g_, it: [_ANY] * 4))


card("Power Nexus", bf_event=_nexus)


# ---------------------------------------------------------------------------------------------- Protective Sands
# "When you conquer here, if you control 4 or fewer runes, you may pay 1 energy to draw 1."
def _sands(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx and len(g.p[info["pid"]].runes) <= 4:
        def res(g_, it):
            if len(g_.p[it.ctrl].runes) <= 4:        # intervening "if", checked again on resolution
                g_.draw(it.ctrl, 1)
        g.queue_trigger(info["pid"], "Protective Sands", res, may=True, cost=may_pay(1))


card("Protective Sands", bf_event=_sands)


# ---------------------------------------------------------------------------------------------- Ravenbloom Conservatory
# "When you defend here, reveal the top card of your Main Deck. If it's a spell, put it in your hand. Otherwise,
# recycle it."  (empty deck: nothing is revealed, no burn out, rule 431.1.c)
def _ravenbloom(g, b, ev, info):
    if ev == "defend" and _first_defend(g, b, info):
        def res(g_, it):
            top = g_.reveal(it.ctrl, 1)
            if not top:
                return
            c = top[0]
            if c.spec["type"] == "Spell":
                g_.to_zone(c, "hand")
            else:
                g_.recycle_cards(it.ctrl, [c])
        g.queue_trigger(info["obj"].ctrl, "Ravenbloom Conservatory", res)


card("Ravenbloom Conservatory", bf_event=_ravenbloom)


# ---------------------------------------------------------------------------------------------- Reaver's Row
# "When you defend here, you may move a friendly unit here to base."
def _reaver(g, b, ev, info):
    if ev == "defend" and _first_defend(g, b, info):
        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.move([u], "base", it.ctrl)
        g.queue_trigger(info["obj"].ctrl, "Reaver's Row", res, may=True,
                        choose=trig_target(lambda g_, it: sorted(friends(g_, it.ctrl, False, b.idx),
                                                                 key=lambda u: value(g_, u)),
                                           lambda g_, it, o: o.ctrl == it.ctrl and o.loc == b.idx,
                                           kind="reaver_move"))


card("Reaver's Row", bf_event=_reaver)


# ---------------------------------------------------------------------------------------------- Reckoner's Arena
# "When you hold here, activate the conquer effects of units here."  Rule 383.4.g: each conquer effect of a unit
# here is checked as if the conquer part of its condition were fulfilled and, if so, put on the chain as if it had
# just triggered. Conquer effects of units = their on_event handlers for 'conquer' and [Hunt] ("When I conquer or
# hold", rule 823). Only the holding player's units can be here at that point.
def _reckoner(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        def res(g_, it):
            pid = it.ctrl
            units = [u for u in g_.units(pid, b.idx)]
            inf = dict(pid=pid, bf=b.idx, units=list(units))
            for u in units:
                im = g_.impl(u)
                if im is not None and im.on_event is not None and u in g_.board:
                    im.on_event(g_, u, "conquer", inf)
            g_.hunt(pid, units)
        g.queue_trigger(info["pid"], "Reckoner's Arena", res)


card("Reckoner's Arena", bf_event=_reckoner)


# ---------------------------------------------------------------------------------------------- Seat of Power
# "When you conquer here, draw 1 for each other battlefield you or allies control."  (1v1: no allies)
def _seat(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx:
        def res(g_, it):
            n = sum(1 for x in g_.bfs if x.idx != b.idx and x.ctrl == it.ctrl)
            if n:
                g_.draw(it.ctrl, n)
        g.queue_trigger(info["pid"], "Seat of Power", res)


card("Seat of Power", bf_event=_seat)


# ---------------------------------------------------------------------------------------------- Shadow Temple
# "When you hold here, [Burn 3]."
def _shadow(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "Shadow Temple", lambda g_, it: g_.burn(it.ctrl, 3))


card("Shadow Temple", bf_event=_shadow)


# ---------------------------------------------------------------------------------------------- Startipped Peak
# "When you hold here, you may channel 1 rune exhausted."
def _peak(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "Startipped Peak", lambda g_, it: g_.channel(it.ctrl, 1, exhausted=True),
                        may=True)


card("Startipped Peak", bf_event=_peak)


# ---------------------------------------------------------------------------------------------- Sunken Temple
# "When you conquer here with one or more [Mighty] units, you may pay 1 energy to draw 1."
def _sunken(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx and any(g.mighty(u) for u in info["units"]):
        g.queue_trigger(info["pid"], "Sunken Temple", lambda g_, it: g_.draw(it.ctrl, 1), may=True, cost=may_pay(1))


card("Sunken Temple", bf_event=_sunken)


# ---------------------------------------------------------------------------------------------- The Arena's Greatest
# "At the start of each player's first Beginning Phase, that player gains 1 point."  (rule 190.6.b example)
def _arena(g, b, ev, info):
    if ev == "beginning_start" and g.p[info["pid"]].turns == 1:
        g.queue_trigger(_bf_ctrl(g, b), "The Arena's Greatest",
                        lambda g_, it: g_.gain_point(it.data["who"], "The Arena's Greatest"), dict(who=info["pid"]))


card("The Arena's Greatest", bf_event=_arena)


# ---------------------------------------------------------------------------------------------- The Candlelit Sanctum
# "When you conquer here, look at the top two cards of your Main Deck. You may recycle one or both of them. Put
# those you don't back in any order."  (the procedure of Predict 2, rule 436; looking never burns out, 431.1.c)
def _candlelit(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "The Candlelit Sanctum", lambda g_, it: g_.predict(it.ctrl, 2))


card("The Candlelit Sanctum", bf_event=_candlelit)


# ---------------------------------------------------------------------------------------------- The Dreaming Tree
# "When a player chooses a friendly unit here with a spell for the first time each turn, they draw 1."
# (targeting trigger, rule 383.4.b: when the spell is finalized)
def _dreaming(g, b, ev, info):
    if ev != "chosen":
        return
    it, o = info["item"], info["obj"]
    if it.kind != "spell" or o.loc != b.idx or o.ctrl != it.ctrl or o.spec["type"] != "Unit":
        return
    key = ("dreaming_tree", b.idx, it.ctrl, g.turn_no)
    if g.stats[key]:
        return
    g.stats[key] = 1
    g.queue_trigger(it.ctrl, "The Dreaming Tree", lambda g_, it_: g_.draw(it_.ctrl, 1))


card("The Dreaming Tree", bf_event=_dreaming)


# ---------------------------------------------------------------------------------------------- The Grand Plaza
# "When you hold here, if you have 7+ units here, you win the game."
def _plaza(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx and len(g.units(info["pid"], b.idx)) >= 7:
        def res(g_, it):
            if len(g_.units(it.ctrl, b.idx)) >= 7:
                g_.log(f"P{it.ctrl} wins with The Grand Plaza")
                g_.win(it.ctrl)
        g.queue_trigger(info["pid"], "The Grand Plaza", res)


card("The Grand Plaza", bf_event=_plaza)


# ---------------------------------------------------------------------------------------------- The Papertree
# "When you hold here, each player channels 1 rune exhausted."
def _papertree(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        def res(g_, it):
            for p in (g_.tp, 1 - g_.tp):
                g_.channel(p, 1, exhausted=True)
        g.queue_trigger(info["pid"], "The Papertree", res)


card("The Papertree", bf_event=_papertree)


# ---------------------------------------------------------------------------------------------- Trapping Grounds
# "When you conquer here, if you assigned 3 or more excess damage, play a 1 might Bird unit token with [Deflect]."
# Excess damage = combat damage assigned to a unit beyond its lethal damage (Game.lethal_need). It is measured on
# the 'damaged' events of the combat here: assigned - lethal = dealt - max(1, might - previous damage).
# The token names no location: it is played to a location it can be played to (rule 187 / play rules): base or a
# battlefield its controller controls.
def _trapping(g, b, ev, info):
    sd = g.sd
    if ev == "damaged" and info["kind"] == "combat" and sd is not None and sd.bf == b.idx:
        o, n = info["obj"], info["amount"]
        prior = o.damage - n
        ex = n - max(1, g.might(o) - prior)
        if ex > 0:
            d = getattr(sd, "_bf_excess", None)
            if d is None:
                d = {}
                sd._bf_excess = d
            d[info["by"]] = d.get(info["by"], 0) + ex
    elif ev == "conquer" and info["bf"] == b.idx:
        pid = info["pid"]
        if sd is None or sd.bf != b.idx or getattr(sd, "_bf_excess", {}).get(pid, 0) < 3:
            return

        def res(g_, it):
            locs = ["base"] + [x.idx for x in g_.bfs if x.ctrl == it.ctrl]
            loc = g_.ask(it.ctrl, "play_location", locs)
            make_token(g_, "Bird", it.ctrl, loc)
        g.queue_trigger(pid, "Trapping Grounds", res)


card("Trapping Grounds", bf_event=_trapping)


# ---------------------------------------------------------------------------------------------- Treasure Hoard
# "When you conquer here, you may pay 1 energy to play a Gold gear token exhausted."
def _hoard(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "Treasure Hoard",
                        lambda g_, it: make_token(g_, "Gold", it.ctrl, "base", ready=False), may=True, cost=may_pay(1))


card("Treasure Hoard", bf_event=_hoard)


# ---------------------------------------------------------------------------------------------- Trifarian War Camp
# "Units here have +1 might. (This includes attackers.)"
card("Trifarian War Camp", aura_might=lambda g, b, o: 1 if o.loc == b.idx and o.spec["type"] == "Unit" else 0)


# ---------------------------------------------------------------------------------------------- Valley of Idols
# "When a player plays a unit here, they may pay 1 energy to [Buff] it."
def _valley(g, b, ev, info):
    c = info.get("card") if ev == "played" else None
    if c is not None and c.spec["type"] == "Unit" and c.loc == b.idx and c in g.board:
        def res(g_, it):
            u = g_.obj(it.data["uid"])
            if u is not None and u.oid == it.data["oid"]:
                g_.buff(u)
        g.queue_trigger(info["pid"], "Valley of Idols", res, dict(uid=c.uid, oid=c.oid), may=True, cost=may_pay(1))


card("Valley of Idols", bf_event=_valley)


# ---------------------------------------------------------------------------------------------- Veiled Temple
# "When you conquer here, you may ready a friendly gear. If it's an Equipment, you may detach it."
def _detach(g, gear):
    u = g.obj(gear.attached_to) if gear.attached_to is not None else None
    if u is not None and gear.uid in u.attached:
        u.attached.remove(gear.uid)
    gear.attached_to = None
    g.log(f"  {gear} is detached")
    g.need_cleanup = True                  # unattached gear at a battlefield is recalled (rule 323, step 5)


def _veiled(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx:
        def opts(g_, it):
            return sorted(g_.gear(it.ctrl), key=lambda x: (not x.exhausted, -x.spec["e"]))

        def res(g_, it):
            x = g_.legal(it, 0)
            if x is None:
                return
            g_.ready_obj(x)
            if "Equipment" in x.spec["tags"] and x.attached_to is not None:
                if g_.ask(it.ctrl, "veiled_detach", [False, True], gear=x):
                    _detach(g_, x)
        g.queue_trigger(info["pid"], "Veiled Temple", res, may=True,
                        choose=trig_target(opts, lambda g_, it, o: o.ctrl == it.ctrl and o.spec["type"] == "Gear",
                                           kind="ready_gear"))


card("Veiled Temple", bf_event=_veiled)


# ---------------------------------------------------------------------------------------------- Zaun Warrens
# "When you conquer here, discard 1, then draw 1."
def _zaun(g, b, ev, info):
    if ev == "conquer" and info["bf"] == b.idx:
        def res(g_, it):
            h = g_.p[it.ctrl].hand
            if h:
                g_.discard(it.ctrl, g_.ask(it.ctrl, "discard", list(h), reason="zaun_warrens"))
            g_.draw(it.ctrl, 1)
        g.queue_trigger(info["pid"], "Zaun Warrens", res)


card("Zaun Warrens", bf_event=_zaun)


# ====================================================================== battlefields using the engine hooks (integration)
from actions import attach, card_kw     # noqa: E402


def _ctrl_is(b, pid):
    return b.ctrl == pid


# Brush (battlefield token, rule 187.8) — "Bird, Cat, Dog, Poro, and Ivern units here have +1 might. When you score
# here, you may replace this with the battlefield it replaced." (created by Ivern, Green Father:
# Game.replace_battlefield / swap_back, rule 438)
_BRUSH_TAGS = frozenset({"Bird", "Cat", "Dog", "Poro", "Ivern"})


def _brush_event(g, b, ev, info):
    if ev in ("conquer", "hold") and info["bf"] == b.idx and b.replaced is not None:
        def res(g_, it):
            bb = g_.bfs[it.data["bf"]]
            if bb.name == "Brush":
                g_.swap_back(bb)
        g.queue_trigger(info["pid"], "Brush", res, dict(bf=b.idx), may=True)


card("Brush", bf_event=_brush_event,
     aura_might=lambda g, b, o: 1 if o.loc == b.idx and o.spec["type"] == "Unit" and g.tags(o) & _BRUSH_TAGS else 0)


# Marai Spire — "While you control this battlefield, friendly [Repeat] costs cost 1 energy less."
card("Marai Spire", cost_aura=lambda g, b, pid, what: [dict(e=-1, on="rep")]
     if _ctrl_is(b, pid) and what["kind"] == "spell" else [])


# Mystic Vortex — "During showdowns here, cards with [Reaction] cost 1 rune of any type more to play. (Hidden cards
# have [Reaction].)"
def _vortex(g, b, pid, what):
    c = what.get("card")
    if c is None or what["kind"] == "ability" or g.sd is None or g.sd.bf != b.idx:
        return []
    im = g.impl(c)
    src = what.get("src")
    react = src == "facedown" or (im is not None and im.timing == "reaction") or \
        card_kw(g, pid, c, "Reaction", src) or (c.spec["type"] == "Gear" and card_kw(g, pid, c, "Quick-Draw", src))
    return [dict(p=[_ANY])] if react else []


card("Mystic Vortex", cost_aura=_vortex)


# Ornn's Forge — "While you control this battlefield, the first friendly non-token gear played each turn costs 1
# energy less." (Game.hist['gear_played'])
card("Ornn's Forge", cost_aura=lambda g, b, pid, what: [dict(e=-1)]
     if _ctrl_is(b, pid) and what["kind"] == "gear" and what["card"] is not None and not what["card"].token
     and g.hist["gear_played"][pid] == 0 else [])


# Piltovan Forge — "While you control this battlefield, the first friendly gear activated ability played each turn
# costs 1 energy less." (Game.hist['gear_abs'])
card("Piltovan Forge", cost_aura=lambda g, b, pid, what: [dict(e=-1)]
     if _ctrl_is(b, pid) and what["kind"] == "ability" and what.get("obj") is not None
     and getattr(what["obj"], "spec", None) is not None and what["obj"].spec["type"] == "Gear"
     and g.hist["gear_abs"][pid] == 0 else [])


# Risen Altar — "[Empower] costs of your units here cost 1 energy or 1 rune of any type less."
card("Risen Altar", cost_aura=lambda g, b, pid, what: [dict(flex=1)]
     if what["kind"] == "ability" and what.get("ab") is not None and what["ab"].get("name") == "Empower"
     and what.get("obj") is not None and getattr(what["obj"], "zone", None) == "board"
     and what["obj"].spec["type"] == "Unit" and what["obj"].ctrl == pid and what["obj"].loc == b.idx else [])


# Sandswept Tomb — "Each spell that chooses one or more units here that are friendly to it costs 1 rune of any type
# less."
def _tomb(g, b, pid, what):
    if what["kind"] != "spell" or what.get("choice") is None:
        return []
    for uid in what["choice"].get("tg", ()):
        o = g.obj(uid) if isinstance(uid, int) else None
        if o is not None and o.spec["type"] == "Unit" and o.loc == b.idx and o.ctrl == pid:
            return [dict(rm=1)]
    return []


card("Sandswept Tomb", cost_aura=_tomb)


# Vaults of Helia — "When you hold here, your non-token units cost 1 energy more to play this turn."
def _vaults_mod(g, eff, pid, what):
    return [dict(e=1)] if what["kind"] == "unit" and what["card"] is not None and not what["card"].token else ()


def _vaults(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "Vaults of Helia", lambda g_, it: g_.effects.append(
            dict(kind="cost_mod", fn=_vaults_mod, pid=it.ctrl, dur="turn")))


card("Vaults of Helia", bf_event=_vaults)


# Heisho, Shell of the World — "Players ignore [Deflect] while paying for spells and abilities choosing something
# here." (actions.deflect_reqs skips the objects at this battlefield)
card("Heisho, Shell of the World")


# Rockfall Path — "Units can't be played here." (tokens included; actions.loc_allowed)
card("Rockfall Path", loc_veto=lambda g, b, pid, c, loc: loc == b.idx and c.spec["type"] == "Unit")


# Dragon Roost — "Any player may pay 2 runes of any type as an additional cost to play a Dragon. If they do, they play
# it to this battlefield." (a location with an additional cost: choice loc_reqs)
card("Dragon Roost", aura_locs=lambda g, b, pid, c, closed: [(b.idx, dict(roost=True, loc_reqs=(_ANY, _ANY)))]
     if c.spec["type"] == "Unit" and "Dragon" in c.spec["tags"] else [])


# Vilemaw's Lair — "Units can't move from here to base." (a recall is not a move)
card("Vilemaw's Lair", aura_cant_move=lambda g, b, o, dest, by, standard: o.loc == b.idx and dest == "base")


# Forge of the Fluft — "While you control this battlefield, friendly legends have '[E]: Attach an Equipment you
# control to a unit you control.'"
def _fluft_choices(g, pid, o):
    eqs = sorted([x for x in g.gear(pid) if "Equipment" in x.spec["tags"]], key=lambda x: (x.attached_to is not None,
                                                                                          x.uid))
    us = sorted(g.units(pid), key=lambda u: (-value(g, u), u.uid))
    return [dict(tg=(x.uid, u.uid)) for x in eqs for u in us if x.attached_to != u.uid]


def _fluft_res(g, it):
    x, u = g.legal(it, 0), g.legal(it, 1)
    if x is not None and u is not None and x.ctrl == it.ctrl and u.ctrl == it.ctrl:
        attach(g, x, u)


_FLUFT_AB = ability("Attach", None, exhaust=True, choices=_fluft_choices, preds=[P_gear, P_friend],
                    resolve=_fluft_res)

card("Forge of the Fluft", grant_abilities=lambda g, b, o: [_FLUFT_AB]
     if getattr(o, "zone", None) == "legend" and b.ctrl == o.owner else [])


# Gardens of Becoming — "Units here have '[E]: Gain 1 XP.'"
_GARDENS_AB = ability("Gain 1 XP", None, exhaust=True, resolve=lambda g, it: g.gain_xp(it.ctrl, 1))

card("Gardens of Becoming", grant_abilities=lambda g, b, o: [_GARDENS_AB]
     if getattr(o, "zone", None) == "board" and o.spec["type"] == "Unit" and o.loc == b.idx else [])


# Altar of Blood — "If a unit here would die during combat, its controller may pay 3 runes of any type to heal it,
# exhaust it, and recall it instead."
def _altar_blood(g, b, u):
    if u.loc != b.idx or g.sd is None or not g.sd.combat or g.sd.bf != b.idx \
            or not g.can_pay(u.ctrl, 0, [_ANY] * 3):
        return None
    pid = u.ctrl
    return dict(name="Altar of Blood", may=True, cost=lambda g_: g_.pay(pid, 0, [_ANY] * 3),
                apply=lambda g_, x: g_.save_unit(x))


card("Altar of Blood", death_rep=_altar_blood)


# Ripper's Bay — "When a unit here is returned to a player's hand, that player may pay 1 energy to channel 1 rune
# exhausted."
def _ripper(g, b, ev, info):
    if ev == "returned" and info["loc"] == b.idx and info["obj"].spec["type"] == "Unit":
        g.queue_trigger(info["pid"], "Ripper's Bay", lambda g_, it: g_.channel(it.ctrl, 1, exhausted=True),
                        may=True, cost=lambda g_, it: g_.can_pay(it.ctrl, 1, []) and g_.pay(it.ctrl, 1, []))


card("Ripper's Bay", bf_event=_ripper)


# Forgotten Library — "While you control this battlefield, when you play a spell, if you spent 4 energy or more,
# [Predict]."
def _library(g, b, ev, info):
    it = info.get("item") if ev == "played" else None
    if it is not None and info["pid"] == b.ctrl and info["card"].spec["type"] == "Spell" \
            and it.data.get("paid_e", 0) >= 4:
        g.queue_trigger(info["pid"], "Forgotten Library", lambda g_, it_: g_.predict(it_.ctrl, 1))


card("Forgotten Library", bf_event=_library)


# The Academy — "When you hold here, give your next spell this turn [Repeat] equal to its base cost."
def _academy_cost(g, card):
    return card.spec["e"], g.power_reqs(card.spec["domains"], card.spec["p"])


def _academy(g, b, ev, info):
    if ev == "hold" and info["bf"] == b.idx:
        g.queue_trigger(info["pid"], "The Academy", lambda g_, it: g_.effects.append(
            dict(kind="grant_repeat", pid=it.ctrl, cost=_academy_cost, next=True, key=f"academy-{it.id}",
                 dur="turn")))


card("The Academy", bf_event=_academy)


# Bandle Tree — "You may hide an additional card here."
card("Bandle Tree", hide_slots=1)
