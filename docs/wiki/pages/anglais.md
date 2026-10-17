---
titre: Version anglaise de la table
resume: comment la table passe en anglais (page et moteur), où ajouter une traduction, comment c'est vérifié
maj: 2026-10-17
sources: riftbound/train/i18n.js, riftbound/engine/lang.py, riftbound/train/verif_anglais.mjs, riftbound/engine/cardsets/test_lang.py
---
Demande de l'utilisateur (2026-10-17) : « fait une version anglaise de l'app ». Une seule table, deux langues.

## Choisir la langue
- Bouton **English / Français** sous le titre du menu principal ; ou l'adresse `train.html?lang=en` (lien à partager).
- Le choix est gardé dans le navigateur (clé `rbt-lang`) ; changer de langue recharge la page (la partie en cours
  est gardée comme d'habitude). Par défaut : français.

## Comment c'est fait
- **Page** : `src.html` reste écrit en français (aucune réécriture des 3 500 lignes). `train/i18n.js`, inséré dans la
  page par `build.sh` (`/*I18N_JS*/`), traduit le texte au moment où il entre dans la page : nœuds texte et attributs
  `title`, `aria-label`, `placeholder`, `label`, juste après chaque `innerHTML` / `textContent` /
  `insertAdjacentHTML` (avant les mesures de mise en page), plus un `MutationObserver` pour le reste ; `confirm()`
  aussi. Dictionnaire `D` (`${}` = partie variable, elle-même traduite) et motifs `P` pour les textes assemblés.
- **Moteur** : `engine/lang.py` (`set_lang`, `tr(fr, en)`, `ASK_EN` = titre anglais de chaque question). La page
  appelle `train.set_lang("en")` au chargement. Questions, choix, coups décrits, conseil, journal (`replay.en_line`,
  le journal du moteur est déjà en anglais), erreurs de deck et de sideboard sortent en anglais.
- **La langue ne change pas la partie** : le moteur ne lit jamais ces textes ; `test_lang.py` joue 3 parties
  (decks au hasard) en français puis en anglais avec les mêmes choix et compare chaque état.
- Les tests de la page qui lisaient du texte français acceptent les deux langues : sons (« Tour/Turn »,
  « Tu pioches/You draw », « meurt/dies »), classes du journal, raccourci espace (oui/yes), replay (« Tu joues/You play »).

## Ajouter ou changer un texte
- Nouveau texte dans `src.html` : ajouter la paire `[français, anglais]` à `D` dans `i18n.js` ; sinon le robot le
  signale (il reste affiché en français, rien ne casse).
- Nouvelle question `g.ask` dans un module : titre anglais dans `lang.ASK_EN` (sinon `test_lang.py` échoue) ;
  libellé ou ligne de journal montré au joueur : `tr("…", "…")`. Voir `engine/cardsets/GUIDE.md`.
- Vocabulaire : Donne = Deal, Manche = Game (d'un match), Conseil = Hint, Reprendre = Undo, Journal = Log,
  Défausse = trash, Passer = Pass, Terminer le tour = End turn ; termes de jeu en anglais officiel (ready, exhaust,
  recycle, banish, buff, channel, gear, battlefield, base, Might).

## Vérification
- `engine/cardsets/test_lang.py` (dans `test_all.py`) : chaque question a un titre anglais ; 3 parties en anglais sans
  aucun texte français ; même partie dans les deux langues ; erreurs de deck en anglais.
- `train/verif_anglais.mjs` (porte : étape `anglais`, `RB_EN=0` pour la sauter) : la table en `?lang=en` sur
  ordinateur et téléphone, menu, choix du deck, aide, mes decks, mes parties, ami, sons, éditeur (import, export),
  nouvelle partie BO1 / BO3, une manche BO3 jouée au hasard jusqu'au bout (mulligan, questions, conseil, défausse,
  journal, fin), sideboard et préparation de la manche 2, replay, tutoriel ; tout texte resté en français est relevé.
  Puis le bouton « Français » remet la table en français.
- Vérifié par robot seulement (toucher simulé), pas essayé sur un vrai téléphone.

## Pas traduit
- Le lecteur de replays à part (`replays.html`, parties enregistrées avec un journal en français), les noms d'événements
  des listes de tournoi (données), le texte des cartes (déjà en anglais), le format d'export d'un deck (« Légende : »).
- Le duel n'est vérifié en anglais que sur son écran d'accueil (le robot du duel joue en français) ; deux joueurs
  peuvent avoir chacun leur langue (seuls les libellés changent).
