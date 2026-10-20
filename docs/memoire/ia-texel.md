---
name: ia-texel
description: Réglage Texel des poids ai.EV (2026-10-20) : 2 004 parties sh ; 50,6 % ± 2,9 sur 80 paires, puis GARDÉ par deux tests séquentiels (53,5 % ± 0,9 regroupé) ; outils dans engine/texel/
metadata:
  type: project
---
**Demande** (2026-10-20) : « ajuster les poids pour que le bot soit meilleur », « pas NNUE ? » → réglage Texel (régression
logistique position → gagnant final), « le plus fiable possible même si ça prend 2 h ».

**Outils** (`riftbound/engine/texel/`, hors de `engine/*.py` : ne changent pas `engine_md5`) :
- `features.py` : `evaluate()` en forme linéaire `const + Σ w_k f_k` (17 clés ; `pts_ramp`, `react_kw` restent à 0).
  `test_texel.py` : égalité exacte avec `ai.evaluate` (93 192 évaluations, écart max 2e-13) et symétrie. **Toute modification
  d'`ai.evaluate` doit être reportée dans `features.py`** (sinon le test échoue).
- `gen.py <parties> <graine0> sh|pol <sortie>` : parties comme `exp_search` mode `defaut`, une position par tour à la première
  décision « main » (là où la recherche évalue ses feuilles), vue par le joueur actif. Reprise possible. Données dans
  `texel/data/` (ignorées par git, régénérables avec les graines).
- `fit.py <données>` : Newton avec recherche de pas (sans elle, divergence K → 1e11 sur 9 493 parties), rappel vers les poids
  actuels choisi par validation croisée **par partie**, contrôle sur 20 % des parties (graine % 5 == 4), bootstrap par partie.
  Résultats : `texel/results_texel.json`.

**Débit** (i7-14700HX, 24 processus) : sh contre sh ≈ 870 parties/h (≈ 74 s par partie et par processus en charge, 17 s seul) ;
politique simple 0,06 s par partie seule.

**Données sh** : 2 004 parties, graines 3100000-3102016, moteur 599127a907, ≈ 13,5 positions par partie.
Ajustement : log-loss contrôle 0,604 → 0,548 (écart par partie −0,76 ± 0,16) : **prédit nettement mieux le gagnant**.
Poids appris (± bootstrap) : pts 6,9 ± 0,8 ; pts_hi 10,4 ± 2,0 ; bf 1,1 ± 1,3 ; fd 5,4 ± 2,3 ; hold_win 4,4 ± 2,6 (au lieu de 40) ;
unit0 5,4 ± 1,0 ; might 3,1 ± 0,4 ; cost 0,44 ± 0,26 ; on_bf −4,6 ± 1,0 ; card0 −0,2 ± 0,8 ; card_e 1,03 ± 0,18 ; react 1,0 ± 1,0 ;
rune 6,6 ± 0,6 ; xp 1,1 ± 1,1 ; trick 2,4 ± 1,3 (leg_emp, deck_low : trop rares, gardés). Critères corrélés entre eux : un poids
seul ne se lit pas comme « la valeur » de la chose (ex. on_bf négatif à côté d'unit0 et might élevés).

**Mesure en partie** (`exp_search.py 80 3300000 "sh@<poids appris>" sh defaut`, moteur 599127a907) : **50,6 % ± 2,9** (z 0,22),
temps par décision 1,9 s contre 1,7 s. Sous 2 écarts-types : on ne sait pas (voir les tests séquentiels plus bas).
Prototype en politique simple (9 493 parties, 3000000+) : poids différents (bf −6,6, hold_win 9,6) : les poids dépendent de
la façon de jouer ; non mesuré en partie.

**Tests séquentiels (règle 9, [[ia-sprt]], moteur 599127a907, worktree propre de main)** :
- SPRT 1, graines 3500000-3500623 : **gardé** après 624 paires, 54 min de calcul, 53,1 % ± 1,2 (LLR 2,97) ;
- SPRT 2 (confirmation), graines 3600000-3600407 : **gardé** après 408 paires, 29 min, 54,0 % ± 1,4 (LLR 3,16) ;
- regroupé (`compare.py --pool`, 1 032 graines) : **53,5 % ± 0,9** ; avec les 80 paires du criblage : 53,3 % ± 0,9.
  Un SPRT s'arrête quand il franchit sa borne : ces moyennes surestiment un peu le vrai gain.
- Le criblage de 80 paires (50,6 % ± 2,9) n'avait pas la puissance de voir +3 points : « on ne sait pas » était juste, pas « aucun effet ».
- **Par défaut dans `ai.EV` depuis le 2026-10-20** (demande de l'utilisateur) ; anciens poids dans `ai.EV_OLD`, `RB_EV=old` les remet.
  Tests ajustés : `point_value_ramp_keeps_total_and_orders_points` (formule historique avec `EV_OLD`),
  `evaluation_weights_default_and_override` (plus de `bf == 3.0` en dur).
- **Temps par décision** : sur les mêmes 144 positions (`exp_temps.py 777003,777011`, deux passages en ordre inverse),
  aucun écart : CPython ≈ 136 ms (p95 ≈ 390 ms), Pyodide 0.26.4 sous node ≈ 295 ms (p95 ≈ 850 ms), anciens comme nouveaux.
  **En partie**, les nouveaux poids réfléchissent ≈ 20 % plus longtemps (40 paires, 3700000+, 24 processus en charge :
  708 ms contre 592 ms, p95 2,58 s contre 2,17 s ; criblage : 1,9 s contre 1,7 s) : même coût de calcul, positions plus
  chargées. Non mesuré : sur un vrai téléphone. Ce bloc de 40 paires (60,0 % ± 4,5) servait au temps, pas de preuve.
- Non mesuré : effet sur Akali contre LeBlanc ; comportement dans la table.

**Ce que ça indiquait avant les tests séquentiels** (hypothèse, pas mesurée) : avec le réglage à la main du 2026-10-09 (chaque poids seul ±3 %), deuxième
signe que les poids de l'évaluation ne sont pas le goulot de l'IA ; plus probablement le bruit de la recherche (1 tirage par
option) ou la politique de simulation. Une mesure de 80 paires ne voit qu'un écart d'environ 6 points ou plus.

Related: [[riftbound-search-ai]], [[ia-tests-decks-varies]].
