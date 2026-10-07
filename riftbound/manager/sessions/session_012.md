# Session 012 — 2026-10-06  (moteur 7daf3a6f86, plans.py 8b2147fcef)

Même moteur que la session 11 (7daf3a6f86 = moteur s004-s010 + `train.py` non importé) : cumul autorisé avec
s004-s011. Graines neuves 204800-205199, liste stock, plan Gorica.

## Questions posées
1. **Fixée** : confirmer sur bloc neuf que Void Gate coûte des points face à Windswept (s011 : −8,8 ± 3,3).
2. Troisième bloc pour Forgotten Monument face à Windswept (s010 −10,2, s011 −1,8).
3. **Fixée** : contre LeBlanc « Hook tempo » (Windswept quand elle commence, Star Spring sinon), le plan présente
   Void Gate face à Star Spring. Est-ce que Sigil forcé ferait mieux ?

## Résultats
| Config | n | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié |
|---|---|---|---|---|
| **REF** plan Gorica, Sigil, contre Hook tempo Windswept | 400 | 50,8 % ± 2,5 | 51,8 / 49,8 | — |
| Void Gate forcé, contre Windswept | 400 | 43,8 % ± 2,5 | 49,7 / 37,9 | **−7,0 ± 3,3** vs REF |
| Forgotten Monument forcé, contre Windswept | 400 | 41,3 % ± 2,5 | 49,7 / 33,0 | **−9,5 ± 3,3** vs REF |
| plan Gorica (son choix), contre Hook tempo | 400 | 56,5 % ± 2,5 | 63,5 / 49,8 | +5,8 ± 2,3 vs REF |
| Sigil forcé, contre Hook tempo | 400 | 56,0 % ± 2,5 | 62,4 / 49,8 | −0,5 ± 2,3 vs choix du plan |

Contre Hook tempo, le plan a présenté Sigil face à Windswept (203 parties) et Void Gate face à Star Spring (197).
Sur ces 197 parties, Sigil forcé − Void Gate = **−1,0 ± 4,7** : on ne sait pas, et rien n'indique que Sigil ferait mieux.

Cumuls (blocs indépendants, même version de jeu) :
- **Void Gate − Sigil face à Windswept** : −8,8 (s011) et −7,0 (s012) → **−7,9 ± 2,3**. Acquis.
- **Forgotten Monument − Sigil face à Windswept** : sur les deux blocs non sélectionnés (s011, s012) **−5,7 ± 2,3** ;
  avec s010 (promu parce que grand) −7,1 ± 1,9. Acquis, ampleur ~6 pts (entre 1 et 10).
- **Windswept présenté par LeBlanc** : +5,8 ± 2,3 (s012) avec +4,3 ± 1,3 (s4, s7, s8) → **+4,7 ± 1,1**. Ici l'écart
  mélange deux choses : le battlefield de LeBlanc et celui d'Akali (Sigil contre Void Gate quand Akali commence).

## Ce que ça change
- **Acquis** : face à Windswept, avec le plan, **Sigil est le bon battlefield**. Void Gate coûte ~8 pts
  (−7,9 ± 2,3) et Forgotten Monument ~6 pts (−5,7 ± 2,3).
- **Pas d'effet mesurable** face à Star Spring : Void Gate (choix du plan) et Sigil se valent (−1,0 ± 4,7 sur
  197 parties). La règle du plan « Void Gate sauf contre Windswept » n'est pas prise en défaut ; le coût de Void
  Gate est propre au matchup contre Windswept.
- La question du battlefield d'Akali contre LeBlanc est **réglée** pour cette IA : Sigil contre Windswept,
  choix libre sinon.

## Autocritique
- En session 11, j'ai écrit que la piste Forgotten Monument de s010 était « surévaluée » et je l'ai rangée dans
  « Chiffres périmés ». Avec un troisième bloc (−9,5), c'est s011 qui était le bloc bas : l'effet existe, autour
  de 6 pts. J'ai sur-corrigé : un seul bloc à −1,8 ± 3,3 ne réfutait pas −10,2, il disait « on ne sait pas ». À
  retenir : une piste non confirmée n'est pas une piste réfutée.
- Le −1,0 ± 4,7 face à Star Spring ne porte que sur 197 parties : il exclut un gros effet (≥ 10 pts) mais pas un
  petit (~5 pts).
- Niveaux absolus : la référence fait 45,5 % (s011) puis 50,8 % (s012), toujours dans la dispersion de ± 5 entre
  blocs.

## Prochaine session (13)
La question du battlefield est close. Questions suivantes :
1. Troisième bloc propre pour Forgotten Monument face à Windswept (le cumul est à 2,5 écarts-types).
2. **Nouvelle question** : niveau d'Akali (plan Gorica) contre l'autre plan LeBlanc du moteur, « Deathknell »,
   avec la référence Hook tempo Windswept sur les mêmes graines.
3. Deuxième bloc Hook tempo (plan) − Sigil forcé, pour resserrer le « pas d'effet » face à Star Spring.
