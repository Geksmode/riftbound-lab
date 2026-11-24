//! IA des deux decks (`ai.py`). Traduction ligne à ligne : mêmes tirages, mêmes copies, mêmes calculs flottants.
//!
//! SearchAgent : pour chaque décision de haut niveau (action principale, priorité, focus), chaque action légale est
//! essayée sur des copies de la partie, jouée jusqu'à l'horizon par la politique de simulation, et notée par
//! `evaluate`. L'information cachée est respectée : dans chaque copie, la main et les cartes cachées adverses sont
//! retirées au hasard parmi les cartes non vues, et les deux decks mélangés (`determinize`).
//! Tous les choix faits pendant la résolution (cibles, « you may »...) suivent les heuristiques de `choose`.
//!
//! Différences de forme avec Python (sans effet sur les parties) :
//! - `choose` renvoie l'indice de l'option (Python : l'option elle-même ; même option, la première égale) ;
//! - `except Exception` de Python (coup qui ne se rejoue pas dans un monde simulé) : le moteur Rust n'a pas
//!   d'exception hors de la fin de partie ; un tel coup y serait une panique (jamais vue dans le différentiel).
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::actions::{loc_of, py_repr};
use crate::game::{Agent, Agents, Decision, Game, Res, Zone, O};
use crate::plans::Plan;
use crate::pyrandom::PyRandom;
use crate::spec::Spec;
use crate::value::{is_bf, Loc, Map, Value};

// ---------------------------------------------------------------------- environnement
/// `os.environ.get(name)` : None hors d'un environnement (WebAssembly) ou si la variable est absente.
pub fn env_var(name: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(name).ok()
    }
}

/// `os.environ.get(name, default)`
pub fn env_or(name: &str, default: &str) -> String {
    env_var(name).unwrap_or_else(|| default.to_string())
}

/// `float(texte)` de Python (espaces autour acceptés).
fn py_float(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or_else(|_| panic!("ValueError: could not convert string to float: {s:?}"))
}

pub const REACTIVE: [&str; 19] = [
    "Discipline",
    "Defy",
    "Not So Fast",
    "Back Off",
    "Block",
    "En Garde",
    "Ki Barrier",
    "Deathgrip",
    "Hidden Blade",
    "Stupefy",
    "Crumbling Sands",
    "Against the Odds",
    "Decree of Focus",
    "Sacrifice",
    "Long Sword",
    "Sterak's Gage",
    "Zhonya's Hourglass",
    "Vi, Peacekeeper",
    "Chakram Dancer",
];
// Tempo (2026-10-03) : à 6 points on est à portée de victoire, à 7 une tenue gagne. RB_TEMPO=0 rend l'ancienne IA.
pub const URGENT_SAMPLES: usize = 6;
// Choix pendant la résolution par simulation courte (2026-10-28, ask_sim) : désactivé par défaut (règle 9 : rejeté).
pub const ASK_SIM_MAX: usize = 12;
pub const ASK_SIM_SKIP: [&str; 3] = ["pay_gold", "pay_rune", "recycle_rune"];
// Cibles et timing des sorts (2026-10-28, spell_pre) : désactivé par défaut (règle 9 : on ne sait pas).
pub const SPELL_NOEFF: f64 = 0.25;
pub const DK_FODDER: [&str; 7] = [
    "Soaring Scout",
    "Honest Broker",
    "Watchful Sentry",
    "Black Rose Dignitary",
    "LeBlanc, Fragmented",
    "Lonely Poro",
    "Scuttle Crab",
];
pub const REACT_CANDS: usize = 6;

/// Poids de l'évaluation (`EV`, `EV_OLD` : dict Python ; `amb_hold` n'existe que si l'IA le pose).
#[derive(Clone, Debug, PartialEq)]
pub struct Ev {
    pub pts: f64,
    pub pts_hi: f64,
    pub bf: f64,
    pub fd: f64,
    pub hold_win: f64,
    pub unit0: f64,
    pub might: f64,
    pub cost: f64,
    pub on_bf: f64,
    pub card0: f64,
    pub card_e: f64,
    pub react: f64,
    pub react_kw: f64,
    pub rune: f64,
    pub leg_emp: f64,
    pub xp: f64,
    pub deck_low: f64,
    pub pts_ramp: f64,
    pub trick: f64,
    pub open: f64,
    pub amb_hold: Option<f64>,
}

impl Ev {
    /// `EV_OLD`
    pub fn old() -> Ev {
        Ev {
            pts: 7.0,
            pts_hi: 4.0,
            bf: 3.0,
            fd: 1.8,
            hold_win: 40.0,
            unit0: 1.0,
            might: 0.8,
            cost: 0.12,
            on_bf: 0.4,
            card0: 1.4,
            card_e: 0.05,
            react: 2.0,
            react_kw: 0.0,
            rune: 0.9,
            leg_emp: 2.0,
            xp: 0.15,
            deck_low: 3.0,
            pts_ramp: 0.0,
            trick: 2.0,
            open: 1.0,
            amb_hold: None,
        }
    }
    /// `EV` (poids du réglage Texel du 2026-10-20, avant RB_EV et RB_TRICK)
    pub fn tuned() -> Ev {
        Ev {
            pts: 6.859,
            pts_hi: 10.36,
            bf: 1.077,
            fd: 5.353,
            hold_win: 4.39,
            unit0: 5.444,
            might: 3.137,
            cost: 0.4435,
            on_bf: -4.586,
            card0: -0.234,
            card_e: 1.026,
            react: 1.041,
            rune: 6.602,
            xp: 1.103,
            trick: 2.371,
            ..Ev::old()
        }
    }
    /// clés de `EV` (ordre du dict Python)
    pub const KEYS: [&'static str; 20] = [
        "pts", "pts_hi", "bf", "fd", "hold_win", "unit0", "might", "cost", "on_bf", "card0", "card_e", "react",
        "react_kw", "rune", "leg_emp", "xp", "deck_low", "pts_ramp", "trick", "open",
    ];
    fn field(&mut self, k: &str) -> Option<&mut f64> {
        Some(match k {
            "pts" => &mut self.pts,
            "pts_hi" => &mut self.pts_hi,
            "bf" => &mut self.bf,
            "fd" => &mut self.fd,
            "hold_win" => &mut self.hold_win,
            "unit0" => &mut self.unit0,
            "might" => &mut self.might,
            "cost" => &mut self.cost,
            "on_bf" => &mut self.on_bf,
            "card0" => &mut self.card0,
            "card_e" => &mut self.card_e,
            "react" => &mut self.react,
            "react_kw" => &mut self.react_kw,
            "rune" => &mut self.rune,
            "leg_emp" => &mut self.leg_emp,
            "xp" => &mut self.xp,
            "deck_low" => &mut self.deck_low,
            "pts_ramp" => &mut self.pts_ramp,
            "trick" => &mut self.trick,
            "open" => &mut self.open,
            _ => return None,
        })
    }
    /// `w.get(k)`
    pub fn get(&self, k: &str) -> Option<f64> {
        if k == "amb_hold" {
            return self.amb_hold;
        }
        self.clone().field(k).map(|x| *x)
    }
    /// `w[k] = v` ; false si la clé n'existe pas
    pub fn set(&mut self, k: &str, v: f64) -> bool {
        if k == "amb_hold" {
            self.amb_hold = Some(v);
            return true;
        }
        match self.field(k) {
            Some(x) => {
                *x = v;
                true
            }
            None => false,
        }
    }
}

/// Réglages lus une fois dans l'environnement (variables globales de `ai.py`).
#[derive(Clone, Debug)]
pub struct Conf {
    pub tempo: bool,
    pub search: String,
    pub sh_extra: f64,
    pub turn_full: bool,
    pub ask_sim: bool,
    pub spell_pre: bool,
    pub pol_trick: bool,
    pub pol_react: bool,
    pub pol_ambush: f64,
    pub tie_react: bool,
    pub lazy_check: bool,
    /// `EV` (après RB_EV et RB_TRICK)
    pub ev: Ev,
    /// `EV_OLD`
    pub ev_old: Ev,
}

/// Variables globales de `ai.py` (TEMPO, SEARCH, EV...).
pub fn conf() -> &'static Conf {
    static C: OnceLock<Conf> = OnceLock::new();
    C.get_or_init(|| {
        let mut ev = Ev::tuned();
        if env_var("RB_EV").as_deref() == Some("old") {
            ev = Ev::old();
        }
        if env_or("RB_TRICK", "1") == "0" {
            ev.trick = 0.0;
            ev.open = 0.0;
        }
        let amb = env_or("RB_AMBUSH", "0");
        Conf {
            tempo: env_or("RB_TEMPO", "1") != "0",
            search: env_or("RB_SEARCH", "sh"),
            sh_extra: py_float(&env_or("RB_SH_EXTRA", "1.0")),
            turn_full: env_or("RB_TURN_FULL", "1") != "0",
            ask_sim: env_or("RB_ASK_SIM", "0") != "0",
            spell_pre: env_or("RB_SPELL_PRE", "0") != "0",
            pol_trick: env_or("RB_TRICK", "1") != "0",
            pol_react: env_or("RB_REACT", "1") != "0",
            pol_ambush: if amb.is_empty() { 0.0 } else { py_float(&amb) },
            tie_react: env_or("RB_TIE_REACT", "0") != "0",
            lazy_check: env_var("RB_LAZY_CHECK").as_deref() == Some("1"),
            ev,
            ev_old: Ev::old(),
        }
    })
}

/// `EV`
pub fn ev() -> &'static Ev {
    &conf().ev
}

/// Valeur d'une entrée de `cfg` (dict Python : nombre, texte ou poids `ev`).
#[derive(Clone, Debug, PartialEq)]
pub enum CfgV {
    Num(f64),
    Str(String),
    Ev(Vec<(String, f64)>),
}

impl CfgV {
    fn truthy(&self) -> bool {
        match self {
            CfgV::Num(x) => *x != 0.0,
            CfgV::Str(s) => !s.is_empty(),
            CfgV::Ev(v) => !v.is_empty(),
        }
    }
}

/// `cfg` de SearchAgent (dict Python, ordre d'insertion).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cfg(pub Vec<(String, CfgV)>);

impl Cfg {
    pub fn new() -> Cfg {
        Cfg(vec![])
    }
    /// `cfg[k] = v`
    pub fn set(&mut self, k: &str, v: CfgV) {
        match self.0.iter_mut().find(|(x, _)| x == k) {
            Some(e) => e.1 = v,
            None => self.0.push((k.to_string(), v)),
        }
    }
    pub fn get(&self, k: &str) -> Option<&CfgV> {
        self.0.iter().find(|(x, _)| x == k).map(|(_, v)| v)
    }
    /// `cfg.get(k, default)` lu comme un nombre
    pub fn num(&self, k: &str, default: f64) -> f64 {
        match self.get(k) {
            None => default,
            Some(CfgV::Num(x)) => *x,
            Some(CfgV::Str(s)) => py_float(s),
            Some(CfgV::Ev(_)) => panic!("cfg[{k}] n'est pas un nombre"),
        }
    }
    /// `bool(cfg.get(k, default))`
    pub fn truthy(&self, k: &str, default: bool) -> bool {
        match self.get(k) {
            None => default,
            Some(v) => v.truthy(),
        }
    }
    /// `cfg.get(k, default)` lu comme un texte
    pub fn text(&self, k: &str, default: &str) -> String {
        match self.get(k) {
            None => default.to_string(),
            Some(CfgV::Str(s)) => s.clone(),
            Some(v) => format!("{v:?}"),
        }
    }
}

