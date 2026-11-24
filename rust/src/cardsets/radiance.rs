//! Paquet radiance : les cartes de l'extension Radiance (RAD, sortie le 2026-10-23). Traduction ligne à ligne de
//! `riftbound/engine/cardsets/radiance.py` (voir `cardsets/GUIDE.md` et `docs/memoire/portage-rust.md`).
//!
//! Nouveaux mots-clés des Core Rules du 2026-10-09 (rules/source/core_rules_2026-10-09.txt), gérés par le moteur :
//! [Deploy] (830 : actions.gear_locations / has_deploy, Game.hold, étape 5 du nettoyage), [Disarm] (831 :
//! Game.combat_trigger_check -> cards.disarm_trigger), [Show Off] (832 : Impl.show_off, actions.show_options /
//! shown_info) et le jeton d'équipement Bomb (187.12). « When I become exhausted » est l'évènement 'exhausted'
//! (Obj.exhausted, Game._advance).
//! Les cartes qui ne peuvent pas être modélisées fidèlement restent hors de IMPL : voir NEEDS_radiance.md.
//!
use std::cell::RefCell;
use std::collections::BTreeSet;

use crate::actions::{
    attach, deflect_reqs, full_choices, gear_locations, has_deploy, open_bf, pay_ctx, play_card, play_forbidden,
    show_options, spell_variants, total_cost,
};
use crate::cards::{Impl, Registry};
use crate::cards_helpers::{
    ability, all_units, cap, empower_ability, enemies, friends, impl_equip, item_by_id, make_token, may_pay,
    p_enemy, p_enemy_v, p_friend_v, p_gear_v, p_unit_bf, p_unit_bf_v, p_unit_v, pay_deflect, play_unit_free,
    tg_choices, token_locations, trig_target, unit_play_choices, value, AbilityArgs, Cost,
};
use crate::game::{Desig, Game, ItemId, Res, Zone, O};
use crate::game_costs::{any_req, reqs_to_value, Req};
use crate::lang::tr;
use crate::spec::spec;
use crate::value::{is_bf, Loc, Map, OptV, Value};

// ====================================================================== petites aides privées (valeurs)
/// Un dict Python construit clé par clé (les clés comme `fn` ne passent pas par `map!`).
fn mk(pairs: &[(&str, Value)]) -> Map {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.set(k, v.clone());
    }
    m
}

fn req(ds: &[&str]) -> Req {
    ds.iter().map(|d| d.to_string()).collect()
}


fn objs_v(xs: &[O]) -> Value {
    Value::List(xs.iter().map(|&o| Value::Obj(o)).collect())
}

fn maps_v(xs: Vec<Map>) -> Value {
    Value::List(xs.into_iter().map(Value::Map).collect())
}

fn maps_of(v: &Value) -> Vec<Map> {
    v.list().iter().map(|x| x.map().clone()).collect()
}

fn objs_of(v: &Value) -> Vec<O> {
    v.list().iter().map(|x| x.obj()).collect()
}

/// `(uid,)` d'une cible
fn tg1(g: &Game, o: O) -> Value {
    Value::Tuple(vec![g.o(o).uid.into()])
}

fn uids_t(g: &Game, xs: &[O]) -> Value {
    Value::Tuple(xs.iter().map(|&o| Value::from(g.o(o).uid)).collect())
}

/// `Opt(label, value)`
fn opt(label: String, value: Value) -> Value {
    OptV { label, value, obj: Value::None }.into()
}

/// Clé de `g.stats` : le `repr` du tuple Python.
fn skey(parts: &[Value]) -> String {
    let s: Vec<String> = parts
        .iter()
        .map(|p| match p {
            Value::Str(s) => format!("'{s}'"),
            Value::Int(i) => i.to_string(),
            Value::Bool(b) => (if *b { "True" } else { "False" }).to_string(),
            Value::None => "None".to_string(),
            v => format!("{v:?}"),
        })
        .collect();
    format!("({})", s.join(", "))
}

/// `g.stats[k] = v`
fn stat_set(g: &mut Game, k: &str, v: i64) {
    if let Some(e) = g.stats.0.iter_mut().find(|(x, _)| x == k) {
        e.1 = v;
    } else {
        g.stats.0.push((k.to_string(), v));
    }
}

fn ty(g: &Game, o: O) -> &'static str {
    g.spec(o).type_.as_str()
}

fn e_of(g: &Game, o: O) -> i64 {
    g.spec(o).e as i64
}

fn p_of(g: &Game, o: O) -> i64 {
    g.spec(o).p as i64
}

fn might_of(g: &Game, o: O) -> i64 {
    g.spec(o).might.unwrap_or(0) as i64
}

/// `o.zone == "board"`
fn zone_is(g: &Game, o: O, z: Zone) -> bool {
    g.o(o).zone == Some(z)
}

/// `info["spec"]["type"]` d'une information rétrospective (`spec` y est le nom de la carte)
fn inf_type(inf: &Map) -> &'static str {
    spec(inf.g("spec").str()).type_.as_str()
}

/// `sorted(xs, key=...)` : tri stable sur une clé calculée une fois par élément.
fn sort_by<K: PartialOrd>(g: &mut Game, xs: Vec<O>, mut key: impl FnMut(&mut Game, O) -> Res<K>) -> Res<Vec<O>> {
    let mut v = Vec::with_capacity(xs.len());
    for x in xs {
        v.push((key(g, x)?, x));
    }
    v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Ok(v.into_iter().map(|p| p.1).collect())
}

/// `itertools.combinations(xs, k)` (même ordre)
fn combinations<T: Clone>(xs: &[T], k: usize) -> Vec<Vec<T>> {
    let n = xs.len();
    let mut out = vec![];
    if k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.iter().map(|&i| xs[i].clone()).collect());
        let mut i = k;
        loop {
            if i == 0 {
                return out;
            }
            i -= 1;
            if idx[i] != i + n - k {
                break;
            }
            if i == 0 {
                return out;
            }
        }
        idx[i] += 1;
        for j in i + 1..k {
            idx[j] = idx[j - 1] + 1;
        }
    }
}

/// `pl.deck.remove(c)`
fn deck_remove(g: &mut Game, pid: usize, c: O) {
    if let Some(i) = g.p[pid].deck.iter().position(|&x| x == c) {
        g.p[pid].deck.remove(i);
    }
}

fn turn() -> Value {
    Value::from("turn")
}

// ====================================================================== fonctions rangées dans des valeurs
fn res_fn(name: &'static str, f: impl Fn(&mut Game, ItemId) -> Res<()> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| {
        f(g, a[0].item())?;
        Ok(Value::None)
    })
}

fn bool_fn(name: &'static str, f: impl Fn(&mut Game, ItemId) -> Res<bool> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| Ok(Value::Bool(f(g, a[0].item())?)))
}

/// options_fn(g, it) -> [obj] de `trig_target`
fn opts_fn(name: &'static str, f: impl Fn(&mut Game, ItemId) -> Res<Vec<O>> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| Ok(objs_v(&f(g, a[0].item())?)))
}

/// prédicat de cible pred(g, it, o)
fn pred_fn(name: &'static str, f: impl Fn(&mut Game, ItemId, O) -> Res<bool> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| Ok(Value::Bool(f(g, a[0].item(), a[1].obj())?)))
}

fn ctx_of(v: &Value) -> Map {
    match v {
        Value::Map(m) => m.clone(),
        _ => Map::new(),
    }
}

/// on_play(g, obj, ctx)
fn on_play_fn(name: &'static str, f: impl Fn(&mut Game, O, &Map) -> Res<()> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| {
        let ctx = ctx_of(&a[1]);
        f(g, a[0].obj(), &ctx)?;
        Ok(Value::None)
    })
}

/// on_event(g, o, ev, info) ; zone_event(g, c, ev, info)
fn on_event_fn(name: &'static str, f: impl Fn(&mut Game, O, &str, &Map) -> Res<()> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| {
        let info = ctx_of(&a[2]);
        f(g, a[0].obj(), a[1].str(), &info)?;
        Ok(Value::None)
    })
}

/// bf_event(g, b, ev, info) et legend_event(g, pid, ev, info) : le premier argument est un entier
fn idx_event_fn(name: &'static str, f: impl Fn(&mut Game, usize, &str, &Map) -> Res<()> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| {
        let info = ctx_of(&a[2]);
        f(g, a[0].int() as usize, a[1].str(), &info)?;
        Ok(Value::None)
    })
}

/// choices(g, pid, ctx) d'un sort
fn choices_fn(name: &'static str, f: impl Fn(&mut Game, usize, &Map) -> Res<Vec<Map>> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| {
        let ctx = ctx_of(&a[1]);
        Ok(maps_v(f(g, a[0].int() as usize, &ctx)?))
    })
}

/// choices(g, pid, obj) d'une capacité
fn ab_choices_fn(name: &'static str, f: impl Fn(&mut Game, usize, O) -> Res<Vec<Map>> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| Ok(maps_v(f(g, a[0].int() as usize, a[1].obj())?)))
}

/// can(g, pid, obj) d'une capacité
fn can_fn(name: &'static str, f: impl Fn(&mut Game, usize, O) -> Res<bool> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| Ok(Value::Bool(f(g, a[0].int() as usize, a[1].obj())?)))
}

/// extra_cost(g, pid, obj, choice) d'une capacité (ne modifie pas le choix)
fn extra_fn(name: &'static str, f: impl Fn(&mut Game, usize, O, &Map) -> Res<()> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| {
        let ch = ctx_of(&a[2]);
        f(g, a[0].int() as usize, a[1].obj(), &ch)?;
        Ok(Value::None)
    })
}

/// fonction (g, pid, card, choice) -> bool (enter_ready)
fn card_ch_bool(name: &'static str, f: impl Fn(&mut Game, usize, O, &Map) -> Res<bool> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| {
        let ch = ctx_of(&a[2]);
        Ok(Value::Bool(f(g, a[0].int() as usize, a[1].obj(), &ch)?))
    })
}

/// cost_mod(g, pid, card, choice) -> (de, dp)
fn cost_mod_fn(name: &'static str, f: impl Fn(&mut Game, usize, O, &Map) -> Res<(i64, i64)> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| {
        let ch = ctx_of(&a[2]);
        let (de, dp) = f(g, a[0].int() as usize, a[1].obj(), &ch)?;
        Ok(Value::Tuple(vec![de.into(), dp.into()]))
    })
}

/// as_played(g, pid, card) -> [dict]
fn as_played_fn(name: &'static str, f: impl Fn(&mut Game, usize, O) -> Res<Vec<Map>> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| Ok(maps_v(f(g, a[0].int() as usize, a[1].obj())?)))
}

/// show_off(g, pid, card, x) -> bool
fn show_off_fn(name: &'static str, f: impl Fn(&mut Game, usize, O, O) -> Res<bool> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| Ok(Value::Bool(f(g, a[0].int() as usize, a[1].obj(), a[2].obj())?)))
}

/// show_key(g, x) -> nombre
fn show_key_fn(name: &'static str, f: impl Fn(&mut Game, O) -> Res<i64> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| Ok(Value::Int(f(g, a[0].obj())?)))
}

/// might_mod(g, o) -> int
fn might_mod_fn(name: &'static str, f: impl Fn(&mut Game, O) -> Res<i64> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| Ok(Value::Int(f(g, a[0].obj())?)))
}

/// aura_kw(g, src, o) -> dict ou None
fn aura_kw_fn(name: &'static str, f: impl Fn(&mut Game, &Value, O) -> Res<Option<Map>> + Send + Sync + 'static) -> Value {
    Value::func(name, move |g, a| Ok(f(g, &a[0], a[1].obj())?.map(Value::Map).unwrap_or(Value::None)))
}

fn imp(name: &str) -> Impl {
    Impl { module: Some("radiance"), ..Impl::new(name) }
}

fn ab(name: &str, cost: Cost, a: AbilityArgs) -> Map {
    ability(name, AbilityArgs { cost, ..a })
}

// ====================================================================== aides privées (radiance.py)
/// `_q(g, o, name, fn, data, **kw)` : met en attente une capacité déclenchée du permanent o (son incarnation est
/// retenue, voir `me`).
#[allow(clippy::too_many_arguments)]
fn q(g: &mut Game, o: O, name: &str, f: Value, data: Map, may: bool, choose: Value, cost: Value) {
    let mut d = data;
    d.set("_oid", g.o(o).oid);
    let (ctrl, uid) = (g.o(o).ctrl, g.o(o).uid);
    g.queue_trigger(ctrl, name, f, d, Some(uid), may, choose, cost);
}

fn q0(g: &mut Game, o: O, name: &str, f: Value) {
    q(g, o, name, f, Map::new(), false, Value::None, Value::None);
}

/// `_me(g, it)` : le permanent source d'un déclenchement s'il est encore le même objet sur le plateau (359.3.e.4).
fn me(g: &Game, it: ItemId) -> Option<O> {
    let o = g.it(it).src.and_then(|u| g.obj(u))?;
    let oid = g.it(it).data.get("_oid").map(|v| v.int()).unwrap_or(g.o(o).oid);
    if g.o(o).oid != oid {
        return None;
    }
    Some(o)
}

fn ctrl(g: &Game, it: ItemId) -> usize {
    g.it(it).ctrl
}

fn dg(g: &Game, it: ItemId, k: &str) -> Value {
    g.it(it).data.g(k)
}

fn src_obj(g: &Game, it: ItemId) -> Option<O> {
    g.it(it).src.and_then(|u| g.obj(u))
}

fn cval(g: &Game, c: O) -> f64 {
    e_of(g, c) as f64 + 2.0 * p_of(g, c) as f64 + might_of(g, c) as f64 / 2.0
}

/// `_discard(g, pid, n)` : défausser n (422) : le joueur choisit ; autant que possible (359.3.e.11).
fn discard_n(g: &mut Game, pid: usize, n: i64) -> Res<()> {
    for _ in 0..n {
        let h = g.p[pid].hand.clone();
        if h.is_empty() {
            return Ok(());
        }
        let opts = sort_by(g, h, |g, c| Ok((cval(g, c), g.o(c).uid)))?;
        let c = g.ask(pid, "discard", objs_v(&opts).list().to_vec(), Map::new())?.obj();
        g.discard(pid, c)?;
    }
    Ok(())
}

fn bf_ask(g: &mut Game, pid: usize, kind: &str, locs: Option<Vec<Loc>>) -> Res<Option<Loc>> {
    let locs = locs.unwrap_or_else(|| g.bf_locs());
    if locs.is_empty() {
        return Ok(None);
    }
    if locs.len() > 1 {
        Ok(g.ask(pid, kind, locs.into_iter().map(Value::Loc).collect(), Map::new())?.opt_loc())
    } else {
        Ok(Some(locs[0]))
    }
}

/// `_bomb(g, pid, loc)` : « Play a Bomb gear token to a battlefield » ([Deploy] : n'importe quel battlefield, 830.1.b).
fn bomb(g: &mut Game, pid: usize, loc: Option<Loc>) -> Res<Option<O>> {
    let loc = match loc {
        Some(l) => l,
        None => match bf_ask(g, pid, "bomb_location", None)? {
            Some(l) => l,
            None => return Ok(None),
        },
    };
    make_token(g, "Bomb", pid, loc, true)
}

/// `_gold(g, pid, ready)` : « play a Gold gear token exhausted »
fn gold(g: &mut Game, pid: usize, ready: bool) -> Res<Option<O>> {
    make_token(g, "Gold", pid, Loc::Base, ready)
}

/// `_unit_token(g, pid, name, locs, ready)` : « Play a ... unit token » : la base ou un battlefield contrôlé (184.2,
/// 355.2), sauf emplacement donné.
fn unit_token(g: &mut Game, pid: usize, name: &str) -> Res<Option<O>> {
    let ls = token_locations(g, pid, name, None)?;
    if ls.is_empty() {
        return Ok(None);
    }
    let loc = if ls.len() > 1 {
        g.ask(pid, "token_location", ls.into_iter().map(Value::Loc).collect(), crate::map! { token = name })?.loc()
    } else {
        ls[0]
    };
    make_token(g, name, pid, loc, false)
}

/// `_from_facedown(info)` : évènement 'played' : la carte a-t-elle été jouée face cachée (Hidden) ?
fn from_facedown(g: &Game, info: &Map) -> bool {
    if let Value::Item(it) = info.g("item") {
        let fr = g.it(it).data.g("from");
        if !fr.is_none() {
            return fr.opt_str() == Some("facedown");
        }
    }
    g.o(info.g("card").obj()).played_from.as_deref() == Some("facedown")
}

fn is_deploy_gear(g: &mut Game, o: O) -> Res<bool> {
    Ok(g.is_gear(o) && has_deploy(g, o)?)
}

fn deflect_ok(g: &mut Game, pid: usize, u: O) -> Res<bool> {
    let ch = crate::map! { tg = tg1(g, u) };
    let reqs = deflect_reqs(g, pid, &ch)?;
    g.can_pay(pid, 0, &reqs, &Value::None)
}

/// `_buff_targets(g, pid, hb)` : « Give a unit +N » : unités alliées d'abord (ordre de l'IA), puis les autres.
fn buff_targets(g: &mut Game, pid: usize, hb: Option<Loc>) -> Res<Vec<O>> {
    let mut fr = friends(g, pid, false, hb)?;
    for u in all_units(g, pid, false, hb)? {
        if g.o(u).ctrl != pid {
            fr.push(u);
        }
    }
    Ok(fr)
}

fn minus_targets(g: &mut Game, pid: usize, hb: Option<Loc>, at_bf: bool) -> Res<Vec<O>> {
    let mut es = enemies(g, pid, at_bf, hb, true)?;
    es.extend(friends(g, pid, at_bf, hb)?);
    Ok(es)
}

/// `_banish_playable(g, pid)` : « Banish the top card of your Main Deck. Until your next Ending Phase, you may play
/// that card. (You still pay its costs.) » (Runeblade Anvil, Akali, Brash) : permission de jouer depuis le
/// bannissement (actions.playable_cards).
fn banish_playable(g: &mut Game, pid: usize) -> Res<Option<O>> {
    if g.p[pid].deck.is_empty() {
        return Ok(None); // pas une pioche : pas de burn out (431)
    }
    let c = g.p[pid].deck[0];
    g.to_zone(c, "banish", false, Some(pid))?;
    let (uid, oid) = (g.o(c).uid, g.o(c).oid);
    let key = Value::Tuple(vec!["banished".into(), uid.into(), oid.into()]);
    g.add_effect(mk(&[
        ("kind", "play_banished".into()),
        ("pid", pid.into()),
        ("uid", uid.into()),
        ("oid", oid.into()),
        ("key", key.clone()),
    ]));
    let end = Value::func("_banish_playable.end", |g, a| {
        let eid = a[0].int() as u64;
        let info = ctx_of(&a[1]);
        let eff = g.eff(eid).cloned().unwrap_or_default();
        if info.g("pid") == eff.g("pid") {
            // « until your next Ending Phase »
            let k = eff.g("key");
            g.effects.retain(|e| e.d.g("key") != k && e.eid != eid);
        }
        Ok(Value::None)
    });
    g.add_effect(mk(&[("on", "end_turn".into()), ("fn", end), ("pid", pid.into()), ("key", key)]));
    let cn = g.cname(c).to_string();
    g.log(|| format!("  P{pid} banishes {cn} (may play it until their next Ending Phase)"));
    Ok(Some(c))
}

type Cand = (Map, i64, Vec<Req>);

/// `_free_choices(g, pid, card, src, ignore_energy, extra_reqs)` : (choix, énergie, reqs) légaux pour jouer card
/// comme une pose limitée (419.3) en ignorant son coût (ou seulement son coût en énergie), plus extra_reqs ; les
/// coûts supplémentaires obligatoires (Deflect...) restent dus. Les équipements [Deploy] vont sur un battlefield ;
/// les choix [Show Off] sont proposés. (locs=None et free=True : seuls usages du module.)
fn free_choices(g: &mut Game, pid: usize, card: O, src: &str, ignore_energy: bool, extra_reqs: &[Req]) -> Res<Vec<Cand>> {
    let Some(im) = g.impl_o(card) else { return Ok(vec![]) };
    if play_forbidden(g, pid, card, src)? {
        return Ok(vec![]);
    }
    let typ = ty(g, card);
    let shows = show_options(g, pid, card, Some(im))?;
    let base: Vec<Map> = if typ == "Unit" {
        let mut b = vec![];
        for ch in unit_play_choices(g, pid, card, src, None)? {
            for sh in &shows {
                b.push(ch.merged(sh));
            }
        }
        b
    } else if typ == "Gear" {
        let mut b = vec![];
        for l in gear_locations(g, pid, card, None)? {
            for sh in &shows {
                b.push(crate::map! { loc = l }.merged(sh));
            }
        }
        b
    } else if typ == "Spell" {
        let ctx = mk(&[("hidden_bf", Value::None), ("card", card.into()), ("src", src.into())]);
        let base = if full_choices(g, Some(pid)) && !im.all_choices.is_none() {
            let v = im.all_choices.call(g, &[pid.into(), ctx.clone().into()])?;
            if v.truthy() {
                maps_of(&v)
            } else {
                vec![Map::new()]
            }
        } else if !im.choices.is_none() {
            maps_of(&im.choices.call(g, &[pid.into(), ctx.clone().into()])?)
        } else {
            vec![Map::new()]
        };
        let mut b = vec![];
        for ch in cap(g, base, 8, Some(pid)) {
            for sh in &shows {
                b.push(ch.merged(sh));
            }
        }
        spell_variants(g, pid, card, im, b, &ctx, false)?
    } else {
        return Ok(vec![]);
    };
    let mut out = vec![];
    for ch in base {
        let mut ch = ch.clone();
        if ignore_energy {
            ch.set("ignore_energy", true);
        } else {
            ch.set("free", true);
        }
        let (e, reqs) = total_cost(g, pid, card, &ch, src)?;
        let mut reqs = reqs;
        reqs.extend(extra_reqs.iter().cloned());
        let pc = pay_ctx(g, card);
        if g.can_pay(pid, e, &reqs, &pc)? {
            out.push((ch, e, reqs));
        }
    }
    Ok(out)
}

/// `_play_from(g, pid, card, src, cands, may)` : paie et joue un des candidats de `free_choices` (choisi par pid).
fn play_from(g: &mut Game, pid: usize, card: O, src: &str, cands: Vec<Cand>, may: bool) -> Res<Value> {
    if cands.is_empty() {
        return Ok(Value::None);
    }
    let mut opts: Vec<Value> = cands.iter().map(|c| Value::Map(c.0.clone())).collect();
    if may {
        opts.push(Value::None);
    }
    let ch = g.ask(pid, "play_choice", opts, crate::map! { card = card })?;
    if ch.is_none() {
        return Ok(Value::None);
    }
    let chm = ch.map().clone();
    let (_, e, reqs) = cands.iter().find(|c| c.0 == chm).cloned().unwrap();
    if e != 0 || !reqs.is_empty() {
        let pc = pay_ctx(g, card);
        if !g.pay(pid, e, &reqs, &pc)? {
            return Ok(Value::None);
        }
    }
    play_card(g, pid, card, src, chm, true)
}

/// `_played_card(info)` : évènement 'played' d'une carte (pas d'un jeton).
fn played_card(info: &Map) -> bool {
    !info.g("token").truthy() && !info.g("card").is_none()
}

/// `_exh_cost_choices(g, pid)` : « As an additional cost, exhaust a friendly unit » : les unités alliées prêtes (les
/// moins précieuses d'abord).
fn exh_cost_choices(g: &mut Game, pid: usize) -> Res<Vec<O>> {
    let us = g.units(Some(pid), None);
    let s = sort_by(g, us, |g, u| Ok((value(g, u)?, g.o(u).uid)))?;
    Ok(s.into_iter().filter(|&u| !g.exhausted(u)).collect())
}

fn pay_exh(g: &mut Game, ch: &Map) {
    let u = ch.g("exh").opt_int().and_then(|x| g.obj(x as u32));
    if let Some(u) = u {
        g.set_exhausted(u, true);
    }
}

fn pay_exh_v() -> Value {
    Value::func("_pay_exh", |g, a| {
        let ch = ctx_of(&a[2]);
        pay_exh(g, &ch);
        Ok(Value::None)
    })
}

/// `_pay_ret` : « As an additional cost, return a friendly unit/gear to its owner's hand » (ret / ret_gear). Le choix
/// modifié (ret_e / ret_gear_e) est rendu.
fn pay_ret_v() -> Value {
    Value::func("_pay_ret", |g, a| {
        let mut ch = ctx_of(&a[2]);
        for k in ["ret", "ret_gear"] {
            let x = ch.g(k).opt_int().and_then(|u| g.obj(u as u32));
            if let Some(x) = x {
                let e = if !g.o(x).token { e_of(g, x) } else { 0 }; // son coût en énergie (206)
                ch.set(&format!("{k}_e"), e);
                g.to_zone(x, "hand", false, None)?;
            }
        }
        Ok(Value::Map(ch))
    })
}

/// `_choose_up_to(n, cands_fn, pred, kind)` : choix à la finalisation de « jusqu'à n » cibles (355.13 : zéro
/// permis) ; le Deflect est payé (809).
fn choose_up_to(
    n: usize,
    cands_fn: impl Fn(&mut Game, ItemId) -> Res<Vec<O>> + Send + Sync + 'static,
    pred: Value,
    kind: &'static str,
) -> Value {
    bool_fn("_choose_up_to.choose", move |g, it| {
        let c = ctrl(g, it);
        let mut cs = vec![];
        for u in cands_fn(g, it)? {
            if deflect_ok(g, c, u)? {
                cs.push(u);
            }
        }
        let k = n.min(cs.len());
        let mut opts = vec![];
        let pool = cap(g, cs.clone(), 4, Some(c));
        let src = if !full_choices(g, Some(c)) { pool } else { cs.clone() };
        for m in (1..=k).rev() {
            for grp in combinations(&src, m) {
                let label = grp.iter().map(|&u| g.label(u)).collect::<Vec<_>>().join(", ");
                opts.push(opt(label, uids_t(g, &grp)));
            }
        }
        opts.push(opt(tr("aucune", "none"), Value::Tuple(vec![])));
        let pick = g.ask(c, kind, opts, crate::map! { item = it })?;
        for uid in pick.opt().value.list().to_vec() {
            let Some(u) = g.obj(uid.int() as u32) else { continue };
            if !pay_deflect(g, it, u)? {
                continue;
            }
            g.add_target(it, u, pred.clone());
        }
        Ok(true)
    })
}

fn mod_target(g: &mut Game, it: ItemId, n: i64, i: usize) -> Res<Option<O>> {
    let u = g.legal(it, i)?;
    if let Some(u) = u {
        g.mod_(u, n, turn(), None)?;
    }
    Ok(u)
}

/// `_gear_ctrl_of_item(g, item)` : contrôleur de l'équipement dont cet item est une capacité (activée, déclenchée ou
/// Deathknell), sinon None.
fn gear_ctrl_of_item(g: &Game, item: Option<ItemId>) -> Option<usize> {
    let item = item?;
    if g.it(item).kind == "spell" {
        return None;
    }
    let mut o = g.it(item).src.and_then(|u| g.obj(u));
    if o.is_none() {
        o = g.it(item).data.g("_src_obj").opt_obj();
    }
    if let Some(o) = o {
        if g.is_gear(o) {
            return Some(g.o(o).ctrl);
        }
    }
    if let Value::Map(info) = g.it(item).data.g("info") {
        if info.g("spec").opt_str().is_some_and(|n| spec(n).is_gear()) {
            return Some(info.g("ctrl").int() as usize);
        }
    }
    None
}

/// `_here_friend(g, src, o)`
fn here_friend(g: &Game, src: O, o: O) -> bool {
    o != src && ty(g, o) == "Unit" && g.o(o).ctrl == g.o(src).ctrl && is_bf(g.o(src).loc) && g.o(o).loc == g.o(src).loc
}

/// `_deal_res(n)`
fn deal_res(n: i64) -> Value {
    res_fn("_deal_res", move |g, it| {
        let u = g.legal(it, 0)?;
        if u.is_some() {
            let c = ctrl(g, it);
            g.deal(u, n, "ability", Some(c), None)?;
        }
        Ok(())
    })
}

/// `_mech_here(ready)`
fn mech_here(ready: bool) -> Value {
    res_fn("_mech_here", move |g, it| {
        let loc = match src_obj(g, it) {
            Some(m) => g.o(m).loc,
            None => dg(g, it, "loc").opt_loc(),
        };
        let c = ctrl(g, it);
        if let Some(loc) = loc {
            make_token(g, "Mech", c, loc, ready)?;
        }
        Ok(())
    })
}

/// `_plus_trigger(name, n, opts_fn, pred)` : « When you play me, give a unit +n might this turn » (n < 0 : « -n ») :
/// l'unité est choisie à la finalisation.
fn plus_trigger(name: &'static str, n: i64, opts: Option<Value>, pred: Value) -> Value {
    on_play_fn("_plus_trigger", move |g, o, ctx| {
        let hb = ctx.g("hidden_bf").opt_loc();
        let fnv = match &opts {
            Some(v) => v.clone(),
            None => {
                if n > 0 {
                    opts_fn("_buff_targets", move |g, it| {
                        let c = ctrl(g, it);
                        buff_targets(g, c, hb)
                    })
                } else {
                    opts_fn("_minus_targets", move |g, it| {
                        let c = ctrl(g, it);
                        minus_targets(g, c, hb, false)
                    })
                }
            }
        };
        let f = res_fn("_plus_trigger.res", move |g, it| mod_target(g, it, n, 0).map(|_| ()));
        q(g, o, name, f, Map::new(), false, trig_target(fnv, pred.clone(), "target", false, true), Value::None);
        Ok(())
    })
}

