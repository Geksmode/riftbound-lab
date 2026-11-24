//! `riftbound/manager/run_session.py` : runner de l'agent manager.
//!
//! ```text
//!   run_session [agenda.json] [budget_secondes]
//! ```
//! Lit l'agenda (liste de jobs JSON), joue chaque job sur UN bloc de graines neuf commun à la session (comparaisons
//! appariées entre jobs de la même session), puis avance `state.json` pour que ces graines ne soient plus jamais
//! réutilisées. Résultats : `results/<session>.json` (avec `per_seed`), même format que Python.
//!
//! Écarts avec le Python (le moteur n'est plus une copie de fichiers) :
//! - pas de copie figée du moteur ni de `plans.py` (`results/engine_session_NNN`, `plans_session_NNN`) : le moteur
//!   Rust est le binaire lui-même, une session ne peut donc pas mélanger deux versions de l'IA ; `engine_md5` est la
//!   version du moteur Rust (« …-rust ») et `plans_md5` la même valeur (les plans sont compilés dans le moteur) ;
//! - `RB_RESUME=n` reprend la session n seulement si le binaire est de la même version que celui qui l'a commencée ;
//! - `RB_MANAGER_DIR` remplace le dossier `riftbound/manager` (essais sans toucher aux fichiers du dépôt).
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use super::exp::play_with;
use super::pool;
use super::py::{self, Dump, PyVal};
use crate::ai::SearchOpts;
use crate::game::{Agent, Counters};
use crate::plans::{plan_agent, plans, Plan};
use crate::pyrandom::PyRandom;
use crate::train::{edit, tload, TDeck};

/// `HERE` : `riftbound/manager`
pub fn here() -> PathBuf {
    match std::env::var("RB_MANAGER_DIR") {
        Ok(d) if !d.is_empty() => d.into(),
        _ => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../riftbound/manager"),
    }
}

fn strs(v: Option<&PyVal>) -> Option<Vec<String>> {
    match v {
        Some(PyVal::List(l)) => Some(l.iter().map(|x| x.py_str()).collect()),
        _ => None,
    }
}

fn pairs(v: Option<&PyVal>) -> Vec<(String, usize)> {
    v.map(|l| l.as_list().iter().map(|x| (x.as_list()[0].py_str(), x.as_list()[1].as_i64() as usize)).collect()).unwrap_or_default()
}

/// `build(spec)` : deck d'un job (base, retraits, ajouts), Err(message) comme les `ValueError` de Python.
pub fn build(spec: &PyVal) -> Result<TDeck, String> {
    let base = spec.get("base").map(|b| b.py_str()).unwrap_or_default();
    let d = pool::attempt(|| {
        let d = tload(&base);
        let (o, a) = (pairs(spec.get("out")), pairs(spec.get("add")));
        let o: Vec<(&str, usize)> = o.iter().map(|(n, q)| (n.as_str(), *q)).collect();
        let a: Vec<(&str, usize)> = a.iter().map(|(n, q)| (n.as_str(), *q)).collect();
        edit(&d, &o, &a)
    })?;
    for n in &d.main {
        if crate::cards::impl_of(n).is_none() {
            return Err(format!("ValueError: carte non modélisée : {n}"));
        }
    }
    if d.main.len() + 1 != 40 {
        // le champion choisi est compté à part
        return Err(format!("ValueError: {} : {}+1 cartes au main deck (40 attendues)", base, d.main.len()));
    }
    Ok(d)
}

/// Issue d'une partie de `one`.
pub struct One {
    pub w: Option<i64>,
    pub turns: i64,
    pub first: usize,
    pub abf: String,
    pub lbf: String,
    pub seed: i64,
}

