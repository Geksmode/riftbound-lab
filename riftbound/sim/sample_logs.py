from final_experiments import *
from riftsim import Game
import random
c = dict(cfg, mulligan="aggro")
found = {"Akali": [], "LeBlanc": []}
for k in range(200):
    random.seed(k); bfl = random.choice(LB_BFS)
    g = Game(C5, LEBLANC, "Targon's Peak", bfl, cfg_a=dict(c), seed=k, log=True)
    w = g.run()
    if w in found and len(found[w]) < 2:
        found[w].append(k)
        open(f"logs/partie_{k}_{w}.txt", "w").write(f"Seed {k} | Akali C5 (Targon's Peak) vs LeBlanc ({bfl}) | vainqueur {w}\n" + "\n".join(g.lines))
print(found)
