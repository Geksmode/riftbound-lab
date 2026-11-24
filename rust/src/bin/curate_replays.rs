//! `curate_replays` : `curate_replays.py` (voir `outils::replays`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::replays::curate_main(&a));
}
