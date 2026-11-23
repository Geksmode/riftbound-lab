//! Paquet body : cartes Body (liste : batches/body.txt). Traduction ligne à ligne de
//! `riftbound/engine/cardsets/body.py` (voir `cardsets/GUIDE.md` et `docs/memoire/portage-rust.md`).
//!
//! Conventions du module Python :
//! - « X and Y deal damage equal to their Mights to each other » (Challenge...) : les dégâts sont infligés par les
//!   unités, pas par le sort (règle 417.6.b.3), donc avec le genre "unit" (pas de bonus de Void Gate, réservé aux
//!   sorts et capacités) et le contrôleur de chaque unité est responsable de ses dégâts (417.6.b.4). Les deux Mights
//!   sont lues avant tout dégât (dégâts simultanés, 417.1.d).
//! - Les choix des capacités déclenchées se font à la finalisation (règle 383.3), en payant [Deflect] pour les unités
//!   ennemies (809).
//! - Les coûts dans les instructions des capacités déclenchées se paient à la finalisation, ceux des sorts à la
//!   résolution (740.4.a).
//!
//! Les cartes qui posent des attributs sur l'objet (`setattr(o, "_body_...")` en Python : Lucian, Merciless ;
//! Miss Fortune, Captain ; Sivir, Ambitious ; Udyr, Wildman) sont dans le module `avec_attrs` en bas de ce fichier
//! (`Obj.attrs`, clé au nom de l'attribut Python).
use std::cmp::Ordering;

use crate::actions::{attach, deflect_reqs, full_choices, play_card, total_cost};
use crate::cards::{Impl, Registry};
use crate::cards_helpers::{
    ability, add_ability, cap, empower_ability, enemies, flow_cost, friends, impl_empower, impl_equip, item_by_id,
    make_token, may_pay, p_enemy_bf_v, p_enemy_v, p_friend, p_friend_bf_v, p_friend_v, p_unit_v,
    pay_deflect, play_unit_free, repeat_cost, repeatable, tg_choices, unit_play_choices, value, when_empowered,
    AbilityArgs, Cost,
};
use crate::game::{Game, ItemId, Res, Zone, O};
use crate::game_costs::{any_req, reqs_to_value, Req};
use crate::map;
use crate::spec::spec;
use crate::value::{is_bf, Loc, Map, Value};

/// `_BODY`
fn body_req() -> Req {
    ["Body".to_string()].into_iter().collect()
}

// ====================================================================== outils de traduction (privés)
fn fv(name: &'static str, f: impl Fn(&mut Game, &[Value]) -> Res<Value> + Send + Sync + 'static) -> Value {
    Value::func(name, f)
}

/// `card(name, ...)` de ce module.
fn new(name: &str) -> Impl {
    Impl { module: Some("body"), ..Impl::new(name) }
}

fn turn() -> Value {
    Value::from("turn")
}

/// tuple d'uids `(u1, u2...)`
fn tup(uids: &[u32]) -> Value {
    Value::Tuple(uids.iter().map(|&u| Value::from(u)).collect())
}

fn maps(v: Vec<Map>) -> Value {
    Value::List(v.into_iter().map(Value::Map).collect())
}

fn ovals(v: &[O]) -> Vec<Value> {
    v.iter().map(|&o| Value::Obj(o)).collect()
}

/// `x is o`
fn is_o(v: &Value, o: O) -> bool {
    v.opt_obj() == Some(o)
}

/// `o in info["units"]`
fn in_units(info: &Map, o: O) -> bool {
    info.g("units").list().iter().any(|v| v.opt_obj() == Some(o))
}

/// `g.units(pid, loc)` de Python avec un emplacement qui peut valoir None (aucune unité n'a l'emplacement None).
fn units_at(g: &Game, pid: Option<usize>, loc: Option<Loc>) -> Vec<O> {
    g.units(pid, None).into_iter().filter(|&u| g.o(u).loc == loc).collect()
}

fn ctrl(g: &Game, it: ItemId) -> usize {
    g.it(it).ctrl
}

/// `g.obj(it.src)`
fn src_board(g: &Game, it: ItemId) -> Option<O> {
    g.it(it).src.and_then(|u| g.obj(u))
}

/// `ctx["hidden_bf"]` : un battlefield (emplacement ou indice) ou None.
fn hbv(v: &Value) -> Option<Loc> {
    match v {
        Value::Loc(l) => Some(*l),
        Value::Int(i) => Some(Loc::Bf(*i as usize)),
        _ => None,
    }
}

/// un battlefield rangé dans un choix ou un évènement (`bf=b.idx`, `info["bf"]`)
fn bfi(v: &Value) -> usize {
    match v {
        Value::Loc(Loc::Bf(i)) => *i,
        v => v.int() as usize,
    }
}

fn bk(b: bool) -> f64 {
    if b {
        1.0
    } else {
        0.0
    }
}

fn cmp_keys(a: &[f64], b: &[f64]) -> Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        match x.partial_cmp(y).unwrap_or(Ordering::Equal) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    a.len().cmp(&b.len())
}

/// `sorted(v, key=...)` : clés calculées une fois par élément, dans l'ordre (un tuple Python devient un vecteur de
/// nombres, un booléen 0/1) ; tri stable.
fn sort_keyed<T: Clone>(v: Vec<T>, keys: Vec<Vec<f64>>) -> Vec<T> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|&a, &b| cmp_keys(&keys[a], &keys[b]));
    idx.into_iter().map(|i| v[i].clone()).collect()
}

/// prédicat de cible `pred(g, it, o)`
fn pred(f: impl Fn(&mut Game, ItemId, O) -> Res<bool> + Send + Sync + 'static) -> Value {
    fv("pred", move |g, a| Ok(Value::Bool(f(g, a[0].item(), a[1].obj())?)))
}

/// `(de, dp)` d'un `cost_mod`
fn de_dp(de: i64, dp: i64) -> Value {
    Value::Tuple(vec![de.into(), dp.into()])
}

/// `(e, reqs)` d'un `extra_cost_fn`
fn e_reqs(e: i64, reqs: &[Req]) -> Value {
    Value::Tuple(vec![e.into(), reqs_to_value(reqs)])
}

/// `itertools.combinations(range(n), k)` (ordre lexicographique des indices)
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
        idx[i - 1] += 1;
        for j in i..k {
            idx[j] = idx[j - 1] + 1;
        }
    }
}

/// dict d'un effet de `g.effects`, clés dans l'ordre de Python (`fn` est un mot réservé de Rust)
fn eff_map(pairs: Vec<(&str, Value)>) -> Map {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.set(k, v);
    }
    m
}

// ====================================================================== helpers (body.py)
/// `_src(g, it)` : le permanent qui a créé l'item, s'il est encore le même objet sur le plateau.
fn src(g: &Game, it: ItemId) -> Option<O> {
    let o = src_board(g, it)?;
    let oid = g.it(it).data.g("oid");
    if !oid.is_none() && g.o(o).oid != oid.int() {
        return None;
    }
    Some(o)
}

/// `_trig(g, o, name, fn, data, **kw)`
#[allow(clippy::too_many_arguments)]
fn trig_full(g: &mut Game, o: O, name: &str, f: Value, data: Map, may: bool, choose: Value, cost: Value) -> bool {
    let mut d = data;
    d.set("oid", g.o(o).oid);
    let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
    g.queue_trigger(c, name, f, d, Some(uid), may, choose, cost);
    true
}

fn trig(g: &mut Game, o: O, name: &str, f: Value) -> bool {
    trig_full(g, o, name, f, Map::new(), false, Value::None, Value::None)
}

/// `_payable(g, it, o)` : le Deflect d'un objet ennemi peut être payé par le contrôleur de l'item (809).
fn payable(g: &mut Game, it: ItemId, o: O) -> Res<bool> {
    let c = ctrl(g, it);
    let ch = map! { tg = tup(&[g.o(o).uid]) };
    let reqs = deflect_reqs(g, c, &ch)?;
    g.can_pay(c, 0, &reqs, &Value::None)
}

/// `_pick(options_fn, pred, n, kind, deflect)` : choix à la finalisation d'une capacité déclenchée qui choisit
/// jusqu'à n objets (n=1 : exactement un). Les objets ennemis avec [Deflect] doivent être payés (809). Renvoie False
/// (capacité retirée) quand rien n'est choisi.
fn pick(
    options_fn: impl Fn(&mut Game, ItemId) -> Res<Vec<O>> + Send + Sync + 'static,
    pr: Value,
    n: usize,
    kind: &'static str,
    deflect: bool,
) -> Value {
    fv("choose", move |g, a| {
        let it = a[0].item();
        let opts = options_fn(g, it)?;
        let mut chosen: Vec<O> = vec![];
        for _ in 0..n {
            let mut cand = vec![];
            for &o in &opts {
                if !chosen.contains(&o) && (!deflect || payable(g, it, o)?) {
                    cand.push(o);
                }
            }
            if cand.is_empty() {
                break;
            }
            let mut options = ovals(&cand);
            if !chosen.is_empty() || n > 1 {
                options.push(Value::None);
            }
            let c = ctrl(g, it);
            let Some(o) = g.ask(c, kind, options, map! { item = it })?.opt_obj() else { break };
            if deflect && !pay_deflect(g, it, o)? {
                break;
            }
            g.add_target(it, o, pr.clone());
            chosen.push(o);
        }
        Ok(Value::Bool(!chosen.is_empty()))
    })
}

/// `_targets(g, it)`
fn targets(g: &mut Game, it: ItemId) -> Res<Vec<O>> {
    let mut out = vec![];
    for i in 0..g.it(it).targets.len() {
        if let Some(o) = g.legal(it, i)? {
            out.push(o);
        }
    }
    Ok(out)
}

/// `_fight(g, a, b)` : a et b s'infligent des dégâts égaux à leurs Mights (simultanés, infligés par les unités).
fn fight(g: &mut Game, a: Option<O>, b: Option<O>) -> Res<()> {
    let (Some(a), Some(b)) = (a, b) else { return Ok(()) };
    if a == b || !g.on_board(a) || !g.on_board(b) {
        return Ok(());
    }
    let ma = 0.max(g.might(a)?);
    let mb = 0.max(g.might(b)?);
    let ca = g.o(a).ctrl;
    g.deal(Some(b), ma, "unit", Some(ca), None)?;
    let cb = g.o(b).ctrl;
    g.deal(Some(a), mb, "unit", Some(cb), None)?;
    Ok(())
}

/// `_fight_score(g, f, e, bonus)` : ordre des combats (allié, ennemi) pour l'IA : tuer l'ennemi, garder l'allié.
fn fight_score(g: &mut Game, f: O, e: O, bonus: i64) -> Res<f64> {
    let mf = g.might(f)? + bonus;
    let me = g.might(e)?;
    let mut s = 0.0;
    if mf > 0 && mf >= me - g.o(e).damage {
        s += 10.0 + value(g, e)?;
    }
    if me > 0 && me >= mf - g.o(f).damage {
        s -= 5.0 + value(g, f)?;
    }
    Ok(-s)
}

/// `_duel_choices(g, pid, frs, ens, bonus, extra)`
fn duel_choices(g: &mut Game, pid: usize, frs: &[O], ens: &[O], bonus: i64, extra: Option<&Map>) -> Res<Vec<Map>> {
    let mut out: Vec<Map> = vec![];
    let mut keys: Vec<Vec<f64>> = vec![];
    for f in cap(g, frs.to_vec(), 4, Some(pid)) {
        for e in cap(g, ens.to_vec(), 4, Some(pid)) {
            let mut d = map! { tg = tup(&[g.o(f).uid, g.o(e).uid]) };
            if let Some(x) = extra {
                if !x.0.is_empty() {
                    d = d.merged(x);
                }
            }
            let s = fight_score(g, f, e, bonus)?;
            keys.push(vec![s, out.len() as f64]);
            out.push(d);
        }
    }
    Ok(sort_keyed(out, keys))
}

/// `_spend_buff(g, o)` : dépenser un buff (745) ; émet 'spend_buff' (Fae Dragon).
fn spend_buff(g: &mut Game, o: Option<O>) -> Res<bool> {
    g.spend_buff(o, None)
}

/// `_dests(g, u)`
fn dests(g: &Game, u: O) -> Vec<Loc> {
    g.locs().into_iter().filter(|&d| Some(d) != g.o(u).loc).collect()
}

/// `_reduced_cost(g, pid, c, ch, src, de)`
fn reduced_cost(g: &mut Game, pid: usize, c: O, ch: &Map, src: &str, de: i64) -> Res<(i64, Vec<Req>)> {
    let (e, reqs) = total_cost(g, pid, c, ch, src)?;
    Ok((0.max(e - de), reqs))
}

/// `_play_reduced(g, pid, c, src, ch, de)` : jouer une carte depuis src en payant son coût réduit de de énergie
/// (Here to Help, Wild Claw).
fn play_reduced(g: &mut Game, pid: usize, c: O, src: &str, ch: Map, de: i64) -> Res<Value> {
    let (e, reqs) = reduced_cost(g, pid, c, &ch, src, de)?;
    let ctx = Value::Map(map! { kind = g.spec(c).type_.to_lowercase(), card = c });
    if !g.pay(pid, e, &reqs, &ctx)? {
        return Ok(Value::None);
    }
    g.effects
        .retain(|ef| !(ef.d.g("kind").opt_str() == Some("heron") && ef.d.g("pid").opt_int() == Some(pid as i64)));
    play_card(g, pid, c, src, ch, true)
}

/// `_ready_runes(g, pid, n)`
fn ready_runes(g: &mut Game, pid: usize, n: usize) -> Res<usize> {
    let ex: Vec<_> = g.p[pid].runes.iter().copied().filter(|&r| g.r(r).exhausted).collect();
    let doms: std::collections::BTreeSet<&str> = ex.iter().map(|&r| g.r(r).domain.as_str()).collect();
    let ndom = doms.len();
    if full_choices(g, Some(pid)) && ex.len() > n && ndom > 1 {
        for _ in 0..n {
            // un humain choisit quelles runes (par domaine)
            let opts: Vec<Value> =
                g.p[pid].runes.iter().copied().filter(|&r| g.r(r).exhausted).map(Value::Rune).collect();
            let r = g.ask(pid, "ready_rune", opts, Map::new())?.rune();
            g.r_mut(r).exhausted = false;
        }
        return Ok(n);
    }
    let ex: Vec<_> = ex.into_iter().take(n).collect();
    for &r in &ex {
        g.r_mut(r).exhausted = false;
    }
    Ok(ex.len())
}

/// `_won(g, o, info)` : o gagne le combat qui vient d'avoir un résultat (il reste au battlefield, règle 466.3).
fn won(g: &Game, o: O, info: &Map) -> bool {
    info.g("pid").opt_int() == Some(g.o(o).ctrl as i64)
        && g.o(o).loc == Some(Loc::Bf(bfi(&info.g("bf"))))
        && g.o(o).desig.is_some()
}

/// on_event de la forme `ev == X and info["obj"] is o`
fn ev_obj(a: &[Value], ev: &str) -> Option<(O, Map)> {
    let o = a[0].obj();
    if a[1].str() == ev && is_o(&a[2].map().g("obj"), o) {
        Some((o, a[2].map().clone()))
    } else {
        None
    }
}

