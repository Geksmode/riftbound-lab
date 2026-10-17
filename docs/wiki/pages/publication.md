---
titre: Publication de la table (Pages, artefact)
resume: comment la table d'entraînement est construite et publiée, et les pièges connus
maj: 2026-10-17
sources: .github/workflows/ci.yml, riftbound/train/build.sh, riftbound/train/precompile.mjs, docs/REPRISE.md §3
---
- **GitHub Pages** : `ci.yml` construit `riftbound/train/build/` (Pyodide 0.26.4 via npm) et le publie depuis `main` si les tests passent. Réglage unique : Settings > Pages > Source = GitHub Actions. Sans base claude.ai, la page affiche « Partie non enregistrée ».
- **Artefact claude.ai** (ancienne voie) : `build/train.html` + fichiers, capacité `db`. Étapes et pièges (URL `blob:` interdites, stdlib en base64, 255 fichiers au plus, `.zip` non servi) : `docs/REPRISE.md` §3.
- Copie maîtresse de la page : `riftbound/train/src.html` (+ `base.css`) ; `build.sh` assemble.
- **Python précompilé** (2026-10-17) : `build.sh` lance `train/precompile.mjs` (node + le Pyodide de `pyodide-cache/`). Les modules de la stdlib que la table charge passent en `.pyc` dans `python_stdlib.b64.txt` (les autres restent en `.py`) ; le moteur est livré en `.py` **et** en bytecode (`py/bytecode.json`, `.pyc` « checked-hash » : Python recompile si le `.py` diffère, jamais de bytecode périmé). Sans node, `build.sh` prévient et livre la stdlib d'origine ; sans `bytecode.json`, le worker compile les `.py` comme avant. Le worker télécharge tout en parallèle du wasm. Mesure : `docs/memoire/riftbound-training-table.md` (« Chargement »).