// ---------------------------------------------------------------------- outils
/// `max(xs, key=...)` de Python : le premier élément de clé maximale.
fn first_max<K: PartialOrd>(keys: &[K]) -> usize {
    let mut b = 0;
    for i in 1..keys.len() {
        if keys[i] > keys[b] {
            b = i;
        }
    }
    b
}

/// `min(xs, key=...)` de Python : le premier élément de clé minimale.
fn first_min<K: PartialOrd>(keys: &[K]) -> usize {
    let mut b = 0;
    for i in 1..keys.len() {
        if keys[i] < keys[b] {
            b = i;
        }
    }
    b
}

/// Indice de la première option égale à v (Python renvoie l'option elle-même).
fn idx_of(options: &[Value], v: &Value) -> usize {
    options
        .iter()
        .position(|o| o == v)
        .unwrap_or_else(|| panic!("choix hors des options : {v:?} pas dans {options:?}"))
}

fn tup(xs: &[&str]) -> Value {
    Value::Tuple(xs.iter().map(|&x| x.into()).collect())
}

fn kind_of(a: &Value) -> &str {
    a.list()[0].str()
}

/// `sum(xs)` de Python sur des flottants : depuis Python 3.12, `sum` compense les erreurs d'arrondi (Neumaier) ; une
/// somme naïve diffère parfois d'un ulp, ce qui change les égalités exactes entre options et les arrondis des notes.
pub fn fsum(xs: impl IntoIterator<Item = f64>) -> f64 {
    let mut it = xs.into_iter();
    let Some(first) = it.next() else { return 0.0 };
    let mut f = 0.0 + first;
    let mut c = 0.0f64;
    for x in it {
        let t = f + x;
        if f.abs() >= x.abs() {
            c += (f - t) + x;
        } else {
            c += (x - t) + f;
        }
        f = t;
    }
    if c != 0.0 && c.is_finite() {
        f += c;
    }
    f
}

/// `(Obj._n, Item._n)`
pub fn counters(g: &Game) -> (u32, u32) {
    (g.counters.obj.load(Ordering::Relaxed), g.counters.item.load(Ordering::Relaxed))
}

/// `Obj._n, Item._n = n`
pub fn set_counters(g: &Game, n: (u32, u32)) {
    g.counters.obj.store(n.0, Ordering::Relaxed);
    g.counters.item.store(n.1, Ordering::Relaxed);
}

fn is_unit_spec(g: &Game, o: O) -> bool {
    g.spec(o).type_ == "Unit"
}

fn kw(sp: &Spec, k: &str) -> bool {
    sp.keywords.iter().any(|x| x == k)
}

// ---------------------------------------------------------------------- évaluation
/// `lasting_might(g, o)` : Might sans les effets « this turn » (ils expirent avant le prochain tour).
pub fn lasting_might(g: &mut Game, o: O) -> Res<i64> {
    let mut m = g.might(o)?;
    m -= g.o(o).mods.iter().filter(|(_, d)| d.opt_str() == Some("turn")).map(|(a, _)| *a).sum::<i64>();
    Ok(m)
}

/// `holds_ambush(g, pid, spec)` : la politique garde cette carte : unité [Ambush] et pid a une unité sur un
/// battlefield (voir POL_AMBUSH).
pub fn holds_ambush(g: &Game, pid: usize, spec: &Spec) -> bool {
    if spec.type_ != "Unit" || !kw(spec, "Ambush") {
        return false;
    }
    // Rengar, Trophy Hunter : « I can be played to a battlefield where there are enemy units »
    let enemy_ok = spec.text.contains("where there are enemy units");
    g.board.iter().any(|&o| {
        let ob = g.o(o);
        is_unit_spec(g, o) && is_bf(ob.loc) && (ob.ctrl == pid || enemy_ok && ob.ctrl != pid)
    })
}

/// `open_count(g, pid)` : nombre de sorts [Action]/[Reaction] de la main que les runes PRÊTES de pid peuvent payer,
/// les moins chers d'abord. Un sort de combat compte seulement si pid a une unité sur le plateau.
pub fn open_count(g: &Game, pid: usize) -> i64 {
    let pl = &g.p[pid];
    let mut ready = pl.runes.iter().filter(|&&r| !g.r(r).exhausted).count() as i64;
    let units = g.board.iter().any(|&o| g.o(o).ctrl == pid && is_unit_spec(g, o));
    let mut costs: Vec<i64> = pl
        .hand
        .iter()
        .filter(|&&c| {
            let sp = g.spec(c);
            sp.type_ == "Spell" && (kw(sp, "Action") || kw(sp, "Reaction")) && (units || !is_trick(sp))
        })
        .map(|&c| (g.spec(c).e + g.spec(c).p) as i64)
        .collect();
    costs.sort();
    let mut n = 0;
    for k in costs {
        if k > ready {
            break;
        }
        ready -= k;
        n += 1;
    }
    n
}

/// `is_trick(spec)` : sort [Action]/[Reaction] dont le texte change la Might « this turn ».
pub fn is_trick(spec: &Spec) -> bool {
    spec.type_ == "Spell"
        && (kw(spec, "Action") || kw(spec, "Reaction"))
        && spec.text.to_lowercase().contains("might this turn")
}

/// `point_value(p, victory, w)`
pub fn point_value(p: i64, victory: i64, w: &Ev) -> f64 {
    let old = w.pts * p as f64 + w.pts_hi * (0.max(p - (victory - 3))) as f64;
    let r = w.pts_ramp;
    if r == 0.0 || victory < 3 {
        return old;
    }
    let top = victory - 1;
    let span = victory - 2;
    let ssum = |q: i64| {
        let mut t = 0.0;
        for k in 1..=q {
            t += 1.0 + r * (k - 1) as f64 / span as f64;
        }
        t
    };
    let c = (w.pts * top as f64 + w.pts_hi * 2.0) / ssum(top);
    let q = p.min(top);
    c * ssum(q) + if p > top { old - (w.pts * top as f64 + w.pts_hi * 2.0) } else { 0.0 }
}

/// `card_value(c, w)`
pub fn card_value(g: &Game, c: O, w: &Ev) -> f64 {
    let n = g.cname(c);
    if REACTIVE.contains(&n) {
        return w.react;
    }
    let sp = g.spec(c);
    if w.react_kw != 0.0 && kw(sp, "Reaction") {
        return w.react;
    }
    w.card0 + w.card_e * sp.e.min(8) as f64
}

/// `unit_eval(g, u, w)`
pub fn unit_eval(g: &mut Game, u: O, w: &Ev) -> Res<f64> {
    let m = 0.max(lasting_might(g, u)?);
    let sp = g.spec(u);
    let mut v = w.unit0 + w.might * m as f64 + w.cost * (sp.e as f64 + 1.5 * sp.p as f64);
    if g.o(u).token && g.cname(u) == "Reflection" || g.has_kw(u, "Temporary")? {
        v = 0.4 + 0.4 * m as f64;
    }
    if g.cname(u) == "Mech" {
        v = 0.8 + 0.8 * m as f64;
    }
    if g.cname(u) == "Karthus, Eternal" {
        v += 2.5;
    }
    if g.o(u).empowered {
        v += 0.5;
    }
    if is_bf(g.o(u).loc) {
        v += w.on_bf;
    }
    Ok(v)
}

/// `GEAR_V.get(name)`
pub fn gear_v(name: &str) -> Option<f64> {
    Some(match name {
        "Zhonya's Hourglass" => 2.5,
        "Baited Hook" => 3.0,
        "Gold" => 0.9,
        "Long Sword" => 0.8,
        "Sterak's Gage" => 1.0,
        "Pendulum Blade" => 0.8,
        _ => return None,
    })
}

/// `danger(g, me)` : l'adversaire est à portée de victoire (6+ sur 8) : plus de long terme.
pub fn danger(g: &Game, me: usize) -> bool {
    g.p[1 - me].points >= g.victory - 2
}

/// `evaluate(g, me, w)` (w None : `EV`)
pub fn evaluate(g: &mut Game, me: usize, w: Option<&Ev>) -> f64 {
    evaluate_res(g, me, w).expect("GameOver pendant evaluate")
}

fn evaluate_res(g: &mut Game, me: usize, w: Option<&Ev>) -> Res<f64> {
    let cf = conf();
    let w = w.unwrap_or(&cf.ev);
    if let Some(win) = g.winner {
        let t = if cf.tempo { 25.0 * g.turn_no as f64 } else { 0.0 }; // gagner tôt, perdre le plus tard possible
        if win == me as i64 {
            return Ok(10000.0 - t);
        }
        if win == 1 - me as i64 {
            return Ok(-10000.0 + t);
        }
        return Ok(0.0);
    }
    let mut s = 0.0;
    for (pid, sign) in [(me, 1.0), (1 - me, -1.0)] {
        let mut v = point_value(g.p[pid].points, g.victory, w);
        let mut held = 0;
        for b in &g.bfs {
            if b.ctrl == Some(pid) {
                v += w.bf;
                held += 1;
                if let Some(fd) = b.facedown() {
                    if g.o(fd).owner == pid {
                        v += w.fd;
                    }
                }
            }
        }
        if cf.tempo && held > 0 && g.p[pid].points >= g.victory - 1 {
            v += w.hold_win; // tenue gagnante au début de son prochain tour
        }
        for o in g.board.clone() {
            if g.o(o).ctrl != pid {
                continue;
            }
            if is_unit_spec(g, o) {
                v += unit_eval(g, o, w)?;
            } else {
                v += gear_v(g.cname(o)).unwrap_or(1.0);
            }
        }
        // sum(card_value(c, w) for c in pl.hand) : `sum` de Python (3.12+) compense les erreurs d'arrondi
        let hs = fsum(g.p[pid].hand.iter().map(|&c| card_value(g, c, w)));
        v += hs;
        if w.trick != 0.0 && g.board.iter().any(|&o| g.o(o).ctrl == pid && is_unit_spec(g, o)) {
            let nr = g.p[pid].runes.len() as i64;
            let n = g.p[pid]
                .hand
                .iter()
                .filter(|&&c| is_trick(g.spec(c)) && (g.spec(c).e + g.spec(c).p) as i64 <= nr)
                .count();
            v += w.trick * n as f64;
        }
        if let Some(ah) = w.amb_hold.filter(|&x| x != 0.0) {
            // (2026-10-28, voir POL_AMBUSH) une unité [Ambush] gardée en main là où elle peut servir, et payable
            let nr = g.p[pid].runes.len() as i64;
            for &c in &g.p[pid].hand {
                let sp = g.spec(c);
                if (sp.e + sp.p) as i64 <= nr && holds_ambush(g, pid, sp) {
                    v += ah * (w.unit0 + w.might * sp.might.unwrap_or(0) as f64 + w.cost * (sp.e as f64 + 1.5 * sp.p as f64));
                }
            }
        }
        v += w.rune * g.p[pid].runes.len() as f64;
        if g.o(g.p[pid].legend).empowered {
            v += w.leg_emp;
        }
        v += w.xp * g.p[pid].xp as f64;
        if g.p[pid].deck.len() < 3 {
            v -= w.deck_low;
        }
        s += sign * v;
    }
    Ok(s)
}

