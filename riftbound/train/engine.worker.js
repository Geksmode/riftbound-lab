// Moteur de la table dans un Web Worker : Pyodide et le module train tournent ici, hors du fil de la page. Une longue
// réflexion de l'IA ne gèle plus l'affichage (le navigateur ne dit plus « la page ne répond pas »).
// Protocole (messages) :
//   page → worker : { init: { ver, py } }                     charge Pyodide et les fichiers du moteur
//                   { id, fn, args }                          appelle train.<fn>(...args) (chaînes JSON en retour)
//                   { id, exec, get }                         robots de vérification : exec(code) dans train, renvoie
//                                                             json.dumps(train.<get>) si get est donné
//   worker → page : { ready: true } | { fatal: message } | { id, r } | { id, err }
// Les messages sont traités un par un, dans l'ordre d'arrivée : même graine = même partie.
importScripts("pyodide/pyodide.js");

let py = null, T = null;

async function init({ ver, py: files }) {
  // Tout part en même temps : stdlib, fichiers du moteur et bytecode se téléchargent pendant que Pyodide charge son wasm.
  // Les fichiers du moteur ont la version de la page en paramètre : sinon le navigateur peut garder l'ancien moteur en
  // cache (GitHub Pages : 10 min) sous une page neuve.
  const bytes = u => fetch(u).then(r => { if (!r.ok) throw new Error(u); return r.arrayBuffer(); }).then(b => new Uint8Array(b));
  const engine = Promise.all([["data/cards_unique.csv", "/rb/cards/cards_unique.csv"], ["data/decks.json", "/rb/decks/decks.json"],
    ...files.map(f => ["py/" + f, "/rb/engine/" + f])].map(([u, dst]) => bytes(u + "?v=" + ver).then(b => [dst, b])));
  engine.catch(() => {});
  // Bytecode du moteur (precompile.mjs) : facultatif, sans lui Python compile les .py comme avant.
  const pyc = fetch("py/bytecode.json?v=" + ver).then(r => r.ok ? r.json() : {}).catch(() => ({}));
  // La bibliothèque standard de Python est publiée en base64 (les .zip ne sont pas servis). Les URL blob: sont interdites par
  // la page claude.ai : on intercepte donc la requête de Pyodide vers stdlib.zip et on lui rend les octets décodés.
  const zip = fetch("pyodide/python_stdlib.b64.txt").then(r => { if (!r.ok) throw new Error("python_stdlib"); return r.text(); }).then(b64ToBytes);
  zip.catch(() => {});
  const stdLibURL = new URL("pyodide/stdlib-virtuel.zip", self.location.href).href;
  const realFetch = self.fetch.bind(self);
  self.fetch = (u, o) => String(u && u.url || u) === stdLibURL ? zip.then(z => new Response(z, { headers: { "Content-Type": "application/zip" } })) : realFetch(u, o);
  try { py = await loadPyodide({ indexURL: new URL("pyodide/", self.location.href).href, stdLibURL }); }
  finally { self.fetch = realFetch; }
  for (const d of ["/rb", "/rb/cards", "/rb/decks", "/rb/engine", "/rb/engine/cardsets"]) py.FS.mkdir(d);
  for (const [dst, b] of await engine) py.FS.writeFile(dst, b);
  // .pyc « checked-hash » : Python vérifie qu'ils viennent du .py posé à côté, sinon il recompile (jamais de bytecode périmé).
  for (const [f, b64] of Object.entries(await pyc)) {
    const dst = "/rb/engine/" + f;
    py.FS.mkdirTree(dst.slice(0, dst.lastIndexOf("/")));
    py.FS.writeFile(dst, b64ToBytes(b64));
  }
  py.runPython("import sys; sys.path.insert(0, '/rb/engine')");
  T = py.pyimport("train");
}

function b64ToBytes(b64) {
  const bin = atob(b64.trim()), out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

// Message d'une erreur Python : la dernière ligne non vide (le type et le texte), comme la page l'affichait avant.
const errMsg = e => String(e && e.message || e);

self.onmessage = async e => {
  const m = e.data;
  if (m.init) {
    try { await init(m.init); self.postMessage({ ready: true }); }
    catch (err) { self.postMessage({ fatal: errMsg(err) }); }
    return;
  }
  try {
    let r;
    if (m.exec !== undefined) {
      py.globals.set("_rb_code", m.exec);
      py.runPython("import train as _t; exec(_rb_code, _t.__dict__)");
      r = m.get ? py.runPython(`import json as _j, train as _t; _j.dumps(_t.__dict__.get(${JSON.stringify(m.get)}), default=str)`) : "true";
    } else {
      const f = T[m.fn];
      if (f === undefined || f === null) throw new Error("T." + m.fn + " n'est pas une fonction");
      r = f(...m.args);
      if (r && typeof r.toJs === "function") { const x = r.toJs(); r.destroy(); r = x; }
    }
    self.postMessage({ id: m.id, r });
  } catch (err) {
    self.postMessage({ id: m.id, err: errMsg(err) });
  }
};
