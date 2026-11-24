//! Plan « général » déduit du deck (`plans_general.py`) : hypothèse à mesurer, pas une vérité. Le rôle (aggro / milieu
//! / contrôle) vient de la courbe du deck : part d'unités à coût ≤ 3 et coût moyen des cartes.
//!   - mulligan : aggro garde les unités ≤ 3 et renvoie les cartes ≥ 5 ; contrôle garde tout sauf les cartes ≥ 7 ;
//!   - shape : aggro valorise la might posée sur les battlefields, contrôle valorise la main.
//! Activation : RB_GENERAL=1 (voir plan_for). Les plans Akali / LeBlanc ont priorité sur lui.
use std::sync::{Arc, OnceLock};

use crate::ai::env_or;
use crate::game::{DeckList, Game, Res, O};
use crate::plans::{plans, NoPlan, Plan};
use crate::value::is_bf;

/// `GENERAL` (RB_GENERAL=1)
pub fn general() -> bool {
    static V: OnceLock<bool> = OnceLock::new();
    *V.get_or_init(|| env_or("RB_GENERAL", "0") == "1")
}
/// `W_MIGHT` (RB_GEN_WM) : poids de la might sur battlefield (aggro)
pub fn w_might() -> f64 {
    static V: OnceLock<f64> = OnceLock::new();
    *V.get_or_init(|| env_or("RB_GEN_WM", "0.3").trim().parse().expect("RB_GEN_WM"))
}
/// `W_HAND` (RB_GEN_WH) : poids de la main (contrôle)
pub fn w_hand() -> f64 {
    static V: OnceLock<f64> = OnceLock::new();
    *V.get_or_init(|| env_or("RB_GEN_WH", "0.3").trim().parse().expect("RB_GEN_WH"))
}

/// `deck_role(deck)` : (rôle, part d'unités ≤ 3, coût moyen)
pub fn deck_role(main: &[String]) -> (&'static str, f64, f64) {
    let (mut cost, mut cheap, mut n) = (0.0f64, 0i64, 0i64);
    for name in main {
        let Some(sp) = crate::spec::specs().get(name) else { continue };
        n += 1;
        cost += sp.e as f64;
        if sp.type_ == "Unit" && sp.e <= 3 {
            cheap += 1;
        }
    }
    let n = n.max(1);
    let (share, avg) = (cheap as f64 / n as f64, cost / n as f64);
    if share >= 0.27 && avg <= 3.4 {
        return ("aggro", share, avg);
    }
    if avg >= 3.75 {
        return ("controle", share, avg);
    }
    ("milieu", share, avg)
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
        let (role, share, avg) = deck_role(&deck.main);
        GeneralPlan { role, share, avg }
    }
}

impl Plan for GeneralPlan {
    fn name(&self) -> &str {
        "general"
    }
    fn mulligan(&self, g: &mut Game, pid: usize) -> Res<Option<Vec<O>>> {
        let h = g.p[pid].hand.clone();
        let lim = match self.role {
            "aggro" => 5,
            "milieu" => 6,
            _ => 7,
        };
        let mut by = h.clone();
        by.sort_by_key(|&c| -(g.spec(c).e as i64));
        let mut out: Vec<O> = by.iter().copied().filter(|&c| g.spec(c).e >= lim).collect();
        if self.role == "aggro" && !h.iter().any(|&c| g.spec(c).type_ == "Unit" && g.spec(c).e <= 3) {
            let extra: Vec<O> = by.iter().copied().filter(|c| !out.contains(c) && g.spec(*c).e >= 4).collect();
            out.extend(extra);
        }
        out.truncate(2);
        Ok(Some(out))
    }
    fn shape(&self, g: &mut Game, me: usize) -> Res<f64> {
        if g.winner.is_some() {
            return Ok(0.0);
        }
        if self.role == "aggro" {
            let mut t = 0;
            for u in g.units(Some(me), None) {
                if is_bf(g.o(u).loc) {
                    t += 0.max(g.might(u)?);
                }
            }
            return Ok(w_might() * t as f64);
        }
        if self.role == "controle" {
            return Ok(w_hand() * g.p[me].hand.len() as f64);
        }
        Ok(0.0)
    }
}

/// `plan_for(deck)` : plan à utiliser pour ce deck (Akali / LeBlanc : plans existants ; sinon général si RB_GENERAL=1).
pub fn plan_for(deck: &DeckList) -> Arc<dyn Plan> {
    let lg = &deck.legend;
    if lg.starts_with("Akali") {
        return plans("Gorica").unwrap();
    }
    if lg.starts_with("LeBlanc") {
        return plans("Hook tempo").unwrap();
    }
    if general() {
        Arc::new(GeneralPlan::new(deck))
    } else {
        Arc::new(NoPlan)
    }
}
