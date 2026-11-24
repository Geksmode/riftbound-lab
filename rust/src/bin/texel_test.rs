//! `texel_test [parties]` : `texel/test_texel.py` (voir `outils::texel_test`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::texel_test::main(&a));
}