// ---------------------------------------------------------------------- choix heuristiques
/// `Heuristics.mulligan(g, pid)` (cfg : `s.cfg`)
pub fn heur_mulligan(cfg: &Cfg, g: &mut Game, pid: usize) -> Res<Vec<O>> {
    let mode = cfg.text("mulligan", "normal");
    let h = g.p[pid].hand.clone();
    let ek = |g: &Game, c: O| -((g.spec(c).e + 2 * g.spec(c).p) as i64);
    if mode == "aggro" {
        // garder les unités de coût 4 ou moins et Shuriken Flip, renvoyer le reste (2 au plus)
        let keep: Vec<O> = h
            .iter()
            .copied()
            .filter(|&c| (g.spec(c).type_ == "Unit" && g.spec(c).e <= 4) || g.cname(c) == "Shuriken Flip")
            .collect();
        let mut rest: Vec<O> = h.iter().copied().filter(|c| !keep.contains(c)).collect();
        rest.sort_by_key(|&c| ek(g, c));
        rest.truncate(2);
        return Ok(rest);
    }
    if mode == "none" {
        return Ok(vec![]);
    }
    let mut out: Vec<O> = vec![];
    let cheap = h.iter().any(|&c| g.spec(c).type_ == "Unit" && g.spec(c).e <= 3);
    let mut by = h.clone();
    by.sort_by_key(|&c| ek(g, c));
    for c in by {
        if out.len() >= 2 {
            break;
        }
        if g.spec(c).e >= 6 {
            out.push(c);
        }
    }
    if !cheap && out.len() < 2 {
        let mut by = h.clone();
        by.sort_by_key(|&c| -(g.spec(c).e as i64));
        let rest: Vec<O> = by.into_iter().filter(|c| !out.contains(c) && g.spec(*c).e >= 4).collect();
        let k = 2 - out.len();
        out.extend(rest.into_iter().take(k));
    }
    Ok(out)
}

/// `cards.value(g, o)`
fn unit_value(g: &mut Game, o: O) -> Res<f64> {
    crate::cards_helpers::value(g, o)
}

/// `Heuristics.choose(g, pid, kind, options, ctx)` : indice de l'option choisie.
pub fn heur_choose(g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
    let objs = |options: &[Value]| -> Vec<O> { options.iter().map(|o| o.obj()).collect() };
    let by_unit_value = |g: &mut Game, xs: &[O]| -> Res<Vec<f64>> {
        let mut k = vec![];
        for &x in xs {
            k.push(unit_value(g, x)?);
        }
        Ok(k)
    };
    match kind {
        "may" => {
            let it = ctx.g("item");
            let name = if it.is_none() { String::new() } else { g.it(it.item()).name.clone() };
            let r = if name.starts_with("Star Spring") {
                false
            } else if name.starts_with("Dusk Rose Lab") {
                let mut any = false;
                for u in g.units(Some(pid), None) {
                    if g.o(u).ctrl == pid && (DK_FODDER.contains(&g.cname(u)) || g.has_kw(u, "Temporary")?) {
                        any = true;
                        break;
                    }
                }
                any
            } else {
                true
            };
            Ok(idx_of(options, &Value::Bool(r)))
        }
        "dusk_kill" => {
            let mut fod = vec![];
            for (i, u) in options.iter().enumerate() {
                if let Some(u) = u.opt_obj() {
                    if g.has_kw(u, "Temporary")? || DK_FODDER.contains(&g.cname(u)) {
                        fod.push((i, u));
                    }
                }
            }
            if fod.is_empty() {
                return Ok(idx_of(options, &Value::None));
            }
            let xs: Vec<O> = fod.iter().map(|x| x.1).collect();
            let k = by_unit_value(g, &xs)?;
            Ok(fod[first_min(&k)].0)
        }
        "target" => {
            let it = ctx.g("item");
            // préférer une unité adverse que l'effet tue, puis la plus précieuse
            let dmg = if it.is_none() { Value::Int(0) } else { g.it(it.item()).data.get("dmg").cloned().unwrap_or(Value::Int(0)) };
            let enemies: Vec<usize> = (0..options.len()).filter(|&i| g.o(options[i].obj()).ctrl != pid).collect();
            if dmg.truthy() {
                let mut kills = vec![];
                for &i in &enemies {
                    let o = options[i].obj();
                    let vg = match g.o(o).loc {
                        Some(Loc::Bf(b)) if g.bfs[b].name == "Void Gate" => 1,
                        _ => 0,
                    };
                    if g.might(o)? - g.o(o).damage <= dmg.int() + vg {
                        kills.push(i);
                    }
                }
                if !kills.is_empty() {
                    let xs: Vec<O> = kills.iter().map(|&i| options[i].obj()).collect();
                    let k = by_unit_value(g, &xs)?;
                    return Ok(kills[first_max(&k)]);
                }
            }
            if !enemies.is_empty() {
                let xs: Vec<O> = enemies.iter().map(|&i| options[i].obj()).collect();
                let k = by_unit_value(g, &xs)?;
                return Ok(enemies[first_max(&k)]);
            }
            Ok(0)
        }
        "zhonya_save" | "copy_target" => {
            let k = by_unit_value(g, &objs(options))?;
            Ok(first_max(&k))
        }
        "damage_order" => panic!("damage_order : jamais posée (une seule option)"),
        "hook_pick" | "mixologist_pick" | "herald_pick" => {
            Ok(options.iter().position(|o| !o.is_none()).unwrap_or_else(|| idx_of(options, &Value::None)))
        }
        "herald_dk_pick" => Ok(0),
        "sacrifice" => {
            let k = by_unit_value(g, &objs(options))?;
            Ok(first_min(&k))
        }
        "discard" => {
            let w = ev();
            let k: Vec<f64> = options
                .iter()
                .map(|c| {
                    let c = c.obj();
                    card_value(g, c, w) + 0.05 * g.spec(c).e as f64
                })
                .collect();
            Ok(first_min(&k))
        }
        "play_location" => Ok(options
            .iter()
            .position(|o| matches!(o, Value::Loc(Loc::Base)) || o.opt_str() == Some("base"))
            .unwrap_or(0)),
        "pay_gold" | "pay_rune" => Ok(0), // le paiement du planificateur (Gold gardé quand une rune peut payer)
        "recycle_rune" => {
            let mut cnt: BTreeMap<String, i64> = BTreeMap::new();
            for &r in &g.p[pid].runes {
                *cnt.entry(g.r(r).domain.clone()).or_default() += 1;
            }
            let mut ex: Vec<usize> = (0..options.len()).filter(|&i| g.r(options[i].rune()).exhausted).collect();
            if ex.is_empty() {
                ex = (0..options.len()).collect();
            }
            let k: Vec<i64> = ex.iter().map(|&i| cnt.get(&g.r(options[i].rune()).domain).copied().unwrap_or(0)).collect();
            Ok(ex[first_max(&k)])
        }
        "equip_target" => {
            let mut k = vec![];
            for o in options {
                let u = o.obj();
                k.push((is_bf(g.o(u).loc), g.might(u)?));
            }
            Ok(first_max(&k))
        }
        _ => Ok(0), // star_spring, ashe_pick et le reste : la première option
    }
}

/// `FastAgent` : politique de simulation qui passe et finit le tour, choix heuristiques.
#[derive(Clone, Default)]
pub struct FastAgent;

