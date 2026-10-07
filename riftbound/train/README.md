# Table d'entraînement (artefact https://claude.ai/artifact/HTmfbgtzcPP7jN7UsCfcHC)

Humain = Akali G2 (joueur 0) contre IA LeBlanc IQ#5, plan Hook tempo (joueur 1). Le vrai moteur Python tourne dans le
navigateur avec Pyodide 0.26.4 (npm `pyodide@0.26.4`, fichiers publiés avec la page ; python_stdlib.zip publié en
base64 .txt car les .zip ne sont pas servis). Logique côté Python : ../engine/train.py (new, step, act, answer, undo, hint, cards).

Republier : src.html + base.css (CSS du lecteur de replays) -> build.sh assemble train.html et copie py/, data/, pyodide/, img/
dans un dossier de publication (build.sh attend le paquet npm pyodide décompressé dans ../pyo/package). Test : tr.py (Playwright).

## GitHub Pages
`.github/workflows/ci.yml` : les tests (`test_all.py`, `t_rand.py 20`) et la construction de la table (`fetch_pyodide.sh` +
`build.sh`) tournent en parallèle ; sur `main`, si les deux passent, `build/` est publié sur GitHub Pages
(réglage une fois : Settings > Pages > Source = « GitHub Actions »). Hors claude.ai la page n'a pas de base :
les parties ne sont pas enregistrées (« Partie non enregistrée »).
