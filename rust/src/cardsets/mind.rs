//! Cartes du paquet mind (`riftbound/engine/cardsets/mind.py`, liste : batches/mind.txt), traduites ligne à ligne.
//! Voir `riftbound/engine/cardsets/GUIDE.md` et `docs/memoire/portage-rust.md`.
//!
//! Les fermetures ne capturent jamais d'objet de jeu modifiable : les objets passent par `item.data`, les entrées de
//! `g.effects` ou des uids (comme en Python, où les copies de l'IA copient la partie mais pas les fermetures).
//!
//! Carte du module Python non enregistrée ici (elle reste bloquée) : The Zero Drive (voir son commentaire plus bas).
use std::cmp::Ordering;

use crate::actions::{full_choices, pay_ctx, play_card, play_forbidden, spell_variants, total_cost};
use crate::cards::{Impl, Registry};
use crate::cards_helpers::{
    ability, add_ability, all_units, cap, empower_ability, enemies, flow_cost, friends, gain_control_item, hb_ok,
    impl_empower, impl_equip, item_by_id, keywords_of, make_token, may_pay, p_enemy, p_enemy_bf_v, p_enemy_v,
    p_friend, p_friend_v, p_unit_bf_v, p_unit_v, play_unit_free, remake_choices, repeat_cost, repeatable,
    tg_choices, token_locations, trig_target, value, when_empowered, AbilityArgs, Cost,
};
use crate::game::{Game, ItemId, Res, Zone, O, R};
use crate::game_costs::{any_req, reqs_to_value, Req};
use crate::map;
use crate::spec::specs;
use crate::value::{is_bf, Loc, Map, Value};

/// `MIND = frozenset({"Mind"})`
fn mind() -> Req {
    std::iter::once("Mind".to_string()).collect()
}

// ====================================================================== aides de traduction (privées)
fn f(name: &'static str, fun: impl Fn(&mut Game, &[Value]) -> Res<Value> + Send + Sync + 'static) -> Value {
    Value::func(name, fun)
}

fn turn() -> Value {
    Value::from("turn")
}

fn ty(g: &Game, o: O) -> &'static str {
    g.spec(o).type_.as_str()
}

fn se(g: &Game, o: O) -> i64 {
    g.spec(o).e as i64
}

fn sp(g: &Game, o: O) -> i64 {
    g.spec(o).p as i64
}

fn cn(g: &Game, o: O) -> String {
    g.cname(o).to_string()
}

fn itc(g: &Game, it: ItemId) -> usize {
    g.it(it).ctrl
}

/// `info["obj"] is o`
fn is_obj(v: &Value, o: O) -> bool {
    v.opt_obj() == Some(o)
}

/// `o in info["units"]`
fn in_list(v: &Value, o: O) -> bool {
    v.list().iter().any(|x| x.opt_obj() == Some(o))
}

fn objs(v: &[O]) -> Value {
    Value::List(v.iter().map(|&o| Value::Obj(o)).collect())
}

fn obj_vec(v: &Value) -> Vec<O> {
    v.list().iter().map(|x| x.obj()).collect()
}

fn maps(v: Vec<Map>) -> Value {
    Value::List(v.into_iter().map(Value::Map).collect())
}

fn map_vec(v: &Value) -> Vec<Map> {
    v.list().iter().map(|x| x.map().clone()).collect()
}

/// `(e, reqs)`
fn cost_v(e: i64, reqs: &[Req]) -> Value {
    Value::Tuple(vec![e.into(), reqs_to_value(reqs)])
}

/// `(uid, ...)` : tuple d'uids d'un choix (`tg=`)
fn tg_of(g: &Game, v: &[O]) -> Value {
    Value::Tuple(v.iter().map(|&o| Value::from(g.o(o).uid)).collect())
}

/// `sorted(v, key=...)` : tri stable sur des clés calculées une fois par élément (comme Python).
fn sort_keyed<T, K: PartialOrd>(v: Vec<T>, keys: Vec<K>) -> Vec<T> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|&a, &b| keys[a].partial_cmp(&keys[b]).unwrap_or(Ordering::Equal));
    let mut slots: Vec<Option<T>> = v.into_iter().map(Some).collect();
    idx.into_iter().map(|i| slots[i].take().unwrap()).collect()
}

/// `itertools.combinations(range(n), k)` (ordre lexicographique des indices)
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn rec(start: usize, n: usize, k: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for i in start..n {
            cur.push(i);
            rec(i + 1, n, k, cur, out);
            cur.pop();
        }
    }
    let mut out = vec![];
    if k <= n {
        rec(0, n, k, &mut vec![], &mut out);
    }
    out
}

/// Clé de `g.stats` pour un tuple Python : son `repr`.
fn stats_has(g: &Game, key: &str) -> bool {
    g.stats.0.iter().any(|(k, _)| k == key)
}

// ====================================================================== private helpers
/// `_src` : the source permanent of a triggered/activated ability, if still on the board.
fn src_of(g: &Game, it: ItemId) -> Option<O> {
    g.it(it).src.and_then(|s| g.obj(s))
}

/// `_here` : 'here' of a source permanent, checked on execution (rule 359.3.f.2): its battlefield/base, or None.
fn here(g: &Game, it: ItemId) -> Option<Loc> {
    src_of(g, it).and_then(|me| g.o(me).loc)
}

/// `_token_locs` : valid locations to play a unit token whose effect names none (rule 355.2: base or a battlefield
/// you control); a hidden spell/play effect must play it at its battlefield (rule 811.1.d.3).
fn token_locs(g: &Game, pid: usize, hb: Option<Loc>) -> Vec<Loc> {
    if let Some(h) = hb {
        return vec![h];
    }
    let mut v = vec![Loc::Base];
    v.extend(g.bfs.iter().filter(|b| b.ctrl == Some(pid)).map(|b| Loc::Bf(b.idx)));
    v
}

/// `_play_token` : play a token (rules 185.2.a, 350.2): it enters the board, its keyword play triggers ([Vision])
/// trigger and 'played' is emitted. Tokens are not cards: n=0 and nothing is added to g.played.
fn play_token(g: &mut Game, name: &str, pid: usize, loc: Loc, ready: bool) -> Res<Option<O>> {
    make_token(g, name, pid, loc, ready) // the engine's single token-play path
}

/// `_play_unit_token(g, name, pid, ready, hb, loc)`
fn play_unit_token(g: &mut Game, name: &str, pid: usize, ready: bool, hb: Option<Loc>, loc: Option<Loc>) -> Res<Option<O>> {
    let loc = match loc {
        Some(l) => l,
        None => {
            let tl = token_locs(g, pid, hb);
            let ls = token_locations(g, pid, name, Some(tl))?;
            if ls.is_empty() {
                return Ok(None);
            }
            g.ask(pid, "play_location", ls.into_iter().map(Value::Loc).collect(), Map::new())?.loc()
        }
    };
    play_token(g, name, pid, loc, ready)
}

/// `_gold` : 'Play a Gold gear token exhausted.' (gear are played to base)
fn gold(g: &mut Game, pid: usize) -> Res<Option<O>> {
    play_token(g, "Gold", pid, Loc::Base, false)
}

/// `_is_mech`
fn is_mech(g: &mut Game, o: O) -> Res<bool> {
    Ok(g.tags(o)?.contains("Mech"))
}

/// `_minus(g, u, n, minimum)`
fn minus(g: &mut Game, u: Option<O>, n: i64, minimum: Option<i64>) -> Res<()> {
    if let Some(u) = u {
        if g.on_board(u) {
            g.mod_(u, -n, turn(), minimum)?;
        }
    }
    Ok(())
}

/// `_rune(g, uid)`
fn rune_of(g: &Game, uid: i64) -> Option<R> {
    for pl in &g.p {
        for &r in &pl.runes {
            if g.r(r).uid as i64 == uid {
                return Some(r);
            }
        }
    }
    None
}

/// `_card_in(zone, uid, oid)`
fn card_in(g: &Game, zone: &[O], uid: i64, oid: i64) -> Option<O> {
    zone.iter().copied().find(|&c| g.o(c).uid as i64 == uid && g.o(c).oid == oid)
}

/// `_trig(g, o, name, fn, data, **kw)`
#[allow(clippy::too_many_arguments)]
fn trig(g: &mut Game, o: O, name: &str, fun: Value, data: Map, may: bool, choose: Value, cost: Value) {
    let (ctrl, uid) = (g.o(o).ctrl, g.o(o).uid);
    g.queue_trigger(ctrl, name, fun, data, Some(uid), may, choose, cost);
}

/// `_trig` sans données ni options
fn trig0(g: &mut Game, o: O, name: &str, fun: Value) {
    trig(g, o, name, fun, Map::new(), false, Value::None, Value::None);
}

/// `_is_here(u, loc, pid)` : u is at loc: a battlefield, or pid's own base (bases are per player).
fn is_here(g: &Game, u: O, loc: Option<Loc>, pid: usize) -> bool {
    g.o(u).loc == loc && (is_bf(loc) || g.o(u).ctrl == pid)
}

/// `_target_here(any_unit)` : trig_target for 'an enemy unit here' ('here' = the source's location, checked on
/// execution).
fn target_here(any_unit: bool) -> Value {
    let opts = f("_target_here.opts", move |g, a| {
        let it = a[0].item();
        let Some(me) = src_of(g, it) else { return Ok(objs(&[])) };
        let ctrl = itc(g, it);
        let pool = if any_unit { all_units(g, ctrl, false, None)? } else { enemies(g, ctrl, false, None, true)? };
        let (ml, mc) = (g.o(me).loc, g.o(me).ctrl);
        let v: Vec<O> = pool.into_iter().filter(|&u| is_here(g, u, ml, mc)).collect();
        Ok(objs(&v))
    });
    let pred = f("_target_here.pred", move |g, a| {
        let (it, u) = (a[0].item(), a[1].obj());
        let Some(me) = src_of(g, it) else { return Ok(false.into()) };
        let (ml, mc) = (g.o(me).loc, g.o(me).ctrl);
        Ok((is_here(g, u, ml, mc) && (any_unit || g.o(u).ctrl != itc(g, it))).into())
    });
    trig_target(opts, pred, "target", false, true)
}

/// `trig_target(options_fn, pred, deflect=True)` avec le type de question par défaut
fn tt(opts: Value, pred: Value) -> Value {
    trig_target(opts, pred, "target", false, true)
}

/// `card(name, ...)` sans autre champ que ceux posés par l'appelant
fn imp(name: &str) -> Impl {
    let mut im = Impl::new(name);
    im.module = Some("mind");
    im
}

/// `dict(k=v, ...)` dans l'ordre donné (clés qui ne sont pas des identifiants Rust : `fn`)
fn mk(pairs: Vec<(&str, Value)>) -> Map {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.set(k, v);
    }
    m
}

fn kw1(k: &str, v: i64) -> Map {
    let mut m = Map::new();
    m.set(k, v);
    m
}

// ====================================================================== units
// Ahri, Inquisitive — "When I attack or defend, give an enemy unit here -2 might this turn, to a minimum of 1
// might."
fn ahri(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if matches!(ev, "attack" | "defend") && is_obj(&info.g("obj"), o) {
        let res = f("_ahri.res", |g, a| {
            let u = g.legal(a[0].item(), 0)?;
            minus(g, u, 2, Some(1))?;
            Ok(Value::None)
        });
        trig(g, o, "Ahri, Inquisitive", res, Map::new(), false, target_here(false), Value::None);
    }
    Ok(Value::None)
}

// Apprentice Mage — "[Empower] 2 energy. When I become [Empowered], [Predict 2]. [Empowered] I have +1 might."
fn mage(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "empowered" && is_obj(&info.g("obj"), o) {
        trig0(
            g,
            o,
            "Apprentice Mage",
            f("_mage.res", |g, a| {
                let c = itc(g, a[0].item());
                g.predict(c, 2)?;
                Ok(Value::None)
            }),
        );
    }
    Ok(Value::None)
}

