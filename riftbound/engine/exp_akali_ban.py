#!/usr/bin/env python3
"""Akali Gorica après le ban du 2026-11-23 : remplacer 3 Stellacorn Herder (« S ») et 3 Zhonya's Hourglass (« Z »).

Demande de l'utilisateur (2026-11-15) : 10 candidats par carte, puis le meilleur en jouant les parties.
  python3 exp_akali_ban.py s1 <parties>   étape 1 : 10 candidats S, Z remplacée par le bouche-trou FILL_Z (graines 710000+)
  python3 exp_akali_ban.py s2 <parties>   étape 2 : meilleur S de l'étape 1 + 10 candidats Z (graines 720000+)
  python3 exp_akali_ban.py temoin <parties>  témoin de s3 (2 meilleurs S, Zhonya -> +2 Not So Fast +1 Block)
  python3 exp_akali_ban.py s3 <parties>   étape 3 : 2 meilleurs S x 2 meilleurs Z, graines NEUVES (730000+)
  python3 exp_akali_ban.py rapport        tableau : taux de victoire et écart apparié au meilleur de chaque étape
Résultats : results_akali_ban.json. Adversaire : LeBlanc IQ#5 tel quel (garde Baited Hook et Star Spring, bannis aussi :
aucune liste LeBlanc d'après le ban n'existe). IA de recherche sans plan des deux côtés (le plan Gorica est construit
autour de Stellacorn Herder et favoriserait les builds qui la gardent). Dans une étape, tous les builds jouent les mêmes
graines : on compare par paires ; on ne regroupe jamais deux étapes. Interactions S x Z : seulement à l'étape 3."""
from version import stamp
import json, sys
ARGS, sys.argv = sys.argv[1:], sys.argv[:1]   # exp.py lit sys.argv à l'import
import exp
from decks import load, edit

G = load("akali_gorica_rq-singapore")
S_OUT, Z_OUT = ("Stellacorn Herder", 3), ("Zhonya's Hourglass", 3)
# 10 candidats S (unités, moteur de déplacement / carte en plus) et 10 candidats Z (protection, interaction peu chère)
CAND_S = {
    "Akali, Silent": [("Akali, Silent", 3)], "Akali, Brash": [("Akali, Brash", 3)],
    "Pakaa Protector": [("Pakaa Protector", 3)], "Scuttle Crab": [("Scuttle Crab", 3)],
    "Adaptatron": [("Adaptatron", 3)], "Irelia, Fervent": [("Irelia, Fervent", 3)],
    "Ahri, Confident": [("Ahri, Confident", 3)], "Jhin, Murderous Artist": [("Jhin, Murderous Artist", 3)],
    "Ferrous Forerunner": [("Ferrous Forerunner", 3)], "Lonely Poro": [("Lonely Poro", 3)],
}
CAND_Z = {
    "Unlicensed Armory": [("Unlicensed Armory", 3)], "Twilight Shroud": [("Twilight Shroud", 3)],
    "Stoic Ferryman": [("Stoic Ferryman", 3)], "Consult the Heavens": [("Consult the Heavens", 3)],
    "Spirit's Refuge": [("Spirit's Refuge", 3)], "Scuttle Crab": [("Scuttle Crab", 3)],
    "Charm": [("Charm", 3)], "En Garde": [("En Garde", 3)], "Hush": [("Hush", 3)],
    "+2 Not So Fast +1 Block": [("Not So Fast", 2), ("Block", 1)],
}
FILL_Z = "+2 Not So Fast +1 Block"   # bouche-trou de l'étape 1 : cartes déjà dans la liste, aucun conflit avec les S
OFF = {"s1": 710000, "s2": 720000, "s3": 730000}
F = exp.HERE / "results_akali_ban.json"


def build(s, z):
    return edit(G, out=[S_OUT, Z_OUT], add=CAND_S[s] + CAND_Z[z])


def rows(stage):
    return [r for r in (json.load(open(F)) if F.exists() else []) if r.get("stage") == stage]


def ranked(stage):
    return sorted(rows(stage), key=lambda r: -r["wr"])


def run(stage, jobs, n):
    done = json.load(open(F)) if F.exists() else []
    for label, s, z in jobs:
        if any(r.get("stage") == stage and r["label"] == label and r["n"] + r["errors"] == n for r in done):
            continue
        r = stamp(exp.run_job((label, build(s, z), exp.L, None, None, None, {}), n, offset=OFF[stage]))
        r.update(stage=stage, s=s, z=z)
        done.append(r)
        json.dump(done, open(F, "w"), indent=1, ensure_ascii=False)
        print(f"[{stage}] {label:55s} {r['wr']:6.1%} ±{r['se']:.3f} n={r['n']} err={r['errors']}", flush=True)


def paired(a, b):
    pa, pb = dict(map(tuple, a["per_seed"])), dict(map(tuple, b["per_seed"]))
    d = [pa[k] - pb[k] for k in sorted(set(pa) & set(pb))]
    m = sum(d) / len(d)
    sd = (sum((x - m) ** 2 for x in d) / (len(d) - 1)) ** .5
    return m, sd / len(d) ** .5


def rapport():
    for st in ("s1", "s2", "s3"):
        rs = ranked(st)
        if not rs:
            continue
        print(f"\n== {st} ({rs[0]['n']} parties par build, mêmes graines) : écart au 1er, apparié")
        for r in rs:
            m, se = paired(r, rs[0]) if r is not rs[0] else (0.0, 0.0)
            tag = "" if r is rs[0] else ("  (sous 2 écarts-types : on ne sait pas)" if abs(m) < 2 * se else "")
            print(f"  {r['label']:55s} {r['wr']:6.1%} ±{r['se']*100:.1f}   {m*100:+5.1f} ±{se*100:.1f}{tag}")


if __name__ == "__main__":
    st = ARGS[0]
    if st == "rapport":
        rapport()
    elif st == "s1":
        run("s1", [(f"S={s} | Z={FILL_Z}", s, FILL_Z) for s in CAND_S], int(ARGS[1]))
    elif st == "s2":
        best_s = ranked("s1")[0]["s"]
        run("s2", [(f"S={best_s} | Z={z}", best_s, z) for z in CAND_Z
                   if not (z == "Scuttle Crab" and best_s == "Scuttle Crab")], int(ARGS[1]))
    elif st == "s3":
        ss = [r["s"] for r in ranked("s1")[:2]]
        zs = [r["z"] for r in ranked("s2")[:2]]
        run("s3", [(f"S={s} | Z={z}", s, z) for s in ss for z in zs if not (s == z == "Scuttle Crab")],
            int(ARGS[1]))
    elif st == "temoin":   # témoin sur les graines de s3 : sans 3e carte ajoutée pour Zhonya (+2 Not So Fast +1 Block)
        ss = [r["s"] for r in ranked("s1")[:2]]
        run("s3", [(f"S={s} | Z={FILL_Z}", s, FILL_Z) for s in ss], int(ARGS[1]))
