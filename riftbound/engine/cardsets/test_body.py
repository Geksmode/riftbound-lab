"""Tests of batch body. Run: RB_CARDSETS=body python3 cardsets/test_body.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from cards import make_token, attach                    # noqa: E402
from game import Item                                   # noqa: E402

T = Suite("body")


def N(**kw):
    """A test game with two battlefields without effects on damage/movement (Forgotten Monument)."""
    return new(bf0="Forgotten Monument", bf1="Forgotten Monument", **kw)


def play(g, pid, name, pred=lambda ch: True, src=None):
    g.apply(opt(g, pid, name, pred, src))
    return settle(g)


def move(g, units, dest):
    g.apply(("move", tuple(u.uid for u in units), dest))
    return settle(g)


def act(g, pid, cname, pred=lambda ch: True, i=None):
    acts = [a for a in act_options(g, pid, cname) if pred(a[3]) and (i is None or a[2] == i)]
    assert acts, act_options(g, pid)
    g.apply(acts[0])
    return settle(g)


# ====================================================================== units
@T.test
def anivia_deals_3_to_enemies_here_on_attack():
    g, _ = N()
    a = put(g, 0, "Anivia, Primal")
    p = put(g, 1, "Legion Rearguard", 1)
    d = put(g, 1, "Mountain Drake", 1)           # 10: survives Anivia's 8 without the trigger
    elsewhere = put(g, 1, "Legion Rearguard")
    move(g, [a], 1)
    assert p.zone == "trash" and d.zone == "trash" and elsewhere.damage == 0


@T.test
def baccai_empower_might_and_deathknell():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    b = put(g, 0, "Baccai Witherclaw")
    assert g.might(b) == 4
    act(g, 0, "Baccai Witherclaw")
    assert b.empowered and g.might(b) == 6
    n = len(g.p[0].runes)
    g.kill([b]); settle(g)
    assert len(g.p[0].runes) == n + 2 and all(r.exhausted for r in g.p[0].runes[-2:])
    b2 = put(g, 0, "Baccai Witherclaw")
    n, k = len(g.p[0].runes), len(g.lines)
    g.kill([b2]); settle(g)
    assert len(g.p[0].runes) == n                 # not Empowered: no Deathknell
    assert not any("Deathknell" in l for l in g.lines[k:]), g.lines[k:]


@T.test
def bilgewater_bully_ganking_while_buffed():
    g, _ = N()
    b = put(g, 0, "Bilgewater Bully")
    assert not g.has_kw(b, "Ganking")
    g.buff(b)
    assert g.has_kw(b, "Ganking") and g.might(b) == 7


@T.test
def brutal_hunter_empowered():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    b = put(g, 0, "Brutal Hunter")
    assert g.might(b) == 4 and not g.has_kw(b, "Ganking")
    act(g, 0, "Brutal Hunter")
    assert g.might(b) == 6 and g.has_kw(b, "Ganking")


@T.test
def buhru_captain_buff_or_draw():
    g, _ = N()
    runes(g, 0, ["Body"] * 4)
    hand(g, 0, "Buhru Captain")
    play(g, 0, "Buhru Captain", lambda ch: ch["loc"] == "base")
    c = [u for u in g.units(0) if u.cname == "Buhru Captain"][0]
    assert c.buff == 1 and len(g.p[0].hand) == 0
    g, _ = N(answers0={"buhru_mode": "draw"})
    runes(g, 0, ["Body"] * 4)
    hand(g, 0, "Buhru Captain")
    play(g, 0, "Buhru Captain", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 1


@T.test
def snapvine_fights_enemy_at_battlefield():
    g, _ = N()
    runes(g, 0, ["Body"] * 7)
    p = put(g, 1, "Laurent Duelist", 1)
    hand(g, 0, "Carnivorous Snapvine")
    play(g, 0, "Carnivorous Snapvine", lambda ch: ch["loc"] == "base")
    s = [u for u in g.units(0) if u.cname == "Carnivorous Snapvine"][0]
    assert p.zone == "trash" and s.damage == 3


@T.test
def cithria_buffed_when_you_play_another_unit():
    g, _ = N()
    runes(g, 0, ["Fury"] * 2)
    c = put(g, 0, "Cithria of Cloudfield")
    hand(g, 0, "Legion Rearguard")
    play(g, 0, "Legion Rearguard", lambda ch: ch["loc"] == "base")
    assert c.buff == 1 and g.might(c) == 2


@T.test
def corrupted_dragon_enters_ready_and_moves_small_enemies():
    g, _ = N()
    runes(g, 0, ["Body"] * 12)
    hand(g, 0, "Corrupted Dragon")
    play(g, 0, "Corrupted Dragon", lambda ch: ch["loc"] == "base" and not ch.get("acc"))
    d = [u for u in g.units(0) if u.cname == "Corrupted Dragon"][0]
    assert not d.exhausted
    g, _ = N()
    g.p[0].points = 5                             # within 3 points of 8: enters exhausted
    runes(g, 0, ["Body"] * 12)
    hand(g, 0, "Corrupted Dragon")
    play(g, 0, "Corrupted Dragon", lambda ch: ch["loc"] == "base")
    d = [u for u in g.units(0) if u.cname == "Corrupted Dragon"][0]
    assert d.exhausted
    g, _ = N()
    d = put(g, 0, "Corrupted Dragon")
    p = put(g, 1, "Legion Rearguard", 1)
    m = put(g, 1, "Mega-Mech", 1)
    move(g, [d], 1)
    assert p.zone == "board" and p.loc == "base" and m.zone == "trash"


@T.test
def crackshot_corsair_pings_on_attack():
    g, _ = N()
    c = put(g, 0, "Crackshot Corsair")
    s = put(g, 1, "Sunlit Guardian", 1)          # 4 as a defender: 1 + 3 kills it
    move(g, [c], 1)
    assert s.zone == "trash"


@T.test
def crowd_favorite_spend_xp_buff():
    g, _ = N()
    c = put(g, 0, "Crowd Favorite")
    assert not act_options(g, 0, "Crowd Favorite")
    g.gain_xp(0, 2)
    act(g, 0, "Crowd Favorite")
    assert c.buff == 1 and g.p[0].xp == 0 and g.kw_value(c, "Hunt") == 1


@T.test
def dame_copies_might_on_attack():
    g, _ = N()
    d = put(g, 0, "Dame the Despoiler")
    d.empowered = True
    e = put(g, 1, "Mountain Drake", 1)
    move(g, [d], 1)
    assert e.zone == "trash" and d.zone == "board" and g.might(d) == 11


@T.test
def dame_not_empowered_no_trigger():
    g, _ = N()
    d = put(g, 0, "Dame the Despoiler")
    e = put(g, 1, "Mountain Drake", 1)
    move(g, [d], 1)
    assert e.zone == "board" and d.zone == "trash"


@T.test
def dazzling_aurora_plays_first_unit_at_end_of_turn():
    g, _ = N()
    put(g, 0, "Dazzling Aurora")
    top = deck_top(g, 0, ["Discipline", "Legion Rearguard", "Back Off"])
    g.apply(("end",))
    settle(g)
    assert top[1].zone == "board" and top[1].ctrl == 0 and top[1].loc == "base"
    assert g.p[0].deck[0] is top[2] and g.p[0].deck[-1] is top[0]


@T.test
def demacian_diplomat_gains_xp():
    g, _ = N()
    runes(g, 0, ["Body"] * 2)
    hand(g, 0, "Demacian Diplomat")
    play(g, 0, "Demacian Diplomat", lambda ch: ch["loc"] == "base")
    assert g.p[0].xp == 1


@T.test
def direwing_ready_with_another_dragon():
    g, _ = N()
    runes(g, 0, ["Body"] * 7)
    put(g, 0, "Mountain Drake")
    hand(g, 0, "Direwing")
    play(g, 0, "Direwing", lambda ch: ch["loc"] == "base")
    assert not [u for u in g.units(0) if u.cname == "Direwing"][0].exhausted
    g, _ = N()
    runes(g, 0, ["Body"] * 7)
    hand(g, 0, "Direwing")
    play(g, 0, "Direwing", lambda ch: ch["loc"] == "base")
    assert [u for u in g.units(0) if u.cname == "Direwing"][0].exhausted


@T.test
def dragonsoul_sage_adds_energy():
    g, _ = N()
    runes(g, 0, ["Fury"])
    s = put(g, 0, "Dragonsoul Sage")
    hand(g, 0, "Legion Rearguard")                     # 2 energy: 1 rune + the Sage
    play(g, 0, "Legion Rearguard", lambda ch: ch["loc"] == "base")
    assert s.exhausted and any(u.cname == "Legion Rearguard" for u in g.units(0))


@T.test
def dune_drake_bonus_against_ready_enemy():
    g, _ = N()
    d = put(g, 0, "Dune Drake")
    put(g, 1, "Legion Rearguard", 1)
    move(g, [d], 1)
    assert g.might(d) == 7
    g, _ = N()
    d = put(g, 0, "Dune Drake")
    put(g, 1, "Legion Rearguard", 1, ready=False)
    move(g, [d], 1)
    assert g.might(d) == 5


@T.test
def fiora_doubles_one_on_one_this_combat():
    g, _ = N()
    f = put(g, 0, "Fiora, Peerless")
    e = put(g, 1, "Laurent Duelist", 1)
    move(g, [f], 1)
    assert e.zone == "trash" and f.zone == "board" and g.might(f) == 3   # doubled only during the combat
    g, _ = N()
    f = put(g, 0, "Fiora, Peerless")
    put(g, 1, "Laurent Duelist", 1)
    put(g, 1, "Legion Rearguard", 1)
    move(g, [f], 1)
    assert f.zone == "trash"                      # not one on one


@T.test
def first_mate_readies_another_unit():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    p = put(g, 0, "Legion Rearguard", ready=False)
    hand(g, 0, "First Mate")
    play(g, 0, "First Mate", lambda ch: ch["loc"] == "base")
    assert not p.exhausted


@T.test
def fretful_feline_and_pirates_jayce_on_ready():
    g, _ = N()
    f = put(g, 0, "Fretful Feline", ready=False)
    j = put(g, 0, "Jayce, Hammer in Hand", ready=False)
    g.ready_obj(f)
    g.ready_obj(j)
    settle(g)
    assert g.might(f) == 7 and g.kw_value(j, "Assault") == 2
    g, _ = N(answers0={"jayce_mode": "Deflect"})
    j = put(g, 0, "Jayce, Hammer in Hand", ready=False)
    g.ready_obj(j)
    settle(g)
    assert g.kw_value(j, "Deflect") == 2 and not g.has_kw(j, "Assault")


@T.test
def keyword_units():
    g, _ = N()
    gr = put(g, 0, "Garen, Rugged")
    assert g.kw_value(gr, "Assault") == 2 and g.kw_value(gr, "Shield") == 2
    lb = put(g, 0, "Laurent Bladekeeper")
    assert g.has_kw(lb, "Ganking")
    mt = put(g, 0, "Master Yi, Tempered")
    gh = put(g, 0, "Gemhand Hunter")
    tv = put(g, 0, "Targonian Visionary")
    assert g.kw_value(mt, "Hunt") == 2 and not g.has_kw(mt, "Deflect") and g.might(gh) == 2
    g.gain_xp(0, 6)
    assert g.has_kw(mt, "Deflect") and g.has_kw(mt, "Ganking") and g.might(gh) == 3 and g.might(tv) == 6
    g.gain_xp(0, 5)
    assert g.might(tv) == 10
    st = put(g, 0, "Stormclaw Ursine")
    assert g.has_kw(st, "Tank")
    for n, kw in (("Kato the Arm", "Deflect"), ("Poppy, Paragon", "Deflect"), ("Qiyana, Victorious", "Deflect"),
                  ("Nilah, Joyful Ascetic", "Ganking"), ("Miss Fortune, Captain", "Ganking"),
                  ("Kraken Hunter", "Assault"), ("Crowd Favorite", "Hunt"), ("Volibear, Imposing", "Tank")):
        assert g.has_kw(put(g, 0, n), kw), n
    assert g.kw_value(put(g, 0, "Sivir, Ambitious"), "Deflect") == 2
    assert g.kw_value(put(g, 0, "Volibear, Imposing"), "Shield") == 3
    assert not g.has_kw(put(g, 0, "Udyr, Wildman"), "Ganking")
    for n in ("Jaull-Fish", "Kraken Hunter", "Lee Sin, Centered", "Miss Fortune, Captain", "Nilah, Joyful Ascetic"):
        assert IMPL[n].accelerate, n
    assert IMPL["Nidalee, Cat Form"].ambush
    for n in ("Jax, Unrelenting", "Lucian, Merciless", "Yone, Blademaster"):
        assert g.has_kw(put(g, 0, n), "Weaponmaster"), n


@T.test
def gentle_gemdragon_readies_runes():
    g, _ = N()
    runes(g, 0, ["Body"] * 8)
    hand(g, 0, "Gentle Gemdragon")
    play(g, 0, "Gentle Gemdragon", lambda ch: ch["loc"] == "base")
    assert sum(1 for r in g.p[0].runes if not r.exhausted) == 2
    # another Dragon played later readies 2 more
    runes(g, 0, ["Body"] * 9)
    hand(g, 0, "Mountain Drake")
    play(g, 0, "Mountain Drake", lambda ch: ch["loc"] == "base")
    assert sum(1 for r in g.p[0].runes if not r.exhausted) == 2


@T.test
def imposing_challenger_moves_smaller_enemy():
    g, _ = N()
    c = put(g, 0, "Imposing Challenger")
    p = put(g, 1, "Legion Rearguard", 1)
    move(g, [c], 1)
    assert p.loc == 0 and g.bfs[1].ctrl == 0


@T.test
def irresistible_faefolk_pulls_enemy():
    g, _ = N()
    f = put(g, 0, "Irresistible Faefolk")
    p = put(g, 1, "Legion Rearguard")
    move(g, [f], 1)
    assert p.loc == 1 and f.zone == "trash"


@T.test
def jaull_fish_cost_per_mighty_unit():
    g, _ = N()
    j = hand(g, 0, "Jaull-Fish")
    e0, _r = total_cost(g, 0, j, dict(loc="base"), "hand")
    put(g, 0, "Mountain Drake")
    put(g, 0, "Mega-Mech")
    put(g, 0, "Legion Rearguard")
    e1, _r = total_cost(g, 0, j, dict(loc="base"), "hand")
    assert e0 == 7 and e1 == 3


@T.test
def jax_attach_pay_to_draw():
    g, _ = N()
    runes(g, 0, ["Body"] * 2)
    j = put(g, 0, "Jax, Unrelenting")
    b = put(g, 0, "Doran's Blade")
    act(g, 0, "Doran's Blade", lambda ch: ch["tg"] == (j.uid,))
    assert b.attached_to == j.uid and len(g.p[0].hand) == 1 and g.might(j) == 5


@T.test
def kato_gives_keywords_and_might():
    g, _ = N()
    k = put(g, 0, "Kato the Arm")
    p = put(g, 0, "Legion Rearguard")
    move(g, [k], 1)
    assert g.has_kw(p, "Deflect") and g.might(p) == 5


@T.test
def khazix_spends_xp_to_deal_might():
    g, _ = N()
    g.p[0].xp = 3
    k = put(g, 0, "Kha'Zix, Evolving Hunter")
    m = put(g, 1, "Mega-Mech", 1)
    move(g, [k], 1)
    assert m.zone == "trash" and g.p[0].xp == 0


@T.test
def kinkou_initiate_draws_with_5_might():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    put(g, 0, "Legion Rearguard")
    hand(g, 0, "Kinkou Initiate")
    play(g, 0, "Kinkou Initiate", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 0
    put(g, 0, "Laurent Duelist")
    runes(g, 0, ["Body"] * 3)
    hand(g, 0, "Kinkou Initiate")
    play(g, 0, "Kinkou Initiate", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 1


@T.test
def kinkou_monk_buffs_two_others():
    g, _ = N()
    runes(g, 0, ["Body"] * 5)
    a, b, c = put(g, 0, "Legion Rearguard"), put(g, 0, "Laurent Duelist"), put(g, 0, "Mega-Mech")
    hand(g, 0, "Kinkou Monk")
    play(g, 0, "Kinkou Monk", lambda ch: ch["loc"] == "base")
    m = [u for u in g.units(0) if u.cname == "Kinkou Monk"][0]
    assert sorted(x.buff for x in (a, b, c)) == [0, 1, 1] and m.buff == 0 and c.buff == 1


@T.test
def kraken_hunter_spends_buffs_for_power():
    g, _ = N()
    p = put(g, 0, "Legion Rearguard")
    g.buff(p)
    k = hand(g, 0, "Kraken Hunter")
    chs = card_choices(g, 0, k, "hand", False, False)
    assert not chs                                  # no runes
    runes(g, 0, ["Body"] * 4)
    e, reqs = total_cost(g, 0, k, dict(loc="base", spend=(p.uid,)), "hand")
    assert e == 3 and len(reqs) == 1
    play(g, 0, "Kraken Hunter", lambda ch: ch.get("spend") == (p.uid,) and ch["loc"] == "base" and not ch["acc"])
    assert p.buff == 0 and any(u.cname == "Kraken Hunter" for u in g.units(0))


@T.test
def lee_sin_buffed_allies_here():
    g, _ = N()
    put(g, 0, "Lee Sin, Centered", 1)
    p = put(g, 0, "Legion Rearguard", 1)
    q = put(g, 0, "Legion Rearguard")
    assert g.might(p) == 2
    g.buff(p); g.buff(q)
    assert g.might(p) == 5 and g.might(q) == 3


@T.test
def legion_marauder_empower_either_cost():
    g, _ = N()
    runes(g, 0, ["Fury"])
    m = put(g, 0, "Legion Marauder")
    acts = act_options(g, 0, "Legion Marauder")
    assert len(acts) == 1                           # 1 energy only (no Body rune)
    g.apply(acts[0]); settle(g)
    assert m.empowered and g.might(m) == 3
    g, _ = N()
    runes(g, 0, ["Body"])
    put(g, 0, "Legion Marauder")
    assert len(act_options(g, 0, "Legion Marauder")) == 2


@T.test
def lucian_readies_first_conquer():
    g, _ = N()
    l = put(g, 0, "Lucian, Merciless")
    move(g, [l], 1)
    assert g.bfs[1].ctrl == 0 and not l.exhausted
    move(g, [l], "base")
    move(g, [l], 0)
    assert g.bfs[0].ctrl == 0 and l.exhausted


@T.test
def master_yi_honed_enters_ready():
    g, _ = N()
    runes(g, 0, ["Body"] * 8)
    hand(g, 0, "Master Yi, Honed")
    play(g, 0, "Master Yi, Honed", lambda ch: ch["loc"] == "base")
    y = [u for u in g.units(0) if u.cname == "Master Yi, Honed"][0]
    assert not y.exhausted and g.has_kw(y, "Ganking")


@T.test
def miss_fortune_readies_something_first_move():
    g, _ = N()
    mf = put(g, 0, "Miss Fortune, Captain")
    p = put(g, 0, "Legion Rearguard", ready=False)
    move(g, [mf], 1)
    assert not p.exhausted
    p.exhausted = True
    move(g, [mf], 0)                               # Ganking; second move this turn: no trigger
    assert p.exhausted


@T.test
def miss_fortune_can_ready_a_rune():
    g, _ = N()
    runes(g, 0, ["Body"])
    g.p[0].runes[0].exhausted = True
    mf = put(g, 0, "Miss Fortune, Captain")
    move(g, [mf], 1)
    assert not g.p[0].runes[0].exhausted


@T.test
def nidalee_draws_on_combat_win():
    g, _ = N()
    n = put(g, 0, "Nidalee, Cat Form")
    put(g, 1, "Legion Rearguard", 1)
    move(g, [n], 1)
    assert len(g.p[0].hand) == 1 and g.bfs[1].ctrl == 0


@T.test
def nilah_gains_xp_when_moving():
    g, _ = N()
    n = put(g, 0, "Nilah, Joyful Ascetic")
    move(g, [n], 1)
    assert g.p[0].xp == 1


@T.test
def noxian_demolitionist_kills_cheap_gear():
    g, _ = N()
    d = put(g, 0, "Noxian Demolitionist")
    s = put(g, 1, "Seal of Strength")
    b = put(g, 1, "Doran's Blade")                 # costs 2 > Might 1
    move(g, [d], 1)
    assert s.zone == "trash" and b.zone == "board"


@T.test
def pit_rookie_buffs_another():
    g, _ = N()
    runes(g, 0, ["Body"] * 2)
    p = put(g, 0, "Legion Rearguard")
    hand(g, 0, "Pit Rookie")
    play(g, 0, "Pit Rookie", lambda ch: ch["loc"] == "base")
    assert p.buff == 1


@T.test
def poppy_ready_and_xp_when_opponent_close():
    g, _ = N()
    runes(g, 0, ["Body"] * 5)
    hand(g, 0, "Poppy, Paragon")
    play(g, 0, "Poppy, Paragon", lambda ch: ch["loc"] == "base")
    assert g.p[0].xp == 0
    g, _ = N()
    g.p[1].points = 5
    runes(g, 0, ["Body"] * 5)
    hand(g, 0, "Poppy, Paragon")
    play(g, 0, "Poppy, Paragon", lambda ch: ch["loc"] == "base")
    p = [u for u in g.units(0) if u.cname == "Poppy, Paragon"][0]
    assert g.p[0].xp == 3 and not p.exhausted


@T.test
def profiteer_moves_empowerment():
    g, _ = N()
    runes(g, 0, ["Body"] * 4)
    m = put(g, 0, "Legion Marauder")
    m.empowered = True
    h = put(g, 0, "Mega-Mech")
    hand(g, 0, "Profiteer")
    play(g, 0, "Profiteer", lambda ch: ch["loc"] == "base")
    assert not m.empowered and h.empowered


@T.test
def qiyana_conquer_channel_or_draw():
    g, _ = N()
    q = put(g, 0, "Qiyana, Victorious")
    move(g, [q], 1)
    assert len(g.p[0].runes) == 1 and g.p[0].runes[0].exhausted
    g, _ = N(answers0={"qiyana_mode": "draw"})
    q = put(g, 0, "Qiyana, Victorious")
    move(g, [q], 1)
    assert len(g.p[0].hand) == 1


@T.test
def repair_specialist_assault_per_gear():
    g, _ = N()
    r = put(g, 0, "Repair Specialist")
    assert g.kw_value(r, "Assault") == 0
    put(g, 0, "Seal of Strength"); put(g, 0, "Doran's Blade")
    put(g, 1, "Seal of Strength")
    assert g.kw_value(r, "Assault") == 2


@T.test
def ruin_runner_untargetable_by_enemies():
    g, _ = N()
    r = put(g, 0, "Ruin Runner", 1)
    runes(g, 1, ["Fury"] * 4)
    hand(g, 1, "Falling Star")
    g.tp = 1
    d = g.advance()
    assert not any(o[0] == "play" and r.uid in o[3].get("tg", ()) for o in d.options)
    assert g.targetable(r, 0) and not g.targetable(r, 1)


@T.test
def sea_monkey_optional_cost_buffs():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    hand(g, 0, "Sea Monkey")
    play(g, 0, "Sea Monkey", lambda ch: ch.get("paid") and ch["loc"] == "base")
    s = [u for u in g.units(0) if u.cname == "Sea Monkey"][0]
    assert s.buff == 1 and all(r.exhausted for r in g.p[0].runes)


@T.test
def sett_buffs_and_spends():
    g, _ = N()
    runes(g, 0, ["Body"] * 6)
    hand(g, 0, "Sett, Brawler")
    play(g, 0, "Sett, Brawler", lambda ch: ch["loc"] == "base")
    s = [u for u in g.units(0) if u.cname == "Sett, Brawler"][0]
    assert s.buff == 1 and g.might(s) == 5
    act(g, 0, "Sett, Brawler")
    assert s.buff == 0 and g.might(s) == 8
    s.exhausted = False
    move(g, [s], 1)                                 # conquer: buff again
    assert s.buff == 1


@T.test
def sivir_excess_damage_after_attack():
    g, _ = N()
    s = put(g, 0, "Sivir, Ambitious")              # 7
    put(g, 1, "Legion Rearguard", 1)                    # needs 2: 5 excess
    m = put(g, 1, "Mega-Mech")
    move(g, [s], 1)
    assert g.bfs[1].ctrl == 0 and m.damage == 5, m.damage
    g, _ = N()
    s = put(g, 0, "Sivir, Ambitious")
    put(g, 1, "Laurent Duelist", 1)               # needs 3: 4 excess
    m = put(g, 1, "Mega-Mech")
    move(g, [s], 1)
    assert g.bfs[1].ctrl == 0 and m.damage == 0


@T.test
def stormclaw_channels():
    g, _ = N()
    runes(g, 0, ["Body"] * 7)
    hand(g, 0, "Stormclaw Ursine")
    play(g, 0, "Stormclaw Ursine", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].runes) == 8


@T.test
def udyr_modes_once_per_turn():
    g, _ = N()
    u = put(g, 0, "Udyr, Wildman")
    p = put(g, 1, "Legion Rearguard", 1)
    assert not act_options(g, 0, "Udyr, Wildman")
    g.buff(u)
    act(g, 0, "Udyr, Wildman", lambda ch: ch["mode"] == "dmg")
    assert p.zone == "trash" and u.buff == 0
    g.buff(u)
    modes = [a[3]["mode"] for a in act_options(g, 0, "Udyr, Wildman")]
    assert "dmg" not in modes and "gank" in modes and "ready" in modes
    act(g, 0, "Udyr, Wildman", lambda ch: ch["mode"] == "gank")
    assert g.has_kw(u, "Ganking")


@T.test
def volibear_draws_once_per_opponent_move():
    g, _ = N(tp=1)
    put(g, 0, "Volibear, Imposing")
    a, b = put(g, 1, "Legion Rearguard"), put(g, 1, "Legion Rearguard")
    move(g, [a, b], 1)
    assert len(g.p[0].hand) == 1
    c = put(g, 1, "Legion Rearguard")
    move(g, [c], 0)
    assert len(g.p[0].hand) == 2


@T.test
def warwick_kills_damaged_enemies():
    g, _ = N()
    runes(g, 0, ["Body"] * 7)
    hand(g, 0, "Warwick, Hunter")
    play(g, 0, "Warwick, Hunter", lambda ch: ch["loc"] == "base")
    w = [u for u in g.units(0) if u.cname == "Warwick, Hunter"][0]
    assert not w.exhausted
    m = put(g, 1, "Mountain Drake", 1)
    m.damage = 1
    move(g, [w], 1)
    assert m.zone == "trash" and w.zone == "board"


@T.test
def wildclaw_shaman_spends_buff():
    g, _ = N()
    runes(g, 0, ["Body"] * 4)
    p = put(g, 0, "Legion Rearguard")
    g.buff(p)
    hand(g, 0, "Wildclaw Shaman")
    play(g, 0, "Wildclaw Shaman", lambda ch: ch["loc"] == "base")
    s = [u for u in g.units(0) if u.cname == "Wildclaw Shaman"][0]
    assert p.buff == 0 and s.buff == 1 and not s.exhausted


@T.test
def yone_conquers_open_battlefield():
    g, _ = N()
    y = put(g, 0, "Yone, Blademaster")
    m = put(g, 1, "Mega-Mech")
    move(g, [y], 1)
    assert m.damage == 5
    g, _ = N()                                      # conquer after a combat: not open
    y = put(g, 0, "Yone, Blademaster")
    put(g, 1, "Legion Rearguard", 1)
    m = put(g, 1, "Mega-Mech")
    move(g, [y], 1)
    assert g.bfs[1].ctrl == 0 and m.damage == 0


@T.test
def yordle_explorer_draws_on_two_power_card():
    g, _ = N()
    runes(g, 0, ["Body"] * 7)
    put(g, 0, "Yordle Explorer")
    hand(g, 0, "Carnivorous Snapvine")
    play(g, 0, "Carnivorous Snapvine", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 1
    runes(g, 0, ["Body"] * 3)
    hand(g, 0, "Buhru Captain")
    g.p[0].hand.remove(g.p[0].hand[0])
    play(g, 0, "Buhru Captain", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 0


# ====================================================================== gear
@T.test
def arena_bar_buffs_exhausted_friend():
    g, _ = N()
    put(g, 0, "Arena Bar")
    p = put(g, 0, "Legion Rearguard")
    assert not act_options(g, 0, "Arena Bar")
    p.exhausted = True
    act(g, 0, "Arena Bar")
    assert p.buff == 1


@T.test
def blood_rose_xp_and_ready():
    g, _ = N()
    runes(g, 0, ["Fury"] * 3)
    br = put(g, 0, "Blood Rose")
    hand(g, 0, "Legion Rearguard")
    play(g, 0, "Legion Rearguard", lambda ch: ch["loc"] == "base")
    assert g.p[0].xp == 1
    p = [u for u in g.units(0) if u.cname == "Legion Rearguard"][0]
    g.gain_xp(0, 2)
    act(g, 0, "Blood Rose", lambda ch: ch["tg"] == (p.uid,))
    assert not p.exhausted and g.p[0].xp == 0 and br.exhausted


@T.test
def equipment_bonuses_and_effects():
    g, _ = N()
    for n, b in (("Boneshiver", 2), ("Doran's Blade", 2), ("Hexdrinker", 1), ("Hunter's Machete", 2),
                 ("Trinity Force", 2), ("Warmog's Armor", 1)):
        assert EQUIP_BONUS[n] == b, n
    u = put(g, 0, "Legion Rearguard")
    attach(g, put(g, 0, "Hexdrinker"), u)
    attach(g, put(g, 0, "Hunter's Machete"), u)
    assert g.might(u) == 5 and g.has_kw(u, "Deflect") and g.kw_value(u, "Hunt") == 1


@T.test
def doran_equip_cost():
    g, _ = N()
    runes(g, 0, ["Fury"])
    u = put(g, 0, "Legion Rearguard")
    put(g, 0, "Doran's Blade")
    assert not act_options(g, 0, "Doran's Blade")
    runes(g, 0, ["Body"])
    act(g, 0, "Doran's Blade")
    assert g.might(u) == 4


@T.test
def boneshiver_warmog_machete_on_conquer():
    g, _ = N()
    u = put(g, 0, "Legion Rearguard")
    attach(g, put(g, 0, "Boneshiver"), u)
    attach(g, put(g, 0, "Warmog's Armor"), u)
    attach(g, put(g, 0, "Hunter's Machete"), u)
    move(g, [u], 1)
    assert len(g.p[0].runes) == 1 and u.buff == 1 and g.p[0].xp == 1 and g.might(u) == 2 + 2 + 1 + 2 + 1


@T.test
def trinity_force_scores_on_hold():
    g, _ = N()
    u = put(g, 0, "Legion Rearguard", 1)
    attach(g, put(g, 0, "Trinity Force"), u)
    g.apply(("end",)); settle(g)
    g.apply(("end",)); settle(g)
    assert g.p[0].points == 2, g.p[0].points


@T.test
def hextech_disc_mech():
    g, _ = N()
    runes(g, 0, ["Body"] * 2)
    d = put(g, 0, "Hextech Disc")
    acts = act_options(g, 0, "Hextech Disc")
    assert len(acts) == 1
    g.apply(acts[0]); settle(g)
    assert d.empowered and d.exhausted
    d.exhausted = False
    act(g, 0, "Hextech Disc")
    assert not d.empowered and d.exhausted and any(u.cname == "Mech" for u in g.units(0))


@T.test
def mistfall_readies_buffed_unit():
    g, _ = N()
    runes(g, 0, ["Body"])
    mf = put(g, 0, "Mistfall")
    p = put(g, 0, "Legion Rearguard", ready=False)
    g.buff(p)
    settle(g)
    assert not p.exhausted and mf.exhausted and len(g.p[0].runes) == 0


@T.test
def petricite_monument_deflect_and_temporary():
    g, _ = N()
    pm = put(g, 0, "Petricite Monument")
    p = put(g, 0, "Legion Rearguard")
    e = put(g, 1, "Legion Rearguard")
    assert g.has_kw(p, "Deflect") and not g.has_kw(e, "Deflect")
    g.apply(("end",)); settle(g)
    g.apply(("end",)); settle(g)
    assert pm.zone == "trash"


@T.test
def platewyrm_egg_adds_energy():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    hand(g, 0, "Platewyrm Egg")
    play(g, 0, "Platewyrm Egg")
    egg = [x for x in g.gear(0) if x.cname == "Platewyrm Egg"][0]
    assert egg.exhausted
    egg.exhausted = False
    runes(g, 0, [])
    assert g.can_pay(0, 1, []) and not g.can_pay(0, 2, [])
    egg.empowered = True
    assert g.can_pay(0, 2, [])


@T.test
def seal_of_strength_adds_body():
    g, _ = N()
    put(g, 0, "Seal of Strength")
    assert g.can_pay(0, 0, [frozenset({"Body"})]) and not g.can_pay(0, 0, [frozenset({"Fury"})])


@T.test
def tools_of_empire_might():
    g, _ = N()
    runes(g, 0, ["Body"] * 2)
    t = put(g, 0, "Tools of Empire")
    p = put(g, 0, "Legion Rearguard")
    act(g, 0, "Tools of Empire", lambda ch: ch.get("tg") == (p.uid,))
    assert g.might(p) == 4
    t.exhausted = False
    act(g, 0, "Tools of Empire", i=0)              # Empower
    t.exhausted = False
    act(g, 0, "Tools of Empire", lambda ch: ch.get("tg") == (p.uid,))
    assert g.might(p) == 8


# ====================================================================== spells
@T.test
def bullet_time_pays_any_amount():
    g, _ = N()
    runes(g, 0, ["Body"] * 4)
    a, b = put(g, 1, "Legion Rearguard", 1), put(g, 1, "Laurent Duelist", 1)
    c = put(g, 1, "Legion Rearguard", 0)
    hand(g, 0, "Bullet Time")
    play(g, 0, "Bullet Time", lambda ch: ch["bf"] == 1)
    assert a.zone == "trash" and b.zone == "trash" and c.damage == 0 and len(g.p[0].runes) == 1


@T.test
def call_to_battle_both_move():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    put(g, 0, "Legion Rearguard", 1)
    l = put(g, 0, "Laurent Duelist")
    m = put(g, 1, "Mega-Mech")
    hand(g, 0, "Call to Battle")
    play(g, 0, "Call to Battle", lambda ch: ch["tg"] == (l.uid,))
    assert m.loc == 1 and m.damage == 0 and l.zone == "trash"


@T.test
def cannon_barrage_hits_units_in_combat():
    g, ag = N()
    runes(g, 0, ["Body"] * 3)
    cb = hand(g, 0, "Cannon Barrage")
    a = put(g, 0, "Mega-Mech")
    d = put(g, 1, "Mountain Drake", 1)
    other = put(g, 1, "Legion Rearguard")
    ag[0].queue = [("play", cb.uid, "hand", {})]
    move(g, [a], 1)
    assert d.zone == "trash" and other.damage == 0


@T.test
def cataclysmic_duel_keeps_one_each():
    g, _ = N()
    runes(g, 0, ["Body"] * 11)
    a, b = put(g, 0, "Mega-Mech"), put(g, 0, "Legion Rearguard")
    c, d = put(g, 1, "Mountain Drake"), put(g, 1, "Legion Rearguard", 1)
    hand(g, 0, "Cataclysmic Duel")
    play(g, 0, "Cataclysmic Duel")
    assert a.zone == "board" and c.zone == "board" and b.zone == "trash" and d.zone == "trash"


@T.test
def catalyst_and_mobilize():
    g, _ = N()
    runes(g, 0, ["Body"] * 4)
    hand(g, 0, "Catalyst of Aeons")
    play(g, 0, "Catalyst of Aeons")
    assert len(g.p[0].runes) == 6 and len(g.p[0].hand) == 0
    g.p[0].rune_deck = g.p[0].rune_deck[:1]
    runes(g, 0, ["Body"] * 4)
    hand(g, 0, "Catalyst of Aeons")
    play(g, 0, "Catalyst of Aeons")
    assert len(g.p[0].runes) == 5 and len(g.p[0].hand) == 1
    runes(g, 0, ["Body"] * 2)
    hand(g, 0, "Mobilize")
    play(g, 0, "Mobilize")
    assert len(g.p[0].runes) == 2 and len(g.p[0].hand) == 2      # no rune left: draw


@T.test
def challenge_units_deal_damage_no_void_gate_bonus():
    g, _ = new()                                    # battlefield 0 is Void Gate
    runes(g, 0, ["Body"] * 3)
    f = put(g, 0, "Mega-Mech")
    e = put(g, 1, "Mountain Drake", 0)
    hand(g, 0, "Challenge")
    play(g, 0, "Challenge")
    assert e.damage == 8 and f.zone == "trash"      # dealt by the units, not the spell: no Void Gate bonus


@T.test
def clash_of_giants_two_enemies():
    g, _ = N()
    runes(g, 0, ["Body"] * 8)
    a, b = put(g, 1, "Mountain Drake"), put(g, 1, "Mega-Mech")
    hand(g, 0, "Clash of Giants")
    play(g, 0, "Clash of Giants")
    assert b.zone == "trash" and a.damage == 8


@T.test
def concentrate_level_discount():
    g, _ = N()
    c = hand(g, 0, "Concentrate")
    assert total_cost(g, 0, c, {}, "hand")[0] == 5
    g.p[0].xp = 6
    assert total_cost(g, 0, c, {}, "hand")[0] == 3
    g.p[0].xp = 11
    assert total_cost(g, 0, c, {}, "hand")[0] == 1
    runes(g, 0, ["Body"])
    play(g, 0, "Concentrate")
    assert len(g.p[0].hand) == 2


@T.test
def confront_units_enter_ready():
    g, _ = N()
    runes(g, 0, ["Body"] * 2 + ["Fury"] * 2)
    hand(g, 0, "Confront")
    play(g, 0, "Confront")
    assert len(g.p[0].hand) == 1
    hand(g, 0, "Legion Rearguard")
    play(g, 0, "Legion Rearguard", lambda ch: ch["loc"] == "base")
    assert not [u for u in g.units(0) if u.cname == "Legion Rearguard"][0].exhausted


@T.test
def decisive_strike_friendly_plus_two():
    g, _ = N()
    runes(g, 0, ["Body"] * 6)
    a, b = put(g, 0, "Legion Rearguard"), put(g, 1, "Legion Rearguard")
    hand(g, 0, "Decisive Strike")
    play(g, 0, "Decisive Strike")
    assert g.might(a) == 4 and g.might(b) == 2


@T.test
def decree_of_strength_and_sabotage_recycle():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    g.p[1].hand = []
    w = hand(g, 1, "Watchful Sentry")              # Mind unit
    s = hand(g, 1, "Discipline")                   # Calm spell
    hand(g, 0, "Decree of Strength")
    play(g, 0, "Decree of Strength")
    assert w.zone == "deck" and g.p[1].deck[-1] is w and s.zone == "hand"
    hand(g, 0, "Sabotage")
    play(g, 0, "Sabotage")
    assert s.zone == "deck"


@T.test
def disposal_order_modes():
    g, _ = N()
    runes(g, 0, ["Body"] * 4)
    a, b = Obj("Discipline", 1), Obj("Falling Star", 1)
    for c in (a, b):
        c.zone = "trash"; g.p[1].trash.append(c)
    hand(g, 0, "Disposal Order")
    play(g, 0, "Disposal Order", lambda ch: ch["mode"] == "recycle" and len(ch["cards"]) == 2)
    assert a.zone == "deck" and b.zone == "deck" and not g.p[1].trash
    hand(g, 0, "Disposal Order")
    play(g, 0, "Disposal Order", lambda ch: ch["mode"] == "draw")
    assert len(g.p[0].hand) == 1


@T.test
def flurry_of_blades_battlefields_only():
    g, _ = N()
    runes(g, 0, ["Body"])
    a, b, c = put(g, 0, "Legion Rearguard", 1), put(g, 1, "Legion Rearguard", 0), put(g, 1, "Legion Rearguard")
    hand(g, 0, "Flurry of Blades")
    play(g, 0, "Flurry of Blades")
    assert a.damage == 1 and b.damage == 1 and c.damage == 0


@T.test
def gentlemens_duel():
    g, _ = N()
    runes(g, 0, ["Body"] * 7)
    f = put(g, 0, "Legion Rearguard")
    e = put(g, 1, "Laurent Duelist")
    hand(g, 0, "Gentlemen's Duel")
    play(g, 0, "Gentlemen's Duel")
    assert e.zone == "trash" and f.damage == 3 and g.might(f) == 5


@T.test
def grim_resolve_xp_on_win():
    g, ag = N()
    runes(g, 0, ["Body"] * 2)
    f = put(g, 0, "Legion Rearguard")
    put(g, 1, "Laurent Duelist", 1)
    hand(g, 0, "Grim Resolve")
    play(g, 0, "Grim Resolve")
    assert g.might(f) == 5
    move(g, [f], 1)
    assert f.zone == "board" and g.bfs[1].ctrl == 0 and g.p[0].xp == 2


@T.test
def guttural_roar_more_when_empowered():
    g, _ = N()
    runes(g, 0, ["Body"] * 4)
    a = put(g, 0, "Legion Rearguard")
    hand(g, 0, "Guttural Roar")
    play(g, 0, "Guttural Roar")
    assert g.might(a) == 4
    b = put(g, 0, "Legion Marauder")
    b.empowered = True
    hand(g, 0, "Guttural Roar")
    play(g, 0, "Guttural Roar", lambda ch: ch["tg"] == (b.uid,))
    assert g.might(b) == 7


@T.test
def here_to_help_plays_unit_cheaper():
    g, _ = N()
    runes(g, 0, ["Body"] * 6)
    put(g, 0, "Legion Rearguard", 1)
    m = hand(g, 0, "Mega-Mech")                    # 7 energy - 3
    hand(g, 0, "Here to Help")
    play(g, 0, "Here to Help")
    assert m.zone == "board" and m.loc == 1 and all(r.exhausted for r in g.p[0].runes)


@T.test
def keepers_verdict_to_deck():
    g, _ = N()
    runes(g, 0, ["Body"] * 4)
    e = put(g, 1, "Mega-Mech", 1)
    hand(g, 0, "Keeper's Verdict")
    play(g, 0, "Keeper's Verdict")
    assert e.zone == "deck" and g.p[1].deck[0] is e


@T.test
def marching_orders_repeat():
    g, _ = N()
    runes(g, 0, ["Body"] * 6)
    f = put(g, 0, "Mega-Mech")
    a, b = put(g, 1, "Legion Rearguard", 1), put(g, 1, "Laurent Duelist", 1)
    hand(g, 0, "Marching Orders")
    play(g, 0, "Marching Orders", lambda ch: ch.get("rep") and set(ch["tg"] + ch["tg2"]) == {f.uid, a.uid, b.uid})
    assert a.zone == "trash" and b.zone == "trash" and f.damage == 5


@T.test
def on_the_hunt_readies():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    a, b = put(g, 0, "Legion Rearguard", ready=False), put(g, 1, "Legion Rearguard", ready=False)
    hand(g, 0, "On the Hunt")
    play(g, 0, "On the Hunt")
    assert not a.exhausted and b.exhausted


@T.test
def onslaught_and_flow():
    g, _ = N()
    runes(g, 0, ["Body"] * 8)
    a = put(g, 0, "Legion Rearguard")
    c = hand(g, 0, "Onslaught")
    play(g, 0, "Onslaught")
    assert g.might(a) == 8 and c.zone == "trash"
    play(g, 0, "Onslaught", src="trash")
    assert g.might(a) == 14 and c.zone == "banish"


@T.test
def overt_operation_spends_and_buffs():
    g, _ = N()
    runes(g, 0, ["Body"] * 7)
    a = put(g, 0, "Legion Rearguard", ready=False)
    g.buff(a)
    b = put(g, 0, "Legion Rearguard")
    hand(g, 0, "Overt Operation")
    play(g, 0, "Overt Operation")
    assert not a.exhausted and a.buff == 1 and b.buff == 1


@T.test
def might_spells():
    g, _ = N()
    runes(g, 0, ["Body"] * 8)
    a = put(g, 0, "Legion Rearguard")
    hand(g, 0, "Primal Strength")
    play(g, 0, "Primal Strength")
    hand(g, 0, "Punch First")
    play(g, 0, "Punch First")
    assert g.might(a) == 14


@T.test
def public_execution_kills_smaller():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    f = put(g, 0, "Laurent Duelist")
    e = put(g, 1, "Legion Rearguard")
    put(g, 1, "Mega-Mech")
    hand(g, 0, "Public Execution")
    play(g, 0, "Public Execution")
    assert e.zone == "trash" and f.zone == "board"


@T.test
def rampage_additional_cost():
    g, _ = N()
    runes(g, 0, ["Body"] * 4)
    f = put(g, 0, "Legion Rearguard")
    e = put(g, 1, "Legion Rearguard")
    hand(g, 0, "Rampage")
    play(g, 0, "Rampage", lambda ch: ch.get("paid"))
    assert e.zone == "trash" and f.zone == "board" and f.damage == 2 and g.might(f) == 4


@T.test
def repulse_counters_enemy_spell():
    g, ag = N(tp=1)
    runes(g, 1, ["Calm"] * 3)
    runes(g, 0, ["Body"] * 2)
    u = put(g, 0, "Legion Rearguard", 1)
    rep = hand(g, 0, "Repulse")
    hand(g, 1, "Back Off")
    g.apply(opt(g, 1, "Back Off", lambda ch: ch["tg"] == (u.uid,)))
    it = g.chain[-1]
    ag[0].queue = [("play", rep.uid, "hand", dict(tg=(u.uid,), item=it.id))]
    settle(g)
    assert not u.stunned and rep.zone == "trash"


@T.test
def riposte_counters_and_pumps():
    g, ag = N(tp=1)
    runes(g, 1, ["Calm"] * 3)
    runes(g, 0, ["Body"] * 2 + ["Order"] * 2)
    u = put(g, 0, "Legion Rearguard", 1)
    rip = hand(g, 0, "Riposte")
    hand(g, 1, "Back Off")
    g.apply(opt(g, 1, "Back Off", lambda ch: ch["tg"] == (u.uid,)))
    it = g.chain[-1]
    ag[0].queue = [("play", rip.uid, "hand", dict(tg=(u.uid,), item=it.id, cost=3))]
    settle(g)
    assert not u.stunned and g.might(u) == 5


@T.test
def show_of_strength_draws_per_mighty():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    put(g, 0, "Mega-Mech"); put(g, 0, "Mountain Drake"); put(g, 0, "Legion Rearguard")
    hand(g, 0, "Show of Strength")
    play(g, 0, "Show of Strength")
    assert len(g.p[0].hand) == 2


@T.test
def showstopper_buffs_and_moves():
    g, _ = N()
    runes(g, 0, ["Body"] * 2)
    a = put(g, 0, "Legion Rearguard")
    hand(g, 0, "Showstopper")
    play(g, 0, "Showstopper", lambda ch: ch["dest"] == 1)
    assert a.buff == 1 and a.loc == 1 and g.bfs[1].ctrl == 0


@T.test
def stare_down_moves_smaller_enemies():
    g, _ = N()
    runes(g, 0, ["Body"] * 2)
    f = put(g, 0, "Laurent Duelist")
    a, b = put(g, 1, "Legion Rearguard", 1), put(g, 1, "Mega-Mech", 1)
    hand(g, 0, "Stare Down")
    play(g, 0, "Stare Down", lambda ch: ch["bf"] == 1)
    assert a.loc == "base" and b.loc == 1 and g.p[0].xp == 1


@T.test
def strike_down_deals_and_detaches():
    g, _ = N()
    runes(g, 0, ["Body"] * 4)
    f = put(g, 0, "Legion Rearguard")
    s = put(g, 0, "Doran's Blade")
    attach(g, s, f)
    e = put(g, 1, "Sunlit Guardian")
    hand(g, 0, "Strike Down")
    play(g, 0, "Strike Down")
    assert e.zone == "trash" and s.attached_to is None and f.attached == [] and g.might(f) == 2


@T.test
def void_assault_moves_both():
    g, _ = N()
    runes(g, 0, ["Body"] * 3)
    f = put(g, 0, "Mega-Mech")
    e = put(g, 1, "Legion Rearguard", 0)
    hand(g, 0, "Void Assault")
    play(g, 0, "Void Assault", lambda ch: ch["d1"] == 1 and ch["d2"] == "base")
    assert f.loc == 1 and e.loc == "base"


@T.test
def wallop_free_with_buff():
    g, _ = N()
    runes(g, 0, [])
    a = put(g, 0, "Legion Rearguard", ready=False)
    g.buff(a)
    hand(g, 0, "Wallop")
    play(g, 0, "Wallop", lambda ch: ch.get("free"))
    assert not a.exhausted and a.buff == 0


@T.test
def wild_claw_plays_reduced_and_empowers():
    g, _ = N()
    runes(g, 0, ["Body"] * 10)
    top = deck_top(g, 0, ["Discipline", "Brutal Hunter", "Legion Rearguard", "Back Off", "Falling Star", "Mega-Mech"])
    hand(g, 0, "Wild Claw")
    play(g, 0, "Wild Claw")
    b = top[1]
    assert b.zone == "board" and b.empowered and g.might(b) == 6
    assert g.p[0].deck[0] is top[5] and top[0].zone == "deck" and top[2].zone == "deck"


# ====================================================================== the cards that needed engine hooks
@T.test
def hook_cards_are_registered():
    for n in ("Akshan, Mischievous", "Ambessa, The Wolf", "Ancient Henge", "Arachnoid Horror", "Dauntless Vanguard",
              "Deadbloom Predator", "Determined Sentry", "Elder Dragon", "Fae Dragon", "Gangplank, Naval",
              "Herald of Scales", "Jagged Cutlass", "Pirate's Haven", "Renekton, Brute", "Rengar, Trophy Hunter",
              "Spoils of War", "Unyielding Spirit", "Wily Newtfish"):
        assert n in IMPL and IMPL[n].module == "cardsets.body", n


# ------------------------------------------------------------------ cards using the engine hooks (integration)
SKb, VSb, MDb = "Shipyard Skulker", "Vanguard Sergeant", "Mountain Drake"      # vanilla 3/4/10


def _locs(g, pid, c, closed=False):
    return sorted(str(ch["loc"]) for ch in card_choices(g, pid, c, "hand", closed, closed))


@T.test
def arachnoid_horror_plays_next_to_a_lone_enemy():
    g, _ = N()
    runes(g, 0, ["Body"] * 10)
    put(g, 1, SKb, 1)
    ah, sk = hand(g, 0, "Arachnoid Horror"), hand(g, 0, SKb)
    assert "1" in _locs(g, 0, ah) and "1" not in _locs(g, 0, sk)
    put(g, 0, "Arachnoid Horror")
    assert "1" in _locs(g, 0, sk)
    put(g, 1, SKb, 1)
    assert "1" not in _locs(g, 0, sk) and "1" not in _locs(g, 0, ah)


@T.test
def dauntless_vanguard_and_deadbloom_to_occupied_enemy_battlefield():
    g, _ = N()
    runes(g, 0, ["Body"] * 10)
    put(g, 1, SKb, 1)
    for n in ("Dauntless Vanguard", "Deadbloom Predator"):
        c = hand(g, 0, n)
        assert _locs(g, 0, c) == ["1", "base"]
    assert g.kw_value(put(g, 0, "Deadbloom Predator"), "Deflect") == 1


@T.test
def rengar_trophy_hunter_ambushes_enemy_battlefields():
    g, _ = N()
    runes(g, 0, ["Body"] * 6)
    put(g, 1, SKb, 1)
    c = hand(g, 0, "Rengar, Trophy Hunter")
    assert _locs(g, 0, c, closed=True) == ["1"]


@T.test
def ambessa_the_wolf_only_hurt_in_combat():
    g, _ = N()
    a = put(g, 0, "Ambessa, The Wolf")
    assert g.deal(a, 2, "spell", 1) == 2
    a.damage = 0
    a.empowered = True
    assert g.might(a) == 7 and g.deal(a, 2, "spell", 1) == 0
    e = put(g, 1, VSb, 1)
    move(g, [a], 1)
    assert e.zone == "trash" and a.damage == 0 and a.zone == "board"      # took 4 in combat (healed after)


@T.test
def unyielding_spirit_prevents_spell_and_ability_damage():
    g, _ = N()
    runes(g, 0, ["Body"] * 2)
    u, e = put(g, 0, VSb), put(g, 1, SKb)
    hand(g, 0, "Unyielding Spirit")
    play(g, 0, "Unyielding Spirit")
    assert g.deal(u, 3, "spell", 1) == 0 and g.deal(e, 3, "ability", 0) == 0 and g.deal(e, 1, "unit", 0) == 1


@T.test
def elder_dragon_any_damage_kills_enemies():
    g, _ = N()
    runes(g, 0, ["Body"] * 12)
    a, b = put(g, 1, MDb), put(g, 1, VSb, 1)
    c = put(g, 1, SKb)
    hand(g, 0, "Elder Dragon")
    play(g, 0, "Elder Dragon", lambda ch: ch["loc"] == "base")
    assert a.zone == "trash" and b.zone == "trash" and c.zone == "board"   # up to one per location
    d = put(g, 1, MDb, 1)
    sk = put(g, 0, SKb)
    move(g, [sk], 1)
    assert d.zone == "trash"


@T.test
def determined_sentry_cant_move_to_base():
    g, _ = N()
    s_ = put(g, 0, "Determined Sentry", 0)
    assert not [o for o in options_of(g, "move") if o[2] == "base"]
    g.move([s_], "base", 0)
    assert s_.loc == 0
    g.recall(s_)
    assert s_.loc == "base"


@T.test
def jagged_cutlass_not_moved_by_enemies():
    g, _ = N()
    runes(g, 0, ["Body"])
    jc, u = put(g, 0, "Jagged Cutlass"), put(g, 0, SKb, 0)
    act(g, 0, "Jagged Cutlass")
    assert g.might(u) == 5
    g.move([u], "base", 1)
    assert u.loc == 0
    g.move([u], "base", 0)
    assert u.loc == "base"


@T.test
def fae_dragon_buffs_and_golds_on_spent_buffs():
    g, _ = N()
    runes(g, 0, ["Body"] * 7)
    us = [put(g, 0, SKb) for _ in range(5)]
    hand(g, 0, "Fae Dragon")
    play(g, 0, "Fae Dragon", lambda ch: ch["loc"] == "base")
    assert sum(u.buff for u in us) + units_named(g, 0, "Fae Dragon")[0].buff == 4
    b = [u for u in us if u.buff][0]
    g.spend_buff(b, 0); settle(g)
    golds = [o for o in g.gear(0) if o.cname == "Gold"]
    assert len(golds) == 1 and golds[0].exhausted


def units_named(g, pid, name):
    return [u for u in g.units(pid) if u.cname == name]


@T.test
def pirates_haven_pumps_units_you_ready():
    g, _ = N()
    put(g, 0, "Pirate's Haven")
    u = put(g, 0, SKb, ready=False)
    g.ready_obj(u, by=1)
    assert g.might(u) == 3
    u.exhausted = True
    g.ready_obj(u, by=0); settle(g)
    assert g.might(u) == 4


@T.test
def renekton_brute_empowers_at_ten_might():
    g, _ = N()
    runes(g, 0, ["Body"] * 6)
    r = put(g, 0, "Renekton, Brute")
    settle(g)
    for _ in range(5):
        act(g, 0, "Renekton, Brute")
    assert not r.empowered
    act(g, 0, "Renekton, Brute")
    assert g.might(r) == 10 and r.empowered and g.has_kw(r, "Ganking") and g.kw_value(r, "Deflect") == 1


@T.test
def spoils_of_war_cheaper_after_an_enemy_death():
    g, _ = N()
    c = hand(g, 0, "Spoils of War")
    assert total_cost(g, 0, c, {}, "hand")[0] == 4
    g.kill([put(g, 0, SKb)])
    assert total_cost(g, 0, c, {}, "hand")[0] == 4
    g.kill([put(g, 1, SKb)])
    assert total_cost(g, 0, c, {}, "hand")[0] == 2


@T.test
def wily_newtfish_after_xp():
    g, _ = N()
    w = put(g, 0, "Wily Newtfish")
    assert g.might(w) == 4 and not g.has_kw(w, "Ganking")
    g.gain_xp(0, 1)
    assert g.might(w) == 5 and g.has_kw(w, "Ganking")


@T.test
def herald_of_scales_reduces_dragons():
    g, _ = N()
    put(g, 0, "Herald of Scales")
    ed, fd, sk = hand(g, 0, "Elder Dragon"), hand(g, 0, "Fae Dragon"), hand(g, 0, SKb)
    assert total_cost(g, 0, ed, {"loc": "base"}, "hand")[0] == 10
    assert total_cost(g, 0, fd, {"loc": "base"}, "hand")[0] == 5
    assert total_cost(g, 0, sk, {"loc": "base"}, "hand")[0] == 3
    assert total_cost(g, 1, hand(g, 1, "Fae Dragon"), {"loc": "base"}, "hand")[0] == 7


@T.test
def ancient_henge_turns_energy_into_power():
    g, _ = N()
    runes(g, 0, ["Calm"] * 2)
    c = hand(g, 0, "Unyielding Spirit")
    assert not card_choices(g, 0, c, "hand", False, False)
    h = put(g, 0, "Ancient Henge")
    assert card_choices(g, 0, c, "hand", False, False)
    play(g, 0, "Unyielding Spirit")
    assert h.exhausted and all(r.exhausted for r in g.p[0].runes)


@T.test
def akshan_steals_an_equipment_until_he_leaves():
    g, _ = N()
    runes(g, 0, ["Body"] * 6)
    e = put(g, 1, SKb)
    jc = put(g, 1, "Jagged Cutlass")
    attach(g, jc, e)
    hand(g, 0, "Akshan, Mischievous")
    play(g, 0, "Akshan, Mischievous", lambda ch: ch["loc"] == "base" and ch.get("akshan"))
    ak = units_named(g, 0, "Akshan, Mischievous")[0]
    assert jc.ctrl == 0 and jc.attached_to == ak.uid and e.attached == [] and g.might(ak) == 6
    g.kill([ak]); settle(g)
    assert jc.ctrl == 1 and jc.attached_to is None and jc.zone == "board"


@T.test
def gangplank_naval_turns_choosing_effects_into_might():
    g, _ = N()
    gp = put(g, 0, "Gangplank, Naval")
    gp.empowered = True
    it = Item("spell", 1, "test")
    g.add_target(it, gp)
    g.resolving = it
    g.stun(gp, 1)
    g.mod(gp, -2)
    g.to_zone(gp, "hand")
    g.resolving = None
    assert not gp.stunned and gp.zone == "board" and g.might(gp) == 6 + 9
    gp.empowered = False
    g.resolving = it
    g.stun(gp, 1)
    g.resolving = None
    assert gp.stunned


if __name__ == "__main__":
    T.main()
