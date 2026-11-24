//! `fuzz_cards [parties] [--module m | cartes...]` : `fuzz_cards.py` (voir `outils::fuzz`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match riftbound::outils::fuzz::main(&a) {
        Ok(errors) => std::process::exit(if errors > 0 { 1 } else { 0 }),
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1)
        }
    }
}
