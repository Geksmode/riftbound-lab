# Session 014 — 2026-10-06  (moteur 7daf3a6f86, plans.py 8b2147fcef)

Même moteur que s011-s013. Graines neuves 205600-205999, contre LeBlanc **Deathknell** (référence) et Hook tempo.

## Questions posées
1. **Fixée** : Hook tempo − Deathknell à battlefields égaux (les deux plans présentent Windswept quand LeBlanc
   commence et Star Spring sinon), en apparié.
2. **Nouvelle** : contre Deathknell, le side G2 au lieu de la liste stock.
3. **Nouvelle** : contre Deathknell, le plan agressif au lieu du plan Gorica.

## Résultats
| Config | n | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la REF |
|---|---|---|---|---|
| **REF** stock, plan Gorica, contre Deathknell | 400 | 49,0 % ± 2,5 | 56,3 / 41,2 | — |
| stock, plan Gorica, contre Hook tempo | 400 | 48,5 % ± 2,5 | 57,3 / 39,2 | −0,5 ± 2,4 |
| G2, plan Gorica, contre Deathknell | 400 | 49,8 % ± 2,5 | 58,7 / 40,2 | +0,8 ± 3,3 |
| stock, plan agressif, contre Deathknell | 400 | 46,0 % ± 2,5 | 51,5 / 40,2 | −3,0 ± 3,2 |

## Ce que ça change
- **Le plan de LeBlanc (Hook tempo ou Deathknell) ne change rien pour Akali** à battlefields égaux : −0,5 ± 2,4.
  Les deux plans du moteur se valent de son point de vue ; ce qui compte, c'est le battlefield que LeBlanc présente.
- **Le side G2 ne change rien non plus contre Deathknell** (+0,8 ± 3,3), comme contre Hook tempo (+0,4 ± 1,9).
- **Jouer agressif contre Deathknell** : −3,0 ± 3,2, on ne sait pas. Contre Hook tempo c'était −6,1 ± 2,1 (acquis) ;
  le sens est le même, l'ampleur n'est pas établie.
- **Commencer** (relu sur les références s007-s013, contre Hook tempo Windswept, sept blocs indépendants) : Akali gagne
  **+9,0 ± 2,2 pts** de plus quand elle commence (de 2 à 16,5 selon le bloc). Contre Hook tempo sans Windswept forcé,
  l'écart monte vers 15-20 pts parce que LeBlanc ne présente Windswept que quand elle commence.

## Autocritique
- Trois questions, trois « pas d'écart ». Le matchup tel que le moteur le joue est maintenant bien décrit par quelques
  facteurs (plan Gorica, Sigil contre Windswept, commencer, Windswept chez LeBlanc) ; le reste est sous ± 3-4 pts.
  Les sessions suivantes ont un rendement décroissant : chaque nouvelle question a de bonnes chances de finir en
  « on ne sait pas » à 400 parties.
- Le chiffre « commencer +9 » vient de lignes qui n'étaient pas conçues pour ça (la répartition commence/second dépend
  de la graine) ; il est non apparié, mais les sept blocs sont indépendants et vont tous dans le même sens.
- L'IA reste la même depuis s004 ; ses limites connues (côté en retard qui passe son tour) pèsent sur tous les chiffres.

## Prochaine session (15)
1. Deuxième bloc : plan agressif contre Deathknell.
2. **Nouvelle** : ce que vaut le plan Gorica contre Deathknell (contre aucun plan) ; contre Hook tempo il vaut ~10 pts.