// Aspiring Engineer — "When you play me, return a gear from your trash to your hand."
// The trash is public: the gear card is a target (rule 355.10.a), chosen as the trigger is finalized.
fn engineer(g: &mut Game, a: &[Value]) -> Res<Value> {
    let o = a[0].obj();
    let choose = f("_engineer.choose", |g, a| {
        let it = a[0].item();
        let ctrl = itc(g, it);
        let gs: Vec<O> = g.p[ctrl].trash.iter().copied().filter(|&c| ty(g, c) == "Gear").collect();
        let keys: Vec<(i64, String)> = gs.iter().map(|&c| (-se(g, c) - 2 * sp(g, c), cn(g, c))).collect();
        let gs = sort_keyed(gs, keys);
        if gs.is_empty() {
            return Ok(false.into());
        }
        let c = g.ask(ctrl, "trash_gear", gs.iter().map(|&x| Value::Obj(x)).collect(), map! { item = it })?.obj();
        let v = Value::Tuple(vec![g.o(c).uid.into(), g.o(c).oid.into()]);
        g.it_mut(it).data.set("card", v);
        Ok(true.into())
    });
    let res = f("_engineer.res", |g, a| {
        let it = a[0].item();
        let ctrl = itc(g, it);
        let k = g.it(it).data.g("card");
        let (uid, oid) = (k.list()[0].int(), k.list()[1].int());
        let tr = g.p[ctrl].trash.clone();
        if let Some(c) = card_in(g, &tr, uid, oid) {
            g.to_zone(c, "hand", false, None)?;
        }
        Ok(Value::None)
    });
    trig(g, o, "Aspiring Engineer", res, Map::new(), false, choose, Value::None);
    Ok(Value::None)
}

// Ava Achiever — "When I attack, you may pay 1 mind rune to play a card with [Hidden] from your hand
// here, ignoring its cost."
// Ruling chosen: a spell played "here" makes its choices like a spell played from Hidden at this battlefield
// (rule 811.1.d.2); a unit is played to this battlefield; a gear is played to base (gear location, rule 355.2).
/// `_hidden_card`
fn hidden_card(g: &Game, c: O) -> bool {
    g.impl_o(c).is_some_and(|im| im.hidden)
}

/// `_ava_plays` : (card, choice) pairs for Ava: base costs ignored (rule 356.1.b.1), additional costs (Deflect...)
/// still paid.
fn ava_plays(g: &mut Game, pid: usize, here: Loc) -> Res<Vec<(O, Map)>> {
    let mut out = vec![];
    let hs: Vec<O> = g.p[pid].hand.iter().copied().filter(|&c| hidden_card(g, c)).collect();
    let keys: Vec<(i64, String)> = hs.iter().map(|&c| (-se(g, c), cn(g, c))).collect();
    for c in sort_keyed(hs, keys) {
        let t = ty(g, c);
        let im = g.impl_o(c).unwrap();
        let chs: Vec<Map> = if t == "Unit" {
            vec![map! { loc = here }]
        } else if t == "Gear" {
            vec![map! { loc = Loc::Base }]
        } else if t == "Spell" {
            let base = if !im.choices.is_none() {
                map_vec(&im.choices.call(g, &[pid.into(), map! { hidden_bf = here, card = c }.into()])?)
            } else {
                vec![Map::new()]
            };
            cap(g, base, 3, Some(pid))
        } else {
            vec![]
        };
        for ch in chs {
            let mut ch = ch;
            ch.set("free", true);
            let (e, reqs) = total_cost(g, pid, c, &ch, "hand")?;
            if g.can_pay(pid, e, &reqs, &Value::None)? {
                out.push((c, ch));
            }
        }
    }
    Ok(out)
}

fn ava(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "attack" && is_obj(&info.g("obj"), o) {
        let cost = f("_ava.cost", |g, a| {
            let it = a[0].item();
            let ctrl = itc(g, it);
            if !g.p[ctrl].hand.iter().any(|&c| hidden_card(g, c)) {
                return Ok(false.into()); // nothing to play: paying would do nothing
            }
            Ok(g.pay(ctrl, 0, &[mind()], &Value::None)?.into())
        });
        let res = f("_ava.res", |g, a| {
            let it = a[0].item();
            let ctrl = itc(g, it);
            let hr = here(g, it);
            let Some(h) = hr.filter(|l| l.is_bf()) else { return Ok(Value::None) };
            let plays = ava_plays(g, ctrl, h)?;
            if plays.is_empty() {
                return Ok(Value::None);
            }
            let opts: Vec<Value> = (0..plays.len()).map(Value::from).collect();
            let pv = Value::List(plays.iter().map(|(c, ch)| Value::Tuple(vec![Value::Obj(*c), Value::Map(ch.clone())])).collect());
            let k = g.ask(ctrl, "ava_play", opts, map! { plays = pv })?.int() as usize;
            let (c, ch) = plays[k].clone();
            let (e, reqs) = total_cost(g, ctrl, c, &ch, "hand")?;
            if (e != 0 || !reqs.is_empty()) && !g.pay(ctrl, e, &reqs, &Value::None)? {
                return Ok(Value::None);
            }
            let r = play_card(g, ctrl, c, "hand", ch, true)?;
            if ty(g, c) == "Spell" && !r.is_none() {
                g.it_mut(r.item()).data.set("hidden_bf", h);
            }
            Ok(Value::None)
        });
        trig(g, o, "Ava Achiever", res, Map::new(), true, Value::None, cost);
    }
    Ok(Value::None)
}

// Bard, Mercurial — "You may exhaust your legend as an additional cost to play me. When you play me, if you paid
// the additional cost, move any number of your units to an open battlefield."
// Open battlefield: unoccupied and uncontrolled (rule 170.11.c).
fn bard_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let pid = itc(g, it);
    let opn: Vec<usize> =
        g.bfs.iter().filter(|b| b.ctrl.is_none() && g.units(None, Some(Loc::Bf(b.idx))).is_empty()).map(|b| b.idx).collect();
    if opn.is_empty() {
        return Ok(Value::None);
    }
    let us = friends(g, pid, false, None)?;
    let mut opts: Vec<(usize, Vec<u32>)> = vec![];
    for &bf in &opn {
        let base: Vec<u32> = us.iter().filter(|&&u| g.o(u).loc == Some(Loc::Base)).map(|&u| g.o(u).uid).collect();
        let everyone: Vec<u32> = us.iter().map(|&u| g.o(u).uid).collect();
        for grp in [base, everyone] {
            if !grp.is_empty() && !opts.contains(&(bf, grp.clone())) {
                opts.push((bf, grp));
            }
        }
        for &u in us.iter().take(3) {
            let one = (bf, vec![g.o(u).uid]);
            if !opts.contains(&one) {
                opts.push(one);
            }
        }
        if full_choices(g, Some(pid)) {
            // a human: any set of units
            for k in 1..=us.len() {
                for grp in combinations(us.len(), k) {
                    let x = (bf, grp.iter().map(|&i| g.o(us[i]).uid).collect::<Vec<u32>>());
                    if !opts.contains(&x) {
                        opts.push(x);
                    }
                }
            }
        }
        opts.push((bf, vec![]));
    }
    let ov: Vec<Value> = opts
        .iter()
        .map(|(bf, grp)| Value::Tuple(vec![(*bf).into(), Value::Tuple(grp.iter().map(|&u| Value::from(u)).collect())]))
        .collect();
    let ch = g.ask(pid, "bard_move", ov, Map::new())?;
    let bf = ch.list()[0].int() as usize;
    let grp: Vec<u32> = ch.list()[1].list().iter().map(|x| x.int() as u32).collect();
    let movers: Vec<O> = grp.iter().filter_map(|&x| g.obj(x)).filter(|&o| g.o(o).ctrl == pid).collect();
    if !movers.is_empty() {
        g.move_(&movers, Loc::Bf(bf), Some(pid), false)?;
    }
    Ok(Value::None)
}

// Blastcone Fae — "[Hidden] When you play me, give a unit -2 might this turn, to a minimum of 1 might."
fn blastcone(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ctx) = (a[0].obj(), a[1].map());
    let hb = ctx.g("hidden_bf").opt_loc();
    let res = f("_blastcone.res", |g, a| {
        let u = g.legal(a[0].item(), 0)?;
        minus(g, u, 2, Some(1))?;
        Ok(Value::None)
    });
    let opts = f("_blastcone.opts", move |g, a| {
        let c = itc(g, a[0].item());
        Ok(objs(&all_units(g, c, false, hb)?))
    });
    trig(g, o, "Blastcone Fae", res, Map::new(), false, tt(opts, p_unit_v()), Value::None);
    Ok(Value::None)
}

// Bubble Bot — "When you play me, ready another friendly Mech."
fn bubble(g: &mut Game, a: &[Value]) -> Res<Value> {
    let o = a[0].obj();
    let me = g.o(o).uid;
    let res = f("_bubble.res", |g, a| {
        let it = a[0].item();
        if g.legal(it, 0)?.is_some() {
            if let Some(u) = g.legal(it, 0)? {
                g.ready_obj(u, None, None)?;
            }
        }
        Ok(Value::None)
    });
    let opts = f("_bubble.opts", move |g, a| {
        let c = itc(g, a[0].item());
        let mut us = vec![];
        for u in g.units(Some(c), None) {
            if is_mech(g, u)? && g.o(u).uid != me {
                us.push(u);
            }
        }
        let mut keys = vec![];
        for &u in &us {
            keys.push((!g.exhausted(u), -value(g, u)?));
        }
        Ok(objs(&sort_keyed(us, keys)))
    });
    let pred = f("_bubble.pred", move |g, a| {
        let (it, u) = (a[0].item(), a[1].obj());
        Ok((g.o(u).ctrl == itc(g, it) && is_mech(g, u)? && g.o(u).uid != me).into())
    });
    trig(g, o, "Bubble Bot", res, Map::new(), false, tt(opts, pred), Value::None);
    Ok(Value::None)
}

// Card Sharp — "When you play me, you and each opponent may play a Gold gear token exhausted. For each opponent
// who did, you play a Gold gear token exhausted."
fn sharp(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let pid = itc(g, it);
    if g.ask(pid, "may", vec![true.into(), false.into()], map! { item = it })?.truthy() {
        gold(g, pid)?;
    }
    let opp = 1 - pid;
    if g.ask(opp, "may", vec![true.into(), false.into()], map! { item = it })?.truthy() {
        gold(g, opp)?;
        gold(g, pid)?;
    }
    Ok(Value::None)
}

/// `lambda g_, it: g_.draw(it.ctrl, n)`
fn draw_fn(n: i64) -> Value {
    f("draw", move |g, a| {
        let c = itc(g, a[0].item());
        g.draw(c, n)?;
        Ok(Value::None)
    })
}

/// `on_play=lambda g, o, ctx: _trig(g, o, name, fn)`
fn on_play_trig(name: &'static str, fun: Value) -> Value {
    f("on_play", move |g, a| {
        trig0(g, a[0].obj(), name, fun.clone());
        Ok(Value::None)
    })
}

// Covert Informant — "[Empower] 3 energy. [Empowered] When I move, draw 1."
fn informant(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "move" && is_obj(&info.g("obj"), o) && g.o(o).empowered {
        trig0(g, o, "Covert Informant", draw_fn(1));
    }
    Ok(Value::None)
}

// Diana, Lunari — "When a showdown begins here, you may pay 1 energy. If you do, [Predict], then reveal the top
// card of your Main Deck. If it's a spell, draw it."  (cost within instructions paid at finalization, 383.3.b)
fn diana_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = itc(g, a[0].item());
    g.predict(pid, 1)?;
    let rv = g.reveal(pid, 1, None, true, None)?;
    if !rv.is_empty() {
        let top: Vec<O> = g.p[pid].deck.iter().take(1).copied().collect();
        if ty(g, rv[0]) == "Spell" && top == rv {
            g.draw(pid, 1)?;
        }
    }
    Ok(Value::None)
}

fn diana(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "showdown_start" && g.o(o).loc == Some(Loc::Bf(info.g("bf").int() as usize)) {
        trig(g, o, "Diana, Lunari", f("_diana_res", diana_res), Map::new(), true, Value::None, may_pay(1, Value::None));
    }
    Ok(Value::None)
}

// Dr. Mundo, Expert — "My Might is increased by the number of cards in your trash. At the start of your Beginning
// Phase, recycle 3 from your trash."  (recycle X from a zone: not targeted, as many as possible, rule 416.6)
/// `_recycle_from_trash(g, pid, n, kind)`
fn recycle_from_trash(g: &mut Game, pid: usize, n: usize, kind: &str) -> Res<Vec<O>> {
    let mut picked: Vec<O> = vec![];
    for _ in 0..n {
        let tr: Vec<O> = g.p[pid].trash.iter().copied().filter(|c| !picked.contains(c)).collect();
        if tr.is_empty() {
            break;
        }
        let keys: Vec<(i64, String)> = tr.iter().map(|&c| (se(g, c) + 2 * sp(g, c), cn(g, c))).collect();
        let tr = sort_keyed(tr, keys);
        picked.push(g.ask(pid, kind, tr.iter().map(|&c| Value::Obj(c)).collect(), Map::new())?.obj());
    }
    if !picked.is_empty() {
        g.recycle_cards(pid, &picked)?;
    }
    Ok(picked)
}

