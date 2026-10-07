# Session 013 — 2026-10-06  (moteur 7daf3a6f86, plans.py 8b2147fcef)

Même moteur que s011-s012 (cumul autorisé avec s004-s012). Graines neuves 205200-205599, liste stock, plan Gorica.

## Questions posées
1. Troisième bloc propre : Forgotten Monument au lieu de Sigil face à Windswept.
2. Deuxième bloc : contre « Hook tempo », le choix du plan (Void Gate face à Star Spring) contre Sigil forcé.
3. **Nouvelle** : niveau d'Akali contre l'autre plan LeBlanc du moteur, « Deathknell », apparié à la référence.

## Résultats
| Config | n | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié |
|---|---|---|---|---|
| **REF** plan Gorica, Sigil, contre Hook tempo Windswept | 400 | 45,0 % ± 2,5 | 50,5 / 39,9 | — |
| Forgotten Monument forcé, contre Windswept | 400 | 39,5 % ± 2,4 | 44,8 / 34,6 | **−5,5 ± 3,3** vs REF |
| plan Gorica (son choix), contre Hook tempo | 400 | 49,2 % ± 2,5 | 59,4 / 39,9 | +4,2 ± 2,4 vs REF |
| Sigil forcé, contre Hook tempo | 400 | 47,8 % ± 2,5 | 56,2 / 39,9 | −1,5 ± 2,2 vs choix du plan |
| plan Gorica contre LeBlanc **Deathknell** | 400 | 46,2 % ± 2,5 | 53,1 / 39,9 | +1,2 ± 2,9 vs REF |

Le plan Deathknell présente les mêmes battlefields que Hook tempo dans le moteur (Windswept quand LeBlanc commence,
Star Spring sinon) ; ses parties diffèrent quand même (166 résultats identiques sur 208 quand LeBlanc commence).
Le 39,9 % commun quand LeBlanc commence vient de parties identiques pour les trois jobs Hook tempo, et d'une
coïncidence pour Deathknell.

Cumuls (blocs indépendants, même version de jeu) :
- **Forgotten Monument − Sigil face à Windswept**, trois blocs non sélectionnés (s011-s013) : **−5,6 ± 1,9**.
- **Sigil − Void Gate face à Star Spring** (parties où le battlefield diffère) : −1,0 ± 4,7 (s012), −3,1 ± 4,5 (s013)
  → **−2,1 ± 3,3** : pas d'effet mesurable, plutôt en faveur de Void Gate.
- **Windswept présenté par LeBlanc** : +4,2 ± 2,4 (s013) → cumul **+4,6 ± 1,0** sur cinq blocs.
- **Niveau de la référence** sur quatre blocs neufs (s010-s013) : 49,8 / 45,5 / 50,8 / 45,0 % → **47,8 %**,
  écart-type entre blocs 2,9 pts pour 2,5 attendus par le hasard. La dispersion « anormale » de la session 2
  ne se retrouve pas avec cette IA (χ² ≈ 4,1 à 3 ddl, p ≈ 0,25).

## Ce que ça change
- **Acquis renforcé** : Forgotten Monument coûte ~6 pts face à Windswept (−5,6 ± 1,9 sur trois blocs propres).
- **Acquis** : face à Star Spring, garder le choix du plan (Void Gate) ; Sigil n'apporte rien (−2,1 ± 3,3).
- **Nouveau** : contre LeBlanc Deathknell, Akali fait le même niveau que contre Hook tempo Windswept
  (+1,2 ± 2,9, on ne sait pas s'il y a un écart). Contre les deux plans LeBlanc du moteur, Akali tourne autour
  de 46-48 %.
- **Question 1b (dispersion entre blocs)** : plus de dispersion anormale pour la référence actuelle ; le ± 5 entre
  blocs de la session 2 était probablement lié à l'ancienne IA ou à une config différente. Je garde la prudence
  (± 3 sur un niveau à un bloc).

## Autocritique
- La comparaison Deathknell n'est appariée que pour moitié : quand Akali commence, LeBlanc présente Star Spring
  contre Deathknell mais Windswept est forcé dans la référence ; l'écart mélange le plan LeBlanc et le battlefield.
  La comparaison propre est « Hook tempo » contre « Deathknell » (mêmes battlefields) : 49,2 contre 46,2 %, à mesurer
  en apparié en s014.
- Le test de dispersion porte sur quatre blocs seulement ; il ne prouve pas l'absence de dispersion, il ne la
  montre pas.

## Prochaine session (14)
1. **Fixé** : Hook tempo − Deathknell à battlefields égaux (apparié, deuxième bloc pour Deathknell).
2. **Nouveau** : contre Deathknell, le side G2 au lieu de la liste stock (le side n'a été testé que contre Hook tempo).
3. **Nouveau** : contre Deathknell, plan agressif au lieu du plan Gorica (agressif coûte ~6 pts contre Hook tempo).
