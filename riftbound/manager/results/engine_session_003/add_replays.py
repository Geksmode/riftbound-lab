#!/usr/bin/env python3
"""Ajoute au lecteur (../replays/) les parties marquantes d'une simulation de plans (exp_plans.py ou équivalent).

À faire après CHAQUE simulation (demande de l'utilisateur, 2026-10-03) :
  1. trouver les donnes instructives :   python3 add_replays.py flips results_plans.json "<config A>" "<config B>" <graine0>
     (graines où A gagne et B perd : même donne, seul le plan change)
  2. lire le déroulé d'une partie :      python3 add_replays.py digest <graine> <deck> <plan Akali|-> <plan LeBlanc|->
  3. écrire une spec JSON (liste d'objets id, group, seed, deck, a_plan, l_plan, title, note, tempo facultatif) puis
                                         python3 add_replays.py add spec.json
  4. republier l'artefact (voir ../replays/README.md).
Fil « Agent manager » : python3 add_replays.py picks ../manager/results/session_NNN_picks.json
  (chaque pick = {id, seed, title, note, job, session, plans_snapshot} ; job au format de manager/run_session.py ;
  la partie est rejouée avec la copie figée de plans.py de la session et vérifiée contre results/session_NNN.json).
Les parties sont déterministes : une graine + deck + plans redonne exactement la partie de la simulation."""
import json, sys
from multiprocessing import Pool
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "replays" / "games"
KEYS = ("joue", "déplace", "prend le", "+1 point", "meurt", "Résolution : LeBlanc, Deceiver", "Résolution : Baited Hook",
        "Résolution : Akali, Rogue", "Résolution : Stellacorn", "Résolution : Astral", "cache", "COMBAT", "décide")


def decks():
    argv, sys.argv = sys.argv, sys.argv[:1]           # exp.py lit sys.argv à l'import
    from exp_gorica import G, G2
    from exp import L
    sys.argv = argv
    return {"G2": G2, "G": G, "Gorica stock": G}, L


def flips(path, a, b, seed0, wa=1.0, wb=0.0):
    rs = [r for r in json.load(open(path)) if r.get("seed0") == seed0]
    R = {r["label"]: dict((int(k), v) for k, v in r["per_seed"]) for r in rs}
    A, B = R[a], R[b]
    return [s for s in sorted(A) if A[s] == wa and B.get(s) == wb]


def rec(seed, deck, a_plan, l_plan):
    from replay import record_plan
    D, L = decks()
    return record_plan(seed, D[deck], L, a_plan or None, l_plan or None)


def digest(rep, width=150):
    out = [f"graine {rep['seed']} plans {rep['plans']} premier={'Akali' if rep['first'] == 0 else 'LeBlanc'} "
           f"bf={[p['bf'] for p in rep['players']]} gagnant={'Akali' if rep['winner'] == 0 else 'LeBlanc'} {rep['pts']}",
           f"mulligan {rep.get('mulligan')}"]
    for fr in rep["frames"]:
        for ind, t in fr["ev"]:
            if t.startswith("Tour "):
                out.append(f"== {t}  score {fr['st']['pts']}")
            elif any(k in t for k in KEYS) and "Déclenchement" not in t:
                out.append("   " + t[:width])
    return "\n".join(out)


def _build(spec):
    from decks import load, edit
    d = load(spec["base"])
    return edit(d, out=[tuple(x) for x in spec.get("out", [])], add=[tuple(x) for x in spec.get("add", [])])


def _job(e):
    sys.argv = sys.argv[:1]
    import ai
    ai.TEMPO = e.get("tempo", ai.TEMPO)              # "tempo": false = ancienne IA (avant le 3 oct. au soir)
    if "job" in e:                                    # pick du manager
        if e.get("snap"):
            sys.path.insert(0, e["snap"])             # plans.py figé de la session, avant tout import de plans
        from replay import record_plan
        j = e["job"]
        rep = record_plan(e["seed"], _build(j["akali"]), _build(j["opp"]), j.get("akali_plan"), j.get("opp_plan"),
                          j.get("akali_bfs"), j.get("opp_bfs"), j.get("first"))
    else:
        rep = rec(e["seed"], e.get("deck", "G2"), e.get("a_plan"), e.get("l_plan"))
    json.dump(rep, open(OUT / f"{e['id']}.json", "w"), ensure_ascii=False, separators=(",", ":"))
    return dict(id=e["id"], file=f"{e['id']}.json", group=e.get("group", ""), session=e.get("session"), seed=e["seed"], title=e["title"],
                note=e["note"], plans=rep["plans"], winner=rep["winner"], pts=rep["pts"], first=rep["first"],
                turns=rep["turns"], bf=[p["bf"] for p in rep["players"]], ia="tempo" if ai.TEMPO else "ancienne")


def add(entries):
    with Pool(4, maxtasksperchild=1) as p:            # un processus par partie : plans.py figé différent possible
        new = p.map(_job, entries, chunksize=1)
    f = OUT / "index.json"
    idx = json.load(open(f)) if f.exists() else []
    ids = {e["id"] for e in new}
    idx = [e for e in idx if e["id"] not in ids] + new
    json.dump(idx, open(f, "w"), ensure_ascii=False, indent=1)
    for e in new:
        print(e["id"], e["seed"], e["plans"], "Akali" if e["winner"] == 0 else "LeBlanc", e["pts"])
    return new


def picks(path):
    """Ajoute les parties choisies par le manager ; vérifie que le gagnant rejoué est celui de la simulation."""
    path = Path(path).resolve()
    mdir = path.parent.parent                         # .../manager
    ps = json.load(open(path))
    entries = []
    for k in ps:
        snap = k.get("plans_snapshot")
        entries.append(dict(k, group=f"Agent manager : session {k.get('session', '?')} ({k['job'].get('label', '')})"[:120],
                            snap=str((mdir / snap).parent) if snap else None))
    new = add(entries)
    res = path.with_name(path.name.replace("_picks", ""))
    if res.exists():
        sim = {}
        data = json.load(open(res))
        for r in (data["results"] if isinstance(data, dict) else data):
            for row in r.get("per_seed", []):
                sim[(r.get("label"), int(row[0]))] = row[1]
        for k, e in zip(ps, new):
            w = sim.get((k["job"].get("label"), k["seed"]))
            got = 1.0 if e["winner"] == 0 else 0.5 if e["winner"] == -1 else 0.0
            if w is not None and w != got:
                print(f"ATTENTION {e['id']} : rejouée {got}, simulation {w}")


if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "flips":
        print(flips(sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5])))
    elif cmd == "digest":
        ap, lp = (None if x == "-" else x for x in sys.argv[4:6])
        print(digest(rec(int(sys.argv[2]), sys.argv[3], ap, lp)))
    elif cmd == "add":
        add(json.load(open(sys.argv[2])))
    elif cmd == "picks":
        picks(sys.argv[2])
