import { existsSync } from "node:fs";
// Robot des puzzles (menu « Puzzles », engine/puzzles.py) : liste, énoncé, partie dans le vrai moteur contre l'IA de la
// table, objectif jugé par le moteur, écran de fin (réussi / raté, solution, leçon), Réessayer, Puzzle suivant, liste
// avec les puzzles réussis, et la liste en anglais.
// Téléphone 393×851 (toucher simulé) : « Tuer avant le combat » réussi (Shuriken Flip touché dans la main, puis
// l'option du coup), « Compter son défenseur » raté (piège : Kai'Sa seule), solution montrée, Réessayer.
// Ordinateur 1400×900 (souris) : « Il attaque ton Héron », au tour de Yi ; Block révélé en réponse à Punch First.
// Les coups qui ne se touchent pas directement (réponses de paiement, passer) sont injectés comme dans les autres robots.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_puzzle.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
const errs = [];

async function open(ctxOpts, query = "") {
  const pg = await (await b.newContext(ctxOpts)).newPage();
  pg.on("pageerror", e => errs.push(String(e)));
  await pg.goto(`http://localhost:${PORT}/train.html${query}`);
  return pg;
}
function kit(pg, touch) {
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
  const tap = async loc => {
    if (!touch) { await loc.click(); await pg.waitForTimeout(250); return; }
    await loc.scrollIntoViewIfNeeded(); const r = await loc.boundingBox(); await pg.touchscreen.tap(r.x + r.width / 2, r.y + r.height / 2); await pg.waitForTimeout(250);
  };
  const tapSel = sel => tap(pg.locator(sel).first());
  const tapText = (sel, re) => tap(pg.locator(sel).filter({ hasText: re }).first());
  // répond 0 aux questions (paiement) et passe tant que ce n'est pas la phase principale (ou la fin) ; stop(V) arrête avant
  const settle = async (stop = "false") => {
    await pg.waitForTimeout(300); await waitFor("!!(V && !working)");
    for (let i = 0; i < 60; i++) {
      if (await ev(`V.winner !== null && V.winner !== undefined`)) break;
      if (await ev(stop)) break;
      if (await ev("!!V.ask")) await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()");
      else if (await ev("!!(V.dec && V.dec.kind !== 'main' && V.dec.options.some(o => o.k === 'pass'))")) await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'pass')))) })()");
      else if (await ev("!!V.busy")) await ev("(async()=>{ await pump(J(T.step())) })()");
      else break;
      await waitFor("!!(V && !working)");
    }
  };
  // indice de l'option du moteur choisie par une expression Python sur a (action brute) et g
  const idx = async expr => Number(await ev(`pyExec(${JSON.stringify(`
from replay import _find
g = W["g"]
def nm(u):
    o = g.obj(u)
    return o.name if o is not None else None
def cn(u):
    o = _find(g, ME, u)
    return o.name if o is not None else None
IDX = next(i for i, a in enumerate(W["d"].options) if (${expr}))
`)}).then(() => pyGet('IDX'))`));
  return { ev, waitFor, tap, tapSel, tapText, settle, idx };
}

