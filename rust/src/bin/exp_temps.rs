//! `exp_temps <graines séparées par des virgules> <version> [<version>...]` : `exp_temps.py` (voir
//! `outils::exp_temps`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 2 {
        eprintln!("exp_temps <graines séparées par des virgules> <version> [<version>...]");
        std::process::exit(2);
    }
    let seeds: Vec<i64> = a[0].split(',').map(|x| x.parse().expect("graine")).collect();
    riftbound::outils::exp_temps::main(&seeds, &a[1..]);
}
