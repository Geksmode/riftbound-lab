import { existsSync } from "node:fs";
// Robot du confort de la table (2026-11-24, d'après Rift Atlas et TCG Arena), sur la vraie page :
// « ? » = liste des raccourcis ; Ctrl+K et bouton ⌕ = recherche d'une carte (flèches, Entrée, fiche) ; J = journal ;
// chiffres = action numérotée du menu d'une carte et réponse d'une bulle ; Entrée = « Valider la cible » ; C = conseil ;
// clic droit = fiche en grand, sans rien jouer ; éditeur : filtre de coût, tris, groupes du deck, alerte avant d'écraser
// un deck modifié ; version anglaise ; téléphone : le bouton ⌕ tient dans la barre.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_qol.mjs <dossier des journaux (inutilisé)> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const PORT = process.argv[3] || "8771";
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0; const ok = (n, v) => { if (!v) fails++; console.log((v ? "OK   " : "ÉCHEC") + " " + n); };
const errs = [];
let pg, ev, waitFor;
async function open(ctxOpts, url = "train.html") {
  pg = await (await b.newContext(ctxOpts)).newPage();
  pg.on("pageerror", e => errs.push(String(e)));
  ev = s => pg.evaluate(s);
  waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(100); } throw new Error("attente : " + s); };
  await pg.goto(`http://localhost:${PORT}/${url}`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
}
const modalTxt = () => ev("$('modal').hidden ? '' : $('mbox').innerText");
const answer0 = () => ev("(async()=>{ await pump(J(T.answer(JSON.stringify(0)))) })()");
const idle = () => waitFor("!!(V && !working && !V.busy)");

// ------------------------------------------------------------ ordinateur, menu
await open({ viewport: { width: 1400, height: 900 } });
await pg.keyboard.press("Shift+?"); await pg.waitForTimeout(200);
ok("« ? » sur le menu : la liste des raccourcis s'ouvre", /Raccourcis clavier/.test(await modalTxt()) && /Ctrl/.test(await modalTxt()));
await pg.keyboard.press("Escape"); await pg.waitForTimeout(200);
ok("Échap : la liste se ferme, le menu est toujours là", await ev("$('modal').hidden && !$('menu').hidden"));
await pg.keyboard.press("Control+k"); await waitFor("!!document.getElementById('fdQ')");
ok("Ctrl+K : la recherche s'ouvre, le champ a le focus", await ev("document.activeElement === $('fdQ')"));
await pg.keyboard.type("akali"); await waitFor("document.querySelectorAll('#fdRes [data-fd]').length > 0");
const names = await ev("[...document.querySelectorAll('#fdRes .nm')].map(x => x.textContent)");
ok(`recherche « akali » : les cartes dont le nom commence par Akali d'abord (${names.join(", ")})`, names.length >= 4 && names.slice(0, 4).every(n => n.startsWith("Akali")));
await pg.keyboard.press("ArrowDown"); await pg.keyboard.press("Enter"); await pg.waitForTimeout(200);
ok("flèche bas puis Entrée : la fiche de la 2e carte s'affiche avec son texte", await ev(`(() => { const s = document.querySelector('#fdCard .csheet'); return !!s && s.dataset.sheet === ${JSON.stringify(names[1])} && s.querySelector('.zt').textContent.length > 20 && !!s.querySelector('img, .zspr'); })()`));
await pg.keyboard.type("j"); await pg.waitForTimeout(100);
ok("taper « j » dans le champ de recherche n'ouvre pas le journal", await ev("!$('drawer').classList.contains('open') && $('fdQ').value === 'akalij'"));
await pg.keyboard.press("Escape"); await pg.waitForTimeout(200);

