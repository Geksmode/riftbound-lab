#!/usr/bin/env python3
"""Run Akali vs LeBlanc experiments with riftsim.py. Usage: python3 run_sims.py [n_games] [--log SEED]"""
import json, sys, random, itertools
from collections import Counter
from pathlib import Path
from riftsim import Game

ROOT = Path(__file__).resolve().parents[1]
DECKS = {d["file"].replace(".txt", ""): d for d in json.load(open(ROOT / "decks" / "decks.json", encoding="utf-8"))}


def from_raw(key):
    d = DECKS[key]
    main, runes = [], []
    for c in d["cards"]:
        if c["section"] in ("champion", "main"): main += [c.get("name_db", c["name"])] * c["qty"]
        if c["section"] == "runes": runes += [c["name"].replace(" Rune", "")] * c["qty"]
    return dict(main=main, runes=runes)


def edit(deck, out=(), add=()):
    main = list(deck["main"])
    for n, q in out:
        for _ in range(q): main.remove(n)
    for n, q in add: main += [n] * q
    assert len(main) == 40, len(main)
    return dict(main=main, runes=deck["runes"])


AKALI_STOCK = from_raw("akali_dongdong_wuhan-open_5th")          # Wuhan top 8 list (40 ok)
LEBLANC = from_raw("leblanc_gyatarina_ccs-iq5_1st")               # IQ#5 winner
LEBLANC_SIDED = edit(LEBLANC, out=[("Deathgrip", 1), ("Hidden Blade", 1)],
                     add=[("LeBlanc, Everywhere At Once", 1), ("Thousand-Tailed Watcher", 1)])

# Anti-LeBlanc build (sideboarded game 2/3): answers to Dragon/Blade/Rex, untargetable threat, kill Hook
AKALI_ANTI = edit(AKALI_STOCK,
                  out=[("Long Sword", 2), ("En Garde", 1), ("Adaptatron", 1), ("Charm", 0)],
                  add=[("Not So Fast", 2), ("Akali, Silent", 1), ("Brittle Steel", 1)])
# Tempo build: more cheap pressure, fewer 7-drops
AKALI_TEMPO = edit(AKALI_ANTI, out=[("Astral Heron", 1), ("Ferrous Forerunner", 1)],
                   add=[("Mournful Witness", 2)])

PLANS = {
    "tuer tout (plan naif)": dict(plan="kill", zhonya_min=4, counter_min=4),
    "plan anti-LeBlanc": dict(plan="leblanc", zhonya_min=5, counter_min=4),
}


def play(n, a_deck, l_deck, bf_a, bf_l_choices, cfg_a, cfg_l=None, seed0=0):
    res = Counter(); turns = 0
    for k in range(n):
        random.seed(seed0 + k)
        bf_l = random.choice(bf_l_choices)
        g = Game(a_deck, l_deck, bf_a, bf_l, cfg_a=dict(cfg_a), cfg_l=dict(cfg_l or {}), seed=seed0 + k)
        res[g.run()] += 1; turns += g.turn
    wr = (res["Akali"] + 0.5 * res["draw"]) / n
    return wr, turns / n, res


LB_BFS = ["Dusk Rose Lab", "Star Spring", "Windswept Hillock"]

if __name__ == "__main__":
    n = int(sys.argv[1]) if len(sys.argv) > 1 and sys.argv[1].isdigit() else 2000
    if "--log" in sys.argv:
        seed = int(sys.argv[sys.argv.index("--log") + 1])
        g = Game(AKALI_ANTI, LEBLANC, "Void Gate", "Star Spring", cfg_a=PLANS["plan anti-LeBlanc"], seed=seed, log=True)
        w = g.run(); print("\n".join(g.lines)); print("WINNER", w); sys.exit()
    rows = []
    def exp(label, deck, plan, bf="Void Gate", cfg_extra=None, ldeck=LEBLANC):
        cfg = dict(PLANS[plan], **(cfg_extra or {}))
        wr, t, res = play(n, deck, ldeck, bf, LB_BFS, cfg)
        rows.append((label, plan, bf, wr, t, dict(res)))
        print(f"{label:28s} | {plan:22s} | {bf:18s} | Akali {wr:5.1%} | {t:4.1f} tours | {dict(res)}", flush=True)
    print(f"{n} parties par ligne\n")
    exp("Stock (Wuhan)", AKALI_STOCK, "tuer tout (plan naif)")
    exp("Stock (Wuhan)", AKALI_STOCK, "plan anti-LeBlanc")
    exp("Anti-LeBlanc", AKALI_ANTI, "tuer tout (plan naif)")
    exp("Anti-LeBlanc", AKALI_ANTI, "plan anti-LeBlanc")
    exp("Tempo", AKALI_TEMPO, "plan anti-LeBlanc")
    for bf in ("Targon's Peak", "Forgotten Monument", "Sigil of the Storm", "Threshold of the Gray"):
        exp("Anti-LeBlanc", AKALI_ANTI, "plan anti-LeBlanc", bf=bf)
    exp("Anti-LeBlanc mull. agressif", AKALI_ANTI, "plan anti-LeBlanc", cfg_extra=dict(mulligan="aggro"))
    exp("Anti-LeBlanc prudent (Vi)", AKALI_ANTI, "plan anti-LeBlanc", cfg_extra=dict(respect_vi=True, aggro=False))
    exp("Anti-LeBlanc sans reserve", AKALI_ANTI, "plan anti-LeBlanc", cfg_extra=dict(hold_nsf=False, hold_defy=False))
    exp("Anti-LeBlanc vs LB side", AKALI_ANTI, "plan anti-LeBlanc", ldeck=LEBLANC_SIDED)
    json.dump([dict(build=r[0], plan=r[1], battlefield=r[2], akali_wr=round(r[3], 4), avg_turns=round(r[4], 1),
                    results=r[5], games=n) for r in rows],
              open(Path(__file__).parent / "results.json", "w"), ensure_ascii=False, indent=1)
