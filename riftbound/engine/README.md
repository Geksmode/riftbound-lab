# Moteur Riftbound fidèle (1v1)

Moteur de règles complet écrit d'après les **Core Rules du 16/07/2026** (`../rules/source/core_rules_2026-07-16.txt`,
récupérées sur app.riftjudge.com). Aucune simplification de carte : chaque carte du pool Akali/LeBlanc est codée
d'après son texte, avec les numéros de règle cités en commentaire.

## Fichiers
Depuis le 2026-11-24 le moteur, les cartes, l'IA et les outils sont en **Rust** (`rust/`, binaires dans
`rust/target/release/`, `cd rust && cargo build --release`). Le moteur Python est supprimé ; son dernier état est
l'étiquette git `moteur-python-final`. Dans ce dossier restent les données (`results*.json`, `empreinte.json`,
`pool.txt`, `exemple_partie.txt`), les guides de modélisation (`cardsets/GUIDE.md`, `NEEDS*.md`) et les mesures
(`banc/`, `texel/`).

| fichier Rust | contenu |
|---|---|
| `rust/src/game.rs`, `game_board.rs`, `game_costs.rs`, `game_events.rs`, `game_turn.rs` | état de partie, zones, runes et paiement des coûts, chaîne (priorité/focus), cleanups, showdowns, combat, contrôle, score, burn out, couches (layers) de calcul de might (ancien `game.py`) |
| `rust/src/actions.rs` | actions discrétionnaires légales et processus de jeu d'une carte / activation d'une capacité (règles 353-359, 398-406) |
| `rust/src/cards.rs`, `cards_helpers.rs` | registre `IMPL` (une entrée par carte et par token), aides génériques pour les mots-clés (`ability`, `parse_cost`, `add_ability`, `when_level`...), modélisation automatique des cartes « mots-clés seulement » (`auto_keywords`, liste `AUTO`), chargeur des paquets, contrôle de deck (`deck_problems` : non modélisée, [Unique]) |
| `rust/src/cardsets/*.rs` | cartes par paquet (`battlefields`, `body`, `calm`, `chaos`, `fury`, `legends`, `mind`, `order`, `radiance`, `pool`), enregistrés dans l'ordre de `cardsets/mod.rs` ; guides de modélisation restés ici : `cardsets/GUIDE.md`, `cardsets/NEEDS*.md` |
| `rust/tests/`, `rust/src/outils/fuzz.rs`, `rust/src/bin/fuzz_cards.rs` | tests (`cargo test --release`), parties aléatoires avec des decks contenant des cartes données (aucune exception, même graine = même partie), référence figée `rust/tests/golden.rs` (300 parties du moteur Python) |
| `rust/src/ai.rs`, `agents.rs` | agents : `SearchAgent` (recherche à 1 coup sur copies de la partie + déroulement par politique), `PolicyAgent`, agent aléatoire, évaluation, déterminisation de l'information cachée |
| `rust/src/decks.rs` | charge les 12 decklists de tournoi depuis `../decks/decks.json` |
| `rust/src/outils/` et `rust/src/bin/` | un binaire par outil : `exp`, `exp_search`, `sprt`, `compare`, `empreinte`, `t_rand`, `run_session`, `banc`, `banc_cout`, `banc_humain`, `texel_gen`, `texel_fit`, `quick` (partie unique avec journal), `train_games`... |
| `rust/src/replay.rs`, `replay_rec.rs`, `bin/batch_replays.rs`, `bin/curate_replays.rs` | enregistreur de replays (états + scores des options de l'IA) pour le lecteur `../replays/viewer.html` |
| `rust/src/plans.rs`, `plans_general.rs`, `plans_legendes.rs`, `bin/exp_plans.rs` | plans de jeu des joueurs (Akali Gorica moteur/agressif, LeBlanc Deathknell/Hook tempo) et leurs simulations (`results_plans.json`) ; sources dans `../gameplans/` |
| `rust/src/train.rs`, `match_.rs`, `lang.rs`, `wasm.rs` | table d'entraînement : API appelée par la page (humain, BO3, textes français et anglais), compilée en WebAssembly (`riftbound.wasm`, `../train/engine.worker.js`) |
| `exemple_partie.txt` | deux parties complètes tour par tour |
| `pool.txt` | texte, coût, might et mots-clés des 85 cartes du pool |

## Ce qui est modélisé
- **Tour** : Awaken, Beginning (effets de début, Temporary avant le score, puis Scoring/tenue), Channel 2 runes
  (3 au premier tour du joueur qui commence en second), Draw 1, Main, End (soin, fin des effets « ce tour »,
  vidage des pools).
- **Runes et coûts** : épuiser = 1 énergie, recycler = 1 power du domaine, la même rune peut faire les deux ;
  Gold (« [Reaction] Tue ceci, [E] : [Add] [A] ») utilisable pendant le paiement ; si un Gold prêt peut remplacer
  le recyclage d'une rune, le joueur choisit (question `pay_gold`, l'IA garde la rune recyclée comme avant) ; pools vidés au début de la Main
  Phase et en fin de tour ; coûts additionnels (Accelerate, Repeat, Deflect, Sacrifice, Atakhan), réductions
  (Astral Heron, Sky Splitter, Legion de Noxus Hopeful), coûts ignorés (Hidden, Baited Hook, Rift Herald).
