---
titre: Carte du dépôt et commandes
resume: où se trouve quoi dans le dépôt, et les commandes de test, de simulation et de construction
maj: 2026-10-07
sources: CLAUDE.md (ancienne version), riftbound/engine/README.md
---
| Dossier | Contenu |
|---|---|
| `riftbound/cards/` | base de cartes (938 cartes uniques, 1231 impressions avec `image_url`), script de reconstruction |
| `riftbound/rules/` | Core Rules du 2026-07-16 (`source/`), résumés FR, règles de tournoi, ban list |
| `riftbound/decks/` | decklists de tournoi parsées (`decks.json`), analyses de matchup |
| `riftbound/engine/` | moteur de règles fidèle 1v1, IA, simulations ; `cardsets/` = toutes les cartes par paquet (lire `engine/README.md` puis `cardsets/GUIDE.md`) |
| `riftbound/gameplans/` | plans de jeu (Akali Gorica, LeBlanc Hook tempo) et notes de vidéos |
| `riftbound/manager/` | agent manager : sessions de simulation toutes les 6 h (`PROTOCOLE.md`, `learnings.md`, `sessions/`) |
| `riftbound/replays/` | lecteur de replays (`viewer.html`) et parties enregistrées |
| `riftbound/train/` | table d'entraînement (navigateur, Pyodide), éditeur de deck, parties de l'utilisateur, analyses |
| `riftbound/puzzles/` | puzzles Akali vs Yi |
| `riftbound/retex/` | retex écrits |
| `riftbound/sim/` | ancien simulateur simplifié, **périmé**, gardé pour l'historique |
| `docs/` | `wiki/` (ce wiki), `memoire/`, `vault/`, `REPRISE.md`, `MISSIONS.md` |
| `.claude/`, `scripts/`, `.github/` | rôles des agents, outils d'équipe et de contrôle, CI |

Commandes (Python 3 standard, aucune dépendance ; le moteur tourne aussi dans Pyodide 0.26.4, 32 bits) :
```bash
cd riftbound/engine
python3 test_all.py                      # toute la suite (968 attendus)
python3 fuzz_cards.py 200 --module fury  # parties aléatoires avec un paquet (0 exception attendu)
RB_CARDSETS=fury,body python3 ...        # ne charger que certains paquets
python3 ../train/t_rand.py 40            # 40 parties avec des decks légaux au hasard
cd ../train && bash fetch_pyodide.sh && bash build.sh   # table dans train/build/
```
Après une modification du moteur : `test_all.py`, `fuzz_cards.py` sur les paquets touchés, 30 parties `t_rand.py`, puis la
table dans un navigateur (robot Playwright `train/dk2.py`) avant de publier. Jamais d'ordre de `set` qui compte
(`sorted()`) : même graine = même partie. Chemins relatifs à `riftbound/` (`Path(__file__).parents[1]`).
