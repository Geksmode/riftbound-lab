//! Textes et photos d'état de la table et des replays (`replay.py`) : traduction du journal du moteur (français ou
//! anglais), description des coups, photo de l'état (`snap`), base de cartes envoyée à la page (`card_db`).
//! L'enregistreur de replays (RecGame, RecAgent, record) dépend de l'IA et n'est pas porté ici.
use std::sync::{Mutex, OnceLock};

use crate::cards_helpers::play_cost_parts;
use crate::game::{Game, O};
use crate::jo;
use crate::json::Json;
use crate::lang::{en, tr};
use crate::regex::{Captures, Regex};
use crate::spec::{equip_bonus, specs};
use crate::value::{Loc, Map, Value};

static NAMES: Mutex<Option<[String; 2]>> = Mutex::new(None);

/// `replay.NAMES` : noms des deux joueurs (la table les pose à chaque partie).
pub fn names() -> [String; 2] {
    NAMES.lock().unwrap().clone().unwrap_or_else(|| ["Akali".into(), "LeBlanc".into()])
}

pub fn set_names(n: [String; 2]) {
    *NAMES.lock().unwrap() = Some(n);
}

fn name(i: &str) -> String {
    names()[i.parse::<usize>().unwrap()].clone()
}

fn uid_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"#(\d+)"))
}

/// `short(x)` : le texte sans les identifiants internes (« Akali#12 » → « Akali »).
pub fn short(x: &str) -> String {
    uid_re().sub(x, |_| String::new())
}

/// `[str(i) for i in g.bf_locs]` contient m : le nom du battlefield.
fn bf_idx(g: &Game, m: &str) -> Option<usize> {
    (0..g.bfs.len()).find(|i| i.to_string() == m)
}

type Rule = (Regex, fn(&Game, &Captures) -> String);

fn rules(src: &[(&str, fn(&Game, &Captures) -> String)]) -> Vec<Rule> {
    src.iter().map(|(p, f)| (Regex::new(p), *f)).collect()
}

fn bf_fr(g: &Game, m: &str) -> String {
    match bf_idx(g, m) {
        Some(i) => g.bfs[i].name.clone(),
        None if m == "base" => "la base".into(),
        None => m.into(),
    }
}

fn bf_en(g: &Game, m: &str) -> String {
    match bf_idx(g, m) {
        Some(i) => g.bfs[i].name.clone(),
        None => m.into(),
    }
}

fn from_fr(w: &str) -> &'static str {
    match w {
        "hand" => "",
        "champ" => " (zone champion)",
        "facedown" => " (carte cachée révélée)",
        _ => " depuis la défausse",
    }
}

fn from_en(w: &str) -> &'static str {
    match w {
        "hand" => "",
        "champ" => " (champion zone)",
        "facedown" => " (hidden card revealed)",
        _ => " from the trash",
    }
}

