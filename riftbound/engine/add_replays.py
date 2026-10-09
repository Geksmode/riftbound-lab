#!/usr/bin/env python3
"""Ajoute au lecteur (../replays/) les parties marquantes d'une simulation de plans (exp_plans.py ou équivalent).

À faire après CHAQUE simulation (demande de l'utilisateur, 2026-10-03) :
  1. trouver les donnes instructives :   python3 add_replays.py flips results_plans.json "<config A>" "<config B>" <graine0>
     (graines où A gagne et B perd : même donne, seul le plan change)
  2. lire le déroulé d'une partie :      python3 add_replays.py digest <graine> <deck> <plan Akali|-> <plan LeBlanc|->
     (partie d'exp_search.py, IA générique : python3 add_replays.py dsearch <graine> <defaut|miroir> <v joueur0> <v joueur1>)
  3. écrire une spec JSON (liste d'objets id, group, seed, deck, a_plan, l_plan, title, note ; tempo, refl, video,
     search [version joueur 0, version joueur 1] d'exp_search.py, mode « defaut »/« miroir » (partie d'exp_search :
     decks et battlefields de setup(), deck/a_plan/l_plan ignorés) facultatifs) puis
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


def _kw(search):
    """[version joueur 0, version joueur 1] d'exp_search (ex. ["sh", "old@h=1"]) -> arguments des deux agents."""
    from exp_search import version
    out = []
    for v in search:
        sm, n, ex, h, cfg = version(v)
        out.append(dict(search=sm, samples=n, sh_extra=ex, horizon=h, cfg=cfg))
    return out


def rec(seed, deck, a_plan, l_plan, search=None, mode=None):
    """search : [version joueur 0, version joueur 1] d'exp_search (ex. ["sh", "old"]), sinon la recherche par défaut.
    mode : « defaut » ou « miroir » = la partie d'exp_search.py (decks, battlefields et premier joueur de setup(),
    IA générique des deux côtés) ; sinon Akali (deck) contre LeBlanc IQ#5."""
    import replay
    kw = _kw(search) if search else None
    if mode in ("defaut", "miroir"):
        import exp_search as X
        from game import Obj, Item
        Obj._n = 0
        Item._n = 0
        dk, _, f = X.setup(seed, mode)
        names = [d["legend"].split(",")[0] for d in dk]
        if names[0] == names[1]:
            names[1] += " (2)"
        replay.NAMES = tuple(names)
        return replay.record_plan(seed, dk[0], dk[1], None, None, [dk[0]["battlefield"]], [dk[1]["battlefield"]], f,
                                  agent_kw=kw)
    D, L = decks()
    return replay.record_plan(seed, D[deck], L, a_plan or None, l_plan or None, agent_kw=kw)


def digest(rep, width=150):
    nm = [p.get("name") or x for p, x in zip(rep["players"], ("Akali", "LeBlanc"))]
    win = nm[rep["winner"]] if rep["winner"] in (0, 1) else "nul"
    out = [f"graine {rep['seed']} plans {rep['plans']} joueurs {nm} premier={nm[rep['first']]} "
           f"bf={[p['bf'] for p in rep['players']]} gagnant={win} {rep['pts']}",
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
    import plans
    plans.KEEP_REFLECTION = e.get("refl", plans.KEEP_REFLECTION)   # false = LeBlanc rappelle le Reflet, cache peu
    plans.VIDEO = e.get("video", plans.VIDEO)                     # true = règles des vidéos LeBlanc
    if "job" in e:                                    # pick du manager
        if e.get("snap"):
            sys.path.insert(0, e["snap"])             # plans.py figé de la session, avant tout import de plans
        from replay import record_plan
        j = e["job"]
        rep = record_plan(e["seed"], _build(j["akali"]), _build(j["opp"]), j.get("akali_plan"), j.get("opp_plan"),
                          j.get("akali_bfs"), j.get("opp_bfs"), j.get("first"))
    else:
        rep = rec(e["seed"], e.get("deck", "G2"), e.get("a_plan"), e.get("l_plan"), e.get("search"), e.get("mode"))
    json.dump(rep, open(OUT / f"{e['id']}.json", "w"), ensure_ascii=False, separators=(",", ":"))
    return dict(id=e["id"], file=f"{e['id']}.json", group=e.get("group", ""), session=e.get("session"), seed=e["seed"], title=e["title"],
                note=e["note"], plans=rep["plans"], winner=rep["winner"], pts=rep["pts"], first=rep["first"],
                turns=rep["turns"], bf=[p["bf"] for p in rep["players"]], ia="tempo" if ai.TEMPO else "ancienne",
                names=[p["name"] for p in rep["players"]])


def add(entries):
    with Pool(4, maxtasksperchild=1) as p:            # un processus par partie : plans.py figé différent possible
        new = p.map(_job, entries, chunksize=1)
    f = OUT / "index.json"
    idx = json.load(open(f)) if f.exists() else []
    ids = {e["id"] for e in new}
    idx = [e for e in idx if e["id"] not in ids] + new
    json.dump(idx, open(f, "w"), ensure_ascii=False, indent=1)
    for e in new:
        print(e["id"], e["seed"], e["plans"], e["names"][e["winner"]] if e["winner"] in (0, 1) else "nul", e["pts"])
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
    elif cmd == "dsearch":                            # dsearch <graine> <defaut|miroir> <version j0> <version j1>
        print(digest(rec(int(sys.argv[2]), None, None, None, sys.argv[4:6], sys.argv[3])))
    elif cmd == "add":
        add(json.load(open(sys.argv[2])))
    elif cmd == "picks":
        picks(sys.argv[2])
