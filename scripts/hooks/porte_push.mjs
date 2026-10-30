// Hook PreToolUse (Bash) : un `git push` qui envoie du code ne part que si scripts/verifier.sh est vert sur ce code.
// Règle de CLAUDE.md (« avant d'intégrer ou de publier, verifier.sh doit être vert ») doublée d'un contrôle mécanique.
// Seul l'agent est arrêté : l'utilisateur pousse ce qu'il veut depuis son propre terminal.
import { CODE, TABLE, couvre, empreinte, git, lire, resume } from "../porte_etat.mjs";
import { base, depot, entree } from "./commun.mjs";

const inp = await entree();
const cmd = String(inp.tool_input?.command || "");
if (!/(^|[\s;&|(])git\b[^;&|\n]*\bpush\b/.test(cmd)) process.exit(0);

const root = depot(inp);
if (!root) process.exit(0);
const b = base(root);
const noms = b ? git(root, "diff", "--name-only", b.sha, "HEAD") : null;
if (noms === null) process.exit(0); // pas de base connue (premier push d'un dépôt neuf) : la CI vérifiera
const fichiers = noms.split("\n").filter((f) => CODE.test(f));
if (!fichiers.length) process.exit(0); // docs, données, résultats : pas besoin de la porte

const etat = lire(root);
if (couvre(etat, fichiers) && etat.empreinte === empreinte(root, false)) process.exit(0);

const table = fichiers.some((f) => TABLE.test(f));
const moteur = fichiers.some((f) => !TABLE.test(f));
const quoi = table && moteur ? "tout" : table ? "table" : "moteur";
console.error(
  [
    `Push bloqué par la porte d'intégration : ${fichiers.length} fichier(s) de code depuis ${b.ref} ` +
      `(${fichiers.slice(0, 5).join(", ")}${fichiers.length > 5 ? "…" : ""}).`,
    resume(root) + ".",
    `Lance \`bash scripts/verifier.sh ${quoi}\` sur ce commit (réglages par défaut, sans RB_PART), corrige jusqu'au vert, puis repousse.`,
    "Si tu ne peux pas la lancer ici, ne pousse pas : dis à l'utilisateur ce qui n'est pas vérifié.",
  ].join("\n"),
);
process.exit(2);
