//! `banc_humain <parties.json ou dossier> [...] [--ver <version>]... [--temoins <n>]` : `banc/humain.py` (voir
//! `outils::humain`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::humain::main(&a));
}
