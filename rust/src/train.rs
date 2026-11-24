//! Partie humain contre IA, pilotée pas à pas (`train.py`) : la table d'entraînement (`riftbound/train/`) appelle ces
//! fonctions depuis son Web Worker (WebAssembly, `wasm.rs`) et reçoit du JSON en texte, **identique** à celui du
//! module Python (mêmes clés, même ordre, mêmes nombres, mêmes séparateurs que `json.dumps`).
//!
//! Le moteur demande certains choix en pleine résolution (g.ask, mulligan) : l'agent humain lève NeedChoice (ici : il
//! range la question dans `PENDING` et remonte `Err(GameOver)`), l'état est restauré depuis la copie prise avant
//! l'opération, la page pose la question, puis l'opération est rejouée avec la réponse (le moteur est déterministe :
//! mêmes compteurs, mêmes générateurs aléatoires).
//!
//! Écarts de représentation avec Python (sans effet sur le JSON) : les options d'un `ask` sont des poignées de la copie
//! de la partie prise au moment de la question (`Need::ag`), comme les objets de la partie abandonnée en Python ; les
//! réponses d'agent qui ne sont pas une option (`_enc` « s » et « v ») ne se produisent pas en Rust (l'agent rend un
//! indice).
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::ai::{Cfg, CfgV, SearchAgent, SearchOpts};
use crate::cards::{impl_of, registry};
use crate::decks::{decks, load};
use crate::game::{Agent, Counters, Decision, DeckList, Game, GameOver, Res, O};
use crate::jo;
use crate::json::{parse, Json};
use crate::lang::{self, tr};
use crate::match_ as m;
use crate::plans::{self, NoPlan, Plan};
use crate::pyrandom::PyRandom;
use crate::replay::{self as rp, describe, find, short, snap, tname};
use crate::spec::{parse_csv, specs};
use crate::value::{Loc, Map, Value};

// ------------------------------------------------------------------ état global (module Python : variables globales)
thread_local! {
    /// (ME, AI, DUEL)
    static MODE: Cell<(usize, usize, bool)> = const { Cell::new((0, 1, false)) };
    /// `W["ailog"]` : journal de l'IA (hors de l'état : « Reprendre » ne l'efface pas)
    static AILOG: RefCell<Vec<Json>> = const { RefCell::new(vec![]) };
    /// adresse de `W["g"]` (`s is W.get("g")`)
    static MAIN: Cell<usize> = const { Cell::new(0) };
    /// question ou erreur levée pendant une opération (NeedChoice, AIDesync, IndexError...)
    static PENDING: RefCell<Option<Interrupt>> = const { RefCell::new(None) };
    static W: RefCell<St> = RefCell::new(St::default());
}

/// `Obj._n` / `Item._n` de la table : un seul compteur pour toutes les parties de la table, comme en Python.
fn counters() -> Arc<Counters> {
    thread_local! {
        static C: Arc<Counters> = Arc::new(Counters::default());
    }
    C.with(|c| c.clone())
}

fn set_counters(on: u32, inn: u32) {
    let c = counters();
    c.obj.store(on, std::sync::atomic::Ordering::Relaxed);
    c.item.store(inn, std::sync::atomic::Ordering::Relaxed);
}

fn get_counters() -> (u32, u32) {
    let c = counters();
    (c.obj.load(std::sync::atomic::Ordering::Relaxed), c.item.load(std::sync::atomic::Ordering::Relaxed))
}

#[allow(non_snake_case)]
fn ME() -> usize {
    MODE.with(|m| m.get().0)
}
#[allow(non_snake_case)]
fn AI() -> usize {
    MODE.with(|m| m.get().1)
}
#[allow(non_snake_case)]
fn DUEL() -> bool {
    MODE.with(|m| m.get().2)
}

enum Interrupt {
    Need(Box<Need>),
    Err(String),
}

fn raise<T>(msg: String) -> Res<T> {
    PENDING.with(|p| {
        let mut p = p.borrow_mut();
        if p.is_none() {
            *p = Some(Interrupt::Err(msg));
        }
    });
    Err(GameOver)
}

/// Question posée à un humain pendant une opération (`NeedChoice`).
#[derive(Clone)]
pub struct Need {
    pub kind: String,
    pub options: Vec<Value>,
    pub ctx: Map,
    pub pid: usize,
    /// la partie au moment de la question : les options et le contexte y désignent leurs objets
    pub ag: Box<Game>,
    pub seen: Option<Seen>,
}

/// Cartes vues depuis un deck (`seen_log`).
#[derive(Clone, Debug)]
pub struct Seen {
    pub ev: String,
    pub pid: usize,
    pub owner: usize,
    pub it: Option<u32>,
    pub src: Option<String>,
    pub uids: Vec<u32>,
    pub names: Vec<String>,
}

/// Ajouts de `TGame` à la partie : journal de la table, dernier vrai choix de l'IA, cartes vues, combats gagnés.
#[derive(Clone, Debug, Default)]
pub struct TableExt {
    pub buf: Vec<(i64, String)>,
    pub last_ai_step: Option<usize>,
    pub last_ai_label: Option<String>,
    pub ask_seen: Option<Seen>,
    pub seen_log: Vec<Seen>,
    pub cw: Vec<i64>,
}

// ------------------------------------------------------------------ crochets de TGame (appelés par Game)
/// `TGame.log`
pub fn tg_log(g: &mut Game, raw: &str) {
    let line = rp::log_line(g, raw);
    let ind = if raw.starts_with("  ") { 1 } else { 0 };
    g.tg.as_mut().unwrap().buf.push((ind, line));
}

/// `TGame.draw` : les pioches de l'humain sont notées, celles de l'IA restent cachées.
pub fn tg_draw(g: &mut Game, pid: usize, c: O) {
    let t = if pid == ME() {
        let n = g.cname(c).to_string();
        tr(format!("Tu pioches {n}"), format!("You draw {n}"))
    } else {
        let a = &rp::names()[AI()];
        tr(format!("{a} pioche une carte"), format!("{a} draws a card"))
    };
    g.tg.as_mut().unwrap().buf.push((1, t));
}

/// `TGame.emit` : combats gagnés depuis le dernier rendu, cartes vues depuis un deck (« look at the top N » n'est vu
/// que par le joueur qui regarde, une carte révélée est vue de tous, 424).
pub fn tg_emit(g: &mut Game, ev: &str, info: &Map) {
    if ev == "combat_won" {
        let p = info.g("pid").int();
        g.tg.as_mut().unwrap().cw.push(p);
    }
    if (ev == "look" || ev == "reveal") && info.g("cards").truthy() && (ev == "reveal" || info.g("pid").int() as usize == ME())
    {
        let pid = info.g("pid").int() as usize;
        let owner = match info.get("owner") {
            Some(v) => v.int() as usize,
            None => pid,
        };
        let it = g.resolving;
        let src = src_name(it.map(|i| g.it(i).name.clone()).unwrap_or_default().as_str());
        let cards: Vec<O> = info.g("cards").list().iter().map(|c| c.obj()).collect();
        let se = Seen {
            ev: ev.to_string(),
            pid,
            owner,
            it: it.map(|i| g.it(i).id),
            src,
            uids: cards.iter().map(|&c| g.o(c).uid).collect(),
            names: cards.iter().map(|&c| g.cname(c).to_string()).collect(),
        };
        g.tg.as_mut().unwrap().seen_log.push(se);
    }
}

/// `TGame.ask` (avant) : cartes vues par l'effet qui pose la question (même élément de chaîne), montrées avec elle.
pub fn tg_ask_pre(g: &mut Game, pid: usize, options: &[Value]) {
    if pid == ME() && !options.is_empty() {
        let it = g.resolving.map(|i| g.it(i).id);
        let tg = g.tg.as_mut().unwrap();
        tg.ask_seen = tg.seen_log.iter().rev().find(|e| e.it == it).cloned();
    }
}

/// `TGame.ask` (après) : chaque réponse de l'IA à une question de la partie réelle va dans son journal.
pub fn tg_ask_post(g: &mut Game, pid: usize, kind: &str, options: &[Value], r: &Value) {
    let main = MAIN.with(|m| m.get()) == g as *const Game as usize;
    if pid == AI()
        && !DUEL()
        && main
        && !options.is_empty()
        && !(options.len() == 1 && !matches!(kind, "may" | "may_target" | "order"))
    {
        let e = enc(r, options);
        AILOG.with(|l| l.borrow_mut().push(Json::Arr(vec!["a".into(), e, kind.into(), options.len().into()])));
    }
}

/// Même objet au sens de `is` en Python, au mieux : les poignées, booléens, None et petits entiers se comparent par
/// valeur ; le reste par égalité (la réponse est toujours une des options).
fn enc(ans: &Value, options: &[Value]) -> Json {
    for (i, o) in options.iter().enumerate() {
        if o == ans && std::mem::discriminant(o) == std::mem::discriminant(ans) {
            return Json::Arr(vec!["i".into(), i.into()]);
        }
    }
    match ans {
        Value::None => Json::Arr(vec!["v".into(), Json::Null]),
        Value::Bool(b) => Json::Arr(vec!["v".into(), (*b).into()]),
        Value::Int(n) => Json::Arr(vec!["v".into(), (*n).into()]),
        Value::Float(f) => Json::Arr(vec!["v".into(), (*f).into()]),
        Value::Str(s) => Json::Arr(vec!["v".into(), (&**s).into()]),
        _ => Json::Null,
    }
}

// ------------------------------------------------------------------ agents
/// `Human` : ne décide rien lui-même, lit les réponses déjà données ou lève NeedChoice. Le moteur lui propose tous
/// les choix légaux (`every_choice`).
#[derive(Clone, Default)]
pub struct Human {
    pub answers: Vec<Json>,
    pub k: usize,
}

impl Human {
    fn next(&mut self, g: &Game, kind: &str, options: &[Value], ctx: &Map, pid: usize) -> Res<Json> {
        if self.k < self.answers.len() {
            self.k += 1;
            return Ok(self.answers[self.k - 1].clone());
        }
        let need = Need {
            kind: kind.to_string(),
            options: options.to_vec(),
            ctx: ctx.clone(),
            pid,
            ag: Box::new(g.clone()),
            seen: None,
        };
        PENDING.with(|p| *p.borrow_mut() = Some(Interrupt::Need(Box::new(need))));
        Err(GameOver)
    }
}

/// `options[i]` de Python (indices négatifs compris) : l'indice réel, ou IndexError.
fn py_index(i: &Json, n: usize) -> Result<usize, String> {
    let i = match i {
        Json::Int(i) => *i,
        Json::Bool(b) => *b as i64,
        Json::Float(_) => return Err("TypeError: list indices must be integers or slices, not float".into()),
        Json::Str(_) => return Err("TypeError: list indices must be integers or slices, not str".into()),
        Json::Null => return Err("TypeError: list indices must be integers or slices, not NoneType".into()),
        Json::Arr(_) => return Err("TypeError: list indices must be integers or slices, not list".into()),
        Json::Obj(_) => return Err("TypeError: list indices must be integers or slices, not dict".into()),
    };
    let j = if i < 0 { i + n as i64 } else { i };
    if j < 0 || j >= n as i64 {
        return Err("IndexError: list index out of range".into());
    }
    Ok(j as usize)
}

