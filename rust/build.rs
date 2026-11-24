// Version du moteur Rust (`version.py engine_md5`, adaptée) : MD5 des sources du moteur, calculé à la compilation pour
// que le binaire porte la version des sources dont il est fait. Même recette que Python (chemin relatif + contenu de
// chaque fichier, dans l'ordre des chemins, 10 caractères hexadécimaux), sur les fichiers .rs de `src/` (sous-dossiers
// compris) hors `src/bin/` (interfaces en ligne de commande). Le suffixe « -rust » vient de `outils::version`.
#[allow(dead_code)]
mod md5 {
    include!("src/outils/md5.rs");
}
use md5::Md5;

fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<(String, std::path::PathBuf)>) {
    let mut ents: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    ents.sort();
    for p in ents {
        let rel: Vec<String> =
            p.strip_prefix(root).unwrap().components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        let rel = rel.join("/");
        if p.is_dir() {
            if rel != "bin" {
                walk(&p, root, out);
            }
        } else if rel.ends_with(".rs") {
            out.push((rel, p));
        }
    }
}

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = vec![];
    walk(&root, &root, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut h = Md5::new();
    for (rel, p) in &files {
        h.update(rel.as_bytes());
        h.update(&std::fs::read(p).unwrap());
    }
    println!("cargo:rustc-env=RB_ENGINE_SRC_MD5={}", &h.hexdigest()[..10]);
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");
}
