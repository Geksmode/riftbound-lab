//! Aides de `cards.py` utilisées par le moteur et les modules de cartes. Portage en cours.
use crate::game::{Game, Res, O};

/// `disarm_trigger(g, u, n)` : [Disarm X] (831).
pub fn disarm_trigger(_g: &mut Game, _u: O, _n: i64) -> Res<()> {
    todo!("portage : cards.disarm_trigger")
}
