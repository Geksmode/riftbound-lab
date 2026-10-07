#!/usr/bin/env python3
"""Rejoue les parties jouées sur la table d'entraînement et en tire ce que le joueur a fait.

La page enregistre chaque partie (collection "games" de sa base) : réglages, suite exacte des choix
(["act", i, libellé, tour], ["ans", x, libellé, tour], ["undo"], ["hint", n]) et journal affiché. Le moteur est
déterministe : en rejouant ces choix avec train.py on retrouve toute la partie, y compris les coups de LeBlanc.

  python3 train_games.py riftbound/train/games/          -> une fiche par partie dans riftbound/train/analyses/
  python3 train_games.py fichier.json --coach            -> ajoute, à chaque coup, ce que l'IA Akali aurait joué

Chaque fiche : résultat, vérification (la partie rejouée finit-elle comme sur la page ?), et tour par tour tes coups,
ceux de LeBlanc et le score. Le fichier choices.jsonl rassemble tous tes coups (état résumé + choix) : c'est la matière
pour apprendre à l'IA de LeBlanc à anticiper ta façon de jouer."""
import json, os, sys, glob
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import train

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "train", "analyses")


def load_games(paths):
    out = []
    for p in paths:
        files = sorted(glob.glob(os.path.join(p, "*.json"))) if os.path.isdir(p) else [p]
        for f in files:
            d = json.load(open(f))
            for g in (d if isinstance(d, list) else [d]):
                g = g.get("data", g) if isinstance(g, dict) else g
                if isinstance(g, dict) and "moves" in g:
                    out.append(g)
    return out


def _state(v):
    st = v["st"]
    me, op = st["p"][0], st["p"][1]
    return dict(t=st["t"], pts=st["pts"], hand=len(me["hand"]), opp_hand=len(op["hand"]),
                runes=len(me["runes"]), opp_runes=len(op["runes"]),
                bfs=[dict(n=b["n"], c=b["c"], me=[u["n"] for u in b["u"] if u["c"] == 0],
                          opp=[u["n"] for u in b["u"] if u["c"] == 1]) for b in st["bfs"]],
                base=[u["n"] for u in me["base"]], opp_base=[u["n"] for u in op["base"]])


def replay(game, coach=False):
    """Rejoue une partie ; renvoie (journal, coups du joueur avec leur état, vue finale)."""
    train.new(game["seed"], game.get("bf"), game.get("first"), game.get("level", 1))
    v = json.loads(train.step())
    log, mine = [], []

    def drain(v):
        while True:
            log.extend(("  " if ind else "") + t for ind, t in v["log"])
            if not v.get("busy"):
                return v
            v = json.loads(train.step())
    v = drain(v)
    for m in game["moves"]:
        k = m[0]
        if k == "hint":
            train.hint(m[1])
            continue
        if k == "undo":
            v = drain(json.loads(train.undo()))
            if mine:
                # le coup repris ne compte pas comme un choix du joueur
                while mine and mine[-1]["k"] != "act":
                    mine.pop()
                if mine:
                    mine.pop()
            continue
        rec = dict(k=k, x=m[1], label=m[2] if len(m) > 2 else "", state=_state(v))
        if k == "act" and v.get("dec"):
            o = v["dec"]["options"][m[1]]
            rec.update(kind=o["k"], phase=v["dec"]["kind"], n_opts=len(v["dec"]["options"]), label=o["label"])
            if coach:
                h = json.loads(train.hint(1))
                rec["coach"] = h[0]["label"] if h else None
            v = drain(json.loads(train.act(m[1])))
        elif k == "ans" and v.get("ask"):
            rec.update(kind="ask:" + v["ask"]["kind"])
            v = drain(json.loads(train.answer(json.dumps(m[1]))))
        else:
            rec["desync"] = True
            mine.append(rec)
            break
        mine.append(rec)
    return log, mine, v


def sheet(game, log, mine, v):
    res = game.get("result") or {}
    w = v.get("winner")
    ok = (not res) or (res.get("winner") == w and res.get("pts") == v["st"]["pts"])
    if game.get("log") and len(game["log"]) < 2500 and not any(m[0] == "undo" for m in game["moves"]):
        # la page garde le journal affiché (sans les lignes retirées par « Reprendre ») : il doit être le même
        ok = ok and [l.strip() for l in game["log"]] == [l.strip() for l in log]
    lines = [f"# Partie {game['id']} ({game.get('started', '')[:16].replace('T', ' ')})", "",
             f"- Donne {game['seed']}, battlefields {' / '.join(game.get('bfs') or [])}, "
             f"{'tu commences' if game.get('whoFirst') == 0 else 'LeBlanc commence'}, IA niveau {game.get('level')}",
             f"- Résultat : {'victoire Akali' if w == 0 else 'victoire LeBlanc' if w == 1 else 'partie non finie'} "
             f"{v['st']['pts'][0]}-{v['st']['pts'][1]} au tour {v['st']['t']}",
             f"- Rejeu fidèle à la page : {'oui' if ok and not any(m.get('desync') for m in mine) else 'NON, à vérifier'}",
             f"- Tes décisions : {len(mine)} (dont {sum(1 for m in mine if m['k'] == 'act' and m.get('kind') not in ('pass', 'end'))} coups joués)", ""]
    coach = [m for m in mine if m.get("coach")]
    if coach:
        same = sum(1 for m in coach if m["coach"] == m["label"])
        lines.append(f"- Même coup que l'IA Akali : {same}/{len(coach)}")
        lines.append("")
    lines.append("## Journal")
    lines += ["```"] + log + ["```"]
    return "\n".join(lines)


def main(args):
    coach = "--coach" in args
    paths = [a for a in args if not a.startswith("--")] or [os.path.join(OUT, "..", "games")]
    games = load_games(paths)
    os.makedirs(OUT, exist_ok=True)
    allc = open(os.path.join(OUT, "choices.jsonl"), "w")
    for g in games:
        log, mine, v = replay(g, coach)
        open(os.path.join(OUT, f"{g['id']}.md"), "w").write(sheet(g, log, mine, v))
        for m in mine:
            allc.write(json.dumps(dict(game=g["id"], won=v.get("winner") == 0, **m), ensure_ascii=False) + "\n")
        print(g["id"], "gagnant", v.get("winner"), v["st"]["pts"], len(mine), "décisions")
    allc.close()


if __name__ == "__main__":
    main(sys.argv[1:])
