"""Égalité exacte entre ai.evaluate et features.linear (Texel), et symétrie des critères.

    python3 texel/test_texel.py [parties]      (défaut 60 ; decks par défaut variés, politique simple)

Positions : toutes les décisions de parties PolicyAgent contre PolicyAgent (milieu de tour compris), les deux points de
vue. Poids : ai.EV, puis des poids tirés au hasard (pts_ramp et react_kw restent à 0)."""
import random, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
import features as F
import ai, plans as P
from game import Game, Obj, Item
from decks import with_bf, load
import train as T


def positions(n, seed0=4_000_000):
    dd = sorted(T.default_decks().values())
    for k in range(n):
        seed = seed0 + k
        r = random.Random(seed)
        da, db = load(r.choice(dd)), load(r.choice(dd))
        decks = [with_bf(da, r.choice(sorted(da["battlefields"]))), with_bf(db, r.choice(sorted(db["battlefields"])))]
        Obj._n = 0
        Item._n = 0
        ag = [ai.PolicyAgent(), ai.PolicyAgent()]
        g = Game(decks, ag, seed=seed, first=r.randrange(2))
        while True:
            d = g.advance()
            if d is None:
                break
            yield g
            g.apply(ag[d.player].decide(g, d))


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 60
    rng = random.Random(1)
    ws = [dict(ai.EV)]
    for _ in range(5):
        w = {k: rng.uniform(-10, 10) for k in ai.EV}
        w["pts_ramp"] = 0.0
        w["react_kw"] = 0.0
        ws.append(w)
    ok = tot = 0
    worst = 0.0
    seen = dict.fromkeys(F.KEYS, 0)
    for g in positions(n):
        if g.winner is not None:
            continue
        for me in (0, 1):
            c, f = F.features(g, me)
            c2, f2 = F.features(g, 1 - me)
            sym = abs(c + c2) < 1e-9 and all(abs(f[k] + f2[k]) < 1e-9 for k in F.KEYS)
            for k in F.KEYS:
                seen[k] += f[k] != 0
            for w in ws:
                e = ai.evaluate(g, me, w)
                lin = F.linear(c, f, w)
                worst = max(worst, abs(e - lin))
                tot += 1
                if abs(e - lin) < 1e-6 * max(1.0, abs(e)) and sym:
                    ok += 1
                elif tot - ok <= 5:
                    print("FAIL", g.turn_no, me, e, lin, "sym" if sym else "ASYM")
    print("critères non nuls (positions) :", seen)
    print(f"écart max {worst:.2e}")
    print(f"{ok}/{tot} tests passed")
    sys.exit(0 if ok == tot else 1)


if __name__ == "__main__":
    main()
