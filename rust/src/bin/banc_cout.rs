//! `banc_cout <pos.jsonl> [tirages] [version] [processus] [seuil]` : `banc/cout.py` (voir `outils::banc`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.is_empty() {
        eprintln!("banc_cout <pos.jsonl> [tirages] [version] [processus] [seuil]");
        std::process::exit(2);
    }
    std::process::exit(riftbound::outils::banc::cout_main(&a));
}
