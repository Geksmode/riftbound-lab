import { existsSync } from "node:fs";
// Playwright : celui de l'environnement cloud Claude, sinon le paquet npm « playwright » (CI).
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
// Robot : bouton « ⚑ Erreur de l'IA » (2026-10-28). Partie graine 12, l'humain finit ses tours jusqu'au tour 7 ; le
// bouton marque le dernier vrai choix de l'IA (au moins deux options), « Autre erreur » exige une précision écrite,
// la marque et les coups de l'IA (champ « ai ») sont gardés avec la partie. Téléphone 360×740 (toucher) puis ordinateur.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_drapeau.mjs <dossier des captures> [port]
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0;
const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
for (const phone of [true, false]) {
  const tag = phone ? "téléphone" : "ordinateur";
  const ctx = await b.newContext(phone ? { viewport: { width: 360, height: 740 }, isMobile: true, hasTouch: true } : { viewport: { width: 1366, height: 800 } });
  const pg = await ctx.newPage(); const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const tap = sel => phone ? pg.tap(sel) : pg.click(sel);
  const waitFor = async (s, ms = 300000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(150); } throw new Error("attente : " + s); };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  await tap("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await ev("document.getElementById('sSeed').value='12'"); await tap("#bStart");
  await waitFor("!!document.getElementById('bKeep')"); await tap("#bKeep");
  ok(`${tag} : bouton grisé avant tout coup de l'IA`, await ev("document.getElementById('bFlag').disabled || !lastAIlab"));
  // l'humain finit ses tours (ou passe) jusqu'au tour 7 : l'IA a joué de vrais choix entre-temps
  for (let k = 0; k < 40; k++) {
    await waitFor("!working && V && (V.dec || V.ask)");
    if (await ev("V.st.t >= 7 && !!V.dec")) break;
    await ev("(()=>{ if (V.ask) return answer(V.ask.kind === 'mulligan' ? [] : 0); const o = V.dec.options; const e = o.find(x => x.k === 'end') || o.find(x => x.k === 'pass') || o[0]; choose(e.i); })()");
    await pg.waitForTimeout(200);
  }
  await waitFor("!working && V && V.dec");
  const box = await ev("(()=>{ const r = document.getElementById('bFlag').getBoundingClientRect(); return { l: r.left, r: r.right, t: r.top, b: r.bottom, clip: document.getElementById('bFlag').scrollWidth > document.getElementById('bFlag').clientWidth + 1 } })()");
  ok(`${tag} : bouton à l'écran, rien de coupé (${JSON.stringify(box)})`, box.l >= 0 && box.r <= (phone ? 360 : 1366) + 1 && box.b <= (phone ? 740 : 800) + 1 && !box.clip);
  ok(`${tag} : bouton actif après un coup de l'IA`, await ev("!document.getElementById('bFlag').disabled"));
  await tap("#bFlag"); await waitFor("!document.getElementById('fpop').hidden", 30000);
  const f = await ev("J(T.flag())");
  const shown = await ev("document.querySelector('#fpop .small').textContent");
  ok(`${tag} : la bulle montre le coup qui sera marqué (${shown})`, !!f.label && shown.endsWith(f.label));
  ok(`${tag} : ce coup est un vrai choix de l'IA`, await ev(`J(T.ai_script()).then(a => a[${f.step}][0] === "d" && a[${f.step}][3] > 1)`));
  ok(`${tag} : la bulle tient à l'écran`, await ev("(()=>{ const r = document.getElementById('fpop').getBoundingClientRect(); return r.left >= 0 && r.right <= innerWidth + 1 && r.top >= 0 && r.bottom <= innerHeight + 1 })()"));
  await pg.screenshot({ path: `${OUT}/drapeau-${phone ? 360 : 1366}.png` });
  await tap("#fpop [data-f=autre]");
  ok(`${tag} : « Autre erreur » ouvre le champ de précision`, await ev("!document.querySelector('#fpop .flnote').hidden"));
  await tap("#flOk"); await pg.waitForTimeout(200);
  ok(`${tag} : sans précision, rien n'est marqué`, await ev("!document.getElementById('fpop').hidden && !REC.moves.some(m => m[0] === 'flag')"));
  await pg.fill("#flNote", "il devait attaquer");
  await pg.keyboard.press("Enter"); await pg.waitForTimeout(300);
  ok(`${tag} : marque gardée avec sa précision`, await ev(`REC.moves.some(m => m[0] === "flag" && m[1] === "autre" && m[3] === ${f.step} && m[5] === "il devait attaquer")`));
  await tap("#bFlag"); await waitFor("!document.getElementById('fpop').hidden", 30000);
  await tap("#fpop [data-f=cible]"); await pg.waitForTimeout(300);
  ok(`${tag} : un coup déjà marqué ne l'est pas deux fois`, await ev("REC.moves.filter(m => m[0] === 'flag').length === 1"));
  await pg.waitForTimeout(3500);
  ok(`${tag} : la partie gardée porte les coups de l'IA et la marque`, await ev("(()=>{ const g = JSON.parse(localStorage.getItem('rbt-games') || '{}')[REC.id] || {}; return (g.ai || []).length > 1 && (g.moves || []).some(m => m[0] === 'flag') })()"));
  ok(`${tag} : aucune erreur JavaScript` + (errs.length ? " : " + errs.slice(0, 2).join(" / ") : ""), errs.length === 0);
  await ctx.close();
}
await b.close();
process.exitCode = fails ? 1 : 0;
