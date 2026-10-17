import { existsSync, readFileSync } from "node:fs";
// Robot de la version anglaise (i18n.js, engine/lang.py) : la table ouverte avec ?lang=en ne montre aucun texte
// français. Il parcourt le menu (tableau de bord, choix du deck, aide, mes decks, mes parties, ami), l'éditeur de deck
// (import, export), les sons, la nouvelle partie (BO1 et BO3), une partie BO3 jouée jusqu'au bout (mulligan, questions,
// conseil, journal, défausse, fin de manche, sideboard, préparation de la manche 2), le replay et le tutoriel.
// À chaque écran, il relève les textes (nœuds texte et attributs title, aria-label, placeholder) restés en français :
// mots ou lettres françaises, ou texte présent tel quel dans src.html (écrit en français) sans être une carte ni un mot
// identique dans les deux langues (OK ci-dessous). Puis il vérifie que le bouton « Français » ramène la table en français.
// Usage : (cd build && python3 -m http.server 8771 &) ; node verif_anglais.mjs <dossier des captures> [port]
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8771";
const SRC = readFileSync(new URL("./src.html", import.meta.url), "utf8");
// textes identiques en français et en anglais (mots du jeu, sigles)
const OK = ["Mulligan", "Sideboard", "Battlefield", "Battlefields", "Combat", "Showdown", "Replay", "Room", "Duel", "Normal",
  "Mode", "Volume", "Studio", "Zen", "Base", "Runes", "Rune", "Decks", "Deck", "Match", "Main Deck", "Chosen Champion", "Rune Pool",
  "Legend", "Champion", "Signature", "Unit", "Spell", "Gear", "Token", "Fury", "Calm", "Mind", "Body", "Chaos", "Order",
  "Accelerate", "Hidden", "Tank", "Backline", "Ganking", "Deflect", "Assault", "Shield", "Temporary", "Empower", "Flow", "Repeat",
  "Riftbound Observatory", "Riftbound", "Predict", "Vision", "Reaction", "Action", "BO1", "BO3", "ST", "TEMP", "EMP", "Lvl", "VS",
  "Français", "Decks.", "Coach", "Champions", "sideboard", "best", "hidden", "Pause", "⏸ Pause", "might", "Might", "Menu", "☰ Menu", "← Menu", "Destination", "Ready", "Points", "Claude", "PeerJS", "WebRTC", "Darius", "Master Yi", "LeBlanc", "Akali",
  "Ambush", "Legion", "Deathknell", "Equip", "Quick-Draw", "Weaponmaster", "Hunt", "Level", "Unique", "Empowered", "Stun", "Burn", "Mighty", "Buff", "Add", "XP", "Focus"];
const FR = /[àâçéèêëîïôûùœ«»]|\b(le|la|les|des|du|une|et|ou|ton|ta|tes|ses|pour|avec|sans|sur|sous|dans|vers|cible|carte|cartes|unité|unités|joue|déplace|pioche|tour|défausse|aucun|aucune|quelle|quel|toi|adverse|passe|termine|gratuit|oui|partie|parties|manche|donne|choisis|touche|suivant|compris|revoir|coup|coups|forte|normale|nouvelle|nouveau|jouer|retour|fin|début|aide|garder|valider|annuler|cibler|cible|cibles|jouable|principale|enregistrement|stockage|plein|choisi|vide|cacher|leçon|défi|lexique|mot|mots)\b/i;
const b = await chromium.launch(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {});
let fails = 0;
const ok = (n, v) => { console.log((v ? "OK   " : "ÉCHEC") + " " + n); if (!v) fails++; };

async function run(W, H, mobile) {
  const found = new Map();   // texte → écran
  try { await run1(W, H, mobile, found); }
  finally { for (const [t, where] of found) console.log(`  reste en français [${where}] : ${JSON.stringify(t)}`); }
}

