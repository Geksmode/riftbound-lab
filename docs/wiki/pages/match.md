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
**Interface** (2026-10-08) : section « Sideboard N/10 » de l'éditeur (interrupteur « Ajouter au : Deck / Sideboard », ⇄ entre deck et sideboard,
import / export) ; « Nouvelle partie » : mode BO1 / BO3, tirage au sort, premier ou second, battlefield parmi ceux permis, écran de sideboard
entre les manches (1 pour 1, champion), score « Manche N · toi X – Y IA ». Chaque manche enregistre `obf` et `match {id, mode, game, seed}` ;
`train_games.py` passe `obf` au rejeu (test discriminant `match_game_replays_faithfully_with_ai_battlefield`).
**Vérifié** par `train/verif_match.mjs` (79 contrôles, vrais clics, 360×740 et 1400×900, dans la porte) : éditeur et sideboard, manche 1 d'un BO3
jouée jusqu'au bout (coups injectés), sideboard, battlefield joué retiré, manche 2 lancée. **Pas testé** : manche 1 gagnée par le joueur,
manche nulle, fin de match, changement de champion au sideboard, « Reprendre » après une fin de manche, base claude.ai réelle.
**Écran « Contre l'IA »** (2026-10-09, à la Smash Bros) : grille des 49 légendes + « ? » (hasard) ; panneaux « Toi » (menthe) et
« IA » (corail), on touche un panneau puis une légende. Chaque légende prend son **deck par défaut** (`train.default_decks` : parmi les
listes de `decks.json` jouables par le moteur, celles avec sideboard d'abord, puis le meilleur classement), le même en BO1 et en BO3.
Une légende sans liste jouable est grisée (« bientôt »). Au 2026-10-09 seules Akali (dongdong, Wuhan Open 5e) et LeBlanc (GYATarina,
CCS IQ#5 1re) en ont une. L'IA **ne sideboarde pas encore** : son sideboard est chargé mais pas utilisé. Robot `train/verif_vs.mjs`
(18 contrôles, 1400×900 et 393×851, vrais clics / toucher simulé) dans la porte ; test `default_deck_per_legend_for_the_vs_screen`.
**Changement** : en BO1 la graine d'une manche vaut donne × 10 + 1 : une « Donne N » ne redonne plus la partie d'avant ce changement.
