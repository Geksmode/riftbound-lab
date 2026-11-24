//! `exp.py <parties> [budget]` (voir `outils::exp`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::exp::suite_main(&a, riftbound::outils::exp::exp_jobs(), "results.json", 42));
}
