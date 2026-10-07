const RB = require('./engine.js'), P = require('./puzzles.js');
const p = P.find(x => x.id === process.argv[2]);
const s = RB.newState(p); s.mine0 = s.units.filter(u => u.owner === 'me').map(u => u.id);
const script = JSON.parse(process.argv[3] || '[]'); // indices of my choices
let k = 0;
for (let i = 0; i < 60; i++) {
  const d = RB.advance(s); if (!d) break;
  if (d.who === 'op') { const o = RB.aiChoose(s, p.goal); RB.apply(s, 'op', o); continue; }
  const vals = d.opts.map(o => { const c = RB.clone(s); c.log = null; RB.apply(c, 'me', o); return RB.solve(c, p.goal, new Map(), { n: 1e6 }); });
  console.log('MY OPTIONS:', d.opts.map((o, j) => `[${j}] ${vals[j]} ${o.t} ${o.name || ''} ${JSON.stringify(o.ch || o.ids || '')} ${o.to ?? ''}`).join('\n  '));
  const j = k < script.length ? script[k++] : vals.indexOf(Math.max(...vals));
  RB.apply(s, 'me', d.opts[j]);
}
console.log(s.log.join('\n')); console.log('GOAL', p.goal(s));
