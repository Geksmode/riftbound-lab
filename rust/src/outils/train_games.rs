//! `train_games.py` et `bilan_parties.py` : rejouer les parties jouées sur la table d'entraînement et en tirer ce que le
//! joueur a fait.
//!
//! ```text
//!   train_games riftbound/train/games/            une fiche par partie dans riftbound/train/analyses/
//!   train_games fichier.json --coach              ajoute, à chaque coup, ce que l'IA Akali aurait joué
//!   bilan_parties [dossier des parties] [choices.jsonl]
//! ```
//! La page enregistre chaque partie (réglages, suite exacte des choix `["act", i, libellé, tour]`, `["ans", x, ...]`,
//! `["undo"]`, `["hint", n]`, journal affiché). Le moteur est déterministe : rejouer ces choix avec l'API de la table
//! (`train::call`) redonne toute la partie, coups de l'IA compris (script `ai` si la page l'a enregistré).
//!
//! Écarts avec le Python :
//! - `--frozen` et `--emit` (moteurs Python figés `train/engine_versions/<ver>` et moteur exact d'un commit,
//!   `git archive` de `riftbound/engine`) : ces copies sont du Python et disparaissent avec lui. `--frozen` rejoue donc
//!   chaque partie avec le moteur actuel (étape « actuel » de Python) et signale celles qui ne sont pas fidèles ;
//!   `--emit` écrit les mêmes lignes `GAME` / `CHOICE` qu'en Python ;
//! - `bilan_parties` : l'ordre des lignes du tableau « Type / Toi / Coach » à égalité de « Toi » est alphabétique
//!   (Python itère un `set` de textes : ordre variable d'une exécution à l'autre) ;
//! - `RB_ANALYSES` (dossier des fiches) et `RB_SHEETS` sont lus comme en Python.
use std::path::{Path, PathBuf};

use crate::json::{self, Json};
use crate::jo;
use crate::train::{self, py_str_json};

/// `OUT` : `riftbound/train/analyses` (ou `RB_ANALYSES`)
pub fn out_dir() -> PathBuf {
    match std::env::var("RB_ANALYSES") {
        Ok(d) if !d.is_empty() => d.into(),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../riftbound/train/analyses"),
    }
}

fn call(f: &str, a: Vec<Json>) -> Result<Json, String> {
    let s = train::call(f, &a)?;
    json::parse(&s).map_err(|e| format!("JSONDecodeError: {e}"))
}

/// `load_games(paths)`
pub fn load_games(paths: &[String]) -> Result<Vec<Json>, String> {
    let mut out = vec![];
    for p in paths {
        let files: Vec<PathBuf> = if Path::new(p).is_dir() {
            let mut v: Vec<PathBuf> = std::fs::read_dir(p)
                .map_err(|e| format!("{p}: {e}"))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|f| f.extension().is_some_and(|x| x == "json"))
                .collect();
            v.sort();
            v
        } else {
            vec![PathBuf::from(p)]
        };
        for f in files {
            let text = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
            let d = json::parse(&text).map_err(|e| format!("{}: {e}", f.display()))?;
            let items: Vec<Json> = match &d {
                Json::Arr(v) => v.clone(),
                x => vec![x.clone()],
            };
            for g in items {
                let g = match &g {
                    Json::Obj(_) if g.get("data").is_some() => g.g("data").clone(),
                    _ => g,
                };
                if matches!(g, Json::Obj(_)) && g.get("moves").is_some() {
                    let mut g = g;
                    if let Json::Obj(v) = &mut g {
                        v.push(("_file".into(), Json::Str(f.display().to_string())));
                    }
                    out.push(g);
                }
            }
        }
    }
    Ok(out)
}

