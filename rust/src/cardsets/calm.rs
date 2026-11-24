//! Paquet calm : les cartes Calm de batches/calm.txt (unités, sorts, équipements). Traduction ligne à ligne de
//! `riftbound/engine/cardsets/calm.py` (voir GUIDE.md et `docs/memoire/portage-rust.md`).
//!
//! Conventions de ce module (celles du Python) :
//! - les capacités déclenchées ne gardent pas d'objet dans leurs fermetures quand le Python ne le fait pas : elles
//!   rangent uid / oid dans les données du déclenchement ou dans l'entrée de `g.effects` ;
//! - les dégâts infligés par une unité à cause d'un sort (« It deals damage equal to its Might ... ») ont l'unité
//!   pour source, pas le sort (règle 417.6.b.3) : type "unit" (pas de dégâts bonus de sort ou de capacité) ;
//! - les jetons joués sans emplacement (« Play a 2 might Sand Soldier unit token ») sont joués comme des unités, à
//!   la base ou sur un battlefield que leur contrôleur contrôle (règles 184.2, 439.2.c) : l'emplacement est demandé
//!   ("play_location").
//! - `g.stats[(...)]` (clés tuple de Python) : clé texte écrite comme le `repr` du tuple Python.
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::actions::{attach, deflect_reqs, full_choices, pay_ctx, play_card, total_cost};
use crate::cards::{Impl, Registry};
use crate::cards_helpers::{
    ability, add_ability, all_units, cap, enemies, flow_cost, friends, gain_control_item, hb_ok, impl_empower,
    impl_equip, item_by_id, make_token, p_enemy_bf_v, p_enemy_v, p_friend_bf_v, p_friend_v, p_unit, p_unit_bf_v,
    p_unit_v, pay_deflect, remake_choices, repeat_cost, repeatable, tg_choices, token_locations, trig_target,
    unit_play_choices, value, when_empowered, AbilityArgs, Cost,
};
use crate::game::{Desig, Game, ItemId, Res, Zone, O, R};
use crate::game_costs::{any_req, reqs_to_value, Req};
use crate::map;
use crate::spec::spec;
use crate::value::{is_bf, Loc, Map, Value};

const MODULE: &str = "cardsets.calm";
const PET_TAGS: [&str; 4] = ["Bird", "Cat", "Dog", "Poro"];

// ====================================================================== petites aides privées (traduction)
fn none() -> Res<Value> {
    Ok(Value::None)
}

fn pid_of(v: &Value) -> usize {
    v.int() as usize
}

fn vo(v: &[O]) -> Vec<Value> {
    v.iter().map(|&o| Value::Obj(o)).collect()
}

fn maps(v: Vec<Map>) -> Value {
    Value::List(v.into_iter().map(Value::Map).collect())
}

/// `(uid, ...)` : tuple d'uids d'un choix (`tg`).
fn tg_of(g: &Game, us: &[O]) -> Value {
    Value::Tuple(us.iter().map(|&u| Value::from(g.o(u).uid)).collect())
}

fn uid(g: &Game, o: O) -> u32 {
    g.o(o).uid
}

fn oid(g: &Game, o: O) -> i64 {
    g.o(o).oid
}

fn ctrl(g: &Game, it: ItemId) -> usize {
    g.it(it).ctrl
}

fn data(g: &Game, it: ItemId, k: &str) -> Value {
    g.it(it).data.g(k)
}

fn is_unit(g: &Game, o: O) -> bool {
    g.spec(o).type_ == "Unit"
}

fn ep(g: &Game, o: O) -> i64 {
    let s = g.spec(o);
    (s.e + 2 * s.p) as i64
}

fn tg_uid(c: &Map, i: usize) -> u32 {
    c.g("tg").list()[i].int() as u32
}

fn same(v: &Value, o: O) -> bool {
    *v == Value::Obj(o)
}

/// `g.units(pid, loc)` pour un emplacement Python qui peut valoir None (aucune unité n'y est).
fn units_at(g: &Game, pid: Option<usize>, loc: Option<Loc>) -> Vec<O> {
    match loc {
        Some(l) => g.units(pid, Some(l)),
        None => vec![],
    }
}

/// Tri stable par clé (`sorted(key=...)`), clés calculées une fois par élément, dans l'ordre.
fn ksort<T, K: PartialOrd>(v: Vec<T>, mut key: impl FnMut(&T) -> Res<K>) -> Res<Vec<T>> {
    let mut ks = Vec::with_capacity(v.len());
    for x in v {
        let k = key(&x)?;
        ks.push((k, x));
    }
    ks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    Ok(ks.into_iter().map(|p| p.1).collect())
}

/// `sorted(key=..., reverse=True)` (stable : les égaux gardent leur ordre).
fn ksort_rev<T, K: PartialOrd>(v: Vec<T>, mut key: impl FnMut(&T) -> Res<K>) -> Res<Vec<T>> {
    let mut ks = Vec::with_capacity(v.len());
    for x in v {
        let k = key(&x)?;
        ks.push((k, x));
    }
    ks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    Ok(ks.into_iter().map(|p| p.1).collect())
}

/// `itertools.combinations(seq, k)` (ordre lexicographique des indices).
fn combinations<T: Clone>(seq: &[T], k: usize) -> Vec<Vec<T>> {
    let n = seq.len();
    let mut out = vec![];
    if k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.iter().map(|&i| seq[i].clone()).collect());
        let mut i = k;
        let mut found = false;
        while i > 0 {
            i -= 1;
            if idx[i] != i + n - k {
                found = true;
                break;
            }
        }
        if !found {
            return out;
        }
        idx[i] += 1;
        for j in i + 1..k {
            idx[j] = idx[j - 1] + 1;
        }
    }
}

fn calm_req() -> Req {
    let mut r = Req::new();
    r.insert("Calm".to_string());
    r
}

/// `(e, reqs)` rendu par une fonction de coût.
fn cost_tuple(e: i64, reqs: &[Req]) -> Value {
    Value::Tuple(vec![e.into(), reqs_to_value(reqs)])
}

fn pair(a: i64, b: i64) -> Value {
    Value::Tuple(vec![a.into(), b.into()])
}

fn trig(
    g: &mut Game,
    pid: usize,
    name: &str,
    f: Value,
    data: Map,
    src: Option<u32>,
    may: bool,
    choose: Value,
    cost: Value,
) {
    g.queue_trigger(pid, name, f, data, src, may, choose, cost);
}

fn reg(r: &mut Registry, mut im: Impl) {
    im.module = Some(MODULE);
    r.card(im);
}

// ====================================================================== helpers (calm.py)
/// `_alive(g, uid, oid)` : le permanent uid s'il est toujours la même incarnation (oid), sinon None.
fn alive(g: &Game, u: Option<u32>, o: i64) -> Option<O> {
    let x = u.and_then(|u| g.obj(u))?;
    if g.o(x).oid == o {
        Some(x)
    } else {
        None
    }
}

/// `_alive(g_, it.src, it.data["oid"])`
fn src_alive(g: &Game, it: ItemId) -> Option<O> {
    alive(g, g.it(it).src, data(g, it, "oid").int())
}

/// `_pet_tags(g, pid)` : tags parmi Bird, Cat, Dog et Poro chez les unités de pid.
fn pet_tags(g: &mut Game, pid: usize) -> Res<BTreeSet<&'static str>> {
    let mut found = BTreeSet::new();
    for u in g.units(Some(pid), None) {
        let t = g.tags(u)?;
        for p in PET_TAGS {
            if t.contains(p) {
                found.insert(p);
            }
        }
    }
    Ok(found)
}

/// `_near_victory(g, pid)` : « If an opponent's score is within 3 points of the Victory Score ».
fn near_victory(g: &Game, pid: usize) -> bool {
    (0..2).filter(|&q| q != pid).any(|q| g.victory - g.p[q].points <= 3)
}

/// `_play_token(g, name, pid, loc, ready)` : loc None = un emplacement où le jeton peut être joué (demandé).
fn play_token(g: &mut Game, name: &str, pid: usize, loc: Option<Loc>, ready: bool) -> Res<Option<O>> {
    let loc = match loc {
        Some(l) => l,
        None => {
            let ls = token_locations(g, pid, name, None)?;
            if ls.is_empty() {
                return Ok(None);
            }
            g.ask(pid, "play_location", ls.into_iter().map(Value::Loc).collect(), Map::new())?.loc()
        }
    };
    make_token(g, name, pid, loc, ready)
}

/// `_draw_card(g, pid, c)` : « Draw it ».
fn draw_card(g: &mut Game, pid: usize, c: O) -> Res<()> {
    g.draw_card(pid, c)?;
    Ok(())
}

fn deck_remove(g: &mut Game, pid: usize, c: O) {
    if let Some(i) = g.p[pid].deck.iter().position(|&x| x == c) {
        g.p[pid].deck.remove(i);
    }
}

/// `_look_pick(g, pid, n, ok, kind)` : regarder les n cartes du dessus, peut révéler une carte ok et la piocher,
/// recycler le reste. Renvoie la carte piochée ou None.
fn look_pick(g: &mut Game, pid: usize, n: usize, ok: fn(&Game, O) -> bool, kind: &str) -> Res<Option<O>> {
    let top0: Vec<O> = g.p[pid].deck.iter().take(n).copied().collect();
    let top = g.look(pid, &top0, None)?;
    if top.is_empty() {
        return Ok(None);
    }
    let cands: Vec<O> = top.iter().copied().filter(|&c| ok(g, c)).collect();
    let cands = ksort(cands, |&c| Ok(-ep(g, c)))?;
    let mut opts = vo(&cands);
    opts.push(Value::None);
    let pick = g.ask(pid, kind, opts, Map::new())?.opt_obj();
    if let Some(p) = pick {
        let cn = g.cname(p).to_string();
        g.log(|| format!("  P{pid} reveals and draws {cn}"));
        draw_card(g, pid, p)?;
    }
    let rest: Vec<O> = top.iter().copied().filter(|&c| Some(c) != pick).collect();
    for &c in &rest {
        deck_remove(g, pid, c);
    }
    g.recycle_cards(pid, &rest)?;
    Ok(pick)
}

fn is_unit_card(g: &Game, c: O) -> bool {
    g.spec(c).type_ == "Unit"
}

fn is_gear_card(g: &Game, c: O) -> bool {
    g.is_gear(c)
}

/// `_units_first(g, pid, hb)` : toutes les unités, alliées d'abord (ordre de l'IA pour les effets positifs).
fn units_first(g: &mut Game, pid: usize, hb: Option<Loc>) -> Res<Vec<O>> {
    let mut v = friends(g, pid, false, hb)?;
    v.extend(enemies(g, pid, false, hb, true)?);
    Ok(v)
}

/// `_mod_trigger(name, n)` : « When you play me, give a unit +n might this turn. »
fn mod_trigger(name: &'static str, n: i64) -> Value {
    Value::func("_mod_trigger.on_play", move |g, a| {
        let o = a[0].obj();
        let res = Value::func("_mod_trigger.res", move |g, a| {
            let it = a[0].item();
            if let Some(u) = g.legal(it, 0)? {
                g.mod_(u, n, "turn".into(), None)?;
            }
            none()
        });
        let opts = Value::func("_mod_trigger.opts", |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            Ok(vo(&units_first(g, c, None)?).into())
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        trig(g, c, name, res, Map::new(), Some(u), false, trig_target(opts, p_unit_v(), "friendly_target", false, true), Value::None);
        none()
    })
}

/// `_hold(o, info)`
fn hold(g: &Game, o: O, info: &Map) -> bool {
    info.g("pid").opt_int() == Some(g.o(o).ctrl as i64) && info.g("units").list().contains(&Value::Obj(o))
}

/// `_spell_items(g, pid)` : sorts de la chaîne qui peuvent être contrés, ennemis d'abord (plus récent d'abord).
fn spell_items(g: &mut Game, pid: usize) -> Res<Vec<ItemId>> {
    let mut sp = vec![];
    for i in g.chain.clone().into_iter().rev() {
        if g.it(i).kind == "spell" && g.counterable(i)? {
            sp.push(i);
        }
    }
    let mut out: Vec<ItemId> = sp.iter().copied().filter(|&i| g.it(i).ctrl != pid).collect();
    out.extend(sp.iter().copied().filter(|&i| g.it(i).ctrl == pid));
    Ok(out)
}

fn item_of(g: &Game, v: &Value) -> Option<ItemId> {
    v.opt_int().and_then(|iid| item_by_id(g, iid))
}

/// `_counter_spell(g, it)`
fn counter_spell() -> Value {
    Value::func("_counter_spell", |g, a| {
        let it = a[0].item();
        let tgt = item_of(g, &data(g, it, "item"));
        if let Some(t) = tgt {
            if g.it(t).kind == "spell" {
                g.counter(t)?;
            }
        }
        none()
    })
}

/// `_ready_runes(g, pid, n)`
fn ready_runes(g: &mut Game, pid: usize, n: usize) -> Res<()> {
    let mut ex: Vec<R> = g.p[pid].runes.iter().copied().filter(|&r| g.r(r).exhausted).collect();
    for _ in 0..n.min(ex.len()) {
        let r = if ex.len() > 1 {
            g.ask(pid, "ready_rune", ex.iter().map(|&r| Value::Rune(r)).collect(), Map::new())?.rune()
        } else {
            ex[0]
        };
        if let Some(i) = ex.iter().position(|&x| x == r) {
            ex.remove(i);
        }
        g.r_mut(r).exhausted = false;
    }
    Ok(())
}

/// `_compositions(total, k, cap)` : façons d'écrire total en k entiers positifs (ordonnées), au plus cap.
fn compositions(total: i64, k: usize, capn: usize) -> Vec<Vec<i64>> {
    fn rec(out: &mut Vec<Vec<i64>>, capn: usize, left: i64, parts: usize, acc: Vec<i64>) {
        if out.len() >= capn {
            return;
        }
        if parts == 1 {
            if left >= 1 {
                let mut v = acc;
                v.push(left);
                out.push(v);
            }
            return;
        }
        for x in 1..(left - parts as i64 + 2) {
            let mut a = acc.clone();
            a.push(x);
            rec(out, capn, left - x, parts - 1, a);
        }
    }
    let mut out = vec![];
    if k >= 1 && total >= k as i64 {
        rec(&mut out, capn, total, k, vec![]);
    }
    out
}

/// `_split_damage(g, pid, amount, targets, source_kind, by_pid)` (règle 355.14) : répartir amount entre les cibles,
/// chacune au moins 1 ; s'il y a plus de cibles que de dégâts, le contrôleur choisit celles qui cessent d'être
/// des cibles (il en garde exactement amount). Renvoie les cibles.
fn split_damage(g: &mut Game, pid: usize, amount: i64, targets: Vec<O>, kind: &str, by_pid: usize) -> Res<Vec<O>> {
    if amount <= 0 || targets.is_empty() {
        return Ok(vec![]);
    }
    let mut targets = targets;
    if targets.len() as i64 > amount {
        let keeps = cap(g, combinations(&targets, amount as usize), 200, Some(pid));
        let keeps = ksort(keeps, |ts| {
            let mut s = 0.0;
            for &t in ts {
                if g.might(t)? - g.o(t).damage <= 1 {
                    s += value(g, t)?;
                }
            }
            Ok(-s)
        })?;
        let opts: Vec<Value> = keeps.iter().map(|ts| Value::Tuple(vo(ts))).collect();
        targets = g.ask(pid, "split_keep", opts, Map::new())?.list().iter().map(|v| v.obj()).collect();
    }
    let splits = compositions(amount, targets.len(), 400);
    let splits = ksort(splits, |sp| {
        let mut s = 0.0;
        for (&t, &n) in targets.iter().zip(sp.iter()) {
            if g.o(t).damage + n >= g.might(t)? {
                s += value(g, t)?;
            }
        }
        Ok(-s)
    })?;
    let opts: Vec<Value> = splits.iter().map(|sp| Value::Tuple(sp.iter().map(|&n| Value::Int(n)).collect())).collect();
    let sp: Vec<i64> = g.ask(pid, "split_damage", opts, Map::new())?.list().iter().map(|v| v.int()).collect();
    for (&t, &n) in targets.iter().zip(sp.iter()) {
        g.deal(Some(t), n, kind, Some(by_pid), None)?;
    }
    Ok(targets)
}

/// `_kill_lethal(g, units, by_pid)` : les unités avec des dégâts mortels meurent maintenant.
fn kill_lethal(g: &mut Game, units: &[O], by_pid: usize) -> Res<Vec<O>> {
    let mut dead = vec![];
    for &u in units {
        if g.on_board(u) && g.lethal(u)? {
            dead.push(u);
        }
    }
    if dead.is_empty() {
        Ok(vec![])
    } else {
        g.kill(&dead, Some(by_pid), false)
    }
}

/// options d'un trig_target : « une unité ennemie ici » (Daisy!, Riven, Yasuo).
fn enemies_here_opts(here: Option<Loc>) -> Value {
    Value::func("enemies_here", move |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        let es: Vec<O> = enemies(g, c, false, None, true)?.into_iter().filter(|&u| g.o(u).loc == here).collect();
        Ok(vo(&es).into())
    })
}

/// prédicat `lambda g_, it, u: u.ctrl != it.ctrl and u.loc == here`
fn enemy_here_pred(here: Option<Loc>) -> Value {
    Value::func("enemy_here_pred", move |g, a| {
        let (it, u) = (a[0].item(), a[1].obj());
        Ok((g.o(u).ctrl != ctrl(g, it) && g.o(u).loc == here).into())
    })
}

/// `(g.legal(it, 0) is not None and g.mod(g.legal(it, 0), n))`
fn mod_legal0(g: &mut Game, it: ItemId, n: i64) -> Res<()> {
    if g.legal(it, 0)?.is_some() {
        let u = g.legal(it, 0)?;
        if let Some(u) = u {
            g.mod_(u, n, "turn".into(), None)?;
        }
    }
    Ok(())
}

/// `lambda g_, it: g_.stun(g_.legal(it, 0), it.ctrl)`
fn stun_legal0(name: &'static str) -> Value {
    Value::func(name, |g, a| {
        let it = a[0].item();
        let u = g.legal(it, 0)?;
        let c = ctrl(g, it);
        g.stun(u, Some(c))?;
        none()
    })
}

/// `lambda g_, it: g_.draw(it.ctrl, 1)`
fn draw1(name: &'static str) -> Value {
    Value::func(name, |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        g.draw(c, 1)?;
        none()
    })
}

/// `lambda g_, it: g_.gain_point(it.ctrl, name)`
fn point(name: &'static str) -> Value {
    Value::func("gain_point", move |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        g.gain_point(c, name)?;
        none()
    })
}

/// `sorted(friends(g, pid), key=lambda u: (u.buff > 0, -value(g, u)))`
fn friends_unbuffed_first(g: &mut Game, pid: usize) -> Res<Vec<O>> {
    let fr = friends(g, pid, false, None)?;
    ksort(fr, |&u| Ok((g.o(u).buff > 0, -value(g, u)?)))
}

