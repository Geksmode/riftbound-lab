// ------------------------------------------------------------ interactions : sélection, cibles, déplacements, flèches
// Une carte touchée ouvre un petit menu (jouer, Empower, Cibler…, Déplacer…). Les cibles se choisissent une à une puis
// « Valider la cible ». Un déplacement se prépare (unités + destination, par glisser ou toucher) puis se lance par bouton :
// rien n'est joué tant que le bouton n'est pas touché.
const S = x => x === null || x === undefined ? null : String(x);
const optsFor = src => { if (!V || !V.dec) return []; const s2 = S(ALIAS[src] ?? src); return V.dec.options.filter(o => S(o.src) === s2 || (o.us && o.us.map(String).includes(s2))); };
const ms = xs => { const m = new Map(); xs.forEach(x => m.set(String(x), (m.get(String(x)) || 0) + 1)); return m; };
function contains(big, small) { const B = ms(big); for (const [k, n] of ms(small)) if ((B.get(k) || 0) < n) return false; return true; }
function rest(big, small) { const B = ms(big); small.forEach(x => B.set(String(x), B.get(String(x)) - 1)); return [...B].filter(([, n]) => n > 0).map(([k]) => k); }
const sameSet = (a, b) => a.length === b.length && contains(a, b);
const hasT = o => (o.t1 || []).length > 0;
const hasMv = o => o.mover !== null && o.mover !== undefined;
const needs = o => hasT(o) || hasMv(o);   // coup qui demande de toucher des cartes (cibles, allié à déplacer…)
const locName = l => l === "base" || l === null || l === undefined ? "base" : (V.st.bfs[+l] || {}).n || l;
const unitEl = u => document.querySelector(`#board .bf [data-uid="${u}"], #board .zone [data-uid="${u}"]`);
const srcEl = u => document.querySelector(`#board [data-uid="${u}"]`);
const zoneEl = z => document.querySelector(`#board [data-drop="${z}"]`);
function uniq(m) { const seen = new Set(); return m.filter(o => { const k = JSON.stringify([o.k, o.label.replace(/cible .*/, ""), [...(o.t1 || [])].map(String).sort(), o.t2 || [], o.acc, o.rep, S(o.loc)]); if (seen.has(k)) return false; seen.add(k); return true; }); }
function flash(msg) { toast(msg, "flash"); }
function reset() { sel = null; mode = null; }
function run(m) {
  m = uniq(m);
  if (!m.length) { flash("Ce coup n'est pas possible."); return; }
  if (m.length === 1) return choose(m[0].i);
  mode = { type: "list", cands: m }; decorate();
}

// ---------- cibles
function startTarget(cands, P = []) {
  const src = sel;
  cands = cands.filter(needs);
  if (!cands.length) return;
  mode = { type: "target", src, cands, P: [], skip: false, M: null, D: null };
  P.forEach(u => addTarget(u, true));
  decorate();
}
const targetable = () => { const u = new Set(); mode.cands.filter(o => contains(o.t1, mode.P)).forEach(o => rest(o.t1, mode.P).forEach(x => u.add(x))); return u; };
function addTarget(u, quiet) {
  u = String(u);
  if (!targetable().has(u)) { if (!quiet) flash("Cette unité n'est pas une cible possible."); return; }
  mode.P.push(u);
  if (!quiet) decorate();
}
// Sorts « cible puis déplacement » (Shuriken Flip) : 1) l'ennemi (ou aucun), 2) l'allié à déplacer, 3) la zone d'arrivée.
const mvMode = () => mode.cands.some(hasMv);
const maxT = () => Math.max(...mode.cands.map(o => o.t1.length));
const matchT = () => mode.cands.filter(o => o.t1.length === mode.P.length && contains(o.t1, mode.P));
function tPhase() {
  if (!mvMode()) return "t1";
  if (!mode.skip && mode.P.length < maxT() && targetable().size) return "t1";
  if (mode.M === null) return "mover";
  if (mode.D === null) return "dest";
  return "done";
}
const moverSet = () => new Set(matchT().filter(hasMv).map(o => S(o.mover)));
const destSet = () => new Set(matchT().filter(o => S(o.mover) === mode.M).map(o => S(o.loc)));
const exactT = () => mvMode() ? matchT().filter(o => S(o.mover) === mode.M && S(o.loc) === mode.D) : matchT();
const cname = u => { const x = srcEl(u); return x && x.dataset.card ? x.dataset.card : "?"; };

