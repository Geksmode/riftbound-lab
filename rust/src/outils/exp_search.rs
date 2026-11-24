//! `exp_search.py` : recherche de l'IA (`ai::SearchAgent::pick`), version A contre version B, parties appariées.
//!
//! ```text
//!   exp_search <paires> <graine0> <A> <B> <akali|miroir|defaut> [processus]
//! ```
//! Une paire = une graine g jouée DEUX fois, mêmes decks, même premier joueur, mêmes battlefields : partie 1, A tient
//! le joueur 0 ; partie 2, B tient le joueur 0 (places échangées). Score de A pour la paire = moyenne des deux
//! parties ; l'écart-type est celui de la moyenne sur les paires. Graines des agents liées au siège (g et g+500000).
//!   akali  : Akali G2 (plan Gorica) contre LeBlanc IQ#5 (plan Hook tempo) ;
//!   defaut : deux decks par défaut (train.default_decks) tirés au hasard, IA générique des deux côtés ;
//!   miroir : un deck légal au hasard (exp_general.rdeck) des deux côtés, IA générique.
//! Versions : VERSIONS (nom -> (mode de recherche, samples, sh_extra)), réglages après @ (`version`) ;
//! « @plan=leg » : plans par légende (plans_legendes.plan_for). Résultats : results_search.json (`stamp`, version du
//! moteur Rust « -rust »). Temps par décision : moyenne et p95 des decide() à plus d'une option, par version.
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::pool;
use super::py::{self, PyVal};
use crate::ai::{Cfg, CfgV, SearchOpts};
use crate::game::{Agent, Decision, Game, Res, O};
use crate::json::Json;
use crate::plans::{plan_agent, plans, NoPlan, Plan};
use crate::pyrandom::PyRandom;
use crate::train::TDeck;
use crate::value::{Map, Value};

/// `VERSIONS` : nom -> (mode de recherche, tirages, sh_extra)
pub const VERSIONS: [(&str, &str, usize, f64); 6] = [
    ("old", "old", 1, 0.0),   // recherche d'avant le 2026-10-09 (niveau Normal)
    ("old2", "old", 2, 0.0),  // ancienne, 2 tirages par option : même budget que sh
    ("crn", "crn", 1, 0.0),   // tirages communs seuls : même budget que old
    ("sh", "sh", 1, 1.0),     // tirages communs + élimination en passes : budget ≈ 2 × old
    ("turn", "turn", 1, 1.0), // tour entier en faisceau (turn_w, turn_d dans cfg) ; réactions en sh
    ("sh05", "sh", 1, 0.5),   // budget ≈ 1,5 × old
];

/// Dossier des scripts Python (`HERE` : `riftbound/engine`), où sont lus et écrits les `results*.json`.
/// `RB_OUT_DIR` le remplace (essais sans toucher aux fichiers du dépôt).
pub fn here() -> std::path::PathBuf {
    match std::env::var("RB_OUT_DIR") {
        Ok(d) if !d.is_empty() => d.into(),
        _ => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../riftbound/engine"),
    }
}

/// `_defaults()` : clés des decks par défaut de chaque légende, triées.
pub fn defaults() -> &'static Vec<String> {
    static D: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    D.get_or_init(|| {
        let mut v: Vec<String> = crate::train::default_decks().into_values().collect();
        v.sort();
        v
    })
}

/// Python : `float(x)` si `x.replace(".", "", 1).lstrip("-").isdigit()`.
fn num_like(x: &str) -> bool {
    let y = x.replacen('.', "", 1);
    let y = y.trim_start_matches('-');
    !y.is_empty() && y.chars().all(|c| c.is_ascii_digit())
}

/// Version analysée : (recherche, tirages, sh_extra, horizon, cfg).
pub type Ver = (String, usize, f64, i64, Cfg);

