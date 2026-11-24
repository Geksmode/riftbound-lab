//! Lecture et écriture JSON minimales (le moteur n'a aucune dépendance) : paquets (`decks.json`), protocole de la
//! table, journaux du test différentiel.
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    /// entier (`int` Python)
    Int(i64),
    /// flottant (`float` Python) : toujours écrit comme `repr(float)` (« 3.0 », « 1e-05 »)
    Float(f64),
    Str(String),
    Arr(Vec<Json>),
    /// objet, ordre des clés conservé
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, k: &str) -> Option<&Json> {
        match self {
            Json::Obj(v) => v.iter().find(|(x, _)| x == k).map(|(_, j)| j),
            _ => None,
        }
    }
    pub fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn num(&self) -> Option<f64> {
        match self {
            Json::Int(n) => Some(*n as f64),
            Json::Float(n) => Some(*n),
            _ => None,
        }
    }
    /// entier (un flottant compte aussi, tronqué comme `int(x)`)
    pub fn int(&self) -> Option<i64> {
        match self {
            Json::Int(n) => Some(*n),
            Json::Float(n) => Some(*n as i64),
            Json::Bool(b) => Some(*b as i64),
            _ => None,
        }
    }
    pub fn obj(&self) -> &[(String, Json)] {
        match self {
            Json::Obj(v) => v,
            _ => &[],
        }
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }

    /// `json.dumps(x)` (séparateurs par défaut « , » et « : ») ; ascii : `ensure_ascii=True` (défaut de Python).
    pub fn dumps(&self, ascii: bool) -> String {
        let mut s = String::new();
        self.write_py(&mut s, ascii);
        s
    }

    fn write_py(&self, out: &mut String, ascii: bool) {
        match self {
            Json::Str(s) => write_str_py(s, out, ascii),
            Json::Arr(v) => {
                out.push('[');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    x.write_py(out, ascii);
                }
                out.push(']');
            }
            Json::Obj(v) => {
                out.push('{');
                for (i, (k, x)) in v.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    write_str_py(k, out, ascii);
                    out.push_str(": ");
                    x.write_py(out, ascii);
                }
                out.push('}');
            }
            x => x.write(out),
        }
    }
    pub fn arr(&self) -> &[Json] {
        match self {
            Json::Arr(v) => v,
            _ => &[],
        }
    }

    /// Écriture compacte, à la manière de `json.dumps(x, separators=(",", ":"), ensure_ascii=False)`.
    pub fn dump(&self) -> String {
        let mut s = String::new();
        self.write(&mut s);
        s
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => {
                let _ = write!(out, "{n}");
            }
            Json::Float(n) => out.push_str(&py_float(*n)),
            Json::Str(s) => write_str(s, out),
            Json::Arr(v) => {
                out.push('[');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    x.write(out);
                }
                out.push(']');
            }
            Json::Obj(v) => {
                out.push('{');
                for (i, (k, x)) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(k, out);
                    out.push(':');
                    x.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_str(s: &str, out: &mut String) {
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
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `repr(float)` de Python : chiffres les plus courts, exposant au-delà de 1e16 ou en deçà de 1e-4 ; NaN et infinis
/// comme `json.dumps` (NaN, Infinity).
pub fn py_float(f: f64) -> String {
    if f.is_nan() {
        return "NaN".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0.0".into() } else { "0.0".into() };
    }
    let e = format!("{f:e}"); // « -1.2345e-5 » : les chiffres les plus courts qui relisent la même valeur
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let n = digits.len() as i32;
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    if (-4..16).contains(&exp) {
        if exp >= 0 {
            if n <= exp + 1 {
                out.push_str(&digits);
                for _ in 0..(exp + 1 - n) {
                    out.push('0');
                }
                out.push_str(".0");
            } else {
                out.push_str(&digits[..(exp + 1) as usize]);
                out.push('.');
                out.push_str(&digits[(exp + 1) as usize..]);
            }
        } else {
            out.push_str("0.");
            for _ in 0..(-exp - 1) {
                out.push('0');
            }
            out.push_str(&digits);
        }
    } else {
        out.push_str(&digits[..1]);
        if n > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        let _ = write!(out, "e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs());
    }
    out
}

/// Texte JSON à la manière de Python ; ascii : `ensure_ascii=True` (\uXXXX, paires de substitution).
fn write_str_py(s: &str, out: &mut String, ascii: bool) {
    if !ascii {
        return write_str(s, out);
    }
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
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
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

pub fn parse(text: &str) -> Result<Json, String> {
    let mut p = Parser { s: text.as_bytes(), i: 0 };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.s.len() {
        return Err(format!("JSON : caractères en trop à {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\n' | b'\r' | b'\t') {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("JSON : '{}' attendu à {}", c as char, self.i))
        }
    }
    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'{') => {
                self.i += 1;
                let mut v = vec![];
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(v));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.eat(b':')?;
                    let x = self.value()?;
                    v.push((k, x));
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(v));
                        }
                        _ => return Err(format!("JSON : ',' ou '}}' attendu à {}", self.i)),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut v = vec![];
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(v));
                }
                loop {
                    v.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(v));
                        }
                        _ => return Err(format!("JSON : ',' ou ']' attendu à {}", self.i)),
                    }
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') if self.s[self.i..].starts_with(b"true") => {
                self.i += 4;
                Ok(Json::Bool(true))
            }
            Some(b'f') if self.s[self.i..].starts_with(b"false") => {
                self.i += 5;
                Ok(Json::Bool(false))
            }
            Some(b'n') if self.s[self.i..].starts_with(b"null") => {
                self.i += 4;
                Ok(Json::Null)
            }
            Some(_) => {
                let st = self.i;
                while self.i < self.s.len() && matches!(self.s[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[st..self.i]).unwrap();
                if !t.contains(['.', 'e', 'E']) {
                    if let Ok(n) = t.parse::<i64>() {
                        return Ok(Json::Int(n));
                    }
                }
                t.parse::<f64>().map(Json::Float).map_err(|_| format!("JSON : nombre invalide {t:?} à {st}"))
            }
            None => Err("JSON : fin inattendue".into()),
        }
    }
    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = vec![];
        loop {
            let Some(&c) = self.s.get(self.i) else { return Err("JSON : texte non fermé".into()) };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.s[self.i];
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'u' => {
                            let mut cp = self.hex4()?;
                            if (0xd800..0xdc00).contains(&cp) && self.s[self.i..].starts_with(b"\\u") {
                                self.i += 2;
                                let lo = self.hex4()?;
                                cp = 0x10000 + ((cp - 0xd800) << 10) + (lo - 0xdc00);
                            }
                            let ch = char::from_u32(cp).unwrap_or('\u{fffd}');
                            let mut b = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                        }
                        _ => return Err(format!("JSON : échappement inconnu à {}", self.i)),
                    }
                }
                c => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|e| e.to_string())
    }
    fn hex4(&mut self) -> Result<u32, String> {
        let t = std::str::from_utf8(&self.s[self.i..self.i + 4]).map_err(|e| e.to_string())?;
        self.i += 4;
        u32::from_str_radix(t, 16).map_err(|e| e.to_string())
    }
}

