"""Audit des cartes « X, then Y » dont la première partie peut ne rien faire : la suite s'applique quand même
(règle 359.3.e : un effet qui ne peut pas être appliqué en entier l'est autant que possible).
Run: python3 cardsets/test_audit_then.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from cardsets.test_audit_texte import play, rich        # noqa: E402

T = Suite("audit_then")


def put_hand_only(g, pid, names):
    g.p[pid].hand = []
    for n in names:
        hand(g, pid, n)


def deck_of(g, pid, n=6):
    g.p[pid].deck = [Obj("Pouty Poro", pid) for _ in range(n)]
    for c in g.p[pid].deck:
        c.zone = "deck"


# "Discard 1, then draw 1/2": with an empty hand the draw still happens
@T.test
def lunar_boon_empty_hand_still_draws_two():
    g, _ = new()
    rich(g)
    deck_of(g, 0)
    hand(g, 0, "Lunar Boon")
    n = len(g.p[0].deck)
    play(g, 0, "Lunar Boon")
    assert len(g.p[0].hand) == 2 and len(g.p[0].deck) == n - 2


@T.test
def lunar_boon_discards_one_then_draws_two():
    g, _ = new()
    rich(g)
    deck_of(g, 0)
    hand(g, 0, "Lunar Boon")
    hand(g, 0, "Pouty Poro")
    play(g, 0, "Lunar Boon")
    assert len(g.p[0].hand) == 2 and sum(1 for c in g.p[0].trash if c.cname == "Pouty Poro") == 1


@T.test
def evershade_stalker_empty_hand_still_draws():
    g, _ = new()
    rich(g)
    deck_of(g, 0)
    hand(g, 0, "Evershade Stalker")
    play(g, 0, "Evershade Stalker", lambda ch: ch["loc"] == "base" and not ch.get("acc"))
    assert len(g.p[0].hand) == 1


@T.test
def traveling_merchant_empty_hand_still_draws():
    g, _ = new()
    deck_of(g, 0)
    m = put(g, 0, "Traveling Merchant")
    g.apply(("move", (m.uid,), 0))
    settle(g)
    assert len(g.p[0].hand) == 1


@T.test
def invert_timelines_empty_hands_draw_four_each():
    g, _ = new()
    rich(g)
    deck_of(g, 0, 8)
    deck_of(g, 1, 8)
    hand(g, 0, "Invert Timelines")
    play(g, 0, "Invert Timelines")
    assert len(g.p[0].hand) == 4 and len(g.p[1].hand) == 4


# Janna, Savior: "heal your units here, then move an enemy unit from here to its base" — heal without an enemy
@T.test
def janna_heals_without_enemy_to_move():
    g, _ = new()
    rich(g)
    f = put(g, 0, "Glasc Mixologist", 0)
    f.damage = 3
    hand(g, 0, "Janna, Savior")
    d = g.advance()
    opts = [o for o in d.options if o[0] == "play" and any(c.uid == o[1] and c.cname == "Janna, Savior" for c in g.p[0].hand)
            and o[3].get("loc") == 0]
    assert opts, [o for o in d.options if o[0] == "play"]
    g.apply(opts[0])
    settle(g)
    assert f.damage == 0


# Showstopper: "Buff a friendly unit in your base, then move it" — an already buffed unit still moves
@T.test
def showstopper_buffed_unit_still_moves():
    g, _ = new()
    rich(g)
    u = put(g, 0, "Pouty Poro", "base")
    g.buff(u)
    hand(g, 0, "Showstopper")
    play(g, 0, "Showstopper", lambda ch: ch["tg"] == (u.uid,) and ch["dest"] == 1)
    assert u.buff == 1 and u.loc == 1


# Void Assault: "Move a friendly unit, then move an enemy unit" — if the first target is gone, the second part applies
@T.test
def void_assault_second_part_without_first_target():
    g, _ = new()
    rich(g)
    f = put(g, 0, "Pouty Poro", "base")
    e = put(g, 1, "Glasc Mixologist", 1)
    hand(g, 0, "Void Assault")
    g.apply(opt(g, 0, "Void Assault", lambda ch: ch["tg"] == (f.uid, e.uid) and ch["d2"] == "base"))
    g.kill([f], 0)
    settle(g)
    assert e.loc == "base"


# Arise!: "Play a Sand Soldier for each Equipment you control. Then ready two of them."
@T.test
def arise_readies_at_most_two_and_works_with_one_equipment():
    g, _ = new()
    rich(g)
    put(g, 0, "Long Sword")
    hand(g, 0, "Arise!")
    play(g, 0, "Arise!")
    toks = [u for u in g.units(0) if u.cname == "Sand Soldier"]
    assert len(toks) == 1 and not toks[0].exhausted
    g, _ = new()
    rich(g)
    for _ in range(3):
        put(g, 0, "Long Sword")
    hand(g, 0, "Arise!")
    play(g, 0, "Arise!")
    toks = [u for u in g.units(0) if u.cname == "Sand Soldier"]
    assert len(toks) == 3 and sum(1 for t in toks if not t.exhausted) == 2


@T.test
def arise_without_equipment_does_nothing_but_is_playable():
    g, _ = new()
    rich(g)
    c = hand(g, 0, "Arise!")
    play(g, 0, "Arise!")
    assert c in g.p[0].trash and not [u for u in g.units(0) if u.cname == "Sand Soldier"]


# Overt Operation: "you may spend its buff to ready it. Then buff all friendly units." (declining is allowed)
@T.test
def overt_operation_decline_then_buff_all():
    g, _ = new(answers0={"overt_spend": lambda opts, ctx: False})
    rich(g)
    a = put(g, 0, "Pouty Poro", "base", ready=False)
    g.buff(a)
    b = put(g, 0, "Soaring Scout", "base")
    hand(g, 0, "Overt Operation")
    play(g, 0, "Overt Operation")
    assert a.exhausted and a.buff == 1 and b.buff == 1


@T.test
def overt_operation_spend_readies_then_buffs_all():
    g, _ = new(answers0={"overt_spend": lambda opts, ctx: True})
    rich(g)
    a = put(g, 0, "Pouty Poro", "base", ready=False)
    g.buff(a)
    hand(g, 0, "Overt Operation")
    play(g, 0, "Overt Operation")
    assert not a.exhausted and a.buff == 1                # spent, then buffed again by the second part


# Lacerate: "If it's Empowered, disempower it. Then kill it if it has 3 might or less."
@T.test
def lacerate_disempower_then_kill_check_uses_new_might():
    g, _ = new()
    runes(g, 0, ["Order"] * 8)
    e = put(g, 1, "Apprentice Mage", 1)
    g.empower(e)                                         # +1 might while Empowered
    settle(g)
    assert e.empowered
    hand(g, 0, "Lacerate")
    m_before = g.might(e)
    play(g, 0, "Lacerate", lambda ch: ch["tg"] == (e.uid,))
    assert not e.empowered
    assert m_before == 4 and e.zone == "trash"            # 4 while Empowered (> 3), 3 after: killed


# Right of Conquest: "Draw 1, then draw 1 for each battlefield you control" (zero battlefields: just 1)
@T.test
def right_of_conquest_zero_battlefields():
    g, _ = new()
    rich(g)
    deck_of(g, 0)
    hand(g, 0, "Right of Conquest")
    play(g, 0, "Right of Conquest")
    assert len(g.p[0].hand) == 1


if __name__ == "__main__":
    T.main()