fn mundo(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "beginning_start" && info.g("pid").int() as usize == g.o(o).ctrl {
        trig0(
            g,
            o,
            "Dr. Mundo, Expert",
            f("_mundo.res", |g, a| {
                let c = itc(g, a[0].item());
                recycle_from_trash(g, c, 3, "recycle_pick")?;
                Ok(Value::None)
            }),
        );
    }
    Ok(Value::None)
}

// Dropboarder — "When you play me, if you control two or more gear, ready me."
fn dropboarder(g: &mut Game, a: &[Value]) -> Res<Value> {
    let o = a[0].obj();
    if g.gear(Some(g.o(o).ctrl)).len() >= 2 {
        let res = f("_dropboarder.res", |g, a| {
            let it = a[0].item();
            let me = src_of(g, it);
            if let Some(me) = me {
                if g.gear(Some(itc(g, it))).len() >= 2 {
                    g.ready_obj(me, None, None)?;
                }
            }
            Ok(Value::None)
        });
        trig0(g, o, "Dropboarder", res);
    }
    Ok(Value::None)
}

// Ekko, Recurrent — "[Accelerate] [Deathknell] — Recycle me to ready your runes."
// "Recycle me" is a cost within instructions, paid as the trigger is finalized (rule 383.3.b, Ekko example).
fn ekko_cost(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c = g.it(it).data.g("info").map().g("obj").obj();
    let owner = g.o(c).owner;
    if g.o(c).token || !g.p[owner].trash.contains(&c) {
        return Ok(false.into());
    }
    g.recycle_cards(owner, &[c])?;
    Ok(true.into())
}

fn ekko_dk(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = itc(g, a[0].item());
    for r in g.p[pid].runes.clone() {
        g.r_mut(r).exhausted = false;
    }
    Ok(Value::None)
}

// Fate Weaver — "When you play me, look at the top 4 cards of your Main Deck. You may reveal a spell with Energy
// cost 4 energy or more from among them and draw it. Recycle the rest."
fn weaver_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let ctrl = itc(g, it);
    let d4: Vec<O> = g.p[ctrl].deck.iter().take(4).copied().collect();
    let top = g.look(ctrl, &d4, None)?;
    let spv: Vec<O> = top.iter().copied().filter(|&c| ty(g, c) == "Spell" && se(g, c) >= 4).collect();
    let keys: Vec<i64> = spv.iter().map(|&c| -se(g, c)).collect();
    let spv = sort_keyed(spv, keys);
    let pick = if !spv.is_empty() {
        let mut opts: Vec<Value> = spv.iter().map(|&c| Value::Obj(c)).collect();
        opts.push(Value::None);
        g.ask(ctrl, "fate_pick", opts, Map::new())?.opt_obj()
    } else {
        None
    };
    let rest: Vec<O> = top.iter().copied().filter(|&c| Some(c) != pick).collect();
    if let Some(p) = pick {
        let r = g.repr(p);
        g.log(|| format!("  P{ctrl} reveals and draws {r}"));
        g.draw_card(ctrl, p)?;
    }
    for &c in &rest {
        if let Some(i) = g.p[ctrl].deck.iter().position(|&x| x == c) {
            g.p[ctrl].deck.remove(i);
        }
    }
    g.recycle_cards(ctrl, &rest)?;
    Ok(Value::None)
}

// Frostcoat Cub — "You may pay 1 mind rune as an additional cost to play me. When you play me, if you paid the
// additional cost, give a unit -2 might this turn."
fn cub(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ctx) = (a[0].obj(), a[1].map());
    if !ctx.g("cub").truthy() {
        return Ok(Value::None);
    }
    let hb = ctx.g("hidden_bf").opt_loc();
    let res = f("_cub.res", |g, a| {
        let u = g.legal(a[0].item(), 0)?;
        minus(g, u, 2, None)?;
        Ok(Value::None)
    });
    let opts = f("_cub.opts", move |g, a| {
        let c = itc(g, a[0].item());
        Ok(objs(&all_units(g, c, false, hb)?))
    });
    trig(g, o, "Frostcoat Cub", res, Map::new(), false, tt(opts, p_unit_v()), Value::None);
    Ok(Value::None)
}

// Hwei, Brooding Painter — "When I move, draw 1, then discard 1. Then, do the following based on the discarded
// card's type: Spell — Draw 1. Gear — Ready up to 2 runes. Unit — Give me +3 might this turn."
fn hwei_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let pid = itc(g, it);
    g.draw(pid, 1)?;
    let hand_ = g.p[pid].hand.clone();
    if hand_.is_empty() {
        return Ok(Value::None);
    }
    let c = g.ask(pid, "discard", hand_.iter().map(|&x| Value::Obj(x)).collect(), map! { reason = "hwei" })?.obj();
    if !g.discard(pid, c)? {
        return Ok(Value::None);
    }
    let t = ty(g, c);
    if t == "Spell" {
        g.draw(pid, 1)?;
    } else if t == "Gear" {
        let ex: Vec<R> = g.p[pid].runes.iter().copied().filter(|&r| g.r(r).exhausted).collect();
        let doms: std::collections::BTreeSet<&str> = ex.iter().map(|&r| g.r(r).domain.as_str()).collect();
        if full_choices(g, Some(pid)) && ex.len() > 2 && doms.len() > 1 {
            for _ in 0..2 {
                // a human chooses which runes (by domain)
                let opts: Vec<Value> =
                    g.p[pid].runes.iter().copied().filter(|&r| g.r(r).exhausted).map(Value::Rune).collect();
                let r = g.ask(pid, "ready_rune", opts, Map::new())?.rune();
                g.r_mut(r).exhausted = false;
            }
        } else {
            for &r in ex.iter().take(2) {
                g.r_mut(r).exhausted = false;
            }
        }
    } else if t == "Unit" {
        if let Some(me) = src_of(g, it) {
            g.mod_(me, 3, turn(), None)?;
        }
    }
    Ok(Value::None)
}

fn hwei(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "move" && is_obj(&info.g("obj"), o) {
        trig0(g, o, "Hwei, Brooding Painter", f("_hwei_res", hwei_res));
    }
    Ok(Value::None)
}

// Icevale Archer — "When I attack, you may pay 1 energy to give a unit here -1 might this turn."
fn archer(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "attack" && is_obj(&info.g("obj"), o) {
        let res = f("_archer.res", |g, a| {
            let u = g.legal(a[0].item(), 0)?;
            minus(g, u, 1, None)?;
            Ok(Value::None)
        });
        trig(g, o, "Icevale Archer", res, Map::new(), true, target_here(true), may_pay(1, Value::None));
    }
    Ok(Value::None)
}

// Jayce, Brilliant Inventor — "When you play me or the first time you play a non-token gear each turn, you may
// ready something besides me that's exhausted."
fn jayce_choose(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let pid = itc(g, it);
    let src = g.it(it).src;
    let mut opts: Vec<Value> = vec![];
    let us: Vec<O> = g.units(Some(pid), None).into_iter().filter(|&u| g.exhausted(u) && Some(g.o(u).uid) != src).collect();
    let mut keys = vec![];
    for &u in &us {
        keys.push(-value(g, u)?);
    }
    let us = sort_keyed(us, keys);
    let pair = |k: &str, x: Value| Value::Tuple(vec![k.into(), x]);
    opts.extend(us.iter().map(|&u| pair("obj", g.o(u).uid.into())));
    opts.extend(
        g.gear(Some(pid))
            .into_iter()
            .filter(|&x| g.exhausted(x) && Some(g.o(x).uid) != src)
            .map(|x| pair("obj", g.o(x).uid.into())),
    );
    if g.exhausted(g.p[pid].legend) {
        opts.push(pair("legend", pid.into()));
    }
    opts.extend(g.p[pid].runes.iter().filter(|&&r| g.r(r).exhausted).map(|&r| pair("rune", g.r(r).uid.into())));
    if opts.is_empty() {
        return Ok(false.into());
    }
    let pick = g.ask(pid, "ready_pick", opts, map! { item = it })?;
    g.it_mut(it).data.set("pick", pick.clone());
    if pick.list()[0].str() == "obj" {
        let o = g.obj(pick.list()[1].int() as u32).unwrap();
        let pred = f("_jayce_choose.pred", |g, a| {
            let (it_, o) = (a[0].item(), a[1].obj());
            Ok((g.exhausted(o) && Some(g.o(o).uid) != g.it(it_).src).into())
        });
        g.add_target(it, o, pred);
    }
    Ok(true.into())
}

fn jayce_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let pick = g.it(it).data.g("pick");
    let (kind, x) = (pick.list()[0].str().to_string(), pick.list()[1].clone());
    if kind == "obj" {
        if let Some(o) = g.legal(it, 0)? {
            g.ready_obj(o, None, None)?;
        }
    } else if kind == "legend" {
        let leg = g.p[x.int() as usize].legend;
        g.set_exhausted(leg, false);
    } else if let Some(r) = rune_of(g, x.int()) {
        if g.r(r).exhausted {
            g.r_mut(r).exhausted = false;
        }
    }
    Ok(Value::None)
}

fn jayce_event(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "played" && info.g("pid").int() as usize == g.o(o).ctrl {
        let card = info.g("card").obj();
        if ty(g, card) == "Gear" && !g.o(card).token {
            let ctrl = g.o(o).ctrl;
            let n = g.played[ctrl].iter().filter(|n| specs()[n.as_str()].type_ == "Gear").count();
            if n == 1 {
                // first non-token gear
                trig(g, o, "Jayce, Brilliant Inventor", f("_jayce_res", jayce_res), Map::new(), true, f("_jayce_choose", jayce_choose), Value::None);
            }
        }
    }
    Ok(Value::None)
}

// Keeper of Masks — "[Hidden] [Temporary] When you play me, play two Reflection unit tokens here. They become
// copies of me."
fn keeper_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let Some(h) = here(g, it) else { return Ok(Value::None) };
    let ctrl = itc(g, it);
    for _ in 0..2 {
        if let Some(t) = play_token(g, "Reflection", ctrl, h, false)? {
            g.o_mut(t).copy = Some("Keeper of Masks".to_string()); // copyable traits incl. Might and keywords
        }
    }
    Ok(Value::None)
}

// Lillia, Fae Fawn — "[Accelerate] When I move from a location, play a 3 might Sprite unit token with [Temporary]
// there."  ("there" is referenced from the trigger condition, rule 359.3.f.3 Lillia example)
fn lillia(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "move" && is_obj(&info.g("obj"), o) {
        let res = f("_lillia.res", |g, a| {
            let it = a[0].item();
            let (c, frm) = (itc(g, it), g.it(it).data.g("frm").loc());
            play_token(g, "Sprite", c, frm, false)?;
            Ok(Value::None)
        });
        trig(g, o, "Lillia, Fae Fawn", res, map! { frm = info.g("frm") }, false, Value::None, Value::None);
    }
    Ok(Value::None)
}

/// `lambda g_, it: _src(g_, it) is not None and g_.mod(_src(g_, it), n)`
fn src_mod(n: i64) -> Value {
    f("src_mod", move |g, a| {
        let it = a[0].item();
        if src_of(g, it).is_some() {
            let me = src_of(g, it).unwrap();
            g.mod_(me, n, turn(), None)?;
        }
        Ok(Value::None)
    })
}

/// `lambda g_, it: _src(g_, it) is not None and g_.ready_obj(_src(g_, it))`
fn src_ready() -> Value {
    f("src_ready", |g, a| {
        let it = a[0].item();
        if src_of(g, it).is_some() {
            let me = src_of(g, it).unwrap();
            g.ready_obj(me, None, None)?;
        }
        Ok(Value::None)
    })
}

// Lux, Illuminated — "When you play a spell that costs 5 energy or more, give me +3 might this turn."
// (printed cost, rule 206 Lux example)
fn lux(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "played" && info.g("pid").int() as usize == g.o(o).ctrl {
        let c = info.g("card").obj();
        if ty(g, c) == "Spell" && !g.o(c).token && se(g, c) >= 5 {
            trig0(g, o, "Lux, Illuminated", src_mod(3));
        }
    }
    Ok(Value::None)
}

