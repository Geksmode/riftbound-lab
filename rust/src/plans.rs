//! Plans de jeu tirés des joueurs réels (`plans.py`). Un plan agit à cinq endroits : mulligan, battlefield présenté,
//! a priori d'une action (`prior`, unités d'évaluation, 7 ≈ 1 point), termes ajoutés à l'évaluation en fin de
//! simulation (`shape`), choix pendant la résolution (`choose`). PlanAgent = SearchAgent + plan ; mêmes simulations,
//! seul le score change.
use std::sync::{Arc, OnceLock};

use crate::actions::loc_of;
use crate::ai::{env_or, SearchAgent, SearchOpts};
use crate::game::{Game, Res, O};
use crate::value::{is_bf, Loc, Map, Value};

/// `VIDEO` (RB_VIDEO, actif par défaut) : vidéos LeBlanc (copie de Ruined Rex / Watcher en priorité, défausse d'un
/// Karthus en double si Glasc peut le ramener, Watcher closer, le Reflet prêt va prendre l'autre battlefield).
pub fn video() -> bool {
    static V: OnceLock<bool> = OnceLock::new();
    *V.get_or_init(|| env_or("RB_VIDEO", "1") != "0")
}

/// `KEEP_REFLECTION` (RB_REFL, actif par défaut ; RB_REFL=0 : ancien LeBlanc)
pub fn keep_reflection() -> bool {
    static V: OnceLock<bool> = OnceLock::new();
    *V.get_or_init(|| env_or("RB_REFL", "1") != "0")
}

pub const DK: [&str; 8] = [
    "Soaring Scout",
    "Honest Broker",
    "Watchful Sentry",
    "Black Rose Dignitary",
    "LeBlanc, Fragmented",
    "Glasc Mixologist",
    "Ruined Rex",
    "Rift Herald",
];

/// `card_of(g, uid)`
pub fn card_of(g: &Game, uid: i64) -> Option<O> {
    for pl in &g.p {
        for z in [&pl.hand, &pl.champ, &pl.trash] {
            for &c in z {
                if g.o(c).uid as i64 == uid {
                    return Some(c);
                }
            }
        }
    }
    for b in &g.bfs {
        if let Some(fd) = b.facedown() {
            if g.o(fd).uid as i64 == uid {
                return Some(fd);
            }
        }
    }
    g.obj(uid as u32)
}

/// `Plan` : plan de jeu (méthodes par défaut = aucun plan).
pub trait Plan: Send + Sync {
    /// `name`
    fn name(&self) -> &str {
        "aucun"
    }
    /// `mulligan(g, pid)` : None = heuristique par défaut
    fn mulligan(&self, _g: &mut Game, _pid: usize) -> Res<Option<Vec<O>>> {
        Ok(None)
    }
    /// `battlefield(deck, first, opp_bf)` : `battlefields` = `deck["battlefields"]` ; None = tirage au hasard
    fn battlefield(&self, _battlefields: &[String], _first: bool, _opp_bf: Option<&str>) -> Option<String> {
        None
    }
    /// `prior(g, me, a)`
    fn prior(&self, _g: &mut Game, _me: usize, _a: &Value) -> Res<f64> {
        Ok(0.0)
    }
    /// `shape(g, me)`
    fn shape(&self, _g: &mut Game, _me: usize) -> Res<f64> {
        Ok(0.0)
    }
    /// `choose(g, pid, kind, options, ctx)` : indice choisi, None = `NotImplemented`
    fn choose(&self, _g: &mut Game, _pid: usize, _kind: &str, _options: &[Value], _ctx: &Map) -> Res<Option<usize>> {
        Ok(None)
    }
}

/// `Plan()` : aucun plan.
#[derive(Clone, Default)]
pub struct NoPlan;
impl Plan for NoPlan {}

// ---------------------------------------------------------------------------------------- outils
fn uid_obj(g: &Game, v: &Value) -> Option<O> {
    match v {
        Value::Int(u) => g.obj(*u as u32),
        _ => None,
    }
}