/// `version(v)` : nom de VERSIONS, éventuellement suivi de réglages : « old@h=1,card0=1.8,pol=1 ». Clés : s
/// (recherche), n (tirages), x (sh_extra), h (horizon) ; une clé de ai.EV = poids de l'évaluation ; toute autre clé =
/// entrée de cfg.
pub fn version(v: &str) -> Ver {
    let (nm, opts) = v.split_once('@').unwrap_or((v, ""));
    let &(_, sm, n, ex) =
        VERSIONS.iter().find(|x| x.0 == nm).unwrap_or_else(|| panic!("KeyError: {}", PyVal::from(nm).py_repr()));
    let (mut sm, mut n, mut ex, mut h) = (sm.to_string(), n, ex, 2i64);
    let mut cfg = Cfg::new();
    let mut ev: Vec<(String, f64)> = vec![];
    for kv in opts.split(',').filter(|s| !s.is_empty()) {
        let (k, x) = kv.split_once('=').unwrap_or((kv, ""));
        match k {
            "s" => sm = x.to_string(),
            "n" => n = x.parse().unwrap_or_else(|_| panic!("ValueError: int({x:?})")),
            "x" => ex = x.parse().unwrap_or_else(|_| panic!("ValueError: float({x:?})")),
            "h" => h = x.parse().unwrap_or_else(|_| panic!("ValueError: int({x:?})")),
            _ if k != "amb_hold" && crate::ai::ev().get(k).is_some() => {
                let f: f64 = x.parse().unwrap_or_else(|_| panic!("ValueError: float({x:?})"));
                match ev.iter_mut().find(|e| e.0 == k) {
                    Some(e) => e.1 = f,
                    None => ev.push((k.to_string(), f)),
                }
            }
            _ => cfg.set(k, if num_like(x) { CfgV::Num(x.parse().unwrap()) } else { CfgV::Str(x.to_string()) }),
        }
    }
    if !ev.is_empty() {
        cfg.set("ev", CfgV::Ev(ev));
    }
    (sm, n, ex, h, cfg)
}

/// Paquets, plans neufs et premier joueur d'une graine (`setup`).
pub struct Setup {
    pub decks: [TDeck; 2],
    pub plans: [Arc<dyn Plan>; 2],
    pub first: usize,
}