/// `_kill_cost_opts(g, pid, gear, units)`
fn kill_cost_opts(g: &mut Game, pid: usize, gear: bool, units: bool) -> Res<Vec<O>> {
    let xs: Vec<O> = g
        .board
        .clone()
        .into_iter()
        .filter(|&o| g.o(o).ctrl == pid && ((units && ty(g, o) == "Unit") || (gear && g.is_gear(o))))
        .collect();
    let s = sort_by(g, xs, |g, o| Ok((value(g, o)?, g.o(o).uid)))?;
    Ok(cap(g, s, 3, Some(pid)))
}

/// `_kill_ch(g, pid, kind)`
fn kill_ch(g: &mut Game, pid: usize, kind: &str) -> Res<Vec<Map>> {
    let xs: Vec<O> = g
        .board
        .clone()
        .into_iter()
        .filter(|&x| g.o(x).ctrl == pid && (if kind == "gear" { g.is_gear(x) } else { ty(g, x) == "Unit" }))
        .collect();
    let s = sort_by(g, xs, |g, x| Ok((value(g, x)?, g.o(x).uid)))?;
    Ok(cap(g, s, 3, Some(pid)).into_iter().map(|x| crate::map! { kill = g.o(x).uid }).collect())
}

fn kill_extra_v() -> Value {
    extra_fn("_kill_extra", |g, pid, _o, ch| {
        let x = ch.g("kill").opt_int().and_then(|u| g.obj(u as u32));
        if let Some(x) = x {
            g.kill(&[x], Some(pid), true)?;
        }
        Ok(())
    })
}

/// `_ready_runes(g, pid, n)`
fn ready_runes(g: &mut Game, pid: usize, n: i64) -> Res<()> {
    for _ in 0..n {
        let ex: Vec<_> = g.p[pid].runes.iter().copied().filter(|&r| g.r(r).exhausted).collect();
        if ex.is_empty() {
            return Ok(());
        }
        let doms: BTreeSet<String> = ex.iter().map(|&r| g.r(r).domain.clone()).collect();
        let r = if doms.len() > 1 {
            g.ask(pid, "ready_rune", ex.iter().map(|&r| Value::Rune(r)).collect(), Map::new())?.rune()
        } else {
            ex[0]
        };
        g.r_mut(r).exhausted = false;
    }
    Ok(())
}

/// `_exh_spell_choices(targets_fn)` : choix d'un sort avec le coût supplémentaire « exhaust a friendly unit » (exh) et
/// une cible (tg).
fn exh_spell_choices(targets_fn: impl Fn(&mut Game, usize, &Map) -> Res<Vec<Value>> + Send + Sync + 'static) -> Value {
    choices_fn("_exh_spell_choices", move |g, pid, ctx| {
        let mut out = vec![];
        let tgs = targets_fn(g, pid, ctx)?;
        let ex = exh_cost_choices(g, pid)?;
        for x in cap(g, ex, 2, Some(pid)) {
            for t in &tgs {
                out.push(crate::map! { tg = t.clone(), exh = g.o(x).uid });
            }
        }
        Ok(out)
    })
}

/// `_ret_spell_choices(targets_fn)`
fn ret_spell_choices(targets_fn: impl Fn(&mut Game, usize, &Map) -> Res<Vec<Value>> + Send + Sync + 'static) -> Value {
    choices_fn("_ret_spell_choices", move |g, pid, ctx| {
        let mut out = vec![];
        let tgs = targets_fn(g, pid, ctx)?;
        let us = g.units(Some(pid), None);
        let s = sort_by(g, us, |g, u| Ok((g.o(u).token, -e_of(g, u), g.o(u).uid)))?;
        let rets = cap(g, s, 2, Some(pid));
        for r in rets {
            let ru = g.o(r).uid;
            for t in &tgs {
                if !t.list().iter().any(|x| x.int() as u32 == ru) {
                    out.push(crate::map! { tg = t.clone(), ret = ru });
                }
            }
        }
        Ok(out)
    })
}

/// `_move_choices(g, us)`
fn move_choices(g: &Game, us: &[O]) -> Vec<Map> {
    let mut out = vec![];
    for &u in us {
        for d in g.locs() {
            if Some(d) != g.o(u).loc {
                out.push(crate::map! { tg = tg1(g, u), dest = d });
            }
        }
    }
    out
}

/// `_moved(g, u)`
fn moved(g: &Game, u: O) -> bool {
    let (uid, oid) = (g.o(u).uid, g.o(u).oid);
    g.hist.moved.contains(&(uid, oid))
}

/// `_has_hidden(g, x)`
fn has_hidden(g: &mut Game, x: O) -> Res<bool> {
    let Some(im) = g.impl_o(x) else { return Ok(false) };
    Ok((im.hidden || g.has_kw(x, "Hidden")?) && !g.o(x).token)
}

/// `_exh_cost_choices` + friends : unités à renforcer en premier (buff 0), puis les autres
fn buff_order(g: &mut Game, pid: usize) -> Res<Vec<O>> {
    let fr = friends(g, pid, false, None)?;
    let mut out: Vec<O> = fr.iter().copied().filter(|&u| g.o(u).buff == 0).collect();
    out.extend(fr.iter().copied().filter(|&u| g.o(u).buff != 0));
    Ok(out)
}

// ====================================================================== jetons
// Bomb (règle 187.12) — "[Deploy] [Deathknell][>] Deal 2 to an enemy unit here and kill all other Bombs you control
// here." L'unité ennemie est choisie quand le Deathknell est finalisé ; sans elle, les Bombs meurent quand même.
fn bomb_choose() -> Value {
    bool_fn("_bomb_choose", |g, it| {
        let info = dg(g, it, "info").map().clone();
        let c = ctrl(g, it);
        let mut cs = vec![];
        for u in enemies(g, c, false, None, true)? {
            if g.o(u).loc == info.g("loc").opt_loc() && deflect_ok(g, c, u)? {
                cs.push(u);
            }
        }
        if full_choices(g, Some(c)) {
            cs.sort_by_key(|&u| g.o(u).uid);
        }
        if !cs.is_empty() {
            let u = g.ask_obj(c, "bomb_target", &cs, crate::map! { item = it })?.unwrap();
            if pay_deflect(g, it, u)? {
                let pred = pred_fn("_bomb_choose.pred", |g, it_, x| {
                    Ok(g.o(x).ctrl != ctrl(g, it_) && g.o(x).loc == dg(g, it_, "info").map().g("loc").opt_loc())
                });
                g.add_target(it, u, pred);
            }
        }
        Ok(true)
    })
}

fn bomb_dk() -> Value {
    res_fn("_bomb_dk", |g, it| {
        let info = dg(g, it, "info").map().clone();
        let c = ctrl(g, it);
        let u = g.legal(it, 0)?;
        if u.is_some() {
            g.deal(u, 2, "ability", Some(c), None)?;
        }
        let (loc, uid) = (info.g("loc").opt_loc(), info.g("uid"));
        let others: Vec<O> = g
            .board
            .clone()
            .into_iter()
            .filter(|&o| g.cname(o) == "Bomb" && g.o(o).ctrl == c && g.o(o).loc == loc && Value::from(g.o(o).uid) != uid)
            .collect();
        if !others.is_empty() {
            g.kill(&others, Some(c), false)?;
        }
        Ok(())
    })
}

fn tokens(r: &mut Registry) {
    r.card(Impl { kw: crate::map! { Deploy = 1 }, deathknell: bomb_dk(), dk_choose: bomb_choose(), ..imp("Bomb") });
}

// ====================================================================== Fury
fn bomb_trigger(name: &'static str) -> Value {
    on_play_fn("_bomb_trigger", move |g, o, _ctx| {
        q0(g, o, name, res_fn("_bomb", |g, it| bomb(g, ctrl(g, it), None).map(|_| ())));
        Ok(())
    })
}

/// `_split_opts(g, pid, us, total)` : répartitions de dégâts entre les unités us : tuples (uid, n), les quelques
/// répartitions sensées de l'IA d'abord ; un humain a toutes les répartitions.
fn split_opts(g: &mut Game, pid: usize, us: &[O], total: i64) -> Res<Vec<Vec<(u32, i64)>>> {
    let mut out: Vec<Vec<(u32, i64)>> = vec![];
    let order = sort_by(g, us.to_vec(), |g, u| Ok((g.o(u).ctrl == pid, -value(g, u)?, g.o(u).uid)))?;
    let (mut left, mut al): (i64, Vec<(u32, i64)>) = (total, vec![]);
    for &u in &order {
        if left <= 0 || g.o(u).ctrl == pid {
            break;
        }
        let n = left.min(1.max(g.might(u)? - g.o(u).damage));
        al.push((g.o(u).uid, n));
        left -= n;
    }
    if !al.is_empty() && left != 0 {
        let l = al.len() - 1;
        al[l] = (al[l].0, al[l].1 + left);
    }
    if !al.is_empty() {
        out.push(al);
    }
    for &u in order.iter().take(2) {
        if g.o(u).ctrl != pid {
            out.push(vec![(g.o(u).uid, total)]);
        }
    }
    if full_choices(g, Some(pid)) {
        fn parts(k: i64, n: i64) -> Vec<Vec<i64>> {
            if k == 1 {
                return vec![vec![n]];
            }
            let mut v = vec![];
            for x in 1..(n - k + 2) {
                for rest in parts(k - 1, n - x) {
                    let mut p = vec![x];
                    p.extend(rest);
                    v.push(p);
                }
            }
            v
        }
        let mut su = us.to_vec();
        su.sort_by_key(|&u| g.o(u).uid);
        for k in 1..=(total.min(us.len() as i64)) {
            for grp in combinations(&su, k as usize) {
                for p in parts(k, total) {
                    out.push(grp.iter().zip(p.iter()).map(|(&u, &n)| (g.o(u).uid, n)).collect());
                }
            }
        }
    }
    let mut res: Vec<Vec<(u32, i64)>> = vec![];
    for a in out {
        if !a.is_empty() && !res.contains(&a) {
            res.push(a);
        }
    }
    Ok(res)
}

/// Rally for War, Cataclysm : battlefields ennemis occupés
fn occupied_enemy_bfs(g: &Game, pid: usize) -> Vec<usize> {
    (0..g.bfs.len())
        .filter(|&i| g.bfs[i].ctrl == Some(1 - pid) && !g.units(None, Some(Loc::Bf(i))).is_empty())
        .collect()
}

/// Forbidden Technique — zone_event
fn ft_zone() -> Value {
    on_event_fn("_ft_zone", |g, c, ev, info| {
        if ev == "played"
            && zone_is(g, c, Zone::Trash)
            && info.g("pid").int() as usize == g.o(c).owner
            && played_card(info)
            && e_of(g, info.g("card").obj()) >= 7
        {
            let res = res_fn("_ft_zone.res", |g, it| {
                let c = ctrl(g, it);
                let uid = dg(g, it, "uid").int() as u32;
                let Some(x) = g.p[c].trash.iter().copied().find(|&y| g.o(y).uid == uid) else { return Ok(()) };
                let cands = free_choices(g, c, x, "trash", false, &[req(&["Fury"]), req(&["Fury"])])?;
                let mut cs = vec![];
                for (ch, e, r) in cands {
                    let pc = pay_ctx(g, x);
                    if g.can_pay(c, e + 1, &r, &pc)? {
                        cs.push((ch, e + 1, r));
                    }
                }
                play_from(g, c, x, "trash", cs, true)?;
                Ok(())
            });
            let (owner, uid) = (g.o(c).owner, g.o(c).uid);
            g.queue_trigger(owner, "Forbidden Technique (trash)", res, crate::map! { uid = uid }, None, false, Value::None, Value::None);
        }
        Ok(())
    })
}

fn banish_trigger(name: &'static str) -> Value {
    res_fn(name, |g, it| banish_playable(g, ctrl(g, it)).map(|_| ()))
}

