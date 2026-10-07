# Missions de l'équipe (décidées par l'utilisateur, 7 octobre 2026)

Le chef envoie chaque mission à l'agent concerné avec `scripts/equipe.sh send <agent> - <<'EOF' ... EOF`, en
rappelant le périmètre et le critère de fin. Un agent n'est « fini » que quand ce critère est vérifié.

## design : menu d'accueil
Créer un **menu d'accueil** qui donne accès à toutes les fonctionnalités de l'outil : nouvelle partie contre l'IA,
création et édition de deck, mes decks, mes parties enregistrées, replays, puzzles, aide/règles de la table.
Navigation claire et retour au menu depuis partout. Ensuite : tout responsive (téléphone ≈ 360 px et grand écran) et
meilleure utilisation du simulateur (voir `.claude/agents/design.md`).
Fin : parcours « menu -> nouvelle partie -> partie jouée jusqu'au bout » et « menu -> éditeur de deck -> partie avec ce
deck » joués dans un vrai navigateur (Playwright, `train/dk2.py`) en format téléphone et ordinateur, captures jointes.
Sans la base claude.ai (GitHub Pages) tout doit fonctionner.

## cartes : toutes les cartes fidèles aux règles
Objectif : **toutes les cartes respectent le texte imprimé et les Core Rules** (`riftbound/rules/source/core_rules_2026-07-16.txt`).
Méthode : lire le texte de chaque carte (`riftbound/cards/cards_unique.csv`, colonne `text`), puis comparer à
son implémentation ; chercher en priorité les formulations qui changent ce qu'on a le droit de jouer :
- **« up to » / « any number »** (19 et 13 cartes) : règle 355.13, on peut choisir zéro cible et la carte se joue quand
  même, y compris la suite de l'effet. Exemple de l'utilisateur : **Shuriken Flip** (« Deal 2 to up to one enemy unit at a
  battlefield, then move a friendly unit »). Vérifié le 7/10 : cette carte se joue déjà sans cible (avec ou sans ennemi),
  mais refaire le test pour **chacune** des cartes « up to » / « any number » : jouable sans cible, la deuxième partie de
  l'effet s'applique quand même, les cartes sans cible possible ne bloquent pas la partie ;
- **« you may »** (247 cartes) : le choix est offert, y compris de ne pas le faire, sans bloquer le reste de l'effet ;
- **« then »** (64 cartes) : la suite se fait même si la première partie n'a rien fait (sauf texte contraire « if you do ») ;
- coûts optionnels et additionnels, réactions, remplacement de mort, copies, priorité, showdown, ordre des déclencheurs.
Pour chaque écart : un test qui échoue d'abord, la règle citée, le correctif, puis la suite entière.
Fin : tableau `riftbound/engine/cardsets/AUDIT_TEXTE.md` (carte, formulation à risque, test, statut) sans carte
« à vérifier » restante, `test_all.py` vert, `fuzz_cards.py` sans exception, 30 parties `t_rand.py` sans erreur.
Prévenir le chef de tout correctif de règle (les chiffres de simulation d'avant ne sont plus comparables).

## ia : IA globale, avec recherche de ressources
Objectif : une IA qui joue **n'importe quel deck** avec une idée de jeu et soit la plus performante possible.
Commencer par **chercher des ressources** : guides de jeu, articles de stratégie Riftbound (rôles aggro / tempo / contrôle,
courbe d'énergie, quand attaquer ou défendre, gestion de la main, choix de battlefield, mulligan), analyses de
tournois, vidéos (via transcriptions collées par l'utilisateur, YouTube est souvent bloqué), et pour la méthode, des
travaux d'IA de jeux de cartes (recherche arborescente avec information cachée, évaluation, réglage par auto-jeu).
Règles : citer la source de chaque idée dans `riftbound/gameplans/` (URL, date) ; ne pas copier de texte long ;
une idée de guide n'est **qu'une hypothèse** tant qu'elle n'est pas mesurée (graines neuves, ± écart-type, « on ne sait pas »
sous 2 écarts-types, une seule version du moteur).
Livrables : (1) note de synthèse des ressources et des principes retenus ; (2) un plan **déduit du deck** (rôle, cartes
clés, courbe) pour toute légende ; (3) un banc d'essai : N decks légaux aléatoires, IA générale contre l'ancienne,
paires de graines, intervalles ; (4) réglage des paramètres par auto-jeu, confirmé sur graines neuves ;
(5) temps par coup mesuré dans le navigateur (Pyodide). Replays des parties instructives après chaque simulation.
Fin : gain annoncé seulement s'il dépasse 2 écarts-types sur graines neuves, avec le chiffre et l'intervalle.

## retex (à faire par le chef et l'agent retex)
Après chaque mission terminée : `/retex` de la session (ce qui tient, ce qui était surévalué, biais, ce qu'on change),
mémoire mise à jour (`docs/memoire/`), chiffres périmés signalés dans `docs/vault/Chiffres périmés.md`.
