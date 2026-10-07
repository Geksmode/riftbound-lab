#!/usr/bin/env bash
# Équipe d'agents Riftbound : une session tmux « rift », une fenêtre « chef » et une fenêtre par agent
# (cartes, design, ia, simulation, retex), chacun dans son worktree git et sur sa branche agent/<nom>.
#
#   scripts/equipe.sh start              crée les worktrees, la session tmux et lance claude dans chaque fenêtre
#   scripts/equipe.sh send <agent> <msg> envoie une tâche à un agent (message en argument, ou sur l'entrée standard avec -)
#   scripts/equipe.sh status             pour chaque agent : dernières lignes de l'écran et dernier rapport
#   scripts/equipe.sh read <agent>       écran récent + dernier rapport de l'agent
#   scripts/equipe.sh launch <agent> <fichier-mission>   lance l'agent en mode non interactif (claude -p) dans sa fenêtre tmux ; journal et marqueur dans $RB_EQUIPE/logs/
#   scripts/equipe.sh attach             ouvre la session tmux (Ctrl-b n / p pour changer de fenêtre, Ctrl-b d pour sortir)
#   scripts/equipe.sh stop               ferme la session tmux (les worktrees et branches restent)
#
# Variables : CLAUDE_CMD (défaut : claude), RB_AGENTS (défaut : cartes design ia simulation retex),
#             RB_WORK (dossier des worktrees, défaut : le dossier parent du dépôt), RB_BASE (branche de départ, défaut : origin/main).
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
S=rift
AGENTS="${RB_AGENTS:-cartes design ia simulation retex}"
WORK="${RB_WORK:-$(dirname "$REPO")}"
BASE="${RB_BASE:-origin/main}"
CLAUDE_CMD="${CLAUDE_CMD:-claude}"
export RB_EQUIPE="${RB_EQUIPE:-$WORK/rb-equipe}"      # boîte aux lettres partagée, hors git : taches/ et rapports/

need() { command -v "$1" >/dev/null || { echo "Il faut $1." >&2; exit 1; }; }
send_text() {   # send_text <fenêtre> <texte> : colle le texte puis valide (gère les messages sur plusieurs lignes)
  local buf="rb-$$"
  printf '%s' "$2" | tmux load-buffer -b "$buf" -
  tmux paste-buffer -d -b "$buf" -t "$S:$1"
  sleep 0.5
  tmux send-keys -t "$S:$1" Enter
}
check_agent() { case " $AGENTS " in *" $1 "*) ;; *) echo "Agent inconnu : $1 (agents : $AGENTS)" >&2; exit 1;; esac; }

cmd="${1:-}"; shift || true
case "$cmd" in
start)
  need tmux; need git
  tmux has-session -t "$S" 2>/dev/null && { echo "La session $S existe déjà (scripts/equipe.sh attach, ou stop)."; exit 1; }
  mkdir -p "$RB_EQUIPE/taches" "$RB_EQUIPE/rapports"
  git -C "$REPO" fetch -q origin || echo "(fetch impossible : je pars de $BASE tel quel)"
  for a in $AGENTS; do
    d="$WORK/rb-$a"
    if [ ! -d "$d" ]; then
      if git -C "$REPO" show-ref -q --verify "refs/heads/agent/$a"; then git -C "$REPO" worktree add -q "$d" "agent/$a"
      else git -C "$REPO" worktree add -q "$d" -b "agent/$a" "$BASE"; fi
    fi
  done
  tmux new-session -d -s "$S" -n chef -c "$REPO" -e RB_EQUIPE="$RB_EQUIPE" -e RB_AGENTS="$AGENTS" "$CLAUDE_CMD"
  for a in $AGENTS; do
    tmux new-window -t "$S" -n "$a" -c "$WORK/rb-$a" -e RB_EQUIPE="$RB_EQUIPE" "$CLAUDE_CMD"
  done
  sleep "${RB_BOOT:-8}"   # laisse claude démarrer
  send_text chef "Tu es le CHEF de l'équipe d'agents Riftbound. Lis .claude/chef.md puis CLAUDE.md et docs/REPRISE.md, et attends mes consignes."
  for a in $AGENTS; do
    send_text "$a" "Tu es l'agent « $a » de l'équipe Riftbound. Lis ton rôle dans .claude/agents/$a.md puis CLAUDE.md. Boîte aux lettres partagée : \$RB_EQUIPE=$RB_EQUIPE. Attends les tâches du chef."
  done
  echo "Équipe lancée : fenêtres chef $AGENTS. Dossier des rapports : $RB_EQUIPE/rapports"
  echo "scripts/equipe.sh attach pour entrer dans tmux."
  ;;
