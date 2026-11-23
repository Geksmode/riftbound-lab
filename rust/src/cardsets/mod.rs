//! Modules de cartes (`cardsets/`), enregistrés dans l'ordre de `cardsets/__init__.py : MODULES`, après les cartes
//! du pool de `cards.py`.
use crate::cards::Registry;

pub mod battlefields;
pub mod body;
pub mod calm;
pub mod chaos;
pub mod fury;
pub mod legends;
pub mod mind;
pub mod order;
pub mod pool;
pub mod radiance;

pub fn register_all(r: &mut Registry) {
    // cards.py : titres des questions, runes et Gold // Buff, cartes du pool, puis cardsets
    crate::cards_helpers::register_core_ask_text(r);
    crate::cards_helpers::register_core_cards(r);
    pool::register(r);
    // MODULES : battlefields, body, calm, chaos, fury, legends, mind, order, radiance
    battlefields::register(r);
    body::register(r);
    calm::register(r);
    chaos::register(r);
    fury::register(r);
    legends::register(r);
    mind::register(r);
    order::register(r);
    radiance::register(r);
    // cartes « mots-clés seulement » non modélisées par un module (register_keyword_only_cards)
    crate::cards_helpers::register_auto(r);
}