- **Chaîne** : états Neutral/Showdown × Open/Closed, priorité et focus, FEPR, counters, cibles invalidées
  (mistargeting) revérifiées à la résolution, instructions liées, remplacements (Zhonya, Prevent, Burn Out).
- **Combat** : déclencheurs d'attaque/défense une seule fois par combat, showdown de combat, somme des might
  (unités stun exclues), assignation par l'attaquant d'abord avec létal complet, Tank d'abord / Backline en
  dernier, dégâts simultanés, cleanup de combat (soin + rappel des attaquants), résultat, prise de contrôle.
- **Score** : conquête et tenue une fois par battlefield par tour, règle du dernier point (il faut avoir scoré
  tous les battlefields, sinon on pioche), victoire à 8 points vérifiée en cleanup, Aspirant's Climb = 9.
- **Hidden** : payer [A] pour poser une carte face cachée sur un battlefield contrôlé (une seule par
  battlefield), jouable dès le tour suivant en Reaction sans payer son coût de base, choix restreints à ce
  battlefield, défaussée si on perd le contrôle.
- **Equipement** : Equip (Main Phase) et Quick-Draw (Reaction, attache à la pose), bonus de might de l'objet
  attaché, texte imprimé inactif, rappel à la base si l'unité meurt.
- **Mots-clés génériques** (règles 805-829, déclarés par les champs de `Impl`) : Action/Reaction (sorts, capacités,
  unités et équipements), Accelerate, Assault/Shield/Deflect N (sommés), Tank, Backline, Ganking, Hidden, Ambush,
  Temporary, Deathknell, Repeat (`repeat_cost`, `repeatable`), Flow (`flow_cost`), Equip (`equip=`, `bonus=`),
  Quick-Draw, Weaponmaster (équipe à la pose pour [A] de moins), Vision et Predict N (`g.predict`), Burn N
  (`g.burn`, burn out compris), Hunt N et Level N (XP : `g.gain_xp`, `g.spend_xp`, `levels=`), Empower/Empowered
  (`empower=`, `g.empower`, `when_empowered`), Legion (`g.legion`), Mighty (`g.mighty`, `when_mighty`, évènement
  `becomes_mighty`), Buff, Stun, Add (capacités utilisées pendant le paiement : `add=`), Unique (contrôle de deck),
  mots-clés dépendants (`kw_if`, `might_if`), auras (`aura_kw`, `aura_might`).
