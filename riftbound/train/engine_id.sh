#!/usr/bin/env bash
# Imprime la version du moteur de la table : empreinte des sources du crate rust/ (chemin relatif + contenu), comme
# version.engine_md5 en Python. build.sh la pose dans le WebAssembly (RB_ENGINE) ; scripts/verifier.sh passe la même
# valeur quand il compile le WebAssembly, pour que cargo ne le recompile pas une seconde fois dans build.sh.
set -e
cd "$(dirname "$0")/../../rust"
find src Cargo.toml -type f \( -name '*.rs' -o -name Cargo.toml \) | LC_ALL=C sort \
  | while read -r f; do printf '%s' "$f"; cat "$f"; done | md5sum | cut -c1-10
