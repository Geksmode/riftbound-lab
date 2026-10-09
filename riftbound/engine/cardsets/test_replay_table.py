"""Mode replay de la table (« Mes parties » → « Revoir ») : une partie jouée par la page (réglages + suite exacte des
choix) se rejoue à l'identique (train.replay_load), image par image, le conseil se calcule à chacune de tes décisions
sans toucher au rejeu, et la partie en cours est rendue en quittant. Run: python3 cardsets/test_replay_table.py"""
import json, os, random, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import Suite                                # noqa: E402
import train                                             # noqa: E402

T = Suite("replay_table")


def _play(seed, n_moves):
    """Joue comme la page : enregistre ["act", i, libellé, tour] / ["ans", x, libellé, tour]. Renvoie l'enregistrement."""
    rng = random.Random(seed)
    meta = json.loads(train.new(seed, None, 0, 1))
    rec = dict(seed=seed, bf=None, first=0, level=1, moves=[], result=None)
    v = json.loads(train.step())
    for _ in range(4000):
        if v.get("busy"):
            v = json.loads(train.step())
        elif v.get("winner") is not None:
            break
        elif v.get("ask"):
            x = [] if v["ask"]["kind"] == "mulligan" else rng.randrange(len(v["ask"]["options"]))
            rec["moves"].append(["ans", x, "", v["st"]["t"]])
            v = json.loads(train.answer(json.dumps(x)))
        elif v.get("dec"):
            if sum(1 for m in rec["moves"] if m[0] == "act") >= n_moves:
                break
            ops = v["dec"]["options"]
            e = [o["i"] for o in ops if o["k"] in ("end", "pass")]
            i = rng.choice(e) if e and rng.random() < .4 else rng.randrange(len(ops))
            rec["moves"].append(["act", i, ops[i]["label"], v["st"]["t"]])
            v = json.loads(train.act(i))
        else:
            v = json.loads(train.step())
    rec["result"] = dict(winner=v.get("winner"), pts=v["st"]["pts"]) if v.get("winner") is not None else None
    return rec, meta, v


@T.test
def a_saved_game_replays_frame_by_frame_and_gives_advice_on_my_moves():
    rec, meta, end = _play(31, 25)
    pts_live = end["st"]["pts"]
    r = json.loads(train.replay_load(json.dumps(rec)))
    assert r["desync"] is None and r["n"] > 10 and r["meta"]["seed"] == 31
    assert r["end"]["pts"] == pts_live and r["end"]["t"] == end["st"]["t"], (r["end"], pts_live)
    f0, fl = json.loads(train.replay_frame(0)), json.loads(train.replay_frame(r["n"] - 1))
    assert f0["v"]["st"]["t"] <= fl["v"]["st"]["t"] and fl["v"]["st"]["pts"] == pts_live
    acts = [j for j, lab in r["mine"] if lab.startswith("Tu joues")
            and len((json.loads(train.replay_frame(j))["v"].get("dec") or {}).get("options", [])) >= 3]
    assert acts
    j = acts[len(acts) // 2]
    before = json.loads(train.replay_frame(r["n"] - 1))
    h = json.loads(train.replay_hint(j))
    assert h and all("label" in x for x in h), h
    assert json.loads(train.replay_frame(r["n"] - 1)) == before        # le conseil ne change pas le rejeu
    assert all(isinstance(l[2], int) and 0 <= l[2] < r["n"] for l in r["log"])


@T.test
def leaving_the_replay_gives_back_the_game_in_progress():
    rec, meta, end = _play(32, 8)                  # une partie déjà enregistrée
    _play(33, 6)                                   # la partie en cours (une autre donne)
    t_live = train.W["g"].turn_no
    hand_live = [c.cname for c in train.W["g"].p[train.ME].hand]
    json.loads(train.replay_load(json.dumps(rec)))
    assert json.loads(train.replay_exit())["prev"] is True
    assert train.W["g"].turn_no == t_live and [c.cname for c in train.W["g"].p[train.ME].hand] == hand_live


if __name__ == "__main__":
    T.main()
