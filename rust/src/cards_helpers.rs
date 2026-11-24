//! Aides génériques de `cards.py` (tout sauf les cartes du pool) : valeur des unités, cibles, prédicats, coûts
//! (`parse_cost`...), capacités (`ability`, `equip_ability_cost`, `empower_ability`), jetons, Disarm, Deflect,
//! contrôle des decks et cartes faites seulement de mots-clés (`register_auto`).
//!
//! Conventions propres à ce module (en plus de `docs/memoire/portage-rust.md`) :
//! - un prédicat de cible Python `P_x(g, it, o)` existe en fonction Rust `p_x(g, it, o)` et en valeur `p_x_v()`
//!   (`Value::func`, arguments `[Item, Obj]`, comme `Game::legal` l'appelle) ;
//! - un coût d'`ability()` s'écrit `Cost::Text("1 energy and 1 calm rune")`, `Cost::Parts(e, reqs)` ou
//!   `Cost::Fn(f)` (f(g, pid, obj, choice) -> `(e, reqs)`) ;
//! - les `Req` rangés dans une valeur sont des listes de domaines (`game_costs::reqs_to_value` / `reqs_of`).
use std::collections::BTreeSet;

use crate::actions::{
    as_uid, attach, card_kw, cost_value, deflect_reqs, dict_with, full_choices, keyword_play_triggers,
    loc_allowed, others_enter_ready, pay_ctx, play_card, play_forbidden, py_str, reqs_tuple, total_cost,
    unit_locations_extra, loc_of,
};
use crate::cards::{registry, Impl, Registry};
use crate::game::{Game, ItemId, Res, O};
use crate::game_board::loc_str;
use crate::game_costs::{any_req, reqs_of, Req};
use crate::lang::tr;
use crate::spec::{spec, specs, DOMAINS};
use crate::value::{is_bf, Loc, Map, OptV, Value};

/// Coût d'une capacité (`cost=` de `ability()` / `equip=` / `empower=` d'`Impl`).
#[derive(Clone, Debug)]
pub enum Cost {
    /// pas de coût (`None` ou '')
    None,
    /// texte imprimé : '1 energy and 1 calm rune'
    Text(String),
    /// tuple (énergie, reqs)
    Parts(i64, Vec<Req>),
    /// fonction f(g, pid, obj, choice) -> (e, reqs)
    Fn(Value),
}

impl From<&str> for Cost {
    fn from(s: &str) -> Self {
        Cost::Text(s.to_string())
    }
}

impl Cost {
    /// la fonction de coût f(g, pid, obj, choice) -> (e, list(reqs))
    fn func(self) -> Value {
        let (e, reqs) = match self {
            Cost::Fn(f) => return f,
            Cost::None => (0, vec![]),
            Cost::Text(t) => parse_cost(&t),
            Cost::Parts(e, r) => (e, r),
        };
        Value::func("cost", move |_g, _a| Ok(cost_value(e, &reqs)))
    }
}

/// Arguments d'`ability(name, ...)` (valeurs par défaut de Python : timing 'main', rien d'autre).
#[derive(Clone, Debug)]
pub struct AbilityArgs {
    pub cost: Cost,
    pub timing: &'static str,
    pub exhaust: bool,
    pub xp: i64,
    pub kill_self: bool,
    /// resolve(g, item) ; None : ne fait rien
    pub resolve: Value,
    /// choices(g, pid, obj) -> [choice]
    pub choices: Value,
    /// liste de prédicats, ou None
    pub preds: Value,
    /// can(g, pid, obj) -> bool
    pub can: Value,
    /// extra_cost(g, pid, obj, choice) : coûts non-ressources (peut renvoyer le choix modifié, voir `actions`)
    pub extra_cost: Value,
    /// les autres clés (`**extra` de Python : printed_e, ignore_deflect, all_choices, can_extra...)
    pub extra: Map,
}

impl Default for AbilityArgs {
    fn default() -> Self {
        AbilityArgs {
            cost: Cost::None,
            timing: "main",
            exhaust: false,
            xp: 0,
            kill_self: false,
            resolve: Value::None,
            choices: Value::None,
            preds: Value::None,
            can: Value::None,
            extra_cost: Value::None,
            extra: Map::new(),
        }
    }
}

/// Champs d'`Impl` rendus par `parse_keywords` (dict Python `fields`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KwFields {
    pub timing: Option<&'static str>,
    pub accelerate: bool,
    pub hidden: bool,
    pub ambush: bool,
    pub quickdraw: bool,
    /// None : pas de clé 'kw' (aucun mot-clé statique)
    pub kw: Option<Map>,
}

/// Paquet donné à `deck_problems` : un dict de deck, ou une liste de noms.
#[derive(Clone, Debug)]
pub enum DeckArg {
    Dict { legend: Option<String>, champion: Option<String>, main: Vec<String>, runes: Vec<String>, battlefields: Vec<String> },
    Names(Vec<String>),
}

// ---------------------------------------------------------------------------------------------- valeurs, cibles
/// `value(g, o)` : valeur grossière d'une unité, pour ordonner les choix de l'IA.
pub fn value(g: &mut Game, o: O) -> Res<f64> {
    let sp = g.spec(o);
    if sp.type_ != "Unit" {
        return Ok(2.0 + sp.e as f64 / 2.0);
    }
    let mut v = g.might(o)? as f64 + (sp.e + 2 * sp.p) as f64 / 3.0;
    if g.cname(o) == "Karthus, Eternal" {
        v += 4.0;
    }
    if g.o(o).token && g.cname(o) == "Reflection" {
        v -= 1.0;
    }
    Ok(v)
}

/// Tri stable par -value (clé calculée une fois par élément, dans l'ordre, comme `sorted`).
fn by_value(g: &mut Game, us: Vec<O>, pid: Option<usize>) -> Res<Vec<O>> {
    let mut keyed = vec![];
    for u in us {
        let first = pid.is_some_and(|p| g.o(u).ctrl == p);
        keyed.push((first, -value(g, u)?, u));
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)));
    Ok(keyed.into_iter().map(|k| k.2).collect())
}

fn filter_where(g: &Game, us: Vec<O>, at_bf: bool, hidden_bf: Option<Loc>) -> Vec<O> {
    let mut us: Vec<O> = us.into_iter().filter(|&u| !at_bf || is_bf(g.o(u).loc)).collect();
    if let Some(hb) = hidden_bf {
        us.retain(|&u| g.o(u).loc == Some(hb));
    }
    us
}

/// `enemies(g, pid, at_bf, hidden_bf, targetable)`
pub fn enemies(g: &mut Game, pid: usize, at_bf: bool, hidden_bf: Option<Loc>, targetable: bool) -> Res<Vec<O>> {
    let mut us = filter_where(g, g.units(Some(1 - pid), None), at_bf, hidden_bf);
    if targetable {
        let mut t = vec![];
        for u in us {
            if g.targetable(u, Some(pid))? {
                t.push(u);
            }
        }
        us = t;
    }
    by_value(g, us, None)
}

/// `friends(g, pid, at_bf, hidden_bf)`
pub fn friends(g: &mut Game, pid: usize, at_bf: bool, hidden_bf: Option<Loc>) -> Res<Vec<O>> {
    let us = filter_where(g, g.units(Some(pid), None), at_bf, hidden_bf);
    by_value(g, us, None)
}

/// `all_units(g, pid, at_bf, hidden_bf)` : ennemies d'abord, puis par valeur.
pub fn all_units(g: &mut Game, pid: usize, at_bf: bool, hidden_bf: Option<Loc>) -> Res<Vec<O>> {
    let us = filter_where(g, g.units(None, None), at_bf, hidden_bf);
    let mut t = vec![];
    for u in us {
        if g.targetable(u, Some(pid))? {
            t.push(u);
        }
    }
    by_value(g, t, Some(pid))
}

/// `hb_ok(it, o)`
pub fn hb_ok(g: &Game, it: ItemId, o: O) -> bool {
    match loc_of(&g.it(it).data.g("hidden_bf")) {
        None => true,
        hb => g.o(o).loc == hb,
    }
}

