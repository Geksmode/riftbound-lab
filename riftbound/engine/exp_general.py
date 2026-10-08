#!/usr/bin/env python3
"""Banc d'essai de l'IA générale : decks légaux au hasard, version A contre version B.

    python3 exp_general.py <paires> <graine0> <versionA> <versionB> [processus]

Une « paire » = deux decks légaux tirés au hasard (graine g) joués DEUX fois avec la même graine de partie et le
même joueur qui commence : A tient le deck 1 dans la partie 1, B tient le deck 1 dans la partie 2 (les places
sont donc échangées). Le score d'une paire pour A = moyenne des deux parties (1 victoire, 0,5 nul, 0 défaite) ;
l'écart-type est celui de la moyenne sur les paires (les deux parties d'une paire ne sont PAS indépendantes).
Versions : voir VERSIONS (nom -> (fabrique de plan, samples, horizon)). « base » = IA générique (aucun plan).
Résultats ajoutés à results_general.json (une entrée par exécution, avec la graine0 : graines neuves = autre graine0).
Temps par coup : moyenne et p95 des appels decide() de chaque version.
"""
from version import stamp
import json, sys, time, os, random, math, traceback
from multiprocessing import Pool
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import train as T
import plans as P
from game import Game, Obj, Item
from decks import with_bf

CAT = json.loads(T.catalog())["cards"]
LEG = [c for c in CAT if c["t"] == "Legend" and c["m"]]
BFS = sorted(c["n"] for c in CAT if c["t"] == "Battlefield" and c["m"] and not c["ban"])


def rdeck(rng):
    """Deck légal au hasard (même procédé que train/t_rand.py)."""
    for _ in range(50):
        lg = rng.choice(LEG)
        dom = set(lg["d"])
        ch = [c for c in CAT if c["t"] == "Unit" and c["s"] == "Champion" and c["m"] and set(c["tg"]) & set(lg["tg"])
              and set(c["d"]) <= dom]
        pool = [c["n"] for c in CAT if c["t"] in ("Unit", "Spell", "Gear") and c["m"] and set(c["d"]) <= dom
                and c["s"] not in ("Champion", "Signature") and not c["ban"] and "Unique" not in c["tx"]]
        if not ch or len(pool) < 14:
            continue
        main = []
        while len(main) < 39:
            n = rng.choice(pool)
            if main.count(n) < 3:
                main.append(n)
        d = dict(name="r", legend=lg["n"], champion=rng.choice(ch)["n"], main=main,
                 runes=[lg["d"][0] + " Rune"] * 6 + [lg["d"][-1] + " Rune"] * 6, battlefields=rng.sample(BFS, 3))
        if not [x for x in T.validate(d) if not x.startswith("⚠")]:
            return T._deck(d, None)
    raise RuntimeError("pas de deck")


def _general(deck):
    import plans_general as PG
    return PG.GeneralPlan(deck)


# nom -> (fabrique(deck) -> plan, samples, horizon)
VERSIONS = {
    "base": (lambda d: P.Plan(), 1, 2),
    "base2": (lambda d: P.Plan(), 1, 2),            # identique à base : sert à mesurer le biais de place
    "general": (_general, 1, 2),
}


class Timed:
    """Enveloppe qui mesure le temps de chaque decide()."""
    def __init__(s, ag, acc):
        s.ag, s.acc = ag, acc

    def __getattr__(s, k):
        return getattr(s.ag, k)

    def decide(s, g, d):
        t = time.perf_counter()
        r = s.ag.decide(g, d)
        if len(d.options) > 1:
            s.acc.append(time.perf_counter() - t)
        return r


def play(seed, da, db, first, va, vb, slot=0):
    """Une partie : va tient da (joueur 0), vb tient db (joueur 1). Renvoie (gagnant 0/1/-1, tours, temps va, temps vb)."""
    Obj._n = 0
    Item._n = 0
    r = random.Random(seed * 7919)
    plans, tm = [], [[], []]
    fa, sa, ha = VERSIONS[va]
    fb, sb, hb = VERSIONS[vb]
    pa, pb = fa(da), fb(db)
    bfa = pa.battlefield(da, first == 0, None) or r.choice(da["battlefields"])
    bfb = pb.battlefield(db, first == 1, bfa) or r.choice(db["battlefields"])
    # graine de l'agent liée à la version (A : +100, B : +500100), pas au siège : sinon deux versions identiques
    # rejouent exactement la même partie en miroir et le test « 50 % » ne dit rien
    ag = [P.PlanAgent(seed + (100 if slot == 0 else 500100), plan=pa, opp_plan=pb, samples=sa, horizon=ha),
          P.PlanAgent(seed + (500100 if slot == 0 else 100), plan=pb, opp_plan=pa, samples=sb, horizon=hb)]
    wr = [Timed(ag[0], tm[0]), Timed(ag[1], tm[1])]
    g = Game([with_bf(da, bfa), with_bf(db, bfb)], wr, seed=seed, first=first)
    while True:
        d = g.advance()
        if d is None:
            break
        g.apply(wr[d.player].decide(g, d))
    return g.winner, g.turn_no, tm