impl Agent for Human {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        let x = self.next(g, kind, options, ctx, pid)?;
        match py_index(&x, options.len()) {
            Ok(i) => Ok(i),
            Err(e) => raise(e),
        }
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        let hand = g.p[pid].hand.clone();
        let opts: Vec<Value> = hand.iter().map(|&c| Value::Obj(c)).collect();
        let out = self.next(g, "mulligan", &opts, &Map::new(), pid)?;
        let mut r = vec![];
        for i in out.arr() {
            match py_index(i, hand.len()) {
                Ok(j) => r.push(hand[j]),
                Err(e) => return raise(e),
            }
        }
        Ok(r)
    }
    fn every_choice(&self) -> bool {
        true
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
    fn as_any(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

fn human_of(a: &mut Box<dyn Agent>) -> Option<&mut Human> {
    a.as_any().and_then(|x| x.downcast_mut::<Human>())
}

/// `Gentle` : IA débutante du tutoriel : une décision sur WILD est prise au hasard parmi les coups légaux (comme
/// agents.RandomAgent), les autres par l'IA normale. Son générateur est copié avec l'état (Reprendre, rejeu).
#[derive(Clone)]
pub struct Gentle {
    pub inner: SearchAgent,
    pub wild: PyRandom,
}

const WILD: f64 = 0.5;

impl Agent for Gentle {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        self.inner.choose(g, pid, kind, options, ctx)
    }
    fn mulligan(&mut self, g: &mut Game, pid: usize) -> Res<Vec<O>> {
        self.inner.mulligan(g, pid)
    }
    fn start(&mut self, g: &mut Game) {
        self.inner.start(g)
    }
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        if self.wild.random() < WILD {
            let mut opts: Vec<Value> = d.options.clone().unwrap();
            if d.kind == "main" && opts.len() > 1 && self.wild.random() < 0.85 {
                opts.retain(|o| o.list()[0].str() != "end");
            }
            if d.kind != "main" && self.wild.random() < 0.6 {
                return Ok(Value::Tuple(vec!["pass".into()]));
            }
            return Ok(self.wild.choice(&opts).clone());
        }
        self.inner.decide(g, d)
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

/// `ScriptAI` : l'IA rejoue les coups enregistrés d'une partie (journal `ailog`) ; au-delà du script, l'IA elle-même.
#[derive(Clone)]
pub struct ScriptAI {
    pub ag: Box<dyn Agent>,
    pub script: Vec<Json>,
}

impl ScriptAI {
    fn e(&self, typ: &str, kind: &str, n: usize) -> Res<Option<Json>> {
        let i = AILOG.with(|l| l.borrow().len());
        if i >= self.script.len() {
            return Ok(None);
        }
        let e = &self.script[i];
        let ea = e.arr();
        let ok = ea.len() >= 4 && ea[0].str() == Some(typ) && ea[2].str() == Some(kind) && ea[3].int() == Some(n as i64);
        if !ok {
            let mut want: Vec<Json> = ea.iter().take(1).cloned().collect();
            want.extend(ea.iter().skip(2).cloned());
            return raise(format!(
                "AIDesync: coup {i} de l'IA : attendu {}, vu {}",
                py_repr_json(&Json::Arr(want)),
                py_repr_json(&Json::Arr(vec![typ.into(), kind.into(), n.into()]))
            ));
        }
        Ok(Some(e.clone()))
    }
}

impl Agent for ScriptAI {
    fn decide(&mut self, g: &mut Game, d: &Decision) -> Res<Value> {
        let opts = d.options.as_ref().unwrap();
        match self.e("d", d.kind, opts.len())? {
            None => self.ag.decide(g, d),
            Some(e) => match py_index(&e.arr()[1], opts.len()) {
                Ok(i) => Ok(opts[i].clone()),
                Err(x) => raise(x),
            },
        }
    }
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize> {
        let e = self.e("a", kind, options.len())?;
        let r = e.and_then(|e| dec(&e.arr()[1], options));
        match r {
            Some(i) => Ok(i),
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

/// `_dec(e, options)` : indice de l'option, None = `_BAD` (l'IA décide elle-même).
fn dec(e: &Json, options: &[Value]) -> Option<usize> {
    let a = e.arr();
    match a.first().and_then(|x| x.str()) {
        Some("i") => {
            let i = a[1].int()?;
            if i >= 0 && (i as usize) < options.len() {
                Some(i as usize)
            } else if i < 0 && ((-i) as usize) <= options.len() {
                Some((options.len() as i64 + i) as usize)
            } else {
                None
            }
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
        Json::Obj(v) => {
            let mut m = Map::new();
            for (k, x) in v {
                m.set(k, json_value(x));
            }
            Value::Map(m)
        }
    }
}

/// `repr()` Python d'une valeur JSON (messages d'erreur, `str(x)`).
pub fn py_repr_json(j: &Json) -> String {
    match j {
        Json::Null => "None".into(),
        Json::Bool(b) => if *b { "True" } else { "False" }.into(),
        Json::Int(n) => n.to_string(),
        Json::Float(f) => crate::json::py_float(*f).replace("Infinity", "inf").replace("NaN", "nan"),
        Json::Str(s) => py_quote(s),
        Json::Arr(v) => format!("[{}]", v.iter().map(py_repr_json).collect::<Vec<_>>().join(", ")),
        Json::Obj(v) => {
            format!("{{{}}}", v.iter().map(|(k, x)| format!("{}: {}", py_quote(k), py_repr_json(x))).collect::<Vec<_>>().join(", "))
        }
    }
}

/// `str(x)` Python d'une valeur JSON.
pub fn py_str_json(j: &Json) -> String {
    match j {
        Json::Str(s) => s.clone(),
        j => py_repr_json(j),
    }
}

fn py_quote(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut out = String::new();
    out.push(q);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == q => {
                out.push('\\');
                out.push(c)
            }
            c => out.push(c),
        }
    }
    out.push(q);
    out
}

// ------------------------------------------------------------------ paquets
/// Paquet de la table (dict Python : legend, champion, main, runes, battlefields, battlefield, sideboard, key).
#[derive(Clone, Debug)]
pub struct TDeck {
    pub legend: String,
    pub champion: Option<String>,
    pub main: Vec<String>,
    pub runes: Vec<String>,
    pub battlefields: Vec<String>,
    pub battlefield: String,
    pub sideboard: Vec<String>,
    pub key: String,
}

impl TDeck {
    fn list(&self) -> DeckList {
        DeckList {
            legend: self.legend.clone(),
            champion: self.champion.clone(),
            main: self.main.clone(),
            runes: self.runes.clone(),
            battlefield: self.battlefield.clone(),
        }
    }
    fn with_bf(&self, bf: &str) -> TDeck {
        let mut d = self.clone();
        d.battlefield = bf.to_string();
        d
    }
    /// le dict Python, pour `validate`
    fn json(&self) -> Json {
        jo! {
            "legend" => self.legend.clone(), "champion" => self.champion.clone(), "main" => self.main.clone(),
            "runes" => self.runes.clone(), "battlefields" => self.battlefields.clone(), "sideboard" => self.sideboard.clone(),
            "battlefield" => self.battlefield.clone(), "key" => self.key.clone(),
        }
    }
}

fn tload(key: &str) -> TDeck {
    let l = load(key, None);
    TDeck {
        legend: l.deck.legend,
        champion: l.deck.champion,
        main: l.deck.main,
        runes: l.deck.runes,
        battlefields: l.battlefields,
        battlefield: l.deck.battlefield,
        sideboard: l.sideboard,
        key: l.key,
    }
}

/// `decks.edit(deck, out, add)`
fn edit(d: &TDeck, out: &[(&str, usize)], add: &[(&str, usize)]) -> TDeck {
    let mut d = d.clone();
    for (n, q) in out {
        for _ in 0..*q {
            let i = d.main.iter().position(|x| x == n).unwrap();
            d.main.remove(i);
        }
    }
    for (n, q) in add {
        for _ in 0..*q {
            d.main.push(n.to_string());
        }
    }
    d
}

/// `G2` : Akali G2 (Gorica)
fn g2() -> TDeck {
    let g = tload("akali_gorica_rq-singapore");
    let gl = edit(&g, &[("Long Sword", 2)], &[("Akali, Silent", 1), ("Ferrous Forerunner", 1)]);
    edit(&gl, &[("Defy", 3)], &[("Ferrous Forerunner", 1), ("Lonely Poro", 1), ("Scuttle Crab", 1)])
}

/// `L` : LeBlanc IQ#5 (Gyatarina)
fn leblanc() -> TDeck {
    tload("leblanc_gyatarina_ccs-iq5_1st")
}

/// `PRESETS`
fn presets() -> Vec<(&'static str, &'static str, TDeck)> {
    vec![("akali-g2", "Akali G2 (Gorica)", g2()), ("leblanc-iq5", "LeBlanc IQ#5 (Gyatarina)", leblanc())]
}

/// `_plan_for(deck)` : plan de jeu connu pour la légende du deck, sinon l'IA générique (aucun plan).
fn plan_for(legend: &str) -> Arc<dyn Plan> {
    if legend.starts_with("Akali") {
        return plans::plans("Gorica").unwrap();
    }
    if legend.starts_with("LeBlanc") {
        return plans::plans("Hook tempo").unwrap();
    }
    Arc::new(NoPlan)
}

fn short_name(d: &TDeck) -> String {
    d.legend.split(',').next().unwrap_or("?").to_string()
}

fn strs(j: &Json) -> Vec<String> {
    j.arr().iter().map(|x| x.str().unwrap_or("").to_string()).collect()
}

/// `_deck(x, default)` : deck donné par la page : clé de PRESETS, clé de decks.json, ou dict {legend, champion, main,
/// runes, battlefields}.
fn deck_arg(x: &Json, default: fn() -> TDeck) -> Result<TDeck, String> {
    match x {
        Json::Null => return Ok(default()),
        Json::Str(s) if s.is_empty() => return Ok(default()),
        Json::Str(s) if !s.trim_start().starts_with('{') => {
            if let Some(p) = presets().into_iter().find(|p| p.0 == s) {
                return Ok(p.2);
            }
            if !decks().contains_key(s.as_str()) {
                return Err(format!("KeyError: {}", py_quote(s)));
            }
            return Ok(tload(s));
        }
        _ => {}
    }
    let d = match x {
        Json::Str(s) => parse(s).map_err(|e| format!("JSONDecodeError: {e}"))?,
        j => j.clone(),
    };
    let err: Vec<String> = validate(&d).into_iter().filter(|e| !e.starts_with('⚠')).collect();
    if !err.is_empty() {
        return Err(format!("ValueError: {}{}", tr("Deck invalide : ", "Invalid deck: "), err.join(" ; ")));
    }
    let bfs = strs(d.g("battlefields"));
    let name = d.g("name");
    Ok(TDeck {
        legend: d.g("legend").str().unwrap_or("").to_string(),
        champion: d.g("champion").str().map(|s| s.to_string()),
        main: strs(d.g("main")),
        runes: strs(d.g("runes")).into_iter().map(|r| r.replace(" Rune", "")).collect(),
        battlefield: bfs[0].clone(),
        battlefields: bfs,
        sideboard: strs(d.g("sideboard")),
        key: if name.truthy() { name.str().unwrap_or("perso").to_string() } else { "perso".into() },
    })
}

// ------------------------------------------------------------------ réglages de l'IA de la table
/// `TABLE_CFG` puis `LEVEL_CFG` (« Très fort » : faisceau de 6 suites, react_n tirages pour les petites décisions).
fn table_cfg(level: i64) -> Cfg {
    let mut c = Cfg::new();
    c.set("ambush", CfgV::Num(0.5));
    c.set("tie_react", CfgV::Num(1.0));
    if level == 6 {
        c.set("turn_w", CfgV::Num(6.0));
        c.set("react_n", CfgV::Num(3.0));
    }
    c
}

// ------------------------------------------------------------------ état de la table
#[derive(Clone)]
struct Save {
    g: Box<Game>,
    d: Option<Decision>,
    on: u32,
    inn: u32,
}

#[derive(Clone)]
enum Op {
    Tick,
    Act(Value),
    /// robots de vérification : `W[key].append(W["g"].ask(ME, "may", [True, False]))`
    AskMay(String),
}

struct Rp {
    frames: Vec<String>,
    svs: BTreeMap<usize, Option<Save>>,
    prev: Option<Save>,
}

#[derive(Default)]
struct St {
    g: Option<Box<Game>>,
    coach: Option<SearchAgent>,
    d: Option<Decision>,
    retry: Option<(Op, Box<Need>)>,
    seed: Option<i64>,
    decks: Vec<TDeck>,
    last_ai: Option<String>,
    last_ai_info: Option<Json>,
    seen_shown: BTreeSet<Vec<u32>>,
    hist: Vec<Save>,
    ask_shown: Option<Json>,
    rp: Option<Rp>,
    /// niveau de l IA de la partie (samples ; réglages : table_cfg)
    level: Option<i64>,
    /// valeurs posées par les robots de vérification (`W["T_YES"]`...)
    extra: BTreeMap<String, Json>,
}

impl St {
    fn g(&mut self) -> &mut Game {
        self.g.as_mut().expect("aucune partie en cours")
    }
    fn set_main(&self) {
        let p = self.g.as_ref().map(|g| &**g as *const Game as usize).unwrap_or(0);
        MAIN.with(|m| m.set(p));
    }
}

fn clone_agents(g: &Game) -> Vec<Box<dyn Agent>> {
    g.agents.0.iter().map(|a| a.clone_box()).collect()
}

/// copie profonde de la partie avec ses agents (`copy.deepcopy(dict(g=W["g"], ag=W["ag"], ...))`)
fn deep(g: &Game) -> Box<Game> {
    let mut c = Box::new(g.clone());
    c.agents.0 = clone_agents(g);
    c
}

/// `_save()`
fn save(w: &mut St) -> Save {
    let (on, inn) = get_counters();
    Save { g: deep(w.g()), d: w.d.clone(), on, inn }
}

/// `_restore(sv)`
fn restore(w: &mut St, sv: &Save) {
    set_counters(sv.on, sv.inn);
    w.g = Some(deep(&sv.g));
    w.d = sv.d.clone();
    w.set_main();
}

/// `TGame(decks, agents, seed, first)`
fn tgame(decks: [&TDeck; 2], agents: Vec<Box<dyn Agent>>, seed: i64, first: usize) -> Box<Game> {
    let mut g = Box::new(Game::new(
        &[decks[0].list(), decks[1].list()],
        agents,
        Some(first),
        seed as i128,
        true,
        crate::spec::VICTORY as i64,
        counters(),
    ));
    g.tg = Some(Box::default());
    g
}

fn system_seed() -> i64 {
    // `random.randrange(10 ** 6)` du générateur du système
    thread_local! {
        static R: RefCell<PyRandom> = RefCell::new(PyRandom::new(
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as i128).unwrap_or(1)
        ));
    }
    R.with(|r| r.borrow_mut().randrange(1_000_000))
}

fn jint(j: &Json) -> Result<i64, String> {
    match j {
        Json::Int(n) => Ok(*n),
        Json::Float(f) => Ok(f.trunc() as i64),
        Json::Bool(b) => Ok(*b as i64),
        Json::Str(s) => s.trim().parse::<i64>().map_err(|_| format!("ValueError: invalid literal for int() with base 10: {}", py_quote(s))),
        _ => Err("TypeError: int() argument must be a string, a bytes-like object or a real number, not 'NoneType'".into()),
    }
}

fn arg(a: &[Json], i: usize) -> Json {
    a.get(i).cloned().unwrap_or(Json::Null)
}

/// `new(seed, bf, first, level, mine, opp, obf)`
fn new(seed: &Json, bf: &Json, first: &Json, level: &Json, mine: &Json, opp: &Json, obf: &Json) -> Result<String, String> {
    duel_off();
    let seed = if seed.is_null() { system_seed() } else { jint(seed)? };
    set_counters(0, 0);
    let mut r = PyRandom::new(seed as i128 * 7919);
    let f = if first.is_null() { r.randrange(2) as usize } else { jint(first)? as usize };
    let level = if level.is_null() { 1 } else { jint(level)? };
    let md = deck_arg(mine, g2)?;
    let od = deck_arg(opp, leblanc)?;
    let (pa, pl) = (plan_for(&md.legend), plan_for(&od.legend));
    let lbf = match obf.str() {
        Some(b) if od.battlefields.iter().any(|x| x == b) => b.to_string(),
        _ => match pl.battlefield(&od.battlefields, f == 1, None).filter(|b| !b.is_empty()) {
            Some(b) => b,
            None => r.choice(&od.battlefields).clone(),
        },
    };
    let abf = match bf.str() {
        Some(b) if md.battlefields.iter().any(|x| x == b) => b.to_string(),
        _ => match pa.battlefield(&md.battlefields, f == 0, Some(&lbf)).filter(|b| !b.is_empty()) {
            Some(b) => b,
            None => r.choice(&md.battlefields).clone(),
        },
    };
    let (a, b) = (md.with_bf(&abf), od.with_bf(&lbf));
    let mut names = [short_name(&a), short_name(&b)];
    if names[0] == names[1] {
        names[1] = format!("{}{}", names[1], tr(" (IA)", " (AI)"));
    }
    rp::set_names(names.clone());
    let hu: Box<dyn Agent> = Box::new(Human::default());
    // level 0 : IA débutante du tutoriel (Gentle) ; sinon level = mondes tirés par décision (1 Normal, 3 Fort, 6 Très
    // fort), réglages en plus dans LEVEL_CFG. L'IA de la table cherche le tour entier (search="turn").
    let ai: Box<dyn Agent> = if level == 0 {
        let inner = plans::plan_agent(
            (seed + 500000) as i128,
            Some(pl.clone()),
            Some(pa.clone()),
            SearchOpts { samples: 1, search: Some("sh".into()), ..SearchOpts::default() },
        );
        Box::new(Gentle { inner, wild: PyRandom::new((seed + 500000) as i128 * 31 + 7) })
    } else {
        Box::new(plans::plan_agent(
            (seed + 500000) as i128,
            Some(pl.clone()),
            Some(pa.clone()),
            SearchOpts { samples: level as usize, search: Some("turn".into()), cfg: table_cfg(level), ..SearchOpts::default() },
        ))
    };
    let coach = plans::plan_agent(
        (seed + 900000) as i128,
        Some(plan_for(&md.legend)),
        Some(plan_for(&od.legend)),
        SearchOpts::default(),
    );
    let g = tgame([&a, &b], vec![hu, ai], seed, f);
    W.with(|w| {
        let mut w = w.borrow_mut();
        *w = St::default();
        AILOG.with(|l| l.borrow_mut().clear());
        w.g = Some(g);
        w.coach = Some(coach);
        w.seed = Some(seed);
        w.level = Some(level);
        w.decks = vec![a.clone(), b.clone()];
        w.set_main();
    });
    let decks_j: Vec<Json> = [&a, &b].iter().map(|x| deck_meta(x)).collect();
    Ok(jo! {
        "seed" => seed, "first" => f, "bf" => vec![abf, lbf], "battlefields" => md.battlefields.clone(),
        "names" => names.to_vec(), "decks" => Json::Arr(decks_j),
    }
    .dumps(true))
}

fn deck_meta(x: &TDeck) -> Json {
    let main: BTreeSet<&String> = x.main.iter().collect();
    jo! {
        "legend" => x.legend.clone(), "champion" => x.champion.clone(),
        "main" => Json::Arr(main.into_iter().map(Json::from).collect()),
    }
}

// ------------------------------------------------------------------ tutoriel
const TUTO_DECKS: (&str, &str) = ("yi-bladesman_swagalisk_rq-la_6th", "darius_mice-diamondhat_rq-utrecht_6th");
const TUTO_SEED: i64 = 202;

/// `tuto()` : partie du tutoriel : donne fixe, tu commences, IA débutante (level 0).
fn tuto() -> Result<String, String> {
    new(
        &TUTO_SEED.into(),
        &Json::Null,
        &0i64.into(),
        &0i64.into(),
        &TUTO_DECKS.0.into(),
        &TUTO_DECKS.1.into(),
        &Json::Null,
    )
}

/// `tuto_seeds(n, start)` : donnes où la main de départ du joueur (qui commence) a au moins deux unités à 2 énergie
/// sans puissance.
fn tuto_seeds(n: usize, start: i64) -> Result<String, String> {
    let mut found = vec![];
    for seed in start..start + 2000 {
        new(&seed.into(), &Json::Null, &0i64.into(), &0i64.into(), &TUTO_DECKS.0.into(), &TUTO_DECKS.1.into(), &Json::Null)?;
        let mut v = parse(&step()?).unwrap();
        while v.g("busy").truthy() {
            v = parse(&step()?).unwrap();
        }
        if !v.g("ask").truthy() || v.g("ask").g("kind").str() != Some("mulligan") {
            continue;
        }
        let hand = strs(v.g("ask").g("options"));
        let cheap = hand
            .iter()
            .filter(|n| {
                let s = &specs()[n.as_str()];
                s.type_ == "Unit" && s.e <= 2 && s.p == 0 && s.keywords.is_empty()
            })
            .count();
        if cheap >= 2 {
            found.push(Json::Arr(vec![seed.into(), hand.into()]));
            if found.len() >= n {
                break;
            }
        }
    }
    Ok(Json::Arr(found).dumps(true))
}

// ------------------------------------------------------------------ duel entre deux joueurs
/// Identifiant du moteur (`version.engine_md5()` en Python) : posé à la construction (RB_ENGINE).
pub fn engine_id() -> &'static str {
    option_env!("RB_ENGINE").unwrap_or("rust")
}

/// `duel_new(seed, bf0, bf1, first, deck0, deck1, me, names)` : partie à deux humains.
#[allow(clippy::too_many_arguments)]
fn duel_new(seed: &Json, bf0: &Json, bf1: &Json, first: &Json, deck0: &Json, deck1: &Json, me: &Json, names: &Json) -> Result<String, String> {
    let me = jint(me)? as usize;
    MODE.with(|m| m.set((me, 1 - me, true)));
    let seed = jint(seed)?;
    set_counters(0, 0);
    let d = [deck_arg(deck0, g2)?, deck_arg(deck1, leblanc)?];
    let pick = |d: &TDeck, b: &Json| match b.str() {
        Some(x) if d.battlefields.iter().any(|y| y == x) => x.to_string(),
        _ => d.battlefields[0].clone(),
    };
    let a = d[0].with_bf(&pick(&d[0], bf0));
    let b = d[1].with_bf(&pick(&d[1], bf1));
    let mut nm: Vec<String> = if names.truthy() { strs(names) } else { vec![short_name(&a), short_name(&b)] };
    if nm[0] == nm[1] {
        nm[1] = format!("{} (2)", nm[1]);
    }
    rp::set_names([nm[0].clone(), nm[1].clone()]);
    let ab = [&a, &b];
    let coach = plans::plan_agent(
        (seed + 900000) as i128,
        Some(plan_for(&ab[me].legend)),
        Some(plan_for(&ab[1 - me].legend)),
        SearchOpts::default(),
    );
    let first = jint(first)? as usize;
    let g = tgame([&a, &b], vec![Box::new(Human::default()), Box::new(Human::default())], seed, first);
    W.with(|w| {
        let mut w = w.borrow_mut();
        *w = St::default();
        AILOG.with(|l| l.borrow_mut().clear());
        w.g = Some(g);
        w.coach = Some(coach);
        w.seed = Some(seed);
        w.decks = vec![a.clone(), b.clone()];
        w.set_main();
    });
    Ok(jo! {
        "seed" => seed, "first" => first, "bf" => vec![a.battlefield.clone(), b.battlefield.clone()], "names" => nm,
        "me" => me, "engine" => engine_id(), "decks" => Json::Arr(vec![deck_meta(&a), deck_meta(&b)]),
    }
    .dumps(true))
}

/// `duel_off()` : revenir aux parties contre l'IA (la place locale redevient 0).
fn duel_off() -> String {
    MODE.with(|m| m.set((0, 1, false)));
    "{}".into()
}

// ------------------------------------------------------------------ match BO1 / BO3
fn match_new(mode: &Json, seed: &Json, mine: &Json, opp: &Json) -> Result<String, String> {
    let seed = match seed {
        Json::Null => system_seed(),
        Json::Str(s) if s.is_empty() => system_seed(),
        s => jint(s)?,
    };
    let md = deck_arg(mine, g2)?;
    let od = deck_arg(opp, leblanc)?;
    let mode = mode.str().unwrap_or("");
    if mode != "bo1" && mode != "bo3" {
        return Err("AssertionError".into());
    }
    Ok(m::new(mode, seed, [md.battlefields, od.battlefields]).dumps(true))
}

fn load_state(s: &Json) -> Result<Json, String> {
    match s {
        Json::Str(t) => parse(t).map_err(|e| format!("JSONDecodeError: {e}")),
        j => Ok(j.clone()),
    }
}

fn match_next(state: &Json, mine: &Json, opp: &Json, human_bf: &Json) -> Result<String, String> {
    let st = load_state(state)?;
    let _md = deck_arg(mine, g2)?;
    let od = deck_arg(opp, leblanc)?;
    let ch = m::chooser(&st);
    let mut first = m::forced_first(&st);
    if first.is_none() && ch == Some(m::AI as i64) {
        first = Some(m::ai_first(&st));
    }
    let pl = plan_for(&od.legend);
    let f1 = first == Some(1);
    let choose = |ok: &[String]| -> Option<String> { pl.battlefield(ok, f1, None).filter(|b| ok.contains(b)) };
    let bfs = m::pick_bfs(&st, Some(&choose), human_bf.str());
    let gn = m::game_no(&st);
    Ok(jo! {
        "game" => gn, "chooser" => ch, "first" => first, "roll" => if gn == 1 { st.g("roll").clone() } else { Json::Null },
        "allowed" => m::allowed_bfs(&st, 0), "ai_bf" => bfs[1].clone(),
        "human_bf" => if st.g("mode").str() == Some("bo1") { Json::from(bfs[0].clone()) } else { Json::Null },
        "sideboard" => m::can_sideboard(&st), "seed" => m::seed_of(&st), "wins" => st.g("wins").clone(), "over" => m::over(&st),
    }
    .dumps(true))
}

fn match_duel_next(state: &Json) -> Result<String, String> {
    let st = load_state(state)?;
    let bo1 = if st.g("mode").str() == Some("bo1") { Json::from(m::pick_bfs(&st, None, None).to_vec()) } else { Json::Null };
    let gn = m::game_no(&st);
    Ok(jo! {
        "game" => gn, "chooser" => m::chooser(&st), "first" => m::forced_first(&st),
        "roll" => if gn == 1 { st.g("roll").clone() } else { Json::Null },
        "allowed" => Json::Arr(vec![m::allowed_bfs(&st, 0).into(), m::allowed_bfs(&st, 1).into()]), "bo1_bfs" => bo1,
        "sideboard" => m::can_sideboard(&st), "seed" => m::seed_of(&st), "wins" => st.g("wins").clone(), "over" => m::over(&st),
    }
    .dumps(true))
}

fn match_record(state: &Json, first: &Json, bfs: &Json, win: &Json) -> Result<String, String> {
    let st = load_state(state)?;
    let b = load_state(bfs)?;
    let mut st = m::record(&st, jint(first)?, b.arr(), jint(win)?);
    let (o, w) = (m::over(&st), m::winner(&st));
    st.set("over", o);
    st.set("winner", w);
    Ok(st.dumps(true))
}

fn match_swap(deck: &Json, out_cards: &Json, in_cards: &Json, champion: &Json) -> Result<String, String> {
    let d = load_state(deck)?;
    let o = strs(&load_state(out_cards)?);
    let i = strs(&load_state(in_cards)?);
    let ch = champion.str().filter(|c| !c.is_empty());
    let d = m::sideboard_swap(&d, &o, &i, ch)?;
    let err: Vec<String> = validate(&d).into_iter().filter(|e| !e.starts_with('⚠')).collect();
    if !err.is_empty() {
        return Err(format!(
            "ValueError: {}{}",
            tr("Deck invalide après sideboard : ", "Invalid deck after sideboarding: "),
            err.join(" ; ")
        ));
    }
    Ok(d.dumps(true))
}

// ------------------------------------------------------------------ éditeur de deck
/// Lignes de `cards_unique.csv` (dict par ligne, en-têtes du fichier).
fn spec_rows() -> &'static Vec<BTreeMap<String, String>> {
    static R: std::sync::OnceLock<Vec<BTreeMap<String, String>>> = std::sync::OnceLock::new();
    R.get_or_init(|| {
        let rows = parse_csv(include_str!("../../riftbound/cards/cards_unique.csv"));
        let head = rows[0].clone();
        rows[1..]
            .iter()
            .filter(|r| r.len() >= head.len())
            .map(|r| head.iter().cloned().zip(r.iter().cloned()).collect())
            .collect()
    })
}

fn col<'a>(r: &'a BTreeMap<String, String>, k: &str) -> &'a str {
    r.get(k).map(|s| s.as_str()).unwrap_or("")
}

