//! Banc humain : les coups de l'IA que le joueur a marqués « Erreur de l'IA » sur la table (`banc/humain.py`,
//! 2026-10-28).
//!
//! ```text
//!   banc_humain <parties.json ou dossier> [...] [--ver <version>]... [--temoins <n>]
//! ```
//! Chaque partie enregistrée par la table depuis le 2026-10-28 porte les coups de l'IA (champ « ai ») et les marques du
//! joueur (coups `["flag", raison, tour, indice du coup de l'IA, texte, précision]`). La partie est rejouée avec le
//! moteur actuel, l'IA suivant ses coups enregistrés (`train::ai_replay_capture`) : fidèle même si l'IA a changé
//! depuis. À chaque coup marqué, et à `temoins` coups non marqués de l'IA (témoins, à plusieurs options, tirés à graine
//! fixe), la position est copiée et chaque version de l'IA (« table », « table@réglages » ou une version d'`exp_search`)
//! y choisit son coup, avec un agent neuf. Sortie : résumé (proportions, intervalle de Wilson à 95 %) et
//! `banc/data/humain.jsonl` (une ligne par position).
use super::exp_search::version;
use super::train_games::{load_games, replay};
use crate::ai::{set_counters, Cfg, SearchOpts};
use crate::game::Agent;
use crate::jo;
use crate::json::Json;
use crate::plans::plan_agent;
use crate::pyrandom::PyRandom;
use crate::replay::describe;
use crate::train::{self, py_str_json};

fn reason_text(r: &Json) -> String {
    match r.str() {
        Some("cible") => "mauvaise cible".into(),
        Some("moment") => "mauvais moment".into(),
        Some("inutile") => "coup inutile".into(),
        Some("autre") => "autre".into(),
        _ => py_str_json(r),
    }
}

/// `wilson(k, n, z)` : (proportion, borne basse, borne haute)
pub fn wilson(k: usize, n: usize) -> (f64, f64, f64) {
    if n == 0 {
        return (0.0, 0.0, 0.0);
    }
    let (k, n, z) = (k as f64, n as f64, 1.96f64);
    let p = k / n;
    let d = 1.0 + z * z / n;
    let c = (p + z * z / (2.0 * n)) / d;
    let h = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / d;
    (p, (c - h).max(0.0), (c + h).min(1.0))
}

