//! Suites d'expériences sur des listes fixées : `exp.py` (Akali Wuhan contre LeBlanc IQ#5), `exp_gorica.py` (Akali
//! Gorica), `exp_plans.py` (plans de jeu), `exp_general.py` (IA générale sur decks légaux au hasard), `run.py`
//! (fonction `play`, sans ligne de commande en Python) et `quick.py`.
//!
//! Compteurs d'objets : chaque partie part de compteurs neufs (comme `Obj._n = 0` d'`exp_plans.py`). `exp.py`,
//! `exp_gorica.py` et `run.py` ne les remettent pas à zéro en Python : leurs numéros d'objets dépendent alors du
//! processus qui a joué les parties précédentes (non déterministe d'une exécution à l'autre) ; ici ils sont remis à
//! zéro, ce qui rend chaque partie fonction de sa seule graine. `quick.py` joue ses parties à la suite dans un seul
//! processus sans remise à zéro : ses compteurs sont partagés d'une partie à l'autre, comme en Python.
use std::sync::Arc;
use std::time::Instant;

use super::exp_search::{append_result, here, pct, rdeck, sc, Timed};
use super::pool;
use super::py::{self, PyVal};
use crate::ai::{Cfg, CfgV, SearchAgent, SearchOpts};
use crate::game::{Agent, Counters, DeckList, Game};
use crate::plans::{plan_agent, plans, NoPlan, Plan};
use crate::pyrandom::PyRandom;
use crate::train::{edit, tload, TDeck};

/// Boucle de partie des scripts (`while True: d = g.advance() ... g.apply(ag[d.player].decide(g, d))`), compteurs
/// d'objets donnés (`Obj._n`, `Item._n`).
pub fn play_with(decks: [DeckList; 2], agents: Vec<Box<dyn Agent>>, seed: i64, first: Option<usize>, counters: Arc<Counters>) -> Game {
    let mut g = Game::new(&decks, agents, first, seed as i128, false, crate::spec::VICTORY as i64, counters);
    for pid in 0..2 {
        let mut a = std::mem::replace(&mut g.agents[pid], Box::new(crate::game_events::FirstChoice));
        a.start(&mut g);
        g.agents[pid] = a;
    }
    loop {
        let Some(d) = g.advance(false) else { break };
        let ph = g.agents[d.player].clone_box();
        let mut a = std::mem::replace(&mut g.agents[d.player], ph);
        let r = a.decide(&mut g, &d);
        g.agents[d.player] = a;
        match r {
            Ok(act) => g.apply(&act),
            Err(_) => break,
        }
    }
    g
}

fn fresh() -> Arc<Counters> {
    Arc::new(Counters::default())
}

/// `cfg` d'une configuration (dict Python : texte ou booléen).
pub fn cfg_of(kv: &[(&str, &str)]) -> Cfg {
    let mut c = Cfg::new();
    for (k, v) in kv {
        c.set(k, if *v == "True" { CfgV::Num(1.0) } else { CfgV::Str(v.to_string()) });
    }
    c
}

fn search_agent(seed: i64, samples: usize, cfg: Cfg) -> Box<dyn Agent> {
    Box::new(SearchAgent::new(seed as i128, SearchOpts { samples, cfg, ..Default::default() }))
}

// ---------------------------------------------------------------------------------------------- listes
/// `exp.A` : Akali Wuhan
pub fn deck_a() -> TDeck {
    tload("akali_dongdong_wuhan-open_5th")
}
/// `exp.L` : LeBlanc IQ#5
pub fn deck_l() -> TDeck {
    crate::train::leblanc()
}
/// `exp.LS` : LeBlanc sidée
pub fn deck_ls() -> TDeck {
    edit(&deck_l(), &[("Deathgrip", 1), ("Hidden Blade", 1)], &[("LeBlanc, Everywhere At Once", 1), ("Thousand-Tailed Watcher", 1)])
}

/// Une configuration : (label, deck Akali, deck LeBlanc, battlefields Akali, battlefields LeBlanc, premier, cfg).
#[derive(Clone)]
pub struct Job {
    pub label: String,
    pub d: TDeck,
    pub l: TDeck,
    pub abf: Option<Vec<String>>,
    pub lbf: Option<Vec<String>>,
    pub first: Option<usize>,
    pub cfg: Cfg,
    /// `_off` d'exp_gorica (graines décalées)
    pub off: i64,
}

fn job(label: &str, d: &TDeck, l: &TDeck, abf: Option<&[&str]>, lbf: Option<&[&str]>, first: Option<usize>, cfg: &[(&str, &str)]) -> Job {
    let v = |x: Option<&[&str]>| x.map(|b| b.iter().map(|s| s.to_string()).collect());
    Job { label: label.into(), d: d.clone(), l: l.clone(), abf: v(abf), lbf: v(lbf), first, cfg: cfg_of(cfg), off: 0 }
}

