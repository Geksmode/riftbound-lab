//! Batch fury : les cartes Fury de `batches/fury.txt` (plus le jeton Shadow Clone, règle 187.11). Traduction ligne à
//! ligne de `riftbound/engine/cardsets/fury.py`. Numéros de règle : core rules 2026-07-16.
use std::collections::BTreeSet;
use std::sync::OnceLock;

use crate::actions::{attach, deflect_reqs, full_choices, pay_ctx, play_card, spell_variants, total_cost};
use crate::cards::{Impl, Registry};
use crate::cards_helpers::{
    ability, add_ability, all_units, cap, enemies, flow_cost, friends, impl_empower, impl_equip, make_token,
    equip_ability_cost, may_pay, p_enemy, p_enemy_v, p_friend, p_friend_bf_v, p_friend_v, p_gear_v, p_unit, p_unit_bf_v,
    p_unit_v, play_unit_free, repeat_cost, repeatable, tg_choices, token_locations, trig_target, unit_play_choices,
    value, when_empowered, AbilityArgs, Cost,
};
use crate::game::{Desig, Game, ItemId, Res, Zone, O};
use crate::game_costs::{any_req, reqs_to_value, Req};
use crate::map;
use crate::spec::spec;
use crate::value::{is_bf, Loc, Map, Value};

const MODULE: &str = "cardsets.fury";

// ====================================================================== petites aides privées de traduction
fn mk(pairs: Vec<(&str, Value)>) -> Map {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.set(k, v);
    }
    m
}

fn turn() -> Value {
    Value::from("turn")
}

fn ctrl(g: &Game, it: ItemId) -> usize {
    g.it(it).ctrl
}

fn ov(v: &[O]) -> Vec<Value> {
    v.iter().map(|&o| Value::Obj(o)).collect()
}

fn maps_v(v: Vec<Map>) -> Value {
    Value::List(v.into_iter().map(Value::Map).collect())
}

fn maps_of(v: &Value) -> Vec<Map> {
    v.list().iter().map(|x| x.map().clone()).collect()
}

fn tgs(g: &Game, us: &[O]) -> Value {
    maps_v(tg_choices(g, us))
}

fn uid(g: &Game, o: O) -> u32 {
    g.o(o).uid
}

fn is_o(v: &Value, o: O) -> bool {
    v.opt_obj() == Some(o)
}

fn has_o(v: &Value, o: O) -> bool {
    v.list().iter().any(|x| x.opt_obj() == Some(o))
}

fn hbv(ctx: &Value) -> Option<Loc> {
    ctx.map().g("hidden_bf").opt_loc()
}

fn is_unit(g: &Game, o: O) -> bool {
    g.spec(o).type_ == "Unit"
}

fn spec_e(g: &Game, o: O) -> i64 {
    g.spec(o).e as i64
}

fn bonus_of(g: &Game, o: O) -> i64 {
    g.equip_bonus_of(g.cname(o))
}

fn is_equipment(g: &Game, o: O) -> bool {
    g.spec(o).tags.contains("Equipment")
}

fn fury_req() -> Req {
    std::iter::once("Fury".to_string()).collect()
}

/// `[FURY]`
fn fury() -> Vec<Req> {
    vec![fury_req()]
}

/// tuple (e, reqs) rendu par une fonction de coût
fn cost_v(e: i64, reqs: &[Req]) -> Value {
    Value::Tuple(vec![e.into(), reqs_to_value(reqs)])
}

/// `sorted(xs, key=...)` : clés calculées d'abord, tri stable.
fn sorted_by<K: PartialOrd, T>(mut v: Vec<(K, T)>) -> Vec<T> {
    v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    v.into_iter().map(|x| x.1).collect()
}

/// `queue_trigger(pid, name, fn, data, src=src)`
fn qt(g: &mut Game, pid: usize, name: &str, f: Value, data: Map, src: Option<u32>) -> ItemId {
    g.queue_trigger(pid, name, f, data, src, false, Value::None, Value::None)
}

/// `_me(g, it)`
fn me(g: &Game, it: ItemId) -> Option<O> {
    g.it(it).src.and_then(|u| g.obj(u))
}

/// `key in g.stats` (Counter)
fn stats_has(g: &Game, key: &str) -> bool {
    g.stats.0.iter().any(|(k, _)| k == key)
}

/// `_cval(c)`
fn cval(g: &Game, c: O) -> i64 {
    let s = g.spec(c);
    s.e as i64 + 2 * s.p as i64
}

/// `itertools.combinations(range(n), k)` (ordre lexicographique)
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = vec![];
    if k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.clone());
        let mut i = k;
        while i > 0 && idx[i - 1] == i - 1 + n - k {
            i -= 1;
        }
        if i == 0 {
            return out;
        }
        let i = i - 1;
        idx[i] += 1;
        for j in i + 1..k {
            idx[j] = idx[j - 1] + 1;
        }
    }
}

// ====================================================================== private helpers
/// `_hand_others(g, pid, card)`
fn hand_others(g: &Game, pid: usize, card: O) -> Vec<O> {
    g.p[pid].hand.iter().copied().filter(|&c| c != card).collect()
}

/// `_discard_one(g, pid, kind, exclude, order)` : Discard 1 card of pid's choice (rule 422). Returns the discarded card
/// or None. order(g, c) -> (clé, uid)
fn discard_one(
    g: &mut Game,
    pid: usize,
    kind: &str,
    exclude: Option<O>,
    order: Option<fn(&Game, O) -> (i64, u32)>,
) -> Res<Option<O>> {
    let mut cs: Vec<O> = g.p[pid].hand.iter().copied().filter(|&c| Some(c) != exclude).collect();
    if cs.is_empty() {
        return Ok(None);
    }
    if let Some(f) = order {
        let keyed: Vec<((i64, u32), O)> = cs.iter().map(|&c| (f(g, c), c)).collect();
        cs = sorted_by(keyed);
    }
    let c = g.ask(pid, kind, ov(&cs), Map::new())?.obj();
    g.discard(pid, c)?;
    Ok(Some(c))
}

fn order_e_desc(g: &Game, c: O) -> (i64, u32) {
    (-spec_e(g, c), g.o(c).uid)
}

fn order_cval(g: &Game, c: O) -> (i64, u32) {
    (cval(g, c), g.o(c).uid)
}

/// `_discard_n(n)`
fn discard_n(n: usize) -> Value {
    Value::func("_discard_n", move |g, a| {
        let it = a[0].item();
        for _ in 0..n {
            let c = ctrl(g, it);
            discard_one(g, c, "discard", None, None)?;
        }
        Ok(Value::None)
    })
}

/// `_killable(g, u, n)`
fn killable(g: &mut Game, u: O, n: i64) -> Res<bool> {
    Ok(g.might(u)? - g.o(u).damage + g.o(u).prevent <= n)
}

/// `_dmg_targets(g, pid, n, at_bf, hb, enemies_only)` : Units a 'deal n' effect may choose, best first for the AI:
/// enemies it kills, other enemies, then friends.
fn dmg_targets(g: &mut Game, pid: usize, n: i64, at_bf: bool, hb: Option<Loc>, enemies_only: bool) -> Res<Vec<O>> {
    let mut us = vec![];
    for u in g.units(None, None) {
        if (!at_bf || is_bf(g.o(u).loc))
            && g.targetable(u, Some(pid))?
            && (hb.is_none() || g.o(u).loc == hb)
            && (!enemies_only || g.o(u).ctrl != pid)
        {
            us.push(u);
        }
    }
    let mut keyed = vec![];
    for u in us {
        let k = (g.o(u).ctrl == pid, !killable(g, u, n)?, -value(g, u)?, uid(g, u));
        keyed.push((k, u));
    }
    Ok(sorted_by(keyed))
}

/// `_buff_targets(g, pid, at_bf, hb)` : Units a beneficial effect may choose, best first for the AI: friends (at
/// battlefields first), then enemies.
fn buff_targets(g: &mut Game, pid: usize, at_bf: bool, hb: Option<Loc>) -> Res<Vec<O>> {
    let mut us = vec![];
    for u in g.units(None, None) {
        if (!at_bf || is_bf(g.o(u).loc)) && g.targetable(u, Some(pid))? && (hb.is_none() || g.o(u).loc == hb) {
            us.push(u);
        }
    }
    let mut keyed = vec![];
    for u in us {
        let k = (g.o(u).ctrl != pid, !is_bf(g.o(u).loc), -value(g, u)?, uid(g, u));
        keyed.push((k, u));
    }
    Ok(sorted_by(keyed))
}

/// `_token(g, pid, name, ready, kind)` : 'Play a ... unit token' without a location: the player picks a legal location
/// (base first).
fn token(g: &mut Game, pid: usize, name: &str, ready: bool, kind: &str) -> Res<Option<O>> {
    let ls = token_locations(g, pid, name, None)?;
    if ls.is_empty() {
        return Ok(None);
    }
    let loc = g.ask(pid, kind, ls.iter().map(|&l| Value::Loc(l)).collect(), Map::new())?.loc();
    make_token(g, name, pid, loc, ready)
}

/// `_gold(g, pid)` : 'Play a Gold gear token exhausted.' (gear is played to base, rule 149.2)
fn gold(g: &mut Game, pid: usize) -> Res<Option<O>> {
    make_token(g, "Gold", pid, Loc::Base, false)
}

/// `_detach(g, gear)` : Detach an Equipment (rule 434): it stays where it is; cleanup recalls unattached gear at
/// battlefields.
fn detach(g: &mut Game, gear: O) {
    let u = g.o(gear).attached_to.and_then(|x| g.obj(x));
    let gu = g.o(gear).uid;
    if let Some(u) = u {
        if let Some(i) = g.o(u).attached.iter().position(|&x| x == gu) {
            g.o_mut(u).attached.remove(i);
        }
    }
    g.o_mut(gear).attached_to = None;
    g.need_cleanup = true;
}

type Cand = (Map, i64, Vec<Req>);

/// `_play_choices(g, pid, card, src, free, discount_e, extra_reqs, locs, extra_ch)` : Legal (choice, energy, reqs) to
/// play card as a limited play (rule 419.3): ignoring its cost (free) or for its cost reduced by discount_e energy,
/// plus extra_reqs (alternative cost such as 'for 1 rune of any type'). Mandatory additional costs (Deflect...) still
/// apply.
#[allow(clippy::too_many_arguments)]
fn play_choices(
    g: &mut Game,
    pid: usize,
    card: O,
    src: &str,
    free: bool,
    discount_e: i64,
    extra_reqs: &[Req],
    locs: Option<Vec<Loc>>,
    extra_ch: Option<&Map>,
) -> Res<Vec<Cand>> {
    let Some(im) = g.impl_o(card) else { return Ok(vec![]) };
    let typ: &str = &g.spec(card).type_;
    let base: Vec<Map> = if typ == "Unit" {
        unit_play_choices(g, pid, card, src, locs)?
    } else if typ == "Gear" {
        vec![map! { loc = Loc::Base }]
    } else if typ == "Spell" {
        let ctx = map! { hidden_bf = Value::None, card = card, src = src };
        let b = if full_choices(g, Some(pid)) && !im.all_choices.is_none() {
            let r = im.all_choices.call(g, &[pid.into(), ctx.clone().into()])?;
            if r.truthy() {
                maps_of(&r)
            } else {
                vec![Map::new()]
            }
        } else if im.choices.truthy() {
            maps_of(&im.choices.call(g, &[pid.into(), ctx.clone().into()])?)
        } else {
            vec![Map::new()]
        };
        let b = cap(g, b, 8, Some(pid));
        spell_variants(g, pid, card, im, b, &ctx, false)?
    } else {
        return Ok(vec![]);
    };
    let mut out = vec![];
    for ch in base {
        let mut ch = match extra_ch {
            Some(x) => ch.merged(x),
            None => ch,
        };
        if free {
            ch.set("free", true);
        }
        let (e, reqs) = total_cost(g, pid, card, &ch, src)?;
        let e = 0.max(e - discount_e);
        let mut reqs = reqs;
        reqs.extend(extra_reqs.iter().cloned());
        let pc = pay_ctx(g, card);
        if g.can_pay(pid, e, &reqs, &pc)? {
            out.push((ch, e, reqs));
        }
    }
    Ok(out)
}

/// `_play_from(g, pid, card, src, cands)` : Pay and play one of the candidates of _play_choices (asked to pid).
fn play_from(g: &mut Game, pid: usize, card: O, src: &str, cands: Vec<Cand>) -> Res<Value> {
    if cands.is_empty() {
        return Ok(Value::None);
    }
    let opts: Vec<Value> = cands.iter().map(|c| Value::Map(c.0.clone())).collect();
    let ch = g.ask(pid, "play_choice", opts, map! { card = card })?;
    let ch = ch.map().clone();
    let (_, e, reqs) = cands.into_iter().find(|c| c.0 == ch).unwrap();
    let pc = pay_ctx(g, card);
    if !g.pay(pid, e, &reqs, &pc)? {
        return Ok(Value::None);
    }
    play_card(g, pid, card, src, ch, true)
}

/// `_replay_offer(g, it, pid, kind, extra_ch)` : Offer made by a resolving spell: once its card has left the chain
/// (its 'played' event, rule 419.4.a), pid may play it again from the trash for 1 rune of any type (Dancing Grenade,
/// Death from Below).
fn replay_offer(g: &mut Game, it: ItemId, pid: usize, kind: &str, extra_ch: Map) {
    let iid = g.it(it).id;
    let cu = g.o(g.it(it).card.unwrap()).uid;
    g.add_effect(mk(vec![
        ("on", "played".into()),
        ("fn", replay_fire()),
        ("item", iid.into()),
        ("card", cu.into()),
        ("pid", pid.into()),
        ("kind", kind.into()),
        ("extra", extra_ch.into()),
        ("dur", turn()),
    ]));
}

/// `_replay_fire(g, eff, info)`
fn replay_fire() -> Value {
    Value::func("_replay_fire", |g, a| {
        let eid = a[0].int() as u64;
        let info = a[1].map();
        let eff = g.eff(eid).cloned().unwrap();
        let item = info.g("item");
        if item.is_none() || g.it(item.item()).id as i64 != eff.g("item").int() {
            return Ok(Value::None);
        }
        g.remove_effect(eid);
        let c = info.g("card").obj();
        if g.o(c).uid as i64 != eff.g("card").int() || g.o(c).zone != Some(Zone::Trash) {
            return Ok(Value::None);
        }
        let pid = eff.g("pid").int() as usize;
        let extra = eff.g("extra").map().clone();
        let cands = play_choices(g, pid, c, "trash", true, 0, &[any_req()], None, Some(&extra))?;
        if cands.is_empty()
            || !g.ask(pid, "may", vec![true.into(), false.into()], map! { reason = eff.g("kind") })?.truthy()
        {
            return Ok(Value::None);
        }
        play_from(g, pid, c, "trash", cands)?;
        Ok(Value::None)
    })
}