// ====================================================================== unités
// Anivia, Primal — "When I attack, deal 3 to all enemy units here."
fn anivia() -> Impl {
    let on_event = fv("_anivia", |g, a| {
        if let Some((o, _)) = ev_obj(a, "attack") {
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let me = src(g_, it);
                let here = match me {
                    Some(m) => g_.o(m).loc,
                    None => g_.it(it).data.g("loc").opt_loc(),
                };
                let c = ctrl(g_, it);
                for u in units_at(g_, Some(1 - c), here) {
                    g_.deal(Some(u), 3, "ability", Some(c), None)?;
                }
                Ok(Value::None)
            });
            let loc = g.o(o).loc;
            trig_full(g, o, "Anivia, Primal", res, map! { loc = loc }, false, Value::None, Value::None);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Anivia, Primal") }
}

// Baccai Witherclaw — "[Empower] [1][A][A] [Empowered] I have +2 might. [Empowered][Deathknell] Channel 2 runes
// exhausted." The Deathknell is a dependent ability: it triggers only if I died while Empowered (look-back, 808).
fn baccai() -> Impl {
    let mut im = Impl {
        might_if: vec![(when_empowered(), 2)],
        deathknell: fv("dk", |g, a| {
            let c = ctrl(g, a[0].item());
            g.channel(c, 2, true)?;
            Ok(Value::None)
        }),
        dk_choose: fv("dk_choose", |g, a| {
            Ok(Value::Bool(g.it(a[0].item()).data.g("info").map().g("empowered").truthy()))
        }),
        ..new("Baccai Witherclaw")
    };
    impl_empower(&mut im, Cost::Text("1 energy and 2 runes of any type".into()));
    im
}

// Bilgewater Bully — "While I'm buffed, I have [Ganking]."
fn bilgewater() -> Impl {
    Impl {
        kw_if: vec![(fv("buffed", |g, a| Ok(Value::Bool(g.o(a[0].obj()).buff > 0))), map! { Ganking = 1 })],
        ..new("Bilgewater Bully")
    }
}

// Brutal Hunter — "[Empower] [3] [Empowered] I have +2 might and [Ganking]."
fn brutal_hunter() -> Impl {
    let mut im = Impl {
        might_if: vec![(when_empowered(), 2)],
        kw_if: vec![(when_empowered(), map! { Ganking = 1 })],
        ..new("Brutal Hunter")
    };
    impl_empower(&mut im, Cost::Text("3 energy".into()));
    im
}

// Buhru Captain — "When you play me, you may draw 1 or buff me."
fn buhru() -> Impl {
    let on_play = fv("_buhru", |g, a| {
        let o = a[0].obj();
        let res = fv("res", |g_, a| {
            let it = a[0].item();
            let me = src(g_, it);
            let mut opts: Vec<Value> = if me.is_some_and(|m| g_.o(m).buff == 0) {
                vec!["buff".into(), "draw".into()]
            } else {
                vec!["draw".into(), "buff".into()]
            };
            opts.push(Value::None);
            let c = ctrl(g_, it);
            let ch = g_.ask(c, "buhru_mode", opts, map! { item = it })?;
            match ch.opt_str() {
                Some("draw") => g_.draw(c, 1)?,
                Some("buff") if me.is_some() => {
                    g_.buff(me, None)?;
                }
                _ => {}
            }
            Ok(Value::None)
        });
        trig(g, o, "Buhru Captain", res);
        Ok(Value::None)
    });
    Impl { on_play, ..new("Buhru Captain") }
}

// Carnivorous Snapvine — "When you play me, choose an enemy unit at a battlefield. We deal damage equal to our
// Mights to each other."
fn snapvine() -> Impl {
    let on_play = fv("_snapvine", |g, a| {
        let o = a[0].obj();
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let res = fv("res", |g_, a| {
            let it = a[0].item();
            let s = src(g_, it);
            let t = g_.legal(it, 0)?;
            fight(g_, s, t)?;
            Ok(Value::None)
        });
        let choose = pick(
            move |g_, it| {
                let c = ctrl(g_, it);
                enemies(g_, c, true, hb, true)
            },
            p_enemy_bf_v(),
            1,
            "target",
            true,
        );
        trig_full(g, o, "Carnivorous Snapvine", res, Map::new(), false, choose, Value::None);
        Ok(Value::None)
    });
    Impl { on_play, ..new("Carnivorous Snapvine") }
}

// Cithria of Cloudfield — "When you play another unit, buff me."
fn cithria() -> Impl {
    let on_event = fv("_cithria", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "played" && info.g("pid").opt_int() == Some(g.o(o).ctrl as i64) {
            let card = info.g("card").obj();
            if card != o && g.spec(card).type_ == "Unit" {
                let res = fv("res", |g_, a| {
                    let s = src(g_, a[0].item());
                    g_.buff(s, None)?;
                    Ok(Value::None)
                });
                trig(g, o, "Cithria of Cloudfield", res);
            }
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Cithria of Cloudfield") }
}

// Corrupted Dragon — "If your score is not within 3 points of the Victory Score, I enter ready. When I attack, you
// may move any number of enemy units here each with 5 might or less to their base."
fn corrupted_dragon() -> Impl {
    let on_event = fv("_cdragon", |g, a| {
        if let Some((o, _)) = ev_obj(a, "attack") {
            let here = g.o(o).loc;
            let pr = pred(move |g_, it, u| {
                Ok(g_.o(u).ctrl != ctrl(g_, it) && g_.o(u).loc == here && g_.might(u)? <= 5)
            });
            let opts = move |g_: &mut Game, it: ItemId| -> Res<Vec<O>> {
                let c = ctrl(g_, it);
                let mut out = vec![];
                for u in enemies(g_, c, false, None, true)? {
                    if g_.o(u).loc == here && g_.might(u)? <= 5 {
                        out.push(u);
                    }
                }
                Ok(out)
            };
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let c = ctrl(g_, it);
                for u in targets(g_, it)? {
                    g_.move_(&[u], Loc::Base, Some(c), false)?;
                }
                Ok(Value::None)
            });
            let choose = pick(opts, pr, 99, "corrupted_dragon", true);
            trig_full(g, o, "Corrupted Dragon", res, Map::new(), false, choose, Value::None);
        }
        Ok(Value::None)
    });
    Impl {
        on_event,
        enter_ready: fv("enter_ready", |g, a| {
            let pid = a[0].int() as usize;
            Ok(Value::Bool(g.victory - g.p[pid].points > 3))
        }),
        ..new("Corrupted Dragon")
    }
}

// Crackshot Corsair — "When I attack, deal 1 to an enemy unit here."
fn corsair() -> Impl {
    let on_event = fv("_corsair", |g, a| {
        if let Some((o, _)) = ev_obj(a, "attack") {
            let here = g.o(o).loc;
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let t = g_.legal(it, 0)?;
                let c = ctrl(g_, it);
                g_.deal(t, 1, "ability", Some(c), None)?;
                Ok(Value::None)
            });
            let choose = pick(
                move |g_, it| {
                    let c = ctrl(g_, it);
                    Ok(enemies(g_, c, false, None, true)?.into_iter().filter(|&u| g_.o(u).loc == here).collect())
                },
                pred(move |g_, it, u| Ok(g_.o(u).ctrl != ctrl(g_, it) && g_.o(u).loc == here)),
                1,
                "target",
                true,
            );
            trig_full(g, o, "Crackshot Corsair", res, map! { dmg = 1 }, false, choose, Value::None);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Crackshot Corsair") }
}

// Crowd Favorite — "[Hunt] Spend 2 XP: [Buff] me."
fn crowd_favorite() -> Impl {
    Impl {
        kw: map! { Hunt = 1 },
        abilities: vec![ability(
            "Buff",
            AbilityArgs {
                xp: 2,
                resolve: fv("res", |g, a| {
                    let s = src_board(g, a[0].item());
                    g.buff(s, None)?;
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..new("Crowd Favorite")
    }
}

// Dame the Despoiler — "[Empower] [5][Body] [Empowered] When I attack or defend, choose a unit here. Increase my
// Might to its Might this turn, then give me +1 might this turn."
fn dame() -> Impl {
    let on_event = fv("_dame", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if (ev == "attack" || ev == "defend") && is_o(&info.g("obj"), o) && g.o(o).empowered {
            let here = g.o(o).loc;
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let Some(me) = src(g_, it) else { return Ok(Value::None) };
                if let Some(t) = g_.legal(it, 0)? {
                    let diff = g_.might(t)? - g_.might(me)?;
                    if diff > 0 {
                        g_.mod_(me, diff, turn(), None)?;
                    }
                }
                g_.mod_(me, 1, turn(), None)?;
                Ok(Value::None)
            });
            let opts = move |g_: &mut Game, it: ItemId| -> Res<Vec<O>> {
                let c = ctrl(g_, it);
                let s = g_.it(it).src;
                let mut us = vec![];
                for u in units_at(g_, None, here) {
                    if Some(g_.o(u).uid) != s && (g_.o(u).ctrl == c || g_.targetable(u, Some(c))?) {
                        us.push(u);
                    }
                }
                let mut keys = vec![];
                for &u in &us {
                    keys.push(vec![-(g_.might(u)? as f64)]);
                }
                Ok(sort_keyed(us, keys))
            };
            let choose = pick(opts, pred(move |g_, _it, u| Ok(g_.o(u).loc == here)), 1, "dame_unit", true);
            trig_full(g, o, "Dame the Despoiler", res, Map::new(), false, choose, Value::None);
        }
        Ok(Value::None)
    });
    let mut im = Impl { on_event, ..new("Dame the Despoiler") };
    impl_empower(&mut im, Cost::Text("5 energy and 1 body rune".into()));
    im
}

// Dazzling Aurora — "At the end of your turn, reveal cards from the top of your Main Deck until you reveal a unit.
// Play it, ignoring its cost, and recycle the rest." (Revealing never burns out, rule 431.1.c.)
fn aurora() -> Impl {
    let on_event = fv("_aurora", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "end_turn" && info.g("pid").opt_int() == Some(g.o(o).ctrl as i64) {
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let c = ctrl(g_, it);
                let mut revealed = vec![];
                let mut unit = None;
                let until = |g2: &Game, x: O| g2.spec(x).type_ == "Unit" && g2.impl_o(x).is_some();
                for x in g_.reveal(c, 1, None, true, Some(&until))? {
                    if g_.spec(x).type_ == "Unit" && g_.impl_o(x).is_some() {
                        unit = Some(x);
                        break;
                    }
                    revealed.push(x);
                }
                if let Some(u) = unit {
                    play_unit_free(g_, c, u, "deck", None, false, None)?;
                }
                for &x in &revealed {
                    if let Some(i) = g_.p[c].deck.iter().position(|&y| y == x) {
                        g_.p[c].deck.remove(i);
                    }
                }
                g_.recycle_cards(c, &revealed)?;
                Ok(Value::None)
            });
            trig(g, o, "Dazzling Aurora", res);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Dazzling Aurora") }
}

// Demacian Diplomat — "When you play me, gain 1 XP."
fn diplomat() -> Impl {
    let on_play = fv("on_play", |g, a| {
        let o = a[0].obj();
        let res = fv("res", |g_, a| {
            let c = ctrl(g_, a[0].item());
            g_.gain_xp(c, 1)?;
            Ok(Value::None)
        });
        trig(g, o, "Demacian Diplomat", res);
        Ok(Value::None)
    });
    Impl { on_play, ..new("Demacian Diplomat") }
}

// Direwing — "I enter ready if you control another Dragon."
fn direwing() -> Impl {
    Impl {
        enter_ready: fv("enter_ready", |g, a| {
            let (pid, c) = (a[0].int() as usize, a[1].obj());
            for u in g.units(Some(pid), None) {
                if g.tags(u)?.contains("Dragon") && u != c {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }),
        ..new("Direwing")
    }
}

// Dragonsoul Sage — "[Reaction][>] [E]: [Add] [1]." (used while paying, rule 429.3)
fn dragonsoul_sage() -> Impl {
    Impl { add: vec![add_ability("1 energy", true, false, Value::None)], ..new("Dragonsoul Sage") }
}

// Dune Drake — "When I attack, give me +2 might this turn if there is a ready enemy unit here."
fn dune_drake() -> Impl {
    let on_event = fv("_drake", |g, a| {
        if let Some((o, _)) = ev_obj(a, "attack") {
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                if let Some(me) = src(g_, it) {
                    let (mc, ml) = (g_.o(me).ctrl, g_.o(me).loc);
                    if units_at(g_, Some(1 - mc), ml).iter().any(|&u| !g_.exhausted(u)) {
                        g_.mod_(me, 2, turn(), None)?;
                    }
                }
                Ok(Value::None)
            });
            trig(g, o, "Dune Drake", res);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Dune Drake") }
}

// Fiora, Peerless — "When I attack or defend one on one, double my Might this combat." (one on one: she and the
// enemy unit there are both alone, rule 740.2.b; "this combat" effects expire when the combat ends, 466.7.c)
fn fiora() -> Impl {
    let on_event = fv("_fiora", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if (ev == "attack" || ev == "defend") && is_o(&info.g("obj"), o) {
            let (oc, ol) = (g.o(o).ctrl, g.o(o).loc);
            if !(g.alone(o, None) && units_at(g, Some(1 - oc), ol).len() == 1) {
                return Ok(Value::None);
            }
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let Some(me) = src(g_, it) else { return Ok(Value::None) };
                let m = g_.might(me)?;
                if m <= 0 {
                    return Ok(Value::None);
                }
                // `entry = [m, "turn"]` : les mods ne sont qu'ajoutés, sommés, vidés en fin de tour ou au
                // changement de zone (oid) ; retirer une entrée égale de la même incarnation revient à retirer
                // celle-ci.
                g_.o_mut(me).mods.push((m, turn()));
                let me_oid = g_.o(me).oid;
                let end = fv("end", move |g2, a| {
                    let eid = a[0].int() as u64;
                    let inf = a[1].map();
                    let eff = g2.eff(eid).cloned().unwrap_or_default();
                    if Value::Loc(Loc::Bf(bfi(&inf.g("bf")))) == eff.g("bf") {
                        if let Some(u) = g2.obj(eff.g("uid").int() as u32) {
                            if g2.o(u).oid == me_oid {
                                let mods = &mut g2.o_mut(u).mods;
                                if let Some(i) = mods.iter().position(|x| x.0 == m && x.1 == turn()) {
                                    mods.remove(i);
                                }
                            }
                        }
                        g2.remove_effect(eid);
                    }
                    Ok(Value::None)
                });
                let (bf, uid) = (g_.o(me).loc, g_.o(me).uid);
                g_.add_effect(eff_map(vec![
                    ("on", "combat_end".into()),
                    ("fn", end),
                    ("dur", turn()),
                    ("bf", bf.into()),
                    ("uid", uid.into()),
                    ("entry", Value::List(vec![m.into(), turn()])),
                ]));
                Ok(Value::None)
            });
            trig(g, o, "Fiora, Peerless", res);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Fiora, Peerless") }
}

// First Mate — "When you play me, ready another unit."
fn first_mate() -> Impl {
    let on_play = fv("_first_mate", |g, a| {
        let o = a[0].obj();
        let opts = |g_: &mut Game, it: ItemId| -> Res<Vec<O>> {
            let c = ctrl(g_, it);
            let s = g_.it(it).src;
            let mut us = vec![];
            for u in g_.units(None, None) {
                if Some(g_.o(u).uid) != s && (g_.o(u).ctrl == c || g_.targetable(u, Some(c))?) {
                    us.push(u);
                }
            }
            let mut keys = vec![];
            for &u in &us {
                keys.push(vec![bk(g_.o(u).ctrl != c), bk(!g_.exhausted(u)), -value(g_, u)?]);
            }
            Ok(sort_keyed(us, keys))
        };
        let res = fv("res", |g_, a| {
            let it = a[0].item();
            if g_.legal(it, 0)?.is_some() {
                if let Some(t) = g_.legal(it, 0)? {
                    g_.ready_obj(t, None, None)?;
                }
            }
            Ok(Value::None)
        });
        let choose = pick(opts, p_unit_v(), 1, "ready_target", true);
        trig_full(g, o, "First Mate", res, Map::new(), false, choose, Value::None);
        Ok(Value::None)
    });
    Impl { on_play, ..new("First Mate") }
}

// Fretful Feline — "When I become ready, give me +2 might this turn."
fn fretful_feline() -> Impl {
    let on_event = fv("on_event", |g, a| {
        if let Some((o, _)) = ev_obj(a, "ready") {
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                if src(g_, it).is_some() {
                    let s = src(g_, it).unwrap();
                    g_.mod_(s, 2, turn(), None)?;
                }
                Ok(Value::None)
            });
            trig(g, o, "Fretful Feline", res);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Fretful Feline") }
}

// Garen, Rugged — "[Assault 2], [Shield 2]"
fn garen() -> Impl {
    Impl { kw: map! { Assault = 2, Shield = 2 }, ..new("Garen, Rugged") }
}

// Gemhand Hunter — "[Hunt] [Level 6] I have +1 might." (the trailing "ambush" of the database is not on the card)
fn gemhand() -> Impl {
    Impl { kw: map! { Hunt = 1 }, levels: vec![(6, map! { might = 1 })], ..new("Gemhand Hunter") }
}

// Gentle Gemdragon — "When you play me or another Dragon, ready up to 2 runes."
fn gemdragon() -> Impl {
    let on_event = fv("_gemdragon", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "played" && info.g("pid").opt_int() == Some(g.o(o).ctrl as i64) {
            let card = info.g("card").obj();
            if card == o || g.tags(card)?.contains("Dragon") {
                let res = fv("res", |g_, a| {
                    let c = ctrl(g_, a[0].item());
                    ready_runes(g_, c, 2)?;
                    Ok(Value::None)
                });
                trig(g, o, "Gentle Gemdragon", res);
            }
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Gentle Gemdragon") }
}

// Imposing Challenger — "When I move, you may move an enemy unit here with less Might than me to a different
// battlefield." ("here" and "me" are checked again on resolution, rule 359.3.f)
fn challenger() -> Impl {
    let on_event = fv("_challenger", |g, a| {
        if let Some((o, _)) = ev_obj(a, "move") {
            if !is_bf(g.o(o).loc) {
                return Ok(Value::None);
            }
            let here = g.o(o).loc;
            let ok = pred(move |g_, it, u| {
                let Some(me) = src(g_, it) else { return Ok(false) };
                Ok(g_.o(u).ctrl != ctrl(g_, it)
                    && g_.o(u).loc == here
                    && g_.o(me).loc == here
                    && g_.might(u)? < g_.might(me)?)
            });
            let opts = move |g_: &mut Game, it: ItemId| -> Res<Vec<O>> {
                let Some(me) = src(g_, it) else { return Ok(vec![]) };
                let c = ctrl(g_, it);
                let mut out = vec![];
                for u in enemies(g_, c, false, None, true)? {
                    if g_.o(u).loc == here && g_.might(u)? < g_.might(me)? {
                        out.push(u);
                    }
                }
                Ok(out)
            };
            let res = fv("res", move |g_, a| {
                let it = a[0].item();
                if let Some(u) = g_.legal(it, 0)? {
                    // « a different battlefield » : le joueur en choisit un avec le Baron Pit (187.9)
                    let c = ctrl(g_, it);
                    let opts: Vec<Value> =
                        g_.bf_locs().into_iter().filter(|&b| Some(b) != here).map(Value::Loc).collect();
                    let dest = g_.ask(c, "move_destination", opts, map! { item = it })?;
                    if let Some(d) = dest.opt_loc() {
                        g_.move_(&[u], d, Some(c), false)?;
                    }
                }
                Ok(Value::None)
            });
            let choose = pick(opts, ok, 1, "target", true);
            trig_full(g, o, "Imposing Challenger", res, Map::new(), true, choose, Value::None);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Imposing Challenger") }
}

// Irresistible Faefolk — "When I move to a battlefield, you may move an enemy unit to that battlefield."
fn faefolk() -> Impl {
    let on_event = fv("_faefolk", |g, a| {
        if let Some((o, info)) = ev_obj(a, "move") {
            let Some(dest) = info.g("to").opt_loc().filter(|l| l.is_bf()) else { return Ok(Value::None) };
            let res = fv("res", move |g_, a| {
                let it = a[0].item();
                if let Some(u) = g_.legal(it, 0)? {
                    if g_.o(u).loc != Some(dest) {
                        let c = ctrl(g_, it);
                        g_.move_(&[u], dest, Some(c), false)?;
                    }
                }
                Ok(Value::None)
            });
            let choose = pick(
                move |g_, it| {
                    let c = ctrl(g_, it);
                    Ok(enemies(g_, c, false, None, true)?.into_iter().filter(|&u| g_.o(u).loc != Some(dest)).collect())
                },
                p_enemy_v(),
                1,
                "faefolk_target",
                true,
            );
            trig_full(g, o, "Irresistible Faefolk", res, Map::new(), true, choose, Value::None);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Irresistible Faefolk") }
}

// Jaull-Fish — "[Accelerate] I cost 2 energy less for each of your [Mighty] units."
fn jaull_fish() -> Impl {
    Impl {
        accelerate: true,
        cost_mod: fv("cost_mod", |g, a| {
            let pid = a[0].int() as usize;
            let mut n = 0;
            for u in g.units(Some(pid), None) {
                if g.mighty(u)? {
                    n += 1;
                }
            }
            Ok(de_dp(2 * n, 0))
        }),
        ..new("Jaull-Fish")
    }
}

// Jax, Unrelenting — "[Weaponmaster] When you attach an Equipment to me, you may pay [1] to draw 1."
fn jax() -> Impl {
    let on_event = fv("_jax", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "attached" && is_o(&info.g("unit"), o) && g.spec(info.g("gear").obj()).tags.contains("Equipment") {
            let res = fv("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.draw(c, 1)?;
                Ok(Value::None)
            });
            trig_full(g, o, "Jax, Unrelenting", res, Map::new(), true, Value::None, may_pay(1, Value::None));
        }
        Ok(Value::None)
    });
    Impl { kw: map! { Weaponmaster = 1 }, on_event, ..new("Jax, Unrelenting") }
}

// Jayce, Hammer in Hand — "When I become ready, choose one to give me this turn — [Assault 2]; [Deflect 2];
// [Ganking]."
fn jayce() -> Impl {
    let on_event = fv("_jayce", |g, a| {
        if let Some((o, _)) = ev_obj(a, "ready") {
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let Some(me) = src(g_, it) else { return Ok(Value::None) };
                let opts: Vec<Value> = if g_.o(me).loc == Some(Loc::Base) {
                    vec!["Assault".into(), "Ganking".into(), "Deflect".into()]
                } else {
                    vec!["Ganking".into(), "Assault".into(), "Deflect".into()]
                };
                let c = ctrl(g_, it);
                let m = g_.ask(c, "jayce_mode", opts, map! { item = it })?;
                let m = m.str().to_string();
                let v = if m == "Ganking" { 1 } else { 2 };
                g_.grant(me, &m, v, turn());
                Ok(Value::None)
            });
            trig(g, o, "Jayce, Hammer in Hand", res);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Jayce, Hammer in Hand") }
}

// Kato the Arm — "[Deflect] When I move to a battlefield, give a friendly unit my keywords and +might equal to my
// Might this turn." (keywords and Might read on resolution, rule 359.3.f)
const KWS: [&str; 10] =
    ["Assault", "Backline", "Deflect", "Ganking", "Hunt", "Shield", "Tank", "Temporary", "Vision", "Weaponmaster"];

fn kato() -> Impl {
    let on_event = fv("_kato", |g, a| {
        if let Some((o, info)) = ev_obj(a, "move") {
            if !is_bf(info.g("to").opt_loc()) {
                return Ok(Value::None);
            }
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let me = src(g_, it);
                let u = g_.legal(it, 0)?;
                let (Some(me), Some(u)) = (me, u) else { return Ok(Value::None) };
                for k in KWS {
                    let v = g_.kw_value(me, k)?;
                    if v > 0 {
                        g_.grant(u, k, v, turn());
                    }
                }
                let m = 0.max(g_.might(me)?);
                g_.mod_(u, m, turn(), None)?;
                Ok(Value::None)
            });
            let opts = |g_: &mut Game, it: ItemId| -> Res<Vec<O>> {
                let c = ctrl(g_, it);
                let s = g_.it(it).src;
                let us = g_.units(Some(c), None);
                let mut keys = vec![];
                for &u in &us {
                    keys.push(vec![bk(Some(g_.o(u).uid) == s), bk(!is_bf(g_.o(u).loc)), -value(g_, u)?]);
                }
                Ok(sort_keyed(us, keys))
            };
            let choose = pick(opts, p_friend_v(), 1, "kato_target", false);
            trig_full(g, o, "Kato the Arm", res, Map::new(), false, choose, Value::None);
        }
        Ok(Value::None)
    });
    Impl { kw: map! { Deflect = 1 }, on_event, ..new("Kato the Arm") }
}

// Kha'Zix, Evolving Hunter — "[Hunt] When I attack, you may spend 3 XP to deal damage equal to my Might to an enemy
// unit here." (cost within instructions of a trigger: paid on finalization, rule 740.4.a.2)
fn khazix() -> Impl {
    let on_event = fv("_khazix", |g, a| {
        if let Some((o, _)) = ev_obj(a, "attack") {
            if g.p[g.o(o).ctrl].xp < 3 {
                return Ok(Value::None);
            }
            let here = g.o(o).loc;
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let me = src(g_, it);
                let u = g_.legal(it, 0)?;
                if let (Some(me), Some(u)) = (me, u) {
                    let m = 0.max(g_.might(me)?);
                    let c = ctrl(g_, it);
                    g_.deal(Some(u), m, "ability", Some(c), None)?;
                }
                Ok(Value::None)
            });
            let dmg = g.might(o)?;
            let cost = fv("cost", |g_, a| {
                let c = ctrl(g_, a[0].item());
                Ok(Value::Bool(g_.spend_xp(c, 3)?))
            });
            let choose = pick(
                move |g_, it| {
                    let c = ctrl(g_, it);
                    Ok(enemies(g_, c, false, None, true)?.into_iter().filter(|&u| g_.o(u).loc == here).collect())
                },
                pred(move |g_, it, u| Ok(g_.o(u).ctrl != ctrl(g_, it) && g_.o(u).loc == here)),
                1,
                "target",
                true,
            );
            trig_full(g, o, "Kha'Zix, Evolving Hunter", res, map! { dmg = dmg }, true, choose, cost);
        }
        Ok(Value::None)
    });
    Impl { kw: map! { Hunt = 1 }, on_event, ..new("Kha'Zix, Evolving Hunter") }
}

// Kinkou Initiate — "When you play me, draw 1 if your other units have total Might 5 or more."
fn kinkou_initiate() -> Impl {
    let on_play = fv("_initiate", |g, a| {
        let o = a[0].obj();
        let res = fv("res", |g_, a| {
            let it = a[0].item();
            let c = ctrl(g_, it);
            let s = g_.it(it).src;
            let mut tot = 0;
            for u in g_.units(Some(c), None) {
                if Some(g_.o(u).uid) != s {
                    tot += 0.max(g_.might(u)?);
                }
            }
            if tot >= 5 {
                g_.draw(c, 1)?;
            }
            Ok(Value::None)
        });
        trig(g, o, "Kinkou Initiate", res);
        Ok(Value::None)
    });
    Impl { on_play, ..new("Kinkou Initiate") }
}

/// `_monk` / Pit Rookie : `sorted([u for u in g.units(ctrl) if u.uid != it.src], key=(u.buff > 0, -value))`
fn other_friends_by_buff(g: &mut Game, it: ItemId) -> Res<Vec<O>> {
    let c = ctrl(g, it);
    let s = g.it(it).src;
    let us: Vec<O> = g.units(Some(c), None).into_iter().filter(|&u| Some(g.o(u).uid) != s).collect();
    let mut keys = vec![];
    for &u in &us {
        keys.push(vec![bk(g.o(u).buff > 0), -value(g, u)?]);
    }
    Ok(sort_keyed(us, keys))
}

// Kinkou Monk — "When you play me, buff up to two other friendly units."
fn kinkou_monk() -> Impl {
    let on_play = fv("_monk", |g, a| {
        let o = a[0].obj();
        let res = fv("res", |g_, a| {
            let it = a[0].item();
            for u in targets(g_, it)? {
                g_.buff(Some(u), None)?;
            }
            Ok(Value::None)
        });
        let choose = pick(other_friends_by_buff, p_friend_v(), 2, "buff_target", false);
        trig_full(g, o, "Kinkou Monk", res, Map::new(), false, choose, Value::None);
        Ok(Value::None)
    });
    Impl { on_play, ..new("Kinkou Monk") }
}

// Kraken Hunter — "[Accelerate] [Assault] As you play me, you may spend any number of buffs as an additional cost.
// Reduce my cost by [Body] for each buff you spend."
fn kraken() -> Impl {
    let as_played = fv("_kraken_extra", |g, a| {
        let pid = a[0].int() as usize;
        let bs: Vec<O> = g.units(Some(pid), None).into_iter().filter(|&u| g.o(u).buff > 0).collect();
        let mut keys = vec![];
        for &u in &bs {
            keys.push(vec![value(g, u)?]);
        }
        let bs = sort_keyed(bs, keys);
        let mut out = vec![Map::new()];
        if full_choices(g, Some(pid)) {
            // un humain : chaque ensemble de buffs
            for n in 1..=bs.len() {
                for grp in combinations(bs.len(), n) {
                    let uids: Vec<u32> = grp.iter().map(|&i| g.o(bs[i]).uid).collect();
                    out.push(map! { spend = tup(&uids) });
                }
            }
            return Ok(maps(out));
        }
        for &u in bs.iter().take(2) {
            out.push(map! { spend = tup(&[g.o(u).uid]) });
        }
        if bs.len() >= 2 {
            out.push(map! { spend = tup(&[g.o(bs[0]).uid, g.o(bs[1]).uid]) });
        }
        Ok(maps(out))
    });
    let pay_extra = fv("_kraken_pay", |g, a| {
        let pid = a[0].int() as usize;
        let ch = a[2].map();
        for uid in ch.g("spend").list() {
            if let Some(u) = g.obj(uid.int() as u32) {
                if g.o(u).ctrl == pid {
                    spend_buff(g, Some(u))?;
                }
            }
        }
        Ok(Value::None)
    });
    let cost_mod = fv("cost_mod", |g, a| {
        let ch = a[2].map();
        let n = ch
            .g("spend")
            .list()
            .iter()
            .filter(|u| g.obj(u.int() as u32).is_some_and(|x| g.o(x).buff > 0))
            .count();
        Ok(de_dp(0, n as i64))
    });
    Impl { accelerate: true, kw: map! { Assault = 1 }, as_played, pay_extra, cost_mod, ..new("Kraken Hunter") }
}

// Laurent Bladekeeper — "Ganking"
fn laurent() -> Impl {
    Impl { kw: map! { Ganking = 1 }, ..new("Laurent Bladekeeper") }
}

// Lee Sin, Centered — "[Accelerate] Other buffed friendly units at my battlefield have +2 might."
fn lee_sin() -> Impl {
    let aura_might = fv("_lee_aura", |g, a| {
        let (s, o) = (a[0].obj(), a[1].obj());
        let (so, oo) = (g.o(s), g.o(o));
        if is_bf(so.loc) && o != s && oo.loc == so.loc && oo.ctrl == so.ctrl && oo.buff > 0 && g.spec(o).type_ == "Unit"
        {
            return Ok(Value::Int(2));
        }
        Ok(Value::Int(0))
    });
    Impl { accelerate: true, aura_might, ..new("Lee Sin, Centered") }
}

// Legion Marauder — "[Empower] — [1] or [Body] [Empowered] I have +1 might."
fn legion_marauder() -> Impl {
    Impl {
        abilities: vec![
            empower_ability(Cost::Text("1 energy".into()), "main", false),
            empower_ability(Cost::Text("1 body rune".into()), "main", false),
        ],
        might_if: vec![(when_empowered(), 1)],
        ..new("Legion Marauder")
    }
}

// Lucian, Merciless — voir `avec_attrs::lucian`.

// Master Yi, Honed — "[Ganking] I enter ready."
fn yi_honed() -> Impl {
    Impl { kw: map! { Ganking = 1 }, enter_ready: Value::Bool(true), ..new("Master Yi, Honed") }
}

// Master Yi, Tempered — "[Hunt 2] [Level 6] I have [Deflect] and [Ganking]."
fn yi_tempered() -> Impl {
    Impl {
        kw: map! { Hunt = 2 },
        levels: vec![(6, map! { kw = map! { Deflect = 1, Ganking = 1 } })],
        ..new("Master Yi, Tempered")
    }
}

// Miss Fortune, Captain — voir `avec_attrs::miss_fortune`.

// Nidalee, Cat Form — "[Ambush] When I win a combat, draw 1." (I win if I remain after combat)
fn nidalee() -> Impl {
    let on_event = fv("on_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "combat_won" && won(g, o, info) {
            let res = fv("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.draw(c, 1)?;
                Ok(Value::None)
            });
            trig(g, o, "Nidalee, Cat Form", res);
        }
        Ok(Value::None)
    });
    Impl { ambush: true, on_event, ..new("Nidalee, Cat Form") }
}

// Nilah, Joyful Ascetic — "[Accelerate] [Ganking] When I move, gain 1 XP."
fn nilah() -> Impl {
    let on_event = fv("on_event", |g, a| {
        if let Some((o, _)) = ev_obj(a, "move") {
            let res = fv("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.gain_xp(c, 1)?;
                Ok(Value::None)
            });
            trig(g, o, "Nilah, Joyful Ascetic", res);
        }
        Ok(Value::None)
    });
    Impl { accelerate: true, kw: map! { Ganking = 1 }, on_event, ..new("Nilah, Joyful Ascetic") }
}

// Noxian Demolitionist — "When I conquer, you may kill a gear with Energy cost no more than my Might."
fn demolitionist() -> Impl {
    let on_event = fv("_demolitionist", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "conquer" && in_units(info, o) {
            let ok = pred(|g_, it, x| {
                if g_.spec(x).type_ != "Gear" {
                    return Ok(false);
                }
                let Some(me) = src(g_, it) else { return Ok(false) };
                Ok(g_.spec(x).e as i64 <= g_.might(me)?)
            });
            let opts = |g_: &mut Game, it: ItemId| -> Res<Vec<O>> {
                let Some(me) = src(g_, it) else { return Ok(vec![]) };
                let c = ctrl(g_, it);
                let mut xs = vec![];
                for x in g_.gear(None) {
                    if g_.spec(x).e as i64 <= g_.might(me)? {
                        xs.push(x);
                    }
                }
                let keys = xs
                    .iter()
                    .map(|&x| vec![bk(g_.o(x).ctrl == c), -(g_.spec(x).e as f64), g_.o(x).uid as f64])
                    .collect();
                Ok(sort_keyed(xs, keys))
            };
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let t = g_.legal(it, 0)?;
                let c = ctrl(g_, it);
                let ts: Vec<O> = t.into_iter().collect();
                g_.kill(&ts, Some(c), false)?;
                Ok(Value::None)
            });
            let choose = pick(opts, ok, 1, "gear_target", true);
            trig_full(g, o, "Noxian Demolitionist", res, Map::new(), true, choose, Value::None);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Noxian Demolitionist") }
}

// Pit Rookie — "When you play me, buff another friendly unit."
fn pit_rookie() -> Impl {
    let on_play = fv("on_play", |g, a| {
        let o = a[0].obj();
        let res = fv("res", |g_, a| {
            let t = g_.legal(a[0].item(), 0)?;
            g_.buff(t, None)?;
            Ok(Value::None)
        });
        let choose = pick(other_friends_by_buff, p_friend_v(), 1, "buff_target", false);
        trig_full(g, o, "Pit Rookie", res, Map::new(), false, choose, Value::None);
        Ok(Value::None)
    });
    Impl { on_play, ..new("Pit Rookie") }
}

// Poppy, Paragon — "[Deflect] When you play me, if an opponent's score is within 3 points of the Victory Score,
// ready me and gain 3 XP." (intervening condition checked on trigger and on resolution)
fn poppy_cond(g: &Game, pid: usize) -> bool {
    g.victory - g.p[1 - pid].points <= 3
}

fn poppy() -> Impl {
    let on_play = fv("_poppy", |g, a| {
        let o = a[0].obj();
        if !poppy_cond(g, g.o(o).ctrl) {
            return Ok(Value::None);
        }
        let res = fv("res", |g_, a| {
            let it = a[0].item();
            let c = ctrl(g_, it);
            if !poppy_cond(g_, c) {
                return Ok(Value::None);
            }
            if let Some(me) = src(g_, it) {
                g_.ready_obj(me, None, None)?;
            }
            g_.gain_xp(c, 3)?;
            Ok(Value::None)
        });
        trig(g, o, "Poppy, Paragon", res);
        Ok(Value::None)
    });
    Impl { kw: map! { Deflect = 1 }, on_play, ..new("Poppy, Paragon") }
}

// Profiteer — "When you play me, you may disempower something you control to empower a legend, unit, or gear."
// (disempowering is the cost within instructions, paid on finalization, rule 740.4.a.2)
fn profiteer() -> Impl {
    let on_play = fv("_profiteer", |g, a| {
        let o = a[0].obj();
        let choose = fv("choose", |g_, a| {
            let it = a[0].item();
            let pid = ctrl(g_, it);
            let mut srcs: Vec<O> =
                g_.board.iter().copied().filter(|&x| g_.o(x).ctrl == pid && g_.o(x).empowered).collect();
            let leg = g_.p[pid].legend;
            if g_.o(leg).empowered {
                srcs.push(leg);
            }
            if srcs.is_empty() {
                return Ok(Value::Bool(false));
            }
            let mut dsts = vec![];
            for x in g_.board.clone() {
                let t = g_.spec(x).type_.as_str();
                if !g_.o(x).empowered
                    && (t == "Unit" || t == "Gear")
                    && (g_.o(x).ctrl == pid || (g_.targetable(x, Some(pid))? && payable(g_, it, x)?))
                {
                    dsts.push(x);
                }
            }
            let mut keys = vec![];
            for &x in &dsts {
                keys.push(vec![bk(g_.o(x).ctrl != pid), -value(g_, x)?]);
            }
            let dsts = sort_keyed(dsts, keys);
            let legs: Vec<O> = g_.p.iter().map(|pl| pl.legend).filter(|&x| !g_.o(x).empowered).collect();
            let lkeys = legs.iter().map(|&x| vec![bk(g_.o(x).owner != pid)]).collect();
            let legs = sort_keyed(legs, lkeys);
            let mut all: Vec<O> = dsts.iter().copied().filter(|&x| g_.o(x).ctrl == pid).collect();
            all.extend(legs.iter().copied().filter(|&x| g_.o(x).owner == pid));
            all.extend(dsts.iter().copied().filter(|&x| g_.o(x).ctrl != pid));
            all.extend(legs.iter().copied().filter(|&x| g_.o(x).owner != pid));
            if all.is_empty() {
                return Ok(Value::Bool(false));
            }
            let mut pairs = vec![];
            for &d in &all {
                for &s in &srcs {
                    pairs.push(Value::Tuple(vec![Value::Obj(s), Value::Obj(d)]));
                }
            }
            let p = g_.ask(pid, "profiteer", pairs, map! { item = it })?;
            let (s, d) = (p.list()[0].obj(), p.list()[1].obj());
            if g_.o(d).zone == Some(Zone::Board) {
                if !pay_deflect(g_, it, d)? {
                    return Ok(Value::Bool(false));
                }
                g_.add_target(it, d, Value::None);
            } else {
                let ow = g_.o(d).owner;
                g_.it_mut(it).data.set("legend", ow);
            }
            g_.disempower(Some(s));
            Ok(Value::Bool(true))
        });
        let res = fv("res", |g_, a| {
            let it = a[0].item();
            let d = if !g_.it(it).targets.is_empty() {
                g_.legal(it, 0)?
            } else if !g_.it(it).data.g("legend").is_none() {
                Some(g_.p[g_.it(it).data.g("legend").int() as usize].legend)
            } else {
                None
            };
            if d.is_some() {
                g_.empower(d, None)?;
            }
            Ok(Value::None)
        });
        trig_full(g, o, "Profiteer", res, Map::new(), true, choose, Value::None);
        Ok(Value::None)
    });
    Impl { on_play, ..new("Profiteer") }
}

// Qiyana, Victorious — "[Deflect] When I conquer, draw 1 or channel 1 rune exhausted."
fn qiyana() -> Impl {
    let on_event = fv("_qiyana", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "conquer" && in_units(info, o) {
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let c = ctrl(g_, it);
                let mut opts: Vec<Value> = vec!["draw".into(), "channel".into()];
                if g_.p[c].runes.len() < 6 && !g_.p[c].rune_deck.is_empty() {
                    opts = vec!["channel".into(), "draw".into()];
                }
                if g_.ask(c, "qiyana_mode", opts, map! { item = it })?.opt_str() == Some("draw") {
                    g_.draw(c, 1)?;
                } else {
                    g_.channel(c, 1, true)?;
                }
                Ok(Value::None)
            });
            trig(g, o, "Qiyana, Victorious", res);
        }
        Ok(Value::None)
    });
    Impl { kw: map! { Deflect = 1 }, on_event, ..new("Qiyana, Victorious") }
}

// Repair Specialist — "I have [Assault] equal to the number of gear you control."
fn repair_specialist() -> Impl {
    Impl {
        aura_kw: fv("aura_kw", |g, a| {
            let (s, o) = (a[0].obj(), a[1].obj());
            if o == s {
                return Ok(Value::Map(map! { Assault = g.gear(Some(g.o(s).ctrl)).len() }));
            }
            Ok(Value::None)
        }),
        ..new("Repair Specialist")
    }
}

// Ruin Runner — "I can't be chosen by enemy spells and abilities."
fn ruin_runner() -> Impl {
    Impl {
        untargetable: fv("untargetable", |g, a| {
            Ok(Value::Bool(a[1].opt_int() != Some(g.o(a[0].obj()).ctrl as i64)))
        }),
        ..new("Ruin Runner")
    }
}

// Sea Monkey — "You may pay [1] as an additional cost to play me. When you play me, if you paid the additional
// cost, buff me."
fn sea_monkey() -> Impl {
    Impl {
        as_played: fv("as_played", |_g, _a| Ok(maps(vec![Map::new(), map! { paid = true }]))),
        extra_cost_fn: fv("extra_cost_fn", |_g, a| {
            Ok(if a[2].map().g("paid").truthy() { e_reqs(1, &[]) } else { e_reqs(0, &[]) })
        }),
        on_play: fv("on_play", |g, a| {
            let o = a[0].obj();
            if a[1].map().g("paid").truthy() {
                let res = fv("res", |g_, a| {
                    let s = src(g_, a[0].item());
                    g_.buff(s, None)?;
                    Ok(Value::None)
                });
                trig(g, o, "Sea Monkey", res);
            }
            Ok(Value::None)
        }),
        ..new("Sea Monkey")
    }
}

// Sett, Brawler — "When I'm played and when I conquer, buff me. Spend my buff: Give me +4 might this turn."
fn sett() -> Impl {
    let buff_me = || {
        fv("res", |g_, a| {
            let s = src(g_, a[0].item());
            g_.buff(s, None)?;
            Ok(Value::None)
        })
    };
    let on_play = fv("on_play", move |g, a| {
        let o = a[0].obj();
        trig(g, o, "Sett, Brawler", buff_me());
        Ok(Value::None)
    });
    let on_event = fv("on_event", move |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "conquer" && in_units(info, o) {
            trig(g, o, "Sett, Brawler (conquer)", buff_me());
        }
        Ok(Value::None)
    });
    Impl {
        on_play,
        on_event,
        abilities: vec![ability(
            "Spend buff",
            AbilityArgs {
                can: fv("can", |g, a| Ok(Value::Bool(g.o(a[1].obj()).buff > 0))),
                extra_cost: fv("extra_cost", |g, a| {
                    let o = a[1].obj();
                    Ok(Value::Bool(spend_buff(g, Some(o))?))
                }),
                resolve: fv("res", |g, a| {
                    if let Some(s) = src_board(g, a[0].item()) {
                        g.mod_(s, 4, turn(), None)?;
                    }
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..new("Sett, Brawler")
    }
}

// Sivir, Ambitious — voir `avec_attrs::sivir`.

// Stormclaw Ursine — "[Tank] When you play me, channel 1 rune exhausted."
fn stormclaw() -> Impl {
    let on_play = fv("on_play", |g, a| {
        let o = a[0].obj();
        let res = fv("res", |g_, a| {
            let c = ctrl(g_, a[0].item());
            g_.channel(c, 1, true)?;
            Ok(Value::None)
        });
        trig(g, o, "Stormclaw Ursine", res);
        Ok(Value::None)
    });
    Impl { kw: map! { Tank = 1 }, on_play, ..new("Stormclaw Ursine") }
}

// Targonian Visionary — "[Level 11] I have +4 might."
fn targonian() -> Impl {
    Impl { levels: vec![(11, map! { might = 4 })], ..new("Targonian Visionary") }
}

// Udyr, Wildman — voir `avec_attrs::udyr`.

// Volibear, Imposing — "[Shield 3] [Tank] When an opponent moves to a battlefield other than mine, draw 1."
// Rules decision: one trigger per move performed by an opponent (units moved together count once).
fn volibear() -> Impl {
    let on_event = fv("_volibear", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        let to = info.g("to");
        if ev == "move"
            && info.g("by").opt_int() == Some(1 - g.o(o).ctrl as i64)
            && is_bf(to.opt_loc())
            && to.opt_loc() != g.o(o).loc
        {
            let uid = g.o(o).uid;
            for &t in &g.trigq {
                let ti = g.it(t);
                if ti.src == Some(uid) && ti.name == "Volibear, Imposing" && ti.data.g("to") == to {
                    return Ok(Value::None);
                }
            }
            let res = fv("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.draw(c, 1)?;
                Ok(Value::None)
            });
            trig_full(g, o, "Volibear, Imposing", res, map! { to = to }, false, Value::None, Value::None);
        }
        Ok(Value::None)
    });
    Impl { kw: map! { Shield = 3, Tank = 1 }, on_event, ..new("Volibear, Imposing") }
}

// Warwick, Hunter — "I enter ready. When I attack, kill all damaged enemy units here."
fn warwick() -> Impl {
    let on_event = fv("_warwick", |g, a| {
        if let Some((o, _)) = ev_obj(a, "attack") {
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let me = src(g_, it);
                let here = match me {
                    Some(m) => g_.o(m).loc,
                    None => g_.it(it).data.g("loc").opt_loc(),
                };
                let c = ctrl(g_, it);
                let ks: Vec<O> = units_at(g_, Some(1 - c), here).into_iter().filter(|&u| g_.o(u).damage > 0).collect();
                g_.kill(&ks, Some(c), false)?;
                Ok(Value::None)
            });
            let loc = g.o(o).loc;
            trig_full(g, o, "Warwick, Hunter", res, map! { loc = loc }, false, Value::None, Value::None);
        }
        Ok(Value::None)
    });
    Impl { enter_ready: Value::Bool(true), on_event, ..new("Warwick, Hunter") }
}

// Wildclaw Shaman — "When you play me, you may spend a buff to buff me and ready me."
fn wildclaw_shaman() -> Impl {
    let on_play = fv("_shaman", |g, a| {
        let o = a[0].obj();
        let cost = fv("cost", |g_, a| {
            let it = a[0].item();
            let c = ctrl(g_, it);
            let s = g_.it(it).src;
            let bs: Vec<O> = g_.units(Some(c), None).into_iter().filter(|&u| g_.o(u).buff > 0).collect();
            let mut keys = vec![];
            for &u in &bs {
                keys.push(vec![bk(Some(g_.o(u).uid) == s), value(g_, u)?]);
            }
            let bs = sort_keyed(bs, keys);
            if bs.is_empty() {
                return Ok(Value::Bool(false));
            }
            let u = g_.ask(c, "spend_buff", ovals(&bs), map! { item = it })?.opt_obj();
            Ok(Value::Bool(spend_buff(g_, u)?))
        });
        let res = fv("res", |g_, a| {
            if let Some(me) = src(g_, a[0].item()) {
                g_.buff(Some(me), None)?;
                g_.ready_obj(me, None, None)?;
            }
            Ok(Value::None)
        });
        trig_full(g, o, "Wildclaw Shaman", res, Map::new(), true, Value::None, cost);
        Ok(Value::None)
    });
    Impl { on_play, ..new("Wildclaw Shaman") }
}

// Yone, Blademaster — "[Weaponmaster] When I conquer an open battlefield, deal damage equal to my Might to an enemy
// unit in a base." Open = unoccupied and uncontrolled (rule 170.11.c): in 1v1 that is a conquer that does not
// follow a combat (an empty battlefield loses its controller at the next cleanup, rule 323).
fn yone() -> Impl {
    let on_event = fv("_yone", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "conquer" && in_units(info, o) && g.sd.as_ref().is_none_or(|sd| !sd.combat) {
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                let me = src(g_, it);
                let u = g_.legal(it, 0)?;
                if let (Some(me), Some(u)) = (me, u) {
                    let m = 0.max(g_.might(me)?);
                    let c = ctrl(g_, it);
                    g_.deal(Some(u), m, "ability", Some(c), None)?;
                }
                Ok(Value::None)
            });
            let dmg = g.might(o)?;
            let choose = pick(
                |g_, it| {
                    let c = ctrl(g_, it);
                    Ok(enemies(g_, c, false, None, true)?
                        .into_iter()
                        .filter(|&u| g_.o(u).loc == Some(Loc::Base))
                        .collect())
                },
                pred(|g_, it, u| Ok(g_.o(u).ctrl != ctrl(g_, it) && g_.o(u).loc == Some(Loc::Base))),
                1,
                "target",
                true,
            );
            trig_full(g, o, "Yone, Blademaster", res, map! { dmg = dmg }, false, choose, Value::None);
        }
        Ok(Value::None)
    });
    Impl { kw: map! { Weaponmaster = 1 }, on_event, ..new("Yone, Blademaster") }
}

// Yordle Explorer — "When you play a card with Power cost [A][A] or more, draw 1." (printed cost, rule 206)
fn yordle_explorer() -> Impl {
    let on_event = fv("on_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "played"
            && info.g("pid").opt_int() == Some(g.o(o).ctrl as i64)
            && g.spec(info.g("card").obj()).p >= 2
        {
            let res = fv("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.draw(c, 1)?;
                Ok(Value::None)
            });
            trig(g, o, "Yordle Explorer", res);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Yordle Explorer") }
}

// ====================================================================== équipements
// Arena Bar — "[E]: Buff an exhausted friendly unit."
fn arena_bar() -> Impl {
    Impl {
        abilities: vec![ability(
            "Buff",
            AbilityArgs {
                exhaust: true,
                preds: Value::List(vec![pred(|g, it, o| Ok(p_friend(g, it, o) && g.exhausted(o)))]),
                choices: fv("choices", |g, a| {
                    let pid = a[0].int() as usize;
                    let us: Vec<O> = g.units(Some(pid), None).into_iter().filter(|&u| g.exhausted(u)).collect();
                    let mut keys = vec![];
                    for &u in &us {
                        keys.push(vec![bk(g.o(u).buff > 0), -value(g, u)?]);
                    }
                    let us = sort_keyed(us, keys);
                    Ok(maps(tg_choices(g, &us)))
                }),
                resolve: fv("res", |g, a| {
                    let t = g.legal(a[0].item(), 0)?;
                    g.buff(t, None)?;
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..new("Arena Bar")
    }
}

// Blood Rose — "When you play a unit, you may pay [1] to gain 1 XP. Spend 3 XP, [E]: Ready a unit."
fn blood_rose() -> Impl {
    let on_event = fv("_blood_rose", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "played"
            && info.g("pid").opt_int() == Some(g.o(o).ctrl as i64)
            && g.spec(info.g("card").obj()).type_ == "Unit"
        {
            let res = fv("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.gain_xp(c, 1)?;
                Ok(Value::None)
            });
            trig_full(g, o, "Blood Rose", res, Map::new(), true, Value::None, may_pay(1, Value::None));
        }
        Ok(Value::None)
    });
    Impl {
        on_event,
        abilities: vec![ability(
            "Ready",
            AbilityArgs {
                xp: 3,
                exhaust: true,
                preds: Value::List(vec![p_unit_v()]),
                choices: fv("choices", |g, a| {
                    let pid = a[0].int() as usize;
                    let mut us = vec![];
                    for u in g.units(None, None) {
                        if g.o(u).ctrl == pid || g.targetable(u, Some(pid))? {
                            us.push(u);
                        }
                    }
                    let mut keys = vec![];
                    for &u in &us {
                        keys.push(vec![bk(g.o(u).ctrl != pid), bk(!g.exhausted(u)), -value(g, u)?]);
                    }
                    let us = sort_keyed(us, keys);
                    Ok(maps(tg_choices(g, &us)))
                }),
                resolve: fv("res", |g, a| {
                    let it = a[0].item();
                    if g.legal(it, 0)?.is_some() {
                        if let Some(t) = g.legal(it, 0)? {
                            g.ready_obj(t, None, None)?;
                        }
                    }
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..new("Blood Rose")
    }
}

// Equipment: Might Bonus and Effect Text read on the card images (not in the card database).
fn equipment(name: &str, cost: &str, bonus: i64) -> Impl {
    let mut im = Impl { bonus: Some(bonus), ..new(name) };
    impl_equip(&mut im, Cost::Text(cost.into()));
    im
}

// Boneshiver — "[Equip] [1][Body]", +2, "When I conquer, channel 1 rune exhausted."
fn boneshiver() -> Impl {
    let effect_event = fv("_boneshiver", |g, a| {
        let (gear, unit, ev, info) = (a[0].obj(), a[1].obj(), a[2].str(), a[3].map());
        if ev == "conquer" && in_units(info, unit) {
            let res = fv("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.channel(c, 1, true)?;
                Ok(Value::None)
            });
            let (c, gu) = (g.o(unit).ctrl, g.o(gear).uid);
            g.queue_trigger(c, "Boneshiver", res, Map::new(), Some(gu), false, Value::None, Value::None);
        }
        Ok(Value::None)
    });
    Impl { effect_event, ..equipment("Boneshiver", "1 energy and 1 body rune", 2) }
}

// Doran's Blade — "[Equip] [Body]", +2
// Hexdrinker — "[Equip] [Body]", +1, "[Deflect]"
// Hunter's Machete — "[Equip] [Body]", +2, "[Hunt]"

// Trinity Force — "[Equip] [Body]", +2, "When I hold, score 1 point." (a point not gained through Conquer: no
// Final Point restriction, rule 471.1.a.1)
fn trinity() -> Impl {
    let effect_event = fv("_trinity", |g, a| {
        let (gear, unit, ev, info) = (a[0].obj(), a[1].obj(), a[2].str(), a[3].map());
        if ev == "hold" && in_units(info, unit) {
            let res = fv("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                g_.gain_point(c, "Trinity Force")?;
                Ok(Value::None)
            });
            let (c, gu) = (g.o(unit).ctrl, g.o(gear).uid);
            g.queue_trigger(c, "Trinity Force", res, Map::new(), Some(gu), false, Value::None, Value::None);
        }
        Ok(Value::None)
    });
    Impl { effect_event, ..equipment("Trinity Force", "1 body rune", 2) }
}

// Warmog's Armor — "[Equip] [Body]", +1, "When I conquer, buff me."
fn warmog() -> Impl {
    let effect_event = fv("_warmog", |g, a| {
        let (gear, unit, ev, info) = (a[0].obj(), a[1].obj(), a[2].str(), a[3].map());
        if ev == "conquer" && in_units(info, unit) {
            let res = fv("_warmog_res", |g_, a| {
                let it = a[0].item();
                let d = g_.it(it).data.clone();
                if let Some(u) = g_.obj(d.g("u").int() as u32) {
                    if g_.o(u).oid == d.g("uoid").int() {
                        g_.buff(Some(u), None)?;
                    }
                }
                Ok(Value::None)
            });
            let (c, gu, uu, uo) = (g.o(unit).ctrl, g.o(gear).uid, g.o(unit).uid, g.o(unit).oid);
            g.queue_trigger(c, "Warmog's Armor", res, map! { u = uu, uoid = uo }, Some(gu), false, Value::None, Value::None);
        }
        Ok(Value::None)
    });
    Impl { effect_event, ..equipment("Warmog's Armor", "1 body rune", 1) }
}

// Hextech Disc — "[Empower] — [E] Disempower this, [1], [E]: Play a 3 might Mech unit token to your base."
fn hextech_disc() -> Impl {
    Impl {
        abilities: vec![
            empower_ability(Cost::None, "main", true),
            ability(
                "Mech",
                AbilityArgs {
                    cost: Cost::Text("1 energy".into()),
                    exhaust: true,
                    can: fv("can", |g, a| Ok(Value::Bool(g.o(a[1].obj()).empowered))),
                    extra_cost: fv("extra_cost", |g, a| Ok(Value::Bool(g.disempower(Some(a[1].obj()))))),
                    resolve: fv("res", |g, a| {
                        let c = ctrl(g, a[0].item());
                        make_token(g, "Mech", c, Loc::Base, false)?;
                        Ok(Value::None)
                    }),
                    ..Default::default()
                },
            ),
        ],
        ..new("Hextech Disc")
    }
}

// Mistfall — "When you buff a friendly unit, you may pay [Body] and exhaust this to ready it." (the 'buff' event names
// the player who buffs)
fn mistfall() -> Impl {
    let on_event = fv("_mistfall", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev != "buff" {
            return Ok(Value::None);
        }
        let u = info.g("obj").obj();
        let oc = g.o(o).ctrl as i64;
        let by = if info.contains("by") { info.g("by") } else { Value::Int(oc) };
        if g.o(u).ctrl as i64 == oc && by == Value::Int(oc) && !g.exhausted(o) {
            let cost = fv("cost", |g_, a| {
                let it = a[0].item();
                let me = src(g_, it);
                let c = ctrl(g_, it);
                let Some(me) = me else { return Ok(Value::Bool(false)) };
                if g_.exhausted(me) || !g_.can_pay(c, 0, &[body_req()], &Value::None)? {
                    return Ok(Value::Bool(false));
                }
                g_.pay(c, 0, &[body_req()], &Value::None)?;
                g_.set_exhausted(me, true);
                Ok(Value::Bool(true))
            });
            let res = fv("res", |g_, a| {
                let d = g_.it(a[0].item()).data.clone();
                if let Some(x) = g_.obj(d.g("u").int() as u32) {
                    if g_.o(x).oid == d.g("uoid").int() {
                        g_.ready_obj(x, None, None)?;
                    }
                }
                Ok(Value::None)
            });
            let (uu, uo) = (g.o(u).uid, g.o(u).oid);
            trig_full(g, o, "Mistfall", res, map! { u = uu, uoid = uo }, true, Value::None, cost);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Mistfall") }
}

// Petricite Monument — "[Temporary] Friendly units have [Deflect]."
fn petricite() -> Impl {
    Impl {
        kw: map! { Temporary = 1 },
        aura_kw: fv("aura_kw", |g, a| {
            let (s, o) = (a[0].obj(), a[1].obj());
            if g.o(o).ctrl == g.o(s).ctrl && g.spec(o).type_ == "Unit" {
                return Ok(Value::Map(map! { Deflect = 1 }));
            }
            Ok(Value::None)
        }),
        ..new("Petricite Monument")
    }
}

// Platewyrm Egg — "This enters exhausted. [Empower] — [1], [E] [Reaction][>] [E]: [Add] [1]. If this is
// [Empowered], [Add] [2] instead."
fn platewyrm() -> Impl {
    Impl {
        enter_exhausted: true,
        abilities: vec![empower_ability(Cost::Text("1 energy".into()), "main", true)],
        add: vec![
            add_ability("1 energy", true, false, fv("can", |g, a| Ok(Value::Bool(!g.o(a[1].obj()).empowered)))),
            add_ability("2 energy", true, false, fv("can", |g, a| Ok(Value::Bool(g.o(a[1].obj()).empowered)))),
        ],
        ..new("Platewyrm Egg")
    }
}

// Seal of Strength — "[E]: [Reaction] — [Add] [Body]."
fn seal_of_strength() -> Impl {
    Impl { add: vec![add_ability("1 body rune", true, false, Value::None)], ..new("Seal of Strength") }
}

// Tools of Empire — "[Empower] [2] [E]: Give a unit +2 might this turn. If this is [Empowered], give that unit +4
// might this turn instead."
fn tools_of_empire() -> Impl {
    let mut im = Impl {
        abilities: vec![ability(
            "Might",
            AbilityArgs {
                exhaust: true,
                preds: Value::List(vec![p_unit_v()]),
                resolve: fv("_tools", |g, a| {
                    let it = a[0].item();
                    let u = g.legal(it, 0)?;
                    let me = src_board(g, it);
                    if let Some(u) = u {
                        let n = if me.is_some_and(|m| g.o(m).empowered) { 4 } else { 2 };
                        g.mod_(u, n, turn(), None)?;
                    }
                    Ok(Value::None)
                }),
                choices: fv("choices", |g, a| {
                    let pid = a[0].int() as usize;
                    let mut us = friends(g, pid, true, None)?;
                    us.extend(friends(g, pid, false, None)?.into_iter().filter(|&u| g.o(u).loc == Some(Loc::Base)));
                    Ok(maps(tg_choices(g, &us)))
                }),
                ..Default::default()
            },
        )],
        ..new("Tools of Empire")
    };
    impl_empower(&mut im, Cost::Text("2 energy".into()));
    im
}

// ====================================================================== sorts
// Bullet Time — "[Action] Pay any amount of [A] to deal that much damage to all enemy units at a battlefield."
// (cost within instructions of a spell: paid on resolution, rule 740.4.a.1)
fn bullet_time() -> Impl {
    let resolve = fv("_bullet", |g, a| {
        let it = a[0].item();
        let pid = ctrl(g, it);
        let bf = bfi(&g.it(it).data.g("bf"));
        let mut mx = 0usize;
        while mx < 12 && g.can_pay(pid, 0, &vec![any_req(); mx + 1], &Value::None)? {
            mx += 1;
        }
        let es = g.units(Some(1 - pid), Some(Loc::Bf(bf)));
        let mut need = 0;
        for &u in &es {
            need = need.max(g.might(u)? - g.o(u).damage);
        }
        if g.bfs[bf].name == "Void Gate" {
            need -= 1;
        }
        let mut opts: Vec<i64> = (0..=mx as i64).rev().collect();
        if 0 < need && need <= mx as i64 {
            let i = opts.iter().position(|&x| x == need).unwrap();
            opts.remove(i);
            opts.insert(0, need);
        }
        let x = g.ask(pid, "bullet_time_x", opts.into_iter().map(Value::Int).collect(), map! { item = it })?;
        if !x.truthy() {
            return Ok(Value::None);
        }
        let x = x.int();
        if !g.pay(pid, 0, &vec![any_req(); x as usize], &Value::None)? {
            return Ok(Value::None);
        }
        for u in g.units(Some(1 - pid), Some(Loc::Bf(bf))) {
            g.deal(Some(u), x, "spell", Some(pid), None)?;
        }
        Ok(Value::None)
    });
    let choices = fv("choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let out: Vec<Map> = (0..g.bfs.len())
            .filter(|&i| hb.is_none() || hb == Some(Loc::Bf(i)))
            .map(|i| map! { bf = g.bfs[i].idx })
            .collect();
        let keys = out.iter().map(|c| vec![-(g.units(Some(1 - pid), Some(Loc::Bf(bfi(&c.g("bf"))))).len() as f64)]).collect();
        Ok(maps(sort_keyed(out, keys)))
    });
    Impl { timing: Some("action"), resolve, choices, ..new("Bullet Time") }
}

// Call to Battle — "Move a unit you control to a battlefield you control. Then, choose an opponent. They move a
// unit they control to the same battlefield."
fn call_to_battle() -> Impl {
    let resolve = fv("_call", |g, a| {
        let it = a[0].item();
        let dest = g.it(it).data.g("dest").loc();
        let c = ctrl(g, it);
        let u = g.legal(it, 0)?;
        if let Some(u) = u {
            if g.bfs[dest.bf().unwrap()].ctrl == Some(c) && g.o(u).loc != Some(dest) {
                g.move_(&[u], dest, Some(c), false)?;
            }
        }
        let opp = 1 - c;
        let us: Vec<O> = g.units(Some(opp), None).into_iter().filter(|&x| g.o(x).loc != Some(dest)).collect();
        let mut keys = vec![];
        for &x in &us {
            keys.push(vec![-value(g, x)?]);
        }
        let us = sort_keyed(us, keys);
        if !us.is_empty() {
            let x = g.ask(opp, "call_to_battle", ovals(&us), map! { item = it })?.obj();
            g.move_(&[x], dest, Some(opp), false)?;
        }
        Ok(Value::None)
    });
    let choices = fv("choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let mut out = vec![];
        for i in 0..g.bfs.len() {
            if g.bfs[i].ctrl == Some(pid) {
                for u in friends(g, pid, false, hb)? {
                    if g.o(u).loc != Some(Loc::Bf(i)) {
                        out.push(map! { tg = tup(&[g.o(u).uid]), dest = Loc::Bf(g.bfs[i].idx) });
                    }
                }
            }
        }
        Ok(maps(out))
    });
    Impl { preds: Value::List(vec![p_friend_v()]), resolve, choices, ..new("Call to Battle") }
}

// Cannon Barrage — "[Reaction] Deal 2 to all enemy units in combat."
fn cannon_barrage() -> Impl {
    Impl {
        timing: Some("reaction"),
        resolve: fv("res", |g, a| {
            let c = ctrl(g, a[0].item());
            for u in g.units(Some(1 - c), None) {
                if g.in_combat(u) {
                    g.deal(Some(u), 2, "spell", Some(c), None)?;
                }
            }
            Ok(Value::None)
        }),
        ..new("Cannon Barrage")
    }
}

// Cataclysmic Duel — "Each player chooses a unit they control. Kill the rest."
fn cataclysmic_duel() -> Impl {
    Impl {
        resolve: fv("_cataclysm", |g, a| {
            let it = a[0].item();
            let mut keep = vec![];
            for pid in [g.tp, 1 - g.tp] {
                let us = g.units(Some(pid), None);
                let mut keys = vec![];
                for &u in &us {
                    keys.push(vec![-value(g, u)?]);
                }
                let us = sort_keyed(us, keys);
                if !us.is_empty() {
                    keep.push(g.ask(pid, "duel_keep", ovals(&us), map! { item = it })?.opt_obj());
                }
            }
            let ks: Vec<O> = g.units(None, None).into_iter().filter(|u| !keep.contains(&Some(*u))).collect();
            let c = ctrl(g, it);
            g.kill(&ks, Some(c), false)?;
            Ok(Value::None)
        }),
        ..new("Cataclysmic Duel")
    }
}

// Catalyst of Aeons — "Channel 2 runes exhausted. If you couldn't channel 2 runes this way, draw 1."
fn catalyst() -> Impl {
    Impl {
        resolve: fv("res", |g, a| {
            let c = ctrl(g, a[0].item());
            if g.channel(c, 2, true)?.len() < 2 {
                g.draw(c, 1)?;
            }
            Ok(Value::None)
        }),
        ..new("Catalyst of Aeons")
    }
}

/// `lambda g, it: _fight(g, g.legal(it, 0), g.legal(it, 1))`
fn fight_targets() -> Value {
    fv("res", |g, a| {
        let it = a[0].item();
        let x = g.legal(it, 0)?;
        let y = g.legal(it, 1)?;
        fight(g, x, y)?;
        Ok(Value::None)
    })
}

// Challenge — "[Action] Choose a friendly unit and an enemy unit. They deal damage equal to their Mights to each
// other."
fn challenge() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_friend_v(), p_enemy_v()]),
        resolve: fight_targets(),
        choices: fv("choices", |g, a| {
            let pid = a[0].int() as usize;
            let hb = hbv(&a[1].map().g("hidden_bf"));
            let fs = friends(g, pid, false, hb)?;
            let es = enemies(g, pid, false, hb, true)?;
            Ok(maps(duel_choices(g, pid, &fs, &es, 0, None)?))
        }),
        ..new("Challenge")
    }
}

// Clash of Giants — "Choose two units. They deal damage equal to their Mights to each other."
fn clash_of_giants() -> Impl {
    let choices = fv("_clash_choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let es0 = enemies(g, pid, false, hb, true)?;
        let es = cap(g, es0, 4, Some(pid));
        let fs0 = friends(g, pid, false, hb)?;
        let fs = cap(g, fs0, 3, Some(pid));
        let mut out = vec![];
        for (i, &x) in es.iter().enumerate() {
            for &y in &es[i + 1..] {
                out.push(map! { tg = tup(&[g.o(x).uid, g.o(y).uid]) });
            }
        }
        out.extend(duel_choices(g, pid, &fs, &es, 0, None)?);
        Ok(maps(out))
    });
    Impl {
        preds: Value::List(vec![p_unit_v(), p_unit_v()]),
        resolve: fight_targets(),
        choices,
        ..new("Clash of Giants")
    }
}

// Concentrate — "Draw 2. [Level 6] This costs [2] less. [Level 11] This costs [4] less instead."
fn concentrate() -> Impl {
    Impl {
        resolve: fv("res", |g, a| {
            let c = ctrl(g, a[0].item());
            g.draw(c, 2)?;
            Ok(Value::None)
        }),
        cost_mod: fv("cost_mod", |g, a| {
            let xp = g.p[a[0].int() as usize].xp;
            Ok(de_dp(if xp >= 11 { 4 } else if xp >= 6 { 2 } else { 0 }, 0))
        }),
        ..new("Concentrate")
    }
}

// Confront — "[Action] Units you play this turn enter ready. Draw 1." (a replacement of the way they enter, rule
// 369.3: g.effects kind='enter_ready', read as each unit or unit token enters)
fn confront() -> Impl {
    Impl {
        timing: Some("action"),
        resolve: fv("_confront", |g, a| {
            let c = ctrl(g, a[0].item());
            let f = fv("_confront_ready", |g_, a| {
                let eid = a[0].int() as u64;
                let (pid, card) = (a[1].int(), a[2].obj());
                let ep = g_.eff(eid).map(|e| e.g("pid")).unwrap_or_default();
                Ok(Value::Bool(Value::Int(pid) == ep && g_.spec(card).type_ == "Unit"))
            });
            g.add_effect(eff_map(vec![("kind", "enter_ready".into()), ("fn", f), ("dur", turn()), ("pid", c.into())]));
            g.draw(c, 1)?;
            Ok(Value::None)
        }),
        ..new("Confront")
    }
}

// Decisive Strike — "[Action] Give friendly units +2 might this turn."
fn decisive_strike() -> Impl {
    Impl {
        timing: Some("action"),
        resolve: fv("res", |g, a| {
            let c = ctrl(g, a[0].item());
            for u in g.units(Some(c), None) {
                g.mod_(u, 2, turn(), None)?;
            }
            Ok(Value::None)
        }),
        ..new("Decisive Strike")
    }
}

/// `_reveal_recycle(g, it, ok, kind)`
fn reveal_recycle(g: &mut Game, it: ItemId, ok: impl Fn(&Game, O) -> bool, kind: &str) -> Res<()> {
    let c = ctrl(g, it);
    let opp = 1 - c;
    g.reveal_hand(opp, c)?;
    let cs: Vec<O> = g.p[opp].hand.iter().copied().filter(|&x| ok(g, x)).collect();
    let keys = cs
        .iter()
        .map(|&x| vec![-((g.spec(x).e + 2 * g.spec(x).p) as f64), g.o(x).uid as f64])
        .collect();
    let cs = sort_keyed(cs, keys);
    if !cs.is_empty() {
        let x = g.ask(c, kind, ovals(&cs), map! { item = it })?.obj();
        g.recycle_cards(opp, &[x])?;
    }
    Ok(())
}

// Decree of Strength — "Choose an opponent. They reveal their hand and you choose a Mind card from it. They
// recycle that card."
fn decree_of_strength() -> Impl {
    Impl {
        resolve: fv("res", |g, a| {
            reveal_recycle(g, a[0].item(), |g_, x| g_.spec(x).domains.iter().any(|d| d == "Mind"), "decree_strength")?;
            Ok(Value::None)
        }),
        ..new("Decree of Strength")
    }
}

// Disposal Order — "[Reaction] Choose one — Choose up to 3 cards from opponents' trashes. Their owners recycle
// them. / Draw 1."
fn disposal_order() -> Impl {
    let resolve = fv("_disposal", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        let d = g.it(it).data.clone();
        if d.g("mode").opt_str() == Some("draw") {
            g.draw(c, 1)?;
            return Ok(Value::None);
        }
        let opp = 1 - c;
        let uids = d.g("cards");
        let cs: Vec<O> = g.p[opp]
            .trash
            .iter()
            .copied()
            .filter(|&x| uids.list().iter().any(|u| u.int() == g.o(x).uid as i64))
            .collect();
        if !cs.is_empty() {
            g.recycle_cards(opp, &cs)?;
        }
        Ok(Value::None)
    });
    let choices = fv("_disposal_choices", |g, a| {
        let pid = a[0].int() as usize;
        let mut out = vec![map! { mode = "draw" }];
        let tr: Vec<O> = g.p[1 - pid].trash.clone();
        let keys = tr
            .iter()
            .map(|&x| {
                vec![
                    bk(g.impl_o(x).is_none_or(|im| !im.flow.truthy())),
                    -((g.spec(x).e + 2 * g.spec(x).p) as f64),
                    g.o(x).uid as f64,
                ]
            })
            .collect();
        let tr = sort_keyed(tr, keys);
        // « up to 3 cards » comprend zéro (règle 355.13)
        let zero = map! { mode = "recycle", cards = Value::Tuple(vec![]) };
        if !tr.is_empty() && full_choices(g, Some(pid)) {
            // un humain : chaque ensemble d'au plus 3 cartes
            for n in 1..=3.min(tr.len()) {
                for grp in combinations(tr.len(), n) {
                    let uids: Vec<u32> = grp.iter().map(|&i| g.o(tr[i]).uid).collect();
                    out.push(map! { mode = "recycle", cards = tup(&uids) });
                }
            }
            out.push(zero);
            return Ok(maps(out));
        }
        if !tr.is_empty() {
            let uids: Vec<u32> = tr.iter().take(3).map(|&x| g.o(x).uid).collect();
            out.push(map! { mode = "recycle", cards = tup(&uids) });
            if tr.len() > 1 {
                out.push(map! { mode = "recycle", cards = tup(&[g.o(tr[0]).uid]) });
            }
        }
        out.push(zero);
        Ok(maps(out))
    });
    Impl { timing: Some("reaction"), resolve, choices, ..new("Disposal Order") }
}

// Flurry of Blades — "[Reaction] Deal 1 to all units at battlefields."
fn flurry() -> Impl {
    Impl {
        timing: Some("reaction"),
        resolve: fv("res", |g, a| {
            let c = ctrl(g, a[0].item());
            for u in g.units(None, None) {
                if is_bf(g.o(u).loc) {
                    g.deal(Some(u), 1, "spell", Some(c), None)?;
                }
            }
            Ok(Value::None)
        }),
        ..new("Flurry of Blades")
    }
}

// Gentlemen's Duel — "[Action] Give a friendly unit +3 might this turn. Then choose an enemy unit. They deal damage
// equal to their Mights to each other." (both chosen as the spell is played, rule 355)
fn gentlemens_duel() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_friend_v(), p_enemy_v()]),
        resolve: fv("_gduel", |g, a| {
            let it = a[0].item();
            let f = g.legal(it, 0)?;
            if let Some(f) = f {
                g.mod_(f, 3, turn(), None)?;
            }
            let e = g.legal(it, 1)?;
            fight(g, f, e)?;
            Ok(Value::None)
        }),
        choices: fv("choices", |g, a| {
            let pid = a[0].int() as usize;
            let hb = hbv(&a[1].map().g("hidden_bf"));
            let fs = friends(g, pid, false, hb)?;
            let es = enemies(g, pid, false, hb, true)?;
            Ok(maps(duel_choices(g, pid, &fs, &es, 3, None)?))
        }),
        ..new("Gentlemen's Duel")
    }
}

