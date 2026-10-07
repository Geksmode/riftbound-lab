"""Load the tournament lists from decks/decks.json into engine deck dicts."""
import json
from pathlib import Path

ROOT = Path('/mnt/attach/project-files/riftbound')
DECKS = {d["file"].replace(".txt", ""): d for d in json.load(open(ROOT / "decks" / "decks.json", encoding="utf-8"))}


def load(key, battlefield=None):
    d = DECKS[key]
    out = dict(legend=None, champion=None, main=[], runes=[], battlefields=[], sideboard=[])
    for c in d["cards"]:
        n = c.get("name_db") or c["name"]
        sec = c["section"]
        if sec == "legend":
            out["legend"] = n
        elif sec == "champion":
            out["champion"] = n
        elif sec == "main":
            out["main"] += [n] * c["qty"]
        elif sec == "runes":
            out["runes"] += [n.replace(" Rune", "")] * c["qty"]
        elif sec == "battlefields":
            out["battlefields"].append(n)
        elif sec == "sideboard":
            out["sideboard"] += [n] * c["qty"]
    out["battlefield"] = battlefield or out["battlefields"][0]
    out["key"] = key
    return out


def edit(deck, out=(), add=(), battlefield=None):
    d = dict(deck)
    main = list(deck["main"])
    for n, q in out:
        for _ in range(q):
            main.remove(n)
    for n, q in add:
        main += [n] * q
    d["main"] = main
    if battlefield:
        d["battlefield"] = battlefield
    return d


def with_bf(deck, bf):
    d = dict(deck)
    d["battlefield"] = bf
    return d
