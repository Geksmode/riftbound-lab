"""Fermetures de capacités et copie profonde de la table (train.py).
La table gère un choix humain (et un conseil, train.hint) en restaurant la partie depuis une copie profonde ; les
fonctions passées à queue_trigger / g.effects ne sont pas copiées et gardent les objets de l'ANCIENNE partie. Une carte
doit donc relire ses objets par uid (et oid) dans ces fermetures. Run: python3 cardsets/test_table_copie.py"""
import os, sys, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *          # noqa: F401,F403

T = Suite("table_copie")


def _main_of_human():
    """Nouvelle partie de table (graine fixe) jusqu'à la première décision principale de l'humain."""
    import train
    v = json.loads(train.new(4, None, 0))
    for _ in range(300):
        if v.get("busy"):
            v = json.loads(train.step())
        elif v.get("ask"):
            v = json.loads(train.answer(json.dumps([] if v["ask"]["kind"] == "mulligan" else 0)))
        elif v.get("dec") and v["dec"]["kind"] == "main":
            return train, v
        else:
            v = json.loads(train.act(0)) if v.get("dec") else json.loads(train.step())
    raise AssertionError("pas de décision principale de l'humain")


def _refresh(train):
    train.W["d"] = None
    v = json.loads(train.step())
    while v.get("busy"):
        v = json.loads(train.step())
    return v


def _play_and_settle(train, v, uid, loc, on_priority=None):
    """Joue la carte uid (loc pour une unité), répond 0 aux questions et passe jusqu'au retour en phase principale.
    on_priority(train) est appelé à chaque priorité de l'humain sur une chaîne non vide, jusqu'à ce qu'il rende vrai."""
    i = [o["i"] for o in v["dec"]["options"] if o["k"] == "play" and o.get("src") == uid
         and (loc is None or o.get("loc") == loc)][0]
    v = json.loads(train.act(i))
    asked = []
    for _ in range(80):
        if v.get("busy"):
            v = json.loads(train.step())
        elif v.get("ask"):
            asked.append(v["ask"]["kind"])
            v = json.loads(train.answer(json.dumps(0)))
        elif v.get("dec") and v["dec"]["kind"] != "main":
            if on_priority is not None and train.W["g"].chain and on_priority(train):
                on_priority = None
                v = _refresh(train)
                continue
            v = json.loads(train.act(next(o["i"] for o in v["dec"]["options"] if o["k"] == "pass")))
        else:
            return v, asked
    raise AssertionError(asked)


@T.test
def ashe_returns_the_banished_card_after_a_table_restore():
    # Ashe, Focused : « When they hold, return it to their hand ». La carte exilée doit revenir même si la table a
    # restauré la partie (conseil demandé) entre l'exil et le hold : avant le correctif, `c in banish` comparait
    # l'objet de l'ancienne partie et la carte restait exilée pour toujours.
    from game import Obj, Rune
    train, v = _main_of_human()
    g, me, ai = train.W["g"], train.ME, train.AI
    g.p[me].runes = [Rune("Order", me) for _ in range(6)]
    c = Obj("Ashe, Focused", me)
    c.zone = "hand"
    g.p[me].hand.append(c)
    assert g.p[ai].hand
    v = _refresh(train)
    v, asked = _play_and_settle(train, v, c.uid, "base")
    g = train.W["g"]
    assert "ashe_pick" in asked or len(g.p[ai].banish) == 1, asked
    assert len(g.p[ai].banish) == 1, [x.cname for x in g.p[ai].banish]
    x = g.p[ai].banish[0].uid
    g.p[me].runes = [Rune("Order", me) for _ in range(4)]     # au moins deux options : le conseil est calculé
    y = Obj("Soaring Scout", me)
    y.zone = "hand"
    g.p[me].hand.append(y)
    _refresh(train)
    g0 = train.W["g"]
    assert json.loads(train.hint())                 # le conseil calcule sur une copie : la partie n'est plus remplacée
    assert train.W["g"] is g0
    train._restore(train._save())                   # ce que font une question à l'humain et « Reprendre »
    g = train.W["g"]
    assert g is not g0 and any(e.get("on") == "hold" for e in g.effects)
    g.hold(ai, g.bfs[0])
    assert any(y.uid == x for y in g.p[ai].hand) and not g.p[ai].banish, \
        ([y.cname for y in g.p[ai].banish], [e for e in g.effects if e.get("on") == "hold"])


@T.test
def fizz_recycles_the_replayed_spell_after_a_table_restore():
    # Fizz, Trickster : « Recycle that spell after you play it » (le sort finit au fond du deck, pas à la défausse).
    # La partie est restaurée depuis une copie profonde pendant que le sort attend sur la chaîne (ce que fait la
    # table à chaque question posée à l'humain ou conseil) : avant le correctif, `c in trash` comparait l'objet de
    # l'ancienne partie et le sort restait à la défausse.
    from game import Obj, Rune
    train, v = _main_of_human()
    g, me, ai = train.W["g"], train.ME, train.AI
    g.p[me].runes = [Rune("Chaos", me) for _ in range(6)]
    s = Obj("Rebuke", me)
    s.zone = "trash"
    g.p[me].trash.append(s)
    e = Obj("Soaring Scout", ai)
    g.enter_board(e, ai, 1, ready=True)
    g.bfs[1].ctrl = ai                              # pas de combat ni de showdown à ouvrir
    c = Obj("Fizz, Trickster", me)
    c.zone = "hand"
    g.p[me].hand.append(c)
    v = _refresh(train)

    seen = []

    def restore(train):                             # quand le sort rejoué attend sur la chaîne
        if not any(e.get("on") == "played" for e in train.W["g"].effects):
            return False
        g0 = train.W["g"]
        train._restore(train._save())               # ce que font _run (NeedChoice), hint et undo
        seen.append(train.W["g"] is not g0)
        return True
    v, asked = _play_and_settle(train, v, c.uid, "base", on_priority=restore)
    g = train.W["g"]
    assert "fizz_pick" in asked and seen == [True], (asked, seen)
    assert any(y.uid == s.uid for y in g.p[me].deck) and not any(y.uid == s.uid for y in g.p[me].trash), \
        ([y.cname for y in g.p[me].trash], g.obj(e.uid))


if __name__ == "__main__":
    T.main()
