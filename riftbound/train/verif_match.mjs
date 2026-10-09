import { existsSync } from "node:fs";
// Playwright : celui de l'environnement cloud Claude, sinon le paquet npm « playwright » (CI).
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
// Robot de vérification du sideboard (éditeur) et du match BO3 (tirage, premier joueur, battlefield, manche jouée jusqu'au
// bout, score, sideboard 1 pour 1, battlefield retiré après une manche gagnée, lancement de la manche 2), par de vrais clics.
// Joué deux fois : téléphone 360×740 (toucher) et ordinateur 1400×900 (souris).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_match.mjs <dossier des captures> [port]
// Donnes : 11 sur téléphone (l'IA gagne le tirage), 12 sur ordinateur (tu gagnes le tirage) : les deux cas sont joués.
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0;
const ok = (n, v) => { console.log((v ? "OK   " : "ÉCHEC") + " " + n); if (!v) fails++; };

async function run(W, H, mobile, SEED) {
  const tag = `${W}`;
  console.log(`\n=== ${W}×${H} ${mobile ? "(téléphone, toucher)" : "(ordinateur, souris)"} ===`);
  const ctx = await b.newContext({ viewport: { width: W, height: H }, isMobile: mobile, hasTouch: mobile });
  const pg = await ctx.newPage(); const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const tap = async sel => { await pg.locator(sel).first().scrollIntoViewIfNeeded(); return mobile ? pg.tap(sel) : pg.click(sel); };
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(150); } throw new Error("attente : " + s); };
  const noHScroll = () => ev("document.documentElement.scrollWidth <= innerWidth + 1 && (!document.querySelector('#modal:not([hidden]) .mbox') || document.querySelector('#mbox').scrollWidth <= document.querySelector('#mbox').clientWidth + 1)");
  const small = sel => ev(`[...document.querySelectorAll(${JSON.stringify(sel)})].map(x=>{const r=x.getBoundingClientRect();return Math.min(r.width,r.height)}).filter(v=>v>0)`);
  const sideN = () => ev("+document.querySelector('#dkSideH b').textContent");

  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=deck]');return b&&!b.disabled})()");
  // ---------- (a) éditeur : copie de LeBlanc IQ#5, sideboard
  await tap("[data-m=deck]"); await waitFor("!!document.getElementById('dkSave')");
  await pg.selectOption("#dkLoad", "leblanc-iq5"); await waitFor("!!document.getElementById('dkSideH')");
  ok("éditeur : copie de LeBlanc IQ#5, section « Sideboard 10/10 »", await sideN() === 10 && /Sideboard/.test(await ev("document.getElementById('dkSideH').textContent")));
  ok("éditeur : deck copié jouable", await ev("document.querySelector('.dkst').classList.contains('ok')"));
  await pg.screenshot({ path: `${OUT}/editeur-sideboard-${tag}.png` });
  // retirer une carte du sideboard (bouton − de la ligne)
  const first = await ev("document.querySelector('[data-ssub]').dataset.ssub");
  await tap(`[data-ssub="${first}"]`);
  ok(`éditeur : − retire « ${first} » du sideboard (9/10)`, await sideN() === 9);
  // ajouter une carte au sideboard depuis la liste, avec l'interrupteur « Ajouter au : Sideboard »
  await tap("#dkToSide");
  ok("éditeur : interrupteur « Ajouter au : Sideboard » actif", await ev("document.getElementById('dkToSide').getAttribute('aria-pressed') === 'true'"));
  const pick = await ev("(()=>{const id=ident(); const c=CAT.cards.find(c=>c.m && MAINT(c) && copies(c.n)===0 && c.d.length && c.d.every(d=>id.includes(d)) && c.s!=='Signature' && c.s!=='Champion'); return c && c.n})()");
  await pg.fill("#dkQ", pick); await pg.waitForTimeout(300);
  await tap(`.dkgrid [data-add="${pick.replace(/"/g, '\\"')}"]`);
  ok(`éditeur : « ${pick} » ajoutée au sideboard depuis la liste (10/10, deck principal inchangé)`, await sideN() === 10 && await ev(`DK.sideboard[${JSON.stringify(pick)}] === 1 && !DK.main[${JSON.stringify(pick)}]`));
  // 11e carte refusée (601.1.c.1)
  await tap(`.dkgrid [data-add="${pick.replace(/"/g, '\\"')}"]`);
  ok("éditeur : 11e carte de sideboard refusée (10 au plus)", await sideN() === 10);
  await tap("#dkToMain");
  // déplacer deck → sideboard → deck
  await tap(`[data-tmain="${pick.replace(/"/g, '\\"')}"]`);
  const afterMove = await ev(`[cnt(DK.sideboard), cnt(DK.main) + (DK.champion?1:0), DK.main[${JSON.stringify(pick)}]||0]`);
  ok(`éditeur : ⇄ passe « ${pick} » du sideboard au deck (sideboard ${afterMove[0]}, deck ${afterMove[1]})`, afterMove[0] === 9 && afterMove[1] === 41 && afterMove[2] === 1);
  ok("éditeur : le deck à 41 cartes reste jouable, le sideboard à 9 aussi", await ev("document.querySelector('.dkst').classList.contains('ok')"));
  await tap(`[data-tside="${pick.replace(/"/g, '\\"')}"]`);
  ok("éditeur : ⇄ la remet au sideboard (10/10, deck 40)", await sideN() === 10 && await ev("cnt(DK.main) + (DK.champion?1:0) === 40"));
  const sz = await small(".dkr button, .rn button, .dkto button");
  if (mobile) ok(`éditeur : boutons ≥ 44 px (${sz.length} boutons, plus petit ${Math.min(...sz)} px)`, sz.length > 0 && Math.min(...sz) >= 44);
  ok("éditeur : pas de défilement horizontal", await noHScroll());
  await pg.fill("#dkName", "LeBlanc SB test");
  await tap("#dkSave"); await pg.waitForTimeout(400);
  const saved = await ev("(()=>{const d=JSON.parse(localStorage.getItem('rbt-decks')||'{}')['leblanc-sb-test']; return d ? Object.values(d.sideboard||{}).reduce((a,b)=>a+b,0) : -1})()");
  ok(`éditeur : deck enregistré avec son sideboard (localStorage « rbt-decks », ${saved} cartes)`, saved === 10);
  await pg.locator(".dkdeck").evaluate(e => { e.scrollTop = e.scrollHeight; });
  await pg.evaluate(() => document.getElementById("dkSideH").scrollIntoView());
  await pg.screenshot({ path: `${OUT}/editeur-sideboard-modifie-${tag}.png` });

  // ---------- (b) BO3 avec ce deck
  await tap("#dkPlay"); await waitFor("!!document.getElementById('bStart')");
  const SAVED = "(d=>JSON.stringify([d.champion, d.main, d.sideboard]))(MYDECKS['leblanc-sb-test'])", sideSaved = await ev(SAVED);
  ok("nouvelle partie : ton deck = le deck enregistré", await ev("document.getElementById('sMine').value") === "my:leblanc-sb-test");
  // écran « Contre l'IA » : panneau IA, puis la légende Akali (le menu ne montre que ses variantes), puis la liste G2
  await tap(".vpad.cpu .vtag"); await tap('.vtile[data-leg="Akali, Rogue Assassin"]');
  await pg.selectOption("#sOpp", "akali-g2");
  await tap("#sBo3");
  ok("nouvelle partie : BO3 choisi, battlefield et premier joueur masqués", await ev("document.getElementById('sBo3').getAttribute('aria-checked')==='true' && document.getElementById('sBf').closest('label').hidden"));
  await pg.click(".vopt summary"); await pg.fill("#sSeed", SEED);   // la donne est dans « Plus d'options »
  await pg.screenshot({ path: `${OUT}/nouvelle-partie-bo3-${tag}.png` });
  await tap("#bStart"); await waitFor("!!document.getElementById('bLaunch')");
  const roll = await ev("document.getElementById('pRoll').textContent");
  ok(`manche 1 : tirage affiché (« ${roll} »)`, /tirage/.test(roll));
  ok("manche 1 : « Lancer » bloqué tant que rien n'est choisi", await ev("document.getElementById('bLaunch').disabled"));
  let firstPick;
  if (await ev("!!document.querySelector('[data-pf]')")) {
    ok("manche 1 : question « Jouer en premier ou en second ? »", /premier ou en second/.test(await ev("document.getElementById('mbox').textContent")));
    firstPick = mobile ? 0 : 1;
    await tap(`[data-pf="${firstPick}"]`);
  } else {
    const dec = await ev("document.getElementById('pDecision').textContent");
    ok(`manche 1 : décision de l'IA affichée (« ${dec} »)`, /L'IA/.test(dec));
  }
  const bfs1 = await ev("[...document.querySelectorAll('[data-pbf]')].map(x=>x.dataset.pbf)");
  ok(`manche 1 : 3 battlefields proposés (${bfs1.join(", ")})`, bfs1.length === 3);
  await tap(`[data-pbf="${bfs1[1]}"]`);
  ok("prépa : pas de défilement horizontal", await noHScroll());
  const tgt = await small("#mbox .choices button, #bLaunch");
  ok(`prépa : boutons ≥ 44 px (plus petit ${Math.min(...tgt)} px)`, Math.min(...tgt) >= 44);
  await pg.screenshot({ path: `${OUT}/bo3-manche1-prepa-${tag}.png` });
  await tap("#bLaunch");
  await waitFor("!!(V && META)");
  const m1 = await ev("({bf: META.bf, first: META.first, chip: document.getElementById('gmatch').textContent, rec: REC.match, obf: REC.obf})");
  ok(`manche 1 lancée : battlefields ${m1.bf.join(" / ")}, ${m1.first === 0 ? "tu commences" : "l'IA commence"}`, m1.bf[0] === bfs1[1] && (firstPick === undefined || m1.first === firstPick));
  ok(`barre d'infos : « ${m1.chip} »`, m1.chip === "Manche 1 · toi 0 – 0 IA");
  ok(`REC.match = ${JSON.stringify(m1.rec)}, obf = ${m1.obf}`, m1.rec && m1.rec.mode === "bo3" && m1.rec.game === 1 && m1.obf === m1.bf[1]);
  let seed = 12345; const rnd = n => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed % n; };
  let moves = 0;
  for (let i = 0; i < 3000; i++) {
    await waitFor("!!(V && !working)");
    if (await ev("V.winner !== null && V.winner !== undefined")) break;
    if (await ev("!!document.getElementById('bKeep')")) { await tap("#bKeep"); continue; }
    if (await ev("!!V.ask")) { const k = await ev("V.ask.options.length"); await ev(`(async()=>{ await answer(${rnd(k)}) })()`); moves++; continue; }
    const k = await ev("V.dec ? V.dec.options.length : 0");
    if (k) { const i2 = rnd(k); await ev(`(async()=>{ await choose(V.dec.options[${i2}].i) })()`); moves++; }
  }
  await waitFor("!!(V && !working)"); await pg.waitForTimeout(700);
  const w1 = await ev("V.winner");
  ok(`manche 1 jouée jusqu'au bout (${moves} coups, tour ${await ev("V.st.t")}, ${await ev("V.st.pts.join('-')")}, ${w1 === 0 ? "victoire" : w1 === 1 ? "défaite" : "nulle"})`, w1 !== null && w1 !== undefined);
  const sc = await ev("({f: (document.getElementById('fScore')||{}).textContent, chip: document.getElementById('gmatch').textContent, wins: MATCH.st.wins, main: document.getElementById('bMain').textContent})");
  const exp = w1 === 0 ? "toi 1 – 0 IA" : w1 === 1 ? "toi 0 – 1 IA" : "toi 0 – 0 IA";
  ok(`score du match : « ${sc.f} », barre « ${sc.chip} », bouton « ${sc.main} »`, sc.f && sc.f.includes("Match : " + exp) && sc.chip.includes(exp) && sc.main === "Manche suivante");
  ok("REC.result et REC.match.wins enregistrés", await ev("!!(REC.result && REC.match && REC.match.wins)"));
  await pg.screenshot({ path: `${OUT}/bo3-fin-manche1-${tag}.png` });

  // ---------- (c) sideboard puis manche 2
  await tap("#bNextGame");
  if (w1 === 0 || w1 === 1) {
    await waitFor("!!document.getElementById('sbOk')", 10000);
    ok("manche 2 : écran de sideboard proposé", true);
    ok("sideboard : « Valider » bloqué sans échange", await ev("document.getElementById('sbOk').disabled"));
    const out = await ev("document.querySelector('[data-sbp=out]').dataset.n"), inn = await ev("document.querySelector('[data-sbp=inn]').dataset.n");
    await tap(`[data-sbp="out"][data-n="${out.replace(/"/g, '\\"')}"]`);
    ok("sideboard : 1 sort, 0 entre → « Valider » bloqué (1 pour 1, 403.4)", await ev("document.getElementById('sbOk').disabled && document.getElementById('sbStat').classList.contains('ko')"));
    await tap(`[data-sbp="inn"][data-n="${inn.replace(/"/g, '\\"')}"]`);
    ok("sideboard : pas de défilement horizontal", await noHScroll());
    const tg2 = await small("#mbox .sbr button:not(:disabled), #sbOk, #sbKeep");
    ok(`sideboard : boutons ≥ 44 px (plus petit ${Math.min(...tg2)} px)`, Math.min(...tg2) >= 44);
    await pg.screenshot({ path: `${OUT}/bo3-sideboard-${tag}.png` });
    const before = await ev("JSON.stringify(MATCH.deck)");
    await tap("#sbOk"); await waitFor("!!document.getElementById('bLaunch')", 10000);
    const d2 = await ev("MATCH.deck"), d1 = JSON.parse(before);
    const c = (xs, n) => xs.filter(x => x === n).length;
    ok(`sideboard : « ${out} » sort, « ${inn} » entre (deck ${d2.main.length + 1}, sideboard ${d2.sideboard.length})`,
      c(d2.main, out) === c(d1.main, out) - 1 + (out === inn ? 1 : 0) && c(d2.main, inn) === c(d1.main, inn) + 1 - (out === inn ? 1 : 0) && d2.main.length === d1.main.length && d2.sideboard.length === d1.sideboard.length);
    ok("sideboard : le deck enregistré dans « Mes decks » n'a pas changé (403.8)", await ev(SAVED) === sideSaved);
  } else {
    await waitFor("!!document.getElementById('bLaunch')", 10000);
    ok("manche 1 nulle : pas de sideboard (403.10)", !(await ev("!!document.getElementById('sbOk')")));
  }
  const prep2 = await ev("({t: document.getElementById('mbox').textContent, bfs: [...document.querySelectorAll('[data-pbf]')].map(x=>x.dataset.pbf), gone: !!document.getElementById('pGone'), pf: !!document.querySelector('[data-pf]')})");
  ok(`manche 2 : score « ${await ev("document.getElementById('pScore').textContent")} »`, prep2.t.includes("Match : " + exp));
  // 486.5 : après une manche gagnée (par l'un ou l'autre), les battlefields joués sont retirés ; après une nulle ils resservent.
  if (w1 === 0 || w1 === 1) ok(`manche 2 : « ${bfs1[1]} » (joué en manche 1, gagnée par ${w1 === 0 ? "toi" : "l'IA"}) n'est plus proposé : ${prep2.bfs.join(", ")} (486.5)`, !prep2.bfs.includes(bfs1[1]) && prep2.bfs.length === 2 && prep2.gone);
  else ok(`manche 2 : tes 3 battlefields restent proposés (manche nulle, 486.5.a) : ${prep2.bfs.join(", ")}`, prep2.bfs.length === 3);
  if (w1 === 0) ok("manche 2 : l'IA a perdu, elle choisit (407.4)", /L'IA a perdu/.test(prep2.t) && !prep2.pf);
  if (w1 === 1) ok("manche 2 : tu as perdu, tu choisis (407.4)", /Tu as perdu/.test(prep2.t) && prep2.pf);
  if (prep2.pf) await tap('[data-pf="0"]');
  await tap(`[data-pbf="${prep2.bfs[0]}"]`);
  await pg.screenshot({ path: `${OUT}/bo3-manche2-prepa-${tag}.png` });
  await tap("#bLaunch"); await waitFor("!!(V && REC && REC.match && REC.match.game === 2 && !working)");
  const m2 = await ev("({bf: META.bf, chip: document.getElementById('gmatch').textContent, mine: REC.mine, seed: META.seed})");
  ok(`manche 2 lancée : ${m2.bf.join(" / ")}, donne de la manche ${m2.seed}, barre « ${m2.chip} »`, m2.bf[0] === prep2.bfs[0] && m2.chip.startsWith("Manche 2 · " + exp));
  if (w1 === 0 || w1 === 1) ok("manche 2 : le moteur reçoit le deck après sideboard (REC.mine)", m2.mine === await ev("JSON.stringify(MATCH.deck)"));
  ok("table : pas de défilement horizontal", await ev("document.documentElement.scrollWidth <= innerWidth + 1"));
  await pg.screenshot({ path: `${OUT}/bo3-manche2-${tag}.png` });
  ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 3).join(" / ") : ""), errs.length === 0);
  await ctx.close();
}

try { await run(360, 740, true, "11"); } catch (e) { ok("parcours 360×740 : " + e.message, false); }
try { await run(1400, 900, false, "12"); } catch (e) { ok("parcours 1400×900 : " + e.message, false); }
await b.close();
console.log(fails ? `\n${fails} échec(s)` : "\nTout est vert.");
process.exitCode = fails ? 1 : 0;
