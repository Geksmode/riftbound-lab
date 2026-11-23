//! Expressions régulières minimales (le moteur n'a aucune dépendance), pour traduire tels quels les motifs de
//! `replay.py` (journal de la table) : littéraux, `.`, `\d`, `\w`, `\s`, `\b`, classes `[...]`, groupes capturants
//! `(...)` et non capturants `(?:...)`, lookahead `(?=...)`, alternatives `|`, quantificateurs `* + ? {n,m}` avides
//! ou paresseux (`*?`), ancres `^ $`. Recherche par retour arrière, comme le module `re` de Python.

#[derive(Clone, Debug)]
enum Node {
    Char(char),
    Any,
    Digit(bool),
    Word(bool),
    Space(bool),
    Class(Vec<(char, char)>, bool),
    Start,
    End,
    WordB,
    Group(Vec<Vec<Node>>, Option<usize>),
    Look(Vec<Vec<Node>>),
    Repeat(Box<Node>, usize, usize, bool),
}

/// Motif compilé.
#[derive(Clone, Debug)]
pub struct Regex {
    alt: Vec<Vec<Node>>,
    ngroups: usize,
}

/// Résultat d'une correspondance : positions (en caractères) de chaque groupe ; le groupe 0 est l'ensemble.
#[derive(Clone, Debug)]
pub struct Captures {
    chars: Vec<char>,
    pub groups: Vec<Option<(usize, usize)>>,
}

impl Captures {
    /// `m[i]` : texte du groupe i, None s'il n'a pas participé.
    pub fn get(&self, i: usize) -> Option<String> {
        self.groups.get(i).copied().flatten().map(|(a, b)| self.chars[a..b].iter().collect())
    }
    /// `m[i]` d'un groupe qui participe toujours (vide sinon).
    pub fn s(&self, i: usize) -> String {
        self.get(i).unwrap_or_default()
    }
    pub fn start(&self) -> usize {
        self.groups[0].unwrap().0
    }
    pub fn end(&self) -> usize {
        self.groups[0].unwrap().1
    }
}

struct Parser<'a> {
    p: &'a [char],
    i: usize,
    ngroups: usize,
}

impl Parser<'_> {
    fn alt(&mut self) -> Vec<Vec<Node>> {
        let mut alts = vec![self.seq()];
        while self.i < self.p.len() && self.p[self.i] == '|' {
            self.i += 1;
            alts.push(self.seq());
        }
        alts
    }
    fn seq(&mut self) -> Vec<Node> {
        let mut out = vec![];
        while self.i < self.p.len() && self.p[self.i] != '|' && self.p[self.i] != ')' {
            let atom = self.atom();
            out.push(self.quant(atom));
        }
        out
    }
    fn quant(&mut self, atom: Node) -> Node {
        if self.i >= self.p.len() {
            return atom;
        }
        let (min, max) = match self.p[self.i] {
            '*' => (0, usize::MAX),
            '+' => (1, usize::MAX),
            '?' => (0, 1),
            '{' => {
                let j = self.p[self.i..].iter().position(|&c| c == '}').map(|x| x + self.i);
                let Some(j) = j else { return atom };
                let inner: String = self.p[self.i + 1..j].iter().collect();
                let (a, b) = match inner.split_once(',') {
                    Some((a, b)) => (a.parse().unwrap_or(0), if b.is_empty() { usize::MAX } else { b.parse().unwrap() }),
                    None => {
                        let n = inner.parse().unwrap();
                        (n, n)
                    }
                };
                self.i = j;
                (a, b)
            }
            _ => return atom,
        };
        self.i += 1;
        let mut greedy = true;
        if self.i < self.p.len() && self.p[self.i] == '?' {
            greedy = false;
            self.i += 1;
        }
        Node::Repeat(Box::new(atom), min, max, greedy)
    }
    fn esc(&mut self, c: char) -> Node {
        match c {
            'd' => Node::Digit(true),
            'D' => Node::Digit(false),
            'w' => Node::Word(true),
            'W' => Node::Word(false),
            's' => Node::Space(true),
            'S' => Node::Space(false),
            'b' => Node::WordB,
            'n' => Node::Char('\n'),
            't' => Node::Char('\t'),
            c => Node::Char(c),
        }
    }
    fn atom(&mut self) -> Node {
        let c = self.p[self.i];
        self.i += 1;
        match c {
            '.' => Node::Any,
            '^' => Node::Start,
            '$' => Node::End,
            '\\' => {
                let e = self.p[self.i];
                self.i += 1;
                self.esc(e)
            }
            '(' => {
                let mut cap = None;
                let mut look = false;
                if self.p[self.i..].starts_with(&['?', ':']) {
                    self.i += 2;
                } else if self.p[self.i..].starts_with(&['?', '=']) {
                    self.i += 2;
                    look = true;
                } else {
                    self.ngroups += 1;
                    cap = Some(self.ngroups);
                }
                let a = self.alt();
                assert_eq!(self.p.get(self.i), Some(&')'), "motif : ')' attendue");
                self.i += 1;
                if look {
                    Node::Look(a)
                } else {
                    Node::Group(a, cap)
                }
            }
            '[' => {
                let mut neg = false;
                if self.p[self.i] == '^' {
                    neg = true;
                    self.i += 1;
                }
                let mut items = vec![];
                let mut first = true;
                while self.p[self.i] != ']' || first {
                    first = false;
                    let mut a = self.p[self.i];
                    self.i += 1;
                    if a == '\\' {
                        a = self.p[self.i];
                        self.i += 1;
                        match a {
                            'd' => {
                                items.push(('0', '9'));
                                continue;
                            }
                            'n' => a = '\n',
                            _ => {}
                        }
                    }
                    if self.p[self.i] == '-' && self.p[self.i + 1] != ']' {
                        let b = self.p[self.i + 1];
                        self.i += 2;
                        items.push((a, b));
                    } else {
                        items.push((a, a));
                    }
                }
                self.i += 1;
                Node::Class(items, neg)
            }
            c => Node::Char(c),
        }
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

type Caps = Vec<Option<(usize, usize)>>;

struct M<'a> {
    s: &'a [char],
}

