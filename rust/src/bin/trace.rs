//! `trace <graine> <paquet0> <paquet1> [graine_agent0 graine_agent1]` : journal d'une partie (test différentiel,
//! même sortie que `rust/diff/trace.py`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let seed: i128 = a[0].parse().unwrap();
    let (a0, a1) = if a.len() >= 5 { (a[3].parse().unwrap(), a[4].parse().unwrap()) } else { (1, 2) };
    for l in riftbound::trace::run(seed, &a[1], &a[2], a0, a1, 5000) {
        println!("{l}");
    }
}
