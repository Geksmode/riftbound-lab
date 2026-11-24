//! `table` : la table d'entraînement en ligne de commande (test différentiel, `rust/diff/table.py`). Chaque ligne lue
//! est un appel `[fonction, [arguments...]]` (JSON) ; chaque réponse est une ligne JSON `{"r": texte}` ou
//! `{"err": texte}`, comme les messages du Web Worker de la page.
use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }
        let j = riftbound::json::parse(&line).expect("appel illisible");
        let a = j.arr();
        let f = a[0].str().unwrap_or("").to_string();
        let args = a.get(1).map(|x| x.arr().to_vec()).unwrap_or_default();
        let r = std::panic::catch_unwind(|| riftbound::train::call(&f, &args));
        let o = match r {
            Ok(Ok(s)) => riftbound::jo! { "r" => s },
            Ok(Err(e)) => riftbound::jo! { "err" => e },
            Err(p) => {
                let m = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()));
                riftbound::jo! { "err" => format!("panic: {}", m.unwrap_or_default()) }
            }
        };
        writeln!(out, "{}", o.dump()).unwrap();
        out.flush().unwrap();
    }
}
