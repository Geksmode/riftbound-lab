//! `t_rand [parties]` : `riftbound/train/t_rand.py` (voir `outils::t_rand`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::t_rand::main(&a));
}
