//! `combats_parties <parties.json ou dossier>... [--json]` : `combats_parties.py` (voir `outils::combats`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::combats::main(&a));
}
