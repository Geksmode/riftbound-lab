// Mini-moteur Riftbound pour les puzzles sur plateau.
// Reprend les règles du moteur fidèle (../engine/game.py) sur le sous-ensemble utile aux puzzles :
// chaîne (le joueur qui joue garde la priorité ; deux passes de suite = résolution du haut de la chaîne),
// showdown (focus, l'attaquant/le contestataire d'abord), combat (somme des might hors unités étourdies,
// l'attaquant répartit d'abord, létal complet avant l'unité suivante, Tank d'abord, rappel des attaquants si
// des défenseurs restent), contrôle et conquête, paiement en runes (épuiser = 1 énergie, recycler = 1 power du
// domaine, la même rune peut faire les deux), Hidden, Accelerate, Flow, réduction de l'Astral Heron, légende
// d'Akali (Empower et repli).
// Aucune restriction ajoutée : toute action légale selon les règles et le texte des cartes est proposée.
(function (root) {
'use strict';

const ME = 'me', OP = 'op';
const other = p => (p === ME ? OP : ME);
const ANY = ['Fury', 'Calm', 'Body', 'Mind', 'Chaos', 'Order'];

// ------------------------------------------------------------------ cartes
const C = {};
function unitCard(name, dom, e, p, might, extra) { C[name] = Object.assign({ name, type: 'unit', dom, e, p, might, timing: 'main' }, extra || {}); }
function spell(name, dom, e, p, timing, extra) { C[name] = Object.assign({ name, type: 'spell', dom, e, p, timing }, extra); }

unitCard("Kai'Sa, Survivor", ['Fury'], 4, 0, 4, { fr: "Kai'Sa", onConquer: 'draw', accelerate: true, text: 'Accelerate (1 énergie + 1 rune Fury en plus : arrive prête). Quand elle conquiert, pioche 1.' });
unitCard('Mournful Witness', ['Calm'], 2, 0, 2, { fr: 'Mournful Witness', empBonus: 2, text: 'Quand un combat où elle était se termine, elle devient Empowered (+2).' });
unitCard('Astral Heron', ['Calm'], 7, 0, 7, { fr: 'Astral Heron', heron: true, text: 'Quand tu joues ta 1re carte du tour, s\'il est sur un champ de bataille, ta carte suivante coûte 2 énergie et 2 runes de moins.' });
unitCard('Pit Rookie', ['Body'], 2, 0, 2, { fr: 'Pit Rookie' });
unitCard('First Mate', ['Body'], 3, 0, 3, { fr: 'First Mate' });
unitCard('Lonely Poro', ['Calm'], 2, 0, 2, { fr: 'Lonely Poro' });
unitCard('Master Yi, Tempered', ['Body'], 4, 0, 4, { fr: 'Master Yi', text: 'Hunt 2.' });
unitCard('Ruin Runner', ['Body'], 6, 0, 5, { fr: 'Ruin Runner', untargetable: true, text: 'Ne peut pas être choisi par les sorts et capacités adverses.' });
unitCard('Rengar, Trophy Hunter', ['Body'], 5, 1, 6, { fr: 'Rengar', timing: 'reaction', ambush: true, text: 'Ambush. Peut être joué sur un champ de bataille où il y a des unités ennemies.' });

const unitsAt = (s, loc, owner) => s.units.filter(u => u.loc === loc && (!owner || u.owner === owner));
const atBf = u => u.loc !== 'base';
const isFury = u => C[u.name].dom.includes('Fury');
const targetable = (s, u, by) => !(C[u.name].untargetable && u.owner !== by);
const restrict = (src, u) => !src || u.loc === src.bf;   // carte jouée face cachée : choix limités à ce champ de bataille
const LOCS = ['base', 0, 1];

spell('Shuriken Flip', ['Fury', 'Calm'], 1, 1, 'main', {
  fr: 'Shuriken Flip', flow: { e: 3, p: 1 }, text: '2 dégâts à jusqu\'à une unité ennemie sur un champ de bataille, puis déplace une unité alliée. Flow : rejouable depuis la défausse pour 3 énergie + 1 rune.',
  choices(s, pid) {
    const tg = [null].concat(s.units.filter(u => u.owner !== pid && atBf(u) && targetable(s, u, pid)).map(u => u.id));
    const out = [];
    for (const t of tg) for (const m of s.units.filter(u => u.owner === pid))
      for (const to of LOCS) if (to !== m.loc) out.push({ t, m: m.id, to });
    return out;
  },
  chosen: ch => [ch.t, ch.m].filter(x => x != null),
  resolve(s, it) {
    const t = byId(s, it.ch.t);
    if (t && atBf(t)) deal(s, t, 2, it);
    cleanupDeaths(s);
    const m = byId(s, it.ch.m);
    if (m && m.owner === it.owner && m.loc !== it.ch.to) moveUnits(s, [m], it.ch.to, false);
  },
  label: (s, ch) => (ch.t != null ? `2 dégâts à ${nm(s, ch.t)}` : 'aucune cible') + `, puis ${nm(s, ch.m)} → ${locName(s, ch.to)}`,
});
spell('Falling Star', ['Fury'], 2, 2, 'main', {
  fr: 'Falling Star', text: '3 dégâts à une unité, puis 3 dégâts à une unité.',
  choices(s, pid) {
    const ts = s.units.filter(u => targetable(s, u, pid)).map(u => u.id);
    const out = [];
    ts.forEach((a, i) => ts.slice(i).forEach(b => out.push({ a, b })));
    return out;
  },
  chosen: ch => [ch.a, ch.b],
  resolve(s, it) { for (const k of ['a', 'b']) { const u = byId(s, it.ch[k]); if (u) deal(s, u, 3, it); cleanupDeaths(s); } },
  label: (s, ch) => ch.a === ch.b ? `6 dégâts à ${nm(s, ch.a)}` : `3 à ${nm(s, ch.a)} et 3 à ${nm(s, ch.b)}`,
});
spell('Back Off', ['Calm'], 3, 0, 'action', {
  fr: 'Back Off', hidden: true, text: 'Hidden. Action. Étourdit une unité (pas de dégâts de combat ce tour). Pioche 1 si jouée depuis la main.',
  choices(s, pid, src) { return s.units.filter(u => targetable(s, u, pid) && restrict(src, u)).map(u => ({ t: u.id })); },
  chosen: ch => [ch.t],
  resolve(s, it) { const u = byId(s, it.ch.t); if (u) { u.stun = true; log(s, `${nm(s, u.id)} est étourdi.`); } if (!it.fromHidden) draw(s, it.owner); },
  label: (s, ch) => `étourdir ${nm(s, ch.t)}`,
});
spell('Block', ['Calm'], 2, 0, 'action', {
  fr: 'Block', hidden: true, text: 'Hidden. Action. Une unité gagne Shield 3 (+3 en défense) et Tank ce tour.',
  choices(s, pid, src) { return s.units.filter(u => targetable(s, u, pid) && restrict(src, u)).map(u => ({ t: u.id })); },
  chosen: ch => [ch.t],
  resolve(s, it) { const u = byId(s, it.ch.t); if (u) { u.shield += 3; u.tank = true; log(s, `${nm(s, u.id)} gagne Shield 3 et Tank.`); } },
  label: (s, ch) => `Shield 3 et Tank sur ${nm(s, ch.t)}`,
});
spell('Discipline', ['Calm'], 2, 0, 'reaction', {
  fr: 'Discipline', text: 'Réaction. +2 de puissance à une unité ce tour. Pioche 1.',
  choices(s, pid) { return s.units.filter(u => targetable(s, u, pid)).map(u => ({ t: u.id })); },
  chosen: ch => [ch.t],
  resolve(s, it) { const u = byId(s, it.ch.t); if (u) pump(s, u, 2); draw(s, it.owner); },
  label: (s, ch) => `+2 à ${nm(s, ch.t)}`,
});
spell('Defy', ['Calm'], 1, 1, 'reaction', {
  fr: 'Defy', text: 'Réaction. Contre un sort qui coûte au plus 4 énergie et au plus 1 rune.',
  choices(s, pid) { return s.chain.filter(x => C[x.name].type === 'spell' && C[x.name].e <= 4 && C[x.name].p + (x.extraP || 0) <= 1).map(x => ({ uid: x.uid })); },
  resolve(s, it) { counter(s, it.ch.uid); },
  label: (s, ch) => { const x = s.chain.find(y => y.uid === ch.uid); return x ? `contrer ${C[x.name].fr} (${x.owner === ME ? 'le tien' : 'de Yi'})` : 'contrer'; },
});
spell('Not So Fast', ['Calm'], 2, 1, 'reaction', {
  fr: 'Not So Fast', text: 'Réaction. Contre un sort ou une capacité adverse qui choisit une de tes unités.',
  choices(s, pid) {
    return s.chain.filter(x => x.owner !== pid && (C[x.name].chosen ? C[x.name].chosen(x.ch) : []).some(id => { const u = byId(s, id); return u && u.owner === pid; }))
      .map(x => ({ uid: x.uid }));
  },
  resolve(s, it) { counter(s, it.ch.uid); },
  label: (s, ch) => { const x = s.chain.find(y => y.uid === ch.uid); return x ? `contrer ${C[x.name].fr}` : 'contrer'; },
});
// --- Yi
spell('En Garde', ['Calm'], 1, 0, 'reaction', {
  fr: 'En Garde', text: 'Réaction. +1 à une unité alliée, puis +1 encore si c\'est ta seule unité à cet endroit.',
  choices(s, pid) { return s.units.filter(u => u.owner === pid).map(u => ({ t: u.id })); },
  chosen: ch => [ch.t],
  resolve(s, it) { const u = byId(s, it.ch.t); if (!u) return; pump(s, u, 1); if (unitsAt(s, u.loc, u.owner).length === 1) pump(s, u, 1); },
  label: (s, ch) => `sur ${nm(s, ch.t)}`,
});
spell('Punch First', ['Body'], 1, 2, 'action', {
  fr: 'Punch First', text: 'Action. +5 de puissance à une unité ce tour.',
  choices(s, pid) { return s.units.filter(u => targetable(s, u, pid)).map(u => ({ t: u.id })); },
  chosen: ch => [ch.t],
  resolve(s, it) { const u = byId(s, it.ch.t); if (u) pump(s, u, 5); },
  label: (s, ch) => `+5 à ${nm(s, ch.t)}`,
});
spell('Decree of Focus', ['Calm'], 1, 0, 'reaction', {
  fr: 'Decree of Focus', text: 'Réaction. +4 à une unité alliée en combat contre une unité Fury, ou choisie par un sort Fury adverse.',
  choices(s, pid) {
    return s.units.filter(u => u.owner === pid && (
      (s.sd && s.sd.combat && u.loc === s.sd.bf && unitsAt(s, u.loc, other(pid)).some(isFury)) ||
      s.chain.some(x => x.owner !== pid && C[x.name].dom.includes('Fury') && C[x.name].chosen && C[x.name].chosen(x.ch).includes(u.id))
    )).map(u => ({ t: u.id }));
  },
  chosen: ch => [ch.t],
  resolve(s, it) {
    // la condition de choix est revérifiée à la résolution (cible devenue illégale = sans effet)
    const u = byId(s, it.ch.t);
    if (!u) return;
    const ok = (s.sd && s.sd.combat && u.loc === s.sd.bf && unitsAt(s, u.loc, other(it.owner)).some(isFury)) ||
      s.chain.some(x => x.owner !== it.owner && C[x.name].dom.includes('Fury') && C[x.name].chosen && C[x.name].chosen(x.ch).includes(u.id));
    if (ok) pump(s, u, 4); else log(s, 'Decree of Focus n\'a plus de cible légale : sans effet.');
  },
  label: (s, ch) => `+4 à ${nm(s, ch.t)}`,
});
spell('Charm', ['Calm'], 1, 1, 'main', {
  fr: 'Charm', text: 'Déplace une unité ennemie.',
  choices(s, pid) {
    const out = [];
    for (const u of s.units.filter(x => x.owner !== pid && targetable(s, x, pid))) for (const to of LOCS) if (to !== u.loc) out.push({ t: u.id, to });
    return out;
  },
  chosen: ch => [ch.t],
  resolve(s, it) { const u = byId(s, it.ch.t); if (u && u.loc !== it.ch.to) moveUnits(s, [u], it.ch.to, false); },
  label: (s, ch) => `${nm(s, ch.t)} → ${locName(s, ch.to)}`,
});
spell('Rampage', ['Body'], 3, 0, 'main', {
  fr: 'Rampage', text: 'Une unité alliée et une unité ennemie s\'infligent leur puissance (+2 à l\'alliée pour 1 rune Body en plus).',
  choices(s, pid) {
    const out = [];
    for (const f of s.units.filter(u => u.owner === pid)) for (const e of s.units.filter(u => u.owner !== pid && targetable(s, u, pid))) { out.push({ f: f.id, e: e.id, x: 0 }); out.push({ f: f.id, e: e.id, x: 1 }); }
    return out;
  },
  extraP: ch => ch.x,
  chosen: ch => [ch.f, ch.e],
  resolve(s, it) {
    const f = byId(s, it.ch.f), e = byId(s, it.ch.e);
    if (!f || !e) return;
    if (it.ch.x) pump(s, f, 2);
    const mf = might(s, f), me = might(s, e);
    deal(s, e, mf, it); deal(s, f, me, it);
    cleanupDeaths(s);
  },
  label: (s, ch) => `${nm(s, ch.f)} contre ${nm(s, ch.e)}${ch.x ? ' (+2, 1 rune Body en plus)' : ''}`,
});
// --- capacités de la légende d'Akali (pas des cartes : elles vont sur la chaîne mais ne sont pas des sorts)
C['Akali: Empower'] = { name: 'Akali: Empower', type: 'ability', dom: ['Fury', 'Calm'], e: 3, p: 1, anyPower: true, timing: 'main', fr: 'Akali (légende) : Empower',
  resolve(s, it) { s.legendState[it.owner].emp = true; log(s, 'Akali est Empowered.'); }, label: () => 'Empower la légende' };
C['Akali: Repli'] = { name: 'Akali: Repli', type: 'ability', dom: ['Fury', 'Calm'], e: 0, p: 0, timing: 'action', fr: 'Akali (légende) : repli',
  choices(s, pid) { return s.sd && s.turn === pid ? s.units.filter(u => u.owner === pid && u.loc === s.sd.bf).map(u => ({ t: u.id })) : []; },
  chosen: ch => [ch.t],
  resolve(s, it) {
    const u = byId(s, it.ch.t);
    if (!u || s.turn !== it.owner || !s.sd || u.loc !== s.sd.bf) return;
    moveUnits(s, [u], 'base', false); u.desig = null;
    if (s.legendState[it.owner].emp) { u.ready = true; log(s, `${nm(s, u.id)} est redressée.`); }
  },
  label: (s, ch) => `ramener ${nm(s, ch.t)} en base${s.legendState.me.emp ? ' et la redresser' : ''}` };

// ------------------------------------------------------------------ état
function byId(s, id) { return id == null ? null : s.units.find(u => u.id === id) || null; }
function nm(s, id) { const u = byId(s, id); return u ? C[u.name].fr : '(disparue)'; }
function locName(s, loc) { return loc === 'base' ? 'base' : s.bfs[loc].name; }
function log(s, msg) { if (s.log) s.log.push(msg); }

function might(s, u) {
  let m = C[u.name].might + u.mod;
  if (u.emp && C[u.name].empBonus) m += C[u.name].empBonus;
  if (u.desig === 'def') m += u.shield;
  if (u.desig === 'def' && s.legend[u.owner] === 'bladesman' && unitsAt(s, u.loc, u.owner).length === 1) m += 2;
  return m;
}
function pump(s, u, n) { u.mod += n; log(s, `${C[u.name].fr} +${n} (puissance ${might(s, u)}).`); }
function deal(s, u, n) { if (n <= 0) return; u.dmg += n; log(s, `${C[u.name].fr} subit ${n} dégâts.`); }
function draw(s, pid) { s.draws[pid]++; log(s, `${pid === ME ? 'Tu pioches' : 'Yi pioche'} 1 carte.`); }

function newState(p) {
  let id = 1;
  const s = {
    turn: ME, turnNo: 0, stage: 'main', legend: { me: 'akali', op: 'bladesman' },
    legendState: { me: { exh: false, emp: !!(p.me.legendEmp) }, op: { exh: false, emp: false } },
    bfs: p.bfs.map(b => ({ name: b.name, ctrl: b.ctrl || null, scored: [] })),
    units: [], hidden: (p.hidden || []).map(h => Object.assign({ turn: -1 }, h)),
    hand: { me: p.me.hand.slice(), op: p.op.hand.slice() },
    trash: { me: (p.me.trash || []).slice(), op: (p.op.trash || []).slice() },
    runes: { me: p.me.runes.map(d => ({ d, r: true })), op: p.op.runes.map(d => ({ d, r: true })) },
    pts: { me: 0, op: 0 }, draws: { me: 0, op: 0 }, played: { me: 0, op: 0 }, heronDisc: { me: false, op: false },
    chain: [], prio: null, passes: 0, sd: null, assign: null, contested: [null, null], uid: 1,
    opTurn: p.opTurn || null, deadIds: [], log: [],
  };
  for (const side of [ME, OP]) for (const u of p[side].units) {
    s.units.push({ id: 'u' + id++, name: u.name, owner: side, loc: u.loc, ready: u.ready !== false, dmg: 0, mod: 0, emp: !!u.emp, shield: 0, tank: false, stun: false, desig: null });
  }
  s.nextId = id;
  if (p.startOp) beginOpTurn(s);
  return s;
}
function clone(s) { const log = s.log; s.log = null; const c = JSON.parse(JSON.stringify(s)); s.log = log; c.log = log ? log.slice() : null; return c; }
function key(s) { const log = s.log; s.log = null; const k = JSON.stringify(s); s.log = log; return k; }

// ------------------------------------------------------------------ runes
function planPay(s, pid, e, pReq, keep) {
  const rs = s.runes[pid];
  const ready = rs.map((r, i) => i).filter(i => rs[i].r);
  if (ready.length < e) return null;
  const used = new Set(), recycle = [];
  // les exigences les plus restrictives d'abord ; recycler de préférence des runes déjà épuisées
  const reqs = pReq.slice().sort((a, b) => a.doms.length - b.doms.length);
  for (const req of reqs) {
    const cand = rs.map((r, i) => i).filter(i => !used.has(i) && req.doms.includes(rs[i].d));
    cand.sort((a, b) => (rs[a].r - rs[b].r) || ((keep[rs[a].d] || 0) - (keep[rs[b].d] || 0)));
    if (!cand.length) return null;
    used.add(cand[0]); recycle.push(cand[0]);
  }
  const readyRec = recycle.filter(i => rs[i].r);
  const exhaust = readyRec.slice(0, e);
  if (exhaust.length < e) {
    const rest = ready.filter(i => !used.has(i)).sort((a, b) => (keep[rs[a].d] || 0) - (keep[rs[b].d] || 0));
    if (rest.length < e - exhaust.length) return null;
    exhaust.push(...rest.slice(0, e - exhaust.length));
  }
  return { exhaust, recycle };
}
function keepValue(s, pid, except) {
  const k = {};
  s.hand[pid].forEach((n, i) => { if (i === except) return; const c = C[n]; if (c.p > 0) for (const d of c.dom) k[d] = (k[d] || 0) + 1; });
  return k;
}
function pay(s, pid, plan) {
  for (const i of plan.exhaust) s.runes[pid][i].r = false;
  const rec = new Set(plan.recycle);
  s.runes[pid] = s.runes[pid].filter((r, i) => !rec.has(i));
}
// coût d'une action de jeu : src = 'hand' | 'hidden' | 'flow' | 'ability'
function costOf(s, pid, name, ch, src) {
  const c = C[name];
  let e, pReq = [];
  if (src === 'flow') { e = c.flow.e; pReq = Array.from({ length: c.flow.p }, () => ({ doms: ANY })); }
  else {
    e = src === 'hidden' ? 0 : c.e;
    const doms = c.anyPower ? ANY : name === 'Rampage' ? ['Body'] : c.dom;
    const np = c.p + (c.extraP ? c.extraP(ch) : 0);
    pReq = Array.from({ length: np }, () => ({ doms }));
    if (ch && ch.acc) { e += 1; pReq.push({ doms: ['Fury'] }); }
  }
  if (src !== 'ability' && s.heronDisc[pid]) {   // Astral Heron : la carte suivante coûte 2 énergie et 2 runes de moins
    e = Math.max(0, e - 2);
    pReq.sort((a, b) => a.doms.length - b.doms.length); pReq = pReq.slice(2);
  }
  return { e, pReq };
}

// ------------------------------------------------------------------ mouvements, contrôle
function moveUnits(s, us, to, standard) {
  for (const u of us) { if (standard) u.ready = false; u.loc = to; }
  log(s, `${us.map(u => C[u.name].fr).join(' et ')} ${us.length > 1 ? 'vont' : 'va'} → ${locName(s, to)}.`);
  if (to !== 'base') {
    const b = s.bfs[to], mover = us[0].owner;
    if (s.contested[to] == null && b.ctrl !== mover) s.contested[to] = mover;
    if (s.sd && s.sd.combat && s.sd.bf === to) for (const u of us) { u.desig = u.owner === s.sd.att ? 'att' : 'def'; if (!s.sd.members.includes(u.id)) s.sd.members.push(u.id); }
  } else for (const u of us) u.desig = null;
}
function lethal(s, u) { return u.dmg > 0 && u.dmg >= might(s, u); }
function cleanupDeaths(s) {
  const dead = s.units.filter(u => lethal(s, u));
  for (const u of dead) { log(s, `${C[u.name].fr} meurt.`); s.deadIds.push(u.id); }
  if (dead.length) s.units = s.units.filter(u => !dead.includes(u));
}
function toTrash(s, it) { if (C[it.name].type === 'spell') (it.flowed ? null : s.trash[it.owner].push(it.name)); }
function counter(s, uid) {
  const i = s.chain.findIndex(x => x.uid === uid);
  if (i >= 0) { const it = s.chain[i]; log(s, `${C[it.name].fr} est contré.`); s.chain.splice(i, 1); toTrash(s, it); }
}
function establish(s, pid, b) {
  const bf = s.bfs[b];
  bf.ctrl = pid;
  if (!bf.scored.includes(pid)) {
    bf.scored.push(pid); s.pts[pid]++;
    log(s, `${pid === ME ? 'Tu conquiers' : 'Yi conquiert'} ${bf.name} : +1 point.`);
    for (const u of unitsAt(s, b, pid)) if (C[u.name].onConquer === 'draw') draw(s, pid);
  }
}

function cleanup(s) {
  cleanupDeaths(s);
  for (let b = 0; b < 2; b++) {
    const bf = s.bfs[b];
    if (bf.ctrl && !unitsAt(s, b, bf.ctrl).length && !s.chain.length && !(s.sd && s.sd.bf === b)) {
      log(s, `${bf.ctrl === ME ? 'Tu perds' : 'Yi perd'} le contrôle de ${bf.name}.`); bf.ctrl = null;
    }
    if (s.contested[b] && !unitsAt(s, b, s.contested[b]).length && !(s.sd && s.sd.bf === b)) s.contested[b] = null;
    if (!s.contested[b] && !(s.sd && s.sd.bf === b)) {
      const intr = s.units.find(u => u.loc === b && u.owner !== bf.ctrl);
      if (intr) s.contested[b] = intr.owner;
    }
  }
  const lost = s.hidden.filter(h => s.bfs[h.bf].ctrl !== h.owner);
  for (const h of lost) { log(s, `${C[h.name].fr} (face cachée) est défaussée : son champ de bataille est perdu.`); s.trash[h.owner].push(h.name); }
  s.hidden = s.hidden.filter(h => s.bfs[h.bf].ctrl === h.owner);
  if (!s.sd && !s.chain.length) {
    for (let b = 0; b < 2; b++) if (s.contested[b]) {
      const both = unitsAt(s, b, ME).length && unitsAt(s, b, OP).length;
      startShowdown(s, b, !!both);
      return;
    }
  } else if (s.sd && !s.sd.combat && !s.chain.length && unitsAt(s, s.sd.bf, ME).length && unitsAt(s, s.sd.bf, OP).length) {
    log(s, 'Le showdown devient un combat.');
    s.sd.combat = true; s.sd.att = s.contested[s.sd.bf];
    beginCombat(s);
  }
}
function startShowdown(s, b, combat) {
  const att = s.contested[b];
  s.sd = { bf: b, combat, att, focus: att, passes: 0, members: [] };
  log(s, combat ? `Combat sur ${s.bfs[b].name}.` : `Showdown sur ${s.bfs[b].name}.`);
  if (combat) beginCombat(s);
}
function beginCombat(s) {
  const sd = s.sd;
  for (const u of unitsAt(s, sd.bf)) { u.desig = u.owner === sd.att ? 'att' : 'def'; if (!sd.members.includes(u.id)) sd.members.push(u.id); }
  sd.focus = sd.att; sd.passes = 0;
}
function perms(a) { if (a.length <= 1) return [a.slice()]; const out = []; a.forEach((x, i) => perms(a.slice(0, i).concat(a.slice(i + 1))).forEach(p => out.push([x].concat(p)))); return out; }
// ordres de répartition légaux (Tank d'abord) ; le joueur choisit
function assignOrders(s, targets) {
  const tank = targets.filter(u => u.tank), rest = targets.filter(u => !u.tank);
  const out = [];
  for (const a of perms(tank)) for (const b of perms(rest)) out.push(a.concat(b).map(u => u.id));
  return out;
}
function assignDamage(s, total, order) {
  const out = new Map(); let left = total;
  order.forEach((id, i) => {
    const u = byId(s, id); if (!u || left <= 0) return;
    const need = Math.max(1, might(s, u) - u.dmg);
    const n = i === order.length - 1 ? left : Math.min(left, need);
    out.set(u, n); left -= n;
  });
  return out;
}
function combatSides(s) {
  const b = s.sd.bf;
  return { atk: unitsAt(s, b).filter(u => u.desig === 'att'), dfn: unitsAt(s, b).filter(u => u.desig === 'def') };
}
const total = (s, us) => us.filter(u => !u.stun).reduce((t, u) => t + Math.max(0, might(s, u)), 0);
// étape des dégâts : l'attaquant choisit son ordre, puis le défenseur
function startDamage(s) {
  const { atk, dfn } = combatSides(s);
  if (!atk.length || !dfn.length) { finishCombat(s); return; }
  const at = total(s, atk), dt = total(s, dfn);
  log(s, `Dégâts de combat : ${s.sd.att === ME ? 'toi' : 'Yi'} ${at}, ${s.sd.att === ME ? 'Yi' : 'toi'} ${dt}.`);
  s.assign = { at, dt, A: null, D: null };
}
function applyDamage(s) {
  const a = s.assign; s.assign = null;
  const A = assignDamage(s, a.at, a.A || []), D = assignDamage(s, a.dt, a.D || []);
  for (const [u, n] of [...A, ...D]) if (n > 0) { u.dmg += n; log(s, `${C[u.name].fr} subit ${n} dégâts.`); }
  finishCombat(s);
}
function finishCombat(s) {
  const sd = s.sd, b = sd.bf;
  cleanupDeaths(s);
  for (const u of s.units) u.dmg = 0;
  const { atk, dfn } = combatSides(s);
  if (atk.length && dfn.length) { for (const u of atk) u.loc = 'base'; log(s, `Des défenseurs restent : ${atk.map(u => C[u.name].fr).join(', ')} ${atk.length > 1 ? 'retournent' : 'retourne'} en base.`); }
  for (const id of sd.members) { const u = byId(s, id); if (u && C[u.name].empBonus && !u.emp) { u.emp = true; log(s, `${C[u.name].fr} devient Empowered (+2).`); } }
  const owners = [...new Set(unitsAt(s, b).map(u => u.owner))];
  for (const u of s.units) u.desig = null;
  s.sd = null; s.contested[b] = null;
  if (owners.length === 1 && s.bfs[b].ctrl !== owners[0]) establish(s, owners[0], b);
  else if (!owners.length) s.bfs[b].ctrl = null;
}
function endShowdown(s) {
  const sd = s.sd;
  if (sd.combat) { startDamage(s); return; }
  const b = sd.bf, owners = [...new Set(unitsAt(s, b).map(u => u.owner))];
  s.sd = null; s.contested[b] = null;
  if (owners.length === 1 && s.bfs[b].ctrl !== owners[0]) establish(s, owners[0], b);
}

// ------------------------------------------------------------------ options
function timingOk(s, pid, t) {
  if (s.chain.length) return t === 'reaction';
  if (s.sd) return t === 'reaction' || t === 'action';
  return s.turn === pid;   // ton tour, état neutre : tout est permis
}
function cardChoices(s, pid, name, src) {
  const c = C[name];
  if (c.type !== 'unit') return c.choices ? c.choices(s, pid, src) : [{}];
  const locs = [];
  if (c.timing === 'main') { locs.push('base'); for (let b = 0; b < 2; b++) if (s.bfs[b].ctrl === pid) locs.push(b); }
  if (c.ambush) { if (!locs.includes('base')) locs.push('base'); for (let b = 0; b < 2; b++) if (unitsAt(s, b).length && !locs.includes(b)) locs.push(b); }
  const out = [];
  for (const loc of locs) { out.push({ loc }); if (c.accelerate) out.push({ loc, acc: 1 }); }
  return out;
}
function playable(s, pid, name, ch, src, keep) {
  const cost = costOf(s, pid, name, ch, src);
  return !!planPay(s, pid, cost.e, cost.pReq, keep);
}
function options(s, pid) {
  if (s.assign) {
    const { atk, dfn } = combatSides(s);
    const targets = s.assign.A == null ? dfn : atk;
    return assignOrders(s, targets).map(order => ({ t: 'assign', order }));
  }
  const out = [];
  const keepBase = keepValue(s, pid);
  const seen = new Set();
  s.hand[pid].forEach((name, hi) => {
    if (seen.has(name)) return; seen.add(name);
    const c = C[name];
    const t = c.type === 'unit' && c.timing === 'main' ? 'mainonly' : c.timing;
    if (t === 'mainonly' ? !(s.turn === pid && !s.sd && !s.chain.length) : (t === 'main' ? !(s.turn === pid && !s.sd && !s.chain.length) : !timingOk(s, pid, t))) return;
    for (const ch of cardChoices(s, pid, name, null)) if (playable(s, pid, name, ch, 'hand', keepValue(s, pid, hi))) out.push({ t: 'play', name, hi, ch });
  });
  s.hidden.forEach((h, k) => {
    if (h.owner !== pid || h.turn >= s.turnNo || !timingOk(s, pid, 'reaction')) return;
    for (const ch of cardChoices(s, pid, h.name, h)) if (playable(s, pid, h.name, ch, 'hidden', keepBase)) out.push({ t: 'play', name: h.name, hid: k, ch });
  });
  const fseen = new Set();
  s.trash[pid].forEach((name, ti) => {
    const c = C[name];
    if (!c.flow || fseen.has(name)) return; fseen.add(name);
    if (!(c.timing === 'main' ? s.turn === pid && !s.sd && !s.chain.length : timingOk(s, pid, c.timing))) return;
    for (const ch of cardChoices(s, pid, name, null)) if (playable(s, pid, name, ch, 'flow', keepBase)) out.push({ t: 'play', name, ti, flow: 1, ch });
  });
  if (s.legend[pid] === 'akali') {
    const L = s.legendState[pid];
    if (!L.emp && s.turn === pid && !s.sd && !s.chain.length && playable(s, pid, 'Akali: Empower', {}, 'ability', keepBase)) out.push({ t: 'act', name: 'Akali: Empower', ch: {} });
    if (!L.exh && s.turn === pid && s.sd && timingOk(s, pid, 'action')) for (const ch of C['Akali: Repli'].choices(s, pid)) out.push({ t: 'act', name: 'Akali: Repli', ch });
  }
  if (s.turn === pid && !s.sd && !s.chain.length) {
    // Hidden : payer 1 rune, poser face cachée sur un champ de bataille que tu contrôles (sans carte cachée)
    const hseen = new Set();
    s.hand[pid].forEach((name, hi) => {
      if (!C[name].hidden || hseen.has(name)) return; hseen.add(name);
      for (let b = 0; b < 2; b++) if (s.bfs[b].ctrl === pid && !s.hidden.some(h => h.bf === b) && planPay(s, pid, 0, [{ doms: ANY }], keepValue(s, pid, hi))) out.push({ t: 'hide', name, hi, bf: b });
    });
    const groups = {};
    for (const u of s.units.filter(u => u.owner === pid && u.ready)) (groups[u.loc] = groups[u.loc] || []).push(u);
    for (const loc in groups) {
      const g = groups[loc];
      const from = loc === 'base' ? 'base' : +loc;
      for (let m = 1; m < (1 << g.length); m++) {
        const ids = g.filter((u, i) => m & (1 << i)).map(u => u.id);
        for (const to of (from === 'base' ? [0, 1] : ['base'])) out.push({ t: 'move', ids, to });
      }
    }
    out.push({ t: 'end' });
  } else out.push({ t: 'pass' });
  return out;
}

function apply(s, pid, o) {
  if (o.t === 'assign') {
    if (s.assign.A == null) s.assign.A = o.order; else s.assign.D = o.order;
    if (s.assign.D != null) applyDamage(s);
    return;
  }
  if (o.t === 'pass') {
    if (s.chain.length) { s.passes++; s.prio = other(s.prio); }
    else if (s.sd) { s.sd.passes++; s.sd.focus = other(s.sd.focus); }
    log(s, `${pid === ME ? 'Tu passes' : 'Yi passe'}.`);
    return;
  }
  if (o.t === 'end') { endTurn(s); return; }
  if (o.t === 'move') { moveUnits(s, o.ids.map(id => byId(s, id)), o.to, true); return; }
  if (o.t === 'hide') {
    pay(s, pid, planPay(s, pid, 0, [{ doms: ANY }], keepValue(s, pid, o.hi)));
    s.hand[pid].splice(o.hi, 1);
    s.hidden.push({ owner: pid, name: o.name, bf: o.bf, turn: s.turnNo });
    log(s, `${pid === ME ? 'Tu caches' : 'Yi cache'} une carte face cachée sur ${s.bfs[o.bf].name}${pid === ME ? ` (${C[o.name].fr}, jouable à partir du tour suivant)` : ''}.`);
    return;
  }
  const src = o.t === 'act' ? 'ability' : o.hid != null ? 'hidden' : o.flow ? 'flow' : 'hand';
  const cost = costOf(s, pid, o.name, o.ch, src);
  pay(s, pid, planPay(s, pid, cost.e, cost.pReq, keepValue(s, pid, src === 'hand' ? o.hi : -1)));
  if (src === 'hidden') s.hidden.splice(o.hid, 1);
  else if (src === 'hand') s.hand[pid].splice(o.hi, 1);
  else if (src === 'flow') s.trash[pid].splice(o.ti, 1);
  const c = C[o.name];
  if (src !== 'ability') {
    if (s.heronDisc[pid]) { s.heronDisc[pid] = false; log(s, 'Réduction de l\'Astral Heron utilisée.'); }
    s.played[pid]++;
    if (s.played[pid] === 1 && s.units.some(u => u.owner === pid && C[u.name].heron && atBf(u))) { s.heronDisc[pid] = true; log(s, 'Astral Heron : ta prochaine carte coûte 2 énergie et 2 runes de moins.'); }
  } else if (o.name === 'Akali: Repli') s.legendState[pid].exh = true;
  const who = pid === ME ? 'Tu joues' : 'Yi joue';
  const inSd = !!s.sd, chainWasEmpty = !s.chain.length;
  if (inSd && chainWasEmpty) s.sd.passes = 0;
  if (c.type === 'unit') {
    const u = { id: 'u' + s.nextId++, name: o.name, owner: pid, loc: o.ch.loc, ready: !!o.ch.acc, dmg: 0, mod: 0, emp: false, shield: 0, tank: false, stun: false, desig: null };
    s.units.push(u);
    log(s, `${who} ${c.fr} → ${locName(s, o.ch.loc)}${o.ch.acc ? ' (Accelerate : prête)' : ''}.`);
    if (o.ch.loc !== 'base' && s.bfs[o.ch.loc].ctrl !== pid && s.contested[o.ch.loc] == null && !(s.sd && s.sd.bf === o.ch.loc)) s.contested[o.ch.loc] = pid;
    if (s.sd && s.sd.combat && o.ch.loc === s.sd.bf) { u.desig = u.owner === s.sd.att ? 'att' : 'def'; s.sd.members.push(u.id); }
    if (s.chain.length) { s.prio = pid; s.passes = 0; }
    else if (inSd) { s.sd.focus = other(s.sd.focus); s.sd.passes = 0; }
    return;
  }
  log(s, `${who} ${c.fr}${src === 'hidden' ? ' (face cachée)' : src === 'flow' ? ' (Flow, depuis la défausse)' : ''} : ${c.label(s, o.ch)}.`);
  s.chain.push({ uid: s.uid++, name: o.name, owner: pid, ch: o.ch, fromHidden: src === 'hidden', flowed: src === 'flow', extraP: c.extraP ? c.extraP(o.ch) : 0 });
  if (chainWasEmpty) s.chainOrigin = inSd ? 'sd' : 'main';
  s.prio = pid; s.passes = 0;   // le joueur qui joue garde la priorité
}

function endTurn(s) {
  log(s, s.turn === ME ? 'Fin de ton tour.' : 'Fin du tour de Yi.');
  for (const u of s.units) { u.mod = 0; u.stun = false; u.shield = 0; u.tank = false; u.dmg = 0; }
  if (s.turn === ME && s.opTurn) beginOpTurn(s);
  else s.stage = 'done';
}
function beginOpTurn(s) {
  s.turn = OP; s.turnNo++;
  s.played = { me: 0, op: 0 }; s.heronDisc = { me: false, op: false };
  log(s, '— Tour de Yi —');
  s.runes.op = s.opTurn.runes.map(d => ({ d, r: true }));
  for (const u of s.units) if (u.owner === OP) u.ready = true;
  if (s.opTurn.hand) s.hand.op = s.opTurn.hand.slice();
  for (const b of s.bfs) b.scored = [];
  s.forced = (s.opTurn.forced || []).slice();
  s.bfs.forEach(b => { if (b.ctrl === OP) { s.pts.op++; b.scored.push(OP); log(s, `Yi tient ${b.name} : +1 point.`); } });
}

function advance(s) {
  for (let guard = 0; guard < 500; guard++) {
    if (s.stage === 'done') return null;
    if (s.assign) {
      if (s.assign.A == null) return { who: s.sd.att, opts: options(s, s.sd.att), kind: 'assign' };
      return { who: other(s.sd.att), opts: options(s, other(s.sd.att)), kind: 'assign' };
    }
    if (s.chain.length) {
      if (s.passes >= 2) {
        const it = s.chain.pop();
        s.passes = 0;
        log(s, `${C[it.name].fr} se résout.`);
        C[it.name].resolve(s, it);
        toTrash(s, it);
        cleanupDeaths(s);
        if (s.chain.length) { s.prio = s.chain[s.chain.length - 1].owner; s.passes = 0; continue; }
        if (s.sd) { if (s.chainOrigin === 'sd') s.sd.focus = other(s.sd.focus); s.sd.passes = 0; }
        s.chainOrigin = null; s.prio = null;
        cleanup(s);
        continue;
      }
      return { who: s.prio, opts: options(s, s.prio), kind: 'prio' };
    }
    if (s.sd) {
      if (s.sd.passes >= 2) { endShowdown(s); if (!s.assign) cleanup(s); continue; }
      return { who: s.sd.focus, opts: options(s, s.sd.focus), kind: 'focus' };
    }
    cleanup(s);
    if (s.sd) continue;
    if (s.turn === OP && s.forced && s.forced.length) {
      const f = s.forced.shift();
      const ids = f.names.map(n => (s.units.find(u => u.owner === OP && u.name === n && u.ready) || {}).id).filter(Boolean);
      if (ids.length) { apply(s, OP, { t: 'move', ids, to: f.to }); continue; }
    }
    return { who: s.turn, opts: options(s, s.turn), kind: 'main' };
  }
  throw new Error('boucle');
}

// ------------------------------------------------------------------ IA / solveur (minimax exact avec mémo)
function solve(s, goal, memo, budget) {
  const k = key(s);
  if (memo.has(k)) return memo.get(k);
  if (budget.n-- <= 0) throw new Error('budget');
  const d = advance(s);
  let v;
  if (!d) v = goal(s);
  else {
    let best = null;
    for (const o of d.opts) {
      const c = clone(s); c.log = null;
      apply(c, d.who, o);
      const x = solve(c, goal, memo, budget);
      if (best === null || (d.who === ME ? x > best : x < best)) best = x;
    }
    v = best;
  }
  memo.set(k, v);
  return v;
}
// Yi ne voit ni ta main, ni ta défausse, ni tes cartes cachées : il joue comme si tu n'avais rien.
function aiChoose(s, goal) {
  const c0 = clone(s); c0.log = null;
  c0.hand.me = []; c0.trash.me = []; c0.hidden = c0.hidden.filter(h => h.owner !== ME);
  const d = advance(c0);
  const memo = new Map(), budget = { n: 600000 };
  let best = null, bo = null;
  d.opts.forEach(o => {
    const c = clone(c0); c.log = null;
    apply(c, d.who, o);
    const v = solve(c, goal, memo, budget);
    const pref = (o.t === 'pass' || o.t === 'end') ? 0 : 1;
    if (best === null || v < best.v || (v === best.v && pref < best.pref)) { best = { v, pref }; bo = o; }
  });
  return bo;
}

const API = { C, ME, OP, newState, clone, key, advance, apply, options, solve, aiChoose, might, byId, nm, locName, unitsAt, costOf, planPay, keepValue };
if (typeof module !== 'undefined') module.exports = API; else root.RB = API;
})(this);
