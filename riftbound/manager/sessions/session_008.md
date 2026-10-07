# Session 008 — 2026-10-05  (moteur 44d6a1b695, plans.py 8b2147fcef, IA tempo + LeBlanc « réel »)

Même version que s004-s007. Bloc neuf 203200-203599, 400 parties par ligne, liste stock. **Questions fixées à
l'avance** (pistes de s007) : plan Gorica − sans plan, et plan Gorica − Gorica agressif, face à LeBlanc « Hook
tempo Windswept ». Résultats bruts : `results/session_008.json`.

## Résultats
| Config | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la référence |
|---|---|---|---|
| **Référence** : plan Gorica | **47,2 % ± 2,5** | 49,0 / 45,5 | — |
| sans plan | 38,8 % ± 2,4 | 43,9 / 33,7 | plan − sans plan **+8,5 ± 3,3** |
| plan Gorica agressif | 41,0 % ± 2,5 | 41,4 / 40,6 | Gorica − agressif **+6,2 ± 3,1** |
| plan Gorica, LeBlanc Hook tempo (battlefield libre) | 55,5 % ± 2,5 | 65,7 / 45,5 | **+8,2 ± 2,4** |

Cumul s007 + s008 (blocs indépendants, même version) : plan − sans plan **+7,7 ± 2,3** ; Gorica − agressif
**+6,1 ± 2,1**. Windswept, cumul s004 + s007 + s008 (1 200 parties) : **+4,3 ± 1,3**.

## Ce que ça change
- **Acquis : le plan Gorica vaut ~8 points contre aucun plan.** Confirmé sur bloc neuf avec la question fixée à
  l'avance (+8,5 ± 3,3, 2,6 ET) ; cumul +7,7 ± 2,3, soit 3 à 12 points.
- **Acquis : jouer agressif coûte ~6 points** face à Windswept. Confirmé (+6,2 ± 3,1, 2,0 ET) ; cumul +6,1 ± 2,1,
  soit 2 à 10 points. Cohérent avec le fil Replays (−8 ± 3, autre IA).
- **Acquis : Windswept coûte ~4 points à Akali avec l'IA actuelle** (+4,3 ± 1,3 sur trois blocs, 3,3 ET ;
  intervalle 1,7 à 6,9). Ce bloc donne +8,2, les deux précédents +2,2 et +2,8 : l'écart entre blocs reste dans le
  hasard attendu.
- Ordre de grandeur pour l'utilisateur : **bien jouer son plan (~8 pts) et le battlefield que présente LeBlanc
  (~4 pts) comptent plus que le side (moins de 4 pts, acquis s006).**

## Autocritique
- Le seuil « 2 ET » est juste atteint pour l'agressif sur ce bloc (2,0) ; c'est le cumul (2,9 ET) et
  l'accord avec une mesure indépendante du fil Replays qui le font passer en acquis.
- « Sans plan » veut dire l'IA de recherche seule : ce n'est pas un joueur humain sans plan. Le chiffre mesure
  ce que le plan apporte à cette IA ; pour un humain, il dit surtout quelles décisions comptent.
- Je ne sais pas encore **d'où** vient le gain du plan : du battlefield présenté (sans plan, Akali présente
  Forgotten Monument au lieu de Sigil) ou des décisions en partie. C'est la question suivante.

## Prochaine session (009)
Décomposer le gain du plan : sans plan mais en présentant Sigil of the Storm (forcé), contre plan Gorica et
contre sans plan, face à Hook tempo Windswept. Question fixée : (sans plan + Sigil) − (sans plan).
