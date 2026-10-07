#!/usr/bin/env python3
"""Enregistre un lot de parties Gorica G2 vs LeBlanc (IQ#5) : python3 batch_replays.py <n> <seed0> <dossier>.
Ecrit un JSON par partie dans <dossier> et un index.json (résumés) pour choisir les parties à regarder."""
import json, sys, os
from multiprocessing import Pool
from pathlib import Path

N, S0, DIR = int(sys.argv[1]), int(sys.argv[2]), Path(sys.argv[3])


def job(seed):
    sys.argv = sys.argv[:1]
    from replay import record, summary
    from exp_gorica import G2
    from exp import L
    try:
        rep = record(seed, G2, L)
    except Exception as e:
        return dict(seed=seed, err=repr(e)[:300])
    json.dump(rep, open(DIR / f"seed_{seed}.json", "w"), ensure_ascii=False, separators=(",", ":"))
    return summary(rep)


if __name__ == "__main__":
    DIR.mkdir(parents=True, exist_ok=True)
    with Pool(os.cpu_count()) as p:
        out = p.map(job, range(S0, S0 + N), chunksize=1)
    json.dump(out, open(DIR / "index.json", "w"), ensure_ascii=False, indent=0)
    good = [o for o in out if "err" not in o]
    w = sum(o["winner"] == 0 for o in good)
    print(f"{len(good)} parties, Akali gagne {w} ({w/len(good):.1%}), erreurs {len(out)-len(good)}")
