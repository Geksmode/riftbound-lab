"""Match BO1 / BO3 (Core Rules 485-486 ; règles de tournoi 403, 407, 601.1.c) et sideboard.
Run: python3 cardsets/test_match.py"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import Suite                                # noqa: E402
import match as M                                        # noqa: E402
import train                                             # noqa: E402

T = Suite("match")
BF = [["A1", "A2", "A3"], ["B1", "B2", "B3"]]


@T.test
def game_one_roll_winner_chooses():                          # 407.1-407.2
    st = M.new("bo3", 5, BF)
    assert M.game_no(st) == 1 and M.chooser(st) == st["roll"] and M.forced_first(st) is None
    assert not M.can_sideboard(st)                             # 403.5 : pas de sideboard en manche 1


@T.test
def loser_chooses_next_and_used_battlefields_are_removed():   # 407.4, 486.5
    st = M.record(M.new("bo3", 5, BF), 0, ["A1", "B2"], 1)     # l'IA gagne la manche 1
    assert M.chooser(st) == 0 and M.can_sideboard(st)
    assert M.allowed_bfs(st, 0) == ["A2", "A3"] and M.allowed_bfs(st, 1) == ["B1", "B3"]
    assert st["wins"] == [0, 1] and not M.over(st)


@T.test
def draw_keeps_first_player_battlefields_and_no_sideboard():  # 407.4, 486.5.a, 403.10
    st = M.record(M.new("bo3", 5, BF), 1, ["A1", "B2"], -1)
    assert M.chooser(st) is None and M.forced_first(st) == 1
    assert M.allowed_bfs(st, 0) == BF[0] and not M.can_sideboard(st)


@T.test
def match_ends_at_two_game_wins():                             # 486.6
    st = M.new("bo3", 5, BF)
    st = M.record(st, 0, ["A1", "B1"], 0)
    st = M.record(st, 1, ["A2", "B2"], 1)
    assert not M.over(st) and M.game_no(st) == 3
    st = M.record(st, 0, ["A3", "B3"], 0)
    assert M.over(st) and M.winner(st) == 0
    one = M.record(M.new("bo1", 5, BF), 0, ["A1", "B1"], 1)
    assert M.over(one) and M.winner(one) == 1                  # 485.6


@T.test
def bo1_battlefields_are_random_bo3_ai_choice_stays_allowed():
    st = M.new("bo1", 9, BF)
    b = M.pick_bfs(st, choose_ai_bf=lambda ok: "B3", human_bf="A3")
    assert b[0] in BF[0] and b[1] in BF[1]                      # 485.5 : hasard, les choix ne comptent pas
    st = M.record(M.new("bo3", 9, BF), 0, ["A1", "B3"], 0)
    b = M.pick_bfs(st, choose_ai_bf=lambda ok: "B3", human_bf="A1")   # B3 et A1 déjà joués : interdits
    assert b[1] in ("B1", "B2") and b[0] in ("A2", "A3")


@T.test
def sideboard_swap_one_for_one_and_champion_change():         # 403.4, 601.1.c.4
    d = dict(champion="C0", main=["x", "y", "C1"], sideboard=["z", "C2"])
    e = M.sideboard_swap(d, ["x"], ["z"])
    assert sorted(e["main"]) == ["C1", "y", "z"] and sorted(e["sideboard"]) == ["C2", "x"]
    try:
        M.sideboard_swap(d, ["x", "y"], ["z"]); assert False
    except ValueError:
        pass
    f = M.sideboard_swap(d, [], [], champion="C2")
    assert f["champion"] == "C2" and "C0" in f["main"] and "C2" not in f["sideboard"]


@T.test
def deck_validation_checks_the_sideboard():                   # 601.1.c
    p = [x for x in json.loads(train.catalog())["presets"] if x["key"] == "leblanc-iq5"][0]["deck"]
    assert len(p["sideboard"]) == 10 and not [e for e in train.validate(p) if not e.startswith("⚠")]
    q = dict(p, sideboard=p["sideboard"] + [p["sideboard"][0]])
    assert any("sideboard : 11/10" in e for e in train.validate(q))
    r = dict(p, sideboard=["Fury Rune"])
    assert any("ni dans le sideboard" in e for e in train.validate(r))


@T.test
def train_match_api_runs_a_game():
    st = train.match_new("bo3", 3)
    nx = json.loads(train.match_next(st))
    assert nx["game"] == 1 and nx["ai_bf"] in json.loads(st)["bfs"][1] and not nx["sideboard"]
    first = nx["first"] if nx["first"] is not None else 0
    v = json.loads(train.new(nx["seed"], nx["allowed"][0], first, 1, None, None, nx["ai_bf"]))
    assert v["bf"] == [nx["allowed"][0], nx["ai_bf"]] and v["first"] == first
    st2 = json.loads(train.match_record(st, first, json.dumps(v["bf"]), 0))
    assert st2["wins"] == [1, 0] and not st2["over"]
    nx2 = json.loads(train.match_next(json.dumps(st2)))
    assert nx2["chooser"] == 1 and nx2["first"] == 1 and nx2["sideboard"] and v["bf"][1] not in [nx2["ai_bf"]]


@T.test
def match_game_replays_faithfully_with_ai_battlefield():
    """Une manche de match enregistrée comme le fait la page (bf, first, obf, coups) se rejoue à l'identique."""
    import random, train_games
    st = train.match_new("bo3", 21)
    nx = json.loads(train.match_next(st))
    first = nx["first"] if nx["first"] is not None else 1
    natural = json.loads(train.new(nx["seed"], nx["allowed"][1], first, 1))["bf"][1]   # choix de l'IA sans obf
    ai_bf = next(b for b in json.loads(st)["bfs"][1] if b != natural)                    # imposer un AUTRE battlefield
    meta = json.loads(train.new(nx["seed"], nx["allowed"][1], first, 1, None, None, ai_bf))
    rng, moves, v = random.Random(4), [], json.loads(train.step())
    for _ in range(3000):
        while v.get("busy"):
            v = json.loads(train.step())
        if v.get("winner") is not None:
            break
        if v.get("ask"):
            x = [] if v["ask"]["kind"] == "mulligan" else rng.randrange(len(v["ask"]["options"]))
            moves.append(["ans", x]); v = json.loads(train.answer(json.dumps(x)))
        elif v.get("dec"):
            i = rng.randrange(len(v["dec"]["options"]))
            moves.append(["act", i]); v = json.loads(train.act(i))
        else:
            v = json.loads(train.step())
    assert meta["bf"][1] == ai_bf
    rec = dict(id="t", seed=nx["seed"], bf=meta["bf"][0], first=first, level=1, obf=ai_bf, moves=moves,
               result=dict(winner=v.get("winner"), pts=v["st"]["pts"]))
    _, mine, w = train_games.replay(rec)
    assert ai_bf in [b.name for b in train.W["g"].bfs], "le rejeu doit utiliser le battlefield imposé de l'IA"
    assert w.get("winner") == v.get("winner") and w["st"]["pts"] == v["st"]["pts"] and not any(m.get("desync") for m in mine)


@T.test
def duel_match_next_has_no_ai_choice():
    st = train.match_new("bo3", 7, "akali-g2", "leblanc-iq5")
    nx = json.loads(train.match_duel_next(st))
    assert nx["chooser"] in (0, 1) and nx["first"] is None and len(nx["allowed"][0]) == 3 and len(nx["allowed"][1]) == 3
    st2 = train.match_record(st, 0, json.dumps([nx["allowed"][0][0], nx["allowed"][1][1]]), 1)
    nx2 = json.loads(train.match_duel_next(st2))
    assert nx2["chooser"] == 0 and nx2["sideboard"] and nx["allowed"][1][1] not in nx2["allowed"][1]
    b = json.loads(train.match_duel_next(train.match_new("bo1", 7, "akali-g2", "leblanc-iq5")))
    assert b["bo1_bfs"][0] in b["allowed"][0] and b["bo1_bfs"][1] in b["allowed"][1]


if __name__ == "__main__":
    T.main()