/// `_state(v)`
fn state(v: &Json) -> Json {
    let st = v.g("st");
    let (me, op) = (&st.g("p").arr()[0], &st.g("p").arr()[1]);
    let names = |us: &[Json], c: i64| -> Json {
        Json::Arr(us.iter().filter(|u| u.g("c").int() == Some(c)).map(|u| u.g("n").clone()).collect())
    };
    let bfs: Vec<Json> = st
        .g("bfs")
        .arr()
        .iter()
        .map(|b| jo! { "n" => b.g("n").clone(), "c" => b.g("c").clone(), "me" => names(b.g("u").arr(), 0), "opp" => names(b.g("u").arr(), 1) })
        .collect();
    jo! {
        "t" => st.g("t").clone(), "pts" => st.g("pts").clone(), "hand" => me.g("hand").arr().len(), "opp_hand" => op.g("hand").arr().len(),
        "runes" => me.g("runes").arr().len(), "opp_runes" => op.g("runes").arr().len(), "bfs" => Json::Arr(bfs),
        "base" => Json::Arr(me.g("base").arr().iter().map(|u| u.g("n").clone()).collect()),
        "opp_base" => Json::Arr(op.g("base").arr().iter().map(|u| u.g("n").clone()).collect()),
    }
}

/// `drain(v)` : le journal de chaque cran jusqu'à ce que la table ne soit plus occupée.
fn drain(log: &mut Vec<String>, mut v: Json) -> Result<Json, String> {
    loop {
        for e in v.g("log").arr() {
            let e = e.arr();
            let ind = e[0].truthy();
            log.push(format!("{}{}", if ind { "  " } else { "" }, e[1].str().unwrap_or("")));
        }
        if !v.g("busy").truthy() {
            return Ok(v);
        }
        v = call("step", vec![])?;
    }
}

/// `replay(game, coach, capture)` : (journal, coups du joueur avec leur état, vue finale).
pub fn replay(game: &Json, coach: bool, capture: &[usize]) -> Result<(Vec<String>, Vec<Json>, Json), String> {
    let level = train::saved_level(game);
    let base = vec![game.g("seed").clone(), game.g("bf").clone(), game.g("first").clone(), level];
    let mut args = base.clone();
    if game.g("obf").truthy() {
        // match BO1/BO3 : battlefield de l'IA choisi avant la manche
        args.extend([game.g("mine").clone(), game.g("opp").clone(), game.g("obf").clone()]);
    } else if game.g("mine").truthy() || game.g("opp").truthy() {
        args.extend([game.g("mine").clone(), game.g("opp").clone()]);
    }
    train::call("new", &args)?;
    if game.g("ai").truthy() {
        train::ai_replay_capture(game.g("ai"), capture)?; // coups de l'IA enregistrés (table depuis le 2026-10-28)
    }
    let mut v = call("step", vec![])?;
    let (mut log, mut mine): (Vec<String>, Vec<Json>) = (vec![], vec![]);
    v = drain(&mut log, v)?;
    for m in game.g("moves").arr() {
        let m = m.arr();
        let k = m[0].str().unwrap_or("");
        if k == "hint" {
            train::call("hint", &[m[1].clone()])?;
            continue;
        }
        if k == "flag" {
            // coup de l'IA marqué comme une erreur (banc/humain.py)
            continue;
        }
        if k == "undo" {
            v = drain(&mut log, call("undo", vec![])?)?;
            if !mine.is_empty() {
                // le coup repris ne compte pas comme un choix du joueur
                while mine.last().is_some_and(|x| x.g("k").str() != Some("act")) {
                    mine.pop();
                }
                mine.pop();
            }
            continue;
        }
        let mut rec = jo! {
            "k" => k, "x" => m.get(1).cloned().unwrap_or(Json::Null),
            "label" => m.get(2).cloned().unwrap_or(Json::Str(String::new())), "state" => state(&v),
        };
        let nopts = v.g("dec").g("options").arr().len() as i64;
        let xi = m.get(1).and_then(|x| x.int());
        if k == "act" && v.g("dec").truthy() && xi.is_some_and(|i| i < nopts) {
            let i = xi.unwrap();
            let ii = if i < 0 { i + nopts } else { i } as usize;
            let o = v.g("dec").g("options").arr()[ii].clone();
            rec.set("kind", o.g("k").clone());
            rec.set("phase", v.g("dec").g("kind").clone());
            rec.set("n_opts", nopts);
            rec.set("label", o.g("label").clone());
            if coach {
                let h = call("hint", vec![Json::Int(1)])?;
                rec.set("coach", h.arr().first().map(|x| x.g("label").clone()).unwrap_or(Json::Null));
            }
            v = drain(&mut log, call("act", vec![Json::Int(i)])?)?;
        } else if k == "ans" && v.g("ask").truthy() {
            rec.set("kind", format!("ask:{}", v.g("ask").g("kind").str().unwrap_or("")));
            v = drain(&mut log, call("answer", vec![Json::Str(m[1].dumps(true))])?)?;
        } else {
            rec.set("desync", true);
            mine.push(rec);
            break;
        }
        mine.push(rec);
    }
    Ok((log, mine, v))
}

