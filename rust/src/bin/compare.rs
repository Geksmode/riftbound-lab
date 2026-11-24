//! `compare` : `compare.py` (voir `outils::compare`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::compare::main(&a));
}
