//! Batch order : les cartes Order de `batches/order.txt` (unités, sorts, équipements). Traduction ligne à ligne de
//! `riftbound/engine/cardsets/order.py` (référence tant que le portage n'est pas fini ; voir
//! `docs/memoire/portage-rust.md`).
//!
//! Carte non enregistrée ici (bloquée) : Hungry Wolf (il manque `Hist.wolf_used`, voir le compte rendu du portage).
use std::collections::BTreeSet;

use crate::actions::{attach, card_choices, full_choices, pay_ctx, play_card, total_cost};
use crate::cards::{Impl, Registry};
use crate::cards_helpers::{
    ability, add_ability, all_units, cap, enemies, flow_cost, friends, impl_empower, impl_equip, is_token,
    make_token, may_pay, p_enemy, p_enemy_bf_v, p_enemy_v, p_friend, p_friend_bf_v, p_friend_v, p_gear_v, p_unit_bf,
    p_unit_v, p_unit_bf_v, play_unit_free, repeat_cost, repeatable, tg_choices, token_locations, trig_target,
    unit_play_choices, value, when_empowered, when_mighty, AbilityArgs, Cost,
};
use crate::game::{Desig, Game, ItemId, Res, Zone, O};
use crate::game_costs::{any_req, reqs_to_value, Req};
use crate::map;
use crate::spec::{spec, specs};
use crate::value::{is_bf, Loc, Map, Value};

/// `ORDER = frozenset({"Order"})`
fn order() -> Req {
    ["Order".to_string()].into_iter().collect()
}

/// `_PETS`
const PETS: [&str; 4] = ["Bird", "Cat", "Dog", "Poro"];

/// `card(name, ...)` de ce module.
fn imp(name: &str) -> Impl {
    Impl { module: Some("order"), ..Impl::new(name) }
}

// ====================================================================== private helpers
/// `sorted(xs, key=key)` : clés calculées une fois chacune, dans l'ordre de la liste, tri stable.
fn sorted_by<T, K: PartialOrd>(
    g: &mut Game,
    xs: Vec<T>,
    mut key: impl FnMut(&mut Game, &T) -> Res<K>,
) -> Res<Vec<T>> {
    let mut ks = Vec::with_capacity(xs.len());
    for x in xs {
        let k = key(g, &x)?;
        ks.push((k, x));
    }
    ks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Ok(ks.into_iter().map(|(_, x)| x).collect())
}

fn vals(v: &[O]) -> Vec<Value> {
    v.iter().map(|&o| Value::Obj(o)).collect()
}

fn maps(v: Vec<Map>) -> Value {
    Value::List(v.into_iter().map(Value::Map).collect())
}

fn tg1(uid: u32) -> Value {
    Value::Tuple(vec![uid.into()])
}

fn tg2(a: u32, b: u32) -> Value {
    Value::Tuple(vec![a.into(), b.into()])
}

/// `c.spec["e"] + 2 * c.spec["p"]`
fn cost_of(g: &Game, c: O) -> i64 {
    let sp = g.spec(c);
    sp.e as i64 + 2 * sp.p as i64
}

/// `info["info"]["spec"]["type"]` : la spec d'un look-back est rangée par son nom.
fn lb_type(inf: &Map) -> &'static str {
    &spec(inf.g("spec").str()).type_
}

/// `x in lst` pour une liste d'objets rangée dans une valeur.
fn has_obj(v: &Value, o: O) -> bool {
    v.list().iter().any(|x| x.opt_obj() == Some(o))
}

fn ctx_hb(ctx: &Value) -> Option<Loc> {
    ctx.map().g("hidden_bf").opt_loc()
}

/// `(e, reqs)` rendu par une fonction de coût.
fn cost_val(e: i64, reqs: &[Req]) -> Value {
    Value::Tuple(vec![e.into(), reqs_to_value(reqs)])
}

fn de_dp(e: i64, p: i64) -> Value {
    Value::Tuple(vec![e.into(), p.into()])
}

/// `_tokens(g, pid, name, n, loc)` : joue n jetons. loc None : l'emplacement de chaque jeton est choisi quand il est
/// joué (règle 355.2, les jetons peuvent aller à des emplacements différents) ; les jetons équipement vont à la base.
fn tokens(g: &mut Game, pid: usize, name: &str, n: i64, loc: Option<Loc>) -> Res<Vec<Option<O>>> {
    let mut out = vec![];
    for _ in 0..n {
        let l = match loc {
            Some(l) => l,
            None => {
                if spec(name).type_ == "Gear" {
                    Loc::Base
                } else {
                    let ls = token_locations(g, pid, name, None)?;
                    if ls.is_empty() {
                        continue;
                    }
                    g.ask(pid, "token_location", ls.into_iter().map(Value::Loc).collect(), map! { token = name })?
                        .loc()
                }
            }
        };
        out.push(make_token(g, name, pid, l, false)?);
    }
    Ok(out)
}

/// `_me(g, it)`
fn me(g: &Game, it: ItemId) -> Option<O> {
    g.it(it).src.and_then(|s| g.obj(s))
}

/// `_here(g, it, default)` : l'emplacement de la source si elle est encore sur le plateau, sinon celui retenu.
fn here(g: &Game, it: ItemId, default: Option<Loc>) -> Option<Loc> {
    match me(g, it) {
        Some(o) => g.o(o).loc,
        None => default,
    }
}

/// `_is_pet(spec)` : une ligne de SPEC ou un ensemble de tags (Game.tags).
fn is_pet(tags: &BTreeSet<String>) -> bool {
    PETS.iter().any(|t| tags.contains(*t))
}

/// `_trash_card(g, pid, uid, oid)`
fn trash_card(g: &Game, pid: usize, uid: i64, oid: i64) -> Option<O> {
    g.p[pid].trash.iter().copied().find(|&c| g.o(c).uid as i64 == uid && g.o(c).oid == oid)
}

/// `*it.data["pick"]`
fn pick_of(v: &Value) -> (i64, i64) {
    let l = v.list();
    (l[0].int(), l[1].int())
}

/// `_pairs(us, n=4, g, pid)`
fn pairs(g: &Game, us: Vec<O>, n: usize, pid: usize) -> Vec<Map> {
    let us = cap(g, us, n, Some(pid));
    let mut out = vec![];
    for (i, &a) in us.iter().enumerate() {
        for &b in &us[i + 1..] {
            out.push(map! { tg = tg2(g.o(a).uid, g.o(b).uid) });
        }
    }
    out
}

/// `_choose_more(g, pid, kind, cands, n, key)` : pid choisit jusqu'à n objets un par un (le meilleur d'abord pour
/// l'IA). `cands` est déjà trié par la clé.
fn choose_more(g: &mut Game, pid: usize, kind: &str, cands: Vec<Value>, n: usize) -> Res<Vec<Value>> {
    let mut picked: Vec<Value> = vec![];
    let mut cands = cands;
    while !cands.is_empty() && picked.len() < n {
        let c = if cands.len() > 1 {
            g.ask(pid, kind, cands.clone(), map! { n = n - picked.len() })?
        } else {
            cands[0].clone()
        };
        picked.push(c.clone());
        if let Some(i) = cands.iter().position(|x| *x == c) {
            cands.remove(i);
        }
    }
    Ok(picked)
}

/// `_two_mod(n)`
fn two_mod(n: i64) -> Value {
    Value::func("_two_mod", move |g, a| {
        let it = a[0].item();
        for i in 0..2 {
            if let Some(u) = g.legal(it, i)? {
                g.mod_(u, n, "turn".into(), None)?;
            }
        }
        Ok(Value::None)
    })
}

/// `_kill_tg(g, it)`
fn kill_tg() -> Value {
    Value::func("_kill_tg", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            let c = g.it(it).ctrl;
            g.kill(&[u], Some(c), false)?;
        }
        Ok(Value::None)
    })
}

/// `_p_small(n, base=P_unit_bf)`
fn p_small(n: i64) -> Value {
    Value::func("_p_small", move |g, a| {
        let (it, o) = (a[0].item(), a[1].obj());
        Ok((p_unit_bf(g, it, o) && g.might(o)? <= n).into())
    })
}

/// `_move_tokens(name, n, token="Recruit")`
fn move_tokens(name: &'static str, n: i64) -> Value {
    let token = "Recruit";
    Value::func("_move_tokens", move |g, a| {
        let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
        if ev == "move" && info.g("obj").opt_obj() == Some(o) && is_bf(info.g("to").opt_loc()) {
            let to = info.g("to").opt_loc();
            let res = Value::func("_move_tokens.res", move |g_, a_| {
                let it = a_[0].item();
                let c = g_.it(it).ctrl;
                let l = here(g_, it, to);
                tokens(g_, c, token, n, l)?;
                Ok(Value::None)
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, name, res, Map::new(), Some(uid), false, Value::None, Value::None);
        }
        Ok(Value::None)
    })
}

/// `_play_tokens_here(name, n, token)` (le corps de l'on_play)
fn play_tokens_here_do(g: &mut Game, o: O, name: &'static str, n: i64, token: &'static str) {
    let h = g.o(o).loc;
    let res = Value::func("_play_tokens_here.res", move |g_, a_| {
        let it = a_[0].item();
        let c = g_.it(it).ctrl;
        let l = here(g_, it, h);
        tokens(g_, c, token, n, l)?;
        Ok(Value::None)
    });
    let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
    g.queue_trigger(c, name, res, Map::new(), Some(uid), false, Value::None, Value::None);
}

/// `_play_tokens_here(name, n, token)`
fn play_tokens_here(name: &'static str, n: i64, token: &'static str) -> Value {
    Value::func("_play_tokens_here", move |g, a| {
        play_tokens_here_do(g, a[0].obj(), name, n, token);
        Ok(Value::None)
    })
}

/// `_other_here_aura(g, src, o)`
fn other_here_aura() -> Value {
    Value::func("_other_here_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        let ok = g.spec(src).type_ == "Unit"
            && o != src
            && g.o(o).ctrl == g.o(src).ctrl
            && g.spec(o).type_ == "Unit"
            && g.o(o).loc == g.o(src).loc;
        Ok(Value::Int(if ok { 1 } else { 0 }))
    })
}

/// `_small_trash_units(g, pid, e, p)`
fn small_trash_units(g: &mut Game, pid: usize, e: i64, p: i64) -> Res<Vec<O>> {
    let cs: Vec<O> = g.p[pid]
        .trash
        .iter()
        .copied()
        .filter(|&c| {
            let sp = g.spec(c);
            sp.type_ == "Unit" && sp.e as i64 <= e && sp.p as i64 <= p
        })
        .collect();
    sorted_by(g, cs, |g, &c| {
        let sp = g.spec(c);
        Ok((-(sp.might.unwrap_or(0) as i64), -(sp.e as i64), g.cname(c).to_string()))
    })
}

/// `_dragon_base(g, o)`
fn dragon_base(g: &Game, o: O) -> i64 {
    let mut b = g.spec(o).might.unwrap_or(0) as i64;
    let (uid, oid) = (g.o(o).uid, g.o(o).oid);
    for ef in &g.effects {
        if ef.d.g("kind").opt_str() == Some("base_might") && ef.d.g("uid") == Value::from(uid) && ef.d.g("oid").int() == oid {
            b = ef.d.g("value").int();
        }
    }
    b
}

/// `_kayle_n(g, o)`
fn kayle_n(g: &Game, o: O) -> i64 {
    if !g.o(o).empowered {
        return 0;
    }
    let (uid, oid) = (g.o(o).uid, g.o(o).oid);
    for ef in &g.effects {
        if ef.d.g("kind").opt_str() == Some("order_kayle") && ef.d.g("uid") == Value::from(uid) && ef.d.g("oid").int() == oid {
            return ef.d.g("n").int();
        }
    }
    1
}

/// `_shen_ok(g, o)`
fn shen_ok(g: &Game, o: Option<O>) -> bool {
    let Some(o) = o else { return false };
    let (c, l) = (g.o(o).ctrl, g.o(o).loc);
    g.units(Some(c), l).into_iter().filter(|&u| u != o).count() == 1
}

/// `_spell_names(g, pid)` : les sorts connus de l'adversaire d'abord (défausse, bannissement), puis tous les autres
/// sorts de la base (ordre alphabétique).
fn spell_names(g: &Game, pid: usize) -> Vec<Value> {
    let opp = 1 - pid;
    let mut seen: Vec<(String, i64)> = vec![];
    for &c in g.p[opp].trash.iter().chain(g.p[opp].banish.iter()) {
        if g.spec(c).type_ == "Spell" {
            let n = g.cname(c).to_string();
            if let Some(e) = seen.iter_mut().find(|(x, _)| *x == n) {
                e.1 += 1;
            } else {
                seen.push((n, 1));
            }
        }
    }
    let mut first = seen.clone();
    first.sort_by(|a, b| (-a.1, &a.0).cmp(&(-b.1, &b.0)));
    let mut out: Vec<Value> = first.into_iter().map(|(n, _)| Value::from(n)).collect();
    for (n, s) in specs() {
        if s.type_ == "Spell" && !seen.iter().any(|(x, _)| x == n) && !is_token(n) {
            out.push(Value::from(n.as_str()));
        }
    }
    out
}

/// `_reksai_options(g, pid, c, here)`
fn reksai_options(g: &mut Game, pid: usize, c: O, here_: Option<Loc>) -> Res<Vec<Value>> {
    if g.impl_o(c).is_none() {
        return Ok(vec![]);
    }
    let out: Vec<Map> = if g.spec(c).type_ == "Unit" {
        let mut out = unit_play_choices(g, pid, c, "deck", Some(here_.into_iter().collect()))?; // "you may play it here"
        let more: Vec<Map> =
            unit_play_choices(g, pid, c, "deck", None)?.into_iter().filter(|ch| !out.contains(ch)).collect();
        out.extend(more);
        let mut kept = vec![];
        for ch in out {
            let (e, reqs) = total_cost(g, pid, c, &ch, "deck")?;
            let ctx = pay_ctx(g, c);
            if g.can_pay(pid, e, &reqs, &ctx)? {
                kept.push(ch);
            }
        }
        kept
    } else {
        card_choices(g, pid, c, "deck", false, false, false)?
    };
    Ok(cap(g, out, 6, Some(pid)).into_iter().map(|ch| Value::Tuple(vec![c.into(), ch.into()])).collect())
}