/// `rdeck(rng)` d'`exp_general.py` : deck légal au hasard (même procédé que train/t_rand.py).
pub fn rdeck(rng: &mut PyRandom) -> TDeck {
    struct Cat {
        cards: Vec<Json>,
        leg: Vec<usize>,
        bfs: Vec<String>,
    }
    static C: std::sync::OnceLock<Cat> = std::sync::OnceLock::new();
    let cat = C.get_or_init(|| {
        let j = crate::json::parse(&crate::train::catalog()).expect("catalogue illisible");
        let cards = j.g("cards").arr().to_vec();
        let leg = (0..cards.len()).filter(|&i| cards[i].g("t").str() == Some("Legend") && cards[i].g("m").truthy()).collect();
        let mut bfs: Vec<String> = cards
            .iter()
            .filter(|c| c.g("t").str() == Some("Battlefield") && c.g("m").truthy() && !c.g("ban").truthy())
            .map(|c| c.g("n").str().unwrap().to_string())
            .collect();
        bfs.sort();
        Cat { cards, leg, bfs }
    });
    let strs = |j: &Json| -> Vec<String> { j.arr().iter().map(|x| x.str().unwrap_or("").to_string()).collect() };
    for _ in 0..50 {
        let lg = &cat.cards[*rng.choice(&cat.leg)];
        let dom = strs(lg.g("d"));
        let ltg = strs(lg.g("tg"));
        let within = |c: &Json| strs(c.g("d")).iter().all(|x| dom.contains(x));
        let ch: Vec<&Json> = cat
            .cards
            .iter()
            .filter(|c| {
                c.g("t").str() == Some("Unit")
                    && c.g("s").str() == Some("Champion")
                    && c.g("m").truthy()
                    && strs(c.g("tg")).iter().any(|t| ltg.contains(t))
                    && within(c)
            })
            .collect();
        let pool: Vec<String> = cat
            .cards
            .iter()
            .filter(|c| {
                matches!(c.g("t").str(), Some("Unit" | "Spell" | "Gear"))
                    && c.g("m").truthy()
                    && within(c)
                    && !matches!(c.g("s").str(), Some("Champion" | "Signature"))
                    && !c.g("ban").truthy()
                    && !c.g("tx").str().unwrap_or("").contains("Unique")
            })
            .map(|c| c.g("n").str().unwrap().to_string())
            .collect();
        if ch.is_empty() || pool.len() < 14 {
            continue;
        }
        let mut main: Vec<String> = vec![];
        while main.len() < 39 {
            let n = rng.choice(&pool).clone();
            if main.iter().filter(|x| **x == n).count() < 3 {
                main.push(n);
            }
        }
        let champ = rng.choice(&ch).g("n").str().unwrap().to_string();
        let r0 = format!("{} Rune", dom[0]);
        let r1 = format!("{} Rune", dom[dom.len() - 1]);
        let runes: Vec<String> = std::iter::repeat_n(r0, 6).chain(std::iter::repeat_n(r1, 6)).collect();
        let bfs = rng.sample(&cat.bfs, 3);
        let d = crate::jo! {
            "name" => "r", "legend" => lg.g("n").str().unwrap().to_string(), "champion" => champ, "main" => main,
            "runes" => runes, "battlefields" => bfs,
        };
        if !crate::train::validate(&d).iter().any(|x| !x.starts_with('⚠')) {
            return crate::train::deck_arg(&d, crate::train::leblanc).expect("deck validé");
        }
    }
    panic!("RuntimeError: pas de deck")
}

/// `setup(seed, mode)` : mêmes decks et premier joueur pour les deux parties d'une paire.
pub fn setup(seed: i64, mode: &str) -> Setup {
    let mut r = PyRandom::new(seed as i128 * 7919);
    if mode == "akali" {
        let f = r.randrange(2) as usize;
        let (pa, pl) = (plans("Gorica").unwrap(), plans("Hook tempo").unwrap());
        let (g2, l) = (crate::train::g2(), crate::train::leblanc());
        let lbf = pl.battlefield(&l.battlefields, f == 1, None).unwrap_or_else(|| r.choice(&l.battlefields).clone());
        let abf =
            pa.battlefield(&g2.battlefields, f == 0, Some(&lbf)).unwrap_or_else(|| r.choice(&g2.battlefields).clone());
        return Setup { decks: [g2.with_bf(&abf), l.with_bf(&lbf)], plans: [pa, pl], first: f };
    }
    let np = || -> Arc<dyn Plan> { Arc::new(NoPlan) };
    if mode == "defaut" {
        let dd = defaults();
        let (ka, kb) = (r.choice(dd).clone(), r.choice(dd).clone());
        let (da, db) = (crate::train::tload(&ka), crate::train::tload(&kb));
        let f = r.randrange(2) as usize;
        let sorted = |d: &TDeck| {
            let mut b = d.battlefields.clone();
            b.sort();
            b
        };
        let ba = r.choice(&sorted(&da)).clone();
        let bb = r.choice(&sorted(&db)).clone();
        return Setup { decks: [da.with_bf(&ba), db.with_bf(&bb)], plans: [np(), np()], first: f };
    }
    let d = rdeck(&mut PyRandom::new(seed as i128));
    let f = r.randrange(2) as usize;
    let mut b = d.battlefields.clone();
    b.sort();
    let (b0, b1) = (r.choice(&b).clone(), r.choice(&b).clone());
    Setup { decks: [d.with_bf(&b0), d.with_bf(&b1)], plans: [np(), np()], first: f }
}

