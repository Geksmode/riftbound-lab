---
titre: Équipe d'agents (chef + sessions tmux)
resume: comment un chef délègue à des agents tmux, qui fait quoi, comment on intègre leur travail
maj: 2026-10-07
sources: scripts/equipe.sh, .claude/chef.md, .claude/agents/, docs/MISSIONS.md
---
- **Chef** : session qui découpe, délègue (`scripts/equipe.sh send` ou `launch`), relit les rapports, vérifie les chiffres, intègre. Rôle : `.claude/chef.md`.
- **Agents** : `cartes` (cartes, interactions, fidélité aux règles), `design` (menu, responsive, ergonomie), `ia` (IA générale et plans),
  `simulation` (matchups, statistiques), `retex` (relecture critique, mémoire, vault). Rôles et périmètres : `.claude/agents/<nom>.md`.
- **Isolation** : un worktree `../rb-<agent>` et une branche `agent/<nom>` par agent ; boîte aux lettres partagée `$RB_EQUIPE`
  (`rapports/`, `logs/`, `taches/`), hors git.
- **Deux modes** : `start` (sessions interactives, l'utilisateur les pilote) ; `launch <agent> <fichier>` (`claude -p`, non interactif, outils limités,
  journal à la fin de l'exécution, marqueur `.done`).
- **Intégration** : les agents ne poussent pas ; le chef fusionne leurs branches dans la branche de travail, relance `test_all.py`, et l'utilisateur fusionne dans `main`.
- **Missions décidées par l'utilisateur** : `docs/MISSIONS.md`. Chaque agent met à jour les pages de son périmètre (voir `WIKI.md`).
- Testé : la mécanique tmux avec `echo`, et un `claude -p` réel sur une tâche triviale. Les missions longues sont en première exécution (2026-10-07).
- **Leçons de la 1re itération** (détail : `riftbound/retex/retex-equipe-iteration1.md`) : un redémarrage du conteneur tue tmux et les agents → sauvegarder les branches `agent/*` toutes les 30 min (permission de pousser `agent/*` donnée par l'utilisateur le 2026-10-08) ; `claude -p` n'écrit son journal qu'à la fin ; le sandbox d'un agent peut interdire `$RB_EQUIPE` → rapport dans le dépôt ; missions petites, critère de fin chiffré.