impl Agent for FastAgent {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        heur_choose(g, pid, kind, options, ctx)
    }
    fn decide(&mut self, _g: &mut Game, d: &Decision) -> Res<Value> {
        if d.kind == "main" {
            return Ok(tup(&["end"]));
        }
        Ok(tup(&["pass"]))
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// `determinize(g, me, rng)` : retire au hasard ce que `me` ne voit pas (main et cartes cachées adverses, ordre des
/// deux decks).
pub fn determinize(g: &mut Game, me: usize, rng: &mut PyRandom) {
    let opp = 1 - me;
    if g.p[opp].revealed_turn != g.turn_no {
        let hidden_fd: Vec<usize> = (0..g.bfs.len())
            .filter(|&b| g.bfs[b].facedown().is_some_and(|c| g.o(c).owner == opp))
            .collect();
        let mut pool: Vec<O> = g.p[opp].deck.iter().chain(g.p[opp].hand.iter()).copied().collect();
        rng.shuffle(&mut pool);
        let nh = g.p[opp].hand.len();
        let deck = pool.split_off(nh);
        g.p[opp].hand = pool;
        g.p[opp].deck = deck;
        for c in g.p[opp].hand.clone() {
            g.o_mut(c).zone = Some(Zone::Hand);
        }
        for c in g.p[opp].deck.clone() {
            g.o_mut(c).zone = Some(Zone::Deck);
        }
        for b in hidden_fd {
            let cands: Vec<O> =
                g.p[opp].deck.iter().copied().filter(|&c| g.impl_o(c).is_some_and(|im| im.hidden)).collect();
            if !cands.is_empty() && rng.random() < 0.85 {
                let new = *rng.choice(&cands);
                let old = g.bfs[b].facedown().unwrap();
                let pos = g.p[opp].deck.iter().position(|&x| x == new).unwrap();
                g.p[opp].deck.remove(pos);
                let (ht, hb) = (g.o(old).hidden_turn, g.o(old).hidden_bf);
                {
                    let n = g.o_mut(new);
                    n.zone = Some(Zone::Facedown);
                    n.hidden_turn = ht;
                    n.hidden_bf = hb;
                }
                {
                    let o = g.o_mut(old);
                    o.zone = Some(Zone::Deck);
                    o.hidden_turn = None;
                    o.hidden_bf = None;
                }
                g.p[opp].deck.push(old);
                g.bfs[b].set_facedown(Some(new));
            }
        }
        rng.shuffle(&mut g.p[opp].deck);
    } else {
        rng.shuffle(&mut g.p[opp].deck);
    }
    rng.shuffle(&mut g.p[me].deck);
}

/// Une entrée de `Game._asklog` : (type, nombre d'options, indice de la réponse, valeur simple).
pub type AskLog = (String, usize, Option<usize>, Option<Value>);

/// `_AskReplay` : rejeu d'ask_sim. Les asks d'avant le j-ième comme dans la partie réelle (`Game._asklog`), le j-ième
/// sur `alt`, ceux d'après par les règles (Heuristics). `seen` = signature (opt_sig) du j-ième. Le même objet sert aux
/// deux joueurs en Python : `k` et `seen` sont partagés entre les copies.
#[derive(Clone)]
pub struct AskReplay {
    pub log: Vec<AskLog>,
    pub j: usize,
    pub alt: usize,
    pub k: Arc<AtomicUsize>,
    pub seen: Arc<Mutex<Option<Vec<String>>>>,
}

impl Agent for AskReplay {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        let k = self.k.fetch_add(1, Ordering::Relaxed);
        if k == self.j {
            *self.seen.lock().unwrap() = Some(opt_sig(g, kind, options));
            return Ok(self.alt);
        }
        if k < self.log.len() {
            let (kd, n, i, v) = &self.log[k];
            if kd == kind && *n == options.len() {
                if let Some(i) = i {
                    return Ok(*i);
                }
                if let Some(v) = v {
                    return Ok(idx_of(options, v));
                }
            }
        }
        heur_choose(g, pid, kind, options, ctx)
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// `keep_played(c, opp, a)` : le coup rejoué par ask_sim est une carte jouée (ou cachée) par l'adversaire depuis sa
/// main : elle est publique, determinize ne doit pas l'avoir renvoyée dans son deck.
pub fn keep_played(c: &mut Game, opp: usize, a: &Value) {
    let k = kind_of(a);
    if k != "play" && k != "hide" {
        return;
    }
    let uid = a.list()[1].clone();
    if c.p[opp].hand.iter().any(|&x| Value::from(c.o(x).uid) == uid) {
        return;
    }
    let card = c.p[opp].deck.iter().copied().find(|&x| Value::from(c.o(x).uid) == uid);
    let Some(card) = card else { return };
    if c.p[opp].hand.is_empty() {
        return;
    }
    let i = c.p[opp].deck.iter().position(|&x| x == card).unwrap();
    let out = c.p[opp].hand.pop().unwrap();
    c.p[opp].deck[i] = out;
    c.o_mut(out).zone = Some(Zone::Deck);
    c.o_mut(card).zone = Some(Zone::Hand);
    c.p[opp].hand.push(card);
}

/// `opt_sig(kind, options)` : uid des objets et runes, repr du reste.
pub fn opt_sig(g: &Game, kind: &str, options: &[Value]) -> Vec<String> {
    let mut v = vec![kind.to_string()];
    for o in options {
        v.push(match o {
            Value::Obj(x) => format!("uid {}", g.o(*x).uid),
            Value::Rune(r) => format!("uid {}", g.r(*r).uid),
            _ => py_repr(g, o),
        });
    }
    v
}

/// `_imm(g, me, a, cnt, seed, determ, ev, fizzle)` : (note EV, note EV_OLD) de me après le coup a résolu, les deux
/// joueurs passant ; fizzle : le sort est contré sans effet. None si le coup ne se joue pas dans ce monde.
fn imm(g: &Game, me: usize, a: &Value, cnt: (u32, u32), seed: i128, determ: bool, ev: &Ev, fizzle: bool) -> Option<(f64, f64)> {
    set_counters(g, cnt);
    let mut c = g.clone_game();
    if determ {
        determinize(&mut c, me, &mut PyRandom::new(seed));
        c.rng = PyRandom::new(seed * 2 + 1);
    }
    c.agents = Agents(vec![Box::new(FastAgent), Box::new(FastAgent)]);
    c.apply(a);
    if fizzle {
        let uid = a.list()[1].clone();
        let it = c.chain.iter().copied().find(|&x| {
            let i = c.it(x);
            i.kind == "spell" && i.card.is_some_and(|cd| Value::from(c.o(cd).uid) == uid)
        });
        let it = it?;
        c.chain.retain(|&x| x != it);
        let (card, ctrl) = (c.it(it).card.unwrap(), c.it(it).ctrl);
        let _ = c.card_leaves_chain(card, "trash", Some(ctrl));
    }
    settle_pass(&mut c, 400);
    if let Some(w) = c.winner {
        let v = if w == me as i64 { 1e4 } else { -1e4 };
        return Some((v, v));
    }
    Some((evaluate(&mut c, me, Some(ev)), evaluate(&mut c, me, Some(&conf().ev_old))))
}

/// `spell_prefilter(g, me, opts, seed, determ, ev)` : options de me sans les variantes de sort qu'une résolution
/// immédiate juge moins bonnes (voir SPELL_PRE).
pub fn spell_prefilter(g: &mut Game, me: usize, opts: &[Value], seed: i128, determ: bool, ev: Option<&Ev>) -> Vec<Value> {
    let pol = PolicyAgent::new();
    let mut groups: Vec<(Value, Vec<Value>)> = vec![];
    for a in opts {
        if kind_of(a) != "play" {
            continue;
        }
        let c = pol.card_of(g, a);
        if c.is_some_and(|c| g.spec(c).type_ == "Spell") {
            let key = Value::Tuple(vec![a.list()[1].clone(), a.list()[2].clone()]);
            match groups.iter_mut().find(|x| x.0 == key) {
                Some(x) => x.1.push(a.clone()),
                None => groups.push((key, vec![a.clone()])),
            }
        }
    }
    let showdown = g.sd.is_some();
    if groups.is_empty() {
        return opts.to_vec();
    }
    let ev = ev.unwrap_or(&conf().ev).clone();
    let mut drop: Vec<String> = vec![];
    let cnt = counters(g);
    for (_, vs) in &groups {
        if vs.len() < 2 && !showdown {
            continue;
        }
        let vals: Vec<Option<(f64, f64)>> = vs.iter().map(|a| imm(g, me, a, cnt, seed, determ, &ev, false)).collect();
        if vals.iter().any(|v| v.is_none()) {
            continue;
        }
        let vals: Vec<(f64, f64)> = vals.into_iter().map(|v| v.unwrap()).collect();
        let k0: Vec<(f64, i64)> = (0..vs.len()).map(|i| (vals[i].0, -(i as i64))).collect();
        let k1: Vec<(f64, i64)> = (0..vs.len()).map(|i| (vals[i].1, -(i as i64))).collect();
        let mut keep: Vec<usize> = vec![first_max(&k0)];
        let b1 = first_max(&k1);
        if !keep.contains(&b1) {
            keep.push(b1);
        }
        if showdown {
            let mut useless = true;
            for &k in &keep {
                let fz = imm(g, me, &vs[k], cnt, seed, determ, &ev, true);
                match fz {
                    None => useless = false,
                    Some(fz) => {
                        if (vals[k].0 - fz.0).max(vals[k].1 - fz.1) > SPELL_NOEFF {
                            useless = false;
                        }
                    }
                }
            }
            if useless {
                keep.clear();
            }
        }
        for i in 0..vs.len() {
            if !keep.contains(&i) {
                drop.push(py_repr(g, &vs[i]));
            }
        }
    }
    set_counters(g, cnt);
    let out: Vec<Value> = opts.iter().filter(|a| !drop.contains(&py_repr(g, a))).cloned().collect();
    if out.is_empty() {
        opts.to_vec()
    } else {
        out
    }
}

/// `settle_pass(c, max_steps)` : les deux joueurs passent jusqu'à la prochaine décision « main » (chaîne résolue,
/// combat du showdown compris).
pub fn settle_pass(c: &mut Game, max_steps: usize) {
    let pass = tup(&["pass"]);
    for _ in 0..max_steps {
        let Some(d) = c.advance(false) else { return };
        if d.kind == "main" {
            return;
        }
        let opts = d.options.unwrap();
        let a = if opts.contains(&pass) { pass.clone() } else { opts[0].clone() };
        c.apply(&a);
    }
}

/// `rollout(g, max_steps)`
pub fn rollout(g: &mut Game, max_steps: usize) {
    for _ in 0..max_steps {
        let Some(d) = g.advance(false) else { return };
        if d.kind == "main" {
            return;
        }
        let mut fa = std::mem::replace(&mut g.agents[0], Box::new(FastAgent));
        let a = fa.decide(g, &d);
        g.agents[0] = fa;
        match a {
            Ok(a) => g.apply(&a),
            Err(_) => return,
        }
    }
}

/// Options de construction de SearchAgent (arguments nommés de Python ; None = valeur par défaut).
#[derive(Clone, Debug)]
pub struct SearchOpts {
    pub samples: usize,
    pub max_cands: usize,
    pub name: String,
    pub horizon: i64,
    pub cfg: Cfg,
    pub search: Option<String>,
    pub sh_extra: Option<f64>,
}

impl Default for SearchOpts {
    fn default() -> Self {
        SearchOpts { samples: 1, max_cands: 40, name: String::new(), horizon: 2, cfg: Cfg::new(), search: None, sh_extra: None }
    }
}

/// `SearchAgent` (et `plans.PlanAgent` quand `plan` est posé : mêmes simulations, seul le score change).
#[derive(Clone)]
pub struct SearchAgent {
    pub cfg: Arc<Cfg>,
    pub rng: PyRandom,
    pub samples: usize,
    pub max_cands: usize,
    pub name: String,
    pub horizon: i64,
    pub search: String,
    pub ev: Option<Ev>,
    pub sh_extra: f64,
    /// mode "turn" : ((tour, joueur), coups restants de la suite choisie)
    pub tplan: Option<((i64, usize), Vec<Value>)>,
    /// PlanAgent : plan du joueur (None : SearchAgent simple)
    pub plan: Option<Arc<dyn Plan>>,
    /// PlanAgent : plan prêté à l'adversaire dans les simulations
    pub opp_plan: Option<Arc<dyn Plan>>,
    /// niveaux 1 à 5 de la table (cfg `wild`) : générateur des coups pris au hasard, séparé de celui de la recherche
    pub wrng: PyRandom,
}

/// Résultat de `pick` : (meilleur coup, sa valeur, [(valeur, coup)]).
pub type Picked = (Value, f64, Vec<(f64, Value)>);

/// Nœud du faisceau de `turn_search`.
struct Node {
    seq: Vec<Value>,
    states: Vec<Game>,
    options: Vec<Value>,
    pri: f64,
}

impl SearchAgent {
    /// `SearchAgent(seed, samples, max_cands, name, horizon, cfg, search, sh_extra)`
    pub fn new(seed: i128, o: SearchOpts) -> SearchAgent {
        let cf = conf();
        let cfg = o.cfg;
        let mut ev: Option<Ev> = match cfg.get("ev") {
            Some(CfgV::Ev(kv)) if !kv.is_empty() => {
                let mut e = cf.ev.clone();
                for (k, v) in kv {
                    e.set(k, *v);
                }
                Some(e)
            }
            _ => None,
        };
        let amb = match cfg.get("ambush") {
            None => cf.pol_ambush,
            Some(v) => {
                if v.truthy() {
                    cfg.num("ambush", 0.0)
                } else {
                    0.0
                }
            }
        };
        if amb != 0.0 {
            let mut e = ev.unwrap_or_else(|| cf.ev.clone());
            e.amb_hold = Some(amb);
            ev = Some(e);
        }
        SearchAgent {
            cfg: Arc::new(cfg),
            rng: PyRandom::new(seed),
            samples: o.samples,
            max_cands: o.max_cands,
            name: o.name,
            horizon: o.horizon,
            search: o.search.filter(|s| !s.is_empty()).unwrap_or_else(|| cf.search.clone()),
            ev,
            sh_extra: o.sh_extra.unwrap_or(cf.sh_extra),
            tplan: None,
            plan: None,
            opp_plan: None,
            wrng: PyRandom::new(seed * 31 + 7),
        }
    }

    /// Niveaux faibles de la table (cfg `wild` = p) : avec la probabilité p, la décision est prise au hasard parmi
    /// les coups légaux (même procédé que `train::Gentle`, l'IA du tutoriel) : en « main », rarement « end » s'il y a
    /// autre chose ; hors « main », souvent « pass ». La suite prévue du tour (tplan) est alors abandonnée.
    fn wild_move(&mut self, d: &Decision) -> Option<Value> {
        let p = self.cfg.num("wild", 0.0);
        if p <= 0.0 || self.wrng.random() >= p {
            return None;
        }
        let mut opts: Vec<Value> = d.options.clone()?;
        if opts.len() < 2 {
            return None;
        }
        self.tplan = None;
        if d.kind == "main" && self.wrng.random() < 0.85 {
            opts.retain(|o| o.list()[0].str() != "end");
        }
        if d.kind != "main" && self.wrng.random() < 0.6 {
            return Some(Value::Tuple(vec!["pass".into()]));
        }
        Some(self.wrng.choice(&opts).clone())
    }

    fn ask_sim_on(&self) -> bool {
        self.cfg.truthy("ask_sim", conf().ask_sim)
    }

    /// `ask_sim(g, pid, kind, options, base)` : meilleure option du choix en cours par simulation courte (voir
    /// ASK_SIM) ; `base` (la règle) si pas de point de reprise, trop d'options, paiement, ou rejeu qui ne retombe pas
    /// sur ce choix. Un seul monde, le même pour toutes les options.
    pub fn ask_sim(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], base: usize) -> Res<usize> {
        let n = options.len();
        if g.ck.ckpt.is_none() || !g.ck.on || n < 2 || n > ASK_SIM_MAX || ASK_SIM_SKIP.contains(&kind) {
            return Ok(base);
        }
        let hi = base;
        let (log, j, sig) = (g.ck.asklog.clone(), g.ck.asklog.len(), opt_sig(g, kind, options));
        let w = self.rng.getrandbits(30) as i128;
        let n0 = counters(g);
        let mut vals: Vec<f64> = vec![];
        let r = (|| -> Option<()> {
            for alt in 0..n {
                let ck = g.ck.ckpt.as_ref().unwrap();
                let (pre, on, inn, a) = (&ck.0, ck.1, ck.2, ck.3.clone());
                set_counters(g, (on, inn));
                let mut c = pre.clone_game();
                let own: Vec<(u32, usize)> = c.p[pid].deck.iter().enumerate().map(|(k, &x)| (c.o(x).uid, k)).collect();
                determinize(&mut c, pid, &mut PyRandom::new(w));
                let nown = own.len();
                let key = |c: &Game, x: O| own.iter().find(|e| e.0 == c.o(x).uid).map(|e| e.1).unwrap_or(nown);
                let mut deck = std::mem::take(&mut c.p[pid].deck);
                deck.sort_by_key(|&x| key(&c, x));
                c.p[pid].deck = deck;
                keep_played(&mut c, 1 - pid, &a);
                c.rng = PyRandom::new(w * 2 + 1);
                let f = AskReplay {
                    log: log.clone(),
                    j,
                    alt,
                    k: Arc::new(AtomicUsize::new(0)),
                    seen: Arc::new(Mutex::new(None)),
                };
                c.agents = Agents(vec![Box::new(f.clone()), Box::new(f.clone())]);
                c.apply(&a);
                settle_pass(&mut c, 400);
                if f.seen.lock().unwrap().as_ref() != Some(&sig) {
                    return None;
                }
                let v = match c.winner {
                    None => self.value(&mut c, pid).ok()?,
                    Some(wn) => {
                        if wn == pid as i64 {
                            1e4
                        } else {
                            -1e4
                        }
                    }
                };
                vals.push(v);
            }
            Some(())
        })();
        set_counters(g, n0);
        if r.is_none() {
            return Ok(base);
        }
        let k: Vec<(f64, bool)> = (0..n).map(|i| (vals[i], i == hi)).collect();
        let b = first_max(&k);
        Ok(if vals[b] > vals[hi] + 1e-9 { b } else { base })
    }

    /// `risky_attack(g, me, a)` : attaque d'un battlefield où l'adversaire a des unités et de quoi poser une unité
    /// [Ambush].
    pub fn risky_attack(&self, g: &Game, me: usize, a: &Value) -> bool {
        let l = a.list();
        if l[0].str() != "move" {
            return false;
        }
        let dest = loc_of(&l[2]);
        if dest == Some(Loc::Base) {
            return false;
        }
        let opp = 1 - me;
        if g.units(Some(opp), dest).is_empty() {
            return false;
        }
        let ready = g.p[opp].runes.iter().filter(|&&r| !g.r(r).exhausted).count();
        ready >= 5 && !g.p[opp].hand.is_empty()
    }

    /// `policies(me)` : agents des simulations (PlanAgent : politiques du plan, `PlanPolicy`).
    pub fn policies(&self, me: usize) -> Vec<Box<dyn Agent>> {
        if self.plan.is_some() {
            let mut ag: Vec<Box<dyn Agent>> = vec![];
            for pid in 0..2 {
                let plan = if pid == me { self.plan.clone() } else { self.opp_plan.clone() };
                ag.push(Box::new(PolicyAgent { cfg: self.cfg.clone(), plan }));
            }
            return ag;
        }
        let pol = PolicyAgent { cfg: self.cfg.clone(), plan: None };
        vec![Box::new(pol.clone()), Box::new(pol)]
    }

    /// `value(c, me)` (PlanAgent : plus `plan.shape`)
    pub fn value(&self, c: &mut Game, me: usize) -> Res<f64> {
        let v = evaluate_res(c, me, self.ev.as_ref())?;
        match &self.plan {
            Some(p) => Ok(v + p.shape(c, me)?),
            None => Ok(v),
        }
    }

    /// `prior_of(g, me, a)` (PlanAgent : `plan.prior`, annulé s'il est négatif à portée de défaite)
    pub fn prior_of(&self, g: &mut Game, me: usize, a: &Value) -> Res<f64> {
        let Some(p) = &self.plan else { return Ok(0.0) };
        let mut pr = p.prior(g, me, a)?;
        if conf().tempo && pr < 0.0 && danger(g, me) {
            pr = 0.0; // à portée de défaite, les principes du plan ne tiennent plus
        }
        Ok(pr)
    }

    /// `one(g, me, a, rng, world)` : valeur d'un rollout. `world` (graine) : le monde caché et l'aléa du jeu sont
    /// fixés par la graine, et les compteurs d'identifiants remis à l'identique.
    pub fn one(&self, g: &Game, me: usize, a: &Value, rng: &mut PyRandom, world: Option<i128>) -> Res<f64> {
        let n0 = counters(g);
        let mut c = g.clone_game();
        determinize(&mut c, me, rng);
        if let Some(w) = world {
            c.rng = PyRandom::new(w * 2 + 1);
        }
        c.agents = Agents(self.policies(me));
        c.apply(a);
        rollout_policy(&mut c, self.horizon, 600, Some(&self.cfg));
        let r = self.value(&mut c, me);
        if world.is_some() {
            set_counters(g, n0);
        }
        r
    }

    /// `score(g, me, a)` : ancienne recherche, s.samples mondes tirés avec s.rng.
    pub fn score(&mut self, g: &Game, me: usize, a: &Value) -> Res<f64> {
        let mut tot = 0.0;
        for _ in 0..self.samples {
            let mut rng = std::mem::replace(&mut self.rng, PyRandom::new(0));
            let r = self.one(g, me, a, &mut rng, None);
            self.rng = rng;
            tot += r?;
        }
        let mut gg = g.clone();
        Ok(tot / self.samples as f64 + self.prior_of(&mut gg, me, a)?)
    }

    /// `tie_break(g, d, opts, picked)` : avec tie_react, une réaction dont toutes les options ont la même note suit la
    /// politique (react_best).
    pub fn tie_break(&mut self, g: &mut Game, d: &Decision, opts: &[Value], picked: Picked) -> Res<Value> {
        let (best, _, scored) = picked;
        if d.kind != "main" && self.cfg.truthy("tie_react", conf().tie_react) {
            let vals: Vec<f64> = scored.iter().map(|x| x.0).collect();
            if vals.len() == opts.len() && opts.len() > 1 {
                let mx = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let mn = vals.iter().cloned().fold(f64::INFINITY, f64::min);
                if mx - mn < 1e-6 {
                    let pol = PolicyAgent { cfg: self.cfg.clone(), plan: None };
                    let a = pol.decide_pol(g, d)?;
                    if opts.contains(&a) {
                        return Ok(a);
                    }
                }
            }
        }
        Ok(best)
    }

    /// `urgent(g, d, me, best_v)`
    pub fn urgent(&self, g: &Game, d: &Decision, me: usize, best_v: f64) -> bool {
        conf().tempo && d.kind == "main" && self.samples < URGENT_SAMPLES && (best_v < -5000.0 || danger(g, me))
    }

    /// `pick(g, d, opts)` : meilleure option et scores [(v, a)] ; en danger ou si tout perd, on refait avec plus de
    /// tirages.
    pub fn pick(&mut self, g: &mut Game, d: &Decision, opts: &[Value]) -> Res<Picked> {
        if self.search == "turn" && d.kind == "main" {
            return self.pick_turn(g, d, opts);
        }
        if self.search != "old" {
            return self.pick_crn(g, d, opts);
        }
        let me = d.player;
        let mut scored = vec![];
        for a in opts {
            scored.push((self.score(g, me, a)?, a.clone()));
        }
        let best_v = scored.iter().map(|x| x.0).fold(f64::NEG_INFINITY, f64::max);
        if self.urgent(g, d, me, best_v) {
            // gagner à tout prix : un seul tirage dit « tout perd » au hasard ; on mesure la chance de survie
            let n0 = self.samples;
            self.samples = URGENT_SAMPLES;
            let mut sc = vec![];
            let mut err = None;
            for a in opts {
                match self.score(g, me, a) {
                    Ok(v) => sc.push((v, a.clone())),
                    Err(e) => {
                        err = Some(e);
                        break;
                    }
                }
            }
            self.samples = n0;
            if let Some(e) = err {
                return Err(e);
            }
            scored = sc;
        }
        let (mut best, mut best_v) = (Value::None, -1e18);
        for (v, a) in &scored {
            if *v > best_v + 1e-9 {
                best = a.clone();
                best_v = *v;
            }
        }
        Ok((best, best_v, scored))
    }

    /// `pick_crn(g, d, opts)` : tirages communs ; mode "sh" : élimination en plusieurs passes.
    pub fn pick_crn(&mut self, g: &mut Game, d: &Decision, opts: &[Value]) -> Res<Picked> {
        let me = d.player;
        let n = opts.len();
        let mut worlds: Vec<i128> = vec![];
        let mut tot = vec![0.0f64; n];
        let mut cnt = vec![0usize; n];
        let mut pri = vec![];
        for a in opts {
            pri.push(self.prior_of(g, me, a)?);
        }
        let gr: &Game = g;

        let run = |s: &mut SearchAgent, i: usize, k: usize, worlds: &mut Vec<i128>, tot: &mut [f64], cnt: &mut [usize]| -> Res<()> {
            while worlds.len() < k {
                worlds.push(s.rng.getrandbits(30) as i128);
            }
            for j in cnt[i]..k {
                tot[i] += s.one(gr, me, &opts[i], &mut PyRandom::new(worlds[j]), Some(worlds[j]))?;
            }
            cnt[i] = cnt[i].max(k);
            Ok(())
        };
        let val = |i: usize, tot: &[f64], cnt: &[usize]| tot[i] / cnt[i] as f64 + pri[i];
        let ranked = |idx: &[usize], tot: &[f64], cnt: &[usize]| {
            let mut v: Vec<usize> = idx.to_vec();
            v.sort_by(|&a, &b| {
                (-val(a, tot, cnt)).partial_cmp(&-val(b, tot, cnt)).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(&b))
            });
            v
        };

        let mut k0 = self.samples;
        for i in 0..n {
            run(self, i, k0, &mut worlds, &mut tot, &mut cnt)?;
        }
        let mx = (0..n).map(|i| val(i, &tot, &cnt)).fold(f64::NEG_INFINITY, f64::max);
        if self.urgent(gr, d, me, mx) {
            k0 = URGENT_SAMPLES; // mêmes premiers mondes, on complète jusqu'à 6
            for i in 0..n {
                run(self, i, k0, &mut worlds, &mut tot, &mut cnt)?;
            }
        }
        let mut surv = ranked(&(0..n).collect::<Vec<_>>(), &tot, &cnt);
        if (self.search == "sh" || self.search == "turn") && n > 1 {
            let mut sizes = vec![];
            let mut m = n;
            while m > 2 {
                m = 2.max(m / 2);
                sizes.push(m);
            }
            if sizes.is_empty() {
                sizes.push(2);
            }
            let budget = (self.sh_extra * n as f64 * self.samples as f64) as i64;
            let per = budget.div_euclid(sizes.len() as i64);
            let mut carry = budget.rem_euclid(sizes.len() as i64);
            let mut k = k0;
            for &m in &sizes {
                surv.truncate(m);
                carry += per;
                let add = carry.div_euclid(m as i64);
                if add != 0 {
                    k = (k as i64 + add) as usize;
                    carry -= add * m as i64;
                    for &i in &surv.clone() {
                        run(self, i, k, &mut worlds, &mut tot, &mut cnt)?;
                    }
                }
                surv = ranked(&surv, &tot, &cnt);
            }
        }
        let scored: Vec<(f64, Value)> = (0..n).map(|i| (val(i, &tot, &cnt), opts[i].clone())).collect();
        let b = surv[0];
        Ok((opts[b].clone(), val(b, &tot, &cnt), scored))
    }

    /// `pick_turn(g, d, opts)` : suite de coups du tour (jusqu'à « end ») au lieu du seul premier coup ; la suite
    /// complète va dans s.tplan. En danger (ou si tout perd), on refait avec URGENT_SAMPLES mondes.
    pub fn pick_turn(&mut self, g: &mut Game, d: &Decision, opts: &[Value]) -> Res<Picked> {
        let res = self.pick_turn_inner(g, d, opts)?;
        if crate::probe::active() {
            // sonde de combats_parties (hors jeu)
            let rp = |g: &Game, v: &Value| crate::outils::empreinte::repr_value(g, v);
            let plan: Vec<String> = self.tplan.as_ref().map(|t| t.1.iter().map(|x| rp(g, x)).collect()).unwrap_or_default();
            crate::probe::on_pick_turn(g.turn_no, rp(g, &res.0), res.1, plan);
        }
        Ok(res)
    }

    fn pick_turn_inner(&mut self, g: &mut Game, d: &Decision, opts: &[Value]) -> Res<Picked> {
        let me = d.player;
        let nw = self.samples.max(URGENT_SAMPLES);
        let worlds: Vec<i128> = (0..nw).map(|_| self.rng.getrandbits(30) as i128).collect();
        let n0 = counters(g);
        let r = (|| {
            let (mut seq, mut v, mut scored) = self.turn_search(g, me, opts, &worlds[..self.samples])?;
            if self.urgent(g, d, me, v) && self.samples < URGENT_SAMPLES {
                (seq, v, scored) = self.turn_search(g, me, opts, &worlds[..URGENT_SAMPLES])?;
            }
            Ok((seq, v, scored))
        })();
        set_counters(g, n0);
        let (seq, v, scored) = r?;
        self.tplan = Some(((g.turn_no, me), seq[1..].to_vec()));
        Ok((seq[0].clone(), v, scored))
    }

    /// `turn_search(g, me, opts, worlds)` : faisceau (largeur turn_w, profondeur turn_d) sur les suites de coups de
    /// `me`, chaque suite jouée dans les MÊMES mondes déterminisés ; « end » ferme la suite.
    pub fn turn_search(&mut self, g: &Game, me: usize, opts: &[Value], worlds: &[i128]) -> Res<(Vec<Value>, f64, Vec<(f64, Value)>)> {
        let wid = self.cfg.num("turn_w", 3.0) as usize;
        let depth = self.cfg.num("turn_d", 5.0) as usize;
        let pol = PolicyAgent { cfg: self.cfg.clone(), plan: None };
        let ags = self.policies(me);
        let set_ags = |c: &mut Game| c.agents = Agents(ags.iter().map(|a| a.clone_box()).collect());
        let mut roots = vec![];
        for &w in worlds {
            let mut c = g.clone_game();
            set_ags(&mut c);
            determinize(&mut c, me, &mut PyRandom::new(w));
            c.rng = PyRandom::new(w * 2 + 1);
            roots.push(c);
        }

        // avance jusqu'à la prochaine décision « main » de me (fenêtres de réaction : politique de simulation)
        let settle = |c: &mut Game| -> Res<Option<Decision>> {
            for _ in 0..300 {
                let dd = c.advance(true);
                match dd {
                    None => return Ok(None),
                    Some(dd) => {
                        if dd.kind == "main" && dd.player == me {
                            return Ok(Some(dd));
                        }
                        let a = pol.decide_pol(c, &dd)?;
                        c.apply(&a);
                    }
                }
            }
            Ok(None)
        };

        let w_open = self.ev.as_ref().unwrap_or(&conf().ev).open;
        let end = tup(&["end"]);
        let value = |s: &SearchAgent, st: &Game, a: &Value| -> Res<f64> {
            let bonus = if *a == end && w_open != 0.0 {
                // fin du tour : bonus des runes laissées prêtes pour les sorts [Action]/[Reaction] en main
                w_open * open_count(st, me) as f64
            } else {
                0.0
            };
            let mut c = st.clone_game();
            set_ags(&mut c);
            c.apply(a);
            if kind_of(a) != "end" {
                settle(&mut c)?;
            }
            rollout_policy(&mut c, s.horizon, 600, Some(&s.cfg));
            Ok(s.value(&mut c, me)? + bonus)
        };
        let advance = |st: &Game, a: &Value| -> Res<(Game, Option<Decision>)> {
            let mut c = st.clone_game();
            set_ags(&mut c);
            c.apply(a);
            let mut dd = settle(&mut c)?;
            if let Some(d) = dd.as_mut() {
                d.options = Some(c.options_main(me)?);
            }
            Ok((c, dd))
        };

        let spell_pre = self.cfg.truthy("spell_pre", conf().spell_pre);
        let mut beam = vec![Node { seq: vec![], states: roots, options: opts.to_vec(), pri: 0.0 }];
        let mut done: Vec<(f64, Vec<Value>, bool)> = vec![]; // [valeur, suite, complète]
        for lvl in 0..depth {
            // (v, rang, nœud, coup, a priori, indice dans done)
            let mut kids: Vec<(f64, usize, usize, Value, f64, usize)> = vec![];
            for (ni, nd) in beam.iter_mut().enumerate() {
                let mut acts = nd.options.clone();
                if lvl > 0 && spell_pre {
                    // coups suivants de la suite : même pré-sélection, dans le premier monde du nœud (déjà tiré)
                    acts = spell_prefilter(&mut nd.states[0], me, &acts, 0, false, self.ev.as_ref());
                }
                if lvl > 0 {
                    acts.truncate(self.max_cands);
                }
                for a in acts {
                    let pri = nd.pri + self.prior_of(&mut nd.states[0], me, &a)?;
                    let mut vals = Vec::with_capacity(nd.states.len());
                    for st in &nd.states {
                        vals.push(value(self, st, &a)?);
                    }
                    let v = fsum(vals.iter().copied()) / nd.states.len() as f64 + pri;
                    let mut seq = nd.seq.clone();
                    seq.push(a.clone());
                    done.push((v, seq, a == end || lvl == depth - 1));
                    if a != end {
                        kids.push((v, kids.len(), ni, a, pri, done.len() - 1));
                    }
                }
            }
            kids.sort_by(|x, y| (-x.0).partial_cmp(&-y.0).unwrap_or(std::cmp::Ordering::Equal).then(x.1.cmp(&y.1)));
            let mut nbeam = vec![];
            for (_, _, ni, a, pri, di) in kids.into_iter().take(wid) {
                let nd = &beam[ni];
                let mut nxt = vec![];
                for st in &nd.states {
                    nxt.push(advance(st, &a)?);
                }
                if nxt.iter().any(|(_, dd)| dd.is_none()) {
                    done[di].2 = true;
                    continue; // partie finie dans un des mondes : valeur déjà comptée
                }
                let o0 = nxt[0].1.as_ref().unwrap().options.as_ref().unwrap();
                let common: Vec<Value> = o0
                    .iter()
                    .filter(|x| nxt.iter().all(|(_, dd)| dd.as_ref().unwrap().options.as_ref().unwrap().contains(x)))
                    .cloned()
                    .collect();
                if common.contains(&end) {
                    let mut seq = nd.seq.clone();
                    seq.push(a);
                    nbeam.push(Node { seq, states: nxt.into_iter().map(|(c, _)| c).collect(), options: common, pri });
                } else {
                    done[di].2 = true; // (pas de suite commune possible : on la garde telle quelle)
                }
            }
            beam = nbeam;
            if beam.is_empty() {
                break;
            }
        }
        // Suites retenues (2026-10-10) : seulement les suites complètes (« end », partie finie, profondeur atteinte).
        let full = self.cfg.truthy("turn_full", conf().turn_full) && done.iter().any(|x| x.2);
        let (mut best_v, mut best) = (-1e18, None);
        let mut first: Vec<(String, (f64, Value))> = vec![];
        let mut part: Vec<(String, (f64, Value))> = vec![];
        fn upd(m: &mut Vec<(String, (f64, Value))>, k: String, v: f64, a: &Value) {
            match m.iter_mut().find(|x| x.0 == k) {
                Some(e) => {
                    if v > e.1 .0 {
                        e.1 = (v, a.clone());
                    }
                }
                None => m.push((k, (v, a.clone()))),
            }
        }
        for (v, seq, ok) in &done {
            if full && !ok {
                upd(&mut part, py_repr(g, &seq[0]), *v, &seq[0]);
                continue;
            }
            if *v > best_v + 1e-9 {
                best_v = *v;
                best = Some(seq.clone());
            }
            upd(&mut first, py_repr(g, &seq[0]), *v, &seq[0]);
        }
        let mut scored = vec![];
        for a in opts {
            let k = py_repr(g, a);
            if let Some(e) = first.iter().find(|x| x.0 == k) {
                scored.push(e.1.clone());
            } else if let Some(e) = part.iter().find(|x| x.0 == k) {
                scored.push(e.1.clone());
            }
        }
        Ok((best.expect("turn_search : aucune suite"), best_v, scored))
    }

    /// `decide(g, d)` de SearchAgent / PlanAgent.
    fn decide_search(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        let all = d.options.as_ref().expect("décision sans options");
        if all.len() == 1 {
            return Ok(all[0].clone());
        }
        let mut opts = all.clone();
        let me = d.player;
        if self.ask_sim_on() {
            g.ck.on = true; // point de reprise de la partie réelle pour ask_sim (Game.apply)
        }
        if self.search == "turn" && d.kind == "main" && !self.cfg.truthy("turn_replan", false) {
            // on suit la suite choisie tant que son prochain coup est légal
            if let Some(tp) = self.tplan.as_mut() {
                if tp.0 == (g.turn_no, me) && !tp.1.is_empty() && opts.contains(&tp.1[0]) {
                    return Ok(tp.1.remove(0));
                }
            }
        }
        if self.cfg.truthy("respect_ambush", false) && d.kind == "main" {
            let f: Vec<Value> = opts.iter().filter(|a| !self.risky_attack(g, me, a)).cloned().collect();
            if !f.is_empty() {
                opts = f;
            }
        }
        if self.cfg.truthy("spell_pre", conf().spell_pre) {
            let seed = self.rng.getrandbits(30) as i128;
            opts = spell_prefilter(g, me, &opts, seed, true, self.ev.as_ref());
            if opts.len() == 1 {
                return Ok(opts[0].clone());
            }
        }
        if opts.len() > self.max_cands {
            let mut o2 = vec![opts[0].clone()];
            o2.extend(self.rng.sample(&opts[1..], self.max_cands - 1));
            opts = o2;
        }
        if self.cfg.truthy("react_n", false) && d.kind != "main" {
            // « Très fort » (2026-10-27) : budget réduit (react_n tirages) pour les décisions autres que « main »
            let n0 = self.samples;
            self.samples = self.cfg.num("react_n", 0.0) as usize;
            let r = self.pick(g, d, &opts).and_then(|p| self.tie_break(g, d, &opts, p));
            self.samples = n0;
            return r;
        }
        let p = self.pick(g, d, &opts)?;
        self.tie_break(g, d, &opts, p)
    }
}