/// `agents(seed, decks, plans, v0, v1)` : version v0 au joueur 0, v1 au joueur 1 ; « @plan=leg » remplace les plans
/// du mode par ceux de plans_legendes, pour soi et pour le modèle de l'adversaire.
pub fn agents(seed: i64, s: &Setup, v0: &str, v1: &str) -> Vec<crate::ai::SearchAgent> {
    let mut ag = vec![];
    for (i, v) in [v0, v1].into_iter().enumerate() {
        let (sm, n, ex, h, mut cfg) = version(v);
        let (mut pl, mut op) = (s.plans[i].clone(), s.plans[1 - i].clone());
        let leg = matches!(cfg.get("plan"), Some(CfgV::Str(x)) if x == "leg");
        cfg.0.retain(|(k, _)| k != "plan");
        if leg {
            pl = crate::plans_legendes::plan_for(&s.decks[i].list());
            op = crate::plans_legendes::plan_for(&s.decks[1 - i].list());
        }
        let o = SearchOpts { samples: n, search: Some(sm), sh_extra: Some(ex), horizon: h, cfg, ..Default::default() };
        ag.push(plan_agent(seed as i128 + 500000 * i as i128, Some(pl), Some(op), o));
    }
    ag
}

/// `exp_general.Timed` : enveloppe qui mesure le temps de chaque decide() à plus d'une option.
#[derive(Clone)]
pub struct Timed {
    pub inner: Box<dyn Agent>,
    pub acc: Arc<Mutex<Vec<f64>>>,
}

impl Agent for Timed {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        self.inner.choose(g, pid, kind, options, ctx)
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        self.inner.mulligan(g, pid)
    }
    fn start(&mut self, g: &mut Game) {
        self.inner.start(g)
    }
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        let t = Instant::now();
        let r = self.inner.decide(g, d)?;
        if d.options.as_ref().is_some_and(|o| o.len() > 1) {
            self.acc.lock().unwrap().push(t.elapsed().as_secs_f64());
        }
        Ok(r)
    }
    fn every_choice(&self) -> bool {
        self.inner.every_choice()
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
    fn as_any(&mut self) -> Option<&mut dyn std::any::Any> {
        self.inner.as_any()
    }
}

/// Partie jouée jusqu'au bout (boucle `while True: d = g.advance() ...` des scripts).
pub fn play_out(decks: [crate::game::DeckList; 2], agents: Vec<Box<dyn Agent>>, seed: i64, first: usize) -> Game {
    crate::agents::play_game(&decks, agents, seed as i128, Some(first), false, usize::MAX)
}

/// Une partie de `play` : (gagnant, tours, temps [joueur 0, joueur 1], premier, légendes).
pub struct Played {
    pub w: Option<i64>,
    pub turns: i64,
    pub tm: [Vec<f64>; 2],
    pub first: usize,
    pub legends: [String; 2],
}

/// `play(seed, mode, v0, v1)`
pub fn play(seed: i64, mode: &str, v0: &str, v1: &str) -> Played {
    let s = setup(seed, mode); // plans neufs à chaque partie (ils gardent un état)
    let tm = [Arc::new(Mutex::new(vec![])), Arc::new(Mutex::new(vec![]))];
    let ag = agents(seed, &s, v0, v1);
    let wr: Vec<Box<dyn Agent>> = ag
        .into_iter()
        .enumerate()
        .map(|(i, a)| Box::new(Timed { inner: Box::new(a), acc: tm[i].clone() }) as Box<dyn Agent>)
        .collect();
    let g = play_out([s.decks[0].list(), s.decks[1].list()], wr, seed, s.first);
    let t = |i: usize| tm[i].lock().unwrap().clone();
    Played {
        w: g.winner,
        turns: g.turn_no,
        tm: [t(0), t(1)],
        first: s.first,
        legends: [s.decks[0].legend.clone(), s.decks[1].legend.clone()],
    }
}

