//! `game.py`, suite : question aux joueurs (`ask`), burn out, regard, révélation, Predict, points, runes et coûts.
use std::collections::BTreeSet;

use crate::cards::registry;
use crate::game::{Counter, Game, Res, Zone, O, R};
use crate::spec::DOMAINS;
use crate::value::{Map, Value};

/// Ensemble de domaines permis pour un power (`frozenset` de Python). Trié, comme `sorted(req)`.
pub type Req = BTreeSet<String>;

/// `ANY`
pub fn any_req() -> Req {
    DOMAINS.iter().map(|d| d.to_string()).collect()
}

/// Une capacité [Add] utilisable en payant : (source, dict de la capacité).
pub type Adder = (O, Map);

/// Un plan de paiement (`dict` rendu par `_plan_payment`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub e: i64,
    pub pool_e_after: i64,
    pub pool_p: Counter,
    pub exhaust: Vec<R>,
    pub recycle: Vec<R>,
    pub gold: i64,
    pub adders: Vec<Adder>,
    pub took: i64,
    /// conversions (`plan_convert`) : dict(o=, ad=, n=, left_e= | left_p=)
    pub conv: Vec<(O, Map, i64, Map)>,
}

/// Choix d'un humain en payant (`pay_choices`) : rune suivante recyclée, ou nombre de Gold.
#[derive(Clone, Debug, PartialEq)]
pub enum PayKey {
    Rune(u32),
    Gold(i64),
}

impl PayKey {
    pub fn value(&self) -> Value {
        match self {
            PayKey::Rune(u) => Value::Tuple(vec!["r".into(), (*u).into()]),
            PayKey::Gold(k) => (*k).into(),
        }
    }
}

/// `plans=alts` de `pay` : pour chaque option (même ordre), le plan de paiement `{gold, recycle}` (runes recyclées).
fn plans_value(alts: &[(PayKey, Plan)]) -> Value {
    Value::List(
        alts.iter()
            .map(|(_, p)| {
                crate::map! { gold = p.gold, recycle = Value::List(p.recycle.iter().map(|&r| Value::Rune(r)).collect()) }
                    .into()
            })
            .collect(),
    )
}

fn reqs_value(reqs: &[Req]) -> Value {
    Value::List(reqs.iter().map(|r| Value::List(r.iter().map(|d| Value::from(d.as_str())).collect())).collect())
}

/// Lit une liste de `Req` rangée dans une valeur (liste de listes de domaines).
pub fn reqs_of(v: &Value) -> Vec<Req> {
    v.list().iter().map(|r| r.list().iter().map(|d| d.str().to_string()).collect()).collect()
}

impl Game {
    /// `ask(pid, kind, options, **ctx)` : choix synchrone d'un joueur. None sans option ; la seule option sans
    /// question, sauf pour 'may', 'may_target' et 'order'.
    pub fn ask(&mut self, pid: usize, kind: &str, options: Vec<Value>, ctx: Map) -> Res<Value> {
        if self.tg.is_some() {
            // TGame.ask : cartes vues par l'effet qui pose la question ; réponses de l'IA notées dans son journal
            crate::train::tg_ask_pre(self, pid, &options);
            let r = self.ask_base(pid, kind, options.clone(), ctx)?;
            crate::train::tg_ask_post(self, pid, kind, &options, &r);
            return Ok(r);
        }
        self.ask_base(pid, kind, options, ctx)
    }

    /// `Game.ask` (sans les ajouts de la table)
    fn ask_base(&mut self, pid: usize, kind: &str, options: Vec<Value>, ctx: Map) -> Res<Value> {
        if options.is_empty() {
            return Ok(Value::None);
        }
        if options.len() == 1 && !matches!(kind, "may" | "may_target" | "order") {
            return Ok(options.into_iter().next().unwrap());
        }
        let mut agent = std::mem::replace(&mut self.agents[pid], Box::new(NoAgent));
        let r = agent.choose(self, pid, kind, &options, &ctx);
        self.agents[pid] = agent;
        let i = r?;
        if self.ck.on && self.ck.ckpt.is_some() {
            // (type, nombre d'options, indice de la réponse, (valeur,) si simple : None, booléen, nombre, texte)
            let v = &options[i];
            let simple = matches!(v, Value::None | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Str(_) | Value::Loc(_));
            self.ck.asklog.push((kind.to_string(), options.len(), Some(i), if simple { Some(v.clone()) } else { None }));
        }
        Ok(options[i].clone())
    }

    /// `ask` dont les options sont des objets : renvoie l'objet choisi (None sans option).
    pub fn ask_obj(&mut self, pid: usize, kind: &str, options: &[O], ctx: Map) -> Res<Option<O>> {
        let v = self.ask(pid, kind, options.iter().map(|&o| Value::Obj(o)).collect(), ctx)?;
        Ok(v.opt_obj())
    }

    /// `ask(pid, kind, [False, True] ...)` ou `[True, False]` : renvoie le booléen choisi.
    pub fn ask_bool(&mut self, pid: usize, kind: &str, options: [bool; 2], ctx: Map) -> Res<bool> {
        Ok(self.ask(pid, kind, options.iter().map(|&b| Value::Bool(b)).collect(), ctx)?.truthy())
    }

    pub fn agent_every_choice(&self, pid: usize) -> bool {
        self.agents.get(pid).is_some_and(|a| a.every_choice())
    }

