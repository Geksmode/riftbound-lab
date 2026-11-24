//! Paquet chaos : les cartes de `batches/chaos.txt` (`riftbound/engine/cardsets/chaos.py`), traduites ligne à ligne.
//! Voir `riftbound/engine/cardsets/GUIDE.md` et `docs/memoire/portage-rust.md`.
//!
//! Conventions d'appel supposées pour les champs de `Impl` lus par `actions` (arguments Python après `g`, dans
//! l'ordre) : `choices(pid, ctx)` -> liste de dict ; `on_play(obj, ctx)` ; `on_event(obj, ev, info)` ;
//! `effect_event(gear, unit, ev, info)` ; `resolve(item)` ; `preds` : liste de `pred(item, obj)` ; capacités :
//! `choices(pid, obj)`, `can(pid, obj)`, `extra_cost(pid, obj, choice)` ; `as_played(pid, card)` -> liste de dict ;
//! `extra_cost_fn(pid, card, choice)` -> tuple (énergie, reqs) ; `enter_ready(pid, card, choice)` ;
//! `cost_mod(pid, card, choice)` -> tuple (énergie, power) ; `opt_parts(pid, card, choice)` -> liste de tuples
//! (clé, énergie, reqs) ; `cost_aura(src, pid, what)` -> liste de dict ; `extra_locs(pid, card, closed)` et
//! `aura_locs(src, pid, card, closed)` -> liste d'emplacements (`Value::Loc`) ; `card_kw(src, pid, card, kw, zone)` ;
//! `enter_loc(pid, card, loc)` -> emplacement. Les reqs rangés dans une valeur suivent `game_costs::reqs_to_value`.
//! La clé de choix `bf` (Bone Skewer, Maduli) est l'indice du battlefield (`Value::Int`), `dest` un `Value::Loc`.
//!
//! Les attributs que le Python pose sur un objet (`o.sarc`, `o.named_tag`) sont rangés dans `Obj.attrs`.
use std::collections::BTreeSet;

use crate::actions::{affordable, attach, deflect_reqs, flow_of, full_choices, open_bf, play_card, total_cost};
use crate::cards::{Impl, Registry};
use crate::cards_helpers::{
    ability, add_ability, cap, enemies, equip_ability_cost, flow_cost, friends, impl_empower, impl_equip, item_by_id,
    make_token, may_pay, p_enemy, p_enemy_bf, p_enemy_bf_v, p_enemy_v, p_friend_bf_v, p_friend_v, p_gear_v, p_unit, p_unit_bf,
    p_unit_bf_v, p_unit_v, pay_deflect, play_unit_free, repeat_cost, repeatable, tg_choices, token_locations,
    trig_target, unit_play_choices, value, when_empowered, AbilityArgs, Cost,
};
use crate::game::{Desig, Game, ItemId, Res, Zone, O};
use crate::game_costs::{any_req, reqs_to_value, Req};
use crate::lang::tr;
use crate::map;
use crate::spec::{spec, BARON_PIT};
use crate::value::{is_bf, Loc, Map, Value};

// ====================================================================== aides privées
/// `Value::func` sans attributs.
fn f(name: &'static str, x: impl Fn(&mut Game, &[Value]) -> Res<Value> + Send + Sync + 'static) -> Value {
    Value::func(name, x)
}

/// liste de dict -> valeur
fn chv(v: Vec<Map>) -> Value {
    Value::List(v.into_iter().map(Value::Map).collect())
}

fn objv(v: &[O]) -> Vec<Value> {
    v.iter().map(|&o| Value::Obj(o)).collect()
}

fn pid_of(v: &Value) -> usize {
    v.int() as usize
}

fn ctrl(g: &Game, it: ItemId) -> usize {
    g.it(it).ctrl
}

fn dget(g: &Game, it: ItemId, k: &str) -> Value {
    g.it(it).data.g(k)
}


/// `(u.uid,)` / `tuple(u.uid for u in us)`
fn uids_t(g: &Game, us: &[O]) -> Value {
    Value::Tuple(us.iter().map(|&u| Value::from(g.o(u).uid)).collect())
}

fn stype(g: &Game, o: O) -> &'static str {
    &g.spec(o).type_
}

/// `frozenset({d})`
fn req1(d: &str) -> Req {
    let mut s = BTreeSet::new();
    s.insert(d.to_string());
    s
}

/// `info["bf"]` (indice) -> emplacement
fn bf_loc(v: &Value) -> Loc {
    Loc::Bf(v.int() as usize)
}

/// `sorted(xs, key=k)` : tri stable sur des clés déjà calculées (dans l'ordre de la liste, comme Python).
fn sort_keys<T, K: PartialOrd>(xs: Vec<T>, keys: Vec<K>) -> Vec<T> {
    let mut z: Vec<(K, T)> = keys.into_iter().zip(xs).collect();
    z.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    z.into_iter().map(|x| x.1).collect()
}

/// `sorted(us, key=lambda u: value(g, u))`
fn by_value(g: &mut Game, us: Vec<O>) -> Res<Vec<O>> {
    let mut ks = vec![];
    for &u in &us {
        ks.push(value(g, u)?);
    }
    Ok(sort_keys(us, ks))
}

/// `sorted(us, key=lambda u: -value(g, u))`
fn by_value_desc(g: &mut Game, us: Vec<O>) -> Res<Vec<O>> {
    let mut ks = vec![];
    for &u in &us {
        ks.push(-value(g, u)?);
    }
    Ok(sort_keys(us, ks))
}

/// `sorted(cs, key=lambda c: -_cval(c))`
fn by_cval_desc(g: &Game, cs: Vec<O>) -> Vec<O> {
    let ks: Vec<f64> = cs.iter().map(|&c| -cval(g, c)).collect();
    sort_keys(cs, ks)
}

/// `itertools.combinations(v, k)` (ordre lexicographique des positions)
fn combinations<T: Clone>(v: &[T], k: usize) -> Vec<Vec<T>> {
    let n = v.len();
    let mut out = vec![];
    if k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.iter().map(|&i| v[i].clone()).collect());
        let mut i = k;
        loop {
            if i == 0 {
                return out;
            }
            i -= 1;
            if idx[i] != i + n - k {
                break;
            }
        }
        idx[i] += 1;
        for j in i + 1..k {
            idx[j] = idx[j - 1] + 1;
        }
    }
}

/// `enemies(g, pid)` (valeurs par défaut de Python)
fn en(g: &mut Game, pid: usize) -> Res<Vec<O>> {
    enemies(g, pid, false, None, true)
}

/// `enemies(g, pid, True)`
fn en_bf(g: &mut Game, pid: usize) -> Res<Vec<O>> {
    enemies(g, pid, true, None, true)
}

/// `friends(g, pid)`
fn fr(g: &mut Game, pid: usize) -> Res<Vec<O>> {
    friends(g, pid, false, None)
}

/// `friends(g, pid, True)`
fn fr_bf(g: &mut Game, pid: usize) -> Res<Vec<O>> {
    friends(g, pid, true, None)
}

/// `_q(g, o, name, fn, data, **kw)` : met en attente une capacité déclenchée du permanent o (son incarnation est
/// retenue, voir `me`).
#[allow(clippy::too_many_arguments)]
fn q(g: &mut Game, o: O, name: &str, fun: Value, data: Map, may: bool, choose: Value, cost: Value) {
    let mut d = data;
    d.set("_oid", g.o(o).oid);
    let (c, u) = (g.o(o).ctrl, g.o(o).uid);
    g.queue_trigger(c, name, fun, d, Some(u), may, choose, cost);
}

/// `_q(g, o, name, fn)`
fn q0(g: &mut Game, o: O, name: &str, fun: Value) {
    q(g, o, name, fun, Map::new(), false, Value::None, Value::None);
}

/// `_me(g, it)` : le permanent source d'un déclenchement s'il est toujours le même objet sur le plateau
/// (règle 359.3.e.4).
fn me(g: &Game, it: ItemId) -> Option<O> {
    let o = g.obj(g.it(it).src?)?;
    let oid = g.o(o).oid;
    let want = match g.it(it).data.get("_oid") {
        Some(v) => v.int(),
        None => oid,
    };
    if oid != want {
        return None;
    }
    Some(o)
}

/// `_loc_now(g, it, here)` : position actuelle de la source d'un déclenchement (relue par uid) ; à défaut, sa
/// position quand il a été mis en attente.
fn loc_now(g: &Game, it: ItemId, here: Option<Loc>) -> Option<Loc> {
    match g.it(it).src.and_then(|u| g.obj(u)) {
        Some(o) => g.o(o).loc,
        None => here,
    }
}

/// `_same(g, uid, oid)`
fn same(g: &Game, u: &Value, oid: &Value) -> Option<O> {
    let o = g.obj(u.int() as u32)?;
    if g.o(o).oid == oid.int() {
        Some(o)
    } else {
        None
    }
}

/// `_cval(c)`
fn cval(g: &Game, c: O) -> f64 {
    let s = g.spec(c);
    s.e as f64 + 2.0 * s.p as f64 + s.might.unwrap_or(0) as f64 / 2.0
}

/// `_discard(g, pid, n)` : défausser n (règle 422) : le joueur choisit ; autant que possible (359.3.e.11).
fn discard_n(g: &mut Game, pid: usize, n: i64) -> Res<()> {
    for _ in 0..n {
        let h = g.p[pid].hand.clone();
        if h.is_empty() {
            return Ok(());
        }
        let c = g.ask(pid, "discard", objv(&h), Map::new())?;
        g.discard(pid, c.obj())?;
    }
    Ok(())
}

/// `_to_hand(g, o)`
fn to_hand(g: &mut Game, o: Option<O>) -> Res<bool> {
    if let Some(o) = o {
        if g.board.contains(&o) {
            g.to_zone(o, "hand", false, None)?;
            return Ok(true);
        }
    }
    Ok(false)
}

/// `_token(g, pid, name, ready)` : « Play a ... token » : un jeton unité se joue comme une unité (185.2.a, 355.2.a),
/// un équipement à la base.
fn token(g: &mut Game, pid: usize, name: &str, ready: bool) -> Res<Option<O>> {
    if spec(name).type_ == "Gear" {
        return make_token(g, name, pid, Loc::Base, ready);
    }
    let ls = token_locations(g, pid, name, None)?;
    if ls.is_empty() {
        return Ok(None);
    }
    let lv: Vec<Value> = ls.iter().map(|&l| Value::Loc(l)).collect();
    let loc = g.ask(pid, "token_location", lv, map! { token = name })?;
    make_token(g, name, pid, loc.loc(), ready)
}

/// `_gold(g, pid)` : « play a Gold gear token exhausted »
fn gold(g: &mut Game, pid: usize) -> Res<Option<O>> {
    make_token(g, "Gold", pid, Loc::Base, false)
}

/// resolve « play a Gold gear token exhausted » (lambda g_, it: _gold(g_, it.ctrl))
fn gold_res() -> Value {
    f("_gold", |g, a| {
        let c = ctrl(g, a[0].item());
        gold(g, c)?;
        Ok(Value::None)
    })
}

/// resolve « draw 1 » (lambda g_, it: g_.draw(it.ctrl, 1))
fn draw1_res() -> Value {
    f("_draw1", |g, a| {
        let c = ctrl(g, a[0].item());
        g.draw(c, 1)?;
        Ok(Value::None)
    })
}

/// resolve « discard 1, then draw 1 »
fn loot_res() -> Value {
    f("_loot", |g, a| {
        let c = ctrl(g, a[0].item());
        discard_n(g, c, 1)?;
        g.draw(c, 1)?;
        Ok(Value::None)
    })
}

/// `_ensure_effect(g, key, on, fn)` : installe une fois un écouteur pour toute la partie.
fn ensure_effect(g: &mut Game, key: &str, on: &str, fun: Value) {
    if !g.effects.iter().any(|e| e.d.g("key").opt_str() == Some(key)) {
        let mut d = Map::new();
        d.set("on", on);
        d.set("fn", fun);
        d.set("key", key);
        g.add_effect(d);
    }
}

/// `_from_facedown(info)` : évènement 'played' : la carte a-t-elle été jouée depuis la face cachée ?
fn from_facedown(g: &Game, info: &Map) -> bool {
    if let Some(it) = info.get("item").filter(|v| !v.is_none()) {
        return g.it(it.item()).data.g("from").opt_str() == Some("facedown");
    }
    g.o(info.g("card").obj()).played_from.as_deref() == Some("facedown")
}

/// `_dests(g, u)`
fn dests(g: &Game, u: O) -> Vec<Loc> {
    g.locs().into_iter().filter(|&d| Some(d) != g.o(u).loc).collect()
}

/// `_deflect_ok(g, pid, u)`
fn deflect_ok(g: &mut Game, pid: usize, u: O) -> Res<bool> {
    let ch = map! { tg = uids_t(g, &[u]) };
    let reqs = deflect_reqs(g, pid, &ch)?;
    g.can_pay(pid, 0, &reqs, &Value::None)
}

/// `_pay_options(g, pid, c, src)` : choix de pose d'une unité jouée par un effet qui paie encore ses coûts.
fn pay_options(g: &mut Game, pid: usize, c: O, src: &str) -> Res<Vec<Map>> {
    let mut out = vec![];
    for ch in unit_play_choices(g, pid, c, src, None)? {
        if affordable(g, pid, c, &ch, src)? {
            out.push(ch);
        }
    }
    Ok(out)
}

/// `_play_paying(g, pid, c, src)`
fn play_paying(g: &mut Game, pid: usize, c: O, src: &str) -> Res<Value> {
    let opts = pay_options(g, pid, c, src)?;
    if opts.is_empty() {
        return Ok(Value::None);
    }
    let ch = g.ask(pid, "play_choice", opts.into_iter().map(Value::Map).collect(), map! { card = c })?;
    play_card(g, pid, c, src, ch.map().clone(), false) // processus normal : ses coûts sont payés (353)
}

/// `_power_ok(g, pid, c)`
fn power_ok(g: &mut Game, pid: usize, c: O) -> Res<bool> {
    let s = g.spec(c);
    let reqs = g.power_reqs(&s.domains, s.p as usize);
    g.can_pay(pid, 0, &reqs, &Value::None)
}

/// `_trash_units(g, pid, fn)`
fn trash_units(g: &Game, pid: usize, pred: impl Fn(&Game, O) -> bool) -> Vec<O> {
    let cs: Vec<O> = g.p[pid]
        .trash
        .iter()
        .copied()
        .filter(|&c| stype(g, c) == "Unit" && g.impl_o(c).is_some() && pred(g, c))
        .collect();
    by_cval_desc(g, cs)
}

/// `_return_from_trash(g, pid, typ, kind)`
fn return_from_trash(g: &mut Game, pid: usize, typ: &str, kind: &str) -> Res<()> {
    let cs: Vec<O> = g.p[pid].trash.iter().copied().filter(|&c| stype(g, c) == typ).collect();
    let cs = by_cval_desc(g, cs);
    let c = g.ask(pid, kind, objv(&cs), Map::new())?;
    if let Some(c) = c.opt_obj() {
        g.to_zone(c, "hand", false, None)?;
    }
    Ok(())
}

/// `_buff_me(n)` : lambda g, it: _me(g, it) is not None and g.mod(_me(g, it), n)
fn buff_me(n: i64) -> Value {
    f("_buff_me", move |g, a| {
        if let Some(m) = me(g, a[0].item()) {
            g.mod_(m, n, "turn".into(), None)?;
        }
        Ok(Value::None)
    })
}

/// `_won_combat(g, o, info)` : évènement 'combat_won' : o était dans ce combat et son contrôleur l'a gagné.
fn won_combat(g: &Game, o: O, info: &Map) -> bool {
    info.g("pid").int() as usize == g.o(o).ctrl && g.sd.as_ref().is_some_and(|sd| sd.members.contains(&g.o(o).uid))
}

/// arguments d'un `on_event(g, o, ev, info)`
fn ev_args(a: &[Value]) -> (O, &str, &Map) {
    (a[0].obj(), a[1].str(), a[2].map())
}

/// `info["obj"] is o`
fn is_obj(info: &Map, o: O) -> bool {
    info.g("obj").opt_obj() == Some(o)
}

/// `o in info["units"]`
fn in_units(info: &Map, o: O) -> bool {
    info.g("units").list().iter().any(|v| v.opt_obj() == Some(o))
}

/// `g.legal(it, i)` pour chaque cible
fn all_legal(g: &mut Game, it: ItemId) -> Res<Vec<Option<O>>> {
    let n = g.it(it).targets.len();
    let mut out = vec![];
    for i in 0..n {
        out.push(g.legal(it, i)?);
    }
    Ok(out)
}

/// `g.move([u], dest, pid)`
fn mv(g: &mut Game, u: O, dest: Loc, pid: usize) -> Res<()> {
    g.move_(&[u], dest, Some(pid), false)?;
    Ok(())
}

/// `g.stats[key]` (clé tuple de Python : ses éléments joints par « _ »)
fn stat_key(name: &str, g: &Game, o: O) -> String {
    format!("{name}_{}_{}_{}", g.o(o).uid, g.o(o).oid, g.turn_no)
}

/// spell choices -> Value
fn tgv(g: &Game, us: &[O]) -> Value {
    chv(tg_choices(g, us))
}

/// un choix vide `dict(k=())`
fn empty_choice(k: &str) -> Map {
    let mut m = Map::new();
    m.set(k, Value::Tuple(vec![]));
    m
}

fn new_impl(name: &str) -> Impl {
    Impl { module: Some("chaos"), ..Impl::new(name) }
}

fn kw1(k: &str) -> Map {
    let mut m = Map::new();
    m.set(k, 1);
    m
}

// ====================================================================== A
// Abandon — "[Reaction] Counter a spell. Return it to its owner's hand instead of putting it in their trash.
// [Predict]."
fn item_of(g: &Game, v: &Value) -> Option<ItemId> {
    v.opt_int().and_then(|i| item_by_id(g, i))
}

fn abandon(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let tgt = item_of(g, &dget(g, it, "item"));
    if let Some(t) = tgt {
        if g.it(t).kind == "spell" {
            let c = g.it(t).card.unwrap();
            // un sort Flow contré est banni, pas mis à la défausse
            if g.counter(t)? && g.o(c).zone == Some(Zone::Trash) {
                g.to_zone(c, "hand", false, None)?;
            }
        }
    }
    let c = ctrl(g, it);
    g.predict(c, 1)?;
    Ok(Value::None)
}

/// `_counter_choices(g, pid, ctx)`
fn counter_choices_v(g: &mut Game, pid: usize) -> Res<Vec<Map>> {
    let mut its = vec![];
    for i in g.chain.clone().into_iter().rev() {
        if g.it(i).kind == "spell" && g.counterable(i)? {
            its.push(i);
        }
    }
    let ks: Vec<bool> = its.iter().map(|&i| g.it(i).ctrl == pid).collect();
    let its = sort_keys(its, ks);
    Ok(its.iter().map(|&i| map! { item = g.it(i).id }).collect())
}

fn counter_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    Ok(chv(counter_choices_v(g, pid_of(&a[0]))?))
}

fn c_abandon() -> Impl {
    Impl {
        timing: Some("reaction"),
        resolve: f("_abandon", abandon),
        choices: f("_counter_choices", counter_choices),
        ..new_impl("Abandon")
    }
}

// Acceptable Losses — "[Action] Each player kills one of their gear."
fn acceptable(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let mut victims = vec![];
    for pid in [g.tp, 1 - g.tp] {
        let gs = g.gear(Some(pid));
        if !gs.is_empty() {
            let ks: Vec<(bool, i32)> = gs.iter().map(|&x| (!g.o(x).token, g.spec(x).e)).collect();
            let gs = sort_keys(gs, ks);
            victims.push(g.ask(pid, "sacrifice_gear", objv(&gs), Map::new())?.obj());
        }
    }
    let c = ctrl(g, it);
    g.kill(&victims, Some(c), false)?;
    Ok(Value::None)
}

fn c_acceptable_losses() -> Impl {
    Impl { timing: Some("action"), resolve: f("_acceptable", acceptable), ..new_impl("Acceptable Losses") }
}

// Ancient Warmonger — "[Accelerate] I have [Assault] equal to the number of enemy units here."
fn c_ancient_warmonger() -> Impl {
    Impl {
        accelerate: true,
        aura_kw: f("_warmonger_kw", |g, a| {
            let (src, o) = (a[0].opt_obj(), a[1].obj());
            if src == Some(o) && is_bf(g.o(o).loc) {
                let n = g.units(Some(1 - g.o(o).ctrl), g.o(o).loc).len();
                let mut m = Map::new();
                m.set("Assault", n);
                return Ok(Value::Map(m));
            }
            Ok(Value::None)
        }),
        ..new_impl("Ancient Warmonger")
    }
}

// Angler Beast — "When you play me, return all units with 2 might or less to their owners' hands."
fn c_angler_beast() -> Impl {
    Impl {
        on_play: f("_angler", |g, a| {
            let res = f("_angler_res", |g_, _a| {
                let mut us = vec![];
                for u in g_.units(None, None) {
                    if g_.might(u)? <= 2 {
                        us.push(u);
                    }
                }
                for u in us {
                    to_hand(g_, Some(u))?;
                }
                Ok(Value::None)
            });
            q0(g, a[0].obj(), "Angler Beast", res);
            Ok(Value::None)
        }),
        ..new_impl("Angler Beast")
    }
}