fn fr_rules() -> &'static Vec<Rule> {
    static R: OnceLock<Vec<Rule>> = OnceLock::new();
    R.get_or_init(|| {
        rules(&[
            (r"^P(\d) plays (.+?) from (hand|champ|facedown|trash) ?(.*)$", |g, m| {
                format!("{} joue {}{}{}", name(&m.s(1)), m.s(2), from_fr(&m.s(3)), fmt_args(g, &m.s(4)))
            }),
            (r"^P(\d) activates (.+?) ?(\(.*\))?$", |g, m| {
                format!("{} active {}{}", name(&m.s(1)), m.s(2), fmt_args(g, &m.get(3).unwrap_or_default()))
            }),
            (r"^P(\d) moves \[(.*)\] -> (.+)$", |g, m| {
                format!("{} déplace {} vers {}", name(&m.s(1)), m.s(2), bf_fr(g, &m.s(3)))
            }),
            (r"^P(\d) hides a card at (.+)$", |_, m| format!("{} cache une carte à {}", name(&m.s(1)), m.s(2))),
            (r"^P(\d) \+1 point \((.+)\) -> (\d+)$", |_, m| {
                format!("{} +1 point ({}) → {}", name(&m.s(1)), pt_why(&m.s(2)), m.s(3))
            }),
            (r"^P(\d) pays Deflect \+(\d+) power$", |_, m| {
                format!("{} paie Deflect : +{} puissance de n'importe quel domaine (règle 809)", name(&m.s(1)), m.s(2))
            }),
            (r"^P(\d) reveals their hand \[(.*)\]$", |_, m| {
                let h = m.s(2);
                format!("{} révèle sa main : {}", name(&m.s(1)), if h.is_empty() { "vide".into() } else { h })
            }),
            (r"^resolve (.+)$", |_, m| format!("Résolution : {}", m.s(1))),
            (r"^(.+) takes (\d+) \((\d+) might\)$", |_, m| {
                format!("{} subit {} dégât(s) (might {})", m.s(1), m.s(2), m.s(3))
            }),
            (r"^(.+) moves (\w+)->(\w+)$", |g, m| {
                format!("{} va de {} à {}", m.s(1), bf_fr(g, &m.s(2)), bf_fr(g, &m.s(3)))
            }),
            (r"^(.+) is stunned$", |_, m| format!("{} est étourdi(e)", m.s(1))),
            (r"^(.+) dies$", |_, m| format!("{} meurt", m.s(1))),
            (r"^(.+) is countered$", |_, m| format!("{} est contré(e)", m.s(1))),
            (r"^(.+) attached to (.+)$", |_, m| format!("{} est attaché(e) à {}", m.s(1), m.s(2))),
            (r"^trigger on chain: (.+)$", |_, m| format!("Déclenchement sur la chaîne : {}", m.s(1))),
            (r"^hidden (.+) at (.+) is trashed$", |_, m| {
                format!("La carte cachée {} à {} part à la défausse", m.s(1), m.s(2))
            }),
            (r"^combat damage: attackers (\d+) vs defenders (\d+)$", |_, m| {
                format!("Dégâts de combat : attaquants {} contre défenseurs {}", m.s(1), m.s(2))
            }),
            (r"^Zhonya's Hourglass saves (.+)$", |_, m| format!("Zhonya's Hourglass sauve {}", m.s(1))),
            (r"^Reflection of (.+) at (.+)$", |g, m| format!("Reflet de {} à {}", m.s(1), bf_fr(g, &m.s(2)))),
            (r"^P(\d) takes control of (.+)$", |_, m| format!("{} prend le contrôle de {}", name(&m.s(1)), m.s(2))),
            (r"^P(\d) draws instead of the final point$", |_, m| {
                format!("{} pioche au lieu du dernier point", name(&m.s(1)))
            }),
            (r"^P(\d) loses control of (.+)$", |_, m| format!("{} perd le contrôle de {}", name(&m.s(1)), m.s(2))),
            (r"^P(\d) draws (.+)$", |_, m| format!("{} pioche {}", name(&m.s(1)), m.s(2))),
            (r"^Baited Hook finds (.+)$", |_, m| format!("Baited Hook trouve {}", m.s(1))),
            (r"^COMBAT at (.+), attacker P(\d)$", |_, m| format!("COMBAT à {}, attaquant {}", m.s(1), name(&m.s(2)))),
            (r"^showdown at (.+), attacker P(\d)$", |_, m| {
                format!("Showdown à {}, initiative {}", m.s(1), name(&m.s(2)))
            }),
            (r"^showdown becomes a combat$", |_, _| "Le showdown devient un combat".into()),
            (r"^WINNER (\d)$", |_, m| format!("VICTOIRE de {}", name(&m.s(1)))),
            (r"^BURN OUT (\d)$", |_, m| format!("{} est à court de deck (Burn Out)", name(&m.s(1)))),
            (r"^Threshold of the Gray: both players add 1 energy$", |_, _| {
                "Threshold of the Gray : chaque joueur ajoute 1 énergie".into()
            }),
            (r"^===== turn (\d+): P(\d) .*points (\d+)-(\d+)$", |_, m| {
                format!("Tour {}, à {} de jouer (score {}-{})", m.s(1), name(&m.s(2)), m.s(3), m.s(4))
            }),
        ])
    })
}