fn fury(r: &mut Registry) {
    // Amateur Demolitionist — "When you play me, play a Bomb gear token to a battlefield."
    r.card(Impl { on_play: bomb_trigger("Amateur Demolitionist"), ..imp("Amateur Demolitionist") });

    // Banner of Conquest — "[Deploy] Friendly units here have [Assault]."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        aura_kw: aura_kw_fn("Banner of Conquest.aura_kw", |g, src, o| {
            let s = src.obj();
            Ok(if zone_is(g, s, Zone::Board) && here_friend(g, s, o) { Some(crate::map! { Assault = 1 }) } else { None })
        }),
        ..imp("Banner of Conquest")
    });

    // Bilgewater Tracker — "When you play me, deal 1 to each of up to 2 units."
    r.card(Impl {
        on_play: on_play_fn("Bilgewater Tracker.on_play", |g, o, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let res = res_fn("_tracker_res", |g, it| {
                for i in 0..g.it(it).targets.len() {
                    let u = g.legal(it, i)?;
                    if u.is_some() {
                        let c = ctrl(g, it);
                        g.deal(u, 1, "ability", Some(c), None)?;
                    }
                }
                Ok(())
            });
            let choose = choose_up_to(
                2,
                move |g, it| {
                    let c = ctrl(g, it);
                    all_units(g, c, false, hb)
                },
                p_unit_v(),
                "tracker_targets",
            );
            q(g, o, "Bilgewater Tracker", res, Map::new(), false, choose, Value::None);
            Ok(())
        }),
        ..imp("Bilgewater Tracker")
    });

    // Brawl — "[Action] Give a unit [Assault] this turn equal to its Might." (sa Might à la résolution)
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_v()]),
        resolve: res_fn("_brawl", |g, it| {
            if let Some(u) = g.legal(it, 0)? {
                if g.might(u)? > 0 {
                    let m = g.might(u)?;
                    g.grant(u, "Assault", m, turn());
                }
            }
            Ok(())
        }),
        choices: choices_fn("Brawl.choices", |g, pid, ctx| {
            let us = buff_targets(g, pid, ctx.g("hidden_bf").opt_loc())?;
            Ok(tg_choices(g, &us))
        }),
        ..imp("Brawl")
    });

    // Lingering Animus — "I enter ready if a friendly unit or gear died this turn."
    r.card(Impl {
        enter_ready: card_ch_bool("Lingering Animus.enter_ready", |g, pid, _c, _ch| {
            Ok(g.hist.died.iter().any(|inf| inf.g("ctrl").int() as usize == pid && matches!(inf_type(inf), "Unit" | "Gear")))
        }),
        ..imp("Lingering Animus")
    });

    // Scorched Earth — "[Action] Deal 5 damage split among units at a single battlefield." (règle 355.14 : chaque
    // unité choisie reçoit au moins 1)
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_bf_v()]),
        resolve: res_fn("_scorched", |g, it| {
            let split = dg(g, it, "split");
            for (i, n) in split.list().to_vec().iter().enumerate() {
                let u = g.legal(it, i)?;
                if u.is_some() {
                    let c = ctrl(g, it);
                    g.deal(u, n.int(), "spell", Some(c), None)?;
                }
            }
            Ok(())
        }),
        choices: choices_fn("_scorched_choices", |g, pid, ctx| {
            let mut out = vec![];
            let hb = ctx.g("hidden_bf").opt_loc();
            for b in g.bf_locs() {
                if hb.is_some() && Some(b) != hb {
                    continue;
                }
                let mut us = vec![];
                for u in g.all_units() {
                    if g.o(u).loc == Some(b) && g.targetable(u, Some(pid))? {
                        us.push(u);
                    }
                }
                for a in split_opts(g, pid, &us, 5)? {
                    out.push(crate::map! {
                        tg = Value::Tuple(a.iter().map(|&(x, _)| Value::from(x)).collect()),
                        split = Value::Tuple(a.iter().map(|&(_, n)| Value::from(n)).collect()),
                    });
                }
            }
            Ok(out)
        }),
        max_choices: 12,
        ..imp("Scorched Earth")
    });

    // Rally for War — "Choose an occupied enemy battlefield. Play three 1 might Recruit unit tokens there."
    r.card(Impl {
        resolve: res_fn("_rally", |g, it| {
            let b = dg(g, it, "bf").int() as usize;
            let c = ctrl(g, it);
            if g.bfs[b].ctrl.is_none() || g.bfs[b].ctrl == Some(c) || g.units(None, Some(Loc::Bf(b))).is_empty() {
                return Ok(()); // n'est plus un battlefield ennemi occupé (359.3.e)
            }
            for _ in 0..3 {
                make_token(g, "Recruit", c, Loc::Bf(b), false)?;
            }
            Ok(())
        }),
        choices: choices_fn("Rally for War.choices", |g, pid, _ctx| {
            Ok(occupied_enemy_bfs(g, pid).into_iter().map(|b| crate::map! { bf = b }).collect())
        }),
        ..imp("Rally for War")
    });

    // War Scout — "[Accelerate] When you play me, give a unit +2 might this turn."
    r.card(Impl { accelerate: true, on_play: plus_trigger("War Scout", 2, None, p_unit_v()), ..imp("War Scout") });

    // Antagonistic Apparatus — "I enter ready. When you play me, play a ready 3 might Mech unit token here."
    r.card(Impl {
        enter_ready: Value::Bool(true),
        on_play: on_play_fn("Antagonistic Apparatus.on_play", |g, o, _ctx| {
            let loc: Value = g.o(o).loc.into();
            q(g, o, "Antagonistic Apparatus", mech_here(true), crate::map! { loc = loc }, false, Value::None, Value::None);
            Ok(())
        }),
        ..imp("Antagonistic Apparatus")
    });

    // Forbidden Technique — "Ready a unit. When you play a card with Energy cost 7 or more, you may play this from your
    // trash for [1][Fury][Fury]. Banish this." (« Banish this » est une instruction du sort : il est banni à sa
    // résolution, d'où qu'il ait été joué.)
    r.card(Impl {
        preds: Value::List(vec![p_unit_v()]),
        banish_after: true,
        hooks: mk(&[("zone_event", ft_zone())]),
        resolve: res_fn("Forbidden Technique.resolve", |g, it| {
            if let Some(u) = g.legal(it, 0)? {
                g.ready_obj(u, None, None)?;
            }
            Ok(())
        }),
        choices: choices_fn("Forbidden Technique.choices", |g, pid, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let mut us: Vec<O> = friends(g, pid, false, hb)?.into_iter().filter(|&u| g.exhausted(u)).collect();
            us.extend(all_units(g, pid, false, hb)?.into_iter().filter(|&u| g.o(u).ctrl == pid && !g.exhausted(u)));
            Ok(tg_choices(g, &us))
        }),
        ..imp("Forbidden Technique")
    });

    // Incorrigible Construct — "[Show Off] a Mech. I enter ready if you showed off."
    r.card(Impl {
        show_off: show_off_fn("Incorrigible Construct.show_off", |g, _pid, _c, x| {
            Ok(if g.board.contains(&x) { g.tags(x)?.contains("Mech") } else { g.spec(x).tags.contains("Mech") })
        }),
        enter_ready: card_ch_bool("Incorrigible Construct.enter_ready", |_g, _pid, _c, ch| Ok(!ch.g("show").is_none())),
        ..imp("Incorrigible Construct")
    });

    // Runeblade Anvil — "When you play this or when this leaves the board, banish the top card of your Main Deck. Until
    // your next Ending Phase, you may play that card. [2][Fury], [E]: Kill this."
    r.card(Impl {
        on_leave: Value::func("_anvil_leave", |g, a| {
            let info = ctx_of(&a[1]);
            let c = info.g("info").map().g("ctrl").int() as usize;
            g.queue_trigger(c, "Runeblade Anvil (leaves)", banish_trigger("_anvil_leave.res"), Map::new(), None, false, Value::None, Value::None);
            Ok(Value::None)
        }),
        on_play: on_play_fn("Runeblade Anvil.on_play", |g, o, _ctx| {
            q0(g, o, "Runeblade Anvil", banish_trigger("_banish_playable"));
            Ok(())
        }),
        abilities: vec![ab(
            "Kill this",
            Cost::from("2 energy and 1 fury rune"),
            AbilityArgs {
                exhaust: true,
                resolve: res_fn("Runeblade Anvil.kill", |g, it| {
                    if let Some(s) = src_obj(g, it) {
                        let c = ctrl(g, it);
                        g.kill(&[s], Some(c), false)?;
                    }
                    Ok(())
                }),
                ..Default::default()
            },
        )],
        ..imp("Runeblade Anvil")
    });

    // Lost to the Sands — "[Action] Choose one — Deal 4 to a unit at a battlefield. Kill a gear."
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![pred_fn("_lts_pred", |g, it, o| {
            Ok(if dg(g, it, "mode").opt_str() == Some("deal") { p_unit_bf(g, it, o) } else { g.is_gear(o) })
        })]),
        resolve: res_fn("_lts", |g, it| {
            let Some(o) = g.legal(it, 0)? else { return Ok(()) };
            let c = ctrl(g, it);
            if dg(g, it, "mode").opt_str() == Some("deal") {
                g.deal(Some(o), 4, "spell", Some(c), None)?;
            } else {
                g.kill(&[o], Some(c), false)?;
            }
            Ok(())
        }),
        choices: choices_fn("_lts_choices", |g, pid, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let mut out: Vec<Map> = minus_targets(g, pid, hb, true)?
                .into_iter()
                .map(|u| crate::map! { mode = "deal", tg = tg1(g, u) })
                .collect();
            let gs: Vec<O> = g.gear(None).into_iter().filter(|&x| hb.is_none() || g.o(x).loc == hb).collect();
            let gs = sort_by(g, gs, |g, x| Ok((g.o(x).ctrl == pid, -e_of(g, x), g.o(x).uid)))?;
            for x in gs {
                if g.targetable(x, Some(pid))? {
                    out.push(crate::map! { mode = "gear", tg = tg1(g, x) });
                }
            }
            Ok(out)
        }),
        max_choices: 10,
        ..imp("Lost to the Sands")
    });

    // Shimmering Swimmer — "You may kill a friendly unit or gear as an additional cost to play me. When you play me, if
    // you paid the additional cost, deal 2 to a unit."
    r.card(Impl {
        as_played: as_played_fn("Shimmering Swimmer.as_played", |g, pid, _c| {
            let mut out = vec![Map::new()];
            for o in kill_cost_opts(g, pid, true, true)? {
                out.push(crate::map! { kill = g.o(o).uid });
            }
            Ok(out)
        }),
        on_play: on_play_fn("Shimmering Swimmer.on_play", |g, o, ctx| {
            if !ctx.g("kill").is_none() {
                let hb = ctx.g("hidden_bf").opt_loc();
                let opts = opts_fn("Shimmering Swimmer.opts", move |g, it| {
                    let c = ctrl(g, it);
                    minus_targets(g, c, hb, false)
                });
                q(g, o, "Shimmering Swimmer", deal_res(2), Map::new(), false, trig_target(opts, p_unit_v(), "target", false, true), Value::None);
            }
            Ok(())
        }),
        ..imp("Shimmering Swimmer")
    });

    // Akali, Brash — "The first time I move each turn, [Add] [1][A]. The second time I move each turn, banish the top
    // card of your Main Deck. Until your next Ending Phase, you may play that card." ([Add] se résout aussitôt, 430.2)
    r.card(Impl {
        on_event: on_event_fn("_akali_brash", |g, o, ev, info| {
            if ev == "move" && info.g("obj").opt_obj() == Some(o) {
                let key = skey(&["akali_brash".into(), g.o(o).uid.into(), g.o(o).oid.into(), g.turn_no.into()]);
                g.stats.add(&key, 1);
                if g.stats.get(&key) == 1 {
                    let c = g.o(o).ctrl;
                    g.add_pool(c, 1, &["A"]);
                } else if g.stats.get(&key) == 2 {
                    q0(g, o, "Akali, Brash", banish_trigger("_banish_playable"));
                }
            }
            Ok(())
        }),
        ..imp("Akali, Brash")
    });

    // Coalesced Energy — "At the end of your turn, recycle all your runes, then channel 4 runes exhausted."
    r.card(Impl {
        on_event: on_event_fn("Coalesced Energy.on_event", |g, o, ev, info| {
            if ev == "end_turn" && info.g("pid").int() as usize == g.o(o).ctrl {
                q0(
                    g,
                    o,
                    "Coalesced Energy",
                    res_fn("_coalesced_res", |g, it| {
                        let c = ctrl(g, it);
                        for r in g.p[c].runes.clone() {
                            g.recycle_rune(c, r)?;
                        }
                        g.channel(c, 4, true)?;
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Coalesced Energy")
    });

    // Detonation Amplifier — "[Deploy] When you play this, you may kill another friendly unit or gear to draw 1.
    // Friendly [Deathknell] effects here trigger an additional time."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        on_play: on_play_fn("_amp_play", |g, o, _ctx| {
            let res = res_fn("_amp_play.res", |g, it| {
                let c = ctrl(g, it);
                let src = g.it(it).src;
                let xs: Vec<O> = g
                    .board
                    .clone()
                    .into_iter()
                    .filter(|&x| g.o(x).ctrl == c && Some(g.o(x).uid) != src && (ty(g, x) == "Unit" || g.is_gear(x)))
                    .collect();
                if xs.is_empty() {
                    return Ok(());
                }
                let s = sort_by(g, xs, |g, x| Ok((value(g, x)?, g.o(x).uid)))?;
                let mut opts: Vec<Value> = s.iter().map(|&x| Value::Obj(x)).collect();
                opts.push(Value::None);
                let x = g.ask(c, "amp_kill", opts, Map::new())?.opt_obj();
                if let Some(x) = x {
                    if !g.kill(&[x], Some(c), true)?.is_empty() {
                        g.draw(c, 1)?;
                    }
                }
                Ok(())
            });
            q0(g, o, "Detonation Amplifier", res);
            Ok(())
        }),
        hooks: mk(&[(
            "dk_extra",
            Value::func("Detonation Amplifier.dk_extra", |g, a| {
                let s = a[0].obj();
                let inf = ctx_of(&a[1]);
                let ok = zone_is(g, s, Zone::Board)
                    && is_bf(g.o(s).loc)
                    && inf.g("ctrl").int() as usize == g.o(s).ctrl
                    && inf.g("loc").opt_loc() == g.o(s).loc
                    && inf.g("uid") != Value::from(g.o(s).uid);
                Ok(Value::Int(if ok { 1 } else { 0 }))
            }),
        )]),
        ..imp("Detonation Amplifier")
    });

    // Drakehound Packmaster — "When you play me, reveal the top card of each opponent's Main Deck. Choose one and banish
    // it, then play it, ignoring its cost. Recycle the rest." (1v1 : une carte révélée ; 419.3.c : si elle ne peut pas
    // être jouée, elle reste bannie)
    r.card(Impl {
        on_play: on_play_fn("Drakehound Packmaster.on_play", |g, o, _ctx| {
            let res = res_fn("_packmaster", |g, it| {
                let c = ctrl(g, it);
                let top = g.reveal(c, 1, Some(1 - c), true, None)?;
                if top.is_empty() {
                    return Ok(());
                }
                let x = top[0];
                g.to_zone(x, "banish", false, Some(c))?;
                let cands = free_choices(g, c, x, "banish", false, &[])?;
                play_from(g, c, x, "banish", cands, false)?;
                Ok(())
            });
            q0(g, o, "Drakehound Packmaster", res);
            Ok(())
        }),
        ..imp("Drakehound Packmaster")
    });

    // Firebrand Raider — "I enter ready. When you play me, ready all enemy units. Other units enter ready. Units don't
    // ready during the Awaken Phase."
    r.card(Impl {
        enter_ready: Value::Bool(true),
        on_play: on_play_fn("Firebrand Raider.on_play", |g, o, _ctx| {
            q0(
                g,
                o,
                "Firebrand Raider",
                res_fn("_raider_res", |g, it| {
                    let c = ctrl(g, it);
                    for u in g.units(Some(1 - c), None) {
                        g.ready_obj(u, None, None)?;
                    }
                    Ok(())
                }),
            );
            Ok(())
        }),
        hooks: mk(&[
            (
                "units_enter_ready",
                Value::func("Firebrand Raider.units_enter_ready", |g, a| {
                    let (s, c) = (a[0].obj(), a[2].obj());
                    Ok(Value::Bool(c != s && ty(g, c) == "Unit" && zone_is(g, s, Zone::Board)))
                }),
            ),
            (
                "aura_no_ready",
                Value::func("Firebrand Raider.aura_no_ready", |g, a| {
                    let o = a[1].obj();
                    Ok(Value::Bool(a[3].opt_str() == Some("awaken") && ty(g, o) == "Unit"))
                }),
            ),
        ]),
        ..imp("Firebrand Raider")
    });

    // Mordekaiser, Twice Slain — "[Accelerate] [Assault 3] Once each turn, if I would die, you may kill a friendly unit
    // to heal me, exhaust me, and recall me instead."
    r.card(Impl {
        accelerate: true,
        kw: crate::map! { Assault = 3 },
        hooks: mk(&[(
            "death_rep",
            Value::func("_morde_rep", |g, a| {
                let s = a[0].obj();
                let u = a[1].obj();
                let key = skey(&["morde_twice".into(), g.o(s).uid.into(), g.o(s).oid.into(), g.turn_no.into()]);
                if u != s || g.stats.get(&key) != 0 {
                    return Ok(Value::None);
                }
                let sc = g.o(s).ctrl;
                let others: Vec<O> =
                    g.units(Some(sc), None).into_iter().filter(|&x| x != s && !g.dying.contains(&x)).collect();
                if others.is_empty() {
                    return Ok(Value::None);
                }
                let pid = sc;
                let suid = g.o(s).uid;
                let cost = Value::func("_morde_rep.cost", move |g, _a| {
                    let xs: Vec<O> = g
                        .units(Some(pid), None)
                        .into_iter()
                        .filter(|&x| g.o(x).uid != suid && !g.dying.contains(&x))
                        .collect();
                    if xs.is_empty() {
                        return Ok(Value::Bool(false));
                    }
                    let xs = sort_by(g, xs, |g, x| Ok((value(g, x)?, g.o(x).uid)))?;
                    let x = g.ask_obj(pid, "morde_kill", &xs, Map::new())?.unwrap();
                    let k = skey(&["morde_twice".into(), suid.into(), g.o(s).oid.into(), g.turn_no.into()]);
                    stat_set(g, &k, 1);
                    Ok(Value::Bool(!g.kill(&[x], Some(pid), true)?.is_empty()))
                });
                let apply = Value::func("_morde_rep.apply", |g, a| {
                    g.save_unit(a[0].obj(), true, true);
                    Ok(Value::None)
                });
                Ok(Value::Map(mk(&[
                    ("name", "Mordekaiser, Twice Slain".into()),
                    ("may", true.into()),
                    ("cost", cost),
                    ("apply", apply),
                ])))
            }),
        )]),
        ..imp("Mordekaiser, Twice Slain")
    });

    // Ekko, Revolutionary — "I enter ready. [Assault] [Action][>] Discard 1, [1][Fury]: Banish me, then play me to your
    // base, ignoring my cost."
    r.card(Impl {
        enter_ready: Value::Bool(true),
        kw: crate::map! { Assault = 1 },
        abilities: vec![ab(
            "Banish and replay",
            Cost::from("1 energy and 1 fury rune"),
            AbilityArgs {
                timing: "action",
                resolve: res_fn("_ekko_rev", |g, it| {
                    let Some(m) = src_obj(g, it) else { return Ok(()) };
                    let c = ctrl(g, it);
                    g.to_zone(m, "banish", false, Some(c))?;
                    let (owner, uid) = (g.o(m).owner, g.o(m).uid);
                    let x = g.p[owner].banish.iter().copied().find(|&x| g.o(x).uid == uid);
                    if let Some(x) = x {
                        play_unit_free(g, c, x, "banish", Some(vec![Loc::Base]), false, None)?;
                    }
                    Ok(())
                }),
                can: can_fn("Ekko, Revolutionary.can", |g, pid, _o| Ok(!g.p[pid].hand.is_empty())),
                extra_cost: extra_fn("Ekko, Revolutionary.extra_cost", |g, pid, _o, _ch| discard_n(g, pid, 1)),
                ..Default::default()
            },
        )],
        ..imp("Ekko, Revolutionary")
    });

    // Whirling Twirligig — "[E]: Play a Gold gear token exhausted. Kill a friendly gear, [E]: Play a 1 might Recruit unit
    // token. Kill a friendly unit, [E]: Play a 3 might Mech unit token."
    r.card(Impl {
        abilities: vec![
            ab(
                "Gold",
                Cost::None,
                AbilityArgs {
                    exhaust: true,
                    resolve: res_fn("Whirling Twirligig.gold", |g, it| gold(g, ctrl(g, it), false).map(|_| ())),
                    ..Default::default()
                },
            ),
            ab(
                "Recruit",
                Cost::None,
                AbilityArgs {
                    exhaust: true,
                    choices: ab_choices_fn("Whirling Twirligig.recruit_choices", |g, pid, _o| kill_ch(g, pid, "gear")),
                    extra_cost: kill_extra_v(),
                    resolve: res_fn("Whirling Twirligig.recruit", |g, it| unit_token(g, ctrl(g, it), "Recruit").map(|_| ())),
                    ..Default::default()
                },
            ),
            ab(
                "Mech",
                Cost::None,
                AbilityArgs {
                    exhaust: true,
                    choices: ab_choices_fn("Whirling Twirligig.mech_choices", |g, pid, _o| kill_ch(g, pid, "unit")),
                    extra_cost: kill_extra_v(),
                    resolve: res_fn("Whirling Twirligig.mech", |g, it| unit_token(g, ctrl(g, it), "Mech").map(|_| ())),
                    ..Default::default()
                },
            ),
        ],
        ..imp("Whirling Twirligig")
    });

    // Ziggs, Short-Fused — "When you play me, play a Bomb gear token to a battlefield. Abilities of friendly gear deal 1
    // Bonus Damage." (Bonus Damage, 712-715 : capacités dont la source est un équipement du contrôleur de Ziggs)
    r.card(Impl {
        on_play: bomb_trigger("Ziggs, Short-Fused"),
        hooks: mk(&[(
            "dmg_bonus",
            Value::func("Ziggs, Short-Fused.dmg_bonus", |g, a| {
                let s = a[0].obj();
                let ctx = ctx_of(&a[1]);
                let item = match ctx.g("item") {
                    Value::Item(i) => Some(i),
                    _ => None,
                };
                let ok = zone_is(g, s, Zone::Board)
                    && ctx.g("kind").opt_str() == Some("ability")
                    && gear_ctrl_of_item(g, item) == Some(g.o(s).ctrl);
                Ok(Value::Int(if ok { 1 } else { 0 }))
            }),
        )]),
        ..imp("Ziggs, Short-Fused")
    });
}

// ====================================================================== Calm
/// effet « can't move this turn » (Entrenched Boar, Hunker Down)
fn cant_move_effect(g: &mut Game, u: O) {
    let (uid, oid) = (g.o(u).uid, g.o(u).oid);
    let f = Value::func("cant_move.fn", |g, a| {
        let eff = g.eff(a[0].int() as u64).cloned().unwrap_or_default();
        let x = a[1].obj();
        Ok(Value::Bool(Value::from(g.o(x).uid) == eff.g("uid") && Value::from(g.o(x).oid) == eff.g("oid")))
    });
    g.add_effect(mk(&[
        ("kind", "cant_move".into()),
        ("uid", uid.into()),
        ("oid", oid.into()),
        ("dur", turn()),
        ("fn", f),
    ]));
}

/// les cartes 'played' d'énergie 7 ou plus du contrôleur (Sanctum Conservator...)
fn played_big(g: &Game, o: O, info: &Map) -> bool {
    info.g("pid").int() as usize == g.o(o).ctrl && played_card(info) && e_of(g, info.g("card").obj()) >= 7
}

/// `objs` d'un évènement 'exhausted' contient o
fn in_objs(info: &Map, k: &str, o: O) -> bool {
    info.g(k).list().iter().any(|x| x.opt_obj() == Some(o))
}

/// `_command_attack` (mode look) et Trifarian Captain : regarder n cartes, en révéler une et la piocher, recycler le
/// reste.
fn look_pick(g: &mut Game, c: usize, n: usize, kind: &str, filt: impl Fn(&Game, O) -> bool) -> Res<()> {
    let deck: Vec<O> = g.p[c].deck.iter().take(n).copied().collect();
    let top = g.look(c, &deck, None)?;
    let gs: Vec<O> = top.iter().copied().filter(|&x| filt(g, x)).collect();
    let pick = if !gs.is_empty() {
        let s = sort_by(g, gs, |g, x| Ok((-cval(g, x), g.o(x).uid)))?;
        let mut opts: Vec<Value> = s.iter().map(|&x| Value::Obj(x)).collect();
        opts.push(Value::None);
        g.ask(c, kind, opts, Map::new())?.opt_obj()
    } else {
        None
    };
    if let Some(p) = pick {
        let cn = g.cname(p).to_string();
        g.log(|| format!("  P{c} reveals {cn}"));
        g.draw_card(c, p)?;
    }
    let rest: Vec<O> = top.iter().copied().filter(|&x| Some(x) != pick && g.p[c].deck.contains(&x)).collect();
    for &x in &rest {
        deck_remove(g, c, x);
    }
    g.recycle_cards(c, &rest)
}

fn me_mod(name: &'static str, n: i64) -> Value {
    res_fn(name, move |g, it| {
        if let Some(m) = me(g, it) {
            g.mod_(m, n, turn(), None)?;
        }
        Ok(())
    })
}

fn draw1(name: &'static str) -> Value {
    res_fn(name, |g, it| g.draw(ctrl(g, it), 1))
}

fn soul_ab() -> Map {
    ab(
        "Give +3",
        Cost::None,
        AbilityArgs {
            exhaust: true,
            preds: Value::List(vec![p_unit_v()]),
            resolve: res_fn("_SOUL_AB.resolve", |g, it| mod_target(g, it, 3, 0).map(|_| ())),
            choices: ab_choices_fn("_SOUL_AB.choices", |g, pid, _o| {
                let us = buff_targets(g, pid, None)?;
                Ok(tg_choices(g, &us))
            }),
            ..Default::default()
        },
    )
}

const RIVEN_MODES: [&str; 3] = ["buff", "stun", "ready"];

fn calm(r: &mut Registry) {
    // Blossoming Dreamroot — "[Deploy] When you win a combat here, draw 1."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        on_event: on_event_fn("Blossoming Dreamroot.on_event", |g, o, ev, info| {
            if ev == "combat_won"
                && info.g("pid").int() as usize == g.o(o).ctrl
                && Some(Loc::Bf(info.g("bf").int() as usize)) == g.o(o).loc
            {
                q0(g, o, "Blossoming Dreamroot", draw1("Blossoming Dreamroot.res"));
            }
            Ok(())
        }),
        ..imp("Blossoming Dreamroot")
    });

    // Consult the Heavens — "[Reaction] Give a unit [Shield 2] this turn. [Predict]."
    r.card(Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![p_unit_v()]),
        resolve: res_fn("_consult", |g, it| {
            if let Some(u) = g.legal(it, 0)? {
                g.grant(u, "Shield", 2, turn());
            }
            g.predict(ctrl(g, it), 1)?;
            Ok(())
        }),
        choices: choices_fn("Consult the Heavens.choices", |g, pid, ctx| {
            let us = buff_targets(g, pid, ctx.g("hidden_bf").opt_loc())?;
            Ok(tg_choices(g, &us))
        }),
        ..imp("Consult the Heavens")
    });

    // Entrenched Boar — "You may have me enter ready. If you do, I can't move this turn."
    r.card(Impl {
        as_played: as_played_fn("Entrenched Boar.as_played", |_g, _pid, _c| Ok(vec![Map::new(), crate::map! { boar = true }])),
        enter_ready: card_ch_bool("Entrenched Boar.enter_ready", |_g, _pid, _c, ch| Ok(ch.g("boar").truthy())),
        on_play: on_play_fn("_boar_play", |g, o, ctx| {
            if ctx.g("boar").truthy() {
                cant_move_effect(g, o);
            }
            Ok(())
        }),
        ..imp("Entrenched Boar")
    });

    // Iridescent Avian — "[E]: [Buff] a unit. If it's already buffed, give it +2 might this turn instead."
    r.card(Impl {
        abilities: vec![ab(
            "Buff",
            Cost::None,
            AbilityArgs {
                exhaust: true,
                preds: Value::List(vec![p_unit_v()]),
                resolve: res_fn("_avian", |g, it| {
                    let Some(u) = g.legal(it, 0)? else { return Ok(()) };
                    if g.o(u).buff > 0 {
                        g.mod_(u, 2, turn(), None)?;
                    } else {
                        g.buff(Some(u), None)?;
                    }
                    Ok(())
                }),
                choices: ab_choices_fn("Iridescent Avian.choices", |g, pid, _o| {
                    let mut us = buff_order(g, pid)?;
                    if full_choices(g, Some(pid)) {
                        us.extend(all_units(g, pid, false, None)?.into_iter().filter(|&u| g.o(u).ctrl != pid));
                    }
                    Ok(tg_choices(g, &us))
                }),
                ..Default::default()
            },
        )],
        ..imp("Iridescent Avian")
    });

    // Ki Burst — "[Action] As an additional cost to play this, exhaust a friendly unit. [Stun] a unit."
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_v()]),
        pay_extra: pay_exh_v(),
        resolve: res_fn("Ki Burst.resolve", |g, it| {
            let u = g.legal(it, 0)?;
            if u.is_some() {
                let c = ctrl(g, it);
                g.stun(u, Some(c))?;
            }
            Ok(())
        }),
        choices: exh_spell_choices(|g, pid, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let mut out = vec![];
            for u in enemies(g, pid, false, hb, true)? {
                if !g.o(u).stunned || full_choices(g, Some(pid)) {
                    out.push(tg1(g, u));
                }
            }
            if full_choices(g, Some(pid)) {
                for u in friends(g, pid, false, hb)? {
                    out.push(tg1(g, u));
                }
            }
            Ok(out)
        }),
        ..imp("Ki Burst")
    });

    // Life from Stone — "[Reaction] As an additional cost to play this, exhaust a friendly unit. Draw 1 and channel 1
    // rune exhausted."
    r.card(Impl {
        timing: Some("reaction"),
        pay_extra: pay_exh_v(),
        resolve: res_fn("Life from Stone.resolve", |g, it| {
            let c = ctrl(g, it);
            g.draw(c, 1)?;
            g.channel(c, 1, true)?;
            Ok(())
        }),
        choices: choices_fn("Life from Stone.choices", |g, pid, _ctx| {
            let ex = exh_cost_choices(g, pid)?;
            Ok(cap(g, ex, 2, Some(pid)).into_iter().map(|x| crate::map! { exh = g.o(x).uid }).collect())
        }),
        ..imp("Life from Stone")
    });

    // Sanctum Conservator — "When you play a card with Energy cost 7 or more, ready me." (coût imprimé, 206)
    r.card(Impl {
        on_event: on_event_fn("Sanctum Conservator.on_event", |g, o, ev, info| {
            if ev == "played" && played_big(g, o, info) {
                q0(
                    g,
                    o,
                    "Sanctum Conservator",
                    res_fn("Sanctum Conservator.res", |g, it| {
                        if let Some(m) = me(g, it) {
                            g.ready_obj(m, None, None)?;
                        }
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Sanctum Conservator")
    });

    // Stoic Ferryman — "[Shield 3] When you play me to a battlefield, you may move another unit you control here to your
    // base."
    r.card(Impl {
        kw: crate::map! { Shield = 3 },
        on_play: on_play_fn("_ferryman", |g, o, _ctx| {
            if !is_bf(g.o(o).loc) {
                return Ok(());
            }
            let here = g.o(o).loc;
            let res = res_fn("_ferryman.res", |g, it| {
                if let Some(u) = g.legal(it, 0)? {
                    let c = ctrl(g, it);
                    g.move_(&[u], Loc::Base, Some(c), false)?;
                }
                Ok(())
            });
            let opts = opts_fn("_ferryman.opts", move |g, it| {
                let c = ctrl(g, it);
                let src = g.it(it).src;
                Ok(friends(g, c, false, None)?.into_iter().filter(|&u| g.o(u).loc == here && Some(g.o(u).uid) != src).collect())
            });
            let pred = pred_fn("_ferryman.pred", move |g, it, u| {
                Ok(g.o(u).ctrl == ctrl(g, it) && g.o(u).loc == here && Some(g.o(u).uid) != g.it(it).src)
            });
            q(g, o, "Stoic Ferryman", res, Map::new(), true, trig_target(opts, pred, "target", false, true), Value::None);
            Ok(())
        }),
        ..imp("Stoic Ferryman")
    });

    // Battle Preparations — "[Reaction] [Show Off] a gear. Give a unit +2 might this turn. If you showed off, ready up
    // to 2 runes."
    r.card(Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![p_unit_v()]),
        resolve: res_fn("_battle_prep", |g, it| {
            mod_target(g, it, 2, 0)?;
            if dg(g, it, "shown").truthy() {
                ready_runes(g, ctrl(g, it), 2)?;
            }
            Ok(())
        }),
        show_off: show_off_fn("Battle Preparations.show_off", |g, _pid, _c, x| {
            Ok(g.is_gear(x))
        }),
        choices: choices_fn("Battle Preparations.choices", |g, pid, ctx| {
            let us = buff_targets(g, pid, ctx.g("hidden_bf").opt_loc())?;
            Ok(tg_choices(g, &us))
        }),
        ..imp("Battle Preparations")
    });

    // Command: Attack — "Choose one — Move a friendly gear with [Deploy] to a battlefield. Look at the top 5 cards of
    // your Main Deck. You may reveal a gear from among them and draw it. Recycle the rest."
    r.card(Impl {
        resolve: res_fn("_command_attack", |g, it| {
            let c = ctrl(g, it);
            if dg(g, it, "mode").opt_str() == Some("move") {
                let x = g.legal(it, 0)?;
                let dest = dg(g, it, "dest").loc();
                if let Some(x) = x {
                    if g.o(x).loc != Some(dest) {
                        g.move_(&[x], dest, Some(c), false)?;
                    }
                }
                return Ok(());
            }
            look_pick(g, c, 5, "gear_pick", |g, x| g.is_gear(x))
        }),
        choices: choices_fn("_ca_choices", |g, pid, _ctx| {
            let mut out = vec![crate::map! { mode = "look" }];
            for x in g.gear(Some(pid)) {
                if has_deploy(g, x)? {
                    for b in g.bf_locs() {
                        if Some(b) != g.o(x).loc {
                            out.push(crate::map! { mode = "move", tg = tg1(g, x), dest = b });
                        }
                    }
                }
            }
            Ok(out)
        }),
        preds: Value::List(vec![pred_fn("Command: Attack.pred", |g, it, o| {
            Ok(g.o(o).ctrl == ctrl(g, it) && is_deploy_gear(g, o)?)
        })]),
        ..imp("Command: Attack")
    });

    // Hunker Down — "[Reaction] Ready a friendly unit. It can't move this turn. Draw 1."
    r.card(Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![p_friend_v()]),
        resolve: res_fn("_hunker", |g, it| {
            if let Some(u) = g.legal(it, 0)? {
                g.ready_obj(u, None, None)?;
                cant_move_effect(g, u);
            }
            g.draw(ctrl(g, it), 1)
        }),
        choices: choices_fn("Hunker Down.choices", |g, pid, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let fr = friends(g, pid, false, hb)?;
            let mut us: Vec<O> = fr.iter().copied().filter(|&u| g.exhausted(u)).collect();
            us.extend(fr.iter().copied().filter(|&u| !g.exhausted(u)));
            Ok(tg_choices(g, &us))
        }),
        ..imp("Hunker Down")
    });

    // Invigorating Bloom — "When you play this, give a friendly unit +3 might this turn. Exhaust a friendly [Mighty]
    // unit, [E]: Draw 1."
    r.card(Impl {
        on_play: plus_trigger(
            "Invigorating Bloom",
            3,
            Some(opts_fn("Invigorating Bloom.opts", |g, it| {
                let c = ctrl(g, it);
                friends(g, c, false, None)
            })),
            p_friend_v(),
        ),
        abilities: vec![ab(
            "Draw",
            Cost::None,
            AbilityArgs {
                exhaust: true,
                extra_cost: extra_fn("_bloom_extra", |g, _pid, _o, ch| {
                    pay_exh(g, ch);
                    Ok(())
                }),
                choices: ab_choices_fn("Invigorating Bloom.choices", |g, pid, _o| {
                    let mut out = vec![];
                    for u in exh_cost_choices(g, pid)? {
                        if g.mighty(u)? {
                            out.push(crate::map! { exh = g.o(u).uid });
                        }
                    }
                    Ok(out)
                }),
                resolve: draw1("Invigorating Bloom.draw"),
                ..Default::default()
            },
        )],
        ..imp("Invigorating Bloom")
    });

    // Storm Raptor — "[Deflect] When a stunned enemy unit dies here, draw 1."
    r.card(Impl {
        kw: crate::map! { Deflect = 1 },
        on_event: on_event_fn("Storm Raptor.on_event", |g, o, ev, info| {
            if ev == "die" {
                let inf = info.g("info").map().clone();
                if inf.g("ctrl").int() as usize != g.o(o).ctrl
                    && inf.g("stunned").truthy()
                    && is_bf(g.o(o).loc)
                    && inf.g("loc").opt_loc() == g.o(o).loc
                    && inf_type(&inf) == "Unit"
                {
                    q0(g, o, "Storm Raptor", draw1("Storm Raptor.res"));
                }
            }
            Ok(())
        }),
        ..imp("Storm Raptor")
    });

    // Watchful Curator — "Your gear have [Deflect]."
    r.card(Impl {
        aura_kw: aura_kw_fn("Watchful Curator.aura_kw", |g, src, o| {
            let s = src.obj();
            Ok(if zone_is(g, s, Zone::Board) && ty(g, s) == "Unit" && g.o(o).ctrl == g.o(s).ctrl && g.is_gear(o) {
                Some(crate::map! { Deflect = 1 })
            } else {
                None
            })
        }),
        ..imp("Watchful Curator")
    });

    // Ahri, Confident — "[Disarm] When you reduce the Might of an enemy unit here by 1 or more, give me +1 might this
    // turn." (évènement 'might_reduced' : un -might donné par un sort ou une capacité que tu contrôles)
    r.card(Impl {
        kw: crate::map! { Disarm = 1 },
        on_event: on_event_fn("Ahri, Confident.on_event", |g, o, ev, info| {
            if ev == "might_reduced" {
                let x = info.g("obj").obj();
                if info.g("by").opt_int() == Some(g.o(o).ctrl as i64)
                    && g.o(x).ctrl != g.o(o).ctrl
                    && is_bf(g.o(o).loc)
                    && g.o(x).loc == g.o(o).loc
                    && info.g("amount").int() >= 1
                {
                    q0(g, o, "Ahri, Confident", me_mod("Ahri, Confident.res", 1));
                }
            }
            Ok(())
        }),
        ..imp("Ahri, Confident")
    });

    // Ephemeral Ward — "Your units have +2 might. When you play a unit, kill this."
    r.card(Impl {
        aura_might: Value::func("Ephemeral Ward.aura_might", |g, a| {
            let (s, o) = (a[0].obj(), a[1].obj());
            Ok(Value::Int(if zone_is(g, s, Zone::Board) && g.o(o).ctrl == g.o(s).ctrl && ty(g, o) == "Unit" && o != s {
                2
            } else {
                0
            }))
        }),
        on_event: on_event_fn("Ephemeral Ward.on_event", |g, o, ev, info| {
            if ev == "played" && info.g("pid").int() as usize == g.o(o).ctrl && ty(g, info.g("card").obj()) == "Unit" {
                q0(
                    g,
                    o,
                    "Ephemeral Ward",
                    res_fn("Ephemeral Ward.res", |g, it| {
                        if let Some(m) = me(g, it) {
                            let c = ctrl(g, it);
                            g.kill(&[m], Some(c), false)?;
                        }
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Ephemeral Ward")
    });

    // Heartening Soul — "When you play me, ready a friendly legend. Friendly legends have "[E]: Give a unit +3 might
    // this turn.""
    r.card(Impl {
        on_play: on_play_fn("Heartening Soul.on_play", |g, o, _ctx| {
            q0(
                g,
                o,
                "Heartening Soul",
                res_fn("_soul_ready", |g, it| {
                    let c = ctrl(g, it);
                    let leg = g.p[c].legend;
                    if g.exhausted(leg) {
                        g.set_exhausted(leg, false);
                        g.emit("ready", crate::map! { obj = leg, by = c })?;
                    }
                    Ok(())
                }),
            );
            Ok(())
        }),
        hooks: mk(&[(
            "grant_abilities",
            Value::func("Heartening Soul.grant_abilities", |g, a| {
                let (s, o) = (a[0].obj(), a[1].obj());
                Ok(if zone_is(g, s, Zone::Board) && zone_is(g, o, Zone::Legend) && g.o(o).owner == g.o(s).ctrl {
                    Value::List(vec![Value::Map(soul_ab())])
                } else {
                    Value::List(vec![])
                })
            }),
        )]),
        ..imp("Heartening Soul")
    });

    // Hush — "[Reaction] Counter a spell with Power cost [A][A] or more." (coût en Power imprimé, 206)
    r.card(Impl {
        timing: Some("reaction"),
        resolve: res_fn("_hush", |g, it| {
            let tgt = item_by_id(g, dg(g, it, "item").opt_int().unwrap_or(-1));
            if let Some(t) = tgt {
                if g.it(t).kind == "spell" && g.it(t).card.is_some_and(|c| p_of(g, c) >= 2) {
                    g.counter(t)?;
                }
            }
            Ok(())
        }),
        choices: choices_fn("Hush.choices", |g, pid, _ctx| {
            let mut out = vec![];
            for i in g.chain.clone().into_iter().rev() {
                if g.it(i).kind == "spell"
                    && g.it(i).card.is_some_and(|c| p_of(g, c) >= 2)
                    && g.counterable(i)?
                    && (g.it(i).ctrl != pid || full_choices(g, Some(pid)))
                {
                    out.push(crate::map! { item = g.it(i).id });
                }
            }
            Ok(out)
        }),
        ..imp("Hush")
    });

    // Orianna, Exquisite — "[Shield] I don't deal combat damage unless there's a friendly gear here."
    r.card(Impl {
        kw: crate::map! { Shield = 1 },
        hooks: mk(&[(
            "no_combat_damage",
            Value::func("Orianna, Exquisite.no_combat_damage", |g, a| {
                let o = a[0].obj();
                Ok(Value::Bool(!g.gear(None).into_iter().any(|x| g.o(x).ctrl == g.o(o).ctrl && g.o(x).loc == g.o(o).loc)))
            }),
        )]),
        ..imp("Orianna, Exquisite")
    });

    // Riven, Broken Blade — "When I become exhausted, choose one that hasn't been chosen this turn — [Buff] me. Spend my
    // buff to [Stun] an enemy unit. Spend my buff to ready me."
    r.card(Impl {
        on_event: on_event_fn("_riven_bb", |g, o, ev, info| {
            if ev != "exhausted" || !in_objs(info, "objs", o) {
                return Ok(());
            }
            let key = Value::Tuple(vec!["riven_bb".into(), g.o(o).uid.into(), g.o(o).oid.into(), g.turn_no.into()]);
            let k2 = key.clone();
            let choose = bool_fn("_riven_bb.choose", move |g, it| {
                let m = me(g, it);
                let done: Vec<Value> =
                    g.effects.iter().filter(|e| e.d.g("key") == k2).map(|e| e.d.g("mode")).collect();
                let mut modes: Vec<&str> = RIVEN_MODES.iter().copied().filter(|x| !done.contains(&Value::from(*x))).collect();
                if modes.is_empty() {
                    return Ok(false);
                }
                let has = m.is_some_and(|m| g.o(m).buff > 0);
                let order: [&str; 3] = if has { ["stun", "ready", "buff"] } else { ["buff", "stun", "ready"] };
                modes.sort_by_key(|x| order.iter().position(|y| y == x).unwrap());
                let label = |x: &str| match x {
                    "buff" => tr("me renforcer (buff)", "buff me"),
                    "stun" => tr("dépenser mon buff : étourdir un ennemi", "spend my buff: stun an enemy"),
                    _ => tr("dépenser mon buff : me préparer", "spend my buff: ready me"),
                };
                let opts: Vec<Value> = modes.iter().map(|&x| opt(label(x), x.into())).collect();
                let c = ctrl(g, it);
                let mv = g.ask(c, "riven_mode", opts, crate::map! { item = it })?.opt().value.clone();
                g.add_effect(mk(&[("key", k2.clone()), ("mode", mv.clone()), ("dur", turn())]));
                g.it_mut(it).data.set("mode", mv.clone());
                if mv.opt_str() == Some("stun") {
                    let mut cs = vec![];
                    for u in enemies(g, c, false, None, true)? {
                        if deflect_ok(g, c, u)? {
                            cs.push(u);
                        }
                    }
                    if !cs.is_empty() {
                        let u = g.ask_obj(c, "target", &cs, crate::map! { item = it })?.unwrap();
                        if pay_deflect(g, it, u)? {
                            g.add_target(it, u, p_enemy_v());
                        }
                    }
                }
                Ok(true)
            });
            let res = res_fn("_riven_bb.res", |g, it| {
                let Some(m) = me(g, it) else { return Ok(()) };
                let mode = dg(g, it, "mode");
                let c = ctrl(g, it);
                match mode.opt_str() {
                    Some("buff") => {
                        g.buff(Some(m), None)?;
                    }
                    Some("stun") => {
                        let u = g.legal(it, 0)?;
                        if u.is_some() && g.spend_buff(Some(m), Some(c))? {
                            g.stun(u, Some(c))?;
                        }
                    }
                    Some("ready") => {
                        if g.spend_buff(Some(m), Some(c))? {
                            g.ready_obj(m, None, None)?;
                        }
                    }
                    _ => {}
                }
                Ok(())
            });
            q(g, o, "Riven, Broken Blade", res, Map::new(), false, choose, Value::None);
            Ok(())
        }),
        ..imp("Riven, Broken Blade")
    });

    // Hunting Blind — "[Deploy] When you play this, you may move an enemy unit to this battlefield. Friendly units here
    // have [Shield 2]."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        on_play: on_play_fn("_blind_play", |g, o, _ctx| {
            let res = res_fn("_blind_play.res", |g, it| {
                let u = g.legal(it, 0)?;
                let m = src_obj(g, it);
                if let (Some(u), Some(m)) = (u, m) {
                    if is_bf(g.o(m).loc) && g.o(u).loc != g.o(m).loc {
                        let (dest, c) = (g.o(m).loc.unwrap(), ctrl(g, it));
                        g.move_(&[u], dest, Some(c), false)?;
                    }
                }
                Ok(())
            });
            let opts = opts_fn("_blind_play.opts", |g, it| {
                let c = ctrl(g, it);
                let mut out = vec![];
                for u in enemies(g, c, false, None, true)? {
                    if let Some(m) = src_obj(g, it) {
                        if g.o(u).loc != g.o(m).loc {
                            out.push(u);
                        }
                    }
                }
                Ok(out)
            });
            q(g, o, "Hunting Blind", res, Map::new(), true, trig_target(opts, p_enemy_v(), "target", false, true), Value::None);
            Ok(())
        }),
        aura_kw: aura_kw_fn("Hunting Blind.aura_kw", |g, src, o| {
            let s = src.obj();
            Ok(if zone_is(g, s, Zone::Board) && here_friend(g, s, o) { Some(crate::map! { Shield = 2 }) } else { None })
        }),
        ..imp("Hunting Blind")
    });

    // K'Sante, Dauntless — "[Shield] When I attack or defend, double the [Shield] of a unit you control here this
    // turn."
    r.card(Impl {
        kw: crate::map! { Shield = 1 },
        on_event: on_event_fn("_ksante_d", |g, o, ev, info| {
            if (ev == "attack" || ev == "defend") && info.g("obj").opt_obj() == Some(o) {
                let here = g.o(o).loc;
                let res = res_fn("_ksante_d.res", |g, it| {
                    if let Some(u) = g.legal(it, 0)? {
                        let n = g.kw_value(u, "Shield")?;
                        if n > 0 {
                            g.grant(u, "Shield", n, turn());
                        }
                    }
                    Ok(())
                });
                let opts = opts_fn("_ksante_d.opts", move |g, it| {
                    let c = ctrl(g, it);
                    let us: Vec<O> = g.units(Some(c), None).into_iter().filter(|&u| g.o(u).loc == here).collect();
                    sort_by(g, us, |g, u| Ok((-g.kw_value(u, "Shield")?, g.o(u).uid)))
                });
                let pred = pred_fn("_ksante_d.pred", move |g, it, u| Ok(g.o(u).ctrl == ctrl(g, it) && g.o(u).loc == here));
                q(g, o, "K'Sante, Dauntless", res, Map::new(), false, trig_target(opts, pred, "target", false, true), Value::None);
            }
            Ok(())
        }),
        ..imp("K'Sante, Dauntless")
    });

    // Resplendent Rostrum — "[E]: Draw 1 and channel 1 rune exhausted. When this becomes exhausted, give a friendly unit
    // +3 might this turn."
    r.card(Impl {
        abilities: vec![ab(
            "Draw",
            Cost::None,
            AbilityArgs {
                exhaust: true,
                resolve: res_fn("Resplendent Rostrum.draw", |g, it| {
                    let c = ctrl(g, it);
                    g.draw(c, 1)?;
                    g.channel(c, 1, true)?;
                    Ok(())
                }),
                ..Default::default()
            },
        )],
        on_event: on_event_fn("Resplendent Rostrum.on_event", |g, o, ev, info| {
            if ev == "exhausted" && in_objs(info, "objs", o) {
                let opts = opts_fn("Resplendent Rostrum.opts", |g, it| {
                    let c = ctrl(g, it);
                    friends(g, c, false, None)
                });
                let res = res_fn("Resplendent Rostrum.res", |g, it| mod_target(g, it, 3, 0).map(|_| ()));
                q(g, o, "Resplendent Rostrum", res, Map::new(), false, trig_target(opts, p_friend_v(), "target", false, true), Value::None);
            }
            Ok(())
        }),
        ..imp("Resplendent Rostrum")
    });
}

// ====================================================================== Mind
fn predict_res(name: &'static str, n: usize) -> Value {
    res_fn(name, move |g, it| g.predict(ctrl(g, it), n).map(|_| ()))
}

/// effet « Its base Might becomes N this turn »
fn base_might(g: &mut Game, u: O, v: i64) {
    let (uid, oid) = (g.o(u).uid, g.o(u).oid);
    g.add_effect(mk(&[
        ("kind", "base_might".into()),
        ("uid", uid.into()),
        ("oid", oid.into()),
        ("value", v.into()),
        ("dur", turn()),
    ]));
}

fn om_key(g: &Game, src: O) -> String {
    skey(&["orianna_mm".into(), g.o(src).uid.into(), g.o(src).oid.into(), g.turn_no.into()])
}

/// `_zpi_boom`
fn zpi_boom() -> Value {
    res_fn("_zpi_boom", |g, it| {
        let m = me(g, it);
        let c = ctrl(g, it);
        for x in g.p[c].hand.clone() {
            g.discard(c, x)?;
        }
        if let Some(m) = m {
            g.kill(&[m], Some(c), false)?;
        }
        g.gain_point(1 - c, "Zero Point Inflection")?;
        Ok(())
    })
}

fn mind(r: &mut Registry) {
    // Fanfare — "The next three times you play a unit this turn, draw 1."
    r.card(Impl {
        resolve: res_fn("_fanfare", |g, it| {
            let pid = ctrl(g, it);
            let fire = Value::func("_fanfare.fire", |g, a| {
                let eid = a[0].int() as u64;
                let info = ctx_of(&a[1]);
                let eff = g.eff(eid).cloned().unwrap_or_default();
                if info.g("pid") != eff.g("pid") || ty(g, info.g("card").obj()) != "Unit" {
                    return Ok(Value::None);
                }
                let left = eff.g("left").int() - 1;
                if let Some(e) = g.eff_mut(eid) {
                    e.set("left", left);
                }
                if left <= 0 {
                    g.remove_effect(eid);
                }
                let p = eff.g("pid").int() as usize;
                g.queue_trigger(p, "Fanfare", draw1("_fanfare.draw"), Map::new(), None, false, Value::None, Value::None);
                Ok(Value::None)
            });
            g.add_effect(mk(&[
                ("on", "played".into()),
                ("fn", fire),
                ("pid", pid.into()),
                ("left", 3.into()),
                ("dur", turn()),
            ]));
            Ok(())
        }),
        ..imp("Fanfare")
    });

    // High Note — "[Reaction] Give a unit -might this turn equal to the number of exhausted units you control."
    r.card(Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![p_unit_v()]),
        resolve: res_fn("_high_note", |g, it| {
            let u = g.legal(it, 0)?;
            let c = ctrl(g, it);
            let n = g.units(Some(c), None).into_iter().filter(|&x| g.exhausted(x)).count() as i64;
            if let Some(u) = u {
                if n != 0 {
                    g.mod_(u, -n, turn(), None)?;
                }
            }
            Ok(())
        }),
        choices: choices_fn("High Note.choices", |g, pid, ctx| {
            let us = minus_targets(g, pid, ctx.g("hidden_bf").opt_loc(), false)?;
            Ok(tg_choices(g, &us))
        }),
        ..imp("High Note")
    });

    // Industrious Mechsmith — "When you play me, play a 3 might Mech unit token here."
    r.card(Impl {
        on_play: on_play_fn("Industrious Mechsmith.on_play", |g, o, _ctx| {
            let loc: Value = g.o(o).loc.into();
            q(g, o, "Industrious Mechsmith", mech_here(false), crate::map! { loc = loc }, false, Value::None, Value::None);
            Ok(())
        }),
        ..imp("Industrious Mechsmith")
    });

    // Mysterious Chronomancer — "When you play me, [Predict 2]. [Disarm 2]"
    r.card(Impl {
        kw: crate::map! { Disarm = 2 },
        on_play: on_play_fn("Mysterious Chronomancer.on_play", |g, o, _ctx| {
            q0(g, o, "Mysterious Chronomancer", predict_res("Mysterious Chronomancer.res", 2));
            Ok(())
        }),
        ..imp("Mysterious Chronomancer")
    });

    // Parallel Convergence — "As an additional cost to play this, return a friendly unit to its owner's hand. Deal 5 to
    // a unit at a battlefield."
    r.card(Impl {
        preds: Value::List(vec![p_unit_bf_v()]),
        pay_extra: pay_ret_v(),
        resolve: res_fn("Parallel Convergence.resolve", |g, it| {
            let u = g.legal(it, 0)?;
            let c = ctrl(g, it);
            g.deal(u, 5, "spell", Some(c), None)?;
            Ok(())
        }),
        choices: ret_spell_choices(|g, pid, ctx| {
            let us = minus_targets(g, pid, ctx.g("hidden_bf").opt_loc(), true)?;
            Ok(us.into_iter().map(|u| tg1(g, u)).collect())
        }),
        ..imp("Parallel Convergence")
    });

    // Resource Extractor — "[Deploy] At the end of your turn, if you control this battlefield, play a Gold gear token
    // exhausted."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        on_event: on_event_fn("Resource Extractor.on_event", |g, o, ev, info| {
            if ev == "end_turn" && info.g("pid").int() as usize == g.o(o).ctrl {
                if let Some(Loc::Bf(b)) = g.o(o).loc {
                    if g.bfs[b].ctrl == Some(g.o(o).ctrl) {
                        q0(g, o, "Resource Extractor", res_fn("Resource Extractor.res", |g, it| gold(g, ctrl(g, it), false).map(|_| ())));
                    }
                }
            }
            Ok(())
        }),
        ..imp("Resource Extractor")
    });

    // Strigian Oracle — "When I become exhausted, [Predict]."
    r.card(Impl {
        on_event: on_event_fn("Strigian Oracle.on_event", |g, o, ev, info| {
            if ev == "exhausted" && in_objs(info, "objs", o) {
                q0(g, o, "Strigian Oracle", predict_res("Strigian Oracle.res", 1));
            }
            Ok(())
        }),
        ..imp("Strigian Oracle")
    });

    // Clanging Sentry — "Your Mechs have [Disarm]." (les valeurs s'additionnent, 831.2)
    r.card(Impl {
        aura_kw: aura_kw_fn("Clanging Sentry.aura_kw", |g, src, o| {
            let s = src.obj();
            Ok(if zone_is(g, s, Zone::Board) && g.o(o).ctrl == g.o(s).ctrl && ty(g, o) == "Unit" && g.tags(o)?.contains("Mech") {
                Some(crate::map! { Disarm = 1 })
            } else {
                None
            })
        }),
        ..imp("Clanging Sentry")
    });

    // Improvised Projectile — "[Action] Choose a friendly gear. Deal damage equal to its Energy cost to a unit at a
    // battlefield." (son coût en énergie imprimé, 206 ; un équipement illégal ne donne aucune information : rien n'est
    // infligé, 359.3.e.12)
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![
            pred_fn("Improvised Projectile.pred0", |g, it, o| Ok(g.is_gear(o) && g.o(o).ctrl == ctrl(g, it))),
            p_unit_bf_v(),
        ]),
        resolve: res_fn("_projectile", |g, it| {
            let (x, u) = (g.legal(it, 0)?, g.legal(it, 1)?);
            if let (Some(x), Some(u)) = (x, u) {
                let n = if !g.o(x).token || g.o(x).copy.as_deref().is_some_and(|c| !c.is_empty()) { e_of(g, x) } else { 0 };
                let c = ctrl(g, it);
                g.deal(Some(u), n, "spell", Some(c), None)?;
            }
            Ok(())
        }),
        choices: choices_fn("_projectile_choices", |g, pid, ctx| {
            let gs = g.gear(Some(pid));
            let gs = sort_by(g, gs, |g, x| Ok((-e_of(g, x), g.o(x).uid)))?;
            let us = minus_targets(g, pid, ctx.g("hidden_bf").opt_loc(), true)?;
            let mut out = vec![];
            for x in cap(g, gs, 2, Some(pid)) {
                for &u in &us {
                    if g.o(u).uid != g.o(x).uid {
                        out.push(crate::map! { tg = Value::Tuple(vec![g.o(x).uid.into(), g.o(u).uid.into()]) });
                    }
                }
            }
            Ok(out)
        }),
        ..imp("Improvised Projectile")
    });

    // Piltovan Guidebot — "When you play a card, you may have my base Might become equal to that card's Energy cost
    // this turn." (son coût en énergie imprimé, 206)
    r.card(Impl {
        on_event: on_event_fn("_guidebot", |g, o, ev, info| {
            if ev == "played" && info.g("pid").int() as usize == g.o(o).ctrl && played_card(info) {
                let n = e_of(g, info.g("card").obj());
                let res = res_fn("_guidebot.res", |g, it| {
                    if let Some(m) = me(g, it) {
                        let v = dg(g, it, "n").int();
                        base_might(g, m, v);
                    }
                    Ok(())
                });
                q(g, o, "Piltovan Guidebot", res, crate::map! { n = n }, true, Value::None, Value::None);
            }
            Ok(())
        }),
        ..imp("Piltovan Guidebot")
    });

    // Riptide Mage — "[Accelerate] When you play me, give an enemy unit -3 might this turn."
    r.card(Impl {
        accelerate: true,
        on_play: on_play_fn("Riptide Mage.on_play", |g, o, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let opts = opts_fn("Riptide Mage.opts", move |g, it| {
                let c = ctrl(g, it);
                enemies(g, c, false, hb, true)
            });
            let res = res_fn("Riptide Mage.res", |g, it| mod_target(g, it, -3, 0).map(|_| ()));
            q(g, o, "Riptide Mage", res, Map::new(), false, trig_target(opts, p_enemy_v(), "target", false, true), Value::None);
            Ok(())
        }),
        ..imp("Riptide Mage")
    });

    // Supernova — "Deal 10 to a unit. Draw 2."
    r.card(Impl {
        preds: Value::List(vec![p_unit_v()]),
        resolve: res_fn("Supernova.resolve", |g, it| {
            let u = g.legal(it, 0)?;
            let c = ctrl(g, it);
            g.deal(u, 10, "spell", Some(c), None)?;
            g.draw(c, 2)
        }),
        choices: choices_fn("Supernova.choices", |g, pid, ctx| {
            let us = minus_targets(g, pid, ctx.g("hidden_bf").opt_loc(), false)?;
            Ok(tg_choices(g, &us))
        }),
        ..imp("Supernova")
    });

    // The Empyrean — "[Show Off] a card. When you play me, if you showed off a card, [Predict] equal to its Power cost,
    // then draw 2." (son coût en Power imprimé tel qu'il a été montré, 832.1.d.1 ; un jeton n'est pas une carte, 052)
    r.card(Impl {
        on_play: on_play_fn("_empyrean", |g, o, ctx| {
            let sh = ctx.g("shown");
            if sh.is_none() {
                return Ok(());
            }
            let res = res_fn("_empyrean.res", |g, it| {
                let p = dg(g, it, "p").int();
                let c = ctrl(g, it);
                if p > 0 {
                    g.predict(c, p as usize)?;
                }
                g.draw(c, 2)
            });
            q(g, o, "The Empyrean", res, crate::map! { p = sh.map().g("p") }, false, Value::None, Value::None);
            Ok(())
        }),
        show_off: show_off_fn("The Empyrean.show_off", |g, _pid, _c, x| Ok(!g.o(x).token)),
        show_key: show_key_fn("The Empyrean.show_key", |g, x| Ok(-p_of(g, x))),
        ..imp("The Empyrean")
    });

    // Ekko, Ingenious — "As an additional cost to play me, return a friendly unit to its owner's hand. [Ganking]"
    r.card(Impl {
        kw: crate::map! { Ganking = 1 },
        pay_extra: pay_ret_v(),
        as_played: as_played_fn("Ekko, Ingenious.as_played", |g, pid, _c| {
            let us = g.units(Some(pid), None);
            let s = sort_by(g, us, |g, u| Ok((!g.o(u).token, -e_of(g, u), g.o(u).uid)))?;
            Ok(cap(g, s, 3, Some(pid)).into_iter().map(|u| crate::map! { ret = g.o(u).uid }).collect())
        }),
        ..imp("Ekko, Ingenious")
    });

    // Gravity Field — "[Deploy] Opponents must pay [1][A] to move one or more units to this battlefield. [Deathknell]
    // Draw 1."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        deathknell: draw1("Gravity Field.deathknell"),
        hooks: mk(&[(
            "move_cost",
            Value::func("Gravity Field.move_cost", |g, a| {
                let s = a[0].obj();
                let pid = a[1].int() as usize;
                let dest = a[3].opt_loc();
                Ok(if zone_is(g, s, Zone::Board) && is_bf(g.o(s).loc) && dest == g.o(s).loc && pid != g.o(s).ctrl {
                    Value::Tuple(vec![1.into(), reqs_to_value(&[any_req()])])
                } else {
                    Value::None
                })
            }),
        )]),
        ..imp("Gravity Field")
    });

    // Kai'Sa, Rebel — "[Disarm] While I'm in a showdown, your spells have [Reaction]."
    r.card(Impl {
        kw: crate::map! { Disarm = 1 },
        hooks: mk(&[(
            "card_kw",
            Value::func("Kai'Sa, Rebel.card_kw", |g, a| {
                let (s, pid, c) = (a[0].obj(), a[1].int() as usize, a[2].obj());
                let ok = a[3].opt_str() == Some("Reaction")
                    && zone_is(g, s, Zone::Board)
                    && pid == g.o(s).ctrl
                    && ty(g, c) == "Spell"
                    && g.sd.as_ref().is_some_and(|sd| g.o(s).loc == Some(Loc::Bf(sd.bf)));
                Ok(Value::Bool(ok))
            }),
        )]),
        ..imp("Kai'Sa, Rebel")
    });

    // Scrutinizing Scope — "[1], [E]: Choose an opponent. They guess a card type. Reveal the top card of your Main Deck.
    // If it has that type, recycle it. Otherwise, draw it."
    r.card(Impl {
        abilities: vec![ab(
            "Guess",
            Cost::from("1 energy"),
            AbilityArgs {
                exhaust: true,
                resolve: res_fn("_scope", |g, it| {
                    let c = ctrl(g, it);
                    let opp = 1 - c;
                    let opts = vec![
                        opt(tr("unité", "unit"), "Unit".into()),
                        opt(tr("sort", "spell"), "Spell".into()),
                        opt(tr("équipement", "gear"), "Gear".into()),
                    ];
                    let guess = g.ask(opp, "scope_guess", opts, Map::new())?.opt().value.clone();
                    let top = g.reveal(c, 1, None, true, None)?;
                    if top.is_empty() {
                        return Ok(());
                    }
                    let x = top[0];
                    // une unit gear a les deux types (178.3)
                    if guess.opt_str() == Some(ty(g, x)) || (guess.opt_str() == Some("Gear") && g.is_gear(x)) {
                        deck_remove(g, c, x);
                        g.recycle_cards(c, &[x])?;
                    } else {
                        g.draw_card(c, x)?;
                    }
                    Ok(())
                }),
                ..Default::default()
            },
        )],
        ..imp("Scrutinizing Scope")
    });

    // Seraphine, Inspiring — "[Disarm 2] Once each turn, when you exhaust three or more friendly units at the same time,
    // you may draw 1." (les objets épuisés par la même action : un seul évènement 'exhausted')
    r.card(Impl {
        kw: crate::map! { Disarm = 2 },
        on_event: on_event_fn("_sera_insp", |g, o, ev, info| {
            if ev == "exhausted" {
                let mine = objs_of(&info.g("objs"))
                    .into_iter()
                    .filter(|&x| g.o(x).ctrl == g.o(o).ctrl && ty(g, x) == "Unit")
                    .count();
                let key = skey(&["sera_insp".into(), g.o(o).uid.into(), g.o(o).oid.into(), g.turn_no.into()]);
                if mine >= 3 && g.stats.get(&key) == 0 {
                    stat_set(g, &key, 1);
                    q(g, o, "Seraphine, Inspiring", draw1("_sera_insp.res"), Map::new(), true, Value::None, Value::None);
                }
            }
            Ok(())
        }),
        ..imp("Seraphine, Inspiring")
    });

    // Spirit of the Forge — "You may return a friendly gear to its owner's hand as an additional cost to play me. When
    // you play me, if you paid the additional cost, ready me and give me +might this turn equal to the Energy cost of
    // the gear you returned."
    r.card(Impl {
        pay_extra: pay_ret_v(),
        on_play: on_play_fn("_forge_play", |g, o, ctx| {
            if ctx.g("ret_gear").is_none() {
                return Ok(());
            }
            let res = res_fn("_forge_play.res", |g, it| {
                if let Some(m) = me(g, it) {
                    g.ready_obj(m, None, None)?;
                    let n = dg(g, it, "n").int();
                    if n != 0 {
                        g.mod_(m, n, turn(), None)?;
                    }
                }
                Ok(())
            });
            let n = ctx.get("ret_gear_e").cloned().unwrap_or(Value::Int(0));
            q(g, o, "Spirit of the Forge", res, crate::map! { n = n }, false, Value::None, Value::None);
            Ok(())
        }),
        as_played: as_played_fn("Spirit of the Forge.as_played", |g, pid, _c| {
            let gs = g.gear(Some(pid));
            let s = sort_by(g, gs, |g, x| Ok((-e_of(g, x), g.o(x).uid)))?;
            let mut out = vec![Map::new()];
            out.extend(cap(g, s, 3, Some(pid)).into_iter().map(|x| crate::map! { ret_gear = g.o(x).uid }));
            Ok(out)
        }),
        ..imp("Spirit of the Forge")
    });

    // Clocktower Guardian — "When you play me or when I hold, [Predict 3]. [Deathknell] Banish the top card of your Main
    // Deck. You may play it, ignoring its Energy cost."
    r.card(Impl {
        deathknell: res_fn("_clock_dk", |g, it| {
            let c = ctrl(g, it);
            if g.p[c].deck.is_empty() {
                return Ok(());
            }
            let x = g.p[c].deck[0];
            g.to_zone(x, "banish", false, Some(c))?;
            let cands = free_choices(g, c, x, "banish", true, &[])?;
            play_from(g, c, x, "banish", cands, true)?;
            Ok(())
        }),
        on_play: on_play_fn("Clocktower Guardian.on_play", |g, o, _ctx| {
            q0(g, o, "Clocktower Guardian", predict_res("Clocktower Guardian.res", 3));
            Ok(())
        }),
        on_event: on_event_fn("Clocktower Guardian.on_event", |g, o, ev, info| {
            if ev == "hold" && info.g("pid").int() as usize == g.o(o).ctrl && in_objs(info, "units", o) {
                q0(g, o, "Clocktower Guardian", predict_res("Clocktower Guardian.res", 3));
            }
            Ok(())
        }),
        ..imp("Clocktower Guardian")
    });

    // Orianna, Mechanical Marvel — "Once each turn, while I'm at a battlefield, you may pay [1][A] less to play a gear."
    // (une réduction : toujours prise, consommée par le premier équipement joué avec elle)
    r.card(Impl {
        hooks: mk(&[(
            "cost_aura",
            Value::func("_om_aura", |g, a| {
                let (s, pid) = (a[0].obj(), a[1].int() as usize);
                let what = ctx_of(&a[2]);
                Ok(if zone_is(g, s, Zone::Board)
                    && is_bf(g.o(s).loc)
                    && pid == g.o(s).ctrl
                    && what.g("card").opt_obj().is_some_and(|c| g.is_gear(c))
                    && g.stats.get(&om_key(g, s)) == 0
                {
                    maps_v(vec![crate::map! { e = -1, rm = 1 }])
                } else {
                    Value::List(vec![])
                })
            }),
        )]),
        on_event: on_event_fn("_om_event", |g, o, ev, info| {
            if ev == "played"
                && info.g("pid").int() as usize == g.o(o).ctrl
                && played_card(info)
                && g.is_gear(info.g("card").obj())
                && is_bf(g.o(o).loc)
                && !g.o(info.g("card").obj()).limited_play
            {
                let k = om_key(g, o);
                stat_set(g, &k, 1);
            }
            Ok(())
        }),
        ..imp("Orianna, Mechanical Marvel")
    });

    // Zero Point Inflection — "When you play a card, draw 2. When you draw 1 or more cards with 9 or more other cards in
    // your hand, discard your hand, kill this, and choose an opponent. They score 1 point."
    r.card(Impl {
        on_event: on_event_fn("_zpi", |g, o, ev, info| {
            let oc = g.o(o).ctrl;
            if ev == "played" && info.g("pid").int() as usize == oc && played_card(info) {
                q0(g, o, "Zero Point Inflection", res_fn("_zpi.draw", |g, it| g.draw(ctrl(g, it), 2)));
            } else if ev == "draw" && info.g("pid").int() as usize == oc && g.p[oc].hand.len() as i64 - 1 >= 9 {
                let key = Value::Tuple(vec!["zpi".into(), g.o(o).uid.into(), g.o(o).oid.into()]);
                let pending = g
                    .trigq
                    .iter()
                    .chain(g.chain.iter())
                    .any(|&t| g.it(t).kind == "trigger" && g.it(t).data.g("_zpi") == key);
                if !pending {
                    q(g, o, "Zero Point Inflection (hand)", zpi_boom(), crate::map! { _zpi = key }, false, Value::None, Value::None);
                }
            }
            Ok(())
        }),
        ..imp("Zero Point Inflection")
    });
}

