//! `train_games <parties.json ou dossier>... [--coach] [--frozen] [--emit]` : `train_games.py` (voir
//! `outils::train_games`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::train_games::main(&a));
}
