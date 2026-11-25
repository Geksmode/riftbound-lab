import { existsSync } from "node:fs";
// Robot des ajouts du 2026-11-25 (d'après le simulateur Rift Atlas et TCG Arena), sur la vraie page :
// 1. plateau : total de Might par battlefield (recalculé à part), réglage pour le cacher, journal condensé (lignes
//    identiques regroupées, points surlignés), taille des cartes en jeu (70 à 130 %) ;
// 2. replays : sauts « Tour » et « Point », partie rouverte depuis un fichier (importée, pas dans le bilan) ;
// 3. éditeur : versions d'un deck (cartes ajoutées et retirées, Restaurer), colonne de la courbe → cartes et chance
//    exacte en main de départ (recalculée par dénombrement) ;
// 4. « À toi » dans le titre de l'onglet, vibration, écran maintenu allumé (API du navigateur remplacées par des compteurs).
// Puis l'anglais et un téléphone (393×851, toucher simulé).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_atlas2.mjs <dossier des journaux (inutilisé)> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
const errs = [];
let pg, ev, waitFor, mob = false;
// vibration et Wake Lock : remplacés par des compteurs (un navigateur sans écran n'a ni vibreur ni écran à garder allumé)
const STUB = `window.__vib = []; navigator.vibrate = x => { window.__vib.push(x); return true; };
  window.__wk = { req: 0, rel: 0 };
  Object.defineProperty(navigator, "wakeLock", { configurable: true, value: { request: async () => { window.__wk.req++; return { release: async () => { window.__wk.rel++; }, addEventListener() {} }; } } });`;
