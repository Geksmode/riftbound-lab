import { existsSync } from "node:fs";
// Robot de l'interface inspirée de RiftAtlas (demande de l'utilisateur, 2026-11-24, captures de RiftAtlas) :
// - miniature de l'illustration sur chaque ligne de la liste du deck ;
// - vue « Stats » de l'éditeur : 40 cartes, énergie et Might moyennes justes, courbe d'énergie dont la somme vaut 40 et
//   dont chaque colonne compte les bonnes cartes, Might des unités, puissance par domaine, runes ; choix gardé au rechargement ;
// - sideboard entre deux manches : deux colonnes avec miniatures, → fait sortir, ← fait entrer, aperçu de la carte
//   (ordinateur), compteurs Main Deck / Sideboard, « Valider » seulement à échanges égaux (403.4).
// Joué deux fois : ordinateur 1500×900 (souris) et téléphone 393×851 (toucher simulé).
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_stats.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {});
let fails = 0;
const ok = (n, v) => { console.log((v ? "OK   " : "ÉCHEC") + " " + n); if (!v) fails++; };

async function run(W, H, mobile) {
  console.log(`\n=== ${W}×${H} ${mobile ? "(téléphone, toucher simulé)" : "(ordinateur, souris)"} ===`);
  const ctx = await b.newContext({ viewport: { width: W, height: H }, isMobile: mobile, hasTouch: mobile });
  const pg = await ctx.newPage(); const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(150); } throw new Error("attente : " + s); };
  const tapLoc = async loc => { await loc.scrollIntoViewIfNeeded(); await pg.waitForTimeout(120); const r = await loc.boundingBox(); if (mobile) await pg.touchscreen.tap(r.x + r.width / 2, r.y + r.height / 2); else await pg.mouse.click(r.x + r.width / 2, r.y + r.height / 2); await pg.waitForTimeout(250); };
  const tap = sel => tapLoc(pg.locator(sel).first());
  const noHScroll = () => ev("document.documentElement.scrollWidth <= innerWidth + 1");
  const open = async () => {
    await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=deck]');return b&&!b.disabled&&!!CAT&&!CHKLATER})()"); await pg.waitForTimeout(200);
    await tap("[data-m=deck]"); await waitFor("!!document.getElementById('dkSave')");
  };
  await pg.goto(`http://localhost:${PORT}/train.html`);
  await ev("localStorage.removeItem('rbt-dkview')");
  await pg.reload(); await open();
  // une liste d'Akali de tournoi avec 10 cartes de sideboard
  const key = await ev("(CAT.presets.find(p=>/akali/i.test(p.deck.legend)&&(p.deck.sideboard||[]).length===10)||{}).key");
  ok(`liste de tournoi d'Akali avec sideboard trouvée (${key})`, !!key);
  await pg.selectOption("#dkLoad", key); await waitFor("!!document.getElementById('dkSideH')");

  // 1. miniatures de la liste
  ok("Liste : chaque ligne de carte a sa miniature illustrée", await ev("(()=>{const L=[...document.querySelectorAll('.dkdeck .dkr')];return L.length>10&&L.every(r=>{const t=r.querySelector('.th > span');return t&&/background-image/.test(t.getAttribute('style')||'')})})()"));
  ok("Liste : miniatures à leur place (46×26, dans la ligne)", await ev("[...document.querySelectorAll('.dkdeck .dkr .th')].every(t=>{const r=t.getBoundingClientRect(),p=t.parentNode.getBoundingClientRect();return Math.abs(r.width-46)<1&&Math.abs(r.height-26)<1&&r.top>=p.top-1&&r.bottom<=p.bottom+1})"));
  await pg.screenshot({ path: `${OUT}/stats-liste-${W}.png` });

  // 2. vue Stats
  await tap("#dkVStat");
  const exp = await ev(`(()=>{const L=[];for(const[n,q]of Object.entries(DK.main))for(let i=0;i<q;i++)L.push(n);L.push(DK.champion);
    const C=n=>CAT.by[n],E=L.map(n=>C(n).e===""?null:+C(n).e).filter(v=>v!==null),U=L.filter(n=>C(n).t==="Unit"),M=U.map(n=>+C(n).mi);
    const cur={};for(const n of L){const e=+C(n).e||0;cur[e]=(cur[e]||0)+1;}
    return {n:L.length,ae:(E.reduce((a,b)=>a+b,0)/E.length).toFixed(1),am:(M.reduce((a,b)=>a+b,0)/M.length).toFixed(1),cur,units:U.length};})()`);
  const got = await ev(`(()=>{const c=[...document.querySelectorAll('#stCurve .stcol')].map(x=>[+x.dataset.e,+x.dataset.n]);const m=[...document.querySelectorAll('#stMight .stcol')].map(x=>+x.dataset.n);
    return {n:+document.querySelector('.sttile b').textContent,ae:document.querySelector('#stAvgE b').textContent,am:document.querySelector('#stAvgM b').textContent,cur:Object.fromEntries(c.filter(x=>x[1])),sum:c.reduce((a,x)=>a+x[1],0),mi:m.reduce((a,b)=>a+b,0)};})()`);
  ok(`Stats : ${got.n} cartes (attendu ${exp.n}, Chosen Champion compris)`, got.n === exp.n && exp.n === 40);
  ok(`Stats : énergie moyenne ${got.ae} (attendu ${exp.ae}), Might moyenne ${got.am} (attendu ${exp.am})`, got.ae === exp.ae && got.am === exp.am);
  ok(`Stats : courbe d'énergie = ${JSON.stringify(got.cur)} (somme ${got.sum})`, got.sum === 40 && JSON.stringify(got.cur) === JSON.stringify(exp.cur));
  ok(`Stats : Might des unités, ${got.mi} unités (attendu ${exp.units})`, got.mi === exp.units);
  ok("Stats : barres de la courbe dans leur cadre, la plus haute est la plus grande", await ev(`(()=>{const C=[...document.querySelectorAll('#stCurve .stcol')];const f=document.getElementById('stCurve').getBoundingClientRect();
    const hs=C.map(c=>c.querySelector('.stbar').getBoundingClientRect().height),ns=C.map(c=>+c.dataset.n);const mx=Math.max(...ns);
    return C.every(c=>{const r=c.getBoundingClientRect();return r.top>=f.top-1&&r.bottom<=f.bottom+1})&&hs[ns.indexOf(mx)]===Math.max(...hs)&&hs.every((h,i)=>ns[i]||h<1)})()`));
  ok("Stats : champion en pointillé dans sa colonne de coût", await ev("(()=>{const e=+CAT.by[DK.champion].e;const c=document.querySelector(`#stCurve .stcol[data-e='${e}'] i.ch`);return !!c})()"));
  ok("Stats : runes du deck = 12 réparties dans la barre Runes", await ev("[...document.querySelectorAll('#stPow .stpbar')][1].querySelectorAll('span').length >= 1 && Object.values(DK.runes).reduce((a,b)=>a+b,0) === 12"));
  ok("Stats : tableau puissance/runes, une ligne par domaine de la légende", await ev("(()=>{const r=[...document.querySelectorAll('.sttab tbody tr')].map(t=>t.cells[0].textContent.trim());return CAT.by[DK.legend].d.every(d=>r.some(x=>x.endsWith(d)))})()"));
  ok("Stats : pas de défilement horizontal", await noHScroll());
  await pg.screenshot({ path: `${OUT}/stats-${W}.png` });
  await pg.reload(); await open();
  ok("Stats : choix gardé après rechargement", await ev("DKV === 'stats' && document.getElementById('dkVStat').getAttribute('aria-pressed') === 'true'"));
  await pg.selectOption("#dkLoad", key); await waitFor("!!DK.legend && !!document.getElementById('stCurve')");
  await tap("#dkVList");
  ok("Liste : retour à la liste", await ev("document.querySelectorAll('.dkdeck .dkr').length > 10 && !document.getElementById('stCurve')"));

  // 3. sideboard entre deux manches (même fenêtre que le BO3 : sbOpen avec un contexte de test)
  await ev(`(()=>{window.__sb=null;sbOpen({deck:toEngine(DK),game:2,score:"1-0",done:(nd,t)=>{window.__sb=[nd,t];closeModal();}})})()`);
  await waitFor("!!document.getElementById('sbOk')");
  ok("Sideboard : deux colonnes, miniature sur chaque ligne", await ev("document.querySelectorAll('#mbox .sbcol').length===2&&[...document.querySelectorAll('#mbox .sbr')].every(r=>!!r.querySelector('.th > span'))"));
  ok("Sideboard : compteurs 39/39 et 10/10", await ev("[...document.querySelectorAll('#mbox .sbcol .grp b')].map(x=>x.textContent).join()==='39/39,10/10'"));
  if (!mobile) {
    ok("Sideboard (ordinateur) : aperçu de la carte affiché", await ev("!!document.getElementById('sbPrev') && getComputedStyle(document.getElementById('sbPrev')).display!=='none'"));
    const n2 = await ev("document.querySelectorAll('#mbox [data-sbp=out]')[3].dataset.n");
    await pg.hover(`#mbox .sbr[data-pv="${n2}"] .nm`); await pg.waitForTimeout(300);
    ok(`Sideboard (ordinateur) : survoler « ${n2} » l'affiche dans l'aperçu`, await ev(`document.querySelector('#sbPrev .t').textContent === ${JSON.stringify(n2)}`));
  }
  ok("Sideboard : Valider bloqué sans échange", await ev("document.getElementById('sbOk').disabled"));
  const out = await ev("document.querySelector('[data-sbp=out]').dataset.n"), inn = await ev("document.querySelector('[data-sbp=inn]').dataset.n");
  await tap(`[data-sbp="out"][data-n="${out}"]`);
  ok(`Sideboard : → fait sortir « ${out} » (38/39, 11/10), Valider encore bloqué`, await ev("[...document.querySelectorAll('#mbox .sbcol .grp b')].map(x=>x.textContent).join()==='38/39,11/10' && document.getElementById('sbOk').disabled"));
  await tap(`[data-sbp="inn"][data-n="${inn}"]`);
  ok(`Sideboard : ← fait entrer « ${inn} » (39/39, 10/10), Valider actif`, await ev("[...document.querySelectorAll('#mbox .sbcol .grp b')].map(x=>x.textContent).join()==='39/39,10/10' && !document.getElementById('sbOk').disabled"));
  const sz = await ev("[...document.querySelectorAll('#mbox .sbr button:not(:disabled), #sbOk, #sbKeep')].map(x=>{const r=x.getBoundingClientRect();return Math.min(r.width,r.height)}).filter(v=>v>0)");
  ok(`Sideboard : boutons ≥ 44 px (plus petit ${Math.min(...sz)} px)`, Math.min(...sz) >= 44);
  ok("Sideboard : noms jamais coupés par les boutons (ligne dans sa colonne)", await ev("[...document.querySelectorAll('#mbox .sbr')].every(r=>{const a=r.getBoundingClientRect(),c=r.parentNode.getBoundingClientRect();return a.right<=c.right+1&&a.left>=c.left-1})"));
  ok("Sideboard : pas de défilement horizontal", await noHScroll());
  await pg.screenshot({ path: `${OUT}/stats-sideboard-${W}.png` });
  await tap("#sbOk"); await waitFor("!!window.__sb", 20000);
  ok(`Sideboard : deck rendu par le moteur (− ${out}, + ${inn})`, await ev(`(()=>{const [nd]=window.__sb,d=toEngine(DK),c=(L,n)=>L.filter(x=>x===n).length;return nd.main.length===d.main.length&&c(nd.main,${JSON.stringify(inn)})===c(d.main,${JSON.stringify(inn)})+1&&c(nd.main,${JSON.stringify(out)})===c(d.main,${JSON.stringify(out)})-1})()`));
  ok(`aucune erreur de page (${errs.length ? errs[0] : "0"})`, !errs.length);
  await ctx.close();
}
await run(1500, 900, false);
await run(393, 851, true);
await b.close();
console.log(fails ? `\n${fails} ÉCHEC(S)` : "\nTout est OK");
process.exit(fails ? 1 : 0);
