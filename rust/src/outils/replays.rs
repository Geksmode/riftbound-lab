//! Outils du lecteur de replays (`riftbound/replays/`) : `add_replays.py`, `batch_replays.py`, `curate_replays.py`.
//! Les parties sont enregistrées par `replay_rec` (même format JSON que `replay.py`).
//!
//! Écarts avec le Python :
//! - `add_replays` joue chaque partie dans un processus à part (le programme se relance lui-même avec `--job`), comme
//!   `Pool(4, maxtasksperchild=1)` : les réglages `tempo`, `refl` et `video` d'une entrée passent par `RB_TEMPO`,
//!   `RB_REFL` et `RB_VIDEO` ;
//! - `picks` : une session du manager Python rejouée avec « son » `plans.py` figé (`plans_snapshot`) n'existe plus ;
//!   les parties sont rejouées avec le moteur actuel, et l'écart avec la simulation est signalé (ATTENTION) ;
//! - dans le dict `cards`, l'ordre des clés est alphabétique (voir `replay_rec`) ;
//! - `RB_REPLAYS_DIR` remplace `riftbound/replays/games` (essais sans toucher aux fichiers du dépôt).
use std::path::{Path, PathBuf};
use std::process::Command;

use super::exp::{deck_l, gorica};
use super::exp_search::{setup, version};
use super::pool;
use super::py::{self, Dump, PyVal};
use crate::ai::SearchOpts;
use crate::json::Json;
use crate::replay::set_names;
use crate::replay_rec::{record, record_plan, summary};
use crate::train::{edit, tload, TDeck};

/// `OUT` : `riftbound/replays/games`
pub fn out_dir() -> PathBuf {
    match std::env::var("RB_REPLAYS_DIR") {
        Ok(d) if !d.is_empty() => d.into(),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../riftbound/replays/games"),
    }
}

const KEYS: [&str; 13] = [
    "joue", "déplace", "prend le", "+1 point", "meurt", "Résolution : LeBlanc, Deceiver", "Résolution : Baited Hook",
    "Résolution : Akali, Rogue", "Résolution : Stellacorn", "Résolution : Astral", "cache", "COMBAT", "décide",
];

/// `decks()` : ({"G2", "G", "Gorica stock"}, L)
fn deck_of(name: &str) -> TDeck {
    let (g, _, _, g2, _) = gorica();
    match name {
        "G2" => g2,
        "G" | "Gorica stock" => g,
        n => panic!("KeyError: {}", PyVal::from(n).py_repr()),
    }
}

/// `flips(path, a, b, seed0, wa, wb)` : graines où A gagne et B perd (même donne, seul le plan change).
pub fn flips(path: &str, a: &str, b: &str, seed0: i64, wa: f64, wb: f64) -> Result<Vec<i64>, String> {
    let all = py::load_file(Path::new(path))?;
    let rs: Vec<&PyVal> = all.as_list().iter().filter(|r| r.get("seed0") == Some(&PyVal::Int(seed0))).collect();
    let table = |label: &str| -> Result<Vec<(i64, f64)>, String> {
        let r = rs
            .iter()
            .rev()
            .find(|r| r.get("label").and_then(|l| l.as_str()) == Some(label))
            .ok_or_else(|| format!("KeyError: {}", PyVal::from(label).py_repr()))?;
        let mut out: Vec<(i64, f64)> = vec![];
        for row in r.get("per_seed").map(|p| p.as_list()).unwrap_or_default() {
            let (k, v) = (row.as_list()[0].as_i64(), row.as_list()[1].as_f64());
            match out.iter_mut().find(|e| e.0 == k) {
                Some(e) => e.1 = v,
                None => out.push((k, v)),
            }
        }
        Ok(out)
    };
    let (ta, tb) = (table(a)?, table(b)?);
    let mut seeds: Vec<i64> = ta.iter().map(|e| e.0).collect();
    seeds.sort();
    Ok(seeds
        .into_iter()
        .filter(|s| {
            ta.iter().find(|e| e.0 == *s).map(|e| e.1) == Some(wa) && tb.iter().find(|e| e.0 == *s).map(|e| e.1) == Some(wb)
        })
        .collect())
}

