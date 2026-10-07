"""Tests of batch mind. Run: RB_CARDSETS=mind python3 cardsets/test_mind.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from actions import attach, play_card                   # noqa: E402

T = Suite("mind")


def trash(g, pid, name):
    o = Obj(name, pid)
    o.zone = "trash"
    g.p[pid].trash.append(o)
    return o


def named(g, pid, name):
    return [u for u in g.board if u.ctrl == pid and u.cname == name]


def play(g, pid, name, pred=lambda ch: True, src=None):
    g.apply(opt(g, pid, name, pred, src))
    return settle(g)


def act(g, pid, cname, pred=lambda o: True):
    acts = [a for a in act_options(g, pid, cname) if pred(a)]
    assert acts, f"no ability option for {cname}"
    g.apply(acts[0])
    return settle(g)


def end_turns(g, n):
    for _ in range(n):
        g.apply(("end",))
        settle(g)


# ------------------------------------------------------------------ chunk 1
@T.test
def ahri_attack_shrinks_defender():
    g, _ = new()
    a = put(g, 0, "Ahri, Inquisitive")
    d = put(g, 1, "Laurent Duelist", 1)
    g.apply(("move", (a.uid,), 1))
    settle(g)
    assert d.zone == "trash" and a.zone == "board" and g.bfs[1].ctrl == 0, (d.zone, a.zone)


@T.test
def ahri_minimum_one():
    g, _ = new()
    a = put(g, 0, "Ahri, Inquisitive")
    d = put(g, 1, "Watchful Sentry", 1)           # 1 might: stays 1
    seen = []
    g.effects.append(dict(on="attack", fn=lambda g_, e, i: seen.append(g_.might(d))))
    g.apply(("move", (a.uid,), 1))
    settle(g)
    assert d.zone == "trash"


@T.test
def apprentice_mage_empower_predicts():
    g, _ = new(answers0={"predict_recycle": True})
    runes(g, 0, ["Mind"] * 2)
    m = put(g, 0, "Apprentice Mage")
    top = deck_top(g, 0, ["Discipline", "Back Off"])
    act(g, 0, "Apprentice Mage")
    assert m.empowered and g.might(m) == 4
    assert g.p[0].deck[0] is not top[0] and top[0] in g.p[0].deck[-2:] and top[1] in g.p[0].deck[-2:]


@T.test
def aspiring_engineer_returns_gear():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    s = trash(g, 0, "Long Sword")
    hand(g, 0, "Aspiring Engineer")
    play(g, 0, "Aspiring Engineer", lambda ch: ch["loc"] == "base")
    assert s in g.p[0].hand and s not in g.p[0].trash


@T.test
def ava_plays_hidden_card_here():
    g, _ = new()
    runes(g, 0, ["Mind"])
    ava = put(g, 0, "Ava Achiever")
    e = put(g, 1, "Mega-Mech", 1)
    fae = hand(g, 0, "Blastcone Fae")
    g.apply(("move", (ava.uid,), 1))
    settle(g)
    assert fae.zone in ("board", "trash") and len(g.p[0].runes) == 0      # paid [Mind] (recycled)
    assert any("Blastcone Fae" in l and "plays" in l for l in g.lines), g.lines[-20:]


@T.test
def ava_spell_restricted_here():
    g, _ = new()
    runes(g, 0, ["Mind"])
    ava = put(g, 0, "Ava Achiever")
    e1 = put(g, 1, "Mountain Drake", 1)
    e0 = put(g, 1, "Pouty Poro", 0)
    hand(g, 0, "Wages of Pain")
    g.apply(("move", (ava.uid,), 1))
    settle(g)
    assert e0.damage == 0 and e0.zone == "board"            # Wages chose a unit at Ava's battlefield only


@T.test
def bard_moves_units_to_open_battlefield():
    g, _ = new()
    runes(g, 0, ["Mind"] * 5)
    u = put(g, 0, "Stalwart Poro")
    hand(g, 0, "Bard, Mercurial")
    play(g, 0, "Bard, Mercurial", lambda ch: ch.get("bard") and ch["loc"] == "base")
    bard = named(g, 0, "Bard, Mercurial")[0]
    assert g.p[0].legend.exhausted
    assert u.loc == 0 and bard.loc == 0 and g.bfs[0].ctrl == 0


@T.test
def bard_without_cost_does_nothing():
    g, _ = new()
    runes(g, 0, ["Mind"] * 5)
    u = put(g, 0, "Stalwart Poro")
    hand(g, 0, "Bard, Mercurial")
    play(g, 0, "Bard, Mercurial", lambda ch: not ch.get("bard") and ch["loc"] == "base")
    assert not g.p[0].legend.exhausted and u.loc == "base"


@T.test
def blastcone_fae_minus_two():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    e = put(g, 1, "Mega-Mech", 1)
    hand(g, 0, "Blastcone Fae")
    play(g, 0, "Blastcone Fae", lambda ch: ch["loc"] == "base")
    assert g.might(e) == 6


@T.test
def blastcone_fae_from_hidden_restricted():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2)
    put(g, 0, "Stalwart Poro", 0)
    far = put(g, 1, "Mega-Mech", 1)
    near = put(g, 1, "Pouty Poro", 0, bf_control=False)
    c = hand(g, 0, "Blastcone Fae")
    g.apply(("hide", c.uid, 0))
    settle(g)
    end_turns(g, 2)
    runes(g, 0, ["Mind"] * 2)
    near.mods.clear()
    g.apply(opt(g, 0, "Blastcone Fae", src="facedown"))
    settle(g)
    assert g.might(far) == 8, g.might(far)


@T.test
def breakneck_mech_aura_and_ready():
    g, _ = new()
    runes(g, 0, ["Mind"] * 10)
    f = put(g, 0, "Forecaster")
    hand(g, 0, "Breakneck Mech")
    play(g, 0, "Breakneck Mech", lambda ch: ch["loc"] == "base")
    b = named(g, 0, "Breakneck Mech")[0]
    assert not b.exhausted
    assert g.has_kw(f, "Ganking") and g.kw_value(f, "Deflect") == 1 and g.has_kw(b, "Deflect")
    o = put(g, 0, "Stalwart Poro")
    assert not g.has_kw(o, "Ganking")


@T.test
def breakneck_mech_exhausted_alone():
    g, _ = new()
    runes(g, 0, ["Mind"] * 10)
    hand(g, 0, "Breakneck Mech")
    play(g, 0, "Breakneck Mech", lambda ch: ch["loc"] == "base")
    assert named(g, 0, "Breakneck Mech")[0].exhausted


@T.test
def bubble_bot_readies_mech():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    m = put(g, 0, "Mega-Mech", ready=False)
    hand(g, 0, "Bubble Bot")
    play(g, 0, "Bubble Bot", lambda ch: ch["loc"] == "base")
    assert not m.exhausted


@T.test
def card_sharp_golds():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    hand(g, 0, "Card Sharp")
    play(g, 0, "Card Sharp", lambda ch: ch["loc"] == "base")
    assert len(named(g, 0, "Gold")) == 2 and len(named(g, 1, "Gold")) == 1
    assert all(x.exhausted for x in g.gear())


@T.test
def card_sharp_opponent_declines():
    g, _ = new(answers1={"may": False})
    runes(g, 0, ["Mind"] * 3)
    hand(g, 0, "Card Sharp")
    play(g, 0, "Card Sharp", lambda ch: ch["loc"] == "base")
    assert len(named(g, 0, "Gold")) == 1 and len(named(g, 1, "Gold")) == 0


@T.test
def draw_on_play_units():
    g, _ = new()
    runes(g, 0, ["Mind"] * 9)
    hand(g, 0, "Cloud Drake")
    hand(g, 0, "Lecturing Yordle")
    play(g, 0, "Cloud Drake", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 2
    play(g, 0, "Lecturing Yordle", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 2
    assert g.has_kw(named(g, 0, "Lecturing Yordle")[0], "Tank")


@T.test
def covert_informant_draws_when_empowered():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    c = put(g, 0, "Covert Informant")
    g.move([c], 0, 0)
    settle(g)
    assert len(g.p[0].hand) == 0
    act(g, 0, "Covert Informant")
    assert c.empowered
    g.move([c], "base", 0)
    settle(g)
    assert len(g.p[0].hand) == 1


@T.test
def diana_predicts_and_draws_spell():
    g, _ = new()
    runes(g, 0, ["Mind"])
    d = put(g, 0, "Diana, Lunari")
    put(g, 1, "Pouty Poro", 1)
    top = deck_top(g, 0, ["Discipline"])
    g.apply(("move", (d.uid,), 1))
    settle(g)
    assert top[0] in g.p[0].hand and g.p[0].runes[0].exhausted


@T.test
def diana_reveals_unit_no_draw():
    g, _ = new()
    runes(g, 0, ["Mind"])
    d = put(g, 0, "Diana, Lunari")
    put(g, 1, "Pouty Poro", 1)
    top = deck_top(g, 0, ["Stalwart Poro"])
    g.apply(("move", (d.uid,), 1))
    settle(g)
    assert top[0] not in g.p[0].hand and g.p[0].deck[0] is top[0]


@T.test
def dr_mundo_might_and_recycle():
    g, _ = new()
    m = put(g, 0, "Dr. Mundo, Expert")
    for n in ("Discipline", "Back Off", "Stalwart Poro", "Long Sword"):
        trash(g, 0, n)
    assert g.might(m) == 10
    end_turns(g, 2)
    assert len(g.p[0].trash) == 1 and g.might(m) == 7, (len(g.p[0].trash), g.might(m))


@T.test
def dramatic_visionary_deathknell_predicts():
    g, _ = new(answers0={"predict_recycle": True})
    v = put(g, 0, "Dramatic Visionary")
    top = deck_top(g, 0, ["Discipline", "Back Off"])
    g.kill([v])
    settle(g)
    assert g.p[0].deck[0] not in top and set(g.p[0].deck[-2:]) == set(top)


@T.test
def dropboarder_ready_with_two_gear():
    g, _ = new()
    runes(g, 0, ["Mind"] * 8)
    hand(g, 0, "Dropboarder")
    hand(g, 0, "Dropboarder")
    put(g, 0, "Long Sword")
    play(g, 0, "Dropboarder", lambda ch: ch["loc"] == "base")
    assert named(g, 0, "Dropboarder")[0].exhausted
    put(g, 0, "Sterak's Gage")
    play(g, 0, "Dropboarder", lambda ch: ch["loc"] == "base")
    assert sum(1 for u in named(g, 0, "Dropboarder") if not u.exhausted) == 1


@T.test
def ekko_recycles_and_readies_runes():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    for r in g.p[0].runes:
        r.exhausted = True
    e = put(g, 0, "Ekko, Recurrent")
    g.kill([e])
    settle(g)
    assert e not in g.p[0].trash and g.p[0].deck[-1] is e
    assert not any(r.exhausted for r in g.p[0].runes)


@T.test
def ekko_accelerate():
    g, _ = new()
    runes(g, 0, ["Mind"] * 8)
    hand(g, 0, "Ekko, Recurrent")
    play(g, 0, "Ekko, Recurrent", lambda ch: ch["loc"] == "base" and ch["acc"])
    assert not named(g, 0, "Ekko, Recurrent")[0].exhausted


@T.test
def fate_weaver_draws_big_spell():
    g, _ = new()
    runes(g, 0, ["Mind"] * 5)
    top = deck_top(g, 0, ["Discipline", "Falling Comet", "Stalwart Poro", "Back Off"])
    hand(g, 0, "Fate Weaver")
    play(g, 0, "Fate Weaver", lambda ch: ch["loc"] == "base")
    assert top[1] in g.p[0].hand and len(g.p[0].hand) == 1
    assert set(g.p[0].deck[-3:]) == {top[0], top[2], top[3]}


@T.test
def forecaster_gives_mechs_vision():
    g, _ = new(answers0={"predict_recycle": True})
    runes(g, 0, ["Mind"] * 3)
    put(g, 0, "Forecaster")
    top = deck_top(g, 0, ["Discipline"])
    hand(g, 0, "Bubble Bot")
    play(g, 0, "Bubble Bot", lambda ch: ch["loc"] == "base")
    assert g.p[0].deck[-1] is top[0]
    assert not g.has_kw(put(g, 0, "Stalwart Poro"), "Vision")


@T.test
def frostcoat_cub_additional_cost():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    e = put(g, 1, "Mega-Mech", 1)
    hand(g, 0, "Frostcoat Cub")
    play(g, 0, "Frostcoat Cub", lambda ch: ch["loc"] == "base" and ch.get("cub"))
    assert g.might(e) == 6 and len(g.p[0].runes) == 3
    hand(g, 0, "Frostcoat Cub")
    runes(g, 0, ["Mind"] * 3)
    play(g, 0, "Frostcoat Cub", lambda ch: ch["loc"] == "base" and not ch.get("cub"))
    assert g.might(e) == 6 and len(g.p[0].runes) == 3


@T.test
def gearhead_doubles_equipment():
    g, _ = new()
    gh = put(g, 0, "Gearhead")
    s = put(g, 0, "Long Sword")
    attach(g, s, gh)
    assert g.might(gh) == 3 + 4


@T.test
def gemcraft_seer_vision_for_others():
    g, _ = new(answers0={"predict_recycle": True})
    runes(g, 0, ["Mind"] * 6)
    put(g, 0, "Gemcraft Seer")
    top = deck_top(g, 0, ["Discipline"])
    hand(g, 0, "Stalwart Poro")
    play(g, 0, "Stalwart Poro", lambda ch: ch["loc"] == "base")
    assert top[0] in g.p[0].deck[-1:]


@T.test
def grumpy_rockbear_empower_discount():
    g, _ = new()
    runes(g, 0, ["Mind"] * 10)
    r = put(g, 0, "Grumpy Rockbear")
    assert not g.has_kw(r, "Deflect")
    act(g, 0, "Grumpy Rockbear")
    assert r.empowered and g.kw_value(r, "Shield") == 3 and g.has_kw(r, "Deflect")
    assert sum(1 for x in g.p[0].runes if x.exhausted) == 2


@T.test
def gustwalker_level():
    g, _ = new()
    u = put(g, 0, "Gustwalker", 0)
    assert g.might(u) == 3 and not g.has_kw(u, "Ganking") and g.kw_value(u, "Hunt") == 2
    g.gain_xp(0, 3)
    assert g.might(u) == 4 and g.has_kw(u, "Ganking")



# ------------------------------------------------------------------ chunk 2
@T.test
def hwei_move_discard_unit_buffs():
    g, _ = new(answers0={"discard": lambda opts, ctx: [c for c in opts if c.spec["type"] == "Unit"][0]})
    h = put(g, 0, "Hwei, Brooding Painter")
    hand(g, 0, "Stalwart Poro")
    deck_top(g, 0, ["Discipline"])
    g.move([h], 0, 0)
    settle(g)
    assert g.might(h) == 8 and len(g.p[0].hand) == 1 and g.p[0].trash[-1].cname == "Stalwart Poro"


@T.test
def hwei_discard_spell_draws_gear_readies():
    g, _ = new(answers0={"discard": lambda opts, ctx: [c for c in opts if c.spec["type"] == "Spell"][0]})
    h = put(g, 0, "Hwei, Brooding Painter")
    deck_top(g, 0, ["Discipline", "Stalwart Poro"])
    g.move([h], 0, 0)
    settle(g)
    assert [c.cname for c in g.p[0].hand] == ["Stalwart Poro"] and g.might(h) == 5
    g, _ = new(answers0={"discard": lambda opts, ctx: [c for c in opts if c.spec["type"] == "Gear"][0]})
    runes(g, 0, ["Mind"] * 3)
    for r in g.p[0].runes:
        r.exhausted = True
    h = put(g, 0, "Hwei, Brooding Painter")
    deck_top(g, 0, ["Long Sword"])
    g.move([h], 0, 0)
    settle(g)
    assert sum(1 for r in g.p[0].runes if not r.exhausted) == 2


@T.test
def icevale_archer_pays_to_shrink():
    g, _ = new()
    runes(g, 0, ["Mind"])
    a = put(g, 0, "Icevale Archer")
    d = put(g, 1, "Stalwart Poro", 1)             # 2 + Shield 1 = 3 as defender
    g.apply(("move", (a.uid,), 1))
    settle(g)
    assert g.p[0].runes[0].exhausted
    assert d.zone == "trash" and a.zone == "trash"   # 2 vs 3-1=2: both die


@T.test
def jayce_brilliant_readies_on_play_and_first_gear():
    g, _ = new()
    runes(g, 0, ["Mind"] * 12)
    u = put(g, 0, "Stalwart Poro", ready=False)
    hand(g, 0, "Jayce, Brilliant Inventor")
    play(g, 0, "Jayce, Brilliant Inventor", lambda ch: ch["loc"] == "base")
    assert not u.exhausted
    u.exhausted = True
    hand(g, 0, "Chemtech Cask")
    hand(g, 0, "Chemtech Cask")
    play(g, 0, "Chemtech Cask")
    assert not u.exhausted
    u.exhausted = True
    play(g, 0, "Chemtech Cask")
    assert u.exhausted                              # only the first non-token gear each turn


@T.test
def jayce_brilliant_readies_rune():
    g, _ = new(answers0={"ready_pick": lambda opts, ctx: [o for o in opts if o[0] == "rune"][0]})
    runes(g, 0, ["Mind"] * 7)
    hand(g, 0, "Jayce, Brilliant Inventor")
    play(g, 0, "Jayce, Brilliant Inventor", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].runes) == 6 and sum(1 for r in g.p[0].runes if r.exhausted) == 4


@T.test
def keeper_of_masks_two_copies():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2)
    hand(g, 0, "Keeper of Masks")
    play(g, 0, "Keeper of Masks", lambda ch: ch["loc"] == "base")
    ks = [u for u in g.units(0) if u.cname == "Keeper of Masks"]
    assert len(ks) == 3 and sum(1 for k in ks if k.token) == 2
    assert all(g.has_kw(k, "Temporary") and g.might(k) == 1 for k in ks)


@T.test
def lillia_leaves_sprite_behind():
    g, _ = new()
    l = put(g, 0, "Lillia, Fae Fawn")
    g.apply(("move", (l.uid,), 0))
    settle(g)
    sp = named(g, 0, "Sprite")
    assert len(sp) == 1 and sp[0].loc == "base" and sp[0].exhausted and g.has_kw(sp[0], "Temporary")


@T.test
def lux_illuminated_big_spell():
    g, _ = new()
    runes(g, 0, ["Mind"] * 10)
    lux = put(g, 0, "Lux, Illuminated")
    e = put(g, 1, "Mountain Drake", 1)
    hand(g, 0, "Falling Comet")
    hand(g, 0, "Smoke Screen")
    play(g, 0, "Smoke Screen")
    assert g.might(lux) == 5
    play(g, 0, "Falling Comet")
    assert g.might(lux) == 8


@T.test
def nasus_once_per_turn():
    g, _ = new()
    n = put(g, 0, "Nasus, Guardian of Knowledge", 1)
    g.p[0].rune_deck = [Rune("Mind", 0) for _ in range(4)]
    a = put(g, 1, "Pouty Poro", 1, bf_control=False)
    b = put(g, 1, "Stalwart Poro", 1, bf_control=False)
    g.kill([a]); settle(g)
    g.kill([b]); settle(g)
    assert len(g.p[0].runes) == 1 and g.p[0].runes[0].exhausted


@T.test
def ornn_might_per_gear():
    g, _ = new()
    o = put(g, 0, "Ornn, Forge God")
    put(g, 0, "Chemtech Cask"); put(g, 0, "Long Sword"); put(g, 1, "Long Sword")
    assert g.might(o) == 6 and g.kw_value(o, "Deflect") == 2 and g.has_kw(o, "Weaponmaster")


@T.test
def patched_porobot_draws_with_three_gear():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    hand(g, 0, "Patched Porobot"); hand(g, 0, "Patched Porobot")
    put(g, 0, "Chemtech Cask"); put(g, 0, "Long Sword")
    play(g, 0, "Patched Porobot", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 1
    put(g, 0, "Sterak's Gage")
    play(g, 0, "Patched Porobot", lambda ch: ch["loc"] == "base")
    assert len(g.p[0].hand) == 1 and all(u.exhausted for u in named(g, 0, "Patched Porobot"))


@T.test
def petal_pixie_counts_temporary():
    g, _ = new()
    p = put(g, 0, "Petal Pixie", 0)
    from cards import make_token
    make_token(g, "Sprite", 0, 0)
    make_token(g, "Sprite", 0, "base")
    assert g.might(p) == 3


@T.test
def pickpocket_kills_cheap_gear_for_gold():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    x = put(g, 1, "Chemtech Cask")
    put(g, 1, "Long Sword")                         # costs 2: not a choice
    hand(g, 0, "Pickpocket")
    play(g, 0, "Pickpocket", lambda ch: ch["loc"] == "base")
    assert x.zone == "trash" and len(named(g, 0, "Gold")) == 1


@T.test
def pit_crew_readies_on_gear():
    g, _ = new()
    runes(g, 0, ["Mind"])
    pc = put(g, 0, "Pit Crew", ready=False)
    hand(g, 0, "Chemtech Cask")
    play(g, 0, "Chemtech Cask")
    assert not pc.exhausted


@T.test
def pit_crew_readies_on_gold_token():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    pc = put(g, 0, "Pit Crew", ready=False)
    hand(g, 0, "Card Sharp")
    play(g, 0, "Card Sharp", lambda ch: ch["loc"] == "base")
    assert not pc.exhausted


@T.test
def plaza_guardian_discount():
    g, _ = new()
    c = hand(g, 0, "Plaza Guardian")
    put(g, 0, "Chemtech Cask"); put(g, 0, "Long Sword"); put(g, 1, "Sterak's Gage")
    e, _r = total_cost(g, 0, c, dict(loc="base"), "hand")
    assert e == 8


@T.test
def plundering_poro_gold_on_conquer():
    g, _ = new()
    p = put(g, 0, "Plundering Poro")
    g.apply(("move", (p.uid,), 0))
    settle(g)
    assert len(named(g, 0, "Gold")) == 1 and named(g, 0, "Gold")[0].exhausted


@T.test
def ravenbloom_student_grows():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    r = put(g, 0, "Ravenbloom Student")
    put(g, 1, "Mega-Mech")
    hand(g, 0, "Smoke Screen")
    play(g, 0, "Smoke Screen")
    assert g.might(r) == 3


@T.test
def renata_abilities_at_battlefield():
    g, _ = new()
    runes(g, 0, ["Mind"] * 10)
    r = put(g, 0, "Renata Glasc, Mastermind")
    assert not act_options(g, 0, "Renata Glasc, Mastermind")
    g.move([r], 0, 0); settle(g)
    pts = g.p[0].points
    act(g, 0, "Renata Glasc, Mastermind", lambda a: IMPL["Renata Glasc, Mastermind"].abilities[a[2]]["name"] == "Draw")
    assert len(g.p[0].hand) == 1
    act(g, 0, "Renata Glasc, Mastermind", lambda a: IMPL["Renata Glasc, Mastermind"].abilities[a[2]]["name"] == "Score")
    assert g.p[0].points == pts + 1 and r.exhausted


@T.test
def riptide_rex_deals_six():
    g, _ = new()
    runes(g, 0, ["Mind"] * 8)
    e = put(g, 1, "Mega-Mech", 1)
    b = put(g, 1, "Pouty Poro")                     # at base: not a choice
    hand(g, 0, "Riptide Rex")
    play(g, 0, "Riptide Rex", lambda ch: ch["loc"] == "base")
    assert e.zone == "board" and b.damage == 0 and e.damage == 6


@T.test
def rumble_mech_aura_and_hold():
    g, _ = new()
    r = put(g, 0, "Rumble, Scrapper", 0)
    m = put(g, 0, "Mega-Mech")
    s = put(g, 0, "Stalwart Poro")
    assert g.might(r) == 5 and g.might(m) == 9 and g.might(s) == 2
    end_turns(g, 2)
    mechs = [u for u in named(g, 0, "Mech") if u.token]
    assert len(mechs) == 1 and mechs[0].loc == "base" and g.might(mechs[0]) == 4


@T.test
def sky_cruiser_discards_gear_to_strike():
    g, _ = new()
    runes(g, 0, ["Mind"])
    put(g, 0, "Sky Cruiser")
    e = put(g, 1, "Pouty Poro", 1)
    gear = hand(g, 0, "Long Sword")
    act(g, 0, "Sky Cruiser")
    assert gear in g.p[0].trash and e.zone == "trash"


@T.test
def soul_shepherd_tokens():
    g, _ = new()
    put(g, 0, "Soul Shepherd")
    from cards import make_token
    t = make_token(g, "Recruit", 0)
    t2 = make_token(g, "Recruit", 1)
    u = put(g, 0, "Stalwart Poro")
    assert g.might(t) == 2 and g.might(t2) == 1 and g.might(u) == 2


@T.test
def spectral_centaur_grows():
    g, _ = new()
    c = put(g, 0, "Spectral Centaur")
    u = put(g, 0, "Stalwart Poro")
    e = put(g, 1, "Pouty Poro")
    g.kill([u]); settle(g)
    g.kill([e]); settle(g)
    assert g.might(c) == 7


@T.test
def sprite_mother_and_queen():
    g, _ = new()
    runes(g, 0, ["Mind"] * 12)
    put(g, 0, "Stalwart Poro", 0)
    hand(g, 0, "Sprite Mother")
    play(g, 0, "Sprite Mother", lambda ch: ch["loc"] == 0)
    sp = named(g, 0, "Sprite")
    assert len(sp) == 1 and sp[0].loc == 0 and not sp[0].exhausted
    hand(g, 0, "Sprite Queen")
    play(g, 0, "Sprite Queen", lambda ch: ch["loc"] == 0)
    sp = named(g, 0, "Sprite")
    assert len(sp) == 2 and sp[1].loc == "base"
    end_turns(g, 2)                                 # old Sprites die (Temporary), Queen makes a new one
    sp = named(g, 0, "Sprite")
    assert len(sp) == 1 and sp[0].loc == "base", sp


@T.test
def swain_scores_with_unit_gear_spell():
    g, _ = new()
    runes(g, 0, ["Mind"] * 12)
    s = put(g, 0, "Swain, Visionary")
    hand(g, 0, "Chemtech Cask"); hand(g, 0, "Smoke Screen"); hand(g, 0, "Stalwart Poro")
    e = put(g, 1, "Watchful Sentry", 1)
    play(g, 0, "Chemtech Cask")
    play(g, 0, "Smoke Screen")
    play(g, 0, "Stalwart Poro", lambda ch: ch["loc"] == "base")
    pts = g.p[0].points
    g.apply(("move", (s.uid,), 1))
    settle(g)
    assert g.bfs[1].ctrl == 0 and g.p[0].points == pts + 2


@T.test
def swain_no_score_without_gear():
    g, _ = new()
    s = put(g, 0, "Swain, Visionary")
    g.apply(("move", (s.uid,), 1))
    settle(g)
    assert g.p[0].points == 1


@T.test
def teemo_defends_reveals_hidden():
    g, _ = new(tp=1)
    t = put(g, 0, "Teemo, Strategist", 1)
    e = put(g, 1, "Mega-Mech")
    top = deck_top(g, 0, ["Back Off", "Block", "Discipline", "Hidden Blade", "Stalwart Poro"])
    g.apply(("move", (e.uid,), 1))
    settle(g)
    assert all(c in g.p[0].deck[-5:] for c in top)
    assert any("Mega-Mech" in l and "takes 3" in l for l in g.lines), [l for l in g.lines if "takes" in l]


@T.test
def viktor_recruit_on_opponents_turn():
    g, _ = new(tp=1)
    runes(g, 0, ["Mind"] * 3)
    put(g, 0, "Viktor, Innovator")
    u = put(g, 0, "Stalwart Poro")
    hand(g, 0, "Smoke Screen")
    e = put(g, 1, "Mega-Mech")
    put(g, 1, "Stalwart Poro")
    g.apply(("move", (e.uid,), 1))
    d = g.advance()
    # P0 reacts during the showdown / combat with a Reaction spell
    for _ in range(10):
        if d.player == 0 and any(o[0] == "play" for o in d.options):
            break
        g.apply(("pass",)); d = g.advance()
    o = [x for x in d.options if x[0] == "play"]
    assert o
    g.apply(o[0])
    settle(g)
    assert len([x for x in named(g, 0, "Recruit") if x.token]) == 1


# ------------------------------------------------------------------ chunk 3: gear
from cards import make_token                            # noqa: E402


@T.test
def bottled_constellation_scores():
    g, _ = new()
    put(g, 0, "Bottled Constellation")
    for _ in range(3):
        make_token(g, "Recruit", 0)
    pts = g.p[0].points
    end_turns(g, 2)
    assert g.p[0].points == pts + 1 and not named(g, 0, "Recruit")


@T.test
def bottled_constellation_needs_three():
    g, _ = new()
    put(g, 0, "Bottled Constellation")
    make_token(g, "Recruit", 0)
    make_token(g, "Recruit", 0)
    pts = g.p[0].points
    end_turns(g, 2)
    assert g.p[0].points == pts and len(named(g, 0, "Recruit")) == 2


@T.test
def chemtech_cask_gold_on_opponents_turn():
    g, _ = new(tp=1)
    runes(g, 0, ["Mind"] * 2)
    cask = put(g, 0, "Chemtech Cask")
    e = put(g, 1, "Mega-Mech")
    c = hand(g, 0, "Smoke Screen")
    play_card(g, 0, c, "hand", dict(tg=(e.uid,)))
    settle(g)
    assert cask.exhausted and len(named(g, 0, "Gold")) == 1


@T.test
def chemtech_cask_not_on_own_turn():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2)
    put(g, 0, "Chemtech Cask")
    put(g, 1, "Mega-Mech")
    hand(g, 0, "Smoke Screen")
    play(g, 0, "Smoke Screen")
    assert not named(g, 0, "Gold")


@T.test
def cloth_armor_quick_draw_shield():
    g, _ = new()
    runes(g, 0, ["Mind"])
    u = put(g, 0, "Stalwart Poro")
    hand(g, 0, "Cloth Armor")
    play(g, 0, "Cloth Armor")
    arm = named(g, 0, "Cloth Armor")[0]
    assert arm.attached_to == u.uid and g.might(u) == 2
    u.desig = "def"
    assert g.might(u) == 2 + 1 + 2


@T.test
def energy_conduit_adds_energy():
    g, _ = new()
    runes(g, 0, ["Mind"])
    put(g, 0, "Energy Conduit")
    assert g.can_pay(0, 2, []) and not g.can_pay(0, 3, [])


@T.test
def garbage_grabber_recycles_and_draws():
    g, _ = new()
    runes(g, 0, ["Mind"])
    gg = put(g, 0, "Garbage Grabber")
    for n in ("Discipline", "Back Off"):
        trash(g, 0, n)
    assert not act_options(g, 0, "Garbage Grabber")
    trash(g, 0, "Long Sword")
    act(g, 0, "Garbage Grabber")
    assert not g.p[0].trash and len(g.p[0].hand) == 1 and gg.exhausted


@T.test
def gutter_palace_bird_and_win():
    g, _ = new()
    pal = put(g, 0, "Gutter Palace")
    hand(g, 0, "Discipline")
    act(g, 0, "Gutter Palace")
    b = named(g, 0, "Bird")
    assert len(b) == 1 and g.has_kw(b[0], "Deflect") and not g.p[0].hand
    g, _ = new()
    put(g, 0, "Gutter Palace")
    for i in range(4):
        put(g, 0, "Stalwart Poro", i % 2)
    for i in range(3):
        hand(g, 0, "Discipline")
    g.p[0].deck = g.p[0].deck[:30]
    g.apply(("end",)); settle(g)                    # P1's turn: P0 draws nothing
    d = None
    g.apply(("end",))
    d = settle(g)
    assert g.winner is None                         # only 3 cards in hand
    g2, _ = new()
    put(g2, 0, "Gutter Palace")
    for i in range(4):
        put(g2, 0, "Stalwart Poro", i % 2)
    for i in range(4):
        hand(g2, 0, "Discipline")
    g2.apply(("end",)); settle(g2)
    g2.apply(("end",)); settle(g2)
    assert g2.winner == 0


@T.test
def hextech_formula_empowers_gear():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2)
    hand(g, 0, "Hextech Formula")
    play(g, 0, "Hextech Formula")
    f = named(g, 0, "Hextech Formula")[0]
    assert f.exhausted
    f.exhausted = False
    t = put(g, 0, "Questionable Tome")
    act(g, 0, "Hextech Formula")
    assert t.empowered and f.exhausted


@T.test
def mushroom_pouch_draws_with_facedown():
    g, _ = new()
    runes(g, 0, ["Mind"])
    put(g, 0, "Mushroom Pouch")
    put(g, 0, "Stalwart Poro", 0)
    c = hand(g, 0, "Consult the Past")
    g.apply(("hide", c.uid, 0)); settle(g)
    end_turns(g, 2)
    assert len(g.p[0].hand) == 2                    # turn draw + Pouch


@T.test
def orb_of_regret_minimum_one():
    g, _ = new()
    put(g, 0, "Orb of Regret")
    e = put(g, 1, "Mega-Mech")
    act(g, 0, "Orb of Regret")
    assert g.might(e) == 7


@T.test
def questionable_tome_cycle():
    g, _ = new()
    runes(g, 0, ["Mind"])
    t = put(g, 0, "Questionable Tome")
    acts = act_options(g, 0, "Questionable Tome")
    assert len(acts) == 1                           # only Empower
    act(g, 0, "Questionable Tome")
    assert t.empowered and t.exhausted
    t.exhausted = False
    act(g, 0, "Questionable Tome")
    assert not t.empowered and t.exhausted and len(g.p[0].hand) == 1


@T.test
def seal_of_insight_adds_mind():
    g, _ = new()
    runes(g, 0, ["Fury"])
    put(g, 0, "Seal of Insight")
    assert g.can_pay(0, 1, [frozenset({"Mind"})])
    assert not g.can_pay(0, 0, [frozenset({"Mind"}), frozenset({"Mind"})])


@T.test
def sprite_fountain_play_and_deathknell():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    hand(g, 0, "Sprite Fountain")
    play(g, 0, "Sprite Fountain")
    sp = named(g, 0, "Sprite")
    assert len(sp) == 1 and not sp[0].exhausted
    end_turns(g, 2)                                 # Temporary: Fountain and Sprite die, Deathknell: new Sprite
    sp = named(g, 0, "Sprite")
    assert not named(g, 0, "Sprite Fountain") and len(sp) == 1 and sp[0].entered_turn == g.turn_no


@T.test
def sumpworks_map_draws_when_opponent_scores():
    g, _ = new(tp=1)
    put(g, 0, "Sumpworks Map")
    e = put(g, 1, "Mega-Mech")
    g.apply(("move", (e.uid,), 0))
    settle(g)
    assert g.bfs[0].ctrl == 1 and len(g.p[0].hand) == 1


@T.test
def zero_drive_banishes_and_replays():
    g, _ = new()
    runes(g, 0, ["Mind"] * 6)
    z = put(g, 0, "The Zero Drive")
    u = put(g, 0, "Cloud Drake")
    act(g, 0, "The Zero Drive", lambda a: IMPL["The Zero Drive"].abilities[a[2]]["name"] == "Equip")
    assert z.attached_to == u.uid and g.might(u) == 7
    assert not [a for a in act_options(g, 0, "The Zero Drive") if IMPL["The Zero Drive"].abilities[a[2]]["name"] == "Release"]
    g.kill([u]); settle(g)
    assert u in g.p[0].banish and z.attached_to is None
    runes(g, 0, ["Mind"] * 4)
    act(g, 0, "The Zero Drive", lambda a: IMPL["The Zero Drive"].abilities[a[2]]["name"] == "Release")
    assert u.zone == "board" and z in g.p[0].banish and len(g.p[0].hand) == 1    # Cloud Drake drew 1


@T.test
def world_atlas_hold_golds():
    g, _ = new()
    a = put(g, 0, "World Atlas")
    u = put(g, 0, "Stalwart Poro", 0)
    attach(g, a, u)
    assert g.might(u) == 4
    end_turns(g, 2)
    assert len(named(g, 0, "Gold")) == 2


# ------------------------------------------------------------------ chunk 3: spells
@T.test
def acceleration_gate_readies():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2 + ["Body"] * 2)
    u = put(g, 0, "Stalwart Poro", ready=False)
    hand(g, 0, "Acceleration Gate")
    play(g, 0, "Acceleration Gate", lambda ch: u.uid in ch["tg"])
    assert not u.exhausted
    assert sum(1 for r in g.p[0].runes if not r.exhausted) >= 2


@T.test
def arcane_shift_replays_and_deals():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2 + ["Chaos"] * 2)
    u = put(g, 0, "Cloud Drake", ready=False)
    e = put(g, 1, "Stalwart Poro", 1)
    hand(g, 0, "Arcane Shift")
    play(g, 0, "Arcane Shift")
    assert u.zone == "board" and e.zone == "trash" and len(g.p[0].hand) == 1
    assert named(g, 0, "Cloud Drake")[0].entered_turn == g.turn_no
    assert any(c.cname == "Arcane Shift" for c in g.p[0].banish)


@T.test
def draw_spells():
    g, _ = new()
    runes(g, 0, ["Mind"] * 20)
    for n in ("Premonition", "Progress Day", "Consult the Past"):
        hand(g, 0, n)
    play(g, 0, "Premonition")
    assert len(g.p[0].hand) == 2 + 3
    play(g, 0, "Progress Day")
    assert len(g.p[0].hand) == 1 + 3 + 4
    play(g, 0, "Consult the Past")
    assert len(g.p[0].hand) == 7 + 2


@T.test
def clairvoyance_predicts_then_draws():
    g, _ = new(answers0={"predict_recycle": lambda o, ctx: ctx["card"].cname != "Discipline"})
    runes(g, 0, ["Mind"] * 7)
    top = deck_top(g, 0, ["Back Off", "Discipline", "Block", "Hidden Blade", "Discipline"])
    hand(g, 0, "Clairvoyance")
    play(g, 0, "Clairvoyance")
    assert [c.cname for c in g.p[0].hand] == ["Discipline", "Discipline"]


@T.test
def consult_the_past_from_hidden():
    g, _ = new()
    runes(g, 0, ["Mind"])
    put(g, 0, "Stalwart Poro", 0)
    c = hand(g, 0, "Consult the Past")
    g.apply(("hide", c.uid, 0)); settle(g)
    end_turns(g, 2)
    n = len(g.p[0].hand)
    play(g, 0, "Consult the Past", src="facedown")
    assert len(g.p[0].hand) == n + 2


@T.test
def convergent_mutation_matches_might():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    a = put(g, 0, "Stalwart Poro")
    b = put(g, 0, "Mega-Mech")
    hand(g, 0, "Convergent Mutation")
    play(g, 0, "Convergent Mutation", lambda ch: ch["tg"] == (a.uid, b.uid))
    assert g.might(a) == 8


@T.test
def crescent_strike_splash():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    a = put(g, 1, "Mega-Mech", 1)
    b = put(g, 1, "Watchful Sentry", 1)
    c = put(g, 1, "Pouty Poro", 0)
    hand(g, 0, "Crescent Strike")
    play(g, 0, "Crescent Strike", lambda ch: ch["tg"] == (a.uid,))
    assert a.damage == 4 and b.zone == "trash" and c.damage == 0


@T.test
def deadly_flourish_gold_when_it_dies():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    e = put(g, 1, "Pouty Poro")
    hand(g, 0, "Deadly Flourish")
    play(g, 0, "Deadly Flourish")
    assert e.zone == "trash" and len(named(g, 0, "Gold")) == 1


@T.test
def deadly_flourish_later_death_this_turn():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    e = put(g, 1, "Mega-Mech")
    other = put(g, 1, "Pouty Poro")
    hand(g, 0, "Deadly Flourish")
    play(g, 0, "Deadly Flourish", lambda ch: ch["tg"] == (e.uid,))
    assert not named(g, 0, "Gold")
    g.kill([other]); settle(g)
    assert not named(g, 0, "Gold")
    g.kill([e]); settle(g)
    assert len(named(g, 0, "Gold")) == 1


@T.test
def downstage_dramatics_repeat():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    hand(g, 0, "Downstage Dramatics")
    play(g, 0, "Downstage Dramatics", lambda ch: ch.get("rep"))
    assert len(g.p[0].hand) == 2


@T.test
def dredge_up_flow_from_trash():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2)
    c = trash(g, 0, "Dredge Up")
    play(g, 0, "Dredge Up", src="trash")
    assert len(g.p[0].hand) == 1 and c in g.p[0].banish


@T.test
def eclipse_minus_four_and_predict():
    g, _ = new(answers0={"predict_recycle": True})
    runes(g, 0, ["Mind"] * 3)
    e = put(g, 1, "Mega-Mech")
    top = deck_top(g, 0, ["Discipline"])
    hand(g, 0, "Eclipse")
    play(g, 0, "Eclipse")
    assert g.might(e) == 4 and g.p[0].deck[-1] is top[0]


@T.test
def comet_and_spark():
    g, _ = new()
    runes(g, 0, ["Mind"] * 7 + ["Order"] * 6)
    e = put(g, 1, "Mega-Mech", 1)
    b = put(g, 1, "Mega-Mech")
    hand(g, 0, "Falling Comet")
    hand(g, 0, "Final Spark")
    play(g, 0, "Falling Comet")
    assert e.damage == 6 and b.damage == 0
    play(g, 0, "Final Spark", lambda ch: ch["tg"] == (b.uid,))
    assert b.zone == "trash"


@T.test
def frigid_touch_repeat():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    e = put(g, 1, "Mega-Mech")
    hand(g, 0, "Frigid Touch")
    play(g, 0, "Frigid Touch", lambda ch: ch.get("rep") and ch["tg"] == (e.uid,) and ch["tg2"] == (e.uid,))
    assert g.might(e) == 4


@T.test
def hostile_takeover_steals_until_end_of_turn():
    g, _ = new()
    runes(g, 0, ["Mind"] * 5 + ["Order"] * 2)
    e = put(g, 1, "Mega-Mech", 1, ready=False)
    hand(g, 0, "Hostile Takeover")
    pts = g.p[0].points
    play(g, 0, "Hostile Takeover")
    assert e.ctrl == 0 and not e.exhausted and g.bfs[1].ctrl == 0 and g.p[0].points == pts + 1
    g.apply(("end",)); settle(g)
    assert e.ctrl == 1 and e.loc == "base"


@T.test
def iterative_design_mech_and_flow():
    g, _ = new()
    runes(g, 0, ["Mind"] * 7)
    hand(g, 0, "Iterative Design")
    play(g, 0, "Iterative Design")
    assert len([u for u in named(g, 0, "Mech") if u.token]) == 1
    play(g, 0, "Iterative Design", src="trash")
    assert len([u for u in named(g, 0, "Mech") if u.token]) == 2 and any(c.cname == "Iterative Design" for c in g.p[0].banish)


@T.test
def mesmerize_modes():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    e = put(g, 1, "Mega-Mech")
    u = put(g, 0, "Stalwart Poro")
    hand(g, 0, "Mesmerize"); hand(g, 0, "Mesmerize")
    play(g, 0, "Mesmerize", lambda ch: ch["mode"] == "minus")
    assert g.might(e) == 6
    play(g, 0, "Mesmerize", lambda ch: ch["mode"] == "return")
    assert u in g.p[0].hand


@T.test
def moonfall_pulls_and_shrinks():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2 + ["Chaos"] * 2)
    put(g, 0, "Stalwart Poro", 0)
    a = put(g, 1, "Mega-Mech")
    b = put(g, 1, "Pouty Poro")
    hand(g, 0, "Moonfall")
    play(g, 0, "Moonfall", lambda ch: ch["bf"] == 0 and ch["tg"] == (a.uid,))
    assert a.loc == 0 and g.might(a) == 6 and g.might(b) == 2


@T.test
def moonlight_affliction_and_smoke_screen():
    g, _ = new()
    runes(g, 0, ["Mind"] * 10)
    e = put(g, 1, "Mountain Drake")
    f = put(g, 1, "Mega-Mech")
    hand(g, 0, "Moonlight Affliction"); hand(g, 0, "Smoke Screen")
    play(g, 0, "Moonlight Affliction", lambda ch: ch["tg"] == (e.uid,))
    assert g.might(e) == 0
    play(g, 0, "Smoke Screen", lambda ch: ch["tg"] == (f.uid,))
    assert g.might(f) == 4


@T.test
def portal_rescue_replays_to_base():
    g, _ = new()
    runes(g, 0, ["Mind"] * 4)
    u = put(g, 0, "Cloud Drake", 1, ready=False)
    u.damage = 3
    hand(g, 0, "Portal Rescue")
    play(g, 0, "Portal Rescue")
    assert u.zone == "board" and u.loc == "base" and u.damage == 0 and len(g.p[0].hand) == 1


@T.test
def production_surge_discount_and_mech():
    g, _ = new()
    c = hand(g, 0, "Production Surge")
    assert total_cost(g, 0, c, {}, "hand")[0] == 4
    put(g, 0, "Forecaster")
    assert total_cost(g, 0, c, {}, "hand")[0] == 2
    runes(g, 0, ["Mind"] * 3)
    play(g, 0, "Production Surge")
    assert len([u for u in named(g, 0, "Mech") if u.token and u.loc == "base"]) == 1 and len(g.p[0].hand) == 1


@T.test
def promising_future_both_play():
    g, _ = new()
    runes(g, 0, ["Mind"] * 6)
    runes(g, 1, ["Order"] * 2)
    deck_top(g, 0, ["Mega-Mech", "Discipline", "Back Off", "Block", "Stalwart Poro"])
    deck_top(g, 1, ["Glasc Mixologist", "Watchful Sentry", "Soaring Scout", "Hidden Blade", "Cull the Weak"])
    hand(g, 0, "Promising Future")
    play(g, 0, "Promising Future")
    assert named(g, 0, "Mega-Mech") and named(g, 1, "Glasc Mixologist")
    assert len(g.p[1].runes) == 1                   # paid [Order] for Mixologist's Power cost


@T.test
def retreat_returns_and_channels():
    g, _ = new()
    runes(g, 0, ["Mind"])
    u = put(g, 0, "Stalwart Poro", 1)
    hand(g, 0, "Retreat")
    n = len(g.p[0].runes)
    play(g, 0, "Retreat")
    assert u in g.p[0].hand and len(g.p[0].runes) == n + 1 and g.p[0].runes[-1].exhausted


@T.test
def rocket_barrage_both_modes():
    g, _ = new()
    runes(g, 0, ["Mind"] * 10)
    e = put(g, 1, "Stalwart Poro")
    x = put(g, 1, "Long Sword")
    hand(g, 0, "Rocket Barrage")
    play(g, 0, "Rocket Barrage", lambda ch: ch.get("rep") and ch["tg"] == (x.uid,) and ch["tg2"] == (e.uid,))
    assert x.zone == "trash" and e.zone == "trash"


@T.test
def shock_blast_discount():
    g, _ = new()
    c = hand(g, 0, "Shock Blast")
    assert total_cost(g, 0, c, {}, "hand")[0] == 3
    t = put(g, 0, "Questionable Tome")
    t.empowered = True
    assert total_cost(g, 0, c, {}, "hand")[0] == 1
    runes(g, 0, ["Mind"] * 2)
    e = put(g, 1, "Pouty Poro", 1)
    play(g, 0, "Shock Blast")
    assert e.zone == "trash"


@T.test
def singularity_two_units():
    g, _ = new()
    runes(g, 0, ["Mind"] * 8)
    a = put(g, 1, "Mega-Mech")
    b = put(g, 1, "Stalwart Poro", 1)
    hand(g, 0, "Singularity")
    play(g, 0, "Singularity", lambda ch: set(ch["tg"]) == {a.uid, b.uid})
    assert a.damage == 6 and b.zone == "trash"


@T.test
def siphon_power_battlefield():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2 + ["Order"])
    u = put(g, 0, "Stalwart Poro", 1, bf_control=False)
    e = put(g, 1, "Mega-Mech", 1)
    w = put(g, 1, "Watchful Sentry", 1)
    hand(g, 0, "Siphon Power")
    g.apply(opt(g, 0, "Siphon Power", lambda ch: ch["bf"] == 1))
    for _ in range(20):
        d = g.advance()
        if not g.chain:
            break
        g.apply(("pass",))
    assert g.might(u) == 3 and g.might(e) == 7 and g.might(w) == 1


@T.test
def smoke_and_mirrors_swaps_with_temporary():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2)
    a = put(g, 0, "Stalwart Poro", 0)
    t = make_token(g, "Sprite", 0, "base")
    hand(g, 0, "Smoke and Mirrors")
    play(g, 0, "Smoke and Mirrors", lambda ch: ch["tg"] == (a.uid, t.uid))
    assert a.loc == "base" and t.loc == 0 and len(g.p[0].hand) == 1


@T.test
def smoke_and_mirrors_no_temporary_only_draws():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2)
    a = put(g, 0, "Stalwart Poro", 0)
    b = put(g, 0, "Pouty Poro")
    hand(g, 0, "Smoke and Mirrors")
    play(g, 0, "Smoke and Mirrors")
    assert a.loc == 0 and b.loc == "base" and len(g.p[0].hand) == 1


@T.test
def sprite_burst_and_call():
    g, _ = new()
    runes(g, 0, ["Mind"] * 8)
    hand(g, 0, "Sprite Burst"); hand(g, 0, "Sprite Call")
    play(g, 0, "Sprite Burst")
    play(g, 0, "Sprite Call")
    sp = named(g, 0, "Sprite")
    assert len(sp) == 3 and not any(x.exhausted for x in sp)


@T.test
def sprite_call_from_hidden_at_battlefield():
    g, _ = new()
    runes(g, 0, ["Mind"])
    put(g, 0, "Stalwart Poro", 0)
    c = hand(g, 0, "Sprite Call")
    g.apply(("hide", c.uid, 0)); settle(g)
    end_turns(g, 2)
    play(g, 0, "Sprite Call", src="facedown")
    sp = named(g, 0, "Sprite")
    assert len(sp) == 1 and sp[0].loc == 0


@T.test
def temporal_breach_replays_same_location():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    u = put(g, 0, "Cloud Drake", 1, ready=False)
    u.damage = 2
    hand(g, 0, "Temporal Breach")
    play(g, 0, "Temporal Breach", lambda ch: ch["tg"] == (u.uid,))
    assert u.zone == "board" and u.loc == 1 and u.damage == 0 and len(g.p[0].hand) == 1


@T.test
def unchecked_power_wipes_battlefields():
    g, _ = new()
    runes(g, 0, ["Mind"] * 9)
    a = put(g, 0, "Stalwart Poro")
    b = put(g, 0, "Mega-Mech", 0)
    e = put(g, 1, "Mountain Drake", 1)
    f = put(g, 1, "Pouty Poro")
    hand(g, 0, "Unchecked Power")
    play(g, 0, "Unchecked Power")
    assert a.exhausted and a.zone == "board" and b.zone == "trash" and e.zone == "trash" and f.zone == "board"


@T.test
def wages_of_pain_damage_and_gold():
    g, _ = new()
    runes(g, 0, ["Mind"] * 3)
    e = put(g, 1, "Stalwart Poro", 1)
    hand(g, 0, "Wages of Pain")
    play(g, 0, "Wages of Pain")
    assert e.zone == "trash" and len(named(g, 0, "Gold")) == 1


# ------------------------------------------------------------------ rule interactions
@T.test
def ekko_with_karthus_recycles_once():
    g, _ = new()
    runes(g, 0, ["Mind"] * 2)
    for r in g.p[0].runes:
        r.exhausted = True
    put(g, 0, "Karthus, Eternal")
    e = put(g, 0, "Ekko, Recurrent")
    n = len(g.p[0].deck)
    g.kill([e]); settle(g)
    assert len(g.p[0].deck) == n + 1 and not any(r.exhausted for r in g.p[0].runes)


@T.test
def hostile_takeover_starts_combat():
    g, _ = new()
    runes(g, 0, ["Mind"] * 5 + ["Order"] * 2)
    big = put(g, 1, "Mega-Mech", 1)
    small = put(g, 1, "Stalwart Poro", 1)
    hand(g, 0, "Hostile Takeover")
    play(g, 0, "Hostile Takeover", lambda ch: ch["tg"] == (big.uid,))
    assert small.zone == "trash" and big.ctrl == 0 and g.bfs[1].ctrl == 0
    g.apply(("end",)); settle(g)
    assert big.ctrl == 1 and big.loc == "base" and g.bfs[1].ctrl is None


@T.test
def teemo_played_from_hidden():
    g, _ = new()
    runes(g, 0, ["Mind"])
    put(g, 0, "Stalwart Poro", 0)
    c = hand(g, 0, "Teemo, Strategist")
    g.apply(("hide", c.uid, 0)); settle(g)
    end_turns(g, 2)
    e = put(g, 1, "Mega-Mech", 0, bf_control=False)
    deck_top(g, 0, ["Back Off", "Block", "Discipline", "Hidden Blade", "Stalwart Poro"])
    play_card(g, 0, c, "facedown", dict(loc=0, acc=False))
    settle(g)
    assert any("Mega-Mech" in l and "takes 4" in l for l in g.lines)      # 3 + Void Gate bonus


@T.test
def sumpworks_map_ignores_burn_out():
    g, _ = new()
    put(g, 0, "Sumpworks Map")
    g.p[0].deck = []
    g.p[0].trash = []
    g.burn_out(0)
    settle(g)
    assert g.p[1].points == 1 and not g.p[0].hand


@T.test
def needs_cards_not_registered():
    for n in ("Applied Researchers", "Eager Apprentice", "Jayce, Man of Progress", "Temporal Portal",
              "Jhin, Meticulous Killer", "Guerilla Warfare", "Frigid Jewel", "Wraith of Echoes", "Ezreal, Dashing",
              "Otterpus", "Blue Sentinel", "Zilean, Time Mage", "Prize of Progress", "Rebuttal",
              "Kai'Sa, Evolutionary", "Mel, Newly Awakened", "Hextech Anomaly", "Malzahar, Fanatic",
              "Experimental Hexplate", "Heimerdinger, Inventor"):
        assert n not in IMPL, n
    names = [l.strip() for l in open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "batches",
                                                  "mind.txt")) if l.strip()]
    assert sum(1 for n in names if n in IMPL and IMPL[n].module == "cardsets.mind") == 100

if __name__ == "__main__":
    T.main()
