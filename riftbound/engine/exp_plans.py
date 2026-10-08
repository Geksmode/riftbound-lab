#!/usr/bin/env python3
"""Plans de jeu des joueurs (plans.py) : python3 exp_plans.py <parties> [graine0] [budget].
Résultats : results_plans.json. Même bloc de graines pour toutes les configurations (comparaisons appariées)."""
from version import stamp
import json, sys, time, os, random
from multiprocessing import Pool
from exp import HERE, L
from exp_gorica import G, G2
from decks import with_bf
import plans as P
import ai


N = int(sys.argv[1]) if len(sys.argv) > 1 else 400
SEED0 = int(sys.argv[2]) if len(sys.argv) > 2 else 70000
BUDGET = float(sys.argv[3]) if len(sys.argv) > 3 else 1e9

JOBS = [  # label, deck Akali, plan Akali, plan LeBlanc
    ("G2, aucun plan (référence)", G2, None, None),
    ("G2 plan moteur vs LeBlanc Deathknell", G2, "Gorica", "Deathknell"),
    ("G2 plan agressif vs LeBlanc Deathknell", G2, "Gorica agressif", "Deathknell"),
    ("G2 sans plan vs LeBlanc Deathknell", G2, None, "Deathknell"),
    ("G2 plan agressif vs LeBlanc sans plan", G2, "Gorica agressif", None),
    ("Gorica stock plan agressif vs LeBlanc Deathknell", G, "Gorica agressif", "Deathknell"),
    ("G2 sans plan vs LeBlanc Hook tempo", G2, None, "Hook tempo"),
    ("G2 plan moteur vs LeBlanc Hook tempo", G2, "Gorica", "Hook tempo"),
    ("G2 plan agressif vs LeBlanc Hook tempo", G2, "Gorica agressif", "Hook tempo"),
    ("G2 plan moteur vs LeBlanc Hook tempo Windswept", G2, "Gorica", "Hook tempo Windswept"),
    ("G2 plan agressif vs LeBlanc Hook tempo Windswept", G2, "Gorica agressif", "Hook tempo Windswept"),
]


def one(arg):
    i, a_deck, ap, lp = arg
    from game import Game, Obj, Item
    Obj._n = 0; Item._n = 0
    r = random.Random(i * 7919)
    f = r.randrange(2)
    pa = P.PLANS[ap]() if ap else None
    pl = P.PLANS[lp]() if lp else None
    lbf = (pl.battlefield(L, f == 1) if pl else None) or r.choice(L["battlefields"])
    abf = (pa.battlefield(a_deck, f == 0, lbf) if pa else None) or r.choice(a_deck["battlefields"])
    ag = [P.PlanAgent(i, plan=pa, opp_plan=pl), P.PlanAgent(i + 500000, plan=pl, opp_plan=pa)]
    g = Game([with_bf(a_deck, abf), with_bf(L, lbf)], ag, seed=i, first=f)
    try:
        while True:
            d = g.advance()
            if d is None:
                break
            g.apply(ag[d.player].decide(g, d))
    except Exception:
        import traceback
        return dict(err=traceback.format_exc()[-700:], seed=i)
    return dict(w=g.winner, turns=g.turn_no, first=f, abf=abf, lbf=lbf, seed=i)


def suffix():
    """IA « gagner à tout prix » (ai.TEMPO) et LeBlanc « réel » (Reflet gardé, cartes cachées : plans.KEEP_REFLECTION)."""
    return ((" · IA tempo" if ai.TEMPO else "") + (" · LeBlanc réel" if P.KEEP_REFLECTION else "")
            + (" · vidéos" if P.VIDEO else ""))


def run(job):
    label, d, ap, lp = job
    label += suffix()
    with Pool(os.cpu_count()) as p:
        out = p.map(one, [(SEED0 + k, d, ap, lp) for k in range(N)], chunksize=2)
    good = [o for o in out if not o.get("err")]
    errs = [o for o in out if o.get("err")]
    w = [1.0 if o["w"] == 0 else 0.5 if o["w"] == -1 else 0.0 for o in good]
    wr = sum(w) / max(1, len(w))
    by = {k: round(sum(1 for o in good if o["first"] == fi and o["w"] == 0) / max(1, sum(1 for o in good if o["first"] == fi)), 3)
          for k, fi in (("akali_first", 0), ("leblanc_first", 1))}
    return stamp(dict(label=label, n=len(good), wr=round(wr, 4), se=round((wr * (1 - wr) / max(1, len(good))) ** .5, 4),
                turns=round(sum(o["turns"] for o in good) / max(1, len(good)), 1), errors=len(errs),
                err=errs[0]["err"] if errs else None, by_first=by, games=N, seed0=SEED0,
                per_seed=[[o["seed"], w[j]] for j, o in enumerate(good)]))


if __name__ == "__main__":
    f = HERE / "results_plans.json"
    done = json.load(open(f)) if f.exists() else []
    have = {(r["label"], r["seed0"]) for r in done if r.get("games") == N}
    t0 = time.time()
    only = os.environ.get("ONLY")                  # ex. ONLY=6,7,8 : indices de JOBS à jouer
    for j, job in enumerate(JOBS):
        if only and str(j) not in only.split(","):
            continue
        if (job[0] + suffix(), SEED0) in have or time.time() - t0 > BUDGET:
            continue
        t = time.time()
        r = run(job)
        done.append(r)
        json.dump(done, open(f, "w"), ensure_ascii=False)
        print(f"{r['label']:50s} {r['wr']:6.1%} +-{r['se']:.3f} {r['turns']:5.1f} tours err={r['errors']} ({round(time.time()-t)}s)", flush=True)
    print("ALL DONE", flush=True)
