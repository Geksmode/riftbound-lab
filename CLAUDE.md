# Riftbound LAB : instructions pour les agents

Labo Riftbound TCG de **thanh huy** (GitHub Geksmode) : cartes et règles, moteur de règles fidèle avec toutes les cartes
modélisées, IA, simulations, table d'entraînement jouable dans le navigateur, replays, retex. Créé pour que n'importe quel
agent reprenne le travail sans rien redemander.

**Contexte minimal : commence par `docs/wiki/index.md`** (table « par tâche »), puis ouvre seulement les pages utiles.
Ne lis pas des dossiers entiers. Conventions du wiki : `docs/wiki/WIKI.md`. Après un résultat, une décision ou un changement
durable : mets à jour la page concernée, l'index et `docs/wiki/log.md`, puis `python3 scripts/wiki_lint.py`.

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
   (± écart-type) et dire « on ne sait pas » sous 2 écarts-types. Ne jamais agréger des chiffres de deux versions du moteur :
   comparer et regrouper avec `riftbound/engine/compare.py`, qui refuse ces cas (`docs/wiki/pages/resultats.md`).
3. **Replays après chaque simulation** : ajouter les 2 à 5 parties les plus instructives au lecteur de replays
   (`riftbound/engine/add_replays.py`, voir `docs/memoire/replays-after-every-sim.md`).
4. **Cartes non modélisées** : une carte absente de `cards.IMPL` est bloquée dans l'éditeur de deck. Ne jamais
   enregistrer une carte avec un effet partiel ou approximé ; une carte bloquée vaut mieux qu'une carte fausse.
5. **Vault** : le vault du projet est `docs/vault/` (notes Obsidian en français, liens `[[...]]`). Le vrai vault de
   l'utilisateur est sur son PC (Obsidian « Cerveau », dossier `20 Projets\Riftbound\`) : n'y écrire que s'il le
   demande lui-même. Chiffres remplacés → `docs/vault/Chiffres périmés.md`.
6. **Règles au plus près** : en cas de doute sur une carte, appliquer la lecture la plus fidèle aux Core Rules
   (`riftbound/rules/source/core_rules_2026-07-16.txt`) et la citer. Ex. Deathgrip : deux cibles obligatoires (355.8).
7. **Ne dis « testé »** que pour un chemin joué de bout en bout dans une vraie partie (navigateur compris pour la table).
   Pour la table, distingue « vérifié par robot (toucher simulé) » et « essayé sur un vrai téléphone » (retex 2026-10-09).

## Conventions
- Moteur : Python 3 standard, aucune dépendance (il tourne aussi dans Pyodide 0.26.4, 32 bits) ; jamais d'ordre
  d'itération d'un `set` qui compte (`sorted()`) : même graine = même partie. Une carte = une entrée `card(nom, ...)`
  avec au moins un test ; les numéros de règle sont cités en commentaire.
- Avant d'intégrer ou de publier : `scripts/verifier.sh` doit être vert (tests, fuzz de chaque paquet, parties aléatoires,
  build et partie jouée dans un navigateur ; `docs/wiki/pages/porte.md`). Commandes et carte du dépôt : `docs/wiki/pages/depot.md`.
- Les résultats de simulation d'avant le 2026-10-06 sont périmés (énergie flottante) : les remesurer avant de les citer.
- Processus de fond (serveur http, PeerJS) : les arrêter par leur PID enregistré, jamais `pkill -f` (il tue ton propre shell).
- Avant de committer sur la branche de travail : vérifier si la PR précédente est fusionnée (`git log origin/main`) ; si oui,
  repartir de `origin/main` (une PR fusionnée ne reçoit plus rien).
- Travail en équipe (chef + agents tmux) : `docs/wiki/pages/equipe.md`, missions : `docs/MISSIONS.md`.
