"""Plans par légende (plans_legendes.py, 2026-10-11) : table complète, mulligan, évaluation, branchement dans exp_search."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
import plans_legendes as PL                             # noqa: E402
import plans as P                                       # noqa: E402
import train as TR                                      # noqa: E402

T = Suite("plans_legendes")


@T.test
def every_default_legend_has_a_plan_with_cards_of_its_deck():
    dd = TR.default_decks()
    assert sorted(dd) == sorted(PL.LEG), set(dd) ^ set(PL.LEG)
    for lg, d in PL.LEG.items():
        assert d["axe"] in ("C", "T") and d["style"] in PL.SEUIL and d["conf"] in ("lu", "recherche", "deck"), lg
        assert d["src"].startswith("https://") and d["note"], lg
        dk = TR.load(dd[lg])
        names = set(dk["main"]) | {dk["champion"]}
        bad = [n for n in d["garde"] + d["cles"] if n not in names]
        assert not bad, (lg, bad)


@T.test
def akali_and_leblanc_keep_their_measured_plans():
    assert isinstance(PL.plan_for({"legend": "Akali, Rogue Assassin"}), P.AkaliGorica)
    assert PL.plan_for({"legend": "LeBlanc, Deceiver"}).name == "Hook tempo"
    assert isinstance(PL.plan_for({"legend": "Viktor, Herald of the Arcane"}), PL.LegendPlan)
    assert type(PL.plan_for({"legend": "Inconnue"})) is P.Plan


@T.test
def mulligan_never_throws_kept_cards_and_throws_expensive_ones():
    g, ag = new()
    for n in ["Dazzling Aurora", "Mobilize", "Elder Dragon", "Harnessed Dragon"]:
        hand(g, 0, n)
    out = PL.LegendPlan("Volibear, Relentless Storm").mulligan(g, 0)   # ramp : garde Aurora et Mobilize
    assert sorted(c.cname for c in out) == ["Elder Dragon", "Harnessed Dragon"], [c.cname for c in out]


@T.test
def aggro_mulligan_digs_for_a_cheap_unit():
    g, ag = new()
    for n in ["Inferna", "Kai'Sa, Survivor", "Ferrous Forerunner", "Cleave"]:
        hand(g, 0, n)
    pl = PL.LegendPlan("Rengar, Pridestalker")                    # aggro, garde Inferna
    out = [c.cname for c in pl.mulligan(g, 0)]
    assert "Inferna" not in out and len(out) <= 2, out
    if not any(c.spec["type"] == "Unit" and c.spec["e"] <= 3 for c in g.p[0].hand):
        assert all(SPEC[n]["e"] >= 4 for n in out), out


@T.test
def conqueror_values_might_on_battlefields_and_key_units():
    g, ag = new()
    pl = PL.LegendPlan("Rengar, Pridestalker")
    v0 = pl.shape(g, 0)
    put(g, 0, "Kai'Sa, Survivor", "base")                      # carte clé en base : W_KEY
    v1 = pl.shape(g, 0)
    put(g, 0, "Noxus Hopeful", 0)                              # unité sur un battlefield : W_CONQ × might
    v2 = pl.shape(g, 0)
    assert abs(v1 - v0 - PL.W_KEY) < 1e-9, (v0, v1)
    assert v2 > v1


@T.test
def holder_values_held_battlefields_and_open_runes_near_defeat():
    g, ag = new()
    pl = PL.LegendPlan("Vex, Gloomist")
    runes(g, 0, ["Calm", "Calm", "Chaos"])
    hand_size = len(g.p[0].hand)
    v0 = pl.shape(g, 0)
    put(g, 0, "Mutated Mouser", 0)                             # tient le battlefield 0
    v1 = pl.shape(g, 0)
    assert abs(v1 - v0 - PL.W_HOLD) < 1e-9, (v0, v1)
    g.p[1].points = g.victory - 2                              # zone létale adverse : runes prêtes valorisées
    v2 = pl.shape(g, 0)
    assert abs(v2 - v1 - PL.W_OPEN * 2) < 1e-9, (v1, v2)
    assert hand_size == len(g.p[0].hand)


@T.test
def exp_search_option_gives_legend_plans_to_that_version_only():
    import exp_search as ES
    for seed in range(424242, 424262):
        decks, plans, f = ES.setup(seed, "defaut")
        if all(d["legend"] not in PL.EXISTING for d in decks):
            break
    a, b = ES.agents(seed, decks, plans, "sh@plan=leg", "sh")
    assert isinstance(a.plan, PL.LegendPlan) and a.plan.legend == decks[0]["legend"]
    assert isinstance(a.opp_plan, PL.LegendPlan) and a.opp_plan.legend == decks[1]["legend"]
    assert type(b.plan) is P.Plan and "plan" not in a.cfg
    b2, a2 = ES.agents(seed, decks, plans, "sh", "sh@plan=leg")     # places échangées
    assert type(b2.plan) is P.Plan and a2.plan.legend == decks[1]["legend"]


@T.test
def a_game_runs_with_legend_plans_on_both_sides():
    import exp_search as ES
    from game import Game
    decks, plans, f = ES.setup(424243, "defaut")
    ag = ES.agents(424243, decks, plans, "old@plan=leg,n=1,h=1", "old@plan=leg,n=1,h=1")
    g = Game(decks, ag, seed=424243, first=f)
    for _ in range(400):                                      # 400 décisions : le début de la partie, sans erreur
        d = g.advance()
        if d is None:
            break
        g.apply(ag[d.player].decide(g, d))


if __name__ == "__main__":
    T.main()
