//! Actions discrétionnaires légales (410) et processus de jeu d'une carte ou d'une capacité (353-359, 398-406) :
//! `actions.py`.
//!
//! Actions (tuples Python, `Value::Tuple`) : `('end',)`, `('pass',)`, `('play', uid, src, choice)`,
//! `('hide', uid, bf)` ou `('hide', uid, bf, (e, reqs))`, `('act', src_uid | ('legend', pid), i, choice)`,
//! `('move', (uids...), dest)`. uid et bf en `Value::Int`, src en `Value::Str`, choice en `Value::Map`, dest et
//! `choice['loc']` en `Value::Loc`.
//!
//! Choix modifiés par un coût : en Python, `Impl.pay_extra(g, pid, card, ch)`, `opt_pay(g, src, pid, card, ch)` et
//! `ab['extra_cost'](g, pid, obj, ch)` peuvent écrire dans `ch`. En Rust le choix est passé par valeur : la fonction
//! renvoie `Value::Map(ch modifié)` pour le remplacer (toute autre valeur le laisse tel quel).
use crate::game::{Game, ItemId, Res, O};
use crate::game_costs::Req;
use crate::value::{Loc, Map, Value};

/// `MAX_CHOICES`
pub const MAX_CHOICES: usize = 8;
/// `REPEAT_KEYS`
pub const REPEAT_KEYS: [&str; 5] = ["rep", "reps", "rep2", "sq", "tg2"];

/// `full_choices(g, pid)` : tous les choix légaux doivent être proposés (joueur humain, ou `g.every_choice`).
pub fn full_choices(g: &Game, pid: Option<usize>) -> bool {
    if g.every_choice {
        return true;
    }
    match pid {
        Some(p) => g.agents.get(p).is_some_and(|a| a.every_choice()),
        None => false,
    }
}

