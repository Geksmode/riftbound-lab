# Riftbound LAB : instructions pour les agents

Ce dépôt est le labo Riftbound TCG de **thanh huy** (GitHub Geksmode). Il contient tout ce qui a été construit avec
Claude entre le 2 et le 7 octobre 2026 : données de cartes et de règles, moteur de règles fidèle avec toutes les cartes
modélisées, IA, simulations de matchups, table d'entraînement jouable dans le navigateur, replays, retex.
Il a été créé pour que n'importe quel agent (un autre compte Claude, une autre session) reprenne le travail sans rien
redemander. Lis ce fichier en entier, puis `docs/REPRISE.md` avant toute tâche.

## L'utilisateur
- Écris-lui **en français**, simplement, en commençant par la réponse.
- Il joue **Akali** (liste « Akali Heron » de Gorica, variante G2) contre **LeBlanc** (IQ#5) et Master Yi. Objectif :
  optimiser ses decks et ses plans de jeu à partir de données empiriques, et s'entraîner contre l'IA.
- Il veut des réponses **factuelles et justes** : pas de simplification du moteur (« il faut savoir modéliser toutes
  les cartes »), pas de chiffre sans intervalle.

## Règles permanentes (décidées par l'utilisateur, ne pas les re-discuter)
1. **Retex** = revue autocritique factuelle (ce qui tient, ce qui était surévalué, les biais, ce qu'on change), pas une
   collecte de comptes rendus de tournoi. Voir `docs/memoire/retex-method.md`.
2. **Simulations** : toujours rejouer une option retenue sur des graines **neuves** avant de l'annoncer ; ne jamais
   regrouper des configurations jouées sur les mêmes graines comme si elles étaient indépendantes ; donner l'intervalle
   (± écart-type) et dire « on ne sait pas » sous 2 écarts-types. Ne jamais agréger des chiffres de deux versions du moteur.
3. **Replays après chaque simulation** : ajouter les 2 à 5 parties les plus instructives au lecteur de replays
   (`riftbound/engine/add_replays.py`, voir `docs/memoire/replays-after-every-sim.md`).
4. **Cartes non modélisées** : une carte absente de `cards.IMPL` est bloquée dans l'éditeur de deck. Ne jamais
   enregistrer une carte avec un effet partiel ou approximé ; une carte bloquée vaut mieux qu'une carte fausse.
5. **Vault** : le vault du projet est `docs/vault/` (notes Obsidian en français, liens `[[...]]`). Le vrai vault de
   l'utilisateur est sur son PC (Obsidian « Cerveau », dossier `20 Projets\Riftbound\`) : n'y écrire que s'il le
   demande lui-même. Mettre à jour les notes du vault quand un résultat change (chiffres remplacés → `Chiffres périmés.md`).
6. **Ne dis « testé »** que pour un chemin joué de bout en bout dans une vraie partie (navigateur compris pour la table).

## Carte du dépôt
| Dossier | Contenu |
|---|---|
| `riftbound/cards/` | base de cartes (938 cartes uniques, 1231 impressions avec `image_url`), script de reconstruction |
| `riftbound/rules/` | Core Rules complètes du 2026-07-16 (`source/`) et résumés FR, règles de tournoi, ban list |
| `riftbound/decks/` | decklists de tournoi parsées (`decks.json`), analyses de matchup |
| `riftbound/engine/` | **moteur de règles fidèle** 1v1 + IA + simulations ; `cardsets/` = toutes les cartes par paquet. Lire `engine/README.md` puis `engine/cardsets/GUIDE.md` |
| `riftbound/gameplans/` | plans de jeu (Akali Gorica, LeBlanc Hook tempo…) et notes de vidéos |
| `riftbound/manager/` | agent manager : sessions de simulation automatiques toutes les 6 h (`PROTOCOLE.md`, `learnings.md`, `sessions/`) |
| `riftbound/replays/` | lecteur de replays graphique (`viewer.html`) et parties enregistrées |
| `riftbound/train/` | **table d'entraînement** (jouer contre l'IA dans le navigateur, Pyodide), éditeur de deck, parties enregistrées de l'utilisateur, outils d'analyse |
| `riftbound/puzzles/` | puzzles Akali vs Yi |
| `riftbound/retex/` | retex écrits |
| `riftbound/sim/` | ancien simulateur simplifié, **périmé**, gardé pour l'historique |
| `docs/memoire/` | la mémoire de travail accumulée par les agents (un fait par fichier, index `MEMORY.md`) |
| `docs/vault/` | le vault Obsidian du projet (index `Riftbound - Accueil.md`) |
| `docs/REPRISE.md` | état exact du travail, éléments liés à l'ancien compte (artefacts, routine) et comment les recréer |

## Commandes
```bash
cd riftbound/engine
python3 test_all.py                      # toute la suite (cartes d'origine + chaque paquet de cardsets/)
python3 test_cards.py                    # les 91 tests d'origine
python3 fuzz_cards.py 200 --module fury  # parties aléatoires avec les cartes d'un paquet (0 exception attendu)
RB_CARDSETS=fury,body python3 ...        # ne charger que certains paquets (travail en parallèle)
python3 train.py 3                       # partie de test de la table d'entraînement (humain au hasard)
python3 ../train/t_rand.py 40            # 40 parties avec des decks légaux au hasard via train.py
cd ../train && bash build.sh             # construit la table d'entraînement dans train/build/ (voir README)
```
Python 3 standard, aucune dépendance (le moteur tourne aussi dans Pyodide 0.26.4, 32 bits, Python 3.12) :
- jamais d'ordre d'itération d'un `set` qui compte (utiliser `sorted()`), la même graine doit donner la même partie ;
- pas de nouvelle dépendance dans le moteur.

## Conventions
- Le moteur cite les numéros de règle en commentaire. Une carte = une entrée `card(nom, ...)`, avec au moins un test.
- Les chemins sont relatifs à `riftbound/` (le code utilise `Path(__file__).parents[1]`) : garder cette arborescence.
- Après une modification du moteur : `test_all.py`, puis `fuzz_cards.py` sur les paquets touchés, puis 30 parties
  `t_rand.py`, puis la table dans un navigateur (robot Playwright `train/dk2.py`) avant de republier.
- Les résultats de simulation d'avant le 2026-10-06 ont été mesurés sans l'énergie flottante : les remesurer avant
  de les citer (`docs/vault/Chiffres périmés.md`).

## Équipe d'agents (sessions tmux + chef)
Pour faire travailler plusieurs agents en parallèle sur ce dépôt : `scripts/equipe.sh start` crée une session tmux `rift`
avec une fenêtre `chef` et une fenêtre par agent (`cartes`, `design`, `ia`, `simulation`, `retex`), chacun dans son worktree
git (`../rb-<agent>`) sur sa branche `agent/<nom>`. Rôles : `.claude/agents/<nom>.md` ; rôle du chef : `.claude/chef.md`.
Le chef délègue avec `scripts/equipe.sh send <agent> "<tâche>"` et relit avec `status` / `read` ; les agents répondent par
des rapports dans `$RB_EQUIPE/rapports/`. Personne ne fusionne dans `main` sauf l'utilisateur (pull request).
