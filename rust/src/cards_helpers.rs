//! Aides génériques de `cards.py` (tout sauf les cartes du pool) : valeur des unités, cibles, prédicats, coûts
//! (`parse_cost`...), capacités (`ability`, `equip_ability_cost`, `empower_ability`), jetons, Disarm, Deflect,
//! contrôle des decks et cartes faites seulement de mots-clés (`register_auto`).
//!
//! Conventions propres à ce module (en plus de `docs/memoire/portage-rust.md`) :
//! - un prédicat de cible Python `P_x(g, it, o)` existe en fonction Rust `p_x(g, it, o)` et en valeur `p_x_v()`
//!   (`Value::func`, arguments `[Item, Obj]`, comme `Game::legal` l'appelle) ;
//! - un coût d'`ability()` s'écrit `Cost::Text("1 energy and 1 calm rune")`, `Cost::Parts(e, reqs)` ou
//!   `Cost::Fn(f)` (f(g, pid, obj, choice) -> `(e, reqs)`) ;
//! - les `Req` rangés dans une valeur sont des listes de domaines (`game_costs::reqs_to_value` / `reqs_of`).
use std::collections::BTreeSet;

use crate::cards::{Impl, Registry};
use crate::game::{Game, ItemId, Res, O};
use crate::game_costs::Req;
use crate::value::{Loc, Map, Value};

/// Coût d'une capacité (`cost=` de `ability()` / `equip=` / `empower=` d'`Impl`).
#[derive(Clone, Debug)]
pub enum Cost {
    /// pas de coût (`None` ou '')
    None,
    /// texte imprimé : '1 energy and 1 calm rune'
    Text(String),
    /// tuple (énergie, reqs)
    Parts(i64, Vec<Req>),
    /// fonction f(g, pid, obj, choice) -> (e, reqs)
    Fn(Value),
}

impl From<&str> for Cost {
    fn from(s: &str) -> Self {
        Cost::Text(s.to_string())
    }
}

/// Arguments d'`ability(name, ...)` (valeurs par défaut de Python : timing 'main', rien d'autre).
#[derive(Clone, Debug)]
pub struct AbilityArgs {
    pub cost: Cost,
    pub timing: &'static str,
    pub exhaust: bool,
    pub xp: i64,
    pub kill_self: bool,
    /// resolve(g, item) ; None : ne fait rien
    pub resolve: Value,
    /// choices(g, pid, obj) -> [choice]
    pub choices: Value,
    /// liste de prédicats, ou None
    pub preds: Value,
    /// can(g, pid, obj) -> bool
    pub can: Value,
    /// extra_cost(g, pid, obj, choice) : coûts non-ressources (peut renvoyer le choix modifié, voir `actions`)
    pub extra_cost: Value,
    /// les autres clés (`**extra` de Python : printed_e, ignore_deflect, all_choices, can_extra...)
    pub extra: Map,
}

impl Default for AbilityArgs {
    fn default() -> Self {
        AbilityArgs {
            cost: Cost::None,
            timing: "main",
            exhaust: false,
            xp: 0,
            kill_self: false,
            resolve: Value::None,
            choices: Value::None,
            preds: Value::None,
            can: Value::None,
            extra_cost: Value::None,
            extra: Map::new(),
        }
    }
}

/// Champs d'`Impl` rendus par `parse_keywords` (dict Python `fields`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KwFields {
    pub timing: Option<&'static str>,
    pub accelerate: bool,
    pub hidden: bool,
    pub ambush: bool,
    pub quickdraw: bool,
    /// None : pas de clé 'kw' (aucun mot-clé statique)
    pub kw: Option<Map>,
}

/// Paquet donné à `deck_problems` : un dict de deck, ou une liste de noms.
#[derive(Clone, Debug)]
pub enum DeckArg {
    Dict { legend: Option<String>, champion: Option<String>, main: Vec<String>, runes: Vec<String>, battlefields: Vec<String> },
    Names(Vec<String>),
}

