# Session 004 — 2026-10-04  (moteur 44d6a1b695, plans.py 8b2147fcef, **IA tempo + LeBlanc « réel » + règles vidéos**)

Nouvelle version de l'IA par rapport à s003 : le fil Replays a fait garder à LeBlanc le Reflet et l'unité copiée
sur le battlefield (retour de l'utilisateur sur le jeu réel) et ajouté des règles tirées de deux vidéos LeBlanc.
Les niveaux ne se comparent donc pas à s003 ; seuls les écarts appariés de cette session comptent. Akali suit le
plan Gorica, LeBlanc IQ5 suit « Hook tempo Windswept » (W) ou « Hook tempo » (libre). 400 parties par ligne,
bloc neuf 201600-201999. Résultats bruts : `results/session_004.json`.

## Questions posées
1. Confirmer sur un bloc neuf la piste de s003 : GL (−2 Long Sword, +1 Akali Silent, +1 Ferrous Forerunner, on
   garde Defy) bat la liste stock et G2.

## Résultats
| Config | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la référence |
|---|---|---|---|
| **Référence** : GL vs Hook tempo Windswept | **49,3 % ± 2,5** | 55,2 / 43,1 | — |
| Gorica stock vs Hook tempo Windswept | 51,3 % ± 2,5 | 57,6 / 44,7 | GL − stock **−2,0 ± 3,4** |
| G2 vs Hook tempo Windswept | 51,5 % ± 2,5 | 58,6 / 44,2 | GL − G2 −2,2 ± 3,4 |
| GL vs Hook tempo (battlefield libre) | 52,5 % ± 2,5 | 61,6 / 43,1 | +3,2 ± 2,3 (effet Windswept) |
| Gorica stock vs Hook tempo | 53,5 % ± 2,5 | 62,1 / 44,7 | GL − stock −1,0 ± 3,3 |

Autres écarts appariés : G2 − stock (W) +0,2 ± 3,2 ; stock libre − stock W +2,2 ± 2,2.

## Ce que ça change
- **La piste GL ne se confirme pas.** Sur le bloc neuf, GL fait −2,0 ± 3,4 contre la liste stock face à Windswept
  et −1,0 ± 3,3 sans, au lieu de +6,3 en s003. Elle quitte les pistes : **aucune des trois listes (stock, G2, GL)
  ne se détache** contre un LeBlanc qui suit un plan. Garder la liste stock reste le choix par défaut.
- **G2 contre stock** : +0,2 ± 3,2. Quatrième mesure sous plans dans le bruit (s001 +5,0, s002 −3,0, s003 +3,8,
  s004 +0,2, trois versions d'IA). Le +6,6 du retex du 2 octobre ne vaut que sans plans.
- **Windswept** : +3,2 ± 2,3 (GL) et +2,2 ± 2,2 (stock) quand LeBlanc ne le force pas. Sous 2 écarts-types :
  avec cette IA, **on ne sait pas** si Windswept coûte encore des points.
- Niveau contre ce LeBlanc : ~49-53 % sur ce bloc, à ± 5 près vu la dispersion entre blocs.

## Autocritique
- **J'avais raison de ne pas annoncer GL en s003.** Le +8,3 était le plus grand de 15 écarts : la sélection l'a
  gonflé, et ce bloc ramène l'écart à zéro. Je ne peux pas séparer cet effet du changement d'IA LeBlanc (les
  deux ont changé en même temps) ; dans les deux cas la piste ne tient pas.
- Le fil Replays dit que les nouveaux comportements LeBlanc n'ont pas d'effet mesurable sur Akali ; ce bloc va
  dans le même sens (niveaux proches de s003) mais je ne compare pas les niveaux entre versions.
- La sortie console du runner n'a pas été enregistrée cette fois (le log ne contient que le début et la fin) ;
  les résultats complets sont dans le JSON. À relancer avec `> results/session_NNN.log`.

## Prochaine session (005)
Les variantes de deck ne se distinguent pas à 400 parties. Accumuler des blocs indépendants de la même version
d'IA pour G2 − stock et GD − stock (la moitié « −3 Defy » de G2 n'a été mesurée qu'en s003) contre Hook tempo
Windswept. Si le moteur change encore, refaire ces jobs en apparié et ne rien regrouper.