fn en_rules() -> &'static Vec<Rule> {
    static R: OnceLock<Vec<Rule>> = OnceLock::new();
    R.get_or_init(|| {
        rules(&[
            (r"^P(\d) plays (.+?) from (hand|champ|facedown|trash) ?(.*)$", |g, m| {
                format!("{} plays {}{}{}", name(&m.s(1)), m.s(2), from_en(&m.s(3)), fmt_args(g, &m.s(4)))
            }),
            (r"^P(\d) activates (.+?) ?(\(.*\))?$", |g, m| {
                format!("{} activates {}{}", name(&m.s(1)), m.s(2), fmt_args(g, &m.get(3).unwrap_or_default()))
            }),
            (r"^P(\d) moves \[(.*)\] -> (.+)$", |g, m| {
                format!("{} moves {} to {}", name(&m.s(1)), m.s(2), bf_en(g, &m.s(3)))
            }),
            (r"^P(\d) \+1 point \((.+)\) -> (\d+)$", |_, m| format!("{} +1 point ({}) → {}", name(&m.s(1)), m.s(2), m.s(3))),
            (r"^P(\d) pays Deflect \+(\d+) power$", |_, m| {
                format!("{} pays Deflect: +{} power of any domain (rule 809)", name(&m.s(1)), m.s(2))
            }),
            (r"^P(\d) reveals their hand \[(.*)\]$", |_, m| {
                let h = m.s(2);
                format!("{} reveals their hand: {}", name(&m.s(1)), if h.is_empty() { "empty".into() } else { h })
            }),
            (r"^resolve (.+)$", |_, m| format!("Resolve: {}", m.s(1))),
            (r"^(.+) takes (\d+) \((\d+) might\)$", |_, m| format!("{} takes {} damage (might {})", m.s(1), m.s(2), m.s(3))),
            (r"^(.+) moves (\w+)->(\w+)$", |g, m| {
                format!("{} moves from {} to {}", m.s(1), bf_en(g, &m.s(2)), bf_en(g, &m.s(3)))
            }),
            (r"^trigger on chain: (.+)$", |_, m| format!("Trigger on the chain: {}", m.s(1))),
            (r"^hidden (.+) at (.+) is trashed$", |_, m| format!("The hidden card {} at {} goes to the trash", m.s(1), m.s(2))),
            (r"^combat damage: attackers (\d+) vs defenders (\d+)$", |_, m| {
                format!("Combat damage: attackers {} vs defenders {}", m.s(1), m.s(2))
            }),
            (r"^Reflection of (.+) at (.+)$", |g, m| format!("Reflection of {} at {}", m.s(1), bf_en(g, &m.s(2)))),
            (r"^COMBAT at (.+), attacker P(\d)$", |_, m| format!("COMBAT at {}, attacker {}", m.s(1), name(&m.s(2)))),
            (r"^showdown at (.+), attacker P(\d)$", |_, m| format!("Showdown at {}, initiative {}", m.s(1), name(&m.s(2)))),
            (r"^showdown becomes a combat$", |_, _| "The showdown becomes a combat".into()),
            (r"^WINNER (\d)$", |_, m| format!("{} WINS", name(&m.s(1)))),
            (r"^BURN OUT (\d)$", |_, m| format!("{} runs out of deck (Burn Out)", name(&m.s(1)))),
            (r"^===== turn (\d+): P(\d) .*points (\d+)-(\d+)$", |_, m| {
                format!("Turn {}, {} to play (score {}-{})", m.s(1), name(&m.s(2)), m.s(3), m.s(4))
            }),
        ])
    })
}

/// `fr_line(g, line)` : traduction française d'une ligne du journal du moteur.
pub fn fr_line(g: &Game, line: &str) -> String {
    let s = line.trim();
    for (re, f) in fr_rules() {
        if let Some(m) = re.match_(s) {
            return short(&f(g, &m));
        }
    }
    short(s)
}