/// `_kw(search)` : [version joueur 0, version joueur 1] d'exp_search -> réglages des deux agents.
fn kw(search: &[String]) -> [SearchOpts; 2] {
    let o = |v: &str| {
        let (sm, n, ex, h, cfg) = version(v);
        SearchOpts { samples: n, search: Some(sm), sh_extra: Some(ex), horizon: h, cfg, ..Default::default() }
    };
    [o(&search[0]), o(&search[1])]
}

/// `rec(seed, deck, a_plan, l_plan, search, mode)`
pub fn rec(seed: i64, deck: &str, a_plan: Option<&str>, l_plan: Option<&str>, search: Option<&[String]>, mode: Option<&str>) -> Json {
    let k = search.map(kw);
    if let Some(m @ ("defaut" | "miroir")) = mode {
        let s = setup(seed, m);
        let mut nm: Vec<String> = s.decks.iter().map(|d| d.legend.split(',').next().unwrap().to_string()).collect();
        if nm[0] == nm[1] {
            nm[1].push_str(" (2)");
        }
        set_names([nm[0].clone(), nm[1].clone()]);
        let (b0, b1) = ([s.decks[0].battlefield.clone()], [s.decks[1].battlefield.clone()]);
        return record_plan(seed, &s.decks[0], &s.decks[1], None, None, Some(&b0), Some(&b1), Some(s.first), k);
    }
    record_plan(seed, &deck_of(deck), &deck_l(), a_plan, l_plan, None, None, None, k)
}

/// Écriture `repr()` d'un dict `mulligan` de Python (clés entières avant le passage par JSON).
fn mull_repr(m: &Json) -> String {
    let rows: Vec<String> =
        m.obj().iter().map(|(k, v)| format!("{}: {}", k, PyVal::from_json(v).py_repr())).collect();
    format!("{{{}}}", rows.join(", "))
}

/// `digest(rep, width)`
pub fn digest(rep: &Json, width: usize) -> String {
    let default = ["Akali", "LeBlanc"];
    let nm: Vec<String> = rep
        .g("players")
        .arr()
        .iter()
        .zip(default)
        .map(|(p, x)| p.g("name").str().filter(|s| !s.is_empty()).unwrap_or(x).to_string())
        .collect();
    let w = rep.g("winner").int();
    let win = match w {
        Some(0) | Some(1) => nm[w.unwrap() as usize].clone(),
        _ => "nul".to_string(),
    };
    let r = |j: &Json| PyVal::from_json(j).py_repr();
    let bfs = Json::Arr(rep.g("players").arr().iter().map(|p| p.g("bf").clone()).collect());
    let mut out = vec![
        format!(
            "graine {} plans {} joueurs {} premier={} bf={} gagnant={} {}",
            rep.g("seed").int().unwrap(),
            r(rep.g("plans")),
            r(&Json::Arr(nm.iter().map(Json::from).collect())),
            nm[rep.g("first").int().unwrap() as usize],
            r(&bfs),
            win,
            r(rep.g("pts"))
        ),
        format!("mulligan {}", mull_repr(rep.g("mulligan"))),
    ];
    for fr in rep.g("frames").arr() {
        for e in fr.g("ev").arr() {
            let t = e.arr()[1].str().unwrap_or("");
            if t.starts_with("Tour ") {
                out.push(format!("== {t}  score {}", r(fr.g("st").g("pts"))));
            } else if KEYS.iter().any(|k| t.contains(k)) && !t.contains("Déclenchement") {
                out.push(format!("   {}", t.chars().take(width).collect::<String>()));
            }
        }
    }
    out.join("\n")
}

