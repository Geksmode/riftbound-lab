//! `version` : affiche la version du moteur Rust (`engine_md5`, suffixe `-rust` ; voir `outils::version`).
fn main() {
    println!("{}", riftbound::outils::version::engine_md5());
}