/// `one(arg)` : Err(traceback) si la partie lève une exception.
#[allow(clippy::too_many_arguments)]
pub fn one(
    i: i64,
    a_deck: &TDeck,
    l_deck: &TDeck,
    ap: Option<&str>,
    lp: Option<&str>,
    abfs: &Option<Vec<String>>,
    lbfs: &Option<Vec<String>>,
    first: Option<usize>,
) -> Result<One, String> {
    let mut r = PyRandom::new(i as i128 * 7919);
    let f = first.unwrap_or_else(|| r.randrange(2) as usize);
    let pa: Option<Arc<dyn Plan>> = ap.map(|x| plans(x).unwrap_or_else(|| panic!("KeyError: {x:?}")));
    let pl: Option<Arc<dyn Plan>> = lp.map(|x| plans(x).unwrap_or_else(|| panic!("KeyError: {x:?}")));
    let lb_list: Vec<String> = lbfs.clone().filter(|b| !b.is_empty()).unwrap_or_else(|| l_deck.battlefields.clone());
    let ab_list: Vec<String> = abfs.clone().filter(|b| !b.is_empty()).unwrap_or_else(|| a_deck.battlefields.clone());
    let lb_forced = lbfs.as_ref().is_some_and(|b| !b.is_empty());
    let ab_forced = abfs.as_ref().is_some_and(|b| !b.is_empty());
    let lbf = (if pl.is_some() && !lb_forced { pl.as_ref().unwrap().battlefield(&l_deck.battlefields, f == 1, None) } else { None })
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| r.choice(&lb_list).clone());
    let abf = (if pa.is_some() && !ab_forced {
        pa.as_ref().unwrap().battlefield(&a_deck.battlefields, f == 0, Some(&lbf))
    } else {
        None
    })
    .filter(|b| !b.is_empty())
    .unwrap_or_else(|| r.choice(&ab_list).clone());
    let (ad, ld) = (a_deck.with_bf(&abf), l_deck.with_bf(&lbf));
    pool::attempt(|| {
        let ag: Vec<Box<dyn Agent>> = vec![
            Box::new(plan_agent(i as i128, pa.clone(), pl.clone(), SearchOpts::default())),
            Box::new(plan_agent(i as i128 + 500000, pl.clone(), pa.clone(), SearchOpts::default())),
        ];
        let g = play_with([ad.list(), ld.list()], ag, i, Some(f), Arc::new(Counters::default()));
        One { w: g.winner, turns: g.turn_no, first: f, abf: abf.clone(), lbf: lbf.clone(), seed: i }
    })
}

/// `run(job, seed0)`
pub fn run(job: &PyVal, seed0: i64) -> Result<PyVal, String> {
    let a = build(job.get("akali").ok_or("KeyError: 'akali'")?)?;
    let l = build(job.get("opp").ok_or("KeyError: 'opp'")?)?;
    let n = job.get("n").map(|x| x.as_i64()).unwrap_or(400);
    let nz = |k: &str| job.get(k).filter(|v| v.truthy()).map(|v| v.py_str());
    let (ap, lp) = (nz("akali_plan"), nz("opp_plan"));
    let (abfs, lbfs) = (strs(job.get("akali_bfs")), strs(job.get("opp_bfs")));
    let first = job.get("first").filter(|v| !matches!(v, PyVal::None)).map(|v| v.as_i64() as usize);
    let seeds: Vec<i64> = (0..n).map(|k| seed0 + k).collect();
    let out = pool::map(pool::cpu_count(), &seeds, |&s| {
        one(s, &a, &l, ap.as_deref(), lp.as_deref(), &abfs, &lbfs, first).map_err(|e| (s, e))
    });
    let good: Vec<&One> = out.iter().filter_map(|o| o.as_ref().ok()).collect();
    let errs: Vec<&(i64, String)> = out.iter().filter_map(|o| o.as_ref().err()).collect();
    let w: Vec<f64> = good.iter().map(|o| if o.w == Some(0) { 1.0 } else if o.w == Some(-1) { 0.5 } else { 0.0 }).collect();
    let ng = good.len().max(1) as f64;
    let wr = py::sum(w.iter().copied()) / ng;
    let by = |fi: usize| {
        let tot = good.iter().filter(|o| o.first == fi).count().max(1) as f64;
        py::round_n(good.iter().filter(|o| o.first == fi && o.w == Some(0)).count() as f64 / tot, 3)
    };
    let turns: i64 = good.iter().map(|o| o.turns).sum();
    Ok(crate::pydict! {
        "label" => job.get("label").map(|x| x.py_str()).unwrap_or_default(), "job" => job.clone(), "n" => good.len() as i64,
        "wr" => py::round_n(wr, 4), "se" => py::round_n((wr * (1.0 - wr) / ng).powf(0.5), 4),
        "turns" => py::round_n(turns as f64 / ng, 1), "errors" => errs.len() as i64,
        "err" => errs.first().map(|e| e.1.clone()),
        "by_first" => crate::pydict! { "akali_first" => by(0), "opp_first" => by(1) },
        "seed0" => seed0,
        "per_seed" => PyVal::List(good.iter().zip(&w).map(|(o, w)| PyVal::List(vec![o.seed.into(), (*w).into(), o.abf.clone().into(), o.lbf.clone().into()])).collect()),
    })
}

