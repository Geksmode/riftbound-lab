//! Banc d'erreurs de cible et de timing de l'IA (`riftbound/engine/banc/banc.py` et `cout.py`, 2026-10-28).
//!
//! ```text
//!   banc gen <parties> <graine0> <defaut|akali|miroir> [version] [processus]
//!   banc bilan <positions.jsonl> [...]
//!   banc rejuge <parties.jsonl> [processus]
//!   banc rejoue <positions.jsonl> <version> [processus] [play|ask]
//!   banc_cout <pos.jsonl> [tirages] [version] [processus] [seuil]
//! ```
//! Parties IA contre IA comme `exp_search::play` (même setup, mêmes graines d'agents), version « turn » par défaut. À
//! chaque choix de l'IA, un juge rejoue les autres choix possibles sur une copie de la position et les note (voir le
//! texte de `banc.py` : « play » = variantes de la même carte, « ask » = choix pendant la résolution ; note = évaluation
//! de l'IA après résolution, deux jeux de poids, regret = écart à la meilleure option quand LES DEUX poids la préfèrent).
//!
//! Mêmes fichiers lus et écrits que Python (`banc/data/pos-*.jsonl`, `parties-*.jsonl`, `cout-*.jsonl`,
//! `results_banc.json`). Une réponse à un `ask` s'écrit `["i", indice]` : l'agent Rust répond toujours par l'indice de
//! l'option (les codages « s » et « v » de Python, listes d'options et valeurs, se relisent par égalité).
//!
//! Écart : les fichiers de parties écrits par le banc Python (moteur Python) sont d'une autre version du moteur : `gen`
//! refuse d'y ajouter des parties (comme Python entre deux versions) et `rejuge` / `rejoue` les rejouent avec le moteur
//! Rust en écartant les parties qui ne retombent pas sur leurs décisions (Desync).
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::empreinte::repr_value;
use super::exp_search::{agents, here, setup};
use super::pool;
use super::py::{self, Dump, PyVal};
use crate::ai::{counters, evaluate, heur_choose, heur_mulligan, set_counters, settle_pass, Cfg, Ev, PolicyAgent};
use crate::game::{Agent, Counters, Decision, Game, Res, O};
use crate::jo;
use crate::json::{self, Json};
use crate::replay::describe;
use crate::value::{Loc, Map, Value};

pub const REGRET: f64 = 2.0; // erreur nette : ≈ un tiers de point de victoire
pub const NOEFFECT: f64 = 0.25;
pub const MAX_ALTS: usize = 40;

/// Dossier du banc (`HERE` de banc.py)
pub fn banc_dir() -> std::path::PathBuf {
    here().join("banc")
}

// ---------------------------------------------------------------------- réponses aux asks : codage pour les rejouer
/// `dec(e, options)` : indice de l'option, None = `_BAD`
pub fn dec(e: &Json, options: &[Value]) -> Option<usize> {
    let a = e.arr();
    match a.first().and_then(|x| x.str()) {
        Some("i") => {
            let i = a[1].int()?;
            (i >= 0 && (i as usize) < options.len()).then_some(i as usize)
        }
        Some("v") => {
            let v = json_value(&a[1]);
            options.iter().position(|o| *o == v)
        }
        _ => None,
    }
}

fn json_value(j: &Json) -> Value {
    match j {
        Json::Null => Value::None,
        Json::Bool(b) => Value::Bool(*b),
        Json::Int(n) => Value::Int(*n),
        Json::Float(f) => Value::Float(*f),
        Json::Str(s) => Value::from(s.as_str()),
        Json::Arr(v) => Value::List(v.iter().map(json_value).collect()),
        Json::Obj(_) => Value::None,
    }
}

/// `label(g, o, pid)`
pub fn label(g: &Game, o: &Value, pid: usize) -> String {
    if let Value::Obj(x) = o {
        let side = if g.o(*x).ctrl == pid { "" } else { " (adv.)" };
        return format!("{}{}", g.cname(*x), side);
    }
    repr_value(g, o).chars().take(60).collect()
}

/// Un ask noté par `Rec`
#[derive(Clone, Debug)]
pub struct AskRec {
    pub pid: usize,
    pub kind: String,
    pub n: usize,
    pub enc: Json,
    pub src: Option<String>,
    pub labels: Vec<String>,
}

/// `Rec` : agent réel enveloppé, note chaque ask (joueur, type, options, réponse).
#[derive(Clone)]
pub struct RecAsk {
    pub ag: Box<dyn Agent>,
    pub log: Arc<Mutex<Vec<AskRec>>>,
}

impl Agent for RecAsk {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        let r = self.ag.choose(g, pid, kind, options, ctx)?;
        let src = match ctx.get("item") {
            Some(Value::Item(i)) => Some(g.it(*i).name.clone()),
            _ => None,
        };
        let labels = options.iter().take(MAX_ALTS).map(|o| label(g, o, pid)).collect();
        self.log.lock().unwrap().push(AskRec {
            pid,
            kind: kind.to_string(),
            n: options.len(),
            enc: Json::Arr(vec!["i".into(), r.into()]),
            src,
            labels,
        });
        Ok(r)
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        self.ag.mulligan(g, pid)
    }
    fn start(&mut self, g: &mut Game) {
        self.ag.start(g)
    }
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        self.ag.decide(g, d)
    }
    fn every_choice(&self) -> bool {
        self.ag.every_choice()
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
    fn as_any(&mut self) -> Option<&mut dyn std::any::Any> {
        self.ag.as_any()
    }
}

/// Un ask du segment d'une décision (type, nombre d'options, codage de la réponse)
#[derive(Clone, Debug)]
pub struct Seg {
    pub enc: Json,
    pub kind: String,
    pub n: usize,
}

/// `Forced` : asks d'avant le j-ième rejoués comme dans la partie, le j-ième forcé sur `alt`, la suite par
/// Heuristics. `seen` garde (type, nombre d'options) du j-ième pour vérifier que le rejeu suit la partie. Les deux
/// joueurs partagent le même état (en Python, le même objet).
#[derive(Clone)]
pub struct Forced {
    pub seg: Arc<Vec<Seg>>,
    pub j: i64,
    pub alt: usize,
    pub k: Arc<AtomicUsize>,
    pub seen: Arc<Mutex<Option<(String, usize)>>>,
    /// `Ask` : le jj-ième ask est posé à `cand` (sa réponse dans `got`)
    pub cand: Option<Arc<Mutex<Box<dyn Agent>>>>,
    pub jj: i64,
    pub got: Arc<Mutex<Option<usize>>>,
}

impl Forced {
    pub fn new(seg: Arc<Vec<Seg>>, j: i64, alt: usize) -> Forced {
        Forced { seg, j, alt, k: Arc::new(AtomicUsize::new(0)), seen: Arc::new(Mutex::new(None)), cand: None, jj: -1, got: Arc::new(Mutex::new(None)) }
    }
    pub fn seen(&self) -> Option<(String, usize)> {
        self.seen.lock().unwrap().clone()
    }
}

