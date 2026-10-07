# Session 010 — 2026-10-05  (moteur 44d6a1b695, plans.py 8b2147fcef, IA tempo + LeBlanc « réel »)

Même version que s004-s009. Bloc neuf 204000-204399, 400 parties par ligne, liste stock, LeBlanc IQ5 « Hook tempo
Windswept ». **Deux questions fixées à l'avance** : (1) confirmer que le plan vaut encore beaucoup à battlefield
égal ; (2) avec le plan, est-ce que le battlefield présenté compte ? Résultats : `results/session_010.json`.

## Résultats
| Config | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la référence |
|---|---|---|---|
| **Référence** : plan Gorica (présente Sigil) | **49,8 % ± 2,5** | 52,6 / 46,6 | — |
| sans plan, Sigil forcé | 39,2 % ± 2,4 | 50,2 / 27,0 | **+10,5 ± 3,1** |
| plan Gorica, Forgotten Monument forcé | 39,5 % ± 2,4 | 42,7 / 36,0 | **+10,2 ± 3,4** |

Cumul s009 + s010 : plan − (sans plan + Sigil) = **+10,7 ± 2,3**.

## Ce que ça change
- **Acquis : à battlefield égal, suivre le plan vaut ~11 points** (+11,0 en s009, +10,5 fixé en s010 ;
  cumul +10,7 ± 2,3, soit 6 à 15 points). Le gain du plan ne vient pas du choix du battlefield.
- **Nouveau : avec le plan, présenter Forgotten Monument au lieu de Sigil coûte 10,2 ± 3,4 points** (3,0 ET).
  Le battlefield compte donc beaucoup **quand Akali joue son plan**, alors qu'en s009 forcer Sigil sans plan
  n'apportait que +3,2 ± 2,6. Les deux ne se contredisent pas forcément : sans plan, l'IA présentait déjà Sigil
  une fois sur trois (136 Forgotten Monument / 134 Sigil / 130 Void Gate sur 400), donc l'écart mesuré en s009
  valait au plus deux tiers de l'effet complet. Mais une partie du −10,2 peut aussi venir du fait que le plan
  Gorica est écrit pour Sigil : lui imposer un autre battlefield le met en défaut. Je ne sais pas séparer les deux.
- Pour l'utilisateur, les deux ensemble : **présenter Sigil of the Storm et jouer le plan vont de pair** ;
  garder le plan mais changer de battlefield coûte autant que de jouer sans plan.

## Autocritique
- Les deux questions étaient fixées, les deux passent 3 ET : c'est solide pour le premier point (confirmé sur
  deux blocs) ; le second n'a qu'un bloc, il reste en « Pistes fortes ».
- « Sigil compte peu sans plan » (s009) et « Sigil compte beaucoup avec le plan » (s010) : je mets les deux dans
  le journal avec l'explication ci-dessus, sans prétendre savoir laquelle des deux causes domine.
- Détail notable : sans plan avec Sigil forcé, Akali fait 50,2 % quand elle commence et 27,0 % quand LeBlanc
  commence. L'écart d'initiative est deux fois plus grand sans plan qu'avec (52,6 / 46,6).

## Prochaine session (011)
Question fixée : **avec le plan, Sigil contre Void Gate forcé** (troisième battlefield du deck), pour savoir si
c'est Sigil qui est fort ou Forgotten Monument qui est faible. Deuxième job : avec le plan et Forgotten Monument
forcé, rejouer l'écart de s010 sur bloc neuf.
