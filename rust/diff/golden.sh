#!/usr/bin/env bash
# Empreintes de référence du moteur Python (avant sa suppression) : MD5 du journal de trace.py pour des parties fixes.
# Le test rust/tests/golden.rs rejoue ces parties avec le moteur Rust et exige le même MD5.
# rust/diff/golden.sh > rust/tests/data/golden.txt   (format : mode graine paquet0 paquet1 md5)
mapfile -t K < <(python3 -c "import json;[print(d['file'][:-4]) for d in json.load(open('riftbound/decks/decks.json'))]")
pair() { local i=$1; if [ "$2" = ia ] && (( i % 5 == 0 )); then echo akali_gorica_rq-singapore leblanc_gyatarina_ccs-iq5_1st
  else echo ${K[$(( (i*7919) % ${#K[@]} ))]} ${K[$(( (i*104729+13) % ${#K[@]} ))]}; fi; }
export -f pair; export K_JSON
one() { m=$1; i=$2; set -- $(pair $i $m); fl=""; [ $m = ia ] && fl=--ia
  h=$(python3 rust/diff/trace.py $i $1 $2 $fl | md5sum | cut -c1-32); echo "$m $i $1 $2 $h"; }
for i in $(seq 7001 7300); do one alea $i; done
for i in $(seq 9001 9012); do echo "ia $i"; done | xargs -P 4 -L1 bash -c 'mapfile -t K < <(python3 -c "import json;[print(d[\"file\"][:-4]) for d in json.load(open(\"riftbound/decks/decks.json\"))]"); m=$0; i=$1; if (( i % 5 == 0 )); then a=akali_gorica_rq-singapore; b=leblanc_gyatarina_ccs-iq5_1st; else a=${K[$(( (i*7919) % ${#K[@]} ))]}; b=${K[$(( (i*104729+13) % ${#K[@]} ))]}; fi; h=$(python3 rust/diff/trace.py $i $a $b --ia | md5sum | cut -c1-32); echo "ia $i $a $b $h"' | sort -k2 -n
