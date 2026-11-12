import { existsSync } from "node:fs";
// Robot de la vue « Images » de l'éditeur de deck (demande de l'utilisateur, 2026-11-12 : une page comme une decklist
// étalée, un exemplaire = une image). Copie de LeBlanc IQ#5, bouton « Images » : 40 images au deck principal (champion
// en premier, puis par coût), 10 au sideboard, légende, 3 battlefields ; menu d'une image (− / + / ⇄ / Voir la carte /
// Fermer) ; masquer le catalogue ; choix gardé après rechargement ; retour à la liste.
// Joué deux fois : ordinateur 1400×900 (souris) et téléphone 393×851 (toucher simulé).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_deckimg.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0;
const ok = (n, v) => { console.log((v ? "OK   " : "ÉCHEC") + " " + n); if (!v) fails++; };

async function run(W, H, mobile) {
  console.log(`\n=== ${W}×${H} ${mobile ? "(téléphone, toucher simulé)" : "(ordinateur, souris)"} ===`);
  const ctx = await b.newContext({ viewport: { width: W, height: H }, isMobile: mobile, hasTouch: mobile, deviceScaleFactor: 2 });
  const pg = await ctx.newPage(); const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(150); } throw new Error("attente : " + s); };
  const tapLoc = async loc => { await loc.scrollIntoViewIfNeeded(); await pg.waitForTimeout(120); const r = await loc.boundingBox(); if (mobile) await pg.touchscreen.tap(r.x + r.width / 2, r.y + r.height / 2); else await pg.mouse.click(r.x + r.width / 2, r.y + r.height / 2); await pg.waitForTimeout(250); };
  const tap = sel => tapLoc(pg.locator(sel).first());
  const popBtn = re => tapLoc(pg.locator("#dkPop button").filter({ hasText: re }).first());
  const n = z => ev(`document.querySelectorAll('#deckView .dkv[data-vz="${z}"]').length`);
  const popOpen = () => ev("!document.getElementById('dkPop').hidden");
  const popInView = () => ev("(()=>{const r=document.getElementById('dkPop').getBoundingClientRect();return r.left>=0&&r.top>=0&&r.right<=innerWidth&&r.bottom<=innerHeight})()");
  const noHScroll = () => ev("document.documentElement.scrollWidth <= innerWidth + 1");
  const open = async () => {
    await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=deck]');return b&&!b.disabled&&!!CAT&&!CHKLATER})()"); await pg.waitForTimeout(200);
    await tap("[data-m=deck]"); await waitFor("!!document.getElementById('dkSave')");
    await pg.selectOption("#dkLoad", "leblanc-iq5"); await waitFor("!!document.getElementById('dkSideH')");
  };

  await pg.goto(`http://localhost:${PORT}/train.html`);
  await open();
  ok("liste par défaut (pas d'image du deck, lignes de la liste)", await n("main") === 0 && await ev("document.querySelectorAll('.dkdeck .dkr').length > 10"));
  await tap("#dkVImg");
  const counts = [await n("champ"), await n("main"), await n("side"), await n("leg"), await n("bf")];
  ok(`Images : 1 champion + 39 au deck principal, 10 au sideboard, 1 légende, 3 battlefields (${counts.join(", ")})`, counts.join() === "1,39,10,1,3");
  ok("Images : le champion choisi est la première image du deck principal", await ev("(()=>{const g=document.querySelector('#deckView .dkv[data-vz=champ]');return !!g&&g===g.parentNode.firstElementChild&&g.dataset.vn===DK.champion})()"));
  ok("Images : deck principal rangé par coût croissant", await ev("(()=>{const e=[...document.querySelectorAll('#deckView .dkv[data-vz=main]')].map(x=>+CAT.by[x.dataset.vn].e||0);return e.every((v,i)=>!i||e[i-1]<=v)})()"));
  ok("Images : chaque image a son illustration", await ev("[...document.querySelectorAll('#deckView .dkv .dkimg')].every(x=>/background-image/.test(x.getAttribute('style')||''))"));
  ok("Images : pas de défilement horizontal", await noHScroll());
  if (!mobile) ok("Images (ordinateur) : au moins 6 images par rangée", await ev("(()=>{const L=[...document.querySelectorAll('#deckView .dkv[data-vz=main]')];const y=L[0].getBoundingClientRect().top;return L.filter(x=>Math.abs(x.getBoundingClientRect().top-y)<2).length>=6})()"));
  await pg.screenshot({ path: `${OUT}/deck-images-${W}.png` });

  // menu d'une image du deck principal : − puis +
  const name = await ev("(()=>{const q=Object.entries(DK.main).find(([n,q])=>q>=2&&n!==DK.champion);return q[0]})()");
  const before = await ev(`DK.main[${JSON.stringify(name)}]`);
  await tapLoc(pg.locator(`#deckView .dkv[data-vz=main][data-vn="${name.replace(/"/g, '\\"')}"]`).first());
  ok(`toucher « ${name} » ouvre son menu, entièrement à l'écran`, await popOpen() && await popInView() && (await ev("document.querySelector('#dkPop .t').textContent")) === name);
  if (mobile) { const sz = await ev("[...document.querySelectorAll('#dkPop button')].map(x=>x.getBoundingClientRect().height)"); ok(`menu : boutons ≥ 44 px (plus petit ${Math.min(...sz)} px)`, Math.min(...sz) >= 44); }
  await pg.screenshot({ path: `${OUT}/deck-images-menu-${W}.png` });
  await popBtn(/^− Retirer$/);
  ok(`− Retirer : ${before} → ${before - 1} exemplaires, 39 cartes, menu resté ouvert sur la carte`, await ev(`DK.main[${JSON.stringify(name)}]`) === before - 1 && await n("main") === 38 && await popOpen());
  await popBtn(/^\+ Ajouter$/);
  ok("+ Ajouter : retour à 40 cartes", await ev(`DK.main[${JSON.stringify(name)}]`) === before && await n("main") === 39);
  await popBtn(/^Voir la carte$/);
  ok("Voir la carte : l'agrandi de la carte s'affiche", await ev(`!document.getElementById('zoom').hidden && document.querySelector('#zoom h4').textContent === ${JSON.stringify(name)}`));
  await popBtn(/^Fermer$/);
  ok("Fermer : le menu se ferme", !(await popOpen()));

  // sideboard → deck → sideboard
  const sname = await ev("document.querySelector('#deckView .dkv[data-vz=side]').dataset.vn");
  await tap("#deckView .dkv[data-vz=side]");
  await popBtn(/Passer au deck principal/);
  ok(`⇄ « ${sname} » passe du sideboard au deck (sideboard 9, deck 41)`, await n("side") === 9 && await ev("cnt(DK.main)+(DK.champion?1:0)") === 41);
  if (await popOpen()) await popBtn(/^Fermer$/);
  await tapLoc(pg.locator(`#deckView .dkv[data-vz=main][data-vn="${sname.replace(/"/g, '\\"')}"]`).first());
  await popBtn(/Passer au sideboard/);
  ok("⇄ la remet au sideboard (10 / 40)", await n("side") === 10 && await ev("cnt(DK.main)+(DK.champion?1:0)") === 40);
  // toucher ailleurs ferme le menu
  if (await popOpen()) { await tap(".dkh"); }
  ok("toucher ailleurs ferme le menu", !(await popOpen()));

  // masquer le catalogue
  await tap("#dkCatT");
  ok("Masquer le catalogue : le deck prend toute la largeur", await ev("getComputedStyle(document.querySelector('.dkcat')).display==='none' && document.querySelector('.dkdeck').getBoundingClientRect().width >= innerWidth - 2"));
  ok("catalogue masqué : pas de défilement horizontal", await noHScroll());
  await pg.screenshot({ path: `${OUT}/deck-images-plein-${W}.png` });
  await tap("#dkCatT");
  ok("Afficher le catalogue : il revient", await ev("getComputedStyle(document.querySelector('.dkcat')).display!=='none'"));

  // choix gardé après rechargement
  await pg.reload(); await open();
  ok("après rechargement : la vue Images est gardée", await n("main") === 39 && await ev("document.getElementById('dkVImg').getAttribute('aria-pressed')==='true'"));
  await tap("#dkVList");
  ok("Liste : retour à la liste", await n("main") === 0 && await ev("document.querySelectorAll('.dkdeck .dkr').length > 10"));
  ok(`aucune erreur JS${errs.length ? " : " + errs.slice(0, 2).join(" | ") : ""}`, errs.length === 0);
  await ctx.close();
}

try { await run(1400, 900, false); await run(393, 851, true); }
catch (e) { console.log("ÉCHEC exception : " + (e && e.stack || e)); fails++; }
await b.close();
console.log(fails ? `\n${fails} ÉCHEC(S)` : "\nverif_deckimg : tout OK");
process.exit(fails ? 1 : 0);
