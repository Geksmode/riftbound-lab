#!/usr/bin/env python3
"""Simplified Monte Carlo simulator: Akali, Rogue Assassin vs LeBlanc, Deceiver.

This is NOT a full rules engine. It models the parts that decide this matchup:
runes (exhaust = 1 energy, recycle = 1 power), 2 battlefields, conquer/hold scoring to 8
(final-point-by-conquest rule included), units entering exhausted, moves, combat by might,
Deathknell (+ Karthus doubling), Baited Hook, removal, counters, Zhonya, stun, LeBlanc's
legend copies, Akali's legend retreat, temporaries. Both sides are played by heuristics.
Costs and might come from cards/cards_unique.csv; behaviours are coded by card name.
Assumptions that are not in the card data are marked ASSUMPTION.
"""
import csv, random
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DB = {r["name"].lower(): r for r in csv.DictReader(open(ROOT / "cards" / "cards_unique.csv", encoding="utf-8"))}

GEAR_MIGHT = {"Long Sword": 2, "Sterak's Gage": 3}  # ASSUMPTION: equipment might bonus (icon, not in DB text)
CHAMPION = {"Akali": "Akali, Deadly Weapon", "LeBlanc": "LeBlanc, Fragmented"}
ASSAULT = {"LeBlanc, Fragmented": 1, "Black Rose Dignitary": 1}
DEATHKNELL = {"LeBlanc, Fragmented", "Honest Broker", "Soaring Scout", "Watchful Sentry", "Black Rose Dignitary",
              "Glasc Mixologist", "Ruined Rex", "Rift Herald", "Scuttle Crab", "Lonely Poro", "Ferrous Forerunner"}
REACTIVE = {"Defy", "Not So Fast", "Discipline", "Back Off", "Zhonya's Hourglass", "Block", "En Garde",
            "Long Sword", "Hidden Blade", "Deathgrip", "Sacrifice", "Crumbling Sands"}


def spec(name):
    r = DB[name.lower()]
    return dict(name=r["name"], type=r["type"], e=int(r["energy_cost"] or 0), p=int(r["power_cost"] or 0),
                dom=r["domain"].split("|"), might=int(r["might"]) if r["might"] else 0)


class Unit:
    def __init__(s, name, owner, might, token=False, temporary=False):
        s.name, s.owner, s.base, s.token, s.temporary = name, owner, might, token, temporary
        s.buff = s.temp = s.dmg = s.gear = 0
        s.exhausted, s.loc, s.stunned, s.empowered, s.watched = True, "base", False, False, False

    def might(s, attacking=False, defending=False):
        m = s.base + s.buff + s.temp + s.gear
        if s.empowered and s.name == "Akali, Deadly Weapon": m += 1
        if s.empowered and s.name == "Mournful Witness": m += 2
        if attacking: m += ASSAULT.get(s.name, 0)
        if s.watched: m = max(min(m, 1), m - 3)
        return max(m, 0)

    def value(s):
        v = s.might() + (3 if s.name == CHAMPION.get(s.owner.legend) else 0)
        v += {"Karthus, Eternal": 5, "Astral Heron": 2, "Akali, Silent": 2, "Stellacorn Herder": 2,
              "Kai'Sa, Survivor": 1}.get(s.name, 0)
        if s.temporary: v -= 3
        return v

    def __repr__(s):
        return f"{s.name}({s.might()})"


class Player:
    def __init__(s, game, legend, deck, runes, cfg):
        s.g, s.legend, s.cfg = game, legend, cfg
        s.deck = [spec(n) for n in deck if n != CHAMPION[legend]]
        s.champion = spec(CHAMPION[legend])
        random.shuffle(s.deck)
        s.rune_deck = list(runes); random.shuffle(s.rune_deck)
        s.runes = []  # [domain, ready]
        s.hand, s.trash, s.units, s.hidden, s.gears = [], [], [], [], []
        s.score, s.gold, s.turns, s.legend_ready, s.legend_emp = 0, 0, 0, True, False
        s.conquered_this_turn = set()

    # ---------- resources
    # A rune can be exhausted for 1 energy AND recycled for 1 power of its domain, so paying
    # e energy + p power needs e ready runes (or gold) and p runes of the domain (any state).
    def energy(s): return sum(1 for r in s.runes if r[1]) + s.gold

    def power(s, doms):
        return sum(1 for r in s.runes if r[0] in doms or "Any" in doms)

    def afford(s, card, e=None, p=None):
        e = card["e"] if e is None else e
        p = card["p"] if p is None else p
        return s.energy() >= e and s.power(card["dom"]) >= p

    def pay(s, e, p, doms):
        doms = set(doms)
        ok = lambda r: r[0] in doms or "Any" in doms
        recyc = sorted([r for r in s.runes if ok(r)], key=lambda r: r[1])[:p]
        assert len(recyc) == p, "not enough power"
        for r in recyc:
            if e > 0 and r[1]: r[1] = False; e -= 1
        for r in s.runes:
            if e == 0: break
            if r[1] and not any(r is x for x in recyc): r[1] = False; e -= 1
        while e > 0 and s.gold > 0: s.gold -= 1; e -= 1
        assert e == 0, "not enough energy"
        for r in recyc:
            s.runes.remove(r); s.rune_deck.append(r[0])

    def channel(s, n, ready=True):
        for _ in range(n):
            if s.rune_deck: s.runes.append([s.rune_deck.pop(0), ready])

    def draw(s, n=1):
        for _ in range(n):
            if s.deck: s.hand.append(s.deck.pop(0))
            else: s.g.burnout(s)

    def opp(s): return s.g.p[1 - s.g.p.index(s)]

    def at(s, loc): return [u for u in s.units if u.loc == loc]

    def has(s, name): return any(c["name"] == name for c in s.hand)

    def take(s, name):
        c = next(c for c in s.hand if c["name"] == name); s.hand.remove(c); return c


