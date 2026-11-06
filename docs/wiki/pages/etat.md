---
titre: État du travail
resume: où en est le projet aujourd'hui, ce qui est fait, en cours, à faire
maj: 2026-11-06
sources: docs/REPRISE.md, git log, CI
---
**Fait et vérifié**
- **Jouer une carte pendant un effet (2026-11-04)** : Fizz, Trickster (et toute question « quelle carte jouer, avec quelles cibles » : `train._ask_play`) montre l'image de chaque sort jouable de la défausse (un exemplaire par nom), puis on touche ses cibles sur le plateau et « Valider la cible » ; « Retour » revient aux sorts. Avant : une liste de textes « Switcheroo (sans payer l'énergie ; cible X, Y) », doublée par les deux exemplaires. Test `cardsets/test_fizz_table.py`, robot `train/verif_fizz.mjs` (téléphone et ordinateur) : vérifié par robot, pas sur un vrai téléphone.
- **[Repeat] sur la table et dans le moteur (2026-11-04)** : retour utilisateur « la table ne propose pas de répéter le sort ». La table ne reconnaissait que le Repeat imprimé codé `rep` : le Repeat accordé (The Academy, Temporal Portal) et ceux codés autrement (Square Up, Temptation, Curtain Call) étaient fusionnés avec le sort normal ; la cible de la répétition n'était jamais affichée. Le moteur ne proposait pas non plus plusieurs Repeat payés ensemble (820.1.c.2, 820.3). Corrigé : une ligne par façon de répéter (« Avec Repeat ×2 (coût total 3 énergie) → Scuttle Crab et Lonely Poro »). Tests `table_marks_every_repeat_variant`, `printed_and_granted_repeat_paid_together` ; vérifié par robot (`train/verif_repeat.mjs`, toucher simulé), pas sur un vrai téléphone.
- **[Show Off] sur la table (2026-10-30)** : le joueur choisit la carte montrée (main ou en jeu) ou « Sans Show Off » ; avant, la page fusionnait ces choix. Vérifié par robot (`train/verif_showoff.mjs`), pas sur un vrai téléphone. Voir [radiance.md](radiance.md).
- **Baron Nashor et Baron Pit (2026-10-19)** : modélisés. Les battlefields sont une liste de taille variable (`g.bfs`, `is_bf`, `g.bf_locs`) ; Nashor ajoute le jeton Baron Pit (3e battlefield, règle 187.9) et y entre (369.3) ; le point final par conquête exige alors les 3 battlefields marqués dans le tour (471.1.b.1), sinon on pioche. Unités vers le Baron Pit depuis n'importe où (sans Ganking). Les jetons ne peuvent plus être un battlefield de départ (Brush compris). Table et lecteur de replays : 3 battlefields côte à côte. Détail : `cardsets/NEEDS_chaos.md`, `NEEDS_battlefields.md`.
- **Decks par défaut des 9 légendes de Radiance (2026-11-06)** : les 58 légendes ont un deck (listes des Showdowns Sacramento et Orlando, reconstituées par recherche web, doutes notés : `riftbound/decks/README.md`). 54 parties de contrôle, 0 erreur. Pas encore de plan par légende pour elles. Le tirage du mode `defaut` d'`exp_search.py` passe de 49 à 58 decks : chiffres d'avant non regroupables avec ceux d'après.
- **Radiance (2026-10-19)** : 166 des 167 cartes de l'extension modélisées (Neeko bloquée), voir [radiance.md](radiance.md).
- Moteur (2026-10-07) : 917 cartes jouables sur 919 à cette date ; Baron Nashor et Baron Pit débloqués le 2026-10-19 (ci-dessus).
- CI GitHub Actions sur `main` : tests et build de la table, tous verts ; publication sur Pages configurée (adresse attendue https://geksmode.github.io/riftbound-lab/, ouverture de la page **non vérifiée**).
- Équipe d'agents en place ([equipe.md](equipe.md)).
- **IA générale (2026-10-09)** : recherche « sh » (tirages communs + élimination en passes) par défaut, +15 points contre l'ancienne sur les decks par défaut (65,3 % ± 2,3, 160 paires), 62,5 % ± 2,9 en Akali contre LeBlanc ; temps Pyodide 417 → 503 ms par décision. Détails : [recherche de l'IA](../../memoire/riftbound-search-ai.md).

