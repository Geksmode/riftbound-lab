import { existsSync } from "node:fs";
// Robot du ciblage au format téléphone : la bulle liste les choix possibles en grandes cartes (camp et emplacement),
// car sur le plateau ils sont petits ou cachés sous la bulle (ta base). Shuriken Flip : ennemi, allié de ta base à
// déplacer, destination, Valider, tout par la liste, puis le sort résolu. Situation posée dans le moteur (Pyodide).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_cibles.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
const pg = await (await b.newContext({ viewport: { width: 393, height: 851 }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 })).newPage();
const errs = []; pg.on("pageerror", e => errs.push(String(e)));
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
const ev = s => pg.evaluate(s);
const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
const py = code => ev(`(()=>{ T.__builtins__.get('exec')(${JSON.stringify(code)}, T.__dict__); return true })()`);
const tapSel = async sel => { const r = await pg.locator(sel).first().boundingBox(); await pg.touchscreen.tap(r.x + r.width / 2, r.y + r.height / 2); await pg.waitForTimeout(250); };
const strip = () => ev("[...document.querySelectorAll('#fbar .pick-strip .pk')].map(b => (b.querySelector('.pkn') || b).textContent + ' / ' + ((b.querySelector('.pkw') || {}).textContent || ''))");
await pg.goto(`http://localhost:${PORT}/train.html`);
await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
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
from game import Obj, Rune
g = W["g"]; me = ME; op = 1 - ME
g.p[me].runes = [Rune(d, me) for d in ("Calm", "Calm", "Fury", "Fury")]
c = Obj("Shuriken Flip", me); c.zone = "hand"; g.p[me].hand.append(c); SF = c.uid
a = Obj("Stellacorn Herder", me); g.enter_board(a, me, "base", ready=True); ALLY = a.uid
g.enter_board(Obj("Mournful Witness", me), me, "base", ready=True)
e = Obj("Watchful Sentry", op); g.enter_board(e, op, 1, ready=True); g.bfs[1].ctrl = op; FOE = e.uid
W["d"] = None
`);
await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(300);
const sf = String(await ev("T.SF")), ally = String(await ev("T.ALLY")), foe = String(await ev("T.FOE"));
const sfu = String(await ev(`ALIAS["${sf}"] ?? "${sf}"`));
await tapSel(`#board [data-uid="${sf}"]`);
const pop = await ev("$('pop').hidden ? '' : $('pop').innerText");
ok(`Shuriken Flip touchée : « Cibler » proposé (${JSON.stringify(pop.replace(/\n/g, " | "))})`, /Cibler/.test(pop));
await tapSel("#pop button[data-tg]");
let st = await strip();
ok(`étape 1 : la bulle liste les ennemis en grandes cartes (${st.join(" ; ")})`, st.some(x => /Watchful Sentry \/ adverse/.test(x)));
await pg.screenshot({ path: OUT + "/cibles-etape1.png" });
await tapSel(`#fbar .pick-strip [data-pk="u:${foe}"]`);
st = await strip();
ok(`étape 2 : l'allié à déplacer se choisit dans la liste, base comprise (${st.join(" ; ")})`, st.some(x => /Stellacorn Herder \/ à toi · base/.test(x)));
await pg.screenshot({ path: OUT + "/cibles-etape2.png" });
await tapSel(`#fbar .pick-strip [data-pk="u:${ally}"]`);
st = await strip();
ok(`étape 3 : les destinations sont des boutons (${st.join(" ; ")})`, st.length > 0);
await tapSel("#fbar .pick-strip .zonepk");
ok("tout est choisi : « Valider » actif", await ev("!!document.querySelector('#fOk') && !document.querySelector('#fOk').disabled"));
await tapSel("#fOk"); await pg.waitForTimeout(600); await waitFor("!!(V && !working)");
for (let i = 0; i < 20; i++) { if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); await waitFor("!!(V && !working)"); } else if (await ev("!!(V.dec && V.dec.kind !== 'main' && V.dec.options.some(o => o.k === 'pass'))")) { await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'pass')))) })()"); await waitFor("!!(V && !working)"); } else break; }
await py(`R = [c.zone for c in W["g"].p[ME].trash if c.uid == ${sfu}] + [W["g"].obj(${ally}).loc if W["g"].obj(${ally}) else None]`);
const r = await ev("JSON.stringify(T.R.toJs ? T.R.toJs() : T.R)");
ok(`Shuriken Flip résolu (défausse, allié déplacé : ${r})`, r.includes("trash") && !r.includes('"base"'));
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
await b.close();
process.exitCode = fails ? 1 : 0;
