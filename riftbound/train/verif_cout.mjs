import { existsSync } from "node:fs";
// Robot des coûts supplémentaires optionnels dans la vraie table (Shimmering Swimmer : « You may kill a friendly unit or
// gear as an additional cost to play me »). Chaque façon de payer doit avoir sa ligne (sans coût, en détruisant l'unité,
// en détruisant le gear) au toucher comme au glisser ; avant le 2026-10-22, les variantes avaient le même libellé et la
// table n'en gardait qu'une (sans coût). Puis on la joue en détruisant le gear et on inflige 2 à l'ennemi.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_cout.mjs <dossier des captures> [port]
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
  await py(JSON.stringify([   // opérations de rust/src/train_dbg.rs
    ["runes", "me", ["Fury", "Fury", "Fury", "Fury", "Fury"]], ["clear_base"],
    { as: "F", op: ["board", "me", "Shipyard Skulker", "base", true] }, { as: "X", op: ["board", "me", "Seal of Discord", "base", true] },
    { as: "E", op: ["board", "op", "Mournful Witness", "base", true] }, { as: "SW", op: ["card", "me", "Shimmering Swimmer", "hand"] },
    ["d_none"]]));
  await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(300);
  const sw = String(await ev("pyGet('SW')"));
  const el = pg.locator(`#board [data-uid="${sw}"]`);
  if (!mob) {   // glisser sur la base : la liste « comment le jouer ? » montre les trois façons de payer
    const bb = await el.boundingBox(), z = await pg.locator('#board .half.p0 [data-drop="base"]').first().boundingBox();
    await pg.mouse.move(bb.x + bb.width / 2, bb.y + bb.height / 2); await pg.mouse.down();
    await pg.mouse.move(z.x + z.width / 2, z.y + z.height / 2, { steps: 12 }); await pg.mouse.up(); await pg.waitForTimeout(300);
    const list = await ev("[...document.querySelectorAll('#fbar button[data-i]')].map(b => b.textContent)");
    ok(`${w} px : glisser sur la base → une ligne par façon de payer (${JSON.stringify(list)})`,
      list.some(t => /Seal of Discord/.test(t)) && list.some(t => /Shipyard Skulker/.test(t)) && list.some(t => /Sans coût/.test(t)));
    await ev("(()=>{ const b = document.getElementById('fNo'); if (b) b.click(); })()"); await pg.waitForTimeout(200);
  }
  if (mob) { const bb = await el.boundingBox(); await pg.touchscreen.tap(bb.x + bb.width - 6, bb.y + bb.height / 2); await pg.waitForTimeout(250);
    if (await ev("document.querySelector('#board .half.p0 .hand').classList.contains('open')")) { const b2 = await el.boundingBox(); await pg.touchscreen.tap(b2.x + b2.width / 2, b2.y + b2.height / 2); } }
  else await el.click();
  await pg.waitForTimeout(300);
  const pop = await ev("$('pop').hidden ? '' : $('pop').innerText");
  ok(`${w} px : le menu propose sans coût / en détruisant l'unité / en détruisant le gear (${JSON.stringify(pop.replace(/\n/g, " | "))})`,
    /Sans coût supplémentaire/.test(pop) && /détruisant Shipyard Skulker/.test(pop) && /détruisant Seal of Discord/.test(pop));
  await ev("(()=>{ const b = [...document.querySelectorAll('#pop button[data-i]')].find(b => /Seal of Discord/.test(b.textContent)); if (b) b.click(); })()");
  await pg.waitForTimeout(500); await waitFor("!!(V && !working)");
  for (let i = 0; i < 12; i++) {
    if (await ev("!!V.ask")) { await ev("(async()=>{ const k = V.ask.options.findIndex(o => /Mournful Witness/.test(o)); await pump(J(T.answer(JSON.stringify(Math.max(0, k))))) })()"); await waitFor("!!(V && !working)"); continue; }
    if (await ev("!!(V.dec && V.dec.kind !== 'main' && V.dec.options.some(o => o.k === 'pass'))")) { await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'pass')))) })()"); await waitFor("!!(V && !working)"); continue; }
    break;
  }
  await py(JSON.stringify([{ as: "Z", op: ["list", ["on_board", { v: "SW" }], ["on_board", { v: "X" }], ["on_board", { v: "F" }], ["damage", { v: "E" }], ["ready_runes", "me"]] }]));
  { const zz = JSON.parse(await ev("pyGet('Z').then(JSON.stringify)")); zz[1] = !zz[1]; if (zz[3] === null) zz[3] = "trash"; await py(JSON.stringify([["set", "Z", zz]])); }
  const z = JSON.parse(await ev("pyGet('Z').then(JSON.stringify)"));
  ok(`${w} px : Swimmer en jeu, Seal of Discord détruit, Shipyard Skulker gardé, 2 dégâts à Mournful Witness (${JSON.stringify(z)})`,
    z[0] && z[1] && z[2] && (z[3] === "trash" || z[3] === 2) && z[4] === 2);
  ok(`${w} px : aucune erreur JavaScript` + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
  await pg.screenshot({ path: `${OUT}/cout-${w}.png` });
}
await b.close();
process.exitCode = fails ? 1 : 0;
