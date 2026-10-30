#!/usr/bin/env node
// État de la porte d'intégration (scripts/verifier.sh), partagé avec les hooks de scripts/hooks/.
// Node plutôt que bash ou Python : Claude Code tourne sous Node, donc ce script marche partout (Windows compris).
//
//   node scripts/porte_etat.mjs ecrire <quoi> <code> [part] [nav] [fuzz] [rand]   (appelé à la fin de verifier.sh)
//   node scripts/porte_etat.mjs etat                                 (résumé lisible)
//
// L'état est dans .claude/state/porte.json (non versionné) : commit, empreinte du contenu des fichiers de code
// (committés ou non), partie lancée (moteur / table / tout), résultat.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

// Fichiers dont un changement exige la porte. Données, résultats, journaux et docs n'en font pas partie.
export const CODE = /^(riftbound\/.*\.(py|mjs|js|html|css|sh)|scripts\/.*)$/;
export const TABLE = /^riftbound\/(train|puzzles|replays)\//;

export function git(cwd, ...args) {
  try {
    return execFileSync("git", args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
  } catch {
    return null;
  }
}

export function racine(cwd) {
  return git(cwd, "rev-parse", "--show-toplevel");
}

function gitZ(cwd, args, input) {
  try {
    return execFileSync("git", args, { cwd, encoding: "utf8", input, stdio: [input ? "pipe" : "ignore", "pipe", "ignore"] });
  } catch {
    return "";
  }
}

// Fichiers de code modifiés, nouveaux ou supprimés dans l'arbre de travail (pas encore committés).
export function codeNonCommitte(root) {
  const champs = gitZ(root, ["status", "--porcelain", "-z", "-uall", "--", "riftbound", "scripts"]).split("\0");
  const out = [];
  for (let i = 0; i < champs.length; i++) {
    const l = champs[i];
    if (!l) continue;
    if (l[0] === "R" || l[0] === "C") i++; // renommage : le chemin d'origine suit, on garde le nouveau
    const f = l.slice(3);
    if (CODE.test(f)) out.push(f);
  }
  return out;
}

// Carte chemin -> objet git des fichiers de code, au commit courant (arbre = false) ou dans l'arbre de travail.
// Les fichiers de travail sont hachés par git lui-même (filtres de fin de ligne compris, Windows CRLF).
export function carte(root, arbre) {
  const m = new Map();
  for (const l of gitZ(root, ["ls-tree", "-r", "-z", "HEAD", "--", "riftbound", "scripts"]).split("\0")) {
    const t = l.indexOf("\t");
    if (t < 0) continue;
    const f = l.slice(t + 1);
    if (CODE.test(f)) m.set(f, l.slice(0, t).split(" ")[2]);
  }
  if (arbre) {
    const modif = codeNonCommitte(root);
    const presents = modif.filter((f) => existsSync(join(root, f)));
    for (const f of modif) m.delete(f);
    if (presents.length) {
      const shas = gitZ(root, ["hash-object", "--stdin-paths"], presents.join("\n") + "\n").trim().split("\n");
      presents.forEach((f, i) => m.set(f, shas[i]));
    }
  }
  return m;
}

// Empreinte du code seul : ne bouge pas quand on committe sans rien changer, ni quand on touche aux données ou aux docs.
export function empreinte(root, arbre = true) {
  const h = createHash("sha1");
  for (const [f, sha] of [...carte(root, arbre)].sort()) h.update(f + " " + sha + "\n");
  return h.digest("hex");
}

export function lire(root) {
  const p = join(root, ".claude", "state", "porte.json");
  try {
    return JSON.parse(readFileSync(p, "utf8"));
  } catch {
    return null;
  }
}

// La porte couvre-t-elle ces fichiers ? (moteur pour le code Python et les scripts, table pour la page)
export function couvre(etat, fichiers) {
  if (!etat || etat.code !== 0) return false;
  // Une porte allégée (RB_FUZZ, RB_RAND sous les valeurs par défaut, RB_NAV=0, une seule partie RB_PART) ne compte pas.
  if (Number(etat.fuzz ?? 200) < 200 || Number(etat.rand ?? 30) < 30) return false;
  const moteur = fichiers.some((f) => !TABLE.test(f));
  const table = fichiers.some((f) => TABLE.test(f));
  const q = etat.quoi;
  if (moteur && !(q === "moteur" || q === "tout")) return false;
  if (table && !((q === "table" || q === "tout") && !etat.part && etat.nav !== "0")) return false;
  return true;
}

export function resume(root) {
  const e = lire(root);
  if (!e) return "porte : jamais lancée dans ce dossier";
  const verte = e.code === 0 ? "verte" : "ROUGE";
  const ici = e.empreinte === empreinte(root) ? "sur le code actuel" : `sur ${e.head.slice(0, 7)} ; le code a changé depuis`;
  const extra = [
    e.part && `partie ${e.part}`,
    e.nav === "0" && "sans robots navigateur",
    (Number(e.fuzz ?? 200) < 200 || Number(e.rand ?? 30) < 30) && `allégée : fuzz ${e.fuzz}, hasard ${e.rand}, ne compte pas`,
  ].filter(Boolean).join(", ");
  return `porte : ${verte} (${e.quoi}${extra ? ", " + extra : ""}) le ${e.date}, ${ici}`;
}

const estCli = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (estCli) {
  const [cmd, quoi, code, part = "", nav = "1", fuzz = "200", rand = "30"] = process.argv.slice(2);
  const root = racine(process.cwd());
  if (!root) process.exit(0); // hors d'un dépôt git (archive) : rien à noter
  if (cmd === "ecrire") {
    const dir = join(root, ".claude", "state");
    mkdirSync(dir, { recursive: true });
    const etat = {
      head: git(root, "rev-parse", "HEAD"),
      empreinte: empreinte(root),
      quoi,
      code: Number(code),
      part,
      nav,
      fuzz,
      rand,
      date: new Date().toISOString().slice(0, 16).replace("T", " "),
    };
    writeFileSync(join(dir, "porte.json"), JSON.stringify(etat, null, 2) + "\n");
    console.log(`(état de la porte noté dans .claude/state/porte.json)`);
  } else if (cmd === "etat") {
    console.log(resume(root));
  } else {
    console.log("usage : node scripts/porte_etat.mjs ecrire <quoi> <code> [part] [nav] [fuzz] [rand] | etat");
    process.exit(2);
  }
}
