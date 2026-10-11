import { existsSync } from "node:fs";
// Robot des sons de la table : les fichiers MP3 de l'ambiance se chargent et se décodent, chaque geste sur une carte
// (toucher, jouer) et la fin du tour font partir un son, le menu « ♪ » coupe le son et change d'ambiance (chargée à son
// tour), et ces choix restent après un rechargement. Les sons sont comptés en espionnant Web Audio (chaque son lancé =
// un appel à start()) : on vérifie qu'ils partent, pas comment ils sonnent.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_son.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
const pg = await (await b.newContext({ viewport: { width: 1400, height: 900 } })).newPage();
await pg.addInitScript(() => {
  window.__snd = 0;
  for (const C of [window.AudioScheduledSourceNode, window.AudioBufferSourceNode, window.OscillatorNode]) {
    const P = C && C.prototype;
    if (P && Object.prototype.hasOwnProperty.call(P, "start")) { const s = P.start; P.start = function (...a) { window.__snd++; return s.apply(this, a); }; }
  }
});
const errs = []; pg.on("pageerror", e => errs.push(String(e)));
const sfx = { ok: new Set(), bad: [] };
pg.on("response", r => { const m = r.url().match(/\/sfx\/(.+\.mp3)$/); if (m) (r.ok() ? sfx.ok.add(m[1]) : sfx.bad.push(m[1] + " " + r.status())); });
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
ok(`son activé par défaut, ambiance Studio (bouton ♪ : « ${await ev("$('bSon').getAttribute('aria-label')")} »)`, await ev("$('bSon').getAttribute('aria-label') === 'Sons : Studio'"));
await pg.waitForTimeout(1500);
const studio = [...sfx.ok].filter(f => f.startsWith("studio/")).length;
ok(`fichiers de l'ambiance Studio chargés : ${studio}/21, aucun en erreur (${sfx.bad.join(", ") || "rien"})`, studio === 21 && !sfx.bad.length);
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
await pg.click("#bSon"); await pg.waitForTimeout(150);
ok("menu ♪ ouvert : son activé et 6 ambiances", await ev("!$('sonPop').hidden && $('sonPop').querySelectorAll('[data-pk]').length === 6"));
ok(`barre de volume à ${await ev("$('sonVol') ? $('sonVol').value : 'absente'")} par défaut`, await ev("!!$('sonVol') && $('sonVol').value === '70'"));
const vb = await pg.locator("#sonVol").boundingBox();   // vrai geste : toucher la barre à 30 % de sa longueur
await pg.mouse.click(vb.x + vb.width * 0.3, vb.y + vb.height / 2); await pg.waitForTimeout(150);
const v30 = +(await ev("$('sonVol').value"));
ok(`volume réglé au doigt : ${v30} (chiffre affiché ${await ev("$('sonVolN').textContent")}, gardé ${await ev("localStorage.getItem('rbt-sonvol')")})`, v30 >= 20 && v30 <= 40 && await ev("$('sonVolN').textContent === $('sonVol').value && localStorage.getItem('rbt-sonvol') === $('sonVol').value"));
n = await snd();
await pg.click("#sonPop [data-pk=cinematic]"); await pg.waitForTimeout(1500);
const cin = [...sfx.ok].filter(f => f.startsWith("cinematic/")).length;
ok(`ambiance Cinéma : son d'essai (${(await snd()) - n}) et ses fichiers chargés (${cin}/21)`, (await snd()) > n && cin === 21 && !sfx.bad.length);
await pg.click("#sonPop [data-son]"); await pg.waitForTimeout(100);
ok("son coupé (bouton barré, « Sons : coupés »)", await ev("$('bSon').classList.contains('off') && $('bSon').getAttribute('aria-label') === 'Sons : coupés'"));
await pg.mouse.click(5, 300); await pg.waitForTimeout(100);
ok("toucher ailleurs ferme le menu ♪", await ev("$('sonPop').hidden"));
n = await snd();
await pg.click("#bUndo").catch(() => {}); await pg.waitForTimeout(200); await idle();
const c = await ev("(()=>{ const e = document.querySelector('#board [data-uid].can'); return e ? e.dataset.uid : null })()");
if (c) { await pg.click(`#board [data-uid="${c}"]`); await pg.waitForTimeout(150); }
ok(`son coupé : plus aucun son (Reprendre, toucher une carte : ${(await snd()) - n})`, (await snd()) === n);
await pg.screenshot({ path: OUT + "/son-coupe.png" });
await pg.reload(); await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
await ev("hideMenu()");
ok("après rechargement : le son reste coupé", await ev("$('bSon').getAttribute('aria-label') === 'Sons : coupés'"));
await pg.click("#bSon"); await pg.click("#sonPop [data-son]"); await pg.waitForTimeout(800);
ok(`après rechargement : volume gardé (${await ev("$('sonVol').value")})`, +(await ev("$('sonVol').value")) === v30);
ok(`rallumé : il sonne de nouveau, ambiance Cinéma gardée (« ${await ev("$('bSon').getAttribute('aria-label')")} »)`, (await snd()) > 0 && await ev("$('bSon').getAttribute('aria-label') === 'Sons : Cinéma'"));
await pg.screenshot({ path: OUT + "/son-menu.png" });
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
await b.close();
process.exitCode = fails ? 1 : 0;
