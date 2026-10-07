# Session 003 — 2026-10-03  (moteur 1935fc174c, plans.py c8afb95476, **IA « tempo »**)

Première session avec la nouvelle IA du fil Replays (« gagner à tout prix », `ai.TEMPO`). Les niveaux ne se
comparent pas aux sessions 1-2 ; seuls les écarts appariés de cette session comptent. Akali suit le plan Gorica,
LeBlanc IQ5 suit « Hook tempo Windswept » (Windswept à chaque partie) ou « Hook tempo ». 400 parties par ligne,
bloc neuf 201200-201599. Résultats bruts : `results/session_003.json`.

## Questions posées
1. G2 contre stock contre un LeBlanc qui suit un plan, avec et sans Windswept forcé.
2. Quelle moitié de G2 compte : −2 Long Sword (GL : +1 Akali Silent, +1 Ferrous Forerunner) ou −3 Defy
   (GD : +2 Ferrous Forerunner, +1 Lonely Poro) ?

## Résultats
| Config | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la référence |
|---|---|---|---|
| **Référence** : G2 vs Hook tempo Windswept | **48,5 % ± 2,5** | 57,7 / 39,2 | — |
| Gorica stock vs Hook tempo Windswept | 44,8 % ± 2,5 | 46,8 / 42,7 | −3,8 ± 3,3 |
| GL (−2 Long Sword) vs Hook tempo Windswept | 51,0 % ± 2,5 | 57,7 / 44,2 | +2,5 ± 3,4 |
| GD (−3 Defy) vs Hook tempo Windswept | 42,8 % ± 2,5 | 46,3 / 39,2 | −5,8 ± 3,3 |
| G2 vs Hook tempo (battlefield libre) | 52,8 % ± 2,5 | 66,2 / 39,2 | +4,2 ± 2,3 |
| Gorica stock vs Hook tempo | 51,0 % ± 2,5 | 59,2 / 42,7 | +2,5 ± 3,2 |

Autres écarts appariés (mêmes graines) : GL − GD **+8,3 ± 3,3** ; GL − stock +6,3 ± 3,3 ; GD − stock −2,0 ± 3,3 ;
G2 − stock sans Windswept forcé +1,8 ± 3,2.

## Ce que ça change
- **G2 contre stock** : +3,8 ± 3,3 face à Windswept, +1,8 ± 3,2 sans. Toujours sous 2 écarts-types. Avec la
  session 2 (−3,0, ancienne IA), **on ne sait toujours pas** si G2 vaut mieux que la liste stock contre un
  LeBlanc qui suit un plan.
- **Piste nouvelle : c'est la moitié Long Sword qui compte, pas la moitié Defy.** GL (on garde les 3 Defy) bat
  GD de 8,3 ± 3,3 (2,5 ET) et la liste stock de 6,3 ± 3,3. Couper les Defy n'apporte rien (GD − stock −2,0).
  Si ça se confirme, le side devient **−2 Long Sword, +1 Akali Silent, +1 Ferrous Forerunner**, en gardant Defy,
  ce qui rejoint le retex riftbound.gg (Defy utile contre le field).
- **Windswept coûte encore ~4 pts avec l'IA tempo** (+4,2 ± 2,3 sans Windswept forcé, 1,8 ET), moins que les
  −8 de la session 1. L'IA tempo défend peut-être mieux la tenue adverse.

## Autocritique
- **GL − GD est l'écart le plus grand parmi 15 comparaisons possibles.** Il est donc gonflé par la sélection.
  Je ne l'annonce pas comme acquis : il passe en « Pistes » jusqu'à un bloc neuf (session 4).
- La session 2 avait déjà classé G2 contre stock « on ne sait pas » ; ce lot ne change pas ce verdict, même si le
  signe est redevenu positif. Je ne fais pas de moyenne avec la session 2, l'IA a changé entre les deux.
- L'IA tempo est plus lente (~15 min par configuration au lieu de ~6) : une session de 6 configurations tient
  dans l'heure et demie, pas plus.

## Prochaine session (004)
Confirmer GL sur un bloc neuf : GL, G2 et stock contre Hook tempo Windswept et contre Hook tempo.
