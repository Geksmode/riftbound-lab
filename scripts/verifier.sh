#!/usr/bin/env bash
# Porte d'intégration : rien n'entre dans une branche de travail ni dans main sans passer par là.
#
#   scripts/verifier.sh            tout : wiki, moteur, table (+ robot navigateur si Playwright et Chromium sont là)
#   scripts/verifier.sh moteur     wiki + tests + empreinte du jeu de l'IA + fuzz de chaque paquet + parties aléatoires
#   scripts/verifier.sh table      construction de la table (+ robots navigateur : téléphone, Hidden, match, duel)
#
# Variables : RB_FUZZ (parties par paquet, défaut 200), RB_RAND (parties t_rand, défaut 30),
#             RB_NAV=0 pour sauter les robots navigateur, RB_MATCH=0 pour sauter le robot du match BO3 (~6 min),
#             RB_DUEL=0 pour sauter le robot du duel entre amis (serveur PeerJS local : paquet npm peer@1.0.2),
#             RB_EN=0 pour sauter le robot de la version anglaise (~10 min),
#             RB_PORT (défaut 8771), RB_JOBS (processus en parallèle pour tests, fuzz et t_rand : défaut, un par cœur),
#             RB_PART=k/4 : seulement la k-ième des 4 parties des robots navigateur (CI : une machine par partie,
#             répartition dans PARTIE ci-dessous ; un robot qui n'y figure pas tourne dans la partie 1).
# Code de sortie 0 seulement si tout passe ; un résumé est imprimé à la fin.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
E="$ROOT/riftbound/engine"; TR="$ROOT/riftbound/train"
QUOI="${1:-tout}"; FUZZ="${RB_FUZZ:-200}"; RAND="${RB_RAND:-30}"; PORT="${RB_PORT:-8771}"
LOG="$(mktemp -d)"; RES=(); ECHEC=0; JOBS="${RB_JOBS:-$(nproc 2>/dev/null || echo 2)}"

# Répartition des robots navigateur en 4 parties de durée voisine (secondes mesurées ici le 2026-10-27, machine 4 cœurs)
#   1 : anglais 138, tuto 60, charge 26, son 14, hidden 10, vs 8                        ≈ 256 s
#   2 : replay 119, nashor 72, espace 42, dash 11, flow 9, cibles 7 (+ drapeau, 2026-10-28, non mesuré)  ≈ 260 s
#   3 : fleches 95, deploy 75, reactif 51, base 21, cout 10, revele 7                     ≈ 259 s
#   4 : duel (~90, CI seulement), match 87, mobile 47, banish 13, predict 10, mulligan 9  ≈ 256 s
declare -A PARTIE=(
  [verif_anglais]=1 [verif_tuto]=1 [verif_charge]=1 [verif_son]=1 [verif_hidden]=1 [verif_vs]=1
  [verif_replay]=2 [verif_nashor]=2 [verif_espace]=2 [verif_dash]=2 [verif_flow]=2 [verif_cibles]=2 [verif_drapeau]=2
  [verif_fleches]=3 [verif_deploy]=3 [verif_reactif]=3 [verif_base]=3 [verif_cout]=3 [verif_revele]=3
  [verif_duel]=4 [verif_match]=4 [verif_mobile]=4 [verif_banish]=4 [verif_predict]=4 [verif_mulligan]=4
)
PART="${RB_PART:-}"
if [ -n "$PART" ] && ! [[ "$PART" =~ ^[1-4]/4$ ]]; then echo "RB_PART doit valoir 1/4, 2/4, 3/4 ou 4/4" >&2; exit 2; fi
mine() { [ -z "$PART" ] || [ "${PARTIE[$1]:-1}" = "${PART%/*}" ]; }   # ce robot tourne-t-il dans cette partie ?

etape() {   # etape <nom> <commande...> : lance, garde la sortie, note OK / ÉCHEC
  local nom="$1"; shift
  local t0=$SECONDS
  if "$@" > "$LOG/$nom.txt" 2>&1; then RES+=("OK     $nom ($((SECONDS - t0)) s) : $(tail -1 "$LOG/$nom.txt")")
  else RES+=("ÉCHEC  $nom ($((SECONDS - t0)) s) : voir $LOG/$nom.txt"); ECHEC=1
    grep -E "ÉCHEC|FAIL|Traceback|Error" "$LOG/$nom.txt" | head -30 >&2   # les contrôles ratés, même loin de la fin
    tail -15 "$LOG/$nom.txt" >&2; fi
}

