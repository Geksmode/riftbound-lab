# Replays graphiques

Lecteur publié : https://claude.ai/artifact/ThZartFw9uTqb6jcjvpgFV (source `viewer.html`, données `games/`, images `img/`).

- `games/index.json` : les 8 parties commentées (Gorica G2 vs LeBlanc IQ#5), choisies dans un lot de 240 parties neuves
  (graines 60000-60239 : Akali 47,9 %, 56 % quand elle commence, 41 % en second).
- Régénérer : `cd ../engine && python3 curate_replays.py` (liste et commentaires dans `PICKS`).
- Nouveau lot : `python3 batch_replays.py 240 60000 <dossier>` écrit un JSON par partie et un `index.json` de résumés
  (écart de score max, retournements, cartes jouées) pour choisir d'autres parties.
- **Après chaque simulation**, ajouter ses parties marquantes : `python3 ../engine/add_replays.py flips|digest|add …`
  (mode d'emploi en tête de `engine/add_replays.py`). Chaque partie a un `group` (la simulation d'origine) et des `plans` ;
  le lecteur les range par simulation. Groupe actuel : « Simulation des plans de jeu » (3 parties sur la même donne
  80002 avec trois plans Akali, plus la défaite type du plan Gorica).
- Une partie seule : `python3 replay.py <graine> [bf_akali|-] [bf_leblanc|-] [premier]`.

L'enregistreur (`engine/replay.py`) est additif : `RecGame` et `RecAgent` héritent du moteur et de l'IA sans les modifier,
font les mêmes tirages aléatoires et remettent les compteurs d'identifiants à zéro, donc une graine redonne la même partie.
