"""Tests of batch legends. Run: RB_CARDSETS=legends python3 cardsets/test_legends.py [fuzz games per legend]"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from game import Showdown, Item, ANY                     # noqa: E402
from cards import make_token, attach                    # noqa: E402

T = Suite("legends")


def leg(g, pid, name):
    """Make name the legend of pid."""
    g.p[pid].legend_name = name
    o = Obj(name, pid)
    o.zone = "legend"
    g.p[pid].legend = o
    return o


def act(g, pid, ab_name, pred=lambda ch: True):
    """Activate the legend ability ab_name of pid (first choice matching pred)."""
    im = IMPL[g.p[pid].legend_name]
    for o in options_of(g, "act"):
        if o[1] == ("legend", pid) and im.abilities[o[2]]["name"] == ab_name and pred(o[3]):
            g.apply(o)
            return o
    raise AssertionError(f"no option for {ab_name}")


def has_act(g, pid, ab_name):
    im = IMPL[g.p[pid].legend_name]
    return any(o[1] == ("legend", pid) and im.abilities[o[2]]["name"] == ab_name for o in options_of(g, "act"))


def attack(g, units, bf=1):
    g.apply(("move", tuple(u.uid for u in units), bf))
    settle(g)


# ------------------------------------------------------------------ Ahri
@T.test
def ahri_enemy_attacker_gets_minus_1_min_1():
    g, _ = new()
    leg(g, 1, "Ahri, Nine-Tailed Fox")
    a = put(g, 0, "Glasc Mixologist")                     # 5
    a2 = put(g, 0, "Soaring Scout")                       # 1: minimum 1
    put(g, 1, "Soaring Scout", 1)
    attack(g, [a, a2])
    assert g.might(a) == 4 and g.might(a2) == 1, (g.might(a), g.might(a2))
    # Ahri's own units attacking don't trigger it
    g2, _ = new()
    leg(g2, 0, "Ahri, Nine-Tailed Fox")
    b = put(g2, 0, "Glasc Mixologist")
    put(g2, 1, "Soaring Scout", 1)
    attack(g2, [b])
    assert g2.might(b) == 5


# ------------------------------------------------------------------ Annie
@T.test
def annie_readies_2_runes_at_end_of_turn():
    g, _ = new()
    leg(g, 0, "Annie, Dark Child")
    runes(g, 0, ["Fury"] * 4)
    for r in g.p[0].runes:
        r.exhausted = True
    g.apply(("end",))
    settle(g)
    assert g.tp == 1
    assert sum(1 for r in g.p[0].runes if not r.exhausted) == 2


# ------------------------------------------------------------------ Azir
@T.test
def azir_sand_soldier_after_equipment_with_weaponmaster():
    g, _ = new()
    leg(g, 0, "Azir, Emperor of the Sands")
    runes(g, 0, ["Fury"] * 6)
    u = put(g, 0, "Pouty Poro")
    assert not has_act(g, 0, "Sand Soldier")              # no Equipment played this turn
    hand(g, 0, "Long Sword")
    g.apply(opt(g, 0, "Long Sword"))
    settle(g)
    sword = [x for x in g.gear(0) if x.cname == "Long Sword"][0]
    assert sword.attached_to == u.uid
    act(g, 0, "Sand Soldier")
    settle(g)
    ss = [x for x in g.units(0) if x.cname == "Sand Soldier"]
    assert len(ss) == 1 and ss[0].loc == "base" and g.has_kw(ss[0], "Weaponmaster")
    assert sword.attached_to == ss[0].uid and g.might(ss[0]) == 4      # Weaponmaster moved the sword
    assert g.p[0].legend.exhausted
    assert not g.has_kw(u, "Weaponmaster")


# ------------------------------------------------------------------ Add legends
@T.test
def darius_add_energy_with_legion():
    g, _ = new()
    leg(g, 0, "Darius, Hand of Noxus")
    runes(g, 0, ["Fury"])
    assert not g.can_pay(0, 2, [])
    g.finalized[0].append("Pouty Poro")
    assert g.can_pay(0, 2, []) and g.pay(0, 2, [])
    assert g.p[0].legend.exhausted and not g.can_pay(0, 1, [])


@T.test
def diana_add_energy_only_in_showdown():
    g, _ = new()
    leg(g, 0, "Diana, Scorn of the Moon")
    runes(g, 0, ["Mind"])
    assert not g.can_pay(0, 2, [])
    g.sd = Showdown(1, False, 0)
    assert g.can_pay(0, 2, []) and g.pay(0, 2, []) and g.p[0].legend.exhausted


@T.test
def kaisa_add_rune_only_for_spells():
    g, _ = new()
    leg(g, 0, "Kai'Sa, Daughter of the Void")
    runes(g, 0, [])
    assert not g.can_pay(0, 0, [frozenset({"Fury"})], dict(kind="unit"))
    assert g.can_pay(0, 0, [frozenset({"Fury"})], dict(kind="spell"))
    runes(g, 0, ["Mind"])
    c = hand(g, 0, "Charm")                                # 1 energy + 1 Calm rune
    e = put(g, 1, "Pouty Poro", 1)
    g.apply(opt(g, 0, "Charm", lambda ch: ch["tg"] == (e.uid,)))
    assert g.p[0].legend.exhausted and c.zone == "chain"


@T.test
def ornn_add_rune_only_for_gear():
    g, _ = new()
    leg(g, 0, "Ornn, Fire Below the Mountain")
    runes(g, 0, [])
    gear = put(g, 0, "Long Sword")
    assert not g.can_pay(0, 0, [ANY], dict(kind="spell"))
    assert g.can_pay(0, 0, [ANY], dict(kind="gear"))
    assert g.can_pay(0, 0, [ANY], dict(kind="ability", obj=gear))
    assert not g.can_pay(0, 0, [ANY], dict(kind="ability", obj=g.p[0].legend))
    u = put(g, 0, "Pouty Poro")
    act_opts = [o for o in options_of(g, "act") if o[1] == gear.uid]
    assert act_opts                                        # Equip [Fury] paid with Ornn
    g.apply(act_opts[0])
    settle(g)
    assert gear.attached_to == u.uid and g.p[0].legend.exhausted


# ------------------------------------------------------------------ Draven
@T.test
def draven_draws_when_winning_combat():
    g, _ = new()
    leg(g, 0, "Draven, Glorious Executioner")
    a = put(g, 0, "Glasc Mixologist")
    put(g, 1, "Soaring Scout", 1)
    attack(g, [a])
    assert len(g.p[0].hand) == 1 and g.bfs[1].ctrl == 0


# ------------------------------------------------------------------ Ezreal
@T.test
def ezreal_needs_two_enemy_choices():
    g, _ = new()
    leg(g, 0, "Ezreal, Prodigal Explorer")
    runes(g, 0, ["Mind"] * 4)
    e1 = put(g, 1, "Pouty Poro", 1)
    e2 = put(g, 1, "Soaring Scout", 1)
    hand(g, 0, "Stupefy")
    g.apply(opt(g, 0, "Stupefy", lambda ch: ch["tg"] == (e1.uid,)))
    settle(g)
    assert not has_act(g, 0, "Draw")
    # one item choosing two enemy units counts once (per spell / ability)
    it = Item("spell", 0, "Falling Star")
    g.add_target(it, e1); g.add_target(it, e2)
    g.on_finalize(it)
    assert g.stats[("ezreal", 0, g.turn_no)] == 2
    # a legend ability is not a unit ability; friendly targets don't count
    it2 = Item("ability", 0, "x", src=g.p[0].legend.uid)
    g.add_target(it2, e1)
    g.on_finalize(it2)
    assert g.stats[("ezreal", 0, g.turn_no)] == 2
    n = len(g.p[0].hand)
    act(g, 0, "Draw")
    settle(g)
    assert len(g.p[0].hand) == n + 1 and g.p[0].legend.exhausted


# ------------------------------------------------------------------ Fiora
@T.test
def fiora_channels_when_unit_becomes_mighty():
    g, _ = new()
    leg(g, 0, "Fiora, Grand Duelist")
    runes(g, 0, [])
    u = put(g, 0, "Stellacorn Herder")                     # 3
    g.need_cleanup = True
    settle(g)
    g.mod(u, 2)
    g.need_cleanup = True
    settle(g)
    assert len(g.p[0].runes) == 1 and g.p[0].runes[0].exhausted and g.p[0].legend.exhausted
    g.mod(u, 2)                                            # already Mighty: no new trigger
    g.p[0].legend.exhausted = False
    g.need_cleanup = True
    settle(g)
    assert len(g.p[0].runes) == 1


# ------------------------------------------------------------------ Garen
@T.test
def garen_draws_2_on_conquer_with_4_units():
    g, _ = new()
    leg(g, 0, "Garen, Might of Demacia")
    us = [put(g, 0, "Pouty Poro") for _ in range(4)]
    attack(g, us)
    assert g.bfs[1].ctrl == 0 and len(g.p[0].hand) == 2
    g2, _ = new()
    leg(g2, 0, "Garen, Might of Demacia")
    us = [put(g2, 0, "Pouty Poro") for _ in range(3)]
    attack(g2, us)
    assert g2.bfs[1].ctrl == 0 and len(g2.p[0].hand) == 0


# ------------------------------------------------------------------ Irelia
@T.test
def irelia_readies_chosen_friendly_unit():
    g, _ = new()
    leg(g, 0, "Irelia, Blade Dancer")
    runes(g, 0, ["Calm"] * 4)
    u = put(g, 0, "Pouty Poro", ready=False)
    hand(g, 0, "Discipline")
    g.apply(opt(g, 0, "Discipline", lambda ch: ch["tg"] == (u.uid,)))
    settle(g)
    assert not u.exhausted and g.p[0].legend.exhausted and g.might(u) == 4
    assert len(g.p[0].runes) == 3                          # 1 rune of any type paid (recycled)


@T.test
def irelia_conquer_pays_1_to_ready_legend():
    g, _ = new()
    leg(g, 0, "Irelia, Blade Dancer").exhausted = True
    runes(g, 0, ["Calm"])
    attack(g, [put(g, 0, "Pouty Poro")])
    assert g.bfs[1].ctrl == 0 and not g.p[0].legend.exhausted and g.p[0].runes[0].exhausted


# ------------------------------------------------------------------ Jax
@T.test
def jax_attaches_detached_then_attached_equipment():
    g, _ = new()
    leg(g, 0, "Jax, Grandmaster At Arms")
    runes(g, 0, ["Calm"])
    sword = put(g, 0, "Long Sword")
    u1 = put(g, 0, "Pouty Poro", 1)
    act(g, 0, "Attach detached", lambda ch: ch["tg"] == (sword.uid, u1.uid))
    settle(g)
    assert sword.attached_to == u1.uid and sword.loc == 1 and g.might(u1) == 4
    assert not has_act(g, 0, "Attach attached")            # legend exhausted
    g.p[0].legend.exhausted = False
    u2 = put(g, 0, "Soaring Scout")
    act(g, 0, "Attach attached", lambda ch: ch["tg"] == (sword.uid, u2.uid))
    settle(g)
    assert sword.attached_to == u2.uid and sword.loc == "base" and g.might(u1) == 2 and g.might(u2) == 3


# ------------------------------------------------------------------ Jayce
@T.test
def jayce_ready_gear_and_empowered_ready_two():
    g, _ = new()
    leg(g, 0, "Jayce, Defender of Tomorrow")
    runes(g, 0, ["Mind"] * 6)
    g1 = make_token(g, "Gold", 0, "base", ready=False)
    g2 = make_token(g, "Gold", 0, "base", ready=False)
    assert not has_act(g, 0, "Ready 2 gear")
    act(g, 0, "Ready a gear", lambda ch: ch["tg"] == (g1.uid,))
    settle(g)
    assert not g1.exhausted and g2.exhausted
    g1.exhausted = True
    g.p[0].legend.exhausted = False
    act(g, 0, "Empower")
    settle(g)
    assert g.p[0].legend.empowered and not g.p[0].legend.exhausted
    act(g, 0, "Ready 2 gear")
    settle(g)
    assert not g1.exhausted and not g2.exhausted


# ------------------------------------------------------------------ Jinx
@T.test
def jinx_draws_at_beginning_with_small_hand():
    for n_hand, drawn in ((1, 1), (2, 0)):
        g, _ = new()
        leg(g, 0, "Jinx, Loose Cannon")
        for _ in range(n_hand):
            hand(g, 0, "Pouty Poro")
        g.emit("beginning_start", pid=0)
        settle(g)
        assert len(g.p[0].hand) == n_hand + drawn, (n_hand, len(g.p[0].hand))


# ------------------------------------------------------------------ Kennen
@T.test
def kennen_empowered_by_play_from_champion_zone():
    g, _ = new()
    leg(g, 0, "Kennen, Heart of the Tempest")
    runes(g, 0, ["Fury"] * 4)
    hand(g, 0, "Pouty Poro")
    g.apply(opt(g, 0, "Pouty Poro"))
    settle(g)
    assert not g.p[0].legend.empowered                     # from hand
    assert not has_act(g, 0, "Assault 2")
    champ(g, 0, "Pouty Poro")
    g.apply(opt(g, 0, "Pouty Poro", src="champ"))
    settle(g)
    assert g.p[0].legend.empowered
    u = g.units(0)[0]
    act(g, 0, "Assault 2", lambda ch: ch["tg"] == (u.uid,))
    settle(g)
    assert g.kw_value(u, "Assault") == 2 and not g.p[0].legend.empowered and g.p[0].legend.exhausted


# ------------------------------------------------------------------ Kha'Zix
@T.test
def khazix_xp_buff_and_move():
    g, _ = new()
    leg(g, 0, "Kha'Zix, Voidreaver")
    a = put(g, 0, "Glasc Mixologist")
    put(g, 1, "Soaring Scout", 1)
    attack(g, [a])
    assert g.p[0].xp == 1
    act(g, 0, "Buff", lambda ch: ch["tg"] == (a.uid,))
    settle(g)
    assert a.buff == 1 and g.p[0].xp == 0
    g.gain_xp(0, 2)
    g.p[0].legend.exhausted = False
    assert a.exhausted and a.loc == 1
    act(g, 0, "Move to base")
    settle(g)
    assert a.loc == "base" and g.p[0].xp == 0


# ------------------------------------------------------------------ Lee Sin
@T.test
def lee_sin_buffs_friendly_unit():
    g, _ = new()
    leg(g, 0, "Lee Sin, Blind Monk")
    runes(g, 0, ["Calm"])
    u = put(g, 0, "Pouty Poro")
    act(g, 0, "Buff")
    settle(g)
    assert u.buff == 1 and g.might(u) == 3 and g.p[0].legend.exhausted


# ------------------------------------------------------------------ Leona
@T.test
def leona_buffs_when_stunning_enemies_once_per_action():
    g, _ = new()
    leg(g, 0, "Leona, Radiant Dawn")
    runes(g, 0, ["Calm"] * 3)
    u = put(g, 0, "Pouty Poro")
    e = put(g, 1, "Soaring Scout", 1)
    hand(g, 0, "Back Off")
    g.apply(opt(g, 0, "Back Off", lambda ch: ch["tg"] == (e.uid,)))
    settle(g)
    assert e.stunned and u.buff == 1
    e2, e3 = put(g, 1, "Pouty Poro", 1), put(g, 1, "Pouty Poro", 1)
    g.stun(e2, 0); g.stun(e3, 0)
    assert sum(1 for t in g.trigq if t.name == "Leona, Radiant Dawn") == 1
    g.trigq = []
    g.stun(u, 0)                                           # own unit: no trigger
    assert not g.trigq


# ------------------------------------------------------------------ Lillia
@T.test
def lillia_sprite_cost_reduced_by_temporary_units():
    g, _ = new()
    leg(g, 0, "Lillia, Bashful Bloom")
    runes(g, 0, ["Calm"] * 3)
    assert not has_act(g, 0, "Sprite")                     # 4 energy
    make_token(g, "Sprite", 0, "base")                     # one friendly Temporary unit: costs 3
    act(g, 0, "Sprite")
    settle(g)
    sp = [u for u in g.units(0) if u.cname == "Sprite"]
    assert len(sp) == 2 and not sp[1].exhausted and g.has_kw(sp[1], "Temporary") and g.might(sp[1]) == 3
    assert all(r.exhausted for r in g.p[0].runes)


# ------------------------------------------------------------------ Lucian
@T.test
def lucian_each_equipment_gives_assault():
    g, _ = new()
    leg(g, 0, "Lucian, Purifier")
    u = put(g, 0, "Pouty Poro")
    assert g.kw_value(u, "Assault") == 0
    attach(g, put(g, 0, "Long Sword"), u)
    assert g.kw_value(u, "Assault") == 1
    attach(g, put(g, 0, "Sterak's Gage"), u)
    assert g.kw_value(u, "Assault") == 2
    u.desig = "att"
    assert g.might(u) == 2 + 2 + 3 + 2
    e = put(g, 1, "Pouty Poro")
    attach(g, put(g, 1, "Long Sword"), e)
    assert g.kw_value(e, "Assault") == 0                   # enemy Equipment


# ------------------------------------------------------------------ Lux
@T.test
def lux_draws_on_spell_costing_5_or_more():
    g, _ = new()
    leg(g, 0, "Lux, Lady of Luminosity")
    runes(g, 0, ["Mind"] * 14)
    hand(g, 0, "Stupefy")
    e = put(g, 1, "Pouty Poro", 1)
    g.apply(opt(g, 0, "Stupefy", lambda ch: ch["tg"] == (e.uid,)))
    settle(g)
    assert len(g.p[0].hand) == 1                           # Stupefy's own draw only
    hand(g, 0, "Time Warp")
    g.apply(opt(g, 0, "Time Warp"))
    settle(g)
    assert len(g.p[0].hand) == 2


# ------------------------------------------------------------------ Master Yi, Wuju Bladesman
@T.test
def master_yi_bladesman_defender_alone():
    g, _ = new()
    leg(g, 1, "Master Yi, Wuju Bladesman")
    d = put(g, 1, "Pouty Poro", 1)
    a = put(g, 0, "Stellacorn Herder")                     # 3 vs 2+2: attacker dies, Poro survives
    attack(g, [a])
    assert a.zone == "trash" and d.zone == "board" and g.bfs[1].ctrl == 1
    d.desig = "def"
    assert g.might(d) == 4
    d2 = put(g, 1, "Pouty Poro", 1)
    d2.desig = "def"
    assert g.might(d) == 2 and g.might(d2) == 2


# ------------------------------------------------------------------ Miss Fortune
@T.test
def miss_fortune_gives_ganking():
    g, _ = new()
    leg(g, 0, "Miss Fortune, Bounty Hunter")
    u = put(g, 0, "Pouty Poro", 0)
    act(g, 0, "Ganking", lambda ch: ch["tg"] == (u.uid,))
    settle(g)
    assert g.has_kw(u, "Ganking") and any(o[0] == "move" and o[2] == 1 for o in options_of(g, "move"))


# ------------------------------------------------------------------ Poppy
@T.test
def poppy_hold_xp_and_draw():
    g, _ = new()
    leg(g, 0, "Poppy, Keeper of the Hammer")
    put(g, 0, "Pouty Poro", 1)
    g.hold(0, g.bfs[1])
    settle(g)
    assert g.p[0].xp == 1
    assert not has_act(g, 0, "Draw")
    g.gain_xp(0, 2)
    act(g, 0, "Draw")
    settle(g)
    assert len(g.p[0].hand) == 1 and g.p[0].xp == 0


# ------------------------------------------------------------------ Pyke
@T.test
def pyke_returns_unit_and_plays_gold():
    g, _ = new()
    leg(g, 0, "Pyke, Bloodharbor Ripper")
    runes(g, 0, ["Fury"])
    u = put(g, 0, "Pouty Poro", 1)
    put(g, 0, "Soaring Scout")                             # at base: not a legal target
    act(g, 0, "Return")
    settle(g)
    assert u in g.p[0].hand and u.zone == "hand"
    golds = [x for x in g.gear(0) if x.cname == "Gold"]
    assert len(golds) == 1 and golds[0].exhausted


# ------------------------------------------------------------------ Rek'Sai
@T.test
def reksai_plays_one_of_top_two_and_recycles_rest():
    g, _ = new()
    leg(g, 0, "Rek'sai, Void Burrower")
    runes(g, 0, ["Fury"] * 3)
    top = deck_top(g, 0, ["Pouty Poro", "Stupefy"])
    attack(g, [put(g, 0, "Soaring Scout")])
    assert g.bfs[1].ctrl == 0 and g.p[0].legend.exhausted
    assert top[0].zone == "board" and top[1] is g.p[0].deck[-1]     # Stupefy (no target, Mind) recycled
    assert sum(1 for r in g.p[0].runes if r.exhausted) == 2


@T.test
def reksai_can_decline():
    g, _ = new(answers0={"reksai_pick": None})
    leg(g, 0, "Rek'sai, Void Burrower")
    runes(g, 0, ["Fury"] * 3)
    top = deck_top(g, 0, ["Pouty Poro", "Pouty Poro"])
    attack(g, [put(g, 0, "Soaring Scout")])
    assert all(c in g.p[0].deck[-2:] for c in top)


# ------------------------------------------------------------------ Rengar
@T.test
def rengar_gives_plus_1_when_playing_unit():
    g, _ = new()
    leg(g, 0, "Rengar, Pridestalker")
    runes(g, 0, ["Fury"] * 2)
    u = put(g, 0, "Stellacorn Herder", 0)
    put(g, 1, "Pouty Poro", 1)
    hand(g, 0, "Pouty Poro")
    g.apply(opt(g, 0, "Pouty Poro", lambda ch: ch["loc"] == "base"))
    settle(g)
    assert g.might(u) == 4


# ------------------------------------------------------------------ Rumble
@T.test
def rumble_mechs_have_shield():
    g, _ = new()
    leg(g, 0, "Rumble, Mechanized Menace")
    m = make_token(g, "Mech", 0)
    p = put(g, 0, "Pouty Poro")
    em = make_token(g, "Mech", 1)
    assert g.has_kw(m, "Shield") and not g.has_kw(p, "Shield") and not g.has_kw(em, "Shield")
    m.desig = "def"
    assert g.might(m) == 4


# ------------------------------------------------------------------ Shen
@T.test
def shen_gives_tank():
    g, _ = new()
    leg(g, 0, "Shen, Eye of Twilight")
    u = put(g, 0, "Pouty Poro")
    act(g, 0, "Tank")
    settle(g)
    assert g.has_kw(u, "Tank") and g.p[0].legend.exhausted


# ------------------------------------------------------------------ Vex
@T.test
def vex_hold_exhaust_to_draw():
    g, _ = new()
    leg(g, 0, "Vex, Gloomist")
    put(g, 0, "Pouty Poro", 1)
    g.hold(0, g.bfs[1])
    settle(g)
    assert len(g.p[0].hand) == 1 and g.p[0].legend.exhausted


# ------------------------------------------------------------------ Vi
@T.test
def vi_readies_unit_after_3_excess_damage():
    g, _ = new()
    leg(g, 0, "Vi, Piltover Enforcer")
    a = put(g, 0, "Glasc Mixologist")                      # 5 on a 1-might unit: 4 excess
    put(g, 1, "Soaring Scout", 1)
    attack(g, [a])
    assert g.bfs[1].ctrl == 0 and not a.exhausted and g.p[0].legend.exhausted


@T.test
def vi_no_trigger_below_3_excess():
    g, _ = new()
    leg(g, 0, "Vi, Piltover Enforcer")
    a = put(g, 0, "Stellacorn Herder")                     # 3 on a 1-might unit: 2 excess
    put(g, 1, "Soaring Scout", 1)
    attack(g, [a])
    assert g.bfs[1].ctrl == 0 and a.exhausted and not g.p[0].legend.exhausted
    # damage already marked reduces the lethal need: 5 on a 2-might unit with 1 damage = 4 excess
    g2, _ = new()
    leg(g2, 0, "Vi, Piltover Enforcer")
    a2 = put(g2, 0, "Glasc Mixologist")
    d2 = put(g2, 1, "Pouty Poro", 1)
    d2.damage = 1
    attack(g2, [a2])
    assert g2.bfs[1].ctrl == 0 and not a2.exhausted


# ------------------------------------------------------------------ Viktor
@T.test
def viktor_plays_recruit():
    g, _ = new()
    leg(g, 0, "Viktor, Herald of the Arcane")
    runes(g, 0, ["Order"])
    act(g, 0, "Recruit")
    settle(g)
    rs = [u for u in g.units(0) if u.cname == "Recruit"]
    assert len(rs) == 1 and rs[0].loc == "base" and rs[0].exhausted and g.might(rs[0]) == 1


# ------------------------------------------------------------------ Volibear
@T.test
def volibear_channels_on_mighty_unit_played():
    g, _ = new()
    leg(g, 0, "Volibear, Relentless Storm")
    runes(g, 0, ["Order"] * 6)
    hand(g, 0, "Glasc Mixologist")
    g.apply(opt(g, 0, "Glasc Mixologist"))
    settle(g)
    assert len(g.p[0].runes) == 6 and g.p[0].runes[-1].exhausted and g.p[0].legend.exhausted  # 1 recycled to pay, 1 channeled
    g.p[0].legend.exhausted = False
    hand(g, 0, "Soaring Scout")
    for r in g.p[0].runes:
        r.exhausted = False
    g.apply(opt(g, 0, "Soaring Scout"))
    settle(g)
    assert len(g.p[0].runes) == 6 and not g.p[0].legend.exhausted


# ------------------------------------------------------------------ Yasuo
@T.test
def yasuo_moves_unit_to_and_from_base():
    g, _ = new()
    leg(g, 0, "Yasuo, Unforgiven")
    runes(g, 0, ["Calm"] * 4)
    u = put(g, 0, "Pouty Poro", ready=False)
    act(g, 0, "Move", lambda ch: ch["tg"] == (u.uid,) and ch["dest"] == 1)
    settle(g)
    assert u.loc == 1 and g.bfs[1].ctrl == 0
    g.p[0].legend.exhausted = False
    act(g, 0, "Move", lambda ch: ch["tg"] == (u.uid,))
    settle(g)
    assert u.loc == "base"


# ------------------------------------------------------------------ fuzz: random games with each legend
def fuzz(n_per_legend=2, verbose=False):
    import random, re, hashlib
    from agents import RandomAgent
    from decks import load, DECKS
    from game import DOMAINS
    mine = sorted(k for k, v in IMPL.items() if v.module == "cardsets.legends")
    keys = sorted(DECKS)
    errors = 0

    def one(name, seed):
        rng = random.Random(seed)
        decks = []
        for i in range(2):
            d = load(keys[(seed + i * 5) % len(keys)])
            d = dict(d, legend=name if i == 0 else mine[(seed * 7 + mine.index(name)) % len(mine)])
            doms = [x for x in SPEC[d["legend"]]["domains"] if x in DOMAINS]
            r = list(d["runes"])
            for j in range(6):
                r[j] = doms[j % len(doms)]
            d["runes"] = r
            m = list(d["main"])
            extra = ["Long Sword", "Sterak's Gage", "Back Off", "Discipline", "Pouty Poro", "Glasc Mixologist"]
            for j in range(6):
                m[rng.randrange(len(m))] = extra[j]
            d["main"] = m
            decks.append(d)
        Obj._n = Item._n = 0
        ag = [RandomAgent(seed), RandomAgent(seed + 7919)]
        g = Game(decks, ag, seed=seed, log=True)
        n = 0
        while n < 4000:
            dd = g.advance()
            if dd is None:
                break
            g.apply(ag[dd.player].decide(g, dd))
            n += 1
        return hashlib.md5("\n".join(re.sub(r"#\d+", "", l) for l in g.lines).encode()).hexdigest(), g

    import traceback
    used = set()
    for name in mine:
        for seed in range(n_per_legend):
            try:
                h, g = one(name, seed)
                used |= set(k[4:].split(":")[0] for k in g.stats if isinstance(k, str) and k.startswith("act_"))
                if seed == 0:
                    assert h == one(name, seed)[0], f"{name}: two runs differ"
            except Exception:
                errors += 1
                print(f"{name} seed {seed}: EXCEPTION")
                traceback.print_exc()
    if verbose:
        print(f"fuzz: {len(mine)} legends x {n_per_legend} games, {errors} exceptions; "
              f"legend abilities used: {len(used & set(mine))}")
        print("legends with abilities never activated:",
              ", ".join(n for n in mine if IMPL[n].abilities and n not in used))
    return errors


@T.test
def fuzz_random_games_with_each_legend():
    assert fuzz(1) == 0


if __name__ == "__main__":
    if len(sys.argv) > 1:
        sys.exit(1 if fuzz(int(sys.argv[1]), verbose=True) else 0)
    T.main()
