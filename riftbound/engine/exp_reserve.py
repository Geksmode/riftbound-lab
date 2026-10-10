#!/usr/bin/env python3
"""Réserve ouverte de l'IA : garde-t-elle des runes pour réagir pendant le tour adverse ? (retour de l'utilisateur,
2026-10-10 : « les cartes en main sont des ressources, laisser des runes ouvertes pour réagir »).

    python3 exp_reserve.py <parties> <graine0> <version> [defaut|akali|miroir] [processus]

Version : comme exp_search.py (« sh », « sh@res=3,pol_keep=1,pol_react=1 »…), la même des deux côtés.
Mesure descriptive (pas un A contre B) : à chaque fin de tour du joueur actif, on regarde sa main et ses runes prêtes
(ai.reserve : carte [Reaction] / [Action] / [Ambush] payable avec les runes prêtes) :
  avec_carte : il finit son tour avec au moins une telle carte en main ;
  a_sec      : … mais aucune n'est payable (runes toutes engagées) ;
  ouverte    : au moins une est payable (réserve ouverte) ;
  utilisee   : réserve ouverte ET il joue une carte pendant le tour adverse qui suit ;
  hors_tour  : cartes jouées pendant le tour adverse, par partie et par joueur ;
  reac_main  : sorts [Reaction] joués dans son propre tour principal (sans fenêtre de réaction), par partie et par joueur.
Taux par partie puis moyenne ± écart-type sur les parties (les tours d'une même partie ne sont pas indépendants).
Résultats ajoutés à results_reserve.json (stamp())."""
from version import stamp
import json, sys, time, math, traceback
from multiprocessing import Pool
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import exp_search as XS
import plans as P
import ai
from game import Game, Obj, Item

_POL = ai.PolicyAgent()
KEYS = ("avec_carte", "a_sec", "ouverte", "utilisee")


def one(arg):
    seed, v, mode = arg
    try:
        Obj._n = 0
        Item._n = 0
        decks, (p0, p1), f = XS.setup(seed, mode)
        sm, n, ex, h, cfg = XS.version(v)
        ag = [P.PlanAgent(seed + 500000 * i, plan=pl, opp_plan=op, samples=n, search=sm, sh_extra=ex, horizon=h, cfg=cfg)
              for i, (pl, op) in enumerate(((p0, p1), (p1, p0)))]
        g = Game(decks, ag, seed=seed, first=f)
        cnt = {k: 0 for k in KEYS + ("tours", "hors_tour", "reac_main")}
        tp, pending = None, None           # pending : joueur qui a fini son tour avec une réserve ouverte
        while True:
            d = g.advance()
            if d is None:
                break
            if g.tp != tp:
                if tp is not None:
                    cnt["tours"] += 1
                    has = any(ai.timed_card(c) for c in g.p[tp].hand)
                    r = ai.reserve(g, tp)
                    cnt["avec_carte"] += has
                    cnt["a_sec"] += has and not r
                    cnt["ouverte"] += bool(r)
                    pending = tp if r else None
                tp = g.tp
            a = ag[d.player].decide(g, d)
            if a[0] == "play":
                c = _POL.card_of(g, a)
                if d.player != g.tp:
                    cnt["hors_tour"] += 1
                    if pending == d.player:
                        cnt["utilisee"] += 1
                        pending = None
                elif d.kind == "main" and c is not None and "Reaction" in c.spec["keywords"]:
                    cnt["reac_main"] += 1
            g.apply(a)
        return dict(seed=seed, legends=[x["legend"] for x in decks], turns=g.turn_no, **cnt)
    except Exception:
        return dict(seed=seed, err=traceback.format_exc()[-800:])


def ms(xs):
    m = sum(xs) / max(1, len(xs))
    sd = math.sqrt(sum((x - m) ** 2 for x in xs) / (len(xs) - 1)) if len(xs) > 1 else 0.0
    return round(m, 4), round(sd / math.sqrt(max(1, len(xs))), 4)


def run(n, seed0, v, mode, procs=4):
    t0 = time.time()
    with Pool(procs) as p:
        out = p.map(one, [(seed0 + k, v, mode) for k in range(n)], chunksize=1)
    good = [o for o in out if "err" not in o]
    errs = [o for o in out if "err" in o]
    rate = lambda o, a, b: o[a] / o[b] if o[b] else None
    res = dict(label=f"réserve {v} · {mode}", version=v, mode=mode, games=len(good), seed0=seed0)
    for k, a, b in (("avec_carte/tours", "avec_carte", "tours"), ("a_sec/avec_carte", "a_sec", "avec_carte"),
                    ("ouverte/tours", "ouverte", "tours"), ("utilisee/ouverte", "utilisee", "ouverte")):
        xs = [r for r in (rate(o, a, b) for o in good) if r is not None]
        res[k] = ms(xs) + (len(xs),)
    for k in ("hors_tour", "reac_main"):
        res[k + "/joueur"] = ms([o[k] / 2 for o in good])
    res.update(errors=len(errs), err=errs[0]["err"] if errs else None, seconds=round(time.time() - t0),
               per_game=[[o["seed"]] + [o[k] for k in KEYS + ("tours", "hors_tour", "reac_main")] for o in good])
    stamp(res)
    return res


if __name__ == "__main__":
    n, seed0, v = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
    mode = sys.argv[4] if len(sys.argv) > 4 else "defaut"
    procs = int(sys.argv[5]) if len(sys.argv) > 5 else 4
    r = run(n, seed0, v, mode, procs)
    f = HERE / "results_reserve.json"
    done = json.load(open(f)) if f.exists() else []
    done.append(r)
    json.dump(done, open(f, "w"), ensure_ascii=False)
    print(f"{r['label']} : {r['games']} parties (graines {seed0}-{seed0 + n - 1}), {r['seconds']} s, erreurs {r['errors']}")
    for k in ("avec_carte/tours", "a_sec/avec_carte", "ouverte/tours", "utilisee/ouverte"):
        m, se, nn = r[k]
        print(f"  {k:18} {m:6.1%} ± {se:.1%}  ({nn} parties)")
    for k in ("hors_tour/joueur", "reac_main/joueur"):
        m, se = r[k]
        print(f"  {k:18} {m:6.2f} ± {se:.2f} par partie")
    if r["err"]:
        print(r["err"])
