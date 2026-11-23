//! Fonctions de `game.py` pas encore portées (provisoire : retirées au fur et à mesure du portage).
use crate::game::{Game, Res, O};
use crate::value::{Map, Value};

impl Game {
    pub fn emit(&mut self, _ev: &str, _info: Map) -> Res<()> {
        todo!("portage : Game.emit")
    }
    pub fn replace_choice_effect(&mut self, _o: O, _what: &str, _duration: Value) -> Res<bool> {
        todo!("portage : Game.replace_choice_effect")
    }
    pub fn burn_out(&mut self, _pid: usize) -> Res<()> {
        todo!("portage : Game.burn_out")
    }
}