pub fn p_unit(g: &Game, it: ItemId, o: O) -> bool {
    g.spec(o).type_ == "Unit" && hb_ok(g, it, o)
}
pub fn p_unit_bf(g: &Game, it: ItemId, o: O) -> bool {
    g.spec(o).type_ == "Unit" && is_bf(g.o(o).loc) && hb_ok(g, it, o)
}
pub fn p_enemy(g: &Game, it: ItemId, o: O) -> bool {
    g.spec(o).type_ == "Unit" && g.o(o).ctrl != g.it(it).ctrl && hb_ok(g, it, o)
}
pub fn p_enemy_bf(g: &Game, it: ItemId, o: O) -> bool {
    p_enemy(g, it, o) && is_bf(g.o(o).loc)
}
pub fn p_friend(g: &Game, it: ItemId, o: O) -> bool {
    g.spec(o).type_ == "Unit" && g.o(o).ctrl == g.it(it).ctrl && hb_ok(g, it, o)
}
pub fn p_friend_bf(g: &Game, it: ItemId, o: O) -> bool {
    p_friend(g, it, o) && is_bf(g.o(o).loc)
}
pub fn p_gear(g: &Game, _it: ItemId, o: O) -> bool {
    g.is_gear(o)
}
pub fn p_unit_v() -> Value {
    Value::func("P_unit", |g, a| Ok(p_unit(g, a[0].item(), a[1].obj()).into()))
}
pub fn p_unit_bf_v() -> Value {
    Value::func("P_unit_bf", |g, a| Ok(p_unit_bf(g, a[0].item(), a[1].obj()).into()))
}
pub fn p_enemy_v() -> Value {
    Value::func("P_enemy", |g, a| Ok(p_enemy(g, a[0].item(), a[1].obj()).into()))
}
pub fn p_enemy_bf_v() -> Value {
    Value::func("P_enemy_bf", |g, a| Ok(p_enemy_bf(g, a[0].item(), a[1].obj()).into()))
}
pub fn p_friend_v() -> Value {
    Value::func("P_friend", |g, a| Ok(p_friend(g, a[0].item(), a[1].obj()).into()))
}
pub fn p_friend_bf_v() -> Value {
    Value::func("P_friend_bf", |g, a| Ok(p_friend_bf(g, a[0].item(), a[1].obj()).into()))
}
pub fn p_gear_v() -> Value {
    Value::func("P_gear", |g, a| Ok(p_gear(g, a[0].item(), a[1].obj()).into()))
}

/// `tg_choices(units)` : [dict(tg=(uid,))]
pub fn tg_choices(g: &Game, units: &[O]) -> Vec<Map> {
    units.iter().map(|&u| crate::map! { tg = Value::Tuple(vec![g.o(u).uid.into()]) }).collect()
}

/// `cap(g, seq, n, pid)` : les n premiers pour l'IA, tout pour un humain.
pub fn cap<T>(g: &Game, seq: Vec<T>, n: usize, pid: Option<usize>) -> Vec<T> {
    if full_choices(g, pid) {
        seq
    } else {
        seq.into_iter().take(n).collect()
    }
}

/// `item_by_id(g, iid)` : l'item de la chaîne de cet id.
pub fn item_by_id(g: &Game, iid: i64) -> Option<ItemId> {
    g.chain.iter().copied().find(|&it| g.it(it).id as i64 == iid)
}

/// `trig_target(options_fn, pred, kind, optional, deflect)` : choix de la cible d'une capacité déclenchée, à la
/// finalisation. options_fn(g, it) -> [obj], pred(g, it, o) ou None. Les cibles ennemies avec [Deflect] se paient
/// (809.1.c), les impayables ne sont pas proposées. Renvoie choose(g, it) -> bool, avec l'attribut `options`
/// (flush_triggers : pas de question « may » sans cible). `optional` n'est pas lu (comme en Python).
pub fn trig_target(options_fn: Value, pred: Value, kind: &'static str, _optional: bool, deflect: bool) -> Value {
    let p2 = pred.clone();
    let opts_of = Value::func("opts_of", move |g, a| {
        let it = a[0].item();
        let ctrl = g.it(it).ctrl;
        let mut opts: Vec<O> = options_fn.call(g, a)?.list().iter().map(|v| v.obj()).collect();
        if !p2.is_none() && full_choices(g, Some(ctrl)) {
            // un humain peut choisir n'importe quelle cible légale
            let mut more = vec![];
            for o in g.board.clone() {
                if !opts.contains(&o) && g.targetable(o, Some(ctrl))? && p2.call(g, &[it.into(), o.into()])?.truthy() {
                    more.push(o);
                }
            }
            more.sort_by_key(|&o| (g.o(o).ctrl == ctrl, g.o(o).uid));
            opts.extend(more);
        }
        if deflect {
            let mut keep = vec![];
            for o in opts {
                let reqs = deflect_reqs(g, ctrl, &crate::map! { tg = Value::Tuple(vec![g.o(o).uid.into()]) })?;
                if g.can_pay(ctrl, 0, &reqs, &Value::None)? {
                    keep.push(o);
                }
            }
            opts = keep;
        }
        Ok(opts.into())
    });
    let of = opts_of.clone();
    Value::func_attrs("choose", crate::map! { options = opts_of }, move |g, a| {
        let it = a[0].item();
        let opts = of.call(g, a)?;
        if opts.list().is_empty() {
            return Ok(false.into());
        }
        let ctrl = g.it(it).ctrl;
        let o = g.ask(ctrl, kind, opts.list().to_vec(), crate::map! { item = it })?;
        let Some(o) = o.opt_obj() else { return Ok(false.into()) };
        if deflect && !pay_deflect(g, it, o)? {
            return Ok(false.into());
        }
        g.add_target(it, o, pred.clone());
        Ok(true.into())
    })
}

/// `may_pay(e, reqs_fn)` : coût optionnel cost(g, it) avec les attributs can, label, label_en.
pub fn may_pay(e: i64, reqs_fn: Value) -> Value {
    let rf = reqs_fn.clone();
    let reqs_of_it = move |g: &mut Game, it: &Value, rf: &Value| -> Res<Vec<Req>> {
        Ok(if rf.is_none() { vec![] } else { reqs_of(&rf.call(g, std::slice::from_ref(it))?) })
    };
    let rf2 = rf.clone();
    let can = Value::func("can", move |g, a| {
        let ctrl = g.it(a[0].item()).ctrl;
        let reqs = reqs_of_it(g, &a[0], &rf2)?;
        Ok(g.can_pay(ctrl, e, &reqs, &Value::None)?.into())
    });
    let has = !reqs_fn.is_none();
    let attrs = crate::map! {
        can = can,
        label = format!("{e} énergie{}", if has { " et des runes" } else { "" }),
        label_en = format!("{e} energy{}", if has { " and runes" } else { "" }),
    };
    Value::func_attrs("cost", attrs, move |g, a| {
        let ctrl = g.it(a[0].item()).ctrl;
        let reqs = reqs_of_it(g, &a[0], &rf)?;
        if !g.can_pay(ctrl, e, &reqs, &Value::None)? {
            return Ok(false.into());
        }
        Ok(g.pay(ctrl, e, &reqs, &Value::None)?.into())
    })
}

/// `extra_label(g, ex)` : libellé d'un choix « as you play me » / coût additionnel (Impl.as_played), pour un humain.
pub fn extra_label(g: &Game, ex: &Map) -> String {
    if ex.0.is_empty() {
        return tr("sans coût additionnel", "no additional cost");
    }
    let mut keys: Vec<&str> = ex.keys().collect();
    keys.sort();
    let mut parts = vec![];
    for k in keys {
        let v = ex.g(k);
        let o = match v {
            Value::Int(u) => g.obj(u as u32),
            _ => None,
        };
        if k == "acc" {
            parts.push(if v.truthy() { tr("payer [Accelerate]", "pay [Accelerate]") } else { tr("sans [Accelerate]", "without [Accelerate]") });
        } else if k == "kill" && o.is_some() {
            let n = g.cname(o.unwrap());
            parts.push(tr(format!("tuer {n}"), format!("kill {n}")));
        } else if k == "kills" {
            parts.push(if v.truthy() {
                let names: Vec<&str> = v.list().iter().filter_map(|u| as_uid(u).and_then(|x| g.obj(x))).map(|x| g.cname(x)).collect();
                tr("tuer ", "kill ") + &names.join(", ")
            } else {
                tr("ne rien tuer", "kill nothing")
            });
        } else if k == "ret_gear" && o.is_some() {
            let n = g.cname(o.unwrap());
            parts.push(tr(format!("renvoyer {n} en main"), format!("return {n} to hand")));
        } else if k == "tag" {
            parts.push(format!("tag {}", py_str(g, &v)));
        } else if k == "xp" {
            parts.push(tr("dépenser de l'XP", "spend XP"));
        } else if let Some(o) = o {
            let n = g.cname(o);
            parts.push(tr(format!("{k} : {n}"), format!("{k}: {n}")));
        } else if v == Value::Bool(true) && matches!(v, Value::Bool(_)) {
            parts.push(tr("payer le coût additionnel", "pay the additional cost"));
        } else {
            parts.push(format!("{k} = {}", py_str(g, &v)));
        }
    }
    parts.join(", ")
}

