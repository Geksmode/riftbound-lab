//! `exp_plans.py <parties> [graine0] [budget]` (voir `outils::exp`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::exp::plans_main(&a));
}