// ---------------------------------------------------------------------------------------------- valeurs, cibles
/// `value(g, o)` : valeur grossière d'une unité, pour ordonner les choix de l'IA.
pub fn value(_g: &mut Game, _o: O) -> Res<f64> {
    todo!()
}
/// `enemies(g, pid, at_bf, hidden_bf, targetable)`
pub fn enemies(_g: &mut Game, _pid: usize, _at_bf: bool, _hidden_bf: Option<Loc>, _targetable: bool) -> Res<Vec<O>> {
    todo!()
}
/// `friends(g, pid, at_bf, hidden_bf)`
pub fn friends(_g: &mut Game, _pid: usize, _at_bf: bool, _hidden_bf: Option<Loc>) -> Res<Vec<O>> {
    todo!()
}
/// `all_units(g, pid, at_bf, hidden_bf)`
pub fn all_units(_g: &mut Game, _pid: usize, _at_bf: bool, _hidden_bf: Option<Loc>) -> Res<Vec<O>> {
    todo!()
}
/// `hb_ok(it, o)`
pub fn hb_ok(_g: &Game, _it: ItemId, _o: O) -> bool {
    todo!()
}
pub fn p_unit(_g: &Game, _it: ItemId, _o: O) -> bool {
    todo!()
}
pub fn p_unit_bf(_g: &Game, _it: ItemId, _o: O) -> bool {
    todo!()
}
pub fn p_enemy(_g: &Game, _it: ItemId, _o: O) -> bool {
    todo!()
}
pub fn p_enemy_bf(_g: &Game, _it: ItemId, _o: O) -> bool {
    todo!()
}
pub fn p_friend(_g: &Game, _it: ItemId, _o: O) -> bool {
    todo!()
}
pub fn p_friend_bf(_g: &Game, _it: ItemId, _o: O) -> bool {
    todo!()
}
pub fn p_gear(_g: &Game, _it: ItemId, _o: O) -> bool {
    todo!()
}
pub fn p_unit_v() -> Value {
    todo!()
}
pub fn p_unit_bf_v() -> Value {
    todo!()
}
pub fn p_enemy_v() -> Value {
    todo!()
}
pub fn p_enemy_bf_v() -> Value {
    todo!()
}
pub fn p_friend_v() -> Value {
    todo!()
}
pub fn p_friend_bf_v() -> Value {
    todo!()
}
pub fn p_gear_v() -> Value {
    todo!()
}
/// `tg_choices(units)` : [dict(tg=(uid,))]
pub fn tg_choices(_g: &Game, _units: &[O]) -> Vec<Map> {
    todo!()
}
/// `cap(g, seq, n, pid)` : les n premiers pour l'IA, tout pour un humain.
pub fn cap<T>(_g: &Game, _seq: Vec<T>, _n: usize, _pid: Option<usize>) -> Vec<T> {
    todo!()
}
/// `item_by_id(g, iid)` : l'item de la chaîne de cet id.
pub fn item_by_id(_g: &Game, _iid: i64) -> Option<ItemId> {
    todo!()
}
/// `trig_target(options_fn, pred, kind, optional, deflect)` : options_fn(g, it) -> [obj], pred(g, it, o) ou None.
/// Renvoie `choose(g, it) -> bool` avec l'attribut `options`.
pub fn trig_target(_options_fn: Value, _pred: Value, _kind: &'static str, _optional: bool, _deflect: bool) -> Value {
    todo!()
}
/// `may_pay(e, reqs_fn)` : coût optionnel cost(g, it) avec les attributs can, label, label_en.
pub fn may_pay(_e: i64, _reqs_fn: Value) -> Value {
    todo!()
}
/// `extra_label(g, ex)`
pub fn extra_label(_g: &Game, _ex: &Map) -> String {
    todo!()
}
/// `play_unit_free(g, pid, c, src, loc_choices, ignore_energy, alt_cost)`
pub fn play_unit_free(
    _g: &mut Game,
    _pid: usize,
    _c: O,
    _src: &str,
    _loc_choices: Option<Vec<Loc>>,
    _ignore_energy: bool,
    _alt_cost: Option<(i64, Vec<Req>)>,
) -> Res<Option<O>> {
    todo!()
}
/// `unit_play_choices(g, pid, c, src, locs)`
pub fn unit_play_choices(_g: &mut Game, _pid: usize, _c: O, _src: &str, _locs: Option<Vec<Loc>>) -> Res<Vec<Map>> {
    todo!()
}
/// `remake_choices(g, it, pid)` (750-755)
pub fn remake_choices(_g: &mut Game, _it: ItemId, _pid: usize) -> Res<()> {
    todo!()
}
/// `play_cost_parts(g, ch)`
pub fn play_cost_parts(_g: &Game, _ch: &Map) -> Vec<String> {
    todo!()
}
/// `_uid_name(g, x)`
pub fn uid_name(_g: &Game, _x: &Value) -> String {
    todo!()
}
/// `_choice_label(g, ch)`
pub fn choice_label(_g: &Game, _ch: &Map) -> String {
    todo!()
}
/// `_CHOICE_UIDS` / `_CHOICE_UIDS_EN` : (clé, français, anglais)
pub const CHOICE_UIDS: &[(&str, &str, &str)] = &[];
/// `gain_control_item(g, it, pid)` (359.3.f)
pub fn gain_control_item(_g: &mut Game, _it: ItemId, _pid: usize) {
    todo!()
}

