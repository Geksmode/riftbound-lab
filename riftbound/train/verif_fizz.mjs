import { existsSync } from "node:fs";
// Robot de Fizz, Trickster (« play a spell from your trash… ») : demande de l'utilisateur du 2026-11-04 (capture : une
// longue liste de textes « Switcheroo (sans payer l'énergie ; cible X, Y) », deux fois). La bulle montre maintenant
// l'image de chaque sort jouable de la défausse (une fois par nom), puis on touche ses cibles sur le plateau et on valide.
// Au format téléphone (cibles dans la bulle) puis ordinateur (cibles touchées sur le plateau).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_fizz.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
const pg = await (await b.newContext({ viewport: { width: 393, height: 851 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 })).newPage();
const errs = []; pg.on("pageerror", e => errs.push(String(e)));
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
const ev = s => pg.evaluate(s);
const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
const py = code => ev(`pyExec(${JSON.stringify(code)})`);
const tap = async loc => { const r = await loc.boundingBox(); await pg.touchscreen.tap(r.x + r.width / 2, r.y + r.height / 2); await pg.waitForTimeout(250); };
const tapSel = sel => tap(pg.locator(sel).first());
const tapText = (sel, re) => tap(pg.locator(sel).filter({ hasText: re }).first());
const settle = async () => {   // répond 0 aux questions et passe jusqu'au retour en phase principale
  await pg.waitForTimeout(400); await waitFor("!!(V && !working)");
  for (let i = 0; i < 30; i++) {
    if (await ev("!!V.ask")) await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()");
    else if (await ev("!!(V.dec && V.dec.kind !== 'main' && V.dec.options.some(o => o.k === 'pass'))")) await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'pass')))) })()");
    else break;
    await waitFor("!!(V && !working)");
  }
};
const fresh = async code => {   // nouvelle partie (graine 4), situation posée dans le moteur (Pyodide)
  await ev("showMenu('main')").catch(() => {});
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev("document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'"); await pg.click("#bStart");
  for (let i = 0; i < 200; i++) {
    await waitFor("!!(V && !working)");
    if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
    if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; }
    if (await ev("!!(V.dec && V.dec.kind === 'main')")) break;
    await ev("(async()=>{ await pump(J(T.act(0))) })()");
  }
  await py(`
from game import Obj, Rune, DOMAINS
g = W["g"]; me = ME; op = 1 - ME
g.p[me].runes = [Rune(d, me) for d in DOMAINS for _ in range(3)]
def H(n):
    c = Obj(n, me); c.zone = "hand"; g.p[me].hand.append(c); return c.uid
${code}
W["d"] = None
`);
  await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(300);
};
await pg.goto(`http://localhost:${PORT}/train.html`);
await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");


const BUBBLE = "({ cards: [...document.querySelectorAll('#fbar .mull.pick button[data-pp] .card')].map(c => c.dataset.card), text: [...document.querySelectorAll('#fbar .prow button')].map(b => b.textContent), long: [...document.querySelectorAll('#fbar button')].filter(b => /cible/.test(b.textContent)).length })";
const SETUP = `
FZ = H("Fizz, Trickster")
for _ in range(2):
    s = Obj("Switcheroo", me); s.zone = "trash"; g.p[me].trash.append(s)
a = Obj("Soaring Scout", me); g.enter_board(a, me, 1, ready=True); A = a.uid
b = Obj("Soaring Scout", me)
g.enter_board(b, me, 1, ready=True); B = b.uid
g.mod(b, 2)
g.bfs[1].ctrl = me`;
for (const phone of [true, false]) {
  if (!phone) await pg.setViewportSize({ width: 1440, height: 900 });
  await fresh(SETUP);
  const fz = String(await ev("pyGet('FZ')")), A = String(await ev("pyGet('A')")), B = String(await ev("pyGet('B')"));
  const tag = phone ? "téléphone" : "ordinateur";
  await tapSel(`#board [data-uid="${fz}"]`);
  await tapText("#pop button[data-i]", /base/i);
  for (let i = 0; i < 30 && !(await ev("!!(V.ask && V.ask.kind === 'fizz_pick')")); i++) {
    await waitFor("!!(V && !working)");
    if (await ev("!!V.ask")) await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()");
    else if (await ev("!!(V.dec && V.dec.options.some(o => o.k === 'pass'))")) await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'pass')))) })()");
    await pg.waitForTimeout(200);
  }
  const bub = await ev(BUBBLE);
  ok(`${tag} : Fizz demande le sort en images, Switcheroo une seule fois, aucun texte « cible … » (${JSON.stringify(bub)})`,
    bub.cards.length === 1 && bub.cards[0] === "Switcheroo" && bub.long === 0 && bub.text.some(t => /Ne rien jouer/.test(t)));
  await pg.screenshot({ path: OUT + `/fizz-sorts-${phone ? "tel" : "pc"}.png` });
  await tapSel('#fbar button[data-pp="Switcheroo"]');
  const hl = await ev("[...document.querySelectorAll('#board .drop-ok')].map(x => x.dataset.uid)");
  ok(`${tag} : Switcheroo touché : les deux Scouts du battlefield sont en surbrillance (${JSON.stringify(hl)})`, hl.includes(A) && hl.includes(B));
  for (const u of [A, B]) {
    const pk = await ev(`!!document.querySelector('#fbar .pick-strip [data-pk="u:${u}"]')`);
    if (phone && pk) await tapSel(`#fbar .pick-strip [data-pk="u:${u}"]`); else await tapSel(`#board [data-uid="${u}"]`);
  }
  await pg.screenshot({ path: OUT + `/fizz-cibles-${phone ? "tel" : "pc"}.png` });
  const okEn = await ev("!document.getElementById('fOk').disabled");
  ok(`${tag} : deux cibles touchées, « Valider la cible » actif`, okEn);
  await tapSel("#fOk");
  await settle();
  await py(`R = [W["g"].might(W["g"].obj(${A})), W["g"].might(W["g"].obj(${B})), any(c.cname == "Switcheroo" for c in W["g"].p[ME].deck)]`);
  const r = JSON.parse(await ev("pyGet('R').then(JSON.stringify)"));
  ok(`${tag} : Switcheroo rejoué sur les deux Scouts (Might échangées) et recyclé dans le deck (${JSON.stringify(r)})`, r[0] > r[1] && r[2]);
}
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
await b.close();
process.exitCode = fails ? 1 : 0;
