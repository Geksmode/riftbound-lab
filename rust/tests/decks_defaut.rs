//! Decks par défaut de l'IA (`train::default_decks`) : chaque légende en a un, il est jouable par le moteur et ne
//! contient aucune carte bannie en Standard (ban list de `cards_unique.csv`, colonne `banned_standard`).
use riftbound::json::Json;
use riftbound::train;

#[test]
fn legaux_et_complets() {
    let cat = riftbound::json::parse(&train::catalog()).expect("catalogue illisible");
    let legendes: Vec<String> = cat
        .g("cards")
        .arr()
        .iter()
        .filter(|c| c.g("t").str() == Some("Legend") && c.g("m").truthy())
        .map(|c| c.g("n").str().unwrap().to_string())
        .collect();
    let dflt = train::default_decks();
    let mut pb: Vec<String> = vec![];
    for lg in &legendes {
        match dflt.get(lg) {
            None => pb.push(format!("{lg} : pas de deck par défaut")),
            Some(k) => {
                let d: Json = train::tload(k).json();
                let e = train::validate(&d);
                if !e.is_empty() {
                    pb.push(format!("{lg} ({k}) : {}", e.join(" | ")));
                }
            }
        }
    }
    assert!(pb.is_empty(), "decks par défaut à corriger :\n{}", pb.join("\n"));
    assert_eq!(dflt.len(), legendes.len());
}