// ====================================================================== Body
fn armory_trig(g: &mut Game, o: O) {
    let res = res_fn("_armory_trig.res", |g, it| {
        let u = g.legal(it, 0)?;
        if u.is_some() {
            g.buff(u, None)?;
        }
        Ok(())
    });
    let opts = opts_fn("_armory_trig.opts", |g, it| {
        let c = ctrl(g, it);
        buff_order(g, c)
    });
    q(g, o, "Pillaged Armory", res, Map::new(), false, trig_target(opts, p_friend_v(), "target", false, true), Value::None);
}

/// `may_pay(e, reqs_fn)` avec des reqs fixes
fn reqs_fn(reqs: Vec<Req>) -> Value {
    Value::func("reqs_fn", move |_g, _a| Ok(reqs_to_value(&reqs)))
}

fn hold_me(g: &Game, o: O, info: &Map) -> bool {
    info.g("pid").int() as usize == g.o(o).ctrl && in_objs(info, "units", o)
}

/// info["bf"] == o.loc
fn bf_is(g: &Game, o: O, info: &Map) -> bool {
    info.g("bf").opt_int().is_some_and(|b| g.o(o).loc == Some(Loc::Bf(b as usize)))
}

fn body(r: &mut Registry) {
    // Hold 'Em — "[Action] This costs [2] less if you've played a unit or gear to a battlefield this turn. Give a unit
    // +3 might this turn."
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_v()]),
        resolve: res_fn("Hold 'Em.resolve", |g, it| mod_target(g, it, 3, 0).map(|_| ())),
        cost_mod: cost_mod_fn("Hold 'Em.cost_mod", |g, pid, _c, _ch| Ok(if g.hist.to_bf[pid] != 0 { (2, 0) } else { (0, 0) })),
        choices: choices_fn("Hold 'Em.choices", |g, pid, ctx| {
            let us = buff_targets(g, pid, ctx.g("hidden_bf").opt_loc())?;
            Ok(tg_choices(g, &us))
        }),
        ..imp("Hold 'Em")
    });

    // Nazuman Ntofo — "[Equip] [1]" ; texte d'effet et bonus de might lus sur l'image : [Shield 2], +0 might.
    let mut im = Impl { bonus: Some(0), equip_kw: Some(crate::map! { Shield = 2 }), ..imp("Nazuman Ntofo") };
    impl_equip(&mut im, Cost::from("1 energy"));
    r.card(im);

    // Overloaded Traveler — "While there's a friendly gear with [Deploy] at my battlefield, I have +1 might."
    r.card(Impl {
        might_mod: might_mod_fn("Overloaded Traveler.might_mod", |g, o| {
            if !is_bf(g.o(o).loc) {
                return Ok(0);
            }
            for x in g.gear(None) {
                if g.o(x).ctrl == g.o(o).ctrl && g.o(x).loc == g.o(o).loc && has_deploy(g, x)? {
                    return Ok(1);
                }
            }
            Ok(0)
        }),
        ..imp("Overloaded Traveler")
    });

    // Pillaged Armory — "[Deploy] When you play this or when you conquer here, [Buff] a friendly unit."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        on_play: on_play_fn("Pillaged Armory.on_play", |g, o, _ctx| {
            armory_trig(g, o);
            Ok(())
        }),
        on_event: on_event_fn("Pillaged Armory.on_event", |g, o, ev, info| {
            if ev == "conquer" && info.g("pid").int() as usize == g.o(o).ctrl && bf_is(g, o, info) {
                armory_trig(g, o);
            }
            Ok(())
        }),
        ..imp("Pillaged Armory")
    });

    // Raving Witness — "When I move, reveal the top card of your Main Deck. If it's [Mighty], draw it. Otherwise,
    // recycle it." (une carte hors du plateau est Mighty avec 5+ de Might imprimée, 706)
    r.card(Impl {
        on_event: on_event_fn("Raving Witness.on_event", |g, o, ev, info| {
            if ev == "move" && info.g("obj").opt_obj() == Some(o) {
                q0(
                    g,
                    o,
                    "Raving Witness",
                    res_fn("_witness_res", |g, it| {
                        let c = ctrl(g, it);
                        let top = g.reveal(c, 1, None, true, None)?;
                        if top.is_empty() {
                            return Ok(());
                        }
                        let x = top[0];
                        if ty(g, x) == "Unit" && g.mighty(x)? {
                            g.draw_card(c, x)?;
                        } else if g.p[c].deck.contains(&x) {
                            deck_remove(g, c, x);
                            g.recycle_cards(c, &[x])?;
                        }
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Raving Witness")
    });

    // Skystrike Squadron — "You may return a friendly gear to its owner's hand as an additional cost to play me. If you
    // do, I cost [2] less. [Deflect]"
    r.card(Impl {
        kw: crate::map! { Deflect = 1 },
        pay_extra: pay_ret_v(),
        cost_mod: cost_mod_fn("Skystrike Squadron.cost_mod", |_g, _pid, _c, ch| {
            Ok(if !ch.g("ret_gear").is_none() { (2, 0) } else { (0, 0) })
        }),
        as_played: as_played_fn("Skystrike Squadron.as_played", |g, pid, _c| {
            let gs = g.gear(Some(pid));
            let s = sort_by(g, gs, |g, x| Ok((e_of(g, x), g.o(x).uid)))?;
            let mut out = vec![Map::new()];
            out.extend(cap(g, s, 3, Some(pid)).into_iter().map(|x| crate::map! { ret_gear = g.o(x).uid }));
            Ok(out)
        }),
        ..imp("Skystrike Squadron")
    });

    // Sneaky Zeebles — "When I conquer, [Add] [A]." (un [Add] se résout aussitôt, 430.2)
    r.card(Impl {
        on_event: on_event_fn("Sneaky Zeebles.on_event", |g, o, ev, info| {
            if ev == "conquer" && hold_me(g, o, info) {
                let c = g.o(o).ctrl;
                g.add_pool(c, 0, &["A"]);
            }
            Ok(())
        }),
        ..imp("Sneaky Zeebles")
    });

    // Wrath of the Freljord — "I enter ready. [Shield 3]"
    r.card(Impl { enter_ready: Value::Bool(true), kw: crate::map! { Shield = 3 }, ..imp("Wrath of the Freljord") });

    // Armored Tusker — "[Shield 3] [Tank] When I move, you may deal damage equal to my [Shield] to a unit at a
    // battlefield." (sa valeur de Shield à la résolution)
    r.card(Impl {
        kw: crate::map! { Shield = 3, Tank = 1 },
        on_event: on_event_fn("Armored Tusker.on_event", |g, o, ev, info| {
            if ev == "move" && info.g("obj").opt_obj() == Some(o) {
                let res = res_fn("_tusker_res", |g, it| {
                    let u = g.legal(it, 0)?;
                    let n = match src_obj(g, it) {
                        Some(m) => g.kw_value(m, "Shield")?,
                        None => dg(g, it, "n").int(),
                    };
                    if u.is_some() && n > 0 {
                        let c = ctrl(g, it);
                        g.deal(u, n, "ability", Some(c), None)?;
                    }
                    Ok(())
                });
                let n = g.kw_value(o, "Shield")?;
                let opts = opts_fn("Armored Tusker.opts", |g, it| {
                    let c = ctrl(g, it);
                    minus_targets(g, c, None, true)
                });
                q(g, o, "Armored Tusker", res, crate::map! { n = n }, true, trig_target(opts, p_unit_bf_v(), "target", false, true), Value::None);
            }
            Ok(())
        }),
        ..imp("Armored Tusker")
    });

    // Avalanche — "Deal 3 to all units in bases."
    r.card(Impl {
        resolve: res_fn("_avalanche", |g, it| {
            let c = ctrl(g, it);
            let us: Vec<O> = g.all_units().into_iter().filter(|&u| g.o(u).loc == Some(Loc::Base)).collect();
            for u in us {
                g.deal(Some(u), 3, "spell", Some(c), None)?;
            }
            Ok(())
        }),
        ..imp("Avalanche")
    });

    // Bloodscent Swarm — "I cost [2] less for each battlefield you've conquered this turn."
    r.card(Impl {
        cost_mod: cost_mod_fn("Bloodscent Swarm.cost_mod", |g, pid, _c, _ch| {
            let n = g.hist.conq_bf[pid].iter().collect::<BTreeSet<_>>().len() as i64;
            Ok((2 * n, 0))
        }),
        ..imp("Bloodscent Swarm")
    });

    // Elnuk Herd — "[Hidden] When you play me, give another friendly unit +3 might this turn, then recycle me if I was
    // played from face down."
    r.card(Impl {
        hidden: true,
        on_play: on_play_fn("_elnuk_play", |g, o, ctx| {
            let fd = ctx.g("src").opt_str() == Some("facedown");
            let res = res_fn("_elnuk_res", |g, it| {
                mod_target(g, it, 3, 0)?;
                let m = me(g, it);
                if dg(g, it, "fd").truthy() {
                    if let Some(m) = m {
                        g.recycle_cards(ctrl(g, it), &[m])?;
                    }
                }
                Ok(())
            });
            let opts = opts_fn("_elnuk_play.opts", |g, it| {
                let c = ctrl(g, it);
                let src = g.it(it).src;
                Ok(friends(g, c, false, None)?.into_iter().filter(|&u| Some(g.o(u).uid) != src).collect())
            });
            let pred = pred_fn("_elnuk_play.pred", |g, it, u| Ok(g.o(u).ctrl == ctrl(g, it) && Some(g.o(u).uid) != g.it(it).src));
            q(g, o, "Elnuk Herd", res, crate::map! { fd = fd }, false, trig_target(opts, pred, "target", false, true), Value::None);
            let oc = g.o(o).ctrl;
            if fd && friends(g, oc, false, None)?.len() <= 1 {
                q0(
                    g,
                    o,
                    "Elnuk Herd (recycle)",
                    res_fn("_elnuk_play.recycle", |g, it| {
                        if let Some(m) = me(g, it) {
                            g.recycle_cards(ctrl(g, it), &[m])?;
                        }
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Elnuk Herd")
    });

    // Primordial Roar — "[Show Off] a unit. If you showed off a unit, deal damage equal to that unit's Might to a unit."
    // (la Might telle qu'elle a été montrée, 832.1.d.1)
    r.card(Impl {
        preds: Value::List(vec![p_unit_v()]),
        resolve: res_fn("_roar", |g, it| {
            let sh = dg(g, it, "shown");
            let u = g.legal(it, 0)?;
            if !sh.is_none() && u.is_some() && sh.map().g("might").int() > 0 {
                let (n, c) = (sh.map().g("might").int(), ctrl(g, it));
                g.deal(u, n, "spell", Some(c), None)?;
            }
            Ok(())
        }),
        show_off: show_off_fn("Primordial Roar.show_off", |g, _pid, _c, x| Ok(ty(g, x) == "Unit")),
        show_key: show_key_fn("Primordial Roar.show_key", |g, x| {
            Ok(-(if g.board.contains(&x) { g.might(x)? } else { might_of(g, x) }))
        }),
        choices: choices_fn("Primordial Roar.choices", |g, pid, ctx| {
            let us = minus_targets(g, pid, ctx.g("hidden_bf").opt_loc(), false)?;
            Ok(tg_choices(g, &us))
        }),
        ..imp("Primordial Roar")
    });

    // Stalwart Goatherd — "[Shield 2] When you play me, you may kill a gear with Energy cost [4] or less."
    r.card(Impl {
        kw: crate::map! { Shield = 2 },
        on_play: on_play_fn("Stalwart Goatherd.on_play", |g, o, _ctx| {
            let res = res_fn("Stalwart Goatherd.res", |g, it| {
                if let Some(x) = g.legal(it, 0)? {
                    let c = ctrl(g, it);
                    g.kill(&[x], Some(c), false)?;
                }
                Ok(())
            });
            let opts = opts_fn("Stalwart Goatherd.opts", |g, it| {
                let c = ctrl(g, it);
                let xs: Vec<O> = g.gear(None).into_iter().filter(|&x| e_of(g, x) <= 4 && g.o(x).ctrl != c).collect();
                sort_by(g, xs, |g, x| Ok((-e_of(g, x), g.o(x).uid)))
            });
            let pred = pred_fn("Stalwart Goatherd.pred", |g, _it, x| Ok(g.is_gear(x) && e_of(g, x) <= 4));
            q(g, o, "Stalwart Goatherd", res, Map::new(), true, trig_target(opts, pred, "target", false, true), Value::None);
            Ok(())
        }),
        ..imp("Stalwart Goatherd")
    });

    // Cataclysm — "Opponents can't play spells this turn. Move a friendly unit to an occupied enemy battlefield."
    r.card(Impl {
        preds: Value::List(vec![p_friend_v()]),
        resolve: res_fn("_cataclysm", |g, it| {
            let c = ctrl(g, it);
            let f = Value::func("_cataclysm.no_play", |g, a| {
                let eff = g.eff(a[0].int() as u64).cloned().unwrap_or_default();
                let (pid, card) = (a[1].int(), a[2].obj());
                Ok(Value::Bool(Value::Int(pid) != eff.g("pid") && ty(g, card) == "Spell"))
            });
            g.add_effect(mk(&[("kind", "no_play".into()), ("pid", c.into()), ("dur", turn()), ("fn", f)]));
            let u = g.legal(it, 0)?;
            let d = dg(g, it, "dest").opt_loc();
            if let (Some(u), Some(Loc::Bf(b))) = (u, d) {
                if g.o(u).loc != d && g.bfs[b].ctrl == Some(1 - c) && !g.units(None, Some(Loc::Bf(b))).is_empty() {
                    g.move_(&[u], Loc::Bf(b), Some(c), false)?;
                }
            }
            Ok(())
        }),
        choices: choices_fn("_cataclysm_choices", |g, pid, ctx| {
            let ds = occupied_enemy_bfs(g, pid);
            let mut out = vec![];
            for u in friends(g, pid, false, ctx.g("hidden_bf").opt_loc())? {
                for &d in &ds {
                    if g.o(u).loc != Some(Loc::Bf(d)) {
                        out.push(crate::map! { tg = tg1(g, u), dest = Loc::Bf(d) });
                    }
                }
            }
            Ok(out)
        }),
        ..imp("Cataclysm")
    });

    // Graves, Blasting Through — "When I attack or defend, deal damage equal to my Might to all enemy units here. I
    // don't deal combat damage."
    r.card(Impl {
        hooks: mk(&[("no_combat_damage", Value::func("Graves.no_combat_damage", |_g, _a| Ok(Value::Bool(true))))]),
        on_event: on_event_fn("Graves, Blasting Through.on_event", |g, o, ev, info| {
            if (ev == "attack" || ev == "defend") && info.g("obj").opt_obj() == Some(o) {
                q0(
                    g,
                    o,
                    "Graves, Blasting Through",
                    res_fn("_graves_res", |g, it| {
                        let Some(m) = me(g, it) else { return Ok(()) };
                        let n = g.might(m)?;
                        let c = ctrl(g, it);
                        let us: Vec<O> = g.units(Some(1 - c), None).into_iter().filter(|&u| g.o(u).loc == g.o(m).loc).collect();
                        for u in us {
                            g.deal(Some(u), n, "ability", Some(c), None)?;
                        }
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Graves, Blasting Through")
    });

    // K'Sante, Courageous — "[Accelerate] [Shield 2] When I hold, channel 1 rune."
    r.card(Impl {
        accelerate: true,
        kw: crate::map! { Shield = 2 },
        on_event: on_event_fn("K'Sante, Courageous.on_event", |g, o, ev, info| {
            if ev == "hold" && hold_me(g, o, info) {
                q0(g, o, "K'Sante, Courageous", res_fn("K'Sante, Courageous.res", |g, it| g.channel(ctrl(g, it), 1, false).map(|_| ())));
            }
            Ok(())
        }),
        ..imp("K'Sante, Courageous")
    });

    // Portable Portal — "[Deploy] When you play this, you may pay [Body] to ready a unit here. Friendly units here have
    // [Ganking]."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        on_play: on_play_fn("_portal_play", |g, o, _ctx| {
            let here = g.o(o).loc;
            let res = res_fn("_portal_play.res", |g, it| {
                if let Some(u) = g.legal(it, 0)? {
                    g.ready_obj(u, None, None)?;
                }
                Ok(())
            });
            let opts = opts_fn("_portal_play.opts", move |g, it| {
                let c = ctrl(g, it);
                let mut out: Vec<O> =
                    friends(g, c, false, None)?.into_iter().filter(|&u| g.o(u).loc == here && g.exhausted(u)).collect();
                out.extend(all_units(g, c, false, None)?.into_iter().filter(|&u| g.o(u).loc == here && g.o(u).ctrl != c));
                Ok(out)
            });
            let pred = pred_fn("_portal_play.pred", move |g, _it, u| Ok(g.o(u).loc == here));
            q(
                g,
                o,
                "Portable Portal",
                res,
                Map::new(),
                true,
                trig_target(opts, pred, "target", false, true),
                may_pay(0, reqs_fn(vec![req(&["Body"])])),
            );
            Ok(())
        }),
        aura_kw: aura_kw_fn("Portable Portal.aura_kw", |g, src, o| {
            let s = src.obj();
            Ok(if zone_is(g, s, Zone::Board) && here_friend(g, s, o) { Some(crate::map! { Ganking = 1 }) } else { None })
        }),
        ..imp("Portable Portal")
    });

    // Staunch Survivor : seulement des mots-clés ([Assault 2], [Shield 2]) : voir la fin du module.

    // Titanic Shrine — "When you play a unit with Might greater than each other unit on the board, exhaust me to draw
    // 1."
    r.card(Impl {
        on_event: on_event_fn("_shrine", |g, o, ev, info| {
            if ev == "played" && info.g("pid").int() as usize == g.o(o).ctrl {
                let u = info.g("card").obj();
                if ty(g, u) == "Unit" && g.board.contains(&u) {
                    let mu = g.might(u)?;
                    let mut all = true;
                    for x in g.all_units() {
                        if x != u && mu <= g.might(x)? {
                            all = false;
                            break;
                        }
                    }
                    if all {
                        let cost = bool_fn("_shrine.cost", |g, it| {
                            let Some(m) = me(g, it) else { return Ok(false) };
                            if g.exhausted(m) {
                                return Ok(false);
                            }
                            g.set_exhausted(m, true);
                            Ok(true)
                        });
                        q(g, o, "Titanic Shrine", draw1("_shrine.res"), Map::new(), false, Value::None, cost);
                    }
                }
            }
            Ok(())
        }),
        ..imp("Titanic Shrine")
    });

    // Evelynn, Consuming — "[Ambush] When you play a card from face down, ready me and give me +1 might this turn."
    r.card(Impl {
        ambush: true,
        on_event: on_event_fn("_eve_consuming", |g, o, ev, info| {
            if ev == "played" && info.g("pid").int() as usize == g.o(o).ctrl && played_card(info) && from_facedown(g, info) {
                q0(
                    g,
                    o,
                    "Evelynn, Consuming",
                    res_fn("_eve_consuming.res", |g, it| {
                        if let Some(m) = me(g, it) {
                            g.ready_obj(m, None, None)?;
                            g.mod_(m, 1, turn(), None)?;
                        }
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Evelynn, Consuming")
    });

    // Jarvan IV, Martial Paragon — "Your gear with [Deploy] have [Action]. When I conquer, if there is a friendly gear
    // with [Deploy] here, channel 1 rune exhausted."
    r.card(Impl {
        hooks: mk(&[(
            "card_kw",
            Value::func("Jarvan IV, Martial Paragon.card_kw", |g, a| {
                let (s, pid, c) = (a[0].obj(), a[1].int() as usize, a[2].obj());
                Ok(Value::Bool(
                    a[3].opt_str() == Some("Action")
                        && zone_is(g, s, Zone::Board)
                        && pid == g.o(s).ctrl
                        && g.is_gear(c)
                        && has_deploy(g, c)?,
                ))
            }),
        )]),
        on_event: on_event_fn("Jarvan IV, Martial Paragon.on_event", |g, o, ev, info| {
            if ev == "conquer" && hold_me(g, o, info) {
                let mut any = false;
                for x in g.gear(None) {
                    if g.o(x).ctrl == g.o(o).ctrl && g.o(x).loc == g.o(o).loc && has_deploy(g, x)? {
                        any = true;
                        break;
                    }
                }
                if any {
                    q0(
                        g,
                        o,
                        "Jarvan IV, Martial Paragon",
                        res_fn("Jarvan IV, Martial Paragon.res", |g, it| g.channel(ctrl(g, it), 1, true).map(|_| ())),
                    );
                }
            }
            Ok(())
        }),
        ..imp("Jarvan IV, Martial Paragon")
    });

    // Seraphic Wyvern — "If you would channel runes, channel that many runes plus 1 exhausted rune instead."
    r.card(Impl {
        hooks: mk(&[(
            "channel_rep",
            Value::func("Seraphic Wyvern.channel_rep", |g, a| {
                let (s, pid) = (a[0].obj(), a[1].int() as usize);
                Ok(Value::Int(if zone_is(g, s, Zone::Board) && pid == g.o(s).ctrl { 1 } else { 0 }))
            }),
        )]),
        ..imp("Seraphic Wyvern")
    });
}

// ====================================================================== Chaos
fn chaos(r: &mut Registry) {
    // Disposal Expert — "I enter ready if you control no other units. [Disarm 2]"
    r.card(Impl {
        kw: crate::map! { Disarm = 2 },
        enter_ready: card_ch_bool("Disposal Expert.enter_ready", |g, pid, c, _ch| {
            Ok(!g.units(Some(pid), None).into_iter().any(|u| u != c))
        }),
        ..imp("Disposal Expert")
    });

    // Explosimian — "When I hold, play three Bomb gear tokens to a single battlefield."
    r.card(Impl {
        on_event: on_event_fn("Explosimian.on_event", |g, o, ev, info| {
            if ev == "hold" && hold_me(g, o, info) {
                q0(
                    g,
                    o,
                    "Explosimian",
                    res_fn("_three_bombs", |g, it| {
                        let c = ctrl(g, it);
                        let loc = bf_ask(g, c, "bomb_location", None)?;
                        for _ in 0..3 {
                            bomb(g, c, loc)?;
                        }
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Explosimian")
    });

    // Momentary Companion — "[Hidden] When you play me, if it's not your turn, give me [Temporary]."
    r.card(Impl {
        hidden: true,
        on_play: on_play_fn("Momentary Companion.on_play", |g, o, _ctx| {
            if g.tp != g.o(o).ctrl {
                q0(
                    g,
                    o,
                    "Momentary Companion",
                    res_fn("Momentary Companion.res", |g, it| {
                        if let Some(m) = me(g, it) {
                            g.grant(m, "Temporary", 1, Value::None);
                        }
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Momentary Companion")
    });

    // Satchel Charge — "[Action] Move a friendly unit, then play a Bomb gear token to a battlefield."
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_friend_v()]),
        resolve: res_fn("_satchel", |g, it| {
            let u = g.legal(it, 0)?;
            let d = dg(g, it, "dest").opt_loc();
            let c = ctrl(g, it);
            if let (Some(u), Some(d)) = (u, d) {
                if g.o(u).loc != Some(d) {
                    g.move_(&[u], d, Some(c), false)?;
                }
            }
            bomb(g, c, None)?;
            Ok(())
        }),
        choices: choices_fn("Satchel Charge.choices", |g, pid, ctx| {
            let us = friends(g, pid, false, ctx.g("hidden_bf").opt_loc())?;
            Ok(move_choices(g, &us))
        }),
        max_choices: 12,
        ..imp("Satchel Charge")
    });

    // Siren of the Shallows : seulement des mots-clés ([Disarm] [Ganking]) : cards.auto_keywords.

    // Siren's Song — "[Reaction] Move an enemy unit that moved this turn."
    r.card(Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![pred_fn("Siren's Song.pred", |g, it, o| Ok(p_enemy(g, it, o) && moved(g, o)))]),
        resolve: res_fn("Siren's Song.resolve", |g, it| {
            if let Some(u) = g.legal(it, 0)? {
                let d = dg(g, it, "dest").loc();
                if g.o(u).loc != Some(d) {
                    let c = ctrl(g, it);
                    g.move_(&[u], d, Some(c), false)?;
                }
            }
            Ok(())
        }),
        choices: choices_fn("Siren's Song.choices", |g, pid, ctx| {
            let us: Vec<O> =
                enemies(g, pid, false, ctx.g("hidden_bf").opt_loc(), true)?.into_iter().filter(|&u| moved(g, u)).collect();
            let mut ch = move_choices(g, &us);
            ch.sort_by_key(|c| c.g("dest").opt_loc() != Some(Loc::Base));
            Ok(ch)
        }),
        max_choices: 10,
        ..imp("Siren's Song")
    });

    // Suave Negotiator — "When you play me, choose an opponent. You and that player each draw 1."
    r.card(Impl {
        on_play: on_play_fn("Suave Negotiator.on_play", |g, o, _ctx| {
            q0(
                g,
                o,
                "Suave Negotiator",
                res_fn("Suave Negotiator.res", |g, it| {
                    let c = ctrl(g, it);
                    g.draw(c, 1)?;
                    g.draw(1 - c, 1)
                }),
            );
            Ok(())
        }),
        ..imp("Suave Negotiator")
    });

    // The Waters' Wage — "[Hidden] Play a Gold gear token exhausted. If it's your turn and you played this from face
    // down, play three ready Gold gear tokens instead."
    r.card(Impl {
        hidden: true,
        resolve: res_fn("_wage", |g, it| {
            let c = ctrl(g, it);
            if g.tp == c && dg(g, it, "from").opt_str() == Some("facedown") {
                for _ in 0..3 {
                    gold(g, c, true)?;
                }
            } else {
                gold(g, c, false)?;
            }
            Ok(())
        }),
        ..imp("The Waters' Wage")
    });

    // An Offer You Can't Refuse — "[Action] Kill a gear. Its controller plays a Gold gear token exhausted."
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_gear_v()]),
        resolve: res_fn("_offer", |g, it| {
            let Some(x) = g.legal(it, 0)? else { return Ok(()) };
            let c = g.o(x).ctrl;
            let k = ctrl(g, it);
            g.kill(&[x], Some(k), false)?;
            gold(g, c, false)?;
            Ok(())
        }),
        choices: choices_fn("An Offer You Can't Refuse.choices", |g, pid, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let mut xs = vec![];
            for x in g.gear(None) {
                if g.targetable(x, Some(pid))? && (hb.is_none() || g.o(x).loc == hb) {
                    xs.push(x);
                }
            }
            let s = sort_by(g, xs, |g, x| Ok((g.o(x).ctrl == pid, -e_of(g, x), g.o(x).uid)))?;
            Ok(tg_choices(g, &s))
        }),
        ..imp("An Offer You Can't Refuse")
    });

    // Demon Shade — "[Hidden] Choose a friendly unit, then choose one — Move it to an open battlefield. Give it
    // [Ganking] this turn if there are no open battlefields."
    r.card(Impl {
        hidden: true,
        preds: Value::List(vec![p_friend_v()]),
        resolve: res_fn("_shade", |g, it| {
            let Some(u) = g.legal(it, 0)? else { return Ok(()) };
            if dg(g, it, "mode").opt_str() == Some("move") {
                let d = dg(g, it, "dest").loc();
                if let Loc::Bf(b) = d {
                    if open_bf(g, b) && g.o(u).loc != Some(d) {
                        let c = ctrl(g, it);
                        g.move_(&[u], d, Some(c), false)?;
                    }
                }
            } else if !(0..g.bfs.len()).any(|b| open_bf(g, b)) {
                g.grant(u, "Ganking", 1, turn());
            }
            Ok(())
        }),
        choices: choices_fn("_shade_choices", |g, pid, ctx| {
            let mut out = vec![];
            for u in friends(g, pid, false, ctx.g("hidden_bf").opt_loc())? {
                for b in 0..g.bfs.len() {
                    if open_bf(g, b) && g.o(u).loc != Some(Loc::Bf(b)) {
                        out.push(crate::map! { tg = tg1(g, u), mode = "move", dest = Loc::Bf(b) });
                    }
                }
                out.push(crate::map! { tg = tg1(g, u), mode = "ganking" });
            }
            Ok(out)
        }),
        max_choices: 10,
        ..imp("Demon Shade")
    });

    // Gutter Guardian — "[Hidden] [Show Off] a card with [Hidden]. When you play me, if you showed off, [Add] [A]."
    r.card(Impl {
        hidden: true,
        show_off: show_off_fn("Gutter Guardian.show_off", |g, _pid, _c, x| has_hidden(g, x)),
        on_play: on_play_fn("Gutter Guardian.on_play", |g, o, ctx| {
            if !ctx.g("show").is_none() {
                let c = g.o(o).ctrl;
                g.add_pool(c, 0, &["A"]);
            }
            Ok(())
        }),
        ..imp("Gutter Guardian")
    });

    // Pitiless Stalker — "[E]: [Stun] an enemy unit."
    r.card(Impl {
        abilities: vec![ab(
            "Stun",
            Cost::None,
            AbilityArgs {
                exhaust: true,
                preds: Value::List(vec![p_enemy_v()]),
                resolve: res_fn("Pitiless Stalker.resolve", |g, it| {
                    let u = g.legal(it, 0)?;
                    if u.is_some() {
                        let c = ctrl(g, it);
                        g.stun(u, Some(c))?;
                    }
                    Ok(())
                }),
                choices: ab_choices_fn("Pitiless Stalker.choices", |g, pid, _o| {
                    let mut us = vec![];
                    for u in enemies(g, pid, false, None, true)? {
                        if !g.o(u).stunned || full_choices(g, Some(pid)) {
                            us.push(u);
                        }
                    }
                    Ok(tg_choices(g, &us))
                }),
                ..Default::default()
            },
        )],
        ..imp("Pitiless Stalker")
    });

    // Reckless Innovator — "[Accelerate] When I become exhausted, choose one — Play a Gold gear token exhausted. Pay
    // [Chaos] to draw 1."
    r.card(Impl {
        accelerate: true,
        on_event: on_event_fn("_innovator", |g, o, ev, info| {
            if ev != "exhausted" || !in_objs(info, "objs", o) {
                return Ok(());
            }
            let choose = bool_fn("_innovator.choose", |g, it| {
                let c = ctrl(g, it);
                let mut opts = vec![opt(tr("jouer un jeton Gold épuisé", "play an exhausted Gold token"), "gold".into())];
                if g.can_pay(c, 0, &[req(&["Chaos"])], &Value::None)? {
                    opts.insert(0, opt(tr("payer [Chaos] pour piocher 1", "pay [Chaos] to draw 1"), "draw".into()));
                }
                let m = g.ask(c, "innovator_mode", opts, crate::map! { item = it })?.opt().value.clone();
                g.it_mut(it).data.set("mode", m);
                Ok(true)
            });
            let res = res_fn("_innovator.res", |g, it| {
                let c = ctrl(g, it);
                if dg(g, it, "mode").opt_str() == Some("gold") {
                    gold(g, c, false)?;
                } else if g.pay(c, 0, &[req(&["Chaos"])], &Value::None)? {
                    g.draw(c, 1)?;
                }
                Ok(())
            });
            q(g, o, "Reckless Innovator", res, Map::new(), false, choose, Value::None);
            Ok(())
        }),
        ..imp("Reckless Innovator")
    });

    // The Good Stuff — "[Deploy] When you play this, discard 1, then draw 3. When you hold here, return this to its
    // owner's hand."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        on_play: on_play_fn("The Good Stuff.on_play", |g, o, _ctx| {
            q0(
                g,
                o,
                "The Good Stuff",
                res_fn("The Good Stuff.res", |g, it| {
                    let c = ctrl(g, it);
                    discard_n(g, c, 1)?;
                    g.draw(c, 3)
                }),
            );
            Ok(())
        }),
        on_event: on_event_fn("The Good Stuff.on_event", |g, o, ev, info| {
            if ev == "hold" && info.g("pid").int() as usize == g.o(o).ctrl && bf_is(g, o, info) {
                q0(g, o, "The Good Stuff (hold)", to_hand_me("The Good Stuff.hold"));
            }
            Ok(())
        }),
        ..imp("The Good Stuff")
    });

    // Bedeviling Stranger — "[Hidden] When you play me, choose a battlefield. If there are 4 or more units there, you
    // may return an enemy unit there to its owner's hand."
    r.card(Impl {
        hidden: true,
        on_play: on_play_fn("_stranger", |g, o, _ctx| {
            let choose = bool_fn("_stranger.choose", |g, it| {
                let mut bfs: Vec<usize> = vec![0, 1];
                bfs.sort_by_key(|&b| -(g.units(None, Some(Loc::Bf(b))).len() as i64));
                let c = ctrl(g, it);
                let b = g.ask(c, "stranger_bf", bfs.into_iter().map(|b| Value::Loc(Loc::Bf(b))).collect(), crate::map! { item = it })?;
                g.it_mut(it).data.set("bf", b);
                Ok(true)
            });
            let res = res_fn("_stranger.res", |g, it| {
                let b = dg(g, it, "bf").opt_loc();
                if g.units(None, b).len() < 4 {
                    return Ok(());
                }
                let c = ctrl(g, it);
                let mut cs = vec![];
                for u in enemies(g, c, false, None, true)? {
                    if g.o(u).loc == b && deflect_ok(g, c, u)? {
                        cs.push(u);
                    }
                }
                if cs.is_empty() {
                    return Ok(());
                }
                let mut opts: Vec<Value> = cs.iter().map(|&u| Value::Obj(u)).collect();
                opts.push(Value::None);
                let u = g.ask(c, "stranger_return", opts, crate::map! { item = it })?.opt_obj();
                if let Some(u) = u {
                    if pay_deflect(g, it, u)? {
                        g.to_zone(u, "hand", false, None)?;
                    }
                }
                Ok(())
            });
            q(g, o, "Bedeviling Stranger", res, Map::new(), false, choose, Value::None);
            Ok(())
        }),
        ..imp("Bedeviling Stranger")
    });

    // Bushwhack Trap — "[Deploy] While you or an ally control this battlefield, that player may hide an additional card
    // here." (1v1 : pas d'allié)
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        hooks: mk(&[(
            "extra_hide_slots",
            Value::func("Bushwhack Trap.extra_hide_slots", |g, a| {
                let (s, b) = (a[0].obj(), a[1].int() as usize);
                Ok(Value::Int(
                    if zone_is(g, s, Zone::Board) && g.o(s).loc == Some(Loc::Bf(b)) && g.bfs[b].ctrl == Some(g.o(s).ctrl) {
                        1
                    } else {
                        0
                    },
                ))
            }),
        )]),
        ..imp("Bushwhack Trap")
    });

    // Evelynn, In Control — "[Disarm] When I hold, if you control a facedown card here, score 1 point."
    // « here » se lit à la résolution : rentrée en base entre-temps, pas de point (la base n'a pas de carte face cachée).
    r.card(Impl {
        kw: crate::map! { Disarm = 1 },
        on_event: on_event_fn("Evelynn, In Control.on_event", |g, o, ev, info| {
            if ev == "hold" && hold_me(g, o, info) {
                q0(
                    g,
                    o,
                    "Evelynn, In Control",
                    res_fn("Evelynn, In Control.res", |g, it| {
                        let Some(m) = me(g, it) else { return Ok(()) };
                        let c = ctrl(g, it);
                        if let Some(Loc::Bf(b)) = g.o(m).loc {
                            if g.bfs[b].facedowns.iter().any(|&x| g.o(x).owner == c) {
                                g.gain_point(c, "Evelynn, In Control")?;
                            }
                        }
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Evelynn, In Control")
    });

    // Lulu, Whimsical — "When I move, you may have another unit's base Might become 4 this turn."
    r.card(Impl {
        on_event: on_event_fn("Lulu, Whimsical.on_event", |g, o, ev, info| {
            if ev == "move" && info.g("obj").opt_obj() == Some(o) {
                let res = res_fn("_lulu", |g, it| {
                    if let Some(u) = g.legal(it, 0)? {
                        base_might(g, u, 4);
                    }
                    Ok(())
                });
                let opts = opts_fn("Lulu, Whimsical.opts", |g, it| {
                    let c = ctrl(g, it);
                    let src = g.it(it).src;
                    let mut us = vec![];
                    for u in g.all_units() {
                        if Some(g.o(u).uid) != src && g.targetable(u, Some(c))? {
                            us.push(u);
                        }
                    }
                    sort_by(g, us, |g, u| {
                        let m = might_of(g, u);
                        Ok((if g.o(u).ctrl == c { m - 4 } else { 4 - m }, g.o(u).uid))
                    })
                });
                let pred = pred_fn("Lulu, Whimsical.pred", |g, it, u| Ok(Some(g.o(u).uid) != g.it(it).src));
                q(g, o, "Lulu, Whimsical", res, Map::new(), true, trig_target(opts, pred, "target", false, true), Value::None);
            }
            Ok(())
        }),
        ..imp("Lulu, Whimsical")
    });

    // Mechanical Bandit — "At the start of your Beginning Phase, each player reveals the top card of their Main Deck. If
    // you revealed a card with the lowest Energy cost, draw it. Recycle the rest." (égalité au plus bas : la tienne est
    // une carte au coût le plus bas)
    r.card(Impl {
        on_event: on_event_fn("Mechanical Bandit.on_event", |g, o, ev, info| {
            if ev == "beginning_start" && info.g("pid").int() as usize == g.o(o).ctrl {
                q0(
                    g,
                    o,
                    "Mechanical Bandit",
                    res_fn("_bandit", |g, it| {
                        let c = ctrl(g, it);
                        let mut shown: Vec<(usize, O)> = vec![];
                        for pid in [c, 1 - c] {
                            let top = g.reveal(pid, 1, None, true, None)?;
                            if !top.is_empty() {
                                shown.push((pid, top[0]));
                            }
                        }
                        if let Some(&(_, mine)) = shown.iter().find(|(p, _)| *p == c) {
                            if shown.iter().all(|&(_, x)| e_of(g, mine) <= e_of(g, x)) {
                                g.draw_card(c, mine)?;
                                shown.retain(|(p, _)| *p != c);
                            }
                        }
                        for (pid, x) in shown {
                            if g.p[pid].deck.contains(&x) {
                                deck_remove(g, pid, x);
                                g.recycle_cards(pid, &[x])?;
                            }
                        }
                        Ok(())
                    }),
                );
            }
            Ok(())
        }),
        ..imp("Mechanical Bandit")
    });

    // Ziggs, Explosive — "[1], [E]: Play a Bomb gear token to a battlefield. When I defend, you may kill a friendly gear
    // to give me +2 might this turn."
    r.card(Impl {
        on_event: on_event_fn("_ziggs_def", |g, o, ev, info| {
            if ev == "defend" && info.g("obj").opt_obj() == Some(o) && !g.gear(Some(g.o(o).ctrl)).is_empty() {
                let cost = bool_fn("_ziggs_def.cost", |g, it| {
                    let c = ctrl(g, it);
                    let gs = g.gear(Some(c));
                    let xs = sort_by(g, gs, |g, x| Ok((value(g, x)?, g.o(x).uid)))?;
                    if xs.is_empty() {
                        return Ok(false);
                    }
                    let x = g.ask_obj(c, "sacrifice_gear", &xs, Map::new())?.unwrap();
                    Ok(!g.kill(&[x], Some(c), true)?.is_empty())
                });
                q(g, o, "Ziggs, Explosive", me_mod("_ziggs_def.res", 2), Map::new(), true, Value::None, cost);
            }
            Ok(())
        }),
        abilities: vec![ab(
            "Bomb",
            Cost::from("1 energy"),
            AbilityArgs {
                exhaust: true,
                resolve: res_fn("Ziggs, Explosive.bomb", |g, it| bomb(g, ctrl(g, it), None).map(|_| ())),
                ..Default::default()
            },
        )],
        ..imp("Ziggs, Explosive")
    });

    // Angler Wyrm — "[Deflect] [E]: Choose an opponent. They must move a unit they control to my battlefield."
    r.card(Impl {
        kw: crate::map! { Deflect = 1 },
        abilities: vec![ab(
            "Lure",
            Cost::None,
            AbilityArgs {
                exhaust: true,
                resolve: res_fn("_wyrm", |g, it| {
                    let Some(m) = src_obj(g, it) else { return Ok(()) };
                    let Some(dest) = g.o(m).loc.filter(|l| l.is_bf()) else { return Ok(()) };
                    let opp = 1 - ctrl(g, it);
                    let mut us = vec![];
                    for u in g.units(Some(opp), None) {
                        if g.o(u).loc != Some(dest) && g.can_move(u, dest, Some(opp), false)? {
                            us.push(u);
                        }
                    }
                    if !us.is_empty() {
                        let s = sort_by(g, us, |g, u| Ok((value(g, u)?, g.o(u).uid)))?;
                        let u = g.ask_obj(opp, "wyrm_move", &s, Map::new())?.unwrap();
                        g.move_(&[u], dest, Some(opp), false)?;
                    }
                    Ok(())
                }),
                can: can_fn("Angler Wyrm.can", |g, _pid, o| Ok(is_bf(g.o(o).loc))),
                ..Default::default()
            },
        )],
        ..imp("Angler Wyrm")
    });

    // Riven, Deserter — "When another friendly unit becomes exhausted, give me +1 might this turn. Exhaust a friendly
    // unit: Ready a unit. Use this ability only once each turn."
    r.card(Impl {
        on_event: on_event_fn("_deserter", |g, o, ev, info| {
            if ev == "exhausted" {
                for x in objs_of(&info.g("objs")) {
                    if x != o && g.o(x).ctrl == g.o(o).ctrl && ty(g, x) == "Unit" {
                        q0(g, o, "Riven, Deserter", me_mod("_deserter.res", 1));
                    }
                }
            }
            Ok(())
        }),
        abilities: vec![ab(
            "Ready",
            Cost::None,
            AbilityArgs {
                preds: Value::List(vec![p_unit_v()]),
                choices: ab_choices_fn("_deserter_choices", |g, pid, _o| {
                    let ex: Vec<O> = g.units(Some(pid), None).into_iter().filter(|&u| g.exhausted(u)).collect();
                    let mut out = vec![];
                    let xs = exh_cost_choices(g, pid)?;
                    for x in cap(g, xs, 2, Some(pid)) {
                        let mut ts = ex.clone();
                        if full_choices(g, Some(pid)) {
                            ts.extend(g.all_units().into_iter().filter(|&u| !g.exhausted(u)));
                        }
                        for t in ts {
                            if t != x {
                                out.push(crate::map! { exh = g.o(x).uid, tg = tg1(g, t) });
                            }
                        }
                    }
                    Ok(out)
                }),
                extra_cost: extra_fn("_deserter_extra", |g, _pid, o, ch| {
                    let k = skey(&["deserter".into(), g.o(o).uid.into(), g.o(o).oid.into(), g.turn_no.into()]);
                    stat_set(g, &k, 1);
                    pay_exh(g, ch);
                    Ok(())
                }),
                can: can_fn("Riven, Deserter.can", |g, _pid, o| {
                    let k = skey(&["deserter".into(), g.o(o).uid.into(), g.o(o).oid.into(), g.turn_no.into()]);
                    Ok(g.stats.get(&k) == 0)
                }),
                resolve: res_fn("Riven, Deserter.resolve", |g, it| {
                    if let Some(u) = g.legal(it, 0)? {
                        g.ready_obj(u, None, None)?;
                    }
                    Ok(())
                }),
                ..Default::default()
            },
        )],
        ..imp("Riven, Deserter")
    });

    // Transpositionist — "[Chaos], [E]: Return a friendly unit to its owner's hand. You may play a unit with less Might
    // than that unit had, ignoring its cost." (l'unité est jouée depuis ta main ; sa Might quand elle a quitté le
    // plateau)
    r.card(Impl {
        abilities: vec![ab(
            "Swap",
            Cost::from("1 chaos rune"),
            AbilityArgs {
                exhaust: true,
                preds: Value::List(vec![p_friend_v()]),
                resolve: res_fn("_transpo", |g, it| {
                    let Some(u) = g.legal(it, 0)? else { return Ok(()) };
                    let m = g.might(u)?;
                    g.to_zone(u, "hand", false, None)?;
                    let c = ctrl(g, it);
                    let cs: Vec<O> =
                        g.p[c].hand.iter().copied().filter(|&x| ty(g, x) == "Unit" && might_of(g, x) < m).collect();
                    if cs.is_empty() {
                        return Ok(());
                    }
                    let s = sort_by(g, cs, |g, x| Ok((-cval(g, x), g.o(x).uid)))?;
                    let mut opts: Vec<Value> = s.iter().map(|&x| Value::Obj(x)).collect();
                    opts.push(Value::None);
                    if let Some(x) = g.ask(c, "transpo_pick", opts, Map::new())?.opt_obj() {
                        let cands = free_choices(g, c, x, "hand", false, &[])?;
                        play_from(g, c, x, "hand", cands, false)?;
                    }
                    Ok(())
                }),
                choices: ab_choices_fn("Transpositionist.choices", |g, pid, _o| {
                    let us = g.units(Some(pid), None);
                    let s = sort_by(g, us, |g, u| Ok((-g.might(u)?, g.o(u).uid)))?;
                    Ok(tg_choices(g, &s))
                }),
                ..Default::default()
            },
        )],
        ..imp("Transpositionist")
    });
}

/// « return this to its owner's hand » (The Good Stuff, Demacian Standard)
fn to_hand_me(name: &'static str) -> Value {
    res_fn(name, |g, it| {
        if let Some(m) = me(g, it) {
            g.to_zone(m, "hand", false, None)?;
        }
        Ok(())
    })
}

// ====================================================================== Order
fn poster(g: &mut Game, o: O) {
    let res = res_fn("_poster.res", |g, it| make_token(g, "Recruit", ctrl(g, it), Loc::Base, false).map(|_| ()));
    q(g, o, "Concert Poster", res, Map::new(), true, Value::None, may_pay(1, Value::None));
}

/// `_burn_away_pred(g, it, o)`
fn burn_away_pred(g: &mut Game, o: O) -> Res<bool> {
    Ok(g.is_gear(o) || (ty(g, o) == "Unit" && g.might(o)? <= 3))
}

/// Mordekaiser, Dominating — `_morde_dom(g, o)`
fn morde_dom(g: &mut Game, o: O) {
    let can = Value::func("_morde_dom.can", |g, a| {
        let c = ctrl(g, a[0].item());
        Ok(Value::Bool(!g.p[c].hand.is_empty()))
    });
    let cost = Value::func_attrs("_morde_dom.cost", crate::map! { can = can }, |g, a| {
        let c = ctrl(g, a[0].item());
        if g.p[c].hand.is_empty() {
            return Ok(Value::Bool(false));
        }
        discard_n(g, c, 1)?;
        Ok(Value::Bool(true))
    });
    let res = res_fn("_morde_dom.res", |g, it| {
        if let Some(x) = g.legal(it, 0)? {
            let c = ctrl(g, it);
            g.kill(&[x], Some(c), false)?;
        }
        Ok(())
    });
    let opts = opts_fn("_morde_dom.opts", |g, it| {
        let c = ctrl(g, it);
        let xs: Vec<O> =
            g.board.clone().into_iter().filter(|&x| g.o(x).ctrl != c && (ty(g, x) == "Unit" || g.is_gear(x))).collect();
        sort_by(g, xs, |g, x| Ok((-value(g, x)?, g.o(x).uid)))
    });
    let pred = pred_fn("_morde_dom.pred", |g, _it, x| Ok(ty(g, x) == "Unit" || g.is_gear(x)));
    q(g, o, "Mordekaiser, Dominating", res, Map::new(), true, trig_target(opts, pred, "target", false, true), cost);
}

fn sna_trig(g: &mut Game, o: O) {
    q0(g, o, "Seraphine, Not Alone", res_fn("_sna_trig.res", |g, it| make_token(g, "Recruit", ctrl(g, it), Loc::Base, false).map(|_| ())));
}

fn order(r: &mut Registry) {
    // Concert Poster — "[Deploy] When you play this or when you conquer here, you may pay [1] to play a 1 might Recruit
    // unit token to your base."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        on_play: on_play_fn("Concert Poster.on_play", |g, o, _ctx| {
            poster(g, o);
            Ok(())
        }),
        on_event: on_event_fn("Concert Poster.on_event", |g, o, ev, info| {
            if ev == "conquer" && info.g("pid").int() as usize == g.o(o).ctrl && bf_is(g, o, info) {
                poster(g, o);
            }
            Ok(())
        }),
        ..imp("Concert Poster")
    });

    // For Demacia! — "[Reaction] Choose a battlefield. Give friendly units there [Shield 2] this turn."
    r.card(Impl {
        timing: Some("reaction"),
        resolve: res_fn("_for_demacia", |g, it| {
            let c = ctrl(g, it);
            let b = Some(Loc::Bf(dg(g, it, "bf").int() as usize));
            for u in g.units(Some(c), None) {
                if g.o(u).loc == b {
                    g.grant(u, "Shield", 2, turn());
                }
            }
            Ok(())
        }),
        choices: choices_fn("For Demacia!.choices", |g, pid, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let mut bs: Vec<usize> =
                (0..g.bfs.len()).filter(|&b| hb.is_none() || Some(Loc::Bf(b)) == hb).collect();
            bs.sort_by_key(|&b| -(g.units(Some(pid), None).into_iter().filter(|&u| g.o(u).loc == Some(Loc::Bf(b))).count() as i64));
            Ok(bs.into_iter().map(|b| crate::map! { bf = b }).collect())
        }),
        ..imp("For Demacia!")
    });

    // Herald of the Emperor — "When you play me, choose a friendly unit. Its base Might becomes 5 this turn."
    r.card(Impl {
        on_play: on_play_fn("Herald of the Emperor.on_play", |g, o, _ctx| {
            let res = res_fn("_herald_emp", |g, it| {
                if let Some(u) = g.legal(it, 0)? {
                    base_might(g, u, 5);
                }
                Ok(())
            });
            let opts = opts_fn("Herald of the Emperor.opts", |g, it| {
                let c = ctrl(g, it);
                let fr = friends(g, c, false, None)?;
                sort_by(g, fr, |g, u| Ok((might_of(g, u), g.o(u).uid)))
            });
            q(g, o, "Herald of the Emperor", res, Map::new(), false, trig_target(opts, p_friend_v(), "target", false, true), Value::None);
            Ok(())
        }),
        ..imp("Herald of the Emperor")
    });

    // Hunting Dog — "[Deathknell] Play a 1 might Bird unit token with [Deflect] to your base."
    r.card(Impl {
        deathknell: res_fn("Hunting Dog.deathknell", |g, it| make_token(g, "Bird", ctrl(g, it), Loc::Base, false).map(|_| ())),
        ..imp("Hunting Dog")
    });

    // Meticulous Restorationist — "When you play a card with Energy cost [7] or more, you may play me from your trash
    // for [Order]." (un coût alternatif, 356.1.a, comme capacité déclenchée de la carte dans la défausse)
    r.card(Impl {
        hooks: mk(&[(
            "zone_event",
            on_event_fn("_resto_zone", |g, c, ev, info| {
                if ev == "played"
                    && zone_is(g, c, Zone::Trash)
                    && info.g("pid").int() as usize == g.o(c).owner
                    && played_card(info)
                    && e_of(g, info.g("card").obj()) >= 7
                {
                    let res = res_fn("_resto_zone.res", |g, it| {
                        let c = ctrl(g, it);
                        let uid = dg(g, it, "uid").int() as u32;
                        if let Some(x) = g.p[c].trash.iter().copied().find(|&y| g.o(y).uid == uid) {
                            play_unit_free(g, c, x, "trash", None, false, Some((0, vec![req(&["Order"])])))?;
                        }
                        Ok(())
                    });
                    let (owner, uid) = (g.o(c).owner, g.o(c).uid);
                    g.queue_trigger(owner, "Meticulous Restorationist (trash)", res, crate::map! { uid = uid }, None, true, Value::None, Value::None);
                }
                Ok(())
            }),
        )]),
        ..imp("Meticulous Restorationist")
    });

    // One-Two Punch — "[Action] [Buff] a friendly unit and [Stun] an enemy unit."
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_friend_v(), p_enemy_v()]),
        resolve: res_fn("_one_two", |g, it| {
            let (f, e) = (g.legal(it, 0)?, g.legal(it, 1)?);
            if f.is_some() {
                g.buff(f, None)?;
            }
            if e.is_some() {
                let c = ctrl(g, it);
                g.stun(e, Some(c))?;
            }
            Ok(())
        }),
        choices: choices_fn("One-Two Punch.choices", |g, pid, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let fr = friends(g, pid, false, hb)?;
            let mut fs: Vec<O> = fr.iter().copied().filter(|&u| g.o(u).buff == 0).collect();
            fs.extend(fr.iter().copied().filter(|&u| g.o(u).buff != 0));
            let fs = cap(g, fs, 3, Some(pid));
            let mut out = vec![];
            for f in fs {
                let es = enemies(g, pid, false, hb, true)?;
                for e in cap(g, es, 3, Some(pid)) {
                    out.push(crate::map! { tg = Value::Tuple(vec![g.o(f).uid.into(), g.o(e).uid.into()]) });
                }
            }
            Ok(out)
        }),
        ..imp("One-Two Punch")
    });

    // Party Crasher — "I cost [1] less for each exhausted unit you control. [Tank]"
    r.card(Impl {
        kw: crate::map! { Tank = 1 },
        cost_mod: cost_mod_fn("Party Crasher.cost_mod", |g, pid, _c, _ch| {
            Ok((g.units(Some(pid), None).into_iter().filter(|&u| g.exhausted(u)).count() as i64, 0))
        }),
        ..imp("Party Crasher")
    });

    // Protector's Shield — "[Quick-Draw] [Equip] [1]" ; texte d'effet et bonus lus sur l'image : [Tank], +1.
    let mut im = Impl {
        timing: Some("reaction"),
        quickdraw: true,
        bonus: Some(1),
        equip_kw: Some(crate::map! { Tank = 1 }),
        ..imp("Protector's Shield")
    };
    impl_equip(&mut im, Cost::from("1 energy"));
    r.card(im);

    // Burn Away — "Banish a unit with 3 might or less or a gear. [Burn 2]."
    r.card(Impl {
        preds: Value::List(vec![pred_fn("_burn_away_pred", |g, _it, o| burn_away_pred(g, o))]),
        resolve: res_fn("_burn_away", |g, it| {
            let c = ctrl(g, it);
            if let Some(o) = g.legal(it, 0)? {
                g.to_zone(o, "banish", false, Some(c))?;
            }
            g.burn(c, 2)?;
            Ok(())
        }),
        choices: choices_fn("Burn Away.choices", |g, pid, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let mut xs = vec![];
            for o in g.board.clone() {
                if g.targetable(o, Some(pid))? && burn_away_pred(g, o)? && (hb.is_none() || g.o(o).loc == hb) {
                    xs.push(o);
                }
            }
            let s = sort_by(g, xs, |g, o| Ok((g.o(o).ctrl == pid, -value(g, o)?, g.o(o).uid)))?;
            Ok(tg_choices(g, &s))
        }),
        ..imp("Burn Away")
    });

    // Dune Devourer — "You may kill a friendly unit as an additional cost to play me. If you do, I cost [2] less.
    // [Deflect]"
    r.card(Impl {
        kw: crate::map! { Deflect = 1 },
        cost_mod: cost_mod_fn("Dune Devourer.cost_mod", |_g, _pid, _c, ch| Ok(if !ch.g("kill").is_none() { (2, 0) } else { (0, 0) })),
        as_played: as_played_fn("Dune Devourer.as_played", |g, pid, _c| {
            let mut out = vec![Map::new()];
            for o in kill_cost_opts(g, pid, false, true)? {
                out.push(crate::map! { kill = g.o(o).uid });
            }
            Ok(out)
        }),
        ..imp("Dune Devourer")
    });

    // Mournful Spirit — "[Deathknell] You may kill a unit or gear here." (là où il est mort ; choisi quand le
    // Deathknell est finalisé)
    r.card(Impl {
        deathknell: res_fn("Mournful Spirit.deathknell", |g, it| {
            if let Some(x) = g.legal(it, 0)? {
                let c = ctrl(g, it);
                g.kill(&[x], Some(c), false)?;
            }
            Ok(())
        }),
        dk_choose: bool_fn("_spirit_choose", |g, it| {
            let loc = dg(g, it, "info").map().g("loc").opt_loc();
            let c = ctrl(g, it);
            let mut cs = vec![];
            for o in g.board.clone() {
                if g.o(o).loc == loc
                    && (ty(g, o) == "Unit" || g.is_gear(o))
                    && g.targetable(o, Some(c))?
                    && deflect_ok(g, c, o)?
                {
                    cs.push(o);
                }
            }
            if cs.is_empty() {
                return Ok(false);
            }
            let cs = sort_by(g, cs, |g, o| Ok((g.o(o).ctrl == c, -value(g, o)?, g.o(o).uid)))?;
            let mut opts: Vec<Value> = cs.iter().map(|&o| Value::Obj(o)).collect();
            opts.push(Value::None);
            let Some(o) = g.ask(c, "spirit_kill", opts, crate::map! { item = it })?.opt_obj() else { return Ok(false) };
            if !pay_deflect(g, it, o)? {
                return Ok(false);
            }
            let pred = pred_fn("_spirit_choose.pred", |g, it_, x| Ok(g.o(x).loc == dg(g, it_, "info").map().g("loc").opt_loc()));
            g.add_target(it, o, pred);
            Ok(true)
        }),
        ..imp("Mournful Spirit")
    });

    // Rousing Display — "[Show Off] a card with Energy cost [7] or more. Play two 1 might Recruit unit tokens to your
    // base. If you showed off, play four to your base instead."
    r.card(Impl {
        resolve: res_fn("_rousing", |g, it| {
            let n = if dg(g, it, "shown").truthy() { 4 } else { 2 };
            let c = ctrl(g, it);
            for _ in 0..n {
                make_token(g, "Recruit", c, Loc::Base, false)?;
            }
            Ok(())
        }),
        show_off: show_off_fn("Rousing Display.show_off", |g, _pid, _c, x| Ok(!g.o(x).token && e_of(g, x) >= 7)),
        ..imp("Rousing Display")
    });

    // Steadfast Outrider — "[Tank] While there are 2 or more other friendly units and/or gear at my battlefield, I have
    // +2 might."
    r.card(Impl {
        kw: crate::map! { Tank = 1 },
        might_mod: might_mod_fn("Steadfast Outrider.might_mod", |g, o| {
            if !is_bf(g.o(o).loc) {
                return Ok(0);
            }
            let n = g
                .board
                .iter()
                .filter(|&&x| x != o && g.o(x).ctrl == g.o(o).ctrl && g.o(x).loc == g.o(o).loc && (ty(g, x) == "Unit" || g.is_gear(x)))
                .count();
            Ok(if n >= 2 { 2 } else { 0 })
        }),
        ..imp("Steadfast Outrider")
    });

    // Trifarian Captain — "When you play me, look at the top 4 cards of your Main Deck. You may reveal from among them a
    // unit or gear with Energy cost no more than [3] and with Power cost no more than [A] and draw it. Recycle the
    // rest."
    r.card(Impl {
        on_play: on_play_fn("Trifarian Captain.on_play", |g, o, _ctx| {
            q0(
                g,
                o,
                "Trifarian Captain",
                res_fn("_captain", |g, it| {
                    let c = ctrl(g, it);
                    look_pick(g, c, 4, "captain_pick", |g, x| {
                        matches!(ty(g, x), "Unit" | "Gear") && e_of(g, x) <= 3 && p_of(g, x) <= 1
                    })
                }),
            );
            Ok(())
        }),
        ..imp("Trifarian Captain")
    });

    // Battlefield Armorer — "When a friendly gear at a battlefield dies, you may pay [1] to return it to its owner's
    // hand." (un jeton cesse d'exister, 186.1 : rien à renvoyer)
    r.card(Impl {
        on_event: on_event_fn("_armorer", |g, o, ev, info| {
            if ev == "die" {
                let inf = info.g("info").map().clone();
                if inf.g("ctrl").int() as usize == g.o(o).ctrl
                    && spec(inf.g("spec").str()).is_gear()
                    && is_bf(inf.g("loc").opt_loc())
                    && !inf.g("token").truthy()
                {
                    let c = inf.g("obj").obj();
                    let res = res_fn("_armorer.res", |g, it| {
                        let (uid, oid) = (dg(g, it, "uid"), dg(g, it, "oid"));
                        let x = (0..g.p.len())
                            .flat_map(|p| g.p[p].trash.clone())
                            .find(|&y| Value::from(g.o(y).uid) == uid && Value::from(g.o(y).oid) == oid);
                        if let Some(x) = x {
                            g.to_zone(x, "hand", false, None)?;
                        }
                        Ok(())
                    });
                    let d = crate::map! { uid = g.o(c).uid, oid = g.o(c).oid };
                    q(g, o, "Battlefield Armorer", res, d, true, Value::None, may_pay(1, Value::None));
                }
            }
            Ok(())
        }),
        ..imp("Battlefield Armorer")
    });

    // Demacian Illuminators — "When one or more friendly units die, [Buff] a friendly unit." (une fois par évènement
    // de mort)
    r.card(Impl {
        on_event: on_event_fn("_illum", |g, o, ev, info| {
            if ev == "die" {
                let inf = info.g("info").map().clone();
                let key = skey(&["illum".into(), g.o(o).uid.into(), g.o(o).oid.into(), inf.g("batch")]);
                if inf.g("ctrl").int() as usize == g.o(o).ctrl
                    && inf_type(&inf) == "Unit"
                    && g.board.contains(&o)
                    && g.stats.get(&key) == 0
                {
                    stat_set(g, &key, 1);
                    let res = res_fn("_illum.res", |g, it| {
                        let u = g.legal(it, 0)?;
                        if u.is_some() {
                            g.buff(u, None)?;
                        }
                        Ok(())
                    });
                    let opts = opts_fn("_illum.opts", |g, it| {
                        let c = ctrl(g, it);
                        buff_order(g, c)
                    });
                    q(g, o, "Demacian Illuminators", res, Map::new(), false, trig_target(opts, p_friend_v(), "target", false, true), Value::None);
                }
            }
            Ok(())
        }),
        ..imp("Demacian Illuminators")
    });

    // Heimerdinger, Academy Instructor — "If you would play a unit token to a location while I'm at a battlefield, you
    // may play a 3 might Mech unit token to that same location instead."
    r.card(Impl {
        hooks: mk(&[(
            "token_swap",
            Value::func("Heimerdinger, Academy Instructor.token_swap", |g, a| {
                let (s, pid) = (a[0].obj(), a[1].int() as usize);
                Ok(if zone_is(g, s, Zone::Board) && is_bf(g.o(s).loc) && pid == g.o(s).ctrl && a[2].str() != "Mech" {
                    Value::from("Mech")
                } else {
                    Value::None
                })
            }),
        )]),
        ..imp("Heimerdinger, Academy Instructor")
    });

    // Jarvan IV, Royal Heir — "[Assault] [Tank] When I attack, you may move a unit you control to my battlefield."
    r.card(Impl {
        kw: crate::map! { Assault = 1, Tank = 1 },
        on_event: on_event_fn("_jarvan_rh", |g, o, ev, info| {
            if ev == "attack" && info.g("obj").opt_obj() == Some(o) {
                let here = g.o(o).loc;
                let res = res_fn("_jarvan_rh.res", move |g, it| {
                    if let (Some(u), Some(h)) = (g.legal(it, 0)?, here) {
                        if g.o(u).loc != here {
                            let c = ctrl(g, it);
                            g.move_(&[u], h, Some(c), false)?;
                        }
                    }
                    Ok(())
                });
                let opts = opts_fn("_jarvan_rh.opts", move |g, it| {
                    let c = ctrl(g, it);
                    let mut out = vec![];
                    for u in friends(g, c, false, None)? {
                        if g.o(u).loc != here && g.can_move(u, here.unwrap(), Some(c), false)? {
                            out.push(u);
                        }
                    }
                    Ok(out)
                });
                let pred = pred_fn("_jarvan_rh.pred", |g, it, u| Ok(g.o(u).ctrl == ctrl(g, it)));
                q(g, o, "Jarvan IV, Royal Heir", res, Map::new(), true, trig_target(opts, pred, "target", false, true), Value::None);
            }
            Ok(())
        }),
        ..imp("Jarvan IV, Royal Heir")
    });

    // Raucous Anthem — "Ready your units and give them +3 might this turn."
    r.card(Impl {
        resolve: res_fn("_anthem", |g, it| {
            let c = ctrl(g, it);
            for u in g.units(Some(c), None) {
                g.ready_obj(u, None, None)?;
                g.mod_(u, 3, turn(), None)?;
            }
            Ok(())
        }),
        ..imp("Raucous Anthem")
    });

    // Remember the Fallen — "Play up to 3 units and/or gear from your trash, each with Energy cost no more than [3] and
    // Power cost no more than [A], ignoring their costs."
    r.card(Impl {
        resolve: res_fn("_remember", |g, it| {
            let pid = ctrl(g, it);
            for _ in 0..3 {
                let mut cs = vec![];
                for c in g.p[pid].trash.clone() {
                    if matches!(ty(g, c), "Unit" | "Gear")
                        && e_of(g, c) <= 3
                        && p_of(g, c) <= 1
                        && g.impl_o(c).is_some()
                        && !free_choices(g, pid, c, "trash", false, &[])?.is_empty()
                    {
                        cs.push(c);
                    }
                }
                if cs.is_empty() {
                    return Ok(());
                }
                let s = sort_by(g, cs, |g, c| Ok((-cval(g, c), g.o(c).uid)))?;
                let mut opts: Vec<Value> = s.iter().map(|&c| Value::Obj(c)).collect();
                opts.push(Value::None);
                let Some(c) = g.ask(pid, "remember_pick", opts, Map::new())?.opt_obj() else { return Ok(()) };
                let cands = free_choices(g, pid, c, "trash", false, &[])?;
                play_from(g, pid, c, "trash", cands, false)?;
            }
            Ok(())
        }),
        ..imp("Remember the Fallen")
    });

    // Dockside Lock-Up — "[Deploy] When you play this, banish a unit here. When you hold here, kill this to play the
    // banished card, ignoring its cost. (Tokens aren't cards.)"
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        on_play: on_play_fn("_lockup_play", |g, o, _ctx| {
            let here = g.o(o).loc;
            let res = res_fn("_lockup_play.res", |g, it| {
                let Some(u) = g.legal(it, 0)? else { return Ok(()) };
                let tok = g.o(u).token;
                let c = ctrl(g, it);
                g.to_zone(u, "banish", false, Some(c))?;
                let m = me(g, it);
                if let Some(m) = m {
                    if !tok {
                        let src = Value::Tuple(vec![g.o(m).uid.into(), g.o(m).oid.into()]);
                        let (cu, co) = (g.o(u).uid, g.o(u).oid);
                        g.add_effect(mk(&[("kind", "lockup".into()), ("src", src), ("card", cu.into()), ("coid", co.into())]));
                    }
                }
                Ok(())
            });
            let opts = opts_fn("_lockup_play.opts", move |g, it| {
                let c = ctrl(g, it);
                let us: Vec<O> = g.all_units().into_iter().filter(|&u| g.o(u).loc == here).collect();
                sort_by(g, us, |g, u| Ok((g.o(u).ctrl == c, -value(g, u)?, g.o(u).uid)))
            });
            let pred = pred_fn("_lockup_play.pred", move |g, _it, u| Ok(g.o(u).loc == here));
            q(g, o, "Dockside Lock-Up", res, Map::new(), false, trig_target(opts, pred, "target", false, true), Value::None);
            Ok(())
        }),
        on_event: on_event_fn("_lockup_hold", |g, o, ev, info| {
            if ev == "hold" && info.g("pid").int() as usize == g.o(o).ctrl && bf_is(g, o, info) {
                let cost = bool_fn("_lockup_hold.cost", |g, it| {
                    let Some(m) = me(g, it) else { return Ok(false) };
                    let c = ctrl(g, it);
                    Ok(!g.kill(&[m], Some(c), true)?.is_empty())
                });
                let res = res_fn("_lockup_hold.res", |g, it| {
                    let ef = dg(g, it, "lock");
                    if ef.is_none() {
                        return Ok(());
                    }
                    let ef = ef.map().clone();
                    let x = (0..g.p.len())
                        .flat_map(|p| g.p[p].banish.clone())
                        .find(|&y| Value::from(g.o(y).uid) == ef.g("card") && Value::from(g.o(y).oid) == ef.g("coid"));
                    if let Some(x) = x {
                        let c = ctrl(g, it);
                        let cands = free_choices(g, c, x, "banish", false, &[])?;
                        play_from(g, c, x, "banish", cands, false)?;
                    }
                    Ok(())
                });
                let src = Value::Tuple(vec![g.o(o).uid.into(), g.o(o).oid.into()]);
                let lock: Value = g
                    .effects
                    .iter()
                    .find(|e| e.d.g("kind").opt_str() == Some("lockup") && e.d.g("src") == src)
                    .map(|e| Value::Map(e.d.clone()))
                    .unwrap_or(Value::None);
                q(g, o, "Dockside Lock-Up (hold)", res, crate::map! { lock = lock }, false, Value::None, cost);
            }
            Ok(())
        }),
        ..imp("Dockside Lock-Up")
    });

    // Mordekaiser, Dominating — "When you play me or when I attack, you may discard 1 to kill a unit or gear."
    r.card(Impl {
        on_play: on_play_fn("Mordekaiser, Dominating.on_play", |g, o, _ctx| {
            morde_dom(g, o);
            Ok(())
        }),
        on_event: on_event_fn("Mordekaiser, Dominating.on_event", |g, o, ev, info| {
            if ev == "attack" && info.g("obj").opt_obj() == Some(o) {
                morde_dom(g, o);
            }
            Ok(())
        }),
        ..imp("Mordekaiser, Dominating")
    });

    // Seraphine, Not Alone — "When you play me or when I become exhausted, play a 1 might Recruit unit token to your
    // base. I have +1 might for each other exhausted unit you control."
    r.card(Impl {
        on_play: on_play_fn("Seraphine, Not Alone.on_play", |g, o, _ctx| {
            sna_trig(g, o);
            Ok(())
        }),
        on_event: on_event_fn("Seraphine, Not Alone.on_event", |g, o, ev, info| {
            if ev == "exhausted" && in_objs(info, "objs", o) {
                sna_trig(g, o);
            }
            Ok(())
        }),
        might_mod: might_mod_fn("Seraphine, Not Alone.might_mod", |g, o| {
            Ok(g.units(Some(g.o(o).ctrl), None).into_iter().filter(|&u| u != o && g.exhausted(u)).count() as i64)
        }),
        ..imp("Seraphine, Not Alone")
    });
}

// ====================================================================== légendes et cartes signature
/// `_ekko_kill_end(g, eff, info)`
fn ekko_kill_end() -> Value {
    Value::func("_ekko_kill_end", |g, a| {
        let eid = a[0].int() as u64;
        let eff = g.eff(eid).cloned().unwrap_or_default();
        if let Some(u) = g.obj(eff.g("uid").int() as u32) {
            if Value::from(g.o(u).oid) == eff.g("oid") {
                let res = res_fn("Kill me at end of turn", |g, it| {
                    if let Some(x) = g.obj(dg(g, it, "uid").int() as u32) {
                        if Value::from(g.o(x).oid) == dg(g, it, "oid") {
                            let c = ctrl(g, it);
                            g.kill(&[x], Some(c), false)?;
                        }
                    }
                    Ok(())
                });
                let (c, uid, oid) = (g.o(u).ctrl, g.o(u).uid, g.o(u).oid);
                g.queue_trigger(c, "Kill me at end of turn", res, crate::map! { uid = uid, oid = oid }, Some(uid), false, Value::None, Value::None);
            }
        }
        g.remove_effect(eid);
        Ok(Value::None)
    })
}

/// `_ekko_boy(g, it)`
fn ekko_boy() -> Value {
    res_fn("_ekko_boy", |g, it| {
        let pid = ctrl(g, it);
        let key = Value::Tuple(vec!["ekko_boy".into(), g.it(it).id.into()]);
        let modf = Value::func("_ekko_boy.mod", |_g, a| {
            let what = ctx_of(&a[2]);
            Ok(if what.g("kind").opt_str() == Some("unit") && !what.g("card").is_none() {
                maps_v(vec![crate::map! { e = -2, rm = 1 }])
            } else {
                Value::List(vec![])
            })
        });
        let k2 = key.clone();
        let mark = Value::func("_ekko_boy.mark", move |g, a| {
            let info = ctx_of(&a[1]);
            if info.g("pid").int() as usize == pid && !info.g("token").truthy() && ty(g, info.g("card").obj()) == "Unit" {
                g.effects.retain(|e| e.d.g("key") != k2);
                let c = info.g("card").obj();
                let (uid, oid) = (g.o(c).uid, g.o(c).oid);
                g.add_effect(mk(&[
                    ("on", "end_turn".into()),
                    ("fn", ekko_kill_end()),
                    ("uid", uid.into()),
                    ("oid", oid.into()),
                    ("dur", turn()),
                ]));
            }
            Ok(Value::None)
        });
        g.add_effect(mk(&[
            ("kind", "cost_mod".into()),
            ("pid", pid.into()),
            ("fn", modf),
            ("once", true.into()),
            ("key", key.clone()),
            ("dur", turn()),
        ]));
        g.add_effect(mk(&[("on", "played".into()), ("fn", mark), ("key", key), ("dur", turn())]));
        Ok(())
    })
}

/// `_chronobreak(g, it)`
fn chronobreak() -> Value {
    res_fn("_chronobreak", |g, it| {
        let pid = ctrl(g, it);
        let n = dg(g, it, "x").opt_int().unwrap_or(0).max(0) as usize;
        let us = g.units(Some(pid), None);
        let mut chosen: Vec<O> = vec![];
        for _ in 0..n.min(us.len()) {
            let mut cs: Vec<O> = us.iter().copied().filter(|u| !chosen.contains(u)).collect();
            cs.sort_by_key(|&u| (!g.exhausted(u), g.o(u).uid));
            let u = g.ask_obj(pid, "chrono_pick", &cs, Map::new())?.unwrap();
            chosen.push(u);
        }
        let mut back: Vec<Value> = vec![];
        for u in chosen {
            let (loc, owner, tok) = (g.o(u).loc, g.o(u).owner, g.o(u).token);
            g.to_zone(u, "banish", false, Some(pid))?;
            if !tok {
                back.push(Value::Tuple(vec![u.into(), loc.into(), owner.into()]));
            }
        }
        while !back.is_empty() {
            let t = if back.len() == 1 { back.remove(0) } else { g.ask(pid, "chrono_order", back.clone(), Map::new())? };
            if let Some(i) = back.iter().position(|x| *x == t) {
                back.remove(i);
            }
            let tl = t.list().to_vec();
            let (u, loc, owner) = (tl[0].obj(), tl[1].opt_loc(), tl[2].int() as usize);
            let uid = g.o(u).uid;
            if let Some(c) = g.p[owner].banish.iter().copied().find(|&x| g.o(x).uid == uid) {
                let mut locs = vec![Loc::Base];
                if let Some(l) = loc.filter(|l| l.is_bf()) {
                    locs.push(l);
                }
                play_unit_free(g, owner, c, "banish", Some(locs), false, None)?;
            }
        }
        Ok(())
    })
}

/// `_minefield(g, it)`
fn minefield() -> Value {
    res_fn("_minefield", |g, it| {
        let c = ctrl(g, it);
        let bombs: Vec<O> = dg(g, it, "bombs")
            .list()
            .iter()
            .filter_map(|u| g.obj(u.int() as u32))
            .filter(|&b| g.o(b).ctrl == c && g.cname(b) == "Bomb")
            .collect();
        let killed = if !bombs.is_empty() { g.kill(&bombs, Some(c), false)? } else { vec![] };
        let n = killed.len();
        if n != 0 {
            let pid = c;
            let at_end = Value::func("_minefield.at_end", move |g, a| {
                let eid = a[0].int() as u64;
                let eff = g.eff(eid).cloned().unwrap_or_default();
                g.remove_effect(eid);
                let res = res_fn("_minefield.res", |g, it| {
                    let c = ctrl(g, it);
                    let loc = bf_ask(g, c, "bomb_location", None)?;
                    for _ in 0..dg(g, it, "n").int() {
                        bomb(g, c, loc)?;
                    }
                    Ok(())
                });
                g.queue_trigger(pid, "Hexplosive Minefield (end of turn)", res, crate::map! { n = eff.g("n") }, None, false, Value::None, Value::None);
                Ok(Value::None)
            });
            g.add_effect(mk(&[("on", "end_turn".into()), ("fn", at_end), ("n", n.into()), ("dur", turn())]));
        }
        Ok(())
    })
}

/// `_ball_trig(g, o)`
fn ball_trig(g: &mut Game, o: O) {
    let choose = bool_fn("_ball_trig.choose", |g, it| {
        let m = src_obj(g, it);
        let c = ctrl(g, it);
        let n = g
            .units(Some(1 - c), None)
            .into_iter()
            .filter(|&u| m.is_some_and(|m| g.o(u).loc == g.o(m).loc) && is_bf(g.o(u).loc))
            .count();
        let mut opts = vec![
            opt(tr("piocher 1", "draw 1"), "draw".into()),
            opt(tr("2 dégâts aux ennemis ici", "deal 2 to enemy units here"), "deal".into()),
        ];
        if n >= 2 {
            opts.reverse();
        }
        let m = g.ask(c, "ball_mode", opts, crate::map! { item = it })?.opt().value.clone();
        g.it_mut(it).data.set("mode", m);
        Ok(true)
    });
    let res = res_fn("_ball_trig.res", |g, it| {
        let m = src_obj(g, it);
        let c = ctrl(g, it);
        if dg(g, it, "mode").opt_str() == Some("draw") {
            g.draw(c, 1)?;
        } else if let Some(m) = m.filter(|&m| is_bf(g.o(m).loc)) {
            let us: Vec<O> = g.units(Some(1 - c), None).into_iter().filter(|&u| g.o(u).loc == g.o(m).loc).collect();
            for u in us {
                g.deal(Some(u), 2, "ability", Some(c), None)?;
            }
        }
        Ok(())
    });
    q(g, o, "The Ball", res, Map::new(), false, choose, Value::None);
}

thread_local! {
    /// `_KS_GUARD`
    static KS_GUARD: RefCell<BTreeSet<u32>> = const { RefCell::new(BTreeSet::new()) };
}

/// `_blade_choices(g, pid, o)`
fn blade_choices() -> Value {
    ab_choices_fn("_blade_choices", |g, pid, o| {
        let mut out = vec![];
        for u in friends(g, pid, false, None)? {
            if Some(g.o(u).uid) == g.o(o).attached_to {
                continue;
            }
            out.push(crate::map! { tg = tg1(g, u) });
            let xs = exh_cost_choices(g, pid)?;
            for x in cap(g, xs, 2, Some(pid)) {
                out.push(crate::map! { tg = tg1(g, u), exh = g.o(x).uid });
            }
        }
        Ok(out)
    })
}

/// Seraphine, Starry-Eyed Songstress : la source est-elle la légende de pid ?
fn legend_of(g: &Game, src: &Value, pid: usize) -> Option<O> {
    let s = src.opt_obj()?;
    if g.o(s).zone == Some(Zone::Legend) && g.o(s).owner == pid {
        Some(s)
    } else {
        None
    }
}

fn legends(r: &mut Registry) {
    // Ekko, Boy Who Shattered Time — "[Empower] — [1], [E] (Use only if not Empowered.) Disempower me, [E]: Reduce the
    // cost of the next unit you play this turn by [2][A] and give it "Kill me at end of turn.""
    r.card(Impl {
        abilities: vec![
            empower_ability(Cost::from("1 energy"), "main", true),
            mk(&[
                ("name", "Next unit".into()),
                ("timing", "main".into()),
                ("exhaust", true.into()),
                ("can", can_fn("Ekko, Boy.can", |g, _pid, o| Ok(g.o(o).empowered))),
                ("cost", Value::func("Ekko, Boy.cost", |_g, _a| Ok(Value::Tuple(vec![0.into(), Value::List(vec![])])))),
                (
                    "extra_cost",
                    Value::func("Ekko, Boy.extra_cost", |g, a| Ok(Value::Bool(g.disempower(Some(a[1].obj()))))),
                ),
                ("resolve", ekko_boy()),
            ]),
        ],
        ..imp("Ekko, Boy Who Shattered Time")
    });

    // Chronobreak — "As an additional cost to play this, pay any amount of Power. Banish that many units you control,
    // then for each, its owner plays that unit back to the same location or to their base, ignoring its cost. (You
    // choose the order.)" (les unités sont choisies à la résolution ; un jeton cesse d'exister une fois banni, 186.1)
    r.card(Impl {
        resolve: chronobreak(),
        choices: choices_fn("Chronobreak.choices", |g, pid, _ctx| {
            let n = g.units(Some(pid), None).len() as i64;
            Ok((0..=n).rev().map(|x| crate::map! { x = x }).collect())
        }),
        extra_cost_fn: Value::func("Chronobreak.extra_cost_fn", |_g, a| {
            let ch = ctx_of(&a[2]);
            let x = ch.g("x").opt_int().unwrap_or(0).max(0) as usize;
            Ok(Value::Tuple(vec![0.into(), reqs_to_value(&vec![any_req(); x])]))
        }),
        ..imp("Chronobreak")
    });

    // Ziggs, Hexplosives Expert — "[1][A], [E]: Play a Bomb gear token to a battlefield."
    r.card(Impl {
        abilities: vec![ab(
            "Bomb",
            Cost::from("1 energy and 1 rune of any type"),
            AbilityArgs {
                exhaust: true,
                resolve: res_fn("Ziggs, Hexplosives Expert.bomb", |g, it| bomb(g, ctrl(g, it), None).map(|_| ())),
                ..Default::default()
            },
        )],
        ..imp("Ziggs, Hexplosives Expert")
    });

    // Hexplosive Minefield — "[Action] Kill any number of friendly Bomb tokens. At end of turn, play that many Bomb
    // tokens to a single battlefield."
    r.card(Impl {
        timing: Some("action"),
        resolve: minefield(),
        choices: choices_fn("_minefield_choices", |g, pid, _ctx| {
            let mut bs: Vec<u32> =
                g.board.iter().filter(|&&o| g.o(o).ctrl == pid && g.cname(o) == "Bomb").map(|&o| g.o(o).uid).collect();
            bs.sort();
            let mut out = vec![];
            for k in (0..=bs.len()).rev() {
                for grp in combinations(&bs, k) {
                    out.push(crate::map! { bombs = Value::Tuple(grp.into_iter().map(Value::from).collect()) });
                    if !full_choices(g, Some(pid)) {
                        break;
                    }
                }
            }
            Ok(out)
        }),
        ..imp("Hexplosive Minefield")
    });

    // Mordekaiser, Iron Revenant — "[Reaction][>] Banish a unit or gear from your trash, [E]: [Add] [A]. Spend this Power
    // only to play units or gear." (un [Add] utilisé en payant une unité ou un équipement, 429 ; son autre coût à
    // l'usage)
    r.card(Impl {
        add: vec![mk(&[
            ("e", 0.into()),
            ("p", Value::List(vec!["A".into()])),
            ("exhaust", true.into()),
            ("kill", false.into()),
            (
                "can",
                Value::func("_iron_can", |g, a| {
                    let pid = a[0].int() as usize;
                    let ctx = &a[2];
                    let ok = match ctx {
                        Value::Map(m) => {
                            matches!(m.g("kind").opt_str(), Some("unit") | Some("gear"))
                                && g.p[pid].trash.iter().any(|&c| matches!(ty(g, c), "Unit" | "Gear"))
                        }
                        _ => false,
                    };
                    Ok(Value::Bool(ok))
                }),
            ),
            (
                "on_use",
                Value::func("_iron_use", |g, a| {
                    let pid = a[0].int() as usize;
                    let cs: Vec<O> = g.p[pid].trash.iter().copied().filter(|&c| matches!(ty(g, c), "Unit" | "Gear")).collect();
                    let cs = sort_by(g, cs, |g, c| Ok((cval(g, c), g.o(c).uid)))?;
                    if !cs.is_empty() {
                        let c = if cs.len() > 1 { g.ask_obj(pid, "banish_from_trash", &cs, Map::new())?.unwrap() } else { cs[0] };
                        g.to_zone(c, "banish", false, Some(pid))?;
                    }
                    Ok(Value::None)
                }),
            ),
        ])],
        ..imp("Mordekaiser, Iron Revenant")
    });

    // Realm of Death — "Give an enemy unit -2 might this turn and a friendly unit +2 might this turn. They deal damage
    // equal to their Mights to each other." (simultanément, dégâts de sort de son contrôleur)
    r.card(Impl {
        preds: Value::List(vec![p_enemy_v(), p_friend_v()]),
        resolve: res_fn("_realm", |g, it| {
            let (e, f) = (g.legal(it, 0)?, g.legal(it, 1)?);
            if let Some(e) = e {
                g.mod_(e, -2, turn(), None)?;
            }
            if let Some(f) = f {
                g.mod_(f, 2, turn(), None)?;
            }
            if let (Some(e), Some(f)) = (e, f) {
                let (me_, mf) = (0.max(g.might(e)?), 0.max(g.might(f)?));
                let c = ctrl(g, it);
                g.deal(Some(e), mf, "spell", Some(c), None)?;
                g.deal(Some(f), me_, "spell", Some(c), None)?;
            }
            Ok(())
        }),
        choices: choices_fn("Realm of Death.choices", |g, pid, _ctx| {
            let es = enemies(g, pid, false, None, true)?;
            let mut out = vec![];
            for e in cap(g, es, 3, Some(pid)) {
                let fr = friends(g, pid, false, None)?;
                let fr = sort_by(g, fr, |g, u| Ok((-g.might(u)?, g.o(u).uid)))?;
                for f in cap(g, fr, 3, Some(pid)) {
                    out.push(crate::map! { tg = Value::Tuple(vec![g.o(e).uid.into(), g.o(f).uid.into()]) });
                }
            }
            Ok(out)
        }),
        ..imp("Realm of Death")
    });

    // Orianna, Lady of Clockwork — "At the start of your Main Phase, you may pay [1] to choose a friendly gear. It
    // becomes a unit gear with Might equal to its Energy cost until you choose another gear with me." (Obj.ugear : une
    // unité dont la Might est son coût en énergie imprimé ; un Equipment attaché se détache en devenant une unité,
    // 143.5.b)
    r.card(Impl {
        legend_event: idx_event_fn("_ori_legend", |g, pid, ev, info| {
            if ev != "main_start" || info.g("pid").int() as usize != pid || g.gear(Some(pid)).is_empty() {
                return Ok(());
            }
            let choose = bool_fn("_ori_legend.choose", |g, it| {
                let c = ctrl(g, it);
                let gs: Vec<O> = g.gear(Some(c)).into_iter().filter(|&x| !g.o(x).ugear).collect();
                if gs.is_empty() {
                    return Ok(false);
                }
                let s = sort_by(g, gs, |g, x| Ok((-e_of(g, x), g.o(x).uid)))?;
                let x = g.ask_obj(c, "orianna_gear", &s, crate::map! { item = it })?.unwrap();
                let pred = pred_fn("_ori_legend.pred", |g, it2, o| Ok(g.o(o).ctrl == ctrl(g, it2) && g.is_gear(o)));
                g.add_target(it, x, pred);
                Ok(true)
            });
            let res = res_fn("_ori_legend.res", |g, it| {
                let Some(x) = g.legal(it, 0)? else { return Ok(()) };
                let c = ctrl(g, it);
                for y in g.board.clone() {
                    if g.o(y).ugear && g.o(y).ctrl == c && g.o(y).attrs.g("ugear_by") == Value::from(c) {
                        g.o_mut(y).ugear = false;
                    }
                }
                if let Some(au) = g.o(x).attached_to {
                    let xu = g.o(x).uid;
                    if let Some(u) = g.obj(au) {
                        if let Some(i) = g.o(u).attached.iter().position(|&a| a == xu) {
                            g.o_mut(u).attached.remove(i);
                        }
                    }
                    g.o_mut(x).attached_to = None;
                }
                g.o_mut(x).ugear = true;
                g.o_mut(x).attrs.set("ugear_by", c);
                g.need_cleanup = true;
                let cn = g.cname(x).to_string();
                g.log(|| format!("  {cn} becomes a unit gear"));
                Ok(())
            });
            g.queue_trigger(pid, "Orianna, Lady of Clockwork", res, Map::new(), None, true, choose, may_pay(1, Value::None));
            Ok(())
        }),
        ..imp("Orianna, Lady of Clockwork")
    });

    // The Ball — "[Deploy] [Ganking] When you play or move this, choose one — Draw 1. Deal 2 to enemy units here."
    r.card(Impl {
        kw: crate::map! { Deploy = 1, Ganking = 1 },
        on_play: on_play_fn("The Ball.on_play", |g, o, _ctx| {
            ball_trig(g, o);
            Ok(())
        }),
        on_event: on_event_fn("The Ball.on_event", |g, o, ev, info| {
            if ev == "move" && info.g("obj").opt_obj() == Some(o) {
                ball_trig(g, o);
            }
            Ok(())
        }),
        ..imp("The Ball")
    });

    // K'Sante, Pride of Nazumah — "[1], [E]: Give a friendly unit "I have [Assault] equal to my [Shield]" this turn."
    // (un texte accordé : gardé comme une marque lue par l'aura de la légende)
    r.card(Impl {
        aura_kw: aura_kw_fn("_ksante_aura", |g, _src, o| {
            let uid = g.o(o).uid;
            if g.o(o).grants.is_empty()
                || KS_GUARD.with(|s| s.borrow().contains(&uid))
                || !g.o(o).grants.iter().any(|(k, _, _)| k == "KSanteAssault")
            {
                return Ok(None);
            }
            KS_GUARD.with(|s| s.borrow_mut().insert(uid));
            let n = g.kw_value(o, "Shield");
            KS_GUARD.with(|s| s.borrow_mut().remove(&uid));
            let n = n?;
            Ok(if n != 0 { Some(crate::map! { Assault = n }) } else { None })
        }),
        abilities: vec![ab(
            "Assault = Shield",
            Cost::from("1 energy"),
            AbilityArgs {
                exhaust: true,
                preds: Value::List(vec![p_friend_v()]),
                resolve: res_fn("K'Sante, Pride of Nazumah.resolve", |g, it| {
                    if let Some(u) = g.legal(it, 0)? {
                        g.grant(u, "KSanteAssault", 1, turn());
                    }
                    Ok(())
                }),
                choices: ab_choices_fn("K'Sante, Pride of Nazumah.choices", |g, pid, _o| {
                    let fr = friends(g, pid, false, None)?;
                    let s = sort_by(g, fr, |g, u| Ok((-g.kw_value(u, "Shield")?, g.o(u).uid)))?;
                    Ok(tg_choices(g, &s))
                }),
                ..Default::default()
            },
        )],
        ..imp("K'Sante, Pride of Nazumah")
    });

    // Ntofo Strikes — "[Action] Give a friendly unit [Shield 2] this turn, then deal damage equal to its [Shield] to an
    // enemy unit." (sa valeur de Shield après l'octroi)
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_friend_v(), p_enemy_v()]),
        resolve: res_fn("_ntofo", |g, it| {
            let (f, e) = (g.legal(it, 0)?, g.legal(it, 1)?);
            let Some(f) = f else { return Ok(()) };
            g.grant(f, "Shield", 2, turn());
            if e.is_some() {
                let (n, c) = (g.kw_value(f, "Shield")?, ctrl(g, it));
                g.deal(e, n, "spell", Some(c), None)?;
            }
            Ok(())
        }),
        choices: choices_fn("Ntofo Strikes.choices", |g, pid, ctx| {
            let hb = ctx.g("hidden_bf").opt_loc();
            let fr = friends(g, pid, false, hb)?;
            let fr = sort_by(g, fr, |g, u| Ok((-g.kw_value(u, "Shield")?, g.o(u).uid)))?;
            let mut out = vec![];
            for f in cap(g, fr, 3, Some(pid)) {
                let es = enemies(g, pid, false, hb, true)?;
                for e in cap(g, es, 3, Some(pid)) {
                    out.push(crate::map! { tg = Value::Tuple(vec![g.o(f).uid.into(), g.o(e).uid.into()]) });
                }
            }
            Ok(out)
        }),
        ..imp("Ntofo Strikes")
    });

    // Riven, The Exile — "Units you play to battlefields you control enter ready." (card.played_to : où elle est jouée)
    r.card(Impl {
        hooks: mk(&[(
            "units_enter_ready",
            Value::func("Riven, The Exile.units_enter_ready", |g, a| {
                let (s, pid, c) = (a[0].obj(), a[1].int() as usize, a[2].obj());
                let ok = pid == g.o(s).owner
                    && zone_is(g, s, Zone::Legend)
                    && ty(g, c) == "Unit"
                    && matches!(g.o(c).played_to, Some(Loc::Bf(b)) if g.bfs[b].ctrl == Some(pid));
                Ok(Value::Bool(ok))
            }),
        )]),
        ..imp("Riven, The Exile")
    });

    // Blade of the Exile — "[Equip] — [A] or exhaust a friendly unit" ; texte d'effet et bonus lus sur la carte :
    // [Ganking], +3 might.
    r.card(Impl {
        abilities: vec![mk(&[
            ("name", "Equip".into()),
            ("timing", "main".into()),
            ("choices", blade_choices()),
            ("preds", Value::List(vec![p_friend_v()])),
            (
                "cost",
                Value::func("_BLADE_EQUIP.cost", |_g, a| {
                    let ch = ctx_of(&a[2]);
                    Ok(if !ch.g("exh").is_none() {
                        Value::Tuple(vec![0.into(), Value::List(vec![])])
                    } else {
                        Value::Tuple(vec![0.into(), reqs_to_value(&[any_req()])])
                    })
                }),
            ),
            ("extra_cost", extra_fn("_BLADE_EQUIP.extra_cost", |g, _pid, _o, ch| {
                pay_exh(g, ch);
                Ok(())
            })),
            (
                "resolve",
                res_fn("_BLADE_EQUIP.resolve", |g, it| {
                    if let (Some(u), Some(s)) = (g.legal(it, 0)?, src_obj(g, it)) {
                        attach(g, s, u)?;
                    }
                    Ok(())
                }),
            ),
        ])],
        bonus: Some(3),
        equip_kw: Some(crate::map! { Ganking = 1 }),
        ..imp("Blade of the Exile")
    });

    // Seraphine, Starry-Eyed Songstress — "As an additional cost to play a spell, you may exhaust me and three friendly
    // units to reduce the cost of that spell by [2]."
    r.card(Impl {
        hooks: mk(&[
            (
                "spell_opts",
                Value::func("_sera_opts", |g, a| {
                    let pid = a[1].int() as usize;
                    let card = a[2].obj();
                    let Some(s) = legend_of(g, &a[0], pid) else { return Ok(Value::List(vec![])) };
                    if g.exhausted(s) || ty(g, card) != "Spell" {
                        return Ok(Value::List(vec![]));
                    }
                    let us = exh_cost_choices(g, pid)?;
                    if us.len() < 3 {
                        return Ok(Value::List(vec![]));
                    }
                    let groups = if full_choices(g, Some(pid)) { combinations(&us, 3) } else { vec![us[..3].to_vec()] };
                    Ok(maps_v(groups.iter().map(|grp| crate::map! { sera = uids_t(g, grp) }).collect()))
                }),
            ),
            (
                "opt_pay",
                Value::func("_sera_pay", |g, a| {
                    let pid = a[1].int() as usize;
                    let ch = ctx_of(&a[3]);
                    if ch.g("sera").truthy() {
                        if let Some(s) = legend_of(g, &a[0], pid) {
                            let mut objs = vec![s];
                            objs.extend(ch.g("sera").list().iter().filter_map(|u| g.obj(u.int() as u32)));
                            for o in objs {
                                g.set_exhausted(o, true); // en même temps (un seul évènement 'exhausted')
                            }
                        }
                    }
                    Ok(Value::None)
                }),
            ),
            (
                "cost_aura",
                Value::func("Seraphine, Starry-Eyed Songstress.cost_aura", |g, a| {
                    let pid = a[1].int() as usize;
                    let what = ctx_of(&a[2]);
                    let sera = match what.g("choice") {
                        Value::Map(m) => m.g("sera").truthy(),
                        _ => false,
                    };
                    Ok(if legend_of(g, &a[0], pid).is_some() && what.g("kind").opt_str() == Some("spell") && sera {
                        maps_v(vec![crate::map! { e = -2 }])
                    } else {
                        Value::List(vec![])
                    })
                }),
            ),
        ]),
        ..imp("Seraphine, Starry-Eyed Songstress")
    });

    // Encore — "Choose two — Deal 2 to a unit. Play two 1 might Recruit unit tokens. Give up to two units each +1 might
    // this turn. Look at the top 2 cards of your Main Deck, draw 1 and recycle the other."
    r.card(Impl {
        preds: Value::List(vec![p_unit_v()]),
        resolve: res_fn("_encore", |g, it| {
            let modes: Vec<String> = dg(g, it, "modes").list().iter().map(|m| m.str().to_string()).collect();
            let has = |m: &str| modes.iter().any(|x| x == m);
            let c = ctrl(g, it);
            let mut i = 0;
            if has("deal") {
                let u = g.legal(it, i)?;
                i += 1;
                if u.is_some() {
                    g.deal(u, 2, "spell", Some(c), None)?;
                }
            }
            if has("recruits") {
                for _ in 0..2 {
                    unit_token(g, c, "Recruit")?;
                }
            }
            if has("plus") {
                for j in i..g.it(it).targets.len() {
                    if let Some(u) = g.legal(it, j)? {
                        g.mod_(u, 1, turn(), None)?;
                    }
                }
            }
            if has("look") {
                let deck: Vec<O> = g.p[c].deck.iter().take(2).copied().collect();
                let top = g.look(c, &deck, None)?;
                if !top.is_empty() {
                    let x = if top.len() > 1 {
                        let s = sort_by(g, top.clone(), |g, x| Ok((-cval(g, x), g.o(x).uid)))?;
                        g.ask_obj(c, "draw_pick", &s, Map::new())?.unwrap()
                    } else {
                        top[0]
                    };
                    g.draw_card(c, x)?;
                    let rest: Vec<O> = top.iter().copied().filter(|&y| y != x && g.p[c].deck.contains(&y)).collect();
                    for &y in &rest {
                        deck_remove(g, c, y);
                    }
                    g.recycle_cards(c, &rest)?;
                }
            }
            Ok(())
        }),
        choices: choices_fn("_encore_choices", |g, pid, _ctx| {
            let full = full_choices(g, Some(pid));
            let mt = minus_targets(g, pid, None, false)?;
            let deal: Vec<Vec<u32>> = cap(g, mt, 3, Some(pid)).into_iter().map(|u| vec![g.o(u).uid]).collect();
            let fr = friends(g, pid, false, None)?;
            let fr: Vec<u32> = cap(g, fr, 3, Some(pid)).into_iter().map(|u| g.o(u).uid).collect();
            let mut plus: Vec<Vec<u32>> = vec![];
            for k in [2, 1] {
                plus.extend(combinations(&fr, k));
            }
            if !full_choices(g, Some(pid)) {
                plus.truncate(2);
            }
            plus.push(vec![]);
            if full_choices(g, Some(pid)) {
                let mut alls: Vec<u32> = g.all_units().into_iter().map(|u| g.o(u).uid).collect();
                alls.sort();
                plus = vec![];
                for k in [2, 1, 0] {
                    plus.extend(combinations(&alls, k));
                }
            }
            let _ = full;
            let mut out = vec![];
            for ab_ in combinations(&["deal", "recruits", "plus", "look"], 2) {
                let (a, b) = (ab_[0], ab_[1]);
                let mut tgs_a = if a == "deal" { deal.clone() } else { vec![vec![]] };
                let mut tgs_b = if b == "plus" { plus.clone() } else { vec![vec![]] };
                if a == "plus" {
                    tgs_a = plus.clone();
                    tgs_b = vec![vec![]];
                }
                for ta in &tgs_a {
                    for tb in &tgs_b {
                        let tg: Vec<u32> = if a == "deal" {
                            ta.iter().chain(tb.iter()).copied().collect()
                        } else if a == "plus" {
                            ta.clone()
                        } else {
                            tb.clone()
                        };
                        if a == "deal" && b == "plus" && ta.iter().any(|x| tb.contains(x)) && !full_choices(g, Some(pid)) {
                            continue;
                        }
                        out.push(crate::map! {
                            modes = Value::Tuple(vec![a.into(), b.into()]),
                            tg = Value::Tuple(tg.into_iter().map(Value::from).collect()),
                        });
                    }
                }
            }
            Ok(out)
        }),
        max_choices: 16,
        ..imp("Encore")
    });

    // Evelynn, Agony's Embrace — "Once each turn, when you play a card from face down, draw 1. Then, if it's not your
    // turn, discard 1."
    r.card(Impl {
        legend_event: idx_event_fn("_eve_legend", |g, pid, ev, info| {
            if ev == "played" && info.g("pid").int() as usize == pid && played_card(info) && from_facedown(g, info) {
                let key = skey(&["eve_agony".into(), pid.into(), g.turn_no.into()]);
                if g.stats.get(&key) != 0 {
                    return Ok(());
                }
                stat_set(g, &key, 1);
                let res = res_fn("_eve_legend.res", |g, it| {
                    let c = ctrl(g, it);
                    g.draw(c, 1)?;
                    if g.tp != c {
                        discard_n(g, c, 1)?;
                    }
                    Ok(())
                });
                g.queue_trigger(pid, "Evelynn, Agony's Embrace", res, Map::new(), None, false, Value::None, Value::None);
            }
            Ok(())
        }),
        ..imp("Evelynn, Agony's Embrace")
    });

    // Last Caress — "[Reaction] The next unit you play this turn (even from face down) can be played to any
    // battlefield. When you play it, [Buff] it."
    r.card(Impl {
        timing: Some("reaction"),
        resolve: res_fn("_caress", |g, it| {
            let pid = ctrl(g, it);
            let key = Value::Tuple(vec!["caress".into(), g.it(it).id.into()]);
            let locs = Value::func("_caress.locs", |g, a| {
                let card = a[1].obj();
                Ok(if ty(g, card) == "Unit" { Value::List(g.bf_locs().into_iter().map(Value::Loc).collect()) } else { Value::List(vec![]) })
            });
            let k2 = key.clone();
            let fire = Value::func("_caress.fire", move |g, a| {
                let info = ctx_of(&a[1]);
                if info.g("pid").int() as usize == pid && !info.g("token").truthy() && ty(g, info.g("card").obj()) == "Unit" {
                    g.effects.retain(|e| e.d.g("key") != k2);
                    let c = info.g("card").obj();
                    let uid = g.o(c).uid;
                    let res = res_fn("Last Caress", |g, it| {
                        let x = g.obj(dg(g, it, "uid").int() as u32);
                        if x.is_some() {
                            g.buff(x, None)?;
                        }
                        Ok(())
                    });
                    g.queue_trigger(pid, "Last Caress", res, crate::map! { uid = uid }, Some(uid), false, Value::None, Value::None);
                }
                Ok(Value::None)
            });
            g.add_effect(mk(&[
                ("kind", "aura_locs".into()),
                ("pid", pid.into()),
                ("fn", locs),
                ("key", key.clone()),
                ("dur", turn()),
            ]));
            g.add_effect(mk(&[("on", "played".into()), ("fn", fire), ("key", key), ("dur", turn())]));
            Ok(())
        }),
        ..imp("Last Caress")
    });

    // Jarvan IV, Exemplar of Demacia — "When you play a gear with [Deploy], you may exhaust me and pay [A] to move a
    // friendly unit to that battlefield."
    r.card(Impl {
        legend_event: idx_event_fn("_jarvan_ex", |g, pid, ev, info| {
            if ev != "played" || info.g("pid").int() as usize != pid {
                return Ok(());
            }
            let x = info.g("card").obj();
            if !g.is_gear(x) || !has_deploy(g, x)? {
                return Ok(());
            }
            let Some(dest) = g.o(x).loc.filter(|l| l.is_bf()) else { return Ok(()) };
            let can = Value::func("_jarvan_ex.can", |g, a| {
                let c = ctrl(g, a[0].item());
                let leg = g.p[c].legend;
                Ok(Value::Bool(!g.exhausted(leg) && g.can_pay(c, 0, &[any_req()], &Value::None)?))
            });
            let cost = Value::func_attrs("_jarvan_ex.cost", crate::map! { can = can }, |g, a| {
                let c = ctrl(g, a[0].item());
                let leg = g.p[c].legend;
                if g.exhausted(leg) || !g.can_pay(c, 0, &[any_req()], &Value::None)? {
                    return Ok(Value::Bool(false));
                }
                g.set_exhausted(leg, true);
                Ok(Value::Bool(g.pay(c, 0, &[any_req()], &Value::None)?))
            });
            let res = res_fn("_jarvan_ex.res", move |g, it| {
                if let Some(u) = g.legal(it, 0)? {
                    if g.o(u).loc != Some(dest) {
                        let c = ctrl(g, it);
                        g.move_(&[u], dest, Some(c), false)?;
                    }
                }
                Ok(())
            });
            let opts = opts_fn("_jarvan_ex.opts", move |g, it| {
                let c = ctrl(g, it);
                Ok(friends(g, c, false, None)?.into_iter().filter(|&u| g.o(u).loc != Some(dest)).collect())
            });
            let pred = pred_fn("_jarvan_ex.pred", |g, it, u| Ok(g.o(u).ctrl == ctrl(g, it)));
            g.queue_trigger(
                pid,
                "Jarvan IV, Exemplar of Demacia",
                res,
                Map::new(),
                None,
                true,
                trig_target(opts, pred, "target", false, true),
                cost,
            );
            Ok(())
        }),
        ..imp("Jarvan IV, Exemplar of Demacia")
    });

    // Demacian Standard — "[Deploy] When one or more friendly units attack here, give one of them +2 might this turn.
    // When you hold here, you may return this to its owner's hand."
    r.card(Impl {
        kw: crate::map! { Deploy = 1 },
        on_event: on_event_fn("_standard", |g, o, ev, info| {
            if ev == "attack" && g.o(info.g("obj").obj()).ctrl == g.o(o).ctrl && bf_is(g, o, info) && g.sd.is_some() {
                let key = skey(&[
                    "standard".into(),
                    g.o(o).uid.into(),
                    g.o(o).oid.into(),
                    g.turn_no.into(),
                    g.stats.get("combats").into(),
                ]);
                if g.stats.get(&key) != 0 {
                    return Ok(());
                }
                stat_set(g, &key, 1);
                let here = g.o(o).loc;
                let opts = opts_fn("_standard.opts", move |g, it| {
                    let c = ctrl(g, it);
                    let us: Vec<O> = g
                        .units(Some(c), None)
                        .into_iter()
                        .filter(|&u| g.o(u).loc == here && g.o(u).desig == Some(Desig::Att))
                        .collect();
                    sort_by(g, us, |g, u| Ok((-g.might(u)?, g.o(u).uid)))
                });
                let pred = pred_fn("_standard.pred", move |g, it, u| Ok(g.o(u).ctrl == ctrl(g, it) && g.o(u).loc == here));
                let res = res_fn("_standard.res", |g, it| mod_target(g, it, 2, 0).map(|_| ()));
                q(g, o, "Demacian Standard", res, Map::new(), false, trig_target(opts, pred, "target", false, true), Value::None);
            } else if ev == "hold" && info.g("pid").int() as usize == g.o(o).ctrl && bf_is(g, o, info) {
                q(g, o, "Demacian Standard (hold)", to_hand_me("_standard.hold"), Map::new(), true, Value::None, Value::None);
            }
            Ok(())
        }),
        ..imp("Demacian Standard")
    });
}

// ====================================================================== battlefields
fn circle_key(g: &Game, b: usize, pid: usize) -> String {
    skey(&["hunters_circle".into(), b.into(), pid.into(), g.turn_no.into()])
}

/// `_market_ab(b)`
fn market_ab(b: usize) -> Map {
    let key = move |g: &Game, pid: usize| skey(&["black_market".into(), b.into(), pid.into(), g.turn_no.into()]);
    ab(
        "Black Market: draw 1",
        Cost::from("2 runes of any type"),
        AbilityArgs {
            resolve: draw1("Black Market.resolve"),
            can: can_fn("Black Market.can", move |g, pid, _o| Ok(g.tp == pid && g.stats.get(&key(g, pid)) == 0)),
            extra_cost: extra_fn("Black Market.extra", move |g, pid, _o, _ch| {
                let k = key(g, pid);
                stat_set(g, &k, 1);
                Ok(())
            }),
            ..Default::default()
        },
    )
}

/// lieu d'une info rétrospective == battlefield b
fn inf_at(inf: &Map, b: usize) -> bool {
    inf.g("loc").opt_loc() == Some(Loc::Bf(b))
}

fn battlefields(r: &mut Registry) {
    // Bandle Scouts' Academy — "When you hide a card here, play a Gold gear token exhausted."
    r.card(Impl {
        bf_event: idx_event_fn("Bandle Scouts' Academy.bf_event", |g, b, ev, info| {
            if ev == "hide" && info.g("bf").opt_int() == Some(b as i64) {
                let p = info.g("pid").int() as usize;
                let res = res_fn("Bandle Scouts' Academy.res", |g, it| gold(g, ctrl(g, it), false).map(|_| ()));
                g.queue_trigger(p, "Bandle Scouts' Academy", res, Map::new(), None, false, Value::None, Value::None);
            }
            Ok(())
        }),
        ..imp("Bandle Scouts' Academy")
    });

    // Black Market — "[A][A]: Draw 1. Each player may use this ability once on their turn (even if they don't control
    // this battlefield)." (une capacité du battlefield que chaque joueur peut activer : donnée à la légende de chacun)
    r.card(Impl {
        hooks: mk(&[(
            "grant_abilities",
            Value::func("_market_grant", |g, a| {
                let b = a[0].int() as usize;
                let o = a[1].obj();
                Ok(if !zone_is(g, o, Zone::Legend) { Value::List(vec![]) } else { Value::List(vec![Value::Map(market_ab(b))]) })
            }),
        )]),
        ..imp("Black Market")
    });

    // Black Rose Sanctum — "For each player, the first time each turn a non-token unit they control dies here, they play
    // a 1 might Recruit unit token to their base."
    r.card(Impl {
        bf_event: idx_event_fn("_sanctum", |g, b, ev, info| {
            if ev == "die" {
                let inf = info.g("info").map().clone();
                if inf_at(&inf, b) && inf_type(&inf) == "Unit" && !inf.g("token").truthy() {
                    let c = inf.g("ctrl").int() as usize;
                    let key = skey(&["rose_sanctum".into(), b.into(), c.into(), g.turn_no.into()]);
                    if g.stats.get(&key) == 0 {
                        stat_set(g, &key, 1);
                        let res = res_fn("_sanctum.res", |g, it| make_token(g, "Recruit", ctrl(g, it), Loc::Base, false).map(|_| ()));
                        g.queue_trigger(c, "Black Rose Sanctum", res, Map::new(), None, false, Value::None, Value::None);
                    }
                }
            }
            Ok(())
        }),
        ..imp("Black Rose Sanctum")
    });

    // Cosmic Vista — "When you hold here, if your Rune Deck is empty, you score 1 point."
    r.card(Impl {
        bf_event: idx_event_fn("Cosmic Vista.bf_event", |g, b, ev, info| {
            if ev == "hold" && info.g("bf").opt_int() == Some(b as i64) {
                let p = info.g("pid").int() as usize;
                let res = res_fn("Cosmic Vista.res", |g, it| {
                    let c = ctrl(g, it);
                    if g.p[c].rune_deck.is_empty() {
                        g.gain_point(c, "Cosmic Vista")?;
                    }
                    Ok(())
                });
                g.queue_trigger(p, "Cosmic Vista", res, Map::new(), None, false, Value::None, Value::None);
            }
            Ok(())
        }),
        ..imp("Cosmic Vista")
    });

    // Durand Memorial — "When you defend here, choose a unit you control here. Give it [Tank] this combat."
    r.card(Impl {
        bf_event: idx_event_fn("_durand", |g, b, ev, info| {
            if ev == "defend" && info.g("bf").opt_int() == Some(b as i64) && g.sd.is_some() {
                let pid = g.o(info.g("obj").obj()).ctrl;
                let key = skey(&["durand".into(), b.into(), pid.into(), g.stats.get("combats").into(), g.turn_no.into()]);
                if g.stats.get(&key) != 0 {
                    return Ok(());
                }
                stat_set(g, &key, 1);
                let res = res_fn("_durand.res", |g, it| {
                    let Some(u) = g.legal(it, 0)? else { return Ok(()) };
                    let ent = ("Tank".to_string(), 1i64, Value::from("combat"));
                    g.o_mut(u).grants.push(ent);
                    let end = Value::func("_durand.end", |g, a| {
                        let eid = a[0].int() as u64;
                        let eff = g.eff(eid).cloned().unwrap_or_default();
                        g.remove_effect(eid);
                        if let Some(x) = g.obj(eff.g("uid").int() as u32) {
                            let ent = ("Tank".to_string(), 1i64, Value::from("combat"));
                            if let Some(i) = g.o(x).grants.iter().position(|e| *e == ent) {
                                g.o_mut(x).grants.remove(i);
                            }
                        }
                        Ok(Value::None)
                    });
                    let uid = g.o(u).uid;
                    g.add_effect(mk(&[("on", "combat_end".into()), ("fn", end), ("uid", uid.into())]));
                    Ok(())
                });
                let opts = opts_fn("_durand.opts", move |g, it| {
                    let c = ctrl(g, it);
                    let us: Vec<O> = g.units(Some(c), None).into_iter().filter(|&u| g.o(u).loc == Some(Loc::Bf(b))).collect();
                    sort_by(g, us, |g, u| Ok((-g.might(u)?, g.o(u).uid)))
                });
                let pred = pred_fn("_durand.pred", move |g, it, u| Ok(g.o(u).ctrl == ctrl(g, it) && g.o(u).loc == Some(Loc::Bf(b))));
                g.queue_trigger(pid, "Durand Memorial", res, Map::new(), None, false, trig_target(opts, pred, "target", false, true), Value::None);
            }
            Ok(())
        }),
        ..imp("Durand Memorial")
    });

    // Firelights' Hideout — "When you conquer here, the next unit you play this turn has [Accelerate]."
    r.card(Impl {
        bf_event: idx_event_fn("_hideout", |g, b, ev, info| {
            if ev == "conquer" && info.g("bf").opt_int() == Some(b as i64) {
                let pid = info.g("pid").int() as usize;
                let res = res_fn("_hideout.res", move |g, it| {
                    let key = Value::Tuple(vec!["hideout".into(), g.it(it).id.into()]);
                    let k2 = key.clone();
                    let fire = Value::func("_hideout.fire", move |g, a| {
                        let inf = ctx_of(&a[1]);
                        if inf.g("pid").int() as usize == pid && !inf.g("token").truthy() && ty(g, inf.g("card").obj()) == "Unit" {
                            g.effects.retain(|e| e.d.g("key") != k2);
                        }
                        Ok(Value::None)
                    });
                    let kwf = Value::func("_hideout.card_kw", |g, a| {
                        let c = a[1].obj();
                        Ok(Value::Bool(a[2].opt_str() == Some("Accelerate") && ty(g, c) == "Unit"))
                    });
                    g.add_effect(mk(&[
                        ("kind", "card_kw".into()),
                        ("pid", pid.into()),
                        ("key", key.clone()),
                        ("dur", turn()),
                        ("fn", kwf),
                    ]));
                    g.add_effect(mk(&[("on", "played".into()), ("fn", fire), ("key", key), ("dur", turn())]));
                    Ok(())
                });
                g.queue_trigger(pid, "Firelights' Hideout", res, Map::new(), None, false, Value::None, Value::None);
            }
            Ok(())
        }),
        ..imp("Firelights' Hideout")
    });

    // Hunters' Circle — "For each player, the first unit or gear they play here on their turn costs [1] less."
    r.card(Impl {
        hooks: mk(&[(
            "cost_aura",
            Value::func("_circle_aura", |g, a| {
                let (b, pid) = (a[0].int() as usize, a[1].int() as usize);
                let what = ctx_of(&a[2]);
                let ch = match what.g("choice") {
                    Value::Map(m) => m,
                    _ => Map::new(),
                };
                Ok(if matches!(what.g("kind").opt_str(), Some("unit") | Some("gear"))
                    && !what.g("card").is_none()
                    && g.tp == pid
                    && ch.g("loc").opt_loc() == Some(Loc::Bf(b))
                    && what.g("src").opt_str() != Some("facedown")
                    && g.stats.get(&circle_key(g, b, pid)) == 0
                {
                    maps_v(vec![crate::map! { e = -1 }])
                } else {
                    Value::List(vec![])
                })
            }),
        )]),
        bf_event: idx_event_fn("_circle_event", |g, b, ev, info| {
            if ev == "played" && info.g("pid").int() as usize == g.tp {
                let c = info.g("card").obj();
                if matches!(ty(g, c), "Unit" | "Gear") && zone_is(g, c, Zone::Board) && g.o(c).loc == Some(Loc::Bf(b)) {
                    let k = circle_key(g, b, info.g("pid").int() as usize);
                    stat_set(g, &k, 1);
                }
            }
            Ok(())
        }),
        ..imp("Hunters' Circle")
    });

    // Packed Amphitheater — "Champion units here have +1 might."
    r.card(Impl {
        aura_might: Value::func("Packed Amphitheater.aura_might", |g, a| {
            let (b, o) = (a[0].int() as usize, a[1].obj());
            Ok(Value::Int(
                if ty(g, o) == "Unit" && g.o(o).loc == Some(Loc::Bf(b)) && g.spec(o).super_ == "Champion" { 1 } else { 0 },
            ))
        }),
        ..imp("Packed Amphitheater")
    });

    // Rakelstake — "For each player, the first time they play a unit or gear here each turn, they [Predict]."
    r.card(Impl {
        bf_event: idx_event_fn("_rakel", |g, b, ev, info| {
            if ev == "played" {
                let c = info.g("card").obj();
                if matches!(ty(g, c), "Unit" | "Gear") && zone_is(g, c, Zone::Board) && g.o(c).loc == Some(Loc::Bf(b)) {
                    let p = info.g("pid").int() as usize;
                    let key = skey(&["rakelstake".into(), b.into(), p.into(), g.turn_no.into()]);
                    if g.stats.get(&key) == 0 {
                        stat_set(g, &key, 1);
                        g.queue_trigger(p, "Rakelstake", predict_res("Rakelstake.res", 1), Map::new(), None, false, Value::None, Value::None);
                    }
                }
            }
            Ok(())
        }),
        ..imp("Rakelstake")
    });

    // Ruined Monastery — "When you conquer here, you may discard 1 and channel 1 rune exhausted."
    r.card(Impl {
        bf_event: idx_event_fn("Ruined Monastery.bf_event", |g, b, ev, info| {
            if ev == "conquer" && info.g("bf").opt_int() == Some(b as i64) {
                let p = info.g("pid").int() as usize;
                let res = res_fn("Ruined Monastery.res", |g, it| {
                    let c = ctrl(g, it);
                    discard_n(g, c, 1)?;
                    g.channel(c, 1, true)?;
                    Ok(())
                });
                g.queue_trigger(p, "Ruined Monastery", res, Map::new(), None, true, Value::None, Value::None);
            }
            Ok(())
        }),
        ..imp("Ruined Monastery")
    });

    // Neeko, Blending In : non modélisée (plusieurs légendes dans la Legend Zone, choix à la construction du deck) :
    // NEEDS_radiance.md.
}

/// Enregistre les cartes du module, dans l'ordre de `radiance.py`.
pub fn register(r: &mut Registry) {
    tokens(r);
    fury(r);
    calm(r);
    mind(r);
    body(r);
    chaos(r);
    order(r);
    legends(r);
    battlefields(r);
    // Titres français des questions de ce module (train.py) ; les anglais dans lang.ASK_EN.
    r.ask_text(&[
        ("bomb_location", "Sur quel champ de bataille jouer le jeton Bomb ?"),
        ("bomb_target", "Bomb : quelle unité ennemie touche-t-elle ?"),
        ("tracker_targets", "Quelles unités (jusqu'à 2) subissent 1 dégât ?"),
        ("amp_kill", "Quelle autre carte alliée détruire pour piocher 1 ?"),
        ("morde_kill", "Quelle unité alliée détruire pour sauver Mordekaiser ?"),
        ("riven_mode", "Riven : quel effet choisir ?"),
        ("gear_pick", "Quel équipement révéler et piocher ?"),
        ("scope_guess", "Devine le type de la carte du dessus :"),
        ("innovator_mode", "Reckless Innovator : quel effet ?"),
        ("stranger_bf", "Quel champ de bataille choisir ?"),
        ("stranger_return", "Quelle unité ennemie renvoyer dans la main de son propriétaire ?"),
        ("wyrm_move", "Quelle unité déplacer vers le champ de bataille d'Angler Wyrm ?"),
        ("transpo_pick", "Quelle unité de ta main jouer gratuitement ?"),
        ("spirit_kill", "Quelle unité ou quel équipement détruire ?"),
        ("captain_pick", "Quelle carte révéler et piocher ?"),
        ("remember_pick", "Quelle carte de ta défausse jouer ?"),
        ("chrono_pick", "Quelle unité bannir ?"),
        ("chrono_order", "Quelle unité rejouer maintenant ?"),
        ("orianna_gear", "Quel équipement devient une unité ?"),
        ("ball_mode", "The Ball : quel effet ?"),
    ]);
    // Staunch Survivor — "[Assault 2], [Shield 2]" (seulement des mots-clés, mais la virgule la tient hors de
    // cards.auto_keywords)
    r.card(Impl { kw: crate::map! { Assault = 2, Shield = 2 }, ..imp("Staunch Survivor") });
}
