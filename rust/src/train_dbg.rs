//! Préparation de positions pour les robots de vérification de la table (`riftbound/train/verif_*.mjs`). Ils
//! exécutaient du Python dans le module train (`exec(code)`) ; le moteur WebAssembly reçoit à la place une liste
//! d'opérations JSON, chacune `[nom, arguments...]`, qui modifient la partie en cours ou lisent une valeur. Les valeurs
//! nommées (`"as"` des opérations, variables des lectures) vont dans `W.extra` et se lisent avec `get`.
//!
//! Qui : "me" (ME) ou "op" (1 - ME). Emplacement : "base" ou l'indice du battlefield. Toutes les opérations font ce
//! que faisait le code Python des robots (mêmes appels du moteur : `Obj(...)`, `enter_board`, `make_token`...).
use std::collections::BTreeMap;

use crate::game::{Game, Rune, Zone, O, R};
use crate::json::Json;
use crate::value::{Loc, Map, Value};

fn who(j: &Json) -> usize {
    let me = crate::train::dbg_me();
    match j.str() {
        Some("op") => 1 - me,
        Some(_) | None => match j.int() {
            Some(p) => p as usize,
            None => me,
        },
    }
}

fn loc(j: &Json) -> Loc {
    match j {
        Json::Int(i) => Loc::Bf(*i as usize),
        _ => Loc::Base,
    }
}

fn new_rune(g: &mut Game, dom: &str, pid: usize) -> R {
    let uid = g.counters.next_obj();
    g.runes_arena.push(Rune { uid, domain: dom.to_string(), owner: pid, exhausted: false });
    R(g.runes_arena.len() as u32 - 1)
}

fn zone_name(z: Option<Zone>) -> Json {
    match z {
        Some(z) => z.name().into(),
        None => Json::Null,
    }
}

fn loc_json(l: Option<Loc>) -> Json {
    match l {
        None => Json::Null,
        Some(Loc::Base) => "base".into(),
        Some(Loc::Bf(i)) => i.into(),
    }
}

fn find_any(g: &Game, uid: u32) -> Option<O> {
    (0..g.objs.len()).map(|i| O(i as u32)).find(|&o| g.o(o).uid == uid && (g.o(o).zone.is_some() || g.on_board(o)))
}

fn uid(j: &Json) -> u32 {
    j.int().unwrap_or(-1) as u32
}