class Game:
    def __init__(s, akali_deck, leblanc_deck, bf_a, bf_l, cfg_a=None, cfg_l=None, seed=None, log=False):
        if seed is not None: random.seed(seed)
        s.logging, s.lines = log, []
        a = Player(s, "Akali", akali_deck["main"], akali_deck["runes"], cfg_a or {})
        l = Player(s, "LeBlanc", leblanc_deck["main"], leblanc_deck["runes"], cfg_l or {})
        s.p = [a, l] if random.random() < 0.5 else [l, a]
        s.first = s.p[0]
        s.bfs = [dict(name=bf_a, ctrl=None), dict(name=bf_l, ctrl=None)]
        random.shuffle(s.bfs)
        s.winner, s.turn, s.active = None, 0, None

    def log(s, *x):
        if s.logging: s.lines.append(" ".join(str(i) for i in x))

    def burnout(s, pl):
        # ASSUMPTION: burn out = opponent scores 1 (rules: shuffle trash, opponent gains a point)
        s.point(pl.opp(), 1, "burnout")

    def point(s, pl, n, why):
        if s.winner: return
        pl.score += n
        s.log(f"  +{n} {pl.legend} ({why}) -> {pl.score}")
        if pl.score >= 8: s.winner = pl

    def bonus(s, unit):  # Void Gate bonus damage for spells/abilities hitting a unit there
        return 1 if isinstance(unit.loc, int) and s.bfs[unit.loc]["name"] == "Void Gate" else 0

    # ---------------------------------------------------------------- deaths
    def damage(s, unit, n, src=None):
        if unit not in unit.owner.units: return
        unit.dmg += n
        if unit.dmg >= unit.might(): s.kill(unit, src)

    def kill(s, unit, src=None, cost=False):
        pl = unit.owner
        if unit not in pl.units: return
        if not cost and unit.value() >= pl.cfg.get("zhonya_min", 4) and not unit.token:
            z = next((h for h in pl.hidden if h["name"] == "Zhonya's Hourglass"), None)
            if z:
                pl.hidden.remove(z); pl.trash.append(z)
                unit.loc, unit.dmg, unit.exhausted = "base", 0, True
                s.log(f"  Zhonya sauve {unit}")
                return
        karthus = any(u.name == "Karthus, Eternal" for u in pl.units)
        pl.units.remove(unit)
        s.log(f"  meurt: {unit.name} ({pl.legend})")
        if not unit.token:
            pl.trash.append(spec(unit.name))
        if unit.name in DEATHKNELL or unit.token and unit.name in DEATHKNELL:
            for _ in range(2 if karthus else 1):
                s.deathknell(unit)

    def deathknell(s, u):
        pl, n = u.owner, u.name
        if n == "LeBlanc, Fragmented": pl.draw(2 if s.phase == "beginning" and s.active is pl else 1)
        elif n == "Honest Broker": pl.gold += 1
        elif n in ("Soaring Scout", "Black Rose Dignitary"): pl.channel(1, ready=False)
        elif n == "Watchful Sentry": pl.draw(1)
        elif n == "Glasc Mixologist":
            c = [c for c in pl.trash if c["type"] == "Unit" and c["e"] <= 3 and c["p"] <= 1]
            if c:
                best = max(c, key=lambda c: (c["name"] == "Karthus, Eternal") * 10 + c["might"])
                pl.trash.remove(best); s.enter(pl, best, "base")
        elif n == "Ruined Rex":
            s.ability_damage(pl, 4, "Rex")
        elif n == "Rift Herald":
            c = [c for c in pl.hand if c["type"] == "Unit" and pl.power(c["dom"]) >= c["p"]]
            if c:
                best = max(c, key=lambda c: c["e"]); pl.hand.remove(best)
                pl.pay(0, best["p"], best["dom"]); s.enter(pl, best, "base")
        elif n == "Lonely Poro":
            if not any(x.loc == u.loc for x in pl.units): pl.draw(1)
        elif n == "Ferrous Forerunner":
            for _ in range(2): s.enter(pl, dict(name="Mech", might=3, type="Unit", e=0, p=0, dom=[]), "base", token=True)

    def ability_damage(s, pl, n, why):
        """pl's ability deals n to the best enemy unit; enemy may counter with Not So Fast."""
        o = pl.opp()
        tg = [u for u in o.units if not s.untargetable(u)]
        if not tg: return
        killable = [u for u in tg if u.might() - u.dmg <= n + s.bonus(u)]
        t = max(killable or tg, key=lambda u: u.value())
        if s.try_counter(o, "ability", target=t, cost=(0, 0)): return
        s.log(f"  {why}: {n} a {t}")
        s.damage(t, n + s.bonus(t), pl)

    def untargetable(s, u):
        return u.name == "Akali, Silent" and not s.in_combat(u)

    def in_combat(s, u):
        return getattr(s, "combat_loc", None) == u.loc

    # ---------------------------------------------------------------- counters / reactions
    def try_counter(s, defender, kind, target=None, cost=(0, 0), spell=None):
        """defender may counter an enemy spell/ability that targets one of its units/gear."""
        cfg = defender.cfg
        if target is not None and isinstance(target, Unit) and target.value() < cfg.get("counter_min", 4):
            return False
        if defender.has("Not So Fast") and target is not None:
            c = spec("Not So Fast")
            if defender.afford(c):
                defender.pay(c["e"], c["p"], c["dom"]); defender.trash.append(defender.take("Not So Fast"))
                s.log(f"  Not So Fast contre {kind}"); return True
        if spell and defender.has("Defy") and spell["e"] <= 4 and spell["p"] <= 1:
            c = spec("Defy")
            if defender.afford(c):
                defender.pay(c["e"], c["p"], c["dom"]); defender.trash.append(defender.take("Defy"))
                s.log(f"  Defy contre {spell['name']}"); return True
        if spell and defender.has("Crumbling Sands") and s.spells_this_turn >= 2:
            c = spec("Crumbling Sands")
            if defender.afford(c):
                defender.pay(c["e"], c["p"], c["dom"]); defender.trash.append(defender.take("Crumbling Sands"))
                s.log(f"  Crumbling Sands contre {spell['name']}"); return True
        return False

    def cast(s, pl, name):
        c = pl.take(name); pl.pay(c["e"], c["p"], c["dom"]); s.spells_this_turn += 1
        s.log(f"  {pl.legend} lance {name}"); return c

    # ---------------------------------------------------------------- entering play
    def enter(s, pl, card, loc, token=False, temporary=False, ready=False):
        u = Unit(card["name"], pl, card["might"], token=token, temporary=temporary)
        u.loc, u.exhausted = loc, not ready
        pl.units.append(u)
        s.log(f"  {pl.legend} joue {u.name} -> {loc}")
        n, o = card["name"], pl.opp()
        if n == "Scuttle Crab": pl.draw(1)
        elif n == "Harnessed Dragon" and not token:
            tg = [x for x in o.units if not s.untargetable(x)]
            if tg:
                t = max(tg, key=lambda x: x.value())
                if not s.try_counter(o, "Harnessed Dragon", target=t): s.log(f"  Dragon tue {t}"); s.kill(t, pl)
        elif n == "Thousand-Tailed Watcher" and not token:
            for x in o.units: x.watched = True
        elif n in ("Disarming Rake", "Tomb-Raider Barbara", "Adaptatron_play"):
            if n == "Disarming Rake" or len(pl.runes) >= 7: s.kill_gear(pl)
        return u

    def kill_gear(s, pl):
        o = pl.opp()
        if o.gears:
            gname = "Baited Hook" if "Baited Hook" in o.gears else o.gears[0]
            o.gears.remove(gname); o.trash.append(spec(gname)); s.log(f"  detruit {gname}")
            return True
        return False

    # ---------------------------------------------------------------- turn
    def run(s, max_turns=40):
        for pl in s.p:
            pl.draw(4); s.mulligan(pl)
        while not s.winner and s.turn < max_turns:
            pl = s.p[s.turn % 2]
            s.take_turn(pl)
            s.turn += 1
        if not s.winner:
            a, l = s.p if s.p[0].legend == "Akali" else s.p[::-1]
            s.winner = a if a.score > l.score else l if l.score > a.score else None
        return s.winner.legend if s.winner else "draw"

    def mulligan(s, pl):
        keep_rule = pl.cfg.get("mulligan", "default")
        def bad(c):
            if pl.legend == "Akali":
                if keep_rule == "aggro": return not (c["type"] == "Unit" and c["e"] <= 4) and c["name"] != "Shuriken Flip"
                return c["e"] >= 6 or (c["name"] in ("Defy", "Not So Fast", "Crumbling Sands") and
                                         sum(x["name"] == c["name"] for x in pl.hand) > 1)
            return c["e"] >= 6
        out = sorted([c for c in pl.hand if bad(c)], key=lambda c: -c["e"])[:2]
        for c in out: pl.hand.remove(c)
        pl.draw(len(out))
        pl.deck.extend(out)

    def take_turn(s, pl):
        s.active, s.phase, s.spells_this_turn = pl, "beginning", 0
        pl.turns += 1
        pl.conquered_this_turn = set()
        s.log(f"T{s.turn+1} {pl.legend} score {pl.score}-{pl.opp().score} runes {len(pl.runes)} main {[c['name'] for c in pl.hand]}")
        for u in pl.units: u.exhausted = False; u.stunned = False
        for r in pl.runes: r[1] = True
        pl.legend_ready = True
        for g in list(pl.gears): pass
        pl.hook_used = False
        # temporaries die
        for u in [u for u in pl.units if u.temporary]: s.kill(u, cost=True)
        if s.winner: return
        # Dusk Rose Lab
        for i, bf in enumerate(s.bfs):
            if bf["name"] == "Dusk Rose Lab" and pl.legend == "LeBlanc":
                c = [u for u in pl.at(i) if u.name in DEATHKNELL and u.might() <= 2]
                if c and len(pl.at(i)) > 1: s.kill(c[0], cost=True); pl.draw(1)
        # scoring (hold)
        for i, bf in enumerate(s.bfs):
            if bf["ctrl"] is pl:
                if bf["name"] == "Forgotten Monument" and pl.turns < 3: continue
                s.point(pl, 1, f"hold {bf['name']}")
                s.legend_copy(pl, i)
                if s.winner: return
        s.phase = "channel"
        pl.channel(3 if (pl is not s.first and pl.turns == 1) else 2)  # ASSUMPTION: 2nd player +1 rune turn 1
        s.phase = "draw"; pl.draw(1)
        if s.winner: return
        s.phase = "main"
        if pl.legend == "Akali": AkaliAI(s, pl).main()
        else: LeBlancAI(s, pl).main()
        # end of turn cleanup
        n = getattr(pl, "targon", 0); pl.targon = 0
        for r in pl.runes:
            if n and not r[1]: r[1] = True; n -= 1
        for p in s.p:
            for u in p.units: u.temp = 0; u.dmg = 0; u.watched = False
        pl.gold = min(pl.gold, 3)

    def legend_copy(s, pl, i):
        if pl.legend != "LeBlanc" or not pl.legend_ready or not pl.hand: return
        here = [u for u in pl.at(i) if not u.temporary]
        if not here: return
        t = max(here, key=lambda u: (u.name in ("Ruined Rex", "Glasc Mixologist", "Watchful Sentry")) * 3 + u.might())
        worst = min(pl.hand, key=lambda c: c["e"] if c["type"] == "Unit" else c["e"] + 2)
        pl.hand.remove(worst); pl.trash.append(worst); pl.legend_ready = False
        tok = s.enter(pl, dict(name=t.name, might=t.base, type="Unit", e=0, p=0, dom=[]), i, token=True,
                      temporary=True, ready=True)
        s.log(f"  LeBlanc copie {t.name}")

    # ---------------------------------------------------------------- combat
    def conquer(s, pl, i, units):
        bf = s.bfs[i]
        if bf["ctrl"] is pl: return
        bf["ctrl"] = pl
        if bf["name"] == "Forgotten Monument" and pl.turns < 3:
            return
        pl.conquered_this_turn.add(i)
        if bf["name"] == "Targon's Peak": pl.targon = getattr(pl, "targon", 0) + 2
        if pl.score + 1 >= 8 and len(pl.conquered_this_turn) < len(s.bfs):
            pl.draw(1); s.log("  dernier point par conquete refuse -> pioche")
        else:
            s.point(pl, 1, f"conquer {bf['name']}")
        for u in units:
            if u.name == "Kai'Sa, Survivor": pl.draw(1)
            if u.name == "Adaptatron" and s.kill_gear(pl): u.buff = max(u.buff, 1)
        s.legend_copy(pl, i)

    def move(s, u, dest, spell=False):
        src = u.loc
        if not spell: u.exhausted = True
        u.loc = dest
        pl = u.owner
        if u.name == "Stellacorn Herder": pl.draw(1)
        if u.name == "Rift Herald" and isinstance(dest, int):
            top = pl.deck[:3]; hit = [c for c in top if c["type"] == "Unit"]
            if hit:
                c = max(hit, key=lambda c: c["might"]); pl.deck.remove(c); pl.hand.append(c)
            top = pl.deck[:2]; del pl.deck[:2]; pl.deck.extend(top)
        if u.name == "Akali, Silent" and isinstance(dest, int): u.temp += 2
        if isinstance(src, int) and s.bfs[src]["name"] == "Back-Alley Bar": u.temp += 1
        if u.name == "Akali, Deadly Weapon":
            dmg = 2 if u.empowered else 1
            o = pl.opp()
            cand = [x for x in o.units if x.loc in (src, dest) and isinstance(x.loc, int) and not s.untargetable(x)]
            AkaliAI(s, pl).ping(cand, dmg)

    def combat(s, att_pl, i, attackers):
        d_pl = att_pl.opp()
        s.combat_loc = i
        for u in attackers: s.move(u, i)
        if s.winner: s.combat_loc = None; return
        defenders = d_pl.at(i)
        if s.bfs[i]["name"] == "Threshold of the Gray" and defenders:
            att_pl.gold += 1; d_pl.gold += 1
        s.log(f"  combat @{s.bfs[i]['name']}: {attackers} vs {defenders}")
        ai = {p: (AkaliAI(s, p) if p.legend == "Akali" else LeBlancAI(s, p)) for p in s.p}
        # reaction rounds: defender then attacker, twice
        for _ in range(2):
            ai[d_pl].react(i, defending=True)
            ai[att_pl].react(i, defending=False)
        attackers = [u for u in att_pl.at(i)]
        defenders = [u for u in d_pl.at(i)]
        if defenders and attackers:
            # Akali legend: retreat a unit that would die (own turn only)
            if att_pl.legend == "Akali": ai[att_pl].legend_retreat(i)
            attackers = [u for u in att_pl.at(i)]
            s.deal(attackers, defenders, True)
            s.deal(defenders, attackers, False)
            for u in list(attackers) + list(defenders):
                if u in u.owner.units and u.dmg >= u.might(attacking=u in attackers, defending=u in defenders):
                    s.kill(u, None)
            for u in att_pl.at(i) + d_pl.at(i):
                if u.name == "Mournful Witness": u.empowered = True
        s.combat_loc = None
        if s.winner: return
        attackers = att_pl.at(i); defenders = d_pl.at(i)
        if attackers and not defenders:
            s.conquer(att_pl, i, attackers)
        elif attackers and defenders:
            for u in attackers: u.loc = "base"  # ASSUMPTION: attackers that do not win go back to base

    def deal(s, src, tgt, attacking):
        total = sum(u.might(attacking=attacking, defending=not attacking) for u in src if not u.stunned)
        order = sorted(tgt, key=lambda u: (not s.is_tank(u), u.name == "LeBlanc, Everywhere At Once",
                                           u.might() - u.dmg))
        for u in order:
            need = u.might(attacking=not attacking, defending=attacking) - u.dmg
            if total <= 0: break
            hit = min(need, total) if u is not order[-1] else total
            u.dmg += hit; total -= hit

    def is_tank(s, u):
        return u.name in ("Blitzcrank, Impassive",) or getattr(u, "tank", False)


