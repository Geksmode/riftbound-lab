//! Réglage Texel des poids de `ai::Ev` (`riftbound/engine/texel/` : `features.py`, `gen.py`, `fit.py`).
//!
//! ```text
//!   texel_gen <parties> <graine0> <sh|turn|pol> <sortie.jsonl> [processus]
//!   texel_fit <données.jsonl> [processus] [boot]
//! ```
//! `features` : critères de l'évaluation sous forme linéaire (miroir exact de `ai::evaluate`) ; `gen` : parties IA
//! contre IA, critères de chaque début de tour et gagnant final ; `fit` : régression logistique « position -> gagnant
//! final » (Newton, validation croisée par partie, bootstrap par partie). Mêmes fichiers lus et écrits que Python
//! (`results_texel.json`).
//!
//! Écarts : la sortie de `gen` est écrite dans l'ordre d'arrivée des parties comme `imap_unordered` (ici : l'ordre
//! des graines) ; les lignes sont en JSON compact.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use super::exp_search::{agents, setup};
use super::pool;
use super::py::{self, Dump, PyVal};
use crate::ai::{self, conf, REACTIVE};
use crate::game::{Agent, Counters, Game};
use crate::json::{self, Json};
use crate::jo;
use crate::pyrandom::PyRandom;
use crate::value::is_bf;

/// `features.KEYS`
pub const KEYS: [&str; 17] = [
    "pts", "pts_hi", "bf", "fd", "hold_win", "unit0", "might", "cost", "on_bf", "card0", "card_e", "react", "rune", "leg_emp", "xp",
    "deck_low", "trick",
];

fn ki(k: &str) -> usize {
    KEYS.iter().position(|x| *x == k).unwrap()
}

/// `features.features(g, me)` : (const, critères dans l'ordre de KEYS) de la position vue par `me` ; partie non finie.
pub fn features(g: &mut Game, me: usize) -> (f64, [f64; 17]) {
    assert!(g.winner.is_none());
    let mut f = [0.0f64; 17];
    let mut cst = 0.0f64;
    for (pid, sign) in [(me, 1.0f64), (1 - me, -1.0f64)] {
        let p = g.p[pid].points;
        f[ki("pts")] += sign * p as f64;
        f[ki("pts_hi")] += sign * 0.max(p - (g.victory - 3)) as f64;
        let mut held = 0;
        for b in &g.bfs {
            if b.ctrl == Some(pid) {
                f[ki("bf")] += sign;
                held += 1;
                if let Some(fd) = b.facedown() {
                    if g.o(fd).owner == pid {
                        f[ki("fd")] += sign;
                    }
                }
            }
        }
        if conf().tempo && held > 0 && g.p[pid].points >= g.victory - 1 {
            f[ki("hold_win")] += sign;
        }
        let mut has_unit = false;
        for o in g.board.clone() {
            if g.o(o).ctrl != pid {
                continue;
            }
            if g.spec(o).type_ != "Unit" {
                cst += sign * ai::gear_v(g.cname(o)).unwrap_or(1.0);
                continue;
            }
            has_unit = true;
            // ai.unit_eval
            let m = 0.max(ai::lasting_might(g, o).expect("GameOver")) as f64;
            let sp = g.spec(o);
            if g.cname(o) == "Mech" {
                // dernier remplacement dans unit_eval : il l'emporte
                cst += sign * (0.8 + 0.8 * m);
            } else if g.o(o).token && g.cname(o) == "Reflection" || g.has_kw(o, "Temporary").expect("GameOver") {
                cst += sign * (0.4 + 0.4 * m);
            } else {
                f[ki("unit0")] += sign;
                f[ki("might")] += sign * m;
                f[ki("cost")] += sign * (sp.e as f64 + 1.5 * sp.p as f64);
            }
            if g.cname(o) == "Karthus, Eternal" {
                cst += sign * 2.5;
            }
            if g.o(o).empowered {
                cst += sign * 0.5;
            }
            if is_bf(g.o(o).loc) {
                f[ki("on_bf")] += sign;
            }
        }
        for &c in &g.p[pid].hand {
            // ai.card_value (react_kw = 0)
            if REACTIVE.contains(&g.cname(c)) {
                f[ki("react")] += sign;
            } else {
                f[ki("card0")] += sign;
                f[ki("card_e")] += sign * g.spec(c).e.min(8) as f64;
            }
        }
        if has_unit {
            let nr = g.p[pid].runes.len() as i64;
            let n = g.p[pid].hand.iter().filter(|&&c| ai::is_trick(g.spec(c)) && (g.spec(c).e + g.spec(c).p) as i64 <= nr).count();
            f[ki("trick")] += sign * n as f64;
        }
        f[ki("rune")] += sign * g.p[pid].runes.len() as f64;
        if g.o(g.p[pid].legend).empowered {
            f[ki("leg_emp")] += sign;
        }
        f[ki("xp")] += sign * g.p[pid].xp as f64;
        if g.p[pid].deck.len() < 3 {
            f[ki("deck_low")] -= sign;
        }
    }
    (cst, f)
}

