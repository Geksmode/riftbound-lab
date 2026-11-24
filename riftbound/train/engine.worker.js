// Moteur de la table dans un Web Worker : le moteur Rust compilé en WebAssembly (riftbound.wasm, crate rust/) tourne
// ici, hors du fil de la page. Une longue réflexion de l'IA ne gèle pas l'affichage.
// Protocole (messages) — inchangé depuis la version Pyodide :
//   page → worker : { init: { ver } }                         charge le module WebAssembly
//                   { id, fn, args }                          appelle train.<fn>(...args) (chaînes JSON en retour)
//                   { id, exec, get }                         robots de vérification : exec = liste d'opérations JSON
//                                                             (rust/src/train_dbg.rs) ; renvoie la valeur nommée get
//   worker → page : { ready: true } | { fatal: message } | { id, r } | { id, err }
// Les messages sont traités un par un, dans l'ordre d'arrivée : même graine = même partie. L'état de la partie et le
// hasard restent dans le module entre deux appels.

let W = null, BUF = null;
const enc = new TextEncoder(), dec = new TextDecoder();

async function init({ ver }) {
  // Le module a la version de la page en paramètre : sinon le navigateur peut garder l'ancien moteur en cache.
  const env = { rb_random: () => Math.random() };
  const url = "riftbound.wasm?v=" + ver;
  let inst;
  try {
    ({ instance: inst } = await WebAssembly.instantiateStreaming(fetch(url), { env }));
  } catch (e) {
    // serveur qui ne donne pas le type application/wasm : compilation depuis les octets
    const r = await fetch(url);
    if (!r.ok) throw new Error("riftbound.wasm : " + r.status);
    ({ instance: inst } = await WebAssembly.instantiate(await r.arrayBuffer(), { env }));
  }
  W = inst.exports;
}

function put(s) {
  const b = enc.encode(s), p = W.rb_alloc(b.length);
  new Uint8Array(W.memory.buffer, p, b.length).set(b);
  return [p, b.length];
}

// train.<fn>(...args) : texte de la réponse, ou exception avec le message d'erreur (dernière ligne : type et texte).
function call(fn, args) {
  const [fp, fl] = put(fn), [ap, al] = put(JSON.stringify(args.map(a => a === undefined ? null : a)));
  let p;
  try { p = W.rb_call(fp, fl, ap, al); }
  catch (e) {
    const n = W.rb_panic_len();
    const msg = n ? dec.decode(new Uint8Array(W.memory.buffer, W.rb_panic(), n).slice()) : String(e && e.message || e);
    throw new Error("le moteur s'est arrêté : " + msg);
  }
  const n = W.rb_len(), bytes = new Uint8Array(W.memory.buffer, p, n + 1).slice();
  const s = dec.decode(bytes.subarray(1));
  if (bytes[0] !== 0) throw new Error(s);
  return s;
}

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
      if (m.exec) call("dbg", [m.exec]);
      r = m.get ? JSON.stringify(JSON.parse(call("dbg", [JSON.stringify([{ as: "_get", op: ["get", m.get] }])]))._get ?? null) : "true";
    } else {
      r = call(m.fn, m.args || []);
    }
    self.postMessage({ id: m.id, r });
  } catch (err) {
    self.postMessage({ id: m.id, err: errMsg(err) });
  }
};
