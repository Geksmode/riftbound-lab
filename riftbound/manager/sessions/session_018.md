# Session 018 — 2026-10-07  (moteur 95452d66e0, plans.py 8b2147fcef) — nouvelle version, 3e bloc

**Version.** Le moteur a été refondu ce matin (07:00-11:00, fil des cartes) : mots-clés génériques et un dossier
`cardsets/` de modules de cartes importé par cards.py. Le runner ne figeait que les `.py` du dossier engine : je l'ai
corrigé pour figer aussi `cardsets/` (hachage inchangé pour les anciennes sessions).
**Test d'équivalence** avant de cumuler : j'ai rejoué REF et agressif de s017 sur les mêmes 200 graines (206800-206999)
avec le moteur vivant avant le lancement, puis avec le moteur figé de s018 après : **400 parties sur 400 identiques
les deux fois** (même vainqueur graine par graine). Les règles du matchup n'ont pas bougé : s018 se cumule avec
s016-s017. (Les modules de cartes étaient encore en cours d'écriture pendant la session ; ils concernent des cartes
hors des deux decks.)

## Question posée
Troisième bloc de la nouvelle version : sans plan (2e bloc), Windswept (3e bloc), commencer, Forgotten Monument forcé
face à Windswept (1er bloc nouvelle version).

## Résultats (graines 207200-207599, liste stock, plan Gorica)
| Config | n | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la REF | Cumul s016-s018 | Ancienne version |
|---|---|---|---|---|---|---|
| **REF** plan Gorica, Sigil, contre Hook tempo Windswept | 400 | 46,0 % ± 2,5 | 48,4 / 44,0 | — | 47,6 % ± 1,4 | ~48 % |
| sans plan | 400 | 32,5 % ± 2,3 | 36,4 / 29,2 | **−13,5 ± 3,3** | **+13,0 ± 2,4** (plan − sans plan) | +10,3 ± 1,6 |
| Forgotten Monument forcé | 400 | 37,0 % ± 2,4 | 43,5 / 31,5 | **−9,0 ± 3,2** | −9,0 ± 3,2 (un bloc) | −5,6 ± 1,9 |
| contre Hook tempo (Windswept seulement quand LeBlanc commence) | 400 | 50,2 % ± 2,5 | 57,6 / 44,0 | +4,2 ± 2,1 | **+3,4 ± 1,3** | +4,6 ± 1,0 |
| Commencer (REF, non apparié) | | | | +4,4 ± 5,0 | +4,1 ± 2,9 | +9,0 ± 2,2 |

## Ce que ça change
- **Le plan Gorica vaut ~13 pts** dans la nouvelle version (deux blocs, +13,0 ± 2,4) : acquis.
- **Windswept présenté par LeBlanc coûte ~3 pts** (+3,4 ± 1,3, trois blocs, 2,6 écarts-types) : acquis retrouvé.
- **Forgotten Monument face à Windswept coûte ~9 pts** sur ce premier bloc : retrouvé (≥ 2 ET), dans le même sens que
  l'ancien −5,6. Sigil reste le seul bon choix face à Windswept.
- Commencer : +4,1 ± 2,9 sur trois blocs, **on ne sait pas**. L'écart avec l'ancien +9,0 (4,9 ± 3,6) n'est pas
  significatif non plus.
- Toutes les recommandations de l'ancienne version tiennent avec les nouvelles règles.

## Autocritique
- Le test d'équivalence porte sur deux configurations (REF et agressif) et 200 graines : il montre que le moteur joue
  ce matchup pareil, pas que chaque carte du pool se comporte pareil dans toutes les situations.
- La session a été figée pendant que les modules de cartes changeaient encore ; le second test sur le moteur figé de
  s018 couvre ce risque.
- Le déclencheur de 18h est arrivé pendant cette session : je n'ai pas lancé de session en plus (une à la fois, et le
  rythme reste celui que l'utilisateur n'a pas encore tranché).
- Les parties du viewer sont choisies parmi 73 donnes où la REF gagne et les deux variantes perdent : illustratives.

## Replays (viewer version 27)
s018-1 à s018-3 : même donne (Akali commence) ; référence 8-5, sans plan 3-8 avec le même Sigil (seule la conduite
change), Forgotten Monument 2-8. s018-4 : défaite type de la référence, 5-4 au tour 11 puis 5-8.

## Prochaine session (19)
Questions jamais rejouées dans la nouvelle version : plan de LeBlanc (Hook tempo − Deathknell à battlefields égaux,
fixé) et liste G2 contre stock (contre Hook tempo Windswept, fixé). Commencer lu dans la REF (4e bloc).
