"""decks.load de référence pour tests/decks.rs. Regénérer : python3 rust/tests/gen_decks.py > rust/tests/data/decks.txt"""
import sys
sys.path.insert(0, "riftbound/engine")
import decks
for k in sorted(decks.DECKS):
    d = decks.load(k)
    print("\t".join([k, d["legend"], d["champion"] or "", "|".join(d["main"]), "|".join(d["runes"]), d["battlefield"],
                     "|".join(d["battlefields"]), "|".join(d["sideboard"])]))
