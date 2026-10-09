// exp_temps.py dans Pyodide (wasm 32 bits, comme la table) : node exp_temps.mjs <graines> <versions...>
// Utilise riftbound/train/pyodide-cache (train/fetch_pyodide.sh le télécharge).
import { createRequire } from "module";
import path from "path";
import { fileURLToPath } from "url";
const require = createRequire(import.meta.url);
const here = path.dirname(fileURLToPath(import.meta.url));
const PK = path.join(here, "..", "train", "pyodide-cache", "package") + "/";
const { loadPyodide } = require(PK + "pyodide.js");
const py = await loadPyodide({ indexURL: PK });
py.FS.mkdirTree("/rb");
py.FS.mount(py.FS.filesystems.NODEFS, { root: path.join(here, "..") }, "/rb");
py.setStdout({ batched: (s) => console.log(s) });
py.globals.set("ARGS", py.toPy(process.argv.slice(2)));
await py.runPythonAsync(`
import sys, runpy
sys.argv = ["/rb/engine/exp_temps.py"] + list(ARGS)
runpy.run_path("/rb/engine/exp_temps.py", run_name="__main__")
`);
