//! `run_session [agenda.json] [budget_secondes]` : `riftbound/manager/run_session.py` (voir
//! `outils::run_session`).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(riftbound::outils::run_session::main(&a));
}
