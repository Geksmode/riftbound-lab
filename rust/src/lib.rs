//! Moteur Riftbound fidèle (1v1), portage en Rust du moteur Python `riftbound/engine`.
//! Contrat et avancement : `docs/memoire/portage-rust.md`. Le moteur Python reste la référence tant que le
//! portage n'est pas fini : même graine = même partie dans les deux moteurs.

pub mod pyrandom;
pub mod spec;