/// `_build(spec)` : deck d'un pick du manager (base, retraits, ajouts).
fn build(spec: &PyVal) -> TDeck {
    let pairs = |k: &str| -> Vec<(String, usize)> {
        spec.get(k)
            .map(|l| l.as_list().iter().map(|x| (x.as_list()[0].py_str(), x.as_list()[1].as_i64() as usize)).collect())
            .unwrap_or_default()
    };
    let (o, a) = (pairs("out"), pairs("add"));
    let o: Vec<(&str, usize)> = o.iter().map(|(n, q)| (n.as_str(), *q)).collect();
    let a: Vec<(&str, usize)> = a.iter().map(|(n, q)| (n.as_str(), *q)).collect();
    edit(&tload(&spec.get("base").unwrap().py_str()), &o, &a)
}

fn strs(v: Option<&PyVal>) -> Option<Vec<String>> {
    match v {
        Some(PyVal::List(l)) => Some(l.iter().map(|x| x.py_str()).collect()),
        _ => None,
    }
}

fn opt_str(v: Option<&PyVal>) -> Option<String> {
    v.filter(|x| x.truthy()).map(|x| x.py_str())
}

/// Une entrée de spec : joue la partie, écrit `<id>.json` et rend la ligne d'index (corps de `_job`).
pub fn job(e: &PyVal) -> PyVal {
    let seed = e.get("seed").unwrap().as_i64();
    let rep = if let Some(j) = e.get("job") {
        // pick du manager
        let first = j.get("first").filter(|v| !matches!(v, PyVal::None)).map(|v| v.as_i64() as usize);
        let (ab, lb) = (strs(j.get("akali_bfs")), strs(j.get("opp_bfs")));
        record_plan(
            seed,
            &build(j.get("akali").unwrap()),
            &build(j.get("opp").unwrap()),
            opt_str(j.get("akali_plan")).as_deref(),
            opt_str(j.get("opp_plan")).as_deref(),
            ab.as_deref(),
            lb.as_deref(),
            first,
            None,
        )
    } else {
        let search = strs(e.get("search"));
        rec(
            seed,
            &e.get("deck").map(|d| d.py_str()).unwrap_or_else(|| "G2".into()),
            opt_str(e.get("a_plan")).as_deref(),
            opt_str(e.get("l_plan")).as_deref(),
            search.as_deref(),
            e.get("mode").filter(|m| m.truthy()).and_then(|m| m.as_str()),
        )
    };
    let id = e.get("id").unwrap().py_str();
    std::fs::create_dir_all(out_dir()).ok();
    std::fs::write(out_dir().join(format!("{id}.json")), rep.dump()).expect("écriture du replay");
    let nz = |k: &str| e.get(k).cloned().unwrap_or(PyVal::None);
    crate::pydict! {
        "id" => id.clone(), "file" => format!("{id}.json"), "group" => e.get("group").cloned().unwrap_or(PyVal::from("")),
        "session" => nz("session"), "seed" => seed, "title" => e.get("title").unwrap().clone(), "note" => e.get("note").unwrap().clone(),
        "plans" => PyVal::from_json(rep.g("plans")), "winner" => PyVal::from_json(rep.g("winner")), "pts" => PyVal::from_json(rep.g("pts")),
        "first" => PyVal::from_json(rep.g("first")), "turns" => PyVal::from_json(rep.g("turns")),
        "bf" => PyVal::List(rep.g("players").arr().iter().map(|p| PyVal::from_json(p.g("bf"))).collect()),
        "ia" => if crate::ai::conf().tempo { "tempo" } else { "ancienne" },
        "names" => PyVal::List(rep.g("players").arr().iter().map(|p| PyVal::from_json(p.g("name"))).collect()),
    }
}

/// Une entrée jouée dans un processus à part (`--job <entrée JSON>` : la ligne d'index sort sur la sortie standard).
fn spawn_job(e: &PyVal) -> PyVal {
    let mut c = Command::new(std::env::current_exe().expect("programme"));
    c.arg("--job").arg(py::dumps(e, Dump { indent: None, ensure_ascii: false }));
    for (k, var) in [("tempo", "RB_TEMPO"), ("refl", "RB_REFL"), ("video", "RB_VIDEO")] {
        if let Some(v) = e.get(k) {
            c.env(var, if v.truthy() { "1" } else { "0" });
        }
    }
    let o = c.output().expect("processus de partie");
    if !o.status.success() {
        panic!("partie en échec : {}", String::from_utf8_lossy(&o.stderr));
    }
    py::parse(String::from_utf8_lossy(&o.stdout).trim()).expect("ligne d'index")
}