// Annie, Stubborn — "When you play me, return a spell from your trash to your hand."
fn c_annie_stubborn() -> Impl {
    Impl {
        on_play: f("_annie", |g, a| {
            let res = f("_annie_res", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                return_from_trash(g_, c, "Spell", "trash_spell")?;
                Ok(Value::None)
            });
            q0(g, a[0].obj(), "Annie, Stubborn", res);
            Ok(Value::None)
        }),
        ..new_impl("Annie, Stubborn")
    }
}

// ====================================================================== B
// Beast Below — "When you play me, return another friendly unit and an enemy unit to their owners' hands."
fn beast_choose(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c = ctrl(g, it);
    let src = g.it(it).src;
    let fr0: Vec<O> = g.units(Some(c), None).into_iter().filter(|&u| Some(g.o(u).uid) != src).collect();
    let frs = by_value(g, fr0)?;
    let mut ens = vec![];
    for u in en(g, c)? {
        if deflect_ok(g, c, u)? {
            ens.push(u);
        }
    }
    if frs.is_empty() && ens.is_empty() {
        return Ok(false.into());
    }
    if !frs.is_empty() {
        let x = g.ask(c, "friend_target", objv(&frs), map! { item = it })?.obj();
        g.add_target(
            it,
            x,
            f("_beast_pred_friend", |g_, a_| {
                let (i, x) = (a_[0].item(), a_[1].obj());
                Ok((g_.o(x).ctrl == g_.it(i).ctrl && Some(g_.o(x).uid) != g_.it(i).src).into())
            }),
        );
    } else {
        g.it_mut(it).targets.push((None, -1, Value::None));
    }
    if !ens.is_empty() {
        let e = g.ask(c, "target", objv(&ens), map! { item = it })?.obj();
        if !pay_deflect(g, it, e)? {
            return Ok(false.into());
        }
        g.add_target(
            it,
            e,
            f("_beast_pred_enemy", |g_, a_| {
                let (i, x) = (a_[0].item(), a_[1].obj());
                Ok((g_.o(x).ctrl != g_.it(i).ctrl).into())
            }),
        );
    }
    Ok(true.into())
}

fn c_beast_below() -> Impl {
    Impl {
        on_play: f("_beast", |g, a| {
            let res = f("_beast_res", |g_, a_| {
                for u in all_legal(g_, a_[0].item())? {
                    to_hand(g_, u)?;
                }
                Ok(Value::None)
            });
            q(g, a[0].obj(), "Beast Below", res, Map::new(), false, f("_beast_choose", beast_choose), Value::None);
            Ok(Value::None)
        }),
        ..new_impl("Beast Below")
    }
}

// Bewitching Spirit — "When you play me, choose a player. They discard 1."
fn c_bewitching_spirit() -> Impl {
    Impl {
        on_play: f("_bewitching_play", |g, a| {
            let res = f("_bewitching", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                let p = g_.ask(c, "choose_player", vec![(1 - c).into(), c.into()], Map::new())?;
                discard_n(g_, pid_of(&p), 1)?;
                Ok(Value::None)
            });
            q0(g, a[0].obj(), "Bewitching Spirit", res);
            Ok(Value::None)
        }),
        ..new_impl("Bewitching Spirit")
    }
}

// Black Market Broker — "When you play a card from face down, play a Gold gear token exhausted."
fn c_black_market_broker() -> Impl {
    Impl {
        on_event: f("_broker", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "played" && pid_of(&info.g("pid")) == g.o(o).ctrl && from_facedown(g, info) {
                q0(g, o, "Black Market Broker", gold_res());
            }
            Ok(Value::None)
        }),
        ..new_impl("Black Market Broker")
    }
}

// Blast Cone — "When you play this, you may move an enemy unit. When you move an enemy unit, you may exhaust this
// to [Stun] it."
fn cone_play(g: &mut Game, a: &[Value]) -> Res<Value> {
    let o = a[0].obj();
    let choose = f("_cone_choose", |g_, a_| {
        let it = a_[0].item();
        let c = ctrl(g_, it);
        let mut opts: Vec<(O, Loc)> = vec![];
        for u in en(g_, c)? {
            if deflect_ok(g_, c, u)? {
                for d in dests(g_, u) {
                    opts.push((u, d));
                }
            }
        }
        if opts.is_empty() {
            return Ok(false.into());
        }
        let mut ks = vec![];
        for &(u, d) in &opts {
            ks.push((d != Loc::Base, -value(g_, u)?));
        }
        let opts = sort_keys(opts, ks);
        let ov: Vec<Value> = opts.iter().map(|&(u, d)| Value::Tuple(vec![u.into(), d.into()])).collect();
        let pick = g_.ask(c, "move_enemy", ov, map! { item = it })?;
        let (u, d) = (pick.list()[0].obj(), pick.list()[1].loc());
        if !pay_deflect(g_, it, u)? {
            return Ok(false.into());
        }
        g_.add_target(it, u, p_enemy_v());
        g_.it_mut(it).data.set("dest", d);
        Ok(true.into())
    });
    let res = f("_cone_res", |g_, a_| {
        let it = a_[0].item();
        let u = g_.legal(it, 0)?;
        let dest = dget(g_, it, "dest").loc();
        if let Some(u) = u {
            if g_.o(u).loc != Some(dest) {
                let c = ctrl(g_, it);
                mv(g_, u, dest, c)?;
            }
        }
        Ok(Value::None)
    });
    q(g, o, "Blast Cone", res, Map::new(), true, choose, Value::None);
    Ok(Value::None)
}

fn cone_event(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = ev_args(a);
    if ev == "move"
        && info.g("by").opt_int() == Some(g.o(o).ctrl as i64)
        && g.o(info.g("obj").obj()).ctrl != g.o(o).ctrl
        && !g.exhausted(o)
    {
        let u = info.g("obj").obj();
        let cost = f("_cone_cost", |g_, a_| {
            let Some(m) = me(g_, a_[0].item()) else { return Ok(false.into()) };
            if g_.exhausted(m) {
                return Ok(false.into());
            }
            g_.set_exhausted(m, true);
            Ok(true.into())
        });
        let res = f("_cone_stun", |g_, a_| {
            let it = a_[0].item();
            if let Some(x) = same(g_, &dget(g_, it, "uid"), &dget(g_, it, "oid")) {
                let c = ctrl(g_, it);
                g_.stun(Some(x), Some(c))?;
            }
            Ok(Value::None)
        });
        let data = map! { uid = g.o(u).uid, oid = g.o(u).oid };
        q(g, o, "Blast Cone (stun)", res, data, true, Value::None, cost);
    }
    Ok(Value::None)
}

fn c_blast_cone() -> Impl {
    Impl { on_play: f("_cone_play", cone_play), on_event: f("_cone_event", cone_event), ..new_impl("Blast Cone") }
}

// Bone Skewer — "[Hidden] Choose a battlefield. An opponent reveals their hand. You may choose a unit from it. They
// play that unit to that battlefield, ignoring any and all costs. When they do, [Stun] it."
fn skewer(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let bf = bf_loc(&dget(g, it, "bf"));
    let c0 = ctrl(g, it);
    let opp = 1 - c0;
    g.reveal_hand(opp, c0)?;
    let us0: Vec<O> =
        g.p[opp].hand.iter().copied().filter(|&c| stype(g, c) == "Unit" && g.impl_o(c).is_some()).collect();
    let ks: Vec<i32> = us0.iter().map(|&c| g.spec(c).might.unwrap_or(0)).collect();
    let us = sort_keys(us0, ks);
    let c = if !us.is_empty() {
        let mut ov = objv(&us);
        ov.push(Value::None);
        g.ask(c0, "skewer_pick", ov, Map::new())?.opt_obj()
    } else {
        None
    };
    let Some(c) = c else { return Ok(Value::None) };
    let u = play_card(g, opp, c, "hand", map! { loc = bf, free = true }, true)?;
    if let Some(u) = u.opt_obj() {
        if g.board.contains(&u) {
            let stun = f("_skewer_stun", |g_, a_| {
                let i = a_[0].item();
                if let Some(x) = same(g_, &dget(g_, i, "uid"), &dget(g_, i, "oid")) {
                    let c = ctrl(g_, i);
                    g_.stun(Some(x), Some(c))?;
                }
                Ok(Value::None)
            });
            let data = map! { uid = g.o(u).uid, oid = g.o(u).oid };
            g.queue_trigger(c0, "Bone Skewer (stun)", stun, data, None, false, Value::None, Value::None);
        }
    }
    Ok(Value::None)
}

fn skewer_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, ctx) = (pid_of(&a[0]), a[1].map());
    let hb = ctx.g("hidden_bf").opt_loc();
    // règle 811.1.d.2-3 : depuis Hidden, ce battlefield
    let bfs: Vec<Loc> = match hb {
        Some(h) => vec![h],
        None => g.bf_locs(),
    };
    let mut ks = vec![];
    for &i in &bfs {
        let mut s = 0;
        for u in g.units(Some(pid), Some(i)) {
            s += g.might(u)?;
        }
        ks.push(-s);
    }
    let bfs = sort_keys(bfs, ks);
    Ok(chv(bfs.iter().map(|&i| map! { bf = i.bf().unwrap() }).collect()))
}

fn c_bone_skewer() -> Impl {
    Impl {
        hidden: true,
        resolve: f("_skewer", skewer),
        choices: f("_skewer_choices", skewer_choices),
        ..new_impl("Bone Skewer")
    }
}

// Boots of Swiftness — "[Equip] [Chaos]" ; card image: Might Bonus +2, Effect Text "[Ganking]".
fn c_boots_of_swiftness() -> Impl {
    let mut im = Impl { bonus: Some(2), equip_kw: Some(kw1("Ganking")), ..new_impl("Boots of Swiftness") };
    impl_equip(&mut im, Cost::Text("1 chaos rune".into()));
    im
}

// ====================================================================== C
// Called Shot — "[Action] [Repeat] [Chaos] Look at the top 2 cards of your Main Deck. Draw one and recycle the
// other."
fn called_shot(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let pid = ctrl(g, it);
    let d2: Vec<O> = g.p[pid].deck.iter().take(2).copied().collect();
    let top = g.look(pid, &d2, None)?; // regarder ne fait jamais burn out (431.1.c)
    if top.is_empty() {
        return Ok(Value::None);
    }
    let st = by_cval_desc(g, top.clone());
    let c = g.ask(pid, "draw_pick", objv(&st), Map::new())?.obj();
    g.draw_card(pid, c)?;
    let rest: Vec<O> = top.into_iter().filter(|&x| x != c).collect();
    g.recycle_cards(pid, &rest)?;
    Ok(Value::None)
}

fn c_called_shot() -> Impl {
    Impl {
        timing: Some("action"),
        repeat: repeat_cost("1 chaos rune"),
        resolve: repeatable(f("_called_shot", called_shot)),
        ..new_impl("Called Shot")
    }
}

// Cemetery Attendant — "When you play me, return a unit from your trash to your hand."
fn c_cemetery_attendant() -> Impl {
    Impl {
        on_play: f("_cemetery", |g, a| {
            let res = f("_cemetery_res", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                return_from_trash(g_, c, "Unit", "trash_unit")?;
                Ok(Value::None)
            });
            q0(g, a[0].obj(), "Cemetery Attendant", res);
            Ok(Value::None)
        }),
        ..new_impl("Cemetery Attendant")
    }
}

// Conscription — "You may spend 5 XP as an additional cost to play this. Choose an enemy unit at a battlefield with
// 3 might or less. If you paid the additional cost, choose any enemy unit at a battlefield instead. Take control of
// it, exhaust it, and recall it."
fn consc_pred(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (it, o) = (a[0].item(), a[1].obj());
    if !p_enemy_bf(g, it, o) {
        return Ok(false.into());
    }
    Ok((dget(g, it, "xp").truthy() || g.might(o)? <= 3).into())
}

fn consc_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = pid_of(&a[0]);
    let es = en_bf(g, pid)?;
    let mut out = vec![];
    for &u in &es {
        if g.might(u)? <= 3 {
            out.push(map! { tg = uids_t(g, &[u]) });
        }
    }
    if g.p[pid].xp >= 5 {
        let mut big = vec![];
        for &u in &es {
            if g.might(u)? > 3 {
                big.push(map! { tg = uids_t(g, &[u]), xp = 5 });
            }
        }
        big.extend(out);
        out = big;
    }
    Ok(chv(out))
}

/// `_take_control(g, pid, u)` : prendre le contrôle d'une unité (188) et la rappeler (455 : pas un déplacement).
fn take_control(g: &mut Game, pid: usize, u: O) {
    g.o_mut(u).ctrl = pid;
    g.recall(u);
    g.need_cleanup = true;
}

fn c_conscription() -> Impl {
    Impl {
        preds: Value::List(vec![f("_consc_pred", consc_pred)]),
        resolve: f("_conscription", |g, a| {
            let it = a[0].item();
            if let Some(u) = g.legal(it, 0)? {
                g.set_exhausted(u, true);
                let c = ctrl(g, it);
                take_control(g, c, u);
            }
            Ok(Value::None)
        }),
        choices: f("_consc_choices", consc_choices),
        ..new_impl("Conscription")
    }
}

// Corrupt Enforcer — "When I move to a battlefield, discard 1. When I win a combat, draw 1."
fn c_corrupt_enforcer() -> Impl {
    Impl {
        on_event: f("_enforcer", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "move" && is_obj(info, o) && is_bf(info.g("to").opt_loc()) {
                let res = f("_enforcer_discard", |g_, a_| {
                    let c = ctrl(g_, a_[0].item());
                    discard_n(g_, c, 1)?;
                    Ok(Value::None)
                });
                q0(g, o, "Corrupt Enforcer", res);
            } else if ev == "combat_won" && won_combat(g, o, info) {
                q0(g, o, "Corrupt Enforcer (win)", draw1_res());
            }
            Ok(Value::None)
        }),
        ..new_impl("Corrupt Enforcer")
    }
}

// Crescent Guardian — "If you've played a spell this turn, you may pay [Chaos] as an additional cost to play me. If
// you do, I enter ready."  ("played" in a non-triggered check = finalized, rule 419.4.b)
fn c_crescent_guardian() -> Impl {
    Impl {
        as_played: f("_cg_as_played", |g, a| {
            let pid = pid_of(&a[0]);
            let mut out = vec![Map::new()];
            if g.spells_played[pid] != 0 {
                out.push(map! { cg = true });
            }
            Ok(chv(out))
        }),
        extra_cost_fn: f("_cg_extra", |_g, a| {
            let ch = a[2].map();
            if ch.g("cg").truthy() {
                Ok(Value::Tuple(vec![0.into(), reqs_to_value(&[req1("Chaos")])]))
            } else {
                Ok(Value::Tuple(vec![0.into(), reqs_to_value(&[])]))
            }
        }),
        enter_ready: f("_cg_ready", |_g, a| Ok(a[2].map().g("cg").truthy().into())),
        ..new_impl("Crescent Guardian")
    }
}

