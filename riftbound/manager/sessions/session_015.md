# Session 015 — 2026-10-06  (moteur 867f1dd6b0, plans.py 8b2147fcef)

**Version.** Le hash change encore à cause de `train.py` seul (modifié à 17:50, toujours importé par aucun module du
jeu) : même jeu que s004-s014. Le fil Replays a changé les **règles** du moteur à 18:31 UTC (rune recyclée épuisée
d'abord, énergie flottante ; bug des cartes en double dans la main). Cette session a figé son moteur à 18:03 : elle
est **dans l'ancienne version**. La suivante sera la première de la nouvelle version.

Graines neuves 206000-206399, liste stock, contre LeBlanc Deathknell.

## Questions posées
1. Deuxième bloc : plan agressif au lieu du plan Gorica contre Deathknell (s014 : −3,0 ± 3,2).
2. **Nouvelle** : ce que vaut le plan Gorica contre Deathknell (contre aucun plan).

## Résultats
| Config | n | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la REF |
|---|---|---|---|---|
| **REF** stock, plan Gorica, contre Deathknell | 400 | 49,3 % ± 2,5 | 57,2 / 41,7 | — |
| stock, plan agressif | 400 | 44,5 % ± 2,5 | 55,7 / 34,0 | −4,8 ± 3,0 |
| stock, sans plan | 400 | 37,5 % ± 2,4 | 48,5 / 27,2 | **−11,8 ± 3,3** |

Cumuls (ancienne version, blocs indépendants) :
- Agressif contre Deathknell : −3,0 (s014), −4,8 (s015) → −4,0 ± 2,2 (1,8 écart-type). Avec Hook tempo (−6,1 ± 2,1,
  s007-s008), et puisque le plan de LeBlanc ne change rien pour Akali (s014) : **−5,1 ± 1,5** contre les deux plans.
- Plan Gorica contre aucun plan : +9,8 ± 1,9 contre Hook tempo (s007-s009), +11,8 ± 3,3 contre Deathknell (s015)
  → **+10,3 ± 1,6** contre les deux plans.

## Ce que ça change
- **Acquis (ancienne version)** : le plan Gorica vaut ~10 pts et jouer agressif coûte ~5 pts, quel que soit le plan
  de LeBlanc.
- Contre Deathknell seul, l'agressif reste sous 2 écarts-types : je ne l'annonce que dans le cumul.
- **Commencer** : contre Deathknell aussi, ~15 pts d'écart sur les références s014-s015 (non apparié).

## Autocritique
- Le cumul « agressif contre les deux plans » repose sur le résultat s014 (Hook tempo ≈ Deathknell pour Akali) ;
  c'est un choix, je l'écris.
- **Replays** : le fichier `add_replays.py` rejoue les parties avec le moteur vivant. Après le changement de règles,
  la première partie s'est rejouée différemment (ATTENTION 0 contre 1). Je les ai réenregistrées avec le moteur figé
  de la session (copie de travail, sans toucher aux fichiers du fil Replays) : 0 avertissement. Procédure ajoutée au
  protocole (étape 5b).
- Tous les chiffres « acquis » du journal sont désormais de l'ancienne version. Ils peuvent changer avec les nouvelles
  règles : l'énergie flottante aide surtout les tours à plusieurs sorts, ce qui pourrait toucher Akali comme LeBlanc.

## Prochaine session (16) — nouvelle version du moteur
Remesurer les acquis principaux sur bloc neuf, sans rien cumuler avec avant : plan Gorica contre aucun plan, plan
agressif, Void Gate au lieu de Sigil contre Windswept, Windswept présenté ou non (Hook tempo), et commencer (lu dans
la référence).
