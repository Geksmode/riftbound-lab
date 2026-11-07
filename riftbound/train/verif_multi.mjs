import { existsSync } from "node:fs";
// Robot des questions « jusqu'à N unités » (Bilgewater Tracker : « deal 1 to each of up to 2 units ») : signalé par
// l'utilisateur le 2026-11-07 (capture : une longue liste de textes « Zhonya's Hourglass, Noxus Hopeful »…). Les unités
// possibles sont en surbrillance et dans la liste en images de la bulle ; on en touche deux, « Valider la cible ».
// Aussi : vignettes de battlefield du choix de la manche suivante (case de planche tournée comme sur le plateau).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_multi.mjs <dossier des captures> [port]
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



const SETUP = `
TR = H("Bilgewater Tracker")
us = []
for n in ("Soaring Scout", "Soaring Scout", "Noxus Hopeful"):
    u = Obj(n, op); g.enter_board(u, op, "base", ready=True); us.append(u.uid)
A, B, C = us
m = Obj("Soaring Scout", me); g.enter_board(m, me, "base", ready=True); M = m.uid`;
for (const phone of [true, false]) {
  if (!phone) await pg.setViewportSize({ width: 1440, height: 900 });
  await fresh(SETUP);
  const tr = String(await ev("pyGet('TR')")), A = String(await ev("pyGet('A')")), B = String(await ev("pyGet('B')")), C = String(await ev("pyGet('C')"));
  const tag = phone ? "téléphone" : "ordinateur";
  await tapSel(`#board [data-uid="${tr}"]`);
  await tapText("#pop button[data-i]", /base/i);
  for (let i = 0; i < 30 && !(await ev("!!(V.ask && V.ask.kind === 'tracker_targets')")); i++) {
    await waitFor("!!(V && !working)");
    if (await ev("!!V.ask")) await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()");
    else if (await ev("!!(V.dec && V.dec.options.some(o => o.k === 'pass'))")) await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'pass')))) })()");
    await pg.waitForTimeout(200);
  }
  await waitFor("!!(V && !working)");
  const bub = await ev("({ multi: !!(mode && mode.multi), text: [...document.querySelectorAll('#fbar .prow button')].map(b => b.textContent), strip: [...document.querySelectorAll('#fbar .pick-strip [data-pk]')].length, hl: [...document.querySelectorAll('#board .drop-ok')].map(x => x.dataset.uid) })");
  ok(`${tag} : Bilgewater Tracker demande ses cibles en touchant les cartes, pas en liste de combinaisons (${JSON.stringify(bub)})`,
    bub.multi && !bub.text.some(t => /,/.test(t)) && bub.text.some(t => /Aucune cible/.test(t)) && [A, B, C].every(u => bub.hl.includes(u)) && bub.strip >= 3);
  await pg.screenshot({ path: OUT + `/multi-cibles-${phone ? "tel" : "pc"}.png` });
  for (const u of [A, C]) {
    if (phone) await tapSel(`#fbar .pick-strip [data-pk="u:${u}"]`); else await tapSel(`#board [data-uid="${u}"]`);
  }
  const st = await ev("({ P: mode.P.slice(), ok: !document.getElementById('fOk').disabled })");
  ok(`${tag} : deux unités touchées, « Valider la cible » actif (${JSON.stringify(st)})`, st.ok && st.P.length === 2);
  await tapSel("#fOk");
  await settle();
  await py(`g = W["g"]; R = [(g.obj(u).damage if g.obj(u) is not None else "mort") for u in (${A}, ${B}, ${C})]`);
  const r = JSON.parse(await ev("pyGet('R').then(JSON.stringify)"));
  ok(`${tag} : 1 dégât sur les deux unités touchées, rien sur l'autre (${JSON.stringify(r)})`, r[0] !== 0 && r[2] !== 0 && r[1] === 0);
}
// Vignettes de battlefield (choix de la manche suivante) : une case de planche (Minefield…) est tournée comme sur le plateau
const th = await ev(`(() => { const out = {}; for (const n of ["Minefield", "Ruined Monastery", "Zaun Warrens"]) {
  const d = document.createElement("div"); d.style.width = "300px"; d.innerHTML = bfThumb(n); document.body.appendChild(d);
  const r = d.querySelector(".bfrot"), box = d.firstChild.getBoundingClientRect();
  const rr = r && r.getBoundingClientRect();
  out[n] = r ? { rot: getComputedStyle(r).transform, w: Math.round(rr.width), h: Math.round(rr.height), bw: Math.round(box.width), bh: Math.round(box.height) } : null;
  d.remove(); } return out; })()`);
ok(`vignettes de battlefield de planche tournées de +90° (à l'endroit, comme sur le plateau) et à la taille de la vignette (${JSON.stringify(th)})`,
  Object.values(th).every(t => t && /^matrix\(0, 1, -1, 0,/.test(t.rot) && Math.abs(t.w - t.bw) <= 3 && Math.abs(t.h - t.bh) <= 3));
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
await b.close();
process.exitCode = fails ? 1 : 0;
