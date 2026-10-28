---
titre: Boucle des parties de l'utilisateur
resume: comment une partie jouée sur la table est gardée, déposée, rejouée fidèlement et analysée
maj: 2026-10-28
sources: riftbound/train/src.html, riftbound/engine/train_games.py, riftbound/engine/bilan_parties.py, .github/workflows/apprentissage.yml
---
1. **Jouer** : sur claude.ai la table enregistre dans sa base (collection `games`) ; ailleurs (GitHub Pages) elle garde les parties
   dans le navigateur (`localStorage`, 60 au plus, sans le journal). Chaque partie note `commit` et `engine` de la construction (build.sh).
2. **Déposer** : menu → Mes parties → « Tout télécharger », ou ⤓ sur une partie pour elle seule (`partie-riftbound-<date>-donne-<n>.json`, liste d'une partie) → fichier JSON à ajouter dans `riftbound/train/games/games/`. Les parties sont repérées par leur id : déposer une partie seule puis le fichier complet ne crée pas de doublon (`bilan_parties.py`).
3. **Rejouer** : `train_games.py <fichiers> --frozen --coach` rejoue chaque partie avec le moteur exact de son commit (`git archive`),
   sinon avec les copies figées `train/engine_versions/v10..v12` puis le moteur actuel ; le premier rejeu fidèle est retenu.
   `RB_ANALYSES=<dossier>` écrit ailleurs que `train/analyses/`.
4. **Bilan** : `bilan_parties.py` → `train/analyses/BILAN.md` (résultats avec intervalle de Wilson, parties inachevées et biais de
   sélection, décisions comparées au coach, rythme).
5. **CI** : le workflow « Apprentissage » fait 3 et 4 à chaque dépôt de parties et publie le bilan dans le résumé du run (artefact `analyses-parties`).

Vérifié le 2026-10-08 : les 17 parties d'avant se rejouent toutes fidèlement (moteurs figés v10, v11, v12) ; une partie jouée
dans Chromium sur la table sans base, téléchargée puis rejouée : fidèle avec le moteur du commit `b2cbe283361d`.
Pas encore fait : le modèle de tes choix utilisé par l'IA de LeBlanc (à décider, voir `etat.md`).

**Coups de l'IA et erreurs marquées (2026-10-28)**. La table enregistre aussi les coups de l'IA (champ `ai` : chaque
décision `["d", indice, type, n]` et chaque réponse à un ask `["a", codage, type, n]`, `train.ai_script`) : le rejeu
(`train_games.py`, « Revoir » de la table) suit ces coups (`train.ScriptAI`) et reste fidèle même si l'IA a changé.
Bouton « ⚑ Erreur de l'IA » (sous Conseil et Reprendre ; drapeau seul sur téléphone) : marque le dernier **vrai choix**
de l'IA (au moins deux options) avec une raison (mauvaise cible, mauvais moment, coup inutile, autre ; « autre » demande
une précision écrite), coup `["flag", raison, tour, indice dans ai, texte du coup, précision]`. Banc humain :
`python3 engine/banc/humain.py <parties> --ver turn --ver "turn@<réglage>"` rejoue chaque coup marqué et 20 coups
témoins par partie, et dit si chaque version y joue autre chose (proportions avec intervalle de Wilson).
Vérifié dans le navigateur intégré (Chromium, ordinateur et 360×740 par robot, coups injectés ; une marque faite au vrai
clic par l'utilisateur sur ordinateur) ; robot `train/verif_drapeau.mjs` écrit pour la porte, **pas encore lancé**
(Playwright absent de ce PC). Parties d'avant le 2026-10-28 : pas de champ `ai`, pas de banc humain.