/// Une position retrouvée : (type, indice du coup de l'IA, marque, partie copiée, compteurs, décision)
type Pos = (&'static str, usize, Option<Json>, crate::game::Game, (u32, u32), crate::game::Decision);

/// `positions(game, n_ctrl, seed)` : rejoue la partie ; (positions retrouvées, nombre voulu)
pub fn positions(game: &Json, n_ctrl: usize, seed: i64) -> Result<(Vec<Pos>, usize), String> {
    let flags: Vec<&Json> = game.g("moves").arr().iter().filter(|m| m.arr()[0].str() == Some("flag")).collect();
    let ai = game.g("ai").arr();
    let mut marked: Vec<(usize, &Json)> = vec![]; // dans l'ordre d'insertion d'un dict
    for m in &flags {
        if let Some(i) = m.arr().get(3).and_then(|x| if let Json::Int(i) = x { Some(*i) } else { None }) {
            if i >= 0 && (i as usize) < ai.len() && ai[i as usize].arr()[0].str() == Some("d") {
                match marked.iter_mut().find(|e| e.0 == i as usize) {
                    Some(e) => e.1 = m,
                    None => marked.push((i as usize, m)),
                }
            }
        }
    }
    let others: Vec<usize> = (0..ai.len())
        .filter(|&i| ai[i].arr()[0].str() == Some("d") && ai[i].arr()[3].int().unwrap_or(0) > 1 && !marked.iter().any(|e| e.0 == i))
        .collect();
    let mut ctrl = PyRandom::new(seed as i128 * 7919 + game.g("seed").int().unwrap() as i128).sample(&others, n_ctrl.min(others.len()));
    ctrl.sort();
    let mut want: Vec<usize> = marked.iter().map(|e| e.0).chain(ctrl).collect();
    want.sort();
    want.dedup();
    replay(game, false, &want)?; // l'IA suit son script et copie ces positions
    let mut caught = train::caught();
    let mut out = vec![];
    for &i in &want {
        if let Some((g, cnt, d)) = caught.remove(&i) {
            let mark = marked.iter().find(|e| e.0 == i).map(|e| e.1.clone());
            out.push((if mark.is_some() { "marque" } else { "temoin" }, i, mark, g, cnt, d));
        }
    }
    Ok((out, want.len()))
}

/// `agent(game, ver)` : « table » (ou « table@réglages ») : l'IA de la table au niveau de la partie ; sinon une version
/// d'`exp_search`.
fn agent(game: &Json, ver: &str) -> crate::ai::SearchAgent {
    let decks = train::current_decks();
    let (a, b) = (&decks[0], &decks[1]);
    let (nm, opts) = ver.split_once('@').map(|(n, o)| (n, Some(o))).unwrap_or((ver, None));
    let seed = game.g("seed").int().unwrap() as i128 + 500000;
    let (pl, op) = (train::table_plan_for(b), train::table_plan_for(a));
    if nm == "table" {
        // niveau de la partie (0, tutoriel : niveau 1) ; les coups au hasard des niveaux 1 à 5 ne sont pas rejoués
        let lvl = train::saved_level(game).int().unwrap().max(1);
        let lo = crate::ai::level_opts(lvl);
        let mut cfg: Cfg = lo.cfg;
        cfg.set("wild", crate::ai::CfgV::Num(0.0));
        if let Some(o) = opts.filter(|o| !o.is_empty()) {
            for (k, v) in version(&format!("turn@{o}")).4 .0 {
                cfg.set(&k, v);
            }
        }
        let o = SearchOpts { samples: lo.samples, search: Some("turn".into()), cfg, ..Default::default() };
        return plan_agent(seed, Some(pl), Some(op), o);
    }
    let (sm, n, ex, h, cfg) = version(ver);
    plan_agent(seed, Some(pl), Some(op), SearchOpts { samples: n, search: Some(sm), sh_extra: Some(ex), horizon: h, cfg, ..Default::default() })
}

/// `main(args)`
pub fn main(args: &[String]) -> i32 {
    let (mut vers, mut files, mut n_ctrl) = (vec![], vec![], 20usize);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--ver" => vers.push(it.next().expect("--ver").clone()),
            "--temoins" => n_ctrl = it.next().expect("--temoins").parse().unwrap(),
            _ => files.push(a.clone()),
        }
    }
    if vers.is_empty() {
        vers.push("table".into());
    }
    let games: Vec<Json> = match load_games(&files) {
        Ok(g) => g.into_iter().filter(|g| g.g("ai").truthy() && g.g("moves").arr().iter().any(|m| m.arr()[0].str() == Some("flag"))).collect(),
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    if games.is_empty() {
        eprintln!("aucune partie avec des coups de l'IA enregistrés et des erreurs marquées (table du 2026-10-28 ou après)");
        return 1;
    }
    let (mut rows, mut missing): (Vec<Json>, usize) = (vec![], 0);
    for game in &games {
        let (pos, n_want) = match positions(game, n_ctrl, 0) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{e}");
                return 1;
            }
        };
        missing += n_want - pos.len();
        for (typ, i, mark, mut g, (on, inn), d) in pos {
            let a0 = d.options.as_ref().unwrap()[game.g("ai").arr()[i].arr()[1].int().unwrap() as usize].clone();
            let note = mark.as_ref().and_then(|m| m.arr().get(5).cloned()).filter(|x| x.truthy()).unwrap_or(Json::Null);
            let label = describe(&mut g, &a0, d.player, None);
            let mut new = vec![];
            for v in &vers {
                set_counters(&g, (on, inn));
                let mut c = g.clone_game();
                let mut ag = agent(game, v);
                let a = ag.decide(&mut c, &d).expect("décision");
                let lab = describe(&mut g, &a, d.player, None);
                new.push((v.clone(), jo! { "changed" => a != a0, "label" => lab }));
            }
            rows.push(jo! {
                "id" => game.g("id").clone(), "seed" => game.g("seed").clone(), "step" => i, "typ" => typ, "turn" => g.turn_no,
                "reason" => mark.as_ref().map(|m| m.arr()[1].clone()).unwrap_or(Json::Null), "note" => note, "label" => label,
                "new" => Json::Obj(new),
            });
        }
    }
    let out = super::banc::banc_dir().join("data").join("humain.jsonl");
    std::fs::create_dir_all(out.parent().unwrap()).ok();
    std::fs::write(&out, rows.iter().map(|r| r.dumps(false) + "\n").collect::<String>()).expect("humain.jsonl");
    let count = |t: &str| rows.iter().filter(|r| r.g("typ").str() == Some(t)).count();
    println!(
        "{} parties, {} coups marqués, {} témoins{}",
        games.len(),
        count("marque"),
        count("temoin"),
        if missing > 0 { format!(", {missing} positions non retrouvées") } else { String::new() }
    );
    for v in &vers {
        let mut line = vec![];
        for typ in ["marque", "temoin"] {
            let rs: Vec<&Json> = rows.iter().filter(|r| r.g("typ").str() == Some(typ)).collect();
            let k = rs.iter().filter(|r| r.g("new").g(v).g("changed").truthy()).count();
            let (p, lo, hi) = wilson(k, rs.len());
            line.push(format!("{typ}s changés {k}/{} = {:.0} % [{:.0} ; {:.0}]", rs.len(), 100.0 * p, 100.0 * lo, 100.0 * hi));
        }
        println!("  {v} : {}", line.join(" ; "));
    }
    for r in &rows {
        if r.g("typ").str() == Some("marque") {
            let why = reason_text(r.g("reason")) + &if r.g("note").truthy() { format!(" : « {} »", py_str_json(r.g("note"))) } else { String::new() };
            println!("  tour {} ({}) : {}", py_str_json(r.g("turn")), why, r.g("label").str().unwrap_or(""));
            for v in &vers {
                let x = r.g("new").g(v);
                println!("      {v} : {}", if x.g("changed").truthy() { format!("autre coup : {}", x.g("label").str().unwrap_or("")) } else { "même coup".into() });
            }
        }
    }
    0
}
