"""Riftbound rules engine (1v1 Duel/Match), written from the Core Rules dated 2026-07-16
(rules/source/core_rules_2026-07-16.txt). Rule numbers are quoted in comments.

The engine is a state machine:
  g.advance() runs the game until a player must take a top-level decision and returns a Decision
  (main action in Neutral Open, priority in a Closed state, focus in a Showdown Open state);
  g.apply(action) applies the chosen action.
Choices made while a card or ability is being played or resolved (targets, "you may", damage
assignment, ordering...) are asked synchronously to the players' agents through g.ask().
Card behaviours live in cards.py (one entry per card name).
"""
import csv, copy, random
from collections import Counter
from pathlib import Path

ROOT = Path('/mnt/attach/project-files/riftbound')
SPEC = {}
for r in csv.DictReader(open(ROOT / "cards" / "cards_unique.csv", encoding="utf-8")):
    SPEC[r["name"]] = dict(
        name=r["name"], type=r["type"], super=r["supertype"], domains=tuple(d for d in r["domain"].split("|") if d),
        e=int(r["energy_cost"] or 0), p=int(r["power_cost"] or 0),
        might=int(r["might"]) if r["might"] not in ("", None) else None,
        tags=set(t for t in r["tags"].split("|") if t), text=r["text"],
        keywords=tuple(k for k in r["keywords"].split("|") if k))
# tokens (rule 187)
SPEC["Mech"] = dict(name="Mech", type="Unit", super="Token", domains=(), e=0, p=0, might=3, tags={"Mech"}, text="")
SPEC["Reflection"] = dict(name="Reflection", type="Unit", super="Token", domains=(), e=0, p=0, might=0, tags=set(), text="")
SPEC["Gold"] = dict(name="Gold", type="Gear", super="Token", domains=(), e=0, p=0, might=None, tags=set(), text="")
# token names printed in card texts ("Play a 1 might Recruit unit token") that have no row of their own
for _n, _m, _t, _x in (("Recruit", 1, {"Recruit"}, ""), ("Sand Soldier", 2, set(), ""), ("Tentacle", 1, {"Bilgewater"}, ""),
                       ("Shadow Clone", 0, set(), "When I attack, you may banish a unit from your trash. If you do, give me "
                                                  "[Assault 4] this turn.")):
    if _n not in SPEC:
        SPEC[_n] = dict(name=_n, type="Unit", super="Token", domains=(), e=0, p=0, might=_m, tags=_t, text=_x)
for _s in SPEC.values():
    _s.setdefault("keywords", ())
# Might Bonus of Equipment is printed in an icon that the card database does not carry (rule 718.4)
EQUIP_BONUS = {"Long Sword": 2, "Sterak's Gage": 3, "Pendulum Blade": 1}
DOMAINS = ("Fury", "Calm", "Mind", "Body", "Chaos", "Order")
VICTORY = 8
ANY = frozenset(DOMAINS)


class GameOver(Exception):
    pass


_CARDS = []


def _cards():
    """The cards module (imported lazily: cards.py imports game.py)."""
    if not _CARDS:
        import cards
        _CARDS.append(cards)
    return _CARDS[0]


class Obj:
    """A card or token, in any zone. Board-only state is reset whenever it changes zone (rule 359.3.e.4)."""
    _n = 0

    def __init__(s, name, owner, token=False):
        Obj._n += 1
        s.uid = Obj._n
        s.name = name            # printed name
        s.owner = owner
        s.ctrl = owner
        s.token = token
        s.zone = None
        s.oid = 0                # incarnation counter: a target is legal only for the same incarnation
        s.reset()

    def reset(s):
        s.loc = None             # 'base' or battlefield index (0/1)
        s.exhausted = False
        s.damage = 0
        s.buff = 0
        s.stunned = False
        s.empowered = False
        s.mods = []              # [amount, duration]  duration: 'turn' | None
        s.grants = []            # [keyword, value, duration]
        s.attached = []          # uids of gear attached to this unit
        s.attached_to = None     # uid of the unit this gear is attached to
        s.desig = None           # 'att' | 'def'
        s.prevent = 0            # remaining prevent value this turn (Ki Barrier)
        s.copy = None            # name copied (Reflection)
        s.ctrl = s.owner
        s.entered_turn = None
        s.hidden_turn = None     # turn number it was hidden (facedown)
        s.hidden_bf = None
        s.was_mighty = None      # last Mighty status seen at cleanup ("becomes Mighty", rule 709)

    @property
    def cname(s):
        return s.copy or s.name

    @property
    def spec(s):
        return SPEC[s.cname]

    def __repr__(s):
        return f"{s.cname}#{s.uid}"


class Rune:
    def __init__(s, domain, owner):
        Obj._n += 1
        s.uid = Obj._n
        s.domain = domain
        s.owner = owner
        s.exhausted = False

    def __repr__(s):
        return f"{s.domain[0]}{'x' if s.exhausted else ''}"


class Player:
    def __init__(s, pid, legend, champion, main, runes, battlefield):
        s.pid = pid
        s.legend_name = legend
        s.legend = Obj(legend, pid)
        s.legend.zone = "legend"
        s.deck = [Obj(n, pid) for n in main]
        for o in s.deck:
            o.zone = "deck"
        s.champ = []
        if champion:
            c = Obj(champion, pid); c.zone = "champ"; s.champ.append(c)
        s.hand, s.trash, s.banish = [], [], []
        s.rune_deck = [Rune(d, pid) for d in runes]
        s.runes = []
        s.pool_e = 0
        s.pool_p = Counter()     # domain -> power, 'A' = power of any domain
        s.points = 0
        s.xp = 0
        s.bf_name = battlefield
        s.turns = 0              # turns started by this player
        s.revealed_turn = -1     # Scuttle Crab: hand revealed to the opponent this turn
        s.gold_spent = 0


class Item:
    """A chain item: spell, activated ability or triggered ability (rules 328-340)."""
    _n = 0

    def __init__(s, kind, ctrl, name, fn=None, card=None, src=None, data=None):
        Item._n += 1
        s.id = Item._n
        s.kind = kind            # 'spell' | 'ability' | 'trigger'
        s.ctrl = ctrl
        s.name = name
        s.fn = fn                # resolver fn(g, item)
        s.card = card            # Obj for spells
        s.src = src              # uid of source object
        s.data = data or {}
        s.targets = []           # list of (uid, oid, predicate) ; predicate(g, item, obj) -> bool
        s.chosen = set()         # uids chosen (targets) -> Not So Fast / Deflect
        s.uncounterable = False

    def __repr__(s):
        return f"<{s.kind} {s.name} P{s.ctrl}>"


class Showdown:
    def __init__(s, bf, combat, attacker):
        s.bf = bf
        s.combat = combat
        s.attacker = attacker                 # player who applied Contested
        s.defender = 1 - attacker
        s.focus = attacker
        s.passes = 0
        s.stage = "open"                     # open -> damage -> result -> control -> end
        s.att_seen, s.def_seen = set(), set()   # units that already got their attack/defend trigger
        s.members = set()                    # units that have been in this combat (Mournful Witness)
        s.recalled = False
        s.assigned = None                    # (attacker's assignment, defender's assignment): {unit: damage}


class Decision:
    def __init__(s, kind, player, options):
        s.kind, s.player, s.options = kind, player, options

    def __repr__(s):
        return f"Decision({s.kind}, P{s.player}, {len(s.options)} options)"


class Battlefield:
    def __init__(s, idx, name, owner):
        s.idx, s.name, s.owner = idx, name, owner
        s.ctrl = None
        s.contested = None        # pid that applied Contested
        s.facedown = None         # Obj
        s.scored = set()          # pids that scored here this turn
        s.sd_staged = False
        s.combat_staged = False


