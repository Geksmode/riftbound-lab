"""Critères de l'évaluation (ai.evaluate) sous forme linéaire, pour le réglage Texel (2026-10-20).

evaluate(g, me, w) = const + somme(w[k] * f[k]) pour les clés de KEYS, quand w["pts_ramp"] = 0 et w["react_kw"] = 0
(valeurs par défaut ; ces deux réglages ne sont pas linéaires et restent hors du réglage). `const` regroupe ce qui ne
dépend d'aucun poids (Reflection et unités [Temporary], Mech, Karthus, unités empowered, équipements GEAR_V).
Chaque critère est la différence « moi − adversaire ». Miroir exact de ai.evaluate : test_texel.py vérifie l'égalité
sur des milliers de positions et des poids tirés au hasard ; toute modification d'ai.evaluate doit être reportée ici.

Ce fichier est hors de engine/*.py : il ne change pas version.engine_md5 (les données et les mesures restent sur la
même version du moteur)."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ai
from game import is_bf

KEYS = ("pts", "pts_hi", "bf", "fd", "hold_win", "unit0", "might", "cost", "on_bf", "card0", "card_e", "react",
        "rune", "leg_emp", "xp", "deck_low", "trick")


def features(g, me):
    """(const, {clé: valeur}) de la position g vue par `me` ; partie non finie."""
    assert g.winner is None
    f = dict.fromkeys(KEYS, 0.0)
    const = 0.0
    for pid, sign in ((me, 1.0), (1 - me, -1.0)):
        pl = g.p[pid]
        p = pl.points
        f["pts"] += sign * p
        f["pts_hi"] += sign * max(0, p - (g.victory - 3))
        held = 0
        for b in g.bfs:
            if b.ctrl == pid:
                f["bf"] += sign
                held += 1
                if b.facedown is not None and b.facedown.owner == pid:
                    f["fd"] += sign
        if ai.TEMPO and held and pl.points >= g.victory - 1:
            f["hold_win"] += sign
        has_unit = False
        for o in g.board:
            if o.ctrl != pid:
                continue
            if o.spec["type"] != "Unit":
                const += sign * ai.GEAR_V.get(o.cname, 1.0)
                continue
            has_unit = True
            # ai.unit_eval
            m = max(0, ai.lasting_might(g, o))
            sp = o.spec
            if o.cname == "Mech":                       # dernier remplacement dans unit_eval : il l'emporte
                const += sign * (0.8 + 0.8 * m)
            elif o.token and o.cname == "Reflection" or g.has_kw(o, "Temporary"):
                const += sign * (0.4 + 0.4 * m)
            else:
                f["unit0"] += sign
                f["might"] += sign * m
                f["cost"] += sign * (sp["e"] + 1.5 * sp["p"])
            if o.cname == "Karthus, Eternal":
                const += sign * 2.5
            if o.empowered:
                const += sign * 0.5
            if is_bf(o.loc):
                f["on_bf"] += sign
        for c in pl.hand:
            # ai.card_value (react_kw = 0)
            if c.cname in ai.REACTIVE:
                f["react"] += sign
            else:
                f["card0"] += sign
                f["card_e"] += sign * min(c.spec["e"], 8)
        if has_unit:
            nr = len(pl.runes)
            f["trick"] += sign * sum(1 for c in pl.hand if ai.is_trick(c.spec) and c.spec["e"] + c.spec["p"] <= nr)
        f["rune"] += sign * len(pl.runes)
        if pl.legend.empowered:
            f["leg_emp"] += sign
        f["xp"] += sign * pl.xp
        if len(pl.deck) < 3:
            f["deck_low"] -= sign
    return const, f


def linear(const, f, w):
    return const + sum(w[k] * f[k] for k in KEYS)
