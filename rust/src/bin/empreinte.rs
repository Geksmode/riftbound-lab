//! `empreinte [--maj]` : `empreinte.py` (voir `outils::empreinte`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::empreinte::main(a.iter().any(|x| x == "--maj")));
}
