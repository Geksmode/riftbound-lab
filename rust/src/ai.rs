//! IA des deux decks (`ai.py`). Traduction ligne à ligne : mêmes tirages, mêmes copies, mêmes calculs flottants.
//!
//! SearchAgent : pour chaque décision de haut niveau (action principale, priorité, focus), chaque action légale est
//! essayée sur des copies de la partie, jouée jusqu'à l'horizon par la politique de simulation, et notée par
//! `evaluate`. L'information cachée est respectée : dans chaque copie, la main et les cartes cachées adverses sont
//! retirées au hasard parmi les cartes non vues, et les deux decks mélangés (`determinize`).
use std::sync::{Arc, OnceLock};

use crate::game::{Agent, Decision, Game, Res, O};
use crate::plans::Plan;
use crate::pyrandom::PyRandom;
use crate::spec::Spec;
use crate::value::{Map, Value};

// ---------------------------------------------------------------------- environnement
/// `os.environ.get(name)` : None hors d'un environnement (WebAssembly) ou si la variable est absente.
pub fn env_var(name: &str) -> Option<String> {
    todo!("{name}")
}

/// `os.environ.get(name, default)`
pub fn env_or(name: &str, default: &str) -> String {
    todo!("{name}{default}")
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
pub const URGENT_SAMPLES: usize = 6;
pub const ASK_SIM_MAX: usize = 12;
pub const ASK_SIM_SKIP: [&str; 3] = ["pay_gold", "pay_rune", "recycle_rune"];
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
        todo!()
    }
    /// clés de `EV` (ordre du dict Python)
    pub const KEYS: [&'static str; 20] = [
        "pts", "pts_hi", "bf", "fd", "hold_win", "unit0", "might", "cost", "on_bf", "card0", "card_e", "react",
        "react_kw", "rune", "leg_emp", "xp", "deck_low", "pts_ramp", "trick", "open",
    ];
    /// `w.get(k)`
    pub fn get(&self, k: &str) -> Option<f64> {
        todo!("{k}")
    }
    /// `w[k] = v` ; false si la clé n'existe pas
    pub fn set(&mut self, k: &str, v: f64) -> bool {
        todo!("{k}{v}")
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
    C.get_or_init(|| todo!())
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

/// `cfg` de SearchAgent (dict Python, ordre d'insertion).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cfg(pub Vec<(String, CfgV)>);

impl Cfg {
    pub fn new() -> Cfg {
        Cfg(vec![])
    }
    /// `cfg[k] = v`
    pub fn set(&mut self, k: &str, v: CfgV) {
        todo!("{k}{v:?}")
    }
    pub fn get(&self, k: &str) -> Option<&CfgV> {
        todo!("{k}")
    }
    /// `cfg.get(k, default)` lu comme un nombre
    pub fn num(&self, k: &str, default: f64) -> f64 {
        todo!("{k}{default}")
    }
    /// `bool(cfg.get(k, default))`
    pub fn truthy(&self, k: &str, default: bool) -> bool {
        todo!("{k}{default}")
    }
    /// `cfg.get(k, default)` lu comme un texte
    pub fn text(&self, k: &str, default: &str) -> String {
        todo!("{k}{default}")
    }
}

// ---------------------------------------------------------------------- évaluation
/// `lasting_might(g, o)` : Might sans les effets « this turn ».
pub fn lasting_might(g: &mut Game, o: O) -> Res<i64> {
    todo!("{o:?}{}", g.turn_no)
}

/// `holds_ambush(g, pid, spec)`
pub fn holds_ambush(g: &Game, pid: usize, spec: &Spec) -> bool {
    todo!("{pid}{}{}", spec.name, g.turn_no)
}

/// `open_count(g, pid)`
pub fn open_count(g: &Game, pid: usize) -> i64 {
    todo!("{pid}{}", g.turn_no)
}

/// `is_trick(spec)`
pub fn is_trick(spec: &Spec) -> bool {
    todo!("{}", spec.name)
}

/// `point_value(p, victory, w)`
pub fn point_value(p: i64, victory: i64, w: &Ev) -> f64 {
    todo!("{p}{victory}{w:?}")
}

/// `card_value(c, w)`
pub fn card_value(g: &Game, c: O, w: &Ev) -> f64 {
    todo!("{c:?}{w:?}{}", g.turn_no)
}

/// `unit_eval(g, u, w)`
pub fn unit_eval(g: &mut Game, u: O, w: &Ev) -> Res<f64> {
    todo!("{u:?}{w:?}{}", g.turn_no)
}

/// `GEAR_V.get(name)`
pub fn gear_v(name: &str) -> Option<f64> {
    todo!("{name}")
}

/// `danger(g, me)` : l'adversaire est à portée de victoire.
pub fn danger(g: &Game, me: usize) -> bool {
    todo!("{me}{}", g.turn_no)
}

/// `evaluate(g, me, w)` (w None : `EV`)
pub fn evaluate(g: &mut Game, me: usize, w: Option<&Ev>) -> f64 {
    todo!("{me}{w:?}{}", g.turn_no)
}

// ---------------------------------------------------------------------- choix heuristiques
/// `Heuristics.mulligan(g, pid)` (cfg : `s.cfg`)
pub fn heur_mulligan(cfg: &Cfg, g: &mut Game, pid: usize) -> Res<Vec<O>> {
    todo!("{cfg:?}{pid}{}", g.turn_no)
}

/// `Heuristics.choose(g, pid, kind, options, ctx)` : indice de l'option choisie.
pub fn heur_choose(g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
    todo!("{pid}{kind}{options:?}{ctx:?}{}", g.turn_no)
}

/// `FastAgent` : politique de simulation qui passe et finit le tour, choix heuristiques.
#[derive(Clone, Default)]
pub struct FastAgent;

impl Agent for FastAgent {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        heur_choose(g, pid, kind, options, ctx)
    }
    fn decide(&mut self, _g: &mut Game, d: &Decision) -> Res<Value> {
        todo!("{}", d.kind)
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// `determinize(g, me, rng)` : retire au hasard ce que `me` ne voit pas.
pub fn determinize(g: &mut Game, me: usize, rng: &mut PyRandom) {
    todo!("{me}{}{}", g.turn_no, rng.random())
}

/// Une entrée de `Game._asklog` : (type, nombre d'options, indice de la réponse, valeur simple).
pub type AskLog = (String, usize, Option<usize>, Option<Value>);

/// `_AskReplay` : rejeu d'ask_sim.
#[derive(Clone)]
pub struct AskReplay {
    pub log: Vec<AskLog>,
    pub j: usize,
    pub alt: usize,
    pub k: usize,
    pub seen: Arc<std::sync::Mutex<Option<Vec<String>>>>,
}

impl Agent for AskReplay {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        todo!("{pid}{kind}{options:?}{ctx:?}{}", g.turn_no)
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// `keep_played(c, opp, a)`
pub fn keep_played(c: &mut Game, opp: usize, a: &Value) {
    todo!("{opp}{a:?}{}", c.turn_no)
}

/// `opt_sig(kind, options)`
pub fn opt_sig(g: &Game, kind: &str, options: &[Value]) -> Vec<String> {
    todo!("{kind}{options:?}{}", g.turn_no)
}

/// `spell_prefilter(g, me, opts, seed, determ, ev)`
pub fn spell_prefilter(g: &mut Game, me: usize, opts: &[Value], seed: i128, determ: bool, ev: Option<&Ev>) -> Vec<Value> {
    todo!("{me}{opts:?}{seed}{determ}{ev:?}{}", g.turn_no)
}

/// `settle_pass(c, max_steps)`
pub fn settle_pass(c: &mut Game, max_steps: usize) {
    todo!("{max_steps}{}", c.turn_no)
}

/// `rollout(g, max_steps)`
pub fn rollout(g: &mut Game, max_steps: usize) {
    todo!("{max_steps}{}", g.turn_no)
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
}

/// Résultat de `pick` : (meilleur coup, sa valeur, [(valeur, coup)]).
pub type Picked = (Value, f64, Vec<(f64, Value)>);

impl SearchAgent {
    /// `SearchAgent(seed, samples, max_cands, name, horizon, cfg, search, sh_extra)`
    pub fn new(seed: i128, o: SearchOpts) -> SearchAgent {
        todo!("{seed}{o:?}")
    }
    /// `ask_sim(g, pid, kind, options, base)`
    pub fn ask_sim(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], base: usize) -> Res<usize> {
        todo!("{pid}{kind}{options:?}{base}{}", g.turn_no)
    }
    /// `risky_attack(g, me, a)`
    pub fn risky_attack(&self, g: &Game, me: usize, a: &Value) -> bool {
        todo!("{me}{a:?}{}", g.turn_no)
    }
    /// `policies(me)` : agents des simulations
    pub fn policies(&self, me: usize) -> Vec<Box<dyn Agent>> {
        todo!("{me}")
    }
    /// `value(c, me)`
    pub fn value(&self, c: &mut Game, me: usize) -> Res<f64> {
        todo!("{me}{}", c.turn_no)
    }
    /// `prior_of(g, me, a)`
    pub fn prior_of(&self, g: &mut Game, me: usize, a: &Value) -> Res<f64> {
        todo!("{me}{a:?}{}", g.turn_no)
    }
    /// `one(g, me, a, rng, world)` : valeur d'un rollout.
    pub fn one(&self, g: &Game, me: usize, a: &Value, rng: &mut PyRandom, world: Option<i128>) -> Res<f64> {
        todo!("{me}{a:?}{world:?}{}{}", g.turn_no, rng.random())
    }
    /// `score(g, me, a)` : ancienne recherche
    pub fn score(&mut self, g: &Game, me: usize, a: &Value) -> Res<f64> {
        todo!("{me}{a:?}{}", g.turn_no)
    }
    /// `tie_break(g, d, opts, picked)`
    pub fn tie_break(&mut self, g: &mut Game, d: &Decision, opts: &[Value], picked: Picked) -> Res<Value> {
        todo!("{}{opts:?}{:?}{}", d.kind, picked.1, g.turn_no)
    }
    /// `urgent(g, d, me, best_v)`
    pub fn urgent(&self, g: &Game, d: &Decision, me: usize, best_v: f64) -> bool {
        todo!("{}{me}{best_v}{}", d.kind, g.turn_no)
    }
    /// `pick(g, d, opts)`
    pub fn pick(&mut self, g: &mut Game, d: &Decision, opts: &[Value]) -> Res<Picked> {
        todo!("{}{opts:?}{}", d.kind, g.turn_no)
    }
    /// `pick_crn(g, d, opts)`
    pub fn pick_crn(&mut self, g: &mut Game, d: &Decision, opts: &[Value]) -> Res<Picked> {
        todo!("{}{opts:?}{}", d.kind, g.turn_no)
    }
    /// `pick_turn(g, d, opts)`
    pub fn pick_turn(&mut self, g: &mut Game, d: &Decision, opts: &[Value]) -> Res<Picked> {
        todo!("{}{opts:?}{}", d.kind, g.turn_no)
    }
    /// `turn_search(g, me, opts, worlds)` : (meilleure suite, sa valeur, [(valeur, premier coup)])
    pub fn turn_search(&mut self, g: &Game, me: usize, opts: &[Value], worlds: &[i128]) -> Res<(Vec<Value>, f64, Vec<(f64, Value)>)> {
        todo!("{me}{opts:?}{worlds:?}{}", g.turn_no)
    }
}

impl Agent for SearchAgent {
    fn start(&mut self, _g: &mut Game) {}
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        todo!("{pid}{}", g.turn_no)
    }
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        todo!("{pid}{kind}{options:?}{ctx:?}{}", g.turn_no)
    }
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        todo!("{}{}", d.kind, g.turn_no)
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------- politique de simulation
/// `best_target_score(g, pid)`
pub fn best_target_score(_g: &Game, _pid: usize) -> i64 {
    0
}

/// `PolicyAgent` (et `plans.PlanPolicy` quand `plan` est posé : choix de résolution du plan).
#[derive(Clone, Default)]
pub struct PolicyAgent {
    pub cfg: Arc<Cfg>,
    pub plan: Option<Arc<dyn Plan>>,
}

impl PolicyAgent {
    pub fn new() -> PolicyAgent {
        PolicyAgent::default()
    }
    /// `_lazy_check(g, d)`
    pub fn lazy_check(&self, g: &mut Game, d: &Decision) -> Res<Value> {
        todo!("{}{}", d.kind, g.turn_no)
    }
    /// `combat_margin(g, pid, bf, extra)`
    pub fn combat_margin(&self, g: &mut Game, pid: usize, bf: usize, extra: &[O]) -> Res<(i64, i64)> {
        todo!("{pid}{bf}{extra:?}{}", g.turn_no)
    }
    /// `main(g, d)`
    pub fn main(&self, g: &mut Game, d: &Decision) -> Res<Value> {
        todo!("{}{}", d.kind, g.turn_no)
    }
    /// `_main(g, pid, opts)`
    pub fn main_inner(&self, g: &mut Game, pid: usize, opts: Option<&[Value]>) -> Res<Value> {
        todo!("{pid}{opts:?}{}", g.turn_no)
    }
    /// `_play_lazy(g, pid)`
    pub fn play_lazy(&self, g: &mut Game, pid: usize) -> Res<Option<Value>> {
        todo!("{pid}{}", g.turn_no)
    }
    /// `reaction_card(g, o)`
    pub fn reaction_card(&self, g: &Game, o: &Value) -> bool {
        todo!("{o:?}{}", g.turn_no)
    }
    /// `trick_card(g, o)`
    pub fn trick_card(&self, g: &Game, o: &Value) -> bool {
        todo!("{o:?}{}", g.turn_no)
    }
    /// `is_unit(g, o)`
    pub fn is_unit(&self, g: &Game, o: &Value) -> bool {
        todo!("{o:?}{}", g.turn_no)
    }
    /// `card_of(g, o)` : carte d'une action `("play", uid, ...)`.
    pub fn card_of(&self, g: &Game, o: &Value) -> Option<O> {
        todo!("{o:?}{}", g.turn_no)
    }
    /// `react(g, d)`
    pub fn react(&self, g: &mut Game, d: &Decision) -> Res<Value> {
        todo!("{}{}", d.kind, g.turn_no)
    }
    /// `react_best(g, pid, bf, plays)`
    pub fn react_best(&self, g: &mut Game, pid: usize, bf: usize, plays: &[Value]) -> Res<Value> {
        todo!("{pid}{bf}{plays:?}{}", g.turn_no)
    }
    /// `margin_now(g, pid, bf)`
    pub fn margin_now(&self, g: &mut Game, pid: usize, bf: usize) -> Res<i64> {
        todo!("{pid}{bf}{}", g.turn_no)
    }
    /// `react_margin(g, pid, bf, a)` : None si a n'a pas pu être joué.
    pub fn react_margin(&self, g: &mut Game, pid: usize, bf: usize, a: &Value) -> Res<Option<f64>> {
        todo!("{pid}{bf}{a:?}{}", g.turn_no)
    }
}

impl Agent for PolicyAgent {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        todo!("{pid}{kind}{options:?}{ctx:?}{}", g.turn_no)
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        heur_mulligan(&self.cfg, g, pid)
    }
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        todo!("{}{}", d.kind, g.turn_no)
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// `rollout_policy(g, horizon, max_steps, cfg)`
pub fn rollout_policy(g: &mut Game, horizon: i64, max_steps: usize, cfg: Option<&Arc<Cfg>>) {
    todo!("{horizon}{max_steps}{cfg:?}{}", g.turn_no)
}
