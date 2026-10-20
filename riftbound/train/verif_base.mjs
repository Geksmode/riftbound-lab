import { existsSync } from "node:fs";
// Robot de la base serrée au format téléphone (393x851 puis 360x740) : 9 unités dans ta base (dont 3 épuisées), 7 dans
// celle de l'adversaire. Le premier toucher ouvre la base en grand (grandes cartes, bandeau « Ta base » + Fermer), elle
// défile, toucher une unité montre ses actions, « Déplacer… » puis un champ de bataille la laisse ouverte, « Fermer » la
// referme, la base adverse s'ouvre aussi (lecture d'une carte), toucher ailleurs referme, une base peu remplie ne s'ouvre
// pas (le toucher agit directement sur la carte). Situation posée dans le moteur (Pyodide).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_base.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0;
const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
for (const [w, h] of [[393, 851], [360, 740]]) {
  console.log(`--- ${w}x${h}`);
  const pg = await (await b.newContext({ viewport: { width: w, height: h }, isMobile: true, hasTouch: true, deviceScaleFactor: 2 })).newPage();
  const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error(s); };
  const py = code => ev(`pyExec(${JSON.stringify(code)})`);
  const box = sel => pg.locator(sel).first().boundingBox();
  const tap = async (sel, dx) => { const r = await box(sel); await pg.touchscreen.tap(r.x + (dx === undefined ? r.width / 2 : dx), r.y + r.height / 2); await pg.waitForTimeout(300); };
  const st = () => ev(`(()=>{ const z = document.querySelector('#board .half.p0 .zbase'), o = document.querySelector('#board .half.p1 .zbase');
    const c = z.querySelector(':scope > .slot:not(.x) .card');
    return { open: z.classList.contains('open'), oopen: o.classList.contains('open'), head: [...document.querySelectorAll('#board .bhead')].map(x => x.innerText.replace(/\\n/g, ' ')),
      cw: c ? Math.round(c.getBoundingClientRect().width) : 0, sw: z.scrollWidth, cw2: z.clientWidth, pop: !$('pop').hidden, popT: $('pop').hidden ? '' : $('pop').innerText.replace(/\\n/g, ' | '),
      zoom: !Z.hidden, mode: mode ? mode.type : null, dest: mode && mode.dest !== undefined ? mode.dest : null, sel, BASEOPEN } })()`);
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await pg.tap("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev("document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'"); await pg.tap("#bStart");
  for (let i = 0; i < 200; i++) {
    await waitFor("!!(V && !working)");
    if (await ev("!!document.getElementById('bKeep')")) { await pg.tap("#bKeep"); continue; }
    if (await ev("!!V.ask")) { await ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()"); continue; }
    if (await ev("!!(V.dec && V.dec.kind === 'main')")) break;
    await ev("(async()=>{ await pump(J(T.act(0))) })()");
  }
  await py(`
from game import Obj
g = W["g"]; me = ME; op = 1 - ME
names = ["Stellacorn Herder", "Mournful Witness", "Noxus Hopeful", "Blitzcrank, Impassive", "Kai'Sa, Survivor", "Astral Heron"]
for k in range(9):
    g.enter_board(Obj(names[k % 6], me), me, "base", ready=(k % 3 != 0))
for k in range(7):
    g.enter_board(Obj("Watchful Sentry", op), op, "base", ready=(k % 2 == 0))
W["d"] = None
`);
  await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(500);
  ok("situation posée : 9 cartes ou plus dans ta base, 7 dans la base adverse", await ev("V.st.p[MEP].base.length >= 9 && V.st.p[OPP()].base.length >= 7"));
  ok("ta base est serrée (baseDense)", await ev("baseDense(document.querySelector('#board .half.p0 .zbase'))"));
  await pg.screenshot({ path: `${OUT}/base-serree-${w}.png` });
  // 1. premier toucher sur une unité à moitié cachée : ouvre la base, ne sélectionne rien
  await tap("#board .half.p0 .zbase > .slot:nth-of-type(4)", 4);
  let s = await st();
  ok(`premier toucher : la base s'ouvre en grand (cartes ${s.cw} px, bandeau ${JSON.stringify(s.head)}), sans menu ni sélection`, s.open && s.cw >= 80 && s.head.length === 1 && /Ta base\s*\d+/.test(s.head[0]) && !s.pop && s.sel === null);
  await pg.screenshot({ path: `${OUT}/base-ouverte-${w}.png` });
  const geo = await ev(`(()=>{ const z = document.querySelector('#board .half.p0 .zbase').getBoundingClientRect(), hd = document.querySelector('#board .bhead').getBoundingClientRect(), bf = document.querySelector('#board .bf').getBoundingClientRect();
    return { zt: z.top, hb: hd.bottom, zr: z.right, vw: innerWidth, bfb: bf.bottom, ht: hd.top } })()`);
  ok(`bandeau juste au-dessus de la rangée, dans l'écran, champs de bataille visibles (bandeau ${Math.round(geo.ht)}-${Math.round(geo.hb)}, rangée dès ${Math.round(geo.zt)}, bas des champs ${Math.round(geo.bfb)})`, Math.abs(geo.hb - geo.zt) <= 2 && geo.zr <= geo.vw + 1 && geo.ht >= geo.bfb - 40);
  ok(`base ouverte : elle défile (${s.sw} px de contenu pour ${s.cw2} px)`, s.sw > s.cw2 + 50);
  await ev("document.querySelector('#board .half.p0 .zbase').scrollLeft = 99999"); await pg.waitForTimeout(200);
  ok("base ouverte : la dernière carte se voit en entier après défilement", await ev("(()=>{ const z = document.querySelector('#board .half.p0 .zbase'), r = z.getBoundingClientRect(), ss = [...z.querySelectorAll(':scope > .slot')]; const c = ss[ss.length - 1].getBoundingClientRect(); return c.right <= r.right + 1 && c.left >= r.left - 1 })()"));
  // 2. toucher une unité prête : ses actions ; « Déplacer… » puis un champ de bataille, la base reste ouverte
  const uid = await ev("(()=>{ const c = document.querySelector('#board .half.p0 .zbase.open .card.can'); if (!c) return null; const z = c.closest('.zbase'); z.scrollLeft = c.closest('.slot').offsetLeft - 100; return c.dataset.uid })()");
  await pg.waitForTimeout(250);
  if (!uid) ok("base ouverte : une unité qu'on peut jouer", false);
  else {
    await tap(`#board [data-uid="${uid}"]`); s = await st();
    ok(`base ouverte : toucher une unité montre ses actions (${JSON.stringify(s.popT.slice(0, 60))}), la base reste ouverte`, s.open && /Déplacer/.test(s.popT));
    await pg.screenshot({ path: `${OUT}/base-menu-${w}.png` });
    await tap("#pop [data-mv]"); s = await st();
    ok(`« Déplacer… » : déplacement préparé (mode ${s.mode}), base toujours ouverte`, s.mode === "move" && s.open);
    await tap('#board .bf[data-drop="0"] .mid'); s = await st();
    ok(`toucher un champ de bataille : destination choisie (${s.dest}), base toujours ouverte`, s.mode === "move" && String(s.dest) === "0" && s.open);
    await ev("mode = null; sel = null; decorate()");
  }
  // 3. Fermer
  await tap("#board .bhead [data-bclose]"); s = await st();
  ok("« Fermer » referme la base", !s.open && !s.head.length);
  // 4. base adverse : s'ouvre, toucher une carte l'agrandit
  await tap("#board .half.p1 .zbase > .slot:nth-of-type(3)", 4); s = await st();
  ok(`base adverse serrée : le premier toucher l'ouvre (bandeau ${JSON.stringify(s.head)})`, s.oopen && !s.open && s.head.length === 1 && /Base de/.test(s.head[0]));
  await pg.screenshot({ path: `${OUT}/base-adverse-${w}.png` });
  await tap("#board .half.p1 .zbase.open > .slot:nth-of-type(2) .card"); s = await st();
  ok("base adverse ouverte : toucher une carte l'agrandit", s.zoom && s.oopen);
  await ev("Z.hidden = true");
  // 5. toucher ailleurs (un champ de bataille) referme
  await tap('#board .bf[data-drop="1"] .mid'); s = await st();
  ok("toucher un champ de bataille sans coup en cours referme la base", !s.oopen && !s.open && s.BASEOPEN === null);
  // 6. après un coup joué, la base se referme
  await tap("#board .half.p0 .zbase > .slot:nth-of-type(4)", 4);
  const u2 = await ev("(()=>{ const c = document.querySelector('#board .half.p0 .zbase.open .card.can'); if (!c) return null; c.closest('.zbase').scrollLeft = c.closest('.slot').offsetLeft - 100; return c.dataset.uid })()");
  await pg.waitForTimeout(250);
  if (u2) {
    const z = await box('#board .bf[data-drop="0"]'), r = await box(`#board [data-uid="${u2}"]`);
    await pg.mouse.move(r.x + r.width / 2, r.y + r.height / 2); await pg.mouse.down();
    await pg.mouse.move(r.x + r.width / 2, r.y - 30, { steps: 4 }); await pg.mouse.move(z.x + z.width / 2, z.y + z.height / 2, { steps: 8 }); await pg.mouse.up();
    await pg.waitForTimeout(300); s = await st();
    ok(`glisser une unité de la base ouverte vers un champ : déplacement préparé (mode ${s.mode}, destination ${s.dest})`, s.mode === "move" && String(s.dest) === "0");
    const n0 = await ev("V.st.p[MEP].base.length");
    await tap("#fbar #fOk");
    await waitFor("!!(V && !working)"); await pg.waitForTimeout(400);
    const after = await ev("({ n: V.st.p[MEP].base.length, open: BASEOPEN })");
    ok(`déplacement lancé : l'unité a quitté la base (${n0} → ${after.n}), base refermée`, after.n < n0 && after.open === null);
  } else ok("seconde ouverture : une unité qu'on peut jouer", false);
  // 7. base peu remplie : pas d'ouverture, le toucher sélectionne
  await py(`
g = W["g"]; me = ME
mine = [o for o in g.board if o.ctrl == me and o.loc == "base"]
keep = [o for o in mine if not o.__dict__.get("_exh")][:2]
for o in mine:
    if o not in keep: g.board.remove(o)
W["d"] = None
`);
  await ev("(async()=>{ await pump(J(T.step())) })()"); await waitFor("!!(V && !working)"); await pg.waitForTimeout(400);
  // (après le déplacement on est en showdown : le toucher agrandit la carte au lieu de montrer ses actions)
  ok("base peu remplie : pas serrée", !(await ev("baseDense(document.querySelector('#board .half.p0 .zbase'))")));
  await tap("#board .half.p0 .zbase > .slot:nth-of-type(1) .card"); s = await st();
  ok(`base peu remplie (${await ev("V.st.p[MEP].base.length")} unités) : pas d'ouverture, le toucher agit directement sur la carte (menu ${s.pop}, agrandi ${s.zoom})`, !s.open && !s.head.length && (s.pop || s.zoom));
  ok("aucune erreur JavaScript" + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
}
await b.close();
process.exitCode = fails ? 1 : 0;
