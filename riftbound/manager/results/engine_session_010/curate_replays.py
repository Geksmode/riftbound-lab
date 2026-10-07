#!/usr/bin/env python3
"""Ré-enregistre la sélection de parties commentées (Gorica G2 vs LeBlanc IQ#5) dans ../replays/games/.
Les graines viennent d'un lot de 240 parties neuves (graines 60000-60239, batch_replays.py) ; les parties
sont déterministes (compteurs d'identifiants remis à zéro), donc la même graine redonne la même partie."""
import json, sys
from multiprocessing import Pool
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "replays" / "games"
PICKS = [
    ("tempo", 60044, "Akali commence, 9-1", "Initiative : Mournful Witness prend Star Spring au tour 3, Shuriken Flip tue Karthus et ouvre Sigil. LeBlanc ne reprend jamais la main."),
    ("etouffee", 60139, "LeBlanc commence, 1-8", "La défaite type en jouant second : Watchful Sentry prend les deux battlefields au tour 3, Sacrifice ramène Karthus, Windswept Hillock lui laisse reprendre le terrain."),
    ("second", 60234, "Akali joue second, 8-1", "Réponse au départ de LeBlanc : Akali reprend Dusk Rose Lab, enchaîne Shuriken Flip vers Void Gate et cache une carte. L'IA LeBlanc reste ensuite passive."),
    ("comeback", 60099, "Retournement de 0-5 à 9-6", "Menée 0-5 face à LeBlanc, Everywhere At Once, Akali reconstruit avec Scuttle Crab, Lonely Poro et Ferrous Forerunner et gagne trois combats d'affilée au tour 10."),
    ("dernier-point", 60029, "Bloquée à 7, perdue 7-8", "Akali mène 7-3, mais le dernier point par conquête exige d'avoir scoré les deux battlefields dans le tour. Elle pioche à la place pendant que LeBlanc remonte par la tenue de Star Spring."),
    ("bras-de-fer", 60037, "Bras de fer gagné 8-7", "Chacune tient un battlefield de la mi-partie à la fin. Marai cachée et le Mech de Forerunner gardent Dusk Rose Lab, et Akali gagne la course à la tenue d'un point."),
    ("side-plan-perdu", 60020, "Tout le side plan en jeu, perdue 7-8", "Silent, Forerunner, Poro et Crab sortent tous, mais deux Hidden Blade tuent Akali puis Forerunner. LeBlanc a commencé et garde un point d'avance dans la course."),
    ("side-plan-gagne", 60068, "Silent et Forerunner, 8-2", "Akali joue second et perd Deadly Weapon tôt, puis déborde LeBlanc avec un board large : Akali, Silent, Forerunner, Kai'Sa et Noxus Hopeful."),
]


def job(pick):
    sys.argv = sys.argv[:1]
    gid, seed, title, note = pick
    from replay import record
    from exp_gorica import G2
    from exp import L
    rep = record(seed, G2, L)
    json.dump(rep, open(OUT / f"{gid}.json", "w"), ensure_ascii=False, separators=(",", ":"))
    return dict(id=gid, file=f"{gid}.json", seed=seed, title=title, note=note, winner=rep["winner"], pts=rep["pts"],
                first=rep["first"], turns=rep["turns"], bf=[p["bf"] for p in rep["players"]])


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    with Pool(4) as p:
        idx = p.map(job, PICKS, chunksize=1)
    GROUP = "Premières parties commentées (G2 sans plan, graines 60000+)"
    for o in idx:
        o.update(group=GROUP, plans=["aucun", "aucun"])
    f = OUT / "index.json"
    old = json.load(open(f)) if f.exists() else []
    ids = {o["id"] for o in idx}
    idx = [o for o in old if o["id"] not in ids] + idx          # garder les parties ajoutées par add_replays.py
    json.dump(idx, open(f, "w"), ensure_ascii=False, indent=1)
    for o in idx:
        print(o["id"], o["seed"], o["winner"], o["pts"])
