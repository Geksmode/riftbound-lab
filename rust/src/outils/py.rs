//! Équivalents exacts de ce que les scripts Python de mesure utilisent : valeurs JSON avec la distinction entier /
//! flottant de Python (`json.load` / `json.dump`, pour relire et réécrire `results*.json` octet pour octet),
//! `repr(float)`, `round(x, n)`, `sum()` (sommation compensée de Neumaier depuis Python 3.12) et le format `%`.
use std::fmt::Write;

/// Valeur JSON telle que Python la voit (`int` et `float` distincts, dict ordonné).
#[derive(Clone, Debug, PartialEq)]
pub enum PyVal {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<PyVal>),
    Dict(Vec<(String, PyVal)>),
}

impl PyVal {
    /// Valeur du module `json` du moteur (`crate::json::Json`) vue comme valeur Python.
    pub fn from_json(j: &crate::json::Json) -> PyVal {
        use crate::json::Json;
        match j {
            Json::Null => PyVal::None,
            Json::Bool(b) => PyVal::Bool(*b),
            Json::Int(i) => PyVal::Int(*i),
            Json::Float(f) => PyVal::Float(*f),
            Json::Str(s) => PyVal::Str(s.clone()),
            Json::Arr(v) => PyVal::List(v.iter().map(PyVal::from_json).collect()),
            Json::Obj(v) => PyVal::Dict(v.iter().map(|(k, x)| (k.clone(), PyVal::from_json(x))).collect()),
        }
    }
    /// L'inverse de `from_json`.
    pub fn to_json(&self) -> crate::json::Json {
        use crate::json::Json;
        match self {
            PyVal::None => Json::Null,
            PyVal::Bool(b) => Json::Bool(*b),
            PyVal::Int(i) => Json::Int(*i),
            PyVal::Float(f) => Json::Float(*f),
            PyVal::Str(s) => Json::Str(s.clone()),
            PyVal::List(v) => Json::Arr(v.iter().map(PyVal::to_json).collect()),
            PyVal::Dict(v) => Json::Obj(v.iter().map(|(k, x)| (k.clone(), x.to_json())).collect()),
        }
    }
    pub fn get(&self, k: &str) -> Option<&PyVal> {
        match self {
            PyVal::Dict(v) => v.iter().find(|(x, _)| x == k).map(|(_, j)| j),
            _ => None,
        }
    }
    /// `d[k] = v` : garde la place d'une clé existante, ajoute sinon (dict Python).
    pub fn set(&mut self, k: &str, v: PyVal) {
        if let PyVal::Dict(d) = self {
            if let Some(e) = d.iter_mut().find(|(x, _)| x == k) {
                e.1 = v;
            } else {
                d.push((k.to_string(), v));
            }
        }
    }
    pub fn remove(&mut self, k: &str) -> Option<PyVal> {
        if let PyVal::Dict(d) = self {
            if let Some(i) = d.iter().position(|(x, _)| x == k) {
                return Some(d.remove(i).1);
            }
        }
        None
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PyVal::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_list(&self) -> &[PyVal] {
        match self {
            PyVal::List(v) => v,
            _ => &[],
        }
    }
    /// `float(x)`
    pub fn as_f64(&self) -> f64 {
        match self {
            PyVal::Int(i) => *i as f64,
            PyVal::Float(f) => *f,
            PyVal::Bool(b) => *b as i64 as f64,
            PyVal::Str(s) => s.trim().parse().expect("float() d'un texte non numérique"),
            _ => panic!("float() de {self:?}"),
        }
    }
    /// `int(x)` (tronque un flottant)
    pub fn as_i64(&self) -> i64 {
        match self {
            PyVal::Int(i) => *i,
            PyVal::Float(f) => f.trunc() as i64,
            PyVal::Bool(b) => *b as i64,
            PyVal::Str(s) => s.trim().parse().expect("int() d'un texte non entier"),
            _ => panic!("int() de {self:?}"),
        }
    }
    /// valeur de vérité Python
    pub fn truthy(&self) -> bool {
        match self {
            PyVal::None => false,
            PyVal::Bool(b) => *b,
            PyVal::Int(i) => *i != 0,
            PyVal::Float(f) => *f != 0.0,
            PyVal::Str(s) => !s.is_empty(),
            PyVal::List(v) => !v.is_empty(),
            PyVal::Dict(v) => !v.is_empty(),
        }
    }
    /// `str(x)`
    pub fn py_str(&self) -> String {
        match self {
            PyVal::Str(s) => s.clone(),
            _ => self.py_repr(),
        }
    }
    /// `repr(x)`
    pub fn py_repr(&self) -> String {
        match self {
            PyVal::None => "None".into(),
            PyVal::Bool(b) => (if *b { "True" } else { "False" }).into(),
            PyVal::Int(i) => i.to_string(),
            PyVal::Float(f) => repr_f64(*f),
            PyVal::Str(s) => {
                let q = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
                let mut o = String::new();
                o.push(q);
                for c in s.chars() {
                    match c {
                        '\\' => o.push_str("\\\\"),
                        '\n' => o.push_str("\\n"),
                        '\r' => o.push_str("\\r"),
                        '\t' => o.push_str("\\t"),
                        c if c == q => {
                            o.push('\\');
                            o.push(c)
                        }
                        c => o.push(c),
                    }
                }
                o.push(q);
                o
            }
            PyVal::List(v) => format!("[{}]", v.iter().map(|x| x.py_repr()).collect::<Vec<_>>().join(", ")),
            PyVal::Dict(v) => format!(
                "{{{}}}",
                v.iter().map(|(k, x)| format!("{}: {}", PyVal::Str(k.clone()).py_repr(), x.py_repr())).collect::<Vec<_>>().join(", ")
            ),
        }
    }
}

impl From<&str> for PyVal {
    fn from(s: &str) -> Self {
        PyVal::Str(s.to_string())
    }
}
impl From<String> for PyVal {
    fn from(s: String) -> Self {
        PyVal::Str(s)
    }
}
impl From<i64> for PyVal {
    fn from(i: i64) -> Self {
        PyVal::Int(i)
    }
}
impl From<f64> for PyVal {
    fn from(f: f64) -> Self {
        PyVal::Float(f)
    }
}
impl From<bool> for PyVal {
    fn from(b: bool) -> Self {
        PyVal::Bool(b)
    }
}
impl<T: Into<PyVal>> From<Option<T>> for PyVal {
    fn from(o: Option<T>) -> Self {
        o.map(Into::into).unwrap_or(PyVal::None)
    }
}
impl<T: Into<PyVal>> From<Vec<T>> for PyVal {
    fn from(v: Vec<T>) -> Self {
        PyVal::List(v.into_iter().map(Into::into).collect())
    }
}

/// `dict(a=..., b=...)` dans l'ordre donné.
#[macro_export]
macro_rules! pydict {
    ($($k:expr => $v:expr),* $(,)?) => {
        $crate::outils::py::PyVal::Dict(vec![$(($k.to_string(), $crate::outils::py::PyVal::from($v))),*])
    };
}

// ------------------------------------------------------------------ nombres

/// `repr(x)` d'un flottant Python (chiffres les plus courts qui relisent la même valeur ; notation scientifique hors
/// de 1e-4 <= |x| < 1e16).
pub fn repr_f64(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return (if x > 0.0 { "inf" } else { "-inf" }).into();
    }
    if x == 0.0 {
        return (if x.is_sign_negative() { "-0.0" } else { "0.0" }).into();
    }
    let e = format!("{:e}", x.abs()); // « 1.2345e-7 » : chiffres les plus courts
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let sign = if x < 0.0 { "-" } else { "" };
    let n = digits.len() as i32;
    if (-4..16).contains(&exp) {
        let point = exp + 1; // position de la virgule dans les chiffres
        let s = if point <= 0 {
            format!("0.{}{}", "0".repeat((-point) as usize), digits)
        } else if point >= n {
            format!("{}{}.0", digits, "0".repeat((point - n) as usize))
        } else {
            format!("{}.{}", &digits[..point as usize], &digits[point as usize..])
        };
        format!("{sign}{s}")
    } else {
        let m = if n > 1 { format!("{}.{}", &digits[..1], &digits[1..]) } else { digits.clone() };
        format!("{sign}{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
    }
}

/// `round(x, n)` (n >= 0) : arrondi correct de la valeur binaire exacte, au pair en cas d'égalité exacte.
pub fn round_n(x: f64, n: usize) -> f64 {
    if !x.is_finite() {
        return x;
    }
    format!("{x:.n$}").parse().unwrap()
}

/// `round(x)` sans chiffres : entier le plus proche, au pair en cas d'égalité.
pub fn round0(x: f64) -> i64 {
    x.round_ties_even() as i64
}

/// `sum(xs)` de flottants (CPython >= 3.12 : départ 0 entier, puis sommation compensée de Neumaier).
pub fn sum(xs: impl IntoIterator<Item = f64>) -> f64 {
    let mut it = xs.into_iter();
    let Some(first) = it.next() else { return 0.0 };
    let mut f = 0.0 + first;
    let mut c = 0.0f64;
    for x in it {
        let t = f + x;
        if f.abs() >= x.abs() {
            c += (f - t) + x;
        } else {
            c += (x - t) + f;
        }
        f = t;
    }
    if c != 0.0 && c.is_finite() {
        f += c;
    }
    f
}

/// `format(x, ".{p}%")`
pub fn pct(x: f64, p: usize) -> String {
    format!("{:.p$}%", x * 100.0)
}

/// `str(x)` d'un flottant dans un f-string `{x}` (identique à `repr`).
pub fn fstr(x: f64) -> String {
    repr_f64(x)
}

// ------------------------------------------------------------------ JSON

/// `json.load` : entiers et flottants distingués comme en Python (NaN, Infinity acceptés).
pub fn parse(text: &str) -> Result<PyVal, String> {
    let mut p = P { s: text.as_bytes(), i: 0 };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.s.len() {
        return Err(format!("texte en trop à l'octet {}", p.i));
    }
    Ok(v)
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn lit(&mut self, w: &str) -> bool {
        if self.s[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            true
        } else {
            false
        }
    }
    fn value(&mut self) -> Result<PyVal, String> {
        self.ws();
        if self.i >= self.s.len() {
            return Err("fin inattendue".into());
        }
        match self.s[self.i] {
            b'{' => {
                self.i += 1;
                let mut v = vec![];
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(PyVal::Dict(v));
                }
                loop {
                    self.ws();
                    let PyVal::Str(k) = self.string()? else { unreachable!() };
                    self.ws();
                    if self.s.get(self.i) != Some(&b':') {
                        return Err(format!("« : » attendu à l'octet {}", self.i));
                    }
                    self.i += 1;
                    let x = self.value()?;
                    // dict Python : une clé répétée garde sa première place et prend la dernière valeur
                    if let Some(e) = v.iter_mut().find(|(a, _): &&mut (String, PyVal)| *a == k) {
                        e.1 = x;
                    } else {
                        v.push((k, x));
                    }
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(PyVal::Dict(v));
                        }
                        _ => return Err(format!("« , » ou « }} » attendu à l'octet {}", self.i)),
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut v = vec![];
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(PyVal::List(v));
                }
                loop {
                    v.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(PyVal::List(v));
                        }
                        _ => return Err(format!("« , » ou « ] » attendu à l'octet {}", self.i)),
                    }
                }
            }
            b'"' => self.string(),
            _ => {
                if self.lit("null") {
                    return Ok(PyVal::None);
                }
                if self.lit("true") {
                    return Ok(PyVal::Bool(true));
                }
                if self.lit("false") {
                    return Ok(PyVal::Bool(false));
                }
                if self.lit("NaN") {
                    return Ok(PyVal::Float(f64::NAN));
                }
                if self.lit("Infinity") {
                    return Ok(PyVal::Float(f64::INFINITY));
                }
                if self.lit("-Infinity") {
                    return Ok(PyVal::Float(f64::NEG_INFINITY));
                }
                let st = self.i;
                let mut float = false;
                while self.i < self.s.len() {
                    match self.s[self.i] {
                        b'0'..=b'9' | b'-' | b'+' => {}
                        b'.' | b'e' | b'E' => float = true,
                        _ => break,
                    }
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[st..self.i]).unwrap();
                if t.is_empty() {
                    return Err(format!("valeur attendue à l'octet {st}"));
                }
                if float {
                    t.parse().map(PyVal::Float).map_err(|_| format!("nombre illisible : {t}"))
                } else {
                    t.parse().map(PyVal::Int).map_err(|_| format!("entier illisible : {t}"))
                }
            }
        }
    }
    fn hex4(&mut self) -> Result<u32, String> {
        let h = std::str::from_utf8(self.s.get(self.i..self.i + 4).ok_or("\\u tronqué")?).map_err(|e| e.to_string())?;
        self.i += 4;
        u32::from_str_radix(h, 16).map_err(|e| e.to_string())
    }
    fn string(&mut self) -> Result<PyVal, String> {
        if self.s.get(self.i) != Some(&b'"') {
            return Err(format!("texte attendu à l'octet {}", self.i));
        }
        self.i += 1;
        let mut out: Vec<u8> = vec![];
        loop {
            let c = *self.s.get(self.i).ok_or("texte non fermé")?;
            self.i += 1;
            match c {
                b'"' => return Ok(PyVal::Str(String::from_utf8(out).map_err(|e| e.to_string())?)),
                b'\\' => {
                    let e = self.s[self.i];
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'/' => out.push(b'/'),
                        b'\\' => out.push(b'\\'),
                        b'"' => out.push(b'"'),
                        b'u' => {
                            let mut cp = self.hex4()?;
                            if (0xD800..0xDC00).contains(&cp) && self.s[self.i..].starts_with(b"\\u") {
                                let save = self.i;
                                self.i += 2;
                                let lo = self.hex4()?;
                                if (0xDC00..0xE000).contains(&lo) {
                                    cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                                } else {
                                    self.i = save;
                                }
                            }
                            let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
                            let mut b = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                        }
                        _ => return Err(format!("échappement inconnu \\{}", e as char)),
                    }
                }
                c => out.push(c),
            }
        }
    }
}

