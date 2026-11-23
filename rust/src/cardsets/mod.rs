//! Modules de cartes (`cardsets/`), enregistrés dans l'ordre de `cardsets/__init__.py : MODULES`, après les cartes
//! du pool de `cards.py`.
use crate::cards::Registry;

pub mod order;
pub mod pool;

pub fn register_all(r: &mut Registry) {
    // cards.py enregistre ses cartes du pool avant de charger cardsets
    pool::register(r);
    // MODULES : battlefields, body, calm, chaos, fury, legends, mind, order, radiance
    order::register(r);
}
