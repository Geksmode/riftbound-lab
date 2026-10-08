---
titre: Boucle des parties de l'utilisateur
resume: comment une partie jouée sur la table est gardée, déposée, rejouée fidèlement et analysée
maj: 2026-10-08
sources: riftbound/train/src.html, riftbound/engine/train_games.py, riftbound/engine/bilan_parties.py, .github/workflows/apprentissage.yml
---
1. **Jouer** : sur claude.ai la table enregistre dans sa base (collection `games`) ; ailleurs (GitHub Pages) elle garde les parties
   dans le navigateur (`localStorage`, 60 au plus, sans le journal). Chaque partie note `commit` et `engine` de la construction (build.sh).
2. **Déposer** : menu → Mes parties → « Télécharger mes parties » → fichier JSON à ajouter dans `riftbound/train/games/games/`.
3. **Rejouer** : `train_games.py <fichiers> --frozen --coach` rejoue chaque partie avec le moteur exact de son commit (`git archive`),
   sinon avec les copies figées `train/engine_versions/v10..v12` puis le moteur actuel ; le premier rejeu fidèle est retenu.
   `RB_ANALYSES=<dossier>` écrit ailleurs que `train/analyses/`.
4. **Bilan** : `bilan_parties.py` → `train/analyses/BILAN.md` (résultats avec intervalle de Wilson, parties inachevées et biais de
   sélection, décisions comparées au coach, rythme).
5. **CI** : le workflow « Apprentissage » fait 3 et 4 à chaque dépôt de parties et publie le bilan dans le résumé du run (artefact `analyses-parties`).

Vérifié le 2026-10-08 : les 17 parties d'avant se rejouent toutes fidèlement (moteurs figés v10, v11, v12) ; une partie jouée
dans Chromium sur la table sans base, téléchargée puis rejouée : fidèle avec le moteur du commit `b2cbe283361d`.
Pas encore fait : le modèle de tes choix utilisé par l'IA de LeBlanc (à décider, voir `etat.md`).
