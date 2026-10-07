# Session 017 — 2026-10-07  (moteur 014259caa8, plans.py 8b2147fcef) — nouvelle version, 2e bloc

**Version.** Même moteur que s016 (seule différence avec le moteur vivant : le chemin ROOT réécrit par le runner).
Cumuls faits **avec s016 seulement**, jamais avec s004-s015.

## Question posée
Deuxième bloc de la nouvelle version : agressif, Void Gate face à Windswept, Windswept présenté ou non, commencer.

## Résultats (graines 206800-207199, liste stock, plan Gorica)
| Config | n | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la REF | Cumul s016-s017 | Ancienne version |
|---|---|---|---|---|---|---|
| **REF** plan Gorica, Sigil, contre Hook tempo Windswept | 400 | 49,5 % ± 2,5 | 50,8 / 48,3 | — | 48,4 % ± 1,8 | ~48 % |
| plan agressif | 400 | 40,8 % ± 2,5 | 46,7 / 35,0 | **−8,8 ± 3,0** | **−7,3 ± 2,2** | −5,1 ± 1,5 |
| Void Gate forcé | 400 | 40,0 % ± 2,5 | 43,7 / 36,5 | **−9,5 ± 3,1** | **−9,3 ± 2,2** | −7,9 ± 2,3 |
| contre Hook tempo (Windswept seulement quand LeBlanc commence) | 400 | 52,8 % ± 2,5 | 57,4 / 48,3 | +3,2 ± 2,2 | +2,9 ± 1,6 | +4,6 ± 1,0 |
| Commencer (REF, non apparié) | | | | +2,5 ± 5,0 | +4,0 ± 3,5 | +9,0 ± 2,2 |

Cumul = moyenne pondérée par l'inverse des variances des deux blocs indépendants (graines 206400+ et 206800+).

## Ce que ça change
- **Agressif et Void Gate sont acquis dans la nouvelle version** : deux blocs, même sens, cumul au-delà de 3 écarts-types
  (−7,3 ± 2,2 et −9,3 ± 2,2). Le plan contre aucun plan (+12,5 ± 3,4) n'a qu'un bloc de la nouvelle version.
- **Windswept** : +2,9 ± 1,6, 1,8 écart-type. Même sens que l'ancien +4,6 mais on ne sait pas encore pour la nouvelle
  version.
- **Commencer** : +4,0 ± 3,5, on ne sait pas. Les deux blocs donnent moins que l'ancien +9 ; avec les nouvelles règles
  (énergie flottante pour les deux joueurs) l'avantage du premier joueur pourrait avoir baissé, **c'est une hypothèse,
  pas un résultat** : l'intervalle contient encore 9.
- Niveau de référence 48,4 % ± 1,8, identique à l'ancienne version.

## Autocritique
- L'agressif ressort plus fort que dans l'ancienne version (−7,3 contre −5,1). Les intervalles se recouvrent largement :
  je n'en conclus pas que les nouvelles règles punissent plus l'agressif.
- « Commencer » reste lu dans des lignes non conçues pour ça (200 parties de chaque côté par bloc). Pour trancher il
  faudra cumuler des blocs ; je le garde comme question, pas comme découverte.
- Parties ajoutées au viewer : choisies parmi 57 donnes où la REF gagne et les deux variantes perdent, donc illustratives,
  pas représentatives.

## Replays
s017-1 à s017-3 : même donne (LeBlanc commence), référence gagne 8-7, agressif perd 3-8, Void Gate perd 4-8.
s017-4 : défaite type de la référence, Akali mène 5-2 puis perd les deux battlefields au tour 10 et perd 5-8.
Viewer : version 26.

## Prochaine session (18)
Troisième bloc de la nouvelle version : sans plan (2e bloc), Windswept (3e bloc), commencer (lu dans la REF), et
Forgotten Monument forcé face à Windswept (acquis ancien −5,6 ± 1,9, jamais rejoué depuis les nouvelles règles).
