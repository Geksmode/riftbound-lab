//! La base de cartes Rust doit être exactement `game.SPEC` (tests/data/spec.txt, produit par tests/gen_spec.py).
use riftbound::spec::specs;

#[test]
fn identique_a_python() {
    let data = include_str!("data/spec.txt");
    let mut n = 0;
    for line in data.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let s = specs().get(f[0]).unwrap_or_else(|| panic!("absente : {}", f[0]));
        let got = [
            s.name.clone(), s.type_.clone(), s.super_.clone(), s.domains.join("|"), s.e.to_string(), s.p.to_string(),
            s.might.map(|m| m.to_string()).unwrap_or_default(),
            s.tags.iter().cloned().collect::<Vec<_>>().join("|"), s.keywords.join("|"),
            s.text.replace('\n', "\\n").replace('\t', "\\t"),
        ]
        .join("\t");
        assert_eq!(got, line);
        n += 1;
    }
    assert_eq!(n, specs().len());
}
