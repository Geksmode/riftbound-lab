//! `check_decks <fichier.json>` : valide des decks lus à l'exécution (pas ceux de `decks.json`, compilés dans le
//! binaire). Entrée : tableau JSON de `{key, legend, champion, main, runes, battlefields, sideboard}` (noms de cartes,
//! runes sans le mot « Rune »). Sortie : une ligne par deck, `clé<TAB>problème | problème…` (vide = deck valide),
//! d'après `train::validate` (règles de construction 103, sideboard 601.1.c, cartes non modélisées, cartes bannies
//! signalées par « ⚠ »). Utilisé par `riftbound/decks/check_raw.py`.
use riftbound::json;

fn main() {
    let path = std::env::args().nth(1).expect("usage : check_decks <fichier.json>");
    let text = std::fs::read_to_string(&path).expect("fichier illisible");
    let j = json::parse(&text).expect("JSON illisible");
    for d in j.arr() {
        let key = d.get("key").and_then(|k| k.str()).unwrap_or("?");
        println!("{key}\t{}", riftbound::train::validate(d).join(" | "));
    }
}
