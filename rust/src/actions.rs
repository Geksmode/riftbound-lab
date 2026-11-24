//! Actions discrétionnaires légales (410) et processus de jeu d'une carte ou d'une capacité (353-359, 398-406) :
//! `actions.py`.
//!
//! Actions (tuples Python, `Value::Tuple`) : `('end',)`, `('pass',)`, `('play', uid, src, choice)`,
//! `('hide', uid, bf)` ou `('hide', uid, bf, (e, reqs))`, `('act', src_uid | ('legend', pid), i, choice)`,
//! `('move', (uids...), dest)`. uid et bf en `Value::Int`, src en `Value::Str`, choice en `Value::Map`, dest et
//! `choice['loc']` en `Value::Loc`.
//!
//! Choix modifiés par un coût : en Python, `Impl.pay_extra(g, pid, card, ch)`, `opt_pay(g, src, pid, card, ch)` et
//! `ab['extra_cost'](g, pid, obj, ch)` peuvent écrire dans `ch`. En Rust le choix est passé par valeur : la fonction
//! renvoie `Value::Map(ch modifié)` pour le remplacer (toute autre valeur le laisse tel quel).
//!
//! `_ids = count(1)` de actions.py n'est utilisé nulle part : rien à porter.
use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::cards::{registry, Impl};
use crate::game::{Game, ItemId, Res, Zone, O};
use crate::game_board::loc_str;
use crate::game_costs::{any_req, reqs_of, reqs_to_value, Req};
use crate::spec::BARON_PIT;
use crate::value::{is_bf, Loc, Map, Value};

/// `MAX_CHOICES`
pub const MAX_CHOICES: usize = 8;
/// `REPEAT_KEYS`
pub const REPEAT_KEYS: [&str; 5] = ["rep", "reps", "rep2", "sq", "tg2"];

/// `full_choices(g, pid)` : tous les choix légaux doivent être proposés (joueur humain, ou `g.every_choice`).
pub fn full_choices(g: &Game, pid: Option<usize>) -> bool {
    if g.every_choice {
        return true;
    }
    match pid {
        Some(p) => g.agents.get(p).is_some_and(|a| a.every_choice()),
        None => false,
    }
}

// ---------------------------------------------------------------------------------------------- petites aides
/// uid d'une valeur (`isinstance(uid, int)`).
pub fn as_uid(v: &Value) -> Option<u32> {
    match v {
        Value::Int(i) => Some(*i as u32),
        _ => None,
    }
}

/// `dict(base, **kv)`
pub fn dict_with(base: &Map, kv: &[(&str, Value)]) -> Map {
    let mut m = base.clone();
    for (k, v) in kv {
        m.set(k, v.clone());
    }
    m
}

/// `tuple(reqs)` : tuple de listes de domaines.
pub fn reqs_tuple(reqs: &[Req]) -> Value {
    Value::Tuple(reqs.iter().map(|r| Value::List(r.iter().map(|d| Value::from(d.as_str())).collect())).collect())
}

/// `(e, list(reqs))` rendu par une fonction de coût.
pub fn cost_value(e: i64, reqs: &[Req]) -> Value {
    Value::Tuple(vec![e.into(), reqs_to_value(reqs)])
}

/// Lit `(e, reqs)` rendu par une fonction de coût.
pub fn cost_of(v: &Value) -> (i64, Vec<Req>) {
    let l = v.list();
    (l[0].int(), reqs_of(&l[1]))
}

/// Un emplacement rangé dans une valeur : `Value::Loc`, un indice de battlefield (`Int`) ou 'base' ; None sinon.
pub fn loc_of(v: &Value) -> Option<Loc> {
    match v {
        Value::Loc(l) => Some(*l),
        Value::Int(i) => Some(Loc::Bf(*i as usize)),
        Value::Str(s) if &**s == "base" => Some(Loc::Base),
        _ => None,
    }
}

fn loc_v(l: Loc) -> Value {
    Value::Loc(l)
}

fn maps(v: &Value) -> Vec<Map> {
    v.list().iter().map(|x| x.map().clone()).collect()
}

/// `card_obj(g, pid, uid, src)`
pub fn card_obj(g: &Game, pid: usize, uid: u32, src: &str) -> Option<O> {
    let pl = &g.p[pid];
    if src == "facedown" {
        for b in &g.bfs {
            for &c in &b.facedowns {
                if g.o(c).uid == uid {
                    return Some(c);
                }
            }
        }
        return None;
    }
    let zone: &[O] = match src {
        "hand" => &pl.hand,
        "champ" => &pl.champ,
        "trash" => &pl.trash,
        "banish" => &pl.banish,
        _ => &[],
    };
    zone.iter().copied().find(|&c| g.o(c).uid == uid)
}

// ---------------------------------------------------------------------------------------------- timing
/// `has_timing(g, pid, im, src, closed, loc_ok_ambush)` : jouable maintenant ? closed : état Closed (Reaction).
pub fn has_timing(im: &Impl, src: &str, closed: bool, loc_ok_ambush: bool) -> bool {
    if src == "facedown" {
        return true; // les cartes cachées ont [Reaction] (811.6)
    }
    let t = if im.quickdraw { Some("reaction") } else { im.timing };
    if closed {
        return t == Some("reaction") || loc_ok_ambush;
    }
    matches!(t, Some("action") | Some("reaction")) || loc_ok_ambush
}

/// `state_neutral_open_main(g, pid)`
pub fn state_neutral_open_main(g: &Game, pid: usize) -> bool {
    g.chain.is_empty() && g.sd.is_none() && g.stage == "main" && g.tp == pid
}

// ---------------------------------------------------------------------------------------------- coût
/// `flow_of(g, pid, card)` : [Flow] imprimé (Impl.flow) ou donné ce tour (effet 'grant_flow' uid/oid).
pub fn flow_of(g: &Game, _pid: usize, card: O) -> Value {
    if let Some(im) = g.impl_o(card) {
        if im.flow.truthy() {
            return im.flow.clone();
        }
    }
    let (uid, oid) = (Value::from(g.o(card).uid), Value::from(g.o(card).oid));
    for ef in &g.effects {
        if ef.d.g("kind").opt_str() == Some("grant_flow") && ef.d.g("uid") == uid && ef.d.g("oid") == oid {
            return ef.d.g("flow");
        }
    }
    Value::None
}

