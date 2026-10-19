import { existsSync, writeFileSync } from "node:fs";
// Robot des équipements [Deploy] (règle 830) dans la vraie table : un deck perso (Kha'Zix + 3 Pillaged Armory, gear
// Body à 1 avec [Deploy]) contre l'IA débutante. Dès que Pillaged Armory est jouable, on le glisse de la main sur un
// battlefield (souris sur ordinateur, toucher simulé par des évènements pointer sur téléphone). Contrôles : seuls les
// battlefields sont proposés (pas la base), l'équipement s'affiche sur ce battlefield du côté du joueur et pas dans la
// base, il reste là après le cleanup ; puis la partie continue jusqu'au bout. Ordinateur (1400x900) et téléphone
// (393x851). Demande du 2026-10-19.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_deploy.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
const GEAR = "Pillaged Armory";
const DECK = { id: "deploy-test", name: "Deploy test", legend: "Kha'Zix, Voidreaver", champion: "Kha'Zix, Mutating Horror",
  main: null, sideboard: {}, runes: null, battlefields: null };
const SEEDS = (process.env.RB_DEPLOY_SEEDS || "8,4,5,6,7,9").split(",").map(Number);
for (const [w, h, mob] of [[1400, 900, false], [393, 851, true]]) {
  const pg = await (await b.newContext({ viewport: { width: w, height: h }, isMobile: mob, hasTouch: mob })).newPage();
  const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 240000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  if (!DECK.main) {          // liste khazix_cuoemrei_rq-los-angeles_20th, les 3 cartes les plus chères remplacées
    const d = JSON.parse(await ev("JSON.stringify(CAT.presets.find(p => p.key === 'khazix_cuoemrei_rq-los-angeles_20th').deck)"));
    const CAT_E = await ev("Object.fromEntries(CAT.cards.map(c => [c.n, +c.e || 0]))");
    const main = d.main.slice(), idx = [...main.keys()].sort((a, c) => (CAT_E[main[c]] || 0) - (CAT_E[main[a]] || 0) || a - c);
    for (const i of idx.slice(0, 3)) main[i] = GEAR;
    DECK.main = {}; for (const n of main) DECK.main[n] = (DECK.main[n] || 0) + 1;
    DECK.runes = {}; for (const r of d.runes) { const k = r.replace(/ Rune$/, ""); DECK.runes[k] = (DECK.runes[k] || 0) + 1; }
    DECK.battlefields = d.battlefields.slice(0, 3);
  }
  await ev(`localStorage.setItem("rbt-decks", ${JSON.stringify(JSON.stringify({ "deploy-test": DECK }))}); localStorage.setItem("rbt-mine", "my:deploy-test")`);
  await pg.reload();
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  const problems = await ev("(async()=>(await J(T.check(JSON.stringify(toEngine(MYDECKS['deploy-test']))))).filter(e => !e.startsWith('⚠')))()").catch(e => String(e));
  ok(`${w} px : le deck avec 3 ${GEAR} passe l'éditeur (problèmes : ${JSON.stringify(problems)})`, Array.isArray(problems) && problems.length === 0);
  let seen = null, after = null;
  for (const seed of SEEDS) {
    await ev("showMenu('main')").catch(() => {}); await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
    await ev(`document.querySelector('.vopt').open = true; $('sSeed').value='${seed}'; $('sFirst').value='0'; $('sLvl').value='0'`);
    await pg.click("#bStart");
    for (let i = 0; i < 3000; i++) {
      await waitFor("!!(V && !working)");
      if (await ev("V.winner !== null && V.winner !== undefined")) break;
      if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
      if (await ev("!!V.ask")) { await ev("(async()=>{ await answer(V.ask.kind === 'mulligan' ? [] : 0) })()"); continue; }
      if (await ev("!!V.dec")) {
        if (seen && !after && await ev(`V.st.t > ${seen.t}`)) {   // après le cleanup de fin de tour : toujours sur le battlefield
          after = await ev(`({ bf: V.st.bfs.map(b => (b.g || []).filter(x => x.u === ${seen.uid}).length), base: V.st.p[MEP].gear.some(x => x.u === ${seen.uid}),
            dom: (() => { const c = document.querySelector('#board .card[data-uid="${seen.uid}"]'); return c ? (c.closest('.bf') ? 'bf' : c.closest('.zbase') ? 'base' : 'autre') : 'absent'; })() })`);
          ok(`${w} px : au tour suivant, ${GEAR} est encore sur son battlefield (moteur ${JSON.stringify(after.bf)}, page : ${after.dom}) ou a été détruit par un adversaire qui y tient`,
            !after.base && (after.dom === "bf" || after.dom === "absent") && (after.dom === "absent") === (after.bf.every(n => n === 0)));
        }
        const opts = await ev(`V.dec.options.filter(x => x.k === 'play' && (x.label || '').includes(${JSON.stringify(GEAR)})).map(x => ({ i: x.i, loc: x.loc, from: x.from_ }))`);
        if (!seen && opts.length) {
          const locs = [...new Set(opts.map(o => String(o.loc)))].sort();
          ok(`${w} px : graine ${seed} : ${GEAR} ne se joue que sur un battlefield (destinations ${locs.join(",")})`, locs.length > 0 && !locs.includes("base"));
          const uid = await ev(`(() => { const c = [...document.querySelectorAll('.hand .card[data-card="${GEAR}"]')].find(c => c.dataset.uid); return c ? +c.dataset.uid : null; })()`);
          const dest = locs[locs.length - 1];
          const box = async sel => { const r = await pg.locator(sel).first().boundingBox(); return r && { x: r.x + r.width / 2, y: r.y + r.height / 2 }; };
          const from = await box(`.hand .card[data-uid="${uid}"]`), to = await box(`#board .bf[data-drop="${dest}"] .mid`);
          const t = await ev("V.st.t");
          if (mob) {           // toucher simulé : évènements pointer de type « touch »
            const send = (type, p) => ev(`(() => { const el = document.elementFromPoint(${p.x}, ${p.y}); el.dispatchEvent(new PointerEvent(${JSON.stringify(type)}, { bubbles: true, cancelable: true, pointerType: "touch", isPrimary: true, pointerId: 7, clientX: ${p.x}, clientY: ${p.y}, buttons: ${type === "pointerup" ? 0 : 1} })); })()`);
            await send("pointerdown", from);
            for (let k = 1; k <= 12; k++) await send("pointermove", { x: from.x + (to.x - from.x) * k / 12, y: from.y + (to.y - from.y) * k / 12 });
            await send("pointerup", to);
          } else {
            await pg.mouse.move(from.x, from.y); await pg.mouse.down();
            for (let k = 1; k <= 12; k++) await pg.mouse.move(from.x + (to.x - from.x) * k / 12, from.y + (to.y - from.y) * k / 12);
            await pg.mouse.up();
          }
          await pg.waitForTimeout(300);
          for (let k = 0; k < 200 && await ev(`!(V.st.bfs.some(b => (b.g || []).some(x => x.u === ${uid})) || V.st.p[MEP].gear.some(x => x.u === ${uid}))`); k++) {
            await waitFor("!!(V && !working)");
            if (await ev("!!V.ask")) await ev("(async()=>{ await answer(0) })()");
            else if (await ev("!V.dec")) await ev("(async()=>{ await pump(J(T.step())) })()");
            else if (await ev(`!!document.querySelector('.hand .card[data-uid="${uid}"]')`)) break;   // le glisser n'a rien joué
            else await pg.waitForTimeout(100);
          }
          const st = await ev(`(() => { const c = document.querySelector('#board .card[data-uid="${uid}"]'), bf = c && c.closest('.bf');
            return { eng: V.st.bfs.map(b => (b.g || []).map(x => x.n + "/" + x.c)), base: V.st.p[MEP].gear.map(x => x.n),
              bf: bf ? bf.dataset.drop : null, side: c && c.closest('.side') ? (c.closest('.side').classList.contains('top') ? 'haut' : 'bas') : null,
              inBase: !!(c && c.closest('.zbase')), sw: document.documentElement.scrollWidth, iw: innerWidth }; })()`);
          await pg.screenshot({ path: `${OUT}/deploy-${w}.png` });
          writeFileSync(`${OUT}/deploy-${w}.json`, JSON.stringify(st, null, 1));
          ok(`${w} px : glissé sur le battlefield ${dest}, ${GEAR} y est dans le moteur (${JSON.stringify(st.eng)})`, (st.eng[+dest] || []).includes(GEAR + "/" + (await ev("MEP"))));
          ok(`${w} px : la page le dessine sur ce battlefield, de ton côté (battlefield ${st.bf}, côté ${st.side}), pas dans la base`, st.bf === dest && st.side === "bas" && !st.inBase && !st.base.includes(GEAR));
          ok(`${w} px : pas de défilement horizontal (page ${st.sw} ≤ ${st.iw})`, st.sw <= st.iw);
          seen = { seed, t, uid };
          continue;
        }
        const pick = await ev(`(()=>{ const o = V.dec.options; return (o.find(x => x.k === 'end' || x.k === 'pass') || o[0]).i; })()`);
        await ev(`(async()=>{ await choose(${pick}) })()`); continue;
      }
      await ev("(async()=>{ await pump(J(T.step())) })()");
    }
    await waitFor("!!(V && !working)");
    if (seen) {
      const end = await ev("({ w: V.winner, pts: V.st.pts })");
      ok(`${w} px : la partie va au bout (fin ${end.pts.join("-")})`, end.w !== null && end.w !== undefined);
      ok(`${w} px : le tour suivant a été vu`, !!after);
      break;
    }
  }
  ok(`${w} px : ${GEAR} joué dans une des graines ${SEEDS.join(",")}`, !!seen);
  ok(`${w} px : aucune erreur JavaScript (${errs.length})`, errs.length === 0);
}
await b.close();
console.log(fails ? `ÉCHEC : ${fails} contrôle(s)` : "verif_deploy : tous les contrôles OK");
process.exit(fails ? 1 : 0);
