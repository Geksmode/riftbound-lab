import { existsSync } from "node:fs";
// Robot du [Repeat] dans la vraie table (retour utilisateur 2026-11-04 : « le simulateur ne propose pas de repeat le
// sort »). Blood Rush ([Repeat] 1 énergie, « Give a unit [Assault 2] ») avec le Repeat donné par The Academy (règles
// 820.1.c.2, 820.3 : chaque Repeat se paie séparément). Avant le correctif, la table fusionnait les variantes au Repeat
// accordé avec le sort normal et ne montrait pas la cible de la répétition ; le moteur ne proposait pas les deux Repeat
// ensemble. On cible Lonely Poro, la liste « comment le jouer ? » doit offrir Sans Repeat / Avec Repeat → cible /
// Avec Repeat ×2, puis on joue ×2 (répétitions : Scuttle Crab puis Lonely Poro) : Poro Assault 4, Crab Assault 2.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_repeat.mjs <dossier des captures> [port]
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
  const py = code => ev(`pyExec(${JSON.stringify(code)})`);
  const tap = async sel => { const bb = await pg.locator(sel).first().boundingBox(); if (mob) await pg.touchscreen.tap(bb.x + bb.width / 2, bb.y + bb.height / 2); else await pg.mouse.click(bb.x + bb.width / 2, bb.y + bb.height / 2); await pg.waitForTimeout(300); };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev("document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'"); await pg.click("#bStart");
  for (let i = 0; i < 200; i++) { await waitFor("!!(V && !working)"); if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; } if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; } if (await ev("!!(V.dec && V.dec.kind === 'main')")) break; await ev("(async()=>{ await pump(J(T.act(0))) })()"); }
  await py(`
from game import Obj, Rune
from cardsets.battlefields import _academy_cost
g = W["g"]; me = ME
g.p[me].runes = [Rune("Fury", me) for _ in range(6)]
for o in [o for o in g.board if o.loc == "base"]:
    g.board.remove(o)
p = Obj("Lonely Poro", me); g.enter_board(p, me, "base", ready=True)
c = Obj("Scuttle Crab", me); g.enter_board(c, me, "base", ready=True)
br = Obj("Blood Rush", me); br.zone = "hand"; g.p[me].hand.append(br)
g.effects.append(dict(kind="grant_repeat", pid=me, cost=_academy_cost, next=True, key="academy-robot", dur="turn"))
BR, PO, CR = br.uid, p.uid, c.uid
W["d"] = None`);
  await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(300);
  const [br, po] = [String(await ev("pyGet('BR')")), String(await ev("pyGet('PO')"))];
  if (mob) { const el = pg.locator(`#board [data-uid="${br}"]`), bb = await el.boundingBox(); await pg.touchscreen.tap(bb.x + bb.width - 6, bb.y + bb.height / 2); await pg.waitForTimeout(250);
    if (await ev("document.querySelector('#board .half.p0 .hand').classList.contains('open')")) { const b2 = await el.boundingBox(); await pg.touchscreen.tap(b2.x + b2.width / 2, b2.y + b2.height / 2); await pg.waitForTimeout(250); } }
  else await tap(`#board [data-uid="${br}"]`);
  await tap("#pop button[data-tg]");                              // « Cibler »
  if (mob) await tap(`#fbar .pick-strip [data-pk="u:${po}"]`);     // téléphone : la cible dans la bulle
  else await tap(`#board [data-uid="${po}"]`);
  await tap("#fOk");                                               // « Valider la cible »
  const list = await ev("[...document.querySelectorAll('#fbar button[data-i]')].map(b => b.textContent)");
  ok(`${w} px : après la cible, Sans Repeat / Avec Repeat → cible / Avec Repeat ×2 (${list.length} lignes : ${JSON.stringify(list.slice(0, 4))}…)`,
    list.some(t => t === "Sans Repeat") && list.some(t => /^Avec Repeat \(coût total .*\) → Scuttle Crab$/.test(t))
    && list.some(t => /^Avec Repeat ×2 \(coût total 3 énergie\) → Scuttle Crab et Lonely Poro$/.test(t)));
  ok(`${w} px : aucune ligne en double`, new Set(list).size === list.length);
  await ev("(()=>{ const b = [...document.querySelectorAll('#fbar button[data-i]')].find(b => b.textContent === 'Avec Repeat ×2 (coût total 3 énergie) → Scuttle Crab et Lonely Poro'); if (b) b.click(); })()");
  await pg.waitForTimeout(500); await waitFor("!!(V && !working)");
  for (let i = 0; i < 12; i++) {
    if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); await waitFor("!!(V && !working)"); continue; }
    if (await ev("!!(V.dec && V.dec.kind !== 'main' && V.dec.options.some(o => o.k === 'pass'))")) { await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'pass')))) })()"); await waitFor("!!(V && !working)"); continue; }
    break;
  }
  await py(`gg = W["g"]; Z = [gg.kw_value(gg.obj(PO), "Assault"), gg.kw_value(gg.obj(CR), "Assault"), sum(1 for r in gg.p[ME].runes if not r.exhausted)]`);
  const z = JSON.parse(await ev("pyGet('Z').then(JSON.stringify)"));
  ok(`${w} px : joué deux fois répété : Poro Assault 4, Crab Assault 2, 3 runes restantes (${JSON.stringify(z)})`, z[0] === 4 && z[1] === 2 && z[2] === 3);
  ok(`${w} px : aucune erreur JavaScript` + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
  await pg.screenshot({ path: `${OUT}/repeat-${w}.png` });
}
await b.close();
process.exitCode = fails ? 1 : 0;
