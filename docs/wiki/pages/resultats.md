---
titre: Chiffres versionnés (résultats de simulation)
resume: chaque résultat porte la version du moteur et ses graines ; compare.py applique les règles de comparaison
maj: 2026-10-08
sources: riftbound/engine/version.py, riftbound/engine/compare.py, riftbound/engine/cardsets/test_resultats.py
---
- **Version du moteur** : `version.engine_md5()` = MD5 (10 caractères) de tous les `.py` de `engine/` et `cardsets/`, même algorithme
  que `manager/run_session.py` (les sessions du manager restent comparables entre elles). Il change à chaque modification de fichier,
  commentaires compris : prudent ; une équivalence se prouve avec `manager/eqtest.py`.
- **Étiquetage** : `version.stamp(résultat)` ajoute `engine_md5` et `seeds` (première, dernière, nombre). Appelé par `exp.py`
  (qui écrit maintenant `per_seed`), `exp_gorica.py`, `exp_plans.py`, `exp_general.py` ; le manager écrit `engine_md5` en tête de session.
- **Comparer** : `python3 compare.py A.json:SEL B.json:SEL` (SEL = index ou morceau du label ; fichiers de session du manager acceptés).
  Mêmes graines → écart apparié ; graines disjointes → écart indépendant ; donne ± écart-type, z, et « on ne sait pas » sous 2 écarts-types.
- **Regrouper** : `python3 compare.py --pool A B ...` : seulement des blocs de même version et sans graine commune.
- **Refus (code 3)** : résultat sans version (anciens fichiers : à remesurer), versions différentes, graines en partie communes, regroupement de blocs qui partagent des graines.
- Vérifié le 2026-10-08 : 10 tests (`cardsets/test_resultats.py`) ; essai sur `manager/results/session_018.json` (comparaison appariée sur 400 graines) et refus entre sessions 16 et 18 (versions différentes).
- Les fichiers `results*.json` d'avant le 2026-10-08 n'ont pas de version : `compare.py` les refuse, ce qui applique la règle « remesurer avant de citer ».