// Cull — "[Equip] [Chaos]" ; card image: Might Bonus +1, Effect Text "When I conquer, play a Gold gear token
// exhausted."
fn c_cull() -> Impl {
    let mut im = Impl {
        bonus: Some(1),
        effect_event: f("_cull_effect", |g, a| {
            let (unit, ev, info) = (a[1].obj(), a[2].str(), a[3].map());
            if ev == "conquer" && pid_of(&info.g("pid")) == g.o(unit).ctrl && in_units(info, unit) {
                let (c, u) = (g.o(unit).ctrl, g.o(unit).uid);
                g.queue_trigger(c, "Cull", gold_res(), Map::new(), Some(u), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..new_impl("Cull")
    };
    impl_equip(&mut im, Cost::Text("1 chaos rune".into()));
    im
}

// Cursed Sarcophagus — "When you play this, banish all units from your trash. [E]: Play a unit banished with this.
// (You must pay its costs.)"  (rule 427.3: cards banished by the same object)
// L'attribut Python `me.sarc = (oid, [uids])` est rangé dans `attrs["sarc"]`.
fn sarc_play(g: &mut Game, a: &[Value]) -> Res<Value> {
    let res = f("_sarc_play_res", |g_, a_| {
        let it = a_[0].item();
        let c0 = ctrl(g_, it);
        let us: Vec<O> = g_.p[c0].trash.iter().copied().filter(|&c| stype(g_, c) == "Unit").collect();
        for &c in &us {
            g_.to_zone(c, "banish", false, None)?;
        }
        if let Some(m) = me(g_, it) {
            let uids = Value::List(us.iter().map(|&c| Value::from(g_.o(c).uid)).collect());
            let v = Value::Tuple(vec![g_.o(m).oid.into(), uids]);
            g_.o_mut(m).attrs.set("sarc", v);
        }
        Ok(Value::None)
    });
    q0(g, a[0].obj(), "Cursed Sarcophagus", res);
    Ok(Value::None)
}

/// `_sarc_cards(g, o)`
fn sarc_cards(g: &Game, o: O) -> Vec<O> {
    let s = g.o(o).attrs.g("sarc");
    if !s.truthy() || s.list()[0] != Value::from(g.o(o).oid) {
        return vec![];
    }
    let ids = s.list()[1].list().to_vec();
    g.p.iter().flat_map(|pl| pl.banish.iter().copied()).filter(|&c| ids.contains(&Value::from(g.o(c).uid))).collect()
}

fn c_cursed_sarcophagus() -> Impl {
    let ab = ability(
        "Unearth",
        AbilityArgs {
            exhaust: true,
            choices: f("_sarc_choices", |g, a| {
                let (pid, o) = (pid_of(&a[0]), a[1].obj());
                let cs = by_cval_desc(g, sarc_cards(g, o));
                let mut out = vec![];
                for c in cs {
                    if !pay_options(g, pid, c, "banish")?.is_empty() {
                        out.push(map! { c = g.o(c).uid });
                    }
                }
                Ok(chv(out))
            }),
            resolve: f("_sarc_res", |g, a| {
                let it = a[0].item();
                let o = g.it(it).src.and_then(|u| g.obj(u));
                let want = dget(g, it, "c");
                let c = g.p.iter().flat_map(|pl| pl.banish.iter().copied()).find(|&x| Value::from(g.o(x).uid) == want);
                if let Some(c) = c {
                    if o.is_none() || sarc_cards(g, o.unwrap()).contains(&c) {
                        let c0 = ctrl(g, it);
                        play_paying(g, c0, c, "banish")?;
                    }
                }
                Ok(Value::None)
            }),
            ..Default::default()
        },
    );
    Impl { on_play: f("_sarc_play", sarc_play), abilities: vec![ab], ..new_impl("Cursed Sarcophagus") }
}

// ====================================================================== D
// Decree of Discord — "Return any number of enemy Order units with total Might 5 or less to their owners' hands."
fn order_enemy(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (it, o) = (a[0].item(), a[1].obj());
    Ok((p_enemy(g, it, o) && g.spec(o).domains.iter().any(|d| d == "Order")).into())
}

fn sum_might(g: &mut Game, us: &[O]) -> Res<i64> {
    let mut s = 0;
    for &u in us {
        s += g.might(u)?;
    }
    Ok(s)
}

fn sum_value(g: &mut Game, us: &[O]) -> Res<f64> {
    let mut s = 0.0;
    for &u in us {
        s += value(g, u)?;
    }
    Ok(s)
}

fn dod_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = pid_of(&a[0]);
    let es0: Vec<O> = en(g, pid)?.into_iter().filter(|&u| g.spec(u).domains.iter().any(|d| d == "Order")).collect();
    let es = cap(g, es0, 6, Some(pid));
    let mut out = vec![];
    for k in (1..=es.len()).rev() {
        for comb in combinations(&es, k) {
            if sum_might(g, &comb)? <= 5 {
                out.push(comb);
            }
        }
    }
    let mut ks = vec![];
    for comb in &out {
        // g.obj(x) pour x dans c["tg"] : les unités choisies (sur le plateau)
        ks.push(-sum_value(g, comb)?);
    }
    let out = sort_keys(out, ks);
    let out: Vec<Map> = out.iter().map(|c| map! { tg = uids_t(g, c) }).collect();
    let mut out = cap(g, out, 7, Some(pid));
    out.push(empty_choice("tg"));
    Ok(chv(out))
}

fn discord(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let mut us: Vec<O> = all_legal(g, it)?.into_iter().flatten().collect();
    if sum_might(g, &us)? > 5 {
        // règle 355.11.b : le groupe ne respecte plus la restriction : choisir un sous-ensemble des cibles d'origine
        let mut subs = vec![];
        for k in (1..=us.len()).rev() {
            for c in combinations(&us, k) {
                if sum_might(g, &c)? <= 5 {
                    subs.push(c);
                }
            }
        }
        let mut ks = vec![];
        for s in &subs {
            ks.push(-sum_value(g, s)?);
        }
        let subs = sort_keys(subs, ks);
        let c = ctrl(g, it);
        let ov: Vec<Value> = subs.iter().map(|s| Value::List(objv(s))).collect();
        let r = g.ask(c, "discord_subset", ov, Map::new())?;
        us = r.list().iter().map(|v| v.obj()).collect();
    }
    for u in us {
        to_hand(g, Some(u))?;
    }
    Ok(Value::None)
}

fn c_decree_of_discord() -> Impl {
    Impl {
        preds: Value::List(vec![f("_order_enemy", order_enemy)]),
        resolve: f("_discord", discord),
        choices: f("_dod_choices", dod_choices),
        ..new_impl("Decree of Discord")
    }
}

// Diana, No Longer Human — "[Ambush] When you play a spell, give me +2 might this turn."
fn c_diana_no_longer_human() -> Impl {
    Impl {
        ambush: true,
        on_event: f("_diana", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "played" && pid_of(&info.g("pid")) == g.o(o).ctrl && stype(g, info.g("card").obj()) == "Spell" {
                q0(g, o, "Diana, No Longer Human", buff_me(2));
            }
            Ok(Value::None)
        }),
        ..new_impl("Diana, No Longer Human")
    }
}

// Doran's Ring — "[Equip] [Chaos]" ; card image: Might Bonus +1, Effect Text "When I conquer, discard 1, then draw
// 1."
fn c_dorans_ring() -> Impl {
    let mut im = Impl {
        bonus: Some(1),
        effect_event: f("_doran_effect", |g, a| {
            let (unit, ev, info) = (a[1].obj(), a[2].str(), a[3].map());
            if ev == "conquer" && pid_of(&info.g("pid")) == g.o(unit).ctrl && in_units(info, unit) {
                let (c, u) = (g.o(unit).ctrl, g.o(unit).uid);
                g.queue_trigger(c, "Doran's Ring", loot_res(), Map::new(), Some(u), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..new_impl("Doran's Ring")
    };
    impl_equip(&mut im, Cost::Text("1 chaos rune".into()));
    im
}

// Downwell — "Return all units and gear to their owners' hands."
fn c_downwell() -> Impl {
    Impl {
        resolve: f("_downwell", |g, _a| {
            let os: Vec<O> =
                g.board.iter().copied().filter(|&x| matches!(stype(g, x), "Unit" | "Gear")).collect();
            for o in os {
                to_hand(g, Some(o))?;
            }
            Ok(Value::None)
        }),
        ..new_impl("Downwell")
    }
}

// Draven, Audacious — "[Deflect] The first time I win a combat each turn, you score 1 point. When I die in combat,
// choose an opponent. They score 1 point."  (points not from Conquer ignore the Final Point rule, 471.1.a.1)
fn draven_die() -> Value {
    f("_draven_die", |g, a| {
        let info = a[1].map();
        let inf = info.g("info");
        let inf = inf.map();
        if inf.g("name").opt_str() == Some("Draven, Audacious") && inf.g("in_combat").truthy() {
            // se déclenche sur sa propre mort (428.1.a.1.b), comme un Deathknell sans en être un (Karthus ne double pas)
            let res = f("_draven_point", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                g_.gain_point(1 - c, "Draven, Audacious")?;
                Ok(Value::None)
            });
            let p = pid_of(&inf.g("ctrl"));
            g.queue_trigger(p, "Draven, Audacious (death)", res, Map::new(), None, false, Value::None, Value::None);
        }
        Ok(Value::None)
    })
}

fn c_draven_audacious() -> Impl {
    Impl {
        kw: kw1("Deflect"),
        on_event: f("_draven", |g, a| {
            let (o, ev, info) = ev_args(a);
            ensure_effect(g, "draven_die", "die", draven_die());
            if ev == "combat_won" && won_combat(g, o, info) {
                let key = stat_key("draven_win", g, o);
                if g.stats.get(&key) != 0 {
                    return Ok(Value::None);
                }
                let cur = g.stats.get(&key);
                g.stats.add(&key, 1 - cur);
                let res = f("_draven_win", |g_, a_| {
                    let c = ctrl(g_, a_[0].item());
                    g_.gain_point(c, "Draven, Audacious")?;
                    Ok(Value::None)
                });
                q0(g, o, "Draven, Audacious", res);
            }
            Ok(Value::None)
        }),
        on_play: f("_draven_play", |g, _a| {
            ensure_effect(g, "draven_die", "die", draven_die());
            Ok(Value::None)
        }),
        ..new_impl("Draven, Audacious")
    }
}

// ====================================================================== E
// Edge of Night — "[Hidden] When you play this from face down, attach it to a unit you control here. [Equip]
// [Chaos]" ; card image: Might Bonus +2.
fn edge_play(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ctx) = (a[0].obj(), a[1].map());
    if ctx.g("src").opt_str() != Some("facedown") {
        return Ok(Value::None);
    }
    let here = ctx.g("hidden_bf").opt_loc();
    let res = f("_edge_res", |g_, a_| {
        let it = a_[0].item();
        let u = g_.legal(it, 0)?;
        let m = me(g_, it);
        if let (Some(u), Some(m)) = (u, m) {
            attach(g_, m, u)?;
        }
        Ok(Value::None)
    });
    let opts = f("_edge_opts", move |g_, a_| {
        let c = ctrl(g_, a_[0].item());
        let us: Vec<O> = fr(g_, c)?.into_iter().filter(|&u| g_.o(u).loc == here).collect();
        Ok(us.into())
    });
    let pred = f("_edge_pred", move |g_, a_| {
        let (it, u) = (a_[0].item(), a_[1].obj());
        Ok((g_.o(u).ctrl == g_.it(it).ctrl && g_.o(u).loc == here).into())
    });
    q(g, o, "Edge of Night", res, Map::new(), false, trig_target(opts, pred, "equip_target", false, true), Value::None);
    Ok(Value::None)
}

fn c_edge_of_night() -> Impl {
    let mut im =
        Impl { hidden: true, bonus: Some(2), on_play: f("_edge_play", edge_play), ..new_impl("Edge of Night") };
    impl_equip(&mut im, Cost::Text("1 chaos rune".into()));
    im
}

// Ember Monk — "When you play a card from [Hidden], give me +2 might this turn." (pas le mot-clé Hidden : il ne se
// cache pas, règle 811.1 ; le CSV range « Hidden » dans keywords parce que le texte le mentionne, image OGN 167 vérifiée)
fn c_ember_monk() -> Impl {
    Impl {
        on_event: f("_ember", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "played" && pid_of(&info.g("pid")) == g.o(o).ctrl && from_facedown(g, info) {
                q0(g, o, "Ember Monk", buff_me(2));
            }
            Ok(Value::None)
        }),
        ..new_impl("Ember Monk")
    }
}

// Evelynn, Entrancing — "[Hidden] [Backline] When you play me from face down on your turn, you may move an enemy
// unit at a different location to my battlefield."  (target chosen freely: rule 811.1.d.2)
fn evelynn(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ctx) = (a[0].obj(), a[1].map());
    if ctx.g("src").opt_str() != Some("facedown") {
        return Ok(Value::None);
    }
    if g.tp != g.o(o).ctrl {
        g.log(|| {
            tr(
                "  Evelynn, Entrancing : jouée depuis la face cachée pendant le tour adverse, pas d'effet (« on your turn »)",
                "  Evelynn, Entrancing: played from face down on the opponent's turn, no effect (\"on your turn\")",
            )
        });
        return Ok(Value::None);
    }
    let here = g.o(o).loc;
    let res = f("_evelynn_res", |g_, a_| {
        let it = a_[0].item();
        let u = g_.legal(it, 0)?;
        let m = me(g_, it);
        if let (Some(u), Some(m)) = (u, m) {
            if is_bf(g_.o(m).loc) && g_.o(u).loc != g_.o(m).loc {
                let (d, c) = (g_.o(m).loc.unwrap(), ctrl(g_, it));
                mv(g_, u, d, c)?;
            }
        }
        Ok(Value::None)
    });
    let opts = f("_evelynn_opts", move |g_, a_| {
        let it = a_[0].item();
        let c = ctrl(g_, it);
        let ln = loc_now(g_, it, here);
        let us: Vec<O> = en(g_, c)?.into_iter().filter(|&u| g_.o(u).loc != ln).collect();
        Ok(us.into())
    });
    let pred = f("_evelynn_pred", |g_, a_| {
        let (it, u) = (a_[0].item(), a_[1].obj());
        Ok((g_.o(u).ctrl != g_.it(it).ctrl).into())
    });
    q(g, o, "Evelynn, Entrancing", res, Map::new(), true, trig_target(opts, pred, "target", false, true), Value::None);
    Ok(Value::None)
}

fn c_evelynn_entrancing() -> Impl {
    Impl { hidden: true, kw: kw1("Backline"), on_play: f("_evelynn", evelynn), ..new_impl("Evelynn, Entrancing") }
}

// Evershade Stalker — "When you play me, discard 1, then draw 1."
fn c_evershade_stalker() -> Impl {
    Impl {
        on_play: f("_evershade", |g, a| {
            q0(g, a[0].obj(), "Evershade Stalker", loot_res());
            Ok(Value::None)
        }),
        ..new_impl("Evershade Stalker")
    }
}

// Existential Dread — "[Action] [Repeat] [2] [Stun] an attacking enemy unit. If it's already stunned, return it to
// its owner's hand instead."
fn dread(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
    if g.o(u).stunned {
        to_hand(g, Some(u))?;
    } else {
        let c = ctrl(g, it);
        g.stun(Some(u), Some(c))?;
    }
    Ok(Value::None)
}

fn c_existential_dread() -> Impl {
    Impl {
        timing: Some("action"),
        repeat: repeat_cost("2 energy"),
        preds: Value::List(vec![f("_attacking_enemy", |g, a| {
            let (it, o) = (a[0].item(), a[1].obj());
            Ok((p_enemy(g, it, o) && g.o(o).desig == Some(Desig::Att)).into())
        })]),
        resolve: repeatable(f("_dread", dread)),
        choices: f("_dread_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let us: Vec<O> = en(g, pid)?.into_iter().filter(|&u| g.o(u).desig == Some(Desig::Att)).collect();
            Ok(tgv(g, &us))
        }),
        ..new_impl("Existential Dread")
    }
}

// ====================================================================== F
// Factory Recall — "[Action] Return a gear to its owner's hand."
fn c_factory_recall() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_gear_v()]),
        resolve: f("_factory_recall", |g, a| {
            let u = g.legal(a[0].item(), 0)?;
            to_hand(g, u)?;
            Ok(Value::None)
        }),
        choices: f("_factory_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let gs = g.gear(None);
            let ks: Vec<(bool, i32)> = gs.iter().map(|&x| (g.o(x).ctrl == pid, -g.spec(x).e)).collect();
            let gs = sort_keys(gs, ks);
            Ok(tgv(g, &gs))
        }),
        ..new_impl("Factory Recall")
    }
}

// Fading Memories — "Give a unit at a battlefield or a gear [Temporary]."
fn c_fading_memories() -> Impl {
    Impl {
        preds: Value::List(vec![f("_fading_pred", |g, a| {
            let o = a[1].obj();
            Ok(((stype(g, o) == "Unit" && is_bf(g.o(o).loc)) || stype(g, o) == "Gear").into())
        })]),
        resolve: f("_fading", |g, a| {
            let it = a[0].item();
            if let Some(u) = g.legal(it, 0)? {
                let u2 = g.legal(it, 0)?.unwrap_or(u);
                g.grant(u2, "Temporary", 1, Value::None);
            }
            Ok(Value::None)
        }),
        choices: f("_fading_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let mut us = en_bf(g, pid)?;
            let og: Vec<O> = g.gear(None).into_iter().filter(|&x| g.o(x).ctrl != pid).collect();
            let ks: Vec<i32> = og.iter().map(|&x| -g.spec(x).e).collect();
            us.extend(sort_keys(og, ks));
            us.extend(g.gear(Some(pid)));
            us.extend(fr_bf(g, pid)?);
            Ok(tgv(g, &us))
        }),
        ..new_impl("Fading Memories")
    }
}

// Fae Porter — "When I move to a battlefield, you may pay [Chaos] to move a unit you control to the same
// battlefield."
fn c_fae_porter() -> Impl {
    Impl {
        on_event: f("_porter", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "move" && is_obj(info, o) && is_bf(info.g("to").opt_loc()) {
                let dest = info.g("to").loc();
                let res = f("_porter_res", move |g_, a_| {
                    let it = a_[0].item();
                    if let Some(u) = g_.legal(it, 0)? {
                        if g_.o(u).loc != Some(dest) {
                            let c = ctrl(g_, it);
                            mv(g_, u, dest, c)?;
                        }
                    }
                    Ok(Value::None)
                });
                let reqs = f("_porter_reqs", |_g, _a| Ok(reqs_to_value(&[req1("Chaos")])));
                let opts = f("_porter_opts", move |g_, a_| {
                    let c = ctrl(g_, a_[0].item());
                    let us: Vec<O> = fr(g_, c)?.into_iter().filter(|&u| g_.o(u).loc != Some(dest)).collect();
                    Ok(us.into())
                });
                let choose = trig_target(opts, p_friend_v(), "friend_target", false, true);
                q(g, o, "Fae Porter", res, Map::new(), true, choose, may_pay(0, reqs));
            }
            Ok(Value::None)
        }),
        ..new_impl("Fae Porter")
    }
}

// Fight or Flight — "[Hidden] [Action] Move a unit from a battlefield to its base."
fn c_fight_or_flight() -> Impl {
    Impl {
        hidden: true,
        timing: Some("action"),
        preds: Value::List(vec![p_unit_bf_v()]),
        resolve: f("_fight_or_flight", |g, a| {
            let it = a[0].item();
            if g.legal(it, 0)?.is_some() {
                let u = g.legal(it, 0)?;
                let c = ctrl(g, it);
                g.move_(&u.into_iter().collect::<Vec<O>>(), Loc::Base, Some(c), false)?;
            }
            Ok(Value::None)
        }),
        choices: f("_fof_choices", |g, a| {
            let (pid, hb) = (pid_of(&a[0]), a[1].map().g("hidden_bf").opt_loc());
            let mut us = enemies(g, pid, true, hb, true)?;
            us.extend(friends(g, pid, true, hb)?);
            Ok(tgv(g, &us))
        }),
        ..new_impl("Fight or Flight")
    }
}

// Fizz, Trickster — "When you play me, you may play a spell from your trash with Energy cost no more than [3],
// ignoring its Energy cost. Recycle that spell after you play it. (You must still pay its Power cost.)"
// A spell countered after being played this way was not played (rule 425.1.b): it goes to the trash (425.1.a.1).
fn spell_play_options(g: &mut Game, pid: usize, c: O, src: &str) -> Res<Vec<(O, Map)>> {
    let Some(im) = g.impl_o(c) else { return Ok(vec![]) };
    let ctx = map! { hidden_bf = Value::None, card = c };
    let base: Vec<Map> = if full_choices(g, Some(pid)) && !im.all_choices.is_none() {
        let r = im.all_choices.call(g, &[pid.into(), ctx.into()])?;
        if r.truthy() {
            r.list().iter().map(|v| v.map().clone()).collect()
        } else {
            vec![Map::new()]
        }
    } else {
        let b: Vec<Map> = if !im.choices.is_none() {
            im.choices.call(g, &[pid.into(), ctx.into()])?.list().iter().map(|v| v.map().clone()).collect()
        } else {
            vec![Map::new()]
        };
        cap(g, b, 8, Some(pid))
    };
    let mut out = vec![];
    for ch in &base {
        let mut vs = vec![ch.clone()];
        if im.repeat.truthy() {
            for ch2 in cap(g, base.clone(), 3, Some(pid)) {
                let mut v = ch.clone();
                v.set("rep", true);
                v.set("tg2", ch2.get("tg").cloned().unwrap_or(Value::Tuple(vec![])));
                vs.push(v);
            }
        }
        for v in vs {
            let mut v = v;
            v.set("ignore_energy", true);
            if affordable(g, pid, c, &v, src)? {
                out.push((c, v));
            }
        }
    }
    Ok(out)
}

fn fizz(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let pid = ctrl(g, it);
    let mut opts = vec![];
    let cs: Vec<O> =
        g.p[pid].trash.iter().copied().filter(|&c| stype(g, c) == "Spell" && g.spec(c).e <= 3).collect();
    for c in by_cval_desc(g, cs) {
        opts.extend(spell_play_options(g, pid, c, "trash")?);
    }
    let pick = if !opts.is_empty() {
        let mut ov: Vec<Value> = opts.into_iter().map(|(c, v)| Value::Tuple(vec![c.into(), v.into()])).collect();
        ov.push(Value::None);
        g.ask(pid, "fizz_pick", ov, Map::new())?
    } else {
        Value::None
    };
    if pick.is_none() {
        return Ok(Value::None);
    }
    let (c, ch) = (pick.list()[0].obj(), pick.list()[1].map().clone());
    let (e, reqs) = total_cost(g, pid, c, &ch, "trash")?;
    if !g.pay(pid, e, &reqs, &map! { kind = "spell", card = c }.into())? {
        return Ok(Value::None);
    }
    let sp = play_card(g, pid, c, "trash", ch.clone(), true)?;
    if sp.is_none() {
        return Ok(Value::None);
    }
    let (iid, cu, owner) = (g.it(sp.item()).id, g.o(c).uid, g.o(c).owner);
    let recycle = f("_fizz_recycle", move |g_, a_| {
        let (eid, info) = (a_[0].int() as u64, a_[1].map());
        let item = info.g("item");
        if !item.is_none() && g_.it(item.item()).id == iid {
            g_.remove_effect(eid);
            // relire la carte par uid : la table et l'IA rejouent sur une copie profonde, où « c » capturé est périmé
            let x = g_.p[owner].trash.iter().copied().find(|&y| g_.o(y).uid == cu);
            if let Some(x) = x {
                g_.recycle_cards(owner, &[x])?; // "Recycle that spell after you play it"
            }
        }
        Ok(Value::None)
    });
    let mut d = Map::new();
    d.set("on", "played");
    d.set("fn", recycle);
    d.set("dur", "turn");
    g.add_effect(d);
    Ok(Value::None)
}

fn c_fizz_trickster() -> Impl {
    Impl {
        on_play: f("_fizz_play", |g, a| {
            q0(g, a[0].obj(), "Fizz, Trickster", f("_fizz", fizz));
            Ok(Value::None)
        }),
        ..new_impl("Fizz, Trickster")
    }
}

// Flash — "[Reaction] Move up to 2 friendly units to base."
fn flash_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = pid_of(&a[0]);
    let frs = fr_bf(g, pid)?;
    let frs = cap(g, frs, 4, Some(pid));
    let mut out: Vec<Map> = combinations(&frs, 2).iter().map(|c| map! { tg = uids_t(g, c) }).collect();
    out.extend(frs.iter().map(|&u| map! { tg = uids_t(g, &[u]) }));
    let mut out = cap(g, out, 7, Some(pid));
    out.push(empty_choice("tg"));
    Ok(chv(out))
}

fn flash(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let us: Vec<O> =
        all_legal(g, it)?.into_iter().flatten().filter(|&u| g.o(u).loc != Some(Loc::Base)).collect();
    if !us.is_empty() {
        let c = ctrl(g, it);
        g.move_(&us, Loc::Base, Some(c), false)?;
    }
    Ok(Value::None)
}

fn c_flash() -> Impl {
    Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![p_friend_v()]),
        resolve: f("_flash", flash),
        choices: f("_flash_choices", flash_choices),
        ..new_impl("Flash")
    }
}

// Forgotten Relic — "When you play this or at the start of your Beginning Phase, [Burn 1]. When you burn a unit this
// way, do this: Give a friendly unit +might equal to the burned card's Might this turn."
fn relic_trigger(g: &mut Game, o: O) {
    let res = f("_relic_res", |g_, a_| {
        let it = a_[0].item();
        let c0 = ctrl(g_, it);
        for c in g_.burn(c0, 1)? {
            if stype(g_, c) == "Unit" {
                let give = f("_relic_give", |g2, a2| {
                    let it2 = a2[0].item();
                    if let Some(u) = g2.legal(it2, 0)? {
                        let m = dget(g2, it2, "m").int();
                        g2.mod_(u, m, "turn".into(), None)?;
                    }
                    Ok(Value::None)
                });
                let opts = f("_relic_opts", |g2, a2| {
                    let c = ctrl(g2, a2[0].item());
                    Ok(fr(g2, c)?.into())
                });
                let m = g_.spec(c).might.unwrap_or(0);
                let choose = trig_target(opts, p_friend_v(), "friend_target", false, true);
                g_.queue_trigger(c0, "Forgotten Relic (unit burned)", give, map! { m = m }, None, false, choose, Value::None);
            }
        }
        Ok(Value::None)
    });
    q0(g, o, "Forgotten Relic", res);
}

fn c_forgotten_relic() -> Impl {
    Impl {
        on_play: f("_relic_play", |g, a| {
            relic_trigger(g, a[0].obj());
            Ok(Value::None)
        }),
        on_event: f("_relic_event", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "beginning_start" && pid_of(&info.g("pid")) == g.o(o).ctrl {
                relic_trigger(g, o);
            }
            Ok(Value::None)
        }),
        ..new_impl("Forgotten Relic")
    }
}

