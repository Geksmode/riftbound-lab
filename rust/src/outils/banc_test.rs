//! `banc/test_banc.py` : juge du banc d'erreurs (`outils::banc`), tests qui ne demandent pas de mise en place de
//! scénario. Les deux tests à scénario de Python (Hextech Ray sur l'unité qu'elle ne peut pas tuer, Cleave sur une unité
//! adverse) dépendent du `testkit` (`new`, `put`, `hand`, `runes`, `opt`), pas encore porté : ils restent à porter avec
//! lui. `cargo test banc_test`.
#![cfg(test)]
use std::sync::Arc;

use super::banc::{dec, play, rescore};
use crate::json::Json;
use crate::value::Value;

#[test]
fn le_juge_ne_change_pas_la_partie() {
    let (a, pos) = play(4000001, "defaut", "sh", true, None);
    let (b, _) = play(4000001, "defaut", "sh", false, None);
    assert_eq!(a.g("script"), b.g("script"));
    assert_eq!(a.g("winner"), b.g("winner"));
    assert!(!pos.is_empty() && pos.iter().all(|p| p.get("regret").is_some()));
}

#[test]
fn le_banc_rejoue_atteint_chaque_position_et_garde_les_choix_inchanges() {
    let (a, pos) = play(4000001, "defaut", "sh", true, None);
    let r = rescore(&a, &pos, "sh", None);
    assert!(r.len() == pos.len() && !r.iter().any(|x| x.get("err").is_some()));
    for x in &r {
        if !x.g("changed").truthy() && x.g("typ").str() == Some("ask") {
            assert_eq!(x.g("new"), x.g("old"), "{x:?}");
        }
    }
}

#[test]
fn reponses_aux_asks_aller_retour() {
    let opts = vec![Value::from("a"), Value::from("b"), Value::Bool(true)];
    // un indice
    assert_eq!(dec(&Json::Arr(vec!["i".into(), 1usize.into()]), &opts), Some(1));
    // hors bornes : l'agent décide lui-même
    assert_eq!(dec(&Json::Arr(vec!["i".into(), 7usize.into()]), &opts), None);
    // une valeur
    assert_eq!(dec(&Json::Arr(vec!["v".into(), Json::Bool(true)]), &opts), Some(2));
    assert_eq!(dec(&Json::Null, &opts), None);
    let _ = Arc::new(0);
}
