"""Valeurs de référence des aides pures de cards.py et actions.py pour tests/helpers.rs.
Regénérer : python3 rust/tests/gen_helpers.py > rust/tests/data/helpers.txt

Lignes (champs séparés par une tabulation) :
  cost    <texte>  <parse_cost>  <power_list>  <repeat_cost>  <flow_cost>
  kw      <nom>    <keywords_of> <parse_keywords>  <auto_keywords non None>  <is_token>
  drop    <reqs>   <n>  <_drop_reqs>
  flex    <variants>    <_flex>
Un req s'écrit avec ses domaines triés séparés par ',', les reqs par ';', une variante 'e:reqs', les variantes par '/'.
"""
import random
import sys

sys.path.insert(0, "riftbound/engine")
import cards  # noqa: E402
from actions import _drop_reqs, _flex  # noqa: E402
from game import DOMAINS, SPEC  # noqa: E402


def esc(s):
    return s.replace("\\", "\\\\").replace("\n", "\\n").replace("\t", "\\t")


def req(r):
    return ",".join(sorted(r))


def reqs(rs):
    return ";".join(req(r) for r in rs)


def variants(vs):
    return "/".join(f"{e}:{reqs(r)}" for e, r in vs)


def fields(f):
    if f is None:
        return "None"
    out = []
    for k in sorted(f):
        v = f[k]
        if k == "kw":
            v = ",".join(f"{a}={b}" for a, b in v.items())
        out.append(f"{k}={v}")
    return " ".join(out)


texts = [SPEC[n]["text"] for n in sorted(SPEC)]
texts += ["", "2 Energy and 1 FURY RUNES", "1 rune of any type", "3 runes of any type", "12energy", "1  calm rune",
          "1 fury runes of any type", "2 energy, 1 mind rune and 2 order runes", "[Repeat] 1 energy and 1 chaos rune",
          "4 energy and 1 body rune", "1 rune of any typo", "10 energy", "1 Runes Of Any Type", "0 energy"]
for t in texts:
    e, r = cards.parse_cost(t)
    pe, pl = cards.power_list(t)
    re_ = cards.repeat_cost(t)
    fl = cards.flow_cost(t)
    print("\t".join(["cost", esc(t), f"{e}|{reqs(r)}", f"{pe}|{','.join(pl)}",
                     f"{re_[0]}|{','.join(re_[1])}|{re_[2]}", f"{fl[0]}|{fl[1]}|{','.join(fl[2])}"]))
for n in sorted(SPEC):
    print("\t".join(["kw", n, ",".join(sorted(cards.keywords_of(n))), fields(cards.parse_keywords(n)),
                     str(int(cards.auto_keywords(n, register=False) is not None)), str(int(cards.is_token(n)))]))

rng = random.Random(1)
pool = [frozenset(DOMAINS)] + [frozenset({d}) for d in DOMAINS] + [frozenset(rng.sample(DOMAINS, 2)) for _ in range(4)]


def rand_reqs():
    return [rng.choice(pool) for _ in range(rng.randrange(0, 5))]


for _ in range(300):
    rs = rand_reqs()
    n = rng.randrange(0, 4)
    print("\t".join(["drop", reqs(rs), str(n), reqs(_drop_reqs(rs, n))]))
for _ in range(300):
    vs = [(rng.randrange(0, 4), rand_reqs()) for _ in range(rng.randrange(1, 4))]
    print("\t".join(["flex", variants(vs), variants(_flex(vs))]))
