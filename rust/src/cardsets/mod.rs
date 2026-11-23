//! Modules de cartes (`cardsets/`), enregistrés dans l'ordre de `cardsets/__init__.py : MODULES`.
use crate::cards::Registry;

pub mod order;

pub fn register_all(r: &mut Registry) {
    order::register(r);
}
