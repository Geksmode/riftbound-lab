"""Payer la puissance avec un Gold ou en recyclant une rune : la table pose la question au joueur (règle 429.3,
demande de l'utilisateur 2026-10-19). Run: python3 cardsets/test_paiement_gold.py"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import Suite                                # noqa: E402
import train                                             # noqa: E402
from cards import make_token                             # noqa: E402

T = Suite("paiement_gold")


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


def _play_flip(gold_ready, want):
    """Shuriken Flip (1 énergie + 1 puissance) avec deux runes prêtes et un Gold ; want(ask) -> index pour pay_gold.
    Renvoie (questions pay_gold vues, Gold encore en jeu, nombre de runes)."""
    v = _to_main()
    g, me = train.W["g"], train.ME
    gold = make_token(g, "Gold", me, "base", ready=gold_ready)
    v = json.loads(train.step())
    i = next(o["i"] for o in v["dec"]["options"] if o["k"] == "play" and "Shuriken Flip" in o["label"])
    v, asks = json.loads(train.act(i)), []
    for _ in range(40):
        if v.get("busy"):
            v = json.loads(train.step())
        elif v.get("ask"):
            a = v["ask"]
            if a["kind"] == "pay_gold":
                asks.append(a)
            v = json.loads(train.answer(json.dumps(want(a) if a["kind"] == "pay_gold" else 0)))
        else:
            break
    g = train.W["g"]
    return asks, any(o.cname == "Gold" and o.ctrl == me for o in g.board), len(g.p[me].runes)


@T.test
def gold_chosen_keeps_both_runes():
    asks, gold, n = _play_flip(True, lambda a: next(i for i, t in enumerate(a["options"]) if t.startswith("utiliser")))
    assert len(asks) == 1, asks
    assert asks[0]["options"] == ["recycler 1 rune(s) (Calm)", "utiliser 1 Gold"] or \
        asks[0]["options"] == ["recycler 1 rune(s) (Fury)", "utiliser 1 Gold"], asks[0]["options"]
    assert "Gold" in asks[0]["title"], asks[0]["title"]
    assert not gold and n == 2


@T.test
def rune_chosen_keeps_the_gold():
    asks, gold, n = _play_flip(True, lambda a: next(i for i, t in enumerate(a["options"]) if t.startswith("recycler")))
    assert len(asks) == 1 and gold and n == 1


@T.test
def exhausted_gold_asks_nothing():
    asks, gold, n = _play_flip(False, lambda a: 0)
    assert not asks and gold and n == 1


if __name__ == "__main__":
    T.main()
