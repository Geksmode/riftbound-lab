//! `compare.py` : compare deux résultats de simulation en appliquant les règles du projet, ou en regroupe plusieurs.
//!
//! ```text
//!   compare A.json:SEL B.json:SEL           écart A - B (± écart-type), apparié si mêmes graines
//!   compare --pool A.json:SEL B.json:SEL... regroupe des blocs de la MÊME configuration
//! ```
//! SEL = index dans la liste des résultats, ou un morceau de « label » (premier résultat qui le contient). Les
//! fichiers de session du manager (`{"engine_md5", "results": [...]}`) sont acceptés. Refus (code 3) si :
//! - un résultat n'a pas de version du moteur (engine_md5) : le remesurer ;
//! - les versions du moteur diffèrent, en particulier **moteur Rust contre moteur Python** (`-rust`, `version.rs`) ;
//! - les graines se recouvrent en partie (ni appariées, ni indépendantes) ;
//! - `--pool` sur des blocs qui partagent des graines (ce ne sont pas des parties indépendantes).
//! Verdict « on ne sait pas » sous 2 écarts-types.
use super::py::{self, PyVal};
use super::version::{is_rust, scores};

pub const REFUS: i32 = 3;

pub const DOC: &str = "Compare deux résultats de simulation en appliquant les règles du projet, ou en regroupe plusieurs.

  compare A.json:SEL B.json:SEL           écart A - B (± écart-type), apparié si mêmes graines
  compare --pool A.json:SEL B.json:SEL... regroupe des blocs de la MÊME configuration

