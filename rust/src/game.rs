//! État de partie et règles (`game.py`). Traduction au plus près : mêmes fonctions (nom Python cité), même ordre
//! d'opérations, mêmes tris. Numéros de règle : Core Rules du 2026-07-16.
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::cards::{impl_of, registry, Impl};
use crate::pyrandom::PyRandom;
use crate::spec::{equip_bonus, spec, ugear_specs, Spec, VICTORY};
use crate::value::{is_bf, Loc, Map, Value};

/// Fin de partie (exception `GameOver` de Python) : remonte jusqu'à `advance` / `apply`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameOver;
pub type Res<T> = Result<T, GameOver>;

/// Poignée d'un objet (carte ou jeton) dans l'arène `Game::objs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct O(pub u32);
/// Poignée d'une rune dans `Game::runes_arena`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct R(pub u32);
/// Poignée d'un item de chaîne dans `Game::items`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub u32);

/// Compteurs globaux de Python (`Obj._n`, `Item._n`) : partagés entre une partie et ses copies, comme en Python où
/// les simulations de l'IA font avancer le même compteur que la partie réelle.
#[derive(Debug, Default)]
pub struct Counters {
    pub obj: AtomicU32,
    pub item: AtomicU32,
}

impl Counters {
    pub fn next_obj(&self) -> u32 {
        self.obj.fetch_add(1, Ordering::Relaxed) + 1
    }
    pub fn next_item(&self) -> u32 {
        self.item.fetch_add(1, Ordering::Relaxed) + 1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Deck,
    Hand,
    Trash,
    Banish,
    Champ,
    Legend,
    Board,
    Chain,
    Facedown,
}

impl Zone {
    pub fn name(self) -> &'static str {
        match self {
            Zone::Deck => "deck",
            Zone::Hand => "hand",
            Zone::Trash => "trash",
            Zone::Banish => "banish",
            Zone::Champ => "champ",
            Zone::Legend => "legend",
            Zone::Board => "board",
            Zone::Chain => "chain",
            Zone::Facedown => "facedown",
        }
    }
    pub fn parse(s: &str) -> Zone {
        match s {
            "deck" => Zone::Deck,
            "hand" => Zone::Hand,
            "trash" => Zone::Trash,
            "banish" => Zone::Banish,
            "champ" => Zone::Champ,
            "legend" => Zone::Legend,
            "board" => Zone::Board,
            "chain" => Zone::Chain,
            "facedown" => Zone::Facedown,
            z => panic!("zone inconnue {z}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Desig {
    Att,
    Def,
}

/// `collections.Counter` à clés texte : ordre d'insertion, valeur 0 pour une clé absente.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counter(pub Vec<(String, i64)>);

impl Counter {
    pub fn get(&self, k: &str) -> i64 {
        self.0.iter().find(|(x, _)| x == k).map(|(_, v)| *v).unwrap_or(0)
    }
    pub fn add(&mut self, k: &str, n: i64) {
        if let Some(e) = self.0.iter_mut().find(|(x, _)| x == k) {
            e.1 += n;
        } else {
            self.0.push((k.to_string(), n));
        }
    }
    /// `bool(counter)` : vrai dès qu'une clé existe (même à 0).
    pub fn truthy(&self) -> bool {
        !self.0.is_empty()
    }
    /// `+counter` : seulement les valeurs positives.
    pub fn positive(&self) -> Counter {
        Counter(self.0.iter().filter(|(_, v)| *v > 0).cloned().collect())
    }
}

/// `game.Obj` : une carte ou un jeton, dans n'importe quelle zone. L'état propre au plateau est remis à zéro à
/// chaque changement de zone (règle 359.3.e.4).
#[derive(Clone, Debug)]
pub struct Obj {
    pub uid: u32,
    pub name: String,
    pub owner: usize,
    pub ctrl: usize,
    pub token: bool,
    pub zone: Option<Zone>,
    /// compteur d'incarnation : une cible n'est légale que pour la même incarnation
    pub oid: i64,
    pub drawn_at: Option<i64>,
    pub loc: Option<Loc>,
    pub exh: bool,
    /// `o._g` posé : l'objet est entré sur le plateau d'une partie (Obj.exhausted)
    pub on_game: bool,
    pub ugear: bool,
    pub damage: i64,
    pub buff: i64,
    pub stunned: bool,
    pub empowered: bool,
    /// [montant, durée]
    pub mods: Vec<(i64, Value)>,
    /// [mot-clé, valeur, durée]
    pub grants: Vec<(String, i64, Value)>,
    /// uids des équipements attachés
    pub attached: Vec<u32>,
    pub attached_to: Option<u32>,
    pub desig: Option<Desig>,
    pub prevent: i64,
    pub copy: Option<String>,
    pub entered_turn: Option<i64>,
    pub hidden_turn: Option<i64>,
    pub hidden_bf: Option<Loc>,
    pub was_mighty: Option<bool>,
    pub might_seen: Option<i64>,
    /// pid -> dégâts marqués ce tour par les sources de ce joueur (ordre d'insertion)
    pub dmg_by: Vec<(usize, i64)>,
    pub last_dmg: Option<(usize, Value)>,
    /// [capacité, durée]
    pub abs: Vec<(Map, Value)>,
    /// [tag, durée]
    pub xtags: Vec<(String, Value)>,
    pub copy_src: Option<O>,
    pub copy_name: Option<String>,
    pub chosen_tag: Option<String>,
    // attributs posés par `actions.play_card` / `cards.make_token` (jamais remis à zéro, comme en Python)
    /// `played_to` : où la carte est jouée (Riven, The Exile)
    pub played_to: Option<Loc>,
    /// `played_from` : zone d'où elle a été jouée ('hand', 'facedown'...)
    pub played_from: Option<String>,
    /// `play_choice` : le choix avec lequel elle a été jouée
    pub play_choice: Option<Map>,
    /// `paid_e` : énergie payée pour la jouer
    pub paid_e: i64,
    /// `limited_play` : jouée par un effet (play_card limited=True)
    pub limited_play: bool,
    /// attributs libres posés par les cartes (`setattr(o, ...)` en Python) : jamais remis à zéro, copiés avec la partie
    pub attrs: Map,
}

impl Obj {
    fn new(uid: u32, name: &str, owner: usize, token: bool) -> Obj {
        let mut o = Obj {
            uid,
            name: name.to_string(),
            owner,
            ctrl: owner,
            token,
            zone: None,
            oid: 0,
            drawn_at: None,
            loc: None,
            exh: false,
            on_game: false,
            ugear: false,
            damage: 0,
            buff: 0,
            stunned: false,
            empowered: false,
            mods: vec![],
            grants: vec![],
            attached: vec![],
            attached_to: None,
            desig: None,
            prevent: 0,
            copy: None,
            entered_turn: None,
            hidden_turn: None,
            hidden_bf: None,
            was_mighty: None,
            might_seen: None,
            dmg_by: vec![],
            last_dmg: None,
            abs: vec![],
            xtags: vec![],
            copy_src: None,
            copy_name: None,
            chosen_tag: None,
            played_to: None,
            played_from: None,
            play_choice: None,
            paid_e: 0,
            limited_play: false,
            attrs: Map::new(),
        };
        o.reset_fields();
        o
    }

    /// `Obj.reset` sans l'effet de bord de `exhausted` (fait par `Game::reset_obj`).
    fn reset_fields(&mut self) {
        self.loc = None;
        self.ugear = false;
        self.damage = 0;
        self.buff = 0;
        self.stunned = false;
        self.empowered = false;
        self.mods.clear();
        self.grants.clear();
        self.attached.clear();
        self.attached_to = None;
        self.desig = None;
        self.prevent = 0;
        self.copy = None;
        self.ctrl = self.owner;
        self.entered_turn = None;
        self.hidden_turn = None;
        self.hidden_bf = None;
        self.was_mighty = None;
        self.might_seen = None;
        self.dmg_by.clear();
        self.last_dmg = None;
        self.abs.clear();
        self.xtags.clear();
        self.copy_src = None;
        self.copy_name = None;
        self.chosen_tag = None;
    }

    pub fn dmg_by(&self, pid: usize) -> i64 {
        self.dmg_by.iter().find(|(p, _)| *p == pid).map(|(_, v)| *v).unwrap_or(0)
    }
}

/// `game.Rune`
#[derive(Clone, Debug)]
pub struct Rune {
    pub uid: u32,
    pub domain: String,
    pub owner: usize,
    pub exhausted: bool,
}

/// `game.Player`
#[derive(Clone, Debug)]
pub struct Player {
    pub pid: usize,
    pub legend_name: String,
    pub legend: O,
    pub deck: Vec<O>,
    pub champ: Vec<O>,
    pub hand: Vec<O>,
    pub trash: Vec<O>,
    pub banish: Vec<O>,
    pub rune_deck: Vec<R>,
    pub runes: Vec<R>,
    pub pool_e: i64,
    /// domaine -> power, 'A' = power de n'importe quel domaine
    pub pool_p: Counter,
    /// [énergie, only(g, pid, ctx) -> bool] : énergie réservée à certains coûts
    pub pool_r: Vec<(i64, Value)>,
    pub points: i64,
    pub xp: i64,
    pub bf_name: String,
    pub turns: i64,
    pub revealed_turn: i64,
    /// uids des cartes de son propre deck que le joueur a vues (436, 817)
    pub seen: BTreeSet<u32>,
    pub gold_spent: i64,
    /// attributs libres posés par les cartes (`setattr(pl, ...)`)
    pub attrs: Map,
}

/// `game.Item` : un item de chaîne (sort, capacité activée ou déclenchée, règles 328-340).
#[derive(Clone, Debug)]
pub struct Item {
    pub id: u32,
    /// 'spell' | 'ability' | 'trigger'
    pub kind: &'static str,
    pub ctrl: usize,
    pub name: String,
    pub fn_: Value,
    pub card: Option<O>,
    /// uid de l'objet source
    pub src: Option<u32>,
    pub data: Map,
    /// (uid, oid, prédicat) ; uid None : cible disparue (Repeat)
    pub targets: Vec<(Option<u32>, i64, Value)>,
    /// uids choisis (cibles)
    pub chosen: BTreeSet<u32>,
    pub uncounterable: bool,
}

/// `game.Showdown`
#[derive(Clone, Debug)]
pub struct Showdown {
    pub bf: usize,
    pub combat: bool,
    pub attacker: usize,
    pub defender: usize,
    pub focus: usize,
    pub passes: i64,
    /// open -> damage -> result -> control -> end
    pub stage: &'static str,
    pub att_seen: BTreeSet<u32>,
    pub def_seen: BTreeSet<u32>,
    /// unités qui ont été dans ce combat, dans l'ordre d'arrivée
    pub members: Vec<u32>,
    pub recalled: bool,
    pub assigned: Value,
    pub excess: [i64; 2],
    pub result: Option<usize>,
    /// attributs libres posés par les cartes (`sd._bf_defend_seen`, `sd._bf_excess`...)
    pub attrs: Map,
}

/// `game.Battlefield`
#[derive(Clone, Debug)]
pub struct Battlefield {
    pub idx: usize,
    pub name: String,
    pub owner: Option<usize>,
    pub ctrl: Option<usize>,
    pub contested: Option<usize>,
    pub facedowns: Vec<O>,
    pub scored: BTreeSet<usize>,
    pub sd_staged: bool,
    pub combat_staged: bool,
    pub replaced: Option<String>,
}

impl Battlefield {
    fn new(idx: usize, name: &str, owner: Option<usize>) -> Self {
        Battlefield {
            idx,
            name: name.to_string(),
            owner,
            ctrl: None,
            contested: None,
            facedowns: vec![],
            scored: BTreeSet::new(),
            sd_staged: false,
            combat_staged: false,
            replaced: None,
        }
    }
    pub fn facedown(&self) -> Option<O> {
        self.facedowns.first().copied()
    }
    /// setter `facedown` : remplace (ou retire avec None) la première carte cachée
    pub fn set_facedown(&mut self, c: Option<O>) {
        match c {
            None => {
                if !self.facedowns.is_empty() {
                    self.facedowns.remove(0);
                }
            }
            Some(c) => {
                if self.facedowns.is_empty() {
                    self.facedowns.push(c);
                } else {
                    self.facedowns[0] = c;
                }
            }
        }
    }
}

/// `game.Decision` : décision de haut niveau (action principale, priorité, focus). options None en mode paresseux.
#[derive(Clone, Debug)]
pub struct Decision {
    pub kind: &'static str,
    pub player: usize,
    pub options: Option<Vec<Value>>,
}

/// Un joueur (IA, humain, hasard). `choose` renvoie l'indice de l'option choisie (en Python : l'option elle-même).
pub trait Agent: Send + Sync {
    fn choose(&mut self, g: &mut Game, pid: usize, kind: &str, options: &[Value], ctx: &Map) -> Res<usize>;
    /// cartes de la main initiale à remettre (au plus 2)
    fn mulligan(&mut self, _g: &mut Game, _pid: usize) -> Res<Vec<O>> {
        Ok(vec![])
    }
    /// début de partie (`start(g)`)
    fn start(&mut self, _g: &mut Game) {}
    /// décision de haut niveau (`decide(g, d)`) : renvoie l'action choisie, une des `d.options`.
    fn decide(&mut self, _g: &mut Game, _d: &Decision) -> Res<Value> {
        panic!("cet agent ne prend pas de décision de haut niveau")
    }
    /// `every_choice` : joueur humain (voit tous les choix légaux)
    fn every_choice(&self) -> bool {
        false
    }
    fn clone_box(&self) -> Box<dyn Agent>;
}

impl Clone for Box<dyn Agent> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Les agents de la partie. Une copie de la partie (`Game.clone`, recherche de l'IA) n'a pas d'agents : l'appelant
/// pose les siens, comme en Python.
#[derive(Default)]
pub struct Agents(pub Vec<Box<dyn Agent>>);

impl Clone for Agents {
    fn clone(&self) -> Self {
        Agents(vec![])
    }
}

impl std::ops::Index<usize> for Agents {
    type Output = Box<dyn Agent>;
    fn index(&self, i: usize) -> &Self::Output {
        &self.0[i]
    }
}

impl std::ops::IndexMut<usize> for Agents {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.0[i]
    }
}

impl Agents {
    pub fn get(&self, i: usize) -> Option<&Box<dyn Agent>> {
        self.0.get(i)
    }
}

/// Une entrée de `g.effects` (dict Python) avec son identité (`eff in s.effects`).
#[derive(Clone, Debug)]
pub struct Effect {
    pub eid: u64,
    pub d: Map,
}

/// `s.hist` : historique du tour de chaque joueur (ce que les textes demandent « ce tour »).
#[derive(Clone, Debug, Default)]
pub struct Hist {
    pub spell_e: [Vec<i64>; 2],
    pub drawn: [i64; 2],
    pub discarded: [i64; 2],
    pub xp: [i64; 2],
    pub hold_pts: [i64; 2],
    pub power_spent: [i64; 2],
    pub died: Vec<Map>,
    pub conquered: Vec<(u32, i64)>,
    pub conq_bf: [Vec<usize>; 2],
    pub chose_enemy: [bool; 2],
    pub gear_abs: [i64; 2],
    pub gear_played: [i64; 2],
    pub tokens: [i64; 2],
    pub to_bf: [i64; 2],
    pub moved: Vec<(u32, i64)>,
    /// Hungry Wolf : (uid, oid) des loups déjà utilisés ce tour (`hist["wolf_used"]`)
    pub wolf_used: Vec<(u32, i64)>,
}

/// `game.Game`
#[derive(Clone)]
pub struct Game {
    pub counters: Arc<Counters>,
    pub objs: Vec<Obj>,
    pub runes_arena: Vec<Rune>,
    pub items: Vec<Item>,
    pub next_eid: u64,
    pub rng: PyRandom,
    pub agents: Agents,
    pub logging: bool,
    pub lines: Vec<String>,
    pub victory: i64,
    pub p: Vec<Player>,
    pub bfs: Vec<Battlefield>,
    /// objets sur le plateau (unités et équipements)
    pub board: Vec<O>,
    pub chain: Vec<ItemId>,
    pub shown_hand: Option<Map>,
    pub priority: Option<usize>,
    pub passes: i64,
    /// 'play' | 'trigger' : ce qui a ouvert la chaîne en cours
    pub chain_origin: Option<&'static str>,
    pub sd: Option<Showdown>,
    /// capacités déclenchées en attente d'être mises sur la chaîne (règle 383.3)
    pub trigq: Vec<ItemId>,
    pub need_cleanup: bool,
    /// gagnant ; -1 pour une partie nulle (tour 80 dépassé)
    pub winner: Option<i64>,
    pub turn_no: i64,
    pub first: usize,
    pub tp: usize,
    pub stage: &'static str,
    pub extra_turns: Vec<Value>,
    pub effects: Vec<Effect>,
    pub played: [Vec<String>; 2],
    pub finalized: [Vec<String>; 2],
    pub spells_played: [i64; 2],
    pub stats: Counter,
    pub history: Vec<Value>,
    pub dep_guard: BTreeSet<u32>,
    pub resolving: Option<ItemId>,
    pub dying: Vec<O>,
    pub phase: Option<&'static str>,
    pub zone_listen: bool,
    pub exh_pending: Vec<O>,
    pub hist: Hist,
    /// effets retirés de `effects` le temps d'appeler leur `fn` (`next_spell` : Python retire l'effet puis appelle
    /// `ef["fn"](g, ef, it)`) : `eff(eid)` les lit encore.
    pub effects_gone: Vec<Effect>,
    /// partie jouée par l'humain (train.Human) : `getattr(s, "every_choice", False)`
    pub every_choice: bool,
    /// point de reprise de la partie réelle (`checkpoint`, `_ckpt`, `_asklog`, ai.ask_sim) : jamais copié
    pub ck: Checkpoint,
}

/// Point de reprise (2026-10-28, ai.ask_sim) : quand `on` est vrai (posé par l'IA sur la partie réelle), `apply` garde
/// une copie de l'état d'avant le coup (`ckpt` : partie, `Obj._n`, `Item._n`, action) et `ask` note chaque réponse
/// donnée depuis (`asklog`). Jamais copié (`_game_copy`) : la copie d'une partie en a un vide.
#[derive(Default)]
pub struct Checkpoint {
    pub on: bool,
    pub ckpt: Option<Box<(Game, u32, u32, Value)>>,
    pub asklog: Vec<crate::ai::AskLog>,
}

impl Clone for Checkpoint {
    fn clone(&self) -> Self {
        Checkpoint::default()
    }
}

/// Paquet d'un joueur : `dict(legend, champion, main, runes, battlefield)`.
#[derive(Clone, Debug)]
pub struct DeckList {
    pub legend: String,
    pub champion: Option<String>,
    pub main: Vec<String>,
    pub runes: Vec<String>,
    pub battlefield: String,
}

fn str_is(v: &Value, s: &str) -> bool {
    v.opt_str() == Some(s)
}

impl Game {
    // ------------------------------------------------------------------ construction
    /// `Game.__init__`. seed None : graine du système (comme `random.Random(None)`) ; le moteur passe toujours une
    /// graine.
    pub fn new(
        decks: &[DeckList; 2],
        agents: Vec<Box<dyn Agent>>,
        first: Option<usize>,
        seed: i128,
        log: bool,
        victory: i64,
        counters: Arc<Counters>,
    ) -> Game {
        let rng = PyRandom::new(seed);
        let mut g = Game {
            counters,
            objs: vec![],
            runes_arena: vec![],
            items: vec![],
            next_eid: 0,
            rng,
            agents: Agents(agents),
            logging: log,
            lines: vec![],
            victory,
            p: vec![],
            bfs: vec![],
            board: vec![],
            chain: vec![],
            shown_hand: None,
            priority: None,
            passes: 0,
            chain_origin: None,
            sd: None,
            trigq: vec![],
            need_cleanup: false,
            winner: None,
            turn_no: 0,
            first: 0,
            tp: 0,
            stage: "setup",
            extra_turns: vec![],
            effects: vec![],
            played: [vec![], vec![]],
            finalized: [vec![], vec![]],
            spells_played: [0, 0],
            stats: Counter::default(),
            history: vec![],
            dep_guard: BTreeSet::new(),
            resolving: None,
            dying: vec![],
            phase: None,
            zone_listen: true,
            exh_pending: vec![],
            hist: Hist::default(),
            effects_gone: vec![],
            every_choice: false,
            ck: Checkpoint::default(),
        };
        for (i, d) in decks.iter().enumerate() {
            let pl = g.new_player(i, d);
            g.p.push(pl);
        }
        for i in 0..2 {
            let name = g.p[i].bf_name.clone();
            g.bfs.push(Battlefield::new(i, &name, Some(i)));
        }
        if g.bfs.iter().any(|b| b.name == "Aspirant's Climb") {
            g.victory += 1;
        }
        g.first = match first {
            Some(f) => f,
            None => g.rng.randrange(2) as usize,
        };
        g.tp = g.first;
        g.reset_hist();
        g
    }

    /// Partie avec les valeurs par défaut (`Game(decks, agents, seed=seed)`) et des compteurs neufs.
    pub fn simple(decks: &[DeckList; 2], agents: Vec<Box<dyn Agent>>, seed: i128) -> Game {
        Game::new(decks, agents, None, seed, false, VICTORY as i64, Arc::new(Counters::default()))
    }

    /// `Obj(name, owner, token)` : nouvel objet dans l'arène.
    pub fn new_obj(&mut self, name: &str, owner: usize, token: bool) -> O {
        let uid = self.counters.next_obj();
        self.objs.push(Obj::new(uid, name, owner, token));
        O(self.objs.len() as u32 - 1)
    }

    /// `Obj.__new__(Obj)` sans uid (`cards.token_locations` : un jeton témoin qui ne prend pas de numéro au compteur).
    /// À retirer avec `drop_probe` dès qu'il a servi.
    pub fn new_probe(&mut self, name: &str, owner: usize) -> O {
        self.objs.push(Obj::new(0, name, owner, true));
        O(self.objs.len() as u32 - 1)
    }

    /// Retire le témoin de `new_probe` (le dernier objet de l'arène).
    pub fn drop_probe(&mut self, o: O) {
        if o.0 as usize + 1 == self.objs.len() {
            self.objs.pop();
        }
    }

    fn new_rune(&mut self, domain: &str, owner: usize) -> R {
        let uid = self.counters.next_obj();
        self.runes_arena.push(Rune { uid, domain: domain.to_string(), owner, exhausted: false });
        R(self.runes_arena.len() as u32 - 1)
    }

    /// `Player.__init__` (les objets sont créés dans le même ordre : légende, deck, champion, runes).
    fn new_player(&mut self, pid: usize, d: &DeckList) -> Player {
        let legend = self.new_obj(&d.legend, pid, false);
        self.o_mut(legend).zone = Some(Zone::Legend);
        let mut deck = vec![];
        for n in &d.main {
            let o = self.new_obj(n, pid, false);
            deck.push(o);
        }
        for &o in &deck {
            self.o_mut(o).zone = Some(Zone::Deck);
        }
        let mut champ = vec![];
        if let Some(c) = &d.champion {
            if !c.is_empty() {
                let o = self.new_obj(c, pid, false);
                self.o_mut(o).zone = Some(Zone::Champ);
                champ.push(o);
            }
        }
        let mut rune_deck = vec![];
        for dom in &d.runes {
            rune_deck.push(self.new_rune(dom, pid));
        }
        Player {
            pid,
            legend_name: d.legend.clone(),
            legend,
            deck,
            champ,
            hand: vec![],
            trash: vec![],
            banish: vec![],
            rune_deck,
            runes: vec![],
            pool_e: 0,
            pool_p: Counter::default(),
            pool_r: vec![],
            points: 0,
            xp: 0,
            bf_name: d.battlefield.clone(),
            turns: 0,
            revealed_turn: -1,
            seen: BTreeSet::new(),
            gold_spent: 0,
            attrs: Map::new(),
        }
    }

    /// `Item(kind, ctrl, name, fn, card, src, data)`
    pub fn new_item(
        &mut self,
        kind: &'static str,
        ctrl: usize,
        name: &str,
        fn_: Value,
        card: Option<O>,
        src: Option<u32>,
        data: Map,
    ) -> ItemId {
        let id = self.counters.next_item();
        self.items.push(Item {
            id,
            kind,
            ctrl,
            name: name.to_string(),
            fn_,
            card,
            src,
            data,
            targets: vec![],
            chosen: BTreeSet::new(),
            uncounterable: false,
        });
        ItemId(self.items.len() as u32 - 1)
    }

    pub fn new_effect(&mut self, d: Map) -> Effect {
        self.next_eid += 1;
        Effect { eid: self.next_eid, d }
    }

    /// `reset_hist` : historique du tour de chaque joueur, remis à zéro à chaque tour.
    pub fn reset_hist(&mut self) {
        self.hist = Hist::default();
    }

    // ------------------------------------------------------------------ accès
    pub fn o(&self, o: O) -> &Obj {
        &self.objs[o.0 as usize]
    }
    pub fn o_mut(&mut self, o: O) -> &mut Obj {
        &mut self.objs[o.0 as usize]
    }
    pub fn r(&self, r: R) -> &Rune {
        &self.runes_arena[r.0 as usize]
    }
    pub fn r_mut(&mut self, r: R) -> &mut Rune {
        &mut self.runes_arena[r.0 as usize]
    }
    pub fn it(&self, i: ItemId) -> &Item {
        &self.items[i.0 as usize]
    }
    pub fn it_mut(&mut self, i: ItemId) -> &mut Item {
        &mut self.items[i.0 as usize]
    }

    /// `Obj.exhausted` (lecture)
    pub fn exhausted(&self, o: O) -> bool {
        self.o(o).exh
    }

    /// setter `Obj.exhausted` : épuiser un permanent sur le plateau (règle 415) est noté pour l'évènement
    /// 'exhausted' (« When I become exhausted », Radiance), émis par `_advance`.
    pub fn set_exhausted(&mut self, o: O, v: bool) {
        let ob = self.o(o);
        if v && !ob.exh && ob.zone == Some(Zone::Board) && ob.on_game {
            self.exh_pending.push(o);
        }
        self.o_mut(o).exh = v;
    }

    /// `Obj.reset`
    pub fn reset_obj(&mut self, o: O) {
        self.o_mut(o).reset_fields();
        self.set_exhausted(o, false);
    }

    /// `Obj.cname` : nom copié (équipement attaché qui copie, Reflection), sinon nom imprimé.
    pub fn cname(&self, o: O) -> &str {
        let ob = self.o(o);
        if let Some(cs) = ob.copy_src {
            let c = self.o(cs);
            if c.copy_name.as_deref().is_some_and(|n| !n.is_empty())
                && c.attached_to == Some(ob.uid)
                && c.zone == Some(Zone::Board)
            {
                return c.copy_name.as_deref().unwrap();
            }
        }
        match &ob.copy {
            Some(c) if !c.is_empty() => c,
            _ => &ob.name,
        }
    }

    /// `Obj.spec`
    pub fn spec(&self, o: O) -> &'static Spec {
        let n = self.cname(o);
        if self.o(o).ugear {
            return &ugear_specs()[n];
        }
        spec(n)
    }

    pub fn type_of(&self, o: O) -> &'static str {
        &self.spec(o).type_
    }

    /// `repr(obj)` : « Nom#uid »
    pub fn repr(&self, o: O) -> String {
        format!("{}#{}", self.cname(o), self.o(o).uid)
    }

    /// `repr(rune)` : initiale du domaine, « x » si épuisée
    pub fn repr_rune(&self, r: R) -> String {
        let ru = self.r(r);
        format!("{}{}", &ru.domain[..1], if ru.exhausted { "x" } else { "" })
    }

    // ------------------------------------------------------------------ utilitaires
    pub fn log(&mut self, msg: impl FnOnce() -> String) {
        if self.logging {
            let m = msg();
            self.lines.push(format!("[T{} P{}] {}", self.turn_no, self.tp, m));
        }
    }

    pub fn opp(&self, pid: usize) -> usize {
        1 - pid
    }

    /// `obj(uid)` : l'objet de cet uid **sur le plateau**, sinon None.
    pub fn obj(&self, uid: u32) -> Option<O> {
        self.board.iter().copied().find(|&o| self.o(o).uid == uid)
    }

    pub fn on_board(&self, o: O) -> bool {
        self.board.contains(&o)
    }

    /// `dup_no` : numéro d'affichage parmi les homonymes en jeu (0 s'il est seul).
    pub fn dup_no(&self, o: Option<O>) -> usize {
        let Some(o) = o else { return 0 };
        if !self.on_board(o) {
            return 0;
        }
        let n = self.cname(o);
        let mut same: Vec<u32> =
            self.board.iter().filter(|&&x| self.cname(x) == n).map(|&x| self.o(x).uid).collect();
        same.sort();
        if same.len() > 1 {
            same.iter().position(|&u| u == self.o(o).uid).unwrap() + 1
        } else {
            0
        }
    }

    /// `label` : « Shipyard Skulker #2 » quand il a des homonymes en jeu.
    pub fn label(&self, o: O) -> String {
        let n = self.dup_no(Some(o));
        if n > 0 {
            format!("{} #{}", self.cname(o), n)
        } else {
            self.cname(o).to_string()
        }
    }

    fn is_unit_like(&self, o: O) -> bool {
        self.o(o).ugear || spec(self.cname(o)).type_ == "Unit"
    }

    /// `units(pid, loc)` : loc None = partout (`"any"`).
    pub fn units(&self, pid: Option<usize>, loc: Option<Loc>) -> Vec<O> {
        self.board
            .iter()
            .copied()
            .filter(|&o| {
                let ob = self.o(o);
                pid.is_none_or(|p| ob.ctrl == p) && loc.is_none_or(|l| ob.loc == Some(l)) && self.is_unit_like(o)
            })
            .collect()
    }

    pub fn all_units(&self) -> Vec<O> {
        self.units(None, None)
    }

    /// `gear(pid)`
    pub fn gear(&self, pid: Option<usize>) -> Vec<O> {
        self.board
            .iter()
            .copied()
            .filter(|&o| {
                let ob = self.o(o);
                pid.is_none_or(|p| ob.ctrl == p) && (ob.ugear || spec(self.cname(o)).type_ == "Gear")
            })
            .collect()
    }

    /// `is_gear` : un équipement, unit gear comprise.
    pub fn is_gear(&self, o: O) -> bool {
        self.spec(o).type_ == "Gear" || self.o(o).ugear
    }

    pub fn at_bf(&self, o: O) -> bool {
        is_bf(self.o(o).loc)
    }

    /// `bf_locs` : indices des battlefields (deux, ou trois avec le Baron Pit, règle 187.9).
    pub fn bf_locs(&self) -> Vec<Loc> {
        (0..self.bfs.len()).map(Loc::Bf).collect()
    }

    /// `locs` : la base et chaque battlefield.
    pub fn locs(&self) -> Vec<Loc> {
        let mut v = vec![Loc::Base];
        v.extend(self.bf_locs());
        v
    }

    /// `add_battlefield` : ajoute un battlefield jeton (Baron Pit, règle 187.9), sans contrôleur ni propriétaire.
    pub fn add_battlefield(&mut self, name: &str) -> Res<usize> {
        let idx = self.bfs.len();
        self.bfs.push(Battlefield::new(idx, name, None));
        self.need_cleanup = true;
        self.log(|| format!("  the {name} battlefield token is added to the board"));
        self.emit("bf_added", crate::map! { bf = idx, name = name })?;
        Ok(idx)
    }

    /// `impl(obj)`
    pub fn impl_o(&self, o: O) -> Option<&'static Impl> {
        impl_of(self.cname(o))
    }
    /// `impl(name)`
    pub fn impl_n(&self, name: &str) -> Option<&'static Impl> {
        impl_of(name)
    }

    pub fn has_kw(&mut self, o: O, kw: &str) -> Res<bool> {
        Ok(self.kw_value(o, kw)? > 0)
    }

    /// `kw_value` : valeur d'un mot-clé (les valeurs d'Assault/Shield/Deflect/Hunt/Vision s'additionnent, règles
    /// 807.2, 809.2, 814.2, 823.2) : imprimés, dépendants (727), accordés, texte des équipements attachés (434.1.c)
    /// et auras des autres objets.
    pub fn kw_value(&mut self, o: O, kw: &str) -> Res<i64> {
        let mut v = 0;
        if let Some(im) = self.impl_o(o) {
            v += im.kw(kw);
            if !im.kw_if.is_empty() || !im.levels.is_empty() {
                v += self.dependent_kw(o, im, kw)?;
            }
        }
        for (k, val, _) in &self.o(o).grants {
            if k == kw {
                v += val;
            }
        }
        for gu in self.o(o).attached.clone() {
            let go = self.obj(gu);
            let ig = go.and_then(|g| self.impl_o(g));
            if let Some(ig) = ig {
                if let Some(ek) = &ig.equip_kw {
                    v += ek.get(kw).map(|x| x.int()).unwrap_or(0);
                }
            }
            if let Some(go) = go {
                if self.o(go).copy_name.as_deref().is_some_and(|n| !n.is_empty()) && self.cname(go) == "Svellsongur" {
                    let cn = self.o(go).copy_name.clone().unwrap();
                    if let Some(ic) = self.impl_n(&cn) {
                        v += ic.kw(kw);
                        if !ic.kw_if.is_empty() || !ic.levels.is_empty() {
                            v += self.dependent_kw(o, ic, kw)?;
                        }
                    }
                }
            }
        }
        if !registry().aura.is_empty() {
            for (src, ia) in self.aura_sources() {
                if !ia.aura_kw.is_none() {
                    let d = ia.aura_kw.call(self, &[src.clone(), Value::Obj(o)])?;
                    if d.truthy() {
                        v += d.map().get(kw).map(|x| x.int()).unwrap_or(0);
                    }
                }
            }
        }
        if kw == "Ganking" {
            if let Some(Loc::Bf(i)) = self.o(o).loc {
                if self.bfs[i].name == "Windswept Hillock" {
                    v += 1;
                }
            }
        }
        Ok(v)
    }

    /// `_dependent_kw` : mots-clés dépendants (727). Pendant l'évaluation d'une condition de cet objet, ses propres
    /// mots-clés dépendants comptent comme absents.
    fn dependent_kw(&mut self, o: O, im: &'static Impl, kw: &str) -> Res<i64> {
        let uid = self.o(o).uid;
        if self.dep_guard.contains(&uid) {
            return Ok(0);
        }
        let mut v = 0;
        if !im.levels.is_empty() {
            let xp = self.p[self.o(o).ctrl].xp;
            for (n, d) in &im.levels {
                if xp >= *n {
                    let k = d.g("kw");
                    if k.truthy() {
                        v += k.map().get(kw).map(|x| x.int()).unwrap_or(0);
                    }
                }
            }
        }
        if !im.kw_if.is_empty() {
            self.dep_guard.insert(uid);
            let r = (|| {
                for (cond, d) in &im.kw_if {
                    if d.contains(kw) && cond.call(self, &[Value::Obj(o)])?.truthy() {
                        v += d.g(kw).int();
                    }
                }
                Ok(())
            })();
            self.dep_guard.remove(&uid);
            r?;
        }
        Ok(v)
    }

    // ------------------------------------------------------------------ capacités statiques des autres objets
    /// `providers(field)` : (source, impl) de chaque objet dont l'Impl définit `field` : permanents sur le plateau dont
    /// le texte imprimé est actif (pas un équipement attaché, 434.1.e), légendes et battlefields, dans un ordre fixe.
    /// La source est `Value::Obj` (permanent, légende) ou `Value::Int(idx)` (battlefield).
    pub fn providers(&self, field: &str) -> Vec<(Value, &'static Impl)> {
        let Some(names) = registry().hook_names(field) else { return vec![] };
        let mut out = vec![];
        for &x in &self.board {
            if self.o(x).attached_to.is_none() {
                if names.contains(self.cname(x)) {
                    out.push((Value::Obj(x), self.impl_o(x).unwrap()));
                }
                if !self.o(x).attached.is_empty() {
                    for n in self.copied_texts(x) {
                        if names.contains(&n) {
                            out.push((Value::Obj(x), self.impl_n(&n).unwrap()));
                        }
                    }
                }
            }
        }
        for pl in &self.p {
            if names.contains(&pl.legend_name) {
                out.push((Value::Obj(pl.legend), self.impl_n(&pl.legend_name).unwrap()));
            }
        }
        for b in &self.bfs {
            if names.contains(&b.name) {
                out.push((Value::Int(b.idx as i64), self.impl_n(&b.name).unwrap()));
            }
        }
        out
    }

    /// `fx_providers(field, unit)` : (équipement, unité, impl) de chaque équipement attaché dont le texte d'effet
    /// définit `field` (434.1.c).
    pub fn fx_providers(&self, field: &str, unit: Option<O>) -> Vec<(O, O, &'static Impl)> {
        let Some(names) = registry().hook_names(field) else { return vec![] };
        let mut out = vec![];
        let us: Vec<O> = match unit {
            Some(u) => vec![u],
            None => self.board.clone(),
        };
        for u in us {
            for &gu in &self.o(u).attached {
                if let Some(go) = self.obj(gu) {
                    if names.contains(self.cname(go)) {
                        out.push((go, u, self.impl_o(go).unwrap()));
                    }
                }
            }
        }
        out
    }

    /// `copied_texts` : noms dont le texte est ajouté à o par un équipement attaché qui le copie (Svellsongur).
    pub fn copied_texts(&self, o: O) -> Vec<String> {
        let mut out = vec![];
        for &gu in &self.o(o).attached {
            if let Some(go) = self.obj(gu) {
                if let Some(cn) = &self.o(go).copy_name {
                    if !cn.is_empty() && self.cname(go) == "Svellsongur" {
                        out.push(cn.clone());
                    }
                }
            }
        }
        out
    }

    /// `effects_of(kind, pid)` : indices des effets continus de ce type (et de ce joueur). Une entrée avec
    /// src=(uid, oid) est une capacité statique de ce permanent : seulement tant qu'il est sur le plateau, pour son
    /// contrôleur actuel.
    pub fn effects_of(&self, kind: &str, pid: Option<usize>) -> Vec<usize> {
        let mut out = vec![];
        for (i, e) in self.effects.iter().enumerate() {
            if !str_is(&e.d.g("kind"), kind) {
                continue;
            }
            let mut p = e.d.g("pid").opt_int().map(|x| x as usize);
            let src = e.d.g("src");
            if !src.is_none() {
                let s = src.list();
                let o = self.obj(s[0].int() as u32);
                match o {
                    Some(o) if self.o(o).oid == s[1].int() && self.on_board(o) => p = Some(self.o(o).ctrl),
                    _ => continue,
                }
            }
            if pid.is_none() || p == pid {
                out.push(i);
            }
        }
        out
    }

    /// `tags(o)` : tags imprimés, choisi à la pose, accordés (xtags), des équipements attachés et des auras.
    pub fn tags(&mut self, o: O) -> Res<BTreeSet<String>> {
        let t = &self.spec(o).tags;
        let ob = self.o(o);
        if !(ob.chosen_tag.is_some() || !ob.xtags.is_empty() || !ob.attached.is_empty() || registry().hook_names("aura_tags").is_some()) {
            return Ok(t.clone());
        }
        let mut t = t.clone();
        if let Some(c) = &ob.chosen_tag {
            t.insert(c.clone());
        }
        t.extend(ob.xtags.iter().map(|x| x.0.clone()));
        for gu in ob.attached.clone() {
            if let Some(go) = self.obj(gu) {
                if let Some(ig) = self.impl_o(go) {
                    if let Some(et) = &ig.equip_tags {
                        t.extend(et.iter().cloned());
                    }
                }
            }
        }
        for (src, im) in self.providers("aura_tags") {
            let r = im.hook("aura_tags").call(self, &[src, Value::Obj(o)])?;
            for x in r.list() {
                t.insert(x.str().to_string());
            }
        }
        Ok(t)
    }

    /// `aura_sources` : objets dont l'Impl donne des mots-clés ou de la Might aux autres (aura_kw / aura_might).
    pub fn aura_sources(&self) -> Vec<(Value, &'static Impl)> {
        let names = &registry().aura;
        let mut out = vec![];
        if names.is_empty() {
            return out;
        }
        for &x in &self.board {
            if names.contains(self.cname(x)) {
                out.push((Value::Obj(x), self.impl_o(x).unwrap()));
            }
        }
        for pl in &self.p {
            if names.contains(&pl.legend_name) {
                out.push((Value::Obj(pl.legend), self.impl_n(&pl.legend_name).unwrap()));
            }
        }
        for b in &self.bfs {
            if names.contains(&b.name) {
                out.push((Value::Int(b.idx as i64), self.impl_n(&b.name).unwrap()));
            }
        }
        out
    }

    /// `level_value` : valeur d'un champ de Level pour le plus haut niveau atteint (824).
    pub fn level_value(&self, o: O, im: &Impl, field: &str) -> Option<Value> {
        let mut best: Option<(i64, Value)> = None;
        let xp = self.p[self.o(o).ctrl].xp;
        for (n, d) in &im.levels {
            if xp >= *n && d.contains(field) && best.as_ref().is_none_or(|b| *n >= b.0) {
                best = Some((*n, d.g(field)));
            }
        }
        best.map(|b| b.1)
    }

    // ------------------------------------------------------------------ XP, Level, Legion, Mighty (706-733)
    /// `level(pid, n)` : [Level N] actif à N XP ou plus (824).
    pub fn level(&self, pid: usize, n: i64) -> bool {
        self.p[pid].xp >= n
    }

    pub fn gain_xp(&mut self, pid: usize, n: i64) -> Res<()> {
        if n <= 0 {
            return Ok(());
        }
        self.p[pid].xp += n;
        self.hist.xp[pid] += n;
        let xp = self.p[pid].xp;
        self.log(|| format!("  P{pid} gains {n} XP -> {xp}"));
        self.emit("xp_gain", crate::map! { pid = pid, n = n })
    }

    /// `spend_xp` (730.2) : False (rien dépensé) s'il a moins de n.
    pub fn spend_xp(&mut self, pid: usize, n: i64) -> Res<bool> {
        if self.p[pid].xp < n {
            return Ok(false);
        }
        self.p[pid].xp -= n;
        self.emit("xp_spend", crate::map! { pid = pid, n = n })?;
        Ok(true)
    }

    /// `legion(pid, card)` : [Legion] une autre carte finalisée ce tour (812.1.c).
    pub fn legion(&self, pid: usize, card: Option<O>) -> bool {
        let mut n = self.finalized[pid].len() as i64;
        if let Some(c) = card {
            let z = self.o(c).zone;
            if matches!(z, Some(Zone::Chain) | Some(Zone::Board)) && self.finalized[pid].iter().any(|x| x == self.cname(c)) {
                n -= 1;
            }
        }
        n >= 1
    }

    pub fn in_combat(&self, o: O) -> bool {
        match &self.sd {
            Some(sd) => sd.combat && self.o(o).loc == Some(Loc::Bf(sd.bf)) && self.o(o).desig.is_some(),
            None => false,
        }
    }

    /// `alone(o, loc)`
    pub fn alone(&self, o: O, loc: Option<Loc>) -> bool {
        let loc = loc.or(self.o(o).loc);
        let ctrl = self.o(o).ctrl;
        !self.all_units().into_iter().any(|u| u != o && self.o(u).ctrl == ctrl && self.o(u).loc == loc)
    }

    /// `might(o)` : Might actuelle (477.3 : augmentations d'abord, puis diminutions).
    pub fn might(&mut self, o: O) -> Res<i64> {
        let mut base = self.spec(o).might.unwrap_or(0) as i64;
        let (uid, oid) = (self.o(o).uid, self.o(o).oid);
        if !self.effects.is_empty() {
            for e in &self.effects {
                // « Its base Might becomes N this turn » (Dragon Form)
                if str_is(&e.d.g("kind"), "base_might") && e.d.g("uid").int() as u32 == uid && e.d.g("oid").int() == oid {
                    base = e.d.g("value").int();
                }
            }
        }
        let ob = self.o(o);
        let mut inc = ob.buff;
        if ob.buff != 0 && !self.effects.is_empty() {
            // Stand United : « Buffs give an additional +1 might to friendly units this turn. »
            let ctrl = ob.ctrl as i64;
            inc += ob.buff
                * self
                    .effects
                    .iter()
                    .filter(|e| str_is(&e.d.g("kind"), "buff_bonus") && e.d.g("pid").int() == ctrl)
                    .count() as i64;
        }
        let mut dec = 0;
        for (amt, _) in &ob.mods {
            if *amt > 0 {
                inc += amt;
            } else {
                dec += amt;
            }
        }
        for &gu in &ob.attached {
            if let Some(go) = self.obj(gu) {
                inc += self.equip_bonus_of(self.cname(go));
            }
        }
        let desig = ob.desig;
        let has_att = !ob.attached.is_empty();
        if desig == Some(Desig::Att) {
            inc += self.kw_value(o, "Assault")?;
        }
        if desig == Some(Desig::Def) {
            inc += self.kw_value(o, "Shield")?;
        }
        let mut ims = vec![self.impl_o(o)];
        if has_att {
            ims.extend(self.copied_texts(o).iter().map(|n| self.impl_n(n)));
        }
        for im in ims.into_iter().flatten() {
            if !im.might_mod.is_none() {
                let m = im.might_mod.call(self, &[Value::Obj(o)])?.int();
                if m > 0 {
                    inc += m;
                } else {
                    dec += m;
                }
            }
            if (!im.levels.is_empty() || !im.might_if.is_empty()) && !self.dep_guard.contains(&uid) {
                let mut m = 0;
                if !im.levels.is_empty() {
                    m += self.level_value(o, im, "might").map(|v| v.int()).unwrap_or(0);
                }
                if !im.might_if.is_empty() {
                    self.dep_guard.insert(uid);
                    let r = (|| {
                        let mut s = 0;
                        for (cond, n) in &im.might_if {
                            if cond.call(self, &[Value::Obj(o)])?.truthy() {
                                s += n;
                            }
                        }
                        Ok(s)
                    })();
                    self.dep_guard.remove(&uid);
                    m += r?;
                }
                if m > 0 {
                    inc += m;
                } else {
                    dec += m;
                }
            }
        }
        if !registry().aura.is_empty() {
            for (src, ia) in self.aura_sources() {
                if !ia.aura_might.is_none() {
                    let m = ia.aura_might.call(self, &[src, Value::Obj(o)])?.int();
                    if m > 0 {
                        inc += m;
                    } else if m < 0 {
                        dec += m;
                    }
                }
            }
        }
        // Forbidding Waste : « While a unit here is defending alone, it has -2 might. »
        if let Some(Loc::Bf(i)) = self.o(o).loc {
            if self.bfs[i].name == "Forbidding Waste" && self.o(o).desig == Some(Desig::Def) && self.alone(o, None) {
                dec -= 2;
            }
        }
        Ok(base + inc + dec)
    }

    /// `EQUIP_BONUS.get(name, 0)`, bonus déclarés par les cartes (`bonus=`) compris.
    pub fn equip_bonus_of(&self, name: &str) -> i64 {
        if let Some(b) = self.impl_n(name).and_then(|im| im.bonus) {
            return b;
        }
        equip_bonus(name) as i64
    }

    /// `mighty(o)` : Might 5 ou plus (706-711) ; Might imprimée hors du plateau.
    pub fn mighty(&mut self, o: O) -> Res<bool> {
        if !self.on_board(o) {
            return Ok(self.spec(o).might.unwrap_or(0) >= 5);
        }
        Ok(self.might(o)? >= 5)
    }

    /// `targetable(o, by_pid, kind)` : impossibilité d'être ciblé (757).
    pub fn targetable(&mut self, o: O, by_pid: Option<usize>) -> Res<bool> {
        if let Some(im) = self.impl_o(o) {
            if !im.untargetable.is_none()
                && im.untargetable.call(self, &[Value::Obj(o), by_pid.into()])?.truthy()
            {
                return Ok(false);
            }
        }
        if let Some(bp) = by_pid {
            if bp != self.o(o).ctrl {
                if self.o(o).grants.iter().any(|(k, _, _)| k == "Untargetable") {
                    return Ok(false);
                }
                for (src, ia) in self.providers("aura_untargetable") {
                    if ia.hook("aura_untargetable").call(self, &[src, Value::Obj(o), bp.into()])?.truthy() {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }

    /// `legal(item, i)` : la i-ème cible d'un item si elle est encore légale (359.3.e), sinon None.
    pub fn legal(&mut self, item: ItemId, i: usize) -> Res<Option<O>> {
        let it = self.it(item);
        if i >= it.targets.len() {
            return Ok(None);
        }
        let (uid, oid, pred) = it.targets[i].clone();
        let ctrl = it.ctrl;
        let Some(o) = uid.and_then(|u| self.obj(u)) else { return Ok(None) };
        if self.o(o).oid != oid {
            return Ok(None);
        }
        if !self.targetable(o, Some(ctrl))? {
            return Ok(None);
        }
        if !pred.is_none() && !pred.call(self, &[Value::Item(item), Value::Obj(o)])?.truthy() {
            return Ok(None);
        }
        Ok(Some(o))
    }

    /// `add_target(item, o, pred)`
    pub fn add_target(&mut self, item: ItemId, o: O, pred: Value) {
        let (uid, oid) = (self.o(o).uid, self.o(o).oid);
        let it = self.it_mut(item);
        it.targets.push((Some(uid), oid, pred));
        it.chosen.insert(uid);
    }

    /// `mod(o, amount, duration, minimum)` : effet arithmétique figé (477.3.b). Un -might donné par un sort ou une
    /// capacité qui a choisi o peut être remplacé (Gangplank, Naval) ou augmenté (minus_extra : Mel).
    pub fn mod_(&mut self, o: O, amount: i64, duration: Value, minimum: Option<i64>) -> Res<()> {
        let mut amount = amount;
        let uid = self.o(o).uid;
        if amount < 0 {
            if let Some(res) = self.resolving {
                if self.it(res).chosen.contains(&uid) {
                    if !self.replace_choice_effect(o, "minus", duration.clone())? {
                        return Ok(());
                    }
                    for (src, im) in self.providers("minus_extra") {
                        let r = self.resolving.unwrap();
                        amount -= im.hook("minus_extra").call(self, &[src, Value::Item(r), Value::Obj(o)])?.opt_int().unwrap_or(0);
                    }
                }
            }
        }
        if let Some(mn) = minimum {
            if amount < 0 {
                let cur = self.might(o)?;
                amount = -((-amount).min(0.max(cur - mn)));
            }
        }
        if amount != 0 {
            self.o_mut(o).mods.push((amount, duration));
            if amount < 0 {
                if let Some(r) = self.resolving {
                    // « When you reduce the Might of an enemy unit by 1 or more » (Ahri, Confident)
                    let by = self.it(r).ctrl;
                    self.emit("might_reduced", crate::map! { obj = o, amount = -amount, by = by })?;
                }
            }
        }
        Ok(())
    }

    /// `grant(o, kw, value, duration)`
    pub fn grant(&mut self, o: O, kw: &str, value: i64, duration: Value) {
        self.o_mut(o).grants.push((kw.to_string(), value, duration));
    }

    // ------------------------------------------------------------------ zones
    /// `new_token(name, pid)`
    pub fn new_token(&mut self, name: &str, pid: usize) -> O {
        self.new_obj(name, pid, true)
    }

    /// `remove_from_zone(o)`
    pub fn remove_from_zone(&mut self, o: O) {
        let owner = self.o(o).owner;
        let pl = &mut self.p[owner];
        for z in [&mut pl.deck, &mut pl.hand, &mut pl.trash, &mut pl.banish, &mut pl.champ] {
            if let Some(i) = z.iter().position(|&x| x == o) {
                z.remove(i);
                return;
            }
        }
        if let Some(i) = self.board.iter().position(|&x| x == o) {
            self.board.remove(i);
            return;
        }
        for b in &mut self.bfs {
            if let Some(i) = b.facedowns.iter().position(|&x| x == o) {
                b.facedowns.remove(i);
                return;
            }
        }
    }

    /// `lookback(o)` : informations rétrospectives d'un permanent qui quitte le plateau (359.3.e.13, 808.1.d.3).
    pub fn lookback(&mut self, o: O) -> Res<Map> {
        let might = self.might(o)?;
        let tags: Vec<Value> = self.tags(o)?.into_iter().map(Value::from).collect();
        let ob = self.o(o).clone();
        let sp = self.spec(o);
        let cost = if !ob.token || ob.copy.is_some() {
            Value::Tuple(vec![sp.e.into(), sp.p.into()])
        } else {
            Value::Tuple(vec![0.into(), 0.into()])
        };
        let att_names: Vec<Value> =
            ob.attached.iter().filter_map(|&u| self.obj(u)).map(|x| Value::from(self.cname(x))).collect();
        let copied: Vec<Value> = if !ob.attached.is_empty() {
            self.copied_texts(o).into_iter().map(Value::from).collect()
        } else {
            vec![]
        };
        Ok(crate::map! {
            obj = o, name = self.cname(o), ctrl = ob.ctrl, loc = ob.loc, might = might, alone = self.alone(o, None),
            token = ob.token, spec = Value::from(sp.name.as_str()), empowered = ob.empowered,
            in_combat = self.in_combat(o),
            attached = Value::List(ob.attached.iter().map(|&u| Value::from(u)).collect()),
            buffed = ob.buff > 0, stunned = ob.stunned, exhausted = ob.exh, uid = ob.uid, oid = ob.oid,
            tags = Value::List(tags), cost = cost, att_names = Value::List(att_names), copied = Value::List(copied),
        })
    }

    /// `trash_dest(o, zone, frm)` : remplacement de zone (Endless Riches).
    pub fn trash_dest(&self, o: O, zone: &str, frm: Option<&str>) -> &'static str {
        if zone == "trash" && frm != Some("deck") && !self.effects.is_empty()
            && !self.effects_of("trash_to_banish", Some(self.o(o).owner)).is_empty()
        {
            return "banish";
        }
        Zone::parse(zone).name()
    }

    /// `to_zone(o, zone, bottom, by)` : vers une zone hors plateau (main, défausse, bannissement, deck). Les jetons
    /// cessent d'exister (186.1). Depuis le plateau : évènements 'leave' et, vers une main, 'returned' ; une carte
    /// bannie émet 'banish'. False si un remplacement l'a gardée où elle était.
    pub fn to_zone(&mut self, o: O, zone: &str, bottom: bool, by: Option<usize>) -> Res<bool> {
        let was_board = self.on_board(o);
        let frm = self.o(o).zone.map(|z| z.name());
        let zone = self.trash_dest(o, zone, frm);
        if was_board && zone == "hand" && self.resolving.is_some() && !self.replace_choice_effect(o, "hand", Value::None)? {
            return Ok(false);
        }
        let info = if was_board { Some(self.lookback(o)?) } else { None };
        self.remove_from_zone(o);
        if was_board {
            self.leave_board_cleanup(o);
        }
        self.reset_obj(o);
        self.o_mut(o).oid += 1;
        self.o_mut(o).zone = Some(Zone::parse(zone));
        if !self.o(o).token {
            let owner = self.o(o).owner;
            let pl = &mut self.p[owner];
            match zone {
                "hand" => pl.hand.push(o),
                "trash" => pl.trash.push(o),
                "banish" => pl.banish.push(o),
                "deck" => {
                    if bottom {
                        pl.deck.push(o)
                    } else {
                        pl.deck.insert(0, o)
                    }
                }
                _ => {}
            }
        }
        if let Some(info) = info {
            if zone == "hand" {
                let owner = self.o(o).owner;
                self.emit("returned", crate::map! { obj = o, loc = info.g("loc"), pid = owner, info = info.clone() })?;
            }
            self.emit("leave", crate::map! { obj = o, info = info, dest = zone })?;
        }
        if zone == "banish" && !self.o(o).token {
            let by = match by {
                Some(b) => b,
                None => match self.resolving {
                    Some(r) => self.it(r).ctrl,
                    None => self.o(o).owner,
                },
            };
            self.emit("banish", crate::map! { pid = by, card = o })?;
        }
        Ok(true)
    }

    /// `leave_board_cleanup` : l'équipement attaché se détache et reste au dernier emplacement (719.5, 435.4.b).
    pub fn leave_board_cleanup(&mut self, o: O) {
        for gu in self.o(o).attached.clone() {
            if let Some(go) = self.obj(gu) {
                self.o_mut(go).attached_to = None;
            }
        }
        self.o_mut(o).attached.clear();
        if let Some(at) = self.o(o).attached_to {
            if let Some(u) = self.obj(at) {
                let uid = self.o(o).uid;
                if let Some(i) = self.o(u).attached.iter().position(|&x| x == uid) {
                    self.o_mut(u).attached.remove(i);
                }
            }
            self.o_mut(o).attached_to = None;
        }
        self.need_cleanup = true;
    }

    /// `enter_board(o, pid, loc, ready)`
    pub fn enter_board(&mut self, o: O, pid: usize, loc: Loc, ready: bool) -> O {
        self.remove_from_zone(o);
        self.reset_obj(o);
        let turn = self.turn_no;
        let ob = self.o_mut(o);
        ob.oid += 1;
        ob.zone = Some(Zone::Board);
        ob.ctrl = pid;
        ob.loc = Some(loc);
        ob.exh = !ready; // entrer épuisé n'est pas l'action Exhaust (415)
        ob.on_game = true;
        ob.entered_turn = Some(turn);
        self.board.push(o);
        self.need_cleanup = true;
        o
    }

    /// `draw(pid, n)`
    pub fn draw(&mut self, pid: usize, n: i64) -> Res<()> {
        for _ in 0..n {
            if self.p[pid].deck.is_empty() {
                self.burn_out(pid)?;
                if self.winner.is_some() {
                    return Ok(());
                }
                if self.p[pid].deck.is_empty() {
                    continue;
                }
            }
            let c = self.p[pid].deck.remove(0);
            self.o_mut(c).zone = Some(Zone::Hand);
            self.p[pid].hand.push(c);
            self.stats.add("draw_seq", 1);
            self.o_mut(c).drawn_at = Some(self.stats.get("draw_seq"));
            if self.stage != "setup" {
                self.hist.drawn[pid] += 1;
                let n = self.hist.drawn[pid];
                self.emit("draw", crate::map! { pid = pid, card = c, n = n })?;
            }
        }
        Ok(())
    }

    /// `draw_card(pid, c)` : piocher une carte donnée de son deck.
    pub fn draw_card(&mut self, pid: usize, c: O) -> Res<bool> {
        let Some(i) = self.p[pid].deck.iter().position(|&x| x == c) else { return Ok(false) };
        self.p[pid].deck.remove(i);
        self.o_mut(c).zone = Some(Zone::Hand);
        self.p[pid].hand.push(c);
        self.stats.add("draw_seq", 1);
        self.o_mut(c).drawn_at = Some(self.stats.get("draw_seq"));
        self.hist.drawn[pid] += 1;
        let n = self.hist.drawn[pid];
        self.emit("draw", crate::map! { pid = pid, card = c, n = n })?;
        Ok(true)
    }

    /// `discard(pid, c)` (422) : de la main vers la défausse ; émet 'discard'.
    pub fn discard(&mut self, pid: usize, c: O) -> Res<bool> {
        if !self.p[pid].hand.contains(&c) {
            return Ok(false);
        }
        self.to_zone(c, "trash", false, None)?;
        let r = self.repr(c);
        self.log(|| format!("  P{pid} discards {r}"));
        self.hist.discarded[pid] += 1;
        if let Some(im) = self.impl_o(c) {
            if !im.on_discard.is_none() {
                im.on_discard.call(self, &[Value::Obj(c), pid.into()])?;
            }
        }
        self.emit("discard", crate::map! { pid = pid, card = c })?;
        Ok(true)
    }
}
