import { existsSync } from "node:fs";
// Robot de Runeblade Anvil dans la vraie table : jouer l'enclume bannit le dessus du deck ; la pile « Bann. » apparaît
// à côté de la défausse (zone publique, 108.6.e) et s'ouvre au toucher ; la carte bannie s'affiche au bout de ta main
// (marquée BANNIE), se joue au toucher ou au clic (« Jouer … (bannie) ») en payant son coût. Ordinateur et téléphone.
// Situation posée dans le moteur (Pyodide).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_banish.mjs <dossier des captures> [port]
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
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev("document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'"); await pg.click("#bStart");
  for (let i = 0; i < 200; i++) { await waitFor("!!(V && !working)"); if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; } if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; } if (await ev("!!(V.dec && V.dec.kind === 'main')")) break; await ev("(async()=>{ await pump(J(T.act(0))) })()"); }
  await py(`
from game import Obj, Rune
g = W["g"]; me = ME
g.p[me].runes = [Rune("Fury", me) for _ in range(5)] + [Rune("Chaos", me) for _ in range(3)]
a = Obj("Runeblade Anvil", me); a.zone = "hand"; g.p[me].hand.append(a)
t = Obj("Shipyard Skulker", me); t.zone = "deck"; g.p[me].deck.insert(0, t)
AN, TOP = a.uid, t.uid
W["d"] = None`);
  await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(300);
  const an = String(await ev("pyGet('AN')")), top = String(await ev("pyGet('TOP')"));
  const tap = async sel => { const el = pg.locator(sel).first();
    if (mob) { const bb = await el.boundingBox(); await pg.touchscreen.tap(bb.x + bb.width - 6, bb.y + bb.height / 2); await pg.waitForTimeout(250);
      if (await ev("document.querySelector('#board .half.p0 .hand').classList.contains('open')")) { const b2 = await el.boundingBox(); await pg.touchscreen.tap(b2.x + b2.width / 2, b2.y + b2.height / 2); } }
    else await el.click();
    await pg.waitForTimeout(300); };
  const playVia = async re => { await ev(`(()=>{ const b = [...document.querySelectorAll('#pop button[data-i]')].find(b => ${re}.test(b.textContent)); if (b) b.click(); })()`);
    await pg.waitForTimeout(500); await waitFor("!!(V && !working)");
    for (let i = 0; i < 10; i++) { if (await ev("!!(V.dec && V.dec.kind !== 'main' && V.dec.options.some(o => o.k === 'pass'))")) { await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'pass')))) })()"); await waitFor("!!(V && !working)"); } else break; } };
  ok(`${w} px : pas de pile de bannies tant qu'elle est vide`, await ev("!document.querySelector('#board .half.p0 .pl.ban:not(.empty)')"));
  await tap(`#board [data-uid="${an}"]`);
  await playVia("/Jouer/");
  const names = await ev("[...document.querySelectorAll('#board .half.p0 .hand .card.ban')].map(c => c.dataset.card)");
  ok(`${w} px : la carte bannie par l'enclume s'affiche au bout de la main (${JSON.stringify(names)})`, names.length === 1 && names[0] === "Shipyard Skulker");
  ok(`${w} px : pile « Bann. » à 1 carte`, await ev("(()=>{ const p = document.querySelector('#board .half.p0 .pl.ban:not(.empty) span'); return !!p && p.textContent === '1'; })()"));
  await pg.screenshot({ path: `${OUT}/banish-pile-${w}.png` });
  ok(`${w} px : la pile reste dans sa colonne (pas de débordement)`, await ev("(()=>{ const p = document.querySelector('#board .half.p0 .pl.ban'), z = p.closest('.zone').getBoundingClientRect(), r = p.getBoundingClientRect(); return r.left >= z.left - 1 && r.right <= z.right + 8; })()"));
  await tap("#board .half.p0 [data-ban]");
  const modal = await ev("$('modal').hidden ? '' : $('mbox').innerText");
  ok(`${w} px : la pile s'ouvre et marque la carte jouable (${JSON.stringify(modal.slice(0, 80).replace(/\n/g, " | "))})`, /bannies/.test(modal) && /JOUABLE/.test(modal));
  await ev("closeModal()");
  await pg.waitForTimeout(200);
  await tap(`#board [data-uid="${top}"]`);
  const pop = await ev("$('pop').hidden ? '' : $('pop').innerText");
  ok(`${w} px : le menu propose de la jouer depuis les bannies (${JSON.stringify(pop.replace(/\n/g, " | "))})`, /bannie/.test(pop));
  await playVia("/bannie/");
  await py(`Z = [o.zone for o in [W["g"].obj(TOP)] if o is not None] + [sum(1 for r in W["g"].p[ME].runes if not r.exhausted)]`);
  const z = JSON.parse(await ev("pyGet('Z').then(JSON.stringify)"));
  ok(`${w} px : Shipyard Skulker jouée depuis les bannies, coût payé (enclume 1 + unité 3 : ${z[z.length - 1]} runes prêtes sur 8)`, z[0] === "board" && z[z.length - 1] === 4);
  ok(`${w} px : plus de carte BANNIE dans la main`, await ev("!document.querySelector('#board .half.p0 .hand .card.ban')"));
  ok(`${w} px : aucune erreur JavaScript` + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
  await pg.screenshot({ path: `${OUT}/banish-${w}.png` });
}
await b.close();
process.exitCode = fails ? 1 : 0;
