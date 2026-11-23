//! Modules de cartes (`cardsets/`), enregistrés dans l'ordre de `cardsets/__init__.py : MODULES`.
use crate::cards::Registry;

pub mod fury;

pub fn register_all(r: &mut Registry) {
    fury::register(r);
}
