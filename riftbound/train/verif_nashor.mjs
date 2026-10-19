import { existsSync, writeFileSync } from "node:fs";
// Robot de Baron Nashor dans la vraie table : un deck perso (Kha'Zix + 3 Baron Nashor) contre l'IA débutante ; on joue
// Nashor dès qu'il est jouable (sinon on passe). Contrôles : le jeton Baron Pit apparaît comme 3e battlefield, Nashor y
// entre, les trois battlefields tiennent à l'écran (pas de défilement horizontal) ; puis la partie continue jusqu'au
// bout avec trois battlefields. Ordinateur (1400x900) et téléphone (393x851). Les coups passent par les fonctions de
// la page (choose / answer), pas par des touchers simulés. Demande du 2026-10-19.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_nashor.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
// Kha'Zix, Voidreaver (liste khazix_cuoemrei_rq-los-angeles_20th) : les 3 cartes les plus chères remplacées par Baron Nashor
const DECK = { id: "nashor-test", name: "Nashor test", legend: "Kha'Zix, Voidreaver", champion: "Kha'Zix, Mutating Horror",
  main: null, sideboard: {}, runes: null, battlefields: null };
const SEEDS = (process.env.RB_NASHOR_SEEDS || "8,4,5,6,7,9").split(",").map(Number);
for (const [w, h, mob] of [[1400, 900, false], [393, 851, true]]) {
  const pg = await (await b.newContext({ viewport: { width: w, height: h }, isMobile: mob, hasTouch: mob })).newPage();
  const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 240000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  if (!DECK.main) {          // le deck de base vient du catalogue de la page (mêmes listes que decks.json)
    const base = await ev("JSON.stringify(CAT.presets.find(p => p.key === 'khazix_cuoemrei_rq-los-angeles_20th').deck)");
    const d = JSON.parse(base);
    const CAT_E = await ev("Object.fromEntries(CAT.cards.map(c => [c.n, +c.e || 0]))");
    const main = d.main.slice(), idx = [...main.keys()].sort((a, c) => (CAT_E[main[c]] || 0) - (CAT_E[main[a]] || 0) || a - c);
    for (const i of idx.slice(0, 3)) main[i] = "Baron Nashor";
    DECK.main = {}; for (const n of main) DECK.main[n] = (DECK.main[n] || 0) + 1;
    DECK.runes = {}; for (const r of d.runes) { const k = r.replace(/ Rune$/, ""); DECK.runes[k] = (DECK.runes[k] || 0) + 1; }
    DECK.battlefields = d.battlefields.slice(0, 3);
  }
  await ev(`localStorage.setItem("rbt-decks", ${JSON.stringify(JSON.stringify({ "nashor-test": DECK }))}); localStorage.setItem("rbt-mine", "my:nashor-test")`);
  await pg.reload();
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  const problems = await ev("(async()=>(await J(T.check(JSON.stringify(toEngine(MYDECKS['nashor-test']))))).filter(e => !e.startsWith('⚠')))()").catch(e => String(e));
  ok(`${w} px : le deck avec 3 Baron Nashor passe l'éditeur (problèmes : ${JSON.stringify(problems)})`, Array.isArray(problems) && problems.length === 0);
  let seen = null;
  for (const seed of SEEDS) {
    await ev("showMenu('main')").catch(() => {}); await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
    await ev(`document.querySelector('.vopt').open = true; $('sSeed').value='${seed}'; $('sFirst').value='0'; $('sLvl').value='0'`);
    ok(`${w} px : graine ${seed}, ton deck = Nashor test`, await ev("$('sMine').value") === "my:nashor-test");
    await pg.click("#bStart");
    for (let i = 0; i < 3000; i++) {
      await waitFor("!!(V && !working)");
      if (await ev("V.winner !== null && V.winner !== undefined")) break;
      if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
      if (await ev("!!V.ask")) { await ev("(async()=>{ await answer(V.ask.kind === 'mulligan' ? [] : 0) })()"); continue; }
      if (await ev("!!V.dec")) {
        if (!seen && await ev("V.st.bfs.length === 3")) {
          seen = { seed, t: await ev("V.st.t") };
          const st = await ev(`(()=>{ const bfs = [...document.querySelectorAll('#board .bfrow .bf')], row = document.querySelector('#board .bfrow').getBoundingClientRect();
            const nash = [...document.querySelectorAll('#board .bf .card[data-card="Baron Nashor"]')].map(c => c.closest('.bf') && [...document.querySelectorAll('#board .bfrow .bf')].indexOf(c.closest('.bf')));
            return { n: bfs.length, names: bfs.map(x => x.querySelector('.bname').textContent), w: bfs.map(x => Math.round(x.getBoundingClientRect().width)),
              right: Math.max(...bfs.map(x => x.getBoundingClientRect().right)), rowR: row.right, sw: document.documentElement.scrollWidth, iw: innerWidth,
              nash, drop: bfs.map(x => x.dataset.drop), eng: V.st.bfs.map(b => b.n),
              tags: [...document.querySelectorAll('#board .bf .sdtag')].map(t => { const r = t.getBoundingClientRect(), m = t.closest('.mid').getBoundingClientRect(); return [Math.round(r.right), Math.round(m.right), t.scrollWidth <= t.clientWidth + 1]; }) }; })()`);
          await pg.screenshot({ path: `${OUT}/nashor-${w}.png` });
          writeFileSync(`${OUT}/nashor-${w}.json`, JSON.stringify(st, null, 1));
          ok(`${w} px : graine ${seed} tour ${seen.t} : 3 battlefields affichés (${st.names.join(" | ")}), le 3e est le Baron Pit`, st.n === 3 && st.names[2] === "Baron Pit" && st.eng[2] === "Baron Pit" && st.drop.join() === "0,1,2");
          ok(`${w} px : l'étiquette Showdown / Combat n'est pas coupée (${JSON.stringify(st.tags)})`, st.tags.every(([r, m, full]) => r <= m && full));
          ok(`${w} px : Baron Nashor est sur le Baron Pit (battlefield ${JSON.stringify(st.nash)})`, st.nash.includes(2));
          ok(`${w} px : les trois tiennent à l'écran, sans défilement horizontal (largeurs ${st.w.join("/")}, bord droit ${Math.round(st.right)} ≤ ${st.iw}, page ${st.sw})`,
            st.sw <= st.iw && st.right <= st.iw + 1 && Math.min(...st.w) >= (mob ? 100 : 300));
        }
        const pick = await ev(`(()=>{ const o = V.dec.options; const n = o.find(x => x.k === 'play' && /Baron Nashor/.test(x.label || ''));
          return (n || o.find(x => x.k === 'end' || x.k === 'pass') || o[0]).i; })()`);
        await ev(`(async()=>{ await choose(${pick}) })()`); continue;
      }
      await ev("(async()=>{ await pump(J(T.step())) })()");
    }
    await waitFor("!!(V && !working)");
    if (seen) {
      const end = await ev("({ w: V.winner, pts: V.st.pts, n: V.st.bfs.length })");
      ok(`${w} px : la partie va au bout avec 3 battlefields (fin ${end.pts.join("-")}, battlefields ${end.n})`, end.w !== null && end.w !== undefined && end.n === 3);
      break;
    }
  }
  ok(`${w} px : Baron Nashor joué dans une des graines ${SEEDS.join(",")}`, !!seen);
  ok(`${w} px : aucune erreur JavaScript (${errs.length})`, errs.length === 0);
}
await b.close();
console.log(fails ? `ÉCHEC : ${fails} contrôle(s)` : "verif_nashor : tous les contrôles OK");
process.exit(fails ? 1 : 0);