/// `features.linear(const, f, w)`
pub fn linear(cst: f64, f: &[f64; 17], w: &dyn Fn(&str) -> f64) -> f64 {
    cst + py::sum(KEYS.iter().enumerate().map(|(i, k)| w(k) * f[i]))
}

// ------------------------------------------------------------------------------------------------------- gen
fn play(seed: i64, mode: &str) -> Json {
    let s = setup(seed, "defaut");
    let ag: Vec<Box<dyn Agent>> = if mode == "sh" || mode == "turn" {
        agents(seed, &s, mode, mode).into_iter().map(|a| Box::new(a) as Box<dyn Agent>).collect()
    } else {
        vec![Box::new(ai::PolicyAgent::new()), Box::new(ai::PolicyAgent::new())]
    };
    let mut g = Game::new(&[s.decks[0].list(), s.decks[1].list()], ag, Some(s.first), seed as i128, false, crate::spec::VICTORY as i64, Arc::new(Counters::default()));
    for pid in 0..2 {
        let mut a = std::mem::replace(&mut g.agents[pid], Box::new(crate::game_events::FirstChoice));
        a.start(&mut g);
        g.agents[pid] = a;
    }
    let mut rows: Vec<Json> = vec![];
    let mut last: Option<i64> = None;
    loop {
        let Some(d) = g.advance(false) else { break };
        if d.kind == "main" && Some(g.turn_no) != last && g.winner.is_none() {
            last = Some(g.turn_no);
            let (c, fv) = features(&mut g, d.player);
            let mut r = vec![Json::Int(g.turn_no), Json::Int(d.player as i64), Json::Float(py::round_n(c, 6))];
            r.extend(fv.iter().map(|x| Json::Float(py::round_n(*x, 6))));
            rows.push(Json::Arr(r));
        }
        let ph = g.agents[d.player].clone_box();
        let mut a = std::mem::replace(&mut g.agents[d.player], ph);
        let r = a.decide(&mut g, &d);
        g.agents[d.player] = a;
        match r {
            Ok(act) => g.apply(&act),
            Err(_) => break,
        }
    }
    jo! {
        "seed" => seed, "mode" => mode, "legends" => vec![s.decks[0].legend.clone(), s.decks[1].legend.clone()], "first" => s.first,
        "winner" => g.winner, "turns" => g.turn_no, "rows" => Json::Arr(rows),
    }
}

