import { existsSync } from "node:fs";
// Robot des flèches (table, ordinateur 1893x899) : une partie jouée au hasard ; après chaque coup, chaque flèche doit partir
// d'une carte VISIBLE (sort de la chaîne : sa partie non rognée ; main adverse ; carte du plateau) et arriver sur une carte
// ou une zone du plateau. Retour utilisateur 2026-10-09 : les flèches des sorts de la chaîne partaient du vide sous la carte
// rognée, et celles du dernier coup de l'IA partaient d'un autre sort.
// Retour utilisateur 2026-10-09 (2) : la chaîne ne montre que les flèches d'UN élément, le haut par défaut ; survol (souris)
// ou toucher (téléphone) d'un autre élément → ses flèches à la place ; on quitte la chaîne → retour au haut. Contrôlé sur une
// chaîne de 2 éléments ciblants (posée dans l'état affiché, sur des unités réelles du plateau) en ordinateur et en téléphone
// 360x740 (toucher simulé), et pendant la partie au hasard (jamais deux éléments de la chaîne avec des flèches à la fois).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_fleches.mjs <dossier des captures> [port] [graines]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771", SEEDS = (process.argv[4] || "5").split(",");
let total = 0;
const ok = (msg, v) => { console.log((v ? "OK   " : "ÉCHEC") + " " + msg); if (!v) total++; };
// Chaîne de 2 éléments ciblants posée dans l'état affiché : [0] (dessous, IA) vise l'unité A, [1] (haut, joueur) vise B.
// Renvoie, pour l'état courant des flèches, de quel élément de la chaîne elles partent et sur quelle unité elles arrivent.
const ARR = `(()=>{
  const V2 = el => { const r = el.getBoundingClientRect(), mb = Math.min(0, parseFloat(getComputedStyle(el).marginBottom) || 0); return [r.left, r.top, r.right, r.top + r.height + mb]; };
  const its = [...document.querySelectorAll('#stack .sitem')].filter(x => x.getClientRects().length).map(x => [x.dataset.ch, V2(x.querySelector('.card'))]);
  return [...document.querySelectorAll('#arrowsG path')].map(p => { const m = p.getAttribute('d').match(/M([\\d.-]+),([\\d.-]+) Q[\\d.,-]+ ([\\d.-]+),([\\d.-]+)/).slice(1).map(Number);
    const it = its.find(([, c]) => m[0] >= c[0] - 1 && m[0] <= c[2] + 1 && m[1] >= c[1] - 1 && m[1] <= c[3] + 1);
    const u = [...document.querySelectorAll('#board .bf [data-uid], #board .zone [data-uid]')].filter(e => window.__AB.includes(e.dataset.uid)).find(e => { const r = e.getBoundingClientRect(); return Math.abs(r.left + r.width/2 - m[2]) < 20 + r.width/2 && Math.abs(r.top + r.height/2 - m[3]) < 20 + r.height/2; });
    return (it ? it[0] : '?') + '>' + (u ? (u.dataset.uid === window.__AB[0] ? 'A' : 'B') : '?'); }).sort().join(',');
})()`;
const CAN = `(()=>{ if (!V || working || mode || sel !== null || (V.st.chain||[]).length) return false;
  const us = [...document.querySelectorAll('#board .bf [data-uid], #board .zone [data-uid]')].filter(e => e.getClientRects().length && e.dataset.card);
  const d = []; us.forEach(e => { if (!d.some(x => x.dataset.uid === e.dataset.uid)) d.push(e); }); return d.length >= 2; })()`;
