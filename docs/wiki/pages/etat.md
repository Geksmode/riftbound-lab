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

**Fait le 2026-10-08 (1re itération des agents, intégrée dans `claude/sleepy-hopper-azt8ub`, non fusionnée dans `main`)**
- `cartes` : 7 cartes corrigées pour le choix « zéro cible » (355.13), 66 tests ajoutés, `AUDIT_TEXTE.md` ; `test_all.py` = 1034/1034, `t_rand.py 30` = 0 erreur (relancés par le chef).
- `design` : menu d'accueil responsive dans `train/src.html` (captures dans `train/captures-menu/`). Non testé sur Pages ni avec la vraie base claude.ai.
- `ia` : banc d'essai `engine/exp_general.py`, plan général `plans_general.py` (drapeau `RB_GENERAL`, non branché dans `train.py`). Gain : **on ne sait pas** (51,9 % ± 3,0, 79 paires).
- Retex : `riftbound/retex/retex-equipe-iteration1.md`. Les chiffres de simulation d'avant 1034 tests ne sont plus comparables.

**Décisions en attente** : Gemdragon / Hwei (approximation, contraire à la règle 4) ; branchement de `plans_general` dans `train.py` ; bug `tg2` dans `game.py::repeat_resolution`.

**À faire, dans cet ordre (décidé par l'utilisateur le 2026-10-07, après la 1re itération des agents)**
1. Porte d'intégration `scripts/verifier.sh` (tests, 30 parties aléatoires, lint du wiki, build de la table), appliquée en CI aux branches `agent/*`.
2. Chiffres de simulation étiquetés : hash du moteur et graines dans chaque fichier de résultats ; un script de comparaison refuse de mélanger deux versions ou deux séries sur les mêmes graines.
3. Tests générés depuis le texte des cartes (`up to` jouable à zéro cible, `you may` refusable, `then` appliqué quand même).
4. Boucle « chaque partie jouée améliore l'IA » : export de la partie, analyse en CI, modèle des choix adopté seulement s'il gagne à plus de 2 écarts-types sur graines neuves.

**Autres tâches en attente**
- Analyse des 17 parties de l'utilisateur (`train_games.py --coach`) : le rejeu plante sur les parties enregistrées avec un moteur plus ancien (correctif tolérant poussé) ; dire si le rejeu n'est pas fidèle, ne pas forcer.
- Système « chaque partie jouée améliore l'IA » : export des parties depuis Pages (pas de base claude.ai), analyse en CI, modèle des choix, mesure sur graines neuves.
- Recréer la routine de l'agent manager (toutes les 6 h, prompt dans `docs/REPRISE.md` §4).
- Republier en artefact claude.ai (optionnel maintenant que Pages existe) : `docs/REPRISE.md` §2-3.
- Retex de la première itération des agents.

Les chiffres de simulation d'avant le 2026-10-06 sont périmés (énergie flottante) : `docs/vault/Chiffres périmés.md`.
