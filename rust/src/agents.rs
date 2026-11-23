//! Agents simples (`agents.py`). RandomAgent sert au fuzz du moteur et au test différentiel.
use crate::game::{Agent, Decision, Game, Res, O};
use crate::pyrandom::PyRandom;
use crate::value::{Map, Value};

#[derive(Clone)]
pub struct RandomAgent {
    pub rng: PyRandom,
}

impl RandomAgent {
    pub fn new(seed: i128) -> Self {
        RandomAgent { rng: PyRandom::new(seed) }
    }
}

impl Agent for RandomAgent {
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        let h = g.p[pid].hand.clone();
        let k = self.rng.randint(0, 2) as usize;
        Ok(self.rng.sample(&h, k))
    }

    fn decide(&mut self, _g: &mut Game, d: &Decision) -> Res<Value> {
        let all = d.options.as_ref().expect("décision sans options");
        let mut opts: Vec<&Value> = all.iter().collect();
        if d.kind == "main" && opts.len() > 1 && self.rng.random() < 0.85 {
            opts.retain(|o| o.list()[0].str() != "end");
        }
        if d.kind != "main" && self.rng.random() < 0.6 {
            return Ok(Value::Tuple(vec!["pass".into()]));
        }
        Ok((*self.rng.choice(&opts)).clone())
    }

    fn choose(&mut self, _g: &mut Game, _pid: usize, kind: &str, options: &[Value], _ctx: &Map) -> Res<usize> {
        if kind == "damage_order" {
            // jamais posée : une seule option (Game.assign_damage)
            panic!("damage_order");
        }
        Ok(self.rng.choice_index(options.len()))
    }

    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// `play_game(decks, agents, seed, first, log, max_decisions)`
pub fn play_game(
    decks: &[crate::game::DeckList; 2],
    agents: Vec<Box<dyn Agent>>,
    seed: i128,
    first: Option<usize>,
    log: bool,
    max_decisions: usize,
) -> Game {
    let mut g = Game::new(
        decks,
        agents,
        first,
        seed,
        log,
        crate::spec::VICTORY as i64,
        std::sync::Arc::new(crate::game::Counters::default()),
    );
    for pid in 0..2 {
        let mut a = std::mem::replace(&mut g.agents[pid], Box::new(crate::game_events::FirstChoice));
        a.start(&mut g);
        g.agents[pid] = a;
    }
    let mut n = 0;
    loop {
        let Some(d) = g.advance(false) else { break };
        let mut a = std::mem::replace(&mut g.agents[d.player], Box::new(crate::game_events::FirstChoice));
        let r = a.decide(&mut g, &d);
        g.agents[d.player] = a;
        match r {
            Ok(act) => g.apply(&act),
            Err(_) => break,
        }
        n += 1;
        if n > max_decisions {
            g.winner = Some(-1);
            break;
        }
    }
    g
}
