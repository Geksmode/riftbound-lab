# Session 016 — 2026-10-07  (moteur 014259caa8, plans.py 8b2147fcef) — NOUVELLE VERSION

**Version.** Premier bloc sur le nouveau moteur : rune prête recyclée épuisée d'abord avec énergie flottante, bug des
cartes en double corrigé (game.py, 2026-10-06 18:31), plus un ordre d'itération fixé dans actions.py et cards.py
(19:59, pour que le moteur tourne pareil en 32 et 64 bits). 91 tests passent. **Rien n'est cumulé avec s004-s015.**

**Incident.** Le conteneur a redémarré vers 00:20 après les deux premiers jobs (déjà écrits dans session_016.json) ; le
journal de console a été perdu. J'ai ajouté au runner une reprise (`RB_RESUME=16`) qui réutilise le moteur figé, les
mêmes graines (206400+) et ne joue que les jobs manquants : les comparaisons restent appariées.

## Question posée
Remesurer les acquis principaux sur bloc neuf : plan contre aucun plan, plan agressif, Void Gate au lieu de Sigil face
à Windswept, Windswept présenté ou non, et commencer.

## Résultats (graines 206400-206799, liste stock, plan Gorica)
| Config | n | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la REF | Ancienne version |
|---|---|---|---|---|---|
| **REF** plan Gorica, Sigil, contre Hook tempo Windswept | 400 | 47,3 % ± 2,5 | 50,0 / 44,5 | — | ~48 % |
| sans plan | 400 | 34,8 % ± 2,4 | 38,5 / 31,0 | **−12,5 ± 3,4** | −10,3 ± 1,6 |
| plan agressif | 400 | 41,8 % ± 2,5 | 44,5 / 39,0 | −5,5 ± 3,3 | −5,1 ± 1,5 |
| Void Gate forcé | 400 | 38,3 % ± 2,4 | 42,5 / 34,0 | **−9,0 ± 3,2** | −7,9 ± 2,3 |
| plan Gorica contre Hook tempo (Windswept seulement quand LeBlanc commence) | 400 | 49,8 % ± 2,5 | 55,0 / 44,5 | +2,5 ± 2,3 | +4,6 ± 1,0 |
| Commencer (REF, non apparié) | | | | +5,5 ± 5,0 | +9,0 ± 2,2 |

## Ce que ça change
- **Les acquis tiennent avec les nouvelles règles**, dans le même sens et à la même ampleur : plan ~+12, Void Gate ~−9,
  agressif ~−5. Les deux premiers dépassent 2 écarts-types dès ce bloc ; l'agressif (1,7 écart-type) attend un second
  bloc de la nouvelle version.
- Windswept (+2,5 ± 2,3) et commencer (+5,5 ± 5,0) sont plus petits sur ce bloc mais compatibles avec les anciens
  chiffres : on ne sait pas encore pour la nouvelle version.
- Niveau de référence 47,3 %, dans la fourchette de l'ancienne version.

## Autocritique
- Un seul bloc de la nouvelle version : je ne réécris pas les acquis, je dis qu'ils sont « retrouvés » et je garde les
  chiffres de l'ancienne version séparés.
- L'écart « commencer » est lu dans une ligne non conçue pour ça (200 parties de chaque côté) : son intervalle est large.
- La reprise après redémarrage rejoue les jobs restants sur le moteur figé : je n'ai pas revérifié graine à graine que
  les deux premiers jobs se rejoueraient à l'identique (le moteur est déterministe, vérifié plus tôt).

## Prochaine session (17)
Deuxième bloc de la nouvelle version : REF, agressif, Void Gate, Hook tempo (Windswept). Ensuite, si tout tient,
reprendre les questions ouvertes.
