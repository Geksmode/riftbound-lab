#!/usr/bin/env python3
"""Combats où l'IA est attaquante dans les parties de la table (2026-10-11, question de l'utilisateur : « pourquoi l'IA
enclenche des combats perdants ? »).

Chaque partie est rejouée avec le moteur exact de son commit (train_games.commit_engine) ; on note chaque combat où
l'IA (P1) est l'attaquant (464.2.c.1 : le joueur dont les unités ont rendu le battlefield Contested, 190.3.a) : unités
et Might au début du showdown et au moment des dégâts, issue, et la valeur que la recherche de l'IA donnait à son coup.

  python3 combats_parties.py riftbound/train/games/games/fichier.json [...]   -> tableau + résumé
  (--json : une ligne par combat). Les parties d'avant la recherche du tour entier (pas de pick_turn) sont sautées."""
import glob, json, os, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))


# ---------------------------------------------------------------- travailleur : une partie, moteur RB_ENGINE
def worker(path):
    sys.path.insert(0, os.environ["RB_ENGINE"])
    sys.path.insert(1, HERE)
    import train, game as G, ai as A
    import train_games as TG
    rec, picks = {}, []

    def units(g, bf, pid):
        return [dict(n=u.cname, m=g.might(u), st=bool(u.stunned)) for u in g.units(pid, bf)]

    old_start, old_step, old_pick = G.Game.start_showdown, G.Game.combat_step, A.SearchAgent.pick_turn

    def start_showdown(s, b, combat):
        if s.logging and b.contested == 1:             # les copies de la recherche ne journalisent pas
            rec[(s.turn_no, b.idx)] = dict(turn=s.turn_no, tp=s.tp, bf=b.name, pts=[s.p[0].points, s.p[1].points],
                                           before=dict(ai=units(s, b.idx, 1), hu=units(s, b.idx, 0)),
                                           hu_ready=sum(1 for r in s.p[0].runes if not r.exhausted),
                                           hu_hand=len(s.p[0].hand))
        return old_start(s, b, combat)

    def combat_step(s):
        sd = s.sd
        r = rec.get((s.turn_no, sd.bf)) if s.logging and sd.attacker == 1 else None
        if r is not None and sd.stage == "damage":
            r["damage"] = dict(ai=units(s, sd.bf, 1), hu=units(s, sd.bf, 0))
        out = old_step(s)
        if r is not None and sd.stage == "end" and "after" not in r:
            r["after"] = dict(ai=units(s, sd.bf, 1), hu=units(s, sd.bf, 0), recalled=bool(sd.recalled))
        return out

    def pick_turn(s, g, d, opts):
        res = old_pick(s, g, d, opts)
        picks.append(dict(turn=g.turn_no, first=repr(res[0]), v=res[1],
                          plan=[repr(x) for x in (s.tplan[1] if s.tplan else [])]))
        return res

    G.Game.start_showdown, G.Game.combat_step, A.SearchAgent.pick_turn = start_showdown, combat_step, pick_turn
    g = json.load(open(path))
    _, _, v = TG.replay(g)
    res = g.get("result") or {}
    faithful = bool(res) and v.get("winner") == res.get("winner") and v["st"]["pts"] == res.get("pts")
    for k in sorted(rec):
        r = rec[k]
        if "damage" not in r:
            continue                                   # showdown sans défenseur
        mv = [p for p in picks if p["turn"] == r["turn"]
              and any(x.startswith("('move'") and "'base'" not in x for x in [p["first"]] + p["plan"])]
        r["v"] = mv[0]["v"] if mv else None
        print("C " + json.dumps(dict(id=g["id"], opp=g.get("names", ["", "?"])[1], faithful=faithful, **r),
                                ensure_ascii=False), flush=True)


# ---------------------------------------------------------------- pilote
def tot(us):
    return sum(max(0, u["m"]) for u in us if not u["st"])


def main(args):
    sys.path.insert(0, HERE)
    import train_games as TG
    games = TG.load_games([a for a in args if not a.startswith("--")])
    rows = []
    with tempfile.TemporaryDirectory() as tmp:
        jobs = []
        for g in games:
            eng = TG.commit_engine(g["commit"], tmp) if g.get("commit") else None
            if eng and "def pick_turn" in open(os.path.join(eng, "ai.py")).read():
                p = os.path.join(tmp, g["id"] + ".json")
                json.dump({k: x for k, x in g.items() if k != "_file"}, open(p, "w"))
                jobs.append((eng, p))
        print(f"{len(jobs)} parties sur {len(games)} rejouées (IA à recherche du tour entier)", file=sys.stderr)

        def run(job):
            out = subprocess.run([sys.executable, os.path.abspath(__file__), "--worker", job[1]],
                                 env=dict(os.environ, RB_ENGINE=job[0]), capture_output=True, text=True)
            return [json.loads(l[2:]) for l in out.stdout.splitlines() if l.startswith("C ")]
        with ThreadPoolExecutor(int(os.environ.get("RB_PROCS", "4"))) as ex:
            for rs in ex.map(run, jobs):
                rows += rs
    for r in rows:
        r["lost"] = not r["after"]["ai"] or r["after"]["recalled"]
        r["own_turn"] = r["tp"] == 1                    # sinon : unité adverse amenée par un effet (Blitzcrank…)
        r["fav"] = tot(r["before"]["ai"]) > tot(r["before"]["hu"])
    if "--json" in args:
        for r in rows:
            print(json.dumps(r, ensure_ascii=False))
        return
    for r in rows:
        print(f"{'PERD ' if r['lost'] else 'gagne'} {'tour IA ' if r['own_turn'] else 'ton tour'} {r['id']:20} "
              f"{r['opp'][:10]:10} T{r['turn']:<2} {r['bf'][:18]:18} Might avant {tot(r['before']['ai'])}-"
              f"{tot(r['before']['hu'])}, aux dégâts {tot(r['damage']['ai'])}-{tot(r['damage']['hu'])}  "
              f"valeur du coup {'?' if r['v'] is None else round(r['v'], 1)}{'' if r['faithful'] else '  (rejeu NON fidèle)'}")
    own = [r for r in rows if r["own_turn"]]
    print(f"\nTour de l'IA : {len(own)} combats, perdus {sum(r['lost'] for r in own)} "
          f"(favorables au début du showdown : {sum(r['lost'] and r['fav'] for r in own)}, "
          f"défavorables : {sum(r['lost'] and not r['fav'] for r in own)}). "
          f"Ton tour (unité adverse amenée par ton effet) : {len(rows) - len(own)}, perdus {sum(r['lost'] for r in rows if not r['own_turn'])}.")


if __name__ == "__main__":
    if sys.argv[1:2] == ["--worker"]:
        worker(sys.argv[2])
    else:
        main(sys.argv[1:])
