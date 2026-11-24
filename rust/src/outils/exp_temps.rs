//! `exp_temps.py` : temps de réflexion de l'IA par décision, sur les MÊMES positions pour chaque version
//! (`exp_search::version`).
//!
//! ```text
//!   exp_temps <graines séparées par des virgules> <version> [<version>...]     ex. exp_temps 777003,777011 old sh
//! ```
//! Positions = toutes les décisions à plus d'une option de parties « defaut » (`exp_search::setup`) jouées par
//! l'ancienne recherche ; chaque version décide sur une copie de chaque position (compteurs d'objets remis à leur
//! valeur de la position). Le programme `exp_temps.mjs` (Pyodide) n'a pas d'équivalent : la table tourne en
//! WebAssembly, voir `rust/src/wasm.rs`.
use std::sync::Arc;
use std::time::Instant;

use super::exp_search::{setup, version};
use crate::ai::{counters, set_counters, SearchOpts};
use crate::game::{Agent, Counters, Decision, Game};
use crate::plans::plan_agent;

/// `positions(seeds)`
pub fn positions(seeds: &[i64]) -> Vec<(Game, Decision, (u32, u32))> {
    let mut pos = vec![];
    for &seed in seeds {
        let s = setup(seed, "defaut");
        let ag: Vec<Box<dyn Agent>> = (0..2)
            .map(|i| {
                let o = SearchOpts { search: Some("old".into()), ..Default::default() };
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
        loop {
            let Some(d) = g.advance(false) else { break };
            if d.options.as_ref().is_some_and(|o| o.len() > 1) {
                pos.push((g.clone_game(), d.clone(), counters(&g)));
            }
            let ph = g.agents[d.player].clone_box();
            let mut a = std::mem::replace(&mut g.agents[d.player], ph);
            let r = a.decide(&mut g, &d);
            g.agents[d.player] = a;
            match r {
                Ok(act) => g.apply(&act),
                Err(_) => break,
            }
        }
    }
    pos
}

/// `main(seeds, versions)`
pub fn main(seeds: &[i64], versions: &[String]) {
    let pos = positions(seeds);
    println!("{} positions", pos.len());
    for v in versions {
        let (sm, n, ex, h, cfg) = version(v);
        let mut tm: Vec<f64> = vec![];
        let mut kind: Vec<(&str, Vec<f64>)> = vec![];
        for (k, (g, d, cnt)) in pos.iter().enumerate() {
            set_counters(g, *cnt);
            let mut c = g.clone_game();
            let o = SearchOpts { samples: n, search: Some(sm.clone()), sh_extra: Some(ex), horizon: h, cfg: cfg.clone(), ..Default::default() };
            let mut a = plan_agent(k as i128, None, None, o);
            let t = Instant::now();
            let _ = a.decide(&mut c, d);
            tm.push(t.elapsed().as_secs_f64());
            let key = if d.kind == "main" { "main" } else { "autres" };
            match kind.iter_mut().find(|e| e.0 == key) {
                Some(e) => e.1.push(*tm.last().unwrap()),
                None => kind.push((key, vec![*tm.last().unwrap()])),
            }
        }
        tm.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let ms = |x: f64| format!("{:.0}", 1000.0 * x);
        let mean = |x: &[f64]| x.iter().sum::<f64>() / x.len() as f64;
        let p95 = |x: &[f64]| x[(0.95 * x.len() as f64) as usize];
        println!("{v} : moyenne {} ms, p95 {} ms, max {} ms", ms(mean(&tm)), ms(p95(&tm)), ms(*tm.last().unwrap()));
        kind.sort_by(|a, b| a.0.cmp(b.0));
        for (kd, mut xs) in kind {
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            println!("  {kd} ({}) : moyenne {} ms, p95 {} ms", xs.len(), ms(mean(&xs)), ms(p95(&xs)));
        }
    }
}
