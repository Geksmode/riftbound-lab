//! `riftbound/train/t_rand.py` : parties au hasard sur la table d'entraînement (API de `train`, comme la page), pour
//! trouver les erreurs des règles. `t_rand [parties]` ; `RB_JOBS=n` fixe le nombre de fils (1 : une après l'autre).
//!
//! Chaque partie repart de zéro (`train::new` remet les compteurs d'objets) et l'état de la table est propre à
//! chaque fil : jouées dans n'importe quel fil, elles sont identiques. Résultats rassemblés dans l'ordre des graines.
use std::collections::BTreeMap;
use std::time::Instant;

use super::pool;
use crate::json::{self, Json};
use crate::jo;
use crate::pyrandom::PyRandom;
use crate::train;

struct Cat {
    cards: Vec<Json>,
    leg: Vec<usize>,
    bf: Vec<String>,
}

fn cat() -> &'static Cat {
    static C: std::sync::OnceLock<Cat> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        let j = json::parse(&train::catalog()).expect("catalogue illisible");
        let cards = j.g("cards").arr().to_vec();
        let leg = (0..cards.len()).filter(|&i| cards[i].g("t").str() == Some("Legend") && cards[i].g("m").truthy()).collect();
        // BF : ordre du catalogue (non trié, contrairement à exp_general.BFS)
        let bf = cards
            .iter()
            .filter(|c| c.g("t").str() == Some("Battlefield") && c.g("m").truthy() && !c.g("ban").truthy())
            .map(|c| c.g("n").str().unwrap().to_string())
            .collect();
        Cat { cards, leg, bf }
    })
}

fn strs(j: &Json) -> Vec<String> {
    j.arr().iter().map(|x| x.str().unwrap_or("").to_string()).collect()
}

/// `rdeck(rng)` : deck légal au hasard (dict du format de la page).
pub fn rdeck(rng: &mut PyRandom) -> Json {
    let cat = cat();
    for _ in 0..50 {
        let lg = &cat.cards[*rng.choice(&cat.leg)];
        let dom = strs(lg.g("d"));
        let ltg = strs(lg.g("tg"));
        let within = |c: &Json| strs(c.g("d")).iter().all(|x| dom.contains(x));
        let ch: Vec<&Json> = cat
            .cards
            .iter()
            .filter(|c| {
                c.g("t").str() == Some("Unit")
                    && c.g("s").str() == Some("Champion")
                    && c.g("m").truthy()
                    && strs(c.g("tg")).iter().any(|t| ltg.contains(t))
                    && within(c)
            })
            .collect();
        let pool: Vec<String> = cat
            .cards
            .iter()
            .filter(|c| {
                matches!(c.g("t").str(), Some("Unit" | "Spell" | "Gear"))
                    && c.g("m").truthy()
                    && within(c)
                    && !matches!(c.g("s").str(), Some("Champion" | "Signature"))
                    && !c.g("ban").truthy()
                    && !c.g("tx").str().unwrap_or("").contains("Unique")
            })
            .map(|c| c.g("n").str().unwrap().to_string())
            .collect();
        if ch.is_empty() || pool.len() < 14 {
            continue;
        }
        let mut main: Vec<String> = vec![];
        while main.len() < 39 {
            let n = rng.choice(&pool).clone();
            if main.iter().filter(|x| **x == n).count() < 3 {
                main.push(n);
            }
        }
        let champ = rng.choice(&ch).g("n").str().unwrap().to_string();
        let r0 = format!("{} Rune", dom[0]);
        let r1 = format!("{} Rune", dom[dom.len() - 1]);
        let runes: Vec<String> = std::iter::repeat_n(r0, 6).chain(std::iter::repeat_n(r1, 6)).collect();
        let bfs = rng.sample(&cat.bf, 3);
        let d = jo! {
            "name" => "r", "legend" => lg.g("n").str().unwrap().to_string(), "champion" => champ, "main" => main,
            "runes" => runes, "battlefields" => bfs,
        };
        if !train::validate(&d).iter().any(|x| !x.starts_with('⚠')) {
            return d;
        }
    }
    panic!("RuntimeError: no deck")
}

/// Résultat d'une partie : questions vues ((type, titre) -> nombre), (titre, 3 premières options), erreur.
pub type Res = (Vec<((String, String), i64)>, Vec<(String, Vec<Json>)>, Option<(i64, String, String, String)>);

