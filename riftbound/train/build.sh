#!/usr/bin/env bash
# Construit la table d'entraînement dans riftbound/train/build/ (page train.html + fichiers servis à côté).
# Prérequis une fois : bash fetch_pyodide.sh (télécharge Pyodide 0.26.4 depuis npm dans pyodide-cache/).
set -e
cd "$(dirname "$0")"
E=$(cd .. && pwd)            # dossier riftbound/
B=build
[ -f pyodide-cache/package/pyodide.js ] || { echo "Lance d'abord : bash fetch_pyodide.sh"; exit 1; }
rm -rf $B && mkdir -p $B/py/cardsets $B/data $B/pyodide $B/img $B/atlas
for f in game.py actions.py cards.py ai.py plans.py decks.py replay.py train.py match.py pool.txt; do cp $E/engine/$f $B/py/; done
# modules de cartes : la liste explicite de cardsets/__init__.py (la page charge les fichiers par nom)
CS=$(python3 -c "
import re, os
s = open('$E/engine/cardsets/__init__.py').read()
names = re.findall(r'[\"\']([a-z0-9_]+)[\"\']', s[s.index('MODULES'):])
print(' '.join(n for n in names if os.path.exists('$E/engine/cardsets/' + n + '.py')))")
cp $E/engine/cardsets/__init__.py $B/py/cardsets/
for n in $CS; do cp $E/engine/cardsets/$n.py $B/py/cardsets/; done
python3 - "$CS" <<'P'
import sys, json
mods = [m for m in sys.argv[1].split() if m]
files = ["cardsets/__init__.py"] + [f"cardsets/{m}.py" for m in mods]
s = open('src.html').read().replace('/*BASE_CSS*/', open('base.css').read())
s = s.replace('/*CARDSETS*/', "".join(", " + json.dumps(f) for f in files))
import subprocess, sys as _s
commit = subprocess.run(["git", "rev-parse", "--short=12", "HEAD"], capture_output=True, text=True).stdout.strip() or None
_s.path.insert(0, "../engine"); import version
s = s.replace("/*BUILD*/{ commit: null, engine: null }", json.dumps(dict(commit=commit, engine=version.engine_md5())))
open('build/train.html', 'w').write(s)
print("modules de cartes :", len(mods))
P
cp $E/cards/cards_unique.csv $E/decks/decks.json $B/data/
cp pyodide-cache/package/{pyodide.js,pyodide.asm.js,pyodide.asm.wasm,pyodide-lock.json} $B/pyodide/
base64 -w0 pyodide-cache/package/python_stdlib.zip > $B/pyodide/python_stdlib.b64.txt
cp $E/replays/img/* $B/img/
cp atlas/* $B/atlas/
# lecteur de replays servi à côté de la table (il lit img/index.json, déjà copié ci-dessus, et games/)
cp $E/replays/viewer.html $B/replays.html
mkdir -p $B/games && cp $E/replays/games/* $B/games/
echo "OK : $B/train.html ($(ls $B/py/cardsets | wc -l) fichiers de cartes, $(ls $B/atlas | wc -l) planches d'images)"
