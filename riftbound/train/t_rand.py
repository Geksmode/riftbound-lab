import sys, json, random, time, traceback, collections
import os; sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "engine"))
import train as T, cards
from game import SPEC
cat = json.loads(T.catalog())["cards"]
L = [c for c in cat if c["t"] == "Legend" and c["m"]]
BF = [c["n"] for c in cat if c["t"] == "Battlefield" and c["m"] and not c["ban"]]
def rdeck(rng):
    for _ in range(50):
        lg = rng.choice(L); dom = set(lg["d"])
        ch = [c for c in cat if c["t"] == "Unit" and c["s"] == "Champion" and c["m"] and set(c["tg"]) & set(lg["tg"]) and set(c["d"]) <= dom]
        pool = [c["n"] for c in cat if c["t"] in ("Unit", "Spell", "Gear") and c["m"] and set(c["d"]) <= dom and c["s"] != "Champion" and c["s"] != "Signature" and not c["ban"] and "Unique" not in c["tx"]]
        if not ch or len(pool) < 14: continue
        main = []
        while len(main) < 39:
            n = rng.choice(pool)
            if main.count(n) < 3: main.append(n)
        d = dict(name="r", legend=lg["n"], champion=rng.choice(ch)["n"], main=main,
                 runes=[lg["d"][0] + " Rune"] * 6 + [lg["d"][-1] + " Rune"] * 6, battlefields=rng.sample(BF, 3))
        e = [x for x in json.loads(T.check(json.dumps(d))) if not x.startswith("⚠")]
        if not e: return d
    raise RuntimeError("no deck")
kinds = collections.Counter(); bad = []; LB = []
N = int(sys.argv[1]) if len(sys.argv) > 1 else 20
t0 = time.time()
for seed in range(N):
    rng = random.Random(seed)
    a, b = rdeck(rng), rdeck(rng)
    try:
        T.new(seed, mine=json.dumps(a), opp=json.dumps(b))
        v = json.loads(T.step()); n = 0
        while v.get("winner") is None and n < 4000:
            n += 1
            if v.get("ask"):
                k = v["ask"]["kind"]; kinds[(k, v["ask"]["title"])] += 1; LB.append((v["ask"]["title"], v["ask"]["options"][:3]))
                x = [] if k == "mulligan" else rng.randrange(len(v["ask"]["options"]))
                v = json.loads(T.answer(json.dumps(x)))
            elif v.get("dec"):
                if n % 40 == 0: json.loads(T.hint(3))
                v = json.loads(T.act(rng.randrange(len(v["dec"]["options"]))))
            else:
                v = json.loads(T.step())
    except Exception as e:
        bad.append((seed, a["legend"], b["legend"], traceback.format_exc()[-600:]))
print(N, "parties", round(time.time() - t0), "s, erreurs", len(bad))
for x in bad[:3]: print(x)
print(sorted(set(k[1] for k in kinds))[:60])
import random as _r; [print(x) for x in _r.Random(1).sample(LB, min(25, len(LB)))]
