import pathlib
d = pathlib.Path(__file__).parent
ui = (d/'ui.html').read_text()
out = ui.replace('/*ENGINE*/', (d/'engine.js').read_text()).replace('/*PUZZLES*/', (d/'puzzles.js').read_text())
(pathlib.Path(__file__).resolve().parent / 'plateau-yi.html').write_text(out)
(d/'plateau-yi.html').write_text(out)
print(len(out))
