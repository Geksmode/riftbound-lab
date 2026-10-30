// Hook Stop : avant de rendre la main, si du code a changé (non committé ou pas encore poussé) et que la porte
// n'est pas verte sur ce code, demande une fois à l'agent de la lancer ou de dire clairement ce qui n'est pas vérifié.
// Contre l'« annonce de victoire » prématurée (règle 7 de CLAUDE.md). Une seule relance : stop_hook_active.
// Désactiver pour une session ou une routine : variable RB_HOOK_ARRET=0 (par ex. dans .claude/settings.local.json, "env").
import { CODE, TABLE, codeNonCommitte, couvre, empreinte, git, lire } from "../porte_etat.mjs";
import { base, depot, entree } from "./commun.mjs";

const inp = await entree();
if (inp.stop_hook_active || process.env.RB_HOOK_ARRET === "0") process.exit(0);
const root = depot(inp);
if (!root) process.exit(0);

const b = base(root);
const commits = b ? (git(root, "diff", "--name-only", b.sha, "HEAD") || "").split("\n") : [];
const fichiers = [...new Set([...commits.filter((f) => CODE.test(f)), ...codeNonCommitte(root)])];
if (!fichiers.length) process.exit(0);

const etat = lire(root);
if (couvre(etat, fichiers) && etat.empreinte === empreinte(root, true)) process.exit(0);

const table = fichiers.some((f) => TABLE.test(f));
const moteur = fichiers.some((f) => !TABLE.test(f));
const quoi = table && moteur ? "tout" : table ? "table" : "moteur";
const reason = [
  `Du code a changé (${fichiers.slice(0, 4).join(", ")}${fichiers.length > 4 ? "…" : ""}) et la porte n'est pas verte dessus.`,
  `Si le travail est fini : lance \`bash scripts/verifier.sh ${quoi}\` (moteur ≈ 3 min 30, robots de la table bien plus) et corrige jusqu'au vert.`,
  "Si tu t'arrêtes exprès avant (question à l'utilisateur, étape intermédiaire, porte impossible ici) : dis-le clairement " +
    "dans ta réponse, avec ce qui n'est pas vérifié. Ne dis pas « testé » ni « fini ».",
].join(" ");
console.log(JSON.stringify({ decision: "block", reason }));