    /// `burn_out(pid)` (431.2) : la défausse revient dans le deck, un adversaire gagne 1 point.
    pub fn burn_out(&mut self, pid: usize) -> Res<()> {
        self.log(|| format!("BURN OUT {pid}"));
        self.stats.add("burnout", 1);
        let mut tr = std::mem::take(&mut self.p[pid].trash);
        self.rng.shuffle(&mut tr);
        for &c in &tr {
            self.o_mut(c).zone = Some(Zone::Deck);
        }
        self.p[pid].deck.extend(tr);
        self.gain_point(1 - pid, "burn out")?;
        if self.p[1 - pid].points >= self.victory && self.p[1 - pid].points > self.p[pid].points {
            self.win(1 - pid)?;
        }
        Ok(())
    }

    /// `recycle_cards(pid, cards)`
    pub fn recycle_cards(&mut self, pid: usize, cards: &[O]) -> Res<()> {
        let mut cards = cards.to_vec();
        self.rng.shuffle(&mut cards);
        for &c in &cards {
            self.to_zone(c, "deck", true, None)?;
        }
        if !cards.is_empty() && self.stage != "setup" {
            self.emit("recycle", crate::map! { pid = pid, cards = cards })?;
        }
        Ok(())
    }

    /// `look(pid, cards, owner)` : regarder le dessus d'un deck sans piocher (Predict, Vision...) ; émet 'look' et
    /// l'`on_seen` des cartes vues (Nocturne, Horrifying).
    pub fn look(&mut self, pid: usize, cards: &[O], owner: Option<usize>) -> Res<Vec<O>> {
        let cards = cards.to_vec();
        if cards.is_empty() {
            return Ok(cards);
        }
        let owner = owner.unwrap_or(pid);
        if owner == pid {
            for &c in &cards {
                let u = self.o(c).uid;
                self.p[pid].seen.insert(u);
            }
        }
        self.emit("look", crate::map! { pid = pid, cards = cards.clone(), owner = owner })?;
        if let Some(names) = registry().hook_names("on_seen") {
            for &c in &cards {
                if names.contains(self.cname(c)) && self.o(c).zone == Some(Zone::Deck) {
                    let im = self.impl_o(c).unwrap();
                    im.hook("on_seen").call(self, &[Value::Obj(c), pid.into()])?;
                }
            }
        }
        Ok(cards)
    }

    /// `reveal_hand(pid, to)` (424.3.a)
    pub fn reveal_hand(&mut self, pid: usize, to: usize) -> Res<Vec<O>> {
        let cards = self.p[pid].hand.clone();
        let uids: Vec<Value> = cards.iter().map(|&c| Value::from(self.o(c).uid)).collect();
        self.shown_hand = Some(crate::map! { pid = pid, to = to, uids = Value::List(uids), turn = self.turn_no });
        let rs: Vec<String> = cards.iter().map(|&c| self.repr(c)).collect();
        self.log(|| format!("  P{pid} reveals their hand [{}]", rs.join(", ")));
        self.emit("reveal_hand", crate::map! { pid = pid, cards = cards.clone(), to = to })?;
        Ok(cards)
    }

    /// `reveal(pid, n, owner, look_first, until)` : révèle le dessus d'un deck (Void Hatchling, Undertitan).
    pub fn reveal(
        &mut self,
        pid: usize,
        n: usize,
        owner: Option<usize>,
        look_first: bool,
        until: Option<&dyn Fn(&Game, O) -> bool>,
    ) -> Res<Vec<O>> {
        let owner = owner.unwrap_or(pid);
        if look_first && !self.p[owner].deck.is_empty() {
            for (src, _im) in self.providers("reveal_rep") {
                if self.o(src.obj()).ctrl == pid && !self.p[owner].deck.is_empty() {
                    let top = self.p[owner].deck[0];
                    self.look(pid, &[top], Some(owner))?;
                    if self.p[owner].deck.contains(&top)
                        && self.ask_bool(pid, "hatchling_recycle", [false, true], crate::map! { card = top })?
                    {
                        let i = self.p[owner].deck.iter().position(|&x| x == top).unwrap();
                        self.p[owner].deck.remove(i);
                        self.recycle_cards(owner, &[top])?;
                    }
                }
            }
        }
        let cards: Vec<O> = match until {
            Some(u) => {
                let mut cs = vec![];
                for &c in &self.p[owner].deck {
                    cs.push(c);
                    if u(self, c) {
                        break;
                    }
                }
                cs
            }
            None => self.p[owner].deck.iter().take(n).copied().collect(),
        };
        for &c in &cards {
            if let Some(im) = self.impl_o(c) {
                let h = im.hook("on_reveal");
                if !h.is_none() {
                    h.call(self, &[Value::Obj(c), pid.into()])?;
                }
            }
        }
        if !cards.is_empty() {
            let rs: Vec<String> = cards.iter().map(|&c| self.repr(c)).collect();
            self.log(|| format!("  P{pid} reveals [{}]", rs.join(", ")));
            self.emit("reveal", crate::map! { pid = pid, cards = cards.clone(), owner = owner })?;
        }
        Ok(cards)
    }

