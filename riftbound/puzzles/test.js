const RB = require('./engine.js'), P = require('./puzzles.js');
function init(p) { const s = RB.newState(p); s.mine0 = s.units.filter(u => u.owner === 'me').map(u => u.id); return s; }
function label(s, o) { if (o.t === 'play') return `play ${o.name} ${JSON.stringify(o.ch)}${o.hid!=null?' (hidden)':''}`; if (o.t === 'move') return `move ${o.ids.map(i=>RB.nm(s,i)).join('+')} -> ${o.to}`; return o.t; }
for (const p of P) {
  if (process.argv[2] && p.id !== process.argv[2]) continue;
  const s = init(p); s.log = null;
  const t0 = Date.now();
  const memo = new Map(), budget = { n: 2e7 };
  const v = RB.solve(RB.clone(s), p.goal, memo, budget);
  console.log(`\n== ${p.id}: value ${v} (${v >= 1000 ? 'SOLVABLE' : 'NO'}) states ${memo.size} ${Date.now() - t0}ms`);
  const d = RB.advance(s);
  for (const o of d.opts) { const c = RB.clone(s); RB.apply(c, d.who, o); const x = RB.solve(c, p.goal, memo, budget); console.log(`  ${x >= 1000 ? 'WIN ' : 'lose'} ${x}  ${label(s, o)}`); }
}