// ====================================================================== tokens
/// `_clone_event` : Shadow Clone
fn clone_event() -> Value {
    Value::func("_clone_event", |g, a| {
        let o = a[0].obj();
        if a[1].str() == "attack" && is_o(&a[2].map().g("obj"), o) {
            let res = Value::func("res", |g_, a| {
                let it = a[0].item();
                let me = me(g_, it);
                let c = ctrl(g_, it);
                let us: Vec<O> = g_.p[c].trash.iter().copied().filter(|&x| is_unit(g_, x)).collect();
                let Some(me) = me else { return Ok(Value::None) };
                if us.is_empty() {
                    return Ok(Value::None);
                }
                let mut opts = ov(&us);
                opts.push(Value::None);
                let Some(x) = g_.ask(c, "clone_banish", opts, Map::new())?.opt_obj() else { return Ok(Value::None) };
                g_.to_zone(x, "banish", false, None)?;
                g_.grant(me, "Assault", 4, turn());
                Ok(Value::None)
            });
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            qt(g, c, "Shadow Clone", res, Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

// ====================================================================== spells
/// `_angle_choices(g, pid, ctx)`
fn angle_choices() -> Value {
    Value::func("_angle_choices", |g, a| {
        let pid = a[0].int() as usize;
        let (mut good, mut bad) = (vec![], vec![]);
        for side in [pid, 1 - pid] {
            let eqs: Vec<O> = g.gear(Some(side)).into_iter().filter(|&x| is_equipment(g, x)).collect();
            let eqs = sorted_by(eqs.into_iter().map(|x| ((-bonus_of(g, x), uid(g, x)), x)).collect());
            let mut us0 = vec![];
            for u in g.units(Some(side), None) {
                if g.targetable(u, Some(pid))? {
                    us0.push(u);
                }
            }
            let mut keyed = vec![];
            for u in us0 {
                keyed.push(((!is_bf(g.o(u).loc), -value(g, u)?, uid(g, u)), u));
            }
            let us = sorted_by(keyed);
            for &x in &eqs {
                for u in cap(g, us.clone(), 3, Some(pid)) {
                    let ch = map! { tg = Value::Tuple(vec![uid(g, u).into(), uid(g, x).into()]) };
                    if g.o(x).attached_to == Some(uid(g, u)) {
                        if side != pid { good.push(ch) } else { bad.push(ch) }
                    } else if side == pid {
                        good.push(ch)
                    } else {
                        bad.push(ch)
                    }
                }
            }
        }
        good.extend(bad);
        Ok(maps_v(good))
    })
}

/// `_angle(g, it)`
fn angle() -> Value {
    Value::func("_angle", |g, a| {
        let it = a[0].item();
        let (u, x) = (g.legal(it, 0)?, g.legal(it, 1)?);
        if let (Some(u), Some(x)) = (u, x) {
            if g.o(u).ctrl == g.o(x).ctrl && is_equipment(g, x) {
                if g.o(x).attached_to == Some(uid(g, u)) {
                    detach(g, x);
                } else {
                    attach(g, x, u)?;
                }
            }
        }
        let c = ctrl(g, it);
        g.draw(c, 1)?;
        Ok(Value::None)
    })
}

/// `_blind_fury(g, it)`
fn blind_fury() -> Value {
    Value::func("_blind_fury", |g, a| {
        let it = a[0].item();
        let pid = ctrl(g, it);
        let opp = 1 - pid;
        let top = g.reveal(opp, 1, None, true, None)?;
        if top.is_empty() {
            return Ok(Value::None); // revealing more than the deck: no burn out (rule 431.1.c)
        }
        let c = top[0];
        g.to_zone(c, "banish", false, Some(pid))?;
        let cands = play_choices(g, pid, c, "banish", true, 0, &[], None, None)?;
        play_from(g, pid, c, "banish", cands)?;
        Ok(Value::None)
    })
}

/// `_blood_rush(g, it)`
fn blood_rush() -> Value {
    Value::func("_blood_rush", |g, a| {
        if let Some(u) = g.legal(a[0].item(), 0)? {
            g.grant(u, "Assault", 2, Value::None);
        }
        Ok(Value::None)
    })
}

/// choices=lambda g, pid, ctx: tg_choices(_buff_targets(g, pid, False, ctx["hidden_bf"]))
fn buff_choices() -> Value {
    Value::func("buff_choices", |g, a| {
        let pid = a[0].int() as usize;
        let us = buff_targets(g, pid, false, hbv(&a[1]))?;
        Ok(tgs(g, &us))
    })
}

/// choices=lambda g, pid, ctx: tg_choices(_dmg_targets(g, pid, n, at_bf, ctx["hidden_bf"]))
fn dmg_choices(n: i64, at_bf: bool) -> Value {
    Value::func("dmg_choices", move |g, a| {
        let pid = a[0].int() as usize;
        let us = dmg_targets(g, pid, n, at_bf, hbv(&a[1]), false)?;
        Ok(tgs(g, &us))
    })
}

/// `_curse_n(g, pid)`
fn curse_n(g: &Game, pid: usize) -> i64 {
    2 + g.p[pid].trash.iter().filter(|&&c| g.cname(c) == "Consuming Curse").count() as i64
}

/// `_curse(g, it)`
fn curse() -> Value {
    Value::func("_curse", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            let c = ctrl(g, it);
            let n = curse_n(g, c);
            g.deal(Some(u), n, "spell", Some(c), None)?;
        }
        Ok(Value::None)
    })
}

/// `_CC_REP`
fn cc_rep(i: usize) -> (i64, Vec<Req>) {
    match i {
        0 => (1, vec![]),
        1 => (0, vec![any_req()]),
        _ => (1, vec![any_req()]),
    }
}

/// `_CC_TARGETED`
const CC_TARGETED: [&str; 3] = ["d2", "d3", "m4"];

/// `_cc_choices(g, pid, ctx)`
fn cc_choices() -> Value {
    Value::func("_cc_choices", |g, a| {
        let pid = a[0].int() as usize;
        let card = a[1].map().g("card").obj();
        let es_bf = enemies(g, pid, true, None, true)?;
        let es_base: Vec<O> =
            enemies(g, pid, false, None, true)?.into_iter().filter(|&u| g.o(u).loc == Some(Loc::Base)).collect();
        let mut best: Vec<(&str, O)> = vec![];
        if !es_bf.is_empty() {
            let mut keyed = vec![];
            for &u in &es_bf {
                keyed.push(((!killable(g, u, 2)?, -value(g, u)?, uid(g, u)), u));
            }
            best.push(("d2", sorted_by(keyed)[0]));
            let mut keyed = vec![];
            for &u in &es_bf {
                keyed.push(((-value(g, u)?, uid(g, u)), u));
            }
            best.push(("m4", sorted_by(keyed)[0]));
        }
        if !es_base.is_empty() {
            let mut keyed = vec![];
            for &u in &es_base {
                keyed.push(((!killable(g, u, 3)?, -value(g, u)?, uid(g, u)), u));
            }
            best.push(("d3", sorted_by(keyed)[0]));
        }
        let bget = |m: &str| best.iter().find(|x| x.0 == m).map(|x| x.1);
        // -4 might first: helps damage
        let modes: Vec<&str> = ["m4", "d2", "d3", "draw"].into_iter().filter(|&m| m == "draw" || bget(m).is_some()).collect();
        let mut out: Vec<((f64, usize), Map)> = vec![];
        for k in (1..=modes.len()).rev() {
            for idx in combinations(modes.len(), k) {
                let combo: Vec<&str> = idx.iter().map(|&i| modes[i]).collect();
                let tg: Vec<Value> = combo
                    .iter()
                    .filter(|m| CC_TARGETED.contains(m))
                    .map(|m| Value::from(uid(g, bget(m).unwrap())))
                    .collect();
                let mut rcs: Vec<Vec<i64>> = vec![(0..(k as i64 - 1)).collect()];
                if k == 2 {
                    rcs.push(vec![1]);
                }
                for rc in rcs {
                    let ch = map! {
                        cc = Value::Tuple(combo.iter().map(|&m| Value::from(m)).collect()),
                        tg = Value::Tuple(tg.clone()),
                        rc = Value::Tuple(rc.iter().map(|&x| Value::from(x)).collect()),
                    };
                    let (e, reqs) = total_cost(g, pid, card, &ch, "hand")?;
                    let pc = pay_ctx(g, card);
                    if !g.can_pay(pid, e, &reqs, &pc)? {
                        continue;
                    }
                    let mut sc: f64 = 0.0;
                    for &m in &combo {
                        if m == "draw" {
                            sc += 1.5;
                        } else if m == "m4" {
                            sc += 1.0 + if combo.contains(&"d2") && bget("d2") == bget("m4") { 2.0 } else { 0.0 };
                        } else {
                            let u = bget(m).unwrap();
                            sc += if killable(g, u, if m == "d2" { 2 } else { 3 })? { value(g, u)? } else { 1.0 };
                        }
                    }
                    let n = out.len();
                    out.push(((-(sc - 0.7 * rc.len() as f64), n), ch));
                    break;
                }
            }
        }
        Ok(maps_v(sorted_by(out)))
    })
}

/// `_curtain_call(g, it)`
fn curtain_call() -> Value {
    Value::func("_curtain_call", |g, a| {
        let it = a[0].item();
        let mut i = 0;
        let cc: Vec<Value> = g.it(it).data.g("cc").list().to_vec();
        for m in cc {
            let m = m.str();
            let c = ctrl(g, it);
            if m == "draw" {
                g.draw(c, 1)?;
                continue;
            }
            let u = g.legal(it, i)?;
            i += 1;
            let Some(u) = u else { continue };
            let loc = g.o(u).loc;
            if m == "d2" && is_bf(loc) {
                g.deal(Some(u), 2, "spell", Some(c), None)?;
            } else if m == "d3" && loc == Some(Loc::Base) {
                g.deal(Some(u), 3, "spell", Some(c), None)?;
            } else if m == "m4" && is_bf(loc) {
                g.mod_(u, -4, turn(), None)?;
            }
        }
        Ok(Value::None)
    })
}

/// `_grenade(g, it)` : Dancing Grenade
fn grenade() -> Value {
    Value::func("_grenade", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
        let key = format!("('dancing_grenade', {}, {})", g.o(g.it(it).card.unwrap()).uid, g.turn_no);
        let n = 2 + if g.it(it).data.g("dg_again").truthy() { g.stats.get(&key) } else { 0 };
        let c = ctrl(g, it);
        if g.deal(Some(u), n, "spell", Some(c), None)? > 0 {
            g.stats.add(&key, 1);
        }
        if g.on_board(u) {
            let uc = g.o(u).ctrl;
            replay_offer(g, it, uc, "dancing_grenade", map! { dg_again = true });
        }
        Ok(Value::None)
    })
}

/// `_danger_zone(g, it)`
fn danger_zone() -> Value {
    Value::func("_danger_zone", |g, a| {
        let c = ctrl(g, a[0].item());
        for u in g.units(Some(c), None) {
            if g.tags(u)?.contains("Mech") {
                g.mod_(u, 1, turn(), None)?;
            }
        }
        Ok(Value::None)
    })
}

/// `_death_mark(g, it)`
fn death_mark() -> Value {
    Value::func("_death_mark", |g, a| {
        let c = ctrl(g, a[0].item());
        g.burn(c, 3)?;
        if g.winner.is_none() {
            token(g, c, "Shadow Clone", false, "token_location")?;
        }
        Ok(Value::None)
    })
}

/// `_dfb(g, it)` : Death from Below
fn dfb() -> Value {
    Value::func("_dfb", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
        let m = g.might(u)?;
        let c = ctrl(g, it);
        g.kill(&[u], Some(c), false)?;
        if m <= 3 {
            replay_offer(g, it, c, "death_from_below", Map::new());
        }
        Ok(Value::None)
    })
}

/// `_detonate(g, it)`
fn detonate() -> Value {
    Value::func("_detonate", |g, a| {
        let it = a[0].item();
        let Some(x) = g.legal(it, 0)? else { return Ok(Value::None) };
        let c = g.o(x).ctrl;
        let k = ctrl(g, it);
        g.kill(&[x], Some(k), false)?;
        g.draw(c, 2)?;
        Ok(Value::None)
    })
}

/// `_disintegrate(g, it)`
fn disintegrate() -> Value {
    Value::func("_disintegrate", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
        let c = ctrl(g, it);
        g.deal(Some(u), 3, "spell", Some(c), None)?;
        if g.on_board(u) && g.lethal(u)? {
            let uu = uid(g, u);
            g.add_effect(mk(vec![
                ("on", "die".into()),
                ("fn", disintegrate_die()),
                ("uid", uu.into()),
                ("pid", c.into()),
                ("dur", turn()),
            ]));
        }
        Ok(Value::None)
    })
}

/// `_disintegrate_die(g, eff, info)`
fn disintegrate_die() -> Value {
    Value::func("_disintegrate_die", |g, a| {
        let eid = a[0].int() as u64;
        let eff = g.eff(eid).cloned().unwrap();
        let o = a[1].map().g("info").map().g("obj").obj();
        if g.o(o).uid as i64 == eff.g("uid").int() {
            g.remove_effect(eid);
            g.draw(eff.g("pid").int() as usize, 1)?;
        }
        Ok(Value::None)
    })
}

/// `_firestorm(g, it)`
fn firestorm() -> Value {
    Value::func("_firestorm", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        let bf = g.it(it).data.g("bf").int() as usize;
        for u in g.units(Some(1 - c), Some(Loc::Bf(bf))) {
            g.deal(Some(u), 3, "spell", Some(c), None)?;
        }
        Ok(Value::None)
    })
}

fn firestorm_choices() -> Value {
    Value::func("firestorm_choices", |g, a| {
        let pid = a[0].int() as usize;
        let mut cs = vec![];
        for b in 0..g.bfs.len() {
            if !g.units(Some(1 - pid), Some(Loc::Bf(b))).is_empty() {
                cs.push(b);
            }
        }
        let mut keyed = vec![];
        for b in cs {
            let mut s = 0.0;
            for u in g.units(Some(1 - pid), Some(Loc::Bf(b))) {
                s += value(g, u)?;
            }
            keyed.push(((-s, b), map! { bf = b }));
        }
        Ok(maps_v(sorted_by(keyed)))
    })
}

/// `_excited(g, it)` : Get Excited!
fn excited() -> Value {
    Value::func("_excited", |g, a| {
        let it = a[0].item();
        let c0 = ctrl(g, it);
        let c = discard_one(g, c0, "excited_discard", None, Some(order_e_desc))?;
        let u = g.legal(it, 0)?;
        if let (Some(c), Some(u)) = (c, u) {
            if spec_e(g, c) > 0 {
                let e = spec_e(g, c);
                g.deal(Some(u), e, "spell", Some(c0), None)?;
            }
        }
        Ok(Value::None)
    })
}

fn excited_choices() -> Value {
    Value::func("excited_choices", |g, a| {
        let pid = a[0].int() as usize;
        let card = a[1].map().g("card").opt_obj();
        let n = g.p[pid].hand.iter().filter(|&&c| Some(c) != card).map(|&c| spec_e(g, c)).chain([0]).max().unwrap();
        let us = dmg_targets(g, pid, n, true, hbv(&a[1]), false)?;
        Ok(tgs(g, &us))
    })
}

/// `_deal_bf(n, draw)` : Hextech Ray / Incinerate / Void Seeker
fn deal_bf(n: i64, draw: i64) -> Value {
    Value::func("_deal_bf", move |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        if let Some(u) = g.legal(it, 0)? {
            g.deal(Some(u), n, "spell", Some(c), None)?;
        }
        if draw != 0 {
            g.draw(c, draw)?;
        }
        Ok(Value::None)
    })
}

/// `_rain_choices(g, pid, ctx)` : Icathian Rain
fn rain_choices() -> Value {
    Value::func("_rain_choices", |g, a| {
        let pid = a[0].int() as usize;
        let es = enemies(g, pid, false, None, true)?;
        if es.is_empty() {
            let us = dmg_targets(g, pid, 2, false, None, false)?;
            return Ok(if us.is_empty() {
                Value::List(vec![])
            } else {
                maps_v(vec![map! { tg = Value::Tuple(vec![Value::from(uid(g, us[0])); 6]) }])
            });
        }
        fn greedy(g: &mut Game, cands: &[O]) -> Res<Vec<u32>> {
            let mut tg: Vec<u32> = vec![];
            for &u in cands {
                let x = g.might(u)? - g.o(u).damage + g.o(u).prevent;
                let need = 1.max(-((-x).div_euclid(2))) as usize;
                if tg.len() + need > 6 {
                    continue;
                }
                tg.extend(std::iter::repeat_n(g.o(u).uid, need));
            }
            while tg.len() < 6 {
                let x = match tg.last() {
                    Some(&l) => l,
                    None => g.o(cands[0]).uid,
                };
                tg.push(x);
            }
            tg.truncate(6);
            Ok(tg)
        }
        let mut keyed = vec![];
        for &u in &es {
            keyed.push(((g.might(u)? - g.o(u).damage, uid(g, u)), u));
        }
        let es2 = sorted_by(keyed);
        let out = vec![greedy(g, &es)?, greedy(g, &es2)?, vec![uid(g, es[0]); 6]];
        let mut res: Vec<Map> = vec![];
        for t in out {
            let ch = map! { tg = Value::Tuple(t.into_iter().map(Value::from).collect()) };
            if !res.contains(&ch) {
                res.push(ch);
            }
        }
        Ok(maps_v(res))
    })
}

/// `_rain(g, it)`
fn rain() -> Value {
    Value::func("_rain", |g, a| {
        let it = a[0].item();
        for i in 0..6 {
            if let Some(u) = g.legal(it, i)? {
                let c = ctrl(g, it);
                g.deal(Some(u), 2, "spell", Some(c), None)?;
            }
        }
        Ok(Value::None)
    })
}

/// `_facedown(g, pid)`
fn facedown(g: &Game, pid: usize) -> bool {
    g.bfs.iter().any(|b| b.facedowns.iter().any(|&c| g.o(c).owner == pid))
}

/// `_harpoon(g, it)`
fn harpoon() -> Value {
    Value::func("_harpoon", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            let c = ctrl(g, it);
            let n = if facedown(g, c) { 4 } else { 2 };
            g.deal(Some(u), n, "spell", Some(c), None)?;
        }
        Ok(Value::None)
    })
}

fn harpoon_choices() -> Value {
    Value::func("harpoon_choices", |g, a| {
        let pid = a[0].int() as usize;
        let n = if facedown(g, pid) { 4 } else { 2 };
        let us = dmg_targets(g, pid, n, true, hbv(&a[1]), false)?;
        Ok(tgs(g, &us))
    })
}

/// `_guillotine(g, it)` : Noxian Guillotine
fn guillotine() -> Value {
    Value::func("_guillotine", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
        let c = ctrl(g, it);
        if g.legion(c, g.it(it).card) {
            g.kill(&[u], Some(c), false)?;
        } else {
            let (uu, oid) = (uid(g, u), g.o(u).oid);
            g.add_effect(mk(vec![
                ("on", "damaged".into()),
                ("fn", guillotine_fire()),
                ("uid", uu.into()),
                ("oid", oid.into()),
                ("pid", c.into()),
                ("dur", turn()),
            ]));
        }
        Ok(Value::None)
    })
}

/// `_guillotine_kill(g, it)`
fn guillotine_kill() -> Value {
    Value::func("_guillotine_kill", |g, a| {
        let it = a[0].item();
        let d = g.it(it).data.clone();
        if let Some(x) = g.obj(d.g("uid").int() as u32) {
            if g.o(x).oid == d.g("oid").int() {
                let c = ctrl(g, it);
                g.kill(&[x], Some(c), false)?;
            }
        }
        Ok(Value::None)
    })
}

/// `_guillotine_fire(g, eff, info)`
fn guillotine_fire() -> Value {
    Value::func("_guillotine_fire", |g, a| {
        let eid = a[0].int() as u64;
        let eff = g.eff(eid).cloned().unwrap();
        let o = a[1].map().g("obj").obj();
        if g.o(o).uid as i64 == eff.g("uid").int() && g.o(o).oid == eff.g("oid").int() {
            g.remove_effect(eid);
            let (uu, oid) = (uid(g, o), g.o(o).oid);
            qt(g, eff.g("pid").int() as usize, "Noxian Guillotine", guillotine_kill(), map! { uid = uu, oid = oid }, None);
        }
        Ok(Value::None)
    })
}

fn enemies_choices() -> Value {
    Value::func("enemies_choices", |g, a| {
        let pid = a[0].int() as usize;
        let us = enemies(g, pid, false, hbv(&a[1]), true)?;
        Ok(tgs(g, &us))
    })
}

/// sorted(friends(g, pid), key=lambda u: (not u.exhausted, -value(g, u), u.uid)) + enemies(g, pid)
fn ready_choices() -> Value {
    Value::func("ready_choices", |g, a| {
        let pid = a[0].int() as usize;
        let mut keyed = vec![];
        for u in friends(g, pid, false, None)? {
            keyed.push(((!g.exhausted(u), -value(g, u)?, uid(g, u)), u));
        }
        let mut us = sorted_by(keyed);
        us.extend(enemies(g, pid, false, None, true)?);
        Ok(tgs(g, &us))
    })
}

/// `_perfect(g, it)` : Perfect Execution
fn perfect() -> Value {
    Value::func("_perfect", |g, a| {
        if let Some(u) = g.legal(a[0].item(), 0)? {
            g.ready_obj(u, None, None)?;
            g.grant(u, "Assault", 3, turn());
        }
        Ok(Value::None)
    })
}

/// `_piercing(g, it)` : Piercing Light
fn piercing() -> Value {
    Value::func("_piercing", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        if let Some(x) = g.legal(it, 0)? {
            g.deal(Some(x), 2, "spell", Some(c), None)?;
        }
        if let Some(y) = g.legal(it, 1)? {
            if g.it(it).targets[1].0 != g.it(it).targets[0].0 {
                g.deal(Some(y), 2, "spell", Some(c), None)?;
            }
        }
        Ok(Value::None)
    })
}

/// `_piercing_choices(g, pid, ctx)`
fn piercing_choices() -> Value {
    Value::func("_piercing_choices", |g, a| {
        let pid = a[0].int() as usize;
        let f = dmg_targets(g, pid, 2, true, hbv(&a[1]), false)?;
        let firsts = cap(g, f, 3, Some(pid));
        let seconds: Vec<O> = if full_choices(g, Some(pid)) {
            // a human: any unit for the second target
            dmg_targets(g, pid, 2, false, None, false)?
        } else {
            dmg_targets(g, pid, 2, false, None, false)?.into_iter().filter(|&u| g.o(u).ctrl != pid).take(3).collect()
        };
        let mut out = vec![];
        for &x in &firsts {
            for &y in &seconds {
                if y != x {
                    out.push(map! { tg = Value::Tuple(vec![uid(g, x).into(), uid(g, y).into()]) });
                }
            }
            out.push(map! { tg = Value::Tuple(vec![uid(g, x).into()]) });
        }
        Ok(maps_v(out))
    })
}

/// `_pursuit(g, it)` : Relentless Pursuit
fn pursuit() -> Value {
    Value::func("_pursuit", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
        let c = ctrl(g, it);
        let dest = g.it(it).data.g("dest");
        if g.o(u).loc.map(Value::Loc).unwrap_or(Value::None) != dest {
            g.move_(&[u], dest.loc(), Some(c), false)?;
        }
        let uu = uid(g, u);
        let eqs: Vec<O> = g
            .gear(Some(g.o(u).ctrl))
            .into_iter()
            .filter(|&x| is_equipment(g, x) && g.o(x).attached_to != Some(uu))
            .collect();
        let free0: Vec<O> = eqs.iter().copied().filter(|&x| g.o(x).attached_to.is_none()).collect();
        let free = sorted_by(free0.into_iter().map(|x| ((-bonus_of(g, x), uid(g, x)), x)).collect());
        let mut opts = ov(&free);
        opts.push(Value::None);
        opts.extend(ov(&eqs.iter().copied().filter(|x| !free.contains(x)).collect::<Vec<O>>()));
        let x = g.ask(c, "pursuit_attach", opts, Map::new())?.opt_obj();
        if let Some(x) = x {
            if g.on_board(u) {
                attach(g, x, u)?;
            }
        }
        let oid = g.o(u).oid;
        g.add_effect(mk(vec![
            ("on", "conquer".into()),
            ("fn", pursuit_fire()),
            ("uid", uu.into()),
            ("oid", oid.into()),
            ("dur", turn()),
        ]));
        Ok(Value::None)
    })
}

/// `_pursuit_back(g, it)`
fn pursuit_back() -> Value {
    Value::func("_pursuit_back", |g, a| {
        let it = a[0].item();
        let d = g.it(it).data.clone();
        if let Some(u) = g.obj(d.g("uid").int() as u32) {
            if g.o(u).oid == d.g("oid").int() && g.o(u).loc != Some(Loc::Base) {
                let c = ctrl(g, it);
                g.move_(&[u], Loc::Base, Some(c), false)?;
            }
        }
        Ok(Value::None)
    })
}