/// Résultat d'une paire (`pair`).
#[derive(Clone, Debug)]
pub struct Pair {
    pub seed: i64,
    pub legends: [String; 2],
    pub first: usize,
    pub a1: f64,
    pub a2: f64,
    pub w: [Option<i64>; 2],
    pub score: f64,
    pub turns: [i64; 2],
    pub t_a: Vec<f64>,
    pub t_b: Vec<f64>,
}

/// `sc(w, me)`
pub fn sc(w: Option<i64>, me: i64) -> f64 {
    match w {
        None | Some(-1) => 0.5,
        Some(x) if x == me => 1.0,
        _ => 0.0,
    }
}

/// `pair((gs, va, vb, mode))` : Err(message) si une partie lève une exception (panique).
pub fn pair(gs: i64, va: &str, vb: &str, mode: &str) -> Result<Pair, String> {
    let (p1, p2) = pool::attempt(|| (play(gs, mode, va, vb), play(gs, mode, vb, va)))?;
    let (a1, a2) = (sc(p1.w, 0), sc(p2.w, 1));
    let mut t_a = p1.tm[0].clone();
    t_a.extend(p2.tm[1].iter());
    let mut t_b = p1.tm[1].clone();
    t_b.extend(p2.tm[0].iter());
    Ok(Pair {
        seed: gs,
        legends: p1.legends,
        first: p1.first,
        a1,
        a2,
        w: [p1.w, p2.w],
        score: (a1 + a2) / 2.0,
        turns: [p1.turns, p2.turns],
        t_a,
        t_b,
    })
}

/// `pct(xs, q)` d'`exp_general.py`
pub fn pct(xs: &[f64], q: f64) -> f64 {
    let mut v = xs.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if v.is_empty() {
        f64::NAN
    } else {
        v[(v.len() - 1).min((q * v.len() as f64) as usize)]
    }
}

/// `per_pair` et `games` d'un résultat
pub fn pair_rows(good: &[Pair]) -> (PyVal, PyVal) {
    let pp = PyVal::List(good.iter().map(|o| PyVal::List(vec![o.seed.into(), o.score.into()])).collect());
    let games = PyVal::List(
        good.iter()
            .map(|o| {
                PyVal::List(vec![
                    o.seed.into(),
                    (o.first as i64).into(),
                    PyVal::List(o.w.iter().map(|w| PyVal::from(*w)).collect()),
                    o.legends[0].clone().into(),
                ])
            })
            .collect(),
    );
    (pp, games)
}

/// Moyenne des tours (`round(sum(sum(o["turns"]) ...) / max(1, 2 * n), 1)`).
pub fn turns_mean(good: &[Pair]) -> f64 {
    let s: i64 = good.iter().map(|o| o.turns[0] + o.turns[1]).sum();
    py::round_n(s as f64 / (2 * good.len()).max(1) as f64, 1)
}