// ------------------------------------------------------------------ téléphone, toucher simulé
{
  const pg = await open({ viewport: { width: 393, height: 851 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 });
  const { ev, waitFor, tapSel, tapText, settle, idx } = kit(pg, true);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=puzzles]');return b&&!b.disabled})()");
  await tapSel("#menu .mitem[data-m=puzzles]");
  await waitFor("document.querySelectorAll('#menu [data-pz]').length > 0");
  const list = await ev("[...document.querySelectorAll('#menu [data-pz]')].map(b => b.dataset.pz)");
  ok(`téléphone : la liste montre les 6 puzzles (${list.join(", ")})`, list.join() === "flip,count,heron,block,runner,decree");
  await pg.screenshot({ path: `${OUT}/puzzle-liste-tel.png` });

  // 1. « Tuer avant le combat » : réussi
  await tapSel("#menu [data-pz=flip]");
  await waitFor("!!document.getElementById('pzBrief')");
  const brief = await ev("document.getElementById('pzBrief').textContent");
  ok("téléphone : énoncé et objectif affichés avant de jouer", /Tuer avant le combat/.test(brief) && /Objectif/.test(brief) && /Grove of the God-Willow/.test(brief));
  await pg.screenshot({ path: `${OUT}/puzzle-enonce-tel.png` });
  await tapSel("#pzGo");
  await waitFor("!!(V && !working && V.dec && V.dec.kind === 'main')");
  const pos = await ev("({ opp: V.st.p[1].units ? null : null, names: [...document.querySelectorAll('#board .card')].map(c => c.dataset.card) })");
  ok("téléphone : la position est sur le plateau (Pit Rookie, Kai'Sa, Mournful Witness, Shuriken Flip en main)",
    ["Pit Rookie", "Kai'Sa, Survivor", "Mournful Witness", "Shuriken Flip"].every(n => pos.names.includes(n)));
  ok("téléphone : pas de bouton Conseil pendant un puzzle, puce Objectif visible",
    await ev("document.getElementById('bHint').hidden && !document.getElementById('gpuz').hidden"));
  const fl = await idx(`a[0] == "play" and cn(a[1]) == "Shuriken Flip" and [nm(u) for u in a[3].get("tg", ())] == ["Pit Rookie"] and a[3].get("dest") == 1 and nm(a[3]["mover"]) == "Kai'Sa, Survivor"`);
  const flipUid = await ev("String(V.st.p[0].hand.find(c => c[1] === 'Shuriken Flip')[0])");
  // au téléphone, le ciblage se fait dans la bulle : ennemi, allié à déplacer, destination, puis Valider
  await tapSel(`#board [data-uid="${flipUid}"]`);
  await tapText("#pop button", /Cibler/);
  const step = async (re, want) => { await tapText("#fbar button", re); return ev(`/${want}/.test(document.getElementById('fbar').innerText)`); };
  const s1 = await step(/Pit Rookie/, "Ennemi :\\s*Pit Rookie");
  const s2 = await step(/Kai'Sa/, "Allié à déplacer :\\s*Kai'Sa");
  await tapText("#fbar button", /Grove/);
  ok("téléphone : Shuriken Flip ciblé dans la bulle (Pit Rookie, puis Kai'Sa, puis Grove)", s1 && s2);
  await tapText("#fbar button", /^Valider$/);
  await settle();
  ok(`téléphone : Shuriken Flip joué (journal : ${(await ev("LOG.map(l => l[1]).filter(t => /Shuriken/.test(t)).join(' | ')"))})`, await ev(`LOG.some(l => /Shuriken Flip/.test(l[1]))`));
  ok("téléphone : Shuriken Flip tue le Pit Rookie et Kai'Sa prend Grove", await ev("V.st.bfs[1].c === 0"));
  await tapSel("#bMain");                                   // Terminer le tour : le moteur juge l'objectif
  await settle();
  await waitFor("!!document.getElementById('pzEnd')", 60000);
  const end1 = await ev("({ t: document.getElementById('pzEnd').textContent, sol: !document.getElementById('pzSol').hidden, res: document.getElementById('gres').textContent, puz: V.puz, done: localStorage.getItem('rbt-puz') })");
  ok(`téléphone : fin « Puzzle réussi ! », solution et leçon montrées (${JSON.stringify(end1.puz)})`,
    /Puzzle réussi/.test(end1.t) && end1.sol && /Solution/.test(end1.t) && /Pourquoi/.test(end1.t) && end1.puz.ok === true && /réussi/.test(end1.res));
  ok("téléphone : puzzle réussi gardé dans le navigateur", /"flip":1/.test(end1.done || ""));
  await pg.screenshot({ path: `${OUT}/puzzle-reussi-tel.png` });

  // 2. « Puzzle suivant » : « Compter son défenseur », piège (Kai'Sa seule) : raté
  await tapSel("#pzNext");
  await waitFor("!!document.getElementById('pzBrief') && /Compter son défenseur/.test(document.getElementById('pzBrief').textContent)");
  ok("téléphone : « Puzzle suivant » ouvre « Compter son défenseur »", true);
  await tapSel("#pzGo");
  await waitFor("!!(V && !working && V.dec && V.dec.kind === 'main')");
  const mv = await idx(`a[0] == "move" and a[2] == 1 and [nm(u) for u in a[1]] == ["Kai'Sa, Survivor"]`);
  await ev(`(async()=>{ await pump(J(T.act(${mv}))) })()`);
  await settle("!!(V.dec && V.dec.kind === 'main')");
  await tapSel("#bMain");
  await settle();
  await waitFor("!!document.getElementById('pzEnd')", 60000);
  const end2 = await ev("({ t: document.getElementById('pzEnd').textContent, sol: !document.getElementById('pzSol').hidden, ok: V.puz.ok })");
  ok("téléphone : piège « Kai'Sa seule » : « Puzzle raté », solution cachée", /Puzzle raté/.test(end2.t) && !end2.sol && end2.ok === false);
  await tapText("#pzEnd button", /Voir la solution/);
  ok("téléphone : « Voir la solution » montre la solution et la leçon", await ev("!document.getElementById('pzSol').hidden && /Discipline/.test(document.getElementById('pzSol').textContent)"));
  await pg.screenshot({ path: `${OUT}/puzzle-rate-tel.png` });
  await tapSel("#pzAgain");
  await waitFor("!!document.getElementById('pzBrief')");
  await tapSel("#pzGo");
  await waitFor("!!(V && !working && V.dec && V.dec.kind === 'main')");
  ok("téléphone : « Réessayer » remet la position de départ (First Mate sur Grove, Kai'Sa en base)",
    await ev("V.st.bfs[1].c === 1 && V.winner === null && V.st.t === 6"));
  // 3. liste : le puzzle réussi est coché
  await ev("showMenu('puzzles')");
  await waitFor("document.querySelectorAll('#menu [data-pz]').length > 0");
  const marks = await ev("[...document.querySelectorAll('#menu .mcard .t')].map(t => t.textContent)");
  ok(`téléphone : la liste coche le puzzle réussi (${marks.slice(0, 2).join(" | ")})`, /✓/.test(marks[0]) && !/✓/.test(marks[1]));
  await pg.context().close();
}

// ------------------------------------------------------------------ ordinateur, souris : au tour de Yi
{
  const pg = await open({ viewport: { width: 1400, height: 900 } });
  const { ev, waitFor, tapSel, settle, idx } = kit(pg, false);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=puzzles]');return b&&!b.disabled})()");
  await tapSel("#menu .mitem[data-m=puzzles]");
  await waitFor("!!document.querySelector('#menu [data-pz=block]')");
  await tapSel("#menu [data-pz=block]");
  await waitFor("!!document.getElementById('pzBrief')");
  await tapSel("#pzGo");
  // Yi attaque Sigil avec Master Yi, puis joue Punch First : on attend la priorité avec Punch First sur la chaîne
  await waitFor("!!(V && !working && V.dec && V.st.chain.some(c => c.n === 'Punch First'))", 120000);
  ok("ordinateur : Yi attaque Sigil et joue Punch First (chaîne)", await ev("V.st.chain.some(c => c.n === 'Punch First')"));
  const bl = await idx(`a[0] == "play" and a[2] == "facedown" and [nm(u) for u in a[3].get("tg", ())] == ["Astral Heron"]`);
  await ev(`(async()=>{ await pump(J(T.act(${bl}))) })()`);
  await settle();
  await waitFor("!!document.getElementById('pzEnd')", 120000);
  const end = await ev("({ t: document.getElementById('pzEnd').textContent, ok: V.puz.ok })");
  ok("ordinateur : Block révélé sur le Héron : « Puzzle réussi ! » à la fin du tour de Yi", /Puzzle réussi/.test(end.t) && end.ok === true);
  await pg.screenshot({ path: `${OUT}/puzzle-block-pc.png` });
  await pg.context().close();
}

// ------------------------------------------------------------------ anglais
{
  const pg = await open({ viewport: { width: 1400, height: 900 } }, "?lang=en");
  const { ev, waitFor, tapSel } = kit(pg, false);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=puzzles]');return b&&!b.disabled})()");
  await tapSel("#menu .mitem[data-m=puzzles]");
  await waitFor("document.querySelectorAll('#menu [data-pz]').length > 0");
  const t = await ev("document.getElementById('menu').textContent");
  ok("anglais : la liste des puzzles est en anglais", /Kill before combat/.test(t) && /one position, one goal/.test(t) && !/Tuer avant|une position/.test(t));
  await pg.context().close();
}

ok(`aucune erreur de page (${errs.slice(0, 2).join(" | ")})`, errs.length === 0);
await b.close();
console.log(fails ? `${fails} ÉCHEC(S)` : "puzzles : tout est OK");
process.exit(fails ? 1 : 0);