/// Une opération ; le résultat d'une lecture.
fn one(g: &mut Game, d: &mut Option<crate::game::Decision>, ex: &mut BTreeMap<String, Json>, level: Option<i64>, op: &[Json]) -> Result<Json, String> {
    let name = op.first().and_then(|x| x.str()).ok_or("opération sans nom")?;
    let a = |i: usize| op.get(i).cloned().unwrap_or(Json::Null);
    Ok(match name {
        // ---- préparation
        "runes" => {
            // g.p[pid].runes = [Rune(d, pid) for d in domaines] ; exh : liste de booléens (facultative)
            let pid = who(&a(1));
            let doms: Vec<String> = a(2).arr().iter().map(|x| x.str().unwrap_or("").to_string()).collect();
            let rs: Vec<R> = doms.iter().map(|x| new_rune(g, x, pid)).collect();
            for (k, &r) in rs.iter().enumerate() {
                if let Some(e) = a(3).arr().get(k) {
                    g.r_mut(r).exhausted = e.truthy();
                }
            }
            g.p[pid].runes = rs;
            Json::Null
        }
        "runes_exh" => {
            let pid = who(&a(1));
            for r in g.p[pid].runes.clone() {
                g.r_mut(r).exhausted = a(2).truthy();
            }
            Json::Null
        }
        "card" => {
            // c = Obj(n, pid) ; c.zone = zone ; ajoutée en main / défausse (fin) ou sur le deck (dessus)
            let pid = who(&a(1));
            let o = g.new_obj(a(2).str().unwrap_or(""), pid, false);
            match a(3).str().unwrap_or("hand") {
                "hand" => {
                    g.o_mut(o).zone = Some(Zone::Hand);
                    g.p[pid].hand.push(o);
                }
                "trash" => {
                    g.o_mut(o).zone = Some(Zone::Trash);
                    g.p[pid].trash.push(o);
                }
                "deck" => {
                    g.o_mut(o).zone = Some(Zone::Deck);
                    g.p[pid].deck.insert(0, o);
                }
                z => return Err(format!("zone inconnue {z}")),
            }
            g.o(o).uid.into()
        }
        "board" => {
            // g.enter_board(Obj(n, pid), pid, loc, ready=ready)
            let pid = who(&a(1));
            let o = g.new_obj(a(2).str().unwrap_or(""), pid, false);
            g.enter_board(o, pid, loc(&a(3)), a(4).truthy());
            g.o(o).uid.into()
        }
        "token" => {
            // make_token(g, n, pid, loc, ready=ready).uid
            let pid = who(&a(1));
            let o = crate::cards_helpers::make_token(g, a(2).str().unwrap_or(""), pid, loc(&a(3)), a(4).truthy())
                .map_err(|_| "GameOver")?;
            o.map(|o| Json::from(g.o(o).uid)).unwrap_or(Json::Null)
        }
        "facedown" => {
            // c = Obj(n, pid) ; c.zone, c.hidden_turn, c.hidden_bf = "facedown", g.turn_no - 1, bf
            let (b, pid) = (a(1).int().unwrap_or(0) as usize, who(&a(2)));
            let o = g.new_obj(a(3).str().unwrap_or(""), pid, false);
            let t = g.turn_no - 1;
            let ob = g.o_mut(o);
            ob.zone = Some(Zone::Facedown);
            ob.hidden_turn = Some(t);
            ob.hidden_bf = Some(Loc::Bf(b));
            g.bfs[b].facedowns.push(o);
            g.o(o).uid.into()
        }
        "clear_facedowns" => {
            for b in &mut g.bfs {
                b.facedowns.clear();
            }
            Json::Null
        }
        "bf_ctrl" => {
            g.bfs[a(1).int().unwrap_or(0) as usize].ctrl = Some(who(&a(2)));
            Json::Null
        }
        "clear_base" => {
            // for o in [o for o in g.board if o.loc == "base"]: g.board.remove(o)
            g.board.retain(|&o| g.objs[o.0 as usize].loc != Some(Loc::Base));
            Json::Null
        }
        "keep_base" => {
            // garde les n premières unités prêtes de pid à la base (o.__dict__["_exh"] faux), retire les autres
            let pid = who(&a(1));
            let n = a(2).int().unwrap_or(0) as usize;
            let mine: Vec<O> = g.board.iter().copied().filter(|&o| g.o(o).ctrl == pid && g.o(o).loc == Some(Loc::Base)).collect();
            let keep: Vec<O> = mine.iter().copied().filter(|&o| !g.o(o).exh).take(n).collect();
            g.board.retain(|o| !mine.contains(o) || keep.contains(o));
            Json::Null
        }
        "kill_units" => {
            // for u in list(g.units(pid)): g.kill([u], pid)
            let pid = who(&a(1));
            for u in g.units(Some(pid), None) {
                g.kill(&[u], Some(pid), false).map_err(|_| "GameOver")?;
            }
            Json::Null
        }
        "pool_e" => {
            g.p[who(&a(1))].pool_e = a(2).int().unwrap_or(0);
            Json::Null
        }
        "academy" => {
            // g.effects.append(dict(kind="grant_repeat", pid=me, cost=_academy_cost, next=True, key=..., dur="turn"))
            let pid = who(&a(1));
            let m = crate::map! {
                kind = "grant_repeat", pid = pid, cost = crate::cardsets::battlefields::academy_cost(), next = true,
                key = "academy-robot", dur = "turn"
            };
            let e = g.new_effect(m);
            g.effects.push(e);
            Json::Null
        }
        "predict" => {
            // g.queue_trigger(pid, "Dramatic Visionary", lambda g_, it: g_.predict(it.ctrl, n)) ; g.flush_triggers()
            let pid = who(&a(1));
            let n = a(2).int().unwrap_or(1) as usize;
            let f = Value::func("predict_robot", move |g, x| {
                let c = g.it(x[0].item()).ctrl;
                g.predict(c, n)?;
                Ok(Value::None)
            });
            g.queue_trigger(pid, "Dramatic Visionary", f, Map::new(), None, false, Value::None, Value::None);
            g.flush_triggers().map_err(|_| "GameOver")?;
            Json::Null
        }
        "d_none" => {
            *d = None;
            Json::Null
        }
        // ---- lectures
        "zone" => zone_name(g.obj(uid(&a(1))).and_then(|o| g.o(o).zone)),
        "zone_any" => zone_name(find_any(g, uid(&a(1))).and_then(|o| g.o(o).zone)),
        "in_trash" => g.p[who(&a(1))].trash.iter().any(|&c| g.o(c).uid == uid(&a(2))).into(),
        "in_banish" => g.p[who(&a(1))].banish.iter().any(|&c| g.o(c).uid == uid(&a(2))).into(),
        "on_board" => g.obj(uid(&a(1))).is_some().into(),
        "loc" => loc_json(g.obj(uid(&a(1))).and_then(|o| g.o(o).loc)),
        "damage" => g.obj(uid(&a(1))).map(|o| Json::from(g.o(o).damage)).unwrap_or(Json::Null),
        "might" => match g.obj(uid(&a(1))) {
            Some(o) => g.might(o).map_err(|_| "GameOver")?.into(),
            None => Json::Null,
        },
        "kw" => match g.obj(uid(&a(1))) {
            Some(o) => g.kw_value(o, a(2).str().unwrap_or("")).map_err(|_| "GameOver")?.into(),
            None => Json::Null,
        },
        "exhausted" => g.obj(uid(&a(1))).map(|o| Json::from(g.o(o).exh)).unwrap_or(Json::Null),
        "ready_runes" => g.p[who(&a(1))].runes.iter().filter(|&&r| !g.r(r).exhausted).count().into(),
        "rune_uids" => Json::Arr(g.p[who(&a(1))].runes.iter().map(|&r| g.r(r).uid.into()).collect()),
        "nrunes" => g.p[who(&a(1))].runes.len().into(),
        "hand_len" => g.p[who(&a(1))].hand.len().into(),
        "hand_uids" => {
            let n = a(2);
            Json::Arr(g.p[who(&a(1))].hand.iter().filter(|&&c| n.is_null() || Some(g.cname(c)) == n.str()).map(|&c| g.o(c).uid.into()).collect())
        }
        "trash_uids" => {
            let n = a(2);
            Json::Arr(g.p[who(&a(1))].trash.iter().filter(|&&c| n.is_null() || Some(g.cname(c)) == n.str()).map(|&c| g.o(c).uid.into()).collect())
        }
        "deck_top" => Json::Arr(g.p[who(&a(1))].deck.iter().take(a(2).int().unwrap_or(1) as usize).map(|&c| g.cname(c).into()).collect()),
        "deck_has" => g.p[who(&a(1))].deck.iter().any(|&c| Some(g.cname(c)) == a(2).str()).into(),
        "unit_names" => Json::Arr(g.units(Some(who(&a(1))), None).iter().map(|&u| g.cname(u).into()).collect()),
        "unit_uids" => Json::Arr(g.units(Some(who(&a(1))), None).iter().map(|&u| g.o(u).uid.into()).collect()),
        "facedowns" => Json::Arr(g.bfs.iter().map(|b| Json::Arr(b.facedowns.iter().map(|&c| g.o(c).uid.into()).collect())).collect()),
        "legends" => Json::Arr(g.p.iter().map(|p| p.legend_name.clone().into()).collect()),
        "ai_cfg" => {
            // [W["ag"][1].samples, W["ag"][1].cfg.get("turn_w")] : niveau et réglages donnés à l'IA par new()
            let lv = level.unwrap_or(1);
            Json::Arr(vec![lv.into(), if lv == 6 { Json::Int(6) } else { Json::Null }])
        }
        "get" => ex.get(a(1).str().unwrap_or("")).cloned().unwrap_or(Json::Null),
        "set" => {
            ex.insert(a(1).str().unwrap_or("").to_string(), a(2));
            Json::Null
        }
        x => return Err(format!("opération inconnue {x}")),
    })
}

