import { existsSync } from "node:fs";
// Robot du paiement « Gold ou rune » (pay_gold, règle 429.3) au format PC, à la souris. Bug signalé le 2026-11-06 :
// le choix se faisait en cliquant le battlefield de gauche ou de droite (les options 0 et 1, nombre de Gold, étaient
// prises pour des battlefields). On répond maintenant en cliquant le Gold de la base ou la rune à recycler.
// 1. Staunch Survivor (2 énergie + 1 puissance Body) avec un Gold prêt : aucun battlefield en surbrillance, le Gold et
//    les 3 runes Body le sont ; clic sur le Gold, Valider : le Gold est dépensé, aucune rune recyclée.
// 2. Clic sur la 3e rune (pas celle que le moteur aurait prise) : c'est elle qui est recyclée, le Gold reste.
// 3. Sans Gold (demande du 2026-11-06) : « quelle rune recycler ? », les 3 runes en surbrillance, la 2e cliquée est recyclée.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_gold.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
const pg = await (await b.newContext({ viewport: { width: 1400, height: 900 }, deviceScaleFactor: 1 })).newPage();
const errs = []; pg.on("pageerror", e => errs.push(String(e)));
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
const ev = s => pg.evaluate(s);
const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
const py = code => ev(`pyExec(${JSON.stringify(code)})`);
const fresh = async (withGold = true) => {   // nouvelle partie (graine 4) ; 3 runes Body prêtes, un Gold prêt en base (ou pas), Staunch Survivor en main
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
from game import Obj, Rune
from cards import make_token
g = W["g"]; me = ME
g.p[me].runes = [Rune("Body", me) for _ in range(3)]
g.p[me].pool_e = 0
c = Obj("Staunch Survivor", me); c.zone = "hand"; g.p[me].hand.append(c); SS = c.uid
GO = make_token(g, "Gold", me, "base", ready=True).uid if ${withGold ? "True" : "False"} else -1
W["d"] = None
`);
  await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(300);
};
const state = async () => {
  await py(`R = [any(o.uid == GO for o in W["g"].board), len(W["g"].p[ME].runes), any(u.uid == SS for u in W["g"].units(ME))]`);
  return JSON.parse(await ev("pyGet('R').then(JSON.stringify)"));
};
await pg.goto(`http://localhost:${PORT}/train.html`);
await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");

for (const useGold of [true, false]) {
  await fresh();
  const go = String(await ev("pyGet('GO')"));
  await py(`RU = [r.uid for r in W["g"].p[ME].runes]`);
  const ru = JSON.parse(await ev("pyGet('RU').then(JSON.stringify)")).map(String);
  await ev("(async()=>{ const i = V.dec.options.findIndex(o => o.k === 'play' && /Staunch Survivor/.test(o.label)); await pump(J(T.act(i))) })()");
  await waitFor("!!(V && !working && V.ask)");
  const a = await ev("({ kind: V.ask.kind, options: V.ask.options, uids: V.ask.uids.map(u => u === null ? null : String(u)), zs: V.ask.zs })");
  ok(`question « Gold ou rune » posée : le Gold et chacune des 3 runes (${JSON.stringify(a)})`, a.kind === "pay_gold" && a.options.length === 4 &&
    [...a.uids].sort().join() === [...ru, go].sort().join());
  await pg.waitForTimeout(300);
  const lit = await ev("[...document.querySelectorAll('#board [data-drop]')].filter(z => z.classList.contains('drop-ok') || z.classList.contains('drop-hover')).map(z => z.dataset.drop)");
  ok(`aucun battlefield (ni la base) en surbrillance (${JSON.stringify(lit)})`, lit.length === 0);
  const gi = a.uids.indexOf(go);
  ok(`le Gold de la base répond « ${a.options[gi]} » et est en surbrillance`, gi >= 0 && /Gold/.test(a.options[gi]) &&
    await ev(`!!document.querySelector('#board [data-uid="${go}"].drop-ok')`));
  const halo = await ev(`${JSON.stringify(ru)}.map(u => { const x = document.querySelector('#board .rune[data-uid="' + u + '"].drop-ok'); return !!x && /214, 178, 94/.test(getComputedStyle(x).boxShadow); })`);
  ok(`les 3 runes sont en surbrillance (halo doré visible) : ${JSON.stringify(halo)}`, halo.every(Boolean));
  if (useGold) await pg.screenshot({ path: OUT + "/gold-choix.png" });
  const pick = useGold ? go : ru[2], pi = a.uids.indexOf(pick);   // la 3e rune : pas celle que le moteur aurait prise
  await pg.click(`#board [data-uid="${pick}"]`); await pg.waitForTimeout(250);
  const ft = await ev("document.querySelector('#fbar .ft') ? document.querySelector('#fbar .ft').innerText : ''");
  ok(`clic sur ${useGold ? "le Gold" : "la 3e rune"} : le choix est affiché (${JSON.stringify(ft)})`, ft.includes(a.options[pi]));
  await pg.click("#fOk"); await pg.waitForTimeout(400); await waitFor("!!(V && !working)");
  const r = await state();
  await py(`RU2 = [r.uid for r in W["g"].p[ME].runes]`);
  const ru2 = JSON.parse(await ev("pyGet('RU2').then(JSON.stringify)")).map(String);
  ok(`${useGold ? "Gold dépensé, 3 runes gardées" : "la rune cliquée est recyclée, les deux autres et le Gold restent"} et Staunch Survivor en jeu (${JSON.stringify([r, ru2])})`,
    useGold ? (!r[0] && r[1] === 3 && r[2]) : (r[0] && r[1] === 2 && r[2] && !ru2.includes(ru[2]) && ru2.includes(ru[0]) && ru2.includes(ru[1])));
}
// ---------- 3. Sans Gold : la table demande quand même quelle rune recycler
await fresh(false);
{
  await py(`RU = [r.uid for r in W["g"].p[ME].runes]`);
  const ru = JSON.parse(await ev("pyGet('RU').then(JSON.stringify)")).map(String);
  await ev("(async()=>{ const i = V.dec.options.findIndex(o => o.k === 'play' && /Staunch Survivor/.test(o.label)); await pump(J(T.act(i))) })()");
  await waitFor("!!(V && !working && V.ask)");
  const a = await ev("({ kind: V.ask.kind, title: V.ask.title, options: V.ask.options, uids: V.ask.uids.map(u => u === null ? null : String(u)), zs: V.ask.zs })");
  ok(`sans Gold : question « quelle rune recycler ? » avec les 3 runes (${JSON.stringify(a)})`, a.kind === "pay_rune" && /quelle rune/.test(a.title) &&
    [...a.uids].sort().join() === [...ru].sort().join() && a.zs.every(z => z === null));
  await pg.waitForTimeout(300);
  const halo = await ev(`${JSON.stringify(ru)}.map(u => !!document.querySelector('#board .rune[data-uid="' + u + '"].drop-ok'))`);
  const lit = await ev("[...document.querySelectorAll('#board [data-drop]')].filter(z => z.classList.contains('drop-ok') || z.classList.contains('drop-hover')).length");
  ok(`les 3 runes en surbrillance, aucun battlefield (${JSON.stringify(halo)}, ${lit})`, halo.every(Boolean) && lit === 0);
  await pg.screenshot({ path: OUT + "/rune-choix.png" });
  await pg.click(`#board [data-uid="${ru[1]}"]`); await pg.waitForTimeout(250);
  await pg.click("#fOk"); await pg.waitForTimeout(400); await waitFor("!!(V && !working)");
  const r = await state();
  await py(`RU2 = [r.uid for r in W["g"].p[ME].runes]`);
  const ru2 = JSON.parse(await ev("pyGet('RU2').then(JSON.stringify)")).map(String);
  ok(`la 2e rune cliquée est recyclée, les deux autres restent, Staunch Survivor en jeu (${JSON.stringify([r, ru2])})`,
    r[1] === 2 && r[2] && !ru2.includes(ru[1]) && ru2.includes(ru[0]) && ru2.includes(ru[2]));
}
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
await b.close();
process.exitCode = fails ? 1 : 0;
