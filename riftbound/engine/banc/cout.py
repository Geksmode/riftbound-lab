#!/usr/bin/env python3
"""Coût en victoires des erreurs du banc (2026-10-28, demande de l'utilisateur : « ces erreurs ont BEAUCOUP d'impact
dans la partie », alors que les tests séquentiels de ask_sim et spell_pre ne montrent pas +2 points).

    python3 banc/cout.py <pos.jsonl> [tirages] [version] [processus] [seuil]

Pour chaque erreur du banc (regret ≥ seuil, 2 par défaut, pour les deux poids : banc.regret2), la partie est rejouée
d'après son script jusqu'à la position, puis jouée jusqu'au bout `tirages` fois avec le choix de l'IA (« mauvais ») et
autant de fois avec la meilleure option du juge (« bon »). Tirage k : mêmes decks remélangés, même aléa du jeu, mêmes
graines d'agents pour les deux choix (paire appariée) ; les deux joueurs continuent avec `version` (défaut « sh », plus
rapide que « turn » de la table). Coût d'une erreur = victoires du joueur qui se trompe avec le bon choix − avec le
mauvais (½ pour une nulle). Écart-type par grappes de parties (plusieurs erreurs d'une même partie ne sont pas
indépendantes). La suite voit la vraie position (mains comprises) : c'est l'issue réelle qui est mesurée.
Sorties : banc/data/cout-<fichier>.jsonl (une ligne par erreur) et banc/results_banc.json."""
import json, math, random, sys, time, traceback
from multiprocessing import Pool
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
_argv, sys.argv = sys.argv, sys.argv[:1]
import banc as B
import exp_search as E
sys.argv = _argv
from game import Game, Obj, Item
from version import engine_md5


def best_alt(p):
    """(indice du choix de l'IA, indice de la meilleure option pour les deux poids) dans p["alts"]."""
    vals = [(x[1], x[2]) for x in p["alts"]]
    i = p["chosen"] if p["typ"] == "ask" else 0
    k = max(range(len(vals)), key=lambda k: (min(vals[k][0] - vals[i][0], vals[k][1] - vals[i][1]), -k))
    return i, k


def at_step(game, step):
    """Partie rejouée d'après son script jusqu'à la décision `step` : (état copié, compteurs, décision, coup de l'IA,
    asks du segment qui suit)."""
    seed, mode = game["seed"], game["mode"]
    Obj._n = 0
    Item._n = 0
    decks, plans, f = E.setup(seed, mode)
    ptr = [0]
    sa = [B.Script(x, game["script"], ptr) for x in E.agents(seed, decks, plans, game["ver"], game["ver"])]
    g = Game(decks, sa, seed=seed, first=f)
    while True:
        d = g.advance()
        if d is None:
            raise B.Desync(f"partie finie avant le pas {step}")
        if ptr[0] == step:
            a0 = d.options[game["script"][step][1]]
            seg, k = [], step + 1
            while k < len(game["script"]) and game["script"][k][0] == "a":
                e = game["script"][k]
                seg.append(dict(enc=e[1], kind=e[2], n=e[3]))
                k += 1
            return g.clone(), (Obj._n, Item._n), d, a0, seg
        g.apply(sa[d.player].decide(g, d))


class Then:
    """Agent de la suite : les asks du segment d'avant le j-ième rejoués, le j-ième forcé sur alt, puis l'agent."""

    def __init__(s, ag, st):
        s.ag, s.st = ag, st

    def __getattr__(s, k):
        return getattr(s.ag, k)

    def decide(s, g, d):
        return s.ag.decide(g, d)

    def choose(s, g, pid, kind, options, ctx):
        st = s.st
        if st.get("k") is not None:
            k = st["k"]
            st["k"] += 1
            if k == st["j"]:
                st["k"] = None
                st["hit"] = (kind, len(options))
                return options[st["alt"]]
            if k < len(st["seg"]):
                r = B.dec(st["seg"][k]["enc"], options)
                if r is not B._BAD:
                    return r
        return s.ag.choose(g, pid, kind, options, ctx)


def playout(game, pre, cnt, action, st, k, ver):
    Obj._n, Item._n = cnt
    c = pre.clone()
    w = game["seed"] * 1000 + k
    r = random.Random(w)
    for pl in c.p:
        r.shuffle(pl.deck)
    c.rng = random.Random(w * 2 + 1)
    decks, plans, _ = E.setup(game["seed"], game["mode"])
    st = dict(st)
    c.agents = [Then(x, st) for x in E.agents(w, decks, plans, ver, ver)]
    c.apply(action)
    for _ in range(6000):
        d = c.advance()
        if d is None:
            break
        c.apply(c.agents[d.player].decide(c, d))
    return c.winner, st.get("hit")