/// `sheet(game, log, mine, v)` : la fiche de la partie (Markdown).
pub fn sheet(game: &Json, log: &[String], mine: &[Json], v: &Json) -> String {
    let res = game.g("result");
    let w = v.g("winner").int();
    let st = v.g("st");
    let mut ok = !res.truthy() || (res.g("winner").int() == w && res.g("pts") == st.g("pts"));
    let glog = game.g("log").arr();
    if game.g("log").truthy() && glog.len() < 2500 && !game.g("moves").arr().iter().any(|m| m.arr()[0].str() == Some("undo")) {
        // la page garde le journal affiché (sans les lignes retirées par « Reprendre ») : il doit être le même
        let a: Vec<&str> = glog.iter().map(|l| l.str().unwrap_or("").trim()).collect();
        let b: Vec<&str> = log.iter().map(|l| l.trim()).collect();
        ok = ok && a == b;
    }
    let started: String = game.g("started").str().unwrap_or("").chars().take(16).collect::<String>().replace('T', " ");
    let bfs: Vec<&str> = game.g("bfs").arr().iter().map(|b| b.str().unwrap_or("")).collect();
    let pts = st.g("pts").arr();
    let played = mine
        .iter()
        .filter(|m| m.g("k").str() == Some("act") && !matches!(m.g("kind").str(), Some("pass") | Some("end")))
        .count();
    let desync = mine.iter().any(|m| m.g("desync").truthy());
    let mut lines = vec![
        format!("# Partie {} ({})", py_str_json(game.g("id")), started),
        String::new(),
        format!(
            "- Donne {}, battlefields {}, {}, IA niveau {}",
            py_str_json(game.g("seed")),
            bfs.join(" / "),
            if game.g("whoFirst").int() == Some(0) { "tu commences" } else { "LeBlanc commence" },
            py_str_json(game.g("level"))
        ),
        format!(
            "- Résultat : {} {}-{} au tour {}",
            if w == Some(0) { "victoire Akali" } else if w == Some(1) { "victoire LeBlanc" } else { "partie non finie" },
            py_str_json(&pts[0]),
            py_str_json(&pts[1]),
            py_str_json(st.g("t"))
        ),
        format!("- Rejeu fidèle à la page : {}", if ok && !desync { "oui" } else { "NON, à vérifier" }),
        format!("- Tes décisions : {} (dont {} coups joués)", mine.len(), played),
        String::new(),
    ];
    let coach: Vec<&Json> = mine.iter().filter(|m| m.g("coach").truthy()).collect();
    if !coach.is_empty() {
        let same = coach.iter().filter(|m| m.g("coach") == m.g("label")).count();
        lines.push(format!("- Même coup que l'IA Akali : {same}/{}", coach.len()));
        lines.push(String::new());
    }
    lines.push("## Journal".into());
    lines.push("```".into());
    lines.extend(log.iter().cloned());
    lines.push("```".into());
    lines.join("\n")
}