async function run1(W, H, mobile, found) {
  console.log(`\n=== ${W}×${H} ${mobile ? "(téléphone, toucher)" : "(ordinateur, souris)"} ===`);
  const ctx = await b.newContext({ viewport: { width: W, height: H }, isMobile: mobile, hasTouch: mobile });
  const pg = await ctx.newPage(); const errs = []; pg.on("pageerror", e => errs.push(String(e)));
  const ev = s => pg.evaluate(s);
  const tap = async sel => { await pg.locator(sel).first().scrollIntoViewIfNeeded(); return mobile ? pg.tap(sel) : pg.click(sel); };
  const waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await ev(s)) return; } catch (e) {} await pg.waitForTimeout(150); } throw new Error("attente : " + s); };
  let names = null;
  // textes visibles de la page (nœuds texte, attributs) restés en français
  async function scan(where) {
    const texts = await ev(`(() => { const out = new Set(); const SK = new Set(["SCRIPT", "STYLE", "TEXTAREA"]);
      const w = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
      for (let n = w.currentNode; n; n = w.nextNode()) {
        if (n.nodeType === 3) { if (!SK.has(n.parentNode.nodeName) && n.parentNode.closest && !n.parentNode.closest("[hidden]")) out.add(n.data); }
        else { if (n.closest("[hidden]")) continue; for (const a of ["title", "aria-label", "placeholder", "label"]) { const v = n.getAttribute(a); if (v) out.add(v); } }
      }
      return [...out]; })()`);
    if (!names) {
      names = await ev("CAT ? CAT.cards.map(c => c.n).concat(CAT.legends.map(l => l.n), CAT.presets.flatMap(p => [p.event, p.player].filter(Boolean).map(x => x.replace(/^Riftbound\\s+/, '')))) : []");
      names = [...new Set(names.flatMap(n => [n, n.split(",")[0]]))].sort((x, y) => y.length - x.length);
    }
    let n = 0;
    for (const raw of texts) {
      const t = raw.replace(/\s+/g, " ").trim();
      if (!/[A-Za-zÀ-ÿ]{2}/.test(t) || OK.includes(t)) continue;
      let s = t; for (const nm of names) if (s.includes(nm)) s = s.split(nm).join("X");
      s = s.replace(/\b[a-z0-9]+(-[a-z0-9]+)+\b/g, "X");   // clés de decks (rq-la, hxn-tempete…)
      const inSrc = new RegExp("[>\"'`]\\s*" + t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "\\s*[<\"'`$]").test(SRC);
      const bad = FR.test(s) || (s === t && t.length >= 4 && !names.includes(t) && inSrc && !/^[A-Z0-9 #·./+–-]+$/.test(t));
      if (bad && !found.has(t)) { found.set(t, where); n++; }
    }
    return n;
  }
  const step = async (where, shot) => { const n = await scan(where); if (shot) await pg.screenshot({ path: `${OUT}/en-${shot}-${W}.png` }); if (n) console.log(`  ${where} : ${n} texte(s) en français`); };

  await pg.goto(`http://localhost:${PORT}/train.html?lang=en`);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=deck]');return b&&!b.disabled&&!!CAT&&!CHKLATER})()"); await pg.waitForTimeout(300);
  ok("page en anglais (html lang=en, I18N.lang = en)", await ev("document.documentElement.lang === 'en' && I18N.lang === 'en'"));
  ok("menu en anglais (« New game »)", await ev("document.querySelector('#menu .mitem[data-m=new]').textContent.includes('New game')"));
  await step("menu", "menu");
  // choix du deck d'entraînement
  await tap(".deckc [data-m=pick]"); await waitFor("!!document.querySelector('.pick .roster')");
  await tap('[data-pleg="Akali, Rogue Assassin"]'); await waitFor("!!document.querySelector('[data-pick]')");
  await step("choisir mon deck", "pick");
  const k = await ev("document.querySelector('[data-pick]:not([disabled])').dataset.pick");
  await tap(`[data-pick="${k}"]`); await waitFor("!!document.querySelector('.deckc:not(.none)')"); await pg.waitForTimeout(500);
  await step("tableau de bord", "dash");
  for (const m of ["help", "mydecks", "games", "friend"]) {
    await ev("menuRender('main')"); await tap(`[data-m=${m}]`); await pg.waitForTimeout(400); await step("menu " + m, m);
  }
  await ev("menuRender('main')");
  // sons
  await ev("document.getElementById('bSon').click()"); await pg.waitForTimeout(200); await step("sons", "sons"); await ev("document.getElementById('bSon').click()");
  // éditeur de deck : deck de référence, import, export
  await tap("[data-m=deck]"); await waitFor("!!document.getElementById('dkSave')");
  await step("éditeur vide", "editeur");
  await pg.selectOption("#dkLoad", "leblanc-iq5"); await waitFor("!!document.getElementById('dkSideH')");
  await step("éditeur LeBlanc", "editeur2");
  await tap("#dkToSide"); await pg.waitForTimeout(200); await step("éditeur sideboard"); await tap("#dkToMain");
  await tap("#dkImp"); await pg.waitForTimeout(300); await step("import", "import"); await ev("closeModal()");
  await tap("#dkExp"); await pg.waitForTimeout(300); await step("export"); await ev("closeModal()");
  await tap("#dkMenu"); await pg.waitForTimeout(300);
  // nouvelle partie contre l'IA : BO1, BO3, options
  await ev("menuRender('main')"); await tap("[data-m=new]"); await waitFor("!!document.getElementById('bStart')");
  await step("nouvelle partie BO1", "new");
  await pg.click(".vopt summary"); await pg.waitForTimeout(200); await step("plus d'options");
  await tap(".vpad.cpu .vtag"); await pg.waitForTimeout(200); await step("panneau IA");
  await tap('.vtile[data-leg="LeBlanc, Deceiver"], .vtile[data-leg^="LeBlanc"]'); await pg.waitForTimeout(200);
  await tap("#sBo3"); await pg.waitForTimeout(200); await step("nouvelle partie BO3", "new-bo3");
  await pg.fill("#sSeed", "12");
  await tap("#bStart"); await waitFor("!!document.getElementById('bLaunch')");
  await step("préparation manche 1", "prep1");
  if (await ev("!!document.querySelector('[data-pf]')")) await tap('[data-pf="0"]');
  const bfs = await ev("[...document.querySelectorAll('[data-pbf]')].map(x=>x.dataset.pbf)");
  await tap(`[data-pbf="${bfs[0]}"]`); await step("préparation choisie");
  await tap("#bLaunch"); await waitFor("!!(V && META)");
  // partie jouée au hasard jusqu'au bout, avec relevés réguliers
  let seed = 4242; const rnd = n => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed % n; };
  let moves = 0, shots = 0, hintDone = false, trashDone = false;
  for (let i = 0; i < 3000; i++) {
    await waitFor("!!(V && !working)");
    if (await ev("V.winner !== null && V.winner !== undefined")) break;
    if (await ev("!!document.getElementById('bKeep')")) { await step("mulligan", "mulligan"); await tap("#bKeep"); continue; }
    if (await ev("!!V.ask")) {
      await step("question « " + (await ev("V.ask.kind")) + " »", shots++ < 3 ? "ask" + shots : null);
      const n = await ev("V.ask.options.length"); await ev(`(async()=>{ await answer(${rnd(n)}) })()`); moves++; continue;
    }
    const n = await ev("V.dec ? V.dec.options.length : 0");
    if (n) {
      if (moves % 12 === 0) await step("plateau, tour " + (await ev("V.st.t")), moves === 12 ? "plateau" : null);
      if (!hintDone && n > 2 && await ev("!document.getElementById('bHint').disabled")) {
        await tap("#bHint"); await waitFor("!document.getElementById('hpop').hidden && !/Thinking/.test(document.getElementById('hpop').textContent)", 120000);
        await step("conseil", "conseil"); hintDone = true; await ev("document.getElementById('hpop').hidden = true");
      }
      if (!trashDone && await ev("V.st.p[0].trash.length > 0")) {
        await ev("document.querySelector('[data-trash]').click()"); await pg.waitForTimeout(300); await step("défausse", "defausse"); await ev("closeModal()"); trashDone = true;
      }
      const i2 = rnd(n); await ev(`(async()=>{ await choose(V.dec.options[${i2}].i) })()`); moves++;
    }
  }
  await waitFor("!!(V && !working)"); await pg.waitForTimeout(700);
  await ev("document.getElementById('bJournal').click()"); await pg.waitForTimeout(300); await step("journal", "journal"); await ev("document.getElementById('bJournal').click()");
  await step("fin de manche", "fin");
  ok(`manche 1 jouée jusqu'au bout en anglais (${moves} coups, ${await ev("V.st.pts.join('-')")})`, await ev("V.winner !== null && V.winner !== undefined"));
  ok("journal du moteur en anglais (« Turn … »)", await ev("LOG.some(([i, t]) => /^Turn \\d+/.test(t)) && !LOG.some(([i, t]) => /^Tour /.test(t))"));
  // manche 2 : sideboard puis préparation
  if (await ev("!!document.getElementById('bNextGame')")) {
    await tap("#bNextGame"); await pg.waitForTimeout(800);
    if (await ev("!!document.getElementById('sbKeep')")) { await step("sideboard", "sideboard"); await tap("#sbKeep"); await pg.waitForTimeout(500); }
    await waitFor("!!document.getElementById('bLaunch')"); await step("préparation manche 2", "prep2");
    await ev("closeModal()");
  }
  // replay de la partie jouée
  await ev("showMenu(); menuRender('games')"); await pg.waitForTimeout(800); await step("mes parties (après une partie)", "games2");
  if (await ev("!!document.querySelector('[data-rp]')")) {
    await tap("[data-rp]"); await waitFor("!!document.getElementById('rpbar')", 300000); await pg.waitForTimeout(500);
    await step("replay", "replay");
    if (await ev("!!document.querySelector('#rpbar [data-rpg]')")) { await ev("document.querySelectorAll('#rpbar [data-rpg]')[4].click()"); await pg.waitForTimeout(800); await step("replay, ta décision"); }
    await ev("document.getElementById('rpQuit').click()"); await pg.waitForTimeout(800);
  }
  // tutoriel : intro, partie guidée, coach
  await ev("showMenu(); menuRender('main')"); await pg.waitForTimeout(300);
  await tap("[data-m=learn]");
  const nIntro = await ev("TINTRO.length");
  for (let i = 0; i < nIntro; i++) { await waitFor("!!document.getElementById('tNext')"); await step("tutoriel, intro " + (i + 1), i === 0 ? "tuto" : null); await tap("#tNext"); }
  await waitFor("!!(V && !working && !document.getElementById('coach').hidden)"); await step("tutoriel, coach", "coach");
  // chaque leçon, défi et astuce du coach (avec la définition de son premier mot), l'attente, le lexique, l'agrandi
  // d'une carte avec ses mots-clés et l'écran de fin : dessinés tour à tour puis relus
  const nSteps = await ev("TSTEPS.length"), nTips = await ev("TTIPS.length");
  for (let j = 0; j < nSteps; j++) { await ev(`(()=>{ TUTO.k = ${j}; TUTO.tip = null; TUTO.open = true; TUTO.fresh = ${j} > 2 ? 0 : -1; const x = TSTEPS[${j}]; TUTO.kw = x.kw ? x.kw[0] : null; const w = x.when; if (w) x.when = () => true; tutoDraw(); if (w) x.when = w; })()`); await step("tutoriel, étape " + (j + 1)); }
  await ev(`(()=>{ const x = TSTEPS.find(s => s.wait); TUTO.k = TSTEPS.indexOf(x); const w = x.when; x.when = () => false; tutoDraw(); x.when = w; })()`); await step("tutoriel, attente");
  for (let j = 0; j < nTips; j++) { await ev(`(()=>{ TUTO.k = 7; TUTO.tip = TTIPS[${j}]; TUTO.kw = TTIPS[${j}].kw ? TTIPS[${j}].kw[0] : null; tutoDraw(); })()`); await step("tutoriel, astuce " + (j + 1)); }
  await ev("TUTO.tip = null; TUTO.k = TSTEPS.length; tutoDraw()"); await step("tutoriel, coach réussi");
  await ev("openGlossary()"); await step("lexique", "lexique"); await ev("closeModal()");
  for (const n of ["Punch First", "Pit Rookie", "Zhonya's Hourglass", "Master Yi, Tempered", "Darius, Hand of Noxus", "Hidden Blade", "Vanguard Captain", "Carrion Dredger", "Inferna"]) {
    await ev(`(()=>{ const e = document.createElement("div"); e.dataset.card = ${JSON.stringify(n)}; document.body.appendChild(e); showZoom(e); e.remove(); })()`); await step("agrandi " + n);
  }
  await ev("$('zoom').hidden = true; tutoEnd(V)"); await step("tutoriel, fin"); await ev("closeModal()");
  // retour au français par le bouton du menu
  await ev("showMenu(); menuRender('main')"); await pg.waitForTimeout(300);
  await Promise.all([pg.waitForNavigation(), tap(".mlang")]);
  await waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=new]');return b&&!b.disabled})()");
  ok("le bouton « Français » remet la table en français (« Nouvelle partie »)", await ev("I18N.lang === 'fr' && document.querySelector('#menu .mitem[data-m=new]').textContent.includes('Nouvelle partie')"));
  ok("le choix est gardé (rbt-lang = fr)", await ev("localStorage.getItem('rbt-lang') === 'fr'"));
  ok(`aucun texte en français sur les écrans parcourus (${found.size} relevé${found.size > 1 ? "s" : ""})`, found.size === 0);
  ok("aucune erreur JS", errs.length === 0); if (errs.length) console.log(errs);
  await ctx.close();
}

await run(1280, 800, false);
await run(360, 740, true);
await b.close();
process.exit(fails ? 1 : 0);
