"""Tests of batch radiance (set Radiance, Core Rules 2026-10-09). Run: python3 cardsets/test_radiance.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from cards import make_token                            # noqa: E402
from game import ANY, DOMAINS                           # noqa: E402
from actions import card_choices, move_options, hide_slots    # noqa: E402

T = Suite("radiance")

V3, V4, V5, V8 = "Shipyard Skulker", "Vanguard Sergeant", "Playful Phantom", "Mega-Mech"     # vanilla units


def N(**kw):
    """new() with two battlefields without abilities."""
    return new(bf0="Test Field A", bf1="Test Field B", **kw)


def R(g, pid, n=3):
    """n runes of each domain."""
    runes(g, pid, [d for d in DOMAINS for _ in range(n)])


def play(g, pid, name, pred=lambda ch: True, src=None):
    g.apply(opt(g, pid, name, pred, src))
    return settle(g)


def base(ch):
    return ch.get("loc") == "base" and not ch.get("acc")


def unit(g, name, pid=None):
    us = [u for u in g.units(pid) if u.cname == name]
    assert us, (name, g.board)
    return us[-1]


def gear(g, name, pid=None):
    xs = [x for x in g.gear(pid) if x.cname == name]
    assert xs, (name, g.board)
    return xs[-1]


def trash(g, pid, name):
    o = Obj(name, pid)
    o.zone = "trash"
    g.p[pid].trash.append(o)
    return o


def facedown(g, pid, name, bf):
    if not g.units(pid, bf):
        put(g, pid, V3, bf)
    g.bfs[bf].ctrl = pid
    o = Obj(name, pid)
    o.zone = "facedown"
    o.hidden_turn = g.turn_no - 1
    o.hidden_bf = bf
    g.bfs[bf].facedowns.append(o)
    return o


def respond(g, pid, name, pred=lambda ch: True, src=None):
    d = g.advance()
    assert d.kind == "priority" and d.player == 1 - pid, d
    g.apply(("pass",))
    g.apply(opt(g, pid, name, pred, src))
    return settle(g)


def act(g, pid, cname, pred=lambda ch: True, i=None):
    for o in act_options(g, pid, cname):
        if pred(o[3]) and (i is None or o[2] == i):
            g.apply(o)
            return settle(g)
    raise AssertionError(f"no ability of {cname}: {act_options(g, pid)}")


def legend(g, pid, name):
    g.p[pid].legend_name = name
    g.p[pid].legend.name = name
    return g.p[pid].legend


def move(g, units, dest):
    g.apply(("move", tuple(u.uid for u in units), dest))
    return settle(g)


def end_turn(g):
    """End the turn: the end of turn triggers resolve, the next turn begins."""
    g.apply(("end",))
    return settle(g)


def bombs(g, pid=None):
    return [x for x in g.board if x.cname == "Bomb" and (pid is None or x.ctrl == pid)]


# ====================================================================== engine: Deploy, Disarm, Show Off, Bomb
@T.test
def deploy_gear_is_played_only_to_a_battlefield_and_stays_there():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Blossoming Dreamroot")
    locs = {o[3]["loc"] for o in options_of(g, "play") if g.p[0].hand and o[1] == g.p[0].hand[0].uid}
    assert locs == {0, 1}, locs                          # any battlefield, even an open one (830.1.b.1)
    play(g, 0, "Blossoming Dreamroot", lambda ch: ch["loc"] == 1)
    x = gear(g, "Blossoming Dreamroot")
    assert x.loc == 1                                    # not recalled at cleanup (830.1.b.3)
    assert g.bfs[1].ctrl is None                         # gear don't take control


@T.test
def deploy_gear_dies_when_an_opponent_holds_there():
    g, _ = N()
    x = put(g, 0, "Blossoming Dreamroot", 1, bf_control=False)
    put(g, 1, V3, 1)
    g.bfs[1].ctrl = 1
    g.hold(1, g.bfs[1])
    settle(g)
    assert x.zone == "trash"


@T.test
def deploy_gear_survives_its_controllers_hold():
    g, _ = N()
    x = put(g, 0, "Blossoming Dreamroot", 1)
    put(g, 0, V3, 1)
    g.hold(0, g.bfs[1])
    settle(g)
    assert x.zone == "board" and x.loc == 1


@T.test
def other_gear_at_a_battlefield_is_still_recalled():
    g, _ = N()
    x = put(g, 0, "Seal of Discord", 1)
    g.need_cleanup = True
    settle(g)
    assert x.loc == "base"


@T.test
def deploy_gear_played_by_an_effect_goes_to_a_battlefield():
    g, _ = N()
    c = trash(g, 0, "Blossoming Dreamroot")
    from cards import play_card
    play_card(g, 0, c, "trash", dict(loc="base", free=True), limited=True)
    assert c.zone == "board" and c.loc in (0, 1)


@T.test
def disarm_gives_minus_might_when_attacking():
    g, _ = N()
    a = put(g, 0, "Trinket Peddler")                       # [Disarm] (keyword-only)
    e = put(g, 1, V4, 1)
    move(g, [a], 1)
    assert e.zone == "trash" or g.might(e) == 3, (e.zone, g.might(e))
    assert any("Disarm" in l for l in g.lines)


@T.test
def disarm_values_add_up_rule_831_2():
    g, _ = N()
    put(g, 0, "Clanging Sentry"); put(g, 0, "Clanging Sentry")
    m = put(g, 0, "Mech")
    assert g.kw_value(m, "Disarm") == 2
    mc = put(g, 0, "Mysterious Chronomancer")
    assert g.kw_value(mc, "Disarm") == 2


@T.test
def show_off_choice_and_snapshot_rule_832_1_d_1():
    g, _ = N()
    R(g, 0)
    big = put(g, 0, V5)                                    # 5 might, shown off
    e = put(g, 1, V5, 1)
    hand(g, 0, "Primordial Roar")
    g.apply(opt(g, 0, "Primordial Roar", lambda ch: ch.get("show") == big.uid and ch.get("tg") == (e.uid,)))
    g.mod(big, -3)                                         # later changes are not considered
    settle(g)
    assert e.zone == "trash"


@T.test
def without_show_off_primordial_roar_does_nothing():
    g, _ = N()
    R(g, 0)
    e = put(g, 1, V3, 1)
    hand(g, 0, "Primordial Roar")
    play(g, 0, "Primordial Roar", lambda ch: ch.get("show") is None)
    assert e.zone == "board" and e.damage == 0


@T.test
def bomb_deathknell_hits_an_enemy_and_kills_other_bombs():
    g, _ = N()
    b1 = make_token(g, "Bomb", 0, 1, ready=True)
    b2 = make_token(g, "Bomb", 0, 1, ready=True)
    e = put(g, 1, V3, 1, bf_control=False)
    g.kill([b1], 0)
    settle(g)
    assert b2.zone != "board" and e.zone == "trash"        # 2 + 2 damage


@T.test
def bomb_without_enemy_still_kills_other_bombs():
    g, _ = N()
    b1 = make_token(g, "Bomb", 0, 0, ready=True)
    b2 = make_token(g, "Bomb", 0, 0, ready=True)
    g.kill([b1], 0)
    settle(g)
    assert not bombs(g)


@T.test
def exhausting_emits_one_event_per_action():
    g, _ = N()
    put(g, 0, "Seraphine, Inspiring")
    us = [put(g, 0, V3) for _ in range(3)]
    n = len(g.p[0].hand)
    deck_top(g, 0, [V4])
    move(g, us, 0)                                          # three units exhausted at the same time
    assert len(g.p[0].hand) == n + 1


@T.test
def entering_exhausted_is_not_becoming_exhausted():
    g, _ = N()
    o = put(g, 0, "Strigian Oracle", ready=False)
    g.need_cleanup = True
    settle(g)
    assert not any("Strigian Oracle" in l and "trigger" in l for l in g.lines)


# ====================================================================== Fury
@T.test
def amateur_demolitionist_plays_a_bomb():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Amateur Demolitionist")
    play(g, 0, "Amateur Demolitionist", base)
    assert len(bombs(g, 0)) == 1 and bombs(g, 0)[0].loc in (0, 1)


@T.test
def banner_of_conquest_gives_assault_here():
    g, _ = N()
    b = put(g, 0, "Banner of Conquest", 0)
    u = put(g, 0, V3, 0)
    v = put(g, 0, V3, 1)
    assert g.kw_value(u, "Assault") == 1 and g.kw_value(v, "Assault") == 0


@T.test
def deploy_gear_is_shown_on_its_battlefield():
    """Table and replays: a gear with [Deploy] played to a battlefield is drawn there (rule 830), not in the base."""
    from replay import snap
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Banner of Conquest")
    play(g, 0, "Banner of Conquest", lambda ch: ch["loc"] == 1)
    st = snap(g)
    assert [x["n"] for x in st["bfs"][1]["g"]] == ["Banner of Conquest"] and st["bfs"][1]["g"][0]["c"] == 0
    assert not st["bfs"][0]["g"] and not st["p"][0]["gear"]


@T.test
def bilgewater_tracker_deals_1_to_two_units():
    g, _ = N()
    R(g, 0)
    e1, e2 = put(g, 1, "Recruit", 1), put(g, 1, "Recruit", 1)
    hand(g, 0, "Bilgewater Tracker")
    play(g, 0, "Bilgewater Tracker", base)
    assert e1.zone == "trash" and e2.zone == "trash"


@T.test
def brawl_assault_equal_to_might():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V4)
    hand(g, 0, "Brawl")
    play(g, 0, "Brawl", lambda ch: ch["tg"] == (u.uid,))
    assert g.kw_value(u, "Assault") == 4


@T.test
def lingering_animus_enters_ready_after_a_friendly_death():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Lingering Animus")
    play(g, 0, "Lingering Animus", base)
    assert unit(g, "Lingering Animus").exhausted
    v = put(g, 0, V3)
    g.kill([v], 0)
    hand(g, 0, "Lingering Animus")
    play(g, 0, "Lingering Animus", base)
    assert not unit(g, "Lingering Animus").exhausted


@T.test
def scorched_earth_splits_5_at_one_battlefield():
    g, _ = N()
    R(g, 0)
    a, b = put(g, 1, V3, 1), put(g, 1, "Recruit", 1)
    hand(g, 0, "Scorched Earth")
    play(g, 0, "Scorched Earth", lambda ch: set(ch["tg"]) == {a.uid, b.uid})
    assert a.zone == "trash" and b.zone == "trash"


@T.test
def rally_for_war_three_recruits_at_an_occupied_enemy_battlefield():
    g, _ = N()
    R(g, 0)
    put(g, 1, V3, 1)
    hand(g, 0, "Rally for War")
    g.apply(opt(g, 0, "Rally for War"))
    settle(g)
    assert sum("plays a Recruit token at 1" in l for l in g.lines) == 3


@T.test
def war_scout_gives_plus_2():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3)
    hand(g, 0, "War Scout")
    play(g, 0, "War Scout", base)
    assert g.might(u) == 5


@T.test
def antagonistic_apparatus_ready_mech_here():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Antagonistic Apparatus")
    play(g, 0, "Antagonistic Apparatus", base)
    a, m = unit(g, "Antagonistic Apparatus"), unit(g, "Mech")
    assert not a.exhausted and not m.exhausted and m.loc == "base"


@T.test
def forbidden_technique_readies_and_is_banished():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3, ready=False)
    c = hand(g, 0, "Forbidden Technique")
    play(g, 0, "Forbidden Technique", lambda ch: ch["tg"] == (u.uid,))
    assert not u.exhausted and c in g.p[0].banish


@T.test
def forbidden_technique_replays_from_trash_after_a_7_cost_card():
    g, _ = N()
    R(g, 0, 4)
    g.zone_listen = True                                    # a card put in the trash by the test
    ft = trash(g, 0, "Forbidden Technique")
    u = put(g, 0, V3, ready=False)
    hand(g, 0, V8)                                          # Mega-Mech costs 7
    play(g, 0, V8, base)
    assert ft in g.p[0].banish and not unit(g, V8).exhausted        # it readied the Mega-Mech


@T.test
def incorrigible_construct_enters_ready_when_showing_a_mech():
    g, _ = N()
    R(g, 0)
    m = put(g, 0, "Mech")
    hand(g, 0, "Incorrigible Construct")
    play(g, 0, "Incorrigible Construct", lambda ch: ch.get("show") == m.uid and ch["loc"] == "base")
    assert not unit(g, "Incorrigible Construct").exhausted


@T.test
def runeblade_anvil_banishes_and_lets_you_play_it():
    g, _ = N()
    R(g, 0)
    top = deck_top(g, 0, [V3])[0]
    hand(g, 0, "Runeblade Anvil")
    play(g, 0, "Runeblade Anvil")
    assert top in g.p[0].banish
    o = next(o for o in options_of(g, "play") if o[1] == top.uid)
    assert o[2] == "banish"
    g.apply(o)
    settle(g)
    assert top.zone == "board"


@T.test
def runeblade_anvil_kill_ability_and_leave_trigger():
    g, _ = N()
    R(g, 0)
    a = put(g, 0, "Runeblade Anvil")
    top = deck_top(g, 0, [V3])[0]
    act(g, 0, "Runeblade Anvil")
    assert a.zone == "trash" and top in g.p[0].banish


@T.test
def lost_to_the_sands_two_modes():
    g, _ = N()
    R(g, 0)
    e = put(g, 1, V4, 1)
    x = put(g, 1, "Seal of Discord")
    hand(g, 0, "Lost to the Sands"); hand(g, 0, "Lost to the Sands")
    play(g, 0, "Lost to the Sands", lambda ch: ch["mode"] == "deal")
    play(g, 0, "Lost to the Sands", lambda ch: ch["mode"] == "gear")
    assert e.zone == "trash" and x.zone == "trash"


@T.test
def shimmering_swimmer_kill_cost_deals_2():
    g, _ = N()
    R(g, 0)
    f = put(g, 0, "Recruit")
    e = put(g, 1, "Recruit", 1)
    hand(g, 0, "Shimmering Swimmer")
    play(g, 0, "Shimmering Swimmer", lambda ch: ch.get("kill") == f.uid and ch["loc"] == "base")
    assert f.zone != "board" and e.zone == "trash"


@T.test
def akali_brash_adds_then_banishes():
    g, _ = N()
    a = put(g, 0, "Akali, Brash")
    a.desig = None
    top = deck_top(g, 0, [V3])[0]
    g.move([a], 0, 0)
    assert g.p[0].pool_e == 1 and g.p[0].pool_p["A"] == 1
    g.move([a], "base", 0)
    settle(g)
    assert top in g.p[0].banish


@T.test
def coalesced_energy_rebuilds_runes_at_end_of_turn():
    g, _ = N()
    runes(g, 0, ["Fury"] * 2)
    g.p[0].rune_deck = [Rune("Mind", 0) for _ in range(6)]
    put(g, 0, "Coalesced Energy")
    end_turn(g)
    rs = g.p[0].runes
    assert len(rs) == 4 and all(r.domain == "Mind" for r in rs), rs


@T.test
def detonation_amplifier_doubles_deathknells_here():
    g, _ = N()
    put(g, 0, "Detonation Amplifier", 1)
    s = put(g, 0, "Hunting Dog", 1)                         # Deathknell: a Bird token
    g.kill([s], 0)
    settle(g)
    assert len([u for u in g.units(0) if u.cname == "Bird"]) == 2


@T.test
def detonation_amplifier_kill_to_draw():
    g, _ = N()
    R(g, 0)
    f = put(g, 0, "Recruit")
    deck_top(g, 0, [V3])
    n = len(g.p[0].hand)
    hand(g, 0, "Detonation Amplifier")
    play(g, 0, "Detonation Amplifier")
    assert f.zone != "board" and len(g.p[0].hand) == n + 1


@T.test
def drakehound_packmaster_plays_opponents_top_card():
    g, _ = N()
    R(g, 0, 4)
    top = deck_top(g, 1, [V4])[0]
    hand(g, 0, "Drakehound Packmaster")
    play(g, 0, "Drakehound Packmaster", base)
    assert top.zone == "board" and top.ctrl == 0 and top.owner == 1


@T.test
def firebrand_raider_readies_everything():
    g, _ = N()
    R(g, 0)
    e = put(g, 1, V3, ready=False)
    hand(g, 0, "Firebrand Raider")
    play(g, 0, "Firebrand Raider", base)
    assert not e.exhausted and not unit(g, "Firebrand Raider").exhausted
    hand(g, 0, V3)
    play(g, 0, V3, base)
    assert not unit(g, V3, 0).exhausted                     # other units enter ready
    e.exhausted = True
    g.tp = 1
    g.stage = "awaken"
    g.turn_step()
    assert e.exhausted                                      # no readying in the Awaken Phase


@T.test
def mordekaiser_twice_slain_survives_once():
    g, _ = N()
    m = put(g, 0, "Mordekaiser, Twice Slain", 1)
    f = put(g, 0, "Recruit")
    g.kill([m], 1)
    settle(g)
    assert m.zone == "board" and m.loc == "base" and m.exhausted and f.zone != "board"
    f2 = put(g, 0, "Recruit")
    g.kill([m], 1)
    assert m.zone == "trash" and f2.zone == "board"


@T.test
def ekko_revolutionary_banishes_and_returns_ready():
    g, _ = N()
    R(g, 0)
    e = put(g, 0, "Ekko, Revolutionary", 1, ready=False)
    hand(g, 0, V3)
    act(g, 0, "Ekko, Revolutionary")
    x = unit(g, "Ekko, Revolutionary")
    assert x.loc == "base" and not x.exhausted and not g.p[0].hand


@T.test
def whirling_twirligig_three_abilities():
    g, _ = N()
    w = put(g, 0, "Whirling Twirligig")
    act(g, 0, "Whirling Twirligig", i=0)
    assert any(x.cname == "Gold" for x in g.gear(0))
    w.exhausted = False
    v = put(g, 0, V3)
    act(g, 0, "Whirling Twirligig", lambda ch: ch.get("kill") == v.uid, i=2)
    assert v.zone == "trash" and unit(g, "Mech")


@T.test
def ziggs_short_fused_bonus_damage_on_gear_abilities():
    g, _ = N()
    put(g, 0, "Ziggs, Short-Fused")
    b = make_token(g, "Bomb", 0, 1, ready=True)
    e = put(g, 1, V3, 1, bf_control=False)
    g.kill([b], 0)
    settle(g)
    assert e.zone == "trash"                                # 2 + 1 Bonus Damage


# ====================================================================== Calm
@T.test
def blossoming_dreamroot_draws_on_combat_win_here():
    g, _ = N()
    put(g, 0, "Blossoming Dreamroot", 1, bf_control=False)
    a = put(g, 0, V5)
    put(g, 1, "Recruit", 1)
    g.bfs[1].ctrl = 1
    deck_top(g, 0, [V3, V3])
    n = len(g.p[0].hand)
    move(g, [a], 1)
    assert len(g.p[0].hand) >= n + 1


@T.test
def consult_the_heavens_shield_2_and_predict():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3)
    deck_top(g, 0, [V4])
    hand(g, 0, "Consult the Heavens")
    play(g, 0, "Consult the Heavens", lambda ch: ch["tg"] == (u.uid,))
    assert g.kw_value(u, "Shield") == 2


@T.test
def entrenched_boar_ready_but_cant_move():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Entrenched Boar")
    play(g, 0, "Entrenched Boar", lambda ch: ch.get("boar") and ch["loc"] == "base")
    b = unit(g, "Entrenched Boar")
    assert not b.exhausted and not g.can_move(b, 0, 0, True)


@T.test
def iridescent_avian_buffs_or_gives_2():
    g, _ = N()
    put(g, 0, "Iridescent Avian")
    u = put(g, 0, V3)
    act(g, 0, "Iridescent Avian", lambda ch: ch["tg"] == (u.uid,))
    assert u.buff == 1
    unit(g, "Iridescent Avian").exhausted = False
    act(g, 0, "Iridescent Avian", lambda ch: ch["tg"] == (u.uid,))
    assert g.might(u) == 6


@T.test
def ki_burst_exhausts_and_stuns():
    g, _ = N()
    R(g, 0)
    f = put(g, 0, V3)
    e = put(g, 1, V4)
    hand(g, 0, "Ki Burst")
    play(g, 0, "Ki Burst", lambda ch: ch["exh"] == f.uid and ch["tg"] == (e.uid,))
    assert f.exhausted and e.stunned


@T.test
def life_from_stone_draw_and_channel():
    g, _ = N()
    R(g, 0)
    f = put(g, 0, V3)
    deck_top(g, 0, [V4])
    n, r = len(g.p[0].hand), len(g.p[0].runes)
    hand(g, 0, "Life from Stone")
    play(g, 0, "Life from Stone")
    assert f.exhausted and len(g.p[0].hand) == n + 1 and len(g.p[0].runes) >= r


@T.test
def sanctum_conservator_readies_on_7_cost_card():
    g, _ = N()
    R(g, 0, 4)
    s = put(g, 0, "Sanctum Conservator", ready=False)
    hand(g, 0, V8)
    play(g, 0, V8, base)
    assert not s.exhausted


@T.test
def stoic_ferryman_moves_another_unit_home():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3, 1)
    hand(g, 0, "Stoic Ferryman")
    play(g, 0, "Stoic Ferryman", lambda ch: ch["loc"] == 1 and not ch.get("acc"))
    assert u.loc == "base" and g.kw_value(unit(g, "Stoic Ferryman"), "Shield") == 3


@T.test
def battle_preparations_show_gear_readies_runes():
    g, _ = N()
    R(g, 0, 1)
    for r in g.p[0].runes:
        r.exhausted = True
    g.p[0].runes[0].exhausted = False
    g.p[0].runes[1].exhausted = False
    x = put(g, 0, "Seal of Discord")
    u = put(g, 0, V3)
    hand(g, 0, "Battle Preparations")
    play(g, 0, "Battle Preparations", lambda ch: ch.get("show") == x.uid and ch["tg"] == (u.uid,))
    assert g.might(u) == 5 and sum(1 for r in g.p[0].runes if not r.exhausted) >= 2


@T.test
def command_attack_moves_deploy_gear_or_finds_gear():
    g, _ = N()
    R(g, 0)
    x = put(g, 0, "Blossoming Dreamroot", 0)
    hand(g, 0, "Command: Attack"); hand(g, 0, "Command: Attack")
    play(g, 0, "Command: Attack", lambda ch: ch.get("mode") == "move")
    assert x.loc == 1
    gtop = deck_top(g, 0, [V3, "Seal of Discord"])[1]
    play(g, 0, "Command: Attack", lambda ch: ch.get("mode") == "look")
    assert gtop in g.p[0].hand


@T.test
def hunker_down_readies_cant_move_draws():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3, ready=False)
    deck_top(g, 0, [V4])
    hand(g, 0, "Hunker Down")
    play(g, 0, "Hunker Down")
    assert not u.exhausted and not g.can_move(u, 0, 0, True) and len(g.p[0].hand) == 1


@T.test
def invigorating_bloom_plus_3_and_draw():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3)
    hand(g, 0, "Invigorating Bloom")
    play(g, 0, "Invigorating Bloom")
    assert g.might(u) == 6
    deck_top(g, 0, [V4])
    act(g, 0, "Invigorating Bloom")
    assert u.exhausted and len(g.p[0].hand) == 1


@T.test
def storm_raptor_draws_when_stunned_enemy_dies_here():
    g, _ = N()
    put(g, 0, "Storm Raptor", 1)
    e = put(g, 1, V3, 1, bf_control=False)
    e.stunned = True
    deck_top(g, 0, [V4])
    g.kill([e], 0)
    settle(g)
    assert len(g.p[0].hand) == 1 and g.kw_value(unit(g, "Storm Raptor"), "Deflect") == 1


@T.test
def watchful_curator_gives_gear_deflect():
    g, _ = N()
    put(g, 0, "Watchful Curator")
    x = put(g, 0, "Seal of Discord")
    assert g.kw_value(x, "Deflect") == 1


@T.test
def ahri_confident_grows_when_reducing_enemy_might_here():
    g, _ = N()
    a = put(g, 0, "Ahri, Confident")
    e = put(g, 1, V5, 1)
    move(g, [a], 1)                                         # Disarm: 5 -> 4, Ahri 4 -> 5 this turn
    assert e.zone == "trash" and a.zone == "board" and g.might(a) == 5


@T.test
def ephemeral_ward_plus_2_until_a_unit_is_played():
    g, _ = N()
    R(g, 0)
    w = put(g, 0, "Ephemeral Ward")
    u = put(g, 0, V3)
    assert g.might(u) == 5
    hand(g, 0, V3)
    play(g, 0, V3, base)
    assert w.zone == "trash" and g.might(u) == 3


@T.test
def heartening_soul_readies_legend_and_grants_ability():
    g, _ = N()
    R(g, 0)
    g.p[0].legend.exhausted = True
    hand(g, 0, "Heartening Soul")
    play(g, 0, "Heartening Soul", base)
    assert not g.p[0].legend.exhausted
    u = put(g, 0, V3)
    acts = [o for o in act_options(g, 0) if isinstance(o[1], tuple)]
    g.apply(next(o for o in acts if o[3].get("tg") == (u.uid,)))
    settle(g)
    assert g.might(u) == 6


@T.test
def hush_counters_spells_with_2_power():
    g, _ = N()
    R(g, 0); R(g, 1)
    u = put(g, 0, V3)
    hand(g, 0, "Avalanche")                                 # Power cost 2
    hand(g, 1, "Hush")
    g.apply(opt(g, 0, "Avalanche"))
    respond(g, 1, "Hush")
    assert u.damage == 0 and u.zone == "board"


@T.test
def orianna_exquisite_needs_a_friendly_gear_here():
    g, _ = N()
    o = put(g, 0, "Orianna, Exquisite", 1)
    assert not g.deals_combat_damage(o)
    put(g, 0, "Blossoming Dreamroot", 1)
    assert g.deals_combat_damage(o)


@T.test
def riven_broken_blade_modes_on_exhaust():
    g, _ = N()
    r = put(g, 0, "Riven, Broken Blade")
    r.exhausted = True
    settle(g)
    assert r.buff == 1
    r.exhausted = False
    e = put(g, 1, V3)
    r.exhausted = True
    settle(g)
    assert e.stunned and r.buff == 0                       # buff spent to stun


@T.test
def hunting_blind_pulls_an_enemy_and_shields():
    g, _ = N()
    R(g, 0)
    f = put(g, 0, V3, 1)
    e = put(g, 1, V3)
    hand(g, 0, "Hunting Blind")
    play(g, 0, "Hunting Blind", lambda ch: ch["loc"] == 1)
    assert any("combat damage: attackers 3 vs defenders 5" in l for l in g.lines)    # pulled in; Shield 2 here
    assert e.zone == "trash"


@T.test
def ksante_dauntless_doubles_shield():
    g, _ = N()
    k = put(g, 0, "K'Sante, Dauntless", 1)
    a = put(g, 1, V8)
    g.tp = 1
    move(g, [a], 1)
    assert any("attackers 8 vs defenders 6" in l for l in g.lines)    # 4 + Shield 1 doubled


@T.test
def resplendent_rostrum_draw_and_plus_3():
    g, _ = N()
    put(g, 0, "Resplendent Rostrum")
    u = put(g, 0, V3)
    deck_top(g, 0, [V4])
    act(g, 0, "Resplendent Rostrum")
    assert len(g.p[0].hand) == 1 and g.might(u) == 6


# ====================================================================== Mind
@T.test
def fanfare_draws_on_next_three_units():
    g, _ = N()
    R(g, 0, 6)
    deck_top(g, 0, [V3] * 4)
    hand(g, 0, "Fanfare")
    play(g, 0, "Fanfare")
    n = len(g.p[0].hand)
    for _ in range(4):
        hand(g, 0, "Recruit (NX)") if False else None
    for _ in range(4):
        make_token(g, "Recruit", 0, "base")
    settle(g)
    assert len(g.p[0].hand) == n + 3


@T.test
def high_note_minus_per_exhausted_unit():
    g, _ = N()
    R(g, 0)
    put(g, 0, V3, ready=False); put(g, 0, V3, ready=False)
    e = put(g, 1, V5)
    hand(g, 0, "High Note")
    play(g, 0, "High Note", lambda ch: ch["tg"] == (e.uid,))
    assert g.might(e) == 3


@T.test
def industrious_mechsmith_mech_here():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Industrious Mechsmith")
    play(g, 0, "Industrious Mechsmith", base)
    assert unit(g, "Mech").loc == "base"


@T.test
def mysterious_chronomancer_predicts_2():
    g, _ = N()
    R(g, 0)
    deck_top(g, 0, [V3, V4])
    hand(g, 0, "Mysterious Chronomancer")
    play(g, 0, "Mysterious Chronomancer", base)
    assert any("predicts 2" in l for l in g.lines)


@T.test
def parallel_convergence_returns_and_deals_5():
    g, _ = N()
    R(g, 0)
    f = put(g, 0, V3)
    e = put(g, 1, V5, 1)
    hand(g, 0, "Parallel Convergence")
    play(g, 0, "Parallel Convergence", lambda ch: ch["ret"] == f.uid and ch["tg"] == (e.uid,))
    assert f in g.p[0].hand and e.zone == "trash"


@T.test
def resource_extractor_gold_at_end_of_turn():
    g, _ = N()
    put(g, 0, "Resource Extractor", 1)
    put(g, 0, V3, 1)
    end_turn(g)
    assert any(x.cname == "Gold" for x in g.gear(0))


@T.test
def strigian_oracle_predicts_when_exhausted():
    g, _ = N()
    o = put(g, 0, "Strigian Oracle")
    deck_top(g, 0, [V3])
    move(g, [o], 0)
    assert any("predicts 1" in l for l in g.lines)


@T.test
def clanging_sentry_mechs_have_disarm():
    g, _ = N()
    put(g, 0, "Clanging Sentry")
    m = put(g, 0, "Mech")
    v = put(g, 0, V3)
    assert g.kw_value(m, "Disarm") == 1 and g.kw_value(v, "Disarm") == 0


@T.test
def improvised_projectile_damage_equals_gear_cost():
    g, _ = N()
    R(g, 0)
    x = put(g, 0, "Resplendent Rostrum")                    # Energy cost 8
    e = put(g, 1, V8, 1)
    hand(g, 0, "Improvised Projectile")
    play(g, 0, "Improvised Projectile", lambda ch: ch["tg"] == (x.uid, e.uid))
    assert e.zone == "trash"


@T.test
def piltovan_guidebot_base_might_becomes_card_cost():
    g, _ = N()
    R(g, 0, 4)
    p = put(g, 0, "Piltovan Guidebot")
    hand(g, 0, V8)
    play(g, 0, V8, base)
    assert g.might(p) == 7


@T.test
def riptide_mage_minus_3():
    g, _ = N()
    R(g, 0)
    e = put(g, 1, V5)
    hand(g, 0, "Riptide Mage")
    play(g, 0, "Riptide Mage", base)
    assert g.might(e) == 2


@T.test
def supernova_deals_10_draws_2():
    g, _ = N()
    R(g, 0, 4)
    e = put(g, 1, V8)
    deck_top(g, 0, [V3, V4])
    hand(g, 0, "Supernova")
    play(g, 0, "Supernova", lambda ch: ch["tg"] == (e.uid,))
    assert e.zone == "trash" and len(g.p[0].hand) == 2


@T.test
def the_empyrean_show_off_predict_power_then_draw_2():
    g, _ = N()
    R(g, 0, 4)
    hand(g, 0, "Avalanche")                                 # Power cost 2
    deck_top(g, 0, [V3, V4, V5])
    hand(g, 0, "The Empyrean")
    play(g, 0, "The Empyrean", lambda ch: ch.get("show") is not None and ch["loc"] == "base")
    assert len(g.p[0].hand) == 3 and any("predicts 2" in l for l in g.lines)


@T.test
def ekko_ingenious_must_return_a_unit():
    g, _ = N()
    R(g, 0)
    c = hand(g, 0, "Ekko, Ingenious")
    assert not [o for o in options_of(g, "play") if o[1] == c.uid]
    f = put(g, 0, V3)
    play(g, 0, "Ekko, Ingenious", lambda ch: ch["ret"] == f.uid and ch["loc"] == "base")
    assert f in g.p[0].hand and g.has_kw(unit(g, "Ekko, Ingenious"), "Ganking")


@T.test
def gravity_field_taxes_opponent_moves_and_draws_on_death():
    g, _ = N()
    x = put(g, 0, "Gravity Field", 1)
    e = put(g, 1, V3)
    g.tp = 1
    assert not [o for o in move_options(g, 1) if o[2] == 1]          # no runes: can't pay [1][A]
    R(g, 1, 1)
    assert [o for o in move_options(g, 1) if o[2] == 1]
    deck_top(g, 0, [V3])
    g.kill([x], 1)
    settle(g)
    assert len(g.p[0].hand) == 1


@T.test
def kaisa_rebel_gives_spells_reaction_in_showdown():
    g, _ = N()
    k = put(g, 0, "Kai'Sa, Rebel", 1)
    c = hand(g, 0, "Avalanche")
    from actions import card_kw
    assert not card_kw(g, 0, c, "Reaction")
    from game import Showdown
    g.sd = Showdown(1, True, 1)
    assert card_kw(g, 0, c, "Reaction")
    g.sd = None


@T.test
def scrutinizing_scope_draws_on_wrong_guess():
    g, _ = N(answers1={"scope_guess": lambda opts, ctx: next(o for o in opts if o.value == "Spell")})
    R(g, 0)
    put(g, 0, "Scrutinizing Scope")
    top = deck_top(g, 0, [V3])[0]
    act(g, 0, "Scrutinizing Scope")
    assert top in g.p[0].hand


@T.test
def spirit_of_the_forge_returns_gear_readies_and_grows():
    g, _ = N()
    R(g, 0)
    x = put(g, 0, "Resplendent Rostrum")
    hand(g, 0, "Spirit of the Forge")
    play(g, 0, "Spirit of the Forge", lambda ch: ch.get("ret_gear") == x.uid and ch["loc"] == "base")
    s = unit(g, "Spirit of the Forge")
    assert x in g.p[0].hand and not s.exhausted and g.might(s) == 12


@T.test
def clocktower_guardian_predict_and_deathknell_play():
    g, _ = N()
    R(g, 0, 4)
    hand(g, 0, "Clocktower Guardian")
    deck_top(g, 0, [V3, V3, V3])
    play(g, 0, "Clocktower Guardian", base)
    c = unit(g, "Clocktower Guardian")
    top = g.p[0].deck[0]
    g.kill([c], 1)
    settle(g)
    assert top.zone == "board" or top in g.p[0].banish


@T.test
def orianna_mechanical_marvel_gear_discount_once():
    g, _ = N()
    put(g, 0, "Orianna, Mechanical Marvel", 1)
    c = hand(g, 0, "Resplendent Rostrum")
    e, reqs = total_cost(g, 0, c, dict(loc="base"), "hand")
    assert e == 7 and reqs == []


@T.test
def zero_point_inflection_draws_and_explodes():
    g, _ = N()
    R(g, 0)
    put(g, 0, "Zero Point Inflection")
    deck_top(g, 0, [V3] * 20)
    for _ in range(8):
        hand(g, 0, V3)
    hand(g, 0, "Recruit") if False else None
    hand(g, 0, "Hunker Down")
    u = put(g, 0, V3, ready=False)
    play(g, 0, "Hunker Down")
    assert g.p[1].points >= 1 and not g.p[0].hand


# ====================================================================== Body
@T.test
def hold_em_discount_after_playing_to_a_battlefield():
    g, _ = N()
    c = hand(g, 0, "Hold 'Em")
    assert total_cost(g, 0, c, dict(), "hand")[0] == 3
    g.hist["to_bf"][0] = 1
    assert total_cost(g, 0, c, dict(), "hand")[0] == 1


@T.test
def nazuman_ntofo_gives_shield_2():
    g, _ = N()
    R(g, 0)
    x = put(g, 0, "Nazuman Ntofo")
    u = put(g, 0, V3)
    act(g, 0, "Nazuman Ntofo")
    assert g.kw_value(u, "Shield") == 2 and g.might(u) == 3


@T.test
def overloaded_traveler_plus_1_with_deploy_gear():
    g, _ = N()
    t = put(g, 0, "Overloaded Traveler", 1)
    assert g.might(t) == 2
    put(g, 0, "Blossoming Dreamroot", 1)
    assert g.might(t) == 3


@T.test
def pillaged_armory_buffs_on_play():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3)
    hand(g, 0, "Pillaged Armory")
    play(g, 0, "Pillaged Armory")
    assert u.buff == 1


@T.test
def raving_witness_draws_mighty_cards():
    g, _ = N()
    w = put(g, 0, "Raving Witness")
    top = deck_top(g, 0, [V5])[0]
    g.move([w], 0, 0)
    settle(g)
    assert top in g.p[0].hand


@T.test
def skystrike_squadron_cheaper_with_returned_gear():
    g, _ = N()
    x = put(g, 0, "Seal of Discord")
    c = hand(g, 0, "Skystrike Squadron")
    assert total_cost(g, 0, c, dict(loc="base", ret_gear=x.uid), "hand")[0] == 3


@T.test
def sneaky_zeebles_adds_on_conquer():
    g, _ = N()
    z = put(g, 0, "Sneaky Zeebles")
    move(g, [z], 1)
    assert g.p[0].pool_p["A"] == 1


@T.test
def wrath_of_the_freljord_enters_ready():
    g, _ = N()
    R(g, 0, 4)
    hand(g, 0, "Wrath of the Freljord")
    play(g, 0, "Wrath of the Freljord", base)
    w = unit(g, "Wrath of the Freljord")
    assert not w.exhausted and g.kw_value(w, "Shield") == 3


@T.test
def armored_tusker_deals_shield_on_move():
    g, _ = N()
    t = put(g, 0, "Armored Tusker")
    e = put(g, 1, V3, 1, bf_control=False)
    g.move([t], 0, 0)
    settle(g)
    assert e.zone == "trash"


@T.test
def avalanche_hits_bases_only():
    g, _ = N()
    R(g, 0)
    a, b = put(g, 1, V3), put(g, 1, V3, 1)
    hand(g, 0, "Avalanche")
    play(g, 0, "Avalanche")
    assert a.zone == "trash" and b.zone == "board"


@T.test
def bloodscent_swarm_cheaper_per_conquest():
    g, _ = N()
    c = hand(g, 0, "Bloodscent Swarm")
    g.hist["conq_bf"][0] = [0, 1]
    assert total_cost(g, 0, c, dict(loc="base"), "hand")[0] == 4


@T.test
def elnuk_herd_plus_3_and_recycled_from_facedown():
    g, _ = N()
    f = facedown(g, 0, "Elnuk Herd", 1)
    u = unit(g, V3, 0)
    play(g, 0, "Elnuk Herd", src="facedown")
    assert g.might(u) == 6 and f.zone == "deck"


@T.test
def stalwart_goatherd_kills_cheap_gear():
    g, _ = N()
    R(g, 0)
    x = put(g, 1, "Seal of Discord")
    hand(g, 0, "Stalwart Goatherd")
    play(g, 0, "Stalwart Goatherd", base)
    assert x.zone == "trash"


@T.test
def cataclysm_no_enemy_spells_and_moves():
    g, _ = N()
    R(g, 0); R(g, 1)
    f = put(g, 0, V3)
    put(g, 1, V3, 1)
    hand(g, 0, "Cataclysm")
    play(g, 0, "Cataclysm")
    c = hand(g, 1, "Hush")
    from actions import play_forbidden
    assert play_forbidden(g, 1, c, "hand")
    assert any("Shipyard Skulker#%d moves base->1" % f.uid in l for l in g.lines)


@T.test
def graves_blasting_through_hits_all_enemies_here():
    g, _ = N()
    gr = put(g, 0, "Graves, Blasting Through")
    a, b = put(g, 1, V3, 1), put(g, 1, V4, 1)
    move(g, [gr], 1)
    assert a.zone == "trash" and b.zone == "trash"


@T.test
def ksante_courageous_channels_on_hold():
    g, _ = N()
    k = put(g, 0, "K'Sante, Courageous", 1)
    n = len(g.p[0].runes)
    g.hold(0, g.bfs[1])
    settle(g)
    assert len(g.p[0].runes) == n + 1


@T.test
def portable_portal_ganking_here():
    g, _ = N()
    put(g, 0, "Portable Portal", 1)
    u = put(g, 0, V3, 1)
    assert g.has_kw(u, "Ganking")


@T.test
def staunch_survivor_keywords():
    g, _ = N()
    s = put(g, 0, "Staunch Survivor")
    assert g.kw_value(s, "Assault") == 2 and g.kw_value(s, "Shield") == 2


@T.test
def titanic_shrine_draws_for_the_biggest_unit():
    g, _ = N()
    R(g, 0, 4)
    put(g, 0, "Titanic Shrine")
    deck_top(g, 0, [V3])
    hand(g, 0, V8)
    play(g, 0, V8, base)
    assert len(g.p[0].hand) == 1 and gear(g, "Titanic Shrine").exhausted


@T.test
def evelynn_consuming_readies_on_facedown_play():
    g, _ = N()
    ev = put(g, 0, "Evelynn, Consuming", ready=False)
    facedown(g, 0, "Elnuk Herd", 1)
    play(g, 0, "Elnuk Herd", src="facedown")
    assert not ev.exhausted


@T.test
def jarvan_martial_paragon_deploy_gear_action_and_channel():
    g, _ = N()
    put(g, 0, "Jarvan IV, Martial Paragon")
    c = hand(g, 0, "Blossoming Dreamroot")
    from actions import eff_timing
    assert eff_timing(g, 0, c, g.impl(c)) == "action"


@T.test
def seraphic_wyvern_extra_exhausted_rune():
    g, _ = N()
    put(g, 0, "Seraphic Wyvern")
    g.p[0].runes = []
    g.p[0].rune_deck = [Rune("Body", 0) for _ in range(5)]
    g.channel(0, 2)
    assert len(g.p[0].runes) == 3 and sum(r.exhausted for r in g.p[0].runes) == 1


# ====================================================================== Chaos
@T.test
def disposal_expert_ready_when_alone():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Disposal Expert")
    play(g, 0, "Disposal Expert", base)
    d = unit(g, "Disposal Expert")
    assert not d.exhausted and g.kw_value(d, "Disarm") == 2


@T.test
def explosimian_three_bombs_on_hold():
    g, _ = N()
    put(g, 0, "Explosimian", 1)
    g.hold(0, g.bfs[1])
    settle(g)
    assert len(bombs(g, 0)) == 3 and len({b.loc for b in bombs(g, 0)}) == 1


@T.test
def momentary_companion_temporary_off_turn():
    g, _ = N()
    f = facedown(g, 0, "Momentary Companion", 1)
    g.tp = 1
    hand(g, 1, "Avalanche")
    R(g, 1)
    g.apply(opt(g, 1, "Avalanche"))
    respond(g, 0, "Momentary Companion", src="facedown")
    m = unit(g, "Momentary Companion")
    assert g.has_kw(m, "Temporary")


@T.test
def satchel_charge_moves_and_bombs():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3)
    hand(g, 0, "Satchel Charge")
    play(g, 0, "Satchel Charge", lambda ch: ch.get("dest") == 0)
    assert u.loc == 0 and len(bombs(g, 0)) == 1


@T.test
def siren_of_the_shallows_keywords():
    g, _ = N()
    s = put(g, 0, "Siren of the Shallows")
    assert g.has_kw(s, "Disarm") and g.has_kw(s, "Ganking")


@T.test
def sirens_song_moves_an_enemy_that_moved():
    g, _ = N()
    R(g, 0)
    e = put(g, 1, V3)
    c = hand(g, 0, "Siren's Song")
    assert not card_choices(g, 0, c, "hand", False, False)
    g.move([e], 1, 1)
    settle(g)
    play(g, 0, "Siren's Song", lambda ch: ch["dest"] == "base")
    assert e.loc == "base"


@T.test
def suave_negotiator_both_draw():
    g, _ = N()
    R(g, 0)
    deck_top(g, 0, [V3]); deck_top(g, 1, [V3])
    hand(g, 0, "Suave Negotiator")
    play(g, 0, "Suave Negotiator", base)
    assert len(g.p[0].hand) == 1 and len(g.p[1].hand) == 1


@T.test
def waters_wage_one_or_three_golds():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "The Waters' Wage")
    play(g, 0, "The Waters' Wage")
    assert len([x for x in g.gear(0) if x.cname == "Gold"]) == 1
    facedown(g, 0, "The Waters' Wage", 1)
    play(g, 0, "The Waters' Wage", src="facedown")
    golds = [x for x in g.gear(0) if x.cname == "Gold"]
    assert len(golds) == 4 and sum(not x.exhausted for x in golds) == 3


@T.test
def an_offer_kills_gear_and_gives_gold():
    g, _ = N()
    R(g, 0)
    x = put(g, 1, "Seal of Discord")
    hand(g, 0, "An Offer You Can't Refuse")
    play(g, 0, "An Offer You Can't Refuse")
    assert x.zone == "trash" and any(y.cname == "Gold" for y in g.gear(1))


@T.test
def demon_shade_moves_to_open_battlefield():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3)
    hand(g, 0, "Demon Shade")
    play(g, 0, "Demon Shade", lambda ch: ch["mode"] == "move")
    assert u.loc in (0, 1)


@T.test
def gutter_guardian_adds_when_showing_hidden():
    g, _ = N()
    R(g, 0)
    h = hand(g, 0, "Demon Shade")
    c = hand(g, 0, "Gutter Guardian")
    chs = card_choices(g, 0, c, "hand", False, False)
    assert any(ch.get("show") == h.uid for ch in chs)
    play(g, 0, "Gutter Guardian", lambda ch: ch.get("show") == h.uid and ch["loc"] == "base")
    assert g.p[0].pool_p["A"] == 1


@T.test
def pitiless_stalker_stuns():
    g, _ = N()
    put(g, 0, "Pitiless Stalker")
    e = put(g, 1, V3)
    act(g, 0, "Pitiless Stalker")
    assert e.stunned


@T.test
def reckless_innovator_gold_or_draw_when_exhausted():
    g, _ = N()
    r = put(g, 0, "Reckless Innovator")
    deck_top(g, 0, [V3])
    move(g, [r], 0)
    assert [x.cname for x in g.gear(0)] == ["Gold"] and not g.p[0].hand       # no [Chaos] to pay: the Gold mode


@T.test
def the_good_stuff_discard_draw_and_return_on_hold():
    g, _ = N()
    R(g, 0)
    hand(g, 0, V3)
    deck_top(g, 0, [V3, V3, V3])
    hand(g, 0, "The Good Stuff")
    play(g, 0, "The Good Stuff", lambda ch: ch["loc"] == 1)
    assert len(g.p[0].hand) == 3
    x = gear(g, "The Good Stuff")
    put(g, 0, V3, 1)
    g.hold(0, g.bfs[1])
    settle(g)
    assert x in g.p[0].hand


@T.test
def bedeviling_stranger_bounces_in_a_crowd():
    g, _ = N()
    put(g, 0, V3, 1); put(g, 0, V3, 1)
    e = put(g, 1, V3, 1, bf_control=False); put(g, 1, V3, 1, bf_control=False)
    s = put(g, 0, "Bedeviling Stranger", 1)               # its play trigger (a combat would start at once here)
    IMPL["Bedeviling Stranger"].on_play(g, s, dict(hidden_bf=1, src="facedown"))
    g.flush_triggers()
    while g.chain:
        g.resolve_top()
    assert len(g.units(1)) == 1 and len(g.p[1].hand) == 1


@T.test
def bushwhack_trap_extra_hide_slot():
    g, _ = N()
    put(g, 0, V3, 1)
    g.bfs[1].ctrl = 0
    assert hide_slots(g, g.bfs[1]) == 1
    put(g, 0, "Bushwhack Trap", 1)
    assert hide_slots(g, g.bfs[1]) == 2


@T.test
def evelynn_in_control_scores_with_facedown_here():
    g, _ = N()
    put(g, 0, "Evelynn, In Control", 1)
    facedown(g, 0, "Demon Shade", 1)
    pts = g.p[0].points
    g.hold(0, g.bfs[1])
    settle(g)
    assert g.p[0].points >= pts + 2                         # hold point + Evelynn


@T.test
def lulu_whimsical_base_might_4():
    g, _ = N()
    l = put(g, 0, "Lulu, Whimsical")
    e = put(g, 1, V8)
    g.move([l], 0, 0)
    settle(g)
    assert g.might(e) == 4


@T.test
def mechanical_bandit_draws_lowest():
    g, _ = N()
    put(g, 0, "Mechanical Bandit")
    mine = deck_top(g, 0, ["Recruit (NX)" if False else V3])[0]
    deck_top(g, 1, [V8])
    g.emit("beginning_start", pid=0)
    settle(g)
    assert mine in g.p[0].hand


@T.test
def ziggs_explosive_bomb_ability_and_defend():
    g, _ = N()
    R(g, 0)
    put(g, 0, "Ziggs, Explosive")
    act(g, 0, "Ziggs, Explosive")
    assert len(bombs(g, 0)) == 1


@T.test
def angler_wyrm_pulls_an_enemy():
    g, _ = N()
    put(g, 0, "Angler Wyrm", 1)
    e = put(g, 1, V3)
    act(g, 0, "Angler Wyrm")
    assert any("Shipyard Skulker#%d moves base->1" % e.uid in l for l in g.lines) and e.zone == "trash"


@T.test
def riven_deserter_grows_and_readies():
    g, _ = N()
    r = put(g, 0, "Riven, Deserter")
    a = put(g, 0, V3)
    b = put(g, 0, V3, ready=False)
    act(g, 0, "Riven, Deserter", lambda ch: ch["exh"] == a.uid and ch["tg"] == (b.uid,))
    assert a.exhausted and not b.exhausted and g.might(r) == 6
    assert not act_options(g, 0, "Riven, Deserter")


@T.test
def transpositionist_swaps_for_smaller_unit():
    g, _ = N()
    R(g, 0)
    put(g, 0, "Transpositionist")
    f = put(g, 0, V5)
    hand(g, 0, V3)
    act(g, 0, "Transpositionist", lambda ch: ch["tg"] == (f.uid,))
    assert f in g.p[0].hand and unit(g, V3, 0)


# ====================================================================== Order
@T.test
def concert_poster_pays_for_a_recruit():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Concert Poster")
    play(g, 0, "Concert Poster")
    assert unit(g, "Recruit", 0).loc == "base"


@T.test
def for_demacia_shields_friendly_units_there():
    g, _ = N()
    R(g, 0)
    a, b = put(g, 0, V3, 1), put(g, 0, V3)
    hand(g, 0, "For Demacia!")
    play(g, 0, "For Demacia!", lambda ch: ch["bf"] == 1)
    assert g.kw_value(a, "Shield") == 2 and g.kw_value(b, "Shield") == 0


@T.test
def herald_of_the_emperor_base_might_5():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, "Recruit")
    hand(g, 0, "Herald of the Emperor")
    play(g, 0, "Herald of the Emperor", base)
    assert g.might(u) == 5


@T.test
def hunting_dog_bird_on_death():
    g, _ = N()
    d = put(g, 0, "Hunting Dog")
    g.kill([d], 1)
    settle(g)
    assert unit(g, "Bird", 0)


@T.test
def meticulous_restorationist_returns_from_trash():
    g, _ = N()
    R(g, 0, 4)
    g.zone_listen = True
    m = trash(g, 0, "Meticulous Restorationist")
    hand(g, 0, V8)
    play(g, 0, V8, base)
    assert m.zone == "board"


@T.test
def one_two_punch_buffs_and_stuns():
    g, _ = N()
    R(g, 0)
    f, e = put(g, 0, V3), put(g, 1, V3)
    hand(g, 0, "One-Two Punch")
    play(g, 0, "One-Two Punch")
    assert f.buff == 1 and e.stunned


@T.test
def party_crasher_cheaper_per_exhausted_unit():
    g, _ = N()
    put(g, 0, V3, ready=False); put(g, 0, V3, ready=False)
    c = hand(g, 0, "Party Crasher")
    assert total_cost(g, 0, c, dict(loc="base"), "hand")[0] == 7


@T.test
def protectors_shield_quick_draw_tank_plus_1():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3)
    hand(g, 0, "Protector's Shield")
    play(g, 0, "Protector's Shield")
    assert g.has_kw(u, "Tank") and g.might(u) == 4


@T.test
def burn_away_banishes_and_burns():
    g, _ = N()
    R(g, 0)
    e = put(g, 1, V3)
    deck_top(g, 0, [V3, V3])
    hand(g, 0, "Burn Away")
    play(g, 0, "Burn Away", lambda ch: ch["tg"] == (e.uid,))
    assert e in g.p[1].banish and len(g.p[0].trash) >= 3


@T.test
def dune_devourer_cheaper_with_a_kill():
    g, _ = N()
    f = put(g, 0, V3)
    c = hand(g, 0, "Dune Devourer")
    assert total_cost(g, 0, c, dict(loc="base", kill=f.uid), "hand")[0] == 5


@T.test
def mournful_spirit_kills_something_here():
    g, _ = N()
    s = put(g, 0, "Mournful Spirit", 1)
    e = put(g, 1, V8, 1, bf_control=False)
    g.kill([s], 1)
    settle(g)
    assert e.zone == "trash"


@T.test
def rousing_display_four_recruits_when_showing_off():
    g, _ = N()
    R(g, 0)
    hand(g, 0, V8)
    hand(g, 0, "Rousing Display")
    play(g, 0, "Rousing Display", lambda ch: ch.get("show") is not None)
    assert len([u for u in g.units(0) if u.cname == "Recruit"]) == 4


@T.test
def steadfast_outrider_plus_2_with_company():
    g, _ = N()
    o = put(g, 0, "Steadfast Outrider", 1)
    put(g, 0, V3, 1)
    assert g.might(o) == 3
    put(g, 0, "Blossoming Dreamroot", 1)
    assert g.might(o) == 5


@T.test
def trifarian_captain_finds_cheap_unit():
    g, _ = N()
    R(g, 0)
    cheap = deck_top(g, 0, [V8, V3])[1]
    hand(g, 0, "Trifarian Captain")
    play(g, 0, "Trifarian Captain", base)
    assert cheap in g.p[0].hand


@T.test
def battlefield_armorer_returns_dead_gear():
    g, _ = N()
    R(g, 0)
    put(g, 0, "Battlefield Armorer")
    x = put(g, 0, "Blossoming Dreamroot", 1)
    g.kill([x], 1)
    settle(g)
    assert x in g.p[0].hand


@T.test
def demacian_illuminators_buff_once_per_death_event():
    g, _ = N()
    i = put(g, 0, "Demacian Illuminators")
    a, b = put(g, 0, "Recruit"), put(g, 0, "Recruit")
    g.kill([a, b], 1)
    settle(g)
    assert i.buff == 1


@T.test
def heimerdinger_academy_swaps_tokens_for_mechs():
    g, _ = N()
    put(g, 0, "Heimerdinger, Academy Instructor", 1)
    t = make_token(g, "Recruit", 0, "base")
    assert t.cname == "Mech"


@T.test
def jarvan_royal_heir_brings_a_friend():
    g, _ = N()
    j = put(g, 0, "Jarvan IV, Royal Heir")
    f = put(g, 0, V3)
    put(g, 1, V3, 1)
    move(g, [j], 1)
    assert f.loc == 1 and any("attackers 11 vs defenders 3" in l for l in g.lines)


@T.test
def raucous_anthem_readies_and_pumps():
    g, _ = N()
    R(g, 0, 4)
    u = put(g, 0, V3, ready=False)
    hand(g, 0, "Raucous Anthem")
    play(g, 0, "Raucous Anthem")
    assert not u.exhausted and g.might(u) == 6


@T.test
def remember_the_fallen_plays_from_trash():
    g, _ = N()
    R(g, 0, 4)
    a, b = trash(g, 0, V3), trash(g, 0, "Seal of Discord")
    hand(g, 0, "Remember the Fallen")
    play(g, 0, "Remember the Fallen")
    assert a.zone == "board" and b.zone == "board"


@T.test
def dockside_lockup_banishes_and_releases_on_hold():
    g, _ = N()
    R(g, 0, 4)
    e = put(g, 1, V4, 1)
    hand(g, 0, "Dockside Lock-Up")
    play(g, 0, "Dockside Lock-Up", lambda ch: ch["loc"] == 1)
    assert e in g.p[1].banish
    put(g, 0, V3, 1)
    g.bfs[1].ctrl = 0
    g.hold(0, g.bfs[1])
    settle(g)
    assert e.zone == "board" and e.ctrl == 0


@T.test
def mordekaiser_dominating_discards_to_kill():
    g, _ = N()
    R(g, 0, 4)
    e = put(g, 1, V8)
    hand(g, 0, V3)
    hand(g, 0, "Mordekaiser, Dominating")
    play(g, 0, "Mordekaiser, Dominating", base)
    assert e.zone == "trash"


@T.test
def seraphine_not_alone_recruits_and_counts_exhausted():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Seraphine, Not Alone")
    play(g, 0, "Seraphine, Not Alone", base)
    s = unit(g, "Seraphine, Not Alone")
    assert unit(g, "Recruit", 0)
    assert g.might(s) == 1 + sum(1 for u in g.units(0) if u is not s and u.exhausted)


# ====================================================================== legends, signatures
@T.test
def ekko_legend_reduces_next_unit_and_kills_it_at_end():
    g, _ = N()
    R(g, 0, 2)
    leg = legend(g, 0, "Ekko, Boy Who Shattered Time")
    act(g, 0, "Ekko, Boy Who Shattered Time", i=0)
    assert leg.empowered
    leg.exhausted = False
    act(g, 0, "Ekko, Boy Who Shattered Time", i=1)
    c = hand(g, 0, V4)
    assert total_cost(g, 0, c, dict(loc="base"), "hand") == (2, [])
    play(g, 0, V4, base)
    u = unit(g, V4, 0)
    end_turn(g)
    assert u.zone == "trash"


@T.test
def chronobreak_replays_units():
    g, _ = N()
    R(g, 0)
    u = put(g, 0, V3, 1)
    u.damage = 2
    hand(g, 0, "Chronobreak")
    play(g, 0, "Chronobreak", lambda ch: ch["x"] == 1)
    x = unit(g, V3, 0)
    assert x.damage == 0 and x.loc in ("base", 1)


@T.test
def ziggs_legend_bombs():
    g, _ = N()
    R(g, 0)
    legend(g, 0, "Ziggs, Hexplosives Expert")
    act(g, 0, "Ziggs, Hexplosives Expert")
    assert len(bombs(g, 0)) == 1


@T.test
def hexplosive_minefield_recreates_bombs_at_end():
    g, _ = N()
    R(g, 0)
    make_token(g, "Bomb", 0, 0); make_token(g, "Bomb", 0, 1)
    hand(g, 0, "Hexplosive Minefield")
    play(g, 0, "Hexplosive Minefield", lambda ch: len(ch["bombs"]) == 2)
    assert not bombs(g, 0)
    end_turn(g)
    assert len(bombs(g, 0)) == 2


@T.test
def mordekaiser_legend_adds_for_units_only():
    g, _ = N()
    legend(g, 0, "Mordekaiser, Iron Revenant")
    runes(g, 0, ["Fury"])
    t = trash(g, 0, V3)
    c = hand(g, 0, V4)
    assert g.can_pay(0, 0, [ANY] * 2, dict(kind="unit", card=c)) and not g.can_pay(0, 0, [ANY] * 2, dict(kind="spell"))
    g.pay(0, 0, [ANY] * 2, dict(kind="unit", card=c))
    assert t in g.p[0].banish


@T.test
def realm_of_death_mutual_damage():
    g, _ = N()
    R(g, 0)
    e, f = put(g, 1, V4), put(g, 0, V3)
    hand(g, 0, "Realm of Death")
    play(g, 0, "Realm of Death", lambda ch: ch["tg"] == (e.uid, f.uid))
    assert e.zone == "trash" and f.zone == "board" and f.damage == 2


@T.test
def orianna_legend_makes_a_unit_gear():
    g, _ = N()
    R(g, 0)
    legend(g, 0, "Orianna, Lady of Clockwork")
    x = put(g, 0, "Resplendent Rostrum")
    g.stage = "main_start"
    g.turn_step()
    settle(g)
    assert x.ugear and x in g.units(0) and g.might(x) == 8 and x in g.gear(0)


@T.test
def the_ball_draws_or_hits_on_play_and_move():
    g, _ = N()
    R(g, 0)
    put(g, 1, V3, 1); put(g, 1, "Recruit", 1)
    hand(g, 0, "The Ball")
    play(g, 0, "The Ball", lambda ch: ch["loc"] == 1)
    assert [u.cname for u in g.units(1)] == [V3] and unit(g, V3, 1).damage == 2     # mode "deal 2 to enemy units here"


@T.test
def ksante_legend_assault_equal_to_shield():
    g, _ = N()
    R(g, 0)
    legend(g, 0, "K'Sante, Pride of Nazumah")
    u = put(g, 0, "Wrath of the Freljord")
    act(g, 0, "K'Sante, Pride of Nazumah")
    assert g.kw_value(u, "Assault") == 3


@T.test
def ntofo_strikes_shield_then_damage():
    g, _ = N()
    R(g, 0)
    f, e = put(g, 0, "K'Sante, Courageous"), put(g, 1, V4)
    hand(g, 0, "Ntofo Strikes")
    play(g, 0, "Ntofo Strikes", lambda ch: ch["tg"] == (f.uid, e.uid))
    assert e.zone == "trash"                                # Shield 2 + 2 = 4


@T.test
def riven_legend_units_enter_ready_at_controlled_battlefield():
    g, _ = N()
    R(g, 0)
    legend(g, 0, "Riven, The Exile")
    put(g, 0, V3, 1)
    hand(g, 0, V4); hand(g, 0, V4)
    play(g, 0, V4, lambda ch: ch["loc"] == 1)
    play(g, 0, V4, lambda ch: ch["loc"] == "base")
    us = [u for u in g.units(0) if u.cname == V4]
    assert not [u for u in us if u.loc == 1][0].exhausted and [u for u in us if u.loc == "base"][0].exhausted


@T.test
def blade_of_the_exile_equip_by_exhausting():
    g, _ = N()
    put(g, 0, "Blade of the Exile")
    u, x = put(g, 0, V3), put(g, 0, V3)
    act(g, 0, "Blade of the Exile", lambda ch: ch.get("exh") == x.uid and ch["tg"] == (u.uid,))
    assert x.exhausted and g.might(u) == 6 and g.has_kw(u, "Ganking")


@T.test
def blade_of_the_exile_human_picks_the_exhausted_unit():
    """Table : le joueur choisit l'unité qu'il épuise (toutes proposées, une ligne distincte par unité)."""
    import train
    g, _ = N()
    g.every_choice = True                                    # joueur humain : tous les choix (actions.full_choices)
    put(g, 0, "Blade of the Exile")
    u = put(g, 0, V3)
    others = [put(g, 0, V4), put(g, 0, V5), put(g, 0, V8)]
    opts = [o for o in act_options(g, 0, "Blade of the Exile") if o[3]["tg"] == (u.uid,) and o[3].get("exh")]
    assert sorted(o[3]["exh"] for o in opts) == sorted(x.uid for x in others + [u])
    labels = [train._act_label(g, o).split("cible")[0] for o in opts]
    assert len(set(labels)) == len(labels) and any(V8 in s for s in labels), labels
    act(g, 0, "Blade of the Exile", lambda ch: ch.get("exh") == others[2].uid and ch["tg"] == (u.uid,))
    assert others[2].exhausted and not others[0].exhausted and not others[1].exhausted


@T.test
def seraphine_legend_discount_by_exhausting_three():
    g, _ = N()
    legend(g, 0, "Seraphine, Starry-Eyed Songstress")
    us = [put(g, 0, V3) for _ in range(3)]
    runes(g, 0, ["Order"] * 1)
    c = hand(g, 0, "Fanfare")                                # 3 energy
    chs = card_choices(g, 0, c, "hand", False, False)
    assert chs and all(ch.get("sera") for ch in chs)
    play(g, 0, "Fanfare")
    assert all(u.exhausted for u in us) and g.p[0].legend.exhausted


@T.test
def encore_two_modes():
    g, _ = N()
    R(g, 0)
    e = put(g, 1, "Recruit")
    hand(g, 0, "Encore")
    play(g, 0, "Encore", lambda ch: ch["modes"] == ("deal", "recruits") and ch["tg"] == (e.uid,))
    assert e.zone == "trash" and len([u for u in g.units(0) if u.cname == "Recruit"]) == 2


@T.test
def evelynn_legend_draws_on_facedown_play():
    g, _ = N()
    legend(g, 0, "Evelynn, Agony's Embrace")
    facedown(g, 0, "Demon Shade", 1)
    deck_top(g, 0, [V3])
    play(g, 0, "Demon Shade", src="facedown")
    assert len(g.p[0].hand) == 1


@T.test
def last_caress_any_battlefield_and_buff():
    g, _ = N()
    R(g, 0)
    put(g, 1, V3, 1)
    hand(g, 0, "Last Caress")
    play(g, 0, "Last Caress")
    c = hand(g, 0, V3)
    assert any(ch["loc"] == 1 for ch in card_choices(g, 0, c, "hand", False, False))
    play(g, 0, V3, lambda ch: ch["loc"] == 1)
    assert any(u.buff for u in g.units(0))


@T.test
def jarvan_legend_moves_to_deployed_gear():
    g, _ = N()
    R(g, 0)
    legend(g, 0, "Jarvan IV, Exemplar of Demacia")
    u = put(g, 0, V3)
    hand(g, 0, "Blossoming Dreamroot")
    play(g, 0, "Blossoming Dreamroot", lambda ch: ch["loc"] == 0)
    assert u.loc == 0 and g.p[0].legend.exhausted


@T.test
def demacian_standard_plus_2_and_return():
    g, _ = N()
    s = put(g, 0, "Demacian Standard", 1, bf_control=False)
    a = put(g, 0, V3)
    put(g, 1, V4, 1)
    move(g, [a], 1)
    assert any("attackers 5 vs defenders 4" in l for l in g.lines)          # +2 to the attacker


# ====================================================================== battlefields
@T.test
def bandle_scouts_academy_gold_on_hide():
    g, _ = new(bf0="Bandle Scouts' Academy", bf1="Test Field B")
    R(g, 0)
    put(g, 0, V3, 0)
    hand(g, 0, "Demon Shade")
    g.apply(next(o for o in options_of(g, "hide")))
    settle(g)
    assert any(x.cname == "Gold" for x in g.gear(0))


@T.test
def black_market_once_per_turn_each_player():
    g, _ = new(bf0="Black Market", bf1="Test Field B")
    R(g, 0)
    deck_top(g, 0, [V3, V3])
    from actions import abilities_of

    def market():
        return [o for o in act_options(g, 0) if isinstance(o[1], tuple)
                and "Black Market" in abilities_of(g, g.p[0].legend)[o[2]]["name"]]
    g.apply(market()[0])
    settle(g)
    assert len(g.p[0].hand) == 1 and not market()


@T.test
def black_rose_sanctum_recruit_on_first_death():
    g, _ = new(bf0="Black Rose Sanctum", bf1="Test Field B")
    u = put(g, 0, V3, 0)
    g.kill([u], 1)
    settle(g)
    assert unit(g, "Recruit", 0).loc == "base"


@T.test
def cosmic_vista_point_with_empty_rune_deck():
    g, _ = new(bf0="Cosmic Vista", bf1="Test Field B")
    put(g, 0, V3, 0)
    g.p[0].rune_deck = []
    g.hold(0, g.bfs[0])
    settle(g)
    assert g.p[0].points == 2


@T.test
def durand_memorial_tank_this_combat():
    g, _ = new(bf0="Durand Memorial", bf1="Test Field B")
    d = put(g, 1, V3, 0)
    a = put(g, 0, V8)
    move(g, [a], 0)
    assert any("Durand Memorial" in l for l in g.lines)


@T.test
def firelights_hideout_accelerate_after_conquest():
    g, _ = new(bf0="Firelights' Hideout", bf1="Test Field B")
    R(g, 0)
    u = put(g, 0, V3)
    move(g, [u], 0)
    c = hand(g, 0, V4)
    assert any(ch.get("acc") for ch in card_choices(g, 0, c, "hand", False, False))


@T.test
def hunters_circle_first_play_here_costs_1_less():
    g, _ = new(bf0="Hunters' Circle", bf1="Test Field B")
    put(g, 0, V3, 0)
    c = hand(g, 0, V4)
    assert total_cost(g, 0, c, dict(loc=0), "hand")[0] == 3
    assert total_cost(g, 0, c, dict(loc="base"), "hand")[0] == 4


@T.test
def packed_amphitheater_champions_plus_1():
    g, _ = new(bf0="Packed Amphitheater", bf1="Test Field B")
    c = put(g, 0, "Ahri, Confident", 0)
    v = put(g, 0, V3, 0)
    assert g.might(c) == 5 and g.might(v) == 3


@T.test
def rakelstake_predict_on_first_play_here():
    g, _ = new(bf0="Rakelstake", bf1="Test Field B")
    R(g, 0)
    put(g, 0, V3, 0)
    deck_top(g, 0, [V3])
    hand(g, 0, V4)
    play(g, 0, V4, lambda ch: ch["loc"] == 0)
    assert any("predicts 1" in l for l in g.lines)


@T.test
def ruined_monastery_discard_and_channel():
    g, _ = new(bf0="Ruined Monastery", bf1="Test Field B")
    u = put(g, 0, V3)
    hand(g, 0, V4)
    n = len(g.p[0].runes)
    move(g, [u], 0)
    assert not g.p[0].hand and len(g.p[0].runes) == n + 1


@T.test
def every_radiance_card_but_neeko_is_modelled():
    import csv
    from game import ROOT
    rad = [r["name"] for r in csv.DictReader(open(ROOT / "cards" / "cards_unique.csv", encoding="utf-8"))
           if r["set_code"] == "RAD"]
    missing = [n for n in rad if n not in IMPL]
    assert missing == ["Neeko, Blending In"], missing


if __name__ == "__main__":
    T.main()
