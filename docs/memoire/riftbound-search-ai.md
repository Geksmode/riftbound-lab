---
name: riftbound-search-ai
description: IA générale (SearchAgent) 2026-10-09 (niveau Très fort de la table 2026-10-27, en fin de page) : tirages communs + élimination en passes (RB_SEARCH, défaut "sh"), mesures sur les 49 decks par défaut, essais d'évaluation et de politique non retenus, temps par coup
metadata:
  type: project
---
**Demande** (2026-10-09) : « améliore l'IA de base pour tous les decks » (SearchAgent / rollout / evaluate de `engine/ai.py`, pas les plans).

**Changement gardé : la recherche** (`ai.SEARCH`, variable `RB_SEARCH`, défaut `"sh"` ; `"old"` = avant le 2026-10-09).
- `crn` (tirages communs) : le k-ième tirage est le même monde caché et le même aléa pour toutes les options d'une décision
  (avant, chaque option avait son propre tirage : on comparait surtout la chance des tirages).
- `sh` : `crn` + élimination en passes (successive halving) : la meilleure moitié reçoit des tirages en plus, jusqu'à 2 options ;
  budget en plus = `sh_extra` (1,0) × options × tirages.

**Mesures** (`engine/exp_search.py`, résultats `engine/results_search.json`, paires appariées : même donne jouée deux fois, places
échangées ; score = moyenne de la paire ; ± écart-type sur les paires). Mode `defaut` : deux decks par défaut (`train.default_decks`,
49 légendes) tirés au hasard, IA générique des deux côtés. Chaque ligne = graines neuves.

| Comparaison | Paires | Graines | Moteur | Score de A |
|---|---|---|---|---|
| sh contre old2 (même budget de simulations) | 200 | 610000+ | 4f90798d18 | **55,8 % ± 1,9** |
| crn contre old (même budget) | 160 | 630000+ | 4f90798d18 | **58,4 % ± 2,3** |
| sh contre old (niveau Normal réel) | 160 | 620000+ | 4f90798d18 | **65,3 % ± 2,3** |
| sh contre crn | 120 | 650000+ | 573611f39e | **58,8 % ± 2,8** |
| sh contre old, Akali G2 (Gorica) contre LeBlanc IQ#5 (Hook tempo) | 120 | 690000+ | 4feb2aae97 | **62,5 % ± 2,9** |

**Essais non retenus** (A = sh + réglage, B = sh ; criblage 80 paires, graines 660000+ ou 680000+) : horizon 1 tour
(`h=1`) 43,1 % ± 3,0 (pire, plus rapide) ; horizon 3 tours 51,2 % ± 2,7 ; sorts [Reaction] comptés comme cartes de réaction
dans la main (`react_kw=1`) 49,4 % ± 1,1 ; politique de simulation qui garde ses sorts [Reaction] (`pol_keep=1`) 51,2 % ± 2,9 ;
poids de l'évaluation `bf=5` 48,8 % ± 1,5, `pts=9` 49,4 % ± 2,1, `might=1.1` 51,9 % ± 2,4, `card0=1.9` 53,1 % ± 2,4 puis
**49,1 % ± 1,6 sur 160 paires neuves (670000+)** : on ne sait pas, rien de gardé. Les réglages restent disponibles
(`exp_search.py ... "sh@card0=1.9" sh defaut`, poids dans `ai.EV`, clés `pol_*` dans `cfg`), défaut inchangé.

