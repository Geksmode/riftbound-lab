# Simulateur Akali vs LeBlanc

Moteur simplifié (pas un moteur de règles complet) pour comparer des choix de deck et de plan de jeu.
Analyse et conclusions : `../decks/matchups/akali-vs-leblanc-simulations.md`.

| Fichier | Rôle |
|---|---|
| `riftsim.py` | moteur + IA Akali / LeBlanc (coûts et might lus dans `../cards/cards_unique.csv`) |
| `run_sims.py` | expériences de base ; `python3 run_sims.py 2000` ou `python3 run_sims.py --log SEED` pour une partie commentée |
| `search_builds.py` | test des échanges 1-pour-1 / 2-pour-2 sur la liste stock (`swaps.json`) |
| `final_experiments.py` | builds combinés + battlefields, mulligan, ordre de jeu (`final_results.json`) |
| `headline.py` | comparaison finale à 4000 parties |
| `sample_logs.py`, `logs/` | parties exemples tour par tour |

Les decks sont lus depuis `../decks/decks.json` (relancer `../decks/parse_decks.py` après ajout d'une liste).
Options IA Akali (`cfg_a`) : plan (`kill` / `leblanc`), mulligan (`default` / `aggro`), respect_vi, aggro, hold_nsf, hold_defy, zhonya_min, counter_min, empower_legend, hide_min_runes.
Hypothèses non issues des données marquées `ASSUMPTION` dans le code.
