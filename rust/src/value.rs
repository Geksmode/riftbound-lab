//! Valeurs dynamiques : ce que le moteur Python range dans des dict sans type fixe (`Item.data`, `info` des
//! évènements, entrées de `g.effects`, options et contexte de `g.ask`). Les cartes se traduisent ainsi sans être
//! réécrites. Les dictionnaires gardent l'ordre d'insertion, comme en Python.
use std::fmt;
use std::sync::Arc;

use crate::game::{Game, ItemId, Res, O, R};

/// Fonction rangée dans une valeur (`eff["fn"]`, `ad["can"]`, prédicats...). Les arguments et le retour sont des
/// valeurs : chaque appelant sait ce qu'il passe, comme en Python. `Err(GameOver)` remonte une fin de partie
/// (l'exception `GameOver` de Python).
pub type DynFn = Arc<dyn Fn(&mut Game, &[Value]) -> Res<Value> + Send + Sync>;

/// Une fonction Python, son nom (journal) et ses attributs (`choose.options`, `cost.can`, `cost.label`...).
#[derive(Clone)]
pub struct Callback(pub DynFn, pub &'static str, pub Option<Arc<Map>>);

impl fmt::Debug for Callback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<fn {}>", self.1)
    }
}

impl PartialEq for Callback {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Emplacement sur le plateau : la base ou un battlefield (`is_bf(loc)` : un indice de `g.bfs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Loc {
    Base,
    Bf(usize),
}

impl Loc {
    pub fn is_bf(self) -> bool {
        matches!(self, Loc::Bf(_))
    }
    pub fn bf(self) -> Option<usize> {
        match self {
            Loc::Bf(i) => Some(i),
            Loc::Base => None,
        }
    }
}

/// `is_bf(loc)` pour un emplacement facultatif (`Obj.loc` vaut None hors du plateau).
pub fn is_bf(loc: Option<Loc>) -> bool {
    matches!(loc, Some(Loc::Bf(_)))
}

/// `game.Opt` : option libellée de `g.ask`.
#[derive(Clone, Debug, PartialEq)]
pub struct OptV {
    pub label: String,
    pub value: Value,
    pub obj: Value,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum Value {
    #[default]
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    Obj(O),
    Rune(R),
    Item(ItemId),
    Loc(Loc),
    /// une liste Python
    List(Vec<Value>),
    /// un tuple Python (distinct de la liste pour les libellés et les égalités)
    Tuple(Vec<Value>),
    Map(Map),
    Opt(Box<OptV>),
    Fn(Callback),
}

/// Dict Python à clés texte, ordre d'insertion conservé.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Map(pub Vec<(Arc<str>, Value)>);

impl Map {
    pub fn new() -> Self {
        Map(Vec::new())
    }
    pub fn get(&self, k: &str) -> Option<&Value> {
        self.0.iter().find(|(x, _)| &**x == k).map(|(_, v)| v)
    }
    pub fn get_mut(&mut self, k: &str) -> Option<&mut Value> {
        self.0.iter_mut().find(|(x, _)| &**x == k).map(|(_, v)| v)
    }
    /// `d.get(k)` : None si absente.
    pub fn g(&self, k: &str) -> Value {
        self.get(k).cloned().unwrap_or(Value::None)
    }
    pub fn set(&mut self, k: &str, v: impl Into<Value>) {
        let v = v.into();
        if let Some(x) = self.get_mut(k) {
            *x = v;
        } else {
            self.0.push((Arc::from(k), v));
        }
    }
    pub fn contains(&self, k: &str) -> bool {
        self.get(k).is_some()
    }
    pub fn remove(&mut self, k: &str) -> Option<Value> {
        let i = self.0.iter().position(|(x, _)| &**x == k)?;
        Some(self.0.remove(i).1)
    }
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(k, _)| &**k)
    }
    /// `{**a, **b}`
    pub fn merged(&self, other: &Map) -> Map {
        let mut m = self.clone();
        for (k, v) in &other.0 {
            m.set(k, v.clone());
        }
        m
    }
}

#[macro_export]
macro_rules! map {
    () => { $crate::value::Map::new() };
    ($($k:ident = $v:expr),+ $(,)?) => {{
        let mut m = $crate::value::Map::new();
        $( m.set(stringify!($k), $crate::value::Value::from($v)); )+
        m
    }};
}

impl Value {
    pub fn is_none(&self) -> bool {
        matches!(self, Value::None)
    }
    /// Vérité Python (`bool(x)`).
    pub fn truthy(&self) -> bool {
        match self {
            Value::None => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(v) | Value::Tuple(v) => !v.is_empty(),
            Value::Map(m) => !m.0.is_empty(),
            _ => true,
        }
    }
    pub fn int(&self) -> i64 {
        match self {
            Value::Int(i) => *i,
            Value::Bool(b) => *b as i64,
            Value::None => 0,
            v => panic!("entier attendu : {v:?}"),
        }
    }
    pub fn opt_int(&self) -> Option<i64> {
        match self {
            Value::None => None,
            v => Some(v.int()),
        }
    }
    pub fn float(&self) -> f64 {
        match self {
            Value::Float(f) => *f,
            Value::Int(i) => *i as f64,
            Value::Bool(b) => *b as i64 as f64,
            v => panic!("nombre attendu : {v:?}"),
        }
    }
    pub fn str(&self) -> &str {
        match self {
            Value::Str(s) => s,
            v => panic!("texte attendu : {v:?}"),
        }
    }
    pub fn opt_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn obj(&self) -> O {
        match self {
            Value::Obj(o) => *o,
            v => panic!("objet attendu : {v:?}"),
        }
    }
    pub fn opt_obj(&self) -> Option<O> {
        match self {
            Value::Obj(o) => Some(*o),
            _ => None,
        }
    }
    pub fn rune(&self) -> R {
        match self {
            Value::Rune(r) => *r,
            v => panic!("rune attendue : {v:?}"),
        }
    }
    pub fn item(&self) -> ItemId {
        match self {
            Value::Item(i) => *i,
            v => panic!("item attendu : {v:?}"),
        }
    }
    pub fn loc(&self) -> Loc {
        match self {
            Value::Loc(l) => *l,
            v => panic!("emplacement attendu : {v:?}"),
        }
    }
    pub fn opt_loc(&self) -> Option<Loc> {
        match self {
            Value::Loc(l) => Some(*l),
            _ => None,
        }
    }
    pub fn list(&self) -> &[Value] {
        match self {
            Value::List(v) | Value::Tuple(v) => v,
            Value::None => &[],
            v => panic!("liste attendue : {v:?}"),
        }
    }
    pub fn list_mut(&mut self) -> &mut Vec<Value> {
        match self {
            Value::List(v) | Value::Tuple(v) => v,
            v => panic!("liste attendue : {v:?}"),
        }
    }
    pub fn map(&self) -> &Map {
        match self {
            Value::Map(m) => m,
            v => panic!("dict attendu : {v:?}"),
        }
    }
    pub fn map_mut(&mut self) -> &mut Map {
        match self {
            Value::Map(m) => m,
            v => panic!("dict attendu : {v:?}"),
        }
    }
    pub fn opt(&self) -> &OptV {
        match self {
            Value::Opt(o) => o,
            v => panic!("Opt attendue : {v:?}"),
        }
    }
    pub fn callback(&self) -> &Callback {
        match self {
            Value::Fn(c) => c,
            v => panic!("fonction attendue : {v:?}"),
        }
    }
    /// Appelle une fonction rangée dans une valeur.
    pub fn call(&self, g: &mut Game, args: &[Value]) -> Res<Value> {
        let f = self.callback().0.clone();
        f(g, args)
    }
    /// Attribut d'une fonction (`getattr(fn, name, None)`).
    pub fn attr(&self, name: &str) -> Value {
        match self {
            Value::Fn(Callback(_, _, Some(a))) => a.g(name),
            _ => Value::None,
        }
    }
    pub fn str_v(s: &str) -> Value {
        Value::Str(Arc::from(s))
    }
    pub fn func(name: &'static str, f: impl Fn(&mut Game, &[Value]) -> Res<Value> + Send + Sync + 'static) -> Value {
        Value::Fn(Callback(Arc::new(f), name, None))
    }
    /// Fonction avec attributs.
    pub fn func_attrs(
        name: &'static str,
        attrs: Map,
        f: impl Fn(&mut Game, &[Value]) -> Res<Value> + Send + Sync + 'static,
    ) -> Value {
        Value::Fn(Callback(Arc::new(f), name, Some(Arc::new(attrs))))
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}
impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}
impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i as i64)
    }
}
impl From<usize> for Value {
    fn from(i: usize) -> Self {
        Value::Int(i as i64)
    }
}
impl From<u32> for Value {
    fn from(i: u32) -> Self {
        Value::Int(i as i64)
    }
}
impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(Arc::from(s))
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(Arc::from(s.as_str()))
    }
}
impl From<O> for Value {
    fn from(o: O) -> Self {
        Value::Obj(o)
    }
}
impl From<R> for Value {
    fn from(r: R) -> Self {
        Value::Rune(r)
    }
}
impl From<ItemId> for Value {
    fn from(i: ItemId) -> Self {
        Value::Item(i)
    }
}
impl From<Loc> for Value {
    fn from(l: Loc) -> Self {
        Value::Loc(l)
    }
}
impl From<Map> for Value {
    fn from(m: Map) -> Self {
        Value::Map(m)
    }
}
impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::List(v)
    }
}
impl From<Vec<O>> for Value {
    fn from(v: Vec<O>) -> Self {
        Value::List(v.into_iter().map(Value::Obj).collect())
    }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(o: Option<T>) -> Self {
        match o {
            Some(x) => x.into(),
            None => Value::None,
        }
    }
}
impl From<OptV> for Value {
    fn from(o: OptV) -> Self {
        Value::Opt(Box::new(o))
    }
}