/// `en_line(g, line)` : ligne du journal pour la version anglaise : le texte du moteur, avec les noms des joueurs,
/// les noms des battlefields et sans les identifiants internes.
pub fn en_line(g: &Game, line: &str) -> String {
    let s = line.trim();
    for (re, f) in en_rules() {
        if let Some(m) = re.match_(s) {
            return short(&f(g, &m));
        }
    }
    static P: OnceLock<Regex> = OnceLock::new();
    let p = P.get_or_init(|| Regex::new(r"\bP([01])\b"));
    short(&p.sub(s, |m| name(&m.s(1))))
}

/// `log_line(g, line)` : ligne du journal de la table dans sa langue.
pub fn log_line(g: &Game, line: &str) -> String {
    if en() {
        en_line(g, line)
    } else {
        fr_line(g, line)
    }
}

/// `pt_why(w)`
pub fn pt_why(w: &str) -> String {
    static P: OnceLock<Regex> = OnceLock::new();
    let p = P.get_or_init(|| Regex::new(r"(conquer|hold) (.+)"));
    if let Some(m) = p.match_(w) {
        return format!("{}{}", if m.s(1) == "conquer" { "conquête de " } else { "tenue de " }, m.s(2));
    }
    w.to_string()
}

/// `fmt_args(g, a)` : arguments d'un coup du journal (« (tg=#5, loc=1) ») en clair.
pub fn fmt_args(g: &Game, a: &str) -> String {
    let a = a.trim().trim_matches(|c| c == '(' || c == ')');
    if a.is_empty() {
        return String::new();
    }
    static P: OnceLock<Regex> = OnceLock::new();
    static Q: OnceLock<Regex> = OnceLock::new();
    let p = P.get_or_init(|| Regex::new(r"(\w+)=(.*?)(?= \w+=|$)"));
    let q = Q.get_or_init(|| Regex::new(r"(#\d+),"));
    let mut parts = vec![];
    for m in p.find_iter(a) {
        let (k, v) = (m.s(1), m.s(2));
        match k.as_str() {
            "tg" | "tg2" => {
                let et = tr(" et ", " and ");
                let v = q.sub(v.trim_matches(','), |m| format!("{}{}", m.s(1), et));
                if !v.is_empty() {
                    parts.push(tr("cible ", "target ") + &v);
                }
            }
            "loc" => parts.push(
                tr("à ", "at ")
                    + &match bf_idx(g, &v) {
                        Some(i) => g.bfs[i].name.clone(),
                        None => tr("la base", "base"),
                    },
            ),
            "dest" => parts.push(
                tr("vers ", "to ")
                    + &match bf_idx(g, &v) {
                        Some(i) => g.bfs[i].name.clone(),
                        None => tr("la base", "base"),
                    },
            ),
            "mover" | "mover_oid" | "kill" | "xp" => continue,
            _ => parts.push(format!("{k}={v}")),
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" — {}", parts.join(", "))
    }
}

/// Battlefield désigné par une valeur (`is_bf(v)` : un indice entier ; `Loc::Bf`).
pub fn bf_of(v: &Value) -> Option<usize> {
    match v {
        Value::Loc(Loc::Bf(i)) => Some(*i),
        Value::Int(i) => Some(*i as usize),
        _ => None,
    }
}

fn uid_of(v: &Value) -> Option<u32> {
    match v {
        Value::Int(i) => Some(*i as u32),
        _ => None,
    }
}

