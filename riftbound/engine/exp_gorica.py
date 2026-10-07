#!/usr/bin/env python3
"""Suite Gorica (Akali Heron, RQ Singapour) : python3 exp_gorica.py <parties> [budget]. Resultats : results_gorica.json."""
import json, sys, time
import exp
from exp import run_job, HERE, N, BUDGET, L, LS
from decks import load, edit

G = load("akali_gorica_rq-singapore")
GL = edit(G, out=[("Long Sword", 2)], add=[("Akali, Silent", 1), ("Ferrous Forerunner", 1)])
GD = edit(G, out=[("Defy", 3)], add=[("Ferrous Forerunner", 2), ("Lonely Poro", 1)])
G2 = edit(GL, out=[("Defy", 3)], add=[("Ferrous Forerunner", 1), ("Lonely Poro", 1), ("Scuttle Crab", 1)])
GB = edit(G2, out=[("Block", 1), ("Not So Fast", 1)], add=[("Scuttle Crab", 2)])
BUILDS = {"Gorica stock": G, "GL -2 Long Sword +Silent +Forerunner": GL,
          "GD -3 Defy +2 Forerunner +Poro": GD, "G2 -2 Long Sword -3 Defy +5 unites": G2,
          "GB G2 -Block -Not So Fast +2 Crab": GB}
JOBS = [(f"build: {k}", d, L, None, None, None, {}) for k, d in BUILDS.items()]
JOBS += [("sideboard: Gorica stock vs LeBlanc sidee", G, LS, None, None, None, {}),
         ("sideboard: G2 vs LeBlanc sidee", G2, LS, None, None, None, {})]
for bf in G["battlefields"]:
    JOBS.append((f"battlefield Akali: {bf} (G2)", G2, L, [bf], None, None, {}))
JOBS += [("confirm (graines 10000+): Gorica stock", G, L, None, None, None, {"_off": 10000}),
         ("confirm (graines 10000+): G2", G2, L, None, None, None, {"_off": 10000}),
         ("confirm (graines 10000+): GL", GL, L, None, None, None, {"_off": 10000}),
         ("confirm (graines 20000+): Gorica stock", G, L, None, None, None, {"_off": 20000}),
         ("confirm (graines 20000+): G2", G2, L, None, None, None, {"_off": 20000})]
LLA = load("leblanc_mill-twistedtcg_rq-la_12th")
JOBS += [("retex LA: Gorica stock vs LeBlanc LA", G, LLA, None, None, None, {}),
         ("retex LA: G2 vs LeBlanc LA", G2, LLA, None, None, None, {})]
for off in (30000, 40000):
    JOBS += [(f"confirm (graines {off}+): Gorica stock", G, L, None, None, None, {"_off": off}),
             (f"confirm (graines {off}+): G2", G2, L, None, None, None, {"_off": off})]
exp.JOBS = JOBS

if __name__ == "__main__":
    f = HERE / "results_gorica.json"
    done = json.load(open(f)) if f.exists() else []
    labels = {r["label"] for r in done if r.get("games") == N}
    t0, left = time.time(), 0
    for job in JOBS:
        if job[0] in labels:
            continue
        if time.time() - t0 > BUDGET:
            left += 1; continue
        t = time.time()
        off = job[6].get("_off", 0)
        r = run_job(job[:6] + ({k: v for k, v in job[6].items() if k != '_off'},), N, off)
        done.append(r)
        json.dump(done, open(f, "w"), indent=1, ensure_ascii=False)
        print(f"{r['label']:44s} {r['wr']:6.1%} +-{r['se']:.3f} {r['turns']:5.1f} tours err={r['errors']} ({round(time.time()-t)}s)", flush=True)
    print(f"RESTE {left} configurations" if left else "ALL DONE", flush=True)
