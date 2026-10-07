# Session 007 — 2026-10-04  (moteur 44d6a1b695, plans.py 8b2147fcef, IA tempo + LeBlanc « réel »)

Même version que s004-s006. Changement de sujet après l'acquis de s006 (les listes se valent) : ce que vaut le
**plan de jeu d'Akali**. Liste stock partout, LeBlanc IQ5 suit « Hook tempo Windswept » sauf la dernière ligne.
400 parties par ligne, bloc neuf 202800-203199. Résultats bruts : `results/session_007.json`.

## Questions posées
1. Plan Gorica contre aucun plan (Akali joue avec l'IA seule), en apparié.
2. Plan Gorica contre « Gorica agressif ».
3. Cumuler l'effet Windswept (LeBlanc ne le force pas).

## Résultats
| Config | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la référence |
|---|---|---|---|
| **Référence** : stock, plan Gorica | **45,5 % ± 2,5** | 53,0 / 36,6 | — |
| stock, sans plan | 38,5 % ± 2,4 | 47,5 / 27,9 | plan − sans plan **+7,0 ± 3,1** |
| stock, plan Gorica agressif | 39,5 % ± 2,4 | 44,7 / 33,3 | Gorica − agressif **+6,0 ± 3,0** |
| stock, plan Gorica, LeBlanc Hook tempo (battlefield libre) | 48,2 % ± 2,5 | 58,1 / 36,6 | +2,8 ± 2,3 |

Cumul Windswept (stock, s004 + s007) : ne pas forcer Windswept vaut **+2,5 ± 1,6** pour Akali.

## Ce que ça change
- **Le plan Gorica vaut environ 7 points** contre aucun plan (+7,0 ± 3,1, 2,3 ET), en apparié pour la première
  fois. La session 1 l'estimait à ~+8 sans appariement : même ordre de grandeur. Il passe en « Pistes fortes »
  et sera rejoué sur un bloc neuf en session 8 avant d'entrer dans les acquis.
- **Jouer agressif coûte environ 6 points** (+6,0 ± 3,0 pour le plan normal, 2,0 ET). Le fil Replays avait
  trouvé −8 ± 3 avec une autre version de l'IA : même sens, mesures indépendantes. Pour l'utilisateur :
  contre LeBlanc qui présente Windswept, le plan qui prend et tient les battlefields bat le plan qui fonce.
- **Windswept** : +2,5 ± 1,6 sur 800 parties, toujours sous 2 ET. Avec cette IA, l'effet existe peut-être mais
  il est petit (intervalle −0,7 à +5,7).

## Autocritique
- Deux questions principales, deux écarts autour de 2 ET : ce sont des comparaisons prévues, pas l'écart le plus
  grand d'un lot, mais une seule session ne suffit pas pour un acquis (règle : bloc neuf).
- Le niveau de référence (45,5 %) est plus bas que les blocs précédents de la même liste (47,8-51,3 %) : c'est
  dans la dispersion habituelle de ± 5.
- Sans plan, Akali tombe à 27,9 % quand LeBlanc commence : c'est là que le plan compte le plus (+8,7 contre
  +5,5 quand Akali commence), écart entre les deux non testé.

## Prochaine session (008)
Rejouer sur un bloc neuf, questions fixées à l'avance : plan Gorica − sans plan et plan Gorica − agressif.
Ajouter Windswept libre pour continuer le cumul.
