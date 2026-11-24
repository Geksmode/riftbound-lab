//! Match BO1 / BO3 entre le joueur (0) et l'IA (1), fidèle aux règles (`match.py`).
//!
//! - Core Rules 485 (1v1 Duel, BO1) : chaque joueur tire AU HASARD un de ses 3 battlefields (485.5).
//! - Core Rules 486 (1v1 Match, BO3) : chaque joueur CHOISIT un de ses 3 battlefields, en même temps que l'autre
//!   (486.5) ; après une manche gagnée, les battlefields joués sont retirés pour le reste du match ; après une nulle ils
//!   peuvent resservir (486.5.a). Le match se gagne en 2 manches (486.6).
//! - Règles de tournoi 407 : manche 1, un joueur tiré au sort choisit de jouer en premier ou en second (407.1-407.2) ;
//!   ensuite le perdant de la manche précédente choisit (407.4) ; après une nulle, le même joueur recommence (407.4).
//! - Règles de tournoi 403 et 601.1.c : sideboard interdit en manche 1 (403.5) et après une nulle (403.10) ; échanges
//!   1 pour 1 (403.4), le Chosen Champion peut changer (601.1.c.4) ; runes, légende et battlefields fixes (403.4.b).
//!
//! L'état est un objet JSON (la page le garde) ; ces fonctions sont pures. Choix de l'IA : jouer en premier ;
//! battlefield choisi par son plan (sinon au hasard) sans connaître celui du joueur (choix simultanés) ; l'IA ne
//! sideboarde pas.
use crate::jo;
use crate::json::Json;
use crate::lang::tr;
use crate::pyrandom::PyRandom;

pub const HUMAN: usize = 0;
pub const AI: usize = 1;

fn strs(j: &Json) -> Vec<String> {
    j.arr().iter().map(|x| x.str().unwrap_or("").to_string()).collect()
}

/// `new(mode, seed, bfs)` : bfs = [battlefields du joueur, battlefields de l'IA] (3 chacun).
pub fn new(mode: &str, seed: i64, bfs: [Vec<String>; 2]) -> Json {
    assert!(mode == "bo1" || mode == "bo3");
    let mut rng = PyRandom::new(seed as i128 * 7907 + 13);
    let roll = rng.randrange(2); // gagnant du tirage au sort de la manche 1 (407.2)
    let [b0, b1] = bfs;
    jo! {
        "mode" => mode, "seed" => seed, "bfs" => Json::Arr(vec![b0.into(), b1.into()]),
        "wins" => vec![0i64, 0], "used" => Json::Arr(vec![Json::Arr(vec![]), Json::Arr(vec![])]),
        "history" => Json::Arr(vec![]), "roll" => roll,
    }
}

pub fn game_no(st: &Json) -> i64 {
    st.g("history").arr().len() as i64 + 1
}

fn wins(st: &Json) -> [i64; 2] {
    let w = st.g("wins").arr();
    [w[0].int().unwrap(), w[1].int().unwrap()]
}

pub fn over(st: &Json) -> bool {
    if st.g("mode").str() == Some("bo1") {
        return !st.g("history").arr().is_empty();
    }
    let w = wins(st);
    w[0].max(w[1]) >= 2
}

/// Vainqueur du match : 0, 1, -1 (égalité) ; None tant qu'il n'est pas fini.
pub fn winner(st: &Json) -> Option<i64> {
    if !over(st) {
        return None;
    }
    let w = wins(st);
    if w[0] != w[1] {
        return Some(if w[0] > w[1] { 0 } else { 1 });
    }
    Some(-1)
}

fn last_winner(st: &Json) -> Option<i64> {
    st.g("history").arr().last().and_then(|h| h.g("winner").int())
}

/// Qui choisit de jouer en premier ou en second pour la manche à venir ; None = imposé (après une nulle).
pub fn chooser(st: &Json) -> Option<i64> {
    if st.g("history").arr().is_empty() {
        return st.g("roll").int();
    }
    let w = last_winner(st);
    match w {
        Some(0) | Some(1) => Some(1 - w.unwrap()),
        _ => None,
    }
}

/// Premier joueur imposé après une nulle (407.4) ; None sinon.
pub fn forced_first(st: &Json) -> Option<i64> {
    if !st.g("history").arr().is_empty() && chooser(st).is_none() {
        return st.g("history").arr().last().unwrap().g("first").int();
    }
    None
}

pub fn allowed_bfs(st: &Json, pid: usize) -> Vec<String> {
    let used = strs(&st.g("used").arr()[pid]);
    strs(&st.g("bfs").arr()[pid]).into_iter().filter(|b| !used.contains(b)).collect()
}

pub fn can_sideboard(st: &Json) -> bool {
    st.g("mode").str() == Some("bo3")
        && !st.g("history").arr().is_empty()
        && matches!(last_winner(st), Some(0) | Some(1))
}

pub fn seed_of(st: &Json) -> i64 {
    st.g("seed").int().unwrap() * 10 + game_no(st)
}

/// L'IA, quand elle choisit, joue en premier.
pub fn ai_first(_st: &Json) -> i64 {
    AI as i64
}

