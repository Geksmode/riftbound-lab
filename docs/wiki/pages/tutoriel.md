---
titre: Tutoriel « Apprendre à jouer »
resume: le mode tutoriel de la table (débutants complets) : intro, leçons et défis pas à pas, lexique, IA débutante, code et vérification
maj: 2026-10-17
sources: riftbound/train/src.html (GLOSS, TSTEPS, TTIPS, TINTRO, tutoCheck, tutoDraw), riftbound/engine/train.py (Gentle, tuto), riftbound/train/verif_tuto.mjs
---
Demandes de l'utilisateur : un mode tutoriel pour un néophyte, pas trop long et ludique (2026-10-10) ; puis « plus
progressif, qui explique vraiment pour les débutants qui ne connaissent pas du tout le jeu, des tooltips pour les
mots-clés, accompagner pas à pas » (2026-10-17) ; « expliquer le focus et la priorité, pourquoi il faut passer, et les
cartes Action et Reaction » (2026-10-17).

**Déroulé** (menu → « Apprendre à jouer ») :
1. **Neuf écrans d'intro** (« Passer l'intro » possible) : ce qu'est Riftbound, le but (8 points), trois sortes de cartes
   (vraies images d'une unité, d'un sort, d'un équipement), les runes, l'ordre d'un tour (redresser, tenir, 2 runes,
   piocher, jouer, terminer : 315), les combats, Action et Reaction (vraies cartes Punch First et Discipline),
   « chacun son tour de parole » (priorité 312, 338-339 ; focus 313, 345-347 ; passer n'est pas perdre son tour),
   puis la liste des 6 défis. Lien « Lexique » sur chaque écran.
2. Une **vraie partie**, moteur complet (aucune règle simplifiée) : seuls la donne et l'adversaire sont choisis.
3. **18 étapes dans l'ordre** : 12 leçons et 6 défis à étoile. Une **leçon** surligne une partie de la table (contour doré
   qui pulse, classe `tfocus`) et l'explique ; « Suivant » passe à la suite. Certaines attendent leur moment (`when`) :
   pendant le tour de Darius, le coach dit d'attendre et de regarder ses coups. Un **défi** est une chose à faire
   (garder sa main, jouer une unité, terminer son tour, aller sur un champ, marquer un point, gagner un combat :
   événement moteur `combat_won`, 467) ; ce qu'il faut toucher est surligné.
   Leçons : la table, lire une carte (mulligan), ton camp, légende et champion, les runes, unité épuisée, nouveau tour
   (après le tour de Darius), les champs, le showdown et le focus (au premier focus : pourquoi passer),
   Action et Reaction (Punch First surligné), tenir un champ, la fin (Conseil, Reprendre, lexique).
4. **Mots à toucher** sous chaque explication (« Touche un mot : ») : la définition s'ouvre dans le coach.
5. **Neuf astuces de situation**, une fois chacune, jamais par-dessus une leçon ni sur un défi qui vient de s'afficher :
   priorité (une carte sur la chaîne : répondre en Reaction ou passer), focus (à toi de parler), une question, un sort jouable, champion jouable, combat, Darius avance, Darius marque, défausse.
6. Puce « Tuto k/18 » (barre du haut) : replie et rouvre le coach ; une leçon dont le moment arrive le rouvre.
7. Écran de fin (6 défis ou fin de partie) : continuer la partie, recommencer le tuto, partie contre l'IA.

**Lexique** (`GLOSS`, dans toute la table, pas seulement le tutoriel) : 28 mots du jeu (Might, énergie, puissance, rune,
épuisé, redresser, base, champ, conquérir, tenir, combat, showdown, priorité, focus, passer, chaîne, légende, champion…) et 31 mots-clés de cartes
(Reaction, Action, Accelerate, Ambush, Assault, Shield, Tank, Deflect, Ganking, Hidden, Legion, Deathknell…), écrits
d'après les Core Rules (numéros en commentaire) et les rappels imprimés sur les cartes. Bouton « Lexique » (coach,
intro). **L'agrandi d'une carte** (survol ou toucher, n'importe quelle partie) explique sous son texte ses mots-clés
`[Ainsi]` puis les mots courants (buff, ready, exhaust, recycle, conquer, hold, XP, token, banish, showdown), 5 au plus.

**Decks et donne** : Maître Yi Wuju Bladesman (`yi-bladesman_swagalisk_rq-la_6th`, facile à jouer ; sa légende donne
+2 might à une unité qui défend seule) contre Darius (`darius_mice-diamondhat_rq-utrecht_6th`). Donne 202, tu commences,
main First Mate, Pit Rookie ×2, Punch First (trouvée par `train.tuto_seeds()`) ; Zhonya's Hourglass piochée au tour 1.

**IA débutante** (`train.Gentle`, `level=0`) : la moitié de ses décisions au hasard parmi les coups légaux, l'autre
moitié par l'IA normale en recherche « sh » ; son hasard a sa propre graine, copiée avec l'état (« Reprendre » et le
rejeu restent exacts). Sa force n'est **pas mesurée** (aucun chiffre à citer).

La partie du tutoriel n'est pas enregistrée dans « Mes parties ». Version anglaise : tous les textes sont dans `i18n.js`.

**Vérification** : test moteur `tutorial_game_is_fixed_and_playable`. Robot `verif_tuto.mjs` (porte) en 1400x900 et
393x851 (toucher simulé), 37 contrôles par taille : intro et lexique, chaque leçon et sa zone surlignée, mot et
définition, agrandi de Punch First (Action expliqué), défis 1 à 6, tour de Darius puis leçons du tour 2, astuces vues,
écran de fin, recommencer, puis partie normale sans coach ; coach et puce mesurés dans l'écran. `verif_anglais.mjs`
dessine chaque étape, astuce, l'attente, le lexique, 9 agrandis et la fin en anglais. Les coups passent par `choose(i)`,
pas par le glisser. **Pas essayé sur un vrai téléphone ni par un humain débutant.**

Pièges : la classe d'animation `pop` de la puce reprenait le style `.pop` (renommée `tbump`) ; l'agrandi a
`pointer-events: none`, d'où des définitions affichées en liste (pas de bulle à toucher dans l'agrandi).
