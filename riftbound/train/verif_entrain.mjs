import { existsSync } from "node:fs";
// Robot des trois outils d'entraînement (demande de l'utilisateur, 2026-11-24) :
// 1. menu « Bilan » : parties finies contre l'IA regroupées par légende adverse, niveau de l'IA et premier joueur,
//    comptes justes, intervalle de Wilson à 95 % recalculé ici, « on ne sait pas » tant que l'intervalle contient 50 % ;
// 2. éditeur → « Tester la main » : 4 cartes (règle 116), mulligan de 2 au plus (117), 2 runes par tour et 3 au premier
//    tour du second joueur (485.7), 1 carte par tour, probabilités exactes recalculées ici par dénombrement ;
// 3. menu « Exercices » : une partie jouée par vrais coups est analysée, les décisions à 3 ou plus du conseil deviennent
//    des exercices ; l'exercice se joue sur le plateau (le coup est noté, pas envoyé au moteur : la partie en cours et
//    l'enregistrement ne bougent pas), verdict, « Réessayer », « Voir la suite », retour au menu.
// Joué deux fois : ordinateur 1400×900 (souris) et téléphone 393×851 (toucher simulé).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_entrain.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {});
let fails = 0;
const ok = (n, v) => { console.log((v ? "OK   " : "ÉCHEC") + " " + n); if (!v) fails++; };
// références indépendantes de la page
const wilson = (w, n) => { const z = 1.96, p = w / n, d = 1 + z * z / n, c = (p + z * z / (2 * n)) / d, h = z * Math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d; return [Math.round(Math.max(0, c - h) * 100), Math.round(Math.min(1, c + h) * 100)]; };
const C = (n, k) => { if (k < 0 || k > n) return 0; let r = 1; for (let i = 1; i <= k; i++) r = r * (n - k + i) / i; return r; };
const atLeast = (k, s, N) => 1 - C(N - k, s) / C(N, s);
const pct = x => (x * 100).toFixed(x > .9995 ? 0 : 1).replace(".", ",") + "\u00a0%";

