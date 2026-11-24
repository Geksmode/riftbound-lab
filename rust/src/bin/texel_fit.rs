//! `texel_fit <données.jsonl> [processus] [boot]` : `texel/fit.py` (voir `outils::texel`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::texel::fit_main(&a));
}
