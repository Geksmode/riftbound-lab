"""Main révélée (règle 424) : les effets « They reveal their hand » (Sabotage, Decree of Strength, Mindsplitter,
Insightful Investigator, Bone Skewer, Ashe Focused, Scuttle Crab) posent Game.shown_hand ; la table montre ces cartes
pendant la résolution (même pendant la question posée au joueur) et jusqu'au coup suivant joué chaîne vide (le focus
est passé). Scuttle Crab montre aussi les cartes face cachée de l'adversaire pendant ce tour.
Run: python3 cardsets/test_main_revelee.py"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
import train                                            # noqa: E402
from game import Item                                   # noqa: E402

T = Suite("main_revelee")
ALL = ["Fury", "Calm", "Mind", "Body", "Chaos", "Order"] * 2


def opp_hand(g):
    g.p[1].hand = []
    return [hand(g, 1, n) for n in ("Discipline", "Watchful Sentry", "Falling Star")]


def play(g, name, pred=lambda ch: True):
    g.apply(opt(g, 0, name, pred))
    return settle(g)


@T.test
def every_reveal_effect_sets_the_revealed_hand():
    cases = [("Sabotage", lambda ch: True), ("Decree of Strength", lambda ch: True),
             ("Mindsplitter", lambda ch: ch.get("loc") == "base"), ("Insightful Investigator", lambda ch: ch.get("loc") == "base"),
             ("Bone Skewer", lambda ch: True), ("Ashe, Focused", lambda ch: ch.get("loc") == "base")]
    for name, pred in cases:
        g, _ = new()
        runes(g, 0, ALL)
        cs = opp_hand(g)
        hand(g, 0, name)
        play(g, name, pred)
        line = f"P1 reveals their hand {cs}"
        assert any(ln.endswith(line) for ln in g.lines), (name, g.lines[-8:])
        if name == "Bone Skewer":                  # l'unité jouée au battlefield ouvre un showdown : focus passé, fini
            continue
        sh = g.shown_hand
        assert sh is not None and sh["pid"] == 1 and sh["to"] == 0, (name, sh)
        assert sh["uids"] == [c.uid for c in cs], (name, sh)   # la main au moment de la révélation (424.3.a)


@T.test
def scuttle_crab_deathknell_reveals_the_hand():
    g, _ = new()
    cs = opp_hand(g)
    c = put(g, 0, "Scuttle Crab")
    g.kill([c])
    settle(g)
    assert g.shown_hand["pid"] == 1 and g.shown_hand["uids"] == [x.uid for x in cs]
    assert g.p[1].revealed_turn == g.turn_no


@T.test
def revealed_hand_ends_on_the_next_action_with_an_empty_chain():
    g, _ = new()
    runes(g, 0, ALL)
    opp_hand(g)
    hand(g, 0, "Sabotage")
    play(g, "Sabotage")
    assert g.shown_hand is not None and not g.chain
    g.apply(("end",))
    assert g.shown_hand is None


@T.test
def revealed_hand_lasts_while_the_chain_is_open():
    g, _ = new()
    g.shown_hand = dict(pid=1, to=0, uids=[], turn=g.turn_no)
    g.chain.append(Item("spell", 0, "dummy", lambda g_, it: None))
    g.priority = 1
    g.apply(("pass",))
    assert g.shown_hand is not None                # coup joué pendant la chaîne : la révélation dure


# ------------------------------------------------------------------ vue de la table (train.py)
def V(s):
    v = json.loads(s)
    while v.get("busy"):
        v = json.loads(train.step())
    return v


def table_at_my_main(seed):
    train.new(seed, None, 0)
    v = V(train.step())
    for _ in range(50):
        if v.get("ask"):
            v = V(train.answer(json.dumps([] if v["ask"]["kind"] == "mulligan" else 0)))
        elif v.get("dec") and v["dec"]["kind"] == "main":
            return v
        else:
            v = V(train.act(0))
    raise AssertionError("pas de décision principale")


def refresh():
    train.W["d"] = None
    return V(train.step())


def names(v, pid=1):
    return [n for _, n in v["st"]["p"][pid]["hand"]]


@T.test
def table_shows_the_hand_during_the_choice_and_until_the_next_action():
    table_at_my_main(11)
    g = train.W["g"]
    for r in g.p[0].runes:
        r.exhausted = False
    g.p[0].runes += [Rune("Body", 0) for _ in range(3)]
    s = hand(g, 0, "Sabotage")
    opp_hand(g)
    v = refresh()
    assert set(names(v)) == {"?"}                  # main adverse cachée avant l'effet
    i = next(o["i"] for o in v["dec"]["options"] if o["k"] == "play" and str(o["src"]) == str(s.uid))
    v = V(train.act(i))
    for _ in range(10):                            # l'IA peut répondre ; on va jusqu'à la question de Sabotage
        if v.get("ask"):
            break
        v = V(train.act(next(o["i"] for o in v["dec"]["options"] if o["k"] == "pass")))
    assert v["ask"]["kind"] == "sabotage", v.get("ask")
    assert names(v) == ["Discipline", "Watchful Sentry", "Falling Star"] and v["st"]["p"][1]["rv"] == 1, names(v)
    v = V(train.answer(json.dumps(0)))
    assert v["dec"]["kind"] == "main" and "?" not in names(v) and len(names(v)) == 2, names(v)
    v = V(train.act(next(o["i"] for o in v["dec"]["options"] if o["k"] == "end")))
    assert "rv" not in v["st"]["p"][1] and set(names(v)) <= {"?"}, names(v)


@T.test
def table_shows_facedown_cards_after_scuttle_crab_this_turn():
    table_at_my_main(12)
    g = train.W["g"]
    opp_hand(g)
    fd = Obj("Back Off", 1)
    fd.zone, fd.hidden_turn, fd.hidden_bf = "facedown", g.turn_no - 1, 1
    g.bfs[1].facedowns.append(fd)
    put(g, 1, "Arena Kingpin", 1)
    g.bfs[1].ctrl = 1
    v = refresh()
    assert v["st"]["bfs"][1]["fd"] == [1, "?"]
    c = put(g, 0, "Scuttle Crab")
    g.kill([c])
    v = refresh()
    while v.get("dec") and v["dec"]["kind"] != "main":     # Deathknell sur la chaîne : on la laisse résoudre
        v = V(train.act(next(o["i"] for o in v["dec"]["options"] if o["k"] == "pass")))
    assert v["st"]["bfs"][1]["fd"] == [1, "Back Off"] and names(v) == ["Discipline", "Watchful Sentry", "Falling Star"], \
        (v["st"]["bfs"][1]["fd"], names(v), v.get("ask"), v.get("dec", {}).get("kind"))
    v = V(train.act(next(o["i"] for o in v["dec"]["options"] if o["k"] != "end")))
    assert v["st"]["bfs"][1]["fd"] == [1, "Back Off"]   # « this turn » : toujours visible après le coup suivant
    assert "rv" not in v["st"]["p"][1]                   # la main, elle, n'est plus révélée


@T.test
def table_shows_every_facedown_card_bandle_tree():
    # Bandle Tree : deux cartes cachées au même endroit ; la table doit recevoir les deux (la 2e n'était pas transmise)
    table_at_my_main(12)
    g = train.W["g"]
    mine = []
    for bf, pid, n in ((1, 0, "Back Off"), (1, 0, "Gust"), (0, 1, "Back Off"), (0, 1, "Gust")):
        fd = Obj(n, pid)
        fd.zone, fd.hidden_turn, fd.hidden_bf = "facedown", g.turn_no - 1, bf
        g.bfs[bf].facedowns.append(fd)
        if pid == 0:
            mine.append(fd.uid)
    v = refresh()
    assert v["st"]["bfs"][1]["fds"] == [[0, "Back Off"], [0, "Gust"]] and v["st"]["bfs"][1]["fdus"] == mine, v["st"]["bfs"][1]
    assert v["st"]["bfs"][0]["fds"] == [[1, "?"], [1, "?"]] and v["st"]["bfs"][0]["fdus"] == [None, None], v["st"]["bfs"][0]


if __name__ == "__main__":
    T.main()