fn bar(s: &str) -> Vec<String> {
    s.split('|').filter(|x| !x.is_empty()).map(|x| x.to_string()).collect()
}

/// `_BAN` : cartes bannies en Standard
fn ban() -> &'static BTreeMap<String, String> {
    static B: std::sync::OnceLock<BTreeMap<String, String>> = std::sync::OnceLock::new();
    B.get_or_init(|| {
        let mut m = BTreeMap::new();
        for x in spec_rows() {
            let b = col(x, "banned_standard");
            if !b.is_empty() {
                m.insert(col(x, "name").to_string(), b.to_string());
            }
        }
        m
    })
}

fn modeled(n: &str) -> bool {
    impl_of(n).is_some()
}

/// `catalog()` : toutes les cartes du jeu pour l'éditeur : m=1 si la carte est modélisée (jouable).
fn catalog() -> String {
    let mut out = vec![];
    for x in spec_rows() {
        if col(x, "supertype") == "Token" {
            continue;
        }
        let n = col(x, "name");
        out.push(jo! {
            "id" => col(x, "id"), "n" => n, "t" => col(x, "type"), "s" => col(x, "supertype"), "d" => bar(col(x, "domain")),
            "e" => col(x, "energy_cost"), "p" => col(x, "power_cost"), "mi" => col(x, "might"), "tg" => bar(col(x, "tags")),
            "k" => col(x, "keywords"), "tx" => col(x, "text"), "set" => col(x, "set_code"), "r" => col(x, "rarity"),
            "ban" => col(x, "banned_standard"),
            "m" => if modeled(n) || col(x, "type") == "Rune" { 1 } else { 0 },
        });
    }
    let mut pre: Vec<Json> =
        presets().into_iter().map(|(k, name, d)| jo! { "key" => k, "name" => name, "deck" => export(&d) }).collect();
    for (k, m) in decks() {
        let d = tload(k);
        let or_empty = |key: &str| -> Json {
            let v = m.g(key);
            if v.truthy() {
                v.clone()
            } else {
                "".into()
            }
        };
        pre.push(jo! {
            "key" => k.clone(), "name" => k.replace('_', " · "), "deck" => export(&d), "player" => or_empty("player"),
            "event" => or_empty("event"), "place" => or_empty("placement"), "date" => or_empty("date"), "ban" => banned(&d),
        });
    }
    let dflt = default_decks();
    let legends: Vec<Json> = spec_rows()
        .iter()
        .filter(|x| col(x, "type") == "Legend")
        .map(|x| {
            let n = col(x, "name");
            jo! { "n" => n, "d" => bar(col(x, "domain")), "key" => dflt.get(n).cloned() }
        })
        .collect();
    jo! { "cards" => Json::Arr(out), "presets" => Json::Arr(pre), "legends" => Json::Arr(legends) }.dumps(false)
}

/// `_place(k)` : classement du deck (999 sans classement)
fn place(k: &str) -> i64 {
    let p = decks()[k].g("placement");
    let s = if p.truthy() { py_str_json(p) } else { String::new() };
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or(999)
}

/// `_banned(d)` : cartes du deck bannies en Standard depuis la liste : jouables, signalées.
fn banned(d: &TDeck) -> Vec<String> {
    let mut all: Vec<&String> = vec![&d.legend];
    all.extend(d.champion.iter());
    all.extend(d.main.iter());
    all.extend(d.sideboard.iter());
    all.extend(d.battlefields.iter());
    let s: BTreeSet<String> = all.into_iter().filter(|n| !n.is_empty() && ban().contains_key(*n)).cloned().collect();
    s.into_iter().collect()
}

/// `default_decks()` : deck par défaut de l'IA pour chaque légende (menu de sélection).
fn default_decks() -> BTreeMap<String, String> {
    let mut best: BTreeMap<String, ((i64, i64, i64, String), String)> = BTreeMap::new();
    for k in decks().keys() {
        let d = tload(k);
        if validate(&d.json()).iter().any(|e| !e.starts_with('⚠')) {
            continue;
        }
        let rank = (
            if banned(&d).is_empty() { 0 } else { 1 },
            if d.sideboard.is_empty() { 1 } else { 0 },
            place(k),
            k.clone(),
        );
        let better = match best.get(&d.legend) {
            None => true,
            Some((r, _)) => rank < *r,
        };
        if better {
            best.insert(d.legend.clone(), (rank, k.clone()));
        }
    }
    best.into_iter().map(|(lg, (_, k))| (lg, k)).collect()
}

fn export(d: &TDeck) -> Json {
    jo! {
        "legend" => d.legend.clone(), "champion" => d.champion.clone(), "main" => d.main.clone(),
        "runes" => d.runes.iter().map(|r| if r.ends_with(" Rune") { r.clone() } else { format!("{r} Rune") }).collect::<Vec<_>>(),
        "battlefields" => d.battlefields.clone(), "sideboard" => d.sideboard.clone(),
    }
}

/// Liste d'un dict Python (`list(d.get(k) or [])`), textes seulement.
fn names_of(d: &Json, k: &str) -> Vec<String> {
    d.g(k).arr().iter().filter_map(|x| x.str().map(|s| s.to_string())).collect()
}

