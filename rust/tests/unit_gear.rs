//! Patched Porobot est une « Gear Unit » imprimée (`spec::UNIT_GEAR`) : il a les deux types (règles 178.2, 178.3).
//! Parties complètes jouées par des agents aléatoires sur des paquets qui contiennent Patched Porobot et Pit Crew ;
//! à chaque Porobot joué on vérifie :
//!   - il entre épuisé (359.2.c ; 359.2.d ne redresse que les « Non-Unit Gear ») ;
//!   - chaque Pit Crew du joueur (« When you play a gear, ready me. ») met son déclenchement sur la chaîne.
use riftbound::agents::RandomAgent;
use riftbound::game::{Agent, Counters, Game, O};
use riftbound::outils::fuzz::build;
use riftbound::spec::{specs, VICTORY};

fn on_board(g: &Game, name: &str, pid: usize) -> Vec<O> {
    g.board.iter().copied().filter(|&o| g.cname(o) == name && g.o(o).ctrl == pid).collect()
}

/// (Porobot joués, dont avec un Pit Crew en jeu)
fn partie(seed: i64) -> (usize, usize) {
    let names = vec!["Patched Porobot".to_string(), "Pit Crew".to_string()];
    let mut rng = riftbound::pyrandom::PyRandom::new(seed as i128);
    let keys: Vec<&String> = riftbound::decks::decks().keys().collect();
    let n = keys.len() as i64;
    let d0 = build(keys[(seed % n) as usize], &names, &mut rng);
    let d1 = build(keys[((seed + 5) % n) as usize], &names, &mut rng);
    let agents: Vec<Box<dyn Agent>> =
        vec![Box::new(RandomAgent::new(seed as i128)), Box::new(RandomAgent::new(seed as i128 + 7919))];
    let mut g =
        Game::new(&[d0, d1], agents, None, seed as i128, true, VICTORY as i64, std::sync::Arc::new(Counters::default()));
    for pid in 0..2 {
        let mut a = std::mem::replace(&mut g.agents[pid], Box::new(riftbound::game_events::FirstChoice));
        a.start(&mut g);
        g.agents[pid] = a;
    }
    let mut joues = 0;
    let mut attendus: Vec<(usize, usize)> = vec![]; // (ligne du « plays », Pit Crew du joueur en jeu)
    for _ in 0..4000 {
        let Some(d) = g.advance(false) else { break };
        let mut a = std::mem::replace(&mut g.agents[d.player], Box::new(riftbound::game_events::FirstChoice));
        let r = a.decide(&mut g, &d);
        g.agents[d.player] = a;
        let Ok(act) = r else { break };
        let avant = g.lines.len();
        let pits: Vec<usize> = (0..2).map(|p| on_board(&g, "Pit Crew", p).len()).collect();
        let poros: Vec<Vec<O>> = (0..2).map(|p| on_board(&g, "Patched Porobot", p)).collect();
        g.apply(&act);
        for pid in 0..2 {
            let joue = format!("P{pid} plays Patched Porobot from hand");
            let Some(i) = g.lines[avant..].iter().position(|l| l.contains(&joue)) else { continue };
            joues += 1;
            // le Porobot qui vient d'entrer à la base : épuisé (sur un battlefield, un effet d'unité peut le redresser :
            // Riven, The Exile, « Units you play to battlefields you control enter ready. »)
            let a_la_base = g.lines[avant + i].contains("loc=base");
            for o in on_board(&g, "Patched Porobot", pid) {
                if a_la_base && !poros[pid].contains(&o) {
                    assert!(g.exhausted(o), "graine {seed} : le Porobot joué doit entrer épuisé\n{}", g.lines[avant.saturating_sub(12)..].join("\n"));
                }
            }
            if pits[pid] > 0 {
                attendus.push((avant + i, pits[pid]));
            }
        }
    }
    // les déclenchements arrivent sur la chaîne avant la carte jouée suivante
    for &(i, k) in &attendus {
        let fin = g.lines[i + 1..].iter().position(|l| l.contains(" plays ")).map_or(g.lines.len(), |j| i + 1 + j);
        let n = g.lines[i..fin].iter().filter(|l| l.ends_with("trigger on chain: Pit Crew")).count();
        assert!(n >= k, "graine {seed} : {k} Pit Crew en jeu, {n} déclenchement(s) après le Porobot\n{}", g.lines[i..fin].join("\n"));
    }
    (joues, attendus.len())
}

#[test]
fn porobot_est_une_unite_et_un_equipement() {
    let sp = &specs()["Patched Porobot"];
    assert_eq!(sp.type_, "Unit");
    assert!(sp.is_gear());
    assert!(!specs()["Pit Crew"].is_gear());
    assert!(specs()["Gold"].is_gear());
}

#[test]
fn pit_crew_se_declenche_quand_on_joue_un_porobot() {
    let (mut joues, mut avec_pit) = (0, 0);
    for seed in 1..=60 {
        let (j, p) = partie(seed);
        joues += j;
        avec_pit += p;
    }
    println!("{joues} Porobot joués, dont {avec_pit} avec un Pit Crew en jeu");
    assert!(avec_pit >= 20, "trop peu de cas joués : {avec_pit}");
}