/// `play_unit_free(g, pid, c, src, loc_choices, ignore_energy, alt_cost)` : jouer une unité par un effet, sans son
/// coût (ou sans son coût en énergie). Les restrictions de jeu s'appliquent ; les coûts additionnels sont choisis et
/// payés. alt_cost : jouée pour ce coût à la place du sien (356.1.a). Renvoie la carte jouée ou None.
pub fn play_unit_free(
    g: &mut Game,
    pid: usize,
    c: O,
    src: &str,
    loc_choices: Option<Vec<Loc>>,
    ignore_energy: bool,
    alt_cost: Option<(i64, Vec<Req>)>,
) -> Res<Option<O>> {
    if play_forbidden(g, pid, c, src)? {
        return Ok(None);
    }
    let im = g.impl_o(c);
    let mut extras: Vec<Map> = match im {
        Some(im) if im.as_played.truthy() => {
            im.as_played.call(g, &[pid.into(), c.into()])?.list().iter().map(|x| x.map().clone()).collect()
        }
        _ => vec![Map::new()],
    };
    if card_kw(g, pid, c, "Accelerate", Some(src))? {
        // les coûts additionnels optionnels peuvent encore être payés (356.1.b.3)
        let mut e2: Vec<Map> = extras.iter().map(|ex| dict_with(ex, &[("acc", true.into())])).collect();
        e2.extend(extras);
        extras = e2;
    }
    let mut payable = vec![];
    for ex in extras {
        let ch = dict_with(&ex, &[("free", (!ignore_energy).into()), ("ignore_energy", ignore_energy.into())]);
        let (mut e, mut reqs) = total_cost(g, pid, c, &ch, src)?;
        if let Some((ae, ar)) = &alt_cost {
            e += ae;
            reqs.extend(ar.iter().cloned());
        }
        let ctx = pay_ctx(g, c);
        if g.can_pay(pid, e, &reqs, &ctx)? {
            payable.push((ex, e, reqs));
        }
    }
    if payable.is_empty() {
        return Ok(None);
    }
    let locs = match loc_choices {
        Some(l) => l,
        None => {
            let mut l = vec![Loc::Base];
            l.extend(g.bfs.iter().filter(|b| b.ctrl == Some(pid)).map(|b| Loc::Bf(b.idx)));
            l
        }
    };
    let mut ok = vec![];
    for l in locs {
        if loc_allowed(g, pid, c, l)? {
            ok.push(l);
        }
    }
    if ok.is_empty() {
        return Ok(None);
    }
    let (ex, e, reqs) = if payable.len() > 1 {
        let opts: Vec<Value> = payable
            .iter()
            .map(|(ex, e, r)| {
                OptV { label: extra_label(g, ex), value: Value::Tuple(vec![ex.clone().into(), (*e).into(), reqs_tuple(r)]), obj: Value::None }
                    .into()
            })
            .collect();
        let pick = g.ask(pid, "play_extra", opts, crate::map! { card = c })?;
        let v = pick.opt().value.list().to_vec();
        (v[0].map().clone(), v[1].int(), reqs_of(&v[2]))
    } else {
        payable.swap_remove(0)
    };
    let loc = g.ask(pid, "play_location", ok.into_iter().map(Value::Loc).collect(), crate::map! { card = c })?;
    let mut ch = dict_with(&ex, &[("loc", loc), ("free", (!ignore_energy).into()), ("ignore_energy", ignore_energy.into())]);
    if e != 0 || !reqs.is_empty() {
        ch.set("pay_cost", Value::Tuple(vec![e.into(), reqs_tuple(&reqs)])); // power (ignore_energy) et coûts additionnels
    }
    Ok(play_card(g, pid, c, src, ch, true)?.opt_obj())
}

/// `unit_play_choices(g, pid, c, src, locs)` : choix de jeu d'une unité jouée par un effet (419.3) : emplacements
/// (locs None : les habituels, `unit_locations` ; sinon exactement locs) sans les interdits, x choix « as you play
/// me », x [Accelerate]. Les coûts ne sont pas vérifiés ici.
pub fn unit_play_choices(g: &mut Game, pid: usize, c: O, src: &str, locs: Option<Vec<Loc>>) -> Res<Vec<Map>> {
    let Some(im) = g.impl_o(c) else { return Ok(vec![]) };
    if g.spec(c).type_ != "Unit" || play_forbidden(g, pid, c, src)? {
        return Ok(vec![]);
    }
    let pairs: Vec<(Loc, Map)> = match locs {
        None => unit_locations_extra(g, pid, c, src, false)?,
        Some(ls) => {
            let mut p = vec![];
            for l in ls {
                if loc_allowed(g, pid, c, l)? {
                    p.push((l, Map::new()));
                }
            }
            p
        }
    };
    let extras: Vec<Map> = if im.as_played.truthy() {
        im.as_played.call(g, &[pid.into(), c.into()])?.list().iter().map(|x| x.map().clone()).collect()
    } else {
        vec![Map::new()]
    };
    let acc = card_kw(g, pid, c, "Accelerate", Some(src))?;
    let accs: &[bool] = if acc { &[false, true] } else { &[false] };
    let mut out = vec![];
    for (l, lx) in &pairs {
        for ex in &extras {
            for &a in accs {
                let mut m = dict_with(ex, &[("loc", Value::Loc(*l)), ("acc", a.into())]);
                for (k, v) in &lx.0 {
                    m.set(k, v.clone());
                }
                out.push(m);
            }
        }
    }
    Ok(out)
}

const COST_KEYS: [&str; 11] = ["acc", "rep", "reps", "flow", "alt", "alt_e", "alt_reqs", "kill", "xp", "free", "ignore_energy"];

fn oids_of(g: &Game, tg: &Value) -> Map {
    let mut m = Map::new();
    for u in tg.list() {
        if let Some(o) = as_uid(u).and_then(|x| g.obj(x)) {
            m.set(&g.o(o).uid.to_string(), g.o(o).oid);
        }
    }
    m
}

/// `remake_choices(g, it, pid)` (750-755) : « You may make new choices for it » pour un sort de la chaîne : cibles,
/// modes, emplacements et destinations du sort et de chaque répétition (les choix « as you play » et les coûts
/// additionnels sont gardés, 752.2). Les objets nouvellement choisis reçoivent leurs effets de ciblage (754).
pub fn remake_choices(g: &mut Game, it: ItemId, pid: usize) -> Res<()> {
    if g.it(it).kind != "spell" {
        return Ok(());
    }
    let card = g.it(it).card.unwrap();
    let Some(im) = g.impl_o(card) else { return Ok(()) };
    if im.choices.is_none() {
        return Ok(());
    }
    let ctx = crate::map! { hidden_bf = g.it(it).data.g("hidden_bf"), card = card, src = g.it(it).data.g("from") };
    let f = if !im.all_choices.is_none() { &im.all_choices } else { &im.choices };
    let cands: Vec<Map> = f
        .call(g, &[pid.into(), ctx.into()])?
        .list()
        .iter()
        .map(|ch| Map(ch.map().0.iter().filter(|(k, _)| !COST_KEYS.contains(&&**k)).cloned().collect()))
        .collect();
    if cands.is_empty() {
        return Ok(());
    }
    let before = g.it(it).chosen.clone();
    let keep: Value = OptV { label: tr("garder les choix actuels", "keep the current choices"), value: Value::None, obj: Value::None }.into();
    let opts_of = |g: &Game, cs: &[&Map]| -> Vec<Value> {
        let mut v: Vec<Value> =
            cs.iter().map(|ch| OptV { label: choice_label(g, ch), value: (*ch).clone().into(), obj: Value::None }.into()).collect();
        v.push(keep.clone());
        v
    };
    let all: Vec<&Map> = cands.iter().collect();
    let opts = opts_of(g, &all);
    let pick = g.ask(pid, "new_choices", opts, crate::map! { item = it })?;
    let preds: Vec<Value> = im.preds.list().to_vec();
    if !pick.is_none() && !pick.opt().value.is_none() {
        let ch = pick.opt().value.map().clone();
        for (k, v) in &ch.0 {
            g.it_mut(it).data.set(k, v.clone());
        }
        g.it_mut(it).targets.clear();
        g.it_mut(it).chosen.clear();
        for (i, uid) in ch.g("tg").list().iter().enumerate() {
            match as_uid(uid).and_then(|u| g.obj(u)) {
                Some(o) => {
                    let pred = if preds.is_empty() { Value::None } else { preds[i.min(preds.len() - 1)].clone() };
                    g.add_target(it, o, pred);
                }
                None => g.it_mut(it).targets.push((as_uid(uid), -1, Value::None)),
            }
        }
        for t in g.it(it).data.g("t2").list().to_vec() {
            let u = t.list()[0].int() as u32;
            g.it_mut(it).chosen.insert(u);
        }
    }
    if !g.it(it).data.g("t2").is_none() {
        // Repeat imprimé : les cibles de la répétition
        let with_tg: Vec<&Map> = cands.iter().filter(|ch| ch.g("tg").truthy()).collect();
        let opts = opts_of(g, &with_tg);
        let pick = g.ask(pid, "new_choices", opts, crate::map! { item = it, repeat = true })?;
        if !pick.is_none() && !pick.opt().value.is_none() {
            let mut t2 = vec![];
            for u in pick.opt().value.map().g("tg").list() {
                if let Some(o) = as_uid(u).and_then(|x| g.obj(x)) {
                    t2.push(Value::Tuple(vec![u.clone(), g.o(o).oid.into()]));
                }
            }
            g.it_mut(it).data.set("t2", Value::List(t2));
        }
    }
    let nreps = g.it(it).data.g("reps").list().len();
    for i in 0..nreps {
        // instances de Repeat données
        let opts = opts_of(g, &all);
        let pick = g.ask(pid, "new_choices", opts, crate::map! { item = it, repeat = true })?;
        if !pick.is_none() && !pick.opt().value.is_none() {
            let pv = pick.opt().value.map().clone();
            let mut rc = g.it(it).data.g("reps").list()[i].map().clone();
            rc.0.retain(|(k, _)| k.starts_with('_'));
            for (k, v) in &pv.0 {
                rc.set(k, v.clone());
            }
            let oids = oids_of(g, &rc.g("tg"));
            rc.set("_oids", oids);
            g.it_mut(it).data.get_mut("reps").unwrap().list_mut()[i] = rc.into();
        }
    }
    for t in g.it(it).data.g("t2").list().to_vec() {
        let u = t.list()[0].int() as u32;
        g.it_mut(it).chosen.insert(u);
    }
    for rc in g.it(it).data.g("reps").list().to_vec() {
        for k in rc.map().g("_oids").map_or_empty().keys() {
            let u: u32 = k.parse().unwrap();
            g.it_mut(it).chosen.insert(u);
        }
    }
    let new: Vec<u32> = g.it(it).chosen.difference(&before).copied().collect();
    for uid in new {
        // 754 : effets de ciblage des objets nouvellement choisis
        if let Some(o) = g.obj(uid) {
            let ctrl = g.it(it).ctrl;
            if g.o(o).ctrl != ctrl && g.spec(o).type_ == "Unit" {
                g.hist.chose_enemy[ctrl] = true;
            }
            g.emit("chosen", crate::map! { obj = o, item = it })?;
        }
    }
    Ok(())
}

