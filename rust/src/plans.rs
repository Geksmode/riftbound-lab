//! Plans de jeu tirés des joueurs réels (`plans.py`). Un plan agit à cinq endroits : mulligan, battlefield présenté,
//! a priori d'une action (`prior`), termes ajoutés à l'évaluation (`shape`), choix pendant la résolution (`choose`).
//! PlanAgent = SearchAgent + plan ; mêmes simulations, seul le score change.
use std::sync::Arc;

use crate::ai::{SearchAgent, SearchOpts};
use crate::game::{Game, Res, O};
use crate::value::{Map, Value};

/// `VIDEO` (RB_VIDEO, actif par défaut)
pub fn video() -> bool {
    todo!()
}

/// `KEEP_REFLECTION` (RB_REFL, actif par défaut)
pub fn keep_reflection() -> bool {
    todo!()
}

pub const DK: [&str; 8] = [
    "Soaring Scout",
    "Honest Broker",
    "Watchful Sentry",
    "Black Rose Dignitary",
    "LeBlanc, Fragmented",
    "Glasc Mixologist",
    "Ruined Rex",
    "Rift Herald",
];

/// `card_of(g, uid)`
pub fn card_of(g: &Game, uid: i64) -> Option<O> {
    todo!("{uid}{}", g.turn_no)
}

/// `Plan` : plan de jeu (méthodes par défaut = aucun plan).
pub trait Plan: Send + Sync {
    /// `name`
    fn name(&self) -> &str {
        "aucun"
    }
    /// `mulligan(g, pid)` : None = heuristique par défaut
    fn mulligan(&self, _g: &mut Game, _pid: usize) -> Res<Option<Vec<O>>> {
        Ok(None)
    }
    /// `battlefield(deck, first, opp_bf)` : `battlefields` = `deck["battlefields"]` ; None = tirage au hasard
    fn battlefield(&self, _battlefields: &[String], _first: bool, _opp_bf: Option<&str>) -> Option<String> {
        None
    }
    /// `prior(g, me, a)`
    fn prior(&self, _g: &mut Game, _me: usize, _a: &Value) -> Res<f64> {
        Ok(0.0)
    }
    /// `shape(g, me)`
    fn shape(&self, _g: &mut Game, _me: usize) -> Res<f64> {
        Ok(0.0)
    }
    /// `choose(g, pid, kind, options, ctx)` : indice choisi, None = `NotImplemented`
    fn choose(&self, _g: &mut Game, _pid: usize, _kind: &str, _options: &[Value], _ctx: &Map) -> Res<Option<usize>> {
        Ok(None)
    }
}

/// `Plan()` : aucun plan.
#[derive(Clone, Default)]
pub struct NoPlan;
impl Plan for NoPlan {}

/// `AkaliGorica`
#[derive(Clone, Default)]
pub struct AkaliGorica;

/// `AkaliGoricaAggro`
#[derive(Clone, Default)]
pub struct AkaliGoricaAggro;

/// `LeBlancDeathknell`
#[derive(Clone, Default)]
pub struct LeBlancDeathknell;

/// `LeBlancHookTempo`
#[derive(Clone, Default)]
pub struct LeBlancHookTempo;

/// `LeBlancHookWindswept`
#[derive(Clone, Default)]
pub struct LeBlancHookWindswept;

impl Plan for AkaliGorica {
    fn name(&self) -> &str {
        "Gorica"
    }
}
impl Plan for AkaliGoricaAggro {
    fn name(&self) -> &str {
        "Gorica agressif"
    }
}
impl Plan for LeBlancDeathknell {
    fn name(&self) -> &str {
        "Deathknell"
    }
}
impl Plan for LeBlancHookTempo {
    fn name(&self) -> &str {
        "Hook tempo"
    }
}
impl Plan for LeBlancHookWindswept {
    fn name(&self) -> &str {
        "Hook tempo Windswept"
    }
}

/// Noms de `PLANS` (ordre du dict Python).
pub const PLAN_NAMES: [&str; 5] = ["Gorica", "Hook tempo", "Hook tempo Windswept", "Gorica agressif", "Deathknell"];

/// `PLANS[name]()` : None si le nom est inconnu.
pub fn plans(name: &str) -> Option<Arc<dyn Plan>> {
    todo!("{name}")
}

/// `PlanPolicy(plan)` : politique de simulation qui fait les choix de résolution du plan.
pub fn plan_policy(plan: Option<Arc<dyn Plan>>) -> crate::ai::PolicyAgent {
    todo!("{}", plan.is_some())
}

/// `PlanAgent(seed, plan, opp_plan, **kw)` : plan None = `Plan()`.
pub fn plan_agent(seed: i128, plan: Option<Arc<dyn Plan>>, opp_plan: Option<Arc<dyn Plan>>, o: SearchOpts) -> SearchAgent {
    todo!("{seed}{}{}{o:?}", plan.is_some(), opp_plan.is_some())
}
