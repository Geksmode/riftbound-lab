//! `mesure/sprt.py` : test séquentiel (SPRT) d'un réglage de l'IA, A contre B en paires appariées, arrêt dès que
//! l'issue est claire.
//!
//! ```text
//!   sprt <graine0> <A> <B> [defaut|miroir|akali] [processus] [max_paires]
//! ```
//! Règle de l'utilisateur (2026-10-20, règle 9 de `CLAUDE.md`) : un réglage est gardé s'il apporte +2 points ou plus,
//! démontré. Hypothèses sur le score moyen de A par paire (`exp_search::pair`) : H0 = 50 %, H1 = 52 % ; alpha = beta =
//! 5 %. LLR en approximation normale (SPRT généralisé, comme fishtest) avec la variance mesurée des paires :
//!     LLR = n (m1 − m0) (2 x̄ − m0 − m1) / (2 v)
//! « gardé » si LLR ≥ ln(0,95/0,05), « rejeté » si LLR ≤ ln(0,05/0,95), « on ne sait pas » à max_paires (défaut 1600).
//! Le LLR est relu toutes les 8 paires à partir de 40, dans l'ordre des graines : mêmes graines -> même décision, quel
//! que soit le nombre de fils. Test de non-infériorité (règle 10) : `RB_SPRT_H0=0.485 RB_SPRT_H1=0.50`. Un réglage
//! gardé se confirme par un second SPRT sur des graines NEUVES. Résultat ajouté à results_search.json (version du
//! moteur Rust, per_pair : `compare` le relit). Mêmes seuils et même logique que le Python.
use std::time::Instant;

use super::exp_search::{append_result, here, pair, pair_rows, Pair};
use super::pool;
use super::py::{self, PyVal};

pub const ALPHA: f64 = 0.05;
pub const BETA: f64 = 0.05;
pub const MIN_PAIRS: usize = 40;
pub const EVERY: usize = 8;

/// `M0`, `M1` : hypothèses (RB_SPRT_H0, RB_SPRT_H1 ; défaut 0,50 et 0,52).
pub fn m0_m1() -> (f64, f64) {
    let f = |k: &str, d: &str| -> f64 {
        let s = std::env::var(k).unwrap_or_else(|_| d.to_string());
        s.trim().parse().unwrap_or_else(|_| panic!("ValueError: could not convert string to float: {s:?}"))
    };
    (f("RB_SPRT_H0", "0.50"), f("RB_SPRT_H1", "0.52"))
}

/// `LOW`, `HIGH`
pub fn bounds() -> (f64, f64) {
    ((BETA / (1.0 - ALPHA)).ln(), ((1.0 - BETA) / ALPHA).ln())
}

/// `llr(xs, m0, m1)`
pub fn llr(xs: &[f64], m0: f64, m1: f64) -> f64 {
    let n = xs.len() as f64;
    let m = py::sum(xs.iter().copied()) / n;
    let v = (py::sum(xs.iter().map(|x| (x - m).powi(2))) / n).max(1e-6);
    n * (m1 - m0) * (2.0 * m - m0 - m1) / (2.0 * v)
}

/// `decide(xs)` : Some("garde"), Some("rejette") ou None (continuer).
pub fn decide(xs: &[f64], m0: f64, m1: f64) -> Option<&'static str> {
    if xs.len() < MIN_PAIRS || xs.len() % EVERY != 0 {
        return None;
    }
    let l = llr(xs, m0, m1);
    let (low, high) = bounds();
    if l >= high {
        Some("garde")
    } else if l <= low {
        Some("rejette")
    } else {
        None
    }
}