impl From<bool> for Json {
    fn from(b: bool) -> Self {
        Json::Bool(b)
    }
}
impl From<i64> for Json {
    fn from(n: i64) -> Self {
        Json::Int(n)
    }
}
impl From<i32> for Json {
    fn from(n: i32) -> Self {
        Json::Int(n as i64)
    }
}
impl From<u32> for Json {
    fn from(n: u32) -> Self {
        Json::Int(n as i64)
    }
}
impl From<usize> for Json {
    fn from(n: usize) -> Self {
        Json::Int(n as i64)
    }
}
impl From<f64> for Json {
    fn from(n: f64) -> Self {
        Json::Float(n)
    }
}
impl From<&str> for Json {
    fn from(s: &str) -> Self {
        Json::Str(s.to_string())
    }
}
impl From<String> for Json {
    fn from(s: String) -> Self {
        Json::Str(s)
    }
}
impl From<&String> for Json {
    fn from(s: &String) -> Self {
        Json::Str(s.clone())
    }
}
impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(o: Option<T>) -> Self {
        match o {
            Some(x) => x.into(),
            None => Json::Null,
        }
    }
}
impl<T: Into<Json>> From<Vec<T>> for Json {
    fn from(v: Vec<T>) -> Self {
        Json::Arr(v.into_iter().map(Into::into).collect())
    }
}

impl Json {
    /// Objet : ajoute (ou remplace, à sa place) une clé, comme `d[k] = v` en Python.
    pub fn set(&mut self, k: &str, v: impl Into<Json>) {
        if let Json::Obj(o) = self {
            let v = v.into();
            if let Some(e) = o.iter_mut().find(|(x, _)| x == k) {
                e.1 = v;
            } else {
                o.push((k.to_string(), v));
            }
        }
    }
    /// `d.pop(k, None)`
    pub fn remove(&mut self, k: &str) -> Option<Json> {
        if let Json::Obj(o) = self {
            let i = o.iter().position(|(x, _)| x == k)?;
            return Some(o.remove(i).1);
        }
        None
    }
    pub fn get_mut(&mut self, k: &str) -> Option<&mut Json> {
        match self {
            Json::Obj(v) => v.iter_mut().find(|(x, _)| x == k).map(|(_, j)| j),
            _ => None,
        }
    }
    pub fn arr_mut(&mut self) -> &mut Vec<Json> {
        match self {
            Json::Arr(v) => v,
            _ => panic!("liste JSON attendue"),
        }
    }
    /// `d.get(k)` : Null si absente.
    pub fn g(&self, k: &str) -> &Json {
        static NULL: Json = Json::Null;
        self.get(k).unwrap_or(&NULL)
    }
    /// Vérité Python
    pub fn truthy(&self) -> bool {
        match self {
            Json::Null => false,
            Json::Bool(b) => *b,
            Json::Int(n) => *n != 0,
            Json::Float(f) => *f != 0.0,
            Json::Str(s) => !s.is_empty(),
            Json::Arr(v) => !v.is_empty(),
            Json::Obj(v) => !v.is_empty(),
        }
    }
}

/// Objet JSON, clés dans l'ordre donné : `jo! { "a" => 1, "b" => "x" }`.
#[macro_export]
macro_rules! jo {
    () => { $crate::json::Json::Obj(vec![]) };
    ($($k:expr => $v:expr),+ $(,)?) => {
        $crate::json::Json::Obj(vec![$(($k.to_string(), $crate::json::Json::from($v))),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comme_json_dumps() {
        for (f, s) in [(3.0, "3.0"), (-0.5, "-0.5"), (1e-5, "1e-05"), (1e16, "1e+16"), (123456.7, "123456.7"), (0.0001, "0.0001")] {
            assert_eq!(py_float(f), s);
        }
        let j = parse(r#"{"a": [1, 2.5, "é"], "b": null}"#).unwrap();
        let bs = char::from(92u8); // barre oblique inverse
        assert_eq!(j.dumps(true), format!("{{\"a\": [1, 2.5, \"{bs}u00e9\"], \"b\": null}}"));
        assert_eq!(j.dumps(false), r#"{"a": [1, 2.5, "é"], "b": null}"#);
    }
}