SEL = index dans la liste des résultats, ou un morceau de « label » (premier résultat qui le contient). Les fichiers
de session du manager ({\"engine_md5\", \"results\": [...]}) sont acceptés. Refus (code 3) si :
- un résultat n'a pas de version du moteur (engine_md5) : le remesurer ;
- les versions du moteur diffèrent (dont moteur Rust « -rust » contre moteur Python) ;
- les graines se recouvrent en partie (ni appariées, ni indépendantes) ;
- --pool sur des blocs qui partagent des graines (ce ne sont pas des parties indépendantes).
Verdict « on ne sait pas » sous 2 écarts-types.";

/// Sortie anticipée : (code de sortie, message sur la sortie standard ou d'erreur).
pub enum Exit {
    /// `refus(msg)` : « REFUS : msg » sur la sortie standard, code 3
    Refus(String),
    /// `sys.exit(msg)` : msg sur la sortie d'erreur, code 1
    Erreur(String),
}

/// Un résultat chargé (`load`) : l'entrée et son nom `_nom`.
pub struct Entree {
    pub e: PyVal,
    pub nom: String,
}

/// `load(spec)`
pub fn load(spec: &str) -> Result<Entree, Exit> {
    let (path, sel) = match spec.rfind(':') {
        Some(i) => (&spec[..i], &spec[i + 1..]),
        None => (spec, "0"),
    };
    let d = py::load_file(std::path::Path::new(path)).map_err(Exit::Erreur)?;
    let top = match &d {
        PyVal::Dict(_) => d.get("engine_md5").cloned(),
        _ => None,
    };
    let items: Vec<PyVal> = match &d {
        PyVal::Dict(_) => d.get("results").map(|r| r.as_list().to_vec()).unwrap_or_default(),
        _ => d.as_list().to_vec(),
    };
    let digits = sel.trim_start_matches('-');
    let e = if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
        let i: i64 = sel.parse().map_err(|_| Exit::Erreur(format!("index illisible : {sel}")))?;
        let n = items.len() as i64;
        let j = if i < 0 { i + n } else { i };
        if j < 0 || j >= n {
            return Err(Exit::Erreur("IndexError: list index out of range".into()));
        }
        items[j as usize].clone()
    } else {
        let lab = |x: &PyVal| x.get("label").or(x.get("A")).map(|v| v.py_str()).unwrap_or_default();
        match items.iter().find(|x| lab(x).contains(sel)) {
            Some(e) => e.clone(),
            None => return Err(Exit::Erreur(format!("{path} : aucun résultat ne contient « {sel} »"))),
        }
    };
    let mut e = e;
    if let Some(t) = top.filter(|t| t.truthy()) {
        if !e.get("engine_md5").is_some_and(|v| v.truthy()) {
            e.set("engine_md5", t);
        }
    }
    let nom = match e.get("label").filter(|v| v.truthy()) {
        Some(l) => l.py_str(),
        None => e.get("A").map(|v| v.py_str()).unwrap_or_else(|| sel.to_string()),
    };
    Ok(Entree { nom: format!("{path}:{nom}"), e })
}

fn refus(msg: String) -> Exit {
    Exit::Refus(msg)
}

/// `check_version(es)`. En plus du Python : refus nommé quand on mélange moteur Rust et moteur Python.
pub fn check_version(es: &[Entree]) -> Result<String, Exit> {
    for e in es {
        if !e.e.get("engine_md5").is_some_and(|v| v.truthy()) {
            return Err(refus(format!(
                "{} n'a pas de version du moteur (engine_md5) : le remesurer avec le moteur actuel.",
                e.nom
            )));
        }
    }
    let v: Vec<String> = es.iter().map(|e| e.e.get("engine_md5").unwrap().py_str()).collect();
    let mut u = v.clone();
    u.sort();
    u.dedup();
    if u.len() > 1 {
        let detail = es.iter().zip(&v).map(|(e, x)| format!("{} = {x}", e.nom)).collect::<Vec<_>>().join(", ");
        if v.iter().any(|x| is_rust(x)) && v.iter().any(|x| !is_rust(x)) {
            return Err(refus(format!(
                "moteur Rust et moteur Python ({detail}) : le moteur Rust est une nouvelle version, ses chiffres ne \
                 se comparent ni ne se regroupent jamais avec ceux du moteur Python (remesurer les deux côtés avec le \
                 même moteur)."
            )));
        }
        return Err(refus(format!(
            "versions du moteur différentes ({detail}) : ne jamais agréger ni comparer deux versions (sauf équivalence \
             prouvée par manager/eqtest.py)."
        )));
    }
    Ok(u.pop().unwrap())
}

/// `mean_se(xs)`
pub fn mean_se(xs: &[f64]) -> (f64, f64) {
    let n = xs.len() as f64;
    let m = py::sum(xs.iter().copied()) / n;
    let sd = if xs.len() > 1 { (py::sum(xs.iter().map(|x| (x - m).powi(2))) / (n - 1.0)).sqrt() } else { 0.0 };
    (m, sd / n.sqrt())
}

/// `verdict(d, se)`
pub fn verdict(d: f64, se: f64) -> (f64, &'static str) {
    let z = if se != 0.0 {
        d / se
    } else if d != 0.0 {
        f64::INFINITY
    } else {
        0.0
    };
    let txt = if z.abs() >= 2.0 { "écart mesurable (≥ 2 écarts-types)" } else { "on ne sait pas (< 2 écarts-types)" };
    (z, txt)
}

/// `compare(a, b)` : lignes affichées, (écart, écart-type).
pub fn compare(a: &Entree, b: &Entree, out: &mut Vec<String>) -> Result<(f64, f64), Exit> {
    let ver = check_version(&[Entree { e: a.e.clone(), nom: a.nom.clone() }, Entree { e: b.e.clone(), nom: b.nom.clone() }])?;
    let (sa, sb) = (scores(&a.e), scores(&b.e));
    if sa.is_empty() || sb.is_empty() {
        return Err(refus("pas de résultat par graine (per_seed / per_pair) : impossible de vérifier les graines.".into()));
    }
    let mut ka: Vec<i64> = sa.iter().map(|x| x.0).collect();
    let mut kb: Vec<i64> = sb.iter().map(|x| x.0).collect();
    ka.sort();
    kb.sort();
    let common = ka.iter().filter(|k| kb.binary_search(k).is_ok()).count();
    let (d, se, kind);
    if ka == kb {
        let get = |s: &[(i64, f64)], k: i64| s.iter().find(|x| x.0 == k).unwrap().1;
        let diffs: Vec<f64> = ka.iter().map(|&k| get(&sa, k) - get(&sb, k)).collect();
        (d, se) = mean_se(&diffs);
        kind = format!("apparié sur {} graines communes", ka.len());
    } else if common == 0 {
        let (ma, ea) = mean_se(&sa.iter().map(|x| x.1).collect::<Vec<_>>());
        let (mb, eb) = mean_se(&sb.iter().map(|x| x.1).collect::<Vec<_>>());
        d = ma - mb;
        se = (ea.powi(2) + eb.powi(2)).sqrt();
        kind = format!("indépendant ({} et {} graines disjointes)", ka.len(), kb.len());
    } else {
        return Err(refus(format!(
            "graines en partie communes ({common} sur {} et {}) : ni apparié ni indépendant.",
            ka.len(),
            kb.len()
        )));
    }
    let (z, txt) = verdict(d, se);
    out.push(format!("moteur {ver} · {kind}"));
    out.push(format!("A = {}\nB = {}", a.nom, b.nom));
    out.push(format!("A - B = {:+.1} ± {:.1} points (z = {:.1}) : {txt}", 100.0 * d, 100.0 * se, z));
    Ok((d, se))
}

/// `pool(es)`
pub fn pool(es: &[Entree], out: &mut Vec<String>) -> Result<(f64, f64), Exit> {
    let ver = check_version(es)?;
    let mut seen: Vec<(i64, &str)> = vec![];
    for e in es {
        for (k, _) in scores(&e.e) {
            if let Some((_, n)) = seen.iter().find(|x| x.0 == k) {
                return Err(refus(format!(
                    "la graine {k} est dans {n} et {} : des blocs sur les mêmes graines ne sont pas indépendants.",
                    e.nom
                )));
            }
            seen.push((k, &e.nom));
        }
    }
    let xs: Vec<f64> = es.iter().flat_map(|e| scores(&e.e).into_iter().map(|x| x.1)).collect();
    let (m, se) = mean_se(&xs);
    out.push(format!("moteur {ver} · {} blocs, {} graines distinctes", es.len(), xs.len()));
    out.push(format!("regroupé = {:.1} % ± {:.1}", 100.0 * m, 100.0 * se));
    Ok((m, se))
}

/// Point d'entrée de la ligne de commande : renvoie le code de sortie.
pub fn main(args: &[String]) -> i32 {
    let mut out = vec![];
    let r = (|| -> Result<(), Exit> {
        if args.first().map(|s| s.as_str()) == Some("--pool") {
            let es = args[1..].iter().map(|x| load(x)).collect::<Result<Vec<_>, _>>()?;
            pool(&es, &mut out)?;
        } else if args.len() == 2 {
            let (a, b) = (load(&args[0])?, load(&args[1])?);
            compare(&a, &b, &mut out)?;
        } else {
            out.push(DOC.to_string());
            return Err(Exit::Erreur(String::new()));
        }
        Ok(())
    })();
    for l in &out {
        println!("{l}");
    }
    match r {
        Ok(()) => 0,
        Err(Exit::Refus(m)) => {
            println!("REFUS : {m}");
            REFUS
        }
        Err(Exit::Erreur(m)) if m.is_empty() => 2,
        Err(Exit::Erreur(m)) => {
            eprintln!("{m}");
            1
        }
    }
}
