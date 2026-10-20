#!/usr/bin/env python3
"""Données Texel : parties IA contre IA, critères de chaque début de tour et gagnant final (2026-10-20).

    python3 texel/gen.py <parties> <graine0> <sh|pol> <sortie.jsonl> [processus]

Parties exactement comme exp_search.play en mode « defaut » (deux decks par défaut tirés au hasard, plans génériques,
mêmes graines d'agents) : sh = SearchAgent sh contre sh (IA Normal), pol = PolicyAgent des deux côtés (prototype rapide).
Position enregistrée = première décision « main » de chaque tour, vue par le joueur actif : c'est le moment où la
recherche évalue ses feuilles (ai.rollout_policy s'arrête au premier « main » après l'horizon).
Une ligne JSON par partie : seed, engine (version.engine_md5), mode, legends, first, winner, turns,
rows = [[tour, joueur actif, const, critères dans l'ordre features.KEYS], ...].
Reprise : les graines déjà présentes dans la sortie sont sautées (même version du moteur exigée)."""
import json, os, sys, time, traceback
from multiprocessing import Pool
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
_argv, sys.argv = sys.argv, sys.argv[:1]      # exp.py (importé par exp_search) lit sys.argv
import exp_search as E
sys.argv = _argv
import features as F
import ai
from game import Game, Obj, Item
from version import engine_md5


def play(seed, mode):
    Obj._n = 0
    Item._n = 0
    decks, plans, f = E.setup(seed, "defaut")
    if mode == "sh":
        ag = E.agents(seed, decks, plans, "sh", "sh")
    else:
        ag = [ai.PolicyAgent(), ai.PolicyAgent()]
    g = Game(decks, ag, seed=seed, first=f)
    rows = []
    last = None
    while True:
        d = g.advance()
        if d is None:
            break
        if d.kind == "main" and g.turn_no != last and g.winner is None:
            last = g.turn_no
            c, fv = F.features(g, d.player)
            rows.append([g.turn_no, d.player, round(c, 6)] + [round(fv[k], 6) for k in F.KEYS])
        g.apply(ag[d.player].decide(g, d))
    return dict(seed=seed, mode=mode, legends=[d["legend"] for d in decks], first=f, winner=g.winner,
                turns=g.turn_no, rows=rows)


def job(arg):
    seed, mode = arg
    t0 = time.time()
    try:
        r = play(seed, mode)
    except Exception:
        return dict(seed=seed, mode=mode, err=traceback.format_exc()[-800:])
    r["sec"] = round(time.time() - t0, 1)
    return r


def main():
    n, seed0, mode, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], Path(sys.argv[4])
    procs = int(sys.argv[5]) if len(sys.argv) > 5 else max(1, (os.cpu_count() or 2) - 4)
    assert mode in ("sh", "pol")
    ver = engine_md5()
    done = set()
    if out.exists():
        for line in out.open(encoding="utf-8"):
            r = json.loads(line)
            assert r["engine"] == ver, f"sortie d'une autre version du moteur ({r['engine']} ≠ {ver})"
            done.add(r["seed"])
    todo = [(seed0 + k, mode) for k in range(n) if seed0 + k not in done]
    print(f"moteur {ver}, {len(todo)} parties à jouer ({len(done)} déjà faites), {procs} processus", flush=True)
    t0 = time.time()
    errs = 0
    with Pool(procs) as p, out.open("a", encoding="utf-8") as fo:
        for i, r in enumerate(p.imap_unordered(job, todo, chunksize=1), 1):
            if "err" in r:
                errs += 1
                print("ERREUR", r["seed"], r["err"], flush=True)
                continue
            r["engine"] = ver
            fo.write(json.dumps(r, separators=(",", ":")) + "\n")
            fo.flush()
            if i % max(1, len(todo) // 50) == 0 or i == len(todo):
                el = time.time() - t0
                print(f"{i}/{len(todo)} parties, {el / 60:.1f} min, reste ≈ {el / i * (len(todo) - i) / 60:.0f} min, "
                      f"erreurs {errs}", flush=True)


if __name__ == "__main__":
    main()