/// `lambda g, pid, ctx: tg_choices(friends(g, pid, False, ctx["hidden_bf"]))`
fn friend_choices() -> Value {
    fv("choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let us = friends(g, pid, false, hb)?;
        Ok(maps(tg_choices(g, &us)))
    })
}

// Grim Resolve — "[Action] Give a friendly unit +3 might this turn. When it wins a combat this turn, gain 2 XP."
fn grim_resolve() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_friend_v()]),
        resolve: fv("_grim", |g, a| {
            let it = a[0].item();
            let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
            g.mod_(u, 3, turn(), None)?;
            let w = fv("won", |g_, a| {
                let eid = a[0].int() as u64;
                let info = a[1].map();
                let eff = g_.eff(eid).cloned().unwrap_or_default();
                if let Some(x) = g_.obj(eff.g("uid").int() as u32) {
                    if g_.o(x).oid == eff.g("oid").int() && won(g_, x, info) {
                        let res = fv("res", |g2, a| {
                            let c = ctrl(g2, a[0].item());
                            g2.gain_xp(c, 2)?;
                            Ok(Value::None)
                        });
                        let (c, xu) = (g_.o(x).ctrl, g_.o(x).uid);
                        g_.queue_trigger(c, "Grim Resolve", res, Map::new(), Some(xu), false, Value::None, Value::None);
                    }
                }
                Ok(Value::None)
            });
            let (uu, uo) = (g.o(u).uid, g.o(u).oid);
            g.add_effect(eff_map(vec![
                ("on", "combat_won".into()),
                ("fn", w),
                ("dur", turn()),
                ("uid", uu.into()),
                ("oid", uo.into()),
            ]));
            Ok(Value::None)
        }),
        choices: friend_choices(),
        ..new("Grim Resolve")
    }
}