/// `_pursuit_fire(g, eff, info)`
fn pursuit_fire() -> Value {
    Value::func("_pursuit_fire", |g, a| {
        let eid = a[0].int() as u64;
        let eff = g.eff(eid).cloned().unwrap();
        for u in a[1].map().g("units").list().to_vec() {
            let u = u.obj();
            if g.o(u).uid as i64 == eff.g("uid").int() && g.o(u).oid == eff.g("oid").int() {
                let (c, uu, oid) = (g.o(u).ctrl, uid(g, u), g.o(u).oid);
                g.queue_trigger(
                    c,
                    "Relentless Pursuit",
                    pursuit_back(),
                    map! { uid = uu, oid = oid },
                    Some(uu),
                    true,
                    Value::None,
                    Value::None,
                );
            }
        }
        Ok(Value::None)
    })
}

/// `_pursuit_choices(g, pid, ctx)`
fn pursuit_choices() -> Value {
    Value::func("_pursuit_choices", |g, a| {
        let pid = a[0].int() as usize;
        let mut out = vec![];
        for u in friends(g, pid, false, hbv(&a[1]))? {
            for d in g.locs() {
                if Some(d) == g.o(u).loc {
                    continue;
                }
                let sc = match d {
                    Loc::Base => 2,
                    Loc::Bf(b) => {
                        if g.bfs[b].ctrl != Some(pid) && g.units(Some(1 - pid), Some(d)).is_empty() {
                            0
                        } else {
                            1
                        }
                    }
                };
                let ds = match d {
                    Loc::Base => "base".to_string(),
                    Loc::Bf(b) => b.to_string(),
                };
                out.push(((sc, -value(g, u)?, uid(g, u), ds), map! { tg = Value::Tuple(vec![uid(g, u).into()]), dest = d }));
            }
        }
        Ok(maps_v(sorted_by(out)))
    })
}

/// `_ruthless_choices(g, pid, ctx)`
fn ruthless_choices() -> Value {
    Value::func("_ruthless_choices", |g, a| {
        let pid = a[0].int() as usize;
        let card = a[1].map().g("card").obj();
        let mut out = vec![];
        let can_disc = !hand_others(g, pid, card).is_empty();
        let ts = dmg_targets(g, pid, 5, true, hbv(&a[1]), false)?;
        for u in cap(g, ts, 4, Some(pid)) {
            let tg = Value::Tuple(vec![uid(g, u).into()]);
            if can_disc && !killable(g, u, 3)? {
                out.push(map! { tg = tg.clone(), disc = true });
            }
            out.push(map! { tg = tg.clone() });
            if can_disc && killable(g, u, 3)? {
                out.push(map! { tg = tg.clone(), disc = true });
            }
        }
        Ok(maps_v(out))
    })
}

/// pay_extra=lambda g, pid, card, ch: ch.get(key) and _discard_one(g, pid, exclude=card)
fn discard_extra(key: &'static str) -> Value {
    Value::func("discard_extra", move |g, a| {
        let pid = a[0].int() as usize;
        let card = a[1].obj();
        if a[2].map().g(key).truthy() {
            discard_one(g, pid, "discard", Some(card), None)?;
        }
        Ok(Value::None)
    })
}

/// `_shakedown(g, it)`
fn shakedown() -> Value {
    Value::func("_shakedown", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
        let opts = if killable(g, u, 6)? && value(g, u)? >= 4.0 { [true, false] } else { [false, true] };
        let uc = g.o(u).ctrl;
        let c = ctrl(g, it);
        if g.ask(uc, "shakedown_let_draw", opts.iter().map(|&b| Value::Bool(b)).collect(), map! { unit = u })?.truthy() {
            g.draw(c, 2)?;
        } else {
            g.deal(Some(u), 6, "spell", Some(c), None)?;
        }
        Ok(Value::None)
    })
}

/// `_square_choices(g, pid, ctx)` : Square Up
fn square_choices() -> Value {
    Value::func("_square_choices", |g, a| {
        let pid = a[0].int() as usize;
        let card = a[1].map().g("card").obj();
        let us = buff_targets(g, pid, false, hbv(&a[1]))?;
        let mut out = tg_choices(g, &cap(g, us.clone(), 4, Some(pid)));
        if !hand_others(g, pid, card).is_empty() {
            let mut rep = vec![];
            for x in cap(g, us.clone(), 2, Some(pid)) {
                for y in cap(g, us.clone(), 2, Some(pid)) {
                    rep.push(map! {
                        tg = Value::Tuple(vec![uid(g, x).into()]),
                        sq = true,
                        tg2 = Value::Tuple(vec![uid(g, y).into()]),
                    });
                }
            }
            let mut o2: Vec<Map> = rep.iter().take(2).cloned().collect();
            o2.extend(out.iter().take(3).cloned());
            o2.extend(rep.iter().skip(2).cloned());
            out = o2;
        }
        Ok(maps_v(out))
    })
}

/// `_square(g, it)`
fn square() -> Value {
    Value::func("_square", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            g.grant(u, "Assault", 4, turn());
        }
        if g.it(it).data.g("sq").truthy() {
            for p in g.it(it).data.g("t2").list().to_vec() {
                let p = p.list();
                let (uu, oid) = (p[0].int() as u32, p[1].int());
                if let Some(v) = g.obj(uu) {
                    let c = ctrl(g, it);
                    if g.o(v).oid == oid && p_unit(g, it, v) && g.targetable(v, Some(c))? {
                        g.grant(v, "Assault", 4, turn());
                    }
                }
            }
        }
        Ok(Value::None)
    })
}

/// `_P_friend_base(g, it, o)`
fn p_friend_base() -> Value {
    Value::func("_P_friend_base", |g, a| {
        let (it, o) = (a[0].item(), a[1].obj());
        Ok((p_friend(g, it, o) && g.o(o).loc == Some(Loc::Base)).into())
    })
}

/// `_storm(g, it)` : Stormbringer
fn storm() -> Value {
    Value::func("_storm", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
        let bf = g.it(it).data.g("bf").int() as usize;
        let m = 0.max(g.might(u)?);
        let c = ctrl(g, it);
        for e in g.units(Some(1 - c), Some(Loc::Bf(bf))) {
            g.deal(Some(e), m, "spell", Some(c), None)?;
        }
        if g.on_board(u) {
            g.move_(&[u], Loc::Bf(bf), Some(c), false)?;
        }
        Ok(Value::None)
    })
}

/// `_storm_choices(g, pid, ctx)`
fn storm_choices() -> Value {
    Value::func("_storm_choices", |g, a| {
        let pid = a[0].int() as usize;
        let mut out = vec![];
        for u in friends(g, pid, false, None)? {
            if g.o(u).loc != Some(Loc::Base) {
                continue;
            }
            let m = g.might(u)?;
            for b in 0..g.bfs.len() {
                let mut sc = 0.0;
                for e in g.units(Some(1 - pid), Some(Loc::Bf(b))) {
                    if killable(g, e, m)? {
                        sc += value(g, e)?;
                    }
                }
                out.push(((-sc, -m, b, uid(g, u)), map! { tg = Value::Tuple(vec![uid(g, u).into()]), bf = b }));
            }
        }
        Ok(maps_v(sorted_by(out)))
    })
}

/// `_sudden(g, it)` : Sudden Storm
fn sudden() -> Value {
    Value::func("_sudden", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            let n = if g.o(u).desig == Some(Desig::Att) { 4 } else { 2 };
            let c = ctrl(g, it);
            g.deal(Some(u), n, "spell", Some(c), None)?;
        }
        Ok(Value::None)
    })
}

fn sudden_choices() -> Value {
    Value::func("sudden_choices", |g, a| {
        let pid = a[0].int() as usize;
        let us = dmg_targets(g, pid, 2, true, hbv(&a[1]), false)?;
        let us = sorted_by(us.into_iter().map(|u| ((g.o(u).ctrl == pid, g.o(u).desig != Some(Desig::Att)), u)).collect());
        Ok(tgs(g, &us))
    })
}

/// `_thrill(g, it)` : Thrill of the Hunt
fn thrill() -> Value {
    Value::func("_thrill", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
        let owner = g.o(u).owner;
        let tok = g.o(u).token;
        g.to_zone(u, "banish", false, None)?;
        if tok || !g.p[owner].banish.contains(&u) {
            return Ok(Value::None);
        }
        let locs = sorted_by(
            g.bf_locs().into_iter().map(|l| ((g.bfs[l.bf().unwrap()].ctrl == Some(owner), l.bf().unwrap()), l)).collect(),
        );
        let cands = play_choices(g, owner, u, "banish", true, 0, &[], Some(locs), None)?;
        play_from(g, owner, u, "banish", cands)?;
        Ok(Value::None)
    })
}

fn thrill_choices() -> Value {
    Value::func("thrill_choices", |g, a| {
        let pid = a[0].int() as usize;
        let mut keyed = vec![];
        for u in friends(g, pid, false, hbv(&a[1]))? {
            keyed.push(((g.o(u).token, is_bf(g.o(u).loc), -value(g, u)?, uid(g, u)), u));
        }
        let us = sorted_by(keyed);
        Ok(tgs(g, &us))
    })
}

/// `_vault(g, it)` : Vault Breaker
fn vault() -> Value {
    Value::func("_vault", |g, a| {
        if let Some(u) = g.legal(a[0].item(), 0)? {
            g.grant(u, "Assault", 2, turn());
            g.grant(u, "Ganking", 1, turn());
        }
        Ok(Value::None)
    })
}

/// `_void_rush(g, it)`
fn void_rush() -> Value {
    Value::func("_void_rush", |g, a| {
        let it = a[0].item();
        let pid = ctrl(g, it);
        let top = g.reveal(pid, 2, None, true, None)?;
        if top.is_empty() {
            return Ok(Value::None);
        }
        let mut cands: Vec<(O, Vec<Cand>)> = vec![];
        for &c in &top {
            let cs = play_choices(g, pid, c, "deck", false, 2, &[], None, None)?;
            cands.push((c, cs));
        }
        let with: Vec<O> = top.iter().copied().filter(|c| !cands.iter().find(|x| x.0 == *c).unwrap().1.is_empty()).collect();
        let sorted = sorted_by(
            with.into_iter().map(|c| ((-(spec_e(g, c) + 2 * g.spec(c).p as i64), uid(g, c)), c)).collect(),
        );
        let mut opts = ov(&sorted);
        opts.push(Value::None);
        let pick = g.ask(pid, "void_rush_pick", opts, Map::new())?.opt_obj();
        if let Some(p) = pick {
            let cs = cands.iter().find(|x| x.0 == p).unwrap().1.clone();
            play_from(g, pid, p, "deck", cs)?;
        }
        for &c in &top {
            if Some(c) != pick && g.p[pid].deck.contains(&c) {
                g.draw_card(pid, c)?;
            }
        }
        Ok(Value::None)
    })
}

// ====================================================================== units
/// resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), n)
fn mod_target(n: i64) -> Value {
    Value::func("mod_target", move |g, a| {
        if let Some(u) = g.legal(a[0].item(), 0)? {
            g.mod_(u, n, turn(), None)?;
        }
        Ok(Value::None)
    })
}

/// resolve=lambda g, it: _me(g, it) is not None and g.mod(_me(g, it), n)
fn mod_me(n: i64) -> Value {
    Value::func("mod_me", move |g, a| {
        if let Some(m) = me(g, a[0].item()) {
            g.mod_(m, n, turn(), None)?;
        }
        Ok(Value::None)
    })
}

/// may_pay(0, lambda g_, it: [FURY])
fn pay_fury() -> Value {
    may_pay(0, Value::func("fury_reqs", |_, _| Ok(reqs_to_value(&fury()))))
}

/// `_reaper(g, o, ev, info)` : Baccai Reaper
fn reaper() -> Value {
    Value::func("_reaper", |g, a| {
        let o = a[0].obj();
        if a[1].str() == "attack" && is_o(&a[2].map().g("obj"), o) {
            let res = Value::func("res", |g_, a| {
                if let Some(m) = me(g_, a[0].item()) {
                    g_.grant(m, "Assault", 2, turn());
                }
                Ok(Value::None)
            });
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            g.queue_trigger(c, "Baccai Reaper", res, Map::new(), Some(u), true, Value::None, pay_fury());
        }
        Ok(Value::None)
    })
}

/// `_twirler_res(g, it)` : Blade Twirler
fn twirler_res() -> Value {
    Value::func("_twirler_res", |g, a| {
        let c = ctrl(g, a[0].item());
        let p = g.ask(c, "burn_player", vec![(1 - c).into(), c.into()], Map::new())?.int() as usize;
        g.burn(p, 1)?;
        Ok(Value::None)
    })
}