    /// `burn(pid, n)` (440) : le dessus du deck à la défausse, burn out si le deck est vide (440.4).
    pub fn burn(&mut self, pid: usize, n: i64) -> Res<Vec<O>> {
        let mut burned = vec![];
        for _ in 0..n {
            if self.p[pid].deck.is_empty() {
                self.burn_out(pid)?;
                if self.winner.is_some() {
                    return Ok(burned);
                }
                if self.p[pid].deck.is_empty() {
                    continue;
                }
            }
            let c = self.p[pid].deck.remove(0);
            self.o_mut(c).zone = Some(Zone::Trash);
            self.p[pid].trash.push(c);
            burned.push(c);
        }
        if !burned.is_empty() {
            let rs: Vec<String> = burned.iter().map(|&c| self.repr(c)).collect();
            self.log(|| format!("  P{pid} burns [{}]", rs.join(", ")));
            self.emit("burn", crate::map! { pid = pid, cards = burned.clone() })?;
        }
        Ok(burned)
    }

    /// `predict(pid, n)` (436) : regarder N cartes, en recycler, remettre les autres dessus dans l'ordre choisi.
    pub fn predict(&mut self, pid: usize, n: usize) -> Res<Vec<O>> {
        let top: Vec<O> = self.p[pid].deck.iter().take(n).copied().collect();
        if top.is_empty() {
            return Ok(vec![]);
        }
        self.look(pid, &top, None)?;
        let now: Vec<O> = self.p[pid].deck.iter().take(n).copied().collect();
        let top: Vec<O> = top.into_iter().filter(|c| now.contains(c)).collect();
        let (mut keep, mut rec) = (vec![], vec![]);
        for (i, &c) in top.iter().enumerate() {
            let ctx = crate::map! { card = c, n = top.len(), i = i, top = top.clone() };
            if self.ask_bool(pid, "predict_recycle", [false, true], ctx)? {
                rec.push(c);
            } else {
                keep.push(c);
            }
        }
        self.p[pid].deck.drain(..top.len());
        let mut order = vec![];
        while !keep.is_empty() {
            let c = if keep.len() > 1 {
                self.ask_obj(pid, "predict_top", &keep.clone(), Map::new())?.unwrap()
            } else {
                keep[0]
            };
            let i = keep.iter().position(|&x| x == c).unwrap();
            keep.remove(i);
            order.push(c);
        }
        self.p[pid].deck.splice(0..0, order);
        let (nt, nr) = (top.len(), rec.len());
        self.log(|| format!("  P{pid} predicts {nt}: recycles {nr}"));
        if !rec.is_empty() {
            self.recycle_cards(pid, &rec)?;
        }
        Ok(rec)
    }

    /// `add_pool(pid, e, power)` : [Add] à la réserve (429). power : domaines, 'A' = n'importe lequel.
    pub fn add_pool(&mut self, pid: usize, e: i64, power: &[&str]) {
        self.p[pid].pool_e += e;
        for d in power {
            self.p[pid].pool_p.add(d, 1);
        }
        if e != 0 || !power.is_empty() {
            let ps: Vec<String> = power.iter().map(|d| format!("'{d}'")).collect();
            self.log(|| format!("  P{pid} adds {e} energy [{}]", ps.join(", ")));
        }
    }

    /// `gain_point(pid, why)` : « Can't score points » (score_veto) arrête tous les points sauf celui du burn out.
    pub fn gain_point(&mut self, pid: usize, why: &str) -> Res<bool> {
        if why != "burn out" {
            for (src, im) in self.providers("score_veto") {
                if im.hook("score_veto").call(self, &[src.clone(), pid.into(), why.into()])?.truthy() {
                    let n = match &src {
                        Value::Obj(o) => self.o(*o).name.clone(),
                        Value::Int(i) => self.bfs[*i as usize].name.clone(),
                        v => format!("{v:?}"),
                    };
                    self.log(|| format!("  P{pid} can't score ({n})"));
                    return Ok(false);
                }
            }
        }
        self.p[pid].points += 1;
        let pts = self.p[pid].points;
        self.log(|| format!("P{pid} +1 point ({why}) -> {pts}"));
        self.emit("point", crate::map! { pid = pid, why = why })?;
        Ok(true)
    }

    /// `win(pid)` : lève GameOver.
    pub fn win(&mut self, pid: usize) -> Res<()> {
        if self.winner.is_none() {
            self.winner = Some(pid as i64);
            self.log(|| format!("WINNER {pid}"));
        }
        Err(crate::game::GameOver)
    }

    // ------------------------------------------------------------------ runes et coûts
    /// `channel(pid, n, exhausted)` (430) ; émet 'channel'.
    pub fn channel(&mut self, pid: usize, n: i64, exhausted: bool) -> Res<Vec<R>> {
        let mut got = vec![];
        let mut extra = 0;
        if n > 0 && self.stage != "setup" {
            // « If you would channel runes, channel that many runes plus 1 exhausted rune instead. » (Seraphic Wyvern)
            for (src, im) in self.providers("channel_rep") {
                extra += im.hook("channel_rep").call(self, &[src, pid.into()])?.opt_int().unwrap_or(0);
            }
        }
        for i in 0..(n + extra) {
            if self.p[pid].rune_deck.is_empty() {
                break;
            }
            let r = self.p[pid].rune_deck.remove(0);
            self.r_mut(r).exhausted = exhausted || i >= n;
            self.p[pid].runes.push(r);
            got.push(r);
        }
        if !got.is_empty() && self.stage != "setup" {
            let rv: Vec<Value> = got.iter().map(|&r| Value::Rune(r)).collect();
            self.emit("channel", crate::map! { pid = pid, runes = Value::List(rv) })?;
        }
        Ok(got)
    }