- **Cartes « mots-clés seulement »** (texte = mots-clés + texte de rappel : unités vanilla, tokens Recruit/Sprite/
  Bird/Sand Soldier/Tentacle, Sunlit Guardian, Shen Kinkou...) : modélisées automatiquement (`cards::AUTO`).
- **Lots de cartes** (`cardsets/`) : 1084 des 1086 cartes jouables (hors runes et tokens) sont modélisées, dont 166
  des 167 de Radiance (`rust/src/cardsets/radiance.rs`, mots-clés des Core Rules du 2026-10-09 : Deploy, Disarm, Show Off,
  jeton Bomb ; bloquées : Neeko, Blending In) ; Baron Nashor et le jeton Baron Pit (troisième battlefield,
  `Game.add_battlefield`, règles 187.9 et 471.1.b) ; crochets
  partagés (remplacements de mort, de point, de révélation, coûts alternatifs, permissions de jouer
  `play_perm`, capacités et [Add] donnés par d'autres objets, battlefield remplacé `replace_battlefield` — règle
  438, Brush d'Ivern...) décrits dans `cardsets/GUIDE.md` §9.
- **Joueur humain** (`rust/src/train.rs`) : chaque question `g.ask` a un titre français (`cards::ASK_TEXT`, `ask_text()`) et un
  titre anglais (`lang::ASK_EN`, version anglaise de la table ; textes montrés au joueur : `lang::tr(fr, en)`),
  ses options sont lisibles (cartes, runes, lieux, `Opt`, choix de jeu, répartitions de dégâts) ; l'humain voit
  tous les choix légaux (`Human.every_choice` → `actions.full_choices`, `cards.cap`, `Impl.all_choices` :
  sous-ensembles de Commander Ledros, répartitions de Volibear, toutes les cibles...), l'IA garde ses listes
  plafonnées et ordonnées (même vitesse).

## Limites connues
- L'IA est une recherche à un coup avec déroulement par politique simple sur 2 tours : elle ne bluffe pas et ne
  planifie pas une combinaison sur plusieurs tours. Les écarts entre options sont plus fiables que les niveaux
  absolus.
- Les valeurs de bonus de might des équipements ne sont pas dans la base de cartes (icône) : Long Sword +2,
  Sterak's Gage +3, Pendulum Blade +1 (relevées sur les cartes), et le texte d'effet de Pendulum Blade
  (+2 might ce tour quand l'unité équipée bouge sur un battlefield) vient de la même source.
- Zhonya's Hourglass utilise le texte de l'errata (riftwatcher), différent de celui de la base de cartes.
- Modes 3-4 joueurs non implémentés. Les cartes hors pool sont modélisées par lots dans `cardsets/` (voir
  `cardsets/GUIDE.md`) ; une carte absente de `cards::IMPL` n'est pas modélisée et le deck builder la refuse.
  Crochets moteur manquants connus : `cardsets/NEEDS.md`. Les battlefields forment une liste `g.bfs` de taille
  variable (2, ou 3 avec le Baron Pit) : un lieu de battlefield se teste avec `is_bf(loc)`, on les parcourt avec
  `g.bf_locs` (ou `g.locs()` avec la base), jamais avec `(0, 1)`.
- Pour l'humain, certains choix combinatoires peuvent être longs (Icathian Rain : toutes les combinaisons de 6
  cibles ; Commander Ledros : tous les sous-ensembles d'unités). Les questions sans titre déclaré affichent
  « Choisis une option : » (le test `every_ask_kind_has_a_french_title` l'empêche pour les sortes littérales).

## Utilisation
```bash
cd rust && cargo build --release && cargo test --release   # moteur, cartes, référence figée Python
cd ../riftbound/engine                                      # les outils lisent et écrivent results*.json, empreinte.json ici
../../rust/target/release/fuzz_cards 50                     # 50 parties aléatoires avec les cartes auto-modélisées
../../rust/target/release/exp 160 480                       # suite d'expériences (relancer pour continuer)
../../rust/target/release/empreinte                         # décisions de l'IA à graines fixes (--maj si le jeu change exprès)
```
