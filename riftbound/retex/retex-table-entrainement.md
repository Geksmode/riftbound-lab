---
tags: [riftbound, retex, outil]
maj: 2026-10-06
---
# Retex : la table d'entraînement (v1 à v9, 5 au 6 octobre 2026)

Outil : https://claude.ai/artifact/HTmfbgtzcPP7jN7UsCfcHC (tu joues Akali G2 contre l'IA LeBlanc IQ#5, le vrai moteur
Python tourne dans la page via Pyodide). Sources : `riftbound/train/` (src.html est la copie maîtresse) et
`engine/train.py`.

## Ce qui tient

- La table fait tourner **le même moteur que les simulations**, pas une copie simplifiée : même règles, mêmes cartes,
  même IA LeBlanc. Les 91 tests de cartes passent toujours après toutes les modifications.
- Les choix vérifiés par une vraie partie dans le navigateur (robot de test `dnd4.py`, 4 parties complètes en v9, PC et
  téléphone) : mulligan, sorts à 1 et 2 cibles, Shuriken Flip en 3 étapes, Empower, Accelerate / Normal, déplacement
  simple et de groupe, copies en double dans la main, défausse, effet optionnel (Oui / Non), cible en cours d'effet
  (Akali, équipement), recyclage de rune, journal, mulligan après « Nouvelle partie ».
- Le coût d'Accelerate affiché (+1 énergie +1 rune du domaine de la carte) est celui de la règle 805.1.a.

## Ce que j'ai mal fait

1. **v1 publiée sans test dans les conditions réelles.** La page claude.ai interdit les URL `blob:` ; le moteur ne se
   chargeait pas (« exit(1) »). Je ne l'ai vu que par ton message. Depuis, je teste avec un serveur local qui applique
   la même politique de sécurité avant de publier.
2. **J'ai annoncé en v6 que les choix en cours d'effet se faisaient sur la table. C'était faux en vraie partie.**
   Je ne l'avais testé qu'avec une question injectée à la main. En vraie partie, la question vient d'une copie de l'état
   de jeu, donc la comparaison « cette carte est-elle sur la table ? » échouait toujours, et Blitzcrank, Zhonya ou
   l'équipement retombaient dans le texte. Tu l'as signalé en v8 ; c'est corrigé (comparaison par identifiant).
   Leçon : un test sur un état fabriqué ne prouve rien sur une vraie partie. Je ne dis « testé » que pour un chemin
   joué de bout en bout.
3. **Les erreurs du moteur figeaient la table sans rien dire** (de v1 à v8). En v9, une erreur de ma part dans
   `train.py` (un import mal placé) bloquait la partie dès le premier tour de LeBlanc : je l'ai trouvée avec le robot
   avant de publier, mais seulement parce que la partie s'arrêtait. Maintenant l'erreur s'affiche et la main revient.
   Après chaque modification du moteur, je fais aussi 30 parties complètes au hasard en Python (0 erreur en v9).
4. **Les boutons : trois allers-retours pour rien (v5 à v8).** Tu as demandé « tout sur le board » ; j'ai d'abord
   déplacé le panneau dans une colonne du plateau, qui restait un encart en haut à droite. C'est ta capture de
   RiftAtlas qui a tranché. Leçon : pour une demande d'interface, demander une référence (capture, site) dès le début.
5. **Les abstractions de l'IA ont fui vers le joueur humain.** Pour aller vite, le moteur ne propose à l'IA que
   quelques combinaisons de cibles (8 au plus, les 3 meilleurs ennemis pour Shuriken Flip). Le joueur humain a hérité de
   ces limites jusqu'à ce que tu les voies (Falling Star en v4, Shuriken Flip en v9). Les sorts à cibles simples de ton
   deck (Falling Star, Discipline, Back Off, Block) et Shuriken Flip sont maintenant complets.

## Ce qu'on ne sait pas encore

- **Les autres cartes à choix multiples ne sont pas vérifiées une par une.** Pas de risque connu dans ton deck
  aujourd'hui, mais une capacité activée avec cible (ex. équiper Sterak's Gage) n'est pas couverte par l'extension
  « toutes les combinaisons ». À vérifier carte par carte quand on ajoutera des decks.
- **Choix jamais vus dans le navigateur :** défausser une carte, où poser l'unité, Herald, ordre des dégâts, écran de
  victoire (le robot perd toujours). Le code existe, mais on ne sait pas s'il s'affiche bien.
- **L'énergie flottante (v6) a changé le moteur pour toutes les simulations.** Les chiffres d'avant le 6 octobre
  (G2 51,2 % contre LeBlanc, etc.) ont été mesurés sans cette règle. On ne sait pas s'ils bougent ; il faudra les
  remesurer sur des parties neuves avant de les citer à nouveau.
- Petites scories : quelques lignes du journal restent en anglais (« P1 plays … from banish ») ; le recyclage de rune
  montre un bouton par rune même quand elles sont identiques.

## Ce que je change

- Tester chaque version dans les conditions réelles (sécurité de la page, vraie partie de bout en bout, PC et
  téléphone) avant de publier, et ne dire « testé » que pour ce qui a été joué ainsi.
- Après toute modification du moteur : tests de cartes + 30 parties au hasard + parties navigateur.
- Pour chaque nouvelle carte à plusieurs choix : vérifier que l'humain voit toutes les options légales, pas la liste
  réduite de l'IA.
- Pour l'interface : partir d'une référence que tu montres, et proposer une maquette avant de tout refaire.

Liens : [[Moteur de règles fidèle]], [[Retex étude Akali-LeBlanc]], [[Chiffres périmés]].
