#!/usr/bin/env python3
"""Réglage Texel des poids de ai.EV (2026-10-20) : régression logistique « position -> gagnant final ».

    python3 texel/fit.py <données.jsonl> [processus] [boot]

Modèle : P(le joueur actif gagne) = sigmoïde(b + K * const + somme(theta[k] * f[k])), f = features.KEYS (moi − adversaire),
théta = K * w. Étapes :
  1. poids actuels (w = ai.EV) : on n'ajuste que K et b -> référence ;
  2. théta ajusté, K gardé (mêmes unités que EV), rappel vers K * EV : lam * somme((s_k * (theta_k − K * EV_k))²), s_k =
     écart-type du critère (ridge sur critères centrés-réduits). lam choisi par validation croisée à 5 plis PAR PARTIE ;
  3. contrôle : log-loss sur 20 % des parties (graine % 5 == 4) jamais vues aux étapes 1 et 2 ;
  4. intervalles : bootstrap PAR PARTIE (les positions d'une même partie ne sont pas indépendantes).
Critères quasi absents des données (moins de MIN_GAMES parties où ils sont non nuls) : poids gardé à sa valeur actuelle.
Python pur (aucune dépendance), Newton ; les plis et le bootstrap tournent en parallèle.
Sortie : texel/results_texel.json (une entrée par réglage, version du moteur des données) et la version à passer à
exp_search.py (« sh@pts=…,bf=… »). Égalité nul / pas de gagnant : étiquette 0,5."""
import json, math, random, sys, time
from multiprocessing import Pool
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
import features as F
import ai

KEYS = F.KEYS
MIN_GAMES = 300
LAMS = (0.0, 1.0, 3.0, 10.0, 30.0, 100.0, 300.0, 1000.0)
D = None            # données partagées par les processus (fork) ou chargées par init


def load(path):
    games = []
    eng = set()
    for line in open(path, encoding="utf-8"):
        r = json.loads(line)
        eng.add(r["engine"])
        rows = []
        for t, pl, c, *f in r["rows"]:
            y = 0.5 if r["winner"] not in (0, 1) else (1.0 if r["winner"] == pl else 0.0)
            rows.append((y, c, f))
        games.append((r["seed"], rows))
    assert len(eng) == 1, f"données de plusieurs versions du moteur : {sorted(eng)}"
    games.sort()
    return games, eng.pop()


def sig(z):
    if z >= 0:
        return 1.0 / (1.0 + math.exp(-z))
    e = math.exp(z)
    return e / (1.0 + e)


def solve(A, v):
    """A x = v (Gauss avec pivot partiel), A symétrique définie positive en pratique."""
    n = len(v)
    M = [A[i][:] + [v[i]] for i in range(n)]
    for i in range(n):
        p = max(range(i, n), key=lambda r: abs(M[r][i]))
        M[i], M[p] = M[p], M[i]
        piv = M[i][i]
        if abs(piv) < 1e-12:
            continue
        for r in range(i + 1, n):
            fct = M[r][i] / piv
            if fct:
                for c in range(i, n + 1):
                    M[r][c] -= fct * M[i][c]
    x = [0.0] * n
    for i in range(n - 1, -1, -1):
        s = M[i][n] - sum(M[i][c] * x[c] for c in range(i + 1, n))
        x[i] = s / M[i][i] if abs(M[i][i]) > 1e-12 else 0.0
    return x


def newton(rows, X, off, prior, pen, x0, iters=30, tol=1e-9):
    """Minimise somme(perte logistique de sigmoïde(off_i + X_i·x)) + somme(pen_j (x_j − prior_j)²) ; rows = étiquettes.
    X : liste de vecteurs (colonnes libres), off : décalage fixe par position. Retourne x."""
    x = list(x0)
    d = len(x)
    for _ in range(iters):
        g = [2 * pen[j] * (x[j] - prior[j]) for j in range(d)]
        H = [[0.0] * d for _ in range(d)]
        for j in range(d):
            H[j][j] = 2 * pen[j]
        for y, xi, o in zip(rows, X, off):
            z = o + sum(a * b for a, b in zip(xi, x))
            p = sig(z)
            r = p - y
            ww = p * (1 - p)
            for j in range(d):
                a = xi[j]
                if a:
                    g[j] += r * a
                    wa = ww * a
                    Hj = H[j]
                    for k in range(j, d):
                        if xi[k]:
                            Hj[k] += wa * xi[k]
        for j in range(d):
            for k in range(j):
                H[j][k] = H[k][j]
        step = solve(H, g)
        # recherche de pas (2026-10-20) : un pas de Newton entier peut diverger (K -> 1e11 sur 9 493 parties) ;
        # on divise le pas par deux tant que l'objectif monte
        f0 = objective(rows, X, off, prior, pen, x)
        t = 1.0
        while True:
            xn = [a - t * s for a, s in zip(x, step)]
            if objective(rows, X, off, prior, pen, xn) <= f0 + 1e-12 or t < 1e-6:
                break
            t /= 2
        x = xn
        if max(abs(t * s) for s in step) < tol:
            break
    return x


