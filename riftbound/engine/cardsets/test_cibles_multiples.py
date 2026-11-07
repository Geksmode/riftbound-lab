"""Questions « jusqu'à N unités » (Bilgewater Tracker, radiance._choose_up_to) : la table reçoit pour chaque option la
liste des unités du groupe (ask.tg), pour les faire toucher au lieu de lister les combinaisons en texte (signalé par
l'utilisateur le 2026-11-07). Run: python3 cardsets/test_cibles_multiples.py"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import Suite                                # noqa: E402
import train                                             # noqa: E402
from game import Obj, Rune, DOMAINS                     # noqa: E402

T = Suite("cibles_multiples")


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


def _tracker_ask():
    _to_main()
    g, me = train.W["g"], train.ME
    g.p[me].runes = [Rune(d, me) for d in DOMAINS for _ in range(3)]
    c = Obj("Bilgewater Tracker", me)
    c.zone = "hand"
    g.p[me].hand.append(c)
    us = []
    for n in ("Soaring Scout", "Noxus Hopeful"):
        u = Obj(n, 1 - me)
        g.enter_board(u, 1 - me, "base", ready=True)
        us.append(u.uid)
    train.W["d"] = None
    v = json.loads(train.step())
    i = next(o["i"] for o in v["dec"]["options"] if o["k"] == "play" and "Bilgewater Tracker" in o["label"])
    v = json.loads(train.act(i))
    for _ in range(40):
        if v.get("ask") and v["ask"]["kind"] == "tracker_targets":
            return v, us, c.uid
        if v.get("busy"):
            v = json.loads(train.step())
        elif v.get("ask"):
            v = json.loads(train.answer(json.dumps(0)))
        elif v.get("dec") and any(o["k"] == "pass" for o in v["dec"]["options"]):
            v = json.loads(train.act(next(o["i"] for o in v["dec"]["options"] if o["k"] == "pass")))
        else:
            break
    raise RuntimeError("pas de question tracker_targets")


@T.test
def options_carry_their_units():
    v, us, me_uid = _tracker_ask()
    a = v["ask"]
    assert a["tg"] is not None and len(a["tg"]) == len(a["options"]), a
    assert [] in a["tg"], a["tg"]                                  # « aucune » : groupe vide
    assert any(sorted(t) == sorted(us) for t in a["tg"]), (a["tg"], us)
    assert all(len(t) <= 2 for t in a["tg"])
    # chaque option nomme exactement ses unités (le libellé texte reste pour le journal)
    assert all(len(t) == (0 if o in ("aucune", "none") else o.count(", ") + 1) for o, t in zip(a["options"], a["tg"]))


@T.test
def answering_the_group_damages_both():
    v, us, _ = _tracker_ask()
    k = next(i for i, t in enumerate(v["ask"]["tg"]) if sorted(t) == sorted(us))
    v = json.loads(train.answer(json.dumps(k)))
    for _ in range(40):
        if v.get("busy"):
            v = json.loads(train.step())
        elif v.get("ask"):
            v = json.loads(train.answer(json.dumps(0)))
        elif v.get("dec") and v["dec"]["kind"] != "main" and any(o["k"] == "pass" for o in v["dec"]["options"]):
            v = json.loads(train.act(next(o["i"] for o in v["dec"]["options"] if o["k"] == "pass")))
        else:
            break
    g = train.W["g"]
    for u in us:
        o = g.obj(u)
        assert o is None or o.damage == 1, (u, o and o.damage)       # 1 dégât (ou morte : Soaring Scout a 1 de Might)


@T.test
def other_asks_have_no_groups():
    _to_main()
    v = json.loads(train.step())
    assert v.get("ask") is None or v["ask"].get("tg") is None


if __name__ == "__main__":
    T.main()
