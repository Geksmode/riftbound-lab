// Puzzles sur plateau : Akali (Heron de Gorica) contre Master Yi, Wuju Bladesman.
// goal(s) renvoie la valeur pour toi : >= 1000 = objectif atteint ; le reste départage les lignes de Yi.
(function (root) {
'use strict';
const SIGIL = 'Sigil of the Storm', GROVE = 'Grove of the God-Willow';
function material(s) { return s.units.reduce((t, u) => t + (u.owner === 'me' ? 1 : -1) * RBC()[u.name].might, 0); }
function RBC() { return (typeof module !== 'undefined' ? require('./engine.js') : root.RB).C; }
const alive = (s, name) => s.units.find(u => u.owner === 'me' && u.name === name);
const lostMine = s => s.deadIds.some(id => /^u/.test(id) && s.mineIds && s.mineIds.includes(id));
const P = [
{
  id: 'flip', title: 'Tuer avant le combat', tag: 'Prendre un champ de bataille',
  brief: "Prends Grove of the God-Willow ce tour-ci. Tu sais que Yi a Punch First en main.",
  bfs: [{ name: SIGIL, ctrl: 'me' }, { name: GROVE, ctrl: 'op' }],
  me: { units: [{ name: 'Mournful Witness', loc: 0, ready: false }, { name: "Kai'Sa, Survivor", loc: 'base' }], hand: ['Shuriken Flip'], runes: ['Fury', 'Calm'] },
  op: { units: [{ name: 'Pit Rookie', loc: 1 }], hand: ['Punch First'], runes: ['Body', 'Body', 'Calm'] },
  goal: s => (s.bfs[1].ctrl === 'me' ? 1000 : 0) + material(s),
  goalText: 'Tu contrôles Grove of the God-Willow à la fin de ton tour.',
  solution: "Shuriken Flip sur le Pit Rookie (2 dégâts), et le déplacement du sort envoie Kai'Sa sur Grove.",
  lesson: "Hors combat, le Rookie n'a que 2 de puissance : la légende ne compte qu'en défense. Et en réponse à ton sort, Yi ne peut jouer que des Réactions : Punch First est une Action, elle ne sort qu'à son tour ou pendant un combat. Le Rookie meurt, Grove est vide, Kai'Sa conquiert et pioche. Si tu attaques directement, le Rookie vaut 4 en défense et Punch First le monte à 9.",
},
{
  id: 'count', title: 'Compter son défenseur', tag: 'Compter',
  brief: "Prends Grove of the God-Willow ce tour-ci. Yi a En Garde en main et une rune Calm ouverte.",
  bfs: [{ name: SIGIL, ctrl: null }, { name: GROVE, ctrl: 'op' }],
  me: { units: [{ name: "Kai'Sa, Survivor", loc: 'base' }, { name: 'Mournful Witness', loc: 'base' }], hand: ['Discipline'], runes: ['Calm', 'Calm'] },
  op: { units: [{ name: 'First Mate', loc: 1 }], hand: ['En Garde'], runes: ['Calm'] },
  goal: s => (s.bfs[1].ctrl === 'me' ? 1000 : 0) + material(s),
  goalText: 'Tu contrôles Grove of the God-Willow à la fin de ton tour.',
  solution: "Attaquer avec Kai'Sa et Witness ensemble, et garder Discipline pour répondre à En Garde (ou la jouer avant : elle reste au-dessus).",
  lesson: "First Mate défend seul : 3 + 2 = 5. Avec En Garde (+1, +1 s'il est seul) il monte à 7. Kai'Sa + Witness font 6 : il faut Discipline pour passer à 8. Avant chaque attaque, compte : puissance imprimée + 2 + le meilleur tour qu'il peut payer.",
},
{
  id: 'heron', title: 'Installer le Héron', tag: 'Plan Héron',
  brief: "Pose le Héron de façon qu'il soit encore sur un champ de bataille à toi à la fin du tour de Yi. À son tour, Yi aura 2 runes (Calm et Body) et Charm en main.",
  bfs: [{ name: SIGIL, ctrl: 'me' }, { name: GROVE, ctrl: 'op' }],
  me: { units: [{ name: 'Mournful Witness', loc: 0, ready: false }], hand: ['Astral Heron', 'Mournful Witness', 'Defy'], runes: ['Calm', 'Calm', 'Calm', 'Calm', 'Fury', 'Fury', 'Fury', 'Fury'] },
  op: { units: [{ name: 'Master Yi, Tempered', loc: 1 }], hand: ['Charm'], runes: [] },
  opTurn: { runes: ['Calm', 'Body'] },
  goal: s => { const h = alive(s, 'Astral Heron'); return (h && h.loc !== 'base' && s.bfs[h.loc].ctrl === 'me' ? 1000 : 0) + material(s); },
  goalText: "À la fin du tour de Yi, l'Astral Heron est vivant, sur un champ de bataille que tu contrôles.",
  solution: "Jouer l'Astral Heron directement sur Sigil of the Storm (tu le contrôles), puis finir le tour avec Defy et une rune Calm ouverte. À son tour, Defy contre Charm.",
  lesson: "Règle 355.2 : une unité se joue dans ta base ou sur un champ de bataille que tu contrôles. Le Héron ne marche pas vers un champ de bataille, il s'y pose. Charm ne se joue qu'au tour de Yi, et il passe sous Defy (1 énergie, 1 rune) : garde Defy ouvert à la fin de ton tour.",
},
{
  id: 'block', title: 'Il attaque ton Héron', tag: 'Défendre',
  brief: "C'est le tour de Yi. Il attaque Sigil of the Storm avec Master Yi. Garde ton Astral Heron en vie. Tu as Block caché sur Sigil.",
  bfs: [{ name: SIGIL, ctrl: 'me' }, { name: GROVE, ctrl: null }],
  hidden: [{ owner: 'me', name: 'Block', bf: 0 }],
  me: { units: [{ name: 'Astral Heron', loc: 0 }], hand: ['Discipline', 'Defy'], runes: ['Calm', 'Calm'] },
  op: { units: [{ name: 'Master Yi, Tempered', loc: 'base' }], hand: [], runes: [] },
  startOp: true,
  opTurn: { runes: ['Body', 'Body', 'Calm'], hand: ['Punch First'], forced: [{ names: ['Master Yi, Tempered'], to: 0 }] },
  goal: s => (alive(s, 'Astral Heron') ? 1000 : 0) + material(s),
  goalText: "L'Astral Heron est encore en vie à la fin du tour de Yi.",
  solution: "Quand Yi joue Punch First (Yi à 9), révéler Block sur le Héron : Shield 3 le monte à 10. Il survit et tue Yi.",
  lesson: "Punch First coûte 2 runes : Defy ne peut pas le contrer. Discipline ne suffit pas (9 contre 9, le Héron meurt). Block se joue depuis la face cachée sans énergie. Et la légende de Yi ne l'aide pas quand c'est lui qui attaque.",
},
{
  id: 'runner', title: 'Ruin Runner', tag: 'Connaître sa liste',
  brief: "Prends Grove of the God-Willow sans perdre ton Astral Heron.",
  bfs: [{ name: SIGIL, ctrl: null }, { name: GROVE, ctrl: 'op' }],
  me: { units: [{ name: 'Astral Heron', loc: 'base' }, { name: 'Mournful Witness', loc: 'base' }], hand: ['Falling Star', 'Back Off', 'Block'], runes: ['Fury', 'Fury', 'Calm', 'Calm'] },
  op: { units: [{ name: 'Ruin Runner', loc: 1 }], hand: [], runes: [] },
  goal: s => (s.bfs[1].ctrl === 'me' && alive(s, 'Astral Heron') ? 1000 : 0) + material(s),
  goalText: "Tu contrôles Grove of the God-Willow et ton Astral Heron est vivant à la fin de ton tour.",
  solution: "Block sur Mournful Witness (Tank), puis attaquer avec Witness et le Héron. Ruin Runner doit mettre ses dégâts sur Witness d'abord : 2 sur Witness, 5 sur le Héron qui survit.",
  lesson: "Ruin Runner ne peut pas être choisi par tes sorts : Falling Star et Back Off ne le touchent pas. Seul le combat le tue, et il vaut 7 en défense seul. Tank oblige l'adversaire à mettre ses dégâts sur cette unité en premier : c'est comme ça qu'on protège une grosse unité dans un combat.",
},
{
  id: 'decree', title: 'Manche 2 : Decree of Focus', tag: 'Sideboard',
  brief: "Manche 2, Yi a sidé Decree of Focus. Prends Grove of the God-Willow sans perdre d'unité. Yi a une rune Calm ouverte, Decree of Focus et En Garde en main.",
  bfs: [{ name: SIGIL, ctrl: null }, { name: GROVE, ctrl: 'op' }],
  me: { units: [{ name: 'Astral Heron', loc: 'base' }, { name: 'Mournful Witness', loc: 'base', emp: true }, { name: "Kai'Sa, Survivor", loc: 'base' }], hand: [], runes: [] },
  op: { units: [{ name: 'Pit Rookie', loc: 1 }], hand: ['Decree of Focus', 'En Garde'], runes: ['Calm'] },
  goal: s => (s.bfs[1].ctrl === "me" && !s.deadIds.some(id => (s.mine0||[]).includes(id)) ? 1000 : 0) + material(s),
  goalText: "Tu contrôles Grove of the God-Willow et tu n'as perdu aucune unité à la fin de ton tour.",
  solution: "Attaquer avec l'Astral Heron seul. Autre ligne : attaquer avec le Héron et Witness, puis utiliser le repli de la légende d'Akali pour ramener Witness en base avant les dégâts.",
  lesson: "Decree of Focus donne +4 à son unité en combat contre une unité Fury (Kai'Sa). Avec le Héron seul (Calm), son meilleur tour est En Garde : Rookie 2 + 2 + 2 = 6, le Héron (7) survit et le tue. Dès que Kai'Sa vient, le Rookie passe à 8. Avec Witness en plus, ses 6 dégâts tuent Witness, sauf si tu la sors du combat avec le repli de ta légende (gratuit, une fois par tour, pendant un combat à ton tour).",
},
];
if (typeof module !== 'undefined') module.exports = P; else root.PUZZLES = P;
})(this);