    /// `recycle_rune(pid, r)` (430.3) : sous le deck de runes ; émet 'rune_recycle'.
    pub fn recycle_rune(&mut self, pid: usize, r: R) -> Res<()> {
        let i = self.p[pid].runes.iter().position(|&x| x == r).unwrap();
        self.p[pid].runes.remove(i);
        self.r_mut(r).exhausted = false;
        self.p[pid].rune_deck.push(r);
        if self.stage != "setup" {
            self.emit("rune_recycle", crate::map! { pid = pid, rune = r })?;
        }
        Ok(())
    }

    /// `golds(pid)` : Gold prêts du joueur.
    pub fn golds(&self, pid: usize) -> Vec<O> {
        self.board
            .iter()
            .copied()
            .filter(|&o| self.o(o).ctrl == pid && matches!(self.cname(o), "Gold" | "Gold // Buff") && !self.o(o).exh)
            .collect()
    }

    /// `_adder_pairs(pid)` : (source, capacité [Add]) du joueur, avant les contrôles d'utilisabilité.
    fn adder_pairs(&mut self, pid: usize) -> Res<Vec<Adder>> {
        let names = &registry().adders;
        let grants = registry().hook_names("grant_add");
        if names.is_empty() && grants.is_none() {
            return Ok(vec![]);
        }
        let mut srcs: Vec<O> =
            self.board.iter().copied().filter(|&o| self.o(o).ctrl == pid && names.contains(self.cname(o))).collect();
        let legend = self.p[pid].legend;
        if names.contains(&self.p[pid].legend_name) {
            srcs.push(legend);
        }
        let mut pairs = vec![];
        for o in srcs {
            let im = if o != legend { self.impl_o(o) } else { self.impl_n(&self.p[pid].legend_name) }.unwrap();
            pairs.extend(im.add.iter().map(|ad| (o, ad.clone())));
        }
        if grants.is_some() {
            // capacités [Add] données par d'autres objets (grant_add : Heimerdinger)
            for (o, im) in self.providers("grant_add") {
                let Value::Obj(o) = o else { continue };
                if self.o(o).ctrl == pid && self.o(o).zone == Some(Zone::Board) {
                    let r = im.hook("grant_add").call(self, &[Value::Obj(o)])?;
                    pairs.extend(r.list().iter().map(|ad| (o, ad.map().clone())));
                }
            }
        }
        Ok(pairs)
    }

    /// `_pay_ctx_free(pid)`
    pub fn pay_ctx_free(&mut self, pid: usize) -> Res<bool> {
        Ok(self.p[pid].pool_r.is_empty() && !self.adder_pairs(pid)?.iter().any(|(_, ad)| ad.contains("can")))
    }

    /// `adders(pid, ctx)` : capacités [Add] utilisables en payant (429.3).
    pub fn adders(&mut self, pid: usize, ctx: &Value) -> Res<Vec<Adder>> {
        let mut out = vec![];
        for (o, ad) in self.adder_pairs(pid)? {
            let exhaust = ad.get("exhaust").map(|v| v.truthy()).unwrap_or(true);
            if exhaust && self.o(o).exh {
                continue;
            }
            if ad.g("timing").opt_str() == Some("action") {
                let blocked = !self.chain.is_empty()
                    || self.sd.as_ref().is_some_and(|sd| sd.focus != pid)
                    || (self.sd.is_none() && self.tp != pid);
                if blocked {
                    continue; // [Action] Add : seulement quand le joueur pourrait jouer une Action
                }
            }
            let can = ad.g("can");
            if !can.is_none() && !can.call(self, &[pid.into(), Value::Obj(o), ctx.clone()])?.truthy() {
                continue;
            }
            out.push((o, ad));
        }
        Ok(out)
    }

    /// `gold_extra(pid)` : énergie qu'un Gold ajoute en plus de son [A] (Renata Glasc).
    pub fn gold_extra(&mut self, pid: usize) -> Res<i64> {
        let mut n = 0;
        for (src, im) in self.providers("gold_extra") {
            n += im.hook("gold_extra").call(self, &[src, pid.into()])?.opt_int().unwrap_or(0);
        }
        Ok(n)
    }

    /// `plan_payment(pid, e, reqs, ctx, _skip)` : un moyen de payer e énergie et les powers reqs ; les
    /// convertisseurs ne sont essayés que si le paiement simple est impossible.
    pub fn plan_payment(&mut self, pid: usize, e: i64, reqs: &[Req], ctx: &Value, skip: &[O]) -> Res<Option<Plan>> {
        let mut plan = self.plan_payment_inner(pid, e, reqs, ctx, skip, 0, &[])?;
        if plan.is_none() && registry().converters {
            plan = self.plan_convert(pid, e, reqs, ctx, skip)?;
        }
        Ok(plan)
    }

