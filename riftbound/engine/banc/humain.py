#!/usr/bin/env python3
"""Banc humain : les coups de l'IA que le joueur a marqués « Erreur de l'IA » sur la table (2026-10-28).

    python3 banc/humain.py <parties.json ou dossier> [...] [--ver <version>]... [--temoins <n>]

Chaque partie enregistrée par la table depuis le 2026-10-28 porte les coups de l'IA (champ « ai », train.ai_script) et
les marques du joueur (coups ["flag", raison, tour, indice du coup de l'IA, texte, précision écrite pour « autre »]). La partie est rejouée avec le moteur
actuel, l'IA suivant ses coups enregistrés (train.ScriptAI) : fidèle même si l'IA a changé depuis. À chaque coup marqué,
et à `temoins` coups non marqués de l'IA (témoins, à plusieurs options, tirés à graine fixe), la position est copiée et
chaque version de l'IA (exp_search.version : « turn », « turn@spell_pre=1 »…, comme la table : plan de la légende,
recherche du tour entier) y choisit son coup, avec un agent neuf.
Lecture : une version qui corrige les erreurs marquées change le coup marqué plus souvent qu'un coup témoin. Changer de
coup ne veut pas dire jouer le bon (le joueur n'a dit que « celui-là est mauvais ») : le nouveau coup est affiché pour
qu'il le juge. La même IA change aussi des coups d'elle-même (bruit de la recherche à 1 tirage) : comparer avec « turn »,
la version de la table.
Sortie : résumé (proportions avec intervalle de Wilson à 95 %) et banc/data/humain.jsonl (une ligne par position)."""
import json, math, os, random, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
_argv, sys.argv = sys.argv, sys.argv[:1]
import exp_search as E
sys.argv = _argv
import plans as P
import train
import train_games as TG
from game import Obj, Item
from replay import describe

REASONS = {"cible": "mauvaise cible", "moment": "mauvais moment", "inutile": "coup inutile", "autre": "autre"}


def wilson(k, n, z=1.96):
    if not n:
        return (0.0, 0.0, 0.0)
    p = k / n
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return (p, max(0.0, c - h), min(1.0, c + h))


def positions(game, n_ctrl, seed=0):
    """Rejoue la partie (coups du joueur depuis moves, de l'IA depuis ai) ; renvoie [(type, indice, marque, copie)]."""
    flags = [m for m in game["moves"] if m[0] == "flag"]
    ai = game["ai"]
    marked = {m[3]: m for m in flags if isinstance(m[3], int) and m[3] < len(ai) and ai[m[3]][0] == "d"}
    others = [i for i, e in enumerate(ai) if e[0] == "d" and e[3] > 1 and i not in marked]
    ctrl = sorted(random.Random(seed * 7919 + game["seed"]).sample(others, min(n_ctrl, len(others))))
    want = sorted(set(marked) | set(ctrl))
    TG.replay(game, capture=want)                       # l'IA suit son script (train.ScriptAI) et copie ces positions
    caught = train.W.get("caught", {})
    out = []
    for i in want:
        if i in caught:
            out.append(("marque" if i in marked else "temoin", i, marked.get(i), caught[i]))
    return out, len(want)


def agent(game, ver):
    sm, n, ex, h, cfg = E.version(ver)
    A, B = train.W["decks"]
    return P.PlanAgent(game["seed"] + 500000, plan=train._plan_for(B), opp_plan=train._plan_for(A), samples=n,
                       search=sm, sh_extra=ex, horizon=h, cfg=cfg)


def main(args):
    vers, files, n_ctrl = [], [], 20
    it = iter(args)
    for a in it:
        if a == "--ver":
            vers.append(next(it))
        elif a == "--temoins":
            n_ctrl = int(next(it))
        else:
            files.append(a)
    vers = vers or ["turn"]
    games = [g for g in TG.load_games(files) if g.get("ai") and any(m[0] == "flag" for m in g.get("moves", []))]
    if not games:
        sys.exit("aucune partie avec des coups de l'IA enregistrés et des erreurs marquées (table du 2026-10-28 ou après)")
    rows, missing = [], 0
    for game in games:
        pos, n_want = positions(game, n_ctrl)
        missing += n_want - len(pos)
        for typ, i, mark, (g, on, inn, d) in pos:
            a0 = d.options[game["ai"][i][1]]
            row = dict(id=game.get("id"), seed=game["seed"], step=i, typ=typ, turn=g.turn_no,
                       reason=mark[1] if mark else None, note=(mark[5] if mark and len(mark) > 5 else None) or None,
                       label=describe(g, a0, d.player), new={})
            for v in vers:
                Obj._n, Item._n = on, inn
                c = g.clone()
                a = agent(game, v).decide(c, d)
                row["new"][v] = dict(changed=a != a0, label=describe(g, a, d.player))
            rows.append(row)
    out = HERE / "data" / "humain.jsonl"
    out.parent.mkdir(exist_ok=True)
    with open(out, "w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    print(f"{len(games)} parties, {sum(r['typ'] == 'marque' for r in rows)} coups marqués, "
          f"{sum(r['typ'] == 'temoin' for r in rows)} témoins" + (f", {missing} positions non retrouvées" if missing else ""))
    for v in vers:
        line = []
        for typ in ("marque", "temoin"):
            rs = [r for r in rows if r["typ"] == typ]
            k = sum(r["new"][v]["changed"] for r in rs)
            p, lo, hi = wilson(k, len(rs))
            line.append(f"{typ}s changés {k}/{len(rs)} = {100 * p:.0f} % [{100 * lo:.0f} ; {100 * hi:.0f}]")
        print(f"  {v} : " + " ; ".join(line))
    for r in rows:
        if r["typ"] == "marque":
            why = REASONS.get(r["reason"], r["reason"]) + (f" : « {r['note']} »" if r.get("note") else "")
            print(f"  tour {r['turn']} ({why}) : {r['label']}")
            for v in vers:
                x = r["new"][v]
                print(f"      {v} : {'autre coup : ' + x['label'] if x['changed'] else 'même coup'}")


if __name__ == "__main__":
    main(sys.argv[1:])
