import sys, time
from collections import Counter
from game import Game
from agents import RandomAgent
from ai import SearchAgent
from decks import load

def run(seed, ag, A, L, log=False):
    g = Game([A, L], ag, seed=seed, log=log)
    while True:
        d = g.advance()
        if d is None: break
        g.apply(ag[d.player].decide(g, d))
    return g

if __name__ == "__main__":
    A = load("akali_dongdong_wuhan-open_5th", "Void Gate"); L = load("leblanc_gyatarina_ccs-iq5_1st", "Star Spring")
    res = Counter(); t = time.time()
    n = int(sys.argv[1])
    mode = sys.argv[2]
    for i in range(n):
        if mode == "sr":
            ag = [SearchAgent(i), RandomAgent(i)]
        elif mode == "rs":
            ag = [RandomAgent(i), SearchAgent(i)]
        else:
            ag = [SearchAgent(i), SearchAgent(i + 99)]
        g = run(i, ag, A, L)
        res[g.winner] += 1
        print(i, g.winner, g.turn_no, [p.points for p in g.p], round(time.time() - t, 1), flush=True)
    print(res)
