---
titre: Chiffres versionnés (résultats de simulation)
resume: chaque résultat porte la version du moteur et ses graines ; compare applique les règles de comparaison
maj: 2026-11-24
sources: rust/src/outils/version.rs, rust/src/outils/compare.rs, rust/src/bin/compare.rs
---
- **Version du moteur** : `engine_md5()` (`rust/src/outils/version.rs`) = MD5 (10 caractères) de tous les `.rs` de `rust/src/` hors
  `src/bin/` (calculé à la compilation par `build.rs`), suivi du suffixe **`-rust`** (ex. `3f9c0a12be-rust`) depuis la bascule du
  2026-11-24. Il change à chaque modification d'un fichier, commentaires compris : prudent. Les sessions du manager
  (`run_session`) écrivent la même version et restent comparables entre elles.
- **Moteur Rust contre moteur Python** : une version sans `-rust` est celle du moteur Python supprimé (dernier état : étiquette git
  `moteur-python-final`). `compare` refuse de comparer ou de regrouper les deux : les chiffres d'avant la bascule restent lisibles
  mais ne s'agrègent pas avec ceux d'après (même règle que pour deux versions du moteur Python).
- **Étiquetage** : `exp`, `exp_gorica`, `exp_plans`, `exp_general` ajoutent `engine_md5`, `per_seed` et `seeds` (première, dernière,
  nombre) à chaque résultat ; `run_session` écrit `engine_md5` en tête de session.
- **Comparer** : `rust/target/release/compare A.json:SEL B.json:SEL` (SEL = index ou morceau du label ; fichiers de session du manager
  acceptés). Mêmes graines → écart apparié ; graines disjointes → écart indépendant ; donne ± écart-type, z, et « on ne sait pas » sous 2 écarts-types.
- **Regrouper** : `compare --pool A B ...` : seulement des blocs de même version et sans graine commune.
- **Refus (code 3)** : résultat sans version (anciens fichiers : à remesurer), versions différentes (dont Rust contre Python), graines en partie communes, regroupement de blocs qui partagent des graines.
- Vérifié le 2026-10-08 (outil Python d'alors) : 10 tests ; essai sur `manager/results/session_018.json` (comparaison appariée sur 400 graines) et refus entre sessions 16 et 18 (versions différentes). Le refus Rust contre Python est codé dans `outils/version.rs` (`is_rust`).
- Les fichiers `results*.json` d'avant le 2026-10-08 n'ont pas de version : `compare` les refuse, ce qui applique la règle « remesurer avant de citer ».
