---
name: ia-niveaux
description: Table, 10 niveaux de l'IA (2026-11-24, demande de l'utilisateur « comme sur Smash », 1 le plus faible, 10 très dur) : réglages (`ai::LEVEL_TABLE`), marches mesurées avec intervalles, temps par décision, compatibilité des anciennes parties
metadata:
  type: project
---
**Demande** (2026-11-24) : « fais-moi 10 niveaux différents comme sur Smash, 1 c'est le plus faible et 10 c'est vraiment très dur ».

**Réglages** : une seule table, `rust/src/ai.rs` (`LEVEL_TABLE`, `level_opts(niveau)`), lue par la table (`train::new`), les
outils (`exp_search` et `sprt` : version `niv1` … `niv10`, sans plan de légende) et le banc des erreurs marquées. Tous les
niveaux cherchent le tour entier (`turn`) avec les réglages de réalisme de la table (`ambush=0.5`, `tie_react=1`, règle 10).

| Niveau | Nom | Coups au hasard (`wild`) | Mondes | Faisceau (`turn_w`) | Mondes hors « main » (`react_n`) |
|---|---|---|---|---|---|
| 1 | Débutant | 70 % | 1 | 3 | 1 |
| 2 | Novice | 50 % | 1 | 3 | 1 |
| 3 | Facile | 35 % | 1 | 3 | 1 |
| 4 | Apprenti | 20 % | 1 | 3 | 1 |
| 5 | Moyen | 8 % | 1 | 3 | 1 |
| 6 | Normal (ancien « Normal ») | 0 | 1 | 3 | 1 |
| 7 | Fort (ancien « Fort ») | 0 | 3 | 3 | 3 |
| 8 | Très fort (ancien « Très fort ») | 0 | 6 | 6 | 3 |
| 9 | Expert | 0 | 12 | 8 | 6 |
| 10 | Maître | 0 | 24 | 12 | 12 |

- Niveaux 1 à 5 : l'IA du niveau 6 qui, avec la probabilité `wild`, prend un coup au hasard parmi les coups légaux
  (`SearchAgent::wild_move`, même procédé que l'IA du tutoriel `train::Gentle` : rarement « finir le tour » s'il y a autre
  chose, souvent « passer » en réaction). Ce sont des erreurs au hasard, pas des erreurs « humaines ».
- Niveaux 7 à 10 : seulement plus de calcul (aucune connaissance en plus). Le niveau 0 reste l'IA du tutoriel.

**Mesures** (decks variés `defaut`, IA sans plan de légende, paires appariées, moteur `07aa82b4bd-rust` ; chaque ligne sur ses
propres graines ; 80 paires = coup d'œil, règle 8).

| Marche | Paires | Graines | Score du niveau du haut |
|---|---|---|---|
| 2 contre 1 | 80 | 11001000+ | **61,3 % ± 3,2** |
| 3 contre 2 | 80 | 11002000+ | **61,3 % ± 3,7** |
| 4 contre 3 | 80 | 11003000+ | **64,4 % ± 2,7** |
| 5 contre 4 | 80 | 11004000+ | **56,9 % ± 3,2** |
| 6 contre 5 | 80 | 11005000+ | **57,5 % ± 3,0** |
| 7 contre 6 | 80 | 11006000+ | **58,1 % ± 3,1** |
| 8 contre 7 | 80 | 11007000+ | 51,9 % ± 2,7 : on ne sait pas |
| 9 contre 8 (test séquentiel 1) | 224 | 10000000+ | **gardé, 55,6 % ± 1,8** (LLR 2,96) |
| 9 contre 8 (test séquentiel 2) | 120, coupé | 12000000+ | ne compte pas : coupé par un redémarrage de la machine cloud à 120 paires (54,2 %, non tranché), rien d'écrit dans `results_search.json` |
| 10 contre 9 | 80 | 11009000+ | 52,5 % ± 2,9 : on ne sait pas |

Un test séquentiel s'arrête quand il franchit sa borne : sa moyenne surestime un peu le vrai gain. Les marches ne
s'additionnent pas (aucun niveau n'a été mesuré contre un niveau éloigné). Le test séquentiel 1 du niveau 9 a été lancé avec
les réglages écrits en toutes lettres (`turn@n=12,turn_w=8,react_n=6,ambush=0.5,tie_react=1` contre
`turn@n=6,turn_w=6,react_n=3,ambush=0.5,tie_react=1`), identiques à `niv9` et `niv8`. 8 contre 7 et 10 contre 9 : pas de test séquentiel fini
(10 contre 9 : 80 paires = 1 h sur 4 cœurs).

