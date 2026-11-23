//! Paquet legends (`riftbound/engine/cardsets/legends.py`, liste : batches/legends.txt). Traduction ligne à ligne
//! du Python (voir `docs/memoire/portage-rust.md`).
//!
//! Les légendes ne sont pas sur le plateau : leurs capacités déclenchées sont `legend_event(g, pid, ev, info)`, leurs
//! capacités activées sont `abilities` (item.src est l'uid de la légende, `g.p[pid].legend` est l'objet légende, qui
//! peut être incliné / empowered), leurs capacités passives sur les unités sont `aura_kw` / `aura_might` (src =
//! l'objet légende, src.owner = son joueur) et leurs capacités [Add] sont `add` (utilisées en payant, règle 429.3).
//!
//! Cartes non enregistrées (il manque un champ d'état dans `game.rs`, voir le compte rendu du portage) : Vi, Piltover
//! Enforcer (`Showdown.legends_excess`) et Jhin, Virtuoso (`Obj.jhin`). Leur traduction est gardée plus bas, hors
//! compilation (`#[cfg(any())]`), écrite contre les champs naturels qui manquent.
use std::cmp::Ordering;

use crate::actions::{attach, card_choices, play_card};
use crate::cards::{Impl, Registry};
use crate::cards_helpers::{
    ability, add_ability, friends, impl_empower, make_token, may_pay, p_friend_bf, p_friend_v, p_gear_v,
    p_unit_bf_v, p_unit_v, tg_choices, trig_target, value, AbilityArgs, Cost,
};
use crate::game::{Desig, Game, ItemId, Res, Zone, O, R};
use crate::game_costs::{any_req, reqs_to_value};
use crate::map;
use crate::spec::spec;
use crate::value::{is_bf, Loc, Map, Value};

const MODULE: &str = "legends";

// ---------------------------------------------------------------------- aides privées
fn nil() -> Res<Value> {
    Ok(Value::None)
}

fn objs(v: &[O]) -> Value {
    Value::List(v.iter().map(|&o| Value::Obj(o)).collect())
}

fn maps(v: Vec<Map>) -> Value {
    Value::List(v.into_iter().map(Value::Map).collect())
}

/// `sorted(v, key=key)` : la clé est calculée une fois par élément, dans l'ordre de la liste ; tri stable.
fn sorted_by<K: PartialOrd>(
    g: &mut Game,
    v: Vec<O>,
    mut key: impl FnMut(&mut Game, O) -> Res<K>,
) -> Res<Vec<O>> {
    let mut ks = Vec::with_capacity(v.len());
    for o in v {
        ks.push((key(g, o)?, o));
    }
    ks.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(Ordering::Equal));
    Ok(ks.into_iter().map(|x| x.1).collect())
}

/// `g.stats[key] = v` (Counter de Python ; les clés tuple s'écrivent comme leur repr).
fn stat_set(g: &mut Game, k: &str, v: i64) {
    let cur = g.stats.get(k);
    g.stats.add(k, v - cur);
}

fn is_type(g: &Game, o: O, t: &str) -> bool {
    g.spec(o).type_ == t
}

fn turn() -> Value {
    Value::from("turn")
}

fn ctrl_of(g: &Game, a: &[Value]) -> usize {
    g.it(a[0].item()).ctrl
}

/// `ctx.get(k)` d'un contexte de paiement (dict ou None).
fn ctx_get(ctx: &Value, k: &str) -> Value {
    match ctx {
        Value::Map(m) => m.g(k),
        _ => Value::None,
    }
}

fn card(r: &mut Registry, im: Impl) {
    r.card(Impl { module: Some(MODULE), ..im });
}

fn tgs(g: &Game, us: &[O]) -> Value {
    maps(tg_choices(g, us))
}

/// `_leg(g, pid)`
fn leg(g: &Game, pid: usize) -> O {
    g.p[pid].legend
}

fn leg_uid(g: &Game, pid: usize) -> Option<u32> {
    Some(g.o(leg(g, pid)).uid)
}

/// `_exhaust_me(g, it)` : coût dans les instructions « exhaust me » d'un déclenchement de légende (règle 383.3.b) :
/// False si elle est déjà inclinée.
fn exhaust_me() -> Value {
    Value::func("_exhaust_me", |g, a| {
        let l = leg(g, ctrl_of(g, a));
        if g.exhausted(l) {
            return Ok(false.into());
        }
        g.set_exhausted(l, true);
        Ok(true.into())
    })
}

/// `_same(g, uid, oid)` : l'objet uid du plateau s'il est encore la même incarnation (règle 359.3.e.2).
fn same(g: &Game, uid: &Value, oid: &Value) -> Option<O> {
    let o = g.obj(uid.int() as u32)?;
    if g.o(o).oid == oid.int() {
        Some(o)
    } else {
        None
    }
}

/// `_is_equipment(o)`
fn is_equipment(g: &Game, o: O) -> bool {
    is_type(g, o, "Gear") && g.spec(o).tags.contains("Equipment")
}

/// `_friends_first(g, pid, us, key)` : unités dans l'ordre de l'IA : les alliées d'abord (par key), puis les ennemies
/// (par valeur).
fn friends_first<K: PartialOrd>(
    g: &mut Game,
    pid: usize,
    us: &[O],
    key: impl FnMut(&mut Game, O) -> Res<K>,
) -> Res<Vec<O>> {
    let mine: Vec<O> = us.iter().copied().filter(|&u| g.o(u).ctrl == pid).collect();
    let mut out = sorted_by(g, mine, key)?;
    let theirs: Vec<O> = us.iter().copied().filter(|&u| g.o(u).ctrl != pid).collect();
    out.extend(sorted_by(g, theirs, value)?);
    Ok(out)
}

/// Résolution « draw n » pour le contrôleur de l'item.
fn draw_res(n: i64) -> Value {
    Value::func("draw", move |g, a| {
        let c = ctrl_of(g, a);
        g.draw(c, n)?;
        nil()
    })
}

fn channel_exh_res() -> Value {
    Value::func("channel", |g, a| {
        let c = ctrl_of(g, a);
        g.channel(c, 1, true)?;
        nil()
    })
}

fn gold_res() -> Value {
    Value::func("gold", |g, a| {
        let c = ctrl_of(g, a);
        make_token(g, "Gold", c, Loc::Base, false)?;
        nil()
    })
}

fn ready_leg_res() -> Value {
    Value::func("ready_legend", |g, a| {
        let l = leg(g, ctrl_of(g, a));
        g.ready_obj(l, None, None)?;
        nil()
    })
}

fn empower_leg_res() -> Value {
    Value::func("empower_legend", |g, a| {
        let c = ctrl_of(g, a);
        let l = leg(g, c);
        g.empower(Some(l), Some(c))?;
        nil()
    })
}

fn units_targetable(g: &mut Game, pid: usize) -> Res<Vec<O>> {
    let mut us = vec![];
    for u in g.all_units() {
        if g.targetable(u, Some(pid))? {
            us.push(u);
        }
    }
    Ok(us)
}

// ---------------------------------------------------------------------- Ahri, Nine-Tailed Fox
// "When an enemy unit attacks a battlefield you control, give it -1 might this turn, to a minimum of 1 might."
fn ahri() -> Value {
    Value::func("_ahri", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev != "attack" {
            return nil();
        }
        let u = info.g("obj").obj();
        if g.o(u).ctrl == pid || g.bfs[info.g("bf").int() as usize].ctrl != Some(pid) {
            return nil();
        }
        let res = Value::func("res", |g_, a_| {
            let it = a_[0].item();
            let o = same(g_, &g_.it(it).data.g("u"), &g_.it(it).data.g("oid"));
            if let Some(o) = o {
                g_.mod_(o, -1, turn(), Some(1))?; // snapshot at resolution (rule 477.3.b)
            }
            nil()
        });
        let (uid, oid) = (g.o(u).uid, g.o(u).oid);
        let src = leg_uid(g, pid);
        g.queue_trigger(pid, "Ahri, Nine-Tailed Fox", res, map! { u = uid, oid = oid }, src, false, Value::None, Value::None);
        nil()
    })
}

// ---------------------------------------------------------------------- Annie, Dark Child
// "At the end of your turn, ready 2 runes."
fn exhausted_runes(g: &Game, pid: usize) -> Vec<R> {
    let mut ex: Vec<R> = g.p[pid].runes.iter().copied().filter(|&r| g.r(r).exhausted).collect();
    ex.sort_by(|&x, &y| (&g.r(x).domain, g.r(x).uid).cmp(&(&g.r(y).domain, g.r(y).uid)));
    ex
}

fn annie_res() -> Value {
    Value::func("_annie_res", |g, a| {
        let c = ctrl_of(g, a);
        for _ in 0..2 {
            let ex = exhausted_runes(g, c);
            if ex.is_empty() {
                return nil();
            }
            let r = g.ask(c, "ready_rune", ex.iter().map(|&r| Value::Rune(r)).collect(), Map::new())?.rune();
            g.r_mut(r).exhausted = false;
        }
        nil()
    })
}

