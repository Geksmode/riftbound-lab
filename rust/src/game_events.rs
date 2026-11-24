//! `game.py`, suite : déclenchements, évènements, boucle principale, chaîne (résolution, Repeat, contre).
use crate::cards::{registry, Impl};
use crate::game::{Agent, Decision, Desig, Game, GameOver, ItemId, Res, Zone, O};
use crate::lang::{en, tr};
use crate::value::{Map, Value};

/// `_temporary_kill`
fn temporary_kill() -> Value {
    Value::func("_temporary_kill", |g, a| {
        let it = a[0].item();
        let (uid, oid) = (g.it(it).data.g("uid").int() as u32, g.it(it).data.g("oid").int());
        if let Some(o) = g.obj(uid) {
            if g.o(o).oid == oid {
                g.kill(&[o], None, false)?;
            }
        }
        Ok(Value::None)
    })
}

/// `_deploy_kill`
pub(crate) fn deploy_kill() -> Value {
    Value::func("_deploy_kill", |g, a| {
        let it = a[0].item();
        let (uid, oid) = (g.it(it).data.g("uid").int() as u32, g.it(it).data.g("oid").int());
        if let Some(o) = g.obj(uid) {
            if g.o(o).oid == oid {
                g.kill(&[o], None, false)?;
            }
        }
        Ok(Value::None)
    })
}

/// `_FirstChoice` : agent d'essai pour `cost_payable` (prend le premier choix).
#[derive(Clone)]
pub struct FirstChoice;

impl Agent for FirstChoice {
    fn choose(&mut self, _g: &mut Game, _pid: usize, _kind: &str, _options: &[Value], _ctx: &Map) -> Res<usize> {
        Ok(0)
    }
    fn clone_box(&self) -> Box<dyn Agent> {
        Box::new(FirstChoice)
    }
}

impl Game {
    /// `queue_trigger(pid, name, fn, data, src, may, choose, cost)` : capacité déclenchée en attente d'être mise sur
    /// la chaîne. choose(g, item) -> bool fait les choix à la finalisation (False la retire, 402.4) ; cost(g, item)
    /// -> bool paie un coût dans les instructions (383.3.b).
    #[allow(clippy::too_many_arguments)]
    pub fn queue_trigger(
        &mut self,
        pid: usize,
        name: &str,
        f: Value,
        data: Map,
        src: Option<u32>,
        may: bool,
        choose: Value,
        cost: Value,
    ) -> ItemId {
        let mut d = data;
        d.set("_may", may);
        d.set("_choose", choose);
        d.set("_cost", cost);
        let it = self.new_item("trigger", pid, name, f, None, src, d);
        self.trigq.push(it);
        it
    }

    /// `flush_triggers` : met les déclenchements en attente sur la chaîne, joueur du tour d'abord (383.3.d.1).
    pub fn flush_triggers(&mut self) -> Res<()> {
        let q = std::mem::take(&mut self.trigq);
        if self.chain.is_empty() && self.chain_origin.is_none() {
            self.chain_origin = Some("trigger");
        }
        for pid in [self.tp, 1 - self.tp] {
            let mine: Vec<ItemId> = q.iter().copied().filter(|&t| self.it(t).ctrl == pid).collect();
            for it in mine {
                let ch = self.it(it).data.g("_choose");
                let name = self.it(it).name.clone();
                let opts = ch.attr("options");
                if !ch.is_none() && !opts.is_none() && !opts.call(self, &[it.into()])?.truthy() {
                    // aucune cible légale : la capacité est retirée (402.4), sans question « utiliser l'effet ? »
                    self.log(|| {
                        tr(
                            format!("  {name} : aucune cible possible, l'effet ne s'applique pas"),
                            format!("  {name}: no possible target, the effect doesn't apply"),
                        )
                    });
                    continue;
                }
                let c = self.it(it).data.g("_cost");
                let may = self.it(it).data.g("_may").truthy();
                if !c.is_none() && may && !self.cost_payable(it, &c)? {
                    // coût optionnel impayable (Valley of Idols sans énergie) : ni question ni effet
                    let lab = if en() { c.attr("label_en") } else { Value::None };
                    let lab = if lab.truthy() { lab } else { c.attr("label") };
                    let lab = if lab.truthy() { lab.str().to_string() } else { tr("le coût", "the cost") };
                    self.log(|| {
                        tr(
                            format!("  {name} : pas de quoi payer ({lab}), l'effet ne s'applique pas"),
                            format!("  {name}: can't pay ({lab}), the effect doesn't apply"),
                        )
                    });
                    continue;
                }
                if may && !self.ask(pid, "may", vec![true.into(), false.into()], crate::map! { item = it })?.truthy() {
                    continue;
                }
                if !ch.is_none() && !ch.call(self, &[it.into()])?.truthy() {
                    continue;
                }
                if !c.is_none() && !c.call(self, &[it.into()])?.truthy() {
                    self.log(|| {
                        tr(
                            format!("  {name} : coût non payé, l'effet ne s'applique pas"),
                            format!("  {name}: cost not paid, the effect doesn't apply"),
                        )
                    });
                    continue;
                }
                self.chain.push(it);
                self.log(|| format!("  trigger on chain: {name}"));
                self.on_finalize(it)?;
            }
        }
        if let Some(&top) = self.chain.last() {
            self.priority = Some(self.it(top).ctrl);
            self.passes = 0;
        }
        self.need_cleanup = true;
        Ok(())
    }