/// `exp.JOBS`
pub fn exp_jobs() -> Vec<Job> {
    let (a, l, ls) = (deck_a(), deck_l(), deck_ls());
    let c1 = edit(&a, &[("Defy", 2)], &[("Mournful Witness", 2)]);
    let c2 = edit(&c1, &[("Long Sword", 1)], &[("Akali, Silent", 1)]);
    let c3 = edit(&c2, &[("Long Sword", 1)], &[("Ferrous Forerunner", 1)]);
    let c5 = edit(&c3, &[("Defy", 1)], &[("Lonely Poro", 1)]);
    let nsf = edit(&c5, &[("En Garde", 1)], &[("Not So Fast", 1)]);
    let steel = edit(&c5, &[("Adaptatron", 1)], &[("Brittle Steel", 1)]);
    let silent2 = edit(&c5, &[("Astral Heron", 1)], &[("Akali, Silent", 1)]);
    let builds = [
        ("stock Wuhan", &a),
        ("C1 -2 Defy +2 Witness", &c1),
        ("C3 +Silent +Forerunner", &c3),
        ("C5 -3 Defy +Poro", &c5),
        ("C5 +Not So Fast", &nsf),
        ("C5 +Brittle Steel", &steel),
        ("C5 +2e Silent -Heron", &silent2),
    ];
    let mut j = vec![];
    for (name, d) in builds {
        j.push(job(&format!("build: {name}"), d, &l, None, None, None, &[]));
    }
    for bf in ["Void Gate", "Targon's Peak", "Forgotten Monument", "Sigil of the Storm"] {
        j.push(job(&format!("battlefield Akali: {bf}"), &c5, &l, Some(&[bf]), None, None, &[]));
    }
    for bf in &l.battlefields {
        j.push(job(&format!("battlefield LeBlanc: {bf}"), &c5, &l, None, Some(&[bf.as_str()]), None, &[]));
    }
    for (cfg, label) in [
        (&[("mulligan", "aggro")][..], "mulligan agressif"),
        (&[("mulligan", "none")][..], "jamais de mulligan"),
        (&[("respect_ambush", "True")][..], "prudent face a Vi"),
    ] {
        j.push(job(&format!("plan: {label}"), &c5, &l, None, None, None, cfg));
    }
    j.push(job("ordre: Akali commence", &c5, &l, None, None, Some(0), &[]));
    j.push(job("ordre: LeBlanc commence", &c5, &l, None, None, Some(1), &[]));
    j.push(job("sideboard: C5 vs LeBlanc sidee", &c5, &ls, None, None, None, &[]));
    j.push(job("sideboard: stock vs LeBlanc sidee", &a, &ls, None, None, None, &[]));
    j.push(job("combo: C5 + Targon + mulligan agressif", &c5, &l, Some(&["Targon's Peak"]), None, None, &[("mulligan", "aggro")]));
    j.push(job("combo: C5 + Targon + mull. agr. vs sidee", &c5, &ls, Some(&["Targon's Peak"]), None, None, &[("mulligan", "aggro")]));
    j.push(job("confirm: C3 + Forgotten Monument", &c3, &l, Some(&["Forgotten Monument"]), None, None, &[]));
    j.push(job("confirm: C3 + Void Gate", &c3, &l, Some(&["Void Gate"]), None, None, &[]));
    j.push(job("confirm: C3 vs LeBlanc sidee", &c3, &ls, None, None, None, &[]));
    j.push(job("confirm: C3 + Forgotten vs LeBlanc sidee", &c3, &ls, Some(&["Forgotten Monument"]), None, None, &[]));
    j.push(job("confirm: stock + Forgotten Monument", &a, &l, Some(&["Forgotten Monument"]), None, None, &[]));
    j
}

/// Listes d'`exp_gorica.py` : (G, GL, GD, G2, GB).
pub fn gorica() -> (TDeck, TDeck, TDeck, TDeck, TDeck) {
    let g = tload("akali_gorica_rq-singapore");
    let gl = edit(&g, &[("Long Sword", 2)], &[("Akali, Silent", 1), ("Ferrous Forerunner", 1)]);
    let gd = edit(&g, &[("Defy", 3)], &[("Ferrous Forerunner", 2), ("Lonely Poro", 1)]);
    let g2 = edit(&gl, &[("Defy", 3)], &[("Ferrous Forerunner", 1), ("Lonely Poro", 1), ("Scuttle Crab", 1)]);
    let gb = edit(&g2, &[("Block", 1), ("Not So Fast", 1)], &[("Scuttle Crab", 2)]);
    (g, gl, gd, g2, gb)
}

