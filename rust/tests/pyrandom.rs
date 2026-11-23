//! Le générateur Rust doit donner exactement les tirages de CPython (`tests/data/pyrandom.txt`, produit par
//! `tests/gen_pyrandom.py`) : même graine = même partie que le moteur Python.
use riftbound::pyrandom::PyRandom;

fn ints(s: &str) -> Vec<i64> {
    if s.is_empty() {
        vec![]
    } else {
        s.split(',').map(|x| x.parse().unwrap()).collect()
    }
}

#[test]
fn identique_a_cpython() {
    let data = include_str!("data/pyrandom.txt");
    let mut cur: Option<(i128, PyRandom)> = None;
    let mut n = 0;
    for line in data.lines() {
        let (lhs, rhs) = line.split_once(" = ").map(|(a, b)| (a, b.to_string())).unwrap_or((line.trim_end_matches(" ="), String::new()));
        let p: Vec<&str> = lhs.split(' ').collect();
        let seed: i128 = p[0].parse().unwrap();
        if cur.as_ref().map(|c| c.0) != Some(seed) {
            cur = Some((seed, PyRandom::new(seed)));
        }
        let r = &mut cur.as_mut().unwrap().1;
        let got = match p[1] {
            "getrandbits" => r.getrandbits(p[2].parse().unwrap()).to_string(),
            "randrange" => r.randbelow(p[2].parse().unwrap()).to_string(),
            "randint" => r.randint(p[2].parse().unwrap(), p[3].parse().unwrap()).to_string(),
            "random" => format!("{:?}", r.random()),
            "uniform" => format!("{:?}", r.uniform(p[2].parse().unwrap(), p[3].parse().unwrap())),
            "shuffle" => {
                let mut x: Vec<usize> = (0..p[2].parse().unwrap()).collect();
                r.shuffle(&mut x);
                x.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")
            }
            "choice" => r.choice_index(p[2].parse().unwrap()).to_string(),
            "sample" => r
                .sample_indices(p[2].parse().unwrap(), p[3].parse().unwrap())
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(","),
            m => panic!("méthode inconnue {m}"),
        };
        // les flottants : même valeur (repr Python et Debug Rust donnent tous deux l'écriture la plus courte)
        if p[1] == "random" || p[1] == "uniform" {
            assert_eq!(got.parse::<f64>().unwrap(), rhs.parse::<f64>().unwrap(), "{line}");
        } else if p[1] == "shuffle" || p[1] == "sample" {
            assert_eq!(ints(&got), ints(&rhs), "{line}");
        } else {
            assert_eq!(got, rhs, "{line}");
        }
        n += 1;
    }
    assert_eq!(n, 442);
}