/// `list(x)` d'une valeur de choix (tuple ou liste d'uid) ; vide sinon.
fn items(v: Option<&Value>) -> Vec<Value> {
    match v {
        Some(Value::List(x)) | Some(Value::Tuple(x)) => x.clone(),
        _ => vec![],
    }
}

/// `a[3]` si c'est un dict (choix d'une action).
fn choice(a: &Value) -> Option<&Map> {
    match a.list().get(3) {
        Some(Value::Map(m)) => Some(m),
        _ => None,
    }
}

/// `dict(a[3]).get("loc")`
fn choice_loc(a: &Value) -> Option<Loc> {
    choice(a).and_then(|m| m.get("loc")).and_then(loc_of)
}

/// `g.obj(a[3].get("tg", (None,))[0]) if a[3].get("tg") else None`
fn first_tg(g: &Game, a: &Value) -> Option<O> {
    let tg = items(choice(a).and_then(|m| m.get("tg")));
    tg.first().and_then(|u| uid_obj(g, u))
}

fn is_legend_src(a: &Value) -> bool {
    matches!(a.list().get(1), Some(Value::Tuple(_)))
}

fn kind(a: &Value) -> &str {
    a.list()[0].str()
}

/// `a[2]` d'un déplacement ou d'une carte cachée, comme emplacement.
fn dest(a: &Value) -> Option<Loc> {
    a.list().get(2).and_then(loc_of)
}

fn units_of(g: &Game, a: &Value) -> Vec<Option<O>> {
    a.list()[1].list().iter().map(|u| uid_obj(g, u)).collect()
}

fn is_reflection(g: &Game, u: O) -> bool {
    g.o(u).token && g.o(u).name == "Reflection"
}

fn sum_might(g: &mut Game, us: Vec<O>) -> Res<i64> {
    let mut t = 0;
    for u in us {
        t += 0.max(g.might(u)?);
    }
    Ok(t)
}

fn bool_idx(options: &[Value], b: bool) -> usize {
    options.iter().position(|o| *o == Value::Bool(b)).expect("choix « may » sans cette réponse")
}

fn first_min_f(keys: &[f64]) -> usize {
    let mut b = 0;
    for i in 1..keys.len() {
        if keys[i] < keys[b] {
            b = i;
        }
    }
    b
}

// ---------------------------------------------------------------------------------------- Akali (Gorica)
/// `AkaliGorica` : « Akali n'est pas un midrange agressif, c'est un deck contrôle lent orienté valeur ; les points ne
/// comptent pas, il faut juste ne pas perdre. » Moteur Stellacorn Herder + Astral Heron.
#[derive(Clone, Default)]
pub struct AkaliGorica;

const G_PROTECT: [&str; 3] = ["Zhonya's Hourglass", "Back Off", "Block"];

fn gorica_throw(g: &Game, pid: usize) -> Vec<O> {
    // renvoyer les sorts/équipements les plus chers, en gardant une première protection
    let mut cand: Vec<O> = g.p[pid].hand.iter().copied().filter(|&c| g.spec(c).type_ != "Unit").collect();
    cand.sort_by_key(|&c| -(g.spec(c).e as i64));
    let prot = cand.iter().copied().find(|&c| G_PROTECT.contains(&g.cname(c)));
    cand.into_iter().filter(|&c| Some(c) != prot).take(2).collect()
}

fn gorica_mulligan(g: &Game, pid: usize) -> Vec<O> {
    // Metafy (Gorica) : renvoyer 2 cartes sauf si la main a Stellacorn ET Heron ; ne jamais renvoyer d'unité.
    let h = &g.p[pid].hand;
    let has = |n: &str| h.iter().any(|&c| g.cname(c) == n);
    if has("Stellacorn Herder") && has("Astral Heron") {
        return vec![];
    }
    gorica_throw(g, pid)
}