// ---------- déplacements
function moveDests(units) { const z = new Set(); for (const u of units) for (const o of optsFor(u)) if (o.k === "move" && contains(o.us, units)) z.add(String(o.loc)); return z; }
function moveOpt() { if (!mode || mode.type !== "move" || mode.dest === null) return null; return V.dec.options.find(o => o.k === "move" && String(o.loc) === mode.dest && sameSet(o.us.map(String), mode.units)) || null; }
function startMove(u, dest = null) {
  if (mode && mode.type === "move") { if (!mode.units.includes(u)) mode.units.push(u); if (dest !== null) mode.dest = dest; }
  else mode = { type: "move", units: [u], dest };
  sel = null; decorate();
}
const canMove = u => optsFor(u).some(o => o.k === "move");

// ---------- menu d'une carte
function popHTML() {
  const ops = optsFor(sel).filter(o => o.k !== "move" || false);
  const name = (srcEl(sel) || {}).dataset ? srcEl(sel).dataset.card : "";
  let h = `<div class="pt">${esc(name || "Carte")}</div>`;
  const groups = new Map();
  for (const o of ops) {
    if (needs(o)) {
      const k = o.k === "act" ? o.label.split(" : ")[0] : "Jouer";
      if (!groups.has(k)) groups.set(k, []);
      groups.get(k).push(o);
    }
  }
  for (const [k, xs] of groups) h += `<button class="tg" data-tg="${esc(k)}">${k === "Jouer" ? "Cibler" : esc(k) + " : cibler"}</button>`;
  const plain = uniq(ops.filter(o => !needs(o))), shown = new Set();
  for (const o of plain) {
    if (shown.has(o.i)) continue;
    let t = o.label;
    const sib = o.k === "play" ? plain.filter(x => x.k === "play" && S(x.loc) === S(o.loc) && x.from_ === o.from_) : [o];
    if (o.k === "play" && sib.length > 1) {
      // plusieurs façons de jouer la carte au même endroit : une ligne de titre puis Accelerate / Normal (ou Repeat)
      h += `<div class="pg">Jouer → ${esc(locName(o.loc))}${o.from_ === "trash" ? " (Flow)" : ""}</div>`;
      for (const x of sib.sort((a, b) => (b.acc ? 1 : 0) - (a.acc ? 1 : 0) || (b.rep ? 1 : 0) - (a.rep ? 1 : 0))) {
        shown.add(x.i);
        h += `<button class="${x.acc ? "acc" : ""}" data-i="${x.i}">${esc([x.acc ? accTxt(x) : "", x.rep ? "Avec Repeat" : ""].filter(Boolean).join(" · ") || "Normal")}</button>`;
      }
      continue;
    }
    if (o.k === "play") t = `Jouer → ${locName(o.loc)}${o.acc ? " (Accelerate)" : ""}${o.rep ? " (Repeat)" : ""}${o.from_ === "trash" ? " (Flow)" : ""}`;
    if (o.k === "hide") t = `Cacher → ${locName(o.loc)}`;
    if (o.k === "act") t = o.label.replace(/^(\w+) : [^(]*\(/, "$1 (");
    h += `<button data-i="${o.i}">${esc(cap(t))}</button>`;
  }
  if (canMove(sel)) h += `<button data-mv="1">Déplacer… (puis choisir la destination)</button>`;
  h += `<button data-read="1">Lire la carte</button>`;
  return h;
}
function placePop() {
  const P = $("pop"), el = srcEl(sel);
  if (!el) { P.hidden = true; return; }
  P.hidden = false;
  const r = el.getBoundingClientRect(), w = P.offsetWidth, h = P.offsetHeight;
  let x = r.left + r.width / 2 - w / 2; x = Math.max(8, Math.min(innerWidth - w - 8, x));
  let y = r.top - h - 10; if (y < 8) y = Math.min(innerHeight - h - 8, r.bottom + 10);
  P.style.left = x + "px"; P.style.top = y + "px";
}
$("pop").addEventListener("click", e => {
  e.stopPropagation();
  const b = e.target.closest("button"); if (!b) return;
  if (b.dataset.i) { $("pop").hidden = true; return choose(+b.dataset.i); }
  if (b.dataset.tg) { const k = b.dataset.tg; return startTarget(optsFor(sel).filter(o => needs(o) && (o.k === "act" ? o.label.split(" : ")[0] === k : k === "Jouer"))); }
  if (b.dataset.mv) return startMove(sel);
  if (b.dataset.read) { const el = srcEl(sel); if (el) showZoom(el); }
});

// ---------- bulle de décision au milieu du plateau (cibles, déplacement, précision, choix pendant un effet)
const nm = s => String(s).replace(/ adverse.*$| \(.*\)$/, "");
function askHTML() {
  const a = mode.a;
  const who = a.item ? `<b>${esc(a.item)}</b> · ` : "";
  if (a.kind === "mulligan") {
    const n = mode.mull.size;
    return `<span class="ft">Mulligan : touche jusqu'à 2 cartes à remettre.</span><div class="mull">${a.options.map((c, i) => `<button data-m="${i}" aria-pressed="${mode.mull.has(i)}" title="${esc(c)}">${card(c, { raw: true, p: 0, cost: true })}</button>`).join("")}</div>
      <div class="prow"><button class="go" id="bKeep">${n ? `Remettre ${n} carte${n > 1 ? "s" : ""}` : "Garder ma main"}</button></div>`;
  }
  if (a.kind === "may") return `<span class="ft">${who}Utiliser cet effet ?</span><div class="prow"><button class="go" data-a="0">Oui</button><button data-a="1">Non</button></div>`;
  const pickable = (a.uids || []).some(u => u !== null) || (a.zs || []).some(z => z !== null);
  const loose = a.options.map((o, i) => i).filter(i => !((a.uids || [])[i] !== null && (a.uids || [])[i] !== undefined) && !((a.zs || [])[i] !== null && (a.zs || [])[i] !== undefined));
  if (pickable) {
    const p = mode.pick;
    return `<span class="ft">${who}${esc(a.title)}${p !== null ? ` <b>${esc(a.options[p])}</b>` : " Touche une carte ou une zone en surbrillance."}</span>
      <div class="prow"><button class="go" id="fOk" ${p !== null ? "" : "disabled"}>Valider</button>${loose.map(i => `<button data-a="${i}">${esc(cap(a.options[i]))}</button>`).join("")}</div>`;
  }
  const asCards = a.options.every(o => CARDS[nm(o)] || IMG[nm(o)]);
  if (asCards) return `<span class="ft">${who}${esc(a.title)}</span><div class="mull pick">${a.options.map((o, i) => `<button data-a="${i}" title="${esc(o)}">${card(nm(o), { raw: true, cost: true })}</button>`).join("")}</div>`;
  return `<span class="ft">${who}${esc(a.title)}</span><div class="prow">${a.options.map((o, i) => `<button class="go" data-a="${i}">${esc(cap(o))}</button>`).join("")}</div>`;
}
// Libellé court d'un coup quand plusieurs variantes existent : Accelerate / Normal, Repeat, zone…
function variantLabel(o, cands) {
  const plays = cands.every(x => x.k === "play" || x.k === "hide");
  if (!plays) return cap(o.label);
  const locs = new Set(cands.map(x => S(x.loc)));
  const parts = [];
  if (locs.size > 1) parts.push((o.k === "hide" ? "Cacher → " : "") + cap(locName(o.loc)));
  if (cands.some(x => x.acc)) parts.push(o.acc ? accTxt(o) : "Normal");
  if (cands.some(x => x.rep)) parts.push(o.rep ? "Avec Repeat" : "Sans Repeat");
  if (cands.some(x => x.from_ === "trash")) parts.push(o.from_ === "trash" ? "Depuis la défausse" : "Depuis la main");
  return parts.length ? parts.join(" · ") : cap(o.label);
}
function accTxt(o) {
  const n = (srcEl(S(ALIAS[o.src] ?? o.src)) || {}).dataset; const c = CARDS[n ? n.card : ""] || {};
  return `Accelerate (+1 énergie +1 ${(c.d || ["rune"])[0]}, arrive prête)`;
}
function fbarHTML() {
  if (V && V.winner !== null && V.winner !== undefined && !working) {
    const w = V.winner === 0;
    return `<span class="ft big ${w ? "me" : "opp"}">${w ? "Victoire !" : V.winner === 1 ? "Défaite" : "Partie nulle"} ${V.st.pts[0]}-${V.st.pts[1]}</span>
      <div class="prow"><button class="go" id="bAgain">Nouvelle partie</button><button id="bSame">Rejouer la même donne</button></div>`;
  }
  if (!mode || working) return "";
  if (mode.type === "ask") return askHTML();
  if (mode.type === "target" && mvMode()) {
    const ph = tPhase(), ok = exactT().length > 0;
    const steps = [["1", "Ennemi", mode.P.length ? cname(mode.P[0]) : mode.skip ? "aucun" : null, ph === "t1"],
      ["2", "Allié à déplacer", mode.M !== null ? cname(mode.M) : null, ph === "mover"],
      ["3", "Vers", mode.D !== null ? locName(mode.D) : null, ph === "dest"]];
    const help = { t1: "Touche l'ennemi qui prend les dégâts.", mover: "Touche l'allié à déplacer.", dest: "Touche la zone où il va.", done: "Tout est choisi." }[ph];
    const canSkip = ph === "t1" && !mode.P.length && mode.cands.some(o => !hasT(o));
    return `<span class="ft"><b>${esc(cname(mode.src))}</b> · ${help}</span>
      <div class="steps">${steps.map(([n, t, v, on]) => `<span class="step ${on ? "on" : ""} ${v !== null ? "ok" : ""}"><i>${n}</i>${t}${v !== null ? ` : <b>${esc(v)}</b>` : ""}</span>`).join("")}</div>
      <div class="prow"><button class="go" id="fOk" ${ok ? "" : "disabled"}>Valider</button>${canSkip ? `<button id="fSkip">Pas d'ennemi</button>` : ""}<button id="fNo">Annuler</button></div>`;
  }
  if (mode.type === "target") {
    const max = Math.max(...mode.cands.map(o => o.t1.length)), ok = exactT().length > 0;
    return `<span class="ft">Cibles ${mode.P.length}/${max} : ${mode.P.length < max ? "touche une carte en surbrillance" : "valide"}${mode.P.length ? " (retouche une cible pour l'enlever)" : ""}</span>
      <div class="prow"><button class="go" id="fOk" ${ok ? "" : "disabled"}>Valider la cible</button><button id="fNo">Annuler</button></div>`;
  }
  if (mode.type === "move") {
    const o = moveOpt(), d = mode.dest;
    const enemy = d !== null && d !== "base" && V.st.bfs[+d].u.some(u => u.c === 1);
    const txt = `${mode.units.length} unité${mode.units.length > 1 ? "s" : ""} → ${d === null ? "touche la destination" : locName(d)}`;
    const why = d !== null && !o ? `<span class="ft flash">Ces unités ne peuvent pas partir ensemble vers cette zone.</span>` : "";
    return `<span class="ft">${esc(txt)}. Touche d'autres unités pour les ajouter.</span>${why}
      <div class="prow"><button class="${enemy ? "fight" : "go"}" id="fOk" ${o ? "" : "disabled"}>${enemy ? "⚔ Lancer le combat" : "Lancer le déplacement"}</button><button id="fNo">Annuler</button></div>`;
  }
  if (mode.type === "list") {
    const nmx = (srcEl(sel) || {}).dataset;
    return `<span class="ft">${nmx && nmx.card ? `<b>${esc(nmx.card)}</b> : ` : ""}comment le jouer ?</span><div class="prow col">${mode.cands.map(o => `<button class="go" data-i="${o.i}">${esc(variantLabel(o, mode.cands))}</button>`).join("")}<button id="fNo">Annuler</button></div>`;
  }
  return "";
}
$("fbar").addEventListener("click", e => {
  e.stopPropagation();
  const m = e.target.closest("[data-m]");
  if (m && mode && mode.type === "ask") {
    const i = +m.dataset.m;
    if (mode.mull.has(i)) mode.mull.delete(i); else if (mode.mull.size < 2) mode.mull.add(i);
    return decorate();
  }
  const b = e.target.closest("button"); if (!b) return;
  if (b.id === "bAgain") return setupForm();
  if (b.id === "bSame") { setupForm(); $("sSeed").value = META.seed; return; }
  if (b.id === "bKeep") return answer([...mode.mull]);
  if (b.dataset.a) return answer(+b.dataset.a);
  if (b.id === "fOk" && mode.type === "ask") return answer(mode.pick);
  if (b.id === "fNo") { reset(); return decorate(); }
  if (b.id === "fSkip" && mode && mode.type === "target") { mode.skip = true; return decorate(); }
  if (b.dataset.i) return choose(+b.dataset.i);
  if (b.id === "fOk" && mode.type === "target") return run(exactT());
  if (b.id === "fOk" && mode.type === "move") { const o = moveOpt(); if (o) choose(o.i); }
});
// La bulle se pose entre les deux battlefields (là où il n'y a pas d'unités) ; le message de LeBlanc juste au-dessus.
function placeFloat() {
  const row = document.querySelector("#board .bfrow"), F = $("fbar"), Tt = $("aiLine");
  if (!row || innerWidth <= 820) { F.style.left = F.style.top = Tt.style.left = Tt.style.top = ""; return; }
  const r = row.getBoundingClientRect(), cx = r.left + r.width / 2;
  if (!F.hidden) {
    // Trois places possibles : milieu des battlefields, au-dessus de la main de LeBlanc, au niveau de ta main.
    // On prend la première qui ne cache ni une carte en surbrillance (cibles, source, unités à déplacer) ni une unité sur un battlefield.
    const w = F.offsetWidth, h = F.offsetHeight, b = $("board").getBoundingClientRect();
    const key = [...document.querySelectorAll("#board .drop-ok, #board .picked, #board .isel, #board .staged, #board .bf .card")].map(x => x.getBoundingClientRect());
    const oh = document.querySelector("#board .hand.top"), mh = document.querySelector("#board .p0 .hand");
    const spots = [[cx, r.top + r.height / 2]];
    if (oh) { const q = oh.getBoundingClientRect(); spots.push([b.left + b.width / 2, q.top + q.height / 2]); }
    if (mh) { const q = mh.getBoundingClientRect(); spots.push([b.left + w / 2 + 8, q.top + q.height / 2], [b.right - w / 2 - 8, q.top + q.height / 2]); }
    const box = ([x, y]) => { const L = Math.max(8, Math.min(innerWidth - w - 8, x - w / 2)), T = Math.max(8, Math.min(innerHeight - h - 8, y - h / 2)); return { L, T, R: L + w, B: T + h }; };
    const hit = q => key.some(k => k.left < q.R && k.right > q.L && k.top < q.B && k.bottom > q.T);
    const best = spots.map(box).find(q => !hit(q)) || box(spots[oh ? 1 : 0]);
    F.style.left = best.L + "px"; F.style.top = best.T + "px";
  }
  if (!Tt.hidden) {
    const w = Tt.offsetWidth;
    Tt.style.left = Math.max(8, cx - w / 2) + "px";
    Tt.style.top = Math.max(8, r.top - Tt.offsetHeight / 2) + "px";
  }
}

// ---------- rendu des états
function decorate() {
  document.querySelectorAll("#board .drop-ok, #board .drop-hover, #board .picked, #board .staged, #board .isel").forEach(x => { x.classList.remove("drop-ok", "drop-hover", "picked", "staged", "isel"); delete x.dataset.n; });
  const active = V && (V.dec || V.ask) && !working;
  if (!active) { mode = null; sel = null; }
  if (sel !== null && !mode) { const el = srcEl(sel); if (el) el.classList.add("isel"); }
  if (mode && mode.type === "target") {
    const ph = tPhase();
    if (ph === "t1") targetable().forEach(u => { const x = unitEl(u); if (x) x.classList.add("drop-ok"); });
    if (ph === "mover") moverSet().forEach(u => { const x = unitEl(u); if (x) x.classList.add("drop-ok"); });
    if (ph === "dest" || ph === "done") destSet().forEach(z => { const x = zoneEl(z); if (x) x.classList.add(z === mode.D ? "drop-hover" : "drop-ok"); });
    if (mode.M !== null) { const x = unitEl(mode.M); if (x) { x.classList.add("staged"); } }
    mode.P.forEach((u, k) => { const x = unitEl(u); if (x) { x.classList.add("picked"); x.dataset.n = (x.dataset.n ? x.dataset.n + "+" : "") + (k + 1); } });
    const se = srcEl(mode.src); if (se) se.classList.add("isel");
  }
  if (mode && mode.type === "ask") {
    (mode.a.uids || []).forEach((u, i) => { if (u === null) return; const x = srcEl(u); if (x) { x.classList.add(i === mode.pick ? "picked" : "drop-ok"); if (i === mode.pick) x.dataset.n = "✓"; } });
    (mode.a.zs || []).forEach((z, i) => { if (z === null) return; const x = zoneEl(z); if (x) x.classList.add(i === mode.pick ? "drop-hover" : "drop-ok"); });
    const se = mode.a.src !== null && mode.a.src !== undefined ? srcEl(mode.a.src) : null; if (se) se.classList.add("isel");
  }
  if (mode && mode.type === "move") {
    mode.units.forEach(u => { const x = unitEl(u); if (x) x.classList.add("staged"); });
    moveDests(mode.units).forEach(z => { const x = zoneEl(z); if (x) x.classList.add("drop-ok"); });
  }
  if (sel !== null && !mode && active) { $("pop").innerHTML = popHTML(); placePop(); } else $("pop").hidden = true;
  const fb = fbarHTML(); $("fbar").innerHTML = fb; $("fbar").hidden = !fb;
  placeFloat();
  drawArrows();
}

// ---------- toucher
document.addEventListener("click", e => {
  if (noClick) { noClick = false; return; }
  if (e.target.closest("#pop, #fbar, #zoom, #hpop")) return;
  if (!$("hpop").hidden && !e.target.closest("#bHint")) $("hpop").hidden = true;
  const el = e.target.closest("[data-card]");
  if (e.target.closest("#modal")) { if (el && el.dataset.card) showZoom(el); else Z.hidden = true; return; }
  const tr = e.target.closest("#board [data-trash]");
  if (tr) { Z.hidden = true; return openTrash(+tr.dataset.trash); }
  if (V && V.ask && mode && mode.type === "ask" && !working && e.target.closest("#board")) {
    const u = e.target.closest("#board [data-uid]"), z = e.target.closest("#board [data-drop]");
    let i = u ? (mode.a.uids || []).findIndex(x => x !== null && String(x) === u.dataset.uid) : -1;
    if (i < 0 && z) i = (mode.a.zs || []).findIndex(x => x !== null && x === z.dataset.drop);
    if (i >= 0) { mode.pick = mode.pick === i ? null : i; Z.hidden = true; decorate(); return; }
  }
  if (V && V.dec && !working && e.target.closest("#board")) {
    const u = e.target.closest("#board [data-uid]"), z = e.target.closest("#board [data-drop]");
    if (mode && mode.type === "target") {
      const ph = tPhase(), drop = id => { mode.P.splice(mode.P.lastIndexOf(id), 1); mode.M = mode.D = null; mode.skip = false; decorate(); };
      if (mode.M !== null && u && u.dataset.uid === mode.M) { mode.M = mode.D = null; decorate(); return; }
      if (ph === "mover" && u && moverSet().has(u.dataset.uid)) { mode.M = u.dataset.uid; decorate(); return; }
      if ((ph === "dest" || ph === "done") && z && destSet().has(z.dataset.drop)) { mode.D = z.dataset.drop; decorate(); return; }
      if (u && mode.P.includes(u.dataset.uid) && !u.classList.contains("drop-ok")) return drop(u.dataset.uid);
      if (ph === "t1" && u && u.classList.contains("drop-ok")) { addTarget(u.dataset.uid); return; }
      if (u && mode.P.includes(u.dataset.uid)) return drop(u.dataset.uid);
    } else if (mode && mode.type === "move") {
      if (u && u.classList.contains("can") && canMove(u.dataset.uid) && !u.closest(".hand")) {
        const id = u.dataset.uid;
        mode.units = mode.units.includes(id) ? mode.units.filter(x => x !== id) : [...mode.units, id];
        if (!mode.units.length) mode = null;
        decorate(); return;
      }
      if (z && z.classList.contains("drop-ok")) { mode.dest = z.dataset.drop; decorate(); return; }
    } else if (u && u.classList.contains("can")) {
      mode = null; sel = sel === u.dataset.uid ? null : u.dataset.uid; Z.hidden = true;
      decorate(); return;
    } else if (sel !== null) { sel = null; decorate(); }
  } else if (sel !== null && !e.target.closest("#ctrl")) { sel = null; if (V) decorate(); }
  if (el && el.dataset.card && !el.closest(".mull")) showZoom(el); else Z.hidden = true;
});

// ---------- glisser
document.addEventListener("pointerdown", e => {
  if (!V || !V.dec || working || e.button > 0) return;
  const c = e.target.closest("#board [data-uid].can");
  if (!c || (mode && mode.type === "target")) return;
  drag = { src: c.dataset.uid, el: c, x0: e.clientX, y0: e.clientY, on: false };
});
document.addEventListener("pointermove", e => {
  if (!drag) return;
  if (!drag.on) {
    if (Math.hypot(e.clientX - drag.x0, e.clientY - drag.y0) < 8) return;
    drag.on = true; Z.hidden = true; $("pop").hidden = true;
    if (!(mode && mode.type === "move")) { mode = null; sel = drag.src; }
    const r = drag.el.getBoundingClientRect(), g = drag.el.cloneNode(true);
    g.classList.add("ghost"); g.classList.remove("isel"); g.style.width = r.width + "px"; g.style.height = r.height + "px";
    document.body.appendChild(g); drag.g = g; drag.w = r.width; drag.h = r.height;
    // surbrillance pendant le glisser : cibles possibles, zones de jeu ou de déplacement
    const ops = optsFor(drag.src);
    ops.forEach(o => { if (hasT(o)) o.t1.forEach(u => { const x = unitEl(u); if (x) x.classList.add("drop-ok"); });
      else if (o.k === "move") { if (contains(o.us, [drag.src])) { const x = zoneEl(o.loc); if (x) x.classList.add("drop-ok"); } }
      else if (o.loc !== null && o.loc !== undefined) { const x = zoneEl(o.loc); if (x) x.classList.add("drop-ok"); } });
  }
  drag.g.style.left = (e.clientX - drag.w / 2) + "px"; drag.g.style.top = (e.clientY - drag.h / 2) + "px";
  document.querySelectorAll(".drop-hover").forEach(x => x.classList.remove("drop-hover"));
  const over = document.elementFromPoint(e.clientX, e.clientY), t = over && over.closest(".drop-ok");
  if (t) t.classList.add("drop-hover");
  drawArrows(e.clientX, e.clientY);
});
document.addEventListener("pointerup", e => {
  if (!drag) return;
  const d = drag; drag = null;
  if (!d.on) return;
  noClick = true; setTimeout(() => { noClick = false; }, 50);
  d.g.remove();
  document.querySelectorAll(".drop-hover").forEach(x => x.classList.remove("drop-hover"));
  const over = document.elementFromPoint(e.clientX, e.clientY);
  dropAt(d.src, over && over.closest("#board [data-uid], #board [data-drop]"));
});
document.addEventListener("pointercancel", () => { if (drag && drag.g) drag.g.remove(); drag = null; });
function dropAt(src, t) {
  if (!t || (t.dataset.uid === src)) { if (!(mode && mode.type === "move")) sel = null; return decorate(); }
  const ops = optsFor(src), isUnit = !!(t.dataset.uid && !t.dataset.drop);
  // 1. sort ou capacité ciblée lâchée sur une unité : on passe au choix des cibles, rien n'est joué avant « Valider »
  if (isUnit && ops.some(o => hasT(o) && o.t1.map(String).includes(t.dataset.uid))) { sel = src; return startTarget(ops.filter(o => hasT(o) && o.t1.map(String).includes(t.dataset.uid)), [t.dataset.uid]); }
  const z = (isUnit ? t.closest("[data-drop]") : t);
  const zone = z ? z.dataset.drop : null;
  if (zone === null) { sel = null; return decorate(); }
  // 2. unité déplacée : déplacement préparé, lancé ensuite par le bouton
  if (ops.some(o => o.k === "move")) {
    if (mode && mode.type === "move" && mode.dest !== null && mode.dest !== zone) mode = null;
    return startMove(src, zone);
  }
  // 3. carte jouée sur une zone
  if (ops.some(hasMv)) { sel = src; return startTarget(ops.filter(needs)); }
  const m = ops.filter(o => o.k !== "move" && (o.loc !== null && o.loc !== undefined ? String(o.loc) === zone : true));
  if (m.some(needs)) { sel = src; return startTarget(m); }
  sel = null; run(m);
}

// ---------- flèches : tes cibles (vert), sorts de la chaîne et dernier coup de LeBlanc (violet), déplacements préparés
function centre(el) { const r = el.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; }
function arrow(a, b, who, dashed) {
  const [x1, y1] = a, [x2, y2] = b, dx = x2 - x1, dy = y2 - y1, L = Math.hypot(dx, dy) || 1;
  const ex = x2 - dx / L * 14, ey = y2 - dy / L * 14, bend = Math.min(80, L / 4);
  const cx = (x1 + ex) / 2 - dy / L * bend, cy = (y1 + ey) / 2 + dx / L * bend;
  const col = who === 0 ? "#3fc3a3" : "#b58cf0";
  return `<path d="M${x1},${y1} Q${cx},${cy} ${ex},${ey}" fill="none" stroke="${col}" stroke-width="3" stroke-linecap="round" ${dashed ? 'stroke-dasharray="7 6"' : ""} marker-end="url(#ah${who === 0 ? "A" : "L"})" opacity=".9"/>`;
}
function drawArrows(px, py) {
  let h = "";
  if (drag && drag.on && px !== undefined) {
    optsFor(drag.src).filter(hasT).forEach(o => o.t1.forEach(u => { const t = unitEl(u); if (t) h += arrow([px, py], centre(t), 0, true); }));
  } else if (mode && mode.type === "target") {
    const se = srcEl(mode.src);
    if (se) {
      const ph = tPhase();
      if (ph === "t1") targetable().forEach(u => { const t = unitEl(u); if (t) h += arrow(centre(se), centre(t), 0, true); });
      [...new Set(mode.P)].forEach(u => { const t = unitEl(u); if (t) h += arrow(centre(se), centre(t), 0, false); });
      const me = mode.M !== null ? unitEl(mode.M) : null;
      if (me) {
        h += arrow(centre(se), centre(me), 0, false);
        if (mode.D !== null) { const zt = zoneEl(mode.D); if (zt) h += arrow(centre(me), centre(zt), 0, false); }
        else destSet().forEach(z => { const zt = zoneEl(z); if (zt) h += arrow(centre(me), centre(zt), 0, true); });
      }
    }
  } else if (mode && mode.type === "ask") {
    const fromEl = (mode.a.src !== null && mode.a.src !== undefined && srcEl(mode.a.src)) || document.querySelector("#stack .sitem .card");
    if (fromEl) (mode.a.uids || []).forEach((u, i) => { if (u === null || String(u) === String(mode.a.src)) return; const t = srcEl(u); if (t) h += arrow(centre(fromEl), centre(t), 0, i !== mode.pick); });
  } else if (mode && mode.type === "move" && mode.dest !== null) {
    const zt = zoneEl(mode.dest);
    if (zt) mode.units.forEach(u => { const x = unitEl(u); if (x) h += arrow(centre(x), centre(zt), 0, false); });
  } else if (sel !== null) {
    const se = srcEl(sel);
    if (se) { const us = new Set(); optsFor(sel).filter(hasT).forEach(o => o.t1.forEach(u => us.add(String(u)))); us.forEach(u => { const t = unitEl(u); if (t) h += arrow(centre(se), centre(t), 0, true); }); }
  }
  const seen = new Set();
  if (V) (V.st.chain || []).forEach((c, k) => {
    const it = document.querySelector(`#stack [data-ch="${k}"] .card`); if (!it || !c.tg) return;
    c.tg.forEach(u => { const t = unitEl(u); if (t) { h += arrow(centre(it), centre(t), c.c, false); seen.add(c.c + ":" + u); } });
  });
  if (lastAI && V) {
    const tg = [...(lastAI.t1 || []), ...(lastAI.t2 || [])];
    const fromEl = document.querySelector("#stack .sitem.c1 .card") || document.querySelector("#board .hand.top");
    if (fromEl) tg.forEach(u => { const t = unitEl(u); if (t && !seen.has("1:" + u)) h += arrow(centre(fromEl), centre(t), 1, false); });
  }
  $("arrowsG").innerHTML = h;
}
let rafA = 0;
const redraw = () => { cancelAnimationFrame(rafA); rafA = requestAnimationFrame(() => { drawArrows(); placeFloat(); if (!$("pop").hidden) placePop(); if (!$("hpop").hidden) placeHint(); }); };
addEventListener("scroll", redraw, { passive: true }); addEventListener("resize", redraw);
document.addEventListener("keydown", e => {
  if (e.key === "Enter" && e.target.closest && e.target.closest("[data-trash]")) return openTrash(+e.target.dataset.trash);
  if (e.key === "Escape" && !$("modal").hidden && $("modal").dataset.closable) return closeModal();
  if (e.key === "Escape") { $("hpop").hidden = true; Z.hidden = true; if ((sel !== null || mode) && V) { reset(); decorate(); } } });


if (store.get("rbt-journal")) journal(true); else badge();
document.addEventListener("keydown", e => { if (e.key === "Escape" && $("drawer").classList.contains("open") && $("modal").hidden) journal(false); });
