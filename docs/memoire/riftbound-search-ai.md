---
name: riftbound-search-ai
description: IA générale (SearchAgent) 2026-10-09 : tirages communs + élimination en passes (RB_SEARCH, défaut "sh"), mesures sur les 49 decks par défaut, essais d'évaluation et de politique non retenus, temps par coup
metadata:
  type: project
---
**Demande** (2026-10-09) : « améliore l'IA de base pour tous les decks » (SearchAgent / rollout / evaluate de `engine/ai.py`, pas les plans).

**Changement gardé : la recherche** (`ai.SEARCH`, variable `RB_SEARCH`, défaut `"sh"` ; `"old"` = avant le 2026-10-09).
- `crn` (tirages communs) : le k-ième tirage est le même monde caché et le même aléa pour toutes les options d'une décision
  (avant, chaque option avait son propre tirage : on comparait surtout la chance des tirages).
- `sh` : `crn` + élimination en passes (successive halving) : la meilleure moitié reçoit des tirages en plus, jusqu'à 2 options ;
  budget en plus = `sh_extra` (1,0) × options × tirages.

**Mesures** (`engine/exp_search.py`, résultats `engine/results_search.json`, paires appariées : même donne jouée deux fois, places
échangées ; score = moyenne de la paire ; ± écart-type sur les paires). Mode `defaut` : deux decks par défaut (`train.default_decks`,
49 légendes) tirés au hasard, IA générique des deux côtés. Chaque ligne = graines neuves.

| Comparaison | Paires | Graines | Moteur | Score de A |
|---|---|---|---|---|
| sh contre old2 (même budget de simulations) | 200 | 610000+ | 4f90798d18 | **55,8 % ± 1,9** |
| crn contre old (même budget) | 160 | 630000+ | 4f90798d18 | **58,4 % ± 2,3** |
| sh contre old (niveau Normal réel) | 160 | 620000+ | 4f90798d18 | **65,3 % ± 2,3** |
| sh contre crn | 120 | 650000+ | 573611f39e | **58,8 % ± 2,8** |
| sh contre old, Akali G2 (Gorica) contre LeBlanc IQ#5 (Hook tempo) | 120 | 690000+ | 4feb2aae97 | **62,5 % ± 2,9** |

**Essais non retenus** (A = sh + réglage, B = sh ; criblage 80 paires, graines 660000+ ou 680000+) : horizon 1 tour
(`h=1`) 43,1 % ± 3,0 (pire, plus rapide) ; horizon 3 tours 51,2 % ± 2,7 ; sorts [Reaction] comptés comme cartes de réaction
dans la main (`react_kw=1`) 49,4 % ± 1,1 ; politique de simulation qui garde ses sorts [Reaction] (`pol_keep=1`) 51,2 % ± 2,9 ;
poids de l'évaluation `bf=5` 48,8 % ± 1,5, `pts=9` 49,4 % ± 2,1, `might=1.1` 51,9 % ± 2,4, `card0=1.9` 53,1 % ± 2,4 puis
**49,1 % ± 1,6 sur 160 paires neuves (670000+)** : on ne sait pas, rien de gardé. Les réglages restent disponibles
(`exp_search.py ... "sh@card0=1.9" sh defaut`, poids dans `ai.EV`, clés `pol_*` dans `cfg`), défaut inchangé.

**Réserve ouverte pour réagir** (2026-10-10, retour de l'utilisateur : « les cartes en main sont des ressources, laisser
des runes ouvertes pour réagir »). Diagnostic dans le code : une carte en main vaut 1,4 (2,0 si réactive) contre environ 4 pour
une unité posée, une rune compte pareil ouverte ou engagée, et la politique des simulations ne réagit presque jamais (sorts
[Reaction] joués dans son propre tour, réponse seulement en affrontement perdu, jamais sur la chaîne).
Mesure descriptive (`engine/exp_reserve.py`, résultats `engine/results_reserve.json`, 60 parties `defaut`, graines 720100+,
moteur 0ba518879e, même version des deux côtés, taux par partie ± écart-type sur les parties) :

| Mesure | IA actuelle (sh) | sh@res=3,pol_keep=1,pol_react=1 |
|---|---|---|
| fins de tour avec une carte [Reaction]/[Action]/[Ambush] en main | 76,3 % ± 2,7 | 78,5 % ± 2,8 |
| … mais aucune payable (à sec) | **67,6 % ± 2,9** | **58,2 % ± 2,9** |
| fins de tour avec une réserve ouverte | 26,5 % ± 2,6 | 34,6 % ± 2,9 |
| réserve ouverte puis utilisée au tour adverse | 26,6 % ± 4,0 | 25,2 % ± 3,4 |
| cartes jouées pendant le tour adverse, par joueur et par partie | 0,68 ± 0,09 | 0,86 ± 0,12 |
| sorts [Reaction] joués dans son propre tour principal | 1,32 ± 0,13 | 1,04 ± 0,11 |

