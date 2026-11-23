//! Langue des textes montrés au joueur (`lang.py` : `tr(fr, en)`, `en()`). Français par défaut.
use std::sync::atomic::{AtomicBool, Ordering};

static EN: AtomicBool = AtomicBool::new(false);

/// `lang.en()`
pub fn en() -> bool {
    EN.load(Ordering::Relaxed)
}

/// `lang.set_lang` : true pour l'anglais.
pub fn set_en(v: bool) {
    EN.store(v, Ordering::Relaxed)
}

/// `lang.tr(fr, en)`
pub fn tr(fr: impl Into<String>, en_: impl Into<String>) -> String {
    if en() {
        en_.into()
    } else {
        fr.into()
    }
}
