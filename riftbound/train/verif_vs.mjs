import { existsSync } from "node:fs";
// Robot de l'écran de sélection contre l'IA (à la Smash Bros) dans la vraie table : 49 légendes + « ? » ; une légende
// sans deck jouable est grisée et ne se choisit pas ; panneau IA actif → la légende touchée prend son deck par défaut
// (train.default_decks) ; chaque menu ne montre que les variantes de sa légende ; panneau Toi actif → la légende va à ton deck ; « ? » tire une légende jouable au lancement ;
// la partie démarre avec les bonnes légendes. Ordinateur et téléphone : tout tient à l'écran (pas de défilement horizontal).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_vs.mjs <dossier des captures> [port]
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
  const tap = async sel => { if (mob) { const bb = await pg.locator(sel).first().boundingBox(); await pg.touchscreen.tap(bb.x + bb.width / 2, bb.y + bb.height / 2); } else await pg.click(sel, { force: true }); await pg.waitForTimeout(120); };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.click("[data-m=new]"); await waitFor("!!document.querySelector('.roster')"); await pg.waitForTimeout(400);
  const n = await ev("document.querySelectorAll('.vtile').length"), on = await ev("[...document.querySelectorAll('.vtile:not(.off)')].map(t => t.dataset.leg)");
  const dflt = await ev("Object.fromEntries(CAT.legends.filter(l => l.key).map(l => [l.n, l.key]))");
  ok(`${w} px : 49 légendes + « ? » (${n}), jouables = légendes avec un deck par défaut + « ? » (${on.length})`, n === 50 && Object.keys(dflt).every(l => on.includes(l)) && on.includes("?"));
  const off = await ev("(document.querySelector('.vtile.off')||{}).dataset.leg");
  const before = await ev("$('sOpp').value"); await tap(`.vtile.off[data-leg="${off}"]`);
  ok(`${w} px : une légende grisée (${off}) ne se choisit pas`, await ev("$('sOpp').value") === before);
  await tap(".vpad.cpu .vtag"); await tap('.vtile[data-leg="Akali, Rogue Assassin"]');
  ok(`${w} px : IA → Akali prend son deck par défaut (${await ev("$('sOpp').value")})`, await ev("$('sOpp').value") === dflt["Akali, Rogue Assassin"] && await ev("document.querySelector('.vtile.cpu').dataset.leg") === "Akali, Rogue Assassin");
  await tap(".vpad.p1 .vtag"); await tap('.vtile[data-leg="LeBlanc, Deceiver"]');
  ok(`${w} px : Toi → LeBlanc va à ton deck (${await ev("$('sMine').value")}), l'IA garde Akali`, await ev("$('sMine').value") === dflt["LeBlanc, Deceiver"] && await ev("$('sOpp').value") === dflt["Akali, Rogue Assassin"] && await ev("document.querySelector('.vpad.p1 .vlg').textContent") === "LeBlanc");
  const legsOf = id => ev(`[...$('${id}').options].map(o => deckLegend(o.value))`);
  const lo = await legsOf("sOpp"), lm = await legsOf("sMine");
  ok(`${w} px : le menu de l'IA ne montre que les variantes d'Akali (${lo.length}), le tien que celles de LeBlanc (${lm.length})`, lo.length >= 2 && lo.every(x => x === "Akali, Rogue Assassin") && lm.length >= 2 && lm.every(x => x === "LeBlanc, Deceiver"));
  ok(`${w} px : le deck par défaut est en tête du menu, marqué ★`, await ev("$('sOpp').options[0].value === $('sOpp').value && $('sOpp').options[0].text.startsWith('★')"));
  const fit = await ev("(()=>{const m=$('mbox'),r=m.getBoundingClientRect();return {sw: document.documentElement.scrollWidth, w: innerWidth, right: r.right, bottom: [...document.querySelectorAll('.vpad')].map(x=>x.getBoundingClientRect().bottom), h: innerHeight}})()");
  ok(`${w} px : pas de défilement horizontal, les deux panneaux visibles sans défiler (${JSON.stringify(fit)})`, fit.sw <= fit.w && fit.right <= fit.w && Math.max(...fit.bottom) <= fit.h);
  const art = await ev("getComputedStyle(document.querySelector('.vtile:not(.off) .vart')).backgroundImage");
  ok(`${w} px : les tuiles montrent l'image de la légende`, /atlas|img/.test(art));
  await pg.screenshot({ path: `${OUT}/vs_${w}.png` });
  await tap('.vtile[data-leg="?"]');
  ok(`${w} px : « ? » met l'IA au hasard`, await ev("document.querySelector('.vpad.cpu .vlg').textContent") === "Au hasard");
  await ev("$('sSeed').value='7'"); await tap("#bStart");
  await waitFor("!!(V && !working)");
  const lg = await ev("(()=>{T.__builtins__.get('exec')(\"LG=[W['g'].p[i].legend_name for i in (0,1)]\", T.__dict__); return T.LG.toJs()})()");
  ok(`${w} px : la partie démarre, toi LeBlanc, l'IA une légende jouable tirée au hasard (${lg})`, lg[0] === "LeBlanc, Deceiver" && Object.keys(dflt).includes(lg[1]));
  ok(`${w} px : aucune erreur JS (${errs.join(" | ")})`, !errs.length);
}
await b.close();
console.log(fails ? `${fails} ÉCHEC(S)` : "tout est OK");
process.exit(fails ? 1 : 0);