    /// `plan_convert` : capacités [Add] dont le coût se paie en ressources ou en tuant (p2e, e2p, kill).
    fn plan_convert(&mut self, pid: usize, e: i64, reqs: &[Req], ctx: &Value, skip: &[O]) -> Res<Option<Plan>> {
        for (o, ad) in self.adders(pid, ctx)? {
            let conv = ad.g("conv");
            if conv.is_none() || skip.contains(&o) {
                continue;
            }
            let mut sk = skip.to_vec();
            sk.push(o);
            let by_len = |reqs: &[Req]| {
                let mut v = reqs.to_vec();
                v.sort_by_key(|q| q.len());
                v
            };
            match conv.str() {
                "p2e" => {
                    let ns: Vec<i64> = if ad.g("var").truthy() { (1..=e).collect() } else { vec![ad.g("n").int()] };
                    for n in ns {
                        let mut r2 = reqs.to_vec();
                        for _ in 0..n {
                            r2.push(any_req());
                        }
                        if let Some(mut sub) = self.plan_payment(pid, 0.max(e - n), &r2, ctx, &sk)? {
                            sub.conv.push((o, ad.clone(), n, crate::map! { left_e = 0.max(n - e) }));
                            return Ok(Some(sub));
                        }
                    }
                }
                "e2p" => {
                    for n in 1..=reqs.len() {
                        let rest: Vec<Req> = by_len(reqs)[n..].to_vec();
                        if let Some(mut sub) = self.plan_payment(pid, e + n as i64, &rest, ctx, &sk)? {
                            sub.conv.push((o, ad.clone(), n as i64, crate::map! { left_e = 0 }));
                            return Ok(Some(sub));
                        }
                    }
                }
                "kill" => {
                    if !ad.g("victims").call(self, &[pid.into(), Value::Obj(o)])?.truthy() {
                        continue;
                    }
                    let k = ad.g("p").list().len();
                    let sorted = by_len(reqs);
                    let rest: Vec<Req> = sorted.get(k..).map(|x| x.to_vec()).unwrap_or_default();
                    if let Some(mut sub) = self.plan_payment(pid, e, &rest, ctx, &sk)? {
                        let left_p = 0.max(k as i64 - reqs.len() as i64);
                        sub.conv.push((o, ad.clone(), k as i64, crate::map! { left_p = left_p }));
                        return Ok(Some(sub));
                    }
                }
                c => panic!("conversion inconnue {c}"),
            }
        }
        Ok(None)
    }