/// `card_kw(g, pid, card, kw, src)` : mots-clés d'une carte hors du plateau, imprimés ou donnés (card_kw des autres
/// objets, effets 'card_kw').
pub fn card_kw(g: &mut Game, pid: usize, card: O, kw: &str, src: Option<&str>) -> Res<bool> {
    let Some(im) = g.impl_o(card) else { return Ok(false) };
    let own = match kw {
        "Quick-Draw" => Some(im.quickdraw),
        "Accelerate" => Some(im.accelerate),
        "Hidden" => Some(im.hidden),
        "Ambush" => Some(im.ambush),
        _ => None,
    };
    if own == Some(true) {
        return Ok(true);
    }
    for (so, ia) in g.providers("card_kw") {
        if ia.hook("card_kw").call(g, &[so, pid.into(), card.into(), kw.into(), src.into()])?.truthy() {
            return Ok(true);
        }
    }
    for ef in g.effects.clone() {
        if ef.d.g("kind").opt_str() == Some("card_kw")
            && ef.d.g("pid") == Value::from(pid)
            && ef.d.g("fn").call(g, &[Value::Int(ef.eid as i64), card.into(), kw.into(), src.into()])?.truthy()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `eff_timing(g, pid, card, im, src)` : timing avec [Action] / [Reaction] donnés.
pub fn eff_timing(g: &mut Game, pid: usize, card: O, im: &Impl, src: Option<&str>) -> Res<Option<&'static str>> {
    let t = im.timing;
    if t != Some("reaction") && card_kw(g, pid, card, "Reaction", src)? {
        return Ok(Some("reaction"));
    }
    if t.is_none() && card_kw(g, pid, card, "Action", src)? {
        return Ok(Some("action"));
    }
    Ok(t)
}

/// `has_deploy(g, card)` (830)
pub fn has_deploy(g: &mut Game, card: O) -> Res<bool> {
    let Some(im) = g.impl_o(card) else { return Ok(false) };
    Ok(im.kw("Deploy") > 0 || g.has_kw(card, "Deploy")?)
}

/// `gear_locations(g, pid, card, hidden_bf)` (830.1.b.1)
pub fn gear_locations(g: &mut Game, pid: usize, card: O, hidden_bf: Option<Loc>) -> Res<Vec<Loc>> {
    if let Some(hb) = hidden_bf {
        return Ok(vec![hb]);
    }
    if has_deploy(g, card)? {
        let mut out = vec![];
        for b in g.bf_locs() {
            if loc_allowed(g, pid, card, b)? {
                out.push(b);
            }
        }
        return Ok(out);
    }
    Ok(vec![Loc::Base])
}

/// `show_options(g, pid, card, im)` : [Show Off] (832).
pub fn show_options(g: &mut Game, pid: usize, card: O, im: Option<&Impl>) -> Res<Vec<Map>> {
    let Some(im) = im else { return Ok(vec![Map::new()]) };
    if im.show_off.is_none() {
        return Ok(vec![Map::new()]);
    }
    let mut cands = vec![];
    for c in g.p[pid].hand.clone() {
        if c != card && im.show_off.call(g, &[pid.into(), card.into(), c.into()])?.truthy() {
            cands.push(c);
        }
    }
    for o in g.board.clone() {
        if g.o(o).ctrl == pid && o != card && im.show_off.call(g, &[pid.into(), card.into(), o.into()])?.truthy() {
            cands.push(o);
        }
    }
    let mut keyed = vec![];
    for &x in &cands {
        let k = if im.show_key.truthy() {
            im.show_key.call(g, &[x.into()])?
        } else {
            let sp = g.spec(x);
            Value::Int(-(sp.e as i64 + 2 * sp.p as i64 + sp.might.unwrap_or(0) as i64))
        };
        keyed.push((k, g.o(x).uid, x));
    }
    keyed.sort_by(|a, b| py_cmp(&a.0, &b.0).then(a.1.cmp(&b.1)));
    let mut cands: Vec<O> = keyed.into_iter().map(|k| k.2).collect();
    if !full_choices(g, Some(pid)) {
        cands.truncate(1);
    }
    let mut out: Vec<Map> = cands.iter().map(|&x| crate::map! { show = g.o(x).uid }).collect();
    out.push(Map::new());
    Ok(out)
}

/// `shown_info(g, pid, uid)` (832.1.d.1)
pub fn shown_info(g: &mut Game, pid: usize, uid: u32) -> Res<Value> {
    let mut o = g.obj(uid);
    let mut where_ = "board";
    if o.is_none() {
        o = g.p[pid].hand.iter().copied().find(|&c| g.o(c).uid == uid);
        where_ = "hand";
    }
    let Some(o) = o else { return Ok(Value::None) };
    let sp = g.spec(o);
    let might = if where_ == "board" && sp.type_ == "Unit" { g.might(o)? } else { sp.might.unwrap_or(0) as i64 };
    let mut m = Map::new();
    m.set("uid", g.o(o).uid);
    m.set("name", g.cname(o));
    m.set("e", sp.e);
    m.set("p", sp.p);
    m.set("type", sp.type_.as_str());
    m.set("where", where_);
    m.set("might", might);
    Ok(m.into())
}

/// `repeat_instances(g, pid, card)` : instances de [Repeat] données (820.1.c.2) : [(clé, e, reqs)].
pub fn repeat_instances(g: &mut Game, pid: usize, card: O) -> Res<Vec<(Value, i64, Vec<Req>)>> {
    let mut out = vec![];
    for (i, ef) in g.effects.clone().into_iter().enumerate() {
        if ef.d.g("kind").opt_str() == Some("grant_repeat") && ef.d.g("pid") == Value::from(pid) {
            let c = ef.d.g("cost").call(g, &[card.into()])?;
            if !c.is_none() {
                let key = ef.d.get("key").cloned().unwrap_or(Value::Int(i as i64));
                let (e, r) = cost_of(&c);
                out.push((Value::Tuple(vec!["eff".into(), key]), e, r));
            }
        }
    }
    for (so, ia) in g.providers("card_kw") {
        let c = ia.hook("card_kw").call(g, &[so.clone(), pid.into(), card.into(), "Repeat".into(), Value::None])?;
        if c.truthy() {
            // getattr(so, "uid", 0), so.name : un battlefield n'a pas d'uid
            let (u, n): (Value, Value) = match &so {
                Value::Obj(o) => (g.o(*o).uid.into(), g.o(*o).name.as_str().into()),
                Value::Int(b) => (0.into(), g.bfs[*b as usize].name.as_str().into()),
                _ => (0.into(), "".into()),
            };
            let (e, r) = cost_of(&c);
            out.push((Value::Tuple(vec!["src".into(), u, n]), e, r));
        }
    }
    Ok(out)
}

fn doms_or_any(doms: &[String]) -> Req {
    if doms.is_empty() {
        any_req()
    } else {
        doms.iter().cloned().collect()
    }
}

/// `_opt_parts(g, pid, card, im, choice, base_e, base_reqs)` : coûts additionnels optionnels payés avec ce choix.
fn opt_parts(g: &mut Game, pid: usize, card: O, im: &Impl, choice: &Map) -> Res<Vec<(String, i64, Vec<Req>)>> {
    if !im.opt_parts.is_none() {
        let r = im.opt_parts.call(g, &[pid.into(), card.into(), choice.clone().into()])?;
        return Ok(r
            .list()
            .iter()
            .map(|t| {
                let t = t.list();
                (t[0].str().to_string(), t[1].int(), reqs_of(&t[2]))
            })
            .collect());
    }
    let mut parts = vec![];
    if choice.g("acc").truthy() {
        parts.push(("acc".to_string(), 1, vec![doms_or_any(&g.spec(card).domains)]));
    }
    if choice.g("rep").truthy() && im.repeat.truthy() {
        let r = im.repeat.list();
        let doms: Vec<String> = r[1].list().iter().map(|d| d.str().to_string()).collect();
        parts.push(("rep".to_string(), r[0].int(), vec![doms_or_any(&doms); r[2].int() as usize]));
    }
    if !im.extra_cost_fn.is_none() {
        let x = im.extra_cost_fn.call(g, &[pid.into(), card.into(), choice.clone().into()])?;
        let x = x.list();
        if x[0].truthy() || x[1].truthy() {
            parts.push(("opt".to_string(), x[0].int(), reqs_of(&x[1])));
        }
    }
    Ok(parts)
}

/// `cost_mods(g, pid, what)` (356.3-356.4) : modificateurs de coût des autres objets (cost_aura) et des effets
/// (kind 'cost_mod', l'entrée porte `_eff` = eid de l'effet).
pub fn cost_mods(g: &mut Game, pid: usize, what: &Map) -> Res<Vec<Map>> {
    let mut out = vec![];
    for (so, ia) in g.providers("cost_aura") {
        let r = ia.hook("cost_aura").call(g, &[so, pid.into(), what.clone().into()])?;
        out.extend(maps(&r));
    }
    for ef in g.effects.clone() {
        let pid_ok = match ef.d.get("pid") {
            None => true,
            Some(v) => *v == Value::from(pid),
        };
        if ef.d.g("kind").opt_str() == Some("cost_mod") && pid_ok {
            let r = ef.d.g("fn").call(g, &[Value::Int(ef.eid as i64), pid.into(), what.clone().into()])?;
            for mut m in maps(&r) {
                m.set("_eff", Value::Int(ef.eid as i64));
                out.push(m);
            }
        }
    }
    Ok(out)
}

/// `_drop_reqs(reqs, n)` : retire n powers, les plus restrictifs d'abord (clé (len(r), sorted(r))).
pub fn drop_reqs(reqs: &[Req], n: usize) -> Vec<Req> {
    let mut r = reqs.to_vec();
    r.sort_by(|a, b| (a.len(), a).cmp(&(b.len(), b)));
    r.into_iter().skip(n).collect()
}

/// `_flex(variants)` : « 1 énergie ou 1 rune de n'importe quel type de moins » (énergie d'abord).
pub fn flex(variants: &[(i64, Vec<Req>)]) -> Vec<(i64, Vec<Req>)> {
    let mut out: Vec<(i64, Vec<Req>)> = vec![];
    for (ve, vr) in variants {
        let mut nv = vec![];
        if *ve > 0 {
            nv.push((ve - 1, vr.clone()));
        }
        if !vr.is_empty() {
            nv.push((*ve, drop_reqs(vr, 1)));
        }
        if nv.is_empty() {
            nv.push((*ve, vr.clone()));
        }
        for x in nv {
            if !out.contains(&x) {
                out.push(x);
            }
        }
    }
    out
}

fn m_int(m: &Map, k: &str) -> i64 {
    m.g(k).opt_int().unwrap_or(0)
}

fn m_on(m: &Map) -> String {
    match m.get("on") {
        None => "all".into(),
        Some(v) => v.opt_str().unwrap_or("").to_string(),
    }
}

/// `_apply_mods(g, pid, e, reqs, parts, mods, ctx)` : augmentations puis réductions (356.3-356.4). Quand une
/// réduction laisse choisir énergie ou rune, la première variante payable est prise (énergie d'abord).
fn apply_mods(
    g: &mut Game,
    pid: usize,
    e: i64,
    reqs: &[Req],
    parts: &[(String, i64, Vec<Req>)],
    mods: &[Map],
    ctx: &Value,
) -> Res<(i64, Vec<Req>)> {
    let mut reqs = reqs.to_vec();
    let mut e = e;
    let whole: Vec<&Map> = mods.iter().filter(|m| m_on(m) == "all").collect();
    for m in &whole {
        // augmentations d'abord (356.3)
        if m_int(m, "e") > 0 {
            e += m_int(m, "e");
        }
        reqs.extend(reqs_of(&m.g("p")));
    }
    let mut variants = vec![(e, reqs)];
    for (tag, pe, pr) in parts {
        // réductions de chaque coût optionnel / de Repeat
        let mut pv = vec![(*pe, pr.clone())];
        for m in mods {
            let on = m_on(m);
            if on == "all" || (on == "rep" && tag != "rep") || tag == "must" {
                continue;
            }
            if m_int(m, "e") < 0 {
                pv = pv.into_iter().map(|(a, b)| (0.max(a + m_int(m, "e")), b)).collect();
            }
            if m.g("rm").truthy() {
                pv = pv.into_iter().map(|(a, b)| (a, drop_reqs(&b, m_int(m, "rm") as usize))).collect();
            }
            for _ in 0..m_int(m, "flex") {
                pv = flex(&pv);
            }
        }
        let mut nv = vec![];
        for (ve, vr) in &variants {
            for (a, b) in &pv {
                let mut r = vr.clone();
                r.extend(b.iter().cloned());
                nv.push((ve + a, r));
            }
        }
        variants = nv;
    }
    for m in &whole {
        // réductions du coût entier (356.4)
        let me = m_int(m, "e");
        if me < 0 {
            let floor = m.g("min_e").opt_int();
            variants = variants
                .into_iter()
                .map(|(ve, vr)| {
                    (
                        match floor {
                            Some(f) => (ve + me).max(ve.min(f)),
                            None => 0.max(ve + me),
                        },
                        vr,
                    )
                })
                .collect();
        }
        if m.g("rm").truthy() {
            variants = variants.into_iter().map(|(ve, vr)| (ve, drop_reqs(&vr, m_int(m, "rm") as usize))).collect();
        }
        for _ in 0..m_int(m, "flex") {
            variants = flex(&variants);
        }
    }
    if variants.len() > 1 {
        for (ve, vr) in &variants {
            if g.can_pay(pid, *ve, vr, ctx)? {
                return Ok((*ve, vr.clone()));
            }
        }
    }
    Ok(variants.swap_remove(0))
}

/// `total_cost(g, pid, card, choice, src)` (356) : coût de base (modifié), coûts additionnels, augmentations,
/// réductions.
pub fn total_cost(g: &mut Game, pid: usize, card: O, choice: &Map, src: &str) -> Res<(i64, Vec<Req>)> {
    let im = g.impl_o(card).expect("total_cost : carte sans Impl");
    let sp = g.spec(card);
    let (mut e, mut p) = (sp.e as i64, sp.p as i64);
    let mut doms = sp.domains.clone();
    let mut alt = false;
    if src == "facedown" || choice.g("free").truthy() {
        e = 0; // Hidden : ignore son coût de base
        p = 0;
    } else if !choice.g("alt").is_none() {
        alt = true; // coût alternatif (356.1.b) à la place du coût de base
        e = choice.g("alt_e").int();
        p = 0;
    } else if choice.g("flow").truthy() {
        let fl = flow_of(g, pid, card);
        let fl = fl.list();
        e = fl[0].int();
        p = fl[1].int();
        if fl.len() > 2 {
            doms = fl[2].list().iter().map(|d| d.str().to_string()).collect();
        }
    }
    if choice.g("ignore_energy").truthy() {
        e = 0;
    }
    if im.cost_mod.truthy() && src != "facedown" && !choice.g("free").truthy() {
        let r = im.cost_mod.call(g, &[pid.into(), card.into(), choice.clone().into()])?;
        let r = r.list();
        e = 0.max(e - r[0].int());
        p = 0.max(p - r[1].int());
    }
    let mut reqs = g.power_reqs(&doms, p.max(0) as usize);
    if alt {
        reqs = reqs_of(&choice.g("alt_reqs"));
    }
    // coûts additionnels optionnels (Accelerate, Repeat, « you may pay ... as an additional cost »)
    let mut parts = opt_parts(g, pid, card, im, choice)?;
    for r in choice.g("reps").list() {
        let r = r.map();
        parts.push(("rep".into(), r.g("_e").int(), reqs_of(&r.g("_reqs")))); // instances de Repeat données payées
    }
    // coûts additionnels obligatoires du lieu (Dragon Roost) et Deflect (809)
    if choice.g("loc_reqs").truthy() {
        reqs.extend(reqs_of(&choice.g("loc_reqs")));
    }
    reqs.extend(deflect_total(g, pid, card, choice)?);
    let mods = if !g.effects.is_empty() || registry().hook_names("cost_aura").is_some() {
        let mut what = Map::new();
        what.set("kind", sp.type_.to_lowercase());
        what.set("card", card);
        what.set("obj", Value::None);
        what.set("ab", Value::None);
        what.set("choice", choice.clone());
        what.set("src", src);
        cost_mods(g, pid, &what)?
    } else {
        vec![]
    };
    let ctx = pay_ctx(g, card);
    let (mut e, mut reqs) = apply_mods(g, pid, e, &reqs, &parts, &mods, &ctx)?;
    // Astral Heron : « your next card costs [2] and [A][A] less »
    let pv = Value::from(pid);
    let n = g.effects.iter().filter(|ef| ef.d.g("kind").opt_str() == Some("heron") && ef.d.g("pid") == pv).count();
    for _ in 0..n {
        e = 0.max(e - 2);
        for _ in 0..2 {
            if !reqs.is_empty() {
                // le power le plus restrictif d'abord
                reqs.sort_by_key(|r| r.len());
                reqs.remove(0);
            }
        }
    }
    Ok((e, reqs))
}

/// `deflect_reqs(g, pid, choice)` (809) : [A] par valeur de Deflect de chaque objet ennemi choisi dans choice['tg']
/// (sauf à Heisho, Shell of the World).
pub fn deflect_reqs(g: &mut Game, pid: usize, choice: &Map) -> Res<Vec<Req>> {
    let mut out = vec![];
    for uid in choice.g("tg").list() {
        if let Some(o) = as_uid(uid).and_then(|u| g.obj(u)) {
            if g.o(o).ctrl != pid {
                if let Some(Loc::Bf(b)) = g.o(o).loc {
                    if g.bfs[b].name == "Heisho, Shell of the World" {
                        continue;
                    }
                }
                let n = g.kw_value(o, "Deflect")?;
                for _ in 0..n {
                    out.push(any_req());
                }
            }
        }
    }
    Ok(out)
}

/// `deflect_total(g, pid, card, choice)` (809.1.c) : cibles du sort puis de chaque répétition payée.
pub fn deflect_total(g: &mut Game, pid: usize, card: O, choice: &Map) -> Res<Vec<Req>> {
    if g.impl_o(card).is_some_and(|im| im.ignore_deflect) {
        return Ok(vec![]);
    }
    let mut out = deflect_reqs(g, pid, choice)?;
    if choice.g("rep").truthy() {
        let tg2 = choice.get("tg2").cloned().unwrap_or(Value::Tuple(vec![]));
        out.extend(deflect_reqs(g, pid, &crate::map! { tg = tg2 })?);
    }
    for r in choice.g("reps").list() {
        let tg = r.map().get("tg").cloned().unwrap_or(Value::Tuple(vec![]));
        out.extend(deflect_reqs(g, pid, &crate::map! { tg = tg })?);
    }
    Ok(out)
}

/// `pay_ctx(card)` : dict(kind=, card=)
pub fn pay_ctx(g: &Game, card: O) -> Value {
    crate::map! { kind = g.spec(card).type_.to_lowercase(), card = card }.into()
}

/// `affordable(g, pid, card, choice, src)`
pub fn affordable(g: &mut Game, pid: usize, card: O, choice: &Map, src: &str) -> Res<bool> {
    let (e, reqs) = total_cost(g, pid, card, choice, src)?;
    let ctx = pay_ctx(g, card);
    g.can_pay(pid, e, &reqs, &ctx)
}

/// `unit_timing_ok(im, closed, timing)` (806.3, 813.3.a) : timing = timing effectif (`im.timing` par défaut).
pub fn unit_timing_ok(timing: Option<&str>, closed: bool) -> bool {
    timing == Some("reaction") || (timing == Some("action") && !closed)
}

// ---------------------------------------------------------------------------------------------- énumération
/// `loc_allowed(g, pid, card, loc)` : only_locs de la carte, loc_veto des autres objets et des effets.
pub fn loc_allowed(g: &mut Game, pid: usize, card: O, loc: Loc) -> Res<bool> {
    if let Some(im) = g.impl_o(card) {
        let ol = im.hook("only_locs");
        if !ol.is_none() && !ol.call(g, &[pid.into(), card.into()])?.list().iter().any(|x| loc_of(x) == Some(loc)) {
            return Ok(false);
        }
    }
    for (so, ia) in g.providers("loc_veto") {
        if ia.hook("loc_veto").call(g, &[so, pid.into(), card.into(), loc_v(loc)])?.truthy() {
            return Ok(false);
        }
    }
    for ef in g.effects.clone() {
        if ef.d.g("kind").opt_str() == Some("loc_veto")
            && ef.d.g("fn").call(g, &[Value::Int(ef.eid as i64), pid.into(), card.into(), loc_v(loc)])?.truthy()
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `unit_locations(g, pid, card, src, closed_or_focus, with_extra=True)` (355.2) : [(loc, choix en plus)].
pub fn unit_locations_extra(g: &mut Game, pid: usize, card: O, src: &str, closed_or_focus: bool) -> Res<Vec<(Loc, Map)>> {
    let im = g.impl_o(card).expect("unit_locations : carte sans Impl");
    let mut locs: Vec<(Loc, Map)> = vec![];
    if src == "facedown" {
        locs.push((g.o(card).hidden_bf.expect("carte cachée sans battlefield"), Map::new()));
    } else {
        if !closed_or_focus {
            locs.push((Loc::Base, Map::new()));
            for b in &g.bfs {
                if b.ctrl == Some(pid) {
                    locs.push((Loc::Bf(b.idx), Map::new()));
                }
            }
        }
        if im.ambush || card_kw(g, pid, card, "Ambush", Some(src))? {
            for b in 0..g.bfs.len() {
                let e = (Loc::Bf(b), Map::new());
                if !locs.contains(&e) && g.units(None, Some(Loc::Bf(b))).iter().any(|&u| g.o(u).ctrl == pid) {
                    locs.push(e);
                }
            }
        }
        let el = im.hook("extra_locs");
        if !el.is_none() {
            for l in el.call(g, &[pid.into(), card.into(), closed_or_focus.into()])?.list() {
                let e = (loc_of(l).expect("emplacement attendu"), Map::new());
                if !locs.contains(&e) {
                    locs.push(e);
                }
            }
        }
        for (so, ia) in g.providers("aura_locs") {
            let r = ia.hook("aura_locs").call(g, &[so, pid.into(), card.into(), closed_or_focus.into()])?;
            for l in r.list() {
                let e = match l {
                    Value::Tuple(t) => (loc_of(&t[0]).expect("emplacement attendu"), t[1].map().clone()),
                    v => (loc_of(v).expect("emplacement attendu"), Map::new()),
                };
                if !locs.contains(&e) {
                    locs.push(e);
                }
            }
        }
        for ef in g.effects.clone() {
            // Last Caress : « The next unit you play this turn can be played to any battlefield. »
            if ef.d.g("kind").opt_str() == Some("aura_locs") && ef.d.g("pid") == Value::from(pid) {
                let r = ef.d.g("fn").call(g, &[Value::Int(ef.eid as i64), card.into(), closed_or_focus.into()])?;
                for l in r.list() {
                    let e = (loc_of(l).expect("emplacement attendu"), Map::new());
                    if !locs.contains(&e) {
                        locs.push(e);
                    }
                }
            }
        }
    }
    let mut out = vec![];
    for (l, ex) in locs {
        if loc_allowed(g, pid, card, l)? {
            out.push((l, ex));
        }
    }
    Ok(out)
}

/// `unit_locations(g, pid, card, src, closed_or_focus)` : les emplacements sans choix en plus.
pub fn unit_locations(g: &mut Game, pid: usize, card: O, src: &str, closed_or_focus: bool) -> Res<Vec<Loc>> {
    Ok(unit_locations_extra(g, pid, card, src, closed_or_focus)?
        .into_iter()
        .filter(|(_, ex)| ex.0.is_empty())
        .map(|x| x.0)
        .collect())
}

/// `open_bf(g, b)` : battlefield ouvert, sans unité ni contrôleur (355.2.b).
pub fn open_bf(g: &Game, b: usize) -> bool {
    g.bfs[b].ctrl.is_none() && g.units(None, Some(Loc::Bf(b))).is_empty()
}

/// `play_forbidden(g, pid, card, src)` : « can't play » (forbid_play des autres objets, effets 'no_play').
pub fn play_forbidden(g: &mut Game, pid: usize, card: O, src: &str) -> Res<bool> {
    for (so, ia) in g.providers("forbid_play") {
        if ia.hook("forbid_play").call(g, &[so, pid.into(), card.into(), src.into()])?.truthy() {
            return Ok(true);
        }
    }
    for ef in g.effects.clone() {
        if ef.d.g("kind").opt_str() == Some("no_play")
            && ef.d.g("fn").call(g, &[Value::Int(ef.eid as i64), pid.into(), card.into(), src.into()])?.truthy()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `card_choices(g, pid, card, src, timing_closed, timing_showdown, every)` : les choix de jeu d'une carte (355),
/// déjà filtrés par le coût. every : tous les choix légaux (joueur humain).
pub fn card_choices(
    g: &mut Game,
    pid: usize,
    card: O,
    src: &str,
    timing_closed: bool,
    timing_showdown: bool,
    every: bool,
) -> Res<Vec<Map>> {
    if !every {
        let fc = full_choices(g, Some(pid));
        return card_choices_inner(g, pid, card, src, timing_closed, timing_showdown, fc);
    }
    let prev = std::mem::replace(&mut g.every_choice, true);
    let r = card_choices_inner(g, pid, card, src, timing_closed, timing_showdown, true);
    g.every_choice = prev;
    r
}

fn as_played_of(g: &mut Game, im: &Impl, pid: usize, card: O) -> Res<Vec<Map>> {
    Ok(if im.as_played.truthy() { maps(&im.as_played.call(g, &[pid.into(), card.into()])?) } else { vec![Map::new()] })
}

fn card_choices_inner(
    g: &mut Game,
    pid: usize,
    card: O,
    src: &str,
    timing_closed: bool,
    timing_showdown: bool,
    every: bool,
) -> Res<Vec<Map>> {
    let Some(im) = g.impl_o(card) else { return Ok(vec![]) };
    if play_forbidden(g, pid, card, src)? {
        return Ok(vec![]);
    }
    let typ = g.spec(card).type_.as_str();
    let restricted = timing_closed || timing_showdown;
    let mut out: Vec<Map> = vec![];
    let hidden_bf = if src == "facedown" { g.o(card).hidden_bf } else { None };
    let qd = typ == "Gear" && card_kw(g, pid, card, "Quick-Draw", Some(src))?;
    let shows = show_options(g, pid, card, Some(im))?;
    let timing = eff_timing(g, pid, card, im, Some(src))?;
    if typ == "Unit" {
        let timed = unit_timing_ok(timing, timing_closed);
        let amb = im.ambush || card_kw(g, pid, card, "Ambush", Some(src))?;
        if restricted && src != "facedown" && !amb && !timed {
            return Ok(vec![]);
        }
        let locs = unit_locations_extra(g, pid, card, src, restricted && src != "facedown" && !timed)?;
        let extra = as_played_of(g, im, pid, card)?;
        let acc = card_kw(g, pid, card, "Accelerate", Some(src))? && src != "facedown";
        let accs: &[bool] = if acc { &[false, true] } else { &[false] };
        for (loc, lx) in &locs {
            for ex in &extra {
                for sh in &shows {
                    for &a in accs {
                        let mut m = dict_with(ex, &[("loc", loc_v(*loc)), ("acc", a.into())]);
                        for (k, v) in lx.0.iter().chain(sh.0.iter()) {
                            m.set(k, v.clone());
                        }
                        out.push(m);
                    }
                }
            }
        }
    } else if typ == "Gear" {
        if restricted && !(qd || src == "facedown" || unit_timing_ok(timing, timing_closed)) {
            return Ok(vec![]);
        }
        let extra = as_played_of(g, im, pid, card)?;
        for loc in gear_locations(g, pid, card, hidden_bf)? {
            for ex in &extra {
                for sh in &shows {
                    let mut m = dict_with(ex, &[("loc", loc_v(loc))]);
                    for (k, v) in &sh.0 {
                        m.set(k, v.clone());
                    }
                    if src == "trash" && flow_of(g, pid, card).truthy() {
                        m.set("flow", true);
                    }
                    out.push(m);
                }
            }
        }
    } else if typ == "Spell" {
        if restricted && src != "facedown" && !matches!(timing, Some("action") | Some("reaction")) {
            return Ok(vec![]);
        }
        if timing_closed && src != "facedown" && timing != Some("reaction") {
            return Ok(vec![]);
        }
        let ctx = crate::map! { hidden_bf = hidden_bf, card = card, src = src };
        let mut base: Vec<Map> = if every && !im.all_choices.is_none() {
            maps(&im.all_choices.call(g, &[pid.into(), ctx.clone().into()])?)
        } else {
            let b: Vec<Map> =
                if im.choices.truthy() { maps(&im.choices.call(g, &[pid.into(), ctx.clone().into()])?) } else { vec![Map::new()] };
            if every {
                b
            } else {
                b.into_iter().take(MAX_CHOICES).collect()
            }
        };
        if src == "trash" && !flow_of(g, pid, card).is_none() {
            base = base.iter().map(|ch| dict_with(ch, &[("flow", true.into())])).collect();
        }
        if shows.len() > 1 {
            let mut nb = vec![];
            for ch in &base {
                for sh in &shows {
                    nb.push(ch.merged(sh));
                }
            }
            base = nb;
        }
        for (so, ia) in g.providers("spell_opts") {
            // coûts additionnels optionnels donnés par d'autres objets (Seraphine, Starry-Eyed Songstress)
            let opts = ia.hook("spell_opts").call(g, &[so, pid.into(), card.into()])?;
            if opts.truthy() {
                let mut add = vec![];
                for ch in &base {
                    for o in opts.list() {
                        add.push(ch.merged(o.map()));
                    }
                }
                base.extend(add);
            }
        }
        out.extend(spell_variants(g, pid, card, im, base, &ctx, every)?);
    }
    if !g.effects.is_empty() {
        // permissions de jeu créées par des effets (kind 'play_perm' : Jayce, Man of Progress), consommées par play_card
        for ef in g.effects.clone() {
            if ef.d.g("kind").opt_str() == Some("play_perm")
                && ef.d.g("pid") == Value::from(pid)
                && ef.d.g("fn").call(g, &[Value::Int(ef.eid as i64), card.into(), src.into()])?.truthy()
            {
                let mut add = vec![];
                for ch in &out {
                    if ch.g("perm").is_none() {
                        let mut m = dict_with(ch, &[("perm", ef.d.g("key"))]);
                        for (k, v) in &ef.d.g("choice").map().0 {
                            m.set(k, v.clone());
                        }
                        add.push(m);
                    }
                }
                out.extend(add);
            }
        }
    }
    if !im.alt_costs.is_none() && src != "facedown" {
        // coûts alternatifs (Impl.alt_costs -> [dict(key=, e=, reqs=)]) ; la seconde liste part de l'ancien `out`
        let alts = im.alt_costs.call(g, &[pid.into(), card.into(), src.into()])?;
        let pft = !g.effects_of("play_from_trash", Some(pid)).is_empty();
        let mut nout: Vec<Map> = out.iter().filter(|ch| src != "trash" || ch.g("flow").truthy() || pft).cloned().collect();
        for a in alts.list() {
            let a = a.map();
            for ch in &out {
                nout.push(dict_with(
                    ch,
                    &[
                        ("flow", false.into()),
                        ("alt", a.g("key")),
                        ("alt_e", a.g("e")),
                        ("alt_reqs", Value::Tuple(a.g("reqs").list().to_vec())),
                    ],
                ));
            }
        }
        out = nout;
    } else if src == "trash" {
        let pft = !g.effects_of("play_from_trash", Some(pid)).is_empty();
        out.retain(|ch| ch.g("flow").truthy() || pft);
    }
    let mut res = vec![];
    for ch in out {
        if affordable(g, pid, card, &ch, src)? {
            res.push(ch);
        }
    }
    Ok(res)
}

/// `spell_variants(g, pid, card, im, base, ctx, every)` : les choix d'un sort avec ses coûts de Repeat optionnels :
/// [Repeat] imprimé (rep=True, cibles de la répétition dans tg2) et chaque instance donnée (reps=(choix,)).
pub fn spell_variants(
    g: &mut Game,
    pid: usize,
    card: O,
    im: &Impl,
    base: Vec<Map>,
    ctx: &Map,
    every: bool,
) -> Res<Vec<Map>> {
    let mut out = vec![];
    let grep = repeat_instances(g, pid, card)?;
    let every = every || full_choices(g, Some(pid));
    let mut second: Vec<Map> = vec![];
    if im.repeat.truthy() || !grep.is_empty() {
        let s: Vec<Map> = if every && !im.all_choices.is_none() {
            maps(&im.all_choices.call(g, &[pid.into(), ctx.clone().into()])?)
        } else if im.choices.truthy() {
            maps(&im.choices.call(g, &[pid.into(), ctx.clone().into()])?)
        } else {
            vec![]
        };
        let s = if s.is_empty() { vec![Map::new()] } else { s };
        // une répétition utilise les choix de base du sort (820.1.c.2)
        second = s.into_iter().filter(|c| !REPEAT_KEYS.iter().any(|k| c.g(k).truthy())).collect();
        if second.is_empty() {
            second = vec![Map::new()];
        }
    }
    let (lim_p, lim_g) = if every { (usize::MAX, usize::MAX) } else { (3, 2) };
    for ch in &base {
        out.push(ch.clone());
        if im.repeat.truthy() {
            // le Repeat peut aussi être payé avec un jeu en Flow
            for ch2 in second.iter().take(lim_p) {
                let tg2 = ch2.get("tg").cloned().unwrap_or(Value::Tuple(vec![]));
                out.push(dict_with(ch, &[("rep", true.into()), ("tg2", tg2)]));
            }
        }
        for (k, re, rr) in &grep {
            // un Repeat donné (820) : le payer rejoue l'effet, avec ses propres choix
            for ch2 in second.iter().take(lim_g) {
                let rc = dict_with(ch2, &[("_key", k.clone()), ("_e", (*re).into()), ("_reqs", reqs_tuple(rr))]);
                out.push(dict_with(ch, &[("reps", Value::Tuple(vec![rc.into()]))]));
            }
        }
        if !grep.is_empty() && grep.len() + im.repeat.truthy() as usize >= 2 {
            out.extend(repeat_combos(ch, im, &grep, &second, lim_p, lim_g));
        }
    }
    Ok(out)
}

/// `_repeat_combos(ch, im, grep, second, lim_p, lim_g)` : plusieurs instances de Repeat payées ensemble (820.1.c.2,
/// 820.3), chacune avec ses propres choix.
fn repeat_combos(ch: &Map, im: &Impl, grep: &[(Value, i64, Vec<Req>)], second: &[Map], lim_p: usize, lim_g: usize) -> Vec<Map> {
    let mut out = vec![];
    let n = grep.len();
    let per: Vec<Map> = second.iter().take(lim_g).cloned().collect();
    for mask in 1usize..(1 << n) {
        let idx: Vec<usize> = (0..n).filter(|i| mask >> i & 1 == 1).collect();
        let printed_opts: &[bool] = if im.repeat.truthy() { &[false, true] } else { &[false] };
        for &printed in printed_opts {
            if idx.len() + (printed as usize) < 2 {
                continue;
            }
            let heads: Vec<Map> = if printed {
                second
                    .iter()
                    .take(lim_p)
                    .map(|c| crate::map! { rep = true, tg2 = c.get("tg").cloned().unwrap_or(Value::Tuple(vec![])) })
                    .collect()
            } else {
                vec![Map::new()]
            };
            if per.is_empty() {
                continue;
            }
            for head in &heads {
                // itertools.product(*per) : le dernier indice varie le plus vite
                let mut pos = vec![0usize; idx.len()];
                'prod: loop {
                    let reps: Vec<Value> = idx
                        .iter()
                        .zip(pos.iter())
                        .map(|(&i, &p)| {
                            dict_with(&per[p], &[("_key", grep[i].0.clone()), ("_e", grep[i].1.into()), ("_reqs", reqs_tuple(&grep[i].2))])
                                .into()
                        })
                        .collect();
                    let mut m = ch.merged(head);
                    m.set("reps", Value::Tuple(reps));
                    out.push(m);
                    let mut j = pos.len();
                    loop {
                        if j == 0 {
                            break 'prod;
                        }
                        j -= 1;
                        pos[j] += 1;
                        if pos[j] < per.len() {
                            break;
                        }
                        pos[j] = 0;
                    }
                }
            }
        }
    }
    out
}

/// `playable_cards(g, pid, closed, showdown)` : [(carte, zone)]
pub fn playable_cards(g: &Game, pid: usize) -> Vec<(O, &'static str)> {
    let pl = &g.p[pid];
    let mut cands: Vec<(O, &'static str)> = pl.hand.iter().map(|&c| (c, "hand")).collect();
    cands.extend(pl.champ.iter().map(|&c| (c, "champ")));
    for b in &g.bfs {
        for &c in &b.facedowns {
            let o = g.o(c);
            if o.owner == pid && o.hidden_turn.is_some_and(|t| t < g.turn_no) {
                cands.push((c, "facedown"));
            }
        }
    }
    for ef in &g.effects {
        // « Until your next Ending Phase, you may play that card. » (Runeblade Anvil, Akali)
        if ef.d.g("kind").opt_str() == Some("play_banished") && ef.d.g("pid") == Value::from(pid) {
            let c = pl
                .banish
                .iter()
                .copied()
                .find(|&x| Value::from(g.o(x).uid) == ef.d.g("uid") && Value::from(g.o(x).oid) == ef.d.g("oid"));
            if let Some(c) = c {
                if !cands.contains(&(c, "banish")) {
                    cands.push((c, "banish"));
                }
            }
        }
    }
    let any_trash = !g.effects_of("play_from_trash", Some(pid)).is_empty(); // Endless Riches
    for &c in &pl.trash {
        let Some(im) = g.impl_o(c) else { continue };
        if any_trash || flow_of(g, pid, c).truthy() || !im.alt_costs.is_none() {
            cands.push((c, "trash"));
        }
    }
    cands
}

/// `play_options(g, pid, closed, showdown, every)`
pub fn play_options(g: &mut Game, pid: usize, closed: bool, showdown: bool, every: bool) -> Res<Vec<Value>> {
    let mut opts = vec![];
    let mut seen: BTreeSet<(String, &str)> = BTreeSet::new();
    for (c, src) in playable_cards(g, pid) {
        let key = (g.cname(c).to_string(), src);
        if seen.contains(&key) && src != "facedown" {
            continue; // des copies identiques donnent les mêmes options
        }
        seen.insert(key);
        for ch in card_choices(g, pid, c, src, closed, showdown, every)? {
            opts.push(Value::Tuple(vec!["play".into(), g.o(c).uid.into(), src.into(), ch.into()]));
        }
    }
    Ok(opts)
}

/// `abilities_of(g, obj)` : capacités activées d'un objet (les siennes, données pour une durée, données par d'autres
/// objets, texte copié). Les indices sont stables pour un état donné.
pub fn abilities_of(g: &mut Game, obj: O) -> Res<Vec<Map>> {
    let leg = g.o(obj).zone == Some(Zone::Legend);
    let im = if leg { g.impl_n(&g.p[g.o(obj).ctrl].legend_name) } else { g.impl_o(obj) };
    let mut out: Vec<Map> = im.map(|im| im.abilities.clone()).unwrap_or_default();
    if !leg && g.o(obj).attached_to.is_some() {
        // 434.1.e : une carte attachée a son texte de règles inactif tant qu'elle reste attachée
        out = vec![];
    }
    out.extend(g.o(obj).abs.iter().map(|a| a.0.clone()));
    if !leg && !g.o(obj).attached.is_empty() {
        for n in g.copied_texts(obj) {
            if let Some(ic) = g.impl_n(&n) {
                out.extend(ic.abilities.iter().cloned());
            }
        }
    }
    for (so, ia) in g.providers("grant_abilities") {
        let r = ia.hook("grant_abilities").call(g, &[so, obj.into()])?;
        out.extend(maps(&r));
    }
    Ok(out)
}

/// `ability_options(g, pid, closed, showdown, main, every)`
pub fn ability_options(g: &mut Game, pid: usize, closed: bool, showdown: bool, main: bool, every: bool) -> Res<Vec<Value>> {
    if !every {
        let fc = full_choices(g, Some(pid));
        return ability_options_inner(g, pid, closed, showdown, main, fc);
    }
    let prev = std::mem::replace(&mut g.every_choice, true);
    let r = ability_options_inner(g, pid, closed, showdown, main, true);
    g.every_choice = prev;
    r
}

fn ability_options_inner(g: &mut Game, pid: usize, closed: bool, showdown: bool, main: bool, every: bool) -> Res<Vec<Value>> {
    let mut opts = vec![];
    let mut srcs: Vec<Option<O>> = g.board.iter().copied().filter(|&o| g.o(o).ctrl == pid).map(Some).collect();
    srcs.push(None); // "legend"
    for o in srcs {
        let obj = o.unwrap_or(g.p[pid].legend);
        for (i, ab) in abilities_of(g, obj)?.into_iter().enumerate() {
            let t = match ab.get("timing") {
                None => "main".to_string(),
                Some(v) => v.opt_str().unwrap_or("").to_string(),
            };
            if closed && t != "reaction" {
                continue;
            }
            if showdown && !closed && t != "action" && t != "reaction" {
                continue;
            }
            if !closed && !showdown && !main {
                continue;
            }
            if g.tp != pid && t == "main" {
                continue;
            }
            let can = ab.g("can");
            if can.truthy() && !can.call(g, &[pid.into(), obj.into()])?.truthy() {
                continue;
            }
            if ab.g("exhaust").truthy() && g.o(obj).exh {
                continue;
            }
            let chs: Vec<Map> = if every && ab.g("all_choices").truthy() {
                maps(&ab.g("all_choices").call(g, &[pid.into(), obj.into()])?)
            } else {
                let c: Vec<Map> =
                    if ab.g("choices").truthy() { maps(&ab.g("choices").call(g, &[pid.into(), obj.into()])?) } else { vec![Map::new()] };
                if every {
                    c
                } else {
                    c.into_iter().take(MAX_CHOICES).collect()
                }
            };
            for ch in chs {
                let (e, reqs) = ability_cost(g, pid, obj, &ab, &ch)?;
                if g.can_pay(pid, e, &reqs, &crate::map! { kind = "ability", obj = obj }.into())? {
                    let src = match o {
                        None => Value::Tuple(vec!["legend".into(), pid.into()]),
                        Some(o) => g.o(o).uid.into(),
                    };
                    opts.push(Value::Tuple(vec!["act".into(), src, i.into(), ch.into()]));
                }
            }
        }
    }
    Ok(opts)
}

/// `ability_cost(g, pid, obj, ab, ch)` : son coût, plus Deflect (809), puis les modificateurs (cost_mods 'ability').
pub fn ability_cost(g: &mut Game, pid: usize, obj: O, ab: &Map, ch: &Map) -> Res<(i64, Vec<Req>)> {
    let (e, mut reqs) = cost_of(&ab.g("cost").call(g, &[pid.into(), obj.into(), ch.clone().into()])?);
    if !ab.g("ignore_deflect").truthy() {
        reqs.extend(deflect_reqs(g, pid, ch)?);
    }
    if !g.effects.is_empty() || registry().hook_names("cost_aura").is_some() {
        let mut what = Map::new();
        what.set("kind", "ability");
        what.set("card", Value::None);
        what.set("obj", obj);
        what.set("ab", ab.clone());
        what.set("choice", ch.clone());
        what.set("src", Value::None);
        let mods = cost_mods(g, pid, &what)?;
        if !mods.is_empty() {
            return apply_mods(g, pid, e, &reqs, &[], &mods, &crate::map! { kind = "ability", obj = obj }.into());
        }
    }
    Ok((e, reqs))
}

/// `move_options(g, pid)` : Standard Move (144, 420.3). Base -> battlefield, battlefield -> base, battlefield ->
/// battlefield seulement avec Ganking (ou vers le Baron Pit, 187.9).
pub fn move_options(g: &mut Game, pid: usize) -> Res<Vec<Value>> {
    let ready: Vec<O> = g.units(Some(pid), None).into_iter().filter(|&u| !g.o(u).exh).collect();
    let mut opts = vec![];
    for d in g.locs() {
        // Baron Pit (187.9) : « Units can move here from anywhere »
        let anywhere = matches!(d, Loc::Bf(b) if g.bfs[b].name == BARON_PIT);
        let mut movers = vec![];
        for &u in &ready {
            let ul = g.o(u).loc;
            if ul != Some(d)
                && (d == Loc::Base || ul == Some(Loc::Base) || anywhere || g.has_kw(u, "Ganking")?)
                && g.can_move(u, d, Some(pid), true)?
            {
                movers.push(u);
            }
        }
        if d == Loc::Base {
            movers.retain(|&u| is_bf(g.o(u).loc));
        }
        if movers.is_empty() {
            continue;
        }
        // un set de tuples, parcouru trié (ordre fixe)
        let mut groups: BTreeSet<Vec<u32>> = BTreeSet::new();
        if full_choices(g, Some(pid)) && movers.len() <= 10 {
            // humain : n'importe quel sous-ensemble d'unités prêtes (144)
            let mut uids: Vec<u32> = movers.iter().map(|&u| g.o(u).uid).collect();
            uids.sort();
            for mask in 1usize..(1 << uids.len()) {
                groups.insert(uids.iter().enumerate().filter(|(i, _)| mask >> i & 1 == 1).map(|(_, &x)| x).collect());
            }
        }
        for &u in &movers {
            groups.insert(vec![g.o(u).uid]);
        }
        let mut by_loc: Vec<(Option<Loc>, Vec<u32>)> = vec![];
        for &u in &movers {
            let l = g.o(u).loc;
            let uid = g.o(u).uid;
            match by_loc.iter_mut().find(|x| x.0 == l) {
                Some(x) => x.1.push(uid),
                None => by_loc.push((l, vec![uid])),
            }
        }
        for (_, us) in &by_loc {
            if us.len() > 1 {
                let mut s = us.clone();
                s.sort();
                groups.insert(s);
                if us.len() > 2 {
                    // tous sauf le plus faible
                    let mut keyed = vec![];
                    for &x in us {
                        let o = g.obj(x).unwrap();
                        keyed.push((g.might(o)?, x));
                    }
                    keyed.sort_by_key(|k| k.0);
                    let mut s2: Vec<u32> = keyed[1..].iter().map(|k| k.1).collect();
                    s2.sort();
                    groups.insert(s2);
                }
            }
        }
        if by_loc.len() > 1 {
            let mut s: Vec<u32> = movers.iter().map(|&u| g.o(u).uid).collect();
            s.sort();
            groups.insert(s);
        }
        for grp in groups {
            let objs: Vec<O> = grp.iter().map(|&u| g.obj(u).unwrap()).collect();
            let (mut e, mut reqs) = (0, vec![]);
            if grp.len() > 1 && registry().hook_names("move_tax").is_some() {
                reqs = g.move_tax(pid, &objs, d)?;
            }
            if registry().hook_names("move_cost").is_some() {
                let (e2, r2) = g.move_cost(pid, &objs, d)?;
                e += e2;
                reqs.extend(r2);
            }
            if (e != 0 || !reqs.is_empty()) && !g.can_pay(pid, e, &reqs, &crate::map! { kind = "move" }.into())? {
                continue;
            }
            opts.push(Value::Tuple(vec!["move".into(), Value::Tuple(grp.iter().map(|&u| Value::from(u)).collect()), loc_v(d)]));
        }
    }
    Ok(opts)
}

/// `hide_costs(g, pid)` (811.1.b) : [A], coûts alternatifs (hide_cost), gratuit (effet 'hide_free').
pub fn hide_costs(g: &mut Game, pid: usize) -> Res<Vec<(i64, Vec<Req>)>> {
    let mut out = vec![(0, vec![any_req()])];
    if !g.effects_of("hide_free", Some(pid)).is_empty() {
        return Ok(vec![(0, vec![])]);
    }
    for (so, ia) in g.providers("hide_cost") {
        let r = ia.hook("hide_cost").call(g, &[so, pid.into()])?;
        for c in r.list() {
            let c = cost_of(c);
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    Ok(out)
}

/// `hide_slots(g, b)` : cartes cachées qu'un battlefield peut porter (Bandle Tree, Bushwhack Trap).
pub fn hide_slots(g: &mut Game, b: usize) -> Res<i64> {
    let name = g.bfs[b].name.clone();
    let mut n = 1 + g.impl_n(&name).map(|im| im.hook("hide_slots").opt_int().unwrap_or(0)).unwrap_or(0);
    for (so, ia) in g.providers("extra_hide_slots") {
        n += ia.hook("extra_hide_slots").call(g, &[so, Value::Int(b as i64)])?.opt_int().unwrap_or(0);
    }
    Ok(n)
}

/// `hide_options(g, pid)`
pub fn hide_options(g: &mut Game, pid: usize) -> Res<Vec<Value>> {
    let mut opts = vec![];
    let mut costs = vec![];
    for c in hide_costs(g, pid)? {
        if g.can_pay(pid, c.0, &c.1, &crate::map! { kind = "hide" }.into())? {
            costs.push(c);
        }
    }
    if costs.is_empty() {
        return Ok(opts);
    }
    let default = (0, vec![any_req()]);
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let cs: Vec<O> = g.p[pid].hand.iter().chain(g.p[pid].champ.iter()).copied().collect();
    for c in cs {
        let Some(im) = g.impl_o(c) else { continue };
        if !(im.hidden || card_kw(g, pid, c, "Hidden", Some("hand"))?) || seen.contains(g.cname(c)) {
            continue;
        }
        seen.insert(g.cname(c).to_string());
        for b in 0..g.bfs.len() {
            if g.bfs[b].ctrl == Some(pid) && (g.bfs[b].facedowns.len() as i64) < hide_slots(g, b)? {
                for cost in &costs {
                    let uid = g.o(c).uid;
                    opts.push(if *cost == default {
                        Value::Tuple(vec!["hide".into(), uid.into(), b.into()])
                    } else {
                        Value::Tuple(vec!["hide".into(), uid.into(), b.into(), Value::Tuple(vec![cost.0.into(), reqs_tuple(&cost.1)])])
                    });
                }
            }
        }
    }
    Ok(opts)
}

/// `main_options(g, pid)`
pub fn main_options(g: &mut Game, pid: usize) -> Res<Vec<Value>> {
    main_options_every(g, pid, false)
}

/// `main_options(g, pid, every)`
pub fn main_options_every(g: &mut Game, pid: usize, every: bool) -> Res<Vec<Value>> {
    let mut opts = vec![Value::Tuple(vec!["end".into()])];
    opts.extend(play_options(g, pid, false, false, every)?);
    opts.extend(ability_options(g, pid, false, false, true, every)?);
    opts.extend(hide_options(g, pid)?);
    opts.extend(move_options(g, pid)?);
    Ok(opts)
}

/// `timed_options(g, pid, closed)`
pub fn timed_options(g: &mut Game, pid: usize, closed: bool) -> Res<Vec<Value>> {
    timed_options_every(g, pid, closed, false)
}

/// `timed_options(g, pid, closed, every)`
pub fn timed_options_every(g: &mut Game, pid: usize, closed: bool, every: bool) -> Res<Vec<Value>> {
    let showdown = g.sd.is_some();
    let mut opts = play_options(g, pid, closed, showdown || closed, every)?;
    opts.extend(ability_options(g, pid, closed, showdown, false, every)?);
    Ok(opts)
}

// ---------------------------------------------------------------------------------------------- exécution
/// `do_action(g, a)`
pub fn do_action(g: &mut Game, a: &Value) -> Res<()> {
    let a = a.list();
    match a[0].str() {
        "play" => {
            let (uid, src) = (as_uid(&a[1]).unwrap(), a[2].str());
            let mut pid = owner_of(g, uid);
            if src == "banish" {
                if let Some(ef) =
                    g.effects.iter().find(|ef| ef.d.g("kind").opt_str() == Some("play_banished") && ef.d.g("uid") == a[1])
                {
                    pid = ef.d.g("pid").int() as usize;
                }
            }
            let card = card_obj(g, pid, uid, src).expect("carte introuvable");
            play_card(g, pid, card, src, a[3].map().clone(), false)?;
        }
        "hide" => {
            let (uid, bf) = (as_uid(&a[1]).unwrap(), a[2].int() as usize);
            let pid = owner_of(g, uid);
            let cost = if a.len() > 3 { cost_of(&a[3]) } else { (0, vec![any_req()]) };
            hide(g, pid, uid, bf, cost)?;
        }
        "act" => {
            activate(g, &a[1], a[2].int() as usize, a[3].map().clone())?;
        }
        "move" => {
            let dest = a[2].loc();
            let units: Vec<O> = a[1].list().iter().map(|u| g.obj(as_uid(u).unwrap()).unwrap()).collect();
            let pid = g.o(units[0]).ctrl;
            let (mut e, mut reqs) = (0, vec![]);
            if units.len() > 1 && registry().hook_names("move_tax").is_some() {
                reqs = g.move_tax(pid, &units, dest)?; // Mageseeker Investigator
            }
            if registry().hook_names("move_cost").is_some() {
                let (e2, r2) = g.move_cost(pid, &units, dest)?; // Gravity Field
                e += e2;
                reqs.extend(r2);
            }
            if (e != 0 || !reqs.is_empty()) && !g.pay(pid, e, &reqs, &crate::map! { kind = "move" }.into())? {
                panic!("cannot pay the move cost");
            }
            for &u in &units {
                g.set_exhausted(u, true); // coût du Standard Move
            }
            if g.logging {
                let us: Vec<String> = units.iter().map(|&u| g.repr(u)).collect();
                let d = loc_str(Some(dest));
                g.log(|| format!("P{pid} moves [{}] -> {d}", us.join(", ")));
            }
            g.move_(&units, dest, Some(pid), true)?;
        }
        k => panic!("action inconnue {k}"),
    }
    Ok(())
}

/// `owner_of(g, uid, src)`
pub fn owner_of(g: &Game, uid: u32) -> usize {
    for pl in &g.p {
        for z in [&pl.hand, &pl.champ, &pl.trash, &pl.banish] {
            if z.iter().any(|&c| g.o(c).uid == uid) {
                return pl.pid;
            }
        }
    }
    for b in &g.bfs {
        for &c in &b.facedowns {
            if g.o(c).uid == uid {
                return g.o(c).owner;
            }
        }
    }
    panic!("card {uid} not found")
}

/// `hide(g, pid, uid, bf, cost)` (421, 811) : payer [A] (ou un autre coût), poser face cachée à un battlefield
/// contrôlé.
pub fn hide(g: &mut Game, pid: usize, uid: u32, bf: usize, cost: (i64, Vec<Req>)) -> Res<()> {
    let c = card_obj(g, pid, uid, "hand").or_else(|| card_obj(g, pid, uid, "champ")).expect("carte à cacher introuvable");
    assert!(g.pay(pid, cost.0, &cost.1, &crate::map! { kind = "hide", card = c }.into())?);
    g.remove_from_zone(c);
    g.reset_obj(c);
    let turn = g.turn_no;
    let ob = g.o_mut(c);
    ob.zone = Some(Zone::Facedown);
    ob.hidden_turn = Some(turn);
    ob.hidden_bf = Some(Loc::Bf(bf));
    g.bfs[bf].facedowns.push(c);
    let n = g.bfs[bf].name.clone();
    g.log(|| format!("P{pid} hides a card at {n}"));
    g.stats.add(&format!("hide_P{pid}"), 1);
    g.need_cleanup = true;
    g.emit("hide", crate::map! { pid = pid, card = c, bf = bf })
}

/// eids des effets de coût à usage unique (`once`) qui s'appliquent à ce coût.
fn used_once(g: &mut Game, pid: usize, what: &Map) -> Res<Vec<u64>> {
    if g.effects.is_empty() {
        return Ok(vec![]);
    }
    let mut used = vec![];
    for m in cost_mods(g, pid, what)? {
        if let Some(eid) = m.g("_eff").opt_int() {
            if g.eff(eid as u64).is_some_and(|e| e.g("once").truthy()) {
                used.push(eid as u64);
            }
        }
    }
    Ok(used)
}

/// `play_card(g, pid, card, src, ch, limited)` : le processus de jeu (353). limited : joué par un effet (Baited Hook,
/// Mixologist...). Renvoie la carte (unité, équipement), l'item (sort), ou None (coût d'un jeu limité impayable).
pub fn play_card(g: &mut Game, pid: usize, card: O, src: &str, ch: Map, limited: bool) -> Res<Value> {
    let mut ch = ch;
    let im = g.impl_o(card).expect("play_card : carte sans Impl");
    let sp = g.spec(card);
    let typ = sp.type_.as_str();
    let mut paid_e = 0;
    if !limited {
        let (e, reqs) = total_cost(g, pid, card, &ch, src)?;
        let mut what = Map::new();
        what.set("kind", typ.to_lowercase());
        what.set("card", card);
        what.set("obj", Value::None);
        what.set("ab", Value::None);
        what.set("choice", ch.clone());
        what.set("src", src);
        let used = used_once(g, pid, &what)?;
        let ctx = pay_ctx(g, card);
        if !g.pay(pid, e, &reqs, &ctx)? {
            let r = g.repr(card);
            panic!("cannot pay for {r} {ch:?}");
        }
        let nd = deflect_total(g, pid, card, &ch)?.len();
        if nd > 0 {
            g.log(|| format!("  P{pid} pays Deflect +{nd} power"));
        }
        paid_e = e;
        // consomme la réduction d'Astral Heron et les effets de coût à usage unique qui ont servi
        let pv = Value::from(pid);
        g.effects
            .retain(|ef| !(ef.d.g("kind").opt_str() == Some("heron") && ef.d.g("pid") == pv) && !used.contains(&ef.eid));
    } else if ch.g("pay_power").truthy() {
        let c2 = dict_with(&ch, &[("ignore_energy", true.into()), ("free", false.into())]);
        let (_, reqs) = total_cost(g, pid, card, &c2, "hand")?;
        if !g.pay(pid, 0, &reqs, &Value::None)? {
            return Ok(Value::None);
        }
    } else if ch.g("pay_cost").truthy() {
        // jeu limité qui paie un coût donné (« you may play me for 1 rune of any type ») : (énergie, reqs)
        let (e, reqs) = cost_of(&ch.g("pay_cost"));
        let ctx = pay_ctx(g, card);
        if !g.pay(pid, e, &reqs, &ctx)? {
            return Ok(Value::None);
        }
        paid_e = e;
    }
    if !ch.g("perm").is_none() {
        // une permission de jeu utilisée (card_choices)
        let perm = ch.g("perm");
        g.effects.retain(|ef| !(ef.d.g("kind").opt_str() == Some("play_perm") && ef.d.g("key") == perm));
    }
    if typ == "Spell" {
        // « Give the next spell you play this turn [Repeat]... »
        let pv = Value::from(pid);
        g.effects
            .retain(|ef| !(ef.d.g("kind").opt_str() == Some("grant_repeat") && ef.d.g("next").truthy() && ef.d.g("pid") == pv));
    }
    // coûts additionnels non-ressources (357.2)
    if !ch.g("kill").is_none() {
        if let Some(k) = as_uid(&ch.g("kill")).and_then(|u| g.obj(u)) {
            let ks = g.spec(k);
            let m = g.might(k)?;
            let mut info = Map::new();
            info.set("e", ks.e);
            info.set("p", ks.p);
            info.set("might", m);
            info.set("name", g.cname(k));
            ch.set("killed_info", info);
            g.kill(&[k], Some(pid), true)?;
        }
    }
    if ch.g("xp").truthy() {
        let n = match ch.g("xp") {
            Value::Bool(_) => 3,
            v => v.int(),
        };
        g.spend_xp(pid, n)?;
    }
    if !ch.g("show").is_none() && ch.g("shown").is_none() {
        // [Show Off] : vérifié au moment où il est montré (832.1.d.1)
        let si = shown_info(g, pid, as_uid(&ch.g("show")).unwrap_or(0))?;
        ch.set("shown", si.clone());
        if !si.is_none() {
            let n = si.map().g("name").str().to_string();
            g.log(|| format!("  P{pid} shows off {n}"));
        }
    }
    if !im.pay_extra.is_none() {
        // coûts additionnels non-ressources de la carte (défausse...)
        if let Value::Map(m) = im.pay_extra.call(g, &[pid.into(), card.into(), ch.clone().into()])? {
            ch = m;
        }
    }
    for (so, ia) in g.providers("opt_pay") {
        // coûts additionnels optionnels donnés par d'autres objets
        if let Value::Map(m) = ia.hook("opt_pay").call(g, &[so, pid.into(), card.into(), ch.clone().into()])? {
            ch = m;
        }
    }
    g.remove_from_zone(card);
    let from_facedown = src == "facedown";
    let hidden_bf = if from_facedown { g.o(card).hidden_bf } else { None };
    g.o_mut(card).zone = Some(Zone::Chain);
    if typ == "Spell" {
        g.hist.spell_e[pid].push(paid_e);
    } else if typ == "Gear" && !g.o(card).token {
        g.hist.gear_played[pid] += 1;
    }
    g.o_mut(card).paid_e = paid_e;
    g.o_mut(card).limited_play = limited;
    let cn = g.cname(card).to_string();
    g.finalized[pid].push(cn.clone());
    if g.logging {
        let f = fmt_choice(g, &ch);
        g.log(|| format!("P{pid} plays {cn} from {src} {f}"));
    }
    g.stats.add(&format!("play_{cn}"), 1);
    if g.chain.is_empty() && g.chain_origin.is_none() {
        g.chain_origin = Some("play");
    }
    if typ == "Unit" || typ == "Gear" {
        let mut loc = loc_of(&ch.g("loc")).unwrap_or(Loc::Base);
        if typ == "Gear" && !from_facedown {
            if has_deploy(g, card)? {
                // « Play this only to a battlefield » (830.1.b)
                if !loc.is_bf() {
                    let mut locs = gear_locations(g, pid, card, None)?;
                    if locs.is_empty() {
                        locs = g.bf_locs();
                    }
                    let opts: Vec<Value> = locs.into_iter().map(loc_v).collect();
                    loc = g.ask(pid, "deploy_location", opts, crate::map! { card = card })?.loc();
                }
            } else {
                loc = Loc::Base;
            }
        }
        g.o_mut(card).played_to = Some(loc); // où elle est jouée (Riven, The Exile)
        let mut ready = (typ == "Gear" && !im.enter_exhausted) || ch.g("acc").truthy() || ch.g("ready").truthy();
        if !ready && (enters_ready(g, pid, card, &ch)? || others_enter_ready(g, pid, card)?) {
            ready = true;
        }
        if !im.enter_loc.is_none() {
            // « As you play me ... I enter there » (Baron Nashor, 369.3)
            loc = loc_of(&im.enter_loc.call(g, &[pid.into(), card.into(), loc_v(loc)])?).expect("emplacement attendu");
        }
        g.enter_board(card, pid, loc, ready);
        if ch.g("tag").truthy() {
            g.o_mut(card).chosen_tag = Some(ch.g("tag").str().to_string()); // « As you play me, choose ... »
        }
        g.o_mut(card).played_from = Some(src.to_string());
        g.o_mut(card).play_choice = Some(ch.clone());
        g.played_event(pid, card, None)?;
        keyword_play_triggers(g, pid, card, hidden_bf)?;
        if im.on_play.truthy() {
            let ctx = dict_with(&ch, &[("hidden_bf", hidden_bf.into()), ("src", src.into())]);
            im.on_play.call(g, &[card.into(), ctx.into()])?;
        }
        if card_kw(g, pid, card, "Quick-Draw", Some(src))? {
            // Quick-Draw : « When you play this, attach it to a unit you control. »
            let qd_choose = Value::func("qd_choose", move |g, a| {
                let it = a[0].item();
                let ctrl = g.it(it).ctrl;
                let mut us = g.units(Some(ctrl), None);
                if let Some(hb) = hidden_bf {
                    let f: Vec<O> = us.iter().copied().filter(|&u| g.o(u).loc == Some(hb)).collect();
                    if !f.is_empty() {
                        us = f;
                    }
                }
                if us.is_empty() {
                    return Ok(false.into());
                }
                let u = g.ask(ctrl, "equip_target", us.into_iter().map(Value::Obj).collect(), crate::map! { gear = card })?;
                g.add_target(it, u.obj(), Value::None);
                Ok(true.into())
            });
            let qd_resolve = Value::func("qd_resolve", |g, a| {
                let it = a[0].item();
                let u = g.legal(it, 0)?;
                let gobj = g.it(it).src.and_then(|s| g.obj(s));
                if let (Some(u), Some(gobj)) = (u, gobj) {
                    attach(g, gobj, u)?;
                }
                Ok(Value::None)
            });
            let uid = g.o(card).uid;
            g.queue_trigger(pid, &format!("Quick-Draw {cn}"), qd_resolve, Map::new(), Some(uid), false, qd_choose, Value::None);
        }
        g.need_cleanup = true;
        return Ok(card.into());
    }
    // sort
    g.spells_played[pid] += 1;
    let mut data = ch.clone();
    data.set("from", src);
    data.set("hidden_bf", hidden_bf);
    data.set("played_by", pid);
    data.set("paid_e", paid_e);
    let uid = g.o(card).uid;
    let it = g.new_item("spell", pid, &cn, Value::None, Some(card), Some(uid), data);
    let pv = Value::from(pid);
    let nexts: Vec<u64> =
        g.effects.iter().filter(|e| e.d.g("kind").opt_str() == Some("next_spell") && e.d.g("pid") == pv).map(|e| e.eid).collect();
    for eid in nexts {
        // « The next spell you play this turn ... » (Ravenborn Tome) : retiré, puis sa fn est appelée
        let Some(i) = g.effects.iter().position(|e| e.eid == eid) else { continue };
        let ef = g.effects.remove(i);
        let f = ef.d.g("fn");
        g.effects_gone.push(ef);
        let r = f.call(g, &[Value::Int(eid as i64), it.into()]);
        if let Some(j) = g.effects_gone.iter().position(|e| e.eid == eid) {
            g.effects_gone.remove(j);
        }
        r?;
    }
    if ch.g("reps").truthy() {
        let mut reps = vec![];
        for rc in ch.g("reps").list() {
            let mut rc = rc.map().clone();
            let mut oids = Map::new();
            for u in rc.g("tg").list() {
                if let Some(o) = as_uid(u).and_then(|x| g.obj(x)) {
                    let ou = g.o(o).uid;
                    oids.set(&ou.to_string(), g.o(o).oid);
                    g.it_mut(it).chosen.insert(ou);
                }
            }
            rc.set("_oids", oids);
            reps.push(Value::Map(rc));
        }
        g.it_mut(it).data.set("reps", Value::List(reps));
    }
    if ch.g("flow").truthy() || im.banish_after {
        g.it_mut(it).data.set("after", "banish");
    }
    if im.uncounterable {
        g.it_mut(it).uncounterable = true;
    }
    let preds: Vec<Value> = im.preds.list().to_vec();
    for (i, uid) in ch.g("tg").list().iter().enumerate() {
        match as_uid(uid) {
            Some(u) => match g.obj(u) {
                Some(o) => {
                    let pred = if preds.is_empty() { Value::None } else { preds[i.min(preds.len() - 1)].clone() };
                    g.add_target(it, o, pred);
                }
                None => g.it_mut(it).targets.push((Some(u), -1, Value::None)),
            },
            None => g.it_mut(it).targets.push((None, -1, Value::None)),
        }
    }
    for uid in ch.g("tg2").list() {
        if let Some(o) = as_uid(uid).and_then(|u| g.obj(u)) {
            let (u, oid) = (g.o(o).uid, g.o(o).oid);
            let d = &mut g.it_mut(it).data;
            if d.get("t2").is_none() {
                d.set("t2", Value::List(vec![]));
            }
            d.get_mut("t2").unwrap().list_mut().push(Value::Tuple(vec![u.into(), oid.into()]));
            g.it_mut(it).chosen.insert(u);
        }
    }
    if !ch.g("item").is_none() {
        g.it_mut(it).data.set("target_item", ch.g("item"));
    }
    g.chain.push(it);
    g.priority = Some(pid);
    g.passes = 0;
    g.on_finalize(it)?;
    g.need_cleanup = true;
    Ok(it.into())
}

/// `others_enter_ready(g, pid, card)` : « Your units enter ready » des autres objets (units_enter_ready) et des
/// effets ('enter_ready', once : retiré) (369.3).
pub fn others_enter_ready(g: &mut Game, pid: usize, card: O) -> Res<bool> {
    for (so, ia) in g.providers("units_enter_ready") {
        if ia.hook("units_enter_ready").call(g, &[so, pid.into(), card.into()])?.truthy() {
            return Ok(true);
        }
    }
    for ef in g.effects.clone() {
        if ef.d.g("kind").opt_str() == Some("enter_ready")
            && ef.d.g("fn").call(g, &[Value::Int(ef.eid as i64), pid.into(), card.into()])?.truthy()
        {
            if ef.d.g("once").truthy() {
                g.remove_effect(ef.eid); // « The next unit you play this turn enters ready »
            }
            return Ok(true);
        }
    }
    Ok(false)
}

/// `enters_ready(g, pid, card, ch)` : « I enter ready » (Impl.enter_ready : True ou fonction) et [Level N] ready=True.
pub fn enters_ready(g: &mut Game, pid: usize, card: O, ch: &Map) -> Res<bool> {
    let im = g.impl_o(card).expect("enters_ready : carte sans Impl");
    let er = &im.enter_ready;
    let yes = match er {
        Value::Bool(true) => true,
        Value::Fn(_) => er.call(g, &[pid.into(), card.into(), ch.clone().into()])?.truthy(),
        _ => false,
    };
    if yes {
        return Ok(true);
    }
    if !im.levels.is_empty() {
        let xp = g.p[pid].xp;
        if im.levels.iter().any(|(n, d)| xp >= *n && d.g("ready").truthy()) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `keyword_play_triggers(g, pid, card, hidden_bf)` : [Vision] (817, un déclenchement par instance) et
/// [Weaponmaster] (821) d'un permanent qui vient d'être joué (803 : les mots-clés d'abord).
pub fn keyword_play_triggers(g: &mut Game, pid: usize, card: O, _hidden_bf: Option<Loc>) -> Res<()> {
    let cn = g.cname(card).to_string();
    let uid = g.o(card).uid;
    for _ in 0..g.kw_value(card, "Vision")? {
        let f = Value::func("vision", |g, a| {
            let c = g.it(a[0].item()).ctrl;
            g.predict(c, 1)?;
            Ok(Value::None)
        });
        g.queue_trigger(pid, &format!("Vision {cn}"), f, Map::new(), Some(uid), false, Value::None, Value::None);
    }
    if g.spec(card).type_ == "Unit" {
        for _ in 0..g.kw_value(card, "Weaponmaster")? {
            weaponmaster_trigger(g, pid, card);
        }
    }
    Ok(())
}

/// `equip_cost(g, pid, gear, unit, discount_any)` : coût de la capacité Equip (e, reqs, capacité) ou None.
/// discount_any : nombre de [A] retirés (Weaponmaster, 821.1.c.3 : seul [A] est réduit).
pub fn equip_cost(g: &mut Game, pid: usize, gear: O, unit: O, discount_any: usize) -> Res<Option<(i64, Vec<Req>, Map)>> {
    let Some(im) = g.impl_o(gear) else { return Ok(None) };
    for ab in &im.abilities {
        if ab.g("name").opt_str() == Some("Equip") {
            let ch = crate::map! { tg = Value::Tuple(vec![g.o(unit).uid.into()]) };
            let (e, mut reqs) = cost_of(&ab.g("cost").call(g, &[pid.into(), gear.into(), ch.into()])?);
            let any = any_req();
            for _ in 0..discount_any {
                if let Some(i) = reqs.iter().position(|r| *r == any) {
                    reqs.remove(i);
                }
            }
            return Ok(Some((e, reqs, ab.clone())));
        }
    }
    Ok(None)
}

/// `weaponmaster_trigger(g, pid, unit)` (821) : « When you play me, you may choose an Equipment you control and pay
/// its Equip cost, reduced by [A], to attach it to me ». Choix à la finalisation, coût payé à la résolution.
pub fn weaponmaster_trigger(g: &mut Game, pid: usize, unit: O) {
    fn options(g: &mut Game, it: ItemId) -> Res<Vec<O>> {
        let Some(u) = g.it(it).src.and_then(|s| g.obj(s)) else { return Ok(vec![]) };
        let ctrl = g.it(it).ctrl;
        let mut out = vec![];
        for x in g.gear(Some(ctrl)) {
            if !g.spec(x).tags.contains("Equipment") || g.o(x).attached_to == Some(g.o(u).uid) {
                continue;
            }
            if let Some((e, reqs, ab)) = equip_cost(g, ctrl, x, u, 1)? {
                if g.can_pay(ctrl, e, &reqs, &crate::map! { kind = "ability", obj = x }.into())?
                    && (ab.g("can_extra").is_none() || ab.g("can_extra").call(g, &[ctrl.into(), x.into()])?.truthy())
                {
                    out.push(x);
                }
            }
        }
        let mut keyed: Vec<(i64, O)> =
            out.iter().map(|&x| (-(g.equip_bonus_of(g.cname(x)) * 10 + g.spec(x).e as i64), x)).collect();
        keyed.sort_by_key(|k| k.0);
        Ok(keyed.into_iter().map(|k| k.1).collect())
    }
    let choose = Value::func("weaponmaster_choose", |g, a| {
        let it = a[0].item();
        let opts = options(g, it)?;
        if opts.is_empty() {
            return Ok(false.into());
        }
        let ctrl = g.it(it).ctrl;
        let unit = g.it(it).src.and_then(|s| g.obj(s));
        let x = g.ask(ctrl, "weaponmaster_pick", opts.into_iter().map(Value::Obj).collect(), crate::map! { unit = unit })?;
        let Some(x) = x.opt_obj() else { return Ok(false.into()) };
        let pred = Value::func("weaponmaster_pred", |g, a| Ok((g.o(a[1].obj()).ctrl == g.it(a[0].item()).ctrl).into()));
        g.add_target(it, x, pred);
        Ok(true.into())
    });
    let resolve = Value::func("weaponmaster_resolve", |g, a| {
        let it = a[0].item();
        let u = g.it(it).src.and_then(|s| g.obj(s));
        let x = g.legal(it, 0)?;
        let (Some(u), Some(x)) = (u, x) else { return Ok(Value::None) };
        if Value::from(g.o(u).oid) != g.it(it).data.g("oid") {
            return Ok(Value::None);
        }
        let ctrl = g.it(it).ctrl;
        let Some((e, reqs, ab)) = equip_cost(g, ctrl, x, u, 1)? else { return Ok(Value::None) };
        if !g.pay(ctrl, e, &reqs, &crate::map! { kind = "ability", obj = x }.into())? {
            return Ok(Value::None);
        }
        if ab.g("extra_cost").truthy() {
            let ch = crate::map! { tg = Value::Tuple(vec![g.o(u).uid.into()]) };
            ab.g("extra_cost").call(g, &[ctrl.into(), x.into(), ch.into()])?;
        }
        attach(g, x, u)?;
        Ok(Value::None)
    });
    let (cn, uid, oid) = (g.cname(unit).to_string(), g.o(unit).uid, g.o(unit).oid);
    g.queue_trigger(pid, &format!("Weaponmaster {cn}"), resolve, crate::map! { oid = oid }, Some(uid), true, choose, Value::None);
}

/// `attach(g, gear, unit)` (434) : détaché de l'unité précédente ; l'emplacement suit l'unité (pas un déplacement).
pub fn attach(g: &mut Game, gear: O, unit: O) -> Res<()> {
    if let Some(prev) = g.o(gear).attached_to {
        if let Some(old) = g.obj(prev) {
            let gu = g.o(gear).uid;
            if let Some(i) = g.o(old).attached.iter().position(|&x| x == gu) {
                g.o_mut(old).attached.remove(i);
            }
        }
    }
    let (uu, gu, loc) = (g.o(unit).uid, g.o(gear).uid, g.o(unit).loc);
    g.o_mut(gear).attached_to = Some(uu);
    g.o_mut(unit).attached.push(gu);
    g.o_mut(gear).loc = loc;
    if g.logging {
        let (a, b) = (g.repr(gear), g.repr(unit));
        g.log(|| format!("  {a} attached to {b}"));
    }
    g.emit("attached", crate::map! { gear = gear, unit = unit })
}

/// `activate(g, src, i, ch)` (376) : les capacités [Add] et Equip passent aussi par la chaîne. src : uid de l'objet
/// (`Value::Int`) ou `('legend', pid)`.
pub fn activate(g: &mut Game, src: &Value, i: usize, ch: Map) -> Res<()> {
    let mut ch = ch;
    let (pid, obj) = match src {
        Value::Tuple(t) => {
            let pid = t[1].int() as usize;
            (pid, g.p[pid].legend)
        }
        v => {
            let o = g.obj(as_uid(v).unwrap()).expect("source de capacité absente");
            (g.o(o).ctrl, o)
        }
    };
    let ab = abilities_of(g, obj)?.swap_remove(i);
    let (e, reqs) = ability_cost(g, pid, obj, &ab, &ch)?;
    let mut what = Map::new();
    what.set("kind", "ability");
    what.set("card", Value::None);
    what.set("obj", obj);
    what.set("ab", ab.clone());
    what.set("choice", ch.clone());
    what.set("src", Value::None);
    let used = used_once(g, pid, &what)?;
    let printed_e = cost_of(&ab.g("cost").call(g, &[pid.into(), obj.into(), ch.clone().into()])?).0;
    if !g.pay(pid, e, &reqs, &crate::map! { kind = "ability", obj = obj }.into())? {
        panic!("cannot pay ability");
    }
    g.effects.retain(|ef| !used.contains(&ef.eid));
    if g.spec(obj).type_ == "Gear" {
        g.hist.gear_abs[pid] += 1;
    }
    if ab.g("exhaust").truthy() {
        g.set_exhausted(obj, true);
    }
    if ab.g("extra_cost").truthy() {
        if let Value::Map(m) = ab.g("extra_cost").call(g, &[pid.into(), obj.into(), ch.clone().into()])? {
            ch = m;
        }
    }
    let name = format!("{}: {}", g.cname(obj), ab.g("name").str());
    let mut data = ch.clone();
    data.set("_ab", ab.g("name"));
    data.set("_cost_e", ab.get("printed_e").cloned().unwrap_or(Value::Int(printed_e))); // coût imprimé (206.1)
    data.set("_src_obj", obj);
    let uid = g.o(obj).uid;
    let it = g.new_item("ability", pid, &name, ab.g("resolve"), None, Some(uid), data);
    let preds: Vec<Value> = ab.g("preds").list().to_vec();
    for (j, uid) in ch.g("tg").list().iter().enumerate() {
        if let Some(o) = as_uid(uid).and_then(|u| g.obj(u)) {
            let pred = if preds.is_empty() { Value::None } else { preds[j.min(preds.len() - 1)].clone() };
            g.add_target(it, o, pred);
        }
    }
    if g.logging {
        let f = fmt_choice(g, &ch);
        let n = name.clone();
        g.log(|| format!("P{pid} activates {n} {f}"));
    }
    let key = format!("act_{}:{}", g.cname(obj), ab.g("name").str());
    g.stats.add(&key, 1);
    if g.chain.is_empty() && g.chain_origin.is_none() {
        g.chain_origin = Some("play");
    }
    g.chain.push(it);
    g.priority = Some(pid);
    g.passes = 0;
    g.on_finalize(it)?;
    g.need_cleanup = true;
    Ok(())
}

/// `fmt_choice(g, ch)` (journal)
pub fn fmt_choice(g: &Game, ch: &Map) -> String {
    let mut parts = vec![];
    for (k, v) in &ch.0 {
        let k: &str = k;
        if k == "tg" || k == "tg2" {
            let names: Vec<String> = v
                .list()
                .iter()
                .map(|u| match as_uid(u).and_then(|x| g.obj(x)) {
                    Some(o) => g.repr(o),
                    None => py_str(g, u),
                })
                .collect();
            parts.push(format!("{k}={}", names.join(",")));
        } else if k == "killed_info" {
            continue;
        } else if (k == "pay_cost" && v.list().len() == 2) || (k == "alt_reqs" && v.truthy()) {
            // (frozenset({...}), ...) : repr Python d'un coût en runes (pay_cost = (e, reqs)). L'ordre des domaines
            // d'un frozenset de plusieurs domaines dépend du hachage en Python (PYTHONHASHSEED) : ici, alphabétique.
            let reqs_repr = |r: &Value| -> String {
                let sets: Vec<String> = r
                    .list()
                    .iter()
                    .map(|r| {
                        let mut ds: Vec<String> = r.list().iter().map(|d| py_repr(g, d)).collect();
                        ds.sort();
                        format!("frozenset({{{}}})", ds.join(", "))
                    })
                    .collect();
                if sets.len() == 1 {
                    format!("({},)", sets[0])
                } else {
                    format!("({})", sets.join(", "))
                }
            };
            if k == "alt_reqs" {
                parts.push(format!("{k}={}", reqs_repr(v)));
            } else {
                let l = v.list();
                parts.push(format!("{k}=({}, {})", py_repr(g, &l[0]), reqs_repr(&l[1])));
            }
        } else if !matches!(v, Value::Bool(false) | Value::None)
            && !matches!(v, Value::Map(m) if m.0.is_empty())
            && !matches!(v, Value::Tuple(t) if t.is_empty())
        {
            parts.push(format!("{k}={}", py_str(g, v)));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("({})", parts.join(" "))
    }
}

/// Comparaison Python (`<`) de deux valeurs : nombres, textes, tuples et listes (lexicographique).
pub fn py_cmp(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Bool(_) | Value::Int(_) | Value::Float(_), Value::Bool(_) | Value::Int(_) | Value::Float(_)) => match (a, b) {
            (Value::Float(_), _) | (_, Value::Float(_)) => a.float().partial_cmp(&b.float()).unwrap_or(Ordering::Equal),
            _ => a.int().cmp(&b.int()),
        },
        (Value::Str(x), Value::Str(y)) => x.cmp(y),
        (Value::List(x), Value::List(y)) | (Value::Tuple(x), Value::Tuple(y)) => {
            for (p, q) in x.iter().zip(y.iter()) {
                let c = py_cmp(p, q);
                if c != Ordering::Equal {
                    return c;
                }
            }
            x.len().cmp(&y.len())
        }
        (Value::Obj(x), Value::Obj(y)) => x.cmp(y),
        (Value::Loc(x), Value::Loc(y)) => x.cmp(y),
        _ => panic!("comparaison impossible : {a:?} < {b:?}"),
    }
}

fn py_quote(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut out = String::new();
    out.push(q);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c == q => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(q);
    out
}

fn py_float(f: f64) -> String {
    if f.is_finite() && f == f.trunc() && f.abs() < 1e16 {
        format!("{f:.1}")
    } else {
        format!("{f}")
    }
}

/// `repr(v)` de Python (journal, libellés). Les emplacements : 'base' ou l'indice du battlefield.
pub fn py_repr(g: &Game, v: &Value) -> String {
    match v {
        Value::None => "None".into(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => py_float(*f),
        Value::Str(s) => py_quote(s),
        Value::Obj(o) => g.repr(*o),
        Value::Rune(r) => g.repr_rune(*r),
        Value::Item(i) => {
            let it = g.it(*i);
            format!("<{} {} P{}>", it.kind, it.name, it.ctrl)
        }
        Value::Loc(Loc::Base) => "'base'".into(),
        Value::Loc(Loc::Bf(i)) => i.to_string(),
        Value::List(l) => format!("[{}]", l.iter().map(|x| py_repr(g, x)).collect::<Vec<_>>().join(", ")),
        Value::Tuple(l) => {
            if l.len() == 1 {
                format!("({},)", py_repr(g, &l[0]))
            } else {
                format!("({})", l.iter().map(|x| py_repr(g, x)).collect::<Vec<_>>().join(", "))
            }
        }
        Value::Map(m) => {
            format!("{{{}}}", m.0.iter().map(|(k, x)| format!("{}: {}", py_quote(k), py_repr(g, x))).collect::<Vec<_>>().join(", "))
        }
        Value::Opt(o) => format!("Opt({})", o.label),
        Value::Fn(c) => format!("<function {}>", c.1),
    }
}

/// `str(v)` de Python (f-string) : un texte tel quel, 'base' sans guillemets, sinon `repr`.
pub fn py_str(g: &Game, v: &Value) -> String {
    match v {
        Value::Str(s) => s.to_string(),
        Value::Loc(Loc::Base) => "base".into(),
        v => py_repr(g, v),
    }
}
