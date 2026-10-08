import { existsSync, mkdirSync } from "node:fs";
// Playwright : celui de l'environnement cloud Claude, sinon le paquet npm « playwright » (CI).
const LOCAL_PW = "/opt/node22/lib/node_modules/playwright/index.mjs", LOCAL_CHROME = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const { chromium } = await import(process.env.PW_MODULE || (existsSync(LOCAL_PW) ? LOCAL_PW : "playwright"));
// Robot du duel entre amis (PeerJS, deux navigateurs) : hôte 1400×900 (souris), invité 360×740 (toucher).
// (a) room BO3 créée par l'hôte, code tapé par l'invité ; (b) préparation de la manche 1 par clics (premier joueur, battlefields
// simultanés) ; (c) manche jouée jusqu'au bout : la page qui a la main joue un coup au hasard par choose(i) / answer(x) (les
// fonctions de la page, qui envoient l'entrée), avec contrôle continu de la synchronisation (score, tour, plateau, mains,
// nombre d'entrées) et du message « En attente » chez l'autre ; (d) score du match identique, manche 2 avec un échange de
// sideboard chez l'invité, battlefields joués retirés ; (e) rechargement de l'invité en pleine manche 2 et reprise au même
// état que l'hôte ; (f) aucune erreur JS.
// Usage : (cd build && python3 -m http.server 8790 &) ; peerjs --port 9123 --host 127.0.0.1 --path /myapp &
//         node verif_duel.mjs <dossier des captures> [port web] [port peer]
const OUT = process.argv[2] || ".", PORT = process.argv[3] || "8790", PEER = process.argv[4] || "9123";
mkdirSync(OUT, { recursive: true });
const URL_ = `http://localhost:${PORT}/train.html?peer=localhost:${PEER}`;
const b = await chromium.launch({ ...(process.env.CHROME_PATH || existsSync(LOCAL_CHROME) ? { executablePath: process.env.CHROME_PATH || LOCAL_CHROME } : {}),
  args: ["--disable-features=WebRtcHideLocalIpsWithMdns"] });
let fails = 0, seed = +(process.env.RB_SEED || Date.now() % 100000);
const rnd = n => { seed = (seed * 1103515245 + 12345) % 2147483648; return seed % n; };
const ok = (n, v) => { console.log((v ? "OK   " : "ÉCHEC") + " " + n); if (!v) fails++; return v; };

async function page(W, H, mobile, name) {
  const ctx = await b.newContext({ viewport: { width: W, height: H }, isMobile: mobile, hasTouch: mobile });
  const pg = await ctx.newPage(); const P = { pg, name, mobile, errs: [] };
  pg.on("pageerror", e => P.errs.push(String(e)));
  pg.on("console", m => { if (m.type() === "error" && !/favicon/.test(m.text() + ((m.location() || {}).url || ""))) P.errs.push("console: " + m.text()); });
  P.ev = s => pg.evaluate(s);
  P.tap = async sel => { await pg.locator(sel).first().scrollIntoViewIfNeeded(); return mobile ? pg.tap(sel) : pg.click(sel); };
  P.waitFor = async (s, ms = 180000) => { const t = Date.now(); while (Date.now() - t < ms) { try { if (await P.ev(s)) return; } catch (e) {} await pg.waitForTimeout(120); } throw new Error(`${name} : attente de ${s}`); };
  P.shot = n => pg.screenshot({ path: `${OUT}/${n}-${name}.png` });
  P.noHScroll = () => P.ev("document.documentElement.scrollWidth <= innerWidth + 1");
  return P;
}
const H = await page(1400, 900, false, "hote"), G = await page(360, 740, true, "invite");
// État comparable des deux côtés (positions absolues : place 0 = hôte, 1 = invité).
const STATE = `(() => { if (!V || !DU || !DU.game) return null; const st = V.st;
  return { idle: !working, n: DU.game.inputs.length, g: DU.game.no, t: st.t, tp: st.tp, pts: st.pts, w: V.winner ?? null,
    bfs: st.bfs.map(b => b.n + ":" + (b.c ?? "-") + ":" + b.u.map(u => u.n + "/" + u.c + "/" + u.m + "/" + (u.d || 0)).sort().join(",")),
    base: st.p.map(p => p.base.map(u => u.n).sort().join(",")), hand: st.p.map(p => p.hand.length), deck: st.p.map(p => p.deck),
    trash: st.p.map(p => p.trash.join(",")), runes: st.p.map(p => p.runes.join(",")),
    dec: !!V.dec, ask: V.ask ? V.ask.kind : null, nopt: V.dec ? V.dec.options.length : V.ask ? V.ask.options.length : 0,
    wait: !!V.wait, waitTxt: (document.getElementById("waitTxt") || {}).textContent || "", score: DU.st.wins }; })()`;