fn gorica_prior(g: &mut Game, me: usize, a: &Value) -> Res<f64> {
    let k = kind(a);
    let turns = g.p[me].turns;
    let mut v = 0.0;
    if k == "act" && is_legend_src(a) {
        // légende
        let i = a.list()[2].int();
        if i == 0 {
            v -= if turns < 6 { 8.0 } else { 1.0 }; // Empower : rare, luxe de fin de partie
        } else if i == 1 {
            // Retreat : la boucle Herder
            let u = first_tg(g, a);
            if u.is_some_and(|u| g.cname(u) == "Stellacorn Herder") {
                v += 4.0;
            } else if u.is_some_and(|u| g.cname(u) == "Astral Heron") {
                v -= 3.0; // Heron reste sur le battlefield
            }
        }
    } else if k == "move" && dest(a).is_some_and(|l| l.is_bf()) {
        let d = dest(a);
        let units = units_of(g, a);
        let herder = units.iter().flatten().any(|&u| g.cname(u) == "Stellacorn Herder");
        let enemies = g.units(Some(1 - me), d);
        let leg_exh = g.o(g.p[me].legend).exh;
        if herder && !leg_exh && enemies.is_empty() {
            v += 3.0; // entrer pour piocher, puis rappeler
        }
        // pas de course aux points en début de partie : ne pas engager seul contre un défenseur
        if !enemies.is_empty() && turns <= 3 && units.len() == 1 {
            v -= 2.0;
        }
        if enemies.iter().any(|&e| g.cname(e) == "Karthus, Eternal") {
            v -= 1.5; // ne pas nourrir les Deathknell (BMU)
        }
    } else if k == "play" {
        let c = card_of(g, a.list()[1].int());
        let loc = choice_loc(a);
        if let Some(c) = c {
            if g.cname(c) == "Astral Heron" {
                if let Some(Loc::Bf(b)) = loc {
                    // Metafy : Heron sur un battlefield seulement avec une carte cachée pour la déclencher
                    let fd = g.bfs[b].facedown();
                    v += if fd.is_some_and(|f| g.o(f).owner == me) { 3.0 } else { -2.0 };
                }
            }
            if g.cname(c) == "Stellacorn Herder" {
                v += 1.5;
            }
        }
    } else if k == "hide" {
        let d = dest(a);
        if g.units(Some(me), None).iter().any(|&u| g.cname(u) == "Astral Heron" && g.o(u).loc == d) {
            v += 1.5; // carte cachée = réduction Heron gratuite
        }
    }
    Ok(v)
}

fn gorica_shape(g: &mut Game, me: usize) -> Res<f64> {
    if g.winner.is_some() {
        return Ok(0.0);
    }
    let mut v = 0.0;
    let us = g.units(Some(me), None);
    for &u in &us {
        if g.cname(u) == "Stellacorn Herder" {
            v += 2.0;
        } else if g.cname(u) == "Astral Heron" {
            v += if is_bf(g.o(u).loc) { 3.0 } else { 1.0 };
        }
    }
    v += 0.4 * g.p[me].hand.len() as f64; // avance de ressources
    if !us.iter().any(|&u| g.o(u).loc == Some(Loc::Base)) && us.iter().any(|&u| is_bf(g.o(u).loc)) {
        v -= 1.5; // ne jamais engager toutes ses unités
    }
    Ok(v)
}

fn gorica_battlefield(battlefields: &[String], first: bool, opp_bf: Option<&str>) -> Option<String> {
    // Metafy : Void Gate à l'aveugle ; contre LeBlanc garder Sigil pour ses parties Windswept
    let pick = if opp_bf == Some("Windswept Hillock") || (opp_bf.is_none() && !first) { "Sigil of the Storm" } else { "Void Gate" };
    battlefields.iter().any(|b| b == pick).then(|| pick.to_string())
}

impl Plan for AkaliGorica {
    fn name(&self) -> &str {
        "Gorica"
    }
    fn mulligan(&self, g: &mut Game, pid: usize) -> Res<Option<Vec<O>>> {
        Ok(Some(gorica_mulligan(g, pid)))
    }
    fn battlefield(&self, battlefields: &[String], first: bool, opp_bf: Option<&str>) -> Option<String> {
        gorica_battlefield(battlefields, first, opp_bf)
    }
    fn prior(&self, g: &mut Game, me: usize, a: &Value) -> Res<f64> {
        gorica_prior(g, me, a)
    }
    fn shape(&self, g: &mut Game, me: usize) -> Res<f64> {
        gorica_shape(g, me)
    }
}

