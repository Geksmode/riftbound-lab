//! Outils de mesure et d'expérience (portage des scripts Python de `riftbound/engine/` qui jouent des parties pour
//! mesurer : `version.py`, `compare.py`, `fuzz_cards.py`, `mesure/sprt.py`, `exp_*.py`...). Les binaires
//! `src/bin/<nom>.rs` n'en sont que l'interface en ligne de commande. Mêmes options, mêmes fichiers lus et écrits,
//! même format de sortie que les scripts Python ; le parallélisme multiprocessus devient des fils (`pool`), sans
//! effet sur les résultats (chaque partie ne dépend que de sa graine).
pub mod compare;
pub mod empreinte;
pub mod exp;
pub mod exp_search;
pub mod exp_temps;
pub mod fuzz;
pub mod md5;
pub mod pool;
pub mod run_session;
pub mod py;
pub mod sprt;
pub mod t_rand;
pub mod version;
