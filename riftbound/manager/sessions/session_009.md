# Session 009 — 2026-10-05  (moteur 44d6a1b695, plans.py 8b2147fcef, IA tempo + LeBlanc « réel »)

Même version que s004-s008. Bloc neuf 203600-203999, 400 parties par ligne, liste stock, LeBlanc IQ5 « Hook tempo
Windswept ». **Question fixée à l'avance** : d'où vient le gain du plan Gorica (acquis s008, ~8 pts) ? Sans plan,
Akali ne présente pas toujours Sigil of the Storm (les replays montrent Forgotten Monument, Void Gate ou Sigil
selon la donne) ; on force Sigil sans plan pour séparer le choix du
battlefield des décisions en partie. Résultats bruts : `results/session_009.json`.

## Résultats
| Config | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié |
|---|---|---|---|
| **Référence** : plan Gorica (présente Sigil) | **49,5 % ± 2,5** | 53,2 / 45,1 | — |
| sans plan, Sigil forcé | 38,5 % ± 2,4 | 39,8 / 37,0 | plan − (sans plan + Sigil) **+11,0 ± 3,3** |
| sans plan (battlefield choisi par l'IA) | 35,2 % ± 2,4 | 37,5 / 32,6 | plan − sans plan +14,2 ± 3,3 |

Question fixée : (sans plan + Sigil) − (sans plan) = **+3,2 ± 2,6**.

## Ce que ça change
- **Le gain du plan vient surtout des décisions en partie, pas du battlefield.** Présenter Sigil au lieu du
  battlefield choisi par l'IA n'apporte que +3,2 ± 2,6 (on ne sait pas si c'est plus que zéro) ; à battlefield égal, le plan
  vaut encore +11,0 ± 3,3 (3,3 ET). Piste forte, à rejouer sur bloc neuf avant d'en faire un acquis.
- Plan − sans plan sur ce bloc : +14,2 ± 3,3. Cumul s007-s009 (trois blocs) : **+9,8 ± 1,9**. L'acquis de s008
  (~8 pts) est confirmé et plutôt dans le haut de l'intervalle.
- Pour l'utilisateur : ce que le plan Gorica fait **pendant** la partie (quand attaquer, quoi tenir, quand garder
  ses cartes) compte plus que le choix du battlefield. Le fil Replays connaît le contenu exact du plan
  (`gameplans/gameplans.md`) : c'est là qu'il faut regarder quelles règles portent le gain.

## Autocritique
- La question fixée (Sigil − sans plan) ne passe pas 2 ET : je ne peux pas dire que le choix du battlefield ne
  compte pas, seulement qu'il compte moins que le reste (au plus ~8 pts, probablement ~3).
- Plan − (sans plan + Sigil) n'était pas la question fixée de cette session : il reste en pistes jusqu'au bloc neuf.
- Le niveau de référence (49,5 %) est dans la fourchette habituelle (45-51 %) ; sans plan, 35,2 % est le plus bas
  mesuré, cohérent avec 38,5-38,8 % en s007-s008 à la dispersion près.

## Prochaine session (010)
Questions fixées : plan − (sans plan + Sigil) sur bloc neuf ; et, avec le plan, présenter Sigil contre Forgotten
Monument (le choix du battlefield compte-t-il quand Akali joue bien ?).