/// `dbg(ops)` : ops = liste d'opérations `[nom, args...]`, ou objet `{"ops": [...]}` ; chaque opération peut porter un
/// nom de résultat : `{"as": "X", "op": [nom, args...]}`. Renvoie l'objet des résultats nommés. `["ask_may", clé]` :
/// pose la question « may » à l'humain pendant une opération (verif_espace) et renvoie la vue de la table.
pub fn dbg(x: &Json) -> Result<String, String> {
    let ops = match x {
        Json::Str(s) => crate::json::parse(s).map_err(|e| format!("JSONDecodeError: {e}"))?,
        j => j.clone(),
    };
    if let Some(Json::Str(k)) = ops.arr().first().and_then(|o| {
        let a = o.arr();
        (a.first().and_then(|n| n.str()) == Some("ask_may")).then(|| a[1].clone())
    }) {
        return crate::train::dbg_ask_may(&k);
    }
    crate::train::dbg_with(|g, d, ex, level| {
        let mut out = Json::Obj(vec![]);
        for o in ops.arr() {
            let (key, op) = match o {
                Json::Obj(_) => (o.g("as").str().map(|s| s.to_string()), o.g("op").arr().to_vec()),
                _ => (None, o.arr().to_vec()),
            };
            let r = one(g, d, ex, level, &op)?;
            if let Some(k) = key {
                ex.insert(k.clone(), r.clone());
                out.set(&k, r);
            }
        }
        Ok(out.dumps(false))
    })?
}