    /// `_plan_payment(pid, e, reqs, ctx, _skip, gold_first, prefer)`. gold_first : Gold dépensés pour du power
    /// avant de recycler une rune (choix du joueur) ; prefer : uids des runes à recycler d'abord.
    #[allow(clippy::too_many_arguments)]
    pub fn plan_payment_inner(
        &mut self,
        pid: usize,
        e: i64,
        reqs: &[Req],
        ctx: &Value,
        skip: &[O],
        gold_first: i64,
        prefer: &[u32],
    ) -> Res<Option<Plan>> {
        let mut pool_e = self.p[pid].pool_e;
        if !self.p[pid].pool_r.is_empty() {
            for (n, only) in self.p[pid].pool_r.clone() {
                if only.call(self, &[pid.into(), ctx.clone()])?.truthy() {
                    pool_e += n;
                }
            }
        }
        let mut pool_p = self.p[pid].pool_p.clone();
        let runes = self.p[pid].runes.clone();
        let ready: Vec<R> = runes.iter().copied().filter(|&r| !self.r(r).exhausted).collect();
        let exh: Vec<R> = runes.iter().copied().filter(|&r| self.r(r).exhausted).collect();
        let golds = self.golds(pid);
        let mut recycle: Vec<R> = vec![];
        let mut use_gold: i64 = 0;
        let adders: Vec<Adder> = self
            .adders(pid, ctx)?
            .into_iter()
            .filter(|a| a.1.g("conv").is_none() && !skip.contains(&a.0))
            .collect();
        let soft: Vec<Adder> = adders.iter().filter(|a| !a.1.g("kill").truthy()).cloned().collect();
        let hard: Vec<Adder> = adders.iter().filter(|a| a.1.g("kill").truthy()).cloned().collect();
        let mut used: Vec<Adder> = vec![];
        let mut used_ids: Vec<O> = vec![];
        let mut extra: i64 = 0;

        fn use_(a: &Adder, used: &mut Vec<Adder>, used_ids: &mut Vec<O>, extra: &mut i64, pool_p: &mut Counter) {
            used.push(a.clone());
            used_ids.push(a.0);
            *extra += a.1.g("e").opt_int().unwrap_or(0);
            for d in a.1.g("p").list() {
                pool_p.add(d.str(), 1);
            }
        }
        fn find(req: Option<&Req>, lst: &[Adder], energy: bool, used_ids: &[O]) -> Option<Adder> {
            for a in lst {
                if used_ids.contains(&a.0) {
                    continue;
                }
                if energy && a.1.g("e").opt_int().unwrap_or(0) > 0 {
                    return Some(a.clone());
                }
                if !energy {
                    let req = req.unwrap();
                    if a.1.g("p").list().iter().any(|d| d.str() == "A" || req.contains(d.str())) {
                        return Some(a.clone());
                    }
                }
            }
            None
        }
        fn from_pool(req: &Req, pool_p: &mut Counter) -> bool {
            if !pool_p.truthy() {
                return false;
            }
            if let Some(dom) = req.iter().find(|d| pool_p.get(d) > 0) {
                pool_p.add(&dom.clone(), -1);
                return true;
            }
            if pool_p.get("A") > 0 {
                pool_p.add("A", -1);
                return true;
            }
            false
        }
        // demande en domaines du reste de la main, pour garder le domaine le plus rare
        let mut need: Option<Counter> = None;
        let demand = |g: &Game, need: &mut Option<Counter>| -> Counter {
            if need.is_none() {
                let mut c = Counter::default();
                for &h in &g.p[pid].hand {
                    let sp = g.spec(h);
                    for d in &sp.domains {
                        c.add(d, sp.p as i64);
                    }
                }
                *need = Some(c);
            }
            need.clone().unwrap()
        };
        let rune_count = |g: &Game| -> Counter {
            let mut c = Counter::default();
            for &r in &g.p[pid].runes {
                c.add(&g.r(r).domain, 1);
            }
            c
        };
        let mut sreqs = reqs.to_vec();
        sreqs.sort_by_key(|q| q.len());
        for req in &sreqs {
            if from_pool(req, &mut pool_p) {
                continue;
            }
            if let Some(a) = find(Some(req), &soft, false, &used_ids) {
                use_(&a, &mut used, &mut used_ids, &mut extra, &mut pool_p);
                from_pool(req, &mut pool_p);
                continue;
            }
            if use_gold < gold_first.min(golds.len() as i64) {
                use_gold += 1;
                continue;
            }
            let mut cands: Vec<R> = runes
                .iter()
                .copied()
                .filter(|&r| prefer.contains(&self.r(r).uid) && req.contains(&self.r(r).domain) && !recycle.contains(&r))
                .collect();
            if cands.is_empty() {
                cands = exh.iter().copied().filter(|&r| req.contains(&self.r(r).domain) && !recycle.contains(&r)).collect();
            }
            if cands.is_empty() {
                cands = ready.iter().copied().filter(|&r| req.contains(&self.r(r).domain) && !recycle.contains(&r)).collect();
            }
            if !cands.is_empty() {
                let (cnt, nd) = (rune_count(self), demand(self, &mut need));
                cands.sort_by(|&a, &b| {
                    let (da, db) = (&self.r(a).domain, &self.r(b).domain);
                    (nd.get(da) - cnt.get(da), da).cmp(&(nd.get(db) - cnt.get(db), db))
                });
                recycle.push(cands[0]);
                continue;
            }
            if use_gold < golds.len() as i64 {
                use_gold += 1;
                continue;
            }
            if let Some(a) = find(Some(req), &hard, false, &used_ids) {
                use_(&a, &mut used, &mut used_ids, &mut extra, &mut pool_p);
                from_pool(req, &mut pool_p);
                continue;
            }
            return Ok(None);
        }
        // énergie
        let mut exhaust: Vec<R> = vec![];
        let mut e_left = e;
        let gx = if !golds.is_empty() { self.gold_extra(pid)? } else { 0 };
        let mut avail = pool_e + extra + gx * use_gold;
        let take = avail.min(e_left);
        avail -= take;
        e_left -= take;
        let took = take;
        let rec_ready: Vec<R> = recycle.iter().copied().filter(|&r| !self.r(r).exhausted).collect();
        for r in rec_ready {
            if e_left <= 0 {
                break;
            }
            exhaust.push(r);
            e_left -= 1;
        }
        for which in 0..3 {
            if which == 1 {
                let mut others: Vec<R> = ready.iter().copied().filter(|r| !recycle.contains(r)).collect();
                let (cnt, nd) = (rune_count(self), demand(self, &mut need));
                others.sort_by(|&a, &b| {
                    let (da, db) = (&self.r(a).domain, &self.r(b).domain);
                    (nd.get(da) - cnt.get(da), da).cmp(&(nd.get(db) - cnt.get(db), db))
                });
                for r in others {
                    if e_left <= 0 {
                        break;
                    }
                    exhaust.push(r);
                    e_left -= 1;
                }
                continue;
            }
            let lst = if which == 0 { &soft } else { &hard };
            while e_left > 0 {
                let Some(a) = find(None, lst, true, &used_ids) else { break };
                let before = extra;
                use_(&a, &mut used, &mut used_ids, &mut extra, &mut pool_p);
                avail += extra - before;
                let t = avail.min(e_left);
                avail -= t;
                e_left -= t;
            }
        }
        while e_left > 0 && gx != 0 && use_gold < golds.len() as i64 {
            use_gold += 1; // Gold utilisé pour son énergie en plus, son [A] flotte
            pool_p.add("A", 1);
            let t = gx.min(e_left);
            avail += gx - t;
            e_left -= t;
        }
        if e_left > 0 {
            return Ok(None);
        }
        Ok(Some(Plan {
            e,
            pool_e_after: avail,
            pool_p,
            exhaust,
            recycle,
            gold: use_gold,
            adders: used,
            took,
            conv: vec![],
        }))
    }

    /// `can_pay(pid, e, reqs, ctx)`
    pub fn can_pay(&mut self, pid: usize, e: i64, reqs: &[Req], ctx: &Value) -> Res<bool> {
        Ok(self.plan_payment(pid, e, reqs, ctx, &[])?.is_some())
    }