trait MapOrEmpty {
    fn map_or_empty(&self) -> Map;
}
impl MapOrEmpty for Value {
    fn map_or_empty(&self) -> Map {
        match self {
            Value::Map(m) => m.clone(),
            _ => Map::new(),
        }
    }
}

/// `_CHOICE_WORDS` / `_CHOICE_WORDS_EN`
const CHOICE_WORDS: &[(&str, &str, &str)] = &[
    ("acc", "avec Accelerate", "with Accelerate"),
    ("free", "gratuitement", "for free"),
    ("ignore_energy", "sans payer l'énergie", "without paying the energy"),
    ("pay_power", "en payant la puissance", "paying the power"),
    ("rep", "répété", "repeated"),
    ("flow", "avec Flow", "with Flow"),
];

/// `_CHOICE_UIDS` / `_CHOICE_UIDS_EN` : (clé, français, anglais)
pub const CHOICE_UIDS: &[(&str, &str, &str)] = &[
    ("kill", "en détruisant", "killing"),
    ("kills", "en détruisant", "killing"),
    ("spend", "en dépensant le buff de", "spending the buff of"),
    ("disc", "en défaussant", "discarding"),
    ("exh", "en épuisant", "exhausting"),
    ("rec", "en recyclant", "recycling"),
    ("ret", "en renvoyant en main", "returning to hand"),
    ("ret_gear", "en renvoyant en main", "returning to hand"),
    ("cards", "cartes :", "cards:"),
    ("cs", "cartes :", "cards:"),
];

/// `_PAID_FLAGS` : coûts additionnels optionnels choisis en jouant une unité ou un équipement.
const PAID_FLAGS: [&str; 13] = ["akshan", "bard", "bb", "bcc", "boar", "calm_paid", "cg", "cub", "gm", "masa", "paid", "pyke", "zed"];
/// `_PLAY_COST_UIDS`
const PLAY_COST_UIDS: [&str; 5] = ["kill", "kills", "ret", "ret_gear", "spend"];

fn choice_uids(k: &str) -> Option<(&'static str, &'static str)> {
    CHOICE_UIDS.iter().find(|x| x.0 == k).map(|x| (x.1, x.2))
}

fn as_seq(v: &Value) -> Vec<Value> {
    match v {
        Value::List(l) | Value::Tuple(l) => l.clone(),
        v => vec![v.clone()],
    }
}

/// `play_cost_parts(g, ch)` : coût additionnel choisi en jouant une carte, en texte (une ligne par façon de payer).
pub fn play_cost_parts(g: &Game, ch: &Map) -> Vec<String> {
    let mut parts = vec![];
    for k in PLAY_COST_UIDS {
        let v = ch.g(k);
        let vs = if v.is_none() { vec![] } else { as_seq(&v) };
        if !vs.is_empty() {
            let (fr, en) = choice_uids(k).unwrap();
            let names: Vec<String> = vs.iter().map(|x| uid_name(g, x)).collect();
            parts.push(tr(fr, en) + " " + &names.join(", "));
        }
    }
    if PAID_FLAGS.iter().any(|k| ch.g(k).truthy()) {
        parts.push(tr("avec le coût supplémentaire", "paying the additional cost"));
    }
    if ch.g("xp").truthy() {
        let x = py_str(g, &ch.g("xp"));
        parts.push(tr(format!("en dépensant {x} XP"), format!("spending {x} XP")));
    }
    parts
}

/// `_uid_name(g, x)` : nom d'un objet (plateau, puis mains, défausses, decks, bannis), sinon le texte de x.
pub fn uid_name(g: &Game, x: &Value) -> String {
    let mut o = as_uid(x).and_then(|u| g.obj(u));
    if o.is_none() {
        if let Some(u) = as_uid(x) {
            o = g
                .p
                .iter()
                .flat_map(|p| p.hand.iter().chain(p.trash.iter()).chain(p.deck.iter()).chain(p.banish.iter()))
                .copied()
                .find(|&c| g.o(c).uid == u);
        }
    }
    match o {
        Some(o) => g.label(o),
        None => py_str(g, x),
    }
}

/// `_tg_name(g, u)` : cible d'un choix (« Shipyard Skulker #2 » quand elle a des homonymes en jeu).
fn tg_name(g: &Game, u: &Value) -> String {
    match as_uid(u).and_then(|x| g.obj(x)) {
        Some(o) => g.label(o),
        None => py_str(g, u),
    }
}

/// `_choice_label(g, ch)` : libellé d'un choix pour un humain.
pub fn choice_label(g: &Game, ch: &Map) -> String {
    let mut keys: Vec<&str> = ch.keys().collect();
    keys.sort();
    let mut parts: Vec<String> = vec![];
    for k in keys {
        let v = ch.g(k);
        if k == "tg" || k == "tg2" {
            let names: Vec<String> = v.list().iter().map(|u| tg_name(g, u)).collect();
            let names = names.join(", ");
            parts.push(if !names.is_empty() {
                (if k == "tg" { tr("cible ", "target ") } else { tr("cible répétée ", "repeat target ") }) + &names
            } else {
                tr("aucune cible", "no target")
            });
        } else if k == "item" {
            parts.push(match v.opt_int().and_then(|i| item_by_id(g, i)) {
                Some(x) => {
                    let n = &g.it(x).name;
                    tr(format!("sort {n}"), format!("spell {n}"))
                }
                None => tr("un sort", "a spell"),
            });
        } else if k == "loc" || k == "dest" {
            match v {
                Value::Loc(Loc::Base) => parts.push(tr("à la base", "to base")),
                Value::Loc(Loc::Bf(i)) => parts.push(g.bfs[i].name.clone()),
                Value::Int(i) => parts.push(g.bfs[i as usize].name.clone()),
                Value::Str(ref s) if &**s == "base" => parts.push(tr("à la base", "to base")),
                _ => {}
            }
        } else if k == "mover" && as_uid(&v).and_then(|u| g.obj(u)).is_some() {
            let l = g.label(as_uid(&v).and_then(|u| g.obj(u)).unwrap());
            parts.push(tr(format!("déplace {l}"), format!("moves {l}")));
        } else if let Some(w) = CHOICE_WORDS.iter().find(|w| w.0 == k) {
            if v.truthy() {
                parts.push(tr(w.1, w.2));
            }
        } else if k == "mover_oid" || k == "oid" {
            continue;
        } else if choice_uids(k).is_some() && !v.is_none() {
            let (fr, en) = choice_uids(k).unwrap();
            let names: Vec<String> = as_seq(&v).iter().map(|x| uid_name(g, x)).collect();
            if !names.is_empty() {
                parts.push(tr(fr, en) + " " + &names.join(", "));
            }
        } else if !v.is_none() && !matches!(v, Value::Bool(false)) {
            parts.push(format!("{k} = {}", py_str(g, &v)));
        }
    }
    let s = parts.join(&tr(" ; ", "; "));
    if s.is_empty() {
        tr("sans choix", "no choice")
    } else {
        s
    }
}

