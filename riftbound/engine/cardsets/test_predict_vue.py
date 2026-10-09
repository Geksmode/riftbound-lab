"""Predict (436) / Vision (817) dans la table : la question montre les cartes vues (en grand, la carte en cours
surlignée) avec « la garder sur le dessus » / « la recycler (sous le deck) », et le dessus connu reste affiché sur
le deck tant que ces cartes restent dessus (retour utilisateur 2026-10-09). Run: python3 cardsets/test_predict_vue.py"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import Suite                                # noqa: E402
import train                                             # noqa: E402

T = Suite("predict_vue")


def _to_main():
    v = json.loads(train.new(4, None, 0))
    for _ in range(300):
        if v.get("busy"):
            v = json.loads(train.step())
        elif v.get("ask"):
            v = json.loads(train.answer(json.dumps([] if v["ask"]["kind"] == "mulligan" else 0)))
        elif v.get("dec") and v["dec"]["kind"] == "main":
            return v
        else:
            v = json.loads(train.act(0)) if v.get("dec") else json.loads(train.step())
    raise RuntimeError("pas de décision principale")


def _predict(n, answer):
    """Predict n pour moi ; answer(ask) -> index. Renvoie (questions vues, vue finale, dessus réel avant)."""
    _to_main()
    g, me = train.W["g"], train.ME
    before = [c.cname for c in g.p[me].deck[:n + 1]]
    g.queue_trigger(me, "Dramatic Visionary", lambda g_, it: g_.predict(it.ctrl, n))
    g.flush_triggers()
    train.W["d"] = None
    v, asks = json.loads(train.step()), []
    for _ in range(40):
        if v.get("busy"):
            v = json.loads(train.step())
        elif v.get("ask"):
            asks.append(v["ask"])
            v = json.loads(train.answer(json.dumps(answer(v["ask"]))))
        elif v.get("dec") and v["dec"]["kind"] != "main":
            v = json.loads(train.act(next(o["i"] for o in v["dec"]["options"] if o["k"] == "pass")))
        else:
            break
    return asks, v, before


@T.test
def predict_two_shows_both_cards_and_the_kept_one_stays_known():
    asks, v, before = _predict(2, lambda a: 1 if a["kind"] == "predict_recycle" and a["cur"] == 0 else 0)
    pr = [a for a in asks if a["kind"] == "predict_recycle"]
    assert len(pr) == 2 and all(a["show"] == before[:2] for a in pr), pr
    assert [a["cur"] for a in pr] == [0, 1]
    assert pr[0]["options"] == ["la garder sur le dessus", "la recycler (sous le deck)"]
    assert before[0] in pr[0]["title"] and "1/2" in pr[0]["title"]
    # la première recyclée, la deuxième gardée : elle est le dessus connu ; la suivante (jamais vue) ne l'est pas
    assert v["st"]["p"][train.ME]["known_top"] == [before[1]], v["st"]["p"][train.ME]["known_top"]


@T.test
def known_top_disappears_when_the_card_leaves_the_top():
    asks, v, before = _predict(1, lambda a: 0)
    assert v["st"]["p"][train.ME]["known_top"] == [before[0]]
    g, me = train.W["g"], train.ME
    g.draw(me, 1)
    assert json.loads(train.step())["st"]["p"][me]["known_top"] == []


if __name__ == "__main__":
    T.main()