/// `AkaliGoricaAggro` : mise à jour Metafy contre LeBlanc : agressif en milieu de partie, tuer Karthus à vue.
#[derive(Clone, Default)]
pub struct AkaliGoricaAggro;

const AGGRO_KEEP: [&str; 9] = [
    "Kai'Sa, Survivor",
    "Darius, Trifarian",
    "Blitzcrank, Impassive",
    "Shuriken Flip",
    "Falling Star",
    "Stellacorn Herder",
    "Back Off",
    "Akali, Silent",
    "Ferrous Forerunner",
];

impl Plan for AkaliGoricaAggro {
    fn name(&self) -> &str {
        "Gorica agressif"
    }
    fn mulligan(&self, g: &mut Game, pid: usize) -> Res<Option<Vec<O>>> {
        let mut h = g.p[pid].hand.clone();
        h.sort_by_key(|&c| -(g.spec(c).e as i64));
        let mut out = vec![];
        let mut herons = 0;
        for c in h {
            if g.cname(c) == "Astral Heron" {
                herons += 1;
                if herons > 1 {
                    out.push(c); // une seule Heron suffit dans ce plan
                }
            } else if !AGGRO_KEEP.contains(&g.cname(c)) && g.spec(c).type_ != "Unit" {
                out.push(c);
            }
        }
        out.truncate(2);
        Ok(Some(out))
    }
    fn battlefield(&self, battlefields: &[String], first: bool, opp_bf: Option<&str>) -> Option<String> {
        gorica_battlefield(battlefields, first, opp_bf)
    }
    fn prior(&self, g: &mut Game, me: usize, a: &Value) -> Res<f64> {
        let mut v = gorica_prior(g, me, a)?;
        let turns = g.p[me].turns;
        let k = kind(a);
        let opp_units = g.units(Some(1 - me), None);
        let karthus: Vec<Value> =
            opp_units.iter().filter(|&&u| g.cname(u) == "Karthus, Eternal").map(|&u| Value::from(g.o(u).uid)).collect();
        let ch = if a.list().len() > 3 { choice(a) } else { None };
        let tg = items(ch.and_then(|m| m.get("tg")));
        let mut tgs = tg.clone();
        tgs.extend(items(ch.and_then(|m| m.get("tg2"))));
        if !karthus.is_empty() && tgs.iter().any(|t| karthus.contains(t)) {
            v += 5.0; // tuer Karthus à vue
        }
        let mut temp: Vec<Value> = vec![];
        for &u in &opp_units {
            if g.has_kw(u, "Temporary")? {
                temp.push(Value::from(g.o(u).uid));
            }
        }
        if !temp.is_empty() && k == "play" && tg.iter().any(|t| temp.contains(t)) {
            v -= 2.0; // viser la source, pas le Reflet qui meurt seul
        }
        if k == "act" && !is_legend_src(a) {
            let o = uid_obj(g, &a.list()[1]);
            if o.is_some_and(|o| g.cname(o) == "Akali, Deadly Weapon") && a.list()[2].int() == 0 {
                if opp_units.iter().any(|&u| is_bf(g.o(u).loc)) {
                    v += 4.0; // empower pour tuer les unités sur les battlefields
                }
            }
        } else if k == "move" && dest(a).is_some_and(|l| l.is_bf()) && 3 <= turns {
            v += 1.5; // pression en milieu de partie
        }
        Ok(v)
    }
    fn shape(&self, g: &mut Game, me: usize) -> Res<f64> {
        if g.winner.is_some() {
            return Ok(0.0);
        }
        let mut v = gorica_shape(g, me)?;
        if g.units(Some(1 - me), None).iter().any(|&u| g.cname(u) == "Karthus, Eternal") {
            v -= 6.0;
        }
        let us: Vec<O> = g.units(Some(me), None).into_iter().filter(|&u| is_bf(g.o(u).loc)).collect();
        v += 0.3 * sum_might(g, us)? as f64;
        Ok(v)
    }
}