/// `validate(d)` : règles de construction (103) et cartes non modélisées. Renvoie la liste des problèmes (vide =
/// jouable).
pub fn validate(d: &Json) -> Vec<String> {
    let s = specs();
    let mut err = vec![];
    let lg = d.g("legend").str().filter(|x| !x.is_empty()).map(|x| x.to_string());
    let ch = d.g("champion").str().filter(|x| !x.is_empty()).map(|x| x.to_string());
    let main = names_of(d, "main");
    let runes = names_of(d, "runes");
    let bfs = names_of(d, "battlefields");
    let side = names_of(d, "sideboard");
    let is_type = |n: &str, t: &str| s.get(n).is_some_and(|x| x.type_ == t);
    let lg_ok = lg.as_deref().is_some_and(|l| is_type(l, "Legend"));
    let mut ident: BTreeSet<String> = BTreeSet::new();
    let mut ltags: BTreeSet<String> = BTreeSet::new();
    if !lg_ok {
        err.push(tr("choisis une légende", "choose a legend"));
    } else {
        let l = &s[lg.as_deref().unwrap()];
        ident = l.domains.iter().cloned().collect();
        ltags = l.tags.clone();
    }
    let ch_ok = ch.as_deref().is_some_and(|c| s.get(c).is_some_and(|x| x.super_ == "Champion" && x.type_ == "Unit"));
    if !ch_ok {
        err.push(tr("choisis un champion (unité Champion)", "choose a champion (Champion unit)"));
    } else if lg_ok && s[ch.as_deref().unwrap()].tags.intersection(&ltags).next().is_none() {
        let c = ch.as_deref().unwrap();
        let lt: Vec<&str> = ltags.iter().map(|x| x.as_str()).collect();
        err.push(tr(
            format!("{c} n'a pas le tag de la légende ({})", lt.join(", ")),
            format!("{c} doesn't have the legend's tag ({})", lt.join(", ")),
        ));
    }
    let nmain = main.len() + if ch.is_some() { 1 } else { 0 };
    if nmain < 40 {
        err.push(tr(
            format!("deck principal : {nmain}/40 cartes (champion compris)"),
            format!("main deck: {nmain}/40 cards (champion included)"),
        ));
    }
    // Counter(main + [ch]) : ordre d'arrivée
    let mut cnt: Vec<(String, i64)> = vec![];
    let add = |c: &mut Vec<(String, i64)>, n: &str| match c.iter_mut().find(|x| x.0 == n) {
        Some(e) => e.1 += 1,
        None => c.push((n.to_string(), 1)),
    };
    for n in main.iter().chain(ch.iter()) {
        add(&mut cnt, n);
    }
    if side.len() > 10 {
        err.push(tr(format!("sideboard : {}/10 cartes au plus", side.len()), format!("sideboard: {}/10 cards at most", side.len())));
    }
    let mut tot = cnt.clone();
    for n in &side {
        add(&mut tot, n);
    }
    tot.sort();
    for (n, q) in &tot {
        if *q > 3 {
            err.push(if side.contains(n) {
                tr(format!("{n} : {q} exemplaires (3 au plus, sideboard compris)"), format!("{n}: {q} copies (3 at most, sideboard included)"))
            } else {
                tr(format!("{n} : {q} exemplaires (3 au plus)"), format!("{n}: {q} copies (3 at most)"))
            });
        }
        if s.get(n.as_str()).is_some_and(|x| x.text.contains("Unique")) && *q > 1 {
            err.push(tr(format!("{n} est Unique : 1 exemplaire au plus"), format!("{n} is Unique: 1 copy at most")));
        }
    }
    let sig: i64 = cnt.iter().filter(|(n, _)| s.get(n.as_str()).is_some_and(|x| x.super_ == "Signature")).map(|x| x.1).sum();
    if sig > 3 {
        err.push(tr(format!("{sig} cartes Signature (3 au plus)"), format!("{sig} Signature cards (3 at most)")));
    }
    if lg.is_some() && !ident.is_empty() {
        let all: BTreeSet<&String> = cnt.iter().map(|x| &x.0).chain(side.iter()).collect();
        for n in all {
            let Some(sp) = s.get(n.as_str()) else {
                err.push(tr(format!("carte inconnue : {n}"), format!("unknown card: {n}")));
                continue;
            };
            if matches!(sp.type_.as_str(), "Rune" | "Battlefield" | "Legend") {
                let ins = side.contains(n);
                err.push(tr(
                    format!("{n} ne va pas dans le deck principal{}", if ins { " ni dans le sideboard" } else { "" }),
                    format!("{n} can't go in the main deck{}", if ins { " or the sideboard" } else { "" }),
                ));
            }
            if !sp.domains.iter().all(|x| ident.contains(x)) {
                err.push(tr(format!("{n} est hors des domaines de la légende"), format!("{n} is outside the legend's domains")));
            }
            if sp.super_ == "Signature" && sp.tags.intersection(&ltags).next().is_none() {
                err.push(tr(format!("{n} est une Signature d'un autre champion"), format!("{n} is another champion's Signature")));
            }
        }
    }
    if runes.len() != 12 {
        err.push(format!("runes : {}/12", runes.len()));
    }
    let rs: BTreeSet<&String> = runes.iter().collect();
    for r in rs {
        let dom = r.replace(" Rune", "");
        if !ident.is_empty() && !ident.contains(&dom) {
            err.push(tr(format!("{r} hors des domaines de la légende"), format!("{r} is outside the legend's domains")));
        }
    }
    let ub: BTreeSet<&String> = bfs.iter().collect();
    if bfs.len() != 3 || ub.len() != 3 {
        err.push(tr(format!("battlefields : {}/3 différents", ub.len()), format!("battlefields: {}/3 different", ub.len())));
    }
    for b in &bfs {
        if !is_type(b, "Battlefield") {
            err.push(tr(format!("{b} n'est pas un battlefield"), format!("{b} is not a battlefield")));
        } else if crate::cards_helpers::is_token(b) {
            // Baron Pit, Brush : « You can't start the game with a token battlefield. »
            err.push(tr(format!("{b} est un jeton : on ne peut pas commencer la partie avec"), format!("{b} is a token: you can't start the game with it")));
        }
    }
    let mut every: BTreeSet<&String> = cnt.iter().map(|x| &x.0).chain(side.iter()).chain(bfs.iter()).collect();
    if let Some(l) = &lg {
        every.insert(l);
    }
    let todo: Vec<&str> = every.iter().filter(|n| s.contains_key(n.as_str()) && !modeled(n)).map(|n| n.as_str()).collect();
    if !todo.is_empty() {
        err.push(tr("pas encore modélisées : ", "not modelled yet: ") + &todo.join(", "));
    }
    let bn: Vec<&str> = every.iter().filter(|n| s.contains_key(n.as_str()) && ban().contains_key(n.as_str())).map(|n| n.as_str()).collect();
    if !bn.is_empty() {
        err.push(tr("⚠ bannies en Standard : ", "⚠ banned in Standard: ") + &bn.join(", "));
    }
    err
}

fn check(x: &Json) -> Result<String, String> {
    let d = load_state(x)?;
    Ok(Json::from(validate(&d)).dumps(false))
}

// ------------------------------------------------------------------ boucle de la partie
/// `_run(op)` : exécute op ; si l'humain doit répondre à une question en cours de résolution, tout est annulé
/// jusqu'à la réponse.
fn run(w: &mut St, op: Op) -> Result<String, String> {
    let sv = save(w);
    let hus: Vec<usize> = {
        let g = w.g();
        (0..g.agents.0.len()).filter(|&i| human_of(&mut g.agents.0[i]).is_some()).collect()
    };
    for &i in &hus {
        human_of(&mut w.g().agents.0[i]).unwrap().k = 0;
    }
    w.g().tg.as_mut().unwrap().buf.clear();
    PENDING.with(|p| *p.borrow_mut() = None);
    exec_op(w, &op);
    let pend = PENDING.with(|p| p.borrow_mut().take());
    match pend {
        Some(Interrupt::Err(e)) => Err(e),
        Some(Interrupt::Need(mut e)) => {
            let shown = shown_now(w.g()); // main révélée pendant la résolution interrompue (Sabotage…)
            w.ask_shown = shown;
            e.seen = if e.pid == ME() { w.g().tg.as_ref().unwrap().ask_seen.clone() } else { None };
            if let Some(se) = &e.seen {
                w.seen_shown.insert(se.uids.clone()); // déjà montrées dans la question : pas une seconde fois après
            }
            let answers: Vec<(usize, Vec<Json>)> =
                hus.iter().map(|&i| (i, human_of(&mut w.g().agents.0[i]).unwrap().answers.clone())).collect();
            restore(w, &sv);
            for (i, a) in answers {
                // les réponses déjà données restent pour la reprise de op
                human_of(&mut w.g().agents.0[i]).unwrap().answers = a;
            }
            w.g().tg.as_mut().unwrap().buf.clear();
            let v = view(w, Some(&e));
            w.retry = Some((op, e));
            Ok(v)
        }
        None => {
            for &i in &hus {
                human_of(&mut w.g().agents.0[i]).unwrap().answers.clear();
            }
            w.retry = None;
            w.ask_shown = None;
            Ok(view(w, None))
        }
    }
}

fn exec_op(w: &mut St, op: &Op) {
    match op {
        Op::Tick => tick(w),
        Op::Act(a) => {
            let a = a.clone();
            w.g().apply(&a);
            w.d = None;
        }
        Op::AskMay(key) => {
            let me = ME();
            if let Ok(v) = w.g().ask(me, "may", vec![Value::Bool(true), Value::Bool(false)], Map::new()) {
                let e = w.extra.entry(key.clone()).or_insert(Json::Arr(vec![]));
                e.arr_mut().push(val_json(&v));
            }
        }
    }
}

/// `_tick()`
fn tick(w: &mut St) {
    let (me, ai, duel) = (ME(), AI(), DUEL());
    if w.d.is_none() {
        let d = w.g().advance(false);
        w.d = d;
        if let Some(d) = &w.d {
            if d.player == me || duel {
                // en duel : mêmes options chez les deux joueurs
                let mut d = d.clone();
                if expand(w.g(), &mut d).is_ok() {
                    w.d = Some(d);
                }
            }
        }
        return;
    }
    let d = w.d.clone().unwrap();
    let opts = d.options.clone().unwrap();
    if (d.player == me || duel) && opts.len() == 1 {
        // rien à choisir (seulement passer) : on passe pour l'humain
        w.g().apply(&opts[0]);
        w.d = None;
        return;
    }
    if d.player == ai && !duel {
        let g = w.g();
        let mut agent = std::mem::replace(&mut g.agents.0[ai], Box::new(crate::game_events::FirstChoice));
        let r = agent.decide(g, &d);
        g.agents.0[ai] = agent;
        let a = match r {
            Ok(a) => a,
            Err(_) => {
                if PENDING.with(|p| p.borrow().is_none()) {
                    PENDING.with(|p| *p.borrow_mut() = Some(Interrupt::Err("GameOver".into())));
                }
                return;
            }
        };
        let idx = opts.iter().position(|o| *o == a).unwrap();
        let n = AILOG.with(|l| {
            let mut l = l.borrow_mut();
            l.push(Json::Arr(vec!["d".into(), idx.into(), d.kind.into(), opts.len().into()]));
            l.len()
        });
        let k = a.list()[0].str().to_string();
        w.last_ai = if k != "pass" { Some(opp_describe(w.g(), &a, ai)) } else { None };
        if k != "pass" && opts.len() > 1 {
            // un vrai choix de l'IA : c'est lui que « Erreur de l'IA » marque
            let la = w.last_ai.clone();
            let tg = w.g().tg.as_mut().unwrap();
            tg.last_ai_step = Some(n - 1);
            tg.last_ai_label = la;
        }
        w.last_ai_info = if k != "pass" && k != "end" { Some(opp_info(w.g(), &a)) } else { None };
        w.g().apply(&a);
        w.d = None;
    }
}

fn with_w<T>(f: impl FnOnce(&mut St) -> Result<T, String>) -> Result<T, String> {
    W.with(|w| {
        let mut w = w.borrow_mut();
        if w.g.is_none() {
            return Err("KeyError: 'g'".into());
        }
        f(&mut w)
    })
}

/// `step()` : avance d'un cran (une décision de l'IA au plus). La page rappelle step() tant que busy est vrai.
fn step() -> Result<String, String> {
    with_w(|w| run(w, Op::Tick))
}

/// `act(i)`
fn act(i: &Json) -> Result<String, String> {
    with_w(|w| {
        let Some(d) = w.d.clone() else { return Err("AttributeError: 'NoneType' object has no attribute 'options'".into()) };
        let opts = d.options.clone().unwrap();
        let a = opts[py_index(&Json::Int(jint(i)?), opts.len())?].clone();
        let (me, duel) = (ME(), DUEL());
        if duel && d.player != me {
            // coup de l'adversaire reçu : affiché comme les coups de l'IA
            let k = a.list()[0].str().to_string();
            w.last_ai = if k != "pass" { Some(opp_describe(w.g(), &a, d.player)) } else { None };
            w.last_ai_info = if k != "pass" && k != "end" { Some(opp_info(w.g(), &a)) } else { None };
        }
        let sv = save(w);
        w.hist.push(sv); // en duel aussi : la page n'autorise « Reprendre » que tant que le joueur garde la main
        let n = w.hist.len();
        if n > 40 {
            w.hist.drain(..n - 40);
        }
        if !duel || d.player == me {
            // coup du joueur : les cartes vues avant ne sont plus montrées
            w.g().tg.as_mut().unwrap().seen_log.clear();
            w.seen_shown.clear();
        }
        run(w, Op::Act(a))
    })
}

/// `answer(x)`
fn answer(x: &Json) -> Result<String, String> {
    with_w(|w| {
        let Some((op, e)) = w.retry.clone() else { return Err("TypeError: cannot unpack non-iterable NoneType object".into()) };
        let v = match x {
            Json::Str(s) => parse(s).map_err(|e| format!("JSONDecodeError: {e}"))?,
            j => j.clone(),
        };
        let pid = e.pid;
        human_of(&mut w.g().agents.0[pid]).ok_or("AttributeError: answers")?.answers.push(v);
        run(w, op)
    })
}

/// `undo()` : reprend le dernier coup du joueur (état juste avant ce coup, y compris les réponses de l'IA qui ont
/// suivi).
fn undo() -> Result<String, String> {
    with_w(|w| {
        if let Some(sv) = w.hist.pop() {
            restore(w, &sv);
            let me = ME();
            if let Some(h) = human_of(&mut w.g().agents.0[me]) {
                h.answers.clear();
            }
            w.retry = None;
            if DUEL() {
                // la flèche du coup repris ne doit pas rester chez l'adversaire
                w.last_ai = None;
                w.last_ai_info = None;
            }
        }
        w.g().tg.as_mut().unwrap().buf.clear();
        Ok(view(w, None))
    })
}

/// `ai_script()` : journal de l'IA de la partie, avec la graine de la partie.
fn ai_script() -> String {
    let seed = W.with(|w| w.borrow().seed);
    let log = AILOG.with(|l| l.borrow().clone());
    jo! { "seed" => seed, "ai" => Json::Arr(log) }.dumps(true)
}

/// `flag()` : dernier vrai choix de l'IA (celui que le joueur peut marquer comme une erreur).
fn flag() -> String {
    let (step, label) = W.with(|w| {
        let w = w.borrow();
        match w.g.as_ref().and_then(|g| g.tg.as_ref()) {
            Some(t) => (t.last_ai_step, t.last_ai_label.clone()),
            None => (None, None),
        }
    });
    let n = AILOG.with(|l| l.borrow().len());
    jo! { "step" => step, "label" => label, "n" => n }.dumps(false)
}

/// `ai_replay(script)` : après new(), l'IA suit les coups enregistrés (ScriptAI) au lieu de les recalculer.
fn ai_replay(w: &mut St, script: &Json) -> Result<(), String> {
    let sc = load_state(script)?;
    let ai = AI();
    let g = w.g();
    let inner = std::mem::replace(&mut g.agents.0[ai], Box::new(crate::game_events::FirstChoice));
    g.agents.0[ai] = Box::new(ScriptAI { ag: inner, script: sc.arr().to_vec() });
    Ok(())
}

/// `_opp_describe(g, a, pid)` : coup de l'adversaire tel que tu le vois : une carte cachée reste secrète (421, 811).
fn opp_describe(g: &mut Game, a: &Value, pid: usize) -> String {
    let l = a.list();
    if l[0].str() == "hide" {
        let b = g.bfs[rp::bf_of(&l[2]).unwrap()].name.clone();
        return tr(format!("cache une carte à {b}"), format!("hides a card at {b}"));
    }
    describe(g, a, pid, Some(ME()))
}