impl Agent for Forced {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        if self.cand.is_some() && self.k.load(Ordering::SeqCst) as i64 == self.jj {
            self.k.fetch_add(1, Ordering::SeqCst);
            let r = self.cand.as_ref().unwrap().lock().unwrap().choose(g, pid, kind, options, ctx)?;
            *self.got.lock().unwrap() = Some(r);
            return Ok(r);
        }
        let k = self.k.fetch_add(1, Ordering::SeqCst);
        if k as i64 == self.j {
            *self.seen.lock().unwrap() = Some((kind.to_string(), options.len()));
            return Ok(self.alt);
        }
        if k < self.seg.len() {
            if let Some(r) = dec(&self.seg[k].enc, options) {
                return Ok(r);
            }
        }
        heur_choose(g, pid, kind, options, ctx)
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        heur_mulligan(&Cfg::new(), g, pid)
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------- juge
/// `note(c, pid)` : (note avec ai.EV, note avec ai.EV_OLD)
pub fn note(c: &mut Game, pid: usize) -> (f64, f64) {
    if let Some(w) = c.winner {
        let v = if w == pid as i64 { 1000.0 } else { -1000.0 };
        return (v, v);
    }
    (evaluate(c, pid, None), evaluate(c, pid, Some(&Ev::old())))
}

/// `regret2(vals, i)` : (accord des deux poids, EV seul, EV_OLD seul)
pub fn regret2(vals: &[(f64, f64)], i: usize) -> (f64, f64, f64) {
    let both = vals.iter().map(|v| (v.0 - vals[i].0).min(v.1 - vals[i].1)).fold(f64::MIN, f64::max);
    let m0 = vals.iter().map(|v| v.0).fold(f64::MIN, f64::max);
    let m1 = vals.iter().map(|v| v.1).fold(f64::MIN, f64::max);
    (py::round_n(both.max(0.0), 3), py::round_n(m0 - vals[i].0, 3), py::round_n(m1 - vals[i].1, 3))
}

/// `replay(pre, cnt, a, pid, agent, fizzle, ckpt)`
pub fn replay(pre: &Game, cnt: (u32, u32), a: &Value, pid: usize, agent: Box<dyn Agent>, fizzle: bool, ckpt: bool) -> Option<(f64, f64)> {
    set_counters(pre, cnt);
    let mut c = pre.clone_game();
    c.agents = crate::game::Agents(vec![agent.clone_box(), agent]);
    c.ck.on = ckpt; // point de reprise pour une version avec ai.ask_sim (rescore)
    c.apply(a);
    if fizzle {
        let uid = a.list()[1].clone();
        let it = c.chain.iter().copied().find(|&x| {
            let i = c.it(x);
            i.kind == "spell" && i.card.is_some_and(|k| Value::from(c.o(k).uid) == uid)
        })?;
        let (card, ctrl) = (c.it(it).card.unwrap(), c.it(it).ctrl);
        c.chain.retain(|&x| x != it);
        let _ = c.card_leaves_chain(card, "trash", Some(ctrl));
    }
    settle_pass(&mut c, 400);
    Some(note(&mut c, pid))
}

fn kv(v: &Value) -> Vec<(String, Value)> {
    match v {
        Value::Map(m) => m.0.iter().map(|(k, x)| (k.to_string(), x.clone())).collect(),
        Value::List(l) | Value::Tuple(l) => l
            .iter()
            .filter_map(|p| match p.list() {
                [k, x] => Some((k.opt_str()?.to_string(), x.clone())),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

fn kv_get(v: &Value, k: &str) -> Option<Value> {
    kv(v).into_iter().rev().find(|e| e.0 == k).map(|e| e.1)
}

/// `full_options(pre, d)` : toutes les options, plafonds levés (`every_choice`)
pub fn full_options(pre: &Game, d: &Decision) -> Res<Vec<Value>> {
    let mut c = pre.clone_game();
    c.every_choice = true;
    match d.kind {
        "main" => c.options_main(d.player),
        "focus" => c.options_focus(d.player),
        _ => c.options_priority(d.player),
    }
}

fn same_card(o: &Value, a: &Value) -> bool {
    let (ol, al) = (o.list(), a.list());
    ol[0].opt_str() == Some("play") && ol[1] == al[1] && ol[2] == al[2]
}

/// `play_alts(pre, d, a)` : (carte, variantes jugées [a en tête], variantes vues par l'IA)
pub fn play_alts(pre: &Game, d: &Decision, a: &Value) -> Option<(O, Vec<Value>, Vec<Value>)> {
    if a.list()[0].opt_str() != Some("play") {
        return None;
    }
    let card = PolicyAgent::new().card_of(pre, a)?;
    let dopts = d.options.clone().unwrap_or_default();
    let mut alts: Vec<Value> = match full_options(pre, d) {
        Ok(v) => v.into_iter().filter(|o| same_card(o, a)).collect(),
        Err(_) => dopts.iter().filter(|o| same_card(o, a)).cloned().collect(),
    };
    // même emplacement (base ou battlefield) que le coup de l'IA
    let wh = |o: &Value| (kv_get(&o.list()[3], "loc"), kv_get(&o.list()[3], "dest"));
    let wa = wh(a);
    alts.retain(|o| wh(o) == wa);
    let seen: Vec<Value> = dopts.iter().filter(|o| same_card(o, a) && wh(o) == wa).cloned().collect();
    if !alts.contains(a) {
        alts.push(a.clone());
    }
    let mut out = vec![a.clone()];
    out.extend(alts.into_iter().filter(|o| o != a).take(MAX_ALTS - 1));
    Some((card, out, seen))
}

fn f3(x: f64) -> Json {
    Json::Float(py::round_n(x, 3))
}

fn empty_forced() -> Box<dyn Agent> {
    Box::new(Forced::new(Arc::new(vec![]), -1, 0))
}

/// `judge_play(pre, cnt, d, a)`
pub fn judge_play(pre: &Game, cnt: (u32, u32), d: &Decision, a: &Value) -> Option<Json> {
    let (card, alts, seen) = play_alts(pre, d, a)?;
    let spell = pre.spec(card).type_ == "Spell";
    if alts.len() < 2 && !spell {
        return None;
    }
    let pid = d.player;
    let vals: Vec<(f64, f64)> = alts.iter().map(|o| replay(pre, cnt, o, pid, empty_forced(), false, false).unwrap()).collect();
    let mut pm = pre.clone_game();
    let alt_rows: Vec<Json> = alts
        .iter()
        .zip(&vals)
        .map(|(o, v)| Json::Arr(vec![Json::Str(describe(&mut pm, o, pid, None)), f3(v.0), f3(v.1), Json::Bool(seen.contains(o))]))
        .collect();
    let mut r = jo! {
        "typ" => "play", "card" => pre.cname(card), "ctype" => pre.spec(card).type_.clone(), "phase" => d.kind, "pid" => pid,
        "alts" => Json::Arr(alt_rows), "n_seen" => seen.len(), "n_all" => alts.len(),
    };
    let rg = regret2(&vals, 0);
    r.set("regret", Json::Float(rg.0));
    r.set("regret_ev", Json::Float(rg.1));
    r.set("regret_old", Json::Float(rg.2));
    if spell {
        if let Some(fz) = replay(pre, cnt, a, pid, empty_forced(), true, false) {
            // sans effet seulement si sans effet pour les deux poids
            let (ee, eo) = (py::round_n(vals[0].0 - fz.0, 3), py::round_n(vals[0].1 - fz.1, 3));
            r.set("effect_ev", Json::Float(ee));
            r.set("effect_old", Json::Float(eo));
            r.set("effect", Json::Float(ee.max(eo)));
        }
        let tg: Vec<i64> = match kv_get(&a.list()[3], "tg") {
            Some(Value::List(l)) | Some(Value::Tuple(l)) => l.iter().filter_map(|u| if let Value::Int(i) = u { Some(*i) } else { None }).collect(),
            _ => vec![],
        };
        let mut tl: Vec<i64> = tg
            .iter()
            .filter_map(|&u| pre.obj(u as u32))
            .filter_map(|o| if let Some(Loc::Bf(i)) = pre.o(o).loc { Some(i as i64) } else { None })
            .collect();
        tl.sort();
        tl.dedup();
        r.set("tg", Json::Arr(tg.into_iter().map(Json::Int).collect()));
        r.set("tg_loc", Json::Arr(tl.into_iter().map(Json::Int).collect()));
    }
    Some(r)
}

/// `judge_asks(pre, cnt, a, seg, asks)` : `asks` = les asks notés du segment (pour leurs libellés)
pub fn judge_asks(pre: &Game, cnt: (u32, u32), a: &Value, seg: &[AskRec]) -> Vec<Json> {
    let mut out = vec![];
    let segs: Arc<Vec<Seg>> = Arc::new(seg.iter().map(|s| Seg { enc: s.enc.clone(), kind: s.kind.clone(), n: s.n }).collect());
    for (j, s) in seg.iter().enumerate() {
        if s.n < 2 || s.enc.arr().first().and_then(|x| x.str()) != Some("i") {
            continue;
        }
        let mut vals = vec![];
        let mut ok = true;
        for alt in 0..s.n.min(MAX_ALTS) {
            let f = Forced::new(segs.clone(), j as i64, alt);
            let v = replay(pre, cnt, a, s.pid, Box::new(f.clone()), false, false);
            if f.seen() != Some((s.kind.clone(), s.n)) {
                ok = false;
                break;
            }
            vals.push(v.unwrap());
        }
        if !ok {
            continue;
        }
        let ch = s.enc.arr()[1].int().unwrap() as usize;
        if ch >= vals.len() {
            continue;
        }
        let rg = regret2(&vals, ch);
        let alts: Vec<Json> = s.labels.iter().zip(&vals).map(|(lb, v)| Json::Arr(vec![Json::Str(lb.clone()), f3(v.0), f3(v.1)])).collect();
        out.push(jo! {
            "typ" => "ask", "kind" => s.kind.clone(), "src" => s.src.clone(), "pid" => s.pid, "j" => j, "chosen" => ch,
            "alts" => Json::Arr(alts), "regret" => rg.0, "regret_ev" => rg.1, "regret_old" => rg.2, "n_all" => s.n,
        });
    }
    out
}

// ---------------------------------------------------------------------- une partie
/// `Script` : rejoue une partie enregistrée : décisions et asks dans l'ordre du script ; mulligan par l'agent d'origine.
#[derive(Clone)]
pub struct ScriptAg {
    pub ag: Box<dyn Agent>,
    pub script: Arc<Vec<Json>>,
    pub ptr: Arc<AtomicUsize>,
}

impl ScriptAg {
    fn next(&self, typ: &str, kind: &str, n: usize) -> Json {
        let i = self.ptr.load(Ordering::SeqCst);
        let bad = match self.script.get(i) {
            None => true,
            Some(e) => {
                let a = e.arr();
                a[0].str() != Some(typ) || a[2].str() != Some(kind) || a[3].int() != Some(n as i64)
            }
        };
        if bad {
            panic!("Desync: pas {i} : attendu {}, vu {typ} {kind} {n}", self.script.get(i).map(|e| e.dumps(false)).unwrap_or("None".into()));
        }
        self.ptr.fetch_add(1, Ordering::SeqCst);
        self.script[i].clone()
    }
}

impl Agent for ScriptAg {
    fn decide(&mut self, _g: &mut Game, d: &Decision) -> Res<Value> {
        let opts = d.options.as_ref().unwrap();
        let e = self.next("d", d.kind, opts.len());
        Ok(opts[e.arr()[1].int().unwrap() as usize].clone())
    }
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        let e = self.next("a", kind, options.len());
        match dec(&e.arr()[1], options) {
            Some(r) => Ok(r),
            None => self.ag.choose(g, pid, kind, options, ctx),
        }
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        self.ag.mulligan(g, pid)
    }
    fn start(&mut self, g: &mut Game) {
        self.ag.start(g)
    }
    fn every_choice(&self) -> bool {
        self.ag.every_choice()
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

fn boxed(v: Vec<crate::ai::SearchAgent>) -> Vec<Box<dyn Agent>> {
    v.into_iter().map(|a| Box::new(a) as Box<dyn Agent>).collect()
}

/// Boucle d'une partie à la façon des scripts (compteurs neufs).
fn new_game(s: &super::exp_search::Setup, seed: i64, ags: Vec<Box<dyn Agent>>) -> Game {
    Game::new(&[s.decks[0].list(), s.decks[1].list()], ags, Some(s.first), seed as i128, false, crate::spec::VICTORY as i64, Arc::new(Counters::default()))
}

fn decide_with(g: &mut Game, d: &Decision) -> Res<Value> {
    let ph = g.agents[d.player].clone_box();
    let mut a = std::mem::replace(&mut g.agents[d.player], ph);
    let r = a.decide(g, d);
    g.agents[d.player] = a;
    r
}

/// `play(seed, mode, ver, judge, script0)` : une partie jouée par `ver` (ou rejouée d'après `script0`, sans
/// recherche : pour rejuger une partie déjà jouée). Rend (la partie, les positions jugées).
pub fn play(seed: i64, mode: &str, ver: &str, judge: bool, script0: Option<&Arc<Vec<Json>>>) -> (Json, Vec<Json>) {
    let s = setup(seed, mode);
    let mut ags = boxed(agents(seed, &s, ver, ver));
    if let Some(sc) = script0 {
        let ptr = Arc::new(AtomicUsize::new(0));
        ags = ags.into_iter().map(|a| Box::new(ScriptAg { ag: a, script: sc.clone(), ptr: ptr.clone() }) as Box<dyn Agent>).collect();
    }
    let asks: Arc<Mutex<Vec<AskRec>>> = Arc::new(Mutex::new(vec![]));
    let rec: Vec<Box<dyn Agent>> = ags.into_iter().map(|a| Box::new(RecAsk { ag: a, log: asks.clone() }) as Box<dyn Agent>).collect();
    let mut g = new_game(&s, seed, rec);
    let mut script: Vec<Json> = vec![]; // ["d", indice, type, n] ou ["a", codage, type, n]
    let mut pos: Vec<Json> = vec![];
    let mut moved: Vec<(i64, String)> = vec![]; // (tour, "u<uid>" ou "bf<idx>")
    let mut pend: Option<(Game, (u32, u32), Decision, Value, usize, usize, i64)> = None;
    let mut k_done = 0usize;
    let add_asks = |script: &mut Vec<Json>, k0: usize| {
        for a in asks.lock().unwrap()[k0..].iter() {
            script.push(Json::Arr(vec!["a".into(), a.enc.clone(), a.kind.clone().into(), a.n.into()]));
        }
    };
    loop {
        let d = g.advance(false);
        add_asks(&mut script, k_done);
        k_done = asks.lock().unwrap().len();
        if let Some((pre, cnt, pd, a, a0, step, turn)) = pend.take() {
            if judge {
                let seg: Vec<AskRec> = asks.lock().unwrap()[a0..k_done].to_vec();
                let now = counters(&g);
                let mut items = vec![];
                if let Some(jp) = judge_play(&pre, cnt, &pd, &a) {
                    items.push(jp);
                }
                items.extend(judge_asks(&pre, cnt, &a, &seg));
                set_counters(&g, now);
                for mut it in items {
                    it.set("seed", seed);
                    it.set("step", step);
                    it.set("turn", turn);
                    pos.push(it);
                }
            }
        }
        let Some(d) = d else { break };
        let cnt = counters(&g);
        let pre = if judge { Some(g.clone_game()) } else { None };
        let Ok(a) = decide_with(&mut g, &d) else { break };
        let opts = d.options.clone().unwrap();
        let idx = opts.iter().position(|o| *o == a).unwrap();
        script.push(Json::Arr(vec!["d".into(), idx.into(), d.kind.into(), opts.len().into()]));
        if a.list()[0].opt_str() == Some("move") && !matches!(a.list()[2], Value::Loc(Loc::Base)) {
            for u in a.list()[1].list() {
                moved.push((g.turn_no, format!("u{}", u.int())));
            }
            moved.push((g.turn_no, format!("bf{}", loc_idx(&a.list()[2]))));
        }
        pend = pre.map(|p| (p, cnt, d.clone(), a.clone(), k_done, script.len() - 1, g.turn_no));
        if pend.is_none() {
            // sans juge, les asks de ce segment sont tout de même comptés
        }
        g.apply(&a);
        add_asks(&mut script, k_done);
        k_done = asks.lock().unwrap().len();
    }
    for p in pos.iter_mut() {
        // une unité ciblée, ou une unité de n'importe qui vers le battlefield d'une cible, attaque plus tard dans le
        // tour : dégâts, Might « ce tour » et malus servent au combat d'après, que la note immédiate ne voit pas
        let turn = p.g("turn").int().unwrap();
        if let Some(e) = p.get("effect").and_then(|x| x.num()) {
            if e <= NOEFFECT
                && (p.g("tg").arr().iter().any(|u| moved.contains(&(turn, format!("u{}", u.int().unwrap()))))
                    || p.g("tg_loc").arr().iter().any(|l| moved.contains(&(turn, format!("bf{}", l.int().unwrap())))))
            {
                p.set("before_attack", true);
            }
        }
    }
    let game = jo! {
        "seed" => seed, "mode" => mode, "ver" => ver, "legends" => vec![s.decks[0].legend.clone(), s.decks[1].legend.clone()],
        "first" => s.first, "winner" => g.winner, "turns" => g.turn_no, "script" => Json::Arr(script),
    };
    (game, pos)
}

fn loc_idx(v: &Value) -> i64 {
    match v {
        Value::Loc(Loc::Bf(i)) => *i as i64,
        Value::Int(i) => *i,
        _ => -1,
    }
}

/// `job(arg)` : (partie, positions) ou (erreur, [])
pub fn job(seed: i64, mode: &str, ver: &str, script0: Option<&Arc<Vec<Json>>>) -> (Json, Vec<Json>) {
    let t0 = Instant::now();
    match pool::attempt(|| play(seed, mode, ver, true, script0)) {
        Err(e) => (jo! { "seed" => seed, "err" => e.chars().rev().take(1200).collect::<Vec<_>>().into_iter().rev().collect::<String>() }, vec![]),
        Ok((mut gm, pos)) => {
            gm.set("sec", py::round_n(t0.elapsed().as_secs_f64(), 1));
            (gm, pos)
        }
    }
}

// ---------------------------------------------------------------------- rejouer le banc avec une autre version
/// `rescore(game, pos, ver, only)` : positions `pos` (d'une même partie) rejouées : choix de la version `ver` à la même
/// position, noté par le juge.
pub fn rescore(game: &Json, pos: &[Json], ver: &str, only: Option<&str>) -> Vec<Json> {
    let seed = game.g("seed").int().unwrap();
    let mode = game.g("mode").str().unwrap();
    let s = setup(seed, mode);
    let orig = boxed(agents(seed, &s, game.g("ver").str().unwrap(), game.g("ver").str().unwrap()));
    let s2 = setup(seed, mode);
    let cand: Vec<Arc<Mutex<Box<dyn Agent>>>> = boxed(agents(seed, &s2, ver, ver)).into_iter().map(|a| Arc::new(Mutex::new(a))).collect();
    let script: Arc<Vec<Json>> = Arc::new(game.g("script").arr().to_vec());
    let ptr = Arc::new(AtomicUsize::new(0));
    let sa: Vec<Box<dyn Agent>> = orig.into_iter().map(|a| Box::new(ScriptAg { ag: a, script: script.clone(), ptr: ptr.clone() }) as Box<dyn Agent>).collect();
    let mut g = new_game(&s, seed, sa);
    let mut want: std::collections::BTreeMap<i64, Vec<&Json>> = Default::default();
    for p in pos {
        if only.is_none() || p.g("typ").str() == only {
            want.entry(p.g("step").int().unwrap()).or_default().push(p);
        }
    }
    let mut out = vec![];
    while !want.is_empty() {
        let Some(d) = g.advance(false) else { break };
        let step = ptr.load(Ordering::SeqCst) as i64;
        if let Some(ps) = want.remove(&step) {
            let cnt = counters(&g);
            let pre = g.clone_game();
            let opts = d.options.clone().unwrap();
            let a0 = opts[script[step as usize].arr()[1].int().unwrap() as usize].clone();
            // asks du segment d'origine (réponses enregistrées), pour Forced
            let mut seg = vec![];
            let mut k = step as usize + 1;
            while k < script.len() && script[k].arr()[0].str() == Some("a") {
                let e = script[k].arr();
                seg.push(Seg { enc: e[1].clone(), kind: e[2].str().unwrap().to_string(), n: e[3].int().unwrap() as usize });
                k += 1;
            }
            let seg = Arc::new(seg);
            for p in ps {
                let now = counters(&g);
                if p.g("typ").str() == Some("play") {
                    let mut c = pre.clone_game();
                    let a = cand[d.player].lock().unwrap().decide(&mut c, &d).expect("décision");
                    set_counters(&g, cnt);
                    let same = a.list()[0].opt_str() == Some("play") && a.list()[1] == a0.list()[1] && a.list()[2] == a0.list()[2];
                    let r = if same { judge_play(&pre, cnt, &d, &a) } else { None };
                    out.push(jo! {
                        "typ" => "play", "card" => p.g("card").clone(), "old" => p.g("regret").clone(),
                        "new" => r.as_ref().map(|r| r.g("regret").clone()).unwrap_or(Json::Null), "same_card" => same,
                        "changed" => a != a0, "old_eff" => p.g("effect").clone(),
                        "new_eff" => r.as_ref().and_then(|r| r.get("effect").cloned()).unwrap_or(Json::Null),
                        "before_attack" => p.g("before_attack").clone(), "seed" => p.g("seed").clone(), "step" => p.g("step").clone(),
                    });
                } else {
                    let pid = p.g("pid").int().unwrap() as usize;
                    let mut f = Forced::new(seg.clone(), -1, 0);
                    f.cand = Some(cand[pid].clone());
                    f.jj = p.g("j").int().unwrap();
                    replay(&pre, cnt, &a0, pid, Box::new(f.clone()), false, true);
                    let vals: Vec<(f64, f64)> = p.g("alts").arr().iter().map(|x| (x.arr()[1].num().unwrap(), x.arr()[2].num().unwrap())).collect();
                    let got = *f.got.lock().unwrap();
                    let new = match got {
                        Some(i) if i < vals.len() => Json::Float(regret2(&vals, i).0),
                        _ => Json::Null,
                    };
                    let chosen = p.g("chosen").int();
                    out.push(jo! {
                        "typ" => "ask", "kind" => p.g("kind").clone(), "old" => p.g("regret").clone(), "new" => new,
                        "changed" => got.map(|x| x as i64) != chosen, "seed" => p.g("seed").clone(), "step" => p.g("step").clone(),
                        "j" => p.g("j").clone(),
                    });
                }
                set_counters(&g, now);
            }
        }
        let Ok(a) = decide_with(&mut g, &d) else { break };
        g.apply(&a);
    }
    out
}

// ---------------------------------------------------------------------- bilan
pub fn is_error(p: &Json) -> bool {
    p.g("regret").num().unwrap_or(0.0) >= REGRET
}

/// `ratio_se(per_game)` : taux (erreurs / positions) et écart-type par grappes
pub fn ratio_se(per_game: &[(f64, f64)]) -> (f64, f64) {
    let m: f64 = py::sum(per_game.iter().map(|x| x.1));
    if m == 0.0 {
        return (0.0, 0.0);
    }
    let r = py::sum(per_game.iter().map(|x| x.0)) / m;
    let k = per_game.len() as f64;
    if per_game.len() < 2 {
        return (r, 0.0);
    }
    let v = k / (k - 1.0) * py::sum(per_game.iter().map(|(ei, mi)| (ei - r * mi).powi(2))) / m.powi(2);
    (r, v.sqrt())
}

fn count_by(items: Vec<String>) -> Vec<(String, i64)> {
    let mut by: Vec<(String, i64)> = vec![];
    for k in items {
        match by.iter_mut().find(|e| e.0 == k) {
            Some(e) => e.1 += 1,
            None => by.push((k, 1)),
        }
    }
    by.sort_by_key(|x| -x.1);
    by.truncate(25);
    by
}

/// `bilan(pos, games)`
pub fn bilan(pos: &[Json], games: &[Json]) -> PyVal {
    let mut seeds: Vec<i64> = games.iter().map(|g| g.g("seed").int().unwrap()).collect();
    seeds.sort();
    seeds.dedup();
    let mut out = crate::pydict! { "games" => seeds.len() as i64, "positions" => pos.len() as i64 };
    type Sel = fn(&Json) -> bool;
    let sels: [(&str, Sel); 3] = [
        ("play", |p| p.g("typ").str() == Some("play") && p.g("n_all").int().unwrap_or(0) > 1),
        ("ask", |p| p.g("typ").str() == Some("ask")),
        ("tout", |p| p.g("typ").str() == Some("ask") || p.g("n_all").int().unwrap_or(0) > 1),
    ];
    for (name, sel) in sels {
        let ps: Vec<&Json> = pos.iter().filter(|p| sel(p)).collect();
        let pg: Vec<(f64, f64)> = seeds
            .iter()
            .map(|&s| {
                (
                    ps.iter().filter(|p| p.g("seed").int() == Some(s) && is_error(p)).count() as f64,
                    ps.iter().filter(|p| p.g("seed").int() == Some(s)).count() as f64,
                )
            })
            .collect();
        let (r, se) = ratio_se(&pg);
        out.set(name, crate::pydict! {
            "n" => ps.len() as i64, "errors" => ps.iter().filter(|p| is_error(p)).count() as i64,
            "rate" => py::round_n(r, 4), "se" => py::round_n(se, 4),
        });
    }
    // cible hors de la liste de l'IA : la meilleure variante n'était pas proposée à l'IA (plafonds)
    let pl: Vec<&Json> = pos.iter().filter(|p| p.g("typ").str() == Some("play") && p.g("n_all").int().unwrap_or(0) > 1 && is_error(p)).collect();
    let unseen = pl
        .iter()
        .filter(|p| {
            // max(p["alts"], key=min(x[1], x[2])) : le premier maximum
            let mut best = &p.g("alts").arr()[0];
            for x in p.g("alts").arr() {
                if x.arr()[1].num().unwrap().min(x.arr()[2].num().unwrap()) > best.arr()[1].num().unwrap().min(best.arr()[2].num().unwrap()) {
                    best = x;
                }
            }
            !best.arr()[3].truthy()
        })
        .count();
    out.set("play_best_unseen", PyVal::Int(unseen as i64));
    let sp: Vec<&Json> = pos.iter().filter(|p| p.g("typ").str() == Some("play") && p.get("effect").is_some()).collect();
    let noe: Vec<&&Json> = sp.iter().filter(|p| p.g("effect").num().unwrap() <= NOEFFECT).collect();
    let pg: Vec<(f64, f64)> = seeds
        .iter()
        .map(|&s| {
            (
                noe.iter().filter(|p| p.g("seed").int() == Some(s) && !p.g("before_attack").truthy()).count() as f64,
                sp.iter().filter(|p| p.g("seed").int() == Some(s)).count() as f64,
            )
        })
        .collect();
    let (r, se) = ratio_se(&pg);
    out.set("spells", crate::pydict! {
        "n" => sp.len() as i64, "no_effect" => noe.len() as i64, "before_attack" => noe.iter().filter(|p| p.g("before_attack").truthy()).count() as i64,
        "rate_no_effect" => py::round_n(r, 4), "se" => py::round_n(se, 4),
    });
    let by = count_by(
        pos.iter()
            .filter(|p| is_error(p))
            .map(|p| if p.g("typ").str() == Some("play") { p.g("card").str().unwrap_or("").to_string() } else { format!("ask:{}", p.g("kind").str().unwrap_or("")) })
            .collect(),
    );
    out.set("errors_by", PyVal::List(by.into_iter().map(|(k, v)| PyVal::List(vec![k.into(), v.into()])).collect()));
    let by = count_by(noe.iter().filter(|p| !p.g("before_attack").truthy()).map(|p| p.g("card").str().unwrap_or("").to_string()).collect());
    out.set("no_effect_by", PyVal::List(by.into_iter().map(|(k, v)| PyVal::List(vec![k.into(), v.into()])).collect()));
    out
}

/// `rsummary(res)`
pub fn rsummary(res: &[Json]) -> PyVal {
    let mut out = crate::pydict! {};
    for typ in ["play", "ask"] {
        let rs: Vec<&Json> = res.iter().filter(|x| x.g("typ").str() == Some(typ)).collect();
        let sc: Vec<&&Json> = rs.iter().filter(|x| !x.g("new").is_null()).collect();
        let old = |x: &Json| x.g("old").num().unwrap();
        let new = |x: &Json| x.g("new").num().unwrap();
        out.set(typ, crate::pydict! {
            "n" => rs.len() as i64, "scored" => sc.len() as i64, "old_errors" => rs.iter().filter(|x| old(x) >= REGRET).count() as i64,
            "new_errors" => sc.iter().filter(|x| new(x) >= REGRET).count() as i64,
            "fixed" => sc.iter().filter(|x| old(x) >= REGRET && REGRET > new(x)).count() as i64,
            "broken" => sc.iter().filter(|x| new(x) >= REGRET && REGRET > old(x)).count() as i64,
            "changed" => rs.iter().filter(|x| x.g("changed").truthy()).count() as i64,
            "other_move" => rs.iter().filter(|x| typ == "play" && !x.g("same_card").truthy()).count() as i64,
        });
    }
    let sp: Vec<&Json> = res.iter().filter(|x| x.g("typ").str() == Some("play") && !x.g("old_eff").is_null()).collect();
    let ne = |e: &Json, x: &Json| e.num().is_some_and(|e| e <= NOEFFECT) && !x.g("before_attack").truthy();
    out.set("spells", crate::pydict! {
        "n" => sp.len() as i64, "old_no_effect" => sp.iter().filter(|x| ne(x.g("old_eff"), x)).count() as i64,
        "new_no_effect" => sp.iter().filter(|x| ne(x.g("new_eff"), x)).count() as i64,
        "new_not_played" => sp.iter().filter(|x| !x.g("same_card").truthy()).count() as i64,
    });
    out
}

fn pc(x: &PyVal) -> String {
    format!("{:.1} % ± {:.1}", 100.0 * x.get("rate").unwrap().as_f64(), 100.0 * x.get("se").unwrap().as_f64())
}

/// `show(b)`
pub fn show(b: &PyVal) {
    let g = |k: &str| b.get(k).unwrap();
    let i = |x: &PyVal, k: &str| x.get(k).unwrap().py_str();
    println!("{} parties, {} positions jugées", g("games").py_str(), g("positions").py_str());
    println!(
        "  cibles des cartes jouées : {}/{} erreurs (regret ≥ {}) = {} ; meilleure cible absente de la liste de l'IA : {}",
        i(g("play"), "errors"),
        i(g("play"), "n"),
        py::fstr(REGRET),
        pc(g("play")),
        g("play_best_unseen").py_str()
    );
    println!("  choix pendant la résolution : {}/{} = {}", i(g("ask"), "errors"), i(g("ask"), "n"), pc(g("ask")));
    println!("  ensemble : {}", pc(g("tout")));
    let s = g("spells");
    println!(
        "  sorts sans effet visible : {}/{} (dont {} avant une attaque de la cible) ; hors « avant attaque » {:.1} % ± {:.1}",
        i(s, "no_effect"),
        i(s, "n"),
        i(s, "before_attack"),
        100.0 * s.get("rate_no_effect").unwrap().as_f64(),
        100.0 * s.get("se").unwrap().as_f64()
    );
    let lst = |k: &str| g(k).as_list().iter().map(|e| format!("{} {}", e.as_list()[0].py_str(), e.as_list()[1].py_str())).collect::<Vec<_>>().join(", ");
    println!("  erreurs par carte / type : {}", lst("errors_by"));
    println!("  sans effet par carte : {}", lst("no_effect_by"));
}

/// `load(paths)` : (positions, parties)
pub fn load(paths: &[String]) -> (Vec<Json>, Vec<Json>) {
    let (mut pos, mut games) = (vec![], vec![]);
    for p in paths {
        for line in std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{p}: {e}")).lines() {
            if !line.trim().is_empty() {
                pos.push(json::parse(line).unwrap());
            }
        }
        let path = std::path::Path::new(p);
        let gp = path.with_file_name(path.file_name().unwrap().to_string_lossy().replace("pos-", "parties-"));
        if gp.exists() {
            for line in std::fs::read_to_string(&gp).unwrap().lines() {
                if !line.trim().is_empty() {
                    games.push(json::parse(line).unwrap());
                }
            }
        }
    }
    if games.is_empty() {
        let mut s: Vec<i64> = pos.iter().map(|p| p.g("seed").int().unwrap()).collect();
        s.sort();
        s.dedup();
        games = s.into_iter().map(|x| jo! { "seed" => x }).collect();
    }
    (pos, games)
}

/// `save(res, games, md5)` : ajoute un résultat à `results_banc.json`
pub fn save(mut res: PyVal, games: &[Json], md5: &str) -> PyVal {
    let seeds: Vec<i64> = games.iter().map(|x| x.g("seed").int().unwrap()).collect();
    res.set("seeds", crate::pydict! { "first" => *seeds.iter().min().unwrap(), "last" => *seeds.iter().max().unwrap(), "n" => games.len() as i64 });
    res.set("engine_md5", PyVal::Str(md5.to_string()));
    append_results(&res);
    res
}

fn append_results(res: &PyVal) {
    let f = banc_dir().join("results_banc.json");
    let mut all = if f.exists() { py::load_file(&f).unwrap() } else { PyVal::List(vec![]) };
    if let PyVal::List(v) = &mut all {
        v.push(res.clone());
    }
    py::dump_file(&f, &all, Dump { indent: Some(1), ensure_ascii: false });
}

fn jl(x: &Json) -> String {
    x.dumps(false)
}

/// `main(args)` de `banc.py`
pub fn main(args: &[String]) -> i32 {
    let md5 = super::version::engine_md5();
    let Some(cmd) = args.first() else {
        eprintln!("banc gen|bilan|rejuge|rejoue ...");
        return 2;
    };
    match cmd.as_str() {
        "bilan" => {
            let (pos, games) = load(&args[1..]);
            show(&bilan(&pos, &games));
            0
        }
        "rejuge" => {
            // rejuger des parties déjà jouées (juge modifié) : rejouées d'après leur script, sans recherche
            let fg = std::path::PathBuf::from(&args[1]);
            let procs: usize = args.get(2).map(|x| x.parse().unwrap()).unwrap_or(4);
            let games: Vec<Json> = std::fs::read_to_string(&fg).unwrap().lines().filter(|l| !l.trim().is_empty()).map(|l| json::parse(l).unwrap()).collect();
            let fp = fg.with_file_name(fg.file_name().unwrap().to_string_lossy().replace("parties-", "pos-"));
            let mut kept: Vec<Json> = vec![];
            let mut op = String::new();
            let todo: Vec<(i64, String, String, Arc<Vec<Json>>)> = games
                .iter()
                .map(|x| (x.g("seed").int().unwrap(), x.g("mode").str().unwrap().to_string(), x.g("ver").str().unwrap().to_string(), Arc::new(x.g("script").arr().to_vec())))
                .collect();
            let res = pool::map(procs, &todo, |(s, m, v, sc)| job(*s, m, v, Some(sc)));
            for (gm, pos) in res {
                if gm.get("err").is_some() {
                    println!("ERREUR {} {}", gm.g("seed").int().unwrap(), gm.g("err").str().unwrap_or("").chars().rev().take(400).collect::<Vec<_>>().into_iter().rev().collect::<String>());
                    continue;
                }
                let seed = gm.g("seed").int().unwrap();
                let mut old = games.iter().find(|x| x.g("seed").int() == Some(seed)).unwrap().clone();
                if old.g("engine").str() != Some(md5.as_str()) {
                    let played = if old.g("engine_played").truthy() { old.g("engine_played").clone() } else { old.g("engine").clone() };
                    old.set("engine_played", played);
                    old.set("engine", md5.clone());
                }
                kept.push(old);
                for x in pos {
                    op.push_str(&jl(&x));
                    op.push('\n');
                }
            }
            std::fs::write(&fp, op).unwrap();
            kept.sort_by_key(|x| x.g("seed").int().unwrap());
            std::fs::write(&fg, kept.iter().map(|x| jl(x) + "\n").collect::<String>()).unwrap();
            println!("{}/{} parties rejouées", kept.len(), games.len());
            let (pos, games2) = load(&[fp.display().to_string()]);
            let b = bilan(&pos, &games2);
            show(&b);
            let mut r = crate::pydict! {
                "label" => format!("banc {} · {} (rejugé)", games2[0].g("ver").str().unwrap_or(""), games2[0].g("mode").str().unwrap_or("")),
                "mode" => games2[0].g("mode").clone().str().unwrap_or("").to_string(), "ver" => games2[0].g("ver").str().unwrap_or("").to_string(),
                "file" => fg.file_name().unwrap().to_string_lossy().to_string(), "regret" => REGRET,
            };
            if let PyVal::Dict(kv) = b {
                for (k, v) in kv {
                    r.set(&k, v);
                }
            }
            save(r, &games2, &md5);
            0
        }
        "rejoue" => {
            let (fp, ver) = (&args[1], &args[2]);
            let procs: usize = args.get(3).map(|x| x.parse().unwrap()).unwrap_or(4);
            let only = args.get(4).map(|s| s.as_str()).filter(|s| *s == "play" || *s == "ask");
            let (pos, games) = load(std::slice::from_ref(fp));
            let t0 = Instant::now();
            let jobs: Vec<(Json, Vec<Json>)> = games
                .iter()
                .map(|gm| (gm.clone(), pos.iter().filter(|p| p.g("seed") == gm.g("seed")).cloned().collect()))
                .collect();
            let res: Vec<Json> = pool::map(procs, &jobs, |(gm, ps)| match pool::attempt(|| rescore(gm, ps, ver, only)) {
                Ok(v) => v,
                Err(e) => vec![jo! { "err" => e.chars().rev().take(800).collect::<Vec<_>>().into_iter().rev().collect::<String>(), "seed" => gm.g("seed").clone() }],
            })
            .into_iter()
            .flatten()
            .collect();
            let errs: Vec<&Json> = res.iter().filter(|x| x.get("err").is_some()).collect();
            let res2: Vec<Json> = res.iter().filter(|x| x.get("err").is_none()).cloned().collect();
            for e in errs.iter().take(3) {
                println!("ERREUR {} {}", e.g("seed").int().unwrap(), e.g("err").str().unwrap_or("").chars().rev().take(400).collect::<Vec<_>>().into_iter().rev().collect::<String>());
            }
            let name = std::path::Path::new(fp).file_name().unwrap().to_string_lossy().to_string();
            let outn = format!("rejoue-{}-{}", ver.replace('@', "_").replace(',', "_").replace('=', "-"), name);
            std::fs::write(std::path::Path::new(fp).with_file_name(outn), res2.iter().map(|x| jl(x) + "\n").collect::<String>()).unwrap();
            let mut out = rsummary(&res2);
            let mut eng: Vec<String> = games.iter().filter_map(|g| g.get("engine").and_then(|e| e.str()).map(|s| s.to_string())).collect();
            eng.sort();
            eng.dedup();
            for (k, v) in [
                ("label", PyVal::Str(format!("banc rejoué {ver} sur {name}"))),
                ("ver", PyVal::Str(ver.clone())),
                ("file", PyVal::Str(name.clone())),
                ("errors", PyVal::Int(errs.len() as i64)),
                ("engine_md5", PyVal::Str(md5.clone())),
                ("games_engine", PyVal::from(eng)),
                ("seconds", PyVal::Int(py::round0(t0.elapsed().as_secs_f64()))),
            ] {
                out.set(k, v);
            }
            println!("{}", py::dumps(&out, Dump { indent: Some(1), ensure_ascii: false }));
            append_results(&out);
            0
        }
        _ => {
            // gen <parties> <graine0> <mode> [version] [processus]
            let (n, seed0, mode): (i64, i64, String) = (args[1].parse().unwrap(), args[2].parse().unwrap(), args[3].clone());
            let ver = args.get(4).cloned().unwrap_or_else(|| "turn".into());
            let procs: usize = args.get(5).map(|x| x.parse().unwrap()).unwrap_or(4);
            let dd = banc_dir().join("data");
            std::fs::create_dir_all(&dd).ok();
            let (fp, fg) = (dd.join(format!("pos-{seed0}.jsonl")), dd.join(format!("parties-{seed0}.jsonl")));
            let mut done: Vec<i64> = vec![];
            if fg.exists() {
                for line in std::fs::read_to_string(&fg).unwrap().lines().filter(|l| !l.trim().is_empty()) {
                    let x = json::parse(line).unwrap();
                    if x.g("engine").str() != Some(md5.as_str()) {
                        eprintln!("{} : autre version du moteur ({} au lieu de {md5}) ; changer de graine0", fg.display(), x.g("engine").str().unwrap_or("None"));
                        return 1;
                    }
                    done.push(x.g("seed").int().unwrap());
                }
            }
            let todo: Vec<i64> = (0..n).map(|k| seed0 + k).filter(|s| !done.contains(s)).collect();
            let t0 = Instant::now();
            let mut errs = 0;
            let open = |p: &std::path::Path| std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
            let (mut op, mut og) = (open(&fp), open(&fg));
            use std::io::Write;
            pool::imap(procs, &todo, |&s| job(s, &mode, &ver, None), |_, (mut gm, pos)| {
                if gm.get("err").is_some() {
                    errs += 1;
                    println!("ERREUR {} {}", gm.g("seed").int().unwrap(), gm.g("err").str().unwrap_or("").chars().rev().take(400).collect::<Vec<_>>().into_iter().rev().collect::<String>());
                    return true;
                }
                gm.set("engine", md5.clone());
                writeln!(og, "{}", jl(&gm)).unwrap();
                for x in &pos {
                    writeln!(op, "{}", jl(x)).unwrap();
                }
                println!("graine {} : {} positions, {} s", gm.g("seed").int().unwrap(), pos.len(), py::fstr(gm.g("sec").num().unwrap()));
                true
            });
            let (pos, games) = load(&[fp.display().to_string()]);
            let b = bilan(&pos, &games);
            show(&b);
            let mut r = crate::pydict! {
                "label" => format!("banc {ver} · {mode}"), "mode" => mode.clone(), "ver" => ver.clone(),
                "file" => fg.file_name().unwrap().to_string_lossy().to_string(), "regret" => REGRET, "errors" => errs as i64,
                "seconds" => py::round0(t0.elapsed().as_secs_f64()),
            };
            if let PyVal::Dict(kv) = b {
                for (k, v) in kv {
                    r.set(&k, v);
                }
            }
            save(r, &games, &md5);
            0
        }
    }
}

// =================================================================================================== cout.py
/// `best_alt(p)` : (indice du choix de l'IA, indice de la meilleure option pour les deux poids)
pub fn best_alt(p: &Json) -> (usize, usize) {
    let vals: Vec<(f64, f64)> = p.g("alts").arr().iter().map(|x| (x.arr()[1].num().unwrap(), x.arr()[2].num().unwrap())).collect();
    let i = if p.g("typ").str() == Some("ask") { p.g("chosen").int().unwrap() as usize } else { 0 };
    let mut best = 0usize;
    let key = |k: usize| (vals[k].0 - vals[i].0).min(vals[k].1 - vals[i].1);
    for k in 1..vals.len() {
        if key(k) > key(best) {
            best = k;
        }
    }
    (i, best)
}

/// `at_step(game, step)` : partie rejouée d'après son script jusqu'à la décision `step` : (état copié, compteurs,
/// décision, coup de l'IA, asks du segment qui suit).
pub fn at_step(game: &Json, step: usize) -> (Game, (u32, u32), Decision, Value, Vec<Seg>) {
    let seed = game.g("seed").int().unwrap();
    let mode = game.g("mode").str().unwrap();
    let s = setup(seed, mode);
    let v = game.g("ver").str().unwrap();
    let script: Arc<Vec<Json>> = Arc::new(game.g("script").arr().to_vec());
    let ptr = Arc::new(AtomicUsize::new(0));
    let sa: Vec<Box<dyn Agent>> =
        boxed(agents(seed, &s, v, v)).into_iter().map(|a| Box::new(ScriptAg { ag: a, script: script.clone(), ptr: ptr.clone() }) as Box<dyn Agent>).collect();
    let mut g = new_game(&s, seed, sa);
    loop {
        let Some(d) = g.advance(false) else { panic!("Desync: partie finie avant le pas {step}") };
        if ptr.load(Ordering::SeqCst) == step {
            let opts = d.options.clone().unwrap();
            let a0 = opts[script[step].arr()[1].int().unwrap() as usize].clone();
            let mut seg = vec![];
            let mut k = step + 1;
            while k < script.len() && script[k].arr()[0].str() == Some("a") {
                let e = script[k].arr();
                seg.push(Seg { enc: e[1].clone(), kind: e[2].str().unwrap().to_string(), n: e[3].int().unwrap() as usize });
                k += 1;
            }
            return (g.clone_game(), counters(&g), d, a0, seg);
        }
        let a = decide_with(&mut g, &d).expect("décision");
        g.apply(&a);
    }
}

/// État de `Then` partagé par les deux agents de la suite (`st` de Python)
#[derive(Clone, Default)]
pub struct ThenState {
    pub k: Option<usize>,
    pub j: usize,
    pub alt: usize,
    pub seg: Arc<Vec<Seg>>,
    pub hit: Option<(String, usize)>,
}

/// `Then` : agent de la suite : les asks du segment d'avant le j-ième rejoués, le j-ième forcé sur alt, puis l'agent.
#[derive(Clone)]
pub struct Then {
    pub ag: Box<dyn Agent>,
    pub st: Arc<Mutex<ThenState>>,
}

impl Agent for Then {
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        self.ag.decide(g, d)
    }
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        {
            let mut st = self.st.lock().unwrap();
            if let Some(k) = st.k {
                st.k = Some(k + 1);
                if k == st.j {
                    st.k = None;
                    st.hit = Some((kind.to_string(), options.len()));
                    return Ok(st.alt);
                }
                if k < st.seg.len() {
                    if let Some(r) = dec(&st.seg[k].enc, options) {
                        return Ok(r);
                    }
                }
            }
        }
        self.ag.choose(g, pid, kind, options, ctx)
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        self.ag.mulligan(g, pid)
    }
    fn start(&mut self, g: &mut Game) {
        self.ag.start(g)
    }
    fn every_choice(&self) -> bool {
        self.ag.every_choice()
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// `playout(game, pre, cnt, action, st, k, ver)` : (gagnant, ask retrouvé)
pub fn playout(game: &Json, pre: &Game, cnt: (u32, u32), action: &Value, st: Option<ThenState>, k: i64, ver: &str) -> (Option<i64>, Option<(String, usize)>) {
    set_counters(pre, cnt);
    let mut c = pre.clone_game();
    let seed = game.g("seed").int().unwrap();
    let w = seed * 1000 + k;
    let mut r = crate::pyrandom::PyRandom::new(w as i128);
    for pl in c.p.iter_mut() {
        r.shuffle(&mut pl.deck);
    }
    c.rng = crate::pyrandom::PyRandom::new(w as i128 * 2 + 1);
    let s = setup(seed, game.g("mode").str().unwrap());
    let st_arc = Arc::new(Mutex::new(st.clone().unwrap_or_default()));
    let ags: Vec<Box<dyn Agent>> = boxed(agents(w, &s, ver, ver))
        .into_iter()
        .map(|a| Box::new(Then { ag: a, st: st_arc.clone() }) as Box<dyn Agent>)
        .collect();
    c.agents = crate::game::Agents(ags);
    c.apply(action);
    for _ in 0..6000 {
        let Some(d) = c.advance(false) else { break };
        let ph = c.agents[d.player].clone_box();
        let mut a = std::mem::replace(&mut c.agents[d.player], ph);
        let r = a.decide(&mut c, &d);
        c.agents[d.player] = a;
        match r {
            Ok(act) => c.apply(&act),
            Err(_) => break,
        }
    }
    let hit = st_arc.lock().unwrap().hit.clone();
    (c.winner, hit)
}

/// `cout.job(arg)`
pub fn cout_job(game: &Json, p: &Json, k: i64, ver: &str) -> Json {
    let res = pool::attempt(|| {
        let step = p.g("step").int().unwrap() as usize;
        let (pre, cnt, d, a0, seg) = at_step(game, step);
        let (bad, good) = best_alt(p);
        let (acts, sts): ([Value; 2], [Option<ThenState>; 2]);
        if p.g("typ").str() == Some("play") {
            let alts = play_alts(&pre, &d, &a0).expect("variantes").1;
            acts = [alts[bad].clone(), alts[good].clone()];
            sts = [None, None];
        } else {
            acts = [a0.clone(), a0.clone()];
            let seg = Arc::new(seg);
            let mk = |alt: usize| Some(ThenState { k: Some(0), j: p.g("j").int().unwrap() as usize, alt, seg: seg.clone(), hit: None });
            sts = [mk(bad), mk(good)];
        }
        let pid = p.g("pid").int().unwrap();
        let sc = |w: Option<i64>| match w {
            None | Some(-1) => 0.5,
            Some(x) if x == pid => 1.0,
            _ => 0.0,
        };
        let mut out = vec![];
        for (a, st) in acts.iter().zip(sts) {
            let want = st.is_some();
            let (w, hit) = playout(game, &pre, cnt, a, st, k, ver);
            if want && hit != Some((p.g("kind").str().unwrap_or("").to_string(), p.g("n_all").int().unwrap() as usize)) {
                panic!("Desync: ask {} non retrouvé : {:?}", p.g("j").int().unwrap_or(-1), hit);
            }
            out.push(sc(w));
        }
        (out[0], out[1])
    });
    match res {
        Ok((bad, good)) => jo! {
            "seed" => p.g("seed").clone(), "step" => p.g("step").clone(), "j" => p.g("j").clone(), "k" => k, "bad" => bad, "good" => good,
        },
        Err(e) => jo! {
            "seed" => p.g("seed").clone(), "step" => p.g("step").clone(), "k" => k,
            "err" => e.chars().rev().take(600).collect::<Vec<_>>().into_iter().rev().collect::<String>(),
        },
    }
}

/// `cluster(per_game)` : moyenne des écarts par erreur et écart-type par grappes de parties
pub fn cluster(per_game: &[Vec<f64>]) -> (f64, f64) {
    let xs: Vec<f64> = per_game.iter().flatten().copied().collect();
    let n = xs.len();
    if n < 2 {
        return (xs.first().copied().unwrap_or(0.0), 0.0);
    }
    let m = py::sum(xs.iter().copied()) / n as f64;
    let k = per_game.len() as f64;
    let v = k / (k - 1.0) * py::sum(per_game.iter().map(|g| (py::sum(g.iter().copied()) - m * g.len() as f64).powi(2))) / (n as f64).powi(2);
    (m, v.sqrt())
}

/// `main(args)` de `cout.py`
pub fn cout_main(args: &[String]) -> i32 {
    let fp = std::path::PathBuf::from(&args[0]);
    let kk: i64 = args.get(1).map(|x| x.parse().unwrap()).unwrap_or(16);
    let ver = args.get(2).cloned().unwrap_or_else(|| "sh".into());
    let procs: usize = args.get(3).map(|x| x.parse().unwrap()).unwrap_or(4);
    let thr: f64 = args.get(4).map(|x| x.parse().unwrap()).unwrap_or(REGRET);
    let (pos, games) = load(&[fp.display().to_string()]);
    let errs: Vec<&Json> = pos
        .iter()
        .filter(|p| p.g("regret").num().unwrap() >= thr && (p.g("typ").str() == Some("ask") || p.g("n_all").int().unwrap() > 1))
        .collect();
    let jobs: Vec<(&Json, &Json, i64)> = errs
        .iter()
        .flat_map(|p| {
            let gm = games.iter().find(|g| g.g("seed") == p.g("seed")).expect("partie du banc");
            (0..kk).map(move |k| (gm, *p, k))
        })
        .collect();
    let t0 = Instant::now();
    let total = jobs.len();
    let mut res: Vec<((i64, i64, Option<i64>), Vec<Json>)> = vec![];
    let mut nerr = 0;
    pool::imap(procs, &jobs, |&(gm, p, k)| cout_job(gm, p, k, &ver), |i, r| {
        if r.get("err").is_some() {
            nerr += 1;
            println!("ERREUR {} {} {}", r.g("seed").int().unwrap(), r.g("step").int().unwrap(), r.g("err").str().unwrap_or("").chars().rev().take(300).collect::<Vec<_>>().into_iter().rev().collect::<String>());
            return true;
        }
        let key = (r.g("seed").int().unwrap(), r.g("step").int().unwrap(), r.g("j").int());
        match res.iter_mut().find(|e| e.0 == key) {
            Some(e) => e.1.push(r),
            None => res.push((key, vec![r])),
        }
        if (i + 1) % 100 == 0 {
            println!("{}/{} parties, {} s", i + 1, total, py::round0(t0.elapsed().as_secs_f64()));
        }
        true
    });
    let mut rows: Vec<Json> = vec![];
    for p in &errs {
        let key = (p.g("seed").int().unwrap(), p.g("step").int().unwrap(), p.g("j").int());
        let Some(rs) = res.iter().find(|e| e.0 == key).map(|e| &e.1) else { continue };
        let n = rs.len() as f64;
        let g_ = py::sum(rs.iter().map(|r| r.g("good").num().unwrap())) / n;
        let b_ = py::sum(rs.iter().map(|r| r.g("bad").num().unwrap())) / n;
        let what = if p.g("card").truthy() { p.g("card").str().unwrap_or("").to_string() } else { format!("ask:{}", p.g("kind").str().unwrap_or("")) };
        rows.push(jo! {
            "seed" => p.g("seed").clone(), "step" => p.g("step").clone(), "typ" => p.g("typ").clone(), "what" => what,
            "src" => p.get("src").cloned().unwrap_or(Json::Null), "regret" => p.g("regret").clone(), "n" => rs.len(),
            "good" => py::round_n(g_, 4), "bad" => py::round_n(b_, 4), "cost" => py::round_n(g_ - b_, 4),
        });
    }
    let name = fp.file_name().unwrap().to_string_lossy().to_string();
    std::fs::write(fp.with_file_name(format!("cout-{name}")), rows.iter().map(|r| jl(r) + "\n").collect::<String>()).unwrap();
    let mut out = crate::pydict! {
        "label" => format!("coût des erreurs du banc, suite {ver}, {kk} tirages"), "file" => name.clone(), "ver" => ver.clone(), "K" => kk,
        "regret" => thr, "errors_measured" => rows.len() as i64, "playout_errors" => nerr as i64, "seconds" => py::round0(t0.elapsed().as_secs_f64()),
    };
    let sels: [(&str, Box<dyn Fn(&Json) -> bool>); 3] = [
        ("tout", Box::new(|_| true)),
        ("play", Box::new(|r| r.g("typ").str() == Some("play"))),
        ("ask", Box::new(|r| r.g("typ").str() == Some("ask"))),
    ];
    for (nm, sel) in sels {
        let rs: Vec<&Json> = rows.iter().filter(|r| sel(r)).collect();
        let mut seeds: Vec<i64> = rs.iter().map(|r| r.g("seed").int().unwrap()).collect();
        seeds.sort();
        seeds.dedup();
        let per: Vec<Vec<f64>> = seeds.iter().map(|&s| rs.iter().filter(|r| r.g("seed").int() == Some(s)).map(|r| r.g("cost").num().unwrap()).collect()).collect();
        let (m, se) = cluster(&per);
        let nn = 1usize.max(rs.len()) as f64;
        out.set(nm, crate::pydict! {
            "n" => rs.len() as i64, "cost" => py::round_n(m, 4), "se" => py::round_n(se, 4),
            "good" => py::round_n(py::sum(rs.iter().map(|r| r.g("good").num().unwrap())) / nn, 4),
            "bad" => py::round_n(py::sum(rs.iter().map(|r| r.g("bad").num().unwrap())) / nn, 4),
        });
    }
    let out = save(out, &games, &super::version::engine_md5()); // save() ajoute seeds et engine_md5, imprimés ensuite
    println!("{}", py::dumps(&out, Dump { indent: Some(1), ensure_ascii: false }));
    let mut top: Vec<&Json> = rows.iter().collect();
    top.sort_by(|a, b| b.g("cost").num().unwrap().partial_cmp(&a.g("cost").num().unwrap()).unwrap());
    for r in top.iter().take(12) {
        println!(
            "  {:<28} {:<24} regret {:>6} : bon {:.2}, mauvais {:.2}, coût {:+.2}",
            r.g("what").str().unwrap_or(""),
            r.g("src").str().unwrap_or(""),
            py::fstr(r.g("regret").num().unwrap()),
            r.g("good").num().unwrap(),
            r.g("bad").num().unwrap(),
            r.g("cost").num().unwrap()
        );
    }
    0
}
