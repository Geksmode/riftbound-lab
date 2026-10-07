#!/usr/bin/env python3
"""Contrôle de santé du wiki (voir docs/wiki/WIKI.md). Code de sortie 1 s'il y a un problème."""
import re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WIKI, MEM = ROOT / "docs" / "wiki", ROOT / "docs" / "memoire"
MAX_LINES, ERR = 80, []
LINK = re.compile(r"\[[^\]]*\]\(([^)#\s]+)(?:#[^)]*)?\)")


def err(msg):
    ERR.append(msg)


index = (WIKI / "index.md").read_text()
for p in sorted((WIKI / "pages").glob("*.md")):
    t = p.read_text()
    m = re.match(r"---\n(.*?)\n---\n", t, re.S)
    if not m:
        err(f"{p.relative_to(ROOT)} : pas d'en-tête")
    else:
        for k in ("titre", "resume", "maj", "sources"):
            if not re.search(rf"^{k}:\s*\S", m.group(1), re.M):
                err(f"{p.relative_to(ROOT)} : champ « {k} » manquant")
    if len(t.splitlines()) > MAX_LINES:
        err(f"{p.relative_to(ROOT)} : {len(t.splitlines())} lignes (plafond {MAX_LINES})")
    if f"pages/{p.name}" not in index:
        err(f"{p.relative_to(ROOT)} : absente de index.md")

for md in [WIKI / "index.md", WIKI / "WIKI.md", WIKI / "log.md", *sorted((WIKI / "pages").glob("*.md"))]:
    for target in LINK.findall(md.read_text()):
        if re.match(r"[a-z]+:", target):
            continue
        if not (md.parent / target).exists():
            err(f"{md.relative_to(ROOT)} : lien cassé vers {target}")

mem_index = (MEM / "MEMORY.md").read_text()
for p in sorted(MEM.glob("*.md")):
    if p.name != "MEMORY.md" and p.name not in mem_index:
        err(f"docs/memoire/{p.name} : absent de MEMORY.md")

if len(index.splitlines()) > 100:
    err(f"index.md : {len(index.splitlines())} lignes (plafond 100)")
log = (WIKI / "log.md").read_text()
dates = re.findall(r"^## \[(\d{4}-\d{2}-\d{2})\]", log, re.M)
if dates != sorted(dates):
    err("log.md : entrées non classées par date")

print("wiki : %d pages, %d problème(s)" % (len(list((WIKI / 'pages').glob('*.md'))), len(ERR)))
for e in ERR:
    print(" -", e)
sys.exit(1 if ERR else 0)
