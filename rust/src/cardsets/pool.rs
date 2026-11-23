//! Cartes du pool définies directement dans `cards.py` (de `_akali_retreat` à `_atakhan_event`) : légendes
//! Akali et LeBlanc, battlefields, jetons, decks Akali et LeBlanc. Traduction ligne à ligne : mêmes conditions,
//! même ordre d'opérations, mêmes tris, mêmes questions (`g.ask`). Enregistré en premier dans `register_all`, comme
//! `cards.py` enregistre ses cartes avant de charger `cardsets`.
use std::collections::BTreeSet;

use crate::actions::full_choices;
use crate::cards::{Impl, Registry};
use crate::cards_helpers::{
    all_units, cap, enemies, equip_ability, friends, item_by_id, make_token, may_pay, p_enemy, p_enemy_bf_v, p_enemy_v,
    p_friend_bf_v, p_friend_v, p_gear_v, p_unit_bf_v, p_unit_v, play_unit_free, tg_choices, trig_target, value,
};
use crate::game::{Game, ItemId, Res, Zone, O, R};
use crate::game_board::loc_str;
use crate::game_costs::{any_req, reqs_to_value, Req};
use crate::map;
use crate::value::{is_bf, Loc, Map, Value};

// ---------------------------------------------------------------------------------------------- aides privées
/// Emplacement rangé dans une valeur : `Loc`, indice de battlefield (`Int`), 'base' ou None.
fn loc_v(v: &Value) -> Option<Loc> {
    match v {
        Value::Loc(l) => Some(*l),
        Value::Int(i) => Some(Loc::Bf(*i as usize)),
        Value::Str(s) if &**s == "base" => Some(Loc::Base),
        _ => None,
    }
}

/// `ctx.get(k)` (le contexte peut être un dict ou None).
fn ctx_get(ctx: &Value, k: &str) -> Value {
    match ctx {
        Value::Map(m) => m.g(k),
        _ => Value::None,
    }
}

/// `ctx["hidden_bf"]`
fn hbf(ctx: &Value) -> Option<Loc> {
    loc_v(&ctx_get(ctx, "hidden_bf"))
}

/// `g.units(pid, loc)` quand loc est l'emplacement d'un objet : None (hors plateau) ne correspond à aucune unité,
/// comme en Python (`u.loc == None`), et non à « partout ».
fn units_at(g: &Game, pid: Option<usize>, loc: Option<Loc>) -> Vec<O> {
    match loc {
        None => vec![],
        Some(l) => g.units(pid, Some(l)),
    }
}

/// `sorted(v, key=key)` avec une clé flottante (tri stable, comme Python).
fn sorted_f<F>(g: &mut Game, v: Vec<O>, mut key: F) -> Res<Vec<O>>
where
    F: FnMut(&mut Game, O) -> Res<f64>,
{
    let mut k = Vec::with_capacity(v.len());
    for o in v {
        k.push((key(g, o)?, o));
    }
    k.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("clé de tri NaN"));
    Ok(k.into_iter().map(|x| x.1).collect())
}

/// `sorted(us, key=lambda u: value(g, u))`
fn by_value(g: &mut Game, v: Vec<O>) -> Res<Vec<O>> {
    sorted_f(g, v, value)
}

/// `sorted(us, key=lambda u: -value(g, u))`
fn by_value_desc(g: &mut Game, v: Vec<O>) -> Res<Vec<O>> {
    sorted_f(g, v, |g, u| Ok(-value(g, u)?))
}

fn maps(v: Vec<Map>) -> Value {
    Value::List(v.into_iter().map(Value::Map).collect())
}

/// `tg_choices(units)` en valeur
fn tgs(g: &Game, us: &[O]) -> Value {
    maps(tg_choices(g, us))
}

fn tup(v: Vec<Value>) -> Value {
    Value::Tuple(v)
}

/// `(e, reqs)` d'un coût de capacité
fn cost_v(e: i64, reqs: Vec<Req>) -> Value {
    tup(vec![e.into(), reqs_to_value(&reqs)])
}

fn req(d: &str) -> Req {
    let mut s = BTreeSet::new();
    s.insert(d.to_string());
    s
}

fn src_obj(g: &Game, it: ItemId) -> Option<O> {
    g.it(it).src.and_then(|u| g.obj(u))
}

fn ctrl(g: &Game, it: ItemId) -> usize {
    g.it(it).ctrl
}

fn has_domain(g: &Game, o: O, d: &str) -> bool {
    g.spec(o).domains.iter().any(|x| x == d)
}

fn spec_might(g: &Game, o: O) -> i64 {
    g.spec(o).might.unwrap_or(0) as i64
}

/// `info[k] is o`
fn is_obj(info: &Value, k: &str, o: O) -> bool {
    info.map().g(k).opt_obj() == Some(o)
}

/// `o in info[k]` (liste d'objets)
fn in_list(info: &Value, k: &str, o: O) -> bool {
    info.map().g(k).list().iter().any(|v| v.opt_obj() == Some(o))
}

fn pid_of(v: &Value) -> usize {
    v.int() as usize
}

/// `g.queue_trigger(pid, name, fn, data, src, may, choose, cost)`
#[allow(clippy::too_many_arguments)]
fn qt(g: &mut Game, pid: usize, name: &str, f: Value, data: Map, src: Option<u32>, may: bool, choose: Value, cost: Value) {
    g.queue_trigger(pid, name, f, data, src, may, choose, cost);
}

fn none() -> Res<Value> {
    Ok(Value::None)
}

/// `itertools.combinations(range(n), k)` : indices dans l'ordre lexicographique.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = vec![];
    if k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.clone());
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

/// `str(loc)` pour `sorted(locs, key=str)` : '0', '1', 'base'.
fn loc_key(l: Option<Loc>) -> String {
    loc_str(l)
}

// ---------------------------------------------------------------------------------------------- légendes
// Akali, Rogue Assassin — "[Empower] [3][A]. [Action][>] [E]: If it's your turn, move a friendly unit in a
// showdown to base and if I'm [Empowered], ready it."
fn akali_retreat() -> Value {
    Value::func("_akali_retreat", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        let u = g.legal(it, 0)?;
        let Some(u) = u else { return none() };
        if g.tp != c {
            return none();
        }
        g.move_(&[u], Loc::Base, Some(c), false)?;
        let me = src_obj(g, it); // "if I'm [Empowered]": the ability's source (Heimerdinger may have it)
        let leg = me.unwrap_or(g.p[c].legend);
        if g.o(leg).empowered {
            g.ready_obj(u, None, None)?;
        }
        none()
    })
}

fn akali_retreat_choices() -> Value {
    Value::func("_akali_retreat_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let Some(sdbf) = g.sd.as_ref().map(|s| s.bf) else { return Ok(Value::List(vec![])) };
        if g.tp != pid {
            return Ok(Value::List(vec![]));
        }
        let out: Vec<Map> = friends(g, pid, false, None)?
            .into_iter()
            .filter(|&u| g.o(u).loc == Some(Loc::Bf(sdbf)))
            .map(|u| map! { tg = tup(vec![g.o(u).uid.into()]) })
            .collect();
        Ok(maps(out))
    })
}

fn akali_rogue_assassin() -> Impl {
    let mut empower = Map::new();
    empower.set("name", "Empower");
    empower.set("timing", "main");
    empower.set("can", Value::func("can", |g, a| Ok((!g.o(a[1].obj()).empowered).into())));
    empower.set("cost", Value::func("cost", |_g, _a| Ok(cost_v(3, vec![any_req()]))));
    empower.set(
        "resolve",
        Value::func("resolve", |g, a| {
            let c = ctrl(g, a[0].item());
            let leg = g.p[c].legend;
            g.empower(Some(leg), None)?;
            none()
        }),
    );
    let mut retreat = Map::new();
    retreat.set("name", "Retreat");
    retreat.set("timing", "action");
    retreat.set("exhaust", true);
    retreat.set("can", Value::func("can", |g, a| Ok((g.sd.is_some() && g.tp == pid_of(&a[0])).into())));
    retreat.set("choices", akali_retreat_choices());
    retreat.set("cost", Value::func("cost", |_g, _a| Ok(cost_v(0, vec![]))));
    retreat.set(
        "preds",
        Value::List(vec![Value::func("pred", |g, a| {
            let (it, o) = (a[0].item(), a[1].obj());
            let ok = g.o(o).ctrl == ctrl(g, it) && g.sd.as_ref().is_some_and(|s| g.o(o).loc == Some(Loc::Bf(s.bf)));
            Ok(ok.into())
        })]),
    );
    retreat.set("resolve", akali_retreat());
    Impl { abilities: vec![empower, retreat], ..Impl::new("Akali, Rogue Assassin") }
}

// LeBlanc, Deceiver — "When you conquer or hold, you may discard 1 and exhaust me to play a ready Reflection unit
// token there. It becomes a copy of another unit there. Give it [Temporary]."
fn reflection(g: &mut Game, pid: usize, loc: Loc, model: Option<O>) -> Res<Option<O>> {
    let Some(t) = make_token(g, "Reflection", pid, loc, true)? else { return Ok(None) };
    if let Some(m) = model {
        let n = g.cname(m).to_string();
        g.o_mut(t).copy = Some(n); // copyable traits incl. stats (Reflection ruling)
    }
    g.grant(t, "Temporary", 1, Value::None);
    let mn = model.map(|m| g.cname(m).to_string()).unwrap_or_else(|| "nothing".to_string());
    let ls = loc_str(Some(loc));
    g.log(|| format!("  Reflection of {mn} at {ls}"));
    g.stats.add(&format!("reflection_P{pid}"), 1);
    Ok(Some(t))
}

fn leblanc_legend() -> Value {
    Value::func("_leblanc_legend", |g, a| {
        let (pid, ev, info) = (pid_of(&a[0]), a[1].str(), &a[2]);
        if !(ev == "conquer" || ev == "hold") || info.map().g("pid").int() as usize != pid {
            return none();
        }
        let bf = info.map().g("bf").int() as usize;
        let cost = Value::func("cost", |g_, a| {
            let it = a[0].item();
            let c = ctrl(g_, it);
            let leg = g_.p[c].legend;
            if g_.exhausted(leg) || g_.p[c].hand.is_empty() {
                return Ok(false.into());
            }
            let hand: Vec<Value> = g_.p[c].hand.iter().map(|&x| x.into()).collect();
            let cd = g_.ask(c, "discard", hand, map! { reason = "leblanc" })?.obj();
            g_.to_zone(cd, "trash", false, None)?;
            g_.set_exhausted(leg, true);
            Ok(true.into())
        });
        let choose = Value::func("choose", move |g_, a| {
            let it = a[0].item();
            let us = g_.units(None, Some(Loc::Bf(bf)));
            if us.is_empty() {
                return Ok(false.into());
            }
            let us = by_value_desc(g_, us)?;
            let c = ctrl(g_, it);
            let o = g_.ask(c, "copy_target", us.into_iter().map(Value::Obj).collect(), map! { item = it })?.obj();
            g_.add_target(it, o, Value::func("pred", move |g2, a2| Ok((g2.o(a2[1].obj()).loc == Some(Loc::Bf(bf))).into())));
            Ok(true.into())
        });
        let res = Value::func("res", move |g_, a| {
            let it = a[0].item();
            let model = g_.legal(it, 0)?;
            let c = ctrl(g_, it);
            reflection(g_, c, Loc::Bf(bf), model)?;
            none()
        });
        qt(g, pid, "LeBlanc, Deceiver", res, map! { bf = bf }, None, true, choose, cost);
        none()
    })
}