// Guttural Roar — "[Action] Give a unit +2 might this turn. If it's [Empowered], give it +4 might this turn instead."
fn guttural_roar() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_v()]),
        resolve: fv("_roar", |g, a| {
            if let Some(u) = g.legal(a[0].item(), 0)? {
                let n = if g.o(u).empowered { 4 } else { 2 };
                g.mod_(u, n, turn(), None)?;
            }
            Ok(Value::None)
        }),
        choices: fv("choices", |g, a| {
            let pid = a[0].int() as usize;
            let hb = hbv(&a[1].map().g("hidden_bf"));
            let us = friends(g, pid, false, hb)?;
            let mut keys = vec![];
            for &u in &us {
                keys.push(vec![bk(!g.o(u).empowered), -value(g, u)?]);
            }
            let us = sort_keyed(us, keys);
            Ok(maps(tg_choices(g, &us)))
        }),
        ..new("Guttural Roar")
    }
}

/// `(c, ch)` : clé de tri `-(e + 2p)` d'une carte
fn card_weight(g: &Game, c: O) -> f64 {
    -((g.spec(c).e + 2 * g.spec(c).p) as f64)
}

// Here to Help — "[Hidden] [Action] You may play a unit from hand to a battlefield you control, reducing its cost
// by [3]."
fn here_to_help() -> Impl {
    let resolve = fv("_here_to_help", |g, a| {
        let it = a[0].item();
        let pid = ctrl(g, it);
        let locs: Vec<Loc> = (0..g.bfs.len()).filter(|&i| g.bfs[i].ctrl == Some(pid)).map(|i| Loc::Bf(g.bfs[i].idx)).collect();
        if locs.is_empty() {
            return Ok(Value::None);
        }
        let mut opts: Vec<(O, Map)> = vec![];
        let mut seen: Vec<String> = vec![];
        for c in g.p[pid].hand.clone() {
            if g.spec(c).type_ != "Unit" || seen.iter().any(|n| n == g.cname(c)) || g.impl_o(c).is_none() {
                continue;
            }
            seen.push(g.cname(c).to_string());
            for ch in unit_play_choices(g, pid, c, "hand", Some(locs.clone()))? {
                let (e, reqs) = reduced_cost(g, pid, c, &ch, "hand", 3)?;
                if g.can_pay(pid, e, &reqs, &Value::Map(map! { kind = "unit", card = c }))? {
                    opts.push((c, ch));
                }
            }
        }
        if opts.is_empty() {
            return Ok(Value::None);
        }
        let keys = opts
            .iter()
            .map(|(c, ch)| vec![card_weight(g, *c), bk(ch.g("acc").truthy()), g.o(*c).uid as f64])
            .collect();
        let opts = sort_keyed(opts, keys);
        let mut ov: Vec<Value> =
            opts.into_iter().map(|(c, ch)| Value::Tuple(vec![Value::Obj(c), Value::Map(ch)])).collect();
        ov.push(Value::None);
        let p = g.ask(pid, "here_to_help", ov, map! { item = it })?;
        if !p.is_none() {
            let (c, ch) = (p.list()[0].obj(), p.list()[1].map().clone());
            play_reduced(g, pid, c, "hand", ch, 3)?;
        }
        Ok(Value::None)
    });
    Impl { hidden: true, timing: Some("action"), resolve, ..new("Here to Help") }
}