// ---------------------------------------------------------------------------------------- LeBlanc (Deathknell)
const NEVER_DISCARD: [&str; 5] = ["Watchful Sentry", "Karthus, Eternal", "Glasc Mixologist", "Ruined Rex", "Mirror Image"];
const DISCARD_ORDER: [&str; 10] = [
    "Sacrifice",
    "Black Rose Dignitary",
    "Baited Hook",
    "Harnessed Dragon",
    "Rift Herald",
    "Mirror Image",
    "Deathgrip",
    "Hidden Blade",
    "Honest Broker",
    "Soaring Scout",
];
const COPY_PREF: [&str; 8] = [
    "Glasc Mixologist",
    "LeBlanc, Fragmented",
    "Vi, Peacekeeper",
    "Rift Herald",
    "Harnessed Dragon",
    "Astral Heron",
    "Ruined Rex",
    "Thousand-Tailed Watcher",
];

/// `LeBlancDeathknell` : « Tu es le deck contrôle. Ne fais pas la course. » Corps Deathknell avant Karthus, Karthus à
/// la base, Glasc cible de copie, échelle Baited Hook, puis un « omega hold » sur un battlefield.
#[derive(Clone, Default)]
pub struct LeBlancDeathknell;

fn dk_mulligan(g: &Game, pid: usize) -> Vec<O> {
    let h = g.p[pid].hand.clone();
    let first = g.first == pid;
    let mut out = vec![];
    let mut karthus = 0;
    let two = h.iter().any(|&c| g.spec(c).type_ == "Unit" && g.spec(c).e <= 3 && DK.contains(&g.cname(c)));
    let mut by = h.clone();
    by.sort_by_key(|&c| -((g.spec(c).e + g.spec(c).p) as i64));
    for c in by {
        let n = g.cname(c);
        if ["Harnessed Dragon", "Rift Herald", "Thousand-Tailed Watcher"].contains(&n) {
            out.push(c);
        } else if n == "Karthus, Eternal" {
            karthus += 1;
            if karthus > 1 {
                out.push(c);
            }
        } else if n == "Hidden Blade" && first {
            out.push(c);
        }
    }
    if first && !two {
        let extra: Vec<O> = h.iter().copied().filter(|c| !out.contains(c) && g.spec(*c).e >= 5).take(2).collect();
        out.extend(extra);
    }
    out.truncate(2);
    out
}

fn dk_prior(g: &mut Game, me: usize, a: &Value) -> Res<f64> {
    let k = kind(a);
    let mut v = 0.0;
    let turns = g.p[me].turns;
    if k == "play" {
        let Some(c) = card_of(g, a.list()[1].int()) else { return Ok(0.0) };
        let n = g.cname(c).to_string();
        let loc = choice_loc(a);
        if turns == 1
            && ["Soaring Scout", "Black Rose Dignitary", "LeBlanc, Fragmented", "Honest Broker", "Watchful Sentry"]
                .contains(&n.as_str())
        {
            v += 3.0; // toujours jouer au tour 1
        }
        if video() && n == "Thousand-Tailed Watcher" {
            let opp_bf = g.units(Some(1 - me), None).iter().filter(|&&u| is_bf(g.o(u).loc)).count();
            if opp_bf >= 2 && !g.bfs.iter().any(|b| b.ctrl == Some(me)) {
                v += 2.0; // closer : l'adversaire a tout poussé, on est dehors
            }
        }
        if n == "Karthus, Eternal" {
            if !g.units(Some(me), None).iter().any(|&u| DK.contains(&g.cname(u)) || g.cname(u) == "Reflection") {
                v -= 3.0; // corps Deathknell d'abord
            }
            if loc.is_some_and(|l| l.is_bf()) {
                v -= 3.0; // Karthus reste à la base
            }
        }
    } else if k == "move" && dest(a).is_some_and(|l| l.is_bf()) {
        let d = dest(a);
        let units = units_of(g, a);
        if units.iter().any(|u| u.is_some_and(|u| g.cname(u) == "Karthus, Eternal")) {
            v -= 4.0;
        }
        let Some(Loc::Bf(b)) = d else { unreachable!() };
        if video()
            && g.bfs[b].ctrl != Some(me)
            && units.iter().any(|u| {
                u.is_some_and(|u| is_reflection(g, u) && is_bf(g.o(u).loc) && g.o(u).loc != d)
            })
        {
            v += 2.0; // Riftlab : le Reflet prêt gank l'autre battlefield
        }
    } else if k == "hide" && keep_reflection() {
        // joueurs réels (utilisateur, 3 oct.) : LeBlanc cache une carte là où il tient, même avec un seul Reflet
        v += 2.5;
        if g.units(Some(me), dest(a)).iter().all(|&u| is_reflection(g, u)) {
            v += 1.5;
        }
    } else if k == "move" && dest(a) == Some(Loc::Base) && keep_reflection() {
        // joueurs réels (utilisateur, 3 oct.) : le Reflet et l'unité copiée restent sur le battlefield
        let refl: Vec<O> =
            g.units(Some(me), None).into_iter().filter(|&u| is_reflection(g, u) && is_bf(g.o(u).loc)).collect();
        for u in units_of(g, a) {
            if let Some(u) = u {
                if refl.iter().any(|&r| {
                    u == r || (g.o(r).copy.as_deref() == Some(g.cname(u)) && g.o(u).loc == g.o(r).loc)
                }) {
                    v -= 8.0;
                    break;
                }
            }
        }
    }
    Ok(v)
}