/// `run(seed0, va, vb, mode, procs, max_pairs)`
pub fn run(seed0: i64, va: &str, vb: &str, mode: &str, procs: usize, max_pairs: i64) -> PyVal {
    let (m0, m1) = m0_m1();
    let (low, high) = bounds();
    let t0 = Instant::now();
    let mut good: Vec<Pair> = vec![];
    let mut errs: Vec<String> = vec![];
    let mut xs: Vec<f64> = vec![];
    let mut verdict: Option<&'static str> = None;
    let seeds: Vec<i64> = (0..max_pairs).map(|k| seed0 + k).collect();
    pool::imap(
        procs,
        &seeds,
        |&s| (s, pair(s, va, vb, mode)),
        |_, (s, o)| {
            match o {
                Err(e) => {
                    println!("ERREUR {s} {e}");
                    errs.push(e);
                    return true;
                }
                Ok(o) => {
                    xs.push(o.score);
                    good.push(o);
                }
            }
            verdict = decide(&xs, m0, m1);
            if xs.len() % 40 == 0 || verdict.is_some() {
                let m = py::sum(xs.iter().copied()) / xs.len() as f64;
                println!(
                    "{} paires : A {}, LLR {:+.2} (bornes {low:.2} / {high:.2}), {:.0} min",
                    xs.len(),
                    py::pct(m, 1),
                    llr(&xs, m0, m1),
                    t0.elapsed().as_secs_f64() / 60.0
                );
            }
            verdict.is_none()
        },
    );
    let n = xs.len() as f64;
    let m = py::sum(xs.iter().copied()) / n;
    let se = (py::sum(xs.iter().map(|x| (x - m).powi(2))) / (n - 1.0) / n).sqrt();
    let (pp, games) = pair_rows(&good);
    let mut res = crate::pydict! {
        "label" => format!("SPRT {va} contre {vb} · {mode}"), "A" => va, "B" => vb, "mode" => mode, "test" => "sprt",
        "h0" => m0, "h1" => m1, "alpha" => ALPHA, "beta" => BETA, "verdict" => verdict.unwrap_or("on ne sait pas"),
        "llr" => py::round_n(llr(&xs, m0, m1), 3), "pairs" => xs.len() as i64, "seed0" => seed0, "max_pairs" => max_pairs,
        "score_A" => py::round_n(m, 4), "se" => py::round_n(se, 4),
        "z" => if se > 0.0 { Some(py::round_n((m - 0.5) / se, 2)) } else { None },
        "errors" => errs.len() as i64, "err" => errs.first().cloned(),
        "turns_mean" => super::exp_search::turns_mean(&good), "seconds" => py::round0(t0.elapsed().as_secs_f64()),
        "per_pair" => pp, "games" => games,
    };
    super::version::stamp(&mut res);
    res
}

/// Ligne de commande (`__main__`).
pub fn main(a: &[String]) -> i32 {
    if a.len() < 3 {
        eprintln!("sprt <graine0> <A> <B> [defaut|miroir|akali] [processus] [max_paires]");
        return 2;
    }
    let seed0: i64 = a[0].parse().unwrap();
    let (va, vb) = (&a[1], &a[2]);
    let mode = a.get(3).map(|s| s.as_str()).unwrap_or("defaut");
    let procs = a.get(4).map(|x| x.parse().unwrap()).unwrap_or(24);
    let mx = a.get(5).map(|x| x.parse().unwrap()).unwrap_or(1600);
    let r = run(seed0, va, vb, mode, procs, mx);
    append_result(&here().join("results_search.json"), &r, py::Dump { indent: None, ensure_ascii: false });
    let g = |k: &str| r.get(k).unwrap();
    let pairs = g("pairs").as_i64();
    println!(
        "{}: {} après {pairs} paires (graines {seed0}-{}), A {} ± {:.3}, LLR {}, erreurs {}, {} s",
        g("label").py_str(),
        g("verdict").py_str().to_uppercase(),
        seed0 + pairs - 1,
        py::pct(g("score_A").as_f64(), 1),
        g("se").as_f64(),
        g("llr").py_str(),
        g("errors").py_str(),
        g("seconds").py_str()
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pyrandom::PyRandom;

    /// `mesure/test_sprt.py` : scores de paire simulés, mêmes tirages que Python (mêmes comptes exacts).
    fn simulate(mu: f64, rng: &mut PyRandom) -> (Option<&'static str>, usize) {
        let q = 1.0 - 1850.0 / 2410.0;
        let (p1, p0) = (q / 2.0 + (mu - 0.5), q / 2.0 - (mu - 0.5));
        let mut xs = vec![];
        while xs.len() < 4000 {
            let u = rng.random();
            xs.push(if u < p1 {
                1.0
            } else if u < p1 + p0 {
                0.0
            } else {
                0.5
            });
            if let Some(v) = decide(&xs, 0.50, 0.52) {
                return (Some(v), xs.len());
            }
        }
        (None, xs.len())
    }

    #[test]
    fn comme_test_sprt_py() {
        assert_eq!(decide(&[0.5; 40], 0.50, 0.52), Some("rejette"));
        assert_eq!(decide(&[1.0; 39], 0.50, 0.52), None);
        let mut rng = PyRandom::new(20261020);
        // (mu, gardés, rejetés, médiane, total des paires) : valeurs de Python
        for (mu, keep, rej, med, tot) in [
            (0.50, 10, 289, 664, 241992),
            (0.52, 284, 14, 664, 249736),
            (0.47, 0, 300, 208, 65304),
            (0.55, 300, 0, 200, 63840),
        ] {
            let res: Vec<_> = (0..300).map(|_| simulate(mu, &mut rng)).collect();
            let mut ns: Vec<usize> = res.iter().map(|r| r.1).collect();
            ns.sort();
            assert_eq!(res.iter().filter(|r| r.0 == Some("garde")).count(), keep, "mu {mu}");
            assert_eq!(res.iter().filter(|r| r.0 == Some("rejette")).count(), rej, "mu {mu}");
            assert_eq!(ns[150], med, "mu {mu}");
            assert_eq!(ns.iter().sum::<usize>(), tot, "mu {mu}");
        }
    }
}