// Keeper's Verdict — "[Action] Choose an enemy unit at a battlefield. Its owner places it on the top or bottom of
// their Main Deck."
fn keepers_verdict() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_enemy_bf_v()]),
        resolve: fv("_verdict", |g, a| {
            let it = a[0].item();
            let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
            let ow = g.o(u).owner;
            let pos = g.ask(ow, "verdict_place", vec!["top".into(), "bottom".into()], map! { item = it, unit = u })?;
            let pos = pos.str().to_string();
            let r = g.repr(u);
            g.log(|| format!("  {r} goes to the {pos} of its owner's deck"));
            g.to_zone(u, "deck", pos == "bottom", None)?;
            Ok(Value::None)
        }),
        choices: fv("choices", |g, a| {
            let pid = a[0].int() as usize;
            let hb = hbv(&a[1].map().g("hidden_bf"));
            let us = enemies(g, pid, true, hb, true)?;
            Ok(maps(tg_choices(g, &us)))
        }),
        ..new("Keeper's Verdict")
    }
}

// Marching Orders — "[Action] [Repeat] [3] Choose a friendly unit anywhere and an enemy unit at a battlefield. They
// deal damage equal to their Mights to each other."
fn marching_orders() -> Impl {
    Impl {
        timing: Some("action"),
        repeat: repeat_cost("3 energy"),
        preds: Value::List(vec![p_friend_v(), p_enemy_bf_v()]),
        resolve: repeatable(fight_targets()),
        choices: fv("choices", |g, a| {
            let pid = a[0].int() as usize;
            let hb = hbv(&a[1].map().g("hidden_bf"));
            let fs = friends(g, pid, false, hb)?;
            let es = enemies(g, pid, true, hb, true)?;
            Ok(maps(duel_choices(g, pid, &fs, &es, 0, None)?))
        }),
        ..new("Marching Orders")
    }
}

// Mobilize — "Channel 1 rune exhausted. If you can't, draw 1."
fn mobilize() -> Impl {
    Impl {
        resolve: fv("res", |g, a| {
            let c = ctrl(g, a[0].item());
            if g.channel(c, 1, true)?.is_empty() {
                g.draw(c, 1)?;
            }
            Ok(Value::None)
        }),
        ..new("Mobilize")
    }
}

