# Riftbound LAB

Labo personnel pour le jeu de cartes **Riftbound** (Riot Games) : un moteur de règles fidèle où les cartes du jeu sont
modélisées une par une, une IA, des simulations de matchups, et une **table d'entraînement** pour jouer contre l'IA
dans le navigateur avec n'importe quel deck.

> Agents (Claude ou autres) : commencez par [`CLAUDE.md`](CLAUDE.md), puis [`docs/REPRISE.md`](docs/REPRISE.md).

## Ce qu'il y a dedans
- **Moteur** (`riftbound/engine/`) : règles complètes d'après les Core Rules du 16/07/2026. 917 cartes jouables sur
  919 (Baron Nashor et Baron Pit manquent : ils demandent un troisième battlefield). 968 tests.
- **Table d'entraînement** (`riftbound/train/`) : page web autonome (le moteur Python tourne dans le navigateur via
  Pyodide). Éditeur de deck avec les 938 cartes et les règles de construction, partie contre l'IA, conseil, retour en
  arrière, journal, enregistrement des parties.
- **Simulations** : `riftbound/engine/exp*.py`, `riftbound/manager/` (sessions automatiques et leurs retex).
- **Replays** (`riftbound/replays/viewer.html`) : parties marquantes des simulations, rejouables coup par coup.
- **Connaissances** : `docs/vault/` (notes Obsidian en français), `docs/memoire/` (mémoire des agents),
  `riftbound/retex/`, `riftbound/gameplans/`, `riftbound/decks/`.

## Démarrage rapide
```bash
cd riftbound/engine
python3 test_all.py          # toute la suite de tests (Python 3.11+, aucune dépendance)
python3 train.py 3           # une partie de test en mode texte
cd ../train
bash fetch_pyodide.sh        # une fois : Pyodide 0.26.4 depuis npm
bash build.sh                # construit build/train.html
python3 serve_csp.py         # http://localhost:8767/train.html (même politique de sécurité que claude.ai)
```

## Données
Cartes : bulk RiftHunt (snapshot du 2026-09-11), `riftbound/cards/build_cards.py` pour rafraîchir. Les images des
cartes (`riftbound/replays/img/`, `riftbound/train/atlas/`) appartiennent à Riot Games et ne servent qu'à l'usage
personnel ; `riftbound/train/get_thumbs.py` les retélécharge.
