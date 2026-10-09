#!/usr/bin/env python3
"""Temps de réflexion de l'IA par décision, sur les MÊMES positions pour chaque version (exp_search.version).

    python3 exp_temps.py <graines séparées par des virgules> <version> [<version>...]
    ex. python3 exp_temps.py 777003,777011 old sh

Positions = toutes les décisions à plus d'une option de parties « defaut » (exp_search.setup) jouées par l'ancienne
recherche ; chaque version décide sur une copie de chaque position. Dans Pyodide (wasm 32 bits, comme la table) :
node exp_temps.mjs <graines> <versions...>."""
import sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
A = sys.argv[1:]
sys.argv = sys.argv[:1]
import exp_search as E
from game import Game, Obj, Item
import plans as P


def positions(seeds):
    pos = []
    for seed in seeds:
        Obj._n = 0
        Item._n = 0
        decks, pls, f = E.setup(seed, "defaut")
        ag = [P.PlanAgent(seed + 500000 * i, plan=pls[i], opp_plan=pls[1 - i], search="old") for i in range(2)]
        g = Game(decks, ag, seed=seed, first=f)
        while True:
            d = g.advance()
            if d is None:
                break
            if len(d.options) > 1:
                pos.append((g.clone(), d, Obj._n, Item._n))
            g.apply(ag[d.player].decide(g, d))
    return pos


def main(seeds, versions):
    pos = positions(seeds)
    print(len(pos), "positions", flush=True)
    for v in versions:
        sm, n, ex, h, cfg = E.version(v)
        tm = []
        for k, (g, d, o, i) in enumerate(pos):
            Obj._n, Item._n = o, i
            c = g.clone()
            a = P.PlanAgent(k, samples=n, search=sm, sh_extra=ex, horizon=h, cfg=cfg)
            t = time.perf_counter()
            a.decide(c, d)
            tm.append(time.perf_counter() - t)
        tm.sort()
        print(f"{v} : moyenne {1000 * sum(tm) / len(tm):.0f} ms, p95 {1000 * tm[int(.95 * len(tm))]:.0f} ms, "
              f"max {1000 * tm[-1]:.0f} ms", flush=True)


if __name__ == "__main__":
    main([int(x) for x in A[0].split(",")], A[1:])
