---
titre: Porte d'intégration
resume: le contrôle unique à passer avant d'intégrer une branche (agent ou travail) et avant main
maj: 2026-10-08
sources: scripts/verifier.sh, riftbound/train/verif_mobile.mjs, .github/workflows/ci.yml
---
`scripts/verifier.sh [moteur|table|tout]` (défaut `tout`) imprime un résumé OK / ÉCHEC et sort avec le code 1 au moindre échec.

| Étape | Ce qui est vérifié |
|---|---|
| wiki | `scripts/wiki_lint.py` : en-têtes, longueur, liens, index |
| tests | `riftbound/engine/test_all.py` : toute la suite |
| fuzz | `fuzz_cards.py` sur `cards.py` et chaque paquet de `cardsets/` (`RB_FUZZ` parties, défaut 200) : 0 exception |
| hasard | `train/t_rand.py` (`RB_RAND` parties, défaut 30) avec des decks légaux au hasard : 0 erreur |
| table | `fetch_pyodide.sh` si besoin + `build.sh` |
| cache | `train/verif_hidden.mjs` : Back Off en main avec un battlefield contrôlé, cachée au doigt (menu « Cacher ») et en la glissant sur le battlefield, puis jouée depuis la face cachée au tour suivant pour 0 énergie, par vrais clics ; enfin Bandle Tree : deux cartes cachées au même battlefield affichées côte à côte (sans se recouvrir, la 2e touchable et jouable) et deux dos de carte côté adverse (12 contrôles en tout). La situation de départ est posée dans le moteur. Puis `train/verif_revele.mjs` (6 contrôles) : Sabotage jouée au doigt, main adverse visible pendant le choix et après, recachée au coup suivant. Puis `verif_charge`, `verif_cibles`, `verif_flow` et `train/verif_vs.mjs` (18 contrôles) : écran de sélection contre l'IA (légendes grisées sans deck, deck par défaut, « ? », partie lancée avec les bonnes légendes, tout tient à l'écran). Puis `verif_fleches.mjs` (flèches), `verif_predict.mjs` (Predict / Vision) `verif_replay.mjs` (mode replay de « Mes parties » : rejeu fidèle, navigation, analyse, retour à la partie en cours), `verif_tuto.mjs` (tutoriel) et `verif_dash.mjs` (tableau de bord du menu : aucun deck par défaut, choix légende puis liste, « Jouer avec ce deck », ordinateur et téléphone). |
| match | `train/verif_match.mjs` (79 contrôles, ~1 min ici) : sideboard dans l'éditeur, BO3 manche 1 puis 2 par vrais clics. Sauté avec `RB_MATCH=0`. |
| duel | `train/verif_duel.mjs` (39 contrôles) : deux navigateurs, serveur PeerJS local, manche jouée et synchronisée, reprise. Sauté avec `RB_DUEL=0` ou sans le paquet `peer`. |
| navigateur | `train/verif_mobile.mjs` (Playwright, Chromium, 360×740) : menu (plus de lien Replays : on revoit ses parties dans « Mes parties »), partie jouée jusqu'au bout, bulle de fin au-dessus de la barre, « Nouvelle partie » au doigt, boutons ± ≥ 44 px, pas de défilement horizontal, aucune erreur JS. Sauté si Playwright est absent ou `RB_NAV=0`. |

- Durée avec les réglages par défaut : environ 6 min (2026-10-08, machine cloud 4 cœurs).
- CI : `tests` = `verifier.sh moteur` ; `build-table` installe Playwright 1.56.1 et lance `verifier.sh table` ; publication sur Pages seulement si les deux passent. La CI tourne sur toutes les branches, `agent/*` comprises.
- Testé : erreur injectée (fichier de mémoire orphelin) : code 1. Limite : dans le robot, les coups sont injectés (`T.act`), pas cliqués sur les cartes.
- Règle d'équipe : le chef ne fusionne une branche `agent/*` qu'après `verifier.sh tout` vert sur la branche fusionnée.