/// `gen.py <parties> <graine0> <sh|turn|pol> <sortie.jsonl> [processus]`
pub fn gen_main(a: &[String]) -> i32 {
    if a.len() < 4 {
        eprintln!("texel_gen <parties> <graine0> <sh|turn|pol> <sortie.jsonl> [processus]");
        return 2;
    }
    let (n, seed0, mode, out): (i64, i64, String, PathBuf) = (a[0].parse().unwrap(), a[1].parse().unwrap(), a[2].clone(), PathBuf::from(&a[3]));
    let procs: usize = a.get(4).map(|x| x.parse().unwrap()).unwrap_or_else(|| (pool::cpu_count().saturating_sub(4)).max(1));
    assert!(["sh", "turn", "pol"].contains(&mode.as_str()));
    let ver = super::version::engine_md5();
    let mut done: Vec<i64> = vec![];
    if out.exists() {
        for line in std::fs::read_to_string(&out).unwrap().lines() {
            let r = json::parse(line).unwrap();
            assert!(r.g("engine").str() == Some(ver.as_str()), "sortie d'une autre version du moteur ({} ≠ {ver})", r.g("engine").str().unwrap_or(""));
            done.push(r.g("seed").int().unwrap());
        }
    }
    let todo: Vec<i64> = (0..n).map(|k| seed0 + k).filter(|s| !done.contains(s)).collect();
    println!("moteur {ver}, {} parties à jouer ({} déjà faites), {procs} processus", todo.len(), done.len());
    let t0 = Instant::now();
    let mut errs = 0;
    let mut fo = std::fs::OpenOptions::new().create(true).append(true).open(&out).expect("sortie");
    let total = todo.len();
    pool::imap(
        procs,
        &todo,
        |&seed| {
            let t = Instant::now();
            pool::attempt(|| {
                let mut r = play(seed, &mode);
                r.set("sec", py::round_n(t.elapsed().as_secs_f64(), 1));
                r
            })
            .map_err(|e| (seed, e))
        },
        |i, r| {
            let i = i + 1;
            match r {
                Err((seed, e)) => {
                    errs += 1;
                    println!("ERREUR {seed} {e}");
                    return true;
                }
                Ok(mut r) => {
                    use std::io::Write;
                    r.set("engine", ver.clone());
                    writeln!(fo, "{}", r.dump()).unwrap();
                    fo.flush().unwrap();
                }
            }
            if i % (total / 50).max(1) == 0 || i == total {
                let el = t0.elapsed().as_secs_f64();
                println!("{i}/{total} parties, {:.1} min, reste ≈ {:.0} min, erreurs {errs}", el / 60.0, el / i as f64 * (total - i) as f64 / 60.0);
            }
            true
        },
    );
    0
}

// ------------------------------------------------------------------------------------------------------- fit
const MIN_GAMES: usize = 300;
const LAMS: [f64; 8] = [0.0, 1.0, 3.0, 10.0, 30.0, 100.0, 300.0, 1000.0];

/// Une position : (étiquette, const, critères).
type Row = (f64, f64, Vec<f64>);
/// Une partie : (graine, positions).
type GameRows = (i64, Vec<Row>);

/// `load(path)` : (parties triées, version du moteur des données)
pub fn load(path: &Path) -> (Vec<GameRows>, String) {
    let mut games: Vec<GameRows> = vec![];
    let mut eng: Vec<String> = vec![];
    for line in std::fs::read_to_string(path).expect("données").lines() {
        let r = json::parse(line).unwrap();
        let e = r.g("engine").str().unwrap_or("").to_string();
        if !eng.contains(&e) {
            eng.push(e);
        }
        let w = r.g("winner").int();
        let mut rows = vec![];
        for x in r.g("rows").arr() {
            let a = x.arr();
            let pl = a[1].int().unwrap();
            let y = if !matches!(w, Some(0) | Some(1)) { 0.5 } else if w == Some(pl) { 1.0 } else { 0.0 };
            rows.push((y, a[2].num().unwrap(), a[3..].iter().map(|v| v.num().unwrap()).collect()));
        }
        games.push((r.g("seed").int().unwrap(), rows));
    }
    assert!(eng.len() == 1, "données de plusieurs versions du moteur : {eng:?}");
    games.sort_by_key(|g| g.0);
    (games, eng.pop().unwrap())
}

fn sig(z: f64) -> f64 {
    if z >= 0.0 {
        1.0 / (1.0 + (-z).exp())
    } else {
        let e = z.exp();
        e / (1.0 + e)
    }
}

