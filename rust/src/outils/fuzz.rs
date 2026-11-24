//! `fuzz_cards.py` : joue des parties à agents aléatoires sur des paquets qui contiennent les cartes données (par
//! défaut : toutes les cartes « mots-clés seulement » modélisées automatiquement, ou les cartes d'un module). Toute
//! action légale doit s'exécuter ; la même graine doit donner la même partie.
//!
//! ```text
//!   fuzz_cards                       # 50 parties, paquets avec les cartes « mots-clés seulement »
//!   fuzz_cards 100 --module my_batch # 100 parties avec les cartes enregistrées par cardsets/my_batch
//!   fuzz_cards 20 "Sunlit Guardian" "Pouty Poro"
//! ```
//! Python joue les parties l'une après l'autre ; ici elles sont réparties sur des fils et affichées dans l'ordre des
//! graines (chaque partie ne dépend que de sa graine).
use std::collections::BTreeSet;

use super::md5::Md5;
use super::pool;
use super::py::PyVal;
use crate::agents::RandomAgent;
use crate::cards::registry;
use crate::cards_helpers::{is_token, NOT_CARDS};
use crate::game::{Agent, Counters, DeckList, Game};
use crate::spec::{specs, DOMAINS, VICTORY};

/// Nom de module au format Python (`cardsets.body`, `auto_keywords`, None pour les cartes de cards.py).
fn py_module(m: Option<&str>) -> Option<String> {
    let m = m?;
    Some(if m.starts_with("cardsets.") || m == "auto_keywords" { m.to_string() } else { format!("cardsets.{m}") })
}

/// `cards.AUTO` : cartes « mots-clés seulement » enregistrées automatiquement, triées.
pub fn auto_cards() -> Vec<String> {
    registry().impls.iter().filter(|(_, v)| v.module == Some("auto_keywords")).map(|(k, _)| k.clone()).collect()
}

/// `playable(name)` (KeyError en Python pour un nom absent de la base : panique ici).
pub fn playable(name: &str) -> bool {
    let sp = specs().get(name).unwrap_or_else(|| panic!("KeyError: {name:?}"));
    matches!(sp.type_.as_str(), "Unit" | "Spell" | "Gear")
        && !is_token(name)
        && registry().impls.contains_key(name)
        && !NOT_CARDS.contains(&name)
}

/// `main[:k]` de Python (k peut être négatif).
fn py_head<T: Clone>(v: &[T], k: i64) -> Vec<T> {
    let n = v.len() as i64;
    let k = if k < 0 { (n + k).max(0) } else { k.min(n) };
    v[..k as usize].to_vec()
}

/// `build(base_key, names, rng)` : une liste de tournoi dont au plus 18 cartes du main sont remplacées par des
/// copies de `names`, runes couvrant leurs domaines.
pub fn build(base_key: &str, names: &[String], rng: &mut crate::pyrandom::PyRandom) -> DeckList {
    let mut d = crate::decks::load(base_key, None).deck;
    let names: Vec<&String> = names.iter().filter(|n| playable(n)).collect();
    if names.is_empty() {
        return d;
    }
    let pick: Vec<String> = (0..18).map(|_| rng.choice(&names).to_string()).collect();
    let mut main = d.main.clone();
    rng.shuffle(&mut main);
    let mut m = py_head(&main, main.len() as i64 - pick.len() as i64);
    m.extend(pick.iter().cloned());
    d.main = m;
    let doms: BTreeSet<&str> = pick
        .iter()
        .flat_map(|n| specs()[n.as_str()].domains.iter().map(|s| s.as_str()))
        .filter(|x| DOMAINS.contains(x))
        .collect();
    let doms: Vec<&str> = doms.into_iter().collect();
    if !doms.is_empty() {
        for i in 0..d.runes.len().min(6) {
            d.runes[i] = doms[i % doms.len()].to_string();
        }
    }
    d
}

