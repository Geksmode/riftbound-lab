# Table d'entraînement « Riftbound Observatory » (artefact https://claude.ai/artifact/HTmfbgtzcPP7jN7UsCfcHC)

Humain = Akali G2 (joueur 0) contre IA LeBlanc IQ#5, plan Hook tempo (joueur 1). Le vrai moteur (Rust) tourne dans le
navigateur, compilé en WebAssembly (`riftbound.wasm`, chargé par `engine.worker.js`). Logique : `../../rust/src/train.rs` et `wasm.rs` (new, step, act, answer, undo, hint, cards).

Republier : src.html + base.css (CSS du lecteur de replays) -> build.sh compile le moteur Rust en wasm, assemble train.html et copie
data/, img/ dans `build/` (cargo et la cible `wasm32-unknown-unknown` requis). Test : tr.py (Playwright).

## GitHub Pages
`.github/workflows/ci.yml` : les tests (`cargo test --release`, `t_rand 20`) et la construction de la table (`fetch_vendor.sh` +
`build.sh`) tournent en parallèle ; sur `main`, si les deux passent, `build/` est publié sur GitHub Pages
(réglage une fois : Settings > Pages > Source = « GitHub Actions »). Hors claude.ai la page n'a pas de base :
les parties ne sont pas enregistrées (« Partie non enregistrée »).
