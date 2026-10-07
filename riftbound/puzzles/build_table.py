# Construit le plateau façon simulateur (table.html) dans le dossier de publication avec les images des cartes.
import json, pathlib, shutil
d = pathlib.Path(__file__).parent
out = pathlib.Path(__file__).resolve().parent / 'build_table'
(out/'img').mkdir(parents=True, exist_ok=True)
idx = json.loads((d.parent/'replays/img/index.json').read_text())
need = ["Kai'Sa, Survivor","Mournful Witness","Astral Heron","Pit Rookie","First Mate","Lonely Poro","Master Yi, Tempered","Ruin Runner",
        "Rengar, Trophy Hunter","Shuriken Flip","Falling Star","Back Off","Block","Discipline","Defy","Not So Fast","En Garde","Punch First",
        "Decree of Focus","Charm","Rampage","Akali, Rogue Assassin","Master Yi, Wuju Bladesman","Sigil of the Storm","Grove of the God-Willow",
        "Calm Rune","Fury Rune","Body Rune"]
m = {}
for n in need:
    if n in idx:
        src = d.parent/'replays'/idx[n]
        shutil.copy(src, out/'img'/src.name); m[n] = 'img/'+src.name
    else: print('missing image', n)
ui = (d/'table.html').read_text()
html = ui.replace('/*ENGINE*/', (d/'engine.js').read_text()).replace('/*PUZZLES*/', (d/'puzzles.js').read_text()).replace('/*IMGMAP*/{}', json.dumps(m, ensure_ascii=False))
(out/'index.html').write_text(html)
print(len(html), len(m), 'images')