/// `gain_control_item(g, it, pid)` (359.3.f) : le contrôleur de l'item change ; celui qui a joué la carte l'a
/// toujours jouée (it.data['played_by']).
pub fn gain_control_item(g: &mut Game, it: ItemId, pid: usize) {
    let ctrl = g.it(it).ctrl;
    if !g.it(it).data.contains("played_by") {
        g.it_mut(it).data.set("played_by", ctrl);
    }
    g.it_mut(it).ctrl = pid;
    let n = g.it(it).name.clone();
    g.log(|| format!("  P{pid} gains control of {n}"));
}

// ---------------------------------------------------------------------------------------------- coûts, capacités
fn starts_ci(cs: &[char], i: usize, word: &str) -> bool {
    let w: Vec<char> = word.chars().collect();
    i + w.len() <= cs.len() && cs[i..i + w.len()].iter().zip(w.iter()).all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
}

/// `parse_cost(text)` : '2 energy and 1 fury rune' -> (2, [{Fury}]) ; '1 rune of any type' -> (0, [ANY]).
/// Expression de Python : `(\d+)\s+(energy|runes? of any type|(fury|calm|mind|body|chaos|order) runes?)`, re.I.
pub fn parse_cost(text: &str) -> (i64, Vec<Req>) {
    let (mut e, mut reqs) = (0i64, vec![]);
    if text.is_empty() {
        return (e, reqs);
    }
    let cs: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if !cs[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < cs.len() && cs[j].is_ascii_digit() {
            j += 1;
        }
        let mut k = j;
        while k < cs.len() && cs[k].is_whitespace() {
            k += 1;
        }
        if k == j {
            i += 1;
            continue;
        }
        let n: i64 = cs[i..j].iter().collect::<String>().parse().unwrap();
        // rune(s)? : 's' facultatif glouton
        let runes = |p: usize| -> Option<usize> {
            if !starts_ci(&cs, p, "rune") {
                return None;
            }
            Some(if starts_ci(&cs, p + 4, "s") { p + 5 } else { p + 4 })
        };
        let mut end = None;
        if starts_ci(&cs, k, "energy") {
            e += n;
            end = Some(k + 6);
        } else if let Some(p) = runes(k).and_then(|p| {
            // « runes? of any type » : si « runes » ne suit pas « of any type », pas de retour arrière utile
            if starts_ci(&cs, p, " of any type") {
                Some(p + 12)
            } else {
                None
            }
        }) {
            for _ in 0..n {
                reqs.push(any_req());
            }
            end = Some(p);
        } else {
            for d in DOMAINS {
                if starts_ci(&cs, k, d) && starts_ci(&cs, k + d.len(), " ") {
                    if let Some(p) = runes(k + d.len() + 1) {
                        let r: Req = [d.to_string()].into_iter().collect();
                        for _ in 0..n {
                            reqs.push(r.clone());
                        }
                        end = Some(p);
                        break;
                    }
                }
            }
        }
        match end {
            Some(p) => i = p,
            None => i += 1,
        }
    }
    (e, reqs)
}

/// `power_list(text)` : '1 energy and 1 rune of any type' -> (1, ['A']) ; '1 fury rune' -> (0, ['Fury']).
pub fn power_list(text: &str) -> (i64, Vec<String>) {
    let (e, reqs) = parse_cost(text);
    let any = any_req();
    (e, reqs.iter().map(|r| if *r == any { "A".to_string() } else { r.iter().next().unwrap().clone() }).collect())
}

fn doms_union(reqs: &[Req]) -> Vec<String> {
    let s: BTreeSet<String> = reqs.iter().flat_map(|r| r.iter().cloned()).collect();
    s.into_iter().collect()
}

fn doms_tuple(doms: &[String]) -> Value {
    if doms.is_empty() {
        Value::Tuple(DOMAINS.iter().map(|d| Value::from(*d)).collect())
    } else {
        Value::Tuple(doms.iter().map(|d| Value::from(d.as_str())).collect())
    }
}

/// `repeat_cost(text)` : tuple (énergie, domaines, power) pour `Impl.repeat`.
pub fn repeat_cost(text: &str) -> Value {
    let (e, reqs) = parse_cost(text);
    let doms = doms_union(&reqs);
    Value::Tuple(vec![e.into(), doms_tuple(&doms), reqs.len().into()])
}

/// `flow_cost(text)` : tuple (énergie, power, domaines) pour `Impl.flow`.
pub fn flow_cost(text: &str) -> Value {
    let (e, reqs) = parse_cost(text);
    let doms = doms_union(&reqs);
    Value::Tuple(vec![e.into(), reqs.len().into(), doms_tuple(&doms)])
}

/// `add_ability(text, exhaust, kill, can)` : entrée d'`Impl.add` (capacité [Add] utilisable en payant).
pub fn add_ability(text: &str, exhaust: bool, kill: bool, can: Value) -> Map {
    let (e, p) = power_list(text);
    crate::map! { e = e, p = Value::List(p.into_iter().map(Value::from).collect()), exhaust = exhaust, kill = kill, can = can }
}

/// `ability(name, cost, timing, exhaust, xp, kill_self, resolve, choices, preds, can, **extra)` : dict d'une capacité
/// activée (Impl.abilities).
pub fn ability(name: &str, a: AbilityArgs) -> Map {
    let resolve = if a.resolve.is_none() { Value::func("noop", |_g, _a| Ok(Value::None)) } else { a.resolve };
    let mut ab = crate::map! { name = name, timing = a.timing, exhaust = a.exhaust, cost = a.cost.func(), resolve = resolve };
    if !a.choices.is_none() {
        ab.set("choices", a.choices);
    }
    if !a.preds.is_none() {
        ab.set("preds", a.preds);
    }
    let xp = a.xp;
    let mut conds = vec![];
    if !a.can.is_none() {
        conds.push(a.can.clone());
    }
    if xp != 0 {
        conds.push(Value::func("xp_can", move |g, a| Ok((g.p[a[0].int() as usize].xp >= xp).into())));
    }
    if !conds.is_empty() {
        ab.set(
            "can",
            Value::func("can", move |g, a| {
                for c in &conds {
                    if !c.call(g, a)?.truthy() {
                        return Ok(false.into());
                    }
                }
                Ok(true.into())
            }),
        );
    }
    let extra_fn = a.extra_cost;
    let kill_self = a.kill_self;
    if xp != 0 || kill_self || !extra_fn.is_none() {
        ab.set(
            "extra_cost",
            Value::func("pay_extra", move |g, a| {
                let (pid, o) = (a[0].int() as usize, a[1].obj());
                if xp != 0 {
                    g.spend_xp(pid, xp)?;
                }
                let mut r = Value::None;
                if !extra_fn.is_none() {
                    r = extra_fn.call(g, a)?;
                }
                if kill_self && g.on_board(o) {
                    g.kill(&[o], Some(pid), true)?;
                }
                // le choix modifié par extra_fn est rendu à l'appelant (convention de `actions`)
                Ok(if matches!(r, Value::Map(_)) { r } else { Value::None })
            }),
        );
    }
    for (k, v) in a.extra.0 {
        ab.set(&k, v);
    }
    ab
}

fn equip_choices() -> Value {
    Value::func("equip_choices", |g, a| {
        let (pid, o) = (a[0].int() as usize, a[1].obj());
        let at = g.o(o).attached_to;
        let fs = friends(g, pid, false, None)?;
        Ok(Value::List(
            fs.into_iter()
                .filter(|&u| Some(g.o(u).uid) != at)
                .map(|u| crate::map! { tg = Value::Tuple(vec![g.o(u).uid.into()]) }.into())
                .collect(),
        ))
    })
}

fn equip_resolve() -> Value {
    Value::func("equip_resolve", |g, a| {
        let it = a[0].item();
        if g.legal(it, 0)?.is_some() {
            if let Some(src) = g.it(it).src.and_then(|s| g.obj(s)) {
                let u = g.legal(it, 0)?.unwrap();
                attach(g, src, u)?;
            }
        }
        Ok(Value::None)
    })
}

/// `equip_ability(domain)` : Equip au coût d'un power de ce domaine.
pub fn equip_ability(domain: &str) -> Map {
    let r: Req = [domain.to_string()].into_iter().collect();
    crate::map! {
        name = "Equip", timing = "main", choices = equip_choices(), cost = Cost::Parts(0, vec![r]).func(),
        preds = Value::List(vec![p_friend_v()]), resolve = equip_resolve(),
    }
}

/// `equip_ability_cost(cost, timing)` (818) : attacher cet équipement à une unité contrôlée.
pub fn equip_ability_cost(cost: Cost, timing: &'static str) -> Map {
    crate::map! {
        name = "Equip", timing = timing, choices = equip_choices(), cost = cost.func(),
        preds = Value::List(vec![p_friend_v()]), resolve = equip_resolve(),
    }
}

