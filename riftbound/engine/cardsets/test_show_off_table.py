"""[Show Off] sur la table (train.py) : quand une carte avec [Show Off] est jouée, le joueur choisit l'objet qu'il
montre (carte de sa main ou objet à lui en jeu qui remplit la condition) ou de ne rien montrer (« you may », 832).
Avant la correction du 2026-10-30, toutes ces variantes avaient le même libellé et la page n'en gardait qu'une
(inter.js uniq). Run: python3 cardsets/test_show_off_table.py"""
import os, sys, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from testkit import *          # noqa: F401,F403
from game import DOMAINS
from test_table_copie import _main_of_human, _refresh, _play_and_settle

T = Suite("show_off_table")


def _setup(*names):
    train, _ = _main_of_human()
    g = train.W["g"]
    runes(g, 0, [d for d in DOMAINS for _ in range(3)])
    cs = [hand(g, 0, n) for n in names]
    return train, g, cs, _refresh(train)


def _plays(v, uid):
    return [o for o in v["dec"]["options"] if o["k"] == "play" and o.get("src") == uid]


def _key(o):
    """Clé de dédoublonnage de la page (inter.js uniq) : une option qui la partage avec une autre disparaît."""
    return (o["label"].split("cible ")[0], tuple(sorted(map(str, o.get("t1") or []))), tuple(o.get("t2") or []),
            o.get("acc"), o.get("rep"), o.get("show"), str(o.get("loc")))


@T.test
def table_offers_each_object_to_show_and_no_show():
    train, g, (cc, mech), v = _setup("Incorrigible Construct", "Mega-Mech")
    ops = [o for o in _plays(v, cc.uid) if o.get("loc") == "base" and not o.get("acc")]
    assert len({_key(o) for o in ops}) == len(ops), [o["label"] for o in ops]
    assert any(o.get("show") == mech.uid and o.get("showW") == "hand" and o.get("showN") == "Mega-Mech" for o in ops), ops
    assert any(o.get("show") is None for o in ops), ops
    assert any("Show Off : Mega-Mech" in o["label"] for o in ops), [o["label"] for o in ops]


@T.test
def table_show_off_applies_when_chosen():
    train, g, (cc, mech), v = _setup("Incorrigible Construct", "Mega-Mech")
    o = next(o for o in _plays(v, cc.uid) if o.get("loc") == "base" and o.get("show") == mech.uid and not o.get("acc"))
    i = o["i"]
    v = json.loads(train.act(i))
    _settle(train, v)
    u = next(u for u in train.W["g"].units(0) if u.cname == "Incorrigible Construct")
    assert not u.exhausted                         # « I enter ready if you showed off »
    assert any(c.uid == mech.uid for c in train.W["g"].p[0].hand)   # la carte montrée reste en main


@T.test
def table_no_show_off_when_declined():
    train, g, (cc, mech), v = _setup("Incorrigible Construct", "Mega-Mech")
    o = next(o for o in _plays(v, cc.uid) if o.get("loc") == "base" and o.get("show") is None and not o.get("acc"))
    v = json.loads(train.act(o["i"]))
    _settle(train, v)
    u = next(u for u in train.W["g"].units(0) if u.cname == "Incorrigible Construct")
    assert u.exhausted


@T.test
def table_targeted_show_off_spell_keeps_every_target_per_show():
    """Primordial Roar : pour chaque objet montré (et sans Show Off), toutes les cibles légales, sans doublon."""
    train, g, (roar,), _ = _setup("Primordial Roar")
    big = put(g, 0, "Playful Phantom")
    es = [put(g, 1, "Shipyard Skulker", "base") for _ in range(3)]
    v = _refresh(train)
    ops = _plays(v, roar.uid)
    assert len({_key(o) for o in ops}) == len(ops), [o["label"] for o in ops]
    for show in (big.uid, None):
        tg = {tuple(o["t1"]) for o in ops if o.get("show") == show}
        assert all((e.uid,) in tg for e in es), (show, tg)
    o = next(o for o in ops if o.get("show") == big.uid and o["t1"] == [es[0].uid])
    v = json.loads(train.act(o["i"]))
    _settle(train, v)
    assert train.W["g"].obj(es[0].uid) is None     # 5 dégâts (Might de Playful Phantom montré)


def _settle(train, v):
    for _ in range(80):
        if v.get("busy"):
            v = json.loads(train.step())
        elif v.get("ask"):
            v = json.loads(train.answer(json.dumps(0)))
        elif v.get("dec") and v["dec"]["kind"] != "main":
            v = json.loads(train.act(next(o["i"] for o in v["dec"]["options"] if o["k"] == "pass")))
        else:
            return v
    raise AssertionError("la partie ne revient pas en phase principale")


if __name__ == "__main__":
    T.main()