class Game:
    def __init__(s, decks, agents, first=None, seed=None, log=False, victory=VICTORY):
        """decks: list of 2 dicts(legend, champion, main(list of 39 names), runes(list of 12 domains), battlefield)"""
        s.rng = random.Random(seed)
        s.agents = agents
        s.logging = log
        s.lines = []
        s.victory = victory
        s.p = [Player(i, d["legend"], d.get("champion"), d["main"], d["runes"], d["battlefield"]) for i, d in enumerate(decks)]
        s.bfs = [Battlefield(i, s.p[i].bf_name, i) for i in range(2)]
        if any(b.name == "Aspirant's Climb" for b in s.bfs):
            s.victory += 1
        s.board = []              # Obj on the board (units and gear)
        s.chain = []
        s.priority = None
        s.passes = 0
        s.chain_origin = None     # 'play' | 'trigger' : what opened the current chain
        s.sd = None
        s.trigq = []              # triggered abilities waiting to be put on the chain (rule 383.3)
        s.need_cleanup = False
        s.winner = None
        s.turn_no = 0
        s.first = s.rng.randrange(2) if first is None else first
        s.tp = s.first            # turn player
        s.stage = "setup"
        s.extra_turns = []
        s.effects = []            # delayed / continuous effects created by spells and abilities
        s.played = [[], []]       # per player: names of cards whose play completed this turn
        s.finalized = [[], []]    # per player: names of cards finalized this turn (Legion, rule 419.4.b)
        s.spells_played = [0, 0]  # spells finalized this turn (Crumbling Sands)
        s.stats = Counter()
        s.history = []
        s._dep_guard = set()      # uids whose dependent keywords are being evaluated (recursion guard)

    # ------------------------------------------------------------------ utilities
    def log(s, *a):
        if s.logging:
            s.lines.append(f"[T{s.turn_no} P{s.tp}] " + " ".join(str(x) for x in a))

    def ask(s, pid, kind, options, **ctx):
        """Synchronous choice for a player (limited actions, targets, optional effects)."""
        if not options:
            return None
        if len(options) == 1 and kind not in ("may", "may_target", "order"):
            return options[0]
        return s.agents[pid].choose(s, pid, kind, options, ctx)

    def opp(s, pid):
        return 1 - pid

    def obj(s, uid):
        for o in s.board:
            if o.uid == uid:
                return o
        return None

    def units(s, pid=None, loc="any"):
        return [o for o in s.board if o.spec["type"] == "Unit" and (pid is None or o.ctrl == pid)
                and (loc == "any" or o.loc == loc)]

    def gear(s, pid=None):
        return [o for o in s.board if o.spec["type"] == "Gear" and (pid is None or o.ctrl == pid)]

    def at_bf(s, o):
        return o.loc in (0, 1)

    def impl(s, o_or_name):
        IMPL = (_CARDS[0] if _CARDS else _cards()).IMPL
        name = o_or_name if isinstance(o_or_name, str) else o_or_name.cname
        return IMPL.get(name)

    def has_kw(s, o, kw):
        return s.kw_value(o, kw) > 0

    def kw_value(s, o, kw):
        """Keyword value (Assault/Shield/Deflect/Hunt/Vision values add up, rules 807.2, 809.2, 814.2, 823.2):
        printed keywords, dependent keywords (Level/Empowered/Mighty/Legion... Impl.levels and Impl.kw_if,
        rule 727), grants, Effect Text of attached Equipment (Impl.equip_kw) and static grants of other objects
        (Impl.aura_kw)."""
        v = 0
        im = s.impl(o)
        if im is not None:
            v += im.kw.get(kw, 0)
            if im.kw_if or im.levels:
                v += s._dependent_kw(o, im, kw)
        for k, val, _ in o.grants:
            if k == kw:
                v += val
        for gu in o.attached:                     # Effect Text of attached Equipment (rule 434.1.c)
            go = s.obj(gu)
            ig = s.impl(go) if go is not None else None
            if ig is not None and ig.equip_kw:
                v += ig.equip_kw.get(kw, 0)
        if _cards().AURA:
            for src, ia in s.aura_sources():
                if ia.aura_kw is not None:
                    d = ia.aura_kw(s, src, o)
                    if d:
                        v += d.get(kw, 0)
        if kw == "Ganking" and o.loc in (0, 1) and s.bfs[o.loc].name == "Windswept Hillock":
            v += 1
        if kw == "Temporary" and o.token and o.name == "Reflection" and o.copy is None:
            pass
        return v

    def _dependent_kw(s, o, im, kw):
        """Dependent keywords (rule 727): Impl.kw_if = [(cond(g, o), {kw: value})], Impl.levels = [(N, {...})].
        While a condition of this object is being evaluated, its own dependent keywords count as absent
        (avoids loops such as "While I'm Mighty, I have [Shield]")."""
        if o.uid in s._dep_guard:
            return 0
        v = 0
        if im.levels:
            xp = s.p[o.ctrl].xp
            for n, d in im.levels:
                if xp >= n and d.get("kw"):
                    v += d["kw"].get(kw, 0)
        if im.kw_if:
            s._dep_guard.add(o.uid)
            try:
                for cond, d in im.kw_if:
                    if kw in d and cond(s, o):
                        v += d[kw]
            finally:
                s._dep_guard.discard(o.uid)
        return v

    def aura_sources(s):
        """Objects whose Impl grants keywords or Might to other objects (Impl.aura_kw / aura_might):
        permanents on the board, legends and battlefields, in a fixed order."""
        names = _cards().AURA
        out = []
        if not names:
            return out
        for x in s.board:
            if x.cname in names:
                out.append((x, s.impl(x)))
        for pl in s.p:
            if pl.legend_name in names:
                out.append((pl.legend, s.impl(pl.legend_name)))
        for b in s.bfs:
            if b.name in names:
                out.append((b, s.impl(b.name)))
        return out

    def level_value(s, o, im, field):
        """Value of a Level field for the highest Level reached ("instead" semantics, rule 824)."""
        best = None
        xp = s.p[o.ctrl].xp
        for n, d in im.levels:
            if xp >= n and field in d and (best is None or n >= best[0]):
                best = (n, d[field])
        return None if best is None else best[1]

    # ------------------------------------------------------------------ XP, Level, Legion, Mighty (rules 706-733)
    def level(s, pid, n):
        """[Level N]: the dependent ability is active while the player has N or more XP (rule 824)."""
        return s.p[pid].xp >= n

    def gain_xp(s, pid, n=1):
        if n <= 0:
            return
        s.p[pid].xp += n
        s.log(f"  P{pid} gains {n} XP -> {s.p[pid].xp}")
        s.emit("xp_gain", pid=pid, n=n)

    def spend_xp(s, pid, n):
        """Spend XP (rule 730.2). Returns False (and spends nothing) if the player has less than n."""
        if s.p[pid].xp < n:
            return False
        s.p[pid].xp -= n
        s.emit("xp_spend", pid=pid, n=n)
        return True

    def legion(s, pid, card=None):
        """[Legion]: another card was finalized by pid this turn (rule 812.1.c). card: the card with Legion,
        excluded once if it has already been finalized (on the chain or on the board)."""
        n = len(s.finalized[pid])
        if card is not None and card.zone in ("chain", "board") and card.cname in s.finalized[pid]:
            n -= 1
        return n >= 1

    def in_combat(s, o):
        return s.sd is not None and s.sd.combat and o.loc == s.sd.bf and o.desig is not None

    def alone(s, o, loc=None):
        loc = o.loc if loc is None else loc
        return not any(u is not o and u.ctrl == o.ctrl and u.loc == loc for u in s.units())

    def might(s, o):
        """Current Might (rule 477.3: increases first, then decreases)."""
        base = o.spec["might"] or 0
        inc = o.buff
        dec = 0
        for amt, _ in o.mods:
            if amt > 0:
                inc += amt
            else:
                dec += amt
        for g in o.attached:
            go = s.obj(g)
            if go is not None:
                inc += EQUIP_BONUS.get(go.cname, 0)
        if o.desig == "att":
            inc += s.kw_value(o, "Assault")
        if o.desig == "def":
            inc += s.kw_value(o, "Shield")
        im = s.impl(o)
        if im is not None and im.might_mod:
            m = im.might_mod(s, o)
            if m > 0:
                inc += m
            else:
                dec += m
        if im is not None and (im.levels or im.might_if) and o.uid not in s._dep_guard:
            m = 0
            if im.levels:
                m += s.level_value(o, im, "might") or 0
            if im.might_if:
                s._dep_guard.add(o.uid)
                try:
                    m += sum(n for cond, n in im.might_if if cond(s, o))
                finally:
                    s._dep_guard.discard(o.uid)
            if m > 0:
                inc += m
            else:
                dec += m
        if _cards().AURA:
            for src, ia in s.aura_sources():
                if ia.aura_might is not None:
                    m = ia.aura_might(s, src, o)
                    if m > 0:
                        inc += m
                    elif m < 0:
                        dec += m
        # Forbidding Waste: "While a unit here is defending alone, it has -2 might."
        if o.loc in (0, 1) and s.bfs[o.loc].name == "Forbidding Waste" and o.desig == "def" and s.alone(o):
            dec -= 2
        return base + inc + dec

    def mighty(s, o):
        """Mighty (rules 706-711): Might 5 or more; printed Might outside the board."""
        if o not in s.board:
            return (o.spec["might"] or 0) >= 5
        return s.might(o) >= 5

    def targetable(s, o, by_pid, kind="spell"):
        """Untargetability (rule 757)."""
        im = s.impl(o)
        if im is not None and im.untargetable and im.untargetable(s, o, by_pid):
            return False
        return True

    def legal(s, item, i):
        """Return the i-th target of a chain item if still legal (rule 359.3.e), else None."""
        if i >= len(item.targets):
            return None
        uid, oid, pred = item.targets[i]
        o = s.obj(uid)
        if o is None or o.oid != oid:
            return None
        if not s.targetable(o, item.ctrl):
            return None
        if pred is not None and not pred(s, item, o):
            return None
        return o

    def add_target(s, item, o, pred=None):
        item.targets.append((o.uid, o.oid, pred))
        item.chosen.add(o.uid)

    def mod(s, o, amount, duration="turn", minimum=None):
        """Arithmetic effect with snapshotting (rule 477.3.b)."""
        if minimum is not None and amount < 0:
            cur = s.might(o)
            amount = -min(-amount, max(0, cur - minimum))
        if amount:
            o.mods.append([amount, duration])

    def grant(s, o, kw, value=1, duration="turn"):
        o.grants.append([kw, value, duration])

    # ------------------------------------------------------------------ zones
    def new_token(s, name, pid):
        t = Obj(name, pid, token=True)
        return t

    def remove_from_zone(s, o):
        pl = s.p[o.owner]
        for z in (pl.deck, pl.hand, pl.trash, pl.banish, pl.champ):
            if o in z:
                z.remove(o)
                return
        if o in s.board:
            s.board.remove(o)
            return
        for b in s.bfs:
            if b.facedown is o:
                b.facedown = None
                return

    def to_zone(s, o, zone, bottom=False):
        """Move a card to a non-board zone (hand, trash, banish, deck). Tokens cease to exist (rule 186.1)."""
        was_board = o in s.board
        s.remove_from_zone(o)
        if was_board:
            s.leave_board_cleanup(o)
        o.reset()
        o.oid += 1
        o.zone = zone
        if o.token:
            return
        pl = s.p[o.owner]
        if zone == "hand":
            pl.hand.append(o)
        elif zone == "trash":
            pl.trash.append(o)
        elif zone == "banish":
            pl.banish.append(o)
        elif zone == "deck":
            if bottom:
                pl.deck.append(o)
            else:
                pl.deck.insert(0, o)

    def leave_board_cleanup(s, o):
        # attached gear detaches and stays at the unit's last location (rule 719.5, 435.4.b)
        for gu in list(o.attached):
            go = s.obj(gu)
            if go is not None:
                go.attached_to = None
        o.attached = []
        if o.attached_to is not None:
            u = s.obj(o.attached_to)
            if u is not None and o.uid in u.attached:
                u.attached.remove(o.uid)
            o.attached_to = None
        s.need_cleanup = True

    def enter_board(s, o, pid, loc, ready=False):
        s.remove_from_zone(o)
        o.reset()
        o.oid += 1
        o.zone = "board"
        o.ctrl = pid
        o.loc = loc
        o.exhausted = not ready
        o.entered_turn = s.turn_no
        s.board.append(o)
        s.need_cleanup = True
        if o.spec["type"] == "Unit" and loc in (0, 1):
            b = s.bfs[loc]
            # rule 323.11.a (applied at cleanup) — units at a battlefield their controller doesn't control
        return o

    def draw(s, pid, n=1):
        pl = s.p[pid]
        for _ in range(n):
            if not pl.deck:
                s.burn_out(pid)
                if s.winner is not None:
                    return
                if not pl.deck:
                    continue
            c = pl.deck.pop(0)
            c.zone = "hand"
            pl.hand.append(c)
            if s.stage != "setup":
                s.emit("draw", pid=pid, card=c)

    def discard(s, pid, c):
        """Discard (rule 422): a card from pid's hand to the trash; emits 'discard' (pid, card)."""
        if c not in s.p[pid].hand:
            return False
        s.to_zone(c, "trash")
        s.log(f"  P{pid} discards {c}")
        s.emit("discard", pid=pid, card=c)
        return True

    def burn_out(s, pid):
        """Rule 431.2: recycle trash into deck, an opponent gains 1 point, then continue."""
        pl = s.p[pid]
        s.log("BURN OUT", pid)
        s.stats["burnout"] += 1
        tr = pl.trash[:]
        pl.trash.clear()
        s.rng.shuffle(tr)
        for c in tr:
            c.zone = "deck"
        pl.deck.extend(tr)
        s.gain_point(1 - pid, "burn out")
        if s.p[1 - pid].points >= s.victory and s.p[1 - pid].points > pl.points:
            s.win(1 - pid)

    def recycle_cards(s, pid, cards):
        cards = list(cards)
        s.rng.shuffle(cards)
        for c in cards:
            s.to_zone(c, "deck", bottom=True)
        if cards and s.stage != "setup":
            s.emit("recycle", pid=pid, cards=cards)

    def burn(s, pid, n=1):
        """Burn N (rule 440): top cards of the Main Deck to the trash, burning out if the deck runs out
        (rule 440.4). Returns the burned cards; emits 'burn' (pid, cards)."""
        pl = s.p[pid]
        burned = []
        for _ in range(n):
            if not pl.deck:
                s.burn_out(pid)
                if s.winner is not None:
                    return burned
                if not pl.deck:
                    continue
            c = pl.deck.pop(0)
            c.zone = "trash"
            pl.trash.append(c)
            burned.append(c)
        if burned:
            s.log(f"  P{pid} burns {burned}")
            s.emit("burn", pid=pid, cards=burned)
        return burned

    def predict(s, pid, n=1):
        """Predict N (rule 436): look at the top N cards, recycle any of them, put the rest back on top in any
        order. Never burns out (436.4.a). Choices: 'predict_recycle' [False, True] per card (ctx card=),
        then 'predict_top' to pick the next card to put on top. Returns the recycled cards."""
        pl = s.p[pid]
        top = pl.deck[:n]
        if not top:
            return []
        keep, rec = [], []
        for c in top:
            if s.ask(pid, "predict_recycle", [False, True], card=c, n=len(top)):
                rec.append(c)
            else:
                keep.append(c)
        del pl.deck[:len(top)]
        order = []
        while keep:
            c = s.ask(pid, "predict_top", list(keep)) if len(keep) > 1 else keep[0]
            keep.remove(c)
            order.append(c)
        pl.deck[0:0] = order
        s.log(f"  P{pid} predicts {len(top)}: recycles {len(rec)}")
        if rec:
            s.recycle_cards(pid, rec)
        return rec

    def add_pool(s, pid, e=0, power=()):
        """[Add] resources to the Rune Pool (rule 429). power: iterable of domains, 'A' = any domain."""
        pl = s.p[pid]
        pl.pool_e += e
        for d in power:
            pl.pool_p[d] += 1
        if e or power:
            s.log(f"  P{pid} adds {e} energy {list(power)}")

    def gain_point(s, pid, why):
        s.p[pid].points += 1
        s.log(f"P{pid} +1 point ({why}) -> {s.p[pid].points}")
        s.emit("point", pid=pid, why=why)

    def win(s, pid):
        if s.winner is None:
            s.winner = pid
            s.log("WINNER", pid)
        raise GameOver()

    # ------------------------------------------------------------------ runes and costs
    def channel(s, pid, n, exhausted=False):
        """Channel n runes (rule 430); emits 'channel' (pid, runes) when at least one was channeled."""
        pl = s.p[pid]
        got = []
        for _ in range(n):
            if not pl.rune_deck:
                break
            r = pl.rune_deck.pop(0)
            r.exhausted = exhausted
            pl.runes.append(r)
            got.append(r)
        if got and s.stage != "setup":
            s.emit("channel", pid=pid, runes=got)
        return got

    def recycle_rune(s, pid, r):
        pl = s.p[pid]
        pl.runes.remove(r)
        r.exhausted = False
        pl.rune_deck.append(r)

    def golds(s, pid):
        return [o for o in s.board if o.ctrl == pid and o.cname in ("Gold", "Gold // Buff") and not o.exhausted]

    def adders(s, pid, ctx=None):
        """[Add] abilities usable while paying (rule 429.3): Impl.add = [dict(e=, p=, exhaust=, kill=, can=)]
        on permanents the player controls and on their legend. ctx describes what is being paid
        (dict(kind='spell'|'unit'|'gear'|'ability'|'hide', card=, obj=)) or is None."""
        names = _cards().ADDERS
        if not names:
            return []
        srcs = [o for o in s.board if o.ctrl == pid and o.cname in names]
        if s.p[pid].legend_name in names:
            srcs.append(s.p[pid].legend)
        out = []
        for o in srcs:
            im = s.impl(o) if o is not s.p[pid].legend else s.impl(s.p[pid].legend_name)
            for ad in im.add:
                if ad.get("exhaust", True) and o.exhausted:
                    continue
                if ad.get("can") is not None and not ad["can"](s, pid, o, ctx):
                    continue
                out.append((o, ad))
        return out

    def plan_payment(s, pid, e, reqs, ctx=None):
        """Find a way to pay e energy and the power requirements reqs (list of allowed-domain sets)
        with the rune pool, runes (exhaust: Add 1 energy, recycle: Add 1 power of its domain), Gold
        tokens and other [Add] abilities (Impl.add). Returns a plan or None."""
        pl = s.p[pid]
        pool_e = pl.pool_e
        pool_p = Counter(pl.pool_p)
        ready = [r for r in pl.runes if not r.exhausted]
        exh = [r for r in pl.runes if r.exhausted]
        golds = s.golds(pid)
        recycle, use_gold = [], 0
        adders = s.adders(pid, ctx)
        soft = [a for a in adders if not a[1].get("kill")]       # Seals, Conduits...: exhaust only
        hard = [a for a in adders if a[1].get("kill")]           # "Kill this, exhaust: Add ..."
        used, used_ids = [], set()
        extra = [0]

        def use(a):
            used.append(a)
            used_ids.add(id(a[0]))
            extra[0] += a[1].get("e", 0)
            for d in a[1].get("p", ()):
                pool_p[d] += 1

        def find(req, lst, energy=False):
            for a in lst:
                if id(a[0]) in used_ids:
                    continue
                if energy and a[1].get("e", 0) > 0:
                    return a
                if not energy and any(d == "A" or d in req for d in a[1].get("p", ())):
                    return a
            return None

        def from_pool(req):
            if not pool_p:
                return False
            dom = next((d for d in sorted(req) if pool_p[d] > 0), None)
            if dom:
                pool_p[dom] -= 1
                return True
            if pool_p["A"] > 0:
                pool_p["A"] -= 1
                return True
            return False
        # domain demand of the rest of the hand, to keep the scarcer domain
        need = Counter()
        for c in pl.hand:
            for d in c.spec["domains"]:
                need[d] += c.spec["p"]
        for req in sorted(reqs, key=lambda q: len(q)):
            # pool first
            if from_pool(req):
                continue
            a = find(req, soft)
            if a is not None:
                use(a)
                from_pool(req)
                continue
            cands = [r for r in exh if r.domain in req and r not in recycle]
            if not cands:
                cands = [r for r in ready if r.domain in req and r not in recycle]
            if cands:
                cnt = Counter(r.domain for r in pl.runes)
                cands.sort(key=lambda r: (need[r.domain] - cnt[r.domain], r.domain))
                recycle.append(cands[0])
                continue
            if use_gold < len(golds):
                use_gold += 1
                continue
            a = find(req, hard)
            if a is not None:
                use(a)
                from_pool(req)
                continue
            return None
        # energy
        exhaust = []
        e_left = e
        avail = pool_e + extra[0]
        take = min(avail, e_left)
        avail -= take
        e_left -= take
        rec_ready = [r for r in recycle if not r.exhausted]
        for r in rec_ready:
            if e_left <= 0:
                break
            exhaust.append(r); e_left -= 1
        for lst in (soft, None, hard):
            if lst is None:
                others = [r for r in ready if r not in recycle]
                cnt = Counter(r.domain for r in pl.runes)
                others.sort(key=lambda r: (need[r.domain] - cnt[r.domain], r.domain))
                for r in others:
                    if e_left <= 0:
                        break
                    exhaust.append(r); e_left -= 1
                continue
            while e_left > 0:
                a = find(None, lst, energy=True)
                if a is None:
                    break
                before = extra[0]
                use(a)
                avail += extra[0] - before
                t = min(avail, e_left)
                avail -= t
                e_left -= t
        if e_left > 0:
            return None
        return dict(e=e, pool_e_after=avail, pool_p=pool_p, exhaust=exhaust, recycle=recycle, gold=use_gold,
                    adders=used)

    def can_pay(s, pid, e, reqs, ctx=None):
        return s.plan_payment(pid, e, reqs, ctx) is not None

    def pay(s, pid, e, reqs, ctx=None):
        plan = s.plan_payment(pid, e, reqs, ctx)
        if plan is None:
            return False
        pl = s.p[pid]
        pl.pool_e = plan["pool_e_after"]
        pl.pool_p = +plan["pool_p"]
        for o, ad in plan["adders"]:
            # [Add] abilities resolve immediately (rule 429.3.a); their resources are already in the plan
            if ad.get("exhaust", True):
                o.exhausted = True
            s.log(f"  P{pid} uses {o.cname}: Add")
            if ad.get("kill"):
                s.kill([o], pid, cost=True)
        for r in plan["exhaust"]:
            r.exhausted = True
        for r in plan["recycle"]:
            if not r.exhausted:
                # Une rune prête qu'on recycle est d'abord épuisée : son énergie reste dans la réserve (énergie
                # « flottante ») jusqu'à la fin du tour, comme le font les joueurs.
                r.exhausted = True
                pl.pool_e += 1
            s.recycle_rune(pid, r)
        for gobj in s.golds(pid)[:plan["gold"]]:
            # Gold: "[Reaction] Kill this, [E]: Add [A]" (Add abilities resolve immediately, rule 429.3.a)
            gobj.exhausted = True
            s.kill([gobj], pid, cost=True)
            pl.gold_spent += 1
        return True

    def power_reqs(s, card_domains, n):
        doms = frozenset(card_domains) if card_domains else ANY
        return [doms] * n

    # ------------------------------------------------------------------ killing, damage, movement
    def deal(s, target, amount, src_kind="spell", src_pid=None):
        """Deal damage from a spell or ability (rule 417). Void Gate adds 1 Bonus Damage (rule 712)."""
        if target is None or target not in s.board or amount <= 0:
            return 0
        if src_kind in ("spell", "ability") and target.loc in (0, 1) and s.bfs[target.loc].name == "Void Gate":
            amount += 1
        if target.prevent:
            p = min(target.prevent, amount)
            target.prevent -= p
            amount -= p
        if amount <= 0:
            return 0
        target.damage += amount
        s.need_cleanup = True
        s.log(f"  {target} takes {amount} ({s.might(target)} might)")
        s.emit("damaged", obj=target, amount=amount, kind=src_kind, by=src_pid)
        return amount

    def lethal(s, o):
        m = s.might(o)
        return o.damage > 0 and o.damage >= m

    def kill(s, objs, by_pid=None, cost=False):
        """Kill permanents (rule 428), with replacement effects (Zhonya's Hourglass) and death triggers.
        Returns the list of objects actually killed."""
        objs = [o for o in objs if o is not None and o in s.board]
        if not objs:
            return []
        # replacement: Zhonya's Hourglass (errata text) "If a friendly unit would die, kill this instead.
        # Heal that unit, exhaust it, and recall it." Each Hourglass applies to one event (rule 370.2, 373).
        saved = []
        units = [o for o in objs if o.spec["type"] == "Unit"]
        for pid in (s.tp, 1 - s.tp):
            mine = [u for u in units if u.ctrl == pid]
            hours = [g for g in s.board if g.ctrl == pid and g.cname == "Zhonya's Hourglass" and g not in objs]
            while mine and hours:
                u = s.ask(pid, "zhonya_save", mine) if len(mine) > 1 else mine[0]
                h = hours.pop(0)
                mine.remove(u)
                saved.append(u)
                s.log(f"  Zhonya's Hourglass saves {u}")
                s.stats[f"zhonya_save_P{pid}"] += 1
                s.kill([h], pid)
                u.damage = 0
                u.exhausted = True
                s.recall(u)
        objs = [o for o in objs if o not in saved]
        if not objs:
            return []
        # lookback info before leaving the board (rules 359.3.e.13, 808.1.d.3)
        infos = []
        for o in objs:
            infos.append(dict(obj=o, name=o.cname, ctrl=o.ctrl, loc=o.loc, might=s.might(o),
                              alone=s.alone(o), token=o.token, spec=o.spec, empowered=o.empowered,
                              in_combat=s.in_combat(o), attached=list(o.attached),
                              cost=(o.spec["e"], o.spec["p"]) if not o.token or o.copy else (0, 0)))
        for o in objs:
            s.log(f"  {o} dies")
            if o in s.board:
                s.board.remove(o)
            s.leave_board_cleanup(o)
        for o in objs:
            s.to_zone(o, "trash")
        # death triggers (Deathknell, rule 808). Karthus: "Your Deathknell effects trigger an additional
        # time" — counted for Karthus still on the board after the deaths (rule 383.2.c.2).
        for inf in infos:
            im = s.impl(inf["name"])
            if im is not None and im.deathknell:
                k = 1 + sum(1 for u in s.units(inf["ctrl"]) if u.cname == "Karthus, Eternal")
                for _ in range(k):
                    s.queue_trigger(inf["ctrl"], f"Deathknell {inf['name']}", im.deathknell, dict(info=inf),
                                    src=None, choose=im.dk_choose)
            s.emit("die", info=inf)
        return objs

    def recall(s, o):
        """Recall (rule 455): to base, not a move."""
        o.loc = "base"
        o.desig = None
        for gu in o.attached:
            go = s.obj(gu)
            if go is not None:
                go.loc = "base"
        s.need_cleanup = True

    def move(s, objs, dest, by_pid, standard=False):
        """Move units (rule 445). dest: 'base' or battlefield index."""
        moved = []
        for o in objs:
            if o not in s.board or o.loc == dest:
                continue
            frm = o.loc
            o.loc = dest
            for gu in o.attached:
                go = s.obj(gu)
                if go is not None:
                    go.loc = dest
            moved.append((o, frm))
            s.log(f"  {o} moves {frm}->{dest}")
        if dest in (0, 1) and moved:
            b = s.bfs[dest]
            mover = moved[0][0].ctrl
            if b.contested is None and b.ctrl != mover:
                b.contested = mover          # rule 450
        for o, frm in moved:
            s.emit("move", obj=o, frm=frm, to=dest, by=by_pid)
        s.need_cleanup = True
        return moved

    def stun(s, o, by_pid):
        if o is None or o not in s.board or o.spec["type"] != "Unit" or o.stunned:
            return
        o.stunned = True
        s.log(f"  {o} is stunned")
        s.emit("stun", obj=o, by=by_pid)

    def buff(s, o):
        """Buff (rule 426): +1 Might counter if the unit has none. Returns True if it was buffed."""
        if o is None or o not in s.board:
            return False
        if o.buff == 0:
            o.buff = 1
            s.emit("buff", obj=o)
            return True
        return False

    def empower(s, o):
        """Empower (rule 441): returns True if o became Empowered (emits 'empowered' obj=)."""
        if o is None or o.empowered:
            return False
        o.empowered = True
        s.log(f"  {o} is empowered")
        s.emit("empowered", obj=o)
        return True

    def disempower(s, o):
        if o is not None and o.empowered:
            o.empowered = False
            return True
        return False

    def ready_obj(s, o):
        if o.exhausted:
            o.exhausted = False
            s.emit("ready", obj=o)

    # ------------------------------------------------------------------ triggers and events
    def queue_trigger(s, pid, name, fn, data=None, src=None, may=False, choose=None, cost=None):
        """A triggered ability waiting to be finalized onto the chain.
        choose(g, item) -> bool : makes the targets/choices at finalization; False removes it (rule 402.4).
        cost(g, item) -> bool : cost within instructions paid at finalization (rule 383.3.b)."""
        it = Item("trigger", pid, name, fn=fn, src=src, data=dict(data or {}))
        it.data["_may"] = may
        it.data["_choose"] = choose
        it.data["_cost"] = cost
        s.trigq.append(it)

    def flush_triggers(s):
        q, s.trigq = s.trigq, []
        if not s.chain and s.chain_origin is None:
            s.chain_origin = "trigger"
        for pid in (s.tp, 1 - s.tp):        # rule 383.3.d.1
            mine = [t for t in q if t.ctrl == pid]
            for it in mine:
                if it.data.get("_may") and not s.ask(pid, "may", [True, False], item=it):
                    continue
                ch = it.data.get("_choose")
                if ch is not None and not ch(s, it):
                    continue
                c = it.data.get("_cost")
                if c is not None and not c(s, it):
                    continue
                s.chain.append(it)
                s.log(f"  trigger on chain: {it.name}")
                s.on_finalize(it)
        if s.chain:
            s.priority = s.chain[-1].ctrl
            s.passes = 0
        s.need_cleanup = True

    def on_finalize(s, item):
        """Targeting effects (Irelia: 'When you choose me')."""
        for uid in item.chosen:
            o = s.obj(uid)
            if o is not None:
                s.emit("chosen", obj=o, item=item)

    def emit(s, ev, **info):
        """Dispatch an event to every permanent, battlefield and active effect that listens to it."""
        if ev == "beginning_start":
            # Temporary (rule 816): "At the start of this permanent's controller's Beginning Phase, before
            # scoring, kill this." LeBlanc, Everywhere At Once: "Your Temporary effects at my battlefield
            # don't trigger."
            for o in list(s.board):
                if o.ctrl == info["pid"] and s.has_kw(o, "Temporary"):
                    if o.loc in (0, 1) and any(u.cname == "LeBlanc, Everywhere At Once" and u.ctrl == o.ctrl
                                               and u.loc == o.loc for u in s.units()):
                        continue
                    s.queue_trigger(o.ctrl, f"Temporary {o}", _temporary_kill, dict(uid=o.uid, oid=o.oid), src=o.uid)
        for o in list(s.board):
            im = s.impl(o)
            if im is not None and im.on_event is not None and o in s.board:
                im.on_event(s, o, ev, info)
            for gu in list(o.attached):          # Effect Text of attached Equipment (rule 718.3)
                go = s.obj(gu)
                ig = s.impl(go) if go is not None else None
                if ig is not None and ig.effect_event is not None:
                    ig.effect_event(s, go, o, ev, info)
        for b in s.bfs:
            im = s.impl(b.name)
            if im is not None and im.bf_event is not None:
                im.bf_event(s, b, ev, info)
        for pid in (0, 1):
            im = s.impl(s.p[pid].legend_name)
            if im is not None and im.legend_event is not None:
                im.legend_event(s, pid, ev, info)
        for eff in list(s.effects):
            if eff in s.effects and eff.get("on") == ev:
                eff["fn"](s, eff, info)

    # ------------------------------------------------------------------ main loop
    def advance(s):
        try:
            return s._advance()
        except GameOver:
            return None

    def _advance(s):
        while True:
            if s.winner is not None:
                return None
            if s.trigq:
                s.flush_triggers()
                continue
            if s.need_cleanup:
                s.cleanup()
                continue
            if s.chain:
                if s.passes >= 2:
                    s.resolve_top()
                    continue
                return Decision("priority", s.priority, s.options_priority(s.priority))
            if s.chain_origin is not None:
                s.chain_closed()
                continue
            if s.sd is not None:
                sd = s.sd
                if sd.stage == "open":
                    if sd.passes >= 2:
                        s.end_showdown()
                        continue
                    return Decision("focus", sd.focus, s.options_focus(sd.focus))
                s.combat_step()
                continue
            if s.stage == "main":
                return Decision("main", s.tp, s.options_main(s.tp))
            s.turn_step()

    def apply(s, action):
        try:
            s._apply(action)
        except GameOver:
            pass

    def _apply(s, a):
        kind = a[0]
        if s.chain:                                  # priority decision
            if kind == "pass":
                s.passes += 1
                s.priority = 1 - s.priority
                return
            s.do_action(a)
            return
        if s.sd is not None and s.sd.stage == "open":
            if kind == "pass":
                s.sd.passes += 1
                s.sd.focus = 1 - s.sd.focus
                return
            s.sd.passes = 0
            s.do_action(a)
            return
        # main phase, Neutral Open
        if kind == "end":
            s.stage = "end_step"
            s.need_cleanup = True
            return
        s.do_action(a)

    def chain_closed(s):
        """Rules 340.2, 346: when the chain empties during a showdown, Focus passes unless the chain was
        opened by a triggered ability or an Add ability."""
        origin, s.chain_origin = s.chain_origin, None
        s.priority = None
        if s.sd is not None and s.sd.stage == "open":
            if origin == "play":
                s.sd.focus = 1 - s.sd.focus
            s.sd.passes = 0
        s.need_cleanup = True

    def resolve_top(s):
        it = s.chain.pop()
        s.passes = 0
        s.log(f"resolve {it.name}")
        if it.kind == "spell":
            im = s.impl(it.card)
            im.resolve(s, it)
            if it.card.zone == "chain":
                dest = it.data.get("after", "trash")
                s.card_leaves_chain(it.card, dest)
            s.played_event(it.ctrl, it.card, it)
        else:
            it.fn(s, it)
        s.need_cleanup = True
        if s.chain:
            s.priority = s.chain[-1].ctrl

    def card_leaves_chain(s, c, dest):
        c.zone = None
        if dest == "banish":
            c.zone = "banish"; s.p[c.owner].banish.append(c)
        else:
            c.zone = "trash"; s.p[c.owner].trash.append(c)
        c.oid += 1

    def counter(s, it):
        """Rule 425: a countered item does nothing and is cleared from the chain (cards to the trash)."""
        if it not in s.chain or it.uncounterable:
            return False
        s.chain.remove(it)
        s.log(f"  {it.name} is countered")
        s.stats[f"countered_{it.name}"] += 1
        if it.kind == "spell":
            s.card_leaves_chain(it.card, it.data.get("after", "trash") if it.data.get("flow") else "trash")
        s.need_cleanup = True
        return True

    def played_event(s, pid, card, item=None):
        """A card's play completed (rule 419.4.a)."""
        s.played[pid].append(card.cname)
        s.emit("played", pid=pid, card=card, item=item, n=len(s.played[pid]))

    # ------------------------------------------------------------------ cleanup (rule 323)
    def cleanup(s):
        s.need_cleanup = False
        # 1. victory
        for pid in (s.tp, 1 - s.tp):
            if s.p[pid].points >= s.victory and s.p[pid].points > s.p[1 - pid].points:
                s.win(pid)
        # 2. combat designations
        sd = s.sd
        if sd is not None and sd.combat and sd.stage in ("open", "damage"):
            for u in s.units():
                if u.loc == sd.bf:
                    want = "att" if u.ctrl == sd.attacker else "def"
                    if u.desig != want:
                        u.desig = want
                    s.combat_trigger_check(u)
                elif u.desig is not None:
                    u.desig = None
        # 3b. lethal damage
        dead = [u for u in s.units() if s.lethal(u)]
        if dead:
            s.kill(dead)
        # "becomes Mighty" (rule 709), only tracked when a modelled card listens to it (Impl.track_mighty)
        if _cards().TRACK_MIGHTY:
            for u in s.units():
                m = s.might(u) >= 5
                if u.was_mighty is False and m:
                    s.emit("becomes_mighty", obj=u)
                u.was_mighty = m
        # 4. control loss
        open_state = not s.chain
        for b in s.bfs:
            if b.ctrl is not None and not any(u.ctrl == b.ctrl for u in s.units(loc=b.idx)):
                if open_state and not (sd is not None and sd.bf == b.idx):
                    s.log(f"  P{b.ctrl} loses control of {b.name}")
                    b.ctrl = None
        # 5. recall unattached gear at battlefields; trash hidden cards at battlefields not controlled by owner
        for g in s.gear():
            if g.loc in (0, 1) and g.attached_to is None:
                g.loc = "base"
        for b in s.bfs:
            if b.facedown is not None and b.ctrl != b.facedown.owner:
                c = b.facedown
                b.facedown = None
                s.log(f"  hidden {c} at {b.name} is trashed")
                c.zone = None
                c.reset(); c.oid += 1; c.zone = "trash"; s.p[c.owner].trash.append(c)
        # 6/7. staging
        for b in s.bfs:
            here = s.units(loc=b.idx)
            ctrls = set(u.ctrl for u in here)
            b.sd_staged = b.contested is not None and any(u.ctrl == b.contested for u in here)
            b.combat_staged = b.contested is not None and len(ctrls) == 2
        # 8. remove contested
        for b in s.bfs:
            if b.contested is not None and not any(u.ctrl == b.contested for u in s.units(loc=b.idx)) \
                    and not (sd is not None and sd.bf == b.idx):
                b.contested = None
                b.sd_staged = b.combat_staged = False
            if b.contested is None and not (sd is not None and sd.bf == b.idx):
                for u in s.units(loc=b.idx):
                    if u.ctrl != b.ctrl:
                        b.contested = u.ctrl          # 8a
                        b.sd_staged = True
                        b.combat_staged = len(set(x.ctrl for x in s.units(loc=b.idx))) == 2
                        s.need_cleanup = True
                        break
        if s.need_cleanup:
            return
        # 9/10. open a showdown or combat in a Neutral Open state
        if sd is None and not s.chain and not s.trigq:
            staged_c = [b for b in s.bfs if b.combat_staged]
            staged_s = [b for b in s.bfs if b.sd_staged and not b.combat_staged]
            if staged_c:
                s.start_showdown(staged_c[0], combat=True)
            elif staged_s:
                s.start_showdown(staged_s[0], combat=False)
        elif sd is not None and sd.stage == "open" and not sd.combat and s.bfs[sd.bf].combat_staged and not s.chain:
            s.log("  showdown becomes a combat")           # 10a
            sd.combat = True
            sd.attacker = s.bfs[sd.bf].contested
            sd.defender = 1 - sd.attacker
            s.begin_combat(sd)

    def combat_trigger_check(s, u):
        sd = s.sd
        if u.desig == "att" and u.uid not in sd.att_seen:
            sd.att_seen.add(u.uid); sd.members.add(u.uid)
            s.emit("attack", obj=u, bf=sd.bf)
        if u.desig == "def" and u.uid not in sd.def_seen:
            sd.def_seen.add(u.uid); sd.members.add(u.uid)
            s.emit("defend", obj=u, bf=sd.bf)

    def start_showdown(s, b, combat):
        b.sd_staged = b.combat_staged = False
        sd = Showdown(b.idx, combat, b.contested)
        s.sd = sd
        s.log(f"{'COMBAT' if combat else 'showdown'} at {b.name}, attacker P{sd.attacker}")
        s.stats["combats" if combat else "showdowns"] += 1
        s.emit("showdown_start", bf=b.idx, combat=combat, sd=sd)
        if combat:
            s.begin_combat(sd)

    def begin_combat(s, sd):
        """Rule 464.2: start of combat effects, designations, attack/defend triggers; attacker has Focus."""
        s.emit("combat_start", bf=sd.bf, sd=sd)
        for u in s.units(loc=sd.bf):
            u.desig = "att" if u.ctrl == sd.attacker else "def"
        for u in sorted(s.units(loc=sd.bf), key=lambda x: x.ctrl != sd.attacker):
            s.combat_trigger_check(u)
        sd.focus = sd.attacker
        sd.passes = 0

    def end_showdown(s):
        sd = s.sd
        if sd.combat:
            sd.stage = "damage"
            return
        # rule 348.2: non-combat showdown
        b = s.bfs[sd.bf]
        here = s.units(loc=b.idx)
        ctrls = set(u.ctrl for u in here)
        s.sd = None
        b.contested = None
        if len(ctrls) == 1:
            pid = ctrls.pop()
            if b.ctrl != pid:
                s.establish_control(pid, b)
        s.need_cleanup = True

    def combat_step(s):
        sd = s.sd
        b = s.bfs[sd.bf]
        if sd.stage == "damage":
            atk = [u for u in s.units(loc=b.idx) if u.desig == "att"]
            dfn = [u for u in s.units(loc=b.idx) if u.desig == "def"]
            if atk and dfn:
                s.combat_damage(sd, atk, dfn)
            # combat cleanup (rule 466.1): kill, heal all units, recall attackers if defenders remain
            dead = [u for u in s.units() if s.lethal(u)]
            if dead:
                s.kill(dead)
            for u in s.units():
                u.damage = 0
            atk = [u for u in s.units(loc=b.idx) if u.desig == "att"]
            dfn = [u for u in s.units(loc=b.idx) if u.desig == "def"]
            if atk and dfn:
                for u in atk:
                    s.recall(u)
                sd.recalled = True
            sd.stage = "result"
            s.need_cleanup = True
            return
        if sd.stage == "result":
            pids = set(u.ctrl for u in s.units(loc=b.idx))
            sd.result = None if sd.recalled or len(pids) != 1 else pids.copy().pop()
            if sd.result is not None:
                s.emit("combat_won", pid=sd.result, bf=sd.bf)
            sd.stage = "control"
            return
        if sd.stage == "control":
            here = s.units(loc=b.idx)
            pids = set(u.ctrl for u in here)
            b.contested = None
            if not pids:
                b.ctrl = None
            elif len(pids) == 1:
                pid = pids.pop()
                if b.ctrl != pid:
                    s.establish_control(pid, b)
            if b.facedown is not None and b.facedown.owner != b.ctrl:
                c = b.facedown; b.facedown = None
                c.reset(); c.oid += 1; c.zone = "trash"; s.p[c.owner].trash.append(c)
            sd.stage = "end"
            s.need_cleanup = True
            return
        if sd.stage == "end":
            members = [s.obj(u) for u in sd.members]
            for u in s.units():
                u.desig = None
            s.sd = None
            s.emit("combat_end", bf=sd.bf, members=[m for m in members if m is not None])
            s.need_cleanup = True

    def combat_damage(s, sd, atk, dfn):
        """Rule 465: sum Might of non-stunned units, attacker assigns first, then dealt simultaneously."""
        a_total = sum(max(0, s.might(u)) for u in atk if not u.stunned)
        d_total = sum(max(0, s.might(u)) for u in dfn if not u.stunned)
        s.log(f"  combat damage: attackers {a_total} vs defenders {d_total}")
        assign_a = s.assign_damage(sd.attacker, a_total, dfn)
        assign_d = s.assign_damage(sd.defender, d_total, atk)
        dealt = []
        sd.assigned = (assign_a, assign_d)       # excess damage checks ("if you assigned 3 or more excess damage")
        for u, n in list(assign_a.items()) + list(assign_d.items()):
            if n <= 0 or u not in s.board:
                continue
            if u.prevent:
                p = min(u.prevent, n); u.prevent -= p; n -= p
            if n > 0:
                u.damage += n
                dealt.append((u, n))
        for u, n in dealt:
            s.emit("damaged", obj=u, amount=n, kind="combat", by=1 - u.ctrl)
        s.need_cleanup = True

    def lethal_need(s, u):
        return max(1, s.might(u) - u.damage) + u.prevent

    def assign_damage(s, pid, total, targets):
        """Legal assignment orders (Tank first, Backline last, lethal in full before the next unit,
        excess only on the last unit). The assigning player chooses the order of units within a tier."""
        out = {}
        if total <= 0 or not targets:
            return out
        tiers = [[u for u in targets if s.has_kw(u, "Tank")],
                 [u for u in targets if not s.has_kw(u, "Tank") and not s.has_kw(u, "Backline")],
                 [u for u in targets if s.has_kw(u, "Backline") and not s.has_kw(u, "Tank")]]
        order = []
        for t in tiers:
            if len(t) > 1:
                t = s.ask(pid, "damage_order", [t], targets=t) or t
            order += t
        left = total
        for i, u in enumerate(order):
            if left <= 0:
                break
            need = s.lethal_need(u)
            n = left if i == len(order) - 1 else min(left, need)
            out[u] = n
            left -= n
        if left > 0 and order:
            out[order[-1]] = out.get(order[-1], 0) + left
        return out

    def establish_control(s, pid, b):
        b.ctrl = pid
        s.log(f"  P{pid} takes control of {b.name}")
        if pid not in b.scored:
            s.conquer(pid, b)

    def can_score(s, pid, b):
        im = s.impl(b.name)
        if im is not None and im.no_score and im.no_score(s, pid, b):
            return False
        return True

    def conquer(s, pid, b):
        """Rule 469.1, 471: conquer scores the battlefield (final point rule) and triggers conquer effects."""
        s.stats[f"conquer_P{pid}"] += 1
        if s.can_score(pid, b):
            b.scored.add(pid)
            pl = s.p[pid]
            if pl.points >= s.victory - 1:
                if all(pid in x.scored for x in s.bfs):
                    s.gain_point(pid, f"conquer {b.name} (final point)")
                else:
                    s.log(f"  P{pid} draws instead of the final point")
                    s.stats["final_point_draw"] += 1
                    s.draw(pid, 1)
            else:
                s.gain_point(pid, f"conquer {b.name}")
        else:
            b.scored.add(pid)
        units = [u for u in s.units(pid, b.idx)]
        s.hunt(pid, units)
        s.emit("conquer", pid=pid, bf=b.idx, units=units)

    def hunt(s, pid, units):
        """[Hunt X]: "When I conquer or hold, my controller gains X XP" (rule 823)."""
        for u in units:
            n = s.kw_value(u, "Hunt")
            if n > 0:
                s.queue_trigger(pid, f"Hunt {u}", lambda g, it: g.gain_xp(it.ctrl, it.data["n"]), dict(n=n), src=u.uid)

    def hold(s, pid, b):
        s.stats[f"hold_P{pid}"] += 1
        if s.can_score(pid, b):
            b.scored.add(pid)
            s.gain_point(pid, f"hold {b.name}")
        else:
            b.scored.add(pid)
        units = [u for u in s.units(pid, b.idx)]
        s.hunt(pid, units)
        s.emit("hold", pid=pid, bf=b.idx, units=units)

    # ------------------------------------------------------------------ turn structure (rules 315-317)
    def setup(s):
        for pl in s.p:
            s.rng.shuffle(pl.deck)
            s.rng.shuffle(pl.rune_deck)
        for pid in (s.first, 1 - s.first):
            s.draw(pid, 4)
        for pid in (s.first, 1 - s.first):
            pl = s.p[pid]
            keep_out = s.agents[pid].mulligan(s, pid) or []
            keep_out = [c for c in keep_out if c in pl.hand][:2]
            for c in keep_out:
                pl.hand.remove(c)
            s.draw(pid, len(keep_out))
            for c in keep_out:
                c.zone = None
            s.recycle_cards(pid, keep_out)
        s.tp = s.first
        s.stage = "awaken"
        s.new_turn_bookkeeping()

    def new_turn_bookkeeping(s):
        s.turn_no += 1
        s.p[s.tp].turns += 1
        s.played = [[], []]
        s.finalized = [[], []]
        s.spells_played = [0, 0]
        for b in s.bfs:
            b.scored = set()
        s.log(f"===== turn {s.turn_no}: P{s.tp} ({s.p[s.tp].legend_name}) points {s.p[0].points}-{s.p[1].points}")

    def turn_step(s):
        st = s.stage
        tp = s.tp
        pl = s.p[tp]
        if st == "setup":
            s.setup()
            return
        if st == "awaken":
            for o in s.board:
                if o.ctrl == tp:
                    s.ready_obj(o)
            for r in pl.runes:
                r.exhausted = False
            pl.legend.exhausted = False
            s.stage = "beginning"
        elif st == "beginning":
            s.emit("beginning_start", pid=tp)
            s.stage = "scoring"
        elif st == "scoring":
            for b in s.bfs:
                if b.ctrl == tp and tp not in b.scored:
                    s.hold(tp, b)
            s.stage = "channel"
        elif st == "channel":
            n = 3 if (tp != s.first and pl.turns == 1) else 2      # rule 486.7
            s.channel(tp, n)
            s.stage = "draw"
        elif st == "draw":
            s.draw(tp, 1)
            s.stage = "main_start"
        elif st == "main_start":
            for p_ in s.p:
                p_.pool_e = 0; p_.pool_p = Counter()
            s.emit("main_start", pid=tp)
            s.stage = "main"
        elif st == "end_step":
            s.emit("end_turn", pid=tp)
            s.stage = "expire"
        elif st == "expire":
            for u in s.units():
                u.damage = 0
                u.stunned = False
                u.prevent = 0
            for o in s.board:
                o.mods = [m for m in o.mods if m[1] != "turn"]
                o.grants = [g for g in o.grants if g[2] != "turn"]
            s.effects = [e for e in s.effects if e.get("dur") != "turn"]
            for p_ in s.p:
                p_.pool_e = 0; p_.pool_p = Counter()
            s.next_turn()
            return
        s.need_cleanup = True

    def next_turn(s):
        if s.turn_no >= 80:
            s.winner = -1
            raise GameOver()
        if s.extra_turns:
            s.tp = s.extra_turns.pop(0)
        else:
            s.tp = 1 - s.tp
        s.stage = "awaken"
        s.new_turn_bookkeeping()

    # ------------------------------------------------------------------ actions
    def options_main(s, pid):
        from actions import main_options
        return main_options(s, pid)

    def options_focus(s, pid):
        from actions import timed_options
        return [("pass",)] + timed_options(s, pid, closed=False)

    def options_priority(s, pid):
        from actions import timed_options
        return [("pass",)] + timed_options(s, pid, closed=True)

    def do_action(s, a):
        from actions import do_action
        do_action(s, a)

    # ------------------------------------------------------------------ cloning (AI lookahead)
    def clone(s):
        ag, lines = s.agents, s.lines
        s.agents, s.lines = None, []
        try:
            g = copy.deepcopy(s)
        finally:
            s.agents, s.lines = ag, lines
        g.logging = False
        return g


def _temporary_kill(g, it):
    o = g.obj(it.data["uid"])
    if o is not None and o.oid == it.data["oid"]:
        g.kill([o])


def play_game(decks, agents, seed=None, first=None, log=False, max_decisions=5000):
    g = Game(decks, agents, first=first, seed=seed, log=log)
    for a in agents:
        a.start(g)
    n = 0
    while True:
        d = g.advance()
        if d is None:
            break
        a = agents[d.player].decide(g, d)
        g.apply(a)
        n += 1
        if n > max_decisions:
            g.winner = -1
            break
    return g
