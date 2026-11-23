//! `game.py`, suite : dégâts, mort et remplacements, battlefields remplacés, rappel, déplacement, stun, buff,
//! empower, préparation.
use crate::cards::registry;
use crate::game::{Desig, Game, ItemId, Res, O};
use crate::game_costs::reqs_of;
use crate::lang::tr;
use crate::value::{is_bf, Loc, Map, OptV, Value};

impl Game {
    // ------------------------------------------------------------------ mort, dégâts, déplacement
    /// `deal(target, amount, src_kind, src_pid, item)` : dégâts d'un sort ou d'une capacité (417) : dégâts bonus
    /// (712-715), puis remplacements sur la cible. Renvoie les dégâts infligés.
    pub fn deal(
        &mut self,
        target: Option<O>,
        amount: i64,
        src_kind: &str,
        src_pid: Option<usize>,
        item: Option<ItemId>,
    ) -> Res<i64> {
        let Some(target) = target else { return Ok(0) };
        let mut amount = amount;
        if !self.on_board(target) || amount <= 0 {
            return Ok(0);
        }
        let item = item.or(self.resolving);
        let src_pid = src_pid.or(item.map(|i| self.it(i).ctrl));
        let ctx = crate::map! { kind = src_kind, by = src_pid, item = item, target = target };
        if src_kind == "spell" || src_kind == "ability" {
            if let Some(Loc::Bf(i)) = self.o(target).loc {
                if self.bfs[i].name == "Void Gate" {
                    amount += 1;
                }
            }
            amount += self.bonus_damage(&ctx)?;
        }
        amount = self.damage_replace(target, amount, &ctx)?;
        if amount <= 0 {
            return Ok(0);
        }
        self.mark_damage(target, amount, src_pid, src_kind);
        if self.logging {
            let (r, m) = (self.repr(target), self.might(target)?);
            self.log(|| format!("  {r} takes {amount} ({m} might)"));
        }
        self.emit("damaged", crate::map! { obj = target, amount = amount, kind = src_kind, by = src_pid })?;
        Ok(amount)
    }

    /// `mark_damage(target, amount, pid, kind)`
    pub fn mark_damage(&mut self, target: O, amount: i64, pid: Option<usize>, kind: &str) {
        let ob = self.o_mut(target);
        ob.damage += amount;
        if let Some(pid) = pid {
            if let Some(e) = ob.dmg_by.iter_mut().find(|(p, _)| *p == pid) {
                e.1 += amount;
            } else {
                ob.dmg_by.push((pid, amount));
            }
            ob.last_dmg = Some((pid, kind.into()));
        }
        self.need_cleanup = true;
    }

    /// `bonus_damage(ctx)` : dégâts bonus d'un sort ou d'une capacité (additionnés, 714).
    pub fn bonus_damage(&mut self, ctx: &Map) -> Res<i64> {
        let mut n = 0;
        for (src, im) in self.providers("dmg_bonus") {
            n += im.hook("dmg_bonus").call(self, &[src, ctx.clone().into()])?.opt_int().unwrap_or(0);
        }
        for (go, u, im) in self.fx_providers("fx_dmg_bonus", None) {
            n += im.hook("fx_dmg_bonus").call(self, &[go.into(), u.into(), ctx.clone().into()])?.opt_int().unwrap_or(0);
        }
        for e in self.effects.clone() {
            if e.d.g("kind").opt_str() == Some("dmg_bonus") {
                n += e.d.g("fn").call(self, &[Value::Int(e.eid as i64), ctx.clone().into()])?.opt_int().unwrap_or(0);
            }
        }
        Ok(n)
    }