/// `_opp_info(g, a)` : flèche du dernier coup adverse ; pour une carte cachée, ni son nom ni son identifiant.
fn opp_info(_g: &mut Game, a: &Value) -> Json {
    let mut out = info(a, None);
    let k = a.list()[0].str();
    out.set("k", k);
    out.set("src", if k == "hide" { Json::Null } else { src(a) });
    out
}

/// `hint(n)` : ce que l'IA (plan de ta légende) jouerait ici : les meilleures options et l'écart de score.
fn hint_w(w: &mut St, n: usize) -> Result<String, String> {
    let me = ME();
    let Some(d) = w.d.clone() else { return Ok("[]".into()) };
    let opts0 = d.options.clone().unwrap();
    if d.player != me || opts0.len() < 2 {
        return Ok("[]".into());
    }
    // Calcul sur une COPIE : la partie en cours n'est jamais remplacée.
    let (n0, i0) = get_counters();
    let mut g2 = deep(w.g());
    let d2 = d.clone();
    let c = w.coach.as_mut().unwrap();
    let mut opts = opts0.clone();
    if opts.len() > c.max_cands {
        let mut o = vec![opts[0].clone()];
        o.extend(c.rng.sample(&opts[1..], c.max_cands - 1));
        opts = o;
    }
    let r = c.pick(&mut g2, &d2, &opts);
    set_counters(n0, i0);
    let (_best, best_v, mut scored) = r.map_err(|_| "GameOver".to_string())?;
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let g = w.g();
    let mut idx: BTreeMap<String, usize> = BTreeMap::new();
    for (i, a) in opts0.iter().enumerate() {
        idx.insert(crate::actions::py_repr(g, a), i);
    }
    let mut out = vec![];
    for (v, a) in scored.iter().take(n) {
        let gap = v - best_v;
        let i = idx.get(&crate::actions::py_repr(g, a)).copied();
        out.push(jo! {
            "i" => i, "label" => describe(g, a, me, None),
            "gap" => if gap < -1000.0 { Json::from(tr("perd la partie", "loses the game")) } else { Json::Float(round1(gap)) },
        });
    }
    Ok(Json::Arr(out).dumps(false))
}

/// `round(x, 1)` de Python (arrondi correct de la valeur décimale exacte)
pub fn round1(x: f64) -> f64 {
    format!("{x:.1}").parse().unwrap()
}

fn hint(n: &Json) -> Result<String, String> {
    let n = if n.is_null() { 4 } else { jint(n)? as usize };
    with_w(|w| hint_w(w, n))
}

fn cards() -> Result<String, String> {
    with_w(|w| {
        let mut names: BTreeSet<String> = BTreeSet::new();
        for d in &w.decks {
            names.extend(d.main.iter().cloned());
            names.insert(d.legend.clone());
            if let Some(c) = &d.champion {
                names.insert(c.clone());
            }
            names.insert(d.battlefield.clone());
            names.extend(d.battlefields.iter().cloned());
        }
        for (n, sp) in specs() {
            if sp.super_ == "Token" {
                names.insert(n.clone());
            }
        }
        for n in ["Mech", "Reflection", "Gold"] {
            names.insert(n.into());
        }
        Ok(rp::card_db(names.iter().filter(|n| !n.is_empty()).map(|n| n.as_str())).dumps(false))
    })
}

// ------------------------------------------------------------------ mode replay (« Mes parties » → « Revoir »)
fn replay_load(game_json: &Json) -> Result<String, String> {
    let game = load_state(game_json)?;
    let prev = W.with(|w| {
        let mut w = w.borrow_mut();
        if w.g.is_some() {
            Some(save(&mut w))
        } else {
            None
        }
    });
    let level = match game.get("level") {
        Some(l) => l.clone(),
        None => Json::Int(1),
    };
    let meta = parse(&new(game.g("seed"), game.g("bf"), game.g("first"), &level, game.g("mine"), game.g("opp"), game.g("obf"))?)
        .unwrap();
    if game.g("ai").truthy() {
        with_w(|w| ai_replay(w, game.g("ai")))?; // coups de l'IA enregistrés : fidèle même si l'IA a changé
    }
    let mut frames: Vec<String> = vec![];
    let mut svs: BTreeMap<usize, Option<Save>> = BTreeMap::new();
    let mut logs: Vec<Json> = vec![];
    let mut mine: Vec<Json> = vec![];
    let mut desync: Option<usize> = None;
    let push = |frames: &mut Vec<String>, logs: &mut Vec<Json>, v: &Json, lab: &str, who: &str| {
        let k = frames.len();
        for e in v.g("log").arr() {
            let e = e.arr();
            logs.push(Json::Arr(vec![e[0].clone(), e[1].clone(), k.into()]));
        }
        let mut v = v.clone();
        v.set("log", Json::Arr(vec![]));
        frames.push(jo! { "v" => v, "lab" => lab, "who" => who }.dumps(false));
    };
    let drain = |frames: &mut Vec<String>, logs: &mut Vec<Json>, v: Json, lab: String, who: &str| -> Result<Json, String> {
        let (mut v, mut lab, mut who) = (v, lab, who.to_string());
        loop {
            if v.g("log").truthy() || !v.g("busy").truthy() {
                let l = if who == "ia" { short_ai(&v) } else { lab.clone() };
                push(frames, logs, &v, &l, &who);
                lab = String::new();
            }
            if !v.g("busy").truthy() {
                return Ok(v);
            }
            v = parse(&step()?).unwrap();
            who = "ia".into();
        }
    };
    let mut v = drain(&mut frames, &mut logs, parse(&step()?).unwrap(), tr("Début de la partie", "Start of the game"), "jeu")?;
    for mv in game.g("moves").arr() {
        let ma = mv.arr();
        let k = ma.first().and_then(|x| x.str()).unwrap_or("");
        if k == "hint" || k == "flag" {
            continue;
        }
        if k == "undo" {
            v = drain(&mut frames, &mut logs, parse(&undo()?).unwrap(), tr("Tu reprends ton dernier coup", "You take back your last move"), "moi")?;
            continue;
        }
        let j = frames.len() - 1; // l'image où tu décides
        let x = ma.get(1).cloned().unwrap_or(Json::Null);
        let nopts = v.g("dec").g("options").arr().len() as i64;
        let xi = match &x {
            Json::Int(i) => Some(*i),
            Json::Bool(b) => Some(*b as i64),
            _ => None,
        };
        if k == "act" && v.g("dec").truthy() && xi.is_some_and(|i| i < nopts && i >= -nopts) {
            let i = xi.unwrap();
            let ii = if i < 0 { i + nopts } else { i } as usize;
            let lab = v.g("dec").g("options").arr()[ii].g("label").str().unwrap_or("").to_string();
            let sv = W.with(|w| save(&mut w.borrow_mut()));
            svs.insert(j, Some(sv));
            mine.push(Json::Arr(vec![j.into(), (tr("Tu joues : ", "You play: ") + &lab).into()]));
            v = drain(&mut frames, &mut logs, parse(&act(&Json::Int(i))?).unwrap(), tr("Toi : ", "You: ") + &lab, "moi")?;
        } else if k == "ans" && v.g("ask").truthy() {
            let a = v.g("ask").clone();
            let ao = a.g("options").arr();
            let lab = match &x {
                Json::Arr(l) => l
                    .iter()
                    .filter_map(|i| match i {
                        Json::Int(i) if *i < ao.len() as i64 && *i >= -(ao.len() as i64) => {
                            let ii = if *i < 0 { i + ao.len() as i64 } else { *i } as usize;
                            Some(ao[ii].str().unwrap_or("").to_string())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
                Json::Int(i) if *i < ao.len() as i64 && *i >= -(ao.len() as i64) => {
                    let ii = if *i < 0 { i + ao.len() as i64 } else { *i } as usize;
                    ao[ii].str().unwrap_or("").to_string()
                }
                x => py_str_json(x),
            };
            svs.insert(j, None); // une réponse à une question : pas de conseil possible
            let title = a.g("title").str().unwrap_or("").to_string();
            let l2 = if lab.is_empty() { tr("rien", "nothing") } else { lab.clone() };
            mine.push(Json::Arr(vec![
                j.into(),
                tr(format!("Tu réponds ({title}) : {l2}"), format!("You answer ({title}): {l2}")).into(),
            ]));
            v = drain(&mut frames, &mut logs, parse(&answer(&Json::Str(x.dump()))?).unwrap(), tr("Toi : ", "You: ") + &l2, "moi")?;
        } else {
            desync = Some(mine.len());
            break;
        }
    }
    let n = frames.len();
    W.with(|w| w.borrow_mut().rp = Some(Rp { frames, svs, prev }));
    let res = game.g("result");
    let same = !res.truthy() || (res.g("winner") == v.g("winner") && res.g("pts") == v.g("st").g("pts"));
    Ok(jo! {
        "meta" => meta, "n" => n, "log" => Json::Arr(logs), "mine" => Json::Arr(mine), "desync" => desync, "same" => same,
        "end" => jo! { "winner" => v.g("winner").clone(), "pts" => v.g("st").g("pts").clone(), "t" => v.g("st").g("t").clone() },
    }
    .dumps(false))
}

fn short_ai(v: &Json) -> String {
    if v.g("ai").truthy() {
        format!("{}{}{}", rp::names()[AI()], tr(" : ", ": "), v.g("ai").str().unwrap_or(""))
    } else {
        String::new()
    }
}

fn replay_frame(k: &Json) -> Result<String, String> {
    let k = jint(k)?;
    W.with(|w| {
        let w = w.borrow();
        let rp = w.rp.as_ref().ok_or("KeyError: 'rp'")?;
        let n = rp.frames.len() as i64;
        let i = if k < 0 { k + n } else { k };
        if i < 0 || i >= n {
            return Err("IndexError: list index out of range".into());
        }
        Ok(rp.frames[i as usize].clone())
    })
}

/// `replay_hint(k, n)` : conseil de l'IA à ta décision de l'image k (état gardé au rejeu) ; la partie rejouée n'est
/// pas touchée.
fn replay_hint(k: &Json, n: &Json) -> Result<String, String> {
    let k = jint(k)? as usize;
    let n = if n.is_null() { 5 } else { jint(n)? as usize };
    with_w(|w| {
        let sv = w.rp.as_ref().ok_or("KeyError: 'rp'")?.svs.get(&k).cloned().flatten();
        let Some(sv) = sv else { return Ok("[]".into()) };
        let cur = save(w);
        restore(w, &sv);
        let r = hint_w(w, n);
        restore(w, &cur);
        r
    })
}

/// `replay_exit()` : la partie en cours avant le replay est rendue telle quelle.
fn replay_exit() -> String {
    W.with(|w| {
        let mut w = w.borrow_mut();
        let rp = w.rp.take();
        if let Some(Rp { prev: Some(prev), .. }) = rp {
            restore(&mut w, &prev);
            return jo! { "prev" => true }.dumps(true);
        }
        jo! { "prev" => false }.dumps(true)
    })
}

// ------------------------------------------------------------------ libellés
/// `_olabel(g, o)` : o désigne un objet de `src` (la partie de la question, ou g lui-même quand `same`).
fn olabel(g: &Game, src: &Game, same: bool, o: &Value) -> String {
    let me = ME();
    match o {
        Value::None => tr("aucun", "none"),
        Value::Bool(true) => tr("oui", "yes"),
        Value::Bool(false) => tr("non", "no"),
        Value::Obj(x) => {
            let ob = src.o(*x);
            let on = g.obj(ob.uid).is_some();
            let wh = match ob.loc {
                None => String::new(),
                Some(l) if on => {
                    " (".to_string()
                        + &match l {
                            Loc::Bf(i) => g.bfs[i].name.clone(),
                            Loc::Base => "base".into(),
                        }
                        + ")"
                }
                Some(_) => String::new(),
            };
            let mine = if ob.ctrl == me { String::new() } else { tr(" adverse", " (enemy)") };
            let name = if on {
                if same {
                    g.label(*x)
                } else {
                    src.cname(*x).to_string() // dup_no : l'objet de la partie abandonnée n'est pas sur ce plateau
                }
            } else {
                src.cname(*x).to_string()
            };
            format!("{name}{mine}{wh}")
        }
        Value::Rune(r) => format!("rune {}", src.r(*r).domain),
        Value::Opt(x) => x.label.clone(),
        Value::Loc(Loc::Bf(i)) if *i < g.bfs.len() => g.bfs[*i].name.clone(),
        Value::Int(i) if *i >= 0 && (*i as usize) < g.bfs.len() => g.bfs[*i as usize].name.clone(),
        Value::Loc(Loc::Base) => tr("la base", "base"),
        Value::Str(s) if &**s == "base" => tr("la base", "base"),
        Value::Int(i) => match g.obj(*i as u32) {
            Some(u) => olabel(g, g, true, &Value::Obj(u)),
            None => i.to_string(),
        },
        Value::Map(mp) => crate::cards_helpers::choice_label(g, mp),
        Value::List(l) | Value::Tuple(l) => {
            if l.is_empty() {
                tr("aucun", "none")
            } else {
                l.iter().map(|x| olabel(g, src, same, x)).collect::<Vec<_>>().join(" + ")
            }
        }
        Value::Str(s) => word(s),
        v => short(&crate::actions::py_str(src, v)),
    }
}

fn word(o: &str) -> String {
    let (fr, en) = match o {
        "top" => ("sur le dessus du deck", "on top of the deck"),
        "bottom" => ("sous le deck", "at the bottom of the deck"),
        "draw" => ("piocher", "draw"),
        "discard" => ("défausser", "discard"),
        "cards" => ("des cartes", "cards"),
        "runes" => ("des runes", "runes"),
        "buff" => ("renforcer (buff)", "buff"),
        "ready" => ("préparer", "ready"),
        "exhaust" => ("épuiser", "exhaust"),
        "channel" => ("canaliser 1 rune", "channel 1 rune"),
        _ => return o.to_string(),
    };
    if lang::en() {
        en.to_string()
    } else {
        fr.to_string()
    }
}

/// `_alabel(g, ask, o)` : libellé d'une option selon la question. None : l'exception de Python (repli sur
/// `_olabel`).
fn alabel_try(g: &Game, ask: &Need, i: usize, o: &Value) -> Option<String> {
    let src = &*ask.ag;
    let me = ME();
    let k = ask.kind.as_str();
    if k == "predict_recycle" {
        return Some(if o.truthy() {
            tr("la recycler (sous le deck)", "recycle it (bottom of the deck)")
        } else {
            tr("la garder sur le dessus", "keep it on top")
        });
    }
    if k == "pay_gold" || k == "pay_rune" {
        let pl = ask.ctx.get("plans")?.list().get(i)?.map().clone();
        let kg = pl.g("gold").int();
        let ru = match o {
            Value::Tuple(t) => Some(t[1].int() as u32),
            _ => None,
        };
        let mut rr: Vec<crate::game::R> = pl.g("recycle").list().iter().map(|r| r.rune()).collect();
        rr.sort_by(|a, b| {
            let (ra, rb) = (src.r(*a), src.r(*b));
            (Some(ra.uid) != ru, &ra.domain).cmp(&(Some(rb.uid) != ru, &rb.domain))
        });
        if let Some(ru) = ru {
            // une rune touchée : on la nomme
            let r = src.r(*rr.iter().find(|x| src.r(**x).uid == ru)?);
            let st = if r.exhausted { tr(" épuisée", " exhausted") } else { tr(" prête", " ready") };
            return Some(tr(format!("recycler la rune {}{st}", r.domain), format!("recycle the {} rune{st}", r.domain)));
        }
        let rs: Vec<&str> = rr.iter().map(|r| src.r(*r).domain.as_str()).collect();
        let rs = rs.join(", ");
        let gold = if kg != 0 { tr(format!("utiliser {kg} Gold"), format!("use {kg} Gold")) } else { String::new() };
        let rec = if !rr.is_empty() {
            tr(format!("recycler {} rune(s) ({rs})", rr.len()), format!("recycle {} rune(s) ({rs})", rr.len()))
        } else {
            String::new()
        };
        return Some([gold, rec].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" + "));
    }
    if k == "damage_pick" {
        let ob = src.o(o.opt_obj()?);
        let need = match ask.ctx.get("need") {
            Some(Value::Map(m)) => match m.get(&ob.uid.to_string()) {
                Some(v) => crate::actions::py_str(src, v),
                None => "?".into(),
            },
            _ => "?".into(),
        };
        let l = olabel(g, src, false, o);
        return Some(tr(format!("{l} (mortel : {need} dégâts)"), format!("{l} (lethal: {need} damage)")));
    }
    if (k == "burn_player" || k == "choose_player") && matches!(o, Value::Int(_)) {
        return Some(if o.int() as usize == me { tr("toi", "you") } else { tr("l'adversaire", "the opponent") });
    }
    if k == "bullet_time_x" {
        return Some(format!("X = {}", crate::actions::py_str(src, o)));
    }
    if k == "ava_play" {
        let pl = ask.ctx.get("plays")?.list().get(usize::try_from(o.opt_int()?).ok()?)?.clone();
        let p = pl.list();
        return Some(format!("{} ({})", src.cname(p[0].opt_obj()?), olabel(g, src, false, &p[1])));
    }
    if k == "entourage" {
        let p = o.list();
        let q = p.first()?.opt_int()?;
        let a = p.get(1)?.opt_str()?;
        return Some(format!(
            "{} {}",
            word(a),
            if q as usize == me { tr("ta légende", "your legend") } else { tr("la légende adverse", "the opponent's legend") }
        ));
    }
    if k == "split_damage" {
        if !o.truthy() {
            return Some(tr("aucune cible", "no target"));
        }
        let l = o.list();
        if l.iter().all(|x| matches!(x, Value::Int(_) | Value::Bool(_))) {
            return Some(l.iter().map(|x| x.int().to_string()).collect::<Vec<_>>().join(" + ") + &tr(" dégâts", " damage"));
        }
        let mut parts = vec![];
        for x in l {
            let p = x.list();
            let (u, n) = (p.first()?, p.get(1)?);
            let n = crate::actions::py_str(src, n);
            let ul = olabel(g, src, false, u);
            parts.push(tr(format!("{n} à {ul}"), format!("{n} to {ul}")));
        }
        return Some(parts.join(", "));
    }
    if (k == "mf_ready" || k == "ready_pick") && matches!(o, Value::Tuple(_)) {
        return Some(olabel(g, src, false, o.list().get(1)?));
    }
    if k == "bard_move" {
        let p = o.list();
        let (bf, grp) = (p.first()?, p.get(1)?);
        let us: Vec<String> = grp.list().iter().map(|u| olabel(g, src, false, u)).collect();
        let us = if us.is_empty() { tr("aucune unité", "no unit") } else { us.join(" + ") };
        let b = olabel(g, src, false, bf);
        return Some(tr(format!("vers {b} : "), format!("to {b}: ")) + &us);
    }
    if (k == "move_enemy" || k == "profiteer") && matches!(o, Value::Tuple(_)) {
        let p = o.list();
        return Some(format!("{} → {}", olabel(g, src, false, p.first()?), olabel(g, src, false, p.get(1)?)));
    }
    if let Value::Tuple(p) = o {
        if p.len() == 2 {
            if let (Value::Obj(c), Value::Map(_)) = (&p[0], &p[1]) {
                return Some(format!("{} ({})", src.cname(*c), olabel(g, src, false, &p[1])));
            }
        }
    }
    None
}

fn alabel(g: &Game, ask: &Need, i: usize, o: &Value) -> String {
    alabel_try(g, ask, i, o).unwrap_or_else(|| olabel(g, &ask.ag, false, o))
}

/// `ASKS`
fn asks(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "mulligan" => "Mulligan : choisis jusqu'à 2 cartes à remettre",
        "may" => "Utiliser cet effet optionnel ?",
        "play_location" => "Où poser l'unité ?",
        "equip_target" => "Équiper quelle unité ?",
        "discard" => "Défausser quelle carte ?",
        "copy_target" => "Copier quelle unité ?",
        "sacrifice" => "Sacrifier quelle unité ?",
        "recycle_rune" => "Recycler quelle rune ?",
        "pay_gold" => "Payer la puissance : avec un Gold ou en recyclant une rune ?",
        "pay_rune" => "Payer la puissance : quelle rune recycler ?",
        "hook_pick" => "Baited Hook : quelle unité prendre ?",
        "zhonya_save" => "Zhonya : sauver quelle unité ?",
        "star_spring" => "Star Spring : quelle unité ?",
        "dusk_kill" => "Détruire quelle unité ?",
        "herald_pick" => "Quelle unité ?",
        "herald_dk_pick" => "Quelle carte ?",
        "mixologist_pick" => "Quelle unité ?",
        "ashe_pick" => "Quelle carte ?",
        "target" => "Cible :",
        "damage_order" => "Ordre des dégâts :",
        "damage_pick" => "Combat : quelle unité reçoit d'abord ses dégâts mortels ?",
        _ => return None,
    })
}

