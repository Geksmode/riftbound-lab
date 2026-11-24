"""game.SPEC de référence pour tests/spec.rs. Regénérer : python3 rust/tests/gen_spec.py > rust/tests/data/spec.txt"""
import sys
sys.path.insert(0, "riftbound/engine")
from game import SPEC
for n in sorted(SPEC):
    s = SPEC[n]
    print("\t".join([n, s["type"], s["super"], "|".join(s["domains"]), str(s["e"]), str(s["p"]),
                     "" if s["might"] is None else str(s["might"]), "|".join(sorted(s["tags"])),
                     "|".join(s["keywords"]), s["text"].replace("\n", "\\n").replace("\t", "\\t")]))
