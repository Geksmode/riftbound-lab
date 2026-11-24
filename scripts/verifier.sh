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
#             RB_ROBOTS (robots navigateur lancés en même temps : défaut, un par cœur ; 1 = en série comme avant),
#             RB_PART=k/4 : seulement la k-ième des 4 parties des robots navigateur (CI : une machine par partie,
#             répartition dans PARTIE ci-dessous ; un robot qui n'y figure pas tourne dans la partie 1).
# Code de sortie 0 seulement si tout passe ; un résumé est imprimé à la fin.
set -uo pipefail
export PYTHONUTF8=1   # Windows lit en cp1252 par défaut : wiki_lint et test_duel plantent sur les accents
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
  [verif_replay]=2 [verif_nashor]=2 [verif_espace]=2 [verif_dash]=2 [verif_flow]=2 [verif_cibles]=2 [verif_drapeau]=2 [verif_showoff]=2 [verif_fizz]=2 [verif_multi]=2
  [verif_fleches]=3 [verif_deploy]=3 [verif_reactif]=3 [verif_base]=3 [verif_cout]=3 [verif_revele]=3 [verif_repeat]=3 [verif_deckimg]=3 [verif_stats]=3
  [verif_duel]=4 [verif_match]=4 [verif_mobile]=4 [verif_banish]=4 [verif_predict]=4 [verif_mulligan]=4 [verif_gold]=4 [verif_regard]=4
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
RS="$ROOT/rust"; BIN="$RS/target/release"
# Moteur Rust (bascule du 2026-11-24 : plus de moteur Python). Les tests comprennent la non-régression contre le
# moteur Python de référence (rust/tests/golden.rs : 300 parties aux journaux identiques, MD5 figés avant sa suppression).
tests()  { cd "$RS" || return 1
           # WebAssembly : même RB_ENGINE que build.sh, sinon cargo le recompile une seconde fois à l'étape table (≈ 70 s)
           cargo build --release -q && RB_ENGINE="$(bash "$TR/engine_id.sh")" cargo build --release -q --lib --target wasm32-unknown-unknown || return 1
           RB_GOLDEN_IA=1 cargo test --release -q > "$LOG/cargo.txt" 2>&1; local rc=$?
           grep -E "FAILED|panicked" "$LOG/cargo.txt" | head -20
           local ok ko; ok=$(grep -E "^test result" "$LOG/cargo.txt" | awk '{s+=$4} END {print s+0}')
           ko=$(grep -E "^test result" "$LOG/cargo.txt" | awk '{s+=$6} END {print s+0}')
           echo "cargo test : $ok tests passés, $ko échec(s) (dont 312 parties identiques à la référence Python)"
           [ "$rc" = 0 ] && [ "$ko" = 0 ]; }
empreinte() { cd "$E" && "$BIN/empreinte"; }   # décisions de l'IA à graines fixes = empreinte.json (--maj si le jeu change exprès)
MODULES="battlefields body calm chaos fury legends mind order radiance"   # rust/src/cardsets (ordre de register_all)
fuzz()   {   # un processus par paquet, JOBS à la fois ; bilan dans l'ordre des paquets
  cd "$E" || return 1
  local out bad=0
  printf '%s\n' cards $MODULES | xargs -P "$JOBS" -I{} sh -c \
    'if [ "$1" = cards ]; then "$4/fuzz_cards" "$2"; else "$4/fuzz_cards" "$2" --module "$1"; fi > "$3/fuzz_$1.txt" 2>&1' _ {} "$FUZZ" "$LOG" "$BIN"
  for m in cards $MODULES; do
    out=$(grep -E "^[0-9]+ games, [0-9]+ exceptions" "$LOG/fuzz_$m.txt" | tail -1)
    echo "$m : ${out:-pas de bilan}"
    case "$out" in *" 0 exceptions"*) ;; *) bad=1; tail -20 "$LOG/fuzz_$m.txt";; esac
  done
  echo "fuzz : $FUZZ parties par paquet, $( [ $bad = 0 ] && echo '0 exception partout' || echo 'EXCEPTIONS' )"
  return $bad
}
hasard() {
  cd "$E" || return 1
  local out; out=$(RB_JOBS="$JOBS" "$BIN/t_rand" "$RAND" 2>&1); local rc=$?
  out=$(grep -E "^[0-9]+ parties" <<<"$out" | head -1); echo "${out:-pas de bilan}"
  [ $rc = 0 ] && grep -q "erreurs 0" <<<"$out"
}
table()  {
  cd "$TR" || return 1
  [ -f vendor-cache/peerjs/package/dist/peerjs.min.js ] || bash fetch_vendor.sh >/dev/null || return 1
  bash build.sh | tail -1
}
# Robots navigateur : tous lancés par robots(), NROB à la fois (RB_ROBOTS, défaut : un par cœur), sur un seul serveur
# de pages. Chaque robot ouvre son propre Chromium et ne partage rien avec les autres. Les plus longs partent d'abord.
# Format : groupe:robot:délai(s). Le groupe est l'étape du résumé (navigateur, cache, match, anglais, duel).
ROBOTS="anglais:verif_anglais:2400 cache:verif_replay:1200 cache:verif_fleches:900 match:verif_match:1500 duel:verif_duel:1800
  cache:verif_deploy:1500 cache:verif_nashor:1500 cache:verif_tuto:900 cache:verif_reactif:1500 navigateur:verif_mobile:900
  cache:verif_espace:600 cache:verif_charge:600 cache:verif_base:900 cache:verif_son:600 cache:verif_banish:900
  cache:verif_dash:600 cache:verif_hidden:900 cache:verif_cout:900 cache:verif_predict:600 cache:verif_flow:600
  cache:verif_mulligan:600 cache:verif_vs:600 cache:verif_revele:600 cache:verif_cibles:600 cache:verif_drapeau:900
  cache:verif_showoff:600 cache:verif_fizz:600 cache:verif_repeat:900 cache:verif_gold:600 cache:verif_regard:900
  cache:verif_multi:600 cache:verif_deckimg:600 cache:verif_stats:600"
