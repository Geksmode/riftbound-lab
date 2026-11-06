import { existsSync } from "node:fs";
// Robot des cartes regardées / révélées depuis un deck (2026-11-06, retour utilisateur : « avec Baited Hook il ne montre
// pas les 5 cartes »). Ordinateur (1400x900, clics) et téléphone (393x851, toucher simulé).
// 1. Baited Hook (vraie carte, activée depuis la table, cible touchée) avec des unités prenables : la bulle montre les
//    5 cartes du dessus en images, les prenables se touchent, les autres sont grisées, bouton « Ne rien prendre » ;
//    toucher Vanguard Sergeant le met en jeu.
// 2. Baited Hook sans unité prenable : pas de question, mais les 5 cartes sont montrées avec « OK », qui les ferme.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_regard.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
for (const [w, h, mob, sc] of [[1400, 900, false, 1], [1400, 900, false, 2], [393, 851, true, 1], [393, 851, true, 2]]) {
  const pg = await (await b.newContext({ viewport: { width: w, height: h }, isMobile: mob, hasTouch: mob })).newPage();
  const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
  const tapL = async l => { if (mob) { const bb = await l.boundingBox(); await pg.touchscreen.tap(bb.x + bb.width / 2, bb.y + bb.height / 2); } else await l.click(); await pg.waitForTimeout(250); };
  const tap = sel => tapL(pg.locator(sel).first());
  const py = code => ev(`pyExec(${JSON.stringify(code)})`);
  const settle = async () => {   // passe la priorité jusqu'à une question ou le retour en phase principale chaîne vide
    for (let i = 0; i < 40; i++) {
      await waitFor("!!(V && !working)"); await pg.waitForTimeout(150);
      if (await ev("!!V.ask")) return;
      if (await ev("!!(V.dec && V.dec.kind === 'main' && !(V.st.chain || []).length)")) return;
      await ev("(async()=>{ const o = V.dec && V.dec.options.find(x => x.k === 'pass'); await pump(J(o ? T.act(o.i) : T.step())) })()");
    }
  };
  const fresh = async top => {
    await ev("showMenu('main')").catch(() => {});
    await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
    await ev("document.querySelector('.vopt').open = true; document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'"); await pg.click("#bStart");
      for (let i = 0; i < 200; i++) {
      await waitFor("!!(V && !working)");
      if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; }
      if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; }
      if (await ev("!!(V.dec && V.dec.kind === 'main')")) break;
      await ev("(async()=>{ await pump(J(T.act(0))) })()");
    }
    await py(`
from game import Obj, Rune
g = W["g"]; me = ME
for u in list(g.units(me)): g.kill([u], me)
g.p[me].runes = [Rune("Order", me), Rune("Order", me)]
def P(n):
    o = Obj(n, me); g.enter_board(o, me, "base", ready=True); return o.uid
SK = P("Shipyard Skulker"); HK = P("Baited Hook")
for n in reversed(${JSON.stringify(top)}):
    c = Obj(n, me); c.zone = "deck"; g.p[me].deck.insert(0, c)
W["d"] = None`);
    await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(300);
    // activer Hook depuis la table : toucher la carte, choisir la capacité, toucher la cible, valider
    const hk = String(await ev("pyGet('HK')")), sk = String(await ev("pyGet('SK')"));
    await tap(`#board [data-uid="${hk}"]`);
    const pop = await ev("$('pop').hidden ? '' : $('pop').innerText");
    const btn = pg.locator("#pop button").filter({ hasText: /Hook/ });
    if (await btn.count()) await tapL(btn.first());
    if (await ev("!!(mode && mode.type === 'target')")) { await tap(`#board [data-uid="${sk}"]`); if (await ev("!!document.getElementById('fOk')")) await tap("#fOk"); }
    await settle();
    return pop;
  };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");

  if (sc === 1) {   // ---------- 1. unités prenables (chaque cas dans sa propre page)
  const TOP = ["Mountain Drake", "Vanguard Sergeant", "Pouty Poro", "Sabotage", "Pouty Poro"];
  const pop = await fresh(TOP);
  const q = await ev(`(()=>{ const p = document.querySelector('#fbar .mull.seen'); if (!p) return null;
    const vis = r => r.top >= 0 && r.bottom <= innerHeight && r.left >= 0 && r.right <= innerWidth;
    return { kind: V.ask && V.ask.kind, cards: [...p.querySelectorAll('.card')].map(c => c.dataset.card),
      img: [...p.querySelectorAll('.card')].every(c => /url/.test(getComputedStyle(c).backgroundImage)),
      tap: [...p.querySelectorAll('button.seenc .card')].map(c => c.dataset.card), grey: [...p.querySelectorAll('.seenc.other .card')].map(c => c.dataset.card),
      btn: [...document.querySelectorAll('#fbar .prow button')].map(x => x.textContent), vis: vis(document.getElementById('fbar').getBoundingClientRect()) } })()`);
  ok(`${w} px : Hook activé depuis la table (menu : ${JSON.stringify(pop.replace(/\n/g, " | ").slice(0, 80))}), question hook_pick (${q && q.kind})`, q && q.kind === "hook_pick");
  ok(`${w} px : la bulle montre les 5 cartes du dessus en images (${q && q.cards})`, q && JSON.stringify(q.cards) === JSON.stringify(TOP) && q.img);
  ok(`${w} px : prenables à toucher (${q && q.tap}), autres grisées (${q && q.grey}), bouton « ${q && q.btn} », bulle entière à l'écran`,
    q && JSON.stringify(q.tap) === JSON.stringify(["Vanguard Sergeant", "Pouty Poro", "Pouty Poro"]) && JSON.stringify(q.grey) === JSON.stringify(["Mountain Drake", "Sabotage"])
    && q.btn.length === 1 && /Ne rien prendre/.test(q.btn[0]) && q.vis);
  await pg.screenshot({ path: `${OUT}/regard-hook-${w}.png` });
  await tap('#fbar button.seenc:has(.card[data-card="Vanguard Sergeant"])');
  await settle();
  await py(`R = [u.cname for u in W["g"].units(ME)]`);
  const units = await ev("pyGet('R')");
  ok(`${w} px : Vanguard Sergeant touché → en jeu (${units}), plus de bulle de cartes vues`, String(units).includes("Vanguard Sergeant") && !(await ev("!!document.querySelector('#fbar .mull.seen')")));

  } else {   // ---------- 2. aucune unité prenable
  const TOP2 = ["Mountain Drake", "Sabotage", "Sabotage", "Long Sword", "Sabotage"];
  await fresh(TOP2);
  const s = await ev(`(()=>{ const p = document.querySelector('#fbar .mull.seen'); return p ? { ask: !!V.ask, t: document.querySelector('#fbar .ft').textContent,
    cards: [...p.querySelectorAll('.card')].map(c => c.dataset.card), okb: !!document.getElementById('bSeenOk') } : null })()`);
  ok(`${w} px : sans unité prenable, les 5 cartes sont quand même montrées (${s && s.cards}) : « ${s && s.t} », bouton OK`,
    s && !s.ask && JSON.stringify(s.cards) === JSON.stringify(TOP2) && /Baited Hook/.test(s.t) && s.okb);
  await pg.screenshot({ path: `${OUT}/regard-vide-${w}.png` });
  await tap("#bSeenOk");
  ok(`${w} px : OK ferme la bulle`, !(await ev("!!document.querySelector('#fbar .mull.seen')")));
  }
  ok(`${w} px : aucune erreur JS (${errs.join(" | ")})`, !errs.length);
}
await b.close();
console.log(fails ? `${fails} ÉCHEC(S)` : "tout est OK");
process.exit(fails ? 1 : 0);