// On the Hunt — "Ready your units."
fn on_the_hunt() -> Impl {
    Impl {
        resolve: fv("res", |g, a| {
            let c = ctrl(g, a[0].item());
            for u in g.units(Some(c), None) {
                g.ready_obj(u, None, None)?;
            }
            Ok(Value::None)
        }),
        ..new("On the Hunt")
    }
}

/// `lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), n)`
fn might_target(n: i64) -> Value {
    fv("res", move |g, a| {
        let it = a[0].item();
        if g.legal(it, 0)?.is_some() {
            if let Some(u) = g.legal(it, 0)? {
                g.mod_(u, n, turn(), None)?;
            }
        }
        Ok(Value::None)
    })
}

// Onslaught — "Give a unit +6 might this turn. [Flow] [4]"
fn onslaught() -> Impl {
    Impl {
        preds: Value::List(vec![p_unit_v()]),
        flow: flow_cost("4 energy"),
        resolve: might_target(6),
        choices: friend_choices(),
        ..new("Onslaught")
    }
}

// Overt Operation — "[Action] For each friendly unit, you may spend its buff to ready it. Then buff all friendly
// units."
fn overt_operation() -> Impl {
    Impl {
        timing: Some("action"),
        resolve: fv("_overt", |g, a| {
            let it = a[0].item();
            let pid = ctrl(g, it);
            for u in g.units(Some(pid), None) {
                if g.o(u).buff > 0 && g.on_board(u) {
                    let opts: Vec<Value> =
                        if g.exhausted(u) { vec![true.into(), false.into()] } else { vec![false.into(), true.into()] };
                    if g.ask(pid, "overt_spend", opts, map! { item = it, unit = u })?.truthy() {
                        spend_buff(g, Some(u))?;
                        g.ready_obj(u, None, None)?;
                    }
                }
            }
            for u in g.units(Some(pid), None) {
                g.buff(Some(u), None)?;
            }
            Ok(Value::None)
        }),
        ..new("Overt Operation")
    }
}

// Primal Strength — "[Action] Give a unit +7 might this turn."
fn primal_strength() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_v()]),
        resolve: might_target(7),
        choices: friend_choices(),
        ..new("Primal Strength")
    }
}

// Public Execution — "Choose a friendly unit. Kill an enemy unit with less Might than it. [Flow] [5][A][A]"
fn public_execution() -> Impl {
    let resolve = fv("_execution", |g, a| {
        let it = a[0].item();
        let f = g.legal(it, 0)?;
        let e = g.legal(it, 1)?;
        if let (Some(f), Some(e)) = (f, e) {
            if g.might(e)? < g.might(f)? {
                let c = ctrl(g, it);
                g.kill(&[e], Some(c), false)?;
            }
        }
        Ok(Value::None)
    });
    let choices = fv("_execution_choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let mut out = vec![];
        let fs = friends(g, pid, false, hb)?;
        let mut keys = vec![];
        for &x in &fs {
            keys.push(vec![g.might(x)? as f64]);
        }
        let fs = sort_keyed(fs, keys);
        for e in enemies(g, pid, false, hb, true)? {
            let mut found = None;
            for &f in &fs {
                if g.might(f)? > g.might(e)? {
                    found = Some(f);
                    break;
                }
            }
            if let Some(f) = found {
                out.push(map! { tg = tup(&[g.o(f).uid, g.o(e).uid]) });
            }
        }
        Ok(maps(out))
    });
    Impl {
        preds: Value::List(vec![p_friend_v(), p_enemy_v()]),
        flow: flow_cost("5 energy and 2 runes of any type"),
        resolve,
        choices,
        ..new("Public Execution")
    }
}

// Punch First — "[Action] Give a unit +5 might this turn."
fn punch_first() -> Impl {
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_v()]),
        resolve: might_target(5),
        choices: friend_choices(),
        ..new("Punch First")
    }
}

// Rampage — "As you play this, you may pay [Body] as an additional cost. Choose a friendly unit and an enemy unit.
// If you paid the additional cost, give the friendly unit +2 might this turn. They deal damage equal to their Mights
// to each other."
fn rampage() -> Impl {
    let resolve = fv("_rampage", |g, a| {
        let it = a[0].item();
        let f = g.legal(it, 0)?;
        if let Some(f) = f {
            if g.it(it).data.g("paid").truthy() {
                g.mod_(f, 2, turn(), None)?;
            }
        }
        let e = g.legal(it, 1)?;
        fight(g, f, e)?;
        Ok(Value::None)
    });
    let choices = fv("_rampage_choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let fs = friends(g, pid, false, hb)?;
        let es = enemies(g, pid, false, hb, true)?;
        let x = duel_choices(g, pid, &fs, &es, 0, None)?;
        let xa = cap(g, x, 4, Some(pid));
        let y = duel_choices(g, pid, &fs, &es, 2, Some(&map! { paid = true }))?;
        let xb = cap(g, y, 4, Some(pid));
        let mut out = vec![];
        for (b, a_) in xb.into_iter().zip(xa) {
            out.push(b);
            out.push(a_);
        }
        Ok(maps(out))
    });
    Impl {
        preds: Value::List(vec![p_friend_v(), p_enemy_v()]),
        resolve,
        choices,
        extra_cost_fn: fv("extra_cost_fn", |_g, a| {
            Ok(if a[2].map().g("paid").truthy() { e_reqs(0, &[body_req()]) } else { e_reqs(0, &[]) })
        }),
        ..new("Rampage")
    }
}

// Repulse — "[Reaction] Choose a friendly unit at a battlefield. Counter an enemy spell or ability that chooses it
// and no other friendly unit."
fn repulse_ok(g: &Game, pid: usize, i: ItemId, u: O) -> bool {
    let fr: Vec<u32> = g.units(Some(pid), None).iter().map(|&x| g.o(x).uid).collect();
    let it = g.it(i);
    it.ctrl != pid && it.chosen.contains(&g.o(u).uid) && it.chosen.iter().filter(|x| fr.contains(x)).count() == 1
}

fn repulse() -> Impl {
    let resolve = fv("_repulse", |g, a| {
        let it = a[0].item();
        let u = g.legal(it, 0)?;
        let iid = g.it(it).data.g("item");
        let tgt = iid.opt_int().and_then(|i| item_by_id(g, i));
        if let (Some(u), Some(t)) = (u, tgt) {
            if repulse_ok(g, ctrl(g, it), t, u) {
                g.counter(t)?;
            }
        }
        Ok(Value::None)
    });
    let choices = fv("_repulse_choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let mut out = vec![];
        for i in g.chain.clone().into_iter().rev() {
            if g.it(i).ctrl == pid || !g.counterable(i)? {
                continue;
            }
            for u in friends(g, pid, true, hb)? {
                if repulse_ok(g, pid, i, u) {
                    out.push(map! { tg = tup(&[g.o(u).uid]), item = g.it(i).id });
                }
            }
        }
        Ok(maps(out))
    });
    Impl { timing: Some("reaction"), preds: Value::List(vec![p_friend_bf_v()]), resolve, choices, ..new("Repulse") }
}

// Riposte — "[Reaction] Choose a friendly unit and a spell. Counter that spell and give that unit +might equal to
// that spell's Energy cost this turn." (printed cost, rule 206; if the spell can't be countered or has left the
// chain, the unit still gets the Might: do as much as you can, rule 055)
fn riposte() -> Impl {
    let resolve = fv("_riposte", |g, a| {
        let it = a[0].item();
        let d = g.it(it).data.clone();
        let tgt = d.g("item").opt_int().and_then(|i| item_by_id(g, i));
        if let Some(t) = tgt {
            if g.it(t).kind == "spell" {
                g.counter(t)?;
            }
        }
        if let Some(u) = g.legal(it, 0)? {
            let n = if d.contains("cost") { d.g("cost").int() } else { 0 };
            g.mod_(u, n, turn(), None)?;
        }
        Ok(Value::None)
    });
    let choices = fv("_riposte_choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let mut out = vec![];
        let spells: Vec<ItemId> = g.chain.clone().into_iter().rev().filter(|&i| g.it(i).kind == "spell").collect();
        let mut keys = vec![];
        for &i in &spells {
            keys.push(vec![bk(g.it(i).ctrl == pid), bk(!g.counterable(i)?)]);
        }
        let spells = sort_keyed(spells, keys);
        for i in cap(g, spells, 3, Some(pid)) {
            let fs = friends(g, pid, false, hb)?;
            for u in cap(g, fs, 3, Some(pid)) {
                let cost = g.spec(g.it(i).card.unwrap()).e;
                out.push(map! { tg = tup(&[g.o(u).uid]), item = g.it(i).id, cost = cost });
            }
        }
        Ok(maps(out))
    });
    Impl { timing: Some("reaction"), preds: Value::List(vec![p_friend_v()]), resolve, choices, ..new("Riposte") }
}

// Sabotage — "Choose an opponent. They reveal their hand. Choose a non-unit card from it, and recycle that card."
fn sabotage() -> Impl {
    Impl {
        resolve: fv("res", |g, a| {
            reveal_recycle(g, a[0].item(), |g_, x| g_.spec(x).type_ != "Unit", "sabotage")?;
            Ok(Value::None)
        }),
        ..new("Sabotage")
    }
}

// Show of Strength — "[Reaction] Draw 1 for each of your [Mighty] units."
fn show_of_strength() -> Impl {
    Impl {
        timing: Some("reaction"),
        resolve: fv("res", |g, a| {
            let c = ctrl(g, a[0].item());
            let mut n = 0;
            for u in g.units(Some(c), None) {
                if g.mighty(u)? {
                    n += 1;
                }
            }
            g.draw(c, n)?;
            Ok(Value::None)
        }),
        ..new("Show of Strength")
    }
}

// Showstopper — "Buff a friendly unit in your base, then move it to a battlefield."
fn showstopper() -> Impl {
    let resolve = fv("_showstopper", |g, a| {
        let it = a[0].item();
        let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
        g.buff(Some(u), None)?;
        let dest = g.it(it).data.g("dest").loc();
        if g.o(u).loc != Some(dest) {
            let c = ctrl(g, it);
            g.move_(&[u], dest, Some(c), false)?;
        }
        Ok(Value::None)
    });
    let choices = fv("choices", |g, a| {
        let pid = a[0].int() as usize;
        let mut out = vec![];
        for u in friends(g, pid, false, None)? {
            if g.o(u).loc == Some(Loc::Base) {
                for i in 0..g.bfs.len() {
                    out.push(map! { tg = tup(&[g.o(u).uid]), dest = Loc::Bf(g.bfs[i].idx) });
                }
            }
        }
        let keys = out.iter().map(|c| vec![bk(g.bfs[c.g("dest").loc().bf().unwrap()].ctrl == Some(pid))]).collect();
        Ok(maps(sort_keyed(out, keys)))
    });
    Impl {
        preds: Value::List(vec![pred(|g, it, o| Ok(p_friend(g, it, o) && g.o(o).loc == Some(Loc::Base)))]),
        resolve,
        choices,
        ..new("Showstopper")
    }
}

// Stare Down — "Choose a friendly unit and a battlefield. Move all enemy units at that battlefield with less Might
// than the chosen unit to their base. Gain 1 XP."
fn stare_down() -> Impl {
    let resolve = fv("_stare", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        if let Some(u) = g.legal(it, 0)? {
            let m = g.might(u)?;
            let bf = bfi(&g.it(it).data.g("bf"));
            let mut es = vec![];
            for e in g.units(Some(1 - c), Some(Loc::Bf(bf))) {
                if g.might(e)? < m {
                    es.push(e);
                }
            }
            if !es.is_empty() {
                g.move_(&es, Loc::Base, Some(c), false)?;
            }
        }
        g.gain_xp(c, 1)?;
        Ok(Value::None)
    });
    let choices = fv("_stare_choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let fs = friends(g, pid, false, hb)?;
        let mut out = vec![];
        for u in cap(g, fs, 3, Some(pid)) {
            for i in 0..g.bfs.len() {
                out.push(map! { tg = tup(&[g.o(u).uid]), bf = g.bfs[i].idx });
            }
        }
        let mut keys = vec![];
        for c in &out {
            let bf = bfi(&c.g("bf"));
            let f = g.obj(c.g("tg").list()[0].int() as u32).unwrap();
            let mut n = 0;
            for e in g.units(Some(1 - pid), Some(Loc::Bf(bf))) {
                if g.might(e)? < g.might(f)? {
                    n += 1;
                }
            }
            keys.push(vec![-(n as f64)]);
        }
        Ok(maps(sort_keyed(out, keys)))
    });
    Impl { preds: Value::List(vec![p_friend_v()]), resolve, choices, ..new("Stare Down") }
}

// Strike Down — "Choose an equipped friendly unit. It deals damage equal to its Might to an enemy unit. Then detach
// an Equipment from it." (the detached gear stays where it is and is recalled at the next cleanup, rule 323)
fn equipped(g: &Game, o: O) -> bool {
    g.o(o).attached.iter().any(|&x| g.obj(x).is_some_and(|y| g.spec(y).tags.contains("Equipment")))
}

fn strike_down() -> Impl {
    let resolve = fv("_strike_down", |g, a| {
        let it = a[0].item();
        let f = g.legal(it, 0)?;
        let e = g.legal(it, 1)?;
        let Some(f) = f else { return Ok(Value::None) };
        let c = ctrl(g, it);
        if let Some(e) = e {
            let m = 0.max(g.might(f)?);
            g.deal(Some(e), m, "unit", Some(c), None)?;
        }
        let eq: Vec<O> = g
            .o(f)
            .attached
            .iter()
            .filter_map(|&x| g.obj(x))
            .filter(|&y| g.spec(y).tags.contains("Equipment"))
            .collect();
        if !eq.is_empty() {
            let keys = eq.iter().map(|&x| vec![g.equip_bonus_of(g.cname(x)) as f64, g.o(x).uid as f64]).collect();
            let eq = sort_keyed(eq, keys);
            let x = g.ask(c, "detach", ovals(&eq), map! { item = it })?.obj();
            let xu = g.o(x).uid;
            let at = &mut g.o_mut(f).attached;
            if let Some(i) = at.iter().position(|&y| y == xu) {
                at.remove(i);
            }
            g.o_mut(x).attached_to = None;
            g.need_cleanup = true;
            let (rx, rf) = (g.repr(x), g.repr(f));
            g.log(|| format!("  {rx} detached from {rf}"));
        }
        Ok(Value::None)
    });
    let choices = fv("_strike_choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let mut out = vec![];
        for f in friends(g, pid, false, hb)? {
            if equipped(g, f) {
                let es = enemies(g, pid, false, hb, true)?;
                for e in cap(g, es, 4, Some(pid)) {
                    out.push(map! { tg = tup(&[g.o(f).uid, g.o(e).uid]) });
                }
            }
        }
        let mut keys = vec![];
        for c in &out {
            let tg = c.g("tg");
            let f = g.obj(tg.list()[0].int() as u32).unwrap();
            let e = g.obj(tg.list()[1].int() as u32).unwrap();
            let s = if g.might(f)? >= g.might(e)? - g.o(e).damage { value(g, e)? } else { 0.0 };
            keys.push(vec![-s]);
        }
        Ok(maps(sort_keyed(out, keys)))
    });
    Impl {
        preds: Value::List(vec![pred(|g, it, o| Ok(p_friend(g, it, o) && equipped(g, o))), p_enemy_v()]),
        resolve,
        choices,
        ..new("Strike Down")
    }
}

// Void Assault — "Move a friendly unit, then move an enemy unit."
fn void_assault() -> Impl {
    let resolve = fv("_void_assault", |g, a| {
        let it = a[0].item();
        let c = ctrl(g, it);
        let d = g.it(it).data.clone();
        let (d1, d2) = (d.g("d1").loc(), d.g("d2").loc());
        if let Some(f) = g.legal(it, 0)? {
            if g.o(f).loc != Some(d1) {
                g.move_(&[f], d1, Some(c), false)?;
            }
        }
        if let Some(e) = g.legal(it, 1)? {
            if g.o(e).loc != Some(d2) {
                g.move_(&[e], d2, Some(c), false)?;
            }
        }
        Ok(Value::None)
    });
    let choices = fv("_void_assault_choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let mut out = vec![];
        let mut keys = vec![];
        let fs = friends(g, pid, false, hb)?;
        for f in cap(g, fs, 2, Some(pid)) {
            for d1 in dests(g, f) {
                let es = enemies(g, pid, false, hb, true)?;
                for e in cap(g, es, 2, Some(pid)) {
                    for d2 in dests(g, e) {
                        let sc = (!d1.is_bf()) as i64 + (d2 != Loc::Base) as i64 - (d1.is_bf() && d1 == d2) as i64;
                        keys.push(vec![sc as f64, out.len() as f64]);
                        out.push(map! { tg = tup(&[g.o(f).uid, g.o(e).uid]), d1 = d1, d2 = d2 });
                    }
                }
            }
        }
        Ok(maps(sort_keyed(out, keys)))
    });
    Impl { preds: Value::List(vec![p_friend_v(), p_enemy_v()]), resolve, choices, ..new("Void Assault") }
}

// Wallop — "[Action] As you play this, you may spend a buff as an additional cost. If you do, ignore this spell's
// cost. Ready a unit."
fn wallop() -> Impl {
    let choices = fv("_wallop_choices", |g, a| {
        let pid = a[0].int() as usize;
        let hb = hbv(&a[1].map().g("hidden_bf"));
        let mut us = vec![];
        for u in g.units(None, None) {
            if (g.o(u).ctrl == pid || g.targetable(u, Some(pid))?) && (hb.is_none() || g.o(u).loc == hb) {
                us.push(u);
            }
        }
        let mut keys = vec![];
        for &u in &us {
            keys.push(vec![bk(g.o(u).ctrl != pid), bk(!g.exhausted(u)), -value(g, u)?]);
        }
        let us = sort_keyed(us, keys);
        let us = cap(g, us, 4, Some(pid));
        let bs: Vec<O> = g.units(Some(pid), None).into_iter().filter(|&u| g.o(u).buff > 0).collect();
        let mut bkeys = vec![];
        for &u in &bs {
            bkeys.push(vec![value(g, u)?]);
        }
        let bs = sort_keyed(bs, bkeys);
        let mut out = vec![];
        for &u in &us {
            out.push(map! { tg = tup(&[g.o(u).uid]) });
            for b in cap(g, bs.clone(), 1, Some(pid)) {
                out.push(map! { tg = tup(&[g.o(u).uid]), free = true, spend = g.o(b).uid });
            }
        }
        Ok(maps(out))
    });
    let pay_extra = fv("_wallop_pay", |g, a| {
        let ch = a[2].map();
        if ch.g("spend").truthy() {
            let o = g.obj(ch.g("spend").int() as u32);
            spend_buff(g, o)?;
        }
        Ok(Value::None)
    });
    let resolve = fv("res", |g, a| {
        let it = a[0].item();
        if g.legal(it, 0)?.is_some() {
            if let Some(u) = g.legal(it, 0)? {
                g.ready_obj(u, None, None)?;
            }
        }
        Ok(Value::None)
    });
    Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_v()]),
        choices,
        pay_extra,
        resolve,
        ..new("Wallop")
    }
}

