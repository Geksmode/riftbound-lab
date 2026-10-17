---
name: riftbound-training-table
description: table d'entraînement (moteur réel dans Pyodide), publiée sur GitHub Pages ; leçons d'interface et de vérification (retex 2026-10-09)
metadata:
  type: project
  modified: 2026-10-05T16:56:40.551Z
---
2026-10-05: user asked "un outil d'entraînement où je joue contre toi". Built artifact https://claude.ai/artifact/HTmfbgtzcPP7jN7UsCfcHC "Riftbound Entraînement" (separate from replay viewer [[riftbound-replays]]).

- User plays Akali G2 vs LeBlanc IQ#5 AI (PlanAgent, plan Hook tempo, tempo AI [[riftbound-tempo-ai]]); level Normal samples=1 / Fort samples=3.
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

## Chargement du moteur (2026-10-17)
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