// Nasus, Guardian of Knowledge — "Once each turn, when an enemy unit here dies, channel 1 rune exhausted."
fn nasus(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "die" {
        let inf = info.g("info");
        let inf = inf.map();
        let ol = g.o(o).loc;
        if specs()[inf.g("spec").str()].type_ == "Unit"
            && inf.g("ctrl").int() as usize != g.o(o).ctrl
            && inf.g("loc").opt_loc() == ol
            && is_bf(ol)
        {
            let key = format!("('nasus', {}, {}, {})", g.o(o).uid, g.o(o).oid, g.turn_no);
            if stats_has(g, &key) {
                // rule 383.3.e
                return Ok(Value::None);
            }
            g.stats.add(&key, 1);
            trig0(
                g,
                o,
                "Nasus, Guardian of Knowledge",
                f("_nasus.res", |g, a| {
                    let c = itc(g, a[0].item());
                    g.channel(c, 1, true)?;
                    Ok(Value::None)
                }),
            );
        }
    }
    Ok(Value::None)
}

// Patched Porobot — "(I enter exhausted.) When you play me, if you control 3 or more other gear, draw 1."
fn porobot(g: &mut Game, a: &[Value]) -> Res<Value> {
    let o = a[0].obj();
    if g.gear(Some(g.o(o).ctrl)).len() >= 3 {
        let res = f("_porobot.res", |g, a| {
            let c = itc(g, a[0].item());
            if g.gear(Some(c)).len() >= 3 {
                g.draw(c, 1)?;
            }
            Ok(Value::None)
        });
        trig0(g, o, "Patched Porobot", res);
    }
    Ok(Value::None)
}

// Pickpocket — "When you play me, you may kill a gear with Energy cost no more than 1 energy. If you do, play a
// Gold gear token exhausted."  (tokens cost 0, rule 185.3.a.1)
fn cheap_gear(g: &Game, x: O) -> bool {
    let ob = g.o(x);
    let e = if ob.token && ob.copy.as_deref().is_none_or(|c| c.is_empty()) { 0 } else { se(g, x) };
    ty(g, x) == "Gear" && e <= 1
}

fn pickpocket_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let x = g.legal(it, 0)?;
    if let Some(x) = x {
        let c = itc(g, it);
        if !g.kill(&[x], Some(c), false)?.is_empty() {
            gold(g, c)?;
        }
    }
    Ok(Value::None)
}

fn pickpocket(g: &mut Game, a: &[Value]) -> Res<Value> {
    let o = a[0].obj();
    let opts = f("_pickpocket.opts", |g, a| {
        let c = itc(g, a[0].item());
        let xs: Vec<O> = g.gear(None).into_iter().filter(|&x| cheap_gear(g, x)).collect();
        let keys: Vec<(bool, i64)> = xs.iter().map(|&x| (g.o(x).ctrl == c, -se(g, x))).collect();
        Ok(objs(&sort_keyed(xs, keys)))
    });
    let pred = f("_cheap_gear", |g, a| Ok(cheap_gear(g, a[1].obj()).into()));
    trig(g, o, "Pickpocket", f("_pickpocket_res", pickpocket_res), Map::new(), true, tt(opts, pred), Value::None);
    Ok(Value::None)
}

/// `info["pid"] == o.ctrl`
fn pid_is_ctrl(g: &Game, info: &Map, o: O) -> bool {
    info.g("pid").int() as usize == g.o(o).ctrl
}

// Pit Crew — "When you play a gear, ready me."
fn pit_crew(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "played" && pid_is_ctrl(g, info, o) && ty(g, info.g("card").obj()) == "Gear" {
        trig0(g, o, "Pit Crew", src_ready());
    }
    Ok(Value::None)
}

// Plundering Poro — "When I conquer, play a Gold gear token exhausted."
fn gold_fn() -> Value {
    f("_gold", |g, a| {
        let c = itc(g, a[0].item());
        gold(g, c)?;
        Ok(Value::None)
    })
}

fn plunder(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "conquer" && pid_is_ctrl(g, info, o) && in_list(&info.g("units"), o) {
        trig0(g, o, "Plundering Poro", gold_fn());
    }
    Ok(Value::None)
}

// Ravenbloom Student — "When you play a spell, give me +1 might this turn."
fn ravenbloom(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "played" && pid_is_ctrl(g, info, o) && ty(g, info.g("card").obj()) == "Spell" {
        trig0(g, o, "Ravenbloom Student", src_mod(1));
    }
    Ok(Value::None)
}

/// `can=lambda g, pid, o: is_bf(o.loc)`
fn can_at_bf() -> Value {
    f("can_at_bf", |g, a| Ok(g.o(a[1].obj()).loc.is_some_and(|l| l.is_bf()).into()))
}

// Riptide Rex — "When you play me, deal 6 to an enemy unit at a battlefield."
fn rex(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ctx) = (a[0].obj(), a[1].map());
    let hb = ctx.g("hidden_bf").opt_loc();
    let res = f("_rex.res", |g, a| {
        let it = a[0].item();
        let u = g.legal(it, 0)?;
        let c = itc(g, it);
        g.deal(u, 6, "ability", Some(c), None)?;
        Ok(Value::None)
    });
    let opts = f("_rex.opts", move |g, a| {
        let c = itc(g, a[0].item());
        Ok(objs(&enemies(g, c, true, hb, true)?))
    });
    trig(g, o, "Riptide Rex", res, map! { dmg = 6 }, false, tt(opts, p_enemy_bf_v()), Value::None);
    Ok(Value::None)
}

// Rumble, Scrapper — "Your Mechs have +1 might (including me). When I hold, play a 3 might Mech unit token to
// your base."
fn rumble(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "hold" && pid_is_ctrl(g, info, o) && in_list(&info.g("units"), o) {
        let res = f("_rumble.res", |g, a| {
            let c = itc(g, a[0].item());
            play_token(g, "Mech", c, Loc::Base, false)?;
            Ok(Value::None)
        });
        trig0(g, o, "Rumble, Scrapper", res);
    }
    Ok(Value::None)
}

// Sky Cruiser — "Discard a gear, 1 energy, exhaust: Deal 4 to a unit at a battlefield."
fn cruiser_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = a[0].int() as usize;
    let gs: Vec<O> = g.p[pid].hand.iter().copied().filter(|&c| ty(g, c) == "Gear").collect();
    let keys: Vec<(i64, String)> = gs.iter().map(|&c| (se(g, c), cn(g, c))).collect();
    let gs = sort_keyed(gs, keys);
    let mut names: Vec<String> = vec![];
    let mut picks: Vec<O> = vec![];
    for c in gs {
        let n = cn(g, c);
        if !names.contains(&n) {
            names.push(n);
            picks.push(c);
        }
    }
    let mut out = vec![];
    for u in all_units(g, pid, true, None)? {
        for c in cap(g, picks.clone(), 2, Some(pid)) {
            out.push(map! { tg = tg_of(g, &[u]), disc = g.o(c).uid });
        }
    }
    Ok(maps(out))
}

fn cruiser_discard(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, ch) = (a[0].int() as usize, a[2].map());
    let d = ch.g("disc");
    let c = g.p[pid].hand.iter().copied().find(|&x| Value::from(g.o(x).uid) == d);
    if let Some(c) = c {
        g.discard(pid, c)?;
    }
    Ok(Value::None)
}

// Spectral Centaur — "When another friendly unit dies, give me +2 might this turn."
fn centaur(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "die" {
        let inf = info.g("info");
        let inf = inf.map();
        if inf.g("ctrl").int() as usize == g.o(o).ctrl
            && specs()[inf.g("spec").str()].type_ == "Unit"
            && !is_obj(&inf.g("obj"), o)
        {
            trig0(g, o, "Spectral Centaur", src_mod(2));
        }
    }
    Ok(Value::None)
}

// Sprite Mother — "When you play me, play a ready 3 might Sprite unit token with [Temporary] here."
fn mother_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    if let Some(h) = here(g, it) {
        let c = itc(g, it);
        play_token(g, "Sprite", c, h, true)?;
    }
    Ok(Value::None)
}

// Sprite Queen — "When you play me or at the start of your Beginning Phase, play a ready 3 might Sprite unit token
// with [Temporary] to your base."
fn queen_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let c = itc(g, a[0].item());
    play_token(g, "Sprite", c, Loc::Base, true)?;
    Ok(Value::None)
}

// Swain, Visionary — "[Vision] When I conquer, if you've played a non-token unit, a non-token gear, and a spell this
// turn, you score 1 point."
fn swain_ok(g: &Game, pid: usize) -> bool {
    let ts: Vec<&str> = g.played[pid].iter().filter_map(|n| specs().get(n.as_str())).map(|s| s.type_.as_str()).collect();
    ["Unit", "Gear", "Spell"].iter().all(|t| ts.contains(t))
}

fn swain(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "conquer" && pid_is_ctrl(g, info, o) && in_list(&info.g("units"), o) && swain_ok(g, g.o(o).ctrl) {
        let res = f("_swain.res", |g, a| {
            let c = itc(g, a[0].item());
            if swain_ok(g, c) {
                g.gain_point(c, "Swain, Visionary")?;
            }
            Ok(Value::None)
        });
        trig0(g, o, "Swain, Visionary", res);
    }
    Ok(Value::None)
}

// Teemo, Strategist — "[Hidden] When I defend or I'm played from [Hidden], reveal the top 5 cards of your Main Deck.
// Deal 1 to an enemy unit here for each card with [Hidden], then recycle them."
// Read as one chosen enemy unit here taking 1 per revealed Hidden card ("+1 might for each" pattern).
fn teemo_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let ctrl = itc(g, it);
    let top = g.reveal(ctrl, 5, None, true, None)?;
    let n = top.iter().filter(|&&c| hidden_card(g, c) || keywords_of(g.cname(c)).contains("Hidden")).count() as i64;
    let u = g.legal(it, 0)?;
    if u.is_some() && n != 0 {
        g.deal(u, n, "ability", Some(ctrl), None)?;
    }
    for &c in &top {
        if let Some(i) = g.p[ctrl].deck.iter().position(|&x| x == c) {
            g.p[ctrl].deck.remove(i);
        }
    }
    g.recycle_cards(ctrl, &top)?;
    Ok(Value::None)
}

fn teemo_trigger(g: &mut Game, o: O) {
    trig(g, o, "Teemo, Strategist", f("_teemo_res", teemo_res), Map::new(), false, target_here(false), Value::None);
}

// Viktor, Innovator — "When you play a card on an opponent's turn, play a 1 might Recruit unit token in your base."
fn viktor(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "played" && pid_is_ctrl(g, info, o) && g.tp != g.o(o).ctrl && !g.o(info.g("card").obj()).token {
        let res = f("_viktor.res", |g, a| {
            let c = itc(g, a[0].item());
            play_token(g, "Recruit", c, Loc::Base, false)?;
            Ok(Value::None)
        });
        trig0(g, o, "Viktor, Innovator", res);
    }
    Ok(Value::None)
}

// ====================================================================== gear
// Bottled Constellation — "At the start of your Main Phase, you may kill 3 other friendly units and/or gear to
// score 1 point."  ("kill 3 ... to" is a cost within instructions, paid at finalization: rule 383.3.b)
fn constellation_cost(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let pid = itc(g, it);
    let src = g.it(it).src;
    let cands: Vec<O> = g
        .board
        .iter()
        .copied()
        .filter(|&o| g.o(o).ctrl == pid && Some(g.o(o).uid) != src && matches!(ty(g, o), "Unit" | "Gear"))
        .collect();
    if cands.len() < 3 {
        return Ok(false.into());
    }
    let mut picked: Vec<O> = vec![];
    for _ in 0..3 {
        let rest: Vec<O> = cands.iter().copied().filter(|o| !picked.contains(o)).collect();
        let mut keys = vec![];
        for &o in &rest {
            keys.push((value(g, o)?, g.o(o).uid));
        }
        let rest = sort_keyed(rest, keys);
        picked.push(g.ask(pid, "constellation_kill", rest.iter().map(|&o| Value::Obj(o)).collect(), map! { item = it })?.obj());
    }
    g.kill(&picked, Some(pid), true)?;
    Ok(true.into())
}

fn constellation(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "main_start" && pid_is_ctrl(g, info, o) {
        let res = f("_constellation.res", |g, a| {
            let c = itc(g, a[0].item());
            g.gain_point(c, "Bottled Constellation")?;
            Ok(Value::None)
        });
        trig(g, o, "Bottled Constellation", res, Map::new(), true, Value::None, f("_constellation_cost", constellation_cost));
    }
    Ok(Value::None)
}

// Chemtech Cask — "When you play a spell on an opponent's turn, you may exhaust me to play a Gold gear token
// exhausted."
fn cask_cost(g: &mut Game, a: &[Value]) -> Res<Value> {
    let me = src_of(g, a[0].item());
    match me {
        Some(me) if !g.exhausted(me) => {
            g.set_exhausted(me, true);
            Ok(true.into())
        }
        _ => Ok(false.into()),
    }
}

