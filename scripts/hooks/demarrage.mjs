// Hook SessionStart : quelques lignes d'état réel en début de session (branche, avance, porte, worktrees,
// fraîcheur de l'état écrit), pour ne pas repartir d'un document périmé. Volontairement court : peu de tokens.
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { git, resume } from "../porte_etat.mjs";
import { base, depot, entree } from "./commun.mjs";

const inp = await entree();
const root = depot(inp);
if (!root) process.exit(0);

const lignes = [];
const branche = git(root, "branch", "--show-current") || "(HEAD détachée)";
const head = git(root, "rev-parse", "--short", "HEAD") || "?";
const b = base(root);
const avance = b ? git(root, "rev-list", "--count", `${b.sha}..HEAD`) : null;
const sale = (git(root, "status", "--porcelain") || "").split("\n").filter(Boolean).length;
lignes.push(
  `branche ${branche} @ ${head}` +
    (avance && avance !== "0" ? `, ${avance} commit(s) non poussé(s) depuis ${b.ref}` : "") +
    (sale ? `, ${sale} fichier(s) modifié(s)` : ""),
);
if (branche === "main") lignes.push("tu es sur main : crée une branche avant de committer du travail (l'utilisateur fusionne par PR).");
lignes.push(resume(root));

// Worktrees : ceux dont le dossier a disparu, et le nombre total.
const wt = (git(root, "worktree", "list", "--porcelain") || "").split("\n\n").filter(Boolean);
const perdus = wt.filter((w) => /\nprunable/.test(w)).map((w) => w.split("\n")[0].replace("worktree ", ""));
if (wt.length > 1)
  lignes.push(`${wt.length - 1} autre(s) worktree(s)` + (perdus.length ? `, dont ${perdus.length} sans dossier (git worktree prune)` : ""));

// Fraîcheur de l'état écrit : la page etat.md sert de mémoire entre sessions.
const etat = join(root, "docs", "wiki", "pages", "etat.md");
if (existsSync(etat)) {
  const maj = (readFileSync(etat, "utf8").match(/^maj:\s*(\d{4}-\d{2}-\d{2})/m) || [])[1];
  if (maj) {
    const jours = Math.floor((Date.now() - Date.parse(maj)) / 86400000);
    if (jours > 7) lignes.push(`docs/wiki/pages/etat.md date du ${maj} (${jours} j) : vérifie avant de t'y fier, mets-la à jour en fin de travail.`);
  }
}
lignes.push("Point d'entrée : docs/wiki/index.md. Garde-fous : docs/wiki/pages/harness.md.");
console.log("[état du dépôt]\n- " + lignes.join("\n- "));
