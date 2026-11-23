"""Puzzles de la table (menu « Puzzles », engine/puzzles.py) : chaque position est montée dans le vrai moteur ; la
solution annoncée atteint l'objectif contre la vraie IA de la table, et le piège annoncé par la leçon le manque.
Run: python3 cardsets/test_puzzles.py"""
import os, sys, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from testkit import *          # noqa: F401,F403
from pilote_puzzles import run, play, move, nm

T = Suite("puzzles")


def on_chain(name):
    return lambda g: any(it.name == name for it in g.chain)


FLIP = lambda g, a: (play("Shuriken Flip", tg=["Pit Rookie"])(g, a) and a[3].get("dest") == 1
                     and nm(g, a[3]["mover"]) == "Kai'Sa, Survivor")
LINES = {
    "flip": ([dict(on="main", pick=FLIP)],
             [dict(on="main", pick=move(["Kai'Sa, Survivor"], 1))]),
    "count": ([dict(on="main", pick=move(["Kai'Sa, Survivor", "Mournful Witness"], 1)),
               dict(on="prio", when=on_chain("En Garde"), pick=play("Discipline", tg=["Kai'Sa, Survivor"]))],
              [dict(on="main", pick=move(["Kai'Sa, Survivor"], 1))]),
    "heron": ([dict(on="main", pick=play("Astral Heron", dest=0)),
               dict(on="prio", when=on_chain("Charm"), pick=play("Defy"))],
              [dict(on="main", pick=play("Astral Heron", dest="base"))]),
    "block": ([dict(on="prio", when=on_chain("Punch First"), pick=play("Block", tg=["Astral Heron"], src="facedown"))],
              [dict(on="prio", when=on_chain("Punch First"), pick=play("Discipline", tg=["Astral Heron"]))]),
    "runner": ([dict(on="main", pick=play("Block", tg=["Mournful Witness"])),
                dict(on="main", pick=move(["Astral Heron", "Mournful Witness"], 1))],
               [dict(on="main", pick=move(["Astral Heron", "Mournful Witness"], 1))]),
    "decree": ([dict(on="main", pick=move(["Astral Heron"], 1))],
               [dict(on="main", pick=move(["Astral Heron", "Kai'Sa, Survivor"], 1))]),
}


@T.test
def every_puzzle_has_a_tested_line():
    import puzzles
    assert sorted(LINES) == sorted(p["id"] for p in puzzles.PUZZLES)
    for p in puzzles.PUZZLES:
        for k in ("title", "tag", "brief", "goal_text", "solution", "lesson"):
            assert len(p[k]) == 2 and all(p[k]), (p["id"], k)


@T.test
def solutions_reach_the_goal_against_the_table_ai():
    for pid, (sol, _) in LINES.items():
        ok, log, left = run(pid, sol)
        assert not left, (pid, "étapes non jouées", log)
        assert ok is True, (pid, log)


@T.test
def traps_miss_the_goal():
    for pid, (_, trap) in LINES.items():
        ok, log, left = run(pid, trap)
        assert not left, (pid, log)
        assert ok is False, (pid, log)


@T.test
def puzzle_position_is_the_announced_one():
    import train
    v = json.loads(train.puzzle_new("block"))
    assert v["puzzle"]["id"] == "block" and v["first"] == 1
    g = train.W["g"]
    assert g.tp == 1 and [b.name for b in g.bfs] == ["Sigil of the Storm", "Grove of the God-Willow"]
    assert [c.name for c in g.bfs[0].facedowns] == ["Block"] and g.bfs[0].ctrl == 0
    assert sorted(c.name for c in g.p[0].hand) == ["Defy", "Discipline"]
    assert not g.p[0].champ and not g.p[1].champ
    v = json.loads(train.step())
    while v.get("busy"):
        v = json.loads(train.step())
    assert v["puz"] == dict(id="block", end=6, turn=6, ok=None), v["puz"]
    # images : les cartes du puzzle hors des decks sont dans la base des cartes de la table
    db = json.loads(train.cards())
    names = db if isinstance(db, dict) else {}
    for n in ("Astral Heron", "Block", "Grove of the God-Willow", "Master Yi, Tempered"):
        assert n in names, n


@T.test
def puzzle_is_reproducible():
    a = run("decree", LINES["decree"][0])[1]
    b = run("decree", LINES["decree"][0])[1]
    assert a == b


@T.test
def a_normal_game_after_a_puzzle_has_no_puzzle():
    import train
    train.puzzle_new("flip")
    v = json.loads(train.new(4, None, 0))
    v = json.loads(train.step())
    assert "puz" not in v and getattr(train.W["g"], "puz", None) is None


if __name__ == "__main__":
    T.main()
