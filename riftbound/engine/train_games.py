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
import json, os, sys, glob, shutil, subprocess, tempfile
HERE = os.path.dirname(os.path.abspath(__file__))
# RB_ENGINE : rejouer avec une copie figée du moteur (train/engine_versions/<ver>, préparée par frozen_engine)
sys.path.insert(0, os.environ.get("RB_ENGINE") or HERE)
import train

OUT = os.environ.get("RB_ANALYSES") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "train", "analyses")


def load_games(paths):
    out = []
    for p in paths:
        files = sorted(glob.glob(os.path.join(p, "*.json"))) if os.path.isdir(p) else [p]
        for f in files:
            d = json.load(open(f))
            for g in (d if isinstance(d, list) else [d]):
                g = g.get("data", g) if isinstance(g, dict) else g
                if isinstance(g, dict) and "moves" in g:
                    g["_file"] = f
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
    base = (game["seed"], game.get("bf"), game.get("first"), game.get("level", 1))
    if game.get("obf"):                                 # match BO1/BO3 : battlefield de l'IA choisi avant la manche
        train.new(*base, game.get("mine"), game.get("opp"), game["obf"])
    elif game.get("mine") or game.get("opp"):
        train.new(*base, game.get("mine"), game.get("opp"))
    else:
        train.new(*base)                                # moteurs figés v10 : pas de decks personnalisés
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
        if k == "act" and v.get("dec") and m[1] < len(v["dec"]["options"]):
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


VERSIONS = os.path.join(HERE, "..", "train", "engine_versions")


def frozen_engine(ver, tmp):
    """Copie de train/engine_versions/<ver> dont les chemins de données pointent vers ce dépôt (comme le manager)."""
    src = os.path.join(VERSIONS, ver or "")
    if not ver or not os.path.isdir(src):
        return None
    dst = os.path.join(tmp, ver)
    shutil.copytree(src, dst)
    root = os.path.abspath(os.path.join(HERE, ".."))
    for dp, _, fs in os.walk(dst):
        for f in fs:
            if f.endswith(".py"):
                q = os.path.join(dp, f)
                t = open(q, encoding="utf-8").read()
                open(q, "w", encoding="utf-8").write(t.replace("Path(__file__).resolve().parents[1]", f"Path({root!r})"))
    return dst


def commit_engine(commit, tmp):
    """Le moteur exact d'un commit (git archive de riftbound/engine), chemins de données pointés vers ce dépôt."""
    dst = os.path.join(tmp, "commit-" + commit)
    if os.path.isdir(dst):
        return os.path.join(dst, "riftbound", "engine")
    os.makedirs(dst)
    root = os.path.abspath(os.path.join(HERE, "..", ".."))
    arc = subprocess.run(["git", "-C", root, "archive", commit, "riftbound/engine"], capture_output=True)
    if arc.returncode:
        return None
    subprocess.run(["tar", "-x", "-C", dst], input=arc.stdout, check=True)
    eng = os.path.join(dst, "riftbound", "engine")
    rb = os.path.abspath(os.path.join(HERE, ".."))
    for dp, _, fs in os.walk(eng):
        for f in fs:
            if f.endswith(".py"):
                q = os.path.join(dp, f)
                t = open(q, encoding="utf-8").read()
                open(q, "w", encoding="utf-8").write(t.replace("Path(__file__).resolve().parents[1]", f"Path({rb!r})"))
    return eng


def analyse(games, coach, emit, engine_label):
    rows = []
    for g in games:
        try:
            log, mine, v = replay(g, coach)
        except Exception as e:                         # deck inconnu de ce moteur, etc.
            print(("GAME " if emit else "") + json.dumps(dict(id=g["id"], error=f"{type(e).__name__}: {e}"[:300],
                                                              engine=engine_label)), flush=True)
            continue
        txt = sheet(g, log, mine, v).replace("\n## Journal", f"- Moteur du rejeu : {engine_label}\n\n## Journal", 1)
        open(os.path.join(os.environ.get("RB_SHEETS") or OUT, f"{g['id']}.md"), "w").write(txt)
        faithful = "Rejeu fidèle à la page : oui" in txt
        for m in mine:
            rows.append(dict(game=g["id"], ver=g.get("ver"), engine=engine_label, faithful=faithful,
                             won=v.get("winner") == 0, **m))
        print(("GAME " if emit else "") + json.dumps(dict(id=g["id"], winner=v.get("winner"), pts=v["st"]["pts"],
                                                          n=len(mine), faithful=faithful, engine=engine_label)), flush=True)
    return rows


