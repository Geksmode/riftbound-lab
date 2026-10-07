"""Audit des cartes à choix facultatif ("up to", "any number"): règle 355.13 — avec zéro cible le sort se joue
quand même, la suite de l'effet s'applique, le maximum est respecté. Run: python3 cardsets/test_audit_texte.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from actions import card_choices                        # noqa: E402

T = Suite("audit_texte")
ALL = ["Fury", "Calm", "Mind", "Body", "Chaos", "Order"] * 2


def play(g, pid, name, pred=lambda ch: True, src=None):
    g.apply(opt(g, pid, name, pred, src))
    return settle(g)


def rich(g, pid=0):
    runes(g, pid, ALL)


def choices(g, name, every=True, pid=0, src="hand"):
    c = next(x for x in g.p[pid].hand if x.cname == name)
    return card_choices(g, pid, c, src, False, False, every=every)


def no_target(ch):
    return not ch.get("tg")


def zero_target_playable(name, setup=None):
    """The spell can be played with no target (human and AI lists), whatever the board."""
    for scenario in ("empty", "enemies", "friends", "both"):
        for every in (True, False):
            g, _ = new()
            rich(g)
            if scenario in ("enemies", "both"):
                put(g, 1, "Pouty Poro", 1)
                put(g, 1, "Soaring Scout", "base")
            if scenario in ("friends", "both"):
                put(g, 0, "Stellacorn Herder", 0)
                put(g, 0, "Lonely Poro", "base")
            if setup:
                setup(g)
            c = hand(g, 0, name)
            chs = choices(g, name, every)
            z = [x for x in chs if no_target(x) and not x.get("mover")]
            assert z, (name, scenario, every, chs)
            g.apply(opt(g, 0, name, lambda ch: ch == z[0]))
            settle(g)
            assert c in g.p[0].trash or c in g.p[0].banish, (name, scenario, "not played", c.zone)


# ------------------------------------------------------------------ Singularity (mind)
# "Deal 6 to each of up to two units."  355.13
@T.test
def singularity_zero_target():
    zero_target_playable("Singularity")


@T.test
def singularity_max_two_and_any_unit():
    g, _ = new()
    rich(g)
    a, b, c = (put(g, 1, "Pouty Poro", 1), put(g, 1, "Soaring Scout", 1), put(g, 1, "Pouty Poro", 0))
    f = put(g, 0, "Lonely Poro", "base")
    hand(g, 0, "Singularity")
    chs = choices(g, "Singularity")
    assert max(len(x["tg"]) for x in chs) == 2
    # "units": a friendly unit is a legal target too (a human may choose it)
    assert any(f.uid in x["tg"] for x in chs)
    play(g, 0, "Singularity", lambda ch: set(ch["tg"]) == {a.uid, b.uid})
    assert a.zone == "trash" and b.zone == "trash" and c.zone == "board"


# ------------------------------------------------------------------ Fox-Fire (calm)
# "Kill any number of units at a battlefield with total Might 4 or less."
@T.test
def foxfire_zero_target():
    zero_target_playable("Fox-Fire")


@T.test
def foxfire_total_might_and_any_unit():
    g, _ = new()
    rich(g)
    a, b = put(g, 1, "Pouty Poro", 1), put(g, 1, "Soaring Scout", 1)
    big = put(g, 1, "Glasc Mixologist", 1)
    f = put(g, 0, "Lonely Poro", 1)
    hand(g, 0, "Fox-Fire")
    chs = choices(g, "Fox-Fire")
    for x in chs:
        us = [g.obj(u) for u in x["tg"]]
        assert sum(g.might(u) for u in us) <= 4 and len({u.loc for u in us}) <= 1, x
    assert not any(big.uid in x["tg"] for x in chs)
    assert any(f.uid in x["tg"] for x in chs)           # "units" includes friendly ones


# ------------------------------------------------------------------ Bellows Breath (cards.py)
# "[Repeat] Deal 1 to up to three units at the same location."
@T.test
def bellows_zero_target():
    zero_target_playable("Bellows Breath")


@T.test
def bellows_max_three_same_location():
    g, _ = new()
    rich(g)
    us = [put(g, 1, "Pouty Poro", 1) for _ in range(4)]
    far = put(g, 1, "Soaring Scout", "base")
    hand(g, 0, "Bellows Breath")
    for every in (True, False):
        for x in choices(g, "Bellows Breath", every):
            objs = [g.obj(u) for u in x["tg"]]
            assert len(objs) <= 3 and len({o.loc for o in objs}) <= 1, x
    play(g, 0, "Bellows Breath", lambda ch: len(ch["tg"]) == 3 and us[0].uid in ch["tg"])
    assert sum(u.damage for u in us) == 3 and far.damage == 0


# ------------------------------------------------------------------ Tricksy Tentacles (calm)
# "Move any number of enemy units with the same controller and a total Might of 8 or less to a single location."
@T.test
def tricksy_zero_target():
    zero_target_playable("Tricksy Tentacles")


@T.test
def tricksy_total_might_max_8():
    g, _ = new()
    rich(g)
    a, b = put(g, 1, "Pouty Poro", 1), put(g, 1, "Soaring Scout", 1)
    big = put(g, 1, "Glasc Mixologist", 0, bf_control=False)
    hand(g, 0, "Tricksy Tentacles")
    for x in choices(g, "Tricksy Tentacles"):
        assert sum(g.might(g.obj(u)) for u in x["tg"]) <= 8


# ------------------------------------------------------------------ Piercing Light (fury)
# "Deal 2 to a unit at a battlefield, then deal 2 to up to one other unit."
@T.test
def piercing_light_second_target_optional():
    g, _ = new()
    rich(g)
    a = put(g, 1, "Glasc Mixologist", 1)
    hand(g, 0, "Piercing Light")
    chs = choices(g, "Piercing Light")
    assert any(x["tg"] == (a.uid,) for x in chs), chs        # only one target: the first part still applies
    play(g, 0, "Piercing Light", lambda ch: ch["tg"] == (a.uid,) and not ch.get("rep"))
    assert a.damage == 2


@T.test
def piercing_light_no_first_target_not_playable():
    g, _ = new()
    rich(g)
    put(g, 1, "Soaring Scout", "base")                       # only a unit in a base: no legal first target
    hand(g, 0, "Piercing Light")
    assert not choices(g, "Piercing Light")


# ------------------------------------------------------------------ Emperor's Divide, Flash, Decree, Shadows
@T.test
def divide_zero_target():
    zero_target_playable("Emperor's Divide")


@T.test
def flash_zero_target_and_max_two():
    zero_target_playable("Flash")
    g, _ = new()
    rich(g)
    us = [put(g, 0, "Pouty Poro", 1) for _ in range(3)]
    hand(g, 0, "Flash")
    assert max(len(x["tg"]) for x in choices(g, "Flash")) == 2
    assert max(len(x["tg"]) for x in choices(g, "Flash", False)) == 2


@T.test
def decree_zero_target():
    zero_target_playable("Decree of Discord")


@T.test
def shadows_zero_target():
    zero_target_playable("Shadows of the Past")


@T.test
def shadows_max_two():
    g, _ = new()
    rich(g)
    for n in ("Pouty Poro", "Soaring Scout", "Lonely Poro"):
        o = Obj(n, 0)
        o.zone = "trash"
        g.p[0].trash.append(o)
    hand(g, 0, "Shadows of the Past")
    for every in (True, False):
        assert max(len(x["cs"]) for x in choices(g, "Shadows of the Past", every)) == 2


# ------------------------------------------------------------------ Moonfall (mind)
# "Choose a battlefield where you have units. You may move up to one enemy unit to that battlefield. Then give
# enemy units there -2 might this turn."
def to_focus(g, mover, bf, name):
    """Attack bf with mover and pass priority until the player can play the named card in the showdown."""
    g.apply(("move", (mover.uid,), bf))
    for _ in range(6):
        d = g.advance()
        if any(o[0] == "play" and g.obj(o[1]) is None and o[2] == "hand" and
               next(c for c in g.p[0].hand if c.uid == o[1]).cname == name for o in d.options):
            return d
        g.apply(("pass",))
    raise AssertionError("no showdown")


@T.test
def moonfall_zero_target_still_shrinks():
    g, _ = new()
    rich(g)
    f = put(g, 0, "Stellacorn Herder", "base")
    e = put(g, 1, "Glasc Mixologist", 0)
    hand(g, 0, "Moonfall")
    m0 = g.might(e)
    to_focus(g, f, 0, "Moonfall")
    g.apply(opt(g, 0, "Moonfall", lambda ch: ch["bf"] == 0 and not ch["tg"]))
    for _ in range(4):                                  # pass priority until the spell has resolved
        if not g.chain:
            break
        g.apply(("pass",))
        g.advance()
    assert e.loc == 0 and g.might(e) == m0 - 2


@T.test
def moonfall_needs_own_units():
    g, _ = new()
    rich(g)
    put(g, 1, "Glasc Mixologist", 0, bf_control=False)
    hand(g, 0, "Moonfall")
    assert not choices(g, "Moonfall")


# ------------------------------------------------------------------ Disposal Order (body)
# "Choose one — Choose up to 3 cards from opponents' trashes. Their owners recycle them. / Draw 1."
@T.test
def disposal_order_recycle_zero_cards():
    g, _ = new()
    rich(g)
    hand(g, 0, "Disposal Order")
    chs = choices(g, "Disposal Order")
    assert any(x["mode"] == "recycle" and not x["cards"] for x in chs), chs
    play(g, 0, "Disposal Order", lambda ch: ch["mode"] == "recycle" and not ch["cards"])


@T.test
def disposal_order_max_three():
    g, _ = new()
    rich(g)
    for _ in range(5):
        o = Obj("Pouty Poro", 1)
        o.zone = "trash"
        g.p[1].trash.append(o)
    hand(g, 0, "Disposal Order")
    for every in (True, False):
        assert max(len(x.get("cards", ())) for x in choices(g, "Disposal Order", every)) == 3


# ------------------------------------------------------------------ Guerilla Warfare (mind)
@T.test
def guerilla_zero_cards_still_hides_free():
    g, _ = new()
    rich(g)
    hand(g, 0, "Guerilla Warfare")
    play(g, 0, "Guerilla Warfare")
    assert any(e.get("kind") == "hide_free" for e in g.effects)


@T.test
def guerilla_returns_at_most_two():
    g, _ = new()
    rich(g)
    for _ in range(3):
        o = Obj("Fox-Fire", 0)
        o.zone = "trash"
        g.p[0].trash.append(o)
    hand(g, 0, "Guerilla Warfare")
    play(g, 0, "Guerilla Warfare")
    assert sum(1 for c in g.p[0].hand if c.cname == "Fox-Fire") == 2


# ------------------------------------------------------------------ Acceleration Gate (mind)
@T.test
def gate_zero_target_and_max_four():
    g, _ = new()
    rich(g)
    for _ in range(6):
        put(g, 0, "Pouty Poro", "base", ready=False)
    hand(g, 0, "Acceleration Gate")
    for x in choices(g, "Acceleration Gate"):
        assert len(x["tg"]) + len(x["runes"]) <= 4
    assert any(not x["tg"] for x in choices(g, "Acceleration Gate"))


# ------------------------------------------------------------------ Shuriken Flip (reference)
@T.test
def flip_no_target_still_moves():
    g, _ = new()
    rich(g)
    m = put(g, 0, "Pouty Poro", "base")
    hand(g, 0, "Shuriken Flip")
    play(g, 0, "Shuriken Flip", lambda ch: not ch["tg"] and ch["mover"] == m.uid and ch["dest"] == 0)
    assert m.loc == 0


@T.test
def piercing_light_second_part_applies_when_first_target_is_gone():
    g, _ = new()
    rich(g)
    a, b = put(g, 1, "Pouty Poro", 1), put(g, 1, "Glasc Mixologist", 1)
    hand(g, 0, "Piercing Light")
    g.apply(opt(g, 0, "Piercing Light", lambda ch: ch["tg"] == (a.uid, b.uid) and not ch.get("rep")))
    g.kill([a], 1)                                     # the first target dies before resolution
    settle(g)
    assert b.damage == 2                               # "then deal 2 to up to one other unit" still happens


@T.test
def singularity_one_target_gone_other_still_hit():
    g, _ = new()
    rich(g)
    a, b = put(g, 1, "Pouty Poro", 1), put(g, 1, "Glasc Mixologist", 1)
    hand(g, 0, "Singularity")
    g.apply(opt(g, 0, "Singularity", lambda ch: set(ch["tg"]) == {a.uid, b.uid}))
    g.kill([a], 1)
    settle(g)
    assert b.zone == "trash"


if __name__ == "__main__":
    T.main()
