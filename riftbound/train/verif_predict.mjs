import { existsSync } from "node:fs";
// Robot Predict / Vision (règles 436, 817) dans la vraie table : un Predict 2 posé dans le moteur ; la question montre les
// deux cartes du dessus (images, la carte en cours surlignée) et deux boutons « la garder sur le dessus » / « la recycler » ;
// après « recycler » la 1re et « garder » la 2e, le deck porte l'œil « 👁 1 » et le toucher ouvre le dessus connu.
// Ordinateur (1400x900, clics) et téléphone (393x851, toucher simulé). Retour utilisateur 2026-10-09.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_predict.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
for (const [w, h, mob] of [[1400, 900, false], [393, 851, true]]) {
  const pg = await (await b.newContext({ viewport: { width: w, height: h }, isMobile: mob, hasTouch: mob })).newPage();
  const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
  const tap = async sel => { const l = pg.locator(sel).first(); if (mob) { const bb = await l.boundingBox(); await pg.touchscreen.tap(bb.x + bb.width / 2, bb.y + bb.height / 2); } else await l.click(); await pg.waitForTimeout(250); };
  const py = code => ev(`(()=>{ T.__builtins__.get('exec')(${JSON.stringify(code)}, T.__dict__); return true })()`);
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev("document.querySelector('.vopt').open = true; document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'"); await pg.click("#bStart");
  for (let i = 0; i < 200; i++) { await waitFor("!!(V && !working)"); if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; } if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; } if (await ev("!!(V.dec && V.dec.kind === 'main')")) break; await ev("(async()=>{ await pump(J(T.act(0))) })()"); }
  await py(`
g = W["g"]; me = ME
TOP = [c.cname for c in g.p[me].deck[:2]]
g.queue_trigger(me, "Dramatic Visionary", lambda g_, it: g_.predict(it.ctrl, 2))
g.flush_triggers()
W["d"] = None`);
  const top = await ev("T.TOP.toJs()");
  await ev("(async()=>{ await pump(J(T.step())) })()");
  for (let i = 0; i < 10; i++) {   // le déclenchement est sur la chaîne : passer la priorité jusqu'à sa résolution
    await waitFor("!!(V && !working)"); if (await ev("!!V.ask")) break;
    await ev("(async()=>{ const o = (V.dec.options || []).find(x => x.k === 'pass'); await pump(J(o ? T.act(o.i) : T.step())) })()");
  }
  await waitFor("!!(V && !working && V.ask)"); await pg.waitForTimeout(300);
  const q = await ev(`(()=>{ const p = document.querySelector('#fbar .mull.seen') || document.querySelector('.mull.seen'); if (!p) return null;
    return { cards: [...p.querySelectorAll('.card')].map(c => c.dataset.card), img: [...p.querySelectorAll('.card')].every(c => /url/.test(getComputedStyle(c).backgroundImage)),
      cur: [...p.querySelectorAll('.seenc')].findIndex(x => x.classList.contains('cur')), btn: [...p.parentElement.querySelectorAll('button[data-a]')].map(x => x.textContent),
      vis: (() => { const r = p.getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight && r.left >= 0 && r.right <= innerWidth; })() } })()`);
  ok(`${w} px : la question Predict montre les 2 cartes du dessus (${q && q.cards}), avec leur image, la 1re surlignée`, q && JSON.stringify(q.cards) === JSON.stringify(top) && q.img && q.cur === 0);
  ok(`${w} px : boutons « garder sur le dessus » / « recycler » (${q && q.btn}), question entière à l'écran`, q && q.btn.length === 2 && /garder/i.test(q.btn[0]) && /recycler/i.test(q.btn[1]) && q.vis);
  await pg.screenshot({ path: `${OUT}/predict-question-${w}.png` });
  await tap('button[data-a="1"]'); await waitFor("!!(V && !working)"); await pg.waitForTimeout(300);
  const cur2 = await ev("(()=>{ const p = document.querySelector('.mull.seen'); return p ? [...p.querySelectorAll('.seenc')].findIndex(x => x.classList.contains('cur')) : -1 })()");
  ok(`${w} px : 2e question, la 2e carte surlignée`, cur2 === 1);
  await tap('button[data-a="0"]'); await waitFor("!!(V && !working)");
  for (let i = 0; i < 10 && await ev("!!(V.dec && V.dec.kind !== 'main')"); i++) { await ev("(async()=>{ const o = V.dec.options.find(x => x.k === 'pass'); await pump(J(T.act(o.i))) })()"); await waitFor("!!(V && !working)"); }
  await pg.waitForTimeout(300);
  const dk = await ev("(()=>{ const d = document.querySelector('#board .half.p0 .pl.deck.known'); return d ? { eye: d.querySelector('.eye').textContent, title: d.title } : null })()");
  ok(`${w} px : le deck montre le dessus connu (${dk && dk.eye}) : ${top[1]}`, dk && /1/.test(dk.eye) && dk.title.includes(top[1]) && !dk.title.includes(top[0] + ","));
  await tap("#board .half.p0 .pl.deck.known");
  const md = await ev("$('modal').hidden ? null : [...$('mbox').querySelectorAll('.card')].map(c => c.dataset.card)");
  ok(`${w} px : toucher le deck ouvre le dessus connu (${md})`, md && md.length === 1 && md[0] === top[1]);
  await pg.screenshot({ path: `${OUT}/predict-dessus-${w}.png` });
  ok(`${w} px : aucune erreur JS (${errs.join(" | ")})`, !errs.length);
}
await b.close();
console.log(fails ? `${fails} ÉCHEC(S)` : "tout est OK");
process.exit(fails ? 1 : 0);