impl Agent for SearchAgent {
    fn start(&mut self, _g: &mut Game) {}
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        if let Some(p) = &self.plan {
            if let Some(r) = p.mulligan(g, pid)? {
                return Ok(r);
            }
        }
        heur_mulligan(&self.cfg, g, pid)
    }
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        if let Some(p) = &self.plan {
            if let Some(r) = p.choose(g, pid, kind, options, ctx)? {
                return Ok(r);
            }
        }
        let base = heur_choose(g, pid, kind, options, ctx)?;
        if self.ask_sim_on() {
            return self.ask_sim(g, pid, kind, options, base);
        }
        Ok(base)
    }
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        if let Some(a) = self.wild_move(d) {
            return Ok(a);
        }
        self.decide_search(g, d)
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// Nombre de niveaux de l'IA de la table (1 = le plus faible, `LEVELS` = le plus fort).
pub const LEVELS: i64 = 10;

/// Niveau 1 à 10 de l'IA de la table (2026-11-24, demande de l'utilisateur : « 10 niveaux comme sur Smash ») :
/// (probabilité d'un coup au hasard, mondes tirés, largeur du faisceau, mondes hors « main »). Niveaux 1 à 5 : l'IA du
/// niveau 6 qui se trompe de moins en moins ; 6, 7, 8 = anciens Normal, Fort, Très fort ; 9 et 10 : plus de calcul.
/// Mesures : `docs/memoire/ia-niveaux.md`.
pub const LEVEL_TABLE: [(f64, usize, usize, usize); 10] = [
    (0.70, 1, 0, 0),
    (0.50, 1, 0, 0),
    (0.35, 1, 0, 0),
    (0.20, 1, 0, 0),
    (0.08, 1, 0, 0),
    (0.0, 1, 0, 0),
    (0.0, 3, 0, 0),
    (0.0, 6, 6, 3),
    (0.0, 12, 8, 6),
    (0.0, 24, 12, 12),
];