def main(args):
    coach, frozen, emit = "--coach" in args, "--frozen" in args, "--emit" in args
    paths = [a for a in args if not a.startswith("--")] or [os.path.join(OUT, "..", "games")]
    games = load_games(paths)
    os.makedirs(OUT, exist_ok=True)
    if emit:                                            # sous-processus lancé avec RB_ENGINE : renvoie ses lignes
        for r in analyse(games, coach, True, os.environ.get("RB_ENGINE_LABEL", "?")):
            print("CHOICE " + json.dumps(r, ensure_ascii=False), flush=True)
        return
    rows = []
    if frozen:
        # Chaque partie est rejouée avec le moteur figé de sa version de page (ver), puis, si le rejeu n'est pas
        # fidèle, avec les versions suivantes et enfin le moteur actuel : on garde le premier rejeu fidèle.
        vers = sorted(d for d in os.listdir(VERSIONS) if os.path.isdir(os.path.join(VERSIONS, d)))
        left = {g["id"]: g for g in games}
        best = {}                                        # id -> (lignes de choix, résumé) du rejeu retenu
        with tempfile.TemporaryDirectory() as tmp:
            engines = {v: frozen_engine(v, tmp) for v in vers}
            # d'abord le moteur exact du commit noté par la page (GitHub Pages, depuis le 2026-10-08)
            commits = sorted({g["commit"] for g in games if g.get("commit")})
            for c in commits:
                e = commit_engine(c, tmp)
                if e:
                    engines["commit " + c] = e
            steps = ["commit " + c for c in commits if "commit " + c in engines] + vers + [None]
            for step in steps:
                if step and step.startswith("commit "):
                    todo = [g for g in left.values() if g.get("commit") == step[7:]]
                else:
                    todo = [g for g in left.values() if step is None or (g.get("ver") or "") <= step]
                if not todo:
                    continue
                sheets = os.path.join(tmp, "fiches-" + (step or "actuel").replace(" ", "-")); os.makedirs(sheets, exist_ok=True)
                env = dict(os.environ, RB_ENGINE=engines[step], RB_ENGINE_LABEL=(step if step.startswith("commit") else "figé " + step), RB_SHEETS=sheets) if step \
                    else dict(os.environ, RB_ENGINE_LABEL="actuel", RB_SHEETS=sheets)
                env.pop("RB_ENGINE", None) if step is None else None
                out = subprocess.run([sys.executable, os.path.abspath(__file__), "--emit"] + (["--coach"] if coach else [])
                                     + [g["_file"] for g in todo], env=env, capture_output=True, text=True)
                got = {}
                for line in out.stdout.splitlines():
                    if line.startswith("CHOICE "):
                        r = json.loads(line[7:]); got.setdefault(r["game"], [[], None])[0].append(r)
                    elif line.startswith("GAME "):
                        r = json.loads(line[5:]); got.setdefault(r["id"], [[], None])[1] = r
                for gid, (rs, summ) in got.items():
                    if summ and summ.get("faithful"):
                        best[gid] = (rs, summ, sheets); left.pop(gid, None)
                    elif summ and gid not in best and not summ.get("error"):
                        best[gid] = (rs, summ, sheets)       # non fidèle : gardé faute de mieux, signalé
            for gid, g in left.items():
                if gid in best:
                    best[gid][1]["faithful"] = False
            for gid in sorted(best):
                rs, summ, sheets = best[gid]
                shutil.copy(os.path.join(sheets, gid + ".md"), os.path.join(OUT, gid + ".md"))
                rows += rs
                print(json.dumps(summ, ensure_ascii=False))
        for gid in sorted(set(g["id"] for g in games) - set(best)):
            print(json.dumps(dict(id=gid, error="aucun moteur ne rejoue cette partie"), ensure_ascii=False))
    else:
        rows = analyse(games, coach, False, "actuel")
    with open(os.path.join(OUT, "choices.jsonl"), "w") as allc:
        for r in rows:
            allc.write(json.dumps(r, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main(sys.argv[1:])
