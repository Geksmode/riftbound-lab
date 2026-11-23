//! Paquet battlefields (`riftbound/engine/cardsets/battlefields.py`) : les battlefields de batches/battlefields.txt.
//! Traduction ligne à ligne du Python (voir `docs/memoire/portage-rust.md`).
//!
//! Conventions (reprises du Python)
//! - Les capacités d'un battlefield sont contrôlées par son contrôleur, ou par le joueur du tour tant qu'il n'est pas
//!   contrôlé (règle 190.6.a/b). Les capacités qui nomment un joueur (« When you conquer here », « they may », « that
//!   player ») sont contrôlées par ce joueur.
//! - « When you defend here » est un déclenchement de joueur (règle 383.4.f) : une fois par combat, quand la première
//!   unité de ce joueur prend la désignation Defender à ce battlefield.
//!
//! Un battlefield passé à un crochet (`bf_event`, `aura_*`, `cost_aura`...) est `Value::Int(idx)`.
//!
//! Les attributs posés à la volée par le Python (`sd._bf_defend_seen`, `sd._bf_excess`, `pl.bf_chosen_champion`)
//! sont rangés dans `attrs`, sous le même nom.
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::actions::{attach, card_kw};
use crate::cards::{Impl, Registry};
use crate::cards_helpers::{
    ability, all_units, friends, make_token, may_pay, p_friend_v, p_gear_v, trig_target, value, AbilityArgs, Cost,
};
use crate::game::{Game, Res, Zone, O};
use crate::game_costs::{any_req, reqs_to_value};
use crate::map;
use crate::value::{is_bf, Loc, Map, Value};

const MODULE: &str = "battlefields";

// ---------------------------------------------------------------------------------------------- aides privées
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

fn here_loc(b: usize) -> Option<Loc> {
    Some(Loc::Bf(b))
}

/// `_bf_ctrl(g, b)` : contrôleur d'une capacité de battlefield qui ne nomme aucun joueur (règle 190.6).
fn bf_ctrl(g: &Game, b: usize) -> usize {
    g.bfs[b].ctrl.unwrap_or(g.tp)
}

/// `_here(b)`
fn here(b: usize) -> Value {
    Value::func("_here", move |g, a| Ok((g.o(a[1].obj()).loc == here_loc(b)).into()))
}

fn is_type(g: &Game, o: O, t: &str) -> bool {
    g.spec(o).type_ == t
}

fn turn() -> Value {
    Value::from("turn")
}

fn card(r: &mut Registry, im: Impl) {
    r.card(Impl { module: Some(MODULE), ..im });
}