def objective(rows, X, off, prior, pen, x):
    s = sum(pen[j] * (x[j] - prior[j]) ** 2 for j in range(len(x)))
    for y, xi, o in zip(rows, X, off):
        z = o + sum(a * b for a, b in zip(xi, x))
        # log(1 + e^z) − y z, stable
        s += (z if z > 0 else 0.0) + math.log1p(math.exp(-abs(z))) - y * z
    return s


def logloss(rows, X, off, x):
    s = 0.0
    for y, xi, o in zip(rows, X, off):
        p = min(1 - 1e-12, max(1e-12, sig(o + sum(a * b for a, b in zip(xi, x)))))
        s -= y * math.log(p) + (1 - y) * math.log(1 - p)
    return s / max(1, len(rows))


# ---------------------------------------------------------------- jeux de données
def flat(games):
    ys, cs, fs = [], [], []
    for _, rows in games:
        for y, c, f in rows:
            ys.append(y)
            cs.append(c)
            fs.append(f)
    return ys, cs, fs


def base_design(games, w):
    """Étape 1 : colonnes (1, eval actuel) -> x = (b, K)."""
    ys, cs, fs = flat(games)
    X = [[1.0, c + sum(w[k] * v for k, v in zip(KEYS, f))] for c, f in zip(cs, fs)]
    return ys, X, [0.0] * len(ys)


def free_design(games, K, free, fixed_w):
    """Étape 2 : colonnes (1, critères libres) ; décalage = K * (const + critères fixés × poids actuels)."""
    ys, cs, fs = flat(games)
    idx = [KEYS.index(k) for k in free]
    fix = [(KEYS.index(k), fixed_w[k]) for k in KEYS if k not in free]
    X = [[1.0] + [f[i] for i in idx] for f in fs]
    off = [K * (c + sum(wk * f[i] for i, wk in fix)) for c, f in zip(cs, fs)]
    return ys, X, off


def fit_free(games, K, free, lam, sd, x0=None, iters=30):
    ys, X, off = free_design(games, K, free, ai.EV)
    prior = [0.0] + [K * ai.EV[k] for k in free]
    pen = [0.0] + [lam * sd[k] ** 2 for k in free]
    return newton(ys, X, off, prior, pen, x0 or prior, iters=iters)


def _cv(arg):
    lam, fold, K, free, sd = arg
    tr = [gm for gm in D if gm[0] % 5 != fold]
    te = [gm for gm in D if gm[0] % 5 == fold]
    x = fit_free(tr, K, free, lam, sd)
    ys, X, off = free_design(te, K, free, ai.EV)
    return lam, fold, logloss(ys, X, off, x), len(ys)


def _boot(arg):
    rep, K, free, lam, sd, x0 = arg
    r = random.Random(7000 + rep)
    smp = [D[r.randrange(len(D))] for _ in range(len(D))]
    return fit_free(smp, K, free, lam, sd, x0=x0, iters=8)


def _init(path):
    global D
    games, _ = load(path)
    D = [gm for gm in games if gm[0] % 5 != 4]          # entraînement seulement


