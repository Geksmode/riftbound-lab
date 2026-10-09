"""Tests of batch battlefields. Run: RB_CARDSETS=battlefields python3 cardsets/test_battlefields.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *          # noqa: F401,F403
from actions import attach
from cards import make_token
import cards
from testkit import options_of

T = Suite("battlefields")


def conquer_with(g, pid, name, bf, extra_runes=4):
    """pid moves a new ready unit from base to the empty battlefield bf and conquers it. Returns the unit."""
    runes(g, pid, ["Calm"] * extra_runes)
    u = put(g, pid, name)
    g.apply(("move", (u.uid,), bf))
    settle(g)
    assert g.bfs[bf].ctrl == pid, g.lines[-10:]
    return u


def hold_with(g, pid, names, bf):
    us = [put(g, pid, n, bf) for n in names]
    g.hold(pid, g.bfs[bf])
    settle(g)
    return us


def attack(g, bf, att="Mournful Witness", dfn="Soaring Scout"):
    """P1 controls bf with a defender, P0 moves an attacker there (combat). Returns (attacker, defender)."""
    d = put(g, 1, dfn, bf)
    a = put(g, 0, att)
    g.apply(("move", (a.uid,), bf))
    return a, d


# ------------------------------------------------------------------ static battlefields
@T.test
def trifarian_war_camp_plus_one_here():
    g, _ = new(bf1="Trifarian War Camp")
    a = put(g, 0, "Soaring Scout", 1)
    b = put(g, 0, "Soaring Scout")
    assert g.might(a) == 2 and g.might(b) == 1


@T.test
def kinkou_temple_tank_units_here():
    g, _ = new(bf1="Kinkou Temple")
    t = put(g, 0, "Blitzcrank, Impassive", 1)
    n = put(g, 0, "Soaring Scout", 1)
    t2 = put(g, 0, "Blitzcrank, Impassive")
    assert g.might(t) == 6 and g.might(n) == 1 and g.might(t2) == 5


@T.test
def black_flame_altar_temporary_units_have_shield():
    g, _ = new(bf1="Black Flame Altar")
    u = put(g, 0, "Soaring Scout", 1)
    v = put(g, 0, "Soaring Scout", 1)
    w = put(g, 0, "Soaring Scout")
    for x in (u, w):
        g.grant(x, "Temporary", 1, None)
    assert g.has_kw(u, "Shield") and not g.has_kw(v, "Shield") and not g.has_kw(w, "Shield")
    u.desig = "def"
    assert g.might(u) == 2


# ------------------------------------------------------------------ hold
@T.test
def altar_to_unity_recruit_in_base():
    g, _ = new(bf1="Altar to Unity")
    hold_with(g, 0, ["Soaring Scout"], 1)
    rec = [u for u in g.units(0) if u.cname == "Recruit"]
    assert len(rec) == 1 and rec[0].loc == "base" and rec[0].token and g.might(rec[0]) == 1


@T.test
def amateur_recital_moves_unit_to_base():
    g, _ = new(bf1="Amateur Recital")
    e = put(g, 1, "Glasc Mixologist", 0)
    hold_with(g, 0, ["Soaring Scout"], 1)
    assert e.loc == "base"


@T.test
def grove_draws_on_hold():
    g, _ = new(bf1="Grove of the God-Willow")
    deck_top(g, 0, ["Discipline"])
    hold_with(g, 0, ["Soaring Scout"], 1)
    assert [c.cname for c in g.p[0].hand] == ["Discipline"]


@T.test
def hallowed_tomb_returns_chosen_champion():
    g, _ = new(bf1="Hallowed Tomb")
    pl = g.p[0]
    c = pl.champ[0]
    g.emit("main_start", pid=0)                 # the battlefield notes the Chosen Champion
    pl.champ.remove(c)
    c.zone = "trash"
    pl.trash.append(c)
    hold_with(g, 0, ["Soaring Scout"], 1)
    assert pl.champ == [c] and c.zone == "champ" and c not in pl.trash


@T.test
def hallowed_tomb_other_copy_not_returned():
    g, _ = new(bf1="Hallowed Tomb")
    pl = g.p[0]
    g.emit("main_start", pid=0)
    c = pl.champ.pop()
    other = Obj(c.name, 0)                      # another copy of the champion card, in the trash
    other.zone = "trash"
    pl.trash.append(other)
    hold_with(g, 0, ["Soaring Scout"], 1)
    assert pl.champ == [] and other in pl.trash


@T.test
def navori_buffs_unit_here():
    g, _ = new(bf1="Navori Fighting Pit")
    u, = hold_with(g, 0, ["Soaring Scout"], 1)
    assert u.buff == 1 and g.might(u) == 2


@T.test
def power_nexus_pays_four_runes_for_a_point():
    g, _ = new(bf1="Power Nexus")
    runes(g, 0, ["Calm"] * 5)
    put(g, 0, "Soaring Scout", 1)
    before = g.p[0].points
    g.hold(0, g.bfs[1])
    settle(g)
    assert g.p[0].points == before + 2 and len(g.p[0].runes) == 1


@T.test
def power_nexus_unpayable_no_point():
    g, _ = new(bf1="Power Nexus")
    runes(g, 0, ["Calm"] * 3)
    put(g, 0, "Soaring Scout", 1)
    before = g.p[0].points
    g.hold(0, g.bfs[1])
    settle(g)
    assert g.p[0].points == before + 1 and len(g.p[0].runes) == 3


@T.test
def reckoners_arena_activates_conquer_effects():
    g, _ = new(bf1="Reckoner's Arena")
    deck_top(g, 0, ["Discipline", "Block"])
    hold_with(g, 0, ["Kai'Sa, Survivor", "Soaring Scout"], 1)
    assert [c.cname for c in g.p[0].hand] == ["Discipline"]          # Kai'Sa: "When I conquer, draw 1"


@T.test
def reckoners_arena_hunt_triggers_again():
    g, _ = new(bf1="Reckoner's Arena")
    with temp_card("Soaring Scout", kw={"Hunt": 1}):
        hold_with(g, 0, ["Soaring Scout"], 1)
        assert g.p[0].xp == 2                                       # hold + activated conquer


@T.test
def shadow_temple_burns_3():
    g, _ = new(bf1="Shadow Temple")
    n, t = len(g.p[0].deck), len(g.p[0].trash)
    hold_with(g, 0, ["Soaring Scout"], 1)
    assert len(g.p[0].deck) == n - 3 and len(g.p[0].trash) == t + 3


@T.test
def startipped_peak_channels_exhausted():
    g, _ = new(bf1="Startipped Peak")
    runes(g, 0, [])
    hold_with(g, 0, ["Soaring Scout"], 1)
    assert len(g.p[0].runes) == 1 and g.p[0].runes[0].exhausted


@T.test
def grand_plaza_wins_with_seven_units():
    g, _ = new(bf1="The Grand Plaza")
    hold_with(g, 0, ["Soaring Scout"] * 7, 1)
    assert g.winner == 0


@T.test
def grand_plaza_six_units_no_win():
    g, _ = new(bf1="The Grand Plaza")
    hold_with(g, 0, ["Soaring Scout"] * 6, 1)
    assert g.winner is None


@T.test
def papertree_each_player_channels():
    g, _ = new(bf1="The Papertree")
    runes(g, 0, []); runes(g, 1, [])
    hold_with(g, 0, ["Soaring Scout"], 1)
    assert len(g.p[0].runes) == 1 and len(g.p[1].runes) == 1
    assert g.p[0].runes[0].exhausted and g.p[1].runes[0].exhausted


# ------------------------------------------------------------------ conquer
@T.test
def emperors_dais_returns_unit_and_plays_sand_soldier():
    g, _ = new(bf1="Emperor's Dais")
    u = conquer_with(g, 0, "Soaring Scout", 1)
    assert u in g.p[0].hand
    ss = [x for x in g.units(0) if x.cname == "Sand Soldier"]
    assert len(ss) == 1 and ss[0].loc == 1 and g.might(ss[0]) == 2 and g.bfs[1].ctrl == 0


@T.test
def emperors_dais_no_energy_nothing():
    g, _ = new(bf1="Emperor's Dais")
    u = conquer_with(g, 0, "Soaring Scout", 1, extra_runes=0)
    assert u in g.board and not [x for x in g.units(0) if x.cname == "Sand Soldier"]


@T.test
def hall_of_legends_readies_legend():
    g, _ = new(bf1="Hall of Legends")
    g.p[0].legend.exhausted = True
    conquer_with(g, 0, "Soaring Scout", 1)
    assert not g.p[0].legend.exhausted
    assert sum(r.exhausted for r in g.p[0].runes) == 1


@T.test
def minefield_trashes_two():
    g, _ = new(bf1="Minefield")
    n, t = len(g.p[0].deck), len(g.p[0].trash)
    conquer_with(g, 0, "Soaring Scout", 1)
    assert len(g.p[0].deck) == n - 2 and len(g.p[0].trash) == t + 2


@T.test
def monastery_spends_buff_to_draw():
    g, _ = new(bf1="Monastery of Hirana")
    b = put(g, 0, "Soaring Scout")
    b.buff = 1
    deck_top(g, 0, ["Discipline"])
    conquer_with(g, 0, "Soaring Scout", 1)
    assert b.buff == 0 and [c.cname for c in g.p[0].hand] == ["Discipline"]


@T.test
def monastery_without_buff_no_draw():
    g, _ = new(bf1="Monastery of Hirana")
    conquer_with(g, 0, "Soaring Scout", 1)
    assert g.p[0].hand == []


@T.test
def protective_sands_draw_with_few_runes():
    g, _ = new(bf1="Protective Sands")
    conquer_with(g, 0, "Soaring Scout", 1, extra_runes=4)
    assert len(g.p[0].hand) == 1


@T.test
def protective_sands_no_draw_with_five_runes():
    g, _ = new(bf1="Protective Sands")
    conquer_with(g, 0, "Soaring Scout", 1, extra_runes=5)
    assert len(g.p[0].hand) == 0


@T.test
def seat_of_power_draws_per_other_battlefield():
    g, _ = new(bf1="Seat of Power")
    put(g, 0, "Soaring Scout", 0)                # P0 controls the other battlefield
    conquer_with(g, 0, "Soaring Scout", 1)
    assert len(g.p[0].hand) == 1


@T.test
def seat_of_power_alone_draws_nothing():
    g, _ = new(bf1="Seat of Power")
    conquer_with(g, 0, "Soaring Scout", 1)
    assert len(g.p[0].hand) == 0


@T.test
def sunken_temple_needs_mighty():
    g, _ = new(bf1="Sunken Temple")
    conquer_with(g, 0, "Glasc Mixologist", 1)
    assert len(g.p[0].hand) == 1
    g, _ = new(bf1="Sunken Temple")
    conquer_with(g, 0, "Soaring Scout", 1)
    assert len(g.p[0].hand) == 0


@T.test
def candlelit_sanctum_looks_at_two_and_recycles():
    g, _ = new(bf1="The Candlelit Sanctum", answers0={"predict_recycle": True})
    a, b = deck_top(g, 0, ["Discipline", "Block"])
    conquer_with(g, 0, "Soaring Scout", 1)
    assert g.p[0].deck[-2:] in ([a, b], [b, a]) and g.p[0].deck[0] not in (a, b)


@T.test
def treasure_hoard_gold_exhausted():
    g, _ = new(bf1="Treasure Hoard")
    conquer_with(g, 0, "Soaring Scout", 1)
    gold = [x for x in g.gear(0) if x.cname == "Gold"]
    assert len(gold) == 1 and gold[0].exhausted and gold[0].loc == "base"


@T.test
def veiled_temple_readies_and_detaches():
    g, _ = new(bf1="Veiled Temple", answers0={"veiled_detach": True})
    runes(g, 0, ["Calm"] * 4)
    u = put(g, 0, "Soaring Scout")
    s = put(g, 0, "Long Sword")
    attach(g, s, u)
    s.exhausted = True
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert not s.exhausted and s.attached_to is None and u.attached == [] and s.loc == "base"


@T.test
def zaun_warrens_discard_then_draw():
    g, _ = new(bf1="Zaun Warrens")
    h = hand(g, 0, "Block")
    deck_top(g, 0, ["Discipline"])
    conquer_with(g, 0, "Soaring Scout", 1)
    assert h in g.p[0].trash and [c.cname for c in g.p[0].hand] == ["Discipline"]


@T.test
def trapping_grounds_excess_damage_bird():
    g, _ = new(bf1="Trapping Grounds")
    attack(g, 1, att="Glasc Mixologist", dfn="Black Rose Dignitary")      # 5 vs 2: 3 excess
    settle(g)
    birds = [u for u in g.units(0) if u.cname == "Bird"]
    assert g.bfs[1].ctrl == 0 and len(birds) == 1 and g.has_kw(birds[0], "Deflect")


@T.test
def trapping_grounds_two_excess_no_bird():
    g, _ = new(bf1="Trapping Grounds")
    attack(g, 1, att="Glasc Mixologist", dfn="Stellacorn Herder")         # 5 vs 3: 2 excess
    settle(g)
    assert g.bfs[1].ctrl == 0 and not [u for u in g.units(0) if u.cname == "Bird"]


@T.test
def trapping_grounds_conquer_without_combat_no_bird():
    g, _ = new(bf1="Trapping Grounds")
    conquer_with(g, 0, "Glasc Mixologist", 1)
    assert not [u for u in g.units(0) if u.cname == "Bird"]


# ------------------------------------------------------------------ defend
@T.test
def fortified_position_shield_2_this_combat():
    g, _ = new(bf1="Fortified Position")
    a, d = attack(g, 1)                          # Witness 2 vs Scout 1 (+2 Shield)
    settle(g)
    assert d in g.board and a not in g.board and g.bfs[1].ctrl == 1
    assert not g.has_kw(d, "Shield")             # the grant ended with the combat


@T.test
def ravenbloom_spell_to_hand():
    g, _ = new(bf1="Ravenbloom Conservatory")
    c, = deck_top(g, 1, ["Discipline"])
    attack(g, 1)
    settle(g)
    assert c in g.p[1].hand


@T.test
def ravenbloom_other_recycled():
    g, _ = new(bf1="Ravenbloom Conservatory")
    c, = deck_top(g, 1, ["Soaring Scout"])
    attack(g, 1)
    settle(g)
    assert c not in g.p[1].hand and g.p[1].deck[-1] is c


@T.test
def reavers_row_defender_moves_to_base():
    g, _ = new(bf1="Reaver's Row")
    a, d = attack(g, 1, att="Soaring Scout", dfn="Glasc Mixologist")
    settle(g)
    assert d.loc == "base" and d in g.board and g.bfs[1].ctrl == 0


# ------------------------------------------------------------------ play / choose
@T.test
def abandoned_hall_spell_gives_plus_one():
    g, _ = new(bf1="Abandoned Hall")
    runes(g, 0, ["Calm"] * 2)
    u = put(g, 0, "Soaring Scout", 1)
    b = put(g, 0, "Soaring Scout")
    hand(g, 0, "Discipline")
    g.apply(opt(g, 0, "Discipline", lambda ch: ch.get("tg") == (b.uid,)))
    settle(g)
    assert g.might(b) == 3 and g.might(u) == 2


@T.test
def valley_of_idols_buffs_unit_played_here():
    g, _ = new(bf1="Valley of Idols")
    runes(g, 0, ["Order"] * 3)
    put(g, 0, "Soaring Scout", 1)
    hand(g, 0, "Soaring Scout")
    g.apply(opt(g, 0, "Soaring Scout", lambda ch: ch.get("loc") == 1))
    settle(g)
    new_u = [u for u in g.units(0, 1)]
    assert sorted(u.buff for u in new_u) == [0, 1]
    assert all(r.exhausted for r in g.p[0].runes)


@T.test
def dreaming_tree_draw_first_time_each_turn():
    g, _ = new(bf1="The Dreaming Tree")
    runes(g, 0, ["Calm"] * 4)
    u = put(g, 0, "Soaring Scout", 1)
    deck_top(g, 0, ["Block", "Block", "Block", "Block"])
    hand(g, 0, "Discipline")
    hand(g, 0, "Discipline")
    g.apply(opt(g, 0, "Discipline", lambda ch: ch.get("tg") == (u.uid,)))
    settle(g)
    assert len(g.p[0].hand) == 1 + 2              # 1 Discipline left, Tree draw + Discipline draw
    g.apply(opt(g, 0, "Discipline", lambda ch: ch.get("tg") == (u.uid,)))
    settle(g)
    assert len(g.p[0].hand) == 3                  # second time this turn: only Discipline's draw


# ------------------------------------------------------------------ beginning phase
@T.test
def frozen_fortress_deals_one_to_each_unit_here():
    g, _ = new(bf1="Frozen Fortress")
    a = put(g, 0, "Soaring Scout", 1)
    b = put(g, 0, "Black Rose Dignitary", 1)
    c = put(g, 0, "Soaring Scout")
    g.emit("beginning_start", pid=0)
    settle(g)
    assert a not in g.board and b.damage == 1 and c.damage == 0


@T.test
def frozen_fortress_before_scoring_in_a_real_turn():
    g, _ = new(bf1="Frozen Fortress")
    put(g, 1, "Soaring Scout", 1)
    pts = g.p[1].points
    g.apply(("end",))
    settle(g)
    assert g.tp == 1 and not g.units(1, 1) and g.p[1].points == pts        # the 1 might unit died before holding


@T.test
def obelisk_first_beginning_phase_channels():
    g, _ = new(bf1="Obelisk of Power")
    runes(g, 0, [])
    g.p[0].turns = 1
    g.emit("beginning_start", pid=0)
    settle(g)
    assert len(g.p[0].runes) == 1 and not g.p[0].runes[0].exhausted
    g.p[0].turns = 2
    g.emit("beginning_start", pid=0)
    settle(g)
    assert len(g.p[0].runes) == 1


@T.test
def arenas_greatest_first_beginning_point():
    g, _ = new(bf1="The Arena's Greatest")
    g.p[1].turns = 1
    g.emit("beginning_start", pid=1)
    settle(g)
    assert g.p[1].points == 1
    g.p[1].turns = 2
    g.emit("beginning_start", pid=1)
    settle(g)
    assert g.p[1].points == 1


# ------------------------------------------------------------------ random games with these battlefields
# ------------------------------------------------------------------ battlefields using the engine hooks (integration)
@T.test
def integration_battlefields_registered():
    for n in ("Brush", "Marai Spire", "Mystic Vortex", "Ornn's Forge", "Piltovan Forge", "Risen Altar",
              "Sandswept Tomb", "Vaults of Helia", "Heisho, Shell of the World", "Rockfall Path", "Dragon Roost",
              "Vilemaw's Lair", "Forge of the Fluft", "Gardens of Becoming", "Altar of Blood", "Ripper's Bay",
              "Forgotten Library", "The Academy", "Bandle Tree"):
        assert n in IMPL and IMPL[n].module == "cardsets.battlefields", n
    assert "Baron Pit" not in IMPL


def _costs(g, pid, c, src="hand", pred=lambda ch: True, closed=False):
    out = []
    for ch in card_choices(g, pid, c, src, closed, closed):
        if pred(ch):
            e, r = total_cost(g, pid, c, ch, src)
            out.append((e, len(r)))
    return sorted(out)


@T.test
def brush_aura_and_swap_back():
    g, _ = new()
    u = put(g, 0, "Soaring Scout", 1)            # a Bird
    w = put(g, 0, "Watchful Sentry", 1)
    old = g.bfs[1].name
    g.replace_battlefield(g.bfs[1], "Brush")
    assert g.might(u) == 2 and g.might(w) == 1 and g.bfs[1].ctrl == 0
    g.hold(0, g.bfs[1])
    settle(g)
    assert g.bfs[1].name == old and g.bfs[1].replaced is None and g.might(u) == 1


@T.test
def marai_spire_repeat_discount():
    g, _ = new(bf1="Marai Spire")
    runes(g, 0, ["Calm"] * 6)
    put(g, 1, "Soaring Scout", 1)
    c = hand(g, 0, "Bellows Breath")
    full = _costs(g, 0, c, pred=lambda ch: ch.get("rep"))
    g.bfs[1].ctrl = 0
    assert [e for e, _ in _costs(g, 0, c, pred=lambda ch: ch.get("rep"))] == [e - 1 for e, _ in full]


@T.test
def mystic_vortex_reaction_tax_in_showdowns_here():
    from game import Showdown
    g, _ = new(bf1="Mystic Vortex")
    runes(g, 0, ["Calm"] * 4)
    c = hand(g, 0, "Stupefy")
    base = _costs(g, 0, c)
    g.sd = Showdown(1, False, 0)
    assert [n for _, n in _costs(g, 0, c, closed=True)] == [n + 1 for _, n in base][:len(_costs(g, 0, c, closed=True))]


@T.test
def ornns_forge_first_gear_discount():
    g, _ = new(bf1="Ornn's Forge")
    g.bfs[1].ctrl = 0
    runes(g, 0, ["Fury"] * 6)
    pb = hand(g, 0, "Pendulum Blade")
    assert _costs(g, 0, pb)[0][0] == 2
    g.hist["gear_played"][0] = 1
    assert _costs(g, 0, pb)[0][0] == 3


@T.test
def piltovan_forge_first_gear_ability_discount():
    from actions import ability_cost, abilities_of
    g, _ = new(bf1="Piltovan Forge")
    g.bfs[1].ctrl = 0
    bh = put(g, 0, "Baited Hook")
    ab = abilities_of(g, bh)[0]
    assert ability_cost(g, 0, bh, ab, dict())[0] == 0
    g.hist["gear_abs"][0] = 1
    assert ability_cost(g, 0, bh, ab, dict())[0] == 1


@T.test
def risen_altar_empower_discount():
    from actions import ability_cost, abilities_of
    g, _ = new(bf1="Risen Altar")
    runes(g, 0, ["Fury"] * 3)
    a = put(g, 0, "Akali, Deadly Weapon", 1)
    b = put(g, 0, "Akali, Deadly Weapon")
    ea = [x for x in abilities_of(g, a) if x["name"] == "Empower"][0]
    ca, cb = ability_cost(g, 0, a, ea, dict()), ability_cost(g, 0, b, ea, dict())
    assert ca[0] + len(ca[1]) == cb[0] + len(cb[1]) - 1


@T.test
def sandswept_tomb_spell_on_friendly_unit_here():
    g, _ = new(bf1="Sandswept Tomb")
    runes(g, 0, ["Fury"] * 4)
    u = put(g, 0, "Soaring Scout", 1)
    v = put(g, 0, "Soaring Scout")
    fs = hand(g, 0, "Falling Star")
    assert len(total_cost(g, 0, fs, dict(tg=(u.uid, v.uid)), "hand")[1]) == 1
    assert len(total_cost(g, 0, fs, dict(tg=(v.uid, v.uid)), "hand")[1]) == 2
    e = put(g, 1, "Soaring Scout", 1)
    assert len(total_cost(g, 0, fs, dict(tg=(e.uid, e.uid)), "hand")[1]) == 2     # not friendly to it


@T.test
def vaults_of_helia_units_cost_more_after_hold():
    g, _ = new(bf1="Vaults of Helia")
    runes(g, 0, ["Calm"] * 4)
    c = hand(g, 0, "Soaring Scout")
    assert _costs(g, 0, c)[0][0] == 2
    hold_with(g, 0, ["Watchful Sentry"], 1)
    assert _costs(g, 0, c)[0][0] == 3


@T.test
def heisho_ignores_deflect_here():
    from actions import deflect_reqs
    g, _ = new(bf1="Heisho, Shell of the World")
    e = put(g, 1, "Irelia, Fervent", 1)
    f = put(g, 1, "Irelia, Fervent", 0)
    assert deflect_reqs(g, 0, dict(tg=(e.uid,))) == [] and len(deflect_reqs(g, 0, dict(tg=(f.uid,)))) == 1


@T.test
def rockfall_path_no_units_played_here():
    from actions import loc_allowed
    g, _ = new(bf1="Rockfall Path")
    g.bfs[1].ctrl = 0
    runes(g, 0, ["Calm"] * 4)
    c = hand(g, 0, "Soaring Scout")
    assert [ch["loc"] for ch in card_choices(g, 0, c, "hand", False, False)] == ["base"]
    assert make_token(g, "Recruit", 0, 1) is None


@T.test
def dragon_roost_pay_two_to_play_dragon_here():
    g, _ = new(bf1="Dragon Roost")
    runes(g, 0, ["Order"] * 12)
    d = hand(g, 0, "Harnessed Dragon")
    put(g, 1, "Soaring Scout")
    chs = card_choices(g, 0, d, "hand", False, False)
    roost = [ch for ch in chs if ch.get("roost")]
    assert roost and roost[0]["loc"] == 1
    assert len(total_cost(g, 0, d, roost[0], "hand")[1]) == len(total_cost(g, 0, d, chs[0], "hand")[1]) + 2
    s_ = hand(g, 0, "Soaring Scout")
    assert not [ch for ch in card_choices(g, 0, s_, "hand", False, False) if ch.get("roost")]


@T.test
def vilemaws_lair_no_moves_to_base():
    g, _ = new(bf1="Vilemaw's Lair")
    u = put(g, 0, "Soaring Scout", 1)
    assert not [o for o in options_of(g, "move") if o[2] == "base"]
    g.move([u], "base", 0)
    assert u.loc == 1
    g.recall(u)
    assert u.loc == "base"


@T.test
def forge_of_the_fluft_legend_attaches():
    from actions import abilities_of
    g, _ = new(bf1="Forge of the Fluft")
    u = put(g, 0, "Soaring Scout")
    pb = put(g, 0, "Pendulum Blade")
    assert not [a for a in abilities_of(g, g.p[0].legend) if a["name"] == "Attach"]
    put(g, 0, "Watchful Sentry", 1)                 # controls the battlefield
    opts = [o for o in act_options(g, 0) if o[1] == ("legend", 0)
            and abilities_of(g, g.p[0].legend)[o[2]]["name"] == "Attach"]
    g.apply(opts[0])
    settle(g)
    assert pb.attached_to == u.uid and g.p[0].legend.exhausted


@T.test
def gardens_of_becoming_units_gain_xp():
    g, _ = new(bf1="Gardens of Becoming")
    u = put(g, 0, "Soaring Scout", 1)
    put(g, 0, "Soaring Scout")
    acts = act_options(g, 0, "Soaring Scout")
    assert len(acts) == 1 and acts[0][1] == u.uid
    g.apply(acts[0])
    settle(g)
    assert g.p[0].xp == 1 and u.exhausted


@T.test
def altar_of_blood_saves_in_combat():
    g, _ = new(bf1="Altar of Blood")
    runes(g, 1, ["Calm"] * 3)
    a, d = attack(g, 1, att="Glasc Mixologist")
    settle(g)
    assert d.zone == "board" and d.loc == "base" and d.exhausted and d.damage == 0
    g2, _ = new(bf1="Altar of Blood")
    runes(g2, 1, ["Calm"] * 3)
    x = put(g2, 1, "Soaring Scout", 1)
    g2.kill([x], 0)
    assert x.zone == "trash"                          # not during combat


@T.test
def rippers_bay_channel_on_return():
    g, _ = new(bf1="Ripper's Bay")
    runes(g, 1, ["Calm"])
    u = put(g, 1, "Soaring Scout", 1)
    n = len(g.p[1].runes)
    g.to_zone(u, "hand")
    settle(g)
    assert len(g.p[1].runes) == n + 1 and g.p[1].runes[-1].exhausted


@T.test
def forgotten_library_predicts_after_big_spell():
    from game import Item
    g, _ = new(bf1="Forgotten Library")
    g.bfs[1].ctrl = 0
    deck_top(g, 0, ["Soaring Scout"])
    c = Obj("Falling Star", 0)
    c.zone = "trash"
    g.played_event(0, c, Item("spell", 0, c.cname, card=c, data=dict(paid_e=3)))
    assert not [t for t in g.trigq if t.name == "Forgotten Library"]
    g.played_event(0, c, Item("spell", 0, c.cname, card=c, data=dict(paid_e=4)))
    assert [t for t in g.trigq if t.name == "Forgotten Library"]


@T.test
def the_academy_next_spell_repeat():
    g, _ = new(bf1="The Academy")
    hold_with(g, 0, ["Soaring Scout"], 1)
    runes(g, 0, ["Fury"] * 8)
    put(g, 1, "Mountain Drake")
    fs = hand(g, 0, "Falling Star")
    assert (4, 4) in _costs(g, 0, fs, pred=lambda ch: bool(ch.get("reps")))


@T.test
def bandle_tree_two_hidden_cards():
    from actions import hide_slots
    g, _ = new(bf1="Bandle Tree")
    assert hide_slots(g, g.bfs[1]) == 2 and hide_slots(g, g.bfs[0]) == 1


def _fuzz(n_per_bf=2):
    import hashlib, re
    from agents import RandomAgent
    from decks import load, DECKS
    from game import Item
    mine = sorted(k for k, v in IMPL.items() if v.module == "cardsets.battlefields" and not cards.is_token(k))
    keys = sorted(DECKS)

    def play(seed, bf_a, bf_b):
        ds = [load(keys[seed % len(keys)], bf_a), load(keys[(seed + 5) % len(keys)], bf_b)]
        ag = [RandomAgent(seed), RandomAgent(seed + 7919)]
        Obj._n = Item._n = 0
        g = Game(ds, ag, seed=seed, log=True)
        n = 0
        while True:
            d = g.advance()
            if d is None or n > 4000:
                break
            g.apply(ag[d.player].decide(g, d))
            n += 1
        return hashlib.md5("\n".join(re.sub(r"#\d+", "", l) for l in g.lines).encode()).hexdigest()

    seed = 0
    for i, bf in enumerate(mine):
        for k in range(n_per_bf):
            other = mine[(i + 1 + k) % len(mine)]
            h = play(seed, bf, other)
            if k == 0 and i % 5 == 0:
                assert play(seed, bf, other) == h, f"{bf}: seed {seed} not deterministic"
            seed += 1
    return seed


@T.test
def fuzz_random_games_with_these_battlefields():
    _fuzz(int(os.environ.get("RB_BF_FUZZ", "2")))


@T.test
def valley_of_idols_without_energy_asks_nothing_and_says_why():
    # Retour utilisateur 2026-10-09 : « oui » à Valley of Idols, mais ni énergie payée ni buff. Sans de quoi payer 1 énergie,
    # la question n'est plus posée et le journal explique pourquoi ; avec de quoi payer, l'énergie part et le buff est posé.
    for nr in (2, 3):
        asked = []
        g, _ = new(bf1="Valley of Idols", answers0={"may": lambda o, c: (asked.append(1), True)[1]})
        runes(g, 0, ["Order"] * nr)                   # Soaring Scout coûte 2
        put(g, 0, "Soaring Scout", 1)
        hand(g, 0, "Soaring Scout")
        g.apply(opt(g, 0, "Soaring Scout", lambda ch: ch.get("loc") == 1))
        settle(g)
        buffs = sorted(u.buff for u in g.units(0, 1))
        if nr == 2:
            assert asked == [] and buffs == [0, 0] and any("pas de quoi payer" in l for l in g.lines), (asked, buffs)
        else:
            assert asked == [1] and buffs == [0, 1] and all(r.exhausted for r in g.p[0].runes), (asked, buffs)


@T.test
def optional_cost_that_cannot_be_paid_is_not_offered_to_a_human():
    # Demande de l'utilisateur : « si le coût supplémentaire ne peut pas être payé, ne propose pas d'activer le bf ».
    # Monastery of Hirana : « When you conquer here, you may spend a buff to draw 1 » ; sans unité buffée, pas de question.
    for buffed in (False, True):
        asked = []
        g, ag = new(bf1="Monastery of Hirana", answers0={"may": lambda o, c: (asked.append(1), True)[1]})
        ag[0].every_choice = True                     # joueur humain : le coût est essayé sur une copie de la partie
        u = put(g, 0, "Arena Kingpin", "base")
        u.buff = 1 if buffed else 0
        g.bfs[1].ctrl = None
        g.apply([o for o in options_of(g, "move") if o[1] == (u.uid,) and o[2] == 1][0])
        settle(g)
        assert g.bfs[1].ctrl == 0
        if buffed:
            assert asked == [1] and g.obj(u.uid).buff == 0, asked
        else:
            assert asked == [] and any("pas de quoi payer" in l for l in g.lines), (asked, g.lines[-6:])


if __name__ == "__main__":
    T.main()
