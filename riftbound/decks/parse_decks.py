#!/usr/bin/env python3
"""Parse raw decklists (decks/raw/*.txt) and join them to cards/cards_unique.csv.

Raw format: '# key: value' header lines, then sections [legend] [champion] [main]
[runes] [battlefields] [sideboard] with lines '<qty> <card name>'.

Outputs (in decks/):
  decks.json          one object per deck (metadata + cards with card id/type/domain/cost)
  deck_cards.csv      one row per (deck, section, card), joined to the card DB
  validation.txt      count checks (main = champion + main = 40, runes 12, bf 3, sb <= 10)
                      and card names not found in the DB
  card_frequency.csv  per legend: % of decks playing each card and avg copies
"""
import csv, json, re, unicodedata
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CARDS = ROOT.parent / "cards" / "cards_unique.csv"


def norm(s):
    s = unicodedata.normalize("NFKD", s).encode("ascii", "ignore").decode()
    return re.sub(r"[^a-z0-9]", "", s.lower())


def load_db():
    db = {}
    for r in csv.DictReader(open(CARDS, encoding="utf-8")):
        db.setdefault(norm(r["name"]), r)
    return db


def parse(path):
    meta, cards, section = {"file": path.name}, [], None
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        if line.startswith("#"):
            k, _, v = line[1:].partition(":")
            meta[k.strip()] = v.strip()
        elif line.startswith("["):
            section = line.strip("[]")
        else:
            qty, name = line.split(" ", 1)
            cards.append({"section": section, "qty": int(qty), "name": name.strip()})
    return meta, cards


def main():
    db = load_db()
    decks, rows, report = [], [], []
    for p in sorted((ROOT / "raw").glob("*.txt")):
        meta, cards = parse(p)
        missing = []
        for c in cards:
            r = db.get(norm(c["name"]))
            if r is None and c["section"] == "runes":
                r = db.get(norm(c["name"].replace(" Rune", "")))
            if r is None:
                missing.append(c["name"])
                c.update(card_id="", type="", domain="", energy_cost="", power_cost="", might="")
            else:
                c.update(card_id=r["id"], name_db=r["name"], type=r["type"], domain=r["domain"],
                         energy_cost=r["energy_cost"], power_cost=r["power_cost"], might=r["might"])
            rows.append({"deck": p.stem, "legend": meta.get("legend"), "player": meta.get("player"),
                         "event": meta.get("event"), "date": meta.get("date"),
                         "placement": meta.get("placement"), **{k: c.get(k, "") for k in
                         ("section", "qty", "name", "card_id", "type", "domain", "energy_cost", "power_cost", "might")}})
        tot = defaultdict(int)
        for c in cards:
            tot[c["section"]] += c["qty"]
        main40 = tot["champion"] + tot["main"]
        checks = {"main_deck(champion+main)": (main40, 40), "runes": (tot["runes"], 12),
                  "battlefields": (tot["battlefields"], 3)}
        flags = [f"{k}={v} (attendu {e})" for k, (v, e) in checks.items() if v != e]
        banned = [c["name"] for c in cards if c.get("card_id") and db[norm(c["name"])]["banned_standard"]]
        if banned:
            flags.append(f"cartes bannies aujourd'hui: {banned} (liste antérieure au ban)")
        if tot["sideboard"] > 10:
            flags.append(f"sideboard={tot['sideboard']} (>10)")
        meta["valid"] = not flags and not missing
        meta["flags"] = flags
        meta["missing_cards"] = missing
        decks.append({**meta, "cards": cards})
        report.append(f"{p.stem}: {'OK' if meta['valid'] else 'A VERIFIER'} " + "; ".join(flags)
                      + (f" | inconnues: {missing}" if missing else ""))

    (ROOT / "decks.json").write_text(json.dumps(decks, ensure_ascii=False, indent=1), encoding="utf-8")
    with open(ROOT / "deck_cards.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader(); w.writerows(rows)
    (ROOT / "validation.txt").write_text("\n".join(report) + "\n", encoding="utf-8")

    # card frequency per legend (main deck = champion + main; sideboard separately)
    freq = []
    by_legend = defaultdict(list)
    for d in decks:
        by_legend[d.get("legend")].append(d)
    for legend, ds in by_legend.items():
        agg = defaultdict(lambda: {"main": [], "sideboard": []})
        for d in ds:
            for c in d["cards"]:
                if c["section"] in ("champion", "main"):
                    agg[c["name"]]["main"].append(c["qty"])
                elif c["section"] == "sideboard":
                    agg[c["name"]]["sideboard"].append(c["qty"])
        n = len(ds)
        for name, a in agg.items():
            r = db.get(norm(name), {})
            freq.append({"legend": legend, "card": name, "card_id": r.get("id", ""), "type": r.get("type", ""),
                         "domain": r.get("domain", ""), "energy_cost": r.get("energy_cost", ""),
                         "decks": n, "main_pct": round(100 * len(a["main"]) / n),
                         "avg_copies_when_played": round(sum(a["main"]) / len(a["main"]), 1) if a["main"] else "",
                         "sideboard_pct": round(100 * len(a["sideboard"]) / n)})
    freq.sort(key=lambda x: (x["legend"], -x["main_pct"], -x["sideboard_pct"], x["card"]))
    with open(ROOT / "card_frequency.csv", "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=list(freq[0].keys()))
        w.writeheader(); w.writerows(freq)
    print("\n".join(report))


if __name__ == "__main__":
    main()
