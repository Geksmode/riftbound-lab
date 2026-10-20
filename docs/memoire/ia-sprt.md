---
name: ia-sprt
description: Règle de l'utilisateur (2026-10-20) : un réglage de l'IA est gardé s'il apporte +2 points démontré par test séquentiel (SPRT 50/52 %), confirmé sur graines neuves ; outil engine/mesure/sprt.py, durées réelles
metadata:
  type: feedback
---
Le 2026-10-20, après le réglage Texel (50,6 % ± 2,9 sur 80 paires, [[ia-texel]]), l'utilisateur a proposé que « chaque gain
soit à prendre en compte ». Proposition retenue (« j'aime bien l'idée ») : **on garde un réglage dès qu'il apporte +2 points ou
plus, démontré par un test séquentiel, puis confirmé sur des graines neuves.**

**Pourquoi :** les petits gains s'additionnent, mais garder tout écart positif mesuré sur 80 paires revient à garder du bruit
(± 2,9 points ; ex. card0=1.9 : 53,1 % puis 49,1 % sur graines neuves) ; le test séquentiel voit +2 points et s'arrête tôt
quand l'issue est claire.

**Application :**
- `python3 engine/mesure/sprt.py <graine0> "<A>" <B> defaut [processus] [max_paires]` : H0 50 %, H1 52 %, alpha = beta = 5 %,
  LLR normal (comme fishtest), lu toutes les 8 paires à partir de 40, plafond 1 600 paires -> « on ne sait pas ».
- « garde » -> second SPRT sur des graines **neuves** ; les deux doivent dire « garde » avant de changer le défaut.
- Durées (simulation, écart-type par paire 0,24, `mesure/test_sprt.py`), à ≈ 870 parties/h sur ce PC (i7-14700HX, 24 processus) :
  vrai 47 % -> ≈ 190 paires (≈ 26 min) ; 50 % -> ≈ 670 paires (≈ 1 h 30, p90 1 600 = 3 h 40) ; 52 % -> ≈ 650 (≈ 1 h 30) ;
  55 % -> ≈ 200 (≈ 28 min). Un réglage sans effet coûte donc environ 1 h 30, pas 20 min (première estimation, fausse).
- Vérifié de bout en bout le 2026-10-20 : `sh` contre `sh`, graines 3400000-3400039, rejet après 40 paires (toutes à 0,5), 283 s.
- Piège Windows : `exp_search.py` (et les autres `exp_*.py`) ouvrent `results_*.json` sans `encoding` -> cp1252 ; le
  2026-10-20 une entrée y a été écrite en cp1252 (fichier illisible en UTF-8, réparé). Lancer avec `PYTHONUTF8=1` ;
  `sprt.py` écrit en UTF-8.

Related: [[ia-tests-decks-varies]], [[retex-method]].
