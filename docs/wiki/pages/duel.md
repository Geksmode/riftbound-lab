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

**Interface faite (2026-10-08, agent design, commit `43ddbc7`)** : menu « Jouer avec un ami » (pseudo, deck, « Créer une room » BO1 / BO3 avec
code de 6 caractères sans I L O 0 1, identifiant PeerJS `rbl-<code>`, lien `#room=CODE`, « Rejoindre ») ; poignée de main (commit, moteur, deck
validé) ; préparation des manches avec `match_duel_next` (tirage, premier/second, battlefields simultanés, sideboard commun avec le mode IA) ;
entrées `{t:"in", g, n, op, v}` appliquées dans l'ordre, resynchronisation `sync` ; plateau retourné pour l'invité ; « En attente de <pseudo> » ;
pastille réseau ; reprise de l'invité qui recharge (`resume`, jeton de session) ; REC par manche (`kind:"duel"`, `args`, `inputs` des deux places).
**Vérifié** par `train/verif_duel.mjs` (dans la porte, étape `duel`) : deux Chromium (hôte 1400×900 souris, invité 360×740 toucher), serveur PeerJS
local (`peerjs --host 127.0.0.1 --path /myapp`) : room BO3 par clics, manche 1 jouée jusqu'au bout avec vérification continue (score, tour, plateau,
mains, défausses, runes, une seule main à la fois, « En attente » chez l'autre), manche 2 avec sideboard et battlefield retiré, rechargement de l'invité
et reprise ; 39 contrôles, 2 graines par l'agent et 1 relance par le chef, tous OK.
**Pas testé** : le vrai serveur 0.peerjs.com (refusé par le proxy du conteneur), deux appareils sur des réseaux différents (NAT, pas de serveur TURN),
la page comme artefact claude.ai ; BO1 en duel ; manche nulle ; fin d'un BO3 complet ; refus de version ou de deck ; room introuvable ; lien `#room=` ;
rechargement pendant la préparation ; changement de champion au sideboard. Si l'**hôte** recharge, la room est perdue (documenté dans l'aide).
Les parties de duel ne sont pas rejouées par `train_games.py` (pas de clé `moves`) : à faire si utile.
**Correctif du 2026-10-08 (bug réel signalé par l'utilisateur)** : l'invité était « Connecté : l'hôte prépare le match… » mais l'hôte restait
« en attente de ton ami ». Non reproduit en local (5 variantes OK : BO1, deck perso, lien, même deck). Cause la plus probable : encodage
binaire par défaut de PeerJS entre navigateurs différents. Changements : encodage JSON (`serialization: "json"`), l'invité renvoie le hello
toutes les 6 s (3 fois), l'hôte affiche « Ton ami se connecte… », journal de connexion avec « Copier le diagnostic » (à demander à
l'utilisateur si le blocage revient). Vérifié en local : robot du duel vert, hello perdu volontairement puis renvoyé, match démarré.

**2026-10-09** : l'utilisateur confirme que le duel fonctionne en réseau réel (après les correctifs JSON et renvoi du hello).
