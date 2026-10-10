import { existsSync } from "node:fs";
// Robot du raccourci barre d'espace dans la vraie table : Espace = gros bouton « Terminer le tour » / « Passer »
// (réaction, showdown), même si une carte a gardé le focus après un clic souris ; sans effet sur le menu,
// dans un champ de saisie, ou quand ce n'est pas à toi. Ordinateur seulement (pas de clavier sur téléphone).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_espace.mjs <dossier des journaux (inutilisé)> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
const pg = await (await b.newContext({ viewport: { width: 1400, height: 900 } })).newPage();
const errs = []; pg.on("pageerror", e => errs.push(String(e)));
const ev = s => pg.evaluate(s);
const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
await pg.goto(`http://localhost:${PORT}/train.html`);
await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
await pg.keyboard.press("Space"); await pg.waitForTimeout(200);
ok("Espace sur le menu : le menu reste ouvert", await ev("!$('menu').hidden"));
await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
await ev("document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'");
await pg.click("#bStart");
// mulligan et questions : réponses directes ; on s'arrête à la première phase principale
for (let i = 0; i < 200; i++) { await waitFor("!!(V && !working)"); if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; } if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; } if (await ev("!!(V.dec && V.dec.kind === 'main')")) break; await ev("(async()=>{ await pump(J(T.act(0))) })()"); }
const t0 = await ev("V.st.t");
// clic souris sur une carte de la main : elle prend le focus ; Espace doit quand même terminer le tour
await pg.locator("#board .half.p0 .hand .card").first().click(); await pg.waitForTimeout(200);
const foc = await ev("document.activeElement && document.activeElement.classList.contains('card')");
await pg.keyboard.press("Escape");
await pg.keyboard.press("Space");
await waitFor(`!!(V && !working && (V.winner != null || V.ask || (V.dec && (V.st.t > ${t0} || V.dec.kind !== 'main'))))`);
ok(`Espace avec le focus sur une carte (${foc ? "focus pris" : "focus non pris"}) termine le tour ${t0} (maintenant tour ${await ev("V.st.t")}, ${await ev("V.dec ? V.dec.kind : V.ask ? 'question' : '?'")})`, await ev(`V.st.t > ${t0} || (V.dec && V.dec.kind !== 'main')`));
// la suite à l'Espace seul : chaque décision où le bouton propose « Passer » ou « Terminer le tour »
const seen = {}; let n = 0;
for (let i = 0; i < 300 && n < 40; i++) {
  await waitFor("!!(V && !working && !V.busy)");
  if (await ev("V.winner != null")) break;
  if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; }
  if (!await ev("!!V.dec")) { await pg.waitForTimeout(100); continue; }
  const k = await ev("V.dec.kind"), lab = await ev("$('bMain').disabled ? '' : $('bMain').textContent");
  if (!lab) { await ev("(async()=>{ await pump(J(T.act(0))) })()"); continue; }
  const before = await ev("LOG.length");
  await pg.keyboard.press("Space");
  await waitFor(`!!(V && (working || LOG.length !== ${before} || !V.dec || V.dec.kind !== ${JSON.stringify(k)}))`, 20000).catch(() => {});
  const moved = await ev(`LOG.length !== ${before} || working`);
  seen[k + " / " + lab] = (seen[k + " / " + lab] || 0) + (moved ? 1 : 0); if (!moved) seen[k + " / " + lab + " (rien)"] = 1; n++;
}
console.log("décisions jouées à l'Espace :", JSON.stringify(seen));
ok("Espace a toujours fait avancer la partie", !Object.keys(seen).some(x => x.endsWith("(rien)")));
ok("Espace a servi à « Passer » (réaction ou showdown)", Object.keys(seen).some(x => / \/ Passer$/.test(x)));
ok("pas d'erreur JavaScript", errs.length === 0); if (errs.length) console.log(errs.join("\n"));
await b.close();
console.log(fails ? `${fails} échec(s)` : "tout est vert"); process.exit(fails ? 1 : 0);