/// `exp_gorica.JOBS`
pub fn gorica_jobs() -> Vec<Job> {
    let (l, ls) = (deck_l(), deck_ls());
    let (g, gl, gd, g2, gb) = gorica();
    let mut j = vec![];
    for (k, d) in [
        ("Gorica stock", &g),
        ("GL -2 Long Sword +Silent +Forerunner", &gl),
        ("GD -3 Defy +2 Forerunner +Poro", &gd),
        ("G2 -2 Long Sword -3 Defy +5 unites", &g2),
        ("GB G2 -Block -Not So Fast +2 Crab", &gb),
    ] {
        j.push(job(&format!("build: {k}"), d, &l, None, None, None, &[]));
    }
    j.push(job("sideboard: Gorica stock vs LeBlanc sidee", &g, &ls, None, None, None, &[]));
    j.push(job("sideboard: G2 vs LeBlanc sidee", &g2, &ls, None, None, None, &[]));
    for bf in &g.battlefields {
        j.push(job(&format!("battlefield Akali: {bf} (G2)"), &g2, &l, Some(&[bf.as_str()]), None, None, &[]));
    }
    let off = |mut x: Job, o: i64| {
        x.off = o;
        x
    };
    j.push(off(job("confirm (graines 10000+): Gorica stock", &g, &l, None, None, None, &[]), 10000));
    j.push(off(job("confirm (graines 10000+): G2", &g2, &l, None, None, None, &[]), 10000));
    j.push(off(job("confirm (graines 10000+): GL", &gl, &l, None, None, None, &[]), 10000));
    j.push(off(job("confirm (graines 20000+): Gorica stock", &g, &l, None, None, None, &[]), 20000));
    j.push(off(job("confirm (graines 20000+): G2", &g2, &l, None, None, None, &[]), 20000));
    let lla = tload("leblanc_mill-twistedtcg_rq-la_12th");
    j.push(job("retex LA: Gorica stock vs LeBlanc LA", &g, &lla, None, None, None, &[]));
    j.push(job("retex LA: G2 vs LeBlanc LA", &g2, &lla, None, None, None, &[]));
    for o in [30000, 40000] {
        j.push(off(job(&format!("confirm (graines {o}+): Gorica stock"), &g, &l, None, None, None, &[]), o));
        j.push(off(job(&format!("confirm (graines {o}+): G2"), &g2, &l, None, None, None, &[]), o));
    }
    j
}

/// Issue d'une partie d'`exp.one` / `exp_plans.one`.
#[derive(Clone, Debug)]
pub struct One {
    pub seed: i64,
    pub w: Option<i64>,
    pub turns: i64,
    pub first: usize,
}

/// `exp.one(arg)`
pub fn exp_one(i: i64, d: &TDeck, l: &TDeck, abfs: &[String], lbfs: &[String], first: Option<usize>, cfg: &Cfg) -> Result<One, String> {
    let mut r = PyRandom::new(i as i128 * 7919);
    let ad = d.with_bf(r.choice(abfs));
    let ld = l.with_bf(r.choice(lbfs));
    let f = first.unwrap_or_else(|| r.randrange(2) as usize);
    pool::attempt(|| {
        let ag = vec![search_agent(i, 1, cfg.clone()), search_agent(i + 500000, 1, Cfg::new())];
        let g = play_with([ad.list(), ld.list()], ag, i, Some(f), fresh());
        One { seed: i, w: g.winner, turns: g.turn_no, first: f }
    })
}

fn win1(w: Option<i64>) -> f64 {
    match w {
        Some(0) => 1.0,
        Some(-1) => 0.5,
        _ => 0.0,
    }
}

/// `exp.run_job(job, n, offset)` ; `games` = N du script.
pub fn run_job(j: &Job, n: i64, offset: i64, games: i64) -> PyVal {
    let abf = j.abf.clone().unwrap_or_else(|| j.d.battlefields.clone());
    let lbf = j.lbf.clone().unwrap_or_else(|| j.l.battlefields.clone());
    let seeds: Vec<i64> = (0..n).map(|i| i + offset).collect();
    let out = pool::map(pool::cpu_count(), &seeds, |&i| exp_one(i, &j.d, &j.l, &abf, &lbf, j.first, &j.cfg));
    let good: Vec<&One> = out.iter().filter_map(|o| o.as_ref().ok()).collect();
    let errs: Vec<&String> = out.iter().filter_map(|o| o.as_ref().err()).collect();
    let ng = good.len().max(1) as f64;
    let wins = good.iter().filter(|o| o.w == Some(0)).count() as f64 + 0.5 * good.iter().filter(|o| o.w == Some(-1)).count() as f64;
    let wr = wins / ng;
    let mut by = vec![];
    for fi in [0usize, 1] {
        let sub: Vec<&&One> = good.iter().filter(|o| o.first == fi).collect();
        if !sub.is_empty() {
            let k = if fi == 0 { "akali_first" } else { "leblanc_first" };
            by.push((k.to_string(), PyVal::Float(py::round_n(sub.iter().filter(|o| o.w == Some(0)).count() as f64 / sub.len() as f64, 3))));
        }
    }
    let turns: i64 = good.iter().map(|o| o.turns).sum();
    crate::pydict! {
        "label" => j.label.clone(), "n" => good.len() as i64, "wr" => py::round_n(wr, 4),
        "se" => py::round_n((wr * (1.0 - wr) / ng).powf(0.5), 4), "turns" => py::round_n(turns as f64 / ng, 1),
        "errors" => errs.len() as i64, "err" => errs.first().map(|e| e.to_string()), "by_first" => PyVal::Dict(by),
        "games" => games,
        "per_seed" => PyVal::List(good.iter().map(|o| PyVal::List(vec![o.seed.into(), win1(o.w).into()])).collect()),
    }
}

