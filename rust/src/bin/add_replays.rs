//! `add_replays flips|digest|dsearch|add|picks ...` : `add_replays.py` (voir `outils::replays`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::replays::add_main(&a));
}
