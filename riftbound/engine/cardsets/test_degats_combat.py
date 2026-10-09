"""Assignation des dégâts de combat par le joueur humain (règle 465.2.c, Tank 815, Backline 826).
Run: python3 cardsets/test_degats_combat.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403

T = Suite("degats_combat")


def human_assigns(names, total, pick_order):
    """Le joueur 0 (humain : tous les choix) assigne `total` dégâts aux unités `names` du joueur 1 ; à chaque question
    il prend la première unité de `pick_order` encore proposée. Renvoie ({nom: dégâts}, questions posées)."""
    asked = []

    def pick(options, ctx):
        asked.append(sorted(o.cname for o in options))
        return next(o for n in pick_order for o in options if o.cname == n)
    g, _ = new(answers0={"damage_pick": pick})
    g.every_choice = True
    us = [put(g, 1, n, 0) for n in names]
    out = g.assign_damage(0, total, us)
    return {u.cname: n for u, n in out.items()}, asked


@T.test
def human_chooses_which_unit_takes_lethal_first():
    # 4 dégâts sur Mournful Witness (2) et Ruined Rex (5) : l'humain choisit Rex d'abord -> 4 sur Rex, rien sur Witness
    out, asked = human_assigns(["Mournful Witness", "Ruined Rex"], 4, ["Ruined Rex"])
    assert out == {"Ruined Rex": 4}, out
    out, _ = human_assigns(["Mournful Witness", "Ruined Rex"], 4, ["Mournful Witness"])
    w = SPEC["Mournful Witness"]["might"]
    assert out == {"Mournful Witness": w, "Ruined Rex": 4 - w}, out
    assert asked == [["Mournful Witness", "Ruined Rex"]]


@T.test
def lethal_in_full_and_excess_on_the_last_unit():
    out, asked = human_assigns(["Mournful Witness", "Soaring Scout"], 20, ["Soaring Scout", "Mournful Witness"])
    s, w = SPEC["Soaring Scout"]["might"], SPEC["Mournful Witness"]["might"]
    assert out == {"Soaring Scout": s, "Mournful Witness": 20 - s}, out   # excédent sur la dernière (465.2.c.4)
    assert len(asked) == 1                                               # la dernière unité n'est pas demandée


@T.test
def tank_must_be_first_and_backline_last():
    out, asked = human_assigns(["Enthusiastic Promoter", "Mournful Witness", "Lecturing Yordle"], 30,
                               ["Enthusiastic Promoter", "Mournful Witness", "Lecturing Yordle"])
    assert asked == [], asked                     # un seul choix permis à chaque palier : rien à demander
    t, w = SPEC["Lecturing Yordle"]["might"], SPEC["Mournful Witness"]["might"]
    assert out == {"Lecturing Yordle": t, "Mournful Witness": w, "Enthusiastic Promoter": 30 - t - w}, out
    out, asked = human_assigns(["Mournful Witness", "Soaring Scout", "Lecturing Yordle"], 30, ["Soaring Scout"])
    assert asked == [["Mournful Witness", "Soaring Scout"]]   # le Tank d'abord, puis le choix entre les deux autres


@T.test
def ai_keeps_its_own_order():
    g, _ = new()
    us = [put(g, 1, n, 0) for n in ("Mournful Witness", "Ruined Rex")]
    out = g.assign_damage(0, 4, us)
    assert sum(out.values()) == 4 and len(out) >= 1


@T.test
def zero_might_unit_needs_one_damage_so_four_does_not_clear_akali_and_crab():
    # 142.4.b : une unité à 0 Might n'a des dégâts mortels qu'à partir de 1. Akali, Silent (4) + Scuttle Crab (0) en
    # défense : 4 dégâts tuent Akali, le Crab survit et le défenseur garde le battlefield ; il en faut 5 (retour utilisateur).
    g, _ = new(tp=1)
    a = put(g, 1, "Master Yi, Tempered", "base")             # Might 4
    put(g, 0, "Akali, Silent", 0)
    put(g, 0, "Scuttle Crab", 0)
    g.bfs[0].ctrl = 0
    g.apply([o for o in options_of(g, "move") if o[1] == (a.uid,) and o[2] == 0][0])
    settle(g)
    assert [u.cname for u in g.units(0, 0)] == ["Scuttle Crab"] and g.bfs[0].ctrl == 0 and g.p[1].points == 0


if __name__ == "__main__":
    T.main()
