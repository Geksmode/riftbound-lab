---
name: plans-par-legende
description: 2026-10-11 : un plan de jeu sourcé pour chacune des 49 légendes des decks par défaut (engine/plans_legendes.py, fiches gameplans/legendes.md), hypothèses non mesurées, activées seulement par « @plan=leg » dans exp_search
metadata:
  type: project
---
Demande de l'utilisateur (2026-10-11) : « chercher chaque gameplan pour chaque légende par défaut ». Il fait les mesures
lui-même, en local (plus rapide).

**Fait :** recherche web le 2026-10-11. Seul riftbound.gg était lisible avec WebFetch (mobalytics, riftmana, tcgplayer,
cardsrealm, reddit, metafy, piltoverarchive : refusés par le proxy). 40 légendes ont un guide riftbound.gg (auteur Den) lu en
entier. Pour 3 légendes (Fiora, Garen, Sivir), le guide décrit un autre archétype que le deck par défaut : le plan est déduit
du deck. Pour 6 légendes (Ivern, Poppy, Renekton, Shen, Vi, Zed), on n'a que des résumés de recherche et des listes. Les guides
donnent surtout les cartes clés (en codes, traduits avec `cards_unique.csv`) et rarement le mulligan.

**Code :** `engine/plans_legendes.py`, table `LEG` (axe conquérant / tenant, style, `garde` pour le mulligan, `cles` valorisées
en jeu, source, date). `LegendPlan` agit sur le mulligan et `shape`. Les poids sont choisis sans mesure (`RB_LEG_*`). Akali et
LeBlanc gardent Gorica et Hook tempo. Rien n'est branché dans `train.py` ni dans la table.
Mesure : `exp_search.py 80 <graines neuves> "sh@plan=leg" sh defaut`, puis une confirmation sur d'autres graines neuves.
Tests : `cardsets/test_plans_legendes.py`. Les fiches `gameplans/legendes.md` sont générées par `python3 plans_legendes.py`.

**À ne pas oublier :** aucune partie de ce plan n'est mesurée. Le choix de battlefield (`Plan.battlefield`) n'est pas codé : en
mode defaut, le battlefield est tiré au hasard. 18 decks par défaut sur 49 ont encore une carte ou un battlefield banni depuis (Stacked Deck, Ekko, Recurrent, The Arena's Greatest,
Aspirant's Climb), signalés par `train._banned`. Les plans suivent ces listes telles quelles.
Related: [[riftbound-gameplans]], [[ia-tests-decks-varies]], [[riftbound-search-ai]].
