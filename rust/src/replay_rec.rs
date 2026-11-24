//! Enregistreur de replays (`replay.py` : `RecGame`, `RecAgent`, `record`, `record_plan`, `summary`). Une partie IA
//! contre IA est jouée en photographiant l'état (`replay::snap`) à chaque ligne du journal ; le résultat est le dict
//! JSON que lit le lecteur de replays (`riftbound/replays/`). Les copies de partie faites par l'IA n'enregistrent
//! rien (`Game::clone_game` remet `rec` à None, comme `RecGame.clone`).
//!
//! Écart avec Python : le dict `cards` (base de cartes) est écrit dans l'ordre alphabétique ; Python itère un `set`
//! de noms, dont l'ordre change d'un processus à l'autre (PYTHONHASHSEED). Le contenu est le même.
use std::sync::Arc;

use crate::ai::{evaluate, Cfg, SearchAgent, SearchOpts};
use crate::game::{Agent, Counters, DeckList, Decision, Game, Res, O};
use crate::jo;
use crate::json::Json;
use crate::outils::py::round_n;
use crate::plans::{plan_agent, plans, Plan};
use crate::pyrandom::PyRandom;
use crate::replay::{card_db, describe, fr_line, names, snap};
use crate::train::TDeck;
use crate::value::Value;

/// Ce que `RecGame` ajoute à une partie : images, notes en attente, décision en attente, mulligans.
#[derive(Clone, Default)]
pub struct RecExt {
    pub frames: Vec<Json>,
    pub pending: Vec<String>,
    pub pdec: Option<Json>,
    /// `g.mull` : joueur -> {hand, out}, dans l'ordre d'insertion
    pub mull: Vec<(usize, Json)>,
}

fn obj_mut<'a>(j: &'a mut Json, k: &str) -> Option<&'a mut Json> {
    match j {
        Json::Obj(v) => v.iter_mut().find(|(x, _)| x == k).map(|(_, j)| j),
        _ => None,
    }
}

/// `round(max(-60, min(60, evaluate(s, 0))), 1)` : les bornes sont des entiers Python (`round(int, 1)` reste entier).
fn ev0(g: &mut Game) -> Json {
    let x = evaluate(g, 0, None);
    if x >= 60.0 {
        Json::Int(60)
    } else if x <= -60.0 {
        Json::Int(-60)
    } else {
        Json::Float(round_n(x, 1))
    }
}

