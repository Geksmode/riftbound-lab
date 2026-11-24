//! `bilan_parties [dossier des parties] [choices.jsonl]` : `bilan_parties.py` (voir `outils::train_games`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::train_games::bilan_main(&a));
}
