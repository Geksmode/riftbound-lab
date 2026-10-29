#!/usr/bin/env python3
"""Test séquentiel (SPRT) d'un réglage de l'IA : A contre B en paires appariées, arrêt dès que l'issue est claire.

    python3 mesure/sprt.py <graine0> <A> <B> [defaut|miroir|akali] [processus] [max_paires]

Règle de l'utilisateur (2026-10-20) : un réglage est gardé s'il apporte +2 points ou plus, démontré. Hypothèses sur le score
moyen de A par paire (exp_search.pair : même donne jouée deux fois, places échangées) : H0 = 50 %, H1 = 52 % ;
alpha = beta = 5 %. LLR en approximation normale (SPRT généralisé, comme fishtest) avec la variance mesurée des paires :
    LLR = n (m1 − m0) (2 x̄ − m0 − m1) / (2 v)
« gardé » si LLR ≥ ln(0,95/0,05), « rejeté » si LLR ≤ ln(0,05/0,95), « on ne sait pas » à max_paires (défaut 1600).
Le LLR est relu toutes les 8 paires à partir de 40, dans l'ordre des graines (imap ordonné) : même graines -> même décision,
quel que soit le nombre de processus. Un réglage gardé se confirme par un second SPRT sur des graines NEUVES.
Durée simulée (écart-type par paire 0,24, 2 410 paires « defaut ») : vrai 47 % -> ≈ 190 paires ; 50 % -> ≈ 670 (p90 1 600) ;
52 % -> ≈ 650 ; 55 % -> ≈ 200. À ≈ 870 parties/h (24 processus, i7-14700HX) : 1 paire ≈ 8 s.
Résultat ajouté à results_search.json (UTF-8, version du moteur, per_pair : compare.py le relit).
Hors de engine/*.py : ne change pas version.engine_md5."""
import json, math, os, sys, time
from multiprocessing import Pool
from pathlib import Path

HERE = Path(__file__).resolve().parent
ENGINE = HERE.parent
sys.path.insert(0, str(ENGINE))
_argv, sys.argv = sys.argv, sys.argv[:1]      # exp.py (importé par exp_search) lit sys.argv
import exp_search as E
sys.argv = _argv

# Hypothèses : par défaut la règle 9 (H0 50 %, H1 52 %). Test de non-infériorité (2026-10-28, réglages « réalisme » :
# démontrer qu'un réglage ne fait pas perdre) : RB_SPRT_H0=0.485 RB_SPRT_H1=0.50 ; « garde » = pas pire que −1,5 point.
M0 = float(os.environ.get("RB_SPRT_H0", "0.50"))
M1 = float(os.environ.get("RB_SPRT_H1", "0.52"))
ALPHA, BETA = 0.05, 0.05
LOW, HIGH = math.log(BETA / (1 - ALPHA)), math.log((1 - BETA) / ALPHA)
MIN_PAIRS, EVERY = 40, 8


def llr(xs, m0=M0, m1=M1):
    n = len(xs)
    m = sum(xs) / n
    v = max(1e-6, sum((x - m) ** 2 for x in xs) / n)
    return n * (m1 - m0) * (2 * m - m0 - m1) / (2 * v)


def decide(xs):
    """'garde', 'rejette' ou None (continuer) pour la suite des scores de paires xs."""
    if len(xs) < MIN_PAIRS or len(xs) % EVERY:
        return None
    L = llr(xs)
    if L >= HIGH:
        return "garde"
    if L <= LOW:
        return "rejette"
    return None


def run(seed0, va, vb, mode="defaut", procs=24, max_pairs=1600):
    t0 = time.time()
    good, errs, xs = [], [], []
    verdict = None
    with Pool(procs) as p:
        for o in p.imap(E.pair, [(seed0 + k, va, vb, mode) for k in range(max_pairs)], chunksize=1):
            if "err" in o:
                errs.append(o)
                print("ERREUR", o["seed"], o["err"], flush=True)
                continue
            good.append(o)
            xs.append(o["score"])
            verdict = decide(xs)
            if len(xs) % 40 == 0 or verdict:
                m = sum(xs) / len(xs)
                print(f"{len(xs)} paires : A {m:.1%}, LLR {llr(xs):+.2f} (bornes {LOW:.2f} / {HIGH:.2f}), "
                      f"{(time.time() - t0) / 60:.0f} min", flush=True)
            if verdict:
                p.terminate()
                break
    n = len(xs)
    m = sum(xs) / n
    se = math.sqrt(sum((x - m) ** 2 for x in xs) / (n - 1) / n)
    res = dict(label=f"SPRT {va} contre {vb} · {mode}", A=va, B=vb, mode=mode, test="sprt",
               h0=M0, h1=M1, alpha=ALPHA, beta=BETA, verdict=verdict or "on ne sait pas", llr=round(llr(xs), 3),
               pairs=n, seed0=seed0, max_pairs=max_pairs, score_A=round(m, 4), se=round(se, 4),
               z=round((m - 0.5) / se, 2) if se > 0 else None, errors=len(errs), err=errs[0]["err"] if errs else None,
               turns_mean=round(sum(sum(o["turns"]) for o in good) / max(1, 2 * n), 1), seconds=round(time.time() - t0),
               per_pair=[[o["seed"], o["score"]] for o in good],
               games=[[o["seed"], o["first"], o["w"], o["legends"][0]] for o in good])
    from version import stamp
    stamp(res)
    return res


if __name__ == "__main__":
    seed0, va, vb = int(sys.argv[1]), sys.argv[2], sys.argv[3]
    mode = sys.argv[4] if len(sys.argv) > 4 else "defaut"
    procs = int(sys.argv[5]) if len(sys.argv) > 5 else 24
    mx = int(sys.argv[6]) if len(sys.argv) > 6 else 1600
    r = run(seed0, va, vb, mode, procs, mx)
    f = ENGINE / "results_search.json"
    done = json.load(open(f, encoding="utf-8")) if f.exists() else []
    done.append(r)
    json.dump(done, open(f, "w", encoding="utf-8"), ensure_ascii=False)
    print(f"{r['label']}: {r['verdict'].upper()} après {r['pairs']} paires (graines {seed0}-{seed0 + r['pairs'] - 1}), "
          f"A {r['score_A']:.1%} ± {r['se']:.3f}, LLR {r['llr']}, erreurs {r['errors']}, {r['seconds']} s")
