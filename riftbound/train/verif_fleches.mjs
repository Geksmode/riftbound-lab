import { existsSync } from "node:fs";
// Robot des flèches (table, ordinateur 1893x899) : une partie jouée au hasard ; après chaque coup, chaque flèche doit partir
// d'une carte VISIBLE (sort de la chaîne : sa partie non rognée ; main adverse ; carte du plateau) et arriver sur une carte
// ou une zone du plateau. Retour utilisateur 2026-10-09 : les flèches des sorts de la chaîne partaient du vide sous la carte
// rognée, et celles du dernier coup de l'IA partaient d'un autre sort.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_fleches.mjs <dossier des captures> [port] [graines]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771", SEEDS = (process.argv[4] || "5").split(",");
let total = 0;
for (const seed of SEEDS) {
  const pg = await (await b.newContext({ viewport: { width: 1893, height: 899 } })).newPage();
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error(s); };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev(`document.querySelector('.vopt').open = true; document.getElementById('sSeed').value='${seed}'`); await pg.click("#bStart");
  let bad = 0, n = 0;
  for (let i = 0; i < 160; i++) {
    await waitFor("!!(V && !working)");
    if (await ev("V.st.winner !== null && V.st.winner !== undefined")) break;
    await pg.waitForTimeout(150);
    const r = await ev(`(()=>{
      const C = el => { const r = el.getBoundingClientRect(); return [r.left + r.width/2, r.top + r.height/2, r.width, r.height]; };
      const V2 = el => { const r = el.getBoundingClientRect(), mb = Math.min(0, parseFloat(getComputedStyle(el).marginBottom) || 0), h = r.height + mb; return [r.left + r.width/2, r.top + h/2, r.width, h]; };
      const starts = [...[...document.querySelectorAll('#stack .card')].map(V2), ...[...document.querySelectorAll('#board .hand.top, #board .card')].map(C)];
      const ends = [...document.querySelectorAll('#board .card, #board .zone, #board .bf')].map(C);
      const near = (L, x, y, tol) => L.some(c => Math.abs(c[0]-x) <= Math.max(tol, c[2]/2) && Math.abs(c[1]-y) <= Math.max(tol, c[3]/2));
      return [...document.querySelectorAll('#arrowsG path')].map(p => { const m = p.getAttribute('d').match(/M([\\d.-]+),([\\d.-]+) Q[\\d.,-]+ ([\\d.-]+),([\\d.-]+)/); const [x1,y1,x2,y2] = m.slice(1).map(Number);
        return { ok1: near(starts, x1, y1, 4), ok2: near(ends, x2, y2, 20), d: [x1,y1,x2,y2].map(Math.round), col: p.getAttribute('stroke') }; })
        .filter(a => !a.ok1 || !a.ok2).map(a => ({ ...a, chain: (V.st.chain||[]).map(c => c.n + ':' + JSON.stringify(c.tg)), ai: lastAI && JSON.stringify([lastAI.t1, lastAI.t2]) })) })()`);
    n++;
    if (r.length) { bad++; if (bad <= 4) { console.log("seed", seed, "étape", i, JSON.stringify(r).slice(0, 600)); await pg.screenshot({ path: `${OUT}/arrows_${seed}_${i}.png` }); } }
    if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
    if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(V.ask.kind === 'mulligan' ? [] : 0)))) })()"); continue; }
    if (await ev("!!V.dec")) { await ev(`(async()=>{ const o = V.dec.options; const k = o.findIndex(x => x.k === 'pass'); const r = Math.random(); const i = (r < .5 || k < 0) ? Math.floor(Math.random() * o.length) : o[k].i; await pump(J(T.act(i))) })()`); continue; }
    await ev("(async()=>{ await pump(J(T.step())) })()");
  }
  total += bad;
  console.log((bad ? "ÉCHEC" : "OK   ") + " flèches : graine " + seed + ", " + n + " états contrôlés, " + bad + " avec une flèche mal placée");
}
await b.close();
process.exit(total ? 1 : 0);
