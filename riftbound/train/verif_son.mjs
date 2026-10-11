import { existsSync } from "node:fs";
// Robot des sons de la table : chaque geste sur une carte (toucher, jouer) et la fin du tour font partir un son,
// le bouton « ♪ » les coupe et le choix reste après un rechargement. Les sons sont comptés en espionnant Web Audio
// (chaque oscillateur ou bruit lancé = un appel à start()) : on vérifie qu'ils partent, pas comment ils sonnent.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_son.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
const pg = await (await b.newContext({ viewport: { width: 1400, height: 900 } })).newPage();
await pg.addInitScript(() => {
  window.__snd = 0;
  const P = window.AudioScheduledSourceNode && AudioScheduledSourceNode.prototype;
  if (P) { const s = P.start; P.start = function (...a) { window.__snd++; return s.apply(this, a); }; }
});
const errs = []; pg.on("pageerror", e => errs.push(String(e)));
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
const ev = s => pg.evaluate(s);
const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(150); } throw new Error("attente : " + s); };
const idle = () => waitFor("!!(V && !working)");
const snd = () => ev("window.__snd");
async function start(seed) {
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev(`document.getElementById('sSeed').value='${seed}'; document.getElementById('sFirst').value='0'`); await pg.click("#bStart");
  for (let i = 0; i < 200; i++) {
    await idle();
    if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
    if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; }
    if (await ev("!!(V.dec && V.dec.kind === 'main' && V.dec.options.some(o => o.k === 'play'))")) return;
    await ev("(async()=>{ await pump(J(T.act(V.dec.options.find(o => o.k === 'end' || o.k === 'pass') ? V.dec.options.find(o => o.k === 'end' || o.k === 'pass').i : 0))) })()");
  }
  throw new Error("pas de carte jouable");
}
await start(process.env.RB_SEED || "4");
ok(`son activé par défaut (bouton ♪ : aria-pressed=${await ev("$('bSon').getAttribute('aria-pressed')")})`, await ev("$('bSon').getAttribute('aria-pressed') === 'true'"));
ok(`sons pendant le début de partie (${await snd()} lancés)`, await snd() > 0);
// une carte de ma main qui se joue sans cible : la toucher, puis « Jouer »
const uid = await ev(`(() => { const o = V.dec.options.find(o => o.k === 'play' && !(o.t1 && o.t1.length)); return o ? String(o.src) : null })()`);
let n = await snd();
await pg.click(`#board .half.p0 .hand [data-uid="${uid}"]`); await pg.waitForTimeout(200);
ok(`toucher une carte de la main : un son (${(await snd()) - n})`, (await snd()) > n);
n = await snd();
const played = await ev("(()=>{ const b = [...document.querySelectorAll('#pop button[data-i]')].find(b => /Jouer/.test(b.textContent)); if (b) { b.click(); return b.textContent } return '' })()");
await pg.waitForTimeout(150);
ok(`jouer la carte (« ${played} ») : un son tout de suite (${(await snd()) - n})`, !!played && (await snd()) > n);
for (let i = 0; i < 30 && !(await ev("!!(V && !working && V.dec && V.dec.options.some(o => o.k === 'end'))")); i++) {
  await idle();
  if (await ev("!!V.ask")) await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()");
  else if (await ev("!!(V.dec && V.dec.options.some(o => o.k === 'pass'))")) await pg.click("#bMain");
}
n = await snd();
await pg.click("#bMain"); await pg.waitForTimeout(150);
ok(`Terminer le tour : un son (${(await snd()) - n})`, (await snd()) > n);
await idle();
await pg.click("#bSon"); await pg.waitForTimeout(100);
ok("bouton ♪ : son coupé (aria-pressed=false, classe off)", await ev("$('bSon').getAttribute('aria-pressed') === 'false' && $('bSon').classList.contains('off')"));
n = await snd();
await pg.click("#bUndo").catch(() => {}); await pg.waitForTimeout(200); await idle();
const c = await ev("(()=>{ const e = document.querySelector('#board [data-uid].can'); return e ? e.dataset.uid : null })()");
if (c) { await pg.click(`#board [data-uid="${c}"]`); await pg.waitForTimeout(150); }
ok(`son coupé : plus aucun son (Reprendre, toucher une carte : ${(await snd()) - n})`, (await snd()) === n);
await pg.screenshot({ path: OUT + "/son-coupe.png" });
await pg.reload(); await waitFor("!!document.getElementById('bSon')");
ok("après rechargement : le son reste coupé", await ev("$('bSon').getAttribute('aria-pressed') === 'false'"));
await pg.click("#bSon");
ok("rallumé : il sonne de nouveau", await ev("$('bSon').getAttribute('aria-pressed') === 'true'") && (await snd()) > 0);
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
await b.close();
process.exitCode = fails ? 1 : 0;
