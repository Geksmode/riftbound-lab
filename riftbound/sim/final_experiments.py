#!/usr/bin/env python3
"""Combine the best swaps into candidate lists and test them (more games, several conditions)."""
import sys, json, random
from collections import Counter
from pathlib import Path
from riftsim import Game
from run_sims import AKALI_STOCK, LEBLANC, LEBLANC_SIDED, LB_BFS, PLANS, edit, play

N = int(sys.argv[1]) if len(sys.argv) > 1 else 2000
cfg = PLANS["plan anti-LeBlanc"]

C1 = edit(AKALI_STOCK, out=[("Defy", 2)], add=[("Mournful Witness", 2)])
C2 = edit(C1, out=[("Long Sword", 1)], add=[("Akali, Silent", 1)])
C3 = edit(C2, out=[("Long Sword", 1)], add=[("Ferrous Forerunner", 1)])
C4 = edit(C3, out=[("En Garde", 1)], add=[("Not So Fast", 1)])
C5 = edit(C3, out=[("Defy", 1)], add=[("Lonely Poro", 1)])
C6 = edit(C3, out=[("Charm", 1)], add=[("Not So Fast", 1)])
C7 = edit(C3, out=[("Adaptatron", 1), ("En Garde", 1)], add=[("Akali, Silent", 1), ("Mischievous Marai", 1)])
BUILDS = {"Stock (Wuhan)": AKALI_STOCK, "C1 -2 Defy +2 Witness": C1, "C2 +Silent": C2, "C3 +Forerunner": C3,
          "C4 +NSF": C4, "C5 -Defy +Poro": C5, "C6 -Charm +NSF": C6, "C7 +2e Silent +Marai": C7}


def split_by_order(n, deck, ldeck, bf, plan_cfg):
    """win rate when Akali goes first vs second"""
    res = {"first": Counter(), "second": Counter()}
    for k in range(n):
        random.seed(k); bfl = random.choice(LB_BFS)
        g = Game(deck, ldeck, bf, bfl, cfg_a=dict(plan_cfg), seed=k)
        key = "first" if g.first.legend == "Akali" else "second"
        res[key][g.run()] += 1
    return {k: round(v["Akali"] / max(1, sum(v.values())), 3) for k, v in res.items()}


if __name__ == "__main__":
    out = {"games": N, "builds": {}, "conditions": {}}
    for name, d in BUILDS.items():
        wr, t, _ = play(N, d, LEBLANC, "Void Gate", LB_BFS, cfg)
        wr2, _, _ = play(N, d, LEBLANC_SIDED, "Void Gate", LB_BFS, cfg, seed0=10 ** 6)
        out["builds"][name] = dict(vs_leblanc=round(wr, 3), vs_leblanc_sided=round(wr2, 3), turns=round(t, 1),
                                   main=sorted(Counter(d["main"]).items()))
        print(f"{name:26s} vs LB {wr:5.1%} | vs LB side {wr2:5.1%} | {t:4.1f} tours", flush=True)
    best = max(out["builds"], key=lambda k: out["builds"][k]["vs_leblanc"] + out["builds"][k]["vs_leblanc_sided"])
    bd = BUILDS[best]
    print("\nmeilleur:", best)
    for bf in ("Void Gate", "Targon's Peak", "Forgotten Monument", "Back-Alley Bar", "Threshold of the Gray"):
        wr, t, _ = play(N, bd, LEBLANC, bf, LB_BFS, cfg)
        out["conditions"][f"bf {bf}"] = round(wr, 3); print(f"  battlefield {bf:22s} {wr:5.1%}", flush=True)
    for lbbf in LB_BFS:
        wr, t, _ = play(N, bd, LEBLANC, "Void Gate", [lbbf], cfg)
        out["conditions"][f"LB bf {lbbf}"] = round(wr, 3); print(f"  LeBlanc joue {lbbf:22s} {wr:5.1%}", flush=True)
    for label, extra in (("mulligan agressif (unites <=4)", dict(mulligan="aggro")),
                         ("prudent face a Vi", dict(respect_vi=True, aggro=False)),
                         ("garder mana pour NSF/Defy", dict(hold_nsf=True, hold_defy=True)),
                         ("ne jamais garder de mana", dict(hold_nsf=False, hold_defy=False)),
                         ("plan naif (tuer tout)", dict(plan="kill")),
                         ("Zhonya sur tout (>=3)", dict(zhonya_min=3)),
                         ("legende Akali non empower", dict(empower_legend=False))):
        wr, t, _ = play(N, bd, LEBLANC, "Void Gate", LB_BFS, dict(cfg, **extra))
        out["conditions"][label] = round(wr, 3); print(f"  {label:32s} {wr:5.1%}", flush=True)
    sp = split_by_order(N, bd, LEBLANC, "Void Gate", cfg)
    out["conditions"]["Akali joue en premier"] = sp["first"]; out["conditions"]["Akali joue en second"] = sp["second"]
    print("  ordre de jeu", sp)
    out["best"] = best
    json.dump(out, open(Path(__file__).parent / "final_results.json", "w"), ensure_ascii=False, indent=1)
