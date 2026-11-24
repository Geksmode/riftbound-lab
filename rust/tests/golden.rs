//! Non-régression contre le moteur Python de référence (supprimé à la bascule) : chaque ligne de
//! `data/golden.txt` (produit par `rust/diff/golden.sh` avec le moteur Python) donne une partie et le MD5 de son
//! journal de trace. Le moteur Rust doit écrire exactement le même journal.
//! Après une correction de règle, les seules parties dont le journal change sont refigées avec le moteur Rust, après
//! comparaison ligne à ligne avec le moteur d'avant (`docs/wiki/pages/porte.md`). Refigées le 2026-11-24 : 11 parties
//! des paquets Ornn et Renata (Patched Porobot est une « Gear Unit », règles 178.2 et 178.3).
//! Les parties de l'IA (lentes) ne tournent qu'avec `RB_GOLDEN_IA=1` ; les parties aléatoires tournent toujours.
use riftbound::outils::md5::md5_hex;
use std::sync::atomic::{AtomicUsize, Ordering};

fn journal(seed: i128, a: &str, b: &str, ia: bool) -> String {
    let mut s = String::new();
    for l in riftbound::trace::run_with(seed, a, b, 1, 2, 5000, ia) {
        s.push_str(&l);
        s.push('\n');
    }
    s
}

fn verifie(mode: &str) {
    let data = include_str!("data/golden.txt");
    let lignes: Vec<Vec<&str>> =
        data.lines().map(|l| l.split(' ').collect::<Vec<_>>()).filter(|f| f[0] == mode).collect();
    assert!(!lignes.is_empty(), "aucune partie de référence « {mode} »");
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2);
    let k = AtomicUsize::new(0);
    let fautes: Vec<String> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..n)
            .map(|_| {
                sc.spawn(|| {
                    let mut f = vec![];
                    loop {
                        let i = k.fetch_add(1, Ordering::Relaxed);
                        let Some(l) = lignes.get(i) else { break };
                        let seed: i128 = l[1].parse().unwrap();
                        let got = md5_hex(journal(seed, l[2], l[3], mode == "ia").as_bytes());
                        if got != l[4] {
                            f.push(format!("{} {} {} {}", l[0], l[1], l[2], l[3]));
                        }
                    }
                    f
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    assert!(fautes.is_empty(), "parties différentes du moteur Python de référence : {fautes:?}");
    println!("{} parties « {mode} » identiques à la référence Python", lignes.len());
}

#[test]
fn parties_aleatoires_identiques_a_python() {
    verifie("alea");
}

#[test]
fn parties_ia_identiques_a_python() {
    if std::env::var("RB_GOLDEN_IA").is_err() {
        return;
    }
    verifie("ia");
}
