//! `exp_search <paires> <graine0> <A> <B> <akali|miroir|defaut> [processus]` : `exp_search.py` (voir
//! `outils::exp_search`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::exp_search::main(&a));
}
