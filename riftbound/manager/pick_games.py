#!/usr/bin/env python3
"""Choisit 2 à 4 parties marquantes d'une session pour le viewer : python3 pick_games.py <NNN>.
Écrit results/session_NNN_picks.json (format convenu avec le fil Replays : id, seed, title, note, job).
Règle : une victoire et une défaite de la référence, puis, pour les 2 jobs à l'écart apparié le plus grand,
une graine où ce job et la référence divergent dans le sens de l'écart (la partie qui « montre » l'effet)."""
import json, os, sys, random
from pathlib import Path
import run_session as R

n = int(sys.argv[1])
d = json.load(open(R.HERE / "results" / f"session_{n:03d}.json"))
res = [r for r in d["results"] if not r["job"].get("fresh") and r["job"].get("seed0") is None]
ref = res[0]
rng = random.Random(n)
picks = []


def add(job, seed, title, note):
    picks.append(dict(id=f"s{n:03d}-{len(picks)+1}", seed=seed, title=title, note=note, job=job, session=n,
                      plans_snapshot=f"results/plans_session_{n:03d}/plans.py",
                      tempo=os.environ.get("RB_TEMPO", "1") != "0",
                      refl=os.environ.get("RB_REFL", "1") != "0",
                      video=os.environ.get("RB_VIDEO", "1") != "0"))


refw = {s: (w, abf, lbf) for s, w, abf, lbf in ref["per_seed"]}
wins = [s for s, v in refw.items() if v[0] == 1.0]
losses = [s for s, v in refw.items() if v[0] == 0.0]
if wins:
    s = rng.choice(wins); add(ref["job"], s, "Référence : victoire", f"{ref['label']} ({refw[s][1]} contre {refw[s][2]}). Référence de la session à {ref['wr']:.1%}.")
if losses:
    s = rng.choice(losses); add(ref["job"], s, "Référence : défaite", f"{ref['label']} ({refw[s][1]} contre {refw[s][2]}).")
scored = []
for r in res[1:]:
    p = R.paired(r, ref)
    if p and p[0] != 0:
        scored.append((abs(p[0]), r, p))
for _, r, p in sorted(scored, key=lambda x: -x[0])[:2]:
    want = (1.0, 0.0) if p[0] > 0 else (0.0, 1.0)   # (résultat du job, résultat de la référence)
    cand = [s for s, w, *_ in r["per_seed"] if s in refw and (w, refw[s][0]) == want]
    if cand:
        s = rng.choice(cand)
        sens = "gagne" if p[0] > 0 else "perd"
        add(r["job"], s, f"{r['label']} : {sens} là où la référence {'perd' if p[0] > 0 else 'gagne'}",
            f"Même donne que la référence. Écart apparié sur la session : {p[0]*100:+.1f} ± {p[1]*100:.1f} pts.")
out = R.HERE / "results" / f"session_{n:03d}_picks.json"
json.dump(picks, open(out, "w"), ensure_ascii=False, indent=1)
for p in picks:
    print(p["id"], p["seed"], p["title"])