    /// `cost_payable(it, c)` : le coût optionnel c d'un « may » peut-il être payé ? c.can quand il le dit ; sinon,
    /// pour un joueur humain seulement, essai sur une copie (premier choix à chaque question).
    pub fn cost_payable(&mut self, it: ItemId, c: &Value) -> Res<bool> {
        let can = c.attr("can");
        if !can.is_none() {
            return Ok(can.call(self, &[it.into()])?.truthy());
        }
        let ctrl = self.it(it).ctrl;
        if !(self.agent_every_choice(ctrl) || self.every_choice) {
            return Ok(true);
        }
        let mut g2 = self.clone_game();
        g2.agents = crate::game::Agents(vec![Box::new(FirstChoice), Box::new(FirstChoice)]);
        match c.call(&mut g2, &[it.into()]) {
            Ok(v) => Ok(v.truthy()),
            Err(GameOver) => Ok(true),
        }
    }

    /// `on_finalize(item)` : effets de ciblage (Irelia : « When you choose me »).
    pub fn on_finalize(&mut self, item: ItemId) -> Res<()> {
        let chosen: Vec<u32> = self.it(item).chosen.iter().copied().collect();
        for uid in chosen {
            if let Some(o) = self.obj(uid) {
                let ctrl = self.it(item).ctrl;
                if self.o(o).ctrl != ctrl && self.spec(o).type_ == "Unit" {
                    self.hist.chose_enemy[ctrl] = true;
                }
                self.emit("chosen", crate::map! { obj = o, item = item })?;
            }
        }
        Ok(())
    }

