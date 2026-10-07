# Moteur Riftbound fidèle (1v1)

Moteur de règles complet écrit d'après les **Core Rules du 16/07/2026** (`../rules/source/core_rules_2026-07-16.txt`,
récupérées sur app.riftjudge.com). Aucune simplification de carte : chaque carte du pool Akali/LeBlanc est codée
d'après son texte, avec les numéros de règle cités en commentaire.

## Fichiers
| fichier | contenu |
|---|---|
| `game.py` | état de partie, zones, runes et paiement des coûts, chaîne (priorité/focus), cleanups, showdowns, combat, contrôle, score, burn out, couches (layers) de calcul de might |
| `actions.py` | actions discrétionnaires légales et processus de jeu d'une carte / activation d'une capacité (règles 353-359, 398-406) |
| `cards.py` | registre `IMPL` (une entrée par carte : 85 cartes du pool + tokens), aides génériques pour les mots-clés (`ability`, `parse_cost`, `add_ability`, `when_level`...), modélisation automatique des cartes « mots-clés seulement » (`auto_keywords`, liste `AUTO`), chargeur des modules `cardsets/`, contrôle de deck (`deck_problems` : non modélisée, [Unique]) |
| `cardsets/` | modules de cartes par lot (liste explicite `MODULES` dans `__init__.py`, chargés à la fin de `cards.py`), `GUIDE.md` (guide de modélisation), `NEEDS.md` (crochets moteur manquants), `test_keywords.py` (mots-clés génériques) |
| `testkit.py`, `test_all.py` | aides de test pour les modules (`Suite`, `temp_card`, `deck_top`...) ; lance `test_cards.py` et tous les `cardsets/test_*.py` |
| `fuzz_cards.py` | parties aléatoires avec des decks contenant des cartes données (ou les cartes d'un module) : aucune exception, même graine = même partie |
| `ai.py` | agents : `SearchAgent` (recherche à 1 coup sur copies de la partie + déroulement par politique), `PolicyAgent` (politique rapide), évaluation, déterminisation de l'information cachée |
| `decks.py` | charge les 12 decklists de tournoi depuis `../decks/decks.json` |
| `test_cards.py` | 91 tests de scénario, au moins un par carte du pool et par règle clé |
| `run.py`, `exp.py` | runner Monte Carlo multiprocess et suite d'expériences reprenable (`results.json`) |
| `quick.py`, `agents.py` | partie unique avec journal, agent aléatoire (fuzzing du moteur) |
| `replay.py`, `batch_replays.py`, `curate_replays.py` | enregistreur de replays (états + scores des options de l'IA) pour le lecteur `../replays/viewer.html` |
| `plans.py`, `exp_plans.py` | plans de jeu des joueurs (Akali Gorica moteur/agressif, LeBlanc Deathknell/Hook tempo) et leurs simulations (`results_plans.json`) ; sources dans `../gameplans/` |
| `exemple_partie.txt` | deux parties complètes tour par tour |
| `pool.txt` | texte, coût, might et mots-clés des 85 cartes du pool |

## Ce qui est modélisé
- **Tour** : Awaken, Beginning (effets de début, Temporary avant le score, puis Scoring/tenue), Channel 2 runes
  (3 au premier tour du joueur qui commence en second), Draw 1, Main, End (soin, fin des effets « ce tour »,
  vidage des pools).
- **Runes et coûts** : épuiser = 1 énergie, recycler = 1 power du domaine, la même rune peut faire les deux ;
  Gold (« [Reaction] Tue ceci, [E] : [Add] [A] ») utilisable pendant le paiement ; pools vidés au début de la Main
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
  Bird/Sand Soldier/Tentacle, Sunlit Guardian, Shen Kinkou...) : modélisées automatiquement (`cards.AUTO`).
- **Lots de cartes** (`cardsets/`) : 917 des 919 cartes jouables (hors runes et tokens) sont modélisées ; crochets
  partagés (remplacements de mort, de point, de révélation, coûts alternatifs, permissions de jouer
  `play_perm`, capacités et [Add] donnés par d'autres objets, battlefield remplacé `replace_battlefield` — règle
  438, Brush d'Ivern...) décrits dans `cardsets/GUIDE.md` §9.
- **Joueur humain** (`train.py`) : chaque question `g.ask` a un titre français (`cards.ASK_TEXT`, `ask_text()`),
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
  `cardsets/GUIDE.md`) ; une carte absente de `cards.IMPL` n'est pas modélisée et le deck builder la refuse.
  Crochets moteur manquants connus : `cardsets/NEEDS.md` (Baron Nashor et Baron Pit : il faudrait un troisième
  battlefield).
- Pour l'humain, certains choix combinatoires peuvent être longs (Icathian Rain : toutes les combinaisons de 6
  cibles ; Commander Ledros : tous les sous-ensembles d'unités). Les questions sans titre déclaré affichent
  « Choisis une option : » (le test `every_ask_kind_has_a_french_title` l'empêche pour les sortes littérales).

## Utilisation
```bash
python3 test_cards.py                 # 91 tests du pool
python3 test_all.py                   # test_cards.py + tous les cardsets/test_*.py
python3 fuzz_cards.py 50              # 50 parties aléatoires avec les cartes auto-modélisées
python3 exp.py 160 480                # suite d'experiences (relancer pour continuer)
python3 run.py                        # fonctions play() pour vos propres expériences
```