// ---------------------------------------------------------------------------------------------- coûts, capacités
/// `parse_cost(text)` : '2 energy and 1 fury rune' -> (2, [{Fury}])
pub fn parse_cost(_text: &str) -> (i64, Vec<Req>) {
    todo!()
}
/// `power_list(text)` : (e, ['A' | domaine])
pub fn power_list(_text: &str) -> (i64, Vec<String>) {
    todo!()
}
/// `repeat_cost(text)` : tuple (énergie, domaines, power) pour `Impl.repeat`
pub fn repeat_cost(_text: &str) -> Value {
    todo!()
}
/// `flow_cost(text)` : tuple (énergie, power, domaines) pour `Impl.flow`
pub fn flow_cost(_text: &str) -> Value {
    todo!()
}
/// `add_ability(text, exhaust, kill, can)` : entrée d'`Impl.add`
pub fn add_ability(_text: &str, _exhaust: bool, _kill: bool, _can: Value) -> Map {
    todo!()
}
/// `ability(name, cost, timing, exhaust, xp, kill_self, resolve, choices, preds, can, **extra)`
pub fn ability(_name: &str, _a: AbilityArgs) -> Map {
    todo!()
}
/// `equip_ability(domain)` : Equip au coût d'un power de ce domaine.
pub fn equip_ability(_domain: &str) -> Map {
    todo!()
}
/// `equip_ability_cost(cost, timing)` (818)
pub fn equip_ability_cost(_cost: Cost, _timing: &'static str) -> Map {
    todo!()
}
/// `empower_ability(cost, timing, exhaust)` (827)
pub fn empower_ability(_cost: Cost, _timing: &'static str, _exhaust: bool) -> Map {
    todo!()
}
/// `Impl(..., equip=cost)` : ajoute la capacité Equip s'il n'y en a pas.
pub fn impl_equip(_im: &mut Impl, _cost: Cost) {
    todo!()
}
/// `Impl(..., empower=cost)` : la capacité Empower en tête.
pub fn impl_empower(_im: &mut Impl, _cost: Cost) {
    todo!()
}
/// `when_level(n)` : cond(g, o)
pub fn when_level(_n: i64) -> Value {
    todo!()
}
pub fn when_empowered() -> Value {
    todo!()
}
pub fn when_mighty() -> Value {
    todo!()
}
pub fn when_legion() -> Value {
    todo!()
}
pub fn when_at_bf() -> Value {
    todo!()
}
/// `repeatable(fn)` : resolve(g, it) qui rejoue fn sur les cibles 't2' quand le Repeat est payé.
pub fn repeatable(_f: Value) -> Value {
    todo!()
}

// ---------------------------------------------------------------------------------------------- jetons, Disarm
/// `token_locations(g, pid, name, locs)`
pub fn token_locations(_g: &mut Game, _pid: usize, _name: &str, _locs: Option<Vec<Loc>>) -> Res<Vec<Loc>> {
    todo!()
}
/// `make_token(g, name, pid, loc, ready)` : None quand le jeton ne peut pas être joué là.
pub fn make_token(_g: &mut Game, _name: &str, _pid: usize, _loc: Loc, _ready: bool) -> Res<Option<O>> {
    todo!()
}
/// `disarm_trigger(g, u, n)` : [Disarm X] (831).
pub fn disarm_trigger(_g: &mut Game, _u: O, _n: i64) -> Res<()> {
    todo!()
}
/// `pay_deflect(g, it, o)` (809)
pub fn pay_deflect(_g: &mut Game, _it: ItemId, _o: O) -> Res<bool> {
    todo!()
}

// ---------------------------------------------------------------------------------------------- decks, mots-clés
/// `deck_problems(deck)`
pub fn deck_problems(_deck: &DeckArg) -> Vec<String> {
    todo!()
}
/// `keywords_of(name)`
pub fn keywords_of(_name: &str) -> BTreeSet<String> {
    todo!()
}
/// `parse_keywords(name)`
pub fn parse_keywords(_name: &str) -> Option<KwFields> {
    todo!()
}
/// `auto_keywords(name, register)` : register = Some(registre) l'enregistre.
pub fn auto_keywords(_r: Option<&mut Registry>, _name: &str) -> Option<Impl> {
    todo!()
}
/// `NOT_CARDS`
pub const NOT_CARDS: &[&str] = &["Buff", "XP Tracker"];
/// `TOKEN_NAMES`
pub const TOKEN_NAMES: &[&str] = &["Bird", "Sprite", "Gold", "Reflection", "Recruit (NX)", "Mech", "Brush", "Bomb", "Baron Pit"];
/// `is_token(name)`
pub fn is_token(_name: &str) -> bool {
    todo!()
}
/// `register_keyword_only_cards()` : à appeler après les modules de cartes. Renvoie les noms enregistrés.
pub fn register_auto(_r: &mut Registry) -> Vec<String> {
    todo!()
}
/// `ask_text(...)` en tête de cards.py
pub fn register_core_ask_text(_r: &mut Registry) {
    todo!()
}
/// Cartes enregistrées par cards.py hors du pool : runes (Fury, Calm, Mind, Order, Body, Chaos) et « Gold // Buff ».
pub fn register_core_cards(_r: &mut Registry) {
    todo!()
}