/// `UID.findall(raw)` : les nombres qui suivent un « # ».
fn uids(raw: &str) -> Vec<i64> {
    let b = raw.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'#' {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                out.push(raw[i + 1..j].parse().unwrap());
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// `RecGame.log` (après `Game.log`) : une image de l'état, ou la ligne ajoutée à la dernière image si l'état n'a pas
/// changé.
pub fn rec_log(g: &mut Game, raw: &str) {
    let txt = fr_line(g, raw);
    let st = snap(g);
    let ids = uids(raw);
    let ind: i64 = if raw.starts_with("  ") { 1 } else { 0 };
    {
        let r = g.rec.as_mut().unwrap();
        if r.pending.is_empty() && r.pdec.is_none() && !raw.starts_with("=====") {
            if let Some(last) = r.frames.last_mut() {
                if obj_mut(last, "st").is_some_and(|s| *s == st) {
                    if let Some(Json::Arr(ev)) = obj_mut(last, "ev") {
                        ev.push(Json::Arr(vec![Json::Int(ind), Json::Str(txt)]));
                    }
                    if let Some(Json::Arr(hl)) = obj_mut(last, "hl") {
                        let mut s: std::collections::BTreeSet<i64> = hl.iter().filter_map(|x| x.int()).collect();
                        s.extend(ids);
                        *hl = s.into_iter().map(Json::Int).collect();
                    }
                    return;
                }
            }
        }
    }
    let e = ev0(g);
    let r = g.rec.as_mut().unwrap();
    let mut f = jo! {
        "ev" => Json::Arr(vec![Json::Arr(vec![Json::Int(ind), Json::Str(txt)])]), "st" => st,
        "hl" => Json::Arr(ids.into_iter().map(Json::Int).collect()), "ev0" => e,
    };
    if !r.pending.is_empty() {
        f.set("notes", Json::Arr(std::mem::take(&mut r.pending).into_iter().map(Json::Str).collect()));
    }
    if let Some(d) = r.pdec.take() {
        f.set("dec", d);
    }
    r.frames.push(f);
}

/// `RecGame.decision_frame`
pub fn decision_frame(g: &mut Game, pid: usize, kind: &str, chosen: &str, alts: Json, n_opts: usize) {
    let st = snap(g);
    let e = ev0(g);
    let r = g.rec.as_mut().unwrap();
    let mut f = jo! {
        "ev" => Json::Arr(vec![Json::Arr(vec![Json::Int(0), Json::Str(format!("{} décide : {}", names()[pid], chosen))])]),
        "st" => st, "hl" => Json::Arr(vec![]), "dec" => jo! { "p" => pid, "k" => kind, "n" => n_opts, "alts" => alts }, "ev0" => e,
    };
    if !r.pending.is_empty() {
        f.set("notes", Json::Arr(std::mem::take(&mut r.pending).into_iter().map(Json::Str).collect()));
    }
    r.frames.push(f);
}

/// `RecAgent` : SearchAgent identique (mêmes tirages aléatoires) qui note les scores des options. `decide` est la
/// copie ancienne de celui de SearchAgent (sans suite du tour déjà choisie, sans `spell_pre`, `react_n` ni
/// `tie_break`), comme en Python.
#[derive(Clone)]
pub struct RecAgent {
    pub inner: SearchAgent,
}

impl Agent for RecAgent {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &crate::value::Map) -> Res<usize> {
        self.inner.choose(g, pid, kind, options, ctx)
    }
    fn start(&mut self, g: &mut Game) {
        self.inner.start(g)
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        let out = self.inner.mulligan(g, pid)?;
        if g.rec.is_some() {
            let hand: Vec<Json> = g.p[pid].hand.clone().into_iter().map(|c| Json::from(g.cname(c))).collect();
            let outn: Vec<Json> = out.iter().map(|&c| Json::from(g.cname(c))).collect();
            let v = jo! { "hand" => Json::Arr(hand), "out" => Json::Arr(outn) };
            let r = g.rec.as_mut().unwrap();
            match r.mull.iter_mut().find(|e| e.0 == pid) {
                Some(e) => e.1 = v,
                None => r.mull.push((pid, v)),
            }
        }
        Ok(out)
    }
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        let full: Vec<Value> = d.options.clone().expect("décision sans options");
        if full.len() == 1 {
            return Ok(full[0].clone());
        }
        let me = d.player;
        let mut opts = full.clone();
        if self.inner.cfg.truthy("respect_ambush", false) && d.kind == "main" {
            let f: Vec<Value> = opts.iter().filter(|a| !self.inner.risky_attack(g, me, a)).cloned().collect();
            if !f.is_empty() {
                opts = f;
            }
        }
        if opts.len() > self.inner.max_cands {
            let rest = self.inner.rng.sample(&opts[1..], self.inner.max_cands - 1);
            let mut o = vec![opts[0].clone()];
            o.extend(rest);
            opts = o;
        }
        let (best, best_v, mut scored) = self.inner.pick(g, d, &opts)?;
        if g.rec.is_some() {
            scored.sort_by(|a, b| (-a.0).partial_cmp(&(-b.0)).unwrap());
            let mut alts: Vec<Json> = vec![];
            for (v, a) in scored.iter().take(6) {
                let t = describe(g, a, me, None);
                alts.push(Json::Arr(vec![Json::Str(t), Json::Float(round_n(v - best_v, 1)), Json::Int((*a == best) as i64)]));
            }
            if !scored.iter().take(6).any(|(_, a)| *a == best) {
                let t = describe(g, &best, me, None);
                alts.push(Json::Arr(vec![Json::Str(t), Json::Float(0.0), Json::Int(1)]));
            }
            let kind = best.list()[0].str().to_string();
            if kind == "pass" {
                // une fenêtre de réaction laissée passer : simple note sur l'image suivante
                if scored.len() > 1 {
                    if let Some((sv, sa)) = scored.iter().find(|x| x.1.list()[0].str() != "pass").cloned() {
                        let gap = sv - best_v;
                        let gap = if gap < -1000.0 { "perdait la partie".to_string() } else { format!("{gap:.1}") };
                        let t = describe(g, &sa, me, None);
                        let note = format!("{} pouvait réagir ({t}, {gap}) mais passe", names()[me]);
                        g.rec.as_mut().unwrap().pending.push(note);
                    }
                }
            } else if kind == "end" {
                let t = describe(g, &best, me, None);
                decision_frame(g, me, d.kind, &t, Json::Arr(alts), full.len());
            } else {
                g.rec.as_mut().unwrap().pdec = Some(jo! { "p" => me, "k" => d.kind, "n" => full.len(), "alts" => Json::Arr(alts) });
            }
        }
        Ok(best)
    }
    fn every_choice(&self) -> bool {
        false
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// Joue la partie (compteurs d'objets neufs) en l'enregistrant ; renvoie la partie finie.
pub fn play_rec(decks: [DeckList; 2], agents: Vec<Box<dyn Agent>>, seed: i64, first: usize) -> Game {
    let mut g = Game::new(&decks, agents, Some(first), seed as i128, true, crate::spec::VICTORY as i64, Arc::new(Counters::default()));
    g.rec = Some(Box::default());
    for pid in 0..2 {
        let mut a = std::mem::replace(&mut g.agents[pid], Box::new(crate::game_events::FirstChoice));
        a.start(&mut g);
        g.agents[pid] = a;
    }
    loop {
        let Some(d) = g.advance(false) else { break };
        let ph = g.agents[d.player].clone_box();
        let mut a = std::mem::replace(&mut g.agents[d.player], ph);
        let r = a.decide(&mut g, &d);
        g.agents[d.player] = a;
        match r {
            Ok(act) => g.apply(&act),
            Err(_) => break,
        }
    }
    g
}

/// La fin commune de `record` et `record_plan` : le dict du replay (`plans` : `record_plan` seulement).
fn report(g: &Game, seed: i64, first: usize, a: &TDeck, l: &TDeck, plans: Option<[String; 2]>) -> Json {
    let mut nm: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for d in [a, l] {
        nm.extend(d.main.iter().cloned());
        nm.insert(d.legend.clone());
        if let Some(c) = &d.champion {
            nm.insert(c.clone());
        }
    }
    for x in ["Mech", "Reflection", "Gold"] {
        nm.insert(x.to_string());
    }
    let rec = g.rec.as_ref().unwrap();
    for fr in &rec.frames {
        for p in fr.g("st").g("p").arr() {
            for c in p.g("hand").arr() {
                if let Some(n) = c.arr().get(1).and_then(|x| x.str()) {
                    nm.insert(n.to_string());
                }
            }
        }
    }
    let nms = names();
    let players: Vec<Json> = [a, l]
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let mut uniq: Vec<&String> = vec![];
            for n in &d.main {
                if !uniq.contains(&n) {
                    uniq.push(n);
                }
            }
            jo! {
                "name" => nms[i].clone(), "legend" => d.legend.clone(), "champion" => d.champion.clone(),
                "bf" => d.battlefield.clone(), "deck" => d.key.clone(), "main" => Json::Arr(uniq.into_iter().map(Json::from).collect()),
            }
        })
        .collect();
    let mull = Json::Obj(rec.mull.iter().map(|(p, v)| (p.to_string(), v.clone())).collect());
    let pts: Vec<i64> = g.p.iter().map(|p| p.points).collect();
    let mut rep = jo! {
        "seed" => seed, "first" => first, "winner" => g.winner, "turns" => g.turn_no, "pts" => pts,
    };
    if let Some(p) = plans {
        rep.set("plans", Json::Arr(p.iter().map(Json::from).collect()));
    }
    rep.set("players", Json::Arr(players));
    rep.set("mulligan", mull);
    rep.set("frames", Json::Arr(rec.frames.clone()));
    rep.set("cards", card_db(nm.iter().filter(|n| !n.is_empty()).map(|n| n.as_str())));
    rep
}

