"""Juge du banc d'erreurs (banc.py). Run: python3 banc/test_banc.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
_argv, sys.argv = sys.argv, sys.argv[:1]
import banc as B                                         # noqa: E402
sys.argv = _argv
from testkit import *                                   # noqa: E402,F403
from game import Obj, Item                              # noqa: E402

T = Suite("banc")


def judged(g, name, pred):
    d = g.advance()
    a = opt(g, 0, name, pred)
    return B.judge_play(g.clone(), (Obj._n, Item._n), d, a)


@T.test
def hextech_ray_on_the_unit_it_cannot_kill_is_a_target_error():
    # Deal 3 to a unit at a battlefield : la 2-Might meurt, pas la grosse ; viser la grosse = regret net
    g, _ = new()
    runes(g, 0, ["Fury", "Fury", "Fury"])
    small = put(g, 1, "Mournful Witness", loc=1)
    big = put(g, 1, "Playful Phantom", loc=1)
    hand(g, 0, "Hextech Ray")
    assert g.might(small) <= 3 < g.might(big), (g.might(small), g.might(big))
    r = judged(g, "Hextech Ray", lambda ch: ch.get("tg") == (big.uid,))
    assert r["n_all"] == 2 and r["regret"] >= B.REGRET, r
    best = max(r["alts"], key=lambda x: x[1])
    assert "Mournful Witness" in best[0], r
    r2 = judged(g, "Hextech Ray", lambda ch: ch.get("tg") == (small.uid,))
    assert r2["regret"] == 0 and r2["effect"] > B.NOEFFECT, r2


@T.test
def cleave_on_an_enemy_unit_has_no_visible_effect():
    # Give a unit [Assault 3] this turn : sur une unité adverse, rien pour soi
    g, _ = new()
    runes(g, 0, ["Fury"])
    e = put(g, 1, "Mournful Witness", loc=1)
    hand(g, 0, "Cleave")
    r = judged(g, "Cleave", lambda ch: ch.get("tg") == (e.uid,))
    assert r["effect"] <= B.NOEFFECT, r


@T.test
def the_judge_does_not_change_the_game():
    a, pos = B.play(4000001, "defaut", "sh")
    b, _ = B.play(4000001, "defaut", "sh", judge=False)
    assert a["script"] == b["script"] and a["winner"] == b["winner"]
    assert pos and all("regret" in p for p in pos)


@T.test
def replayed_bench_reaches_every_position_and_keeps_unchanged_choices():
    a, pos = B.play(4000001, "defaut", "sh")
    r = B.rescore(a, pos, "sh")
    assert len(r) == len(pos) and not any("err" in x for x in r)
    assert all(x["new"] == x["old"] for x in r if not x["changed"] and x["typ"] == "ask"), r


@T.test
def ask_answers_round_trip():
    opts = [Obj("Mournful Witness", 0), Obj("Sea Monkey", 1), True]
    for ans in (opts[1], True, [opts[0], opts[1]], (opts[1],), None, 3):
        e = B.enc(ans, opts)
        assert e is not None and B.dec(e, opts) == ans, (ans, e)


if __name__ == "__main__":
    T.main()
