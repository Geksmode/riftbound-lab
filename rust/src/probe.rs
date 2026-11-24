//! Sonde des combats de l'IA (`combats_parties.py`, 2026-10-11) : en Python, le script remplace `Game.start_showdown`,
//! `Game.combat_step` et `SearchAgent.pick_turn` par des enveloppes qui notent ce qu'elles voient. Ici, ces trois points
//! du moteur appellent la sonde quand elle est allumée dans le fil courant (un test de `Cell`, sans effet sur le jeu).
//! Rien n'est noté dans les copies de la recherche (`logging` faux), comme en Python.
use std::cell::{Cell, RefCell};

use crate::game::Game;
use crate::json::Json;
use crate::jo;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static REC: RefCell<Vec<((i64, usize), Json)>> = const { RefCell::new(Vec::new()) };
    static PICKS: RefCell<Vec<Json>> = const { RefCell::new(Vec::new()) };
}

/// La sonde est-elle allumée dans ce fil ?
pub fn active() -> bool {
    ON.with(|o| o.get())
}

/// Allume (et remet à zéro) la sonde du fil courant.
pub fn start() {
    ON.with(|o| o.set(true));
    REC.with(|r| r.borrow_mut().clear());
    PICKS.with(|r| r.borrow_mut().clear());
}

/// Éteint la sonde ; rend les combats notés (par (tour, battlefield), dans l'ordre d'insertion d'un dict) et les choix
/// de `pick_turn`.
pub fn stop() -> (Vec<((i64, usize), Json)>, Vec<Json>) {
    ON.with(|o| o.set(false));
    (REC.with(|r| std::mem::take(&mut *r.borrow_mut())), PICKS.with(|r| std::mem::take(&mut *r.borrow_mut())))
}

/// `units(g, bf, pid)` : [{n, m, st}] des unités de pid au battlefield bf
fn units(g: &mut Game, bf: usize, pid: usize) -> Json {
    let us = g.units(Some(pid), Some(crate::value::Loc::Bf(bf)));
    let mut out = vec![];
    for u in us {
        let m = g.might(u).unwrap_or(0);
        out.push(jo! { "n" => g.cname(u), "m" => m, "st" => g.o(u).stunned });
    }
    Json::Arr(out)
}

fn side(g: &mut Game, bf: usize) -> Json {
    jo! { "ai" => units(g, bf, 1), "hu" => units(g, bf, 0) }
}

/// Avant `start_showdown(b, combat)` : le battlefield est contesté par l'IA (P1).
pub fn on_start_showdown(g: &mut Game, b: usize) {
    if !g.logging || g.bfs[b].contested != Some(1) {
        return;
    }
    let ready = g.p[0].runes.iter().filter(|&&r| !g.r(r).exhausted).count();
    let r = jo! {
        "turn" => g.turn_no, "tp" => g.tp, "bf" => g.bfs[b].name.clone(), "pts" => vec![g.p[0].points, g.p[1].points],
        "before" => side(g, b), "hu_ready" => ready, "hu_hand" => g.p[0].hand.len(),
    };
    let key = (g.turn_no, b);
    REC.with(|rec| {
        let mut rec = rec.borrow_mut();
        match rec.iter_mut().find(|e| e.0 == key) {
            Some(e) => e.1 = r,
            None => rec.push((key, r)),
        }
    });
}

fn with_rec(g: &Game, f: impl FnOnce(&mut Json)) {
    let (key, att) = match &g.sd {
        Some(sd) => ((g.turn_no, sd.bf), sd.attacker),
        None => return,
    };
    if !g.logging || att != 1 {
        return;
    }
    REC.with(|rec| {
        if let Some(e) = rec.borrow_mut().iter_mut().find(|e| e.0 == key) {
            f(&mut e.1);
        }
    });
}

fn has_rec(g: &Game) -> bool {
    let Some(sd) = &g.sd else { return false };
    let key = (g.turn_no, sd.bf);
    g.logging && sd.attacker == 1 && REC.with(|rec| rec.borrow().iter().any(|e| e.0 == key))
}

/// Entrée de `combat_step` à l'étape « damage » : les unités au moment des dégâts.
pub fn on_combat_damage(g: &mut Game) {
    if !has_rec(g) {
        return;
    }
    let bf = g.sd.as_ref().unwrap().bf;
    let s = side(g, bf);
    with_rec(g, |r| r.set("damage", s));
}

/// `combat_step` vient de passer à l'étape « end » : les unités restantes et si l'attaque a été rappelée.
pub fn on_combat_end(g: &mut Game) {
    if !has_rec(g) {
        return;
    }
    let (bf, recalled) = {
        let sd = g.sd.as_ref().unwrap();
        (sd.bf, sd.recalled)
    };
    let mut s = side(g, bf);
    s.set("recalled", recalled);
    with_rec(g, |r| {
        if r.get("after").is_none() {
            r.set("after", s);
        }
    });
}

/// Après `SearchAgent.pick_turn` : (tour, premier coup, valeur, suite du tour), les coups en `repr` Python.
pub fn on_pick_turn(turn: i64, first: String, v: f64, plan: Vec<String>) {
    PICKS.with(|p| p.borrow_mut().push(jo! { "turn" => turn, "first" => first, "v" => v, "plan" => plan }));
}
