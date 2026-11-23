//! Moteur Riftbound fidèle (1v1), portage en Rust du moteur Python `riftbound/engine`.
//! Contrat et avancement : `docs/memoire/portage-rust.md`. Le moteur Python reste la référence tant que le
//! portage n'est pas fini : même graine = même partie dans les deux moteurs.

pub mod actions;
pub mod agents;
pub mod cards;
pub mod cards_helpers;
pub mod cardsets;
pub mod decks;
pub mod game;
pub mod game_board;
pub mod game_costs;
pub mod game_events;
pub mod game_turn;
pub mod json;
pub mod lang;
pub mod match_;
pub mod pyrandom;
pub mod regex;
pub mod replay;
pub mod spec;
pub mod trace;
pub mod value;
