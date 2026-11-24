#!/usr/bin/env bash
# Construit la table d'entraînement dans riftbound/train/build/ (page train.html + fichiers servis à côté).
# Le moteur est le crate Rust rust/ compilé en WebAssembly (riftbound.wasm), chargé par engine.worker.js.
# Prérequis : cargo et la cible wasm32-unknown-unknown (rustup target add wasm32-unknown-unknown) ; une fois,
# bash fetch_vendor.sh (PeerJS pour les duels entre amis).
set -e
cd "$(dirname "$0")"
E=$(cd .. && { pwd -W 2>/dev/null || pwd; })   # dossier riftbound/ (pwd -W : chemin C:/… lisible par Python sous Git Bash)
R=$(cd ../../rust && pwd)
B=build
[ -f vendor-cache/peerjs/package/dist/peerjs.min.js ] || { echo "Lance d'abord : bash fetch_vendor.sh"; exit 1; }
# version du moteur : empreinte des sources du crate (chemin relatif + contenu), comme version.engine_md5 en Python
ENGINE=$(cd "$R" && find src Cargo.toml -type f \( -name '*.rs' -o -name Cargo.toml \) | LC_ALL=C sort \
  | while read -r f; do printf '%s' "$f"; cat "$f"; done | md5sum | cut -c1-10)
(cd "$R" && RB_ENGINE="$ENGINE" cargo build --release --lib --target wasm32-unknown-unknown --quiet)
rm -rf $B && mkdir -p $B/img $B/atlas
cp "$R/target/wasm32-unknown-unknown/release/riftbound.wasm" $B/
ENGINE="$ENGINE" python3 - <<'P'
import os, json, subprocess
s = open('src.html').read().replace('/*BASE_CSS*/', open('base.css').read()).replace('/*DA_CSS*/', open('da.css').read())
s = s.replace('/*I18N_JS*/', open('i18n.js').read())   # version anglaise (i18n.js), dans la page même
commit = subprocess.run(["git", "rev-parse", "--short=12", "HEAD"], capture_output=True, text=True).stdout.strip() or None
s = s.replace("/*BUILD*/{ commit: null, engine: null }", json.dumps(dict(commit=commit, engine=os.environ["ENGINE"])))
open('build/train.html', 'w').write(s)
P
cp engine.worker.js $B/
cp $E/replays/img/* $B/img/
cp atlas/* $B/atlas/
# sons de la table (banque uisfx, audio CC0 : sfx/LICENSE-AUDIO)
cp -r sfx $B/sfx
# PeerJS pour les duels entre amis (téléchargé par fetch_vendor.sh)
mkdir -p $B/vendor && cp vendor-cache/peerjs/package/dist/peerjs.min.js $B/vendor/
# lecteur de replays servi à côté de la table (il lit img/index.json, déjà copié ci-dessus, et games/)
cp $E/replays/viewer.html $B/replays.html
mkdir -p $B/games && cp $E/replays/games/* $B/games/
echo "OK : $B/train.html (moteur WebAssembly $ENGINE, $(( $(stat -c %s $B/riftbound.wasm) / 1024 )) Ko, $(ls $B/atlas | wc -l) planches d'images)"
