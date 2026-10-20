"""Tests de mesure/sprt.py sur des scores de paires simulés (pas de partie jouée).

    python3 mesure/test_sprt.py

Scores de paire 0 / 0,5 / 1 avec la dispersion mesurée (23 % de paires non nulles, 2 410 paires « defaut »)."""
import random, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sprt as S

Q = 1 - 1850 / 2410


def simulate(mu, rng, cap=4000):
    p1, p0 = Q / 2 + (mu - .5), Q / 2 - (mu - .5)
    xs = []
    while len(xs) < cap:
        u = rng.random()
        xs.append(1.0 if u < p1 else 0.0 if u < p1 + p0 else 0.5)
        v = S.decide(xs)
        if v:
            return v, len(xs)
    return None, len(xs)


def main():
    rng = random.Random(20261020)
    ok = tot = 0

    def check(name, cond, info=""):
        nonlocal ok, tot
        tot += 1
        ok += bool(cond)
        print(("ok   " if cond else "FAIL ") + name, info)

    # A = B : chaque paire vaut exactement 0,5 -> rejet à la première lecture
    v, n = S.decide([0.5] * 40), 40
    check("A = B rejeté dès 40 paires", v == "rejette")
    check("pas de décision avant 40 paires", S.decide([1.0] * 39) is None)
    for mu, lo, hi in ((0.50, 0.0, 0.09), (0.52, 0.90, 1.0), (0.47, 0.0, 0.01), (0.55, 0.99, 1.0)):
        res = [simulate(mu, rng) for _ in range(300)]
        acc = sum(v == "garde" for v, _ in res) / len(res)
        check(f"vrai {mu:.0%} : gardé {acc:.0%} dans [{lo:.0%}, {hi:.0%}]", lo <= acc <= hi,
              f"(médiane {sorted(n for _, n in res)[150]} paires)")
    print(f"{ok}/{tot} tests passed")
    sys.exit(0 if ok == tot else 1)


if __name__ == "__main__":
    main()
