//! `texel/test_texel.py [parties]` : égalité exacte entre `ai::evaluate` et `texel::features` + `linear`, et symétrie des
//! critères, sur toutes les décisions de parties PolicyAgent contre PolicyAgent (milieu de tour compris), les deux
//! points de vue, avec les poids `ai::EV` puis des poids tirés au hasard (`pts_ramp` et `react_kw` restent à 0).
use std::sync::Arc;

use super::texel::{features, linear, KEYS};
use crate::ai::{self, conf};
use crate::game::{Agent, Counters, Game};
use crate::pyrandom::PyRandom;

/// Rend (réussis, total, écart max, nombre de positions où chaque critère est non nul).
pub fn run(n: i64) -> (usize, usize, f64, [usize; 17]) {
    let mut rng = PyRandom::new(1);
    let mut ws: Vec<ai::Ev> = vec![conf().ev.clone()];
    for _ in 0..5 {
        let mut w = conf().ev.clone();
        for k in ai::Ev::KEYS {
            w.set(k, rng.uniform(-10.0, 10.0));
        }
        w.set("pts_ramp", 0.0);
        w.set("react_kw", 0.0);
        ws.push(w);
    }
    let (mut ok, mut tot, mut worst) = (0usize, 0usize, 0.0f64);
    let mut seen = [0usize; 17];
    let dd = super::exp_search::defaults();
    for k in 0..n {
        let seed = 4_000_000 + k;
        let mut r = PyRandom::new(seed as i128);
        let (da, db) = (crate::train::tload(r.choice(dd)), crate::train::tload(r.choice(dd)));
        let sorted = |d: &crate::train::TDeck| {
            let mut b = d.battlefields.clone();
            b.sort();
            b
        };
        let ba = r.choice(&sorted(&da)).clone();
        let bb = r.choice(&sorted(&db)).clone();
        let first = r.randrange(2) as usize;
        let ags: Vec<Box<dyn Agent>> = vec![Box::new(ai::PolicyAgent::new()), Box::new(ai::PolicyAgent::new())];
        let mut g = Game::new(
            &[da.with_bf(&ba).list(), db.with_bf(&bb).list()],
            ags,
            Some(first),
            seed as i128,
            false,
            crate::spec::VICTORY as i64,
            Arc::new(Counters::default()),
        );
        loop {
            let Some(d) = g.advance(false) else { break };
            if g.winner.is_none() {
                for me in 0..2 {
                    let (c, f) = features(&mut g, me);
                    let (c2, f2) = features(&mut g, 1 - me);
                    let sym = (c + c2).abs() < 1e-9 && (0..17).all(|i| (f[i] + f2[i]).abs() < 1e-9);
                    for i in 0..17 {
                        seen[i] += (f[i] != 0.0) as usize;
                    }
                    for w in &ws {
                        let e = ai::evaluate(&mut g, me, Some(w));
                        let lin = linear(c, &f, &|k| w.get(k).unwrap());
                        worst = worst.max((e - lin).abs());
                        tot += 1;
                        if (e - lin).abs() < 1e-6 * 1.0f64.max(e.abs()) && sym {
                            ok += 1;
                        } else if tot - ok <= 5 {
                            println!("FAIL {} {me} {e} {lin} {}", g.turn_no, if sym { "sym" } else { "ASYM" });
                        }
                    }
                }
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
    (ok, tot, worst, seen)
}

/// `test_texel.py [parties]` (défaut 60) : code de sortie
pub fn main(a: &[String]) -> i32 {
    let n: i64 = a.first().map(|x| x.parse().unwrap()).unwrap_or(60);
    let (ok, tot, worst, seen) = run(n);
    println!(
        "critères non nuls (positions) : {{{}}}",
        KEYS.iter().enumerate().map(|(i, k)| format!("'{k}': {}", seen[i])).collect::<Vec<_>>().join(", ")
    );
    println!("écart max {worst:.2e}");
    println!("{ok}/{tot} tests passed");
    if ok == tot {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn evaluate_egale_features_lineaires() {
        let (ok, tot, worst, _) = super::run(6);
        assert!(tot > 0 && ok == tot, "{ok}/{tot}, écart max {worst}");
    }

    #[test]
    fn fmt_g_comme_python() {
        for (x, p, s) in [
            (7.346, 4, "7.346"),
            (-0.86, 4, "-0.86"),
            (0.2201, 4, "0.2201"),
            (1234567.0, 4, "1.235e+06"),
            (0.00001234, 4, "1.234e-05"),
            (10.36, 4, "10.36"),
            (100.0, 4, "100"),
        ] {
            assert_eq!(crate::outils::texel::fmt_g(x, p), s);
        }
    }
}