/// Options de `json.dump` : `indent`, `ensure_ascii`.
#[derive(Clone, Copy)]
pub struct Dump {
    pub indent: Option<usize>,
    pub ensure_ascii: bool,
}

/// `json.dumps(v, indent=..., ensure_ascii=...)` (séparateurs par défaut de Python : « , » et « : », « , » en fin de
/// ligne avec indent).
pub fn dumps(v: &PyVal, o: Dump) -> String {
    let mut s = String::new();
    write_val(v, o, 0, &mut s);
    s
}

fn write_val(v: &PyVal, o: Dump, level: usize, out: &mut String) {
    match v {
        PyVal::None => out.push_str("null"),
        PyVal::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        PyVal::Int(i) => {
            let _ = write!(out, "{i}");
        }
        PyVal::Float(f) => {
            if f.is_nan() {
                out.push_str("NaN")
            } else if f.is_infinite() {
                out.push_str(if *f > 0.0 { "Infinity" } else { "-Infinity" })
            } else {
                out.push_str(&repr_f64(*f))
            }
        }
        PyVal::Str(s) => write_str(s, o.ensure_ascii, out),
        PyVal::List(v) => {
            if v.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, x) in v.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                    if o.indent.is_none() {
                        out.push(' ');
                    }
                }
                newline(o, level + 1, out);
                write_val(x, o, level + 1, out);
            }
            newline(o, level, out);
            out.push(']');
        }
        PyVal::Dict(v) => {
            if v.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (k, x)) in v.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                    if o.indent.is_none() {
                        out.push(' ');
                    }
                }
                newline(o, level + 1, out);
                write_str(k, o.ensure_ascii, out);
                out.push_str(": ");
                write_val(x, o, level + 1, out);
            }
            newline(o, level, out);
            out.push('}');
        }
    }
}