/// `empower_ability(cost, timing, exhaust)` (827) : « Empower this. Use only if not Empowered » (permanents et
/// légendes).
pub fn empower_ability(cost: Cost, timing: &'static str, exhaust: bool) -> Map {
    let resolve = Value::func("empower_resolve", |g, a| {
        let it = a[0].item();
        let src = g.it(it).src;
        let mut o = src.and_then(|s| g.obj(s));
        let leg = g.p[g.it(it).ctrl].legend;
        if o.is_none() && Some(g.o(leg).uid) == src {
            o = Some(leg); // capacités de légende : item.src est l'uid de la légende
        }
        if let Some(o) = o {
            g.empower(Some(o), None)?;
        }
        Ok(Value::None)
    });
    let can = Value::func("empower_can", |g, a| Ok((!g.o(a[1].obj()).empowered).into()));
    ability("Empower", AbilityArgs { cost, timing, exhaust, resolve, can, ..Default::default() })
}

/// `Impl(..., equip=cost)` : ajoute la capacité Equip s'il n'y en a pas.
pub fn impl_equip(im: &mut Impl, cost: Cost) {
    if !im.abilities.iter().any(|a| a.g("name").opt_str() == Some("Equip")) {
        im.abilities.push(equip_ability_cost(cost, "main"));
    }
}

/// `Impl(..., empower=cost)` : la capacité Empower en tête.
pub fn impl_empower(im: &mut Impl, cost: Cost) {
    im.abilities.insert(0, empower_ability(cost, "main", false));
}

/// `when_level(n)` : cond(g, o) de kw_if / might_if
pub fn when_level(n: i64) -> Value {
    Value::func("when_level", move |g, a| Ok((g.p[g.o(a[0].obj()).ctrl].xp >= n).into()))
}
pub fn when_empowered() -> Value {
    Value::func("when_empowered", |g, a| Ok(g.o(a[0].obj()).empowered.into()))
}
pub fn when_mighty() -> Value {
    Value::func("when_mighty", |g, a| Ok(g.mighty(a[0].obj())?.into()))
}
pub fn when_legion() -> Value {
    Value::func("when_legion", |g, a| {
        let o = a[0].obj();
        Ok(g.legion(g.o(o).ctrl, Some(o)).into())
    })
}
pub fn when_at_bf() -> Value {
    Value::func("when_at_bf", |g, a| Ok(is_bf(g.o(a[0].obj()).loc).into()))
}

/// `repeatable(fn)` (820) : quand le coût de Repeat est payé, fn rejoue avec les cibles de la répétition ('t2',
/// vérifiées avec les mêmes prédicats).
pub fn repeatable(f: Value) -> Value {
    Value::func("repeatable", move |g, a| {
        let it = a[0].item();
        f.call(g, &[it.into()])?;
        if g.it(it).data.g("rep").truthy() {
            let mut data = g.it(it).data.clone();
            data.set("rep", false);
            let (ctrl, name, card, src) = (g.it(it).ctrl, g.it(it).name.clone(), g.it(it).card, g.it(it).src);
            let it2 = g.new_item("spell", ctrl, &name, Value::None, card, src, data);
            let preds: Vec<Value> = card.and_then(|c| g.impl_o(c)).map(|im| im.preds.list().to_vec()).unwrap_or_default();
            for (i, t) in g.it(it).data.g("t2").list().to_vec().iter().enumerate() {
                let t = t.list();
                let pred = if preds.is_empty() { Value::None } else { preds[i.min(preds.len() - 1)].clone() };
                g.it_mut(it2).targets.push((Some(t[0].int() as u32), t[1].int(), pred));
            }
            f.call(g, &[it2.into()])?;
        }
        Ok(Value::None)
    })
}

