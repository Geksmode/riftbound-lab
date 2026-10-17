import { existsSync } from "node:fs";
// Playwright : celui de l'environnement cloud Claude, sinon le paquet npm « playwright » (CI).
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
// Robot : pendant le mulligan, on peut lire chaque carte (bouton « Lire » sous la carte au doigt, survol à la souris),
// sans la mettre de côté ; toucher la carte la met toujours de côté. Téléphone 360×740 (toucher) puis ordinateur 1366×800.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_mulligan.mjs <dossier des captures> [port]
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0;
const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
for (const phone of [true, false]) {
  const tag = phone ? "téléphone" : "ordinateur";
  const ctx = await b.newContext(phone ? { viewport: { width: 360, height: 740 }, isMobile: true, hasTouch: true } : { viewport: { width: 1366, height: 800 } });
  const pg = await ctx.newPage(); const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const tap = sel => phone ? pg.tap(sel) : pg.click(sel);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(150); } throw new Error("attente : " + s); };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await tap("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev("document.getElementById('sSeed').value='7'"); await tap("#bStart");
  await waitFor("!!document.getElementById('bKeep')");
  const names = await ev("[...document.querySelectorAll('#fbar .mull.mm [data-m]')].map(x => x.querySelector('[data-card]').dataset.card)");
  const rd = await ev("[...document.querySelectorAll('#fbar .mull.mm [data-rd]')].map(x => { const r = x.getBoundingClientRect(); return { w: r.width, h: r.height, b: r.bottom } })");
  ok(`${tag} : un bouton « Lire » sous chacune des ${names.length} cartes`, names.length >= 4 && rd.length === names.length && rd.every(r => r.w > 0 && r.b <= innerHeightOf(phone)));
  // lire chaque carte : l'agrandi montre son nom et son texte, la carte n'est pas mise de côté
  let allRead = true;
  for (let i = 0; i < names.length; i++) {
    await tap(`#fbar [data-rd="${i}"]`); await pg.waitForTimeout(150);
    const z = await ev("(()=>{ const z = document.getElementById('zoom'); return { on: !z.hidden, h: (z.querySelector('h4') || {}).textContent, t: (z.querySelector('.zt') || {}).textContent || '' } })()");
    if (!(z.on && z.h === names[i] && z.t.length > 0)) { allRead = false; console.log("   carte " + i, names[i], JSON.stringify(z)); }
    if (i === 0) await pg.screenshot({ path: `${OUT}/mulligan-lire-${phone ? 360 : 1366}.png` });
  }
  ok(`${tag} : « Lire » ouvre l'agrandi de la bonne carte, avec son texte (${names.join(", ")})`, allRead);
  ok(`${tag} : lire ne met aucune carte de côté`, await ev("mode.mull.size === 0 && document.getElementById('bKeep').textContent.includes('Garder')"));
  await tap(`#fbar [data-rd="${names.length - 1}"]`); await pg.waitForTimeout(150);
  ok(`${tag} : le même bouton referme l'agrandi`, await ev("document.getElementById('zoom').hidden"));
  // ouvert puis carte touchée : l'agrandi se ferme, la carte est mise de côté
  await tap('#fbar [data-rd="0"]'); await tap('#fbar [data-m="1"]'); await pg.waitForTimeout(150);
  ok(`${tag} : toucher une carte la met de côté et ferme l'agrandi`, await ev("mode.mull.has(1) && mode.mull.size === 1 && document.getElementById('zoom').hidden"));
  if (!phone) {
    await pg.hover('#fbar [data-m="2"] [data-card]'); await pg.waitForTimeout(150);
    ok(`${tag} : survoler une carte du mulligan l'agrandit`, await ev(`!document.getElementById('zoom').hidden && document.querySelector('#zoom h4').textContent === ${JSON.stringify(names[2])}`));
  }
  ok(`${tag} : la bulle tient à l'écran`, await ev("(()=>{ const r = document.getElementById('fbar').getBoundingClientRect(); return r.left >= 0 && r.right <= innerWidth + 1 && r.top >= 0 && r.bottom <= innerHeight + 1 })()"));
  await tap("#bKeep"); await waitFor("!(V && V.ask && V.ask.kind === 'mulligan') && !working");
  ok(`${tag} : mulligan validé (1 carte remise), partie lancée`, await ev("!document.getElementById('bKeep')"));
  ok(`${tag} : aucune erreur JavaScript` + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
  await ctx.close();
}
function innerHeightOf(phone) { return phone ? 740 : 800; }
await b.close();
process.exitCode = fails ? 1 : 0;