// Wild Claw — "Look at the top 5 cards of your Main Deck. You may banish a unit or gear from among them and play it,
// reducing its Energy cost by [5]. Recycle the rest. Then you may do this: Empower it."
fn wild_claw() -> Impl {
    let resolve = fv("_wild_claw", |g, a| {
        let it = a[0].item();
        let pid = ctrl(g, it);
        let top5: Vec<O> = g.p[pid].deck.iter().take(5).copied().collect();
        let top = g.look(pid, &top5, None)?;
        let mut opts: Vec<(O, Map)> = vec![];
        for &c in &top {
            let t = g.spec(c).type_.clone();
            if (t != "Unit" && t != "Gear") || g.impl_o(c).is_none() {
                continue;
            }
            let chs = if t == "Unit" {
                unit_play_choices(g, pid, c, "banish", None)?
            } else {
                vec![map! { loc = Loc::Base }]
            };
            for ch in chs {
                let (e, reqs) = reduced_cost(g, pid, c, &ch, "banish", 5)?;
                if g.can_pay(pid, e, &reqs, &Value::Map(map! { kind = t.to_lowercase(), card = c }))? {
                    opts.push((c, ch));
                }
            }
        }
        let keys = opts
            .iter()
            .map(|(c, ch)| {
                vec![
                    card_weight(g, *c),
                    bk(ch.g("loc") != Value::Loc(Loc::Base)),
                    bk(ch.g("acc").truthy()),
                    g.o(*c).uid as f64,
                ]
            })
            .collect();
        let opts = sort_keyed(opts, keys);
        let p = if !opts.is_empty() {
            let mut ov: Vec<Value> =
                opts.into_iter().map(|(c, ch)| Value::Tuple(vec![Value::Obj(c), Value::Map(ch)])).collect();
            ov.push(Value::None);
            g.ask(pid, "wild_claw", ov, map! { item = it })?
        } else {
            Value::None
        };
        let mut played = Value::None;
        if !p.is_none() {
            let (c, ch) = (p.list()[0].obj(), p.list()[1].map().clone());
            if let Some(i) = g.p[pid].deck.iter().position(|&x| x == c) {
                g.p[pid].deck.remove(i);
            }
            g.o_mut(c).zone = Some(Zone::Banish);
            g.p[pid].banish.push(c);
            played = play_reduced(g, pid, c, "banish", ch, 5)?;
        }
        let rest: Vec<O> = top.iter().copied().filter(|x| g.p[pid].deck.contains(x)).collect();
        for &x in &rest {
            if let Some(i) = g.p[pid].deck.iter().position(|&y| y == x) {
                g.p[pid].deck.remove(i);
            }
        }
        g.recycle_cards(pid, &rest)?;
        if let Some(pl) = played.opt_obj() {
            if g.on_board(pl)
                && !g.o(pl).empowered
                && g.ask(pid, "may", vec![true.into(), false.into()], map! { item = it })?.truthy()
            {
                g.empower(Some(pl), None)?;
            }
        }
        Ok(Value::None)
    });
    Impl { resolve, ..new("Wild Claw") }
}

// ====================================================================== cartes qui utilisent les crochets du moteur
/// `_alone_enemy_bfs(g, pid)` : battlefields occupés où une unité ennemie est seule (exactement une unité de cet
/// adversaire).
fn alone_enemy_bfs(g: &Game, pid: usize) -> Value {
    Value::List(
        (0..g.bfs.len())
            .filter(|&i| g.units(Some(1 - pid), Some(Loc::Bf(i))).len() == 1)
            .map(|i| Value::Loc(Loc::Bf(g.bfs[i].idx)))
            .collect(),
    )
}

// Arachnoid Horror — "[Hunt 2] I can be played to an occupied battlefield if an enemy unit is alone there. Friendly
// units can be played to an occupied battlefield if an enemy unit is alone there."
fn arachnoid() -> Impl {
    let mut im = Impl { kw: map! { Hunt = 2 }, ..new("Arachnoid Horror") };
    im.hooks.set(
        "extra_locs",
        fv("extra_locs", |g, a| {
            let (pid, closed) = (a[0].int() as usize, a[2].truthy());
            Ok(if closed { Value::List(vec![]) } else { alone_enemy_bfs(g, pid) })
        }),
    );
    im.hooks.set(
        "aura_locs",
        fv("aura_locs", |g, a| {
            let (s, pid, c, closed) = (a[0].obj(), a[1].int() as usize, a[2].obj(), a[3].truthy());
            Ok(if closed || pid != g.o(s).ctrl || g.spec(c).type_ != "Unit" {
                Value::List(vec![])
            } else {
                alone_enemy_bfs(g, pid)
            })
        }),
    );
    im
}

/// `_occupied_enemy_bfs(g, pid, closed)`
fn occupied_enemy_bfs(g: &Game, pid: usize, closed: bool) -> Value {
    if closed {
        return Value::List(vec![]);
    }
    Value::List(
        (0..g.bfs.len())
            .filter(|&i| g.bfs[i].ctrl == Some(1 - pid) && !g.units(None, Some(Loc::Bf(i))).is_empty())
            .map(|i| Value::Loc(Loc::Bf(g.bfs[i].idx)))
            .collect(),
    )
}

fn occupied_extra_locs() -> Value {
    fv("extra_locs", |g, a| Ok(occupied_enemy_bfs(g, a[0].int() as usize, a[2].truthy())))
}

// Dauntless Vanguard — "You may play me to an occupied enemy battlefield."
fn dauntless_vanguard() -> Impl {
    let mut im = new("Dauntless Vanguard");
    im.hooks.set("extra_locs", occupied_extra_locs());
    im
}

// Deadbloom Predator — "[Deflect] You may play me to an occupied enemy battlefield."
fn deadbloom() -> Impl {
    let mut im = Impl { kw: map! { Deflect = 1 }, ..new("Deadbloom Predator") };
    im.hooks.set("extra_locs", occupied_extra_locs());
    im
}

// Rengar, Trophy Hunter — "[Ambush] I can be played to a battlefield where there are enemy units (even if you don't
// have units there)." (it extends Ambush's permissions, with its Reaction timing, rule 822.1.d)
fn rengar() -> Impl {
    let mut im = Impl { ambush: true, ..new("Rengar, Trophy Hunter") };
    im.hooks.set(
        "extra_locs",
        fv("extra_locs", |g, a| {
            let pid = a[0].int() as usize;
            Ok(Value::List(
                (0..g.bfs.len())
                    .filter(|&i| !g.units(Some(1 - pid), Some(Loc::Bf(i))).is_empty())
                    .map(|i| Value::Loc(Loc::Bf(g.bfs[i].idx)))
                    .collect(),
            ))
        }),
    );
    im
}

// Ambessa, The Wolf — "[Empower] [3][Body] [Empowered][>] I have +3 might and can't be dealt damage unless I'm in
// combat."
fn ambessa() -> Impl {
    let mut im = Impl { might_if: vec![(when_empowered(), 3)], ..new("Ambessa, The Wolf") };
    im.hooks.set(
        "dmg_prevent",
        fv("dmg_prevent", |g, a| {
            let o = a[0].obj();
            Ok(Value::Bool(g.o(o).empowered && !g.in_combat(o)))
        }),
    );
    impl_empower(&mut im, Cost::Text("3 energy and 1 body rune".into()));
    im
}

// Unyielding Spirit — "[Reaction] Prevent all spell and ability damage this turn."
fn unyielding_spirit() -> Impl {
    Impl {
        timing: Some("reaction"),
        resolve: fv("res", |g, _a| {
            let f = fv("_spirit_prevent", |_g, a| {
                let k = a[2].map().g("kind");
                Ok(Value::Bool(matches!(k.opt_str(), Some("spell") | Some("ability"))))
            });
            g.add_effect(eff_map(vec![("kind", "prevent_damage".into()), ("fn", f), ("dur", turn())]));
            Ok(Value::None)
        }),
        ..new("Unyielding Spirit")
    }
}

// Elder Dragon — "Any amount of your damage is enough to kill enemy units. When you play me, choose up to one enemy
// unit at each location. Deal 1 to them." (rule 142.4.c: Game.lethal / lethal_need with Impl.lethal_any)
fn elder_dragon() -> Impl {
    let choose = || {
        fv("_elder_choose", |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            for loc in g.locs() {
                let mut es = vec![];
                for u in g.units(Some(1 - c), Some(loc)) {
                    if g.targetable(u, Some(c))? && payable(g, it, u)? {
                        es.push(u);
                    }
                }
                let mut keys = vec![];
                for &u in &es {
                    keys.push(vec![-value(g, u)?, g.o(u).uid as f64]);
                }
                let es = sort_keyed(es, keys);
                if es.is_empty() {
                    continue;
                }
                let mut opts = ovals(&es);
                opts.push(Value::None);
                let u = g.ask(c, "elder_dragon", opts, map! { item = it, loc = loc })?.opt_obj();
                if let Some(u) = u {
                    if pay_deflect(g, it, u)? {
                        g.add_target(it, u, p_enemy_v());
                    }
                }
            }
            Ok(Value::Bool(!g.it(it).targets.is_empty()))
        })
    };
    let res = || {
        fv("_elder_res", |g, a| {
            let it = a[0].item();
            let c = ctrl(g, it);
            for u in targets(g, it)? {
                g.deal(Some(u), 1, "ability", Some(c), None)?;
            }
            Ok(Value::None)
        })
    };
    let mut im = Impl {
        on_play: fv("on_play", move |g, a| {
            let o = a[0].obj();
            trig_full(g, o, "Elder Dragon", res(), Map::new(), false, choose(), Value::None);
            Ok(Value::None)
        }),
        ..new("Elder Dragon")
    };
    im.hooks.set(
        "lethal_any",
        fv("lethal_any", |g, a| Ok(Value::Bool(g.o(a[1].obj()).ctrl != g.o(a[0].obj()).ctrl))),
    );
    im
}

// Determined Sentry — "I can't move to base." (a recall is not a move, rule 455)
fn determined_sentry() -> Impl {
    let mut im = new("Determined Sentry");
    im.hooks.set("cant_move", fv("cant_move", |_g, a| Ok(Value::Bool(a[1].opt_loc() == Some(Loc::Base)))));
    im
}

// Jagged Cutlass — "[Equip] [Body]". Might Bonus +2, Effect Text (card image): "I can't be moved by enemy spells and
// abilities."
fn jagged_cutlass() -> Impl {
    let mut im = equipment("Jagged Cutlass", "1 body rune", 2);
    im.hooks.set(
        "fx_cant_move",
        fv("fx_cant_move", |g, a| {
            let (o, by, standard) = (a[1].obj(), a[3].opt_int(), a[4].truthy());
            Ok(Value::Bool(!standard && by.is_some() && by != Some(g.o(o).ctrl as i64)))
        }),
    );
    im
}

// Fae Dragon — "When you play me, buff up to four friendly units. When you spend a buff, play a Gold gear token
// exhausted."
fn fae_dragon() -> Impl {
    let on_event = fv("_fae_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "spend_buff" && info.g("pid").opt_int() == Some(g.o(o).ctrl as i64) {
            let res = fv("res", |g_, a| {
                let c = ctrl(g_, a[0].item());
                make_token(g_, "Gold", c, Loc::Base, false)?;
                Ok(Value::None)
            });
            trig(g, o, "Fae Dragon", res);
        }
        Ok(Value::None)
    });
    let on_play = fv("on_play", |g, a| {
        let o = a[0].obj();
        // choisies comme cibles quand la capacité a été mise sur la chaîne (355.10)
        let res = fv("_fae_res", |g_, a| {
            let it = a[0].item();
            for u in targets(g_, it)? {
                g_.buff(Some(u), None)?;
            }
            Ok(Value::None)
        });
        let opts = |g_: &mut Game, it: ItemId| -> Res<Vec<O>> {
            let c = ctrl(g_, it);
            let us = g_.units(Some(c), None);
            let mut keys = vec![];
            for &u in &us {
                keys.push(vec![bk(g_.o(u).buff > 0), -value(g_, u)?, g_.o(u).uid as f64]);
            }
            Ok(sort_keyed(us, keys))
        };
        let choose = pick(opts, p_friend_v(), 4, "fae_dragon_buff", false);
        trig_full(g, o, "Fae Dragon", res, Map::new(), false, choose, Value::None);
        Ok(Value::None)
    });
    Impl { on_event, on_play, ..new("Fae Dragon") }
}

// Pirate's Haven — "When you ready a friendly unit, give it +1 might this turn." (the Awaken readying is by the turn
// player, rule 415.3.a)
fn pirates_haven() -> Impl {
    let on_event = fv("_haven_event", |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        let u = if ev == "ready" { info.g("obj").opt_obj() } else { None };
        let Some(u) = u else { return Ok(Value::None) };
        let oc = g.o(o).ctrl;
        if g.o(u).ctrl == oc && info.g("by").opt_int() == Some(oc as i64) && g.spec(u).type_ == "Unit" {
            let res = fv("res", |g_, a| {
                let d = g_.it(a[0].item()).data.clone();
                if let Some(x) = g_.obj(d.g("u").int() as u32) {
                    if g_.o(x).oid == d.g("uoid").int() {
                        g_.mod_(x, 1, turn(), None)?;
                    }
                }
                Ok(Value::None)
            });
            let (uu, uo) = (g.o(u).uid, g.o(u).oid);
            trig_full(g, o, "Pirate's Haven", res, map! { u = uu, uoid = uo }, false, Value::None, Value::None);
        }
        Ok(Value::None)
    });
    Impl { on_event, ..new("Pirate's Haven") }
}