async function run(W, H, mobile) {
  console.log(`\n=== ${W}×${H} ${mobile ? "(téléphone, toucher simulé)" : "(ordinateur, souris)"} ===`);
  const ctx = await b.newContext({ viewport: { width: W, height: H }, isMobile: mobile, hasTouch: mobile });
  const pg = await ctx.newPage(); const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 240000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(120); } throw new Error("attente : " + s); };
  const tapLoc = async loc => { await loc.scrollIntoViewIfNeeded(); await pg.waitForTimeout(100); const r = await loc.boundingBox(); if (mobile) await pg.touchscreen.tap(r.x + r.width / 2, r.y + r.height / 2); else await pg.mouse.click(r.x + r.width / 2, r.y + r.height / 2); await pg.waitForTimeout(220); };
  const tap = sel => tapLoc(pg.locator(sel).first());
  const noHScroll = () => ev("document.documentElement.scrollWidth <= innerWidth + 1");
  const menuReady = () => waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=deck]');return b&&!b.disabled&&!!CAT&&!CHKLATER})()");

  await pg.goto(`http://localhost:${PORT}/train.html`);
  await menuReady();

  // ---------------------------------------------------------------- 1. bilan
  const key = await ev("(CAT.presets.find(p=>/akali/i.test(p.deck.legend)&&presetOK(p))||{}).key");
  ok(`une liste d'Akali jouable sert de deck d'entraînement (${key})`, !!key);
  // 22 parties écrites dans le navigateur : 12 victoires sur 12 contre LeBlanc (favorable), 0 sur 6 contre Yi (défavorable), 3 sur 4
  // contre Darius dont une nulle à part ; 2 de plus avec un autre deck ; un duel et une partie en cours, à ne pas compter
  const mk = (i, opp, win, lvl, first, mine, extra = {}) => ({ id: "t" + i, seed: i, names: ["Akali", opp], level: lvl, lvs: 10, whoFirst: first, mine, moves: [], started: "2026-11-2" + (i % 4) + "T10:00:00Z", updated: "2026-11-2" + (i % 4) + "T10:30:00Z",
    result: win === null ? null : { winner: win === 2 ? null : win ? 0 : 1, pts: win === 2 ? [5, 5] : win ? [8, 5] : [4, 8], turn: 12 }, ...extra });
  const G = []; let id = 0;
  for (let i = 0; i < 12; i++) G.push(mk(id++, "LeBlanc", 1, i < 6 ? 6 : 8, i % 2, key));
  for (let i = 0; i < 6; i++) G.push(mk(id++, "Master Yi", 0, 8, i % 2, key));
  for (let i = 0; i < 4; i++) G.push(mk(id++, "Darius", i < 3 ? 1 : 0, 10, 0, key));
  G.push(mk(id++, "Darius", 2, 10, 0, key));
  G.push(mk(id++, "LeBlanc", 0, 6, 1, "autre"), mk(id++, "LeBlanc", 0, 6, 1, "autre"));
  G.push(mk(id++, "Ami", 1, 6, 0, key, { duel: { me: 0, room: "ABCD" } }), mk(id++, "LeBlanc", null, 6, 0, key));
  await ev(`localStorage.setItem('rbt-games', ${JSON.stringify(JSON.stringify(Object.fromEntries(G.map(g => [g.id, g]))))}); localStorage.setItem('rbt-train', ${JSON.stringify(key)}); localStorage.removeItem('rbt-puz')`);
  await pg.reload(); await menuReady();
  await tap("#menu [data-m=bilan]"); await waitFor("!!document.getElementById('bilOpp')");
  const rowsOf = sel => ev(`[...document.querySelectorAll('${sel} .bilrow')].map(r=>({k:r.dataset.k,w:+r.dataset.w,l:+r.dataset.l,lo:+r.dataset.lo,hi:+r.dataset.hi,v:r.querySelector('.bilv').textContent,t:r.textContent.replace(/\\s+/g,' ').trim()}))`);
  const find = (L, k) => L.find(r => r.k === String(k)) || {};
  let tot = (await rowsOf("#bilTot"))[0], opp = await rowsOf("#bilOpp"), lvl = await rowsOf("#bilLvl"), fst = await rowsOf("#bilFst");
  ok(`Bilan du deck : ${tot.w} V, ${tot.l} D (attendu 15 et 7 ; duel, partie en cours et autre deck exclus)`, tot.w === 15 && tot.l === 7);
  const lb = find(opp, "LeBlanc"), yi = find(opp, "Master Yi"), da = find(opp, "Darius");
  const wl = wilson(12, 12), wy = wilson(0, 6), wd = wilson(3, 4);
  ok(`LeBlanc 12-0 : intervalle ${lb.lo}-${lb.hi} % (attendu ${wl}), « ${lb.v} »`, lb.w === 12 && lb.l === 0 && lb.lo === wl[0] && lb.hi === wl[1] && lb.v === "favorable");
  ok(`Master Yi 0-6 : intervalle ${yi.lo}-${yi.hi} % (attendu ${wy}), « ${yi.v} »`, yi.w === 0 && yi.l === 6 && yi.lo === wy[0] && yi.hi === wy[1] && yi.v === "défavorable");
  ok(`Darius 3-1 et une nulle à part : intervalle ${da.lo}-${da.hi} % (attendu ${wd}), « ${da.v} » ; « ${da.t} »`, da.w === 3 && da.l === 1 && da.lo === wd[0] && da.hi === wd[1] && da.v === "on ne sait pas" && /1 N/.test(da.t) && /75 %/.test(da.t));
  ok(`adversaires triés par nombre de parties (${opp.map(r => r.k).join(", ")})`, opp.map(r => r.k).join() === "LeBlanc,Master Yi,Darius");
  ok(`par niveau : 6 → ${find(lvl, 6).w}-${find(lvl, 6).l}, 8 → ${find(lvl, 8).w}-${find(lvl, 8).l}, 10 → ${find(lvl, 10).w}-${find(lvl, 10).l}`, find(lvl, 6).w === 6 && find(lvl, 6).l === 0 && find(lvl, 8).w === 6 && find(lvl, 8).l === 6 && find(lvl, 10).w === 3 && find(lvl, 10).l === 1);
  ok(`selon qui commence : toi ${find(fst, 0).w}-${find(fst, 0).l}, l'IA ${find(fst, 1).w}-${find(fst, 1).l} (somme = total)`, find(fst, 0).w + find(fst, 1).w === 15 && find(fst, 0).l + find(fst, 1).l === 7 && find(fst, 0).w === 9 && find(fst, 0).l === 4);
  ok("Bilan : pas de défilement horizontal, chaque ligne dans sa carte", await noHScroll() && await ev("[...document.querySelectorAll('.bilrow')].every(r=>{const a=r.getBoundingClientRect(),p=r.parentNode.getBoundingClientRect();return a.left>=p.left-1&&a.right<=p.right+1&&[...r.children].every(c=>{const x=c.getBoundingClientRect();return x.left>=a.left-1&&x.right<=a.right+1})})"));
  await pg.screenshot({ path: `${OUT}/entrain-bilan-${W}.png` });
  await tap("#menu [data-bil=all]"); await waitFor("document.querySelector('[data-bil=all]').getAttribute('aria-pressed') === 'true'");
  tot = (await rowsOf("#bilTot"))[0]; const lb2 = find(await rowsOf("#bilOpp"), "LeBlanc");
  ok(`« Tous les decks » : ${tot.w} V, ${tot.l} D (attendu 15 et 9), LeBlanc ${lb2.w}-${lb2.l} (attendu 12-2)`, tot.w === 15 && tot.l === 9 && lb2.w === 12 && lb2.l === 2);
  await ev("BILALL = false; localStorage.setItem('rbt-games', '{}'); showMenu('bilan')"); await waitFor("!!document.getElementById('bilNone')");
  ok("sans partie finie : le bilan le dit, sans tableau", await ev("!document.getElementById('bilOpp')"));

  // ---------------------------------------------------------------- 2. testeur de main
  await ev("showMenu('main')"); await tap("#menu [data-m=deck]"); await waitFor("!!document.getElementById('dkSave')");
  await pg.selectOption("#dkLoad", key); await waitFor("!!DK.legend && cnt(DK.main) === 39");
  await tap("#dkHand"); await waitFor("!!document.getElementById('htHand')");
  const hand = () => ev("[...document.querySelectorAll('#htHand .htc')].map(c=>({n:c.dataset.n,out:c.classList.contains('out'),nw:c.classList.contains('new'),img:/background-image/.test(c.querySelector('.dkimg').getAttribute('style')||'')}))");
  let hd = await hand();
  ok(`main de départ : ${hd.length} cartes illustrées, toutes du deck, deck restant ${await ev("HT.deck.length")} (attendu 4 et 35)`, hd.length === 4 && hd.every(c => c.img) && await ev("HT.hand.every(c=>DK.main[c.n]>0) && HT.deck.length === 35"));
  ok("le champion choisi n'est jamais dans la pioche au-delà de ses exemplaires du deck", await ev("(()=>{const c={};for(const n of [...HT.deck,...HT.hand.map(x=>x.n)])c[n]=(c[n]||0)+1;return Object.keys(DK.main).every(n=>c[n]===DK.main[n])&&Object.keys(c).length===Object.keys(DK.main).length})()"));
  const tapCard = i => tapLoc(pg.locator("#htHand .htc").nth(i));
  await tapCard(0); await tapCard(2); await tapCard(3);
  hd = await hand();
  ok(`mulligan : 2 cartes au plus (3 touchées, ${hd.filter(c => c.out).length} renvoyées), bouton « ${await ev("document.getElementById('htKeep').textContent")} »`, hd.filter(c => c.out).length === 2 && hd[0].out && hd[2].out && !hd[3].out);
  const aside = [hd[0].n, hd[2].n], kept = [hd[1].n, hd[3].n];
  await tap("#htKeep"); await waitFor("HT.phase === 'play'");
  hd = await hand();
  ok(`après mulligan : ${hd.length} cartes, les 2 gardées en tête, 2 piochées, les 2 renvoyées sous le deck`, hd.length === 4 && hd[0].n === kept[0] && hd[1].n === kept[1] && hd.filter(c => c.nw).length === 2
    && await ev(`JSON.stringify(HT.deck.slice(-2).sort()) === ${JSON.stringify(JSON.stringify(aside.slice().sort()))} && HT.deck.length === 35`));
  await tap("#htNext"); await waitFor("HT.turn === 1");
  ok(`tour 1 en commençant : ${await ev("HT.runes.length")} runes (attendu 2), ${await ev("HT.hand.length")} cartes (attendu 5)`, await ev("HT.runes.length === 2 && HT.hand.length === 5 && +document.getElementById('htRunes').dataset.n === 2"));
  await tap("#htNext"); await tapCard(0); await waitFor("HT.played === 1");
  ok(`tour 2 : 4 runes, une carte sortie de la main (${await ev("HT.hand.length")} en main, attendu 5)`, await ev("HT.runes.length === 4 && HT.hand.length === 5 && HT.turn === 2"));
  await tap("#mbox [data-ht=second]"); await waitFor("HT.phase === 'mull' && !HT.first");
  await tap("#htKeep"); await tap("#htNext"); await waitFor("HT.turn === 1");
  ok(`en second : ${await ev("HT.runes.length")} runes au tour 1 (attendu 3, règle 485.7), puis ${await ev("(htNext(), HT.runes.length)")} au tour 2 (attendu 5)`, await ev("HT.runes.length === 5 && HT.hand.length === 6"));
  for (let i = 0; i < 6; i++) await ev("htNext()");
  ok(`les 12 runes sorties, jamais plus (${await ev("HT.runes.length")}), aux couleurs du deck`, await ev("HT.runes.length === 12 && Object.entries(DK.runes).every(([d,q])=>HT.runes.filter(x=>x===d).length===q)"));
  await ev("htRender()");
  // probabilités : chaque ligne recalculée par dénombrement
  const tab = await ev("[...document.querySelectorAll('#htTab tbody tr[data-ht=sel]')].map(r=>({n:r.dataset.n,k:+r.dataset.k,c:[...r.cells].slice(2).map(td=>td.textContent)}))");
  const bad = tab.filter(r => [4, 6, 8, 10].some((s, j) => r.c[j] !== pct(atLeast(r.k, s, 39))));
  ok(`probabilités : ${tab.length} lignes, toutes égales au dénombrement (ex. ${tab[0].n} ×${tab[0].k} : ${tab[0].c.join(" | ")})`, tab.length > 8 && bad.length === 0);
  const two = tab.slice(0, 2);
  await tapLoc(pg.locator("#htTab tbody tr[data-ht=sel]").nth(0)); await tapLoc(pg.locator("#htTab tbody tr[data-ht=sel]").nth(1)); await waitFor("!!document.getElementById('htGrp')");
  const grp = await ev("({k:+document.getElementById('htGrp').dataset.k,c:[...document.getElementById('htGrp').cells].slice(2).map(td=>td.textContent)})");
  ok(`deux lignes cochées : au moins une des ${grp.k} cartes = ${grp.c.join(" | ")}`, grp.k === two[0].k + two[1].k && [4, 6, 8, 10].every((s, j) => grp.c[j] === pct(atLeast(grp.k, s, 39))));
  const lay = await ev("(()=>{const m=document.getElementById('mbox').getBoundingClientRect();const cs=[...document.querySelectorAll('#htHand .htc')].map(c=>c.getBoundingClientRect());return {n:cs.length,in:cs.every(r=>r.left>=m.left-1&&r.right<=m.right+1),w:Math.round(Math.min(...cs.map(r=>r.width))),tab:document.getElementById('htTab').getBoundingClientRect().right<=m.right+1,msw:document.getElementById('mbox').scrollWidth<=document.getElementById('mbox').clientWidth+1,line:[...document.querySelectorAll('#htTab tbody td')].every(td=>{if(td.cellIndex<2)return true;const r=document.createRange();r.selectNodeContents(td);return r.getClientRects().length===1})}})()");
  ok(`testeur de main : pas de défilement horizontal, ${lay.n} cartes dans la fenêtre (les plus étroites ${lay.w} px), tableau dans la fenêtre, pourcentages sur une ligne`, await noHScroll() && lay.in && lay.w >= (mobile ? 60 : 88) && lay.tab && lay.msw && lay.line || (console.log('      détail :', JSON.stringify(lay)), false));
  await ev("document.getElementById('mbox').scrollTop = 0"); await pg.screenshot({ path: `${OUT}/entrain-main-${W}.png` });
  await ev("closeModal(); document.getElementById('dkMenu').click()"); await pg.waitForTimeout(300);
  if (await ev("!document.getElementById('modal').hidden")) await ev("closeModal()");

  // ---------------------------------------------------------------- 3. exercices
  const game = async (seed, n) => {   // une partie par vrais appels de la page (choose / answer enregistrent les coups)
    await ev("showMenu('main')"); await tap("#menu [data-m=new]"); await waitFor("!!document.getElementById('bStart')");
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
  await game(21, 22);
  const target = await ev("REC.id");
  await game(22, 5);
  const live = await ev("JSON.stringify([V.st.t, V.st.pts, V.st.p[0].hand])"), liveRec = await ev("REC.id + ':' + REC.moves.length");
  await ev("showMenu('puz')"); await waitFor(`!!document.querySelector('[data-pzscan="${target}"]')`);
  ok("« Exercices » : aucune liste avant analyse, les parties jouées sont proposées", await ev("document.querySelectorAll('.pzrow').length === 0 && document.querySelectorAll('[data-pzscan]').length >= 2"));
  const t0 = Date.now();
  await tap(`[data-pzscan="${target}"]`);
  await waitFor("!!document.getElementById('pzStop') || !!(pzStore()[" + JSON.stringify(target) + "])");
  await waitFor(`!!pzStore()[${JSON.stringify(target)}] && !working`, 600000);
  const sc = await ev(`pzStore()[${JSON.stringify(target)}]`);
  console.log(`      analyse : ${sc.n} décisions en ${((Date.now() - t0) / 1000).toFixed(0)} s, ${sc.items.length} exercices, écarts ${sc.items.map(x => x.lose ? "perd" : x.gap + "±" + x.se).join("  ")}`);
  ok(`analyse complète (${sc.n}/${sc.total} décisions), ${sc.items.length} exercices, tous à 3 ou plus du conseil et à plus de 2 erreurs types, coup joué ≠ coup conseillé`, sc.n === sc.total && sc.n >= 10 && !sc.part && sc.items.length >= 1 && sc.items.every(x => x.gap <= -3 && (x.se === null || -x.gap >= 2 * x.se) && x.mine && x.best && x.mine !== x.best));
  ok("l'analyse ne touche pas à la partie en cours ni à son enregistrement", await ev("JSON.stringify([V.st.t, V.st.pts, V.st.p[0].hand])") === live && await ev("REC.id + ':' + REC.moves.length") === liveRec && await ev("!RP"));
  await waitFor("document.querySelectorAll('.pzrow').length > 0");
  ok(`la liste montre ${sc.items.length} exercice(s), le plus grand écart en tête, sans dévoiler le coup`, await ev(`(()=>{const R=[...document.querySelectorAll('.pzrow')];const it=pzStore()[${JSON.stringify(target)}].items;return R.length===it.length&&+R[0].dataset.k===pzList()[0].it.k&&R.every(r=>it.every(x=>!r.textContent.includes(x.best)))})()`));
  ok("Exercices (liste) : pas de défilement horizontal", await noHScroll());
  await pg.screenshot({ path: `${OUT}/entrain-exercices-${W}.png` });
  await tapLoc(pg.locator(".pzrow [data-pzgo]").first());
  await waitFor("!!(RP && RP.pz && RP.shown === RP.pz.k && document.getElementById('pzAsk'))");
  const ex = await ev("({k:RP.pz.k,best:RP.pz.it.best,mine:RP.pz.it.mine,nopt:V.dec?V.dec.options.length:0,warn:RP.warn,menu:$('menu').hidden,undo:$('bUndo').hidden,hint:$('bHint').hidden,nav:!!document.querySelector('#rpbar [data-rpg]'),main:$('bMain').disabled})");
  ok(`l'exercice s'ouvre sur le plateau à ta décision (${ex.nopt} coups possibles), sans Conseil, sans Reprendre, sans navigation qui dévoile la suite`, ex.nopt >= 2 && ex.menu && ex.undo && ex.hint && !ex.nav && !ex.warn.length);
  const inBar = () => ev(`(()=>{ const r = $('rpbar').getBoundingClientRect(), c = $('ctrl').getBoundingClientRect(); const over = ${mobile} && r.bottom > c.top + 1;
    return r.width > 0 && r.left >= 0 && r.right <= innerWidth + 1 && r.top >= 0 && r.bottom <= innerHeight + 1 && !over && document.documentElement.scrollWidth <= innerWidth + 1 })()`);
  ok("barre de l'exercice à l'écran, sans couvrir la commande", await inBar());
  await pg.screenshot({ path: `${OUT}/entrain-exercice-${W}.png` });
  // a) un vrai geste : le gros bouton (terminer le tour / passer) s'il est offert, sinon le coup joué dans la partie
  const viaBtn = await ev("!$('bMain').disabled && V.dec.options.some(o => o.k === 'end' || o.k === 'pass')");
  if (viaBtn) await tap("#bMain"); else await ev("choose(V.dec.options.find(o => normLab(o.label) === normLab(RP.pz.it.mine)).i)");
  await waitFor("!!(RP && RP.pz && RP.pz.res && document.getElementById('pzV'))", 180000);
  const r1 = await ev("({ok:document.getElementById('pzV').dataset.ok,txt:document.getElementById('pzV').textContent,rows:[...document.querySelectorAll('#rpbar .hintrow')].map(r=>r.textContent),ans:RP.pz.ans,dec:!!V.dec,lab:RP.pz.lab,coh:RP.pz.res.me?pzWorse(RP.pz.res.me)===(document.getElementById('pzV').dataset.ok==='0'):document.getElementById('pzV').dataset.ok==='0'})");
  ok(`coup noté ${viaBtn ? "par le gros bouton" : "(coup de la partie)"} : « ${r1.lab} » → « ${r1.txt.slice(0, 70)} » (${r1.rows.length} coups affichés, le meilleur en tête, ton choix marqué)`, r1.rows.length >= 2 && /meilleur|best/.test(r1.rows[0]) && r1.rows.some(x => /ton choix/.test(x)) && !r1.dec
    && r1.coh);
  ok("le coup n'est pas envoyé au moteur : enregistrement et partie en cours inchangés", await ev("REC.id + ':' + REC.moves.length") === liveRec);
  ok("verdict à l'écran, sans couvrir la commande", await inBar());
  await pg.screenshot({ path: `${OUT}/entrain-verdict-${W}.png` });
  // b) réessayer avec le coup préféré de l'IA : réussi
  await tap("#rpbar [data-pz=retry]"); await waitFor("pzLive() && RP.shown === RP.pz.k && !!V.dec");
  await ev("choose(V.dec.options.find(o => normLab(o.label) === normLab(RP.pz.it.best)).i)");
  await waitFor("!!(RP && RP.pz && RP.pz.res && RP.pz.ans >= 0 && document.getElementById('pzV'))", 180000);
  ok(`« Réessayer » puis le coup préféré de l'IA : « ${await ev("document.getElementById('pzV').textContent")} »`, await ev("document.getElementById('pzV').dataset.ok === '1' && pzStore()[RP.pz.gid].done[RP.pz.k] === 1"));
  // c) réessayer avec le coup de la partie : pas réussi, mais l'exercice reste acquis
  await tap("#rpbar [data-pz=retry]"); await waitFor("pzLive() && RP.shown === RP.pz.k && !!V.dec");
  await ev("choose(V.dec.options.find(o => normLab(o.label) === normLab(RP.pz.it.mine)).i)");
  await waitFor("!!(RP && RP.pz && RP.pz.res && RP.pz.ans >= 0 && document.getElementById('pzV'))", 180000);
  ok(`le coup joué dans la partie : « ${(await ev("document.getElementById('pzV').textContent")).slice(0, 60)} », marqué « joué dans la partie »`, await ev("document.getElementById('pzV').dataset.ok === '0' && [...document.querySelectorAll('#rpbar .hintrow')].some(r=>/joué dans la partie/.test(r.textContent)) && pzStore()[RP.pz.gid].done[RP.pz.k] === 1") || (console.log("      détail :", await ev("JSON.stringify([document.getElementById('pzV').dataset.ok, [...document.querySelectorAll('#rpbar .hintrow')].map(r=>r.textContent), RP.pz.it, pzStore()[RP.pz.gid].done])")), false));
  // d) voir la suite : la barre de lecture normale du replay
  await tap("#rpbar [data-pz=replay]"); await waitFor("!!document.querySelector('#rpbar [data-rpg]') && !!document.getElementById('rpQuit')");
  ok("« Voir la suite de la partie » rend la barre de lecture du replay", await ev("!RP.pz && !V.dec"));
  await tap("#rpQuit"); await waitFor("!RP && !$('menu').hidden && MENUV === 'puz' && document.querySelectorAll('.pzrow').length > 0");
  ok("quitter ramène au menu « Exercices », l'exercice est marqué réussi", await ev("[...document.querySelectorAll('.pzrow .bilv')].some(x=>/réussi/.test(x.textContent))"));
  await ev("hideMenu()");
  ok("la partie en cours revient telle quelle", await ev("JSON.stringify([V.st.t, V.st.pts, V.st.p[0].hand])") === live && await ev("REC.id + ':' + REC.moves.length") === liveRec);
  // e) « Voir la réponse » sans jouer, sur un autre exercice s'il y en a un
  if (sc.items.length > 1) {
    await ev("showMenu('puz')"); await waitFor("document.querySelectorAll('.pzrow').length > 1");
    await tapLoc(pg.locator(".pzrow [data-pzgo]").nth(1)); await waitFor("!!(RP && RP.pz && RP.shown === RP.pz.k && document.getElementById('pzAsk'))");
    await tap("#rpbar [data-pz=show]"); await waitFor("!!(RP && RP.pz && RP.pz.res && document.getElementById('pzV'))", 180000);
    ok("« Voir la réponse » montre le conseil sans compter l'exercice comme fait", await ev("RP.pz.ans === -1 && document.querySelectorAll('#rpbar .hintrow').length >= 2 && pzStore()[RP.pz.gid].done[RP.pz.k] === undefined"));
    // f) « Exercice suivant » (gros bouton) : un autre exercice, de nouveau jouable
    const k0 = await ev("RP.pz.k");
    ok(`gros bouton « ${await ev("$('bMain').textContent")} » actif après le verdict`, await ev("!$('bMain').disabled") || (console.log("      détail :", await ev("JSON.stringify([RP.busy, working, $('bMain').disabled])")), false));
    await tap("#bMain"); await waitFor("!!(RP && RP.pz && RP.pz.k !== " + k0 + " && RP.shown === RP.pz.k && pzLive() && V.dec && document.getElementById('pzAsk'))");
    ok(`« Exercice suivant » ouvre un exercice pas encore réussi (étape ${k0} → ${await ev("RP.pz.k")})`, await ev("pzStore()[RP.pz.gid].done[RP.pz.k] !== 1"));
    await tap("#rpbar [data-pz=quit]"); await waitFor("!RP && MENUV === 'puz'");
  }
  await ev(`document.querySelector('[data-pzdel="${target}"]').click()`); await waitFor("document.querySelectorAll('.pzrow').length === 0");
  ok("« Oublier » retire les exercices de la partie", await ev(`!pzStore()[${JSON.stringify(target)}]`));
  ok(`aucune erreur JavaScript${errs.length ? " : " + errs.join(" | ") : ""}`, errs.length === 0);
  await ctx.close();
}
await run(1400, 900, false);
await run(393, 851, true);
await b.close();
console.log(fails ? `\n${fails} ÉCHEC(S)` : "\nTout est bon.");
process.exit(fails ? 1 : 0);
