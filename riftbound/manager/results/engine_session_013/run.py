#!/usr/bin/env python3
"""Monte Carlo runner. Usage: python3 run.py <experiment> [games]"""
import sys, json, time, random, os
from collections import Counter
from multiprocessing import Pool
from pathlib import Path

HERE = Path(__file__).resolve().parent


def one(arg):
    i, a_deck, l_deck, a_bfs, l_bfs, first, samples = arg
    from game import Game
    from ai import SearchAgent
    from decks import with_bf
    r = random.Random(i * 7919)
    A = with_bf(a_deck, r.choice(a_bfs))
    L = with_bf(l_deck, r.choice(l_bfs))
    f = r.randrange(2) if first is None else first
    ag = [SearchAgent(i, samples=samples), SearchAgent(i + 500000, samples=samples)]
    g = Game([A, L], ag, seed=i, first=f)
    try:
        while True:
            d = g.advance()
            if d is None:
                break
            g.apply(ag[d.player].decide(g, d))
    except Exception as e:
        return dict(err=repr(e)[:200], seed=i)
    return dict(w=g.winner, turns=g.turn_no, pts=[p.points for p in g.p], first=f,
                bf=(A["battlefield"], L["battlefield"]))


def play(n, a_deck, l_deck, a_bfs=None, l_bfs=None, first=None, samples=1, seed0=0, procs=None, label=""):
    a_bfs = a_bfs or a_deck["battlefields"]
    l_bfs = l_bfs or l_deck["battlefields"]
    args = [(seed0 + i, a_deck, l_deck, a_bfs, l_bfs, first, samples) for i in range(n)]
    with Pool(procs or os.cpu_count()) as p:
        out = p.map(one, args, chunksize=2)
    errs = [o for o in out if o.get("err")]
    good = [o for o in out if not o.get("err")]
    wins = sum(1 for o in good if o["w"] == 0) + 0.5 * sum(1 for o in good if o["w"] == -1)
    wr = wins / max(1, len(good))
    res = dict(label=label, n=len(good), wr=round(wr, 4),
               se=round((wr * (1 - wr) / max(1, len(good))) ** .5, 4),
               turns=round(sum(o["turns"] for o in good) / max(1, len(good)), 1),
               errors=len(errs), err_sample=errs[:2])
    by_first = {}
    for f in (0, 1):
        sub = [o for o in good if o["first"] == f]
        if sub:
            by_first["akali_first" if f == 0 else "leblanc_first"] = round(
                sum(1 for o in sub if o["w"] == 0) / len(sub), 3)
    res["by_first"] = by_first
    return res