fn call(f: &str, a: Vec<Json>) -> Result<Json, String> {
    let s = train::call(f, &a)?;
    json::parse(&s).map_err(|e| format!("JSONDecodeError: {e}"))
}

/// `one(seed)`
pub fn one(seed: i64) -> Res {
    let mut kinds: Vec<((String, String), i64)> = vec![];
    let mut lb: Vec<(String, Vec<Json>)> = vec![];
    let mut rng = PyRandom::new(seed as i128);
    let (a, b) = {
        let a = rdeck(&mut rng);
        let b = rdeck(&mut rng);
        (a, b)
    };
    let (la, lbn) = (a.g("legend").str().unwrap_or("").to_string(), b.g("legend").str().unwrap_or("").to_string());
    let r = pool::attempt(|| -> Result<(), String> {
        train::call("new", &[Json::Int(seed), Json::Null, Json::Null, Json::Null, Json::Str(a.dumps(true)), Json::Str(b.dumps(true))])?;
        let mut v = call("step", vec![])?;
        let mut n = 0;
        while v.g("winner").is_null() && n < 4000 {
            n += 1;
            if v.g("ask").truthy() {
                let ask = v.g("ask");
                let k = ask.g("kind").str().unwrap_or("").to_string();
                let title = ask.g("title").str().unwrap_or("").to_string();
                match kinds.iter_mut().find(|e| e.0 == (k.clone(), title.clone())) {
                    Some(e) => e.1 += 1,
                    None => kinds.push(((k.clone(), title.clone()), 1)),
                }
                let opts = ask.g("options").arr();
                lb.push((title, opts.iter().take(3).cloned().collect()));
                let x = if k == "mulligan" { Json::Arr(vec![]) } else { Json::Int(rng.randrange(opts.len() as i64)) };
                v = call("answer", vec![Json::Str(x.dumps(true))])?;
            } else if v.g("dec").truthy() {
                if n % 40 == 0 {
                    call("hint", vec![Json::Int(3)])?;
                }
                let k = v.g("dec").g("options").arr().len();
                v = call("act", vec![Json::Int(rng.randrange(k as i64))])?;
            } else {
                v = call("step", vec![])?;
            }
        }
        Ok(())
    });
    let err = match r {
        Ok(Ok(())) => None,
        Ok(Err(e)) | Err(e) => {
            let t: Vec<char> = e.chars().collect();
            let tail: String = t[t.len().saturating_sub(600)..].iter().collect();
            Some((seed, la, lbn, tail))
        }
    };
    (kinds, lb, err)
}

fn repr_str(s: &str) -> String {
    super::py::PyVal::Str(s.to_string()).py_repr()
}

/// `__main__`
pub fn main(a: &[String]) -> i32 {
    let n: i64 = a.first().map(|x| x.parse().expect("nombre de parties")).unwrap_or(20);
    let jobs: usize = match std::env::var("RB_JOBS") {
        Ok(j) if !j.is_empty() => j.parse().expect("RB_JOBS"),
        _ => pool::cpu_count(),
    };
    let t0 = Instant::now();
    let seeds: Vec<i64> = (0..n).collect();
    let res = pool::map(jobs, &seeds, |&s| one(s));
    let mut kinds: BTreeMap<(String, String), i64> = BTreeMap::new();
    let (mut bad, mut lb) = (vec![], vec![]);
    for (k, l, err) in res {
        for (key, c) in k {
            *kinds.entry(key).or_insert(0) += c;
        }
        lb.extend(l);
        if let Some(e) = err {
            bad.push(e);
        }
    }
    println!("{} parties {} s, erreurs {}", n, super::py::round0(t0.elapsed().as_secs_f64()), bad.len());
    for (s, la, lbn, tb) in bad.iter().take(3) {
        println!("({}, {}, {}, {})", s, repr_str(la), repr_str(lbn), repr_str(tb));
    }
    let mut titles: Vec<String> = kinds.keys().map(|k| k.1.clone()).collect();
    titles.sort();
    titles.dedup();
    println!("[{}]", titles.iter().take(60).map(|t| repr_str(t)).collect::<Vec<_>>().join(", "));
    let k = 25.min(lb.len());
    for i in PyRandom::new(1).sample_indices(lb.len(), k) {
        let (t, o) = &lb[i];
        println!("({}, [{}])", repr_str(t), o.iter().map(train::py_repr_json).collect::<Vec<_>>().join(", "));
    }
    0
}
