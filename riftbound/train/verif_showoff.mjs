import { existsSync } from "node:fs";
// Robot du [Show Off] (règle 832) au format téléphone : en jouant une carte avec [Show Off], la table demande quelle
// carte montrer (main ou objet à toi en jeu qui remplit la condition), en images, ou « Sans Show Off ». Demandes du
// 2026-10-30 (avant : la page fusionnait ces variantes, même libellé, et le joueur n'avait jamais le choix ; puis
// « au lieu d'avoir du texte, montre les cartes qu'on peut show off »).
// 1. Incorrigible Construct (Show Off a Mech, entre prêt) : menu de la carte, « Jouer → base : Show Off… », la bulle
//    montre l'image de Mega-Mech et « Sans Show Off » ; avec Show Off il entre prêt, sans il entre épuisé.
// 2. Primordial Roar (sort ciblé) : cible touchée, Valider, puis la bulle montre les unités en images ; Playful
//    Phantom (5, en jeu) touché tue l'ennemi.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_showoff.mjs <dossier des captures> [port]
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
// la bulle du Show Off : noms des cartes en images, boutons texte
const BUBBLE = "({ cards: [...document.querySelectorAll('#fbar .mull.show .card')].map(c => c.dataset.card), text: [...document.querySelectorAll('#fbar .prow button[data-i]')].map(b => b.textContent) })";
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

// ---------- 1. Incorrigible Construct, avec puis sans Show Off
for (const withShow of [true, false]) {
  await fresh(`CC = H("Incorrigible Construct"); MM = H("Mega-Mech")`);
  const cc = String(await ev("pyGet('CC')"));
  await tapSel(`#board [data-uid="${cc}"]`);
  const pop = await ev("$('pop').hidden ? '' : $('pop').innerText");
  ok(`Incorrigible Construct touché : le menu propose « Show Off… » (${JSON.stringify(pop.replace(/\n/g, " | "))})`, /Show Off…/.test(pop));
  await tapSel("#pop button[data-sl]");
  const bub = await ev(BUBBLE);
  ok(`la bulle montre les cartes montrables en images et « Sans Show Off » (${JSON.stringify(bub)})`,
    bub.cards.includes("Mega-Mech") && bub.cards.every(n => n !== "Incorrigible Construct") && bub.text.some(t => /Sans Show Off/.test(t)));
  if (withShow) await pg.screenshot({ path: OUT + "/showoff-menu.png" });
  if (withShow) await tapSel('#fbar .mull.show button[data-i]:has(.card[data-card="Mega-Mech"])');
  else await tapText("#fbar .prow button[data-i]", /Sans Show Off/);
  await settle();
  await py(`R = [[u.exhausted for u in W["g"].units(ME) if u.uid == ${cc}], [c.uid for c in W["g"].p[ME].hand if c.cname == "Mega-Mech"]]`);
  const r = JSON.parse(await ev("pyGet('R').then(JSON.stringify)"));
  ok(`${withShow ? "avec" : "sans"} Show Off : Incorrigible Construct entre ${withShow ? "prêt" : "épuisé"}, Mega-Mech reste en main (${JSON.stringify(r)})`,
    r[0].length === 1 && r[0][0] === !withShow && r[1].length === 1);
}

// ---------- 2. Primordial Roar : cible, Valider, puis l'unité à montrer
await fresh(`
PR = H("Primordial Roar")
a = Obj("Playful Phantom", me); g.enter_board(a, me, "base", ready=True); PH = a.uid
e = Obj("Shipyard Skulker", op); g.enter_board(e, op, "base", ready=True); FOE = e.uid`);
const pr = String(await ev("pyGet('PR')")), ph = String(await ev("pyGet('PH')")), foe = String(await ev("pyGet('FOE')"));
await tapSel(`#board [data-uid="${pr}"]`);
await tapSel("#pop button[data-tg]");
const pk = await ev(`!!document.querySelector('#fbar .pick-strip [data-pk="u:${foe}"]')`);
if (pk) await tapSel(`#fbar .pick-strip [data-pk="u:${foe}"]`); else await tapSel(`#board [data-uid="${foe}"]`);
await tapSel("#fOk");
const bub = await ev(BUBBLE);
ok(`Primordial Roar, cible choisie : la bulle montre les unités à montrer en images et « Sans Show Off » (${JSON.stringify(bub)})`,
  bub.cards.includes("Playful Phantom") && bub.text.some(t => /Sans Show Off/.test(t)));
await pg.screenshot({ path: OUT + "/showoff-liste.png" });
await tapSel('#fbar .mull.show button[data-i]:has(.card[data-card="Playful Phantom"])');
await settle();
await py(`R = [W["g"].obj(${foe}) is None, W["g"].obj(${ph}) is not None]`);
const r = JSON.parse(await ev("pyGet('R').then(JSON.stringify)"));
ok(`Playful Phantom (5) montré : Primordial Roar tue Shipyard Skulker, Playful Phantom reste en jeu (${JSON.stringify(r)})`, r[0] && r[1]);
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
await b.close();
process.exitCode = fails ? 1 : 0;
