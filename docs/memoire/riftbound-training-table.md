---
name: riftbound-training-table
description: table d'entraînement (moteur Rust en WebAssembly depuis le 2026-11-24 ; avant : Pyodide), publiée sur GitHub Pages ; leçons d'interface et de vérification (retex 2026-10-09)
metadata:
  type: project
  modified: 2026-10-05T16:56:40.551Z
---
**2026-11-24 : la table tourne sur le moteur Rust compilé en WebAssembly** (`rust/src/train.rs`, `wasm.rs` ; `riftbound.wasm` dans `train/engine.worker.js`, même protocole `{id, fn, args}`). Pyodide, `precompile.mjs` et `fetch_pyodide.sh` sont retirés ; `engine/train.py`, `engine/lang.py` et `engine/cardsets/test_*.py` n'existent plus (équivalents `train.rs`, `lang.rs`, `cargo test`). Les sections ci-dessous datées d'avant sont l'historique du moteur Pyodide ; détail de la bascule : [portage-rust](portage-rust.md).

2026-10-05: user asked "un outil d'entraînement où je joue contre toi". Built artifact https://claude.ai/artifact/HTmfbgtzcPP7jN7UsCfcHC "Riftbound Entraînement" (separate from replay viewer [[riftbound-replays]]).

- User plays Akali G2 vs LeBlanc IQ#5 AI (PlanAgent, plan Hook tempo, tempo AI [[riftbound-tempo-ai]]); level Normal samples=1 / Fort samples=3 / Très fort samples=6 + turn_w=6, react_n=3 (2026-10-27, [[riftbound-search-ai]]).
- 2026-11-24 : les niveaux Normal / Fort / Très fort sont devenus les niveaux 6 / 7 / 8 d'une échelle de 1 à 10 (dix boutons, « − » et « + » sur téléphone) : [[ia-niveaux]].
- Real Python engine in browser via Pyodide 0.26.4 hosted inside the artifact (python_stdlib.zip published as base64 .txt because .zip isn't served; blob URL as stdLibURL). jsdelivr is blocked in the container, npm registry works (npm pack pyodide@0.26.4).
- engine/train.py: Human agent raises NeedChoice for g.ask/mulligan; state deep-copied before each op and restored, then op replayed with answers (engine deterministic). undo (40 deep), hint = Akali Gorica-plan AI top options. AI hand/facedown masked.
- Sources riftbound/train/ (src.html, base.css, build.sh, tr.py Playwright bot). Engine edits must be re-copied via build.sh and republished.
- Viewer CSP blocks fetch of blob: URLs (v1 died with "Program terminated with exit(1)"). v2 intercepts window.fetch for a virtual stdlib URL. Test locally with scratchpad-style CSP server (connect-src self) before publishing.
- v3-v5 (2026-10-06): drag and drop + arrows; all legal target combos for the human (train._expand); nothing auto-fires: card popover (Jouer → zone, Empower with cost, 🎯 Cibler…, Déplacer…), targets then "Valider", moves staged then "Lancer le déplacement / ⚔ Lancer le combat".
- v6 (2026-10-06): engine game.pay now floats energy: a READY rune recycled for power is exhausted first, +1 pool_e (pool empties at turn boundaries). User-reported real-play trick; affects all sims after this date (91/91 tests pass; not re-measured). Page: duplicate hand copies alias to the copy that has options (engine dedupes options by card name); mid-resolution choices (g.ask with board objects) picked by clicking the unit then "Valider la cible"; floating energy shown on rune line.
- v7 (2026-10-06): user wants every game button on the board, not in the top-right panel. #actBox (status, Terminer/Passer, Conseil, ask/mulligan), #fbar (Valider la cible / Lancer le combat, now static) and Reprendre mon coup live in .ctrlcol, the board's right column (236px, above the chain); boardgrid min-width 1144. Phone: .ctrlcol sticky at the bottom of the board. The right panel keeps only the journal and Nouvelle partie.
- v8 (2026-10-06): user wants a RiftAtlas-like UI with no interaction box. Score track (0-8 circles) left of the board; right column = turn info, big END TURN/Passer button (#bMain), Conseil (popover #hpop), ↶ Reprendre, chain. Every question (mulligan, may Oui/Non, targets, buttons) sits in a bubble (#fbar, class prompt) placed by placeFloat() on the first spot that hides no highlighted card (BF middle, LeBlanc's hand, your hand row). Deck/trash piles are clickable and open a modal with the trash list. Setup is a modal. Accelerate shows as "Accelerate (+1 énergie +1 domaine) / Normal". Fixed bug: ask uids were always null (asks come from a restored deep copy, so compare by uid, not identity); asks now also expose hand cards, zones (zs) and the source (src). inter.js is now extracted from src.html (src.html is the master copy). Test bot dnd3.py.
- v9 (2026-10-06): Shuriken Flip = 3 picks on the board (enemy or "Pas d'ennemi", ally to move, destination zone; steps 1/2/3 in the bubble); train._expand adds all enemy x ally x zone combos for options with "mover". No emoji, "Cibler" text. Journal is a hidden right drawer (top-bar button with unread count). Opening Nouvelle partie no longer drops a pending mulligan. pump() catches engine errors (toast, no frozen table). Modern look (gradients, glass, animations).

## État 2026-10-09 (après les PR #1 à #15)
Publiée par la CI sur GitHub Pages (plus l'artefact). DA « observatoire » (`train/da.css`, maquette Claude Design). Ordinateur : plateau pleine largeur, taille fixe pendant la partie, runes en vraies cartes, légende agrandie. Téléphone : plein écran sans défilement, rangées qui se resserrent, main agrandie au toucher (plus de 8 cartes), ciblage par liste dans la bulle. Robots dans la porte : verif_mobile, verif_hidden, verif_revele, verif_charge, verif_cibles, verif_match, verif_duel. Détails : `docs/wiki/log.md`, retex `riftbound/retex/retex-session-2026-10-07-09.md`.

## Tutoriel (2026-10-10)
Menu « Apprendre à jouer » : partie guidée Yi contre Darius, IA débutante, 10 leçons et 6 défis pas à pas, zones surlignées, lexique ; l'agrandi d'une carte explique ses mots-clés (toute la table). Détails : `docs/wiki/pages/tutoriel.md`.

## Menu en tableau de bord (2026-10-10)
Choix de l'utilisateur parmi 5 dispositions : le tableau de bord. En haut, « Ton deck d'entraînement » (illustration de la légende, liste, bilan victoires/défaites avec ce deck, « Jouer avec ce deck », « Changer », « Modifier ») et « Dernière partie » (reprendre, revoir, ou tutoriel pour un nouveau). En dessous, une rangée compacte : Nouvelle partie, ami, tutoriel, éditeur, Mes decks, Mes parties, Aide. **Aucun deck par défaut** (demande de l'utilisateur) : un nouveau joueur voit « Aucun deck choisi » et le choisit (légende, puis liste : ses decks, puis les listes de tournoi, la conseillée ★ en tête). Clé `rbt-train` (forme `my:<id>` ou clé de liste). Robot : `train/verif_dash.mjs`.

## Leçons (retex 2026-10-09, validées par l'utilisateur)
- **Quand** je touche à la mise en page, **aucune taille ne doit dépendre du contenu** (chaîne, unités, runes) et je **vérifie par mesure automatique** à plusieurs tailles et sur une situation chargée : rien hors de son cadre, aucun libellé coupé, aucun défilement, une seule échelle sur une partie entière. *Pourquoi :* plateau qui rapetissait en cours de partie, boutons hors cadre, runes coupées, main qui débordait, tous sur mes propres captures sans que je les relève. *(PR #11 à #14)*
- **Quand** je teste un geste, **prendre le cas difficile** (carte à plusieurs actions, cible cachée sous la bulle), pas le cas simple. **Sur téléphone**, tout choix doit être faisable hors du plateau (liste dans la bulle) et aucune rangée ne défile. *Pourquoi :* le premier robot Hidden passait avec Zhonya alors que Back Off ne se cachait pas au glisser ; base inaccessible pour Shuriken Flip. *(PR #9, #15)*
- **Pièges CSS :** limiter un sélecteur à son contexte (`.shell > .main`, pas `.main`, classe aussi portée par des boutons) ; une image individuelle (IMG) n'a pas de taille de fond, contrairement aux planches : `background-size: cover`. *Pourquoi :* bouton du sideboard étiré à 779 px (attrapé par le robot du match), runes coupées. *(PR #12, #13)*
- **Quand** un texte décrit un coup adverse (bulle, journal, flèche), **vérifier qu'il ne révèle rien de caché**. *Pourquoi :* « LeBlanc cache Hidden Blade » (`train._opp_describe`). *(PR #11)*


2026-10-10: renamed "Riftbound Entraînement" → "Riftbound Observatory" (user choice, fits the « observatoire » DA). Same artifact URL.

## Sons (2026-10-11)
Demande de l'utilisateur : « plus réactive, des sons quand on interagit avec les cartes ». Premier essai en sons synthétisés
(oscillateurs Web Audio) jugé « trop 8 bits » : remplacés par de vrais MP3 (`train/sfx/`, paquet npm `uisfx` 0.4.0, audio
CC0). Sons : carte touchée / relâchée, soulevée, posée, cachée, capacité, cible, déplacement, combat, unité qui meurt,
point (toi / l'adversaire), fin de tour, passer, réponse, coup refusé, pioche, coup de l'IA, nouveau tour, reprendre,
victoire / défaite ; éditeur de deck : ajout / retrait. Menu « ♪ » de la barre du haut : son activé / coupé et six
ambiances (Studio par défaut, Cinéma, Zen, Feutré, Bois, Verre ; clés `rbt-son`, `rbt-sonpack`). Correspondance
événement → fichier : `SFX.MAP` dans `src.html`. Le mode replay reste muet. Robot : `train/verif_son.mjs`.
2026-10-11 (suite) : l'utilisateur trouve ces sons « vraiment pas bien » et veut des bruits de cartes comme sur TCG Arena (prise, pose…) et une barre de volume. Barre de volume ajoutée au menu « ♪ » (0-100, défaut 70, courbe en carré, clé `rbt-sonvol`). Bruits de cartes : pas encore faits, les sites de sons (kenney.nl, freesound, OpenGameArt, Wikimedia) sont bloqués par le réseau de l'environnement ; visée : Kenney « Casino Audio » (CC0, vrais enregistrements de cartes). Écartés faute de licence claire : sons de KPat, de `tinker-spider-solitaire`, de la démo Phaser.
2026-10-11 (bruits de cartes) : ambiance « Cartes » ajoutée et mise par défaut (7 dans le menu : Cartes + les six uisfx). Vrais enregistrements Kenney « Casino Audio » 1.1 (CC0, `train/sfx/cartes/LICENSE-KENNEY.txt`), zip fourni par l'utilisateur ; `train/sfx/importer_kenney.sh <zip>` en tire 24 MP3 mono 80 kb/s (232 Ko) et `liste.json` (variantes). Table `CARTES` de `SFX` : toucher / soulever / fin de tour = card-slide (8 variantes), poser / jouer / cible = card-place (4), déplacement / combat / mort = card-shove (4), pioche = cards-pack-take-out (2), mulligan (mélange de début de partie) = card-shuffle, victoire / défaite = card-fan (2), point marqué = chip-lay (3). Variante tirée par un xorshift à part (pas `Math.random`), jamais deux fois la même d'affilée. Gains réglés à la mesure des crêtes (0 dB Kenney contre -8 dB uisfx), pas encore à l'oreille.

## Moteur dans un Web Worker (2026-10-11)
Signalé par l'utilisateur : quand l'IA réfléchit longtemps, le navigateur dit que la page ne répond pas. Cause : Pyodide
tournait sur le fil de la page, et l'IA cherche le tour entier depuis le 2026-10-10 (~3 × plus lent). Le moteur tourne
maintenant dans `train/engine.worker.js` (copié par `build.sh`, sans URL `blob:`). Dans la page, `T.f(...)` rend une
promesse (`await J(T.f(...))`, `pump` attend la réponse) ; les appels passent un par un, dans l'ordre : même graine = même
partie. Verdicts de deck (`train.check`) en cache `CHK`, remplis par `loadCatalog` ; fins de manche (`match_record`)
enregistrées après le rendu (bouton éteint pendant l'attente) ; `GEN` arrête la boucle d'une partie remplacée. Robots :
`pyExec(code, nom)` / `pyGet(nom)` remplacent `T.__builtins__` et `T.X.toJs()`. Robot `train/verif_reactif.mjs` (IA
forte, graine 11, deux passages sur une machine chargée) : réponse du moteur la plus longue 20,9 s et 21,6 s, page jamais
bloquée plus de 44 ms pendant ce temps, Journal
ouvert au clic pendant que l'IA joue (vérifié par robot, pas essayé sur un vrai téléphone). Une réflexion en cours ne
s'interrompt pas : « Nouvelle partie » attend la fin du coup de l'IA.
Piège trouvé par `verif_replay` : pendant que le moteur crée une partie, l'ancien plateau restait jouable et ses coups
allaient dans l'ancien enregistrement ; `start`, `launchGame` et `tutoStart` tiennent `working` dès le clic.
Second piège (CI, `verif_match`) : un rendu repris au retour du moteur remplace les boutons sous le doigt. L'éditeur ne
met plus à jour que son verdict (`dkVerdict`) ; le menu ne s'active qu'une fois le catalogue chargé et vérifié (`boot`).

## Version anglaise (2026-10-17)
Demande de l'utilisateur : « fait une version anglaise de l'app ». Bouton English / Français sous le titre du menu, ou
`train.html?lang=en` ; choix gardé (`rbt-lang`), changer recharge la page. `src.html` reste en français : `train/i18n.js`
(inséré par `build.sh`) traduit le texte à l'entrée dans la page ; le moteur parle anglais via `engine/lang.py`
(`train.set_lang`). Robot `train/verif_anglais.mjs`, test `engine/cardsets/test_lang.py`. Détail : `docs/wiki/pages/anglais.md`.

## Lire les cartes du mulligan (2026-10-17)
Demande de l'utilisateur : « pendant le mulligan, on puisse lire les cartes qu'on a ». Avant, toucher une carte de la bulle
la mettait de côté et l'agrandi y était coupé (`.mull` exclu). Maintenant : bouton « Lire » sous chaque carte (ouvre
l'agrandi, le même bouton le referme, toucher une carte le ferme), et survol à la souris sur ordinateur (classe `.mull.mm`,
les autres bulles de cartes gardent leur comportement). Robot `train/verif_mulligan.mjs` (vérifié par robot, toucher
simulé ; pas essayé sur un vrai téléphone).

## Chargement du moteur avec Pyodide (2026-10-17, périmé : remplacé par le wasm le 2026-11-24)
Demande de l'utilisateur : « le chargement de l'engine est long ». Cause principale : Pyodide 0.26.4 livre sa stdlib en `.py`,
donc à chaque ouverture il recompilait ~120 modules (asyncio, typing, inspect…) puis tout le moteur (~1 Mo de `.py`).
Correctif : `train/precompile.mjs` (lancé par `build.sh`) précompile avec le Python de Pyodide (node) la stdlib chargée et
le moteur (`py/bytecode.json`, `.pyc` checked-hash, `.py` toujours servis) ; le worker télécharge stdlib, moteur et
bytecode en même temps que le wasm, la page démarre le worker avant les index d'images.
Mesures (Chromium sans limite CPU, serveur local `http.server` sans gzip, temps jusqu'au menu actif, ± écart-type) :
- local, cache froid : 3487 ± 565 ms → 1625 ± 122 ms (n = 6) ; cache chaud : 3024 ± 546 → 1239 ± 150 ms (n = 6).
- réseau lent simulé (10 Mbit/s, 150 ms) : froid 17477 ± 298 → 17224 ± 70 ms (n = 4, on ne sait pas : le téléchargement
  domine, surtout le wasm de 10 Mo) ; chaud 12069 ± 1031 → 9782 ± 348 ms (n = 4).
- node seul : `loadPyodide` ~1,8 s → ~0,6 s, `import train` ~530 → ~100 ms.
Coût : +2,3 Mo bruts au premier chargement (bytecode.json 2,0 Mo, stdlib 3,1 → 3,9 Mo) ; avec gzip (GitHub Pages
compresse le texte) environ +1,4 Mo. Pas essayé sur un vrai téléphone (limite CPU du worker non simulée).

## Base serrée ouverte en grand (2026-10-20)
Demande de l'utilisateur (capture : 9 cartes dans sa base, dures à toucher) : « cliquer sur la base et avoir un menu
déroulant de toutes les cartes présentes pour interagir avec ». Sur téléphone, une base serrée (`baseDense` : une carte
cachée d'au moins un cinquième par sa voisine), la sienne ou celle de l'adversaire, s'ouvre au premier toucher comme la
main serrée : rangée de grandes cartes (88 px) qui défile au doigt, au-dessus de sa moitié de plateau (en bas pour toi, en
haut pour l'adversaire), bandeau « Ta base / Base de … · nombre · Fermer » ; les champs de bataille restent visibles. On y
touche une carte (ses actions, ou l'agrandi) ou on la glisse (un geste horizontal fait défiler). Elle reste ouverte pendant
un ciblage, un déplacement préparé ou une question ; elle se ferme après un coup, au bouton « Fermer » ou en touchant
ailleurs. État `BASEOPEN`, `setBaseOpen`, `bheadHTML` dans `src.html`. Piège CSS : un enfant en position absolue d'une grille
prend sa case (`grid-area`) comme cadre, d'où `grid-area: auto !important`. Robot `train/verif_base.mjs` (393x851 et
360x740, dans la porte) : vérifié par robot (toucher simulé), pas essayé sur un vrai téléphone.

2026-10-21, Runeblade Anvil (demande de l'utilisateur : « une pile de cartes bannies, et les utiliser avec l'effet des
cartes ») : chaque joueur a une pile « Bann. » à côté de sa défausse (cachée si vide ; Banishment publique, 108.6.e ; champ
`ban` de `replay.snap`), qui s'ouvre au toucher (`openBanish`, cartes encore jouables marquées JOUABLE). Une carte bannie
que tu peux jouer (Runeblade Anvil, Akali, Brash : `banu` dans `train._view`) s'affiche au bout de la main comme un Flow,
marquée BANNIE ; le menu dit « Jouer → … (bannie) ». Sur téléphone, trois piles se partagent la colonne (`:has`). Robot
`train/verif_banish.mjs` (dans la porte) : vérifié par robot, pas essayé sur un vrai téléphone.

## Jouer une carte pendant un effet (2026-11-04)
Demande de l'utilisateur : « si une carte demande des cibles, afficher les cartes qui peuvent être la cible, pas des textes ».
Pour une question dont les options sont « (carte, choix) » (Fizz, Trickster, Ava Achiever…), `train._ask_play` envoie
`ask.pc[i] = {n, u, tg}`. La bulle (`askHTML`, `playPicks`) montre chaque carte une fois en image + « Ne rien jouer » ;
toucher une carte ouvre le mode `target` habituel avec `mode.ans` (nom de la carte) : les cibles se touchent sur le plateau
ou dans la liste de la bulle, « Valider la cible » répond à la question (`pickI` → `answer`), « Retour » / Échap
revient à la bulle (`unpick`). Robot `train/verif_fizz.mjs`.

Questions à options entières (2026-11-06) : `train.py` mettait dans `zs` tout entier égal à un battlefield ; pour `pay_gold`
(0/1 Gold), `bullet_time_x`, `burn_player`, `choose_player`, `ava_play`, ces entiers ne sont pas des lieux (`NOT_LOC`).
Payer la puissance (humain) : `pay_gold` (si un Gold peut payer) ou `pay_rune`, une question par rune recyclée, posée dès que plusieurs runes peuvent payer ; on touche le Gold ou la rune, la rune touchée est recyclée (`game.pay_choices`, `_pay_gold_uids`). Les tests qui pilotent la table doivent y répondre (0 = la rune du planificateur). Robot `train/verif_gold.mjs`.

Cibles multiples (2026-11-07) : une question « jusqu'à N unités » (`radiance._choose_up_to` : Opt de valeur tuple d'uid,
`()` = aucune) envoie `ask.tg` (uid de chaque option, `train._ask_multi`) ; `show()` crée directement un mode `target`
(`multiMode`, `mode.multi`, `ans` = nom de la carte) : « Valider la cible » répond avec l'option du groupe touché,
« Aucune cible » pour `()`, « Effacer » / Échap vide la sélection (pas de retour possible : la question doit être répondue).
Battlefields des planches (`atlas/`) : carte imprimée de côté, haut à gauche → `.bfrot` tourne de **+90°** (avant -90° :
tête en bas sur le plateau). `bfThumb(n)` pour les vignettes du choix de battlefield (match BO3 et duel). Robot `train/verif_multi.mjs`.

## Cartes agrandies sur ordinateur (2026-11-24)
Demande : « les cartes sont trop petites ». Dans `@media (min-width: 1000px)` : `--cw/--chh` 74×104 → 96×134 (taille de maquette, mise à l'échelle par `fitBoard`), main 104×145, légende 116×162 (adverse 92×128), rangées `.half.p0` 150/100/60, `.half.p1` 52/150, `.bf` 144/auto/144. Côtés de battlefield et main en `container-type: inline-size` avec marge négative calculée sur `--n`/`--nx` (chevauchement, jamais de défilement). Éventail plafonné (lift ≤ 12 px, rotation ≤ 9°). Mesure à 1893×909 : unité 86×120 (avant 68×95). Robot `verif_charge.mjs` (taille minimale et main à l'écran).
Runes (même jour) : `.half` en colonnes `1fr 1fr 262px`, aires p0 "runes base leg / hand hand leg / hand hand pile", p1 "ohand ohand pile / base runes leg" ; zone runes `--rw 88px --rh 123px`, chevauchement sur `--n/--nx`, `.res-line` et `.floatpool` en absolu en haut de la zone.

## Confort d'après Rift Atlas et TCG Arena (2026-11-24)
Demande : « regarde TCG arena, regarde rift atlas les différents outils, et améliore l'outil pour la quality of life ». Les deux sites sont bloqués par le réseau de l'environnement par défaut : environnement cloud « riftbound-lab » (hôtes `tcg-arena.fr`, `riftatlas.com` et leurs `www`). Les simulateurs eux-mêmes (`play.riftatlas.com`, partie TCG Arena) n'ont pas été ouverts : seulement les pages, l'éditeur de Rift Atlas et les notes de version.
- Bloc « confort » de `src.html` (avant `boot()`) : `numBtns` / `kbTag` (numéros `data-kb` posés par `decorate` sur les boutons de `#pop` et sur `.prow button[data-a], [data-i]` de `#fbar` ; affichés en CSS sur ordinateur seulement), `cardInfo` / `sheetHTML` / `openCard` (fiche : catalogue `CAT.by` s'il est chargé, sinon `CARDS` de la partie), `findCards` / `openFind` (8 résultats : nom qui commence, nom qui contient, mot-clé ou tag, texte), `openKeys`.
- Touches : chiffres par `e.code` (`Digit1`, `Numpad1`), jamais `e.key` (AZERTY). Rien dans un champ de saisie, ni fenêtre ouverte ; J, C, Entrée, chiffres seulement sur la table. Entrée et chiffres suivent la règle de la barre d'espace (`tabNav`, `blur` avant `click`).
- Clic droit : `contextmenu` seulement si le dernier pointeur est une souris (au doigt, l'appui long sert à glisser). Fenêtre déjà ouverte : l'agrandi habituel.
- Éditeur : `DKF.cost`, `DKF.sort` (`DKSORT`), `DKG` (groupes de `dkList`), `DKSNAP` / `dkMark` / `dkDirty` / `dkLose` (`confirm` avant Nouveau, Ouvrir, Importer). Fermer l'éditeur ne demande rien : le deck reste en mémoire jusqu'au rechargement de la page. Un robot qui écrase un deck modifié doit accepter le dialogue (`pg.once("dialog", d => d.accept())`).
- Pas repris : le déplacement libre des cartes, compteurs, jetons à la main (notre moteur applique les règles), comptes, matchmaking, cosmétiques. Idées restantes vues chez eux : lien de partage d'un deck, cartes favorites, « Suggestions » de Rift Atlas.
Robot `train/verif_qol.mjs` (porte, partie 3).

## Bilan, testeur de main, exercices (2026-11-24)
Demande de l'utilisateur après une liste de propositions : « 1 et 2 c'est ok, ensuite faire des puzzles ». Tout est dans
`train/src.html` (bloc avant le lexique), sans changement du moteur ; traductions dans `i18n.js`.
- **Bilan** (`bilanLoad`, menu `bilan`) : parties finies contre l'IA (`fetchGames`, pas les duels ; une manche de BO3 =
  une partie), groupées par `names[1]`, `lvlOf(g)`, `whoFirst`. Intervalle de Wilson à 95 % (`wilson`) ; verdict
  « favorable / défavorable » seulement si l'intervalle exclut 50 %, sinon « on ne sait pas ».
- **Tester la main** (`htOpen`, bouton `dkHand` de l'éditeur) : tirage en JavaScript sur `DK.main` (39 cartes, le champion
  choisi commence dans sa zone), règles 116, 117.1 à 117.3, 416.1, 430.4.a, 485.7, 315.4.b. Chances : `htProb`
  (hypergéométrique exacte), colonnes 4, 6, 8, 10 cartes vues (`HT_SEEN`).
- **Exercices** (`pzScan`, `pzStart`, `pzBar`, menu `puz`, clé `rbt-puz`) : `train.replay_load` puis `train.replay_hint`
  à chaque décision « Tu joues ». L'exercice est un replay ouvert à l'étape de la décision avec `dec` laissé actif
  (`pzLive`) ; `choose` intercepte le coup (`pzAnswer`) au lieu d'appeler le moteur.
- **Leçon** : **quand** j'affiche un conseil de l'IA comme une note, **je le calcule plusieurs fois et je donne l'erreur
  type**. *Pourquoi :* `hint` tire ses mondes au hasard (le hasard du coach n'est pas dans l'état sauvegardé) ; le robot a
  montré le « meilleur » coup d'un calcul à -4,7 au calcul suivant, et un écart-type d'environ 5 à 12 sur l'écart d'un
  coup (7 positions, une partie, niveau du conseil de la table). `pzEval` moyenne 8 calculs, écarts appariés ; un coup
  n'est dit moins bon qu'au-delà de 2 points et de 2 erreurs types. Même jour, demande de l'utilisateur (« oui pour le conseil
  bruité ») : « Conseil » et « Analyser ce coup » passent par `hintEval` (8 calculs, ± erreur type par ligne) ; le texte
  « sous 2 c'est du bruit » est remplacé partout (bulle, replay, aide). Contrôlé par `verif_entrain` et `verif_replay`.
- Robot `train/verif_entrain.mjs` (porte, partie 4) : comptes et intervalles du bilan recalculés à part, chances
  recalculées par dénombrement, exercice joué par le gros bouton puis par les coups conseillé et joué ; vérifié par
  robot (toucher simulé), pas sur un vrai téléphone.

## D'après le simulateur Rift Atlas (2026-11-25)
Demande : « en te basant sur tcg arena et riftatlas les simulateurs de riftbound, améliore l'outil encore ». Lu : notes de
version de riftatlas.com (`/changelog/2026/week/N`, entrées `data-changelog-entry`), accueil de tcg-arena.fr. Le simulateur
`play.riftatlas.com` s'ouvre (hôtes ajoutés à l'environnement « riftbound-lab ») mais « Solo Room » demande un compte :
aucune partie ouverte chez eux. Tout est dans `train/src.html`, moteur inchangé ; traductions dans `i18n.js`.
- **Réglages** `SET` (`rbt-cs`, `rbt-might`, `rbt-alert`, `rbt-wake`), section « Affichage » du menu ♪ (`sonPop`). `--cs`
  multiplie `--cw/--chh`, les rangées de battlefield, la base et les runes dans `@media (min-width: 1000px)` ; `fitBoard`
  remet ensuite le plateau à l'échelle, donc 130 % ne donne pas +30 % à l'écran. Téléphone : réglage masqué.
- **Total de Might** : `bfHTML`, `.bfsum.top/.bot` en absolu dans `.bf` (`pointer-events: none`). Somme des `u.m` de la vue
  (`Game::might` : buffs, équipements, Assault / Shield une fois l'unité désignée).
- **Journal** : `logPut` regroupe les lignes identiques à la suite (`data-x`, `.lx`), jamais les titres de tour ; `LOG`
  garde toutes les lignes. Classes `pt` (`+1 point (`) et `cb` (`Combat ` / `Showdown `). Un robot qui compte les lignes
  doit additionner les `data-x`.
- **Replay** : `RP.turns` / `RP.pts` = étapes des lignes « Tour N » et « +1 point » de `r.log` ; boutons `data-rpj`.
- **Import** : `importGames` (bouton `bImpGames`, fichier déposé sur le menu « Mes parties ») ; partie marquée `imp: 1`,
  exclue de `bilanLoad` et de `dashGames` ; même id = pas remplacée.
- **Versions** : `dkhPush` dans `saveDeck` (clé `rbt-dkhist`, 30 par deck, suit le renommage, supprimée avec le deck),
  `dkhDiff`, `dkhOpen`. Navigateur seulement (pas dans la base claude.ai).
- **Stats** : `STSEL`, section `#stSel` (`htProb`, `HT_SEEN`).
- **À toi** : `nudge(v)` à la fin de `show`, dans `showMenu` / `hideMenu` et sur `visibilitychange` ; `wakeSync`.
- **Leçon** : **quand** je me sers de `I18N.t` hors du texte de la page (titre de l'onglet), **je teste `I18N.lang`**.
  *Pourquoi :* `I18N.t` traduit même en français ; le robot a vu « ● Your move » sur la page française.
- Pas repris : goldfish sur le plateau, suggestions de cartes, export vidéo, arrêts de phase (le moteur demande déjà
  chaque décision). Robot `train/verif_atlas2.mjs` (porte, partie 3).
