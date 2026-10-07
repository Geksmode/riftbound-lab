#!/usr/bin/env python3
"""Collecte le retex (retour d'experience terrain) d'une ou plusieurs legendes sur riftbound.gg.
Usage : python3 retex_fetch.py Akali LeBlanc [--since 2026-08-01] [--max 25] [--out dossier]
Passe par l'API WordPress publique (riftbound.gg/wp-json), seule partie du site lisible sans JS.
Sortie : un .md par article pertinent (paragraphes citant les legendes, lignes de tier list,
records type 11-2-1, slugs des decks embarques) + un index retex_sources.md."""
import argparse, html, json, re, sys, urllib.request, urllib.parse
from pathlib import Path

API = "https://riftbound.gg/wp-json/wp/v2/posts"
CARDS = Path(__file__).resolve().parent.parent / "cards" / "cards_all_printings.csv"


def get(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "riftbound-lab"}), timeout=30) as r:
        return json.load(r)


def codes():
    import csv
    m = {}
    if CARDS.exists():
        for r in csv.DictReader(open(CARDS, encoding="utf-8")):
            m.setdefault(f"{r['set_code']}-{r['collector_number']}".upper(), r["name"])
    return m


def text(rendered, names):
    c = re.sub(r"<style.*?</style>|<script.*?</script>", "", rendered, flags=re.S)
    decks = re.findall(r'data-deck="([^"]+)"', c)
    c = re.sub(r"</p>|<br ?/?>|</h\d>|</li>|</tr>", "\n", c)
    t = html.unescape(re.sub(r"<[^>]+>", " ", c))
    t = re.sub(r"[ \t]+", " ", t)
    m = codes()
    t = re.sub(r"\b([A-Z]{3}-\d{3})\b", lambda x: f"{m.get(x.group(1), x.group(1))} ({x.group(1)})", t)
    lines = [l.strip() for l in t.split("\n") if l.strip()]
    pat = re.compile("|".join(re.escape(n) for n in names), re.I)
    keep = []
    for i, l in enumerate(lines):
        if pat.search(l):
            ctx = lines[i:i + 3] if len(l) < 40 else [l]   # titre court (tier/record) -> garder 2 lignes de contexte
            keep.append(" / ".join(ctx))
    return keep, [d for d in decks if pat.search(d)]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("legends", nargs="+")
    ap.add_argument("--since", default="2000-01-01")
    ap.add_argument("--max", type=int, default=25)
    ap.add_argument("--out", default="retex_raw")
    a = ap.parse_args()
    out = Path(a.out); out.mkdir(parents=True, exist_ok=True)
    seen = {}
    for n in a.legends:
        q = urllib.parse.urlencode({"search": n, "per_page": a.max, "after": a.since + "T00:00:00",
                                    "_fields": "id,date,link,title"})
        for p in get(f"{API}?{q}"):
            seen[p["id"]] = p
    idx = ["# Sources retex riftbound.gg", "", f"Légendes : {', '.join(a.legends)} ; depuis {a.since}", ""]
    for pid, p in sorted(seen.items(), key=lambda kv: kv[1]["date"], reverse=True):
        full = get(f"{API}/{pid}?_fields=content")
        keep, decks = text(full["content"]["rendered"], a.legends)
        title = html.unescape(p["title"]["rendered"])
        f = out / f"{p['date'][:10]}_{pid}.md"
        f.write_text(f"# {title}\n{p['link']} ({p['date'][:10]})\n\n## Decks embarqués\n" +
                     "\n".join(f"- {d}" for d in decks) + "\n\n## Extraits\n" +
                     "\n".join(f"- {k}" for k in keep) + "\n", encoding="utf-8")
        idx.append(f"- {p['date'][:10]} [{title}]({p['link']}) : {len(keep)} extraits, {len(decks)} decks → {f.name}")
    (out / "retex_sources.md").write_text("\n".join(idx) + "\n", encoding="utf-8")
    print("\n".join(idx))


if __name__ == "__main__":
    sys.exit(main())