/// Réglages de l'IA de la table au niveau `level` (borné à 1..=LEVELS) : recherche du tour entier, réglages de
/// réalisme de la table (`ambush`, `tie_react`, règle 10) et ligne de `LEVEL_TABLE`.
pub fn level_opts(level: i64) -> SearchOpts {
    let (wild, n, w, rn) = LEVEL_TABLE[(level.clamp(1, LEVELS) - 1) as usize];
    let mut c = Cfg::new();
    c.set("ambush", CfgV::Num(0.5));
    c.set("tie_react", CfgV::Num(1.0));
    if w > 0 {
        c.set("turn_w", CfgV::Num(w as f64));
        c.set("react_n", CfgV::Num(rn as f64));
    }
    if wild > 0.0 {
        c.set("wild", CfgV::Num(wild));
    }
    SearchOpts { samples: n, search: Some("turn".into()), cfg: c, ..SearchOpts::default() }
}

/// Niveau d'une partie enregistrée, sur l'échelle 1 à 10 : les parties d'avant le 2026-11-24 (sans `lvs`) portent le
/// nombre de mondes (1 Normal, 3 Fort, 6 Très fort) = niveaux 6, 7, 8 ; 0 reste l'IA du tutoriel.
pub fn level_from_saved(level: i64, scale10: bool) -> i64 {
    if scale10 || level == 0 {
        return level;
    }
    match level {
        3 => 7,
        6 => 8,
        _ => 6,
    }
}

