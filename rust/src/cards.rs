//! Registre des comportements de cartes (`cards.py` : `Impl`, `IMPL`, `HOOKS`, `AURA`, `ADDERS`...).
//! Une carte = une entrée `card(nom, Impl { ... })` dans un module de `cardsets/`. Les champs qui sont des fonctions
//! en Python sont des `Value::Fn` (arguments et retour en `Value`), pour traduire chaque carte au plus près.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use crate::spec::specs;
use crate::value::{Map, Value};

/// Champs de `Impl` lus par le moteur sur les *autres* objets (`HOOK_FIELDS` de `cards.py`) : chaque carte qui en
/// définit un est rangée dans `HOOKS[champ]` (`Game::providers`).
pub const HOOK_FIELDS: &[&str] = &[
    "death_rep", "fx_death_rep",
    "dmg_bonus", "fx_dmg_bonus", "dmg_prevent", "aura_dmg_prevent", "lethal_any",
    "no_combat_damage", "aura_no_combat", "ignore_tank",
    "cost_aura",
    "extra_locs", "aura_locs", "loc_veto", "only_locs", "forbid_play", "card_kw",
    "units_enter_ready", "cant_move", "fx_cant_move", "aura_cant_move", "move_tax",
    "grant_abilities", "aura_untargetable", "aura_tags",
    "no_ready", "aura_no_ready", "channel_mod", "score_veto", "point_rep", "scoring_extra", "fx_swap_scoring",
    "choice_rep", "minus_extra", "no_counter", "gold_extra", "hide_cost", "hide_slots", "token_rep",
    "grant_add",
    "zone_event", "on_seen", "on_reveal", "reveal_rep",
    "dk_extra", "channel_rep", "move_cost", "extra_hide_slots", "spell_opts", "opt_pay", "token_swap",
];

/// `cards.Impl`. Les champs fonction valent `Value::None` quand la carte ne les définit pas.
#[derive(Clone, Debug, Default)]
pub struct Impl {
    pub name: String,
    pub timing: Option<&'static str>,
    pub hidden: bool,
    pub ambush: bool,
    pub accelerate: bool,
    pub quickdraw: bool,
    pub flow: Value,
    pub repeat: Value,
    /// mots-clés statiques {mot-clé: valeur}
    pub kw: Map,
    pub choices: Value,
    pub resolve: Value,
    pub on_play: Value,
    pub deathknell: Value,
    pub on_event: Value,
    pub might_mod: Value,
    pub untargetable: Value,
    pub cost_mod: Value,
    pub as_played: Value,
    pub bf_event: Value,
    pub legend_event: Value,
    pub effect_event: Value,
    pub no_score: Value,
    /// prédicats de cible revérifiés à la résolution (liste de fonctions) ou None
    pub preds: Value,
    pub dk_choose: Value,
    /// capacités activées (dict par capacité)
    pub abilities: Vec<Map>,
    pub uncounterable: bool,
    pub banish_after: bool,
    pub max_choices: i64,
    /// [(cond(g, o), {kw: valeur})]
    pub kw_if: Vec<(Value, Map)>,
    /// [(cond(g, o), montant)]
    pub might_if: Vec<(Value, i64)>,
    /// [(N, dict(might=, kw={}, ready=True))]
    pub levels: Vec<(i64, Map)>,
    pub aura_kw: Value,
    pub aura_might: Value,
    pub equip_kw: Option<Map>,
    /// False, True ou fonction
    pub enter_ready: Value,
    pub enter_exhausted: bool,
    pub enter_loc: Value,
    /// capacités [Add] : dict(e=, p=[...], exhaust=, kill=, can=...)
    pub add: Vec<Map>,
    pub track_mighty: bool,
    pub ignore_deflect: bool,
    pub extra_cost_fn: Value,
    pub pay_extra: Value,
    pub auto: bool,
    pub module: Option<&'static str>,
    /// champs de HOOK_FIELDS (fonction ou valeur)
    pub hooks: Map,
    pub multi_buff: bool,
    pub assign_last: bool,
    pub tie_recall_all: bool,
    pub equip_tags: Option<Vec<String>>,
    pub might_watch: Option<i64>,
    pub fx_deathknell: Value,
    pub on_discard: Value,
    pub on_leave: Value,
    pub opt_parts: Value,
    pub repeat_all: Value,
    pub all_choices: Value,
    pub alt_costs: Value,
    pub show_off: Value,
    pub show_key: Value,
    /// bonus de might d'un équipement (`bonus=` : ajouté à EQUIP_BONUS)
    pub bonus: Option<i64>,
}

