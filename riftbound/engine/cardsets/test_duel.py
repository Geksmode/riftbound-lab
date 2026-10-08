"""Duel entre deux joueurs : deux processus (un par navigateur) appliquent la même suite d'entrées et restent
identiques ; à chaque instant un seul a une décision ou une question, l'autre attend. Run: python3 cardsets/test_duel.py"""
import json, os, random, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import Suite                                # noqa: E402

T = Suite("duel")
ENGINE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WORKER = r'''
import json, sys
sys.path.insert(0, sys.argv[1])
import train
for line in sys.stdin:
    c = json.loads(line)
    if c["op"] == "new":
        out = train.duel_new(*c["args"])
    elif c["op"] == "step":
        out = train.step()
    elif c["op"] == "act":
        out = train.act(c["i"])
    elif c["op"] == "answer":
        out = train.answer(json.dumps(c["x"]))
    sys.stdout.write(out.replace("\n", " ") + "\n"); sys.stdout.flush()
'''


class Client:
    def __init__(s):
        s.p = subprocess.Popen([sys.executable, "-c", WORKER, ENGINE], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True)

    def call(s, **c):
        s.p.stdin.write(json.dumps(c) + "\n"); s.p.stdin.flush()
        line = s.p.stdout.readline()
        if not line:
            raise RuntimeError("le joueur a planté : " + s.p.stderr.read()[-1500:])
        return json.loads(line)

    def settle(s, v):
        while v.get("busy"):
            v = s.call(op="step")
        return v

    def close(s):
        s.p.stdin.close(); s.p.wait()


def play_duel(seed, max_inputs=4000):
    a, b = Client(), Client()
    try:
        args = [seed, None, None, seed % 2, "akali-g2", "leblanc-iq5"]
        a.call(op="new", args=args + [0, ["Hôte", "Invité"]])
        b.call(op="new", args=args + [1, ["Hôte", "Invité"]])
        va, vb = a.settle(a.call(op="step")), b.settle(b.call(op="step"))
        rng, n = random.Random(seed), 0
        while va.get("winner") is None and n < max_inputs:
            turn = [v for v in (va, vb) if v.get("dec") or v.get("ask")]
            assert len(turn) == 1, ("un seul joueur doit avoir la main", va.keys(), vb.keys())
            v = turn[0]
            other = vb if v is va else va
            assert other.get("wait"), "l'autre joueur doit attendre"
            if v.get("ask"):
                k = v["ask"]["kind"]
                cmd = dict(op="answer", x=[] if k == "mulligan" else rng.randrange(len(v["ask"]["options"])))
            else:
                cmd = dict(op="act", i=rng.randrange(len(v["dec"]["options"])))
            va, vb = a.settle(a.call(**cmd)), b.settle(b.call(**cmd))
            n += 1
        return va, vb, n
    finally:
        a.close(); b.close()


@T.test
def two_players_stay_in_sync_until_the_end():
    for seed in (3, 8):
        va, vb, n = play_duel(seed)
        assert va.get("winner") is not None and va["winner"] == vb["winner"], (seed, n)
        assert va["st"]["pts"] == vb["st"]["pts"] and va["st"]["t"] == vb["st"]["t"]
        assert va["me"] == 0 and vb["me"] == 1
        bf = lambda v: [(b["n"], sorted(u["n"] for u in b["u"])) for b in v["st"]["bfs"]]
        assert bf(va) == bf(vb)


@T.test
def each_player_sees_only_their_own_hand():
    a = Client()
    try:
        a.call(op="new", args=[5, None, None, 0, "akali-g2", "leblanc-iq5", 1, ["H", "I"]])
        v = a.settle(a.call(op="step"))
        st = v["st"]
        assert all(c[1] == "?" for c in st["p"][0]["hand"]) and not any(c[1] == "?" for c in st["p"][1]["hand"])
    finally:
        a.close()


if __name__ == "__main__":
    T.main()