def main():
    path = sys.argv[1]
    procs = int(sys.argv[2]) if len(sys.argv) > 2 else 8
    nboot = int(sys.argv[3]) if len(sys.argv) > 3 else 200
    t0 = time.time()
    games, eng = load(path)
    train = [gm for gm in games if gm[0] % 5 != 4]
    test = [gm for gm in games if gm[0] % 5 == 4]
    npos = sum(len(r) for _, r in games)
    print(f"{len(games)} parties, {npos} positions, moteur {eng} ; entraînement {len(train)}, contrôle {len(test)}",
          flush=True)

    # critères : parties où ils sont non nuls, écart-type (entraînement)
    _, _, fs = flat(train)
    nz = {k: sum(1 for _, rows in train if any(f[i] for _, _, f in rows)) for i, k in enumerate(KEYS)}
    sd = {}
    for i, k in enumerate(KEYS):
        col = [f[i] for f in fs]
        m = sum(col) / len(col)
        sd[k] = math.sqrt(sum((v - m) ** 2 for v in col) / len(col)) or 1.0
    free = [k for k in KEYS if nz[k] >= MIN_GAMES]
    print("parties où le critère est non nul :", nz, flush=True)
    print("critères libres :", free, flush=True)

    # 1. référence : poids actuels, K et b ajustés
    ys, X, off = base_design(train, ai.EV)
    b0, K = newton(ys, X, off, [0.0, 0.0], [0.0, 0.0], [0.0, 0.01])
    assert 0 < K < 10, f"échelle K aberrante ({K}) : ajustement de référence divergé"
    yt, Xt, ot = base_design(test, ai.EV)
    ll_base = logloss(yt, Xt, ot, [b0, K])
    ll_const = logloss(yt, [[1.0]] * len(yt), [0.0] * len(yt),
                       [math.log(max(1e-9, sum(ys) / len(ys)) / max(1e-9, 1 - sum(ys) / len(ys)))])
    print(f"référence : K = {K:.4f}, b = {b0:.3f}, log-loss contrôle {ll_base:.5f} (sans critère {ll_const:.5f})",
          flush=True)

    # 2. lam par validation croisée par partie
    with Pool(procs, initializer=_init, initargs=(path,)) as p:
        cv = p.map(_cv, [(lam, fold, K, free, sd) for lam in LAMS for fold in range(4)], chunksize=1)
    cvl = {}
    for lam, fold, ll, n in cv:
        a = cvl.setdefault(lam, [0.0, 0])
        a[0] += ll * n
        a[1] += n
    cvl = {lam: a[0] / a[1] for lam, a in cvl.items()}
    lam = min(cvl, key=cvl.get)
    print("validation croisée (log-loss par lam) :", {k: round(v, 5) for k, v in cvl.items()}, "-> lam", lam, flush=True)

    x = fit_free(train, K, free, lam, sd)
    ys2, X2, o2 = free_design(test, K, free, ai.EV)
    ll_fit = logloss(ys2, X2, o2, x)
    w_new = dict(ai.EV)
    for j, k in enumerate(free):
        w_new[k] = x[1 + j] / K

    # écart de log-loss sur le contrôle, avec écart-type par partie (apparié)
    diffs = []
    for _, rows in test:
        yy, XX, oo = base_design([(0, rows)], ai.EV)
        a = logloss(yy, XX, oo, [b0, K]) * len(yy)
        yy, XX, oo = free_design([(0, rows)], K, free, ai.EV)
        bb = logloss(yy, XX, oo, x) * len(yy)
        diffs.append(bb - a)
    n_t = sum(len(r) for _, r in test)
    md = sum(diffs) / len(diffs)
    se_d = math.sqrt(sum((d - md) ** 2 for d in diffs) / (len(diffs) - 1) / len(diffs))
    print(f"contrôle : log-loss {ll_base:.5f} -> {ll_fit:.5f} (écart par partie {md:+.4f} ± {se_d:.4f})", flush=True)

    # 4. bootstrap par partie
    with Pool(procs, initializer=_init, initargs=(path,)) as p:
        boots = p.map(_boot, [(r, K, free, lam, sd, x) for r in range(nboot)], chunksize=1)
    se = {}
    for j, k in enumerate(free):
        vals = [bt[1 + j] / K for bt in boots]
        m = sum(vals) / len(vals)
        se[k] = math.sqrt(sum((v - m) ** 2 for v in vals) / (len(vals) - 1))

    print("\npoids : actuel -> appris (± écart-type bootstrap)")
    for k in KEYS:
        if k in free:
            z = (w_new[k] - ai.EV[k]) / se[k] if se[k] else 0.0
            print(f"  {k:9} {ai.EV[k]:7.3f} -> {w_new[k]:7.3f} ± {se[k]:.3f}  (écart {z:+.1f} é.-t.)")
        else:
            print(f"  {k:9} {ai.EV[k]:7.3f}    gardé (critère trop rare)")
    ver = "sh@" + ",".join(f"{k}={w_new[k]:.4g}" for k in free)
    print("\nversion pour exp_search.py :", ver)

    from version import stamp
    res = dict(label=f"Texel {Path(path).name}", data=str(Path(path).name), engine_data=eng, games=len(games),
               positions=npos, train=len(train), test=len(test), K=K, b=b0, lam=lam, cv=cvl, free=free, nonzero=nz,
               ev_old={k: ai.EV[k] for k in KEYS}, ev_new={k: w_new[k] for k in KEYS}, se=se, boot=nboot,
               ll_test_base=ll_base, ll_test_fit=ll_fit, ll_test_const=ll_const, ll_diff=md, ll_diff_se=se_d,
               version=ver, seconds=round(time.time() - t0))
    stamp(res)
    f = HERE / "results_texel.json"
    done = json.load(open(f, encoding="utf-8")) if f.exists() else []
    done.append(res)
    json.dump(done, open(f, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print(f"{res['seconds']} s, résultat ajouté à {f.name}")


if __name__ == "__main__":
    main()
