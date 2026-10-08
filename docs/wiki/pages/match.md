---
titre: Match BO1 / BO3 et sideboard
resume: déroulé d'un match fidèle aux règles (tirage, premier joueur, battlefields, sideboard) et où c'est codé
maj: 2026-10-08
sources: riftbound/engine/match.py, riftbound/engine/train.py (match_*), Core Rules 485-486, règles de tournoi 403, 407, 601.1.c
---
**Règles appliquées** (texte officiel du dépôt, `riftbound/rules/source/`)
- BO1 (Core Rules 485) : chaque joueur tire **au hasard** un de ses 3 battlefields (485.5) ; une manche.
- BO3 (486) : à chaque manche, chaque joueur **choisit** un de ses battlefields, en même temps que l'autre (486.5) ;
  après une manche gagnée, les deux battlefields joués sont retirés pour le match ; après une nulle, ils peuvent resservir (486.5.a) ;
  2 manches gagnées (486.6).
- Premier joueur (tournoi 407) : manche 1, le gagnant d'un tirage au sort choisit de jouer en premier ou en second (407.1-407.2) ;
  ensuite le **perdant** de la manche précédente choisit (407.4) ; après une nulle, même premier joueur.
- Sideboard (403, 601.1.c) : 10 cartes au plus, cartes valides pour le Main Deck, limite de 3 exemplaires sur deck + sideboard ;
  interdit en manche 1 (403.5) et après une nulle (403.10) ; échange 1 pour 1 (403.4) ; le Chosen Champion peut changer (601.1.c.4) ;
  runes, légende et battlefields fixes (403.4.b) ; retour au deck enregistré au match suivant (403.8).
- Écart avec la description de l'utilisateur (2026-10-08) : il pensait que le perdant choisit le battlefield ; d'après 486.5
  chaque joueur choisit le sien, le perdant ne choisit que le premier joueur. Règle appliquée : le texte officiel (règle 6 de `CLAUDE.md`).

**Code** : `engine/match.py` (fonctions pures sur un état JSON), API `train.match_new / match_next / match_record / match_swap`,
`train.new(..., obf=)` pour le battlefield de l'IA ; validation du sideboard dans `train.validate`. Tests `cardsets/test_match.py` (8).
**Choix de l'IA** : jouer en premier quand elle choisit ; battlefield par son plan, sinon au hasard, sans connaître le tien ; pas de sideboard.
**Interface** : en cours (2026-10-08, agent design) : section Sideboard de l'éditeur, mode BO1 / BO3 dans « Nouvelle partie ».