Les deux colonnes sont jouées sur les mêmes donnes (pas indépendantes) : elles montrent que les réglages déplacent le
comportement, pas qu'ils font gagner. Réglages ajoutés, tous désactivés par défaut : `res` (poids de `ai.EV`, bonus si le
joueur finit son tour en cours avec `ai.reserve()` ≥ 1 : runes prêtes + carte à réaction payable), `pol_react` (politique des
simulations : répond avec un sort à ce qui cible ses objets, joue en affrontement perdu ou égal), `pol_keep` (existant).
**Taux de victoire non mesuré** : à faire sur le Mac de l'utilisateur (criblage environ 80 paires `defaut`, puis graines neuves).
Replays : groupe « IA : réserve ouverte » (`ia-res-720113a/b`, même donne).

**Deuxième version, demandée par l'utilisateur** (2026-10-10), trois réglages de la politique des simulations, désactivés par défaut :
- `pol_hold` : ne joue pas ce qui laisserait trop peu de runes prêtes pour payer sa meilleure réaction en main
  (`held_reaction`, `keeps_reserve` ; coût imprimé, Puissance payée d'abord avec des runes engagées). « Poser A puis garder
  2 runes » devient comparable à « tout poser » ;
- `pol_swing` : en combat sur un champ de bataille que l'on défend, joue la réaction qui fait passer l'affrontement de perdu
  ou égal à gagné. Chaque option est essayée sur une copie de la partie avec les vraies cartes (`probe`, au plus `SWING_MAX` = 6
  options), sinon on passe. Sur la chaîne, répond avec un sort à ce qui cible ses objets (comme `pol_react`) ;
- `pol_wary=N` : n'attaque pas un champ de bataille occupé avec une avance de Might de N ou moins si l'adversaire a 2 runes
  prêtes ou plus et une carte en main (information publique seulement). L'exception est de casser une tenue gagnante.

Même mesure, même moteur (2df144c930), mêmes 60 donnes (720100+, donc les deux colonnes ne sont pas indépendantes) :

| Mesure | IA actuelle (sh) | sh@res=3,pol_hold=1,pol_swing=1,pol_wary=2 |
|---|---|---|
| fins de tour avec une carte à réaction en main | 76,3 % ± 2,7 | 78,8 % ± 2,9 |
| … mais aucune payable (à sec) | **67,6 % ± 2,9** | **43,4 % ± 3,1** |
| fins de tour avec une réserve ouverte | 26,5 % ± 2,6 | 45,7 % ± 2,9 |
| réserve ouverte puis utilisée au tour adverse | 26,6 % ± 4,0 | 29,7 % ± 3,1 |
| cartes jouées pendant le tour adverse, par joueur et par partie | 0,68 ± 0,09 | 1,02 ± 0,10 |
| sorts [Reaction] joués dans son propre tour principal | 1,32 ± 0,13 | 1,12 ± 0,11 |

L'IA actuelle donne exactement les mêmes chiffres sur les deux moteurs (défaut inchangé). Taux de victoire et temps par
décision non mesurés (`probe` copie la partie à chaque option essayée) : à faire sur le Mac.

**Temps par décision** (décisions à plusieurs options). Mêmes 100 positions (graines 777003 et 777011, `engine/exp_temps.py`, Pyodide : `node engine/exp_temps.mjs`) :
CPython old 267 ms (p95 1,5 s), sh 355 ms (p95 1,75 s) ; **Pyodide 0.26.4 sous node** old 417 ms (p95 2,4 s),
sh 503 ms (p95 2,4 s), crn 389 ms (p95 2,0 s). Dans les parties des simulations (CPython, 4 processus) : old ≈ 245 ms,
sh ≈ 360 ms (× 1,5). Le p95 vient surtout du mode « urgent » (6 tirages quand l'adversaire est à 6+).
Non mesuré : temps dans un vrai navigateur (téléphone compris).

**Ce qui reste inconnu** : effet sur les decks au hasard (`miroir`) ; niveau Fort (3 tirages) ; réglage de `sh_extra` ;
les pourcentages Akali contre LeBlanc publiés avant le 2026-10-09 ont été mesurés avec l'ancienne recherche (autre moteur).
Replays : groupes « IA générale » du lecteur (`ia-sh-620040a/b`, `ia-sh-akali-690001a/b`, `ia-h1-660006`).
Related: [[riftbound-tempo-ai]], [[riftbound-gameplans]].