// ====================================================================== G
// Gust — "[Reaction] Return a unit at a battlefield with 3 might or less to its owner's hand."
fn c_gust() -> Impl {
    Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![f("_small_bf", |g, a| {
            let (it, o) = (a[0].item(), a[1].obj());
            Ok((p_unit_bf(g, it, o) && g.might(o)? <= 3).into())
        })]),
        resolve: f("_gust", |g, a| {
            let u = g.legal(a[0].item(), 0)?;
            to_hand(g, u)?;
            Ok(Value::None)
        }),
        choices: f("_gust_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let mut us = vec![];
            for u in en_bf(g, pid)? {
                if g.might(u)? <= 3 {
                    us.push(u);
                }
            }
            for u in fr_bf(g, pid)? {
                if g.might(u)? <= 3 {
                    us.push(u);
                }
            }
            Ok(tgv(g, &us))
        }),
        ..new_impl("Gust")
    }
}

// Gust Monk — "You may pay [1] as an additional cost to play me. When you play me, if you paid the additional
// cost, banish a card from any trash to give a unit [Assault 2] this turn."
fn gust_monk(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ctx) = (a[0].obj(), a[1].map());
    if !ctx.g("gm").truthy() {
        return Ok(Value::None);
    }
    // « [banish X] to [give Y] » : un coût dans l'instruction (355.10.c.1)
    let cost = f("_gust_monk_cost", |g_, a_| {
        let it = a_[0].item();
        let c0 = ctrl(g_, it);
        let mut cs = vec![];
        for p in [1 - c0, c0] {
            cs.extend(g_.p[p].trash.iter().copied());
        }
        if cs.is_empty() {
            return Ok(false.into());
        }
        let ks: Vec<(bool, f64)> = cs.iter().map(|&c| (g_.o(c).owner == c0, -cval(g_, c))).collect();
        let cs = sort_keys(cs, ks);
        let c = g_.ask(c0, "banish_from_trash", objv(&cs), Map::new())?.obj();
        g_.to_zone(c, "banish", false, None)?;
        Ok(true.into())
    });
    let res = f("_gust_monk_res", |g_, a_| {
        if let Some(u) = g_.legal(a_[0].item(), 0)? {
            g_.grant(u, "Assault", 2, "turn".into());
        }
        Ok(Value::None)
    });
    let opts = f("_gust_monk_opts", |g_, a_| {
        let c = ctrl(g_, a_[0].item());
        let mut us = fr(g_, c)?;
        us.extend(en(g_, c)?);
        Ok(us.into())
    });
    q(g, o, "Gust Monk", res, Map::new(), false, trig_target(opts, p_unit_v(), "friend_target", false, true), cost);
    Ok(Value::None)
}

fn c_gust_monk() -> Impl {
    Impl {
        on_play: f("_gust_monk", gust_monk),
        as_played: f("_gm_as_played", |_g, _a| Ok(chv(vec![Map::new(), map! { gm = true }]))),
        extra_cost_fn: f("_gm_extra", |_g, a| {
            let e = if a[2].map().g("gm").truthy() { 1 } else { 0 };
            Ok(Value::Tuple(vec![e.into(), reqs_to_value(&[])]))
        }),
        ..new_impl("Gust Monk")
    }
}

// ====================================================================== H
// Hard Bargain — "[Reaction] [Repeat] [2] Counter a spell unless its controller pays [2]."
// The repetition may choose another spell (rule 820.2.a), so it is modelled with its own choice (rep2/item2) priced
// by extra_cost_fn instead of the generic Impl.repeat (which only re-chooses unit targets).
fn bargain_once(g: &mut Game, it: ItemId, iid: &Value) -> Res<()> {
    let tgt = item_of(g, iid);
    let Some(t) = tgt else { return Ok(()) };
    if g.it(t).kind != "spell" || !g.counterable(t)? {
        return Ok(());
    }
    let p = g.it(t).ctrl;
    if g.can_pay(p, 2, &[], &Value::None)?
        && g.ask(p, "pay_or_countered", vec![true.into(), false.into()], map! { item = it })?.truthy()
    {
        g.pay(p, 2, &[], &Value::None)?;
        return Ok(());
    }
    g.counter(t)?;
    Ok(())
}

fn c_hard_bargain() -> Impl {
    Impl {
        timing: Some("reaction"),
        resolve: f("_bargain", |g, a| {
            let it = a[0].item();
            bargain_once(g, it, &dget(g, it, "item"))?;
            if dget(g, it, "rep2").truthy() {
                bargain_once(g, it, &dget(g, it, "item2"))?;
            }
            Ok(Value::None)
        }),
        choices: f("_bargain_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let base = counter_choices_v(g, pid)?;
            let mut reps = vec![];
            for x in cap(g, base.clone(), 2, Some(pid)) {
                for y in cap(g, base.clone(), 2, Some(pid)) {
                    let mut m = x.clone();
                    m.set("rep2", true);
                    m.set("item2", y.g("item"));
                    reps.push(m);
                }
            }
            // variantes avec répétition parmi les 8 premiers choix
            let k = base.len().min(3);
            let mut out: Vec<Map> = base[..k].to_vec();
            out.extend(reps);
            out.extend(base[k..].iter().cloned());
            Ok(chv(out))
        }),
        opt_parts: f("_bargain_parts", |_g, a| {
            if a[2].map().g("rep2").truthy() {
                Ok(Value::List(vec![Value::Tuple(vec!["rep".into(), 2.into(), reqs_to_value(&[])])]))
            } else {
                Ok(Value::List(vec![]))
            }
        }),
        ..new_impl("Hard Bargain")
    }
}

// Harpoon Squad — "When I move from a battlefield, give me +2 might this turn."
fn c_harpoon_squad() -> Impl {
    Impl {
        on_event: f("_harpoon", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "move" && is_obj(info, o) && is_bf(info.g("frm").opt_loc()) {
                q0(g, o, "Harpoon Squad", buff_me(2));
            }
            Ok(Value::None)
        }),
        ..new_impl("Harpoon Squad")
    }
}

// Heedless Resurrection — "[Reaction] As an additional cost to play this, kill a friendly unit. Play a unit from
// your trash that costs no more Energy and no more Power than the killed unit, ignoring its cost."
// (look-back at the killed unit's cost, rule 359.3.e.13; a token costs 0, rule 185.3.a.1)
fn c_heedless_resurrection() -> Impl {
    Impl {
        timing: Some("reaction"),
        resolve: f("_heedless", |g, a| {
            let it = a[0].item();
            let ki = dget(g, it, "killed_info");
            if !ki.truthy() {
                return Ok(Value::None);
            }
            let (ke, kp) = (ki.map().g("e").int(), ki.map().g("p").int());
            let c0 = ctrl(g, it);
            let cands = trash_units(g, c0, |g_, c| g_.spec(c).e as i64 <= ke && g_.spec(c).p as i64 <= kp);
            let pick = g.ask(c0, "resurrect_pick", objv(&cands), Map::new())?;
            if let Some(p) = pick.opt_obj() {
                play_unit_free(g, c0, p, "trash", None, false, None)?;
            }
            Ok(Value::None)
        }),
        choices: f("_heedless_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let us = g.units(Some(pid), None);
            let us = by_value(g, us)?;
            Ok(chv(us.iter().map(|&u| map! { kill = g.o(u).uid }).collect()))
        }),
        ..new_impl("Heedless Resurrection")
    }
}

// ====================================================================== I
// Illaoi, Prophet of the Great Kraken — "When you play me or when I score, play a 1 might Tentacle unit token.
// I have +1 might for each token unit you control."  ("I score": I'm among the units of a conquer or hold, 471.2)
fn illaoi_token(g: &mut Game, o: O) {
    let res = f("_illaoi_res", |g_, a_| {
        let c = ctrl(g_, a_[0].item());
        token(g_, c, "Tentacle", false)?;
        Ok(Value::None)
    });
    q0(g, o, "Illaoi, Prophet of the Great Kraken", res);
}

fn c_illaoi() -> Impl {
    Impl {
        on_play: f("_illaoi_play", |g, a| {
            illaoi_token(g, a[0].obj());
            Ok(Value::None)
        }),
        on_event: f("_illaoi_event", |g, a| {
            let (o, ev, info) = ev_args(a);
            if (ev == "conquer" || ev == "hold") && pid_of(&info.g("pid")) == g.o(o).ctrl && in_units(info, o) {
                illaoi_token(g, o);
            }
            Ok(Value::None)
        }),
        might_mod: f("_illaoi_might", |g, a| {
            let o = a[0].obj();
            Ok(g.units(Some(g.o(o).ctrl), None).iter().filter(|&&u| g.o(u).token).count().into())
        }),
        ..new_impl("Illaoi, Prophet of the Great Kraken")
    }
}

// Insightful Investigator — "When you play me, choose an opponent. They reveal their hand. You may pay 2 XP to
// choose a card from their hand. If you do, they discard that card and draw 1."
fn investigator(g: &mut Game, a: &[Value]) -> Res<Value> {
    let c0 = ctrl(g, a[0].item());
    let opp = 1 - c0;
    g.reveal_hand(opp, c0)?;
    let hand_ = g.p[opp].hand.clone();
    if hand_.is_empty() || g.p[c0].xp < 2 {
        return Ok(Value::None);
    }
    let mut ov = objv(&by_cval_desc(g, hand_));
    ov.push(Value::None);
    let c = g.ask(c0, "investigator_pick", ov, Map::new())?;
    let Some(c) = c.opt_obj() else { return Ok(Value::None) };
    if !g.spend_xp(c0, 2)? {
        return Ok(Value::None);
    }
    g.discard(opp, c)?;
    g.draw(opp, 1)?;
    Ok(Value::None)
}

fn c_insightful_investigator() -> Impl {
    Impl {
        on_play: f("_investigator_play", |g, a| {
            q0(g, a[0].obj(), "Insightful Investigator", f("_investigator", investigator));
            Ok(Value::None)
        }),
        ..new_impl("Insightful Investigator")
    }
}

// Invert Timelines — "Each player discards their hand, then draws 4."
fn c_invert_timelines() -> Impl {
    Impl {
        resolve: f("_invert", |g, _a| {
            for pid in [g.tp, 1 - g.tp] {
                for c in g.p[pid].hand.clone() {
                    g.discard(pid, c)?;
                }
            }
            for pid in [g.tp, 1 - g.tp] {
                g.draw(pid, 4)?;
            }
            Ok(Value::None)
        }),
        ..new_impl("Invert Timelines")
    }
}

// Isolate — "Move an enemy unit from a battlefield to its base. Then, if there's an enemy unit alone at that
// battlefield, draw 1."
fn c_isolate() -> Impl {
    Impl {
        preds: Value::List(vec![p_enemy_bf_v()]),
        resolve: f("_isolate", |g, a| {
            let it = a[0].item();
            // « that battlefield » désigne celui de la cible (359.3.e.14.a)
            let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
            let bf = g.o(u).loc;
            let c = ctrl(g, it);
            mv(g, u, Loc::Base, c)?;
            if g.units(Some(1 - c), bf).iter().any(|&e| g.alone(e, None)) {
                g.draw(c, 1)?;
            }
            Ok(Value::None)
        }),
        choices: f("_isolate_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let es = en_bf(g, pid)?;
            let mut ks = vec![];
            for &u in &es {
                ks.push((g.units(Some(g.o(u).ctrl), g.o(u).loc).len() != 2, -value(g, u)?));
            }
            let es = sort_keys(es, ks);
            Ok(tgv(g, &es))
        }),
        ..new_impl("Isolate")
    }
}

// ====================================================================== J
// Jae Medarda — "When you choose me with a spell, draw 1."
fn c_jae_medarda() -> Impl {
    Impl {
        on_event: f("_jae", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "chosen" && is_obj(info, o) {
                let item = info.g("item").item();
                if g.it(item).kind == "spell" && g.it(item).ctrl == g.o(o).ctrl {
                    q0(g, o, "Jae Medarda", draw1_res());
                }
            }
            Ok(Value::None)
        }),
        ..new_impl("Jae Medarda")
    }
}

// Jinx, Rebel — "When you discard one or more cards, ready me and give me +1 might this turn."
// The discards of one action (Invert Timelines) trigger once: a pending trigger of this Jinx is not doubled.
fn c_jinx_rebel() -> Impl {
    Impl {
        on_event: f("_jinx", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "discard" && pid_of(&info.g("pid")) == g.o(o).ctrl {
                let u = g.o(o).uid;
                if g.trigq.iter().any(|&t| g.it(t).src == Some(u) && g.it(t).name == "Jinx, Rebel") {
                    return Ok(Value::None);
                }
                let res = f("_jinx_res", |g_, a_| {
                    if let Some(m) = me(g_, a_[0].item()) {
                        g_.ready_obj(m, None, None)?;
                        g_.mod_(m, 1, "turn".into(), None)?;
                    }
                    Ok(Value::None)
                });
                q0(g, o, "Jinx, Rebel", res);
            }
            Ok(Value::None)
        }),
        ..new_impl("Jinx, Rebel")
    }
}

// ====================================================================== K
// Kayn, Unleashed — "[Ganking] If I have moved twice this turn, I don't take damage."
// From his second move this turn all damage to him is prevented (Game.deal and combat damage use Obj.prevent,
// which is cleared at the end of the turn).
fn c_kayn_unleashed() -> Impl {
    Impl {
        kw: kw1("Ganking"),
        on_event: f("_kayn", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "move" && is_obj(info, o) {
                let key = stat_key("kayn_moves", g, o);
                g.stats.add(&key, 1);
                if g.stats.get(&key) >= 2 && g.o(o).prevent < 1_000_000 {
                    g.o_mut(o).prevent += 1_000_000;
                }
            }
            Ok(Value::None)
        }),
        ..new_impl("Kayn, Unleashed")
    }
}

// Kha'Zix, Mutating Horror — "[Ambush] When I attack or defend, if an enemy unit is alone here, give me +2 might
// this turn and gain 2 XP."  (intervening if: checked again on resolution)
fn enemy_alone_here(g: &Game, o: O) -> bool {
    is_bf(g.o(o).loc) && g.units(Some(1 - g.o(o).ctrl), g.o(o).loc).iter().any(|&e| g.alone(e, None))
}

fn c_khazix() -> Impl {
    Impl {
        ambush: true,
        on_event: f("_khazix", |g, a| {
            let (o, ev, info) = ev_args(a);
            if (ev == "attack" || ev == "defend") && is_obj(info, o) && enemy_alone_here(g, o) {
                let res = f("_khazix_res", |g_, a_| {
                    let it = a_[0].item();
                    if let Some(m) = me(g_, it) {
                        if enemy_alone_here(g_, m) {
                            g_.mod_(m, 2, "turn".into(), None)?;
                            let c = ctrl(g_, it);
                            g_.gain_xp(c, 2)?;
                        }
                    }
                    Ok(Value::None)
                });
                q0(g, o, "Kha'Zix, Mutating Horror", res);
            }
            Ok(Value::None)
        }),
        ..new_impl("Kha'Zix, Mutating Horror")
    }
}

// Kharox — "[Empower] [6][Chaos][Chaos] When I become [Empowered], choose an opponent. They [Burn 3]. Then you may
// do this: Choose a unit in their trash and play it, ignoring its cost."
fn kharox_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let c0 = ctrl(g, a[0].item());
    let opp = 1 - c0;
    g.burn(opp, 3)?;
    let cands = trash_units(g, opp, |_, _| true);
    let pick = if !cands.is_empty() {
        let mut ov = objv(&cands);
        ov.push(Value::None);
        g.ask(c0, "kharox_pick", ov, Map::new())?
    } else {
        Value::None
    };
    if let Some(p) = pick.opt_obj() {
        play_unit_free(g, c0, p, "trash", None, false, None)?;
    }
    Ok(Value::None)
}

fn c_kharox() -> Impl {
    let mut im = Impl {
        on_event: f("_kharox", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "empowered" && is_obj(info, o) {
                q0(g, o, "Kharox", f("_kharox_res", kharox_res));
            }
            Ok(Value::None)
        }),
        ..new_impl("Kharox")
    };
    impl_empower(&mut im, Cost::Text("6 energy and 2 chaos runes".into()));
    im
}

// Kinkou Lifeblade — "[Empower] [2] [Empowered] I have +1 might and [Ganking]."
fn c_kinkou_lifeblade() -> Impl {
    let mut im = Impl {
        might_if: vec![(when_empowered(), 1)],
        kw_if: vec![(when_empowered(), kw1("Ganking"))],
        ..new_impl("Kinkou Lifeblade")
    };
    impl_empower(&mut im, Cost::Text("2 energy".into()));
    im
}

// Kog'Maw, Caustic — "[Deathknell] Deal 4 to all units at my battlefield."
fn c_kogmaw_caustic() -> Impl {
    Impl {
        deathknell: f("_kogmaw", |g, a| {
            let it = a[0].item();
            let loc = dget(g, it, "info").map().g("loc").opt_loc();
            if is_bf(loc) {
                let c = ctrl(g, it);
                for u in g.units(None, loc) {
                    g.deal(Some(u), 4, "ability", Some(c), None)?;
                }
            }
            Ok(Value::None)
        }),
        ..new_impl("Kog'Maw, Caustic")
    }
}

// ====================================================================== L
// Last Rites — "[Equip] [Chaos], Recycle 2 cards from your trash" ; card image: Might Bonus +2, Effect Text "When I
// conquer or hold, you may play a unit from your trash. (You still pay its costs.)"
fn rites_extra(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = pid_of(&a[0]);
    let mut picks: Vec<O> = vec![];
    for _ in 0..2 {
        let rest: Vec<O> = g.p[pid].trash.iter().copied().filter(|c| !picks.contains(c)).collect();
        if rest.is_empty() {
            break;
        }
        let ks: Vec<f64> = rest.iter().map(|&c| cval(g, c)).collect();
        let rest = sort_keys(rest, ks);
        picks.push(g.ask(pid, "recycle_from_trash", objv(&rest), Map::new())?.obj());
    }
    g.recycle_cards(pid, &picks)?;
    Ok(Value::None)
}

fn rites_equip() -> Map {
    let mut ab = equip_ability_cost(Cost::Text("1 chaos rune".into()), "main");
    ab.set("can", f("_rites_can", |g, a| Ok((g.p[pid_of(&a[0])].trash.len() >= 2).into())));
    ab.set("can_extra", f("_rites_can_extra", |g, a| Ok((g.p[pid_of(&a[0])].trash.len() >= 2).into())));
    ab.set("extra_cost", f("_rites_extra", rites_extra));
    ab
}