**Confirmations sur graines neuves, moteur `151f55e0f4-rust`** (après la correction de Patched Porobot : lignes séparées,
jamais regroupées avec celles du moteur `07aa82b4bd-rust`, règle 2 ; tests séquentiels `sprt`, H0 50 %, H1 52 %, 5 % / 5 %,
4 processus, decks variés `defaut`).

| Marche | Verdict | Paires | Graines | Score du niveau du haut | LLR (bornes ± 2,94) | Moteur |
|---|---|---|---|---|---|---|
| 9 contre 8 (confirmation) | **GARDE** | 520 | 14000000-14000519 | **53,3 % ± 1,2** | +3,00 | `151f55e0f4-rust` |
| 8 contre 7 (test séquentiel 1) | **GARDE** | 568 | 15000000-15000567 | **52,8 % ± 1,1** | +3,04 | `151f55e0f4-rust` |
| 8 contre 7 (test séquentiel 2, confirmation) | en cours | | 16000000+ | | | `151f55e0f4-rust` |

**Marche 9 contre 8 : démontrée selon la règle 9** (deux tests séquentiels « garde », sur des graines différentes, un par
version du moteur). Le gain est plus petit que ce que disait le premier test : 53,3 % ± 1,2 ici contre 55,6 % ± 1,8 (un test
séquentiel qui s'arrête sur sa borne surestime ; le vrai gain est sans doute de l'ordre de +2 à +5 points).

**Temps par décision** (`exp_temps 777003,777011 niv6 niv8 niv9 niv10`, moteur natif, un cœur d'une machine cloud à
4 cœurs, 137 positions ; « main » = préparation du tour, les coups suivants de la suite choisie sont immédiats) :
niveau 8 : 0,63 s en moyenne (p95 3,2 s) ; niveau 9 : 1,6 s (p95 7,8 s, max 21 s) ; niveau 10 : 4,2 s (p95 20 s, max 64 s).
Dans le navigateur (WebAssembly) : **non mesuré** ; compter plus lent que le natif.

**Table** : dix boutons 1 à 10 dans le panneau de l'IA (ordinateur) ; sur téléphone (≤ 820 px) « − » et « + » autour de
« Niv. 6 Normal », parce que dix boutons ne tiennent pas sur une rangée et que deux rangées poussaient « Commencer » hors de
l'écran. Choix gardé dans le navigateur (`rbt-niv`, défaut 6 ; l'ancien `rbt-lvl` 1 / 3 / 6 est repris comme 6 / 7 / 8).
Bandeau de la partie et « Mes parties » : « IA niveau N ». Robot `train/verif_vs.mjs` (1400×900 souris et 393×851 toucher
simulé) : vérifié par robot, pas essayé sur un vrai téléphone.

**Anciennes parties** : une partie enregistrée porte `level` et, depuis ce changement, `lvs: 10`. Sans `lvs`, `level` est
l'ancien nombre de mondes (1, 3, 6) et se lit comme niveau 6, 7, 8 (`ai::level_from_saved`, `train::saved_level` côté moteur,
`lvlOf` côté page) : mêmes réglages qu'avant. Le rejeu suit de toute façon les coups enregistrés de l'IA (`ScriptAI`).

**Ce qui reste inconnu** : la force face à un humain (tout est mesuré IA contre IA) ; l'effet avec les plans de légende de la
table (Akali, LeBlanc) ; le temps réel dans un navigateur et sur téléphone ; la marche 10 contre 9.

Related: [[riftbound-search-ai]], [[ia-sprt]], [[riftbound-training-table]].
