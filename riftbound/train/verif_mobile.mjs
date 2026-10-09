import { existsSync } from "node:fs";
// Playwright : celui de l'environnement cloud Claude, sinon le paquet npm « playwright » (CI).
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
// Robot de vérification de la table au format téléphone (Playwright, Chromium de /opt/pw-browsers).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_mobile.mjs <dossier des captures> [port]
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771", W = 360, H = 740;
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
const ctx = await b.newContext({ viewport: { width: W, height: H }, isMobile: true, hasTouch: true });
const pg = await ctx.newPage(); const errs = []; pg.on("pageerror", e => errs.push(String(e)));
const ok = (n, v) => console.log((v ? "OK   " : "ÉCHEC") + " " + n);
const ev = s => pg.evaluate(s);
const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(150); } throw new Error("attente : " + s); };
await pg.goto(`http://localhost:${PORT}/train.html`);
await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
// le lecteur des simulations n'est plus dans le menu : on revoit ses parties depuis « Mes parties » (verif_replay.mjs)
ok("menu : plus d'entrée Replays, « Mes parties » présent", await ev("!document.querySelector('#menu a[href=\"replays.html\"]') && !!document.querySelector('#menu [data-m=games]')"));
await pg.tap("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
await ev("document.getElementById('sSeed').value='7'"); await pg.tap("#bStart");
let seed = 12345; const rnd = n => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed % n; };
let moves = 0;
for (let i = 0; i < 2500; i++) {
  await waitFor("!!(V && !working)");
  if (await ev("V.winner !== null && V.winner !== undefined")) break;
  if (await ev("!!document.getElementById('bKeep')")) { await pg.tap("#bKeep"); continue; }
  if (await ev("!!V.ask")) { const k = await ev("V.ask.options.length"); await ev(`(async()=>{ await pump(J(T.answer(JSON.stringify(${rnd(k)})))) })()`); moves++; continue; }
  const k = await ev("V.dec ? V.dec.options.length : 0");
  if (k) { await ev(`(async()=>{ await pump(J(T.act(${rnd(k)}))) })()`); moves++; }
}
await waitFor("!!(V && !working)"); await pg.waitForTimeout(600);
const fin = await ev("V.winner !== null && V.winner !== undefined");
ok(`partie jouée jusqu'au bout (${moves} coups, tour ${await ev("V.st.t")}, ${await ev("V.st.pts.join('-')")})`, fin);
const geo = await ev(`(()=>{const f=document.getElementById('fbar').getBoundingClientRect(), c=document.querySelector('.ctrlcol').getBoundingClientRect(); return {fb:f.bottom, ct:c.top, fh:document.getElementById('fbar').hidden}})()`);
ok(`bulle de fin au-dessus de la barre de commande (bas bulle ${Math.round(geo.fb)} ≤ haut barre ${Math.round(geo.ct)})`, !geo.fh && geo.fb <= geo.ct + 1);
await pg.screenshot({ path: OUT + "/fin-de-partie-360.png" });
await pg.tap("#bAgain"); await waitFor("!!document.getElementById('bStart')", 10000).catch(() => {});
ok("« Nouvelle partie » de la bulle cliquable au doigt", await ev("!!document.getElementById('bStart')"));
await pg.keyboard.press("Escape"); await pg.waitForTimeout(300);
await ev("document.getElementById('bMenu') && document.getElementById('bMenu').click()"); 
await waitFor("!!document.querySelector('[data-m=deck]')", 10000); await pg.tap("[data-m=deck]"); await waitFor("!!document.getElementById('dkSave')");
await ev("(()=>{ DK = fromEngine(CAT.presets[1].deck, 'Test'); DK.id=''; dkRender(); })()"); await pg.waitForTimeout(400);
const sz = await ev("[...document.querySelectorAll('.dkr button, .rn button')].map(x=>{const r=x.getBoundingClientRect();return Math.min(r.width,r.height)}).filter(v=>v>0)");
ok(`éditeur : boutons ± ≥ 44 px (${sz.length} boutons, plus petit ${Math.min(...sz)} px)`, sz.length > 0 && Math.min(...sz) >= 44);
const hs = await ev("document.documentElement.scrollWidth <= innerWidth + 1");
ok("éditeur : pas de défilement horizontal", hs);
await pg.screenshot({ path: OUT + "/editeur-360.png" });
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
await b.close();
process.exitCode = errs.length ? 1 : 0;