fn c_last_rites() -> Impl {
    Impl {
        abilities: vec![rites_equip()],
        bonus: Some(2),
        effect_event: f("_rites_effect", |g, a| {
            let (unit, ev, info) = (a[1].obj(), a[2].str(), a[3].map());
            if (ev == "conquer" || ev == "hold") && pid_of(&info.g("pid")) == g.o(unit).ctrl && in_units(info, unit) {
                let res = f("_rites_res", |g_, a_| {
                    let c0 = ctrl(g_, a_[0].item());
                    let mut cands = vec![];
                    for c in trash_units(g_, c0, |_, _| true) {
                        if !pay_options(g_, c0, c, "trash")?.is_empty() {
                            cands.push(c);
                        }
                    }
                    let pick = if !cands.is_empty() {
                        let mut ov = objv(&cands);
                        ov.push(Value::None);
                        g_.ask(c0, "rites_pick", ov, Map::new())?
                    } else {
                        Value::None
                    };
                    if let Some(p) = pick.opt_obj() {
                        play_paying(g_, c0, p, "trash")?;
                    }
                    Ok(Value::None)
                });
                let (c, u) = (g.o(unit).ctrl, g.o(unit).uid);
                g.queue_trigger(c, "Last Rites", res, Map::new(), Some(u), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..new_impl("Last Rites")
    }
}

// Loyal Pup — "When you defend at a battlefield, you may move me there."
fn c_loyal_pup() -> Impl {
    Impl {
        on_event: f("_pup", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "combat_start"
                && pid_of(&info.g("sd").map().g("defender")) == g.o(o).ctrl
                && g.o(o).loc != Some(bf_loc(&info.g("bf")))
            {
                let bf = bf_loc(&info.g("bf"));
                let res = f("_pup_res", move |g_, a_| {
                    let it = a_[0].item();
                    if let Some(m) = me(g_, it) {
                        if g_.o(m).loc != Some(bf) {
                            let c = ctrl(g_, it);
                            mv(g_, m, bf, c)?;
                        }
                    }
                    Ok(Value::None)
                });
                q(g, o, "Loyal Pup", res, Map::new(), true, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..new_impl("Loyal Pup")
    }
}

// Lunar Boon — "[Reaction] Discard 1, then draw 2."
fn c_lunar_boon() -> Impl {
    Impl {
        timing: Some("reaction"),
        resolve: f("_lunar_boon", |g, a| {
            let c = ctrl(g, a[0].item());
            discard_n(g, c, 1)?;
            g.draw(c, 2)?;
            Ok(Value::None)
        }),
        ..new_impl("Lunar Boon")
    }
}

// ====================================================================== M
// Maddened Marauder — "[Tank] When you play me, move a unit from a battlefield to its base."
fn c_maddened_marauder() -> Impl {
    Impl {
        kw: kw1("Tank"),
        on_play: f("_marauder", |g, a| {
            let res = f("_marauder_res", |g_, a_| {
                let it = a_[0].item();
                if let Some(u) = g_.legal(it, 0)? {
                    let c = ctrl(g_, it);
                    mv(g_, u, Loc::Base, c)?;
                }
                Ok(Value::None)
            });
            let opts = f("_marauder_opts", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                let mut us = en_bf(g_, c)?;
                us.extend(fr_bf(g_, c)?);
                Ok(us.into())
            });
            let choose = trig_target(opts, p_unit_bf_v(), "target", false, true);
            q(g, a[0].obj(), "Maddened Marauder", res, Map::new(), false, choose, Value::None);
            Ok(Value::None)
        }),
        ..new_impl("Maddened Marauder")
    }
}

// Megatusk — "[Ganking] Spend 3 XP: Give your units here [Ganking] this turn."
fn c_megatusk() -> Impl {
    let res = f("_megatusk", |g, a| {
        let it = a[0].item();
        let Some(m) = g.it(it).src.and_then(|u| g.obj(u)) else { return Ok(Value::None) };
        let c = ctrl(g, it);
        for u in g.units(Some(c), g.o(m).loc) {
            g.grant(u, "Ganking", 1, "turn".into());
        }
        Ok(Value::None)
    });
    Impl {
        kw: kw1("Ganking"),
        abilities: vec![ability("Stampede", AbilityArgs { xp: 3, resolve: res, ..Default::default() })],
        ..new_impl("Megatusk")
    }
}

// Mel, Defiant Soul — "[Empower] — Discard a spell. When I become [Empowered], banish an enemy unit at a
// battlefield with 3 might or less."
fn mel_spells(g: &Game, pid: usize) -> Vec<O> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = vec![];
    let h = g.p[pid].hand.clone();
    let ks: Vec<f64> = h.iter().map(|&c| cval(g, c)).collect();
    for c in sort_keys(h, ks) {
        if stype(g, c) == "Spell" && !seen.contains(g.cname(c)) {
            seen.insert(g.cname(c).to_string());
            out.push(c);
        }
    }
    out
}

fn c_mel_defiant_soul() -> Impl {
    let on_event = f("_mel_event", |g, a| {
        let (o, ev, info) = ev_args(a);
        if ev == "empowered" && is_obj(info, o) {
            let res = f("_mel_res", |g_, a_| {
                if let Some(u) = g_.legal(a_[0].item(), 0)? {
                    g_.to_zone(u, "banish", false, None)?;
                }
                Ok(Value::None)
            });
            let opts = f("_mel_opts", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                let mut us = vec![];
                for u in en_bf(g_, c)? {
                    if g_.might(u)? <= 3 {
                        us.push(u);
                    }
                }
                Ok(us.into())
            });
            let pred = f("_mel_pred", |g_, a_| {
                let (it, u) = (a_[0].item(), a_[1].obj());
                Ok((p_enemy_bf(g_, it, u) && g_.might(u)? <= 3).into())
            });
            q(g, o, "Mel, Defiant Soul", res, Map::new(), false, trig_target(opts, pred, "target", false, true), Value::None);
        }
        Ok(Value::None)
    });
    let ab = ability(
        "Empower",
        AbilityArgs {
            can: f("_mel_can", |g, a| {
                let (pid, o) = (pid_of(&a[0]), a[1].obj());
                Ok((!g.o(o).empowered && !mel_spells(g, pid).is_empty()).into())
            }),
            choices: f("_mel_choices", |g, a| {
                let pid = pid_of(&a[0]);
                Ok(chv(mel_spells(g, pid).iter().map(|&c| map! { disc = g.o(c).uid }).collect()))
            }),
            extra_cost: f("_mel_discard", |g, a| {
                let (pid, ch) = (pid_of(&a[0]), a[2].map());
                let want = ch.g("disc");
                let c = g.p[pid].hand.iter().copied().find(|&x| Value::from(g.o(x).uid) == want);
                if let Some(c) = c {
                    g.discard(pid, c)?;
                }
                Ok(Value::None)
            }),
            resolve: f("_mel_empower", |g, a| {
                let it = a[0].item();
                if let Some(o) = g.it(it).src.and_then(|u| g.obj(u)) {
                    g.empower(Some(o), None)?;
                }
                Ok(Value::None)
            }),
            ..Default::default()
        },
    );
    Impl { on_event, abilities: vec![ab], ..new_impl("Mel, Defiant Soul") }
}

// Minah Swiftfoot — "When I move to a battlefield, choose one — Each player discards 1. / Each player draws 1."
fn c_minah_swiftfoot() -> Impl {
    Impl {
        on_event: f("_minah", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "move" && is_obj(info, o) && is_bf(info.g("to").opt_loc()) {
                let choose = f("_minah_choose", |g_, a_| {
                    let it = a_[0].item();
                    let c = ctrl(g_, it);
                    let m = g_.ask(c, "minah_mode", vec!["draw".into(), "discard".into()], map! { item = it })?;
                    g_.it_mut(it).data.set("mode", m);
                    Ok(true.into())
                });
                let res = f("_minah_res", |g_, a_| {
                    let it = a_[0].item();
                    for p in [g_.tp, 1 - g_.tp] {
                        if dget(g_, it, "mode").opt_str() == Some("draw") {
                            g_.draw(p, 1)?;
                        } else {
                            discard_n(g_, p, 1)?;
                        }
                    }
                    Ok(Value::None)
                });
                q(g, o, "Minah Swiftfoot", res, Map::new(), false, choose, Value::None);
            }
            Ok(Value::None)
        }),
        ..new_impl("Minah Swiftfoot")
    }
}

// Mindsplitter — "When you play me, choose an opponent. They reveal their hand. Choose a card from it, and they
// discard that card."
fn c_mindsplitter() -> Impl {
    Impl {
        on_play: f("_mindsplitter_play", |g, a| {
            let res = f("_mindsplitter", |g_, a_| {
                let c0 = ctrl(g_, a_[0].item());
                let opp = 1 - c0;
                g_.reveal_hand(opp, c0)?;
                if !g_.p[opp].hand.is_empty() {
                    let h = by_cval_desc(g_, g_.p[opp].hand.clone());
                    let c = g_.ask(c0, "mindsplitter_pick", objv(&h), Map::new())?.obj();
                    g_.discard(opp, c)?;
                }
                Ok(Value::None)
            });
            q0(g, a[0].obj(), "Mindsplitter", res);
            Ok(Value::None)
        }),
        ..new_impl("Mindsplitter")
    }
}

// Mister Root — "[Accelerate] When I move to a battlefield, gain 2 XP."
fn c_mister_root() -> Impl {
    Impl {
        accelerate: true,
        on_event: f("_root", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "move" && is_obj(info, o) && is_bf(info.g("to").opt_loc()) {
                let res = f("_root_res", |g_, a_| {
                    let c = ctrl(g_, a_[0].item());
                    g_.gain_xp(c, 2)?;
                    Ok(Value::None)
                });
                q0(g, o, "Mister Root", res);
            }
            Ok(Value::None)
        }),
        ..new_impl("Mister Root")
    }
}

// Morbid Return — "[Action] Return a unit from your trash to your hand."  (targets a card in the trash, 355.9.a)
fn c_morbid_return() -> Impl {
    Impl {
        timing: Some("action"),
        resolve: f("_morbid", |g, a| {
            let it = a[0].item();
            let (c0, want) = (ctrl(g, it), dget(g, it, "c"));
            let c = g.p[c0].trash.iter().copied().find(|&x| Value::from(g.o(x).uid) == want);
            if let Some(c) = c {
                g.to_zone(c, "hand", false, None)?;
            }
            Ok(Value::None)
        }),
        choices: f("_morbid_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let cs = trash_units(g, pid, |_, _| true);
            Ok(chv(cs.iter().map(|&c| map! { c = g.o(c).uid }).collect()))
        }),
        ..new_impl("Morbid Return")
    }
}

// ====================================================================== O / P
// Overzealous Fan — "When I defend, you may kill me to move an attacking unit to its base."
fn c_overzealous_fan() -> Impl {
    Impl {
        on_event: f("_fan", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "defend" && is_obj(info, o) {
                let cost = f("_fan_cost", |g_, a_| {
                    let it = a_[0].item();
                    let Some(m) = me(g_, it) else { return Ok(false.into()) };
                    let c = ctrl(g_, it);
                    g_.kill(&[m], Some(c), true)?;
                    Ok(true.into())
                });
                let res = f("_fan_res", |g_, a_| {
                    let it = a_[0].item();
                    if let Some(u) = g_.legal(it, 0)? {
                        let c = ctrl(g_, it);
                        mv(g_, u, Loc::Base, c)?;
                    }
                    Ok(Value::None)
                });
                let opts = f("_fan_opts", |g_, a_| {
                    let c = ctrl(g_, a_[0].item());
                    let us: Vec<O> = en(g_, c)?.into_iter().filter(|&u| g_.o(u).desig == Some(Desig::Att)).collect();
                    Ok(us.into())
                });
                let pred = f("_fan_pred", |g_, a_| Ok((g_.o(a_[1].obj()).desig == Some(Desig::Att)).into()));
                q(g, o, "Overzealous Fan", res, Map::new(), true, trig_target(opts, pred, "target", false, true), cost);
            }
            Ok(Value::None)
        }),
        ..new_impl("Overzealous Fan")
    }
}

// Pack of Wonders — "[Hidden] [E]: Return another friendly gear, unit, or [Hidden] card to its owner's hand."
fn pack_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, o) = (pid_of(&a[0]), a[1].obj());
    let bd = g.board.clone();
    let bd = by_value_desc(g, bd)?;
    let mut out: Vec<Map> = bd
        .iter()
        .copied()
        .filter(|&x| g.o(x).ctrl == pid && x != o && matches!(stype(g, x), "Unit" | "Gear"))
        .map(|x| map! { tg = uids_t(g, &[x]) })
        .collect();
    for b in &g.bfs {
        for &c in &b.facedowns {
            if g.o(c).owner == pid {
                out.push(map! { fd = b.idx, fdu = g.o(c).uid });
            }
        }
    }
    Ok(chv(out))
}

fn pack(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let fd = dget(g, it, "fd");
    if !fd.is_none() {
        let b = fd.int() as usize;
        let fdu = g.it(it).data.get("fdu").cloned();
        let c0 = ctrl(g, it);
        for c in g.bfs[b].facedowns.clone() {
            let want = fdu.clone().unwrap_or(Value::from(g.o(c).uid));
            if Value::from(g.o(c).uid) == want && g.o(c).owner == c0 {
                g.to_zone(c, "hand", false, None)?;
                break;
            }
        }
        return Ok(Value::None);
    }
    let u = g.legal(it, 0)?;
    to_hand(g, u)?;
    Ok(Value::None)
}

// Pack of Wonders n'a pas le mot-clé Hidden (il le mentionne seulement, image OGN 181 vérifiée) : ne se cache pas (811.1).
fn c_pack_of_wonders() -> Impl {
    let pred = f("_pack_pred", |g, a| {
        let (it, x) = (a[0].item(), a[1].obj());
        Ok((g.o(x).ctrl == g.it(it).ctrl && Some(g.o(x).uid) != g.it(it).src).into())
    });
    let ab = ability(
        "Return",
        AbilityArgs {
            exhaust: true,
            choices: f("_pack_choices", pack_choices),
            resolve: f("_pack", pack),
            preds: Value::List(vec![pred]),
            ..Default::default()
        },
    );
    Impl { abilities: vec![ab], ..new_impl("Pack of Wonders") }
}

// Possession — "[Action] Choose an enemy unit at a battlefield. Take control of it and recall it."
fn c_possession() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_enemy_bf_v()]),
        resolve: f("_possession", |g, a| {
            let it = a[0].item();
            if g.legal(it, 0)?.is_some() {
                if let Some(u) = g.legal(it, 0)? {
                    let c = ctrl(g, it);
                    take_control(g, c, u);
                }
            }
            Ok(Value::None)
        }),
        choices: f("_possession_choices", |g, a| {
            let es = en_bf(g, pid_of(&a[0]))?;
            Ok(tgv(g, &es))
        }),
        ..new_impl("Possession")
    }
}

// Pyke, Returned — "[Hidden] [Backline] Once each turn, when an enemy unit dies while I'm at a battlefield, play a
// Gold gear token exhausted."
fn c_pyke_returned() -> Impl {
    Impl {
        hidden: true,
        kw: kw1("Backline"),
        on_event: f("_pyke", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "die" && is_bf(g.o(o).loc) {
                let inf = info.g("info");
                let inf = inf.map();
                if pid_of(&inf.g("ctrl")) != g.o(o).ctrl && spec(inf.g("spec").str()).type_ == "Unit" {
                    let key = stat_key("pyke", g, o);
                    if g.stats.get(&key) != 0 {
                        return Ok(Value::None);
                    }
                    let cur = g.stats.get(&key);
                    g.stats.add(&key, 1 - cur);
                    q0(g, o, "Pyke, Returned", gold_res());
                }
            }
            Ok(Value::None)
        }),
        ..new_impl("Pyke, Returned")
    }
}

// ====================================================================== R
// Ravenbloom Prefect — "When an opponent plays a gear, you may banish me to banish it."
fn c_ravenbloom_prefect() -> Impl {
    Impl {
        on_event: f("_ravenbloom", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "played" && pid_of(&info.g("pid")) != g.o(o).ctrl && stype(g, info.g("card").obj()) == "Gear" {
                let gear = info.g("card").obj();
                let cost = f("_ravenbloom_cost", |g_, a_| {
                    let Some(m) = me(g_, a_[0].item()) else { return Ok(false.into()) };
                    g_.to_zone(m, "banish", false, None)?;
                    Ok(true.into())
                });
                let res = f("_ravenbloom_res", |g_, a_| {
                    let it = a_[0].item();
                    if let Some(x) = same(g_, &dget(g_, it, "uid"), &dget(g_, it, "oid")) {
                        g_.to_zone(x, "banish", false, None)?;
                    }
                    Ok(Value::None)
                });
                let data = map! { uid = g.o(gear).uid, oid = g.o(gear).oid };
                q(g, o, "Ravenbloom Prefect", res, data, true, Value::None, cost);
            }
            Ok(Value::None)
        }),
        ..new_impl("Ravenbloom Prefect")
    }
}

// Rebuke — "[Action] Return a unit at a battlefield to its owner's hand."
fn c_rebuke() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_bf_v()]),
        resolve: f("_rebuke", |g, a| {
            let u = g.legal(a[0].item(), 0)?;
            to_hand(g, u)?;
            Ok(Value::None)
        }),
        choices: f("_rebuke_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let mut us = en_bf(g, pid)?;
            us.extend(fr_bf(g, pid)?);
            Ok(tgv(g, &us))
        }),
        ..new_impl("Rebuke")
    }
}

// Rhasa the Sunderer — "I cost [1] less for each card in your trash."
fn c_rhasa_the_sunderer() -> Impl {
    Impl {
        cost_mod: f("_rhasa", |g, a| {
            let n = g.p[pid_of(&a[0])].trash.len();
            Ok(Value::Tuple(vec![n.into(), 0.into()]))
        }),
        ..new_impl("Rhasa the Sunderer")
    }
}

// Ride The Wind — "[Action] Move a friendly unit and ready it."
fn c_ride_the_wind() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_friend_v()]),
        resolve: f("_ride", |g, a| {
            let it = a[0].item();
            let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
            let dest = dget(g, it, "dest").loc();
            if g.o(u).loc != Some(dest) {
                let c = ctrl(g, it);
                mv(g, u, dest, c)?;
            }
            g.ready_obj(u, None, None)?;
            Ok(Value::None)
        }),
        choices: f("_ride_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let mut out: Vec<(O, Loc)> = vec![];
            for u in fr(g, pid)? {
                for d in dests(g, u) {
                    out.push((u, d));
                }
            }
            let mut ks = vec![];
            for &(u, d) in &out {
                let good = matches!(d, Loc::Bf(i) if g.bfs[i].ctrl != Some(pid));
                ks.push((!good, -value(g, u)?));
            }
            let out = sort_keys(out, ks);
            Ok(chv(out.iter().map(|&(u, d)| map! { tg = uids_t(g, &[u]), dest = d }).collect()))
        }),
        max_choices: 10,
        ..new_impl("Ride The Wind")
    }
}

// ====================================================================== S
// Scryer's Bloom — "This enters exhausted. Kill this, [1], [E]: [Predict 2], then draw 1. Gain 1 XP."
fn c_scryers_bloom() -> Impl {
    let res = f("_scry", |g, a| {
        let c = ctrl(g, a[0].item());
        g.predict(c, 2)?;
        g.draw(c, 1)?;
        g.gain_xp(c, 1)?;
        Ok(Value::None)
    });
    let ab = ability(
        "Scry",
        AbilityArgs {
            cost: Cost::Text("1 energy".into()),
            exhaust: true,
            kill_self: true,
            resolve: res,
            ..Default::default()
        },
    );
    Impl { enter_exhausted: true, abilities: vec![ab], ..new_impl("Scryer's Bloom") }
}

// Seal of Discord — "[E]: [Reaction] — [Add] [Chaos]."
fn c_seal_of_discord() -> Impl {
    Impl { add: vec![add_ability("1 chaos rune", true, false, Value::None)], ..new_impl("Seal of Discord") }
}

// Shadow Order Disciple — "When I move, you may [Burn 1] to give me +1 might this turn."
fn c_shadow_order_disciple() -> Impl {
    Impl {
        on_event: f("_disciple", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "move" && is_obj(info, o) {
                let cost = f("_disciple_cost", |g_, a_| {
                    let c = ctrl(g_, a_[0].item());
                    g_.burn(c, 1)?;
                    Ok(true.into())
                });
                q(g, o, "Shadow Order Disciple", buff_me(1), Map::new(), true, Value::None, cost);
            }
            Ok(Value::None)
        }),
        ..new_impl("Shadow Order Disciple")
    }
}

// Shadowblade Lurker — "I cost [2] less for each card with my name in your trash."
fn c_shadowblade_lurker() -> Impl {
    Impl {
        cost_mod: f("_lurker", |g, a| {
            let pid = pid_of(&a[0]);
            let n = g.p[pid].trash.iter().filter(|&&x| g.cname(x) == "Shadowblade Lurker").count();
            Ok(Value::Tuple(vec![(2 * n).into(), 0.into()]))
        }),
        ..new_impl("Shadowblade Lurker")
    }
}

// Shadows of the Past — "Return up to 2 units from trashes to their owners' hands."
fn c_shadows_of_the_past() -> Impl {
    Impl {
        resolve: f("_shadows", |g, a| {
            let it = a[0].item();
            let cs = dget(g, it, "cs");
            for u in cs.list() {
                let c = g.p.iter().flat_map(|pl| pl.trash.iter().copied()).find(|&x| Value::from(g.o(x).uid) == *u);
                if let Some(c) = c {
                    g.to_zone(c, "hand", false, None)?;
                }
            }
            Ok(Value::None)
        }),
        choices: f("_shadows_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let mut cs = trash_units(g, pid, |_, _| true);
            cs.extend(trash_units(g, 1 - pid, |_, _| true));
            let cs = cap(g, cs, 4, Some(pid));
            let mut out: Vec<Map> = combinations(&cs, 2).iter().map(|c| map! { cs = uids_t(g, c) }).collect();
            out.extend(cs.iter().map(|&c| map! { cs = uids_t(g, &[c]) }));
            let mut out = cap(g, out, 7, Some(pid));
            out.push(empty_choice("cs"));
            Ok(chv(out))
        }),
        ..new_impl("Shadows of the Past")
    }
}

// Sinister Poro — "When I attack, you may pay [1] to move an enemy unit here to its base."
fn c_sinister_poro() -> Impl {
    Impl {
        on_event: f("_sinister", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "attack" && is_obj(info, o) {
                let here = g.o(o).loc;
                let res = f("_sinister_res", |g_, a_| {
                    let it = a_[0].item();
                    if let Some(u) = g_.legal(it, 0)? {
                        let c = ctrl(g_, it);
                        mv(g_, u, Loc::Base, c)?;
                    }
                    Ok(Value::None)
                });
                let opts = f("_sinister_opts", move |g_, a_| {
                    let c = ctrl(g_, a_[0].item());
                    let us: Vec<O> = en(g_, c)?.into_iter().filter(|&u| g_.o(u).loc == here).collect();
                    Ok(us.into())
                });
                let pred = f("_sinister_pred", move |g_, a_| {
                    let (it, u) = (a_[0].item(), a_[1].obj());
                    Ok((g_.o(u).ctrl != g_.it(it).ctrl && g_.o(u).loc == here).into())
                });
                let choose = trig_target(opts, pred, "target", false, true);
                q(g, o, "Sinister Poro", res, Map::new(), true, choose, may_pay(1, Value::None));
            }
            Ok(Value::None)
        }),
        ..new_impl("Sinister Poro")
    }
}

// Soulgorger — "When you play me, you may play a unit from your trash, ignoring its Energy cost. (You must still
// pay its Power cost.)"
fn from_trash_ignore_energy(g: &mut Game, pid: usize, kind: &str, may: bool) -> Res<()> {
    let mut cands = vec![];
    for c in trash_units(g, pid, |_, _| true) {
        if power_ok(g, pid, c)? {
            cands.push(c);
        }
    }
    if cands.is_empty() {
        return Ok(());
    }
    let mut ov = objv(&cands);
    if may {
        ov.push(Value::None);
    }
    let pick = g.ask(pid, kind, ov, Map::new())?;
    if let Some(p) = pick.opt_obj() {
        play_unit_free(g, pid, p, "trash", None, true, None)?;
    }
    Ok(())
}

