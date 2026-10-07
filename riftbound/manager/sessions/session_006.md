# Session 006 — 2026-10-04  (moteur 44d6a1b695, plans.py 8b2147fcef, IA tempo + LeBlanc « réel »)

Même version que s004-s005 : blocs indépendants (202400-202799), cumulables. Akali suit le plan Gorica,
LeBlanc IQ5 suit « Hook tempo Windswept ». 400 parties par ligne. Résultats bruts : `results/session_006.json`.

## Questions posées
1. **Question fixée à l'avance** : GL − GD (couper les 2 Long Sword plutôt que les 3 Defy), piste de s003 et s005.
2. Cumuler G2 − stock et GL − stock.

## Résultats
| Config | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la référence |
|---|---|---|---|
| **Référence** : GL (−2 Long Sword) | **43,8 % ± 2,5** | 50,5 / 37,9 | — |
| GD (−3 Defy) | 50,5 % ± 2,5 | 55,9 / 45,8 | GL − GD **−6,8 ± 3,3** |
| Gorica stock | 47,8 % ± 2,5 | 52,2 / 43,9 | GL − stock −4,0 ± 3,3 |
| G2 | 49,8 % ± 2,5 | 59,7 / 41,1 | GL − G2 −6,0 ± 3,3 |

Cumul s004-s006 (même version, blocs indépendants) :
| Écart | Blocs | Cumul ± ET |
|---|---|---|
| G2 − stock | s4, s5, s6 (1 200 parties) | **+0,4 ± 1,9** |
| GL − stock | s4, s5, s6 (1 200) | −0,3 ± 1,9 |
| GD − stock | s5, s6 (800) | +0,2 ± 2,4 |
| GL − GD | s5, s6 (800) | +0,1 ± 2,4 |

## Ce que ça change
- **Réfuté : « couper les Long Sword vaut mieux que couper les Defy ».** La question fixée à l'avance donne
  −6,8 ± 3,3, le sens inverse de s005 (+7,5). Le +7,5 était le plus grand de 6 écarts, donc gonflé par la sélection.
- **Acquis (avec cette IA) : contre un LeBlanc qui suit un plan et présente Windswept, les quatre listes se
  valent à ± 4 points près.** G2 − stock +0,4 ± 1,9 sur 1 200 parties appariées : l'intervalle à 2 ET va de −3,4
  à +4,2. Choisir entre stock, G2, GL et GD ne change pas le matchup de façon mesurable dans ce moteur.
- Conséquence pour la suite : arrêter de tester des variantes de side de quelques cartes ; leur effet est
  plus petit que ce que 400 parties peuvent voir. Passer aux questions de jeu (plans, battlefield, Windswept).

## Autocritique
- **Trois sessions de suite, l'écart le plus grand d'une session a changé de signe à la suivante** (GL − stock,
  puis GL − GD). Fixer la question avant de jouer, comme cette fois, est ce qui a permis de le voir proprement :
  à garder pour toute piste.
- GL − GD est passé de +7,5 à −6,8 (différence 14,3 ± 4,7, 3 ET). La sélection explique une partie ; le reste
  rejoint la question ouverte de la dispersion entre blocs. Le cumul sur trois blocs reste l'estimation à retenir.
- Le niveau absolu varie aussi : 43,8 % pour GL ici contre 53,2 % en s005, même liste, même IA (± 5 par bloc).

## Prochaine session (007)
Changer de sujet : ce que vaut le **plan de jeu d'Akali** en apparié (plan Gorica contre aucun plan contre
« Gorica agressif »), liste stock contre Hook tempo Windswept, plus Hook tempo sans Windswept pour cumuler
l'effet Windswept avec cette IA.