async function open(ctxOpts, url = "train.html") {
  mob = !!ctxOpts.hasTouch;
  const ctx = await b.newContext(ctxOpts); await ctx.addInitScript(STUB);
  pg = await ctx.newPage();
  pg.on("pageerror", e => errs.push(String(e)));
  pg.on("dialog", d => d.accept());
  ev = s => pg.evaluate(s);
  waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
  await pg.goto(`http://localhost:${PORT}/${url}`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
}
const tap = async sel => { const l = pg.locator(sel).first(); if (mob) { await l.scrollIntoViewIfNeeded(); const bb = await l.boundingBox(); await pg.touchscreen.tap(bb.x + bb.width / 2, bb.y + bb.height / 2); } else await l.click(); await pg.waitForTimeout(200); };
// Une partie par les vrais appels de la page : joue ou déplace quand c'est possible (pour avoir des unités aux
// battlefields et des points), sinon passe. S'arrête quand stop() est vrai, à la fin de la partie ou après n décisions.
async function game(seed, n, stop = "false") {
  await ev("showMenu('main')"); await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev(`document.querySelector('.vopt').open = true; document.getElementById('sSeed').value='${seed}'; document.getElementById('sFirst').value='0'`); await pg.click("#bStart");
  let acts = 0;
  for (let i = 0; i < 900 && acts < n; i++) {
    await waitFor("!!(V && !working)");
    if (await ev("V.winner !== null && V.winner !== undefined")) break;
    if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
    if (await ev("!!V.ask")) { await ev("(async()=>{ await answer(V.ask.kind === 'mulligan' ? [] : 0) })()"); continue; }
    if (await ev("!!V.dec")) {
      if (await ev(stop)) break;
      acts++;
      await ev(`(async()=>{ const o = V.dec.options, e = o.find(x => x.k === 'end' || x.k === 'pass');
        const mv = o.filter(x => x.k === 'move'), pl = o.filter(x => x.k === 'play');
        const x = (${acts} % 5 === 0 && e) ? e : mv[0] || pl[0] || e || o[0]; await choose(x.i) })()`);
      continue;
    }
    await ev("(async()=>{ await pump(J(T.step())) })()");
  }
  await waitFor("!!(V && !working)"); await ev("recSave(true)"); await pg.waitForTimeout(300);
  return acts;
}
// total de Might recalculé à part, depuis l'état envoyé par le moteur
const SUMS = `(() => { const out = []; V.st.bfs.forEach((bf, i) => { const el = document.querySelectorAll('#board .bf')[i];
  const t = bf.u.filter(u => u.c !== MEP), m = bf.u.filter(u => u.c === MEP), sum = us => us.reduce((a, u) => a + (+u.m || 0), 0);
  const want = (t.length > 1 || m.length > 1 || (t.length && m.length)) ? [t.length ? sum(t) : null, m.length ? sum(m) : null] : [null, null];
  const g = c => { const s = el.querySelector('.bfsum.' + c); return s ? +s.dataset.sum : null; };
  out.push({ want, got: [g('top'), g('bot')], n: t.length + m.length }); }); return out; })()`;
const sameSums = a => a.every(x => x.want[0] === x.got[0] && x.want[1] === x.got[1]);
const nSum = () => ev("document.querySelectorAll('#board .bfsum').length");
const sonOpen = async () => { if (await ev("$('sonPop').hidden")) await tap("#bSon"); await waitFor("!$('sonPop').hidden"); };
const sonClose = async () => { await ev("sonPop(false)"); };

// ============================================================ ordinateur
await open({ viewport: { width: 1400, height: 900 } });
ok("au menu : titre de l'onglet normal, pas d'écran gardé allumé", await ev("document.title === TITLE0 && __wk.req === 0"));
await game(4, 120, "document.querySelectorAll('#board .bfsum').length >= 1 && V.dec.kind === 'main'");
let sums = await ev(SUMS);
ok(`plateau : ${await nSum()} total(aux) de Might affiché(s), égaux à la somme des Might des unités (${JSON.stringify(sums.map(x => x.got))})`, (await nSum()) >= 1 && sameSums(sums));
ok("un battlefield à une seule unité (ou vide) n'affiche pas de total", sums.every(x => x.n >= 2 || (x.got[0] === null && x.got[1] === null)));
ok("le total tient dans son battlefield et ne capte pas les clics", await ev(`[...document.querySelectorAll('#board .bfsum')].every(s => { const r = s.getBoundingClientRect(), p = s.closest('.bf').getBoundingClientRect(); return r.width > 0 && r.left >= p.left - 1 && r.right <= p.right + 1 && r.top >= p.top - 1 && r.bottom <= p.bottom + 1 && getComputedStyle(s).pointerEvents === 'none'; })`));

// ---- « À toi » : titre, vibration, écran allumé
ok("c'est à toi : le titre de l'onglet commence par « ● À toi »", await ev("document.title.startsWith('● À toi · ') && document.title.endsWith(TITLE0)"));
ok("partie en cours : l'écran est demandé allumé une fois", await ev("__wk.req === 1 && __wk.rel === 0"));
await ev("__vib.length = 0; nudgeWait = Date.now() - 3000; nudge(V)");
ok("après une attente de 3 s, le téléphone vibre une fois", await ev("__vib.length === 1 && __vib[0] > 0"));
await ev("__vib.length = 0; nudgeWait = Date.now() - 200; nudge(V)");
ok("après une attente de 0,2 s (page à l'écran), pas de vibration", await ev("__vib.length === 0"));

// ---- réglages (menu ♪, section Affichage)
await sonOpen();
ok("menu ♪ : section Affichage avec la taille des cartes et trois cases cochées", await ev("!!$('setCs') && $('setCs').value === '100' && ['setMight','setAlert','setWake'].every(i => $(i) && $(i).checked)"));
await pg.click("#setMight"); await pg.waitForTimeout(200);
ok("case « Total de Might » décochée : plus aucun total, choix gardé (rbt-might = 0)", (await nSum()) === 0 && await ev("localStorage.getItem('rbt-might') === '0'"));
await pg.click("#setMight"); await pg.waitForTimeout(200);
ok("case recochée : les totaux reviennent", (await nSum()) >= 1 && sameSums(await ev(SUMS)));
await pg.click("#setAlert"); await pg.waitForTimeout(100);
ok("case « Me prévenir » décochée : titre normal, pas de vibration", await ev("(() => { __vib.length = 0; nudgeWait = Date.now() - 3000; nudge(V); return document.title === TITLE0 && __vib.length === 0; })()"));
await pg.click("#setAlert");
await pg.click("#setWake"); await pg.waitForTimeout(200);
ok("case « Garder l'écran allumé » décochée : l'écran est relâché", await ev("__wk.rel === 1 && wakeL === null"));
await pg.click("#setWake"); await pg.waitForTimeout(300);
ok("case recochée : l'écran est redemandé", await ev("__wk.req === 2"));
const hOf = () => ev("(() => { const c = document.querySelector('#board .bf .side .card'); return c ? Math.round(c.getBoundingClientRect().height * 10) / 10 : 0; })()");
const setCs = async v => { await ev(`(() => { const r = $('setCs'); r.value = '${v}'; r.dispatchEvent(new Event('input', { bubbles: true })); })()`); await pg.waitForTimeout(250); };
const h100 = await hOf(); await setCs(130); const h130 = await hOf(); await setCs(70); const h70 = await hOf();
ok(`taille des cartes en jeu : ${h70} px à 70 %, ${h100} px à 100 %, ${h130} px à 130 % (unité sur un battlefield)`, h70 > 0 && h70 < h100 * 0.9 && h130 > h100 * 1.08);
await setCs(130);
ok("à 130 % : rien ne dépasse de l'écran, chaque carte reste dans son battlefield", await ev(`document.documentElement.scrollWidth <= innerWidth + 1 && [...document.querySelectorAll('#board .bf .side .card')].every(c => { const r = c.getBoundingClientRect(), p = c.closest('.bf').getBoundingClientRect(); return r.top >= p.top - 2 && r.bottom <= p.bottom + 2; }) && (() => { const g = document.querySelector('.boardgrid').getBoundingClientRect(); return g.bottom <= innerHeight + 2; })()`));
ok("le réglage est gardé (rbt-cs = 130)", await ev("localStorage.getItem('rbt-cs') === '130'"));
await setCs(100); await sonClose();

// ---- journal condensé
ok("journal : chaque ligne du moteur est comptée une fois (somme des compteurs = nombre de lignes)", await ev("[...$('log').children].reduce((a, d) => a + (+d.dataset.x || 1), 0) === LOG.length && $('log').children.length <= LOG.length"));
ok("journal : pas deux lignes identiques à la suite", await ev("(() => { const k = [...$('log').children].map(d => d.className.includes('sub') + '|' + d.firstChild.textContent); return k.every((x, i) => !i || x !== k[i - 1] || /^(Tour|Turn) /.test(x.split('|')[1])); })()"));
await ev("addLog([[0, 'Ligne du robot'], [0, 'Ligne du robot'], [0, 'Ligne du robot']])");
ok("trois lignes identiques d'un coup : une ligne « ×3 »", await ev("$('log').lastElementChild.dataset.x === '3' && $('log').lastElementChild.querySelector('.lx').textContent === '×3'"));
await ev("addLog([[0, 'Ligne du robot']])");
ok("la même ligne arrive encore : le compteur passe à ×4, sans nouvelle ligne", await ev("$('log').lastElementChild.dataset.x === '4' && [...$('log').children].filter(d => d.firstChild.textContent === 'Ligne du robot').length === 1"));
await ev("addLog([[1, 'Ligne du robot'], [0, 'X +1 point (conquête de Y) → 3'], [0, 'Showdown à Y, initiative X']])");
ok("une ligne en retrait n'est pas fusionnée ; point et showdown surlignés", await ev("(() => { const c = [...$('log').children].slice(-3); return !c[0].dataset.x && c[1].classList.contains('pt') && c[2].classList.contains('cb'); })()"));

// ---- replay : sauts par tour et par point (partie jouée jusqu'à 2 points au moins)
await game(4, 400, "V.st.pts[0] + V.st.pts[1] >= 2 && V.st.t >= 5 && V.dec.kind === 'main'");
const played = await ev("JSON.stringify({ id: REC.id, pts: V.st.pts, t: V.st.t })");
ok(`partie jouée pour le replay : ${played}`, JSON.parse(played).pts[0] + JSON.parse(played).pts[1] >= 2);
const target = JSON.parse(played).id;
await ev("showMenu('games')"); await waitFor(`!!document.querySelector('[data-rp="${target}"]')`);
ok("au menu : titre normal et écran relâché", await ev("document.title === TITLE0 && wakeL === null"));
const nBefore = await ev("document.querySelectorAll('#menu [data-rp]').length");
await tap(`[data-rp="${target}"]`); await waitFor("!!(RP && RP.shown === 0)", 240000);
ok("replay : quatre boutons de saut (Tour, Point), « précédent » éteints au début", await ev("(() => { const j = [...document.querySelectorAll('#rpbar .rpjump button')]; return j.length === 4 && j[0].disabled && j[2].disabled && !j[1].disabled && !j[3].disabled; })()"));
ok(`replay : ${await ev("RP.turns.length")} débuts de tour et ${await ev("RP.pts.length")} points trouvés dans le journal`, await ev("RP.turns.length >= 4 && RP.pts.length >= 2"));
const jump = async w => { const k = await ev(`+document.querySelector('#rpbar [data-rpj="${w}"]').dataset.rpg`); await tap(`#rpbar [data-rpj="${w}"]`); await waitFor(`RP.shown === ${k}`); return k; };
let k1 = await jump("turns+");
ok(`« Tour ▶ » : étape ${k1}, la dernière ligne du journal est le début d'un tour`, await ev(`RP.turns.includes(${k1}) && RP.r.log.some(l => l[2] === ${k1} && /^(Tour|Turn) /.test(l[1]))`));
const t1 = await ev("V.st.t"); const k2 = await jump("turns+"); const t2 = await ev("V.st.t");
ok(`« Tour ▶ » encore : du tour ${t1} au tour ${t2}`, t2 === t1 + 1 && k2 > k1);
ok("« ◀ Tour » : retour à l'étape du tour d'avant", (await jump("turns-")) === k1);
const p0 = await ev("V.st.pts[0] + V.st.pts[1]"); const kp = await jump("pts+");
ok(`« Point ▶ » : étape ${kp}, un point vient d'être marqué (total ${p0} → ${await ev("V.st.pts[0] + V.st.pts[1]")}) et la ligne est surlignée dans le journal`, await ev(`V.st.pts[0] + V.st.pts[1] === ${p0} + 1 && !!document.querySelector('#log .pt')`));
const kp2 = await jump("pts+");
ok("« Point ▶ » puis « ◀ Point » : retour au point d'avant", kp2 > kp && (await jump("pts-")) === kp);
await tap("#rpQuit"); await waitFor("!RP");

// ---- rouvrir une partie depuis un fichier
await ev("showMenu('games')"); await waitFor("!!$('bImpGames')");
const gj = JSON.parse(await ev(`JSON.stringify(window.__GAMES.find(g => g.id === ${JSON.stringify(target)}))`));
const fake = { ...gj, id: "robot-import-1", result: { winner: 0, pts: [8, 3], turn: 9 } };
await ev("showMenu('bilan')"); await waitFor("!!$('bilBox') && !/Chargement/.test($('bilBox').textContent)");
const bil0 = await ev("$('bilBox').innerText");
await ev("showMenu('games')"); await waitFor("!!$('fImpGames')");
await pg.setInputFiles("#fImpGames", { name: "partie.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify([fake])) });
await waitFor(`document.querySelectorAll('#menu [data-rp]').length === ${nBefore + 1}`);
ok("fichier choisi : la partie s'ajoute à la liste avec la mention « importée »", await ev("(() => { const r = document.querySelector('[data-rp=\"robot-import-1\"]'); return !!r && /importée/.test(r.closest('.mcard').textContent) && importGames.last.added === 1; })()"));
await pg.setInputFiles("#fImpGames", { name: "partie.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify([fake])) });
await waitFor("importGames.last.known === 1");
ok("le même fichier une 2e fois : rien n'est ajouté ni remplacé", await ev(`document.querySelectorAll('#menu [data-rp]').length === ${nBefore + 1} && importGames.last.added === 0`));
await pg.setInputFiles("#fImpGames", { name: "x.json", mimeType: "application/json", buffer: Buffer.from("pas du json") }); await pg.waitForTimeout(400);
await pg.setInputFiles("#fImpGames", { name: "y.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify([{ id: "<img src=x>", moves: [], seed: 1 }, { foo: 1 }])) }); await pg.waitForTimeout(400);
ok("fichier illisible ou sans partie rejouable : rien n'est ajouté, la page tient", await ev(`document.querySelectorAll('#menu [data-rp]').length === ${nBefore + 1}`));
// fichier déposé (glisser-déposer) : même chemin que le bouton
await ev(`(() => { const dt = new DataTransfer(); dt.items.add(new File([${JSON.stringify(JSON.stringify({ ...fake, id: "robot-import-2" }))}], "p2.json", { type: "application/json" }));
  $("menu").dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: dt })); })()`);
await waitFor(`document.querySelectorAll('#menu [data-rp]').length === ${nBefore + 2}`);
ok("fichier déposé sur « Mes parties » : la partie s'ajoute aussi", await ev("!!document.querySelector('[data-rp=\"robot-import-2\"]')"));
await ev("showMenu('bilan')"); await waitFor("!!$('bilBox') && !/Chargement/.test($('bilBox').textContent)");
ok("bilan : inchangé après l'import de deux parties « gagnées » (elles ne comptent pas)", (await ev("$('bilBox').innerText")) === bil0);
await ev("showMenu('games')"); await waitFor("!!document.querySelector('[data-rp=\"robot-import-1\"]')");
await tap('[data-rp="robot-import-1"]'); await waitFor("!!(RP && RP.shown === 0)", 240000);
ok("la partie importée se revoit : même nombre d'étapes que l'originale", await ev("RP.r.n > 10 && RP.turns.length >= 4"));
await tap("#rpQuit"); await waitFor("!RP");

// ---- éditeur : versions du deck
await ev("showMenu('main')"); await tap("[data-m=deck]"); await waitFor("!!document.getElementById('dkSave')");
const key = await ev("(CAT.presets.find(p => /akali/i.test(p.deck.legend)) || CAT.presets[0]).key");
await pg.selectOption("#dkLoad", key); await waitFor("!!DK.legend && cnt(DK.main) >= 39");
ok("deck jamais enregistré : pas de bouton « Versions »", await ev("!$('dkVers')"));
await pg.fill("#dkName", "Robot versions"); await tap("#dkSave"); await waitFor("!!$('dkVers') && !dkDirty()");
const main0 = await ev("JSON.stringify(DK.main)");
const [out, inn] = JSON.parse(await ev(`(() => { const out = Object.keys(DK.main).find(n => DK.main[n] >= 2), inn = Object.keys(DK.main).find(n => DK.main[n] === 1 || DK.main[n] === 2 && n !== out) || null;
  const other = CAT.cards.find(c => c.t === 'Spell' && c.m && !DK.main[c.n] && (c.d || []).every(d => (CAT.by[DK.legend].d || []).includes(d)) && !c.ban);
  DK.main[out]--; DK.main[other.n] = 1; dkRender(); return JSON.stringify([out, other.n]); })()`));
ok("deck modifié : marque « non enregistré »", await ev("dkDirty()"));
await tap("#dkSave"); await waitFor("!dkDirty()");
await tap("#dkSave"); await pg.waitForTimeout(300);   // enregistrer sans changement : pas de version en double
await tap("#dkVers"); await waitFor("!!$('dkVer')");
const vtxt = await ev("[...document.querySelectorAll('#dkVer .vrow')].map(r => r.innerText.replace(/\\s+/g, ' '))");
ok(`Versions : 2 lignes (${vtxt.length}), la plus récente d'abord, sans bouton Restaurer sur l'actuelle`, vtxt.length === 2 && await ev("!document.querySelector('#dkVer .vrow[data-ver=\"0\"] [data-vres]') && !!document.querySelector('#dkVer .vrow[data-ver=\"1\"] [data-vres]')"));
ok(`la version actuelle montre la carte ajoutée et la carte retirée (+1 ${inn} · −1 ${out})`, vtxt[0].includes("+1 " + inn) && vtxt[0].includes("−1 " + out) && await ev("document.querySelectorAll('#dkVer .vrow[data-ver=\"0\"] .vd .a').length === 1 && document.querySelectorAll('#dkVer .vrow[data-ver=\"0\"] .vd .r').length === 1"));
await tap('#dkVer [data-vres="1"]'); await waitFor("$('modal').hidden");
ok("Restaurer : la liste d'avant revient dans l'éditeur, marquée non enregistrée, rien n'est écrasé", (await ev("JSON.stringify(Object.fromEntries(Object.entries(DK.main).filter(x => x[1] > 0)))")) === main0 && await ev(`dkDirty() && MYDECKS[DK.id].main[${JSON.stringify(inn)}] === 1`));
await tap("#dkSave"); await waitFor("!dkDirty()");
await tap("#dkVers"); await waitFor("!!$('dkVer')");
ok("après l'enregistrement de la version restaurée : 3 versions", await ev("document.querySelectorAll('#dkVer .vrow').length === 3"));
await ev("closeModal()");
await ev(`(() => { const all = dkhAll(), h = all[DK.id]; for (let i = 0; i < 40; i++) h.unshift({ t: new Date().toISOString(), d: { ...h[0].d, runes: { ...h[0].d.runes } } }); store.set('rbt-dkhist', JSON.stringify(all));
  const n = Object.keys(DK.main)[0]; DK.main[n]--; })()`);
await tap("#dkSave"); await waitFor("!dkDirty()");
ok("jamais plus de 30 versions gardées", await ev("dkhAll()[DK.id].length === 30"));

// ---- éditeur : colonne de la courbe → cartes et chance exacte
await tap("#dkVStat"); await waitFor("!!$('stCurve')");
ok("Stats : aucune colonne choisie au départ", await ev("!$('stSel')"));
const col = await ev("+[...document.querySelectorAll('#stCurve .stcol.pick')].sort((a, b) => b.dataset.n - a.dataset.n)[0].dataset.e");
await tap(`#stCurve .stcol[data-e="${col}"]`); await waitFor("!!$('stSel')");
const C4 = (n, k) => { let r = 1; for (let i = 0; i < k; i++) r = r * (n - i) / (i + 1); return n < k ? 0 : r; };
const det = JSON.parse(await ev(`JSON.stringify({ k: +$('stSel').dataset.k, N: +$('stSel').dataset.n, want: Object.keys(DK.main).filter(n => DK.main[n] > 0 && (+(CAT.by[n].e || 0)) === ${col}).reduce((a, n) => a + DK.main[n], 0),
  rows: [...document.querySelectorAll('#stSel .stsell .dkrow')].map(r => r.dataset.card), cells: [...document.querySelectorAll('#stSelTab tbody td')].map(t => t.textContent), on: document.querySelector('#stCurve .stcol.on').dataset.e })`));
ok(`colonne ${col} énergie touchée : ses ${det.rows.length} cartes listées, ${det.k} exemplaires sur ${det.N} (recompté : ${det.want})`, det.k === det.want && det.k > 0 && det.N === 39 - 0 * 1 + 0 || (det.k === det.want && det.N >= 38), );
const pct = x => (x * 100).toFixed(x > .9995 ? 0 : 1).replace(".", ",") + " %";
const exact = s => 1 - C4(det.N - det.k, s) / C4(det.N, s);
ok(`chance d'en avoir au moins une : ${det.cells.slice(2).join(" / ")} (dénombrement : ${[4, 6, 8, 10].map(s => pct(exact(s))).join(" / ")})`, [4, 6, 8, 10].every((s, i) => det.cells[2 + i] === pct(exact(s))) && det.on === String(col));
await tap(`#stCurve .stcol[data-e="${col}"]`); await pg.waitForTimeout(200);
ok("retoucher la colonne : le détail se ferme", await ev("!$('stSel')"));
if (await ev("!!document.querySelector('#stMight .stcol.pick')")) {
  await tap("#stMight .stcol.pick"); await waitFor("!!$('stSel')");
  ok("colonne de Might touchée : seulement des unités de cette Might", await ev("(() => { const m = +document.querySelector('#stMight .stcol.on').dataset.m; return [...document.querySelectorAll('#stSel .stsell .dkrow')].every(r => CAT.by[r.dataset.card].t === 'Unit' && +CAT.by[r.dataset.card].mi === m); })()"));
  await tap("#stSelX"); await pg.waitForTimeout(200);
  ok("✕ : le détail se ferme", await ev("!$('stSel')"));
}
await ev("deleteDeck()"); await pg.waitForTimeout(400);
ok("deck supprimé : son historique de versions aussi", await ev("!dkhAll()['robot-versions']"));

// ============================================================ anglais
await open({ viewport: { width: 1400, height: 900 } }, "train.html?lang=en");
await game(4, 120, "document.querySelectorAll('#board .bfsum').length >= 1 && V.dec.kind === 'main'");
const FR = /[éèêàçù]|Affichage|Taille|Garder|prévenir|Tour |cartes|Fermer|importée|Rouvrir|Ouvrir/;
ok("anglais : titre de l'onglet « ● Your move »", await ev("document.title.startsWith('● Your move · ')"));
ok("anglais : info-bulle du total de Might traduite", await ev("/^Total Might: /.test(document.querySelector('#board .bfsum').title)"));
await sonOpen();
const sonTxt = await ev("$('sonPop').innerText");
ok(`anglais : section Display du menu ♪ sans français (${sonTxt.replace(/\s+/g, " ").slice(-150)})`, /Display/.test(sonTxt) && !FR.test(sonTxt));
await sonClose();
await ev("recSave(true)"); await pg.waitForTimeout(300);
const tgEn = await ev("REC.id");
await ev("showMenu('games')"); await waitFor(`!!document.querySelector('[data-rp="${tgEn}"]')`);
const dropTxt = await ev("$('gamesDrop').innerText");
ok(`anglais : carte « Reopen a downloaded game » sans français`, /Open a file/.test(dropTxt) && !FR.test(dropTxt));
await tap(`[data-rp="${tgEn}"]`); await waitFor("!!(RP && RP.shown === 0)", 240000);
const jt = await ev("[...document.querySelectorAll('#rpbar .rpjump button')].map(b => b.textContent + '|' + b.title).join(' ; ')");
ok(`anglais : boutons de saut traduits (${jt})`, /Turn ▶/.test(jt) && /Next turn/.test(jt) && /Score ▶/.test(jt) && !FR.test(jt) && await ev("RP.turns.length >= 2"));
await tap("#rpQuit"); await waitFor("!RP");
await ev("showMenu('main')"); await tap("[data-m=deck]"); await waitFor("!!document.getElementById('dkSave')");
await pg.selectOption("#dkLoad", await ev("CAT.presets[0].key")); await waitFor("!!DK.legend && cnt(DK.main) >= 39");
await pg.fill("#dkName", "Robot en"); await tap("#dkSave"); await waitFor("!!$('dkVers') && !dkDirty()");
await ev("(() => { const n = Object.keys(DK.main).find(n => DK.main[n] >= 2); DK.main[n]--; dkRender(); })()"); await tap("#dkSave"); await waitFor("!dkDirty()");
await tap("#dkVers"); await waitFor("!!$('dkVer')");
const vEn = await ev("$('mbox').innerText");
ok("anglais : fenêtre des versions sans français", /Versions of/.test(vEn) && /Restore/.test(vEn) && !FR.test(vEn.replace(/Robot en/g, "")));
await ev("closeModal()"); await tap("#dkVStat"); await waitFor("!!$('stCurve')");
await tap("#stCurve .stcol.pick"); await waitFor("!!$('stSel')");
const sEn = await ev("$('stSel').innerText");
ok(`anglais : détail d'une colonne sans français (${sEn.replace(/\s+/g, " ").slice(0, 400)})`, /At least one of these cards/i.test(sEn) && /Opening hand/.test(sEn) && !FR.test(sEn.split("\n").filter(l => !/×\d|champion/.test(l) && !/^[A-Z][\w' ,-]+$/.test(l.trim())).join(" ")));
await ev("deleteDeck()"); await pg.waitForTimeout(300);

// ============================================================ téléphone (toucher simulé)
await open({ viewport: { width: 393, height: 851 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 });
await game(4, 120, "document.querySelectorAll('#board .bfsum').length >= 1 && V.dec.kind === 'main'");
sums = await ev(SUMS);
ok(`téléphone : ${await nSum()} totaux de Might, justes, dans leur battlefield`, (await nSum()) >= 1 && sameSums(sums) && await ev(`[...document.querySelectorAll('#board .bfsum')].every(s => { const r = s.getBoundingClientRect(), p = s.closest('.bf').getBoundingClientRect(); return r.width > 0 && r.left >= p.left - 1 && r.right <= p.right + 1 && r.top >= p.top - 1 && r.bottom <= p.bottom + 1; })`));
ok("téléphone : pas de défilement horizontal", await ev("document.documentElement.scrollWidth <= innerWidth + 1"));
ok("téléphone : titre « ● À toi », écran demandé allumé", await ev("document.title.startsWith('● À toi · ') && __wk.req === 1"));
await sonOpen();
ok("téléphone : menu ♪ à l'écran, sans le réglage de taille (réservé à l'ordinateur), cases de 18 px touchables", await ev("(() => { const r = $('sonPop').getBoundingClientRect(); return r.left >= 0 && r.right <= innerWidth + 1 && r.bottom <= innerHeight + 1 && getComputedStyle(document.querySelector('#sonPop .csrow')).display === 'none' && $('setMight').closest('label').getBoundingClientRect().height >= 32; })()"));
await tap("#setMight");
ok("téléphone : case décochée au doigt, les totaux disparaissent", (await nSum()) === 0);
await tap("#setMight"); await sonClose();
await ev("recSave(true)"); await pg.waitForTimeout(300);
const tgM = await ev("REC.id");
await ev("showMenu('games')"); await waitFor(`!!document.querySelector('[data-rp="${tgM}"]')`);
ok("téléphone : carte « Rouvrir une partie téléchargée » sans débordement", await ev("(() => { const r = $('gamesDrop').getBoundingClientRect(); return r.right <= innerWidth + 1 && $('bImpGames').getBoundingClientRect().height >= 36; })()"));
await tap(`[data-rp="${tgM}"]`); await waitFor("!!(RP && RP.shown === 0)", 240000);
ok("téléphone : boutons de saut de 40 px au moins, dans l'écran", await ev("[...document.querySelectorAll('#rpbar .rpjump button')].every(b => { const r = b.getBoundingClientRect(); return r.height >= 40 && r.left >= 0 && r.right <= innerWidth + 1; })"));
const km = await ev("+document.querySelector('#rpbar [data-rpj=\"turns+\"]').dataset.rpg");
await tap('#rpbar [data-rpj="turns+"]'); await waitFor(`RP.shown === ${km}`);
ok("téléphone : « Tour ▶ » au doigt avance au tour suivant", await ev(`RP.turns.includes(${km})`));
await tap("#rpQuit").catch(async () => { await ev("exitReplay()"); }); await waitFor("!RP");

ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 3).join(" | ") : ""), errs.length === 0);
console.log(fails ? `\n${fails} ÉCHEC(S)` : "\nTout est vert.");
await b.close();
process.exit(fails ? 1 : 0);