wiki()   { python3 "$ROOT/scripts/wiki_lint.py"; }
tests()  { cd "$E" && RB_JOBS="$JOBS" python3 test_all.py; }
empreinte() { cd "$E" && python3 empreinte.py; }   # décisions de l'IA à graines fixes = empreinte.json (--maj si le jeu change exprès)
fuzz()   {   # un processus par paquet, JOBS à la fois ; bilan dans l'ordre des paquets
  cd "$E" || return 1
  local mods out bad=0
  mods=$(python3 -c "import re; s=open('cardsets/__init__.py').read(); print(' '.join(re.findall(r'[\"\x27]([a-z0-9_]+)[\"\x27]', s[s.index('MODULES'):])))")
  printf '%s\n' cards.py $mods | xargs -P "$JOBS" -I{} sh -c \
    'if [ "$1" = cards.py ]; then python3 fuzz_cards.py "$2"; else python3 fuzz_cards.py "$2" --module "$1"; fi > "$3/fuzz_$1.txt" 2>&1' _ {} "$FUZZ" "$LOG"
  for m in cards.py $mods; do
    out=$(grep -E "^[0-9]+ games, [0-9]+ exceptions" "$LOG/fuzz_$m.txt" | tail -1)
    echo "$m : ${out:-pas de bilan}"
    case "$out" in *" 0 exceptions"*) ;; *) bad=1; tail -20 "$LOG/fuzz_$m.txt";; esac
  done
  echo "fuzz : $FUZZ parties par paquet, $( [ $bad = 0 ] && echo '0 exception partout' || echo 'EXCEPTIONS' )"
  return $bad
}
hasard() {
  cd "$E" || return 1
  local out; out=$(RB_JOBS="$JOBS" python3 ../train/t_rand.py "$RAND" 2>&1); local rc=$?
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
  mine verif_mobile || { echo "robot navigateur : dans une autre partie ($PART)"; return 0; }
  (cd "$TR/build" && exec python3 -m http.server "$PORT" >/dev/null 2>&1) & local srv=$!
  sleep 1
  local out; out=$(cd "$TR" && timeout 900 node verif_mobile.mjs "$LOG" "$PORT" 2>&1); local rc=$?
  kill $srv 2>/dev/null
  echo "$out"
  [ $rc = 0 ] && ! grep -q "^ÉCHEC" <<<"$out" && echo "robot navigateur : $(grep -c '^OK' <<<"$out") contrôles OK (captures dans $LOG)"
}

