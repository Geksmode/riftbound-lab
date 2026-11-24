#!/usr/bin/env bash
# Série différentielle : N parties (graines 1..N), paires de paquets tirées par la graine ; affiche les écarts.
# rust/diff/serie.sh N [debut]
N=${1:-50}; S=${2:-1}; D=$(mktemp -d); ok=0; ko=0; err=0
mapfile -t K < <(python3 -c "import json;[print(d['file'][:-4]) for d in json.load(open('riftbound/decks/decks.json'))]")
for ((i=S; i<S+N; i++)); do
  a=${K[$(( (i*7919) % ${#K[@]} ))]}; b=${K[$(( (i*104729+13) % ${#K[@]} ))]}
  python3 rust/diff/trace.py $i $a $b > $D/p.txt 2>$D/pe.txt || { echo "PYERR $i $a $b: $(tail -1 $D/pe.txt)"; err=$((err+1)); continue; }
  ./rust/target/release/trace $i $a $b > $D/r.txt 2>$D/re.txt || { echo "RSERR $i $a $b: $(grep -m1 panicked -A1 $D/re.txt | tr '\n' ' ')"; err=$((err+1)); continue; }
  if cmp -s $D/p.txt $D/r.txt; then ok=$((ok+1)); else ko=$((ko+1)); l=$(cmp $D/p.txt $D/r.txt | awk '{print $NF}'); echo "DIFF $i $a $b ligne $l"; fi
done
echo "identiques=$ok differentes=$ko erreurs=$err"
