---
titre: Carte du dépôt et commandes
resume: où se trouve quoi dans le dépôt, et les commandes de test, de simulation et de construction
maj: 2026-11-24
sources: CLAUDE.md (ancienne version), riftbound/engine/README.md
---
| Dossier | Contenu |
|---|---|
| `riftbound/cards/` | base de cartes (938 cartes uniques, 1231 impressions avec `image_url`), script de reconstruction |
| `riftbound/rules/` | Core Rules du 2026-07-16 (`source/`), résumés FR, règles de tournoi, ban list |
| `riftbound/decks/` | decklists de tournoi parsées (`decks.json`), analyses de matchup |
| `rust/` | **moteur de règles fidèle 1v1, 1037 cartes, IA, outils et table en WebAssembly** (crate Rust ; `src/`, `src/bin/` = un binaire par outil, `src/cardsets/` = cartes par paquet, `tests/`, `diff/`) ; binaires dans `rust/target/release/` |
| `riftbound/engine/` | données du moteur (`results*.json`, `empreinte.json`, `pool.txt`, `banc/`, `texel/`) et guides de modélisation `cardsets/GUIDE.md` ; lire `engine/README.md` (le moteur Python est supprimé, dernier état : étiquette git `moteur-python-final`) |
| `riftbound/gameplans/` | plans de jeu (Akali Gorica, LeBlanc Hook tempo) et notes de vidéos |
| `riftbound/manager/` | agent manager : sessions de simulation toutes les 6 h (`PROTOCOLE.md`, `learnings.md`, `sessions/`) |
| `riftbound/replays/` | lecteur de replays (`viewer.html`) et parties enregistrées |
| `riftbound/train/` | table d'entraînement (navigateur, moteur Rust en WebAssembly), éditeur de deck, parties de l'utilisateur, analyses |
| `riftbound/puzzles/` | puzzles Akali vs Yi |
| `riftbound/retex/` | retex écrits |
| `riftbound/sim/` | ancien simulateur simplifié (Python), **périmé**, gardé pour l'historique |
| `docs/` | `wiki/` (ce wiki), `memoire/`, `vault/`, `REPRISE.md`, `MISSIONS.md` |
| `.claude/`, `scripts/`, `.github/` | rôles des agents, outils d'équipe et de contrôle, CI |

Commandes (le moteur est en Rust : `cd rust && cargo build --release` ; seuls `scripts/wiki_lint.py`, `riftbound/decks/*.py`, `riftbound/cards/*.py`, `riftbound/tools/`, `riftbound/sim/` et les scripts Playwright de `riftbound/train/` restent en Python). Les outils lisent et écrivent leurs fichiers (`results*.json`, `empreinte.json`) à côté : les lancer depuis `riftbound/engine` :
```bash
cd rust && cargo test --release          # toute la suite, dont 300 parties identiques à la référence Python (tests/golden.rs)
cd ../riftbound/engine
../../rust/target/release/empreinte      # décisions de l'IA à graines fixes = empreinte.json (--maj si le jeu change exprès)
../../rust/target/release/fuzz_cards 200 --module fury  # parties aléatoires avec un paquet (0 exception attendu)
../../rust/target/release/t_rand 40      # 40 parties avec des decks légaux au hasard
python3 ../decks/check_raw.py ../decks/raw/<fichier>.txt   # valide un deck brut avec le moteur (outil check_decks) ; collecte riftatlas : ../decks/riftatlas/README.md
../../rust/target/release/sprt <graine0> <A> <B>   # test séquentiel (en-tête de rust/src/bin/*.rs pour les options)
cd ../train && bash fetch_vendor.sh && bash build.sh   # table dans train/build/ (moteur Rust en WebAssembly : cargo + cible wasm32-unknown-unknown)
```
Binaires disponibles : `rust/src/bin/` (`exp`, `exp_search`, `sprt`, `compare`, `banc`, `banc_cout`, `banc_humain`, `texel_gen`, `texel_fit`, `run_session`, `train_games`, `add_replays`...) ; options en tête de chaque `rust/src/bin/*.rs` et `rust/src/outils/*.rs`.

Après une modification du moteur : `cargo test --release`, `fuzz_cards` sur les paquets touchés, 30 parties `t_rand`, puis la
table dans un navigateur (robot Playwright `train/dk2.py`) avant de publier. Jamais d'ordre de `set`/`HashMap` qui compte
(trier) : même graine = même partie. Chemins relatifs à `riftbound/`.
