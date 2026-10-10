---
titre: Tutoriel « Apprendre à jouer »
resume: le mode tutoriel de la table (néophytes) : déroulé, decks, IA débutante, défis, code et vérification
maj: 2026-10-10
sources: riftbound/train/src.html (tutoIntro, tutoStart, tutoCheck), riftbound/engine/train.py (Gentle, tuto), riftbound/train/verif_tuto.mjs
---
Demande de l'utilisateur (2026-10-10) : un mode tutoriel pour expliquer Riftbound à un néophyte, **pas trop long et ludique**.

**Déroulé** (menu → « Apprendre à jouer ») :
1. Quatre écrans d'intro (8 points pour gagner, les runes, unités et combats, les 6 défis), « Passer l'intro » possible.
2. Une **vraie partie**, moteur complet (aucune règle simplifiée) : seuls la donne et l'adversaire sont choisis.
3. Six défis dans l'ordre, une étoile chacun : garder sa main, jouer une unité, terminer son tour, aller sur un champ de
   bataille, marquer un point, gagner un combat (attaque ou défense, événement moteur `combat_won`, règle 467).
4. Un coach (carte en haut du plateau, « Compris » la replie ; la puce « Tuto k/6 » de la barre du haut la rouvre)
   explique chaque défi, et donne une fois chacune quatre astuces de situation : fenêtre de réaction, question d'une
   carte, Darius sur un champ, Darius marque.
5. Écran de fin (6 défis ou fin de partie) : continuer la partie, recommencer le tuto, partie contre l'IA.

**Decks et donne** : Maître Yi Wuju Bladesman (`yi-bladesman_swagalisk_rq-la_6th`, choix de l'utilisateur : facile à jouer,
montre les bases ; sa légende donne +2 might à une unité qui défend seule) contre Darius (`darius_mice-diamondhat_rq-utrecht_6th`).
Donne 202, tu commences, main First Mate, Pit Rookie ×2, Punch First (trouvée par `train.tuto_seeds()`).

**IA débutante** (`train.Gentle`, `level=0`) : la moitié de ses décisions au hasard parmi les coups légaux (comme
`agents.RandomAgent`), l'autre moitié par l'IA normale en recherche « sh » ; mulligan et choix en cours de résolution par
l'IA normale. Son hasard a sa propre graine, copiée avec l'état : « Reprendre » et le rejeu restent exacts. Sa force n'est
**pas mesurée** (aucun chiffre à citer) ; 4 parties où « toi » jouait au hasard : Darius gagne les 4, ce qui ne dit rien
d'un humain.

La partie du tutoriel n'est pas enregistrée dans « Mes parties ».

**Vérification** : test `tutorial_game_is_fixed_and_playable` (main annoncée, partie complète, même suite de coups = même
partie). Robot `verif_tuto.mjs` (dans `scripts/verifier.sh`, étape cache) en 1400x900 et 393x851 (toucher simulé) : intro,
« Garder ma main », coach et puce, défis 1 à 6, astuce, écran de fin, recommencer, puis partie normale sans coach ;
coach et puce mesurés dans l'écran. Les coups (jouer, déplacer, puis conseil de l'IA) passent par `choose(i)`, pas par
le glisser. **Pas essayé sur un vrai téléphone ni par un humain néophyte.**

Piège rencontré : la classe d'animation `pop` de la puce reprenait le style `.pop` (menus de carte, `position: fixed`) ;
renommée `tbump`.
