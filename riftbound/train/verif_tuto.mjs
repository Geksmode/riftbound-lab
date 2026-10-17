import { existsSync } from "node:fs";
// Robot du tutoriel (menu « Apprendre à jouer ») dans la vraie table : sept écrans d'intro (lexique ouvert et refermé,
// écran des cartes), leçons et défis dans l'ordre (zone surlignée à chaque leçon, mot du coach et sa définition, lexique
// depuis le coach, agrandi d'une carte avec ses mots-clés), tour de Darius puis leçons du tour 2, astuces, fin du tutoriel.
// Menu, intro, « Suivant », « Garder ma main », « Terminer le tour », « Compris » et les mots se font par de vrais clics ;
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
  const nIntro = await ev("TINTRO.length");
  for (let i = 0; i < nIntro; i++) {
    await waitFor("!!document.getElementById('tNext')");
    if (i === 0) await pg.screenshot({ path: `${OUT}/tuto-${w}-intro.png` });
    if (i === 2) { ok(`${w} px : écran des trois sortes de cartes avec leurs images`, await ev("document.querySelectorAll('#tIntro .tcards .card').length === 3"));
      await pg.screenshot({ path: `${OUT}/tuto-${w}-intro-cartes.png` }); }
    if (i === 0) {   // le lexique depuis l'intro, puis retour au même écran
      await tap("#tGlos"); await waitFor("!!document.querySelector('#mbox .glist')");
      ok(`${w} px : lexique ouvert depuis l'intro (${await ev("document.querySelectorAll('#mbox .gl').length")} mots)`, await ev("document.querySelectorAll('#mbox .gl').length >= 40"));
      await pg.screenshot({ path: `${OUT}/tuto-${w}-lexique.png` });
      await tap("#mClose"); await waitFor("!!document.getElementById('tNext')");
      ok(`${w} px : fermer le lexique ramène à l'intro`, await ev("!!document.getElementById('tIntro')"));
    }
    await tap("#tNext");
  }
  await waitFor("!!document.getElementById('bKeep')");
  ok(`${w} px : partie lancée (Master Yi contre Darius, tu commences)`, await ev("PN[0] === 'Master Yi' && PN[1] === 'Darius' && META.first === 0"));
  const cur = () => ev("TUTO && TUTO.k < TSTEPS.length ? TSTEPS[TUTO.k].t : ''");
  const defis = () => ev("TUTO ? tDone(TUTO.k) : -1");
  const focus = () => ev("document.querySelectorAll('.tfocus').length");
  ok(`${w} px : leçon 1 affichée au mulligan, avec la table surlignée`, await ev("!$('coach').hidden && !!$('cNext')") && (await cur()) === "Bienvenue à la table" && (await focus()) > 0);
  await fits("leçon 1");
  await pg.screenshot({ path: `${OUT}/tuto-${w}-lecon1.png` });
  await tap("#cNext");
  ok(`${w} px : « Suivant » passe à la leçon « Lire une carte », la main du mulligan surlignée`, (await cur()) === "Lire une carte" && await ev("!!document.querySelector('#fbar .mull.tfocus, #fbar .tfocus')"));
  // un mot du coach : sa définition s'affiche, puis se replie
  await tap(".coach .kw");
  ok(`${w} px : toucher un mot du coach montre sa définition`, await ev("!!document.querySelector('#coach .kwdef') && $('coach').querySelector('.kw').getAttribute('aria-expanded') === 'true'"));
  await fits("définition");
  await pg.screenshot({ path: `${OUT}/tuto-${w}-mot.png` });
  await tap(".coach .kw");
  ok(`${w} px : le toucher à nouveau la replie`, await ev("!document.querySelector('#coach .kwdef')"));
  await tap("#cNext");
  ok(`${w} px : défi 1 « Garde ta main », bouton surligné`, (await cur()) === "Garde ta main" && await ev("$('bKeep').classList.contains('tfocus')"));
  await tap("#bKeep"); await idle();
  ok(`${w} px : défi 1 réussi après « Garder ma main », leçon « Ton camp »`, (await defis()) === 1 && (await cur()) === "Ton camp");
  // leçons « Ton camp », « Ta légende… », « Les runes… » : chacune surligne sa zone
  for (const [t, sel] of [["Ton camp", ".half.p0 .zrow"], ["Ta légende et ton champion", ".half.p0 .zone.legend"], ["Les runes, ton énergie", ".half.p0 .zrunes"]]) {
    ok(`${w} px : leçon « ${t} », zone surlignée`, (await cur()) === t && await ev(`!!document.querySelector('#board ${sel}.tfocus')`));
    await tap("#cNext");
  }
  await fits("défi 2");
  await tap("#cOk");
  ok(`${w} px : « Compris » replie le coach, la puce reste (« ${await ev("$('gtuto').innerText")} »)`, await ev("$('coach').hidden && !$('gtuto').hidden && !document.querySelector('.tfocus')"));
  await tap("#gtuto");
  ok(`${w} px : la puce rouvre le coach`, await ev("!$('coach').hidden"));
  // le lexique depuis le coach
  await tap("#cGlos"); await waitFor("!!document.querySelector('#mbox .glist')"); await tap("#mClose");
  ok(`${w} px : lexique ouvert depuis le coach puis fermé`, await ev("$('modal').hidden"));
  // agrandi d'une carte : ses mots-clés expliqués (Punch First : Action)
  await ev("showZoom(document.querySelector('#board .half.p0 .hand [data-card=\"Punch First\"]'))");
  ok(`${w} px : l'agrandi de Punch First explique Action`, await ev("!$('zoom').hidden && [...$('zoom').querySelectorAll('.zkw .gl b')].some(b => b.textContent === 'Action')"));
  await pg.screenshot({ path: `${OUT}/tuto-${w}-agrandi.png` });
  await ev("$('zoom').hidden = true");
  await tap("#cOk");
  // défi 2 : jouer Pit Rookie
  const play = await ev("(V.dec.options.find(o => o.k === 'play' && /Pit Rookie/.test(o.label)) || {}).i");
  ok(`${w} px : Pit Rookie jouable au tour 1`, play !== undefined);
  await ev(`choose(${play})`); await idle();
  ok(`${w} px : défi 2 réussi (unité jouée), leçon « Une unité arrive épuisée », base surlignée`, (await defis()) === 2 && (await cur()) === "Une unité arrive épuisée" && await ev("!!document.querySelector('#board .half.p0 .zbase.tfocus')"));
  await tap("#cNext");
  ok(`${w} px : défi 3, « Terminer le tour » surligné`, (await cur()) === "Termine ton tour" && await ev("$('bMain').classList.contains('tfocus')"));
  await pg.screenshot({ path: `${OUT}/tuto-${w}-defi3.png` });
  await fits("défi 3");
  await tap("#bMain"); await idle();
  ok(`${w} px : défi 3 réussi (tour terminé)`, (await defis()) === 3);
  // tour de Darius puis leçons « Un nouveau tour » et « Les champs de bataille »
  for (let i = 0; i < 60 && !(await ev("!!$('cNext') && TSTEPS[TUTO.k].t === 'Un nouveau tour'")); i++) {
    await idle();
    if (await ev("!$('coach').hidden && !!TUTO.tip")) { await tap("#cOk"); continue; }
    if (await ev("!!V.ask")) { await ev("answer(0)"); continue; }
    if (await ev("V.dec.kind !== 'main'")) { await ev("choose(V.dec.options[0].i)"); continue; }
    if (await ev("$('coach').hidden")) await tap("#gtuto");
  }
  ok(`${w} px : leçon « Un nouveau tour » à ton tour 2`, (await cur()) === "Un nouveau tour" && await ev("!!$('cNext') && V.st.tp === 0"));
  await tap("#cNext");
  ok(`${w} px : leçon « Les champs de bataille », champs surlignés`, (await cur()) === "Les champs de bataille" && await ev("document.querySelectorAll('#board .bfrow .bf.tfocus').length === 2"));
  await tap("#cNext");
  ok(`${w} px : défi 4 affiché sans astuce par-dessus`, (await cur()) === "Va sur un champ de bataille" && await ev("!TUTO.tip"));
  // défi 4 : aller sur un champ de bataille (la page propose le déplacement vers un champ)
  for (let i = 0; i < 40 && (await defis()) === 3; i++) {
    if (await ev("!!$('cNext')")) { await tap("#cNext"); continue; }
    if (await ev("!!V.ask")) { await ev("answer(0)"); await idle(); continue; }
    const mv = await ev("(V.dec.options.find(o => o.k === 'move' && !/base/i.test(o.label)) || {}).i");
    await ev(`choose(${mv !== undefined ? mv : "V.dec.options[0].i"})`); await idle();
  }
  ok(`${w} px : défi 4 réussi (unité sur un champ de bataille)`, (await defis()) >= 4);
  await pg.screenshot({ path: `${OUT}/tuto-${w}-defi4.png` });
  await fits("défi 4+");
  // la suite : le conseil de l'IA joue à ta place jusqu'à la fin du tutoriel ou de la partie
  let seenTip = false; const tips = new Set(), lessons = new Set();
  for (let i = 0; i < 400; i++) {
    if (await ev("!!document.getElementById('tEnd')")) break;
    if (await ev("V.winner !== null && V.winner !== undefined")) { await pg.waitForTimeout(300); break; }
    if (await ev("!$('coach').hidden && !!TUTO.tip")) { seenTip = true; tips.add(await ev("TUTO.tip.id")); await tap("#cOk"); continue; }
    if (await ev("!$('coach').hidden && !!$('cNext')")) { lessons.add(await cur()); await tap("#cNext"); continue; }
    if (await ev("!!V.ask")) { await ev(`answer(V.ask.kind === "mulligan" ? [] : 0)`); await idle(); continue; }
    const hs = JSON.parse(await ev("T.hint(3)"));
    await ev(`choose(${hs.length ? hs[0].i : "V.dec.options[0].i"})`); await idle();
  }
  await waitFor("!!document.getElementById('tEnd')", 20000).catch(() => {});
  const end = await ev("document.getElementById('tEnd') ? document.getElementById('tEnd').innerText : ''");
  const kk = await defis();
  ok(`${w} px : écran de fin du tutoriel (${kk}/6 défis, partie ${await ev("V.winner === null || V.winner === undefined ? 'en cours' : 'finie ' + V.st.pts.join('-')")})`, !!end);
  ok(`${w} px : au moins une astuce de situation montrée (${[...tips].join(", ")})`, seenTip);
  ok(`${w} px : leçons lues en route (${[...lessons].join(", ")})`, lessons.has("Tenir un champ") || kk < 6);
  await pg.screenshot({ path: `${OUT}/tuto-${w}-fin.png` });
  if (await ev("!!document.getElementById('tAgain')")) {
    await tap("#tAgain"); await waitFor("!!document.getElementById('bKeep')");
    ok(`${w} px : « Recommencer le tuto » repart de l'étape 1, même main`, (await ev("TUTO.k")) === 0 && await ev("V.ask.options.slice().sort().join() === 'First Mate,Pit Rookie,Pit Rookie,Punch First'"));
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
