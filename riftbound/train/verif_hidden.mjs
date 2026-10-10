import { existsSync } from "node:fs";
// Robot du mot-clé Hidden (règles 421, 811) dans la vraie table : une carte Hidden en main se cache au doigt
// (menu « Cacher → … ») ET en la glissant sur un battlefield contrôlé, puis se joue depuis la face cachée.
// La situation est posée dans le moteur (Pyodide) au début d'un tour à moi ; tout le reste passe par les gestes.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_hidden.mjs <dossier des captures> [port] [carte]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771", CARD = process.argv[4] || "Back Off";
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
// Situation : la carte en main, 5 runes prêtes (elle peut AUSSI être jouée tout de suite), une unité à moi au battlefield 0 (que je contrôle), une unité adverse en base.
const SETUP = `
from game import Obj, Rune
g = W["g"]; me = ME
while len(g.p[me].runes) < 5: g.p[me].runes.append(Rune("Calm", me))
def _mk(name, pid, zone):
    o = Obj(name, pid); o.zone = zone; return o
for r in g.p[me].runes: r.exhausted = False
c = _mk(${JSON.stringify(CARD)}, me, "hand"); g.p[me].hand.append(c); HID_UID = c.uid
for b in g.bfs: b.facedowns.clear()
u = Obj("Brazen Buccaneer", me); g.enter_board(u, me, 0, ready=True); g.bfs[0].ctrl = me
e = Obj("Brazen Buccaneer", 1 - me); g.enter_board(e, 1 - me, "base", ready=True)
W["d"] = None
`;
await start(process.env.RB_SEED || "4");
await py(SETUP); await refresh(); await idle();
const uid = String(await ev("T.HID_UID")), ouid = String(await ev(`ALIAS["${uid}"] ?? "${uid}"`));   // exemplaire qui porte les options
const opts = await ev(`V.dec.options.filter(o => String(o.src) === "${ouid}").map(o => o.k + ":" + o.loc + ":" + (o.t1||[]).length)`);
ok(`${CARD} en main : le moteur propose de la cacher (${opts.join(", ")})`, opts.some(o => o.startsWith("hide:0")));
await pg.screenshot({ path: OUT + "/hidden-avant.png" });
// 1. au doigt
await pg.click(`#board .hand [data-uid="${uid}"]`); await pg.waitForTimeout(300);
const pop = await ev("$('pop').hidden ? '' : $('pop').innerText");
ok(`toucher la carte : « Cacher » dans le menu (${JSON.stringify(pop.replace(/\n/g, " | "))})`, /Cacher/.test(pop));
await ev("sel=null; mode=null; decorate()"); await pg.waitForTimeout(200);
// 2. au glisser sur le battlefield contrôlé
const s = await pg.locator(`#board .hand [data-uid="${uid}"]`).boundingBox();
const z = await pg.locator(`#board [data-drop="0"]`).first().boundingBox();
await pg.mouse.move(s.x + s.width / 2, s.y + s.height / 2); await pg.mouse.down();
await pg.mouse.move(s.x + s.width / 2 + 20, s.y + s.height / 2 - 20, { steps: 3 });
await pg.mouse.move(z.x + z.width / 2, z.y + 12, { steps: 10 });
console.log("  pendant le glisser : zones en surbrillance", await ev("[...document.querySelectorAll('.drop-ok')].map(x=>x.dataset.drop||x.dataset.uid).join(',')"),
  "· sous le doigt", await ev(`(()=>{const e=document.elementFromPoint(${z.x + z.width / 2},${z.y + 12}); const t=e&&e.closest('#board [data-uid], #board [data-drop]'); return t? (t.dataset.drop??('u'+t.dataset.uid)) : String(e&&e.className)})()`));
await pg.mouse.up();
await pg.waitForTimeout(500); await idle();
const after = await ev(`({inHand: !!document.querySelector('#board .hand [data-uid="${ouid}"]'), mode: mode ? mode.type : null,
  fb: $('fbar').hidden ? '' : $('fbar').innerText})`);
await pg.screenshot({ path: OUT + "/hidden-apres-glisser.png" });
const fd = await facedowns();
ok(`glisser sur le battlefield : carte cachée (en main : ${after.inHand}, mode : ${after.mode}, bulle : ${JSON.stringify(after.fb.slice(0, 80))})`,
   !after.inHand && after.mode !== "target" && JSON.parse(fd)[0].includes(+ouid));