def side_might(units, attacking):
    return sum(u.might(attacking=attacking, defending=not attacking) for u in units if not u.stunned)


# ====================================================================== AIs
class BaseAI:
    def __init__(s, g, pl): s.g, s.pl, s.o = g, pl, pl.opp()

    def play_unit(s, card, loc="base", accelerate=False):
        pl = s.pl
        pl.pay(card["e"] + (1 if accelerate else 0), card["p"] + (1 if accelerate else 0),
               card["dom"])
        if card in pl.hand: pl.hand.remove(card)
        return s.g.enter(pl, card, loc, ready=accelerate)

    def reserve(s): return 0

    def free_energy(s): return s.pl.energy() - s.reserve()

    def movement(s):
        g, pl = s.g, s.pl
        if g.winner: return
        for _ in range(3):
            ready = [u for u in pl.units if not u.exhausted and not u.stunned and (u.loc == "base" or (
                isinstance(u.loc, int) and g.bfs[u.loc]["name"] == "Windswept Hillock" and len(pl.at(u.loc)) > 1))]
            if not ready: break
            best = None
            for i, bf in enumerate(g.bfs):
                enemies = s.o.at(i)
                mine_there = pl.at(i)
                if enemies:
                    need = side_might(enemies, False) + s.margin(i)
                    grp, tot = [], 0
                    for u in sorted(ready, key=lambda u: -u.might(attacking=True)):
                        grp.append(u); tot += u.might(attacking=True)
                        if tot > need: break
                    if tot > need and len(grp) <= len(ready):
                        sc = 3 + (bf["ctrl"] is not None) - 0.1 * len(grp)
                        if best is None or sc > best[0]: best = (sc, i, grp)
                elif bf["ctrl"] is not pl:
                    u = min(ready, key=lambda u: u.value())
                    sc = 2.5
                    if u.loc == i: continue
                    if best is None or sc > best[0]: best = (sc, i, [u])
            if not best: break
            _, i, grp = best
            g.combat(pl, i, grp)
            if g.winner: return
        # defend own battlefields with spare ready units
        ready = [u for u in pl.units if not u.exhausted and not u.stunned and u.loc == "base"]
        for i, bf in enumerate(g.bfs):
            if bf["ctrl"] is pl and ready and not pl.at(i) and s.should_defend():
                u = max(ready, key=lambda u: u.might()); ready.remove(u); g.move(u, i)

    def should_defend(s): return True

    def margin(s, i): return 0


