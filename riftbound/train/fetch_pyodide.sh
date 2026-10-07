#!/usr/bin/env bash
# Télécharge Pyodide 0.26.4 (paquet npm officiel) dans pyodide-cache/package/. jsdelivr peut être bloqué : npm passe.
set -e
cd "$(dirname "$0")"
mkdir -p pyodide-cache && cd pyodide-cache
npm pack pyodide@0.26.4 >/dev/null
tar xzf pyodide-0.26.4.tgz && rm pyodide-0.26.4.tgz
ls package/pyodide.asm.wasm package/python_stdlib.zip
