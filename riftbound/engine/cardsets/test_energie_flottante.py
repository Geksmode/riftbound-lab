"""Énergie flottante (Rune Pool, règle 429) : épuiser une rune prête pour 1 énergie juste avant de la recycler pour sa
puissance. Exemple de l'utilisateur : Punch First (1 énergie + 2 puissance Body) joué en première carte du tour.
Run: python3 cardsets/test_energie_flottante.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from replay import snap                             # noqa: E402

T = Suite("energie_flottante")


def punch_first_with(domains):
    g, _ = new()
    runes(g, 0, domains)
    u = put(g, 0, "Mournful Witness")
    hand(g, 0, "Punch First")
    g.apply(opt(g, 0, "Punch First", lambda ch: ch.get("tg") == (u.uid,)))
    settle(g)
    return g


@T.test
def punch_first_floats_the_energy_of_recycled_runes():
    # deux runes Body épuisées pour 2 énergie puis recyclées pour 2 puissance : 1 énergie paie le sort, 1 flotte,
    # et les deux runes Fury restent prêtes
    g = punch_first_with(["Body", "Body", "Fury", "Fury"])
    p = g.p[0]
    assert p.pool_e == 1, p.pool_e
    assert sorted((r.domain, r.exhausted) for r in p.runes) == [("Fury", False), ("Fury", False)]


@T.test
def page_receives_floating_energy_and_power():
    g = punch_first_with(["Body", "Body", "Fury", "Fury"])
    g.p[0].pool_p["Calm"] += 1
    st = snap(g)
    assert st["p"][0]["pe"] == 1 and st["p"][0]["pp"] == {"Calm": 1}, st["p"][0]


if __name__ == "__main__":
    T.main()
