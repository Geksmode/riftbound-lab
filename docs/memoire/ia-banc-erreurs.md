---
name: ia-banc-erreurs
description: Banc d'erreurs de cible et de timing de l'IA (2026-10-28) : juge à deux jeux de poids, 96 parties turn sur decks variés, 3,4 % ± 0,4 de choix nettement mauvais, 11 % de sorts sans effet visible ; outil engine/banc/ ; ask_sim rejeté (49,3 % ± 1,1), spell_pre « on ne sait pas » (50,8 % ± 0,7) ; coût moyen d'une erreur en victoires −0,8 ± 2,4 points (banc/cout.py)
metadata:
  type: project
---
**Demande** (2026-10-28) : l'utilisateur voit l'IA « faire encore beaucoup d'erreurs de séquence », jouer des cartes « au
mauvais timing ou sur la mauvaise cible » ; « oui, le banc » (monter un banc d'erreurs avant de corriger).

**Pourquoi pas tes parties** : les 17 parties de `train/games/` ont été jouées avec les moteurs figés v10 à v12 ; leurs
positions ne se retrouvent pas avec le moteur actuel. Le banc vient de parties IA contre IA (moteur actuel).

**Outil** (`riftbound/engine/banc/`, hors de `engine/*.py` : ne change pas `engine_md5`) :
- `banc.py gen <parties> <graine0> defaut [turn] [processus]` : parties comme `exp_search` (setup, graines d'agents) ;
  `turn` = IA Normal de la table. À chaque choix, le juge rejoue les autres options sur une copie : variantes de la même
  carte (toutes, plafonds levés, même emplacement base/battlefield) et choix pendant la résolution (`Game.ask`).
  Note = `ai.evaluate` après résolution (les deux joueurs passent jusqu'au prochain « main », combat compris).
- **Deux jeux de poids** (EV Texel et EV_OLD) ; erreur = une autre option meilleure de 2 ou plus pour LES DEUX.
  Avec EV seul, 20,5 % ± 1,4 « d'erreurs », dont 150 paiements par rune au lieu du Gold et toutes les Accelerate :
  rune 6,6 et on_bf −4,6 sont des poids corrélés ([[ia-texel]]), pas des valeurs à lire seules.
- Sorts : effet = note après résolution − note si le sort était contré (mêmes coûts, mêmes déclencheurs). « Sans effet »
  = ≤ 0,25 pour les deux poids, hors « avant attaque » (cible, ou unité vers son battlefield, qui attaque plus tard
  dans le tour). Candidat au mauvais timing, pas erreur sûre : le juge ne compare pas avec « garder la carte ».
- `banc.py rejuge <parties.jsonl>` : rejuge d'après les scripts enregistrés (sans recherche, ≈ 1 s par partie).
  `banc.py rejoue <pos.jsonl> <version>` : mêmes positions, choix d'une autre version, erreurs appariées par position.
- `banc/test_banc.py` (5 tests, ≈ 1 min) : Hextech Ray sur la mauvaise cible, Cleave sur une unité adverse, le juge ne
  change pas la partie (script identique avec et sans juge), rejeu fidèle. Pas dans `verifier.sh` (comme `texel/`).
- Limites : juge omniscient (voit les mains), horizon = la résolution ; il ne voit pas les erreurs de long terme ni
  l'ordre des coups sur un tour entier.

**Mesure** (parties jouées avec le moteur a832cbbc81, rejugées à l'identique avec ac320dd88e, 96 parties `turn` contre `turn`, graines 4100000-4100095, decks par défaut,
écart-type par grappes de parties) : 2 618 positions jugées, 2 121 avec un vrai choix.
- Choix nettement mauvais : **3,4 % ± 0,4** (71) ; cibles des cartes jouées 6,0 % ± 1,1 (31/518), choix pendant la
  résolution 2,5 % ± 0,4 (40/1 603). Soit ≈ 0,7 par partie (deux camps).
- Sorts sans effet visible : **11,1 % ± 1,0** des sorts joués (104/935, sans les 83 « avant attaque ») ; 54 en phase
  principale, 46 sans aucune cible. Cartes : Bellows Breath 14, Flurry of Blades 13, Eclipse 7, Punch First 6, Defy 6.
- Cibles : la meilleure était dans la liste de l'IA 30 fois sur 31 (les plafonds MAX_CHOICES ne sont pas la cause) ;
  la recherche à 1 tirage choisit mal parmi des options qu'elle voit.
- Résolution : `may` 13 (Irresistible Faefolk 6 : Heuristics répond toujours « oui », Faefolk tire une grosse unité
  adverse et meurt), `promising_pick` 8 (première carte prise), puis predict_recycle, pay_gold, target (Harnessed Dragon,
  Kennen), fizz_pick.
- **Rejeu des mêmes positions par la même IA** (`rejoue ... turn`, agent neuf à chaque partie : plan et suite de tour
  repartent de zéro, pas seulement l'aléa) : coup différent **429 fois sur 1 015** (42 %), dont 388 avec une autre carte
  ou rien ; parmi les 627 où elle rejoue la même carte, 19 erreurs au lieu de 31 (6 corrigées, 4 nouvelles). Sorts :
  368 fois sur 935 elle ne rejoue pas ce sort à ce moment ; 50 « sans effet » au lieu de 104. Asks : 40 erreurs sur 40,
  identiques (règles fixes de `Heuristics.choose`, sans hasard).
- Lecture : erreurs de résolution = **systématiques** (règles fixes, à corriger carte par carte ou par simulation) ;
  cibles et timing des sorts = surtout **le bruit** de la recherche à 1 tirage (le même choix ne revient que 6 fois sur
  10), donc à corriger par plus de tirages ou une politique de simulation meilleure, pas par une règle.

**Essai 1 : choix de résolution par simulation** (`ai.ask_sim`, `RB_ASK_SIM` / cfg `ask_sim`, 2026-10-28, demande de
l'utilisateur « oui, la résolution »). Point de reprise dans `Game.apply` (`checkpoint`, `_ckpt`, `_asklog`, jamais copiés) ;
au choix en cours, l'IA rejoue le coup avec chaque option (≤ 12, sauf paiements) dans un monde tiré (main adverse au
hasard, son deck dans l'ordre), les deux passent jusqu'au « main », note `value` ; égalité : la règle. Garde-fous : même
signature d'options (uid) au rejeu, carte jouée par l'adversaire gardée en main (`keep_played`, bug trouvé par le
premier SPRT, graine 5000214, run arrêté), sinon la règle. Tests `cardsets/test_ask_sim.py` (6) ; empreinte identique
drapeau coupé ; 1 347 tests verts.
- Banc (mêmes positions, `rejoue ... "turn@ask_sim=1" 24 ask`) : erreurs de résolution **40 -> 8** (32 corrigées, 0
  nouvelle, 178 choix changés). En partie construit : même méthode que le juge (une note après résolution).
- **Règle 9 : rejeté** (`mesure/sprt.py 5100000 "sh@ask_sim=1" sh defaut`, moteur ac320dd88e) : 648 paires, **49,3 % ± 1,1**,
  LLR −3,01, 28 min. Pas de gain de +2 points ; pas de perte démontrée non plus (z −0,65). 32 erreurs corrigées sur
  96 parties ≈ 0,17 par joueur et par partie : trop peu pour bouger le score, ou la note immédiate juge mal ces choix
  à long terme (on ne sait pas lequel). Défaut inchangé (coupé). Temps par décision non mesuré.
- Reste possible sur décision de l'utilisateur (comme les sorts de combat du 2026-10-10) : l'allumer pour la table
  parce qu'il retire des erreurs visibles, sans gain de victoires démontré.

**Essai 2 : pré-sélection des cibles de sorts** (`ai.spell_prefilter`, `RB_SPELL_PRE` / cfg `spell_pre`, 2026-10-28,
« cibles des sorts »). Avant la recherche, chaque variante d'un même sort est résolue sur une copie (un monde tiré, les
deux passent) ; on garde la meilleure pour EV et pour EV_OLD ; en showdown, un sort sans effet est retiré. Aussi aux
coups suivants de la recherche du tour entier. Tests `cardsets/test_spell_pre.py` (3).
- Banc (`rejoue ... "turn@spell_pre=1" 24 play`) : erreurs de cible **0** sur 554 positions où la même carte est rejouée,
  contre 19 sur 627 pour l'IA actuelle rejouée ; sorts sans effet 35 contre 50. En partie construit (même méthode que le juge).
- **Règle 9 : on ne sait pas** (`sprt.py 5200000 "sh@spell_pre=1" sh defaut`, 1 600 paires, plafond atteint) :
  **50,8 % ± 0,7** (z ≈ 1,1). Défaut inchangé (coupé). Temps : le run a duré 254 min, mais l'horloge a sauté pendant
  (40 paires en 146 min, machine en veille probable) : temps par décision non mesuré.

**Coût d'une erreur en victoires** (`banc/cout.py`, 2026-10-28, demande de l'utilisateur : « ces erreurs ont
BEAUCOUP d'impact »). Chacune des 71 erreurs rejouée jusqu'au bout 16 fois avec le choix de l'IA et 16 fois avec la
meilleure option du juge (paires appariées : decks remélangés, aléa et graines d'agents identiques), suite jouée par `sh`
des deux côtés, vraie position. 1 114 paires sur 1 136 jouées (22 écartées : le remélange change un choix d'avant, ex.
Elder Dragon, graine 4100036). Résultats (`results_banc.json`, moteur 93e1e7acc0, 8 468 s dont ≈ 1 h 30 de veille du PC) :
- **Coût moyen d'une erreur : −0,8 ± 2,4 points de victoire** (tout) ; cibles des sorts +2,0 ± 3,4 ; choix de
  résolution −3,0 ± 3,4. Sous 2 écarts-types : **on ne sait pas** ; au-dessus de +4 points par erreur, c'est exclu (2 é.-t.).
- Quelques erreurs coûtent beaucoup : Punch First qui laissait passer une victoire (+69 points), Wages of Pain (+38),
  Falling Star (+31), Elder Dragon (+27), Harnessed Dragon (+20). Mais 7 « erreurs » sur 71 font mieux que la « bonne »
  option de 25 points ou plus : **Irresistible Faefolk qui tire l'unité adverse gagne plus souvent** (0,88 contre 0,25 ;
  0,75 contre 1,00 ailleurs), fizz_pick −0,88. 42 sur 71 ont un coût entre −10 et +10 points (16 tirages : ± ≈ 12 par position).
- Lecture : la note juste après la résolution (le juge, donc aussi ask_sim et spell_pre) se trompe souvent sur ce qui fait
  gagner : c'est pourquoi les deux correctifs ne montrent pas de gain. Limite : suite IA contre IA ; un humain qui punit
  les erreurs n'est pas modélisé.
- Prochaine étape logique : un banc v2 où une erreur est **confirmée par les parties jouées jusqu'au bout** (coût > 0 à
  2 écarts-types, plus de tirages), et corriger seulement celles-là.

**Banc humain** (2026-10-28, l'utilisateur : « il faut que le simulateur se comporte comme un humain » ; « oui, le
bouton ») : les erreurs que **le joueur** marque sur la table remplacent la note immédiate comme juge. Outils
`train.flag` / `train.ai_script` / `train.ScriptAI`, bouton de la table, `banc/humain.py` (détail :
`docs/wiki/pages/parties.md`). Aucune partie marquée pour l'instant (une partie d'essai). Règle « réalisme » (garder un
réglage qui corrige des erreurs marquées s'il est démontré qu'il ne fait pas perdre : test de non-infériorité) proposée,
pas encore décidée par l'utilisateur.

**Premières parties marquées** (2026-10-28, artefact, niveau Très fort, 7 parties Akali contre LeBlanc et Master Yi,
14 erreurs marquées, toutes dans `riftbound/train/games/games/mvs*.json`). Deux bugs trouvés et corrigés en route : le
script de l'IA d'une manche de BO3 était écrasé par celui de la manche suivante (6 parties sur 7 sans coups de l'IA) ;
« Reprendre » effaçait des coups de l'IA que le rejeu rejoue (3 parties non rejouables) → journal en temps réel
`W["ailog"]`. Coups de l'IA recalculés avec le moteur exact (d014de4a0a), 7 rejeux fidèles.
- Banc humain (`humain.py --ver table --ver "table@spell_pre=1" --temoins 4`, 14 marques, 24 témoins) : IA de la table
  change 6/14 marques (43 % [21 ; 67]) et 9/24 témoins (38 % [21 ; 57]) : bruit. Avec `spell_pre` : 9/14 marques
  (64 % [39 ; 84]) contre 6/24 témoins (25 % [12 ; 45]) ; corrige Charm qui envoie Akali conquérir Seat of Power
  (la recherche notait −53,8 contre −62,8 pour la renvoyer en base ; la note immédiate dit l'inverse) et En Garde en
  showdown vide. Ne corrige pas : unités [Ambush] (Vi Peacekeeper, Rengar Trophy Hunter) jouées dans son tour ou en
  base au lieu d'être gardées, Rengar envoyé mourir, Karthus posé sur un battlefield, déplacements.
- Limite de la marque : elle vise le dernier vrai choix de l'IA ; une remarque sur un coup plus ancien (Rengar en base,
  marqué sur Sabotage) tombe sur le mauvais coup.

**Essai 3 : unités [Ambush] et réaction sans préférence** (2026-10-28, « corriger les Ambush », mesure demandée :
« sur mes marques et par un test de non-infériorité ») — **allumé pour la table** (`train.TABLE_CFG`, tous niveaux sauf
le tutoriel ; coupé par défaut ailleurs : `RB_AMBUSH`, `RB_TIE_REACT`).
- `ambush = f` : la politique de simulation garde une unité [Ambush] tant qu'elle peut servir (unité sur un
  battlefield ; Rengar, Trophy Hunter : battlefield avec ennemis), et la feuille la compte pour f × sa valeur posée
  (`amb_hold`, hors de `ai.EV`). La politique seule ne changeait aucun coup marqué. f = 0,5.
- `tie_react` : réaction dont toutes les options ont la même note → la politique (`react_best`). Cause trouvée au
  « suicide du Rengar » : la recherche du tour avait prévu Rengar + Punch First (11 contre 9) puis Emperor's Dais pour
  gagner ; au combat, la recherche de la réaction notait tout −9550 (sa politique n'attaque pas un 5 contre 5) et
  passait.
- Positions marquées (IA de la table, Très fort, 3 tirages chacune) : Rengar en base T5 gardé 3/3 (avant 1/3) ;
  combat de Rengar T17 : Punch First 3/3 (avant 0/3) ; Vi, Peacekeeper T5 : inchangé 0/3.
- Non-infériorité (`RB_SPRT_H0=0.485 RB_SPRT_H1=0.50 mesure/sprt.py`, `sh@ambush=0.5,tie_react=1` contre `sh`, decks
  variés) : gardé après 688 paires (5300000-5300687, 50,6 % ± 0,8), confirmé après 1 448 paires (5400000-5401447,
  49,9 % ± 0,6). Mesuré en `sh`, pas en `turn` (table) ; pas de gain de victoires démontré. Temps par décision non mesuré.
- Reste à faire : Vi posée pendant son showdown (garde Windswept Hillock, la recherche y voit les points de tenue) ;
  la politique de simulation n'attaque pas quand l'égalité de Might gagne la partie.

**Pistes** (non faites, chacune à mesurer sur ce banc puis par la règle 9) : asks par simulation courte (le juge lui-même, au lieu de options[0]) ; politique de simulation
qui vise avec une note immédiate ; chaque réglage gardé seulement par la règle 9 ([[ia-sprt]]).

Related: [[riftbound-search-ai]], [[ia-texel]], [[ia-tests-decks-varies]].
