# Journal du wiki (le plus récent en bas)

Format : `## [AAAA-MM-JJ] type | titre` (types : ingest, decision, lint, restructure). Une à trois lignes par entrée.

## [2026-10-02] ingest | Premier retex et vault
Premier retex : chiffres surévalués corrigés (voir `docs/vault/Chiffres périmés.md`) ; notes du vault créées.

## [2026-10-06] ingest | Table d'entraînement et enregistrement des parties
Table d'entraînement v10 à v12, `train_games.py`, 17 parties de l'utilisateur ; énergie flottante (anciens chiffres périmés).

## [2026-10-07] restructure | Dépôt GitHub, CI et GitHub Pages
Dépôt `Geksmode/riftbound-lab` créé (PR 1) : `.github/workflows/ci.yml` (tests et build de la table en parallèle, publication sur Pages depuis `main`).

## [2026-10-07] decision | Équipe d'agents en sessions tmux, avec un chef
`scripts/equipe.sh`, rôles dans `.claude/` (PR 2), missions dans `docs/MISSIONS.md`. Détail : `pages/equipe.md`.

## [2026-10-07] restructure | Architecture en wiki
Ajout de `docs/wiki/` (index, conventions, journal, contrôle `scripts/wiki_lint.py`) ; `CLAUDE.md` réduit aux règles ; carte du dépôt et état déplacés en pages.

## [2026-10-07] decision | Ordre des chantiers de process
Porte d'intégration, chiffres versionnés, tests générés depuis le texte des cartes, boucle d'apprentissage des parties. Détail : `pages/etat.md`.

## [2026-10-08] ingest | Première itération des agents et retex
Intégration de `agent/cartes`, `agent/ia`, `agent/design` ; 1034 tests ; retex écrit. Voir `pages/etat.md` et `riftbound/retex/retex-equipe-iteration1.md`.

