//! Journal du test différentiel (même écriture que `rust/diff/trace.py`).
use std::sync::{Arc, Mutex};

use crate::game::{Agent, Decision, Game, Res, O};
use crate::value::{Loc, Map, Value};

/// Écriture canonique d'une valeur (options de g.ask, actions).
pub fn canon(g: &Game, v: &Value) -> String {
    match v {
        Value::None => "N".into(),
        Value::Bool(true) => "T".into(),
        Value::Bool(false) => "F".into(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => format!("{f:?}"),
        Value::Str(s) => crate::json::Json::Str(s.to_string()).dump(),
        Value::Obj(o) => format!("O{}", g.o(*o).uid),
        Value::Rune(r) => format!("R{}", g.r(*r).uid),
        Value::Item(i) => format!("I{}", g.it(*i).id),
        Value::Loc(Loc::Base) => "\"base\"".into(),
        Value::Loc(Loc::Bf(i)) => i.to_string(),
        Value::List(v) | Value::Tuple(v) => {
            format!("[{}]", v.iter().map(|x| canon(g, x)).collect::<Vec<_>>().join(","))
        }
        Value::Map(m) => {
            let mut kv: Vec<(&str, &Value)> = m.0.iter().map(|(k, v)| (&**k, v)).collect();
            kv.sort_by(|a, b| a.0.cmp(b.0));
            let parts: Vec<String> = kv
                .iter()
                .map(|(k, v)| format!("{}:{}", crate::json::Json::Str(k.to_string()).dump(), canon(g, v)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Opt(o) => format!("Opt({}|{})", crate::json::Json::Str(o.label.clone()).dump(), canon(g, &o.value)),
        Value::Fn(_) => "F".into(),
    }
}

fn loc(l: Option<Loc>) -> String {
    match l {
        None => "N".into(),
        Some(Loc::Base) => "base".into(),
        Some(Loc::Bf(i)) => i.to_string(),
    }
}

/// Résumé de l'état (écrit à chaque décision de haut niveau).
pub fn state(g: &mut Game) -> String {
    let mut parts = vec![format!(
        "t{} tp{} st={} pts={}-{} xp={}-{}",
        g.turn_no, g.tp, g.stage, g.p[0].points, g.p[1].points, g.p[0].xp, g.p[1].xp
    )];
    for pl in &g.p {
        let runes: String = pl.runes.iter().map(|&r| g.repr_rune(r)).collect();
        parts.push(format!(
            "P{} h={} d={} tr={} b={} r={} rd={} e={}",
            pl.pid,
            pl.hand.len(),
            pl.deck.len(),
            pl.trash.len(),
            pl.banish.len(),
            runes,
            pl.rune_deck.len(),
            pl.pool_e
        ));
    }
    for o in g.board.clone() {
        let m = g.might(o).unwrap_or(-999);
        let ob = g.o(o);
        parts.push(format!(
            "{}#{}@{} c{} m{} d{} {}{}{}",
            g.cname(o),
            ob.uid,
            loc(ob.loc),
            ob.ctrl,
            m,
            ob.damage,
            if ob.exh { "x" } else { "" },
            if ob.stunned { "s" } else { "" },
            if ob.buff != 0 { "b" } else { "" }
        ));
    }
    for b in &g.bfs {
        let o = |x: Option<usize>| x.map(|v| v.to_string()).unwrap_or("None".into());
        parts.push(format!("bf{}:{} c={} k={}", b.idx, b.name, o(b.ctrl), o(b.contested)));
    }
    parts.join(" | ")
}

/// Agent qui note chaque question et chaque décision de l'agent qu'il enveloppe.
#[derive(Clone)]
pub struct Trace {
    pub inner: Box<dyn Agent>,
    pub out: Arc<Mutex<Vec<String>>>,
}

impl Agent for Trace {
    fn start(&mut self, g: &mut Game) {
        self.inner.start(g)
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        let r = self.inner.mulligan(g, pid)?;
        let v: Vec<Value> = r.iter().map(|&o| Value::Obj(o)).collect();
        let line = format!("mul P{pid} {}", canon(g, &Value::List(v)));
        self.out.lock().unwrap().push(line);
        Ok(r)
    }
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        let a = self.inner.decide(g, d)?;
        let st = state(g);
        let opts = d.options.as_ref().unwrap();
        let i = opts.iter().position(|o| *o == a).unwrap();
        let line = format!(
            "dec {} P{} n={} -> {} {} :: {}",
            d.kind,
            d.player,
            opts.len(),
            i,
            canon(g, &a),
            opts.iter().map(|o| canon(g, o)).collect::<Vec<_>>().join(" ")
        );
        let mut out = self.out.lock().unwrap();
        out.push(format!("st {st}"));
        out.push(line);
        Ok(a)
    }
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        let i = self.inner.choose(g, pid, kind, options, ctx)?;
        let line = format!(
            "ask {kind} P{pid} n={} -> {i} :: {}",
            options.len(),
            options.iter().map(|o| canon(g, o)).collect::<Vec<_>>().join(" ")
        );
        self.out.lock().unwrap().push(line);
        Ok(i)
    }
    fn every_choice(&self) -> bool {
        self.inner.every_choice()
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// Une partie tracée : même protocole que `rust/diff/trace.py run`.
pub fn run(seed: i128, k0: &str, k1: &str, a0: i128, a1: i128, max_decisions: usize) -> Vec<String> {
    let out = Arc::new(Mutex::new(vec![]));
    let d0 = crate::decks::load(k0, None).deck;
    let d1 = crate::decks::load(k1, None).deck;
    let agents: Vec<Box<dyn Agent>> = vec![
        Box::new(Trace { inner: Box::new(crate::agents::RandomAgent::new(a0)), out: out.clone() }),
        Box::new(Trace { inner: Box::new(crate::agents::RandomAgent::new(a1)), out: out.clone() }),
    ];
    let g = crate::agents::play_game(&[d0, d1], agents, seed, None, false, max_decisions);
    let w = match g.winner {
        Some(w) => w.to_string(),
        None => "None".into(),
    };
    let mut v = out.lock().unwrap().clone();
    v.push(format!("fin winner={w} t{} pts={}-{}", g.turn_no, g.p[0].points, g.p[1].points));
    v
}