/// `run(n, seed0, va, vb, mode, procs)`
pub fn run(n: i64, seed0: i64, va: &str, vb: &str, mode: &str, procs: usize) -> PyVal {
    let t0 = Instant::now();
    let seeds: Vec<i64> = (0..n).map(|k| seed0 + k).collect();
    let out = pool::map(procs, &seeds, |&s| (s, pair(s, va, vb, mode)));
    let good: Vec<Pair> = out.iter().filter_map(|(_, r)| r.as_ref().ok().cloned()).collect();
    let errs: Vec<String> = out.iter().filter_map(|(_, r)| r.as_ref().err().cloned()).collect();
    let sc: Vec<f64> = good.iter().map(|o| o.score).collect();
    let len = sc.len();
    let m = py::sum(sc.iter().copied()) / len.max(1) as f64;
    let var = py::sum(sc.iter().map(|x| (x - m).powi(2))) / (len as i64 - 1).max(1) as f64;
    let se = (var / len.max(1) as f64).sqrt();
    let ta: Vec<f64> = good.iter().flat_map(|o| o.t_a.iter().copied()).collect();
    let tb: Vec<f64> = good.iter().flat_map(|o| o.t_b.iter().copied()).collect();
    let mean_ms = |t: &[f64]| py::round_n(1000.0 * py::sum(t.iter().copied()) / t.len().max(1) as f64, 1);
    let ng = good.len().max(1) as f64;
    let (pp, games) = pair_rows(&good);
    let mut res = crate::pydict! {
        "label" => format!("{va} contre {vb} · {mode}"), "A" => va, "B" => vb, "mode" => mode,
        "pairs" => good.len() as i64, "seed0" => seed0, "score_A" => py::round_n(m, 4), "se" => py::round_n(se, 4),
        "z" => if se > 0.0 { Some(py::round_n((m - 0.5) / se, 2)) } else { None },
        "A_as_player0" => py::round_n(py::sum(good.iter().map(|o| o.a1)) / ng, 4),
        "A_as_player1" => py::round_n(py::sum(good.iter().map(|o| o.a2)) / ng, 4),
        "ms_per_decision" => crate::pydict! {
            "A_mean" => mean_ms(&ta), "A_p95" => py::round_n(1000.0 * pct(&ta, 0.95), 1),
            "B_mean" => mean_ms(&tb), "B_p95" => py::round_n(1000.0 * pct(&tb, 0.95), 1),
            "A_n" => ta.len() as i64, "B_n" => tb.len() as i64,
        },
        "turns_mean" => turns_mean(&good), "errors" => errs.len() as i64, "err" => errs.first().cloned(),
        "seconds" => py::round0(t0.elapsed().as_secs_f64()),
        "per_pair" => pp, "games" => games,
    };
    super::version::stamp(&mut res);
    res
}

/// Ajoute un résultat à un fichier de résultats (liste JSON), comme
/// `done = json.load(open(f)) if f.exists() else []; done.append(r); json.dump(done, open(f, "w"), ...)`.
pub fn append_result(file: &std::path::Path, r: &PyVal, o: py::Dump) {
    let mut done = if file.exists() { py::load_file(file).unwrap_or_else(|e| panic!("{e}")) } else { PyVal::List(vec![]) };
    if let PyVal::List(v) = &mut done {
        v.push(r.clone());
    }
    py::dump_file(file, &done, o);
}

fn fz(v: Option<&PyVal>) -> String {
    v.map(|x| x.py_str()).unwrap_or_else(|| "None".into())
}

/// Ligne de ligne de commande (`__main__`).
pub fn main(a: &[String]) -> i32 {
    if a.len() < 5 {
        eprintln!("exp_search <paires> <graine0> <A> <B> <akali|miroir|defaut> [processus]");
        return 2;
    }
    let (n, seed0): (i64, i64) = (a[0].parse().unwrap(), a[1].parse().unwrap());
    let (va, vb, mode) = (&a[2], &a[3], &a[4]);
    let procs = a.get(5).map(|x| x.parse().unwrap()).unwrap_or(4);
    let r = run(n, seed0, va, vb, mode, procs);
    append_result(&here().join("results_search.json"), &r, py::Dump { indent: None, ensure_ascii: false });
    let g = |k: &str| r.get(k).unwrap();
    println!(
        "{}: A {} ± {:.3} (z={}) sur {} paires (graines {seed0}-{}), A joueur0 {:.2} / joueur1 {:.2}, ms/décision {}, \
         tours {}, erreurs {}, {}s",
        g("label").py_str(),
        py::pct(g("score_A").as_f64(), 1),
        g("se").as_f64(),
        fz(r.get("z")),
        g("pairs").py_str(),
        seed0 + n - 1,
        g("A_as_player0").as_f64(),
        g("A_as_player1").as_f64(),
        g("ms_per_decision").py_repr(),
        g("turns_mean").py_str(),
        g("errors").py_str(),
        g("seconds").py_str()
    );
    if let Some(PyVal::Str(e)) = r.get("err") {
        println!("{e}");
    }
    0
}
