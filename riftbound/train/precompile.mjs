// Précompile le Python de la table (appelé par build.sh) : sans cela, Pyodide recompile à chaque chargement de la page
// la bibliothèque standard (livrée en .py) et tout le moteur, ce qui faisait l'essentiel de l'attente du menu.
// Le bytecode est produit par le Python de Pyodide lui-même (node), donc même version et même nombre magique (3.12).
//  - stdlib : les modules effectivement chargés (démarrage de Pyodide, import train, catalogue, une donne) passent en .pyc
//    dans build/pyodide/python_stdlib.b64.txt ; les autres restent en .py : un import imprévu marche toujours.
//  - moteur : build/py/bytecode.json = { "__pycache__/game.cpython-312.pyc": base64, ... }, écrit à côté des .py par le
//    worker. Ces .pyc sont « checked-hash » : Python compare le hachage du .py chargé et recompile s'il diffère, donc un
//    bytecode périmé ne peut jamais tourner. Les .py restent servis (traces d'erreur, engine_md5 des duels).
// Usage : node precompile.mjs <dossier build>   (après fetch_pyodide.sh)
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const PKG = path.join(HERE, "pyodide-cache/package/");
const OUT = path.resolve(process.argv[2] || path.join(HERE, "build"));
const { loadPyodide } = await import(path.join(PKG, "pyodide.mjs"));

// Le moteur tel que le worker le pose dans le navigateur (/rb/engine, /rb/cards, /rb/decks).
const setup = py => {
  py.FS.mkdirTree("/out"); py.mountNodeFS("/out", OUT);
  py.runPython(`
import shutil, os, sys
shutil.copytree('/out/py', '/rb/engine', ignore=shutil.ignore_patterns('__pycache__', 'bytecode.json'))
os.makedirs('/rb/cards'); os.makedirs('/rb/decks')
shutil.copy('/out/data/cards_unique.csv', '/rb/cards/'); shutil.copy('/out/data/decks.json', '/rb/decks/')
sys.path.insert(0, '/rb/engine')`);
};

const py = await loadPyodide({ indexURL: PKG });
setup(py);
py.FS.writeFile("/tmp/stdlib.zip", readFileSync(path.join(PKG, "python_stdlib.zip")));
const report = py.runPython(`
import sys, zipfile, importlib.util, marshal, base64, json, glob, py_compile, os
sys.dont_write_bytecode = True
import train
train.catalog(); train.default_decks(); train.new(seed=1)
ZIP = '/lib/python312.zip/'
hot = {m.__file__[len(ZIP):] for m in list(sys.modules.values()) if (getattr(m, '__file__', None) or '').startswith(ZIP)}

# stdlib : un .pyc « unchecked-hash » remplace le .py de chaque module chargé (zipimport l'utilise tel quel)
zi = zipfile.ZipFile('/tmp/stdlib.zip')
zo = zipfile.ZipFile('/tmp/out.zip', 'w', zipfile.ZIP_DEFLATED, compresslevel=9)
n = 0
for i in zi.infolist():
    d, name = zi.read(i), i.filename
    if name in hot:
        code = compile(d, ZIP + name, 'exec', dont_inherit=True)
        d = importlib.util.MAGIC_NUMBER + (1).to_bytes(4, 'little') + importlib.util.source_hash(d) + marshal.dumps(code)
        name += 'c'; n += 1
    zo.writestr(zipfile.ZipInfo(name, i.date_time) if i.is_dir() else name, d)
zo.close()
open('/out/pyodide/python_stdlib.b64.txt', 'w').write(base64.b64encode(open('/tmp/out.zip', 'rb').read()).decode())

# moteur : .pyc « checked-hash » compilés depuis les .py exacts du build, au chemin où le worker les pose
eng = {}
for f in sorted(glob.glob('/rb/engine/*.py') + glob.glob('/rb/engine/cardsets/*.py')):
    c = py_compile.compile(f, doraise=True, invalidation_mode=py_compile.PycInvalidationMode.CHECKED_HASH)
    eng[os.path.relpath(c, '/rb/engine')] = base64.b64encode(open(c, 'rb').read()).decode()
open('/out/py/bytecode.json', 'w').write(json.dumps(eng, sort_keys=True))
f"stdlib : {n} modules précompilés sur {sum(x.endswith('.py') for x in zi.namelist())}, moteur : {len(eng)} fichiers"
`);

// Contrôle : un Pyodide neuf avec la stdlib produite et le bytecode du moteur importe train et sert le catalogue.
const b64 = readFileSync(path.join(OUT, "pyodide/python_stdlib.b64.txt"), "utf8");
writeFileSync(path.join(OUT, "pyodide/.stdlib-check.zip"), Buffer.from(b64, "base64"));
const py2 = await loadPyodide({ indexURL: PKG, stdLibURL: path.join(OUT, "pyodide/.stdlib-check.zip") });
setup(py2);
const check = py2.runPython(`
import json, base64, os, sys, importlib.util
for k, v in json.load(open('/out/py/bytecode.json')).items():
    p = os.path.join('/rb/engine', k); os.makedirs(os.path.dirname(p), exist_ok=True); b = base64.b64decode(v)
    src = os.path.join(os.path.dirname(os.path.dirname(p)), os.path.basename(p).split('.')[0] + '.py')
    assert b[8:16] == importlib.util.source_hash(open(src, 'rb').read()), k   # bytecode du .py servi, pas d'un autre
    open(p, 'wb').write(b)
sys.dont_write_bytecode = True
import train
assert len(json.loads(train.catalog())['cards']) > 0
assert sys.modules['json'].__spec__.loader.__class__.__name__ == 'zipimporter'
str(len(sys.modules))`);
const { unlinkSync } = await import("node:fs");
unlinkSync(path.join(OUT, "pyodide/.stdlib-check.zip"));
console.log(report + " ; contrôle : train importé (" + check + " modules)");
