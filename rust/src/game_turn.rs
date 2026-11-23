//! `game.py`, fin : nettoyage (323), showdowns et combat (464-466), contrôle et score (469-471), structure du tour
//! (315-317), copie de la partie.
use std::collections::BTreeSet;

use crate::cards::registry;
use crate::game::{Counter, Desig, Game, GameOver, Res, Showdown, Zone, O};
use crate::game_events::deploy_kill;
use crate::value::{is_bf, Loc, Map, Value};

impl Game {
    /// `cleanup` (323)
    pub fn cleanup(&mut self) -> Res<()> {
        self.need_cleanup = false;
        // 1. victoire
        for pid in [self.tp, 1 - self.tp] {
            if self.p[pid].points >= self.victory && self.p[pid].points > self.p[1 - pid].points {
                self.win(pid)?;
            }
        }
        // 2. désignations de combat
        if let Some(sd) = self.sd.clone() {
            if sd.combat && (sd.stage == "open" || sd.stage == "damage") {
                for u in self.all_units() {
                    if self.o(u).loc == Some(Loc::Bf(sd.bf)) {
                        let want = if self.o(u).ctrl == sd.attacker { Desig::Att } else { Desig::Def };
                        if self.o(u).desig != Some(want) {
                            self.o_mut(u).desig = Some(want);
                        }
                        self.combat_trigger_check(u)?;
                    } else if self.o(u).desig.is_some() {
                        self.o_mut(u).desig = None;
                    }
                }
            }
        }
        // 3b. dégâts mortels
        let mut dead = vec![];
        for u in self.all_units() {
            if self.lethal(u)? {
                dead.push(u);
            }
        }
        if !dead.is_empty() {
            self.kill(&dead, None, false)?;
        }
        // « becomes Mighty » (709), seulement quand une carte modélisée l'écoute
        if registry().track_mighty {
            for u in self.all_units() {
                let m = self.might(u)? >= 5;
                if self.o(u).was_mighty == Some(false) && m {
                    self.emit("becomes_mighty", crate::map! { obj = u })?;
                }
                self.o_mut(u).was_mighty = Some(m);
            }
        }
        // « When my Might becomes N or more » (might_watch = N) : émet 'might_reached'
        if let Some(names) = registry().hook_names("might_watch") {
            for u in self.all_units() {
                if names.contains(self.cname(u)) {
                    let n = self.impl_o(u).unwrap().might_watch.unwrap();
                    let m = self.might(u)?;
                    if self.o(u).might_seen.is_some_and(|s| s < n && n <= m) {
                        self.emit("might_reached", crate::map! { obj = u, n = n })?;
                    }
                    self.o_mut(u).might_seen = Some(m);
                }
            }
        }
        // 4. perte de contrôle
        let open_state = self.chain.is_empty();
        let sd_bf = self.sd.as_ref().map(|s| s.bf);
        for i in 0..self.bfs.len() {
            if let Some(c) = self.bfs[i].ctrl {
                if !self.units(None, Some(Loc::Bf(i))).iter().any(|&u| self.o(u).ctrl == c)
                    && open_state
                    && sd_bf != Some(i)
                {
                    let n = self.bfs[i].name.clone();
                    self.log(|| format!("  P{c} loses control of {n}"));
                    self.bfs[i].ctrl = None;
                }
            }
        }
        // 5. rappel des équipements non attachés aux battlefields ; cartes cachées défaussées
        for g in self.gear(None) {
            if self.at_bf(g) && self.o(g).attached_to.is_none() && self.spec(g).type_ != "Unit" && !self.has_kw(g, "Deploy")? {
                self.o_mut(g).loc = Some(Loc::Base); // équipement non-unité sans [Deploy] (323.6, 830.1.b.3)
            }
        }
        for i in 0..self.bfs.len() {
            for c in self.bfs[i].facedowns.clone() {
                if self.bfs[i].ctrl != Some(self.o(c).owner) {
                    let (r, n) = (self.repr(c), self.bfs[i].name.clone());
                    self.log(|| format!("  hidden {r} at {n} is trashed"));
                    self.trash_facedown(i, c);
                }
            }
        }
        // 6/7. mise en place
        for i in 0..self.bfs.len() {
            let here = self.units(None, Some(Loc::Bf(i)));
            let ctrls: BTreeSet<usize> = here.iter().map(|&u| self.o(u).ctrl).collect();
            let cont = self.bfs[i].contested;
            self.bfs[i].sd_staged = cont.is_some() && here.iter().any(|&u| Some(self.o(u).ctrl) == cont);
            self.bfs[i].combat_staged = cont.is_some() && ctrls.len() == 2;
        }
        // 8. retirer Contested
        for i in 0..self.bfs.len() {
            let cont = self.bfs[i].contested;
            if cont.is_some()
                && !self.units(None, Some(Loc::Bf(i))).iter().any(|&u| Some(self.o(u).ctrl) == cont)
                && sd_bf != Some(i)
            {
                let b = &mut self.bfs[i];
                b.contested = None;
                b.sd_staged = false;
                b.combat_staged = false;
            }
            if self.bfs[i].contested.is_none() && sd_bf != Some(i) {
                for u in self.units(None, Some(Loc::Bf(i))) {
                    if Some(self.o(u).ctrl) != self.bfs[i].ctrl {
                        // 8a
                        let n = self
                            .units(None, Some(Loc::Bf(i)))
                            .iter()
                            .map(|&x| self.o(x).ctrl)
                            .collect::<BTreeSet<_>>()
                            .len();
                        let b = &mut self.bfs[i];
                        b.contested = Some(self.objs[u.0 as usize].ctrl);
                        b.sd_staged = true;
                        b.combat_staged = n == 2;
                        self.need_cleanup = true;
                        break;
                    }
                }
            }
        }
        if self.need_cleanup {
            return Ok(());
        }
        // 9/10. ouvrir un showdown ou un combat dans un état Neutral Open
        if self.sd.is_none() && self.chain.is_empty() && self.trigq.is_empty() {
            let staged_c: Vec<usize> = (0..self.bfs.len()).filter(|&i| self.bfs[i].combat_staged).collect();
            let staged_s: Vec<usize> =
                (0..self.bfs.len()).filter(|&i| self.bfs[i].sd_staged && !self.bfs[i].combat_staged).collect();
            if let Some(&b) = staged_c.first() {
                self.start_showdown(b, true)?;
            } else if let Some(&b) = staged_s.first() {
                self.start_showdown(b, false)?;
            }
        } else if let Some(sd) = self.sd.clone() {
            if sd.stage == "open" && !sd.combat && self.bfs[sd.bf].combat_staged && self.chain.is_empty() {
                self.log(|| "  showdown becomes a combat".to_string()); // 10a
                let att = self.bfs[sd.bf].contested.unwrap();
                let sd = self.sd.as_mut().unwrap();
                sd.combat = true;
                sd.attacker = att;
                sd.defender = 1 - att;
                self.begin_combat()?;
            }
        }
        Ok(())
    }

