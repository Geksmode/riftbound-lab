#!/usr/bin/env bash
# Porte d'intégration : rien n'entre dans une branche de travail ni dans main sans passer par là.
#
#   scripts/verifier.sh            tout : wiki, moteur, table (+ robot navigateur si Playwright et Chromium sont là)
#   scripts/verifier.sh moteur     wiki + tests + fuzz de chaque paquet + parties aléatoires
#   scripts/verifier.sh table      construction de la table (+ robots navigateur : téléphone, Hidden, match, duel)
#
# Variables : RB_FUZZ (parties par paquet, défaut 200), RB_RAND (parties t_rand, défaut 30),
#             RB_NAV=0 pour sauter les robots navigateur, RB_MATCH=0 pour sauter le robot du match BO3 (~6 min),
#             RB_DUEL=0 pour sauter le robot du duel entre amis (serveur PeerJS local : paquet npm peer@1.0.2),
#             RB_PORT (défaut 8771).
# Code de sortie 0 seulement si tout passe ; un résumé est imprimé à la fin.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
E="$ROOT/riftbound/engine"; TR="$ROOT/riftbound/train"
QUOI="${1:-tout}"; FUZZ="${RB_FUZZ:-200}"; RAND="${RB_RAND:-30}"; PORT="${RB_PORT:-8771}"
LOG="$(mktemp -d)"; RES=(); ECHEC=0

etape() {   # etape <nom> <commande...> : lance, garde la sortie, note OK / ÉCHEC
  local nom="$1"; shift
  local t0=$SECONDS
  if "$@" > "$LOG/$nom.txt" 2>&1; then RES+=("OK     $nom ($((SECONDS - t0)) s) : $(tail -1 "$LOG/$nom.txt")")
  else RES+=("ÉCHEC  $nom ($((SECONDS - t0)) s) : voir $LOG/$nom.txt"); ECHEC=1; tail -15 "$LOG/$nom.txt" >&2; fi
}

wiki()   { python3 "$ROOT/scripts/wiki_lint.py"; }
tests()  { cd "$E" && python3 test_all.py; }
fuzz()   {
  cd "$E" || return 1
  local mods out bad=0
  mods=$(python3 -c "import re; s=open('cardsets/__init__.py').read(); print(' '.join(re.findall(r'[\"\x27]([a-z0-9_]+)[\"\x27]', s[s.index('MODULES'):])))")
  for m in "" $mods; do
    out=$(python3 fuzz_cards.py "$FUZZ" ${m:+--module "$m"} 2>&1 | grep -E "^[0-9]+ games, [0-9]+ exceptions" | tail -1)
    echo "${m:-cards.py} : ${out:-pas de bilan}"
    case "$out" in *" 0 exceptions"*) ;; *) bad=1;; esac
  done
  echo "fuzz : $FUZZ parties par paquet, $( [ $bad = 0 ] && echo '0 exception partout' || echo 'EXCEPTIONS' )"
  return $bad
}
hasard() {
  cd "$E" || return 1
  local out; out=$(python3 ../train/t_rand.py "$RAND" 2>&1); local rc=$?
  out=$(grep -E "^[0-9]+ parties" <<<"$out" | head -1); echo "${out:-pas de bilan}"
  [ $rc = 0 ] && grep -q "erreurs 0" <<<"$out"
}
table()  {
  cd "$TR" || return 1
  [ -f pyodide-cache/package/pyodide.js ] || bash fetch_pyodide.sh >/dev/null || return 1
  bash build.sh | tail -1
}
navigateur() {
  local havepw=0
  [ -f /opt/node22/lib/node_modules/playwright/index.mjs ] && havepw=1
  (cd "$TR" && node -e "require.resolve('playwright')" >/dev/null 2>&1) && havepw=1
  if [ "${RB_NAV:-1}" = 0 ] || [ $havepw = 0 ]; then echo "robot navigateur sauté (RB_NAV=0 ou Playwright absent)"; return 0; fi
  (cd "$TR/build" && exec python3 -m http.server "$PORT" >/dev/null 2>&1) & local srv=$!
  sleep 1
  local out; out=$(cd "$TR" && timeout 900 node verif_mobile.mjs "$LOG" "$PORT" 2>&1); local rc=$?
  kill $srv 2>/dev/null
  echo "$out"
  [ $rc = 0 ] && ! grep -q "^ÉCHEC" <<<"$out" && echo "robot navigateur : $(grep -c '^OK' <<<"$out") contrôles OK (captures dans $LOG)"
}

