import { existsSync } from "node:fs";
// Robot de charge au format téléphone : 17 cartes en main, 6 unités de chaque côté sur chaque champ de bataille,
// 7 unités en base, 12 runes dont une partie épuisée. Aucune carte ne doit sortir de sa zone, aucune zone ne doit
// déborder et la page ne doit pas défiler (393x851, 360x740, 412x915). Situation posée dans le moteur (Pyodide).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_charge.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2], PORT = process.argv[3] || "8791";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0;
for (const [w, h] of [[393, 851], [360, 740], [412, 915]]) {
  const pg = await (await b.newContext({ viewport: { width: w, height: h }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 })).newPage();
  const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error(s); };
  const py = code => ev(`(()=>{ T.__builtins__.get('exec')(${JSON.stringify(code)}, T.__dict__); return true })()`);
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev("document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'"); await pg.click("#bStart");
  for (let i = 0; i < 200; i++) {
    await waitFor("!!(V && !working)");
    if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
    if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; }
    if (await ev("!!(V.dec && V.dec.kind === 'main')")) break;
    await ev("(async()=>{ await pump(J(T.act(0))) })()");
  }
  await py(`
from game import Obj, Rune
g = W["g"]; me = ME; op = 1 - ME
names = ["Stellacorn Herder", "Mournful Witness", "Noxus Hopeful", "Blitzcrank, Impassive", "Kai'Sa, Survivor", "Astral Heron"]
for k in range(12):
    c = Obj(["Falling Star", "Discipline", "Block", "Stellacorn Herder", "Not So Fast", "Mournful Witness"][k % 6], me); c.zone = "hand"; g.p[me].hand.append(c)
for loc in (0, 1):
    for k in range(6):
        g.enter_board(Obj(names[k % 6], me), me, loc, ready=True); g.enter_board(Obj("Watchful Sentry", op), op, loc, ready=(k % 2 == 0))
for k in range(7):
    g.enter_board(Obj(names[k % 6], me), me, "base", ready=(k % 3 != 0))
    g.enter_board(Obj("Watchful Sentry", op), op, "base", ready=True)
g.p[me].runes = [Rune(["Calm", "Fury"][k % 2], me) for k in range(12)]
for k, r in enumerate(g.p[me].runes): r.exhausted = (k % 4 == 0)
W["d"] = None
`);
  await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(500);
  const m = JSON.parse(await ev(`(()=>{
    const out = [];
    const check = (sel, kids) => document.querySelectorAll(sel).forEach(z => { const r = z.getBoundingClientRect(); z.querySelectorAll(kids).forEach(c => { const q = c.getBoundingClientRect(); if (q.width && (q.left < r.left - 2 || q.right > r.right + 2)) out.push(sel + ' : ' + (c.dataset.card || c.className) + ' ' + Math.round(q.left) + '-' + Math.round(q.right) + ' hors ' + Math.round(r.left) + '-' + Math.round(r.right)); }); });
    check('.half.p0 .hand', ':scope > .card'); check('.zrow > .zone:nth-child(3)', ':scope > .slot'); check('.bf .side', ':scope > .slot'); check('.zrow > .zone:nth-child(2)', ':scope > .runeslot');
    const unit = document.querySelector('.bf .side > .slot'), hc = document.querySelector('.half.p0 .hand > .card');
    const sc = [...document.querySelectorAll('.zone, .bf .side, .hand')].filter(e => e.scrollWidth > e.clientWidth + 1).map(e => e.className + '[' + (e.closest('.half') ? e.closest('.half').className : '') + '] ' + e.scrollWidth + '/' + e.clientWidth + ' ' + getComputedStyle(e).overflowX + ' ' + [...e.children].map(c => c.className + ':' + Math.round(c.getBoundingClientRect().right - e.getBoundingClientRect().left)).join(','));
    return JSON.stringify({ hors: out.slice(0, 6), nHors: out.length, page: document.documentElement.scrollHeight > innerHeight + 1 || document.documentElement.scrollWidth > innerWidth + 1,
      unite: unit ? Math.round(unit.getBoundingClientRect().width) + 'x' + Math.round(unit.getBoundingClientRect().height) : null,
      main: hc ? Math.round(hc.getBoundingClientRect().width) + 'x' + Math.round(hc.getBoundingClientRect().height) : null, defile: sc }) })()`));
  const bad = m.nHors > 0 || m.page || m.defile.length > 0 || errs.length;
  if (bad) fails++;
  console.log((bad ? "ÉCHEC " : "OK    ") + `${w}x${h}`, JSON.stringify(m), "erreurs JS", errs.length);
  await pg.screenshot({ path: `${OUT}/charge-${w}.png` });
}
await b.close();
process.exitCode = fails ? 1 : 0;
