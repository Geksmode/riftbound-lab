#!/usr/bin/env python3
"""Valide un ou plusieurs fichiers de deck bruts (decks/raw/*.txt, format : en-tête « # clé: valeur » puis sections
[legend] [champion] [main] [runes] [battlefields] [sideboard], lignes « <qté> <nom> »).

Contrôles : noms connus de la base de cartes (suggestion du nom le plus proche sinon), règles de construction (103) et
du sideboard (601.1.c) via train.validate, cartes non modélisées (cards.IMPL), 12 runes, 3 battlefields.
Usage : python3 decks/check_raw.py decks/raw/jinx_*.txt     (code de sortie 1 si un deck a un problème)"""
import csv, difflib, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "engine"))
NAMES = {r["name"] for r in csv.DictReader(open(os.path.join(ROOT, "cards", "cards_unique.csv"), encoding="utf-8"))}


def parse(path):
    head, secs, sec = {}, {}, None
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if not line:
            continue
        if line.startswith("#"):
            k, _, v = line[1:].partition(":")
            head[k.strip()] = v.strip()
            continue
        m = re.match(r"^\[(\w+)\]$", line)
        if m:
            sec = m.group(1)
            secs.setdefault(sec, [])
            continue
        m = re.match(r"^(\d+)\s+(.+)$", line)
        if not m or sec is None:
            raise ValueError(f"ligne illisible : {line!r}")
        secs[sec].append((int(m.group(1)), m.group(2).strip()))
    return head, secs


def check(path):
    import train
    head, secs = parse(path)
    probs = []
    for sec, items in secs.items():
        for q, n in items:
            if n not in NAMES:
                near = difflib.get_close_matches(n, NAMES, n=1, cutoff=0.6)
                probs.append(f"[{sec}] carte inconnue « {n} »" + (f" (voulais-tu « {near[0]} » ?)" if near else ""))
    flat = lambda s: [n for q, n in secs.get(s, []) for _ in range(q)]
    deck = dict(legend=(flat("legend") or [None])[0], champion=(flat("champion") or [None])[0], main=flat("main"),
                runes=[r.replace(" Rune", "") for r in flat("runes")], battlefields=flat("battlefields"),
                sideboard=flat("sideboard"))
    if len(deck["runes"]) != 12:
        probs.append(f"runes : {len(deck['runes'])}/12")
    if len(deck["battlefields"]) != 3:
        probs.append(f"battlefields : {len(deck['battlefields'])}/3")
    if not probs:
        probs += train.validate(deck)
    return head, deck, probs


if __name__ == "__main__":
    bad = 0
    for p in sys.argv[1:]:
        try:
            head, deck, probs = check(p)
        except Exception as e:                       # noqa: BLE001
            head, deck, probs = {}, {}, [f"lecture impossible : {e}"]
        bad += bool(probs)
        print(("OK    " if not probs else "ÉCHEC ") + os.path.basename(p) +
              (f" ({deck.get('legend')}, main {len(deck.get('main', [])) + 1}, sideboard {len(deck.get('sideboard', []))})" if deck else ""))
        for x in probs:
            print("      - " + x)
    sys.exit(1 if bad else 0)