class AkaliAI(BaseAI):
    def reserve(s):
        pl, cfg = s.pl, s.pl.cfg
        r = 0
        if pl.has("Not So Fast") and cfg.get("hold_nsf", True): r = max(r, 3)
        if pl.has("Defy") and cfg.get("hold_defy", True): r = max(r, 2)
        return min(r, max(0, len(pl.runes) - 3))

    def margin(s, i):
        # respect Vi ambush / Watcher; aggressive configs take more risk
        o = s.o
        m = 0 if s.pl.cfg.get("aggro", True) else 1
        if o.at(i) and o.energy() >= 6 and s.pl.cfg.get("respect_vi", False): m += 3
        return m

    def ping(s, cand, dmg):
        g, plan = s.g, s.pl.cfg.get("plan", "kill")
        if not cand: return
        killable = [u for u in cand if u.might() - u.dmg <= dmg + g.bonus(u)]
        pick = None
        if plan == "kill":
            pick = max(killable, key=lambda u: u.value()) if killable else max(cand, key=lambda u: u.value())
        else:  # "leblanc": only Karthus, champion, or units that block a battlefield
            pri = [u for u in killable if u.name in ("Karthus, Eternal", "LeBlanc, Fragmented", "Glasc Mixologist")]
            pick = pri[0] if pri else (max(killable, key=lambda u: u.value()) if killable else None)
        if pick:
            if g.try_counter(s.o, "ping", target=pick): return
            g.damage(pick, dmg + g.bonus(pick), s.pl)

    def removal_targets(s, min_value):
        plan = s.pl.cfg.get("plan", "kill")
        tg = [u for u in s.o.units if isinstance(u.loc, int) and not u.temporary]
        if plan == "leblanc":
            # kill Karthus first, then only what blocks a battlefield; avoid feeding deathknells
            k = [u for u in s.o.units if u.name == "Karthus, Eternal" and isinstance(u.loc, int)]
            if k: return k
            tg = [u for u in tg if u.name not in ("Watchful Sentry", "Soaring Scout", "Honest Broker",
                                                  "Black Rose Dignitary", "Ruined Rex")]
        return sorted([u for u in tg if u.value() >= min_value], key=lambda u: -u.value())

    def main(s):
        g, pl, o = s.g, s.pl, s.o
        cfg = pl.cfg
        # empower legend / champion with spare mana late
        # 1. gear removal on Hook
        for n in ("Brittle Steel",):
            if pl.has(n) and "Baited Hook" in o.gears and pl.afford(spec(n)):
                c = g.cast(pl, n)
                if not g.try_counter(o, n, spell=c): g.kill_gear(pl)
                pl.trash.append(c)
        # 2. removal spells
        s.removal()
        if g.winner: return
        # 3. deploy units
        s.deploy()
        if g.winner: return
        # 4. clear blockers (Charm / Flip / Falling Star) so ready units can conquer, then combat
        s.enable_attacks()
        s.charm()
        s.flip_move()
        s.movement()
        if g.winner: return
        s.deploy(post=True)
        s.hide()
        if not pl.legend_emp and pl.cfg.get("empower_legend", True) and pl.energy() - s.reserve() >= 3 and len(pl.runes) >= 7:
            pl.pay(3, 1, ["Any"]); pl.legend_emp = True
        if pl.energy() >= 3 and pl.power(["Fury"]) >= 1:
            dw = next((u for u in pl.units if u.name == "Akali, Deadly Weapon" and not u.empowered), None)
            if dw and pl.energy() - 3 >= s.reserve(): pl.pay(2, 1, ["Fury"]); dw.empowered = True

    def removal(s):
        g, pl, o = s.g, s.pl, s.o
        for _ in range(4):
            done = False
            if pl.has("Falling Star") and pl.afford(spec("Falling Star")):
                tg = s.removal_targets(3)
                kills = [u for u in tg if u.might() - u.dmg <= 3 + g.bonus(u)]
                big = [u for u in tg if u.might() - u.dmg <= 6 + 2 * g.bonus(u)]
                if len(kills) >= 1 or big:
                    c = g.cast(pl, "Falling Star")
                    if not g.try_counter(o, "Falling Star", spell=c, target=(kills or big)[0]):
                        if len(kills) >= 2:
                            for u in kills[:2]: g.damage(u, 3 + g.bonus(u), pl)
                        else:
                            t = (big or kills)[0]
                            g.damage(t, 3 + g.bonus(t), pl); g.damage(t, 3 + g.bonus(t), pl)
                    pl.trash.append(c); done = True
            if pl.has("Shuriken Flip") and pl.afford(spec("Shuriken Flip")):
                tg = [u for u in s.removal_targets(2) if u.might() - u.dmg <= 2 + g.bonus(u)]
                if tg:
                    c = g.cast(pl, "Shuriken Flip")
                    if not g.try_counter(o, "Shuriken Flip", spell=c, target=tg[0]):
                        g.damage(tg[0], 2 + g.bonus(tg[0]), pl); s.flip_followup()
                    pl.trash.append(c); done = True
            if not done: break

    def enable_attacks(s):
        g, pl, o = s.g, s.pl, s.o
        for _ in range(3):
            ready = [u for u in pl.units if not u.exhausted and not u.stunned and u.loc == "base"]
            if not ready: return
            atk = side_might(ready, True)
            acted = False
            for i, bf in enumerate(g.bfs):
                en = o.at(i)
                if not en or atk > side_might(en, False) + s.margin(i): continue
                # cheapest single answer that makes the attack winning
                for u in sorted(en, key=lambda u: -u.might()):
                    rest = side_might([x for x in en if x is not u], False) + s.margin(i)
                    if atk <= rest: continue
                    left = u.might() - u.dmg
                    if pl.has("Charm") and pl.afford(spec("Charm")):
                        c = g.cast(pl, "Charm")
                        if not g.try_counter(o, "Charm", spell=c, target=u):
                            u.loc, u.exhausted = "base", True; g.log(f"  Charm renvoie {u}")
                        pl.trash.append(c); acted = True; break
                    if pl.has("Shuriken Flip") and pl.afford(spec("Shuriken Flip")) and left <= 2 + g.bonus(u):
                        c = g.cast(pl, "Shuriken Flip")
                        if not g.try_counter(o, "Shuriken Flip", spell=c, target=u): g.damage(u, 2 + g.bonus(u), pl)
                        pl.trash.append(c); acted = True; break
                    if pl.has("Falling Star") and pl.afford(spec("Falling Star")) and left <= 6 + 2 * g.bonus(u):
                        c = g.cast(pl, "Falling Star")
                        if not g.try_counter(o, "Falling Star", spell=c, target=u):
                            g.damage(u, 3 + g.bonus(u), pl); g.damage(u, 3 + g.bonus(u), pl)
                        pl.trash.append(c); acted = True; break
                if acted: break
            if not acted: return

    def flip_followup(s):
        """move a friendly unit for free (spell move): conquer an empty bf or draw with Herder."""
        g, pl = s.g, s.pl
        for i, bf in enumerate(g.bfs):
            if not s.o.at(i) and bf["ctrl"] is not pl:
                cand = [u for u in pl.units if u.loc == "base"]
                if cand:
                    u = max(cand, key=lambda u: (u.name in ("Stellacorn Herder", "Akali, Deadly Weapon"), u.might()))
                    g.move(u, i, spell=True)
                    if not s.o.at(i) and u in pl.units: g.conquer(pl, i, [u])
                    return

    def flip_move(s):
        # cast Shuriken Flip just for the move if a conquest is available (flow from trash too)
        g, pl = s.g, s.pl
        for i, bf in enumerate(g.bfs):
            if not s.o.at(i) and bf["ctrl"] is not pl and not [u for u in pl.units if not u.exhausted and u.loc == "base"]:
                if pl.has("Shuriken Flip") and pl.afford(spec("Shuriken Flip")) and [u for u in pl.units if u.loc == "base"]:
                    c = g.cast(pl, "Shuriken Flip")
                    if not g.try_counter(s.o, "Shuriken Flip", spell=c):
                        s.flip_followup()
                    pl.trash.append(c)
                    return

    def charm(s):
        g, pl = s.g, s.pl
        if not pl.has("Charm"): return
        for i, bf in enumerate(g.bfs):
            en = s.o.at(i)
            ready = [u for u in pl.units if not u.exhausted and u.loc == "base"]
            if len(en) == 1 and ready and pl.afford(spec("Charm")) and en[0].might() >= 3:
                c = g.cast(pl, "Charm")
                if not g.try_counter(s.o, "Charm", spell=c, target=en[0]):
                    en[0].loc = "base"; en[0].exhausted = True
                    g.log(f"  Charm renvoie {en[0]}")
                pl.trash.append(c)
                return

    def deploy(s, post=False):
        g, pl = s.g, s.pl
        units = sorted([c for c in pl.hand if c["type"] == "Unit"], key=lambda c: -c["e"])
        if pl.champion:
            units = [pl.champion] + units
        for c in units:
            res = 0 if post else s.reserve()
            if pl.energy() - res >= c["e"] and pl.afford(c):
                acc = c["name"] == "Kai'Sa, Survivor" and pl.afford(c, c["e"] + 1, c["p"] + 1) and \
                    pl.energy() - res >= c["e"] + 1
                if c is pl.champion:
                    pl.champion = None
                    pl.pay(c["e"], c["p"], c["dom"]); g.enter(pl, c, "base")
                else:
                    s.play_unit(c, accelerate=acc)
        for n in ("Long Sword", "Sterak's Gage"):
            if pl.has(n) and pl.units and pl.energy() - s.reserve() >= spec(n)["e"] and pl.afford(spec(n)):
                c = g.cast(pl, n); pl.trash.append(c)
                u = max(pl.units, key=lambda u: u.value()); u.gear += GEAR_MIGHT[n]

    def hide(s):
        pl = s.pl
        for n in ("Zhonya's Hourglass", "Back Off", "Mischievous Marai"):
            if pl.has(n) and len(pl.runes) >= pl.cfg.get("hide_min_runes", 6) and len(pl.hidden) < 2:
                c = pl.take(n); pl.pay(0, 1, ["Any"]); pl.hidden.append(c)

    def react(s, i, defending):
        g, pl, o = s.g, s.pl, s.o
        mine, theirs = pl.at(i), o.at(i)
        if not mine or not theirs: return
        my_m, their_m = side_might(mine, not defending), side_might(theirs, defending)
        losing = my_m <= their_m if not defending else my_m < their_m
        if not losing: return
        # Back Off (hidden or hand): stun their biggest
        h = next((h for h in pl.hidden if h["name"] == "Back Off"), None)
        if h or (pl.has("Back Off") and pl.afford(spec("Back Off"))):
            t = max(theirs, key=lambda u: u.might(attacking=defending))
            if h: pl.hidden.remove(h); pl.trash.append(h); pl.draw(0)
            else: c = g.cast(pl, "Back Off"); pl.trash.append(c); pl.draw(1)
            if not g.try_counter(o, "Back Off", target=None):
                t.stunned = True; g.log(f"  Back Off stun {t}")
            return
        if defending:
            h = next((h for h in pl.hidden if h["name"] == "Mischievous Marai"), None)
            if h:
                pl.hidden.remove(h); u = g.enter(pl, h, i)
                t = [x for x in theirs if x.might() - x.dmg <= 2 + g.bonus(x)]
                if t: g.damage(t[0], 2, pl)
                return
        for n, bonus in (("Discipline", 2), ("Long Sword", 2), ("En Garde", 1), ("Block", 3)):
            if pl.has(n) and pl.afford(spec(n)):
                if n == "Block" and not defending: continue
                c = g.cast(pl, n)
                if g.try_counter(o, n, spell=c): pl.trash.append(c); return
                u = max(mine, key=lambda u: u.value())
                if n == "Long Sword": u.gear += 2
                else: u.temp += bonus + (1 if n == "En Garde" and len(mine) == 1 else 0)
                if n == "Discipline": pl.draw(1)
                pl.trash.append(c)
                return

    def legend_retreat(s, i):
        g, pl, o = s.g, s.pl, s.o
        if not pl.legend_ready: return
        mine, theirs = pl.at(i), o.at(i)
        if side_might(mine, True) > side_might(theirs, False): return
        # we lose: send the most valuable unit home instead of letting it die
        u = max(mine, key=lambda u: u.value())
        if u.value() >= 4:
            pl.legend_ready = False
            g.log(f"  legende Akali rappelle {u}")
            g.move(u, "base", spell=True)
            if pl.legend_emp: u.exhausted = False


