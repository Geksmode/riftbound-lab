//! `exp_gorica.py <parties> [budget]` (voir `outils::exp`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::exp::suite_main(&a, riftbound::outils::exp::gorica_jobs(), "results_gorica.json", 44));
}
