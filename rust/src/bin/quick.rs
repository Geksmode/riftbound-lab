//! `quick.py <n> <sr|rs|ss>` (voir `outils::exp`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::exp::quick_main(&a));
}