fn cask(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "played"
        && pid_is_ctrl(g, info, o)
        && g.tp != g.o(o).ctrl
        && ty(g, info.g("card").obj()) == "Spell"
        && !g.exhausted(o)
    {
        trig(g, o, "Chemtech Cask", gold_fn(), Map::new(), true, Value::None, f("_cask_cost", cask_cost));
    }
    Ok(Value::None)
}

// Gutter Palace — "At the start of your Beginning Phase, if you have exactly 4 cards in hand and exactly 4 units at
// battlefields, you win the game. Discard 1, exhaust: Play a 1 might Bird unit token with [Deflect]."
fn palace_ok(g: &Game, pid: usize) -> bool {
    g.p[pid].hand.len() == 4 && g.units(Some(pid), None).into_iter().filter(|&u| g.at_bf(u)).count() == 4
}

fn palace(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "beginning_start" && pid_is_ctrl(g, info, o) && palace_ok(g, g.o(o).ctrl) {
        let res = f("_palace.res", |g, a| {
            let c = itc(g, a[0].item());
            g.win(c)?;
            Ok(Value::None)
        });
        trig0(g, o, "Gutter Palace", res);
    }
    Ok(Value::None)
}

fn palace_discard(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = a[0].int() as usize;
    let hand_: Vec<Value> = g.p[pid].hand.iter().map(|&c| Value::Obj(c)).collect();
    let c = g.ask(pid, "discard", hand_, map! { reason = "gutter_palace" })?;
    if let Some(c) = c.opt_obj() {
        g.discard(pid, c)?;
    }
    Ok(Value::None)
}

// Mushroom Pouch — "At the start of your Beginning Phase, if you control a facedown card at a battlefield, draw 1."
fn pouch(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "beginning_start"
        && pid_is_ctrl(g, info, o)
        && g.bfs.iter().any(|b| b.facedowns.iter().any(|&c| g.o(c).owner == g.o(o).ctrl))
    {
        trig0(g, o, "Mushroom Pouch", draw_fn(1));
    }
    Ok(Value::None)
}

// Sprite Fountain — "[Temporary] When you play this, play a ready 3 might Sprite unit token with [Temporary] to your
// base. [Deathknell] Repeat this gear's play effect."
fn fountain_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let c = itc(g, a[0].item());
    play_token(g, "Sprite", c, Loc::Base, true)?;
    Ok(Value::None)
}

// Sumpworks Map — "[Reaction] [Temporary] When an opponent scores, draw 1."
// Scoring (rule 469): a Conquer or Hold that scores, or a point scored by a card effect ("score 1 point"); gaining
// a point from an opponent's Burn Out is not scoring.
fn sump_map(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    let opp = 1 - g.o(o).ctrl;
    let mut hit = false;
    if matches!(ev, "conquer" | "hold")
        && info.g("pid").int() as usize == opp
        && g.can_score(opp, info.g("bf").int() as usize)?
    {
        hit = true;
    } else if ev == "point" && info.g("pid").int() as usize == opp {
        let why = info.g("why");
        let why = why.str();
        if why != "burn out" && !(why.starts_with("conquer") || why.starts_with("hold")) {
            hit = true;
        }
    }
    if hit {
        trig0(g, o, "Sumpworks Map", draw_fn(1));
    }
    Ok(Value::None)
}

// The Zero Drive — "[Equip] 1 energy and 1 mind rune. 3 energy and 1 mind rune, Banish this: Play all units
// banished with this, ignoring their costs. (Use only if unattached.)"; Might Bonus +2 and Effect Text
// "[Deathknell] — Banish me." (card image).
// NON PORTÉE (carte bloquée) : le Python range la liste des cartes bannies sur l'objet lui-même
// (`x.__dict__.setdefault("zero_banished", [])`), un attribut libre qui survit aux changements de zone et aux
// copies de l'IA. `game::Obj` n'a pas de champ équivalent (les `xtags` sont remis à zéro à chaque changement de
// zone) : il faut un champ `zero_banished: Vec<(i64, O)>` (ou des attributs libres) dans `Obj` pour la traduire
// fidèlement.

// World Atlas — "[Equip] 1 mind rune"; Might Bonus +2 and Effect Text "When I hold, play two Gold gear tokens
// exhausted." (card image)
fn atlas(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (_gear, unit, ev, info) = (a[0].obj(), a[1].obj(), a[2].str(), a[3].map());
    if ev == "hold" && info.g("pid").int() as usize == g.o(unit).ctrl && in_list(&info.g("units"), unit) {
        let res = f("_atlas.res", |g, a| {
            let c = itc(g, a[0].item());
            gold(g, c)?;
            gold(g, c)?;
            Ok(Value::None)
        });
        let (ctrl, uid) = (g.o(unit).ctrl, g.o(unit).uid);
        g.queue_trigger(ctrl, "World Atlas", res, Map::new(), Some(uid), false, Value::None, Value::None);
    }
    Ok(Value::None)
}

// ====================================================================== spells
/// `_dmg_spell(n)`
fn dmg_spell(n: i64) -> Value {
    f("_dmg_spell", move |g, a| {
        let it = a[0].item();
        let u = g.legal(it, 0)?;
        let c = itc(g, it);
        g.deal(u, n, "spell", Some(c), None)?;
        Ok(Value::None)
    })
}

/// `_minus_spell(n, minimum)`
fn minus_spell(n: i64, minimum: Option<i64>) -> Value {
    f("_minus_spell", move |g, a| {
        let u = g.legal(a[0].item(), 0)?;
        minus(g, u, n, minimum)?;
        Ok(Value::None)
    })
}

/// `_P_perm`
fn p_perm(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (it, o) = (a[0].item(), a[1].obj());
    Ok((matches!(ty(g, o), "Unit" | "Gear") && hb_ok(g, it, o)).into())
}

/// `ctx["hidden_bf"]`
fn ctx_hb(ctx: &Value) -> Option<Loc> {
    ctx.map().g("hidden_bf").opt_loc()
}

// Acceleration Gate — "Ready up to 4 units, gear, and/or runes."
// Runes are chosen as targets too (rule 355.9.a): they are kept as uids in choice['runes']. Readying something
// that is not exhausted does nothing, so runes about to be exhausted to pay for the spell can be chosen.
fn gate_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = a[0].int() as usize;
    let us: Vec<O> = g.units(Some(pid), None).into_iter().filter(|&u| g.exhausted(u)).collect();
    let mut keys = vec![];
    for &u in &us {
        keys.push(-value(g, u)?);
    }
    let mut perms = sort_keyed(us, keys);
    let gs: Vec<O> = g.gear(Some(pid)).into_iter().filter(|&x| g.exhausted(x)).collect();
    let keys: Vec<i64> = gs.iter().map(|&x| -se(g, x)).collect();
    perms.extend(sort_keyed(gs, keys));
    let rs = g.p[pid].runes.clone();
    let keys: Vec<(bool, String, u32)> = rs.iter().map(|&r| (!g.r(r).exhausted, g.r(r).domain.clone(), g.r(r).uid)).collect();
    let rs = sort_keyed(rs, keys);
    let mut ks: Vec<usize> = [4usize, 2, 0].iter().map(|&n| perms.len().min(n)).collect();
    ks.sort();
    ks.dedup();
    ks.reverse();
    let mut out: Vec<Map> = vec![];
    for k in ks {
        let runes = Value::Tuple(rs.iter().take(4 - k).map(|&r| Value::from(g.r(r).uid)).collect());
        let ch = map! { tg = tg_of(g, &perms[..k]), runes = runes };
        if !out.contains(&ch) {
            out.push(ch);
        }
    }
    Ok(maps(out))
}

fn gate(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    for i in 0..g.it(it).targets.len() {
        if let Some(o) = g.legal(it, i)? {
            g.ready_obj(o, None, None)?;
        }
    }
    for uid in g.it(it).data.g("runes").list().to_vec() {
        if let Some(r) = rune_of(g, uid.int()) {
            g.r_mut(r).exhausted = false;
        }
    }
    Ok(Value::None)
}

// Arcane Shift — "[Action] Banish a friendly unit, then its owner plays it, ignoring its cost. Deal 3 to an enemy
// unit at a battlefield. Banish this."
fn shift(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    if let Some(u) = g.legal(it, 0)? {
        let (owner, tok) = (g.o(u).owner, g.o(u).token);
        g.to_zone(u, "banish", false, None)?; // a token ceases to exist (rule 186.1)
        if !tok && g.p[owner].banish.contains(&u) {
            play_unit_free(g, owner, u, "banish", None, false, None)?;
        }
    }
    let e = g.legal(it, 1)?;
    if e.is_some() {
        let c = itc(g, it);
        g.deal(e, 3, "spell", Some(c), None)?;
    }
    Ok(Value::None)
}

fn shift_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = a[0].int() as usize;
    let fr = friends(g, pid, false, None)?;
    let mut keys = vec![];
    for &u in &fr {
        keys.push((!g.exhausted(u), -g.o(u).damage as f64, -value(g, u)?));
    }
    let fr = cap(g, sort_keyed(fr, keys), 2, Some(pid));
    let es0 = enemies(g, pid, true, None, true)?;
    let es = cap(g, es0, 4, Some(pid));
    let mut out = vec![];
    for &e in &es {
        for &fu in &fr {
            out.push(map! { tg = tg_of(g, &[fu, e]) });
        }
    }
    Ok(maps(out))
}

// Convergent Mutation — "[Reaction] Choose a friendly unit. This turn, increase its Might to the Might of another
// friendly unit."
fn mutation(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let (x, y) = (g.legal(it, 0)?, g.legal(it, 1)?);
    let (Some(x), Some(y)) = (x, y) else { return Ok(Value::None) };
    if x == y {
        return Ok(Value::None);
    }
    let diff = g.might(y)? - g.might(x)?;
    if diff > 0 {
        g.mod_(x, diff, turn(), None)?;
    }
    Ok(Value::None)
}

fn mutation_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, hb) = (a[0].int() as usize, ctx_hb(&a[1]));
    let fr = friends(g, pid, false, hb)?;
    let mut keys = vec![];
    for &u in &fr {
        keys.push(g.might(u)?);
    }
    let mut out: Vec<(O, O)> = vec![];
    for x in sort_keyed(fr, keys) {
        let fs = friends(g, pid, false, None)?;
        let mut keys = vec![];
        for &u in &fs {
            keys.push(-g.might(u)?);
        }
        for y in sort_keyed(fs, keys) {
            if y != x && g.might(y)? > g.might(x)? {
                out.push((x, y));
            }
        }
    }
    let mut keys = vec![];
    for &(x, y) in &out {
        keys.push((!g.in_combat(x), g.might(x)? - g.might(y)?));
    }
    let out = sort_keyed(out, keys);
    Ok(maps(out.into_iter().map(|(x, y)| map! { tg = tg_of(g, &[x, y]) }).collect()))
}

// Crescent Strike — "[Action] Choose a battlefield and an enemy unit there. Deal 4 to that unit and 1 to each other
// enemy unit there."
fn crescent(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let bf = g.it(it).data.g("bf").int() as usize;
    let c = itc(g, it);
    let u = g.legal(it, 0)?;
    if u.is_some() {
        g.deal(u, 4, "spell", Some(c), None)?;
    }
    let first = g.it(it).data.g("tg").list()[0].clone();
    let es: Vec<O> = g
        .units(Some(1 - c), None)
        .into_iter()
        .filter(|&x| g.o(x).loc == Some(Loc::Bf(bf)) && Value::from(g.o(x).uid) != first)
        .collect();
    for e in es {
        g.deal(Some(e), 1, "spell", Some(c), None)?;
    }
    Ok(Value::None)
}

fn crescent_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, hb) = (a[0].int() as usize, ctx_hb(&a[1]));
    let mut out: Vec<Map> = vec![];
    for u in enemies(g, pid, true, hb, true)? {
        let bf = g.o(u).loc.and_then(|l| l.bf()).unwrap();
        out.push(map! { bf = bf, tg = tg_of(g, &[u]) });
    }
    let mut keys = vec![];
    for c in &out {
        let bf = c.g("bf").int() as usize;
        let n = g.units(Some(1 - pid), Some(Loc::Bf(bf))).len() as f64;
        let u = g.obj(c.g("tg").list()[0].int() as u32).unwrap();
        keys.push(-(n + value(g, u)? / 4.0));
    }
    Ok(maps(sort_keyed(out, keys)))
}

