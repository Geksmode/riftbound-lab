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
| navigateur | `train/verif_mobile.mjs` (Playwright, Chromium, 360×740) : menu, lien Replays et lecteur, partie jouée jusqu'au bout, bulle de fin au-dessus de la barre, « Nouvelle partie » au doigt, boutons ± ≥ 44 px, pas de défilement horizontal, aucune erreur JS. Sauté si Playwright est absent ou `RB_NAV=0`. |

- Durée avec les réglages par défaut : environ 6 min (2026-10-08, machine cloud 4 cœurs).
- CI : `tests` = `verifier.sh moteur` ; `build-table` installe Playwright 1.56.1 et lance `verifier.sh table` ; publication sur Pages seulement si les deux passent. La CI tourne sur toutes les branches, `agent/*` comprises.
- Testé : erreur injectée (fichier de mémoire orphelin) : code 1. Limite : dans le robot, les coups sont injectés (`T.act`), pas cliqués sur les cartes.
- Règle d'équipe : le chef ne fusionne une branche `agent/*` qu'après `verifier.sh tout` vert sur la branche fusionnée.
