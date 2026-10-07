#!/usr/bin/env python3
"""Deck-building search: test single 1-for-1 / 2-for-2 swaps on the stock Akali list vs LeBlanc.
Same seeds for every build (common random numbers) so differences are less noisy."""
import sys, json
from pathlib import Path
from run_sims import AKALI_STOCK, LEBLANC, LB_BFS, PLANS, edit, play

N = int(sys.argv[1]) if len(sys.argv) > 1 else 1000
cfg = PLANS["plan anti-LeBlanc"]

IN = ["Not So Fast", "Akali, Silent", "Brittle Steel", "Disarming Rake", "Tomb-Raider Barbara",
      "Mournful Witness", "Mischievous Marai", "Irelia, Fervent", "Ferrous Forerunner", "Back Off",
      "Block", "Sky Splitter", "Darius, Trifarian", "Blitzcrank, Impassive", "Lonely Poro"]
OUT = ["Long Sword", "En Garde", "Adaptatron", "Defy", "Astral Heron", "Charm", "Scuttle Crab", "Falling Star"]

base_wr, _, _ = play(N, AKALI_STOCK, LEBLANC, "Void Gate", LB_BFS, cfg)
print(f"base stock: {base_wr:.1%} ({N} parties)")
rows = []
for o in OUT:
    for i in IN:
        cnt_in = AKALI_STOCK["main"].count(i)
        if cnt_in >= 3 or o not in AKALI_STOCK["main"]: continue
        for q in (1, 2):
            if AKALI_STOCK["main"].count(o) < q or cnt_in + q > 3: continue
            d = edit(AKALI_STOCK, out=[(o, q)], add=[(i, q)])
            wr, t, _ = play(N, d, LEBLANC, "Void Gate", LB_BFS, cfg)
            rows.append(dict(out=o, inn=i, q=q, wr=wr, delta=wr - base_wr))
            print(f"-{q} {o:14s} +{q} {i:22s} {wr:6.1%} ({wr - base_wr:+.1%})", flush=True)
rows.sort(key=lambda r: -r["wr"])
json.dump(dict(base=base_wr, games=N, swaps=rows), open(Path(__file__).parent / "swaps.json", "w"), indent=1)
print("\nTOP 12"); [print(f"-{r['q']} {r['out']} +{r['q']} {r['inn']}: {r['wr']:.1%} ({r['delta']:+.1%})") for r in rows[:12]]
