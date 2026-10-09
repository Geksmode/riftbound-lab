import { existsSync } from "node:fs";
// Robot du mode replay (« Mes parties » → « Revoir ») dans la vraie table : une partie jouée par vrais coups (enregistrée
// dans le navigateur), une autre en cours ; on revoit la première : même plateau, barre de lecture (étapes, tes
// décisions ⇤ ⇥, curseur, lecture auto), « Analyser ce coup » donne le conseil de l'IA ; en quittant, la partie en cours
// revient telle quelle. Ordinateur (1400x900, clics) et téléphone (393x851, toucher simulé). Demande du 2026-10-09.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_replay.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
for (const [w, h, mob] of [[1400, 900, false], [393, 851, true]]) {
  const pg = await (await b.newContext({ viewport: { width: w, height: h }, isMobile: mob, hasTouch: mob })).newPage();
  const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 240000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
  const tap = async sel => { const l = pg.locator(sel).first(); if (mob) { await l.scrollIntoViewIfNeeded(); const bb = await l.boundingBox(); await pg.touchscreen.tap(bb.x + bb.width / 2, bb.y + bb.height / 2); } else await l.click(); await pg.waitForTimeout(200); };
  const game = async (seed, n) => {   // une partie par vrais appels de la page (choose / answer enregistrent les coups)
    await pg.click("#bNew").catch(async () => { await ev("showMenu('main')"); await pg.click("[data-m=new]"); }); await waitFor("!!document.getElementById('bStart')");
    await ev(`document.querySelector('.vopt').open = true; document.getElementById('sSeed').value='${seed}'; document.getElementById('sFirst').value='0'`); await pg.click("#bStart");
    let acts = 0;
    for (let i = 0; i < 400 && acts < n; i++) {
      await waitFor("!!(V && !working)");
      if (await ev("V.winner !== null && V.winner !== undefined")) break;
      if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
      if (await ev("!!V.ask")) { await ev("(async()=>{ await answer(V.ask.kind === 'mulligan' ? [] : 0) })()"); continue; }
      if (await ev("!!V.dec")) { acts++; await ev(`(async()=>{ const o = V.dec.options; const e = o.filter(x => x.k === 'end' || x.k === 'pass'); const x = (${i} % 3 && e.length) ? e[0] : o[${i} % o.length]; await choose(x.i) })()`); continue; }
      await ev("(async()=>{ await pump(J(T.step())) })()");
    }
    await waitFor("!!(V && !working)"); await ev("recSave(true)"); await pg.waitForTimeout(300);
  };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')"); await ev("closeModal()");
  await game(21, 18);
  const pts1 = await ev("V.st.pts.join('-') + ' t' + V.st.t"), target = await ev("REC.id");
  await game(22, 6);
  const live = await ev("JSON.stringify([V.st.t, V.st.pts, V.st.p[0].hand])");
  await ev("showMenu('games')"); await waitFor("!!document.querySelector('[data-rp]')");
  const ids = await ev("[...document.querySelectorAll('[data-rp]')].map(b => b.dataset.rp)");
  ok(`${w} px : « Mes parties » propose « Revoir » (${ids.length} parties)`, ids.length >= 2);
  await tap(`[data-rp="${target}"]`);
  await waitFor("!!(RP && document.getElementById('rpbar'))");
  const st = await ev("({ n: RP.r.n, mine: RP.r.mine.length, warn: RP.warn, bar: !!$('rpbar'), board: !!document.querySelector('#board .bfrow'), menu: $('menu').hidden })");
  ok(`${w} px : le replay s'ouvre sur le plateau (${st.n} étapes, ${st.mine} décisions à toi, sans avertissement : ${JSON.stringify(st.warn)})`, st.n > 10 && st.mine > 3 && st.bar && st.board && st.menu && !st.warn.length);
  await tap('#rpbar [data-rpg="' + (await ev("RP.r.n - 1")) + '"]');
  const endpts = await ev("V.st.pts.join('-') + ' t' + V.st.t");
  ok(`${w} px : ⏭ va à la fin, même score et même tour que la partie jouée (${endpts} / ${pts1})`, endpts === pts1);
  await tap('#rpbar [data-rpg="0"]');
  await tap('#rpbar button[title="Ta décision suivante"]');
  const k1 = await ev("RP.k"); const isMine = await ev("RP.r.mine.some(x => x[0] === RP.k)");
  ok(`${w} px : ⇥ va à ta première décision (étape ${k1})`, isMine && k1 > 0);
  // une décision « Tu joues » avec plusieurs coups possibles : analyse
  const kA = await ev("(()=>{ for (const [j, lab] of RP.r.mine) { if (!lab.startsWith('Tu joues')) continue; const f = J(T.replay_frame(j)); if (((f.v.dec || {}).options || []).length >= 3) return j; } return -1 })()");
  await ev(`rpGo(${kA})`);
  const lab = await ev("$('rpbar').querySelector('.rpmine') ? $('rpbar').querySelector('.rpq').textContent : ''");
  await tap("#rpAn"); await waitFor("!!(RP && RP.hint[RP.k])", 120000); await pg.waitForTimeout(200);
  const rows = await ev("[...document.querySelectorAll('#rpbar .rpmine .hintrow')].map(r => r.textContent)");
  ok(`${w} px : « Analyser ce coup » donne le conseil de l'IA (${rows.length} coups ; « ${lab.slice(0, 60)} »)`, kA >= 0 && rows.length >= 2 && /meilleur/.test(rows[0]));
  const inBar = await ev(`(()=>{ const r = $('rpbar').getBoundingClientRect(), c = $('ctrl').getBoundingClientRect(), q = $('rpQuit').getBoundingClientRect();
    const over = ${mob} && r.bottom > c.top + 1;            // téléphone : jamais sur la barre de commande
    return { ok: r.width > 0 && r.left >= 0 && r.right <= innerWidth + 1 && r.top >= 0 && r.bottom <= innerHeight + 1 && !over && document.documentElement.scrollWidth <= innerWidth + 1,
      r: [r.top, r.bottom].map(Math.round), c: Math.round(c.top), quit: q.height > 0 } })()`);
  ok(`${w} px : la barre de lecture tient à l'écran (haut et bas ${JSON.stringify(inBar.r)}, commande à ${inBar.c}), sans couvrir la commande`, inBar.ok);
  await pg.screenshot({ path: `${OUT}/replay-${w}.png` });
  if (mob) {   // réduire le panneau : il ne garde que la navigation et laisse voir la main
    await tap("#rpFold");
    const fo = await ev("(()=>{ const r = $('rpbar').getBoundingClientRect(), hd = document.querySelector('#board .half.p0 .hand').getBoundingClientRect(); return { h: Math.round(r.height), handVisible: r.top >= hd.top - 1 || r.top > hd.bottom - 20, mine: getComputedStyle(document.querySelector('#rpbar .rpmine') || document.body).display } })()");
    ok(`${w} px : « Réduire » laisse la navigation (${fo.h} px de haut) et masque le détail`, fo.h < 170 && fo.mine === "none");
    await pg.screenshot({ path: `${OUT}/replay-reduit-${w}.png` });
    await tap("#rpFold");
  }
  await tap("#bMain"); await pg.waitForTimeout(2500);
  const k2 = await ev("RP.k"); await tap("#bMain");
  ok(`${w} px : ▶ Lecture avance toute seule, ⏸ l'arrête (${kA} → ${k2})`, k2 > kA && await ev("!RP.timer"));
  await tap("#rpQuit");
  const back = await ev("JSON.stringify([V.st.t, V.st.pts, V.st.p[0].hand])");
  ok(`${w} px : « Quitter le replay » rend la partie en cours telle quelle`, back === live && await ev("!RP && !$('rpbar')"));
  ok(`${w} px : aucune erreur JS (${errs.join(" | ")})`, !errs.length);
}
await b.close();
console.log(fails ? `${fails} ÉCHEC(S)` : "tout est OK");
process.exit(fails ? 1 : 0);
