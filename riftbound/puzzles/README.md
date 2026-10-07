# Puzzles sur plateau

Plateau interactif publié : https://claude.ai/artifact/Ds4TSVoR6tcNRbQKBKYpcP

- `engine.js` : mini-moteur JS (chaîne, showdown, combat, contrôle, runes), règles reprises de `../engine/game.py`.
  L'IA de Yi (`aiChoose`) est un minimax exact qui ne voit ni ta main, ni ta défausse, ni tes cartes cachées.
  Aucune restriction ajoutée (demande du 2026-10-04) : priorité gardée après avoir joué, cibles libres selon le
  texte, Hidden, Accelerate, Flow, réduction du Heron, légende d'Akali (Empower, repli), ordre de répartition
  des dégâts choisi par le joueur, bouton d'annulation.
- `puzzles.js` : les puzzles (position de départ, objectif `goal`, solution, leçon).
- `test.js` : solveur, vérifie que chaque puzzle a une solution et liste les premiers coups gagnants/perdants
  (`node test.js [id]`). `trace.js <id> [choix]` : déroule une partie avec l'IA ; `trace2.js <id> [coup]` : ligne full-info.
- `ui.html` + `build.py` : interface ; `python3 build.py` produit `plateau-yi.html` (fichier publié).

Ajouter un puzzle : une entrée dans `puzzles.js`, les cartes manquantes dans `engine.js`, puis `node test.js <id>`
doit dire SOLVABLE et les pièges doivent perdre.

## Plateau façon simulateur (v4)
`table.html` est l'interface façon TCG Arena / RiftAtlas : tapis, vraies images des cartes, unités et runes
épuisées tournées, main en éventail, aperçu en grand au survol, glisser-déposer pour déplacer ou jouer une unité.
`python3 build_table.py` injecte engine.js, puzzles.js et la table des images, et écrit index.html + img/ dans le
scratchpad ; publier avec `files` pour les images. `ui.html` / `build.py` restent l'ancienne version sans images.
