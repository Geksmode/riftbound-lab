"""Politique de simulation sans liste des coups (Game.advance(lazy=True), ai.PolicyAgent, 2026-10-27) : même coup
qu'avec la liste complète, et une partie jouée « sans liste » est la même partie."""
import os, sys, random
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
import ai                                               # noqa: E402
import train as TR                                      # noqa: E402
from game import Game, Obj, Item, Decision              # noqa: E402
from decks import with_bf, load                         # noqa: E402

T = Suite("ai_lazy")


def decks(seed):
    dd = sorted(TR.default_decks().values())
    r = random.Random(seed)
    da, db = load(r.choice(dd)), load(r.choice(dd))
    return [with_bf(da, r.choice(sorted(da["battlefields"]))), with_bf(db, r.choice(sorted(db["battlefields"])))], \
        r.randrange(2)


def play(seed, lazy, check=None):
    """Partie PolicyAgent contre PolicyAgent ; check(g, d) appelé à chaque décision sans liste. -> suite des coups."""
    dk, f = decks(seed)
    Obj._n = Item._n = 0
    ag = [ai.PolicyAgent(), ai.PolicyAgent()]
    g = Game(dk, ag, seed=seed, first=f)
    out = []
    for _ in range(3000):
        d = g.advance(lazy=lazy)
        if d is None:
            break
        if check:
            check(g, d)
        a = ag[d.player].decide(g, d)
        out.append(a)
        g.apply(a)
    return out, g.winner, g.turn_no


@T.test
def lazy_policy_picks_the_same_move_as_with_the_full_list():
    pol = ai.PolicyAgent()
    n = [0]

    def check(g, d):
        if d.kind != "main" and not ai.POL_REACT:
            return                                       # réaction tirée au hasard : consomme g.rng
        gen = dict(main=g.options_main, focus=g.options_focus, priority=g.options_priority)[d.kind]
        full = Decision(d.kind, d.player, gen(d.player))
        a, b = pol.decide(g, d), pol.decide(g, full)
        assert a == b, (d.kind, a, b)
        n[0] += 1
    for seed in (4_100_001, 4_100_002):
        play(seed, True, check)
    assert n[0] > 100, n[0]


@T.test
def lazy_game_is_the_same_game():
    for seed in (4_100_003, 4_100_004):
        assert play(seed, True) == play(seed, False)


if __name__ == "__main__":
    T.main()