// Deadly Flourish — "Deal 3 to an enemy unit. When it dies this turn, play a Gold gear token exhausted."
fn flourish_die(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (eid, info) = (a[0].int() as u64, a[1].map());
    let o = info.g("info").map().g("obj").obj();
    let eff = g.eff(eid).cloned().unwrap_or_default();
    if g.o(o).uid as i64 == eff.g("uid").int() && g.o(o).oid == eff.g("oid").int() + 1 {
        // the same incarnation (to_zone adds 1 to oid)
        g.remove_effect(eid);
        let pid = eff.g("pid").int() as usize;
        g.queue_trigger(pid, "Deadly Flourish", gold_fn(), Map::new(), None, false, Value::None, Value::None);
    }
    Ok(Value::None)
}

fn flourish(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
    let c = itc(g, it);
    g.deal(Some(u), 3, "spell", Some(c), None)?;
    let (uid, oid) = (g.o(u).uid, g.o(u).oid);
    g.add_effect(mk(vec![
        ("on", "die".into()),
        ("fn", f("_flourish_die", flourish_die)),
        ("dur", "turn".into()),
        ("uid", uid.into()),
        ("oid", oid.into()),
        ("pid", c.into()),
    ]));
    Ok(Value::None)
}

// Eclipse — "[Reaction] Give a unit -4 might this turn. [Predict]."
fn eclipse(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let u = g.legal(it, 0)?;
    minus(g, u, 4, None)?;
    let c = itc(g, it);
    g.predict(c, 1)?;
    Ok(Value::None)
}

/// `_units_choices(g, pid, ctx, at_bf=False)`
fn units_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, hb) = (a[0].int() as usize, ctx_hb(&a[1]));
    let us = all_units(g, pid, false, hb)?;
    Ok(maps(tg_choices(g, &us)))
}

/// `lambda g, pid, ctx: tg_choices(enemies(g, pid, at_bf, ctx["hidden_bf"]))`
fn enemy_choices(at_bf: bool) -> Value {
    f("enemy_choices", move |g, a| {
        let (pid, hb) = (a[0].int() as usize, ctx_hb(&a[1]));
        let us = enemies(g, pid, at_bf, hb, true)?;
        Ok(maps(tg_choices(g, &us)))
    })
}

// Hostile Takeover — "[Hidden] Take control of an enemy unit at a battlefield. Ready it. Lose control of that unit
// and recall it at end of turn."  (a combat or a conquest follows from the cleanup, rule 323)
fn takeover_end(g: &mut Game, a: &[Value]) -> Res<Value> {
    let eid = a[0].int() as u64;
    let eff = g.eff(eid).cloned().unwrap_or_default();
    g.remove_effect(eid);
    let pid = eff.g("pid").int() as usize;
    g.queue_trigger(pid, "Hostile Takeover (end of turn)", f("_takeover_back", takeover_back), eff, None, false, Value::None, Value::None);
    Ok(Value::None)
}

fn takeover_back(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let d = g.it(it).data.clone();
    let Some(u) = g.obj(d.g("uid").int() as u32) else { return Ok(Value::None) };
    if g.o(u).oid != d.g("oid").int() {
        return Ok(Value::None);
    }
    if g.o(u).ctrl as i64 == d.g("pid").int() {
        g.o_mut(u).ctrl = d.g("prev").int() as usize;
    }
    g.recall(u);
    Ok(Value::None)
}

fn takeover(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
    let prev = g.o(u).ctrl;
    let c = itc(g, it);
    g.o_mut(u).ctrl = c;
    let r = g.repr(u);
    g.log(|| format!("  P{c} takes control of {r}"));
    g.need_cleanup = true;
    g.ready_obj(u, None, None)?;
    let (uid, oid) = (g.o(u).uid, g.o(u).oid);
    g.add_effect(mk(vec![
        ("on", "end_turn".into()),
        ("fn", f("_takeover_end", takeover_end)),
        ("dur", "turn".into()),
        ("uid", uid.into()),
        ("oid", oid.into()),
        ("pid", c.into()),
        ("prev", prev.into()),
    ]));
    Ok(Value::None)
}

// Mesmerize — "[Reaction] Choose one — Return a friendly unit to its owner's hand. / Give an enemy unit -2 might
// this turn."
fn mesmerize(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
    if g.it(it).data.g("mode").str() == "return" {
        g.to_zone(u, "hand", false, None)?;
    } else {
        g.mod_(u, -2, turn(), None)?;
    }
    Ok(Value::None)
}

fn mesmerize_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, hb) = (a[0].int() as usize, ctx_hb(&a[1]));
    let mut out: Vec<Map> = vec![];
    for u in enemies(g, pid, false, hb, true)? {
        out.push(map! { mode = "minus", tg = tg_of(g, &[u]) });
    }
    let fr = friends(g, pid, false, hb)?;
    let mut keys = vec![];
    for &u in &fr {
        keys.push((-g.o(u).damage as f64, -value(g, u)?));
    }
    for u in sort_keyed(fr, keys) {
        out.push(map! { mode = "return", tg = tg_of(g, &[u]) });
    }
    Ok(maps(out))
}

// Moonfall — "[Action] Choose a battlefield where you have units. You may move up to one enemy unit to that
// battlefield. Then give enemy units there -2 might this turn."
fn moonfall(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let bf = Loc::Bf(g.it(it).data.g("bf").int() as usize);
    let c = itc(g, it);
    if !g.units(Some(c), None).into_iter().any(|u| g.o(u).loc == Some(bf)) {
        return Ok(Value::None); // the chosen battlefield is no longer a legal choice
    }
    if !g.it(it).targets.is_empty() {
        if let Some(u) = g.legal(it, 0)? {
            if g.o(u).loc != Some(bf) {
                g.move_(&[u], bf, Some(c), false)?;
            }
        }
    }
    let es: Vec<O> = g.units(Some(1 - c), None).into_iter().filter(|&x| g.o(x).loc == Some(bf)).collect();
    for e in es {
        g.mod_(e, -2, turn(), None)?;
    }
    Ok(Value::None)
}

fn moonfall_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = a[0].int() as usize;
    let mut out: Vec<Map> = vec![];
    for bf in g.bf_locs() {
        if !g.units(Some(pid), None).into_iter().any(|u| g.o(u).loc == Some(bf)) {
            continue;
        }
        let b = bf.bf().unwrap();
        let es = enemies(g, pid, false, None, true)?;
        let v: Vec<Map> = es.into_iter().filter(|&u| g.o(u).loc != Some(bf)).map(|u| map! { bf = b, tg = tg_of(g, &[u]) }).collect();
        out.extend(cap(g, v, 3, Some(pid)));
        out.push(map! { bf = b, tg = Value::Tuple(vec![]) });
    }
    let keys: Vec<i64> = out
        .iter()
        .map(|c| -(g.units(Some(1 - pid), Some(Loc::Bf(c.g("bf").int() as usize))).len() as i64 + c.g("tg").list().len() as i64))
        .collect();
    Ok(maps(sort_keyed(out, keys)))
}

// Portal Rescue — "[Action] Banish a friendly unit, then play it to base, ignoring its cost."
fn portal(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
    let tok = g.o(u).token;
    g.to_zone(u, "banish", false, None)?;
    let owner = g.o(u).owner;
    if !tok && g.p[owner].banish.contains(&u) {
        let c = itc(g, it);
        play_card(g, c, u, "banish", map! { loc = Loc::Base, free = true }, true)?;
    }
    Ok(Value::None)
}

/// `tg_choices(sorted(friends(g, pid, False, ctx["hidden_bf"]), key=lambda u: (-u.damage, not u.exhausted,
/// -value(g, u))))` (Portal Rescue, Temporal Breach)
fn friends_damaged_first(g: &mut Game, pid: usize, hb: Option<Loc>) -> Res<Vec<O>> {
    let fr = friends(g, pid, false, hb)?;
    let mut keys = vec![];
    for &u in &fr {
        keys.push((-g.o(u).damage as f64, !g.exhausted(u), -value(g, u)?));
    }
    Ok(sort_keyed(fr, keys))
}

// Promising Future — "Each player looks at the top 5 cards of their Main Deck, chooses one, then recycles the rest.
// Starting with the next player, each player plays those cards, ignoring Energy costs. (They must still pay Power
// costs.)"  A chosen card that can't be played (no legal choice, Power unpayable) stays on top of its deck.
fn pf_plays(g: &mut Game, pid: usize, c: O) -> Res<Vec<Map>> {
    let Some(im) = g.impl_o(c) else { return Ok(vec![]) };
    let t = ty(g, c);
    let chs: Vec<Map> = if t == "Unit" {
        let mut ls = vec![Loc::Base];
        ls.extend(g.bfs.iter().filter(|b| b.ctrl == Some(pid)).map(|b| Loc::Bf(b.idx)));
        ls.into_iter().map(|l| map! { loc = l }).collect()
    } else if t == "Gear" {
        vec![map! { loc = Loc::Base }]
    } else {
        let base = if !im.choices.is_none() {
            map_vec(&im.choices.call(g, &[pid.into(), map! { hidden_bf = Value::None, card = c }.into()])?)
        } else {
            vec![Map::new()]
        };
        cap(g, base, 8, Some(pid))
    };
    let mut out = vec![];
    for ch in chs {
        let mut ch = ch;
        ch.set("ignore_energy", true);
        ch.set("pay_power", true);
        let mut ch2 = ch.clone();
        ch2.set("free", false);
        let (_, reqs) = total_cost(g, pid, c, &ch2, "hand")?;
        if g.can_pay(pid, 0, &reqs, &Value::None)? {
            out.push(ch);
        }
    }
    Ok(out)
}

fn promising(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let ctrl = itc(g, it);
    let mut picks: [Option<O>; 2] = [None, None];
    for pid in [ctrl, 1 - ctrl] {
        let d5: Vec<O> = g.p[pid].deck.iter().take(5).copied().collect();
        let top = g.look(pid, &d5, None)?;
        if top.is_empty() {
            continue;
        }
        let mut keys = vec![];
        for &x in &top {
            keys.push((pf_plays(g, pid, x)?.is_empty(), -se(g, x), cn(g, x)));
        }
        let sorted = sort_keyed(top.clone(), keys);
        let c = g.ask(pid, "promising_pick", sorted.iter().map(|&x| Value::Obj(x)).collect(), Map::new())?.obj();
        let rest: Vec<O> = top.iter().copied().filter(|&x| x != c).collect();
        for &x in &rest {
            let i = g.p[pid].deck.iter().position(|&y| y == x).unwrap();
            g.p[pid].deck.remove(i);
        }
        g.recycle_cards(pid, &rest)?;
        picks[pid] = Some(c);
    }
    for pid in [1 - ctrl, ctrl] {
        // starting with the next player
        let Some(c) = picks[pid] else { continue };
        if !g.p[pid].deck.contains(&c) {
            continue;
        }
        let plays = pf_plays(g, pid, c)?;
        if plays.is_empty() {
            continue;
        }
        let ch = g.ask(pid, "promising_play", maps(plays).list().to_vec(), map! { card = c })?.map().clone();
        play_card(g, pid, c, "deck", ch, true)?;
    }
    Ok(Value::None)
}

// Retreat — "[Reaction] Return a friendly unit to its owner's hand. Its owner channels 1 rune exhausted."
fn retreat(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
    let owner = g.o(u).owner;
    g.to_zone(u, "hand", false, None)?;
    g.channel(owner, 1, true)?;
    Ok(Value::None)
}

// Rocket Barrage — "[Repeat] 4 energy and 1 mind rune. Choose one — Deal 4 to a unit in a base. / Kill a gear."
// The mode follows from the chosen object (a unit or a gear), so each repetition can choose another mode.
fn barrage_pred(g: &mut Game, a: &[Value]) -> Res<Value> {
    let o = a[1].obj();
    Ok(((ty(g, o) == "Unit" && g.o(o).loc == Some(Loc::Base)) || ty(g, o) == "Gear").into())
}

fn barrage_one(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let Some(o) = g.legal(it, 0)? else { return Ok(Value::None) };
    let c = itc(g, it);
    if ty(g, o) == "Unit" {
        g.deal(Some(o), 4, "spell", Some(c), None)?;
    } else {
        g.kill(&[o], Some(c), false)?;
    }
    Ok(Value::None)
}

fn barrage_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = a[0].int() as usize;
    let us: Vec<O> = enemies(g, pid, false, None, true)?.into_iter().filter(|&u| g.o(u).loc == Some(Loc::Base)).collect();
    let gs: Vec<O> = g.gear(None).into_iter().filter(|&x| g.o(x).ctrl != pid).collect();
    let keys: Vec<i64> = gs.iter().map(|&x| -se(g, x)).collect();
    let gs = sort_keyed(gs, keys);
    let mut all = cap(g, us, 5, Some(pid));
    all.extend(cap(g, gs, 5, Some(pid)));
    Ok(maps(tg_choices(g, &all)))
}

