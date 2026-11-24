//! Version du moteur (`version.py`), pour étiqueter chaque résultat de simulation (règle 2 de `CLAUDE.md` : ne jamais
//! agréger deux versions).
//!
//! Le moteur Rust est une **nouvelle version** du moteur : son identifiant est le MD5 de ses sources (10 caractères,
//! même recette que Python : chemin relatif + contenu, fichiers .rs de `rust/src/` hors `src/bin/`, calculé à la
//! compilation par `build.rs`) suivi de `-rust`, par exemple `3f9c0a12be-rust`. Il ne peut donc jamais être égal à
//! un `engine_md5` du moteur Python (10 caractères hexadécimaux sans suffixe) : `compare.py` comme `compare` (Rust)
//! refusent de comparer ou de regrouper des chiffres des deux moteurs.
use super::py::PyVal;

/// Suffixe des versions du moteur Rust.
pub const SUFFIXE: &str = "-rust";

/// `engine_md5()` du moteur Rust : MD5 des sources (10 caractères) + `-rust`.
pub fn engine_md5() -> String {
    format!("{}{}", env!("RB_ENGINE_SRC_MD5"), SUFFIXE)
}

/// Version du moteur Rust ?
pub fn is_rust(v: &str) -> bool {
    v.ends_with(SUFFIXE)
}

/// `scores(entry)` : {graine: score} d'un résultat (per_seed des expériences et du manager, per_pair
/// d'exp_general), dans l'ordre d'insertion d'un dict Python (une graine répétée garde sa première place).
pub fn scores(entry: &PyVal) -> Vec<(i64, f64)> {
    let rows = [entry.get("per_seed"), entry.get("per_pair")]
        .into_iter()
        .flatten()
        .find(|v| v.truthy())
        .map(|v| v.as_list().to_vec())
        .unwrap_or_default();
    let mut out: Vec<(i64, f64)> = vec![];
    for r in rows {
        let r = r.as_list();
        let (k, v) = (r[0].as_i64(), r[1].as_f64());
        if let Some(e) = out.iter_mut().find(|e| e.0 == k) {
            e.1 = v;
        } else {
            out.push((k, v));
        }
    }
    out
}

/// `stamp(entry)` : ajoute la version du moteur et le résumé des graines (à appeler avant l'écriture).
pub fn stamp(entry: &mut PyVal) {
    entry.set("engine_md5", PyVal::Str(engine_md5()));
    let mut s: Vec<i64> = scores(entry).into_iter().map(|x| x.0).collect();
    s.sort();
    if !s.is_empty() {
        entry.set("seeds", crate::pydict! {"first" => s[0], "last" => s[s.len() - 1], "n" => s.len() as i64});
    }
}