fn newline(o: Dump, level: usize, out: &mut String) {
    if let Some(n) = o.indent {
        out.push('\n');
        out.push_str(&" ".repeat(n * level));
    }
}

fn write_str(s: &str, ascii: bool, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c if ascii && (c as u32) > 0x7e => {
                let mut b = [0u16; 2];
                for u in c.encode_utf16(&mut b) {
                    let _ = write!(out, "\\u{:04x}", u);
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `json.load(open(f))` d'un fichier
pub fn load_file(path: &std::path::Path) -> Result<PyVal, String> {
    let t = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&t).map_err(|e| format!("{}: {e}", path.display()))
}

/// `json.dump(v, open(f, "w"), ...)`
pub fn dump_file(path: &std::path::Path, v: &PyVal, o: Dump) {
    std::fs::write(path, dumps(v, o)).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repr_comme_python() {
        for (x, r) in [
            (0.1 + 0.2, "0.30000000000000004"),
            (1e-7, "1e-07"),
            (1e16, "1e+16"),
            (123456.789, "123456.789"),
            (0.0001, "0.0001"),
            (1.0, "1.0"),
            (-2.5, "-2.5"),
            (1e15, "1000000000000000.0"),
            (1.5e300, "1.5e+300"),
        ] {
            assert_eq!(repr_f64(x), r);
        }
        assert_eq!(round_n(2.675, 2), 2.67);
        assert_eq!(round_n(0.125, 2), 0.12);
        assert_eq!(round0(2.5), 2);
        assert_eq!(sum(vec![0.1; 10]), 1.0);
    }

    #[test]
    fn json_aller_retour() {
        let a = r#"[{"label": "a \u00e9 \"x\"", "n": 3, "wr": 0.5, "x": 1.0, "l": [], "d": {}, "z": null}]"#;
        let u = a.replace("\\u00e9", "\u{e9}");
        assert_ne!(a, u);
        let v = parse(a).unwrap();
        assert_eq!(dumps(&v, Dump { indent: None, ensure_ascii: false }), u);
        assert_eq!(dumps(&v, Dump { indent: None, ensure_ascii: true }), a);
    }

    /// Les fichiers de résultats du dépôt, écrits par `json.dump` de Python, se relisent et se réécrivent à l'octet
    /// près (la suite d'un fichier existant est ainsi écrite exactement comme par Python).
    #[test]
    fn resultats_du_depot_a_l_octet() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../riftbound");
        let mut files = vec![];
        for d in ["engine", "manager/results"] {
            for e in std::fs::read_dir(root.join(d)).unwrap() {
                let p = e.unwrap().path();
                if p.extension().is_some_and(|x| x == "json") && p.file_name().unwrap() != "empreinte.json" {
                    files.push(p);
                }
            }
        }
        assert!(files.len() >= 5);
        for p in files {
            let t = std::fs::read_to_string(&p).unwrap();
            let v = parse(&t).unwrap();
            let ok = [None, Some(1), Some(2)].into_iter().any(|indent| {
                [false, true].into_iter().any(|a| dumps(&v, Dump { indent, ensure_ascii: a }) == t)
            });
            assert!(ok, "{} : réécriture différente", p.display());
        }
    }
}