/// `describe(g, a, pid, viewer)` : texte d'une action de haut niveau, avant qu'elle soit appliquée.
pub fn describe(g: &mut Game, a: &Value, pid: usize, viewer: Option<usize>) -> String {
    let l = a.list();
    let k = l[0].str();
    match k {
        "pass" => tr("passe", "passes"),
        "end" => tr("termine son tour", "ends their turn"),
        "play" => {
            let uid = uid_of(&l[1]).unwrap_or(0);
            let ch = l[3].map().clone();
            let c = find(g, pid, uid);
            let n = c.map(|c| g.cname(c).to_string()).unwrap_or_else(|| "?".into());
            let mut extra: Vec<String> = vec![];
            for (kk, v) in &ch.0 {
                let kk: &str = kk;
                if (kk == "tg" || kk == "tg2") && v.truthy() {
                    let names: Vec<String> =
                        v.list().iter().map(|u| tname(g, u, pid, viewer)).collect();
                    extra.push(tr("cible ", "target ") + &names.join(&tr(" et ", " and ")));
                } else if kk == "loc" {
                    extra.push(
                        tr("à ", "at ")
                            + &match bf_of(v) {
                                Some(i) => g.bfs[i].name.clone(),
                                None => tr("la base", "base"),
                            },
                    );
                } else if kk == "dest" && !v.is_none() {
                    extra.push(
                        tr("vers ", "to ")
                            + &match bf_of(v) {
                                Some(i) => g.bfs[i].name.clone(),
                                None => tr("la base", "base"),
                            },
                    );
                } else if (kk == "accelerate" || kk == "repeat") && v.truthy() {
                    extra.push(kk.to_string());
                }
            }
            let mut ex = play_cost_parts(g, &ch); // avant la cible (la table coupe les libellés à « cible »)
            ex.extend(extra);
            let mut extra = ex;
            if !ch.g("show").is_none() {
                // [Show Off] (832) : l'objet montré, avant la cible aussi
                let sh = match ch.get("shown") {
                    Some(v) if v.truthy() => v.clone(),
                    _ => crate::actions::shown_info(g, pid, uid_of(&ch.g("show")).unwrap_or(0)).unwrap_or(Value::None),
                };
                if !sh.is_none() {
                    let nm = sh.map().g("name");
                    let nm = nm.opt_str().unwrap_or("");
                    extra.insert(0, tr(format!("Show Off : {nm}"), format!("Show Off: {nm}")));
                }
            }
            if let Some(c) = c {
                let nd = crate::actions::deflect_total(g, pid, c, &ch).map(|v| v.len()).unwrap_or(0);
                if nd > 0 {
                    extra.push(tr(format!("Deflect +{nd} puissance"), format!("Deflect +{nd} power")));
                }
            }
            tr(format!("joue {n}"), format!("plays {n}"))
                + &if extra.is_empty() { String::new() } else { format!(" ({})", extra.join(", ")) }
        }
        "hide" => {
            let c = find(g, pid, uid_of(&l[1]).unwrap_or(0));
            let cn = c.map(|c| g.cname(c).to_string()).unwrap_or_else(|| "?".into());
            let b = g.bfs[bf_of(&l[2]).unwrap()].name.clone();
            tr(format!("cache {cn} à {b}"), format!("hides {cn} at {b}"))
        }
        "act" => {
            let src = &l[1];
            let n = match src {
                Value::Tuple(t) | Value::List(t) => g.p[t[1].int() as usize].legend_name.clone(),
                v => match uid_of(v).and_then(|u| g.obj(u)) {
                    Some(o) => g.cname(o).to_string(),
                    None => "?".into(),
                },
            };
            tr(format!("active {n}"), format!("activates {n}"))
        }
        "move" => {
            let us: Vec<String> = l[1]
                .list()
                .iter()
                .map(|u| match uid_of(u).and_then(|x| g.obj(x)) {
                    Some(o) => g.label(o),
                    None => crate::actions::py_str(g, u),
                })
                .collect();
            let us = us.join(", ");
            let dest = match bf_of(&l[2]) {
                Some(i) => g.bfs[i].name.clone(),
                None => tr("la base", "base"),
            };
            tr(format!("déplace {us} vers "), format!("moves {us} to ")) + &dest
        }
        _ => crate::actions::py_str(g, a),
    }
}

/// `_tname(g, u, pid, viewer)` : cible d'un coup de pid, vue par viewer (pid par défaut).
pub fn tname(g: &Game, u: &Value, pid: usize, viewer: Option<usize>) -> String {
    let Some(o) = uid_of(u).and_then(|x| g.obj(x)) else { return crate::actions::py_str(g, u) };
    let viewer = viewer.unwrap_or(pid);
    let ctrl = g.o(o).ctrl;
    if ctrl == pid {
        return g.label(o);
    }
    g.label(o)
        + &if viewer == pid {
            tr(" adverse", " (enemy)")
        } else if ctrl == viewer {
            tr(" (à toi)", " (yours)")
        } else {
            String::new()
        }
}

