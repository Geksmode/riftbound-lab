"""Plan « général » déduit du deck (hypothèse à mesurer, pas une vérité). Additif : ne touche pas aux plans existants.

Le rôle (aggro / milieu / contrôle) vient de la courbe du deck : part d'unités à coût ≤ 3 et coût moyen des cartes.
  - mulligan : aggro garde les unités ≤ 3 et renvoie les cartes ≥ 5 ; contrôle garde tout sauf les cartes ≥ 7 ;
  - shape (fin de simulation) : aggro valorise la might posée sur les battlefields, contrôle valorise la main.
Activation dans le jeu : RB_GENERAL=1 (voir plan_for). Les plans Akali / LeBlanc ont priorité sur lui.
Aucun ordre de set ne compte ici.
"""
import os
from plans import Plan, PLANS
from game import SPEC

GENERAL = os.environ.get("RB_GENERAL", "0") == "1"
W_MIGHT = float(os.environ.get("RB_GEN_WM", "0.3"))   # poids de la might sur battlefield (aggro)
W_HAND = float(os.environ.get("RB_GEN_WH", "0.3"))    # poids de la main (contrôle)


def deck_role(deck):
    """Renvoie (rôle, part d'unités ≤3, coût moyen) pour un deck {main: [noms]}."""
    cost, cheap, n = 0.0, 0, 0
    for name in deck["main"]:
        sp = SPEC.get(name)
        if not sp:
            continue
        n += 1
        cost += sp["e"]
        if sp["type"] == "Unit" and sp["e"] <= 3:
            cheap += 1
    n = max(1, n)
    share, avg = cheap / n, cost / n
    if share >= 0.27 and avg <= 3.4:
        return "aggro", share, avg
    if avg >= 3.75:
        return "controle", share, avg
    return "milieu", share, avg


class GeneralPlan(Plan):
    name = "general"

    def __init__(s, deck):
        s.role, s.share, s.avg = deck_role(deck)

    def mulligan(s, g, pid):
        h = g.p[pid].hand
        lim = {"aggro": 5, "milieu": 6, "controle": 7}[s.role]
        out = [c for c in sorted(h, key=lambda c: -c.spec["e"]) if c.spec["e"] >= lim]
        if s.role == "aggro" and not any(c.spec["type"] == "Unit" and c.spec["e"] <= 3 for c in h):
            out += [c for c in sorted(h, key=lambda c: -c.spec["e"]) if c not in out and c.spec["e"] >= 4]
        return out[:2]

    def shape(s, g, me):
        if g.winner is not None:
            return 0.0
        if s.role == "aggro":
            return W_MIGHT * sum(max(0, g.might(u)) for u in g.units(me) if u.loc in (0, 1))
        if s.role == "controle":
            return W_HAND * len(g.p[me].hand)
        return 0.0


def plan_for(deck):
    """Plan à utiliser pour ce deck (Akali / LeBlanc : plans existants ; sinon général si RB_GENERAL=1)."""
    lg = deck.get("legend") or ""
    if lg.startswith("Akali"):
        return PLANS["Gorica"]()
    if lg.startswith("LeBlanc"):
        return PLANS["Hook tempo"]()
    return GeneralPlan(deck) if GENERAL else Plan()
