//! `exp_general.py <paires> <graine0> <A> <B> [processus]` (voir `outils::exp`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::exp::general_main(&a));
}