const same = (a, b) => JSON.stringify({ ...a, idle: 0, dec: 0, ask: 0, nopt: 0, wait: 0, waitTxt: 0 }) === JSON.stringify({ ...b, idle: 0, dec: 0, ask: 0, nopt: 0, wait: 0, waitTxt: 0 });

async function settled() {   // les deux pages au repos, au même nombre d'entrées
  const t = Date.now();
  while (Date.now() - t < 60000) {
    const [a, c] = [await H.ev(STATE), await G.ev(STATE)];
    if (a && c && a.idle && c.idle && a.n === c.n && (a.w !== null || (!!(a.dec || a.ask) !== !!(c.dec || c.ask)))) return [a, c];
    await H.pg.waitForTimeout(60);
  }
  throw new Error("les deux pages ne se stabilisent pas : " + JSON.stringify([await H.ev(STATE), await G.ev(STATE)]));
}
// Joue jusqu'à la fin de la manche (ou max entrées) ; contrôle la synchro à chaque coup. Renvoie le nombre d'entrées.
async function play(max, label) {
  let n = 0, desync = 0, waitBad = 0, both = 0, checks = 0;
  for (;;) {
    const [a, c] = await settled();
    checks++;
    if (!same(a, c)) { desync++; if (desync < 3) console.log("  écart :", JSON.stringify(a), "\n         ", JSON.stringify(c)); }
    if (a.w !== null) break;
    if (n >= max) break;
    const [me, ot, st] = (a.dec || a.ask) ? [H, G, a] : [G, H, c];
    const other = me === H ? c : a;
    if ((a.dec || a.ask) && (c.dec || c.ask)) both++;
    if (!other.wait || !/En attente/.test(other.waitTxt)) waitBad++;
    if (st.ask) await me.ev(`(async()=>{ await answer(${st.ask === "mulligan" ? "[]" : rnd(st.nopt)}) })()`);
    else await me.ev(`(async()=>{ await choose(V.dec.options[${rnd(st.nopt)}].i) })()`);
    n++;
    if (n % 100 === 0) console.log(`  ${label} : ${n} entrées, tour ${a.t}, points ${a.pts.join("-")}`);
  }
  ok(`${label} : ${checks} contrôles, mêmes score / tour / plateau / mains / défausses / runes des deux côtés`, desync === 0);
  ok(`${label} : à chaque coup un seul joueur a la main`, both === 0);
  ok(`${label} : l'autre page affiche « En attente de … » à chaque coup`, waitBad === 0);
  return n;
}
async function prep(game) {
  // le joueur qui choisit le premier joueur clique ; les battlefields sont choisis des deux côtés
  await H.waitFor("!!document.getElementById('pScore')"); await G.waitFor("!!document.getElementById('pScore')");
  const ch = await H.ev("DU.prep.chooser");
  ok(`manche ${game} : même préparation des deux côtés (choix par ${ch === 0 ? "l'hôte" : ch === 1 ? "l'invité" : "personne"})`, JSON.stringify(await H.ev("DU.prep.nx")) === JSON.stringify(await G.ev("DU.prep.nx")));
  await H.shot(`m${game}-prep`); await G.shot(`m${game}-prep`);
  if (ch === 0 || ch === 1) {
    const [C, O] = ch === 0 ? [H, G] : [G, H];
    ok(`manche ${game} : l'autre voit « … choisit qui commence »`, /choisit qui commence/.test(await O.ev("document.getElementById('pDecision').textContent")));
    await C.tap(game === 1 ? "[data-df=first]" : "[data-df=second]");
    await O.waitFor("DU.prep.first !== null");
    ok(`manche ${game} : choix du premier joueur reçu (${await O.ev("document.getElementById('pDecision').textContent")})`, (await O.ev("DU.prep.first")) === (await C.ev("DU.prep.first")));
  }
  // battlefields : l'hôte d'abord ; l'invité ne doit pas voir lequel avant d'avoir choisi le sien
  const hb = await H.ev("DU.prep.nx.allowed[0][0]");
  await H.tap("[data-dbf]"); await H.tap("#bDuelBf");
  await G.waitFor("!!DU.prep.bf[0]");
  const gTxt = await G.ev("document.getElementById('pOppBf').textContent");
  ok(`manche ${game} : l'invité sait que l'hôte a choisi, sans voir lequel (« ${gTxt.trim()} »)`, /a choisi/.test(gTxt) && !gTxt.includes(hb));
  const gone = await G.ev("(()=>{const e=document.getElementById('pGone');return e?e.textContent:''})()");
  const allowed = await G.ev("DU.prep.nx.allowed[1]");
  const pick = allowed[allowed.length - 1];
  await G.tap(`[data-dbf="${pick}"]`); await G.tap("#bDuelBf");
  await H.waitFor("DU.game && DU.game.no === " + game + " && V && !working", 60000); await G.waitFor("DU.game && DU.game.no === " + game + " && V && !working", 60000);
  const mh = await H.ev("JSON.stringify([META.bf, META.first, META.seed])"), mg = await G.ev("JSON.stringify([META.bf, META.first, META.seed])");
  ok(`manche ${game} : mêmes battlefields, premier joueur et graine (${mh})`, mh === mg && JSON.parse(mh)[0][0] === hb && JSON.parse(mh)[0][1] === pick);
  return gone;
}

