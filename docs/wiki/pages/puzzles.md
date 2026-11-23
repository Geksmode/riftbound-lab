---
titre: Puzzles de la table
resume: menu « Puzzles » : 6 positions Akali contre Master Yi montées dans le vrai moteur, objectif jugé par le moteur, solution et leçon
maj: 2026-11-23
sources: riftbound/engine/puzzles.py, riftbound/engine/train.py (puzzle_new, PuzzleAgent, TGame.next_turn), riftbound/train/src.html (puzzleStart, puzEnd), riftbound/engine/cardsets/test_puzzles.py, riftbound/train/verif_puzzle.mjs
---
Demande de l'utilisateur (2026-11-23, « fais les puzzles ») : les puzzles étaient prévus dans le menu (mission du 2026-10-07)
mais n'existaient qu'à part (`riftbound/puzzles/`, plateau publié en artefact) sur un **mini-moteur JS simplifié**.

**Ce qui est fait**
- Menu → « Puzzles » : liste des 6 puzzles (✓ = réussi dans ce navigateur, clé `rbt-puz`), énoncé et objectif avant de
  jouer, puce « Objectif » dans la barre du haut pendant la partie, écran de fin « Puzzle réussi ! » / « Puzzle raté »
  (solution et leçon ; cachées après un échec, bouton « Voir la solution »), Réessayer, Puzzle suivant, Tous les puzzles.
  Pas de « Conseil » pendant un puzzle ; « Reprendre » marche. Pas enregistré dans « Mes parties ». Anglais : textes `(fr, en)`
  dans `puzzles.py`, page dans `i18n.js`.
- **Vrai moteur, aucune règle simplifiée** : `puzzles.setup` monte la position dans un `TGame` (unités, mains, runes prêtes
  ou épuisées, contrôle des battlefields, carte cachée au tour précédent, runes du dessus du deck de Yi), puis la partie
  repart en phase principale. L'objectif est jugé par le moteur à la fin du tour fixé (`TGame.next_turn` : `g.puz_ok`,
  `winner` = toi si atteint). Les copies de l'IA s'arrêtent au même endroit : sa recherche joue **contre l'objectif**.
- IA de la table niveau Normal (`PuzzleAgent`, recherche « turn », `table_cfg`), avec l'énoncé imposé quand il en donne un
  (« il attaque Sigil avec Master Yi » : premier déplacement forcé, puzzle 4).
- Choix de modélisation (dans `puzzles.py`) : decks Akali G2 et Yi Bladesman (swagalisk) derrière la position ; dessus des
  deux decks = leurs cartes les plus chères (pas de réponse gratuite piochée) ; zones des champions vides (positions d'origine
  sans champion) ; tour 6, 2 points chacun ; même puzzle = même partie (graine fixe).

**Vérifié**
- `cardsets/test_puzzles.py` (6 tests) : pour chacun des 6 puzzles, la **solution annoncée gagne** contre l'IA de la table et
  **le piège de la leçon perd** ; position du puzzle 4 ; reproductible ; une partie normale après un puzzle n'en garde rien.
- Robot `train/verif_puzzle.mjs` (porte, partie 3) : téléphone 393×851 toucher simulé (liste, énoncé, Shuriken Flip ciblé dans
  la bulle, fin réussie, puzzle suivant raté par le piège, Voir la solution, Réessayer, ✓ dans la liste), ordinateur 1400×900
  souris (puzzle 4 au tour de Yi, Block en réponse à Punch First : coup injecté), liste en anglais.
  **Vérifié par robot, pas essayé sur un vrai téléphone.**

**Limites**
- Les pièges sont des lignes perdantes, pas une preuve que la solution est la seule : pas de solveur exhaustif sur le vrai
  moteur (l'ancien `puzzles/test.js` résolvait le mini-moteur JS, pas celui-ci).
- 6 puzzles seulement, tous Akali contre Yi. Ajouter un puzzle : une entrée dans `PUZZLES` (`engine/puzzles.py`), sa solution et
  son piège dans `LINES` (`cardsets/test_puzzles.py`).