    /// `trash_facedown(b, c)` : une carte cachée retirée d'un battlefield va dans la défausse de son propriétaire
    /// (323.7.5).
    pub fn trash_facedown(&mut self, b: usize, c: O) {
        let i = self.bfs[b].facedowns.iter().position(|&x| x == c).unwrap();
        self.bfs[b].facedowns.remove(i);
        let dest = self.trash_dest(c, "trash", Some("facedown"));
        self.reset_obj(c);
        self.o_mut(c).oid += 1;
        self.o_mut(c).zone = Some(Zone::parse(dest));
        let owner = self.o(c).owner;
        if dest == "banish" {
            self.p[owner].banish.push(c);
        } else {
            self.p[owner].trash.push(c);
        }
    }

    /// `combat_trigger_check(u)` : déclenchements d'attaque et de défense, une fois par combat.
    pub fn combat_trigger_check(&mut self, u: O) -> Res<()> {
        let uid = self.o(u).uid;
        let bf = self.sd.as_ref().unwrap().bf;
        if self.o(u).desig == Some(Desig::Att) && !self.sd.as_ref().unwrap().att_seen.contains(&uid) {
            let sd = self.sd.as_mut().unwrap();
            sd.att_seen.insert(uid);
            add_member(sd, uid);
            let n = self.kw_value(u, "Disarm")?;
            if n > 0 {
                // [Disarm X] (831) : un seul déclenchement d'attaque, valeur totale
                crate::cards_helpers::disarm_trigger(self, u, n)?;
            }
            self.emit("attack", crate::map! { obj = u, bf = bf })?;
        }
        if self.o(u).desig == Some(Desig::Def) && !self.sd.as_ref().unwrap().def_seen.contains(&uid) {
            let sd = self.sd.as_mut().unwrap();
            sd.def_seen.insert(uid);
            add_member(sd, uid);
            self.emit("defend", crate::map! { obj = u, bf = bf })?;
        }
        Ok(())
    }

