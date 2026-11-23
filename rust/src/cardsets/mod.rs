//! Modules de cartes (`cardsets/`), enregistrés dans l'ordre de `cardsets/__init__.py : MODULES`.
use crate::cards::Registry;

pub mod battlefields;
pub mod legends;

pub fn register_all(r: &mut Registry) {
    battlefields::register(r);
    legends::register(r);
}
