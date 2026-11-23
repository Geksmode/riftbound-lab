//! Plan « général » déduit du deck (`plans_general.py`) : rôle aggro / milieu / contrôle tiré de la courbe.
use std::sync::Arc;

use crate::game::DeckList;
use crate::plans::Plan;

/// `GENERAL` (RB_GENERAL=1)
pub fn general() -> bool {
    todo!()
}
/// `W_MIGHT` (RB_GEN_WM)
pub fn w_might() -> f64 {
    todo!()
}
/// `W_HAND` (RB_GEN_WH)
pub fn w_hand() -> f64 {
    todo!()
}

/// `deck_role(deck)` : (rôle, part d'unités ≤ 3, coût moyen)
pub fn deck_role(main: &[String]) -> (&'static str, f64, f64) {
    todo!("{main:?}")
}

/// `GeneralPlan(deck)`
#[derive(Clone, Debug)]
pub struct GeneralPlan {
    pub role: &'static str,
    pub share: f64,
    pub avg: f64,
}

impl GeneralPlan {
    pub fn new(deck: &DeckList) -> GeneralPlan {
        todo!("{deck:?}")
    }
}

impl Plan for GeneralPlan {
    fn name(&self) -> &str {
        "general"
    }
}

/// `plan_for(deck)`
pub fn plan_for(deck: &DeckList) -> Arc<dyn Plan> {
    todo!("{deck:?}")
}