fn annie() -> Value {
    Value::func("_annie", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "end_turn" && info.g("pid").int() as usize == pid {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Annie, Dark Child", annie_res(), Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Azir, Emperor of the Sands
// "Your Sand Soldiers have [Weaponmaster]. 1 energy, [E]: Play a 2 might Sand Soldier unit token to your base.
// Use only if you've played an Equipment this turn."
fn azir_aura() -> Value {
    Value::func("_azir_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        if g.o(o).ctrl == g.o(src).owner && is_type(g, o, "Unit") && g.cname(o) == "Sand Soldier" {
            return Ok(map! { Weaponmaster = 1 }.into());
        }
        nil()
    })
}

fn azir_play() -> Value {
    // Tokens are played (rule 185.2.a): make_token runs its play triggers, incl. [Weaponmaster] from the aura (rule 821).
    Value::func("_azir_play", |g, a| {
        let c = ctrl_of(g, a);
        make_token(g, "Sand Soldier", c, Loc::Base, false)?;
        nil()
    })
}

// ---------------------------------------------------------------------- [Add] legends
/// `_ornn_can(g, pid, o, ctx)`
fn ornn_can() -> Value {
    Value::func("_ornn_can", |g, a| {
        let ctx = &a[2];
        if ctx.is_none() {
            return Ok(false.into());
        }
        if ctx_get(ctx, "kind").opt_str() == Some("gear") {
            return Ok(true.into());
        }
        let ob = ctx_get(ctx, "obj").opt_obj();
        Ok((ctx_get(ctx, "kind").opt_str() == Some("ability")
            && ob.is_some_and(|ob| g.o(ob).zone == Some(Zone::Board) && is_type(g, ob, "Gear")))
        .into())
    })
}

// ---------------------------------------------------------------------- Draven, Glorious Executioner
// "When you win a combat, draw 1."
fn draven() -> Value {
    Value::func("_draven", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "combat_won" && info.g("pid").int() as usize == pid {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Draven, Glorious Executioner", draw_res(1), Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Ezreal, Prodigal Explorer
// "[E]: [Reaction] — Draw 1. Use only if you've chosen enemy units and/or gear twice this turn with spells or unit
// abilities."  Counted once per spell / unit ability that chose at least one enemy unit or gear (a targeting
// event, rule 383.4.b, happens when the item is finalized).
/// `_unit_item(g, item)`
fn unit_item(g: &Game, item: ItemId) -> bool {
    let it = g.it(item);
    if it.kind == "spell" {
        return true;
    }
    if it.kind == "trigger" && it.name.starts_with("Deathknell ") {
        return true; // Deathknell of a unit (its source has left the board)
    }
    let src = it.src.and_then(|s| g.obj(s));
    src.is_some_and(|s| is_type(g, s, "Unit"))
}

fn ezreal() -> Value {
    Value::func("_ezreal", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev != "chosen" {
            return nil();
        }
        let (it, o) = (info.g("item").item(), info.g("obj").obj());
        let t = g.spec(o).type_.as_str();
        if g.it(it).ctrl != pid || g.o(o).ctrl == pid || !(t == "Unit" || t == "Gear") || !unit_item(g, it) {
            return nil();
        }
        // clés tuple ("ezreal_last", pid) et ("ezreal", pid, g.turn_no), écrites comme leur repr
        let last = format!("('ezreal_last', {pid})");
        let id = g.it(it).id as i64;
        if g.stats.get(&last) == id {
            return nil();
        }
        stat_set(g, &last, id);
        let k = format!("('ezreal', {}, {})", pid, g.turn_no);
        g.stats.add(&k, 1);
        nil()
    })
}

// ---------------------------------------------------------------------- Fiora, Grand Duelist
// "When one of your units becomes [Mighty], you may exhaust me to channel 1 rune exhausted."
fn fiora() -> Value {
    Value::func("_fiora", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "becomes_mighty" && g.o(info.g("obj").obj()).ctrl == pid {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Fiora, Grand Duelist", channel_exh_res(), Map::new(), src, true, Value::None, exhaust_me());
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Garen, Might of Demacia
// "When you conquer, if you have 4+ units at that battlefield, draw 2."  The "if" is part of the trigger
// condition (rule 383.2.a.1): checked when the conquer happens only.
fn garen() -> Value {
    Value::func("_garen", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("pid").int() as usize == pid && info.g("units").list().len() >= 4 {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Garen, Might of Demacia", draw_res(2), Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Irelia, Blade Dancer
// "When you choose a friendly unit, you may exhaust me and pay 1 rune of any type to ready it. When you conquer,
// you may pay 1 energy to ready me."
/// `_irelia_cost` (avec ses attributs `can`, `label`, `label_en`)
fn irelia_cost() -> Value {
    // Pas de question « utiliser l'effet ? » (retour utilisateur) quand la légende est déjà inclinée, qu'aucune rune ne
    // peut être payée, ou que l'unité choisie est déjà prête (« ready it » ne ferait rien).
    let can = Value::func("_irelia_can", |g, a| {
        let it = a[0].item();
        let c = g.it(it).ctrl;
        let o = same(g, &g.it(it).data.g("u"), &g.it(it).data.g("oid"));
        let l = leg(g, c);
        Ok((!g.exhausted(l) && g.can_pay(c, 0, &[any_req()], &Value::None)? && o.is_some_and(|o| g.exhausted(o))).into())
    });
    let attrs = map! {
        can = can, label = "légende prête et 1 rune, pour une unité inclinée",
        label_en = "a ready legend and 1 rune, for an exhausted unit",
    };
    Value::func_attrs("_irelia_cost", attrs, |g, a| {
        let c = ctrl_of(g, a);
        let l = leg(g, c);
        if g.exhausted(l) || !g.can_pay(c, 0, &[any_req()], &Value::None)? {
            return Ok(false.into());
        }
        g.set_exhausted(l, true);
        Ok(g.pay(c, 0, &[any_req()], &Value::None)?.into())
    })
}

fn irelia_ready() -> Value {
    Value::func("_irelia_ready", |g, a| {
        let it = a[0].item();
        let o = same(g, &g.it(it).data.g("u"), &g.it(it).data.g("oid"));
        if let Some(o) = o {
            g.ready_obj(o, None, None)?;
        }
        nil()
    })
}

fn irelia() -> Value {
    Value::func("_irelia", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "chosen" {
            let (it, o) = (info.g("item").item(), info.g("obj").obj());
            if g.it(it).ctrl == pid && g.o(o).ctrl == pid && is_type(g, o, "Unit") {
                let (uid, oid) = (g.o(o).uid, g.o(o).oid);
                let src = leg_uid(g, pid);
                g.queue_trigger(pid, "Irelia, Blade Dancer", irelia_ready(), map! { u = uid, oid = oid }, src, true, Value::None, irelia_cost());
            }
        } else if ev == "conquer" && info.g("pid").int() as usize == pid {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Irelia, Blade Dancer (conquer)", ready_leg_res(), Map::new(), src, true, Value::None, may_pay(1, Value::None));
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Jax, Grandmaster At Arms
// "1 energy, [E]: Attach a detached Equipment you control to a unit you control. [E]: Attach an attached
// Equipment you control to a unit you control."  (Attach, rule 434: the Equipment's location follows the unit.)
fn jax_choices(attached: bool) -> Value {
    Value::func("_jax_choices", move |g, a| {
        let pid = a[0].int() as usize;
        let gears: Vec<O> = g
            .gear(Some(pid))
            .into_iter()
            .filter(|&x| is_equipment(g, x) && g.o(x).attached_to.is_some() == attached)
            .collect();
        let gears = sorted_by(g, gears, |g2, x| Ok((-g2.equip_bonus_of(g2.cname(x)), g2.o(x).uid)))?;
        let mut out = vec![];
        let fr = friends(g, pid, false, None)?;
        for u in sorted_by(g, fr, |g2, u| Ok((!is_bf(g2.o(u).loc), -value(g2, u)?)))? {
            for &x in &gears {
                if g.o(x).attached_to != Some(g.o(u).uid) {
                    out.push(map! { tg = Value::Tuple(vec![g.o(x).uid.into(), g.o(u).uid.into()]) });
                }
            }
        }
        Ok(maps(out))
    })
}

fn jax_pred(attached: bool) -> Value {
    Value::func("_jax_pred", move |g, a| {
        let (it, x) = (a[0].item(), a[1].obj());
        Ok((g.o(x).ctrl == g.it(it).ctrl && is_equipment(g, x) && g.o(x).attached_to.is_some() == attached).into())
    })
}

fn jax_res() -> Value {
    Value::func("_jax_res", |g, a| {
        let it = a[0].item();
        let (x, u) = (g.legal(it, 0)?, g.legal(it, 1)?);
        if let (Some(x), Some(u)) = (x, u) {
            if g.o(x).attached_to != Some(g.o(u).uid) {
                attach(g, x, u)?;
            }
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Jayce, Defender of Tomorrow
// "[Empower] 2 energy and 2 runes of any type. 1 energy, [E]: Ready a gear. [Empowered][>] 1 energy, [E]: Ready
// 2 gear."
fn gear_order(g: &mut Game, pid: usize) -> Res<Vec<O>> {
    let gs = g.gear(None);
    sorted_by(g, gs, |g2, x| Ok((!g2.exhausted(x), g2.o(x).ctrl != pid, -g2.spec(x).e, g2.o(x).uid)))
}

fn jayce_one() -> Value {
    Value::func("_jayce_one", |g, a| {
        let pid = a[0].int() as usize;
        let out: Vec<Map> = gear_order(g, pid)?
            .into_iter()
            .filter(|&x| g.exhausted(x))
            .map(|x| map! { tg = Value::Tuple(vec![g.o(x).uid.into()]) })
            .collect();
        Ok(maps(out))
    })
}

fn jayce_two() -> Value {
    Value::func("_jayce_two", |g, a| {
        let pid = a[0].int() as usize;
        let gs = gear_order(g, pid)?;
        let mut out = vec![];
        for (i, &x) in gs.iter().enumerate() {
            if !g.exhausted(x) || i >= 4 {
                break;
            }
            for &y in &gs[i + 1..] {
                out.push(map! { tg = Value::Tuple(vec![g.o(x).uid.into(), g.o(y).uid.into()]) });
            }
        }
        Ok(maps(out))
    })
}

fn jayce_res() -> Value {
    Value::func("_jayce_res", |g, a| {
        let it = a[0].item();
        for i in 0..g.it(it).targets.len() {
            if let Some(x) = g.legal(it, i)? {
                g.ready_obj(x, None, None)?;
            }
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Jinx, Loose Cannon
// "At start of your Beginning Phase, draw 1 if you have one or fewer cards in your hand."  (The "if" is part of
// the effect, checked on resolution: rule 383.2.a.1, Loose Cannon example.)
fn jinx() -> Value {
    Value::func("_jinx", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "beginning_start" && info.g("pid").int() as usize == pid {
            let res = Value::func("res", |g_, a_| {
                let c = ctrl_of(g_, a_);
                if g_.p[c].hand.len() <= 1 {
                    g_.draw(c, 1)?;
                }
                nil()
            });
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Jinx, Loose Cannon", res, Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Kennen, Heart of the Tempest
// "When you play a card from anywhere other than your hand, empower me. [Action][>] Disempower me, [E]: Give a
// unit [Assault 2] this turn."  (Tokens are not cards, rule 185.)
/// `_played_from(info)`
fn played_from(g: &Game, info: &Map) -> Option<String> {
    let c = info.g("card").obj();
    if is_type(g, c, "Spell") {
        return match info.g("item") {
            Value::Item(it) => g.it(it).data.g("from").opt_str().map(|s| s.to_string()),
            _ => None,
        };
    }
    g.o(c).played_from.clone()
}

fn kennen() -> Value {
    Value::func("_kennen", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "played"
            && info.g("pid").int() as usize == pid
            && !g.o(info.g("card").obj()).token
            && played_from(g, info).as_deref() != Some("hand")
        {
            let res = Value::func("res", |g_, a_| {
                let l = leg(g_, ctrl_of(g_, a_));
                g_.empower(Some(l), None)?;
                nil()
            });
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Kennen, Heart of the Tempest", res, Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

fn kennen_choices() -> Value {
    Value::func("_kennen_choices", |g, a| {
        let pid = a[0].int() as usize;
        let us = units_targetable(g, pid)?;
        let s = friends_first(g, pid, &us, |g2, u| Ok((!is_bf(g2.o(u).loc), -value(g2, u)?)))?;
        Ok(tgs(g, &s))
    })
}

/// `g.legal(it, 0) is not None and g.grant(g.legal(it, 0), kw, n, "turn")`
fn grant_res(kw: &'static str, n: i64) -> Value {
    Value::func("grant", move |g, a| {
        let it = a[0].item();
        if g.legal(it, 0)?.is_some() {
            if let Some(u) = g.legal(it, 0)? {
                g.grant(u, kw, n, turn());
            }
        }
        nil()
    })
}

/// `_disempower_me(g, pid, o, ch)` : la source de la capacité (Heimerdinger, Inventor peut l'emprunter).
fn disempower_me() -> Value {
    Value::func("_disempower_me", |g, a| {
        g.disempower(Some(a[1].obj()));
        nil()
    })
}

/// `_empowered(g, pid, o)`
fn empowered() -> Value {
    Value::func("_empowered", |g, a| Ok(g.o(a[1].obj()).empowered.into()))
}

// ---------------------------------------------------------------------- Kha'Zix, Voidreaver
// "When you win a combat, gain 1 XP. Spend 1 XP, [E]: [Buff] a unit. Spend 2 XP, [E]: Move an exhausted friendly
// unit from a battlefield to its base."
fn khazix() -> Value {
    Value::func("_khazix", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "combat_won" && info.g("pid").int() as usize == pid {
            let res = Value::func("res", |g_, a_| {
                let c = ctrl_of(g_, a_);
                g_.gain_xp(c, 1)?;
                nil()
            });
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Kha'Zix, Voidreaver", res, Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

/// `_buff_choices(g, pid, o, only_friends)`
fn buff_choices(only_friends: bool) -> Value {
    Value::func("_buff_choices", move |g, a| {
        let pid = a[0].int() as usize;
        let mut us = vec![];
        for u in g.all_units() {
            if g.targetable(u, Some(pid))? && (!only_friends || g.o(u).ctrl == pid) {
                us.push(u);
            }
        }
        let s = friends_first(g, pid, &us, |g2, u| Ok((g2.o(u).buff > 0, -value(g2, u)?)))?;
        Ok(tgs(g, &s))
    })
}

fn buff_res() -> Value {
    Value::func("buff", |g, a| {
        let u = g.legal(a[0].item(), 0)?;
        g.buff(u, None)?;
        nil()
    })
}

/// `_p_exh_friend_bf(g, it, o)`
fn p_exh_friend_bf() -> Value {
    Value::func("_p_exh_friend_bf", |g, a| {
        let (it, o) = (a[0].item(), a[1].obj());
        Ok((p_friend_bf(g, it, o) && g.exhausted(o)).into())
    })
}

fn khazix_move() -> Value {
    Value::func("_khazix_move", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            let c = g.it(it).ctrl;
            g.move_(&[u], Loc::Base, Some(c), false)?;
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Leona, Radiant Dawn
// "When you stun one or more enemy units, buff a friendly unit."  Units stunned by the same action trigger it once:
// their 'stun' events are emitted before the triggers are put on the chain, so a pending Leona trigger is reused.
fn leona() -> Value {
    Value::func("_leona", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev != "stun" || info.g("by").opt_int() != Some(pid as i64) || g.o(info.g("obj").obj()).ctrl == pid {
            return nil();
        }
        if g.trigq.iter().any(|&t| g.it(t).ctrl == pid && g.it(t).name == "Leona, Radiant Dawn") {
            return nil();
        }
        let opts = Value::func("opts", |g_, a_| {
            let c = ctrl_of(g_, a_);
            let us = g_.units(Some(c), None);
            let s = sorted_by(g_, us, |g2, u| Ok((g2.o(u).buff > 0, -value(g2, u)?)))?;
            Ok(objs(&s))
        });
        let src = leg_uid(g, pid);
        g.queue_trigger(
            pid,
            "Leona, Radiant Dawn",
            buff_res(),
            Map::new(),
            src,
            false,
            trig_target(opts, p_friend_v(), "buff_target", false, true),
            Value::None,
        );
        nil()
    })
}

// ---------------------------------------------------------------------- Lillia, Bashful Bloom
// "4 energy, [E]: Play a ready 3 might Sprite unit token with [Temporary]. This ability costs 1 energy less for
// each friendly unit with [Temporary]."
fn lillia_cost() -> Value {
    Value::func("_lillia_cost", |g, a| {
        let pid = a[0].int() as usize;
        let mut n = 0;
        for u in g.units(Some(pid), None) {
            if g.has_kw(u, "Temporary")? {
                n += 1;
            }
        }
        Ok(Value::Tuple(vec![Value::Int(0.max(4 - n)), reqs_to_value(&[])]))
    })
}

fn lillia_res() -> Value {
    Value::func("_lillia_res", |g, a| {
        let c = ctrl_of(g, a);
        make_token(g, "Sprite", c, Loc::Base, true)?;
        nil()
    })
}

// ---------------------------------------------------------------------- Lucian, Purifier
// "Your Equipment each give [Assault]."  Each Equipment you control attached to a unit gives it [Assault]
// (Assault values add up, rule 807.2).
fn lucian_aura() -> Value {
    Value::func("_lucian_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        if !is_type(g, o, "Unit") || g.o(o).attached.is_empty() {
            return nil();
        }
        let mut n = 0;
        for &gu in &g.o(o).attached {
            if let Some(x) = g.obj(gu) {
                if g.o(x).ctrl == g.o(src).owner && is_equipment(g, x) {
                    n += 1;
                }
            }
        }
        Ok(if n != 0 { map! { Assault = n }.into() } else { Value::None })
    })
}

// ---------------------------------------------------------------------- Lux, Lady of Luminosity
// "When you play a spell that costs 5 energy or more, draw 1."  Cost = printed cost (rule 206).
fn lux() -> Value {
    Value::func("_lux", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "played" && info.g("pid").int() as usize == pid {
            let c = info.g("card").obj();
            if is_type(g, c, "Spell") && g.spec(c).e >= 5 {
                let src = leg_uid(g, pid);
                g.queue_trigger(pid, "Lux, Lady of Luminosity", draw_res(1), Map::new(), src, false, Value::None, Value::None);
            }
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Master Yi, Wuju Bladesman
// "While a friendly unit defends alone, it gets +2 might."  (Alone: no other friendly unit there, rule 740.2.a.)
fn yi_aura() -> Value {
    Value::func("_yi_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        Ok(Value::Int(
            if g.o(o).ctrl == g.o(src).owner && is_type(g, o, "Unit") && g.o(o).desig == Some(Desig::Def) && g.alone(o, None) {
                2
            } else {
                0
            },
        ))
    })
}

// ---------------------------------------------------------------------- Miss Fortune, Bounty Hunter
// "[E]: Give a unit [Ganking] this turn."
fn mf_choices() -> Value {
    Value::func("_mf_choices", |g, a| {
        let pid = a[0].int() as usize;
        let us = units_targetable(g, pid)?;
        let s = friends_first(g, pid, &us, |g2, u| {
            Ok((g2.has_kw(u, "Ganking")?, !is_bf(g2.o(u).loc), g2.exhausted(u), -value(g2, u)?))
        })?;
        Ok(tgs(g, &s))
    })
}

// ---------------------------------------------------------------------- Poppy, Keeper of the Hammer
// "When you hold, gain 1 XP. Spend 3 XP, [E]: Draw 1."
fn poppy() -> Value {
    Value::func("_poppy", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("pid").int() as usize == pid {
            let res = Value::func("res", |g_, a_| {
                let c = ctrl_of(g_, a_);
                g_.gain_xp(c, 1)?;
                nil()
            });
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Poppy, Keeper of the Hammer", res, Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Pyke, Bloodharbor Ripper
// "1 energy, [E]: Return a friendly unit at a battlefield to its owner's hand. Play a Gold gear token exhausted."
// The ability resolves even if its target became illegal (rule 359.3.e.1): the Gold is still played.
fn pyke() -> Value {
    Value::func("_pyke", |g, a| {
        let it = a[0].item();
        if let Some(u) = g.legal(it, 0)? {
            g.to_zone(u, "hand", false, None)?; // to its owner's hand; a token ceases to exist (rule 186.1)
        }
        let c = g.it(it).ctrl;
        make_token(g, "Gold", c, Loc::Base, false)?;
        nil()
    })
}

fn pyke_choices() -> Value {
    Value::func("_pyke_choices", |g, a| {
        let pid = a[0].int() as usize;
        let fr = friends(g, pid, true, None)?;
        let s = sorted_by(g, fr, |g2, u| Ok((!g2.exhausted(u), g2.o(u).token, -g2.spec(u).e)))?;
        Ok(tgs(g, &s))
    })
}

// ---------------------------------------------------------------------- Rek'Sai, Void Burrower
// "When you conquer, you may exhaust me to reveal the top 2 cards of your Main Deck. You may play one. Then
// recycle the rest."  Playing it is a Limited Play (rule 419.3): its costs are paid as normal; it is played during
// the resolution of the ability, so it is not restricted to the Main Phase or to [Action]/[Reaction] cards.
fn reksai_res() -> Value {
    Value::func("_reksai_res", |g, a| {
        let pid = ctrl_of(g, a);
        let top = g.reveal(pid, 2, None, true, None)?; // fewer cards: reveal as many as possible (rule 431.1.c)
        if top.is_empty() {
            return nil();
        }
        let mut opts: Vec<(O, Vec<Map>)> = vec![];
        for &c in &top {
            let chs = card_choices(g, pid, c, "deck", false, false, false)?;
            if !chs.is_empty() {
                opts.push((c, chs));
            }
        }
        opts.sort_by_key(|x| {
            let s = g.spec(x.0);
            -(s.e + 2 * s.p)
        });
        let mut ov: Vec<Value> = opts.iter().map(|(c, _)| Value::Obj(*c)).collect();
        ov.push(Value::None);
        let pick = g.ask(pid, "reksai_pick", ov, Map::new())?.opt_obj();
        if let Some(p) = pick {
            let chs = opts.iter().find(|(c, _)| *c == p).unwrap().1.clone();
            let ch = g.ask(pid, "reksai_choice", chs.into_iter().map(Value::Map).collect(), map! { card = p })?;
            play_card(g, pid, p, "deck", ch.map().clone(), false)?;
        }
        let rest: Vec<O> = top.iter().copied().filter(|&c| Some(c) != pick && g.p[pid].deck.contains(&c)).collect();
        g.recycle_cards(pid, &rest)?;
        nil()
    })
}

fn reksai() -> Value {
    Value::func("_reksai", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("pid").int() as usize == pid {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Rek'sai, Void Burrower", reksai_res(), Map::new(), src, true, Value::None, exhaust_me());
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Rengar, Pridestalker
// "When you play a unit, give a unit +1 might this turn."
fn rengar() -> Value {
    Value::func("_rengar", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "played" && info.g("pid").int() as usize == pid && is_type(g, info.g("card").obj(), "Unit") {
            let opts = Value::func("opts", |g_, a_| {
                let c = ctrl_of(g_, a_);
                let us = units_targetable(g_, c)?;
                let s = friends_first(g_, c, &us, |g2, u| Ok((!is_bf(g2.o(u).loc), -value(g2, u)?)))?;
                Ok(objs(&s))
            });
            let res = Value::func("res", |g_, a_| {
                let it = a_[0].item();
                if g_.legal(it, 0)?.is_some() {
                    if let Some(u) = g_.legal(it, 0)? {
                        g_.mod_(u, 1, turn(), None)?;
                    }
                }
                nil()
            });
            let src = leg_uid(g, pid);
            g.queue_trigger(
                pid,
                "Rengar, Pridestalker",
                res,
                Map::new(),
                src,
                false,
                trig_target(opts, p_unit_v(), "rengar_target", false, true),
                Value::None,
            );
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Rumble, Mechanized Menace
// "Your Mechs have [Shield]."
fn rumble_aura() -> Value {
    Value::func("_rumble_aura", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        Ok(if g.o(o).ctrl == g.o(src).owner && is_type(g, o, "Unit") && g.tags(o)?.contains("Mech") {
            map! { Shield = 1 }.into()
        } else {
            Value::None
        })
    })
}

// ---------------------------------------------------------------------- Shen, Eye of Twilight
// "[Action][>] [E]: Give a friendly unit [Tank] this turn."
fn shen_choices() -> Value {
    Value::func("_shen_choices", |g, a| {
        let pid = a[0].int() as usize;
        let fr = friends(g, pid, false, None)?;
        let s = sorted_by(g, fr, |g2, u| Ok((!g2.in_combat(u), g2.has_kw(u, "Tank")?, -g2.might(u)?)))?;
        Ok(tgs(g, &s))
    })
}

// ---------------------------------------------------------------------- Vex, Gloomist
// "When you or an ally hold, you may exhaust me to draw 1."  (1v1: no ally.)
fn vex() -> Value {
    Value::func("_vex", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("pid").int() as usize == pid {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Vex, Gloomist", draw_res(1), Map::new(), src, true, Value::None, exhaust_me());
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Viktor, Herald of the Arcane
// "1 energy, [E]: Play a 1 might Recruit unit token."  (To its controller's base.)
fn viktor() -> Value {
    Value::func("_viktor", |g, a| {
        let c = ctrl_of(g, a);
        make_token(g, "Recruit", c, Loc::Base, false)?;
        nil()
    })
}

// ---------------------------------------------------------------------- Volibear, Relentless Storm
// "When you play a [Mighty] unit, you may exhaust me to channel 1 rune exhausted."
fn volibear() -> Value {
    Value::func("_volibear", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "played" && info.g("pid").int() as usize == pid {
            let c = info.g("card").obj();
            if is_type(g, c, "Unit") && g.mighty(c)? {
                let src = leg_uid(g, pid);
                g.queue_trigger(pid, "Volibear, Relentless Storm", channel_exh_res(), Map::new(), src, true, Value::None, exhaust_me());
            }
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Yasuo, Unforgiven
// "2 energy, [E]: Move a friendly unit to or from its base."
fn yasuo_choices() -> Value {
    Value::func("_yasuo_choices", |g, a| {
        let pid = a[0].int() as usize;
        let mut out: Vec<(O, Loc)> = vec![];
        for u in friends(g, pid, false, None)? {
            if g.o(u).loc == Some(Loc::Base) {
                for b in g.bf_locs() {
                    out.push((u, b));
                }
            } else {
                out.push((u, Loc::Base));
            }
        }
        let mut keyed = Vec::with_capacity(out.len());
        for (u, d) in out {
            let k = if d == Loc::Base {
                (1i64, !g.exhausted(u), -value(g, u)?)
            } else {
                let b = d.bf().unwrap();
                (0i64, g.bfs[b].ctrl == Some(pid), -value(g, u)?)
            };
            keyed.push((k, u, d));
        }
        keyed.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(Ordering::Equal));
        Ok(maps(
            keyed
                .into_iter()
                .map(|(_, u, d)| map! { tg = Value::Tuple(vec![g.o(u).uid.into()]), dest = d })
                .collect(),
        ))
    })
}

fn yasuo() -> Value {
    Value::func("_yasuo", |g, a| {
        let it = a[0].item();
        let u = g.legal(it, 0)?;
        let d = g.it(it).data.g("dest").opt_loc();
        let (Some(u), Some(d)) = (u, d) else { return nil() };
        if (d == Loc::Base && is_bf(g.o(u).loc)) || (d.is_bf() && g.o(u).loc == Some(Loc::Base)) {
            let c = g.it(it).ctrl;
            g.move_(&[u], d, Some(c), false)?;
        }
        nil()
    })
}

// ====================================================================== legends using the engine hooks (integration)
/// `_empower_echo(name)` : « When you empower something else, empower me. » (le joueur qui donne Empower : évènement
/// 'empowered' by=, règle 441)
fn empower_echo(name: &'static str) -> Value {
    Value::func("_empower_echo", move |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "empowered" && info.g("by").opt_int() == Some(pid as i64) && info.g("obj").obj() != leg(g, pid) {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, name, empower_leg_res(), Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

fn ambessa_choices() -> Value {
    Value::func("_ambessa_choices", |g, a| {
        let pid = a[0].int() as usize;
        let us = units_targetable(g, pid)?;
        let s = sorted_by(g, us, |g2, u| Ok((g2.o(u).ctrl != pid, !g2.exhausted(u), -value(g2, u)?, g2.o(u).uid)))?;
        Ok(tgs(g, &s))
    })
}

fn ready_target_res() -> Value {
    Value::func("ready", |g, a| {
        let it = a[0].item();
        if g.legal(it, 0)?.is_some() {
            if let Some(u) = g.legal(it, 0)? {
                g.ready_obj(u, None, None)?;
            }
        }
        nil()
    })
}

fn mel_choices() -> Value {
    Value::func("_mel_choices", |g, a| {
        let pid = a[0].int() as usize;
        let mut us = vec![];
        for u in g.all_units() {
            if is_bf(g.o(u).loc) && g.targetable(u, Some(pid))? {
                us.push(u);
            }
        }
        let s = sorted_by(g, us, |g2, u| Ok((g2.o(u).ctrl == pid, -value(g2, u)?, g2.o(u).uid)))?;
        Ok(tgs(g, &s))
    })
}

fn mel_res() -> Value {
    Value::func("mel", |g, a| {
        let it = a[0].item();
        if g.legal(it, 0)?.is_some() {
            if let Some(u) = g.legal(it, 0)? {
                g.mod_(u, -2, turn(), None)?;
            }
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Ivern, Green Father
// "When you conquer or hold, you may exhaust me to replace that battlefield with a Brush battlefield token."
// (Game.replace_battlefield, rule 438; the Brush token is in battlefields.rs)
fn ivern() -> Value {
    Value::func("_ivern", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if (ev == "conquer" || ev == "hold") && info.g("pid").int() as usize == pid {
            let res = Value::func("res", |g_, a_| {
                let b = g_.it(a_[0].item()).data.g("bf").int() as usize;
                g_.replace_battlefield(b, "Brush")?;
                nil()
            });
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Ivern, Green Father", res, map! { bf = info.g("bf") }, src, true, Value::None, exhaust_me());
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Master Yi, Wuju Master
// "[Level 6][>] Your units have +1 might. [Level 11][>] Your units enter ready."
fn yi_master_might() -> Value {
    Value::func("_yi_master_might", |g, a| {
        let (src, o) = (a[0].obj(), a[1].obj());
        let owner = g.o(src).owner;
        Ok(Value::Int(if g.o(o).ctrl == owner && is_type(g, o, "Unit") && g.p[owner].xp >= 6 { 1 } else { 0 }))
    })
}

fn yi_master_ready() -> Value {
    Value::func("_yi_master_ready", |g, a| {
        let (src, pid, c) = (a[0].obj(), a[1].int() as usize, a[2].obj());
        Ok((pid == g.o(src).owner && is_type(g, c, "Unit") && g.p[pid].xp >= 11).into())
    })
}

// ---------------------------------------------------------------------- Nasus, Curator of the Sands
// "When you play a unit, gear, or activated ability with Energy cost 7 energy or more, you may exhaust me to ready
// up to 2 runes." (the printed Energy cost, rule 206)
fn nasus_res() -> Value {
    Value::func("_nasus_res", |g, a| {
        let c = ctrl_of(g, a);
        for _ in 0..2 {
            let ex = exhausted_runes(g, c);
            if ex.is_empty() {
                return nil();
            }
            let mut opts: Vec<Value> = ex.iter().map(|&r| Value::Rune(r)).collect();
            opts.push(Value::None);
            let r = g.ask(c, "nasus_ready_rune", opts, Map::new())?;
            if r.is_none() {
                return nil();
            }
            g.r_mut(r.rune()).exhausted = false;
        }
        nil()
    })
}

fn nasus() -> Value {
    Value::func("_nasus", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if info.g("pid").opt_int() != Some(pid as i64) {
            return nil();
        }
        let hit = (ev == "played" && {
            let c = info.g("card").obj();
            let t = g.spec(c).type_.as_str();
            (t == "Unit" || t == "Gear") && g.spec(c).e >= 7
        }) || (ev == "activated" && info.g("cost_e").opt_int().unwrap_or(0) >= 7);
        if hit {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Nasus, Curator of the Sands", nasus_res(), Map::new(), src, true, Value::None, exhaust_me());
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Renata Glasc, Chem-Baroness
// "When you or an ally hold, you may exhaust me to play a Gold gear token exhausted. While your score is within 3
// points of the Victory Score, your Gold [ADD] an additional 1 energy." (1v1: no ally)
fn renata() -> Value {
    Value::func("_renata", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("pid").int() as usize == pid {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Renata Glasc, Chem-Baroness", gold_res(), Map::new(), src, true, Value::None, exhaust_me());
        }
        nil()
    })
}

fn renata_gold() -> Value {
    Value::func("_renata_gold", |g, a| {
        let (src, pid) = (a[0].obj(), a[1].int() as usize);
        Ok(Value::Int(if pid == g.o(src).owner && g.victory - g.p[pid].points <= 3 { 1 } else { 0 }))
    })
}

// ---------------------------------------------------------------------- Renekton, Butcher of the Sands
// "[Reaction][>] 2 runes of any type, exhaust: [Add] 2 energy. Spend this Energy only to play units or activated
// abilities of units." (an [Add] with a resource cost: Game.plan_convert 'p2e'; the rest floats restricted)
/// `_renekton_only(g, pid, ctx)`
fn renekton_only(g: &Game, ctx: &Value) -> bool {
    if !ctx.truthy() {
        return false;
    }
    if ctx_get(ctx, "kind").opt_str() == Some("unit") {
        return true;
    }
    let o = ctx_get(ctx, "obj").opt_obj();
    ctx_get(ctx, "kind").opt_str() == Some("ability")
        && o.is_some_and(|o| g.o(o).zone == Some(Zone::Board) && is_type(g, o, "Unit"))
}

// ---------------------------------------------------------------------- Sett, The Boss
// "When a buffed unit you control would die, you may pay 1 rune of any type and exhaust me to spend its buff and
// recall it exhausted instead. When you conquer, ready me." (recall doesn't heal: rule 455)
fn sett_rep() -> Value {
    Value::func("_sett_rep", |g, a| {
        let (src, u) = (a[0].obj(), a[1].obj());
        let pid = g.o(src).owner;
        if g.o(u).ctrl != pid || g.o(u).buff <= 0 || g.exhausted(src) || !g.can_pay(pid, 0, &[any_req()], &Value::None)? {
            return nil();
        }
        let cost = Value::func("cost", move |g_, _a| {
            let l = leg(g_, pid);
            if g_.exhausted(l) || !g_.pay(pid, 0, &[any_req()], &Value::None)? {
                return Ok(false.into());
            }
            g_.set_exhausted(l, true);
            Ok(true.into())
        });
        let apply = Value::func("apply", move |g_, a_| {
            let x = a_[0].obj();
            g_.spend_buff(Some(x), Some(pid))?;
            g_.save_unit(x, false, true);
            nil()
        });
        Ok(map! { name = "Sett, The Boss", apply = apply, may = true, cost = cost }.into())
    })
}

fn sett() -> Value {
    Value::func("_sett", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("pid").int() as usize == pid {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Sett, The Boss", ready_leg_res(), Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Sivir, Battle Mistress
// "When you recycle a rune, you may exhaust me to play a Gold gear token exhausted. When one or more enemy units
// die, ready me." (the deaths of one kill trigger it once, like Leona)
fn sivir() -> Value {
    Value::func("_sivir", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "rune_recycle" && info.g("pid").int() as usize == pid {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Sivir, Battle Mistress", gold_res(), Map::new(), src, true, Value::None, exhaust_me());
        } else if ev == "die" && {
            let inf = info.g("info");
            let inf = inf.map();
            spec(inf.g("spec").str()).type_ == "Unit" && inf.g("ctrl").int() as usize != pid
        } {
            if g.trigq.iter().any(|&t| g.it(t).ctrl == pid && g.it(t).name == "Sivir, Battle Mistress (ready)") {
                return nil();
            }
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Sivir, Battle Mistress (ready)", ready_leg_res(), Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------- Teemo, Swift Scout
// "You may pay 1 energy to hide a card with [Hidden] instead of 1 rune of any type. 1 energy, exhaust: Put a Teemo
// unit you own into your hand from your Champion Zone or the board."
fn teemo_cands(g: &mut Game, pid: usize) -> Res<Vec<O>> {
    let mut out: Vec<O> = g.p[pid]
        .champ
        .iter()
        .copied()
        .filter(|&c| is_type(g, c, "Unit") && g.spec(c).tags.contains("Teemo"))
        .collect();
    let mut bs = vec![];
    for u in g.board.clone() {
        if g.o(u).owner == pid && is_type(g, u, "Unit") && g.tags(u)?.contains("Teemo") {
            bs.push(u);
        }
    }
    out.extend(sorted_by(g, bs, |g2, u| Ok((!g2.exhausted(u), g2.o(u).uid)))?);
    Ok(out)
}

fn teemo_res() -> Value {
    Value::func("_teemo_res", |g, a| {
        let it = a[0].item();
        let ctrl = g.it(it).ctrl;
        let d = g.it(it).data.clone();
        let cu = d.g("c").int() as u32;
        let board = d.g("zone").opt_str() == Some("board");
        let c = if board { g.obj(cu) } else { g.p[ctrl].champ.iter().copied().find(|&x| g.o(x).uid == cu) };
        let Some(c) = c else { return nil() };
        if g.o(c).oid != d.g("oid").int() {
            return nil();
        }
        if board {
            g.to_zone(c, "hand", false, None)?;
        } else {
            if let Some(i) = g.p[ctrl].champ.iter().position(|&x| x == c) {
                g.p[ctrl].champ.remove(i);
            }
            g.o_mut(c).zone = Some(Zone::Hand);
            g.p[ctrl].hand.push(c);
        }
        nil()
    })
}

fn teemo_choices() -> Value {
    Value::func("_teemo_choices", |g, a| {
        let pid = a[0].int() as usize;
        let cs = teemo_cands(g, pid)?;
        Ok(maps(
            cs.into_iter()
                .map(|c| {
                    let z = if g.on_board(c) { "board" } else { "champ" };
                    map! { c = g.o(c).uid, oid = g.o(c).oid, zone = z }
                })
                .collect(),
        ))
    })
}

fn teemo_hide() -> Value {
    Value::func("_teemo_hide", |g, a| {
        let (src, pid) = (a[0].obj(), a[1].int() as usize);
        Ok(if pid == g.o(src).owner {
            Value::List(vec![Value::Tuple(vec![Value::Int(1), Value::List(vec![])])])
        } else {
            Value::List(vec![])
        })
    })
}

// ---------------------------------------------------------------------- Zed, Master of Shadows
// "When you banish a card you own, empower me. [Action][>] Disempower me, exhaust: Discard 1, then draw 1."
fn zed() -> Value {
    Value::func("_zed", |g, a| {
        let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "banish" && info.g("pid").opt_int() == Some(pid as i64) && g.o(info.g("card").obj()).owner == pid {
            let src = leg_uid(g, pid);
            g.queue_trigger(pid, "Zed, Master of Shadows", empower_leg_res(), Map::new(), src, false, Value::None, Value::None);
        }
        nil()
    })
}

fn zed_res() -> Value {
    Value::func("_zed_res", |g, a| {
        let c = ctrl_of(g, a);
        let mut h = g.p[c].hand.clone();
        h.sort_by_key(|&x| {
            let s = g.spec(x);
            (s.e + 2 * s.p, g.o(x).uid)
        });
        if !h.is_empty() {
            let d = g.ask(c, "zed_discard", objs(&h).list().to_vec(), Map::new())?;
            g.discard(c, d.obj())?;
        }
        g.draw(c, 1)?;
        nil()
    })
}

// ---------------------------------------------------------------------- enregistrement
fn hooks(kv: Vec<(&str, Value)>) -> Map {
    let mut m = Map::new();
    for (k, v) in kv {
        m.set(k, v);
    }
    m
}

fn text(s: &str) -> Cost {
    Cost::Text(s.to_string())
}

fn can_fn(f: impl Fn(&mut Game, &[Value]) -> Res<Value> + Send + Sync + 'static) -> Value {
    Value::func("can", f)
}

pub fn register(r: &mut Registry) {
    card(r, Impl { legend_event: ahri(), ..Impl::new("Ahri, Nine-Tailed Fox") });
    card(r, Impl { legend_event: annie(), ..Impl::new("Annie, Dark Child") });
    card(
        r,
        Impl {
            aura_kw: azir_aura(),
            abilities: vec![ability(
                "Sand Soldier",
                AbilityArgs {
                    cost: text("1 energy"),
                    exhaust: true,
                    resolve: azir_play(),
                    can: can_fn(|g, a| {
                        let pid = a[0].int() as usize;
                        Ok(g.played[pid].iter().any(|n| spec(n).tags.contains("Equipment")).into())
                    }),
                    ..Default::default()
                },
            )],
            ..Impl::new("Azir, Emperor of the Sands")
        },
    );

    // [Add] legends
    // Darius, Hand of Noxus — "[E]: [Reaction], [Legion] — [Add] 1 energy." (Legion: you've played a card this
    // turn, rule 812; while paying for a card, that card is not yet finalized)
    card(
        r,
        Impl {
            add: vec![add_ability("1 energy", true, false, can_fn(|g, a| Ok(g.legion(a[0].int() as usize, None).into())))],
            ..Impl::new("Darius, Hand of Noxus")
        },
    );
    // Diana, Scorn of the Moon — "[Reaction][>] [E]: [Add] 1 energy. Spend this Energy only during showdowns."
    // The Add is only used while paying (rule 429.3), so the energy is spent at once: only during a showdown.
    card(
        r,
        Impl { add: vec![add_ability("1 energy", true, false, can_fn(|g, _a| Ok(g.sd.is_some().into())))], ..Impl::new("Diana, Scorn of the Moon") },
    );
    // Kai'Sa, Daughter of the Void — "[E]: [Reaction] — [Add] 1 rune of any type. Use only to play spells."
    card(
        r,
        Impl {
            add: vec![add_ability(
                "1 rune of any type",
                true,
                false,
                can_fn(|_g, a| Ok((!a[2].is_none() && ctx_get(&a[2], "kind").opt_str() == Some("spell")).into())),
            )],
            ..Impl::new("Kai'Sa, Daughter of the Void")
        },
    );
    // Ornn, Fire Below the Mountain — "[E]: [Reaction] — [Add] 1 rune of any type. Use only to play gear or use gear
    // abilities."
    card(r, Impl { add: vec![add_ability("1 rune of any type", true, false, ornn_can())], ..Impl::new("Ornn, Fire Below the Mountain") });

    card(r, Impl { legend_event: draven(), ..Impl::new("Draven, Glorious Executioner") });
    card(
        r,
        Impl {
            legend_event: ezreal(),
            abilities: vec![ability(
                "Draw",
                AbilityArgs {
                    timing: "reaction",
                    exhaust: true,
                    resolve: draw_res(1),
                    can: can_fn(|g, a| {
                        let pid = a[0].int() as usize;
                        let k = format!("('ezreal', {}, {})", pid, g.turn_no);
                        Ok((g.stats.get(&k) >= 2).into())
                    }),
                    ..Default::default()
                },
            )],
            ..Impl::new("Ezreal, Prodigal Explorer")
        },
    );
    card(r, Impl { legend_event: fiora(), track_mighty: true, ..Impl::new("Fiora, Grand Duelist") });
    card(r, Impl { legend_event: garen(), ..Impl::new("Garen, Might of Demacia") });
    card(r, Impl { legend_event: irelia(), ..Impl::new("Irelia, Blade Dancer") });
    card(
        r,
        Impl {
            abilities: vec![
                ability(
                    "Attach detached",
                    AbilityArgs {
                        cost: text("1 energy"),
                        exhaust: true,
                        choices: jax_choices(false),
                        preds: Value::List(vec![jax_pred(false), p_friend_v()]),
                        resolve: jax_res(),
                        ..Default::default()
                    },
                ),
                ability(
                    "Attach attached",
                    AbilityArgs {
                        exhaust: true,
                        choices: jax_choices(true),
                        preds: Value::List(vec![jax_pred(true), p_friend_v()]),
                        resolve: jax_res(),
                        ..Default::default()
                    },
                ),
            ],
            ..Impl::new("Jax, Grandmaster At Arms")
        },
    );
    let mut jayce = Impl {
        abilities: vec![
            ability(
                "Ready a gear",
                AbilityArgs {
                    cost: text("1 energy"),
                    exhaust: true,
                    choices: jayce_one(),
                    preds: Value::List(vec![p_gear_v()]),
                    resolve: jayce_res(),
                    ..Default::default()
                },
            ),
            ability(
                "Ready 2 gear",
                AbilityArgs {
                    cost: text("1 energy"),
                    exhaust: true,
                    choices: jayce_two(),
                    preds: Value::List(vec![p_gear_v(), p_gear_v()]),
                    resolve: jayce_res(),
                    can: can_fn(|g, a| Ok(g.o(a[1].obj()).empowered.into())),
                    ..Default::default()
                },
            ),
        ],
        ..Impl::new("Jayce, Defender of Tomorrow")
    };
    impl_empower(&mut jayce, text("2 energy and 2 runes of any type"));
    card(r, jayce);
    card(r, Impl { legend_event: jinx(), ..Impl::new("Jinx, Loose Cannon") });
    card(
        r,
        Impl {
            legend_event: kennen(),
            abilities: vec![ability(
                "Assault 2",
                AbilityArgs {
                    timing: "action",
                    exhaust: true,
                    choices: kennen_choices(),
                    preds: Value::List(vec![p_unit_v()]),
                    can: can_fn(|g, a| Ok(g.o(a[1].obj()).empowered.into())),
                    extra_cost: Value::func("extra_cost", |g, a| {
                        g.disempower(Some(a[1].obj()));
                        nil()
                    }),
                    resolve: grant_res("Assault", 2),
                    ..Default::default()
                },
            )],
            ..Impl::new("Kennen, Heart of the Tempest")
        },
    );
    card(
        r,
        Impl {
            legend_event: khazix(),
            abilities: vec![
                ability(
                    "Buff",
                    AbilityArgs {
                        xp: 1,
                        exhaust: true,
                        choices: buff_choices(false),
                        preds: Value::List(vec![p_unit_v()]),
                        resolve: buff_res(),
                        ..Default::default()
                    },
                ),
                ability(
                    "Move to base",
                    AbilityArgs {
                        xp: 2,
                        exhaust: true,
                        preds: Value::List(vec![p_exh_friend_bf()]),
                        resolve: khazix_move(),
                        choices: Value::func("choices", |g, a| {
                            let pid = a[0].int() as usize;
                            let us: Vec<O> = friends(g, pid, true, None)?.into_iter().filter(|&u| g.exhausted(u)).collect();
                            Ok(tgs(g, &us))
                        }),
                        ..Default::default()
                    },
                ),
            ],
            ..Impl::new("Kha'Zix, Voidreaver")
        },
    );
    // Lee Sin, Blind Monk — "1 energy, [E]: Buff a friendly unit."
    card(
        r,
        Impl {
            abilities: vec![ability(
                "Buff",
                AbilityArgs {
                    cost: text("1 energy"),
                    exhaust: true,
                    preds: Value::List(vec![p_friend_v()]),
                    resolve: buff_res(),
                    choices: buff_choices(true),
                    ..Default::default()
                },
            )],
            ..Impl::new("Lee Sin, Blind Monk")
        },
    );
    card(r, Impl { legend_event: leona(), ..Impl::new("Leona, Radiant Dawn") });
    card(
        r,
        Impl {
            abilities: vec![ability(
                "Sprite",
                AbilityArgs { cost: Cost::Fn(lillia_cost()), exhaust: true, resolve: lillia_res(), ..Default::default() },
            )],
            ..Impl::new("Lillia, Bashful Bloom")
        },
    );
    card(r, Impl { aura_kw: lucian_aura(), ..Impl::new("Lucian, Purifier") });
    card(r, Impl { legend_event: lux(), ..Impl::new("Lux, Lady of Luminosity") });
    card(r, Impl { aura_might: yi_aura(), ..Impl::new("Master Yi, Wuju Bladesman") });
    // Miss Fortune, Bounty Hunter — "[E]: Give a unit [Ganking] this turn."
    card(
        r,
        Impl {
            abilities: vec![ability(
                "Ganking",
                AbilityArgs {
                    exhaust: true,
                    choices: mf_choices(),
                    preds: Value::List(vec![p_unit_v()]),
                    resolve: grant_res("Ganking", 1),
                    ..Default::default()
                },
            )],
            ..Impl::new("Miss Fortune, Bounty Hunter")
        },
    );
    card(
        r,
        Impl {
            legend_event: poppy(),
            abilities: vec![ability("Draw", AbilityArgs { xp: 3, exhaust: true, resolve: draw_res(1), ..Default::default() })],
            ..Impl::new("Poppy, Keeper of the Hammer")
        },
    );
    card(
        r,
        Impl {
            abilities: vec![ability(
                "Return",
                AbilityArgs {
                    cost: text("1 energy"),
                    exhaust: true,
                    preds: Value::List(vec![crate::cards_helpers::p_friend_bf_v()]),
                    resolve: pyke(),
                    choices: pyke_choices(),
                    ..Default::default()
                },
            )],
            ..Impl::new("Pyke, Bloodharbor Ripper")
        },
    );
    card(r, Impl { legend_event: reksai(), ..Impl::new("Rek'sai, Void Burrower") });
    card(r, Impl { legend_event: rengar(), ..Impl::new("Rengar, Pridestalker") });
    card(r, Impl { aura_kw: rumble_aura(), ..Impl::new("Rumble, Mechanized Menace") });
    // Shen, Eye of Twilight — "[Action][>] [E]: Give a friendly unit [Tank] this turn."
    card(
        r,
        Impl {
            abilities: vec![ability(
                "Tank",
                AbilityArgs {
                    timing: "action",
                    exhaust: true,
                    preds: Value::List(vec![p_friend_v()]),
                    choices: shen_choices(),
                    resolve: grant_res("Tank", 1),
                    ..Default::default()
                },
            )],
            ..Impl::new("Shen, Eye of Twilight")
        },
    );
    card(r, Impl { legend_event: vex(), ..Impl::new("Vex, Gloomist") });
    // Vi, Piltover Enforcer : non enregistrée (Showdown.legends_excess : voir plus bas)
    card(
        r,
        Impl {
            abilities: vec![ability(
                "Recruit",
                AbilityArgs { cost: text("1 energy"), exhaust: true, resolve: viktor(), ..Default::default() },
            )],
            ..Impl::new("Viktor, Herald of the Arcane")
        },
    );
    card(r, Impl { legend_event: volibear(), ..Impl::new("Volibear, Relentless Storm") });
    card(
        r,
        Impl {
            abilities: vec![ability(
                "Move",
                AbilityArgs {
                    cost: text("2 energy"),
                    exhaust: true,
                    choices: yasuo_choices(),
                    preds: Value::List(vec![p_friend_v()]),
                    resolve: yasuo(),
                    ..Default::default()
                },
            )],
            ..Impl::new("Yasuo, Unforgiven")
        },
    );

    // legends using the engine hooks (integration)
    r.ask_text(&[("nasus_ready_rune", "Nasus : quelle rune redresser ?"), ("zed_discard", "Zed : quelle carte défausser ?")]);

    // Ambessa, Matriarch of War — "When you empower something else, empower me. Disempower me, 1 rune of any type,
    // exhaust: Ready a unit."
    card(
        r,
        Impl {
            legend_event: empower_echo("Ambessa, Matriarch of War"),
            abilities: vec![ability(
                "Ready",
                AbilityArgs {
                    cost: text("1 rune of any type"),
                    exhaust: true,
                    preds: Value::List(vec![p_unit_v()]),
                    can: empowered(),
                    extra_cost: disempower_me(),
                    choices: ambessa_choices(),
                    resolve: ready_target_res(),
                    ..Default::default()
                },
            )],
            ..Impl::new("Ambessa, Matriarch of War")
        },
    );
    // Mel, Soul's Reflection — "When you empower something else, empower me. Disempower me, exhaust: Give a unit at a
    // battlefield -2 might this turn."
    card(
        r,
        Impl {
            legend_event: empower_echo("Mel, Soul's Reflection"),
            abilities: vec![ability(
                "-2 might",
                AbilityArgs {
                    cost: Cost::None,
                    exhaust: true,
                    preds: Value::List(vec![p_unit_bf_v()]),
                    can: empowered(),
                    extra_cost: disempower_me(),
                    choices: mel_choices(),
                    resolve: mel_res(),
                    ..Default::default()
                },
            )],
            ..Impl::new("Mel, Soul's Reflection")
        },
    );
    card(r, Impl { legend_event: ivern(), ..Impl::new("Ivern, Green Father") });
    // Jhin, Virtuoso : non enregistrée (Obj.jhin : voir plus bas)
    card(
        r,
        Impl {
            aura_might: yi_master_might(),
            hooks: hooks(vec![("units_enter_ready", yi_master_ready())]),
            ..Impl::new("Master Yi, Wuju Master")
        },
    );
    card(r, Impl { legend_event: nasus(), ..Impl::new("Nasus, Curator of the Sands") });
    card(r, Impl { legend_event: renata(), hooks: hooks(vec![("gold_extra", renata_gold())]), ..Impl::new("Renata Glasc, Chem-Baroness") });
    card(
        r,
        Impl {
            add: vec![map! {
                conv = "p2e", n = 2, exhaust = true,
                only = Value::func("_renekton_only", |g, a| Ok(renekton_only(g, &a[1]).into())),
                can = Value::func("can", |g, a| Ok(renekton_only(g, &a[2]).into())),
            }],
            ..Impl::new("Renekton, Butcher of the Sands")
        },
    );
    card(r, Impl { legend_event: sett(), hooks: hooks(vec![("death_rep", sett_rep())]), ..Impl::new("Sett, The Boss") });
    card(r, Impl { legend_event: sivir(), ..Impl::new("Sivir, Battle Mistress") });
    card(
        r,
        Impl {
            hooks: hooks(vec![("hide_cost", teemo_hide())]),
            abilities: vec![ability(
                "Teemo to hand",
                AbilityArgs {
                    cost: text("1 energy"),
                    exhaust: true,
                    can: can_fn(|g, a| Ok((!teemo_cands(g, a[0].int() as usize)?.is_empty()).into())),
                    choices: teemo_choices(),
                    resolve: teemo_res(),
                    ..Default::default()
                },
            )],
            ..Impl::new("Teemo, Swift Scout")
        },
    );
    card(
        r,
        Impl {
            legend_event: zed(),
            abilities: vec![ability(
                "Discard/draw",
                AbilityArgs {
                    cost: Cost::None,
                    timing: "action",
                    exhaust: true,
                    can: empowered(),
                    extra_cost: disempower_me(),
                    resolve: zed_res(),
                    ..Default::default()
                },
            )],
            ..Impl::new("Zed, Master of Shadows")
        },
    );
}

// ====================================================================== cartes non enregistrées (état manquant)
// Traductions écrites contre les champs naturels qui manquent dans `game.rs` ; jamais compilées tant que ces champs
// n'existent pas. Champs attendus :
// - `Showdown.legends_excess: Option<[i64; 2]>`  (Python : `sd.legends_excess`, créé à la volée) ; `sd.assigned`
//   est lu comme deux listes de (objet, dégâts) : `mine[u]` = les dégâts assignés à u.
// - `Obj.jhin: Option<Vec<(O, i64)>>`            (Python : `leg.jhin`, les sorts bannis avec Jhin et leur oid)
#[cfg(any())]
mod missing_state {
    use super::*;

    // Vi, Piltover Enforcer — "When you conquer, if you assigned 3 or more excess damage, you may exhaust me to ready a
    // unit." Excess damage = combat damage assigned to a unit beyond the lethal damage it needed (rules 142.4,
    // 437.5.a, 465.2.c.3). It is measured when the combat damage is dealt (the 'damaged' events of the combat, before
    // the units die) from the assignment kept in g.sd.assigned, and stored on the showdown. The "if" is part of the
    // trigger condition (rule 383.2.a.1).
    fn vi_record(g: &mut Game, pid: usize, info: &Map) -> Res<()> {
        let Some(sd) = g.sd.as_ref() else { return Ok(()) };
        if !sd.combat || sd.assigned.is_none() {
            return Ok(());
        }
        let mine = sd.assigned.list()[if pid == sd.attacker { 0 } else { 1 }].clone();
        let u = info.g("obj").obj();
        let Some(n) = mine.list().iter().find(|p| p.list()[0].obj() == u).map(|p| p.list()[1].int()) else {
            return Ok(());
        };
        let dealt = info.g("amount").int();
        let prevent_before = g.o(u).prevent + (n - dealt); // the rest of the assignment was prevented
        let need = 1.max(g.might(u)? - (g.o(u).damage - dealt)) + prevent_before;
        if n > need {
            let sd = g.sd.as_mut().unwrap();
            let ex = sd.legends_excess.get_or_insert([0, 0]);
            ex[pid] += n - need;
        }
        Ok(())
    }

    fn vi() -> Value {
        Value::func("_vi", |g, a| {
            let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
            if ev == "damaged" && info.g("kind").opt_str() == Some("combat") {
                vi_record(g, pid, info)?;
            } else if ev == "conquer" && info.g("pid").int() as usize == pid {
                let Some(sd) = g.sd.as_ref() else { return nil() };
                if !sd.combat || sd.bf != info.g("bf").int() as usize {
                    return nil();
                }
                if sd.legends_excess.unwrap_or([0, 0])[pid] < 3 {
                    return nil();
                }
                let opts = Value::func("opts", |g_, a_| {
                    let c = ctrl_of(g_, a_);
                    let us = units_targetable(g_, c)?;
                    Ok(objs(&friends_first(g_, c, &us, |g2, u| Ok((!g2.exhausted(u), -value(g2, u)?)))?))
                });
                let src = leg_uid(g, pid);
                g.queue_trigger(
                    pid,
                    "Vi, Piltover Enforcer",
                    ready_target_res(),
                    Map::new(),
                    src,
                    true,
                    trig_target(opts, p_unit_v(), "ready_target", false, true),
                    exhaust_me(),
                );
            }
            nil()
        })
    }

    // Jhin, Virtuoso — "When you play a spell, if you spent 4 energy or more, you may banish it. Then, if there are
    // four spells banished with me, put each in its trash, channel 4 runes, and draw 1." (the spells banished with
    // Jhin: legend.jhin)
    fn jhin_res() -> Value {
        Value::func("_jhin_res", |g, a| {
            let it = a[0].item();
            let c0 = g.it(it).ctrl;
            let l = leg(g, c0);
            let c = g.it(it).data.g("card").obj();
            if g.o(c).zone == Some(Zone::Trash) && g.o(c).oid == g.it(it).data.g("oid").int() {
                g.to_zone(c, "banish", false, Some(c0))?;
                let oid = g.o(c).oid;
                let mut j = g.o(l).jhin.clone().unwrap_or_default();
                j.push((c, oid));
                g.o_mut(l).jhin = Some(j);
            }
            let with_me: Vec<(O, i64)> = g
                .o(l)
                .jhin
                .clone()
                .unwrap_or_default()
                .into_iter()
                .filter(|&(x, oid)| g.o(x).zone == Some(Zone::Banish) && g.o(x).oid == oid)
                .collect();
            g.o_mut(l).jhin = Some(with_me.clone());
            if with_me.len() >= 4 {
                g.o_mut(l).jhin = Some(vec![]);
                for (x, _) in with_me {
                    g.to_zone(x, "trash", false, Some(c0))?;
                }
                g.channel(c0, 4, false)?;
                g.draw(c0, 1)?;
            }
            nil()
        })
    }

    fn jhin() -> Value {
        Value::func("_jhin", |g, a| {
            let (pid, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
            let Value::Item(it) = info.g("item") else { return nil() };
            if ev == "played"
                && info.g("pid").int() as usize == pid
                && is_type(g, info.g("card").obj(), "Spell")
                && g.it(it).data.g("paid_e").opt_int().unwrap_or(0) >= 4
            {
                let c = info.g("card").obj();
                let oid = g.o(c).oid;
                let src = leg_uid(g, pid);
                g.queue_trigger(pid, "Jhin, Virtuoso", jhin_res(), map! { card = c, oid = oid }, src, true, Value::None, Value::None);
            }
            nil()
        })
    }
}