    /// `emit(ev, **info)` : distribue un évènement à chaque permanent, battlefield, légende et effet actif qui
    /// l'écoute. Un permanent qui quitte le plateau reçoit son propre 'leave' (on_leave).
    pub fn emit(&mut self, ev: &str, info: Map) -> Res<()> {
        if self.tg.is_some() {
            crate::train::tg_emit(self, ev, &info); // TGame.emit : combats gagnés, cartes vues
        }
        let infov = Value::Map(info.clone());
        if ev == "leave" {
            let name = info.g("info").map().g("name");
            if let Some(im) = self.impl_n(name.str()) {
                if !im.on_leave.is_none() {
                    im.on_leave.call(self, &[info.g("obj"), infov.clone()])?;
                }
            }
        }
        if ev == "beginning_start" {
            // Temporary (816) : « At the start of this permanent's controller's Beginning Phase, before scoring, kill
            // this. » LeBlanc, Everywhere At Once : « Your Temporary effects at my battlefield don't trigger. »
            let pid = info.g("pid").int() as usize;
            for o in self.board.clone() {
                if self.o(o).ctrl == pid && self.has_kw(o, "Temporary")? {
                    let loc = self.o(o).loc;
                    if crate::value::is_bf(loc)
                        && self.all_units().into_iter().any(|u| {
                            self.cname(u) == "LeBlanc, Everywhere At Once" && self.o(u).ctrl == pid && self.o(u).loc == loc
                        })
                    {
                        continue;
                    }
                    let (uid, oid, r) = (self.o(o).uid, self.o(o).oid, self.repr(o));
                    self.queue_trigger(
                        pid,
                        &format!("Temporary {r}"),
                        temporary_kill(),
                        crate::map! { uid = uid, oid = oid },
                        Some(uid),
                        false,
                        Value::None,
                        Value::None,
                    );
                }
            }
        }
        let evv = Value::from(ev);
        for o in self.board.clone() {
            let im = self.impl_o(o);
            if let Some(im) = im {
                if !im.on_event.is_none() && self.on_board(o) {
                    im.on_event.call(self, &[o.into(), evv.clone(), infov.clone()])?;
                }
            }
            for gu in self.o(o).attached.clone() {
                // texte d'effet des équipements attachés (718.3)
                let go = self.obj(gu);
                let ig = go.and_then(|g| self.impl_o(g));
                if let Some(ig) = ig {
                    if !ig.effect_event.is_none() {
                        ig.effect_event.call(self, &[go.unwrap().into(), o.into(), evv.clone(), infov.clone()])?;
                    }
                }
                if let Some(go) = go {
                    let cn = self.o(go).copy_name.clone();
                    if cn.as_deref().is_some_and(|n| !n.is_empty()) && self.cname(go) == "Svellsongur" && self.on_board(o) {
                        if let Some(ic) = self.impl_n(&cn.unwrap()) {
                            if !ic.on_event.is_none() {
                                ic.on_event.call(self, &[o.into(), evv.clone(), infov.clone()])?;
                            }
                        }
                    }
                }
            }
        }
        for i in 0..self.bfs.len() {
            let name = self.bfs[i].name.clone();
            if let Some(im) = self.impl_n(&name) {
                if !im.bf_event.is_none() {
                    im.bf_event.call(self, &[Value::Int(i as i64), evv.clone(), infov.clone()])?;
                }
            }
        }
        for pid in 0..2 {
            let ln = self.p[pid].legend_name.clone();
            if let Some(im) = self.impl_n(&ln) {
                if !im.legend_event.is_none() {
                    im.legend_event.call(self, &[pid.into(), evv.clone(), infov.clone()])?;
                }
            }
        }
        for e in self.effects.clone() {
            if self.has_effect(e.eid) && e.d.g("on").opt_str() == Some(ev) {
                e.d.g("fn").call(self, &[Value::Int(e.eid as i64), infov.clone()])?;
            }
        }
        if self.zone_listen {
            if let Some(names) = registry().hook_names("zone_event") {
                let mut found = false;
                for pid in 0..2 {
                    let cs: Vec<O> = self.p[pid].hand.iter().chain(self.p[pid].trash.iter()).copied().collect();
                    for c in cs {
                        if names.contains(self.cname(c)) {
                            found = true;
                            let im = self.impl_o(c).unwrap();
                            im.hook("zone_event").call(self, &[c.into(), evv.clone(), infov.clone()])?;
                        }
                    }
                }
                if !found && self.stage != "setup" {
                    let any_card = self.p.iter().any(|pl| {
                        pl.deck.iter().chain(pl.champ.iter()).chain(pl.banish.iter()).any(|&c| names.contains(self.cname(c)))
                    }) || self.board.iter().any(|&o| names.contains(self.cname(o)))
                        || self
                            .chain
                            .iter()
                            .copied()
                            .chain(self.resolving)
                            .any(|it| self.it(it).card.is_some_and(|c| names.contains(self.cname(c))))
                        || self.bfs.iter().any(|b| b.facedowns.iter().any(|&c| names.contains(self.cname(c))));
                    if !any_card {
                        self.zone_listen = false; // aucune carte de la partie ne peut écouter depuis une main ou une défausse
                    }
                }
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------ boucle principale
    /// `advance(lazy)` : fait avancer la partie jusqu'à une décision de haut niveau (None : partie finie). lazy :
    /// la décision n'a pas d'options (l'IA ne génère que les coups dont elle a besoin).
    pub fn advance(&mut self, lazy: bool) -> Option<Decision> {
        self.advance_inner(lazy).unwrap_or_default()
    }

    fn advance_inner(&mut self, lazy: bool) -> Res<Option<Decision>> {
        loop {
            if self.winner.is_some() {
                return Ok(None);
            }
            if !self.exh_pending.is_empty() {
                let batch = std::mem::take(&mut self.exh_pending);
                let batch: Vec<O> = batch.into_iter().filter(|&o| self.on_board(o)).collect();
                if !batch.is_empty() {
                    self.emit("exhausted", crate::map! { objs = batch })?;
                }
                continue;
            }
            if !self.trigq.is_empty() {
                self.flush_triggers()?;
                continue;
            }
            if self.need_cleanup {
                self.cleanup()?;
                continue;
            }
            if !self.chain.is_empty() {
                if self.passes >= 2 {
                    self.resolve_top()?;
                    continue;
                }
                let p = self.priority.unwrap();
                let options = if lazy { None } else { Some(self.options_priority(p)?) };
                return Ok(Some(Decision { kind: "priority", player: p, options }));
            }
            if self.chain_origin.is_some() {
                self.chain_closed();
                continue;
            }
            if let Some(sd) = &self.sd {
                if sd.stage == "open" {
                    if sd.passes >= 2 {
                        self.end_showdown()?;
                        continue;
                    }
                    let f = sd.focus;
                    let options = if lazy { None } else { Some(self.options_focus(f)?) };
                    return Ok(Some(Decision { kind: "focus", player: f, options }));
                }
                self.combat_step()?;
                continue;
            }
            if self.stage == "main" {
                let tp = self.tp;
                let options = if lazy { None } else { Some(self.options_main(tp)?) };
                return Ok(Some(Decision { kind: "main", player: tp, options }));
            }
            self.turn_step()?;
        }
    }

    /// `apply(action)` : applique l'action choisie (GameOver est absorbé : `winner` est posé).
    pub fn apply(&mut self, action: &Value) {
        let _ = self.apply_inner(action);
    }

    fn apply_inner(&mut self, a: &Value) -> Res<()> {
        let kind = a.list()[0].str().to_string();
        if self.shown_hand.is_some() && self.chain.is_empty() {
            self.shown_hand = None; // l'effet qui a révélé la main est résolu et on rejoue : fin
        }
        if !self.chain.is_empty() {
            // décision de priorité
            if kind == "pass" {
                self.passes += 1;
                self.priority = Some(1 - self.priority.unwrap());
                return Ok(());
            }
            return self.do_action(a);
        }
        if self.sd.as_ref().is_some_and(|sd| sd.stage == "open") {
            if kind == "pass" {
                let sd = self.sd.as_mut().unwrap();
                sd.passes += 1;
                sd.focus = 1 - sd.focus;
                return Ok(());
            }
            self.sd.as_mut().unwrap().passes = 0;
            return self.do_action(a);
        }
        // Main Phase, Neutral Open
        if kind == "end" {
            self.stage = "end_step";
            self.need_cleanup = true;
            return Ok(());
        }
        self.do_action(a)
    }

    /// `chain_closed` (340.2, 346) : la chaîne se vide pendant un showdown : le Focus passe, sauf si la chaîne a été
    /// ouverte par une capacité déclenchée ou [Add].
    pub fn chain_closed(&mut self) {
        let origin = self.chain_origin.take();
        self.priority = None;
        if let Some(sd) = self.sd.as_mut() {
            if sd.stage == "open" {
                if origin == Some("play") {
                    sd.focus = 1 - sd.focus;
                }
                sd.passes = 0;
            }
        }
        self.need_cleanup = true;
    }

    /// `resolve_top` : résout le dernier item de la chaîne.
    pub fn resolve_top(&mut self) -> Res<()> {
        let it = self.chain.pop().unwrap();
        self.passes = 0;
        let name = self.it(it).name.clone();
        self.log(|| format!("resolve {name}"));
        let prev = self.resolving.replace(it);
        let r = (|| {
            if self.it(it).kind == "spell" {
                let card = self.it(it).card.unwrap();
                let im = self.impl_o(card).expect("sort sans Impl");
                im.resolve.call(self, &[it.into()])?;
                self.repeat_resolution(it, im)?;
                if self.o(card).zone == Some(Zone::Chain) {
                    let dest = self.it(it).data.g("after");
                    let dest = dest.opt_str().unwrap_or("trash").to_string();
                    let ctrl = self.it(it).ctrl;
                    self.card_leaves_chain(card, &dest, Some(ctrl))?;
                }
            } else {
                let f = self.it(it).fn_.clone();
                f.call(self, &[it.into()])?;
            }
            Ok(())
        })();
        self.resolving = prev;
        r?;
        let kind = self.it(it).kind;
        if kind == "spell" {
            let pb = self.it(it).data.g("played_by").opt_int().map(|x| x as usize).unwrap_or(self.it(it).ctrl);
            let card = self.it(it).card.unwrap();
            self.played_event(pb, card, Some(it))?;
        } else if kind == "ability" && !self.it(it).data.g("_ab").is_none() {
            // « When you use/play an activated ability » se déclenche à la résolution (377.2.a)
            let d = self.it(it).data.clone();
            let src = self.it(it).src.and_then(|u| self.obj(u));
            let obj: Value = match src {
                Some(o) => o.into(),
                None => d.g("_src_obj"),
            };
            let ctrl = self.it(it).ctrl;
            let cost_e = d.g("_cost_e").opt_int().unwrap_or(0);
            self.emit("activated", crate::map! { pid = ctrl, obj = obj, ab = d.g("_ab"), cost_e = cost_e, item = it })?;
        }
        self.need_cleanup = true;
        if let Some(&top) = self.chain.last() {
            self.priority = Some(self.it(top).ctrl);
        }
        Ok(())
    }

    /// `repeat_resolution(it, im)` : [Repeat] générique (820) : chaque coût de Repeat payé rejoue l'effet du sort avec ce
    /// choix (nouvelles cibles vérifiées avec les mêmes prédicats).
    pub fn repeat_resolution(&mut self, it: ItemId, im: &'static Impl) -> Res<()> {
        let reps = self.it(it).data.g("reps");
        if !reps.truthy() {
            return Ok(());
        }
        let preds: Vec<Value> = im.preds.list().to_vec();
        for ch in reps.list() {
            let card = self.it(it).card.unwrap();
            if self.o(card).zone != Some(Zone::Chain) {
                break;
            }
            let ch = ch.map().clone();
            let mut data = self.it(it).data.merged(&ch);
            data.set("rep", false);
            data.set("reps", Value::None);
            data.set("tg2", Value::Tuple(vec![]));
            let (ctrl, name, src) = (self.it(it).ctrl, self.it(it).name.clone(), self.it(it).src);
            let it2 = self.new_item("spell", ctrl, &name, Value::None, Some(card), src, data);
            let oids = ch.g("_oids");
            for (i, uid) in ch.g("tg").list().iter().enumerate() {
                let o = match uid {
                    Value::Int(u) => self.obj(*u as u32),
                    _ => None,
                };
                let pred = if preds.is_empty() { Value::None } else { preds[i.min(preds.len() - 1)].clone() };
                let want = |g: &Game, o: O| -> i64 {
                    if oids.is_none() {
                        return g.o(o).oid;
                    }
                    oids.map().get(&uid.int().to_string()).map(|v| v.int()).unwrap_or(g.o(o).oid)
                };
                match o {
                    Some(o) if self.o(o).oid == want(self, o) => {
                        let (u, w) = (self.o(o).uid, want(self, o));
                        self.it_mut(it2).targets.push((Some(u), w, pred));
                    }
                    _ => {
                        let u = match uid {
                            Value::Int(u) => Some(*u as u32),
                            _ => None,
                        };
                        self.it_mut(it2).targets.push((u, -1, pred));
                    }
                }
            }
            let chosen = self.it(it).chosen.clone();
            self.it_mut(it2).chosen = chosen;
            self.log(|| format!("  {name} repeats"));
            im.resolve.call(self, &[it2.into()])?;
        }
        Ok(())
    }

    /// `card_leaves_chain(c, dest, by)` : un sort quitte la chaîne (défausse, bannissement, ou 'recycle' sous le deck).
    pub fn card_leaves_chain(&mut self, c: O, dest: &str, by: Option<usize>) -> Res<()> {
        let dest = if dest == "recycle" { "recycle" } else { self.trash_dest(c, dest, Some("chain")) };
        let owner = self.o(c).owner;
        self.o_mut(c).zone = None;
        match dest {
            "banish" => {
                self.o_mut(c).zone = Some(Zone::Banish);
                self.p[owner].banish.push(c);
            }
            "recycle" => {
                self.o_mut(c).zone = Some(Zone::Deck);
                self.p[owner].deck.push(c);
            }
            _ => {
                self.o_mut(c).zone = Some(Zone::Trash);
                self.p[owner].trash.push(c);
            }
        }
        self.o_mut(c).oid += 1;
        if dest == "banish" {
            self.emit("banish", crate::map! { pid = by.unwrap_or(owner), card = c })?;
        }
        Ok(())
    }

    /// `counter(it)` (425) : un item contré ne fait rien et quitte la chaîne.
    pub fn counter(&mut self, it: ItemId) -> Res<bool> {
        if !self.chain.contains(&it) || !self.counterable(it)? {
            return Ok(false);
        }
        let i = self.chain.iter().position(|&x| x == it).unwrap();
        self.chain.remove(i);
        let name = self.it(it).name.clone();
        self.log(|| format!("  {name} is countered"));
        self.stats.add(&format!("countered_{name}"), 1);
        if self.it(it).kind == "spell" {
            let d = self.it(it).data.clone();
            let after = d.g("after");
            let after = after.opt_str().unwrap_or("trash");
            let dest = if d.g("flow").truthy() || after == "recycle" { after } else { "trash" };
            let (card, ctrl) = (self.it(it).card.unwrap(), self.it(it).ctrl);
            self.card_leaves_chain(card, dest, Some(ctrl))?;
        }
        self.need_cleanup = true;
        Ok(true)
    }

    /// `counterable(it)` : « This can't be countered » et no_counter des autres objets (Mel).
    pub fn counterable(&mut self, it: ItemId) -> Res<bool> {
        if self.it(it).uncounterable {
            return Ok(false);
        }
        for (src, im) in self.providers("no_counter") {
            if im.hook("no_counter").call(self, &[src, it.into()])?.truthy() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `played_event(pid, card, item)` : la pose d'une carte est terminée (419.4.a). Les jetons émettent 'played'
    /// avec n=0 et token=True et ne vont pas dans g.played.
    pub fn played_event(&mut self, pid: usize, card: O, item: Option<ItemId>) -> Res<()> {
        if self.o(card).zone == Some(Zone::Board) && self.at_bf(card) {
            self.hist.to_bf[pid] += 1;
        }
        if self.o(card).token {
            self.hist.tokens[pid] += 1;
            return self.emit("played", crate::map! { pid = pid, card = card, item = item, n = 0, token = true });
        }
        let cn = self.cname(card).to_string();
        self.played[pid].push(cn);
        let n = self.played[pid].len();
        self.emit("played", crate::map! { pid = pid, card = card, item = item, n = n, token = false })
    }

    /// Valeur de `sd` donnée aux écouteurs ('showdown_start', 'combat_start') : un instantané lisible.
    pub fn sd_value(&self) -> Value {
        match &self.sd {
            Some(sd) => crate::map! {
                bf = sd.bf, combat = sd.combat, attacker = sd.attacker, defender = sd.defender, focus = sd.focus,
            }
            .into(),
            None => Value::None,
        }
    }

    pub fn desig_is(&self, o: O, d: Desig) -> bool {
        self.o(o).desig == Some(d)
    }
}