// Shock Blast — "[Action] This costs 2 energy less if you control something that's [Empowered]. Deal 4 to a unit at
// a battlefield."
fn empowered_any(g: &Game, pid: usize) -> bool {
    g.o(g.p[pid].legend).empowered || g.board.iter().any(|&o| g.o(o).empowered && g.o(o).ctrl == pid)
}

// Singularity — "Deal 6 to each of up to two units."
fn singularity(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c = itc(g, it);
    for i in 0..g.it(it).targets.len() {
        let u = g.legal(it, i)?;
        if u.is_some() {
            g.deal(u, 6, "spell", Some(c), None)?;
        }
    }
    Ok(Value::None)
}

fn singularity_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    // "each of up to two units": any units (a human may choose a friendly one), and zero targets (rule 355.13)
    let (pid, hb) = (a[0].int() as usize, ctx_hb(&a[1]));
    let pool = if full_choices(g, Some(pid)) { all_units(g, pid, false, hb)? } else { enemies(g, pid, false, hb, true)? };
    let es = cap(g, pool, 4, Some(pid));
    let mut out: Vec<Vec<O>> = vec![];
    for (i, &x) in es.iter().enumerate() {
        for &y in &es[i + 1..] {
            out.push(vec![x, y]);
        }
    }
    for &x in &es {
        out.push(vec![x]);
    }
    let mut keys = vec![];
    for c in &out {
        let mut s = 0.0f64;
        for &u in c {
            let v = value(g, u)?;
            let enemy = (g.o(u).ctrl != pid) as i64 as f64;
            let low = (g.might(u)? - g.o(u).damage <= 6) as i64 as f64;
            s += v * enemy * low;
        }
        keys.push(-s);
    }
    let mut res: Vec<Map> = sort_keyed(out, keys).into_iter().map(|c| map! { tg = tg_of(g, &c) }).collect();
    res.push(map! { tg = Value::Tuple(vec![]) });
    Ok(maps(res))
}

// Siphon Power — "[Reaction] Choose a battlefield. Give friendly units there +1 might this turn and enemy units
// there -1 might this turn, to a minimum of 1 might."
fn siphon(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let bf = Loc::Bf(g.it(it).data.g("bf").int() as usize);
    let c = itc(g, it);
    let us: Vec<O> = g.units(None, None).into_iter().filter(|&x| g.o(x).loc == Some(bf)).collect();
    for u in us {
        if g.o(u).ctrl == c {
            g.mod_(u, 1, turn(), None)?;
        } else {
            g.mod_(u, -1, turn(), Some(1))?;
        }
    }
    Ok(Value::None)
}

fn siphon_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let _pid = a[0].int();
    let out: Vec<Map> =
        g.bfs.iter().filter(|b| !g.units(None, Some(Loc::Bf(b.idx))).is_empty()).map(|b| map! { bf = b.idx }).collect();
    let keys: Vec<(bool, i64)> = out
        .iter()
        .map(|c| {
            let b = c.g("bf").int() as usize;
            (g.sd.as_ref().is_none_or(|sd| sd.bf != b), -(g.units(None, Some(Loc::Bf(b))).len() as i64))
        })
        .collect();
    Ok(maps(sort_keyed(out, keys)))
}

// Smoke and Mirrors — "[Hidden] [Action] Choose a unit you control and another unit you control at a different
// location. If at least one of them has [Temporary], move each to the other's location. Draw 1."
// Played from Hidden, only the first unit must be at that battlefield (rule 811.1.d.2.a example).
fn smoke_mirrors(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c = itc(g, it);
    let (x, y) = (g.legal(it, 0)?, g.legal(it, 1)?);
    if let (Some(x), Some(y)) = (x, y) {
        if g.o(x).loc != g.o(y).loc && (g.has_kw(x, "Temporary")? || g.has_kw(y, "Temporary")?) {
            let (la, lb) = (g.o(x).loc, g.o(y).loc);
            g.move_(&[x], lb.unwrap(), Some(c), false)?;
            g.move_(&[y], la.unwrap(), Some(c), false)?;
        }
    }
    g.draw(c, 1)?;
    Ok(Value::None)
}

fn smoke_mirrors_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, hb) = (a[0].int() as usize, ctx_hb(&a[1]));
    let mut out: Vec<(O, O)> = vec![];
    for x in friends(g, pid, false, hb)? {
        for y in friends(g, pid, false, None)? {
            if y != x && g.o(y).loc != g.o(x).loc {
                out.push((x, y));
            }
        }
    }
    let mut keys = vec![];
    for &(x, y) in &out {
        let any = g.has_kw(x, "Temporary")? || g.has_kw(y, "Temporary")?;
        keys.push(!any);
    }
    let out = sort_keyed(out, keys);
    Ok(maps(out.into_iter().map(|(x, y)| map! { tg = tg_of(g, &[x, y]) }).collect()))
}

// Temporal Breach — "[Hidden] Banish a unit, then its owner plays it to the same location, ignoring its cost."
fn breach(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
    let (loc, owner, tok) = (g.o(u).loc, g.o(u).owner, g.o(u).token);
    g.to_zone(u, "banish", false, None)?;
    if !tok && g.p[owner].banish.contains(&u) {
        play_card(g, owner, u, "banish", map! { loc = loc, free = true }, true)?;
    }
    Ok(Value::None)
}

// Unchecked Power — "Exhaust all friendly units, then deal 12 to ALL units at battlefields."
fn unchecked(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c = itc(g, it);
    for u in g.units(Some(c), None) {
        g.set_exhausted(u, true);
    }
    let us: Vec<O> = g.units(None, None).into_iter().filter(|&x| g.at_bf(x)).collect();
    for u in us {
        g.deal(Some(u), 12, "spell", Some(c), None)?;
    }
    Ok(Value::None)
}

// ====================================================================== cards using the engine hooks (integration)
/// `_play_spell_limited(g, pid, c, src, flags, extra)` : play the spell c from src as a limited play (rule 419.3)
/// with cost flags (ignore_energy...): its choices and optional additional costs (Repeat) are offered; the cost left
/// (Power, additional costs) is paid.
fn play_spell_limited(g: &mut Game, pid: usize, c: O, src: &str, flags: &Map, extra: Option<&Map>) -> Res<Value> {
    let Some(im) = g.impl_o(c) else { return Ok(Value::None) };
    if play_forbidden(g, pid, c, src)? {
        return Ok(Value::None);
    }
    let ctx = map! { hidden_bf = Value::None, card = c, src = src };
    let base: Vec<Map> = if full_choices(g, Some(pid)) && !im.all_choices.is_none() {
        let r = im.all_choices.call(g, &[pid.into(), ctx.clone().into()])?;
        if r.truthy() {
            map_vec(&r)
        } else {
            vec![Map::new()]
        }
    } else if !im.choices.is_none() {
        map_vec(&im.choices.call(g, &[pid.into(), ctx.clone().into()])?)
    } else {
        vec![Map::new()]
    };
    let mut out: Vec<(Map, i64, Vec<Req>)> = vec![];
    let base = cap(g, base, 8, Some(pid));
    for ch in spell_variants(g, pid, c, im, base, &ctx, false)? {
        let ch = ch.merged(flags);
        let (e, reqs) = total_cost(g, pid, c, &ch, src)?;
        let pc = pay_ctx(g, c);
        if g.can_pay(pid, e, &reqs, &pc)? {
            out.push((ch, e, reqs));
        }
    }
    if out.is_empty() {
        return Ok(Value::None);
    }
    let ch = if out.len() > 1 {
        let opts: Vec<Value> = out.iter().map(|x| Value::Map(x.0.clone())).collect();
        g.ask(pid, "play_choice", opts, map! { card = c })?.map().clone()
    } else {
        out[0].0.clone()
    };
    let (_, e, reqs) = out.iter().find(|x| x.0 == ch).cloned().unwrap();
    let mut ch = match extra {
        Some(x) => ch.merged(x),
        None => ch,
    };
    if e != 0 || !reqs.is_empty() {
        ch.set("pay_cost", Value::Tuple(vec![e.into(), Value::Tuple(reqs_to_value(&reqs).list().to_vec())]));
    }
    play_card(g, pid, c, src, ch, true)
}

// Jayce, Man of Progress — "When you play me, you may kill a friendly gear. If you do, you may play a gear with Energy
// cost no more than 7 energy from hand this turn, ignoring its Energy cost." (a play permission, consumed when used)
fn jayce_perm(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (card, src) = (a[1].obj(), a[2].str());
    Ok((src == "hand" && ty(g, card) == "Gear" && se(g, card) <= 7).into())
}

fn mop_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let c = itc(g, it);
    let gs = g.gear(Some(c));
    let keys: Vec<(i64, u32)> = gs.iter().map(|&x| (se(g, x) + 2 * sp(g, x), g.o(x).uid)).collect();
    let gs = sort_keyed(gs, keys);
    if gs.is_empty() {
        return Ok(Value::None);
    }
    let x = if gs.len() > 1 {
        g.ask(c, "jayce_kill_gear", gs.iter().map(|&x| Value::Obj(x)).collect(), Map::new())?.obj()
    } else {
        gs[0]
    };
    if !g.kill(&[x], Some(c), false)?.is_empty() {
        let id = g.it(it).id;
        g.add_effect(mk(vec![
            ("kind", "play_perm".into()),
            ("pid", c.into()),
            ("key", format!("jayce-{id}").into()),
            ("fn", f("_jayce_perm", jayce_perm)),
            ("choice", map! { ignore_energy = true }.into()),
            ("dur", "turn".into()),
        ]));
    }
    Ok(Value::None)
}

// Temporal Portal — "1 rune of any type, [E]: Give the next spell you play this turn [Repeat] equal to its cost."
// (its cost: the spell's printed Energy and Power cost)
fn portal_cost(g: &mut Game, a: &[Value]) -> Res<Value> {
    let card = a[0].obj();
    let s = g.spec(card);
    let reqs = g.power_reqs(&s.domains, s.p as usize);
    Ok(cost_v(s.e as i64, &reqs))
}

fn portal_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let (c, id) = (itc(g, it), g.it(it).id);
    g.add_effect(mk(vec![
        ("kind", "grant_repeat".into()),
        ("pid", c.into()),
        ("cost", f("_portal_cost", portal_cost)),
        ("next", true.into()),
        ("key", format!("portal-{id}").into()),
        ("dur", "turn".into()),
    ]));
    Ok(Value::None)
}

// Jhin, Meticulous Killer — "[Vision] If you've spent 4 energy or more to play a spell this turn, you may play me for
// 1 mind rune." (an alternative cost, rule 356.1.a)
fn mk_alt(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, src) = (a[0].int() as usize, a[2].str());
    if src == "trash" && g.effects_of("play_from_trash", Some(pid)).is_empty() {
        return Ok(Value::List(vec![]));
    }
    if g.hist.spell_e[pid].iter().any(|&e| e >= 4) {
        return Ok(Value::List(vec![map! { key = "jhin", e = 0, reqs = reqs_to_value(&[mind()]) }.into()]));
    }
    Ok(Value::List(vec![]))
}

// Guerilla Warfare — "Return up to two cards with [Hidden] from your trash to your hand. You can hide cards ignoring
// costs this turn."
fn guerilla(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let pid = itc(g, it);
    for _ in 0..2 {
        let cs: Vec<O> = g.p[pid].trash.iter().copied().filter(|&c| keywords_of(g.cname(c)).contains("Hidden")).collect();
        let keys: Vec<(i64, u32)> = cs.iter().map(|&c| (-(se(g, c) + 2 * sp(g, c)), g.o(c).uid)).collect();
        let cs = sort_keyed(cs, keys);
        if cs.is_empty() {
            break;
        }
        let mut opts: Vec<Value> = cs.iter().map(|&c| Value::Obj(c)).collect();
        opts.push(Value::None);
        let Some(c) = g.ask(pid, "guerilla_return", opts, Map::new())?.opt_obj() else { break };
        g.to_zone(c, "hand", false, None)?;
    }
    g.add_effect(map! { kind = "hide_free", pid = pid, dur = "turn" });
    Ok(Value::None)
}