// ---------------------------------------------------------------------------------------------- Abandoned Hall
// "When a player plays a spell, they may give a unit they control here +1 might this turn."
// (a spell is played when it resolves, rule 419.4.a: engine event 'played')
fn abandoned_hall() -> Value {
    Value::func("_abandoned_hall", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev != "played" || !is_type(g, info.g("card").obj(), "Spell") {
            return nil();
        }
        let pid = info.g("pid").int() as usize;
        if g.units(Some(pid), here_loc(b)).is_empty() {
            return nil();
        }
        let res = Value::func("res", |g_, a_| {
            let it = a_[0].item();
            if let Some(u) = g_.legal(it, 0)? {
                g_.mod_(u, 1, turn(), None)?;
            }
            nil()
        });
        let opts = Value::func("opts", move |g_, a_| {
            let c = g_.it(a_[0].item()).ctrl;
            Ok(objs(&friends(g_, c, false, here_loc(b))?))
        });
        let pred = Value::func("pred", move |g_, a_| {
            let (it, o) = (a_[0].item(), a_[1].obj());
            Ok((g_.o(o).ctrl == g_.it(it).ctrl && g_.o(o).loc == here_loc(b)).into())
        });
        g.queue_trigger(pid, "Abandoned Hall", res, Map::new(), None, true, trig_target(opts, pred, "target", false, true), Value::None);
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Altar to Unity
// "When you hold here, play a 1 might Recruit unit token in your base."
fn altar_unity() -> Value {
    Value::func("_altar_unity", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                make_token(g_, "Recruit", c, Loc::Base, false)?;
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Altar to Unity", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Amateur Recital
// "When you hold here, you may move a unit at a battlefield to its base."
fn recital() -> Value {
    Value::func("_recital", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a_| {
                let it = a_[0].item();
                if let Some(u) = g_.legal(it, 0)? {
                    let c = g_.it(it).ctrl;
                    g_.move_(&[u], Loc::Base, Some(c), false)?;
                }
                nil()
            });
            let opts = Value::func("opts", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                Ok(objs(&all_units(g_, c, true, None)?))
            });
            let pred = Value::func("pred", |g_, a_| Ok(is_bf(g_.o(a_[1].obj()).loc).into()));
            g.queue_trigger(
                info.g("pid").int() as usize,
                "Amateur Recital",
                res,
                Map::new(),
                None,
                true,
                trig_target(opts, pred, "target", false, true),
                Value::None,
            );
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Black Flame Altar
// "Units here with [Temporary] have [Shield]."
thread_local! {
    /// `_BFA_GUARD` : cette aura ne donne pas Temporary ; ne pas l'évaluer pendant le test de Temporary.
    static BFA_GUARD: RefCell<BTreeSet<u32>> = const { RefCell::new(BTreeSet::new()) };
}

fn bfa_aura() -> Value {
    Value::func("_bfa_aura", |g, a| {
        let (b, o) = (a[0].int() as usize, a[1].obj());
        let uid = g.o(o).uid;
        if g.o(o).loc != here_loc(b) || !is_type(g, o, "Unit") || BFA_GUARD.with(|s| s.borrow().contains(&uid)) {
            return nil();
        }
        BFA_GUARD.with(|s| s.borrow_mut().insert(uid));
        let t = g.has_kw(o, "Temporary");
        BFA_GUARD.with(|s| s.borrow_mut().remove(&uid));
        Ok(if t? { map! { Shield = 1 }.into() } else { Value::None })
    })
}

// ---------------------------------------------------------------------------------------------- Emperor's Dais
// "When you conquer here, you may pay 1 energy and return a unit you control here to its owner's hand. If you do,
// play a 2 might Sand Soldier unit token here."  (costs within instructions, rule 355.10.c.1: not a target)
fn dais() -> Value {
    Value::func("_dais", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b {
            let here = b;
            let res = Value::func("res", move |g_, a_| {
                let it = a_[0].item();
                let c = g_.it(it).ctrl;
                let us = g_.units(Some(c), here_loc(here));
                let us = sorted_by(g_, us, value)?;
                if us.is_empty() || !g_.can_pay(c, 1, &[], &Value::None)? {
                    return nil();
                }
                if !g_.ask(c, "may", vec![true.into(), false.into()], map! { item = it })?.truthy() {
                    return nil();
                }
                let u = g_.ask(c, "dais_return", us.iter().map(|&x| Value::Obj(x)).collect(), Map::new())?.opt_obj();
                let Some(u) = u else { return nil() };
                if !g_.pay(c, 1, &[], &Value::None)? {
                    return nil();
                }
                g_.to_zone(u, "hand", false, None)?;
                make_token(g_, "Sand Soldier", c, Loc::Bf(here), false)?;
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Emperor's Dais", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Frozen Fortress
// "At the start of each player's Beginning Phase, deal 1 to each unit here. (This happens before scoring.)"
fn frozen() -> Value {
    Value::func("_frozen", |g, a| {
        let (b, ev) = (a[0].int() as usize, a[1].str());
        if ev == "beginning_start" {
            let res = Value::func("res", move |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                for u in g_.units(None, here_loc(b)) {
                    g_.deal(Some(u), 1, "ability", Some(c), None)?;
                }
                nil()
            });
            let p = bf_ctrl(g, b);
            g.queue_trigger(p, "Frozen Fortress", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

/// Résolution « draw n » pour le contrôleur de l'item.
fn draw_res(n: i64) -> Value {
    Value::func("draw", move |g, a| {
        let c = g.it(a[0].item()).ctrl;
        g.draw(c, n)?;
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Grove of the God-Willow
// "When you hold here, draw 1."
fn grove() -> Value {
    Value::func("_grove", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            g.queue_trigger(info.g("pid").int() as usize, "Grove of the God-Willow", draw_res(1), Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Hall of Legends
// "When you conquer here, you may pay 1 energy to ready your legend."
fn hall() -> Value {
    Value::func("_hall", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let leg = g_.p[c].legend;
                g_.ready_obj(leg, None, None)?;
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Hall of Legends", res, Map::new(), None, true, Value::None, may_pay(1, Value::None));
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Kinkou Temple
// "Units here with [Tank] have +1 might."
fn kinkou() -> Value {
    Value::func("_kinkou", |g, a| {
        let (b, o) = (a[0].int() as usize, a[1].obj());
        Ok(Value::Int(if g.o(o).loc == here_loc(b) && is_type(g, o, "Unit") && g.has_kw(o, "Tank")? { 1 } else { 0 }))
    })
}

// ---------------------------------------------------------------------------------------------- Minefield
// "When you conquer here, put the top 2 cards of your Main Deck into your trash."  (= burning, rules 440.1, 431.1.b)
fn minefield() -> Value {
    Value::func("_minefield", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                g_.burn(c, 2)?;
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Minefield", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Monastery of Hirana
// "When you conquer here, you may spend a buff to draw 1."  (spend a buff: rule 702.2.b, a unit you control;
// cost within instructions, rule 355.10.c.1)
fn monastery() -> Value {
    Value::func("_monastery", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b {
            let cost = Value::func("cost", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let us: Vec<O> = g_.units(Some(c), None).into_iter().filter(|&u| g_.o(u).buff > 0).collect();
                if us.is_empty() {
                    return Ok(false.into());
                }
                let s = sorted_by(g_, us, value)?;
                let u = g_.ask(c, "spend_buff", objs(&s).list().to_vec(), Map::new())?.opt_obj();
                let Some(u) = u else { return Ok(false.into()) };
                g_.spend_buff(Some(u), Some(c))?;
                Ok(true.into())
            });
            g.queue_trigger(info.g("pid").int() as usize, "Monastery of Hirana", draw_res(1), Map::new(), None, true, Value::None, cost);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Navori Fighting Pit
// "When you hold here, buff a unit here."
fn navori() -> Value {
    Value::func("_navori", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            let opts = Value::func("opts", move |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let mut us = vec![];
                for u in g_.units(None, here_loc(b)) {
                    if g_.targetable(u, Some(c))? {
                        us.push(u);
                    }
                }
                let s = sorted_by(g_, us, |g2, u| Ok((g2.o(u).ctrl != c, g2.o(u).buff > 0, -value(g2, u)?)))?;
                Ok(objs(&s))
            });
            let res = Value::func("res", |g_, a_| {
                if let Some(u) = g_.legal(a_[0].item(), 0)? {
                    g_.buff(Some(u), None)?;
                }
                nil()
            });
            g.queue_trigger(
                info.g("pid").int() as usize,
                "Navori Fighting Pit",
                res,
                Map::new(),
                None,
                false,
                trig_target(opts, here(b), "buff_target", false, true),
                Value::None,
            );
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Obelisk of Power
// "At the start of each player's first Beginning Phase, that player channels 1 rune."
fn obelisk() -> Value {
    Value::func("_obelisk", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "beginning_start" && g.p[info.g("pid").int() as usize].turns == 1 {
            let res = Value::func("res", |g_, a_| {
                let who = g_.it(a_[0].item()).data.g("who").int() as usize;
                g_.channel(who, 1, false)?;
                nil()
            });
            let p = bf_ctrl(g, b);
            g.queue_trigger(p, "Obelisk of Power", res, map! { who = info.g("pid") }, None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Power Nexus
// "When you hold here, you may pay 4 runes of any type to score 1 point."  (not a conquer point: no Final Point
// restriction, rule 471.1.a.1)
fn nexus() -> Value {
    Value::func("_nexus", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                g_.gain_point(c, "Power Nexus")?;
                nil()
            });
            let reqs = Value::func("reqs", |_g, _a| Ok(reqs_to_value(&vec![any_req(); 4])));
            g.queue_trigger(info.g("pid").int() as usize, "Power Nexus", res, Map::new(), None, true, Value::None, may_pay(0, reqs));
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Protective Sands
// "When you conquer here, if you control 4 or fewer runes, you may pay 1 energy to draw 1."
fn sands() -> Value {
    Value::func("_sands", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b && g.p[info.g("pid").int() as usize].runes.len() <= 4 {
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                if g_.p[c].runes.len() <= 4 {
                    // intervening "if", checked again on resolution
                    g_.draw(c, 1)?;
                }
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Protective Sands", res, Map::new(), None, true, Value::None, may_pay(1, Value::None));
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Reckoner's Arena
// "When you hold here, activate the conquer effects of units here."  Rule 383.4.g: each conquer effect of a unit
// here is checked as if the conquer part of its condition were fulfilled and, if so, put on the chain as if it had
// just triggered. Conquer effects of units = their on_event handlers for 'conquer' and [Hunt] ("When I conquer or
// hold", rule 823). Only the holding player's units can be here at that point.
fn reckoner() -> Value {
    Value::func("_reckoner", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            let res = Value::func("res", move |g_, a_| {
                let pid = g_.it(a_[0].item()).ctrl;
                let units = g_.units(Some(pid), here_loc(b));
                let inf = map! { pid = pid, bf = b, units = units.clone() };
                for &u in &units {
                    let im = g_.impl_o(u);
                    if let Some(im) = im {
                        if !im.on_event.is_none() && g_.on_board(u) {
                            im.on_event.call(g_, &[u.into(), "conquer".into(), Value::Map(inf.clone())])?;
                        }
                    }
                }
                g_.hunt(pid, &units)?;
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Reckoner's Arena", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Seat of Power
// "When you conquer here, draw 1 for each other battlefield you or allies control."  (1v1: no allies)
fn seat() -> Value {
    Value::func("_seat", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b {
            let res = Value::func("res", move |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let n = g_.bfs.iter().filter(|x| x.idx != b && x.ctrl == Some(c)).count() as i64;
                if n != 0 {
                    g_.draw(c, n)?;
                }
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Seat of Power", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Shadow Temple
// "When you hold here, [Burn 3]."
fn shadow() -> Value {
    Value::func("_shadow", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                g_.burn(c, 3)?;
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Shadow Temple", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

/// Résolution « channel 1 rune exhausted » pour le contrôleur de l'item.
fn channel_exh_res() -> Value {
    Value::func("channel", |g, a| {
        let c = g.it(a[0].item()).ctrl;
        g.channel(c, 1, true)?;
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Startipped Peak
// "When you hold here, you may channel 1 rune exhausted."
fn peak() -> Value {
    Value::func("_peak", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            g.queue_trigger(info.g("pid").int() as usize, "Startipped Peak", channel_exh_res(), Map::new(), None, true, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Sunken Temple
// "When you conquer here with one or more [Mighty] units, you may pay 1 energy to draw 1."
fn sunken() -> Value {
    Value::func("_sunken", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b {
            let mut any = false;
            for u in info.g("units").list() {
                if g.mighty(u.obj())? {
                    any = true;
                    break;
                }
            }
            if any {
                g.queue_trigger(info.g("pid").int() as usize, "Sunken Temple", draw_res(1), Map::new(), None, true, Value::None, may_pay(1, Value::None));
            }
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- The Arena's Greatest
// "At the start of each player's first Beginning Phase, that player gains 1 point."  (rule 190.6.b example)
fn arena() -> Value {
    Value::func("_arena", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "beginning_start" && g.p[info.g("pid").int() as usize].turns == 1 {
            let res = Value::func("res", |g_, a_| {
                let who = g_.it(a_[0].item()).data.g("who").int() as usize;
                g_.gain_point(who, "The Arena's Greatest")?;
                nil()
            });
            let p = bf_ctrl(g, b);
            g.queue_trigger(p, "The Arena's Greatest", res, map! { who = info.g("pid") }, None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- The Candlelit Sanctum
// "When you conquer here, look at the top two cards of your Main Deck. You may recycle one or both of them. Put
// those you don't back in any order."  (the procedure of Predict 2, rule 436; looking never burns out, 431.1.c)
fn candlelit() -> Value {
    Value::func("_candlelit", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                g_.predict(c, 2)?;
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "The Candlelit Sanctum", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- The Dreaming Tree
// "When a player chooses a friendly unit here with a spell for the first time each turn, they draw 1."
// (targeting trigger, rule 383.4.b: when the spell is finalized)
fn dreaming() -> Value {
    Value::func("_dreaming", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev != "chosen" {
            return nil();
        }
        let (it, o) = (info.g("item").item(), info.g("obj").obj());
        let ctrl = g.it(it).ctrl;
        if g.it(it).kind != "spell" || g.o(o).loc != here_loc(b) || g.o(o).ctrl != ctrl || !is_type(g, o, "Unit") {
            return nil();
        }
        // clé tuple ("dreaming_tree", b.idx, it.ctrl, g.turn_no), écrite comme son repr
        let key = format!("('dreaming_tree', {}, {}, {})", b, ctrl, g.turn_no);
        if g.stats.get(&key) != 0 {
            return nil();
        }
        stat_set(g, &key, 1);
        g.queue_trigger(ctrl, "The Dreaming Tree", draw_res(1), Map::new(), None, false, Value::None, Value::None);
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- The Grand Plaza
// "When you hold here, if you have 7+ units here, you win the game."
fn plaza() -> Value {
    Value::func("_plaza", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            let pid = info.g("pid").int() as usize;
            if g.units(Some(pid), here_loc(b)).len() >= 7 {
                let res = Value::func("res", move |g_, a_| {
                    let c = g_.it(a_[0].item()).ctrl;
                    if g_.units(Some(c), here_loc(b)).len() >= 7 {
                        g_.log(|| format!("P{c} wins with The Grand Plaza"));
                        g_.win(c)?;
                    }
                    nil()
                });
                g.queue_trigger(pid, "The Grand Plaza", res, Map::new(), None, false, Value::None, Value::None);
            }
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- The Papertree
// "When you hold here, each player channels 1 rune exhausted."
fn papertree() -> Value {
    Value::func("_papertree", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, _a| {
                let tp = g_.tp;
                for p in [tp, 1 - tp] {
                    g_.channel(p, 1, true)?;
                }
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "The Papertree", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

/// « play a Gold gear token exhausted » pour le contrôleur de l'item.
fn gold_res() -> Value {
    Value::func("gold", |g, a| {
        let c = g.it(a[0].item()).ctrl;
        make_token(g, "Gold", c, Loc::Base, false)?;
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Treasure Hoard
// "When you conquer here, you may pay 1 energy to play a Gold gear token exhausted."
fn hoard() -> Value {
    Value::func("_hoard", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b {
            g.queue_trigger(info.g("pid").int() as usize, "Treasure Hoard", gold_res(), Map::new(), None, true, Value::None, may_pay(1, Value::None));
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Trifarian War Camp
// "Units here have +1 might. (This includes attackers.)"
fn trifarian() -> Value {
    Value::func("_trifarian", |g, a| {
        let (b, o) = (a[0].int() as usize, a[1].obj());
        Ok(Value::Int(if g.o(o).loc == here_loc(b) && is_type(g, o, "Unit") { 1 } else { 0 }))
    })
}

// ---------------------------------------------------------------------------------------------- Valley of Idols
// "When a player plays a unit here, they may pay 1 energy to [Buff] it."
fn valley() -> Value {
    Value::func("_valley", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        let c = if ev == "played" { info.g("card").opt_obj() } else { None };
        if let Some(c) = c {
            if is_type(g, c, "Unit") && g.o(c).loc == here_loc(b) && g.on_board(c) {
                let res = Value::func("res", |g_, a_| {
                    let it = a_[0].item();
                    let (uid, oid) = (g_.it(it).data.g("uid").int() as u32, g_.it(it).data.g("oid").int());
                    if let Some(u) = g_.obj(uid) {
                        if g_.o(u).oid == oid {
                            g_.buff(Some(u), None)?;
                        }
                    }
                    nil()
                });
                let (uid, oid) = (g.o(c).uid, g.o(c).oid);
                g.queue_trigger(
                    info.g("pid").int() as usize,
                    "Valley of Idols",
                    res,
                    map! { uid = uid, oid = oid },
                    None,
                    true,
                    Value::None,
                    may_pay(1, Value::None),
                );
            }
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Veiled Temple
// "When you conquer here, you may ready a friendly gear. If it's an Equipment, you may detach it."
/// `_detach(g, gear)`
fn detach(g: &mut Game, gear: O) {
    let u = g.o(gear).attached_to.and_then(|x| g.obj(x));
    let guid = g.o(gear).uid;
    if let Some(u) = u {
        if let Some(i) = g.o(u).attached.iter().position(|&x| x == guid) {
            g.o_mut(u).attached.remove(i);
        }
    }
    g.o_mut(gear).attached_to = None;
    let r = g.repr(gear);
    g.log(|| format!("  {r} is detached"));
    g.need_cleanup = true; // unattached gear at a battlefield is recalled (rule 323, step 5)
}

fn veiled() -> Value {
    Value::func("_veiled", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b {
            let opts = Value::func("opts", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let gs = g_.gear(Some(c));
                let s = sorted_by(g_, gs, |g2, x| Ok((!g2.exhausted(x), -g2.spec(x).e)))?;
                Ok(objs(&s))
            });
            let res = Value::func("res", |g_, a_| {
                let it = a_[0].item();
                let Some(x) = g_.legal(it, 0)? else { return nil() };
                g_.ready_obj(x, None, None)?;
                if g_.spec(x).tags.contains("Equipment") && g_.o(x).attached_to.is_some() {
                    let c = g_.it(it).ctrl;
                    if g_.ask(c, "veiled_detach", vec![false.into(), true.into()], map! { gear = x })?.truthy() {
                        detach(g_, x);
                    }
                }
                nil()
            });
            let pred = Value::func("pred", |g_, a_| {
                let (it, o) = (a_[0].item(), a_[1].obj());
                Ok((g_.o(o).ctrl == g_.it(it).ctrl && is_type(g_, o, "Gear")).into())
            });
            g.queue_trigger(
                info.g("pid").int() as usize,
                "Veiled Temple",
                res,
                Map::new(),
                None,
                true,
                trig_target(opts, pred, "ready_gear", false, true),
                Value::None,
            );
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Zaun Warrens
// "When you conquer here, discard 1, then draw 1."
fn zaun() -> Value {
    Value::func("_zaun", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "conquer" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let h = g_.p[c].hand.clone();
                if !h.is_empty() {
                    let d = g_.ask(c, "discard", objs(&h).list().to_vec(), map! { reason = "zaun_warrens" })?;
                    g_.discard(c, d.obj())?;
                }
                g_.draw(c, 1)?;
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Zaun Warrens", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ====================================================================== battlefields using the engine hooks (integration)
fn ctrl_is(g: &Game, b: usize, pid: usize) -> bool {
    g.bfs[b].ctrl == Some(pid)
}

// Brush (battlefield token, rule 187.8) — "Bird, Cat, Dog, Poro, and Ivern units here have +1 might. When you score
// here, you may replace this with the battlefield it replaced." (created by Ivern, Green Father:
// Game.replace_battlefield / swap_back, rule 438)
const BRUSH_TAGS: [&str; 5] = ["Bird", "Cat", "Dog", "Poro", "Ivern"];

fn brush_event() -> Value {
    Value::func("_brush_event", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if (ev == "conquer" || ev == "hold") && info.g("bf").int() as usize == b && g.bfs[b].replaced.is_some() {
            let res = Value::func("res", |g_, a_| {
                let bb = g_.it(a_[0].item()).data.g("bf").int() as usize;
                if g_.bfs[bb].name == "Brush" {
                    g_.swap_back(bb)?;
                }
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Brush", res, map! { bf = b }, None, true, Value::None, Value::None);
        }
        nil()
    })
}

fn brush_might() -> Value {
    Value::func("_brush_might", |g, a| {
        let (b, o) = (a[0].int() as usize, a[1].obj());
        let ok = g.o(o).loc == here_loc(b) && is_type(g, o, "Unit") && {
            let t = g.tags(o)?;
            BRUSH_TAGS.iter().any(|x| t.contains(*x))
        };
        Ok(Value::Int(if ok { 1 } else { 0 }))
    })
}

// Marai Spire — "While you control this battlefield, friendly [Repeat] costs cost 1 energy less."
fn marai() -> Value {
    Value::func("_marai", |g, a| {
        let (b, pid, what) = (a[0].int() as usize, a[1].int() as usize, a[2].map());
        Ok(if ctrl_is(g, b, pid) && what.g("kind").opt_str() == Some("spell") {
            maps(vec![map! { e = -1, on = "rep" }])
        } else {
            Value::List(vec![])
        })
    })
}

// Mystic Vortex — "During showdowns here, cards with [Reaction] cost 1 rune of any type more to play. (Hidden cards
// have [Reaction].)"
fn vortex() -> Value {
    Value::func("_vortex", |g, a| {
        let (b, pid, what) = (a[0].int() as usize, a[1].int() as usize, a[2].map());
        let c = what.g("card").opt_obj();
        let Some(c) = c else { return Ok(Value::List(vec![])) };
        if what.g("kind").opt_str() == Some("ability") || g.sd.as_ref().is_none_or(|sd| sd.bf != b) {
            return Ok(Value::List(vec![]));
        }
        let im = g.impl_o(c);
        let srcv = what.g("src");
        let src = srcv.opt_str();
        let react = src == Some("facedown")
            || im.is_some_and(|im| im.timing == Some("reaction"))
            || card_kw(g, pid, c, "Reaction", src)?
            || (is_type(g, c, "Gear") && card_kw(g, pid, c, "Quick-Draw", src)?);
        Ok(if react { maps(vec![map! { p = reqs_to_value(&[any_req()]) }]) } else { Value::List(vec![]) })
    })
}

// Ornn's Forge — "While you control this battlefield, the first friendly non-token gear played each turn costs 1
// energy less." (Game.hist['gear_played'])
fn ornn_forge() -> Value {
    Value::func("_ornn_forge", |g, a| {
        let (b, pid, what) = (a[0].int() as usize, a[1].int() as usize, a[2].map());
        let card = what.g("card").opt_obj();
        Ok(
            if ctrl_is(g, b, pid)
                && what.g("kind").opt_str() == Some("gear")
                && card.is_some_and(|c| !g.o(c).token)
                && g.hist.gear_played[pid] == 0
            {
                maps(vec![map! { e = -1 }])
            } else {
                Value::List(vec![])
            },
        )
    })
}

// Piltovan Forge — "While you control this battlefield, the first friendly gear activated ability played each turn
// costs 1 energy less." (Game.hist['gear_abs'])
fn piltovan() -> Value {
    Value::func("_piltovan", |g, a| {
        let (b, pid, what) = (a[0].int() as usize, a[1].int() as usize, a[2].map());
        let obj = what.g("obj").opt_obj();
        Ok(
            if ctrl_is(g, b, pid)
                && what.g("kind").opt_str() == Some("ability")
                && obj.is_some_and(|o| is_type(g, o, "Gear"))
                && g.hist.gear_abs[pid] == 0
            {
                maps(vec![map! { e = -1 }])
            } else {
                Value::List(vec![])
            },
        )
    })
}

// Risen Altar — "[Empower] costs of your units here cost 1 energy or 1 rune of any type less."
fn risen() -> Value {
    Value::func("_risen", |g, a| {
        let (b, pid, what) = (a[0].int() as usize, a[1].int() as usize, a[2].map());
        let ab = what.g("ab");
        let obj = what.g("obj").opt_obj();
        let ok = what.g("kind").opt_str() == Some("ability")
            && !ab.is_none()
            && ab.map().g("name").opt_str() == Some("Empower")
            && obj.is_some_and(|o| {
                g.o(o).zone == Some(Zone::Board)
                    && is_type(g, o, "Unit")
                    && g.o(o).ctrl == pid
                    && g.o(o).loc == here_loc(b)
            });
        Ok(if ok { maps(vec![map! { flex = 1 }]) } else { Value::List(vec![]) })
    })
}

// Sandswept Tomb — "Each spell that chooses one or more units here that are friendly to it costs 1 rune of any type
// less."
fn tomb() -> Value {
    Value::func("_tomb", |g, a| {
        let (b, pid, what) = (a[0].int() as usize, a[1].int() as usize, a[2].map());
        let choice = what.g("choice");
        if what.g("kind").opt_str() != Some("spell") || choice.is_none() {
            return Ok(Value::List(vec![]));
        }
        for uid in choice.map().g("tg").list() {
            let o = match uid {
                Value::Int(_) | Value::Bool(_) => g.obj(uid.int() as u32),
                _ => None,
            };
            if let Some(o) = o {
                if is_type(g, o, "Unit") && g.o(o).loc == here_loc(b) && g.o(o).ctrl == pid {
                    return Ok(maps(vec![map! { rm = 1 }]));
                }
            }
        }
        Ok(Value::List(vec![]))
    })
}

// Vaults of Helia — "When you hold here, your non-token units cost 1 energy more to play this turn."
/// `_vaults_mod(g, eff, pid, what)`
fn vaults_mod() -> Value {
    Value::func("_vaults_mod", |g, a| {
        let what = a[2].map();
        let card = what.g("card").opt_obj();
        Ok(if what.g("kind").opt_str() == Some("unit") && card.is_some_and(|c| !g.o(c).token) {
            maps(vec![map! { e = 1 }])
        } else {
            Value::Tuple(vec![])
        })
    })
}

fn vaults() -> Value {
    Value::func("_vaults", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                g_.add_effect(map! { kind = "cost_mod", fn = vaults_mod(), pid = c, dur = "turn" });
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "Vaults of Helia", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// Rockfall Path — "Units can't be played here." (tokens included; actions.loc_allowed)
fn rockfall() -> Value {
    Value::func("_rockfall", |g, a| {
        let (b, c, loc) = (a[0].int() as usize, a[2].obj(), &a[3]);
        Ok((loc.opt_loc() == here_loc(b) && is_type(g, c, "Unit")).into())
    })
}

// Dragon Roost — "Any player may pay 2 runes of any type as an additional cost to play a Dragon. If they do, they play
// it to this battlefield." (a location with an additional cost: choice loc_reqs)
fn roost() -> Value {
    Value::func("_roost", |g, a| {
        let (b, c) = (a[0].int() as usize, a[2].obj());
        Ok(if is_type(g, c, "Unit") && g.spec(c).tags.contains("Dragon") {
            let any = reqs_to_value(&[any_req()]).list()[0].clone();
            Value::List(vec![Value::Tuple(vec![
                Value::Loc(Loc::Bf(b)),
                map! { roost = true, loc_reqs = Value::Tuple(vec![any.clone(), any]) }.into(),
            ])])
        } else {
            Value::List(vec![])
        })
    })
}

// Vilemaw's Lair — "Units can't move from here to base." (a recall is not a move)
fn vilemaw() -> Value {
    Value::func("_vilemaw", |g, a| {
        let (b, o, dest) = (a[0].int() as usize, a[1].obj(), &a[2]);
        Ok((g.o(o).loc == here_loc(b) && dest.opt_loc() == Some(Loc::Base)).into())
    })
}

// Forge of the Fluft — "While you control this battlefield, friendly legends have '[E]: Attach an Equipment you
// control to a unit you control.'"
fn fluft_choices() -> Value {
    Value::func("_fluft_choices", |g, a| {
        let pid = a[0].int() as usize;
        let eqs: Vec<O> = g.gear(Some(pid)).into_iter().filter(|&x| g.spec(x).tags.contains("Equipment")).collect();
        let eqs = sorted_by(g, eqs, |g2, x| Ok((g2.o(x).attached_to.is_some(), g2.o(x).uid)))?;
        let us = g.units(Some(pid), None);
        let us = sorted_by(g, us, |g2, u| Ok((-value(g2, u)?, g2.o(u).uid)))?;
        let mut out = vec![];
        for &x in &eqs {
            for &u in &us {
                if g.o(x).attached_to != Some(g.o(u).uid) {
                    out.push(map! { tg = Value::Tuple(vec![g.o(x).uid.into(), g.o(u).uid.into()]) });
                }
            }
        }
        Ok(maps(out))
    })
}

fn fluft_res() -> Value {
    Value::func("_fluft_res", |g, a| {
        let it = a[0].item();
        let (x, u) = (g.legal(it, 0)?, g.legal(it, 1)?);
        if let (Some(x), Some(u)) = (x, u) {
            let c = g.it(it).ctrl;
            if g.o(x).ctrl == c && g.o(u).ctrl == c {
                attach(g, x, u)?;
            }
        }
        nil()
    })
}

fn fluft_grant(ab: Map) -> Value {
    Value::func("_fluft_grant", move |g, a| {
        let (b, o) = (a[0].int() as usize, a[1].obj());
        Ok(if g.o(o).zone == Some(Zone::Legend) && g.bfs[b].ctrl == Some(g.o(o).owner) {
            Value::List(vec![ab.clone().into()])
        } else {
            Value::List(vec![])
        })
    })
}

// Gardens of Becoming — "Units here have '[E]: Gain 1 XP.'"
fn gardens_grant(ab: Map) -> Value {
    Value::func("_gardens_grant", move |g, a| {
        let (b, o) = (a[0].int() as usize, a[1].obj());
        Ok(if g.o(o).zone == Some(Zone::Board) && is_type(g, o, "Unit") && g.o(o).loc == here_loc(b) {
            Value::List(vec![ab.clone().into()])
        } else {
            Value::List(vec![])
        })
    })
}

// Altar of Blood — "If a unit here would die during combat, its controller may pay 3 runes of any type to heal it,
// exhaust it, and recall it instead."
fn altar_blood() -> Value {
    Value::func("_altar_blood", |g, a| {
        let (b, u) = (a[0].int() as usize, a[1].obj());
        let ctrl = g.o(u).ctrl;
        if g.o(u).loc != here_loc(b)
            || g.sd.as_ref().is_none_or(|sd| !sd.combat || sd.bf != b)
            || !g.can_pay(ctrl, 0, &vec![any_req(); 3], &Value::None)?
        {
            return nil();
        }
        let pid = ctrl;
        let cost = Value::func("cost", move |g_, _a| Ok(g_.pay(pid, 0, &vec![any_req(); 3], &Value::None)?.into()));
        let apply = Value::func("apply", |g_, a_| {
            g_.save_unit(a_[0].obj(), true, true);
            nil()
        });
        Ok(map! { name = "Altar of Blood", may = true, cost = cost, apply = apply }.into())
    })
}

// Ripper's Bay — "When a unit here is returned to a player's hand, that player may pay 1 energy to channel 1 rune
// exhausted."
fn ripper() -> Value {
    Value::func("_ripper", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "returned" && info.g("loc").opt_loc() == here_loc(b) && is_type(g, info.g("obj").obj(), "Unit") {
            let cost = Value::func("cost", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                Ok((g_.can_pay(c, 1, &[], &Value::None)? && g_.pay(c, 1, &[], &Value::None)?).into())
            });
            g.queue_trigger(info.g("pid").int() as usize, "Ripper's Bay", channel_exh_res(), Map::new(), None, true, Value::None, cost);
        }
        nil()
    })
}

// Forgotten Library — "While you control this battlefield, when you play a spell, if you spent 4 energy or more,
// [Predict]."
fn library() -> Value {
    Value::func("_library", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        let it = if ev == "played" { info.g("item") } else { Value::None };
        if let Value::Item(it) = it {
            let pid = info.g("pid").int() as usize;
            if Some(pid) == g.bfs[b].ctrl
                && is_type(g, info.g("card").obj(), "Spell")
                && g.it(it).data.g("paid_e").opt_int().unwrap_or(0) >= 4
            {
                let res = Value::func("res", |g_, a_| {
                    let c = g_.it(a_[0].item()).ctrl;
                    g_.predict(c, 1)?;
                    nil()
                });
                g.queue_trigger(pid, "Forgotten Library", res, Map::new(), None, false, Value::None, Value::None);
            }
        }
        nil()
    })
}

// The Academy — "When you hold here, give your next spell this turn [Repeat] equal to its base cost."
/// `_academy_cost(g, card)` : (e, reqs)
fn academy_cost() -> Value {
    Value::func("_academy_cost", |g, a| {
        let c = a[0].obj();
        let sp = g.spec(c);
        let reqs = g.power_reqs(&sp.domains, sp.p as usize);
        Ok(Value::Tuple(vec![Value::Int(sp.e as i64), reqs_to_value(&reqs)]))
    })
}

fn academy() -> Value {
    Value::func("_academy", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "hold" && info.g("bf").int() as usize == b {
            let res = Value::func("res", |g_, a_| {
                let it = a_[0].item();
                let (c, id) = (g_.it(it).ctrl, g_.it(it).id);
                g_.add_effect(map! {
                    kind = "grant_repeat", pid = c, cost = academy_cost(), next = true, key = format!("academy-{id}"),
                    dur = "turn",
                });
                nil()
            });
            g.queue_trigger(info.g("pid").int() as usize, "The Academy", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- enregistrement
fn hooks(kv: Vec<(&str, Value)>) -> Map {
    let mut m = Map::new();
    for (k, v) in kv {
        m.set(k, v);
    }
    m
}

pub fn register(r: &mut Registry) {
    card(r, Impl { bf_event: abandoned_hall(), ..Impl::new("Abandoned Hall") });
    card(r, Impl { bf_event: altar_unity(), ..Impl::new("Altar to Unity") });
    card(r, Impl { bf_event: recital(), ..Impl::new("Amateur Recital") });
    card(r, Impl { aura_kw: bfa_aura(), ..Impl::new("Black Flame Altar") });
    card(r, Impl { bf_event: dais(), ..Impl::new("Emperor's Dais") });
    card(r, Impl { bf_event: fortified(), ..Impl::new("Fortified Position") });
    card(r, Impl { bf_event: frozen(), ..Impl::new("Frozen Fortress") });
    card(r, Impl { bf_event: grove(), ..Impl::new("Grove of the God-Willow") });
    card(r, Impl { bf_event: hall(), ..Impl::new("Hall of Legends") });
    card(r, Impl { bf_event: hallowed(), ..Impl::new("Hallowed Tomb") });
    card(r, Impl { aura_might: kinkou(), ..Impl::new("Kinkou Temple") });
    card(r, Impl { bf_event: minefield(), ..Impl::new("Minefield") });
    card(r, Impl { bf_event: monastery(), ..Impl::new("Monastery of Hirana") });
    card(r, Impl { bf_event: navori(), ..Impl::new("Navori Fighting Pit") });
    card(r, Impl { bf_event: obelisk(), ..Impl::new("Obelisk of Power") });
    card(r, Impl { bf_event: nexus(), ..Impl::new("Power Nexus") });
    card(r, Impl { bf_event: sands(), ..Impl::new("Protective Sands") });
    card(r, Impl { bf_event: ravenbloom(), ..Impl::new("Ravenbloom Conservatory") });
    card(r, Impl { bf_event: reaver(), ..Impl::new("Reaver's Row") });
    card(r, Impl { bf_event: reckoner(), ..Impl::new("Reckoner's Arena") });
    card(r, Impl { bf_event: seat(), ..Impl::new("Seat of Power") });
    card(r, Impl { bf_event: shadow(), ..Impl::new("Shadow Temple") });
    card(r, Impl { bf_event: peak(), ..Impl::new("Startipped Peak") });
    card(r, Impl { bf_event: sunken(), ..Impl::new("Sunken Temple") });
    card(r, Impl { bf_event: arena(), ..Impl::new("The Arena's Greatest") });
    card(r, Impl { bf_event: candlelit(), ..Impl::new("The Candlelit Sanctum") });
    card(r, Impl { bf_event: dreaming(), ..Impl::new("The Dreaming Tree") });
    card(r, Impl { bf_event: plaza(), ..Impl::new("The Grand Plaza") });
    card(r, Impl { bf_event: papertree(), ..Impl::new("The Papertree") });
    card(r, Impl { bf_event: trapping(), ..Impl::new("Trapping Grounds") });
    card(r, Impl { bf_event: hoard(), ..Impl::new("Treasure Hoard") });
    card(r, Impl { aura_might: trifarian(), ..Impl::new("Trifarian War Camp") });
    card(r, Impl { bf_event: valley(), ..Impl::new("Valley of Idols") });
    card(r, Impl { bf_event: veiled(), ..Impl::new("Veiled Temple") });
    card(r, Impl { bf_event: zaun(), ..Impl::new("Zaun Warrens") });

    // battlefields using the engine hooks (integration)
    card(r, Impl { bf_event: brush_event(), aura_might: brush_might(), ..Impl::new("Brush") });
    card(r, Impl { hooks: hooks(vec![("cost_aura", marai())]), ..Impl::new("Marai Spire") });
    card(r, Impl { hooks: hooks(vec![("cost_aura", vortex())]), ..Impl::new("Mystic Vortex") });
    card(r, Impl { hooks: hooks(vec![("cost_aura", ornn_forge())]), ..Impl::new("Ornn's Forge") });
    card(r, Impl { hooks: hooks(vec![("cost_aura", piltovan())]), ..Impl::new("Piltovan Forge") });
    card(r, Impl { hooks: hooks(vec![("cost_aura", risen())]), ..Impl::new("Risen Altar") });
    card(r, Impl { hooks: hooks(vec![("cost_aura", tomb())]), ..Impl::new("Sandswept Tomb") });
    card(r, Impl { bf_event: vaults(), ..Impl::new("Vaults of Helia") });
    // Heisho, Shell of the World — "Players ignore [Deflect] while paying for spells and abilities choosing something
    // here." (actions.deflect_reqs skips the objects at this battlefield)
    card(r, Impl::new("Heisho, Shell of the World"));
    card(r, Impl { hooks: hooks(vec![("loc_veto", rockfall())]), ..Impl::new("Rockfall Path") });
    card(r, Impl { hooks: hooks(vec![("aura_locs", roost())]), ..Impl::new("Dragon Roost") });
    card(r, Impl { hooks: hooks(vec![("aura_cant_move", vilemaw())]), ..Impl::new("Vilemaw's Lair") });
    let fluft_ab = ability(
        "Attach",
        AbilityArgs {
            cost: Cost::None,
            exhaust: true,
            choices: fluft_choices(),
            preds: Value::List(vec![p_gear_v(), p_friend_v()]),
            resolve: fluft_res(),
            ..Default::default()
        },
    );
    card(r, Impl { hooks: hooks(vec![("grant_abilities", fluft_grant(fluft_ab))]), ..Impl::new("Forge of the Fluft") });
    let gardens_ab = ability(
        "Gain 1 XP",
        AbilityArgs {
            cost: Cost::None,
            exhaust: true,
            resolve: Value::func("gain_xp", |g, a| {
                let c = g.it(a[0].item()).ctrl;
                g.gain_xp(c, 1)?;
                nil()
            }),
            ..Default::default()
        },
    );
    card(r, Impl { hooks: hooks(vec![("grant_abilities", gardens_grant(gardens_ab))]), ..Impl::new("Gardens of Becoming") });
    card(r, Impl { hooks: hooks(vec![("death_rep", altar_blood())]), ..Impl::new("Altar of Blood") });
    card(r, Impl { bf_event: ripper(), ..Impl::new("Ripper's Bay") });
    card(r, Impl { bf_event: library(), ..Impl::new("Forgotten Library") });
    card(r, Impl { bf_event: academy(), ..Impl::new("The Academy") });
    // Bandle Tree — "You may hide an additional card here."
    card(r, Impl { hooks: hooks(vec![("hide_slots", Value::Int(1))]), ..Impl::new("Bandle Tree") });
    // Baron Pit (battlefield token, rule 187.9) — "(You can't start the game with a token battlefield.) Units can move
    // here from anywhere." Added to the board by Baron Nashor (Game.add_battlefield): a third battlefield, which the
    // Final Point then requires too (rule 471.1.b.1). "From anywhere" is read in the Standard Move
    // (actions.move_options): a unit at another battlefield may move here without [Ganking]. The deck editor refuses it
    // as a starting battlefield.
    card(r, Impl::new("Baron Pit"));
}

// ====================================================================== cartes à état (attributs libres `attrs`)
/// `_first_defend(g, b, info)` : vrai la première fois qu'une unité d'un joueur défend à b dans le combat en cours
/// (règle 383.4.f.2.a). `sd._bf_defend_seen` : liste de (b, joueur) dans `sd.attrs`.
fn first_defend(g: &mut Game, b: usize, info: &Map) -> bool {
    let o = info.g("obj").obj();
    if info.g("bf").int() as usize != b || g.sd.is_none() {
        return false;
    }
    let key = Value::Tuple(vec![Value::from(b), Value::from(g.o(o).ctrl)]);
    let sd = g.sd.as_mut().unwrap();
    if !sd.attrs.contains("_bf_defend_seen") {
        sd.attrs.set("_bf_defend_seen", Value::List(vec![]));
    }
    let seen = sd.attrs.get_mut("_bf_defend_seen").unwrap().list_mut();
    if seen.contains(&key) {
        return false;
    }
    seen.push(key);
    true
}

// ---------------------------------------------------------------------------------------------- Fortified Position
// "When you defend here, choose a unit. It gains [Shield 2] this combat."
// Écart de forme (sans effet sur le jeu) : Python retire *ce* grant par identité (`k is eff["grant"]`) ; ici la durée
// du grant est un dict {dur: "turn", id: eid de l'effet de fin}, retiré par égalité. Ce dict n'étant pas "turn", la
// fin de tour ne l'enlève pas : la fin du combat (toujours dans le tour) l'enlève, et un changement de zone vide les
// grants.
fn fortified() -> Value {
    Value::func("_fortified", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "defend" && first_defend(g, b, info) {
            let pid = g.o(info.g("obj").obj()).ctrl;
            let opts = Value::func("opts", move |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let mine: Vec<O> =
                    friends(g_, c, false, None)?.into_iter().filter(|&u| g_.o(u).loc == here_loc(b)).collect();
                let rest: Vec<O> = all_units(g_, c, false, None)?.into_iter().filter(|u| !mine.contains(u)).collect();
                let rest = sorted_by(g_, rest, |g2, u| Ok(g2.o(u).ctrl != c))?;
                let mut out = mine;
                out.extend(rest);
                Ok(objs(&out))
            });
            let res = Value::func("res", |g_, a_| {
                let Some(u) = g_.legal(a_[0].item(), 0)? else { return nil() };
                let id = g_.next_eid + 1; // eid que recevra l'effet de fin ajouté juste après
                let mark: Value = map! { dur = "turn", id = id as i64 }.into();
                g_.o_mut(u).grants.push(("Shield".to_string(), 2, mark.clone()));
                let end = Value::func("end", |g2, a2| {
                    // "this combat": the grant ends with the combat
                    let eid = a2[0].int() as u64;
                    let eff = g2.eff(eid).cloned().unwrap_or_default();
                    if g2.has_effect(eid) {
                        g2.remove_effect(eid);
                    }
                    if let Some(x) = g2.obj(eff.g("uid").int() as u32) {
                        let gr = eff.g("grant");
                        if let Some(i) = g2.o(x).grants.iter().position(|k| k.2 == gr) {
                            g2.o_mut(x).grants.remove(i);
                        }
                    }
                    nil()
                });
                let uid = g_.o(u).uid;
                let eid = g_.add_effect(map! { on = "combat_end", fn = end, dur = "turn", uid = uid, grant = mark });
                debug_assert_eq!(eid, id);
                nil()
            });
            g.queue_trigger(
                pid,
                "Fortified Position",
                res,
                Map::new(),
                None,
                false,
                trig_target(opts, Value::None, "fortified_shield", false, true),
                Value::None,
            );
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Hallowed Tomb
// "When you hold here, you may return your Chosen Champion from your trash to your Champion Zone if it is empty."
/// `_champ_uid(g, pid)` : uid de la carte Chosen Champion de pid. Notée au premier évènement vu par ce battlefield :
/// le premier évènement d'une partie (Beginning Phase du tour 1) précède tout jeu, le champion est encore dans sa zone.
fn champ_uid(g: &Game, pid: usize) -> Option<u32> {
    g.p[pid].attrs.g("bf_chosen_champion").opt_int().map(|x| x as u32)
}

fn hallowed_find(g: &Game, p: usize) -> Option<O> {
    let uid = champ_uid(g, p)?;
    g.p[p].trash.iter().copied().find(|&c| g.o(c).uid == uid)
}

fn hallowed() -> Value {
    Value::func("_hallowed", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        for p in 0..g.p.len() {
            if !g.p[p].attrs.contains("bf_chosen_champion") {
                let v: Value = g.p[p].champ.first().map(|&c| g.o(c).uid).into();
                g.p[p].attrs.set("bf_chosen_champion", v);
            }
        }
        if ev == "hold" && info.g("bf").int() as usize == b {
            let pid = info.g("pid").int() as usize;
            let res = Value::func("res", |g_, a_| {
                let c0 = g_.it(a_[0].item()).ctrl;
                let Some(c) = hallowed_find(g_, c0) else { return nil() };
                if !g_.p[c0].champ.is_empty() {
                    return nil();
                }
                if let Some(i) = g_.p[c0].trash.iter().position(|&x| x == c) {
                    g_.p[c0].trash.remove(i);
                }
                g_.reset_obj(c);
                g_.o_mut(c).oid += 1;
                g_.o_mut(c).zone = Some(Zone::Champ);
                g_.p[c0].champ.push(c);
                let r = g_.repr(c);
                g_.log(|| format!("  Hallowed Tomb returns {r} to the Champion Zone"));
                nil()
            });
            if hallowed_find(g, pid).is_some() && g.p[pid].champ.is_empty() {
                g.queue_trigger(pid, "Hallowed Tomb", res, Map::new(), None, true, Value::None, Value::None);
            }
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Ravenbloom Conservatory
// "When you defend here, reveal the top card of your Main Deck. If it's a spell, put it in your hand. Otherwise,
// recycle it."  (empty deck: nothing is revealed, no burn out, rule 431.1.c)
fn ravenbloom() -> Value {
    Value::func("_ravenbloom", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "defend" && first_defend(g, b, info) {
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let top = g_.reveal(c, 1, None, true, None)?;
                if top.is_empty() {
                    return nil();
                }
                let x = top[0];
                if is_type(g_, x, "Spell") {
                    g_.to_zone(x, "hand", false, None)?;
                } else {
                    g_.recycle_cards(c, &[x])?;
                }
                nil()
            });
            let p = g.o(info.g("obj").obj()).ctrl;
            g.queue_trigger(p, "Ravenbloom Conservatory", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Reaver's Row
// "When you defend here, you may move a friendly unit here to base."
fn reaver() -> Value {
    Value::func("_reaver", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        if ev == "defend" && first_defend(g, b, info) {
            let res = Value::func("res", |g_, a_| {
                let it = a_[0].item();
                if let Some(u) = g_.legal(it, 0)? {
                    let c = g_.it(it).ctrl;
                    g_.move_(&[u], Loc::Base, Some(c), false)?;
                }
                nil()
            });
            let opts = Value::func("opts", move |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let f = friends(g_, c, false, here_loc(b))?;
                Ok(objs(&sorted_by(g_, f, value)?))
            });
            let pred = Value::func("pred", move |g_, a_| {
                let (it, o) = (a_[0].item(), a_[1].obj());
                Ok((g_.o(o).ctrl == g_.it(it).ctrl && g_.o(o).loc == here_loc(b)).into())
            });
            let p = g.o(info.g("obj").obj()).ctrl;
            g.queue_trigger(p, "Reaver's Row", res, Map::new(), None, true, trig_target(opts, pred, "reaver_move", false, true), Value::None);
        }
        nil()
    })
}

// ---------------------------------------------------------------------------------------------- Trapping Grounds
// "When you conquer here, if you assigned 3 or more excess damage, play a 1 might Bird unit token with [Deflect]."
// Excess damage = combat damage assigned to a unit beyond its lethal damage (Game.lethal_need). It is measured on
// the 'damaged' events of the combat here: assigned - lethal = dealt - max(1, might - previous damage).
// The token names no location: it is played to a location it can be played to (rule 187 / play rules): base or a
// battlefield its controller controls.
// `sd._bf_excess` : dict joueur -> excès dans `sd.attrs` (clé : texte du joueur, "None" sans joueur).
fn trapping() -> Value {
    Value::func("_trapping", |g, a| {
        let (b, ev, info) = (a[0].int() as usize, a[1].str(), a[2].map());
        let sd_here = g.sd.as_ref().map(|sd| sd.bf == b);
        if ev == "damaged" && info.g("kind").opt_str() == Some("combat") && sd_here == Some(true) {
            let (o, n) = (info.g("obj").obj(), info.g("amount").int());
            let prior = g.o(o).damage - n;
            let ex = n - 1.max(g.might(o)? - prior);
            if ex > 0 {
                let by = match info.g("by") {
                    Value::None => "None".to_string(),
                    v => v.int().to_string(),
                };
                let sd = g.sd.as_mut().unwrap();
                if !sd.attrs.contains("_bf_excess") {
                    sd.attrs.set("_bf_excess", Map::new());
                }
                let d = sd.attrs.get_mut("_bf_excess").unwrap().map_mut();
                let cur = d.g(&by).opt_int().unwrap_or(0);
                d.set(&by, cur + ex);
            }
        } else if ev == "conquer" && info.g("bf").int() as usize == b {
            let pid = info.g("pid").int() as usize;
            let got = match g.sd.as_ref() {
                Some(sd) if sd.bf == b => {
                    let d = sd.attrs.g("_bf_excess");
                    if d.is_none() {
                        0
                    } else {
                        d.map().g(&pid.to_string()).opt_int().unwrap_or(0)
                    }
                }
                _ => return nil(),
            };
            if got < 3 {
                return nil();
            }
            let res = Value::func("res", |g_, a_| {
                let c = g_.it(a_[0].item()).ctrl;
                let mut locs = vec![Value::Loc(Loc::Base)];
                for x in &g_.bfs {
                    if x.ctrl == Some(c) {
                        locs.push(Value::Loc(Loc::Bf(x.idx)));
                    }
                }
                let loc = g_.ask(c, "play_location", locs, Map::new())?.loc();
                make_token(g_, "Bird", c, loc, false)?;
                nil()
            });
            g.queue_trigger(pid, "Trapping Grounds", res, Map::new(), None, false, Value::None, Value::None);
        }
        nil()
    })
}
