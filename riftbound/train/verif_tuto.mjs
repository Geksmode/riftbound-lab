import { existsSync } from "node:fs";
// Robot du tutoriel (menu « Apprendre à jouer ») dans la vraie table : intro, mulligan, défis dans l'ordre, coach et puce,
// fin du tutoriel. Menu, intro, « Garder ma main », « Terminer le tour » et « Compris » se font par de vrais clics ;
// les coups (jouer, déplacer) passent par choose(i) de la page, puis le conseil de l'IA joue à ta place.
// Mesures : coach et puce dans l'écran, rien de coupé, pas de défilement. Ordinateur et téléphone.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_tuto.mjs <dossier des captures> [port]
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
  const tap = async sel => { if (mob) await pg.tap(sel); else await pg.click(sel); };
  const idle = () => waitFor("!!(V && !working && (V.dec || V.ask || (V.winner !== null && V.winner !== undefined)))");
  const k = () => ev("TUTO ? TUTO.k : -1");
  // le coach et la puce tiennent dans l'écran, sans texte coupé
  const fits = async lab => {
    const r = await ev(`(()=>{ const out = []; for (const id of ["coach", "gtuto"]) { const e = $(id); if (e.hidden) continue; const q = e.getBoundingClientRect();
      out.push({ id, in: q.left >= 0 && q.top >= 0 && q.right <= innerWidth + 0.5 && q.bottom <= innerHeight + 0.5, cut: id === "gtuto" ? false : e.scrollHeight > e.clientHeight + 1 }); }
      return { out, scroll: document.documentElement.scrollWidth > innerWidth + 1 } })()`);
    ok(`${w} px, ${lab} : coach et puce dans l'écran, pas de défilement horizontal (${JSON.stringify(r)})`, r.out.length && r.out.every(x => x.in && !x.cut) && !r.scroll);
  };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=learn]');return b&&!b.disabled})()");
  await tap("[data-m=learn]");
  for (let i = 0; i < 4; i++) { await waitFor("!!document.getElementById('tNext')"); if (i === 0) await pg.screenshot({ path: `${OUT}/tuto-${w}-intro.png` }); await tap("#tNext"); }
  await waitFor("!!document.getElementById('bKeep')");
  ok(`${w} px : partie lancée (Master Yi contre Darius, tu commences)`, await ev("PN[0] === 'Master Yi' && PN[1] === 'Darius' && META.first === 0"));
  ok(`${w} px : défi 1 affiché au mulligan`, await ev("!$('coach').hidden && $('coach').innerText.includes('Garde ta main')"));
  await fits("mulligan");
  await pg.screenshot({ path: `${OUT}/tuto-${w}-mulligan.png` });
  await tap("#bKeep"); await idle();
  ok(`${w} px : défi 1 réussi après « Garder ma main »`, (await k()) === 1);
  await tap("#cOk");
  ok(`${w} px : « Compris » replie le coach, la puce reste (« ${await ev("$('gtuto').innerText")} »)`, await ev("$('coach').hidden && !$('gtuto').hidden"));
  await tap("#gtuto");
  ok(`${w} px : la puce rouvre le coach`, await ev("!$('coach').hidden"));
  await tap("#cOk");
  // défi 2 : jouer Pit Rookie
  const play = await ev("(V.dec.options.find(o => o.k === 'play' && /Pit Rookie/.test(o.label)) || {}).i");
  ok(`${w} px : Pit Rookie jouable au tour 1`, play !== undefined);
  await ev(`choose(${play})`); await idle();
  ok(`${w} px : défi 2 réussi (unité jouée)`, (await k()) === 2);
  await pg.screenshot({ path: `${OUT}/tuto-${w}-defi3.png` });
  await fits("défi 3");
  if (await ev("!$('coach').hidden")) await tap("#cOk");
  await tap("#bMain"); await idle();
  ok(`${w} px : défi 3 réussi (tour terminé)`, (await k()) >= 3);
  // défi 4 : aller sur un champ de bataille (la page propose le déplacement vers un champ)
  for (let i = 0; i < 40 && (await k()) === 3; i++) {
    if (await ev("!!document.getElementById('bKeep')")) { await tap("#bKeep"); await idle(); continue; }
    if (await ev("!!V.ask")) { await ev("answer(0)"); await idle(); continue; }
    const mv = await ev("(V.dec.options.find(o => o.k === 'move' && !/base/i.test(o.label)) || {}).i");
    await ev(`choose(${mv !== undefined ? mv : "V.dec.options[0].i"})`); await idle();
  }
  ok(`${w} px : défi 4 réussi (unité sur un champ de bataille)`, (await k()) >= 4);
  await pg.screenshot({ path: `${OUT}/tuto-${w}-defi4.png` });
  await fits("défi 4+");
  // la suite : le conseil de l'IA joue à ta place jusqu'à la fin du tutoriel ou de la partie
  let seenTip = false;
  for (let i = 0; i < 400; i++) {
    if (await ev("!!document.getElementById('tEnd')")) break;
    if (await ev("V.winner !== null && V.winner !== undefined")) { await pg.waitForTimeout(300); break; }
    if (await ev("!$('coach').hidden && !!TUTO.tip")) { seenTip = true; await tap("#cOk"); }
    if (await ev("!!V.ask")) { await ev(`answer(V.ask.kind === "mulligan" ? [] : 0)`); await idle(); continue; }
    const hs = JSON.parse(await ev("T.hint(3)"));
    await ev(`choose(${hs.length ? hs[0].i : "V.dec.options[0].i"})`); await idle();
  }
  await waitFor("!!document.getElementById('tEnd')", 20000).catch(() => {});
  const end = await ev("document.getElementById('tEnd') ? document.getElementById('tEnd').innerText : ''");
  const kk = await k();
  ok(`${w} px : écran de fin du tutoriel (${kk}/6 défis, partie ${await ev("V.winner === null || V.winner === undefined ? 'en cours' : 'finie ' + V.st.pts.join('-')")})`, !!end);
  ok(`${w} px : au moins une astuce de situation montrée`, seenTip);
  await pg.screenshot({ path: `${OUT}/tuto-${w}-fin.png` });
  if (await ev("!!document.getElementById('tAgain')")) {
    await tap("#tAgain"); await waitFor("!!document.getElementById('bKeep')");
    ok(`${w} px : « Recommencer le tuto » repart du défi 1, même main`, (await k()) === 0 && await ev("V.ask.options.slice().sort().join() === 'First Mate,Pit Rookie,Pit Rookie,Punch First'"));
  }
  // une partie normale ensuite : coach et puce disparaissent
  await ev("showMenu()"); await tap("[data-m=new]"); await waitFor("!!document.getElementById('bStart')"); await tap("#bStart");
  await waitFor("!!(V && META && META.names && TUTO === null)", 60000).catch(() => {});
  ok(`${w} px : partie normale sans coach ni puce`, await ev("TUTO === null && $('coach').hidden && $('gtuto').hidden"));
  ok(`${w} px : aucune erreur de page (${errs.join(" | ").slice(0, 200)})`, errs.length === 0);
}
await b.close();
console.log(fails ? `${fails} échec(s)` : "tout est OK");
process.exit(fails ? 1 : 0);
