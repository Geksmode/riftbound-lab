//! `empreinte.py` : garde-fou des décisions de l'IA à graines fixes (porte d'intégration, étape « empreinte »).
//!
//! ```text
//!   empreinte          compare à empreinte.json : code 1 si une décision a changé
//!   empreinte --maj    réécrit empreinte.json (changement voulu du jeu : carte, règle, IA)
//! ```
//! Parties à graines fixes (`exp_search::setup` « defaut »), jouées par la recherche de l'IA « old » et « turn ».
//! Chaque décision prise (joueur, type, coup choisi) et le résultat final entrent dans un MD5 par partie : le texte
//! haché est le `repr` Python de `(joueur, type, coup)` (`repr_value`), donc le MD5 est celui du moteur Python tant
//! que l'IA joue les mêmes parties. Une partie par fil, compteurs d'objets neufs (`Obj._n = 0`).
use std::sync::Arc;
use std::time::Instant;

use super::exp_search::{here, setup};
use super::md5::Md5;
use super::pool;
use super::py::{self, PyVal};
use crate::ai::SearchOpts;
use crate::game::{Agent, Counters, Game};
use crate::plans::plan_agent;
use crate::value::{Loc, Value};

/// (graine, recherche) : quatre parties « old » et une « turn »
pub const PARTIES: [(i64, &str); 5] = [(910001, "old"), (910002, "old"), (910003, "old"), (910004, "old"), (910005, "turn")];

