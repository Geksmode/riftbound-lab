# Journal d'apprentissages de l'agent manager

État courant de ce qu'on sait sur la liste **Akali Heron de Gorica** (RQ Singapour) et ses matchups.
**Versions de l'IA** : s001 plans fd91de6a59 ; s002 plans 7a015a956e (moteur non haché avant s003), ancienne IA. **À partir de s003 : IA « tempo »** (ai.TEMPO, choisie parce
que l'utilisateur veut une IA qui joue pour gagner) ; les niveaux s001-s002 ne se comparent plus, seuls les
écarts appariés restent informatifs (avec prudence). Un chiffre
n'est comparable qu'à un chiffre de la même version ; à chaque changement d'IA, les niveaux sont à remesurer.
**s004+ : IA tempo + LeBlanc « réel » (moteur 44d6a1b695, plans 8b2147fcef).** Changement arrivé le 2026-10-03 22h UTC (actif par défaut ; RB_REFL=0 / RB_VIDEO=0 pour couper) : le fil Replays a modifié l'IA LeBlanc pour qu'elle garde le clone et
l'unité sur le battlefield au lieu de renvoyer le clone en base (retour de l'utilisateur sur le jeu réel), plus des règles LeBlanc tirées de deux vidéos. Si
`engine_md5` ou `plans_md5` diffère de s003 (1935fc174c / c8afb95476), la session est une **nouvelle version** :
la confirmation de la piste GL doit alors se refaire en apparié dans la session, sans comparer aux niveaux s003.
**s011 : moteur 7daf3a6f86** — seule différence avec 44d6a1b695 : un nouveau `train.py` qu'aucun module du jeu
n'importe (tous les autres `.py` identiques, plans inchangés). Traité comme la même version de jeu que s004-s010.
**2026-10-06 18:31 UTC : changement de règles dans le moteur** (fil Replays, à la demande de l'utilisateur) : recycler
une rune prête pour payer de la puissance l'épuise d'abord et son énergie reste disponible jusqu'à la fin du tour
(énergie flottante), pour les deux joueurs ; correction aussi d'un bug où une seule copie d'une carte en double dans la
main donnait des coups légaux. **s015 a été figée avant (18:03) : elle reste dans la version s004-s014. À partir de
s016 : nouvelle version, ne rien cumuler avec avant, remesurer les acquis principaux.**

Chaque session relit ce fichier, joue de nouvelles expériences, puis **réécrit** les sections 1 à 4 (pas
d'ajout en bas) et ajoute une ligne au §5. Seuls les chiffres joués sur des **graines neuves** entrent ici.

## 0. Nouvelle version du moteur (s016+, règles de rune du 6 octobre) — ne pas cumuler avec la suite
s018 tourne sur le moteur refondu du 7 octobre (modules de cartes) : test d'équivalence 400/400 parties identiques à
s017 (deux fois) → même version de jeu pour ce matchup, cumulée avec s016-s017.

| Fait | s016 | s017 | s018 | Cumul nouvelle version | Ancienne version (s004-s015) | Statut |
|---|---|---|---|---|---|---|
| Plan Gorica contre aucun plan | +12,5 ± 3,4 | — | +13,5 ± 3,3 | **+13,0 ± 2,4** | +10,3 ± 1,6 | **acquis** |
| Void Gate au lieu de Sigil face à Windswept | −9,0 ± 3,2 | −9,5 ± 3,1 | — | **−9,3 ± 2,2** | −7,9 ± 2,3 | **acquis** |
| Forgotten Monument au lieu de Sigil face à Windswept | — | — | −9,0 ± 3,2 | −9,0 ± 3,2 | −5,6 ± 1,9 | retrouvé (≥ 2 ET, un bloc) |
| Plan agressif | −5,5 ± 3,3 | −8,8 ± 3,0 | — | **−7,3 ± 2,2** | −5,1 ± 1,5 | **acquis** |
| Windswept présenté par LeBlanc | +2,5 ± 2,3 | +3,2 ± 2,2 | +4,2 ± 2,1 | **+3,4 ± 1,3** | +4,6 ± 1,0 | **acquis** (2,6 ET) |
| Commencer (non apparié) | +5,5 ± 5,0 | +2,5 ± 5,0 | +4,4 ± 5,0 | +4,1 ± 2,9 | +9,0 ± 2,2 | on ne sait pas |
| Niveau de la référence | 47,3 % | 49,5 % | 46,0 % | 47,6 % ± 1,4 | ~48 % | inchangé |
| Plan de LeBlanc (Hook tempo − Deathknell), liste G2 − stock | — | — | — | — | −0,5 ± 2,4 ; +0,4 ± 1,9 | à rejouer en s019 |

Les sections 1 à 3 ci-dessous sont **l'ancienne version** (s004-s015) sauf mention contraire.

## 1. Acquis (≥ 2 écarts-types, rejoué sur graines neuves)
| Fait | Chiffre | Source |
|---|---|---|
| **Le plan Gorica vaut ~10 pts contre aucun plan**, contre les deux plans LeBlanc (IA s004-s015) | Hook tempo : +7,0 / +8,5 / +14,2 → +9,8 ± 1,9 (s007-s009) ; Deathknell +11,8 ± 3,3 (s015) ; cumul **+10,3 ± 1,6** | sessions 007-015 |
| **Jouer agressif coûte ~5 pts**, contre les deux plans LeBlanc (IA s004-s015) | Hook tempo −6,1 ± 2,1 (s007-s008) ; Deathknell −3,0 / −4,8 → −4,0 ± 2,2 (s014-s015) ; cumul **−5,1 ± 1,5** | sessions 007-015 |
| **À battlefield égal, suivre le plan Gorica vaut ~11 pts** (le gain du plan n'est pas le choix du battlefield) | +11,0 ± 3,3 (s009), +10,5 ± 3,1 fixé (s010) ; cumul **+10,7 ± 2,3** | sessions 009-010 |
| **Face à Windswept, avec le plan, Sigil est le bon battlefield** : Void Gate coûte ~8 pts, Forgotten Monument ~6 pts | Void Gate −8,8 (s011, fixé), −7,0 (s012) → **−7,9 ± 2,3** ; FM −1,8 (s011), −9,5 (s012), −5,5 (s013) → **−5,6 ± 1,9** (blocs non sélectionnés) | sessions 010-013 |
| **Windswept présenté par LeBlanc coûte ~4-5 pts à Akali** (IA s004+) | +2,2 / +2,8 / +8,2 / +5,8 / +4,2 sur cinq blocs ; cumul **+4,6 ± 1,0** | sessions 004, 007, 008, 012, 013 |
| Niveau de la référence (stock, plan Gorica, contre Hook tempo Windswept) : **~48 %** ; dispersion entre blocs compatible avec le hasard | 49,8 / 45,5 / 50,8 / 45,0 % (s010-s013), écart-type 2,9 pour 2,5 attendus | sessions 010-013 |
| **Les listes stock, G2, GL et GD se valent à ± 4 pts** contre LeBlanc Hook tempo Windswept (IA s004+) | G2 − stock +0,4 ± 1,9 et GL − stock −0,3 ± 1,9 (1 200 parties appariées, 3 blocs) ; GD − stock +0,2 ± 2,4 ; GL − GD +0,1 ± 2,4 (800) | sessions 004-006 |
| G2 bat Gorica stock **quand aucun joueur ne suit de plan** (ne pas l'étendre au-delà) | +6,6 ± 2,8 pts (640 parties chacun) | retex 2026-10-02 |
| Un bloc de 400 parties peut s'écarter de ~6 pts de la moyenne : un niveau absolu sur un bloc vaut ± 5 | 3 blocs de la même config : 53,3 / 40,0 / 47,3 % ; moteur vérifié déterministe | session 002 |
| Contre LeBlanc Hook tempo qui présente toujours Windswept, G2 + plan Gorica fait ~47 % | 46,8 % sur 1 200 parties (3 blocs), ± 4 vu la dispersion | session 002 |
| **Commencer aide Akali : ~9 pts** (plan Gorica contre Hook tempo Windswept, IA s004+) | +9,0 ± 2,2 sur sept blocs indépendants (s007-s013), de 2 à 16,5 selon le bloc ; 5 à 10 pts au retex du 2 octobre | sessions 007-013 |
| **Le plan de LeBlanc (Hook tempo ou Deathknell) ne change rien pour Akali** à battlefields égaux ; contre Deathknell, Akali fait ~48 % | Hook tempo − Deathknell −0,5 ± 2,4 (s014, fixé) ; 46,2 % (s013), 49,0 % (s014) | sessions 013-014 |
| Le moteur applique les règles des 85 cartes du pool | 91 tests, 0 erreur sur ~11 000 parties | engine/test_cards.py |
| Windswept Hillock présenté par LeBlanc coûte des points à Akali, surtout quand Akali commence | −8,0 ± 3,3 contre Star Spring (G2, plans) après −18 sur un lot indépendant : ampleur 5 à 11 pts | session 001 |
| G2 + plans contre LeBlanc IQ5 (plan Deathknell, qui choisit mal son battlefield) | 59,5 % sur un bloc (± 5 vu la dispersion entre blocs) | session 001 |

## 2. Pistes (mesurées une fois, à confirmer)
| Piste | Chiffre | Source |
|---|---|---|
| Face à Star Spring (Hook tempo, Akali commence), Void Gate (choix du plan) et Sigil se valent | Sigil forcé − Void Gate −1,0 ± 4,7 (s012), −3,1 ± 4,5 (s013) → −2,1 ± 3,3 : garder le choix du plan | sessions 012-013 |
| Sans plan, forcer Sigil n'apporte que +3,2 ± 2,6 ; mais l'IA sans plan présentait déjà Sigil 1 fois sur 3 | 136 FM / 134 Sigil / 130 Void Gate sur 400 (s009) | session 009 |
| Les plans de jeu aident Akali | ~+8 pts (59,5 % avec plans contre 51,2 % sans, blocs de graines différents, non apparié) | session 001, results_plans.json |
| Battlefield d'Akali face à Windswept : Sigil ≥ Forgotten Monument > Void Gate, sans preuve | Sigil − Void Gate +4,0 ± 3,4 ; Sigil − FM +0,3 ± 3,3 | session 002 |
| G2 + plan contre LeBlanc LA comme contre IQ5 | 55,8 % ± 2,5, écart à IQ5 −3,8 ± 3,4 (indiscernable) | session 001 |
| Quand seul LeBlanc suit son plan, Akali perd du terrain | 45,3 % (400) | engine/results_plans.json |

## 3. Réfuté ou sans effet mesurable
- « Couper les Long Sword vaut mieux que couper les Defy » : réfuté par la question fixée à l'avance, GL − GD −6,8 ± 3,3 (s006) après +7,5 (s005, écart sélectionné) ; cumul +0,1 ± 2,4.
- G2 meilleur que stock sous plans : +0,4 ± 1,9 sur 1 200 parties contre Hook tempo (s004-s006), +0,8 ± 3,3 contre Deathknell (s014) ; le +6,6 du 2 octobre ne vaut que sans plans.
- GL (−2 Long Sword, on garde Defy) meilleur que stock ou G2 : **non confirmé** sur bloc neuf (GL − stock −2,0 ± 3,4 face à Windswept, −1,0 ± 3,3 sans ; GL − G2 −2,2 ± 3,4). Le +8,3 de s003 était gonflé par la sélection (session 004).
- Garder Block et Not So Fast « vaut 10 pts » : non soutenu.
- Classement des battlefields d'Akali (Sigil, Void Gate, Forgotten Monument) : indiscernables **sans plan et avec l'ancienne IA** (s002) ; avec le plan et l'IA s004+, Void Gate ressort (voir Pistes).
- (Correction s012) J'avais rangé ici « Forgotten Monument coûte ~10 pts » après le −1,8 de s011 : à tort, le troisième bloc (−9,5) le remet en acquis à ~6 pts. Une piste non confirmée n'est pas réfutée.
- Mulligan agressif, jouer autour de Vi : dans le bruit.

## 4. Questions ouvertes, par priorité
1. Battlefield d'Akali : réglé (Sigil contre Windswept, choix libre sinon). Troisième bloc propre Forgotten Monument et deuxième bloc Star Spring en s013.
1a. Nouvelle version du moteur : plan, agressif, Void Gate, Windswept acquis et Forgotten Monument retrouvé (s016-s018). s019 rejoue le plan de LeBlanc et G2 ; commencer reste ouvert (+4,1 ± 2,9).
1c. Quelles règles du plan portent le gain : à voir avec le fil Replays (gameplans/gameplans.md), le manager ne modifie pas les plans.
1b. Dispersion entre blocs : avec l'IA actuelle elle ne dépasse plus le hasard (4 blocs de référence, s013) ; question mise en sommeil.
2. Voir 1.
3. Résolu par le fil Replays (2026-10-03) : plan « Hook tempo Windswept » ajouté ; Akali plan moteur ~53 ± 2 %
   contre Hook tempo ; plan agressif jamais meilleur (−8 ± 3 face à Windswept) ; avec Sigil, le plan moteur
   ne perd rien face à Windswept (−1 ± 2). Source : gameplans/gameplans.md.
4. G2 contre stock : il manque un lot pour trancher sous plans (accumuler des blocs indépendants).
5. Le niveau absolu du moteur (~45-55 %) contre le ~35 % terrain non vérifié : à recaler sur de vraies parties.
6. Autres matchups (Kennen, Irelia, Master Yi, Jayce) : impossible tant que leurs cartes ne sont pas modélisées.
7. Sideboard réel de LeBlanc : inconnu, la « LeBlanc sidée » du moteur est une invention.

## 5. Historique des sessions
| Session | Date | Questions | Conclusion en une ligne | Retex |
|---|---|---|---|---|
| 001 | 2026-10-03 | plans, G2 vs stock, LeBlanc LA, Windswept | Windswept coûte ~8 pts quand Akali commence ; le 59,5 % avec plans est flatté, ~52 % réaliste | [session_001](sessions/session_001.md) |
| 002 | 2026-10-03 | pire cas Windswept : G2 vs stock, battlefield Akali | ~47 % ± 4 contre LeBlanc qui joue bien ; l'avantage de G2 n'y apparaît pas (stock +3,0 ± 3,3) ; Sigil ≥ Void Gate sans preuve | [session_002](sessions/session_002.md) |
| 003 | 2026-10-03 | IA tempo : G2 vs stock, GL vs GD | G2 vs stock toujours indécis ; piste : la moitié Long Sword compte (GL − GD +8,3 ± 3,3), couper Defy n'apporte rien | [session_003](sessions/session_003.md) |
| 004 | 2026-10-04 | LeBlanc « réel » : confirmer GL | Piste GL non confirmée (GL − stock −2,0 ± 3,4) ; stock, G2 et GL indiscernables sous plans ; Windswept +2 à +3 pts, dans le bruit | [session_004](sessions/session_004.md) |
| 005 | 2026-10-04 | cumul G2/GL vs stock, GD | G2 − stock cumulé −0,3 ± 2,3 ; GL − stock +1,6 ± 2,4 ; piste revenue : GL − GD +7,5 ± 3,4 (couper Long Sword plutôt que Defy) | [session_005](sessions/session_005.md) |
| 006 | 2026-10-04 | GL − GD fixé à l'avance ; cumul | GL − GD −6,8 ± 3,3 : piste réfutée ; acquis : stock, G2, GL, GD à ± 4 pts (G2 − stock +0,4 ± 1,9 sur 1 200) | [session_006](sessions/session_006.md) |
| 007 | 2026-10-04 | plan de jeu d'Akali | Plan Gorica +7,0 ± 3,1 contre aucun plan ; agressif −6,0 ± 3,0 ; Windswept cumulé +2,5 ± 1,6 | [session_007](sessions/session_007.md) |
| 008 | 2026-10-05 | confirmer plan et agressif (fixé) | Acquis : plan +7,7 ± 2,3, agressif −6,1 ± 2,1, Windswept −4,3 ± 1,3 (cumuls) | [session_008](sessions/session_008.md) |
| 009 | 2026-10-05 | d'où vient le gain du plan (fixé : Sigil sans plan) | Sigil seul +3,2 ± 2,6 ; à Sigil égal le plan vaut +11,0 ± 3,3 ; plan cumulé +9,8 ± 1,9 | [session_009](sessions/session_009.md) |
| 010 | 2026-10-05 | plan à battlefield égal ; battlefield avec le plan | Acquis : plan +10,7 ± 2,3 à Sigil égal. Nouveau : avec le plan, Forgotten Monument coûte 10,2 ± 3,4 | [session_010](sessions/session_010.md) |
| 011 | 2026-10-05 | Sigil − Void Gate (fixé) ; rejouer Sigil − Forgotten Monument | Void Gate coûte ~9 pts face à Windswept (−8,8 ± 3,3, piste forte) ; la piste Forgotten Monument retombe (−1,8 ± 3,3) | [session_011](sessions/session_011.md) |
| 012 | 2026-10-06 | confirmer Void Gate (fixé) ; FM ; Sigil face à Star Spring (fixé) | Acquis : face à Windswept, Void Gate −7,9 ± 2,3 et Forgotten Monument −5,7 ± 2,3 contre Sigil ; face à Star Spring, pas d'écart (−1,0 ± 4,7) | [session_012](sessions/session_012.md) |
| 013 | 2026-10-06 | FM 3e bloc ; Star Spring 2e bloc ; LeBlanc Deathknell | FM −5,6 ± 1,9 (trois blocs) ; Star Spring : garder Void Gate (−2,1 ± 3,3) ; contre Deathknell 46,2 %, même niveau ; dispersion entre blocs normale | [session_013](sessions/session_013.md) |
| 014 | 2026-10-06 | Deathknell : plan LeBlanc, side G2, agressif | Trois « pas d'écart » : Hook tempo ≈ Deathknell (−0,5 ± 2,4), G2 ≈ stock (+0,8 ± 3,3), agressif −3,0 ± 3,2 ; commencer vaut ~9 pts (sept blocs) | [session_014](sessions/session_014.md) |
| 015 | 2026-10-06 | Deathknell : agressif (2e bloc), valeur du plan | Plan +11,8 ± 3,3 contre Deathknell (cumul deux plans +10,3 ± 1,6) ; agressif −4,8 ± 3,0 (cumul deux plans −5,1 ± 1,5). Dernière session de l'ancienne version du moteur | [session_015](sessions/session_015.md) |
| 016 | 2026-10-07 | NOUVEAU MOTEUR : remesure des acquis | Plan +12,5 ± 3,4, Void Gate −9,0 ± 3,2, agressif −5,5 ± 3,3 : les acquis tiennent ; Windswept et commencer plus petits, à confirmer. Reprise après redémarrage du conteneur | [session_016](sessions/session_016.md) |
| 017 | 2026-10-07 | nouvelle version, 2e bloc | Acquis dans la nouvelle version : agressif −7,3 ± 2,2, Void Gate −9,3 ± 2,2 (cumuls s016-s017) ; Windswept +2,9 ± 1,6 et commencer +4,0 ± 3,5 : on ne sait pas encore | [session_017](sessions/session_017.md) |
| 018 | 2026-10-07 | nouvelle version, 3e bloc (moteur refondu, équivalence vérifiée) | Acquis nouvelle version : plan +13,0 ± 2,4, Windswept +3,4 ± 1,3 ; Forgotten Monument −9,0 ± 3,2 retrouvé ; commencer +4,1 ± 2,9 on ne sait pas | [session_018](sessions/session_018.md) |