    /// `start_showdown(b, combat)`
    pub fn start_showdown(&mut self, b: usize, combat: bool) -> Res<()> {
        self.bfs[b].sd_staged = false;
        self.bfs[b].combat_staged = false;
        let attacker = self.bfs[b].contested.unwrap();
        self.sd = Some(Showdown {
            bf: b,
            combat,
            attacker,
            defender: 1 - attacker,
            focus: attacker,
            passes: 0,
            stage: "open",
            att_seen: BTreeSet::new(),
            def_seen: BTreeSet::new(),
            members: vec![],
            recalled: false,
            assigned: Value::None,
            excess: [0, 0],
            result: None,
            attrs: Map::new(),
        });
        let n = self.bfs[b].name.clone();
        self.log(|| format!("{} at {n}, attacker P{attacker}", if combat { "COMBAT" } else { "showdown" }));
        self.stats.add(if combat { "combats" } else { "showdowns" }, 1);
        let sdv = self.sd_value();
        self.emit("showdown_start", crate::map! { bf = b, combat = combat, sd = sdv })?;
        if combat {
            self.begin_combat()?;
        }
        Ok(())
    }

    /// `begin_combat(sd)` (464.2) : effets de début de combat, désignations, déclenchements ; l'attaquant a le Focus.
    pub fn begin_combat(&mut self) -> Res<()> {
        let (bf, attacker) = {
            let sd = self.sd.as_ref().unwrap();
            (sd.bf, sd.attacker)
        };
        let sdv = self.sd_value();
        self.emit("combat_start", crate::map! { bf = bf, sd = sdv })?;
        for u in self.units(None, Some(Loc::Bf(bf))) {
            self.o_mut(u).desig = Some(if self.o(u).ctrl == attacker { Desig::Att } else { Desig::Def });
        }
        let mut us = self.units(None, Some(Loc::Bf(bf)));
        us.sort_by_key(|&x| self.o(x).ctrl != attacker);
        for u in us {
            self.combat_trigger_check(u)?;
        }
        let sd = self.sd.as_mut().unwrap();
        sd.focus = sd.attacker;
        sd.passes = 0;
        Ok(())
    }

    /// `end_showdown`
    pub fn end_showdown(&mut self) -> Res<()> {
        let sd = self.sd.as_mut().unwrap();
        if sd.combat {
            sd.stage = "damage";
            return Ok(());
        }
        // 348.2 : showdown hors combat
        let b = sd.bf;
        let ctrls: BTreeSet<usize> = self.units(None, Some(Loc::Bf(b))).iter().map(|&u| self.o(u).ctrl).collect();
        self.sd = None;
        self.bfs[b].contested = None;
        if ctrls.len() == 1 {
            let pid = *ctrls.iter().next().unwrap();
            if self.bfs[b].ctrl != Some(pid) {
                self.establish_control(pid, b)?;
            }
        }
        self.need_cleanup = true;
        Ok(())
    }

    /// `combat_step`
    pub fn combat_step(&mut self) -> Res<()> {
        let (stage, b) = {
            let sd = self.sd.as_ref().unwrap();
            (sd.stage, sd.bf)
        };
        let here = |g: &Game, d: Desig| -> Vec<O> {
            g.units(None, Some(Loc::Bf(b))).into_iter().filter(|&u| g.o(u).desig == Some(d)).collect()
        };
        match stage {
            "damage" => {
                let (atk, dfn) = (here(self, Desig::Att), here(self, Desig::Def));
                if !atk.is_empty() && !dfn.is_empty() {
                    self.combat_damage(&atk, &dfn)?;
                }
                // nettoyage de combat (466.1) : morts, soin de toutes les unités, rappel des attaquants s'il reste des défenseurs
                let mut dead = vec![];
                for u in self.all_units() {
                    if self.lethal(u)? {
                        dead.push(u);
                    }
                }
                if !dead.is_empty() {
                    self.kill(&dead, None, false)?;
                }
                for u in self.all_units() {
                    self.o_mut(u).damage = 0;
                    self.o_mut(u).dmg_by.clear();
                }
                let (mut atk, dfn) = (here(self, Desig::Att), here(self, Desig::Def));
                if !atk.is_empty() && !dfn.is_empty() {
                    // Symbol of the Solari : « If a combat where you are the attacker ends in a tie, recall ALL units instead. »
                    let attacker = self.sd.as_ref().unwrap().attacker;
                    if self
                        .providers("tie_recall_all")
                        .iter()
                        .any(|(src, im)| self.o(src.obj()).ctrl == attacker && im.tie_recall_all)
                    {
                        atk.extend(dfn);
                        self.log(|| "  tie: every unit is recalled".to_string());
                    }
                    for u in atk {
                        self.recall(u);
                    }
                    self.sd.as_mut().unwrap().recalled = true;
                }
                self.sd.as_mut().unwrap().stage = "result";
                self.need_cleanup = true;
            }
            "result" => {
                let pids: BTreeSet<usize> = self.units(None, Some(Loc::Bf(b))).iter().map(|&u| self.o(u).ctrl).collect();
                let recalled = self.sd.as_ref().unwrap().recalled;
                let result = if recalled || pids.len() != 1 { None } else { pids.iter().next().copied() };
                self.sd.as_mut().unwrap().result = result;
                if let Some(r) = result {
                    self.emit("combat_won", crate::map! { pid = r, bf = b })?;
                }
                self.sd.as_mut().unwrap().stage = "control";
            }
            "control" => {
                let pids: BTreeSet<usize> = self.units(None, Some(Loc::Bf(b))).iter().map(|&u| self.o(u).ctrl).collect();
                self.bfs[b].contested = None;
                if pids.is_empty() {
                    self.bfs[b].ctrl = None;
                } else if pids.len() == 1 {
                    let pid = *pids.iter().next().unwrap();
                    if self.bfs[b].ctrl != Some(pid) {
                        self.establish_control(pid, b)?;
                    }
                }
                for c in self.bfs[b].facedowns.clone() {
                    if Some(self.o(c).owner) != self.bfs[b].ctrl {
                        self.trash_facedown(b, c);
                    }
                }
                self.sd.as_mut().unwrap().stage = "end";
                self.need_cleanup = true;
            }
            "end" => {
                let members: Vec<O> = self.sd.as_ref().unwrap().members.iter().filter_map(|&u| self.obj(u)).collect();
                for u in self.all_units() {
                    self.o_mut(u).desig = None;
                }
                self.sd = None;
                self.emit("combat_end", crate::map! { bf = b, members = members })?;
                self.need_cleanup = true;
            }
            s => panic!("étape de combat inconnue {s}"),
        }
        Ok(())
    }

