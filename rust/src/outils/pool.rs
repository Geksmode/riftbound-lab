//! `multiprocessing.Pool` des scripts Python, en fils (`std::thread`). Chaque tâche est une fonction pure de ses
//! arguments (graine...) : le résultat ne dépend ni du nombre de fils ni de l'ordre d'exécution, et il est rendu dans
//! l'ordre des arguments, comme `Pool.map` / `Pool.imap`.
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;

/// Nombre de fils par défaut (`os.cpu_count()`).
pub fn cpu_count() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

/// `Pool(procs).map(f, args)` : résultats dans l'ordre des arguments.
pub fn map<A: Sync, R: Send>(procs: usize, args: &[A], f: impl Fn(&A) -> R + Sync) -> Vec<R> {
    let mut out: Vec<Option<R>> = (0..args.len()).map(|_| None).collect();
    imap(procs, args, f, |i, r| {
        out[i] = Some(r);
        true
    });
    out.into_iter().map(|r| r.unwrap()).collect()
}

/// `Pool(procs).imap(f, args)` : `take(i, r)` reçoit les résultats dans l'ordre des arguments ; s'il renvoie false,
/// les tâches restantes sont abandonnées (`p.terminate()`).
pub fn imap<A: Sync, R: Send>(
    procs: usize,
    args: &[A],
    f: impl Fn(&A) -> R + Sync,
    mut take: impl FnMut(usize, R) -> bool,
) {
    let procs = procs.max(1).min(args.len().max(1));
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel::<(usize, std::thread::Result<R>)>();
    std::thread::scope(|s| {
        for _ in 0..procs {
            let tx = tx.clone();
            let (next, stop, f) = (&next, &stop, &f);
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn_scoped(s, move || loop {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    if i >= args.len() {
                        break;
                    }
                    let r = catch_unwind(AssertUnwindSafe(|| f(&args[i])));
                    if tx.send((i, r)).is_err() {
                        break;
                    }
                })
                .expect("fil de calcul");
        }
        drop(tx);
        let mut pending: std::collections::BTreeMap<usize, R> = Default::default();
        let mut want = 0;
        while want < args.len() {
            let Ok((i, r)) = rx.recv() else { break };
            match r {
                Ok(r) => {
                    pending.insert(i, r);
                }
                Err(e) => {
                    stop.store(true, Ordering::SeqCst);
                    std::panic::resume_unwind(e);
                }
            }
            while let Some(r) = pending.remove(&want) {
                want += 1;
                if !take(want - 1, r) {
                    stop.store(true, Ordering::SeqCst);
                    want = args.len();
                    break;
                }
            }
        }
        stop.store(true, Ordering::SeqCst);
        drop(rx);
    });
}

/// Exécute `f` en rattrapant une panique (l'`except Exception` des scripts) : Err(message).
pub fn attempt<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|e| {
        if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            "panique".to_string()
        }
    })
}