    /// `damage_replace(target, amount, ctx)` : remplacements des dégâts (367, 437) : prévention totale, « la
    /// prochaine fois », valeur de Prevent (Ki Barrier), puis doublement.
    pub fn damage_replace(&mut self, target: O, amount: i64, ctx: &Map) -> Res<i64> {
        let mut amount = amount;
        if amount <= 0 {
            return Ok(0);
        }
        if let Some(im) = self.impl_o(target) {
            let h = im.hook("dmg_prevent");
            if !h.is_none() && h.call(self, &[target.into(), ctx.clone().into()])?.truthy() {
                return Ok(0);
            }
        }
        for (src, ia) in self.providers("aura_dmg_prevent") {
            if ia.hook("aura_dmg_prevent").call(self, &[src, target.into(), ctx.clone().into()])?.truthy() {
                return Ok(0);
            }
        }
        let (uid, oid) = (self.o(target).uid, self.o(target).oid);
        if !self.effects.is_empty() {
            for e in self.effects.clone() {
                if e.d.g("kind").opt_str() == Some("prevent_damage")
                    && e.d.g("fn").call(self, &[Value::Int(e.eid as i64), target.into(), ctx.clone().into()])?.truthy()
                {
                    return Ok(0);
                }
            }
            for e in self.effects.clone() {
                if e.d.g("kind").opt_str() == Some("prevent_next")
                    && e.d.g("uid").int() == uid as i64
                    && e.d.g("oid").int() == oid
                {
                    self.remove_effect(e.eid); // « the next time it would be dealt damage this turn »
                    let r = self.repr(target);
                    self.log(|| format!("  damage to {r} is prevented"));
                    return Ok(0);
                }
            }
        }
        let pv = self.o(target).prevent;
        if pv != 0 {
            let p = pv.min(amount);
            self.o_mut(target).prevent -= p;
            amount -= p;
        }
        if amount > 0 && !self.effects.is_empty() {
            for e in &self.effects {
                if e.d.g("kind").opt_str() == Some("double_damage")
                    && e.d.g("uid").int() == uid as i64
                    && e.d.g("oid").int() == oid
                {
                    amount *= 2;
                }
            }
        }
        Ok(amount)
    }