/// `record(seed, a_deck, l_deck, a_bf, l_bf, first, cfg)` : joue la partie `seed` comme `exp.one` (mêmes tirages de
/// battlefield et de premier joueur) et l'enregistre.
pub fn record(seed: i64, a_deck: &TDeck, l_deck: &TDeck, a_bf: Option<&str>, l_bf: Option<&str>, first: Option<usize>, cfg: Option<Cfg>) -> Json {
    let mut r = PyRandom::new(seed as i128 * 7919);
    let abf = a_bf.map(|x| x.to_string()).unwrap_or_else(|| r.choice(&a_deck.battlefields).clone());
    let lbf = l_bf.map(|x| x.to_string()).unwrap_or_else(|| r.choice(&l_deck.battlefields).clone());
    let f = first.unwrap_or_else(|| r.randrange(2) as usize);
    let (a, l) = (a_deck.with_bf(&abf), l_deck.with_bf(&lbf));
    let o0 = SearchOpts { cfg: cfg.unwrap_or_default(), ..Default::default() };
    let ag: Vec<Box<dyn Agent>> = vec![
        Box::new(RecAgent { inner: SearchAgent::new(seed as i128, o0) }),
        Box::new(RecAgent { inner: SearchAgent::new(seed as i128 + 500000, SearchOpts::default()) }),
    ];
    let g = play_rec([a.list(), l.list()], ag, seed, f);
    report(&g, seed, f, &a, &l, None)
}

