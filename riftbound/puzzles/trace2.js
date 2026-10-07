// déroule une ligne en full-info : moi = meilleur coup (minimax), Yi = minimax full-info ; premier coup imposé
const RB = require('./engine.js'), P = require('./puzzles.js');
const p = P.find(x => x.id === process.argv[2]);
const s = RB.newState(p); s.mine0 = s.units.filter(u => u.owner === 'me').map(u => u.id);
const first = process.argv[3];
let firstDone = false;
const memo = new Map();
for (let i = 0; i < 80; i++) {
  const d = RB.advance(s); if (!d) break;
  const vals = d.opts.map(o => { const c = RB.clone(s); c.log = null; RB.apply(c, d.who, o); return RB.solve(c, p.goal, memo, { n: 1e7 }); });
  let j = d.who === 'me' ? vals.indexOf(Math.max(...vals)) : vals.indexOf(Math.min(...vals));
  if (!firstDone && d.who === 'me' && first) { j = d.opts.findIndex(o => JSON.stringify(o).includes(first)); firstDone = true; }
  RB.apply(s, d.who, d.opts[j]);
}
console.log(s.log.join('\n')); console.log('GOAL', p.goal(s));
