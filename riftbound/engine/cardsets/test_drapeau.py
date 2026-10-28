"""Coups de l'IA enregistrés par la table (TGame.aiscript) et erreurs marquées par le joueur (train.flag), 2026-10-28.
Run: python3 cardsets/test_drapeau.py"""
import json, os, random, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
import train                                            # noqa: E402

T = Suite("drapeau")


def drive(rr, v, moves, until_turn):
    """Joue l'humain au hasard (comme train.py en test) jusqu'au tour until_turn ; moves reçoit les coups comme la page."""
    while v.get("winner") is None and v["st"]["t"] < until_turn:
        if v.get("ask"):
            x = [] if v["ask"]["kind"] == "mulligan" else rr.randrange(len(v["ask"]["options"]))
            moves.append(["ans", x])
            v = json.loads(train.answer(json.dumps(x)))
        elif v.get("dec"):
            ops = v["dec"]["options"]
            i = rr.randrange(len(ops))
            moves.append(["act", i, ops[i]["label"], v["st"]["t"]])
            v = json.loads(train.act(i))
        else:
            v = json.loads(train.step())
    return v


def game(seed=5, turns=6):
    rr = random.Random(seed)
    train.new(seed, None, None, 0)
    moves = []
    v = drive(rr, json.loads(train.step()), moves, turns)
    f = json.loads(train.flag())
    moves.append(["flag", "cible", v["st"]["t"], f["step"], "?"])
    v = drive(rr, v, moves, turns + 2)
    return dict(seed=seed, level=0, moves=moves, ai=json.loads(train.ai_script())["ai"], pts=v["st"]["pts"]), f


@T.test
def table_records_every_ai_decision_and_answer():
    g, f = game()
    ai = g["ai"]
    assert ai and all(e[0] in ("d", "a") and len(e) == 4 for e in ai)
    assert f["step"] is not None and ai[f["step"]][0] == "d"
    assert any(e[0] == "d" for e in ai)


@T.test
def recorded_game_replays_identically_even_with_another_ai():
    g, f = game()
    # rejeu avec une IA d'un autre niveau (Normal au lieu de débutante) : les coups de l'IA viennent du script
    train.new(g["seed"], None, None, 1)
    train.ai_replay(g["ai"], capture=[f["step"]])
    v = json.loads(train.step())
    rr_moves = [m for m in g["moves"] if m[0] != "flag"]
    for m in rr_moves:
        while v.get("busy") or not (v.get("ask") or v.get("dec")):
            v = json.loads(train.step())
        v = json.loads(train.answer(json.dumps(m[1])) if m[0] == "ans" else train.act(m[1]))
    while v.get("busy"):
        v = json.loads(train.step())
    assert v["st"]["pts"] == g["pts"], (v["st"]["pts"], g["pts"])
    assert json.loads(train.ai_script())["ai"][:len(g["ai"])] == g["ai"]
    assert f["step"] in train.W["caught"]               # position du coup marqué copiée au passage


@T.test
def game_with_undo_replays_identically():
    # 2026-10-28 (parties de l'utilisateur) : avec « Reprendre », l'ancien enregistrement (dans l'état) perdait les coups
    # de l'IA repris ; le rejeu les rejoue pourtant avant le « Reprendre ». Le journal garde tout, dans l'ordre réel.
    rr = random.Random(9)
    train.new(9, None, None, 0)
    moves = []
    v = drive(rr, json.loads(train.step()), moves, 4)
    n0 = len(json.loads(train.ai_script())["ai"])
    for _ in range(2):                                  # deux coups puis « Reprendre » : l'IA a pu jouer entre-temps
        while not v.get("dec"):
            v = drive(rr, v, moves, v["st"]["t"] + 1)
        i = len(v["dec"]["options"]) - 1
        moves.append(["act", i])
        v = json.loads(train.act(i))
        while v.get("busy"):
            v = json.loads(train.step())
        moves.append(["undo"])
        v = json.loads(train.undo())
    v = drive(rr, v, moves, 8)
    g = dict(seed=9, level=0, moves=moves, ai=json.loads(train.ai_script())["ai"], result=dict(pts=v["st"]["pts"]))
    assert len(g["ai"]) >= n0                           # « Reprendre » n'efface rien du journal
    train.new(9, None, None, 1)                         # autre IA : les coups viennent du journal
    train.ai_replay(g["ai"])
    v = json.loads(train.step())
    for m in moves:
        while v.get("busy") or not (v.get("ask") or v.get("dec") or m[0] == "undo"):
            v = json.loads(train.step())
        v = json.loads(train.undo() if m[0] == "undo" else train.answer(json.dumps(m[1])) if m[0] == "ans" else train.act(m[1]))
    while v.get("busy"):
        v = json.loads(train.step())
    assert v["st"]["pts"] == g["result"]["pts"] and json.loads(train.ai_script())["ai"] == g["ai"]


@T.test
def search_copies_and_hints_do_not_write_the_log():
    train.new(3, None, None, 1)
    v = json.loads(train.step())
    while not v.get("dec"):
        v = json.loads(train.step()) if v.get("busy") or not v.get("ask") else json.loads(train.answer("[]"))
    n = len(json.loads(train.ai_script())["ai"])
    train.hint(2)                                       # le conseil cherche sur des copies où l'IA répond aux asks
    assert len(json.loads(train.ai_script())["ai"]) == n


if __name__ == "__main__":
    T.main()