send)
  need tmux; a="${1:?agent}"; shift; check_agent "$a"
  msg="${1:?message (ou - pour lire stdin)}"; [ "$msg" = "-" ] && msg="$(cat)"
  tmux list-windows -t "$S" -F '#W' | grep -qx "$a" || { echo "Fenêtre $a introuvable : scripts/equipe.sh start d'abord." >&2; exit 1; }
  mkdir -p "$RB_EQUIPE/taches"; printf '%s\n' "$msg" > "$RB_EQUIPE/taches/$a-$(date +%Y%m%d-%H%M%S).md"
  send_text "$a" "[tâche du chef] $msg  — Quand c'est fini : committe, pousse ta branche et écris ton rapport dans \$RB_EQUIPE/rapports/."
  echo "Tâche envoyée à $a."
  ;;
read|status)
  need tmux
  list="${1:-$AGENTS}"
  for a in $list; do
    echo "=== $a ==="
    tmux capture-pane -p -t "$S:$a" -S -15 2>/dev/null | sed '/^[[:space:]]*$/d' | tail -"${RB_LINES:-8}" || echo "(fenêtre absente)"
    r="$(ls -t "$RB_EQUIPE/rapports/$a"-*.md 2>/dev/null | head -1 || true)"
    if [ -n "$r" ]; then echo "--- dernier rapport : $r"; [ "$cmd" = read ] && cat "$r" || head -5 "$r"; else echo "--- aucun rapport"; fi
  done
  ;;
launch)
  need tmux; need git
  a="${1:?agent}"; f="${2:?fichier de mission}"; check_agent "$a"
  d="$WORK/rb-$a"; mkdir -p "$RB_EQUIPE/logs" "$RB_EQUIPE/rapports" "$RB_EQUIPE/taches"
  if [ ! -d "$d" ]; then
    if git -C "$REPO" show-ref -q --verify "refs/heads/agent/$a"; then git -C "$REPO" worktree add -q "$d" "agent/$a"
    else git -C "$REPO" worktree add -q "$d" -b "agent/$a" "$BASE"; fi
  fi
  TOOLS="${RB_TOOLS:-Read Edit Write Glob Grep WebFetch WebSearch Bash(git:*) Bash(python3:*) Bash(bash:*) Bash(node:*) Bash(npm:*) Bash(npx:*) Bash(ls:*) Bash(cat:*) Bash(cd:*) Bash(mkdir:*) Bash(cp:*) Bash(mv:*) Bash(grep:*) Bash(sed:*) Bash(head:*) Bash(tail:*) Bash(wc:*) Bash(sort:*) Bash(diff:*) Bash(date:*) Bash(echo:*) Bash(time:*)}"
  R="$RB_EQUIPE/logs/$a.run.sh"
  {
    echo '#!/usr/bin/env bash'
    echo "cd \"$d\" || exit 1"
    echo "rm -f \"$RB_EQUIPE/logs/$a.done\""
    echo "export RB_EQUIPE=\"$RB_EQUIPE\""
    printf '%s -p "$(cat %q)" --agent %q --permission-mode acceptEdits --allowedTools %q 2>&1 | tee %q\n' "$CLAUDE_CMD" "$f" "$a" "$TOOLS" "$RB_EQUIPE/logs/$a.log"
    echo "echo \"fin \$(date +%H:%M:%S)\" > \"$RB_EQUIPE/logs/$a.done\"; sleep 5"
  } > "$R"
  if tmux has-session -t "$S" 2>/dev/null; then tmux new-window -t "$S" -n "$a" -c "$d" "bash $R"
  else tmux new-session -d -s "$S" -n "$a" -c "$d" "bash $R"; fi
  echo "Agent $a lancé (journal : $RB_EQUIPE/logs/$a.log)."
  ;;
attach) tmux attach -t "$S" ;;
stop) tmux kill-session -t "$S" && echo "Session fermée (worktrees et branches conservés)." ;;
*) sed -n '2,13p' "${BASH_SOURCE[0]}"; exit 1 ;;
esac
