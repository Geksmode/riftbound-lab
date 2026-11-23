---
resume: portage complet du moteur (règles, cartes, IA, table) de Python vers Rust, décidé par l'utilisateur le 2026-11-23
maj: 2026-11-23
sources: demande de l'utilisateur (2026-11-23, « pas de mesure, tout en Rust »), riftbound/engine/, rust/
---
# Portage du moteur en Rust

**Décision de l'utilisateur (2026-11-23)** : réécrire tout le « back » en Rust, sans mesure préalable. Ne pas re-discuter.

## Contrat
- **Livrable** : crate `rust/` (`riftbound`), même moteur que `riftbound/engine` (règles, 1084 cartes modélisées, IA de
  recherche, plans, match), compilé en natif (simulations) et en WebAssembly (table, à la place de Pyodide dans
  `riftbound/train/engine.worker.js`, même protocole de messages `{id, fn, args}`).
- **Contraintes** (`CLAUDE.md`) : aucune carte approximée (une carte non portée reste bloquée) ; même graine = même
  partie ; numéros de règle en commentaire ; une carte = au moins un test ; nouveau moteur = nouvelle version
  (`version.py`), aucun chiffre agrégé avec ceux de Python (`compare.py`).
- **Référence** : le moteur Python reste la référence tant que le portage n'est pas fini. Le Rust doit jouer
  **exactement** la même partie que Python sur les mêmes graines (test différentiel : journal des décisions et
  état à chaque décision identiques). Une divergence est un bug du portage, sauf bug prouvé du Python (corrigé
  des deux côtés).
- **Hors périmètre (pour l'instant)** : scripts d'analyse ponctuels (`exp_*.py`, `banc/`, `texel/`), qui peuvent
  appeler le moteur Rust plus tard par une liaison.
- **Critère de fin** : (1) tous les tests de scénario portés et verts (`cargo test`) ; (2) test différentiel vert sur
  N graines × decks variés (N à fixer, au moins 2 000 parties IA contre IA) ; (3) `scripts/verifier.sh` vert avec
  la table sur le moteur WebAssembly.

## Étapes
1. Hasard identique à Python (`random.Random` : MT19937, `randrange`, `shuffle`, `choice`, `sample`, `random`,
   `getrandbits`, `uniform`, `randint`), vérifié contre CPython.
2. État de partie, zones, runes et coûts, chaîne, combat, score (`game.py`, `actions.py`).
3. Registre des cartes et mots-clés génériques (`cards.py`), puis les paquets (`cardsets/`) un par un, chacun avec
   ses tests portés.
4. IA (`ai.py`, `plans*.py`), puis `train.py` / `match.py` (interface de la table) et la liaison WebAssembly.
5. Bascule de la table, nouvelle version du moteur, remesure des chiffres.

## Avancement
- 2026-11-23 : contrat, crate créée, étape 1.
