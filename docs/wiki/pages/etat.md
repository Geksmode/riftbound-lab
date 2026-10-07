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

**En cours (2026-10-07)** : première itération de trois agents : `cartes` (audit « up to » / « any number »), `design` (menu d'accueil), `ia` (ressources et banc d'essai de l'IA générale). Résultats : rapports dans `$RB_EQUIPE/rapports/`, intégration par le chef.

**À faire**
- Analyse des 17 parties de l'utilisateur (`train_games.py --coach`) : le rejeu plante sur les parties enregistrées avec un moteur plus ancien (correctif tolérant poussé) ; dire si le rejeu n'est pas fidèle, ne pas forcer.
- Système « chaque partie jouée améliore l'IA » : export des parties depuis Pages (pas de base claude.ai), analyse en CI, modèle des choix, mesure sur graines neuves.
- Recréer la routine de l'agent manager (toutes les 6 h, prompt dans `docs/REPRISE.md` §4).
- Republier en artefact claude.ai (optionnel maintenant que Pages existe) : `docs/REPRISE.md` §2-3.
- Retex de la première itération des agents.

Les chiffres de simulation d'avant le 2026-10-06 sont périmés (énergie flottante) : `docs/vault/Chiffres périmés.md`.
