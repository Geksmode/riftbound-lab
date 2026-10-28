"""Unités [Ambush] gardées par la politique de simulation pour les jouer en réaction (ai.POL_AMBUSH, 2026-10-28).
Run: python3 cardsets/test_ambush_ia.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
import ai                                               # noqa: E402
from game import SPEC                                   # noqa: E402

T = Suite("ambush_ia")
VI, RENGAR = SPEC["Vi, Peacekeeper"], SPEC["Rengar, Trophy Hunter"]


def table(**cfg):
    g, _ = new()
    runes(g, 0, ["Order"] * 4 + ["Body"] * 4)
    hand(g, 0, "Vi, Peacekeeper")
    return g


@T.test
def ambush_is_kept_only_where_it_can_be_used():
    g = table()
    assert not ai.holds_ambush(g, 0, VI)                # aucune unité sur un battlefield : l'Ambush ne servirait pas
    put(g, 0, "Mournful Witness", loc=1)
    assert ai.holds_ambush(g, 0, VI)
    assert not ai.holds_ambush(g, 0, SPEC["Mournful Witness"])
    g2 = table()
    put(g2, 1, "Mournful Witness", loc=1)
    assert ai.holds_ambush(g2, 0, RENGAR) and not ai.holds_ambush(g2, 0, VI)   # Rengar : battlefield avec ennemis


def policy_move(g, ambush, lazy):
    pol = ai.PolicyAgent()
    pol.cfg = dict(ambush=ambush)
    d = g.advance(lazy=True) if lazy else g.advance()
    return pol.decide(g, d)


@T.test
def policy_keeps_vi_for_a_reaction_in_its_own_turn():
    for lazy in (False, True):
        g = table()
        put(g, 0, "Mournful Witness", loc=1)
        g.bfs[0].ctrl = 0                               # rien à conquérir : la politique joue une unité ou finit
        g.bfs[1].ctrl = 0
        a = policy_move(g, False, lazy)
        assert a[0] == "play" and ai.PolicyAgent().card_of(g, a).cname == "Vi, Peacekeeper", (lazy, a)
        b = policy_move(g, True, lazy)
        assert not (b[0] == "play" and ai.PolicyAgent().card_of(g, b).cname == "Vi, Peacekeeper"), (lazy, b)


@T.test
def kept_vi_is_played_as_a_reaction_when_it_wins_the_combat():
    # l'adversaire attaque le battlefield où l'on a une 2-Might avec une 4-Might : Vi (5) en réaction retourne le combat
    g, _ = new(tp=1)
    runes(g, 0, ["Order"] * 4 + ["Body"] * 4)
    hand(g, 0, "Vi, Peacekeeper")
    put(g, 0, "Mournful Witness", loc=1)
    att = put(g, 1, "Playful Phantom")
    d = g.advance()
    g.apply(next(o for o in d.options if o[0] == "move" and o[1] == (att.uid,) and o[2] == 1))
    pol = ai.PolicyAgent()
    pol.cfg = dict(ambush=1)
    for _ in range(20):
        d = g.advance()
        if d.player == 0 and d.kind in ("focus", "priority"):
            a = pol.decide(g, d)
            if a[0] == "play":
                break
            g.apply(a)
        else:
            g.apply(("pass",))
    assert a[0] == "play" and pol.card_of(g, a).cname == "Vi, Peacekeeper", a


@T.test
def tied_reaction_follows_the_policy():
    # même situation : si la recherche note toutes les réactions pareil, tie_react suit la politique (Vi en réaction)
    g, _ = new(tp=1)
    runes(g, 0, ["Order"] * 4 + ["Body"] * 4)
    hand(g, 0, "Vi, Peacekeeper")
    put(g, 0, "Mournful Witness", loc=1)
    att = put(g, 1, "Playful Phantom")
    d = g.advance()
    g.apply(next(o for o in d.options if o[0] == "move" and o[1] == (att.uid,) and o[2] == 1))
    for _ in range(20):
        d = g.advance()
        if d.player == 0 and d.kind in ("focus", "priority") and any(o[0] == "play" for o in d.options):
            break
        g.apply(("pass",))
    opts = list(d.options)
    tied = (opts[0], -9550.0, [(-9550.0, o) for o in opts])
    on = ai.SearchAgent(cfg=dict(tie_react=1, ambush=1)).tie_break(g, d, opts, tied)
    off = ai.SearchAgent(cfg=dict(ambush=1)).tie_break(g, d, opts, tied)
    assert on[0] == "play" and ai.PolicyAgent().card_of(g, on).cname == "Vi, Peacekeeper", on
    assert off == opts[0]
    split = (opts[0], 1.0, [(1.0 if o == opts[0] else 0.0, o) for o in opts])
    assert ai.SearchAgent(cfg=dict(tie_react=1)).tie_break(g, d, opts, split) == opts[0]   # une préférence : gardée


@T.test
def off_by_default():
    assert not ai.POL_AMBUSH or os.environ.get("RB_AMBUSH")
    assert not ai.TIE_REACT or os.environ.get("RB_TIE_REACT")


if __name__ == "__main__":
    T.main()
