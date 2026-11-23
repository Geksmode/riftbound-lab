//! Plans de jeu par légende (`plans_legendes.py`). Hypothèses tirées de guides, non mesurées ; activées seulement à
//! la demande (`exp_search.py ... "sh@plan=leg"`). La table `LEG` est dans le code Python (pas de fichier lu) : elle est
//! recopiée ici à l'identique. `fiches()` (génération de `gameplans/legendes.md`) n'est pas portée.
use std::sync::Arc;

use crate::game::DeckList;
use crate::plans::Plan;

/// `W_CONQ` (RB_LEG_WC)
pub fn w_conq() -> f64 {
    todo!()
}
/// `W_HOLD` (RB_LEG_WT)
pub fn w_hold() -> f64 {
    todo!()
}
/// `W_OPEN` (RB_LEG_WO)
pub fn w_open() -> f64 {
    todo!()
}
/// `W_KEY` (RB_LEG_WK)
pub fn w_key() -> f64 {
    todo!()
}
/// `W_HAND` (RB_LEG_WH)
pub fn w_hand() -> f64 {
    todo!()
}

/// `SEUIL[style]`
pub fn seuil(style: &str) -> i64 {
    todo!("{style}")
}

/// Fiche d'une légende (`L(axe, style, garde, cles, conf, src, maj, note)`).
#[derive(Clone, Debug)]
pub struct Leg {
    pub axe: &'static str,
    pub style: &'static str,
    pub garde: &'static [&'static str],
    pub cles: &'static [&'static str],
    pub conf: &'static str,
    pub src: &'static str,
    pub maj: &'static str,
    pub note: &'static str,
}

/// `LEG` : (légende, fiche), ordre du dict Python.
pub fn leg() -> &'static [(&'static str, Leg)] {
    todo!()
}

/// `SANS_PLAN`
pub const SANS_PLAN: [&str; 9] = [
    "Ekko, Boy Who Shattered Time",
    "Evelynn, Agony's Embrace",
    "Jarvan IV, Exemplar of Demacia",
    "K'Sante, Pride of Nazumah",
    "Mordekaiser, Iron Revenant",
    "Orianna, Lady of Clockwork",
    "Riven, The Exile",
    "Seraphine, Starry-Eyed Songstress",
    "Ziggs, Hexplosives Expert",
];

/// `EXISTING`
pub const EXISTING: [(&str, &str); 2] = [("Akali, Rogue Assassin", "Gorica"), ("LeBlanc, Deceiver", "Hook tempo")];

/// `LegendPlan(legend)`
#[derive(Clone, Debug)]
pub struct LegendPlan {
    pub legend: String,
    pub d: Leg,
}

impl LegendPlan {
    pub fn new(legend: &str) -> LegendPlan {
        todo!("{legend}")
    }
}

impl Plan for LegendPlan {
    fn name(&self) -> &str {
        "legende"
    }
}

/// `plan_for(deck)`
pub fn plan_for(deck: &DeckList) -> Arc<dyn Plan> {
    todo!("{deck:?}")
}