class LeBlancAI(BaseAI):
    def reserve(s):
        pl = s.pl
        r = 0
        if pl.has("Vi, Peacekeeper") and any(isinstance(u.loc, int) for u in pl.units): r = 6
        return min(r, max(0, len(pl.runes) - 4))

    def margin(s, i):
        return 1 if s.o.energy() >= 2 else 0

    def main(s):
        g, pl, o = s.g, s.pl, s.o
        # Baited Hook
        s.hook()
        if g.winner: return
        s.big_removal()
        s.deploy()
        if g.winner: return
        s.mirror()
        s.movement()
        if g.winner: return
        s.deploy(post=True)
        if pl.has("Hidden Blade") and len(pl.hidden) < 1 and len(pl.runes) >= 6:
            c = pl.take("Hidden Blade"); pl.pay(0, 1, ["Any"]); pl.hidden.append(c)

    def hook(s):
        g, pl = s.g, s.pl
        if "Baited Hook" not in pl.gears: return
        for _ in range(pl.gears.count("Baited Hook")):
            if not (pl.energy() >= 1 and pl.power(["Order"]) >= 1): return
            karthus = any(u.name == "Karthus, Eternal" for u in pl.units)
            cand = [u for u in pl.units if not u.temporary and u.name != "Karthus, Eternal" and
                    (u.might() >= 5 or (karthus and u.name in DEATHKNELL))]
            if not cand: return
            u = max(cand, key=lambda u: (u.name == "Ruined Rex") * 3 + u.might())
            pl.pay(1, 1, ["Order"])
            m = u.might()
            g.log(f"  Hook sacrifie {u}")
            g.kill(u, cost=True)
            if g.winner: return
            top = pl.deck[:5]
            hits = [c for c in top if c["type"] == "Unit" and c["might"] <= m + 1]
            if hits:
                best = max(hits, key=lambda c: (c["name"] == "Harnessed Dragon") * 2 + c["might"])
                pl.deck.remove(best)
                rest = [c for c in pl.deck[:4]]
                g.enter(pl, best, "base")
            # recycle rest of the top
            top = pl.deck[:4]; del pl.deck[:4]; pl.deck.extend(top)
            if g.winner: return

    def big_removal(s):
        g, pl, o = s.g, s.pl, s.o
        tg = [u for u in o.units if isinstance(u.loc, int) and not g.untargetable(u) and u.value() >= 5]
        if tg and pl.has("Hidden Blade") and pl.afford(spec("Hidden Blade")):
            t = max(tg, key=lambda u: u.value())
            c = g.cast(pl, "Hidden Blade")
            if not g.try_counter(o, "Hidden Blade", spell=c, target=t):
                g.kill(t, pl); o.draw(2)
            pl.trash.append(c)

    def deploy(s, post=False):
        g, pl = s.g, s.pl
        champ_played = pl.champion is None
        units = sorted([c for c in pl.hand if c["type"] in ("Unit",) and c["name"] != "Vi, Peacekeeper"],
                       key=lambda c: (c["name"] == "Karthus, Eternal", c["e"]), reverse=True)
        if not post and pl.has("Vi, Peacekeeper") and not any(isinstance(u.loc, int) for u in pl.units):
            units.insert(0, next(c for c in pl.hand if c["name"] == "Vi, Peacekeeper"))
        if pl.champion: units.append(pl.champion)
        for c in units:
            res = 0 if post else s.reserve()
            if pl.energy() - res >= c["e"] and pl.afford(c):
                acc = c["name"] == "Thousand-Tailed Watcher" and pl.afford(c, c["e"] + 1, c["p"] + 1)
                if c is pl.champion:
                    pl.champion = None; pl.pay(c["e"], c["p"], c["dom"]); g.enter(pl, c, "base")
                else:
                    s.play_unit(c, accelerate=acc)
                if g.winner: return
        if pl.has("Baited Hook") and pl.afford(spec("Baited Hook")) and pl.energy() - (0 if post else s.reserve()) >= 3:
            c = pl.take("Baited Hook"); pl.pay(c["e"], c["p"], c["dom"]); pl.gears.append("Baited Hook")

    def mirror(s):
        g, pl = s.g, s.pl
        if not pl.has("Mirror Image") or not pl.afford(spec("Mirror Image")): return
        allu = [u for u in pl.units + s.o.units if not u.token]
        if not allu: return
        t = max(allu, key=lambda u: (u.name == "Ruined Rex") * 4 + u.base)
        if t.base < 5: return
        c = g.cast(pl, "Mirror Image")
        if not g.try_counter(s.o, "Mirror Image", spell=c):
            g.enter(pl, dict(name=t.name, might=t.base, type="Unit", e=0, p=0, dom=[]), "base", token=True,
                    temporary=True, ready=True)
        pl.trash.append(c)

    def react(s, i, defending):
        g, pl, o = s.g, s.pl, s.o
        mine, theirs = pl.at(i), o.at(i)
        if not theirs: return
        my_m, their_m = side_might(mine, not defending), side_might(theirs, defending)
        losing = (my_m < their_m) if defending else (my_m <= their_m)
        if defending and losing and pl.has("Vi, Peacekeeper") and mine and pl.afford(spec("Vi, Peacekeeper")):
            c = pl.take("Vi, Peacekeeper"); pl.pay(c["e"], c["p"], c["dom"]); g.enter(pl, c, i)
            return
        # Hidden Blade on the biggest attacker
        h = next((h for h in pl.hidden if h["name"] == "Hidden Blade"), None)
        if losing and (h or (pl.has("Hidden Blade") and pl.afford(spec("Hidden Blade")))) and \
                pl.power(["Order"]) >= 1:
            tg = [u for u in theirs if not g.untargetable(u)]
            if tg:
                t = max(tg, key=lambda u: u.might())
                if h: pl.hidden.remove(h); c = h; pl.pay(0, 1, ["Order"]); g.log("  LeBlanc revele Hidden Blade")
                else: c = g.cast(pl, "Hidden Blade")
                if not g.try_counter(o, "Hidden Blade", spell=c, target=t):
                    g.kill(t, pl); o.draw(2)
                pl.trash.append(c)
                return
        # Sacrifice a mighty unit that dies anyway / Deathgrip
        if losing and mine:
            mighty = [u for u in mine if u.might() >= 5]
            if pl.has("Sacrifice") and mighty and pl.afford(spec("Sacrifice")):
                c = g.cast(pl, "Sacrifice"); u = max(mighty, key=lambda u: (u.name == "Ruined Rex", u.might()))
                if not g.try_counter(o, "Sacrifice", spell=c):
                    g.kill(u, cost=True); pl.draw(2); pl.channel(1, ready=False)
                pl.trash.append(c)
                return
            if pl.has("Deathgrip") and len(mine) >= 2 and pl.afford(spec("Deathgrip")):
                c = g.cast(pl, "Deathgrip")
                small = min(mine, key=lambda u: u.value()); big = max(mine, key=lambda u: u.might())
                if not g.try_counter(o, "Deathgrip", spell=c) and small is not big:
                    m = small.might(); g.kill(small, cost=True); big.temp += m; pl.draw(1)
                pl.trash.append(c)