// ---------------------------------------------------------------------- politique de simulation
/// `best_target_score(g, pid)`
pub fn best_target_score(_g: &Game, _pid: usize) -> i64 {
    0
}

/// `PolicyAgent` (et `plans.PlanPolicy` quand `plan` est posé : choix de résolution du plan). Jeu à règles fixes,
/// sans copie : politique des simulations et adversaire de référence.
#[derive(Clone, Default)]
pub struct PolicyAgent {
    pub cfg: Arc<Cfg>,
    pub plan: Option<Arc<dyn Plan>>,
}

impl PolicyAgent {
    pub fn new() -> PolicyAgent {
        PolicyAgent::default()
    }

    fn pol_react(&self) -> bool {
        self.cfg.truthy("pol_react", conf().pol_react)
    }

    fn ambush(&self) -> bool {
        match self.cfg.get("ambush") {
            None => conf().pol_ambush != 0.0,
            Some(v) => v.truthy(),
        }
    }

    /// `decide(g, d)` (sans `&mut self` : la politique n'a pas d'état)
    pub fn decide_pol(&self, g: &mut Game, d: &Decision) -> Res<Value> {
        if conf().lazy_check && d.options.is_none() {
            return self.lazy_check(g, d);
        }
        if d.kind != "main" {
            return self.react(g, d);
        }
        self.main(g, d)
    }

    /// `_lazy_check(g, d)` : RB_LAZY_CHECK=1 : le coup choisi sans la liste des coups doit être celui choisi avec la
    /// liste complète (contrôle, lent).
    pub fn lazy_check(&self, g: &mut Game, d: &Decision) -> Res<Value> {
        let r = if d.kind != "main" { self.react(g, d)? } else { self.main(g, d)? };
        if d.kind != "main" && !self.pol_react() {
            return Ok(r);
        }
        let opts = match d.kind {
            "main" => g.options_main(d.player)?,
            "focus" => g.options_focus(d.player)?,
            _ => g.options_priority(d.player)?,
        };
        let full = Decision { kind: d.kind, player: d.player, options: Some(opts) };
        let r2 = if d.kind != "main" { self.react(g, &full)? } else { self.main(g, &full)? };
        if r != r2 {
            panic!("coup sans liste {r:?} != coup avec la liste {r2:?} ({})", d.kind);
        }
        Ok(r)
    }

    /// `combat_margin(g, pid, bf, extra)`
    pub fn combat_margin(&self, g: &mut Game, pid: usize, bf: usize, extra: &[O]) -> Res<(i64, i64)> {
        let mut mine = 0;
        for u in g.units(Some(pid), Some(Loc::Bf(bf))) {
            if !g.o(u).stunned {
                mine += 0.max(g.might(u)?);
            }
        }
        let mut ex = 0;
        for &u in extra {
            ex += 0.max(g.might(u)?);
        }
        mine += ex;
        let mut theirs = 0;
        for u in g.units(Some(1 - pid), Some(Loc::Bf(bf))) {
            if !g.o(u).stunned {
                theirs += 0.max(g.might(u)?);
            }
        }
        Ok((mine, theirs))
    }

    /// `main(g, d)`
    pub fn main(&self, g: &mut Game, d: &Decision) -> Res<Value> {
        // Decision sans coups (Game.advance(lazy=True), simulations) : on génère seulement les familles de coups dont la
        // politique a besoin, dans l'ordre où elle les regarde (mêmes décisions qu'avec la liste complète).
        self.main_inner(g, d.player, d.options.as_deref())
    }