fn dk_shape(g: &mut Game, me: usize) -> Res<f64> {
    if g.winner.is_some() {
        return Ok(0.0);
    }
    let mut v = 0.0;
    for u in g.units(Some(me), None) {
        if g.cname(u) == "Karthus, Eternal" {
            v += if g.o(u).loc == Some(Loc::Base) { 1.5 } else { -1.0 };
        }
    }
    // omega hold : de la might sur un battlefield qu'on tient
    for b in 0..g.bfs.len() {
        if g.bfs[b].ctrl == Some(me) {
            let us = g.units(Some(me), Some(Loc::Bf(g.bfs[b].idx)));
            v += 0.25 * sum_might(g, us)? as f64;
        }
    }
    Ok(v)
}

/// `_spare_karthus(g, pid)` : deux Karthus en main et un Glasc encore disponible (jeu, main ou deck).
fn spare_karthus(g: &Game, pid: usize) -> bool {
    let pl = &g.p[pid];
    if pl.hand.iter().filter(|&&c| g.cname(c) == "Karthus, Eternal").count() < 2 {
        return false;
    }
    g.units(Some(pid), None).iter().any(|&u| g.cname(u) == "Glasc Mixologist")
        || pl.hand.iter().chain(pl.deck.iter()).any(|&c| g.cname(c) == "Glasc Mixologist")
}