/// `add(entries)`
pub fn add(entries: &[PyVal]) -> Vec<PyVal> {
    let new = pool::map(4, entries, spawn_job);
    let f = out_dir().join("index.json");
    let idx = if f.exists() { py::load_file(&f).unwrap() } else { PyVal::List(vec![]) };
    let ids: Vec<String> = new.iter().map(|e| e.get("id").unwrap().py_str()).collect();
    let mut out: Vec<PyVal> =
        idx.as_list().iter().filter(|e| !ids.contains(&e.get("id").unwrap().py_str())).cloned().collect();
    out.extend(new.iter().cloned());
    py::dump_file(&f, &PyVal::List(out), Dump { indent: Some(1), ensure_ascii: false });
    for e in &new {
        let w = e.get("winner").unwrap();
        let who = match w {
            PyVal::Int(0) | PyVal::Int(1) => e.get("names").unwrap().as_list()[w.as_i64() as usize].py_str(),
            _ => "nul".into(),
        };
        println!("{} {} {} {} {}", e.get("id").unwrap().py_str(), e.get("seed").unwrap().py_str(), e.get("plans").unwrap().py_repr(), who, e.get("pts").unwrap().py_repr());
    }
    new
}

/// `picks(path)` : parties choisies par le manager ; vérifie que le gagnant rejoué est celui de la simulation.
pub fn picks(path: &str) -> Result<(), String> {
    let path = std::fs::canonicalize(path).map_err(|e| format!("{path}: {e}"))?;
    let ps = py::load_file(&path)?;
    let entries: Vec<PyVal> = ps
        .as_list()
        .iter()
        .map(|k| {
            let label = k.get("job").and_then(|j| j.get("label")).map(|l| l.py_str()).unwrap_or_default();
            let sess = k.get("session").map(|s| s.py_str()).unwrap_or_else(|| "?".into());
            let g: String = format!("Agent manager : session {sess} ({label})").chars().take(120).collect();
            let mut d = k.clone();
            d.set("group", PyVal::Str(g));
            d.set("snap", PyVal::None);
            d
        })
        .collect();
    let new = add(&entries);
    let res = path.with_file_name(path.file_name().unwrap().to_string_lossy().replace("_picks", ""));
    if res.exists() {
        let data = py::load_file(&res)?;
        let rows: Vec<PyVal> = match &data {
            PyVal::List(l) => l.clone(),
            d => d.get("results").map(|r| r.as_list().to_vec()).unwrap_or_default(),
        };
        let mut sim: Vec<((String, i64), f64)> = vec![];
        for r in &rows {
            for row in r.get("per_seed").map(|p| p.as_list()).unwrap_or_default() {
                let key = (r.get("label").map(|l| l.py_str()).unwrap_or_else(|| "None".into()), row.as_list()[0].as_i64());
                match sim.iter_mut().find(|e| e.0 == key) {
                    Some(e) => e.1 = row.as_list()[1].as_f64(),
                    None => sim.push((key, row.as_list()[1].as_f64())),
                }
            }
        }
        for (k, e) in ps.as_list().iter().zip(&new) {
            let key = (
                k.get("job").and_then(|j| j.get("label")).map(|l| l.py_str()).unwrap_or_else(|| "None".into()),
                k.get("seed").unwrap().as_i64(),
            );
            let w = e.get("winner").unwrap();
            let got = if *w == PyVal::Int(0) { 1.0 } else if *w == PyVal::Int(-1) { 0.5 } else { 0.0 };
            if let Some(s) = sim.iter().find(|x| x.0 == key).map(|x| x.1) {
                if s != got {
                    println!("ATTENTION {} : rejouée {}, simulation {}", e.get("id").unwrap().py_str(), py::fstr(got), py::fstr(s));
                }
            }
        }
    }
    Ok(())
}

