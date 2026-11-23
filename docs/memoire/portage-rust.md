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

## Architecture (fixée le 2026-11-23, à suivre pour que le différentiel tienne)
- **Traduction au plus près** de la structure Python : mêmes fonctions, même ordre d'opérations, mêmes tris (le tri
  Rust `sort_by` est stable comme `sorted`), mêmes listes ordonnées (jamais de `HashMap` dont l'ordre compte :
  `Vec` ou `BTreeMap`). Le nom de la fonction Python est cité au-dessus de chaque fonction Rust.
- **Objets** : arène `Vec<Obj>` dans `Game`, poignée `O` (indice) ; les zones sont des `Vec<O>`. L'`uid` reste un
  champ (ordre de tri, cibles, libellés) et suit **le même compteur que Python** : `Obj._n` est global au processus
  et les copies de l'IA le font avancer, donc en Rust le compteur est partagé entre une partie et ses copies
  (`Arc<AtomicU32>`). Le harnais différentiel Python remet `Obj._n` et `Item._n` à 0 avant chaque partie.
- **Valeurs dynamiques** (`data` des items, `info` des évènements, options de `g.ask`, entrées de `g.effects`) :
  enum `Value` (None, Bool, Int, Float, Str, Obj, Rune, Loc, List, Map ordonnée, Opt, Fn), pour traduire les cartes
  sans les réécrire.
- **Comportements** : les champs de `Impl` qui sont des fonctions deviennent `Option<Arc<dyn Fn ... + Send + Sync>>`
  (fermetures immuables, partagées entre les copies). `Item.fn`, les `effects[*].fn`, prédicats de cible idem.
- **Choix** : `g.ask(pid, kind, options, ctx)` reste synchrone ; les agents sont des `Box<dyn Agent>` sortis de la
  partie le temps de l'appel.
- **Hasard** : `pyrandom::PyRandom` (identique à CPython).
- **Différentiel** : le Python écrit pour chaque `ask` et chaque décision (type, libellés des options, indice choisi)
  et un résumé d'état à chaque décision ; le Rust doit produire le même fichier, octet pour octet.

## Conventions de traduction (pour porter un module de cartes)
- Une fonction Python `f(g, a, b, ...)` rangée dans une carte devient `Value::func("nom", |g, a| ...)` : `a` contient
  les arguments **après** `g`, dans l'ordre Python (`a[0]`, `a[1]`...), et la fonction renvoie `Res<Value>`
  (`Err(GameOver)` remonte une fin de partie : toujours propager avec `?`).
- Objets : `Value::Obj(O)` ; battlefield passé à un crochet (`bf_event`, `providers` d'un battlefield) :
  `Value::Int(idx)` ; item : `Value::Item(ItemId)` ; effet de `g.effects` passé à son `fn` : `Value::Int(eid)`
  (`g.eff(eid)`, `g.eff_mut(eid)`, `g.remove_effect(eid)`) ; emplacement : `Value::Loc(Loc::Base | Loc::Bf(i))`.
- `info` d'un évènement : `Value::Map` (copie : un écouteur qui modifie `info` ne le fait pas pour les suivants).
  `info["sd"]` est un instantané (`bf, combat, attacker, defender, focus`). `lookback(o)["spec"]` est le **nom** de
  la carte (lire la spec avec `spec(nom)`).
- Clés entières de dict Python (`_oids`, `need` de `damage_pick`) : texte de l'entier dans une `Map`.
- `sd.assigned` : tuple de deux listes de `(objet, dégâts)`, dans l'ordre d'assignation.
- `sd.members` : ordre d'arrivée (Python : ensemble d'entiers itéré dans l'ordre de CPython) → point à aligner
  (voir « Divergences connues »).

## Divergences connues à aligner (le différentiel les montrera)
- `game.combat_step` (étape « end ») itère `sd.members`, un `set` d'uids : ordre de CPython, pas l'ordre d'arrivée.
  À corriger côté Python avec `sorted(sd.members)` (règle `CLAUDE.md` : jamais d'ordre de `set` qui compte), et côté
  Rust trier par uid.
- `Game.ask` : la mémoire du point de reprise (`checkpoint`, `_asklog`, `ai.ask_sim`) n'est pas encore portée.
- `Game._memo` (mémoire de génération des coups) : pas portée (optimisation sans effet sur le jeu, `empreinte.py`).
- Contexte `plans=` de `pay_gold` / `pay_rune` non passé aux agents (à faire avec `train.py`).

## Avancement
- 2026-11-23 : contrat, crate créée ; hasard identique à CPython (442 tirages) ; base de cartes identique (1111
  entrées) ; `game.py` entier porté (`rust/src/game*.rs`, compile sans avertissement, pas encore jouable :
  `actions.py`, aides de `cards.py`, modules de cartes, IA restent à porter).