fn c_soulgorger() -> Impl {
    Impl {
        on_play: f("_soulgorger", |g, a| {
            let res = f("_soulgorger_res", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                from_trash_ignore_energy(g_, c, "soulgorger_pick", true)?;
                Ok(Value::None)
            });
            q0(g, a[0].obj(), "Soulgorger", res);
            Ok(Value::None)
        }),
        ..new_impl("Soulgorger")
    }
}

// Spiderling — "[Hidden] I have +1 might for each other unit you control here with my name. Your deck can have any
// number of cards named Spiderling."
fn c_spiderling() -> Impl {
    Impl {
        hidden: true,
        might_mod: f("_spiderling", |g, a| {
            let o = a[0].obj();
            let n = g
                .units(Some(g.o(o).ctrl), g.o(o).loc)
                .iter()
                .filter(|&&u| u != o && g.cname(u) == "Spiderling")
                .count();
            Ok(n.into())
        }),
        ..new_impl("Spiderling")
    }
}

// Spirit Wheel — "When you choose a friendly unit, you may pay [1] and exhaust this to draw 1."
fn c_spirit_wheel() -> Impl {
    Impl {
        on_event: f("_wheel", |g, a| {
            let (o, ev, info) = ev_args(a);
            let u = if ev == "chosen" { info.g("obj").opt_obj() } else { None };
            let Some(u) = u else { return Ok(Value::None) };
            if g.it(info.g("item").item()).ctrl == g.o(o).ctrl
                && g.o(u).ctrl == g.o(o).ctrl
                && stype(g, u) == "Unit"
                && !g.exhausted(o)
            {
                let cost = f("_wheel_cost", |g_, a_| {
                    let it = a_[0].item();
                    let c = ctrl(g_, it);
                    let Some(m) = me(g_, it) else { return Ok(false.into()) };
                    if g_.exhausted(m) || !g_.can_pay(c, 1, &[], &Value::None)? {
                        return Ok(false.into());
                    }
                    g_.pay(c, 1, &[], &Value::None)?;
                    g_.set_exhausted(m, true);
                    Ok(true.into())
                });
                q(g, o, "Spirit Wheel", draw1_res(), Map::new(), true, Value::None, cost);
            }
            Ok(Value::None)
        }),
        ..new_impl("Spirit Wheel")
    }
}

// Stacked Deck — "[Action] Look at the top 3 cards of your Main Deck. Put 1 into your hand and recycle the rest."
fn c_stacked_deck() -> Impl {
    Impl {
        timing: Some("action"),
        resolve: f("_stacked", |g, a| {
            let pid = ctrl(g, a[0].item());
            let d3: Vec<O> = g.p[pid].deck.iter().take(3).copied().collect();
            let top = g.look(pid, &d3, None)?;
            if top.is_empty() {
                return Ok(Value::None);
            }
            let st = by_cval_desc(g, top.clone());
            let c = g.ask(pid, "stacked_pick", objv(&st), Map::new())?.obj();
            if let Some(i) = g.p[pid].deck.iter().position(|&x| x == c) {
                g.p[pid].deck.remove(i);
            }
            g.o_mut(c).zone = Some(Zone::Hand);
            g.p[pid].hand.push(c);
            let rest: Vec<O> = top.into_iter().filter(|&x| x != c).collect();
            g.recycle_cards(pid, &rest)?;
            Ok(Value::None)
        }),
        ..new_impl("Stacked Deck")
    }
}

// Star-Crossed — "[Reaction] Return a friendly unit and an enemy unit to their owners' hands."
fn c_star_crossed() -> Impl {
    Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![p_friend_v(), p_enemy_v()]),
        resolve: f("_star_crossed", |g, a| {
            let it = a[0].item();
            let us = [g.legal(it, 0)?, g.legal(it, 1)?];
            for u in us {
                to_hand(g, u)?;
            }
            Ok(Value::None)
        }),
        choices: f("_star_crossed_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let es = en(g, pid)?;
            let es = cap(g, es, 4, Some(pid));
            let mut out = vec![];
            for &e in &es {
                let us = g.units(Some(pid), None);
                let us = by_value(g, us)?;
                for x in cap(g, us, 2, Some(pid)) {
                    out.push(map! { tg = uids_t(g, &[x, e]) });
                }
            }
            Ok(chv(out))
        }),
        ..new_impl("Star-Crossed")
    }
}

// Stealthy Pursuer — "When a friendly unit moves from my location, I may be moved with it."
fn c_stealthy_pursuer() -> Impl {
    Impl {
        on_event: f("_pursuer", |g, a| {
            let (o, ev, info) = ev_args(a);
            let u = if ev == "move" { info.g("obj").opt_obj() } else { None };
            let Some(u) = u else { return Ok(Value::None) };
            if u != o && g.o(u).ctrl == g.o(o).ctrl && info.g("frm").opt_loc() == g.o(o).loc {
                let dest = info.g("to").loc();
                let res = f("_pursuer_res", move |g_, a_| {
                    let it = a_[0].item();
                    if let Some(m) = me(g_, it) {
                        if g_.o(m).loc != Some(dest) {
                            let c = ctrl(g_, it);
                            mv(g_, m, dest, c)?;
                        }
                    }
                    Ok(Value::None)
                });
                q(g, o, "Stealthy Pursuer", res, Map::new(), true, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..new_impl("Stealthy Pursuer")
    }
}

// Switcheroo — "[Hidden] [Action] Swap the Might of two units at the same battlefield this turn." (rule 433)
fn swap(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let (x, y) = (g.legal(it, 0)?, g.legal(it, 1)?);
    let (Some(x), Some(y)) = (x, y) else { return Ok(Value::None) };
    if g.o(x).loc != g.o(y).loc || !is_bf(g.o(x).loc) {
        return Ok(Value::None);
    }
    let (ma, mb) = (g.might(x)?, g.might(y)?);
    if ma == mb {
        return Ok(Value::None);
    }
    let (lo, hi) = if ma < mb { (x, y) } else { (y, x) };
    let d = (ma - mb).abs();
    g.mod_(lo, d, "turn".into(), None)?;
    g.mod_(hi, -d, "turn".into(), None)?;
    Ok(Value::None)
}

fn swap_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, hb) = (pid_of(&a[0]), a[1].map().g("hidden_bf").opt_loc());
    let mut out: Vec<(O, O)> = vec![];
    let bfs = match hb {
        Some(h) => vec![h],
        None => g.bf_locs(),
    };
    for bf in bfs {
        let mut us = vec![];
        for u in g.units(None, Some(bf)) {
            if g.targetable(u, Some(pid))? {
                us.push(u);
            }
        }
        for c in combinations(&us, 2) {
            // deux unités de même Might sont des cibles légales (l'échange ne change rien) : proposées au joueur
            // humain, pas à l'IA pour qui ce choix ne sert à rien
            if g.might(c[0])? != g.might(c[1])? || full_choices(g, Some(pid)) {
                out.push((c[0], c[1]));
            }
        }
    }
    let mut ks = vec![];
    for &(x, y) in &out {
        let k = if g.o(x).ctrl == g.o(y).ctrl {
            0
        } else {
            let (fo, eo) = if g.o(x).ctrl == pid { (x, y) } else { (y, x) };
            -(g.might(eo)? - g.might(fo)?)
        };
        ks.push(k);
    }
    let out = sort_keys(out, ks);
    Ok(chv(out.iter().map(|&(x, y)| map! { tg = uids_t(g, &[x, y]) }).collect()))
}

fn c_switcheroo() -> Impl {
    Impl {
        hidden: true,
        timing: Some("action"),
        preds: Value::List(vec![p_unit_bf_v(), p_unit_bf_v()]),
        resolve: f("_swap", swap),
        choices: f("_swap_choices", swap_choices),
        ..new_impl("Switcheroo")
    }
}

// ====================================================================== T
// Tail-Cloaked Matriarch — "[Empower] [2][Chaos] When I become [Empowered], you may choose a unit in your trash with
// Energy cost no more than [3] and Power cost no more than [A]. Play it to your base, ignoring its cost."
fn c_tail_cloaked_matriarch() -> Impl {
    let mut im = Impl {
        on_event: f("_matriarch", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "empowered" && is_obj(info, o) {
                let res = f("_matriarch_res", |g_, a_| {
                    let c0 = ctrl(g_, a_[0].item());
                    let cands = trash_units(g_, c0, |g2, c| g2.spec(c).e <= 3 && g2.spec(c).p <= 1);
                    let pick = if !cands.is_empty() {
                        let mut ov = objv(&cands);
                        ov.push(Value::None);
                        g_.ask(c0, "matriarch_pick", ov, Map::new())?
                    } else {
                        Value::None
                    };
                    if let Some(p) = pick.opt_obj() {
                        play_unit_free(g_, c0, p, "trash", Some(vec![Loc::Base]), false, None)?;
                    }
                    Ok(Value::None)
                });
                q0(g, o, "Tail-Cloaked Matriarch", res);
            }
            Ok(Value::None)
        }),
        ..new_impl("Tail-Cloaked Matriarch")
    };
    impl_empower(&mut im, Cost::Text("2 energy and 1 chaos rune".into()));
    im
}

// Teemo, Scout — "[Hidden] When you play me, give me +3 might this turn."
fn c_teemo_scout() -> Impl {
    Impl {
        hidden: true,
        on_play: f("_teemo", |g, a| {
            q0(g, a[0].obj(), "Teemo, Scout", buff_me(3));
            Ok(Value::None)
        }),
        ..new_impl("Teemo, Scout")
    }
}

// Temptation — "[Repeat] [2] Move an enemy unit to a location where there's a unit with the same controller."
// The repetition chooses its own unit and destination (rule 820.2.a): choice keys rep2 / tg2 / dest2, priced by
// extra_cost_fn (with the Deflect of the second target).
fn tempt_dests(g: &Game, u: O) -> Vec<Loc> {
    g.locs()
        .into_iter()
        .filter(|&d| Some(d) != g.o(u).loc && g.units(Some(g.o(u).ctrl), Some(d)).iter().any(|&x| x != u))
        .collect()
}

fn tempt_once(g: &mut Game, it: ItemId, u: Option<O>, dest: Option<Loc>) -> Res<()> {
    if let Some(u) = u {
        let c = ctrl(g, it);
        if g.o(u).ctrl != c {
            if let Some(d) = dest {
                if tempt_dests(g, u).contains(&d) {
                    mv(g, u, d, c)?;
                }
            }
        }
    }
    Ok(())
}

fn temptation(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let u = g.legal(it, 0)?;
    tempt_once(g, it, u, dget(g, it, "dest").opt_loc())?;
    let t2 = dget(g, it, "t2");
    if dget(g, it, "rep2").truthy() && t2.truthy() {
        let p = &t2.list()[0];
        let mut u2 = same(g, &p.list()[0], &p.list()[1]);
        let c = ctrl(g, it);
        if let Some(x) = u2 {
            if !g.targetable(x, Some(c))? {
                u2 = None;
            }
        }
        tempt_once(g, it, u2, dget(g, it, "dest2").opt_loc())?;
    }
    Ok(Value::None)
}

fn tempt_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = pid_of(&a[0]);
    let mut base: Vec<(O, Loc)> = vec![];
    for u in en(g, pid)? {
        for d in tempt_dests(g, u) {
            base.push((u, d));
        }
    }
    let mut ks = vec![];
    for &(u, d) in &base {
        ks.push((d == Loc::Base, -value(g, u)?));
    }
    let base = sort_keys(base, ks);
    let base: Vec<Map> = base.iter().map(|&(u, d)| map! { tg = uids_t(g, &[u]), dest = d }).collect();
    let mut reps = vec![];
    for x in cap(g, base.clone(), 2, Some(pid)) {
        for y in cap(g, base.clone(), 2, Some(pid)) {
            let mut m = x.clone();
            m.set("rep2", true);
            m.set("tg2", y.g("tg"));
            m.set("dest2", y.g("dest"));
            reps.push(m);
        }
    }
    // variantes avec répétition parmi les 8 premiers choix
    let k = base.len().min(3);
    let mut out: Vec<Map> = base[..k].to_vec();
    out.extend(reps);
    out.extend(base[k..].iter().cloned());
    Ok(chv(out))
}

/// `_tempt_parts` : le coût du Repeat (chaque coût est un coût additionnel optionnel séparé) et le Deflect de la
/// cible de la répétition.
fn tempt_parts(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, ch) = (pid_of(&a[0]), a[2].map());
    if !ch.g("rep2").truthy() {
        return Ok(Value::List(vec![]));
    }
    let tg2 = ch.get("tg2").cloned().unwrap_or(Value::Tuple(vec![]));
    let dr = deflect_reqs(g, pid, &map! { tg = tg2 })?;
    Ok(Value::List(vec![
        Value::Tuple(vec!["rep".into(), 2.into(), reqs_to_value(&[])]),
        Value::Tuple(vec!["must".into(), 0.into(), reqs_to_value(&dr)]),
    ]))
}

fn c_temptation() -> Impl {
    Impl {
        preds: Value::List(vec![p_enemy_v()]),
        resolve: f("_temptation", temptation),
        choices: f("_tempt_choices", tempt_choices),
        opt_parts: f("_tempt_parts", tempt_parts),
        ..new_impl("Temptation")
    }
}

// The Harrowing — "Play a unit from your trash, ignoring its Energy cost. (You must still pay its Power cost.)"
fn c_the_harrowing() -> Impl {
    Impl {
        resolve: f("_harrowing", |g, a| {
            let c = ctrl(g, a[0].item());
            from_trash_ignore_energy(g, c, "harrowing_pick", false)?;
            Ok(Value::None)
        }),
        ..new_impl("The Harrowing")
    }
}

// The List — "As you play this, name a tag. [E]: Give a unit with the named tag -2 might this turn."
// L'attribut Python `o.named_tag = (oid, tag)` est rangé dans `attrs["named_tag"]`.
/// `_all_tags()` : tous les tags de la base, triés
fn all_tags() -> &'static Vec<String> {
    static T: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let s: BTreeSet<String> = crate::spec::specs().values().flat_map(|s| s.tags.iter().cloned()).collect();
        s.into_iter().collect()
    })
}

fn list_play(g: &mut Game, a: &[Value]) -> Res<Value> {
    let o = a[0].obj();
    let c = g.o(o).ctrl;
    let mut seen: Vec<String> = vec![];
    for u in enemies(g, c, false, None, false)? {
        for t in g.tags(u)? {
            if !seen.contains(&t) {
                seen.push(t);
            }
        }
    }
    let mut tags: Vec<Value> = seen.iter().map(|t| Value::from(t.as_str())).collect();
    tags.extend(all_tags().iter().filter(|t| !seen.contains(t)).map(|t| Value::from(t.as_str())));
    let t = g.ask(c, "name_tag", tags, Map::new())?; // « As you play this » (règle 355.1)
    let v = Value::Tuple(vec![g.o(o).oid.into(), t]);
    g.o_mut(o).attrs.set("named_tag", v);
    Ok(Value::None)
}

/// `_list_tag(o)`
fn list_tag(g: &Game, o: O) -> Option<String> {
    let t = g.o(o).attrs.g("named_tag");
    if t.truthy() && t.list()[0] == Value::from(g.o(o).oid) {
        return t.list()[1].opt_str().map(|s| s.to_string());
    }
    None
}

/// `_list_tag(o) in g.tags(u)`
fn has_list_tag(g: &mut Game, o: O, u: O) -> Res<bool> {
    match list_tag(g, o) {
        Some(t) => Ok(g.tags(u)?.contains(&t)),
        None => Ok(false),
    }
}

fn c_the_list() -> Impl {
    let ab = ability(
        "Mark",
        AbilityArgs {
            exhaust: true,
            choices: f("_list_choices", |g, a| {
                let (pid, o) = (pid_of(&a[0]), a[1].obj());
                let mut us = en(g, pid)?;
                us.extend(fr(g, pid)?);
                let mut out = vec![];
                for u in us {
                    if has_list_tag(g, o, u)? {
                        out.push(u);
                    }
                }
                Ok(tgv(g, &out))
            }),
            preds: Value::List(vec![f("_list_pred", |g, a| {
                let (it, u) = (a[0].item(), a[1].obj());
                match g.it(it).src.and_then(|s| g.obj(s)) {
                    Some(o) => Ok(has_list_tag(g, o, u)?.into()),
                    None => Ok(false.into()),
                }
            })]),
            resolve: f("_list_mark", |g, a| {
                let it = a[0].item();
                if g.legal(it, 0)?.is_some() {
                    if let Some(u) = g.legal(it, 0)? {
                        g.mod_(u, -2, "turn".into(), None)?;
                    }
                }
                Ok(Value::None)
            }),
            ..Default::default()
        },
    );
    Impl { on_play: f("_list_play", list_play), abilities: vec![ab], ..new_impl("The List") }
}

// The Syren — "[1], [E]: Move a friendly unit at a battlefield to your base."
fn c_the_syren() -> Impl {
    let ab = ability(
        "Recall",
        AbilityArgs {
            cost: Cost::Text("1 energy".into()),
            exhaust: true,
            preds: Value::List(vec![p_friend_bf_v()]),
            choices: f("_syren_choices", |g, a| {
                let us = fr_bf(g, pid_of(&a[0]))?;
                Ok(tgv(g, &us))
            }),
            resolve: f("_syren", |g, a| {
                let it = a[0].item();
                if g.legal(it, 0)?.is_some() {
                    let u = g.legal(it, 0)?;
                    let c = ctrl(g, it);
                    g.move_(&u.into_iter().collect::<Vec<O>>(), Loc::Base, Some(c), false)?;
                }
                Ok(Value::None)
            }),
            ..Default::default()
        },
    );
    Impl { abilities: vec![ab], ..new_impl("The Syren") }
}

// Tideturner — "[Hidden] When you play me, you may choose a friendly unit. Move me to its location and it to my
// original location."  (its target may be chosen freely when played from Hidden, rule 811.1.d.2 example)
fn c_tideturner() -> Impl {
    Impl {
        hidden: true,
        on_play: f("_tideturner", |g, a| {
            let o = a[0].obj();
            let here = g.o(o).loc;
            let res = f("_tideturner_res", |g_, a_| {
                let it = a_[0].item();
                let u = g_.legal(it, 0)?;
                let m = me(g_, it);
                let (Some(u), Some(m)) = (u, m) else { return Ok(Value::None) };
                if g_.o(u).loc == g_.o(m).loc {
                    return Ok(Value::None);
                }
                let (orig, there) = (g_.o(m).loc.unwrap(), g_.o(u).loc.unwrap());
                let c = ctrl(g_, it);
                mv(g_, m, there, c)?;
                mv(g_, u, orig, c)?;
                Ok(Value::None)
            });
            let opts = f("_tideturner_opts", move |g_, a_| {
                let it = a_[0].item();
                let (c, src) = (ctrl(g_, it), g_.it(it).src);
                let ln = loc_now(g_, it, here);
                let us: Vec<O> =
                    fr(g_, c)?.into_iter().filter(|&u| Some(g_.o(u).uid) != src && g_.o(u).loc != ln).collect();
                Ok(us.into())
            });
            let pred = f("_tideturner_pred", |g_, a_| {
                let (it, u) = (a_[0].item(), a_[1].obj());
                Ok((g_.o(u).ctrl == g_.it(it).ctrl && Some(g_.o(u).uid) != g_.it(it).src).into())
            });
            let choose = trig_target(opts, pred, "friend_target", false, true);
            q(g, o, "Tideturner", res, Map::new(), true, choose, Value::None);
            Ok(Value::None)
        }),
        ..new_impl("Tideturner")
    }
}

// Tornado Warrior — "[Hidden] When you play me from face down, you may empower something here. Disempower it at end
// of turn."
fn tornado(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ctx) = (a[0].obj(), a[1].map());
    if ctx.g("src").opt_str() != Some("facedown") {
        return Ok(Value::None);
    }
    let here = ctx.g("hidden_bf").opt_loc();
    let res = f("_tornado_res", |g_, a_| {
        let it = a_[0].item();
        let Some(x) = g_.legal(it, 0)? else { return Ok(Value::None) };
        if !g_.empower(Some(x), None)? {
            return Ok(Value::None);
        }
        let (xu, xo, pid) = (g_.o(x).uid, g_.o(x).oid, ctrl(g_, it));
        let at_end = f("_tornado_end", move |g2, a2| {
            g2.remove_effect(a2[0].int() as u64);
            let dis = f("_tornado_dis", move |g3, _a3| {
                if let Some(y) = same(g3, &Value::from(xu), &Value::from(xo)) {
                    g3.disempower(Some(y));
                }
                Ok(Value::None)
            });
            g2.queue_trigger(pid, "Tornado Warrior (end of turn)", dis, Map::new(), None, false, Value::None, Value::None);
            Ok(Value::None)
        });
        let mut d = Map::new();
        d.set("on", "end_turn");
        d.set("fn", at_end);
        d.set("dur", "turn");
        g_.add_effect(d);
        Ok(Value::None)
    });
    let opts = f("_tornado_opts", move |g_, a_| {
        let c = ctrl(g_, a_[0].item());
        let xs: Vec<O> = g_.board.iter().copied().filter(|&x| g_.o(x).loc == here && !g_.o(x).empowered).collect();
        let mut ks = vec![];
        for &x in &xs {
            ks.push((g_.o(x).ctrl != c, -value(g_, x)?));
        }
        Ok(sort_keys(xs, ks).into())
    });
    let pred = f("_tornado_pred", move |g_, a_| Ok((g_.o(a_[1].obj()).loc == here).into()));
    q(g, o, "Tornado Warrior", res, Map::new(), true, trig_target(opts, pred, "empower_target", false, true), Value::None);
    Ok(Value::None)
}

