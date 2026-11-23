//! Aides pures de cards.py et actions.py contre Python (tests/data/helpers.txt, produit par tests/gen_helpers.py) :
//! parse_cost, power_list, repeat_cost, flow_cost sur le texte de chaque carte, keywords_of et parse_keywords sur
//! chaque carte, auto_keywords et is_token, _drop_reqs et _flex sur des cas tirés au hasard.
use riftbound::actions::{drop_reqs, flex};
use riftbound::cards_helpers::{auto_keywords, is_token, flow_cost, keywords_of, parse_cost, parse_keywords, power_list, repeat_cost, KwFields};
use riftbound::game_costs::Req;
use riftbound::value::Value;

fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(x) => out.push(x),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn req_s(r: &Req) -> String {
    r.iter().cloned().collect::<Vec<_>>().join(",")
}

fn reqs_s(rs: &[Req]) -> String {
    rs.iter().map(req_s).collect::<Vec<_>>().join(";")
}

fn parse_reqs(s: &str) -> Vec<Req> {
    if s.is_empty() {
        return vec![];
    }
    s.split(';').map(|r| r.split(',').map(|d| d.to_string()).collect()).collect()
}

fn variants_s(vs: &[(i64, Vec<Req>)]) -> String {
    vs.iter().map(|(e, r)| format!("{e}:{}", reqs_s(r))).collect::<Vec<_>>().join("/")
}

fn parse_variants(s: &str) -> Vec<(i64, Vec<Req>)> {
    s.split('/')
        .map(|v| {
            let (e, r) = v.split_once(':').unwrap();
            (e.parse().unwrap(), parse_reqs(r))
        })
        .collect()
}

fn strs(v: &Value) -> String {
    v.list().iter().map(|d| d.str().to_string()).collect::<Vec<_>>().join(",")
}

fn fields_s(f: &Option<KwFields>) -> String {
    let Some(f) = f else { return "None".into() };
    let mut out = vec![];
    // clés triées, comme le générateur Python
    if f.accelerate {
        out.push("accelerate=True".to_string());
    }
    if f.ambush {
        out.push("ambush=True".to_string());
    }
    if f.hidden {
        out.push("hidden=True".to_string());
    }
    if let Some(kw) = &f.kw {
        out.push(format!("kw={}", kw.0.iter().map(|(k, v)| format!("{k}={}", v.int())).collect::<Vec<_>>().join(",")));
    }
    if f.quickdraw {
        out.push("quickdraw=True".to_string());
    }
    if let Some(t) = f.timing {
        out.push(format!("timing={t}"));
    }
    out.join(" ")
}

#[test]
fn identiques_a_python() {
    let data = include_str!("data/helpers.txt");
    let (mut nc, mut nk, mut nd, mut nf) = (0, 0, 0, 0);
    for line in data.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "cost" => {
                let t = unesc(f[1]);
                let (e, r) = parse_cost(&t);
                assert_eq!(format!("{e}|{}", reqs_s(&r)), f[2], "parse_cost {t:?}");
                let (pe, pl) = power_list(&t);
                assert_eq!(format!("{pe}|{}", pl.join(",")), f[3], "power_list {t:?}");
                let rc = repeat_cost(&t);
                let rc = rc.list();
                assert_eq!(format!("{}|{}|{}", rc[0].int(), strs(&rc[1]), rc[2].int()), f[4], "repeat_cost {t:?}");
                let fc = flow_cost(&t);
                let fc = fc.list();
                assert_eq!(format!("{}|{}|{}", fc[0].int(), fc[1].int(), strs(&fc[2])), f[5], "flow_cost {t:?}");
                nc += 1;
            }
            "kw" => {
                let n = f[1];
                assert_eq!(keywords_of(n).into_iter().collect::<Vec<_>>().join(","), f[2], "keywords_of {n:?}");
                assert_eq!(fields_s(&parse_keywords(n)), f[3], "parse_keywords {n:?}");
                assert_eq!(auto_keywords(None, n).is_some() as i32, f[4].parse::<i32>().unwrap(), "auto_keywords {n:?}");
                assert_eq!(is_token(n) as i32, f[5].parse::<i32>().unwrap(), "is_token {n:?}");
                nk += 1;
            }
            "drop" => {
                let rs = parse_reqs(f[1]);
                let n: usize = f[2].parse().unwrap();
                assert_eq!(reqs_s(&drop_reqs(&rs, n)), f[3], "_drop_reqs {line}");
                nd += 1;
            }
            "flex" => {
                let vs = parse_variants(f[1]);
                assert_eq!(variants_s(&flex(&vs)), f[2], "_flex {line}");
                nf += 1;
            }
            k => panic!("ligne inconnue {k}"),
        }
    }
    assert!(nc > 1000 && nk > 1000 && nd == 300 && nf == 300, "{nc} {nk} {nd} {nf}");
}
