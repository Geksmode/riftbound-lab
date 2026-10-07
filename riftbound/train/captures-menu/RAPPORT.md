# Rapport design, itération 1 : menu d'accueil (2026-10-07)

(Écrit ici, dans le dépôt, car le sandbox de cette exécution interdit d'écrire dans `$RB_EQUIPE/rapports/` : au chef de le copier.)

## Fait (fichier touché : `riftbound/train/src.html` uniquement ; moteur et `base.css` intacts)
- **Menu d'accueil** plein écran, affiché au démarrage à la place de la fenêtre « Nouvelle partie » : Nouvelle partie, Créer / modifier un deck, Mes decks (liste : Modifier / Jouer), Mes parties, Replays (lien `../replays/viewer.html`, affiché seulement si un `HEAD` sur ce fichier réussit, sinon masqué), Aide (9 lignes de règles de la table). « Reprendre la partie » / « Revoir la partie terminée » apparaît quand une partie existe.
- **Retour au menu partout** : bouton « ☰ Menu » dans la barre de la table, « ← Menu » (et ✕) dans l'éditeur, « ← Menu » dans les sous-vues, fermeture (✕, clic hors fenêtre, Échap) de « Nouvelle partie » ramène au menu s'il n'y a pas de partie. Échap dans une sous-vue revient au menu.
- **Mes parties** : lit la collection `games` de la base ; sans base : « Enregistrement indisponible hors claude.ai ».
- **Responsive** : grille auto (1 colonne à 360 px, 3 à 1400 px), cibles ≥ 44 px dans le menu, barre de la table et boutons de commande portés à 44 px sous 820 px.
- **Deux correctifs au passage** : (1) les bulles (`toast`) passaient sous l'éditeur de deck (z-index 45 < 55), maintenant 80 ; (2) lancer une nouvelle partie avant l'enregistrement différé (2,5 s) perdait la fin de la partie précédente, et un enregistrement tardif pouvait s'écrire sous l'id de la partie suivante : `recSave` fige maintenant la partie visée et `start()` enregistre l'ancienne avant de la remplacer.

## Vérification (Chromium 1194 réel, serveur local sur `build/`)
**Playwright n'est pas installé et `pip install` est refusé dans ce sandbox** : le test (`menu_test.mjs`, dans le scratchpad de la session, non committé) pilote donc Chromium via le protocole DevTools : vrais clics souris (1400) / tactiles (360, émulation mobile) aux coordonnées des éléments. Les coups de jeu, eux, sont injectés comme dans `dk2.py` (`T.act`/`T.answer` choisis au hasard, mulligan par le bouton réel), pas cliqués sur les cartes.
Exécutions : 360×740 et 1400×900, sans base ; 360×740 et 1400×900 avec une base `claude.use("db")` **simulée en mémoire** (la vraie base claude.ai n'a pas été testée).
- Parcours 1 : menu → Nouvelle partie (donne 7) → 45 coups (tour 13) → ☰ Menu → Reprendre : OK aux 4 exécutions.
- Parcours 2 : menu → éditeur → deck copié du préréglage LeBlanc, enregistré → ← Menu → Mes decks → Jouer (deck présélectionné) → 26 coups (tour 9) : OK.
- 23 contrôles par exécution sans base, tous OK : pas de défilement horizontal (menu, aide, parties, setup, table, éditeur, mes decks), cibles ≥ 44 px (menu, Mes decks), 5 entrées présentes, Replays masqué sans viewer, aucune erreur JS. Avec Replays : lien affiché si `build/replays/viewer.html` existe (testé à 360 px avec un fichier factice).
- Avec base simulée : Mes parties liste les 2 parties, la première avec son état à jour (tour 13) après le correctif.
- Captures : `captures-menu/` (360x740-* et 1400x900-*, logs `.log`).

## Pas testé / limites
- Base claude.ai réelle, GitHub Pages, Pyodide hors Chromium, Safari/Firefox, vrai téléphone. Aucun test de clic sur les cartes ni glisser-déposer.
- `build.sh` ne copie pas `replays/viewer.html` : le lien Replays reste donc masqué dans `build/` ; sur GitHub Pages il dépend de la disposition du site (à vérifier par le chef).
- Problèmes d'ergonomie repérés, **non traités** (itérations suivantes) : à 360 px la bulle de fin de partie (« Défaite 0-8 / Nouvelle partie / Rejouer la même donne ») chevauche la barre de commande collante (voir `360x740-06-table-milieu.png`) ; dans l'éditeur à 360 px le volet du deck occupe 38 % de la hauteur et les boutons ± (24 px) sont trop petits pour le tactile.
- Les coups aléatoires donnent des parties perdues ; rien n'est dit ici sur la force de l'IA.

## Questions pour le chef
1. Où est servi `replays/viewer.html` par rapport à `train.html` sur GitHub Pages ? (le lien suppose `../replays/viewer.html`)
2. OK pour que je copie `menu_test.mjs` (robot sans Playwright) dans `train/` ?

Pas de `git push` (consigne). Commit sur la branche `agent/design`.
