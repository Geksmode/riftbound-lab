// Balayage de tous les textes de src.html (littéraux JS lus par l'analyseur acorn, texte et attributs HTML) : affiche
// chaque morceau qui contient du texte et n'a pas de traduction anglaise complète dans i18n.js (sans traduction, ou
// traduction qui garde des lettres françaises). Les morceaux assemblés à l'exécution (« Tour de » + nom) sortent aussi :
// vérifier la phrase entière avant d'ajouter une entrée. Ne remplace pas verif_anglais.mjs (page réelle).
// Usage : node scan_textes.js [src.html] [i18n.js]   (acorn : npm, ou ACORN=chemin/acorn.js)
const fs = require("fs"), path = require("path");
const acorn = require(process.env.ACORN || ["/opt/node-tools/node_modules/acorn/dist/acorn.js", "acorn"].find(p => { try { require.resolve(p); return true; } catch (e) { return false; } }));
const SRC = process.argv[2] || path.join(__dirname, "src.html"), I18N = process.argv[3] || path.join(__dirname, "i18n.js");
global.localStorage = { getItem: () => "fr", setItem() {} }; global.location = { search: "", href: "x" };
global.window = global; global.document = { documentElement: {}, addEventListener() {} };
eval(fs.readFileSync(I18N, "utf8"));
const tx = window.I18N.tx;
const html = fs.readFileSync(SRC, "utf8");
const chunks = [];
const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map(m => m[1]).filter(s => !s.includes("/*I18N_JS*/") || s.length > 30);
const body = html.replace(/<style>[\s\S]*?<\/style>/g, "").replace(/<script[\s\S]*?<\/script>/g, "");
for (const t of body.split(/<[^>]*>/)) chunks.push(["html", t]);
for (const m of body.matchAll(/(?:title|aria-label|placeholder|label)="([^"]*)"/g)) chunks.push(["html-attr", m[1]]);
for (const sc of scripts) {
  let ast; try { ast = acorn.parse(sc, { ecmaVersion: 2022, locations: true }); } catch (e) { console.error("parse", e.message); continue; }
  (function visit(n) {
    if (!n || typeof n.type !== "string") return;
    if (n.type === "Literal" && typeof n.value === "string") chunks.push([n.loc.start.line, n.value]);
    if (n.type === "TemplateLiteral") chunks.push([n.loc.start.line, n.quasis.map(q => q.value.cooked).join("\u0001")]);
    for (const k in n) { const v = n[k]; if (k === "loc") continue; if (Array.isArray(v)) v.forEach(visit); else if (v && typeof v === "object") visit(v); }
  })(ast);
}
const out = new Map();
for (const [ln, raw] of chunks) {
  for (let c of raw.split(/<[^>]*>/)) {
    // attributs dans les gabarits
    c = c.replace(/\s+/g, " ").trim();
    if (!/[A-Za-zÀ-ÿ]{3}/.test(c.replace(/\u0001/g, ""))) continue;
    if (!/ /.test(c) && !/[À-ÿ]/.test(c)) continue;               // un seul mot sans accent : identifiants, classes
    if (/^[\w\-.#:\[\]="'() >,*+~^$|\u0001]+$/.test(c) && /[.#\[=]/.test(c)) continue;   // sélecteurs
    const ok = ["Pit Rookie", "2", "Darius", "x"].some(f => { const r = tx(c.replace(/\u0001/g, f)); return r !== null && !/[àâçéèêëîïôûùœ«»]/.test(r); });
    if (!ok) out.set(c.replace(/\u0001/g, "${}"), ln);
  }
  for (const m of raw.matchAll(/(?:title|aria-label|placeholder)="([^"\u0001]*)"/g)) { const c = m[1].trim(); if (/[a-zà-ÿ]{3} /i.test(c) && tx(c) === null) out.set("[attr] " + c, ln); }
}
for (const [k, l] of out) console.log(l + "\t" + JSON.stringify(k));
console.error(out.size);