// ====================================================================== units
/// Affectionate Poro — "When a combat that I was in ends, if I haven't been dealt damage this turn, draw 1."
fn aff_poro() -> Value {
    Value::func("_aff_poro", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        let k = format!("('calm_dmg', {}, {}, {})", uid(g, o), oid(g, o), g.turn_no);
        if ev == "damaged" && same(&info.g("obj"), o) {
            g.stats.add(&k, 1);
        } else if ev == "combat_end" && info.g("members").list().contains(&Value::Obj(o)) && g.stats.get(&k) == 0 {
            let res = Value::func("_aff_poro.res", |g, a| {
                let it = a[0].item();
                let k = format!(
                    "('calm_dmg', {}, {}, {})",
                    g.it(it).src.map(|s| s as i64).unwrap_or(-1),
                    data(g, it, "oid").int(),
                    g.turn_no
                );
                if g.stats.get(&k) == 0 {
                    let c = ctrl(g, it);
                    g.draw(c, 1)?;
                }
                none()
            });
            let (c, u, od) = (g.o(o).ctrl, uid(g, o), oid(g, o));
            trig(g, c, "Affectionate Poro", res, map! { oid = od }, Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Ahri, Alluring — "When I hold, you score 1 point."
fn ahri() -> Value {
    Value::func("_ahri", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "hold" && hold(g, o, info) {
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            trig(g, c, "Ahri, Alluring", point("Ahri, Alluring"), Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Allay, Eager Admirer — aura : "While I'm at a battlefield, your other units here have [Deflect]."
fn allay_aura() -> Value {
    Value::func("allay_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        if g.on_board(src)
            && is_bf(g.o(src).loc)
            && o != src
            && g.o(o).ctrl == g.o(src).ctrl
            && g.o(o).loc == g.o(src).loc
            && is_unit(g, o)
        {
            Ok(map! { Deflect = 1 }.into())
        } else {
            none()
        }
    })
}

/// Aphelios, Exalted — "When you attach an Equipment to me, choose one that hasn't been chosen this turn — Ready 2
/// runes. / Channel 1 rune exhausted. / Buff a friendly unit."
fn aphelios() -> Value {
    Value::func("_aphelios", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev != "attached" || !same(&info.g("unit"), o) {
            return none();
        }
        let gear = info.g("gear").obj();
        if !g.spec(gear).tags.contains("Equipment") || g.o(gear).ctrl != g.o(o).ctrl {
            return none();
        }
        let od = oid(g, o);
        let choose = Value::func("_aphelios.choose", move |g, a| {
            let it = a[0].item();
            let src = g.it(it).src.map(|s| s as i64).unwrap_or(-1);
            let key = |g: &Game, m: &str| format!("('calm_aph', {src}, {od}, {}, '{m}')", g.turn_no);
            let used: Vec<&str> = ["ready", "channel", "buff"].into_iter().filter(|m| g.stats.get(&key(g, m)) != 0).collect();
            let mut modes: Vec<&str> = ["ready", "channel", "buff"].into_iter().filter(|m| !used.contains(m)).collect();
            let c = ctrl(g, it);
            if modes.contains(&"ready") && !g.p[c].runes.iter().any(|&r| g.r(r).exhausted) {
                modes.retain(|&m| m != "ready");
                modes.push("ready"); // ordre de l'IA seulement : le mode reste légal
            }
            if modes.is_empty() {
                return Ok(false.into());
            }
            let m = g.ask(c, "aphelios_mode", modes.iter().map(|&m| Value::from(m)).collect(), Map::new())?;
            let ms = m.str().to_string();
            let k = key(g, &ms);
            g.stats.add(&k, 1);
            g.it_mut(it).data.set("mode", m);
            if ms == "buff" {
                let us = friends_unbuffed_first(g, c)?;
                if !us.is_empty() {
                    let x = g.ask_obj(c, "friendly_target", &us, map! { item = it })?.unwrap();
                    g.add_target(it, x, p_friend_v());
                }
            }
            Ok(true.into())
        });
        let res = Value::func("_aphelios.res", |g, a| {
            let it = a[0].item();
            let m = data(g, it, "mode");
            let c = ctrl(g, it);
            match m.opt_str() {
                Some("ready") => ready_runes(g, c, 2)?,
                Some("channel") => {
                    g.channel(c, 1, true)?;
                }
                Some("buff") => {
                    if let Some(u) = g.legal(it, 0)? {
                        g.buff(Some(u), None)?;
                    }
                }
                _ => {}
            }
            none()
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        trig(g, c, "Aphelios, Exalted", res, Map::new(), Some(u), false, choose, Value::None);
        none()
    })
}

/// Apprentice Smith — "When I move, reveal the top card of your Main Deck. If it's a gear, draw it. Otherwise,
/// recycle it."
fn smith_res() -> Value {
    Value::func("_smith_res", |g, a| {
        let it = a[0].item();
        let c0 = ctrl(g, it);
        let top = g.reveal(c0, 1, None, true, None)?;
        if top.is_empty() {
            return none();
        }
        let c = top[0];
        if is_gear_card(g, c) {
            draw_card(g, c0, c)?;
        } else {
            deck_remove(g, c0, c);
            g.recycle_cards(c0, &[c])?;
        }
        none()
    })
}

/// Azir, Ascendant — "[Calm]: [Action] — Choose a unit you control. Move me to its location and it to my original
/// location. If it's equipped, you may attach one of its Equipment to me. Use only once per turn."
fn azir_res() -> Value {
    Value::func("_azir_res", |g, a| {
        let it = a[0].item();
        let az = g.it(it).src.and_then(|s| g.obj(s));
        let u = g.legal(it, 0)?;
        let (Some(az), Some(u)) = (az, u) else { return none() };
        let (a_loc, u_loc) = (g.o(az).loc, g.o(u).loc);
        let c = ctrl(g, it);
        g.move_(&[az], u_loc.unwrap(), Some(c), false)?;
        g.move_(&[u], a_loc.unwrap(), Some(c), false)?;
        let eq: Vec<O> = g
            .o(u)
            .attached
            .clone()
            .into_iter()
            .filter_map(|x| g.obj(x))
            .filter(|&x| g.spec(x).tags.contains("Equipment"))
            .collect();
        if !eq.is_empty() {
            let eq = ksort(eq, |&x| Ok(-g.equip_bonus_of(g.cname(x))))?;
            let mut opts = vo(&eq);
            opts.push(Value::None);
            let x = g.ask(c, "azir_attach", opts, Map::new())?.opt_obj();
            if let Some(x) = x {
                if g.on_board(az) {
                    attach(g, x, az)?;
                }
            }
        }
        none()
    })
}

fn azir_choices() -> Value {
    Value::func("_azir_choices", |g, a| {
        let (pid, o) = (pid_of(&a[0]), a[1].obj());
        let us: Vec<O> = friends(g, pid, false, None)?
            .into_iter()
            .filter(|&u| u != o && (g.o(u).loc != g.o(o).loc || !g.o(u).attached.is_empty()))
            .collect();
        let us = ksort_rev(us, |&u| Ok((is_bf(g.o(u).loc), g.o(u).attached.len())))?;
        Ok(maps(us.iter().map(|&u| map! { tg = tg_of(g, &[u]) }).collect()))
    })
}

fn azir_key(g: &Game, o: O) -> String {
    format!("('calm_azir', {}, {}, {})", uid(g, o), oid(g, o), g.turn_no)
}

/// Caitlyn, Patrolling — "[E]: Deal damage equal to my Might to a unit at a battlefield. Use this ability only
/// while I'm at a battlefield."
fn cait_res() -> Value {
    Value::func("_cait_res", |g, a| {
        let it = a[0].item();
        let me = g.it(it).src.and_then(|s| g.obj(s));
        let u = g.legal(it, 0)?;
        let (Some(me), Some(u)) = (me, u) else { return none() }; // Might nulle (règle 359.3.e.12)
        let m = g.might(me)?;
        let c = ctrl(g, it);
        g.deal(Some(u), m, "ability", Some(c), None)?;
        none()
    })
}

/// Daisy! — "When I attack while your units have all 4 tags, [Stun] an enemy unit here."
fn daisy() -> Value {
    Value::func("_daisy", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "attack" && same(&info.g("obj"), o) {
            let c = g.o(o).ctrl;
            if pet_tags(g, c)?.len() == 4 {
                let here = g.o(o).loc;
                let u = uid(g, o);
                let ch = trig_target(enemies_here_opts(here), enemy_here_pred(here), "target", false, true);
                trig(g, c, "Daisy!", stun_legal0("_daisy.res"), Map::new(), Some(u), false, ch, Value::None);
            }
        }
        none()
    })
}

/// Eclipse Herald — "When you stun an enemy unit, ready me and give me +1 might this turn."
fn eclipse() -> Value {
    Value::func("_eclipse", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "stun" && info.g("by").opt_int() == Some(g.o(o).ctrl as i64) && g.o(info.g("obj").obj()).ctrl != g.o(o).ctrl {
            let res = Value::func("_eclipse.res", |g, a| {
                let it = a[0].item();
                if let Some(me) = src_alive(g, it) {
                    g.ready_obj(me, None, None)?;
                    g.mod_(me, 1, "turn".into(), None)?;
                }
                none()
            });
            let (c, u, od) = (g.o(o).ctrl, uid(g, o), oid(g, o));
            trig(g, c, "Eclipse Herald", res, map! { oid = od }, Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Enthusiastic Promoter — "[Backline] When I hold, [Buff] all units here."
fn promoter() -> Value {
    Value::func("_promoter", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "hold" && hold(g, o, info) {
            let bf = info.g("bf").int() as usize;
            let res = Value::func("_promoter.res", move |g, _a| {
                for u in g.units(None, Some(Loc::Bf(bf))) {
                    g.buff(Some(u), None)?;
                }
                none()
            });
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            trig(g, c, "Enthusiastic Promoter", res, Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Frisky Hunter — "When you play me, play a 1 might Bird unit token with [Deflect] here."
fn frisky() -> Value {
    Value::func("_frisky", |g, a| {
        let o = a[0].obj();
        let res = Value::func("_frisky.res", |g, a| {
            let it = a[0].item();
            let me = src_alive(g, it);
            let c = ctrl(g, it);
            let loc = match me {
                Some(m) => g.o(m).loc.unwrap(),
                None => data(g, it, "loc").loc(),
            };
            make_token(g, "Bird", c, loc, false)?;
            none()
        });
        let (c, u, od, loc) = (g.o(o).ctrl, uid(g, o), oid(g, o), g.o(o).loc);
        trig(g, c, "Frisky Hunter", res, map! { oid = od, loc = loc }, Some(u), false, Value::None, Value::None);
        none()
    })
}

/// Nasus, Ascended — "[Empowered] When I conquer, you score 1 point."
fn nasus() -> Value {
    Value::func("_nasus", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "conquer" && hold(g, o, info) && g.o(o).empowered {
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            trig(g, c, "Nasus, Ascended", point("Nasus, Ascended"), Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Guardian of the Passage — "When I hold, you may return a unit or gear from your trash to your hand."
fn passage() -> Value {
    Value::func("_passage", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev != "hold" || !hold(g, o, info) {
            return none();
        }
        let choose = Value::func("_passage.choose", |g, a| {
            let it = a[0].item();
            let c0 = ctrl(g, it);
            let cs: Vec<O> = g.p[c0]
                .trash
                .iter()
                .copied()
                .filter(|&c| matches!(g.spec(c).type_.as_str(), "Unit" | "Gear"))
                .collect();
            if cs.is_empty() {
                return Ok(false.into());
            }
            let cs = ksort(cs, |&c| Ok(-ep(g, c)))?;
            let c = g.ask_obj(c0, "return_pick", &cs, map! { item = it })?.unwrap();
            let u = uid(g, c);
            g.it_mut(it).data.set("card", u);
            Ok(true.into())
        });
        let res = Value::func("_passage.res", |g, a| {
            let it = a[0].item();
            let c0 = ctrl(g, it);
            let want = data(g, it, "card");
            let c = g.p[c0].trash.iter().copied().find(|&x| Value::from(g.o(x).uid) == want);
            if let Some(c) = c {
                g.to_zone(c, "hand", false, None)?;
            }
            none()
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        trig(g, c, "Guardian of the Passage", res, Map::new(), Some(u), true, choose, Value::None);
        none()
    })
}

/// Iascylla — "When I hold, at the start of your next Main Phase, you may move an enemy unit to this battlefield."
fn iascylla_main() -> Value {
    Value::func("_iascylla_main", |g, a| {
        let eid = a[0].int() as u64;
        let info = a[1].map();
        let eff = g.eff(eid).unwrap().clone();
        if info.g("pid") != eff.g("pid") {
            return none();
        }
        g.remove_effect(eid);
        let bf = eff.g("bf").int() as usize;
        let res = Value::func("_iascylla_main.res", move |g, a| {
            let it = a[0].item();
            if let Some(u) = g.legal(it, 0)? {
                if g.o(u).loc != Some(Loc::Bf(bf)) {
                    let c = ctrl(g, it);
                    g.move_(&[u], Loc::Bf(bf), Some(c), false)?;
                }
            }
            none()
        });
        let opts = Value::func("_iascylla_main.opts", move |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            let es: Vec<O> =
                enemies(g, c, false, None, true)?.into_iter().filter(|&u| g.o(u).loc != Some(Loc::Bf(bf))).collect();
            Ok(vo(&es).into())
        });
        let pred = Value::func("_iascylla_main.pred", |g, a| {
            let (it, u) = (a[0].item(), a[1].obj());
            Ok((g.o(u).ctrl != ctrl(g, it)).into())
        });
        let pid = pid_of(&eff.g("pid"));
        trig(g, pid, "Iascylla (Main Phase)", res, Map::new(), None, true, trig_target(opts, pred, "target", false, true), Value::None);
        none()
    })
}

fn iascylla() -> Value {
    Value::func("_iascylla", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "hold" && hold(g, o, info) {
            let (pid, bf) = (g.o(o).ctrl, info.g("bf"));
            let res = Value::func("_iascylla.res", move |g, _a| {
                let mut d = Map::new();
                d.set("on", "main_start");
                d.set("fn", iascylla_main());
                d.set("pid", pid);
                d.set("bf", bf.clone());
                g.add_effect(d);
                none()
            });
            let u = uid(g, o);
            trig(g, pid, "Iascylla", res, Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Ivern, Nurturer — "When you play me or when I hold, look at the top 3 cards of your Main Deck. You may reveal a
/// unit from among them and draw it. Recycle the rest. Then if you revealed a Bird, Cat, Dog, or Poro, do this:
/// [Buff] a friendly unit."
fn ivern_res() -> Value {
    Value::func("_ivern_res", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        let pick = look_pick(g, c, 3, is_unit_card, "ivern_pick")?;
        if let Some(p) = pick {
            if PET_TAGS.iter().any(|t| g.spec(p).tags.contains(*t)) {
                let us = friends_unbuffed_first(g, c)?;
                if !us.is_empty() {
                    let x = g.ask_obj(c, "friendly_target", &us, Map::new())?;
                    g.buff(x, None)?;
                }
            }
        }
        none()
    })
}

/// Ornn, Blacksmith — "When you play me or when I hold, look at the top 4 cards of your Main Deck. You may reveal a
/// gear from among them and draw it. Then recycle the rest."
fn ornn_res() -> Value {
    Value::func("_ornn_res", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        look_pick(g, c, 4, is_gear_card, "ornn_pick")?;
        none()
    })
}

/// `on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, name, res, src=o.uid)`
fn play_trigger(name: &'static str, res: fn() -> Value) -> Value {
    Value::func("on_play", move |g, a| {
        let o = a[0].obj();
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        trig(g, c, name, res(), Map::new(), Some(u), false, Value::None, Value::None);
        none()
    })
}

/// `on_event=lambda g, o, ev, info: ev == "hold" and _hold(o, info) and g.queue_trigger(o.ctrl, name, res, src=o.uid)`
fn hold_trigger(name: &'static str, res: fn() -> Value) -> Value {
    Value::func("on_event", move |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "hold" && hold(g, o, info) {
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            trig(g, c, name, res(), Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Janna, Savior — "[Reaction] When you play me, heal your units here, then move an enemy unit from here to its
/// base."
fn janna() -> Value {
    Value::func("_janna", |g, a| {
        let o = a[0].obj();
        let here = g.o(o).loc;
        let res = Value::func("_janna.res", move |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            for u in units_at(g, Some(c), here) {
                g.o_mut(u).damage = 0;
            }
            if let Some(u) = g.legal(it, 0)? {
                if g.o(u).loc == here {
                    g.move_(&[u], Loc::Base, Some(c), false)?;
                }
            }
            none()
        });
        let choose = Value::func("_janna.choose", move |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            let es0: Vec<O> =
                enemies(g, c, false, None, true)?.into_iter().filter(|&u| g.o(u).loc == here && is_bf(here)).collect();
            let mut es = vec![];
            for u in es0 {
                let reqs = deflect_reqs(g, c, &map! { tg = tg_of(g, &[u]) })?;
                if g.can_pay(c, 0, &reqs, &Value::None)? {
                    es.push(u);
                }
            }
            if !es.is_empty() {
                let u = g.ask_obj(c, "target", &es, map! { item = it })?.unwrap();
                if pay_deflect(g, it, u)? {
                    let pred = Value::func("_janna.pred", move |g, a| {
                        let (it2, x) = (a[0].item(), a[1].obj());
                        Ok((g.o(x).ctrl != ctrl(g, it2) && g.o(x).loc == here).into())
                    });
                    g.add_target(it, u, pred);
                }
            }
            Ok(true.into()) // le soin a lieu même sans unité ennemie
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        trig(g, c, "Janna, Savior", res, Map::new(), Some(u), false, choose, Value::None);
        none()
    })
}

thread_local! {
    /// `_LEONA_GUARD` (ensemble global du module Python)
    static LEONA_GUARD: RefCell<BTreeSet<u32>> = const { RefCell::new(BTreeSet::new()) };
    /// `_REFUGE_GUARD`
    static REFUGE_GUARD: Cell<bool> = const { Cell::new(false) };
}

/// Leona, Zealot — "Stunned enemy units here have -8 might, to a minimum of 1 might."
fn leona_aura() -> Value {
    Value::func("_leona_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        let su = uid(g, src);
        if LEONA_GUARD.with(|s| s.borrow().contains(&su))
            || !g.on_board(src)
            || !is_unit(g, o)
            || !g.o(o).stunned
            || g.o(o).ctrl == g.o(src).ctrl
            || g.o(o).loc != g.o(src).loc
        {
            return Ok(0.into());
        }
        LEONA_GUARD.with(|s| s.borrow_mut().insert(su));
        let m = g.might(o); // Might sans cet effet
        LEONA_GUARD.with(|s| s.borrow_mut().remove(&su));
        let m = m?;
        Ok(Value::Int(-(8i64.min(0i64.max(m - 1)))))
    })
}

/// Master Yi, Unstoppable — coûts de Level.
fn yi_cost() -> Value {
    Value::func("_yi_cost", |g, a| {
        let pid = pid_of(&a[0]);
        let xp = g.p[pid].xp;
        for (n, d) in [(11, (6, 3)), (6, (4, 2)), (3, (2, 1))] {
            if xp >= n {
                return Ok(pair(d.0, d.1));
            }
        }
        Ok(pair(0, 0))
    })
}

/// Monch — `_opp_stunned(g, pid)`
fn opp_stunned(g: &Game, pid: usize) -> bool {
    g.units(Some(1 - pid), None).iter().any(|&u| g.o(u).stunned)
}

/// `as_played=lambda g, pid, c: [dict(calm_paid=True), dict()]`
fn calm_paid_as_played() -> Value {
    Value::func("as_played", |_g, _a| Ok(maps(vec![map! { calm_paid = true }, Map::new()])))
}

/// `extra_cost_fn=lambda g, pid, c, ch: (0, [frozenset({"Calm"})]) if ch.get("calm_paid") else (0, [])`
fn calm_paid_extra() -> Value {
    Value::func("extra_cost_fn", |_g, a| {
        if a[2].map().g("calm_paid").truthy() {
            Ok(cost_tuple(0, &[calm_req()]))
        } else {
            Ok(cost_tuple(0, &[]))
        }
    })
}

/// Nami, Headstrong — on_play, prochain unité, tenue.
fn nami_play() -> Value {
    Value::func("_nami_play", |g, a| {
        let (o, ctx) = (a[0].obj(), a[1].map());
        if !ctx.g("calm_paid").truthy() {
            return none();
        }
        let hb = ctx.g("hidden_bf").opt_loc();
        let opts = Value::func("_nami_play.opts", move |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            Ok(vo(&enemies(g, c, false, hb, true)?).into())
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        let ch = trig_target(opts, p_enemy_v(), "target", false, true);
        trig(g, c, "Nami, Headstrong", stun_legal0("_nami_play.res"), Map::new(), Some(u), false, ch, Value::None);
        none()
    })
}

fn nami_next() -> Value {
    Value::func("_nami_next", |g, a| {
        let eid = a[0].int() as u64;
        let info = a[1].map();
        let eff = g.eff(eid).unwrap().clone();
        let card = info.g("card").obj();
        if info.g("pid") != eff.g("pid") || !is_unit(g, card) {
            return none();
        }
        g.remove_effect(eid);
        let res = Value::func("_nami_next.res", |g, a| {
            let it = a[0].item();
            let u = alive(g, Some(data(g, it, "u").int() as u32), data(g, it, "oid").int());
            if let Some(u) = u {
                g.ready_obj(u, None, None)?;
                g.buff(Some(u), None)?;
            }
            none()
        });
        let (cu, co) = (uid(g, card), oid(g, card));
        trig(g, pid_of(&eff.g("pid")), "Nami, Headstrong (next unit)", res, map! { u = cu, oid = co }, None, false, Value::None, Value::None);
        none()
    })
}

fn nami_event() -> Value {
    Value::func("_nami_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "hold" && hold(g, o, info) {
            let pid = g.o(o).ctrl;
            let res = Value::func("_nami_event.res", move |g, _a| {
                let mut d = Map::new();
                d.set("on", "played");
                d.set("fn", nami_next());
                d.set("pid", pid);
                d.set("dur", "turn");
                g.add_effect(d);
                none()
            });
            let u = uid(g, o);
            trig(g, pid, "Nami, Headstrong", res, Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Pakaa Protector — "When I move, reveal the top card of your Main Deck. If it's a unit, draw it. Otherwise, put
/// it in your trash and give me +2 might this turn."
fn pakaa_res() -> Value {
    Value::func("_pakaa_res", |g, a| {
        let it = a[0].item();
        let c0 = ctrl(g, it);
        let top = g.reveal(c0, 1, None, true, None)?;
        if top.is_empty() {
            return none();
        }
        let c = top[0];
        if is_unit_card(g, c) {
            draw_card(g, c0, c)?;
        } else {
            g.to_zone(c, "trash", false, None)?;
            if let Some(me) = src_alive(g, it) {
                g.mod_(me, 2, "turn".into(), None)?;
            }
        }
        none()
    })
}

/// Poro Herder — `_has_poro(g, pid)`
fn has_poro(g: &mut Game, pid: usize) -> Res<bool> {
    for u in g.units(Some(pid), None) {
        if g.tags(u)?.contains("Poro") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn herder_res() -> Value {
    Value::func("_herder_res", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        if !has_poro(g, c)? {
            return none();
        }
        if let Some(me) = src_alive(g, it) {
            g.buff(Some(me), None)?;
        }
        g.draw(c, 1)?;
        none()
    })
}

/// Legion Quartermaster — `sorted(g.gear(pid), key=lambda x: (not x.token, x.spec["e"] + 2 * x.spec["p"], x.uid))`
fn qm_gear(g: &Game, pid: usize) -> Vec<O> {
    let mut gs = g.gear(Some(pid));
    gs.sort_by_key(|&x| (!g.o(x).token, ep(g, x), g.o(x).uid));
    gs
}

fn qm_pay() -> Value {
    Value::func("_qm_pay", |g, a| {
        let (pid, ch) = (pid_of(&a[0]), a[2].map());
        let x = if ch.contains("ret_gear") {
            g.obj(ch.g("ret_gear").int() as u32)
        } else {
            // joué par un effet (play_unit_free) : le coût est quand même payé
            let gs = qm_gear(g, pid);
            if !gs.is_empty() {
                g.ask_obj(pid, "return_gear", &gs, Map::new())?
            } else {
                None
            }
        };
        if let Some(x) = x {
            g.to_zone(x, "hand", false, None)?;
        }
        none()
    })
}

/// Ribbon Dancer — "When I move to a battlefield, give another friendly unit +1 might this turn."
fn ribbon() -> Value {
    Value::func("_ribbon", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "move" && same(&info.g("obj"), o) && is_bf(info.g("to").opt_loc()) {
            let res = Value::func("_ribbon.res", |g, a| {
                let it = a[0].item();
                mod_legal0(g, it, 1)?;
                none()
            });
            let opts = Value::func("_ribbon.opts", |g, a| {
                let it = a[0].item();
                let (c, s) = (ctrl(g, it), g.it(it).src);
                let us: Vec<O> = friends(g, c, false, None)?.into_iter().filter(|&u| Some(g.o(u).uid) != s).collect();
                Ok(vo(&us).into())
            });
            let pred = Value::func("_ribbon.pred", |g, a| {
                let (it, u) = (a[0].item(), a[1].obj());
                Ok((g.o(u).ctrl == ctrl(g, it) && Some(g.o(u).uid) != g.it(it).src).into())
            });
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            trig(g, c, "Ribbon Dancer", res, Map::new(), Some(u), false, trig_target(opts, pred, "friendly_target", false, true), Value::None);
        }
        none()
    })
}

/// Riven, Shattered — `_equipment_on(g, u)`
fn equipment_on(g: &Game, u: O) -> i64 {
    g.o(u).attached.iter().filter(|&&x| g.obj(x).is_some_and(|x| g.spec(x).tags.contains("Equipment"))).count() as i64
}

fn riven() -> Value {
    Value::func("_riven", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "attack" && same(&info.g("obj"), o) {
            let here = g.o(o).loc;
            let res = Value::func("_riven.res", |g, a| {
                let it = a[0].item();
                let me = src_alive(g, it);
                let u = g.legal(it, 0)?;
                if let (Some(me), Some(u)) = (me, u) {
                    let n = 2 * equipment_on(g, me);
                    let c = ctrl(g, it);
                    g.deal(Some(u), n, "ability", Some(c), None)?;
                }
                none()
            });
            let (c, u, od, dmg) = (g.o(o).ctrl, uid(g, o), oid(g, o), 2 * equipment_on(g, o));
            let ch = trig_target(enemies_here_opts(here), enemy_here_pred(here), "target", false, true);
            trig(g, c, "Riven, Shattered", res, map! { oid = od, dmg = dmg }, Some(u), false, ch, Value::None);
        }
        none()
    })
}

/// Royal Entourage — "When you play me, ready or exhaust a legend."
fn entourage() -> Value {
    Value::func("_entourage", |g, a| {
        let o = a[0].obj();
        let choose = Value::func("_entourage.choose", |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            let mut opts: Vec<(i64, usize, &str)> = vec![];
            for q in [c, 1 - c] {
                let leg = g.p[q].legend;
                let (good, bad) = if q == c { ("ready", "exhaust") } else { ("exhaust", "ready") };
                opts.push((if (good == "ready") == g.exhausted(leg) { 0 } else { 1 }, q, good));
                opts.push((2, q, bad));
            }
            opts.sort();
            let vals: Vec<Value> =
                opts.iter().map(|&(_, q, act)| Value::Tuple(vec![Value::from(q), Value::from(act)])).collect();
            let pick = g.ask(c, "entourage", vals, Map::new())?;
            let pv = pick.list();
            let (l, ac) = (pv[0].clone(), pv[1].clone());
            g.it_mut(it).data.set("leg", l);
            g.it_mut(it).data.set("act", ac);
            Ok(true.into())
        });
        let res = Value::func("_entourage.res", |g, a| {
            let it = a[0].item();
            let leg = g.p[pid_of(&data(g, it, "leg"))].legend;
            if data(g, it, "act").opt_str() == Some("ready") {
                g.ready_obj(leg, None, None)?;
            } else {
                g.set_exhausted(leg, true);
            }
            none()
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        trig(g, c, "Royal Entourage", res, Map::new(), Some(u), false, choose, Value::None);
        none()
    })
}

/// Shadow — `_shadow_pred(g, it, o)`
fn shadow_pred() -> Value {
    Value::func("_shadow_pred", |g, a| {
        let (it, o) = (a[0].item(), a[1].obj());
        let me = g.it(it).src.and_then(|s| g.obj(s));
        Ok(me
            .is_some_and(|me| {
                is_unit(g, o) && g.o(o).ctrl != ctrl(g, it) && g.o(o).desig == Some(Desig::Att) && g.o(o).loc == g.o(me).loc
            })
            .into())
    })
}

/// Shen, Scourge of Shadows — `_shen_ok(g, uid, pid, bf)`
fn shen_ok(g: &Game, u: Option<u32>, pid: usize, bf: usize) -> bool {
    g.units(Some(pid), Some(Loc::Bf(bf))).iter().filter(|&&x| Some(g.o(x).uid) != u).count() == 1
}

fn shen() -> Value {
    Value::func("_shen", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "hold" && hold(g, o, info) && shen_ok(g, Some(uid(g, o)), g.o(o).ctrl, info.g("bf").int() as usize) {
            let bf = info.g("bf").int() as usize;
            let res = Value::func("_shen.res", move |g, a| {
                let it = a[0].item();
                let c = ctrl(g, it);
                if shen_ok(g, g.it(it).src, c, bf) {
                    g.draw(c, 1)?;
                }
                none()
            });
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            trig(g, c, "Shen, Scourge of Shadows", res, Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Simian Ancestor — "When you buff me, ready me."
fn simian() -> Value {
    Value::func("_simian", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        let c = g.o(o).ctrl;
        let by = if info.contains("by") { info.g("by") } else { Value::from(c) };
        if ev == "buff" && same(&info.g("obj"), o) && by == Value::from(c) {
            let res = Value::func("_simian.res", |g, a| {
                let it = a[0].item();
                if let Some(me) = src_alive(g, it) {
                    g.ready_obj(me, None, None)?;
                }
                none()
            });
            let (u, od) = (uid(g, o), oid(g, o));
            trig(g, c, "Simian Ancestor", res, map! { oid = od }, Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Solari Shieldbearer — "When you play me, stun a unit."
fn shieldbearer() -> Value {
    Value::func("_shieldbearer", |g, a| {
        let o = a[0].obj();
        let opts = Value::func("_shieldbearer.opts", |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            let es = enemies(g, c, false, None, true)?;
            let mut v: Vec<O> = es.iter().copied().filter(|&u| !g.o(u).stunned).collect();
            v.extend(es.iter().copied().filter(|&u| g.o(u).stunned));
            v.extend(friends(g, c, false, None)?);
            Ok(vo(&v).into())
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        let ch = trig_target(opts, p_unit_v(), "target", false, true);
        trig(g, c, "Solari Shieldbearer", stun_legal0("_shieldbearer.res"), Map::new(), Some(u), false, ch, Value::None);
        none()
    })
}

/// Sona, Harmonious — "While I'm at a battlefield, ready 4 friendly runes at the end of your turn."
fn sona() -> Value {
    Value::func("_sona", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "end_turn" && info.g("pid").opt_int() == Some(g.o(o).ctrl as i64) && is_bf(g.o(o).loc) {
            let res = Value::func("_sona.res", |g, a| {
                let it = a[0].item();
                let c = ctrl(g, it);
                ready_runes(g, c, 4)?;
                none()
            });
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            trig(g, c, "Sona, Harmonious", res, Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Taric, Protector — "Other friendly units here have [Shield]."
fn taric_aura() -> Value {
    Value::func("taric_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        if g.on_board(src) && o != src && g.o(o).ctrl == g.o(src).ctrl && g.o(o).loc == g.o(src).loc && is_unit(g, o) {
            Ok(map! { Shield = 1 }.into())
        } else {
            none()
        }
    })
}

/// Trevor Snoozebottom — "When I hold, play a ready 3 might Sprite unit token with [Temporary] here."
fn trevor() -> Value {
    Value::func("_trevor", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "hold" && hold(g, o, info) {
            let bf = info.g("bf").int() as usize;
            let res = Value::func("_trevor.res", move |g, a| {
                let it = a[0].item();
                let c = ctrl(g, it);
                make_token(g, "Sprite", c, Loc::Bf(bf), true)?;
                none()
            });
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            trig(g, c, "Trevor Snoozebottom", res, Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Vex, Mocking — "When you [Stun] an enemy unit at a battlefield, you may move me to that battlefield."
fn vex() -> Value {
    Value::func("_vex", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        let u = if ev == "stun" { info.g("obj").opt_obj() } else { None };
        let Some(u) = u else { return none() };
        let c = g.o(o).ctrl;
        if info.g("by").opt_int() == Some(c as i64) && g.o(u).ctrl != c && is_bf(g.o(u).loc) && g.o(o).loc != g.o(u).loc {
            let bf = g.o(u).loc.unwrap();
            let res = Value::func("_vex.res", move |g, a| {
                let it = a[0].item();
                if let Some(me) = src_alive(g, it) {
                    if g.o(me).loc != Some(bf) {
                        let c = ctrl(g, it);
                        g.move_(&[me], bf, Some(c), false)?;
                    }
                }
                none()
            });
            let (ou, od) = (uid(g, o), oid(g, o));
            trig(g, c, "Vex, Mocking", res, map! { oid = od }, Some(ou), true, Value::None, Value::None);
        }
        none()
    })
}

/// Yasuo, Remorseful — "When I attack, deal damage equal to my Might to an enemy unit here."
fn yasuo() -> Value {
    Value::func("_yasuo", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "attack" && same(&info.g("obj"), o) {
            let here = g.o(o).loc;
            let res = Value::func("_yasuo.res", |g, a| {
                let it = a[0].item();
                let me = src_alive(g, it);
                let u = g.legal(it, 0)?;
                if let (Some(me), Some(u)) = (me, u) {
                    // Might lue à l'exécution (règle 359.3.f.2)
                    let m = g.might(me)?;
                    let c = ctrl(g, it);
                    g.deal(Some(u), m, "ability", Some(c), None)?;
                }
                none()
            });
            let (c, u, od) = (g.o(o).ctrl, uid(g, o), oid(g, o));
            let dmg = g.might(o)?;
            let ch = trig_target(enemies_here_opts(here), enemy_here_pred(here), "target", false, true);
            trig(g, c, "Yasuo, Remorseful", res, map! { oid = od, dmg = dmg }, Some(u), false, ch, Value::None);
        }
        none()
    })
}

/// Yuumi, Magical Cat — "When I attack or defend, give one of your other units here +3 might and [Tank] this turn."
fn yuumi() -> Value {
    Value::func("_yuumi", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if (ev == "attack" || ev == "defend") && same(&info.g("obj"), o) {
            let here = g.o(o).loc;
            let res = Value::func("_yuumi.res", |g, a| {
                let it = a[0].item();
                if let Some(u) = g.legal(it, 0)? {
                    g.mod_(u, 3, "turn".into(), None)?;
                    g.grant(u, "Tank", 1, "turn".into());
                }
                none()
            });
            let opts = Value::func("_yuumi.opts", move |g, a| {
                let it = a[0].item();
                let (c, s) = (ctrl(g, it), g.it(it).src);
                let us: Vec<O> = friends(g, c, false, None)?
                    .into_iter()
                    .filter(|&u| g.o(u).loc == here && Some(g.o(u).uid) != s)
                    .collect();
                Ok(vo(&us).into())
            });
            let pred = Value::func("_yuumi.pred", move |g, a| {
                let (it, u) = (a[0].item(), a[1].obj());
                Ok((g.o(u).ctrl == ctrl(g, it) && g.o(u).loc == here).into())
            });
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            trig(g, c, "Yuumi, Magical Cat", res, Map::new(), Some(u), false, trig_target(opts, pred, "friendly_target", false, true), Value::None);
        }
        none()
    })
}

// ====================================================================== gear and equipment
/// Brutalizer — Effect Text: "If this was attached to me this turn, I have an additional +2 might."
fn brut_key(g: &Game, src: O, o: O) -> String {
    format!("('calm_brut', {}, {}, {}, {}, {})", uid(g, src), oid(g, src), uid(g, o), oid(g, o), g.turn_no)
}

fn brut_event() -> Value {
    Value::func("_brut_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "attached" && same(&info.g("gear"), o) {
            let u = info.g("unit").obj();
            let k = brut_key(g, o, u);
            let cur = g.stats.get(&k);
            g.stats.add(&k, 1 - cur); // g.stats[k] = 1
        }
        none()
    })
}

fn brut_aura() -> Value {
    Value::func("_brut_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        if g.o(src).attached_to != Some(uid(g, o)) || !g.on_board(src) {
            return Ok(0.into());
        }
        Ok(Value::Int(if g.stats.get(&brut_key(g, src, o)) != 0 { 2 } else { 0 }))
    })
}

/// Hand Hammer — "I have +2 might while I'm at a battlefield with exactly one other unit you control."
fn hammer_aura() -> Value {
    Value::func("_hammer_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        if g.o(src).attached_to != Some(uid(g, o)) || !g.on_board(src) || !is_bf(g.o(o).loc) {
            return Ok(0.into());
        }
        let n = units_at(g, Some(g.o(src).ctrl), g.o(o).loc).into_iter().filter(|&u| u != o).count();
        Ok(Value::Int(if n == 1 { 2 } else { 0 }))
    })
}

/// Forgefire Cape — Effect Text: "When I attack or defend, deal 2 to all enemy units here."
fn cape_effect() -> Value {
    Value::func("_cape_effect", |g, a| {
        let (unit, ev, info) = (a[1].obj(), a[2].str(), a[3].map());
        if (ev == "attack" || ev == "defend") && same(&info.g("obj"), unit) {
            let here = g.o(unit).loc;
            let res = Value::func("_cape_effect.res", move |g, a| {
                let it = a[0].item();
                let c = ctrl(g, it);
                for u in units_at(g, Some(1 - c), here) {
                    g.deal(Some(u), 2, "ability", Some(c), None)?;
                }
                none()
            });
            let (c, u) = (g.o(unit).ctrl, uid(g, unit));
            trig(g, c, "Forgefire Cape", res, Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Shurelya's Requiem — Effect Text: "Your units here have [Ganking]."
fn shurelya_aura() -> Value {
    Value::func("_shurelya_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        let u = g.o(src).attached_to.and_then(|x| g.obj(x));
        let Some(u) = u else { return none() };
        if !g.on_board(src) || !is_unit(g, o) || g.o(o).ctrl != g.o(src).ctrl || g.o(o).loc != g.o(u).loc {
            return none();
        }
        Ok(map! { Ganking = 1 }.into())
    })
}

/// Mask of Foresight — "When a friendly unit attacks or defends alone, give it +1 might this turn."
fn mask() -> Value {
    Value::func("_mask", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "attack" || ev == "defend" {
            let u = info.g("obj").obj();
            if g.o(u).ctrl == g.o(o).ctrl && g.alone(u, None) {
                let res = Value::func("_mask.res", |g, a| {
                    let it = a[0].item();
                    let x = alive(g, Some(data(g, it, "u").int() as u32), data(g, it, "oid").int());
                    if let Some(x) = x {
                        g.mod_(x, 1, "turn".into(), None)?;
                    }
                    none()
                });
                let (c, su, uu, uo) = (g.o(o).ctrl, uid(g, o), uid(g, u), oid(g, u));
                trig(g, c, "Mask of Foresight", res, map! { u = uu, oid = uo }, Some(su), false, Value::None, Value::None);
            }
        }
        none()
    })
}

/// Spirit's Refuge — "Friendly buffed units have [Deflect] if they didn't already."
fn refuge_aura() -> Value {
    Value::func("_refuge_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        if REFUGE_GUARD.with(|c| c.get())
            || !g.on_board(src)
            || !is_unit(g, o)
            || g.o(o).ctrl != g.o(src).ctrl
            || g.o(o).buff <= 0
        {
            return none();
        }
        let oc = g.o(o).ctrl;
        let first = g
            .board
            .iter()
            .copied()
            .filter(|&x| g.cname(x) == "Spirit's Refuge" && g.o(x).ctrl == oc)
            .min_by_key(|&x| g.o(x).uid);
        if first != Some(src) {
            return none(); // « if they didn't already » : un seul Deflect au plus
        }
        REFUGE_GUARD.with(|c| c.set(true));
        let has = g.kw_value(o, "Deflect");
        REFUGE_GUARD.with(|c| c.set(false));
        let has = has? > 0;
        if has {
            none()
        } else {
            Ok(map! { Deflect = 1 }.into())
        }
    })
}

fn refuge_play() -> Value {
    Value::func("_refuge_play", |g, a| {
        let o = a[0].obj();
        let res = Value::func("_refuge_play.res", |g, a| {
            let it = a[0].item();
            let u = g.legal(it, 0)?;
            g.buff(u, None)?;
            none()
        });
        let opts = Value::func("_refuge_play.opts", |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            let mut fr = friends(g, c, false, None)?;
            fr.sort_by_key(|&u| g.o(u).buff > 0);
            Ok(vo(&fr).into())
        });
        let (c, u) = (g.o(o).ctrl, uid(g, o));
        trig(g, c, "Spirit's Refuge", res, Map::new(), Some(u), false, trig_target(opts, p_friend_v(), "friendly_target", false, true), Value::None);
        none()
    })
}

/// Forgotten Signpost — "[Action][>] Exhaust a unit you control, [E]: Move a different unit you control to the
/// location of the unit you exhausted to pay for this ability."
fn signpost_choices() -> Value {
    Value::func("_signpost_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let mut out: Vec<Map> = vec![];
        let fr = friends(g, pid, false, None)?;
        for &e in &fr {
            if g.exhausted(e) {
                continue;
            }
            for m in friends(g, pid, false, None)? {
                if m != e && g.o(m).loc != g.o(e).loc {
                    out.push(map! { exh = uid(g, e), exh_oid = oid(g, e), tg = tg_of(g, &[m]) });
                }
            }
        }
        let out = ksort(out, |c| {
            let e = g.obj(c.g("exh").int() as u32).unwrap();
            Ok((!is_bf(g.o(e).loc), value(g, e)?))
        })?;
        Ok(maps(out))
    })
}

fn signpost_pay() -> Value {
    Value::func("_signpost_pay", |g, a| {
        let ch = a[2].map();
        if let Some(e) = g.obj(ch.g("exh").int() as u32) {
            g.set_exhausted(e, true);
        }
        none()
    })
}

fn signpost_res() -> Value {
    Value::func("_signpost_res", |g, a| {
        let it = a[0].item();
        let e = alive(g, Some(data(g, it, "exh").int() as u32), data(g, it, "exh_oid").int());
        let m = g.legal(it, 0)?;
        let (Some(e), Some(m)) = (e, m) else { return none() };
        if g.o(m).loc == g.o(e).loc {
            return none(); // emplacement nul (règle 359.3.e.12)
        }
        let (l, c) = (g.o(e).loc.unwrap(), ctrl(g, it));
        g.move_(&[m], l, Some(c), false)?;
        none()
    })
}

// ====================================================================== spells
fn res_fn(name: &'static str, f: fn(&mut Game, ItemId) -> Res<()>) -> Value {
    Value::func(name, move |g, a| {
        f(g, a[0].item())?;
        none()
    })
}

fn hb_of(ctx: &Map) -> Option<Loc> {
    ctx.g("hidden_bf").opt_loc()
}

/// `lambda g, pid, ctx: tg_choices(_units_first(g, pid, ctx["hidden_bf"]))`
fn units_first_choices() -> Value {
    Value::func("units_first_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let us = units_first(g, pid, hb)?;
        Ok(maps(tg_choices(g, &us)))
    })
}

/// Alpha Strike — "[Action] Choose a friendly unit. It deals damage equal to its Might split among enemy units at
/// battlefields. Then for each unit this kills, do this: Gain 1 XP."  (répartition : règle 355.14)
fn alpha_choices() -> Value {
    Value::func("_alpha_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let mut out: Vec<Map> = vec![];
        let es_all = enemies(g, pid, true, hb, true)?;
        let fr = friends(g, pid, false, None)?;
        let fr = ksort(fr, |&u| Ok(-g.might(u)?))?;
        for f in cap(g, fr, 3, Some(pid)) {
            let m = g.might(f)?;
            if m <= 0 || es_all.is_empty() {
                continue;
            }
            let es = ksort(es_all.clone(), |&u| Ok((1i64.max(g.might(u)? - g.o(u).damage), -value(g, u)?)))?;
            let (mut pick, mut left) = (vec![], m);
            for &e in &es {
                let need = 1i64.max(g.might(e)? - g.o(e).damage);
                if need <= left && (pick.len() as i64) < m {
                    pick.push(e);
                    left -= need;
                }
            }
            if !pick.is_empty() {
                let mut t = vec![f];
                t.extend(pick.iter().copied());
                out.push(map! { tg = tg_of(g, &t) });
            }
            for e in cap(g, es_all.clone(), 2, Some(pid)) {
                let c = map! { tg = tg_of(g, &[f, e]) };
                if !out.contains(&c) {
                    out.push(c);
                }
            }
            if full_choices(g, Some(pid)) {
                // un humain : chaque ensemble d'ennemis (chacun prend au moins 1)
                for n in 1..=(m.min(es_all.len() as i64) as usize) {
                    for grp in combinations(&es_all, n) {
                        let mut t = vec![f];
                        t.extend(grp);
                        let c = map! { tg = tg_of(g, &t) };
                        if !out.contains(&c) {
                            out.push(c);
                        }
                    }
                }
            }
        }
        Ok(maps(out))
    })
}

fn alpha(g: &mut Game, it: ItemId) -> Res<()> {
    let Some(f) = g.legal(it, 0)? else { return Ok(()) };
    let n = g.it(it).targets.len();
    let mut ts = vec![];
    for i in 1..n {
        ts.push(g.legal(it, i)?);
    }
    let ts: Vec<O> = ts.into_iter().flatten().collect();
    let c = ctrl(g, it);
    let m = g.might(f)?;
    let hit = split_damage(g, c, m, ts, "unit", c)?;
    let killed = kill_lethal(g, &hit, c)?;
    g.gain_xp(c, killed.len() as i64)
}

/// Arise! — "Play a 2 might Sand Soldier unit token for each Equipment you control. Then ready two of them."
fn arise(g: &mut Game, it: ItemId) -> Res<()> {
    let c = ctrl(g, it);
    let n = g.gear(Some(c)).into_iter().filter(|&x| g.spec(x).tags.contains("Equipment")).count();
    let mut toks = vec![];
    for _ in 0..n {
        toks.push(play_token(g, "Sand Soldier", c, None, false)?);
    }
    // Python : g.ready_obj(None) lèverait une exception quand le jeton n'a pas pu être joué (voir le compte rendu)
    for t in toks.into_iter().take(2).flatten() {
        g.ready_obj(t, None, None)?;
    }
    Ok(())
}

/// Combat Experience — "[Reaction] Give a unit +1 might this turn. [Level 6][>] Give it +3 might this turn instead."
fn combat_exp(g: &mut Game, it: ItemId) -> Res<()> {
    if g.legal(it, 0)?.is_some() {
        let u = g.legal(it, 0)?;
        let n = if g.level(ctrl(g, it), 6) { 3 } else { 1 };
        if let Some(u) = u {
            g.mod_(u, n, "turn".into(), None)?;
        }
    }
    Ok(())
}

/// Defiant Dance — "[Reaction] Give a unit +2 might this turn and another unit -2 might this turn."
fn dance(g: &mut Game, it: ItemId) -> Res<()> {
    let (a, b) = (g.legal(it, 0)?, g.legal(it, 1)?);
    if let Some(a) = a {
        g.mod_(a, 2, "turn".into(), None)?;
    }
    if let Some(b) = b {
        g.mod_(b, -2, "turn".into(), None)?;
    }
    Ok(())
}

fn dance_choices() -> Value {
    Value::func("_dance_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let (fr, en) = (friends(g, pid, false, hb)?, enemies(g, pid, false, hb, true)?);
        let mut out: Vec<Map> = vec![];
        for x in cap(g, fr.clone(), 3, Some(pid)) {
            for y in cap(g, en.clone(), 3, Some(pid)) {
                out.push(map! { tg = tg_of(g, &[x, y]) });
            }
        }
        let mut us = fr.clone();
        us.extend(en.iter().copied());
        let mut more = vec![];
        for &x in &us {
            for &y in &us {
                let c = map! { tg = tg_of(g, &[x, y]) };
                if x != y && !out.contains(&c) {
                    more.push(c);
                }
            }
        }
        out.extend(more);
        Ok(maps(out))
    })
}

/// Dragon's Rage — "Move an enemy unit. Then choose another enemy unit at its destination. They deal damage equal
/// to their Mights to each other."  (les deux unités ennemies sont choisies au moment de jouer, règle 355.5)
fn rage(g: &mut Game, it: ItemId) -> Res<()> {
    let Some(a) = g.legal(it, 0)? else { return Ok(()) }; // instructions liées (règle 359.3.e.14.a)
    let dest = data(g, it, "dest").loc();
    let c = ctrl(g, it);
    if g.o(a).loc != Some(dest) {
        g.move_(&[a], dest, Some(c), false)?;
    }
    let b = g.legal(it, 1)?;
    let Some(b) = b else { return Ok(()) };
    if !g.on_board(a) || g.o(b).loc != g.o(a).loc {
        return Ok(());
    }
    let (ma, mb) = (g.might(a)?, g.might(b)?);
    let ac = g.o(a).ctrl;
    g.deal(Some(b), ma, "unit", Some(ac), None)?; // règle 417.6.b.3-4 : les unités sont les sources
    let bc = g.o(b).ctrl;
    g.deal(Some(a), mb, "unit", Some(bc), None)?;
    Ok(())
}

fn rage_choices() -> Value {
    Value::func("_rage_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let es = enemies(g, pid, false, None, true)?;
        let mut out: Vec<Map> = vec![];
        for x in cap(g, es.clone(), 4, Some(pid)) {
            for d in g.locs() {
                if Some(d) == g.o(x).loc {
                    continue;
                }
                for &y in &es {
                    if y != x && g.o(y).loc == Some(d) {
                        out.push(map! { tg = tg_of(g, &[x, y]), dest = d });
                    }
                }
            }
        }
        let out = ksort(out, |c| {
            let (x, y) = (g.obj(tg_uid(c, 0)).unwrap(), g.obj(tg_uid(c, 1)).unwrap());
            let mut s = 0.0;
            if g.might(x)? >= g.might(y)? - g.o(y).damage {
                s -= value(g, y)?;
            }
            if g.might(y)? >= g.might(x)? - g.o(x).damage {
                s -= value(g, x)?;
            }
            Ok(s)
        })?;
        Ok(maps(out))
    })
}

/// Emperor's Divide — "[Hidden] [Action] Move any number of friendly units at a battlefield to their base."
fn divide(g: &mut Game, it: ItemId) -> Res<()> {
    let n = g.it(it).targets.len();
    let mut us = vec![];
    for i in 0..n {
        us.push(g.legal(it, i)?);
    }
    let us: Vec<O> = us.into_iter().flatten().collect();
    if !us.is_empty() {
        let c = ctrl(g, it);
        g.move_(&us, Loc::Base, Some(c), false)?;
    }
    Ok(())
}

fn divide_choices() -> Value {
    Value::func("_divide_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let mut out: Vec<Map> = vec![];
        for b in g.bf_locs() {
            if hb.is_some() && Some(b) != hb {
                continue;
            }
            let us = friends(g, pid, true, Some(b))?;
            if us.is_empty() {
                continue;
            }
            out.push(map! { tg = tg_of(g, &us) });
            if us.len() > 1 {
                for &u in &us {
                    out.push(map! { tg = tg_of(g, &[u]) });
                }
            }
        }
        out.push(map! { tg = Value::Tuple(vec![]) });
        Ok(maps(out))
    })
}

/// Flurry of Feathers — "[Reaction] Choose one — Counter a spell. / Play four 1 might Bird unit tokens with
/// [Deflect]."
fn do_counter_spell(g: &mut Game, it: ItemId) -> Res<()> {
    let tgt = item_of(g, &data(g, it, "item"));
    if let Some(t) = tgt {
        if g.it(t).kind == "spell" {
            g.counter(t)?;
        }
    }
    Ok(())
}

fn flurry(g: &mut Game, it: ItemId) -> Res<()> {
    if data(g, it, "mode").opt_str() == Some("counter") {
        do_counter_spell(g, it)?;
    } else {
        let c = ctrl(g, it);
        for _ in 0..4 {
            play_token(g, "Bird", c, None, false)?;
        }
    }
    Ok(())
}

fn flurry_choices() -> Value {
    Value::func("flurry_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let items = spell_items(g, pid)?;
        let v: Vec<Map> = items.iter().map(|&i| map! { mode = "counter", item = g.it(i).id }).collect();
        let mut out = cap(g, v, 6, Some(pid));
        out.push(map! { mode = "birds" });
        Ok(maps(out))
    })
}

/// Fox-Fire — "[Hidden] [Action] Kill any number of units at a battlefield with total Might 4 or less."
/// Ciblage de groupe (règle 355.11.b) : si les cibles ne conviennent plus à la résolution, le contrôleur choisit un
/// sous-ensemble qui convient.
fn foxfire_ok(g: &mut Game, grp: &[O]) -> Res<bool> {
    if grp.is_empty() {
        return Ok(false);
    }
    let locs: BTreeSet<Option<Loc>> = grp.iter().map(|&u| g.o(u).loc).collect();
    if locs.len() != 1 || !is_bf(g.o(grp[0]).loc) {
        return Ok(false);
    }
    let mut s = 0;
    for &u in grp {
        s += g.might(u)?;
    }
    Ok(s <= 4)
}

fn foxfire(g: &mut Game, it: ItemId) -> Res<()> {
    let targets = g.it(it).targets.clone();
    let c = ctrl(g, it);
    let mut us = vec![];
    for (u, o, _) in &targets {
        let x = u.and_then(|u| g.obj(u));
        if let Some(x) = x {
            if g.o(x).oid == *o && g.targetable(x, Some(c))? && is_unit(g, x) {
                us.push(x);
            }
        }
    }
    if us.is_empty() {
        return Ok(());
    }
    if !foxfire_ok(g, &us)? {
        let mut subs = vec![];
        for k in (1..=us.len()).rev() {
            for grp in combinations(&us, k) {
                if foxfire_ok(g, &grp)? {
                    subs.push(grp);
                }
            }
        }
        if subs.is_empty() {
            return Ok(());
        }
        let subs = ksort(subs, |grp| {
            let mut s = 0.0;
            for &u in grp {
                s += value(g, u)? * (if g.o(u).ctrl != c { 1.0 } else { -1.0 });
            }
            Ok(-s)
        })?;
        let opts: Vec<Value> = subs.iter().map(|grp| Value::Tuple(vo(grp))).collect();
        us = g.ask(c, "foxfire_subset", opts, Map::new())?.list().iter().map(|v| v.obj()).collect();
    }
    g.kill(&us, Some(c), false)?;
    Ok(())
}

fn foxfire_choices() -> Value {
    Value::func("_foxfire_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let mut out: Vec<Map> = vec![];
        for b in g.bf_locs() {
            if hb.is_some() && Some(b) != hb {
                continue;
            }
            // « units » : un humain peut aussi choisir des unités alliées ; l'IA seulement des ennemies
            let es0 = if full_choices(g, Some(pid)) {
                all_units(g, pid, true, Some(b))?
            } else {
                enemies(g, pid, true, Some(b), true)?
            };
            let es = cap(g, es0, 6, Some(pid));
            let mut cands = vec![];
            for k in 1..=es.len() {
                for grp in combinations(&es, k) {
                    let mut s = 0;
                    for &u in &grp {
                        s += g.might(u)?;
                    }
                    if s <= 4 {
                        cands.push(grp);
                    }
                }
            }
            let cands = ksort(cands, |grp| {
                let mut s = 0.0;
                for &u in grp {
                    s += value(g, u)? * (if g.o(u).ctrl != pid { 1.0 } else { 0.0 });
                }
                Ok(-s)
            })?;
            for grp in cap(g, cands, 4, Some(pid)) {
                out.push(map! { tg = tg_of(g, &grp) });
            }
        }
        out.push(map! { tg = Value::Tuple(vec![]) }); // « any number » comprend zéro (règle 355.13)
        Ok(maps(out))
    })
}

/// Friendship — "[Reaction] Choose a unit. Give it +1 might this turn for each of the following tags among your
/// units — Bird, Cat, Dog, and Poro."
fn friendship(g: &mut Game, it: ItemId) -> Res<()> {
    if g.legal(it, 0)?.is_some() {
        let u = g.legal(it, 0)?;
        let c = ctrl(g, it);
        let n = pet_tags(g, c)?.len() as i64;
        if let Some(u) = u {
            g.mod_(u, n, "turn".into(), None)?;
        }
    }
    Ok(())
}

/// Last Breath — "[Action] Ready a friendly unit. It deals damage equal to its Might to an enemy unit at a
/// battlefield."
fn last_breath(g: &mut Game, it: ItemId) -> Res<()> {
    let (f, e) = (g.legal(it, 0)?, g.legal(it, 1)?);
    let Some(f) = f else { return Ok(()) };
    g.ready_obj(f, None, None)?;
    if let Some(e) = e {
        let m = g.might(f)?;
        let c = ctrl(g, it);
        g.deal(Some(e), m, "unit", Some(c), None)?;
    }
    Ok(())
}

fn lb_choices() -> Value {
    Value::func("_lb_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let fr = friends(g, pid, false, hb)?;
        let fr = cap(g, fr, 3, Some(pid));
        let mut out: Vec<Map> = vec![];
        for f in fr {
            let en = enemies(g, pid, true, hb, true)?;
            for e in cap(g, en, 3, Some(pid)) {
                out.push(map! { tg = tg_of(g, &[f, e]) });
            }
        }
        let out = ksort(out, |c| {
            let (f, e) = (g.obj(tg_uid(c, 0)).unwrap(), g.obj(tg_uid(c, 1)).unwrap());
            Ok(-(if g.might(f)? >= g.might(e)? - g.o(e).damage { value(g, e)? } else { 0.0 }))
        })?;
        Ok(maps(out))
    })
}

/// Last Stand — "[Action] Double a friendly unit's Might this turn. Give it [Temporary]."
fn last_stand(g: &mut Game, it: ItemId) -> Res<()> {
    if let Some(u) = g.legal(it, 0)? {
        let m = g.might(u)?;
        g.mod_(u, m, "turn".into(), None)?;
        g.grant(u, "Temporary", 1, Value::None);
    }
    Ok(())
}

/// Meditation — "[Reaction] As an additional cost to play this, you may exhaust a friendly unit. If you do, draw 2.
/// Otherwise, draw 1."
fn med_pay() -> Value {
    Value::func("_med_pay", |g, a| {
        let ch = a[2].map();
        let u = if ch.g("exh").truthy() { g.obj(ch.g("exh").int() as u32) } else { None };
        if let Some(u) = u {
            g.set_exhausted(u, true);
        }
        none()
    })
}

fn med_choices() -> Value {
    Value::func("med_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let us = g.units(Some(pid), None);
        let us = ksort(us, |&u| value(g, u))?;
        let v: Vec<Map> = us.iter().filter(|&&u| !g.exhausted(u)).map(|&u| map! { exh = uid(g, u) }).collect();
        let mut out = cap(g, v, 4, Some(pid));
        out.push(Map::new());
        Ok(maps(out))
    })
}

/// Party Favors — "Each other player chooses Cards or Runes. For each player that chooses Cards, you and that
/// player each draw 1. For each player that chooses Runes, you and that player each channel 1 rune exhausted."
fn favors(g: &mut Game, it: ItemId) -> Res<()> {
    let c = ctrl(g, it);
    let opp = 1 - c;
    let pick = g.ask(opp, "party_favors", vec!["cards".into(), "runes".into()], Map::new())?;
    for q in [c, opp] {
        if pick.opt_str() == Some("cards") {
            g.draw(q, 1)?;
        } else {
            g.channel(q, 1, true)?;
        }
    }
    Ok(())
}

/// Reinforce — "Look at the top 5 cards of your Main Deck. You may banish a unit from among them, then play it,
/// reducing its cost by [5]. Recycle the remaining cards."
fn reinforce(g: &mut Game, it: ItemId) -> Res<()> {
    let pid = ctrl(g, it);
    let top0: Vec<O> = g.p[pid].deck.iter().take(5).copied().collect();
    let top = g.look(pid, &top0, None)?;
    let mut opts: Vec<(O, Map)> = vec![];
    for &c in &top {
        let im = g.impl_o(c);
        if !is_unit(g, c) || im.is_none() {
            continue;
        }
        for ch in unit_play_choices(g, pid, c, "banish", None)? {
            let (e, reqs) = total_cost(g, pid, c, &ch, "banish")?;
            let ctx = pay_ctx(g, c);
            if g.can_pay(pid, 0i64.max(e - 5), &reqs, &ctx)? {
                opts.push((c, ch));
            }
        }
    }
    let opts = ksort(opts, |o| Ok((-ep(g, o.0), o.1.g("loc").opt_loc() != Some(Loc::Base), o.1.g("acc").truthy())))?;
    let mut vals: Vec<Value> =
        cap(g, opts, 12, Some(pid)).into_iter().map(|(c, ch)| Value::Tuple(vec![c.into(), ch.into()])).collect();
    vals.push(Value::None);
    let pick = g.ask(pid, "reinforce_pick", vals, Map::new())?;
    let pick: Option<(O, Map)> = if pick.is_none() {
        None
    } else {
        let v = pick.list();
        Some((v[0].obj(), v[1].map().clone()))
    };
    let rest: Vec<O> = top.iter().copied().filter(|&c| pick.as_ref().is_none_or(|p| c != p.0)).collect();
    if let Some((c, ch)) = pick {
        let (e, reqs) = total_cost(g, pid, c, &ch, "banish")?;
        g.to_zone(c, "banish", false, None)?;
        let cn = g.cname(c).to_string();
        g.log(|| format!("  Reinforce banishes and plays {cn}"));
        let ctx = pay_ctx(g, c);
        if g.pay(pid, 0i64.max(e - 5), &reqs, &ctx)? {
            play_card(g, pid, c, "banish", ch.clone(), true)?;
        }
    }
    for &c in &rest {
        deck_remove(g, pid, c);
    }
    g.recycle_cards(pid, &rest)?;
    Ok(())
}

/// Resonating Strike — "[Hidden] [Reaction] Choose a battlefield you control and a unit you control at a different
/// location. Move that unit to that battlefield and give it +2 might this turn."
fn resonating(g: &mut Game, it: ItemId) -> Res<()> {
    let u = g.legal(it, 0)?;
    let b = data(g, it, "bf").int() as usize;
    let Some(u) = u else { return Ok(()) };
    let c = ctrl(g, it);
    if g.bfs[b].ctrl == Some(c) && g.o(u).loc != Some(Loc::Bf(b)) {
        g.move_(&[u], Loc::Bf(b), Some(c), false)?;
    }
    g.mod_(u, 2, "turn".into(), None)
}

fn resonating_choices() -> Value {
    Value::func("_resonating_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let mut out: Vec<Map> = vec![];
        for bi in 0..g.bfs.len() {
            let idx = g.bfs[bi].idx;
            if g.bfs[bi].ctrl != Some(pid) || (hb.is_some() && hb != Some(Loc::Bf(idx))) {
                continue;
            }
            for u in friends(g, pid, false, None)? {
                if g.o(u).loc != Some(Loc::Bf(idx)) {
                    out.push(map! { bf = idx, tg = tg_of(g, &[u]) });
                }
            }
        }
        Ok(maps(out))
    })
}

/// Rune Prison — "[Action] Stun a unit."
fn rune_prison_choices() -> Value {
    Value::func("rune_prison_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let mut v: Vec<O> = enemies(g, pid, false, hb, true)?.into_iter().filter(|&u| !g.o(u).stunned).collect();
        v.extend(enemies(g, pid, false, hb, true)?.into_iter().filter(|&u| g.o(u).stunned));
        v.extend(friends(g, pid, false, hb)?);
        Ok(maps(tg_choices(g, &v)))
    })
}

/// Sanction — "[Reaction] Choose one — Empower a unit. Disempower it at end of turn. / Disempower a unit that's
/// [Empowered]. Empower it at end of turn."
fn sanction_end() -> Value {
    Value::func("_sanction_end", |g, a| {
        let eid = a[0].int() as u64;
        let eff = g.eff(eid).unwrap().clone();
        g.remove_effect(eid);
        let res = Value::func("_sanction_end.res", |g, a| {
            let it = a[0].item();
            let u = alive(g, Some(data(g, it, "u").int() as u32), data(g, it, "oid").int());
            let Some(u) = u else { return none() };
            if data(g, it, "mode").opt_str() == Some("emp") {
                g.disempower(Some(u));
            } else {
                g.empower(Some(u), None)?;
            }
            none()
        });
        let d = map! { u = eff.g("u"), oid = eff.g("oid"), mode = eff.g("mode") };
        trig(g, pid_of(&eff.g("pid")), "Sanction (end of turn)", res, d, None, false, Value::None, Value::None);
        none()
    })
}

fn sanction(g: &mut Game, it: ItemId) -> Res<()> {
    let Some(u) = g.legal(it, 0)? else { return Ok(()) };
    let mode = data(g, it, "mode");
    if mode.opt_str() == Some("emp") {
        g.empower(Some(u), None)?;
    } else {
        g.disempower(Some(u));
    }
    let mut d = Map::new();
    d.set("on", "end_turn");
    d.set("fn", sanction_end());
    d.set("pid", ctrl(g, it));
    d.set("u", uid(g, u));
    d.set("oid", oid(g, u));
    d.set("mode", mode);
    d.set("dur", "turn");
    g.add_effect(d);
    Ok(())
}

fn sanction_choices() -> Value {
    Value::func("_sanction_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let (fr, en) = (friends(g, pid, false, hb)?, enemies(g, pid, false, hb, true)?);
        let mut out: Vec<Map> = vec![];
        for &u in en.iter().filter(|&&u| g.o(u).empowered) {
            out.push(map! { tg = tg_of(g, &[u]), mode = "dis" });
        }
        for &u in fr.iter().filter(|&&u| !g.o(u).empowered) {
            out.push(map! { tg = tg_of(g, &[u]), mode = "emp" });
        }
        for &u in en.iter().filter(|&&u| !g.o(u).empowered) {
            out.push(map! { tg = tg_of(g, &[u]), mode = "emp" });
        }
        for &u in fr.iter().filter(|&&u| g.o(u).empowered) {
            out.push(map! { tg = tg_of(g, &[u]), mode = "dis" });
        }
        Ok(maps(out))
    })
}

fn sanction_pred() -> Value {
    Value::func("sanction_pred", |g, a| {
        let (it, o) = (a[0].item(), a[1].obj());
        Ok((p_unit(g, it, o) && (data(g, it, "mode").opt_str() == Some("emp") || g.o(o).empowered)).into())
    })
}

/// Shadow Dash — "Move an enemy unit to a battlefield where you have units. If you have exactly two units there,
/// they each get +1 might this turn. [Flow] [5][A][A]"
fn dash(g: &mut Game, it: ItemId) -> Res<()> {
    let u = g.legal(it, 0)?;
    let d = data(g, it, "dest").loc();
    let c = ctrl(g, it);
    if let Some(u) = u {
        if g.o(u).loc != Some(d) && !g.units(Some(c), Some(d)).is_empty() {
            g.move_(&[u], d, Some(c), false)?;
        }
    }
    let mine = g.units(Some(c), Some(d));
    if mine.len() == 2 {
        for x in mine {
            g.mod_(x, 1, "turn".into(), None)?;
        }
    }
    Ok(())
}

fn dash_choices() -> Value {
    Value::func("_dash_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let mut out: Vec<Map> = vec![];
        for u in enemies(g, pid, false, None, true)? {
            for b in g.bf_locs() {
                if g.o(u).loc != Some(b) && !g.units(Some(pid), Some(b)).is_empty() {
                    out.push(map! { tg = tg_of(g, &[u]), dest = b });
                }
            }
        }
        let out = ksort(out, |c| {
            let n = g.units(Some(pid), Some(c.g("dest").loc())).len();
            let x = g.obj(tg_uid(c, 0)).unwrap();
            Ok((n != 2, -value(g, x)?))
        })?;
        Ok(maps(out))
    })
}

/// Siphoning Strike — "Deal 4 to a unit at a battlefield. If you control 7 or more runes, deal 7 to it instead.
/// When it dies this turn, channel 1 rune exhausted."
fn siphon_die() -> Value {
    Value::func("_siphon_die", |g, a| {
        let eid = a[0].int() as u64;
        let info = a[1].map();
        let eff = g.eff(eid).unwrap().clone();
        let o = info.g("info").map().g("obj").obj();
        if Value::from(g.o(o).uid) != eff.g("u") || g.o(o).oid != eff.g("oid").int() + 1 {
            return none();
        }
        g.remove_effect(eid);
        let res = Value::func("_siphon_die.res", |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            g.channel(c, 1, true)?;
            none()
        });
        trig(g, pid_of(&eff.g("pid")), "Siphoning Strike (dies)", res, Map::new(), None, false, Value::None, Value::None);
        none()
    })
}

fn siphon(g: &mut Game, it: ItemId) -> Res<()> {
    let Some(u) = g.legal(it, 0)? else { return Ok(()) };
    let c = ctrl(g, it);
    let mut d = Map::new();
    d.set("on", "die");
    d.set("fn", siphon_die());
    d.set("pid", c);
    d.set("u", uid(g, u));
    d.set("oid", oid(g, u));
    d.set("dur", "turn");
    g.add_effect(d);
    let n = if g.p[c].runes.len() >= 7 { 7 } else { 4 };
    g.deal(Some(u), n, "spell", Some(c), None)?;
    Ok(())
}

fn enemies_bf_choices() -> Value {
    Value::func("enemies_bf_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let es = enemies(g, pid, true, hb, true)?;
        Ok(maps(tg_choices(g, &es)))
    })
}

/// Skyward Strike — "Move an enemy unit. [Level 6][>] [Stun] an enemy unit."
fn skyward(g: &mut Game, it: ItemId) -> Res<()> {
    let u = g.legal(it, 0)?;
    let dest = data(g, it, "dest").loc();
    let c = ctrl(g, it);
    if let Some(u) = u {
        if g.o(u).loc != Some(dest) {
            g.move_(&[u], dest, Some(c), false)?;
        }
    }
    if g.it(it).targets.len() > 1 && g.level(c, 6) {
        let s = g.legal(it, 1)?;
        g.stun(s, Some(c))?;
    }
    Ok(())
}

fn skyward_choices() -> Value {
    Value::func("_skyward_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let es = enemies(g, pid, false, None, true)?;
        let lv = g.level(pid, 6);
        let mut out: Vec<Map> = vec![];
        for u in cap(g, es.clone(), 4, Some(pid)) {
            for d in g.locs() {
                if Some(d) == g.o(u).loc {
                    continue;
                }
                if lv {
                    let ss: Vec<O> = if full_choices(g, Some(pid)) {
                        es.clone()
                    } else {
                        let ns: Vec<O> = es.iter().copied().filter(|&x| !g.o(x).stunned).take(2).collect();
                        if !ns.is_empty() {
                            ns
                        } else {
                            es.iter().copied().take(1).collect()
                        }
                    };
                    for s in ss {
                        out.push(map! { tg = tg_of(g, &[u, s]), dest = d });
                    }
                } else {
                    out.push(map! { tg = tg_of(g, &[u]), dest = d });
                }
            }
        }
        let out = ksort(out, |c| {
            let x = g.obj(tg_uid(c, 0)).unwrap();
            Ok((c.g("dest").opt_loc() != Some(Loc::Base), -value(g, x)?))
        })?;
        Ok(maps(out))
    })
}

/// Thwonk! — "[Action] [Repeat] [2] Stun an attacking unit."
fn thwonk_pred() -> Value {
    Value::func("thwonk_pred", |g, a| {
        let (it, o) = (a[0].item(), a[1].obj());
        Ok((is_unit(g, o) && g.o(o).desig == Some(Desig::Att) && hb_ok(g, it, o)).into())
    })
}

fn thwonk_choices() -> Value {
    Value::func("thwonk_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let mut us = enemies(g, pid, false, hb, true)?;
        us.extend(friends(g, pid, false, hb)?);
        let us: Vec<O> = us.into_iter().filter(|&u| g.o(u).desig == Some(Desig::Att)).collect();
        Ok(maps(tg_choices(g, &us)))
    })
}

/// Tricksy Tentacles — "Move any number of enemy units with the same controller and a total Might of 8 or less to
/// a single location."  (ciblage de groupe, règle 355.11)
fn same_ctrl_might8(g: &mut Game, grp: &[O]) -> Res<bool> {
    let cs: BTreeSet<usize> = grp.iter().map(|&u| g.o(u).ctrl).collect();
    if cs.len() != 1 {
        return Ok(false);
    }
    let mut s = 0;
    for &u in grp {
        s += g.might(u)?;
    }
    Ok(s <= 8)
}

fn tricksy(g: &mut Game, it: ItemId) -> Res<()> {
    let n = g.it(it).targets.len();
    let mut us = vec![];
    for i in 0..n {
        us.push(g.legal(it, i)?);
    }
    let mut us: Vec<O> = us.into_iter().flatten().collect();
    if us.is_empty() {
        return Ok(());
    }
    let c = ctrl(g, it);
    let cs: BTreeSet<usize> = us.iter().map(|&u| g.o(u).ctrl).collect();
    let mut over = cs.len() > 1;
    if !over {
        let mut s = 0;
        for &u in &us {
            s += g.might(u)?;
        }
        over = s > 8;
    }
    if over {
        let mut subs = vec![];
        for k in (1..=us.len()).rev() {
            for grp in combinations(&us, k) {
                if same_ctrl_might8(g, &grp)? {
                    subs.push(grp);
                }
            }
        }
        if subs.is_empty() {
            return Ok(());
        }
        let subs = ksort(subs, |grp| {
            let mut s = 0.0;
            for &u in grp {
                s += value(g, u)?;
            }
            Ok(-s)
        })?;
        let opts: Vec<Value> = subs.iter().map(|grp| Value::Tuple(vo(grp))).collect();
        us = g.ask(c, "tricksy_subset", opts, Map::new())?.list().iter().map(|v| v.obj()).collect();
    }
    let dest = data(g, it, "dest").loc();
    let movers: Vec<O> = us.into_iter().filter(|&u| g.o(u).loc != Some(dest)).collect();
    if !movers.is_empty() {
        g.move_(&movers, dest, Some(c), false)?;
    }
    Ok(())
}

fn tricksy_choices() -> Value {
    Value::func("_tricksy_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let es0 = enemies(g, pid, false, None, true)?;
        let es = cap(g, es0, 6, Some(pid));
        let mut grps = vec![];
        for k in (1..=es.len()).rev() {
            for grp in combinations(&es, k) {
                let mut s = 0;
                for &u in &grp {
                    s += g.might(u)?;
                }
                if s <= 8 {
                    grps.push(grp);
                }
            }
        }
        let grps = ksort(grps, |grp| {
            let mut s = 0.0;
            for &u in grp {
                if is_bf(g.o(u).loc) {
                    s += value(g, u)?;
                }
            }
            Ok(-s)
        })?;
        let mut out: Vec<Map> = vec![];
        for grp in cap(g, grps, 3, Some(pid)) {
            for d in g.locs() {
                if grp.iter().any(|&u| g.o(u).loc != Some(d)) {
                    out.push(map! { tg = tg_of(g, &grp), dest = d });
                }
            }
        }
        out.sort_by_key(|c| c.g("dest").opt_loc() != Some(Loc::Base));
        out.push(map! { tg = Value::Tuple(vec![]) }); // « any number » comprend zéro (règle 355.13)
        Ok(maps(out))
    })
}

/// Wind Wall — "[Reaction] Counter a spell."
fn wind_wall_choices() -> Value {
    Value::func("wind_wall_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let items = spell_items(g, pid)?;
        Ok(maps(items.iter().map(|&i| map! { item = g.it(i).id }).collect()))
    })
}

/// Zenith Blade — "[Action] Stun an enemy unit at a battlefield. You may move a friendly unit to that enemy unit's
/// battlefield."
fn zenith(g: &mut Game, it: ItemId) -> Res<()> {
    let Some(e) = g.legal(it, 0)? else { return Ok(()) };
    let c = ctrl(g, it);
    g.stun(Some(e), Some(c))?;
    let m = g.legal(it, 1)?;
    if let Some(m) = m {
        if is_bf(g.o(e).loc) && g.o(m).loc != g.o(e).loc {
            let l = g.o(e).loc.unwrap();
            g.move_(&[m], l, Some(c), false)?;
        }
    }
    Ok(())
}

fn zenith_choices() -> Value {
    Value::func("_zenith_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let es = enemies(g, pid, true, hb, true)?;
        let mut order: Vec<O> = es.iter().copied().filter(|&u| !g.o(u).stunned).collect();
        order.extend(es.iter().copied().filter(|&u| g.o(u).stunned));
        let mut out: Vec<Map> = vec![];
        for e in order {
            out.push(map! { tg = tg_of(g, &[e]) });
            let fr = friends(g, pid, false, None)?;
            for m in cap(g, fr, 2, Some(pid)) {
                if g.o(m).loc != g.o(e).loc {
                    out.push(map! { tg = tg_of(g, &[e, m]) });
                }
            }
        }
        Ok(maps(out))
    })
}

// ====================================================================== cartes qui utilisent les crochets du moteur
/// Alpha Wildclaw — "Your units here with less Might than me can't be chosen by enemy spells and abilities."
fn wildclaw_aura() -> Value {
    Value::func("_wildclaw_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        if !(o != src && g.o(o).ctrl == g.o(src).ctrl && is_unit(g, o) && g.o(o).loc == g.o(src).loc && g.on_board(src)) {
            return Ok(false.into());
        }
        Ok((g.might(o)? < g.might(src)?).into())
    })
}

/// Twilight Shroud — "Give a friendly unit +1 might this turn. It can't be chosen by enemy spells and abilities
/// this turn. [Flow] 2 energy"
fn shroud(g: &mut Game, it: ItemId) -> Res<()> {
    if let Some(u) = g.legal(it, 0)? {
        g.mod_(u, 1, "turn".into(), None)?;
        g.grant(u, "Untargetable", 1, "turn".into());
    }
    Ok(())
}

/// Counter Strike — "[Reaction] Choose a unit. The next time that unit would be dealt damage this turn, prevent
/// it. Draw 1."
fn counter_strike(g: &mut Game, it: ItemId) -> Res<()> {
    if let Some(u) = g.legal(it, 0)? {
        let mut d = Map::new();
        d.set("kind", "prevent_next");
        d.set("uid", uid(g, u));
        d.set("oid", oid(g, u));
        d.set("dur", "turn");
        g.add_effect(d);
    }
    let c = ctrl(g, it);
    g.draw(c, 1)
}

fn counter_strike_choices() -> Value {
    Value::func("counter_strike_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let fr = friends(g, pid, false, hb)?;
        let mut v = ksort(fr, |&u| Ok((-value(g, u)?, g.o(u).uid)))?;
        let mut en = enemies(g, pid, false, hb, true)?;
        en.sort_by_key(|&u| g.o(u).uid);
        v.extend(en);
        Ok(maps(tg_choices(g, &v)))
    })
}

/// Highlander — "[Reaction] Choose a friendly unit. The next time it dies this turn, recall it exhausted instead."
fn highlander_rep() -> Value {
    Value::func("_highlander_rep", |g, a| {
        let eid = a[0].int() as u64;
        let u = a[1].obj();
        let eff = g.eff(eid).unwrap().clone();
        if Value::from(g.o(u).uid) != eff.g("uid") || Value::from(g.o(u).oid) != eff.g("oid") {
            return none();
        }
        let apply = Value::func("_highlander_rep.apply", move |g, a| {
            let x = a[0].obj();
            if g.has_effect(eid) {
                g.remove_effect(eid);
            }
            g.save_unit(x, false, true);
            none()
        });
        Ok(map! { name = "Highlander", apply = apply }.into())
    })
}

fn highlander(g: &mut Game, it: ItemId) -> Res<()> {
    if let Some(u) = g.legal(it, 0)? {
        let mut d = Map::new();
        d.set("kind", "death_rep");
        d.set("fn", highlander_rep());
        d.set("uid", uid(g, u));
        d.set("oid", oid(g, u));
        d.set("dur", "turn");
        g.add_effect(d);
    }
    Ok(())
}

fn friends_by_value_choices() -> Value {
    Value::func("friends_by_value_choices", |g, a| {
        let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
        let fr = friends(g, pid, false, hb)?;
        let v = ksort(fr, |&u| Ok((-value(g, u)?, g.o(u).uid)))?;
        Ok(maps(tg_choices(g, &v)))
    })
}

/// Guardian Angel — Effect Text : "If I would die, kill Guardian Angel instead. Heal me, exhaust me, and recall me."
fn angel_rep() -> Value {
    Value::func("_angel_rep", |_g, a| {
        let gear = a[0].obj();
        let apply = Value::func("_angel_rep.apply", move |g, a| {
            let x = a[0].obj();
            if g.on_board(gear) {
                let c = g.o(x).ctrl;
                g.kill(&[gear], Some(c), false)?;
            }
            g.save_unit(x, true, true);
            none()
        });
        Ok(map! { name = "Guardian Angel", apply = apply }.into())
    })
}

/// Vilemaw — "[Ambush] Enemy units here with less Might than me don't deal combat damage. When I hold, draw 1."
fn vilemaw_event() -> Value {
    Value::func("_vilemaw_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "hold" && hold(g, o, info) {
            let (c, u) = (g.o(o).ctrl, uid(g, o));
            trig(g, c, "Vilemaw", draw1("_vilemaw_event.res"), Map::new(), Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Helm of Suppression — "Opponents' spells cost 1 energy more. If this is [Empowered], they cost 1 energy and 1
/// rune of any type more instead."
fn helm_tax() -> Value {
    Value::func("_helm_tax", |g, a| {
        let (src, pid, what) = (a[0].obj(), pid_of(&a[1]), a[2].map());
        if pid == g.o(src).ctrl || what.g("kind").opt_str() != Some("spell") {
            return Ok(Value::Tuple(vec![]));
        }
        if g.o(src).empowered {
            Ok(maps(vec![map! { e = 1, p = reqs_to_value(&[any_req()]) }]))
        } else {
            Ok(maps(vec![map! { e = 1 }]))
        }
    })
}

/// Lilting Lullaby — "[Reaction] Counter a spell. Its controller can't play spells this turn."
fn lullaby(g: &mut Game, it: ItemId) -> Res<()> {
    let tgt = item_of(g, &data(g, it, "item"));
    let Some(t) = tgt else { return Ok(()) };
    if g.it(t).kind != "spell" {
        return Ok(());
    }
    let who = g.it(t).ctrl;
    g.counter(t)?;
    let mut d = Map::new();
    d.set("kind", "no_play");
    d.set("pid", who);
    d.set("dur", "turn");
    d.set(
        "fn",
        Value::func("_lullaby.no_play", |g, a| {
            let (eid, pid, c) = (a[0].int() as u64, a[1].clone(), a[2].obj());
            let ep = g.eff(eid).map(|e| e.g("pid")).unwrap_or_default();
            Ok((pid == ep && g.spec(c).type_ == "Spell").into())
        }),
    );
    g.add_effect(d);
    Ok(())
}

fn lullaby_choices() -> Value {
    Value::func("lullaby_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let sp: Vec<ItemId> = g.chain.clone().into_iter().rev().filter(|&i| g.it(i).kind == "spell").collect();
        let sp = ksort(sp, |&i| Ok((g.it(i).ctrl == pid, !g.counterable(i)?)))?;
        Ok(maps(sp.iter().map(|&i| map! { item = g.it(i).id }).collect()))
    })
}

/// Lillia, Protector of Dreams — "When you play a token unit, give me +1 might this turn. Your token units have
/// [Tank]."
fn lillia_event() -> Value {
    Value::func("_lillia_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "played"
            && info.g("token").truthy()
            && info.g("pid").opt_int() == Some(g.o(o).ctrl as i64)
            && is_unit(g, info.g("card").obj())
        {
            let res = Value::func("_lillia_event.res", |g, a| {
                let it = a[0].item();
                if src_alive(g, it).is_some() {
                    if let Some(me) = src_alive(g, it) {
                        g.mod_(me, 1, "turn".into(), None)?;
                    }
                }
                none()
            });
            let (c, u, od) = (g.o(o).ctrl, uid(g, o), oid(g, o));
            trig(g, c, "Lillia, Protector of Dreams", res, map! { oid = od }, Some(u), false, Value::None, Value::None);
        }
        none()
    })
}

/// Solari Shrine — "When you kill a stunned enemy unit, you may exhaust this to draw 1."
fn shrine_event() -> Value {
    Value::func("_shrine_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev != "die" {
            return none();
        }
        let d = info.g("info");
        let d = d.map();
        let c = g.o(o).ctrl;
        if d.g("stunned").truthy()
            && d.g("by").opt_int() == Some(c as i64)
            && d.g("ctrl").opt_int() != Some(c as i64)
            && spec(d.g("spec").str()).type_ == "Unit"
            && !g.exhausted(o)
        {
            let cost = Value::func("_shrine_event.cost", |g, a| {
                let it = a[0].item();
                let me = src_alive(g, it);
                let Some(me) = me else { return Ok(false.into()) };
                if g.exhausted(me) {
                    return Ok(false.into());
                }
                let cc = ctrl(g, it);
                if !g.ask(cc, "solari_shrine", vec![true.into(), false.into()], Map::new())?.truthy() {
                    return Ok(false.into());
                }
                g.set_exhausted(me, true);
                Ok(true.into())
            });
            let (u, od) = (uid(g, o), oid(g, o));
            trig(g, c, "Solari Shrine", draw1("_shrine_event.res"), map! { oid = od }, Some(u), false, Value::None, cost);
        }
        none()
    })
}

/// Svellsongur — "As this is attached to a unit, copy that unit's text to this Equipment's effect text for as long
/// as this is attached to it."
fn svell_event() -> Value {
    Value::func("_svell_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "attached" && same(&info.g("gear"), o) {
            let u = info.g("unit").obj();
            let cn = g.cname(u).to_string();
            g.o_mut(o).copy_name = Some(cn);
            let r = g.repr(u);
            g.log(|| format!("  Svellsongur copies the text of {r}"));
        }
        none()
    })
}

/// Mystic Reversal — "[Reaction] Gain control of a spell. You may make new choices for it."
fn reversal(g: &mut Game, it: ItemId) -> Res<()> {
    let tgt = item_of(g, &data(g, it, "item"));
    let Some(t) = tgt else { return Ok(()) };
    if g.it(t).kind != "spell" {
        return Ok(());
    }
    let c = ctrl(g, it);
    gain_control_item(g, t, c);
    remake_choices(g, t, c)
}

fn reversal_choices() -> Value {
    Value::func("reversal_choices", |g, a| {
        let pid = pid_of(&a[0]);
        let mut sp: Vec<ItemId> = g.chain.clone().into_iter().rev().filter(|&i| g.it(i).kind == "spell").collect();
        sp.sort_by_key(|&i| g.it(i).ctrl == pid);
        Ok(maps(sp.iter().map(|&i| map! { item = g.it(i).id }).collect()))
    })
}

// ====================================================================== enregistrement (ordre de calm.py)
fn preds(v: Vec<Value>) -> Value {
    Value::List(v)
}

fn kwm(pairs: &[(&str, i64)]) -> Map {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.set(k, *v);
    }
    m
}

pub fn register(r: &mut Registry) {
    // ---------------------------------------------------------------- units
    // Affectionate Poro — "When a combat that I was in ends, if I haven't been dealt damage this turn, draw 1."
    reg(r, Impl { on_event: aff_poro(), ..Impl::new("Affectionate Poro") });
    // Ahri, Alluring — "When I hold, you score 1 point."
    reg(r, Impl { on_event: ahri(), ..Impl::new("Ahri, Alluring") });
    // Allay, Eager Admirer — "[Deflect] While I'm at a battlefield, your other units here have [Deflect]."
    reg(r, Impl { kw: kwm(&[("Deflect", 1)]), aura_kw: allay_aura(), ..Impl::new("Allay, Eager Admirer") });
    // Aphelios, Exalted — "When you attach an Equipment to me, choose one that hasn't been chosen this turn — Ready 2
    // runes. / Channel 1 rune exhausted. / Buff a friendly unit."
    reg(r, Impl { on_event: aphelios(), ..Impl::new("Aphelios, Exalted") });
    // Apprentice Smith — "When I move, reveal the top card of your Main Deck. If it's a gear, draw it. Otherwise,
    // recycle it."
    reg(
        r,
        Impl {
            on_event: Value::func("smith_event", |g, a| {
                let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
                if ev == "move" && same(&info.g("obj"), o) {
                    let (c, u) = (g.o(o).ctrl, uid(g, o));
                    trig(g, c, "Apprentice Smith", smith_res(), Map::new(), Some(u), false, Value::None, Value::None);
                }
                none()
            }),
            ..Impl::new("Apprentice Smith")
        },
    );
    // Azir, Ascendant — "[Calm]: [Action] — Choose a unit you control. Move me to its location and it to my original
    // location. If it's equipped, you may attach one of its Equipment to me. Use only once per turn."
    reg(
        r,
        Impl {
            abilities: vec![ability(
                "Swap",
                AbilityArgs {
                    cost: Cost::from("1 calm rune"),
                    timing: "action",
                    preds: preds(vec![p_friend_v()]),
                    choices: azir_choices(),
                    resolve: azir_res(),
                    can: Value::func("azir_can", |g, a| {
                        let o = a[1].obj();
                        Ok((g.stats.get(&azir_key(g, o)) == 0).into())
                    }),
                    extra_cost: Value::func("_azir_mark", |g, a| {
                        let o = a[1].obj();
                        let k = azir_key(g, o);
                        g.stats.add(&k, 1);
                        none()
                    }),
                    ..Default::default()
                },
            )],
            ..Impl::new("Azir, Ascendant")
        },
    );
    // Caitlyn, Patrolling — "I must be assigned combat damage last. [E]: Deal damage equal to my Might to a unit at a
    // battlefield. Use this ability only while I'm at a battlefield."
    // La première capacité est Backline (la règle 465.2.c.8 parle de « Caitlyn, Patrolling with the Backline ability »).
    reg(
        r,
        Impl {
            kw: kwm(&[("Backline", 1)]),
            abilities: vec![ability(
                "Shoot",
                AbilityArgs {
                    exhaust: true,
                    preds: preds(vec![p_unit_bf_v()]),
                    resolve: cait_res(),
                    can: Value::func("cait_can", |g, a| Ok(is_bf(g.o(a[1].obj()).loc).into())),
                    choices: Value::func("cait_choices", |g, a| {
                        let pid = pid_of(&a[0]);
                        let us = all_units(g, pid, true, None)?;
                        Ok(maps(tg_choices(g, &us)))
                    }),
                    ..Default::default()
                },
            )],
            ..Impl::new("Caitlyn, Patrolling")
        },
    );
    // Clockwork Keeper — "As you play me, you may pay [Calm] as an additional cost. If you do, draw 1."
    reg(
        r,
        Impl {
            as_played: calm_paid_as_played(),
            extra_cost_fn: calm_paid_extra(),
            on_play: Value::func("clockwork_on_play", |g, a| {
                let (o, ctx) = (a[0].obj(), a[1].map());
                if ctx.g("calm_paid").truthy() {
                    let c = g.o(o).ctrl;
                    g.draw(c, 1)?;
                }
                none()
            }),
            ..Impl::new("Clockwork Keeper")
        },
    );
    // Daisy! — "I enter ready. Reduce my cost by 1 energy for each of the following tags among your units — Bird,
    // Cat, Dog, and Poro. When I attack while your units have all 4 tags, [Stun] an enemy unit here."
    reg(
        r,
        Impl {
            enter_ready: Value::Bool(true),
            on_event: daisy(),
            cost_mod: Value::func("daisy_cost", |g, a| {
                let pid = pid_of(&a[0]);
                Ok(pair(pet_tags(g, pid)?.len() as i64, 0))
            }),
            ..Impl::new("Daisy!")
        },
    );
    // Eclipse Herald — "When you stun an enemy unit, ready me and give me +1 might this turn."
    reg(r, Impl { on_event: eclipse(), ..Impl::new("Eclipse Herald") });
    // Enthusiastic Promoter — "[Backline] When I hold, [Buff] all units here."
    reg(r, Impl { kw: kwm(&[("Backline", 1)]), on_event: promoter(), ..Impl::new("Enthusiastic Promoter") });
    // Field Musicians — "When you play me, give a unit +3 might this turn."
    reg(r, Impl { on_play: mod_trigger("Field Musicians", 3), ..Impl::new("Field Musicians") });
    // Whiteflame Protector — "When you play me, give a unit +8 might this turn."
    reg(r, Impl { on_play: mod_trigger("Whiteflame Protector", 8), ..Impl::new("Whiteflame Protector") });
    // Frisky Hunter — "When you play me, play a 1 might Bird unit token with [Deflect] here."
    reg(r, Impl { on_play: frisky(), ..Impl::new("Frisky Hunter") });
    // Frostcoat Mother — "[Empower] [12]. This ability costs [1] less for each rune you control. [Empowered] I have
    // +3 might."
    {
        let mut im = Impl { might_if: vec![(when_empowered(), 3)], ..Impl::new("Frostcoat Mother") };
        impl_empower(
            &mut im,
            Cost::Fn(Value::func("frostcoat_empower", |g, a| {
                let pid = pid_of(&a[0]);
                Ok(cost_tuple(0i64.max(12 - g.p[pid].runes.len() as i64), &[]))
            })),
        );
        reg(r, im);
    }
    // Steel Paws — "[Deflect] [Empower] [7] [Empowered] I have +7 might."
    {
        let mut im = Impl { kw: kwm(&[("Deflect", 1)]), might_if: vec![(when_empowered(), 7)], ..Impl::new("Steel Paws") };
        impl_empower(&mut im, Cost::from("7 energy"));
        reg(r, im);
    }
    // Serene Ascetic — "[Empower] [3] [Empowered] I have [Deflect] and [Shield 3]."
    {
        let mut im =
            Impl { kw_if: vec![(when_empowered(), kwm(&[("Deflect", 1), ("Shield", 3)]))], ..Impl::new("Serene Ascetic") };
        impl_empower(&mut im, Cost::from("3 energy"));
        reg(r, im);
    }
    // Nasus, Ascended — "[Deflect 2] [Empower] [8] [Empowered] When I conquer, you score 1 point."
    {
        let mut im = Impl { kw: kwm(&[("Deflect", 2)]), on_event: nasus(), ..Impl::new("Nasus, Ascended") };
        impl_empower(&mut im, Cost::from("8 energy"));
        reg(r, im);
    }
    // Guardian of the Passage — "When I hold, you may return a unit or gear from your trash to your hand."
    reg(r, Impl { on_event: passage(), ..Impl::new("Guardian of the Passage") });
    // Herald of Spring — "[Hunt] When you play me, gain 2 XP."
    reg(
        r,
        Impl {
            kw: kwm(&[("Hunt", 1)]),
            on_play: Value::func("herald_on_play", |g, a| {
                let o = a[0].obj();
                let res = Value::func("herald.res", |g, a| {
                    let it = a[0].item();
                    let c = ctrl(g, it);
                    g.gain_xp(c, 2)?;
                    none()
                });
                let (c, u) = (g.o(o).ctrl, uid(g, o));
                trig(g, c, "Herald of Spring", res, Map::new(), Some(u), false, Value::None, Value::None);
                none()
            }),
            ..Impl::new("Herald of Spring")
        },
    );
    // Iascylla — "When I hold, at the start of your next Main Phase, you may move an enemy unit to this battlefield."
    reg(r, Impl { on_event: iascylla(), ..Impl::new("Iascylla") });
    // Ivern, Nurturer — "When you play me or when I hold, look at the top 3 cards of your Main Deck. You may reveal a
    // unit from among them and draw it. Recycle the rest. Then if you revealed a Bird, Cat, Dog, or Poro, do this:
    // [Buff] a friendly unit."
    reg(
        r,
        Impl {
            on_play: play_trigger("Ivern, Nurturer", ivern_res),
            on_event: hold_trigger("Ivern, Nurturer", ivern_res),
            ..Impl::new("Ivern, Nurturer")
        },
    );
    // Ornn, Blacksmith — "When you play me or when I hold, look at the top 4 cards of your Main Deck. You may reveal
    // a gear from among them and draw it. Then recycle the rest."
    reg(
        r,
        Impl {
            on_play: play_trigger("Ornn, Blacksmith", ornn_res),
            on_event: hold_trigger("Ornn, Blacksmith", ornn_res),
            ..Impl::new("Ornn, Blacksmith")
        },
    );
    // Janna, Savior — "[Reaction] When you play me, heal your units here, then move an enemy unit from here to its
    // base."
    reg(r, Impl { timing: Some("reaction"), on_play: janna(), ..Impl::new("Janna, Savior") });
    // Leona, Zealot — "If an opponent's score is within 3 points of the Victory Score, I enter ready. Stunned enemy
    // units here have -8 might, to a minimum of 1 might."
    reg(
        r,
        Impl {
            enter_ready: Value::func("leona_enter_ready", |g, a| Ok(near_victory(g, pid_of(&a[0])).into())),
            aura_might: leona_aura(),
            ..Impl::new("Leona, Zealot")
        },
    );
    // Master Yi, Meditative — "While you have 8+ runes, I have +4 might."
    reg(
        r,
        Impl {
            might_if: vec![(
                Value::func("yi_runes", |g, a| {
                    let o = a[0].obj();
                    Ok((g.p[g.o(o).ctrl].runes.len() >= 8).into())
                }),
                4,
            )],
            ..Impl::new("Master Yi, Meditative")
        },
    );
    // Master Yi, Unstoppable — "[Level 3] I cost [2][Calm] less. [Level 6] I cost [4][Calm][Calm] less instead.
    // [Level 11] I cost [6][Calm][Calm][Calm] less instead. [Level 16] I can't be chosen by enemy spells and
    // abilities."
    reg(
        r,
        Impl {
            cost_mod: yi_cost(),
            untargetable: Value::func("yi_untargetable", |g, a| {
                let o = a[0].obj();
                let c = g.o(o).ctrl;
                Ok((a[1].opt_int() != Some(c as i64) && g.p[c].xp >= 16).into())
            }),
            ..Impl::new("Master Yi, Unstoppable")
        },
    );
    // Monch — "If an opponent controls a stunned unit, I cost [2] less and enter ready."
    reg(
        r,
        Impl {
            cost_mod: Value::func("monch_cost", |g, a| {
                Ok(if opp_stunned(g, pid_of(&a[0])) { pair(2, 0) } else { pair(0, 0) })
            }),
            enter_ready: Value::func("monch_ready", |g, a| Ok(opp_stunned(g, pid_of(&a[0])).into())),
            ..Impl::new("Monch")
        },
    );
    // Mosstomper — "[Hunt 2] [Level 3] I have +1 might and [Deflect]."
    reg(
        r,
        Impl {
            kw: kwm(&[("Hunt", 2)]),
            levels: vec![(3, map! { might = 1, kw = kwm(&[("Deflect", 1)]) })],
            ..Impl::new("Mosstomper")
        },
    );
    // Nami, Headstrong — "You may pay [Calm] as an additional cost to play me. When you play me, if you paid the
    // additional cost, [Stun] an enemy unit. When I hold, the next time you play a unit this turn, ready it and
    // [Buff] it."
    reg(
        r,
        Impl {
            on_play: nami_play(),
            on_event: nami_event(),
            as_played: calm_paid_as_played(),
            extra_cost_fn: calm_paid_extra(),
            ..Impl::new("Nami, Headstrong")
        },
    );
    // Ol' Poro — "I can't be played on your first, second, or third turns."
    reg(
        r,
        Impl {
            as_played: Value::func("olporo_as_played", |g, a| {
                Ok(if g.p[pid_of(&a[0])].turns > 3 { maps(vec![Map::new()]) } else { maps(vec![]) })
            }),
            ..Impl::new("Ol' Poro")
        },
    );
    // Pakaa Protector — "When I move, reveal the top card of your Main Deck. If it's a unit, draw it. Otherwise, put
    // it in your trash and give me +2 might this turn."
    reg(
        r,
        Impl {
            on_event: Value::func("pakaa_event", |g, a| {
                let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
                if ev == "move" && same(&info.g("obj"), o) {
                    let (c, u, od) = (g.o(o).ctrl, uid(g, o), oid(g, o));
                    trig(g, c, "Pakaa Protector", pakaa_res(), map! { oid = od }, Some(u), false, Value::None, Value::None);
                }
                none()
            }),
            ..Impl::new("Pakaa Protector")
        },
    );
    // Poro Herder — "When you play me, if you control a Poro, buff me and draw 1."
    reg(
        r,
        Impl {
            on_play: Value::func("herder_on_play", |g, a| {
                let o = a[0].obj();
                let c = g.o(o).ctrl;
                if has_poro(g, c)? {
                    let (u, od) = (uid(g, o), oid(g, o));
                    trig(g, c, "Poro Herder", herder_res(), map! { oid = od }, Some(u), false, Value::None, Value::None);
                }
                none()
            }),
            ..Impl::new("Poro Herder")
        },
    );
    // Legion Quartermaster — "As an additional cost to play me, return a friendly gear to its owner's hand."
    reg(
        r,
        Impl {
            pay_extra: qm_pay(),
            as_played: Value::func("qm_as_played", |g, a| {
                let gs = qm_gear(g, pid_of(&a[0]));
                Ok(maps(gs.iter().map(|&x| map! { ret_gear = uid(g, x) }).collect()))
            }),
            ..Impl::new("Legion Quartermaster")
        },
    );
    // Ribbon Dancer — "When I move to a battlefield, give another friendly unit +1 might this turn."
    reg(r, Impl { on_event: ribbon(), ..Impl::new("Ribbon Dancer") });
    // Riven, Shattered — "[Weaponmaster] When I attack, choose an enemy unit here. Deal 2 to it for each Equipment
    // attached to me."
    reg(r, Impl { kw: kwm(&[("Weaponmaster", 1)]), on_event: riven(), ..Impl::new("Riven, Shattered") });
    // Royal Entourage — "When you play me, ready or exhaust a legend."
    reg(r, Impl { on_play: entourage(), ..Impl::new("Royal Entourage") });
    // Shadow — "If you play me to a battlefield, I enter ready. [Action][>] [1][A], [E]: [Stun] an enemy unit
    // attacking here."
    reg(
        r,
        Impl {
            enter_ready: Value::func("shadow_ready", |_g, a| Ok(is_bf(a[2].map().g("loc").opt_loc()).into())),
            abilities: vec![ability(
                "Stun",
                AbilityArgs {
                    cost: Cost::from("1 energy and 1 rune of any type"),
                    timing: "action",
                    exhaust: true,
                    preds: preds(vec![shadow_pred()]),
                    resolve: stun_legal0("shadow_stun"),
                    choices: Value::func("shadow_choices", |g, a| {
                        let (pid, o) = (pid_of(&a[0]), a[1].obj());
                        let us: Vec<O> = enemies(g, pid, false, None, true)?
                            .into_iter()
                            .filter(|&u| g.o(u).desig == Some(Desig::Att) && g.o(u).loc == g.o(o).loc && !g.o(u).stunned)
                            .collect();
                        Ok(maps(tg_choices(g, &us)))
                    }),
                    ..Default::default()
                },
            )],
            ..Impl::new("Shadow")
        },
    );
    // Shen, Scourge of Shadows — "When I hold, if there is exactly one other unit you control here, draw 1."
    reg(r, Impl { on_event: shen(), ..Impl::new("Shen, Scourge of Shadows") });
    // Simian Ancestor — "When you buff me, ready me." (l'évènement 'buff' nomme le joueur qui buffe : by)
    reg(r, Impl { on_event: simian(), ..Impl::new("Simian Ancestor") });
    // Solari Shieldbearer — "When you play me, stun a unit."
    reg(r, Impl { on_play: shieldbearer(), ..Impl::new("Solari Shieldbearer") });
    // Sona, Harmonious — "While I'm at a battlefield, ready 4 friendly runes at the end of your turn."
    reg(r, Impl { on_event: sona(), ..Impl::new("Sona, Harmonious") });
    // Taric, Protector — "[Shield] [Tank] Other friendly units here have [Shield]."
    reg(r, Impl { kw: kwm(&[("Shield", 1), ("Tank", 1)]), aura_kw: taric_aura(), ..Impl::new("Taric, Protector") });
    // Tasty Faefolk — "[Accelerate] [Deathknell] Channel 2 runes exhausted and draw 1."
    reg(
        r,
        Impl {
            accelerate: true,
            deathknell: Value::func("faefolk_dk", |g, a| {
                let it = a[0].item();
                let c = ctrl(g, it);
                g.channel(c, 2, true)?;
                g.draw(c, 1)?;
                none()
            }),
            ..Impl::new("Tasty Faefolk")
        },
    );
    // Trevor Snoozebottom — "[Shield] When I hold, play a ready 3 might Sprite unit token with [Temporary] here."
    reg(r, Impl { kw: kwm(&[("Shield", 1)]), on_event: trevor(), ..Impl::new("Trevor Snoozebottom") });
    // Vex, Mocking — "[Shield] [Tank] When you [Stun] an enemy unit at a battlefield, you may move me to that
    // battlefield."
    reg(r, Impl { kw: kwm(&[("Shield", 1), ("Tank", 1)]), on_event: vex(), ..Impl::new("Vex, Mocking") });
    // Wielder of Water — "While I'm attacking or defending alone, I have +2 might."
    reg(
        r,
        Impl {
            might_mod: Value::func("wielder_might", |g, a| {
                let o = a[0].obj();
                Ok(Value::Int(if g.o(o).desig.is_some() && g.alone(o, None) { 2 } else { 0 }))
            }),
            ..Impl::new("Wielder of Water")
        },
    );
    // Wizened Elder — "While I'm buffed, I have an additional +1 might."
    reg(
        r,
        Impl {
            might_mod: Value::func("elder_might", |g, a| Ok(Value::Int(if g.o(a[0].obj()).buff > 0 { 1 } else { 0 }))),
            ..Impl::new("Wizened Elder")
        },
    );
    // Wuju Apprentice — "[Hunt] [Level 6] When you play me, draw 1."
    reg(
        r,
        Impl {
            kw: kwm(&[("Hunt", 1)]),
            on_play: Value::func("wuju_on_play", |g, a| {
                let o = a[0].obj();
                let c = g.o(o).ctrl;
                if g.level(c, 6) {
                    let u = uid(g, o);
                    trig(g, c, "Wuju Apprentice", draw1("wuju.res"), Map::new(), Some(u), false, Value::None, Value::None);
                }
                none()
            }),
            ..Impl::new("Wuju Apprentice")
        },
    );
    // Yasuo, Remorseful — "When I attack, deal damage equal to my Might to an enemy unit here."
    reg(r, Impl { on_event: yasuo(), ..Impl::new("Yasuo, Remorseful") });
    // Yuumi, Magical Cat — "[Tank] When I attack or defend, give one of your other units here +3 might and [Tank] this
    // turn."
    reg(r, Impl { kw: kwm(&[("Tank", 1)]), on_event: yuumi(), ..Impl::new("Yuumi, Magical Cat") });

    // ---------------------------------------------------------------- gear and equipment
    // (Might Bonus et Effect Text des Equipment lus sur les images des cartes, voir GUIDE.md section 2)
    // Brutalizer — "[Equip] [Calm]" ; Might Bonus +1 ; Effect Text: "If this was attached to me this turn, I have an
    // additional +2 might."
    {
        let mut im = Impl { bonus: Some(1), on_event: brut_event(), aura_might: brut_aura(), ..Impl::new("Brutalizer") };
        impl_equip(&mut im, Cost::from("1 calm rune"));
        reg(r, im);
    }
    // Doran's Shield — "[Equip] [Calm]" ; Might Bonus +1 ; Effect Text: "[Tank]".
    {
        let mut im = Impl { bonus: Some(1), equip_kw: Some(kwm(&[("Tank", 1)])), ..Impl::new("Doran's Shield") };
        impl_equip(&mut im, Cost::from("1 calm rune"));
        reg(r, im);
    }
    // Hand Hammer — "[Equip] [Calm]" ; Might Bonus +1 ; Effect Text: "I have +2 might while I'm at a battlefield with
    // exactly one other unit you control."
    {
        let mut im = Impl { bonus: Some(1), aura_might: hammer_aura(), ..Impl::new("Hand Hammer") };
        impl_equip(&mut im, Cost::from("1 calm rune"));
        reg(r, im);
    }
    // Soul Sword — "[Equip] [Calm]" ; Might Bonus +1 ; Effect Text: "[Level 3] I have an additional +1 might."
    {
        let mut im = Impl {
            bonus: Some(1),
            aura_might: Value::func("soul_sword_aura", |g, a| {
                let (src, o) = (a[0].obj(), a[1].obj());
                Ok(Value::Int(
                    if g.o(src).attached_to == Some(uid(g, o)) && g.on_board(src) && g.p[g.o(src).ctrl].xp >= 3 { 1 } else { 0 },
                ))
            }),
            ..Impl::new("Soul Sword")
        };
        impl_equip(&mut im, Cost::from("1 calm rune"));
        reg(r, im);
    }
    // Forgefire Cape — "[Unique] [Equip] [A]" ; Might Bonus +3 ; Effect Text: "When I attack or defend, deal 2 to all
    // enemy units here."
    {
        let mut im = Impl { bonus: Some(3), effect_event: cape_effect(), ..Impl::new("Forgefire Cape") };
        impl_equip(&mut im, Cost::from("1 rune of any type"));
        reg(r, im);
    }
    // Shurelya's Requiem — "[Unique] [Equip] [A] When you play this, ready your units." ; Might Bonus +2 ;
    // Effect Text: "Your units here have [Ganking]."
    {
        let mut im = Impl {
            bonus: Some(2),
            aura_kw: shurelya_aura(),
            on_play: Value::func("shurelya_on_play", |g, a| {
                let o = a[0].obj();
                let res = Value::func("shurelya.res", |g, a| {
                    let it = a[0].item();
                    let c = ctrl(g, it);
                    for u in g.units(Some(c), None) {
                        g.ready_obj(u, None, None)?;
                    }
                    none()
                });
                let (c, u) = (g.o(o).ctrl, uid(g, o));
                trig(g, c, "Shurelya's Requiem", res, Map::new(), Some(u), false, Value::None, Value::None);
                none()
            }),
            ..Impl::new("Shurelya's Requiem")
        };
        impl_equip(&mut im, Cost::from("1 rune of any type"));
        reg(r, im);
    }
    // Heart of Dark Ice — "[E]: Give a unit +3 might this turn."
    reg(
        r,
        Impl {
            abilities: vec![ability(
                "Chill",
                AbilityArgs {
                    exhaust: true,
                    preds: preds(vec![p_unit_v()]),
                    resolve: Value::func("chill_res", |g, a| {
                        mod_legal0(g, a[0].item(), 3)?;
                        none()
                    }),
                    choices: Value::func("chill_choices", |g, a| {
                        let us = units_first(g, pid_of(&a[0]), None)?;
                        Ok(maps(tg_choices(g, &us)))
                    }),
                    ..Default::default()
                },
            )],
            ..Impl::new("Heart of Dark Ice")
        },
    );
    // Honeyfruit — "This enters exhausted. [Reaction][>] [E]: [Add] [A]. [Level 6][>] [Reaction][>] [E]: [Add] [1][A]."
    // ([Level 6] remplace la première capacité : une seule des deux existe à la fois)
    reg(
        r,
        Impl {
            enter_exhausted: true,
            add: vec![
                add_ability(
                    "1 rune of any type",
                    true,
                    false,
                    Value::func("honeyfruit_can1", |g, a| Ok((g.p[pid_of(&a[0])].xp < 6).into())),
                ),
                add_ability(
                    "1 energy and 1 rune of any type",
                    true,
                    false,
                    Value::func("honeyfruit_can2", |g, a| Ok((g.p[pid_of(&a[0])].xp >= 6).into())),
                ),
            ],
            ..Impl::new("Honeyfruit")
        },
    );
    // Mask of Foresight — "When a friendly unit attacks or defends alone, give it +1 might this turn."
    reg(r, Impl { on_event: mask(), ..Impl::new("Mask of Foresight") });
    // Poro Snax — "When you play this, draw 1. [1][Calm], [E], Kill this: Draw 1."
    reg(
        r,
        Impl {
            on_play: Value::func("snax_on_play", |g, a| {
                let o = a[0].obj();
                let (c, u) = (g.o(o).ctrl, uid(g, o));
                trig(g, c, "Poro Snax", draw1("snax.res"), Map::new(), Some(u), false, Value::None, Value::None);
                none()
            }),
            abilities: vec![ability(
                "Snack",
                AbilityArgs {
                    cost: Cost::from("1 energy and 1 calm rune"),
                    exhaust: true,
                    kill_self: true,
                    resolve: draw1("snack_res"),
                    ..Default::default()
                },
            )],
            ..Impl::new("Poro Snax")
        },
    );
    // Seal of Focus — "[E]: [Reaction] — [Add] [Calm]."
    reg(r, Impl { add: vec![add_ability("1 calm rune", true, false, Value::None)], ..Impl::new("Seal of Focus") });
    // Spirit's Refuge — "When you play this, buff a friendly unit. Friendly buffed units have [Deflect] if they
    // didn't already."
    reg(r, Impl { on_play: refuge_play(), aura_kw: refuge_aura(), ..Impl::new("Spirit's Refuge") });
    // Forgotten Signpost — "[Action][>] Exhaust a unit you control, [E]: Move a different unit you control to the
    // location of the unit you exhausted to pay for this ability."
    reg(
        r,
        Impl {
            abilities: vec![ability(
                "Signpost",
                AbilityArgs {
                    timing: "action",
                    exhaust: true,
                    preds: preds(vec![p_friend_v()]),
                    choices: signpost_choices(),
                    extra_cost: signpost_pay(),
                    resolve: signpost_res(),
                    ..Default::default()
                },
            )],
            ..Impl::new("Forgotten Signpost")
        },
    );

    // ---------------------------------------------------------------- spells
    // Alpha Strike — "[Action] Choose a friendly unit. It deals damage equal to its Might split among enemy units at
    // battlefields. Then for each unit this kills, do this: Gain 1 XP."  (répartition : règle 355.14)
    reg(
        r,
        Impl {
            timing: Some("action"),
            preds: preds(vec![p_friend_v(), p_enemy_bf_v()]),
            resolve: res_fn("_alpha", alpha),
            choices: alpha_choices(),
            ..Impl::new("Alpha Strike")
        },
    );
    // Arise! — "Play a 2 might Sand Soldier unit token for each Equipment you control. Then ready two of them."
    reg(r, Impl { resolve: res_fn("_arise", arise), ..Impl::new("Arise!") });
    // Combat Experience — "[Reaction] Give a unit +1 might this turn. [Level 6][>] Give it +3 might this turn
    // instead."
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            preds: preds(vec![p_unit_v()]),
            resolve: res_fn("combat_exp", combat_exp),
            choices: units_first_choices(),
            ..Impl::new("Combat Experience")
        },
    );
    // Defiant Dance — "[Reaction] Give a unit +2 might this turn and another unit -2 might this turn."
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            preds: preds(vec![p_unit_v(), p_unit_v()]),
            resolve: res_fn("_dance", dance),
            choices: dance_choices(),
            ..Impl::new("Defiant Dance")
        },
    );
    // Desert's Call — "[Repeat] [2] Play a 2 might Sand Soldier unit token."
    reg(
        r,
        Impl {
            repeat: repeat_cost("2 energy"),
            resolve: repeatable(Value::func("desert_call", |g, a| {
                let c = ctrl(g, a[0].item());
                play_token(g, "Sand Soldier", c, None, false)?;
                none()
            })),
            ..Impl::new("Desert's Call")
        },
    );
    // Double Trouble — "[Repeat] [2] Look at the top 3 cards of your Main Deck. You may reveal a unit from among them
    // and draw it. Recycle the rest."
    reg(
        r,
        Impl {
            repeat: repeat_cost("2 energy"),
            resolve: repeatable(Value::func("double_trouble", |g, a| {
                let c = ctrl(g, a[0].item());
                look_pick(g, c, 3, is_unit_card, "double_pick")?;
                none()
            })),
            ..Impl::new("Double Trouble")
        },
    );
    // Dragon's Rage — "Move an enemy unit. Then choose another enemy unit at its destination. They deal damage equal
    // to their Mights to each other."  (les deux unités ennemies sont choisies au moment de jouer, règle 355.5)
    reg(
        r,
        Impl {
            preds: preds(vec![p_enemy_v(), p_enemy_v()]),
            resolve: res_fn("_rage", rage),
            choices: rage_choices(),
            ..Impl::new("Dragon's Rage")
        },
    );
    // Emperor's Divide — "[Hidden] [Action] Move any number of friendly units at a battlefield to their base."
    reg(
        r,
        Impl {
            timing: Some("action"),
            hidden: true,
            preds: preds(vec![p_friend_bf_v()]),
            resolve: res_fn("_divide", divide),
            choices: divide_choices(),
            ..Impl::new("Emperor's Divide")
        },
    );
    // Feral Strength — "[Reaction] [Repeat] [2] Give a unit +2 might this turn."
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            repeat: repeat_cost("2 energy"),
            preds: preds(vec![p_unit_v()]),
            resolve: repeatable(Value::func("feral_strength", |g, a| {
                mod_legal0(g, a[0].item(), 2)?;
                none()
            })),
            choices: units_first_choices(),
            ..Impl::new("Feral Strength")
        },
    );
    // Find Your Center — "[Action] If an opponent's score is within 3 points of the Victory Score, this costs [2]
    // less. Draw 1 and channel 1 rune exhausted."
    reg(
        r,
        Impl {
            timing: Some("action"),
            cost_mod: Value::func("center_cost", |g, a| {
                Ok(if near_victory(g, pid_of(&a[0])) { pair(2, 0) } else { pair(0, 0) })
            }),
            resolve: Value::func("center_res", |g, a| {
                let c = ctrl(g, a[0].item());
                g.draw(c, 1)?;
                g.channel(c, 1, true)?;
                none()
            }),
            ..Impl::new("Find Your Center")
        },
    );
    // Flurry of Feathers — "[Reaction] Choose one — Counter a spell. / Play four 1 might Bird unit tokens with
    // [Deflect]."
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            resolve: res_fn("_flurry", flurry),
            choices: flurry_choices(),
            ..Impl::new("Flurry of Feathers")
        },
    );
    // Fox-Fire — "[Hidden] [Action] Kill any number of units at a battlefield with total Might 4 or less."
    reg(
        r,
        Impl {
            timing: Some("action"),
            hidden: true,
            preds: preds(vec![p_unit_v()]),
            resolve: res_fn("_foxfire", foxfire),
            choices: foxfire_choices(),
            ..Impl::new("Fox-Fire")
        },
    );
    // Friendship — "[Reaction] Choose a unit. Give it +1 might this turn for each of the following tags among your
    // units — Bird, Cat, Dog, and Poro."
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            preds: preds(vec![p_unit_v()]),
            resolve: res_fn("friendship", friendship),
            choices: units_first_choices(),
            ..Impl::new("Friendship")
        },
    );
    // Last Breath — "[Action] Ready a friendly unit. It deals damage equal to its Might to an enemy unit at a
    // battlefield."
    reg(
        r,
        Impl {
            timing: Some("action"),
            preds: preds(vec![p_friend_v(), p_enemy_bf_v()]),
            resolve: res_fn("_last_breath", last_breath),
            choices: lb_choices(),
            ..Impl::new("Last Breath")
        },
    );
    // Last Stand — "[Action] Double a friendly unit's Might this turn. Give it [Temporary]."
    reg(
        r,
        Impl {
            timing: Some("action"),
            preds: preds(vec![p_friend_v()]),
            resolve: res_fn("_last_stand", last_stand),
            choices: Value::func("last_stand_choices", |g, a| {
                let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
                let fr = friends(g, pid, false, hb)?;
                Ok(maps(tg_choices(g, &fr)))
            }),
            ..Impl::new("Last Stand")
        },
    );
    // Meditation — "[Reaction] As an additional cost to play this, you may exhaust a friendly unit. If you do, draw
    // 2. Otherwise, draw 1."
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            pay_extra: med_pay(),
            resolve: Value::func("med_res", |g, a| {
                let it = a[0].item();
                let n = if data(g, it, "exh").truthy() { 2 } else { 1 };
                let c = ctrl(g, it);
                g.draw(c, n)?;
                none()
            }),
            choices: med_choices(),
            ..Impl::new("Meditation")
        },
    );
    // Party Favors — "Each other player chooses Cards or Runes. For each player that chooses Cards, you and that
    // player each draw 1. For each player that chooses Runes, you and that player each channel 1 rune exhausted."
    reg(r, Impl { resolve: res_fn("_favors", favors), ..Impl::new("Party Favors") });
    // Reinforce — "Look at the top 5 cards of your Main Deck. You may banish a unit from among them, then play it,
    // reducing its cost by [5]. Recycle the remaining cards."
    reg(r, Impl { resolve: res_fn("_reinforce", reinforce), ..Impl::new("Reinforce") });
    // Resonating Strike — "[Hidden] [Reaction] Choose a battlefield you control and a unit you control at a different
    // location. Move that unit to that battlefield and give it +2 might this turn."
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            hidden: true,
            preds: preds(vec![p_friend_v()]),
            resolve: res_fn("_resonating", resonating),
            choices: resonating_choices(),
            ..Impl::new("Resonating Strike")
        },
    );
    // Rune Prison — "[Action] Stun a unit."
    reg(
        r,
        Impl {
            timing: Some("action"),
            preds: preds(vec![p_unit_v()]),
            resolve: stun_legal0("rune_prison_res"),
            choices: rune_prison_choices(),
            ..Impl::new("Rune Prison")
        },
    );
    // Sanction — "[Reaction] Choose one — Empower a unit. Disempower it at end of turn. / Disempower a unit that's
    // [Empowered]. Empower it at end of turn."
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            resolve: res_fn("_sanction", sanction),
            choices: sanction_choices(),
            preds: preds(vec![sanction_pred()]),
            ..Impl::new("Sanction")
        },
    );
    // Shadow Dash — "Move an enemy unit to a battlefield where you have units. If you have exactly two units there,
    // they each get +1 might this turn. [Flow] [5][A][A]"
    reg(
        r,
        Impl {
            preds: preds(vec![p_enemy_v()]),
            resolve: res_fn("_dash", dash),
            choices: dash_choices(),
            flow: flow_cost("5 energy and 2 runes of any type"),
            ..Impl::new("Shadow Dash")
        },
    );
    // Siphoning Strike — "Deal 4 to a unit at a battlefield. If you control 7 or more runes, deal 7 to it instead.
    // When it dies this turn, channel 1 rune exhausted."
    reg(
        r,
        Impl {
            preds: preds(vec![p_unit_bf_v()]),
            resolve: res_fn("_siphon", siphon),
            choices: enemies_bf_choices(),
            ..Impl::new("Siphoning Strike")
        },
    );
    // Skyward Strike — "Move an enemy unit. [Level 6][>] [Stun] an enemy unit."
    reg(
        r,
        Impl {
            preds: preds(vec![p_enemy_v(), p_enemy_v()]),
            resolve: res_fn("_skyward", skyward),
            choices: skyward_choices(),
            ..Impl::new("Skyward Strike")
        },
    );
    // Thwonk! — "[Action] [Repeat] [2] Stun an attacking unit."
    reg(
        r,
        Impl {
            timing: Some("action"),
            repeat: repeat_cost("2 energy"),
            preds: preds(vec![thwonk_pred()]),
            resolve: repeatable(stun_legal0("thwonk_res")),
            choices: thwonk_choices(),
            ..Impl::new("Thwonk!")
        },
    );
    // Tricksy Tentacles — "Move any number of enemy units with the same controller and a total Might of 8 or less to
    // a single location."  (ciblage de groupe, règle 355.11)
    reg(
        r,
        Impl {
            preds: preds(vec![p_enemy_v()]),
            resolve: res_fn("_tricksy", tricksy),
            choices: tricksy_choices(),
            ..Impl::new("Tricksy Tentacles")
        },
    );
    // Wind Wall — "[Reaction] Counter a spell."
    reg(
        r,
        Impl { timing: Some("reaction"), resolve: counter_spell(), choices: wind_wall_choices(), ..Impl::new("Wind Wall") },
    );
    // Zenith Blade — "[Action] Stun an enemy unit at a battlefield. You may move a friendly unit to that enemy unit's
    // battlefield."
    reg(
        r,
        Impl {
            timing: Some("action"),
            preds: preds(vec![p_enemy_bf_v(), p_friend_v()]),
            resolve: res_fn("_zenith", zenith),
            choices: zenith_choices(),
            ..Impl::new("Zenith Blade")
        },
    );

    // ---------------------------------------------------------------- cartes qui utilisent les crochets du moteur
    r.ask_text(&[
        ("solari_shrine", "Solari Shrine : épuiser pour piocher 1 ?"),
        ("new_choices", "Faire de nouveaux choix pour ce sort ?"),
    ]);
    // Alpha Wildclaw — "[Tank] Your units here with less Might than me can't be chosen by enemy spells and abilities."
    // (une aura lue par Game.targetable, aussi quand les cibles sont revérifiées à la résolution, règle 758.2.a)
    {
        let mut im = Impl { kw: kwm(&[("Tank", 1)]), ..Impl::new("Alpha Wildclaw") };
        im.hooks.set("aura_untargetable", wildclaw_aura());
        reg(r, im);
    }
    // Twilight Shroud — "Give a friendly unit +1 might this turn. It can't be chosen by enemy spells and abilities
    // this turn. [Flow] 2 energy"
    reg(
        r,
        Impl {
            flow: flow_cost("2 energy"),
            preds: preds(vec![p_friend_v()]),
            resolve: res_fn("_shroud", shroud),
            choices: units_first_choices(),
            ..Impl::new("Twilight Shroud")
        },
    );
    // Counter Strike — "[Reaction] Choose a unit. The next time that unit would be dealt damage this turn, prevent
    // it. Draw 1." (un effet 'prevent_next', consommé par les premiers dégâts d'un sort, d'une capacité ou d'un
    // combat ; l'unité reçoit quand même des dégâts de combat mortels assignés normalement, règle 465.2.c.10)
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            preds: preds(vec![p_unit_v()]),
            resolve: res_fn("_counter_strike", counter_strike),
            choices: counter_strike_choices(),
            ..Impl::new("Counter Strike")
        },
    );
    // Esteemed Hierophant — "While you control 7 or more runes, prevent all damage that enemy spells and abilities
    // would deal to me."
    {
        let mut im = Impl::new("Esteemed Hierophant");
        im.hooks.set(
            "dmg_prevent",
            Value::func("hierophant_prevent", |g, a| {
                let (o, ctx) = (a[0].obj(), a[1].map());
                let k = ctx.g("kind");
                let by = ctx.g("by");
                let c = g.o(o).ctrl;
                Ok((matches!(k.opt_str(), Some("spell" | "ability" | "trigger"))
                    && !by.is_none()
                    && by.opt_int() != Some(c as i64)
                    && g.p[c].runes.len() >= 7)
                    .into())
            }),
        );
        reg(r, im);
    }
    // Rabadon's Deathcrown — "[Unique] [Equip] [A]". Might Bonus +3, Effect Text (image de la carte) : "Your spells and
    // abilities deal 3 Bonus Damage." (Bonus Damage, règles 712-715 : chaque instance de leurs dégâts augmente de 3)
    {
        let mut im = Impl { bonus: Some(3), ..Impl::new("Rabadon's Deathcrown") };
        impl_equip(&mut im, Cost::from("1 rune of any type"));
        im.hooks.set(
            "fx_dmg_bonus",
            Value::func("rabadon_bonus", |g, a| {
                let (u, ctx) = (a[1].obj(), a[2].map());
                let ok = matches!(ctx.g("kind").opt_str(), Some("spell" | "ability" | "trigger"))
                    && ctx.g("by").opt_int() == Some(g.o(u).ctrl as i64);
                Ok(Value::Int(if ok { 3 } else { 0 }))
            }),
        );
        reg(r, im);
    }
    // Highlander — "[Reaction] Choose a friendly unit. The next time it dies this turn, recall it exhausted instead."
    // (pas de soin : une unité encore marquée de dégâts mortels meurt de nouveau au nettoyage suivant)
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            preds: preds(vec![p_friend_v()]),
            resolve: res_fn("_highlander", highlander),
            choices: friends_by_value_choices(),
            ..Impl::new("Highlander")
        },
    );
    // Guardian Angel — "[Equip] [Calm]". Might Bonus +1, Effect Text (image de la carte) : "If I would die, kill
    // Guardian Angel instead. Heal me, exhaust me, and recall me." (un remplacement dans le texte de l'unité équipée,
    // règles 370-373)
    {
        let mut im = Impl { bonus: Some(1), ..Impl::new("Guardian Angel") };
        impl_equip(&mut im, Cost::from("1 calm rune"));
        im.hooks.set("fx_death_rep", angel_rep());
        reg(r, im);
    }
    // Vilemaw — "[Ambush] Enemy units here with less Might than me don't deal combat damage. When I hold, draw 1."
    {
        let mut im = Impl { ambush: true, on_event: vilemaw_event(), ..Impl::new("Vilemaw") };
        im.hooks.set(
            "aura_no_combat",
            Value::func("vilemaw_no_combat", |g, a| {
                let (src, o) = (a[0].obj(), a[1].obj());
                if !(g.o(o).ctrl != g.o(src).ctrl && g.o(o).loc == g.o(src).loc && is_bf(g.o(src).loc)) {
                    return Ok(false.into());
                }
                Ok((g.might(o)? < g.might(src)?).into())
            }),
        );
        reg(r, im);
    }
    // Helm of Suppression — "[Empower] 4 energy and 1 calm rune. Opponents' spells cost 1 energy more. If this is
    // [Empowered], they cost 1 energy and 1 rune of any type more instead."
    {
        let mut im = Impl::new("Helm of Suppression");
        impl_empower(&mut im, Cost::from("4 energy and 1 calm rune"));
        im.hooks.set("cost_aura", helm_tax());
        reg(r, im);
    }
    // Lilting Lullaby — "[Reaction] Counter a spell. Its controller can't play spells this turn."
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            resolve: res_fn("_lullaby", lullaby),
            choices: lullaby_choices(),
            ..Impl::new("Lilting Lullaby")
        },
    );
    // Mageseeker Warden — "While I'm at a battlefield, opponents can only play units to their base. While I'm at a
    // battlefield, spells and abilities can't ready enemy units and gear." (pas le réveil, une action du jeu)
    {
        let mut im = Impl::new("Mageseeker Warden");
        im.hooks.set(
            "loc_veto",
            Value::func("warden_loc_veto", |g, a| {
                let (src, pid, c, loc) = (a[0].obj(), pid_of(&a[1]), a[2].obj(), a[3].opt_loc());
                Ok((is_bf(g.o(src).loc) && pid != g.o(src).ctrl && is_unit(g, c) && loc != Some(Loc::Base)).into())
            }),
        );
        im.hooks.set(
            "aura_no_ready",
            Value::func("warden_no_ready", |g, a| {
                let (src, o) = (a[0].obj(), a[1].obj());
                Ok((is_bf(g.o(src).loc) && g.o(o).ctrl != g.o(src).ctrl && a[3].opt_str() != Some("awaken")).into())
            }),
        );
        reg(r, im);
    }
    // Jax, Unmatched — "[Deflect] Each Equipment in your hand has [Quick-Draw]."
    {
        let mut im = Impl { kw: kwm(&[("Deflect", 1)]), ..Impl::new("Jax, Unmatched") };
        im.hooks.set(
            "card_kw",
            Value::func("jax_card_kw", |g, a| {
                let (src, pid, c) = (a[0].obj(), pid_of(&a[1]), a[2].obj());
                Ok((a[3].opt_str() == Some("Quick-Draw")
                    && a[4].opt_str() == Some("hand")
                    && pid == g.o(src).ctrl
                    && g.spec(c).type_ == "Gear"
                    && g.spec(c).tags.contains("Equipment"))
                .into())
            }),
        );
        reg(r, im);
    }
    // Lee Sin, Ascetic — "[Shield] [E]: Buff me. I can have any number of buffs."
    reg(
        r,
        Impl {
            kw: kwm(&[("Shield", 1)]),
            multi_buff: true,
            abilities: vec![ability(
                "Buff",
                AbilityArgs {
                    exhaust: true,
                    resolve: Value::func("leesin_buff", |g, a| {
                        let it = a[0].item();
                        let me = g.it(it).src.and_then(|s| g.obj(s));
                        g.buff(me, None)?;
                        none()
                    }),
                    ..Default::default()
                },
            )],
            ..Impl::new("Lee Sin, Ascetic")
        },
    );
    // Stand United — "[Hidden] [Action] Buff a friendly unit. Buffs give an additional +1 might to friendly units
    // this turn." (un effet 'buff_bonus' lu par Game.might : chaque buff des unités du joueur, aussi les suivants)
    reg(
        r,
        Impl {
            hidden: true,
            timing: Some("action"),
            preds: preds(vec![p_friend_v()]),
            resolve: Value::func("_stand_united", |g, a| {
                let it = a[0].item();
                if let Some(u) = g.legal(it, 0)? {
                    g.buff(Some(u), None)?;
                }
                let mut d = Map::new();
                d.set("kind", "buff_bonus");
                d.set("pid", ctrl(g, it));
                d.set("dur", "turn");
                g.add_effect(d);
                none()
            }),
            choices: Value::func("stand_united_choices", |g, a| {
                let (pid, hb) = (pid_of(&a[0]), hb_of(a[1].map()));
                let fr = friends(g, pid, false, hb)?;
                let v = ksort(fr, |&u| Ok((g.o(u).buff > 0, -value(g, u)?, g.o(u).uid)))?;
                Ok(maps(tg_choices(g, &v)))
            }),
            ..Impl::new("Stand United")
        },
    );
    // Lillia, Protector of Dreams — "When you play a token unit, give me +1 might this turn. Your token units have
    // [Tank]."
    reg(
        r,
        Impl {
            on_event: lillia_event(),
            aura_kw: Value::func("lillia_aura", |g, a| {
                let (src, o) = (a[0].obj(), a[1].obj());
                Ok(if g.o(o).token
                    && g.o(o).ctrl == g.o(src).ctrl
                    && is_unit(g, o)
                    && is_unit(g, src)
                    && g.o(src).zone == Some(Zone::Board)
                {
                    map! { Tank = 1 }.into()
                } else {
                    Map::new().into()
                })
            }),
            ..Impl::new("Lillia, Protector of Dreams")
        },
    );
    // Sandstone Chimera — "While I'm at a battlefield, players only channel 1 rune at the start of their Channel
    // Phase."
    {
        let mut im = Impl::new("Sandstone Chimera");
        im.hooks.set(
            "channel_mod",
            Value::func("chimera_channel", |g, a| {
                let (src, n) = (a[0].obj(), a[2].int());
                Ok(Value::Int(if is_bf(g.o(src).loc) { n.min(1) } else { n }))
            }),
        );
        reg(r, im);
    }
    // Tianna Crownguard — "[Deflect] While I'm at a battlefield, opponents can't score points."
    {
        let mut im = Impl { kw: kwm(&[("Deflect", 1)]), ..Impl::new("Tianna Crownguard") };
        im.hooks.set(
            "score_veto",
            Value::func("tianna_veto", |g, a| {
                let (src, pid) = (a[0].obj(), pid_of(&a[1]));
                Ok((is_bf(g.o(src).loc) && pid != g.o(src).ctrl).into())
            }),
        );
        reg(r, im);
    }
    // Needlessly Large Yordle — "[Shield 5] [Tank] I cost 2 energy and 1 calm rune less for each point you scored
    // from holding this turn." (Game.hist['hold_pts'])
    reg(
        r,
        Impl {
            kw: kwm(&[("Shield", 5), ("Tank", 1)]),
            cost_mod: Value::func("yordle_cost", |g, a| {
                let h = g.hist.hold_pts[pid_of(&a[0])];
                Ok(pair(2 * h, h))
            }),
            ..Impl::new("Needlessly Large Yordle")
        },
    );
    // Shadow Watcher — "If a friendly unit died during your Beginning Phase this turn, I enter ready." (morts notées
    // avec l'étape du tour où elles ont eu lieu : Game.hist['died'], phases 'beginning' et 'scoring')
    reg(
        r,
        Impl {
            enter_ready: Value::func("watcher_ready", |g, a| {
                let pid = pid_of(&a[0]) as i64;
                Ok(g.hist
                    .died
                    .iter()
                    .any(|d| {
                        d.g("ctrl").opt_int() == Some(pid)
                            && spec(d.g("spec").str()).type_ == "Unit"
                            && d.g("tp").opt_int() == Some(pid)
                            && matches!(d.g("phase").opt_str(), Some("beginning" | "scoring"))
                    })
                    .into())
            }),
            ..Impl::new("Shadow Watcher")
        },
    );
    // Solari Shrine — "When you kill a stunned enemy unit, you may exhaust this to draw 1." (le joueur qui tue : le
    // contrôleur du sort ou de la capacité, ou de la source des dégâts mortels, règle 428.5)
    reg(r, Impl { on_event: shrine_event(), ..Impl::new("Solari Shrine") });
    // Svellsongur — "[Equip] 1 energy and 1 calm rune". Might Bonus +0 (image de la carte). "As this is attached to a
    // unit, copy that unit's text to this Equipment's effect text for as long as this is attached to it." Le texte
    // copié s'ajoute à l'unité (Game.copied_texts) : ses capacités déclenchées, Deathknells, modificateurs de Might,
    // mots-clés, capacités activées et effets sur d'autres objets comptent une seconde fois.
    {
        let mut im = Impl { bonus: Some(0), on_event: svell_event(), ..Impl::new("Svellsongur") };
        impl_equip(&mut im, Cost::from("1 energy and 1 calm rune"));
        reg(r, im);
    }
    // Mystic Reversal — "[Reaction] Gain control of a spell. You may make new choices for it."
    reg(
        r,
        Impl {
            timing: Some("reaction"),
            resolve: res_fn("_reversal", reversal),
            choices: reversal_choices(),
            ..Impl::new("Mystic Reversal")
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combinations_comme_itertools() {
        let v = [1, 2, 3, 4];
        assert_eq!(combinations(&v, 2), vec![vec![1, 2], vec![1, 3], vec![1, 4], vec![2, 3], vec![2, 4], vec![3, 4]]);
        assert_eq!(combinations(&v, 0), vec![Vec::<i32>::new()]);
        assert!(combinations(&v, 5).is_empty());
        assert_eq!(combinations(&v, 4), vec![vec![1, 2, 3, 4]]);
    }

    #[test]
    fn compositions_comme_python() {
        assert_eq!(compositions(4, 2, 400), vec![vec![1, 3], vec![2, 2], vec![3, 1]]);
        assert_eq!(compositions(3, 3, 400), vec![vec![1, 1, 1]]);
        assert!(compositions(2, 3, 400).is_empty());
        assert_eq!(compositions(10, 3, 5).len(), 5);
    }
}