/// `__main__` d'`exp.py` (`results.json`, largeur 42) et d'`exp_gorica.py` (`results_gorica.json`, largeur 44) :
/// suite reprenable, configurations déjà faites (même label, même N) sautées.
pub fn suite_main(a: &[String], jobs: Vec<Job>, file: &str, width: usize) -> i32 {
    let n: i64 = a.first().map(|x| x.parse().unwrap()).unwrap_or(160);
    let budget: f64 = a.get(1).map(|x| x.parse().unwrap()).unwrap_or(480.0);
    let f = here().join(file);
    let mut done = if f.exists() { py::load_file(&f).unwrap() } else { PyVal::List(vec![]) };
    let labels: Vec<String> = done
        .as_list()
        .iter()
        .filter(|r| r.get("games") == Some(&PyVal::Int(n)))
        .filter_map(|r| r.get("label").map(|l| l.py_str()))
        .collect();
    let t0 = Instant::now();
    let mut left = 0;
    for j in &jobs {
        if labels.contains(&j.label) {
            continue;
        }
        if t0.elapsed().as_secs_f64() > budget {
            left += 1;
            continue;
        }
        let t = Instant::now();
        let mut r = run_job(j, n, j.off, n);
        super::version::stamp(&mut r);
        if let PyVal::List(v) = &mut done {
            v.push(r.clone());
        }
        py::dump_file(&f, &done, py::Dump { indent: Some(1), ensure_ascii: false });
        println!(
            "{:<width$} {:>6} +-{:.3} {:>5.1} tours err={} ({}s)",
            r.get("label").unwrap().py_str(),
            py::pct(r.get("wr").unwrap().as_f64(), 1),
            r.get("se").unwrap().as_f64(),
            r.get("turns").unwrap().as_f64(),
            r.get("errors").unwrap().py_str(),
            py::round0(t.elapsed().as_secs_f64())
        );
    }
    println!("{}", if left > 0 { format!("RESTE {left} configurations") } else { "ALL DONE".into() });
    0
}

// ---------------------------------------------------------------------------------------------- exp_plans
/// `exp_plans.JOBS` : (label, deck Akali, plan Akali, plan LeBlanc).
pub fn plans_jobs() -> Vec<(String, TDeck, Option<&'static str>, Option<&'static str>)> {
    let (g, _, _, g2, _) = gorica();
    let j = |l: &str, d: &TDeck, a: Option<&'static str>, b: Option<&'static str>| (l.to_string(), d.clone(), a, b);
    vec![
        j("G2, aucun plan (référence)", &g2, None, None),
        j("G2 plan moteur vs LeBlanc Deathknell", &g2, Some("Gorica"), Some("Deathknell")),
        j("G2 plan agressif vs LeBlanc Deathknell", &g2, Some("Gorica agressif"), Some("Deathknell")),
        j("G2 sans plan vs LeBlanc Deathknell", &g2, None, Some("Deathknell")),
        j("G2 plan agressif vs LeBlanc sans plan", &g2, Some("Gorica agressif"), None),
        j("Gorica stock plan agressif vs LeBlanc Deathknell", &g, Some("Gorica agressif"), Some("Deathknell")),
        j("G2 sans plan vs LeBlanc Hook tempo", &g2, None, Some("Hook tempo")),
        j("G2 plan moteur vs LeBlanc Hook tempo", &g2, Some("Gorica"), Some("Hook tempo")),
        j("G2 plan agressif vs LeBlanc Hook tempo", &g2, Some("Gorica agressif"), Some("Hook tempo")),
        j("G2 plan moteur vs LeBlanc Hook tempo Windswept", &g2, Some("Gorica"), Some("Hook tempo Windswept")),
        j("G2 plan agressif vs LeBlanc Hook tempo Windswept", &g2, Some("Gorica agressif"), Some("Hook tempo Windswept")),
    ]
}

