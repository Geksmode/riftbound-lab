//! Base de cartes (`game.SPEC`) : lue depuis `riftbound/cards/cards_unique.csv`, plus les jetons (règle 187) et les
//! bonus de might des équipements (`EQUIP_BONUS`). Traduction de l'en-tête de `game.py`.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

pub const DOMAINS: [&str; 6] = ["Fury", "Calm", "Mind", "Body", "Chaos", "Order"];
pub const VICTORY: i32 = 8;
pub const BARON_PIT: &str = "Baron Pit";

#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub name: String,
    pub type_: String,
    pub super_: String,
    pub domains: Vec<String>,
    pub e: i32,
    pub p: i32,
    pub might: Option<i32>,
    pub tags: BTreeSet<String>,
    pub text: String,
    pub keywords: Vec<String>,
    /// unit gear (`Obj.spec` d'une unit gear : `gear=True`)
    pub gear: bool,
}

/// Le CSV de la base de cartes, embarqué (le moteur WebAssembly n'a pas de système de fichiers).
const CSV: &str = include_str!("../../riftbound/cards/cards_unique.csv");

/// Lecture d'un CSV au format du module `csv` de Python (guillemets doublés, champs entre guillemets sur plusieurs
/// lignes).
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    let mut any = false;
    while let Some(c) = chars.next() {
        any = true;
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                any = false;
            }
            _ => field.push(c),
        }
    }
    if any {
        row.push(field);
        rows.push(row);
    }
    rows
}

fn split_bar(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split('|').filter(|x| !x.is_empty()).map(|x| x.to_string())
}

fn token(name: &str, type_: &str, might: Option<i32>, tags: &[&str], text: &str) -> Spec {
    Spec {
        name: name.into(),
        type_: type_.into(),
        super_: "Token".into(),
        domains: vec![],
        e: 0,
        p: 0,
        might,
        tags: tags.iter().map(|t| t.to_string()).collect(),
        text: text.into(),
        keywords: vec![],
        gear: false,
    }
}

fn build() -> BTreeMap<String, Spec> {
    let rows = parse_csv(CSV);
    let head = &rows[0];
    let col = |n: &str| head.iter().position(|h| h == n).unwrap_or_else(|| panic!("colonne {n} absente"));
    let (c_name, c_type, c_super, c_dom) = (col("name"), col("type"), col("supertype"), col("domain"));
    let (c_e, c_p, c_m, c_tags, c_text, c_kw) =
        (col("energy_cost"), col("power_cost"), col("might"), col("tags"), col("text"), col("keywords"));
    let mut out = BTreeMap::new();
    for r in &rows[1..] {
        if r.len() < head.len() {
            continue;
        }
        let int = |s: &str| if s.is_empty() { 0 } else { s.parse::<i32>().unwrap() };
        out.insert(
            r[c_name].clone(),
            Spec {
                name: r[c_name].clone(),
                type_: r[c_type].clone(),
                super_: r[c_super].clone(),
                domains: split_bar(&r[c_dom]).collect(),
                e: int(&r[c_e]),
                p: int(&r[c_p]),
                might: if r[c_m].is_empty() { None } else { Some(r[c_m].parse().unwrap()) },
                tags: split_bar(&r[c_tags]).collect(),
                text: r[c_text].clone(),
                keywords: split_bar(&r[c_kw]).collect(),
                gear: false,
            },
        );
    }
    // jetons (règle 187)
    out.insert("Mech".into(), token("Mech", "Unit", Some(3), &["Mech"], ""));
    out.insert("Reflection".into(), token("Reflection", "Unit", Some(0), &[], ""));
    out.insert("Gold".into(), token("Gold", "Gear", None, &[], ""));
    // règle 187.12 (Core Rules 2026-10-09, Radiance)
    out.insert(
        "Bomb".into(),
        token(
            "Bomb",
            "Gear",
            None,
            &["Bomb"],
            "[Deploy] [Deathknell][>] Deal 2 to an enemy unit here and kill all other Bombs you control here.",
        ),
    );
    // noms de jetons imprimés dans les textes, sans ligne à eux
    for (n, m, t, x) in [
        ("Recruit", 1, &["Recruit"][..], ""),
        ("Sand Soldier", 2, &[][..], ""),
        ("Tentacle", 1, &["Bilgewater"][..], ""),
        (
            "Shadow Clone",
            0,
            &[][..],
            "When I attack, you may banish a unit from your trash. If you do, give me [Assault 4] this turn.",
        ),
    ] {
        out.entry(n.to_string()).or_insert_with(|| token(n, "Unit", Some(m), t, x));
    }
    out
}

/// `game.SPEC`.
pub fn specs() -> &'static BTreeMap<String, Spec> {
    static S: OnceLock<BTreeMap<String, Spec>> = OnceLock::new();
    S.get_or_init(build)
}

pub fn spec(name: &str) -> &'static Spec {
    specs().get(name).unwrap_or_else(|| panic!("carte inconnue : {name}"))
}

/// Spec d'une unit gear (`Obj.spec` quand `ugear`) : une Unit dont la Might vaut son coût en énergie.
pub fn ugear_specs() -> &'static BTreeMap<String, Spec> {
    static S: OnceLock<BTreeMap<String, Spec>> = OnceLock::new();
    S.get_or_init(|| {
        specs()
            .iter()
            .map(|(k, v)| {
                let mut s = v.clone();
                s.type_ = "Unit".into();
                s.might = Some(v.e);
                s.gear = true;
                (k.clone(), s)
            })
            .collect()
    })
}

/// `EQUIP_BONUS` : le bonus de might des équipements est imprimé dans une icône absente de la base (règle 718.4).
pub fn equip_bonus(name: &str) -> i32 {
    match name {
        "Long Sword" => 2,
        "Sterak's Gage" => 3,
        "Pendulum Blade" => 1,
        _ => 0,
    }
}
