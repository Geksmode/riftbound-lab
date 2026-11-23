"""Tirages de référence de CPython pour tests/pyrandom.rs (une ligne par tirage : graine méthode args = résultat).
Regénérer : python3 rust/tests/gen_pyrandom.py > rust/tests/data/pyrandom.txt"""
import random

SEEDS = [0, 1, 7, 42, 4242, 777003, 2**31 - 1, 2**32, 2**32 + 5, 123456789 * 7919, -17, 2**64 + 3, 20261020]
for seed in SEEDS:
    r = random.Random(seed)
    for k in [1, 5, 13, 32, 33, 47, 64]:
        print(seed, "getrandbits", k, "=", r.getrandbits(k))
    for n in [1, 2, 3, 7, 10, 39, 100, 1000, 2**33 + 1]:
        print(seed, "randrange", n, "=", r.randrange(n))
    print(seed, "randint", -3, 9, "=", r.randint(-3, 9))
    print(seed, "random", "=", repr(r.random()))
    print(seed, "uniform", 0.5, 2.25, "=", repr(r.uniform(0.5, 2.25)))
    for n in [0, 1, 2, 5, 12, 39, 40]:
        x = list(range(n)); r.shuffle(x)
        print(seed, "shuffle", n, "=", ",".join(map(str, x)))
    print(seed, "choice", 17, "=", r.choice(range(17)))
    for n, k in [(5, 0), (10, 3), (10, 10), (40, 6), (30, 2), (200, 7), (1000, 40)]:
        print(seed, "sample", n, k, "=", ",".join(map(str, r.sample(range(n), k))))