// ---------------------------------------------------------------------------------------------- battlefields
fn bar() -> Value {
    Value::func("_bar", |g, a| {
        // "When a unit moves from here, give it +1 might this turn."
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), &a[2]);
        if ev == "move" && loc_v(&info.map().g("frm")) == Some(Loc::Bf(b)) {
            let o = info.map().g("obj").obj();
            let oc = g.o(o).ctrl;
            let uid = g.o(o).uid;
            let f = Value::func("bar_res", |g_, a| {
                let it = a[0].item();
                let u = g_.it(it).data.g("u").int() as u32;
                if let Some(x) = g_.obj(u) {
                    g_.mod_(x, 1, "turn".into(), None)?;
                }
                none()
            });
            qt(g, oc, "Back-Alley Bar", f, map! { u = uid }, None, false, Value::None, Value::None);
        }
        none()
    })
}

fn dusk() -> Value {
    Value::func("_dusk", |g, a| {
        // "At the start of your Beginning Phase, you may kill a unit you control here to draw 1."
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), &a[2]);
        if ev == "beginning_start" {
            let pid = info.map().g("pid").int() as usize;
            if g.units(Some(pid), Some(Loc::Bf(b))).is_empty() {
                return none();
            }
            let res = Value::func("res", move |g_, a| {
                let it = a[0].item();
                let c = ctrl(g_, it);
                let us = g_.units(Some(c), Some(Loc::Bf(b)));
                if us.is_empty() {
                    return none();
                }
                let mut opts = vec![Value::None];
                opts.extend(us.into_iter().map(Value::Obj));
                let u = g_.ask(c, "dusk_kill", opts, Map::new())?;
                let Some(u) = u.opt_obj() else { return none() };
                g_.kill(&[u], Some(c), true)?;
                g_.draw(c, 1)?;
                none()
            });
            qt(g, pid, "Dusk Rose Lab", res, Map::new(), None, true, Value::None, Value::None);
        }
        none()
    })
}

fn sigil() -> Value {
    Value::func("_sigil", |g, a| {
        // "When you conquer here, recycle one of your runes."
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), &a[2]);
        if ev == "conquer" && info.map().g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a| {
                let it = a[0].item();
                let c = ctrl(g_, it);
                let rs = g_.p[c].runes.clone();
                if !rs.is_empty() {
                    let r: R = g_.ask(c, "recycle_rune", rs.into_iter().map(Value::Rune).collect(), Map::new())?.rune();
                    if !g_.r(r).exhausted {
                        // a ready rune is first exhausted for its [Add] 1 Energy (rule 429.3.a: Add Reactions resolve
                        // even during an ability's resolution); the energy floats in the Rune Pool, as in Game.pay
                        g_.p[c].pool_e += 1;
                    }
                    g_.recycle_rune(c, r)?;
                }
                none()
            });
            let pid = info.map().g("pid").int() as usize;
            qt(g, pid, "Sigil of the Storm", res, Map::new(), None, false, Value::None, Value::None);
        }
        none()
    })
}

fn star_spring() -> Value {
    Value::func("_star_spring", |g, a| {
        // "The first time a player plays a non-token unit here each turn, they may move another unit they control
        // here to its base."
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), &a[2]);
        if ev != "played" {
            return none();
        }
        let card = info.map().g("card").obj();
        if g.spec(card).type_ == "Unit" && !g.o(card).token && g.o(card).loc == Some(Loc::Bf(b)) {
            let pid = info.map().g("pid").int() as usize;
            let key = format!("('star', {b}, {pid}, {})", g.turn_no);
            if g.stats.0.iter().any(|(k, _)| *k == key) {
                return none();
            }
            g.stats.add(&key, 1);
            let played = g.o(card).uid;
            let choose = Value::func("choose", move |g_, a| {
                // comparer par uid : la table rejoue la partie depuis une copie profonde (choix humain), « is » y échoue
                let it = a[0].item();
                let c = ctrl(g_, it);
                let us: Vec<O> =
                    g_.units(Some(c), Some(Loc::Bf(b))).into_iter().filter(|&u| g_.o(u).uid != played).collect();
                if us.is_empty() {
                    return Ok(false.into());
                }
                let u = g_.ask(c, "star_spring", us.into_iter().map(Value::Obj).collect(), Map::new())?.obj();
                g_.add_target(it, u, Value::func("pred", move |g2, a2| Ok((g2.o(a2[1].obj()).loc == Some(Loc::Bf(b))).into())));
                Ok(true.into())
            });
            let res = Value::func("res", |g_, a| {
                let it = a[0].item();
                if let Some(u) = g_.legal(it, 0)? {
                    let c = ctrl(g_, it);
                    g_.move_(&[u], Loc::Base, Some(c), false)?;
                }
                none()
            });
            qt(g, pid, "Star Spring", res, Map::new(), None, true, choose, Value::None);
        }
        none()
    })
}

fn targon() -> Value {
    Value::func("_targon", |g, a| {
        // "When you conquer here, ready 2 runes at the end of this turn."
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), &a[2]);
        if ev == "conquer" && info.map().g("bf").int() as usize == b {
            let pid = info.map().g("pid").int() as usize;
            let res = Value::func("res", move |g_, _a| {
                let at_end = Value::func("at_end", move |g2, a2| {
                    let eid = a2[0].int() as u64;
                    g2.remove_effect(eid);
                    let ready2 = Value::func("ready2", |g3, a3| {
                        let it3 = a3[0].item();
                        let c = ctrl(g3, it3);
                        let mut ex: Vec<R> = g3.p[c].runes.iter().copied().filter(|&r| g3.r(r).exhausted).collect();
                        let doms: BTreeSet<String> = ex.iter().map(|&r| g3.r(r).domain.clone()).collect();
                        if full_choices(g3, Some(c)) && ex.len() > 2 && doms.len() > 1 {
                            ex = vec![]; // a human chooses which runes (by domain)
                            for _ in 0..2 {
                                let opts: Vec<Value> =
                                    g3.p[c].runes.iter().copied().filter(|&r| g3.r(r).exhausted).map(Value::Rune).collect();
                                let r = g3.ask(c, "ready_rune", opts, Map::new())?.rune();
                                g3.r_mut(r).exhausted = false;
                            }
                        }
                        for &r in ex.iter().take(2) {
                            g3.r_mut(r).exhausted = false;
                        }
                        none()
                    });
                    g2.queue_trigger(pid, "Targon's Peak (end of turn)", ready2, Map::new(), None, false, Value::None, Value::None);
                    none()
                });
                if g_.stage == "expire" {
                    return none();
                }
                let mut d = Map::new();
                d.set("on", "end_turn");
                d.set("fn", at_end);
                d.set("dur", "turn");
                d.set("pid", pid);
                g_.add_effect(d);
                none()
            });
            qt(g, pid, "Targon's Peak", res, Map::new(), None, false, Value::None, Value::None);
        }
        none()
    })
}

fn threshold() -> Value {
    Value::func("_threshold", |g, a| {
        // "When combat starts here, the attacker and defender each [Add] 1 energy." (Add resolves immediately)
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), &a[2]);
        if ev == "combat_start" && info.map().g("bf").int() as usize == b {
            let sd = info.map().g("sd");
            let (att, def) = (sd.map().g("attacker").int() as usize, sd.map().g("defender").int() as usize);
            g.p[att].pool_e += 1;
            g.p[def].pool_e += 1;
            g.log(|| "  Threshold of the Gray: both players add 1 energy".to_string());
        }
        none()
    })
}

// ---------------------------------------------------------------------------------------------- deck Akali
fn dw_event() -> Value {
    Value::func("_dw_event", |g, a| {
        // Akali, Deadly Weapon — "When I move, you may deal 1 to a unit at a battlefield I moved to or from.
        // If I'm [Empowered], deal 2 instead."
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "move" && is_obj(info, "obj", o) {
            let locs: Vec<Loc> = [info.map().g("frm"), info.map().g("to")]
                .iter()
                .filter_map(loc_v)
                .filter(|l| l.is_bf())
                .collect();
            if locs.is_empty() {
                return none();
            }
            let dmg: i64 = if g.o(o).empowered { 2 } else { 1 };
            let l1 = locs.clone();
            let opts = Value::func("opts", move |g_, a| {
                let it = a[0].item();
                let c = ctrl(g_, it);
                let us: Vec<O> =
                    all_units(g_, c, false, None)?.into_iter().filter(|&u| g_.o(u).loc.is_some_and(|l| l1.contains(&l))).collect();
                Ok(us.into())
            });
            let l2 = locs.clone();
            let pred = Value::func("pred", move |g_, a| Ok(g_.o(a[1].obj()).loc.is_some_and(|l| l2.contains(&l)).into()));
            let res = Value::func("res", |g_, a| {
                let it = a[0].item();
                if let Some(u) = g_.legal(it, 0)? {
                    let d = g_.it(it).data.g("dmg").int();
                    let c = ctrl(g_, it);
                    g_.deal(Some(u), d, "ability", Some(c), None)?;
                }
                none()
            });
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            let choose = trig_target(opts, pred, "target", false, true);
            qt(g, oc, "Akali, Deadly Weapon", res, map! { dmg = dmg }, Some(uid), true, choose, Value::None);
        }
        none()
    })
}

fn akali_deadly_weapon() -> Impl {
    let mut empower = Map::new();
    empower.set("name", "Empower");
    empower.set("timing", "main");
    empower.set("can", Value::func("can", |g, a| Ok((!g.o(a[1].obj()).empowered).into())));
    empower.set("cost", Value::func("cost", |_g, _a| Ok(cost_v(2, vec![req("Fury")]))));
    empower.set(
        "resolve",
        Value::func("resolve", |g, a| {
            if let Some(me) = src_obj(g, a[0].item()) {
                g.empower(Some(me), None)?;
            }
            none()
        }),
    );
    Impl {
        on_event: dw_event(),
        might_mod: Value::func("might_mod", |g, a| Ok(Value::Int(if g.o(a[0].obj()).empowered { 1 } else { 0 }))),
        abilities: vec![empower],
        ..Impl::new("Akali, Deadly Weapon")
    }
}

/// trigger « donne +n might ce tour à la source » (g_.obj(it.src) is not None and g_.mod(g_.obj(it.src), n))
fn mod_src(n: i64) -> Value {
    Value::func("mod_src", move |g_, a| {
        if let Some(me) = src_obj(g_, a[0].item()) {
            g_.mod_(me, n, "turn".into(), None)?;
        }
        none()
    })
}

/// trigger « pioche 1 » (g_.draw(it.ctrl, 1))
fn draw1() -> Value {
    Value::func("draw1", |g_, a| {
        let c = ctrl(g_, a[0].item());
        g_.draw(c, 1)?;
        none()
    })
}