/// `_src(g, a)`
fn src(a: &Value) -> Json {
    let l = a.list();
    match l[0].str() {
        "play" | "hide" => val_json(&l[1]),
        "act" => match &l[1] {
            Value::Tuple(_) | Value::List(_) => "legend".into(),
            v => val_json(v),
        },
        "move" => val_json(&l[1].list()[0]),
        _ => Json::Null,
    }
}

/// Valeur simple du moteur en JSON (uids, emplacements : entier ou « base », textes).
fn val_json(v: &Value) -> Json {
    match v {
        Value::None => Json::Null,
        Value::Bool(b) => Json::Bool(*b),
        Value::Int(i) => Json::Int(*i),
        Value::Float(f) => Json::Float(*f),
        Value::Str(s) => Json::Str(s.to_string()),
        Value::Loc(Loc::Base) => "base".into(),
        Value::Loc(Loc::Bf(i)) => (*i).into(),
        Value::List(l) | Value::Tuple(l) => Json::Arr(l.iter().map(val_json).collect()),
        Value::Map(m) => Json::Obj(m.0.iter().map(|(k, x)| (k.to_string(), val_json(x))).collect()),
        _ => Json::Null,
    }
}

fn seq(v: &Value) -> Vec<Value> {
    match v {
        Value::List(l) | Value::Tuple(l) => l.clone(),
        Value::None => vec![],
        v => vec![v.clone()],
    }
}

/// `_info(a, g)` : données structurées d'une action pour le glisser-déposer et les flèches.
fn info(a: &Value, g: Option<&mut Game>) -> Json {
    let l = a.list();
    let k = l[0].str();
    let me = ME();
    match k {
        "play" => {
            let ch = l[3].map().clone();
            // [Repeat] (820) : nombre de répétitions payées et leurs cibles, quelle que soit la façon dont la carte les code
            let reps = seq(&ch.g("reps"));
            let nrep = (if ch.g("rep").truthy() || ch.g("rep2").truthy() || ch.g("sq").truthy() { 1 } else { 0 })
                + reps.len()
                + seq(&ch.g("rc")).len();
            let mut t2 = seq(&ch.g("tg2"));
            for r in &reps {
                t2.extend(seq(&r.map().g("tg")));
            }
            let loc = match ch.get("loc") {
                Some(v) => v.clone(),
                None => ch.g("dest"),
            };
            let mut out = jo! {
                "t1" => Json::Arr(seq(&ch.g("tg")).iter().map(val_json).collect()),
                "t2" => Json::Arr(t2.iter().map(val_json).collect()),
                "loc" => val_json(&loc), "mover" => val_json(&ch.g("mover")), "acc" => ch.g("acc").truthy(),
                "rep" => nrep, "from_" => val_json(&l[2]),
            };
            if let Some(g) = g {
                if nrep > 0 && !t2.is_empty() {
                    let ns: Vec<String> = t2.iter().filter(|u| matches!(u, Value::Int(_))).map(|u| tname(g, u, me, None)).collect();
                    out.set("t2n", ns.join(&tr(" et ", " and ")));
                }
                if nrep > 0 {
                    if let Some(c) = find(g, me, l[1].opt_int().unwrap_or(-1) as u32) {
                        let ri = crate::actions::repeat_instances(g, me, c).map(|v| v.len()).unwrap_or(0);
                        let has = g.impl_o(c).is_some_and(|im| im.repeat.truthy());
                        if ri + has as usize >= 2 {
                            // plusieurs [Repeat] possibles : la table montre le coût total de chaque façon de répéter
                            if let Ok((e, reqs)) = crate::actions::total_cost(g, me, c, &ch, l[2].str()) {
                                out.set("repc", cost_txt(e, &reqs));
                            }
                        }
                    }
                }
                out.set("xc", crate::cards_helpers::play_cost_parts(g, &ch).join(", "));
                if !ch.g("show").is_none() {
                    // [Show Off] (832) : l'objet montré
                    if let Ok(Value::Map(sh)) = crate::actions::shown_info(g, me, ch.g("show").int() as u32) {
                        out.set("show", val_json(&sh.g("uid")));
                        out.set("showN", val_json(&sh.g("name")));
                        out.set("showW", val_json(&sh.g("where")));
                    }
                }
            }
            out
        }
        "act" => {
            let ch = match l.get(3) {
                Some(Value::Map(m)) => m.clone(),
                _ => Map::new(),
            };
            jo! {
                "t1" => Json::Arr(seq(&ch.g("tg")).iter().map(val_json).collect()), "loc" => val_json(&ch.g("dest")),
                "exh" => val_json(&ch.g("exh")),
            }
        }
        "hide" | "move" => jo! { "loc" => val_json(&l[2]) },
        _ => jo! {},
    }
}

/// `_cost_txt(e, reqs)`
fn cost_txt(e: i64, reqs: &[crate::game_costs::Req]) -> String {
    let mut parts = if e != 0 { vec![tr(format!("{e} énergie"), format!("{e} energy"))] } else { vec![] };
    let any = crate::game_costs::any_req();
    let mut cnt: Vec<(String, i64)> = vec![];
    for r in reqs {
        let k = if *r == any {
            tr("rune au choix", "rune of any type")
        } else {
            r.iter().cloned().collect::<Vec<_>>().join(&tr(" ou ", " or "))
        };
        match cnt.iter_mut().find(|x| x.0 == k) {
            Some(x) => x.1 += 1,
            None => cnt.push((k, 1)),
        }
    }
    parts.extend(cnt.into_iter().map(|(k, n)| format!("{n} {k}")));
    if parts.is_empty() {
        tr("gratuit", "free")
    } else {
        parts.join(" + ")
    }
}

/// `_act_label(g, a)` : nom et coût d'une capacité activée.
fn act_label(g: &mut Game, a: &Value) -> String {
    let me = ME();
    let l = a.list();
    let ch = match l.get(3) {
        Some(Value::Map(m)) => m.clone(),
        _ => Map::new(),
    };
    let (obj, name, im) = match &l[1] {
        Value::Tuple(t) | Value::List(t) => {
            let p = t[1].int() as usize;
            let n = g.p[p].legend_name.clone();
            (Some(g.p[p].legend), n.clone(), impl_of(&n))
        }
        v => {
            let o = v.opt_int().and_then(|u| g.obj(u as u32));
            let n = o.map(|o| g.cname(o).to_string()).unwrap_or_else(|| "?".into());
            (o, n, o.and_then(|o| g.impl_o(o)))
        }
    };
    let r = (|| -> Option<String> {
        let im = im?;
        let obj = obj?;
        let ab = im.abilities.get(l[2].opt_int()? as usize)?.clone();
        let c = ab.g("cost").call(g, &[me.into(), obj.into(), ch.clone().into()]).ok()?;
        let (e, reqs) = crate::actions::cost_of(&c);
        let mut extra = String::new();
        // Coût choisi (« en épuisant X »…) avant la cible : la table regroupe les variantes par libellé sans la cible
        let mut keys: Vec<&str> = ch.keys().collect();
        keys.sort();
        for k in keys {
            if let Some((fr, en)) = crate::cards_helpers::CHOICE_UIDS.iter().find(|x| x.0 == k).map(|x| (x.1, x.2)) {
                let v = ch.g(k);
                if !v.is_none() {
                    let vs = seq(&v);
                    if !vs.is_empty() {
                        let ns: Vec<String> = vs.iter().map(|x| crate::cards_helpers::uid_name(g, x)).collect();
                        extra += &format!(", {} {}", tr(fr, en), ns.join(", "));
                    }
                }
            }
        }
        if ch.g("tg").truthy() {
            let ns: Vec<String> = ch.g("tg").list().iter().map(|u| tname(g, u, me, None)).collect();
            extra += &(tr(", cible ", ", target ") + &ns.join(&tr(" et ", " and ")));
        }
        let exh = if ab.g("exhaust").truthy() { tr(", épuise", ", exhausts") } else { String::new() };
        let an = crate::actions::py_str(g, &ab.g("name"));
        Some(format!("{an}{}{name} ({}{exh}{extra})", tr(" : ", ": "), cost_txt(e, &reqs)))
    })();
    match r {
        Some(s) => s,
        None => describe(g, a, me, None),
    }
}

/// `_ask_uid(g, o)` : objet d'un choix cliquable sur la table : unité ou équipement en jeu, ou carte de ta main.
fn ask_uid(g: &Game, src: &Game, o: &Value) -> Json {
    let me = ME();
    let uid = match o {
        Value::Obj(x) => src.o(*x).uid,
        Value::Rune(r) => {
            let u = src.r(*r).uid;
            if g.p[me].runes.iter().any(|&x| g.r(x).uid == u) {
                return u.into(); // une de tes runes : on la touche sur le plateau
            }
            u
        }
        _ => return Json::Null,
    };
    // La question vient d'une copie de l'état (rejouée ensuite) : on compare par uid, pas par identité.
    if g.obj(uid).is_some() || g.p[me].hand.iter().any(|&c| g.o(c).uid == uid) {
        return uid.into();
    }
    Json::Null
}

/// Questions dont les options entières ne sont pas des battlefields.
const NOT_LOC: [&str; 6] = ["pay_gold", "pay_rune", "bullet_time_x", "burn_player", "choose_player", "ava_play"];

/// `_pay_gold_uids(g, ask)` : l'option ("r", uid) se répond en touchant cette rune, l'option k en touchant le k-ième
/// Gold prêt.
fn pay_gold_uids(g: &Game, ask: &Need) -> Vec<Json> {
    let golds = g.golds(ask.pid);
    let mine: BTreeSet<u32> = g.p[ask.pid].runes.iter().map(|&r| g.r(r).uid).collect();
    let mut out = vec![];
    for o in &ask.options {
        match o {
            Value::Tuple(t) => {
                let u = t[1].int() as u32;
                out.push(if mine.contains(&u) { u.into() } else { Json::Null });
            }
            v => {
                let k = v.int();
                out.push(if 0 < k && k as usize <= golds.len() { g.o(golds[k as usize - 1]).uid.into() } else { Json::Null });
            }
        }
    }
    if ask.pid == ME() {
        out
    } else {
        vec![Json::Null; out.len()]
    }
}