/// `__main__` d'`add_replays.py`
pub fn add_main(a: &[String]) -> i32 {
    if a.first().map(|s| s.as_str()) == Some("--job") {
        // sous-processus : une partie
        let e = py::parse(&a[1]).expect("entrée illisible");
        println!("{}", py::dumps(&job(&e), Dump { indent: None, ensure_ascii: false }));
        return 0;
    }
    let Some(cmd) = a.first() else {
        eprintln!("add_replays flips|digest|dsearch|add|picks ...");
        return 2;
    };
    let arg = |i: usize| a.get(i).unwrap_or_else(|| panic!("IndexError: argument {i}")).as_str();
    match cmd.as_str() {
        "flips" => match flips(arg(1), arg(2), arg(3), arg(4).parse().unwrap(), 1.0, 0.0) {
            Ok(v) => println!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")),
            Err(e) => {
                eprintln!("{e}");
                return 1;
            }
        },
        "digest" => {
            let ap = Some(arg(3)).filter(|s| *s != "-");
            let lp = a.get(4).map(|s| s.as_str()).filter(|s| *s != "-");
            println!("{}", digest(&rec(arg(1).parse().unwrap(), arg(2), ap, lp, None, None), 150));
        }
        "dsearch" => {
            // dsearch <graine> <defaut|miroir> <version j0> <version j1>
            let v = [arg(3).to_string(), arg(4).to_string()];
            println!("{}", digest(&rec(arg(1).parse().unwrap(), "G2", None, None, Some(&v), Some(arg(2))), 150));
        }
        "add" => {
            let spec = match py::load_file(Path::new(arg(2 - 1))) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    return 1;
                }
            };
            add(spec.as_list());
        }
        "picks" => {
            if let Err(e) = picks(arg(1)) {
                eprintln!("{e}");
                return 1;
            }
        }
        _ => {}
    }
    0
}

// ------------------------------------------------------------------------------------------------ batch_replays
fn ratio_pct(x: f64) -> String {
    format!("{:.1}%", 100.0 * x)
}

/// `batch_replays.py <n> <seed0> <dossier>` : un lot de parties Gorica G2 contre LeBlanc IQ#5.
pub fn batch_main(a: &[String]) -> i32 {
    if a.len() < 3 {
        eprintln!("batch_replays <n> <seed0> <dossier>");
        return 2;
    }
    let (n, s0): (i64, i64) = (a[0].parse().unwrap(), a[1].parse().unwrap());
    let dir = PathBuf::from(&a[2]);
    std::fs::create_dir_all(&dir).expect("dossier");
    let (_, _, _, g2, _) = gorica();
    let l = deck_l();
    let seeds: Vec<i64> = (s0..s0 + n).collect();
    let out: Vec<Result<Json, (i64, String)>> = pool::map(pool::cpu_count(), &seeds, |&seed| {
        match pool::attempt(|| record(seed, &g2, &l, None, None, None, None)) {
            Ok(rep) => {
                std::fs::write(dir.join(format!("seed_{seed}.json")), rep.dump()).expect("écriture");
                Ok(summary(&rep))
            }
            Err(e) => Err((seed, e)),
        }
    });
    let rows: Vec<PyVal> = out
        .iter()
        .map(|r| match r {
            Ok(j) => PyVal::from_json(j),
            Err((s, e)) => crate::pydict! { "seed" => *s, "err" => e.chars().take(300).collect::<String>() },
        })
        .collect();
    py::dump_file(&dir.join("index.json"), &PyVal::List(rows), Dump { indent: Some(0), ensure_ascii: false });
    let good: Vec<&Json> = out.iter().filter_map(|r| r.as_ref().ok()).collect();
    let w = good.iter().filter(|o| o.g("winner").int() == Some(0)).count();
    println!("{} parties, Akali gagne {} ({}), erreurs {}", good.len(), w, ratio_pct(w as f64 / good.len() as f64), out.len() - good.len());
    0
}

