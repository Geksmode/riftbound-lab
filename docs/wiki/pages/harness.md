---
titre: Garde-fous automatiques (harness)
resume: ce que les hooks de .claude/ font respecter mécaniquement, contrat de tâche, escalade, worktrees
maj: 2026-10-30
sources: .claude/settings.json, scripts/hooks/, scripts/porte_etat.mjs, .worktreeinclude, https://github.com/walkinglabs/learn-harness-engineering, https://code.claude.com/docs/en/worktrees
---
Principe (« double encodage ») : une règle importante est écrite dans `CLAUDE.md` **et** tenue par un contrôle
mécanique, parce qu'un agent chargé oublie une règle écrite. Ce qui existait déjà : `compare.py` (refuse d'agréger deux
versions du moteur), l'éditeur qui bloque les cartes non modélisées, `empreinte.py`, la porte `scripts/verifier.sh`, la CI.

## Hooks de `.claude/settings.json` (Node, donc aussi sous Windows)
| Hook | Fichier | Effet |
|---|---|---|
| SessionStart | `scripts/hooks/demarrage.mjs` | 4 à 6 lignes d'état réel : branche, commits non poussés, porte, worktrees perdus, `etat.md` périmée (> 7 j) |
| PreToolUse (Bash) | `scripts/hooks/porte_push.mjs` | `git push` refusé si du code a changé depuis la branche suivie (ou `origin/main`) et que la porte n'est pas verte **sur ce code** |
| Stop | `scripts/hooks/arret.mjs` | une relance avant de rendre la main si du code a changé sans porte verte : lancer la porte, ou dire ce qui n'est pas vérifié |
| permissions.deny | `Bash(pkill -f:*)` | interdit `pkill -f` (il tue le shell de l'agent) |

- « Code » = `riftbound/**/*.{py,mjs,js,html,css,sh}` et `scripts/` ; données, résultats, journaux et docs n'exigent rien.
- Code de `riftbound/train|puzzles|replays/` : il faut `verifier.sh table` ou `tout`, entier (sans `RB_PART`, sans `RB_NAV=0`) ;
  le reste : `moteur` ou `tout`. Une porte allégée (`RB_FUZZ` < 200, `RB_RAND` < 30) ne compte pas.
- État : `verifier.sh` appelle `node scripts/porte_etat.mjs ecrire …` à la fin ; il note dans `.claude/state/porte.json`
  (non versionné) l'empreinte du **contenu** des fichiers de code (committer sans rien changer ne la casse pas).
  Lire : `node scripts/porte_etat.mjs etat`.
- Seul l'agent est arrêté : l'utilisateur pousse ce qu'il veut depuis son terminal. Couper la relance de fin de session
  (routine, exploration) : `"env": {"RB_HOOK_ARRET": "0"}` dans `.claude/settings.local.json`.
- Vérifié le 2026-10-30 (clone Linux) : push bloqué sans porte (code 2), autorisé après porte verte, autorisé après un
  commit du code vérifié, bloqué après modification ; relance de fin unique ; données seules ignorées ; porte allégée refusée.
  **Non essayé sous Windows** (Claude Code y lance les hooks via Git Bash : à confirmer à la première session).

## Contrat de tâche (pour le chef et pour toute tâche non triviale)
Avant de coder, écrire : **livrable** (fichiers), **contraintes** (règles de `CLAUDE.md` concernées), **hors périmètre**
(ce qu'on ne touche pas), **critère de fin** mesurable (commande et résultat attendu). Fin = critère vérifié, pas code écrit.

## Escalade
Même erreur deux fois de suite après correction : arrêter d'éditer, montrer la sortie exacte, proposer la cause et le
correctif, attendre l'utilisateur (ou le chef). Pas de troisième essai à l'aveugle.

## Worktrees
- Équipe tmux : `scripts/equipe.sh` crée `../rb-<agent>` sur `agent/<nom>` (inchangé).
- Session isolée ponctuelle : `claude -w <nom>` → `.claude/worktrees/<nom>/`, branche `worktree-<nom>`, nettoyée en sortie
  si rien n'a changé ; `claude -w <nom> --resume` pour reprendre. Préférer ça aux copies faites à la main.
- `.worktreeinclude` recopie `riftbound/train/pyodide-cache/` dans chaque nouveau worktree (pas de re-téléchargement).
- `.claude/worktrees/` est ignoré par git. Lister / ranger : `git worktree list`, `git worktree remove <dossier>`, `git worktree prune`.
- Sous-agent isolé : `isolation: worktree` dans l'en-tête d'un rôle de `.claude/agents/` (pas mis par défaut : les rôles
  servent aussi à `claude --agent` dans les fenêtres tmux, déjà isolées).
