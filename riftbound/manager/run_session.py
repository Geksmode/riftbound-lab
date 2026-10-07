#!/usr/bin/env python3
"""Runner de l'agent manager : python3 run_session.py [agenda.json] [budget_secondes].

Lit l'agenda (liste de jobs JSON), joue chaque job sur UN bloc de graines neuf commun à la session
(comparaisons appariées entre jobs de la même session), puis avance state.json pour que plus jamais
ces graines ne soient réutilisées. Résultats : results/<session>.json (avec per_seed).

Job : {"label": str, "n": 400,
       "akali": {"base": "akali_gorica_rq-singapore", "out": [["Defy", 3]], "add": [["Lonely Poro", 1]]},
       "opp":   {"base": "leblanc_gyatarina_ccs-iq5_1st", "out": [], "add": []},
       "akali_plan": "Gorica" | null, "opp_plan": "Deathknell" | null,
       "akali_bfs": [..] | null, "opp_bfs": [..] | null, "first": null | 0 | 1,
       "fresh": false}   # fresh=true : bloc de graines à part (confirmation d'une option retenue)
"""
import json, os, sys, time, random
from pathlib import Path
from multiprocessing import Pool

HERE = Path(__file__).resolve().parent
ENGINE = HERE.parent / "engine"
ENGINE_MD5 = None
RESUME = int(os.environ.get("RB_RESUME", "0")) if __name__ == "__main__" else 0
if RESUME:
    # Reprise d'une session interrompue (conteneur redémarré) : même moteur figé, mêmes graines, jobs manquants seulement.
    ENGINE = HERE / "results" / f"engine_session_{RESUME:03d}"
    ENGINE_MD5 = json.load(open(HERE / "results" / f"session_{RESUME:03d}.json"))["engine_md5"]
elif __name__ == "__main__":
    # Figer tout le moteur (game, ai, cards, plans...) au début de la session : d'autres fils modifient ai.py et
    # plans.py, et une session ne doit jamais mélanger deux versions de l'IA.
    import shutil, hashlib
    _n = json.load(open(HERE / "state.json"))["session"] + 1
    _snap = HERE / "results" / f"engine_session_{_n:03d}"
    _snap.mkdir(parents=True, exist_ok=True)
    _h = hashlib.md5()
    # cardsets/ (modules de cartes importés par cards.py depuis le 2026-10-07) est figé avec le reste ;
    # le hachage n'inclut ses fichiers que s'il existe, donc les anciens hachages ne changent pas.
    for _f in sorted(ENGINE.glob("*.py")) + sorted((ENGINE / "cardsets").glob("*.py")):
        _rel = _f.relative_to(ENGINE).as_posix()
        _src = _f.read_text(encoding="utf-8")
        _h.update(_rel.encode() + _src.encode())
        # les modules repèrent riftbound/ par parents[1] : pointer la copie vers le vrai dossier
        _src = _src.replace("Path(__file__).resolve().parents[1]", f"Path({str(HERE.parent)!r})")
        (_snap / _rel).parent.mkdir(parents=True, exist_ok=True)
        (_snap / _rel).write_text(_src, encoding="utf-8")
    ENGINE, ENGINE_MD5 = _snap, _h.hexdigest()[:10]
sys.path.insert(0, str(ENGINE))
from decks import load, edit, with_bf  # noqa: E402
from cards import IMPL  # noqa: E402

STATE = HERE / "state.json"
SNAP = None   # dossier contenant la copie figée de plans.py pour cette session


def _plans():
    """plans.py figé au début de la session : un autre fil peut modifier engine/plans.py en cours de route."""
    if SNAP and SNAP not in sys.path:
        sys.path.insert(0, SNAP)
    import plans
    return plans


def build(spec):
    d = load(spec["base"])
    d = edit(d, out=[tuple(x) for x in spec.get("out", [])], add=[tuple(x) for x in spec.get("add", [])])
    for n in d["main"]:
        if n not in IMPL:
            raise ValueError(f"carte non modélisée : {n}")
    if len(d["main"]) + 1 != 40:   # le champion choisi est compté à part
        raise ValueError(f"{spec['base']} : {len(d['main'])}+1 cartes au main deck (40 attendues)")
    return d


def one(arg):
    i, a_deck, l_deck, ap, lp, abfs, lbfs, first, snap = arg
    global SNAP
    SNAP = snap
    P = _plans()
    from game import Game, Obj, Item
    Obj._n = 0; Item._n = 0
    r = random.Random(i * 7919)
    f = r.randrange(2) if first is None else first
    pa = P.PLANS[ap]() if ap else None
    pl = P.PLANS[lp]() if lp else None
    lbf = (pl.battlefield(l_deck, f == 1) if pl and not lbfs else None) or r.choice(lbfs or l_deck["battlefields"])
    abf = (pa.battlefield(a_deck, f == 0, lbf) if pa and not abfs else None) or r.choice(abfs or a_deck["battlefields"])
    ag = [P.PlanAgent(i, plan=pa, opp_plan=pl), P.PlanAgent(i + 500000, plan=pl, opp_plan=pa)]
    g = Game([with_bf(a_deck, abf), with_bf(l_deck, lbf)], ag, seed=i, first=f)
    try:
        while True:
            d = g.advance()
            if d is None:
                break
            g.apply(ag[d.player].decide(g, d))
    except Exception:
        import traceback
        return dict(err=traceback.format_exc()[-700:], seed=i)
    return dict(w=g.winner, turns=g.turn_no, first=f, abf=abf, lbf=lbf, seed=i)


