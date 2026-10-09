"""Recherche de l'IA (ai.SearchAgent.pick, 2026-10-09) : tirages communs et élimination en plusieurs passes.
Run: python3 cardsets/test_recherche.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from game import Game, Obj, Item                        # noqa: E402
import ai                                               # noqa: E402
import plans as P                                       # noqa: E402

T = Suite("recherche")


def midgame(seed=5, turns=4):
    """Partie Akali contre LeBlanc (IA ancienne recherche) arrêtée à une décision principale à au moins 4 options."""
    Obj._n = 0
    Item._n = 0
    ag = [P.PlanAgent(seed, search="old"), P.PlanAgent(seed + 1, search="old")]
    g = Game([A, L], ag, seed=seed, first=0)
    while True:
        d = g.advance()
        assert d is not None
        if d.kind == "main" and g.turn_no >= turns and len(d.options) >= 4:
            return g, d
        g.apply(ag[d.player].decide(g, d))


@T.test
def identical_options_get_identical_values_with_common_worlds():
    g, d = midgame()
    a = d.options[1]
    me = d.player
    ag = P.PlanAgent(7, samples=3, search="crn")
    _, _, scored = ag.pick(g, d, [a, a, a])
    assert scored[0][0] == scored[1][0] == scored[2][0], scored


@T.test
def search_is_deterministic_and_leaves_counters():
    g, d = midgame()
    n0, i0 = Obj._n, Item._n
    r1 = P.PlanAgent(9, search="sh").pick(g, d, list(d.options))
    assert (Obj._n, Item._n) == (n0, i0)
    r2 = P.PlanAgent(9, search="sh").pick(g, d, list(d.options))
    assert r1[0] == r2[0] and [v for v, _ in r1[2]] == [v for v, _ in r2[2]]


@T.test
def halving_budget_is_options_times_samples_times_one_plus_extra():
    g, d = midgame()
    opts = list(d.options)
    calls = []

    class Count(P.PlanAgent):
        def one(s, g, me, a, rng, world=None):
            calls.append(world)
            return 0.0
    for n in sorted({2, 3, len(opts)}):
        for extra in (0.5, 1.0):
            calls.clear()
            Count(1, search="sh", sh_extra=extra).pick(g, d, opts[:n])
            assert n <= len(calls) <= int(n * (1 + extra)), (n, extra, len(calls))
            assert extra < 1 or len(calls) > n
    calls.clear()
    Count(1, search="crn").pick(g, d, opts)
    assert len(calls) == len(opts)


@T.test
def best_of_halving_is_among_the_most_sampled():
    g, d = midgame()
    ag = P.PlanAgent(3, search="sh")
    seen = {}
    one = ag.one

    def spy(g_, me, a, rng, world=None):
        seen[repr(a)] = seen.get(repr(a), 0) + 1
        return one(g_, me, a, rng, world)
    ag.one = spy
    best, _, _ = ag.pick(g, d, list(d.options))
    assert seen[repr(best)] == max(seen.values()), seen


@T.test
def evaluation_weights_default_and_override():
    """ai.EV (poids de l'évaluation) : sans réglage, la valeur est celle des poids par défaut ; un réglage cfg["ev"]
    ne touche que l'agent qui le porte (les essais d'auto-jeu comparent deux agents dans la même partie)."""
    g, d = midgame()
    me = d.player
    v0 = ai.evaluate(g, me)
    assert v0 == ai.evaluate(g, me, dict(ai.EV))
    hand = len(g.p[me].hand) - len(g.p[1 - me].hand)
    w = dict(ai.EV, card0=ai.EV["card0"] + 1.0, react=ai.EV["react"] + 1.0)
    assert abs(ai.evaluate(g, me, w) - v0 - hand) < 1e-9
    a, b = P.PlanAgent(1, cfg={"ev": {"bf": 9.0}}), P.PlanAgent(1)
    assert a.ev["bf"] == 9.0 and b.ev is None and ai.EV["bf"] == 3.0


if __name__ == "__main__":
    T.main()
