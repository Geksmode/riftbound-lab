//! `texel_gen <parties> <graine0> <sh|turn|pol> <sortie.jsonl> [processus]` : `texel/gen.py` (voir `outils::texel`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::texel::gen_main(&a));
}
