//! `banc gen|bilan|rejuge|rejoue ...` : `banc/banc.py` (voir `outils::banc`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::banc::main(&a));
}
