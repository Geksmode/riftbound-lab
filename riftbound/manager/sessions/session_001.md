# Session 001 — 2026-10-03

Moteur fidèle, plans de jeu de `engine/plans.py` (version fd91de6a59, Gorica pour Akali, Deathknell pour
LeBlanc). 400 parties par ligne, toutes sur le bloc de graines neuf 200000-200399 (comparaisons appariées).
Résultats bruts : `results/session_001.json`.

## Questions posées
1. Le gain des plans de jeu tient-il sur des graines neuves ?
2. G2 bat-il encore Gorica stock quand les deux joueurs suivent un plan ?
3. G2 + plan tient-il contre la LeBlanc de LA (12e) ?
4. Windswept Hillock est-il le pire battlefield adverse pour Akali ?

## Résultats
| Config (Akali plan Gorica, LeBlanc plan Deathknell) | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la référence |
|---|---|---|---|
| **Référence** : G2 vs LeBlanc IQ5 | **59,5 % ± 2,5** | 69,5 / 50,7 | — |
| Gorica stock vs LeBlanc IQ5 | 54,5 % ± 2,5 | 61,5 / 48,4 | −5,0 ± 3,3 |
| G2 vs LeBlanc LA (12e) | 55,8 % ± 2,5 | 60,4 / 51,6 | −3,8 ± 3,4 |
| G2, LeBlanc présente toujours Windswept Hillock | 52,5 % ± 2,5 | 54,5 / 50,7 | −7,0 ± 2,4 |
| G2, LeBlanc présente toujours Star Spring | 60,5 % ± 2,4 | 69,5 / 52,6 | +1,0 ± 2,3 |

Windswept contre Star Spring, apparié : **−8,0 ± 3,3 pts** pour Akali.

## Ce que ça change
- **Plans de jeu** : 59,5 % ± 2,5 sur graines neuves, contre 57,8 % dans le fil Replays (graines 70000+).
  Le niveau tient. Le gain par rapport à « sans plan » n'a pas été rejoué en apparié ici : il reste une piste
  (environ +8 pts en comparant à 51,2 % ± 2,0 mesuré sans plans sur d'autres graines).
- **G2 contre stock avec plans** : +5,0 ± 3,3, soit 1,5 écart-type. Seul, ce lot ne tranche pas. Il va dans le
  même sens que le +6,6 ± 2,8 mesuré sans plans. G2 reste le meilleur choix connu contre LeBlanc, mais **on ne
  sait pas** si les plans réduisent son avantage.
- **LeBlanc LA** : −3,8 ± 3,4 contre IQ5, on ne sait pas s'il y a une différence. G2 + plan fait 55,8 % contre
  elle.
- **Windswept Hillock passe en acquis, avec un écart plus petit** : −8,0 ± 3,3 contre Star Spring sur ce lot,
  après −18 sur un lot indépendant (liste Wuhan, sans plans). Même sens deux fois ; ampleur plutôt 5 à 11 pts.
  Tout l'effet vient des parties où **Akali commence** (54,5 % contre 69,5 %) ; quand LeBlanc commence,
  le battlefield ne change presque rien (50,7 contre 52,6).

## Autocritique
- **La référence est flattée par le plan LeBlanc.** Le plan Deathknell ne présente Windswept que quand LeBlanc
  commence, c'est-à-dire là où ça ne change rien. Un LeBlanc qui présente Windswept à chaque partie ramène
  Akali à 52,5 %. Le 59,5 % mesure donc en partie une erreur du plan adverse, pas la force d'Akali.
  **Chiffre réaliste pour toi : ~52 %, pas ~60 %.**
- Le niveau absolu reste au-dessus du ~35 % terrain non vérifié. Les plans l'ont encore monté.
  Le moteur est sans doute optimiste pour Akali ; les écarts entre options sont plus fiables que les niveaux.
- Les plans ont été écrits par Claude à partir de guides : un gain « des plans » peut venir de la façon dont
  je les ai codés autant que de la stratégie des joueurs.
- Incident : `plans.py` a été modifié par un autre fil 2 minutes après le début de la session. La référence a
  été rejouée sur les mêmes graines avec la version finale : résultat identique (59,5 %), donc les écarts
  appariés sont valides. Le runner fige désormais `plans.py` au début de chaque session.

## Prochaine session (002)
Partir du pire cas réaliste : LeBlanc présente Windswept à chaque partie et suit le plan « Hook tempo »
(meilleur plan LeBlanc du fil Replays). Questions :
1. G2 contre stock dans ce pire cas.
2. Plan Gorica contre plan Gorica agressif dans ce pire cas.
3. Quel battlefield Akali présente face à Windswept : Sigil of the Storm (conseil de Gorica) contre Void Gate.