NROB="${RB_ROBOTS:-$JOBS}"
havepw() { [ -f /opt/node22/lib/node_modules/playwright/index.mjs ] || (cd "$TR" && node -e "require.resolve('playwright')" >/dev/null 2>&1); }
peerjs() { local c; for c in "$TR/node_modules/.bin/peerjs" /tmp/claude-0/peersrv/node_modules/.bin/peerjs; do [ -x "$c" ] && echo "$c" && return 0; done; return 1; }
saute() {   # saute <groupe> <robot> : pourquoi ce robot ne tourne pas ici (rien = il tourne)
  [ "${RB_NAV:-1}" = 0 ] && { echo "sauté (RB_NAV=0)"; return; }
  [ "$1" = match ] && [ "${RB_MATCH:-1}" = 0 ] && { echo "sauté (RB_MATCH=0)"; return; }
  [ "$1" = anglais ] && [ "${RB_EN:-1}" = 0 ] && { echo "sauté (RB_EN=0)"; return; }
  [ "$1" = duel ] && [ "${RB_DUEL:-1}" = 0 ] && { echo "sauté (RB_DUEL=0)"; return; }
  mine "$2" || { echo "dans une autre partie ($PART)"; return; }
  havepw || { echo "sauté (Playwright absent)"; return; }
  [ "$1" = duel ] && ! peerjs >/dev/null && { echo "sauté (serveur PeerJS absent : npm install peer@1.0.2 dans riftbound/train)"; return; }
  return 0
}
un_robot() {   # un_robot <groupe> <robot> <délai> : sortie, code et durée dans $LOG/rob/
  local g="$1" r="$2" t="$3" t0=$SECONDS rc
  case "$g" in
    match) (cd "$TR" && timeout "$t" node "$r.mjs" "$LOG/match" "$PORT") ;;
    duel)  (cd "$TR" && timeout "$t" node "$r.mjs" "$LOG/duel" "$PORT" "$((PORT + 3))") ;;
    *)     (cd "$TR" && timeout "$t" node "$r.mjs" "$LOG" "$PORT") ;;
  esac > "$LOG/rob/$r.txt" 2>&1; rc=$?
  echo "$rc $((SECONDS - t0))" > "$LOG/rob/$r.rc"
}
robots() {   # lance tous les robots qui tournent ici, NROB à la fois ; le bilan par étape est fait par bilan()
  mkdir -p "$LOG/rob"
  local x g r t afaire=() psrv="" srv
  for x in $ROBOTS; do IFS=: read -r g r t <<<"$x"; [ -z "$(saute "$g" "$r")" ] && afaire+=("$x"); done
  [ ${#afaire[@]} = 0 ] && { echo "aucun robot navigateur à lancer ici"; return 0; }
  (cd "$TR/build" && exec python3 -m http.server "$PORT" >/dev/null 2>&1) & srv=$!
  if printf '%s\n' "${afaire[@]}" | grep -q "^duel:"; then
    "$(peerjs)" --port "$((PORT + 3))" --host 127.0.0.1 --path /myapp >/dev/null 2>&1 & psrv=$!
  fi
  sleep 2
  for x in "${afaire[@]}"; do   # NROB robots à la fois : dès qu'un finit, le suivant part
    while [ "$(jobs -rp | grep -cvxE "$srv|${psrv:-x}")" -ge "$NROB" ]; do wait -n 2>/dev/null || sleep 1; done
    IFS=: read -r g r t <<<"$x"; un_robot "$g" "$r" "$t" &
  done
  while [ "$(jobs -rp | grep -cvxE "$srv|${psrv:-x}")" -gt 0 ]; do wait -n 2>/dev/null || sleep 1; done
  kill $srv $psrv 2>/dev/null
  for x in "${afaire[@]}"; do IFS=: read -r g r t <<<"$x"; read -r rc t < "$LOG/rob/$r.rc"; printf '%4s s  %s%s\n' "$t" "$r" "$([ "$rc" = 0 ] || echo "  (code $rc)")"; done | sort -rn
  echo "${#afaire[@]} robots, $NROB à la fois"
}
bilan() {   # bilan <groupe> <libellé> : résultat d'une étape d'après les sorties de robots() (mêmes contrôles qu'en série)
  local groupe="$1" lib="$2" x g r t rc d n=0 s=0 ko=0 ok=0 pourquoi="" f="$LOG/$1.txt"
  : > "$f"
  for x in $ROBOTS; do IFS=: read -r g r t <<<"$x"; [ "$g" = "$groupe" ] || continue
    pourquoi=$(saute "$g" "$r"); [ -n "$pourquoi" ] && continue
    n=$((n + 1))
    if [ -f "$LOG/rob/$r.rc" ]; then read -r rc d < "$LOG/rob/$r.rc"; else rc=99; d=0; echo "$r : pas lancé" >> "$f"; fi
    s=$((s + d)); cat "$LOG/rob/$r.txt" >> "$f" 2>/dev/null
    ok=$((ok + $(grep -c '^OK' "$LOG/rob/$r.txt" 2>/dev/null)))
    if [ "$rc" != 0 ] || grep -q "ÉCHEC" "$LOG/rob/$r.txt" 2>/dev/null; then ko=1; echo "$r : ÉCHEC (code $rc)" >> "$f"; fi
  done
  if [ $n = 0 ]; then RES+=("OK     $groupe (0 s) : $lib ${pourquoi:-sauté}"); return; fi
  if [ $ko = 0 ]; then RES+=("OK     $groupe ($s s de robots) : $lib${PART:+ de la partie $PART} : $n robot(s), $ok contrôles OK")
  else RES+=("ÉCHEC  $groupe ($s s de robots) : voir $f"); ECHEC=1
    grep -E "ÉCHEC|FAIL|Traceback|Error" "$f" | head -30 >&2; tail -15 "$f" >&2; fi
}
navig() {   # tous les robots, puis une ligne de résumé par étape, comme quand ils tournaient en série
  etape robots robots
  bilan navigateur "robot navigateur (téléphone 360x740)"
  bilan cache "robots de la table (Hidden, main révélée, charge, ciblage, Flow, sélection contre l IA, flèches, Predict, replay, tutoriel, sons, réactivité, Baron Nashor, Deploy…)"
  bilan match "robot du match BO3"
  bilan anglais "robot de la version anglaise"
  bilan duel "robot du duel entre amis"
}

case "$QUOI" in
  moteur) etape wiki wiki; etape tests tests; etape empreinte empreinte; etape fuzz fuzz; etape hasard hasard ;;
  table)  etape table table; navig ;;
  tout)   etape wiki wiki; etape tests tests; etape empreinte empreinte; etape fuzz fuzz; etape hasard hasard; etape table table; navig ;;
  *) sed -n '2,10p' "${BASH_SOURCE[0]}"; exit 2 ;;
esac

echo; echo "=== Porte d'intégration ($QUOI) ==="; printf '%s\n' "${RES[@]}"
[ $ECHEC = 0 ] && echo "=== TOUT PASSE ===" || echo "=== ÉCHEC : ne pas intégrer ==="
# État noté pour les hooks de .claude/ (push et fin de session) : voir scripts/porte_etat.mjs
command -v node >/dev/null 2>&1 && (cd "$ROOT" && node scripts/porte_etat.mjs ecrire "$QUOI" "$ECHEC" "$PART" "${RB_NAV:-1}" "$FUZZ" "$RAND") || true
exit $ECHEC