// ====================================================================== cards
pub fn register(r: &mut Registry) {
    // Albus Ferros — "When you play me, spend any number of buffs. For each buff spent, channel 1 rune exhausted."
    // (rule 702.2.b: a player spends buffs only from units they control, one buff per unit at most.)
    let albus_res = Value::func("_albus_res", |g, a| {
        let it = a[0].item();
        let pid = g.it(it).ctrl;
        let mut n = 0;
        loop {
            let us: Vec<O> = g.units(Some(pid), None).into_iter().filter(|&u| g.o(u).buff > 0).collect();
            let us = sorted_by(g, us, |g, &u| value(g, u))?;
            if us.is_empty() {
                break;
            }
            let mut opts = vals(&us);
            opts.push(Value::None);
            let u = g.ask(pid, "spend_buff", opts, map! { reason = "Albus Ferros" })?;
            let Some(u) = u.opt_obj() else { break };
            g.spend_buff(Some(u), Some(pid))?;
            n += 1;
        }
        if n != 0 {
            g.channel(pid, n, true)?;
        }
        Ok(Value::None)
    });
    r.card(Impl {
        on_play: Value::func("Albus Ferros.on_play", move |g, a| {
            let o = a[0].obj();
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, "Albus Ferros", albus_res.clone(), Map::new(), Some(uid), false, Value::None, Value::None);
            Ok(Value::None)
        }),
        ..imp("Albus Ferros")
    });

    // Altar of Memories — "When a friendly unit dies, you may exhaust me to draw 1, then put a card from your hand on
    // the top or bottom of your Main Deck."
    r.card(Impl {
        on_event: Value::func("_altar_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "die" {
                let inf = info.g("info");
                let inf = inf.map();
                if inf.g("ctrl").int() as usize == g.o(o).ctrl && lb_type(inf) == "Unit" {
                    // cost within the instruction (rule 383.3.b)
                    let cost = Value::func("_altar_event.cost", |g_, a_| {
                        let it = a_[0].item();
                        let Some(m) = me(g_, it) else { return Ok(false.into()) };
                        if g_.exhausted(m) {
                            return Ok(false.into());
                        }
                        g_.set_exhausted(m, true);
                        Ok(true.into())
                    });
                    let res = Value::func("_altar_event.res", |g_, a_| {
                        let it = a_[0].item();
                        let pid = g_.it(it).ctrl;
                        g_.draw(pid, 1)?;
                        let hand = g_.p[pid].hand.clone();
                        if hand.is_empty() {
                            return Ok(Value::None);
                        }
                        let hs = sorted_by(g_, hand, |g, &x| Ok(cost_of(g, x)))?;
                        let c = g_.ask(pid, "altar_card", vals(&hs), Map::new())?.obj();
                        let w = g_.ask(pid, "altar_where", vec!["bottom".into(), "top".into()], map! { card = c })?;
                        g_.to_zone(c, "deck", w.opt_str() == Some("bottom"), None)?;
                        Ok(Value::None)
                    });
                    let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                    g.queue_trigger(c, "Altar of Memories", res, Map::new(), Some(uid), true, Value::None, cost);
                }
            }
            Ok(Value::None)
        }),
        ..imp("Altar of Memories")
    });

    // Ambessa, Respected and Feared — "[Empower] [1][Order][Order]. [Empowered] I have [Assault 2]. [Empowered] When I
    // attack, kill an enemy unit here with less Might than me."
    fn ambessa_ok(g: &mut Game, it: ItemId, u: O, h: Option<Loc>) -> Res<bool> {
        let m = match me(g, it) {
            Some(x) => g.might(x)?,
            None => g.it(it).data.g("m").int(),
        };
        Ok(g.spec(u).type_ == "Unit" && g.o(u).ctrl != g.it(it).ctrl && g.o(u).loc == h && g.might(u)? < m)
    }
    let mut im = Impl {
        kw_if: vec![(when_empowered(), map! { Assault = 2 })],
        on_event: Value::func("_ambessa_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "attack" && info.g("obj").opt_obj() == Some(o) && g.o(o).empowered {
                let h = g.o(o).loc;
                let ok = Value::func("_ambessa_event.ok", move |g_, a_| Ok(ambessa_ok(g_, a_[0].item(), a_[1].obj(), h)?.into()));
                let res = Value::func("_ambessa_event.res", |g_, a_| {
                    let it = a_[0].item();
                    if let Some(u) = g_.legal(it, 0)? {
                        let c = g_.it(it).ctrl;
                        g_.kill(&[u], Some(c), false)?;
                    }
                    Ok(Value::None)
                });
                let opts = Value::func("_ambessa_event.options", move |g_, a_| {
                    let it = a_[0].item();
                    let c = g_.it(it).ctrl;
                    let mut out = vec![];
                    for u in enemies(g_, c, false, None, true)? {
                        if ambessa_ok(g_, it, u, h)? {
                            out.push(u);
                        }
                    }
                    Ok(out.into())
                });
                let m = g.might(o)?;
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                let choose = trig_target(opts, ok, "target", false, true);
                g.queue_trigger(c, "Ambessa, Respected and Feared", res, map! { m = m }, Some(uid), false, choose, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Ambessa, Respected and Feared")
    };
    impl_empower(&mut im, Cost::from("1 energy and 2 order runes"));
    r.card(im);

    // Aurok General — "[Empower] [3][Order]. [Empowered] Your units that are [Empowered] have +2 might (including me)."
    let mut im = Impl {
        aura_might: Value::func("Aurok General.aura_might", |g, a| {
            let (src, o) = (a[0].obj(), a[1].obj());
            let ok = g.o(src).empowered
                && g.spec(src).type_ == "Unit"
                && g.o(o).ctrl == g.o(src).ctrl
                && g.spec(o).type_ == "Unit"
                && g.o(o).empowered;
            Ok(Value::Int(if ok { 2 } else { 0 }))
        }),
        ..imp("Aurok General")
    };
    impl_empower(&mut im, Cost::from("3 energy and 1 order rune"));
    r.card(im);

    // Azir, Sovereign — "[Accelerate] When I attack, you may move any number of your token units to this battlefield."
    r.card(Impl {
        accelerate: true,
        on_event: Value::func("_azir_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "attack" && info.g("obj").opt_obj() == Some(o) {
                let h = g.o(o).loc;
                let res = Value::func("_azir_event.res", move |g_, a_| {
                    let it = a_[0].item();
                    let pid = g_.it(it).ctrl;
                    let movers: Vec<O> =
                        g_.units(Some(pid), None).into_iter().filter(|&u| g_.o(u).token && g_.o(u).loc != h).collect();
                    let mut pick = vec![];
                    for u in movers {
                        if g_.ask(pid, "azir_move", vec![true.into(), false.into()], map! { unit = u })?.truthy() {
                            pick.push(u);
                        }
                    }
                    if !pick.is_empty() {
                        if let Some(h) = h {
                            g_.move_(&pick, h, Some(pid), false)?;
                        }
                    }
                    Ok(Value::None)
                });
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                g.queue_trigger(c, "Azir, Sovereign", res, Map::new(), Some(uid), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Azir, Sovereign")
    });

    // B.F. Sword — "[Equip] [Order]" ; Might Bonus +3 (card image).
    let mut im = Impl { bonus: Some(3), ..imp("B.F. Sword") };
    impl_equip(&mut im, Cost::from("1 order rune"));
    r.card(im);

    // Back to Back — "[Reaction] Give two friendly units each +2 might this turn." (two targets, rule 355.8)
    r.card(Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![p_friend_v(), p_friend_v()]),
        resolve: two_mod(2),
        choices: Value::func("Back to Back.choices", |g, a| {
            let pid = a[0].int() as usize;
            let fs = friends(g, pid, false, ctx_hb(&a[1]))?;
            Ok(maps(pairs(g, fs, 4, pid)))
        }),
        ..imp("Back to Back")
    });

    // Bandle Soldier — "[Level 3] I enter ready."
    r.card(Impl { levels: vec![(3, map! { ready = true })], ..imp("Bandle Soldier") });

    // Blade of the Ruined King — "[Equip] — [Order], Kill a friendly unit" ; Might Bonus +4 (card image).
    let blade_choices = Value::func("_blade_choices", |g, a| {
        let (pid, o) = (a[0].int() as usize, a[1].obj());
        let fs = friends(g, pid, false, None)?;
        let fs = sorted_by(g, fs, |g, &u| Ok(-g.might(u)?))?;
        let at = g.o(o).attached_to;
        let us: Vec<O> = fs.into_iter().filter(|&u| Some(g.o(u).uid) != at).collect();
        let vs = g.units(Some(pid), None);
        let vs = sorted_by(g, vs, |g, &x| value(g, x))?;
        // killing the unit to equip would make the choice illegal (rule 355.16); each target with its cheapest victim
        // first (only the first MAX_CHOICES options are offered)
        let mut prs: Vec<(O, O)> = vec![];
        for &u in &us {
            for &v in &vs {
                if v != u {
                    prs.push((u, v));
                }
            }
        }
        let mut first: Vec<(O, O)> = vec![];
        for &u in &us {
            if let Some(&p) = prs.iter().find(|pv| pv.0 == u) {
                first.push(p);
            }
        }
        let rest: Vec<(O, O)> = prs.iter().copied().filter(|pv| !first.contains(pv)).collect();
        Ok(maps(
            first
                .into_iter()
                .chain(rest)
                .map(|(u, v)| map! { tg = tg1(g.o(u).uid), victim = g.o(v).uid })
                .collect(),
        ))
    });
    let blade_extra = Value::func("_blade_extra", |g, a| {
        let (pid, ch) = (a[0].int() as usize, a[2].map());
        let mut v = if ch.g("victim").truthy() { g.obj(ch.g("victim").int() as u32) } else { None };
        if v.is_none() {
            // Weaponmaster: the cost is paid on resolution
            let tg = match ch.get("tg") {
                Some(t) => t.list()[0].clone(),
                None => Value::None,
            };
            let us: Vec<O> =
                g.units(Some(pid), None).into_iter().filter(|&u| Value::from(g.o(u).uid) != tg).collect();
            let vs = sorted_by(g, us, |g, &x| value(g, x))?;
            v = if !vs.is_empty() { g.ask_obj(pid, "sacrifice", &vs, Map::new())? } else { None };
        }
        if let Some(v) = v {
            g.kill(&[v], Some(pid), true)?;
        }
        Ok(Value::None)
    });
    let mut ab = Map::new();
    ab.set("name", "Equip");
    ab.set("timing", "main");
    ab.set("choices", blade_choices);
    ab.set("cost", Value::func("Blade of the Ruined King.cost", |_g, _a| Ok(cost_val(0, &[order()]))));
    ab.set("preds", Value::List(vec![p_friend_v()]));
    ab.set("can", Value::func("Blade of the Ruined King.can", |g, a| Ok((g.units(Some(a[0].int() as usize), None).len() >= 2).into())));
    ab.set("extra_cost", blade_extra);
    ab.set(
        "can_extra",
        Value::func("Blade of the Ruined King.can_extra", |g, a| Ok((g.units(Some(a[0].int() as usize), None).len() >= 2).into())),
    );
    ab.set(
        "resolve",
        Value::func("Blade of the Ruined King.resolve", |g, a| {
            let it = a[0].item();
            if g.legal(it, 0)?.is_some() {
                if let Some(src) = g.it(it).src.and_then(|s| g.obj(s)) {
                    let u = g.legal(it, 0)?;
                    if let Some(u) = u {
                        attach(g, src, u)?;
                    }
                }
            }
            Ok(Value::None)
        }),
    );
    r.card(Impl { bonus: Some(4), abilities: vec![ab], ..imp("Blade of the Ruined King") });

    // Blast of Power — "[Action] Kill a unit at a battlefield."
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_bf_v()]),
        resolve: kill_tg(),
        choices: Value::func("Blast of Power.choices", |g, a| {
            let pid = a[0].int() as usize;
            let es = enemies(g, pid, true, ctx_hb(&a[1]), true)?;
            Ok(maps(tg_choices(g, &es)))
        }),
        ..imp("Blast of Power")
    });

    // Blood Money — "[Action] Kill a unit at a battlefield with 2 might or less. If it was an enemy unit, play a Gold
    // gear token exhausted. If it was a friendly unit, play two Gold gear tokens exhausted."
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_small(2)]),
        resolve: Value::func("_blood_money", |g, a| {
            let it = a[0].item();
            let Some(u) = g.legal(it, 0)? else {
                return Ok(Value::None); // linked instructions need the target (rule 359.3.e)
            };
            let c = g.it(it).ctrl;
            let friendly = g.o(u).ctrl == c;
            g.kill(&[u], Some(c), false)?;
            tokens(g, c, "Gold", if friendly { 2 } else { 1 }, None)?;
            Ok(Value::None)
        }),
        choices: Value::func("Blood Money.choices", |g, a| {
            let pid = a[0].int() as usize;
            let us = all_units(g, pid, true, ctx_hb(&a[1]))?;
            let mut out = vec![];
            for u in us {
                if g.might(u)? <= 2 {
                    out.push(u);
                }
            }
            Ok(maps(tg_choices(g, &out)))
        }),
        ..imp("Blood Money")
    });

    // Bonds of Strength — "[Reaction] [Repeat] [2] Give two friendly units each +1 might this turn."
    r.card(Impl {
        timing: Some("reaction"),
        repeat: repeat_cost("2 energy"),
        preds: Value::List(vec![p_friend_v(), p_friend_v()]),
        resolve: repeatable(two_mod(1)),
        choices: Value::func("Bonds of Strength.choices", |g, a| {
            let pid = a[0].int() as usize;
            let fs = friends(g, pid, false, ctx_hb(&a[1]))?;
            Ok(maps(pairs(g, fs, 4, pid)))
        }),
        ..imp("Bonds of Strength")
    });

    // Call to Glory — "[Reaction] As you play this, you may spend a buff as an additional cost. If you do, ignore this
    // spell's cost. Give a unit +3 might this turn."
    r.card(Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![p_unit_v()]),
        pay_extra: Value::func("_ctg_pay", |g, a| {
            let (pid, ch) = (a[0].int() as usize, a[2].map());
            if ch.g("spend").truthy() {
                if let Some(b) = g.obj(ch.g("spend").int() as u32) {
                    if g.o(b).ctrl == pid && g.o(b).buff > 0 {
                        g.spend_buff(Some(b), Some(pid))?;
                    }
                }
            }
            Ok(Value::None)
        }),
        choices: Value::func("_ctg_choices", |g, a| {
            let pid = a[0].int() as usize;
            let hb = ctx_hb(&a[1]);
            let mut us = friends(g, pid, false, hb)?;
            us.extend(enemies(g, pid, false, hb, true)?);
            let bs: Vec<O> = g.units(Some(pid), None).into_iter().filter(|&u| g.o(u).buff > 0).collect();
            let buffed = sorted_by(g, bs, |g, &u| value(g, u))?;
            let mut out = vec![];
            for &u in &us {
                if !buffed.is_empty() {
                    out.push(map! { tg = tg1(g.o(u).uid), free = true, spend = g.o(buffed[0]).uid });
                }
            }
            out.extend(tg_choices(g, &us));
            Ok(maps(out))
        }),
        resolve: Value::func("Call to Glory.resolve", |g, a| {
            let it = a[0].item();
            if g.legal(it, 0)?.is_some() {
                if let Some(u) = g.legal(it, 0)? {
                    g.mod_(u, 3, "turn".into(), None)?;
                }
            }
            Ok(Value::None)
        }),
        ..imp("Call to Glory")
    });

    // Carrion Dredger — "[Deathknell] Play a 1 might Bird unit token with [Deflect] to your base."
    r.card(Impl {
        deathknell: Value::func("Carrion Dredger.deathknell", |g, a| {
            let it = a[0].item();
            let c = g.it(it).ctrl;
            make_token(g, "Bird", c, Loc::Base, false)?;
            Ok(Value::None)
        }),
        ..imp("Carrion Dredger")
    });

    // Commander Ledros — "As you play me, you may kill any number of friendly units as an additional cost. Reduce my
    // cost by [Order] for each killed this way. [Deflect] [Ganking]"
    r.card(Impl {
        kw: map! { Deflect = 1, Ganking = 1 },
        as_played: Value::func("_ledros_as_played", |g, a| {
            let pid = a[0].int() as usize;
            let us = g.units(Some(pid), None);
            let us = sorted_by(g, us, |g, &u| value(g, u))?;
            let uids: Vec<Value> = us.iter().map(|&u| Value::from(g.o(u).uid)).collect();
            let mut out = vec![Map::new()];
            if full_choices(g, Some(pid)) {
                // a human: any set of friendly units (itertools.combinations order)
                for k in 1..=uids.len() {
                    for grp in combinations(uids.len(), k) {
                        out.push(map! { kills = Value::Tuple(grp.iter().map(|&i| uids[i].clone()).collect()) });
                    }
                }
                return Ok(maps(out));
            }
            for k in 1..=uids.len().min(4) {
                out.push(map! { kills = Value::Tuple(uids[..k].to_vec()) });
            }
            for u in uids.iter().skip(1).take(5) {
                // any single unit (prefixes: the cheapest ones)
                out.push(map! { kills = Value::Tuple(vec![u.clone()]) });
            }
            Ok(maps(out))
        }),
        pay_extra: Value::func("_ledros_pay", |g, a| {
            let (pid, ch) = (a[0].int() as usize, a[2].map());
            let vs: Vec<O> = ch
                .g("kills")
                .list()
                .iter()
                .filter_map(|u| g.obj(u.int() as u32))
                .filter(|&v| g.o(v).ctrl == pid)
                .collect();
            if !vs.is_empty() {
                g.kill(&vs, Some(pid), true)?;
            }
            Ok(Value::None)
        }),
        cost_mod: Value::func("Commander Ledros.cost_mod", |_g, a| {
            Ok(de_dp(0, a[2].map().g("kills").list().len() as i64))
        }),
        ..imp("Commander Ledros")
    });

    // Corina Veraza — "[Accelerate] When I move to a battlefield, play three 1 might Recruit unit tokens here."
    r.card(Impl { accelerate: true, on_event: move_tokens("Corina Veraza", 3), ..imp("Corina Veraza") });

    // Crimson Pigeons — "I have +2 might while I'm attacking with another unit."
    r.card(Impl {
        might_mod: Value::func("Crimson Pigeons.might_mod", |g, a| {
            let o = a[0].obj();
            let ok = g.o(o).desig == Some(Desig::Att)
                && g
                    .units(Some(g.o(o).ctrl), g.o(o).loc)
                    .into_iter()
                    .any(|u| u != o && g.o(u).desig == Some(Desig::Att));
            Ok(Value::Int(if ok { 2 } else { 0 }))
        }),
        ..imp("Crimson Pigeons")
    });

    // Cruel Patron — "As an additional cost to play me, kill a friendly unit."
    r.card(Impl {
        as_played: Value::func("Cruel Patron.as_played", |g, a| {
            let pid = a[0].int() as usize;
            let us = g.units(Some(pid), None);
            let us = sorted_by(g, us, |g, &u| value(g, u))?;
            Ok(maps(us.iter().map(|&u| map! { kill = g.o(u).uid }).collect()))
        }),
        ..imp("Cruel Patron")
    });

    // Darius, Executioner — "[Legion] — When you play me, ready me. Other friendly units have +1 might here."
    r.card(Impl {
        on_play: Value::func("_darius_play", |g, a| {
            let o = a[0].obj();
            if g.legion(g.o(o).ctrl, Some(o)) {
                let res = Value::func("_darius_play.res", |g_, a_| {
                    let it = a_[0].item();
                    if let Some(m) = me(g_, it) {
                        g_.ready_obj(m, None, None)?;
                    }
                    Ok(Value::None)
                });
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                g.queue_trigger(c, "Darius, Executioner", res, Map::new(), Some(uid), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        aura_might: other_here_aura(),
        ..imp("Darius, Executioner")
    });

    // Disciple of Shen — "[Hidden] I have [Shield 3] while I'm at a battlefield with exactly one other unit you control."
    r.card(Impl {
        hidden: true,
        kw_if: vec![(
            Value::func("Disciple of Shen.cond", |g, a| {
                let o = a[0].obj();
                let l = g.o(o).loc;
                Ok((is_bf(l) && g.units(Some(g.o(o).ctrl), l).into_iter().filter(|&u| u != o).count() == 1).into())
            }),
            map! { Shield = 3 },
        )],
        ..imp("Disciple of Shen")
    });

    // Divine Judgment — "Each player chooses 2 units, 2 gear, 2 runes, and 2 cards in their hands. Recycle the rest."
    // Not targeted (rule 355.10.e): the choices are made on resolution, starting with the turn player.
    r.card(Impl {
        resolve: Value::func("_divine", |g, a| {
            let _it = a[0].item();
            let mut keep: Vec<(usize, [Vec<Value>; 4])> = vec![];
            for pid in [g.tp, 1 - g.tp] {
                let us = g.units(Some(pid), None);
                let us = sorted_by(g, us, |g, &u| Ok(-value(g, u)?))?;
                let ku = choose_more(g, pid, "judgment_unit", vals(&us), 2)?;
                let gs = g.gear(Some(pid));
                let gs = sorted_by(g, gs, |g, &x| Ok(-cost_of(g, x)))?;
                let kg = choose_more(g, pid, "judgment_gear", vals(&gs), 2)?;
                let rs = g.p[pid].runes.clone();
                let rs = sorted_by(g, rs, |g, &r| Ok((g.r(r).exhausted, g.r(r).domain.clone())))?;
                let kr = choose_more(g, pid, "judgment_rune", rs.into_iter().map(Value::Rune).collect(), 2)?;
                let hs = g.p[pid].hand.clone();
                let hs = sorted_by(g, hs, |g, &c| Ok(-cost_of(g, c)))?;
                let kh = choose_more(g, pid, "judgment_hand", vals(&hs), 2)?;
                keep.push((pid, [ku, kg, kr, kh]));
            }
            for pid in [g.tp, 1 - g.tp] {
                let [ku, kg, kr, kh] = keep.iter().find(|(p, _)| *p == pid).unwrap().1.clone();
                let mut cards_: Vec<O> =
                    g.units(Some(pid), None).into_iter().filter(|&o| !ku.contains(&Value::Obj(o))).collect();
                cards_.extend(g.gear(Some(pid)).into_iter().filter(|&o| !kg.contains(&Value::Obj(o))));
                cards_.extend(g.p[pid].hand.iter().copied().filter(|&c| !kh.contains(&Value::Obj(c))));
                let rr: Vec<_> = g.p[pid].runes.iter().copied().filter(|&r| !kr.contains(&Value::Rune(r))).collect();
                for r_ in rr {
                    g.recycle_rune(pid, r_)?;
                }
                if !cards_.is_empty() {
                    g.recycle_cards(pid, &cards_)?;
                }
            }
            Ok(Value::None)
        }),
        ..imp("Divine Judgment")
    });

    // Divining Shells — "[Vision] [Action] Kill this, [E]: Give a unit +2 might this turn."
    r.card(Impl {
        kw: map! { Vision = 1 },
        abilities: vec![ability(
            "Shells",
            AbilityArgs {
                timing: "action",
                exhaust: true,
                kill_self: true,
                preds: Value::List(vec![p_unit_v()]),
                choices: Value::func("Divining Shells.choices", |g, a| {
                    let fs = friends(g, a[0].int() as usize, false, None)?;
                    Ok(maps(tg_choices(g, &fs)))
                }),
                resolve: Value::func("Divining Shells.resolve", |g, a| {
                    let it = a[0].item();
                    if g.legal(it, 0)?.is_some() {
                        if let Some(u) = g.legal(it, 0)? {
                            g.mod_(u, 2, "turn".into(), None)?;
                        }
                    }
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..imp("Divining Shells")
    });

    // Drag Under — "[Action] I cost [2] less to play from anywhere other than your hand. Kill a unit at a battlefield."
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![p_unit_bf_v()]),
        resolve: kill_tg(),
        cost_mod: Value::func("Drag Under.cost_mod", |g, a| {
            let c = a[1].obj();
            Ok(if g.o(c).zone != Some(Zone::Hand) { de_dp(2, 0) } else { de_dp(0, 0) })
        }),
        choices: Value::func("Drag Under.choices", |g, a| {
            let pid = a[0].int() as usize;
            let es = enemies(g, pid, true, ctx_hb(&a[1]), true)?;
            Ok(maps(tg_choices(g, &es)))
        }),
        ..imp("Drag Under")
    });

    // Dragon Form — "Choose a unit. Its base Might becomes 5 this turn. [Flow] [3]"
    // A 'base_might' effect read by Game.might (the base replaces the printed or copied Might, rule 477.3; the latest
    // one wins).
    r.card(Impl {
        flow: flow_cost("3 energy"),
        preds: Value::List(vec![p_unit_v()]),
        resolve: Value::func("_dragon_form", |g, a| {
            let it = a[0].item();
            let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
            let (c, uid, oid) = (g.it(it).ctrl, g.o(u).uid, g.o(u).oid);
            g.add_effect(map! { kind = "base_might", pid = c, uid = uid, oid = oid, value = 5, dur = "turn" });
            Ok(Value::None)
        }),
        choices: Value::func("_dragon_choices", |g, a| {
            let pid = a[0].int() as usize;
            let hb = ctx_hb(&a[1]);
            let fr: Vec<O> = friends(g, pid, false, hb)?.into_iter().filter(|&u| dragon_base(g, u) < 5).collect();
            let fr = sorted_by(g, fr, |g, &u| Ok(dragon_base(g, u)))?;
            let en: Vec<O> = enemies(g, pid, false, hb, true)?.into_iter().filter(|&u| dragon_base(g, u) > 5).collect();
            let en = sorted_by(g, en, |g, &u| Ok(-dragon_base(g, u)))?;
            let rest: Vec<O> =
                all_units(g, pid, false, hb)?.into_iter().filter(|u| !fr.contains(u) && !en.contains(u)).collect();
            let all: Vec<O> = fr.into_iter().chain(en).chain(rest).collect();
            Ok(maps(tg_choices(g, &all)))
        }),
        ..imp("Dragon Form")
    });

    // Eminent Benefactor — "When I hold, play two Gold gear tokens exhausted."
    r.card(Impl {
        on_event: Value::func("Eminent Benefactor.on_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "hold" && has_obj(&info.g("units"), o) {
                let res = Value::func("Eminent Benefactor.res", |g_, a_| {
                    let c = g_.it(a_[0].item()).ctrl;
                    tokens(g_, c, "Gold", 2, None)?;
                    Ok(Value::None)
                });
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                g.queue_trigger(c, "Eminent Benefactor", res, Map::new(), Some(uid), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Eminent Benefactor")
    });

    // Enthralling Protector — "[Hunt] Spend 2 XP: [Buff] me."
    r.card(Impl {
        kw: map! { Hunt = 1 },
        abilities: vec![ability(
            "Buff",
            AbilityArgs {
                xp: 2,
                resolve: Value::func("Enthralling Protector.resolve", |g, a| {
                    let it = a[0].item();
                    if me(g, it).is_some() {
                        let m = me(g, it);
                        g.buff(m, None)?;
                    }
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..imp("Enthralling Protector")
    });

    // Escaped Grayback — "[Empower] — Kill a friendly unit. [Empowered] I have +2 might."
    r.card(Impl {
        might_if: vec![(when_empowered(), 2)],
        abilities: vec![ability(
            "Empower",
            AbilityArgs {
                can: Value::func("Escaped Grayback.can", |g, a| {
                    let (pid, o) = (a[0].int() as usize, a[1].obj());
                    Ok((!g.o(o).empowered && g.units(Some(pid), None).into_iter().any(|u| u != o)).into())
                }),
                choices: Value::func("Escaped Grayback.choices", |g, a| {
                    let (pid, o) = (a[0].int() as usize, a[1].obj());
                    let us = g.units(Some(pid), None);
                    let us = sorted_by(g, us, |g, &u| value(g, u))?;
                    Ok(maps(us.into_iter().filter(|&u| u != o).map(|u| map! { victim = g.o(u).uid }).collect()))
                }),
                extra_cost: Value::func("Escaped Grayback.extra_cost", |g, a| {
                    let (pid, ch) = (a[0].int() as usize, a[2].map());
                    let v: Vec<O> = g.obj(ch.g("victim").int() as u32).into_iter().collect();
                    let k = g.kill(&v, Some(pid), true)?;
                    Ok(k.into())
                }),
                resolve: Value::func("_grayback_empower", |g, a| {
                    let it = a[0].item();
                    if let Some(m) = me(g, it) {
                        g.empower(Some(m), None)?;
                    }
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..imp("Escaped Grayback")
    });

    // Eye of the Herald — "[Equip] [Order]" ; Might Bonus +0 and Effect Text (card image): "When I move, play a 1 might
    // Recruit unit token here."
    let mut im = Impl {
        bonus: Some(0),
        effect_event: Value::func("_eye_effect", |g, a| {
            let (unit, ev, info) = (a[1].obj(), a[2].str(), a[3].map());
            if ev == "move" && info.g("obj").opt_obj() == Some(unit) {
                let to = info.g("to").opt_loc();
                let res = Value::func("_eye_effect.res", move |g_, a_| {
                    let it = a_[0].item();
                    let c = g_.it(it).ctrl;
                    let l = here(g_, it, to);
                    tokens(g_, c, "Recruit", 1, l)?;
                    Ok(Value::None)
                });
                let (c, uid) = (g.o(unit).ctrl, g.o(unit).uid);
                g.queue_trigger(c, "Eye of the Herald", res, Map::new(), Some(uid), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Eye of the Herald")
    };
    impl_equip(&mut im, Cost::from("1 order rune"));
    r.card(im);

    // Facebreaker — "[Hidden] [Action] Stun a friendly unit and an enemy unit at the same battlefield."
    r.card(Impl {
        timing: Some("action"),
        hidden: true,
        preds: Value::List(vec![p_friend_bf_v(), p_enemy_bf_v()]),
        resolve: Value::func("_facebreaker", |g, a| {
            let it = a[0].item();
            for i in 0..2 {
                if let Some(u) = g.legal(it, i)? {
                    let c = g.it(it).ctrl;
                    g.stun(Some(u), Some(c))?;
                }
            }
            Ok(Value::None)
        }),
        choices: Value::func("_facebreaker_choices", |g, a| {
            let pid = a[0].int() as usize;
            let mut out = vec![];
            for e in enemies(g, pid, true, ctx_hb(&a[1]), true)? {
                let fs = g.units(Some(pid), g.o(e).loc);
                for f in sorted_by(g, fs, |g, &u| value(g, u))? {
                    out.push(map! { tg = tg2(g.o(f).uid, g.o(e).uid) });
                }
            }
            Ok(maps(out))
        }),
        ..imp("Facebreaker")
    });

    // Faithful Manufactor — "When you play me, play a 1 might Recruit unit token here."
    r.card(Impl { on_play: play_tokens_here("Faithful Manufactor", 1, "Recruit"), ..imp("Faithful Manufactor") });

    // Fiora, Victorious — "While I'm [Mighty], I have [Deflect], [Ganking], and [Shield]."
    r.card(Impl {
        kw_if: vec![(when_mighty(), map! { Deflect = 1, Ganking = 1, Shield = 1 })],
        ..imp("Fiora, Victorious")
    });

    // Fiora, Worthy — "When a unit you control becomes [Mighty], you may pay [Order] to ready it."
    r.card(Impl {
        track_mighty: true,
        on_event: Value::func("_fiora_worthy", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "becomes_mighty" && g.o(info.g("obj").obj()).ctrl == g.o(o).ctrl {
                let u = info.g("obj").obj();
                let res = Value::func("_fiora_worthy.res", |g_, a_| {
                    let it = a_[0].item();
                    let d = g_.it(it).data.clone();
                    if let Some(x) = g_.obj(d.g("uid").int() as u32) {
                        if g_.o(x).oid == d.g("oid").int() {
                            g_.ready_obj(x, None, None)?;
                        }
                    }
                    Ok(Value::None)
                });
                let reqs = Value::func("_fiora_worthy.reqs", |_g, _a| Ok(reqs_to_value(&[order()])));
                let (c, uid, uu, uo) = (g.o(o).ctrl, g.o(o).uid, g.o(u).uid, g.o(u).oid);
                g.queue_trigger(c, "Fiora, Worthy", res, map! { uid = uu, oid = uo }, Some(uid), true, Value::None, may_pay(0, reqs));
            }
            Ok(Value::None)
        }),
        ..imp("Fiora, Worthy")
    });

    // Forge of the Future — "When you play this, play a 1 might Recruit unit token at your base. Kill this: Recycle up
    // to 4 cards from trashes."
    r.card(Impl {
        on_play: Value::func("Forge of the Future.on_play", |g, a| {
            let o = a[0].obj();
            let res = Value::func("Forge of the Future.res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                tokens(g_, c, "Recruit", 1, Some(Loc::Base))?;
                Ok(Value::None)
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, "Forge of the Future", res, Map::new(), Some(uid), false, Value::None, Value::None);
            Ok(Value::None)
        }),
        abilities: vec![ability(
            "Recycle",
            AbilityArgs {
                kill_self: true,
                resolve: Value::func("_forge_res", |g, a| {
                    let it = a[0].item();
                    let pid = g.it(it).ctrl;
                    let mut picked: Vec<O> = vec![];
                    while picked.len() < 4 {
                        let mut cands: Vec<Value> = vec![];
                        for p in [1 - pid, pid] {
                            for &c in &g.p[p].trash {
                                if !picked.contains(&c) {
                                    cands.push(c.into());
                                }
                            }
                        }
                        if cands.is_empty() {
                            break;
                        }
                        cands.push(Value::None);
                        let c = g.ask(pid, "forge_recycle", cands, map! { n = 4 - picked.len() })?;
                        let Some(c) = c.opt_obj() else { break };
                        picked.push(c);
                    }
                    if !picked.is_empty() {
                        g.recycle_cards(pid, &picked)?; // each card goes to the bottom of its owner's Main Deck
                    }
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..imp("Forge of the Future")
    });

    // Garen, Commander — "Other friendly units have +1 might here."
    r.card(Impl { aura_might: other_here_aura(), ..imp("Garen, Commander") });

    // Glowstone — "[Empower] [A][A]. Disempower this, [E]: Choose a player. They gain control of this and recall it.
    // At the end of your turn, kill this and deal 5 to all units you control."
    let mut im = Impl {
        on_event: Value::func("_glow_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "end_turn" && info.g("pid").int() as usize == g.o(o).ctrl {
                let res = Value::func("_glow_event.res", |g_, a_| {
                    let it = a_[0].item();
                    let d = g_.it(it).data.clone();
                    let c = g_.it(it).ctrl;
                    if let Some(m) = g_.obj(d.g("uid").int() as u32) {
                        if g_.o(m).oid == d.g("oid").int() {
                            g_.kill(&[m], Some(c), false)?;
                        }
                    }
                    for u in g_.units(Some(c), None) {
                        g_.deal(Some(u), 5, "ability", Some(c), None)?;
                    }
                    Ok(Value::None)
                });
                let (c, uid, oid) = (g.o(o).ctrl, g.o(o).uid, g.o(o).oid);
                g.queue_trigger(c, "Glowstone", res, map! { uid = uid, oid = oid }, Some(uid), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        abilities: vec![ability(
            "Give",
            AbilityArgs {
                exhaust: true,
                can: Value::func("Glowstone.can", |g, a| Ok(g.o(a[1].obj()).empowered.into())),
                choices: Value::func("Glowstone.choices", |_g, a| {
                    let pid = a[0].int() as usize;
                    Ok(maps(vec![map! { player = 1 - pid }, map! { player = pid }]))
                }),
                extra_cost: Value::func("Glowstone.extra_cost", |g, a| Ok(g.disempower(Some(a[1].obj())).into())),
                resolve: Value::func("_glow_give", |g, a| {
                    let it = a[0].item();
                    let Some(m) = me(g, it) else { return Ok(Value::None) };
                    let p = g.it(it).data.g("player").int() as usize;
                    g.o_mut(m).ctrl = p;
                    g.recall(m);
                    let rm = g.repr(m);
                    g.log(|| format!("  P{p} gains control of {rm}"));
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..imp("Glowstone")
    };
    impl_empower(&mut im, Cost::from("2 runes of any type"));
    r.card(im);

    // Grand Strategem — "[Action] Give friendly units +5 might this turn."
    r.card(Impl {
        timing: Some("action"),
        resolve: Value::func("_strategem", |g, a| {
            let c = g.it(a[0].item()).ctrl;
            for u in g.units(Some(c), None) {
                g.mod_(u, 5, "turn".into(), None)?;
            }
            Ok(Value::None)
        }),
        ..imp("Grand Strategem")
    });

    // Guards! — "[Hidden] Play a 2 might Sand Soldier unit token. You may pay [Order] to ready it."
    // From Hidden, the token is played at that battlefield (rule 811.1.d.3).
    r.card(Impl {
        hidden: true,
        resolve: Value::func("_guards", |g, a| {
            let it = a[0].item();
            let pid = g.it(it).ctrl;
            let hb = g.it(it).data.g("hidden_bf").opt_loc();
            // Python : `_tokens(...)[0]` (IndexError si aucun emplacement n'est possible : porté tel quel)
            let t = tokens(g, pid, "Sand Soldier", 1, hb)?[0];
            if let Some(t) = t {
                if g.can_pay(pid, 0, &[order()], &Value::None)?
                    && g.ask(pid, "may", vec![true.into(), false.into()], map! { item = it, reason = "Guards! ready" })?.truthy()
                    && g.pay(pid, 0, &[order()], &Value::None)?
                {
                    g.ready_obj(t, None, None)?;
                }
            }
            Ok(Value::None)
        }),
        ..imp("Guards!")
    });

    // Heroic Charge — "[Action] Give a friendly unit +1 might this turn and [Stun] an enemy unit at its location."
    r.card(Impl {
        timing: Some("action"),
        preds: Value::List(vec![
            p_friend_v(),
            Value::func("_hc_enemy", |g, a| {
                let (it, o) = (a[0].item(), a[1].obj());
                let f = g.it(it).targets.first().and_then(|t| t.0).and_then(|u| g.obj(u));
                Ok((p_enemy(g, it, o) && f.is_some_and(|f| g.o(o).loc == g.o(f).loc)).into())
            }),
        ]),
        resolve: Value::func("_heroic", |g, a| {
            let it = a[0].item();
            let f = g.legal(it, 0)?;
            let e = g.legal(it, 1)?;
            if let Some(f) = f {
                g.mod_(f, 1, "turn".into(), None)?;
            }
            if let Some(e) = e {
                let c = g.it(it).ctrl;
                g.stun(Some(e), Some(c))?;
            }
            Ok(Value::None)
        }),
        choices: Value::func("_hc_choices", |g, a| {
            let pid = a[0].int() as usize;
            let hb = ctx_hb(&a[1]);
            let mut out = vec![];
            for e in enemies(g, pid, false, hb, true)? {
                for f in friends(g, pid, false, hb)? {
                    if g.o(f).loc == g.o(e).loc {
                        out.push((map! { tg = tg2(g.o(f).uid, g.o(e).uid) }, e));
                    }
                }
            }
            let out = sorted_by(g, out, |g, (_, e)| Ok(g.o(*e).stunned))?;
            Ok(maps(out.into_iter().map(|(m, _)| m).collect()))
        }),
        ..imp("Heroic Charge")
    });

    // Imperial Decree — "[Action] When any unit takes damage this turn, kill it."
    r.card(Impl {
        timing: Some("action"),
        resolve: Value::func("_decree", |g, a| {
            let pid = g.it(a[0].item()).ctrl;
            let on_damage = Value::func("_decree.on_damage", |g_, a_| {
                let eid = a_[0].int() as u64;
                let u = a_[1].map().g("obj").obj();
                if g_.spec(u).type_ != "Unit" {
                    return Ok(Value::None);
                }
                let res = Value::func("_decree.res", |g2, a2| {
                    let it2 = a2[0].item();
                    let d = g2.it(it2).data.clone();
                    if let Some(x) = g2.obj(d.g("uid").int() as u32) {
                        if g2.o(x).oid == d.g("oid").int() {
                            let c = g2.it(it2).ctrl;
                            g2.kill(&[x], Some(c), false)?;
                        }
                    }
                    Ok(Value::None)
                });
                let ep = g_.eff(eid).unwrap().g("pid").int() as usize;
                let (uid, oid) = (g_.o(u).uid, g_.o(u).oid);
                g_.queue_trigger(ep, "Imperial Decree", res, map! { uid = uid, oid = oid }, None, false, Value::None, Value::None);
                Ok(Value::None)
            });
            g.add_effect(map! { on = "damaged", fn = on_damage, dur = "turn", pid = pid });
            Ok(Value::None)
        }),
        ..imp("Imperial Decree")
    });

    // Karma, Channeler — "[Vision] When you recycle one or more cards, buff a friendly unit." (runes aren't cards:
    // recycling a rune emits no 'recycle' event)
    r.card(Impl {
        kw: map! { Vision = 1 },
        on_event: Value::func("_karma_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "recycle" && info.g("pid").int() as usize == g.o(o).ctrl && info.g("cards").truthy() {
                let res = Value::func("_karma_event.res", |g_, a_| {
                    let it = a_[0].item();
                    if g_.legal(it, 0)?.is_some() {
                        let u = g_.legal(it, 0)?;
                        g_.buff(u, None)?;
                    }
                    Ok(Value::None)
                });
                let opts = Value::func("_karma_event.options", |g_, a_| {
                    let c = g_.it(a_[0].item()).ctrl;
                    let fs = friends(g_, c, false, None)?;
                    Ok(sorted_by(g_, fs, |g, &u| Ok(g.o(u).buff > 0))?.into())
                });
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                let choose = trig_target(opts, p_friend_v(), "target", false, true);
                g.queue_trigger(c, "Karma, Channeler", res, Map::new(), Some(uid), false, choose, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Karma, Channeler")
    });

    // Kayle, Justified — "[Empower] [3]. I can be [Empowered] up to three times. I have +2 might for each time I'm
    // [Empowered]. While I'm [Empowered] three times, I have [Deflect 3] and [Ganking]." (rule 441.1.c.1)
    // The number of times is kept in g.effects; it counts only while the Empowered status is on (a Disempower removes
    // the status, rule 442.1).
    r.card(Impl {
        might_mod: Value::func("Kayle, Justified.might_mod", |g, a| Ok(Value::Int(2 * kayle_n(g, a[0].obj())))),
        kw_if: vec![(
            Value::func("Kayle, Justified.cond", |g, a| Ok((kayle_n(g, a[0].obj()) >= 3).into())),
            map! { Deflect = 3, Ganking = 1 },
        )],
        abilities: vec![ability(
            "Empower",
            AbilityArgs {
                cost: Cost::from("3 energy"),
                can: Value::func("Kayle, Justified.can", |g, a| Ok((kayle_n(g, a[1].obj()) < 3).into())),
                resolve: Value::func("_kayle_empower", |g, a| {
                    let it = a[0].item();
                    let Some(o) = me(g, it) else { return Ok(Value::None) };
                    let n = kayle_n(g, o);
                    if n >= 3 {
                        return Ok(Value::None);
                    }
                    let (uid, oid, c) = (g.o(o).uid, g.o(o).oid, g.it(it).ctrl);
                    g.effects.retain(|ef| !(ef.d.g("kind").opt_str() == Some("order_kayle") && ef.d.g("uid") == Value::from(uid)));
                    g.add_effect(map! { kind = "order_kayle", pid = c, uid = uid, oid = oid, n = n + 1 });
                    if n == 0 {
                        g.empower(Some(o), None)?;
                    } else {
                        let ro = g.repr(o);
                        g.log(|| format!("  {ro} is empowered again ({})", n + 1));
                        g.emit("empowered", map! { obj = o })?;
                    }
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..imp("Kayle, Justified")
    });

    // Keeper of Law — "I cost [2] and [Order] less if you control a battlefield with exactly two units there."
    r.card(Impl {
        cost_mod: Value::func("Keeper of Law.cost_mod", |g, a| {
            let pid = a[0].int() as usize;
            let ok = g.bfs.iter().any(|b| b.ctrl == Some(pid) && g.units(None, Some(Loc::Bf(b.idx))).len() == 2);
            Ok(if ok { de_dp(2, 1) } else { de_dp(0, 0) })
        }),
        ..imp("Keeper of Law")
    });

    // King's Edict — "Starting with the next player, each other player chooses a unit you don't control that hasn't
    // been chosen for this spell. Kill those units." (1v1: the opponent chooses; not targeted, rule 355.10.e)
    r.card(Impl {
        resolve: Value::func("_edict", |g, a| {
            let pid = g.it(a[0].item()).ctrl;
            let mut chosen: Vec<O> = vec![];
            for p in [1 - pid] {
                let us: Vec<O> =
                    g.all_units().into_iter().filter(|&u| g.o(u).ctrl != pid && !chosen.contains(&u)).collect();
                if !us.is_empty() {
                    let us = sorted_by(g, us, |g, &u| Ok((g.o(u).ctrl != p, value(g, u)?)))?;
                    let c = g.ask(p, "sacrifice", vals(&us), Map::new())?;
                    chosen.push(c.obj());
                }
            }
            g.kill(&chosen, Some(pid), false)?;
            Ok(Value::None)
        }),
        ..imp("King's Edict")
    });

    // Lacerate — "Choose a unit. If it's [Empowered], disempower it. Then kill it if it has 3 might or less.
    // [Flow] [4][Order][Order]"
    r.card(Impl {
        flow: flow_cost("4 energy and 2 order runes"),
        preds: Value::List(vec![p_unit_v()]),
        resolve: Value::func("_lacerate", |g, a| {
            let it = a[0].item();
            let Some(u) = g.legal(it, 0)? else { return Ok(Value::None) };
            g.disempower(Some(u));
            if g.might(u)? <= 3 {
                let c = g.it(it).ctrl;
                g.kill(&[u], Some(c), false)?;
            }
            Ok(Value::None)
        }),
        choices: Value::func("_lacerate_choices", |g, a| {
            let pid = a[0].int() as usize;
            let hb = ctx_hb(&a[1]);
            let es = enemies(g, pid, false, hb, true)?;
            let mut all = sorted_by(g, es, |g, &u| {
                let m = g.might(u)? - if g.o(u).empowered { 2 } else { 0 };
                Ok((m > 3, -value(g, u)?))
            })?;
            all.extend(friends(g, pid, false, hb)?);
            Ok(maps(tg_choices(g, &all)))
        }),
        ..imp("Lacerate")
    });

    // Leona, Determined — "[Shield] When I attack, stun an enemy unit here."
    r.card(Impl {
        kw: map! { Shield = 1 },
        on_event: Value::func("_leona_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "attack" && info.g("obj").opt_obj() == Some(o) {
                let h = g.o(o).loc;
                let ok = Value::func("_leona_event.ok", move |g_, a_| {
                    let (it, u) = (a_[0].item(), a_[1].obj());
                    Ok((p_enemy(g_, it, u) && g_.o(u).loc == h).into())
                });
                let res = Value::func("_leona_event.res", |g_, a_| {
                    let it = a_[0].item();
                    let u = g_.legal(it, 0)?;
                    let c = g_.it(it).ctrl;
                    g_.stun(u, Some(c))?;
                    Ok(Value::None)
                });
                let opts = Value::func("_leona_event.options", move |g_, a_| {
                    let it = a_[0].item();
                    let c = g_.it(it).ctrl;
                    let es: Vec<O> =
                        enemies(g_, c, false, None, true)?.into_iter().filter(|&u| p_enemy(g_, it, u) && g_.o(u).loc == h).collect();
                    Ok(sorted_by(g_, es, |g, &u| Ok(g.o(u).stunned))?.into())
                });
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                let choose = trig_target(opts, ok, "target", false, true);
                g.queue_trigger(c, "Leona, Determined", res, Map::new(), Some(uid), false, choose, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Leona, Determined")
    });

    // Lightning Rush — "Look at the top 3 cards of your Main Deck. You may choose a card from among them and draw it.
    // Put the rest into your trash. [Flow] [2][A]"
    r.card(Impl {
        flow: flow_cost("2 energy and 1 rune of any type"),
        resolve: Value::func("_rush", |g, a| {
            let pid = g.it(a[0].item()).ctrl;
            let d: Vec<O> = g.p[pid].deck.iter().take(3).copied().collect();
            let top = g.look(pid, &d, None)?;
            if top.is_empty() {
                return Ok(Value::None);
            }
            let s = sorted_by(g, top.clone(), |g, &c| Ok(-cost_of(g, c)))?;
            let mut opts = vals(&s);
            opts.push(Value::None);
            let pick = g.ask(pid, "rush_pick", opts, Map::new())?.opt_obj();
            if let Some(p) = pick {
                g.draw_card(pid, p)?;
            }
            for c in top {
                if Some(c) != pick {
                    g.to_zone(c, "trash", false, None)?;
                }
            }
            Ok(Value::None)
        }),
        ..imp("Lightning Rush")
    });

    // Loyal Poro — "[Deathknell] If I didn't die alone, draw 1."
    r.card(Impl {
        deathknell: Value::func("Loyal Poro.deathknell", |g, a| {
            let it = a[0].item();
            if !g.it(it).data.g("info").map().g("alone").truthy() {
                let c = g.it(it).ctrl;
                g.draw(c, 1)?;
            }
            Ok(Value::None)
        }),
        ..imp("Loyal Poro")
    });

    // Lux, Crownguard — "[E]: [Reaction] — [Add] [2]. Use only to play spells." ([Add] abilities are used while paying)
    r.card(Impl {
        add: vec![add_ability(
            "2 energy",
            true,
            false,
            Value::func("Lux, Crownguard.can", |_g, a| {
                let ctx = &a[2];
                Ok((!ctx.is_none() && ctx.map().g("kind").opt_str() == Some("spell")).into())
            }),
        )],
        ..imp("Lux, Crownguard")
    });

    // Machine Evangel — "[Deathknell] Play three 1 might Recruit unit tokens into your base."
    r.card(Impl {
        deathknell: Value::func("Machine Evangel.deathknell", |g, a| {
            let c = g.it(a[0].item()).ctrl;
            tokens(g, c, "Recruit", 3, Some(Loc::Base))?;
            Ok(Value::None)
        }),
        ..imp("Machine Evangel")
    });

    // Masa, Crashing Thunder — "You may pay [Order] as an additional cost to play me. When you play me, if you paid the
    // additional cost, [Stun] an enemy unit at a battlefield."
    r.card(Impl {
        on_play: Value::func("_masa_play", |g, a| {
            let (o, ctx) = (a[0].obj(), a[1].map());
            if !ctx.g("masa").truthy() {
                return Ok(Value::None);
            }
            let hb = ctx.g("hidden_bf").opt_loc();
            let res = Value::func("_masa_play.res", |g_, a_| {
                let it = a_[0].item();
                let u = g_.legal(it, 0)?;
                let c = g_.it(it).ctrl;
                g_.stun(u, Some(c))?;
                Ok(Value::None)
            });
            let opts = Value::func("_masa_play.options", move |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let es = enemies(g_, c, true, hb, true)?;
                Ok(sorted_by(g_, es, |g, &u| Ok(g.o(u).stunned))?.into())
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            let choose = trig_target(opts, p_enemy_bf_v(), "target", false, true);
            g.queue_trigger(c, "Masa, Crashing Thunder", res, Map::new(), Some(uid), false, choose, Value::None);
            Ok(Value::None)
        }),
        as_played: Value::func("Masa, Crashing Thunder.as_played", |_g, _a| {
            Ok(maps(vec![map! { masa = true }, Map::new()]))
        }),
        extra_cost_fn: Value::func("Masa, Crashing Thunder.extra_cost_fn", |_g, a| {
            Ok(if a[2].map().g("masa").truthy() { cost_val(0, &[order()]) } else { cost_val(0, &[]) })
        }),
        ..imp("Masa, Crashing Thunder")
    });

    // Noxian Drummer — "When I move to a battlefield, play a 1 might Recruit unit token here."
    r.card(Impl { on_event: move_tokens("Noxian Drummer", 1), ..imp("Noxian Drummer") });

    // Noxian Emissary — "[Empower] [1][Order]. [Empowered] [Deathknell] Play two 1 might Recruit unit tokens to your
    // base." (look-back: Empowered when it died, rule 808.1.d.3)
    let mut im = Impl {
        deathknell: Value::func("Noxian Emissary.deathknell", |g, a| {
            let it = a[0].item();
            if g.it(it).data.g("info").map().g("empowered").truthy() {
                let c = g.it(it).ctrl;
                tokens(g, c, "Recruit", 2, Some(Loc::Base))?;
            }
            Ok(Value::None)
        }),
        ..imp("Noxian Emissary")
    };
    impl_empower(&mut im, Cost::from("1 energy and 1 order rune"));
    r.card(im);

    // Peak Guardian — "When you play me, buff me. Then, if I am at a battlefield, buff all other friendly units there."
    let peak = Value::func("_peak", |g, a| {
        let it = a[0].item();
        let Some(m) = me(g, it) else { return Ok(Value::None) };
        g.buff(Some(m), None)?;
        let l = g.o(m).loc;
        if is_bf(l) {
            let c = g.it(it).ctrl;
            for u in g.units(Some(c), l) {
                if u != m {
                    g.buff(Some(u), None)?;
                }
            }
        }
        Ok(Value::None)
    });
    r.card(Impl {
        on_play: Value::func("Peak Guardian.on_play", move |g, a| {
            let o = a[0].obj();
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, "Peak Guardian", peak.clone(), Map::new(), Some(uid), false, Value::None, Value::None);
            Ok(Value::None)
        }),
        ..imp("Peak Guardian")
    });

    // Poppy, Defender of the Meek — "You may spend 3 XP as an additional cost to play me. If you do, I cost [3] less.
    // [Ambush] [Tank]"
    r.card(Impl {
        ambush: true,
        kw: map! { Tank = 1 },
        as_played: Value::func("Poppy, Defender of the Meek.as_played", |g, a| {
            let pid = a[0].int() as usize;
            let mut out = if g.p[pid].xp >= 3 { vec![map! { xp = 3 }] } else { vec![] };
            out.push(Map::new());
            Ok(maps(out))
        }),
        cost_mod: Value::func("Poppy, Defender of the Meek.cost_mod", |_g, a| {
            Ok(if a[2].map().g("xp").truthy() { de_dp(3, 0) } else { de_dp(0, 0) })
        }),
        ..imp("Poppy, Defender of the Meek")
    });

    // Recruit the Vanguard — "[Action] Play four 1 might Recruit unit tokens." (each to your base or a battlefield you
    // control; from Hidden — it has no Hidden — n/a)
    r.card(Impl {
        timing: Some("action"),
        resolve: Value::func("Recruit the Vanguard.resolve", |g, a| {
            let c = g.it(a[0].item()).ctrl;
            tokens(g, c, "Recruit", 4, None)?;
            Ok(Value::None)
        }),
        ..imp("Recruit the Vanguard")
    });

    // Rek'Sai, Swarm Queen — "When I attack, you may reveal the top 2 cards of your Main Deck. You may play one. Then
    // recycle the rest. If the played card is a unit, you may play it here."
    // Playing it is a limited play (rule 419.3): costs are paid, timing is ignored.
    let reksai_res = Value::func("_reksai_res", |g, a| {
        let it = a[0].item();
        let pid = g.it(it).ctrl;
        let h = g.it(it).data.g("here").opt_loc();
        let top = g.reveal(pid, 2, None, true, None)?;
        if top.is_empty() {
            return Ok(Value::None);
        }
        let mut opts = vec![];
        for &c in &top {
            opts.extend(reksai_options(g, pid, c, h)?);
        }
        let pick = if !opts.is_empty() {
            opts.push(Value::None);
            g.ask(pid, "reksai_play", opts, Map::new())?
        } else {
            Value::None
        };
        let mut played = None;
        if !pick.is_none() {
            let (c, ch) = (pick.list()[0].obj(), pick.list()[1].map().clone());
            let (e, reqs) = total_cost(g, pid, c, &ch, "deck")?;
            let ctx = pay_ctx(g, c);
            if g.pay(pid, e, &reqs, &ctx)? {
                played = Some(c);
                play_card(g, pid, c, "deck", ch.clone(), true)?;
            }
        }
        let rest: Vec<O> =
            top.iter().copied().filter(|&c| Some(c) != played && g.p[pid].deck.contains(&c)).collect();
        for &c in &rest {
            if let Some(i) = g.p[pid].deck.iter().position(|&x| x == c) {
                g.p[pid].deck.remove(i);
            }
        }
        if !rest.is_empty() {
            g.recycle_cards(pid, &rest)?;
        }
        Ok(Value::None)
    });
    r.card(Impl {
        on_event: Value::func("_reksai_event", move |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "attack" && info.g("obj").opt_obj() == Some(o) {
                let (c, uid, l) = (g.o(o).ctrl, g.o(o).uid, g.o(o).loc);
                g.queue_trigger(c, "Rek'Sai, Swarm Queen", reksai_res.clone(), map! { here = l }, Some(uid), true, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Rek'Sai, Swarm Queen")
    });

    // Royal Guard — "When you play me, play a 2 might Sand Soldier unit token here."
    r.card(Impl { on_play: play_tokens_here("Royal Guard", 1, "Sand Soldier"), ..imp("Royal Guard") });

    // Sacred Shears — "[Equip] [Order]" ; Might Bonus +1 and Effect Text (card image): "[Deathknell] — Draw 1."
    // The equipped unit has the Deathknell (Impl.fx_deathknell, read by Game.kill from the look-back attachments, rule
    // 808.1.d.3): it triggers even if the gear dies at the same time; Karthus, Eternal doubles it.
    let mut im = Impl {
        bonus: Some(1),
        fx_deathknell: Value::func("Sacred Shears.fx_deathknell", |g, a| {
            let c = g.it(a[0].item()).ctrl;
            g.draw(c, 1)?;
            Ok(Value::None)
        }),
        ..imp("Sacred Shears")
    };
    impl_equip(&mut im, Cost::from("1 order rune"));
    r.card(im);

    // Sandshifter — "When you play me, kill an enemy unit with 3 might or less."
    fn sand_ok(g: &mut Game, it: ItemId, u: O) -> Res<bool> {
        Ok(p_enemy(g, it, u) && g.might(u)? <= 3)
    }
    r.card(Impl {
        on_play: Value::func("_sandshifter", |g, a| {
            let o = a[0].obj();
            let ok = Value::func("_sandshifter.ok", |g_, a_| Ok(sand_ok(g_, a_[0].item(), a_[1].obj())?.into()));
            let opts = Value::func("_sandshifter.options", |g_, a_| {
                let it = a_[0].item();
                let c = g_.it(it).ctrl;
                let mut out = vec![];
                for u in enemies(g_, c, false, None, true)? {
                    if sand_ok(g_, it, u)? {
                        out.push(u);
                    }
                }
                Ok(out.into())
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            let choose = trig_target(opts, ok, "target", false, true);
            g.queue_trigger(c, "Sandshifter", kill_tg(), Map::new(), Some(uid), false, choose, Value::None);
            Ok(Value::None)
        }),
        ..imp("Sandshifter")
    });

    // Scrutinizing Sergeant — "When you play me, gain 1 XP for each friendly unit."
    r.card(Impl {
        on_play: Value::func("Scrutinizing Sergeant.on_play", |g, a| {
            let o = a[0].obj();
            let res = Value::func("Scrutinizing Sergeant.res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let n = g_.units(Some(c), None).len() as i64;
                g_.gain_xp(c, n)?;
                Ok(Value::None)
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, "Scrutinizing Sergeant", res, Map::new(), Some(uid), false, Value::None, Value::None);
            Ok(Value::None)
        }),
        ..imp("Scrutinizing Sergeant")
    });

    // Seal of Unity — "[E]: [Reaction] — [Add] [Order]."
    r.card(Impl { add: vec![add_ability("1 order rune", true, false, Value::None)], ..imp("Seal of Unity") });

    // Sett, Kingpin — "[Tank] I get +1 might for each buffed friendly unit at my battlefield."
    r.card(Impl {
        kw: map! { Tank = 1 },
        might_mod: Value::func("Sett, Kingpin.might_mod", |g, a| {
            let o = a[0].obj();
            let l = g.o(o).loc;
            Ok(Value::Int(if is_bf(l) {
                g.units(Some(g.o(o).ctrl), l).into_iter().filter(|&u| g.o(u).buff > 0).count() as i64
            } else {
                0
            }))
        }),
        ..imp("Sett, Kingpin")
    });

    // Shadow's Call — "Choose a friendly unit without [Temporary]. Give it [Temporary]. Draw 2."
    r.card(Impl {
        preds: Value::List(vec![Value::func("_P_no_temp", |g, a| {
            let (it, o) = (a[0].item(), a[1].obj());
            Ok((p_friend(g, it, o) && !g.has_kw(o, "Temporary")?).into())
        })]),
        resolve: Value::func("_shadows_call", |g, a| {
            let it = a[0].item();
            if let Some(u) = g.legal(it, 0)? {
                g.grant(u, "Temporary", 1, Value::None);
            }
            let c = g.it(it).ctrl;
            g.draw(c, 2)?;
            Ok(Value::None)
        }),
        choices: Value::func("Shadow's Call.choices", |g, a| {
            let pid = a[0].int() as usize;
            let mut us = vec![];
            for u in friends(g, pid, false, ctx_hb(&a[1]))? {
                if !g.has_kw(u, "Temporary")? {
                    us.push(u);
                }
            }
            let us = sorted_by(g, us, |g, &u| value(g, u))?;
            Ok(maps(tg_choices(g, &us)))
        }),
        ..imp("Shadow's Call")
    });

    // Shard of Undoing — "The first time a friendly unit dies during your Beginning Phase each turn, each opponent must
    // kill one of their units." (Beginning Phase: stages beginning/scoring, and 'channel' before the Channel Phase runs,
    // when the hold triggers resolve, rule 315.2)
    r.card(Impl {
        on_event: Value::func("_shard_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            let inf = if ev == "die" { info.g("info") } else { Value::None };
            if inf.is_none() || inf.map().g("ctrl").int() as usize != g.o(o).ctrl || lb_type(inf.map()) != "Unit" {
                return Ok(Value::None);
            }
            if g.tp != g.o(o).ctrl || !matches!(g.stage, "beginning" | "scoring" | "channel") {
                return Ok(Value::None);
            }
            // Python : clé tuple ("order_shard", uid, turn_no) de g.stats, écrite comme son repr
            let key = format!("('order_shard', {}, {})", g.o(o).uid, g.turn_no);
            if g.stats.0.iter().any(|(k, _)| *k == key) {
                return Ok(Value::None);
            }
            g.stats.add(&key, 1);
            let res = Value::func("_shard_event.res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                for p in [1 - c] {
                    let us = g_.units(Some(p), None);
                    if !us.is_empty() {
                        let us = sorted_by(g_, us, |g, &u| value(g, u))?;
                        let v = g_.ask(p, "sacrifice", vals(&us), Map::new())?.obj();
                        g_.kill(&[v], Some(p), false)?;
                    }
                }
                Ok(Value::None)
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, "Shard of Undoing", res, Map::new(), Some(uid), false, Value::None, Value::None);
            Ok(Value::None)
        }),
        ..imp("Shard of Undoing")
    });

    // Shen, Leader of the Kinkou Order — "[Shield] When I hold, if there is exactly one other unit you control here, you
    // score 1 point." (intervening condition checked when it triggers and on resolution, rule 383.2.a.1)
    r.card(Impl {
        kw: map! { Shield = 1 },
        on_event: Value::func("_shen_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "hold" && has_obj(&info.g("units"), o) && shen_ok(g, Some(o)) {
                let res = Value::func("_shen_event.res", |g_, a_| {
                    let it = a_[0].item();
                    if shen_ok(g_, me(g_, it)) {
                        let c = g_.it(it).ctrl;
                        g_.gain_point(c, "Shen, Leader of the Kinkou Order")?;
                    }
                    Ok(Value::None)
                });
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                g.queue_trigger(c, "Shen, Leader of the Kinkou Order", res, Map::new(), Some(uid), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Shen, Leader of the Kinkou Order")
    });

    // Shepherd's Heirloom — "When you play this, gain 1 XP. [Equip] — Spend 1 XP" ; Might Bonus +2 (card image).
    let mut ab = Map::new();
    ab.set("name", "Equip");
    ab.set("timing", "main");
    ab.set(
        "choices",
        Value::func("Shepherd's Heirloom.choices", |g, a| {
            let (pid, o) = (a[0].int() as usize, a[1].obj());
            let at = g.o(o).attached_to;
            let fs = friends(g, pid, false, None)?;
            Ok(maps(fs.into_iter().filter(|&u| Some(g.o(u).uid) != at).map(|u| map! { tg = tg1(g.o(u).uid) }).collect()))
        }),
    );
    ab.set("cost", Value::func("Shepherd's Heirloom.cost", |_g, _a| Ok(cost_val(0, &[]))));
    ab.set("preds", Value::List(vec![p_friend_v()]));
    ab.set("can", Value::func("Shepherd's Heirloom.can", |g, a| Ok((g.p[a[0].int() as usize].xp >= 1).into())));
    ab.set("can_extra", Value::func("Shepherd's Heirloom.can_extra", |g, a| Ok((g.p[a[0].int() as usize].xp >= 1).into())));
    ab.set(
        "extra_cost",
        Value::func("Shepherd's Heirloom.extra_cost", |g, a| Ok(g.spend_xp(a[0].int() as usize, 1)?.into())),
    );
    ab.set(
        "resolve",
        Value::func("Shepherd's Heirloom.resolve", |g, a| {
            let it = a[0].item();
            if g.legal(it, 0)?.is_some() {
                if let Some(src) = g.it(it).src.and_then(|s| g.obj(s)) {
                    let u = g.legal(it, 0)?;
                    if let Some(u) = u {
                        attach(g, src, u)?;
                    }
                }
            }
            Ok(Value::None)
        }),
    );
    r.card(Impl {
        bonus: Some(2),
        on_play: Value::func("Shepherd's Heirloom.on_play", |g, a| {
            let o = a[0].obj();
            let res = Value::func("Shepherd's Heirloom.res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                g_.gain_xp(c, 1)?;
                Ok(Value::None)
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, "Shepherd's Heirloom", res, Map::new(), Some(uid), false, Value::None, Value::None);
            Ok(Value::None)
        }),
        abilities: vec![ab],
        ..imp("Shepherd's Heirloom")
    });

    // Solari Chief — "When you play me, choose an enemy unit. If it is stunned, kill it. Otherwise, stun it."
    r.card(Impl {
        on_play: Value::func("Solari Chief.on_play", |g, a| {
            let o = a[0].obj();
            let res = Value::func("_chief_res", |g_, a_| {
                let it = a_[0].item();
                let Some(u) = g_.legal(it, 0)? else { return Ok(Value::None) };
                let c = g_.it(it).ctrl;
                if g_.o(u).stunned {
                    g_.kill(&[u], Some(c), false)?;
                } else {
                    g_.stun(Some(u), Some(c))?;
                }
                Ok(Value::None)
            });
            let opts = Value::func("Solari Chief.options", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let es = enemies(g_, c, false, None, true)?;
                Ok(sorted_by(g_, es, |g, &u| Ok(!g.o(u).stunned))?.into())
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            let choose = trig_target(opts, p_enemy_v(), "target", false, true);
            g.queue_trigger(c, "Solari Chief", res, Map::new(), Some(uid), false, choose, Value::None);
            Ok(Value::None)
        }),
        ..imp("Solari Chief")
    });

    // Solari Sunhawk — "[Empower] [2]. [Empowered] I have +1 might and [Deflect 2]."
    let mut im = Impl {
        might_if: vec![(when_empowered(), 1)],
        kw_if: vec![(when_empowered(), map! { Deflect = 2 })],
        ..imp("Solari Sunhawk")
    };
    impl_empower(&mut im, Cost::from("2 energy"));
    r.card(im);

    // Soul Harvest — "Kill a unit at a battlefield with 3 might or less."
    r.card(Impl {
        preds: Value::List(vec![p_small(3)]),
        resolve: kill_tg(),
        choices: Value::func("Soul Harvest.choices", |g, a| {
            let pid = a[0].int() as usize;
            let mut out = vec![];
            for u in all_units(g, pid, true, ctx_hb(&a[1]))? {
                if g.might(u)? <= 3 {
                    out.push(u);
                }
            }
            Ok(maps(tg_choices(g, &out)))
        }),
        ..imp("Soul Harvest")
    });

    // Spectral Matron — "When you play me, you may play a unit costing no more than [3] and no more than [A] from your
    // trash, ignoring its cost." (the card in the trash is chosen as the trigger is finalized, rule 355.10.a)
    r.card(Impl {
        on_play: Value::func("Spectral Matron.on_play", |g, a| {
            let o = a[0].obj();
            let res = Value::func("_matron_res", |g_, a_| {
                let it = a_[0].item();
                let c = g_.it(it).ctrl;
                let (u, oi) = pick_of(&g_.it(it).data.g("pick"));
                if let Some(x) = trash_card(g_, c, u, oi) {
                    play_unit_free(g_, c, x, "trash", None, false, None)?;
                }
                Ok(Value::None)
            });
            let choose = Value::func("_matron_choose", |g_, a_| {
                let it = a_[0].item();
                let c = g_.it(it).ctrl;
                let cands = small_trash_units(g_, c, 3, 1)?;
                if cands.is_empty() {
                    return Ok(false.into());
                }
                let x = g_.ask(c, "mixologist_pick", vals(&cands), Map::new())?.obj();
                let (u, oi) = (g_.o(x).uid, g_.o(x).oid);
                g_.it_mut(it).data.set("pick", Value::Tuple(vec![u.into(), oi.into()]));
                Ok(true.into())
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, "Spectral Matron", res, Map::new(), Some(uid), true, choose, Value::None);
            Ok(Value::None)
        }),
        ..imp("Spectral Matron")
    });

    // Stalking Wolf — "[Ambush] As an additional cost to play me, kill a Bird, Cat, Dog, or Poro you control. You may
    // play me to its battlefield (even if you don't have other units there)."
    // The killed unit is still at its battlefield when the locations are listed, so Ambush already offers it.
    r.card(Impl {
        ambush: true,
        as_played: Value::func("Stalking Wolf.as_played", |g, a| {
            let pid = a[0].int() as usize;
            let us = g.units(Some(pid), None);
            let us = sorted_by(g, us, |g, &u| value(g, u))?;
            let mut out = vec![];
            for u in us {
                if is_pet(&g.tags(u)?) {
                    out.push(map! { kill = g.o(u).uid });
                }
            }
            Ok(maps(out))
        }),
        ..imp("Stalking Wolf")
    });

    // Starhound — "When you play me, return a Bird, Cat, Dog, or Poro from your trash to your hand."
    r.card(Impl {
        on_play: Value::func("Starhound.on_play", |g, a| {
            let o = a[0].obj();
            let res = Value::func("_starhound_res", |g_, a_| {
                let it = a_[0].item();
                let c = g_.it(it).ctrl;
                let (u, oi) = pick_of(&g_.it(it).data.g("pick"));
                if let Some(x) = trash_card(g_, c, u, oi) {
                    g_.to_zone(x, "hand", false, None)?;
                }
                Ok(Value::None)
            });
            let choose = Value::func("_starhound_choose", |g_, a_| {
                let it = a_[0].item();
                let c = g_.it(it).ctrl;
                let cs: Vec<O> = g_.p[c].trash.iter().copied().filter(|&x| is_pet(&g_.spec(x).tags)).collect();
                let cands = sorted_by(g_, cs, |g, &x| Ok((-cost_of(g, x), g.cname(x).to_string())))?;
                if cands.is_empty() {
                    return Ok(false.into());
                }
                let x = g_.ask(c, "starhound_pick", vals(&cands), Map::new())?.obj();
                let (u, oi) = (g_.o(x).uid, g_.o(x).oid);
                g_.it_mut(it).data.set("pick", Value::Tuple(vec![u.into(), oi.into()]));
                Ok(true.into())
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, "Starhound", res, Map::new(), Some(uid), false, choose, Value::None);
            Ok(Value::None)
        }),
        ..imp("Starhound")
    });

    // The Ruination — "Kill all units."
    r.card(Impl {
        resolve: Value::func("The Ruination.resolve", |g, a| {
            let c = g.it(a[0].item()).ctrl;
            let us = g.all_units();
            g.kill(&us, Some(c), false)?;
            Ok(Value::None)
        }),
        ..imp("The Ruination")
    });

    // Trifarian Gloryseeker — "[Legion] — When you play me, buff me."
    r.card(Impl {
        on_play: Value::func("_gloryseeker", |g, a| {
            let o = a[0].obj();
            if g.legion(g.o(o).ctrl, Some(o)) {
                let res = Value::func("_gloryseeker.res", |g_, a_| {
                    let m = me(g_, a_[0].item());
                    g_.buff(m, None)?;
                    Ok(Value::None)
                });
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                g.queue_trigger(c, "Trifarian Gloryseeker", res, Map::new(), Some(uid), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Trifarian Gloryseeker")
    });

    // Trove Golem — "When you play me, play four Gold gear tokens exhausted."
    r.card(Impl {
        on_play: Value::func("Trove Golem.on_play", |g, a| {
            let o = a[0].obj();
            let res = Value::func("Trove Golem.res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                tokens(g_, c, "Gold", 4, None)?;
                Ok(Value::None)
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, "Trove Golem", res, Map::new(), Some(uid), false, Value::None, Value::None);
            Ok(Value::None)
        }),
        ..imp("Trove Golem")
    });

    // Trusty Ramhound — "While you have another unit here, I have +1 might."
    r.card(Impl {
        might_mod: Value::func("Trusty Ramhound.might_mod", |g, a| {
            Ok(Value::Int(if g.alone(a[0].obj(), None) { 0 } else { 1 }))
        }),
        ..imp("Trusty Ramhound")
    });

    // Ultrasoft Poro — "[Deflect] [E]: Play two 1 might Bird unit tokens with [Deflect]. Use this ability only while I'm
    // at a battlefield."
    r.card(Impl {
        kw: map! { Deflect = 1 },
        abilities: vec![ability(
            "Birds",
            AbilityArgs {
                exhaust: true,
                can: Value::func("Ultrasoft Poro.can", |g, a| Ok(is_bf(g.o(a[1].obj()).loc).into())),
                resolve: Value::func("Ultrasoft Poro.resolve", |g, a| {
                    let c = g.it(a[0].item()).ctrl;
                    tokens(g, c, "Bird", 2, None)?;
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..imp("Ultrasoft Poro")
    });

    // Undying Loyalty — "This costs [2] less if you choose a Bird, Cat, Dog, or Poro. Play a unit with cost no more than
    // [2] and no more than [A] from your trash, ignoring its cost." (the unit card in the trash is a target, 355.10.a)
    r.card(Impl {
        choices: Value::func("_loyalty_choices", |g, a| {
            let pid = a[0].int() as usize;
            let cs = small_trash_units(g, pid, 2, 1)?;
            let cs = sorted_by(g, cs, |g, &c| Ok((!is_pet(&g.spec(c).tags), -(g.spec(c).might.unwrap_or(0) as i64))))?;
            let mut out = vec![];
            let mut seen: BTreeSet<String> = BTreeSet::new();
            for c in cs {
                let n = g.cname(c).to_string();
                if !seen.contains(&n) {
                    seen.insert(n);
                    out.push(map! { pick = Value::Tuple(vec![g.o(c).uid.into(), g.o(c).oid.into()]) });
                }
            }
            Ok(maps(out))
        }),
        resolve: Value::func("_loyalty_res", |g, a| {
            let it = a[0].item();
            let c = g.it(it).ctrl;
            let (u, oi) = pick_of(&g.it(it).data.g("pick"));
            if let Some(x) = trash_card(g, c, u, oi) {
                play_unit_free(g, c, x, "trash", None, false, None)?;
            }
            Ok(Value::None)
        }),
        cost_mod: Value::func("_loyalty_cost", |g, a| {
            let (pid, ch) = (a[0].int() as usize, a[2].map());
            let c = if ch.g("pick").truthy() {
                let (u, oi) = pick_of(&ch.g("pick"));
                trash_card(g, pid, u, oi)
            } else {
                None
            };
            Ok(if c.is_some_and(|c| is_pet(&g.spec(c).tags)) { de_dp(2, 0) } else { de_dp(0, 0) })
        }),
        ..imp("Undying Loyalty")
    });

    // Unsung Hero — "[Deathknell] — If I was [Mighty], draw 2." (look-back Might, rule 808.1.d.3)
    r.card(Impl {
        deathknell: Value::func("Unsung Hero.deathknell", |g, a| {
            let it = a[0].item();
            if g.it(it).data.g("info").map().g("might").int() >= 5 {
                let c = g.it(it).ctrl;
                g.draw(c, 2)?;
            }
            Ok(Value::None)
        }),
        ..imp("Unsung Hero")
    });

    // Vanguard Armory — "[E]: Play three 1 might Recruit unit tokens. (You may play them to different locations.)"
    r.card(Impl {
        abilities: vec![ability(
            "Recruits",
            AbilityArgs {
                exhaust: true,
                resolve: Value::func("Vanguard Armory.resolve", |g, a| {
                    let c = g.it(a[0].item()).ctrl;
                    tokens(g, c, "Recruit", 3, None)?;
                    Ok(Value::None)
                }),
                ..Default::default()
            },
        )],
        ..imp("Vanguard Armory")
    });

    // Vanguard Attendant — "I enter ready."
    r.card(Impl { enter_ready: Value::Bool(true), ..imp("Vanguard Attendant") });

    // Vanguard Captain — "[Legion] — When you play me, play two 1 might Recruit unit tokens here."
    r.card(Impl {
        on_play: Value::func("_captain", |g, a| {
            let o = a[0].obj();
            if g.legion(g.o(o).ctrl, Some(o)) {
                play_tokens_here_do(g, o, "Vanguard Captain", 2, "Recruit");
            }
            Ok(Value::None)
        }),
        ..imp("Vanguard Captain")
    });

    // Vengeance — "Kill a unit."
    r.card(Impl {
        preds: Value::List(vec![p_unit_v()]),
        resolve: kill_tg(),
        choices: Value::func("Vengeance.choices", |g, a| {
            let pid = a[0].int() as usize;
            let es = enemies(g, pid, false, ctx_hb(&a[1]), true)?;
            Ok(maps(tg_choices(g, &es)))
        }),
        ..imp("Vengeance")
    });

    // Viktor, Leader — "When another non-Recruit unit you control dies, play a 1 might Recruit unit token into your base."
    // (Viktor dying at the same time doesn't see it, rule 383.2.c.2)
    r.card(Impl {
        on_event: Value::func("_viktor_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev != "die" {
                return Ok(Value::None);
            }
            let inf = info.g("info");
            let inf = inf.map();
            let sp = spec(inf.g("spec").str());
            if inf.g("obj").opt_obj() != Some(o)
                && inf.g("ctrl").int() as usize == g.o(o).ctrl
                && sp.type_ == "Unit"
                && !sp.tags.contains("Recruit")
            {
                let res = Value::func("_viktor_event.res", |g_, a_| {
                    let c = g_.it(a_[0].item()).ctrl;
                    tokens(g_, c, "Recruit", 1, Some(Loc::Base))?;
                    Ok(Value::None)
                });
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                g.queue_trigger(c, "Viktor, Leader", res, Map::new(), Some(uid), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Viktor, Leader")
    });

    // Xin Zhao, Vigilant — "[Tank] I enter ready if you have two or more other units in your base."
    r.card(Impl {
        kw: map! { Tank = 1 },
        enter_ready: Value::func("Xin Zhao, Vigilant.enter_ready", |g, a| {
            let (pid, c) = (a[0].int() as usize, a[1].obj());
            Ok((g.units(Some(pid), Some(Loc::Base)).into_iter().filter(|&u| u != c).count() >= 2).into())
        }),
        ..imp("Xin Zhao, Vigilant")
    });

    // Zaun Punk — "You may kill a friendly gear as an additional cost to play me. When you play me, if you paid the
    // additional cost, kill a gear."
    r.card(Impl {
        on_play: Value::func("_punk_play", |g, a| {
            let (o, ctx) = (a[0].obj(), a[1].map());
            if ctx.g("kill").is_none() {
                return Ok(Value::None);
            }
            let opts = Value::func("_punk_play.options", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let gs = g_.gear(None);
                Ok(sorted_by(g_, gs, |g, &x| Ok((g.o(x).ctrl == c, -(g.spec(x).e as i64))))?.into())
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            let choose = trig_target(opts, p_gear_v(), "target", false, true);
            g.queue_trigger(c, "Zaun Punk", kill_tg(), Map::new(), Some(uid), false, choose, Value::None);
            Ok(Value::None)
        }),
        as_played: Value::func("Zaun Punk.as_played", |g, a| {
            let pid = a[0].int() as usize;
            let gs = g.gear(Some(pid));
            let gs = sorted_by(g, gs, |g, &x| Ok((g.spec(x).e, g.o(x).uid)))?;
            let mut out = vec![Map::new()];
            out.extend(gs.into_iter().map(|x| map! { kill = g.o(x).uid }));
            Ok(maps(out))
        }),
        ..imp("Zaun Punk")
    });

    // ====================================================================== cards using the engine hooks (integration)
    r.ask_text(&[
        ("fallen_feline_name", "Fallen Feline : nomme un sort"),
        ("ivern_tag", "Ivern, Friend to All : quel tag gagne-t-il ?"),
        ("spectacles_copy", "Shady Spectacles : de quelle unité alliée l'unité équipée devient-elle une copie ?"),
        ("vanguard_helm", "Vanguard Helm : quelle autre unité alliée améliorer ?"),
    ]);

    // Galio, Indefatigable — "[Deflect] [Tank] I don't deal combat damage."
    r.card(Impl {
        kw: map! { Deflect = 1, Tank = 1 },
        hooks: map! { no_combat_damage = Value::func("Galio, Indefatigable.no_combat_damage", |_g, _a| Ok(true.into())) },
        ..imp("Galio, Indefatigable")
    });

    // Sacred Protector — "I don't deal combat damage unless I'm at a battlefield with exactly one other unit you control."
    r.card(Impl {
        hooks: map! { no_combat_damage = Value::func("Sacred Protector.no_combat_damage", |g, a| {
            let o = a[0].obj();
            let l = g.o(o).loc;
            Ok((!(is_bf(l) && g.units(Some(g.o(o).ctrl), l).into_iter().filter(|&u| u != o).count() == 1)).into())
        }) },
        ..imp("Sacred Protector")
    });

    // Tactical Retreat — "[Reaction] Choose a friendly unit. The next time it would die this turn, heal it, exhaust it,
    // and recall it instead." (a 'death_rep' effect for that object, used once: rules 370-373)
    let retreat_rep = Value::func("_retreat_rep", |g, a| {
        let (eid, u) = (a[0].int() as u64, a[1].obj());
        let eff = g.eff(eid).cloned().unwrap_or_default();
        if Value::from(g.o(u).uid) != eff.g("uid") || Value::from(g.o(u).oid) != eff.g("oid") {
            return Ok(Value::None);
        }
        let apply = Value::func("_retreat_rep.apply", move |g_, a_| {
            let x = a_[0].obj();
            if g_.has_effect(eid) {
                g_.remove_effect(eid);
            }
            g_.save_unit(x, true, true);
            Ok(Value::None)
        });
        Ok(map! { name = "Tactical Retreat", apply = apply }.into())
    });
    r.card(Impl {
        timing: Some("reaction"),
        preds: Value::List(vec![p_friend_v()]),
        resolve: Value::func("_retreat", move |g, a| {
            let it = a[0].item();
            if let Some(u) = g.legal(it, 0)? {
                let (uid, oid) = (g.o(u).uid, g.o(u).oid);
                g.add_effect(map! { kind = "death_rep", fn = retreat_rep.clone(), uid = uid, oid = oid, dur = "turn" });
            }
            Ok(Value::None)
        }),
        choices: Value::func("Tactical Retreat.choices", |g, a| {
            let fs = friends(g, a[0].int() as usize, false, None)?;
            let fs = sorted_by(g, fs, |g, &u| Ok(-value(g, u)?))?;
            Ok(maps(tg_choices(g, &fs)))
        }),
        ..imp("Tactical Retreat")
    });

    // Soraka, Wanderer — "I must be assigned combat damage last. If another unit you control here would die, if it has
    // less Might than me, instead heal it, exhaust it, and recall it." One replacement applied in one sequence to every
    // qualifying simultaneous death (rule 373.2), "here" and Mights read as it applies (rule 370.4).
    r.card(Impl {
        assign_last: true,
        hooks: map! { death_rep = Value::func("_soraka_rep", |g, a| {
            let (src, u) = (a[0].obj(), a[1].obj());
            if u == src
                || g.o(u).ctrl != g.o(src).ctrl
                || g.spec(u).type_ != "Unit"
                || !g.on_board(src)
                || g.o(u).loc != g.o(src).loc
                || g.might(u)? >= g.might(src)?
            {
                return Ok(Value::None);
            }
            let apply = Value::func("_soraka_rep.apply", |g_, a_| {
                g_.save_unit(a_[0].obj(), true, true);
                Ok(Value::None)
            });
            Ok(map! { name = "Soraka, Wanderer", apply = apply, multi = true }.into())
        }) },
        ..imp("Soraka, Wanderer")
    });

    // Symbol of the Solari — "If a combat where you are the attacker ends in a tie, recall ALL units instead."
    r.card(Impl { tie_recall_all: true, ..imp("Symbol of the Solari") });

    // Renata Glasc, Industrialist — "Your tokens enter ready." (unit and gear tokens, rule 369.3: it replaces the way
    // they enter, also for "play a Gold gear token exhausted")
    r.card(Impl {
        hooks: map! { units_enter_ready = Value::func("Renata Glasc, Industrialist.units_enter_ready", |g, a| {
            let (src, pid, c) = (a[0].obj(), a[1].int() as usize, a[2].obj());
            Ok((g.o(src).ctrl == pid && g.o(c).token).into())
        }) },
        ..imp("Renata Glasc, Industrialist")
    });

    // Rally the Troops — "[Action] When a friendly unit is played this turn, buff it. Draw 1." (tokens are played too)
    r.card(Impl {
        timing: Some("action"),
        resolve: Value::func("_rally", |g, a| {
            let it = a[0].item();
            let pid = g.it(it).ctrl;
            let played = Value::func("_rally_played", |g_, a_| {
                let (eid, info) = (a_[0].int() as u64, a_[1].map());
                let ep = g_.eff(eid).unwrap().g("pid").int() as usize;
                let c = info.g("card").obj();
                if info.g("pid").int() as usize == ep
                    && g_.spec(c).type_ == "Unit"
                    && g_.o(c).zone == Some(Zone::Board)
                    && g_.o(c).ctrl == ep
                {
                    let res = Value::func("_rally_played.res", |g2, a2| {
                        let d = g2.it(a2[0].item()).data.clone();
                        if let Some(x) = g2.obj(d.g("uid").int() as u32) {
                            if g2.o(x).oid == d.g("oid").int() {
                                g2.buff(Some(x), None)?;
                            }
                        }
                        Ok(Value::None)
                    });
                    let (uid, oid) = (g_.o(c).uid, g_.o(c).oid);
                    g_.queue_trigger(ep, "Rally the Troops", res, map! { uid = uid, oid = oid }, None, false, Value::None, Value::None);
                }
                Ok(Value::None)
            });
            g.add_effect(map! { on = "played", fn = played, pid = pid, dur = "turn" });
            g.draw(pid, 1)?;
            Ok(Value::None)
        }),
        ..imp("Rally the Troops")
    });

    // Reluctant Leader — "When you play another unit, give me +2 might this turn." (tokens included, rule 185.2.a)
    r.card(Impl {
        on_event: Value::func("_leader_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "played" && info.g("pid").int() as usize == g.o(o).ctrl {
                let c = info.g("card").obj();
                if c != o && g.spec(c).type_ == "Unit" {
                    let res = Value::func("_leader_event.res", |g_, a_| {
                        let it = a_[0].item();
                        if let Some(m) = g_.it(it).src.and_then(|s| g_.obj(s)) {
                            if g_.o(m).oid == g_.it(it).data.g("oid").int() {
                                g_.mod_(m, 2, "turn".into(), None)?;
                            }
                        }
                        Ok(Value::None)
                    });
                    let (ct, uid, oid) = (g.o(o).ctrl, g.o(o).uid, g.o(o).oid);
                    g.queue_trigger(ct, "Reluctant Leader", res, map! { oid = oid }, Some(uid), false, Value::None, Value::None);
                }
            }
            Ok(Value::None)
        }),
        ..imp("Reluctant Leader")
    });

    // Fallen Feline — "When you play me, name a spell. While I'm at a battlefield, opponents can't play spells with that
    // name." The name is kept for this object (uid/oid) in g.effects.
    r.card(Impl {
        on_play: Value::func("_feline_play", |g, a| {
            let o = a[0].obj();
            let res = Value::func("_feline_play.res", |g_, a_| {
                let it = a_[0].item();
                let c = g_.it(it).ctrl;
                let names = spell_names(g_, c);
                let name = g_.ask(c, "fallen_feline_name", names, Map::new())?;
                let (src, oid) = (g_.it(it).src, g_.it(it).data.g("oid"));
                g_.add_effect(map! { kind = "feline_name", uid = src, oid = oid, name = name.clone() });
                let n = match &name {
                    Value::Str(s) => s.to_string(),
                    _ => "None".to_string(),
                };
                g_.log(|| format!("  Fallen Feline names {n}"));
                Ok(Value::None)
            });
            let (c, uid, oid) = (g.o(o).ctrl, g.o(o).uid, g.o(o).oid);
            g.queue_trigger(c, "Fallen Feline", res, map! { oid = oid }, Some(uid), false, Value::None, Value::None);
            Ok(Value::None)
        }),
        hooks: map! { forbid_play = Value::func("_feline_forbid", |g, a| {
            let (src, pid, c) = (a[0].obj(), a[1].int() as usize, a[2].obj());
            if pid == g.o(src).ctrl || !is_bf(g.o(src).loc) || g.spec(c).type_ != "Spell" {
                return Ok(false.into());
            }
            let (uid, oid, n) = (g.o(src).uid, g.o(src).oid, g.cname(c).to_string());
            Ok(g.effects
                .iter()
                .any(|e| {
                    e.d.g("kind").opt_str() == Some("feline_name")
                        && e.d.g("uid") == Value::from(uid)
                        && e.d.g("oid").int() == oid
                        && e.d.g("name").opt_str() == Some(n.as_str())
                })
                .into())
        }) },
        ..imp("Fallen Feline")
    });

    // Mageseeker Investigator — "Opponents must pay [A] for each unit beyond the first to move multiple units to my
    // battlefield at the same time."
    r.card(Impl {
        hooks: map! { move_tax = Value::func("Mageseeker Investigator.move_tax", |g, a| {
            let (src, pid, n, dest) = (a[0].obj(), a[1].int() as usize, a[2].list().len(), a[3].opt_loc());
            let l = g.o(src).loc;
            let reqs: Vec<Req> = if pid != g.o(src).ctrl && is_bf(l) && dest == l && n > 1 {
                vec![any_req(); n - 1]
            } else {
                vec![]
            };
            Ok(reqs_to_value(&reqs))
        }) },
        ..imp("Mageseeker Investigator")
    });

    // Hungry Wolf — "[Order]: Ready me and give me +1 might this turn. Use only if you've chosen an enemy unit this turn
    // and only once each turn." NON ENREGISTRÉE : le Python note l'usage dans `g.hist["wolf_used"]` (remis à zéro à
    // chaque tour avec `reset_hist`) ; le `Hist` de Rust n'a pas ce champ (API manquante : `Hist.wolf_used:
    // Vec<(u32, i64)>`). La carte reste bloquée.

    // Ivern, Friend to All — "As you play me, choose Bird, Cat, Dog, or Poro. I gain that tag. When I conquer or hold,
    // score 1 point if your units have all of the following tags among them — Bird, Cat, Dog, and Poro."
    fn have_tags(g: &mut Game, pid: usize) -> Res<BTreeSet<String>> {
        let mut have = BTreeSet::new();
        for u in g.units(Some(pid), None) {
            have.extend(g.tags(u)?);
        }
        Ok(have)
    }
    r.card(Impl {
        as_played: Value::func("_ivern_as_played", |g, a| {
            let pid = a[0].int() as usize;
            let have = have_tags(g, pid)?;
            let mut ts: Vec<&str> = PETS.to_vec();
            ts.sort_by(|x, y| (have.contains(*x), *x).cmp(&(have.contains(*y), *y)));
            Ok(maps(ts.into_iter().map(|t| map! { tag = t }).collect()))
        }),
        on_event: Value::func("_ivern_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if (ev == "conquer" || ev == "hold") && has_obj(&info.g("units"), o) {
                let res = Value::func("_ivern_event.res", |g_, a_| {
                    let c = g_.it(a_[0].item()).ctrl;
                    let have = have_tags(g_, c)?;
                    if PETS.iter().all(|t| have.contains(*t)) {
                        g_.gain_point(c, "Ivern, Friend to All")?;
                    }
                    Ok(Value::None)
                });
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                g.queue_trigger(c, "Ivern, Friend to All", res, Map::new(), Some(uid), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Ivern, Friend to All")
    });

    // Shady Spectacles — "[Equip] [1][Order]. As this is attached to a unit, choose another friendly unit. The equipped
    // unit becomes a copy of that unit for as long as this is attached to it." Might Bonus +0 (card image). The copy is
    // read through the gear (Obj.cname): it ends as soon as the gear is no longer attached to that unit.
    let mut im = Impl {
        bonus: Some(0),
        on_event: Value::func("_spectacles_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev == "attached" && info.g("gear").opt_obj() == Some(o) {
                let u = info.g("unit").obj();
                let xs: Vec<O> = g.units(Some(g.o(o).ctrl), None).into_iter().filter(|&x| x != u).collect();
                let others = sorted_by(g, xs, |g, &x| Ok((-value(g, x)?, g.o(x).uid)))?;
                g.o_mut(o).copy_name = None;
                if !others.is_empty() {
                    let c = g.o(o).ctrl;
                    let m = g.ask(c, "spectacles_copy", vals(&others), map! { unit = u })?.obj();
                    let mn = g.cname(m).to_string();
                    g.o_mut(o).copy_name = Some(mn);
                    g.o_mut(u).copy_src = Some(o);
                    let (ru, rm) = (g.repr(u), g.repr(m));
                    g.log(|| format!("  {ru} becomes a copy of {rm}"));
                }
            }
            Ok(Value::None)
        }),
        ..imp("Shady Spectacles")
    };
    impl_equip(&mut im, Cost::from("1 energy and 1 order rune"));
    r.card(im);

    // Undertitan — "When you play me, give your other units +2 might this turn. As I'm revealed from your deck, [Add] [2]."
    r.card(Impl {
        on_play: Value::func("_undertitan_play", |g, a| {
            let o = a[0].obj();
            let res = Value::func("_undertitan_play.res", |g_, a_| {
                let it = a_[0].item();
                let (c, src) = (g_.it(it).ctrl, g_.it(it).src);
                for u in g_.units(Some(c), None) {
                    if Some(g_.o(u).uid) != src {
                        g_.mod_(u, 2, "turn".into(), None)?;
                    }
                }
                Ok(Value::None)
            });
            let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
            g.queue_trigger(c, "Undertitan", res, Map::new(), Some(uid), false, Value::None, Value::None);
            Ok(Value::None)
        }),
        hooks: map! { on_reveal = Value::func("Undertitan.on_reveal", |g, a| {
            let owner = g.o(a[0].obj()).owner;
            g.add_pool(owner, 2, &[]);
            Ok(Value::None)
        }) },
        ..imp("Undertitan")
    });

    // Vanguard Helm — "When a buffed friendly unit dies, buff another friendly unit." (look-back 'buffed', rule 808.1.d.3)
    r.card(Impl {
        on_event: Value::func("_helm_event", |g, a| {
            let (o, ev, info) = (a[0].obj(), a[1].str(), a[2].map());
            if ev != "die" {
                return Ok(Value::None);
            }
            let inf = info.g("info");
            let inf = inf.map();
            if inf.g("buffed").truthy() && inf.g("ctrl").int() as usize == g.o(o).ctrl && lb_type(inf) == "Unit" {
                let res = Value::func("_helm_event.res", |g_, a_| {
                    let c = g_.it(a_[0].item()).ctrl;
                    let us = g_.units(Some(c), None);
                    let us = sorted_by(g_, us, |g, &u| Ok((g.o(u).buff > 0, -value(g, u)?, g.o(u).uid)))?;
                    if !us.is_empty() {
                        let t = if us.len() > 1 { g_.ask(c, "vanguard_helm", vals(&us), Map::new())?.obj() } else { us[0] };
                        g_.buff(Some(t), None)?;
                    }
                    Ok(Value::None)
                });
                let (c, uid) = (g.o(o).ctrl, g.o(o).uid);
                g.queue_trigger(c, "Vanguard Helm", res, Map::new(), Some(uid), false, Value::None, Value::None);
            }
            Ok(Value::None)
        }),
        ..imp("Vanguard Helm")
    });
}

/// `itertools.combinations(range(n), k)` : indices, dans l'ordre de Python.
fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = vec![];
    if k > n {
        return out;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        out.push(idx.clone());
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