/// `analyse(games, coach, emit, engine_label)` : une fiche par partie ; les coups du joueur de toutes les parties.
pub fn analyse(games: &[Json], coach: bool, emit: bool, label: &str) -> Vec<Json> {
    let pre = if emit { "GAME " } else { "" };
    let mut rows = vec![];
    for g in games {
        let r = crate::outils::pool::attempt(|| replay(g, coach, &[]));
        let (log, mine, v) = match r {
            Ok(Ok(x)) => x,
            Ok(Err(e)) | Err(e) => {
                let e: String = e.chars().take(300).collect();
                println!("{pre}{}", jo! { "id" => g.g("id").clone(), "error" => e, "engine" => label }.dumps(true));
                continue;
            }
        };
        let txt = sheet(g, &log, &mine, &v).replacen("\n## Journal", &format!("- Moteur du rejeu : {label}\n\n## Journal"), 1);
        let dir = match std::env::var("RB_SHEETS") {
            Ok(d) if !d.is_empty() => PathBuf::from(d),
            _ => out_dir(),
        };
        std::fs::write(dir.join(format!("{}.md", py_str_json(g.g("id")))), &txt).expect("écriture de la fiche");
        let faithful = txt.contains("Rejeu fidèle à la page : oui");
        for m in &mine {
            let mut row = jo! {
                "game" => g.g("id").clone(), "ver" => g.g("ver").clone(), "engine" => label, "faithful" => faithful,
                "won" => v.g("winner").int() == Some(0),
            };
            for (k, x) in m.obj() {
                row.set(k, x.clone());
            }
            rows.push(row);
        }
        println!(
            "{pre}{}",
            jo! {
                "id" => g.g("id").clone(), "winner" => v.g("winner").clone(), "pts" => v.g("st").g("pts").clone(),
                "n" => mine.len(), "faithful" => faithful, "engine" => label,
            }
            .dumps(true)
        );
    }
    rows
}

/// `main(args)` de `train_games.py`
pub fn main(args: &[String]) -> i32 {
    let (coach, frozen, emit) = (args.iter().any(|a| a == "--coach"), args.iter().any(|a| a == "--frozen"), args.iter().any(|a| a == "--emit"));
    let mut paths: Vec<String> = args.iter().filter(|a| !a.starts_with("--")).cloned().collect();
    if paths.is_empty() {
        paths.push(out_dir().join("..").join("games").display().to_string());
    }
    let games = match load_games(&paths) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    std::fs::create_dir_all(out_dir()).ok();
    if frozen {
        eprintln!("--frozen : les moteurs Python figés n'existent plus ; chaque partie est rejouée avec le moteur actuel.");
    }
    let label = std::env::var("RB_ENGINE_LABEL").ok().filter(|_| emit).unwrap_or_else(|| "actuel".to_string());
    let rows = analyse(&games, coach, emit, &label);
    if emit {
        // sous-processus : renvoie ses lignes
        for r in rows {
            println!("CHOICE {}", r.dumps(false));
        }
        return 0;
    }
    let mut out = String::new();
    for r in &rows {
        out.push_str(&r.dumps(false));
        out.push('\n');
    }
    std::fs::write(out_dir().join("choices.jsonl"), out).expect("écriture de choices.jsonl");
    0
}

// ------------------------------------------------------------------------------------------------ bilan_parties
fn kind_of(label: &str) -> &'static str {
    if label.starts_with("joue") {
        "jouer une carte"
    } else if label.starts_with("déplace") {
        "déplacer"
    } else if label.starts_with("termine") || label.starts_with("passe") {
        "passer / finir"
    } else if label.starts_with("cache") {
        "cacher"
    } else {
        "légende / capacité"
    }
}

fn pts_text(p: &Json) -> String {
    p.arr().iter().map(py_str_json).collect::<Vec<_>>().join("-")
}

