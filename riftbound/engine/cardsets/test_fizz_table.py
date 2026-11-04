"""Fizz, Trickster sur la table : la question « Quel sort lancer ? » décrit chaque option comme une carte (image) et ses
cibles (uids touchés sur le plateau), pas seulement comme un texte. Signalé par l'utilisateur le 2026-11-04 (capture :
une longue liste « Switcheroo (sans payer l'énergie ; cible X, Y) », deux fois, pour deux Switcheroo en défausse).
Run: python3 cardsets/test_fizz_table.py"""
import os, sys, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from testkit import *          # noqa: F401,F403
from test_table_copie import _main_of_human, _refresh

T = Suite("fizz_table")


@T.test
def fizz_pick_describes_spells_and_targets():
    from game import Obj, Rune
    train, v = _main_of_human()
    g, me, ai = train.W["g"], train.ME, train.AI
    g.p[me].runes = [Rune("Chaos", me) for _ in range(6)]
    sw = []
    for _ in range(2):                              # deux exemplaires en défausse
        s = Obj("Switcheroo", me)
        s.zone = "trash"
        g.p[me].trash.append(s)
        sw.append(s.uid)
    a = Obj("Soaring Scout", me)
    g.enter_board(a, me, 1, ready=True)
    e = Obj("Soaring Scout", me)
    g.enter_board(e, me, 1, ready=True)
    g.bfs[1].ctrl = me                              # pas de combat ni de showdown à ouvrir
    c = Obj("Fizz, Trickster", me)
    c.zone = "hand"
    g.p[me].hand.append(c)
    v = _refresh(train)
    i = [o["i"] for o in v["dec"]["options"] if o["k"] == "play" and o.get("src") == c.uid and o.get("loc") == "base"][0]
    v = json.loads(train.act(i))
    for _ in range(40):
        if v.get("ask") and v["ask"]["kind"] == "fizz_pick":
            break
        if v.get("busy"):
            v = json.loads(train.step())
        elif v.get("ask"):
            v = json.loads(train.answer(json.dumps(0)))
        elif v.get("dec") and v["dec"]["kind"] != "main":
            v = json.loads(train.act(next(o["i"] for o in v["dec"]["options"] if o["k"] == "pass")))
        else:
            raise AssertionError("pas de question fizz_pick")
    ask = v["ask"]
    pc = ask["pc"]
    assert len(pc) == len(ask["options"]) and pc[-1] is None, pc          # dernière option : ne rien lancer
    plays = [p for p in pc if p]
    assert plays and all(p["n"] == "Switcheroo" and p["u"] in sw for p in plays), plays
    # toutes les cibles sont des unités en jeu (touchables), et la paire des deux Scouts est proposée
    on_board = {o.uid for o in g.board}
    assert all(len(p["tg"]) == 2 and set(p["tg"]) <= on_board for p in plays), plays
    assert any(sorted(p["tg"]) == sorted([a.uid, e.uid]) for p in plays), plays
    # on répond par l'option (les deux Scouts) : le sort est rejoué sur ces cibles
    k = next(j for j, p in enumerate(pc) if p and sorted(p["tg"]) == sorted([a.uid, e.uid]))
    v = json.loads(train.answer(json.dumps(k)))
    g = train.W["g"]
    assert any(it.name == "Switcheroo" and sorted(it.data.get("tg", ())) == sorted([a.uid, e.uid]) for it in g.chain) \
        or any(y.cname == "Switcheroo" for y in g.p[me].deck), [it.name for it in g.chain]


if __name__ == "__main__":
    T.main()