impl Impl {
    pub fn new(name: &str) -> Self {
        Impl { name: name.to_string(), max_choices: 8, enter_ready: Value::Bool(false), ..Default::default() }
    }
    /// valeur d'un champ de HOOK_FIELDS (None si absent)
    pub fn hook(&self, field: &str) -> Value {
        self.hooks.g(field)
    }
    pub fn kw(&self, k: &str) -> i64 {
        self.kw.get(k).map(|v| v.int()).unwrap_or(0)
    }
}

/// `IMPL`, `HOOKS`, `AURA`, `ADDERS`, `CONVERTERS`, `TRACK_MIGHTY` et les titres des questions (`ASK_TEXT`).
#[derive(Default)]
pub struct Registry {
    pub impls: BTreeMap<String, Impl>,
    pub hooks: BTreeMap<String, BTreeSet<String>>,
    pub aura: BTreeSet<String>,
    pub adders: BTreeSet<String>,
    pub converters: bool,
    pub track_mighty: bool,
    pub ask_text: BTreeMap<String, String>,
}

impl Registry {
    /// `card(name, **kw)` : enregistre le comportement d'une carte (un nom enregistré deux fois est une erreur).
    pub fn card(&mut self, im: Impl) {
        let name = im.name.clone();
        if let Some(prev) = self.impls.get(&name) {
            panic!("card {name:?} is already registered (by {}); a card has one Impl", prev.module.unwrap_or("cards.rs"));
        }
        if !specs().contains_key(&name) {
            panic!("card {name:?} is not in the card database (game.SPEC)");
        }
        if !im.aura_kw.is_none() || !im.aura_might.is_none() {
            self.aura.insert(name.clone());
        }
        if !im.add.is_empty() {
            self.adders.insert(name.clone());
        }
        if im.track_mighty {
            self.track_mighty = true;
        }
        for f in HOOK_FIELDS {
            if im.hook(f).truthy() {
                self.hooks.entry(f.to_string()).or_default().insert(name.clone());
            }
        }
        if im.tie_recall_all {
            self.hooks.entry("tie_recall_all".into()).or_default().insert(name.clone());
        }
        if im.might_watch.is_some_and(|n| n != 0) {
            self.hooks.entry("might_watch".into()).or_default().insert(name.clone());
        }
        if im.add.iter().any(|ad| ad.g("conv").truthy()) {
            self.converters = true;
        }
        self.impls.insert(name, im);
    }

    /// `ask_text(**kinds)`
    pub fn ask_text(&mut self, kinds: &[(&str, &str)]) {
        for (k, v) in kinds {
            if let Some(prev) = self.ask_text.get(*k) {
                if prev != v {
                    panic!("ask kind {k:?} already has the title {prev:?}");
                }
            }
            self.ask_text.insert(k.to_string(), v.to_string());
        }
    }

    pub fn hook_names(&self, field: &str) -> Option<&BTreeSet<String>> {
        self.hooks.get(field).filter(|s| !s.is_empty())
    }
}

/// Le registre, construit une fois par `cardsets::register_all` (ordre de `cardsets/__init__.py : MODULES`), en
/// lecture seule ensuite : partagé par toutes les parties et leurs copies.
pub fn registry() -> &'static Registry {
    static R: OnceLock<Registry> = OnceLock::new();
    R.get_or_init(|| {
        let mut r = Registry::default();
        crate::cardsets::register_all(&mut r);
        r
    })
}

pub fn impl_of(name: &str) -> Option<&'static Impl> {
    registry().impls.get(name)
}
