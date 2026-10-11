"""Politique de simulation de l'IA : réactions en showdown (ai.PolicyAgent.react_best, 2026-10-11)."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
import ai                                               # noqa: E402

T = Suite("ai_react")


def attack(g, att):
    """P1 (joueur actif) déplace att vers le battlefield 0 ; avance jusqu'à la première décision de P0 en showdown."""
    d = g.advance()
    mv = [o for o in d.options if o[0] == "move" and o[1] == (att.uid,) and o[2] == 0]
    g.apply(mv[0])
    for _ in range(50):
        d = g.advance()
        if d.player == 0 and d.kind in ("focus", "priority"):
            return d
        g.apply(("pass",))
    raise RuntimeError("pas de décision de P0")


def setup(attacker):
    g, ag = new(tp=1)
    runes(g, 0, ["Calm"] * 4)
    put(g, 0, "Stellacorn Herder", 0)               # 3 Might, P0 tient le battlefield 0
    att = put(g, 1, attacker, "base")
    hand(g, 0, "Long Sword")
    hand(g, 0, "Discipline")                        # +2 Might ce tour : retourne un combat 3 contre 4
    return g, att


@T.test
def defender_plays_the_spell_that_wins_the_combat():
    g, att = setup("Noxus Hopeful")                 # 4 Might contre 3
    d = attack(g, att)
    pol = ai.PolicyAgent()
    a = pol.decide(g, d)
    assert a[0] == "play", a
    assert pol.react_margin(g, 0, 0, a) > 0          # le sort choisi fait gagner le combat à P0
    assert g.sd is not None and g.sd.stage == "open"  # la copie n'a pas touché la partie


@T.test
def defender_passes_when_already_winning():
    g, att = setup("Watchful Sentry")               # 1 Might contre 3
    d = attack(g, att)
    assert ai.PolicyAgent().decide(g, d) == ("pass",)


if __name__ == "__main__":
    T.main()