fn silent_event() -> Value {
    Value::func("_silent_event", |g, a| {
        // Akali, Silent — "When I move to a battlefield, give me +2 might this turn."
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "move" && is_obj(info, "obj", o) && loc_v(&info.map().g("to")).is_some_and(|l| l.is_bf()) {
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            qt(g, oc, "Akali, Silent", mod_src(2), Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

/// `sorted(g_.gear(), key=lambda x: x.ctrl == it.ctrl)`
fn gear_opts() -> Value {
    Value::func("gear_opts", |g_, a| {
        let c = ctrl(g_, a[0].item());
        let mut gs = g_.gear(None);
        gs.sort_by_key(|&x| g_.o(x).ctrl == c);
        Ok(gs.into())
    })
}

fn adaptatron() -> Value {
    Value::func("_adaptatron", |g, a| {
        // "When I conquer, you may kill a gear. If you do, buff me."
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "conquer" && info.map().g("pid").int() as usize == g.o(o).ctrl && in_list(info, "units", o) {
            let res = Value::func("res", |g_, a| {
                let it = a[0].item();
                let gg = g_.legal(it, 0)?;
                if let Some(gg) = gg {
                    let c = ctrl(g_, it);
                    if !g_.kill(&[gg], Some(c), false)?.is_empty() {
                        if let Some(me) = src_obj(g_, it) {
                            g_.buff(Some(me), None)?;
                        }
                    }
                }
                none()
            });
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            let choose = trig_target(gear_opts(), p_gear_v(), "target", false, true);
            qt(g, oc, "Adaptatron", res, Map::new(), Some(uid), true, choose, Value::None);
        }
        none()
    })
}

// Against the Odds — "[Reaction] Give a friendly unit at a battlefield +2 might this turn for each enemy unit there."
fn ato() -> Value {
    Value::func("_ato", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            let c = ctrl(g, it);
            let n = units_at(g, Some(1 - c), g.o(u).loc).len() as i64;
            g.mod_(u, 2 * n, "turn".into(), None)?;
        }
        none()
    })
}

fn heron_event() -> Value {
    Value::func("_heron_event", |g, a| {
        // Astral Heron — "When you play your first card each turn, if I'm at a battlefield, the next card you play this
        // turn costs [2] and [A][A] less." (errata du 2026-08-03 : « this turn » ; avant, la réduction attendait la carte
        // suivante même au tour adverse). dur="turn" : retirée à la fin du tour (expire).
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "played"
            && info.map().g("pid").int() as usize == g.o(o).ctrl
            && info.map().g("n") == Value::Int(1)
            && is_bf(g.o(o).loc)
            && !g.o(info.map().g("card").obj()).token
        {
            let pid = g.o(o).ctrl;
            let f = Value::func("heron_res", move |g_, _a| {
                g_.add_effect(map! { kind = "heron", pid = pid, dur = "turn" });
                none()
            });
            let uid = g.o(o).uid;
            qt(g, pid, "Astral Heron", f, Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

// Back Off — "[Hidden] [Action] [Stun] a unit. If you played this from your hand, draw 1."
fn back_off() -> Value {
    Value::func("_back_off", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        if let Some(u) = g.legal(it, 0)? {
            g.stun(Some(u), Some(c))?;
        }
        if g.it(it).data.g("from").opt_str() == Some("hand") {
            g.draw(c, 1)?;
        }
        none()
    })
}

fn back_off_choices() -> Value {
    Value::func("_back_off_choices", |g, a| {
        // "[Stun] a unit" : n'importe quelle unité, amie ou ennemie, même déjà étourdie ; depuis Hidden, une unité de ce
        // battlefield (811.1.d). L'IA garde sa liste courte : ennemis pas encore étourdis.
        let (pid, hb) = (pid_of(&a[0]), hbf(&a[1]));
        if full_choices(g, Some(pid)) {
            let us = all_units(g, pid, false, hb)?;
            return Ok(tgs(g, &us));
        }
        let us: Vec<O> = enemies(g, pid, false, hb, true)?.into_iter().filter(|&u| !g.o(u).stunned).collect();
        Ok(tgs(g, &us))
    })
}

// Blitzcrank, Impassive — "[Tank] When you play me to a battlefield, you may move an enemy unit to here.
// When I hold, return me to my owner's hand."
fn blitz_play() -> Value {
    Value::func("_blitz_play", |g, a| {
        let o = a[0].obj();
        let Some(here) = g.o(o).loc.filter(|l| l.is_bf()) else { return none() };
        let res = Value::func("res", move |g_, a| {
            let it = a[0].item();
            if let Some(u) = g_.legal(it, 0)? {
                let c = ctrl(g_, it);
                g_.move_(&[u], here, Some(c), false)?;
            }
            none()
        });
        let opts = Value::func("opts", move |g_, a| {
            let c = ctrl(g_, a[0].item());
            let us: Vec<O> = enemies(g_, c, false, None, true)?.into_iter().filter(|&u| g_.o(u).loc != Some(here)).collect();
            Ok(us.into())
        });
        let pred = Value::func("pred", |g_, a| Ok((g_.o(a[1].obj()).ctrl != ctrl(g_, a[0].item())).into()));
        let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
        let choose = trig_target(opts, pred, "target", false, true);
        qt(g, oc, "Blitzcrank", res, Map::new(), Some(uid), true, choose, Value::None);
        none()
    })
}

fn blitz_event() -> Value {
    Value::func("_blitz_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "hold" && info.map().g("pid").int() as usize == g.o(o).ctrl && in_list(info, "units", o) {
            let f = Value::func("blitz_hold", |g_, a| {
                if let Some(me) = src_obj(g_, a[0].item()) {
                    g_.to_zone(me, "hand", false, None)?;
                }
                none()
            });
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            qt(g, oc, "Blitzcrank (hold)", f, Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

// Block — "[Hidden] [Action] Give a unit [Shield 3] and [Tank] this turn."
fn block() -> Value {
    Value::func("_block", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            g.grant(u, "Shield", 3, "turn".into());
            g.grant(u, "Tank", 1, "turn".into());
        }
        none()
    })
}

// Brittle Steel — "Kill a gear. [Flow] [4][Fury]"
fn kill_gear() -> Value {
    Value::func("_kill_gear", |g, a| {
        let it = a[0].item();
        if let Some(gg) = g.legal(it, 0)? {
            let c = ctrl(g, it);
            g.kill(&[gg], Some(c), false)?;
        }
        none()
    })
}

// Charm — "Move an enemy unit."
fn charm() -> Value {
    Value::func("_charm", |g, a| {
        let it = a[0].item();
        let dest = loc_v(&g.it(it).data.g("dest"));
        if let Some(u) = g.legal(it, 0)? {
            if g.o(u).loc != dest {
                let c = ctrl(g, it);
                g.move_(&[u], dest.expect("Charm : destination"), Some(c), false)?;
            }
        }
        none()
    })
}

fn charm_choices() -> Value {
    Value::func("_charm_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let mut out: Vec<Map> = vec![];
        for u in enemies(g, pid, false, None, true)? {
            for d in g.locs() {
                if Some(d) != g.o(u).loc {
                    out.push(map! { tg = tup(vec![g.o(u).uid.into()]), dest = d });
                }
            }
        }
        out.sort_by_key(|c| loc_v(&c.g("dest")) != Some(Loc::Base));
        Ok(maps(out))
    })
}

// Crumbling Sands — "[Reaction] Counter a spell if an opponent has played another spell this turn."
fn crumbling() -> Value {
    Value::func("_crumbling", |g, a| {
        let it = a[0].item();
        let iid = g.it(it).data.g("item").opt_int();
        let Some(tgt) = iid.and_then(|i| item_by_id(g, i)) else { return none() };
        let opp = 1 - ctrl(g, it);
        let n = g.spells_played[opp] - if g.it(tgt).ctrl == opp { 1 } else { 0 };
        if n >= 1 {
            g.counter(tgt)?;
        }
        none()
    })
}

// Darius, Trifarian — "When you play your second card in a turn, give me +2 might this turn and ready me."
fn darius() -> Value {
    Value::func("_darius", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "played" && info.map().g("pid").int() as usize == g.o(o).ctrl && info.map().g("n") == Value::Int(2) {
            let res = Value::func("res", |g_, a| {
                if let Some(me) = src_obj(g_, a[0].item()) {
                    g_.mod_(me, 2, "turn".into(), None)?;
                    g_.ready_obj(me, None, None)?;
                }
                none()
            });
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            qt(g, oc, "Darius", res, Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

// Decree of Focus — "[Reaction] Choose a friendly unit that's in combat with an enemy Fury unit or that's being
// chosen by an enemy Fury spell. Give it +4 might this turn."
fn dof_ok(g: &Game, pid: usize, u: O) -> bool {
    if g.in_combat(u)
        && units_at(g, Some(1 - pid), g.o(u).loc).into_iter().any(|e| g.o(e).desig.is_some() && has_domain(g, e, "Fury"))
    {
        return true;
    }
    let uid = g.o(u).uid;
    g.chain.iter().any(|&i| {
        let x = g.it(i);
        x.kind == "spell" && x.ctrl != pid && has_domain(g, x.card.unwrap(), "Fury") && x.chosen.contains(&uid)
    })
}

/// `resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), n)`
fn mod_target(n: i64) -> Value {
    Value::func("mod_target", move |g, a| {
        let it = a[0].item();
        if g.legal(it, 0)?.is_some() {
            if let Some(u) = g.legal(it, 0)? {
                g.mod_(u, n, "turn".into(), None)?;
            }
        }
        none()
    })
}

// Defy — "[Reaction] Counter a spell that costs no more than 4 energy and no more than 1 rune of any type."
fn defy() -> Value {
    Value::func("_defy", |g, a| {
        let it = a[0].item();
        let iid = g.it(it).data.g("item").opt_int();
        if let Some(tgt) = iid.and_then(|i| item_by_id(g, i)) {
            let x = g.it(tgt);
            if x.kind == "spell" {
                let c = x.card.unwrap();
                if g.spec(c).e <= 4 && g.spec(c).p <= 1 {
                    g.counter(tgt)?;
                }
            }
        }
        none()
    })
}

// Disarming Rake — "When you play me, you may kill a gear."
fn rake() -> Value {
    Value::func("_rake", |g, a| {
        let o = a[0].obj();
        let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
        let choose = trig_target(gear_opts(), p_gear_v(), "target", false, true);
        qt(g, oc, "Disarming Rake", kill_gear(), Map::new(), Some(uid), true, choose, Value::None);
        none()
    })
}

// Discipline — "[Reaction] Give a unit +2 might this turn. Draw 1."
fn discipline() -> Value {
    Value::func("_discipline", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            g.mod_(u, 2, "turn".into(), None)?;
        }
        let c = ctrl(g, it);
        g.draw(c, 1)?;
        none()
    })
}

// En Garde — "[Reaction] Give a friendly unit +1 might this turn, then an additional +1 might this turn if it is
// the only unit you control there."
fn en_garde() -> Value {
    Value::func("_en_garde", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            g.mod_(u, 1, "turn".into(), None)?;
            if g.alone(u, None) {
                g.mod_(u, 1, "turn".into(), None)?;
            }
        }
        none()
    })
}

/// `tg_choices(friends(g, pid))`
fn friends_choices() -> Value {
    Value::func("friends_choices", |g, a| {
        let us = friends(g, pid_of(&a[0]), false, None)?;
        Ok(tgs(g, &us))
    })
}

// Falling Star — "Deal 3 to a unit. Deal 3 to a unit."
fn falling_star() -> Value {
    Value::func("_falling_star", |g, a| {
        let it = a[0].item();
        for i in 0..2 {
            if let Some(u) = g.legal(it, i)? {
                let c = ctrl(g, it);
                g.deal(Some(u), 3, "spell", Some(c), None)?;
            }
        }
        none()
    })
}

fn fs_choices() -> Value {
    Value::func("_fs_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let es0 = enemies(g, pid, false, None, true)?;
        let es = cap(g, es0, 4, Some(pid));
        let mut out: Vec<(f64, Map)> = vec![];
        for (i, &x) in es.iter().enumerate() {
            for &y in &es[i..] {
                let ch = map! { tg = tup(vec![g.o(x).uid.into(), g.o(y).uid.into()]) };
                // score(c)
                let s = if x == y {
                    let left = g.might(x)? - g.o(x).damage;
                    let v = value(g, x)?;
                    -(if left > 3 { v } else { v - 3.0 })
                } else {
                    let lx = g.might(x)? - g.o(x).damage <= 3;
                    let vx = value(g, x)?;
                    let ly = g.might(y)? - g.o(y).damage <= 3;
                    let vy = value(g, y)?;
                    -(vx * (lx as i64 as f64) + vy * (ly as i64 as f64))
                };
                out.push((s, ch));
            }
        }
        out.sort_by(|p, q| p.0.partial_cmp(&q.0).expect("clé de tri NaN"));
        Ok(maps(out.into_iter().map(|x| x.1).collect()))
    })
}

// Ferrous Forerunner — "[Deathknell] Play two 3 might Mech unit tokens to your base."
fn forerunner_dk() -> Value {
    Value::func("_forerunner_dk", |g, a| {
        let c = ctrl(g, a[0].item());
        for _ in 0..2 {
            make_token(g, "Mech", c, Loc::Base, false)?;
        }
        none()
    })
}

// Irelia, Fervent — "[Deflect] When you choose or ready me, give me +1 might this turn."
fn irelia() -> Value {
    Value::func("_irelia", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        let chosen = ev == "chosen" && is_obj(info, "obj", o) && g.it(info.map().g("item").item()).ctrl == g.o(o).ctrl;
        if chosen || (ev == "ready" && is_obj(info, "obj", o)) {
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            qt(g, oc, "Irelia", mod_src(1), Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

// Kai'Sa, Survivor — "[Accelerate] When I conquer, draw 1."
fn kaisa() -> Value {
    Value::func("_kaisa", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "conquer" && info.map().g("pid").int() as usize == g.o(o).ctrl && in_list(info, "units", o) {
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            qt(g, oc, "Kai'Sa", draw1(), Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

fn pendulum_effect() -> Value {
    Value::func("_pendulum_effect", |g, a| {
        // Effect text (attached): the equipped unit gets +2 might this turn when it moves to a battlefield.
        let (unit, ev, info) = (a[1].obj(), a[2].str(), &a[3]);
        if ev == "move" && is_obj(info, "obj", unit) && loc_v(&info.map().g("to")).is_some_and(|l| l.is_bf()) {
            let (uc, uid) = (g.o(unit).ctrl, g.o(unit).uid);
            qt(g, uc, "Pendulum Blade", mod_src(2), Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

// Mischievous Marai — "[Hidden] When you play me to a battlefield, deal 2 to an enemy unit here."
fn marai() -> Value {
    Value::func("_marai", |g, a| {
        let o = a[0].obj();
        let Some(here) = g.o(o).loc.filter(|l| l.is_bf()) else { return none() };
        let res = Value::func("res", |g_, a| {
            let it = a[0].item();
            if let Some(u) = g_.legal(it, 0)? {
                let c = ctrl(g_, it);
                g_.deal(Some(u), 2, "ability", Some(c), None)?;
            }
            none()
        });
        let choose = trig_target(enemies_here(Some(here)), pred_here_enemy(Some(here)), "target", false, true);
        let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
        qt(g, oc, "Mischievous Marai", res, Map::new(), Some(uid), false, choose, Value::None);
        none()
    })
}

/// `lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here]`
fn enemies_here(here: Option<Loc>) -> Value {
    Value::func("enemies_here", move |g_, a| {
        let c = ctrl(g_, a[0].item());
        let us: Vec<O> = enemies(g_, c, false, None, true)?.into_iter().filter(|&u| g_.o(u).loc == here).collect();
        Ok(us.into())
    })
}

/// `lambda g_, it, u: u.loc == here and u.ctrl != it.ctrl`
fn pred_here_enemy(here: Option<Loc>) -> Value {
    Value::func("pred_here_enemy", move |g_, a| {
        let (it, u) = (a[0].item(), a[1].obj());
        Ok((g_.o(u).loc == here && g_.o(u).ctrl != ctrl(g_, it)).into())
    })
}

// Mournful Witness — "When a combat that I was in ends, empower me. [Empowered] I have +2 might."
fn witness() -> Value {
    Value::func("_witness", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "combat_end" && in_list(info, "members", o) {
            let f = Value::func("witness_res", |g_, a| {
                if let Some(me) = src_obj(g_, a[0].item()) {
                    g_.empower(Some(me), None)?;
                }
                none()
            });
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            qt(g, oc, "Mournful Witness", f, Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

// Not So Fast — "[Reaction] Counter an enemy spell or ability that chooses a friendly unit or gear."
fn nsf_items(g: &mut Game, pid: usize) -> Res<Vec<ItemId>> {
    let mine: BTreeSet<u32> = g.board.iter().filter(|&&o| g.o(o).ctrl == pid).map(|&o| g.o(o).uid).collect();
    let mut out = vec![];
    for i in g.chain.clone().into_iter().rev() {
        if g.it(i).ctrl != pid && g.it(i).chosen.iter().any(|u| mine.contains(u)) && g.counterable(i)? {
            out.push(i);
        }
    }
    Ok(out)
}

fn nsf() -> Value {
    Value::func("_nsf", |g, a| {
        let it = a[0].item();
        let iid = g.it(it).data.g("item").opt_int();
        if let Some(tgt) = iid.and_then(|i| item_by_id(g, i)) {
            g.counter(tgt)?;
        }
        none()
    })
}

// Scuttle Crab — "When you play me, draw 1. [Deathknell] Choose an opponent. They reveal their hand. You can look
// at their facedown cards this turn. Gain 1 XP."
fn crab_dk() -> Value {
    Value::func("_crab_dk", |g, a| {
        let c = ctrl(g, a[0].item());
        g.reveal_hand(1 - c, c)?;
        g.p[1 - c].revealed_turn = g.turn_no; // cartes face cachée visibles ce tour
        g.gain_xp(c, 1)?;
        none()
    })
}

// Shuriken Flip — "Deal 2 to up to one enemy unit at a battlefield, then move a friendly unit.
// [Flow] [3][A]"
fn flip() -> Value {
    Value::func("_flip", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        let tg = g.it(it).data.g("tg");
        if tg.truthy() && !tg.list()[0].is_none() {
            if let Some(u) = g.legal(it, 0)? {
                g.deal(Some(u), 2, "spell", Some(c), None)?;
            }
        }
        let mover = g.it(it).data.g("mover");
        let m = if mover.truthy() { g.obj(mover.int() as u32) } else { None };
        if let Some(m) = m {
            let dest = loc_v(&g.it(it).data.g("dest"));
            if g.o(m).ctrl == c && Value::Int(g.o(m).oid) == g.it(it).data.g("mover_oid") && g.o(m).loc != dest {
                // `if it.data.get("hidden_bf") is None or True` : toujours vrai
                g.move_(&[m], dest.expect("Shuriken Flip : destination"), Some(c), false)?;
            }
        }
        none()
    })
}

fn flip_choices() -> Value {
    Value::func("_flip_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hbf(&a[1]));
        let es0 = enemies(g, pid, true, hb, true)?;
        let es = cap(g, es0, 3, Some(pid));
        let mut tgs_: Vec<Value> = es.iter().map(|&e| tup(vec![g.o(e).uid.into()])).collect();
        tgs_.push(tup(vec![]));
        let mut fr = friends(g, pid, false, None)?;
        if let Some(h) = hb {
            let here: Vec<O> = fr.iter().copied().filter(|&u| g.o(u).loc == Some(h)).collect();
            if !here.is_empty() {
                fr = here;
            }
        }
        let mut moves: Vec<(u32, i64, Loc)> = vec![];
        for &u in &fr {
            for d in g.locs() {
                if Some(d) != g.o(u).loc {
                    moves.push((g.o(u).uid, g.o(u).oid, d));
                }
            }
        }
        let mut out: Vec<Map> = vec![];
        for t in &tgs_ {
            for &(m, oid, d) in &moves {
                out.push(map! { tg = t.clone(), mover = m, mover_oid = oid, dest = d });
            }
            if moves.is_empty() {
                out.push(map! { tg = t.clone(), mover = Value::None, dest = Value::None });
            }
        }
        // score(c)
        let mut keyed: Vec<(f64, Map)> = vec![];
        for c in out {
            let mut s: f64 = 0.0;
            let tg = c.g("tg");
            if tg.truthy() {
                let e = g.obj(tg.list()[0].int() as u32).expect("Shuriken Flip : cible");
                let low = g.might(e)? - g.o(e).damage <= 2;
                s -= value(g, e)? * (low as i64 as f64) + 1.0;
            }
            let mv = c.g("mover");
            if mv.truthy() {
                let m = g.obj(mv.int() as u32).expect("Shuriken Flip : unité déplacée");
                let dest = loc_v(&c.g("dest"));
                if let Some(Loc::Bf(i)) = dest {
                    if g.bfs[i].ctrl != Some(pid) {
                        s -= 2.0 + g.might(m)? as f64 / 2.0;
                    }
                }
                if matches!(g.cname(m), "Akali, Deadly Weapon" | "Stellacorn Herder") {
                    s -= 1.0;
                }
            }
            keyed.push((s, c));
        }
        keyed.sort_by(|p, q| p.0.partial_cmp(&q.0).expect("clé de tri NaN"));
        Ok(maps(keyed.into_iter().map(|x| x.1).collect()))
    })
}

// Stellacorn Herder — "When I move, draw 1."
fn herder() -> Value {
    Value::func("_herder", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "move" && is_obj(info, "obj", o) {
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            qt(g, oc, "Stellacorn Herder", draw1(), Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

// Tomb-Raider Barbara — "When you play me, if you control 7 or more runes, choose an enemy gear. If it's
// [Empowered], disempower it. Otherwise, kill it."
fn barbara() -> Value {
    Value::func("_barbara", |g, a| {
        let o = a[0].obj();
        let oc = g.o(o).ctrl;
        if g.p[oc].runes.len() < 7 {
            return none();
        }
        let res = Value::func("res", |g_, a| {
            let it = a[0].item();
            let Some(gg) = g_.legal(it, 0)? else { return none() };
            if g_.o(gg).empowered {
                g_.o_mut(gg).empowered = false;
            } else {
                let c = ctrl(g_, it);
                g_.kill(&[gg], Some(c), false)?;
            }
            none()
        });
        let opts = Value::func("opts", |g_, a| {
            let c = ctrl(g_, a[0].item());
            let xs: Vec<O> = g_.gear(None).into_iter().filter(|&x| g_.o(x).ctrl != c).collect();
            Ok(xs.into())
        });
        let pred = Value::func("pred", |g_, a| Ok((g_.o(a[1].obj()).ctrl != ctrl(g_, a[0].item())).into()));
        let choose = trig_target(opts, pred, "target", false, true);
        let uid = g.o(o).uid;
        qt(g, oc, "Tomb-Raider Barbara", res, Map::new(), Some(uid), false, choose, Value::None);
        none()
    })
}

// Zhonya's Hourglass — "[Hidden] If a friendly unit would die, kill this instead. Heal that unit, exhaust it,
// and recall it." (errata; replacement effect applied by Game.kill, rules 370-373). An Hourglass dying in the same
// event doesn't save a unit (as in the original engine).
fn zhonya_rep() -> Value {
    Value::func("_zhonya_rep", |g, a| {
        let (h, u) = (a[0].obj(), a[1].obj());
        if g.o(u).ctrl != g.o(h).ctrl || g.spec(u).type_ != "Unit" || g.dying.contains(&h) {
            return none();
        }
        let apply = Value::func("apply", move |g_, a| {
            let x = a[0].obj();
            let hc = g_.o(h).ctrl;
            g_.stats.add(&format!("zhonya_save_P{hc}"), 1);
            g_.kill(&[h], Some(hc), false)?;
            g_.save_unit(x, true, true);
            none()
        });
        let mut d = Map::new();
        d.set("name", "Zhonya's Hourglass");
        d.set("apply", apply);
        Ok(d.into())
    })
}

// ---------------------------------------------------------------------------------------------- deck LeBlanc
// Baited Hook — "[1][Order], [E]: Kill a friendly unit. Look at the top 5 cards of your Main Deck. You may banish
// a unit from among them that has Might up to 1 more than the killed unit and play it, ignoring its cost.
// Then recycle the rest."
fn hook() -> Value {
    Value::func("_hook", |g, a| {
        let it = a[0].item();
        let pid = ctrl(g, it);
        let u = g.legal(it, 0)?;
        let mut killed_might = None;
        if let Some(u) = u {
            killed_might = Some(g.might(u)?);
            g.kill(&[u], Some(pid), false)?;
        }
        let top5: Vec<O> = g.p[pid].deck.iter().take(5).copied().collect();
        let top = g.look(pid, &top5, None)?;
        let cands: Vec<O> = match killed_might {
            Some(km) => {
                top.iter().copied().filter(|&c| g.spec(c).type_ == "Unit" && spec_might(g, c) <= km + 1).collect()
            }
            None => vec![],
        };
        let cands = sorted_f(g, cands, |g, c| Ok(-(spec_might(g, c) as f64) - g.spec(c).e as f64 / 10.0))?;
        let mut opts = vec![Value::None];
        opts.extend(cands.into_iter().map(Value::Obj));
        let pick = g.ask(pid, "hook_pick", opts, Map::new())?.opt_obj();
        let rest: Vec<O> = top.iter().copied().filter(|&c| Some(c) != pick).collect();
        if let Some(pick) = pick {
            let n = g.cname(pick).to_string();
            g.log(|| format!("  Baited Hook finds {n}"));
            g.stats.add("hook_hit", 1);
            let pl = &mut g.p[pid];
            let i = pl.deck.iter().position(|&c| c == pick).expect("Baited Hook : carte hors du deck");
            pl.deck.remove(i);
            g.o_mut(pick).zone = Some(Zone::Banish);
            g.p[pid].banish.push(pick);
            play_unit_free(g, pid, pick, "banish", None, false, None)?;
        }
        for &c in &rest {
            if let Some(i) = g.p[pid].deck.iter().position(|&x| x == c) {
                g.p[pid].deck.remove(i);
            }
        }
        g.recycle_cards(pid, &rest)?;
        none()
    })
}

fn baited_hook() -> Impl {
    let mut ab = Map::new();
    ab.set("name", "Hook");
    ab.set("timing", "main");
    ab.set("exhaust", true);
    ab.set("preds", Value::List(vec![p_friend_v()]));
    ab.set(
        "choices",
        Value::func("choices", |g, a| {
            let pid = pid_of(&a[0]);
            let us = g.units(Some(pid), None);
            let us = by_value(g, us)?;
            Ok(maps(us.into_iter().map(|u| map! { tg = tup(vec![g.o(u).uid.into()]) }).collect()))
        }),
    );
    ab.set("cost", Value::func("cost", |_g, _a| Ok(cost_v(1, vec![req("Order")]))));
    ab.set("resolve", hook());
    Impl { abilities: vec![ab], ..Impl::new("Baited Hook") }
}

// Bellows Breath — "[Action] [Repeat] [1][Mind] Deal 1 to up to three units at the same location."
fn bellows() -> Value {
    Value::func("_bellows", |g, a| {
        let it = a[0].item();
        let n = g.it(it).targets.len();
        let mut first = vec![];
        for i in 0..n {
            first.push(g.legal(it, i)?);
        }
        let mut groups: Vec<Vec<Option<O>>> = vec![first];
        let t2 = g.it(it).data.g("t2");
        if t2.truthy() {
            let grp: Vec<Option<O>> = t2
                .list()
                .iter()
                .map(|p| {
                    let p = p.list();
                    let (u, oid) = (p[0].int() as u32, p[1].int());
                    g.obj(u).filter(|&o| g.o(o).oid == oid)
                })
                .collect();
            groups.push(grp);
        }
        let c = ctrl(g, it);
        for grp in groups {
            let grp: Vec<O> = grp.into_iter().flatten().collect();
            if grp.is_empty() {
                continue;
            }
            // max(set(u.loc for u in grp), key=nombre d'unités) : à égalité, Python garde le premier de l'ordre
            // d'itération du set (battlefields par indice, puis 'base' le plus souvent) ; ici : battlefields par
            // indice croissant, puis la base.
            let mut locs: Vec<Option<Loc>> = vec![];
            for &u in &grp {
                let l = g.o(u).loc;
                if !locs.contains(&l) {
                    locs.push(l);
                }
            }
            locs.sort_by_key(|l| match l {
                Some(Loc::Bf(i)) => (0, *i),
                Some(Loc::Base) => (1, 0),
                None => (2, 0),
            });
            let mut best = locs[0];
            let mut best_n = 0;
            for &l in &locs {
                let k = grp.iter().filter(|&&u| g.o(u).loc == l).count();
                if k > best_n {
                    best_n = k;
                    best = l;
                }
            }
            for &u in &grp {
                if g.o(u).loc == best {
                    g.deal(Some(u), 1, "spell", Some(c), None)?;
                }
            }
        }
        none()
    })
}

fn bellows_choices() -> Value {
    Value::func("_bellows_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hbf(&a[1]));
        let mut out: Vec<Map> = vec![];
        let mut locs: Vec<Option<Loc>> = vec![];
        for u in enemies(g, pid, false, hb, true)? {
            let l = g.o(u).loc;
            if !locs.contains(&l) {
                locs.push(l);
            }
        }
        locs.sort_by_key(|&l| loc_key(l));
        for l in locs {
            let es: Vec<O> = enemies(g, pid, false, None, true)?.into_iter().filter(|&u| g.o(u).loc == l).take(3).collect();
            out.push(map! { tg = tup(es.iter().map(|&u| g.o(u).uid.into()).collect()) });
        }
        let mut keyed: Vec<(i64, Map)> = vec![];
        for c in out {
            let mut k = 0;
            for u in c.g("tg").list() {
                let o = g.obj(u.int() as u32).expect("Bellows Breath : cible");
                if g.might(o)? - g.o(o).damage <= 1 {
                    k += 1;
                }
            }
            keyed.push((-k, c));
        }
        keyed.sort_by_key(|x| x.0);
        let mut out: Vec<Map> = keyed.into_iter().map(|x| x.1).collect();
        if full_choices(g, Some(pid)) {
            // a human: any one to three units at one location
            let mut ls: Vec<Option<Loc>> = vec![];
            for u in g.all_units() {
                let l = g.o(u).loc;
                if !ls.contains(&l) {
                    ls.push(l);
                }
            }
            ls.sort_by_key(|&l| loc_key(l));
            for l in ls {
                if hb.is_some() && l != hb {
                    continue;
                }
                let mut us = vec![];
                for u in g.all_units() {
                    if g.o(u).loc == l && g.targetable(u, Some(pid))? {
                        us.push(u);
                    }
                }
                us.sort_by_key(|&u| g.o(u).uid);
                for k in 1..=3 {
                    for grp in combinations(us.len(), k) {
                        let ch = map! { tg = tup(grp.iter().map(|&i| g.o(us[i]).uid.into()).collect()) };
                        if !out.contains(&ch) {
                            out.push(ch);
                        }
                    }
                }
            }
        }
        out.push(map! { tg = tup(vec![]) }); // "up to three" includes zero (rule 355.13)
        Ok(maps(out))
    })
}

/// `deathknell=lambda g, it: g.channel(it.ctrl, 1, exhausted=True)`
fn channel1_exhausted() -> Value {
    Value::func("channel1_exhausted", |g, a| {
        let c = ctrl(g, a[0].item());
        g.channel(c, 1, true)?;
        none()
    })
}

// Chakram Dancer — "[Ambush] When you play me, give your other units here [Shield] this turn."
fn chakram() -> Value {
    Value::func("_chakram", |g, a| {
        let o = a[0].obj();
        let here = g.o(o).loc;
        let res = Value::func("res", move |g_, a| {
            let it = a[0].item();
            let c = ctrl(g_, it);
            let src = g_.it(it).src;
            for u in units_at(g_, Some(c), here) {
                if Some(g_.o(u).uid) != src {
                    g_.grant(u, "Shield", 1, "turn".into());
                }
            }
            none()
        });
        let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
        qt(g, oc, "Chakram Dancer", res, Map::new(), Some(uid), false, Value::None, Value::None);
        none()
    })
}

/// « each player kills one of their units » : (g.tp, 1 - g.tp), `sacrifice`, triés par valeur.
fn sacrifice_victims(g: &mut Game, skip: Option<usize>) -> Res<Vec<O>> {
    let mut victims = vec![];
    for pid in [g.tp, 1 - g.tp] {
        if Some(pid) == skip {
            continue;
        }
        let us = g.units(Some(pid), None);
        if !us.is_empty() {
            let us = by_value(g, us)?;
            let v = g.ask(pid, "sacrifice", us.into_iter().map(Value::Obj).collect(), Map::new())?;
            victims.push(v);
        }
    }
    Ok(victims.into_iter().filter_map(|v| v.opt_obj()).collect())
}

// Cull the Weak — "Each player kills one of their units."
fn cull() -> Value {
    Value::func("_cull", |g, _a| {
        let victims = sacrifice_victims(g, None)?;
        g.kill(&victims, None, false)?;
        none()
    })
}

// Deathgrip — "[Reaction] Kill a friendly unit to give +might equal to its Might to another friendly unit this
// turn. Draw 1."  Deux cibles obligatoires (355.7, 355.8) : tg[0] = l'unité alliée tuée, tg[1] = une AUTRE unité
// alliée. Sans deux unités alliées, le sort ne peut pas être joué (décision de l'utilisateur, 2026-10-08 : suivre les
// règles au plus près). À la résolution, une cible devenue illégale ne fait rien, le reste s'applique (359.3.e).
fn deathgrip() -> Value {
    Value::func("_deathgrip", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        let (victim, tgt) = (g.legal(it, 0)?, g.legal(it, 1)?);
        if let Some(victim) = victim {
            let m = g.might(victim)?; // sa Might au moment de mourir
            let killed = g.kill(&[victim], Some(c), false)?;
            if let Some(tgt) = tgt {
                if !killed.is_empty() && tgt != victim && g.on_board(tgt) {
                    g.mod_(tgt, 0.max(m), "turn".into(), None)?;
                }
            }
        }
        g.draw(c, 1)?;
        none()
    })
}

fn deathgrip_choices() -> Value {
    Value::func("_deathgrip_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hbf(&a[1]));
        let fr = friends(g, pid, false, hb)?;
        let v0 = by_value(g, fr.clone())?;
        let victims = cap(g, v0, 3, Some(pid)); // l'IA sacrifie d'abord le moins utile
        let g0 = sorted_f(g, fr, |g, u| Ok(-(g.might(u)? as f64)))?;
        let gets = cap(g, g0, 3, Some(pid));
        let mut out = vec![];
        for &v in &victims {
            for &t in &gets {
                if t != v {
                    out.push(map! { tg = tup(vec![g.o(v).uid.into(), g.o(t).uid.into()]) });
                }
            }
        }
        Ok(maps(out))
    })
}

// Glasc Mixologist — "[Deathknell] You may play a unit with cost no more than 3 energy and no more than 1 rune of
// any type from your trash, ignoring its cost."
fn mixo() -> Value {
    Value::func("_mixo", |g, a| {
        let pid = ctrl(g, a[0].item());
        let cands: Vec<O> = g.p[pid]
            .trash
            .iter()
            .copied()
            .filter(|&c| g.spec(c).type_ == "Unit" && g.spec(c).e <= 3 && g.spec(c).p <= 1)
            .collect();
        let cands = sorted_f(g, cands, |g, c| Ok((-spec_might(g, c) - g.spec(c).e as i64) as f64))?;
        let mut opts = vec![Value::None];
        opts.extend(cands.into_iter().map(Value::Obj));
        if let Some(pick) = g.ask(pid, "mixologist_pick", opts, Map::new())?.opt_obj() {
            play_unit_free(g, pid, pick, "trash", None, false, None)?;
        }
        none()
    })
}

/// `lambda g_, it: g_.kill([g_.legal(it, 0)], it.ctrl)`
fn kill_target() -> Value {
    Value::func("kill_target", |g_, a| {
        let it = a[0].item();
        let u = g_.legal(it, 0)?;
        let c = ctrl(g_, it);
        let us: Vec<O> = u.into_iter().collect();
        g_.kill(&us, Some(c), false)?;
        none()
    })
}

// Harnessed Dragon — "When you play me, kill an enemy unit."
fn dragon() -> Value {
    Value::func("_dragon", |g, a| {
        let (o, ctx) = (a[0].obj(), &a[1]);
        let hb = hbf(ctx);
        let opts = Value::func("opts", move |g_, a| {
            let c = ctrl(g_, a[0].item());
            Ok(enemies(g_, c, false, hb, true)?.into())
        });
        let choose = trig_target(opts, p_enemy_v(), "target", false, true);
        let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
        qt(g, oc, "Harnessed Dragon", kill_target(), Map::new(), Some(uid), false, choose, Value::None);
        none()
    })
}

// Hidden Blade — "[Hidden] [Action] Kill a unit at a battlefield. Its controller draws 2."
fn blade() -> Value {
    Value::func("_blade", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else {
            return none(); // linked instruction ignored too (rule 359.3.e.14.a)
        };
        let c = g.o(u).ctrl;
        let ic = ctrl(g, it);
        g.kill(&[u], Some(ic), false)?;
        g.draw(c, 2)?;
        none()
    })
}

// Honest Broker — "[Deathknell] Play a Gold gear token exhausted."
fn broker() -> Value {
    Value::func("_broker", |g, a| {
        let c = ctrl(g, a[0].item());
        make_token(g, "Gold", c, Loc::Base, false)?;
        none()
    })
}

// Kennen, Keeper of Balance — "[Hidden] When you play me or I attack, you may pay [2] to [Stun] a unit.
// While there's a stunned enemy unit here, I have +2 might."
fn kennen_trigger(g: &mut Game, o: O, hb: Option<Loc>) {
    let res = Value::func("res", |g_, a| {
        let it = a[0].item();
        if let Some(u) = g_.legal(it, 0)? {
            let c = ctrl(g_, it);
            g_.stun(Some(u), Some(c))?;
        }
        none()
    });
    let opts = Value::func("opts", move |g_, a| {
        let c = ctrl(g_, a[0].item());
        let mut us: Vec<O> = enemies(g_, c, false, hb, true)?.into_iter().filter(|&u| !g_.o(u).stunned).collect();
        // `+ [u for u in friends(g_, it.ctrl) if hb is None or u.loc == hb][:0]` : liste vide
        let fr = friends(g_, c, false, None)?;
        us.extend(fr.into_iter().filter(|&u| hb.is_none() || g_.o(u).loc == hb).take(0));
        Ok(us.into())
    });
    let choose = trig_target(opts, p_unit_v(), "target", false, true);
    let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
    qt(g, oc, "Kennen", res, Map::new(), Some(uid), true, choose, may_pay(2, Value::None));
}

// Ki Barrier — "[Reaction] Choose a unit. Prevent the next 7 damage that would be dealt to it this turn."
fn ki() -> Value {
    Value::func("_ki", |g, a| {
        if let Some(u) = g.legal(a[0].item(), 0)? {
            g.o_mut(u).prevent += 7;
        }
        none()
    })
}

// Rift Herald — "When I move to a battlefield, look at the top 3 cards of your Main Deck. You may reveal a unit
// from among them and draw it. Recycle the rest. [Deathknell] Play a unit from your hand to your base, ignoring
// its Energy cost. (You must still pay its Power cost.)"
fn herald_move() -> Value {
    Value::func("_herald_move", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "move" && is_obj(info, "obj", o) && loc_v(&info.map().g("to")).is_some_and(|l| l.is_bf()) {
            let res = Value::func("res", |g_, a| {
                let it = a[0].item();
                let c = ctrl(g_, it);
                let top3: Vec<O> = g_.p[c].deck.iter().take(3).copied().collect();
                let top = g_.look(c, &top3, None)?;
                let us: Vec<O> = top.iter().copied().filter(|&x| g_.spec(x).type_ == "Unit").collect();
                let mut opts = vec![Value::None];
                opts.extend(us.into_iter().map(Value::Obj));
                let pick = g_.ask(c, "herald_pick", opts, Map::new())?.opt_obj();
                let rest: Vec<O> = top.iter().copied().filter(|&x| Some(x) != pick).collect();
                if let Some(pick) = pick {
                    g_.draw_card(c, pick)?;
                }
                for &x in &rest {
                    let i = g_.p[c].deck.iter().position(|&y| y == x).expect("Rift Herald : carte hors du deck");
                    g_.p[c].deck.remove(i);
                }
                g_.recycle_cards(c, &rest)?;
                none()
            });
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            qt(g, oc, "Rift Herald", res, Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

fn herald_dk() -> Value {
    Value::func("_herald_dk", |g, a| {
        let pid = ctrl(g, a[0].item());
        let mut cands = vec![];
        for c in g.p[pid].hand.clone() {
            if g.spec(c).type_ == "Unit" {
                let reqs = g.power_reqs(&g.spec(c).domains, g.spec(c).p as usize);
                if g.can_pay(pid, 0, &reqs, &Value::None)? {
                    cands.push(c);
                }
            }
        }
        cands.sort_by_key(|&c| -g.spec(c).e);
        let pick = g.ask(pid, "herald_dk_pick", cands.into_iter().map(Value::Obj).collect(), Map::new())?.opt_obj();
        if let Some(pick) = pick {
            play_unit_free(g, pid, pick, "hand", Some(vec![Loc::Base]), true, None)?;
        }
        none()
    })
}

// Ruined Rex — "[Deathknell] Deal 4 to an enemy unit."
fn rex_dk() -> Value {
    Value::func("_rex_dk", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            let c = ctrl(g, it);
            g.deal(Some(u), 4, "ability", Some(c), None)?;
        }
        none()
    })
}

/// `trig_target(lambda g_, it: enemies(g_, it.ctrl), P_enemy)`
fn enemies_opts() -> Value {
    Value::func("enemies_opts", |g_, a| {
        let c = ctrl(g_, a[0].item());
        Ok(enemies(g_, c, false, None, true)?.into())
    })
}

// Safety Inspector — "You may spend 3 XP as an additional cost to play me. When you play me, each player must kill
// one of their units. If you paid my additional cost, you don't kill a unit this way."
fn inspector() -> Value {
    Value::func("_inspector", |g, a| {
        let (o, ctx) = (a[0].obj(), &a[1]);
        let paid = ctx_get(ctx, "xp").truthy();
        let res = Value::func("res", move |g_, a| {
            let c = ctrl(g_, a[0].item());
            let victims = sacrifice_victims(g_, if paid { Some(c) } else { None })?;
            g_.kill(&victims, None, false)?;
            none()
        });
        let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
        qt(g, oc, "Safety Inspector", res, Map::new(), Some(uid), false, Value::None, Value::None);
        none()
    })
}

// Salvage — "[Action] You may kill a gear. Draw 1."
fn salvage() -> Value {
    Value::func("_salvage", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        if !g.it(it).targets.is_empty() {
            if let Some(gg) = g.legal(it, 0)? {
                g.kill(&[gg], Some(c), false)?;
            }
        }
        g.draw(c, 1)?;
        none()
    })
}

// Thousand-Tailed Watcher — "[Accelerate] When you play me, give enemy units -3 might this turn, to a minimum of 1."
fn watcher() -> Value {
    Value::func("_watcher", |g, a| {
        let o = a[0].obj();
        let res = Value::func("res", |g_, a| {
            let c = ctrl(g_, a[0].item());
            for u in g_.units(Some(1 - c), None) {
                g_.mod_(u, -3, "turn".into(), Some(1))?;
            }
            none()
        });
        let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
        qt(g, oc, "Thousand-Tailed Watcher", res, Map::new(), Some(uid), false, Value::None, Value::None);
        none()
    })
}

// Vi, Peacekeeper — "[Ambush] When I attack, [Stun] an enemy unit here."
fn vi() -> Value {
    Value::func("_vi", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "attack" && is_obj(info, "obj", o) {
            let here = g.o(o).loc;
            let res = Value::func("vi_res", |g_, a| {
                let it = a[0].item();
                let u = g_.legal(it, 0)?;
                let c = ctrl(g_, it);
                g_.stun(u, Some(c))?;
                none()
            });
            let choose = trig_target(enemies_here(here), pred_here_enemy(here), "target", false, true);
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            qt(g, oc, "Vi", res, Map::new(), Some(uid), false, choose, Value::None);
        }
        none()
    })
}

// Ashe, Focused — "When you play me, choose an opponent. They reveal their hand. Choose a card revealed this way
// and banish it. When they hold, return it to their hand (even if I'm no longer on the board)."
fn ashe() -> Value {
    Value::func("_ashe", |g, a| {
        let o = a[0].obj();
        let res = Value::func("res", |g_, a| {
            let c = ctrl(g_, a[0].item());
            let opp = 1 - c;
            g_.reveal_hand(opp, c)?;
            let mut hand = g_.p[opp].hand.clone();
            if hand.is_empty() {
                return none();
            }
            hand.sort_by_key(|&x| -(g_.spec(x).e + 2 * g_.spec(x).p));
            let cd = g_.ask(c, "ashe_pick", hand.into_iter().map(Value::Obj).collect(), Map::new())?.obj();
            g_.to_zone(cd, "banish", false, None)?;
            let back = Value::func("back", move |g2, a2| {
                // relire la carte par uid/oid : la table (choix humain, conseil) et l'IA (clone) travaillent sur une copie
                // profonde de la partie, où « c » capturé n'est plus l'objet de l'exil
                let eid = a2[0].int() as u64;
                let info = &a2[1];
                let e = g2.eff(eid).cloned().unwrap_or_default();
                let (uid, oid) = (e.g("uid").int() as u32, e.g("oid").int());
                let x = g2.p[opp].banish.iter().copied().find(|&y| g2.o(y).uid == uid && g2.o(y).oid == oid);
                if info.map().g("pid").int() as usize == opp {
                    if let Some(x) = x {
                        g2.remove_effect(eid);
                        let i = g2.p[opp].banish.iter().position(|&y| y == x).unwrap();
                        g2.p[opp].banish.remove(i);
                        g2.o_mut(x).zone = Some(Zone::Hand);
                        g2.p[opp].hand.push(x);
                    }
                }
                none()
            });
            let mut d = Map::new();
            d.set("on", "hold");
            d.set("fn", back);
            d.set("uid", g_.o(cd).uid);
            d.set("oid", g_.o(cd).oid);
            g_.add_effect(d);
            none()
        });
        let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
        qt(g, oc, "Ashe", res, Map::new(), Some(uid), false, Value::None, Value::None);
        none()
    })
}

// Atakhan — "You may kill a friendly unit as an additional cost to play me. If you do, I cost [1] less for each
// Energy it costs and [Order] less for each Power it costs. [Ganking] When I attack, the defender must kill one of
// their units here."
fn atakhan_cost() -> Value {
    Value::func("_atakhan_cost", |g, a| {
        let ch = &a[2];
        let kv = ctx_get(ch, "kill");
        let k = if kv.truthy() { g.obj(kv.int() as u32) } else { None };
        let Some(k) = k else { return Ok(tup(vec![0.into(), 0.into()])) };
        let real = !g.o(k).token || g.o(k).copy.as_deref().is_some_and(|c| !c.is_empty());
        let (e, p) = if real { (g.spec(k).e, g.spec(k).p) } else { (0, 0) };
        Ok(tup(vec![e.into(), p.into()]))
    })
}

fn atakhan_event() -> Value {
    Value::func("_atakhan_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
        if ev == "attack" && is_obj(info, "obj", o) {
            let here = g.o(o).loc;
            let res = Value::func("res", move |g_, a| {
                let d = 1 - ctrl(g_, a[0].item());
                let us = units_at(g_, Some(d), here);
                if !us.is_empty() {
                    let us = by_value(g_, us)?;
                    let v = g_.ask(d, "sacrifice", us.into_iter().map(Value::Obj).collect(), Map::new())?;
                    let vs: Vec<O> = v.opt_obj().into_iter().collect();
                    g_.kill(&vs, Some(d), false)?;
                }
                none()
            });
            let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
            qt(g, oc, "Atakhan", res, Map::new(), Some(uid), false, Value::None, Value::None);
        }
        none()
    })
}

// ---------------------------------------------------------------------------------------------- enregistrement
fn f(name: &'static str, x: impl Fn(&mut Game, &[Value]) -> Res<Value> + Send + Sync + 'static) -> Value {
    Value::func(name, x)
}

fn kw1(k: &str) -> Map {
    let mut m = Map::new();
    m.set(k, 1);
    m
}

/// `flow=(e, p, domaines)`
fn flow(e: i64, p: i64, doms: &[&str]) -> Value {
    tup(vec![e.into(), p.into(), tup(doms.iter().map(|&d| Value::from(d)).collect())])
}

pub fn register(r: &mut Registry) {
    let card = |r: &mut Registry, im: Impl| r.card(im);
    let plain = |r: &mut Registry, n: &str| r.card(Impl::new(n));

    card(r, akali_rogue_assassin());
    card(r, Impl { legend_event: leblanc_legend(), ..Impl::new("LeBlanc, Deceiver") });

    // ================================================================== battlefields
    card(r, Impl { bf_event: bar(), ..Impl::new("Back-Alley Bar") });
    card(r, Impl { bf_event: dusk(), ..Impl::new("Dusk Rose Lab") });
    plain(r, "Forbidding Waste"); // passive in Game.might
    card(
        r,
        Impl {
            no_score: f("no_score", |g, a| Ok((g.p[pid_of(&a[0])].turns < 3).into())),
            ..Impl::new("Forgotten Monument")
        },
    );
    plain(r, "Void Gate"); // Bonus Damage in Game.deal
    plain(r, "Windswept Hillock"); // Ganking in Game.kw_value
    plain(r, "Aspirant's Climb"); // victory score +1 in Game.__init__
    card(r, Impl { bf_event: sigil(), ..Impl::new("Sigil of the Storm") });
    card(r, Impl { bf_event: star_spring(), ..Impl::new("Star Spring") });
    card(r, Impl { bf_event: targon(), ..Impl::new("Targon's Peak") });
    card(r, Impl { bf_event: threshold(), ..Impl::new("Threshold of the Gray") });

    // ================================================================== tokens
    plain(r, "Mech");
    plain(r, "Reflection");
    plain(r, "Gold"); // Add ability used while paying (Game.pay)

    // ================================================================== Akali deck
    card(r, akali_deadly_weapon());
    card(
        r,
        Impl {
            on_event: silent_event(),
            untargetable: f("untargetable", |g, a| {
                let o = a[0].obj();
                let by = a[1].opt_int().map(|x| x as usize);
                Ok((by != Some(g.o(o).ctrl) && !g.in_combat(o)).into())
            }),
            ..Impl::new("Akali, Silent")
        },
    );
    card(r, Impl { on_event: adaptatron(), ..Impl::new("Adaptatron") });
    card(
        r,
        Impl {
            timing: Some("reaction"),
            preds: Value::List(vec![p_friend_bf_v()]),
            resolve: ato(),
            choices: f("choices", |g, a| {
                let (pid, hb) = (pid_of(&a[0]), hbf(&a[1]));
                let us: Vec<O> = friends(g, pid, true, hb)?
                    .into_iter()
                    .filter(|&u| !units_at(g, Some(1 - pid), g.o(u).loc).is_empty())
                    .collect();
                Ok(tgs(g, &us))
            }),
            ..Impl::new("Against the Odds")
        },
    );
    card(r, Impl { on_event: heron_event(), ..Impl::new("Astral Heron") });
    card(
        r,
        Impl {
            timing: Some("action"),
            hidden: true,
            preds: Value::List(vec![p_unit_v()]),
            resolve: back_off(),
            choices: back_off_choices(),
            ..Impl::new("Back Off")
        },
    );
    card(r, Impl { kw: kw1("Tank"), on_play: blitz_play(), on_event: blitz_event(), ..Impl::new("Blitzcrank, Impassive") });
    card(
        r,
        Impl {
            timing: Some("action"),
            hidden: true,
            preds: Value::List(vec![p_unit_v()]),
            resolve: block(),
            choices: f("choices", |g, a| {
                let us = friends(g, pid_of(&a[0]), false, hbf(&a[1]))?;
                Ok(tgs(g, &us))
            }),
            ..Impl::new("Block")
        },
    );
    card(
        r,
        Impl {
            preds: Value::List(vec![p_gear_v()]),
            resolve: kill_gear(),
            flow: flow(4, 1, &["Fury"]),
            choices: f("choices", |g, a| {
                let pid = pid_of(&a[0]);
                let mut gs = g.gear(None);
                gs.sort_by_key(|&x| (g.o(x).ctrl == pid, -g.spec(x).e));
                Ok(tgs(g, &gs))
            }),
            ..Impl::new("Brittle Steel")
        },
    );
    card(
        r,
        Impl {
            preds: Value::List(vec![p_enemy_v()]),
            resolve: charm(),
            choices: charm_choices(),
            max_choices: 10,
            ..Impl::new("Charm")
        },
    );
    card(
        r,
        Impl {
            timing: Some("reaction"),
            resolve: crumbling(),
            choices: f("choices", |g, a| {
                let pid = pid_of(&a[0]);
                let out: Vec<Map> = g
                    .chain
                    .iter()
                    .filter(|&&i| g.it(i).kind == "spell" && g.it(i).ctrl != pid)
                    .map(|&i| map! { item = g.it(i).id })
                    .collect();
                Ok(maps(out))
            }),
            ..Impl::new("Crumbling Sands")
        },
    );
    card(r, Impl { on_event: darius(), ..Impl::new("Darius, Trifarian") });
    card(
        r,
        Impl {
            timing: Some("reaction"),
            preds: Value::List(vec![p_friend_v()]),
            resolve: mod_target(4),
            choices: f("choices", |g, a| {
                let pid = pid_of(&a[0]);
                let us: Vec<O> = friends(g, pid, false, None)?.into_iter().filter(|&u| dof_ok(g, pid, u)).collect();
                Ok(tgs(g, &us))
            }),
            ..Impl::new("Decree of Focus")
        },
    );
    // Decree of Insight — "[Reaction] Ignore [Deflect] while paying this spell's cost. Give an enemy Body unit -5
    // might this turn."
    card(
        r,
        Impl {
            timing: Some("reaction"),
            preds: Value::List(vec![f("pred", |g, a| {
                let (it, o) = (a[0].item(), a[1].obj());
                Ok((p_enemy(g, it, o) && has_domain(g, o, "Body")).into())
            })]),
            resolve: mod_target(-5),
            choices: f("choices", |g, a| {
                let us: Vec<O> =
                    enemies(g, pid_of(&a[0]), false, None, true)?.into_iter().filter(|&u| has_domain(g, u, "Body")).collect();
                Ok(tgs(g, &us))
            }),
            ..Impl::new("Decree of Insight")
        },
    );
    // Decree of Rage — "[Action] This can't be countered. Deal 4 to an enemy Calm unit."
    card(
        r,
        Impl {
            timing: Some("action"),
            uncounterable: true,
            preds: Value::List(vec![f("pred", |g, a| {
                let (it, o) = (a[0].item(), a[1].obj());
                Ok((p_enemy(g, it, o) && has_domain(g, o, "Calm")).into())
            })]),
            resolve: f("resolve", |g, a| {
                let it = a[0].item();
                let u = g.legal(it, 0)?;
                let c = ctrl(g, it);
                g.deal(u, 4, "spell", Some(c), None)?;
                none()
            }),
            choices: f("choices", |g, a| {
                let us: Vec<O> =
                    enemies(g, pid_of(&a[0]), false, None, true)?.into_iter().filter(|&u| has_domain(g, u, "Calm")).collect();
                Ok(tgs(g, &us))
            }),
            ..Impl::new("Decree of Rage")
        },
    );
    // Decree of Unity — "Kill an enemy Chaos unit or gear."
    card(
        r,
        Impl {
            preds: Value::List(vec![f("pred", |g, a| {
                let (it, o) = (a[0].item(), a[1].obj());
                Ok((g.o(o).ctrl != ctrl(g, it) && has_domain(g, o, "Chaos")).into())
            })]),
            resolve: kill_target(),
            choices: f("choices", |g, a| {
                let pid = pid_of(&a[0]);
                let os: Vec<O> =
                    g.board.iter().copied().filter(|&o| g.o(o).ctrl != pid && has_domain(g, o, "Chaos")).collect();
                Ok(tgs(g, &os))
            }),
            ..Impl::new("Decree of Unity")
        },
    );
    card(
        r,
        Impl {
            timing: Some("reaction"),
            resolve: defy(),
            choices: f("choices", |g, a| {
                let pid = pid_of(&a[0]);
                let mut out = vec![];
                for i in g.chain.clone().into_iter().rev() {
                    let ok = {
                        let x = g.it(i);
                        x.kind == "spell" && x.ctrl != pid && {
                            let c = x.card.unwrap();
                            g.spec(c).e <= 4 && g.spec(c).p <= 1
                        }
                    };
                    if ok && g.counterable(i)? {
                        out.push(map! { item = g.it(i).id });
                    }
                }
                Ok(maps(out))
            }),
            ..Impl::new("Defy")
        },
    );
    card(r, Impl { on_play: rake(), ..Impl::new("Disarming Rake") });
    card(
        r,
        Impl {
            timing: Some("reaction"),
            preds: Value::List(vec![p_unit_v()]),
            resolve: discipline(),
            choices: friends_choices(),
            ..Impl::new("Discipline")
        },
    );
    card(
        r,
        Impl {
            timing: Some("reaction"),
            preds: Value::List(vec![p_friend_v()]),
            resolve: en_garde(),
            choices: friends_choices(),
            ..Impl::new("En Garde")
        },
    );
    card(
        r,
        Impl {
            preds: Value::List(vec![p_unit_v(), p_unit_v()]),
            resolve: falling_star(),
            choices: fs_choices(),
            ..Impl::new("Falling Star")
        },
    );
    card(r, Impl { deathknell: forerunner_dk(), ..Impl::new("Ferrous Forerunner") });
    card(r, Impl { kw: kw1("Deflect"), on_event: irelia(), ..Impl::new("Irelia, Fervent") });
    card(r, Impl { accelerate: true, on_event: kaisa(), ..Impl::new("Kai'Sa, Survivor") });
    // Lonely Poro — "[Deathknell] If I died alone, draw 1."
    card(
        r,
        Impl {
            deathknell: f("deathknell", |g, a| {
                let it = a[0].item();
                if g.it(it).data.g("info").map().g("alone").truthy() {
                    let c = ctrl(g, it);
                    g.draw(c, 1)?;
                }
                none()
            }),
            ..Impl::new("Lonely Poro")
        },
    );
    // Long Sword / Sterak's Gage — "[Quick-Draw] [Equip] [Fury]/[Calm]" ; Pendulum Blade — "[Equip] [Fury]"
    card(r, Impl { timing: Some("reaction"), quickdraw: true, abilities: vec![equip_ability("Fury")], ..Impl::new("Long Sword") });
    card(
        r,
        Impl { timing: Some("reaction"), quickdraw: true, abilities: vec![equip_ability("Calm")], ..Impl::new("Sterak's Gage") },
    );
    card(r, Impl { abilities: vec![equip_ability("Fury")], effect_event: pendulum_effect(), ..Impl::new("Pendulum Blade") });
    card(r, Impl { hidden: true, on_play: marai(), ..Impl::new("Mischievous Marai") });
    card(
        r,
        Impl {
            on_event: witness(),
            might_mod: f("might_mod", |g, a| Ok(Value::Int(if g.o(a[0].obj()).empowered { 2 } else { 0 }))),
            ..Impl::new("Mournful Witness")
        },
    );
    card(
        r,
        Impl {
            timing: Some("reaction"),
            resolve: nsf(),
            choices: f("choices", |g, a| {
                let items = nsf_items(g, pid_of(&a[0]))?;
                Ok(maps(items.into_iter().map(|i| map! { item = g.it(i).id }).collect()))
            }),
            ..Impl::new("Not So Fast")
        },
    );
    // Noxus Hopeful — "[Legion] I cost 2 energy less."
    card(
        r,
        Impl {
            cost_mod: f("cost_mod", |g, a| {
                let pid = pid_of(&a[0]);
                Ok(if !g.finalized[pid].is_empty() { tup(vec![2.into(), 0.into()]) } else { tup(vec![0.into(), 0.into()]) })
            }),
            ..Impl::new("Noxus Hopeful")
        },
    );
    card(
        r,
        Impl {
            on_play: f("on_play", |g, a| {
                let o = a[0].obj();
                let (oc, uid) = (g.o(o).ctrl, g.o(o).uid);
                qt(g, oc, "Scuttle Crab", draw1(), Map::new(), Some(uid), false, Value::None, Value::None);
                none()
            }),
            deathknell: crab_dk(),
            ..Impl::new("Scuttle Crab")
        },
    );
    card(
        r,
        Impl {
            // tuple(sorted(("Fury", "Calm", "Mind", "Body", "Chaos", "Order")))
            flow: flow(3, 1, &["Body", "Calm", "Chaos", "Fury", "Mind", "Order"]),
            preds: Value::List(vec![p_enemy_bf_v()]),
            resolve: flip(),
            choices: flip_choices(),
            max_choices: 12,
            ..Impl::new("Shuriken Flip")
        },
    );
    // Sky Splitter — "[Action] This spell's Energy cost is reduced by the highest Might among units you control.
    // Deal 5 to a unit at a battlefield."
    card(
        r,
        Impl {
            timing: Some("action"),
            preds: Value::List(vec![p_unit_bf_v()]),
            cost_mod: f("cost_mod", |g, a| {
                let pid = pid_of(&a[0]);
                let mut m = 0;
                for u in g.units(Some(pid), None) {
                    m = m.max(g.might(u)?);
                }
                Ok(tup(vec![m.into(), 0.into()]))
            }),
            resolve: f("resolve", |g, a| {
                let it = a[0].item();
                let u = g.legal(it, 0)?;
                let c = ctrl(g, it);
                g.deal(u, 5, "spell", Some(c), None)?;
                none()
            }),
            choices: f("choices", |g, a| {
                let us = enemies(g, pid_of(&a[0]), true, hbf(&a[1]), true)?;
                Ok(tgs(g, &us))
            }),
            ..Impl::new("Sky Splitter")
        },
    );
    card(r, Impl { on_event: herder(), ..Impl::new("Stellacorn Herder") });
    card(r, Impl { on_play: barbara(), ..Impl::new("Tomb-Raider Barbara") });
    let mut zh = Impl { hidden: true, ..Impl::new("Zhonya's Hourglass") };
    zh.hooks.set("death_rep", zhonya_rep());
    card(r, zh);

    // ================================================================== LeBlanc deck
    card(r, baited_hook());
    card(
        r,
        Impl {
            timing: Some("action"),
            repeat: tup(vec![1.into(), tup(vec!["Mind".into()]), 1.into()]),
            preds: Value::List(vec![p_unit_v()]),
            resolve: bellows(),
            choices: bellows_choices(),
            ..Impl::new("Bellows Breath")
        },
    );
    // Black Rose Dignitary — "[Assault] [Deathknell] Channel 1 rune exhausted."
    card(r, Impl { kw: kw1("Assault"), deathknell: channel1_exhausted(), ..Impl::new("Black Rose Dignitary") });
    card(r, Impl { ambush: true, on_play: chakram(), ..Impl::new("Chakram Dancer") });
    card(r, Impl { resolve: cull(), ..Impl::new("Cull the Weak") });
    card(
        r,
        Impl {
            timing: Some("reaction"),
            preds: Value::List(vec![p_friend_v(), p_friend_v()]),
            resolve: deathgrip(),
            choices: deathgrip_choices(),
            ..Impl::new("Deathgrip")
        },
    );
    card(r, Impl { deathknell: mixo(), ..Impl::new("Glasc Mixologist") });
    card(r, Impl { on_play: dragon(), ..Impl::new("Harnessed Dragon") });
    card(
        r,
        Impl {
            timing: Some("action"),
            hidden: true,
            preds: Value::List(vec![p_unit_bf_v()]),
            resolve: blade(),
            choices: f("choices", |g, a| {
                let us = enemies(g, pid_of(&a[0]), true, hbf(&a[1]), true)?;
                Ok(tgs(g, &us))
            }),
            ..Impl::new("Hidden Blade")
        },
    );
    card(r, Impl { deathknell: broker(), ..Impl::new("Honest Broker") });
    // Karthus, Eternal — "Your [Deathknell] effects trigger an additional time." (in Game.kill)
    plain(r, "Karthus, Eternal");
    card(
        r,
        Impl {
            hidden: true,
            on_play: f("on_play", |g, a| {
                let hb = hbf(&a[1]);
                kennen_trigger(g, a[0].obj(), hb);
                none()
            }),
            on_event: f("on_event", |g, a| {
                let (o, ev, info) = (a[0].obj(), a[1].str(), &a[2]);
                if ev == "attack" && is_obj(info, "obj", o) {
                    kennen_trigger(g, o, None);
                }
                none()
            }),
            might_mod: f("might_mod", |g, a| {
                let o = a[0].obj();
                let st = units_at(g, Some(1 - g.o(o).ctrl), g.o(o).loc).into_iter().any(|u| g.o(u).stunned);
                Ok(Value::Int(if st { 2 } else { 0 }))
            }),
            ..Impl::new("Kennen, Keeper of Balance")
        },
    );
    card(
        r,
        Impl {
            timing: Some("reaction"),
            preds: Value::List(vec![p_unit_v()]),
            resolve: ki(),
            choices: friends_choices(),
            ..Impl::new("Ki Barrier")
        },
    );
    // LeBlanc, Everywhere At Once — "[Backline] Your [Temporary] effects at my battlefield don't trigger."
    card(r, Impl { kw: kw1("Backline"), ..Impl::new("LeBlanc, Everywhere At Once") });
    // LeBlanc, Fragmented — "[Assault] [Deathknell] Draw 1. If it's your Beginning Phase, draw 2 instead."
    card(
        r,
        Impl {
            kw: kw1("Assault"),
            deathknell: f("deathknell", |g, a| {
                let c = ctrl(g, a[0].item());
                let n = if g.tp == c && (g.stage == "beginning" || g.stage == "scoring") { 2 } else { 1 };
                g.draw(c, n)?;
                none()
            }),
            ..Impl::new("LeBlanc, Fragmented")
        },
    );
    // Mirror Image — "Choose a unit. Play a ready Reflection unit token to your base. It becomes a copy of that unit.
    // Give it [Temporary]."
    card(
        r,
        Impl {
            preds: Value::List(vec![p_unit_v()]),
            resolve: f("_mirror", |g, a| {
                let it = a[0].item();
                let u = g.legal(it, 0)?;
                let c = ctrl(g, it);
                reflection(g, c, Loc::Base, u)?;
                none()
            }),
            choices: f("choices", |g, a| {
                let pid = pid_of(&a[0]);
                let mut us = vec![];
                for u in g.all_units() {
                    if g.targetable(u, Some(pid))? {
                        us.push(u);
                    }
                }
                let us = by_value_desc(g, us)?;
                Ok(tgs(g, &us))
            }),
            ..Impl::new("Mirror Image")
        },
    );
    card(r, Impl { on_event: herald_move(), deathknell: herald_dk(), ..Impl::new("Rift Herald") });
    card(
        r,
        Impl {
            deathknell: rex_dk(),
            dk_choose: trig_target(enemies_opts(), p_enemy_v(), "target", false, true),
            ..Impl::new("Ruined Rex")
        },
    );
    // Sacrifice — "[Reaction] As an additional cost to play this, kill a friendly [Mighty] unit. Draw 2 and channel 1
    // rune exhausted."
    card(
        r,
        Impl {
            timing: Some("reaction"),
            choices: f("choices", |g, a| {
                let pid = pid_of(&a[0]);
                let us = g.units(Some(pid), None);
                let us = by_value(g, us)?;
                let mut out = vec![];
                for u in us {
                    if g.mighty(u)? {
                        out.push(map! { kill = g.o(u).uid });
                    }
                }
                Ok(maps(out))
            }),
            resolve: f("resolve", |g, a| {
                let c = ctrl(g, a[0].item());
                g.draw(c, 2)?;
                g.channel(c, 1, true)?;
                none()
            }),
            ..Impl::new("Sacrifice")
        },
    );
    card(
        r,
        Impl {
            on_play: inspector(),
            as_played: f("as_played", |g, a| {
                let pid = pid_of(&a[0]);
                let mut out = vec![Value::Map(Map::new())];
                if g.p[pid].xp >= 3 {
                    out.push(map! { xp = true }.into());
                }
                Ok(Value::List(out))
            }),
            ..Impl::new("Safety Inspector")
        },
    );
    card(
        r,
        Impl {
            timing: Some("action"),
            preds: Value::List(vec![p_gear_v()]),
            resolve: salvage(),
            choices: f("choices", |g, a| {
                let pid = pid_of(&a[0]);
                let xs: Vec<O> = g.gear(None).into_iter().filter(|&x| g.o(x).ctrl != pid).collect();
                let mut out = tg_choices(g, &xs);
                out.push(map! { tg = tup(vec![]) });
                Ok(maps(out))
            }),
            ..Impl::new("Salvage")
        },
    );
    // Soaring Scout — "[Deathknell] Channel 1 rune exhausted."
    card(r, Impl { deathknell: channel1_exhausted(), ..Impl::new("Soaring Scout") });
    // Stupefy — "[Reaction] Give a unit -1 might this turn, to a minimum of 1 might. Draw 1."
    card(
        r,
        Impl {
            timing: Some("reaction"),
            preds: Value::List(vec![p_unit_v()]),
            resolve: f("resolve", |g, a| {
                let it = a[0].item();
                if g.legal(it, 0)?.is_some() {
                    if let Some(u) = g.legal(it, 0)? {
                        g.mod_(u, -1, "turn".into(), Some(1))?;
                    }
                }
                let c = ctrl(g, it);
                g.draw(c, 1)?;
                none()
            }),
            choices: f("choices", |g, a| {
                let us = enemies(g, pid_of(&a[0]), false, None, true)?;
                Ok(tgs(g, &us))
            }),
            ..Impl::new("Stupefy")
        },
    );
    card(r, Impl { accelerate: true, on_play: watcher(), ..Impl::new("Thousand-Tailed Watcher") });
    // Time Warp — "Take a turn after this one. Banish this."
    card(
        r,
        Impl {
            banish_after: true,
            resolve: f("resolve", |g, a| {
                let c = ctrl(g, a[0].item());
                g.extra_turns.insert(0, c.into());
                none()
            }),
            ..Impl::new("Time Warp")
        },
    );
    // Turn to Dust — "Give a gear [Temporary]."
    card(
        r,
        Impl {
            preds: Value::List(vec![p_gear_v()]),
            resolve: f("resolve", |g, a| {
                let it = a[0].item();
                if g.legal(it, 0)?.is_some() {
                    if let Some(x) = g.legal(it, 0)? {
                        g.grant(x, "Temporary", 1, Value::None);
                    }
                }
                none()
            }),
            choices: f("choices", |g, a| {
                let pid = pid_of(&a[0]);
                let xs: Vec<O> = g.gear(None).into_iter().filter(|&x| g.o(x).ctrl != pid).collect();
                Ok(tgs(g, &xs))
            }),
            ..Impl::new("Turn to Dust")
        },
    );
    card(r, Impl { ambush: true, on_event: vi(), ..Impl::new("Vi, Peacekeeper") });
    // Watchful Sentry — "[Deathknell] Draw 1."
    card(r, Impl { deathknell: draw1(), ..Impl::new("Watchful Sentry") });
    card(r, Impl { on_play: ashe(), ..Impl::new("Ashe, Focused") });
    card(
        r,
        Impl {
            kw: kw1("Ganking"),
            cost_mod: atakhan_cost(),
            on_event: atakhan_event(),
            as_played: f("as_played", |g, a| {
                let pid = pid_of(&a[0]);
                let us = g.units(Some(pid), None);
                let us = by_value(g, us)?;
                let us = cap(g, us, 3, Some(pid));
                let mut out = vec![Value::Map(Map::new())];
                out.extend(us.into_iter().map(|u| Value::Map(map! { kill = g.o(u).uid })));
                Ok(Value::List(out))
            }),
            ..Impl::new("Atakhan")
        },
    );
}