    /// `_main(g, pid, opts)`
    pub fn main_inner(&self, g: &mut Game, pid: usize, opts: Option<&[Value]>) -> Res<Value> {
        let (mut plays, mut moves, acts): (Vec<Value>, Vec<Value>, Vec<Value>);
        match opts {
            None => {
                acts = crate::actions::ability_options(g, pid, false, false, true, false)?;
                plays = vec![];
                moves = vec![];
            }
            Some(o) => {
                plays = o.iter().filter(|x| kind_of(x) == "play").cloned().collect();
                moves = o.iter().filter(|x| kind_of(x) == "move").cloned().collect();
                acts = o.iter().filter(|x| kind_of(x) == "act").cloned().collect();
            }
        }
        // 1. valeur gratuite : capacités qui piochent / rampent (Hook)
        for o in &acts {
            if let Value::Int(uid) = o.list()[1] {
                if let Some(src) = g.obj(uid as u32) {
                    if g.cname(src) == "Baited Hook" {
                        return Ok(o.clone());
                    }
                }
            }
        }
        // 2. attaquer ou conquérir
        if opts.is_none() {
            moves = crate::actions::move_options(g, pid)?;
        }
        let tempo = conf().tempo;
        let (mut best, mut bv): (Option<Value>, f64) = (None, 0.0);
        for o in &moves {
            let l = o.list();
            let units: Vec<O> = l[1].list().iter().map(|u| g.obj(u.int() as u32).expect("g.obj None")).collect();
            let Some(Loc::Bf(dest)) = loc_of(&l[2]) else { continue };
            let bctrl = g.bfs[dest].ctrl;
            let (mine, theirs) = self.combat_margin(g, pid, dest, &units)?;
            let mut v = if theirs == 0 {
                if bctrl != Some(pid) {
                    5.0
                } else {
                    0.0
                }
            } else if mine > theirs {
                3.0 + 0.2 * (mine - theirs) as f64
            } else {
                -1.0
            };
            if tempo && bctrl == Some(1 - pid) && g.p[1 - pid].points >= g.victory - 1 && mine > theirs {
                v += 4.0; // casser la tenue gagnante avant de conquérir ailleurs
            }
            v -= 0.1 * units.len() as f64;
            if v > bv {
                best = Some(o.clone());
                bv = v;
            }
        }
        if let Some(b) = best {
            return Ok(b);
        }
        // 3. jouer la plus grosse unité possible
        if opts.is_none() {
            if let Some(o) = self.play_lazy(g, pid)? {
                return Ok(o);
            }
            plays = vec![];
        }
        if !plays.is_empty() && self.ambush() {
            plays.retain(|o| {
                let c = self.card_of(g, o).expect("card_of None (Python : AttributeError)");
                !holds_ambush(g, pid, g.spec(c))
            });
        }
        let mut units: Vec<Value> = plays.iter().filter(|o| self.is_unit(g, o)).cloned().collect();
        if !units.is_empty() {
            units.sort_by_key(|o| -(g.spec(self.card_of(g, o).unwrap()).might.unwrap_or(0) as i64));
            return Ok(units[0].clone());
        }
        // 4. équipements / sorts
        if !plays.is_empty() && self.cfg.truthy("pol_keep", false) {
            // (essai) garder les sorts [Reaction] pour les fenêtres de réaction
            plays.retain(|o| !self.reaction_card(g, o));
        }
        if !plays.is_empty() && self.cfg.truthy("pol_trick", conf().pol_trick) {
            // (2026-10-10) garder les sorts de combat (is_trick) pour les showdowns
            plays.retain(|o| !self.trick_card(g, o));
        }
        if let Some(p) = plays.first() {
            return Ok(p.clone());
        }
        if let Some(o) = acts.first() {
            return Ok(o.clone());
        }
        Ok(tup(&["end"]))
    }

    /// `_play_lazy(g, pid)` : étapes 3 et 4 de _main sans la liste complète des cartes jouables.
    pub fn play_lazy(&self, g: &mut Game, pid: usize) -> Res<Option<Value>> {
        let mut cands: Vec<(O, &'static str, Option<O>)> = vec![];
        let mut seen: Vec<(String, &'static str)> = vec![];
        for (c, src) in crate::actions::playable_cards(g, pid) {
            let key = (g.cname(c).to_string(), src);
            if seen.contains(&key) && src != "facedown" {
                continue;
            }
            seen.push(key);
            let k = self.card_of(g, &Value::Tuple(vec!["play".into(), g.o(c).uid.into()]));
            cands.push((c, src, k));
        }
        let mut units: Vec<(O, &'static str, O)> =
            cands.iter().filter_map(|&(c, s, k)| k.filter(|&k| g.spec(k).type_ == "Unit").map(|k| (c, s, k))).collect();
        if self.ambush() {
            units.retain(|x| !holds_ambush(g, pid, g.spec(x.2)));
        }
        units.sort_by_key(|x| -(g.spec(x.2).might.unwrap_or(0) as i64)); // tri stable : l'ordre des cartes à égalité
        for (c, src, _) in units {
            let chs = crate::actions::card_choices(g, pid, c, src, false, false, false)?;
            if let Some(ch) = chs.into_iter().next() {
                return Ok(Some(Value::Tuple(vec!["play".into(), g.o(c).uid.into(), src.into(), ch.into()])));
            }
        }
        let keep = self.cfg.truthy("pol_keep", false);
        let trick = self.cfg.truthy("pol_trick", conf().pol_trick);
        for (c, src, k) in cands {
            if k.is_some_and(|k| g.spec(k).type_ == "Unit") {
                continue; // (aucun choix : vu ci-dessus)
            }
            let o = Value::Tuple(vec!["play".into(), g.o(c).uid.into()]);
            if keep && self.reaction_card(g, &o) || trick && self.trick_card(g, &o) {
                continue;
            }
            let chs = crate::actions::card_choices(g, pid, c, src, false, false, false)?;
            if let Some(ch) = chs.into_iter().next() {
                return Ok(Some(Value::Tuple(vec!["play".into(), g.o(c).uid.into(), src.into(), ch.into()])));
            }
        }
        Ok(None)
    }

    /// `reaction_card(g, o)`
    pub fn reaction_card(&self, g: &Game, o: &Value) -> bool {
        self.card_of(g, o).is_some_and(|c| g.spec(c).type_ == "Spell" && kw(g.spec(c), "Reaction"))
    }

    /// `trick_card(g, o)`
    pub fn trick_card(&self, g: &Game, o: &Value) -> bool {
        self.card_of(g, o).is_some_and(|c| is_trick(g.spec(c)))
    }

    /// `is_unit(g, o)`
    pub fn is_unit(&self, g: &Game, o: &Value) -> bool {
        self.card_of(g, o).is_some_and(|c| g.spec(c).type_ == "Unit")
    }

    /// `card_of(g, o)` : carte d'une action `("play", uid, ...)` (main, champion, défausse, cartes cachées).
    pub fn card_of(&self, g: &Game, o: &Value) -> Option<O> {
        let uid = &o.list()[1];
        for pl in &g.p {
            for z in [&pl.hand, &pl.champ, &pl.trash] {
                for &c in z {
                    if Value::from(g.o(c).uid) == *uid {
                        return Some(c);
                    }
                }
            }
        }
        for b in &g.bfs {
            for &c in &b.facedowns {
                if Value::from(g.o(c).uid) == *uid {
                    return Some(c);
                }
            }
        }
        None
    }

    /// `react(g, d)`
    pub fn react(&self, g: &mut Game, d: &Decision) -> Res<Value> {
        let pid = d.player;
        let pass = tup(&["pass"]);
        let bf = match &g.sd {
            Some(sd) if sd.stage == "open" => sd.bf,
            _ => return Ok(pass),
        };
        let pr = self.pol_react();
        if pr && g.units(Some(pid), Some(Loc::Bf(bf))).is_empty() {
            return Ok(pass); // (react_best passerait : inutile de générer les cartes)
        }
        let plays: Vec<Value> = match &d.options {
            None => {
                // Game.advance(lazy=True) : seulement les cartes (timed_options)
                let closed = d.kind == "priority";
                crate::actions::play_options(g, pid, closed, true, false)?
            }
            Some(o) => o.iter().filter(|x| kind_of(x) == "play").cloned().collect(),
        };
        if plays.is_empty() {
            return Ok(pass);
        }
        if pr {
            return self.react_best(g, pid, bf, &plays);
        }
        let (mine, theirs) = self.combat_margin(g, pid, bf, &[])?;
        let losing = mine <= theirs && !g.units(Some(pid), Some(Loc::Bf(bf))).is_empty();
        if losing && g.rng.random() < 0.8 {
            return Ok(plays[0].clone());
        }
        Ok(pass)
    }

    /// `react_best(g, pid, bf, plays)` : chaque sort jouable (REACT_CANDS au plus) est essayé sur une copie, la chaîne
    /// résolue, puis noté par l'écart de Might au battlefield ; on joue le meilleur s'il améliore l'écart obtenu en
    /// passant et que, sans lui, pid ne gagne pas le combat.
    pub fn react_best(&self, g: &mut Game, pid: usize, bf: usize, plays: &[Value]) -> Res<Value> {
        let pass = tup(&["pass"]);
        if g.units(Some(pid), Some(Loc::Bf(bf))).is_empty() {
            return Ok(pass);
        }
        let base =
            if !g.chain.is_empty() { self.react_margin(g, pid, bf, &pass)? } else { Some(self.margin_now(g, pid, bf)? as f64) };
        let Some(base) = base else { return Ok(pass) };
        if base > 0.0 {
            return Ok(pass);
        }
        let (mut best, mut bv) = (None, base);
        for o in plays.iter().take(REACT_CANDS) {
            let v = self.react_margin(g, pid, bf, o)?;
            if let Some(v) = v {
                if v > bv + 1e-9 {
                    best = Some(o.clone());
                    bv = v;
                }
            }
        }
        Ok(best.unwrap_or(pass))
    }

    /// `margin_now(g, pid, bf)`
    pub fn margin_now(&self, g: &mut Game, pid: usize, bf: usize) -> Res<i64> {
        let (mine, theirs) = self.combat_margin(g, pid, bf, &[])?;
        Ok(mine - theirs)
    }

    /// `react_margin(g, pid, bf, a)` : écart de Might à bf pour pid après a et la résolution de la chaîne (copie ;
    /// compteurs d'objets restaurés). +1000 / −1000 si la partie finit ; None si a n'a pas pu être joué.
    pub fn react_margin(&self, g: &mut Game, pid: usize, bf: usize, a: &Value) -> Res<Option<f64>> {
        let n0 = counters(g);
        let r = (|| {
            let mut c = g.clone_game();
            // c.agents = g.agents : les mêmes agents (dans la partie réelle, ceux des joueurs)
            c.agents = Agents(g.agents.0.iter().map(|x| x.clone_box()).collect());
            c.apply(a);
            for _ in 0..80 {
                if c.chain.is_empty() && c.chain_origin.is_none() && c.trigq.is_empty() {
                    break;
                }
                let dd = c.advance(true);
                let Some(dd) = dd else {
                    return Ok(Some(if c.winner == Some(pid as i64) { 1000.0 } else { -1000.0 }));
                };
                if dd.kind != "priority" {
                    break;
                }
                c.apply(&tup(&["pass"]));
            }
            Ok(Some(self.margin_now(&mut c, pid, bf)? as f64))
        })();
        set_counters(g, n0);
        r
    }
}

impl Agent for PolicyAgent {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        if let Some(p) = &self.plan {
            if let Some(r) = p.choose(g, pid, kind, options, ctx)? {
                return Ok(r);
            }
        }
        heur_choose(g, pid, kind, options, ctx)
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        heur_mulligan(&self.cfg, g, pid)
    }
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        self.decide_pol(g, d)
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// `rollout_policy(g, horizon, max_steps, cfg)` : la politique joue jusqu'à ce que `horizon` tours soient passés (ou
/// la fin de la partie).
pub fn rollout_policy(g: &mut Game, horizon: i64, max_steps: usize, cfg: Option<&Arc<Cfg>>) {
    let pol = PolicyAgent { cfg: cfg.cloned().unwrap_or_default(), plan: None };
    let stop = g.turn_no + horizon;
    for _ in 0..max_steps {
        let Some(d) = g.advance(true) else { return }; // coups générés par la politique, selon son besoin
        if d.kind == "main" && g.turn_no > stop {
            return;
        }
        match pol.decide_pol(g, &d) {
            Ok(a) => g.apply(&a),
            Err(_) => return,
        }
    }
}
