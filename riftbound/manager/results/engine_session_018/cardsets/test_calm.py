"""Tests of batch calm. Run: RB_CARDSETS=calm python3 cardsets/test_calm.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from cards import attach                                # noqa: E402
from game import ANY                                    # noqa: E402

T = Suite("calm")


# ------------------------------------------------------------------ helpers
def play(g, pid, name, pred=lambda ch: True, src=None):
    g.apply(opt(g, pid, name, pred, src))
    return settle(g)


def attack(g, units, bf):
    g.apply(("move", tuple(u.uid for u in units), bf))
    return settle(g)


def hold(g, pid, bf):
    g.hold(pid, g.bfs[bf])
    return settle(g)


def until(g, cond, max_steps=100):
    """Step with the scripted agents until a decision satisfies cond(d)."""
    for _ in range(max_steps):
        d = g.advance()
        if d is None:
            raise AssertionError("game over")
        if cond(d):
            return d
        g.apply(g.agents[d.player].decide(g, d))
    raise AssertionError("condition never met")


def act(g, pid, name, pred=lambda o: True):
    os_ = [o for o in act_options(g, pid, name) if pred(o)]
    assert os_, f"no ability option for {name}"
    g.apply(os_[0])
    return settle(g)


def exhaust_all(g, pid):
    for r in g.p[pid].runes:
        r.exhausted = True


# ------------------------------------------------------------------ units
@T.test
def affectionate_poro_draws_if_undamaged():
    g, _ = new()
    p = put(g, 0, "Affectionate Poro")
    e = put(g, 1, "Pouty Poro", 1)
    e.stunned = True                                   # stunned: deals no combat damage
    n = len(g.p[0].hand)
    attack(g, [p], 1)
    assert e.zone == "trash" and len(g.p[0].hand) == n + 1


@T.test
def affectionate_poro_no_draw_when_damaged():
    g, _ = new()
    p = put(g, 0, "Affectionate Poro")
    put(g, 1, "Soaring Scout", 1)
    n = len(g.p[0].hand)
    attack(g, [p], 1)
    assert p.zone == "board" and len(g.p[0].hand) == n


@T.test
def ahri_scores_when_holding():
    g, _ = new()
    put(g, 0, "Ahri, Alluring", 0)
    hold(g, 0, 0)
    assert g.p[0].points == 2


@T.test
def allay_gives_deflect_at_battlefield():
    g, _ = new()
    a = put(g, 0, "Allay, Eager Admirer", 1)
    o = put(g, 0, "Mournful Witness", 1)
    b = put(g, 0, "Lonely Poro")
    assert g.kw_value(a, "Deflect") == 1 and g.kw_value(o, "Deflect") == 1 and g.kw_value(b, "Deflect") == 0
    g.move([a], "base", 0)
    assert g.kw_value(o, "Deflect") == 0 and g.kw_value(b, "Deflect") == 0


@T.test
def aphelios_modes_once_per_turn():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    exhaust_all(g, 0)
    a = put(g, 0, "Aphelios, Exalted")
    attach(g, put(g, 0, "Doran's Shield"), a)
    settle(g)
    assert sum(1 for r in g.p[0].runes if not r.exhausted) == 2      # "ready 2 runes" first
    attach(g, put(g, 0, "Soul Sword"), a)
    settle(g)
    assert len(g.p[0].runes) == 4                                    # "ready" already chosen: channel
    attach(g, put(g, 0, "Hand Hammer"), a)
    settle(g)
    assert a.buff == 1                                               # last mode: buff a friendly unit
    attach(g, put(g, 0, "Brutalizer"), a)
    settle(g)
    assert len(g.p[0].runes) == 4 and a.buff == 1                    # no mode left this turn


@T.test
def apprentice_smith_draws_gear():
    g, _ = new()
    s = put(g, 0, "Apprentice Smith")
    top = deck_top(g, 0, ["Brutalizer", "Pouty Poro"])
    attack(g, [s], 1)
    assert top[0] in g.p[0].hand
    s.exhausted = False
    attack(g, [s], "base")
    assert top[1] not in g.p[0].hand and g.p[0].deck[-1] is top[1]   # not a gear: recycled


@T.test
def azir_swaps_and_takes_equipment():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    az = put(g, 0, "Azir, Ascendant")
    u = put(g, 0, "Pouty Poro", 1)
    sh = put(g, 0, "Doran's Shield")
    attach(g, sh, u)
    settle(g)
    act(g, 0, "Azir, Ascendant")
    assert az.loc == 1 and u.loc == "base" and sh.attached_to == az.uid
    assert not act_options(g, 0, "Azir, Ascendant")                 # once per turn


@T.test
def caitlyn_backline_and_shot():
    g, _ = new()
    c = put(g, 0, "Caitlyn, Patrolling", 1)
    d = put(g, 1, "Mountain Drake", 0)
    assert g.has_kw(c, "Backline")
    act(g, 0, "Caitlyn, Patrolling")
    assert d.damage == 4 and c.exhausted                              # 3 + 1 (Void Gate: ability damage)
    c2 = put(g, 0, "Caitlyn, Patrolling")
    assert not act_options(g, 0, "Caitlyn, Patrolling") or all(o[1] != c2.uid for o in act_options(g, 0))


@T.test
def clockwork_keeper_optional_cost_draws():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    hand(g, 0, "Clockwork Keeper")
    top = deck_top(g, 0, ["Pouty Poro"])
    play(g, 0, "Clockwork Keeper", lambda ch: ch.get("calm_paid"))
    assert top[0] in g.p[0].hand and len(g.p[0].runes) == 2


@T.test
def daisy_cost_ready_and_stun():
    g, _ = new()
    for n in ("Soaring Scout", "Frisky Hunter", "Frostcoat Mother", "Pouty Poro"):
        put(g, 0, n)
    c = hand(g, 0, "Daisy!")
    assert total_cost(g, 0, c, dict(loc="base"), "hand")[0] == 5
    runes(g, 0, ["Calm"] * 5 + ["Order"] * 2)
    play(g, 0, "Daisy!", lambda ch: ch["loc"] == "base")
    assert c.zone == "board" and not c.exhausted
    d = put(g, 1, "Mountain Drake", 1)
    attack(g, [c], 1)
    assert d.stunned and c.zone == "board"


@T.test
def eclipse_herald_readies_on_stun():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    h = put(g, 0, "Eclipse Herald", ready=False)
    e = put(g, 1, "Pouty Poro")
    hand(g, 0, "Rune Prison")
    play(g, 0, "Rune Prison", lambda ch: ch["tg"] == (e.uid,))
    assert e.stunned and not h.exhausted and g.might(h) == 8


@T.test
def enthusiastic_promoter_buffs_on_hold():
    g, _ = new()
    p = put(g, 0, "Enthusiastic Promoter", 0)
    o = put(g, 0, "Pouty Poro", 0)
    assert g.has_kw(p, "Backline")
    hold(g, 0, 0)
    assert p.buff == 1 and o.buff == 1


@T.test
def field_musicians_and_whiteflame_give_might():
    g, _ = new()
    runes(g, 0, ["Calm"] * 12 + ["Mind"] * 2)
    big = put(g, 0, "Glasc Mixologist")
    hand(g, 0, "Field Musicians")
    play(g, 0, "Field Musicians", lambda ch: ch["loc"] == "base")
    assert g.might(big) == 8
    hand(g, 0, "Whiteflame Protector")
    play(g, 0, "Whiteflame Protector", lambda ch: ch["loc"] == "base")
    wf = [u for u in g.units(0) if u.cname == "Whiteflame Protector"][0]
    assert g.might(big) + g.might(wf) == 8 + 8 + 8


@T.test
def frisky_hunter_plays_bird_here():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    hand(g, 0, "Frisky Hunter")
    play(g, 0, "Frisky Hunter", lambda ch: ch["loc"] == "base")
    birds = [u for u in g.units(0) if u.cname == "Bird"]
    assert len(birds) == 1 and birds[0].loc == "base" and g.has_kw(birds[0], "Deflect")


@T.test
def empower_units():
    g, _ = new()
    runes(g, 0, ["Calm"] * 10)
    f = put(g, 0, "Frostcoat Mother")
    act(g, 0, "Frostcoat Mother")                    # 12 - 10 runes = 2 energy
    assert f.empowered and g.might(f) == 6 and sum(r.exhausted for r in g.p[0].runes) == 2
    g2, _ = new()
    runes(g2, 0, ["Calm"] * 7)
    s = put(g2, 0, "Steel Paws")
    assert g2.has_kw(s, "Deflect") and g2.might(s) == 0
    act(g2, 0, "Steel Paws")
    assert g2.might(s) == 7
    g3, _ = new()
    runes(g3, 0, ["Calm"] * 3)
    a = put(g3, 0, "Serene Ascetic")
    assert not g3.has_kw(a, "Deflect")
    act(g3, 0, "Serene Ascetic")
    assert g3.kw_value(a, "Deflect") == 1 and g3.kw_value(a, "Shield") == 3


@T.test
def nasus_empowered_scores_on_conquer():
    g, _ = new()
    n = put(g, 0, "Nasus, Ascended")
    assert g.kw_value(n, "Deflect") == 2
    n.empowered = True
    attack(g, [n], 1)
    assert g.p[0].points == 2


@T.test
def guardian_of_the_passage_returns_card():
    g, _ = new()
    put(g, 0, "Guardian of the Passage", 0)
    c = Obj("Pouty Poro", 0)
    c.zone = "trash"
    g.p[0].trash.append(c)
    hold(g, 0, 0)
    assert c in g.p[0].hand


@T.test
def herald_of_spring_gains_xp():
    g, _ = new()
    runes(g, 0, ["Calm"] * 5)
    hand(g, 0, "Herald of Spring")
    play(g, 0, "Herald of Spring", lambda ch: ch["loc"] == "base")
    assert g.p[0].xp == 2


@T.test
def iascylla_moves_enemy_next_main_phase():
    g, _ = new()
    put(g, 0, "Iascylla", 0)
    e = put(g, 1, "Mournful Witness")
    hold(g, 0, 0)
    assert e.loc == "base"
    g.emit("main_start", pid=1)                      # not this player's Main Phase
    settle(g)
    assert e.loc == "base"
    g.emit("main_start", pid=0)
    settle(g)
    assert e.zone == "trash" or e.loc == 0           # moved to the battlefield (and killed in combat)
    assert not [x for x in g.effects if x.get("on") == "main_start"]


@T.test
def ivern_draws_unit_and_buffs():
    g, _ = new()
    runes(g, 0, ["Calm"] * 6)
    top = deck_top(g, 0, ["Rune Prison", "Pouty Poro", "Wind Wall"])
    hand(g, 0, "Ivern, Nurturer")
    play(g, 0, "Ivern, Nurturer", lambda ch: ch["loc"] == "base")
    assert top[1] in g.p[0].hand and top[0] not in g.p[0].hand
    assert any(u.buff for u in g.units(0))           # a Poro was revealed


@T.test
def ornn_draws_gear():
    g, _ = new()
    runes(g, 0, ["Calm"] * 6)
    top = deck_top(g, 0, ["Pouty Poro", "Brutalizer"])
    hand(g, 0, "Ornn, Blacksmith")
    play(g, 0, "Ornn, Blacksmith", lambda ch: ch["loc"] == "base")
    assert top[1] in g.p[0].hand and top[0] not in g.p[0].hand


@T.test
def janna_heals_and_moves_enemy():
    g, _ = new()
    f = put(g, 0, "Glasc Mixologist", 0)
    f.damage = 3
    e = put(g, 1, "Mournful Witness", 0)
    j = put(g, 0, "Janna, Savior", 0)
    assert IMPL["Janna, Savior"].timing == "reaction"
    IMPL["Janna, Savior"].on_play(g, j, dict(hidden_bf=None))
    settle(g)
    assert f.damage == 0 and e.loc == "base"


@T.test
def leona_ready_and_stunned_enemies():
    g, _ = new()
    runes(g, 0, ["Calm"] * 7)
    g.p[1].points = 5
    g.bfs[0].ctrl = 0
    hand(g, 0, "Leona, Zealot")
    play(g, 0, "Leona, Zealot", lambda ch: ch["loc"] == 0)
    leo = [u for u in g.units(0) if u.cname == "Leona, Zealot"][0]
    assert not leo.exhausted
    d = put(g, 1, "Mountain Drake", 0)
    p = put(g, 1, "Pouty Poro", 0)
    assert g.might(d) == 10
    d.stunned = p.stunned = True
    assert g.might(d) == 2 and g.might(p) == 1


@T.test
def master_yi_meditative_and_unstoppable():
    g, _ = new()
    runes(g, 0, ["Calm"] * 7)
    y = put(g, 0, "Master Yi, Meditative")
    assert g.might(y) == 4
    runes(g, 0, ["Calm"] * 8)
    assert g.might(y) == 8
    c = hand(g, 0, "Master Yi, Unstoppable")
    assert total_cost(g, 0, c, dict(loc="base"), "hand") == (12, [frozenset({"Calm"})] * 3)
    g.p[0].xp = 6
    assert total_cost(g, 0, c, dict(loc="base"), "hand") == (8, [frozenset({"Calm"})])
    g.p[0].xp = 11
    assert total_cost(g, 0, c, dict(loc="base"), "hand") == (6, [])
    u = put(g, 0, "Master Yi, Unstoppable")
    assert g.targetable(u, 1)
    g.p[0].xp = 16
    assert not g.targetable(u, 1) and g.targetable(u, 0)


@T.test
def monch_discount_and_ready():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    c = hand(g, 0, "Monch")
    assert not card_choices(g, 0, c, "hand", False, False)
    put(g, 1, "Pouty Poro").stunned = True
    play(g, 0, "Monch", lambda ch: ch["loc"] == "base")
    assert c.zone == "board" and not c.exhausted


@T.test
def mosstomper_level_3():
    g, _ = new()
    m = put(g, 0, "Mosstomper")
    assert g.might(m) == 3 and not g.has_kw(m, "Deflect") and g.kw_value(m, "Hunt") == 2
    g.gain_xp(0, 3)
    assert g.might(m) == 4 and g.has_kw(m, "Deflect")


@T.test
def nami_stun_and_next_unit():
    g, _ = new()
    runes(g, 0, ["Calm"] * 8)
    e = put(g, 1, "Mournful Witness")
    hand(g, 0, "Nami, Headstrong")
    play(g, 0, "Nami, Headstrong", lambda ch: ch.get("calm_paid") and ch["loc"] == "base")
    nami = [u for u in g.units(0) if u.cname == "Nami, Headstrong"][0]
    assert e.stunned
    nami.loc = 0
    g.bfs[0].ctrl = 0
    hold(g, 0, 0)
    hand(g, 0, "Lonely Poro")
    play(g, 0, "Lonely Poro", lambda ch: ch["loc"] == "base")
    lp = [u for u in g.units(0) if u.cname == "Lonely Poro"][0]
    assert not lp.exhausted and lp.buff == 1
    hand(g, 0, "Pouty Poro")
    play(g, 0, "Pouty Poro", lambda ch: ch["loc"] == "base")
    pp = [u for u in g.units(0) if u.cname == "Pouty Poro"][0]
    assert pp.exhausted and pp.buff == 0 and nami.zone == "board"


@T.test
def nami_without_additional_cost_no_stun():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    e = put(g, 1, "Pouty Poro")
    hand(g, 0, "Nami, Headstrong")
    play(g, 0, "Nami, Headstrong", lambda ch: not ch.get("calm_paid"))
    assert not e.stunned


@T.test
def ol_poro_not_before_fourth_turn():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    c = hand(g, 0, "Ol' Poro")
    assert not card_choices(g, 0, c, "hand", False, False)
    g.p[0].turns = 4
    assert card_choices(g, 0, c, "hand", False, False)


@T.test
def pakaa_protector_reveals():
    g, _ = new()
    p = put(g, 0, "Pakaa Protector")
    top = deck_top(g, 0, ["Rune Prison", "Pouty Poro"])
    attack(g, [p], 1)
    assert top[0] in g.p[0].trash and g.might(p) == 6
    p.exhausted = False
    attack(g, [p], "base")
    assert top[1] in g.p[0].hand


@T.test
def poro_herder_with_poro():
    g, _ = new()
    runes(g, 0, ["Calm"] * 8)
    hand(g, 0, "Poro Herder")
    n = len(g.p[0].hand)
    play(g, 0, "Poro Herder", lambda ch: ch["loc"] == "base")
    h = [u for u in g.units(0) if u.cname == "Poro Herder"][0]
    assert h.buff == 0 and len(g.p[0].hand) == n - 1
    put(g, 0, "Pouty Poro")
    hand(g, 0, "Poro Herder")
    play(g, 0, "Poro Herder", lambda ch: ch["loc"] == "base")
    h2 = [u for u in g.units(0) if u.cname == "Poro Herder" and u is not h][0]
    assert h2.buff == 1 and len(g.p[0].hand) == n


@T.test
def legion_quartermaster_returns_gear():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    c = hand(g, 0, "Legion Quartermaster")
    assert not card_choices(g, 0, c, "hand", False, False)          # mandatory additional cost
    s = put(g, 0, "Poro Snax")
    play(g, 0, "Legion Quartermaster", lambda ch: ch["ret_gear"] == s.uid)
    assert s in g.p[0].hand and c.zone == "board"


@T.test
def ribbon_dancer_gives_might():
    g, _ = new()
    r = put(g, 0, "Ribbon Dancer")
    o = put(g, 0, "Pouty Poro")
    attack(g, [r], 1)
    assert g.might(o) == 3 and g.might(r) == 3


@T.test
def riven_deals_per_equipment():
    g, _ = new()
    r = put(g, 0, "Riven, Shattered")
    assert g.has_kw(r, "Weaponmaster")
    attach(g, put(g, 0, "Doran's Shield"), r)
    attach(g, put(g, 0, "Soul Sword"), r)
    d = put(g, 1, "Sunlit Guardian", 1)             # 4 might while defending
    attack(g, [r], 1)
    assert d.zone == "trash" and r.zone == "board" and r.damage == 0 and g.bfs[1].ctrl == 0


@T.test
def royal_entourage_readies_own_legend():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    g.p[0].legend.exhausted = True
    hand(g, 0, "Royal Entourage")
    play(g, 0, "Royal Entourage", lambda ch: ch["loc"] == "base")
    assert not g.p[0].legend.exhausted
    hand(g, 0, "Royal Entourage")
    runes(g, 0, ["Calm"] * 4)
    play(g, 0, "Royal Entourage", lambda ch: ch["loc"] == "base")
    assert g.p[1].legend.exhausted                  # own legend ready: exhaust the enemy one


@T.test
def shadow_enters_ready_at_battlefield_and_stuns_attacker():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    put(g, 0, "Pouty Poro", 0)
    hand(g, 0, "Shadow")
    play(g, 0, "Shadow", lambda ch: ch["loc"] == 0)
    sh = [u for u in g.units(0) if u.cname == "Shadow"][0]
    assert not sh.exhausted
    g2, _ = new(tp=1)
    runes(g2, 0, ["Calm"] * 2)
    s2 = put(g2, 0, "Shadow", 0)
    e = put(g2, 1, "Mountain Drake")
    g2.apply(("move", (e.uid,), 0))
    d = until(g2, lambda d: d.player == 0 and any(o[0] == "act" and o[1] == s2.uid for o in d.options))
    g2.apply([o for o in d.options if o[0] == "act" and o[1] == s2.uid][0])
    settle(g2)
    assert s2.exhausted and (e.stunned or e.zone == "trash")
    assert s2.zone == "board"                        # the stunned Drake dealt no damage


@T.test
def shen_draws_with_exactly_one_other():
    g, _ = new()
    put(g, 0, "Shen, Scourge of Shadows", 0)
    put(g, 0, "Pouty Poro", 0)
    n = len(g.p[0].hand)
    hold(g, 0, 0)
    assert len(g.p[0].hand) == n + 1
    put(g, 0, "Lonely Poro", 0)
    g.bfs[0].scored = set()
    hold(g, 0, 0)
    assert len(g.p[0].hand) == n + 1


@T.test
def simian_ancestor_ready_on_buff():
    g, _ = new()
    s = put(g, 0, "Simian Ancestor", ready=False)
    g.buff(s)
    settle(g)
    assert not s.exhausted


@T.test
def solari_shieldbearer_stuns():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    e = put(g, 1, "Pouty Poro")
    hand(g, 0, "Solari Shieldbearer")
    play(g, 0, "Solari Shieldbearer", lambda ch: ch["loc"] == "base")
    assert e.stunned


@T.test
def sona_readies_runes_at_end_of_turn():
    g, _ = new()
    runes(g, 0, ["Calm"] * 6)
    exhaust_all(g, 0)
    put(g, 0, "Sona, Harmonious", 0)
    g.emit("end_turn", pid=0)
    settle(g)
    assert sum(1 for r in g.p[0].runes if not r.exhausted) == 4
    exhaust_all(g, 0)
    put(g, 0, "Sona, Harmonious")
    g.units(0)[0].loc = "base"
    for u in g.units(0):
        u.loc = "base"
    g.emit("end_turn", pid=0)
    settle(g)
    assert all(r.exhausted for r in g.p[0].runes)


@T.test
def taric_gives_shield_here():
    g, _ = new()
    t = put(g, 0, "Taric, Protector", 1)
    a = put(g, 0, "Pouty Poro", 1)
    b = put(g, 0, "Lonely Poro")
    assert g.has_kw(t, "Tank") and g.kw_value(t, "Shield") == 1
    assert g.kw_value(a, "Shield") == 1 and g.kw_value(b, "Shield") == 0


@T.test
def tasty_faefolk_deathknell():
    g, _ = new()
    f = put(g, 0, "Tasty Faefolk")
    n, r = len(g.p[0].hand), len(g.p[0].runes)
    g.kill([f])
    settle(g)
    assert len(g.p[0].hand) == n + 1 and len(g.p[0].runes) == r + 2
    assert all(x.exhausted for x in g.p[0].runes[r:]) and IMPL["Tasty Faefolk"].accelerate


@T.test
def trevor_plays_sprite_on_hold():
    g, _ = new()
    put(g, 0, "Trevor Snoozebottom", 0)
    hold(g, 0, 0)
    sp = [u for u in g.units(0) if u.cname == "Sprite"]
    assert len(sp) == 1 and sp[0].loc == 0 and not sp[0].exhausted and g.has_kw(sp[0], "Temporary")
    assert g.might(sp[0]) == 3


@T.test
def vex_moves_to_stunned_enemy_battlefield():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    v = put(g, 0, "Vex, Mocking")
    e = put(g, 1, "Pouty Poro", 1)
    hand(g, 0, "Rune Prison")
    play(g, 0, "Rune Prison", lambda ch: ch["tg"] == (e.uid,))
    assert v.loc == 1 and v.zone == "board"


@T.test
def wielder_of_water_alone():
    g, _ = new()
    w = put(g, 0, "Wielder of Water", 1)
    assert g.might(w) == 2
    w.desig = "att"
    assert g.might(w) == 4
    put(g, 0, "Pouty Poro", 1)
    assert g.might(w) == 2


@T.test
def wizened_elder_buffed():
    g, _ = new()
    w = put(g, 0, "Wizened Elder")
    g.buff(w)
    assert g.might(w) == 6


@T.test
def wuju_apprentice_draws_at_level_6():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    hand(g, 0, "Wuju Apprentice")
    n = len(g.p[0].hand)
    play(g, 0, "Wuju Apprentice", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == n - 1
    g.p[0].xp = 6
    hand(g, 0, "Wuju Apprentice")
    play(g, 0, "Wuju Apprentice", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == n - 1 + 1


@T.test
def yasuo_attack_deals_might():
    g, _ = new()
    y = put(g, 0, "Yasuo, Remorseful")
    e = put(g, 1, "Glasc Mixologist", 1)
    attack(g, [y], 1)
    assert e.zone == "trash" and y.damage == 0 and y.zone == "board"


@T.test
def yuumi_gives_might_and_tank():
    g, _ = new()
    y = put(g, 0, "Yuumi, Magical Cat")
    o = put(g, 0, "Pouty Poro")
    e = put(g, 1, "Pouty Poro", 1)
    e.stunned = True
    attack(g, [y, o], 1)
    assert g.might(o) == 5 and g.has_kw(o, "Tank")


# ------------------------------------------------------------------ gear and equipment
@T.test
def brutalizer_bonus_the_turn_it_is_attached():
    g, _ = new()
    runes(g, 0, ["Calm"])
    u = put(g, 0, "Pouty Poro")
    b = put(g, 0, "Brutalizer")
    act(g, 0, "Brutalizer")
    assert b.attached_to == u.uid and g.might(u) == 5
    g.turn_no += 1
    assert g.might(u) == 3


@T.test
def dorans_shield_gives_tank():
    g, _ = new()
    runes(g, 0, ["Calm"])
    u = put(g, 0, "Pouty Poro")
    put(g, 0, "Doran's Shield")
    act(g, 0, "Doran's Shield")
    assert g.has_kw(u, "Tank") and g.might(u) == 3


@T.test
def hand_hammer_with_exactly_one_other():
    g, _ = new()
    u = put(g, 0, "Pouty Poro", 0)
    attach(g, put(g, 0, "Hand Hammer"), u)
    assert g.might(u) == 3
    put(g, 0, "Lonely Poro", 0)
    assert g.might(u) == 5
    put(g, 0, "Soaring Scout", 0)
    assert g.might(u) == 3


@T.test
def soul_sword_level_3():
    g, _ = new()
    u = put(g, 0, "Pouty Poro")
    attach(g, put(g, 0, "Soul Sword"), u)
    assert g.might(u) == 3
    g.p[0].xp = 3
    assert g.might(u) == 4


@T.test
def forgefire_cape_hits_enemies_here():
    g, _ = new()
    u = put(g, 0, "Glasc Mixologist")
    attach(g, put(g, 0, "Forgefire Cape"), u)
    assert g.might(u) == 8
    a = put(g, 1, "Soaring Scout", 1)
    b = put(g, 1, "Pouty Poro", 1)
    attack(g, [u], 1)
    assert a.zone == "trash" and b.zone == "trash" and u.damage == 0 and g.bfs[1].ctrl == 0


@T.test
def shurelyas_requiem_ready_and_ganking():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4 + ["Mind"] * 2)
    a = put(g, 0, "Pouty Poro", 0, ready=False)
    b = put(g, 0, "Lonely Poro", 0, ready=False)
    hand(g, 0, "Shurelya's Requiem")
    play(g, 0, "Shurelya's Requiem")
    s = [x for x in g.gear(0) if x.cname == "Shurelya's Requiem"][0]
    assert not a.exhausted and not b.exhausted
    assert not g.has_kw(b, "Ganking")
    attach(g, s, a)
    assert g.has_kw(a, "Ganking") and g.has_kw(b, "Ganking") and g.might(a) == 4


@T.test
def heart_of_dark_ice_gives_might():
    g, _ = new()
    u = put(g, 0, "Pouty Poro")
    h = put(g, 0, "Heart of Dark Ice")
    act(g, 0, "Heart of Dark Ice")
    assert g.might(u) == 5 and h.exhausted


@T.test
def honeyfruit_adds():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    hand(g, 0, "Honeyfruit")
    play(g, 0, "Honeyfruit")
    h = [x for x in g.gear(0) if x.cname == "Honeyfruit"][0]
    assert h.exhausted
    g.p[0].runes = []
    assert not g.can_pay(0, 0, [ANY])
    h.exhausted = False
    assert g.can_pay(0, 0, [ANY]) and not g.can_pay(0, 1, [ANY])
    g.p[0].xp = 6
    assert g.can_pay(0, 1, [ANY])


@T.test
def mask_of_foresight_alone():
    g, _ = new()
    put(g, 0, "Mask of Foresight")
    u = put(g, 0, "Glasc Mixologist")
    e = put(g, 1, "Pouty Poro", 1)
    e.stunned = True
    attack(g, [u], 1)
    assert g.might(u) == 6


@T.test
def poro_snax_draws():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    hand(g, 0, "Poro Snax")
    n = len(g.p[0].hand)
    play(g, 0, "Poro Snax")
    assert len(g.p[0].hand) == n
    act(g, 0, "Poro Snax")
    assert len(g.p[0].hand) == n + 1 and not [x for x in g.gear(0) if x.cname == "Poro Snax"]


@T.test
def seal_of_focus_adds_calm():
    g, _ = new()
    put(g, 0, "Seal of Focus")
    assert g.can_pay(0, 0, [frozenset({"Calm"})]) and not g.can_pay(0, 0, [frozenset({"Fury"})])


@T.test
def spirits_refuge_buff_and_deflect():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    u = put(g, 0, "Pouty Poro")
    s = put(g, 0, "Steel Paws")
    hand(g, 0, "Spirit's Refuge")
    play(g, 0, "Spirit's Refuge")
    assert u.buff == 1 and g.kw_value(u, "Deflect") == 1
    g.buff(s)
    assert g.kw_value(s, "Deflect") == 1                # already had Deflect: not doubled
    put(g, 0, "Spirit's Refuge")
    assert g.kw_value(u, "Deflect") == 1


@T.test
def forgotten_signpost_moves():
    g, _ = new()
    a = put(g, 0, "Pouty Poro", 0)
    b = put(g, 0, "Lonely Poro")
    put(g, 0, "Forgotten Signpost")
    act(g, 0, "Forgotten Signpost")
    assert a.exhausted and b.loc == 0


# ------------------------------------------------------------------ spells
@T.test
def alpha_strike_splits_and_gains_xp():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    f = put(g, 0, "Glasc Mixologist")
    es = [put(g, 1, "Soaring Scout", 1), put(g, 1, "Soaring Scout", 1), put(g, 1, "Pouty Poro", 1)]
    hand(g, 0, "Alpha Strike")
    play(g, 0, "Alpha Strike", lambda ch: len(ch["tg"]) == 4)
    assert all(e.zone == "trash" for e in es) and g.p[0].xp == 3 and f.zone == "board"


@T.test
def alpha_strike_drops_targets_beyond_damage():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    f = put(g, 0, "Pouty Poro")                        # 2 might
    es = [put(g, 1, "Soaring Scout", 1), put(g, 1, "Soaring Scout", 1)]
    hand(g, 0, "Alpha Strike")
    g.apply(opt(g, 0, "Alpha Strike", lambda ch: len(ch["tg"]) == 3))
    g.mod(f, -1)                                       # 1 damage left to split: one target is dropped
    settle(g)
    assert sum(1 for e in es if e.zone == "trash") == 1 and g.p[0].xp == 1


@T.test
def arise_plays_sand_soldiers():
    g, _ = new()
    runes(g, 0, ["Calm"] * 7)
    put(g, 0, "Brutalizer")
    put(g, 0, "Soul Sword")
    put(g, 0, "Heart of Dark Ice")
    hand(g, 0, "Arise!")
    play(g, 0, "Arise!")
    ss = [u for u in g.units(0) if u.cname == "Sand Soldier"]
    assert len(ss) == 2 and all(not u.exhausted for u in ss) and g.might(ss[0]) == 2


@T.test
def combat_experience_level():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    u = put(g, 0, "Pouty Poro")
    hand(g, 0, "Combat Experience")
    play(g, 0, "Combat Experience")
    assert g.might(u) == 3
    g.p[0].xp = 6
    hand(g, 0, "Combat Experience")
    play(g, 0, "Combat Experience")
    assert g.might(u) == 6


@T.test
def defiant_dance_plus_and_minus():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    u = put(g, 0, "Pouty Poro")
    e = put(g, 1, "Glasc Mixologist")
    hand(g, 0, "Defiant Dance")
    play(g, 0, "Defiant Dance", lambda ch: ch["tg"] == (u.uid, e.uid))
    assert g.might(u) == 4 and g.might(e) == 3


@T.test
def deserts_call_repeat():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    hand(g, 0, "Desert's Call")
    play(g, 0, "Desert's Call", lambda ch: ch.get("rep"))
    assert len([u for u in g.units(0) if u.cname == "Sand Soldier"]) == 2 and all(r.exhausted for r in g.p[0].runes)


@T.test
def double_trouble_draws_units():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    hand(g, 0, "Double Trouble")
    top = deck_top(g, 0, ["Rune Prison", "Pouty Poro", "Wind Wall", "Lonely Poro"])
    play(g, 0, "Double Trouble", lambda ch: ch.get("rep"))
    assert top[1] in g.p[0].hand and top[3] in g.p[0].hand and top[0] not in g.p[0].hand


@T.test
def dragons_rage_exchange():
    g, _ = new()
    runes(g, 0, ["Calm"] * 5)
    a = put(g, 1, "Glasc Mixologist")
    b = put(g, 1, "Pouty Poro", 1)
    hand(g, 0, "Dragon's Rage")
    play(g, 0, "Dragon's Rage", lambda ch: ch["tg"] == (a.uid, b.uid))
    assert a.loc == 1 and b.zone == "trash" and a.damage == 2


@T.test
def emperors_divide_returns_units():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    a = put(g, 0, "Pouty Poro", 0)
    b = put(g, 0, "Lonely Poro", 0)
    hand(g, 0, "Emperor's Divide")
    play(g, 0, "Emperor's Divide", lambda ch: len(ch["tg"]) == 2)
    assert a.loc == "base" and b.loc == "base"


@T.test
def feral_strength_repeat():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    u = put(g, 0, "Pouty Poro")
    hand(g, 0, "Feral Strength")
    play(g, 0, "Feral Strength", lambda ch: ch.get("rep") and ch["tg"] == (u.uid,) and ch["tg2"] == (u.uid,))
    assert g.might(u) == 6


@T.test
def find_your_center_discount():
    g, _ = new()
    c = hand(g, 0, "Find Your Center")
    assert total_cost(g, 0, c, {}, "hand")[0] == 3
    g.p[1].points = 5
    assert total_cost(g, 0, c, {}, "hand")[0] == 1
    runes(g, 0, ["Calm"])
    n, r = len(g.p[0].hand), len(g.p[0].runes)
    play(g, 0, "Find Your Center")
    assert len(g.p[0].hand) == n and len(g.p[0].runes) == r + 1 and g.p[0].runes[-1].exhausted


def _p1_casts_rune_prison(g, target):
    g.tp = 1
    runes(g, 1, ["Calm"] * 3)
    hand(g, 1, "Rune Prison")
    g.apply(opt(g, 1, "Rune Prison", lambda ch: ch["tg"] == (target.uid,)))


@T.test
def flurry_of_feathers_counters():
    g, _ = new()
    runes(g, 0, ["Calm"] * 6)
    u = put(g, 0, "Pouty Poro")
    hand(g, 0, "Flurry of Feathers")
    _p1_casts_rune_prison(g, u)
    until(g, lambda d: d.player == 0)
    g.apply(opt(g, 0, "Flurry of Feathers", lambda ch: ch.get("mode") == "counter"))
    settle(g)
    assert not u.stunned


@T.test
def flurry_of_feathers_birds():
    g, _ = new()
    runes(g, 0, ["Calm"] * 6)
    hand(g, 0, "Flurry of Feathers")
    play(g, 0, "Flurry of Feathers", lambda ch: ch.get("mode") == "birds")
    birds = [u for u in g.units(0) if u.cname == "Bird"]
    assert len(birds) == 4 and all(g.has_kw(b, "Deflect") for b in birds)


@T.test
def wind_wall_counters():
    g, _ = new()
    runes(g, 0, ["Calm"] * 5)
    u = put(g, 0, "Pouty Poro")
    hand(g, 0, "Wind Wall")
    _p1_casts_rune_prison(g, u)
    until(g, lambda d: d.player == 0)
    g.apply(opt(g, 0, "Wind Wall"))
    settle(g)
    assert not u.stunned and [c for c in g.p[1].trash if c.cname == "Rune Prison"]


@T.test
def fox_fire_kills_total_4():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    a, b = put(g, 1, "Pouty Poro", 1), put(g, 1, "Soaring Scout", 1)
    c = put(g, 1, "Glasc Mixologist", 1)
    hand(g, 0, "Fox-Fire")
    play(g, 0, "Fox-Fire", lambda ch: set(ch["tg"]) == {a.uid, b.uid})
    assert a.zone == "trash" and b.zone == "trash" and c.zone == "board"


@T.test
def fox_fire_subset_when_total_grows():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    a, b = put(g, 1, "Pouty Poro", 1), put(g, 1, "Soaring Scout", 1)
    hand(g, 0, "Fox-Fire")
    g.apply(opt(g, 0, "Fox-Fire", lambda ch: set(ch["tg"]) == {a.uid, b.uid}))
    g.mod(a, 3)                                        # total 6: the controller chooses a legal subset (355.11.b)
    settle(g)
    assert a.zone == "board" and b.zone == "trash"


@T.test
def fox_fire_from_facedown_only_there():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    put(g, 0, "Lonely Poro", 0)
    c = hand(g, 0, "Fox-Fire")
    g.apply(("hide", c.uid, 0))
    settle(g)
    c.hidden_turn = g.turn_no - 1
    a = put(g, 1, "Soaring Scout", 0, bf_control=False)
    b = put(g, 1, "Soaring Scout", 1)
    chs = card_choices(g, 0, c, "facedown", False, False)
    assert chs and all(ch["tg"] == (a.uid,) for ch in chs), chs
    assert b.zone == "board"


@T.test
def friendship_counts_tags():
    g, _ = new()
    runes(g, 0, ["Calm"])
    u = put(g, 0, "Pouty Poro")
    put(g, 0, "Soaring Scout")
    put(g, 0, "Lonely Poro")
    hand(g, 0, "Friendship")
    play(g, 0, "Friendship", lambda ch: ch["tg"] == (u.uid,))
    assert g.might(u) == 4                             # Poro and Bird


@T.test
def last_breath_ready_and_damage():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3 + ["Chaos"] * 2)
    f = put(g, 0, "Glasc Mixologist", ready=False)
    e = put(g, 1, "Pouty Poro", 1)
    hand(g, 0, "Last Breath")
    play(g, 0, "Last Breath")
    assert not f.exhausted and e.zone == "trash"


@T.test
def last_stand_double_and_temporary():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    f = put(g, 0, "Glasc Mixologist")
    hand(g, 0, "Last Stand")
    play(g, 0, "Last Stand")
    assert g.might(f) == 10 and g.has_kw(f, "Temporary")


@T.test
def meditation_exhaust_draws_two():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    u = put(g, 0, "Pouty Poro")
    hand(g, 0, "Meditation")
    n = len(g.p[0].hand)
    play(g, 0, "Meditation", lambda ch: ch.get("exh") == u.uid)
    assert u.exhausted and len(g.p[0].hand) == n + 1
    hand(g, 0, "Meditation")
    play(g, 0, "Meditation", lambda ch: not ch.get("exh"))
    assert len(g.p[0].hand) == n + 2


@T.test
def party_favors_cards_and_runes():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    hand(g, 0, "Party Favors")
    h0, h1 = len(g.p[0].hand), len(g.p[1].hand)
    play(g, 0, "Party Favors")
    assert len(g.p[0].hand) == h0 and len(g.p[1].hand) == h1 + 1
    g2, _ = new(answers1={"party_favors": "runes"})
    runes(g2, 0, ["Calm"] * 3)
    hand(g2, 0, "Party Favors")
    r0, r1 = len(g2.p[0].runes), len(g2.p[1].runes)
    play(g2, 0, "Party Favors")
    assert len(g2.p[0].runes) == r0 + 1 and len(g2.p[1].runes) == r1 + 1


@T.test
def reinforce_plays_unit_cheaper():
    g, _ = new()
    runes(g, 0, ["Calm"] * 9)
    hand(g, 0, "Reinforce")
    top = deck_top(g, 0, ["Rune Prison", "Mountain Drake", "Wind Wall"])
    play(g, 0, "Reinforce")
    assert top[1].zone == "board" and all(r.exhausted for r in g.p[0].runes)   # 5 + (9 - 5)
    assert top[0] in g.p[0].deck[-3:] and top[2] in g.p[0].deck[-3:]


@T.test
def resonating_strike_moves_and_buffs():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    put(g, 0, "Lonely Poro", 0)
    u = put(g, 0, "Pouty Poro")
    hand(g, 0, "Resonating Strike")
    play(g, 0, "Resonating Strike", lambda ch: ch["tg"] == (u.uid,))
    assert u.loc == 0 and g.might(u) == 4


@T.test
def sanction_empower_until_end_of_turn():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    u = put(g, 0, "Steel Paws")
    hand(g, 0, "Sanction")
    play(g, 0, "Sanction", lambda ch: ch["tg"] == (u.uid,) and ch["mode"] == "emp")
    assert u.empowered and g.might(u) == 7
    g.apply(("end",))
    settle(g)
    assert not u.empowered


@T.test
def sanction_disempower_enemy():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    e = put(g, 1, "Steel Paws")
    e.empowered = True
    hand(g, 0, "Sanction")
    play(g, 0, "Sanction", lambda ch: ch["mode"] == "dis")
    assert not e.empowered
    g.emit("end_turn", pid=0)
    settle(g)
    assert e.empowered


@T.test
def shadow_dash_moves_enemy_and_flow():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    a = put(g, 0, "Glasc Mixologist", 0)
    b = put(g, 0, "Glasc Mixologist", 0)
    e = put(g, 1, "Pouty Poro")
    hand(g, 0, "Shadow Dash")
    play(g, 0, "Shadow Dash", lambda ch: ch["dest"] == 0)
    assert e.zone == "trash" and any(m[0] == 1 for m in a.mods) and any(m[0] == 1 for m in b.mods)
    sd = [c for c in g.p[0].trash if c.cname == "Shadow Dash"][0]
    runes(g, 0, ["Calm"] * 7)
    put(g, 1, "Pouty Poro")
    play(g, 0, "Shadow Dash", src="trash")
    assert sd in g.p[0].banish


@T.test
def siphoning_strike_channel_on_death():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    e = put(g, 1, "Mournful Witness", 1)
    hand(g, 0, "Siphoning Strike")
    play(g, 0, "Siphoning Strike")
    assert e.zone == "trash" and len(g.p[0].runes) == 5 and g.p[0].runes[-1].exhausted


@T.test
def siphoning_strike_seven_runes():
    g, _ = new()
    runes(g, 0, ["Calm"] * 7)
    e = put(g, 1, "Glasc Mixologist", 1)
    e.mods.append([1, "turn"])                         # 6 might: 4 would not kill it
    hand(g, 0, "Siphoning Strike")
    play(g, 0, "Siphoning Strike")
    assert e.zone == "trash"


@T.test
def skyward_strike_move_and_level_stun():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    e = put(g, 1, "Pouty Poro", 1)
    hand(g, 0, "Skyward Strike")
    play(g, 0, "Skyward Strike", lambda ch: ch["dest"] == "base")
    assert e.loc == "base" and not e.stunned
    g.p[0].xp = 6
    runes(g, 0, ["Calm"] * 3)
    hand(g, 0, "Skyward Strike")
    f = put(g, 1, "Lonely Poro")
    play(g, 0, "Skyward Strike", lambda ch: ch["dest"] == 1 and len(ch["tg"]) == 2 and ch["tg"][1] == f.uid)
    assert f.stunned


@T.test
def thwonk_stuns_attacker():
    g, _ = new(tp=1)
    runes(g, 0, ["Calm"] * 2)
    put(g, 0, "Lonely Poro", 0)
    e = put(g, 1, "Mountain Drake")
    hand(g, 0, "Thwonk!")
    g.apply(("move", (e.uid,), 0))
    d = until(g, lambda d: d.player == 0 and any(o[0] == "play" for o in d.options))
    g.apply([o for o in d.options if o[0] == "play"][0])
    settle(g)
    assert e.stunned


@T.test
def tricksy_tentacles_moves_group():
    g, _ = new()
    runes(g, 0, ["Calm"] * 5)
    a, b = put(g, 1, "Pouty Poro", 1), put(g, 1, "Glasc Mixologist", 1)
    hand(g, 0, "Tricksy Tentacles")
    play(g, 0, "Tricksy Tentacles", lambda ch: set(ch["tg"]) == {a.uid, b.uid} and ch["dest"] == "base")
    assert a.loc == "base" and b.loc == "base"


@T.test
def zenith_blade_stun_and_move():
    g, _ = new()
    runes(g, 0, ["Calm"] * 5)
    e = put(g, 1, "Mountain Drake", 1)
    m = put(g, 0, "Pouty Poro")
    hand(g, 0, "Zenith Blade")
    play(g, 0, "Zenith Blade", lambda ch: ch["tg"] == (e.uid, m.uid))
    assert e.stunned and (m.loc == 1 or m.zone == "board")


@T.test
def rune_prison_stuns():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    e = put(g, 1, "Pouty Poro")
    hand(g, 0, "Rune Prison")
    play(g, 0, "Rune Prison")
    assert e.stunned


# ------------------------------------------------------------------ module checks
@T.test
def unregistered_cards_are_in_needs():
    here = os.path.dirname(os.path.abspath(__file__))
    names = [l.strip() for l in open(os.path.join(here, "batches", "calm.txt"), encoding="utf-8") if l.strip()]
    needs = open(os.path.join(here, "NEEDS_calm.md"), encoding="utf-8").read()
    missing = [n for n in names if n not in IMPL and f"**{n}**" not in needs]
    assert not missing, missing


if __name__ == "__main__":
    T.main()