/// `exp_plans.one(arg)`
pub fn plans_one(i: i64, d: &TDeck, ap: Option<&str>, lp: Option<&str>) -> Result<One, String> {
    let l = deck_l();
    let mut r = PyRandom::new(i as i128 * 7919);
    let f = r.randrange(2) as usize;
    let pa: Option<Arc<dyn Plan>> = ap.map(|x| plans(x).unwrap());
    let pl: Option<Arc<dyn Plan>> = lp.map(|x| plans(x).unwrap());
    let lbf = pl.as_ref().and_then(|p| p.battlefield(&l.battlefields, f == 1, None)).unwrap_or_else(|| r.choice(&l.battlefields).clone());
    let abf = pa
        .as_ref()
        .and_then(|p| p.battlefield(&d.battlefields, f == 0, Some(&lbf)))
        .unwrap_or_else(|| r.choice(&d.battlefields).clone());
    pool::attempt(|| {
        let ag: Vec<Box<dyn Agent>> = vec![
            Box::new(plan_agent(i as i128, pa.clone(), pl.clone(), SearchOpts::default())),
            Box::new(plan_agent(i as i128 + 500000, pl.clone(), pa.clone(), SearchOpts::default())),
        ];
        let g = play_with([d.with_bf(&abf).list(), l.with_bf(&lbf).list()], ag, i, Some(f), fresh());
        One { seed: i, w: g.winner, turns: g.turn_no, first: f }
    })
}

/// `exp_plans.suffix()`
pub fn plans_suffix() -> String {
    format!(
        "{}{}{}",
        if crate::ai::conf().tempo { " · IA tempo" } else { "" },
        if crate::plans::keep_reflection() { " · LeBlanc réel" } else { "" },
        if crate::plans::video() { " · vidéos" } else { "" }
    )
}

/// `exp_plans.run(job)`
pub fn plans_run(job: &(String, TDeck, Option<&str>, Option<&str>), n: i64, seed0: i64) -> PyVal {
    let (label, d, ap, lp) = job;
    let label = format!("{label}{}", plans_suffix());
    let seeds: Vec<i64> = (0..n).map(|k| seed0 + k).collect();
    let out = pool::map(pool::cpu_count(), &seeds, |&i| plans_one(i, d, *ap, *lp));
    let good: Vec<&One> = out.iter().filter_map(|o| o.as_ref().ok()).collect();
    let errs: Vec<&String> = out.iter().filter_map(|o| o.as_ref().err()).collect();
    let w: Vec<f64> = good.iter().map(|o| win1(o.w)).collect();
    let ng = good.len().max(1) as f64;
    let wr = py::sum(w.iter().copied()) / w.len().max(1) as f64;
    let by: Vec<(String, PyVal)> = [("akali_first", 0usize), ("leblanc_first", 1)]
        .iter()
        .map(|(k, fi)| {
            let won = good.iter().filter(|o| o.first == *fi && o.w == Some(0)).count() as f64;
            let all = good.iter().filter(|o| o.first == *fi).count().max(1) as f64;
            (k.to_string(), PyVal::Float(py::round_n(won / all, 3)))
        })
        .collect();
    let turns: i64 = good.iter().map(|o| o.turns).sum();
    let mut r = crate::pydict! {
        "label" => label, "n" => good.len() as i64, "wr" => py::round_n(wr, 4),
        "se" => py::round_n((wr * (1.0 - wr) / ng).powf(0.5), 4), "turns" => py::round_n(turns as f64 / ng, 1),
        "errors" => errs.len() as i64, "err" => errs.first().map(|e| e.to_string()), "by_first" => PyVal::Dict(by),
        "games" => n, "seed0" => seed0,
        "per_seed" => PyVal::List(good.iter().zip(&w).map(|(o, x)| PyVal::List(vec![o.seed.into(), (*x).into()])).collect()),
    };
    super::version::stamp(&mut r);
    r
}

/// `__main__` d'`exp_plans.py <parties> [graine0] [budget]` (variable ONLY : indices de JOBS à jouer).
pub fn plans_main(a: &[String]) -> i32 {
    let n: i64 = a.first().map(|x| x.parse().unwrap()).unwrap_or(400);
    let seed0: i64 = a.get(1).map(|x| x.parse().unwrap()).unwrap_or(70000);
    let budget: f64 = a.get(2).map(|x| x.parse().unwrap()).unwrap_or(1e9);
    let f = here().join("results_plans.json");
    let mut done = if f.exists() { py::load_file(&f).unwrap() } else { PyVal::List(vec![]) };
    let have: Vec<(String, i64)> = done
        .as_list()
        .iter()
        .filter(|r| r.get("games") == Some(&PyVal::Int(n)))
        .map(|r| (r.get("label").map(|x| x.py_str()).unwrap_or_default(), r.get("seed0").map(|x| x.as_i64()).unwrap_or(i64::MIN)))
        .collect();
    let t0 = Instant::now();
    let only = std::env::var("ONLY").ok().filter(|s| !s.is_empty());
    for (j, job) in plans_jobs().iter().enumerate() {
        if let Some(o) = &only {
            if !o.split(',').any(|x| x == j.to_string()) {
                continue;
            }
        }
        if have.contains(&(format!("{}{}", job.0, plans_suffix()), seed0)) || t0.elapsed().as_secs_f64() > budget {
            continue;
        }
        let t = Instant::now();
        let r = plans_run(job, n, seed0);
        if let PyVal::List(v) = &mut done {
            v.push(r.clone());
        }
        py::dump_file(&f, &done, py::Dump { indent: None, ensure_ascii: false });
        println!(
            "{:<50} {:>6} +-{:.3} {:>5.1} tours err={} ({}s)",
            r.get("label").unwrap().py_str(),
            py::pct(r.get("wr").unwrap().as_f64(), 1),
            r.get("se").unwrap().as_f64(),
            r.get("turns").unwrap().as_f64(),
            r.get("errors").unwrap().py_str(),
            py::round0(t.elapsed().as_secs_f64())
        );
    }
    println!("ALL DONE");
    0
}