/// `card_obj(g, pid, uid, src)`
pub fn card_obj(_g: &Game, _pid: usize, _uid: u32, _src: &str) -> Option<O> {
    todo!()
}
/// `has_timing(g, pid, im, src, closed, loc_ok_ambush)`
pub fn has_timing(_im: &crate::cards::Impl, _src: &str, _closed: bool, _loc_ok_ambush: bool) -> bool {
    todo!()
}
/// `state_neutral_open_main(g, pid)`
pub fn state_neutral_open_main(_g: &Game, _pid: usize) -> bool {
    todo!()
}
/// `flow_of(g, pid, card)` : (énergie, power, domaines) ou None.
pub fn flow_of(_g: &Game, _pid: usize, _card: O) -> Value {
    todo!()
}
/// `card_kw(g, pid, card, kw, src)`
pub fn card_kw(_g: &mut Game, _pid: usize, _card: O, _kw: &str, _src: Option<&str>) -> Res<bool> {
    todo!()
}
/// `eff_timing(g, pid, card, im, src)`
pub fn eff_timing(_g: &mut Game, _pid: usize, _card: O, _im: &crate::cards::Impl, _src: Option<&str>) -> Res<Option<&'static str>> {
    todo!()
}
/// `has_deploy(g, card)`
pub fn has_deploy(_g: &mut Game, _card: O) -> Res<bool> {
    todo!()
}
/// `gear_locations(g, pid, card, hidden_bf)`
pub fn gear_locations(_g: &mut Game, _pid: usize, _card: O, _hidden_bf: Option<Loc>) -> Res<Vec<Loc>> {
    todo!()
}
/// `show_options(g, pid, card, im)` : [dict(show=uid)..., {}]
pub fn show_options(_g: &mut Game, _pid: usize, _card: O, _im: Option<&crate::cards::Impl>) -> Res<Vec<Map>> {
    todo!()
}
/// `shown_info(g, pid, uid)` : dict ou None.
pub fn shown_info(_g: &mut Game, _pid: usize, _uid: u32) -> Res<Value> {
    todo!()
}
/// `repeat_instances(g, pid, card)` : [(clé, e, reqs)]
pub fn repeat_instances(_g: &mut Game, _pid: usize, _card: O) -> Res<Vec<(Value, i64, Vec<Req>)>> {
    todo!()
}
/// `cost_mods(g, pid, what)` (356.3-356.4) ; l'entrée d'un effet porte `_eff` = son eid.
pub fn cost_mods(_g: &mut Game, _pid: usize, _what: &Map) -> Res<Vec<Map>> {
    todo!()
}
/// `_drop_reqs(reqs, n)`
pub fn drop_reqs(_reqs: &[Req], _n: usize) -> Vec<Req> {
    todo!()
}
/// `_flex(variants)`
pub fn flex(_variants: &[(i64, Vec<Req>)]) -> Vec<(i64, Vec<Req>)> {
    todo!()
}
/// `total_cost(g, pid, card, choice, src)` (356) : (énergie, reqs).
pub fn total_cost(_g: &mut Game, _pid: usize, _card: O, _choice: &Map, _src: &str) -> Res<(i64, Vec<Req>)> {
    todo!()
}
/// `deflect_reqs(g, pid, choice)` (809)
pub fn deflect_reqs(_g: &mut Game, _pid: usize, _choice: &Map) -> Res<Vec<Req>> {
    todo!()
}
/// `deflect_total(g, pid, card, choice)`
pub fn deflect_total(_g: &mut Game, _pid: usize, _card: O, _choice: &Map) -> Res<Vec<Req>> {
    todo!()
}
/// `pay_ctx(card)` : dict(kind=, card=)
pub fn pay_ctx(_g: &Game, _card: O) -> Value {
    todo!()
}
/// `affordable(g, pid, card, choice, src)`
pub fn affordable(_g: &mut Game, _pid: usize, _card: O, _choice: &Map, _src: &str) -> Res<bool> {
    todo!()
}
/// `unit_timing_ok(im, closed, timing)` : timing = le timing effectif (`im.timing` par défaut).
pub fn unit_timing_ok(_timing: Option<&str>, _closed: bool) -> bool {
    todo!()
}
/// `loc_allowed(g, pid, card, loc)`
pub fn loc_allowed(_g: &mut Game, _pid: usize, _card: O, _loc: Loc) -> Res<bool> {
    todo!()
}
/// `unit_locations(g, pid, card, src, closed_or_focus, with_extra=True)` : [(loc, choix en plus)]
pub fn unit_locations_extra(_g: &mut Game, _pid: usize, _card: O, _src: &str, _closed_or_focus: bool) -> Res<Vec<(Loc, Map)>> {
    todo!()
}
/// `unit_locations(g, pid, card, src, closed_or_focus)` : les emplacements sans choix en plus.
pub fn unit_locations(_g: &mut Game, _pid: usize, _card: O, _src: &str, _closed_or_focus: bool) -> Res<Vec<Loc>> {
    todo!()
}
/// `open_bf(g, b)` (355.2.b)
pub fn open_bf(_g: &Game, _b: usize) -> bool {
    todo!()
}
/// `play_forbidden(g, pid, card, src)`
pub fn play_forbidden(_g: &mut Game, _pid: usize, _card: O, _src: &str) -> Res<bool> {
    todo!()
}
/// `card_choices(g, pid, card, src, timing_closed, timing_showdown, every)`
pub fn card_choices(
    _g: &mut Game,
    _pid: usize,
    _card: O,
    _src: &str,
    _timing_closed: bool,
    _timing_showdown: bool,
    _every: bool,
) -> Res<Vec<Map>> {
    todo!()
}
/// `spell_variants(g, pid, card, im, base, ctx, every)`
pub fn spell_variants(
    _g: &mut Game,
    _pid: usize,
    _card: O,
    _im: &crate::cards::Impl,
    _base: Vec<Map>,
    _ctx: &Map,
    _every: bool,
) -> Res<Vec<Map>> {
    todo!()
}
/// `playable_cards(g, pid, closed, showdown)` : [(carte, zone)]
pub fn playable_cards(_g: &Game, _pid: usize) -> Vec<(O, &'static str)> {
    todo!()
}
/// `play_options(g, pid, closed, showdown, every)`
pub fn play_options(_g: &mut Game, _pid: usize, _closed: bool, _showdown: bool, _every: bool) -> Res<Vec<Value>> {
    todo!()
}
/// `abilities_of(g, obj)`
pub fn abilities_of(_g: &mut Game, _obj: O) -> Res<Vec<Map>> {
    todo!()
}
/// `ability_options(g, pid, closed, showdown, main, every)`
pub fn ability_options(_g: &mut Game, _pid: usize, _closed: bool, _showdown: bool, _main: bool, _every: bool) -> Res<Vec<Value>> {
    todo!()
}
/// `ability_cost(g, pid, obj, ab, ch)`
pub fn ability_cost(_g: &mut Game, _pid: usize, _obj: O, _ab: &Map, _ch: &Map) -> Res<(i64, Vec<Req>)> {
    todo!()
}
/// `move_options(g, pid)` (144, 420.3)
pub fn move_options(_g: &mut Game, _pid: usize) -> Res<Vec<Value>> {
    todo!()
}
/// `hide_costs(g, pid)` (811.1.b)
pub fn hide_costs(_g: &mut Game, _pid: usize) -> Res<Vec<(i64, Vec<Req>)>> {
    todo!()
}
/// `hide_slots(g, b)`
pub fn hide_slots(_g: &mut Game, _b: usize) -> Res<i64> {
    todo!()
}
/// `hide_options(g, pid)`
pub fn hide_options(_g: &mut Game, _pid: usize) -> Res<Vec<Value>> {
    todo!()
}
/// `main_options(g, pid)`
pub fn main_options(g: &mut Game, pid: usize) -> Res<Vec<Value>> {
    main_options_every(g, pid, false)
}
/// `main_options(g, pid, every)`
pub fn main_options_every(_g: &mut Game, _pid: usize, _every: bool) -> Res<Vec<Value>> {
    todo!()
}
/// `timed_options(g, pid, closed)`
pub fn timed_options(g: &mut Game, pid: usize, closed: bool) -> Res<Vec<Value>> {
    timed_options_every(g, pid, closed, false)
}
/// `timed_options(g, pid, closed, every)`
pub fn timed_options_every(_g: &mut Game, _pid: usize, _closed: bool, _every: bool) -> Res<Vec<Value>> {
    todo!()
}
/// `do_action(g, a)`
pub fn do_action(_g: &mut Game, _a: &Value) -> Res<()> {
    todo!()
}
/// `owner_of(g, uid, src)`
pub fn owner_of(_g: &Game, _uid: u32) -> usize {
    todo!()
}
/// `hide(g, pid, uid, bf, cost)` (421, 811)
pub fn hide(_g: &mut Game, _pid: usize, _uid: u32, _bf: usize, _cost: (i64, Vec<Req>)) -> Res<()> {
    todo!()
}
/// `play_card(g, pid, card, src, ch, limited)` (353) : la carte (unité, équipement), l'item (sort), ou None.
pub fn play_card(_g: &mut Game, _pid: usize, _card: O, _src: &str, _ch: Map, _limited: bool) -> Res<Value> {
    todo!()
}
/// `others_enter_ready(g, pid, card)` (369.3)
pub fn others_enter_ready(_g: &mut Game, _pid: usize, _card: O) -> Res<bool> {
    todo!()
}
/// `enters_ready(g, pid, card, ch)`
pub fn enters_ready(_g: &mut Game, _pid: usize, _card: O, _ch: &Map) -> Res<bool> {
    todo!()
}
/// `keyword_play_triggers(g, pid, card, hidden_bf)` : [Vision] (817), [Weaponmaster] (821).
pub fn keyword_play_triggers(_g: &mut Game, _pid: usize, _card: O, _hidden_bf: Option<Loc>) -> Res<()> {
    todo!()
}
/// `equip_cost(g, pid, gear, unit, discount_any)` : (e, reqs, capacité) ou None.
pub fn equip_cost(_g: &mut Game, _pid: usize, _gear: O, _unit: O, _discount_any: usize) -> Res<Option<(i64, Vec<Req>, Map)>> {
    todo!()
}
/// `weaponmaster_trigger(g, pid, unit)` (821)
pub fn weaponmaster_trigger(_g: &mut Game, _pid: usize, _unit: O) {
    todo!()
}
/// `attach(g, gear, unit)` (434)
pub fn attach(_g: &mut Game, _gear: O, _unit: O) -> Res<()> {
    todo!()
}
/// `activate(g, src, i, ch)` (376) : src = `Value::Int(uid)` ou `('legend', pid)`.
pub fn activate(_g: &mut Game, _src: &Value, _i: usize, _ch: Map) -> Res<()> {
    todo!()
}
/// `fmt_choice(g, ch)` (journal)
pub fn fmt_choice(_g: &Game, _ch: &Map) -> String {
    todo!()
}
/// `repr(v)` de Python (journal, libellés).
pub fn py_repr(_g: &Game, _v: &Value) -> String {
    todo!()
}
/// `str(v)` de Python (f-string).
pub fn py_str(_g: &Game, _v: &Value) -> String {
    todo!()
}
#[allow(dead_code)]
fn _unused(_: ItemId) {}