fn twirler() -> Value {
    Value::func("_twirler", |g, a| {
        let o = a[0].obj();
        if a[1].str() == "move" && is_o(&a[2].map().g("obj"), o) {
            let key = format!("('blade_twirler', {}, {}, {})", g.o(o).uid, g.o(o).oid, g.turn_no);
            if stats_has(g, &key) {
                return Ok(Value::None);
            }
            g.stats.add(&key, 1);
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            qt(g, c, "Blade Twirler", twirler_res(), Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

/// deal n "ability" to the first target (`_cadet_res`, `_kat_res`, `_bow_res`)
fn deal_ability(n: i64) -> Value {
    Value::func("deal_ability", move |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            let c = ctrl(g, it);
            g.deal(Some(u), n, "ability", Some(c), None)?;
        }
        Ok(Value::None)
    })
}

/// `_cadet_play(g, o, ctx)` : Blast Corps Cadet
fn cadet_play() -> Value {
    Value::func("_cadet_play", |g, a| {
        let o = a[0].obj();
        if a[1].map().g("bcc").truthy() {
            let opts = Value::func("opts", |g_, a| {
                let c = ctrl(g_, a[0].item());
                Ok(ov(&dmg_targets(g_, c, 2, true, None, false)?).into())
            });
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            g.queue_trigger(
                c,
                "Blast Corps Cadet",
                deal_ability(2),
                map! { dmg = 2 },
                Some(u),
                false,
                trig_target(opts, p_unit_bf_v(), "target", false, true),
                Value::None,
            );
        }
        Ok(Value::None)
    })
}

/// as_played=lambda g, pid, c: [dict(key=True), dict()]
fn as_played_opt(key: &'static str) -> Value {
    Value::func("as_played", move |_, _| Ok(maps_v(vec![mk(vec![(key, true.into())]), Map::new()])))
}

/// as_played=lambda g, pid, c: ([dict(key=True)] if _hand_others(g, pid, c) else []) + [dict()]
fn as_played_discard(key: &'static str) -> Value {
    Value::func("as_played", move |g, a| {
        let (pid, c) = (a[0].int() as usize, a[1].obj());
        let mut out = vec![];
        if !hand_others(g, pid, c).is_empty() {
            out.push(mk(vec![(key, true.into())]));
        }
        out.push(Map::new());
        Ok(maps_v(out))
    })
}

/// Dangerous Duo
fn duo() -> Value {
    Value::func("_duo", |g, a| {
        let o = a[0].obj();
        let c = g.o(o).ctrl;
        if g.legion(c, Some(o)) {
            let opts = Value::func("opts", |g_, a| {
                let c = ctrl(g_, a[0].item());
                Ok(ov(&buff_targets(g_, c, false, None)?).into())
            });
            let u = uid(g, o);
            g.queue_trigger(
                c,
                "Dangerous Duo",
                mod_target(2),
                Map::new(),
                Some(u),
                false,
                trig_target(opts, p_unit_v(), "friendly_target", false, true),
                Value::None,
            );
        }
        Ok(Value::None)
    })
}

/// `_vanquisher(g, o, ev, info)` : Draven, Vanquisher
fn vanquisher() -> Value {
    Value::func("_vanquisher", |g, a| {
        let o = a[0].obj();
        let ev = a[1].str();
        let info = a[2].map();
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        if ev == "combat_won"
            && info.g("pid") == Value::from(c)
            && g.o(o).loc == info.g("bf").opt_int().map(|b| Loc::Bf(b as usize))
            && g.sd.as_ref().is_some_and(|sd| sd.members.contains(&u))
        {
            let res = Value::func("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                gold(g_, c)?;
                Ok(Value::None)
            });
            qt(g, c, "Draven, Vanquisher (Gold)", res, Map::new(), Some(u));
        }
        if (ev == "attack" || ev == "defend") && is_o(&info.g("obj"), o) {
            g.queue_trigger(c, "Draven, Vanquisher", mod_me(2), Map::new(), Some(u), true, Value::None, pay_fury());
        }
        Ok(Value::None)
    })
}

/// `_dunebreaker(g, o, ev, info)`
fn dunebreaker() -> Value {
    Value::func("_dunebreaker", |g, a| {
        let o = a[0].obj();
        let info = a[2].map();
        let c = g.o(o).ctrl;
        if a[1].str() == "hold" && info.g("pid") == Value::from(c) && has_o(&info.g("units"), o) {
            let res = Value::func("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.draw(c, 2)?;
                Ok(Value::None)
            });
            let u = uid(g, o);
            qt(g, c, "Dunebreaker", res, Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

/// `_eclipse(g, o, ev, info)` : Eclipse Dragon
fn eclipse() -> Value {
    Value::func("_eclipse", |g, a| {
        let o = a[0].obj();
        let c = g.o(o).ctrl;
        if a[1].str() == "move" && is_o(&a[2].map().g("obj"), o) && g.p[c].runes.len() <= 4 {
            let res = Value::func("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                if g_.p[c].runes.len() <= 4 {
                    g_.draw(c, 1)?;
                }
                Ok(Value::None)
            });
            let u = uid(g, o);
            qt(g, c, "Eclipse Dragon", res, Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

/// `_fewer_runes_start(might, ganking)` : Forsaken Baccai / Oasis Raider
fn fewer_runes_start(might: i64, ganking: bool) -> Value {
    let res = Value::func("res", move |g, a| {
        if let Some(m) = me(g, a[0].item()) {
            g.mod_(m, might, turn(), None)?;
            if ganking {
                g.grant(m, "Ganking", 1, turn());
            }
        }
        Ok(Value::None)
    });
    Value::func("ev_fn", move |g, a| {
        let o = a[0].obj();
        let c = g.o(o).ctrl;
        if a[1].str() == "beginning_start"
            && a[2].map().g("pid") == Value::from(c)
            && g.p[c].runes.len() < g.p[1 - c].runes.len()
        {
            let (n, u) = (g.cname(o).to_string(), uid(g, o));
            qt(g, c, &n, res.clone(), Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

/// Gem Jammer
fn gem_jammer() -> Value {
    Value::func("gem_jammer", |g, a| {
        let o = a[0].obj();
        let res = Value::func("res", |g_, a| {
            if let Some(u) = g_.legal(a[0].item(), 0)? {
                g_.grant(u, "Ganking", 1, turn());
            }
            Ok(Value::None)
        });
        let opts = Value::func("opts", |g_, a| {
            let c = ctrl(g_, a[0].item());
            let mut keyed = vec![];
            for u in friends(g_, c, false, None)? {
                keyed.push(((g_.has_kw(u, "Ganking")?, !is_bf(g_.o(u).loc), -value(g_, u)?, uid(g_, u)), u));
            }
            let mut us = sorted_by(keyed);
            us.extend(enemies(g_, c, false, None, true)?);
            Ok(ov(&us).into())
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        g.queue_trigger(
            c,
            "Gem Jammer",
            res,
            Map::new(),
            Some(u),
            false,
            trig_target(opts, p_unit_v(), "friendly_target", false, true),
            Value::None,
        );
        Ok(Value::None)
    })
}

/// `_apothecary_choose(g, it)` : Grim Apothecary
fn apothecary_choose() -> Value {
    Value::func("_apothecary_choose", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        let us = friends(g, c, true, None)?;
        if us.is_empty() {
            return Ok(false.into());
        }
        let hurt: Vec<O> = us.iter().copied().filter(|&u| g.o(u).damage > 0 || g.o(u).stunned).collect();
        let mut opts = ov(&hurt);
        opts.push(Value::None);
        opts.extend(ov(&us.iter().copied().filter(|u| !hurt.contains(u)).collect::<Vec<O>>()));
        let Some(u) = g.ask(c, "apothecary_pick", opts, Map::new())?.opt_obj() else { return Ok(false.into()) };
        g.add_target(it, u, p_friend_bf_v());
        Ok(true.into())
    })
}

/// `_apothecary_res(g, it)`
fn apothecary_res() -> Value {
    Value::func("_apothecary_res", |g, a| {
        if let Some(u) = g.legal(a[0].item(), 0)? {
            g.to_zone(u, "hand", false, None)?;
        }
        Ok(Value::None)
    })
}

/// `_vox(g, o, ev, info)` : Inviolus Vox
fn vox() -> Value {
    Value::func("_vox", |g, a| {
        let o = a[0].obj();
        let info = a[2].map();
        let c = g.o(o).ctrl;
        if a[1].str() == "conquer" && info.g("pid") == Value::from(c) && has_o(&info.g("units"), o) {
            let opts = Value::func("opts", |g_, a| {
                let c = ctrl(g_, a[0].item());
                Ok(ov(&friends(g_, c, false, None)?).into())
            });
            let u = uid(g, o);
            g.queue_trigger(
                c,
                "Inviolus Vox",
                mod_target(8),
                Map::new(),
                Some(u),
                false,
                trig_target(opts, p_friend_v(), "friendly_target", false, true),
                Value::None,
            );
        }
        Ok(Value::None)
    })
}

/// `_jhin(g, o, ev, info)` : Abilities that add resources resolve immediately and can't be reacted to (rule 429.3.a).
fn jhin() -> Value {
    Value::func("_jhin", |g, a| {
        let o = a[0].obj();
        if a[1].str() == "move" && is_o(&a[2].map().g("obj"), o) {
            let c = g.o(o).ctrl;
            g.add_pool(c, 1, &["A"]);
        }
        Ok(Value::None)
    })
}

/// on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, name, fn, src=o.uid)
fn play_trigger(name: &'static str, f: fn() -> Value) -> Value {
    Value::func("on_play", move |g, a| {
        let o = a[0].obj();
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        qt(g, c, name, f(), Map::new(), Some(u));
        Ok(Value::None)
    })
}

fn discard_1() -> Value {
    discard_n(1)
}

fn discard_2() -> Value {
    discard_n(2)
}

/// Kadregrin the Infernal
fn kadregrin_res() -> Value {
    Value::func("kadregrin", |g, a| {
        let c = ctrl(g, a[0].item());
        let mut n = 0;
        for u in g.units(Some(c), None) {
            if g.mighty(u)? {
                n += 1;
            }
        }
        g.draw(c, n)?;
        Ok(Value::None)
    })
}

/// `_katarina(g, o, ev, info)` : Katarina, Reckless
fn katarina() -> Value {
    Value::func("_katarina", |g, a| {
        let o = a[0].obj();
        let ev = a[1].str();
        let info = a[2].map();
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        if ev == "hide" && info.g("pid") == Value::from(c) {
            let res = Value::func("res", |g_, a| {
                if let Some(m) = me(g_, a[0].item()) {
                    g_.ready_obj(m, None, None)?;
                }
                Ok(Value::None)
            });
            qt(g, c, "Katarina (ready)", res, Map::new(), Some(u));
        }
        if ev == "played" && info.g("pid") == Value::from(c) {
            let it0 = info.g("item");
            let src: Option<String> = if !it0.is_none() {
                g.it(it0.item()).data.g("from").opt_str().map(|s| s.to_string())
            } else {
                g.o(info.g("card").obj()).played_from.clone()
            };
            if src.as_deref() == Some("facedown") {
                let opts = Value::func("opts", |g_, a| {
                    let c = ctrl(g_, a[0].item());
                    Ok(ov(&enemies(g_, c, false, None, true)?).into())
                });
                g.queue_trigger(
                    c,
                    "Katarina, Reckless",
                    deal_ability(2),
                    map! { dmg = 2 },
                    Some(u),
                    false,
                    trig_target(opts, p_enemy_v(), "target", false, true),
                    Value::None,
                );
            }
        }
        Ok(Value::None)
    })
}

/// `_broadmane(g, o, ctx)` : Lord Broadmane
fn broadmane() -> Value {
    Value::func("_broadmane", |g, a| {
        let o = a[0].obj();
        let here = g.o(o).loc;
        let res = Value::func("res", move |g_, a| {
            let it = a[0].item();
            let c = ctrl(g_, it);
            for u in g_.units(Some(c), here) {
                if Some(g_.o(u).uid) != g_.it(it).src {
                    g_.grant(u, "Assault", 1, turn());
                }
            }
            Ok(Value::None)
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        qt(g, c, "Lord Broadmane", res, Map::new(), Some(u));
        Ok(Value::None)
    })
}

/// trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
///             lambda g_, it, u: P_enemy(g_, it, u) and u.loc == here, deflect=True)
fn enemy_here_target(here: Option<Loc>) -> Value {
    let opts = Value::func("opts", move |g, a| {
        let c = ctrl(g, a[0].item());
        let us: Vec<O> = enemies(g, c, false, None, true)?.into_iter().filter(|&u| g.o(u).loc == here).collect();
        Ok(ov(&us).into())
    });
    let pred = Value::func("pred", move |g, a| {
        let (it, u) = (a[0].item(), a[1].obj());
        Ok((p_enemy(g, it, u) && g.o(u).loc == here).into())
    });
    trig_target(opts, pred, "target", false, true)
}

/// `_lucian_res(g, it)`
fn lucian_res() -> Value {
    Value::func("_lucian_res", |g, a| {
        let it = a[0].item();
        let (m, u) = (me(g, it), g.legal(it, 0)?);
        if let (Some(m), Some(u)) = (m, u) {
            let n = g.kw_value(m, "Assault")?;
            let c = ctrl(g, it);
            g.deal(Some(u), n, "ability", Some(c), None)?;
        }
        Ok(Value::None)
    })
}

/// `_lucian(g, o, ev, info)`
fn lucian() -> Value {
    Value::func("_lucian", |g, a| {
        let o = a[0].obj();
        if a[1].str() == "attack" && is_o(&a[2].map().g("obj"), o) {
            let here = g.o(o).loc;
            let dmg = g.kw_value(o, "Assault")?;
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            g.queue_trigger(
                c,
                "Lucian, Gunslinger",
                lucian_res(),
                map! { dmg = dmg },
                Some(u),
                false,
                enemy_here_target(here),
                Value::None,
            );
        }
        Ok(Value::None)
    })
}

/// `_morgana_res(g, it)`
fn morgana_res() -> Value {
    Value::func("_morgana_res", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            if g.o(u).damage > 0 {
                let (d, c) = (g.o(u).damage, ctrl(g, it));
                g.deal(Some(u), d, "ability", Some(c), None)?;
            }
        }
        Ok(Value::None)
    })
}

/// Morgana, Vindictive : on_play
fn morgana_play() -> Value {
    Value::func("morgana_play", |g, a| {
        let o = a[0].obj();
        let opts = Value::func("opts", |g_, a| {
            let c = ctrl(g_, a[0].item());
            let mut keyed = vec![];
            for u in all_units(g_, c, false, None)? {
                let d = g_.o(u).damage;
                keyed.push(((g_.o(u).ctrl == c, d == 0, !killable(g_, u, d)?, -value(g_, u)?, uid(g_, u)), u));
            }
            Ok(ov(&sorted_by(keyed)).into())
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        g.queue_trigger(
            c,
            "Morgana, Vindictive",
            morgana_res(),
            Map::new(),
            Some(u),
            false,
            trig_target(opts, p_unit_v(), "morgana_target", false, true),
            Value::None,
        );
        Ok(Value::None)
    })
}

/// resolve=lambda g, it: _me(g, it) is not None and g.empower(_me(g, it))
fn empower_me() -> Value {
    Value::func("empower_me", |g, a| {
        if let Some(m) = me(g, a[0].item()) {
            g.empower(Some(m), None)?;
        }
        Ok(Value::None)
    })
}

/// `_pyke_res(g, it)`
fn pyke_res() -> Value {
    Value::func("_pyke_res", |g, a| {
        if let Some(m) = me(g, a[0].item()) {
            g.ready_obj(m, None, None)?;
            g.mod_(m, 2, turn(), None)?;
        }
        Ok(Value::None)
    })
}

/// on_play=lambda g, o, ctx: ctx.get(key) and g.queue_trigger(o.ctrl, name, fn, src=o.uid)
fn play_trigger_if(key: &'static str, name: &'static str, f: fn() -> Value) -> Value {
    Value::func("on_play", move |g, a| {
        let o = a[0].obj();
        if a[1].map().g(key).truthy() {
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            qt(g, c, name, f(), Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

/// extra_cost_fn=lambda g, pid, c, ch: (e, [FURY]) if ch.get(key) else (0, [])
fn fury_extra_cost(key: &'static str, e: i64) -> Value {
    Value::func("extra_cost_fn", move |_, a| {
        Ok(if a[2].map().g(key).truthy() { cost_v(e, &fury()) } else { cost_v(0, &[]) })
    })
}

/// `_rell_res(g, it)` : Rell, Magnetic
fn rell_res() -> Value {
    Value::func("_rell_res", |g, a| {
        let it = a[0].item();
        let pid = ctrl(g, it);
        let cs: Vec<O> = g.p[pid]
            .hand
            .iter()
            .copied()
            .filter(|&c| g.spec(c).type_ == "Gear" && is_equipment(g, c) && g.spec(c).e <= 2)
            .collect();
        let mut cands: Vec<(O, Vec<Cand>)> = vec![];
        for &c in &cs {
            let x = play_choices(g, pid, c, "hand", true, 0, &[], None, None)?;
            cands.push((c, x));
        }
        let with: Vec<O> = cs.iter().copied().filter(|c| !cands.iter().find(|x| x.0 == *c).unwrap().1.is_empty()).collect();
        let cs = sorted_by(with.into_iter().map(|c| ((-bonus_of(g, c), uid(g, c)), c)).collect());
        let mut opts = ov(&cs);
        opts.push(Value::None);
        let Some(pick) = g.ask(pid, "rell_pick", opts, Map::new())?.opt_obj() else { return Ok(Value::None) };
        let cc = cands.iter().find(|x| x.0 == pick).unwrap().1.clone();
        let x = play_from(g, pid, pick, "hand", cc)?;
        let m = me(g, it);
        if let (Some(x), Some(m)) = (x.opt_obj(), m) {
            if g.on_board(x) {
                attach(g, x, m)?;
            }
        }
        Ok(Value::None)
    })
}

/// on_event=lambda g, o, ev, info: ev == "attack" and info["obj"] is o and g.queue_trigger(o.ctrl, name, fn, src=o.uid)
fn attack_trigger(name: &'static str, f: fn() -> Value) -> Value {
    Value::func("on_event", move |g, a| {
        let o = a[0].obj();
        if a[1].str() == "attack" && is_o(&a[2].map().g("obj"), o) {
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            qt(g, c, name, f(), Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

/// `_renekton_res(g, it)`
fn renekton_res() -> Value {
    Value::func("_renekton_res", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        if g.p[c].runes.len() > 4 {
            return Ok(Value::None);
        }
        let here = g.it(it).data.g("here").opt_loc();
        for u in g.units(Some(1 - c), here) {
            g.deal(Some(u), 2, "ability", Some(c), None)?;
        }
        Ok(Value::None)
    })
}

/// `_renekton(g, o, ev, info)`
fn renekton() -> Value {
    Value::func("_renekton", |g, a| {
        let o = a[0].obj();
        let c = g.o(o).ctrl;
        if a[1].str() == "attack" && is_o(&a[2].map().g("obj"), o) && g.p[c].runes.len() <= 4 {
            let (here, u) = (g.o(o).loc, uid(g, o));
            qt(g, c, "Renekton, Rage Fueled", renekton_res(), map! { here = here }, Some(u));
        }
        Ok(Value::None)
    })
}

/// `_rumble_res(g, it)` : Rumble, Hotheaded
fn rumble_res() -> Value {
    Value::func("_rumble_res", |g, a| {
        let it = a[0].item();
        let pid = ctrl(g, it);
        let src = g.it(it).src;
        let others: Vec<O> = g.units(Some(pid), None).into_iter().filter(|&u| Some(g.o(u).uid) != src).collect();
        let mechs: Vec<O> = g.p[pid]
            .trash
            .iter()
            .copied()
            .filter(|&c| g.spec(c).type_ == "Unit" && g.spec(c).tags.contains("Mech"))
            .collect();
        let mut opts = vec![];
        let mut keyed = vec![];
        for &u in &others {
            keyed.push(((value(g, u)?, uid(g, u)), u));
        }
        let os = sorted_by(keyed);
        for u in cap(g, os, 4, Some(pid)) {
            let m = 0.max(g.might(u)?);
            let ms = sorted_by(
                mechs.iter().map(|&c| ((-(g.spec(c).might.unwrap_or(0) as i64), uid(g, c)), c)).collect(),
            );
            for c in cap(g, ms, 4, Some(pid)) {
                if !play_choices(g, pid, c, "trash", false, m, &[], None, None)?.is_empty() {
                    opts.push(Value::Tuple(vec![uid(g, u).into(), uid(g, c).into()]));
                }
            }
        }
        opts.push(Value::None);
        let pick = g.ask(pid, "rumble_pick", opts, Map::new())?;
        if pick.is_none() {
            return Ok(Value::None);
        }
        let p = pick.list();
        let u = g.obj(p[0].int() as u32).unwrap();
        let cu = p[1].int() as u32;
        let c = g.p[pid].trash.iter().copied().find(|&x| g.o(x).uid == cu).unwrap();
        let m = 0.max(g.might(u)?);
        g.recycle_cards(pid, &[u])?;
        let cands = play_choices(g, pid, c, "trash", false, m, &[], None, None)?;
        play_from(g, pid, c, "trash", cands)?;
        Ok(Value::None)
    })
}

/// on_event : ev == "conquer" and info["pid"] == o.ctrl and o in info["units"] and g.queue_trigger(...)
fn conquer_trigger(name: &'static str, f: fn() -> Value) -> Value {
    Value::func("on_event", move |g, a| {
        let o = a[0].obj();
        let info = a[2].map();
        let c = g.o(o).ctrl;
        if a[1].str() == "conquer" && info.g("pid") == Value::from(c) && has_o(&info.g("units"), o) {
            let u = uid(g, o);
            qt(g, c, name, f(), Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

/// `_scrapyard_res(g, it)`
fn scrapyard_res() -> Value {
    Value::func("_scrapyard_res", |g, a| {
        discard_n(2).call(g, a)?;
        let c = ctrl(g, a[0].item());
        g.draw(c, 2)?;
        Ok(Value::None)
    })
}

/// `_tibbers(g, it)`
fn tibbers() -> Value {
    Value::func("_tibbers", |g, a| {
        let c = ctrl(g, a[0].item());
        let us: Vec<O> = g.units(None, None).into_iter().filter(|&u| is_bf(g.o(u).loc)).collect();
        for u in us {
            g.deal(Some(u), 3, "ability", Some(c), None)?;
        }
        Ok(Value::None)
    })
}

/// `_reveler(g, o, ev, info)` : Twilight Reveler
fn reveler() -> Value {
    Value::func("_reveler", |g, a| {
        let o = a[0].obj();
        if a[1].str() == "attack" && is_o(&a[2].map().g("obj"), o) {
            let res = Value::func("res", |g_, a| {
                if let Some(u) = g_.legal(a[0].item(), 0)? {
                    g_.ready_obj(u, None, None)?;
                }
                Ok(Value::None)
            });
            let opts = Value::func("opts", |g_, a| {
                let it = a[0].item();
                let c = ctrl(g_, it);
                let src = g_.it(it).src;
                let mut keyed = vec![];
                for u in friends(g_, c, false, None)? {
                    if Some(g_.o(u).uid) != src {
                        keyed.push(((!g_.exhausted(u), -value(g_, u)?, uid(g_, u)), u));
                    }
                }
                Ok(ov(&sorted_by(keyed)).into())
            });
            let pred = Value::func("pred", |g_, a| {
                let (it, u) = (a[0].item(), a[1].obj());
                Ok((p_friend(g_, it, u) && Some(g_.o(u).uid) != g_.it(it).src).into())
            });
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            g.queue_trigger(
                c,
                "Twilight Reveler",
                res,
                Map::new(),
                Some(u),
                false,
                trig_target(opts, pred, "friendly_target", false, true),
                Value::None,
            );
        }
        Ok(Value::None)
    })
}

/// `_vayne(g, o, ev, info)` : Vayne, Hunter
fn vayne() -> Value {
    Value::func("_vayne", |g, a| {
        let o = a[0].obj();
        let info = a[2].map();
        let c = g.o(o).ctrl;
        if a[1].str() == "conquer" && info.g("pid") == Value::from(c) && has_o(&info.g("units"), o) {
            let res = Value::func("res", |g_, a| {
                if let Some(m) = me(g_, a[0].item()) {
                    g_.to_zone(m, "hand", false, None)?;
                }
                Ok(Value::None)
            });
            let u = uid(g, o);
            g.queue_trigger(c, "Vayne, Hunter", res, Map::new(), Some(u), true, Value::None, may_pay(1, Value::None));
        }
        Ok(Value::None)
    })
}

/// `_recycle_one(g, pid, o, ch)` : Vi, Destructive (rule 416.3)
fn recycle_one() -> Value {
    Value::func("_recycle_one", |g, a| {
        let pid = a[0].int() as usize;
        let tr = g.p[pid].trash.clone();
        if !tr.is_empty() {
            let sorted = sorted_by(tr.into_iter().map(|c| ((cval(g, c), uid(g, c)), c)).collect());
            let c = g.ask(pid, "recycle_trash", ov(&sorted), Map::new())?.obj();
            g.recycle_cards(pid, &[c])?;
        }
        Ok(Value::None)
    })
}

/// `_split_options(g, pid, us, total)` : Allocations of total damage among units (tuples of (uid, n)), best first.
fn split_options(g: &mut Game, pid: usize, us: &[O], total: i64) -> Res<Vec<Vec<(u32, i64)>>> {
    let mut keyed = vec![];
    for &u in us {
        keyed.push(((-value(g, u)?, uid(g, u)), u));
    }
    let us = sorted_by(keyed);
    fn need(g: &mut Game, u: O) -> Res<i64> {
        Ok(1.max(g.might(u)? - g.o(u).damage + g.o(u).prevent))
    }
    let mut out: Vec<Vec<(u32, i64)>> = vec![];
    let mut keyed = vec![];
    for &u in &us {
        keyed.push(((need(g, u)?, -value(g, u)?, uid(g, u)), u));
    }
    let by_need = sorted_by(keyed);
    for order in [us.clone(), by_need] {
        let mut left = total;
        let mut al: Vec<(u32, i64)> = vec![];
        for &u in &order {
            let n = need(g, u)?;
            if n <= left {
                al.push((uid(g, u), n));
                left -= n;
            }
        }
        if !al.is_empty() && left != 0 {
            al[0] = (al[0].0, al[0].1 + left);
        } else if al.is_empty() && !order.is_empty() {
            al = vec![(uid(g, order[0]), total)];
        }
        out.push(al);
    }
    for &u in us.iter().take(2) {
        out.push(vec![(uid(g, u), total)]);
    }
    if full_choices(g, Some(pid)) {
        // a human: every split (each chosen unit takes at least 1)
        fn parts(k: i64, n: i64) -> Vec<Vec<i64>> {
            if k == 1 {
                return vec![vec![n]];
            }
            let mut out = vec![];
            for x in 1..(n - k + 2) {
                for rest in parts(k - 1, n - x) {
                    let mut v = vec![x];
                    v.extend(rest);
                    out.push(v);
                }
            }
            out
        }
        for k in 1..=(total.min(us.len() as i64)) {
            for grp in combinations(us.len(), k as usize) {
                for p in parts(k, total) {
                    out.push(grp.iter().zip(p.iter()).map(|(&i, &n)| (uid(g, us[i]), n)).collect());
                }
            }
        }
    }
    let mut res: Vec<Vec<(u32, i64)>> = vec![];
    for x in out {
        if !x.is_empty() && !res.contains(&x) {
            res.push(x);
        }
    }
    Ok(res)
}

fn split_v(al: &[(u32, i64)]) -> Value {
    Value::Tuple(al.iter().map(|&(u, n)| Value::Tuple(vec![u.into(), n.into()])).collect())
}

fn tg_of(al: &[(u32, i64)]) -> Map {
    map! { tg = Value::Tuple(al.iter().map(|&(u, _)| Value::from(u)).collect()) }
}

/// `_voli_choose(g, it)` : Volibear, Furious
fn voli_choose() -> Value {
    Value::func("_voli_choose", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        let here = g.it(it).data.g("here").opt_loc();
        let us: Vec<O> = enemies(g, c, false, None, true)?.into_iter().filter(|&u| g.o(u).loc == here).collect();
        let mut opts = vec![];
        for al in split_options(g, c, &us, 5)? {
            let r = deflect_reqs(g, c, &tg_of(&al))?;
            if g.can_pay(c, 0, &r, &Value::None)? {
                opts.push(split_v(&al));
            }
        }
        if opts.is_empty() {
            return Ok(false.into());
        }
        opts.push(Value::Tuple(vec![])); // "any number" includes zero (rule 355.13)
        let pick = g.ask(c, "split_damage", opts, map! { item = it })?;
        let al: Vec<(u32, i64)> = pick.list().iter().map(|x| (x.list()[0].int() as u32, x.list()[1].int())).collect();
        let reqs = deflect_reqs(g, c, &tg_of(&al))?;
        if !reqs.is_empty() && !g.pay(c, 0, &reqs, &Value::Map(map! { kind = "ability" }))? {
            return Ok(false.into());
        }
        for &(u, _) in &al {
            let pred = Value::func("pred", |g_, a| {
                let (it_, u) = (a[0].item(), a[1].obj());
                Ok((p_enemy(g_, it_, u) && g_.o(u).loc == g_.it(it_).data.g("here").opt_loc()).into())
            });
            let o = g.obj(u).unwrap();
            g.add_target(it, o, pred);
        }
        let split: Vec<Value> = al.iter().map(|&(_, n)| Value::from(n)).collect();
        g.it_mut(it).data.set("split", Value::List(split));
        Ok(true.into())
    })
}

/// `_voli_res(g, it)`
fn voli_res() -> Value {
    Value::func("_voli_res", |g, a| {
        let it = a[0].item();
        let split = g.it(it).data.g("split").list().to_vec();
        for (i, n) in split.iter().enumerate() {
            if let Some(u) = g.legal(it, i)? {
                let c = ctrl(g, it);
                g.deal(Some(u), n.int(), "ability", Some(c), None)?;
            }
        }
        Ok(Value::None)
    })
}

fn volibear() -> Value {
    Value::func("volibear", |g, a| {
        let o = a[0].obj();
        if a[1].str() == "attack" && is_o(&a[2].map().g("obj"), o) {
            let (c, u, here) = (g.o(o).ctrl, uid(g, o), g.o(o).loc);
            g.queue_trigger(
                c,
                "Volibear, Furious",
                voli_res(),
                map! { here = here },
                Some(u),
                false,
                voli_choose(),
                Value::None,
            );
        }
        Ok(Value::None)
    })
}

/// on_play of Zed, From the Shadows
fn zed_res() -> Value {
    Value::func("zed_res", |g, a| {
        let c = ctrl(g, a[0].item());
        token(g, c, "Shadow Clone", false, "token_location")?;
        Ok(Value::None)
    })
}

// ====================================================================== gear
/// `_rig_choices(g, pid, o)` : Assembly Rig
fn rig_choices() -> Value {
    Value::func("_rig_choices", |g, a| {
        let pid = a[0].int() as usize;
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut out = vec![];
        let tr = g.p[pid].trash.clone();
        for c in sorted_by(tr.into_iter().map(|c| ((cval(g, c), uid(g, c)), c)).collect()) {
            if is_unit(g, c) && !seen.contains(g.cname(c)) {
                seen.insert(g.cname(c).to_string());
                out.push(map! { rec = uid(g, c) });
            }
        }
        Ok(maps_v(cap(g, out, 3, Some(pid))))
    })
}

/// `_rig_pay(g, pid, o, ch)`
fn rig_pay() -> Value {
    Value::func("_rig_pay", |g, a| {
        let pid = a[0].int() as usize;
        let rec = a[2].map().g("rec");
        let c = g.p[pid].trash.iter().copied().find(|&x| Value::from(g.o(x).uid) == rec);
        if let Some(c) = c {
            g.recycle_cards(pid, &[c])?;
        }
        Ok(Value::None)
    })
}

/// `_beans_cost(g, it)` : Fresh Beans
fn beans_cost() -> Value {
    Value::func("_beans_cost", |g, a| {
        let Some(m) = me(g, a[0].item()) else { return Ok(false.into()) };
        if g.exhausted(m) {
            return Ok(false.into());
        }
        g.set_exhausted(m, true);
        Ok(true.into())
    })
}

/// `_beans(g, o, ev, info)`
fn beans() -> Value {
    Value::func("_beans", |g, a| {
        let o = a[0].obj();
        let info = a[2].map();
        let c = g.o(o).ctrl;
        if a[1].str() == "played"
            && info.g("pid") == Value::from(c)
            && is_unit(g, info.g("card").obj())
            && g.sd.is_some()
            && !g.exhausted(o)
        {
            let res = Value::func("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.draw(c, 1)?;
                Ok(Value::None)
            });
            let u = uid(g, o);
            g.queue_trigger(c, "Fresh Beans", res, Map::new(), Some(u), true, Value::None, beans_cost());
        }
        Ok(Value::None)
    })
}

/// `_sun_disc(g, it)`
fn sun_disc() -> Value {
    Value::func("_sun_disc", |g, a| {
        let c = ctrl(g, a[0].item());
        if g.legion(c, None) {
            g.add_effect(mk(vec![
                ("kind", "enter_ready".into()),
                ("fn", unit_of_pid("_sun_disc_ready")),
                ("pid", c.into()),
                ("once", true.into()),
                ("dur", turn()),
            ]));
        }
        Ok(Value::None)
    })
}

/// `_sun_disc_ready(g, eff, pid, c)` / `_bushwhack_ready(g, eff, pid, c)` : pid == eff["pid"] and c is a unit
fn unit_of_pid(name: &'static str) -> Value {
    Value::func(name, |g, a| {
        let eid = a[0].int() as u64;
        let ep = g.eff(eid).map(|e| e.g("pid")).unwrap_or(Value::None);
        Ok((a[1] == ep && is_unit(g, a[2].obj())).into())
    })
}

/// `_bow(g, gear, unit, ev, info)` : Recurve Bow
fn bow() -> Value {
    Value::func("_bow", |g, a| {
        let (gear, unit) = (a[0].obj(), a[1].obj());
        let ev = a[2].str();
        if (ev == "attack" || ev == "defend") && is_o(&a[3].map().g("obj"), unit) {
            let here = g.o(unit).loc;
            let (c, gu) = (g.o(unit).ctrl, uid(g, gear));
            g.queue_trigger(c, "Recurve Bow", deal_ability(2), map! { dmg = 2 }, Some(gu), false, enemy_here_target(here), Value::None);
        }
        Ok(Value::None)
    })
}

// ====================================================================== cards using the engine hooks (integration)
/// `_alive(g, uid, oid)`
fn alive(g: &Game, u: u32, oid: i64) -> Option<O> {
    g.obj(u).filter(|&o| g.o(o).oid == oid)
}

/// `_in_zone(g, c, oid, zones)` : The card c if it is still the same object (oid) in one of its owner's zones.
fn in_zone(g: &Game, c: O, oid: i64, zones: &[&str]) -> Option<O> {
    let pl = &g.p[g.o(c).owner];
    for z in zones {
        let zone = match *z {
            "trash" => &pl.trash,
            "hand" => &pl.hand,
            "banish" => &pl.banish,
            "deck" => &pl.deck,
            _ => &pl.champ,
        };
        if zone.contains(&c) && g.o(c).oid == oid {
            return Some(c);
        }
    }
    None
}

/// `_tome_bonus(g, eff, ctx)` : Ravenborn Tome
fn tome_bonus() -> Value {
    Value::func("_tome_bonus", |g, a| {
        let eid = a[0].int() as u64;
        let iid = g.eff(eid).map(|e| e.g("iid")).unwrap_or(Value::None);
        let ctx = a[1].map();
        let item = ctx.g("item");
        Ok(Value::from(
            if ctx.g("kind").opt_str() == Some("spell") && !item.is_none() && Value::from(g.it(item.item()).id) == iid {
                1
            } else {
                0
            },
        ))
    })
}

/// `_tome_next(g, eff, it)`
fn tome_next() -> Value {
    Value::func("_tome_next", |g, a| {
        let iid = g.it(a[1].item()).id;
        g.add_effect(mk(vec![
            ("kind", "dmg_bonus".into()),
            ("fn", tome_bonus()),
            ("iid", iid.into()),
            ("dur", turn()),
        ]));
        Ok(Value::None)
    })
}

/// `_lotus(g, it)` : Lotus Trap
fn lotus() -> Value {
    Value::func("_lotus", |g, a| {
        if let Some(u) = g.legal(a[0].item(), 0)? {
            let (uu, oid) = (uid(g, u), g.o(u).oid);
            g.add_effect(mk(vec![
                ("kind", "double_damage".into()),
                ("uid", uu.into()),
                ("oid", oid.into()),
                ("dur", turn()),
            ]));
        }
        Ok(Value::None)
    })
}

fn lotus_choices() -> Value {
    Value::func("lotus_choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1]);
        let mut keyed = vec![];
        for u in g.units(None, None) {
            if g.targetable(u, Some(pid))? && (hb.is_none() || g.o(u).loc == hb) {
                keyed.push(((g.o(u).ctrl == pid, -value(g, u)?, uid(g, u)), u));
            }
        }
        let us = sorted_by(keyed);
        Ok(tgs(g, &us))
    })
}

/// `_smite_rep(g, eff, u)`
fn smite_rep() -> Value {
    Value::func("_smite_rep", |g, a| {
        let eid = a[0].int() as u64;
        let u = a[1].obj();
        let eff = g.eff(eid).cloned().unwrap();
        if g.o(u).uid as i64 != eff.g("uid").int() || g.o(u).oid != eff.g("oid").int() {
            return Ok(Value::None);
        }
        let pid = eff.g("pid").int() as usize;
        let apply = Value::func("apply", move |g_, a| {
            g_.to_zone(a[0].obj(), "banish", false, Some(pid))?;
            Ok(Value::None)
        });
        Ok(mk(vec![("name", "Smite".into()), ("apply", apply)]).into())
    })
}

/// `_smite(g, it)`
fn smite() -> Value {
    Value::func("_smite", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
        if !is_bf(g.o(u).loc) {
            return Ok(Value::None);
        }
        let (uu, oid, c) = (uid(g, u), g.o(u).oid, ctrl(g, it));
        g.add_effect(mk(vec![
            ("kind", "death_rep".into()),
            ("fn", smite_rep()),
            ("uid", uu.into()),
            ("oid", oid.into()),
            ("pid", c.into()),
            ("dur", turn()),
        ]));
        g.deal(Some(u), 3, "spell", Some(c), None)?;
        Ok(Value::None)
    })
}

/// `_armory_rep(g, eff, u)` : Unlicensed Armory
fn armory_rep() -> Value {
    Value::func("_armory_rep", |g, a| {
        let eid = a[0].int() as u64;
        let u = a[1].obj();
        let eff = g.eff(eid).cloned().unwrap();
        let pid = eff.g("pid").int() as usize;
        if g.o(u).uid as i64 != eff.g("uid").int()
            || g.o(u).oid != eff.g("oid").int()
            || !g.can_pay(pid, 0, &fury(), &Value::None)?
        {
            return Ok(Value::None);
        }
        let cost = Value::func("cost", move |g_, _| {
            if !g_.can_pay(pid, 0, &fury(), &Value::None)? {
                return Ok(false.into());
            }
            Ok(g_.pay(pid, 0, &fury(), &Value::None)?.into())
        });
        let apply = Value::func("apply", move |g_, a| {
            if g_.has_effect(eid) {
                g_.remove_effect(eid);
            }
            g_.save_unit(a[0].obj(), false, true);
            Ok(Value::None)
        });
        Ok(mk(vec![
            ("name", "Unlicensed Armory".into()),
            ("apply", apply),
            ("may", true.into()),
            ("cost", cost),
        ])
        .into())
    })
}

/// `_armory_res(g, it)`
fn armory_res() -> Value {
    Value::func("_armory_res", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            let (uu, oid, c) = (uid(g, u), g.o(u).oid, ctrl(g, it));
            g.add_effect(mk(vec![
                ("kind", "death_rep".into()),
                ("fn", armory_rep()),
                ("uid", uu.into()),
                ("oid", oid.into()),
                ("pid", c.into()),
                ("dur", turn()),
            ]));
        }
        Ok(Value::None)
    })
}

/// `_bushwhack(g, it)`
fn bushwhack() -> Value {
    Value::func("_bushwhack", |g, a| {
        let c = ctrl(g, a[0].item());
        g.add_effect(mk(vec![
            ("kind", "enter_ready".into()),
            ("fn", unit_of_pid("_bushwhack_ready")),
            ("pid", c.into()),
            ("dur", turn()),
        ]));
        gold(g, c)?;
        Ok(Value::None)
    })
}

/// `_brynhir_no_play(g, eff, pid, c, zone)`
fn brynhir_no_play() -> Value {
    Value::func("_brynhir_no_play", |g, a| {
        let eid = a[0].int() as u64;
        let ep = g.eff(eid).map(|e| e.g("pid")).unwrap_or(Value::None);
        Ok((a[1] != ep && !g.o(a[2].obj()).token).into())
    })
}

fn brynhir_res() -> Value {
    Value::func("brynhir_res", |g, a| {
        let c = ctrl(g, a[0].item());
        g.add_effect(mk(vec![
            ("kind", "no_play".into()),
            ("fn", brynhir_no_play()),
            ("pid", c.into()),
            ("dur", turn()),
        ]));
        Ok(Value::None)
    })
}

/// `_chompers_res(g, it)` : Flame Chompers
fn chompers_res() -> Value {
    Value::func("_chompers_res", |g, a| {
        let it = a[0].item();
        let d = g.it(it).data.clone();
        let mut c = None;
        for pl in &g.p {
            for &x in pl.trash.iter().chain(pl.banish.iter()) {
                if Value::from(g.o(x).uid) == d.g("c") {
                    c = Some(x);
                }
            }
        }
        let Some(c) = c else { return Ok(Value::None) };
        if g.o(c).oid != d.g("oid").int() {
            return Ok(Value::None);
        }
        let zone = g.o(c).zone.map(|z| z.name()).unwrap_or("");
        let p = ctrl(g, it);
        play_unit_free(g, p, c, zone, None, false, Some((0, fury())))?;
        Ok(Value::None)
    })
}

/// `_rocket_res(g, it)` : Super Mega Death Rocket!
fn rocket_res() -> Value {
    Value::func("_rocket_res", |g, a| {
        let d = g.it(a[0].item()).data.clone();
        if let Some(c) = in_zone(g, d.g("card").obj(), d.g("oid").int(), &["trash"]) {
            g.to_zone(c, "hand", false, None)?;
        }
        Ok(Value::None)
    })
}

/// `_rocket_cost(g, it)`
fn rocket_cost() -> Value {
    Value::func("_rocket_cost", |g, a| {
        let c = ctrl(g, a[0].item());
        if g.p[c].hand.is_empty() {
            return Ok(false.into());
        }
        Ok(discard_one(g, c, "rocket_discard", None, Some(order_cval))?.is_some().into())
    })
}

/// `_rocket_zone(g, c, ev, info)`
fn rocket_zone() -> Value {
    Value::func("_rocket_zone", |g, a| {
        let c = a[0].obj();
        let owner = g.o(c).owner;
        if a[1].str() == "conquer" && a[2].map().g("pid") == Value::from(owner) && g.o(c).zone == Some(Zone::Trash) {
            let oid = g.o(c).oid;
            g.queue_trigger(
                owner,
                "Super Mega Death Rocket!",
                rocket_res(),
                map! { card = c, oid = oid },
                None,
                true,
                Value::None,
                rocket_cost(),
            );
        }
        Ok(Value::None)
    })
}

/// `_phoenix_res(g, it)` : Immortal Phoenix
fn phoenix_res() -> Value {
    Value::func("_phoenix_res", |g, a| {
        let it = a[0].item();
        let d = g.it(it).data.clone();
        if let Some(c) = in_zone(g, d.g("card").obj(), d.g("oid").int(), &["trash"]) {
            let p = ctrl(g, it);
            play_unit_free(g, p, c, "trash", None, false, Some((1, fury())))?;
        }
        Ok(Value::None)
    })
}

/// `_phoenix_zone(g, c, ev, info)`
fn phoenix_zone() -> Value {
    Value::func("_phoenix_zone", |g, a| {
        let c = a[0].obj();
        if a[1].str() != "die" || g.o(c).zone != Some(Zone::Trash) {
            return Ok(Value::None);
        }
        let d = a[2].map().g("info");
        let d = d.map();
        let owner = g.o(c).owner;
        if spec(d.g("spec").str()).type_ == "Unit"
            && d.g("by") == Value::from(owner)
            && d.g("by_kind").opt_str() == Some("spell")
        {
            let oid = g.o(c).oid;
            g.queue_trigger(
                owner,
                "Immortal Phoenix",
                phoenix_res(),
                map! { card = c, oid = oid },
                None,
                true,
                Value::None,
                Value::None,
            );
        }
        Ok(Value::None)
    })
}

/// `_riches_res(g, it)` : Endless Riches
fn riches_res() -> Value {
    Value::func("_riches_res", |g, a| {
        let c = ctrl(g, a[0].item());
        let cs: Vec<O> = g.p[c].hand.iter().chain(g.p[c].trash.iter()).copied().collect();
        for x in cs {
            g.to_zone(x, "banish", false, Some(c))?;
        }
        g.burn(c, 7)?;
        Ok(Value::None)
    })
}

/// `_riches_play(g, o, ctx)`
fn riches_play() -> Value {
    Value::func("_riches_play", |g, a| {
        let o = a[0].obj();
        let (u, oid, c) = (uid(g, o), g.o(o).oid, g.o(o).ctrl);
        for k in ["skip_draw", "play_from_trash", "trash_to_banish"] {
            g.add_effect(mk(vec![("kind", k.into()), ("src", Value::Tuple(vec![u.into(), oid.into()]))]));
        }
        qt(g, c, "Endless Riches", riches_res(), Map::new(), Some(u));
        Ok(Value::None)
    })
}

/// `_revna_event(g, o, ev, info)` : Revna the Lorekeeper
fn revna_event() -> Value {
    Value::func("_revna_event", |g, a| {
        let o = a[0].obj();
        let info = a[2].map();
        let it = if a[1].str() == "played" { info.g("item") } else { Value::None };
        let c = g.o(o).ctrl;
        if !it.is_none()
            && info.g("pid") == Value::from(c)
            && g.spec(info.g("card").obj()).type_ == "Spell"
            && g.it(it.item()).data.get("paid_e").map(|v| v.int()).unwrap_or(0) >= 4
        {
            let res = Value::func("res", |g_, a| {
                let it_ = a[0].item();
                let oid = g_.it(it_).data.g("oid").int();
                if let Some(x) = g_.it(it_).src.and_then(|s| alive(g_, s, oid)) {
                    g_.ready_obj(x, None, None)?;
                }
                Ok(Value::None)
            });
            let (u, oid) = (uid(g, o), g.o(o).oid);
            qt(g, c, "Revna the Lorekeeper", res, map! { oid = oid }, Some(u));
        }
        Ok(Value::None)
    })
}

/// `_axe_res(g, it)` : Blighted Battleaxe
fn axe_res() -> Value {
    Value::func("_axe_res", |g, a| {
        let it = a[0].item();
        let d = g.it(it).data.clone();
        let u = alive(g, d.g("u").int() as u32, d.g("uoid").int());
        let gear = g.it(it).src.and_then(|s| alive(g, s, d.g("oid").int()));
        if let (Some(gear), Some(u)) = (gear, u) {
            if g.o(gear).attached_to == Some(uid(g, u)) {
                let gu = uid(g, gear);
                let i = g.o(u).attached.iter().position(|&x| x == gu).unwrap();
                g.o_mut(u).attached.remove(i);
                g.o_mut(gear).attached_to = None;
                g.need_cleanup = true;
                let r = g.repr(gear);
                g.log(|| format!("  {r} is unattached"));
            }
        }
        if let Some(u) = u {
            let c = ctrl(g, it);
            g.deal(Some(u), 4, "ability", Some(c), None)?;
        }
        Ok(Value::None)
    })
}

/// `_axe_event(g, gear, u, ev, info)`
fn axe_event() -> Value {
    Value::func("_axe_event", |g, a| {
        let (gear, u) = (a[0].obj(), a[1].obj());
        let c = g.o(u).ctrl;
        let key = (g.o(u).uid, g.o(u).oid);
        if a[2].str() == "end_turn" && a[3].map().g("pid") == Value::from(c) && !g.hist.conquered.contains(&key) {
            let (gu, goid) = (uid(g, gear), g.o(gear).oid);
            qt(g, c, "Blighted Battleaxe", axe_res(), map! { u = key.0, uoid = key.1, oid = goid }, Some(gu));
        }
        Ok(Value::None)
    })
}

/// `_firebrand_mod(g, eff, pid, what)` : Raging Firebrand (une seule fonction, comme en Python : les dict d'effet
/// se comparent par égalité)
fn firebrand_mod() -> Value {
    static F: OnceLock<Value> = OnceLock::new();
    F.get_or_init(|| {
        Value::func("_firebrand_mod", |_, a| {
            let what = a[2].map();
            Ok(if what.g("kind").opt_str() == Some("spell") && !what.g("card").is_none() {
                maps_v(vec![map! { e = -5 }])
            } else {
                Value::Tuple(vec![])
            })
        })
    })
    .clone()
}

/// `_firebrand_used(g, eff, it)` : a spell played by an effect also is "the next spell you play"
fn firebrand_used() -> Value {
    static F: OnceLock<Value> = OnceLock::new();
    F.get_or_init(|| {
        Value::func("_firebrand_used", |g, a| {
            let eid = a[0].int() as u64;
            let m = g.eff(eid).map(|e| e.g("mod")).unwrap_or(Value::None);
            // `if eff["mod"] in g.effects: g.effects.remove(eff["mod"])` (égalité de dict)
            if let Value::Map(m) = m {
                if let Some(e) = g.effects.iter().find(|e| e.d == m).map(|e| e.eid) {
                    g.remove_effect(e);
                }
            }
            Ok(Value::None)
        })
    })
    .clone()
}

/// `_firebrand_res(g, it)`
fn firebrand_res() -> Value {
    Value::func("_firebrand_res", |g, a| {
        let c = ctrl(g, a[0].item());
        let m = mk(vec![
            ("kind", "cost_mod".into()),
            ("fn", firebrand_mod()),
            ("pid", c.into()),
            ("once", true.into()),
            ("dur", turn()),
        ]);
        g.add_effect(m.clone());
        g.add_effect(mk(vec![
            ("kind", "next_spell".into()),
            ("fn", firebrand_used()),
            ("pid", c.into()),
            ("mod", m.into()),
            ("dur", turn()),
        ]));
        Ok(Value::None)
    })
}

/// `_dominus_ready(g, it)`
fn dominus_ready() -> Value {
    Value::func("_dominus_ready", |g, a| {
        if let Some(o) = me(g, a[0].item()) {
            g.ready_obj(o, None, None)?;
        }
        Ok(Value::None)
    })
}

/// `_DOMINUS_AB` (un seul dict, comme en Python)
fn dominus_ab() -> Map {
    static AB: OnceLock<Map> = OnceLock::new();
    AB.get_or_init(|| {
        ability("Ready me", AbilityArgs { cost: "2 runes of any type".into(), resolve: dominus_ready(), ..Default::default() })
    })
    .clone()
}

/// `_dominus(g, it)`
fn dominus() -> Value {
    Value::func("_dominus", |g, a| {
        let Some(u) = g.legal(a[0].item(), 0)? else { return Ok(Value::None) };
        let m = g.might(u)?;
        if m > 0 {
            g.mod_(u, m, turn(), None)?;
        }
        g.o_mut(u).abs.push((dominus_ab(), turn()));
        Ok(Value::None)
    })
}

/// `_excess(g, pid)`
fn excess(g: &Game, pid: usize) -> i64 {
    match &g.sd {
        Some(sd) if sd.combat => sd.excess[pid],
        _ => 0,
    }
}

/// `_trynd_event(g, o, ev, info)` : Tryndamere, Barbarian
fn trynd_event() -> Value {
    Value::func("_trynd_event", |g, a| {
        let o = a[0].obj();
        let c = g.o(o).ctrl;
        if a[1].str() == "conquer"
            && has_o(&a[2].map().g("units"), o)
            && g.sd.as_ref().is_some_and(|sd| sd.combat && sd.attacker == c)
            && excess(g, c) >= 5
        {
            let res = Value::func("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.gain_point(c, "Tryndamere")?;
                Ok(Value::None)
            });
            let u = uid(g, o);
            qt(g, c, "Tryndamere, Barbarian", res, Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

/// `_yeti_event(g, o, ev, info)` : Yeti Brawler
fn yeti_event() -> Value {
    Value::func("_yeti_event", |g, a| {
        let o = a[0].obj();
        let c = g.o(o).ctrl;
        if a[1].str() == "conquer" && has_o(&a[2].map().g("units"), o) && excess(g, c) >= 3 {
            let res = Value::func("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                gold(g_, c)?;
                gold(g_, c)?;
                Ok(Value::None)
            });
            let u = uid(g, o);
            qt(g, c, "Yeti Brawler", res, Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

/// `_gauntlets_cost(g, pid, o, ch)` : Hextech Gauntlets
fn gauntlets_cost() -> Value {
    Value::func("_gauntlets_cost", |g, a| {
        let tg = a[2].map().g("tg");
        let u = if tg.truthy() { g.obj(tg.list()[0].int() as u32) } else { None };
        let m = match u {
            Some(u) => 0.max(g.might(u)?),
            None => 0,
        };
        Ok(cost_v(0.max(3 - m), &[any_req()]))
    })
}

/// `_gauntlets_event(g, gear, u, ev, info)`
fn gauntlets_event() -> Value {
    Value::func("_gauntlets_event", |g, a| {
        let (gear, u) = (a[0].obj(), a[1].obj());
        let c = g.o(u).ctrl;
        if a[2].str() == "conquer" && has_o(&a[3].map().g("units"), u) && excess(g, c) >= 3 {
            let res = Value::func("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.draw(c, 1)?;
                Ok(Value::None)
            });
            let gu = uid(g, gear);
            qt(g, c, "Hextech Gauntlets", res, Map::new(), Some(gu));
        }
        Ok(Value::None)
    })
}

/// `_bramble_res(g, it)` : Red Brambleback
fn bramble_res() -> Value {
    Value::func("_bramble_res", |g, a| {
        let c = ctrl(g, a[0].item());
        let mut keyed = vec![];
        for u in g.units(Some(c), None) {
            keyed.push(((g.o(u).buff > 0, -value(g, u)?, uid(g, u)), u));
        }
        let us = sorted_by(keyed);
        if !us.is_empty() {
            let u = if us.len() > 1 { g.ask(c, "brambleback_buff", ov(&us), Map::new())?.obj() } else { us[0] };
            g.buff(Some(u), None)?;
        }
        Ok(Value::None)
    })
}

/// on_event=lambda g, o, ev, info: ev == "conquer" and o in info["units"] and g.queue_trigger(...)
fn bramble_event() -> Value {
    Value::func("bramble_event", |g, a| {
        let o = a[0].obj();
        if a[1].str() == "conquer" && has_o(&a[2].map().g("units"), o) {
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            qt(g, c, "Red Brambleback", bramble_res(), Map::new(), Some(u));
        }
        Ok(Value::None)
    })
}

fn src_obj(v: &Value) -> O {
    v.obj()
}

// ====================================================================== registre
fn reg(r: &mut Registry, mut im: Impl) {
    im.module = Some(MODULE);
    r.card(im);
}

fn kw(pairs: &[(&str, i64)]) -> Map {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.set(k, *v);
    }
    m
}

fn preds(ps: Vec<Value>) -> Value {
    Value::List(ps)
}

/// `pub fn register` : les cartes de fury.py, dans le même ordre.
pub fn register(r: &mut Registry) {
    // ================================================================== tokens
    // Shadow Clone — "When I attack, you may banish a unit from your trash. If you do, give me [Assault 4] this turn."
    // (rule 187.11)
    reg(r, Impl { on_event: clone_event(), ..Impl::new("Shadow Clone") });

    // ================================================================== spells
    // Angle Shot — "[Reaction] Choose a unit and an Equipment with the same controller. Attach that Equipment to that
    // unit or detach that Equipment from that unit. Draw 1."  (attach if it isn't attached to that unit, else detach)
    reg(r, Impl {
        timing: Some("reaction"),
        preds: preds(vec![p_unit_v(), p_gear_v()]),
        resolve: angle(),
        choices: angle_choices(),
        ..Impl::new("Angle Shot")
    });

    // Blind Fury — "[Action] Each opponent reveals the top card of their Main Deck. Choose one and banish it, then
    // play it, ignoring its cost. Then recycle the rest."  (1v1: one revealed card, so nothing is left to recycle;
    // rule 419.3.c: if it can't be played, nothing more happens and it stays banished)
    reg(r, Impl { timing: Some("action"), resolve: blind_fury(), ..Impl::new("Blind Fury") });

    // Blood Rush — "[Action] [Repeat] 1 energy. Give a unit [Assault 2]." (no duration is printed: the grant lasts
    // while the unit stays on the board)
    reg(r, Impl {
        timing: Some("action"),
        repeat: repeat_cost("1 energy"),
        preds: preds(vec![p_unit_v()]),
        resolve: repeatable(blood_rush()),
        choices: buff_choices(),
        ..Impl::new("Blood Rush")
    });

    // Cleave — "[Action] Give a unit [Assault 3] this turn."
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_v()]),
        resolve: Value::func("cleave", |g, a| {
            if let Some(u) = g.legal(a[0].item(), 0)? {
                g.grant(u, "Assault", 3, turn());
            }
            Ok(Value::None)
        }),
        choices: buff_choices(),
        ..Impl::new("Cleave")
    });

    // Consuming Curse — "[Action] Deal 2 to a unit at a battlefield. This deals 1 Bonus Damage for each card with
    // this name in your trash." (Bonus Damage, rules 712-715: counted as it resolves; this card is still on the chain)
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_bf_v()]),
        resolve: curse(),
        choices: Value::func("curse_choices", |g, a| {
            let pid = a[0].int() as usize;
            let n = curse_n(g, pid);
            let us = dmg_targets(g, pid, n, true, hbv(&a[1]), false)?;
            Ok(tgs(g, &us))
        }),
        ..Impl::new("Consuming Curse")
    });

    // Curtain Call — "[Repeat] — 1 energy / 1 rune of any type / 1 energy and 1 rune of any type. Choose one you
    // haven't already chosen — Draw 1. / Deal 2 to a unit at a battlefield. / Deal 3 to a unit at a base. / Give a
    // unit at a battlefield -4 might this turn."  Choice: cc = the modes in execution order, tg = their targets, rc =
    // the Repeat costs paid (one per extra mode, each cost at most once, rule 820).
    reg(r, Impl {
        preds: preds(vec![p_unit_v()]),
        resolve: curtain_call(),
        choices: cc_choices(),
        opt_parts: Value::func("cc_opt_parts", |_, a| {
            let rc = a[2].map().g("rc");
            let out: Vec<Value> = rc
                .list()
                .iter()
                .map(|i| {
                    let (e, reqs) = cc_rep(i.int() as usize);
                    Value::Tuple(vec!["rep".into(), e.into(), reqs_to_value(&reqs)])
                })
                .collect();
            Ok(Value::List(out))
        }),
        ..Impl::new("Curtain Call")
    });

    // Dancing Grenade — "Deal 2 to a unit. Its controller may play this spell again for 1 rune of any type. If they
    // do, this deals 1 additional Bonus Damage for each time this spell has dealt damage this turn."
    // The new play happens right after this one resolves (the card must be in the trash to be played again).
    reg(r, Impl {
        preds: preds(vec![p_unit_v()]),
        resolve: grenade(),
        choices: Value::func("grenade_choices", |g, a| {
            let pid = a[0].int() as usize;
            let us = dmg_targets(g, pid, 2, false, hbv(&a[1]), true)?;
            Ok(tgs(g, &us))
        }),
        ..Impl::new("Dancing Grenade")
    });

    // Danger Zone — "[Reaction] [Repeat] 1 energy and 1 rune of any type. Give your Mechs +1 might this turn."
    reg(r, Impl {
        timing: Some("reaction"),
        repeat: repeat_cost("1 energy and 1 rune of any type"),
        resolve: repeatable(danger_zone()),
        ..Impl::new("Danger Zone")
    });

    // Death Mark — "[Burn 3]. Play a 0 might Shadow Clone unit token. [Flow] 1 energy and 2 runes of any type"
    reg(r, Impl { resolve: death_mark(), flow: flow_cost("1 energy and 2 runes of any type"), ..Impl::new("Death Mark") });

    // Death from Below — "Kill a unit at a battlefield. Then, if it had 3 might or less, you may play this from your
    // trash for 1 rune of any type."  Decision: the offer is made once, right after this play resolves and the card
    // is in the trash (same mechanism as Dancing Grenade); it is not a lasting permission.
    reg(r, Impl {
        preds: preds(vec![p_unit_bf_v()]),
        resolve: dfb(),
        choices: Value::func("dfb_choices", |g, a| {
            let pid = a[0].int() as usize;
            let hb = hbv(&a[1]);
            let mut keyed = vec![];
            for u in enemies(g, pid, true, hb, true)? {
                keyed.push(((g.might(u)? > 3, -value(g, u)?, uid(g, u)), u));
            }
            let mut us = sorted_by(keyed);
            us.extend(friends(g, pid, true, hb)?);
            Ok(tgs(g, &us))
        }),
        ..Impl::new("Death from Below")
    });

    // Detonate — "Kill a gear. Its controller draws 2."
    reg(r, Impl {
        preds: preds(vec![p_gear_v()]),
        resolve: detonate(),
        choices: Value::func("detonate_choices", |g, a| {
            let pid = a[0].int() as usize;
            let xs = sorted_by(
                g.gear(None).into_iter().map(|x| ((g.o(x).ctrl == pid, -spec_e(g, x), uid(g, x)), x)).collect(),
            );
            Ok(tgs(g, &xs))
        }),
        ..Impl::new("Detonate")
    });

    // Disintegrate — "[Action] Deal 3 to a unit at a battlefield. If this kills it, draw 1."  The unit dies in the
    // cleanup that follows (rule 428.5.c attributes that kill to this spell): the draw happens then.
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_bf_v()]),
        resolve: disintegrate(),
        choices: dmg_choices(3, true),
        ..Impl::new("Disintegrate")
    });

    // Firestorm — "Deal 3 to all enemy units at a battlefield."
    reg(r, Impl { resolve: firestorm(), choices: firestorm_choices(), ..Impl::new("Firestorm") });

    // Get Excited! — "[Action] Discard 1. Deal its Energy cost as damage to a unit at a battlefield."
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_bf_v()]),
        resolve: excited(),
        choices: excited_choices(),
        ..Impl::new("Get Excited!")
    });

    // Hextech Ray / Incinerate / Void Seeker — "[Action] Deal N to a unit at a battlefield. (Draw 1.)"
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_bf_v()]),
        resolve: deal_bf(3, 0),
        choices: dmg_choices(3, true),
        ..Impl::new("Hextech Ray")
    });
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_bf_v()]),
        resolve: deal_bf(2, 0),
        choices: dmg_choices(2, true),
        ..Impl::new("Incinerate")
    });
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_bf_v()]),
        resolve: deal_bf(4, 1),
        choices: dmg_choices(4, true),
        ..Impl::new("Void Seeker")
    });

    // Icathian Rain — "Deal 2 to a unit." six times (six targets; the same unit may be chosen several times)
    reg(r, Impl { preds: preds(vec![p_unit_v()]), resolve: rain(), choices: rain_choices(), ..Impl::new("Icathian Rain") });

    // Monster Harpoon — "[Action] Deal 2 to a unit at a battlefield. If you control a facedown card, deal 4 to it
    // instead."
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_bf_v()]),
        resolve: harpoon(),
        choices: harpoon_choices(),
        ..Impl::new("Monster Harpoon")
    });

    // Noxian Guillotine — "[Action] Choose a unit. Kill it the next time it takes damage this turn. [Legion] — Kill
    // it now instead."  (rule 391 delayed triggered ability; rule 158.2: with Legion the delayed part is ignored)
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_v()]),
        resolve: guillotine(),
        choices: enemies_choices(),
        ..Impl::new("Noxian Guillotine")
    });

    // Perfect Execution — "Ready a unit and give it [Assault 3] this turn. [Flow] 3 energy and 1 fury rune"
    reg(r, Impl {
        preds: preds(vec![p_unit_v()]),
        resolve: perfect(),
        flow: flow_cost("3 energy and 1 fury rune"),
        choices: ready_choices(),
        ..Impl::new("Perfect Execution")
    });

    // Piercing Light — "[Repeat] 2 energy and 1 fury rune. Deal 2 to a unit at a battlefield, then deal 2 to up to
    // one other unit."
    reg(r, Impl {
        repeat: repeat_cost("2 energy and 1 fury rune"),
        preds: preds(vec![p_unit_bf_v(), p_unit_v()]),
        resolve: repeatable(piercing()),
        choices: piercing_choices(),
        ..Impl::new("Piercing Light")
    });

    // Relentless Pursuit — "[Action] Move a friendly unit. You may attach an Equipment with the same controller to
    // it. This turn, that unit has 'When I conquer, you may move me to my base.'"  (a spell's move is not a Standard
    // Move: any destination, rule 144.4 restricts only the Standard Move)
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_friend_v()]),
        resolve: pursuit(),
        choices: pursuit_choices(),
        ..Impl::new("Relentless Pursuit")
    });

    // Right of Conquest — "Draw 1, then draw 1 for each battlefield you or allies control." (1v1: no allies)
    reg(r, Impl {
        resolve: Value::func("right_of_conquest", |g, a| {
            let c = ctrl(g, a[0].item());
            g.draw(c, 1)?;
            let n = g.bfs.iter().filter(|b| b.ctrl == Some(c)).count() as i64;
            g.draw(c, n)?;
            Ok(Value::None)
        }),
        ..Impl::new("Right of Conquest")
    });

    // Ruthless Strike — "[Action] As an additional cost to play this, you may discard 1. Deal 3 to a unit at a
    // battlefield. If you paid the additional cost, deal 5 to it instead."
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_bf_v()]),
        choices: ruthless_choices(),
        pay_extra: discard_extra("disc"),
        resolve: Value::func("ruthless", |g, a| {
            let it = a[0].item();
            if let Some(u) = g.legal(it, 0)? {
                let n = if g.it(it).data.g("disc").truthy() { 5 } else { 3 };
                let c = ctrl(g, it);
                g.deal(Some(u), n, "spell", Some(c), None)?;
            }
            Ok(Value::None)
        }),
        ..Impl::new("Ruthless Strike")
    });

    // Shakedown — "[Reaction] Choose an enemy unit. Deal 6 to it unless its controller has you draw 2."
    reg(r, Impl {
        timing: Some("reaction"),
        preds: preds(vec![p_enemy_v()]),
        resolve: shakedown(),
        choices: enemies_choices(),
        ..Impl::new("Shakedown")
    });

    // Square Up — "[Repeat] — Discard 1. Give a unit [Assault 4] this turn."  The Repeat cost is not a resource cost:
    // choice 'sq' (repeat paid, rule 820) with the repetition's target in 'tg2'.
    reg(r, Impl {
        preds: preds(vec![p_unit_v()]),
        resolve: square(),
        choices: square_choices(),
        pay_extra: discard_extra("sq"),
        ..Impl::new("Square Up")
    });

    // Stormbringer — "Choose a friendly unit in your base. Deal damage equal to its Might to all enemy units at a
    // battlefield, then move your unit there."
    reg(r, Impl {
        preds: preds(vec![p_friend_base()]),
        resolve: storm(),
        choices: storm_choices(),
        ..Impl::new("Stormbringer")
    });

    // Sudden Storm — "[Hidden] [Action] Deal 2 to a unit at a battlefield. If it's attacking, deal 4 to it instead."
    reg(r, Impl {
        timing: Some("action"),
        hidden: true,
        preds: preds(vec![p_unit_bf_v()]),
        resolve: sudden(),
        choices: sudden_choices(),
        ..Impl::new("Sudden Storm")
    });

    // Thermo Beam — "[Action] Kill all gear."
    reg(r, Impl {
        timing: Some("action"),
        resolve: Value::func("thermo_beam", |g, a| {
            let c = ctrl(g, a[0].item());
            let xs = g.gear(None);
            g.kill(&xs, Some(c), false)?;
            Ok(Value::None)
        }),
        ..Impl::new("Thermo Beam")
    });

    // Thrill of the Hunt — "[Reaction] Banish a friendly unit, then its owner plays it to any battlefield, ignoring
    // its cost."  (a token ceases to exist in the banishment, rule 186.1)
    reg(r, Impl {
        timing: Some("reaction"),
        preds: preds(vec![p_friend_v()]),
        resolve: thrill(),
        choices: thrill_choices(),
        ..Impl::new("Thrill of the Hunt")
    });

    // Upstage Comedy — "[Repeat] 2 energy. Ready a unit."
    reg(r, Impl {
        repeat: repeat_cost("2 energy"),
        preds: preds(vec![p_unit_v()]),
        resolve: repeatable(Value::func("upstage", |g, a| {
            if let Some(u) = g.legal(a[0].item(), 0)? {
                g.ready_obj(u, None, None)?;
            }
            Ok(Value::None)
        })),
        choices: ready_choices(),
        ..Impl::new("Upstage Comedy")
    });

    // Vault Breaker — "[Action] Give a unit [Assault 2] and [Ganking] this turn."
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_v()]),
        resolve: vault(),
        choices: buff_choices(),
        ..Impl::new("Vault Breaker")
    });

    // Void Rush — "Reveal the top 2 cards of your Main Deck. You may play one of them, reducing its cost by 2 energy.
    // Draw any you did not play this way."
    reg(r, Impl { resolve: void_rush(), ..Impl::new("Void Rush") });

    // ================================================================== units
    // Arena Kingpin — "I enter ready. [E]: Give a unit +3 might this turn."
    reg(r, Impl {
        enter_ready: true.into(),
        abilities: vec![ability(
            "Might",
            AbilityArgs {
                exhaust: true,
                preds: preds(vec![p_unit_v()]),
                choices: Value::func("kingpin_choices", |g, a| {
                    let pid = a[0].int() as usize;
                    let us = buff_targets(g, pid, false, None)?;
                    Ok(tgs(g, &us))
                }),
                resolve: mod_target(3),
                ..Default::default()
            },
        )],
        ..Impl::new("Arena Kingpin")
    });

    // Baccai Reaper — "When I attack, you may pay 1 fury rune to give me [Assault 2] this turn."
    reg(r, Impl { on_event: reaper(), ..Impl::new("Baccai Reaper") });

    // Baccai Sandspinner — "[Empower] 5 energy. This ability costs 3 energy less if you control 4 or fewer runes.
    // [Empowered] I have [Deflect] and [Assault 2]."
    {
        let mut im = Impl {
            kw_if: vec![(when_empowered(), kw(&[("Deflect", 1), ("Assault", 2)]))],
            ..Impl::new("Baccai Sandspinner")
        };
        impl_empower(
            &mut im,
            Cost::Fn(Value::func("sandspinner_cost", |g, a| {
                let pid = a[0].int() as usize;
                Ok(cost_v(if g.p[pid].runes.len() <= 4 { 2 } else { 5 }, &[]))
            })),
        );
        reg(r, im);
    }

    // Battering Ram — "I cost 1 energy less for each card you've played this turn, to a minimum of 1 energy."
    // (rule 419.4.b: cards finalized this turn, countered ones included)
    reg(r, Impl {
        cost_mod: Value::func("ram_cost_mod", |g, a| {
            let (pid, c) = (a[0].int() as usize, a[1].obj());
            let n = 0.max((g.finalized[pid].len() as i64).min(spec_e(g, c) - 1));
            Ok(Value::Tuple(vec![n.into(), 0.into()]))
        }),
        ..Impl::new("Battering Ram")
    });

    // Blade Twirler — "The first time I move each turn, choose a player. They [Burn 1]."
    reg(r, Impl { on_event: twirler(), ..Impl::new("Blade Twirler") });

    // Blast Corps Cadet — "You may pay 1 energy and 1 fury rune as an additional cost to play me. When you play me, if
    // you paid the additional cost, deal 2 to a unit at a battlefield."
    reg(r, Impl {
        on_play: cadet_play(),
        as_played: as_played_opt("bcc"),
        extra_cost_fn: fury_extra_cost("bcc", 1),
        ..Impl::new("Blast Corps Cadet")
    });

    // Brazen Buccaneer — "As you play me, you may discard 1 as an additional cost. If you do, reduce my cost by 2
    // energy."
    reg(r, Impl {
        as_played: as_played_discard("bb"),
        cost_mod: Value::func("bb_cost_mod", |_, a| {
            Ok(if a[2].map().g("bb").truthy() {
                Value::Tuple(vec![2.into(), 0.into()])
            } else {
                Value::Tuple(vec![0.into(), 0.into()])
            })
        }),
        pay_extra: discard_extra("bb"),
        ..Impl::new("Brazen Buccaneer")
    });

    // Captain Farron — "Other friendly units here have [Assault]."
    reg(r, Impl {
        aura_kw: Value::func("farron_aura", |g, a| {
            let (src, o) = (src_obj(&a[0]), a[1].obj());
            Ok(
                if o != src
                    && g.on_board(src)
                    && is_unit(g, o)
                    && g.o(o).ctrl == g.o(src).ctrl
                    && g.o(o).loc == g.o(src).loc
                {
                    kw(&[("Assault", 1)]).into()
                } else {
                    Value::None
                },
            )
        }),
        ..Impl::new("Captain Farron")
    });

    // Chemtech Enforcer — "[Assault 2] When you play me, discard 1."
    reg(r, Impl {
        kw: kw(&[("Assault", 2)]),
        on_play: play_trigger("Chemtech Enforcer", discard_1),
        ..Impl::new("Chemtech Enforcer")
    });

    // Dangerous Duo — "[Legion] — When you play me, give a unit +2 might this turn."
    reg(r, Impl { on_play: duo(), ..Impl::new("Dangerous Duo") });

    // Draven, Showboat — "My Might is increased by your points."
    reg(r, Impl {
        might_mod: Value::func("showboat", |g, a| Ok(g.p[g.o(a[0].obj()).ctrl].points.into())),
        ..Impl::new("Draven, Showboat")
    });

    // Draven, Vanquisher — "When I win a combat, play a Gold gear token exhausted. When I attack or defend, you may
    // pay 1 fury rune. If you do, give me +2 might this turn."
    reg(r, Impl { on_event: vanquisher(), ..Impl::new("Draven, Vanquisher") });

    // Dunebreaker — "If you have two or fewer cards in your hand, I enter ready. When I hold, draw 2."
    reg(r, Impl {
        enter_ready: Value::func("dunebreaker_ready", |g, a| Ok((g.p[a[0].int() as usize].hand.len() <= 2).into())),
        on_event: dunebreaker(),
        ..Impl::new("Dunebreaker")
    });

    // Eager Drakehound — "I enter ready."
    reg(r, Impl { enter_ready: true.into(), ..Impl::new("Eager Drakehound") });

    // Eclipse Dragon — "[Accelerate] When I move, if you control 4 or fewer runes, draw 1."
    reg(r, Impl { accelerate: true, on_event: eclipse(), ..Impl::new("Eclipse Dragon") });

    // Forsaken Baccai — "If you control fewer runes than an opponent at the start of your Beginning Phase, give me +1
    // might this turn."  /  Oasis Raider — "[same], give me +2 might and [Ganking] this turn."
    reg(r, Impl { on_event: fewer_runes_start(1, false), ..Impl::new("Forsaken Baccai") });
    reg(r, Impl { on_event: fewer_runes_start(2, true), ..Impl::new("Oasis Raider") });

    // Gem Jammer — "When you play me, give a unit [Ganking] this turn." (the Ganking keyword is only in that effect)
    reg(r, Impl { on_play: gem_jammer(), ..Impl::new("Gem Jammer") });

    // Grim Apothecary — "[Ambush] When you play me, you may return a friendly unit at a battlefield to its owner's
    // hand."
    reg(r, Impl {
        ambush: true,
        on_play: Value::func("apothecary_play", |g, a| {
            let o = a[0].obj();
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            g.queue_trigger(c, "Grim Apothecary", apothecary_res(), Map::new(), Some(u), false, apothecary_choose(), Value::None);
            Ok(Value::None)
        }),
        ..Impl::new("Grim Apothecary")
    });

    // Inviolus Vox — "When I conquer, give a friendly unit +8 might this turn."
    reg(r, Impl { on_event: vox(), ..Impl::new("Inviolus Vox") });

    // Jhin, Murderous Artist — "[Deflect] [Ganking] When I move, [Add] 1 energy and 1 rune of any type."
    // Abilities that add resources resolve immediately and can't be reacted to (rule 429.3.a).
    reg(r, Impl { kw: kw(&[("Deflect", 1), ("Ganking", 1)]), on_event: jhin(), ..Impl::new("Jhin, Murderous Artist") });

    // Jinx, Demolitionist — "[Accelerate] [Assault 2] When you play me, discard 2."
    reg(r, Impl {
        accelerate: true,
        kw: kw(&[("Assault", 2)]),
        on_play: play_trigger("Jinx, Demolitionist", discard_2),
        ..Impl::new("Jinx, Demolitionist")
    });

    // Kadregrin the Infernal — "When you play me, draw 1 for each of your [Mighty] units."
    reg(r, Impl { on_play: play_trigger("Kadregrin the Infernal", kadregrin_res), ..Impl::new("Kadregrin the Infernal") });

    // Katarina, Reckless — "When you hide a card, ready me. When you play a card from face down, deal 2 to an enemy
    // unit."
    reg(r, Impl { on_event: katarina(), ..Impl::new("Katarina, Reckless") });

    // Lord Broadmane — "[Ambush] When you play me, give your other units here [Assault] this turn."
    reg(r, Impl { ambush: true, on_play: broadmane(), ..Impl::new("Lord Broadmane") });

    // Lucian, Gunslinger — "[Assault] When I attack, deal damage equal to my [Assault] to an enemy unit here."
    reg(r, Impl { kw: kw(&[("Assault", 1)]), on_event: lucian(), ..Impl::new("Lucian, Gunslinger") });

    // Morgana, Vindictive — "[Ambush] When you play me, deal damage to a unit equal to the damage marked on it."
    reg(r, Impl { ambush: true, on_play: morgana_play(), ..Impl::new("Morgana, Vindictive") });

    // Punching Poro — "[Empower] — Discard 1. [Empowered] I have +1 might."
    reg(r, Impl {
        might_if: vec![(when_empowered(), 1)],
        abilities: vec![ability(
            "Empower",
            AbilityArgs {
                can: Value::func("poro_can", |g, a| {
                    let (pid, o) = (a[0].int() as usize, a[1].obj());
                    Ok((!g.o(o).empowered && !g.p[pid].hand.is_empty()).into())
                }),
                extra_cost: Value::func("poro_extra", |g, a| {
                    discard_one(g, a[0].int() as usize, "discard", None, None)?;
                    Ok(Value::None)
                }),
                resolve: empower_me(),
                ..Default::default()
            },
        )],
        ..Impl::new("Punching Poro")
    });

    // Pyke, Dockside Butcher — "[Hidden] [Ganking] You may pay 1 fury rune as an additional cost to play me. When you
    // play me, if you paid the additional cost, ready me and give me +2 might this turn."
    reg(r, Impl {
        hidden: true,
        kw: kw(&[("Ganking", 1)]),
        on_play: play_trigger_if("pyke", "Pyke, Dockside Butcher", pyke_res),
        as_played: as_played_opt("pyke"),
        extra_cost_fn: fury_extra_cost("pyke", 0),
        ..Impl::new("Pyke, Dockside Butcher")
    });

    // Rell, Magnetic — "[Tank] When I attack, you may play an Equipment with Energy cost no more than 2 energy,
    // ignoring its cost, and attach it to me."  (played from the hand, rule 419.1.a)
    reg(r, Impl { kw: kw(&[("Tank", 1)]), on_event: attack_trigger("Rell, Magnetic", rell_res), ..Impl::new("Rell, Magnetic") });

    // Renekton, Rage Fueled — "[Accelerate] When I attack, if you control 4 or fewer runes, deal 2 to all enemy units
    // here."
    reg(r, Impl { accelerate: true, on_event: renekton(), ..Impl::new("Renekton, Rage Fueled") });

    // Rumble, Hotheaded — "Your Mechs each have [Assault]. When I conquer, you may recycle another friendly unit to
    // play a Mech from your trash. Reduce its Energy cost by the Might of the unit you recycled."
    reg(r, Impl {
        on_event: conquer_trigger("Rumble, Hotheaded", rumble_res),
        aura_kw: Value::func("rumble_aura", |g, a| {
            let (src, o) = (src_obj(&a[0]), a[1].obj());
            Ok(if g.on_board(src) && is_unit(g, o) && g.o(o).ctrl == g.o(src).ctrl && g.tags(o)?.contains("Mech") {
                kw(&[("Assault", 1)]).into()
            } else {
                Value::None
            })
        }),
        ..Impl::new("Rumble, Hotheaded")
    });

    // Scorchclaw — "[Hunt 2] [Level 3] I have +1 might and enter ready."
    reg(r, Impl {
        kw: kw(&[("Hunt", 2)]),
        levels: vec![(3, map! { might = 1, ready = true })],
        ..Impl::new("Scorchclaw")
    });

    // Scrapyard Champion — "[Legion] — When you play me, discard 2, then draw 2."
    reg(r, Impl {
        on_play: Value::func("scrapyard_play", |g, a| {
            let o = a[0].obj();
            let c = g.o(o).ctrl;
            if g.legion(c, Some(o)) {
                let u = uid(g, o);
                qt(g, c, "Scrapyard Champion", scrapyard_res(), Map::new(), Some(u));
            }
            Ok(Value::None)
        }),
        ..Impl::new("Scrapyard Champion")
    });

    // Shadow Assassin — "I enter ready if you have a card with my name in your trash."
    reg(r, Impl {
        enter_ready: Value::func("assassin_ready", |g, a| {
            let pid = a[0].int() as usize;
            Ok(g.p[pid].trash.iter().any(|&x| g.cname(x) == "Shadow Assassin").into())
        }),
        ..Impl::new("Shadow Assassin")
    });

    // Shadow Fiend — "[Empower] 2 energy and 1 fury rune. [Empowered] I have [Assault 3]."
    {
        let mut im = Impl { kw_if: vec![(when_empowered(), kw(&[("Assault", 3)]))], ..Impl::new("Shadow Fiend") };
        impl_empower(&mut im, "2 energy and 1 fury rune".into());
        reg(r, im);
    }

    // Tibbers — "When you play me, deal 3 to all units at battlefields."
    reg(r, Impl { on_play: play_trigger("Tibbers", tibbers), ..Impl::new("Tibbers") });

    // Twilight Reveler — "When I attack, ready another friendly unit."
    reg(r, Impl { on_event: reveler(), ..Impl::new("Twilight Reveler") });

    // Vayne, Hunter — "[Assault 3] If an opponent controls a battlefield, I enter ready. When I conquer, you may pay
    // 1 energy to return me to my owner's hand."
    reg(r, Impl {
        kw: kw(&[("Assault", 3)]),
        on_event: vayne(),
        enter_ready: Value::func("vayne_ready", |g, a| {
            let pid = a[0].int() as usize;
            Ok(g.bfs.iter().any(|b| b.ctrl == Some(1 - pid)).into())
        }),
        ..Impl::new("Vayne, Hunter")
    });

    // Vi, Destructive — "[Ganking] Recycle 1 from your trash: Give me +1 might this turn." (rule 416.3)
    reg(r, Impl {
        kw: kw(&[("Ganking", 1)]),
        abilities: vec![ability(
            "Might",
            AbilityArgs {
                can: Value::func("vi_can", |g, a| Ok((!g.p[a[0].int() as usize].trash.is_empty()).into())),
                extra_cost: recycle_one(),
                resolve: mod_me(1),
                ..Default::default()
            },
        )],
        ..Impl::new("Vi, Destructive")
    });

    // Vi, Hotheaded — "[Deflect] 2 energy and 1 fury rune: Double my Might this turn." (rules 432, 477.3.c)
    reg(r, Impl {
        kw: kw(&[("Deflect", 1)]),
        abilities: vec![ability(
            "Double",
            AbilityArgs {
                cost: "2 energy and 1 fury rune".into(),
                resolve: Value::func("vi_double", |g, a| {
                    if let Some(m) = me(g, a[0].item()) {
                        let n = 0.max(g.might(m)?);
                        g.mod_(m, n, turn(), None)?;
                    }
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..Impl::new("Vi, Hotheaded")
    });

    // Void Drone — "I cost 2 energy less to play from anywhere other than your hand."
    reg(r, Impl {
        cost_mod: Value::func("drone_cost_mod", |g, a| {
            Ok(if g.o(a[1].obj()).zone != Some(Zone::Hand) {
                Value::Tuple(vec![2.into(), 0.into()])
            } else {
                Value::Tuple(vec![0.into(), 0.into()])
            })
        }),
        ..Impl::new("Void Drone")
    });

    // Volibear, Furious — "[Deflect 2] When I attack, deal 5 damage split among any number of enemy units here."
    reg(r, Impl { kw: kw(&[("Deflect", 2)]), on_event: volibear(), ..Impl::new("Volibear, Furious") });

    // Xerath, Freed — "1 fury rune, [E]: Deal 3 to a unit. Use this ability only while I'm at a battlefield."
    reg(r, Impl {
        abilities: vec![ability(
            "Deal 3",
            AbilityArgs {
                cost: "1 fury rune".into(),
                exhaust: true,
                preds: preds(vec![p_unit_v()]),
                can: Value::func("xerath_can", |g, a| Ok(is_bf(g.o(a[1].obj()).loc).into())),
                choices: Value::func("xerath_choices", |g, a| {
                    let pid = a[0].int() as usize;
                    let us = dmg_targets(g, pid, 3, false, None, false)?;
                    Ok(tgs(g, &us))
                }),
                resolve: deal_ability(3),
                ..Default::default()
            },
        )],
        ..Impl::new("Xerath, Freed")
    });

    // Zed, From the Shadows — "You may discard 1 as an additional cost to play me. When you play me, if you paid the
    // additional cost, play a 0 might Shadow Clone unit token."
    reg(r, Impl {
        on_play: play_trigger_if("zed", "Zed, From the Shadows", zed_res),
        as_played: as_played_discard("zed"),
        pay_extra: discard_extra("zed"),
        ..Impl::new("Zed, From the Shadows")
    });

    // ================================================================== gear
    // Assembly Rig — "1 energy and 1 fury rune, Recycle a unit from your trash, [E]: Play a 3 might Mech unit token
    // to your base."
    reg(r, Impl {
        abilities: vec![ability(
            "Mech",
            AbilityArgs {
                cost: "1 energy and 1 fury rune".into(),
                exhaust: true,
                choices: rig_choices(),
                extra_cost: rig_pay(),
                resolve: Value::func("rig_res", |g, a| {
                    let c = ctrl(g, a[0].item());
                    make_token(g, "Mech", c, Loc::Base, false)?;
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..Impl::new("Assembly Rig")
    });

    // Fresh Beans — "When you play a unit during a showdown, you may exhaust this to draw 1."
    reg(r, Impl { on_event: beans(), ..Impl::new("Fresh Beans") });

    // Iron Ballista — "This enters exhausted. [E]: Deal 2 to a unit at a battlefield."
    reg(r, Impl {
        enter_exhausted: true,
        abilities: vec![ability(
            "Deal 2",
            AbilityArgs {
                exhaust: true,
                preds: preds(vec![p_unit_bf_v()]),
                choices: Value::func("ballista_choices", |g, a| {
                    let pid = a[0].int() as usize;
                    let us = dmg_targets(g, pid, 2, true, None, false)?;
                    Ok(tgs(g, &us))
                }),
                resolve: deal_ability(2),
                ..Default::default()
            },
        )],
        ..Impl::new("Iron Ballista")
    });

    // Rage Amplifier — "[Empower] 6 energy and 1 fury rune. Your units have +1 might. If I'm [Empowered], they have +2
    // might instead."
    {
        let mut im = Impl {
            aura_might: Value::func("amplifier_aura", |g, a| {
                let (src, o) = (src_obj(&a[0]), a[1].obj());
                Ok(Value::from(if g.on_board(src) && is_unit(g, o) && g.o(o).ctrl == g.o(src).ctrl {
                    if g.o(src).empowered {
                        2
                    } else {
                        1
                    }
                } else {
                    0
                }))
            }),
            ..Impl::new("Rage Amplifier")
        };
        impl_empower(&mut im, "6 energy and 1 fury rune".into());
        reg(r, im);
    }

    // Seal of Rage — "[E]: [Reaction] — [Add] 1 fury rune."
    reg(r, Impl { add: vec![add_ability("1 fury rune", true, false, Value::None)], ..Impl::new("Seal of Rage") });

    // Sun Disc — "[E]: [Legion] — The next unit you play this turn enters ready." (a one-shot 'enter_ready' effect:
    // a replacement of the way that unit enters, rule 369.3; unit tokens are played too, rule 185.2.a)
    reg(r, Impl {
        abilities: vec![ability(
            "Legion",
            AbilityArgs {
                exhaust: true,
                can: Value::func("sun_disc_can", |g, a| Ok(g.legion(a[0].int() as usize, None).into())),
                resolve: sun_disc(),
                ..Default::default()
            },
        )],
        ..Impl::new("Sun Disc")
    });

    // Equipment (Might Bonus read on the card images, rule 718.4)
    // Recurve Bow — "[Equip] 1 fury rune", +0; Effect Text: "When I attack or defend, deal 2 to an enemy unit here."
    {
        let mut im = Impl { bonus: Some(0), effect_event: bow(), ..Impl::new("Recurve Bow") };
        impl_equip(&mut im, "1 fury rune".into());
        reg(r, im);
    }

    // Serrated Dirk — "[Equip] 1 fury rune", +0; Effect Text: "[Assault 2]"
    {
        let mut im = Impl { bonus: Some(0), equip_kw: Some(kw(&[("Assault", 2)])), ..Impl::new("Serrated Dirk") };
        impl_equip(&mut im, "1 fury rune".into());
        reg(r, im);
    }

    // Spinning Axe — "[Quick-Draw] [Equip] 1 rune of any type [Temporary] (If this is unattached, kill it at the start
    // of its controller's Beginning Phase, before scoring.)", +3
    {
        let mut im = Impl {
            timing: Some("reaction"),
            quickdraw: true,
            bonus: Some(3),
            kw_if: vec![(
                Value::func("axe_unattached", |g, a| Ok(g.o(a[0].obj()).attached_to.is_none().into())),
                kw(&[("Temporary", 1)]),
            )],
            ..Impl::new("Spinning Axe")
        };
        impl_equip(&mut im, "1 rune of any type".into());
        reg(r, im);
    }

    // ================================================================== cards using the engine hooks (integration)
    r.ask_text(&[
        ("armory_discard", "Unlicensed Armory : quelle carte défausser ?"),
        ("rocket_discard", "Super Mega Death Rocket! : quelle carte défausser pour le reprendre ?"),
        ("brambleback_buff", "Red Brambleback : quelle unité alliée améliorer ?"),
    ]);

    // Annie, Fiery — "Your spells and abilities deal 1 Bonus Damage."
    {
        let mut im = Impl::new("Annie, Fiery");
        im.hooks.set(
            "dmg_bonus",
            Value::func("annie_bonus", |g, a| {
                let src = src_obj(&a[0]);
                let ctx = a[1].map();
                let k = ctx.g("kind");
                let k = k.opt_str();
                Ok(Value::from(
                    if matches!(k, Some("spell") | Some("ability") | Some("trigger")) && ctx.g("by") == Value::from(g.o(src).ctrl) {
                        1
                    } else {
                        0
                    },
                ))
            }),
        );
        reg(r, im);
    }

    // Ravenborn Tome — "[E]: The next spell you play this turn deals 1 Bonus Damage." (bound to that spell's chain
    // item)
    reg(r, Impl {
        abilities: vec![ability(
            "Bonus Damage",
            AbilityArgs {
                cost: Cost::None,
                exhaust: true,
                resolve: Value::func("tome_res", |g, a| {
                    let c = ctrl(g, a[0].item());
                    g.add_effect(mk(vec![
                        ("kind", "next_spell".into()),
                        ("fn", tome_next()),
                        ("pid", c.into()),
                        ("dur", turn()),
                    ]));
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..Impl::new("Ravenborn Tome")
    });

    // Lotus Trap — "[Hidden] [Reaction] Choose a unit. Double all damage that would be dealt to it this turn."
    reg(r, Impl {
        hidden: true,
        timing: Some("reaction"),
        preds: preds(vec![p_unit_v()]),
        resolve: lotus(),
        choices: lotus_choices(),
        ..Impl::new("Lotus Trap")
    });

    // Smite — "[Action] Deal 3 to a unit at a battlefield. If it would die this turn, banish it instead."
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_bf_v()]),
        resolve: smite(),
        choices: dmg_choices(3, true),
        ..Impl::new("Smite")
    });

    // Unlicensed Armory — "Discard 1, [E]: Choose a friendly unit. The next time it dies this turn, you may pay [Fury]
    // to recall it exhausted instead."
    reg(r, Impl {
        abilities: vec![ability(
            "Save",
            AbilityArgs {
                cost: Cost::None,
                exhaust: true,
                resolve: armory_res(),
                preds: preds(vec![p_friend_v()]),
                can: Value::func("armory_can", |g, a| Ok((!g.p[a[0].int() as usize].hand.is_empty()).into())),
                choices: Value::func("armory_choices", |g, a| {
                    let pid = a[0].int() as usize;
                    let mut keyed = vec![];
                    for u in g.units(Some(pid), None) {
                        keyed.push(((-value(g, u)?, uid(g, u)), u));
                    }
                    let us = sorted_by(keyed);
                    Ok(tgs(g, &us))
                }),
                extra_cost: Value::func("armory_extra", |g, a| {
                    discard_one(g, a[0].int() as usize, "armory_discard", None, Some(order_cval))?;
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..Impl::new("Unlicensed Armory")
    });

    // Bushwhack — "[Hidden] Friendly units enter ready this turn. Play a Gold gear token exhausted."
    reg(r, Impl { hidden: true, resolve: bushwhack(), ..Impl::new("Bushwhack") });

    // Magma Wurm — "Other friendly units enter ready."
    {
        let mut im = Impl::new("Magma Wurm");
        im.hooks.set(
            "units_enter_ready",
            Value::func("wurm_ready", |g, a| {
                let (src, pid, c) = (src_obj(&a[0]), a[1].int() as usize, a[2].obj());
                Ok((pid == g.o(src).ctrl && c != src && is_unit(g, c) && is_unit(g, src)).into())
            }),
        );
        reg(r, im);
    }

    // Brynhir Thundersong — "When you play me, opponents can't play cards this turn."
    reg(r, Impl { on_play: play_trigger("Brynhir Thundersong", brynhir_res), ..Impl::new("Brynhir Thundersong") });

    // Noxus Saboteur — "Your opponents' [Hidden] cards can't be revealed here." (a hidden card is revealed when it is
    // played from face down, rule 811)
    {
        let mut im = Impl::new("Noxus Saboteur");
        im.hooks.set(
            "forbid_play",
            Value::func("saboteur_forbid", |g, a| {
                let (src, pid, c) = (src_obj(&a[0]), a[1].int() as usize, a[2].obj());
                let zone = a[3].opt_str();
                let sl = g.o(src).loc;
                Ok((zone == Some("facedown") && pid != g.o(src).ctrl && is_bf(sl) && g.o(c).hidden_bf == sl).into())
            }),
        );
        reg(r, im);
    }

    // Perched Grimwyrm — "Play me only to a battlefield you conquered this turn." (Game.hist['conq_bf'])
    {
        let mut im = Impl::new("Perched Grimwyrm");
        im.hooks.set(
            "only_locs",
            Value::func("grimwyrm_locs", |g, a| {
                let pid = a[0].int() as usize;
                let s: BTreeSet<usize> = g.hist.conq_bf[pid].iter().copied().collect();
                Ok(Value::List(s.into_iter().map(|b| Value::Loc(Loc::Bf(b))).collect()))
            }),
        );
        reg(r, im);
    }

    // Rengar, Pouncing — "[Reaction] [Assault 2] I can be played to a battlefield you're attacking."
    {
        let mut im = Impl { timing: Some("reaction"), kw: kw(&[("Assault", 2)]), ..Impl::new("Rengar, Pouncing") };
        im.hooks.set(
            "extra_locs",
            Value::func("rengar_locs", |g, a| {
                let pid = a[0].int() as usize;
                Ok(match &g.sd {
                    Some(sd) if sd.combat && sd.attacker == pid => Value::List(vec![Value::Loc(Loc::Bf(sd.bf))]),
                    _ => Value::List(vec![]),
                })
            }),
        );
        reg(r, im);
    }

    // Rek'Sai, Breacher — "[Accelerate] [Assault] Friendly units played from anywhere other than a player's hand have
    // [Accelerate]."
    {
        let mut im = Impl { accelerate: true, kw: kw(&[("Assault", 1)]), ..Impl::new("Rek'Sai, Breacher") };
        im.hooks.set(
            "card_kw",
            Value::func("reksai_kw", |g, a| {
                let (src, pid, c) = (src_obj(&a[0]), a[1].int() as usize, a[2].obj());
                let k = a[3].str();
                let zone = a[4].opt_str();
                Ok((k == "Accelerate" && pid == g.o(src).ctrl && is_unit(g, c) && zone.is_some() && zone != Some("hand"))
                    .into())
            }),
        );
        reg(r, im);
    }

    // Undying Legion — "[Legion][>] You may play me from your trash for 3 energy and 1 fury rune." (an alternative
    // cost, rule 356.1.a)
    reg(r, Impl {
        alt_costs: Value::func("legion_alt", |g, a| {
            let (pid, c) = (a[0].int() as usize, a[1].obj());
            Ok(if a[2].opt_str() == Some("trash") && g.legion(pid, Some(c)) {
                maps_v(vec![map! { key = "legion", e = 3, reqs = reqs_to_value(&fury()) }])
            } else {
                Value::List(vec![])
            })
        }),
        ..Impl::new("Undying Legion")
    });

    // Flame Chompers — "When you discard me, you may pay [Fury] to play me."
    reg(r, Impl {
        on_discard: Value::func("chompers_discard", |g, a| {
            let (c, pid) = (a[0].obj(), a[1].int() as usize);
            let (u, oid) = (uid(g, c), g.o(c).oid);
            g.queue_trigger(pid, "Flame Chompers", chompers_res(), map! { c = u, oid = oid }, None, true, Value::None, Value::None);
            Ok(Value::None)
        }),
        ..Impl::new("Flame Chompers")
    });

    // Super Mega Death Rocket! — "Deal 5 to a unit. When you conquer, you may discard 1 to return this from your
    // trash to your hand." (a triggered ability of the card in the trash: Impl.zone_event)
    {
        let mut im = Impl {
            preds: preds(vec![p_unit_v()]),
            resolve: Value::func("rocket", |g, a| {
                let it = a[0].item();
                if let Some(u) = g.legal(it, 0)? {
                    let c = ctrl(g, it);
                    g.deal(Some(u), 5, "spell", Some(c), None)?;
                }
                Ok(Value::None)
            }),
            choices: Value::func("rocket_choices", |g, a| {
                let pid = a[0].int() as usize;
                let us = dmg_targets(g, pid, 5, false, hbv(&a[1]), false)?;
                Ok(tgs(g, &us))
            }),
            ..Impl::new("Super Mega Death Rocket!")
        };
        im.hooks.set("zone_event", rocket_zone());
        reg(r, im);
    }

    // Immortal Phoenix — "[Assault 2] When you kill a unit with a spell, you may pay 1 energy and 1 fury rune to play
    // me from your trash." (it triggers from the trash, also when it is the unit killed, rule 383.2.c.1; the killer
    // is the controller of the spell, or of the spell whose damage was lethal, rule 428.5)
    {
        let mut im = Impl { kw: kw(&[("Assault", 2)]), ..Impl::new("Immortal Phoenix") };
        im.hooks.set("zone_event", phoenix_zone());
        reg(r, im);
    }

    // Endless Riches — "When you play this, banish your hand and trash, then [Burn 7]. Skip your Draw Phase. You may
    // play cards from your trash. If a card would go to your trash from anywhere other than your Main Deck, banish it
    // instead." The three static abilities are g.effects entries bound to this permanent (src): they apply while it
    // is on the board, for its controller (Game.effects_of).
    reg(r, Impl { on_play: riches_play(), ..Impl::new("Endless Riches") });

    // Prepared Neophyte — "If you've spent 4 energy or more to play a spell this turn, I have +4 might."
    reg(r, Impl {
        might_if: vec![(
            Value::func("neophyte_cond", |g, a| Ok(g.hist.spell_e[g.o(a[0].obj()).ctrl].iter().any(|&e| e >= 4).into())),
            4,
        )],
        ..Impl::new("Prepared Neophyte")
    });

    // Revna the Lorekeeper — "[Ganking] When you play a spell, if you spent 4 energy or more, ready me."
    reg(r, Impl { kw: kw(&[("Ganking", 1)]), on_event: revna_event(), ..Impl::new("Revna the Lorekeeper") });

    // Raging Soul — "If you've discarded a card this turn, I have [Assault] and [Ganking]."
    reg(r, Impl {
        kw_if: vec![(
            Value::func("soul_cond", |g, a| Ok((g.hist.discarded[g.o(a[0].obj()).ctrl] > 0).into())),
            kw(&[("Assault", 1), ("Ganking", 1)]),
        )],
        ..Impl::new("Raging Soul")
    });

    // Towering Pairofant — "[Assault] If a unit died this turn, I enter ready."
    reg(r, Impl {
        kw: kw(&[("Assault", 1)]),
        enter_ready: Value::func("pairofant_ready", |g, _| {
            Ok(g.hist.died.iter().any(|d| spec(d.g("spec").str()).type_ == "Unit").into())
        }),
        ..Impl::new("Towering Pairofant")
    });

    // Blighted Battleaxe — "[Equip] 1 energy and 1 fury rune". Might Bonus +4, Effect Text (card image): "At the end
    // of your turn, if I didn't conquer this turn, unattach this and deal 4 to me." (Game.hist['conquered'])
    {
        let mut im = Impl { bonus: Some(4), effect_event: axe_event(), ..Impl::new("Blighted Battleaxe") };
        impl_equip(&mut im, "1 energy and 1 fury rune".into());
        reg(r, im);
    }

    // Raging Firebrand — "When you play me, the next spell you play this turn costs 5 energy less."
    reg(r, Impl { on_play: play_trigger("Raging Firebrand", firebrand_res), ..Impl::new("Raging Firebrand") });

    // Dominus — "[Action] This turn, double a unit's Might and give it '2 runes of any type: Ready me.'"
    reg(r, Impl {
        timing: Some("action"),
        preds: preds(vec![p_unit_v()]),
        resolve: dominus(),
        choices: buff_choices(),
        ..Impl::new("Dominus")
    });

    // Dune Surfer — "You ignore [Tank] while assigning combat damage here."
    {
        let mut im = Impl::new("Dune Surfer");
        im.hooks.set(
            "ignore_tank",
            Value::func("surfer_tank", |g, a| {
                let (src, pid, bf) = (src_obj(&a[0]), a[1].int() as usize, a[2].int() as usize);
                Ok((pid == g.o(src).ctrl && g.o(src).loc == Some(Loc::Bf(bf))).into())
            }),
        );
        reg(r, im);
    }

    // Tryndamere, Barbarian — "When I conquer after an attack, if you assigned 5 or more excess damage to enemy units,
    // you score 1 point." (the excess damage of the combat's assignment: Showdown.excess)
    reg(r, Impl { on_event: trynd_event(), ..Impl::new("Tryndamere, Barbarian") });

    // Yeti Brawler — "When I conquer, if you assigned 3 or more excess damage, play two Gold gear tokens exhausted."
    reg(r, Impl { on_event: yeti_event(), ..Impl::new("Yeti Brawler") });

    // Hextech Gauntlets — "[Equip] 3 energy and 1 rune of any type. This ability's Energy cost is reduced by the Might
    // of the unit you choose." Might Bonus +3, Effect Text (card image): "When I conquer, if you assigned 3 or more
    // excess damage, draw 1."
    {
        let mut ab = equip_ability_cost(Cost::Fn(gauntlets_cost()), "main");
        ab.set("printed_e", 3);
        reg(r, Impl {
            bonus: Some(3),
            abilities: vec![ab],
            effect_event: gauntlets_event(),
            ..Impl::new("Hextech Gauntlets")
        });
    }

    // Red Brambleback — "[Accelerate] Your conquer effects for conquering here trigger an additional time. When I
    // conquer, [Buff] a friendly unit." (Impl.scoring_extra: the triggered abilities of the conquer are queued once
    // more)
    {
        let mut im = Impl { accelerate: true, on_event: bramble_event(), ..Impl::new("Red Brambleback") };
        im.hooks.set(
            "scoring_extra",
            Value::func("bramble_extra", |g, a| {
                let (src, pid, bf) = (src_obj(&a[0]), a[1].int() as usize, a[2].int() as usize);
                Ok(Value::from(
                    if a[3].str() == "conquer" && pid == g.o(src).ctrl && g.o(src).loc == Some(Loc::Bf(bf)) { 1 } else { 0 },
                ))
            }),
        );
        reg(r, im);
    }

    // Skyfall of Areion — "[Equip] 1 energy and 1 fury rune". Might Bonus +2, Effect Text (card image): "My hold
    // effects are also conquer effects, and vice versa." (Game.scoring_event delivers the other event to the unit's
    // own abilities)
    {
        let mut im = Impl { bonus: Some(2), ..Impl::new("Skyfall of Areion") };
        impl_equip(&mut im, "1 energy and 1 fury rune".into());
        im.hooks.set("fx_swap_scoring", true);
        reg(r, im);
    }

    // Void Hatchling — "If you would reveal cards from a deck, look at the top card first. You may recycle it. Then
    // reveal those cards." (Game.reveal)
    {
        let mut im = Impl::new("Void Hatchling");
        im.hooks.set("reveal_rep", true);
        reg(r, im);
    }

    // Minotaur Reckoner — "Units can't move to base." (every unit; a recall is not a move, rule 455)
    {
        let mut im = Impl::new("Minotaur Reckoner");
        im.hooks.set(
            "aura_cant_move",
            Value::func("reckoner_cant_move", |_, a| Ok((a[2] == Value::Loc(Loc::Base)).into())),
        );
        reg(r, im);
    }
}
