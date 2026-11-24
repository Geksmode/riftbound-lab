//! Listes de tournoi (`decks.py`) : `riftbound/decks/decks.json`, embarqué.
use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::game::DeckList;
use crate::json::{parse, Json};

const DECKS_JSON: &str = include_str!("../../riftbound/decks/decks.json");

/// Paquet chargé : la liste jouable et ce qui l'accompagne (`decks.load`).
#[derive(Clone, Debug)]
pub struct LoadedDeck {
    pub key: String,
    pub deck: DeckList,
    pub battlefields: Vec<String>,
    pub sideboard: Vec<String>,
}

/// `DECKS` : clé (nom du fichier sans `.txt`) -> entrée de decks.json.
pub fn decks() -> &'static BTreeMap<String, Json> {
    static D: OnceLock<BTreeMap<String, Json>> = OnceLock::new();
    D.get_or_init(|| {
        let j = parse(DECKS_JSON).expect("decks.json illisible");
        j.arr()
            .iter()
            .map(|d| (d.get("file").and_then(|f| f.str()).unwrap().replace(".txt", ""), d.clone()))
            .collect()
    })
}

/// `load(key, battlefield)`
pub fn load(key: &str, battlefield: Option<&str>) -> LoadedDeck {
    let d = decks().get(key).unwrap_or_else(|| panic!("paquet inconnu : {key}"));
    let (mut legend, mut champion) = (None, None);
    let (mut main, mut runes, mut bfs, mut side) = (vec![], vec![], vec![], vec![]);
    for c in d.get("cards").map(|c| c.arr()).unwrap_or(&[]) {
        let n = c
            .get("name_db")
            .and_then(|x| x.str())
            .filter(|s| !s.is_empty())
            .or_else(|| c.get("name").and_then(|x| x.str()))
            .unwrap()
            .to_string();
        let q = c.get("qty").and_then(|x| x.num()).unwrap_or(1.0) as usize;
        match c.get("section").and_then(|x| x.str()).unwrap_or("") {
            "legend" => legend = Some(n),
            "champion" => champion = Some(n),
            "main" => main.extend(std::iter::repeat_n(n, q)),
            "runes" => runes.extend(std::iter::repeat_n(n.replace(" Rune", ""), q)),
            "battlefields" => bfs.push(n),
            "sideboard" => side.extend(std::iter::repeat_n(n, q)),
            _ => {}
        }
    }
    let bf = battlefield.map(|b| b.to_string()).unwrap_or_else(|| bfs[0].clone());
    LoadedDeck {
        key: key.to_string(),
        deck: DeckList { legend: legend.expect("paquet sans légende"), champion, main, runes, battlefield: bf },
        battlefields: bfs,
        sideboard: side,
    }
}