try {
  await H.pg.goto(URL_); await G.pg.goto(URL_);
  for (const P of [H, G]) await P.waitFor("(()=>{const b=document.querySelector('#menu .mitem[data-m=friend]');return b&&!b.disabled})()");
  // ---------- (a) room BO3
  await H.tap("[data-m=friend]"); await H.pg.fill("#dName", "Huy"); await H.pg.selectOption("#dDeck", "akali-g2");
  await H.tap("#dBo3"); await H.tap("#bCreate");
  await H.waitFor("!!document.getElementById('roomCode')", 30000);
  const code = (await H.ev("document.getElementById('roomCode').textContent")).trim();
  ok(`hôte : room ouverte, code « ${code} » (6 caractères sans I, L, O, 0, 1)`, /^[A-HJKMNP-Z2-9]{6}$/.test(code));
  ok("hôte : lien de la room avec #room=CODE", (await H.ev("document.getElementById('roomLink').value")).endsWith("#room=" + code));
  await H.shot("a-room");
  await G.tap("[data-m=friend]"); await G.pg.fill("#dName", "Ami"); await G.pg.selectOption("#dDeck", "leblanc-iq5");
  await G.pg.fill("#dCode", code.toLowerCase());
  ok("invité : écran « Jouer avec un ami » sans défilement horizontal", await G.noHScroll());
  const btn = await G.ev("(()=>{const r=document.getElementById('bJoin').getBoundingClientRect();return Math.min(r.width,r.height)})()");
  ok(`invité : bouton Rejoindre assez grand pour le doigt (${btn} px)`, btn >= 44);
  await G.shot("a-rejoindre");
  await G.tap("#bJoin");
  // ---------- (b) manche 1
  await prep(1);
  ok("noms : hôte « Huy », invité « Ami » des deux côtés", JSON.stringify(await H.ev("PN")) === JSON.stringify(["Huy", "Ami"]) && JSON.stringify(await G.ev("PN")) === JSON.stringify(["Huy", "Ami"]));
  ok("invité : sa main en bas (« Ta main »), celle de l'hôte cachée en haut", await G.ev("document.querySelector('#board .half.p0 .who-hand').textContent.startsWith('Ta main') && document.querySelectorAll('#board .hand.top .card.back').length === V.st.p[0].hand.length"));
  ok("en duel : ni « Reprendre » ni « Conseil »", await H.ev("document.getElementById('bUndo').hidden && document.getElementById('bHint').hidden") && await G.ev("document.getElementById('bUndo').hidden && document.getElementById('bHint').hidden"));
  await H.shot("b-manche1-debut"); await G.shot("b-manche1-debut");
  // ---------- (c) manche 1 jusqu'au bout
  const n1 = await play(6000, "manche 1");
  const [e1, e2] = [await H.ev(STATE), await G.ev(STATE)];
  ok(`manche 1 terminée des deux côtés après ${n1} entrées (gagnant : place ${e1.w}, points ${e1.pts.join("-")})`, e1.w !== null && e1.w === e2.w);
  // ---------- (d) score du match
  await H.waitFor("!!document.getElementById('fScore')"); await G.waitFor("!!document.getElementById('fScore')");
  const sh = await H.ev("JSON.stringify(DU.st.wins)"), sg = await G.ev("JSON.stringify(DU.st.wins)");
  ok(`fin de manche 1 : même score de match (${sh})`, sh === sg && sh !== "[0,0]");
  ok(`fin de manche 1 : texte du score chez l'invité (« ${await G.ev("document.getElementById('fScore').textContent")} »)`, /Match : toi \d – \d Huy/.test(await G.ev("document.getElementById('fScore').textContent")));
  ok("fin de manche 1 : manche enregistrée dans ce navigateur (kind duel, entrées des deux places)", await G.ev("(()=>{const g=Object.values(JSON.parse(localStorage.getItem('rbt-games')||'{}')).find(x=>x.kind==='duel'&&x.match&&x.match.game===1);return !!g && g.inputs.length===" + n1 + " && new Set(g.inputs.map(x=>x[2])).size===2 && g.result && g.args.length===6})()"));
  await H.shot("d-fin-manche1"); await G.shot("d-fin-manche1");
  // manche 2 : chacun touche « Manche suivante » ; l'hôte garde son deck, l'invité échange une carte
  await H.tap("#bNextGame"); await G.tap("#bNextGame");
  await H.waitFor("!!document.getElementById('sbKeep')"); await G.waitFor("!!document.getElementById('sbKeep')");
  ok("manche 2 : écran de sideboard des deux côtés (403)", true);
  await G.shot("d-sideboard");
  await H.tap("#sbKeep");
  const outN = await G.ev("document.querySelector('[data-sbp=out]').dataset.n"), inN = await G.ev("document.querySelector('[data-sbp=inn]').dataset.n");
  await G.tap(`[data-sbp=out][data-n="${outN}"]`); await G.tap(`[data-sbp=inn][data-n="${inN}"]`);
  await G.tap("#sbOk");
  await G.waitFor("!!document.getElementById('pScore')");
  const sbOK = await H.ev(`(()=>{const d=DU.prep.deck[1]; return !!d && d.main.filter(x=>x===${JSON.stringify(inN)}).length === DU.decks[1].main.filter(x=>x===${JSON.stringify(inN)}).length + 1})()`);
  ok(`manche 2 : l'hôte a reçu le deck de l'invité (− ${outN}, + ${inN})`, sbOK);
  const winner1 = e1.w;
  const gone = await prep(2);
  const goneOk = winner1 === 0 || winner1 === 1 ? /retiré/.test(gone) : true;
  ok(`manche 2 : battlefield joué en manche 1 retiré (${gone || "manche nulle : rien de retiré"})`, goneOk);
  ok("manche 2 : deck de l'invité modifié dans les arguments de duel_new", await H.ev(`JSON.parse(DU.game.args[5]).main.includes(${JSON.stringify(inN)})`) && await G.ev(`JSON.parse(DU.game.args[5]).main.includes(${JSON.stringify(inN)})`));
  const n2 = await play(30, "manche 2 (début)");
  await H.shot("d-manche2"); await G.shot("d-manche2");
  // ---------- (e) rechargement de l'invité en pleine manche 2
  const before = await H.ev(STATE);
  await G.pg.reload();
  await G.waitFor("DU && DU.game && DU.game.no === 2 && V && !working && DU.game.inputs.length === " + before.n, 240000);
  const after = await G.ev(STATE);
  ok(`rechargement de l'invité : même état que l'hôte (${before.n} entrées rejouées, tour ${after.t})`, same(before, after));
  ok("rechargement : l'invité est de nouveau connecté", await G.ev("!!(DU.conn && DU.conn.open)") && await H.ev("!!(DU.conn && DU.conn.open)"));
  await G.shot("e-reprise");
  await play(25, "manche 2 (après reprise)");
  await H.shot("e-apres-reprise"); await G.shot("e-apres-reprise");
  ok("pas de défilement horizontal (hôte et invité)", await H.noHScroll() && await G.noHScroll());
} catch (e) { ok("déroulé complet : " + e.message, false); await H.shot("erreur").catch(() => {}); await G.shot("erreur").catch(() => {}); }
// ---------- (f) erreurs JS
for (const P of [H, G]) ok(`${P.name} : aucune erreur JS${P.errs.length ? " (" + P.errs.slice(0, 3).join(" | ") + ")" : ""}`, P.errs.length === 0);
await b.close();
console.log(fails ? `\n${fails} échec(s)` : "\nTout est vert.");
process.exit(fails ? 1 : 0);
