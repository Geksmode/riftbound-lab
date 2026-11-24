"""Référence de `decks.load` pour tests/decks.rs, écrite à part du moteur Rust : lit `riftbound/decks/decks.json` comme
le faisait `decks.load` du moteur Python (supprimé le 2026-11-24 ; ce script redonne à l'octet près son ancienne sortie).
Regénérer après un ajout de deck : python3 rust/tests/gen_decks.py > rust/tests/data/decks.txt"""
import json

for d in sorted(json.load(open("riftbound/decks/decks.json", encoding="utf-8")), key=lambda d: d["file"]):
    sec = {s: [] for s in ("legend", "champion", "main", "runes", "battlefields", "sideboard")}
    for c in d["cards"]:
        name = c.get("name_db") or c["name"]
        if c["section"] == "runes":
            name = name.replace(" Rune", "")
        many = c["section"] in ("main", "runes", "sideboard")
        sec[c["section"]] += [name] * (c["qty"] if many else 1)
    print("\t".join([d["file"].replace(".txt", ""), sec["legend"][-1], (sec["champion"] or [""])[-1], "|".join(sec["main"]),
                     "|".join(sec["runes"]), sec["battlefields"][0], "|".join(sec["battlefields"]), "|".join(sec["sideboard"])]))