fn c_tornado_warrior() -> Impl {
    Impl { hidden: true, on_play: f("_tornado", tornado), ..new_impl("Tornado Warrior") }
}

// Traveling Merchant — "When I move, discard 1, then draw 1."
fn c_traveling_merchant() -> Impl {
    Impl {
        on_event: f("_merchant", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "move" && is_obj(info, o) {
                q0(g, o, "Traveling Merchant", loot_res());
            }
            Ok(Value::None)
        }),
        ..new_impl("Traveling Merchant")
    }
}

// Treasure Hunter — "When I move, play a Gold gear token exhausted."
fn c_treasure_hunter() -> Impl {
    Impl {
        on_event: f("_hunter", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "move" && is_obj(info, o) {
                q0(g, o, "Treasure Hunter", gold_res());
            }
            Ok(Value::None)
        }),
        ..new_impl("Treasure Hunter")
    }
}

// Twilight Step — "Move a unit with 3 might or less. [Flow] [4][Chaos]"
fn twilight_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = pid_of(&a[0]);
    let mut us = fr(g, pid)?;
    us.extend(en(g, pid)?);
    let mut out: Vec<(O, Loc)> = vec![];
    for u in us {
        if g.might(u)? <= 3 {
            for d in dests(g, u) {
                out.push((u, d));
            }
        }
    }
    let ks: Vec<bool> = out
        .iter()
        .map(|&(u, d)| {
            let good = (g.o(u).ctrl == pid && d.is_bf()) || (g.o(u).ctrl != pid && d == Loc::Base);
            !good
        })
        .collect();
    let out = sort_keys(out, ks);
    Ok(chv(out.iter().map(|&(u, d)| map! { tg = uids_t(g, &[u]), dest = d }).collect()))
}

fn c_twilight_step() -> Impl {
    Impl {
        flow: flow_cost("4 energy and 1 chaos rune"),
        preds: Value::List(vec![f("_twilight_pred", |g, a| {
            let (it, o) = (a[0].item(), a[1].obj());
            Ok((p_unit(g, it, o) && g.might(o)? <= 3).into())
        })]),
        choices: f("_twilight_choices", twilight_choices),
        max_choices: 10,
        resolve: f("_twilight", |g, a| {
            let it = a[0].item();
            let dest = dget(g, it, "dest").loc();
            if g.legal(it, 0)?.is_some() && {
                let u = g.legal(it, 0)?.unwrap();
                g.o(u).loc != Some(dest)
            } {
                if let Some(u) = g.legal(it, 0)? {
                    let c = ctrl(g, it);
                    mv(g, u, dest, c)?;
                }
            }
            Ok(Value::None)
        }),
        ..new_impl("Twilight Step")
    }
}

// Twisted Fate, Gambler — "When I attack, reveal the top rune of your rune deck, then recycle it. Do one of the
// following based on its domain: [Fury] — Deal 2 to an enemy unit here and 1 to all other enemy units here.
// [Mind] — Draw 1. [Order] — Stun an enemy unit."  (other domains: nothing)
fn tf_pick(g: &mut Game, it: ItemId, opts: Vec<O>, kind: &str) -> Res<Option<O>> {
    let c = ctrl(g, it);
    let mut ok = vec![];
    for u in opts {
        if deflect_ok(g, c, u)? {
            ok.push(u);
        }
    }
    if ok.is_empty() {
        return Ok(None);
    }
    let u = g.ask(c, kind, objv(&ok), map! { item = it })?.opt_obj();
    let Some(u) = u else { return Ok(None) };
    if !pay_deflect(g, it, u)? {
        return Ok(None);
    }
    Ok(Some(u))
}

fn tf_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c = ctrl(g, it);
    if g.p[c].rune_deck.is_empty() {
        return Ok(Value::None);
    }
    let r = g.p[c].rune_deck.remove(0);
    g.p[c].rune_deck.push(r); // recycler : dessous du deck de runes
    let dom = g.r(r).domain.clone();
    g.log(|| format!("  Twisted Fate reveals a {dom} rune"));
    let here = dget(g, it, "here").opt_loc();
    if dom == "Fury" {
        let os: Vec<O> = en(g, c)?.into_iter().filter(|&u| g.o(u).loc == here).collect();
        if let Some(u) = tf_pick(g, it, os, "target")? {
            let others: Vec<O> = g.units(Some(1 - c), here).into_iter().filter(|&x| x != u).collect();
            g.deal(Some(u), 2, "ability", Some(c), None)?;
            for x in others {
                g.deal(Some(x), 1, "ability", Some(c), None)?;
            }
        }
    } else if dom == "Mind" {
        g.draw(c, 1)?;
    } else if dom == "Order" {
        let es = en(g, c)?;
        let ks: Vec<bool> = es.iter().map(|&x| g.o(x).stunned).collect();
        let es = sort_keys(es, ks);
        if let Some(u) = tf_pick(g, it, es, "target")? {
            g.stun(Some(u), Some(c))?;
        }
    }
    Ok(Value::None)
}

fn c_twisted_fate_gambler() -> Impl {
    Impl {
        on_event: f("_tf", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "attack" && is_obj(info, o) {
                let data = map! { here = g.o(o).loc };
                q(g, o, "Twisted Fate, Gambler", f("_tf_res", tf_res), data, false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..new_impl("Twisted Fate, Gambler")
    }
}

// ====================================================================== U / V / W
// Undercover Agent — "[Deathknell] Discard 2, then draw 2."
fn c_undercover_agent() -> Impl {
    Impl {
        deathknell: f("_undercover", |g, a| {
            let c = ctrl(g, a[0].item());
            discard_n(g, c, 2)?;
            g.draw(c, 2)?;
            Ok(Value::None)
        }),
        ..new_impl("Undercover Agent")
    }
}

// Up from the Deep — "Play two 1 might Tentacle unit tokens. [Flow] [3]"
fn c_up_from_the_deep() -> Impl {
    Impl {
        flow: flow_cost("3 energy"),
        resolve: f("_up_from_the_deep", |g, a| {
            let c = ctrl(g, a[0].item());
            token(g, c, "Tentacle", false)?;
            token(g, c, "Tentacle", false)?;
            Ok(Value::None)
        }),
        ..new_impl("Up from the Deep")
    }
}

// Vicious Snapjaws — "When another friendly unit dies, gain 1 XP."
fn c_vicious_snapjaws() -> Impl {
    Impl {
        on_event: f("_snapjaws", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "die" {
                let inf = info.g("info");
                let inf = inf.map();
                if pid_of(&inf.g("ctrl")) == g.o(o).ctrl
                    && inf.g("obj").opt_obj() != Some(o)
                    && spec(inf.g("spec").str()).type_ == "Unit"
                {
                    let res = f("_snapjaws_res", |g_, a_| {
                        let c = ctrl(g_, a_[0].item());
                        g_.gain_xp(c, 1)?;
                        Ok(Value::None)
                    });
                    q0(g, o, "Vicious Snapjaws", res);
                }
            }
            Ok(Value::None)
        }),
        ..new_impl("Vicious Snapjaws")
    }
}

// Walking Roost — "[Deflect] When you play me, choose an opponent. They play a 1 might Bird unit token with
// [Deflect]."  (the opponent controls the token, rule 182, and plays it to one of their locations)
fn c_walking_roost() -> Impl {
    Impl {
        kw: kw1("Deflect"),
        on_play: f("_roost", |g, a| {
            let res = f("_roost_res", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                token(g_, 1 - c, "Bird", false)?;
                Ok(Value::None)
            });
            q0(g, a[0].obj(), "Walking Roost", res);
            Ok(Value::None)
        }),
        ..new_impl("Walking Roost")
    }
}

// Whirlwind — "Starting with the next player, each player may return a unit to its owner's hand."
fn c_whirlwind() -> Impl {
    Impl {
        resolve: f("_whirlwind", |g, a| {
            let c = ctrl(g, a[0].item());
            for pid in [1 - c, c] {
                let e0 = g.units(Some(1 - pid), None);
                let ens = by_value_desc(g, e0)?;
                let o0 = g.units(Some(pid), None);
                let own = by_value(g, o0)?;
                if ens.is_empty() && own.is_empty() {
                    continue;
                }
                let mut ov = objv(&ens);
                ov.push(Value::None);
                ov.extend(objv(&own));
                let pick = g.ask(pid, "whirlwind_pick", ov, Map::new())?;
                to_hand(g, pick.opt_obj())?;
            }
            Ok(Value::None)
        }),
        ..new_impl("Whirlwind")
    }
}

// Wind and Ghosts — "[Action] Choose a unit at a battlefield. If it has 3 might or less, banish it. Otherwise,
// return it to its owner's hand."
fn c_wind_and_ghosts() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_bf_v()]),
        resolve: f("_wind_ghosts", |g, a| {
            let Some(u) = g.legal(a[0].item(), 0)? else { return Ok(Value::None) };
            if g.might(u)? <= 3 {
                g.to_zone(u, "banish", false, None)?;
            } else {
                to_hand(g, Some(u))?;
            }
            Ok(Value::None)
        }),
        choices: f("_wind_ghosts_choices", |g, a| {
            let pid = pid_of(&a[0]);
            let mut us = en_bf(g, pid)?;
            us.extend(fr_bf(g, pid)?);
            Ok(tgv(g, &us))
        }),
        ..new_impl("Wind and Ghosts")
    }
}

// Windsinger — "Hidden When you play me, you may return another unit at a battlefield with 3 might or less to its
// owner's hand."  (the database text lost the brackets of [Hidden]; the card has the keyword)
fn windsinger_ok(g: &mut Game, it: ItemId, u: O, hb: Option<Loc>) -> Res<bool> {
    Ok(Some(g.o(u).uid) != g.it(it).src
        && is_bf(g.o(u).loc)
        && g.might(u)? <= 3
        && (hb.is_none() || g.o(u).loc == hb))
}

fn c_windsinger() -> Impl {
    Impl {
        hidden: true,
        on_play: f("_windsinger", |g, a| {
            let (o, ctx) = (a[0].obj(), a[1].map());
            let hb = ctx.g("hidden_bf").opt_loc();
            let res = f("_windsinger_res", |g_, a_| {
                let u = g_.legal(a_[0].item(), 0)?;
                to_hand(g_, u)?;
                Ok(Value::None)
            });
            let opts = f("_windsinger_opts", move |g_, a_| {
                let it = a_[0].item();
                let c = ctrl(g_, it);
                let mut us = en_bf(g_, c)?;
                us.extend(fr_bf(g_, c)?);
                let mut out = vec![];
                for u in us {
                    if windsinger_ok(g_, it, u, hb)? {
                        out.push(u);
                    }
                }
                Ok(out.into())
            });
            let pred = f("_windsinger_pred", move |g_, a_| Ok(windsinger_ok(g_, a_[0].item(), a_[1].obj(), hb)?.into()));
            q(g, o, "Windsinger", res, Map::new(), true, trig_target(opts, pred, "target", false, true), Value::None);
            Ok(Value::None)
        }),
        ..new_impl("Windsinger")
    }
}

// ====================================================================== Y / Z
// Yasuo, Windrider — "[Ganking] The third time I move in a turn, you score 1 point."
fn c_yasuo_windrider() -> Impl {
    Impl {
        kw: kw1("Ganking"),
        on_event: f("_yasuo", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "move" && is_obj(info, o) {
                let key = stat_key("yasuo_moves", g, o);
                g.stats.add(&key, 1);
                if g.stats.get(&key) == 3 {
                    let res = f("_yasuo_res", |g_, a_| {
                        let c = ctrl(g_, a_[0].item());
                        g_.gain_point(c, "Yasuo, Windrider")?;
                        Ok(Value::None)
                    });
                    q0(g, o, "Yasuo, Windrider", res);
                }
            }
            Ok(Value::None)
        }),
        ..new_impl("Yasuo, Windrider")
    }
}

// Zaunite Bouncer — "When you play me, return another unit at a battlefield to its owner's hand."
fn c_zaunite_bouncer() -> Impl {
    Impl {
        on_play: f("_bouncer", |g, a| {
            let res = f("_bouncer_res", |g_, a_| {
                let u = g_.legal(a_[0].item(), 0)?;
                to_hand(g_, u)?;
                Ok(Value::None)
            });
            let opts = f("_bouncer_opts", |g_, a_| {
                let it = a_[0].item();
                let (c, src) = (ctrl(g_, it), g_.it(it).src);
                let mut us = en_bf(g_, c)?;
                us.extend(fr_bf(g_, c)?);
                let us: Vec<O> = us.into_iter().filter(|&u| Some(g_.o(u).uid) != src).collect();
                Ok(us.into())
            });
            let pred = f("_bouncer_pred", |g_, a_| {
                let (it, u) = (a_[0].item(), a_[1].obj());
                Ok((Some(g_.o(u).uid) != g_.it(it).src && is_bf(g_.o(u).loc)).into())
            });
            let choose = trig_target(opts, pred, "target", false, true);
            q(g, a[0].obj(), "Zaunite Bouncer", res, Map::new(), false, choose, Value::None);
            Ok(Value::None)
        }),
        ..new_impl("Zaunite Bouncer")
    }
}

// Zed, Without a Sound — "When I conquer, play a 0 might Shadow Clone unit token to your base. [Action] [1][Chaos]:
// Move me and a Shadow Clone you control to each other's locations."  (Shadow Clone is registered by cardsets/fury)
fn c_zed_without_a_sound() -> Impl {
    let ab = ability(
        "Shadow Swap",
        AbilityArgs {
            cost: Cost::Text("1 energy and 1 chaos rune".into()),
            timing: "action",
            preds: Value::List(vec![f("_zed_pred", |g, a| {
                let (it, u) = (a[0].item(), a[1].obj());
                Ok((g.o(u).ctrl == g.it(it).ctrl && g.cname(u) == "Shadow Clone").into())
            })]),
            choices: f("_zed_choices", |g, a| {
                let (pid, o) = (pid_of(&a[0]), a[1].obj());
                let us: Vec<O> = g
                    .units(Some(pid), None)
                    .into_iter()
                    .filter(|&u| g.cname(u) == "Shadow Clone" && g.o(u).loc != g.o(o).loc)
                    .collect();
                Ok(tgv(g, &us))
            }),
            resolve: f("_zed_swap", |g, a| {
                let it = a[0].item();
                let m = g.it(it).src.and_then(|u| g.obj(u));
                let cl = g.legal(it, 0)?;
                let (Some(m), Some(cl)) = (m, cl) else { return Ok(Value::None) };
                if g.o(m).loc == g.o(cl).loc {
                    return Ok(Value::None);
                }
                let (x, y) = (g.o(m).loc.unwrap(), g.o(cl).loc.unwrap());
                let c = ctrl(g, it);
                mv(g, m, y, c)?;
                mv(g, cl, x, c)?;
                Ok(Value::None)
            }),
            ..Default::default()
        },
    );
    Impl {
        on_event: f("_zed_event", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "conquer" && pid_of(&info.g("pid")) == g.o(o).ctrl && in_units(info, o) {
                let res = f("_zed_clone", |g_, a_| {
                    let c = ctrl(g_, a_[0].item());
                    make_token(g_, "Shadow Clone", c, Loc::Base, false)?;
                    Ok(Value::None)
                });
                q0(g, o, "Zed, Without a Sound", res);
            }
            Ok(Value::None)
        }),
        abilities: vec![ab],
        ..new_impl("Zed, Without a Sound")
    }
}

// ====================================================================== cartes qui utilisent les crochets du moteur
/// `_open_bfs(g, closed)` : battlefields ouverts (`Value::Loc`)
fn open_bfs(g: &Game, closed: bool) -> Value {
    if closed {
        return Value::List(vec![]);
    }
    Value::List((0..g.bfs.len()).filter(|&i| open_bf(g, i)).map(|i| Value::Loc(Loc::Bf(i))).collect())
}

fn extra_locs_open() -> Value {
    f("_open_bfs", |g, a| Ok(open_bfs(g, a[2].truthy())))
}

fn with_hook(mut im: Impl, field: &str, v: Value) -> Impl {
    im.hooks.set(field, v);
    im
}

// Miss Fortune, Buccaneer — "You may play me to an open battlefield. Friendly units may be played to open
// battlefields."
fn c_miss_fortune_buccaneer() -> Impl {
    let im = with_hook(new_impl("Miss Fortune, Buccaneer"), "extra_locs", extra_locs_open());
    with_hook(
        im,
        "aura_locs",
        f("_mf_aura_locs", |g, a| {
            let (src, pid, c, closed) = (a[0].obj(), pid_of(&a[1]), a[2].obj(), a[3].truthy());
            if pid == g.o(src).ctrl && stype(g, c) == "Unit" {
                Ok(open_bfs(g, closed))
            } else {
                Ok(Value::List(vec![]))
            }
        }),
    )
}

// Ocean Drake — "You may play me to an open battlefield. When you play me, you may return a non-Dragon unit to its
// owner's hand."
fn drake_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c = ctrl(g, it);
    let mut us = vec![];
    for u in g.units(None, None) {
        if !g.tags(u)?.contains("Dragon") {
            us.push(u);
        }
    }
    let mut ks = vec![];
    for &u in &us {
        ks.push((g.o(u).ctrl == c, -value(g, u)?, g.o(u).uid));
    }
    let us = sort_keys(us, ks);
    if !us.is_empty() {
        let mut ov = objv(&us);
        ov.push(Value::None);
        let u = g.ask(c, "ocean_drake", ov, map! { item = it })?;
        if let Some(u) = u.opt_obj() {
            to_hand(g, Some(u))?;
        }
    }
    Ok(Value::None)
}

fn c_ocean_drake() -> Impl {
    let im = Impl {
        on_play: f("_drake_play", |g, a| {
            q(g, a[0].obj(), "Ocean Drake", f("_drake_res", drake_res), Map::new(), true, Value::None, Value::None);
            Ok(Value::None)
        }),
        ..new_impl("Ocean Drake")
    };
    with_hook(im, "extra_locs", extra_locs_open())
}

// Sai Scout — "[Vision] You may play me to an open battlefield."
fn c_sai_scout() -> Impl {
    with_hook(Impl { kw: kw1("Vision"), ..new_impl("Sai Scout") }, "extra_locs", extra_locs_open())
}

// Sneaky Deckhand — "You may play me to an open battlefield."
fn c_sneaky_deckhand() -> Impl {
    with_hook(new_impl("Sneaky Deckhand"), "extra_locs", extra_locs_open())
}

// Irelia, Graceful — "Your spells that choose me cost 1 energy or 1 rune of any type less."
fn c_irelia_graceful() -> Impl {
    with_hook(
        new_impl("Irelia, Graceful"),
        "cost_aura",
        f("_irelia_aura", |g, a| {
            let (src, pid, what) = (a[0].obj(), pid_of(&a[1]), a[2].map());
            let chv0 = what.g("choice");
            let ch = if chv0.truthy() { chv0.map().clone() } else { Map::new() };
            let mut chosen: Vec<Value> = ch.g("tg").list().to_vec();
            chosen.extend(ch.g("tg2").list().iter().cloned());
            for rc in ch.g("reps").list() {
                chosen.extend(rc.map().g("tg").list().iter().cloned());
            }
            if pid == g.o(src).ctrl
                && what.g("kind").opt_str() == Some("spell")
                && chosen.contains(&Value::from(g.o(src).uid))
            {
                return Ok(chv(vec![map! { flex = 1 }]));
            }
            Ok(Value::List(vec![]))
        }),
    )
}

// Stargazer — "Spells with [Flow] you play from your trash cost 2 energy less, to a minimum of 1 energy."
fn c_stargazer() -> Impl {
    with_hook(
        new_impl("Stargazer"),
        "cost_aura",
        f("_stargazer_aura", |g, a| {
            let (src, pid, what) = (a[0].obj(), pid_of(&a[1]), a[2].map());
            if pid == g.o(src).ctrl
                && what.g("kind").opt_str() == Some("spell")
                && what.g("src").opt_str() == Some("trash")
                && flow_of(g, pid, what.g("card").obj()).truthy()
            {
                return Ok(chv(vec![map! { e = -2, min_e = 1 }]));
            }
            Ok(Value::List(vec![]))
        }),
    )
}