// ---------------------------------------------------------------------------------------------- exp_general
/// `exp_general.VERSIONS` : nom -> (plan, tirages, horizon).
fn general_version(v: &str, d: &TDeck) -> (Arc<dyn Plan>, usize, i64) {
    match v {
        "base" | "base2" => (Arc::new(NoPlan), 1, 2),
        "general" => (Arc::new(crate::plans_general::GeneralPlan::new(&d.list())), 1, 2),
        _ => panic!("KeyError: {}", PyVal::from(v).py_repr()),
    }
}

/// `exp_general.play(seed, da, db, first, va, vb, slot)` : (gagnant, tours, temps va, temps vb).
pub fn general_play(seed: i64, da: &TDeck, db: &TDeck, first: usize, va: &str, vb: &str, slot: usize) -> (Option<i64>, i64, [Vec<f64>; 2]) {
    let mut r = PyRandom::new(seed as i128 * 7919);
    let (pa, sa, ha) = general_version(va, da);
    let (pb, sb, hb) = general_version(vb, db);
    let bfa = pa.battlefield(&da.battlefields, first == 0, None).unwrap_or_else(|| r.choice(&da.battlefields).clone());
    let bfb = pb.battlefield(&db.battlefields, first == 1, Some(&bfa)).unwrap_or_else(|| r.choice(&db.battlefields).clone());
    let (sd0, sd1) = if slot == 0 { (100, 500100) } else { (500100, 100) };
    let tm = [Arc::new(std::sync::Mutex::new(vec![])), Arc::new(std::sync::Mutex::new(vec![]))];
    let a0 = plan_agent((seed + sd0) as i128, Some(pa.clone()), Some(pb.clone()), SearchOpts { samples: sa, horizon: ha, ..Default::default() });
    let a1 = plan_agent((seed + sd1) as i128, Some(pb), Some(pa), SearchOpts { samples: sb, horizon: hb, ..Default::default() });
    let wr: Vec<Box<dyn Agent>> = vec![
        Box::new(Timed { inner: Box::new(a0), acc: tm[0].clone() }),
        Box::new(Timed { inner: Box::new(a1), acc: tm[1].clone() }),
    ];
    let g = play_with([da.with_bf(&bfa).list(), db.with_bf(&bfb).list()], wr, seed, Some(first), fresh());
    let t = |i: usize| tm[i].lock().unwrap().clone();
    (g.winner, g.turn_no, [t(0), t(1)])
}

/// `exp_general.pair` : (graine, légendes, premier, a1, a2, score, tours, tA, tB) ou l'erreur.
#[allow(clippy::type_complexity)]
pub fn general_pair(gs: i64, va: &str, vb: &str) -> (i64, Result<(f64, f64, f64, [i64; 2], Vec<f64>, Vec<f64>), String>) {
    let mut rng = PyRandom::new(gs as i128);
    let (da, db) = (rdeck(&mut rng), rdeck(&mut rng));
    let first = rng.randrange(2) as usize;
    let r = pool::attempt(|| (general_play(gs, &da, &db, first, va, vb, 0), general_play(gs, &da, &db, first, vb, va, 1)));
    (
        gs,
        r.map(|((w1, t1, tm1), (w2, t2, tm2))| {
            let (a1, a2) = (sc(w1, 0), sc(w2, 1));
            let mut ta = tm1[0].clone();
            ta.extend(&tm2[1]);
            let mut tb = tm1[1].clone();
            tb.extend(&tm2[0]);
            (a1, a2, (a1 + a2) / 2.0, [t1, t2], ta, tb)
        }),
    )
}