    /// `lethal(o)` : dégâts mortels (142.4), ou n'importe quels dégâts d'un joueur dont les dégâts suffisent
    /// toujours (lethal_any : Elder Dragon).
    pub fn lethal(&mut self, o: O) -> Res<bool> {
        if self.o(o).damage <= 0 {
            return Ok(false);
        }
        if self.o(o).damage >= self.might(o)? {
            return Ok(true);
        }
        if !self.o(o).dmg_by.is_empty() {
            for (src, im) in self.providers("lethal_any") {
                let sc = self.o(src.obj()).ctrl;
                if self.o(o).dmg_by(sc) > 0 && im.hook("lethal_any").call(self, &[src, o.into()])?.truthy() {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// `kill(objs, by_pid, cost)` : tuer des permanents (428), avec les remplacements (370-373) et les
    /// déclenchements de mort. Renvoie les objets vraiment tués.
    pub fn kill(&mut self, objs: &[O], by_pid: Option<usize>, cost: bool) -> Res<Vec<O>> {
        let objs: Vec<O> = objs.iter().copied().filter(|&o| self.on_board(o)).collect();
        if objs.is_empty() {
            return Ok(vec![]);
        }
        let it = self.resolving;
        let by_pid = by_pid.or(it.map(|i| self.it(i).ctrl));
        let saved = self.death_replacements(&objs)?;
        let objs: Vec<O> = objs.into_iter().filter(|o| !saved.contains(o) && self.on_board(*o)).collect();
        if objs.is_empty() {
            return Ok(vec![]);
        }
        // informations rétrospectives avant de quitter le plateau (359.3.e.13, 808.1.d.3)
        let mut infos = vec![];
        self.stats.add("kill_batch", 1); // un seul évènement de mort (« one or more die »)
        for &o in &objs {
            let mut inf = self.lookback(o)?;
            inf.set("batch", self.stats.get("kill_batch"));
            let (mut by, mut kind): (Value, Value) = (by_pid.into(), it.map(|i| Value::from(self.it(i).kind)).into());
            if let Some((p, k)) = self.o(o).last_dmg.clone() {
                if self.lethal(o)? && !cost {
                    by = p.into(); // meurt de dégâts mortels : attribué à leur source
                    kind = k;
                }
            }
            inf.set("by", by);
            inf.set("by_kind", kind);
            inf.set("stage", self.stage);
            inf.set("phase", self.phase);
            inf.set("tp", self.tp);
            inf.set("turn", self.turn_no);
            inf.set("as_cost", cost);
            infos.push(inf);
        }
        for &o in &objs {
            let r = self.repr(o);
            self.log(|| format!("  {r} dies"));
            if let Some(i) = self.board.iter().position(|&x| x == o) {
                self.board.remove(i);
            }
            self.leave_board_cleanup(o);
        }
        for &o in &objs {
            self.to_zone(o, "trash", false, None)?;
        }
        // déclenchements de mort (Deathknell, 808). Karthus : « Your Deathknell effects trigger an additional time »,
        // compté pour les Karthus encore sur le plateau après les morts (383.2.c.2).
        for inf in infos {
            self.hist.died.push(inf.clone());
            let name = inf.g("name").str().to_string();
            let mut dks: Vec<(String, Value, Value)> = vec![];
            if let Some(im) = self.impl_n(&name) {
                if im.deathknell.truthy() {
                    dks.push((name.clone(), im.deathknell.clone(), im.dk_choose.clone()));
                }
            }
            for n in inf.g("copied").list() {
                // texte copié (Svellsongur) : son Deathknell aussi
                if let Some(ic) = self.impl_n(n.str()) {
                    if ic.deathknell.truthy() {
                        dks.push((format!("{} (copy)", n.str()), ic.deathknell.clone(), ic.dk_choose.clone()));
                    }
                }
            }
            for n in inf.g("att_names").list() {
                // Deathknell dans le texte d'effet d'un équipement attaché
                if let Some(ig) = self.impl_n(n.str()) {
                    if !ig.fx_deathknell.is_none() {
                        dks.push((format!("{name} ({})", n.str()), ig.fx_deathknell.clone(), Value::None));
                    }
                }
            }
            let ctrl = inf.g("ctrl").int() as usize;
            if !dks.is_empty() {
                let mut k = 1 + self
                    .units(Some(ctrl), None)
                    .into_iter()
                    .filter(|&u| self.cname(u) == "Karthus, Eternal")
                    .count() as i64;
                for (src, im) in self.providers("dk_extra") {
                    // Detonation Amplifier : « Friendly [Deathknell] effects here trigger an additional time. »
                    k += im.hook("dk_extra").call(self, &[src, inf.clone().into()])?.opt_int().unwrap_or(0);
                }
                for (n, f, choose) in &dks {
                    for _ in 0..k {
                        self.queue_trigger(
                            ctrl,
                            &format!("Deathknell {n}"),
                            f.clone(),
                            crate::map! { info = inf.clone() },
                            None,
                            false,
                            choose.clone(),
                            Value::None,
                        );
                    }
                }
            }
            self.emit("die", crate::map! { info = inf.clone() })?;
            let o = inf.g("obj").obj();
            let dest: Value = self.o(o).zone.map(|z| z.name()).into();
            self.emit("leave", crate::map! { obj = o, info = inf, dest = dest, killed = true })?;
        }
        Ok(objs)
    }

    /// `death_replacements(objs)` : applique les « would die ... instead » (370-373). Renvoie les objets sauvés.
    pub fn death_replacements(&mut self, objs: &[O]) -> Res<Vec<O>> {
        let reg = registry();
        if reg.hook_names("death_rep").is_none()
            && reg.hook_names("fx_death_rep").is_none()
            && !self.effects.iter().any(|e| e.d.g("kind").opt_str() == Some("death_rep"))
        {
            return Ok(vec![]);
        }
        let mut saved = vec![];
        let mut used: Vec<Value> = vec![];
        let prev = std::mem::replace(&mut self.dying, objs.to_vec());
        let r = self.replace_deaths(objs, &mut saved, &mut used);
        self.dying = prev;
        r?;
        Ok(saved)
    }

    fn replace_deaths(&mut self, objs: &[O], saved: &mut Vec<O>, used: &mut Vec<Value>) -> Res<()> {
        for pid in [self.tp, 1 - self.tp] {
            // 373.1 : ordre du tour
            let mut mine: Vec<O> = objs.iter().copied().filter(|&o| self.o(o).ctrl == pid).collect();
            while !mine.is_empty() {
                let mut pairs: Vec<(O, Map)> = vec![];
                for &u in &mine {
                    for r in self.death_reps(u, objs)? {
                        if used.contains(&r.g("key")) {
                            continue;
                        }
                        pairs.push((u, r));
                    }
                }
                if pairs.is_empty() {
                    break;
                }
                let mand: Vec<(O, Map)> = pairs.iter().filter(|p| !p.1.g("may").truthy()).cloned().collect();
                let choice: (O, Map);
                if !mand.is_empty() && mand.iter().all(|p| p.1.g("name").opt_str() == Some("Zhonya's Hourglass")) {
                    // Zhonya's Hourglass : le joueur choisit l'unité qu'il sauve
                    let mut us: Vec<O> = vec![];
                    for (u, _) in &mand {
                        if !us.contains(u) {
                            us.push(*u);
                        }
                    }
                    let u = if us.len() > 1 { self.ask_obj(pid, "zhonya_save", &us, Map::new())?.unwrap() } else { us[0] };
                    choice = mand.iter().find(|p| p.0 == u).unwrap().clone();
                } else {
                    let src = if mand.is_empty() { &pairs } else { &mand };
                    let mut opts: Vec<Value> = src
                        .iter()
                        .map(|(u, r)| {
                            let (rn, cn) = (r.g("name").str().to_string(), self.cname(*u).to_string());
                            OptV {
                                label: tr(format!("{rn} : sauver {cn}"), format!("{rn}: save {cn}")),
                                value: Value::Tuple(vec![(*u).into(), r.clone().into()]),
                                obj: (*u).into(),
                            }
                            .into()
                        })
                        .collect();
                    if mand.is_empty() {
                        opts.push(
                            OptV { label: tr("ne rien remplacer", "replace nothing"), value: Value::None, obj: Value::None }
                                .into(),
                        );
                    }
                    let pick = self.ask(pid, "death_replace", opts, Map::new())?;
                    if pick.is_none() || pick.opt().value.is_none() {
                        break;
                    }
                    let v = pick.opt().value.list().to_vec();
                    choice = (v[0].obj(), v[1].map().clone());
                }
                let (u, r) = choice;
                let cost = r.g("cost");
                if !cost.is_none() && !cost.call(self, &[])?.truthy() {
                    let key = r.g("key");
                    used.push(if key.truthy() { key } else { Value::Str(format!("id:{}", self.fresh_id()).into()) });
                    continue;
                }
                if !r.g("key").is_none() {
                    used.push(r.g("key"));
                }
                let mut targets = vec![u];
                if r.g("multi").truthy() {
                    for &x in &mine.clone() {
                        if x != u && self.death_reps(x, objs)?.iter().any(|rr| rr.g("key") == r.g("key")) {
                            targets.push(x);
                        }
                    }
                }
                for x in targets {
                    let (rn, xr) = (r.g("name").str().to_string(), self.repr(x));
                    self.log(|| format!("  {rn} replaces the death of {xr}"));
                    self.stats.add(&format!("death_rep_{rn}"), 1);
                    r.g("apply").call(self, &[x.into()])?;
                    saved.push(x);
                    if let Some(i) = mine.iter().position(|&m| m == x) {
                        mine.remove(i);
                    }
                }
            }
        }
        Ok(())
    }

    /// Identifiant neuf (remplace `id(objet)` de Python quand il sert de clé unique).
    pub fn fresh_id(&mut self) -> u64 {
        self.next_eid += 1;
        self.next_eid
    }

    fn death_reps(&mut self, u: O, _objs: &[O]) -> Res<Vec<Map>> {
        let mut out = vec![];
        for (src, im) in self.providers("death_rep") {
            let r = im.hook("death_rep").call(self, &[src.clone(), u.into()])?;
            if !r.is_none() {
                let mut r = r.map().clone();
                if !r.contains("key") {
                    let sid: Value = match &src {
                        Value::Obj(o) => self.o(*o).uid.into(),
                        v => v.clone(),
                    };
                    r.set("key", Value::Tuple(vec!["src".into(), sid, r.g("name")]));
                }
                out.push(r);
            }
        }
        if !self.o(u).attached.is_empty() {
            for (go, _, im) in self.fx_providers("fx_death_rep", Some(u)) {
                let r = im.hook("fx_death_rep").call(self, &[go.into(), u.into()])?;
                if !r.is_none() {
                    let mut r = r.map().clone();
                    if !r.contains("key") {
                        r.set("key", Value::Tuple(vec!["fx".into(), self.o(go).uid.into(), r.g("name")]));
                    }
                    out.push(r);
                }
            }
        }
        for e in self.effects.clone() {
            if e.d.g("kind").opt_str() == Some("death_rep") {
                let r = e.d.g("fn").call(self, &[Value::Int(e.eid as i64), u.into()])?;
                if !r.is_none() {
                    let mut r = r.map().clone();
                    if !r.contains("key") {
                        r.set("key", Value::Tuple(vec!["eff".into(), Value::Int(e.eid as i64)]));
                    }
                    out.push(r);
                }
            }
        }
        Ok(out)
    }

    /// `save_unit(u, heal, exhaust)` : « Heal it, exhaust it, and recall it ».
    pub fn save_unit(&mut self, u: O, heal: bool, exhaust: bool) {
        if heal {
            self.o_mut(u).damage = 0;
            self.o_mut(u).dmg_by.clear();
        }
        if exhaust {
            self.set_exhausted(u, true);
        }
        self.recall(u);
    }

    /// `replace_battlefield(b, name)` : remplace un battlefield par un jeton (438) ; émet 'bf_replaced'.
    pub fn replace_battlefield(&mut self, b: usize, name: &str) -> Res<()> {
        let old = self.bfs[b].name.clone();
        if self.bfs[b].replaced.is_none() {
            self.bfs[b].replaced = Some(old.clone());
        }
        self.bfs[b].name = name.to_string();
        self.need_cleanup = true;
        self.log(|| format!("  {old} is replaced with {name}"));
        self.emit("bf_replaced", crate::map! { bf = b, old = old, new = name })
    }

    /// `swap_back(b)` (438.7.b)
    pub fn swap_back(&mut self, b: usize) -> Res<bool> {
        let Some(rep) = self.bfs[b].replaced.take() else { return Ok(false) };
        let old = std::mem::replace(&mut self.bfs[b].name, rep);
        self.need_cleanup = true;
        let new = self.bfs[b].name.clone();
        self.log(|| format!("  {old} is swapped back to {new}"));
        self.emit("bf_replaced", crate::map! { bf = b, old = old, new = new })?;
        Ok(true)
    }

    /// `recall(o)` (455) : vers la base, ce n'est pas un déplacement.
    pub fn recall(&mut self, o: O) {
        self.o_mut(o).loc = Some(Loc::Base);
        self.o_mut(o).desig = None;
        for gu in self.o(o).attached.clone() {
            if let Some(go) = self.obj(gu) {
                self.o_mut(go).loc = Some(Loc::Base);
            }
        }
        self.need_cleanup = true;
    }

    /// `can_move(o, dest, by_pid, standard)` : restrictions de déplacement (359.3.e.6).
    pub fn can_move(&mut self, o: O, dest: Loc, by_pid: Option<usize>, standard: bool) -> Res<bool> {
        let args = |x: Vec<Value>| -> Vec<Value> {
            let mut v = x;
            v.extend([dest.into(), by_pid.into(), standard.into()]);
            v
        };
        if let Some(im) = self.impl_o(o) {
            let h = im.hook("cant_move");
            if !h.is_none() && h.call(self, &args(vec![o.into()]))?.truthy() {
                return Ok(false);
            }
        }
        if !self.o(o).attached.is_empty() {
            for (go, _, ig) in self.fx_providers("fx_cant_move", Some(o)) {
                if ig.hook("fx_cant_move").call(self, &args(vec![go.into(), o.into()]))?.truthy() {
                    return Ok(false);
                }
            }
        }
        for (src, ia) in self.providers("aura_cant_move") {
            if ia.hook("aura_cant_move").call(self, &args(vec![src, o.into()]))?.truthy() {
                return Ok(false);
            }
        }
        for e in self.effects.clone() {
            if e.d.g("kind").opt_str() == Some("cant_move")
                && e.d.g("fn").call(self, &args(vec![Value::Int(e.eid as i64), o.into()]))?.truthy()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `move_tax(pid, units, dest)` : coût de déplacer plusieurs unités à la fois (Mageseeker Investigator).
    pub fn move_tax(&mut self, pid: usize, units: &[O], dest: Loc) -> Res<Vec<crate::game_costs::Req>> {
        let mut reqs = vec![];
        for (src, im) in self.providers("move_tax") {
            let r = im.hook("move_tax").call(self, &[src, pid.into(), units.to_vec().into(), dest.into()])?;
            reqs.extend(reqs_of(&r));
        }
        Ok(reqs)
    }

    /// `move_cost(pid, units, dest)` : (e, reqs) à payer pour déplacer ces unités (Gravity Field).
    pub fn move_cost(&mut self, pid: usize, units: &[O], dest: Loc) -> Res<(i64, Vec<crate::game_costs::Req>)> {
        let (mut e, mut reqs) = (0, vec![]);
        for (src, im) in self.providers("move_cost") {
            let c = im.hook("move_cost").call(self, &[src, pid.into(), units.to_vec().into(), dest.into()])?;
            if c.truthy() {
                let c = c.list();
                e += c[0].int();
                reqs.extend(reqs_of(&c[1]));
            }
        }
        Ok((e, reqs))
    }

    /// `move(objs, dest, by_pid, standard)` (445). Renvoie [(objet, ancien emplacement)].
    pub fn move_(&mut self, objs: &[O], dest: Loc, by_pid: Option<usize>, standard: bool) -> Res<Vec<(O, Option<Loc>)>> {
        let mut ok = vec![];
        for &o in objs {
            if self.on_board(o) && self.o(o).loc != Some(dest) && self.can_move(o, dest, by_pid, standard)? {
                ok.push(o);
            }
        }
        let mut objs = ok;
        if !standard && !objs.is_empty() && by_pid.is_some() && registry().hook_names("move_cost").is_some() {
            let bp = by_pid.unwrap();
            let (e, reqs) = self.move_cost(bp, &objs, dest)?;
            if (e != 0 || !reqs.is_empty()) && !self.pay(bp, e, &reqs, &crate::map! { kind = "move" }.into())? {
                return Ok(vec![]);
            }
        }
        if !standard && objs.len() > 1 && registry().hook_names("move_tax").is_some() {
            let mover = self.o(objs[0]).ctrl;
            while objs.len() > 1 {
                let reqs = self.move_tax(mover, &objs, dest)?;
                if reqs.is_empty() || self.pay(mover, 0, &reqs, &Value::None)? {
                    break;
                }
                objs.pop();
            }
        }
        let mut moved = vec![];
        for &o in &objs {
            let frm = self.o(o).loc;
            self.o_mut(o).loc = Some(dest);
            for gu in self.o(o).attached.clone() {
                if let Some(go) = self.obj(gu) {
                    self.o_mut(go).loc = Some(dest);
                }
            }
            moved.push((o, frm));
            if self.logging {
                let r = self.repr(o);
                let f = loc_str(frm);
                let d = loc_str(Some(dest));
                self.log(|| format!("  {r} moves {f}->{d}"));
            }
        }
        // un équipement avec [Deploy] peut aussi se déplacer (830)
        let units_moved: Vec<O> = moved.iter().map(|m| m.0).filter(|&o| self.spec(o).type_ == "Unit").collect();
        for &o in &units_moved {
            let (u, oid) = (self.o(o).uid, self.o(o).oid);
            self.hist.moved.push((u, oid));
        }
        if let Loc::Bf(i) = dest {
            if !units_moved.is_empty() {
                let mover = self.o(units_moved[0]).ctrl;
                let b = &mut self.bfs[i];
                if b.contested.is_none() && b.ctrl != Some(mover) {
                    b.contested = Some(mover); // 450
                }
            }
        }
        for &(o, frm) in &moved {
            self.emit("move", crate::map! { obj = o, frm = frm, to = dest, by = by_pid })?;
        }
        self.need_cleanup = true;
        Ok(moved)
    }

    /// `replace_choice_effect(o, what, duration)` : Gangplank, Naval (choice_rep). True si l'effet a lieu normalement.
    pub fn replace_choice_effect(&mut self, o: O, what: &str, duration: Value) -> Res<bool> {
        let Some(it) = self.resolving else { return Ok(true) };
        if !self.it(it).chosen.contains(&self.o(o).uid) {
            return Ok(true);
        }
        if let Some(im) = self.impl_o(o) {
            let h = im.hook("choice_rep");
            if !h.is_none() && h.call(self, &[o.into(), it.into(), what.into(), duration])?.truthy() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `stun(o, by_pid)`
    pub fn stun(&mut self, o: Option<O>, by_pid: Option<usize>) -> Res<()> {
        let Some(o) = o else { return Ok(()) };
        if !self.on_board(o) || self.spec(o).type_ != "Unit" || self.o(o).stunned {
            return Ok(());
        }
        if !self.replace_choice_effect(o, "stun", Value::None)? {
            return Ok(());
        }
        self.o_mut(o).stunned = true;
        let r = self.repr(o);
        self.log(|| format!("  {r} is stunned"));
        self.emit("stun", crate::map! { obj = o, by = by_pid })
    }

    /// `buff(o, by)` (426) : un marqueur +1 si l'unité n'en a pas (autant qu'on veut avec multi_buff).
    pub fn buff(&mut self, o: Option<O>, by: Option<usize>) -> Res<bool> {
        let Some(o) = o else { return Ok(false) };
        if !self.on_board(o) {
            return Ok(false);
        }
        let multi = self.impl_o(o).is_some_and(|im| im.multi_buff);
        if self.o(o).buff == 0 || multi {
            self.o_mut(o).buff += 1;
            let by = by.unwrap_or_else(|| match self.resolving {
                Some(r) => self.it(r).ctrl,
                None => self.o(o).ctrl,
            });
            self.emit("buff", crate::map! { obj = o, by = by })?;
            return Ok(true);
        }
        Ok(false)
    }

    /// `spend_buff(o, pid)` (745)
    pub fn spend_buff(&mut self, o: Option<O>, pid: Option<usize>) -> Res<bool> {
        let Some(o) = o else { return Ok(false) };
        if self.o(o).buff <= 0 {
            return Ok(false);
        }
        self.o_mut(o).buff -= 1;
        self.need_cleanup = true;
        let p = pid.unwrap_or(self.o(o).ctrl);
        self.emit("spend_buff", crate::map! { obj = o, pid = p })?;
        Ok(true)
    }

    /// `empower(o, by)` (441)
    pub fn empower(&mut self, o: Option<O>, by: Option<usize>) -> Res<bool> {
        let Some(o) = o else { return Ok(false) };
        if self.o(o).empowered {
            return Ok(false);
        }
        self.o_mut(o).empowered = true;
        let by = by.unwrap_or_else(|| match self.resolving {
            Some(r) => self.it(r).ctrl,
            None => self.o(o).ctrl,
        });
        let r = self.repr(o);
        self.log(|| format!("  {r} is empowered"));
        self.emit("empowered", crate::map! { obj = o, by = by })?;
        Ok(true)
    }

    pub fn disempower(&mut self, o: Option<O>) -> bool {
        match o {
            Some(o) if self.o(o).empowered => {
                self.o_mut(o).empowered = false;
                true
            }
            _ => false,
        }
    }

    /// `can_ready(o, by, kind)`
    pub fn can_ready(&mut self, o: O, by: Option<usize>, kind: Option<&str>) -> Res<bool> {
        if let Some(im) = self.impl_o(o) {
            let h = im.hook("no_ready");
            if !h.is_none() && h.call(self, &[o.into()])?.truthy() {
                return Ok(false);
            }
        }
        for (src, ia) in self.providers("aura_no_ready") {
            if ia.hook("aura_no_ready").call(self, &[src, o.into(), by.into(), kind.into()])?.truthy() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `ready_obj(o, by, kind)` ; émet 'ready'.
    pub fn ready_obj(&mut self, o: O, by: Option<usize>, kind: Option<&str>) -> Res<()> {
        if self.o(o).exh {
            let by = by.or(self.resolving.map(|r| self.it(r).ctrl));
            let kind: Option<String> = kind.map(|k| k.to_string()).or(self.resolving.map(|r| self.it(r).kind.to_string()));
            if !self.can_ready(o, by, kind.as_deref())? {
                return Ok(());
            }
            self.set_exhausted(o, false);
            self.emit("ready", crate::map! { obj = o, by = by })?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------ effets (dict de g.effects)
    pub fn has_effect(&self, eid: u64) -> bool {
        self.effects.iter().any(|e| e.eid == eid)
    }
    pub fn eff(&self, eid: u64) -> Option<&Map> {
        self.effects.iter().find(|e| e.eid == eid).map(|e| &e.d)
    }
    pub fn eff_mut(&mut self, eid: u64) -> Option<&mut Map> {
        self.effects.iter_mut().find(|e| e.eid == eid).map(|e| &mut e.d)
    }
    pub fn remove_effect(&mut self, eid: u64) {
        if let Some(i) = self.effects.iter().position(|e| e.eid == eid) {
            self.effects.remove(i);
        }
    }
    /// `g.effects.append(d)` ; renvoie l'identifiant de l'effet.
    pub fn add_effect(&mut self, d: Map) -> u64 {
        let e = self.new_effect(d);
        let id = e.eid;
        self.effects.push(e);
        id
    }

    pub fn desig_str(d: Option<Desig>) -> Value {
        match d {
            Some(Desig::Att) => "att".into(),
            Some(Desig::Def) => "def".into(),
            None => Value::None,
        }
    }
}

/// Écriture Python d'un emplacement dans le journal ('base', 0, 1, None).
pub fn loc_str(l: Option<Loc>) -> String {
    match l {
        Some(Loc::Base) => "base".into(),
        Some(Loc::Bf(i)) => i.to_string(),
        None => "None".into(),
    }
}

/// `is_bf` pour une valeur.
pub fn value_is_bf(v: &Value) -> bool {
    is_bf(v.opt_loc())
}