/// `play(seed, names)` : la partie jouée (compteurs d'objets remis à zéro : ils apparaissent dans le journal).
pub fn play(seed: i64, names: &[String]) -> Game {
    let mut rng = crate::pyrandom::PyRandom::new(seed as i128);
    let keys: Vec<&String> = crate::decks::decks().keys().collect();
    let n = keys.len() as i64;
    let d0 = build(keys[(seed % n) as usize], names, &mut rng);
    let d1 = build(keys[((seed + 5) % n) as usize], names, &mut rng);
    let agents: Vec<Box<dyn Agent>> =
        vec![Box::new(RandomAgent::new(seed as i128)), Box::new(RandomAgent::new(seed as i128 + 7919))];
    let mut g = Game::new(&[d0, d1], agents, None, seed as i128, true, VICTORY as i64, std::sync::Arc::new(Counters::default()));
    for pid in 0..2 {
        let mut a = std::mem::replace(&mut g.agents[pid], Box::new(crate::game_events::FirstChoice));
        a.start(&mut g);
        g.agents[pid] = a;
    }
    let mut k = 0;
    loop {
        let Some(d) = g.advance(false) else { break };
        if k > 4000 {
            break;
        }
        let mut a = std::mem::replace(&mut g.agents[d.player], Box::new(crate::game_events::FirstChoice));
        let r = a.decide(&mut g, &d);
        g.agents[d.player] = a;
        match r {
            Ok(act) => g.apply(&act),
            Err(_) => break,
        }
        k += 1;
    }
    g
}

/// `digest(g)` : MD5 du journal, numéros d'objets retirés (`re.sub(r"#\d+", "", l)`).
pub fn digest(g: &Game) -> String {
    let mut h = Md5::new();
    for (i, l) in g.lines.iter().enumerate() {
        if i > 0 {
            h.update(b"\n");
        }
        let mut s = String::with_capacity(l.len());
        let mut it = l.chars().peekable();
        while let Some(c) = it.next() {
            if c == '#' && it.peek().is_some_and(|x| x.is_ascii_digit()) {
                while it.peek().is_some_and(|x| x.is_ascii_digit()) {
                    it.next();
                }
            } else {
                s.push(c);
            }
        }
        h.update(s.as_bytes());
    }
    h.hexdigest()
}

/// Cartes jouées dans la partie (`play_<nom>` de `g.stats`).
pub fn played(g: &Game) -> Vec<String> {
    g.stats.0.iter().filter_map(|(k, _)| k.strip_prefix("play_").map(|s| s.to_string())).collect()
}

/// Résultat d'une graine : (empreinte, cartes jouées) ou le message de l'exception.
fn one(seed: i64, names: &[String]) -> Result<(String, Vec<String>), String> {
    pool::attempt(|| {
        let g = play(seed, names);
        let h1 = digest(&g);
        if seed < 5 {
            let h2 = digest(&play(seed, names));
            assert!(h1 == h2, "seed {seed}: two runs differ");
        }
        (h1, played(&g))
    })
}

/// `main(argv)` : renvoie le nombre d'exceptions (`Err` : sortie « not modelled »).
pub fn main(argv: &[String]) -> Result<usize, String> {
    let first_num = argv.first().is_some_and(|a| !a.is_empty() && a.chars().all(|c| c.is_ascii_digit()));
    let n: i64 = if first_num { argv[0].parse().unwrap() } else { 50 };
    let rest = if first_num { &argv[1..] } else { argv };
    let names: Vec<String> = if rest.first().map(|s| s.as_str()) == Some("--module") {
        let want = format!("cardsets.{}", rest[1]);
        registry()
            .impls
            .iter()
            .filter(|(_, v)| py_module(v.module).as_deref() == Some(want.as_str()))
            .map(|(k, _)| k.clone())
            .collect()
    } else if !rest.is_empty() {
        rest.to_vec()
    } else {
        auto_cards()
    };
    let missing: Vec<PyVal> =
        names.iter().filter(|x| !registry().impls.contains_key(x.as_str())).map(|x| PyVal::Str(x.clone())).collect();
    if !missing.is_empty() {
        return Err(format!("not modelled: {}", PyVal::List(missing).py_repr()));
    }
    let seeds: Vec<i64> = (0..n).collect();
    let out = pool::map(pool::cpu_count(), &seeds, |&s| one(s, &names));
    let mut errors = 0;
    let mut played_all: BTreeSet<String> = BTreeSet::new();
    for (seed, r) in seeds.iter().zip(out) {
        match r {
            Ok((_, p)) => played_all.extend(p),
            Err(e) => {
                errors += 1;
                println!("seed {seed}: EXCEPTION");
                eprintln!("{e}");
            }
        }
    }
    let set: BTreeSet<&String> = names.iter().collect();
    let hit = set.iter().filter(|x| played_all.contains(x.as_str())).count();
    println!("{n} games, {errors} exceptions; {hit}/{} of the cards were played at least once", names.len());
    let never: Vec<&str> =
        set.iter().filter(|x| playable(x) && !played_all.contains(x.as_str())).map(|x| x.as_str()).collect();
    if !never.is_empty() {
        println!("never played: {}", never.join(", "));
    }
    Ok(errors)
}