def run(job, seed0):
    a, l = build(job["akali"]), build(job["opp"])
    n = job.get("n", 400)
    args = [(seed0 + k, a, l, job.get("akali_plan"), job.get("opp_plan"), job.get("akali_bfs"),
             job.get("opp_bfs"), job.get("first"), SNAP) for k in range(n)]
    with Pool(os.cpu_count()) as p:
        out = p.map(one, args, chunksize=2)
    good = [o for o in out if not o.get("err")]
    errs = [o for o in out if o.get("err")]
    w = [1.0 if o["w"] == 0 else 0.5 if o["w"] == -1 else 0.0 for o in good]
    wr = sum(w) / max(1, len(w))
    by = {k: round(sum(1 for o in good if o["first"] == fi and o["w"] == 0) / max(1, sum(1 for o in good if o["first"] == fi)), 3)
          for k, fi in (("akali_first", 0), ("opp_first", 1))}
    return dict(label=job["label"], job=job, n=len(good), wr=round(wr, 4),
                se=round((wr * (1 - wr) / max(1, len(good))) ** .5, 4),
                turns=round(sum(o["turns"] for o in good) / max(1, len(good)), 1), errors=len(errs),
                err=errs[0]["err"] if errs else None, by_first=by, seed0=seed0,
                per_seed=[[o["seed"], w[j], o["abf"], o["lbf"]] for j, o in enumerate(good)])


def paired(r1, r2):
    """Différence appariée r1 - r2 (mêmes graines) et son écart-type."""
    m1 = {s: w for s, w, *_ in r1["per_seed"]}
    d = [m1[s] - w for s, w, *_ in r2["per_seed"] if s in m1]
    if len(d) < 2:
        return None
    mu = sum(d) / len(d)
    var = sum((x - mu) ** 2 for x in d) / (len(d) - 1)
    return round(mu, 4), round((var / len(d)) ** .5, 4), len(d)


if __name__ == "__main__":
    agenda = Path(sys.argv[1]) if len(sys.argv) > 1 else HERE / "agenda.json"
    budget = float(sys.argv[2]) if len(sys.argv) > 2 else 3000
    jobs = json.load(open(agenda, encoding="utf-8"))["jobs"]
    st = json.load(open(STATE))
    session = RESUME or st["session"] + 1
    import shutil, hashlib
    snap = HERE / "results" / f"plans_session_{session:03d}"
    snap.mkdir(parents=True, exist_ok=True)
    if not RESUME:
        shutil.copy(ENGINE / "plans.py", snap / "plans.py")
    SNAP = str(snap)
    plans_md5 = hashlib.md5((snap / "plans.py").read_bytes()).hexdigest()[:10]
    print(f"moteur figé ({ENGINE_MD5}) dans {ENGINE}, plans.py {plans_md5}", flush=True)
    out_f = HERE / "results" / f"session_{session:03d}.json"
    if RESUME:
        prev = json.load(open(out_f))
        shared0, done = prev["shared_seed0"], prev["results"]
        print(f"reprise de la session {session} : {len(done)} job(s) déjà joué(s), graines {shared0}+", flush=True)
    else:
        span = max(j.get("n", 400) for j in jobs)
        shared0 = st["next_seed"]
        st["next_seed"] += span
        st["session"] = session
        json.dump(st, open(STATE, "w"), indent=1)   # réserver les graines avant de jouer
        done = []
    t0 = time.time()
    played = {r["label"] for r in done}
    for job in jobs:
        build(job["akali"]); build(job["opp"])   # valider avant de dépenser du calcul
    for job in jobs:
        if job["label"] in played:
            continue
        if time.time() - t0 > budget:
            print(f"BUDGET épuisé, non joué : {job['label']}", flush=True)
            continue
        if job.get("seed0") is not None:   # réparation : rejouer exactement un bloc déjà réservé
            s0 = job["seed0"]
        elif job.get("fresh"):
            st = json.load(open(STATE)); s0 = st["next_seed"]; st["next_seed"] += job.get("n", 400)
            json.dump(st, open(STATE, "w"), indent=1)
        else:
            s0 = shared0
        t = time.time()
        r = run(job, s0)
        done.append(r)
        json.dump(dict(session=session, shared_seed0=shared0, plans_md5=plans_md5, engine_md5=ENGINE_MD5, results=done), open(out_f, "w"), ensure_ascii=False)
        print(f"{r['label']:55s} {r['wr']:6.1%} ±{r['se']:.3f} n={r['n']} graines {s0}+ err={r['errors']} ({round(time.time()-t)}s)", flush=True)
    base = next((r for r in done if not r["job"].get("fresh")), None)
    for r in done:
        if r is not base and base and not r["job"].get("fresh"):
            p = paired(r, base)
            if p:
                print(f"  apparié vs '{base['label']}': {r['label']}: {p[0]*100:+.1f} ± {p[1]*100:.1f} pts (n={p[2]})", flush=True)
    print(f"SESSION {session} TERMINÉE -> {out_f}", flush=True)