// Vex, Cheerless — "While I'm in combat, friendly spells cost 1 energy and 1 rune of any type less to a minimum of 1
// energy, and enemy spells cost 1 energy and 1 rune of any type more."
fn c_vex_cheerless() -> Impl {
    with_hook(
        new_impl("Vex, Cheerless"),
        "cost_aura",
        f("_vex_aura", |g, a| {
            let (src, pid, what) = (a[0].obj(), pid_of(&a[1]), a[2].map());
            if what.g("kind").opt_str() != Some("spell") || !g.in_combat(src) {
                return Ok(Value::List(vec![]));
            }
            if pid == g.o(src).ctrl {
                return Ok(chv(vec![map! { e = -1, rm = 1, min_e = 1 }]));
            }
            Ok(chv(vec![map! { e = 1, p = reqs_to_value(&[any_req()]) }]))
        }),
    )
}

// Ezreal, Prodigy — "When you play me, discard 1, then draw 2. Optional additional costs you pay cost 1 energy or 1
// rune of any type less." (each optional additional cost: Accelerate, each Repeat cost, "you may pay ... as an
// additional cost")
fn c_ezreal_prodigy() -> Impl {
    let im = Impl {
        on_play: f("_ezreal_play", |g, a| {
            let res = f("_ezreal_res", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                discard_n(g_, c, 1)?;
                g_.draw(c, 2)?;
                Ok(Value::None)
            });
            q0(g, a[0].obj(), "Ezreal, Prodigy", res);
            Ok(Value::None)
        }),
        ..new_impl("Ezreal, Prodigy")
    };
    with_hook(
        im,
        "cost_aura",
        f("_ezreal_aura", |g, a| {
            if pid_of(&a[1]) == g.o(a[0].obj()).ctrl {
                Ok(chv(vec![map! { flex = 1, on = "opt" }]))
            } else {
                Ok(Value::List(vec![]))
            }
        }),
    )
}

// Syndra, Transcendent — "While I'm in a showdown, your spells have [Repeat] 2 energy and 1 chaos rune." (a granted
// Repeat instance, rule 820.1.c.2: actions.repeat_instances; its repetition makes its own choices)
fn c_syndra_transcendent() -> Impl {
    with_hook(
        new_impl("Syndra, Transcendent"),
        "card_kw",
        f("_syndra_kw", |g, a| {
            let (src, pid, c, kw) = (a[0].obj(), pid_of(&a[1]), a[2].obj(), a[3].str());
            if kw == "Repeat"
                && pid == g.o(src).ctrl
                && stype(g, c) == "Spell"
                && g.sd.as_ref().is_some_and(|sd| g.o(src).loc == Some(Loc::Bf(sd.bf)))
            {
                return Ok(Value::Tuple(vec![2.into(), reqs_to_value(&[req1("Chaos")])]));
            }
            Ok(false.into())
        }),
    )
}

// Kennen, Storm of Shuriken — "When you play me, [Burn 2]. When I conquer, give a spell in your trash [Flow] equal to
// its cost this turn." (a 'grant_flow' effect on that card: its Energy and Power costs, of its domains)
fn kennen_conquer(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c0 = ctrl(g, it);
    let sp: Vec<O> =
        g.p[c0].trash.iter().copied().filter(|&c| stype(g, c) == "Spell" && g.impl_o(c).is_some()).collect();
    let ks: Vec<(f64, u32)> = sp.iter().map(|&c| (-cval(g, c), g.o(c).uid)).collect();
    let sp = sort_keys(sp, ks);
    if sp.is_empty() {
        return Ok(Value::None);
    }
    let c = g.ask(c0, "kennen_flow", objv(&sp), map! { item = it })?.obj();
    let s = g.spec(c);
    let doms = Value::Tuple(s.domains.iter().map(|d| Value::from(d.as_str())).collect());
    let flow = Value::Tuple(vec![s.e.into(), s.p.into(), doms]);
    g.add_effect(map! { kind = "grant_flow", uid = g.o(c).uid, oid = g.o(c).oid, dur = "turn", flow = flow });
    let r = g.repr(c);
    g.log(|| format!("  {r} has [Flow] this turn"));
    Ok(Value::None)
}

fn c_kennen_storm_of_shuriken() -> Impl {
    Impl {
        on_event: f("_kennen_event", |g, a| {
            let (o, ev, info) = ev_args(a);
            if ev == "conquer" && in_units(info, o) {
                q0(g, o, "Kennen, Storm of Shuriken", f("_kennen_conquer", kennen_conquer));
            }
            Ok(Value::None)
        }),
        on_play: f("_kennen_play", |g, a| {
            let res = f("_kennen_burn", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                g_.burn(c, 2)?;
                Ok(Value::None)
            });
            q0(g, a[0].obj(), "Kennen, Storm of Shuriken", res);
            Ok(Value::None)
        }),
        ..new_impl("Kennen, Storm of Shuriken")
    }
}

// Mask Mother — "When you discard me, you may pay 1 energy to give a friendly unit +2 might this turn."
fn mask_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c = ctrl(g, it);
    let us = g.units(Some(c), None);
    let mut ks = vec![];
    for &u in &us {
        ks.push((-value(g, u)?, g.o(u).uid));
    }
    let us = sort_keys(us, ks);
    if !us.is_empty() {
        let u = if us.len() > 1 { g.ask(c, "mask_mother", objv(&us), map! { item = it })?.obj() } else { us[0] };
        g.mod_(u, 2, "turn".into(), None)?;
    }
    Ok(Value::None)
}

fn c_mask_mother() -> Impl {
    Impl {
        on_discard: f("_mask_discard", |g, a| {
            let pid = pid_of(&a[1]);
            g.queue_trigger(pid, "Mask Mother", f("_mask_res", mask_res), Map::new(), None, true, Value::None, may_pay(1, Value::None));
            Ok(Value::None)
        }),
        ..new_impl("Mask Mother")
    }
}

// Scrapheap — "When this is played, discarded, or killed, draw 1." (killed: its own 'leave' event from a kill; this is
// not a Deathknell)
fn c_scrapheap() -> Impl {
    Impl {
        on_play: f("_scrapheap_play", |g, a| {
            q0(g, a[0].obj(), "Scrapheap", draw1_res());
            Ok(Value::None)
        }),
        on_discard: f("_scrapheap_discard", |g, a| {
            let pid = pid_of(&a[1]);
            g.queue_trigger(pid, "Scrapheap", draw1_res(), Map::new(), None, false, Value::None, Value::None);
            Ok(Value::None)
        }),
        on_leave: f("_scrapheap_leave", |g, a| {
            let info = a[1].map();
            if info.g("killed").truthy() {
                let p = pid_of(&info.g("info").map().g("ctrl"));
                g.queue_trigger(p, "Scrapheap", draw1_res(), Map::new(), None, false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..new_impl("Scrapheap")
    }
}

// Treasure Trove — "When this leaves the board, draw 1 and channel 1 rune exhausted. [Chaos], [E]: Kill this."
fn c_treasure_trove() -> Impl {
    let ab = ability(
        "Kill this",
        AbilityArgs {
            cost: Cost::Text("1 chaos rune".into()),
            exhaust: true,
            resolve: f("_trove_kill", |g, a| {
                let it = a[0].item();
                if let Some(m) = me(g, it) {
                    let c = ctrl(g, it);
                    g.kill(&[m], Some(c), false)?;
                }
                Ok(Value::None)
            }),
            ..Default::default()
        },
    );
    Impl {
        on_leave: f("_trove_leave", |g, a| {
            let p = pid_of(&a[1].map().g("info").map().g("ctrl"));
            let res = f("_trove_res", |g_, a_| {
                let c = ctrl(g_, a_[0].item());
                g_.draw(c, 1)?;
                g_.channel(c, 1, true)?;
                Ok(Value::None)
            });
            g.queue_trigger(p, "Treasure Trove", res, Map::new(), None, false, Value::None, Value::None);
            Ok(Value::None)
        }),
        abilities: vec![ab],
        ..new_impl("Treasure Trove")
    }
}

// Maduli the Gatekeeper — "I can't be readied. [Chaos]: Move me to an occupied enemy battlefield if my Might is greater
// than the total Might of enemy units there."
fn maduli_ok(g: &mut Game, o: O, b: usize) -> Res<bool> {
    let es = g.units(Some(1 - g.o(o).ctrl), Some(Loc::Bf(b)));
    if es.is_empty() || g.bfs[b].ctrl != Some(1 - g.o(o).ctrl) {
        return Ok(false);
    }
    let m = g.might(o)?;
    Ok(m > sum_might(g, &es)?)
}

fn c_maduli_the_gatekeeper() -> Impl {
    let ab = ability(
        "Move",
        AbilityArgs {
            cost: Cost::Text("1 chaos rune".into()),
            resolve: f("_maduli_res", |g, a| {
                let it = a[0].item();
                let o = g.it(it).src.and_then(|u| g.obj(u));
                let b = dget(g, it, "bf");
                if let (Some(o), Some(b)) = (o, b.opt_int()) {
                    let b = b as usize;
                    if g.o(o).loc != Some(Loc::Bf(b)) && maduli_ok(g, o, b)? {
                        let c = ctrl(g, it);
                        mv(g, o, Loc::Bf(b), c)?;
                    }
                }
                Ok(Value::None)
            }),
            choices: f("_maduli_choices", |g, a| {
                let o = a[1].obj();
                let mut out = vec![];
                for i in 0..g.bfs.len() {
                    if Some(Loc::Bf(i)) != g.o(o).loc && maduli_ok(g, o, i)? {
                        out.push(map! { bf = i });
                    }
                }
                Ok(chv(out))
            }),
            ..Default::default()
        },
    );
    let im = Impl { abilities: vec![ab], ..new_impl("Maduli the Gatekeeper") };
    with_hook(im, "no_ready", f("_maduli_no_ready", |_g, _a| Ok(true.into())))
}

// Vex, Apathetic — "[Deflect] When an opponent plays a unit while I'm at a battlefield, [Stun] it. They can't move it
// this turn."
fn vex_ap_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let Some(u) = same(g, &dget(g, it, "u"), &dget(g, it, "uoid")) else { return Ok(Value::None) };
    let c = ctrl(g, it);
    g.stun(Some(u), Some(c))?;
    let no_move = f("_vex_no_move", |g_, a_| {
        let (eid, o, by) = (a_[0].int() as u64, a_[1].obj(), &a_[3]);
        let Some(e) = g_.eff(eid) else { return Ok(false.into()) };
        Ok((Value::from(g_.o(o).uid) == e.g("uid") && Value::from(g_.o(o).oid) == e.g("oid") && *by == e.g("pid")).into())
    });
    let mut d = Map::new();
    d.set("kind", "cant_move");
    d.set("fn", no_move);
    d.set("uid", g.o(u).uid);
    d.set("oid", g.o(u).oid);
    d.set("pid", g.o(u).ctrl);
    d.set("dur", "turn");
    g.add_effect(d);
    Ok(Value::None)
}

fn c_vex_apathetic() -> Impl {
    Impl {
        kw: kw1("Deflect"),
        on_event: f("_vex_ap_event", |g, a| {
            let (o, ev, info) = ev_args(a);
            let c = if ev == "played" { info.g("card").opt_obj() } else { None };
            if let Some(c) = c {
                if pid_of(&info.g("pid")) != g.o(o).ctrl
                    && stype(g, c) == "Unit"
                    && is_bf(g.o(o).loc)
                    && g.board.contains(&c)
                {
                    let data = map! { u = g.o(c).uid, uoid = g.o(c).oid };
                    q(g, o, "Vex, Apathetic", f("_vex_ap_res", vex_ap_res), data, false, Value::None, Value::None);
                }
            }
            Ok(Value::None)
        }),
        ..new_impl("Vex, Apathetic")
    }
}

// Sivir, Mercenary — "[Accelerate] If you've spent at least 2 runes of any type this turn, I have +2 might and
// [Ganking]." (Game.hist['power_spent'])
fn sivir_merc() -> Value {
    f("_sivir_merc", |g, a| Ok((g.hist.power_spent[g.o(a[0].obj()).ctrl] >= 2).into()))
}

fn c_sivir_mercenary() -> Impl {
    Impl {
        accelerate: true,
        might_if: vec![(sivir_merc(), 2)],
        kw_if: vec![(sivir_merc(), kw1("Ganking"))],
        ..new_impl("Sivir, Mercenary")
    }
}

// Nocturne, Horrifying — "[Ganking] When you look at cards from the top of your deck (and don't draw them) and see me,
// you may play me for 1 rune of any type." (Impl.on_seen, called by Game.look; when the trigger resolves he must
// still be in the Main Deck, not drawn since)
fn nocturne_alt(ch: &Map) -> Map {
    let mut m = ch.clone();
    m.set("alt", "nocturne");
    m.set("alt_e", 0);
    m.set("alt_reqs", Value::Tuple(reqs_to_value(&[any_req()]).list().to_vec()));
    m
}

fn nocturne_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c0 = ctrl(g, it);
    let want = dget(g, it, "c");
    let mut c = None;
    for &x in &g.p[c0].deck {
        if Value::from(g.o(x).uid) == want {
            c = Some(x);
        }
    }
    let Some(c) = c else { return Ok(Value::None) };
    if Value::from(g.o(c).oid) != dget(g, it, "oid") || Value::from(g.o(c).drawn_at) != dget(g, it, "drawn") {
        return Ok(Value::None);
    }
    let ctx: Value = map! { kind = "unit", card = c }.into();
    let mut chs = vec![];
    for ch in unit_play_choices(g, c0, c, "deck", None)? {
        let (e, reqs) = total_cost(g, c0, c, &nocturne_alt(&ch), "deck")?;
        if g.can_pay(c0, e, &reqs, &ctx)? {
            chs.push(ch);
        }
    }
    if chs.is_empty() {
        return Ok(Value::None);
    }
    let ch = if chs.len() > 1 {
        g.ask(c0, "play_choice", chs.into_iter().map(Value::Map).collect(), map! { card = c })?.map().clone()
    } else {
        chs.remove(0)
    };
    let ch = nocturne_alt(&ch);
    let (e, reqs) = total_cost(g, c0, c, &ch, "deck")?;
    if g.pay(c0, e, &reqs, &ctx)? {
        play_card(g, c0, c, "deck", ch, true)?;
    }
    Ok(Value::None)
}

fn c_nocturne_horrifying() -> Impl {
    with_hook(
        Impl { kw: kw1("Ganking"), ..new_impl("Nocturne, Horrifying") },
        "on_seen",
        f("_nocturne_seen", |g, a| {
            let (c, pid) = (a[0].obj(), pid_of(&a[1]));
            if pid == g.o(c).owner {
                let data = map! { c = g.o(c).uid, oid = g.o(c).oid, drawn = g.o(c).drawn_at };
                g.queue_trigger(pid, "Nocturne, Horrifying", f("_nocturne_res", nocturne_res), data, None, true, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
    )
}

// Baron Nashor — "As you play me, add the Baron Pit battlefield token to the board if it's not there already. If you
// do, I enter there. (It has "Units can move here from anywhere.") I can't be chosen by enemy spells and abilities.
// Other friendly units have +2 might." The last sentence of the first ability replaces where he enters (rule 369.3,
// whose example is this card): only when the token was added by this play. Entering there applies Contested (rule
// 190.3.a.1), so he usually conquers the Baron Pit. With three battlefields on the board, the Final Point by conquer
// needs all three scored that turn (rule 471.1.b.1).
fn c_baron_nashor() -> Impl {
    Impl {
        enter_loc: f("_nashor_enter", |g, a| {
            if g.bfs.iter().any(|b| b.name == BARON_PIT) {
                return Ok(a[2].clone());
            }
            Ok(Value::Loc(Loc::Bf(g.add_battlefield(BARON_PIT)?)))
        }),
        untargetable: f("_nashor_untargetable", |g, a| {
            Ok((a[1].opt_int() != Some(g.o(a[0].obj()).ctrl as i64)).into())
        }),
        aura_might: f("_nashor_aura", |g, a| {
            let (src, o) = (a[0].obj(), a[1].obj());
            let ok = stype(g, src) == "Unit" && o != src && g.o(o).ctrl == g.o(src).ctrl && stype(g, o) == "Unit";
            Ok(if ok { 2 } else { 0 }.into())
        }),
        ..new_impl("Baron Nashor")
    }
}



// ====================================================================== enregistrement (ordre du Python)
pub fn register(r: &mut Registry) {
    r.card(c_abandon());
    r.card(c_acceptable_losses());
    r.card(c_ancient_warmonger());
    r.card(c_angler_beast());
    r.card(c_annie_stubborn());
    r.card(c_beast_below());
    r.card(c_bewitching_spirit());
    r.card(c_black_market_broker());
    r.card(c_blast_cone());
    r.card(c_bone_skewer());
    r.card(c_boots_of_swiftness());
    r.card(c_called_shot());
    r.card(c_cemetery_attendant());
    r.card(c_conscription());
    r.card(c_corrupt_enforcer());
    r.card(c_crescent_guardian());
    r.card(c_cull());
    r.card(c_cursed_sarcophagus());
    r.card(c_decree_of_discord());
    r.card(c_diana_no_longer_human());
    r.card(c_dorans_ring());
    r.card(c_downwell());
    r.card(c_draven_audacious());
    r.card(c_edge_of_night());
    r.card(c_ember_monk());
    r.card(c_evelynn_entrancing());
    r.card(c_evershade_stalker());
    r.card(c_existential_dread());
    r.card(c_factory_recall());
    r.card(c_fading_memories());
    r.card(c_fae_porter());
    r.card(c_fight_or_flight());
    r.card(c_fizz_trickster());
    r.card(c_flash());
    r.card(c_forgotten_relic());
    r.card(c_gust());
    r.card(c_gust_monk());
    r.card(c_hard_bargain());
    r.card(c_harpoon_squad());
    r.card(c_heedless_resurrection());
    r.card(c_illaoi());
    r.card(c_insightful_investigator());
    r.card(c_invert_timelines());
    r.card(c_isolate());
    r.card(c_jae_medarda());
    r.card(c_jinx_rebel());
    r.card(c_kayn_unleashed());
    r.card(c_khazix());
    r.card(c_kharox());
    r.card(c_kinkou_lifeblade());
    r.card(c_kogmaw_caustic());
    r.card(c_last_rites());
    r.card(c_loyal_pup());
    r.card(c_lunar_boon());
    r.card(c_maddened_marauder());
    r.card(c_megatusk());
    r.card(c_mel_defiant_soul());
    r.card(c_minah_swiftfoot());
    r.card(c_mindsplitter());
    r.card(c_mister_root());
    r.card(c_morbid_return());
    r.card(c_overzealous_fan());
    r.card(c_pack_of_wonders());
    r.card(c_possession());
    r.card(c_pyke_returned());
    r.card(c_ravenbloom_prefect());
    r.card(c_rebuke());
    r.card(c_rhasa_the_sunderer());
    r.card(c_ride_the_wind());
    r.card(c_scryers_bloom());
    r.card(c_seal_of_discord());
    r.card(c_shadow_order_disciple());
    r.card(c_shadowblade_lurker());
    r.card(c_shadows_of_the_past());
    r.card(c_sinister_poro());
    r.card(c_soulgorger());
    r.card(c_spiderling());
    r.card(c_spirit_wheel());
    r.card(c_stacked_deck());
    r.card(c_star_crossed());
    r.card(c_stealthy_pursuer());
    r.card(c_switcheroo());
    r.card(c_tail_cloaked_matriarch());
    r.card(c_teemo_scout());
    r.card(c_temptation());
    r.card(c_the_harrowing());
    r.card(c_the_list());
    r.card(c_the_syren());
    r.card(c_tideturner());
    r.card(c_tornado_warrior());
    r.card(c_traveling_merchant());
    r.card(c_treasure_hunter());
    r.card(c_twilight_step());
    r.card(c_twisted_fate_gambler());
    r.card(c_undercover_agent());
    r.card(c_up_from_the_deep());
    r.card(c_vicious_snapjaws());
    r.card(c_walking_roost());
    r.card(c_whirlwind());
    r.card(c_wind_and_ghosts());
    r.card(c_windsinger());
    r.card(c_yasuo_windrider());
    r.card(c_zaunite_bouncer());
    r.card(c_zed_without_a_sound());
    r.ask_text(&[
        ("ocean_drake", "Ocean Drake : quelle unité non-Dragon renvoyer dans la main de son propriétaire ?"),
        ("kennen_flow", "Kennen, Storm of Shuriken : à quel sort de ta défausse donner [Flow] ?"),
        ("mask_mother", "Mask Mother : quelle unité alliée gagne +2 ?"),
        ("maduli_dest", "Maduli the Gatekeeper : vers quel champ de bataille ?"),
        ("nocturne_play", "Nocturne, Horrifying : le jouer pour 1 rune ?"),
    ]);
    r.card(c_miss_fortune_buccaneer());
    r.card(c_ocean_drake());
    r.card(c_sai_scout());
    r.card(c_sneaky_deckhand());
    r.card(c_irelia_graceful());
    r.card(c_stargazer());
    r.card(c_vex_cheerless());
    r.card(c_ezreal_prodigy());
    r.card(c_syndra_transcendent());
    r.card(c_kennen_storm_of_shuriken());
    r.card(c_mask_mother());
    r.card(c_scrapheap());
    r.card(c_treasure_trove());
    r.card(c_maduli_the_gatekeeper());
    r.card(c_vex_apathetic());
    r.card(c_sivir_mercenary());
    r.card(c_nocturne_horrifying());
    r.card(c_baron_nashor());
}
