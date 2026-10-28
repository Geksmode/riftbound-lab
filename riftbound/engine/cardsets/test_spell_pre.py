"""Pré-sélection des cibles de sorts par résolution immédiate (ai.spell_prefilter, 2026-10-28).
Run: python3 cardsets/test_spell_pre.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
import ai                                               # noqa: E402

T = Suite("spell_pre")


def plays(opts, name, g):
    pol = ai.PolicyAgent()
    return [o for o in opts if o[0] == "play" and pol.card_of(g, o).cname == name]


@T.test
def hextech_ray_keeps_only_the_target_it_kills():
    g, _ = new()
    runes(g, 0, ["Fury", "Fury", "Fury"])
    small = put(g, 1, "Mournful Witness", loc=1)
    put(g, 1, "Playful Phantom", loc=1)
    hand(g, 0, "Hextech Ray")
    d = g.advance()
    before = plays(d.options, "Hextech Ray", g)
    after = plays(ai.spell_prefilter(g, 0, d.options, 7), "Hextech Ray", g)
    assert len(before) == 2 and len(after) == 1, (before, after)
    assert dict(after[0][3])["tg"] == (small.uid,)
    assert all(o in ai.spell_prefilter(g, 0, d.options, 7) for o in d.options if o[0] != "play")


@T.test
def cleave_in_a_showdown_only_on_the_attacker():
    # Give a unit [Assault 3] this turn : sur le défenseur adverse, rien ; sur son attaquant, le combat est gagné
    g, _ = new()
    runes(g, 0, ["Fury"])
    me = put(g, 0, "Mournful Witness")
    put(g, 1, "Mournful Witness", loc=1)
    hand(g, 0, "Cleave")
    d = g.advance()
    g.apply(next(o for o in d.options if o[0] == "move" and o[1] == (me.uid,) and o[2] == 1))
    d = g.advance()
    while d.player != 0 or not plays(d.options, "Cleave", g):
        g.apply(("pass",))
        d = g.advance()
    assert g.sd is not None and len(plays(d.options, "Cleave", g)) == 2
    after = plays(ai.spell_prefilter(g, 0, d.options, 7), "Cleave", g)
    assert len(after) == 1 and dict(after[0][3])["tg"] == (me.uid,), after


@T.test
def search_agent_uses_it_only_with_the_flag():
    assert not ai.SPELL_PRE or os.environ.get("RB_SPELL_PRE")
    g, _ = new()
    runes(g, 0, ["Fury", "Fury", "Fury"])
    put(g, 1, "Mournful Witness", loc=1)
    big = put(g, 1, "Playful Phantom", loc=1)
    hand(g, 0, "Hextech Ray")
    d = g.advance()
    a = ai.SearchAgent(seed=1, cfg=dict(spell_pre=1)).decide(g, d)
    assert not (a[0] == "play" and dict(a[3]).get("tg") == (big.uid,)), a


if __name__ == "__main__":
    T.main()
