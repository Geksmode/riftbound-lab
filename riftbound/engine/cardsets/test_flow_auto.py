"""Flow (règle 829) sur TOUS les sorts modélisés qui l'ont : jouables depuis ta défausse pour leur coût de Flow imprimé
(coût alternatif qui remplace le coût de base, 829.1.c.1), puis bannis ; pas sans les ressources ; un sort sans Flow ne
se joue pas depuis la défausse. Run: python3 cardsets/test_flow_auto.py"""
import csv, os, re, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from actions import total_cost                          # noqa: E402
from game import ANY                                    # noqa: E402

T = Suite("flow_auto")
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ROWS = {r["name"]: r for r in csv.DictReader(open(os.path.join(ROOT, "cards", "cards_unique.csv"), encoding="utf-8"))}
FLOW = sorted(n for n, r in ROWS.items() if r["type"] == "Spell" and n in IMPL and re.search(r"\[Flow\] \d", r["text"]))
ALL = ["Fury", "Calm", "Mind", "Body", "Chaos", "Order"] * 3
NUM = {"1": 1, "2": 2, "3": 3, "4": 4, "5": 5}


def printed_flow(n):
    """« [Flow] 3 energy and 1 rune of any type » -> (3, [ANY]) ; « 2 order runes » -> (0, [{Order}, {Order}])."""
    t = re.search(r"\[Flow\] ([^(]*)\(", ROWS[n]["text"]).group(1).strip()
    e = int(re.match(r"(\d+) energy", t).group(1)) if re.match(r"\d+ energy", t) else 0
    reqs = []
    for k, dom in re.findall(r"(\d+) (\w+) runes?", t):
        reqs += [ANY if dom in ("rune", "runes") else frozenset({dom.capitalize()})] * int(k)
    for k in re.findall(r"(\d+) runes? of any type", t):
        reqs += [ANY] * int(k)
    return e, sorted(reqs, key=lambda r: sorted(r))


def board(g):
    put(g, 0, "Arena Kingpin", 0)
    g.bfs[0].ctrl = 0
    put(g, 0, "Brazen Buccaneer", "base")
    put(g, 1, "Determined Sentry", 1)     # Might 1, sans Deflect : cible de Public Execution (ennemi de Might inférieure)
    g.bfs[1].ctrl = 1
    put(g, 1, "Long Sword", "base")       # un équipement pour Brittle Steel (« Kill a gear »)


def trash_plays(g, c):
    d = g.advance()
    return [a for a in d.options if a[0] == "play" and a[1] == c.uid and a[2] == "trash"]


@T.test
def every_flow_spell_plays_from_trash_for_its_printed_flow_cost_then_is_banished():
    assert len(FLOW) >= 15, FLOW
    bad = []
    for n in FLOW:
        g, _ = new()
        g.every_choice = True
        runes(g, 0, ALL)
        board(g)
        c = Obj(n, 0)
        c.zone = "trash"
        g.p[0].trash.append(c)
        ps = trash_plays(g, c)
        if not ps:
            bad.append((n, "pas jouable depuis la défausse"))
            continue
        e, reqs = total_cost(g, 0, c, dict(ps[0][3]), "trash")
        if (e, sorted(reqs, key=lambda r: sorted(r))) != printed_flow(n):
            bad.append((n, "coût", (e, reqs), printed_flow(n)))
        g.apply(ps[0])
        settle(g)
        if c.zone != "banish":
            bad.append((n, "pas banni", c.zone))
    assert not bad, bad


@T.test
def no_flow_play_without_the_resources():
    bad = []
    for n in FLOW:
        g, _ = new()
        g.every_choice = True
        e, reqs = printed_flow(n)
        runes(g, 0, ALL[:max(0, e - 1)])      # une énergie de moins que le coût de Flow
        board(g)
        c = Obj(n, 0)
        c.zone = "trash"
        g.p[0].trash.append(c)
        if e and trash_plays(g, c):
            bad.append(n)
    assert not bad, bad


@T.test
def a_spell_without_flow_does_not_play_from_trash():
    g, _ = new()
    runes(g, 0, ALL)
    board(g)
    for n in ("Falling Star", "Discipline", "Block"):
        c = Obj(n, 0)
        c.zone = "trash"
        g.p[0].trash.append(c)
        assert not trash_plays(g, c), n


if __name__ == "__main__":
    T.main()
