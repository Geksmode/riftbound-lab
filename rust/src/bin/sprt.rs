//! `sprt <graine0> <A> <B> [defaut|miroir|akali] [processus] [max_paires]` : `mesure/sprt.py` (voir `outils::sprt`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::sprt::main(&a));
}