// ------------------------------------------------------------ partie : donne 4, Akali contre LeBlanc
await pg.click("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
await ev("document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'");
await pg.click("#bStart");
for (let i = 0; i < 200; i++) { await waitFor("!!(V && !working)"); if (await ev("!!document.getElementById('bKeep')")) { await pg.click("#bKeep"); continue; } if (await ev("!!V.ask")) { await answer0(); continue; } if (await ev("!!V.dec")) break; await pg.waitForTimeout(100); }
// avance jusqu'à une phase principale qui offre un sort à une cible (Back Off, Block…) : il sert à Entrée
const tgt = "V.dec.options.find(o => o.k === 'play' && (o.t1 || []).length === 1 && document.querySelector(`#board .half.p0 .hand [data-uid=\"${o.src}\"]`) && unitEl(o.t1[0]))";
let first = true, found = false;
for (let turn = 0; turn < 14 && !found; turn++) {
  for (let i = 0; i < 300; i++) {
    await idle();
    if (await ev("!!V.ask")) { await answer0(); continue; }
    if (!await ev("!!V.dec")) { await pg.waitForTimeout(100); continue; }
    if (await ev("V.dec.kind === 'main' && V.dec.options.some(o => o.k !== 'end' && o.k !== 'pass')")) break;
    await ev("(async()=>{ const o = V.dec.options.find(o => o.k === 'end' || o.k === 'pass') || V.dec.options[0]; await choose(o.i) })()");
  }
  if (first) {
    first = false;
    const snap = "JSON.stringify([V.st, V.dec && V.dec.options.map(o => o.label)])", s0 = await ev(snap);
    await pg.keyboard.press("j"); await pg.waitForTimeout(300);
    const o1 = await ev("$('drawer').classList.contains('open')");
    const sticky = await ev("(() => { const t = document.querySelector('#log .turn'); return !!t && getComputedStyle(t).position === 'sticky'; })()");
    await pg.keyboard.press("j"); await pg.waitForTimeout(300);
    ok("J : ouvre puis referme le journal ; les titres « Tour N » restent en haut du journal", o1 && sticky && await ev("!$('drawer').classList.contains('open')"));
    // clic droit sur une carte de la main : fiche en grand, rien n'est joué ni choisi
    const cn = await ev("document.querySelector('#board .half.p0 .hand .card').dataset.card");
    await pg.locator("#board .half.p0 .hand .card").first().click({ button: "right" }); await pg.waitForTimeout(200);
    ok(`clic droit sur « ${cn} » : sa fiche s'ouvre en grand`, await ev(`(() => { const s = document.querySelector('#mbox .csheet'); return !!s && s.dataset.sheet === ${JSON.stringify(cn)}; })()`));
    await pg.keyboard.press("Escape"); await pg.waitForTimeout(200);
    ok("Échap : la fiche se ferme, la partie n'a pas bougé, aucune carte choisie", await ev("$('modal').hidden && sel === null && !mode") && await ev(snap) === s0);
    // chiffre : l'action numérotée du menu d'une carte jouable
    await pg.locator("#board .half.p0 .hand .card.can").first().click(); await pg.waitForTimeout(200);
    const pop = await ev("$('pop').hidden ? null : [...$('pop').querySelectorAll('button')].map(b => [b.dataset.kb, b.textContent])");
    ok(`menu d'une carte : boutons numérotés (${JSON.stringify(pop)})`, !!pop && pop.length >= 2 && pop.every((x, i) => x[0] === String(i + 1)));
    await pg.keyboard.press("Digit" + pop.length); await pg.waitForTimeout(250);   // le dernier bouton : « Lire la carte »
    ok(`chiffre ${pop.length} (« ${pop[pop.length - 1][1]} ») : même effet que le clic, rien n'est joué`, await ev(snap) === s0 && await ev("!$('zoom').hidden || !$('modal').hidden || $('pop').hidden"));
    await pg.keyboard.press("Escape"); await pg.waitForTimeout(150); await pg.keyboard.press("Escape"); await pg.waitForTimeout(150);
    // C : conseil
    await pg.keyboard.press("c"); await waitFor("!$('hpop').hidden && document.querySelectorAll('#hpop .hintrow').length > 0", 240000);
    ok("C : le conseil s'affiche", true);
    await pg.keyboard.press("Escape"); await idle();
    // bulle oui / non : le chiffre 2 répond « Non »
    await ev("(async()=>{ await pump(J(T.dbg(JSON.stringify([['ask_may', 'T_NO']])))) })()"); await waitFor("!!(V && !working && V.ask)");
    const kb = await ev("[...document.querySelectorAll('#fbar .prow button')].map(b => b.dataset.kb + ':' + b.textContent)");
    await pg.keyboard.press("Digit2"); await waitFor("!!(V && !working && !V.ask)");
    ok(`bulle « Utiliser cet effet ? » (${kb.join(", ")}) : le chiffre 2 répond Non`, JSON.stringify(await ev("pyGet('T_NO')")) === "[false]");
    await idle();
  }
  if (await ev(`!!(V.dec && ${tgt})`)) {
    found = true;
    const o = await ev(`(() => { const o = ${tgt}; return { src: o.src, t: o.t1[0], label: o.label }; })()`);
    const h0 = await ev("V.st.p[MEP] ? 0 : 0"), opts0 = await ev("JSON.stringify(V.dec.options.map(o => o.label))");
    await pg.locator(`#board .half.p0 .hand [data-uid="${o.src}"]`).first().click(); await pg.waitForTimeout(200);
    const pop = await ev("$('pop').hidden ? [] : [...$('pop').querySelectorAll('button')].map(b => b.textContent)");
    const ci = pop.findIndex(t => /Cibler/.test(t));
    await pg.keyboard.press("Digit" + (ci + 1)); await pg.waitForTimeout(250);
    const inT = await ev("!!mode && mode.type === 'target'");
    await ev(`unitEl(${JSON.stringify(String(o.t))}).click()`); await pg.waitForTimeout(250);
    const ready = await ev("(() => { const b = document.getElementById('fOk'); return !!b && !b.disabled; })()");
    await pg.keyboard.press("Enter"); await idle();
    for (let i = 0; i < 20 && await ev("!!V.ask"); i++) { await answer0(); await idle(); }
    ok(`« ${o.label} » : chiffre ${ci + 1} = Cibler (${JSON.stringify(pop)}), cible touchée, Entrée valide et le sort part`, ci >= 0 && inT && ready && await ev("!mode") && await ev("JSON.stringify((V.dec || {options: []}).options.map(o => o.label))") !== opts0);
    break;
  }
  await ev("(async()=>{ const o = V.dec.options.find(o => o.k === 'play' && !(o.t1 || []).length) || V.dec.options.find(o => o.k === 'end'); await choose(o.i) })()");
  await idle();
  for (let i = 0; i < 20 && await ev("!!V.ask"); i++) { await answer0(); await idle(); }
  if (await ev("!!V.dec && V.dec.kind === 'main' && V.dec.options.some(o => o.k === 'end')")) await ev("(async()=>{ await choose(V.dec.options.find(o => o.k === 'end').i) })()");
}
ok("un sort à une cible a été trouvé pour essayer Entrée", found);

// ------------------------------------------------------------ éditeur de deck
await pg.click("#bDecks"); await waitFor("!$('deckView').hidden && !!document.getElementById('dkCost')");
await ev("DKF.mine = false; DKF.only = false; dkRender()");
await pg.selectOption("#dkCost", "2"); await pg.waitForTimeout(200);
const c2 = await ev("[...document.querySelectorAll('.dkgrid .dkc')].map(x => CAT.by[x.dataset.card]).map(c => [c.t, c.e])");
ok(`filtre « Coût 2 » : ${c2.length} cartes affichées, toutes à 2 énergie, ni légende ni rune ni battlefield`, c2.length > 20 && c2.every(([t, e]) => +e === 2 && e !== "" && !["Legend", "Battlefield", "Rune"].includes(t)));
await pg.selectOption("#dkCost", "7"); await pg.waitForTimeout(200);
const c7 = await ev("[...document.querySelectorAll('.dkgrid .dkc')].map(x => +CAT.by[x.dataset.card].e)");
ok(`filtre « Coût 7+ » : ${c7.length} cartes, toutes à 7 énergie ou plus`, c7.length > 0 && c7.every(e => e >= 7));
await pg.selectOption("#dkCost", ""); await pg.selectOption("#dkSort", "n"); await pg.waitForTimeout(200);
const byN = await ev("[...document.querySelectorAll('.dkgrid .dkc')].map(x => x.dataset.card)");
ok("tri par nom : ordre alphabétique", byN.length === 120 && byN.every((n, i) => !i || byN[i - 1].localeCompare(n) <= 0));
await pg.selectOption("#dkSort", "mi"); await pg.waitForTimeout(200);
const byM = await ev("[...document.querySelectorAll('.dkgrid .dkc')].map(x => CAT.by[x.dataset.card].mi).map(v => v === '' || v == null ? -1 : +v)");
ok(`tri par Might : décroissant (de ${byM[0]} à ${byM[byM.length - 1]})`, byM[0] >= 8 && byM.every((v, i) => !i || byM[i - 1] >= v));
await pg.selectOption("#dkSort", "");
// deck de référence, groupes
await pg.selectOption("#dkLoad", "akali-g2"); await pg.waitForTimeout(300);
await ev("DKV = 'list'; dkRender()");
const gType = await ev("[...document.querySelectorAll('.dklist .dkg')].map(x => x.textContent)");
await pg.selectOption("#dkGrp", "e"); await pg.waitForTimeout(200);
const gE = await ev("[...document.querySelectorAll('.dklist .dkg')].map(x => x.textContent)");
const sumE = gE.reduce((a, t) => a + +(/\((\d+)\)/.exec(t) || [0, 0])[1], 0);
ok(`groupes par énergie (${gE.join(", ")}) : 39 cartes en tout, groupes par coût croissant`, gE.length >= 3 && gE.every(t => /^Énergie \d\+? \(\d+\)$/.test(t)) && sumE === 39 && gE.every((t, i) => !i || parseInt(gE[i - 1].slice(8)) < parseInt(t.slice(8))));
await pg.selectOption("#dkGrp", "dom"); await pg.waitForTimeout(200);
const gD = await ev("[...document.querySelectorAll('.dklist .dkg')].map(x => x.textContent)");
ok(`groupes par domaine (${gD.join(", ")}) : 39 cartes en tout`, gD.length >= 2 && gD.reduce((a, t) => a + +(/\((\d+)\)$/.exec(t) || [0, 0])[1], 0) === 39);
await pg.selectOption("#dkGrp", "type"); await pg.waitForTimeout(200);
ok(`groupes par type, comme avant (${gType.join(", ")})`, JSON.stringify(await ev("[...document.querySelectorAll('.dklist .dkg')].map(x => x.textContent)")) === JSON.stringify(gType) && /^Unités/.test(gType[0]));
// deck modifié : marque « non enregistré », et alerte avant « Nouveau »
ok("deck tout juste ouvert : pas de marque « non enregistré »", await ev("$('dkUns').hidden"));
await pg.locator(".dklist [data-sub]").nth(1).click(); await pg.waitForTimeout(200);
ok("une carte retirée : la marque « non enregistré » apparaît", await ev("!$('dkUns').hidden"));
const sig = await ev("dkSig(DK)");
let asked = "";
pg.once("dialog", d => { asked = d.message(); d.dismiss(); });
await pg.click("#dkNew"); await pg.waitForTimeout(300);
ok(`« Nouveau » sur un deck modifié : question posée (« ${asked} ») ; refus = le deck est gardé`, /non enregistrées/.test(asked) && await ev("dkSig(DK)") === sig);
pg.once("dialog", d => d.accept());
await pg.click("#dkNew"); await pg.waitForTimeout(300);
ok("« Nouveau » accepté : deck vide, plus de marque", await ev("!DK.legend && !Object.keys(DK.main).length && $('dkUns').hidden"));
let asked2 = false; const h = () => { asked2 = true; }; pg.on("dialog", h);
await pg.click("#dkNew"); await pg.waitForTimeout(200); pg.off("dialog", h);
ok("« Nouveau » sur un deck non modifié : aucune question", !asked2);
ok("aucune erreur de page (ordinateur)", errs.length === 0); if (errs.length) console.log(errs.join("\n"));

// ------------------------------------------------------------ version anglaise
await open({ viewport: { width: 1400, height: 900 } }, "train.html?lang=en");
await pg.keyboard.press("Shift+?"); await pg.waitForTimeout(300);
const en = await modalTxt();
ok("anglais : la liste des raccourcis est traduite, sans texte français", /Keyboard shortcuts/.test(en) && /Backspace/.test(en) && !/[éèàç]|Raccourcis|Entrée/.test(en));
await pg.keyboard.press("Escape"); await pg.keyboard.press("Control+k"); await waitFor("!!document.getElementById('fdQ')");
ok("anglais : la recherche est traduite", /Find a card/.test(await modalTxt()) && await ev("$('fdQ').placeholder") === "Name, keyword or card text");

// ------------------------------------------------------------ téléphone : le bouton ⌕ tient dans la barre et ouvre la recherche
await open({ viewport: { width: 393, height: 851 }, hasTouch: true, isMobile: true });
await pg.tap("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
await ev("document.getElementById('sSeed').value='4'; document.getElementById('sFirst').value='0'");
await pg.tap("#bStart"); await waitFor("!!(V && !working) && !!document.getElementById('bKeep')");
const r = await ev("(() => { const r = $('bFind').getBoundingClientRect(); return { l: r.left, r: r.right, t: r.top, b: r.bottom, w: innerWidth, sw: document.documentElement.scrollWidth }; })()");
ok(`téléphone 393 px : le bouton ⌕ est à l'écran (${Math.round(r.l)}-${Math.round(r.r)} px), pas de défilement horizontal`, r.l >= 0 && r.r <= r.w && r.b > r.t && r.sw <= r.w);
await pg.tap("#bFind"); await waitFor("!!document.getElementById('fdQ')");
await ev("(() => { const q = $('fdQ'); q.value = 'Falling Star'; q.oninput(); })()"); await waitFor("!!document.querySelector('#fdCard .csheet')");
const sh = await ev("(() => { const s = document.querySelector('#fdCard .csheet').getBoundingClientRect(), m = $('mbox').getBoundingClientRect(); return { sl: s.left, sr: s.right, ml: m.left, mr: m.right, w: innerWidth, name: document.querySelector('#fdCard .csheet').dataset.sheet }; })()");
ok(`téléphone : un seul résultat (« ${sh.name} ») affiche sa fiche tout de suite, dans la fenêtre et dans l'écran`, sh.name === "Falling Star" && sh.sl >= sh.ml && sh.sr <= sh.mr + 1 && sh.mr <= sh.w);
await ev("$('mClose').click()"); await pg.waitForTimeout(200);
ok("téléphone : fermer la recherche rend le mulligan en attente", await ev("$('modal').hidden && !!document.getElementById('bKeep')"));
ok("aucune erreur de page", errs.length === 0); if (errs.length) console.log(errs.join("\n"));
await b.close();
console.log(fails ? `\n${fails} ÉCHEC(S)` : "\nTout est bon.");
process.exit(fails ? 1 : 0);