**Fait le 2026-10-08 (1re itération des agents, intégrée dans `claude/sleepy-hopper-azt8ub`, non fusionnée dans `main`)**
- `cartes` : 7 cartes corrigées pour le choix « zéro cible » (355.13), 66 tests ajoutés, `AUDIT_TEXTE.md` ; `test_all.py` = 1034/1034, `t_rand.py 30` = 0 erreur (relancés par le chef).
- `design` : menu d'accueil responsive dans `train/src.html` (captures dans `train/captures-menu/`). Non testé sur Pages ni avec la vraie base claude.ai.
- `ia` : banc d'essai `engine/exp_general.py`, plan général `plans_general.py` (drapeau `RB_GENERAL`, non branché dans `train.py`). Gain : **on ne sait pas** (51,9 % ± 3,0, 79 paires).
- Gemdragon / Hwei : « ready up to 2 runes » = le maximum possible jusqu'à 2, rien si tout est prêt (décision de l'utilisateur, 2026-10-08, testé).
- Retex : `riftbound/retex/retex-equipe-iteration1.md`. Les chiffres de simulation d'avant 1034 tests ne sont plus comparables.

**Corrigé le 2026-10-08** : bug `tg2` du Repeat accordé (test `granted_repeat_never_reuses_printed_repeat_variant`) ; lien Replays du menu (`build.sh` publie `replays.html` + `games/`) ; bulle de fin de partie au-dessus de la barre à 360 px ; boutons ± de l'éditeur à 44 px. Vérifié par `train/verif_mobile.mjs` (Chromium, 360×740, partie jouée jusqu'au bout, coups injectés).

**Énergie flottante (2026-10-08)** : le moteur épuise une rune prête avant de la recycler (Punch First en 1re carte : 1 énergie flotte, test `cardsets/test_energie_flottante.py`) ; la table affiche un badge « ⚡ N énergie flottante » et la puissance flottante par domaine dans la zone des runes. Vérifié dans Chromium 360×740 sur une partie réelle (graine 0, deck Sivir avec Punch First, coups injectés).

**Dégâts de combat (2026-10-08)** : à la fin d'un showdown de combat, le joueur humain choisit une à une l'unité qui reçoit ses dégâts mortels (465.2.c.3), parmi celles que les règles permettent (Tank d'abord 815, Backline en dernier 826), excédent sur la dernière (465.2.c.4) ; l'IA garde son ordre. Tests `cardsets/test_degats_combat.py` ; vérifié dans Chromium 1400×900 sur une partie réelle (graine 0) : clic sur l'unité puis Valider.

**Décision en attente** : branchement de `plans_general` dans `train.py`.

**Plans par légende (2026-10-11)** : un plan sourcé pour chacune des 49 légendes par défaut ([mémoire](../../memoire/plans-par-legende.md), `riftbound/gameplans/legendes.md`), non mesuré ; à mesurer en local par l'utilisateur (`exp_search.py ... "sh@plan=leg" sh defaut`) avant tout branchement dans la table.

**Duel entre amis** : fait ([duel.md](duel.md)) ; branche à fusionner par PR (Deflect sur les répétitions, duel).

**À faire, dans cet ordre (décidé par l'utilisateur le 2026-10-07, après la 1re itération des agents)**
1. ~~Porte d'intégration~~ **fait le 2026-10-08** : [porte.md](porte.md).
2. ~~Chiffres versionnés~~ **fait le 2026-10-08** : [resultats.md](resultats.md).
3. ~~Tests générés depuis le texte des cartes~~ **fait en partie le 2026-10-08** : `engine/cardsets/test_texte_auto.py` (233 sorts : cible obligatoire injouable sur plateau vide, « up to / any number » jouable et toujours avec un choix sans cible, résolution sans erreur ; 15 unités « up to »). Pas encore généré : « you may » (refus proposé) et « then » au-delà de la résolution sans erreur. **Deathgrip** corrigé (décision de l'utilisateur : règles au plus près) : deux cibles alliées obligatoires (355.8), injouable sans deux unités alliées.
4. Boucle « chaque partie jouée améliore l'IA » : export de la partie, analyse en CI, modèle des choix adopté seulement s'il gagne à plus de 2 écarts-types sur graines neuves.

**Autres tâches en attente**
- Analyse des 17 parties de l'utilisateur (`train_games.py --coach`) : le rejeu plante sur les parties enregistrées avec un moteur plus ancien (correctif tolérant poussé) ; dire si le rejeu n'est pas fidèle, ne pas forcer.
- Système « chaque partie jouée améliore l'IA » : export des parties depuis Pages (pas de base claude.ai), analyse en CI, modèle des choix, mesure sur graines neuves.
- Recréer la routine de l'agent manager (toutes les 6 h, prompt dans `docs/REPRISE.md` §4).
- Republier en artefact claude.ai (optionnel maintenant que Pages existe) : `docs/REPRISE.md` §2-3.
- Retex de la première itération des agents.

Les chiffres de simulation d'avant le 2026-10-06 sont périmés (énergie flottante) : `docs/vault/Chiffres périmés.md`.