/// `_find(g, pid, uid)` : carte du joueur (main, zone champion, défausse, cartes cachées) ou objet en jeu.
pub fn find(g: &Game, pid: usize, uid: u32) -> Option<O> {
    let pl = &g.p[pid];
    let fds = g.bfs.iter().flat_map(|b| b.facedowns.iter());
    for &c in pl.hand.iter().chain(pl.champ.iter()).chain(pl.trash.iter()).chain(fds) {
        if g.o(c).uid == uid {
            return Some(c);
        }
    }
    g.obj(uid)
}

// ---------------------------------------------------------------------------------- photo de l'état
fn unit(g: &mut Game, o: O) -> Json {
    let m = g.might(o).unwrap_or(0);
    let ob = g.o(o).clone();
    let cn = g.cname(o).to_string();
    let mut d = jo! {
        "u" => ob.uid, "n" => cn.clone(), "c" => ob.ctrl, "m" => m,
        "b" => specs().get(&cn).and_then(|s| s.might).map(|x| x as i64),
    };
    if ob.damage != 0 {
        d.set("d", ob.damage);
    }
    if ob.exh {
        d.set("x", 1);
    }
    if ob.stunned {
        d.set("s", 1);
    }
    if ob.buff != 0 {
        d.set("bf", ob.buff);
    }
    if ob.empowered {
        d.set("e", 1);
    }
    if ob.token {
        d.set("t", 1);
    }
    let dn = g.dup_no(Some(o));
    if dn > 0 {
        d.set("dn", dn); // homonymes en jeu : « #2 » sur la carte et dans les choix
    }
    if cn != ob.name {
        d.set("cp", 1); // copie d'une autre unité (Reflection, Shady Spectacles)
    }
    if !ob.attached.is_empty() {
        let gs: Vec<Json> =
            ob.attached.iter().filter_map(|&x| g.obj(x)).map(|x| Json::from(g.cname(x))).collect();
        d.set("g", Json::Arr(gs));
    }
    let mut kws = vec![];
    for kw in ["Temporary", "Tank", "Backline", "Ganking", "Deflect", "Assault", "Shield"] {
        if g.has_kw(o, kw).unwrap_or(false) {
            kws.push(Json::from(kw));
        }
    }
    if !kws.is_empty() {
        d.set("k", Json::Arr(kws));
    }
    d
}

