import { existsSync } from "node:fs";
// Robot de la main révélée (règle 424) dans la vraie table : Sabotage jouée au doigt, la main de l'adversaire
// s'affiche face visible pendant le choix de la carte à recycler, reste visible après, puis se recache quand on joue
// le coup suivant (« Terminer le tour »). La situation est posée dans le moteur (Pyodide) au début d'un tour à moi.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_revele.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771", CARD = "Sabotage";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
const pg = await (await b.newContext({ viewport: { width: 1400, height: 900 } })).newPage();
const errs = []; pg.on("pageerror", e => errs.push(String(e)));
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
const ev = s => pg.evaluate(s);
const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(150); } throw new Error("attente : " + s); };
const idle = () => waitFor("!!(V && !working)");
const py = code => ev(`(()=>{ T.__builtins__.get('exec')(${JSON.stringify(code)}, T.__dict__); return true })()`);
const refresh = () => ev("(async()=>{ await pump(J(T.step())) })()");
async function start(seed) {
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev(`document.getElementById('sSeed').value='${seed}'; document.getElementById('sFirst').value='0'`); await pg.click("#bStart");
  for (let i = 0; i < 200; i++) {            // mulligan gardé, jusqu'à ma première décision du tour
    await idle();
    if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
    if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; }
    if (await ev("!!(V.dec && V.dec.kind === 'main')")) return;
    await ev("(async()=>{ await pump(J(T.act(0))) })()");
  }
  throw new Error("pas de décision principale");
}
const facedowns = () => py("FD = [[c.uid for c in b.facedowns] for b in W['g'].bfs]").then(() => ev("JSON.stringify(T.FD.toJs ? T.FD.toJs() : T.FD)"));
const SETUP = `
from game import Obj, Rune
g = W["g"]; me = ME
for r in g.p[me].runes: r.exhausted = False
g.p[me].runes += [Rune("Body", me) for _ in range(3)]
c = Obj("Sabotage", me); c.zone = "hand"; g.p[me].hand.append(c); SAB = c.uid
g.p[1 - me].hand = []
for n in ("Discipline", "Watchful Sentry", "Falling Star"):
    o = Obj(n, 1 - me); o.zone = "hand"; g.p[1 - me].hand.append(o)
W["d"] = None
`;
const shown = () => ev("document.querySelectorAll('#board .hand.top .card:not(.back)').length");
const label = () => ev("(document.querySelector('#board .hand.top .who-hand')||{}).innerText || ''");
await start(process.env.RB_SEED || "4");
await py(SETUP); await refresh(); await idle();
ok("avant l'effet : main adverse cachée", await shown() === 0);
const uid = String(await ev("T.SAB"));
await pg.click(`#board .hand [data-uid="${uid}"]`); await pg.waitForTimeout(300);
const pop = await ev("$('pop').hidden ? '' : $('pop').innerText");
await ev("(()=>{ const b = [...document.querySelectorAll('#pop button[data-i]')].find(b => /Jouer/.test(b.textContent)); if (b) b.click(); })()");
for (let i = 0; i < 20; i++) {                    // l'IA peut répondre : on passe jusqu'à la question de Sabotage
  await idle();
  if (await ev("!!V.ask")) break;
  if (await ev("!!(V.dec && V.dec.options.some(o => o.k === 'pass'))")) await pg.click("#bMain");
  else await pg.waitForTimeout(200);
}
ok(`Sabotage jouée au doigt (menu : ${JSON.stringify(pop.replace(/\n/g, " | "))}) : question « ${await ev("V.ask ? V.ask.title : ''")} »`, await ev("!!(V.ask && V.ask.kind === 'sabotage')"));
await pg.screenshot({ path: OUT + "/revele-pendant.png" });
ok(`pendant le choix : les 3 cartes de l'adversaire visibles (${await shown()}) et marquées « ${await label()} »`, await shown() === 3 && /révélée/.test(await label()));
await pg.click("#fbar [data-a]"); await pg.waitForTimeout(300); await idle();
await pg.screenshot({ path: OUT + "/revele-apres.png" });
ok(`effet résolu, focus pas encore passé : les 2 cartes restantes visibles (${await shown()})`, await shown() === 2 && await ev("!!(V.dec && V.dec.kind === 'main')"));
await pg.click("#bMain"); await pg.waitForTimeout(400); await idle();
ok(`coup suivant joué (Terminer le tour) : main adverse recachée (${await shown()} visible)`, await shown() === 0 && !/révélée/.test(await label()));
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
await b.close();
process.exitCode = fails ? 1 : 0;
