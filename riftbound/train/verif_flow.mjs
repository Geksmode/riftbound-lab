import { existsSync } from "node:fs";
// Robot de Flow (règle 829) dans la vraie table : un sort de ta défausse jouable pour son coût de Flow s'affiche au bout
// de ta main (marqué FLOW), se joue au toucher ou au clic (« Jouer … (Flow) »), coûte son coût de Flow, puis est banni.
// Onslaught sans cible ne doit pas apparaître. Ordinateur et téléphone. Situation posée dans le moteur (Pyodide).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_flow.mjs <dossier des captures> [port]
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
  const py = code => ev(`(()=>{ T.__builtins__.get('exec')(${JSON.stringify(code)}, T.__dict__); return true })()`);
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev("document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'"); await pg.click("#bStart");
  for (let i = 0; i < 200; i++) { await waitFor("!!(V && !working)"); if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; } if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; } if (await ev("!!(V.dec && V.dec.kind === 'main')")) break; await ev("(async()=>{ await pump(J(T.act(0))) })()"); }
  await py(`
from game import Obj, Rune
g = W["g"]; me = ME
g.p[me].runes = [Rune("Calm", me) for _ in range(5)]
for n in ("Dredge Up", "Onslaught"):
    c = Obj(n, me); c.zone = "trash"; g.p[me].trash.append(c)
DU = [c.uid for c in g.p[me].trash if c.cname == "Dredge Up"][0]
H0 = len(g.p[me].hand)
W["d"] = None`);
  await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(300);
  const du = String(await ev("T.DU")), h0 = await ev("T.H0");
  const names = await ev("[...document.querySelectorAll('#board .half.p0 .hand .card.flow')].map(c => c.dataset.card)");
  ok(`${w} px : Dredge Up jouable avec Flow au bout de la main, pas Onslaught sans cible (${JSON.stringify(names)})`, names.length === 1 && names[0] === "Dredge Up");
  const el = pg.locator(`#board [data-uid="${du}"]`);
  if (mob) { const bb = await el.boundingBox(); await pg.touchscreen.tap(bb.x + bb.width - 6, bb.y + bb.height / 2); await pg.waitForTimeout(250);
    if (await ev("document.querySelector('#board .half.p0 .hand').classList.contains('open')")) { const b2 = await el.boundingBox(); await pg.touchscreen.tap(b2.x + b2.width / 2, b2.y + b2.height / 2); } }
  else await el.click();
  await pg.waitForTimeout(300);
  const pop = await ev("$('pop').hidden ? '' : $('pop').innerText");
  ok(`${w} px : le menu propose de la jouer avec Flow (${JSON.stringify(pop.replace(/\n/g, " | "))})`, /Flow/.test(pop));
  await ev("(()=>{ const b = [...document.querySelectorAll('#pop button[data-i]')].find(b => /Flow/.test(b.textContent)); if (b) b.click(); })()");
  await pg.waitForTimeout(500); await waitFor("!!(V && !working)");
  for (let i = 0; i < 10; i++) { if (await ev("!!(V.dec && V.dec.kind !== 'main' && V.dec.options.some(o => o.k === 'pass'))")) { await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'pass')))) })()"); await waitFor("!!(V && !working)"); } else break; }
  await py(`Z = [c.zone for c in W["g"].p[ME].banish if c.uid == DU] + [len(W["g"].p[ME].hand), sum(1 for r in W["g"].p[ME].runes if not r.exhausted)]`);
  const z = JSON.parse(await ev("JSON.stringify(T.Z.toJs ? T.Z.toJs() : T.Z)"));
  ok(`${w} px : coût de Flow payé (2 énergie : ${z[z.length - 1]} runes prêtes sur 5), carte piochée (main ${h0} → ${z[z.length - 2]}), Dredge Up bannie`, z[0] === "banish" && z[z.length - 1] === 3 && z[z.length - 2] === h0 + 1);
  ok(`${w} px : aucune erreur JavaScript` + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
  await pg.screenshot({ path: `${OUT}/flow-${w}.png` });
}
await b.close();
process.exitCode = fails ? 1 : 0;
