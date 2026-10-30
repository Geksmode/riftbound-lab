// Outils communs aux hooks de .claude/settings.json (lecture de l'entrée, base de comparaison).
import { git, racine } from "../porte_etat.mjs";

export async function entree() {
  let s = "";
  for await (const c of process.stdin) s += c;
  try {
    return JSON.parse(s || "{}");
  } catch {
    return {};
  }
}

// Dossier du dépôt de la session. On part de `cwd` (fourni par Claude Code) et pas de $CLAUDE_PROJECT_DIR,
// qui reste sur le dépôt principal quand la session travaille dans un worktree.
export function depot(inp) {
  return racine(inp.cwd || process.cwd());
}

// Point de départ du travail pas encore poussé : la branche suivie si elle existe, sinon origin/main.
export function base(root) {
  const amont = git(root, "rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}");
  for (const ref of [amont, "origin/main", "main"]) {
    if (!ref) continue;
    const mb = git(root, "merge-base", "HEAD", ref);
    if (mb) return { ref, sha: mb };
  }
  return null;
}