/// `record_plan(seed, a_deck, l_deck, a_plan, l_plan, a_bfs, l_bfs, first, agent_kw)` : comme `exp_plans.one` et
/// `manager/run_session.one` (mêmes tirages, mêmes plans, battlefields ou premier joueur éventuellement forcés).
/// `agent_kw` : réglages des deux agents (recherche d'`exp_search.version`).
#[allow(clippy::too_many_arguments)]
pub fn record_plan(
    seed: i64,
    a_deck: &TDeck,
    l_deck: &TDeck,
    a_plan: Option<&str>,
    l_plan: Option<&str>,
    a_bfs: Option<&[String]>,
    l_bfs: Option<&[String]>,
    first: Option<usize>,
    agent_kw: Option<[SearchOpts; 2]>,
) -> Json {
    let mut r = PyRandom::new(seed as i128 * 7919);
    let f = first.unwrap_or_else(|| r.randrange(2) as usize);
    let get = |x: Option<&str>| -> Option<Arc<dyn Plan>> { x.filter(|s| !s.is_empty()).map(|n| plans(n).unwrap_or_else(|| panic!("KeyError: {n:?}"))) };
    let (pa, pl) = (get(a_plan), get(l_plan));
    let lforced: Option<&[String]> = l_bfs.filter(|b| !b.is_empty());
    let aforced: Option<&[String]> = a_bfs.filter(|b| !b.is_empty());
    let lbf = (if pl.is_some() && lforced.is_none() { pl.as_ref().unwrap().battlefield(&l_deck.battlefields, f == 1, None) } else { None })
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| r.choice(lforced.unwrap_or(&l_deck.battlefields)).clone());
    let abf = (if pa.is_some() && aforced.is_none() {
        pa.as_ref().unwrap().battlefield(&a_deck.battlefields, f == 0, Some(&lbf))
    } else {
        None
    })
    .filter(|b| !b.is_empty())
    .unwrap_or_else(|| r.choice(aforced.unwrap_or(&a_deck.battlefields)).clone());
    let (a, l) = (a_deck.with_bf(&abf), l_deck.with_bf(&lbf));
    let kw = agent_kw.unwrap_or_else(|| [SearchOpts::default(), SearchOpts::default()]);
    let [k0, k1] = kw;
    let ag: Vec<Box<dyn Agent>> = vec![
        Box::new(RecAgent { inner: plan_agent(seed as i128, pa.clone(), pl.clone(), k0) }),
        Box::new(RecAgent { inner: plan_agent(seed as i128 + 500000, pl.clone(), pa.clone(), k1) }),
    ];
    let g = play_rec([a.list(), l.list()], ag, seed, f);
    let nm = |x: Option<&str>| x.filter(|s| !s.is_empty()).unwrap_or("aucun").to_string();
    report(&g, seed, f, &a, &l, Some([nm(a_plan), nm(l_plan)]))
}

/// `summary(rep)` : indicateurs pour choisir les parties intéressantes (écart max de score, retournements, cartes
/// jouées).
pub fn summary(rep: &Json) -> Json {
    let nms = names();
    let (mut max_def, mut lead_changes, mut last) = ([0i64; 2], 0i64, 0i64);
    let mut played: [std::collections::BTreeSet<String>; 2] = Default::default();
    for fr in rep.g("frames").arr() {
        let pts = fr.g("st").g("pts").arr();
        let (a, l) = (pts[0].int().unwrap(), pts[1].int().unwrap());
        max_def[0] = max_def[0].max(l - a);
        max_def[1] = max_def[1].max(a - l);
        let lead = (a > l) as i64 - (a < l) as i64;
        if lead != 0 && last != 0 && lead != last {
            lead_changes += 1;
        }
        if lead != 0 {
            last = lead;
        }
        for e in fr.g("ev").arr() {
            let t = e.arr()[1].str().unwrap_or("");
            for (i, n) in nms.iter().enumerate() {
                let pre = format!("{n} joue ");
                if let Some(rest) = t.strip_prefix(&pre) {
                    let x = rest.split(" (").next().unwrap().split(" — ").next().unwrap().split(" depuis").next().unwrap();
                    played[i].insert(x.to_string());
                }
            }
        }
    }
    let bf: Vec<Json> = rep.g("players").arr().iter().map(|p| p.g("bf").clone()).collect();
    jo! {
        "seed" => rep.g("seed").clone(), "first" => rep.g("first").clone(), "winner" => rep.g("winner").clone(),
        "turns" => rep.g("turns").clone(), "pts" => rep.g("pts").clone(), "bf" => Json::Arr(bf),
        "akali_max_behind" => max_def[0], "akali_max_ahead" => max_def[1], "lead_changes" => lead_changes,
        "played" => Json::Arr(played.iter().map(|s| Json::Arr(s.iter().map(Json::from).collect())).collect()),
        "frames" => rep.g("frames").arr().len(),
    }
}