**Temps par décision** (décisions à plusieurs options). Mêmes 100 positions (graines 777003 et 777011, `engine/exp_temps.py`, Pyodide : `node engine/exp_temps.mjs`) :
CPython old 267 ms (p95 1,5 s), sh 355 ms (p95 1,75 s) ; **Pyodide 0.26.4 sous node** old 417 ms (p95 2,4 s),
sh 503 ms (p95 2,4 s), crn 389 ms (p95 2,0 s). Dans les parties des simulations (CPython, 4 processus) : old ≈ 245 ms,
sh ≈ 360 ms (× 1,5). Le p95 vient surtout du mode « urgent » (6 tirages quand l'adversaire est à 6+).
Non mesuré : temps dans un vrai navigateur (téléphone compris).

**Ce qui reste inconnu** : effet sur les decks au hasard (`miroir`) ; niveau Fort (3 tirages) ; réglage de `sh_extra` ;
les pourcentages Akali contre LeBlanc publiés avant le 2026-10-09 ont été mesurés avec l'ancienne recherche (autre moteur).
Replays : groupes « IA générale » du lecteur (`ia-sh-620040a/b`, `ia-sh-akali-690001a/b`, `ia-h1-660006`).
**Sorts de combat et runes ouvertes (2026-10-10, décision de l'utilisateur, gardée quel que soit le résultat des mesures :
« ça simule le comportement humain » ; valeurs « je te fais confiance »).** Cause de départ : Maître Yi jetait Punch First
dans son tour sans combat derrière (sur 11 cas regardés, le sort ne servait qu'à 2). Une carte en main valait ≈ 1,45 quelle
qu'elle soit, le +5 « ce tour » ne compte pas (`lasting_might`), et un seul tirage par coup fait plus de bruit que ce coût.
- `ai.is_trick` : sort [Action]/[Reaction] (seuls jouables en showdown, 308.1.a) dont le texte change la Might « this turn »
  (31 cartes, malus compris). En main : `EV["trick"]` = 2,0 de plus avec une unité sur le plateau et assez de runes.
- La politique de simulation garde ces sorts pour les showdowns (`POL_TRICK`) : sans elle, la valeur en main disparaissait
  (elle les jetait dans son tour).
- `ai.open_count` / `EV["open"]` = 1,0 par sort [Action]/[Reaction] payable avec les runes **prêtes** au moment où la recherche
  du tour entier finit le tour (« end ») : runes gardées ouvertes pour le tour adverse. Seulement en mode `turn` (en `sh`, seul
  « end » recevrait le bonus).
- Recherche du tour entier : seules les suites complètes (« end », partie finie, profondeur) sont retenues ; avant, une suite
  d'un coup notée avec la politique qui finissait le tour gagnait (Punch First, puis fin du tour sans attaque).
- Valeurs choisies par raisonnement, **pas mesurées** : trick 2 (une unité tuée ≈ 2 à 5, un point 7) ; open 1 (moins qu'une unité
  posée). Retour arrière : `RB_TRICK=0` (trick, open, politique), `RB_TURN_FULL=0`.
- Mesures (decks variés, 80 paires, moteur 630b5d9a60) : suites complètes contre ancien choix 48,8 % ± 2,8 (810000+) ;
  trick=2 sans la politique contre rien 48,1 % ± 1,7 (811000+, Mac de l'utilisateur) : on ne sait pas. trick + politique
  (812000+) : pas reçu. Temps par décision en `turn` : 1,1 s au lieu de 1,4 s avec les suites complètes.
- Comportement (Yi Bladesman contre Akali G2, IA de la table, 40 parties, graines 900000-900039, nouveau réglage sans `open`) :
  Punch First 29 fois en showdown, 12 fois dans son tour (cible au combat 7 fois, inutile 5 fois). Ancienne IA
  (`RB_TRICK=0 RB_TURN_FULL=0`), mêmes graines : 19 en showdown, 30 dans son tour (inutile 12 fois). Les parties divergent
  après le premier coup différent : comptes non appariés, 40 parties seulement.

**Poids de l'évaluation** : depuis le 2026-10-20, `ai.EV` = poids Texel ([[ia-texel]], 53,5 % ± 0,9 contre les poids ci-dessus) ;
les mesures de cette page ont été faites avec les anciens poids (`ai.EV_OLD`, `RB_EV=old`).

**Niveaux de la table et niveau « Très fort » (2026-10-27, demande de l'utilisateur : « un niveau au-dessus, rendre l'IA
réellement plus forte » ; piste choisie : plus de budget de recherche).** La table joue `search="turn"` avec `samples` = niveau
(`train.new`, `train.LEVEL_CFG`) : Normal 1, Fort 3, Très fort 6 avec `turn_w=6` (faisceau de 6 suites) et `react_n=3`
(3 mondes seulement pour les décisions hors « main » : réactions, focus, cibles ; choix de l'utilisateur : « ok qu'il prenne son
temps pour optimiser son tour », pas pour un petit choix). Équivalent `exp_search` : `turn@n=6,turn_w=6,react_n=3`.
Coups d'œil (80 paires, mode `defaut`, moteur a832cbbc81, graines neuves à chaque ligne) :

| Comparaison | Graines | Score de A | ms par décision A / B (CPython, machine chargée) |
|---|---|---|---|
| Fort (`turn@n=3`) contre Normal (`turn`) | 4000000+ | 52,5 % ± 3,3 (on ne sait pas) | 4 371 / 2 173 |
| `turn@n=6` contre Fort | 4100000+ | 53,1 % ± 3,1 (on ne sait pas) | 7 859 / 5 017 |
| `turn@n=3,turn_w=6` contre Fort | 4200000+ | 53,1 % ± 3,0 (on ne sait pas) | 8 433 / 4 888 |
| `turn@n=6,turn_w=6` contre Fort | 4300000+ | 55,6 % ± 2,8 (z 2,0) | 12 076 / 4 335 |

Chaque levier seul rapporte peu (Fort ne bat pas Normal de façon visible) ; ensemble, l'écart devient visible.
Test séquentiel (règle 9, [[ia-sprt]]), `turn@n=6,turn_w=6,react_n=3` contre `turn@n=3`, moteur de la branche `ia/tres-fort`
(recherche accélérée, décisions identiques) : **SPRT 1, graines 4400000-4400295 : gardé après 296 paires, 55,2 % ± 1,7
(LLR 2,98), 179 min.** SPRT 2 (confirmation, graines 4500000+) : en cours au moment de l'écriture, voir `docs/wiki/log.md`.
Un SPRT s'arrête quand il franchit sa borne : la moyenne surestime un peu le vrai gain.
Table : partie jouée de bout en bout dans le navigateur intégré (Pyodide, donne 4242, coups humains au hasard pilotés par
script, machine chargée par 24 processus) : 12 tours, IA gagnante 8-3, attente la plus longue entre deux coups humains 22 s,
4 attentes > 10 s, médiane 0,2 s ; aucune erreur JS. Non essayé sur un vrai téléphone ; temps Pyodide par décision non mesuré
sur machine libre. Robot `train/verif_vs.mjs` : bouton, réglages reçus par le moteur, bandeau « IA très forte ».

Related: [[riftbound-tempo-ai]], [[riftbound-gameplans]], [[ia-texel]].
