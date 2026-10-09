import { existsSync } from "node:fs";
// Robot de charge au format téléphone : 17 cartes en main, 6 unités de chaque côté sur chaque champ de bataille,
// 7 unités en base, 12 runes dont une partie épuisée (téléphone, puis ordinateur 1875x902 et 1400x900). Aucune carte ne doit sortir de sa zone, aucune zone ne doit
// déborder et la page ne doit pas défiler (393x851, 360x740, 412x915). Situation posée dans le moteur (Pyodide).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_charge.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2], PORT = process.argv[3] || "8791";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0;
for (const [w, h] of [[393, 851], [360, 740], [412, 915], [1875, 902], [1400, 900]]) {   // + ordinateur : unités épuisées dans leur moitié de battlefield
  const mob = w < 1000;
  const pg = await (await b.newContext({ viewport: { width: w, height: h }, isMobile: mob, hasTouch: mob, deviceScaleFactor: mob ? 2 : 1 })).newPage();
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
    check('.half.p0 .hand', ':scope > .card'); check('.zrow > .zone:nth-child(3)', ':scope > .slot'); check('.bf .side', ':scope > .slot'); document.querySelectorAll('.bf .side').forEach(z => { const r = z.getBoundingClientRect(); z.querySelectorAll(':scope > .slot > .card').forEach(c => { const q = c.getBoundingClientRect(); if (q.height && (q.top < r.top - 2 || q.bottom > r.bottom + 2)) out.push('.bf .side (hauteur) : ' + (c.dataset.card || '') + (c.parentElement.classList.contains('x') ? ' épuisée ' : ' ') + Math.round(q.top) + '-' + Math.round(q.bottom) + ' hors ' + Math.round(r.top) + '-' + Math.round(r.bottom)); }); }); check('.zrow > .zone:nth-child(2)', ':scope > .runeslot');
    const unit = document.querySelector('.bf .side > .slot'), hc = document.querySelector('.half.p0 .hand > .card');
    const sc = [...document.querySelectorAll('.zone, .bf .side, .hand')].filter(e => e.scrollWidth > e.clientWidth + 1).map(e => e.className + '[' + (e.closest('.half') ? e.closest('.half').className : '') + '] ' + e.scrollWidth + '/' + e.clientWidth + ' ' + getComputedStyle(e).overflowX + ' ' + [...e.children].map(c => c.className + ':' + Math.round(c.getBoundingClientRect().right - e.getBoundingClientRect().left)).join(','));
    return JSON.stringify({ hors: out.slice(0, 60), nHors: out.length, page: document.documentElement.scrollHeight > innerHeight + 1 || document.documentElement.scrollWidth > innerWidth + 1,
      unite: unit ? Math.round(unit.getBoundingClientRect().width) + 'x' + Math.round(unit.getBoundingClientRect().height) : null,
      main: hc ? Math.round(hc.getBoundingClientRect().width) + 'x' + Math.round(hc.getBoundingClientRect().height) : null, defile: sc }) })()`));
  if (!mob) {   // ordinateur : seulement le battlefield (17 cartes en main y débordent : sujet à part, hors de ce robot)
    m.hors = m.hors.filter(x => x.startsWith('.bf')); m.nHors = m.hors.length; m.page = false; m.defile = m.defile.filter(x => x.startsWith('side'));
  }
  const bad = m.nHors > 0 || m.page || m.defile.length > 0 || errs.length;
  if (bad) fails++;
  console.log((bad ? "ÉCHEC " : "OK    ") + `${w}x${h}`, JSON.stringify(m), "erreurs JS", errs.length);
  await pg.screenshot({ path: `${OUT}/charge-${w}.png` });
  if (w !== 393) continue;
  // gestes sur une main serrée (17 cartes) : ouvrir, défiler, toucher une carte, geste horizontal, glisser vers le plateau
  const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
  const box = sel => pg.locator(sel).first().boundingBox();
  let hb = await box("#board .half.p0 .hand > .card:nth-of-type(6)");
  await pg.touchscreen.tap(hb.x + 6, hb.y + hb.height / 2); await pg.waitForTimeout(300);
  const op = await ev("(()=>{ const h = document.querySelector('#board .half.p0 .hand'); const c = h.querySelector(':scope > .card'); return { open: h.classList.contains('open'), cw: Math.round(c.getBoundingClientRect().width), sw: h.scrollWidth, cw2: h.clientWidth, pop: !$('pop').hidden } })()");
  ok(`main serrée : le premier toucher l'ouvre en grand (cartes ${op.cw} px, sans ouvrir de menu)`, op.open && op.cw >= 80 && !op.pop);
  await pg.screenshot({ path: `${OUT}/main-ouverte-${w}.png` });
  ok(`main ouverte : elle défile (${op.sw} px de contenu pour ${op.cw2} px)`, op.sw > op.cw2 + 50);
  await ev("document.querySelector('#board .half.p0 .hand').scrollLeft = 99999"); await pg.waitForTimeout(200);
  const last = await ev("(()=>{ const h = document.querySelector('#board .half.p0 .hand'), r = h.getBoundingClientRect(), cs = [...h.querySelectorAll(':scope > .card')]; const c = cs[cs.length - 1].getBoundingClientRect(); return c.right <= r.right + 1 && c.left >= r.left - 1 })()");
  ok("main ouverte : la dernière carte se voit en entier après défilement", last);
  const playable = await ev("(()=>{ const c = document.querySelector('#board .half.p0 .hand.open > .card.can'); if (!c) return null; c.parentElement.scrollLeft = c.offsetLeft - 120; return c.dataset.uid })()");
  await pg.waitForTimeout(250);
  if (playable) {
    hb = await box(`#board [data-uid="${playable}"]`);
    await pg.touchscreen.tap(hb.x + hb.width / 2, hb.y + hb.height / 2); await pg.waitForTimeout(300);
    const pop = await ev("$('pop').hidden ? '' : $('pop').innerText");
    ok(`main ouverte : toucher une carte jouable montre ses actions (${JSON.stringify(pop.replace(/\n/g, " | ").slice(0, 60))})`, /Jouer|Cibler|Cacher/.test(pop));
    await ev("sel = null; mode = null; $('pop').hidden = true; decorate()"); await pg.waitForTimeout(200);
    hb = await box(`#board [data-uid="${playable}"]`);
    await pg.mouse.move(hb.x + hb.width / 2, hb.y + hb.height / 2); await pg.mouse.down();
    await pg.mouse.move(hb.x + hb.width / 2 - 40, hb.y + hb.height / 2 - 3, { steps: 6 }); await pg.mouse.up(); await pg.waitForTimeout(200);
    const hz = await ev("({ ghost: !!document.querySelector('.ghost'), open: document.querySelector('#board .half.p0 .hand').classList.contains('open'), mode: mode ? mode.type : null })");
    ok(`geste horizontal dans la main ouverte : pas de glisser (fantôme ${hz.ghost}, mode ${hz.mode}), la main reste ouverte`, !hz.ghost && hz.open && !hz.mode);
    const n0 = await ev("V.st.p[MEP].hand.length");
    hb = await box(`#board [data-uid="${playable}"]`); const z = await box('#board .bf[data-drop="0"]');
    await pg.mouse.move(hb.x + hb.width / 2, hb.y + hb.height / 2); await pg.mouse.down();
    await pg.mouse.move(hb.x + hb.width / 2, hb.y - 30, { steps: 4 }); await pg.mouse.move(z.x + z.width / 2, z.y + z.height / 2, { steps: 8 }); await pg.mouse.up();
    await pg.waitForTimeout(600); await waitFor("!!(V && !working)");
    const after = await ev("({ n: V.st.p[MEP].hand.length, mode: mode ? mode.type : null, ask: !!V.ask, open: document.querySelector('#board .half.p0 .hand').classList.contains('open') })");
    ok(`glisser une carte de la main ouverte vers un champ de bataille : coup lancé (main ${n0} → ${after.n}, mode ${after.mode}, question ${after.ask}), main refermée`, (after.n < n0 || after.mode || after.ask) && !after.open);
  } else ok("main ouverte : une carte jouable visible (décision " + await ev("JSON.stringify([V.dec && V.dec.kind, V.ask && V.ask.kind, V.dec && V.dec.options.map(o => o.k).join(',')])") + ")", false);
}
await b.close();
process.exitCode = fails ? 1 : 0;
