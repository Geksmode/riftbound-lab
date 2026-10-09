---
titre: État du travail
resume: où en est le projet aujourd'hui, ce qui est fait, en cours, à faire
maj: 2026-10-07
sources: docs/REPRISE.md, git log, CI
---
**Fait et vérifié**
- Moteur : 917 cartes jouables sur 919 (bloquées : Baron Nashor et Baron Pit, il faut un troisième battlefield). `test_all.py` = 968/968, `t_rand.py 20` = 0 erreur (2026-10-07).
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
