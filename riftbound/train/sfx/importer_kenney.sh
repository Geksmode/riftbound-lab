#!/usr/bin/env bash
# Importe les vrais bruits de cartes du paquet Kenney « Casino Audio » (CC0, https://kenney.nl/assets/casino-audio)
# dans sfx/cartes/ : chaque fichier card*.ogg / chip*.ogg devient un MP3 mono, silence de tête coupé, et
# sfx/cartes/liste.json donne le nombre de variantes de chaque son (card-place → 4 : card-place-1..4.mp3).
# Usage : bash sfx/importer_kenney.sh <kenney_casino-audio.zip>
set -e
cd "$(dirname "$0")"
Z=$(realpath "$1"); [ -f "$Z" ] || { echo "zip introuvable : $1"; exit 1; }
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
(cd "$T" && unzip -q "$Z")
rm -rf cartes && mkdir cartes
find "$T" -iname 'License*.txt' -exec cp {} cartes/LICENSE-KENNEY.txt \; -quit
n=0
while IFS= read -r f; do
  b=$(basename "$f" .ogg)
  ffmpeg -nostdin -loglevel error -y -i "$f" -ac 1 -ar 44100 \
    -af "silenceremove=start_periods=1:start_threshold=-50dB" -codec:a libmp3lame -b:a 80k "cartes/$b.mp3"
  n=$((n + 1))
done < <(find "$T" -type f \( -iname 'card*.ogg' -o -iname 'chip*.ogg' \) | sort)
[ "$n" -gt 0 ] || { echo "aucun card*.ogg ni chip*.ogg dans le zip"; exit 1; }
python3 - <<'P'
import os, re, json
c = {}
for f in sorted(os.listdir("cartes")):
    m = re.fullmatch(r"(.+?)(?:-(\d+))?\.mp3", f)
    if m: c[m[1]] = max(c.get(m[1], 0), int(m[2] or 0))
json.dump(c, open("cartes/liste.json", "w"), indent=0, sort_keys=True)
print(json.dumps(c, sort_keys=True))
P
echo "$n fichiers, $(du -sk cartes | cut -f1) Ko dans sfx/cartes/"
