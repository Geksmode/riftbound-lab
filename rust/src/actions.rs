//! Actions discrétionnaires légales et processus de jeu d'une carte ou d'une capacité (`actions.py`, règles 353-359,
//! 398-406). Portage en cours.
use crate::game::{Game, Res};
use crate::value::Value;

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

pub fn main_options(_g: &mut Game, _pid: usize) -> Res<Vec<Value>> {
    todo!("portage : actions.main_options")
}

pub fn timed_options(_g: &mut Game, _pid: usize, _priority: bool) -> Res<Vec<Value>> {
    todo!("portage : actions.timed_options")
}

pub fn do_action(_g: &mut Game, _a: &Value) -> Res<()> {
    todo!("portage : actions.do_action")
}