async function chainTest(pg, ev, phone, tag) {
  if (!phone) await pg.mouse.move(2, 2);
  await ev(`(()=>{ const us = []; [...document.querySelectorAll('#board .bf [data-uid], #board .zone [data-uid]')].filter(e => e.getClientRects().length && e.dataset.card).forEach(e => { if (!us.some(x => x.dataset.uid === e.dataset.uid)) us.push(e); });
    window.__AB = [us[0].dataset.uid, us[us.length - 1].dataset.uid]; window.__chain0 = V.st.chain;
    V.st.chain = [{ c: 1, k: 'spell', n: us[0].dataset.card, tg: [us[0].dataset.uid] }, { c: 0, k: 'spell', n: us[us.length - 1].dataset.card, tg: [us[us.length - 1].dataset.uid] }];
    show(V); drawArrows(); })()`);
  await pg.waitForTimeout(300);
  const a0 = await ev(ARR);
  ok(`${tag} chaîne de 2 éléments ciblants : par défaut seules les flèches du haut de la chaîne (${a0})`, a0 === "1>B");
  await pg.screenshot({ path: `${OUT}/chaine_${tag}_defaut.png` });
  // point de la partie VISIBLE de la carte (en ordinateur, les éléments anciens sont rognés : leur centre est caché)
  const at = async k => { await ev(`document.querySelector('#stack [data-ch="${k}"]').scrollIntoView({ block: 'nearest' })`);
    // premier point de la carte (de haut en bas) qui touche vraiment cet élément : sur téléphone, le haut de la carte
    // peut passer sous la main de l'adversaire
    return ev(`(()=>{ const r = document.querySelector('#stack [data-ch="${k}"] .card').getBoundingClientRect(), x = r.left + r.width / 2;
      for (let y = r.top + Math.min(14, r.height / 2); y < r.bottom; y += 4) { const e = document.elementFromPoint(x, y); if (e && e.closest('#stack [data-ch="${k}"]')) return [x, y]; }
      return [x, r.top + Math.min(14, r.height / 2)]; })()`); };
  const [x0, y0] = await at(0);
  if (phone) await pg.touchscreen.tap(x0, y0); else await pg.mouse.move(x0, y0);
  await pg.waitForTimeout(300);
  const a1 = await ev(ARR);
  ok(`${tag} ${phone ? "toucher" : "survol"} de l'élément du dessous : ses flèches à la place (${a1})`, a1 === "0>A");
  if (phone) ok(`${tag} le toucher ouvre toujours la carte en grand`, await ev("!document.getElementById('zoom').hidden"));
  await pg.screenshot({ path: `${OUT}/chaine_${tag}_${phone ? "toucher" : "survol"}.png` });
  if (phone) {
    // la carte ouverte en grand peut recouvrir le haut de la chaîne : un vrai joueur la ferme d'abord (toucher ailleurs)
    const [x1, y1] = await at(1);
    if (!(await ev(`!!document.elementFromPoint(${x1}, ${y1})?.closest('#stack [data-ch="1"]')`))) await ev("document.getElementById('zoom').hidden = true");
    ok(`${tag} carte en grand fermée : toujours les flèches de l'élément touché (${await ev(ARR)})`, (await ev(ARR)) === "0>A");
    await pg.touchscreen.tap(x1, y1);
  } else await pg.mouse.move(2, 2);
  await pg.waitForTimeout(300);
  const a2 = await ev(ARR);
  ok(`${tag} ${phone ? "toucher du haut de la chaîne" : "sortie du survol"} : retour aux flèches du haut (${a2})`, a2 === "1>B");
  await ev("V.st.chain = window.__chain0; show(V); drawArrows()");
  await pg.waitForTimeout(200);
}
async function step(pg, ev) {
  if (await ev("!!document.getElementById('bKeep')")) { await ev("document.getElementById('bKeep').click()"); return; }
  if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(V.ask.kind === 'mulligan' ? [] : 0)))) })()"); return; }
  if (await ev("!!V.dec")) { await ev(`(async()=>{ const o = V.dec.options; const k = o.findIndex(x => x.k === 'pass'); const r = Math.random(); const i = (r < .5 || k < 0) ? Math.floor(Math.random() * o.length) : o[k].i; await pump(J(T.act(i))) })()`); return; }
  await ev("(async()=>{ await pump(J(T.step())) })()");
}
let multiTot = 0;
for (const seed of SEEDS) {
  const pg = await (await b.newContext({ viewport: { width: 1893, height: 899 } })).newPage();
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error(s); };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev(`document.querySelector('.vopt').open = true; document.getElementById('sSeed').value='${seed}'`); await pg.click("#bStart");
  let bad = 0, n = 0, chainDone = false;
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
      const its = [...document.querySelectorAll('#stack .sitem')].filter(x => x.getClientRects().length).map(x => [x.dataset.ch, V2(x.querySelector('.card'))]);
      const from = new Set([...document.querySelectorAll('#arrowsG path')].map(p => p.getAttribute('d').match(/M([\\d.-]+),([\\d.-]+)/).slice(1).map(Number))
        .map(([x, y]) => (its.find(([, c]) => Math.abs(c[0]-x) <= c[2]/2 + 1 && Math.abs(c[1]-y) <= c[3]/2 + 1) || [null])[0]).filter(k => k !== null));
      const multi = (V.st.chain||[]).filter(c => (c.tg||[]).length).length >= 2;
      if (from.size > 1) return [{ plusieurs: [...from], chain: (V.st.chain||[]).map(c => c.n + ':' + JSON.stringify(c.tg)) }];
      window.__multi = (window.__multi || 0) + (multi ? 1 : 0);
      return [...document.querySelectorAll('#arrowsG path')].map(p => { const m = p.getAttribute('d').match(/M([\\d.-]+),([\\d.-]+) Q[\\d.,-]+ ([\\d.-]+),([\\d.-]+)/); const [x1,y1,x2,y2] = m.slice(1).map(Number);
        return { ok1: near(starts, x1, y1, 4), ok2: near(ends, x2, y2, 20), d: [x1,y1,x2,y2].map(Math.round), col: p.getAttribute('stroke') }; })
        .filter(a => !a.ok1 || !a.ok2).map(a => ({ ...a, chain: (V.st.chain||[]).map(c => c.n + ':' + JSON.stringify(c.tg)), ai: lastAI && JSON.stringify([lastAI.t1, lastAI.t2]) })) })()`);
    n++;
    if (!chainDone && await ev(CAN)) { chainDone = true; await chainTest(pg, ev, false, "ordinateur"); }
    if (r.length) { bad++; if (bad <= 4) { console.log("seed", seed, "étape", i, JSON.stringify(r).slice(0, 600)); await pg.screenshot({ path: `${OUT}/arrows_${seed}_${i}.png` }); } }
    if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
    if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(V.ask.kind === 'mulligan' ? [] : 0)))) })()"); continue; }
    if (await ev("!!V.dec")) { await ev(`(async()=>{ const o = V.dec.options; const k = o.findIndex(x => x.k === 'pass'); const r = Math.random(); const i = (r < .5 || k < 0) ? Math.floor(Math.random() * o.length) : o[k].i; await pump(J(T.act(i))) })()`); continue; }
    await ev("(async()=>{ await pump(J(T.step())) })()");
  }
  total += bad; multiTot += await ev("window.__multi || 0");
  ok("chaîne posée et contrôlée en ordinateur (graine " + seed + ")", chainDone);
  console.log((bad ? "ÉCHEC" : "OK   ") + " flèches : graine " + seed + ", " + n + " états contrôlés, " + bad + " avec une flèche mal placée");
}
console.log("info : états de la partie au hasard avec ≥ 2 éléments ciblants dans la chaîne : " + multiTot);
{ // téléphone 360x740, toucher simulé
  const pg = await (await b.newContext({ viewport: { width: 360, height: 740 }, isMobile: true, hasTouch: true })).newPage();
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error(s); };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.tap("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev(`document.querySelector('.vopt').open = true; document.getElementById('sSeed').value='${SEEDS[0]}'`); await pg.tap("#bStart");
  let done = false;
  for (let i = 0; i < 300 && !done; i++) {
    await waitFor("!!(V && !working)");
    if (await ev("V.st.winner !== null && V.st.winner !== undefined")) break;
    await pg.waitForTimeout(80);
    if (await ev(CAN)) { done = true; await chainTest(pg, ev, true, "téléphone"); break; }
    await step(pg, ev);
  }
  ok("chaîne posée et contrôlée en téléphone", done);
}
await b.close();
process.exit(total ? 1 : 0);
