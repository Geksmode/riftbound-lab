---
tags: [riftbound, matchup, akali, leblanc]
maj: 2026-10-02
---
# Akali vs LeBlanc

[[Akali, Rogue Assassin]] contre [[LeBlanc, Deceiver]]. Matchup défavorable, ampleur réelle inconnue.
Chiffres du [[Moteur de règles fidèle]], **corrigés au retex** ([[Retex étude Akali-LeBlanc]]).

## Données terrain
| Source | Résultat | Fiabilité |
|---|---|---|
| Riftools | Akali **35,2 %** (83-153, 236 matchs) | **non vérifiée** : lue dans un résumé de recherche, jamais sur la page |
| Yomi's Place, RQ Barcelone | LeBlanc ~80 % (10 matchs) | échantillon minuscule |

Toute la calibration du moteur repose sur ce ~35 % non vérifié. Le moteur donne ~45 % à la liste Gorica :
soit le moteur est optimiste pour Akali (hypothèse : l'IA joue mal Hidden et Baited Hook), soit le chiffre
terrain est faux ou vient d'autres listes. **On ne sait pas.**

## Ce qui est mesuré (et tient)
| Levier | Effet | Solidité |
|---|---|---|
| **Couper Long Sword et Defy pour des unités** | **+6,6 ± 2,8 pts** (Gorica, 640 parties neuves) ; +10,4 contre la LeBlanc de LA ; liste Wuhan à battlefield fixé : 37,5 % contre 30,0 % | réel **sans plans** ; contre un LeBlanc qui suit un plan, +5,0 ± 3,3 puis −3,0 ± 3,3 : **on ne sait pas** ([[Agent manager]] s1-s2) |
| **Commencer la partie** | Akali gagne plus en commençant dans les 47 configurations testées ; seule paire contrôlée : 45,0 % contre 37,5 % | réel, ampleur **5 à 10 points** |
| **LeBlanc présente Windswept Hillock** | **−8,0 ± 3,3 pts** pour Akali contre Star Spring (G2 avec plans, 400 parties neuves, session 1 de l'[[Agent manager]]) ; l'effet vient des parties où Akali commence | **réel** (même sens sur deux lots indépendants), ampleur 5 à 11 pts |
| Liste Gorica meilleure que Wuhan | 44,7 % (640) contre 31,6 % (320) | réel |

Pourquoi Hillock : le Ganking lui permet de déplacer ses grosses unités d'un battlefield à l'autre et de
reprendre celui que tu tiens. En BO3, un battlefield ne se joue qu'une fois par match.

## Ce qui est dans le bruit (pas de recommandation)
Battlefield présenté par Akali · politique de mulligan · Not So Fast · Block · retrait d'équipement contre
Baited Hook · jouer autour de Vi en Ambush. Aucun de ces choix n'a d'effet mesurable à ±4-5 points.

## Recommandations
1. **Side** : −3 Defy, −2 Long Sword ; +1 Akali, Silent, +2 Ferrous Forerunner, +1 Lonely Poro, +1 Scuttle
   Crab (voir [[Akali Heron (Gorica)]]).
2. **Commence** quand tu as le choix.
3. Le reste est un choix libre.

## Idées de plan de jeu (lecture des cartes, **non mesurées**)
- Gagner aux points avant que son moteur tourne, en tuant le moins possible : chaque unité tuée la nourrit.
- Bouger plutôt que tuer : Charm, Back Off, Blitzcrank, capacité de la légende.
- Cibles prioritaires : **Karthus** (double ses Deathknell), puis Baited Hook.
- Ne pas tuer Ruined Rex quand ses 4 dégâts tombent sur ta meilleure unité.
- Garder Zhonya caché contre Harnessed Dragon, Hidden Blade, Ruined Rex.
- Respecter Thousand-Tailed Watcher quand elle a 7+ ressources.
- Ne pas garder 2 runes ouvertes pour Defy au détriment du développement (cohérent avec la mesure ci-dessus).

## Non modélisé
Vilemaw's Lair, Decree of Rage, Decree of Focus ; le vrai sideboard de LeBlanc (la « LeBlanc sidée » du moteur
est inventée, ses chiffres testent une hypothèse).

Sources : `riftbound/retex/retex-etude-akali-leblanc.md` (référence), `riftbound/decks/matchups/`.

## Niveau contre un LeBlanc qui joue bien (Agent manager, session 2)
Contre LeBlanc « Hook tempo » qui présente Windswept à chaque partie, G2 + plan Gorica fait **~47 % ± 4**
(1 200 parties sur 3 blocs : 53,3 / 40,0 / 47,3 %). Un bloc de 400 parties peut s'écarter de ~6 points :
ne pas lire un niveau absolu sur un seul bloc. Battlefield d'Akali face à Windswept : Sigil of the Storm
(40,0 %) ≥ Forgotten Monument (39,8 %) > Void Gate (36,0 %), écarts dans le bruit ; garder Sigil.

Session 3, nouvelle IA « tempo » (niveaux non comparables) : G2 48,5 % ± 2,5 face à Windswept forcé,
52,8 % sans ; Windswept coûte donc ~4 pts (+4,2 ± 2,3). Piste à confirmer : garder les Defy et ne couper
que les 2 Long Sword (voir [[Akali Heron (Gorica)]]).

Session 4, LeBlanc « réel » (garde Reflet et unité copiée, règles tirées de vidéos) : stock 51,3 %, G2 51,5 %,
GL 49,3 % face à Windswept forcé (± 2,5 chacun, ± 5 entre blocs) ; écarts appariés tous dans le bruit. La piste
GL ne tient pas. Ne pas forcer Windswept : +2 à +3 pts, on ne sait pas si ça compte encore.

Session 5, même IA : stock 48,0 %, G2 47,2 %, GD 45,8 %, GL 53,2 % face à Windswept. Sur 800 parties
(s4+s5), G2 − stock −0,3 ± 2,3. Piste : garder les Defy plutôt que les Long Sword (GL − GD +7,5 ± 3,4).

Session 6 : cette piste est réfutée (−6,8 ± 3,3 sur la question fixée à l'avance). **Les quatre listes se valent
à ± 4 points** (G2 − stock +0,4 ± 1,9 sur 1 200 parties). Prochaines questions : le plan de jeu et Windswept.

Session 7 (liste stock, même IA) : **le plan compte plus que le side.** Plan Gorica 45,5 %, sans plan 38,5 %,
plan agressif 39,5 % face à Windswept : +7,0 ± 3,1 pour le plan, −6,0 ± 3,0 pour l'agressif (pistes, à rejouer en
session 8). Sans plan, Akali tombe à 28 % quand LeBlanc commence ; elle ne présente pas toujours Sigil
(Forgotten Monument, Void Gate ou Sigil selon la donne, d'après les replays de la session 9).

**Session 8 : confirmé sur bloc neuf.** Ce qui compte dans ce matchup (moteur, IA actuelle) :
| Facteur | Effet pour Akali | Statut |
|---|---|---|
| Suivre le plan Gorica (contre aucun plan) | **+10,3 ± 1,6** pts | acquis, contre Hook tempo et Deathknell (s7-s15) |
| … dont le choix de Sigil seul | +3,2 ± 2,6 pts | on ne sait pas (s9) |
| … dont les décisions en partie (à Sigil égal) | **+10,7 ± 2,3** pts | acquis (s9-s10) |
| Jouer agressif au lieu du plan | **−5,1 ± 1,5** pts | acquis, contre Hook tempo et Deathknell (s7-s15) |
| LeBlanc présente Windswept Hillock | **−4,6 ± 1,0** pts | acquis (s4, s7, s8, s12, s13) |
| Avec le plan, présenter Void Gate au lieu de Sigil (face à Windswept) | **−7,9 ± 2,3** pts | acquis (s11-s12) |
| Avec le plan, présenter Forgotten Monument au lieu de Sigil (face à Windswept) | **−5,6 ± 1,9** pts | acquis (s11-s13) |
| Face à Star Spring : Sigil au lieu de Void Gate | −2,1 ± 3,3 pts | pas d'effet, garder Void Gate (s12-s13) |
| Side G2 / demi-sides au lieu de la liste stock | moins de ± 4 pts | acquis (s4-s6), aussi contre Deathknell (s14) |
| Commencer la partie | **+9,0 ± 2,2** pts | acquis (sept blocs, s7-s13) |
| Plan de LeBlanc Hook tempo ou Deathknell (même battlefield) | −0,5 ± 2,4 pts | pas d'effet (s14) |

Session 10 (bloc neuf, liste stock, question fixée à l'avance) : plan Gorica avec Sigil 49,8 % ± 2,5, sans plan
avec Sigil forcé 39,3 %, plan Gorica avec Forgotten Monument forcé 39,5 %. À battlefield égal le plan vaut
**+10,5 ± 3,1** sur ce bloc, soit **+10,7 ± 2,3** cumulé avec la session 9 : c'est un acquis, le gain du plan
vient de ses décisions en partie et non du battlefield présenté. Nouvelle piste : avec le plan, présenter
Forgotten Monument au lieu de Sigil coûte **10,2 ± 3,4** pts — à prendre avec réserve, le plan Gorica est écrit
pour Sigil, donc la mesure mélange la valeur du battlefield et l'adéquation plan/battlefield.

Session 11 (bloc neuf, question fixée) : face à Windswept, avec le plan, **Sigil 45,5 %**, Void Gate forcé 36,8 %,
Forgotten Monument forcé 43,8 %. Void Gate coûte **8,8 ± 3,3** pts, qu'Akali commence ou non ; la perte de 10 pts
avec Forgotten Monument vue en session 10 ne se retrouve pas (−1,8 ± 3,3). Conseil pratique : **contre Windswept,
présente Sigil of the Storm** ; Forgotten Monument est au mieux équivalent, Void Gate est nettement moins bon.

**Session 12 : le battlefield d'Akali est réglé.** Contre Windswept, présente **Sigil of the Storm** : Void Gate
coûte ~8 pts et Forgotten Monument ~6 pts (deux blocs neufs chacun). Contre Star Spring, Void Gate (le choix du
plan) et Sigil se valent. Correction : en session 11 j'avais jugé la perte avec Forgotten Monument surévaluée
après un seul bloc à −1,8 ; le bloc suivant (−9,5) montre que l'effet existe, autour de 6 pts.

Session 13 : contre l'autre plan LeBlanc du moteur (**Deathknell**), Akali fait **46,2 %**, comme contre Hook tempo
Windswept (+1,2 ± 2,9, pas d'écart mesurable). Niveau de référence (liste stock, plan Gorica, contre Hook tempo
Windswept) sur quatre blocs neufs : **~48 %**, avec une dispersion entre blocs normale (± 3 sur un bloc). La session 14
teste, contre Deathknell, le side G2 et le plan agressif.

**Session 14 : ce qui compte dans ce matchup** (moteur, IA actuelle), par ordre d'importance : suivre le plan Gorica
(~10 pts), commencer (~9 pts), présenter Sigil contre Windswept (6 à 8 pts), ne pas jouer agressif (~6 pts contre
Hook tempo), le battlefield que présente LeBlanc (Windswept ~5 pts). Le side (G2 ou stock) et le plan précis de
LeBlanc (Hook tempo ou Deathknell) ne changent rien de mesurable.

**Session 15 et changement de moteur.** Contre Deathknell aussi, le plan Gorica vaut ~12 pts et l'agressif coûte ~5 pts.
Le 6 octobre à 18h31, le moteur a changé de règles (une rune recyclée est d'abord épuisée et son énergie reste
disponible jusqu'à la fin du tour ; correction des cartes en double dans la main). **Tous les chiffres de cette page
viennent de l'ancienne version** ; la session 16 les remesure sur le nouveau moteur.

**Session 16, nouveau moteur** (rune recyclée épuisée d'abord, énergie flottante) : les conseils ne changent pas.
Plan Gorica +12,5 ± 3,4 contre aucun plan, Void Gate −9,0 ± 3,2 au lieu de Sigil face à Windswept, agressif −5,5 ± 3,3,
niveau de référence 47 %. Windswept (+2,5 ± 2,3) et commencer (+5,5 ± 5,0) sont plus petits sur ce premier bloc :
deuxième bloc en session 17.

**Session 17, deuxième bloc du nouveau moteur** : **ne joue pas agressif** (−7,3 ± 2,2 sur deux blocs) et **présente Sigil
face à Windswept** (Void Gate −9,3 ± 2,2) : acquis aussi avec les nouvelles règles. Windswept +2,9 ± 1,6 et commencer
+4,0 ± 3,5 : on ne sait pas encore, peut-être moins qu'avant pour commencer. Niveau de référence 48 %.