// Frigid Jewel — "When you draw your second card each turn, give a friendly unit +2 might this turn." (draw event n=
// the number of cards that player drew this turn)
fn jewel(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "draw" && pid_is_ctrl(g, info, o) && info.g("n").int() == 2 {
        let res = f("_jewel.res", |g, a| {
            let it = a[0].item();
            if g.legal(it, 0)?.is_some() {
                let u = g.legal(it, 0)?.unwrap();
                g.mod_(u, 2, turn(), None)?;
            }
            Ok(Value::None)
        });
        let opts = f("_jewel.opts", |g, a| {
            let c = itc(g, a[0].item());
            let us = g.units(Some(c), None);
            let mut keys = vec![];
            for &u in &us {
                keys.push((!g.at_bf(u), -value(g, u)?));
            }
            Ok(objs(&sort_keyed(us, keys)))
        });
        trig(g, o, "Frigid Jewel", res, Map::new(), false, trig_target(opts, p_friend_v(), "buff_target", false, true), Value::None);
    }
    Ok(Value::None)
}

// Wraith of Echoes — "The first time a friendly unit dies each turn, draw 1." (Game.hist['died'] counts the deaths
// made before it entered the board; units dying together: the first one)
fn wraith(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev != "die" {
        return Ok(Value::None);
    }
    let d = info.g("info");
    let d = d.map();
    let ctrl = g.o(o).ctrl;
    if d.g("ctrl").int() as usize != ctrl || specs()[d.g("spec").str()].type_ != "Unit" || is_obj(&d.g("obj"), o) {
        return Ok(Value::None);
    }
    let n = g
        .hist
        .died
        .iter()
        .filter(|x| x.g("ctrl").int() as usize == ctrl && specs()[x.g("spec").str()].type_ == "Unit")
        .count();
    if n == 1 {
        trig0(g, o, "Wraith of Echoes", draw_fn(1));
    }
    Ok(Value::None)
}

// Ezreal, Dashing — "When I attack or defend, deal damage equal to my Might to an enemy unit here. I don't deal
// combat damage. 1 mind rune: [Action] — Move me to your base."
fn dashing_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let (me, u) = (src_of(g, it), g.legal(it, 0)?);
    if let (Some(me), Some(u)) = (me, u) {
        let m = g.might(me)?.max(0);
        let c = itc(g, it);
        g.deal(Some(u), m, "ability", Some(c), None)?;
    }
    Ok(Value::None)
}

fn dashing_back(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    if let Some(me) = src_of(g, it) {
        if g.o(me).loc != Some(Loc::Base) {
            let c = itc(g, it);
            g.move_(&[me], Loc::Base, Some(c), false)?;
        }
    }
    Ok(Value::None)
}

// Otterpus — "If a player would score 1 point from conquering or holding during their first or second turn, they draw
// 1 instead." (a replacement of the point, rule 370)
fn otterpus(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, why) = (a[1].int() as usize, a[3].str());
    if matches!(why, "conquer" | "hold") && g.tp == pid && g.p[pid].turns <= 2 {
        g.log(|| format!("  Otterpus: P{pid} draws instead of scoring"));
        g.draw(pid, 1)?;
        return Ok(true.into());
    }
    Ok(false.into())
}

// Blue Sentinel — "[Shield 2] Your hold effects for holding here trigger an additional time. When I hold, [Add] 1
// rune of any type at the start of your next Main Phase."
fn sentinel_main(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (eid, info) = (a[0].int() as u64, a[1].map());
    let eff = g.eff(eid).cloned().unwrap_or_default();
    if info.g("pid") != eff.g("pid") {
        return Ok(Value::None);
    }
    g.remove_effect(eid);
    let pid = eff.g("pid").int() as usize;
    g.p[pid].pool_p.add("A", 1);
    g.log(|| format!("  Blue Sentinel: P{pid} adds 1 rune of any type"));
    Ok(Value::None)
}

fn sentinel_event(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "hold" && in_list(&info.g("units"), o) {
        let res = f("_sentinel.res", |g, a| {
            let c = itc(g, a[0].item());
            g.add_effect(mk(vec![("on", "main_start".into()), ("fn", f("_sentinel_main", sentinel_main)), ("pid", c.into())]));
            Ok(Value::None)
        });
        trig0(g, o, "Blue Sentinel", res);
    }
    Ok(Value::None)
}

// Zilean, Time Mage — "Once each turn, if you would play a token unit while I'm at a battlefield, you may play that
// token and an additional copy of it instead." (make_token)
fn zilean(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (src, pid, name) = (a[0].obj(), a[1].int() as usize, a[2].clone());
    if pid != g.o(src).ctrl || !g.at_bf(src) {
        return Ok(false.into());
    }
    let (uid, oid) = (g.o(src).uid as i64, g.o(src).oid);
    if g.effects.iter().any(|e| {
        e.d.g("kind").opt_str() == Some("zilean_used") && e.d.g("uid").int() == uid && e.d.g("oid").int() == oid
    }) {
        return Ok(false.into());
    }
    if !g.ask(pid, "zilean_copy", vec![true.into(), false.into()], map! { card = name })?.truthy() {
        return Ok(false.into());
    }
    g.add_effect(map! { kind = "zilean_used", uid = uid, oid = oid, dur = "turn" });
    Ok(true.into())
}

// Prize of Progress — "When you use an activated ability of a gear, give me +1 might this turn." ([Add] abilities
// used while paying included: event 'activated')
fn prize(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
    if ev == "activated" && pid_is_ctrl(g, info, o) {
        if let Some(x) = info.g("obj").opt_obj() {
            if ty(g, x) == "Gear" {
                trig0(g, o, "Prize of Progress", src_mod(1));
            }
        }
    }
    Ok(Value::None)
}

// Rebuttal — "[Reaction] Choose a spell with Energy cost no more than 4 energy. You may pay 1 rune of any type. If you
// do, gain control of it and you may make new choices for it. Otherwise, counter it."
fn rebuttal(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let iid = g.it(it).data.g("item");
    let tgt = iid.opt_int().and_then(|i| item_by_id(g, i));
    let Some(tgt) = tgt.filter(|&t| g.it(t).kind == "spell") else { return Ok(Value::None) };
    let c = itc(g, it);
    if g.can_pay(c, 0, &[any_req()], &Value::None)?
        && g.ask(c, "rebuttal_pay", vec![true.into(), false.into()], map! { item = tgt })?.truthy()
        && g.pay(c, 0, &[any_req()], &Value::None)?
    {
        gain_control_item(g, tgt, c);
        remake_choices(g, tgt, c)?;
    } else {
        g.counter(tgt)?;
    }
    Ok(Value::None)
}

fn rebuttal_choices(g: &mut Game, a: &[Value]) -> Res<Value> {
    let pid = a[0].int() as usize;
    let its: Vec<ItemId> = g
        .chain
        .iter()
        .rev()
        .copied()
        .filter(|&i| g.it(i).kind == "spell" && g.it(i).card.is_some_and(|c| se(g, c) <= 4))
        .collect();
    let keys: Vec<bool> = its.iter().map(|&i| g.it(i).ctrl == pid).collect();
    Ok(maps(sort_keyed(its, keys).into_iter().map(|i| map! { item = g.it(i).id }).collect()))
}

// Kai'Sa, Evolutionary — "[Ganking] When I conquer, you may play a spell from your trash with Energy cost less than
// your points without paying its Energy cost. Then recycle it." ("Then recycle it": wherever it leaves the chain,
// resolved or countered: item.data['after'] = 'recycle')
fn kaisa_res(g: &mut Game, a: &[Value]) -> Res<Value> {
    let it = a[0].item();
    let pid = itc(g, it);
    let pts = g.p[pid].points;
    let cs: Vec<O> = g.p[pid].trash.iter().copied().filter(|&c| ty(g, c) == "Spell" && se(g, c) < pts).collect();
    let keys: Vec<(i64, u32)> = cs.iter().map(|&c| (-se(g, c), g.o(c).uid)).collect();
    let mut cs = sort_keyed(cs, keys);
    while !cs.is_empty() {
        let mut opts: Vec<Value> = cs.iter().map(|&c| Value::Obj(c)).collect();
        opts.push(Value::None);
        let Some(c) = g.ask(pid, "kaisa_spell", opts, Map::new())?.opt_obj() else { return Ok(Value::None) };
        if !play_spell_limited(g, pid, c, "trash", &map! { ignore_energy = true }, Some(&map! { after = "recycle" }))?.is_none() {
            return Ok(Value::None);
        }
        let i = cs.iter().position(|&x| x == c).unwrap();
        cs.remove(i);
    }
    Ok(Value::None)
}

// Heimerdinger, Inventor — "I have all exhaust abilities of all friendly legends, units, and gear." (activated
// abilities with [E] in their cost, and [Add] abilities with [E]: Gold's "Kill this, [E]: [Add] [A]" included; the
// ability's "me"/"this" is Heimerdinger, rule 376)
/// Identité d'un dict de capacité (`id(a)` de Python) : la capacité d'un `Impl` est (nom, liste, indice) ; une
/// capacité accordée à un objet (`x.abs`) n'a pas d'identité stable en Rust, on la compare par égalité.
#[derive(PartialEq)]
enum AbId {
    Impl(String, &'static str, usize),
    Abs(Map),
    Gold,
}

fn heimer_others(g: &Game, src: O) -> Vec<(O, Option<&'static Impl>)> {
    let pid = g.o(src).ctrl;
    let mut out = vec![(g.p[pid].legend, g.impl_n(&g.p[pid].legend_name))];
    for &x in &g.board {
        if x != src && g.o(x).ctrl == pid && matches!(ty(g, x), "Unit" | "Gear") && g.o(x).attached_to.is_none() {
            out.push((x, g.impl_o(x)));
        }
    }
    out
}

fn heimer_abilities(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (src, obj) = (a[0].obj(), a[1].obj());
    if obj != src {
        return Ok(Value::List(vec![]));
    }
    let mut out: Vec<Value> = vec![];
    let mut seen: Vec<AbId> = vec![];
    for (x, im) in heimer_others(g, src) {
        let mut abs_: Vec<(AbId, Map)> = vec![];
        if let Some(im) = im {
            abs_.extend(im.abilities.iter().enumerate().map(|(i, a)| (AbId::Impl(im.name.clone(), "abilities", i), a.clone())));
        }
        abs_.extend(g.o(x).abs.iter().map(|(a, _)| (AbId::Abs(a.clone()), a.clone())));
        if g.o(x).zone == Some(Zone::Board) && !g.o(x).attached.is_empty() {
            for n in g.copied_texts(x) {
                if let Some(ic) = g.impl_n(&n) {
                    abs_.extend(ic.abilities.iter().enumerate().map(|(i, a)| (AbId::Impl(ic.name.clone(), "abilities", i), a.clone())));
                }
            }
        }
        for (id, a) in abs_ {
            if a.g("exhaust").truthy() && !seen.contains(&id) {
                seen.push(id);
                out.push(Value::Map(a));
            }
        }
    }
    Ok(Value::List(out))
}

/// `_HEIMER_GOLD`
fn heimer_gold() -> Map {
    map! { p = Value::List(vec!["A".into()]), exhaust = true, kill = true }
}

fn heimer_adds(g: &mut Game, a: &[Value]) -> Res<Value> {
    let src = a[0].obj();
    let mut out: Vec<Value> = vec![];
    let mut seen: Vec<AbId> = vec![];
    for (x, im) in heimer_others(g, src) {
        if matches!(g.cname(x), "Gold" | "Gold // Buff") && !seen.contains(&AbId::Gold) {
            seen.push(AbId::Gold);
            out.push(Value::Map(heimer_gold()));
        }
        if let Some(im) = im {
            for (i, ad) in im.add.iter().enumerate() {
                let id = AbId::Impl(im.name.clone(), "add", i);
                if ad.get("exhaust").map(|v| v.truthy()).unwrap_or(true) && !seen.contains(&id) {
                    seen.push(id);
                    out.push(Value::Map(ad.clone()));
                }
            }
        }
    }
    Ok(Value::List(out))
}

// Malzahar, Fanatic — "Kill a friendly unit or gear, [E]: [Action] — [Add] 2 runes of any type."
fn malz_victims(g: &mut Game, a: &[Value]) -> Res<Value> {
    let (pid, o) = (a[0].int() as usize, a[1].obj());
    let xs: Vec<O> = g
        .board
        .iter()
        .copied()
        .filter(|&x| g.o(x).ctrl == pid && x != o && matches!(ty(g, x), "Unit" | "Gear"))
        .collect();
    let keys: Vec<(i64, u32)> = xs.iter().map(|&x| (se(g, x) + 2 * sp(g, x), g.o(x).uid)).collect();
    Ok(objs(&sort_keyed(xs, keys)))
}

// @@SUITE@@
