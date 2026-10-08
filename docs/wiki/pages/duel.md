---
titre: Duel entre amis (room avec code, PeerJS)
resume: état du duel à deux humains (moteur fait et testé, interface à faire) et spécification complète de l'interface
maj: 2026-10-08
sources: riftbound/engine/train.py (duel_new, match_duel_next), riftbound/engine/cardsets/test_duel.py, riftbound/train/fetch_pyodide.sh, build.sh
---
**Décidé avec l'utilisateur (2026-10-08)** : connexion WebRTC via PeerJS (serveur public de mise en relation 0.peerjs.com, sans compte),
room créée par l'hôte avec un code, modes BO1 et BO3 avec les règles de match habituelles ([match.md](match.md)), pas de spectateurs.
Limite acceptée : chaque navigateur fait tourner toute la partie (la main adverse est lisible dans la console).

**Fait et testé (moteur)**
- `train.duel_new(seed, bf0, bf1, first, deck0, deck1, me, names)` : deux places humaines (0 = hôte, 1 = invité) ; les deux navigateurs
  l'appellent avec les mêmes arguments sauf `me`, puis appliquent la même suite d'entrées `act(i)` / `answer(x)` dans le même ordre.
  Vue : champ `me`, `wait: "adversaire"` quand l'autre a la main, coups adverses dans `ai` / `aii` ; pas de « Reprendre » en duel.
- Correctif nécessaire au duel : `_expand` construit les options pour le joueur qui décide (avant : toujours `ME`), sinon désynchronisation.
- `train.match_duel_next(state)` : préparation d'une manche sans choix d'IA (chooser, first imposé, battlefields permis par place, `bo1_bfs`,
  sideboard, graine). Tests : `cardsets/test_duel.py` (deux processus synchronisés jusqu'à la fin, main adverse cachée), `test_match.py`.
- PeerJS 1.5.5 servi avec la page : `build/vendor/peerjs.min.js` (téléchargé par `fetch_pyodide.sh`).
- Serveur PeerJS local pour les robots : `npm install peer@1.0.2` puis `peerjs --port 9123 --path /myapp` (arrêter par PID).

**À faire (interface, `riftbound/train/src.html`)** : agent design arrêté avant d'avoir écrit quoi que ce soit (2026-10-08, demande de
l'utilisateur d'attendre la session suivante). Spécification à reprendre telle quelle :
1. Menu « Jouer avec un ami » : pseudo (localStorage), deck ; « Créer une room » (BO1 / BO3, code de 6 caractères sans ambiguïté,
   identifiant PeerJS `rbl-<code>`, bouton « Copier le lien » `#room=CODE`) ou « Rejoindre » (code, ou lien).
2. Poignée de main : l'invité envoie `{t:"hello", name, deck, commit, engine}` ; l'hôte refuse si `BUILD.commit`/`engine` diffèrent ou si
   `train.check(deck)` signale une erreur ; sinon `match_new` et `{t:"match", state, decks, names, mode}`.
3. Préparation de chaque manche avec `match_duel_next` : tirage (manche 1) ; le joueur `chooser` envoie `{t:"first"}` ; battlefields
   choisis simultanément (`{t:"bf"}`, révélés quand les deux sont reçus) ou `bo1_bfs` ; sideboard si permis (`{t:"deck"}`) ; puis l'hôte
   envoie `{t:"start", args}` et les deux appellent `duel_new(...args, maPlace, names)`.
4. Pendant la manche : chaque entrée `{t:"in", n, op:"act"|"ans", v}` (numéro de séquence) appliquée dans l'ordre ; conseil désactivé.
5. Fin de manche : `match_record` des deux côtés, score du match, manche suivante / fin du match.
6. Reconnexion : l'hôte garde les arguments et les entrées de la manche ; `{t:"resume", args, inputs}` pour que l'invité qui recharge rejoue tout.
7. Enregistrement REC par manche avec `duel {room, me, names}`, les arguments de `duel_new` et toutes les entrées des deux places.
8. `?peer=host:port` pour utiliser un serveur PeerJS local (tests) ; sinon 0.peerjs.com.
9. Robot `train/verif_duel.mjs` : deux pages (1400×900 hôte, 360×740 invité), room BO3 par clics, manche jouée jusqu'au bout avec
   vérification continue de la synchronisation, manche 2 avec sideboard, rechargement de l'invité et reprise, aucune erreur JS.
Non testable ici a priori : le vrai serveur 0.peerjs.com (réseau du conteneur filtré).