/// `pick_bfs(st, choose_ai_bf, human_bf)` : battlefields de la manche : BO1 au hasard pour les deux (485.5) ; BO3
/// choix (486.5), l'IA par `choose_ai_bf(allowed)` ou au hasard. Le choix de l'IA ne dépend pas de celui du joueur.
pub fn pick_bfs(
    st: &Json,
    choose_ai_bf: Option<&dyn Fn(&[String]) -> Option<String>>,
    human_bf: Option<&str>,
) -> [String; 2] {
    let mut rng = PyRandom::new(seed_of(st) as i128 * 31 + 5);
    if st.g("mode").str() == Some("bo1") {
        let b0 = strs(&st.g("bfs").arr()[0]);
        let b1 = strs(&st.g("bfs").arr()[1]);
        let x = rng.choice(&b0).clone();
        let y = rng.choice(&b1).clone();
        return [x, y];
    }
    let ok_ai = allowed_bfs(st, AI);
    // `(choose_ai_bf(ok_ai) if choose_ai_bf else None) or rng.choice(ok_ai)`
    let mut b_ai = match choose_ai_bf.and_then(|f| f(&ok_ai)).filter(|b| !b.is_empty()) {
        Some(b) => b,
        None => rng.choice(&ok_ai).clone(),
    };
    if !ok_ai.contains(&b_ai) {
        b_ai = rng.choice(&ok_ai).clone();
    }
    let ok_h = allowed_bfs(st, HUMAN);
    let b_h = match human_bf {
        Some(h) if ok_h.iter().any(|b| b == h) => h.to_string(),
        _ => rng.choice(&ok_h).clone(),
    };
    [b_h, b_ai]
}

/// `record(st, first, bfs, win)` : enregistre une manche finie : premier joueur, battlefields joués, vainqueur (0, 1
/// ou -1 pour nulle).
pub fn record(st: &Json, first: i64, bfs: &[Json], win: i64) -> Json {
    let mut st = st.clone();
    // dict(st, wins=list(...), used=[list(x)...], history=list(...)) : les clés gardent leur place
    let w = st.g("wins").clone();
    st.set("wins", w);
    let u = st.g("used").clone();
    st.set("used", u);
    let h = st.g("history").clone();
    st.set("history", h);
    let gn = game_no(&st);
    st.get_mut("history").unwrap().arr_mut().push(jo! {
        "game" => gn, "first" => first, "bfs" => Json::Arr(bfs.to_vec()), "winner" => win,
    });
    if win == 0 || win == 1 {
        let wv = st.get_mut("wins").unwrap().arr_mut();
        let cur = wv[win as usize].int().unwrap();
        wv[win as usize] = Json::Int(cur + 1);
        for p in 0..2 {
            let b = bfs[p].clone();
            st.get_mut("used").unwrap().arr_mut()[p].arr_mut().push(b); // retirés pour le reste du match (486.5)
        }
    }
    st
}

/// `list.remove(x)` : ValueError si absent.
fn remove_one(v: &mut Vec<String>, n: &str) -> Result<(), String> {
    match v.iter().position(|x| x == n) {
        Some(i) => {
            v.remove(i);
            Ok(())
        }
        None => Err("ValueError: list.remove(x): x not in list".into()),
    }
}

/// `sideboard_swap(deck, out_cards, in_cards, champion)` : échange 1 pour 1 (403.4) : out_cards quittent le Main Deck
/// pour le sideboard, in_cards font l'inverse. Le Chosen Champion peut être changé pour une unité Champion du Main Deck
/// ou du sideboard (601.1.c.4).
pub fn sideboard_swap(deck: &Json, out_cards: &[String], in_cards: &[String], champion: Option<&str>) -> Result<Json, String> {
    if out_cards.len() != in_cards.len() {
        return Err(format!(
            "ValueError: {}",
            tr(
                "échange 1 pour 1 : autant de cartes sortent que de cartes entrent (règle 403.4)",
                "1-for-1 swap: as many cards go out as come in (rule 403.4)"
            )
        ));
    }
    let mut main = strs(deck.g("main"));
    let mut side = strs(deck.g("sideboard"));
    for n in out_cards {
        remove_one(&mut main, n)?;
    }
    for n in in_cards {
        remove_one(&mut side, n)?;
    }
    main.extend(in_cards.iter().cloned());
    side.extend(out_cards.iter().cloned());
    let mut d = deck.clone();
    d.set("main", main.clone());
    d.set("sideboard", side.clone());
    let cur = deck.g("champion").str().map(|s| s.to_string());
    if let Some(champion) = champion.filter(|c| !c.is_empty()) {
        if Some(champion) != cur.as_deref() {
            let in_main = main.iter().any(|x| x == champion);
            if !in_main && !side.iter().any(|x| x == champion) {
                return Err(format!(
                    "ValueError: {}",
                    tr(
                        "le nouveau Chosen Champion doit venir du Main Deck ou du sideboard (601.1.c.4)",
                        "the new Chosen Champion must come from the Main Deck or the sideboard (601.1.c.4)"
                    )
                ));
            }
            if in_main {
                remove_one(&mut main, champion)?;
            } else {
                remove_one(&mut side, champion)?;
            }
            // l'ancien champion retourne dans le Main Deck (deck["champion"] : KeyError s'il manque)
            match deck.get("champion") {
                Some(Json::Str(c)) => main.push(c.clone()),
                Some(_) => return Err("TypeError: champion".into()),
                None => return Err("KeyError: 'champion'".into()),
            }
            d.set("main", main);
            d.set("sideboard", side);
            d.set("champion", champion);
        }
    }
    Ok(d)
}