/// `snap(g)` : photo de l'état pour la page.
pub fn snap(g: &mut Game) -> Json {
    let pts: Vec<i64> = g.p.iter().map(|p| p.points).collect();
    let mut st = jo! { "t" => g.turn_no, "tp" => g.tp, "pts" => pts, "v" => g.victory };
    let nb = g.bfs.len();
    let mut base: [Vec<Json>; 2] = [vec![], vec![]];
    let mut at_bf: Vec<Vec<Json>> = vec![vec![]; nb];
    let mut loose_gear: [Vec<Json>; 2] = [vec![], vec![]];
    let mut bf_gear: Vec<Vec<Json>> = vec![vec![]; nb]; // équipement sur un battlefield ([Deploy], 830)
    for o in g.board.clone() {
        if g.spec(o).type_ == "Unit" {
            let (loc, ctrl) = (g.o(o).loc, g.o(o).ctrl);
            match loc {
                Some(Loc::Base) => {
                    let u = unit(g, o);
                    base[ctrl].push(u)
                }
                Some(Loc::Bf(i)) => {
                    let u = unit(g, o);
                    at_bf[i].push(u)
                }
                None => {}
            }
        } else if g.o(o).attached_to.is_none() {
            let ob = g.o(o);
            let mut gd = jo! { "u" => ob.uid, "n" => g.cname(o), "x" => if ob.exh { 1 } else { 0 } };
            match ob.loc {
                Some(Loc::Bf(i)) if i < nb => {
                    gd.set("c", ob.ctrl);
                    bf_gear[i].push(gd);
                }
                _ => loose_gear[ob.ctrl].push(gd),
            }
        }
    }
    let mut pl = vec![];
    for (i, p) in g.p.iter().enumerate() {
        let leg = g.o(p.legend);
        let mut pp: Vec<(String, i64)> = p.pool_p.0.iter().filter(|(_, n)| *n != 0).cloned().collect();
        pp.sort_by(|a, b| a.0.cmp(&b.0));
        pl.push(jo! {
            "hand" => Json::Arr(p.hand.iter().map(|&c| Json::Arr(vec![g.o(c).uid.into(), g.cname(c).into()])).collect()),
            "champ" => Json::Arr(p.champ.iter().map(|&c| Json::from(g.cname(c))).collect()),
            "deck" => p.deck.len(),
            "trash" => Json::Arr(p.trash.iter().map(|&c| Json::from(g.cname(c))).collect()),
            "ban" => Json::Arr(p.banish.iter().map(|&c| Json::from(g.cname(c))).collect()), // Banishment : zone publique (108.6.e)
            "runes" => Json::Arr(p.runes.iter().map(|&r| {
                let ru = g.r(r);
                let d: String = ru.domain.chars().take(2).collect();
                Json::from(d + if ru.exhausted { "x" } else { "" })
            }).collect()),
            "rdeck" => p.rune_deck.len(),
            "leg" => jo! { "n" => p.legend_name.clone(), "x" => if leg.exh { 1 } else { 0 }, "e" => if leg.empowered { 1 } else { 0 } },
            "base" => Json::Arr(std::mem::take(&mut base[i])),
            "gear" => Json::Arr(std::mem::take(&mut loose_gear[i])),
            "xp" => p.xp, "pe" => p.pool_e,
            "pp" => Json::Obj(pp.into_iter().map(|(k, v)| (k, Json::Int(v))).collect()), // Rune Pool : flottante
        });
    }
    st.set("p", Json::Arr(pl));
    // fd : la première carte cachée (anciens replays) ; fds : toutes, une par carte (Bandle Tree en autorise deux)
    let mut bfs = vec![];
    for (i, b) in g.bfs.iter().enumerate() {
        let fd = b.facedown().map(|c| Json::Arr(vec![g.o(c).owner.into(), g.cname(c).into()]));
        bfs.push(jo! {
            "n" => b.name.clone(), "c" => b.ctrl, "fd" => fd,
            "fds" => Json::Arr(b.facedowns.iter().map(|&c| Json::Arr(vec![g.o(c).owner.into(), g.cname(c).into()])).collect()),
            "u" => Json::Arr(std::mem::take(&mut at_bf[i])), "g" => Json::Arr(std::mem::take(&mut bf_gear[i])),
        });
    }
    st.set("bfs", Json::Arr(bfs));
    let chain: Vec<Json> = g
        .chain
        .iter()
        .map(|&it| {
            let it = g.it(it);
            jo! { "n" => short(&it.name), "c" => it.ctrl, "k" => it.kind }
        })
        .collect();
    st.set("chain", Json::Arr(chain));
    if let Some(sd) = &g.sd {
        st.set("sd", jo! { "bf" => sd.bf, "combat" => sd.combat, "a" => sd.attacker });
    }
    st
}

/// `card_db(names)` : textes et chiffres des cartes pour la page.
pub fn card_db<'a>(names: impl IntoIterator<Item = &'a str>) -> Json {
    let mut db = jo! {};
    for n in names {
        let Some(sp) = specs().get(n) else { continue };
        let mut tags: Vec<&String> = sp.tags.iter().collect();
        tags.sort();
        let mut d = jo! {
            "t" => sp.type_.clone(), "s" => sp.super_.clone(),
            "d" => Json::Arr(sp.domains.iter().map(Json::from).collect()),
            "e" => sp.e as i64, "p" => sp.p as i64, "m" => sp.might.map(|x| x as i64), "tx" => sp.text.clone(),
            "tg" => Json::Arr(tags.into_iter().map(Json::from).collect()),
        };
        let eb = equip_bonus(n);
        if eb != 0 {
            d.set("eb", eb as i64);
        }
        db.set(n, d);
    }
    db
}

#[allow(dead_code)]
fn _unused(_: Map) {}