/// `paired(r1, r2)` : différence appariée r1 − r2 (mêmes graines), son écart-type et n.
pub fn paired(r1: &PyVal, r2: &PyVal) -> Option<(f64, f64, usize)> {
    let rows = |r: &PyVal| -> Vec<(i64, f64)> {
        r.get("per_seed").map(|p| p.as_list().iter().map(|x| (x.as_list()[0].as_i64(), x.as_list()[1].as_f64())).collect()).unwrap_or_default()
    };
    let m1 = rows(r1);
    let d: Vec<f64> = rows(r2)
        .iter()
        .filter_map(|(s, w)| m1.iter().rev().find(|e| e.0 == *s).map(|e| e.1 - w))
        .collect();
    if d.len() < 2 {
        return None;
    }
    let mu = py::sum(d.iter().copied()) / d.len() as f64;
    let var = py::sum(d.iter().map(|x| (x - mu).powi(2))) / (d.len() - 1) as f64;
    Some((py::round_n(mu, 4), py::round_n((var / d.len() as f64).powf(0.5), 4), d.len()))
}

/// Ligne de commande (`__main__`).
pub fn main(a: &[String]) -> i32 {
    let h = here();
    let agenda = a.first().map(PathBuf::from).unwrap_or_else(|| h.join("agenda.json"));
    let budget: f64 = a.get(1).map(|x| x.parse().expect("budget")).unwrap_or(3000.0);
    let resume: i64 = std::env::var("RB_RESUME").ok().and_then(|x| x.parse().ok()).unwrap_or(0);
    let state_f = h.join("state.json");
    let jobs = match py::load_file(&agenda) {
        Ok(v) => v.get("jobs").map(|j| j.as_list().to_vec()).unwrap_or_default(),
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let md5 = super::version::engine_md5();
    let mut st = py::load_file(&state_f).unwrap_or_else(|e| panic!("{e}"));
    let session = if resume != 0 { resume } else { st.get("session").unwrap().as_i64() + 1 };
    let out_f = h.join("results").join(format!("session_{session:03}.json"));
    if resume != 0 {
        let prev = py::load_file(&out_f).unwrap_or_else(|e| panic!("{e}"));
        let was = prev.get("engine_md5").map(|x| x.py_str()).unwrap_or_default();
        if was != md5 {
            eprintln!("reprise impossible : la session {resume} a été commencée avec le moteur {was}, ce binaire est {md5}");
            return 1;
        }
    }
    std::fs::create_dir_all(h.join("results")).ok();
    println!("moteur Rust ({md5}), plans compilés dans le moteur");
    let (shared0, mut done): (i64, Vec<PyVal>) = if resume != 0 {
        let prev = py::load_file(&out_f).unwrap();
        let d = prev.get("results").map(|r| r.as_list().to_vec()).unwrap_or_default();
        let s0 = prev.get("shared_seed0").unwrap().as_i64();
        println!("reprise de la session {session} : {} job(s) déjà joué(s), graines {s0}+", d.len());
        (s0, d)
    } else {
        let span = jobs.iter().map(|j| j.get("n").map(|x| x.as_i64()).unwrap_or(400)).max().unwrap_or(400);
        let s0 = st.get("next_seed").unwrap().as_i64();
        st.set("next_seed", PyVal::Int(s0 + span));
        st.set("session", PyVal::Int(session));
        py::dump_file(&state_f, &st, Dump { indent: Some(1), ensure_ascii: true }); // réserver les graines avant de jouer
        (s0, vec![])
    };
    let t0 = Instant::now();
    let played: Vec<String> = done.iter().map(|r| r.get("label").map(|l| l.py_str()).unwrap_or_default()).collect();
    for job in &jobs {
        // valider avant de dépenser du calcul
        for k in ["akali", "opp"] {
            if let Err(e) = build(job.get(k).unwrap_or(&PyVal::None)) {
                eprintln!("{e}");
                return 1;
            }
        }
    }
    let label = |j: &PyVal| j.get("label").map(|l| l.py_str()).unwrap_or_default();
    for job in &jobs {
        if played.contains(&label(job)) {
            continue;
        }
        if t0.elapsed().as_secs_f64() > budget {
            println!("BUDGET épuisé, non joué : {}", label(job));
            continue;
        }
        let s0 = if let Some(s) = job.get("seed0").filter(|v| !matches!(v, PyVal::None)) {
            s.as_i64() // réparation : rejouer exactement un bloc déjà réservé
        } else if job.get("fresh").is_some_and(|v| v.truthy()) {
            let mut st = py::load_file(&state_f).unwrap();
            let s0 = st.get("next_seed").unwrap().as_i64();
            st.set("next_seed", PyVal::Int(s0 + job.get("n").map(|x| x.as_i64()).unwrap_or(400)));
            py::dump_file(&state_f, &st, Dump { indent: Some(1), ensure_ascii: true });
            s0
        } else {
            shared0
        };
        let t = Instant::now();
        let r = match run(job, s0) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
                return 1;
            }
        };
        done.push(r.clone());
        let doc = crate::pydict! {
            "session" => session, "shared_seed0" => shared0, "plans_md5" => md5.clone(), "engine_md5" => md5.clone(),
            "results" => PyVal::List(done.clone()),
        };
        py::dump_file(&out_f, &doc, Dump { indent: None, ensure_ascii: false });
        println!(
            "{:<55} {:>6} ±{:.3} n={} graines {}+ err={} ({}s)",
            r.get("label").unwrap().py_str(),
            py::pct(r.get("wr").unwrap().as_f64(), 1),
            r.get("se").unwrap().as_f64(),
            r.get("n").unwrap().py_str(),
            s0,
            r.get("errors").unwrap().py_str(),
            py::round0(t.elapsed().as_secs_f64())
        );
    }
    let is_fresh = |r: &PyVal| r.get("job").and_then(|j| j.get("fresh")).is_some_and(|v| v.truthy());
    let base = done.iter().find(|r| !is_fresh(r)).cloned();
    if let Some(base) = &base {
        for r in &done {
            if !std::ptr::eq(r, done.iter().find(|x| !is_fresh(x)).unwrap()) && !is_fresh(r) {
                if let Some(p) = paired(r, base) {
                    println!(
                        "  apparié vs '{}': {}: {:+.1} ± {:.1} pts (n={})",
                        base.get("label").unwrap().py_str(),
                        r.get("label").unwrap().py_str(),
                        p.0 * 100.0,
                        p.1 * 100.0,
                        p.2
                    );
                }
            }
        }
    }
    println!("SESSION {session} TERMINÉE -> {}", out_f.display());
    0
}
