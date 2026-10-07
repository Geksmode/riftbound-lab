---
titre: Publication de la table (Pages, artefact)
resume: comment la table d'entraînement est construite et publiée, et les pièges connus
maj: 2026-10-07
sources: .github/workflows/ci.yml, riftbound/train/build.sh, docs/REPRISE.md §3
---
- **GitHub Pages** : `ci.yml` construit `riftbound/train/build/` (Pyodide 0.26.4 via npm) et le publie depuis `main` si les tests passent. Réglage unique : Settings > Pages > Source = GitHub Actions. Sans base claude.ai, la page affiche « Partie non enregistrée ».
- **Artefact claude.ai** (ancienne voie) : `build/train.html` + fichiers, capacité `db`. Étapes et pièges (URL `blob:` interdites, stdlib en base64, 255 fichiers au plus, `.zip` non servi) : `docs/REPRISE.md` §3.
- Copie maîtresse de la page : `riftbound/train/src.html` (+ `base.css`) ; `build.sh` assemble.