impl M<'_> {
    fn one(&self, n: &Node, pos: usize) -> Option<usize> {
        let c = *self.s.get(pos)?;
        let ok = match n {
            Node::Char(x) => c == *x,
            Node::Any => c != '\n',
            Node::Digit(y) => c.is_ascii_digit() == *y,
            Node::Word(y) => is_word(c) == *y,
            Node::Space(y) => c.is_whitespace() == *y,
            Node::Class(items, neg) => items.iter().any(|&(a, b)| a <= c && c <= b) != *neg,
            _ => unreachable!(),
        };
        ok.then_some(pos + 1)
    }

    fn alt(&self, alts: &[Vec<Node>], pos: usize, caps: &mut Caps, k: &mut dyn FnMut(usize, &mut Caps) -> bool) -> bool {
        for seq in alts {
            if self.seq(seq, pos, caps, k) {
                return true;
            }
        }
        false
    }

    fn seq(&self, seq: &[Node], pos: usize, caps: &mut Caps, k: &mut dyn FnMut(usize, &mut Caps) -> bool) -> bool {
        let Some((n, rest)) = seq.split_first() else { return k(pos, caps) };
        match n {
            Node::Start => pos == 0 && self.seq(rest, pos, caps, k),
            Node::End => pos == self.s.len() && self.seq(rest, pos, caps, k),
            Node::WordB => {
                let a = pos > 0 && is_word(self.s[pos - 1]);
                let b = pos < self.s.len() && is_word(self.s[pos]);
                a != b && self.seq(rest, pos, caps, k)
            }
            Node::Group(alts, cap) => {
                let cap = *cap;
                self.alt(alts, pos, caps, &mut |e, caps: &mut Caps| {
                    let old = cap.map(|c| caps[c]);
                    if let Some(c) = cap {
                        caps[c] = Some((pos, e));
                    }
                    if self.seq(rest, e, caps, k) {
                        return true;
                    }
                    if let (Some(c), Some(o)) = (cap, old) {
                        caps[c] = o;
                    }
                    false
                })
            }
            Node::Look(alts) => {
                let mut c2 = caps.clone();
                let ok = self.alt(alts, pos, &mut c2, &mut |_, _| true);
                ok && self.seq(rest, pos, caps, k)
            }
            Node::Repeat(inner, min, max, greedy) => self.rep(inner, *min, *max, *greedy, 0, pos, rest, caps, k),
            _ => match self.one(n, pos) {
                Some(p) => self.seq(rest, p, caps, k),
                None => false,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn rep(
        &self,
        inner: &Node,
        min: usize,
        max: usize,
        greedy: bool,
        count: usize,
        pos: usize,
        rest: &[Node],
        caps: &mut Caps,
        k: &mut dyn FnMut(usize, &mut Caps) -> bool,
    ) -> bool {
        let more = |caps: &mut Caps, k: &mut dyn FnMut(usize, &mut Caps) -> bool| -> bool {
            if count >= max {
                return false;
            }
            self.seq(std::slice::from_ref(inner), pos, caps, &mut |p, caps: &mut Caps| {
                if p == pos && count >= min {
                    return false; // pas de progrès : on arrête la répétition
                }
                self.rep(inner, min, max, greedy, count + 1, p, rest, caps, k)
            })
        };
        if greedy {
            if more(caps, k) {
                return true;
            }
            count >= min && self.seq(rest, pos, caps, k)
        } else {
            if count >= min && self.seq(rest, pos, caps, k) {
                return true;
            }
            more(caps, k)
        }
    }
}

impl Regex {
    pub fn new(pat: &str) -> Regex {
        let p: Vec<char> = pat.chars().collect();
        let mut ps = Parser { p: &p, i: 0, ngroups: 0 };
        let alt = ps.alt();
        assert!(ps.i == p.len(), "motif invalide : {pat}");
        Regex { alt, ngroups: ps.ngroups }
    }

    fn at(&self, chars: &[char], pos: usize) -> Option<Caps> {
        let m = M { s: chars };
        let mut caps: Caps = vec![None; self.ngroups + 1];
        let mut end = None;
        let mut res = None;
        if m.alt(&self.alt, pos, &mut caps, &mut |e, c: &mut Caps| {
            end = Some(e);
            res = Some(c.clone());
            true
        }) {
            let mut c = res.unwrap();
            c[0] = Some((pos, end.unwrap()));
            return Some(c);
        }
        None
    }

    /// `re.match(pat, s)` : correspondance au début du texte.
    pub fn match_(&self, s: &str) -> Option<Captures> {
        let chars: Vec<char> = s.chars().collect();
        let g = self.at(&chars, 0)?;
        Some(Captures { chars, groups: g })
    }

    /// `re.finditer(pat, s)`
    pub fn find_iter(&self, s: &str) -> Vec<Captures> {
        let chars: Vec<char> = s.chars().collect();
        let mut out = vec![];
        let mut pos = 0;
        while pos <= chars.len() {
            match self.at(&chars, pos) {
                Some(g) => {
                    let (a, b) = g[0].unwrap();
                    out.push(Captures { chars: chars.clone(), groups: g });
                    pos = if b == a { b + 1 } else { b };
                }
                None => pos += 1,
            }
        }
        out
    }

    /// `re.sub(pat, f, s)` : chaque correspondance remplacée par `f(m)`.
    pub fn sub(&self, s: &str, f: impl Fn(&Captures) -> String) -> String {
        let chars: Vec<char> = s.chars().collect();
        let mut out = String::new();
        let mut last = 0;
        for m in self.find_iter(s) {
            let (a, b) = m.groups[0].unwrap();
            out.extend(chars[last..a].iter());
            out.push_str(&f(&m));
            last = b;
        }
        out.extend(chars[last..].iter());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motifs_du_journal() {
        let r = Regex::new(r"^P(\d) plays (.+?) from (hand|champ|facedown|trash) ?(.*)$");
        let m = r.match_("P0 plays Akali#3 from hand (tg=#5,)").unwrap();
        assert_eq!(m.s(1), "0");
        assert_eq!(m.s(2), "Akali#3");
        assert_eq!(m.s(3), "hand");
        assert_eq!(m.s(4), "(tg=#5,)");
        let r = Regex::new(r"^P(\d) activates (.+?) ?(\(.*\))?$");
        let m = r.match_("P1 activates Foo#2").unwrap();
        assert_eq!(m.s(2), "Foo#2");
        assert_eq!(m.get(3), None);
        let r = Regex::new(r"(\w+)=(.*?)(?= \w+=|$)");
        let v: Vec<(String, String)> = r.find_iter("tg=#5,#6, loc=1 x=2").iter().map(|m| (m.s(1), m.s(2))).collect();
        assert_eq!(v, vec![("tg".into(), "#5,#6,".into()), ("loc".into(), "1".into()), ("x".into(), "2".into())]);
        assert_eq!(Regex::new(r"#(\d+)").sub("Akali#12 et Yi#3", |_| String::new()), "Akali et Yi");
        assert_eq!(Regex::new(r"\bP([01])\b").sub("P0 gagne, XP1 non", |m| format!("<{}>", m.s(1))), "<0> gagne, XP1 non");
    }
}
