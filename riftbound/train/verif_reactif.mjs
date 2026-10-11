import { existsSync } from "node:fs";
// Playwright : celui de l'environnement cloud Claude, sinon le paquet npm « playwright » (CI).
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
// Robot : la page reste réactive pendant que l'IA réfléchit (moteur dans le Web Worker engine.worker.js).
// Une partie contre l'IA forte, coups du joueur au hasard (graine fixe). Mesures : la plus longue réponse du moteur
// (dans le worker) et le plus long blocage du fil de la page (minuterie de 50 ms). Avant le worker, les deux étaient égaux.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_reactif.mjs <dossier des captures> [port]
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
const ctx = await b.newContext({ viewport: { width: 1400, height: 900 } });
const pg = await ctx.newPage(); const errs = []; pg.on("pageerror", e => errs.push(String(e)));
const ok = (n, v) => console.log((v ? "OK   " : "ÉCHEC") + " " + n);
const ev = s => pg.evaluate(s);
const waitFor = async (s, ms = 300000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
await pg.goto(`http://localhost:${PORT}/train.html`);
await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
ok("moteur chargé dans le worker (aucune trace de Pyodide dans la page)", await ev("!!ENGW && typeof loadPyodide === 'undefined'"));
// sondes : durée de chaque appel au moteur, et plus grand écart entre deux tops d'une minuterie de 50 ms (fil de la page)
await ev(`(()=>{ window.__M = { call: 0, fn: "", gap: 0, gapAI: 0 };
  const orig = engineCall;
  engineCall = msg => { const t = performance.now(); return orig(msg).then(r => { const d = performance.now() - t; if (d > __M.call) { __M.call = d; __M.fn = msg.fn || "exec"; } return r; }); };
  let last = performance.now();
  setInterval(() => { const n = performance.now(), g = n - last - 50; last = n; if (g > __M.gap) __M.gap = g; if (working && g > __M.gapAI) __M.gapAI = g; }, 50);
})()`);
await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
await ev("document.getElementById('sSeed').value='11'; document.getElementById('sLvl').value='3'");
await pg.click("#bStart");
let seed = 4242; const rnd = n => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed % n; };
let moves = 0, probes = 0, slowProbe = 0, journalOK = false;
for (let i = 0; i < 2500; i++) {
  // pendant que l'IA joue : la page répond (évaluation aller-retour) et un bouton réagit au clic
  while (await ev("!V || working")) {
    const t = Date.now(); await ev("1"); const d = Date.now() - t; probes++; if (d > slowProbe) slowProbe = d;
    if (!journalOK && await ev("!!V && working && V.busy")) {
      await pg.click("#bJournal", { timeout: 2000 }).catch(() => {});
      journalOK = await ev("document.getElementById('drawer').classList.contains('open')");
      await pg.click("#bJournal", { timeout: 2000 }).catch(() => {});
    }
    await pg.waitForTimeout(100);
  }
  if (await ev("V.winner !== null && V.winner !== undefined")) break;
  if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
  if (await ev("!!V.ask")) { const k = await ev("V.ask.options.length"); await ev(`(()=>{ pump(J(T.answer(JSON.stringify(${rnd(k)})))); return 1 })()`); moves++; continue; }
  const k = await ev("V.dec ? V.dec.options.length : 0");
  if (k) { await ev(`(()=>{ pump(J(T.act(${rnd(k)}))); return 1 })()`); moves++; }
}
await waitFor("!!(V && !working)"); await pg.waitForTimeout(300);
const M = await ev("__M");
ok(`partie jouée jusqu'au bout contre l'IA forte (${moves} coups, tour ${await ev("V.st.t")}, ${await ev("V.st.pts.join('-')")})`, await ev("V.winner !== null && V.winner !== undefined"));
ok(`un bouton réagit pendant que l'IA joue (Journal ouvert au clic)`, journalOK);
console.log(`INFO  plus longue réponse du moteur : ${Math.round(M.call)} ms (${M.fn}) ; plus long blocage de la page pendant le jeu de l'IA : ${Math.round(M.gapAI)} ms, en tout : ${Math.round(M.gap)} ms ; ${probes} sondes, la plus lente ${slowProbe} ms`);
ok(`page jamais bloquée plus de 500 ms pendant le jeu de l'IA (${Math.round(M.gapAI)} ms)`, M.gapAI < 500);
ok(`sondes de la page pendant le jeu de l'IA : toutes sous 1 s (${slowProbe} ms)`, slowProbe < 1000);
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 3).join(" | ") : ""), errs.length === 0);
await pg.screenshot({ path: OUT + "/reactif-fin.png" });
await b.close();
