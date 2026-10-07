# Session 005 — 2026-10-04  (moteur 44d6a1b695, plans.py 8b2147fcef, IA tempo + LeBlanc « réel »)

Même version d'IA que s004 : les écarts appariés de s004 et s005 viennent de blocs de graines indépendants
(201600-201999 et 202000-202399) et peuvent donc se cumuler. Akali suit le plan Gorica, LeBlanc IQ5 suit
« Hook tempo Windswept ». 400 parties par ligne. Résultats bruts : `results/session_005.json`.

## Questions posées
1. Accumuler G2 − stock sur un deuxième bloc de la même version.
2. Mesurer GD (−3 Defy, +2 Ferrous Forerunner, +1 Lonely Poro) contre stock avec cette IA ; GL rejoué pour le cumul.

## Résultats
| Config | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la référence |
|---|---|---|---|
| **Référence** : Gorica stock | **48,0 % ± 2,5** | 52,0 / 43,9 | — |
| G2 | 47,2 % ± 2,5 | 54,9 / 39,3 | −0,8 ± 3,2 |
| GD (−3 Defy) | 45,8 % ± 2,5 | 46,1 / 45,4 | −2,2 ± 3,3 |
| GL (−2 Long Sword) | 53,2 % ± 2,5 | 58,8 / 47,4 | +5,2 ± 3,4 |

Autres écarts appariés : GL − GD **+7,5 ± 3,4** ; GD − G2 −1,5 ± 3,3.

Cumul s004 + s005 (même version, blocs indépendants, 800 parties appariées par écart) :
G2 − stock **−0,3 ± 2,3** ; GL − stock **+1,6 ± 2,4**.

## Ce que ça change
- **G2 contre stock** : −0,3 ± 2,3 sur 800 parties. Avec cette IA, l'intervalle à 2 écarts-types va de −5 à +4 :
  s'il existe un avantage de G2 sous plans, il est petit. Garder la liste stock par défaut.
- **GL contre stock** : +5,2 sur ce bloc, −2,0 sur le précédent ; cumul +1,6 ± 2,4, **on ne sait pas**.
- **GL contre GD** : +7,5 ± 3,4 (2,2 ET), après +8,3 ± 3,3 en s003 avec une autre IA. Deux fois le même sens et la
  même taille. Lecture possible : **si on side, mieux vaut couper les Long Sword que les Defy**. Ce n'est pas
  encore acquis (voir autocritique) : rejoué en session 6 comme question principale, fixée à l'avance.

## Autocritique
- GL − GD n'était pas la question posée de cette session (les jobs se comparaient à stock) : c'est le plus grand
  de 6 écarts possibles, donc il est flatté. Le même écart en s003 était aussi le plus grand, et c'était une
  autre IA : je ne cumule pas les deux. Il reste en « Pistes ».
- GL − stock a changé de 7 points entre deux blocs (−2,0 puis +5,2) : c'est compatible avec le hasard
  (différence 7,2 ± 4,8), et ça rappelle qu'un bloc seul ne tranche pas un écart de quelques points.
- GD et G2 perdent tous deux les Defy et font tous deux un peu moins bien que GL ; G2 garde aussi le −2 Long
  Sword. Si les Defy comptent, G2 serait pénalisé par sa moitié Defy : cohérent avec G2 ≈ stock, sans preuve.

## Prochaine session (006)
Question fixée à l'avance : **GL − GD** sur un bloc neuf, même version. Jobs : référence GL, puis GD, stock, G2
contre Hook tempo Windswept. Si le moteur change, ne rien cumuler avec s004-s005.
