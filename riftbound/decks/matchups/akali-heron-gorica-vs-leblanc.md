# Akali Heron (liste Gorica) vs LeBlanc

> **Correctif du 2026-10-02 (retex)** : sur 640 parties neuves par liste, G2 fait 51,2 % contre 44,7 % pour
> la liste stock, soit **+6,6 ± 2,8 points**. Le +8,7 ci-dessous incluait le lot où G2 avait été choisi.
> L'affirmation « garder Block et Not So Fast vaut 10 points » n'est **pas** soutenue, et le classement des
> battlefields est du bruit. La « LeBlanc sidée » est une hypothèse construite, pas un vrai sideboard.
> Voir `retex/retex-etude-akali-leblanc.md`.

Rédigé le 2026-10-02. Moteur fidèle `engine/` (même IA et même LeBlanc IQ #5 que
`akali-vs-leblanc-moteur-fidele.md`). Script : `engine/exp_gorica.py`, résultats bruts : `engine/results_gorica.json`.
Liste de départ : Gorica, RQ Singapour (`decks/raw/akali_gorica_rq-singapore.txt`). 2 400 parties.

## En bref
- La liste de Gorica telle quelle gagne **44,2 %** (480 parties sur 3 jeux de graines) : déjà mieux que la
  liste Wuhan (40,0 % contre 33,1 % sur les mêmes parties).
- **G2 = −2 Long Sword, −3 Defy, +5 unités** gagne **52,9 %** (480 parties) : **+8,7 points**, environ
  2,7 écarts-types, donc un gain réel. C'est le même enseignement que pour la liste Wuhan.
- Contre la LeBlanc sidée, l'écart est encore plus grand : **46,2 % contre 35,0 %**.
- Garde **Block et Not So Fast** : les remplacer par 2 Scuttle Crab fait retomber G2 de 57,5 % à 47,5 %
  sur les mêmes parties.

## Deck building (160 parties par ligne)
| Liste | Graines 0+ | 10000+ | 20000+ | Moyenne |
|---|---|---|---|---|
| Gorica stock | 40,0 % | 44,4 % | 48,1 % | **44,2 %** |
| GL : −2 Long Sword, +1 Akali, Silent, +1 Ferrous Forerunner | 48,1 % | 48,1 % | | 48,1 % |
| GD : −3 Defy, +2 Ferrous Forerunner, +1 Lonely Poro | 47,5 % | | | 47,5 % |
| **G2 : −2 Long Sword, −3 Defy, +1 Silent, +2 Forerunner, +1 Poro, +1 Scuttle Crab** | 57,5 % | 47,5 % | 53,8 % | **52,9 %** |
| GB : G2 −Block −Not So Fast +2 Scuttle Crab | 47,5 % | | | 47,5 % |

Chaque coupe seule (Long Sword ou Defy) vaut environ +4 points ; les deux ensemble se cumulent.

## Battlefields (liste G2, 160 parties chacun)
| Battlefield présenté | Winrate |
|---|---|
| Sigil of the Storm | 55,6 % |
| Void Gate | 52,5 % |
| Forgotten Monument | 46,9 % |

Écart de 8,7 points entre Sigil et Forgotten Monument, à la limite du bruit (±4,5 pts par ligne) : léger
avantage à Sigil of the Storm, sans certitude. Côté LeBlanc, l'étude précédente reste valable : Windswept
Hillock est son meilleur battlefield contre toi.

## Sideboard (parties 2 et 3)
| | vs LeBlanc de base | vs LeBlanc sidée |
|---|---|---|
| Gorica stock | 44,2 % | 35,0 % |
| G2 | 52,9 % | 46,2 % |

## Deck recommandé (G2, 40 cartes)
**Unités (24)** : 1 Akali, Deadly Weapon (champion) · 2 Mischievous Marai · 3 Mournful Witness ·
3 Kai'Sa, Survivor · 1 Noxus Hopeful · 3 Stellacorn Herder · 2 Blitzcrank, Impassive · 1 Darius, Trifarian ·
3 Astral Heron · 1 Akali, Silent · 2 Ferrous Forerunner · 1 Lonely Poro · 1 Scuttle Crab
**Équipement (4)** : 3 Zhonya's Hourglass · 1 Sterak's Gage
**Sorts (12)** : 3 Shuriken Flip · 3 Discipline · 2 Back Off · 2 Falling Star · 1 Block · 1 Not So Fast
**Runes** : 6 Calm / 6 Fury · **Battlefields** : Sigil of the Storm, Void Gate, Forgotten Monument

En BO3 sans changer de main deck : sors **3 Defy et 2 Long Sword**, rentre **1 Akali, Silent,
2 Ferrous Forerunner, 1 Lonely Poro, 1 Scuttle Crab**.

## Plan de jeu
Les leviers mesurés dans l'étude principale restent valables (même adversaire, même moteur) :
- **Prends l'initiative** quand tu as le choix : commencer valait +14 points.
- **Mulligan** : garde une main qui pose une unité aux tours 1-2 ; la politique de mulligan n'est pas un levier.
- **Ne garde pas 2 runes ouvertes pour un contre** : développer le board rapporte plus que Defy.
- Tu peux jouer prudemment face à Vi en Ambush (5 runes prêtes + une carte en main) sans perdre de winrate.

## Limites
Même IA à un coup que l'étude principale : les écarts entre listes sont plus fiables que les niveaux
absolus. Les lignes à 160 parties ont ±4 points d'écart-type ; seules les moyennes sur 480 parties
(stock et G2) sont solides.
