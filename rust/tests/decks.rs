//! Les paquets chargés en Rust sont ceux de `decks.load` (Python).
use riftbound::decks::{decks, load};

#[test]
fn identiques_a_python() {
    let data = include_str!("data/decks.txt");
    let mut n = 0;
    for line in data.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let d = load(f[0], None);
        let got = [
            d.key.clone(), d.deck.legend.clone(), d.deck.champion.clone().unwrap_or_default(), d.deck.main.join("|"),
            d.deck.runes.join("|"), d.deck.battlefield.clone(), d.battlefields.join("|"), d.sideboard.join("|"),
        ]
        .join("\t");
        assert_eq!(got, line);
        n += 1;
    }
    assert_eq!(n, decks().len());
}
