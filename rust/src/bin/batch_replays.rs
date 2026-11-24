//! `batch_replays <n> <seed0> <dossier>` : `batch_replays.py` (voir `outils::replays`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::replays::batch_main(&a));
}