fn dk_choose(g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<Option<usize>> {
    if kind == "may" {
        if let Value::Item(it) = ctx.g("item") {
            if g.it(it).name.starts_with("LeBlanc, Deceiver") {
                // ne pas cloner si cela force à défausser une carte clé
                let r = g.p[pid].hand.iter().any(|&c| !NEVER_DISCARD.contains(&g.cname(c)))
                    || (video() && spare_karthus(g, pid));
                return Ok(Some(bool_idx(options, r)));
            }
        }
        return Ok(None);
    }
    if kind == "discard" && ctx.g("reason").opt_str() == Some("leblanc") {
        let mut big = false;
        for u in g.units(Some(pid), None) {
            if g.might(u)? >= 5 {
                big = true;
                break;
            }
        }
        let spare_k = video() && spare_karthus(g, pid);
        let rank = |c: O| -> f64 {
            let n = g.cname(c);
            if n == "Sacrifice" && big {
                return 50.0;
            }
            if n == "Karthus, Eternal" && spare_k {
                return 30.0; // « un Karthus est plus sûr en défausse » (Glasc le ramène)
            }
            if NEVER_DISCARD.contains(&n) {
                return 100.0;
            }
            DISCARD_ORDER.iter().position(|&x| x == n).map(|i| i as f64).unwrap_or(40.0)
        };
        let k: Vec<f64> = options.iter().map(|o| rank(o.obj())).collect();
        return Ok(Some(first_min_f(&k)));
    }
    if kind == "copy_target" {
        let pref: Vec<&str> = if video() {
            let mut p = vec!["Ruined Rex", "Thousand-Tailed Watcher"];
            p.extend(COPY_PREF.iter().copied().filter(|n| *n != "Ruined Rex" && *n != "Thousand-Tailed Watcher"));
            p
        } else {
            COPY_PREF.to_vec()
        };
        let mut k = vec![];
        for o in options {
            let u = o.obj();
            let r = match pref.iter().position(|&x| x == g.cname(u)) {
                Some(i) => i as f64,
                None => 50.0 - crate::cards_helpers::value(g, u)?,
            };
            k.push(r);
        }
        return Ok(Some(first_min_f(&k)));
    }
    Ok(None)
}

impl Plan for LeBlancDeathknell {
    fn name(&self) -> &str {
        "Deathknell"
    }
    fn mulligan(&self, g: &mut Game, pid: usize) -> Res<Option<Vec<O>>> {
        Ok(Some(dk_mulligan(g, pid)))
    }
    fn battlefield(&self, battlefields: &[String], first: bool, _opp_bf: Option<&str>) -> Option<String> {
        let pick = if first { "Windswept Hillock" } else { "Star Spring" };
        battlefields.iter().any(|b| b == pick).then(|| pick.to_string())
    }
    fn prior(&self, g: &mut Game, me: usize, a: &Value) -> Res<f64> {
        dk_prior(g, me, a)
    }
    fn shape(&self, g: &mut Game, me: usize) -> Res<f64> {
        dk_shape(g, me)
    }
    fn choose(&self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<Option<usize>> {
        dk_choose(g, pid, kind, options, ctx)
    }
}

/// `LeBlancHookTempo` : liste IQ #5, version occidentale « tempo » construite autour de Baited Hook ; LeBlanc doit
/// occuper les battlefields (le moteur du Deceiver tourne sur conquête ou tenue).
#[derive(Clone, Default)]
pub struct LeBlancHookTempo;

fn hook_mulligan(g: &Game, pid: usize) -> Vec<O> {
    let mut out = dk_mulligan(g, pid);
    let h = g.p[pid].hand.clone();
    if !h.iter().filter(|c| !out.contains(c)).any(|&c| g.spec(c).type_ == "Unit" && g.spec(c).e <= 3) {
        let mut by = h.clone();
        by.sort_by_key(|&c| -(g.spec(c).e as i64));
        let extra: Vec<O> = by.into_iter().filter(|c| !out.contains(c) && g.spec(*c).e >= 5).collect();
        out.extend(extra);
    }
    out.truncate(2);
    out
}

fn hook_prior(g: &mut Game, me: usize, a: &Value) -> Res<f64> {
    let mut v = dk_prior(g, me, a)?;
    let k = kind(a);
    if k == "move" && dest(a).is_some_and(|l| l.is_bf()) {
        let units = units_of(g, a);
        if units.iter().all(|u| u.is_some_and(|u| g.cname(u) != "Karthus, Eternal")) {
            v += if g.p[me].turns <= 6 { 2.0 } else { 1.0 }; // conquérir pour déclencher la légende
        }
    } else if k == "act" && !is_legend_src(a) {
        let o = uid_obj(g, &a.list()[1]);
        let t = first_tg(g, a);
        if let (Some(o), Some(t)) = (o, t) {
            if g.cname(o) == "Baited Hook" {
                if DK.contains(&g.cname(t)) || g.has_kw(t, "Temporary")? {
                    v += 2.5; // échelle Hook sur un corps Deathknell ou un Reflet
                } else if g.cname(t) == "Karthus, Eternal" {
                    v -= 6.0;
                }
            }
        }
    }
    Ok(v)
}

fn hook_shape(g: &mut Game, me: usize) -> Res<f64> {
    if g.winner.is_some() {
        return Ok(0.0);
    }
    let mut v = dk_shape(g, me)?;
    for b in 0..g.bfs.len() {
        if g.bfs[b].ctrl == Some(me) {
            let mut any = false;
            for u in g.units(Some(me), Some(Loc::Bf(g.bfs[b].idx))) {
                if !g.has_kw(u, "Temporary")? {
                    any = true;
                    break;
                }
            }
            if any {
                v += 1.5; // tenue = nouveau Reflet au prochain tour
            }
        }
    }
    Ok(v)
}

impl Plan for LeBlancHookTempo {
    fn name(&self) -> &str {
        "Hook tempo"
    }
    fn mulligan(&self, g: &mut Game, pid: usize) -> Res<Option<Vec<O>>> {
        Ok(Some(hook_mulligan(g, pid)))
    }
    fn battlefield(&self, battlefields: &[String], first: bool, opp_bf: Option<&str>) -> Option<String> {
        LeBlancDeathknell.battlefield(battlefields, first, opp_bf)
    }
    fn prior(&self, g: &mut Game, me: usize, a: &Value) -> Res<f64> {
        hook_prior(g, me, a)
    }
    fn shape(&self, g: &mut Game, me: usize) -> Res<f64> {
        hook_shape(g, me)
    }
    fn choose(&self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<Option<usize>> {
        dk_choose(g, pid, kind, options, ctx)
    }
}

/// `LeBlancHookWindswept` : Hook tempo qui présente toujours Windswept Hillock.
#[derive(Clone, Default)]
pub struct LeBlancHookWindswept;

impl Plan for LeBlancHookWindswept {
    fn name(&self) -> &str {
        "Hook tempo Windswept"
    }
    fn mulligan(&self, g: &mut Game, pid: usize) -> Res<Option<Vec<O>>> {
        Ok(Some(hook_mulligan(g, pid)))
    }
    fn battlefield(&self, battlefields: &[String], _first: bool, _opp_bf: Option<&str>) -> Option<String> {
        battlefields.iter().any(|b| b == "Windswept Hillock").then(|| "Windswept Hillock".to_string())
    }
    fn prior(&self, g: &mut Game, me: usize, a: &Value) -> Res<f64> {
        hook_prior(g, me, a)
    }
    fn shape(&self, g: &mut Game, me: usize) -> Res<f64> {
        hook_shape(g, me)
    }
    fn choose(&self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<Option<usize>> {
        dk_choose(g, pid, kind, options, ctx)
    }
}

/// Noms de `PLANS` (ordre du dict Python).
pub const PLAN_NAMES: [&str; 5] = ["Gorica", "Hook tempo", "Hook tempo Windswept", "Gorica agressif", "Deathknell"];

/// `PLANS[name]()` : None si le nom est inconnu.
pub fn plans(name: &str) -> Option<Arc<dyn Plan>> {
    Some(match name {
        "Gorica" => Arc::new(AkaliGorica),
        "Hook tempo" => Arc::new(LeBlancHookTempo),
        "Hook tempo Windswept" => Arc::new(LeBlancHookWindswept),
        "Gorica agressif" => Arc::new(AkaliGoricaAggro),
        "Deathknell" => Arc::new(LeBlancDeathknell),
        _ => return None,
    })
}

// ---------------------------------------------------------------------------------------- agents
/// `PlanPolicy(plan)` : politique de simulation qui fait les choix de résolution du plan (défausse, copie...).
pub fn plan_policy(plan: Option<Arc<dyn Plan>>) -> crate::ai::PolicyAgent {
    crate::ai::PolicyAgent { cfg: Default::default(), plan }
}

/// `PlanAgent(seed, plan, opp_plan, **kw)` : plan None = `Plan()`.
pub fn plan_agent(seed: i128, plan: Option<Arc<dyn Plan>>, opp_plan: Option<Arc<dyn Plan>>, o: SearchOpts) -> SearchAgent {
    let mut s = SearchAgent::new(seed, o);
    s.plan = Some(plan.unwrap_or_else(|| Arc::new(NoPlan)));
    s.opp_plan = opp_plan;
    s
}