/// `_ask_play(g, ask, o)` : option « jouer telle carte avec tel choix » : {n, u, tg} ou None.
fn ask_play(ask: &Need, o: &Value) -> Json {
    let src = &*ask.ag;
    let mut o = o.clone();
    if ask.kind == "ava_play" {
        if let Value::Int(i) = o {
            match ask.ctx.get("plays").and_then(|p| p.list().get(i as usize).cloned()) {
                Some(x) => o = x,
                None => return Json::Null,
            }
        }
    }
    if let Value::Tuple(p) = &o {
        if p.len() == 2 {
            if let (Value::Obj(c), Value::Map(ch)) = (&p[0], &p[1]) {
                let tg: Vec<Json> = seq(&ch.g("tg")).iter().filter(|u| matches!(u, Value::Int(_))).map(val_json).collect();
                return jo! { "n" => src.cname(*c), "u" => src.o(*c).uid, "tg" => Json::Arr(tg) };
            }
        }
    }
    Json::Null
}

/// `_ask_multi(g, ask)` : question « jusqu'à N unités » : pour chaque option, la liste de ses uid.
fn ask_multi(g: &Game, ask: &Need) -> Json {
    if NOT_LOC.contains(&ask.kind.as_str()) || ask.pid != ME() {
        return Json::Null;
    }
    let mut out = vec![];
    for o in &ask.options {
        let Value::Opt(x) = o else { return Json::Null };
        let Value::Tuple(v) = &x.value else { return Json::Null };
        if !v.iter().all(|u| matches!(u, Value::Int(i) if g.obj(*i as u32).is_some())) {
            return Json::Null;
        }
        out.push(Json::Arr(v.iter().map(val_json).collect()));
    }
    if out.iter().any(|x| x.truthy()) {
        Json::Arr(out)
    } else {
        Json::Null
    }
}

/// `_ask_src(g, ask)`
fn ask_src(g: &Game, ask: &Need) -> Json {
    let src = &*ask.ag;
    if let Some(Value::Item(it)) = ask.ctx.get("item") {
        if let Some(s) = src.it(*it).src {
            if g.obj(s).is_some() {
                return s.into();
            }
        }
    }
    match ask.ctx.get("card") {
        Some(Value::Obj(c)) => src.o(*c).uid.into(),
        Some(Value::Rune(r)) => src.r(*r).uid.into(),
        _ => Json::Null,
    }
}