console.log("  cartes face cachée par battlefield :", fd);
// 3. tour suivant : la carte cachée se joue depuis la face cachée, au doigt, pour 0 énergie (811.1.b)
if (JSON.parse(await facedowns())[0].includes(+ouid)) {
  await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'end')))) })()");
  let mine = false;
  for (let i = 0; i < 400 && !mine; i++) {
    await idle();
    if (await ev("V.winner !== null && V.winner !== undefined")) break;
    if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(V.ask.options.length - 1)))) })()"); continue; }
    if (await ev(`!!(V.dec && V.dec.options.some(o => String(o.src) === "${ouid}" && o.k === "play"))`)) { mine = true; break; }
    if (await ev("!!V.dec")) await ev("(async()=>{ const i = V.dec.options.findIndex(o => o.k === 'pass' || o.k === 'end'); await pump(J(T.act(i < 0 ? 0 : i))) })()");
  }
  ok("tour suivant : la carte cachée peut être jouée", mine);
  if (mine) {
    await py("for _r in W['g'].p[ME].runes: _r.exhausted = True\nW['d'] = None"); await refresh(); await idle();   // 0 énergie disponible
    const el = pg.locator(`#board [data-uid="${ouid}"]`).first();
    ok("la carte cachée est affichée et touchable sur le battlefield", await el.count() > 0);
    await el.click(); await pg.waitForTimeout(300);
    const pop2 = await ev("$('pop').hidden ? '' : $('pop').innerText");
    ok(`toucher la carte cachée : on peut la jouer sans énergie (${JSON.stringify(pop2.replace(/\n/g, " | "))})`, /Cibler|Jouer/.test(pop2));
    if (/Cibler/.test(pop2)) {
      await pg.click("#pop button[data-tg]"); await pg.waitForTimeout(300);
      const t = await ev("(()=>{ const x = document.querySelector('#board .drop-ok[data-uid]'); return x ? x.dataset.uid : null })()");
      if (t) { await pg.click(`#board [data-uid="${t}"]`); await pg.waitForTimeout(200); }
      await ev("(()=>{ const b = [...document.querySelectorAll('#fbar button')].find(b => /Valider/.test(b.textContent)); if (b) b.click(); })()");
      await pg.waitForTimeout(400); await idle();
      for (let i = 0; i < 20; i++) { if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); await idle(); } else break; }
      for (let i = 0; i < 40; i++) {             // l'adversaire peut répondre : on passe jusqu'à la résolution
        await idle();
        if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; }
        if (await ev("!!(V.dec && V.dec.kind !== 'main' && V.dec.options.some(o => o.k === 'pass'))")) { await ev("(async()=>{ await pump(J(T.act(V.dec.options.findIndex(o => o.k === 'pass')))) })()"); continue; }
        break;
      }
      console.log("  après : mode", await ev("mode ? mode.type : null"), "· bulle", JSON.stringify(await ev("$('fbar').hidden ? '' : $('fbar').innerText.slice(0,120)")));
      await py("FDZ = [c.zone for c in W['g'].p[ME].trash if c.uid == " + ouid + "]");
      const z = await ev("JSON.stringify(T.FDZ.toJs ? T.FDZ.toJs() : T.FDZ)");
      await pg.screenshot({ path: OUT + "/hidden-jouee.png" });
      ok(`la carte cachée est jouée et résolue (cible ${t}, défausse : ${z})`, z.includes("trash"));
    }
  }
}
// 4. Bandle Tree (« You may hide an additional card here ») : deux cartes cachées au même endroit, chacune affichée
//    à part et touchable ; celles de l'adversaire en dos de carte. La 2e n'était pas transmise à la table.
await py(`
from game import Obj
g = W["g"]
for b in g.bfs: b.facedowns.clear()
BT = []
for bf, pid, n in ((0, ME, ${JSON.stringify(CARD)}), (0, ME, ${JSON.stringify(CARD)}), (1, 1 - ME, "Back Off"), (1, 1 - ME, "Gust")):
    c = Obj(n, pid); c.zone, c.hidden_turn, c.hidden_bf = "facedown", g.turn_no - 1, bf
    g.bfs[bf].facedowns.append(c)
    if pid == ME: BT.append(c.uid)
for r in g.p[ME].runes: r.exhausted = False
W["d"] = None
`); await refresh(); await idle();
const bt = JSON.parse(await ev("JSON.stringify(T.BT.toJs ? T.BT.toJs() : T.BT)")).map(String);
const shown = await ev(`[...document.querySelectorAll('#board [data-drop="0"] .fd [data-uid]')].map(x => x.dataset.uid)`);
const opp = await ev(`(()=>{ const f = document.querySelector('#board [data-drop="1"] .fd'); return f ? [f.children.length, f.innerText.trim()] : [0, ""] })()`);
await pg.screenshot({ path: OUT + "/hidden-bandle-tree.png" });
ok(`Bandle Tree : mes deux cartes cachées sont affichées séparément (${shown.join(", ")})`, shown.length === 2 && bt.every(u => shown.includes(u)));
ok(`Bandle Tree : les deux cartes cachées adverses sont deux dos de carte (${opp.join(" · ")})`, opp[0] === 2 && /2 cachées/.test(opp[1]));
const r2 = await pg.locator(`#board [data-uid="${bt[1]}"]`).first().boundingBox(), r1 = await pg.locator(`#board [data-uid="${bt[0]}"]`).first().boundingBox();
ok("Bandle Tree : les deux cartes ne se recouvrent pas", !!(r1 && r2) && (r1.x + r1.width <= r2.x || r2.x + r2.width <= r1.x));
if (r2) {
  await pg.mouse.click(r2.x + r2.width / 2, r2.y + r2.height / 2); await pg.waitForTimeout(300);
  const pop3 = await ev("$('pop').hidden ? '' : $('pop').innerText");
  ok(`Bandle Tree : toucher la 2e carte cachée permet de la jouer (${JSON.stringify(pop3.replace(/\n/g, " | "))})`, /Cibler|Jouer/.test(pop3));
}
ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
await b.close();
process.exitCode = fails ? 1 : 0;