    /// `deals_combat_damage(u)` (465) : pas stun, ni « I don't deal combat damage », ni exclue par un autre objet.
    pub fn deals_combat_damage(&mut self, u: O) -> Res<bool> {
        if self.o(u).stunned {
            return Ok(false);
        }
        if let Some(im) = self.impl_o(u) {
            let h = im.hook("no_combat_damage");
            if !h.is_none() && h.call(self, &[u.into()])?.truthy() {
                return Ok(false);
            }
        }
        for (src, ia) in self.providers("aura_no_combat") {
            if ia.hook("aura_no_combat").call(self, &[src, u.into()])?.truthy() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `combat_damage(sd, atk, dfn)` (465) : l'attaquant assigne d'abord, dégâts simultanés.
    pub fn combat_damage(&mut self, atk: &[O], dfn: &[O]) -> Res<()> {
        let mut a_total = 0;
        for &u in atk {
            if self.deals_combat_damage(u)? {
                a_total += 0.max(self.might(u)?);
            }
        }
        let mut d_total = 0;
        for &u in dfn {
            if self.deals_combat_damage(u)? {
                d_total += 0.max(self.might(u)?);
            }
        }
        self.log(|| format!("  combat damage: attackers {a_total} vs defenders {d_total}"));
        let (attacker, defender) = {
            let sd = self.sd.as_mut().unwrap();
            sd.excess = [0, 0];
            (sd.attacker, sd.defender)
        };
        let assign_a = self.assign_damage(attacker, a_total, dfn)?;
        let assign_d = self.assign_damage(defender, d_total, atk)?;
        let mut dealt = vec![];
        let asv = |a: &Vec<(O, i64)>| Value::List(a.iter().map(|(u, n)| Value::Tuple(vec![(*u).into(), (*n).into()])).collect());
        self.sd.as_mut().unwrap().assigned = Value::Tuple(vec![asv(&assign_a), asv(&assign_d)]);
        for (u, n) in assign_a.into_iter().chain(assign_d) {
            if n <= 0 || !self.on_board(u) {
                continue;
            }
            let by = 1 - self.o(u).ctrl;
            let n = self.damage_replace(u, n, &crate::map! { kind = "combat", by = by, item = Value::None, target = u })?;
            if n > 0 {
                self.mark_damage(u, n, Some(by), "combat");
                dealt.push((u, n));
            }
        }
        for (u, n) in dealt {
            let by = 1 - self.o(u).ctrl;
            self.emit("damaged", crate::map! { obj = u, amount = n, kind = "combat", by = by })?;
        }
        self.need_cleanup = true;
        Ok(())
    }

    /// `lethal_need(u, pid)` (465.2.c) : 1 suffit quand les dégâts du joueur qui assigne sont toujours mortels pour u.
    pub fn lethal_need(&mut self, u: O, pid: Option<usize>) -> Res<i64> {
        if let Some(pid) = pid {
            if pid != self.o(u).ctrl {
                for (src, im) in self.providers("lethal_any") {
                    if self.o(src.obj()).ctrl == pid && im.hook("lethal_any").call(self, &[src.clone(), u.into()])?.truthy() {
                        return Ok(1 + self.o(u).prevent);
                    }
                }
            }
        }
        Ok(1.max(self.might(u)? - self.o(u).damage) + self.o(u).prevent)
    }

    /// `assign_damage(pid, total, targets)` : ordres d'assignation légaux (Tank d'abord, Backline et « assigné en
    /// dernier » à la fin, létal complet avant l'unité suivante, excédent sur la dernière).
    pub fn assign_damage(&mut self, pid: usize, total: i64, targets: &[O]) -> Res<Vec<(O, i64)>> {
        let mut out: Vec<(O, i64)> = vec![];
        if total <= 0 || targets.is_empty() {
            return Ok(out);
        }
        let bf = self.o(targets[0]).loc;
        let mut no_tank = false;
        for (src, im) in self.providers("ignore_tank") {
            if im.hook("ignore_tank").call(self, &[src, pid.into(), bf.into()])?.truthy() {
                no_tank = true;
                break;
            }
        }
        let mut tank = vec![];
        let mut last = vec![];
        for &u in targets {
            tank.push(!no_tank && self.has_kw(u, "Tank")?);
            let al = self.impl_o(u).is_some_and(|im| im.assign_last);
            last.push(self.has_kw(u, "Backline")? || al);
        }
        let tiers: Vec<Vec<O>> = vec![
            targets.iter().enumerate().filter(|(i, _)| tank[*i]).map(|(_, &u)| u).collect(),
            targets.iter().enumerate().filter(|(i, _)| !tank[*i] && !last[*i]).map(|(_, &u)| u).collect(),
            targets.iter().enumerate().filter(|(i, _)| last[*i] && !tank[*i]).map(|(_, &u)| u).collect(),
        ];
        if crate::actions::full_choices(self, Some(pid)) {
            return self.assign_by_hand(pid, total, tiers);
        }
        let mut order = vec![];
        for t in tiers {
            // ask(pid, "damage_order", [t], targets=t) : une seule option, donc t
            order.extend(t);
        }
        let mut left = total;
        let mut excess = 0;
        let n_order = order.len();
        for (i, &u) in order.iter().enumerate() {
            if left <= 0 {
                break;
            }
            let need = self.lethal_need(u, Some(pid))?;
            let n = if i == n_order - 1 { left } else { left.min(need) };
            out.push((u, n));
            excess += 0.max(n - need);
            left -= n;
        }
        if left > 0 && !order.is_empty() {
            let last_u = *order.last().unwrap();
            if let Some(e) = out.iter_mut().find(|e| e.0 == last_u) {
                e.1 += left;
            } else {
                out.push((last_u, left));
            }
            excess += left;
        }
        if let Some(sd) = self.sd.as_mut() {
            sd.excess[pid] = excess;
        }
        Ok(out)
    }

    /// `assign_by_hand(pid, total, tiers)` : joueur humain, unité par unité (465.2.c.3).
    pub fn assign_by_hand(&mut self, pid: usize, total: i64, tiers: Vec<Vec<O>>) -> Res<Vec<(O, i64)>> {
        let (mut out, mut left, mut excess) = (vec![], total, 0);
        let mut tiers: Vec<Vec<O>> = tiers.into_iter().filter(|t| !t.is_empty()).collect();
        while left > 0 && !tiers.is_empty() {
            let t = tiers[0].clone();
            let u = if t.len() == 1 {
                t[0]
            } else {
                let mut need = Map::new();
                for &x in &t {
                    let n = self.lethal_need(x, Some(pid))?;
                    need.set(&self.o(x).uid.to_string(), n);
                }
                self.ask_obj(pid, "damage_pick", &t, crate::map! { left = left, need = need })?.unwrap()
            };
            let i = tiers[0].iter().position(|&x| x == u).unwrap();
            tiers[0].remove(i);
            if tiers[0].is_empty() {
                tiers.remove(0);
            }
            let need = self.lethal_need(u, Some(pid))?;
            let n = if tiers.is_empty() { left } else { left.min(need) };
            out.push((u, n));
            excess += 0.max(n - need);
            left -= n;
        }
        if let Some(sd) = self.sd.as_mut() {
            sd.excess[pid] = excess;
        }
        Ok(out)
    }

    /// `establish_control(pid, b)`
    pub fn establish_control(&mut self, pid: usize, b: usize) -> Res<()> {
        self.bfs[b].ctrl = Some(pid);
        let n = self.bfs[b].name.clone();
        self.log(|| format!("  P{pid} takes control of {n}"));
        if !self.bfs[b].scored.contains(&pid) {
            self.conquer(pid, b)?;
        }
        Ok(())
    }

    /// `can_score(pid, b)`
    pub fn can_score(&mut self, pid: usize, b: usize) -> Res<bool> {
        let name = self.bfs[b].name.clone();
        if let Some(im) = self.impl_n(&name) {
            if im.no_score.truthy() && im.no_score.call(self, &[pid.into(), Value::Int(b as i64)])?.truthy() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `score_point(pid, b, why)` (469-471) : remplacement point_rep (Otterpus).
    pub fn score_point(&mut self, pid: usize, b: usize, why: &str) -> Res<bool> {
        for (src, im) in self.providers("point_rep") {
            if im.hook("point_rep").call(self, &[src, pid.into(), Value::Int(b as i64), why.into()])?.truthy() {
                return Ok(false);
            }
        }
        let n = self.bfs[b].name.clone();
        self.gain_point(pid, &format!("{why} {n}"))
    }

    /// `conquer(pid, b)` (469.1, 471) : la conquête marque le battlefield (règle du dernier point) et déclenche.
    pub fn conquer(&mut self, pid: usize, b: usize) -> Res<()> {
        self.stats.add(&format!("conquer_P{pid}"), 1);
        if self.can_score(pid, b)? {
            self.bfs[b].scored.insert(pid);
            if self.p[pid].points >= self.victory - 1 {
                if self.bfs.iter().all(|x| x.scored.contains(&pid)) {
                    self.score_point(pid, b, "conquer")?;
                } else {
                    self.log(|| format!("  P{pid} draws instead of the final point"));
                    self.stats.add("final_point_draw", 1);
                    self.draw(pid, 1)?;
                }
            } else {
                self.score_point(pid, b, "conquer")?;
            }
        } else {
            self.bfs[b].scored.insert(pid);
        }
        let units = self.units(Some(pid), Some(Loc::Bf(b)));
        for &u in &units {
            let (uid, oid) = (self.o(u).uid, self.o(u).oid);
            self.hist.conquered.push((uid, oid));
        }
        self.hist.conq_bf[pid].push(b);
        self.scoring_event("conquer", pid, b, &units)
    }

    /// `scoring_event(ev, pid, b, units)` : effets de conquête / tenue (Hunt, évènements), « additional time »
    /// (scoring_extra) et équipements qui échangent les deux (fx_swap_scoring).
    pub fn scoring_event(&mut self, ev: &str, pid: usize, b: usize, units: &[O]) -> Res<()> {
        let n0 = self.trigq.len();
        self.hunt(pid, units)?;
        self.emit(ev, crate::map! { pid = pid, bf = b, units = units.to_vec() })?;
        let other = if ev == "conquer" { "hold" } else { "conquer" };
        for &u in units {
            if self.on_board(u) && !self.o(u).attached.is_empty() && !self.fx_providers("fx_swap_scoring", Some(u)).is_empty() {
                self.emit_self(u, other, crate::map! { pid = pid, bf = b, units = units.to_vec() })?;
            }
        }
        let mut extra = 0;
        for (src, im) in self.providers("scoring_extra") {
            extra += im
                .hook("scoring_extra")
                .call(self, &[src, pid.into(), Value::Int(b as i64), ev.into()])?
                .opt_int()
                .unwrap_or(0);
        }
        if extra != 0 {
            let new: Vec<_> = self.trigq[n0..].iter().copied().filter(|&t| self.it(t).ctrl == pid).collect();
            for _ in 0..extra {
                for &t in &new {
                    let mut t2 = self.it(t).clone();
                    t2.id = self.counters.next_item();
                    self.items.push(t2);
                    let id = crate::game::ItemId(self.items.len() as u32 - 1);
                    self.trigq.push(id);
                }
            }
        }
        Ok(())
    }

    /// `emit_self(o, ev, info)` : un évènement seulement aux capacités déclenchées de o (et de ses équipements).
    pub fn emit_self(&mut self, o: O, ev: &str, info: Map) -> Res<()> {
        let infov = Value::Map(info);
        if let Some(im) = self.impl_o(o) {
            if !im.on_event.is_none() {
                im.on_event.call(self, &[o.into(), ev.into(), infov.clone()])?;
            }
        }
        for gu in self.o(o).attached.clone() {
            if let Some(go) = self.obj(gu) {
                if let Some(ig) = self.impl_o(go) {
                    if !ig.effect_event.is_none() {
                        ig.effect_event.call(self, &[go.into(), o.into(), ev.into(), infov.clone()])?;
                    }
                }
            }
        }
        Ok(())
    }

    /// `hunt(pid, units)` : [Hunt X] (823).
    pub fn hunt(&mut self, pid: usize, units: &[O]) -> Res<()> {
        for &u in units {
            let n = self.kw_value(u, "Hunt")?;
            if n > 0 {
                let f = Value::func("hunt", |g, a| {
                    let it = a[0].item();
                    let (c, n) = (g.it(it).ctrl, g.it(it).data.g("n").int());
                    g.gain_xp(c, n)?;
                    Ok(Value::None)
                });
                let (r, uid) = (self.repr(u), self.o(u).uid);
                self.queue_trigger(pid, &format!("Hunt {r}"), f, crate::map! { n = n }, Some(uid), false, Value::None, Value::None);
            }
        }
        Ok(())
    }

    /// `hold(pid, b)`
    pub fn hold(&mut self, pid: usize, b: usize) -> Res<()> {
        self.stats.add(&format!("hold_P{pid}"), 1);
        if self.can_score(pid, b)? {
            self.bfs[b].scored.insert(pid);
            if self.score_point(pid, b, "hold")? {
                self.hist.hold_pts[pid] += 1;
            }
        } else {
            self.bfs[b].scored.insert(pid);
        }
        let units = self.units(Some(pid), Some(Loc::Bf(b)));
        self.scoring_event("hold", pid, b, &units)?;
        // [Deploy] (830.1.b) : « When an opponent holds here, kill this. »
        for o in self.board.clone() {
            if self.o(o).loc == Some(Loc::Bf(b)) && self.o(o).ctrl != pid && self.is_gear(o) && self.has_kw(o, "Deploy")? {
                let (ctrl, uid, oid, cn) = (self.o(o).ctrl, self.o(o).uid, self.o(o).oid, self.cname(o).to_string());
                self.queue_trigger(
                    ctrl,
                    &format!("Deploy {cn}"),
                    deploy_kill(),
                    crate::map! { uid = uid, oid = oid },
                    Some(uid),
                    false,
                    Value::None,
                    Value::None,
                );
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------ structure du tour (315-317)
    /// `setup`
    pub fn setup(&mut self) -> Res<()> {
        for pid in 0..2 {
            let mut d = std::mem::take(&mut self.p[pid].deck);
            self.rng.shuffle(&mut d);
            self.p[pid].deck = d;
            let mut r = std::mem::take(&mut self.p[pid].rune_deck);
            self.rng.shuffle(&mut r);
            self.p[pid].rune_deck = r;
        }
        for pid in [self.first, 1 - self.first] {
            self.draw(pid, 4)?;
        }
        for pid in [self.first, 1 - self.first] {
            let mut agent = std::mem::replace(&mut self.agents[pid], Box::new(crate::game_events::FirstChoice));
            let r = agent.mulligan(self, pid);
            self.agents[pid] = agent;
            let keep_out: Vec<O> = r?.into_iter().filter(|c| self.p[pid].hand.contains(c)).take(2).collect();
            for c in &keep_out {
                let i = self.p[pid].hand.iter().position(|x| x == c).unwrap();
                self.p[pid].hand.remove(i);
            }
            self.draw(pid, keep_out.len() as i64)?;
            for &c in &keep_out {
                self.o_mut(c).zone = None;
            }
            self.recycle_cards(pid, &keep_out)?;
        }
        self.tp = self.first;
        self.stage = "awaken";
        self.new_turn_bookkeeping();
        Ok(())
    }

    /// `new_turn_bookkeeping`
    pub fn new_turn_bookkeeping(&mut self) {
        self.turn_no += 1;
        let tp = self.tp;
        self.p[tp].turns += 1;
        self.played = [vec![], vec![]];
        self.finalized = [vec![], vec![]];
        self.spells_played = [0, 0];
        self.reset_hist();
        for b in &mut self.bfs {
            b.scored.clear();
        }
        let (t, ln, a, b) = (self.turn_no, self.p[tp].legend_name.clone(), self.p[0].points, self.p[1].points);
        self.log(|| format!("===== turn {t}: P{tp} ({ln}) points {a}-{b}"));
    }

    /// `turn_step`
    pub fn turn_step(&mut self) -> Res<()> {
        let st = self.stage;
        self.phase = Some(st);
        let tp = self.tp;
        match st {
            "setup" => return self.setup(),
            "awaken" => {
                for o in self.board.clone() {
                    if self.o(o).ctrl == tp {
                        self.ready_obj(o, Some(tp), Some("awaken"))?;
                    }
                }
                for r in self.p[tp].runes.clone() {
                    self.r_mut(r).exhausted = false;
                }
                let l = self.p[tp].legend;
                self.set_exhausted(l, false);
                self.stage = "beginning";
            }
            "beginning" => {
                self.emit("beginning_start", crate::map! { pid = tp })?;
                self.stage = "scoring";
            }
            "scoring" => {
                for b in 0..self.bfs.len() {
                    if self.bfs[b].ctrl == Some(tp) && !self.bfs[b].scored.contains(&tp) {
                        self.hold(tp, b)?;
                    }
                }
                self.stage = "channel";
            }
            "channel" => {
                let mut n = if tp != self.first && self.p[tp].turns == 1 { 3 } else { 2 }; // 486.7
                for (src, im) in self.providers("channel_mod") {
                    // Sandstone Chimera
                    n = im.hook("channel_mod").call(self, &[src, tp.into(), n.into()])?.int();
                }
                self.channel(tp, n, false)?;
                self.stage = "draw";
            }
            "draw" => {
                if self.effects_of("skip_draw", Some(tp)).is_empty() {
                    // Endless Riches : « Skip your Draw Phase. »
                    self.draw(tp, 1)?;
                }
                self.stage = "main_start";
            }
            "main_start" => {
                for p in &mut self.p {
                    p.pool_e = 0;
                    p.pool_p = Counter::default();
                    p.pool_r.clear();
                }
                self.emit("main_start", crate::map! { pid = tp })?;
                self.stage = "main";
            }
            "end_step" => {
                self.emit("end_turn", crate::map! { pid = tp })?;
                self.stage = "expire";
            }
            "expire" => {
                for u in self.all_units() {
                    let o = self.o_mut(u);
                    o.damage = 0;
                    o.dmg_by.clear();
                    o.stunned = false;
                    o.prevent = 0;
                }
                let turn: Value = "turn".into();
                for o in self.board.clone() {
                    let ob = self.o_mut(o);
                    ob.mods.retain(|m| m.1 != turn);
                    ob.grants.retain(|g| g.2 != turn);
                    ob.abs.retain(|a| a.1 != turn);
                    ob.xtags.retain(|t| t.1 != turn);
                    ob.dmg_by.clear();
                }
                self.effects.retain(|e| e.d.g("dur") != turn);
                for p in &mut self.p {
                    p.pool_e = 0;
                    p.pool_p = Counter::default();
                    p.pool_r.clear();
                }
                return self.next_turn();
            }
            s => panic!("étape inconnue {s}"),
        }
        self.need_cleanup = true;
        Ok(())
    }

    /// `next_turn` : partie nulle (winner = -1) au-delà du tour 80.
    pub fn next_turn(&mut self) -> Res<()> {
        if self.turn_no >= 80 {
            self.winner = Some(-1);
            return Err(GameOver);
        }
        if !self.extra_turns.is_empty() {
            self.tp = self.extra_turns.remove(0).int() as usize;
        } else {
            self.tp = 1 - self.tp;
        }
        self.stage = "awaken";
        self.new_turn_bookkeeping();
        Ok(())
    }

    // ------------------------------------------------------------------ actions
    pub fn options_main(&mut self, pid: usize) -> Res<Vec<Value>> {
        crate::actions::main_options(self, pid)
    }

    pub fn options_focus(&mut self, pid: usize) -> Res<Vec<Value>> {
        let mut v = vec![Value::Tuple(vec!["pass".into()])];
        v.extend(crate::actions::timed_options(self, pid, false)?);
        Ok(v)
    }

    pub fn options_priority(&mut self, pid: usize) -> Res<Vec<Value>> {
        let mut v = vec![Value::Tuple(vec!["pass".into()])];
        v.extend(crate::actions::timed_options(self, pid, true)?);
        Ok(v)
    }

    pub fn do_action(&mut self, a: &Value) -> Res<()> {
        crate::actions::do_action(self, a)
    }

    // ------------------------------------------------------------------ copie (anticipation de l'IA)
    /// `clone` : copie profonde sans agents (à reposer par l'appelant) ni journal.
    pub fn clone_game(&self) -> Game {
        let mut g = self.clone();
        g.lines.clear();
        g.logging = false;
        g
    }
}

/// `sd.members.add(uid)` : ensemble, gardé dans l'ordre d'arrivée.
fn add_member(sd: &mut Showdown, uid: u32) {
    if !sd.members.contains(&uid) {
        sd.members.push(uid);
    }
}

#[allow(dead_code)]
fn _unused(_: Zone, _: Loc) -> bool {
    is_bf(None)
}