// ---------------------------------------------------------------------------------------------- curate_replays
/// `PICKS` de `curate_replays.py` : (id, graine, titre, note)
const PICKS: [(&str, i64, &str, &str); 8] = [
    ("tempo", 60044, "Akali commence, 9-1", "Initiative : Mournful Witness prend Star Spring au tour 3, Shuriken Flip tue Karthus et ouvre Sigil. LeBlanc ne reprend jamais la main."),
    ("etouffee", 60139, "LeBlanc commence, 1-8", "La défaite type en jouant second : Watchful Sentry prend les deux battlefields au tour 3, Sacrifice ramène Karthus, Windswept Hillock lui laisse reprendre le terrain."),
    ("second", 60234, "Akali joue second, 8-1", "Réponse au départ de LeBlanc : Akali reprend Dusk Rose Lab, enchaîne Shuriken Flip vers Void Gate et cache une carte. L'IA LeBlanc reste ensuite passive."),
    ("comeback", 60099, "Retournement de 0-5 à 9-6", "Menée 0-5 face à LeBlanc, Everywhere At Once, Akali reconstruit avec Scuttle Crab, Lonely Poro et Ferrous Forerunner et gagne trois combats d'affilée au tour 10."),
    ("dernier-point", 60029, "Bloquée à 7, perdue 7-8", "Akali mène 7-3, mais le dernier point par conquête exige d'avoir scoré les deux battlefields dans le tour. Elle pioche à la place pendant que LeBlanc remonte par la tenue de Star Spring."),
    ("bras-de-fer", 60037, "Bras de fer gagné 8-7", "Chacune tient un battlefield de la mi-partie à la fin. Marai cachée et le Mech de Forerunner gardent Dusk Rose Lab, et Akali gagne la course à la tenue d'un point."),
    ("side-plan-perdu", 60020, "Tout le side plan en jeu, perdue 7-8", "Silent, Forerunner, Poro et Crab sortent tous, mais deux Hidden Blade tuent Akali puis Forerunner. LeBlanc a commencé et garde un point d'avance dans la course."),
    ("side-plan-gagne", 60068, "Silent et Forerunner, 8-2", "Akali joue second et perd Deadly Weapon tôt, puis déborde LeBlanc avec un board large : Akali, Silent, Forerunner, Kai'Sa et Noxus Hopeful."),
];

/// `curate_replays.py` : ré-enregistre la sélection de parties commentées dans `replays/games/`.
pub fn curate_main(_a: &[String]) -> i32 {
    let out = out_dir();
    std::fs::create_dir_all(&out).expect("dossier");
    let (_, _, _, g2, _) = gorica();
    let l = deck_l();
    let idx: Vec<PyVal> = pool::map(4, &PICKS, |&(gid, seed, title, note)| {
        let rep = record(seed, &g2, &l, None, None, None, None);
        std::fs::write(out.join(format!("{gid}.json")), rep.dump()).expect("écriture");
        let g = |k: &str| PyVal::from_json(rep.g(k));
        crate::pydict! {
            "id" => gid, "file" => format!("{gid}.json"), "seed" => seed, "title" => title, "note" => note, "winner" => g("winner"),
            "pts" => g("pts"), "first" => g("first"), "turns" => g("turns"),
            "bf" => PyVal::List(rep.g("players").arr().iter().map(|p| PyVal::from_json(p.g("bf"))).collect()),
            "group" => "Premières parties commentées (G2 sans plan, graines 60000+)", "plans" => vec!["aucun", "aucun"],
        }
    });
    let f = out.join("index.json");
    let old = if f.exists() { py::load_file(&f).unwrap() } else { PyVal::List(vec![]) };
    let ids: Vec<String> = idx.iter().map(|o| o.get("id").unwrap().py_str()).collect();
    // garder les parties ajoutées par add_replays
    let mut all: Vec<PyVal> = old.as_list().iter().filter(|o| !ids.contains(&o.get("id").unwrap().py_str())).cloned().collect();
    all.extend(idx.iter().cloned());
    py::dump_file(&f, &PyVal::List(all.clone()), Dump { indent: Some(1), ensure_ascii: false });
    for o in &all {
        println!("{} {} {} {}", o.get("id").unwrap().py_str(), o.get("seed").unwrap().py_str(), o.get("winner").unwrap().py_str(), o.get("pts").unwrap().py_repr());
    }
    0
}