cache() {   # robots : verif_hidden.mjs (cacher, jouer depuis la face cachée), verif_revele.mjs (main révélée), verif_charge.mjs (téléphone chargé), verif_cibles.mjs (ciblage par liste), verif_flow.mjs (Flow depuis la défausse), verif_vs.mjs (sélection contre l IA), verif_fleches.mjs (flèches), verif_predict.mjs (Predict), verif_replay.mjs (mode replay), verif_tuto.mjs (tutoriel), verif_dash.mjs (tableau de bord du menu), verif_espace.mjs (barre d espace)
  if [ "${RB_NAV:-1}" = 0 ]; then echo "robot Hidden sauté (RB_NAV=0)"; return 0; fi
  [ -f /opt/node22/lib/node_modules/playwright/index.mjs ] || (cd "$TR" && node -e "require.resolve('playwright')" >/dev/null 2>&1) \
    || { echo "robot Hidden sauté (Playwright absent)"; return 0; }
  local port=$((PORT + 4))
  (cd "$TR/build" && exec python3 -m http.server "$port" >/dev/null 2>&1) & local srv=$!
  sleep 1
  local out; out=$(cd "$TR" && timeout 900 node verif_hidden.mjs "$LOG" "$port" 2>&1 && timeout 600 node verif_revele.mjs "$LOG" "$port" 2>&1 && timeout 600 node verif_charge.mjs "$LOG" "$port" 2>&1 && timeout 600 node verif_cibles.mjs "$LOG" "$port" 2>&1 && timeout 600 node verif_flow.mjs "$LOG" "$port" 2>&1 && timeout 600 node verif_vs.mjs "$LOG" "$port" 2>&1 && timeout 900 node verif_fleches.mjs "$LOG" "$port" 2>&1 && timeout 600 node verif_predict.mjs "$LOG" "$port" 2>&1 && timeout 1200 node verif_replay.mjs "$LOG" "$port" 2>&1 && timeout 900 node verif_tuto.mjs "$LOG" "$port" 2>&1 && timeout 600 node verif_dash.mjs "$LOG" "$port" 2>&1 && timeout 600 node verif_espace.mjs "$LOG" "$port" 2>&1); local rc=$?
  kill $srv 2>/dev/null
  echo "$out"
  [ $rc = 0 ] && ! grep -q "ÉCHEC" <<<"$out" && echo "robots Hidden, main révélée, charge, ciblage, Flow, sélection contre l IA, flèches, Predict, replay et tutoriel : $(grep -c '^OK' <<<"$out") contrôles OK"
}

match() {   # robot du match BO3 et du sideboard (train/verif_match.mjs), en 360x740 et 1400x900
  if [ "${RB_NAV:-1}" = 0 ] || [ "${RB_MATCH:-1}" = 0 ]; then echo "robot du match sauté (RB_NAV=0 ou RB_MATCH=0)"; return 0; fi
  [ -f /opt/node22/lib/node_modules/playwright/index.mjs ] || (cd "$TR" && node -e "require.resolve('playwright')" >/dev/null 2>&1) \
    || { echo "robot du match sauté (Playwright absent)"; return 0; }
  local port=$((PORT + 1))
  (cd "$TR/build" && exec python3 -m http.server "$port" >/dev/null 2>&1) & local srv=$!
  sleep 1
  local out; out=$(cd "$TR" && timeout 1500 node verif_match.mjs "$LOG/match" "$port" 2>&1); local rc=$?
  kill $srv 2>/dev/null
  echo "$out" | tail -40
  [ $rc = 0 ] && ! grep -q "ÉCHEC" <<<"$out" && echo "robot du match : $(grep -c '^OK' <<<"$out") contrôles OK"
}

duel() {   # robot du duel entre amis (train/verif_duel.mjs) : deux navigateurs, serveur PeerJS local
  if [ "${RB_NAV:-1}" = 0 ] || [ "${RB_DUEL:-1}" = 0 ]; then echo "robot du duel sauté (RB_NAV=0 ou RB_DUEL=0)"; return 0; fi
  local pj=""
  for c in "$TR/node_modules/.bin/peerjs" /tmp/claude-0/peersrv/node_modules/.bin/peerjs; do [ -x "$c" ] && pj="$c" && break; done
  [ -n "$pj" ] || { echo "robot du duel sauté (serveur PeerJS absent : npm install peer@1.0.2 dans riftbound/train)"; return 0; }
  local wport=$((PORT + 2)) pport=$((PORT + 3))
  "$pj" --port "$pport" --host 127.0.0.1 --path /myapp >/dev/null 2>&1 & local psrv=$!
  (cd "$TR/build" && exec python3 -m http.server "$wport" >/dev/null 2>&1) & local wsrv=$!
  sleep 2
  local out; out=$(cd "$TR" && timeout 1800 node verif_duel.mjs "$LOG/duel" "$wport" "$pport" 2>&1); local rc=$?
  kill $psrv $wsrv 2>/dev/null
  echo "$out" | tail -40
  [ $rc = 0 ] && ! grep -q "ÉCHEC" <<<"$out" && echo "robot du duel : $(grep -c '^OK' <<<"$out") contrôles OK"
}

case "$QUOI" in
  moteur) etape wiki wiki; etape tests tests; etape fuzz fuzz; etape hasard hasard ;;
  table)  etape table table; etape navigateur navigateur; etape cache cache; etape match match; etape duel duel ;;
  tout)   etape wiki wiki; etape tests tests; etape fuzz fuzz; etape hasard hasard; etape table table; etape navigateur navigateur; etape cache cache; etape match match; etape duel duel ;;
  *) sed -n '2,10p' "${BASH_SOURCE[0]}"; exit 2 ;;
esac

echo; echo "=== Porte d'intégration ($QUOI) ==="; printf '%s\n' "${RES[@]}"
[ $ECHEC = 0 ] && echo "=== TOUT PASSE ===" || echo "=== ÉCHEC : ne pas intégrer ==="
exit $ECHEC