def pair(arg):
    gs, va, vb = arg
    rng = random.Random(gs)
    da, db = rdeck(rng), rdeck(rng)
    first = rng.randrange(2)
    try:
        w1, t1, tm1 = play(gs, da, db, first, va, vb)      # A tient deck 1 (joueur 0)
        w2, t2, tm2 = play(gs, da, db, first, vb, va, slot=1)      # B tient deck 1 : A tient deck 2 (joueur 1)
    except Exception:
        return dict(seed=gs, err=traceback.format_exc()[-800:])
    sc = lambda w, me: 0.5 if w in (-1, None) else (1.0 if w == me else 0.0)
    a1, a2 = sc(w1, 0), sc(w2, 1)
    return dict(seed=gs, legends=[da["legend"], db["legend"]], first=first, a1=a1, a2=a2, score=(a1 + a2) / 2,
                turns=[t1, t2], tA=tm1[0] + tm2[1], tB=tm1[1] + tm2[0])


def pct(xs, q):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(q * len(xs)))] if xs else float("nan")


def run(n, seed0, va, vb, procs=2):
    t0 = time.time()
    with Pool(procs) as p:
        out = p.map(pair, [(seed0 + k, va, vb) for k in range(n)], chunksize=1)
    good = [o for o in out if "err" not in o]
    errs = [o for o in out if "err" in o]
    sc = [o["score"] for o in good]
    m = sum(sc) / max(1, len(sc))
    var = sum((x - m) ** 2 for x in sc) / max(1, len(sc) - 1)
    se = math.sqrt(var / max(1, len(sc)))
    ta = [x for o in good for x in o["tA"]]
    tb = [x for o in good for x in o["tB"]]
    # biais de place : A gagne-t-il plus quand il tient le deck 1 / le joueur 0 ?
    s1 = sum(o["a1"] for o in good) / max(1, len(good))
    s2 = sum(o["a2"] for o in good) / max(1, len(good))
    first_wr = sum((1.0 if o["a1"] == 1 and o["first"] == 0 else 0) + (1.0 if o["a2"] == 0 and o["first"] == 0 else 0)
                   for o in good)    # parties gagnées par le joueur 0 quand il commence (information)
    res = dict(A=va, B=vb, pairs=len(good), seed0=seed0, score_A=round(m, 4), se=round(se, 4),
               z=round((m - 0.5) / se, 2) if se > 0 else None,
               A_as_player0=round(s1, 4), A_as_player1=round(s2, 4),
               ms_per_decision=dict(A_mean=round(1000 * sum(ta) / max(1, len(ta)), 1), A_p95=round(1000 * pct(ta, .95), 1),
                                    B_mean=round(1000 * sum(tb) / max(1, len(tb)), 1), B_p95=round(1000 * pct(tb, .95), 1)),
               turns_mean=round(sum(sum(o["turns"]) for o in good) / max(1, 2 * len(good)), 1),
               errors=len(errs), err=errs[0]["err"] if errs else None, seconds=round(time.time() - t0),
               per_pair=[[o["seed"], o["score"]] for o in good], engine=os.popen("git -C %s rev-parse --short HEAD" % HERE).read().strip())
    stamp(res)
    return res


if __name__ == "__main__":
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 20
    seed0 = int(sys.argv[2]) if len(sys.argv) > 2 else 100000
    va = sys.argv[3] if len(sys.argv) > 3 else "base"
    vb = sys.argv[4] if len(sys.argv) > 4 else "base2"
    procs = int(sys.argv[5]) if len(sys.argv) > 5 else 2
    r = run(n, seed0, va, vb, procs)
    f = HERE / "results_general.json"
    done = json.load(open(f)) if f.exists() else []
    done.append(r)
    json.dump(done, open(f, "w"), ensure_ascii=False)
    print(f"{va} contre {vb}: {r['score_A']:.1%} ± {r['se']:.3f} (z={r['z']}) sur {r['pairs']} paires, "
          f"joueur0 {r['A_as_player0']:.2f} / joueur1 {r['A_as_player1']:.2f}, ms/coup {r['ms_per_decision']}, "
          f"erreurs {r['errors']}, {r['seconds']}s")
    if r["err"]:
        print(r["err"])