/// `repr(x)` Python d'une valeur du moteur (options de décision, coups).
pub fn repr_value(g: &Game, v: &Value) -> String {
    match v {
        Value::None => "None".into(),
        Value::Bool(b) => (if *b { "True" } else { "False" }).into(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => py::repr_f64(*f),
        Value::Str(s) => PyVal::Str(s.to_string()).py_repr(),
        Value::Obj(o) => g.repr(*o),
        Value::Rune(r) => g.repr_rune(*r),
        Value::Item(i) => {
            let it = g.it(*i);
            format!("<{} {} P{}>", it.kind, it.name, it.ctrl)
        }
        Value::Loc(Loc::Base) => "'base'".into(),
        Value::Loc(Loc::Bf(i)) => i.to_string(),
        Value::List(v) => format!("[{}]", v.iter().map(|x| repr_value(g, x)).collect::<Vec<_>>().join(", ")),
        Value::Tuple(v) => match v.len() {
            1 => format!("({},)", repr_value(g, &v[0])),
            _ => format!("({})", v.iter().map(|x| repr_value(g, x)).collect::<Vec<_>>().join(", ")),
        },
        Value::Map(m) => format!(
            "{{{}}}",
            m.0.iter()
                .map(|(k, x)| format!("{}: {}", PyVal::Str(k.to_string()).py_repr(), repr_value(g, x)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Opt(o) => format!("Opt({})", o.label),
        Value::Fn(f) => format!("<function {}>", f.1),
    }
}

/// Résultat d'une partie de l'empreinte.
#[derive(Clone, Debug)]
pub struct Partie {
    pub seed: i64,
    pub search: String,
    pub legends: [String; 2],
    pub decisions: i64,
    pub winner: Option<i64>,
    pub turns: i64,
    pub md5: String,
    pub s: f64,
}

/// `partie((graine, recherche))`
pub fn partie(seed: i64, search: &str) -> Partie {
    let t0 = Instant::now();
    let s = setup(seed, "defaut");
    // PlanAgent(seed + 500000 * i, plan=pls[i], opp_plan=pls[1 - i], search=search) : réglages par défaut
    let ag: Vec<Box<dyn Agent>> = (0..2)
        .map(|i| {
            let o = SearchOpts { search: Some(search.to_string()), ..Default::default() };
            Box::new(plan_agent(seed as i128 + 500000 * i as i128, Some(s.plans[i].clone()), Some(s.plans[1 - i].clone()), o))
                as Box<dyn Agent>
        })
        .collect();
    let mut g = Game::new(
        &[s.decks[0].list(), s.decks[1].list()],
        ag,
        Some(s.first),
        seed as i128,
        false,
        crate::spec::VICTORY as i64,
        Arc::new(Counters::default()),
    );
    for pid in 0..2 {
        let mut a = std::mem::replace(&mut g.agents[pid], Box::new(crate::game_events::FirstChoice));
        a.start(&mut g);
        g.agents[pid] = a;
    }
    let (mut h, mut n) = (Md5::new(), 0i64);
    loop {
        let Some(d) = g.advance(false) else { break };
        let ph = g.agents[d.player].clone_box();
        let mut a = std::mem::replace(&mut g.agents[d.player], ph);
        let r = a.decide(&mut g, &d);
        g.agents[d.player] = a;
        let Ok(act) = r else { break };
        h.update(format!("({}, {}, {})", d.player, PyVal::Str(d.kind.to_string()).py_repr(), repr_value(&g, &act)).as_bytes());
        g.apply(&act);
        n += 1;
    }
    let w = match g.winner {
        Some(w) => w.to_string(),
        None => "None".into(),
    };
    h.update(format!("({}, {}, [{}, {}])", w, g.turn_no, g.p[0].points, g.p[1].points).as_bytes());
    Partie {
        seed,
        search: search.to_string(),
        legends: [s.decks[0].legend.clone(), s.decks[1].legend.clone()],
        decisions: n,
        winner: g.winner,
        turns: g.turn_no,
        md5: h.hexdigest(),
        s: py::round_n(t0.elapsed().as_secs_f64(), 1),
    }
}

fn row(r: &Partie) -> PyVal {
    crate::pydict! {
        "seed" => r.seed, "search" => r.search.as_str(), "legends" => r.legends.to_vec(), "decisions" => r.decisions,
        "winner" => r.winner, "turns" => r.turns, "md5" => r.md5.as_str(), "s" => r.s,
    }
}

/// `main(maj)` : code de sortie
pub fn main(maj: bool) -> i32 {
    let t0 = Instant::now();
    let out = pool::map(4.min(PARTIES.len()), &PARTIES, |&(s, k)| partie(s, k));
    let total = py::round_n(t0.elapsed().as_secs_f64(), 1);
    let file = here().join("empreinte.json");
    if maj {
        let v = crate::pydict! {
            "engine_md5" => super::version::engine_md5(), "rust" => env!("CARGO_PKG_VERSION"), "seconds" => total,
            "games" => PyVal::List(out.iter().map(row).collect()),
        };
        py::dump_file(&file, &v, py::Dump { indent: Some(1), ensure_ascii: false });
        println!("empreinte.json réécrit : {} parties, {} s", out.len(), py::fstr(total));
        return 0;
    }
    let refv = if file.exists() { py::load_file(&file).ok() } else { None };
    let refs: Vec<PyVal> = refv.as_ref().and_then(|v| v.get("games")).map(|g| g.as_list().to_vec()).unwrap_or_default();
    let mut bad = 0;
    for r in &out {
        let x = refs
            .iter()
            .find(|x| x.get("seed").map(|s| s.as_i64()) == Some(r.seed) && x.get("search").and_then(|s| s.as_str()) == Some(&r.search));
        let ok = x.is_some_and(|x| x.get("md5").and_then(|m| m.as_str()) == Some(&r.md5));
        if !ok {
            bad += 1;
        }
        let why = if ok {
            String::new()
        } else if let Some(x) = x {
            let g = |k: &str| x.get(k).map(|v| v.py_str()).unwrap_or_default();
            format!(
                " : décisions changées (référence {} décisions, gagnant {}, {} tours ; maintenant {}, {}, {})",
                g("decisions"),
                g("winner"),
                g("turns"),
                r.decisions,
                r.winner.map(|w| w.to_string()).unwrap_or("None".into()),
                r.turns
            )
        } else {
            " (pas de référence)".into()
        };
        println!(
            "{}  graine {} {:4} {} : {} décisions, {} s{}",
            if ok { "OK" } else { "ÉCHEC" },
            r.seed,
            r.search,
            r.legends.join(" / "),
            r.decisions,
            py::fstr(r.s),
            why
        );
    }
    if bad > 0 {
        println!(
            "ÉCHEC : le jeu de l'IA a changé. Si c'est voulu (carte, règle, réglage de l'IA), lance « empreinte --maj » \
             et committe empreinte.json avec le changement ; sinon, une optimisation a un effet de bord."
        );
        return 1;
    }
    println!("empreinte : {} parties identiques à la référence, {} s", out.len(), py::fstr(total));
    0
}
