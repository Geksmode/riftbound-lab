#!/usr/bin/env bash
# Télécharge PeerJS 1.5.5 (paquet npm officiel ; duels entre amis : WebRTC, mise en relation par le serveur public de
# PeerJS) dans vendor-cache/, servi avec la page. Le moteur n'a plus besoin de Pyodide (WebAssembly, crate rust/).
set -e
cd "$(dirname "$0")"
mkdir -p vendor-cache && cd vendor-cache
npm pack peerjs@1.5.5 >/dev/null
mkdir -p peerjs && tar xzf peerjs-1.5.5.tgz -C peerjs && rm peerjs-1.5.5.tgz
ls peerjs/package/dist/peerjs.min.js
