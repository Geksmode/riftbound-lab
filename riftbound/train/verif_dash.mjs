import { existsSync } from "node:fs";
// Robot du tableau de bord du menu (ordinateur et téléphone) : un nouveau joueur n'a aucun deck d'entraînement par défaut ;
// « Choisir mon deck » → légende → liste ; le tableau de bord montre ce deck et son bilan ; « Jouer avec ce deck » ouvre la
// sélection contre l'IA avec ce deck. Pas de défilement horizontal, aucune erreur JS.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_dash.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
for (const [W, H, mob] of [[360, 740, true], [1280, 800, false]]) {
  const ctx = await b.newContext({ viewport: { width: W, height: H }, isMobile: mob, hasTouch: mob });
  const pg = await ctx.newPage(); const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(150); } throw new Error("attente : " + s); };
  const tap = s => mob ? pg.tap(s) : pg.click(s);
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await waitFor("!!CAT"); await pg.waitForTimeout(800);
  ok(W + " : nouveau joueur, aucun deck choisi", await ev("!!document.querySelector('.deckc.none') && !store.get('rbt-train')"));
  ok(W + " : pas de défilement horizontal", await ev("document.getElementById('menu').scrollWidth <= innerWidth"));
  await pg.screenshot({ path: `${OUT}/dash-vide-${W}.png`, fullPage: true });
  await tap(".deckc [data-m=pick]"); await waitFor("!!document.querySelector('.pick .roster')");
  await tap('[data-pleg="Akali, Rogue Assassin"]'); await waitFor("!!document.querySelector('[data-pick]')");
  await pg.screenshot({ path: `${OUT}/choix-${W}.png`, fullPage: true });
  const k = await ev("document.querySelector('[data-pick]:not([disabled])').dataset.pick");
  await tap(`[data-pick="${k}"]`); await waitFor("!!document.querySelector('.deckc:not(.none)')"); await pg.waitForTimeout(800);
  ok(W + " : deck choisi " + k, await ev(`store.get('rbt-train') === ${JSON.stringify(k)}`));
  ok(W + " : bilan affiché", await ev("document.getElementById('dBil').textContent.length > 0"));
  await pg.screenshot({ path: `${OUT}/dash-deck-${W}.png`, fullPage: true });
  await tap("[data-m=trainplay]"); await waitFor("!!document.getElementById('bStart')");
  ok(W + " : « Jouer avec ce deck » ouvre la sélection avec ce deck", await ev(`document.getElementById('sMine').value === ${JSON.stringify(k)}`));
  ok(W + " : aucune erreur JS", errs.length === 0); if (errs.length) console.log(errs);
  await ctx.close();
}
await b.close();
process.exit(fails ? 1 : 0);
