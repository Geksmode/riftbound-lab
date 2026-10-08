#!/usr/bin/env python3
"""Suite d'experiences reprenable : python3 exp.py <parties> [budget_secondes].
Chaque configuration terminee est ajoutee a results.json ; relancer continue la ou on s'est arrete."""
from version import stamp
import json, sys, time, os
from pathlib import Path
from multiprocessing import Pool
from decks import load, edit, with_bf

HERE = Path(__file__).resolve().parent
N = int(sys.argv[1]) if len(sys.argv) > 1 else 160
BUDGET = float(sys.argv[2]) if len(sys.argv) > 2 else 480

A = load("akali_dongdong_wuhan-open_5th")
L = load("leblanc_gyatarina_ccs-iq5_1st")
LS = edit(L, out=[("Deathgrip", 1), ("Hidden Blade", 1)],
          add=[("LeBlanc, Everywhere At Once", 1), ("Thousand-Tailed Watcher", 1)])
C1 = edit(A, out=[("Defy", 2)], add=[("Mournful Witness", 2)])
C2 = edit(C1, out=[("Long Sword", 1)], add=[("Akali, Silent", 1)])
C3 = edit(C2, out=[("Long Sword", 1)], add=[("Ferrous Forerunner", 1)])
C5 = edit(C3, out=[("Defy", 1)], add=[("Lonely Poro", 1)])
NSF = edit(C5, out=[("En Garde", 1)], add=[("Not So Fast", 1)])
STEEL = edit(C5, out=[("Adaptatron", 1)], add=[("Brittle Steel", 1)])
SILENT2 = edit(C5, out=[("Astral Heron", 1)], add=[("Akali, Silent", 1)])
BUILDS = {"stock Wuhan": A, "C1 -2 Defy +2 Witness": C1, "C3 +Silent +Forerunner": C3,
          "C5 -3 Defy +Poro": C5, "C5 +Not So Fast": NSF, "C5 +Brittle Steel": STEEL,
          "C5 +2e Silent -Heron": SILENT2}

JOBS = []
for name, d in BUILDS.items():
    JOBS.append((f"build: {name}", d, L, None, None, None, {}))
for bf in ("Void Gate", "Targon's Peak", "Forgotten Monument", "Sigil of the Storm"):
    JOBS.append((f"battlefield Akali: {bf}", C5, L, [bf], None, None, {}))
for bf in L["battlefields"]:
    JOBS.append((f"battlefield LeBlanc: {bf}", C5, L, None, [bf], None, {}))
for cfg, label in (({"mulligan": "aggro"}, "mulligan agressif"), ({"mulligan": "none"}, "jamais de mulligan"),
                   ({"respect_ambush": True}, "prudent face a Vi")):
    JOBS.append((f"plan: {label}", C5, L, None, None, None, cfg))
JOBS.append(("ordre: Akali commence", C5, L, None, None, 0, {}))
JOBS.append(("ordre: LeBlanc commence", C5, L, None, None, 1, {}))
JOBS.append(("sideboard: C5 vs LeBlanc sidee", C5, LS, None, None, None, {}))
JOBS.append(("sideboard: stock vs LeBlanc sidee", A, LS, None, None, None, {}))
JOBS.append(("combo: C5 + Targon + mulligan agressif", C5, L, ["Targon's Peak"], None, None, {"mulligan": "aggro"}))
JOBS.append(("combo: C5 + Targon + mull. agr. vs sidee", C5, LS, ["Targon's Peak"], None, None, {"mulligan": "aggro"}))
# confirmations sur les meilleures options
JOBS.append(("confirm: C3 + Forgotten Monument", C3, L, ["Forgotten Monument"], None, None, {}))
JOBS.append(("confirm: C3 + Void Gate", C3, L, ["Void Gate"], None, None, {}))
JOBS.append(("confirm: C3 vs LeBlanc sidee", C3, LS, None, None, None, {}))
JOBS.append(("confirm: C3 + Forgotten vs LeBlanc sidee", C3, LS, ["Forgotten Monument"], None, None, {}))
JOBS.append(("confirm: stock + Forgotten Monument", A, L, ["Forgotten Monument"], None, None, {}))


def one(arg):
    i, a_deck, l_deck, a_bfs, l_bfs, first, cfg = arg
    import random
    from game import Game
    from ai import SearchAgent
    r = random.Random(i * 7919)
    Ad = with_bf(a_deck, r.choice(a_bfs))
    Ld = with_bf(l_deck, r.choice(l_bfs))
    f = r.randrange(2) if first is None else first
    ag = [SearchAgent(i, cfg=cfg), SearchAgent(i + 500000)]
    g = Game([Ad, Ld], ag, seed=i, first=f)
    try:
        while True:
            d = g.advance()
            if d is None:
                break
            g.apply(ag[d.player].decide(g, d))
    except Exception:
        import traceback
        return dict(err=traceback.format_exc()[-700:], seed=i)
    return dict(w=g.winner, turns=g.turn_no, pts=[p.points for p in g.p], first=f, seed=i)


def run_job(job, n, offset=0):
    label, d, l, abf, lbf, first, cfg = job
    args = [(i + offset, d, l, abf or d["battlefields"], lbf or l["battlefields"], first, cfg) for i in range(n)]
    with Pool(os.cpu_count()) as p:
        out = p.map(one, args, chunksize=2)
    good = [o for o in out if not o.get("err")]
    errs = [o for o in out if o.get("err")]
    wins = sum(1 for o in good if o["w"] == 0) + 0.5 * sum(1 for o in good if o["w"] == -1)
    wr = wins / max(1, len(good))
    by = {}
    for fi in (0, 1):
        sub = [o for o in good if o["first"] == fi]
        if sub:
            by["akali_first" if fi == 0 else "leblanc_first"] = round(sum(1 for o in sub if o["w"] == 0) / len(sub), 3)
    return dict(label=label, n=len(good), wr=round(wr, 4), se=round((wr * (1 - wr) / max(1, len(good))) ** .5, 4),
                turns=round(sum(o["turns"] for o in good) / max(1, len(good)), 1), errors=len(errs),
                err=errs[0]["err"] if errs else None, by_first=by, games=N,
                per_seed=[[o["seed"], 1.0 if o["w"] == 0 else 0.5 if o["w"] == -1 else 0.0] for o in good])


if __name__ == "__main__":
    f = HERE / "results.json"
    done = json.load(open(f)) if f.exists() else []
    labels = {r["label"] for r in done if r.get("games") == N}
    t0 = time.time()
    left = 0
    for job in JOBS:
        if job[0] in labels:
            continue
        if time.time() - t0 > BUDGET:
            left += 1
            continue
        t = time.time()
        r = stamp(run_job(job, N))
        done.append(r)
        json.dump(done, open(f, "w"), indent=1, ensure_ascii=False)
        print(f"{r['label']:42s} {r['wr']:6.1%} +-{r['se']:.3f} {r['turns']:5.1f} tours err={r['errors']}"
              f" ({round(time.time()-t)}s)", flush=True)
    print(f"RESTE {left} configurations" if left else "ALL DONE", flush=True)
