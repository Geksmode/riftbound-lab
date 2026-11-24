//! `combats_parties.py` : combats où l'IA est attaquante dans les parties de la table (2026-10-11, question de
//! l'utilisateur : « pourquoi l'IA enclenche des combats perdants ? »).
//!
//! ```text
//!   combats_parties riftbound/train/games/games/fichier.json [...]  [--json]
//! ```
//! Chaque partie est rejouée ; on note chaque combat où l'IA (P1) est l'attaquant (464.2.c.1) : unités et Might au
//! début du showdown et au moment des dégâts, issue, et la valeur que la recherche de l'IA donnait à son coup. En Python
//! les trois points du moteur sont enveloppés par le script ; ici ils appellent la sonde `probe` (voir ce module).
//!
//! Écarts avec le Python :
//! - le Python rejoue chaque partie avec le moteur exact de son commit (`git archive` de `riftbound/engine`) : cette
//!   copie est du Python et disparaît avec lui. Toutes les parties qui portent un `commit` sont rejouées avec le moteur
//!   actuel (celles d'avant la recherche du tour entier ne sont plus reconnues par le texte de `ai.py` : elles sont
//!   rejouées aussi) ; la colonne « rejeu NON fidèle » signale celles qui ne retombent pas sur leur résultat ;
//! - `RB_PROCS` : nombre de fils (4 par défaut), un fil par partie.
use super::pool;
use super::train_games::{load_games, replay};
use crate::jo;
use crate::json::Json;
use crate::probe;
use crate::train::py_str_json;

/// Combats d'une partie (`worker(path)`) : une ligne JSON par combat avec défenseur.
pub fn worker(game: &Json) -> Result<Vec<Json>, String> {
    probe::start();
    let r = replay(game, false, &[]);
    let (rec, picks) = probe::stop();
    let (_, _, v) = r?;
    let res = game.g("result");
    let faithful = res.truthy() && v.g("winner") == res.g("winner") && v.g("st").g("pts") == res.g("pts");
    let opp = game.g("names").arr().get(1).cloned().unwrap_or(Json::from("?"));
    let mut keys: Vec<&((i64, usize), Json)> = rec.iter().collect();
    keys.sort_by_key(|e| e.0);
    let mut out = vec![];
    for (_, r) in keys {
        if r.get("damage").is_none() {
            continue; // showdown sans défenseur
        }
        let turn = r.g("turn").int().unwrap();
        let mv = picks.iter().find(|p| {
            p.g("turn").int() == Some(turn)
                && std::iter::once(p.g("first")).chain(p.g("plan").arr().iter()).any(|x| {
                    let s = x.str().unwrap_or("");
                    s.starts_with("('move'") && !s.contains("'base'")
                })
        });
        let mut row = jo! { "id" => game.g("id").clone(), "opp" => opp.clone(), "faithful" => faithful };
        for (k, x) in r.obj() {
            row.set(k, x.clone());
        }
        row.set("v", mv.map(|p| p.g("v").clone()).unwrap_or(Json::Null));
        out.push(row);
    }
    Ok(out)
}

/// `tot(us)` : Might total des unités non étourdies
fn tot(us: &Json) -> i64 {
    us.arr().iter().filter(|u| !u.g("st").truthy()).map(|u| u.g("m").int().unwrap_or(0).max(0)).sum()
}

/// `main(args)`
pub fn main(args: &[String]) -> i32 {
    let paths: Vec<String> = args.iter().filter(|a| !a.starts_with("--")).cloned().collect();
    let games = match load_games(&paths) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    // Python ne rejoue que les parties qui notent leur commit ; RB_COMBATS_TOUTES=1 les rejoue toutes (essais)
    let toutes = std::env::var("RB_COMBATS_TOUTES").is_ok_and(|x| !x.is_empty());
    let jobs: Vec<&Json> = games.iter().filter(|g| toutes || g.g("commit").truthy()).collect();
    eprintln!("{} parties sur {} rejouées (IA à recherche du tour entier)", jobs.len(), games.len());
    let procs: usize = std::env::var("RB_PROCS").ok().and_then(|x| x.parse().ok()).unwrap_or(4);
    let mut rows: Vec<Json> = vec![];
    for r in pool::map(procs, &jobs, |g| worker(g)) {
        match r {
            Ok(v) => rows.extend(v),
            Err(e) => eprintln!("{e}"),
        }
    }
    for r in rows.iter_mut() {
        let after = r.g("after").clone();
        let lost = after.g("ai").arr().is_empty() || after.g("recalled").truthy();
        let own = r.g("tp").int() == Some(1); // sinon : unité adverse amenée par un effet (Blitzcrank…)
        let fav = tot(r.g("before").g("ai")) > tot(r.g("before").g("hu"));
        r.set("lost", lost);
        r.set("own_turn", own);
        r.set("fav", fav);
    }
    if args.iter().any(|a| a == "--json") {
        for r in &rows {
            println!("{}", r.dumps(false));
        }
        return 0;
    }
    for r in &rows {
        let v = r.g("v");
        let vtxt = match v.num() {
            Some(x) => py_round1(x),
            None => "?".into(),
        };
        let opp: String = py_str_json(r.g("opp")).chars().take(10).collect();
        let bf: String = r.g("bf").str().unwrap_or("").chars().take(18).collect();
        println!(
            "{} {} {:<20} {:<10} T{:<2} {:<18} Might avant {}-{}, aux dégâts {}-{}  valeur du coup {}{}",
            if r.g("lost").truthy() { "PERD " } else { "gagne" },
            if r.g("own_turn").truthy() { "tour IA " } else { "ton tour" },
            r.g("id").str().unwrap_or(""),
            opp,
            r.g("turn").int().unwrap(),
            bf,
            tot(r.g("before").g("ai")),
            tot(r.g("before").g("hu")),
            tot(r.g("damage").g("ai")),
            tot(r.g("damage").g("hu")),
            vtxt,
            if r.g("faithful").truthy() { "" } else { "  (rejeu NON fidèle)" }
        );
    }
    let own: Vec<&Json> = rows.iter().filter(|r| r.g("own_turn").truthy()).collect();
    let n_lost = |it: &[&Json], f: &dyn Fn(&Json) -> bool| it.iter().filter(|r| r.g("lost").truthy() && f(r)).count();
    println!(
        "\nTour de l'IA : {} combats, perdus {} (favorables au début du showdown : {}, défavorables : {}). Ton tour (unité adverse amenée par ton effet) : {}, perdus {}.",
        own.len(),
        n_lost(&own, &|_| true),
        n_lost(&own, &|r| r.g("fav").truthy()),
        n_lost(&own, &|r| !r.g("fav").truthy()),
        rows.len() - own.len(),
        rows.iter().filter(|r| !r.g("own_turn").truthy() && r.g("lost").truthy()).count()
    );
    0
}

/// `round(x, 1)` imprimé
fn py_round1(x: f64) -> String {
    super::py::repr_f64(super::py::round_n(x, 1))
}
