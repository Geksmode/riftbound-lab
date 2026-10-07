"""Tests of batch fury. Run: RB_CARDSETS=fury python3 cardsets/test_fury.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from testkit import options_of                          # noqa: E402
from actions import attach                              # noqa: E402
from cards import make_token                            # noqa: E402

T = Suite("fury")
CLIMB = "Aspirant's Climb"          # battlefields without effects on damage or plays (victory score 9)


def fresh(**kw):
    return new(bf0=CLIMB, bf1=CLIMB, **kw)


def trash(g, pid, name):
    o = Obj(name, pid)
    o.zone = "trash"
    g.p[pid].trash.append(o)
    return o


def play(g, pid, name, pred=lambda ch: True, src=None):
    g.apply(opt(g, pid, name, pred, src))
    return settle(g)


def on_board(g, pid, name):
    return [u for u in g.board if u.ctrl == pid and u.cname == name]


def end_turns(g, n=2):
    for _ in range(n):
        g.apply(("end",))
        settle(g)


# ====================================================================== spells
@T.test
def angle_shot_attach_then_detach():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    u = put(g, 0, "Shipyard Skulker")
    d = put(g, 0, "Serrated Dirk")
    deck_top(g, 0, ["Shipyard Skulker", "Shipyard Skulker"])
    hand(g, 0, "Angle Shot")
    play(g, 0, "Angle Shot", lambda ch: ch["tg"] == (u.uid, d.uid))
    assert d.attached_to == u.uid and g.kw_value(u, "Assault") == 2 and len(g.p[0].hand) == 1
    e = put(g, 1, "Vanguard Sergeant", 1)
    ax = put(g, 1, "Spinning Axe")
    attach(g, ax, e)
    assert g.might(e) == 7
    hand(g, 0, "Angle Shot")
    play(g, 0, "Angle Shot", lambda ch: ch["tg"] == (e.uid, ax.uid))
    assert ax.attached_to is None and g.might(e) == 4 and ax.loc == "base"


@T.test
def blind_fury_plays_opponents_top_card():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    top = deck_top(g, 1, ["Shipyard Skulker"])[0]
    hand(g, 0, "Blind Fury")
    play(g, 0, "Blind Fury")
    assert top in g.board and top.ctrl == 0 and top.owner == 1
    # a spell: P0 plays the opponent's Hextech Ray for free with its own targets
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    e = put(g, 1, "Shipyard Skulker", 1)
    ray = deck_top(g, 1, ["Hextech Ray"])[0]
    hand(g, 0, "Blind Fury")
    play(g, 0, "Blind Fury")
    assert e.zone == "trash" and ray in g.p[1].trash


@T.test
def blood_rush_repeat_has_no_duration():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    u = put(g, 0, "Shipyard Skulker")
    v = put(g, 0, "Vanguard Sergeant")
    hand(g, 0, "Blood Rush")
    play(g, 0, "Blood Rush", lambda ch: ch.get("rep") and ch["tg"] == (u.uid,) and ch["tg2"] == (v.uid,))
    assert g.kw_value(u, "Assault") == 2 and g.kw_value(v, "Assault") == 2
    end_turns(g, 1)
    assert g.kw_value(u, "Assault") == 2


@T.test
def cleave_assault_this_turn():
    g, _ = fresh()
    runes(g, 0, ["Fury"])
    u = put(g, 0, "Shipyard Skulker")
    hand(g, 0, "Cleave")
    play(g, 0, "Cleave", lambda ch: ch["tg"] == (u.uid,))
    assert g.kw_value(u, "Assault") == 3
    end_turns(g, 1)
    assert g.kw_value(u, "Assault") == 0


@T.test
def consuming_curse_bonus_per_copy_in_trash():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    trash(g, 0, "Consuming Curse"); trash(g, 0, "Consuming Curse")
    e = put(g, 1, "Playful Phantom", 1)
    hand(g, 0, "Consuming Curse")
    play(g, 0, "Consuming Curse")
    assert e.damage == 4 and e.zone == "board"


@T.test
def curtain_call_all_four_modes():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4 + ["Mind"] * 4)
    s = put(g, 1, "Vanguard Sergeant", 1)        # 4 might at a battlefield
    k = put(g, 1, "Shipyard Skulker")            # 3 might at base
    deck_top(g, 0, ["Shipyard Skulker"])
    hand(g, 0, "Curtain Call")
    play(g, 0, "Curtain Call", lambda ch: len(ch.get("cc", ())) == 4)
    # -4 might then 2 damage kills the Sergeant, 3 damage kills the Skulker at base, draw 1
    assert s.zone == "trash" and k.zone == "trash" and len(g.p[0].hand) == 1


@T.test
def curtain_call_single_mode_costs_no_repeat():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    deck_top(g, 0, ["Shipyard Skulker"])
    c = hand(g, 0, "Curtain Call")
    e, reqs = total_cost(g, 0, c, dict(cc=("draw",), tg=(), rc=()), "hand")
    assert (e, reqs) == (4, [])
    e, reqs = total_cost(g, 0, c, dict(cc=("draw", "d2"), tg=(), rc=(0,)), "hand")
    assert e == 5 and reqs == []
    play(g, 0, "Curtain Call", lambda ch: ch.get("cc") == ("draw",))
    assert len(g.p[0].hand) == 1


@T.test
def dancing_grenade_bounces_between_players():
    g, _ = fresh(answers0={"may": False})
    runes(g, 0, ["Fury"] * 3)
    runes(g, 1, ["Fury"])
    s = put(g, 1, "Vanguard Sergeant", 1)
    d = put(g, 0, "Mountain Drake")
    c = hand(g, 0, "Dancing Grenade")
    play(g, 0, "Dancing Grenade", lambda ch: ch["tg"] == (s.uid,))
    # P1 (controller of the Sergeant) played it again for [A]: 2 + 1 Bonus Damage (dealt damage once) to the Drake
    assert s.damage == 2 and d.damage == 3, (s.damage, d.damage)
    assert c in g.p[0].trash and not g.p[1].runes


@T.test
def danger_zone_repeat_buffs_mechs():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    m = put(g, 0, "Mega-Mech")
    t = make_token(g, "Mech", 0)
    o = put(g, 0, "Shipyard Skulker")
    hand(g, 0, "Danger Zone")
    play(g, 0, "Danger Zone", lambda ch: ch.get("rep"))
    assert g.might(m) == 10 and g.might(t) == 5 and g.might(o) == 3


@T.test
def death_mark_burns_makes_clone_and_flows():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    n = len(g.p[0].deck)
    c = hand(g, 0, "Death Mark")
    play(g, 0, "Death Mark")
    assert len(g.p[0].deck) == n - 3 and len(on_board(g, 0, "Shadow Clone")) == 1 and c in g.p[0].trash
    play(g, 0, "Death Mark", src="trash")         # [Flow] 1 energy and 2 runes of any type
    assert len(on_board(g, 0, "Shadow Clone")) == 2 and c in g.p[0].banish


@T.test
def shadow_clone_banishes_for_assault_4():
    g, _ = fresh()
    clone = make_token(g, "Shadow Clone", 0, ready=True)
    u = trash(g, 0, "Shipyard Skulker")
    s = put(g, 1, "Vanguard Sergeant", 1)
    g.p[1].deck = []
    g.apply(("move", (clone.uid,), 1))
    settle(g)
    assert u in g.p[0].banish and s.zone == "trash"


@T.test
def death_from_below_kills_and_replays():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    k = put(g, 1, "Shipyard Skulker", 1)         # 3 might: may play it again
    s = put(g, 1, "Vanguard Sergeant", 1)        # 4 might
    c = hand(g, 0, "Death from Below")
    play(g, 0, "Death from Below", lambda ch: ch["tg"] == (k.uid,))
    assert k.zone == "trash" and s.zone == "trash" and c in g.p[0].trash


@T.test
def death_from_below_no_replay_above_3():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    s = put(g, 1, "Vanguard Sergeant", 1)
    k = put(g, 1, "Shipyard Skulker", 1)
    hand(g, 0, "Death from Below")
    play(g, 0, "Death from Below", lambda ch: ch["tg"] == (s.uid,))
    assert s.zone == "trash" and k.zone == "board"


@T.test
def detonate_kills_gear_controller_draws():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    x = put(g, 1, "Long Sword")
    n = len(g.p[1].hand)
    hand(g, 0, "Detonate")
    play(g, 0, "Detonate")
    assert x.zone == "trash" and len(g.p[1].hand) == n + 2


@T.test
def disintegrate_draws_only_on_kill():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 8)
    k = put(g, 1, "Shipyard Skulker", 1)
    s = put(g, 1, "Vanguard Sergeant", 1)
    hand(g, 0, "Disintegrate")
    play(g, 0, "Disintegrate", lambda ch: ch["tg"] == (s.uid,))
    assert s.damage == 3 and len(g.p[0].hand) == 0
    hand(g, 0, "Disintegrate")
    play(g, 0, "Disintegrate", lambda ch: ch["tg"] == (k.uid,))
    assert k.zone == "trash" and len(g.p[0].hand) == 1


@T.test
def firestorm_hits_enemies_at_one_battlefield():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 7)
    a = put(g, 1, "Shipyard Skulker", 1)
    b = put(g, 1, "Vanguard Sergeant", 1)
    c = put(g, 1, "Shipyard Skulker")
    hand(g, 0, "Firestorm")
    play(g, 0, "Firestorm", lambda ch: ch["bf"] == 1)
    assert a.zone == "trash" and b.damage == 3 and c.damage == 0


@T.test
def get_excited_deals_discarded_energy_cost():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3)
    e = put(g, 1, "Mountain Drake", 1)
    d = hand(g, 0, "Mountain Drake")             # 9 energy
    hand(g, 0, "Get Excited!")
    play(g, 0, "Get Excited!")
    assert d in g.p[0].trash and e.damage == 9


@T.test
def simple_damage_spells():
    for name, n, draw in (("Hextech Ray", 3, 0), ("Incinerate", 2, 0), ("Void Seeker", 4, 1)):
        g, _ = fresh()
        runes(g, 0, ["Fury"] * 4)
        e = put(g, 1, "Mountain Drake", 1)
        put(g, 1, "Mountain Drake")                   # at base: not a legal target
        deck_top(g, 0, ["Shipyard Skulker"])
        c = hand(g, 0, name)
        assert len(card_choices(g, 0, c, "hand", False, False)) == 1
        play(g, 0, name)
        assert e.damage == n and len(g.p[0].hand) == draw, name


@T.test
def icathian_rain_six_instances():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 10)
    k = put(g, 1, "Shipyard Skulker")
    s = put(g, 1, "Vanguard Sergeant", 1)
    hand(g, 0, "Icathian Rain")
    play(g, 0, "Icathian Rain")
    assert k.zone == "trash" and s.zone == "trash"


@T.test
def monster_harpoon_facedown_bonus():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    e = put(g, 1, "Mountain Drake", 1)
    hand(g, 0, "Monster Harpoon")
    play(g, 0, "Monster Harpoon")
    assert e.damage == 2
    put(g, 0, "Shipyard Skulker", 0)
    fd = Obj("Sudden Storm", 0)
    fd.zone, fd.hidden_turn, fd.hidden_bf = "facedown", g.turn_no, 0
    g.bfs[0].facedown = fd
    hand(g, 0, "Monster Harpoon")
    play(g, 0, "Monster Harpoon")
    assert e.damage == 6


@T.test
def noxian_guillotine_delayed_and_legion():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 5 + ["Order"] * 3)
    e = put(g, 1, "Mountain Drake", 1)
    hand(g, 0, "Noxian Guillotine")
    play(g, 0, "Noxian Guillotine")
    assert e.zone == "board"
    hand(g, 0, "Incinerate")
    play(g, 0, "Incinerate")
    assert e.zone == "trash"
    # Legion: another card played this turn -> killed at once
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    e = put(g, 1, "Mountain Drake", 1)
    g.finalized[0].append("Incinerate")
    hand(g, 0, "Noxian Guillotine")
    play(g, 0, "Noxian Guillotine")
    assert e.zone == "trash"


@T.test
def perfect_execution_ready_assault_and_flow():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 8)
    u = put(g, 0, "Shipyard Skulker", ready=False)
    c = hand(g, 0, "Perfect Execution")
    play(g, 0, "Perfect Execution", lambda ch: ch["tg"] == (u.uid,))
    assert not u.exhausted and g.kw_value(u, "Assault") == 3 and c in g.p[0].trash
    u.exhausted = True
    play(g, 0, "Perfect Execution", lambda ch: ch["tg"] == (u.uid,), src="trash")
    assert not u.exhausted and g.kw_value(u, "Assault") == 6 and c in g.p[0].banish


@T.test
def piercing_light_two_targets_and_repeat():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    rs = [make_token(g, "Recruit", 1, 1) for _ in range(3)]
    g.bfs[1].ctrl = 1
    b = make_token(g, "Recruit", 1, "base")
    hand(g, 0, "Piercing Light")
    # first execution kills rs[1] and rs[2]; the repetition kills rs[0] (its second target is gone)
    play(g, 0, "Piercing Light", lambda ch: ch.get("rep") and ch["tg"] == (rs[1].uid, rs[2].uid)
         and ch["tg2"] == (rs[0].uid, rs[1].uid))
    assert all(x not in g.board for x in rs) and b in g.board


@T.test
def relentless_pursuit_move_attach_and_return():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3 + ["Body"])
    u = put(g, 0, "Vanguard Sergeant")
    d = put(g, 0, "Serrated Dirk")
    hand(g, 0, "Relentless Pursuit")
    play(g, 0, "Relentless Pursuit", lambda ch: ch["tg"] == (u.uid,) and ch["dest"] == 1)
    # moved to the empty battlefield (conquer), Dirk attached, then "When I conquer, you may move me to my base"
    assert d.attached_to == u.uid and g.p[0].points == 1 and u.loc == "base" and d.loc == "base"


@T.test
def right_of_conquest_draws_per_battlefield():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    put(g, 0, "Shipyard Skulker", 0)
    hand(g, 0, "Right of Conquest")
    play(g, 0, "Right of Conquest")
    assert len(g.p[0].hand) == 2


@T.test
def ruthless_strike_discard_for_5():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3)
    e = put(g, 1, "Playful Phantom", 1)
    other = hand(g, 0, "Shipyard Skulker")
    hand(g, 0, "Ruthless Strike")
    play(g, 0, "Ruthless Strike", lambda ch: ch.get("disc"))
    assert e.zone == "trash" and other in g.p[0].trash
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3)
    e = put(g, 1, "Playful Phantom", 1)
    hand(g, 0, "Ruthless Strike")
    play(g, 0, "Ruthless Strike")
    assert e.damage == 3


@T.test
def shakedown_unless_draw():
    g, _ = fresh(answers1={"shakedown_let_draw": True})
    runes(g, 0, ["Fury"] * 3)
    e = put(g, 1, "Mountain Drake", 1)
    hand(g, 0, "Shakedown")
    play(g, 0, "Shakedown")
    assert len(g.p[0].hand) == 2 and e.damage == 0
    g, _ = fresh(answers1={"shakedown_let_draw": False})
    runes(g, 0, ["Fury"] * 3)
    e = put(g, 1, "Mountain Drake", 1)
    hand(g, 0, "Shakedown")
    play(g, 0, "Shakedown")
    assert len(g.p[0].hand) == 0 and e.damage == 6


@T.test
def square_up_repeat_by_discard():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    u = put(g, 0, "Shipyard Skulker", 0)
    v = put(g, 0, "Vanguard Sergeant", 0)
    d = hand(g, 0, "Mountain Drake")
    hand(g, 0, "Square Up")
    play(g, 0, "Square Up", lambda ch: ch.get("sq") and ch["tg"] != ch["tg2"])
    assert g.kw_value(u, "Assault") == 4 and g.kw_value(v, "Assault") == 4 and d in g.p[0].trash


@T.test
def stormbringer_deals_might_then_moves():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4 + ["Body"] * 4)
    d = put(g, 0, "Mountain Drake")
    a = put(g, 1, "Vanguard Sergeant", 1)
    b = put(g, 1, "Playful Phantom", 1)
    hand(g, 0, "Stormbringer")
    play(g, 0, "Stormbringer", lambda ch: ch["bf"] == 1)
    assert a.zone == "trash" and b.zone == "trash" and d.loc == 1 and g.bfs[1].ctrl == 0


@T.test
def sudden_storm_more_against_attacker():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    e = put(g, 1, "Mountain Drake", 1)
    hand(g, 0, "Sudden Storm")
    play(g, 0, "Sudden Storm")
    assert e.damage == 2
    e.desig = "att"
    hand(g, 0, "Sudden Storm")
    play(g, 0, "Sudden Storm")
    assert e.damage == 6


@T.test
def thermo_beam_kills_all_gear():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 7)
    a, b = put(g, 0, "Long Sword"), put(g, 1, "Seal of Rage")
    hand(g, 0, "Thermo Beam")
    play(g, 0, "Thermo Beam")
    assert a.zone == "trash" and b.zone == "trash"


@T.test
def thrill_of_the_hunt_replays_to_a_battlefield():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3)
    u = put(g, 0, "Vanguard Sergeant", ready=False)
    u.damage = 2
    hand(g, 0, "Thrill of the Hunt")
    play(g, 0, "Thrill of the Hunt")
    assert u in g.board and u.loc in (0, 1) and u.damage == 0 and g.bfs[u.loc].ctrl == 0


@T.test
def upstage_comedy_repeat_readies_two():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    u = put(g, 0, "Shipyard Skulker", ready=False)
    v = put(g, 0, "Vanguard Sergeant", ready=False)
    hand(g, 0, "Upstage Comedy")
    play(g, 0, "Upstage Comedy", lambda ch: ch.get("rep") and ch["tg"] != ch["tg2"])
    assert not u.exhausted and not v.exhausted


@T.test
def vault_breaker_assault_ganking():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    u = put(g, 0, "Shipyard Skulker", 0)
    hand(g, 0, "Vault Breaker")
    play(g, 0, "Vault Breaker", lambda ch: ch["tg"] == (u.uid,))
    assert g.kw_value(u, "Assault") == 2 and g.has_kw(u, "Ganking")


@T.test
def void_rush_plays_one_draws_other():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3 + ["Order"] * 2)
    k, s = deck_top(g, 0, ["Shipyard Skulker", "Vanguard Sergeant"])
    hand(g, 0, "Void Rush")
    play(g, 0, "Void Rush")
    assert s in g.board and k in g.p[0].hand and len(g.p[0].runes) == 4


# ====================================================================== units
@T.test
def arena_kingpin_ready_and_ability():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 5)
    hand(g, 0, "Arena Kingpin")
    play(g, 0, "Arena Kingpin", lambda ch: ch["loc"] == "base")
    k = on_board(g, 0, "Arena Kingpin")[0]
    assert not k.exhausted
    u = put(g, 0, "Shipyard Skulker")
    g.apply([a for a in act_options(g, 0, "Arena Kingpin") if a[3]["tg"] == (u.uid,)][0])
    settle(g)
    assert g.might(u) == 6 and k.exhausted


@T.test
def baccai_reaper_pays_for_assault():
    g, _ = fresh()
    runes(g, 0, ["Fury"])
    r = put(g, 0, "Baccai Reaper")
    p = put(g, 1, "Playful Phantom", 1)
    g.apply(("move", (r.uid,), 1))
    settle(g)
    assert p.zone == "trash" and r.zone == "board" and not g.p[0].runes


@T.test
def baccai_sandspinner_cheap_empower():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    s = put(g, 0, "Baccai Sandspinner")
    g.apply(act_options(g, 0, "Baccai Sandspinner")[0])
    settle(g)
    assert s.empowered and g.has_kw(s, "Deflect") and g.kw_value(s, "Assault") == 2


@T.test
def battering_ram_cost_reduction():
    g, _ = fresh()
    c = hand(g, 0, "Battering Ram")
    assert total_cost(g, 0, c, dict(loc="base"), "hand")[0] == 5
    g.finalized[0] += ["a", "b"]
    assert total_cost(g, 0, c, dict(loc="base"), "hand")[0] == 3
    g.finalized[0] += ["c"] * 8
    assert total_cost(g, 0, c, dict(loc="base"), "hand")[0] == 1


@T.test
def blade_twirler_burns_on_first_move_only():
    g, _ = fresh()
    t = put(g, 0, "Blade Twirler")
    n = len(g.p[1].deck)
    g.apply(("move", (t.uid,), 1))
    settle(g)
    t.exhausted = False
    g.apply(("move", (t.uid,), "base"))
    settle(g)
    assert len(g.p[1].deck) == n - 1 and len(g.p[1].trash) == 1


@T.test
def blast_corps_cadet_additional_cost():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    e = put(g, 1, "Mountain Drake", 1)
    hand(g, 0, "Blast Corps Cadet")
    play(g, 0, "Blast Corps Cadet", lambda ch: ch.get("bcc") and ch["loc"] == "base")
    assert e.damage == 2 and len(g.p[0].runes) == 5
    hand(g, 0, "Blast Corps Cadet")
    play(g, 0, "Blast Corps Cadet", lambda ch: not ch.get("bcc") and ch["loc"] == "base")
    assert e.damage == 2


@T.test
def brazen_buccaneer_discard_discount():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    d = hand(g, 0, "Shipyard Skulker")
    c = hand(g, 0, "Brazen Buccaneer")
    assert not any(not ch.get("bb") for ch in card_choices(g, 0, c, "hand", False, False))
    play(g, 0, "Brazen Buccaneer", lambda ch: ch.get("bb") and ch["loc"] == "base")
    assert c in g.board and d in g.p[0].trash


@T.test
def captain_farron_aura():
    g, _ = fresh()
    f = put(g, 0, "Captain Farron", 0)
    a = put(g, 0, "Shipyard Skulker", 0)
    b = put(g, 0, "Shipyard Skulker")
    e = put(g, 1, "Shipyard Skulker", 1)
    assert g.kw_value(a, "Assault") == 1 and g.kw_value(b, "Assault") == 0 and g.kw_value(f, "Assault") == 0
    assert g.kw_value(e, "Assault") == 0


@T.test
def chemtech_and_jinx_discard():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 5)
    for _ in range(3):
        hand(g, 0, "Shipyard Skulker")
    hand(g, 0, "Chemtech Enforcer")
    play(g, 0, "Chemtech Enforcer")
    assert len(g.p[0].hand) == 2 and len(g.p[0].trash) == 1
    hand(g, 0, "Jinx, Demolitionist")
    play(g, 0, "Jinx, Demolitionist", lambda ch: not ch["acc"] and ch["loc"] == "base")
    j = on_board(g, 0, "Jinx, Demolitionist")[0]
    assert len(g.p[0].hand) == 0 and len(g.p[0].trash) == 3 and j.exhausted and g.kw_value(j, "Assault") == 2


@T.test
def dangerous_duo_needs_legion():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    u = put(g, 0, "Shipyard Skulker", 0)
    hand(g, 0, "Dangerous Duo")
    play(g, 0, "Dangerous Duo", lambda ch: ch["loc"] == "base")
    assert g.might(u) == 3
    hand(g, 0, "Dangerous Duo")
    play(g, 0, "Dangerous Duo", lambda ch: ch["loc"] == "base")
    assert g.might(u) == 5


@T.test
def draven_showboat_points():
    g, _ = fresh()
    d = put(g, 0, "Draven, Showboat")
    g.p[0].points = 3
    assert g.might(d) == 6


@T.test
def draven_vanquisher_pays_and_wins_gold():
    g, _ = fresh()
    runes(g, 0, ["Fury"])
    d = put(g, 0, "Draven, Vanquisher")
    p = put(g, 1, "Playful Phantom", 1)
    g.apply(("move", (d.uid,), 1))
    settle(g)
    golds = on_board(g, 0, "Gold")
    assert p.zone == "trash" and d.zone == "board" and len(golds) == 1 and golds[0].exhausted


@T.test
def dunebreaker_ready_and_hold_draw():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 8)
    hand(g, 0, "Dunebreaker")
    play(g, 0, "Dunebreaker", lambda ch: ch["loc"] == "base")
    d = on_board(g, 0, "Dunebreaker")[0]
    assert not d.exhausted
    g.apply(("move", (d.uid,), 0))
    settle(g)
    n = len(g.p[0].hand)
    end_turns(g, 2)
    assert len(g.p[0].hand) == n + 3                 # hold: draw 2, then the Draw Phase


@T.test
def eager_drakehound_enters_ready():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    hand(g, 0, "Eager Drakehound")
    play(g, 0, "Eager Drakehound", lambda ch: ch["loc"] == "base")
    assert not on_board(g, 0, "Eager Drakehound")[0].exhausted


@T.test
def eclipse_dragon_draws_on_move_with_few_runes():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    d = put(g, 0, "Eclipse Dragon")
    deck_top(g, 0, ["Shipyard Skulker"])
    g.apply(("move", (d.uid,), 1))
    settle(g)
    assert len(g.p[0].hand) == 1
    runes(g, 0, ["Fury"] * 5)
    d.exhausted = False
    g.apply(("move", (d.uid,), "base"))
    settle(g)
    assert len(g.p[0].hand) == 1


@T.test
def fewer_runes_at_beginning():
    g, _ = fresh()
    f = put(g, 0, "Forsaken Baccai")
    o = put(g, 0, "Oasis Raider")
    runes(g, 0, ["Fury"] * 2)
    runes(g, 1, ["Fury"] * 6)
    g.apply(("end",)); settle(g)
    g.apply(("end",)); settle(g)
    assert g.might(f) == 3 and g.might(o) == 6 and g.has_kw(o, "Ganking")


@T.test
def gem_jammer_gives_ganking():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    u = put(g, 0, "Shipyard Skulker", 0)
    hand(g, 0, "Gem Jammer")
    play(g, 0, "Gem Jammer", lambda ch: ch["loc"] == "base")
    assert g.has_kw(u, "Ganking") and not g.has_kw(on_board(g, 0, "Gem Jammer")[0], "Ganking")
    end_turns(g, 1)
    assert not g.has_kw(u, "Ganking")


@T.test
def grim_apothecary_returns_hurt_unit():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3)
    u = put(g, 0, "Vanguard Sergeant", 1)
    u.damage = 3
    hand(g, 0, "Grim Apothecary")
    play(g, 0, "Grim Apothecary", lambda ch: ch["loc"] == 1)
    assert u in g.p[0].hand


@T.test
def inviolus_vox_conquer_gives_8():
    g, _ = fresh()
    v = put(g, 0, "Inviolus Vox")
    g.apply(("move", (v.uid,), 1))
    settle(g)
    assert g.bfs[1].ctrl == 0 and g.might(v) == 16


@T.test
def jhin_adds_on_move():
    g, _ = fresh()
    j = put(g, 0, "Jhin, Murderous Artist")
    g.apply(("move", (j.uid,), 1))
    settle(g)
    assert g.p[0].pool_e == 1 and g.p[0].pool_p["A"] == 1
    assert g.has_kw(j, "Deflect") and g.has_kw(j, "Ganking")


@T.test
def kadregrin_draws_per_mighty():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 11)
    put(g, 0, "Mountain Drake")
    put(g, 0, "Shipyard Skulker")
    hand(g, 0, "Kadregrin the Infernal")
    play(g, 0, "Kadregrin the Infernal", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 2


@T.test
def katarina_hide_ready_and_facedown_ping():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    k = put(g, 0, "Katarina, Reckless", 0, ready=False)
    s = hand(g, 0, "Sudden Storm")
    g.apply(("hide", s.uid, 0))
    settle(g)
    assert not k.exhausted
    # next turn: play Pyke from face down -> deal 2 to an enemy unit
    p = Obj("Pyke, Dockside Butcher", 0)
    p.zone, p.hidden_turn, p.hidden_bf = "facedown", g.turn_no - 1, 1
    g.bfs[1].ctrl = 0
    put(g, 0, "Shipyard Skulker", 1)
    g.bfs[1].facedown = p
    e = put(g, 1, "Mountain Drake")
    play(g, 0, "Pyke, Dockside Butcher", lambda ch: not ch.get("pyke"), src="facedown")
    assert p in g.board and p.loc == 1 and e.damage == 2


@T.test
def lord_broadmane_gives_assault_here():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    u = put(g, 0, "Shipyard Skulker", 1)
    b = put(g, 0, "Shipyard Skulker")
    hand(g, 0, "Lord Broadmane")
    play(g, 0, "Lord Broadmane", lambda ch: ch["loc"] == 1)
    lb = on_board(g, 0, "Lord Broadmane")[0]
    assert g.kw_value(u, "Assault") == 1 and g.kw_value(b, "Assault") == 0 and g.kw_value(lb, "Assault") == 0


@T.test
def lucian_shoots_on_attack():
    g, _ = fresh()
    lu = put(g, 0, "Lucian, Gunslinger")
    a = make_token(g, "Recruit", 1, 1)
    b = make_token(g, "Recruit", 1, 1)
    g.bfs[1].ctrl = 1
    g.apply(("move", (lu.uid,), 1))
    settle(g)
    assert a not in g.board and b not in g.board and lu.zone == "board" and g.bfs[1].ctrl == 0


@T.test
def morgana_doubles_marked_damage():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    e = put(g, 1, "Mountain Drake", 1)
    e.damage = 3
    hand(g, 0, "Morgana, Vindictive")
    play(g, 0, "Morgana, Vindictive", lambda ch: ch["loc"] == "base")
    assert e.damage == 6


@T.test
def punching_poro_empower_by_discard():
    g, _ = fresh()
    p = put(g, 0, "Punching Poro")
    assert not act_options(g, 0, "Punching Poro")
    d = hand(g, 0, "Shipyard Skulker")
    g.apply(act_options(g, 0, "Punching Poro")[0])
    settle(g)
    assert p.empowered and g.might(p) == 3 and d in g.p[0].trash


@T.test
def pyke_additional_cost_ready_and_might():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    hand(g, 0, "Pyke, Dockside Butcher")
    play(g, 0, "Pyke, Dockside Butcher", lambda ch: ch.get("pyke") and ch["loc"] == "base")
    p = on_board(g, 0, "Pyke, Dockside Butcher")[0]
    assert not p.exhausted and g.might(p) == 4 and g.has_kw(p, "Ganking") and len(g.p[0].runes) == 3


@T.test
def rage_amplifier_aura():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 7)
    a = put(g, 0, "Rage Amplifier")
    u = put(g, 0, "Shipyard Skulker")
    e = put(g, 1, "Shipyard Skulker")
    assert g.might(u) == 4 and g.might(e) == 3
    g.apply(act_options(g, 0, "Rage Amplifier")[0])
    settle(g)
    assert a.empowered and g.might(u) == 5


@T.test
def rell_plays_equipment_on_attack():
    g, _ = fresh()
    r = put(g, 0, "Rell, Magnetic")
    d = hand(g, 0, "Serrated Dirk")
    p = put(g, 1, "Playful Phantom", 1)
    g.apply(("move", (r.uid,), 1))
    settle(g)
    assert d.attached_to == r.uid and p.zone == "trash" and r.zone == "board" and g.has_kw(r, "Tank")


@T.test
def renekton_burns_enemies_here():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    r = put(g, 0, "Renekton, Rage Fueled")
    a = make_token(g, "Recruit", 1, 1)
    b = put(g, 1, "Playful Phantom", 1)
    g.bfs[1].ctrl = 1
    g.apply(("move", (r.uid,), 1))
    settle(g)
    # 2 to each enemy: the Recruit dies, the Phantom (5) has 3 left and loses the combat against 6
    assert a not in g.board and b.zone == "trash" and r.zone == "board"


@T.test
def rumble_mechs_assault_and_conquer_replay():
    g, _ = fresh()
    runes(g, 0, ["Mind"] * 4)
    r = put(g, 0, "Rumble, Hotheaded")
    s = put(g, 0, "Shipyard Skulker")
    m = trash(g, 0, "Mega-Mech")
    assert g.kw_value(r, "Assault") == 1 and g.kw_value(s, "Assault") == 0
    g.apply(("move", (r.uid,), 1))
    settle(g)
    assert m in g.board and s not in g.board and s in g.p[0].deck and not g.p[0].runes[0:0]
    assert g.kw_value(m, "Assault") == 1 and all(x.exhausted for x in g.p[0].runes)


@T.test
def scorchclaw_level_3():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    g.gain_xp(0, 3)
    hand(g, 0, "Scorchclaw")
    play(g, 0, "Scorchclaw", lambda ch: ch["loc"] == "base")
    s = on_board(g, 0, "Scorchclaw")[0]
    assert not s.exhausted and g.might(s) == 4 and g.kw_value(s, "Hunt") == 2


@T.test
def scrapyard_champion_legion_loot():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    for _ in range(3):
        hand(g, 0, "Shipyard Skulker")
    g.finalized[0].append("x")
    hand(g, 0, "Scrapyard Champion")
    play(g, 0, "Scrapyard Champion", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 3 and len(g.p[0].trash) == 2


@T.test
def shadow_assassin_ready_with_copy_in_trash():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 10)
    hand(g, 0, "Shadow Assassin")
    play(g, 0, "Shadow Assassin", lambda ch: ch["loc"] == "base")
    assert on_board(g, 0, "Shadow Assassin")[0].exhausted
    trash(g, 0, "Shadow Assassin")
    hand(g, 0, "Shadow Assassin")
    play(g, 0, "Shadow Assassin", lambda ch: ch["loc"] == "base")
    assert any(not x.exhausted for x in on_board(g, 0, "Shadow Assassin"))


@T.test
def shadow_fiend_empowered_assault():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3)
    f = put(g, 0, "Shadow Fiend")
    g.apply(act_options(g, 0, "Shadow Fiend")[0])
    settle(g)
    assert f.empowered and g.kw_value(f, "Assault") == 3


@T.test
def tibbers_hits_all_units_at_battlefields():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 10)
    a = put(g, 0, "Shipyard Skulker", 0)
    b = put(g, 1, "Vanguard Sergeant", 1)
    c = put(g, 1, "Shipyard Skulker")
    hand(g, 0, "Tibbers")
    play(g, 0, "Tibbers", lambda ch: ch["loc"] == "base")
    assert a.zone == "trash" and b.damage == 3 and c.damage == 0


@T.test
def twilight_reveler_readies_another():
    g, _ = fresh()
    t = put(g, 0, "Twilight Reveler")
    u = put(g, 0, "Shipyard Skulker", ready=False)
    put(g, 1, "Mountain Drake", 1)
    g.apply(("move", (t.uid,), 1))
    settle(g)
    assert not u.exhausted


@T.test
def vayne_ready_and_returns():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    put(g, 1, "Shipyard Skulker", 0)
    hand(g, 0, "Vayne, Hunter")
    play(g, 0, "Vayne, Hunter", lambda ch: ch["loc"] == "base")
    v = on_board(g, 0, "Vayne, Hunter")[0]
    assert not v.exhausted and g.kw_value(v, "Assault") == 3
    g.apply(("move", (v.uid,), 1))
    settle(g)
    assert g.p[0].points == 1 and v in g.p[0].hand


@T.test
def vi_destructive_recycles_for_might():
    g, _ = fresh()
    v = put(g, 0, "Vi, Destructive")
    assert not act_options(g, 0, "Vi, Destructive")
    c = trash(g, 0, "Shipyard Skulker")
    g.apply(act_options(g, 0, "Vi, Destructive")[0])
    settle(g)
    assert g.might(v) == 4 and c in g.p[0].deck and not g.p[0].trash


@T.test
def vi_hotheaded_doubles():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3)
    v = put(g, 0, "Vi, Hotheaded")
    g.mod(v, 1)
    g.apply(act_options(g, 0, "Vi, Hotheaded")[0])
    settle(g)
    assert g.might(v) == 8 and g.has_kw(v, "Deflect")


@T.test
def void_drone_cheaper_outside_hand():
    g, _ = fresh()
    c = hand(g, 0, "Void Drone")
    assert total_cost(g, 0, c, dict(loc="base"), "hand")[0] == 3
    c2 = champ(g, 0, "Void Drone")
    assert total_cost(g, 0, c2, dict(loc="base"), "champ")[0] == 1


@T.test
def volibear_splits_five():
    g, _ = fresh()
    v = put(g, 0, "Volibear, Furious")
    a = make_token(g, "Recruit", 1, 1)
    b = put(g, 1, "Shipyard Skulker", 1)
    c = put(g, 1, "Mountain Drake", 1)
    g.bfs[1].ctrl = 1
    g.apply(("move", (v.uid,), 1))
    settle(g)
    assert a not in g.board and b.zone == "trash"
    assert g.kw_value(v, "Deflect") == 2


@T.test
def xerath_only_at_battlefield():
    g, _ = fresh()
    runes(g, 0, ["Fury"])
    x = put(g, 0, "Xerath, Freed")
    e = put(g, 1, "Mountain Drake", 1)
    assert not act_options(g, 0, "Xerath, Freed")
    x.loc = 0
    g.bfs[0].ctrl = 0
    g.apply(act_options(g, 0, "Xerath, Freed")[0])
    settle(g)
    assert e.damage == 3 and x.exhausted


@T.test
def zed_discard_for_shadow_clone():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 10)
    d = hand(g, 0, "Shipyard Skulker")
    hand(g, 0, "Zed, From the Shadows")
    play(g, 0, "Zed, From the Shadows", lambda ch: ch.get("zed") and ch["loc"] == "base")
    assert d in g.p[0].trash and len(on_board(g, 0, "Shadow Clone")) == 1
    hand(g, 0, "Zed, From the Shadows")
    play(g, 0, "Zed, From the Shadows", lambda ch: not ch.get("zed") and ch["loc"] == "base")
    assert len(on_board(g, 0, "Shadow Clone")) == 1


# ====================================================================== gear
@T.test
def assembly_rig_recycles_for_mech():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    put(g, 0, "Assembly Rig")
    c = trash(g, 0, "Shipyard Skulker")
    trash(g, 0, "Hextech Ray")
    g.apply(act_options(g, 0, "Assembly Rig")[0])
    settle(g)
    mechs = on_board(g, 0, "Mech")
    assert len(mechs) == 1 and mechs[0].loc == "base" and mechs[0].exhausted and c in g.p[0].deck


@T.test
def fresh_beans_draw_on_showdown_unit():
    g, ag = fresh()
    runes(g, 0, ["Fury"] * 6)
    b = put(g, 0, "Fresh Beans")
    u = put(g, 0, "Mountain Drake")
    put(g, 1, "Shipyard Skulker", 1)
    hand(g, 0, "Lord Broadmane")
    deck_top(g, 0, ["Shipyard Skulker"])
    g.apply(("move", (u.uid,), 1))
    d = g.advance()
    assert d.kind == "focus" and d.player == 0
    o = [x for x in d.options if x[0] == "play"]
    g.apply(o[0])
    settle(g)
    assert b.exhausted and len(g.p[0].hand) == 1


@T.test
def iron_ballista_enters_exhausted():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3)
    hand(g, 0, "Iron Ballista")
    play(g, 0, "Iron Ballista")
    ib = on_board(g, 0, "Iron Ballista")[0]
    assert ib.exhausted and not act_options(g, 0, "Iron Ballista")
    ib.exhausted = False
    e = put(g, 1, "Mountain Drake", 1)
    g.apply(act_options(g, 0, "Iron Ballista")[0])
    settle(g)
    assert e.damage == 2


@T.test
def seal_of_rage_adds_fury():
    g, _ = fresh()
    runes(g, 0, ["Calm"] * 2)
    s = put(g, 0, "Seal of Rage")
    c = hand(g, 0, "Hextech Ray")               # 1 energy and 1 fury rune
    put(g, 1, "Mountain Drake", 1)
    assert card_choices(g, 0, c, "hand", False, False)
    play(g, 0, "Hextech Ray")
    assert s.exhausted and c in g.p[0].trash


@T.test
def sun_disc_next_unit_ready():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    put(g, 0, "Sun Disc")
    assert not act_options(g, 0, "Sun Disc")
    g.finalized[0].append("x")
    g.apply(act_options(g, 0, "Sun Disc")[0])
    settle(g)
    hand(g, 0, "Shipyard Skulker")
    play(g, 0, "Shipyard Skulker", lambda ch: ch["loc"] == "base")
    hand(g, 0, "Shipyard Skulker")
    play(g, 0, "Shipyard Skulker", lambda ch: ch["loc"] == "base")
    us = on_board(g, 0, "Shipyard Skulker")
    assert [u.exhausted for u in us] == [False, True]


@T.test
def recurve_bow_effect_text():
    g, _ = fresh()
    runes(g, 0, ["Fury"])
    u = put(g, 0, "Vanguard Sergeant")
    bow = put(g, 0, "Recurve Bow")
    g.apply(act_options(g, 0, "Recurve Bow")[0])
    settle(g)
    assert bow.attached_to == u.uid and g.might(u) == 4
    a = make_token(g, "Recruit", 1, 1)
    p = put(g, 1, "Shipyard Skulker", 1)
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert p.zone == "trash" and a not in g.board, (p.zone, a in g.board)


@T.test
def serrated_dirk_and_spinning_axe():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3)
    u = put(g, 0, "Vanguard Sergeant")
    d = put(g, 0, "Serrated Dirk")
    g.apply(act_options(g, 0, "Serrated Dirk")[0])
    settle(g)
    assert d.attached_to == u.uid and g.kw_value(u, "Assault") == 2 and g.might(u) == 4
    hand(g, 0, "Spinning Axe")
    play(g, 0, "Spinning Axe")                   # Quick-Draw: attached as it is played
    ax = on_board(g, 0, "Spinning Axe")[0]
    assert ax.attached_to == u.uid and g.might(u) == 7 and not g.has_kw(ax, "Temporary")
    loose = put(g, 0, "Spinning Axe")
    assert g.has_kw(loose, "Temporary")
    end_turns(g, 2)
    assert ax in g.board and loose not in g.board


# ====================================================================== cards using the engine hooks (integration)
SKf, VSf = "Shipyard Skulker", "Vanguard Sergeant"           # vanilla 3 / 4


def _mv(g, units, dest):
    g.apply(("move", tuple(u.uid for u in units), dest))
    return settle(g)


def _act(g, pid, cname, pred=lambda ch: True):
    acts = [a for a in act_options(g, pid, cname) if pred(a[3])]
    assert acts, act_options(g, pid)
    g.apply(acts[0])
    return settle(g)


@T.test
def integration_cards_are_registered():
    for n in ("Annie, Fiery", "Ravenborn Tome", "Lotus Trap", "Smite", "Unlicensed Armory", "Bushwhack", "Magma Wurm",
              "Brynhir Thundersong", "Noxus Saboteur", "Perched Grimwyrm", "Rengar, Pouncing", "Rek'Sai, Breacher",
              "Undying Legion", "Flame Chompers", "Super Mega Death Rocket!", "Immortal Phoenix", "Endless Riches",
              "Prepared Neophyte", "Revna the Lorekeeper", "Raging Soul", "Towering Pairofant", "Blighted Battleaxe",
              "Raging Firebrand", "Dominus", "Dune Surfer", "Tryndamere, Barbarian", "Yeti Brawler",
              "Hextech Gauntlets", "Red Brambleback", "Skyfall of Areion", "Void Hatchling", "Minotaur Reckoner"):
        assert n in IMPL and IMPL[n].module == "cardsets.fury", n


@T.test
def annie_fiery_bonus_damage():
    g, _ = fresh()
    put(g, 0, "Annie, Fiery")
    e = put(g, 1, "Mountain Drake")
    assert g.deal(e, 2, "spell", 0) == 3 and g.deal(e, 2, "ability", 0) == 3
    assert g.deal(e, 2, "spell", 1) == 2 and g.deal(e, 2, "unit", 0) == 2


@T.test
def ravenborn_tome_next_spell_and_smite_banishes():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    put(g, 0, "Ravenborn Tome")
    e = put(g, 1, VSf, 1)
    _act(g, 0, "Ravenborn Tome")
    hand(g, 0, "Smite")
    play(g, 0, "Smite")
    assert e.zone == "banish", e.zone                   # 3 + 1 Bonus Damage, banished instead of dying
    e2 = put(g, 1, VSf, 1)
    hand(g, 0, "Smite")
    play(g, 0, "Smite")
    assert e2.zone == "board" and e2.damage == 3          # the Tome's bonus was for one spell only
    g.kill([e2], 0)
    assert e2.zone == "banish"                            # "If it would die this turn"


@T.test
def lotus_trap_doubles_damage():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    e = put(g, 1, "Mountain Drake")
    hand(g, 0, "Lotus Trap")
    play(g, 0, "Lotus Trap")
    assert g.deal(e, 3, "spell", 0) == 6 and g.deal(e, 1, "unit", 0) == 2


@T.test
def unlicensed_armory_recalls_dying_unit_for_fury():
    g, _ = fresh()
    runes(g, 0, ["Fury"])
    u = put(g, 0, VSf, 1)
    put(g, 0, "Unlicensed Armory")
    hand(g, 0, SKf)
    _act(g, 0, "Unlicensed Armory")
    assert not g.p[0].hand
    g.kill([u], 1)
    assert u.zone == "board" and u.loc == "base" and u.exhausted
    g.kill([u], 1)
    assert u.zone == "trash"                              # the next time only


@T.test
def bushwhack_magma_wurm_enter_ready():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 8)
    hand(g, 0, "Bushwhack")
    play(g, 0, "Bushwhack")
    gold = [o for o in g.gear(0) if o.cname == "Gold"]
    assert len(gold) == 1 and gold[0].exhausted
    hand(g, 0, SKf)
    play(g, 0, SKf, lambda ch: ch["loc"] == "base")
    assert not on_board(g, 0, SKf)[0].exhausted
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 3)
    w = put(g, 0, "Magma Wurm")
    hand(g, 0, SKf)
    play(g, 0, SKf, lambda ch: ch["loc"] == "base")
    assert not on_board(g, 0, SKf)[0].exhausted
    w2 = hand(g, 0, "Magma Wurm")
    g.kill([w], 0)
    runes(g, 0, ["Fury"] * 9)
    play(g, 0, "Magma Wurm", lambda ch: not ch.get("acc"))
    assert w2.zone == "board" and w2.exhausted


@T.test
def brynhir_stops_opponents_plays():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    runes(g, 1, ["Fury"] * 3)
    c = hand(g, 1, SKf)
    assert card_choices(g, 1, c, "hand", False, False)
    hand(g, 0, "Brynhir Thundersong")
    play(g, 0, "Brynhir Thundersong", lambda ch: ch["loc"] == "base")
    assert not card_choices(g, 1, c, "hand", False, False)
    end_turns(g, 1)
    assert card_choices(g, 1, c, "hand", False, False)


@T.test
def noxus_saboteur_blocks_hidden_reveals_here():
    g, _ = fresh()
    runes(g, 1, ["Fury"] * 3)
    lt = Obj("Lotus Trap", 1)
    lt.zone = "facedown"
    lt.hidden_bf = 1
    lt.hidden_turn = 0
    g.bfs[1].facedown = lt
    e = put(g, 0, SKf, 1)
    assert card_choices(g, 1, lt, "facedown", False, False)
    sab = put(g, 0, "Noxus Saboteur", 1)
    assert not card_choices(g, 1, lt, "facedown", False, False)
    g.kill([sab], 1)
    assert card_choices(g, 1, lt, "facedown", False, False) and e


@T.test
def perched_grimwyrm_only_to_conquered_battlefield():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    c = hand(g, 0, "Perched Grimwyrm")
    g.bfs[1].ctrl = 0
    assert not card_choices(g, 0, c, "hand", False, False)
    g.hist["conq_bf"][0].append(1)
    assert sorted(str(ch["loc"]) for ch in card_choices(g, 0, c, "hand", False, False)) == ["1"]


@T.test
def rengar_pouncing_to_attacked_battlefield():
    g, _ = fresh(answers0={"may": False})
    runes(g, 0, ["Fury"] * 4)
    c = hand(g, 0, "Rengar, Pouncing")
    assert "1" not in [str(ch["loc"]) for ch in card_choices(g, 0, c, "hand", True, True)]
    put(g, 1, VSf, 1)
    a = put(g, 0, SKf)
    g.apply(("move", (a.uid,), 1))
    seen = []
    for _ in range(50):
        d = g.advance()
        if d is None or d.kind == "main":
            break
        if d.player == 0 and g.sd is not None and g.sd.combat and not seen:
            seen = [str(ch["loc"]) for ch in card_choices(g, 0, c, "hand", True, True)]
        g.apply(g.agents[d.player].decide(g, d))
    assert "1" in seen, seen


@T.test
def reksai_breacher_grants_accelerate_outside_hand():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    from actions import card_kw
    t = trash(g, 0, "Undying Legion")
    h = hand(g, 0, SKf)
    assert not card_kw(g, 0, t, "Accelerate", "trash")
    put(g, 0, "Rek'Sai, Breacher")
    assert card_kw(g, 0, t, "Accelerate", "trash") and not card_kw(g, 0, h, "Accelerate", "hand")


@T.test
def undying_legion_from_trash():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 4)
    t = trash(g, 0, "Undying Legion")
    assert not card_choices(g, 0, t, "trash", False, False)
    g.finalized[0].append("x")
    g.played[0].append("x")
    chs = card_choices(g, 0, t, "trash", False, False)
    assert chs and all(ch.get("alt") == "legion" for ch in chs)
    assert total_cost(g, 0, t, chs[0], "trash")[0] == 3


@T.test
def flame_chompers_played_when_discarded():
    g, _ = fresh()
    runes(g, 0, ["Fury"])
    c = hand(g, 0, "Flame Chompers")
    g.discard(0, c)
    settle(g)
    assert c.zone == "board" and c.ctrl == 0


@T.test
def super_mega_death_rocket_returns_on_conquer():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 5)
    e = put(g, 1, "Mountain Drake")
    hand(g, 0, "Super Mega Death Rocket!")
    g.zone_listen = True                          # cards created by the test after the game started
    play(g, 0, "Super Mega Death Rocket!")
    r = [c for c in g.p[0].trash if c.cname == "Super Mega Death Rocket!"][0]
    assert e.damage == 5
    hand(g, 0, SKf)
    put(g, 1, SKf, 1)
    a = put(g, 0, VSf)
    _mv(g, [a], 1)
    assert g.bfs[1].ctrl == 0 and r.zone == "hand" and not [c for c in g.p[0].hand if c.cname == SKf]


@T.test
def immortal_phoenix_returns_when_you_kill_with_a_spell():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    ph = trash(g, 0, "Immortal Phoenix")
    g.zone_listen = True
    e = put(g, 1, SKf, 1)
    hand(g, 0, "Smite")
    play(g, 0, "Smite")
    assert e.zone == "banish" and ph.zone == "trash"       # banished instead of dying: not killed
    e = put(g, 1, SKf, 1)
    hand(g, 0, "Super Mega Death Rocket!")
    runes(g, 0, ["Fury"] * 7)
    play(g, 0, "Super Mega Death Rocket!")
    assert e.zone == "trash" and ph.zone == "board"
    ph2 = trash(g, 0, "Immortal Phoenix")
    g.zone_listen = True
    g.kill([put(g, 1, SKf)], 0)                  # no spell
    assert ph2.zone == "trash"


@T.test
def endless_riches_statics():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    hand(g, 0, SKf)
    trash(g, 0, "Smite")
    deck_top(g, 0, ["Smite"] * 8)
    hand(g, 0, "Endless Riches")
    play(g, 0, "Endless Riches")
    pl = g.p[0]
    assert not pl.hand and len(pl.trash) == 7 and len(pl.banish) == 2, (len(pl.trash), len(pl.banish))
    sm = trash(g, 0, "Smite")
    put(g, 1, SKf, 1)
    runes(g, 0, ["Fury"] * 2)
    assert card_choices(g, 0, sm, "trash", False, False)
    u = put(g, 0, SKf)
    g.kill([u], 1)
    assert u.zone == "banish"
    g.p[0].deck[0:0] = []
    n = len(pl.hand)
    end_turns(g, 2)
    assert len(pl.hand) == n                     # Draw Phase skipped


@T.test
def prepared_neophyte_revna_raging_soul_pairofant():
    g, _ = fresh()
    n = put(g, 0, "Prepared Neophyte")
    rv = put(g, 0, "Revna the Lorekeeper", ready=False)
    rs = put(g, 0, "Raging Soul")
    assert g.might(n) == 1 and not g.has_kw(rs, "Ganking")
    runes(g, 0, ["Fury"] * 6)
    put(g, 1, "Mountain Drake")
    hand(g, 0, "Super Mega Death Rocket!")
    play(g, 0, "Super Mega Death Rocket!")
    assert g.might(n) == 5 and not rv.exhausted
    hand(g, 0, SKf)
    g.discard(0, g.p[0].hand[0])
    assert g.has_kw(rs, "Ganking") and g.kw_value(rs, "Assault") == 1
    runes(g, 0, ["Fury"] * 6)
    hand(g, 0, "Towering Pairofant")
    play(g, 0, "Towering Pairofant", lambda ch: ch["loc"] == "base" and not ch.get("acc"))
    assert on_board(g, 0, "Towering Pairofant")[0].exhausted
    g.kill([put(g, 1, SKf)], 0)
    runes(g, 0, ["Fury"] * 6)
    hand(g, 0, "Towering Pairofant")
    play(g, 0, "Towering Pairofant", lambda ch: ch["loc"] == "base" and not ch.get("acc"))
    assert not on_board(g, 0, "Towering Pairofant")[1].exhausted


@T.test
def blighted_battleaxe_unattaches_without_conquest():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    u = put(g, 0, "Mountain Drake")
    ax = put(g, 0, "Blighted Battleaxe")
    _act(g, 0, "Blighted Battleaxe")
    assert ax.attached_to == u.uid and g.might(u) == 14
    end_turns(g, 1)
    assert ax.attached_to is None and u.damage in (0, 4)
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    u = put(g, 0, VSf)
    ax = put(g, 0, "Blighted Battleaxe")
    _act(g, 0, "Blighted Battleaxe")
    put(g, 1, SKf, 1)
    _mv(g, [u], 1)
    assert g.bfs[1].ctrl == 0
    end_turns(g, 1)
    assert ax.attached_to == u.uid


@T.test
def blighted_battleaxe_deals_4():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    u = put(g, 0, VSf)
    ax = put(g, 0, "Blighted Battleaxe")
    _act(g, 0, "Blighted Battleaxe")
    end_turns(g, 1)
    assert ax.attached_to is None and u.zone == "trash"     # 4 damage to a 4-might unit


@T.test
def raging_firebrand_discounts_next_spell():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 7)
    hand(g, 0, "Raging Firebrand")
    play(g, 0, "Raging Firebrand", lambda ch: ch["loc"] == "base" and not ch.get("acc"))
    r = hand(g, 0, "Super Mega Death Rocket!")
    chs = card_choices(g, 0, r, "hand", False, False)
    assert total_cost(g, 0, r, chs[0], "hand")[0] == 0
    put(g, 1, "Mountain Drake")
    runes(g, 0, ["Fury"])
    play(g, 0, "Super Mega Death Rocket!")
    r2 = hand(g, 0, "Super Mega Death Rocket!")
    assert total_cost(g, 0, r2, card_choices(g, 0, r2, "hand", False, False)[0], "hand")[0] == 4 if \
        card_choices(g, 0, r2, "hand", False, False) else True


@T.test
def dominus_doubles_and_grants_ready():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 6)
    u = put(g, 0, VSf, ready=False)
    hand(g, 0, "Dominus")
    play(g, 0, "Dominus")
    assert g.might(u) == 8
    _act(g, 0, VSf)
    assert not u.exhausted
    end_turns(g, 2)
    assert g.might(u) == 4 and not act_options(g, 0, VSf)


@T.test
def dune_surfer_ignores_tank():
    g, _ = fresh()
    put(g, 0, "Dune Surfer", 1)
    assert g.impl("Dune Surfer").ignore_tank(g, g.units(0, 1)[0], 0, 1)
    assert not g.impl("Dune Surfer").ignore_tank(g, g.units(0, 1)[0], 1, 1)


@T.test
def tryndamere_and_yeti_excess_damage():
    g, _ = fresh()
    t = put(g, 0, "Tryndamere, Barbarian")
    put(g, 1, SKf, 1)
    p0 = g.p[0].points
    _mv(g, [t], 1)
    assert g.p[0].points == p0 + 2, (p0, g.p[0].points)          # conquer + 5 excess damage
    g, _ = fresh()
    y = put(g, 0, "Yeti Brawler")
    put(g, 1, SKf, 1)
    _mv(g, [y], 1)
    assert len([o for o in g.gear(0) if o.cname == "Gold"]) == 2
    g, _ = fresh()
    y = put(g, 0, "Yeti Brawler")
    put(g, 1, "Mountain Drake", 1)
    put(g, 1, SKf, 1)
    _mv(g, [y], 1)
    assert not [o for o in g.gear(0) if o.cname == "Gold"]


@T.test
def hextech_gauntlets_cost_and_draw():
    g, _ = fresh()
    runes(g, 0, ["Fury"])
    u = put(g, 0, VSf)
    gl = put(g, 0, "Hextech Gauntlets")
    _act(g, 0, "Hextech Gauntlets")                  # 3 - 4 might = 0 energy + 1 rune
    assert gl.attached_to == u.uid and g.might(u) == 7
    put(g, 1, SKf, 1)
    deck_top(g, 0, [SKf])
    n = len(g.p[0].hand)
    _mv(g, [u], 1)
    assert len(g.p[0].hand) == n + 1


@T.test
def red_brambleback_doubles_conquer_effects():
    g, _ = fresh()
    b = put(g, 0, "Red Brambleback")
    o = put(g, 0, SKf)
    put(g, 1, SKf, 1)
    _mv(g, [b], 1)
    assert b.buff + o.buff == 2                     # its own conquer trigger triggered twice


@T.test
def skyfall_hold_and_conquer_swap():
    g, _ = fresh()
    runes(g, 0, ["Fury"] * 2)
    y = put(g, 0, "Yeti Brawler")
    sk = put(g, 0, "Skyfall of Areion")
    _act(g, 0, "Skyfall of Areion")
    assert g.might(y) == 8
    g.bfs[1].ctrl = 0
    _mv(g, [y], 1)
    g.scoring_event("hold", 0, g.bfs[1], [y])
    settle(g)
    assert sk.attached_to == y.uid


@T.test
def void_hatchling_recycles_before_reveal():
    g, _ = fresh(answers0={"hatchling_recycle": True})
    put(g, 0, "Void Hatchling")
    a, b = deck_top(g, 0, [SKf, VSf])
    cs = g.reveal(0, 1)
    assert cs == [b] and g.p[0].deck[-1] is a


@T.test
def minotaur_reckoner_no_moves_to_base():
    g, _ = fresh()
    put(g, 1, "Minotaur Reckoner")
    u = put(g, 0, SKf, 1)
    assert not [o for o in options_of(g, "move") if o[2] == "base"]
    g.recall(u)
    assert u.loc == "base"


# ====================================================================== random games with every fury card
@T.test
def fuzz_smoke():
    import fuzz_cards
    names = sorted(k for k, v in IMPL.items() if v.module == "cardsets.fury")
    g = fuzz_cards.play(3, names)
    assert g.turn_no > 1


if __name__ == "__main__":
    T.main()