def job(arg):
    game, p, k, ver = arg
    try:
        pre, cnt, d, a0, seg = at_step(game, p["step"])
        bad, good = best_alt(p)
        if p["typ"] == "play":
            alts = B.play_alts(pre, d, a0)[1]
            acts, sts = (alts[bad], alts[good]), ({}, {})
        else:
            acts = (a0, a0)
            sts = tuple(dict(k=0, j=p["j"], alt=x, seg=seg) for x in (bad, good))
        sc = lambda w: 0.5 if w in (None, -1) else (1.0 if w == p["pid"] else 0.0)
        out = []
        for a, st in zip(acts, sts):
            w, hit = playout(game, pre, cnt, a, st, k, ver)
            if st and hit != (p["kind"], p["n_all"]):
                raise B.Desync(f"ask {p['j']} non retrouvé : {hit}")
            out.append(sc(w))
        return dict(seed=p["seed"], step=p["step"], j=p.get("j"), k=k, bad=out[0], good=out[1])
    except Exception:
        return dict(seed=p["seed"], step=p["step"], k=k, err=traceback.format_exc()[-600:])


def cluster(per_game):
    """Moyenne des écarts par erreur et écart-type par grappes de parties : per_game = [[écart par erreur, ...], ...]."""
    xs = [x for g in per_game for x in g]
    n = len(xs)
    if n < 2:
        return (xs[0] if xs else 0.0), 0.0
    m = sum(xs) / n
    k = len(per_game)
    v = k / (k - 1) * sum((sum(g) - m * len(g)) ** 2 for g in per_game) / n ** 2
    return m, math.sqrt(v)


def main(args):
    fp = Path(args[0])
    K = int(args[1]) if len(args) > 1 else 16
    ver = args[2] if len(args) > 2 else "sh"
    procs = int(args[3]) if len(args) > 3 else 4
    thr = float(args[4]) if len(args) > 4 else B.REGRET
    pos, games = B.load([fp])
    by = {g["seed"]: g for g in games}
    errs = [p for p in pos if p["regret"] >= thr and (p["typ"] == "ask" or p["n_all"] > 1)]
    jobs = [(by[p["seed"]], p, k, ver) for p in errs for k in range(K)]
    t0 = time.time()
    res, bad = {}, []
    with Pool(procs) as pool:
        for i, r in enumerate(pool.imap_unordered(job, jobs, chunksize=1)):
            if "err" in r:
                bad.append(r)
                print("ERREUR", r["seed"], r["step"], r["err"][-300:], flush=True)
                continue
            res.setdefault((r["seed"], r["step"], r["j"]), []).append(r)
            if (i + 1) % 100 == 0:
                print(f"{i + 1}/{len(jobs)} parties, {round(time.time() - t0)} s", flush=True)
    rows = []
    for p in errs:
        rs = res.get((p["seed"], p["step"], p.get("j")), [])
        if not rs:
            continue
        g_, b_ = sum(r["good"] for r in rs) / len(rs), sum(r["bad"] for r in rs) / len(rs)
        rows.append(dict(seed=p["seed"], step=p["step"], typ=p["typ"], what=p.get("card") or "ask:" + p["kind"],
                         src=p.get("src"), regret=p["regret"], n=len(rs), good=round(g_, 4), bad=round(b_, 4),
                         cost=round(g_ - b_, 4)))
    with open(fp.with_name(f"cout-{fp.name}"), "w", encoding="utf-8") as o:
        for r in rows:
            o.write(json.dumps(r, ensure_ascii=False) + "\n")
    out = dict(label=f"coût des erreurs du banc, suite {ver}, {K} tirages", file=fp.name, ver=ver, K=K, regret=thr,
               errors_measured=len(rows), playout_errors=len(bad), seconds=round(time.time() - t0))
    for name, sel in (("tout", lambda r: True), ("play", lambda r: r["typ"] == "play"), ("ask", lambda r: r["typ"] == "ask")):
        rs = [r for r in rows if sel(r)]
        seeds = sorted({r["seed"] for r in rs})
        m, se = cluster([[r["cost"] for r in rs if r["seed"] == s] for s in seeds])
        out[name] = dict(n=len(rs), cost=round(m, 4), se=round(se, 4),
                         good=round(sum(r["good"] for r in rs) / max(1, len(rs)), 4),
                         bad=round(sum(r["bad"] for r in rs) / max(1, len(rs)), 4))
    games_ = [g for g in games]
    B.save(out, games_, engine_md5(HERE.parent))
    print(json.dumps(out, ensure_ascii=False, indent=1))
    for r in sorted(rows, key=lambda r: -r["cost"])[:12]:
        print(f"  {r['what']:<28} {r.get('src') or '':<24} regret {r['regret']:>6} : bon {r['good']:.2f}, "
              f"mauvais {r['bad']:.2f}, coût {r['cost']:+.2f}")


if __name__ == "__main__":
    main(sys.argv[1:])