    /// `gold_choices` : les façons de payer avec un Gold prêt au lieu de recycler une rune (429.3), {nombre de Gold:
    /// plan}, le plan par défaut d'abord.
    pub fn gold_choices(&mut self, pid: usize, e: i64, reqs: &[Req], ctx: &Value, plan: &Plan) -> Res<Vec<(i64, Plan)>> {
        let mut alts = vec![(plan.gold, plan.clone())];
        let ng = self.golds(pid).len() as i64;
        if !plan.conv.is_empty() || plan.recycle.is_empty() || plan.gold >= ng {
            return Ok(alts);
        }
        for k in (plan.gold + 1)..=ng.min(reqs.len() as i64) {
            if let Some(alt) = self.plan_payment_inner(pid, e, reqs, ctx, &[], k, &[])? {
                let maxk = alts.iter().map(|a| a.0).max().unwrap();
                let best = &alts.iter().find(|a| a.0 == maxk).unwrap().1;
                if alt.recycle.len() < best.recycle.len() {
                    alts.push((k, alt));
                }
            }
        }
        Ok(alts)
    }

    /// `pay_choices` : ce qu'un humain choisit en payant du power (429.3) : la prochaine rune recyclée, ou un
    /// nombre de Gold.
    #[allow(clippy::too_many_arguments)]
    pub fn pay_choices(
        &mut self,
        pid: usize,
        e: i64,
        reqs: &[Req],
        ctx: &Value,
        plan: &Plan,
        kf: i64,
        chosen: &[u32],
    ) -> Res<Vec<(PayKey, Plan)>> {
        let mut out = vec![];
        let rec: Vec<u32> = plan.recycle.iter().map(|&r| self.r(r).uid).collect();
        let nxt = rec.iter().copied().find(|u| !chosen.contains(u));
        if let Some(nxt) = nxt {
            out.push((PayKey::Rune(nxt), plan.clone()));
            for r in self.p[pid].runes.clone() {
                let ru = self.r(r).uid;
                if chosen.contains(&ru) || ru == nxt {
                    continue;
                }
                let mut pref = chosen.to_vec();
                pref.push(ru);
                if let Some(alt) = self.plan_payment_inner(pid, e, reqs, ctx, &[], kf, &pref)? {
                    let alt_rec: Vec<u32> = alt.recycle.iter().map(|&x| self.r(x).uid).collect();
                    if alt.gold == plan.gold && alt_rec.len() == rec.len() && pref.iter().all(|u| alt_rec.contains(u)) {
                        out.push((PayKey::Rune(ru), alt));
                    }
                }
            }
            let ng = self.golds(pid).len() as i64;
            for k in (plan.gold + 1)..=ng.min(reqs.len() as i64) {
                if let Some(alt) = self.plan_payment_inner(pid, e, reqs, ctx, &[], k, chosen)? {
                    let alt_rec: Vec<u32> = alt.recycle.iter().map(|&x| self.r(x).uid).collect();
                    if alt_rec.len() < rec.len() && chosen.iter().all(|u| alt_rec.contains(u)) {
                        out.push((PayKey::Gold(k), alt));
                    }
                }
            }
        }
        Ok(out)
    }