/// `solve(A, v)` : A x = v (Gauss avec pivot partiel)
fn solve(a: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    let n = v.len();
    let mut m: Vec<Vec<f64>> = (0..n).map(|i| a[i].iter().copied().chain([v[i]]).collect()).collect();
    for i in 0..n {
        let mut p = i;
        for r in i..n {
            if m[r][i].abs() > m[p][i].abs() {
                p = r;
            }
        }
        m.swap(i, p);
        let piv = m[i][i];
        if piv.abs() < 1e-12 {
            continue;
        }
        for r in i + 1..n {
            let fct = m[r][i] / piv;
            if fct != 0.0 {
                for c in i..=n {
                    let t = fct * m[i][c];
                    m[r][c] -= t;
                }
            }
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let s = m[i][n] - py::sum((i + 1..n).map(|c| m[i][c] * x[c]));
        x[i] = if m[i][i].abs() > 1e-12 { s / m[i][i] } else { 0.0 };
    }
    x
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    py::sum(a.iter().zip(b).map(|(p, q)| p * q))
}

fn objective(rows: &[f64], x_: &[Vec<f64>], off: &[f64], prior: &[f64], pen: &[f64], x: &[f64]) -> f64 {
    let mut s = py::sum((0..x.len()).map(|j| pen[j] * (x[j] - prior[j]).powi(2)));
    for ((y, xi), o) in rows.iter().zip(x_).zip(off) {
        let z = o + dot(xi, x);
        // log(1 + e^z) − y z, stable
        s += (if z > 0.0 { z } else { 0.0 }) + (-z.abs()).exp().ln_1p() - y * z;
    }
    s
}

fn logloss(rows: &[f64], x_: &[Vec<f64>], off: &[f64], x: &[f64]) -> f64 {
    let mut s = 0.0;
    for ((y, xi), o) in rows.iter().zip(x_).zip(off) {
        let p = (1.0f64 - 1e-12).min(1e-12f64.max(sig(o + dot(xi, x))));
        s -= y * p.ln() + (1.0 - y) * (1.0 - p).ln();
    }
    s / (1usize.max(rows.len())) as f64
}

/// `newton(rows, X, off, prior, pen, x0, iters, tol)`
fn newton(rows: &[f64], x_: &[Vec<f64>], off: &[f64], prior: &[f64], pen: &[f64], x0: &[f64], iters: usize, tol: f64) -> Vec<f64> {
    let mut x = x0.to_vec();
    let d = x.len();
    for _ in 0..iters {
        let mut g: Vec<f64> = (0..d).map(|j| 2.0 * pen[j] * (x[j] - prior[j])).collect();
        let mut h = vec![vec![0.0; d]; d];
        for j in 0..d {
            h[j][j] = 2.0 * pen[j];
        }
        for ((y, xi), o) in rows.iter().zip(x_).zip(off) {
            let z = o + dot(xi, &x);
            let p = sig(z);
            let r = p - y;
            let ww = p * (1.0 - p);
            for j in 0..d {
                let a = xi[j];
                if a != 0.0 {
                    g[j] += r * a;
                    let wa = ww * a;
                    for k in j..d {
                        if xi[k] != 0.0 {
                            h[j][k] += wa * xi[k];
                        }
                    }
                }
            }
        }
        for j in 0..d {
            for k in 0..j {
                h[j][k] = h[k][j];
            }
        }
        let step = solve(&h, &g);
        // recherche de pas : un pas de Newton entier peut diverger ; on divise le pas par deux tant que l'objectif monte
        let f0 = objective(rows, x_, off, prior, pen, &x);
        let mut t = 1.0;
        let xn = loop {
            let xn: Vec<f64> = x.iter().zip(&step).map(|(a, s)| a - t * s).collect();
            if objective(rows, x_, off, prior, pen, &xn) <= f0 + 1e-12 || t < 1e-6 {
                break xn;
            }
            t /= 2.0;
        };
        x = xn;
        if step.iter().map(|s| (t * s).abs()).fold(f64::MIN, f64::max) < tol {
            break;
        }
    }
    x
}

struct Flat {
    ys: Vec<f64>,
    cs: Vec<f64>,
    fs: Vec<Vec<f64>>,
}

fn flat(games: &[&GameRows]) -> Flat {
    let mut r = Flat { ys: vec![], cs: vec![], fs: vec![] };
    for (_, rows) in games {
        for (y, c, f) in rows {
            r.ys.push(*y);
            r.cs.push(*c);
            r.fs.push(f.clone());
        }
    }
    r
}

/// poids actuels `ai.EV`
fn ev_w(k: &str) -> f64 {
    conf().ev.get(k).unwrap()
}

type Design = (Vec<f64>, Vec<Vec<f64>>, Vec<f64>);

/// `base_design(games, w)` : étape 1, colonnes (1, eval actuel) -> x = (b, K).
fn base_design(games: &[&GameRows]) -> Design {
    let f = flat(games);
    let x = f.cs.iter().zip(&f.fs).map(|(c, fv)| vec![1.0, c + py::sum(KEYS.iter().zip(fv).map(|(k, v)| ev_w(k) * v))]).collect();
    let n = f.ys.len();
    (f.ys, x, vec![0.0; n])
}

/// `free_design(games, K, free)` : étape 2, colonnes (1, critères libres) ; décalage = K * (const + critères fixés × poids actuels).
fn free_design(games: &[&GameRows], k: f64, free: &[&str]) -> Design {
    let f = flat(games);
    let idx: Vec<usize> = free.iter().map(|x| ki(x)).collect();
    let fix: Vec<(usize, f64)> = KEYS.iter().filter(|x| !free.contains(x)).map(|x| (ki(x), ev_w(x))).collect();
    let x = f.fs.iter().map(|fv| std::iter::once(1.0).chain(idx.iter().map(|&i| fv[i])).collect()).collect();
    let off = f.cs.iter().zip(&f.fs).map(|(c, fv)| k * (c + py::sum(fix.iter().map(|&(i, wk)| wk * fv[i])))).collect();
    (f.ys, x, off)
}

fn fit_free(games: &[&GameRows], k: f64, free: &[&str], lam: f64, sd: &[f64; 17], x0: Option<&[f64]>, iters: usize) -> Vec<f64> {
    let (ys, x, off) = free_design(games, k, free);
    let prior: Vec<f64> = std::iter::once(0.0).chain(free.iter().map(|f| k * ev_w(f))).collect();
    let pen: Vec<f64> = std::iter::once(0.0).chain(free.iter().map(|f| lam * sd[ki(f)].powi(2))).collect();
    newton(&ys, &x, &off, &prior, &pen, x0.unwrap_or(&prior), iters, 1e-9)
}

/// `main()` de `fit.py`
pub fn fit_main(a: &[String]) -> i32 {
    if a.is_empty() {
        eprintln!("texel_fit <données.jsonl> [processus] [boot]");
        return 2;
    }
    let path = PathBuf::from(&a[0]);
    let procs: usize = a.get(1).map(|x| x.parse().unwrap()).unwrap_or(8);
    let nboot: usize = a.get(2).map(|x| x.parse().unwrap()).unwrap_or(200);
    let t0 = Instant::now();
    let (games, eng) = load(&path);
    let train: Vec<&GameRows> = games.iter().filter(|g| g.0 % 5 != 4).collect();
    let test: Vec<&GameRows> = games.iter().filter(|g| g.0 % 5 == 4).collect();
    let npos: usize = games.iter().map(|g| g.1.len()).sum();
    println!("{} parties, {} positions, moteur {} ; entraînement {}, contrôle {}", games.len(), npos, eng, train.len(), test.len());

    // critères : parties où ils sont non nuls, écart-type (entraînement)
    let fl = flat(&train);
    let nz: Vec<usize> = (0..KEYS.len()).map(|i| train.iter().filter(|(_, rows)| rows.iter().any(|(_, _, f)| f[i] != 0.0)).count()).collect();
    let mut sd = [1.0f64; 17];
    for i in 0..KEYS.len() {
        let col: Vec<f64> = fl.fs.iter().map(|f| f[i]).collect();
        let m = py::sum(col.iter().copied()) / col.len() as f64;
        let s = (py::sum(col.iter().map(|v| (v - m).powi(2))) / col.len() as f64).sqrt();
        sd[i] = if s != 0.0 { s } else { 1.0 };
    }
    let free: Vec<&str> = KEYS.iter().copied().filter(|k| nz[ki(k)] >= MIN_GAMES).collect();
    let nz_d = format!("{{{}}}", KEYS.iter().enumerate().map(|(i, k)| format!("'{k}': {}", nz[i])).collect::<Vec<_>>().join(", "));
    let free_l = format!("[{}]", free.iter().map(|k| format!("'{k}'")).collect::<Vec<_>>().join(", "));
    println!("parties où le critère est non nul : {nz_d}");
    println!("critères libres : {free_l}");

    // 1. référence : poids actuels, K et b ajustés
    let (ys, x, off) = base_design(&train);
    let bk = newton(&ys, &x, &off, &[0.0, 0.0], &[0.0, 0.0], &[0.0, 0.01], 30, 1e-9);
    let (b0, k) = (bk[0], bk[1]);
    assert!(0.0 < k && k < 10.0, "échelle K aberrante ({k}) : ajustement de référence divergé");
    let (yt, xt, ot) = base_design(&test);
    let ll_base = logloss(&yt, &xt, &ot, &[b0, k]);
    let pbar = py::sum(ys.iter().copied()) / ys.len() as f64;
    let ll_const = logloss(&yt, &vec![vec![1.0]; yt.len()], &vec![0.0; yt.len()], &[((1e-9f64).max(pbar) / (1e-9f64).max(1.0 - pbar)).ln()]);
    println!("référence : K = {k:.4}, b = {b0:.3}, log-loss contrôle {ll_base:.5} (sans critère {ll_const:.5})");

    // 2. lam par validation croisée par partie
    let jobs: Vec<(f64, usize)> = LAMS.iter().flat_map(|&l| (0..4).map(move |f| (l, f))).collect();
    let cv = pool::map(procs, &jobs, |&(lam, fold)| {
        let tr: Vec<&GameRows> = train.iter().copied().filter(|g| g.0 % 5 != fold as i64).collect();
        let te: Vec<&GameRows> = train.iter().copied().filter(|g| g.0 % 5 == fold as i64).collect();
        let x = fit_free(&tr, k, &free, lam, &sd, None, 30);
        let (ys, xx, off) = free_design(&te, k, &free);
        (lam, fold, logloss(&ys, &xx, &off, &x), ys.len())
    });
    let mut cvl: Vec<(f64, f64, usize)> = vec![];
    for (lam, _, ll, n) in cv {
        match cvl.iter_mut().find(|e| e.0 == lam) {
            Some(e) => {
                e.1 += ll * n as f64;
                e.2 += n;
            }
            None => cvl.push((lam, ll * n as f64, n)),
        }
    }
    let cvl: Vec<(f64, f64)> = cvl.into_iter().map(|(l, s, n)| (l, s / n as f64)).collect();
    let mut lam = cvl[0];
    for e in &cvl {
        if e.1 < lam.1 {
            lam = *e;
        }
    }
    let lam = lam.0;
    println!(
        "validation croisée (log-loss par lam) : {{{}}} -> lam {}",
        cvl.iter().map(|(l, v)| format!("{}: {}", py::repr_f64(*l), py::repr_f64(py::round_n(*v, 5)))).collect::<Vec<_>>().join(", "),
        py::repr_f64(lam)
    );

    let x = fit_free(&train, k, &free, lam, &sd, None, 30);
    let (ys2, x2, o2) = free_design(&test, k, &free);
    let ll_fit = logloss(&ys2, &x2, &o2, &x);
    let mut w_new: Vec<f64> = KEYS.iter().map(|k| ev_w(k)).collect();
    for (j, kk) in free.iter().enumerate() {
        w_new[ki(kk)] = x[1 + j] / k;
    }

    // écart de log-loss sur le contrôle, avec écart-type par partie (apparié)
    let mut diffs: Vec<f64> = vec![];
    for g in &test {
        let one = [*g];
        let (yy, xx, oo) = base_design(&one);
        let aa = logloss(&yy, &xx, &oo, &[b0, k]) * yy.len() as f64;
        let (yy, xx, oo) = free_design(&one, k, &free);
        let bb = logloss(&yy, &xx, &oo, &x) * yy.len() as f64;
        diffs.push(bb - aa);
    }
    let md = py::sum(diffs.iter().copied()) / diffs.len() as f64;
    let se_d = (py::sum(diffs.iter().map(|d| (d - md).powi(2))) / (diffs.len() - 1) as f64 / diffs.len() as f64).sqrt();
    println!("contrôle : log-loss {ll_base:.5} -> {ll_fit:.5} (écart par partie {md:+.4} ± {se_d:.4})");

    // 4. bootstrap par partie
    let reps: Vec<usize> = (0..nboot).collect();
    let boots = pool::map(procs, &reps, |&rep| {
        let mut r = PyRandom::new(7000 + rep as i128);
        let smp: Vec<&GameRows> = (0..train.len()).map(|_| train[r.randrange(train.len() as i64) as usize]).collect();
        fit_free(&smp, k, &free, lam, &sd, Some(&x), 8)
    });
    let mut se = [0.0f64; 17];
    for (j, kk) in free.iter().enumerate() {
        let vals: Vec<f64> = boots.iter().map(|bt| bt[1 + j] / k).collect();
        let m = py::sum(vals.iter().copied()) / vals.len() as f64;
        se[ki(kk)] = (py::sum(vals.iter().map(|v| (v - m).powi(2))) / (vals.len() - 1) as f64).sqrt();
    }
    println!("\npoids : actuel -> appris (± écart-type bootstrap)");
    for kk in KEYS {
        let i = ki(kk);
        if free.contains(&kk) {
            let z = if se[i] != 0.0 { (w_new[i] - ev_w(kk)) / se[i] } else { 0.0 };
            println!("  {:<9} {:>7.3} -> {:>7.3} ± {:.3}  (écart {:+.1} é.-t.)", kk, ev_w(kk), w_new[i], se[i], z);
        } else {
            println!("  {:<9} {:>7.3}    gardé (critère trop rare)", kk, ev_w(kk));
        }
    }
    let ver = format!("sh@{}", free.iter().map(|k| format!("{k}={}", fmt_g(w_new[ki(k)], 4))).collect::<Vec<_>>().join(","));
    println!("\nversion pour exp_search.py : {ver}");

    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let dict = |f: &dyn Fn(usize) -> PyVal| PyVal::Dict(KEYS.iter().enumerate().map(|(i, k)| (k.to_string(), f(i))).collect());
    let mut res = crate::pydict! {
        "label" => format!("Texel {name}"), "data" => name.clone(), "engine_data" => eng, "games" => games.len() as i64,
        "positions" => npos as i64, "train" => train.len() as i64, "test" => test.len() as i64, "K" => k, "b" => b0,
        "lam" => lam, "cv" => PyVal::Dict(cvl.iter().map(|(l, v)| (py::repr_f64(*l), PyVal::Float(*v))).collect()),
        "free" => free.iter().map(|k| k.to_string()).collect::<Vec<_>>(), "nonzero" => dict(&|i| PyVal::Int(nz[i] as i64)),
        "ev_old" => dict(&|i| PyVal::Float(ev_w(KEYS[i]))), "ev_new" => dict(&|i| PyVal::Float(w_new[i])),
        "se" => PyVal::Dict(free.iter().map(|k| (k.to_string(), PyVal::Float(se[ki(k)]))).collect()), "boot" => nboot as i64,
        "ll_test_base" => ll_base, "ll_test_fit" => ll_fit, "ll_test_const" => ll_const, "ll_diff" => md, "ll_diff_se" => se_d,
        "version" => ver.clone(), "seconds" => py::round0(t0.elapsed().as_secs_f64()),
    };
    super::version::stamp(&mut res);
    let f = super::exp_search::here().join("texel").join("results_texel.json");
    let mut done = if f.exists() { py::load_file(&f).unwrap() } else { PyVal::List(vec![]) };
    if let PyVal::List(v) = &mut done {
        v.push(res.clone());
    }
    py::dump_file(&f, &done, Dump { indent: Some(1), ensure_ascii: false });
    println!("{} s, résultat ajouté à {}", res.get("seconds").unwrap().py_str(), f.file_name().unwrap().to_string_lossy());
    0
}

/// `format(x, ".{p}g")` de Python
pub fn fmt_g(x: f64, p: usize) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let e = format!("{:.*e}", p - 1, x);
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let strip = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if exp < -4 || exp >= p as i32 {
        format!("{}e{}{:02}", strip(mant), if exp < 0 { '-' } else { '+' }, exp.abs())
    } else {
        strip(&format!("{:.*}", (p as i32 - 1 - exp).max(0) as usize, x))
    }
}
