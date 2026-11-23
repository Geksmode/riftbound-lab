//! Modules de cartes (`cardsets/`), enregistrés dans l'ordre de `cardsets/__init__.py : MODULES`.
use crate::cards::Registry;

pub mod radiance;

pub fn register_all(r: &mut Registry) {
    radiance::register(r);
}
