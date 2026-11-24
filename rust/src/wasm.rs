//! Liaison WebAssembly de la table (sans dépendance) : le Web Worker de la page (`riftbound/train/engine.worker.js`)
//! écrit le nom de la fonction et ses arguments (JSON) dans la mémoire du module, appelle `rb_call`, puis lit la
//! réponse. Remplace `train.<fn>(*args)` de Pyodide ; l'état de la partie et le hasard restent dans le module entre
//! deux appels, comme dans le module Python.
//!
//! Protocole : `rb_alloc(n)` réserve n octets (le worker y copie le texte) ; `rb_call(f, nf, a, na)` lit le nom (UTF-8)
//! et les arguments (tableau JSON), libère ces deux tampons et renvoie l'adresse de la réponse : un octet d'état (0 :
//! résultat, 1 : erreur Python, en texte) suivi du texte UTF-8, de longueur `rb_len()`. La réponse reste valable
//! jusqu'à l'appel suivant. Après un arrêt brutal (panic : instruction `unreachable`), `rb_panic` / `rb_panic_len`
//! donnent le message.
use std::cell::RefCell;

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static PANIC: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Réserve n octets dans la mémoire du module.
#[no_mangle]
pub extern "C" fn rb_alloc(n: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(n.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Libère un tampon de `rb_alloc`.
///
/// # Safety
/// p vient de `rb_alloc(n)` et n'a pas encore été libéré.
#[no_mangle]
pub unsafe extern "C" fn rb_free(p: *mut u8, n: usize) {
    drop(Vec::from_raw_parts(p, 0, n.max(1)));
}

fn take(p: *mut u8, n: usize) -> String {
    // SAFETY : le worker a écrit n octets à l'adresse p (rb_alloc(n))
    let v = unsafe { Vec::from_raw_parts(p, n, n.max(1)) };
    String::from_utf8_lossy(&v).into_owned()
}

/// Appelle `train.<f>(*args)`.
///
/// # Safety
/// f (nf octets) et a (na octets) viennent de `rb_alloc` ; ils sont libérés ici.
#[no_mangle]
pub unsafe extern "C" fn rb_call(f: *mut u8, nf: usize, a: *mut u8, na: usize) -> *const u8 {
    install_hook();
    let f = take(f, nf);
    let a = take(a, na);
    let r = match crate::json::parse(&a) {
        Ok(j) => crate::train::call(&f, j.arr()),
        Err(e) => Err(format!("arguments illisibles : {e}")),
    };
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        o.clear();
        match r {
            Ok(s) => {
                o.push(0);
                o.extend_from_slice(s.as_bytes());
            }
            Err(e) => {
                o.push(1);
                o.extend_from_slice(e.as_bytes());
            }
        }
        o.as_ptr()
    })
}

/// Longueur du texte de la dernière réponse (sans l'octet d'état).
#[no_mangle]
pub extern "C" fn rb_len() -> usize {
    OUT.with(|o| o.borrow().len().saturating_sub(1))
}

/// Message du dernier arrêt brutal.
#[no_mangle]
pub extern "C" fn rb_panic() -> *const u8 {
    PANIC.with(|p| p.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn rb_panic_len() -> usize {
    PANIC.with(|p| p.borrow().len())
}

fn install_hook() {
    use std::sync::Once;
    static H: Once = Once::new();
    H.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let msg = info.to_string();
            PANIC.with(|p| {
                if let Ok(mut p) = p.try_borrow_mut() {
                    p.clear();
                    p.extend_from_slice(msg.as_bytes());
                }
            });
        }));
    });
}

#[link(wasm_import_module = "env")]
extern "C" {
    /// `Math.random()` du navigateur (graine d'une partie sans graine : `random.randrange(10 ** 6)`)
    fn rb_random() -> f64;
}

/// Nombre au hasard du navigateur, dans [0, 1).
pub fn random() -> f64 {
    // SAFETY : fonction fournie par le worker, sans argument
    unsafe { rb_random() }
}
