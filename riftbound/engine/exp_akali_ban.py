#!/usr/bin/env python3
"""Akali Gorica après le ban du 2026-11-23 (Stellacorn Herder et Zhonya's Hourglass bannis) : remplacer ces 6 cartes.
python3 exp_akali_ban.py <parties> <graine0> [noms...] -> results_akali_ban.json (une ligne par build et par graine0).
Adversaire : LeBlanc IQ#5 tel quel (il garde Baited Hook et Star Spring, eux aussi bannis : aucune liste LeBlanc d'après le
ban n'existe encore). Mêmes graines pour tous les builds d'un lot : comparer par paires (per_seed), jamais regrouper deux lots
joués sur les mêmes graines. Candidats tirés des cartes jouées par les autres listes Akali (decks/card_frequency.csv)."""
from version import stamp
import json, sys
import exp
from decks import load, edit

G = load("akali_gorica_rq-singapore")
BAN = [("Stellacorn Herder", 3), ("Zhonya's Hourglass", 3)]
BUILDS = {
    "stock (illégal, référence)": G,
    "A +3 Silent +3 Crab": edit(G, out=BAN, add=[("Akali, Silent", 3), ("Scuttle Crab", 3)]),
    "B +3 Silent +3 Poro": edit(G, out=BAN, add=[("Akali, Silent", 3), ("Lonely Poro", 3)]),
    "C +3 Adaptatron +3 Crab": edit(G, out=BAN, add=[("Adaptatron", 3), ("Scuttle Crab", 3)]),
    "D +3 Silent +2 Crab +1 Forerunner": edit(G, out=BAN, add=[("Akali, Silent", 3), ("Scuttle Crab", 2),
                                                               ("Ferrous Forerunner", 1)]),
    "E +3 Irelia Fervent +3 Crab": edit(G, out=BAN, add=[("Irelia, Fervent", 3), ("Scuttle Crab", 3)]),
    "F +3 Silent +2 Crab +1 Sterak": edit(G, out=BAN, add=[("Akali, Silent", 3), ("Scuttle Crab", 2),
                                                           ("Sterak's Gage", 1)]),
}

if __name__ == "__main__":
    n, off = int(sys.argv[1]), int(sys.argv[2])
    names = sys.argv[3:] or list(BUILDS)
    f = exp.HERE / "results_akali_ban.json"
    done = json.load(open(f)) if f.exists() else []
    for k in names:
        if any(r["label"] == k and r.get("off") == off and r["n"] + r["errors"] == n for r in done):
            continue
        r = stamp(exp.run_job((k, BUILDS[k], exp.L, None, None, None, {}), n, offset=off))
        r["off"] = off
        done.append(r)
        json.dump(done, open(f, "w"), indent=1, ensure_ascii=False)
        print(f"{k:38s} {r['wr']:6.1%} ±{r['se']:.3f} n={r['n']} err={r['errors']}", flush=True)
