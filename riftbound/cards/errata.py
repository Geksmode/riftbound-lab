"""Errata officiels absents de la source des cartes (api.rifthunt.com) : (nom, ancien morceau de texte, nouveau).

Errata du 2026-08-03 (riftbound.gg « Vendetta Errata Updates », FAQ Vendetta de playriftbound.com du 2026-08-14 pour
Astral Heron), signalés par l'utilisateur le 2026-10-11. Appliqués par build_cards.py ; `python3 errata.py` met à jour
les fichiers déjà construits (idempotent)."""
import csv, json, os

ERRATA = [
    ("Draven, Vanquisher", "you may pay 1 fury rune. If you do, give me", "you may pay 1 fury rune to give me"),
    ("Emperor's Dais", "to its owner's hand. If you do, play", "to its owner's hand to play"),
    ("Fizz, Trickster", "Recycle that spell after you play it.", "Then recycle it."),
    ("Diana, Lunari", "pay 1 energy. If you do, [Predict],", "pay 1 energy to [Predict],"),
    ("Stalking Wolf", "You may play me to its battlefield (even if you don't have other units there).",
     "You may [Ambush] me to its battlefield, even if you don't have other units there."),
    ("Astral Heron", "your next card costs", "the next card you play this turn costs"),
    ("Gangplank, Naval", "give me +3 might instead.", "give me +3 might this turn instead."),
    ("Resonating Strike", "[Reaction] (Play on your turn or in showdowns.)",
     "[Reaction] (Play any time, even before spells and abilities resolve.)"),
]


def apply(name, text):
    for n, old, new in ERRATA:
        if n == name and old in text:
            text = text.replace(old, new)
    return text


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    for fn in ("cards_unique.csv", "cards_all_printings.csv"):
        p = os.path.join(here, fn)
        with open(p, newline="", encoding="utf-8") as f:
            rd = csv.DictReader(f)
            cols, rows = rd.fieldnames, list(rd)
        for r in rows:
            r["text"] = apply(r["name"], r["text"])
        with open(p, "w", newline="", encoding="utf-8") as f:
            wr = csv.DictWriter(f, cols)
            wr.writeheader()
            wr.writerows(rows)
    p = os.path.join(here, "cards_all_printings.json")
    rows = json.load(open(p, encoding="utf-8"))
    for r in rows:
        r["text"] = apply(r["name"], r["text"])
    json.dump(rows, open(p, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    for n, old, new in ERRATA:
        ok = any(r["name"] == n and new in r["text"] for r in rows)
        print(("OK  " if ok else "ABSENT ") + n)