/// `exp_general.run(n, seed0, va, vb, procs)`
pub fn general_run(n: i64, seed0: i64, va: &str, vb: &str, procs: usize) -> PyVal {
    let t0 = Instant::now();
    let seeds: Vec<i64> = (0..n).map(|k| seed0 + k).collect();
    let out = pool::map(procs, &seeds, |&s| general_pair(s, va, vb));
    let good: Vec<(i64, &(f64, f64, f64, [i64; 2], Vec<f64>, Vec<f64>))> =
        out.iter().filter_map(|(s, r)| r.as_ref().ok().map(|x| (*s, x))).collect();
    let errs: Vec<&String> = out.iter().filter_map(|(_, r)| r.as_ref().err()).collect();
    let sc: Vec<f64> = good.iter().map(|o| o.1 .2).collect();
    let len = sc.len();
    let m = py::sum(sc.iter().copied()) / len.max(1) as f64;
    let var = py::sum(sc.iter().map(|x| (x - m).powi(2))) / (len as i64 - 1).max(1) as f64;
    let se = (var / len.max(1) as f64).sqrt();
    let ta: Vec<f64> = good.iter().flat_map(|o| o.1 .4.iter().copied()).collect();
    let tb: Vec<f64> = good.iter().flat_map(|o| o.1 .5.iter().copied()).collect();
    let ng = good.len().max(1) as f64;
    let mean_ms = |t: &[f64]| py::round_n(1000.0 * py::sum(t.iter().copied()) / t.len().max(1) as f64, 1);
    let turns: i64 = good.iter().map(|o| o.1 .3[0] + o.1 .3[1]).sum();
    let engine = std::process::Command::new("git")
        .args(["-C", &here().to_string_lossy(), "rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let mut res = crate::pydict! {
        "A" => va, "B" => vb, "pairs" => good.len() as i64, "seed0" => seed0, "score_A" => py::round_n(m, 4),
        "se" => py::round_n(se, 4), "z" => if se > 0.0 { Some(py::round_n((m - 0.5) / se, 2)) } else { None },
        "A_as_player0" => py::round_n(py::sum(good.iter().map(|o| o.1 .0)) / ng, 4),
        "A_as_player1" => py::round_n(py::sum(good.iter().map(|o| o.1 .1)) / ng, 4),
        "ms_per_decision" => crate::pydict! {
            "A_mean" => mean_ms(&ta), "A_p95" => py::round_n(1000.0 * pct(&ta, 0.95), 1),
            "B_mean" => mean_ms(&tb), "B_p95" => py::round_n(1000.0 * pct(&tb, 0.95), 1),
        },
        "turns_mean" => py::round_n(turns as f64 / (2 * good.len()).max(1) as f64, 1), "errors" => errs.len() as i64,
        "err" => errs.first().map(|e| e.to_string()), "seconds" => py::round0(t0.elapsed().as_secs_f64()),
        "per_pair" => PyVal::List(good.iter().map(|o| PyVal::List(vec![o.0.into(), o.1 .2.into()])).collect()),
        "engine" => engine,
    };
    super::version::stamp(&mut res);
    res
}

/// `__main__` d'`exp_general.py [paires] [graine0] [A] [B] [processus]`
pub fn general_main(a: &[String]) -> i32 {
    let n: i64 = a.first().map(|x| x.parse().unwrap()).unwrap_or(20);
    let seed0: i64 = a.get(1).map(|x| x.parse().unwrap()).unwrap_or(100000);
    let va = a.get(2).cloned().unwrap_or("base".into());
    let vb = a.get(3).cloned().unwrap_or("base2".into());
    let procs = a.get(4).map(|x| x.parse().unwrap()).unwrap_or(2);
    let r = general_run(n, seed0, &va, &vb, procs);
    append_result(&here().join("results_general.json"), &r, py::Dump { indent: None, ensure_ascii: false });
    let g = |k: &str| r.get(k).unwrap();
    println!(
        "{va} contre {vb}: {} ± {:.3} (z={}) sur {} paires, joueur0 {:.2} / joueur1 {:.2}, ms/coup {}, erreurs {}, {}s",
        py::pct(g("score_A").as_f64(), 1),
        g("se").as_f64(),
        g("z").py_str(),
        g("pairs").py_str(),
        g("A_as_player0").as_f64(),
        g("A_as_player1").as_f64(),
        g("ms_per_decision").py_repr(),
        g("errors").py_str(),
        g("seconds").py_str()
    );
    if let Some(PyVal::Str(e)) = r.get("err") {
        println!("{e}");
    }
    0
}

// ---------------------------------------------------------------------------------------------- run.py, quick.py
/// `run.py : one(arg)` : (gagnant, tours, points, premier, battlefields) ou l'erreur.
pub fn run_one(i: i64, a_deck: &TDeck, l_deck: &TDeck, a_bfs: &[String], l_bfs: &[String], first: Option<usize>, samples: usize) -> Result<PyVal, String> {
    let mut r = PyRandom::new(i as i128 * 7919);
    let a = a_deck.with_bf(r.choice(a_bfs));
    let l = l_deck.with_bf(r.choice(l_bfs));
    let f = first.unwrap_or_else(|| r.randrange(2) as usize);
    pool::attempt(|| {
        let ag = vec![search_agent(i, samples, Cfg::new()), search_agent(i + 500000, samples, Cfg::new())];
        let g = play_with([a.list(), l.list()], ag, i, Some(f), fresh());
        crate::pydict! {
            "w" => g.winner, "turns" => g.turn_no, "pts" => vec![g.p[0].points, g.p[1].points], "first" => f as i64,
            "bf" => vec![a.battlefield.clone(), l.battlefield.clone()],
        }
    })
    .map_err(|e| e.chars().take(200).collect())
}

/// `run.play(n, a_deck, l_deck, a_bfs, l_bfs, first, samples, seed0, procs, label)` (pas de ligne de commande en
/// Python non plus).
#[allow(clippy::too_many_arguments)]
pub fn run_play(n: i64, a_deck: &TDeck, l_deck: &TDeck, a_bfs: Option<Vec<String>>, l_bfs: Option<Vec<String>>, first: Option<usize>, samples: usize, seed0: i64, procs: Option<usize>, label: &str) -> PyVal {
    let a_bfs = a_bfs.filter(|v| !v.is_empty()).unwrap_or_else(|| a_deck.battlefields.clone());
    let l_bfs = l_bfs.filter(|v| !v.is_empty()).unwrap_or_else(|| l_deck.battlefields.clone());
    let seeds: Vec<i64> = (0..n).map(|i| seed0 + i).collect();
    let out = pool::map(procs.unwrap_or_else(pool::cpu_count), &seeds, |&i| (i, run_one(i, a_deck, l_deck, &a_bfs, &l_bfs, first, samples)));
    let errs: Vec<PyVal> = out
        .iter()
        .filter_map(|(i, o)| o.as_ref().err().map(|e| crate::pydict! {"err" => e.clone(), "seed" => *i}))
        .collect();
    let good: Vec<&PyVal> = out.iter().filter_map(|(_, o)| o.as_ref().ok()).collect();
    let w = |o: &PyVal| o.get("w").cloned().unwrap_or(PyVal::None);
    let ng = good.len().max(1) as f64;
    let wins = good.iter().filter(|o| w(o) == PyVal::Int(0)).count() as f64 + 0.5 * good.iter().filter(|o| w(o) == PyVal::Int(-1)).count() as f64;
    let wr = wins / ng;
    let turns: i64 = good.iter().map(|o| o.get("turns").unwrap().as_i64()).sum();
    let mut by = vec![];
    for f in [0i64, 1] {
        let sub: Vec<&&PyVal> = good.iter().filter(|o| o.get("first") == Some(&PyVal::Int(f))).collect();
        if !sub.is_empty() {
            let k = if f == 0 { "akali_first" } else { "leblanc_first" };
            by.push((k.to_string(), PyVal::Float(py::round_n(sub.iter().filter(|o| w(o) == PyVal::Int(0)).count() as f64 / sub.len() as f64, 3))));
        }
    }
    crate::pydict! {
        "label" => label, "n" => good.len() as i64, "wr" => py::round_n(wr, 4),
        "se" => py::round_n((wr * (1.0 - wr) / ng).powf(0.5), 4), "turns" => py::round_n(turns as f64 / ng, 1),
        "errors" => errs.len() as i64, "err_sample" => PyVal::List(errs.into_iter().take(2).collect()),
        "by_first" => PyVal::Dict(by),
    }
}

/// `quick.py <n> <sr|rs|ss>` : parties jouées à la suite, compteurs d'objets partagés (comme le processus Python).
pub fn quick_main(a: &[String]) -> i32 {
    let ad = tload("akali_dongdong_wuhan-open_5th").with_bf("Void Gate");
    let ld = deck_l().with_bf("Star Spring");
    let n: i64 = a[0].parse().unwrap();
    let mode = a[1].as_str();
    let counters = fresh();
    let mut res: Vec<(Option<i64>, i64)> = vec![];
    let t = Instant::now();
    for i in 0..n {
        let s = |x: i64| -> Box<dyn Agent> { search_agent(x, 1, Cfg::new()) };
        let r = |x: i64| -> Box<dyn Agent> { Box::new(crate::agents::RandomAgent::new(x as i128)) };
        let ag = match mode {
            "sr" => vec![s(i), r(i)],
            "rs" => vec![r(i), s(i)],
            _ => vec![s(i), s(i + 99)],
        };
        let g = play_with([ad.list(), ld.list()], ag, i, None, counters.clone());
        match res.iter_mut().find(|x| x.0 == g.winner) {
            Some(x) => x.1 += 1,
            None => res.push((g.winner, 1)),
        }
        println!(
            "{i} {} {} [{}, {}] {}",
            PyVal::from(g.winner).py_repr(),
            g.turn_no,
            g.p[0].points,
            g.p[1].points,
            py::repr_f64(py::round_n(t.elapsed().as_secs_f64(), 1))
        );
    }
    // repr(Counter) : par nombre décroissant, ordre d'insertion à égalité
    res.sort_by(|a, b| b.1.cmp(&a.1));
    let parts: Vec<String> = res.iter().map(|(k, v)| format!("{}: {v}", PyVal::from(*k).py_repr())).collect();
    println!("Counter({{{}}})", parts.join(", "));
    0
}