// Renekton, Brute — "[1]: Give me +1 might this turn. When my Might becomes 10 or more, empower me. [Empowered][>] I
// have [Deflect] and [Ganking]." (Impl.might_watch: the 'might_reached' event at cleanup)
fn renekton() -> Impl {
    let on_event = fv("_renekton_event", |g, a| {
        if let Some((o, _)) = ev_obj(a, "might_reached") {
            let res = fv("res", |g_, a| {
                let s = src(g_, a[0].item());
                g_.empower(s, None)?;
                Ok(Value::None)
            });
            trig(g, o, "Renekton, Brute", res);
        }
        Ok(Value::None)
    });
    Impl {
        might_watch: Some(10),
        on_event,
        kw_if: vec![(when_empowered(), map! { Deflect = 1, Ganking = 1 })],
        abilities: vec![ability(
            "+1 might",
            AbilityArgs {
                cost: Cost::Text("1 energy".into()),
                resolve: fv("res", |g, a| {
                    let it = a[0].item();
                    if src(g, it).is_some() {
                        let s = src(g, it).unwrap();
                        g.mod_(s, 1, turn(), None)?;
                    }
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..new("Renekton, Brute")
    }
}

// Spoils of War — "[Reaction] If an enemy unit has died this turn, this costs [2] less. Draw 2." (Game.hist['died'])
fn spoils_of_war() -> Impl {
    Impl {
        timing: Some("reaction"),
        resolve: fv("res", |g, a| {
            let c = ctrl(g, a[0].item());
            g.draw(c, 2)?;
            Ok(Value::None)
        }),
        cost_mod: fv("cost_mod", |g, a| {
            let pid = a[0].int() as i64;
            let died = g
                .hist
                .died
                .iter()
                .any(|d| d.g("ctrl").int() != pid && spec(d.g("spec").str()).type_ == "Unit");
            Ok(if died { de_dp(2, 0) } else { de_dp(0, 0) })
        }),
        ..new("Spoils of War")
    }
}

// Wily Newtfish — "If you've gained XP this turn, I have +1 might and [Ganking]." (Game.hist['xp'])
fn newt() -> Value {
    fv("_newt", |g, a| Ok(Value::Bool(g.hist.xp[g.o(a[0].obj()).ctrl] > 0)))
}

fn wily_newtfish() -> Impl {
    Impl { might_if: vec![(newt(), 1)], kw_if: vec![(newt(), map! { Ganking = 1 })], ..new("Wily Newtfish") }
}

// Herald of Scales — "Your Dragons' Energy costs are reduced by 2 energy, to a minimum of 1 energy."
fn herald_of_scales() -> Impl {
    let mut im = new("Herald of Scales");
    im.hooks.set(
        "cost_aura",
        fv("cost_aura", |g, a| {
            let (s, pid, what) = (a[0].obj(), a[1].int() as usize, a[2].map());
            if pid == g.o(s).ctrl {
                if let Some(c) = what.g("card").opt_obj() {
                    if g.tags(c)?.contains("Dragon") {
                        return Ok(maps(vec![map! { e = -2, min_e = 1 }]));
                    }
                }
            }
            Ok(Value::Tuple(vec![]))
        }),
    );
    im
}

// Ancient Henge — "[E]: [Reaction] — Pay any amount of Energy to [Add] that much [A]." (an [Add] whose cost is energy:
// Game.plan_convert 'e2p')
fn ancient_henge() -> Impl {
    Impl { add: vec![map! { conv = "e2p", var = true, exhaust = true }], ..new("Ancient Henge") }
}

// Akshan, Mischievous — "[Weaponmaster] You may pay [Body][Body] as an additional cost to play me. When you play me, if
// you paid the additional cost, move an enemy gear to your base. You control it until I leave the board. If it's an
// Equipment, attach it to me." If Akshan has already left the board when the ability resolves, the duration is over
// and nothing happens.
fn akshan_return() -> Value {
    fv("_akshan_return", |g, a| {
        let eid = a[0].int() as u64;
        let info = a[1].map();
        let eff = g.eff(eid).cloned().unwrap_or_default();
        if info.g("obj").opt_obj().map(|o| g.o(o).uid as i64) != Some(eff.g("ak").int())
            || info.g("info").map().g("oid") != eff.g("ak_oid")
        {
            return Ok(Value::None);
        }
        g.remove_effect(eid);
        let Some(x) = g.obj(eff.g("gear").int() as u32) else { return Ok(Value::None) };
        if g.o(x).oid == eff.g("gear_oid").int() && g.on_board(x) {
            if let Some(at) = g.o(x).attached_to {
                let xu = g.o(x).uid;
                if let Some(u) = g.obj(at) {
                    let v = &mut g.o_mut(u).attached;
                    if let Some(i) = v.iter().position(|&y| y == xu) {
                        v.remove(i);
                    }
                }
                g.o_mut(x).attached_to = None;
            }
            let prev = eff.g("prev").int() as usize;
            g.o_mut(x).ctrl = prev;
            g.o_mut(x).loc = Some(Loc::Base);
            let r = g.repr(x);
            g.log(|| format!("  P{prev} gets {r} back"));
            g.need_cleanup = true;
        }
        Ok(Value::None)
    })
}

fn akshan() -> Impl {
    let akshan_res = fv("_akshan_res", |g, a| {
        let it = a[0].item();
        let Some(me) = src(g, it) else { return Ok(Value::None) };
        let c = ctrl(g, it);
        let gs = g.gear(Some(1 - c));
        let keys = gs.iter().map(|&x| vec![-(g.spec(x).e as f64), g.o(x).uid as f64]).collect();
        let gs = sort_keyed(gs, keys);
        if gs.is_empty() {
            return Ok(Value::None);
        }
        let x = g.ask(c, "akshan_gear", ovals(&gs), map! { item = it })?.obj();
        let prev = g.o(x).ctrl;
        if let Some(at) = g.o(x).attached_to {
            let xu = g.o(x).uid;
            if let Some(u) = g.obj(at) {
                let v = &mut g.o_mut(u).attached;
                if let Some(i) = v.iter().position(|&y| y == xu) {
                    v.remove(i);
                }
            }
            g.o_mut(x).attached_to = None;
        }
        g.o_mut(x).ctrl = c;
        g.o_mut(x).loc = Some(Loc::Base);
        let (mu, mo, xu, xo) = (g.o(me).uid, g.o(me).oid, g.o(x).uid, g.o(x).oid);
        g.add_effect(eff_map(vec![
            ("on", "leave".into()),
            ("fn", akshan_return()),
            ("ak", mu.into()),
            ("ak_oid", mo.into()),
            ("gear", xu.into()),
            ("gear_oid", xo.into()),
            ("prev", prev.into()),
        ]));
        let r = g.repr(x);
        g.log(|| format!("  P{c} takes {r}"));
        if g.spec(x).tags.contains("Equipment") {
            attach(g, x, me)?;
        }
        g.need_cleanup = true;
        Ok(Value::None)
    });
    Impl {
        kw: map! { Weaponmaster = 1 },
        as_played: fv("as_played", |_g, _a| Ok(maps(vec![map! { akshan = true }, Map::new()]))),
        extra_cost_fn: fv("extra_cost_fn", |_g, a| {
            Ok(if a[2].map().g("akshan").truthy() { e_reqs(0, &[body_req(), body_req()]) } else { e_reqs(0, &[]) })
        }),
        on_play: fv("on_play", move |g, a| {
            let o = a[0].obj();
            if a[1].map().g("akshan").truthy() {
                trig(g, o, "Akshan, Mischievous", akshan_res.clone());
            }
            Ok(Value::None)
        }),
        ..new("Akshan, Mischievous")
    }
}

// Gangplank, Naval — "[Empower] [Body][Body] [Empowered][>] If a spell or ability that chooses me would stun me, give
// me -might, or return me to hand, give me +3 might this turn instead." (Game.replace_choice_effect ; errata du
// 2026-08-03 : « this turn » ; avant, le +3 était permanent ou prenait la durée du -might remplacé).
fn gangplank() -> Impl {
    let mut im = new("Gangplank, Naval");
    im.hooks.set(
        "choice_rep",
        fv("_gangplank_rep", |g, a| {
            let o = a[0].obj();
            if !g.o(o).empowered {
                return Ok(Value::Bool(false));
            }
            let r = g.repr(o);
            g.log(|| format!("  {r}: +3 might this turn instead"));
            g.o_mut(o).mods.push((3, turn()));
            Ok(Value::Bool(true))
        }),
    );
    impl_empower(&mut im, Cost::Text("2 body runes".into()));
    im
}

// ====================================================================== enregistrement (ordre de body.py)
pub fn register(r: &mut Registry) {
    // unités
    r.card(anivia());
    r.card(baccai());
    r.card(bilgewater());
    r.card(brutal_hunter());
    r.card(buhru());
    r.card(snapvine());
    r.card(cithria());
    r.card(corrupted_dragon());
    r.card(corsair());
    r.card(crowd_favorite());
    r.card(dame());
    r.card(aurora());
    r.card(diplomat());
    r.card(direwing());
    r.card(dragonsoul_sage());
    r.card(dune_drake());
    r.card(fiora());
    r.card(first_mate());
    r.card(fretful_feline());
    r.card(garen());
    r.card(gemhand());
    r.card(gemdragon());
    r.card(challenger());
    r.card(faefolk());
    r.card(jaull_fish());
    r.card(jax());
    r.card(jayce());
    r.card(kato());
    r.card(khazix());
    r.card(kinkou_initiate());
    r.card(kinkou_monk());
    r.card(kraken());
    r.card(laurent());
    r.card(lee_sin());
    r.card(legion_marauder());
    r.card(avec_attrs::lucian());
    r.card(yi_honed());
    r.card(yi_tempered());
    r.card(avec_attrs::miss_fortune());
    r.card(nidalee());
    r.card(nilah());
    r.card(demolitionist());
    r.card(pit_rookie());
    r.card(poppy());
    r.card(profiteer());
    r.card(qiyana());
    r.card(repair_specialist());
    r.card(ruin_runner());
    r.card(sea_monkey());
    r.card(sett());
    r.card(avec_attrs::sivir());
    r.card(stormclaw());
    r.card(targonian());
    r.card(avec_attrs::udyr());
    r.card(volibear());
    r.card(warwick());
    r.card(wildclaw_shaman());
    r.card(yone());
    r.card(yordle_explorer());
    // équipements
    r.card(arena_bar());
    r.card(blood_rose());
    r.card(boneshiver());
    r.card(equipment("Doran's Blade", "1 body rune", 2));
    r.card(Impl { equip_kw: Some(map! { Deflect = 1 }), ..equipment("Hexdrinker", "1 body rune", 1) });
    r.card(Impl { equip_kw: Some(map! { Hunt = 1 }), ..equipment("Hunter's Machete", "1 body rune", 2) });
    r.card(trinity());
    r.card(warmog());
    r.card(hextech_disc());
    r.card(mistfall());
    r.card(petricite());
    r.card(platewyrm());
    r.card(seal_of_strength());
    r.card(tools_of_empire());
    // sorts
    r.card(bullet_time());
    r.card(call_to_battle());
    r.card(cannon_barrage());
    r.card(cataclysmic_duel());
    r.card(catalyst());
    r.card(challenge());
    r.card(clash_of_giants());
    r.card(concentrate());
    r.card(confront());
    r.card(decisive_strike());
    r.card(decree_of_strength());
    r.card(disposal_order());
    r.card(flurry());
    r.card(gentlemens_duel());
    r.card(grim_resolve());
    r.card(guttural_roar());
    r.card(here_to_help());
    r.card(keepers_verdict());
    r.card(marching_orders());
    r.card(mobilize());
    r.card(on_the_hunt());
    r.card(onslaught());
    r.card(overt_operation());
    r.card(primal_strength());
    r.card(public_execution());
    r.card(punch_first());
    r.card(rampage());
    r.card(repulse());
    r.card(riposte());
    r.card(sabotage());
    r.card(show_of_strength());
    r.card(showstopper());
    r.card(stare_down());
    r.card(strike_down());
    r.card(void_assault());
    r.card(wallop());
    r.card(wild_claw());
    // cartes qui utilisent les crochets du moteur
    r.ask_text(&[
        ("elder_dragon", "Elder Dragon : quelle unité ennemie à cet endroit (jusqu'à une) ?"),
        ("fae_dragon_buff", "Fae Dragon : quelle unité alliée améliorer (jusqu'à quatre) ?"),
        ("akshan_gear", "Akshan, Mischievous : quel équipement ennemi déplacer dans ta base ?"),
    ]);
    r.card(arachnoid());
    r.card(dauntless_vanguard());
    r.card(deadbloom());
    r.card(rengar());
    r.card(ambessa());
    r.card(unyielding_spirit());
    r.card(elder_dragon());
    r.card(determined_sentry());
    r.card(jagged_cutlass());
    r.card(fae_dragon());
    r.card(pirates_haven());
    r.card(renekton());
    r.card(spoils_of_war());
    r.card(wily_newtfish());
    r.card(herald_of_scales());
    r.card(ancient_henge());
    r.card(akshan());
    r.card(gangplank());
}

#[cfg(test)]
mod tests {
    use super::combinations;

    #[test]
    fn combinaisons_comme_itertools() {
        assert_eq!(combinations(3, 2), vec![vec![0, 1], vec![0, 2], vec![1, 2]]);
        assert_eq!(combinations(4, 3), vec![vec![0, 1, 2], vec![0, 1, 3], vec![0, 2, 3], vec![1, 2, 3]]);
        assert_eq!(combinations(2, 2), vec![vec![0, 1]]);
        assert_eq!(combinations(1, 1), vec![vec![0]]);
        assert!(combinations(1, 2).is_empty());
    }
}

// ====================================================================== cartes avec attributs d'objet
/// Cartes qui posent des attributs sur l'objet (`setattr(o, "_body_...")` en Python) : `Obj.attrs`, jamais remis à
/// zéro, copiés avec la partie.
mod avec_attrs {
    use super::*;
    use crate::cards_helpers::p_unit_bf_v;

    /// `_first_each_turn(o, key, g)` : vrai la première fois pour (incarnation, clé) dans un tour.
    fn first_each_turn(g: &mut Game, o: O, key: &str) -> bool {
        let attr = format!("_body_{key}");
        let mark = Value::Tuple(vec![g.turn_no.into(), g.o(o).oid.into()]);
        if g.o(o).attrs.g(&attr) == mark {
            return false;
        }
        g.o_mut(o).attrs.set(&attr, mark);
        true
    }

    // Lucian, Merciless — "[Weaponmaster] The first time I conquer each turn, ready me."
    pub fn lucian() -> Impl {
        let on_event = fv("_lucian", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map().clone());
            if ev == "conquer" && in_units(&info, o) && first_each_turn(g, o, "lucian") {
                let res = fv("res", |g_, a| {
                    let it = a[0].item();
                    if src(g_, it).is_some() {
                        let s = src(g_, it).unwrap();
                        g_.ready_obj(s, None, None)?;
                    }
                    Ok(Value::None)
                });
                trig(g, o, "Lucian, Merciless", res);
            }
            Ok(Value::None)
        });
        Impl { kw: map! { Weaponmaster = 1 }, on_event, ..new("Lucian, Merciless") }
    }

    // Miss Fortune, Captain — "[Accelerate] [Ganking] The first time I move each turn, you may ready something else
    // that's exhausted." (anything on the board: units, gear, runes, legends; rules 107, 415.1)
    pub fn miss_fortune() -> Impl {
        let on_event = fv("_mf", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map().clone());
            if !(ev == "move" && is_o(&info.g("obj"), o) && first_each_turn(g, o, "mf")) {
                return Ok(Value::None);
            }
            let choose = fv("choose", |g_, a| {
                let it = a[0].item();
                let pid = ctrl(g_, it);
                let s = g_.it(it).src;
                let mine: Vec<O> = g_
                    .units(Some(pid), None)
                    .into_iter()
                    .filter(|&u| g_.exhausted(u) && Some(g_.o(u).uid) != s)
                    .collect();
                let mut keys = vec![];
                for &u in &mine {
                    keys.push(vec![-value(g_, u)?]);
                }
                let mine = sort_keyed(mine, keys);
                let rs0: Vec<_> = g_.p[pid].runes.iter().copied().filter(|&r| g_.r(r).exhausted).collect();
                let rs = cap(g_, rs0, 1, Some(pid));
                let mut other = vec![];
                for x in g_.board.clone() {
                    if g_.exhausted(x)
                        && Some(g_.o(x).uid) != s
                        && !mine.contains(&x)
                        && (g_.o(x).ctrl == pid || (g_.targetable(x, Some(pid))? && payable(g_, it, x)?))
                    {
                        other.push(x);
                    }
                }
                let okeys = other
                    .iter()
                    .map(|&x| vec![bk(g_.o(x).ctrl != pid), bk(g_.spec(x).type_ != "Gear"), g_.o(x).uid as f64])
                    .collect();
                let other = sort_keyed(other, okeys);
                let legs: Vec<O> = g_.p.iter().map(|pl| pl.legend).filter(|&x| g_.exhausted(x)).collect();
                let lkeys = legs.iter().map(|&x| vec![bk(g_.o(x).owner != pid)]).collect();
                let legs = sort_keyed(legs, lkeys);
                let er0: Vec<_> = g_.p[1 - pid].runes.iter().copied().filter(|&r| g_.r(r).exhausted).collect();
                let erune = cap(g_, er0, 1, Some(pid));
                let t = |k: &str, v: Value| Value::Tuple(vec![k.into(), v]);
                let mut opts: Vec<Value> = mine.iter().map(|&x| t("obj", x.into())).collect();
                opts.extend(rs.iter().map(|&r| t("rune", r.into())));
                opts.extend(legs.iter().map(|&x| t("legend", x.into())));
                opts.extend(other.iter().map(|&x| t("obj", x.into())));
                opts.extend(erune.iter().map(|&r| t("rune", r.into())));
                if opts.is_empty() {
                    return Ok(Value::Bool(false));
                }
                let p = g_.ask(pid, "mf_ready", opts, map! { item = it })?;
                let (k, x) = (p.list()[0].str().to_string(), p.list()[1].clone());
                if k == "obj" {
                    if !pay_deflect(g_, it, x.obj())? {
                        return Ok(Value::Bool(false));
                    }
                    g_.add_target(it, x.obj(), Value::None);
                } else {
                    g_.it_mut(it).data.set("thing", Value::Tuple(vec![k.as_str().into(), x]));
                }
                Ok(Value::Bool(true))
            });
            let res = fv("res", |g_, a| {
                let it = a[0].item();
                if !g_.it(it).targets.is_empty() {
                    if let Some(x) = g_.legal(it, 0)? {
                        g_.ready_obj(x, None, None)?;
                    }
                } else if g_.it(it).data.g("thing").truthy() {
                    match g_.it(it).data.g("thing").list()[1].clone() {
                        Value::Rune(r) => g_.r_mut(r).exhausted = false,
                        Value::Obj(x) => g_.set_exhausted(x, false),
                        _ => {}
                    }
                }
                Ok(Value::None)
            });
            trig_full(g, o, "Miss Fortune, Captain", res, Map::new(), true, choose, Value::None);
            Ok(Value::None)
        });
        Impl { accelerate: true, kw: map! { Ganking = 1 }, on_event, ..new("Miss Fortune, Captain") }
    }

    // Sivir, Ambitious — "[Deflect 2] When I conquer after an attack, if you assigned 5 or more excess damage to enemy
    // units, you may deal that much to an enemy unit." Excess damage on a unit = damage assigned to it beyond the
    // lethal damage it needed when damage was assigned (rule 142.4.b), from the attacker's assignment
    // (Showdown.assigned). `_body_sivir` = (turn_no, bf, {uid: excès}) ; le dict à clés entières devient une Map à
    // clés texte, sommée dans l'ordre des entiers.
    pub fn sivir() -> Impl {
        let on_event = fv("_sivir", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map().clone());
            if ev == "combat_start" {
                let rec =
                    Value::Tuple(vec![g.turn_no.into(), Value::Int(bfi(&info.g("bf")) as i64), Map::new().into()]);
                g.o_mut(o).attrs.set("_body_sivir", rec);
            } else if ev == "damaged" && info.g("kind").opt_str() == Some("combat") {
                let u = info.g("obj").obj();
                let rec = g.o(o).attrs.g("_body_sivir");
                let Some(sd) = g.sd.clone() else { return Ok(Value::None) };
                if sd.assigned.is_none()
                    || g.o(u).ctrl == g.o(o).ctrl
                    || sd.attacker != g.o(o).ctrl
                    || rec.is_none()
                    || rec.list()[0] != Value::Int(g.turn_no)
                    || rec.list()[1] != Value::Int(sd.bf as i64)
                {
                    return Ok(Value::None);
                }
                let a_ = sd.assigned.list()[0]
                    .list()
                    .iter()
                    .find(|p| p.list()[0].opt_obj() == Some(u))
                    .map(|p| p.list()[1].int())
                    .unwrap_or(0);
                let n = info.g("amount").int();
                let d0 = g.o(u).damage - n; // dégâts marqués avant ces dégâts de combat
                let p0 = (a_ - n) + g.o(u).prevent; // prévention qu'elle avait quand les dégâts ont été assignés
                let need = 1.max(g.might(u)? - d0) + p0;
                let key = g.o(u).uid.to_string();
                let mut rec = rec;
                rec.list_mut()[2].map_mut().set(&key, 0.max(a_ - need));
                g.o_mut(o).attrs.set("_body_sivir", rec);
            } else if ev == "conquer" && in_units(&info, o) {
                let rec = g.o(o).attrs.g("_body_sivir");
                let Some(sd) = g.sd.clone() else { return Ok(Value::None) };
                if !sd.combat
                    || sd.attacker != g.o(o).ctrl
                    || sd.bf != bfi(&info.g("bf"))
                    || rec.is_none()
                    || rec.list()[0] != Value::Int(g.turn_no)
                    || rec.list()[1] != Value::Int(sd.bf as i64)
                {
                    return Ok(Value::None);
                }
                let m = rec.list()[2].map();
                let mut ks: Vec<i64> = m.keys().map(|k| k.parse().unwrap()).collect();
                ks.sort();
                let ex: i64 = ks.iter().map(|k| m.g(&k.to_string()).int()).sum();
                if ex < 5 {
                    return Ok(Value::None);
                }
                let res = fv("res", |g_, a| {
                    let it = a[0].item();
                    let t = g_.legal(it, 0)?;
                    let (d, c) = (g_.it(it).data.g("dmg").int(), ctrl(g_, it));
                    g_.deal(t, d, "ability", Some(c), None)?;
                    Ok(Value::None)
                });
                let choose = pick(
                    |g_, it| {
                        let c = ctrl(g_, it);
                        enemies(g_, c, false, None, true)
                    },
                    p_enemy_v(),
                    1,
                    "target",
                    true,
                );
                trig_full(g, o, "Sivir, Ambitious", res, map! { dmg = ex }, true, choose, Value::None);
            }
            Ok(Value::None)
        });
        Impl { kw: map! { Deflect = 2 }, on_event, ..new("Sivir, Ambitious") }
    }

    // Udyr, Wildman — "Spend my buff: Choose one you've not chosen this turn — Deal 2 to a unit at a battlefield.
    // Stun a unit at a battlefield. Ready me. Give me [Ganking] this turn."
    fn udyr_used(g: &Game, o: O) -> Vec<String> {
        let rec = g.o(o).attrs.g("_body_udyr");
        if rec.is_none() || rec.list()[0] != Value::Int(g.turn_no) || rec.list()[1] != Value::Int(g.o(o).oid) {
            return vec![];
        }
        rec.list()[2].list().iter().map(|v| v.str().to_string()).collect()
    }

    pub fn udyr() -> Impl {
        let choices = fv("_udyr_choices", |g, a| {
            let (pid, o) = (a[0].int() as usize, a[1].obj());
            let used = udyr_used(g, o);
            let has = |k: &str| used.iter().any(|x| x == k);
            let mut out = vec![];
            if !has("ready") && g.exhausted(o) {
                out.push(map! { mode = "ready" });
            }
            if !has("dmg") {
                let mut es = vec![];
                for u in enemies(g, pid, true, None, true)? {
                    if g.might(u)? - g.o(u).damage <= 2 {
                        es.push(u);
                    }
                }
                for u in enemies(g, pid, true, None, true)? {
                    if g.might(u)? - g.o(u).damage > 2 {
                        es.push(u);
                    }
                }
                for u in cap(g, es, 3, Some(pid)) {
                    out.push(map! { mode = "dmg", tg = tup(&[g.o(u).uid]) });
                }
            }
            if !has("stun") {
                let st: Vec<Map> = enemies(g, pid, true, None, true)?
                    .into_iter()
                    .filter(|&u| !g.o(u).stunned)
                    .map(|u| map! { mode = "stun", tg = tup(&[g.o(u).uid]) })
                    .collect();
                out.extend(cap(g, st, 2, Some(pid)));
            }
            if !has("gank") && !g.has_kw(o, "Ganking")? {
                out.push(map! { mode = "gank" });
            }
            if !has("ready") && !g.exhausted(o) {
                out.push(map! { mode = "ready" });
            }
            Ok(maps(out))
        });
        let pay = fv("_udyr_pay", |g, a| {
            let (o, ch) = (a[1].obj(), a[2].map().clone());
            spend_buff(g, Some(o))?;
            let mut used: Vec<Value> = udyr_used(g, o).into_iter().map(Value::from).collect();
            used.push(ch.g("mode"));
            let rec = Value::Tuple(vec![g.turn_no.into(), g.o(o).oid.into(), Value::Tuple(used)]);
            g.o_mut(o).attrs.set("_body_udyr", rec);
            Ok(Value::None)
        });
        let res = fv("_udyr_res", |g, a| {
            let it = a[0].item();
            let m = g.it(it).data.g("mode").str().to_string();
            let me = src_board(g, it);
            let c = ctrl(g, it);
            if m == "dmg" || m == "stun" {
                if let Some(u) = g.legal(it, 0)? {
                    if m == "dmg" {
                        g.deal(Some(u), 2, "ability", Some(c), None)?;
                    } else {
                        g.stun(Some(u), Some(c))?;
                    }
                }
            } else if let Some(me) = me {
                if m == "ready" {
                    g.ready_obj(me, None, None)?;
                } else {
                    g.grant(me, "Ganking", 1, turn());
                }
            }
            Ok(Value::None)
        });
        Impl {
            abilities: vec![ability(
                "Spend buff",
                AbilityArgs {
                    can: fv("can", |g, a| Ok(Value::Bool(g.o(a[1].obj()).buff > 0))),
                    choices,
                    preds: Value::List(vec![p_unit_bf_v()]),
                    extra_cost: pay,
                    resolve: res,
                    ..Default::default()
                },
            )],
            ..new("Udyr, Wildman")
        }
    }
}
