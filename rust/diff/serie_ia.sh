#!/usr/bin/env bash
# Série différentielle de l'IA : N parties (graines S..S+N-1) jouées par l'IA par défaut de la table des deux côtés
# (trace.py --ia), paires de paquets tirées par la graine comme serie.sh ; une graine sur 5 joue Akali Gorica contre
# LeBlanc (IQ#5). Parties en parallèle (P processus). Affiche les écarts.
# rust/diff/serie_ia.sh N [debut] [P]
N=${1:-20}; S=${2:-1}; P=${3:-4}; D=$(mktemp -d)
export D
one() {
  i=$1
  mapfile -t K < <(python3 -c "import json;[print(d['file'][:-4]) for d in json.load(open('riftbound/decks/decks.json'))]")
  if (( i % 5 == 0 )); then a=akali_gorica_rq-singapore; b=leblanc_gyatarina_ccs-iq5_1st
  else a=${K[$(( (i*7919) % ${#K[@]} ))]}; b=${K[$(( (i*104729+13) % ${#K[@]} ))]}; fi
  python3 rust/diff/trace.py $i $a $b --ia > $D/p$i.txt 2>$D/pe$i.txt || { echo "PYERR $i $a $b: $(tail -1 $D/pe$i.txt)"; return; }
  ./rust/target/release/trace $i $a $b --ia > $D/r$i.txt 2>$D/re$i.txt || { echo "RSERR $i $a $b: $(grep -m1 panicked -A1 $D/re$i.txt | tr '\n' ' ')"; return; }
  if cmp -s $D/p$i.txt $D/r$i.txt; then echo "OK $i $a $b"; rm -f $D/p$i.txt $D/r$i.txt $D/pe$i.txt $D/re$i.txt
  else echo "DIFF $i $a $b ligne $(cmp $D/p$i.txt $D/r$i.txt | awk '{print $NF}') ($D)"; fi
}
export -f one
seq $S $((S+N-1)) | xargs -P $P -I{} bash -c 'one {}' | tee $D/res.txt
echo "identiques=$(grep -c '^OK' $D/res.txt) differentes=$(grep -c '^DIFF' $D/res.txt) erreurs=$(grep -c 'ERR' $D/res.txt)"