/// `_src_name(n)` : nom de carte de l'effet en cours : « Baited Hook: Hook » (capacité) → « Baited Hook ».
fn src_name(n: &str) -> Option<String> {
    let mut n = n.to_string();
    if !n.is_empty() && !specs().contains_key(&n) {
        if let Some((a, _)) = n.rsplit_once(": ") {
            if specs().contains_key(a) {
                n = a.to_string();
            }
        }
    }
    let s = short(&n);
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// `_opt_card_uid(o)`
fn opt_card_uid(src: &Game, o: &Value) -> Option<u32> {
    match o {
        Value::Obj(x) => Some(src.o(*x).uid),
        _ => None,
    }
}

/// `_seen_title(se)` : légende des cartes vues : qui les voit, d'où, par quel effet.
fn seen_title(se: &Seen) -> String {
    let me = ME();
    let src = se.src.as_ref().map(|s| format!("{s} : ")).unwrap_or_default();
    let n = se.names.len();
    if se.ev == "look" {
        let whose = if se.owner == me { tr("ton deck", "your deck") } else { tr("le deck adverse", "the opponent's deck") };
        return src
            + &tr(
                format!("tu regardes {} du dessus de {whose}", if n == 1 { "la carte".to_string() } else { format!("les {n} cartes") }),
                format!("you look at the top {} of {whose}", if n == 1 { "card".to_string() } else { format!("{n} cards") }),
            );
    }
    let ai = &rp::names()[AI()];
    let who = if se.pid == me { tr("Tu révèles", "You reveal") } else { tr(format!("{ai} révèle"), format!("{ai} reveals")) };
    let whose = if se.owner == me {
        tr("ton deck", "your deck")
    } else if se.pid == se.owner {
        tr("son deck", "their deck")
    } else {
        tr("le deck adverse", "the opponent's deck")
    };
    src + &tr(
        format!("{who} {} du dessus de {whose}", if n == 1 { "la carte".to_string() } else { format!("{n} cartes") }),
        format!("{who} the top {} of {whose}", if n == 1 { "card".to_string() } else { format!("{n} cards") }),
    )
}

/// `_ask_title(g, ask)` : titre de la question, précédé du nom de la carte qui demande quand il n'y figure pas déjà.
fn ask_title(g: &Game, ask: &Need) -> String {
    let src = &*ask.ag;
    if ask.kind == "predict_recycle" {
        let cn = match ask.ctx.get("card") {
            Some(Value::Obj(c)) => src.cname(*c).to_string(),
            _ => "?".into(),
        };
        let n = ask.ctx.get("n").map(|v| v.int()).unwrap_or(1);
        let i = ask.ctx.get("i").map(|v| v.int()).unwrap_or(0);
        let (cf, ce) = if n > 1 { (format!(" (carte {}/{n})", i + 1), format!(" (card {}/{n})", i + 1)) } else { (String::new(), String::new()) };
        return tr(
            format!("Predict : dessus de ton deck{cf} : {cn}. La recycler sous le deck, ou la garder ? (règle 436)"),
            format!("Predict: top of your deck{ce}: {cn}. Recycle it to the bottom of the deck, or keep it? (rule 436)"),
        );
    }
    if ask.kind == "predict_top" {
        return tr(
            "Predict : quelle carte remettre sur le dessus ? (la première choisie sera piochée en premier)",
            "Predict: which card do you put back on top? (the first one chosen will be drawn first)",
        );
    }
    if ask.kind == "damage_pick" {
        let left = crate::actions::py_str(src, &ask.ctx.g("left"));
        return tr(
            format!("Combat : il te reste {left} dégâts à assigner. Quelle unité reçoit d'abord ses dégâts mortels ? (règle 465.2.c : mortel en entier avant la suivante, l'excédent va à la dernière)"),
            format!("Combat: you have {left} damage left to assign. Which unit receives its lethal damage first? (rule 465.2.c: lethal in full before the next one, the excess goes to the last one)"),
        );
    }
    let mut t = asks(&ask.kind).map(|s| s.to_string()).or_else(|| registry().ask_text.get(&ask.kind).cloned());
    if let Some(x) = &t {
        t = Some(lang::ask_title(&ask.kind, x));
    }
    let mut nm = match ask.ctx.get("item") {
        Some(Value::Item(it)) => src.it(*it).name.clone(),
        _ => String::new(),
    };
    if nm.is_empty() {
        if let Json::Int(u) = ask_src(g, ask) {
            if let Some(o) = g.obj(u as u32) {
                nm = g.cname(o).to_string();
            }
        }
    }
    let t = t.unwrap_or_else(|| tr("Choisis une option :", "Choose an option:"));
    if !nm.is_empty() && !t.contains(&short(&nm)) && ask.kind != "mulligan" {
        return format!("{} — {t}", short(&nm));
    }
    t
}

/// `_shown_now(g)` : main révélée telle qu'elle est maintenant : {pid, cards: [[uid, nom, révélée?]...]}.
fn shown_now(g: &Game) -> Option<Json> {
    let sh = g.shown_hand.as_ref()?;
    let pid = sh.g("pid").int() as usize;
    let uids: Vec<i64> = sh.g("uids").list().iter().map(|u| u.int()).collect();
    let cards: Vec<Json> = g.p[pid]
        .hand
        .iter()
        .map(|&c| {
            let u = g.o(c).uid;
            Json::Arr(vec![u.into(), g.cname(c).into(), uids.contains(&(u as i64)).into()])
        })
        .collect();
    Some(jo! { "pid" => pid, "cards" => Json::Arr(cards) })
}

/// `_view(ask)` : ce que la page affiche.
fn view(w: &mut St, ask: Option<&Need>) -> String {
    let (me, ai, duel) = (ME(), AI(), DUEL());
    let ask_shown = w.ask_shown.clone();
    let last_ai = w.last_ai.take();
    let last_ai_info = w.last_ai_info.take();
    let hist_n = w.hist.len();
    let seen_shown = w.seen_shown.clone();
    let d = w.d.clone();
    let g = w.g.as_mut().unwrap();
    let mut st = snap(g);
    // Main de l'adversaire cachée, sauf les cartes qu'un effet vient de révéler (Sabotage, Scuttle Crab… 424)
    let sh = (if ask.is_some() { ask_shown } else { None }).or_else(|| shown_now(g));
    let sh_pid = sh.as_ref().map(|s| s.g("pid").int().unwrap() as usize);
    {
        let p = st.get_mut("p").unwrap().arr_mut();
        if sh_pid == Some(ai) {
            let hand: Vec<Json> = sh
                .as_ref()
                .unwrap()
                .g("cards")
                .arr()
                .iter()
                .map(|c| {
                    let c = c.arr();
                    Json::Arr(vec![0.into(), if c[2].truthy() { c[1].clone() } else { "?".into() }])
                })
                .collect();
            p[ai].set("hand", Json::Arr(hand));
            p[ai].set("rv", 1);
        } else {
            let n = p[ai].g("hand").arr().len();
            p[ai].set("hand", Json::Arr((0..n).map(|_| Json::Arr(vec![0.into(), "?".into()])).collect()));
        }
        if sh_pid == Some(me) {
            p[me].set("rv", 1); // ta main est révélée à l'adversaire
        }
    }
    let look = g.p[ai].revealed_turn == g.turn_no; // Scuttle Crab : « you can look at their facedown cards this turn »
    for b in st.get_mut("bfs").unwrap().arr_mut() {
        if !b.g("fd").is_null() && b.g("fd").arr()[0].int() == Some(ai as i64) && !look {
            b.set("fd", Json::Arr(vec![ai.into(), "?".into()]));
        }
        if !look {
            let fds: Vec<Json> = b
                .g("fds")
                .arr()
                .iter()
                .map(|x| {
                    let x = x.arr();
                    if x[0].int() == Some(ai as i64) {
                        Json::Arr(vec![ai.into(), "?".into()])
                    } else {
                        Json::Arr(vec![x[0].clone(), x[1].clone()])
                    }
                })
                .collect();
            b.set("fds", Json::Arr(fds));
        }
    }
    let tg = g.tg.as_mut().unwrap();
    let buf: Vec<Json> = tg.buf.iter().map(|(i, t)| Json::Arr(vec![(*i).into(), t.clone().into()])).collect();
    let cw = std::mem::take(&mut tg.cw); // vainqueurs des combats terminés depuis le dernier rendu (467)
    let seen_log = tg.seen_log.clone();
    {
        let chain = st.get_mut("chain").unwrap().arr_mut();
        for (c, it) in chain.iter_mut().zip(g.chain.iter()) {
            let it = g.it(*it);
            let mut t: Vec<Json> = seq(&it.data.g("tg")).iter().map(val_json).collect();
            t.extend(seq(&it.data.g("tg2")).iter().map(val_json));
            c.set("tg", Json::Arr(t));
            c.set("src", it.src);
        }
    }
    {
        let pl = &g.p[me];
        let pm = &mut st.get_mut("p").unwrap().arr_mut()[me];
        pm.set("champu", Json::Arr(pl.champ.iter().map(|&c| g.o(c).uid.into()).collect()));
        pm.set("runeu", Json::Arr(pl.runes.iter().map(|&r| g.r(r).uid.into()).collect()));
        pm.set("trashu", Json::Arr(pl.trash.iter().map(|&c| Json::Arr(vec![g.o(c).uid.into(), g.cname(c).into()])).collect()));
        pm.set("banu", Json::Arr(pl.banish.iter().map(|&c| Json::Arr(vec![g.o(c).uid.into(), g.cname(c).into()])).collect()));
        let mut kt = vec![]; // dessus connu (Predict, Vision) : tant qu'il reste dessus
        for &c in &pl.deck {
            if !pl.seen.contains(&g.o(c).uid) {
                break;
            }
            kt.push(Json::from(g.cname(c)));
        }
        pm.set("known_top", Json::Arr(kt));
    }
    for (b, gb) in st.get_mut("bfs").unwrap().arr_mut().iter_mut().zip(g.bfs.iter()) {
        if let Some(fd) = gb.facedown() {
            if g.o(fd).owner == me {
                b.set("fdu", g.o(fd).uid);
            }
        }
        b.set("fdus", Json::Arr(gb.facedowns.iter().map(|&c| if g.o(c).owner == me { g.o(c).uid.into() } else { Json::Null }).collect()));
    }
    let mut out = jo! {
        "st" => st, "log" => Json::Arr(buf), "winner" => g.winner, "ai" => last_ai, "aii" => last_ai_info,
        "cw" => cw, "undo" => hist_n,
    };
    if ask.is_none() {
        // cartes vues ou révélées sans question à poser : montrées jusqu'au prochain coup du joueur (act)
        let seen: Vec<Json> = seen_log
            .iter()
            .filter(|e| !seen_shown.contains(&e.uids))
            .map(|e| jo! { "t" => seen_title(e), "cards" => e.names.clone() })
            .collect();
        out.set("seen", Json::Arr(seen));
    }
    if let (Some(a), true) = (ask, duel) {
        if a.pid != me {
            out.set("wait", "adversaire"); // question posée à l'adversaire : on attend sa réponse
            out.set("me", me);
            return out.dumps(false);
        }
    }
    if let Some(a) = ask {
        let gg: &Game = g;
        let src = &*a.ag;
        let options: Vec<Json> = a.options.iter().enumerate().map(|(i, o)| alabel(gg, a, i, o).into()).collect();
        let uids: Vec<Json> = if a.kind == "pay_gold" || a.kind == "pay_rune" {
            pay_gold_uids(gg, a)
        } else {
            a.options.iter().map(|o| ask_uid(gg, src, o)).collect()
        };
        let nb = gg.bfs.len();
        let zs: Vec<Json> = a
            .options
            .iter()
            .map(|o| {
                if NOT_LOC.contains(&a.kind.as_str()) {
                    return Json::Null;
                }
                match o {
                    Value::Loc(Loc::Base) => "base".into(),
                    Value::Str(s) if &**s == "base" => "base".into(),
                    Value::Loc(Loc::Bf(i)) if *i < nb => i.to_string().into(),
                    Value::Int(i) if *i >= 0 && (*i as usize) < nb => i.to_string().into(),
                    _ => Json::Null,
                }
            })
            .collect();
        let item = match a.ctx.get("item") {
            Some(Value::Item(it)) => {
                let s = short(&src.it(*it).name);
                if s.is_empty() {
                    Json::Null
                } else {
                    s.into()
                }
            }
            _ => Json::Null,
        };
        let mut aj = jo! {
            "kind" => a.kind.clone(), "title" => ask_title(gg, a), "options" => Json::Arr(options), "uids" => Json::Arr(uids),
            "zs" => Json::Arr(zs), "src" => ask_src(gg, a), "item" => item,
            "pc" => Json::Arr(a.options.iter().map(|o| ask_play(a, o)).collect()), "tg" => ask_multi(gg, a),
        };
        if a.kind == "predict_recycle" && a.ctx.get("card").is_some_and(|c| !c.is_none()) {
            // les cartes vues (Predict N), en grand dans la question
            let top = match a.ctx.get("top") {
                Some(t) if t.truthy() => t.list().to_vec(),
                _ => vec![a.ctx.g("card")],
            };
            aj.set("show", Json::Arr(top.iter().map(|c| Json::from(src.cname(c.obj()))).collect()));
            aj.set("cur", a.ctx.get("i").map(|v| v.int()).unwrap_or(0));
        } else if let Some(se) = &a.seen {
            // cartes regardées ou révélées par cet effet : toutes en grand, et pour chacune l'indice du choix qui la prend
            aj.set("show", se.names.clone());
            let pick: Vec<Json> = se
                .uids
                .iter()
                .map(|u| {
                    let ix: Vec<usize> =
                        a.options.iter().enumerate().filter(|(_, o)| opt_card_uid(src, o) == Some(*u)).map(|(i, _)| i).collect();
                    if ix.len() == 1 {
                        Json::from(ix[0])
                    } else {
                        Json::Int(-1)
                    }
                })
                .collect();
            aj.set("pick", Json::Arr(pick));
            aj.set("seenT", seen_title(se));
        }
        out.set("ask", aj);
    } else if g.winner.is_some() {
    } else if d.as_ref().is_some_and(|d| d.player == me) {
        let d = d.unwrap();
        let opts = d.options.clone().unwrap();
        // Le moteur ne donne des options qu'à un exemplaire par nom de carte : les autres renvoient vers celui-là.
        let mut have: Vec<((String, String), Json)> = vec![];
        for a in &opts {
            let l = a.list();
            let k = l[0].str();
            if k == "play" || k == "hide" {
                if let Some(c) = find(g, me, l[1].opt_int().unwrap_or(-1) as u32) {
                    let key = (g.cname(c).to_string(), if k == "play" { l[2].str().to_string() } else { "hand".into() });
                    if !have.iter().any(|x| x.0 == key) {
                        have.push((key, val_json(&l[1])));
                    }
                }
            }
        }
        let mut alias = jo! {};
        for &c in &g.p[me].hand {
            let key = (g.cname(c).to_string(), "hand".to_string());
            if let Some((_, u)) = have.iter().find(|x| x.0 == key) {
                if u.int() != Some(g.o(c).uid as i64) {
                    alias.set(&g.o(c).uid.to_string(), u.clone());
                }
            }
        }
        out.set("alias", alias);
        let mut dopts = vec![];
        for (i, a) in opts.iter().enumerate() {
            let l = a.list();
            let k = l[0].str();
            let label = if k == "act" { act_label(g, a) } else { describe(g, a, me, None) };
            let mut o = jo! {
                "i" => i, "k" => k, "src" => src(a),
                "us" => if k == "move" { val_json(&l[1]) } else { Json::Null }, "label" => label,
            };
            if let Json::Obj(kv) = info(a, Some(g)) {
                for (kk, vv) in kv {
                    o.set(&kk, vv);
                }
            }
            dopts.push(o);
        }
        out.set("dec", jo! { "kind" => d.kind, "options" => Json::Arr(dopts) });
    } else if duel && d.as_ref().is_some_and(|d| d.player != me) {
        out.set("wait", "adversaire"); // décision de l'adversaire : on attend son coup
    } else {
        out.set("busy", true);
    }
    out.set("me", me);
    out.dumps(false)
}

// ------------------------------------------------------------------ cibles pour l'humain
/// clé d'une action (`repr(a[:3] + (sorted(a[3].items()),))`)
fn akey(g: &Game, a: &Value) -> String {
    let l = a.list();
    if l.len() > 3 {
        if let Value::Map(m) = &l[3] {
            let mut items: Vec<(&str, &Value)> = m.0.iter().map(|(k, v)| (&**k, v)).collect();
            items.sort_by(|x, y| x.0.cmp(y.0));
            let its: Vec<Value> = items.into_iter().map(|(k, v)| Value::Tuple(vec![k.into(), v.clone()])).collect();
            let mut t: Vec<Value> = l[..3].to_vec();
            t.push(Value::List(its));
            return crate::actions::py_repr(g, &Value::Tuple(t));
        }
    }
    crate::actions::py_repr(g, a)
}

/// Item factice (`_It`) pour évaluer les prédicats de cible au moment du choix ; retiré ensuite (aucun numéro pris).
fn fake_item(g: &mut Game, pid: usize, hb: Option<Loc>) -> crate::game::ItemId {
    let mut data = Map::new();
    data.set("hidden_bf", hb);
    g.items.push(crate::game::Item {
        id: 0,
        kind: "spell",
        ctrl: pid,
        name: String::new(),
        fn_: Value::None,
        card: None,
        src: None,
        data,
        targets: vec![],
        chosen: BTreeSet::new(),
        uncounterable: false,
    });
    crate::game::ItemId(g.items.len() as u32 - 1)
}

fn combos(n: usize, k: usize, rep: bool) -> Vec<Vec<usize>> {
    let mut out = vec![];
    let mut cur = vec![];
    fn rec(n: usize, k: usize, rep: bool, start: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for i in start..n {
            cur.push(i);
            rec(n, k, rep, if rep { i } else { i + 1 }, cur, out);
            cur.pop();
        }
    }
    rec(n, k, rep, 0, &mut cur, &mut out);
    out
}

/// `_expand(g, d)` : pour le joueur humain, toutes les combinaisons de cibles légales (prédicats de la carte, Deflect
/// payé), et toutes les variantes des sorts « cible puis déplacement ».
fn expand(g: &mut Game, d: &mut Decision) -> Res<()> {
    let pid = d.player;
    let mut opts = d.options.clone().unwrap();
    let mut seen: BTreeSet<String> = opts.iter().map(|a| akey(g, a)).collect();
    // groupes : (uid, src, show) -> actions, dans l'ordre d'apparition
    let mut groups: Vec<((Value, Value, Value), Vec<Value>)> = vec![];
    for a in &opts {
        let l = a.list();
        if l[0].str() == "play" {
            if let Value::Map(ch) = &l[3] {
                if ch.contains("tg") && ch.keys().all(|k| k == "tg" || k == "flow" || k == "show") {
                    let key = (l[1].clone(), l[2].clone(), ch.g("show"));
                    match groups.iter_mut().find(|x| x.0 == key) {
                        Some(x) => x.1.push(a.clone()),
                        None => groups.push((key, vec![a.clone()])),
                    }
                }
            }
        }
    }
    for ((uid, srcv, _show), lst) in groups {
        let tg0 = lst[0].list()[3].map().g("tg");
        let n = tg0.list().len();
        if n == 0
            || lst.iter().any(|a| {
                let t = a.list()[3].map().g("tg");
                t.list().len() != n || !t.list().iter().all(|x| matches!(x, Value::Int(_) | Value::Bool(_)))
            })
        {
            continue;
        }
        let card = find(g, pid, uid.int() as u32);
        let Some(card) = card else { continue };
        let Some(im) = g.impl_o(card) else { continue };
        let preds = im.preds.list().to_vec();
        if preds.is_empty() {
            continue;
        }
        let pr: Vec<Value> = (0..n).map(|i| preds[i.min(preds.len() - 1)].clone()).collect();
        let hb = if srcv.str() == "facedown" { g.o(card).hidden_bf } else { None };
        let it = fake_item(g, pid, hb);
        let rep = lst.iter().any(|a| {
            let t: BTreeSet<i64> = a.list()[3].map().g("tg").list().iter().map(|x| x.int()).collect();
            t.len() < n
        });
        let same = pr.iter().all(|p| *p == pr[0]);
        let mut cs: Vec<Vec<O>> = vec![];
        let r = (|| -> Res<()> {
            if same {
                let mut slot = vec![];
                for o in g.board.clone() {
                    if g.targetable(o, Some(pid))? && pr[0].call(g, &[Value::Item(it), Value::Obj(o)])?.truthy() {
                        slot.push(o);
                    }
                }
                for c in combos(slot.len(), n, rep) {
                    cs.push(c.into_iter().map(|i| slot[i]).collect());
                }
            } else {
                // prédicats dépendants (la 2e cible selon la 1re) : it.targets se remplit au fil des cibles
                fn walk(g: &mut Game, it: crate::game::ItemId, pr: &[Value], pid: usize, rep: bool, i: usize, pre: &mut Vec<O>, out: &mut Vec<Vec<O>>) -> Res<()> {
                    if i == pr.len() {
                        out.push(pre.clone());
                        return Ok(());
                    }
                    let ts: Vec<(Option<u32>, i64, Value)> = pre.iter().map(|&o| (Some(g.o(o).uid), g.o(o).oid, Value::None)).collect();
                    let ch: BTreeSet<u32> = pre.iter().map(|&o| g.o(o).uid).collect();
                    g.it_mut(it).targets = ts;
                    g.it_mut(it).chosen = ch;
                    let mut cand = vec![];
                    for o in g.board.clone() {
                        if g.targetable(o, Some(pid))? && pr[i].call(g, &[Value::Item(it), Value::Obj(o)])?.truthy() {
                            cand.push(o);
                        }
                    }
                    for o in cand {
                        if rep || !pre.contains(&o) {
                            pre.push(o);
                            walk(g, it, pr, pid, rep, i + 1, pre, out)?;
                            pre.pop();
                        }
                    }
                    Ok(())
                }
                walk(g, it, &pr, pid, rep, 0, &mut vec![], &mut cs)?;
            }
            Ok(())
        })();
        g.items.pop();
        r?;
        let mut extra = lst[0].list()[3].map().clone();
        extra.remove("tg");
        for c in cs {
            let mut ch = extra.clone();
            ch.set("tg", Value::Tuple(c.iter().map(|&o| Value::from(g.o(o).uid)).collect()));
            let a = Value::Tuple(vec!["play".into(), uid.clone(), srcv.clone(), ch.clone().into()]);
            let k = akey(g, &a);
            if seen.contains(&k) || !crate::actions::affordable(g, pid, card, &ch, srcv.str())? {
                continue;
            }
            seen.insert(k);
            opts.push(a);
        }
    }
    // Sorts « cible puis déplacement » (Shuriken Flip) : l'humain a droit à toutes les combinaisons.
    let mut mgroups: Vec<((Value, Value), Vec<Value>)> = vec![];
    for a in d.options.as_ref().unwrap() {
        let l = a.list();
        if l[0].str() == "play" {
            if let Value::Map(ch) = &l[3] {
                if ch.contains("mover") {
                    let key = (l[1].clone(), l[2].clone());
                    match mgroups.iter_mut().find(|x| x.0 == key) {
                        Some(x) => x.1.push(a.clone()),
                        None => mgroups.push((key, vec![a.clone()])),
                    }
                }
            }
        }
    }
    for ((uid, srcv), lst) in mgroups {
        let Some(card) = find(g, pid, uid.int() as u32) else { continue };
        let Some(im) = g.impl_o(card) else { continue };
        let hb = if srcv.str() == "facedown" { g.o(card).hidden_bf } else { None };
        let it = fake_item(g, pid, hb);
        let r = (|| -> Res<(Vec<O>, Vec<O>)> {
            let mut ens = vec![];
            for e in crate::cards_helpers::enemies(g, pid, false, hb, true)? {
                if im.preds.list().is_empty() || im.preds.list()[0].call(g, &[Value::Item(it), Value::Obj(e)])?.truthy() {
                    ens.push(e);
                }
            }
            let mut fr = crate::cards_helpers::friends(g, pid, false, None)?;
            if hb.is_some() {
                let f2: Vec<O> = fr.iter().copied().filter(|&u| g.o(u).loc == hb).collect();
                if !f2.is_empty() {
                    fr = f2;
                }
            }
            Ok((ens, fr))
        })();
        g.items.pop();
        let (ens, fr) = r?;
        let mut extra = lst[0].list()[3].map().clone();
        for k in ["tg", "mover", "mover_oid", "dest"] {
            extra.remove(k);
        }
        let mut ts: Vec<Value> = ens.iter().map(|&e| Value::Tuple(vec![g.o(e).uid.into()])).collect();
        ts.push(Value::Tuple(vec![]));
        for t in ts {
            for &u in &fr {
                for dst in g.locs() {
                    if Some(dst) == g.o(u).loc {
                        continue;
                    }
                    let mut ch = extra.clone();
                    ch.set("tg", t.clone());
                    ch.set("mover", g.o(u).uid);
                    ch.set("mover_oid", g.o(u).oid);
                    ch.set("dest", dst);
                    let a = Value::Tuple(vec!["play".into(), uid.clone(), srcv.clone(), ch.clone().into()]);
                    let k = akey(g, &a);
                    if seen.contains(&k) || !crate::actions::affordable(g, pid, card, &ch, srcv.str())? {
                        continue;
                    }
                    seen.insert(k);
                    opts.push(a);
                }
            }
        }
    }
    d.options = Some(opts);
    Ok(())
}

// ------------------------------------------------------------------ appel depuis la page
/// Appel d'une fonction du module (`train.<fn>(*args)`) : le JSON renvoyé, ou le texte de l'erreur Python.
pub fn call(f: &str, a: &[Json]) -> Result<String, String> {
    match f {
        "set_lang" => Ok(lang::set_lang(arg(a, 0).str().unwrap_or("")).to_string()),
        "new" => new(&arg(a, 0), &arg(a, 1), &arg(a, 2), &arg(a, 3), &arg(a, 4), &arg(a, 5), &arg(a, 6)),
        "tuto" => tuto(),
        "tuto_seeds" => tuto_seeds(
            if arg(a, 0).is_null() { 3 } else { jint(&arg(a, 0))? as usize },
            if arg(a, 1).is_null() { 1 } else { jint(&arg(a, 1))? },
        ),
        "duel_new" => duel_new(&arg(a, 0), &arg(a, 1), &arg(a, 2), &arg(a, 3), &arg(a, 4), &arg(a, 5), &arg(a, 6), &arg(a, 7)),
        "duel_off" => Ok(duel_off()),
        "match_new" => match_new(&arg(a, 0), &arg(a, 1), &arg(a, 2), &arg(a, 3)),
        "match_next" => match_next(&arg(a, 0), &arg(a, 1), &arg(a, 2), &arg(a, 3)),
        "match_duel_next" => match_duel_next(&arg(a, 0)),
        "match_record" => match_record(&arg(a, 0), &arg(a, 1), &arg(a, 2), &arg(a, 3)),
        "match_swap" => match_swap(&arg(a, 0), &arg(a, 1), &arg(a, 2), &arg(a, 3)),
        "catalog" => Ok(catalog()),
        "check" => check(&arg(a, 0)),
        "step" => step(),
        "act" => act(&arg(a, 0)),
        "answer" => answer(&arg(a, 0)),
        "undo" => undo(),
        "ai_script" => Ok(ai_script()),
        "flag" => Ok(flag()),
        "cards" => cards(),
        "hint" => hint(&arg(a, 0)),
        "replay_load" => replay_load(&arg(a, 0)),
        "replay_frame" => replay_frame(&arg(a, 0)),
        "replay_hint" => replay_hint(&arg(a, 0), &arg(a, 1)),
        "replay_exit" => Ok(replay_exit()),
        "dbg" => crate::train_dbg::dbg(&arg(a, 0)),
        _ => Err(format!("AttributeError: module 'train' has no attribute '{f}'")),
    }
}

// ------------------------------------------------------------------ accès pour les robots (train_dbg.rs)
pub(crate) fn dbg_with<T>(f: impl FnOnce(&mut Game, &mut Option<Decision>, &mut BTreeMap<String, Json>, Option<i64>) -> T) -> Result<T, String> {
    W.with(|w| {
        let mut w = w.borrow_mut();
        let St { g, d, extra, level, .. } = &mut *w;
        let g = g.as_mut().ok_or("KeyError: 'g'")?;
        Ok(f(g, d, extra, *level))
    })
}

#[allow(non_snake_case)]
pub(crate) fn dbg_me() -> usize {
    ME()
}

/// Pour les robots : une question `may` posée pendant une opération (`_run(op)` avec `W["g"].ask(ME, "may", [True,
/// False])`, verif_espace) ; la réponse va dans `extra[key]`.
pub(crate) fn dbg_ask_may(key: &str) -> Result<String, String> {
    with_w(|w| run(w, Op::AskMay(key.to_string())))
}
