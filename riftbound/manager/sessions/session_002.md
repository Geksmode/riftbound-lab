# Session 002 — 2026-10-03

Pire cas réaliste : LeBlanc IQ5 suit le plan « Hook tempo Windswept » (présente Windswept Hillock à chaque
partie), Akali suit le plan Gorica. `plans.py` figé (7a015a956e). 400 parties par ligne, bloc de graines neuf
200400-200799 (apparié). Résultats bruts : `results/session_002.json`.

## Questions posées
1. G2 bat-il encore Gorica stock dans ce pire cas ?
2. Quel battlefield Akali doit présenter : Sigil of the Storm, Void Gate ou Forgotten Monument ?
(Plan prudent contre agressif retiré : déjà tranché par le fil Replays.)

## Résultats
| Config | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la référence |
|---|---|---|---|
| **Référence** : G2 (le plan choisit Sigil) | **40,0 % ± 2,5** | 43,9 / 36,6 | — |
| Gorica stock | 43,0 % ± 2,5 | 50,3 / 36,6 | +3,0 ± 3,3 |
| G2, toujours Sigil of the Storm | 40,0 % ± 2,5 | identique à la référence | 0 |
| G2, toujours Void Gate | 36,0 % ± 2,4 | 38,5 / 33,8 | −4,0 ± 3,4 |
| G2, toujours Forgotten Monument | 39,8 % ± 2,5 | 42,8 / 37,1 | −0,2 ± 3,3 |
| Contrôle : référence rejouée sur un 2e bloc neuf (200800+) | 47,3 % ± 2,5 | 54,5 / 39,9 | non apparié |

## Contrôle de l'écart avec le fil Replays
Le fil Replays donnait 53,3 % pour la même config (graines 80000+). J'ai rejoué 100 de ses graines avec mon
runner : **100 résultats identiques sur 100**. J'ai aussi rejoué 20 de mes parties seules, dans l'ordre inverse :
0 différence. Le moteur est donc déterministe et les deux runners sont équivalents. Les trois blocs de 400
donnent **53,3 / 40,0 / 47,3 %**. L'écart entre blocs (écart-type 6,6 pts) est plus grand que ce que
prévoit le hasard binomial (2,5 pts), et je ne sais pas pourquoi. Moyenne des 1 200 parties indépendantes :
**46,8 %**. Vu la dispersion, l'incertitude honnête est d'environ ± 4 pts, pas ± 1,4.

## Ce que ça change
- **Le niveau contre un LeBlanc qui présente toujours Windswept est ~47 % ± 4**, pas 52-53 %.
- **G2 contre stock : l'avantage de G2 disparaît dans ce cas** (stock +3,0 ± 3,3). Trois blocs indépendants
  donnent G2 − stock = +6,6 ± 2,8 (sans plans), +5,0 ± 3,3 (Deathknell), −3,0 ± 3,3 (Hook tempo Windswept).
  Les conditions diffèrent, donc la moyenne pondérée (+3,3 ± 1,8) n'est qu'indicative. **On ne sait plus si
  G2 vaut mieux que la liste stock contre un LeBlanc qui joue bien.** G2 sort de « Acquis ».
- **Battlefield d'Akali** : Sigil 40,0, Forgotten Monument 39,8, Void Gate 36,0. Sigil contre Void Gate :
  +4,0 ± 3,4 (1,2 ET). Le conseil de Gorica (Sigil face à Windswept) va dans le bon sens, mais **on ne sait
  pas** le prouver. Garder Sigil ne coûte rien.

## Autocritique
- **J'ai annoncé des niveaux absolus sur un seul bloc de 400 parties.** La session 1 annonçait « 59,5 %
  flatté, ~52 % réaliste ». Un bloc peut s'écarter de ~6 pts de la moyenne ici : ces niveaux valent ± 5,
  pas ± 2,5. Seuls les écarts appariés dans un même bloc restent fiables à ± 3,3.
- **G2 était classé « Acquis » sur un seul contexte** (LeBlanc sans plan). Je l'ai étendu à tort au LeBlanc
  qui joue un plan. La règle « rejoué sur graines neuves » ne suffit pas : il faut aussi rejouer dans les
  conditions où on veut appliquer la conclusion.
- La dispersion anormale entre blocs n'est pas expliquée. Une piste à tester : une stratégie du plan ou de
  l'IA qui ne se déclenche que dans certaines donnes, ce qui rend les parties plus « tout ou rien ».

## Prochaine session (003)
Trancher G2 contre stock contre un LeBlanc qui joue bien, sur un nouveau bloc, avec et sans Windswept forcé,
et isoler la moitié Long Sword (GL) de la moitié Defy.