    /// `pay(pid, e, reqs, ctx)`
    pub fn pay(&mut self, pid: usize, e: i64, reqs: &[Req], ctx: &Value) -> Res<bool> {
        let Some(mut plan) = self.plan_payment(pid, e, reqs, ctx, &[])? else { return Ok(false) };
        if self.agent_every_choice(pid) {
            // joueur humain : choisit chaque rune recyclée, ou un Gold à la place, une question à la fois
            if plan.conv.is_empty() {
                let (mut kf, mut chosen): (i64, Vec<u32>) = (0, vec![]);
                loop {
                    let alts = self.pay_choices(pid, e, reqs, ctx, &plan, kf, &chosen)?;
                    if alts.len() <= 1 {
                        break;
                    }
                    let kind = if alts.iter().any(|a| matches!(a.0, PayKey::Gold(_))) { "pay_gold" } else { "pay_rune" };
                    let opts: Vec<Value> = alts.iter().map(|a| a.0.value()).collect();
                    // plans=alts (dict option -> plan en Python) : ici une liste parallèle aux options, lue par la
                    // table pour nommer chaque façon de payer (train._alabel)
                    let ch = crate::map! {
                        plans = plans_value(&alts),
                        chosen = Value::Tuple(chosen.iter().map(|&u| u.into()).collect())
                    };
                    let o = self.ask(pid, kind, opts.clone(), ch)?;
                    let i = opts.iter().position(|x| *x == o).unwrap();
                    plan = alts[i].1.clone();
                    match &alts[i].0 {
                        PayKey::Rune(u) => chosen.push(*u),
                        PayKey::Gold(k) => kf = *k,
                    }
                }
            }
        } else {
            let alts = self.gold_choices(pid, e, reqs, ctx, &plan)?;
            if alts.len() > 1 {
                let opts: Vec<Value> = alts.iter().map(|a| a.0.into()).collect();
                let pv: Vec<(PayKey, Plan)> = alts.iter().map(|a| (PayKey::Gold(a.0), a.1.clone())).collect();
                let k = self.ask(pid, "pay_gold", opts, crate::map! { plans = plans_value(&pv) })?.int();
                plan = alts.into_iter().find(|a| a.0 == k).unwrap().1;
            }
        }
        // l'énergie réservée se dépense d'abord quand elle peut payer ce coût (le plan l'a comptée)
        let took = plan.took;
        let mut used_r = 0;
        if !self.p[pid].pool_r.is_empty() {
            let mut keep = vec![];
            for ent in self.p[pid].pool_r.clone() {
                if took > used_r && ent.1.call(self, &[pid.into(), ctx.clone()])?.truthy() {
                    let u = ent.0.min(took - used_r);
                    used_r += u;
                    if ent.0 > u {
                        keep.push((ent.0 - u, ent.1.clone()));
                    }
                } else if ent.0 > 0 {
                    keep.push(ent);
                }
            }
            self.p[pid].pool_r = keep;
        }
        // réserve libre après paiement : ce que le plan a laissé, moins l'énergie réservée encore là
        let mut restricted = 0;
        for (n, only) in self.p[pid].pool_r.clone() {
            if only.call(self, &[pid.into(), ctx.clone()])?.truthy() {
                restricted += n;
            }
        }
        self.p[pid].pool_e = plan.pool_e_after - restricted;
        self.p[pid].pool_p = plan.pool_p.positive();
        self.hist.power_spent[pid] += reqs.len() as i64;
        for (o, ad, n, left) in &plan.conv {
            let (o, n) = (*o, *n);
            if ad.get("exhaust").map(|v| v.truthy()).unwrap_or(true) {
                self.set_exhausted(o, true);
            }
            let (cn, conv) = (self.cname(o).to_string(), ad.g("conv").str().to_string());
            self.log(|| format!("  P{pid} uses {cn}: Add ({conv} {n})"));
            if conv == "kill" {
                let vs = ad.g("victims").call(self, &[pid.into(), Value::Obj(o)])?;
                let vs: Vec<O> = vs.list().iter().map(|v| v.obj()).collect();
                let v = if vs.len() > 1 { self.ask_obj(pid, "add_kill", &vs, crate::map! { src = o })?.unwrap() } else { vs[0] };
                self.kill(&[v], Some(pid), true)?;
                for _ in 0..left.g("left_p").opt_int().unwrap_or(0) {
                    self.p[pid].pool_p.add("A", 1);
                }
            }
            let le = left.g("left_e").opt_int().unwrap_or(0);
            if le != 0 {
                let only = ad.g("only");
                if !only.is_none() {
                    self.p[pid].pool_r.push((le, only));
                } else {
                    self.p[pid].pool_e += le;
                }
            }
            self.ability_used(pid, Some(o), "Add", 0, None)?;
        }
        for (o, ad) in plan.adders.clone() {
            // les capacités [Add] se résolvent tout de suite (429.3.a) ; leurs ressources sont déjà dans le plan
            if ad.get("exhaust").map(|v| v.truthy()).unwrap_or(true) {
                self.set_exhausted(o, true);
            }
            let cn = self.cname(o).to_string();
            self.log(|| format!("  P{pid} uses {cn}: Add"));
            let on_use = ad.g("on_use");
            if !on_use.is_none() {
                on_use.call(self, &[pid.into(), Value::Obj(o)])?; // autres coûts de la capacité (Mordekaiser)
            }
            self.ability_used(pid, Some(o), "Add", 0, None)?;
            if ad.g("kill").truthy() {
                self.kill(&[o], Some(pid), true)?;
            }
        }
        for &r in &plan.exhaust {
            self.r_mut(r).exhausted = true;
        }
        for &r in &plan.recycle {
            if !self.r(r).exhausted {
                // une rune prête recyclée est d'abord épuisée : son énergie flotte dans la réserve jusqu'à la fin du tour
                self.r_mut(r).exhausted = true;
                self.p[pid].pool_e += 1;
            }
            self.recycle_rune(pid, r)?;
        }
        let gs: Vec<O> = self.golds(pid).into_iter().take(plan.gold as usize).collect();
        for gobj in gs {
            // Gold : « [Reaction] Kill this, [E]: Add [A] »
            self.set_exhausted(gobj, true);
            self.ability_used(pid, Some(gobj), "Add", 0, None)?;
            self.kill(&[gobj], Some(pid), true)?;
            self.p[pid].gold_spent += 1;
        }
        Ok(true)
    }

    /// `ability_used(pid, o, name, cost_e, item)` : une capacité [Add] utilisée ; émet 'activated'.
    pub fn ability_used(
        &mut self,
        pid: usize,
        o: Option<O>,
        name: &str,
        cost_e: i64,
        item: Option<crate::game::ItemId>,
    ) -> Res<()> {
        if let Some(o) = o {
            if self.is_gear(o) {
                self.hist.gear_abs[pid] += 1;
            }
        }
        self.emit("activated", crate::map! { pid = pid, obj = o, ab = name, cost_e = cost_e, item = item })
    }

    /// `power_reqs(card_domains, n)`
    pub fn power_reqs(&self, card_domains: &[String], n: usize) -> Vec<Req> {
        let doms: Req = if card_domains.is_empty() { any_req() } else { card_domains.iter().cloned().collect() };
        vec![doms; n]
    }
}

/// Agent fictif posé le temps d'un appel (l'agent réel est sorti de la partie pendant `choose`).
#[derive(Clone)]
struct NoAgent;

impl crate::game::Agent for NoAgent {
    fn choose(&mut self, _g: &mut Game, _pid: usize, _kind: &str, _options: &[Value], _ctx: &Map) -> Res<usize> {
        panic!("question posée à un joueur pendant son propre choix")
    }
    fn clone_box(&self) -> Box<dyn crate::game::Agent> {
        Box::new(NoAgent)
    }
}

pub fn reqs_to_value(reqs: &[Req]) -> Value {
    reqs_value(reqs)
}