// ---------------------------------------------------------------------------------------------- jetons, Disarm
/// `token_locations(g, pid, name, locs)` : où pid peut jouer un jeton unité de ce nom (184.2, 355.2), sans les
/// emplacements interdits (loc_allowed). Le jeton témoin ne prend pas d'uid.
pub fn token_locations(g: &mut Game, pid: usize, name: &str, locs: Option<Vec<Loc>>) -> Res<Vec<Loc>> {
    let probe = g.new_probe(name, pid);
    let locs = locs.unwrap_or_else(|| {
        let mut l = vec![Loc::Base];
        l.extend(g.bfs.iter().filter(|b| b.ctrl == Some(pid)).map(|b| Loc::Bf(b.idx)));
        l
    });
    let mut out = vec![];
    let mut err = None;
    for l in locs {
        match loc_allowed(g, pid, probe, l) {
            Ok(true) => out.push(l),
            Ok(false) => {}
            Err(e) => {
                err = Some(e);
                break;
            }
        }
    }
    g.drop_probe(probe);
    match err {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

/// `make_token(g, name, pid, loc, ready)` : jouer un jeton (185.2.a, 187) : le seul chemin des jetons joués par un
/// effet. None quand un jeton unité ne peut pas être joué là (358.3.a).
pub fn make_token(g: &mut Game, name: &str, pid: usize, loc: Loc, ready: bool) -> Res<Option<O>> {
    make_token_inner(g, name, pid, loc, ready, true)
}

fn make_token_inner(g: &mut Game, name: &str, pid: usize, loc: Loc, ready: bool, copy: bool) -> Res<Option<O>> {
    let mut name = name.to_string();
    let mut ready = ready;
    if copy && spec(&name).type_ == "Unit" {
        for (src, im) in g.providers("token_swap") {
            // Heimerdinger, Academy Instructor (remplacement facultatif)
            let nn = im.hook("token_swap").call(g, &[src, pid.into(), name.as_str().into(), Value::Loc(loc)])?;
            if nn.truthy() && nn.str() != name {
                let ctx = crate::map! { token = name.as_str(), new = nn.clone() };
                if g.ask(pid, "token_swap", vec![true.into(), false.into()], ctx)?.truthy() {
                    name = nn.str().to_string();
                    break;
                }
            }
        }
    }
    let t = g.new_token(&name, pid);
    if spec(&name).type_ == "Unit" && !loc_allowed(g, pid, t, loc)? {
        let l = loc_str(Some(loc));
        g.log(|| format!("  a {name} token can't be played at {l}"));
        return Ok(None);
    }
    let mut extra = 0;
    if copy && spec(&name).type_ == "Unit" {
        for (src, im) in g.providers("token_rep") {
            if im.hook("token_rep").call(g, &[src, pid.into(), name.as_str().into()])?.truthy() {
                extra += 1;
            }
        }
    }
    g.o_mut(t).played_to = Some(loc);
    if !ready && others_enter_ready(g, pid, t)? {
        ready = true; // « Your tokens enter ready » (369.3)
    }
    g.enter_board(t, pid, loc, ready);
    let l = loc_str(Some(loc));
    g.log(|| format!("  P{pid} plays a {name} token at {l}{}", if ready { " ready" } else { "" }));
    keyword_play_triggers(g, pid, t, None)?;
    g.played_event(pid, t, None)?;
    for _ in 0..extra {
        make_token_inner(g, &name, pid, loc, ready, false)?;
    }
    Ok(Some(t))
}

/// `disarm_trigger(g, u, n)` : [Disarm X] (831) : « When I attack, give an enemy unit here -X might this turn » (un
/// déclenchement avec la valeur totale, 831.1.d.1). Cible choisie à la finalisation (Deflect payé, 809).
pub fn disarm_trigger(g: &mut Game, u: O, n: i64) -> Res<()> {
    let here = g.o(u).loc;
    let res = Value::func("disarm", |g, a| {
        let it = a[0].item();
        if let Some(e) = g.legal(it, 0)? {
            let n = g.it(it).data.g("n").int();
            g.mod_(e, -n, "turn".into(), None)?;
        }
        Ok(Value::None)
    });
    let opts = Value::func("disarm_options", move |g, a| {
        let ctrl = g.it(a[0].item()).ctrl;
        let es = enemies(g, ctrl, false, None, true)?;
        Ok(es.into_iter().filter(|&e| g.o(e).loc == here).collect::<Vec<O>>().into())
    });
    let pred = Value::func("disarm_pred", move |g, a| {
        let e = a[1].obj();
        Ok((g.o(e).ctrl != g.it(a[0].item()).ctrl && g.o(e).loc == here).into())
    });
    let choose = trig_target(opts, pred, "disarm_target", false, true);
    let (ctrl, uid, cn) = (g.o(u).ctrl, g.o(u).uid, g.cname(u).to_string());
    g.queue_trigger(ctrl, &format!("Disarm {cn}"), res, crate::map! { n = n }, Some(uid), false, choose, Value::None);
    Ok(())
}

/// `pay_deflect(g, it, o)` (809) : payer le Deflect de o pour une capacité déclenchée. False si impayable.
pub fn pay_deflect(g: &mut Game, it: ItemId, o: O) -> Res<bool> {
    let ctrl = g.it(it).ctrl;
    let reqs = deflect_reqs(g, ctrl, &crate::map! { tg = Value::Tuple(vec![g.o(o).uid.into()]) })?;
    if reqs.is_empty() {
        return Ok(true);
    }
    g.pay(ctrl, 0, &reqs, &crate::map! { kind = "ability" }.into())
}

// ---------------------------------------------------------------------------------------------- decks, mots-clés
/// `deck_problems(deck)` : cartes non modélisées (absentes d'IMPL), jetons, cartes [Unique] en plusieurs exemplaires
/// (825).
pub fn deck_problems(deck: &DeckArg) -> Vec<String> {
    let (cards_, names): (Vec<String>, Vec<String>) = match deck {
        DeckArg::Dict { legend, champion, main, runes, battlefields } => {
            let mut cards_: Vec<String> = champion.iter().cloned().collect();
            cards_.extend(main.iter().cloned());
            cards_.retain(|n| !n.is_empty());
            let mut others: Vec<String> = legend.iter().cloned().collect();
            others.extend(battlefields.iter().cloned());
            others.extend(runes.iter().map(|r| if r.ends_with(" Rune") { r.clone() } else { format!("{r} Rune") }));
            let mut names = cards_.clone();
            names.extend(others.into_iter().filter(|n| !n.is_empty()));
            (cards_, names)
        }
        DeckArg::Names(n) => (n.clone(), n.clone()),
    };
    let mut out = vec![];
    let set: BTreeSet<&String> = names.iter().collect();
    for n in set {
        if !registry().impls.contains_key(n) {
            out.push(format!("{n}: not modelled"));
        } else if is_token(n) {
            out.push(format!("{n}: a token can't be put in a deck (rule 187)"));
        }
    }
    let mut cnt: std::collections::BTreeMap<&String, usize> = std::collections::BTreeMap::new();
    for n in &cards_ {
        *cnt.entry(n).or_default() += 1;
    }
    for (n, c) in cnt {
        if specs().contains_key(n) && keywords_of(n).contains("Unique") && c > 1 {
            out.push(format!("{n}: [Unique], {c} copies"));
        }
    }
    out
}

/// `re.sub(r"\([^)]*\)", rep, text)`
fn strip_parens(text: &str, rep: &str) -> String {
    let cs: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '(' {
            if let Some(j) = cs[i + 1..].iter().position(|&c| c == ')') {
                out.push_str(rep);
                i += j + 2;
                continue;
            }
        }
        out.push(cs[i]);
        i += 1;
    }
    out
}

/// `keywords_of(name)` : mots-clés imprimés (entre crochets, texte de rappel retiré).
pub fn keywords_of(name: &str) -> BTreeSet<String> {
    let t: Vec<char> = strip_parens(&spec(name).text, "").chars().collect();
    let mut out = BTreeSet::new();
    let mut i = 0;
    // r"\[([A-Z][A-Za-z\-]+)"
    while i < t.len() {
        if t[i] == '[' && i + 2 < t.len() && t[i + 1].is_ascii_uppercase() {
            let mut j = i + 2;
            while j < t.len() && (t[j].is_ascii_alphabetic() || t[j] == '-') {
                j += 1;
            }
            if j > i + 2 {
                out.insert(t[i + 1..j].iter().collect());
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// `_KW_SIMPLE`
const KW_SIMPLE: [&str; 14] = [
    "Action", "Reaction", "Accelerate", "Hidden", "Tank", "Ganking", "Backline", "Temporary", "Ambush", "Unique", "Vision",
    "Weaponmaster", "Quick-Draw", "Deploy",
];
/// `_KW_VALUE`
const KW_VALUE: [&str; 5] = ["Assault", "Shield", "Deflect", "Hunt", "Disarm"];

/// `_collect(m, fields)`
fn collect(simple: Option<&str>, valued: Option<&str>, n: Option<i64>, f: &mut KwFields) {
    match simple {
        Some("Action") => f.timing = Some("action"),
        Some("Reaction") => f.timing = Some("reaction"),
        Some("Accelerate") => f.accelerate = true,
        Some("Hidden") => f.hidden = true,
        Some("Ambush") => f.ambush = true,
        Some("Quick-Draw") => f.quickdraw = true,
        Some(s) => f.kw.get_or_insert_with(Map::new).set(s, 1),
        None => {
            let v = valued.unwrap();
            let kw = f.kw.get_or_insert_with(Map::new);
            let cur = kw.g(v).opt_int().unwrap_or(0);
            kw.set(v, cur + n.unwrap_or(1));
        }
    }
}

/// `parse_keywords(name)` : si le texte n'est fait que de mots-clés et de texte de rappel, les champs d'Impl qui le
/// modélisent, sinon None.
pub fn parse_keywords(name: &str) -> Option<KwFields> {
    let sp = specs().get(name)?;
    let t: Vec<char> = strip_parens(&sp.text, " ").chars().collect();
    let mut f = KwFields { kw: Some(Map::new()), ..Default::default() };
    let mut rest = String::new();
    let mut i = 0;
    // r"\[(SIMPLE)\]|\[(VALUE)(?: (\d+))?\]"
    while i < t.len() {
        if t[i] == '[' {
            if let Some(j) = t[i + 1..].iter().position(|&c| c == ']') {
                let inner: String = t[i + 1..i + 1 + j].iter().collect();
                let mut hit = false;
                if KW_SIMPLE.contains(&inner.as_str()) {
                    collect(Some(&inner), None, None, &mut f);
                    hit = true;
                } else if KW_VALUE.contains(&inner.as_str()) {
                    collect(None, Some(&inner), None, &mut f);
                    hit = true;
                } else if let Some((w, d)) = inner.split_once(' ') {
                    if KW_VALUE.contains(&w) && !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()) {
                        collect(None, Some(w), Some(d.parse().unwrap()), &mut f);
                        hit = true;
                    }
                }
                if hit {
                    rest.push(' ');
                    i += j + 2;
                    continue;
                }
            }
        }
        rest.push(t[i]);
        i += 1;
    }
    if !rest.trim_matches(|c| matches!(c, ' ' | '.' | '\n' | '\t')).is_empty() {
        return None;
    }
    if f.kw.as_ref().is_some_and(|m| m.0.is_empty()) {
        f.kw = None;
    }
    Some(f)
}

/// L'Impl d'une carte faite seulement de mots-clés (sans l'enregistrer), ou None.
fn build_auto(name: &str) -> Option<Impl> {
    let f = parse_keywords(name)?;
    let sp = spec(name);
    if matches!(sp.type_.as_str(), "Rune" | "Battlefield" | "Legend") {
        return None;
    }
    if sp.type_ == "Gear" && sp.tags.contains("Equipment") {
        return None; // le bonus de Might n'est pas dans la base (EQUIP_BONUS)
    }
    let mut im = Impl::new(name);
    im.timing = f.timing;
    im.accelerate = f.accelerate;
    im.hidden = f.hidden;
    im.ambush = f.ambush;
    im.quickdraw = f.quickdraw;
    if let Some(kw) = f.kw {
        im.kw = kw;
    }
    im.auto = true;
    Some(im)
}

/// `auto_keywords(name, register)` : l'Impl d'une carte dont le texte n'est que mots-clés et texte de rappel
/// (Sunlit Guardian : [Shield] [Tank]) ; r = Some(registre) l'enregistre.
pub fn auto_keywords(r: Option<&mut Registry>, name: &str) -> Option<Impl> {
    let im = build_auto(name)?;
    if let Some(r) = r {
        r.card(im.clone());
    }
    Some(im)
}

/// `NOT_CARDS` : imprimées dans la base mais pas des cartes jouables.
pub const NOT_CARDS: &[&str] = &["Buff", "XP Tracker"];
/// `TOKEN_NAMES` : jetons sans le supertype Token dans la base.
pub const TOKEN_NAMES: &[&str] = &["Bird", "Sprite", "Gold", "Reflection", "Recruit (NX)", "Mech", "Brush", "Bomb", "Baron Pit"];

/// `is_token(name)` : un jeton (187) ne va pas dans un deck.
pub fn is_token(name: &str) -> bool {
    specs().get(name).is_some_and(|sp| sp.super_ == "Token" || TOKEN_NAMES.contains(&name))
}

/// `register_keyword_only_cards()` : à appeler après les modules de cartes. Renvoie les noms enregistrés.
pub fn register_auto(r: &mut Registry) -> Vec<String> {
    let mut out = vec![];
    for name in specs().keys() {
        if r.impls.contains_key(name) || NOT_CARDS.contains(&name.as_str()) {
            continue;
        }
        if let Some(mut im) = build_auto(name) {
            im.module = Some("auto_keywords");
            r.card(im);
            out.push(name.clone());
        }
    }
    out
}

/// Titres français des questions communes et par carte de cards.py (`ask_text(...)` en tête du fichier).
pub fn register_core_ask_text(r: &mut Registry) {
    r.ask_text(&[
        ("choose", "Choisis une cible :"),
        ("add_kill", "Quelle unité détruire pour produire la ressource ?"),
        ("deploy_location", "Sur quel champ de bataille jouer cet équipement ?"),
        ("move_destination", "Vers quel autre champ de bataille déplacer cette unité ?"),
        ("disarm_target", "Disarm : quelle unité ennemie affaiblir ?"),
        ("token_swap", "Jouer un jeton Mech à la place ?"),
        ("altar_card", "Quelle carte de ta main placer ?"),
        ("altar_where", "Sur le dessus ou sous ton deck ?"),
        ("aphelios_mode", "Quel mode ?"),
        ("apothecary_pick", "Quelle unité soigner ?"),
        ("ava_play", "Quelle carte jouer ?"),
        ("azir_attach", "Quel équipement attacher ?"),
        ("azir_move", "Déplacer cette unité ?"),
        ("banish_from_trash", "Quelle carte de la défausse bannir ?"),
        ("bard_move", "Quelles unités déplacer, et où ?"),
        ("buff_target", "Quelle unité renforcer (buff) ?"),
        ("buhru_mode", "Piocher 1 ou te renforcer ?"),
        ("bullet_time_x", "Combien de runes payer (X) ?"),
        ("burn_player", "Quel joueur subit l'effet ?"),
        ("call_to_battle", "Quelle unité déplacer vers ce champ de bataille ?"),
        ("choose_player", "Quel joueur défausse ?"),
        ("clone_banish", "Quelle unité bannir ?"),
        ("constellation_kill", "Quelle unité détruire pour payer ?"),
        ("corrupted_dragon", "Quelles unités ennemies renvoyer à leur base ?"),
        ("dais_return", "Quelle unité renvoyer en main ?"),
        ("dame_unit", "Quelle unité de ce lieu ?"),
        ("death_replace", "Remplacer la mort de l'unité ?"),
        ("decree_strength", "Quelle carte révélée prendre ?"),
        ("detach", "Quel équipement détacher ?"),
        ("discord_subset", "Quelles unités ennemies renvoyer en main ?"),
        ("double_pick", "Quelle unité prendre parmi les cartes vues ?"),
        ("draw_pick", "Quelle carte piocher ?"),
        ("duel_keep", "Quelle unité garder ?"),
        ("empower_target", "Quelle unité de ce lieu ?"),
        ("entourage", "Quelle légende, et la préparer ou l'épuiser ?"),
        ("excited_discard", "Défausser quelle carte ?"),
        ("faefolk_target", "Quelle unité ennemie déplacer ici ?"),
        ("fate_pick", "Quel sort jouer ?"),
        ("fizz_pick", "Quel sort lancer ?"),
        ("forge_recycle", "Quelle carte recycler ?"),
        ("fortified_shield", "Quelle unité protéger ?"),
        ("foxfire_subset", "Quelles unités cibler ?"),
        ("friend_target", "Quelle unité choisir ?"),
        ("friendly_target", "Quelle unité cibler ?"),
        ("gear_target", "Quel équipement cibler ?"),
        ("harrowing_pick", "Quelle unité de ta défausse jouer ?"),
        ("hatchling_recycle", "Recycler la carte du dessus du deck ?"),
        ("here_to_help", "Quelle unité jouer ?"),
        ("investigator_pick", "Quelle carte de sa main choisir ?"),
        ("ivern_pick", "Quelle unité prendre parmi les cartes vues ?"),
        ("jayce_mode", "Quel mot-clé gagner ?"),
        ("judgment_gear", "Quels équipements garder ?"),
        ("judgment_hand", "Quelles cartes de ta main garder ?"),
        ("judgment_rune", "Quelles runes garder ?"),
        ("judgment_unit", "Quelles unités garder ?"),
        ("kato_target", "Quelle unité alliée cibler ?"),
        ("kharox_pick", "Quelle unité de sa défausse jouer ?"),
        ("matriarch_pick", "Quelle unité jouer à ta base ?"),
        ("mf_ready", "Que préparer : une unité ou une rune ?"),
        ("minah_mode", "Piocher ou défausser ?"),
        ("mindsplitter_pick", "Quelle carte de la main adverse défausser ?"),
        ("morgana_target", "Quelle unité cibler ?"),
        ("move_enemy", "Quelle unité ennemie déplacer, et où ?"),
        ("name_tag", "Quel nom de champion choisir ?"),
        ("ornn_pick", "Quel équipement prendre parmi les cartes vues ?"),
        ("overt_spend", "Dépenser le buff de cette unité ?"),
        ("party_favors", "Que choisis-tu : des cartes ou des runes ?"),
        ("pay_or_countered", "Payer 2 pour ne pas être contré ?"),
        ("play_choice", "Comment jouer cette carte ?"),
        ("play_extra", "Payer un coût supplémentaire ?"),
        ("predict_recycle", "Recycler les cartes regardées ?"),
        ("predict_top", "Quelle carte remettre sur le dessus ?"),
        ("profiteer", "Quelle unité déplacer, et vers laquelle ?"),
        ("promising_pick", "Quelle carte jouer ?"),
        ("promising_play", "Comment jouer cette carte ?"),
        ("pursuit_attach", "Quel équipement attacher ?"),
        ("qiyana_mode", "Piocher 1 ou canaliser 1 rune ?"),
        ("ready_gear", "Quel équipement préparer ?"),
        ("ready_pick", "Que préparer ?"),
        ("ready_rune", "Quelle rune préparer ?"),
        ("ready_target", "Quelle unité préparer ?"),
        ("reaver_move", "Quelle unité déplacer ?"),
        ("recycle_from_trash", "Quelle carte de la défausse recycler ?"),
        ("recycle_pick", "Quelle carte de la défausse recycler ?"),
        ("recycle_trash", "Quelle carte de la défausse recycler ?"),
        ("reinforce_pick", "Quelle unité jouer ?"),
        ("reksai_choice", "Où jouer cette carte ?"),
        ("reksai_pick", "Quelle carte révélée jouer ?"),
        ("reksai_play", "Quelle carte révélée jouer ?"),
        ("rell_pick", "Quel équipement de ta main jouer et attacher ?"),
        ("rengar_target", "Quelle unité cibler ?"),
        ("resurrect_pick", "Quelle unité de ta défausse ramener ?"),
        ("return_gear", "Quel équipement renvoyer en main ?"),
        ("return_pick", "Quelle carte renvoyer en main ?"),
        ("rites_pick", "Quelle unité de ta défausse jouer ?"),
        ("rumble_pick", "Quelles cartes choisir (Mech à recycler, Mech à reprendre) ?"),
        ("rush_pick", "Quelle carte prendre en main ?"),
        ("sabotage", "Quelle carte révélée recycler ?"),
        ("sacrifice_gear", "Quel équipement sacrifier ?"),
        ("shakedown_let_draw", "Laisser l'adversaire piocher ?"),
        ("skewer_pick", "Quelle unité jouer sur ce champ de bataille ?"),
        ("soulgorger_pick", "Quelle carte de ta défausse jouer ?"),
        ("spend_buff", "Quelle unité dépense son buff ?"),
        ("split_damage", "Comment répartir les dégâts ?"),
        ("split_keep", "Entre quelles unités répartir les dégâts ?"),
        ("stacked_pick", "Quelle carte prendre en main ?"),
        ("starhound_pick", "Quelle carte de ta défausse reprendre en main ?"),
        ("token_location", "Où créer le jeton ?"),
        ("trash_gear", "Quel équipement mettre à la défausse ?"),
        ("trash_spell", "Quel sort de ta défausse reprendre ?"),
        ("trash_unit", "Quelle unité de ta défausse reprendre ?"),
        ("tricksy_subset", "Quelles unités choisir ?"),
        ("veiled_detach", "Détacher cet équipement ?"),
        ("verdict_place", "Sur le dessus ou sous le deck ?"),
        ("void_rush_pick", "Quelle carte jouer ?"),
        ("weaponmaster_pick", "Quel équipement attacher ?"),
        ("whirlwind_pick", "Quelle unité renvoyer dans la main de son propriétaire ?"),
        ("wild_claw", "Quelle unité jouer ?"),
    ]);
}

/// Cartes enregistrées par cards.py hors du pool : runes (Fury, Calm, Mind, Order, Body, Chaos) et « Gold // Buff »
/// (impression double face du jeton Gold, Game.golds).
pub fn register_core_cards(r: &mut Registry) {
    for n in ["Fury Rune", "Calm Rune", "Mind Rune", "Order Rune", "Body Rune", "Chaos Rune", "Gold // Buff"] {
        r.card(Impl::new(n));
    }
}
