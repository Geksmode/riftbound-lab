#!/usr/bin/env python3
"""Recherche de l'IA (ai.SearchAgent.pick) : version A contre version B, parties appariées.

    python3 exp_search.py <paires> <graine0> <A> <B> <akali|miroir> [processus]

Une paire = une graine g jouée DEUX fois, mêmes decks, même premier joueur, mêmes battlefields : partie 1, A tient le
joueur 0 ; partie 2, B tient le joueur 0 (places échangées). Score de A pour la paire = moyenne des deux parties ;
l'écart-type est celui de la moyenne sur les paires (les deux parties d'une paire ne sont pas indépendantes).
Graines des agents liées au siège (g et g+500000, comme exp_plans.one et replay.record_plan) : A = B donne
exactement 0,5 par paire, et une partie se rejoue avec add_replays (clé « search »).
  akali  : Akali G2 (plan Gorica) contre LeBlanc IQ#5 (plan Hook tempo), tirage du premier joueur et des
           battlefields exactement comme exp_plans.one ;
  miroir : un deck légal au hasard (exp_general.rdeck) des deux côtés, IA générique (plans.Plan()).
Versions : VERSIONS (nom -> (mode de recherche, samples, sh_extra)). Résultats : results_search.json (stamp()).
Temps par décision : moyenne et p95 des decide() à plus d'une option, par version."""
from version import stamp
import json, sys, time, os, random, math, traceback
from multiprocessing import Pool
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_argv, sys.argv = sys.argv, sys.argv[:1]      # exp.py lit sys.argv à l'import
from exp import L
from exp_gorica import G2
sys.argv = _argv
import plans as P
from game import Game, Obj, Item
from decks import with_bf
from exp_general import Timed, pct

VERSIONS = {
    "old": ("old", 1, 0.0),      # recherche d'avant le 2026-10-09 (niveau Normal)
    "old2": ("old", 2, 0.0),     # ancienne, 2 tirages par option : même budget que sh
    "crn": ("crn", 1, 0.0),      # tirages communs seuls : même budget que old
    "sh": ("sh", 1, 1.0),        # tirages communs + élimination en passes : budget ≈ 2 × old
    "sh05": ("sh", 1, 0.5),      # budget ≈ 1,5 × old
}


def setup(seed, mode):
    """(decks, plans neufs, premier joueur) de la graine : mêmes decks et premier joueur pour les deux parties."""
    r = random.Random(seed * 7919)
    if mode == "akali":
        f = r.randrange(2)
        pa, pl = P.PLANS["Gorica"](), P.PLANS["Hook tempo"]()
        lbf = pl.battlefield(L, f == 1) or r.choice(L["battlefields"])
        abf = pa.battlefield(G2, f == 0, lbf) or r.choice(G2["battlefields"])
        return [with_bf(G2, abf), with_bf(L, lbf)], [pa, pl], f
    from exp_general import rdeck
    d = rdeck(random.Random(seed))
    f = r.randrange(2)
    b = sorted(d["battlefields"])
    return [with_bf(d, r.choice(b)), with_bf(d, r.choice(b))], [P.Plan(), P.Plan()], f


def play(seed, mode, v0, v1):
    Obj._n = 0
    Item._n = 0
    decks, (p0, p1), f = setup(seed, mode)          # plans neufs à chaque partie (ils gardent un état)
    tm = [[], []]
    ag = []
    for i, (v, pl, op) in enumerate(((v0, p0, p1), (v1, p1, p0))):
        sm, n, ex = VERSIONS[v]
        ag.append(P.PlanAgent(seed + 500000 * i, plan=pl, opp_plan=op, samples=n, search=sm, sh_extra=ex))
    wr = [Timed(ag[0], tm[0]), Timed(ag[1], tm[1])]
    g = Game(decks, wr, seed=seed, first=f)
    while True:
        d = g.advance()
        if d is None:
            break
        g.apply(wr[d.player].decide(g, d))
    return g.winner, g.turn_no, tm, f, [d["legend"] for d in decks]


def pair(arg):
    gs, va, vb, mode = arg
    try:
        w1, t1, tm1, f, lg = play(gs, mode, va, vb)
        w2, t2, tm2, _, _ = play(gs, mode, vb, va)
    except Exception:
        return dict(seed=gs, err=traceback.format_exc()[-800:])
    sc = lambda w, me: 0.5 if w in (-1, None) else (1.0 if w == me else 0.0)
    a1, a2 = sc(w1, 0), sc(w2, 1)
    return dict(seed=gs, legends=lg, first=f, a1=a1, a2=a2, w=[w1, w2], score=(a1 + a2) / 2, turns=[t1, t2],
                tA=tm1[0] + tm2[1], tB=tm1[1] + tm2[0])


def run(n, seed0, va, vb, mode, procs=4):
    t0 = time.time()
    with Pool(procs) as p:
        out = p.map(pair, [(seed0 + k, va, vb, mode) for k in range(n)], chunksize=1)
    good = [o for o in out if "err" not in o]
    errs = [o for o in out if "err" in o]
    sc = [o["score"] for o in good]
    m = sum(sc) / max(1, len(sc))
    var = sum((x - m) ** 2 for x in sc) / max(1, len(sc) - 1)
    se = math.sqrt(var / max(1, len(sc)))
    ta = [x for o in good for x in o["tA"]]
    tb = [x for o in good for x in o["tB"]]
    res = dict(label=f"{va} contre {vb} · {mode}", A=va, B=vb, mode=mode, pairs=len(good), seed0=seed0,
               score_A=round(m, 4), se=round(se, 4), z=round((m - 0.5) / se, 2) if se > 0 else None,
               A_as_player0=round(sum(o["a1"] for o in good) / max(1, len(good)), 4),
               A_as_player1=round(sum(o["a2"] for o in good) / max(1, len(good)), 4),
               ms_per_decision=dict(A_mean=round(1000 * sum(ta) / max(1, len(ta)), 1), A_p95=round(1000 * pct(ta, .95), 1),
                                    B_mean=round(1000 * sum(tb) / max(1, len(tb)), 1), B_p95=round(1000 * pct(tb, .95), 1),
                                    A_n=len(ta), B_n=len(tb)),
               turns_mean=round(sum(sum(o["turns"]) for o in good) / max(1, 2 * len(good)), 1),
               errors=len(errs), err=errs[0]["err"] if errs else None, seconds=round(time.time() - t0),
               per_pair=[[o["seed"], o["score"]] for o in good],
               games=[[o["seed"], o["first"], o["w"], o["legends"][0]] for o in good])
    stamp(res)
    return res


if __name__ == "__main__":
    n, seed0, va, vb, mode = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], sys.argv[4], sys.argv[5]
    procs = int(sys.argv[6]) if len(sys.argv) > 6 else 4
    r = run(n, seed0, va, vb, mode, procs)
    f = HERE / "results_search.json"
    done = json.load(open(f)) if f.exists() else []
    done.append(r)
    json.dump(done, open(f, "w"), ensure_ascii=False)
    print(f"{r['label']}: A {r['score_A']:.1%} ± {r['se']:.3f} (z={r['z']}) sur {r['pairs']} paires "
          f"(graines {seed0}-{seed0 + n - 1}), A joueur0 {r['A_as_player0']:.2f} / joueur1 {r['A_as_player1']:.2f}, "
          f"ms/décision {r['ms_per_decision']}, tours {r['turns_mean']}, erreurs {r['errors']}, {r['seconds']}s")
    if r["err"]:
        print(r["err"])
