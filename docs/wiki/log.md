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

## [2026-10-08] decision | Gemdragon et Hwei : « ready up to 2 runes »
Le maximum possible jusqu'à 2, rien si toutes les runes sont prêtes (décision de l'utilisateur) ; tests des cas limites ajoutés.

## [2026-10-08] ingest | Problèmes connus corrigés
Repeat accordé (`tg2`), lien Replays, bulle de fin de partie à 360 px, boutons ± de l'éditeur ; robot `train/verif_mobile.mjs`.

## [2026-10-08] ingest | Porte d'intégration
`scripts/verifier.sh` (wiki, tests, fuzz, parties aléatoires, build, robot navigateur), branchée dans la CI. Page `pages/porte.md`.

## [2026-10-08] ingest | Chiffres versionnés
`engine/version.py` (même hachage que le manager), `engine/compare.py` (apparié / indépendant / refus), étiquetage des expériences. Page `pages/resultats.md`.

## [2026-10-08] ingest | Tests générés depuis le texte des cartes
`cardsets/test_texte_auto.py` : 233 sorts et 15 unités contrôlés automatiquement ; 8 faux positifs du classement expliqués ; Deathgrip à trancher.

## [2026-10-08] ingest | Indicateur d'énergie flottante
Badge dans la zone des runes de la table, puissance flottante exportée (`replay.snap` : `pp`) ; test Punch First.

## [2026-10-08] decision | Règles au plus près ; Deathgrip
Nouvelle règle permanente 6 de `CLAUDE.md` : en cas de doute, la lecture la plus fidèle aux Core Rules. Deathgrip : deux cibles alliées obligatoires (355.8).

## [2026-10-08] ingest | Assignation des dégâts de combat par le joueur
`game.assign_by_hand` : question « damage_pick » dans la table ; règles 465.2.c, 815, 826.

## [2026-10-08] ingest | Boucle des parties
Parties gardées dans le navigateur et téléchargeables, commit noté, rejeu par moteur exact (17/17 fidèles), `bilan_parties.py`, workflow « Apprentissage ». Page `pages/parties.md`.

## [2026-10-08] ingest | Match BO1 / BO3 et sideboard (moteur)
`match.py`, API `train.match_*`, sideboard validé (601.1.c). Écart signalé : en BO3 chaque joueur choisit son battlefield (486.5). Page `pages/match.md`.

## [2026-10-08] ingest | Interface du match BO1 / BO3 et du sideboard
Agent design : éditeur (sideboard), mode de match, robot `verif_match.mjs` ajouté à la porte ; chef : `obf` au rejeu.