/// `bilan_parties.py [dossier des parties] [choices.jsonl]` : écrit `BILAN.md` à côté de `choices.jsonl` et l'imprime.
pub fn bilan_main(args: &[String]) -> i32 {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("../riftbound/engine");
    let games = args.first().cloned().unwrap_or_else(|| here.join("..").join("train").join("games").join("games").display().to_string());
    let choices = args.get(1).map(PathBuf::from).unwrap_or_else(|| here.join("..").join("train").join("analyses").join("choices.jsonl"));
    let out_f = choices.parent().unwrap().join("BILAN.md");
    // parties : id -> partie, dans l'ordre alphabétique des ids
    let mut meta: Vec<(String, Json)> = vec![];
    let mut files: Vec<PathBuf> = std::fs::read_dir(&games)
        .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).filter(|f| f.extension().is_some_and(|x| x == "json")).collect())
        .unwrap_or_default();
    files.sort();
    for f in files {
        let d = json::parse(&std::fs::read_to_string(&f).unwrap()).unwrap();
        let items: Vec<Json> = match &d {
            Json::Arr(v) => v.clone(),
            x => vec![x.clone()],
        };
        for g in items {
            let g = if g.get("data").is_some() { g.g("data").clone() } else { g };
            if matches!(g, Json::Obj(_)) && g.get("moves").is_some() {
                let id = py_str_json(g.g("id"));
                match meta.iter_mut().find(|e| e.0 == id) {
                    Some(e) => e.1 = g,
                    None => meta.push((id, g)),
                }
            }
        }
    }
    meta.sort_by(|a, b| a.0.cmp(&b.0));
    let rows: Vec<Json> = match std::fs::read_to_string(&choices) {
        Ok(t) => t.lines().filter(|l| !l.is_empty()).map(|l| json::parse(l).unwrap()).collect(),
        Err(_) => vec![],
    };
    let by = |gid: &str| -> Vec<&Json> { rows.iter().filter(|r| r.g("game").str() == Some(gid)).collect() };
    let mut l: Vec<String> = vec![
        "# Bilan de tes parties".into(),
        String::new(),
        format!("{} parties ; rejeu : `train_games.py --frozen --coach` ; décisions : {}.", meta.len(), rows.len()),
        String::new(),
        "| Partie | Toi contre | Résultat | Rejeu fidèle | Moteur | Même coup que le coach |".into(),
        "|---|---|---|---|---|---|".into(),
    ];
    let (mut wins, mut losses) = (0i64, 0i64);
    let mut stopped: Vec<Json> = vec![];
    for (gid, g) in &meta {
        let rs = by(gid);
        let n: Vec<Json> = match g.g("names") {
            x if x.truthy() => x.arr().to_vec(),
            _ => vec![Json::from("Akali"), Json::from("LeBlanc")],
        };
        let res = g.g("result");
        let rtxt = if res.truthy() {
            wins += (res.g("winner").int() == Some(0)) as i64;
            losses += (res.g("winner").int() == Some(1)) as i64;
            format!(
                "{}{}",
                match res.g("winner").int() {
                    Some(0) => "victoire ",
                    Some(1) => "défaite ",
                    _ => "nulle ",
                },
                pts_text(res.g("pts"))
            )
        } else {
            let pts = if !rs.is_empty() { rs[rs.len() - 1].g("state").g("pts").clone() } else { g.g("pts").clone() };
            stopped.push(pts.clone());
            format!("non finie ({})", if pts.truthy() { pts_text(&pts) } else { "?".into() })
        };
        let co: Vec<&&Json> = rs.iter().filter(|r| r.g("coach").truthy()).collect();
        let same = co.iter().filter(|r| r.g("coach") == r.g("label")).count();
        let fid = if !rs.is_empty() && rs.iter().all(|r| r.g("faithful").truthy()) { "oui" } else if !rs.is_empty() { "NON" } else { "?" };
        l.push(format!(
            "| {} | {} (toi : {}) | {} | {} | {} | {}/{} |",
            gid,
            if n.len() > 1 { py_str_json(&n[1]) } else { "?".into() },
            py_str_json(&n[0]),
            rtxt,
            fid,
            if !rs.is_empty() { rs[0].get("engine").map(py_str_json).unwrap_or_else(|| "?".into()) } else { "?".into() },
            same,
            co.len()
        ));
    }
    let fin = wins + losses;
    l.push(String::new());
    l.push("## Résultats".into());
    l.push(format!("- Parties finies : {} ({} victoires, {} défaites).", fin, wins, losses));
    if fin > 0 {
        let (finf, z) = (fin as f64, 1.96f64);
        let p = wins as f64 / finf; // intervalle de Wilson à 95 % (juste aussi à 0 % ou 100 %)
        let c = (p + z * z / (2.0 * finf)) / (1.0 + z * z / finf);
        let h = z * (p * (1.0 - p) / finf + z * z / (4.0 * finf * finf)).sqrt() / (1.0 + z * z / finf);
        l.push(format!(
            "- Taux de victoire sur les parties finies : {:.0} %, intervalle de Wilson à 95 % : [{:.0} %, {:.0} %].",
            100.0 * p,
            100.0 * (c - h).max(0.0),
            100.0 * (c + h).min(1.0)
        ));
    }
    if !stopped.is_empty() {
        let behind = stopped
            .iter()
            .filter(|s| s.truthy() && s.arr()[0].num().unwrap_or(0.0) < s.arr()[1].num().unwrap_or(0.0))
            .count();
        l.push(format!(
            "- Parties inachevées : {}, dont {} arrêtées en étant mené au score : les compter comme des défaites donnerait au plus {}/{} victoires. **Biais de sélection possible** : ne pas lire le taux ci-dessus comme ton niveau réel.",
            stopped.len(), behind, wins, fin + stopped.len() as i64
        ));
    }
    // M : (type de ton coup, type du coach) -> nombre, dans l'ordre d'apparition
    let mut m: Vec<((&'static str, &'static str), i64)> = vec![];
    for r in &rows {
        if !r.g("coach").is_null() && r.get("coach").is_some() && r.g("k").str() == Some("act") {
            let key = (kind_of(r.g("label").str().unwrap_or("")), kind_of(r.g("coach").str().unwrap_or("")));
            match m.iter_mut().find(|e| e.0 == key) {
                Some(e) => e.1 += 1,
                None => m.push((key, 1)),
            }
        }
    }
    if !m.is_empty() {
        let (mut you, mut coach): (std::collections::BTreeMap<&str, i64>, std::collections::BTreeMap<&str, i64>) = Default::default();
        for ((a, b), n) in &m {
            *you.entry(a).or_insert(0) += n;
            *coach.entry(b).or_insert(0) += n;
        }
        let mut keys: Vec<&str> = you.keys().chain(coach.keys()).copied().collect();
        keys.sort();
        keys.dedup();
        keys.sort_by_key(|k| -you.get(k).copied().unwrap_or(0));
        l.push(String::new());
        l.push("## Tes décisions comparées au coach (IA Akali)".into());
        l.push("| Type | Toi | Coach |".into());
        l.push("|---|---|---|".into());
        for k in keys {
            l.push(format!("| {} | {} | {} |", k, you.get(k).copied().unwrap_or(0), coach.get(k).copied().unwrap_or(0)));
        }
        l.push(String::new());
        l.push("Les écarts les plus fréquents (toi → coach) :".into());
        let mut ms = m.clone();
        ms.sort_by_key(|e| -e.1);
        for ((a, b), n) in ms {
            if a != b && n >= 5 {
                l.push(format!("- {a} → {b} : {n} fois"));
            }
        }
    }
    l.push(String::new());
    l.push("## Rythme".into());
    l.push("| Partie | 1er Akali, Deadly Weapon (tour) | 1re entrée sur un battlefield (tour) | cartes cachées | score au tour 7 | au tour 11 |".into());
    l.push("|---|---|---|---|---|---|".into());
    for (gid, _) in &meta {
        let rs = by(gid);
        let t = |r: &Json| py_str_json(r.g("state").g("t"));
        let dw = rs.iter().find(|r| r.g("label").str().unwrap_or("").starts_with("joue Akali, Deadly Weapon")).map(|r| t(r)).unwrap_or_else(|| "—".into());
        let mv = rs
            .iter()
            .find(|r| {
                let lab = r.g("label").str().unwrap_or("");
                lab.starts_with("déplace") && !lab.split("vers").last().unwrap_or("").contains("base")
            })
            .map(|r| t(r))
            .unwrap_or_else(|| "—".into());
        let hide = rs.iter().filter(|r| r.g("label").str().unwrap_or("").starts_with("cache")).count();
        let sc = |turn: i64| {
            rs.iter()
                .find(|r| r.g("state").g("t").int().unwrap_or(0) >= turn)
                .map(|r| pts_text(r.g("state").g("pts")))
                .unwrap_or_else(|| "—".into())
        };
        l.push(format!("| {} | {} | {} | {} | {} | {} |", gid, dw, mv, hide, sc(7), sc(11)));
    }
    let txt = l.join("\n") + "\n";
    std::fs::write(&out_f, &txt).expect("écriture de BILAN.md");
    println!("{txt}");
    0
}