cache() {   # robots : verif_hidden.mjs (cacher, jouer depuis la face cachée), verif_revele.mjs (main révélée), verif_charge.mjs (téléphone chargé), verif_cibles.mjs (ciblage par liste), verif_flow.mjs (Flow depuis la défausse), verif_vs.mjs (sélection contre l IA), verif_fleches.mjs (flèches), verif_predict.mjs (Predict), verif_replay.mjs (mode replay), verif_tuto.mjs (tutoriel), verif_dash.mjs (tableau de bord du menu), verif_espace.mjs (barre d espace), verif_son.mjs (sons et menu ♪), verif_mulligan.mjs (lire les cartes du mulligan), verif_reactif.mjs (page réactive pendant que l IA réfléchit : moteur dans un Web Worker), verif_nashor.mjs (Baron Nashor : 3e battlefield), verif_deploy.mjs (équipement [Deploy] posé sur un battlefield), verif_base.mjs (base serrée ouverte en grand au toucher), verif_banish.mjs (pile des bannies, jouer la carte de Runeblade Anvil), verif_cout.mjs (coûts supplémentaires optionnels : une ligne par façon de payer), verif_drapeau.mjs (bouton « Erreur de l IA » : marque, précision, coups de l IA gardés)
  if [ "${RB_NAV:-1}" = 0 ]; then echo "robot Hidden sauté (RB_NAV=0)"; return 0; fi
  [ -f /opt/node22/lib/node_modules/playwright/index.mjs ] || (cd "$TR" && node -e "require.resolve('playwright')" >/dev/null 2>&1) \
    || { echo "robot Hidden sauté (Playwright absent)"; return 0; }
  local port=$((PORT + 4))
  (cd "$TR/build" && exec python3 -m http.server "$port" >/dev/null 2>&1) & local srv=$!
  sleep 1
  local out="" o rc=0 r t n=0
  for r in verif_hidden:900 verif_revele:600 verif_charge:600 verif_cibles:600 verif_flow:600 verif_vs:600 verif_fleches:900 \
           verif_predict:600 verif_replay:1200 verif_tuto:900 verif_dash:600 verif_espace:600 verif_son:600 verif_mulligan:600 \
           verif_reactif:1500 verif_nashor:1500 verif_deploy:1500 verif_base:900 verif_banish:900 verif_cout:900 verif_drapeau:900; do
    t=${r#*:}; r=${r%%:*}
    mine "$r" || continue
    n=$((n + 1))
    o=$(cd "$TR" && timeout "$t" node "$r.mjs" "$LOG" "$port" 2>&1); rc=$?
    out+="$o"$'\n'
    [ $rc = 0 ] || break                                    # le premier échec arrête la série, comme avant
  done
  kill $srv 2>/dev/null
  echo "$out"
  [ $rc = 0 ] && ! grep -q "ÉCHEC" <<<"$out" && echo "$n robots (Hidden, main révélée, charge, ciblage, Flow, sélection contre l IA, flèches, Predict, replay, tutoriel, sons, réactivité, Baron Nashor, Deploy…)${PART:+ de la partie $PART} : $(grep -c '^OK' <<<"$out") contrôles OK"
}

match() {   # robot du match BO3 et du sideboard (train/verif_match.mjs), en 360x740 et 1400x900
  if [ "${RB_NAV:-1}" = 0 ] || [ "${RB_MATCH:-1}" = 0 ]; then echo "robot du match sauté (RB_NAV=0 ou RB_MATCH=0)"; return 0; fi
  mine verif_match || { echo "robot du match : dans une autre partie ($PART)"; return 0; }
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

anglais() {   # robot de la version anglaise (train/verif_anglais.mjs) : toute la table avec ?lang=en, aucun texte français
  if [ "${RB_NAV:-1}" = 0 ] || [ "${RB_EN:-1}" = 0 ]; then echo "robot de la version anglaise sauté (RB_NAV=0 ou RB_EN=0)"; return 0; fi
  mine verif_anglais || { echo "robot de la version anglaise : dans une autre partie ($PART)"; return 0; }
  [ -f /opt/node22/lib/node_modules/playwright/index.mjs ] || (cd "$TR" && node -e "require.resolve('playwright')" >/dev/null 2>&1) \
    || { echo "robot de la version anglaise sauté (Playwright absent)"; return 0; }
  local port=$((PORT + 5))
  (cd "$TR/build" && exec python3 -m http.server "$port" >/dev/null 2>&1) & local srv=$!
  sleep 1
  local out; out=$(cd "$TR" && timeout 2400 node verif_anglais.mjs "$LOG" "$port" 2>&1); local rc=$?
  kill $srv 2>/dev/null
  echo "$out" | tail -40
  [ $rc = 0 ] && ! grep -q "ÉCHEC" <<<"$out" && echo "robot de la version anglaise : $(grep -c '^OK' <<<"$out") contrôles OK"
}

duel() {   # robot du duel entre amis (train/verif_duel.mjs) : deux navigateurs, serveur PeerJS local
  if [ "${RB_NAV:-1}" = 0 ] || [ "${RB_DUEL:-1}" = 0 ]; then echo "robot du duel sauté (RB_NAV=0 ou RB_DUEL=0)"; return 0; fi
  mine verif_duel || { echo "robot du duel : dans une autre partie ($PART)"; return 0; }
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
  moteur) etape wiki wiki; etape tests tests; etape empreinte empreinte; etape fuzz fuzz; etape hasard hasard ;;
  table)  etape table table; etape navigateur navigateur; etape cache cache; etape match match; etape anglais anglais; etape duel duel ;;
  tout)   etape wiki wiki; etape tests tests; etape empreinte empreinte; etape fuzz fuzz; etape hasard hasard; etape table table; etape navigateur navigateur; etape cache cache; etape match match; etape anglais anglais; etape duel duel ;;
  *) sed -n '2,10p' "${BASH_SOURCE[0]}"; exit 2 ;;
esac

echo; echo "=== Porte d'intégration ($QUOI) ==="; printf '%s\n' "${RES[@]}"
[ $ECHEC = 0 ] && echo "=== TOUT PASSE ===" || echo "=== ÉCHEC : ne pas intégrer ==="
exit $ECHEC
