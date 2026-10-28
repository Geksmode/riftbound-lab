"""Choix pendant la résolution par simulation courte (ai.ask_sim, 2026-10-28) et point de reprise de Game.
Run: python3 cardsets/test_ask_sim.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
import ai                                               # noqa: E402

T = Suite("ask_sim")


class Spy(ai.SearchAgent):
    """SearchAgent qui note ses réponses aux asks."""

    def __init__(s, **kw):
        super().__init__(seed=3, **kw)
        s.seen = []

    def mulligan(s, g, pid):
        return []

    def choose(s, g, pid, kind, options, ctx):
        r = super().choose(g, pid, kind, options, ctx)
        s.seen.append((kind, r))
        return r


def faefolk_move(sim):
    # Faefolk (1 Might) va seule sur un battlefield vide ; l'adversaire a une 5-Might à sa base. « you may move an
    # enemy unit to that battlefield » : tirer la 5-Might, c'est perdre Faefolk au combat au lieu de conquérir.
    g, ag = new()
    spy = Spy(cfg=dict(ask_sim=sim))
    g.agents[0] = spy
    g.checkpoint = sim
    f = put(g, 0, "Irresistible Faefolk")
    put(g, 1, "Playful Phantom")
    g.bfs[0].ctrl = None
    d = g.advance()
    mv = next(o for o in d.options if o[0] == "move" and o[1] == (f.uid,) and o[2] == 0)
    g.apply(mv)
    settle(g)
    return g, spy, f


@T.test
def rule_pulls_the_big_enemy_unit_and_loses_faefolk():
    g, spy, f = faefolk_move(False)
    assert ("may", True) in spy.seen, spy.seen
    assert f not in g.board


@T.test
def ask_sim_declines_and_keeps_faefolk():
    g, spy, f = faefolk_move(True)
    assert ("may", False) in spy.seen, spy.seen
    assert f in g.board and f.loc == 0


@T.test
def checkpoint_is_never_copied():
    g, _ = new()
    g.checkpoint = True
    g.apply(g.advance().options[0])
    assert g._ckpt is not None
    c = g.clone()
    assert not c.checkpoint and c._ckpt is None and "_asklog" not in c.__dict__
    assert g._ckpt[0]._ckpt is None


@T.test
def replay_must_land_on_the_same_cards():
    # une carte de la main adverse révélée par l'effet est retirée au hasard dans le monde simulé : même nombre
    # d'options, autres cartes ; la signature (uid) les distingue et ask_sim garde alors la règle
    g, _ = new()
    a, b = hand(g, 1, "Hextech Ray"), hand(g, 1, "Cleave")
    c = g.clone()
    assert ai.opt_sig("sabotage", [a, b]) == ai.opt_sig("sabotage", c.p[1].hand)
    assert ai.opt_sig("sabotage", [a, b]) != ai.opt_sig("sabotage", [b, a])
    assert ai.opt_sig("may", [True, False]) == ("may", "True", "False")


@T.test
def card_played_by_the_opponent_stays_in_the_simulated_world():
    # SPRT du 2026-10-28, graine 5000214 : le coup rejoué était une carte jouée par l'adversaire, renvoyée dans son deck
    # par determinize (« card 68 not found »)
    import random
    g, _ = new()
    g.p[1].hand = []
    for n in ("Hextech Ray", "Cleave", "Hextech Ray"):
        hand(g, 1, n)
    x = g.p[1].hand[1]
    for k in range(20):
        c = g.clone()
        ai.determinize(c, 0, random.Random(k))
        ai.keep_played(c, 1, ("play", x.uid, "hand", {}))
        assert any(y.uid == x.uid and y.zone == "hand" for y in c.p[1].hand), k
        assert len(c.p[1].hand) == 3 and all(y.zone == "deck" for y in c.p[1].deck)


@T.test
def ask_sim_off_by_default_and_without_checkpoint():
    assert not ai.ASK_SIM or os.environ.get("RB_ASK_SIM")
    g, _ = new()
    s = ai.SearchAgent(cfg=dict(ask_sim=1))
    assert s.ask_sim(g, 0, "may", [True, False], True) is True     # pas de point de reprise : la règle


if __name__ == "__main__":
    T.main()
