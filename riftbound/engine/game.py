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

ROOT = Path(__file__).resolve().parents[1]
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
        s.drawn_at = None        # draw sequence number of its last draw (Game.draw / draw_card)
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
        s.might_seen = None      # last Might seen at cleanup (Impl.might_watch thresholds)
        s.dmg_by = {}            # pid -> damage marked this turn by that player's sources (rule 142.4.c)
        s.last_dmg = None        # (pid, kind) of the last damage marked on it (kill attribution, rule 428.5.c)
        s.abs = []               # [ability dict, duration] activated abilities granted to this object
        s.xtags = []             # [tag, duration] tags granted to this object
        s.copy_src = None        # gear whose attachment makes this unit a copy (Shady Spectacles)
        s.copy_name = None       # gear: name of the unit its attached unit copies
        s.chosen_tag = None      # tag chosen as it was played (Ivern, Friend to All)

    @property
    def cname(s):
        cs = s.copy_src
        if cs is not None and cs.copy_name and cs.attached_to == s.uid and cs.zone == "board":
            return cs.copy_name                  # copy for as long as the gear stays attached to it
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
        s.pool_r = []            # [energy, only(ctx) -> bool]: energy that may only be spent on some costs
        s.points = 0
        s.xp = 0
        s.bf_name = battlefield
        s.turns = 0              # turns started by this player
        s.revealed_turn = -1     # Scuttle Crab: hand revealed to the opponent this turn
        s.seen = set()           # uids des cartes de son propre deck que le joueur a vues (Predict, Vision : 436, 817)
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
        s.excess = [0, 0]                    # excess damage each player assigned (beyond lethal, rule 465.2.c)


class Opt:
    """A labelled option of g.ask (when the option is not a card, a rune, a location or a boolean).
    label: French text shown to a human player (train._olabel); value: what the effect uses."""
    def __init__(s, label, value=None, obj=None):
        s.label, s.value, s.obj = label, value, obj

    def __repr__(s):
        return f"Opt({s.label})"


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
        s.facedowns = []          # hidden cards here (one, or more with Bandle Tree)
        s.scored = set()          # pids that scored here this turn
        s.sd_staged = False
        s.combat_staged = False
        s.replaced = None         # battlefield name this token battlefield replaced (Brush, rule 438)

    @property
    def facedown(s):
        """The first (usually only) facedown card here, or None."""
        return s.facedowns[0] if s.facedowns else None

    @facedown.setter
    def facedown(s, c):
        """Replace (or with None remove) the first facedown card, keeping the others."""
        if c is None:
            s.facedowns = s.facedowns[1:]
        elif s.facedowns:
            s.facedowns = [c] + s.facedowns[1:]
        else:
            s.facedowns = [c]


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
        s.shown_hand = None    # main révélée (424) : dict(pid, to, uids, turn), voir reveal_hand
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
        s.resolving = None        # chain item being resolved (source of the current spell/ability effects)
        s._dying = []             # permanents of the kill being replaced (Game.death_replacements)
        s.phase = None            # turn step being executed (the effects it causes happen in it): 'beginning',
        #                           'scoring' (both the Beginning Phase), 'channel', 'draw', 'main_start' (Main)...
        s.zone_listen = True      # cards outside the board may listen to events (Impl.zone_event); see emit
        s.reset_hist()

    def reset_hist(s):
        """Per-turn history of each player (what card texts ask about "this turn"), reset at each new turn."""
        s.hist = dict(spell_e=[[], []],        # energy actually paid for each spell played (rule 356)
                      drawn=[0, 0], discarded=[0, 0], xp=[0, 0], hold_pts=[0, 0], power_spent=[0, 0],
                      died=[],                  # look-back info of every permanent that died (with stage, by)
                      conquered=[],             # (uid, oid) of the units that conquered
                      conq_bf=[[], []],         # battlefields each player conquered
                      chose_enemy=[False, False],   # chose an enemy unit with a spell, ability or trigger
                      gear_abs=[0, 0],          # activated abilities of gear played
                      gear_played=[0, 0],       # non-token gear played
                      tokens=[0, 0])            # token units played

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

    def dup_no(s, o):
        """Numéro d'affichage (1, 2…) d'un objet en jeu quand plusieurs objets en jeu portent le même nom (des deux
        côtés), par ordre d'uid ; 0 s'il est seul. La table l'affiche sur la carte et dans les choix (« #2 »)."""
        if o is None or o not in s.board:
            return 0
        same = sorted(x.uid for x in s.board if x.cname == o.cname)
        return same.index(o.uid) + 1 if len(same) > 1 else 0

    def label(s, o):
        """Nom d'un objet pour les libellés de choix : « Shipyard Skulker #2 » quand il a des homonymes en jeu."""
        n = s.dup_no(o)
        return f"{o.cname} #{n}" if n else o.cname

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
            if go is not None and go.copy_name and go.cname == "Svellsongur":
                ic = s.impl(go.copy_name)        # copied text appended to the unit
                if ic is not None:
                    v += ic.kw.get(kw, 0)
                    if ic.kw_if or ic.levels:
                        v += s._dependent_kw(o, ic, kw)
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

    # ------------------------------------------------------------------ static abilities of other objects
    def providers(s, field):
        """(src, impl) of every object whose Impl defines `field` (cards.HOOKS): permanents on the board whose
        printed text is active (not attached gear, rule 434.1.e), legends and battlefields, in a fixed order.
        A unit with copied text (Svellsongur) appears once per copy of the text."""
        names = _cards().HOOKS.get(field)
        if not names:
            return []
        out = []
        for x in s.board:
            if x.attached_to is None:
                if x.cname in names:
                    out.append((x, s.impl(x)))
                if x.attached:
                    for n in s.copied_texts(x):
                        if n in names:
                            out.append((x, s.impl(n)))
        for pl in s.p:
            if pl.legend_name in names:
                out.append((pl.legend, s.impl(pl.legend_name)))
        for b in s.bfs:
            if b.name in names:
                out.append((b, s.impl(b.name)))
        return out

    def fx_providers(s, field, unit=None):
        """(gear, unit, impl) for each Equipment attached to unit (or to any unit) whose Effect Text defines
        `field` (the Effect Text is appended to the unit's text, rule 434.1.c)."""
        names = _cards().HOOKS.get(field)
        if not names:
            return []
        out = []
        for u in ([unit] if unit is not None else s.board):
            for gu in u.attached:
                go = s.obj(gu)
                if go is not None and go.cname in names:
                    out.append((go, u, s.impl(go)))
        return out

    def copied_texts(s, o):
        """Names whose text is appended to o by attached gear that copies it (Svellsongur)."""
        out = []
        for gu in o.attached:
            go = s.obj(gu)
            if go is not None and go.copy_name and go.cname == "Svellsongur":
                out.append(go.copy_name)
        return out

    def effects_of(s, kind, pid=None):
        """Continuous effects of a kind (g.effects entries with kind=), optionally for one player (pid=). An entry
        with src=(uid, oid) is a static ability of that permanent (Endless Riches): it applies only while that
        object is on the board, for its current controller."""
        out = []
        for e in s.effects:
            if e.get("kind") != kind:
                continue
            p = e.get("pid")
            if e.get("src") is not None:
                o = s.obj(e["src"][0])
                if o is None or o.oid != e["src"][1] or o not in s.board:
                    continue
                p = o.ctrl
            if pid is None or p == pid:
                out.append(e)
        return out

    def tags(s, o):
        """Tags of an object (rule 1xx: characteristics): printed tags, the tag chosen as it was played, tags
        granted by effects (xtags), by attached Equipment (Impl.equip_tags) and by auras (Impl.aura_tags)."""
        t = o.spec["tags"]
        if not (o.chosen_tag or o.xtags or o.attached or _cards().HOOKS.get("aura_tags")):
            return t
        t = set(t)
        if o.chosen_tag:
            t.add(o.chosen_tag)
        t.update(x[0] for x in o.xtags)
        for gu in o.attached:
            go = s.obj(gu)
            ig = s.impl(go) if go is not None else None
            if ig is not None and ig.equip_tags:
                t.update(ig.equip_tags)
        for src, im in s.providers("aura_tags"):
            t.update(im.aura_tags(s, src, o) or ())
        return t

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
        s.hist["xp"][pid] += n
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
        if s.effects:
            for e in s.effects:                     # "Its base Might becomes N this turn" (Dragon Form)
                if e.get("kind") == "base_might" and e["uid"] == o.uid and e["oid"] == o.oid:
                    base = e["value"]
        inc = o.buff
        if o.buff and s.effects:
            # Stand United: "Buffs give an additional +1 might to friendly units this turn."
            inc += o.buff * sum(1 for e in s.effects if e.get("kind") == "buff_bonus" and e["pid"] == o.ctrl)
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
        ims = [s.impl(o)]
        if o.attached:
            ims += [s.impl(n) for n in s.copied_texts(o)]
        for im in ims:
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
        """Untargetability (rule 757): the object's own text, an "Untargetable" grant ("can't be chosen by enemy
        spells and abilities this turn") and auras of other objects (Impl.aura_untargetable)."""
        im = s.impl(o)
        if im is not None and im.untargetable and im.untargetable(s, o, by_pid):
            return False
        if by_pid is not None and by_pid != o.ctrl:
            if o.grants and any(k == "Untargetable" for k, _, _ in o.grants):
                return False
            for src, ia in s.providers("aura_untargetable"):
                if ia.aura_untargetable(s, src, o, by_pid):
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
        """Arithmetic effect with snapshotting (rule 477.3.b). A -might given by a spell or ability that chose o
        can be replaced (Gangplank, Naval) or increased (Impl.minus_extra of other objects: Mel, Newly Awakened)."""
        if amount < 0 and s.resolving is not None and o.uid in s.resolving.chosen:
            if not s.replace_choice_effect(o, "minus", duration):
                return
            for src, im in s.providers("minus_extra"):
                amount -= im.minus_extra(s, src, s.resolving, o) or 0
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
            if o in b.facedowns:
                b.facedowns.remove(o)
                return

    def lookback(s, o):
        """Look-back information of a permanent leaving the board (rules 359.3.e.13, 808.1.d.3)."""
        return dict(obj=o, name=o.cname, ctrl=o.ctrl, loc=o.loc, might=s.might(o), alone=s.alone(o), token=o.token,
                    spec=o.spec, empowered=o.empowered, in_combat=s.in_combat(o), attached=list(o.attached),
                    buffed=o.buff > 0, stunned=o.stunned, exhausted=o.exhausted, uid=o.uid, oid=o.oid,
                    tags=set(s.tags(o)), cost=(o.spec["e"], o.spec["p"]) if not o.token or o.copy else (0, 0),
                    att_names=[s.obj(u).cname for u in o.attached if s.obj(u) is not None],
                    copied=s.copied_texts(o) if o.attached else [])

    def trash_dest(s, o, zone, frm=None):
        """Zone replacement (Endless Riches: "If a card would go to your trash from anywhere other than your Main
        Deck, banish it instead.")."""
        if zone == "trash" and frm != "deck" and s.effects and s.effects_of("trash_to_banish", o.owner):
            return "banish"
        return zone

    def to_zone(s, o, zone, bottom=False, by=None):
        """Move a card to a non-board zone (hand, trash, banish, deck). Tokens cease to exist (rule 186.1).
        From the board it emits 'leave' (obj, info, dest) and, to a hand, 'returned' (obj, loc, pid); a banished
        card emits 'banish' (pid = player who banishes: by, else the controller of the resolving item, else the
        owner, card). Returns False if a replacement effect kept it where it was."""
        was_board = o in s.board
        frm = o.zone
        zone = s.trash_dest(o, zone, frm)
        if was_board and zone == "hand" and s.resolving is not None and not s.replace_choice_effect(o, "hand"):
            return False
        info = s.lookback(o) if was_board else None
        s.remove_from_zone(o)
        if was_board:
            s.leave_board_cleanup(o)
        o.reset()
        o.oid += 1
        o.zone = zone
        if not o.token:
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
        if was_board:
            if zone == "hand":
                s.emit("returned", obj=o, loc=info["loc"], pid=o.owner, info=info)
            s.emit("leave", obj=o, info=info, dest=zone)
        if zone == "banish" and not o.token:
            if by is None:
                by = s.resolving.ctrl if s.resolving is not None else o.owner
            s.emit("banish", pid=by, card=o)
        return True

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
            s.stats["draw_seq"] += 1
            c.drawn_at = s.stats["draw_seq"]
            if s.stage != "setup":
                s.hist["drawn"][pid] += 1
                s.emit("draw", pid=pid, card=c, n=s.hist["drawn"][pid])

    def draw_card(s, pid, c):
        """Draw a given card of pid's Main Deck (an effect that says "draw it" after looking at the deck)."""
        pl = s.p[pid]
        if c not in pl.deck:
            return False
        pl.deck.remove(c)
        c.zone = "hand"
        pl.hand.append(c)
        s.stats["draw_seq"] += 1
        c.drawn_at = s.stats["draw_seq"]
        s.hist["drawn"][pid] += 1
        s.emit("draw", pid=pid, card=c, n=s.hist["drawn"][pid])
        return True

    def discard(s, pid, c):
        """Discard (rule 422): a card from pid's hand to the trash; emits 'discard' (pid, card)."""
        if c not in s.p[pid].hand:
            return False
        s.to_zone(c, "trash")
        s.log(f"  P{pid} discards {c}")
        s.hist["discarded"][pid] += 1
        im = s.impl(c)
        if im is not None and im.on_discard is not None:
            im.on_discard(s, c, pid)                # "When you discard me" (wherever the card went)
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

    def look(s, pid, cards, owner=None):
        """A player looks at cards from the top of a Main Deck without drawing them (Predict, Vision, "look at the
        top N"...): emits 'look' (pid, cards, owner) and the seen cards' own Impl.on_seen (Nocturne, Horrifying)."""
        cards = list(cards)
        if not cards:
            return cards
        owner = pid if owner is None else owner
        if owner == pid:
            s.p[pid].seen.update(c.uid for c in cards)     # la table montre le dessus connu tant qu'il reste dessus
        s.emit("look", pid=pid, cards=cards, owner=owner)
        names = _cards().HOOKS.get("on_seen")
        if names:
            for c in cards:
                if c.cname in names and c.zone == "deck":
                    s.impl(c).on_seen(s, c, pid)
        return cards

    def reveal_hand(s, pid, to):
        """« They reveal their hand » (424.3.a) : les cartes en main de pid à cet instant sont révélées. L'état dure
        jusqu'à la fin de la résolution de l'effet (424.1.a.3) ; la table le garde affiché jusqu'au coup suivant joué
        chaîne vide (le focus est passé), voir _apply. Les cartes ajoutées ensuite ne sont pas révélées (424.3.a.1)."""
        cards = list(s.p[pid].hand)
        s.shown_hand = dict(pid=pid, to=to, uids=[c.uid for c in cards], turn=s.turn_no)
        s.log(f"  P{pid} reveals their hand {cards}")
        s.emit("reveal_hand", pid=pid, cards=cards, to=to)
        return cards

    def reveal(s, pid, n=1, owner=None, look_first=True, until=None):
        """Reveal the top n cards of a Main Deck (owner's, default pid's) and return them (they stay on top; the
        caller moves them). Replacement effects: Void Hatchling ("If you would reveal cards from a deck, look at the
        top card first. You may recycle it. Then reveal those cards."), Undertitan ("As I'm revealed from your deck,
        [Add] [2]"). Emits 'reveal' (pid, cards, owner)."""
        owner = pid if owner is None else owner
        pl = s.p[owner]
        if look_first and pl.deck:
            for src, im in s.providers("reveal_rep"):
                if src.ctrl == pid and pl.deck:
                    top = pl.deck[0]
                    s.look(pid, [top], owner)
                    if top in pl.deck and s.ask(pid, "hatchling_recycle", [False, True], card=top):
                        pl.deck.remove(top)
                        s.recycle_cards(owner, [top])
        if until is not None:                      # "reveal cards ... until you reveal a ..."
            cards = []
            for c in pl.deck:
                cards.append(c)
                if until(c):
                    break
        else:
            cards = pl.deck[:n]
        for c in cards:
            im = s.impl(c)
            if im is not None and im.on_reveal is not None:
                im.on_reveal(s, c, pid)
        if cards:
            s.log(f"  P{pid} reveals {cards}")
            s.emit("reveal", pid=pid, cards=list(cards), owner=owner)
        return cards

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
        s.look(pid, top)
        top = [c for c in top if c in pl.deck[:n]]
        keep, rec = [], []
        for i, c in enumerate(top):
            if s.ask(pid, "predict_recycle", [False, True], card=c, n=len(top), i=i, top=list(top)):
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
        """Gain 1 point. "Can't score points" (Impl.score_veto: Tianna Crownguard) stops every point but the one
        from an opponent's Burn Out (not a score). Returns True if the point was gained."""
        if why != "burn out":
            for src, im in s.providers("score_veto"):
                if im.score_veto(s, src, pid, why):
                    s.log(f"  P{pid} can't score ({src.name if hasattr(src, 'name') else src})")
                    return False
        s.p[pid].points += 1
        s.log(f"P{pid} +1 point ({why}) -> {s.p[pid].points}")
        s.emit("point", pid=pid, why=why)
        return True

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
        """Recycle a rune (rule 430.3): to the bottom of the rune deck; emits 'rune_recycle' (pid, rune)."""
        pl = s.p[pid]
        pl.runes.remove(r)
        r.exhausted = False
        pl.rune_deck.append(r)
        if s.stage != "setup":
            s.emit("rune_recycle", pid=pid, rune=r)

    def golds(s, pid):
        return [o for o in s.board if o.ctrl == pid and o.cname in ("Gold", "Gold // Buff") and not o.exhausted]

    def adders(s, pid, ctx=None):
        """[Add] abilities usable while paying (rule 429.3): Impl.add = [dict(e=, p=, exhaust=, kill=, can=)]
        on permanents the player controls and on their legend. ctx describes what is being paid
        (dict(kind='spell'|'unit'|'gear'|'ability'|'hide', card=, obj=)) or is None."""
        names = _cards().ADDERS
        grants = _cards().HOOKS.get("grant_add")
        if not names and not grants:
            return []
        srcs = [o for o in s.board if o.ctrl == pid and o.cname in names]
        if s.p[pid].legend_name in names:
            srcs.append(s.p[pid].legend)
        pairs = []
        for o in srcs:
            im = s.impl(o) if o is not s.p[pid].legend else s.impl(s.p[pid].legend_name)
            pairs += [(o, ad) for ad in im.add]
        if grants:          # [Add] abilities an object has from others (Impl.grant_add(g, src): Heimerdinger)
            for o, im in s.providers("grant_add"):
                if o.ctrl == pid and getattr(o, "zone", None) == "board":
                    pairs += [(o, ad) for ad in im.grant_add(s, o) or ()]
        out = []
        for o, ad in pairs:
            if ad.get("exhaust", True) and o.exhausted:
                continue
            if ad.get("timing") == "action" and (s.chain or (s.sd is not None and s.sd.focus != pid)
                                                 or (s.sd is None and s.tp != pid)):
                continue                        # [Action] Add: only when the player could play an Action
            if ad.get("can") is not None and not ad["can"](s, pid, o, ctx):
                continue
            out.append((o, ad))
        return out

    def gold_extra(s, pid):
        """Energy a Gold token adds on top of its [A] (Impl.gold_extra(g, src, pid): Renata Glasc, Chem-Baroness)."""
        return sum(im.gold_extra(s, src, pid) or 0 for src, im in s.providers("gold_extra"))

    def plan_payment(s, pid, e, reqs, ctx=None, _skip=()):
        """Find a way to pay e energy and the power requirements reqs (list of allowed-domain sets)
        with the rune pool, runes (exhaust: Add 1 energy, recycle: Add 1 power of its domain), Gold
        tokens and other [Add] abilities (Impl.add). [Add] abilities with a cost (converters: dict(conv=...)) are
        tried only when the plain payment is impossible (plan_convert). Returns a plan or None."""
        plan = s._plan_payment(pid, e, reqs, ctx, _skip)
        if plan is None and _cards().CONVERTERS:
            plan = s.plan_convert(pid, e, reqs, ctx, _skip)
        return plan

    def plan_convert(s, pid, e, reqs, ctx, skip):
        """[Add] abilities whose cost is paid with resources or a kill (Impl.add entries with conv=):
          'p2e' n power of any type -> n energy (fixed n, or 'var' up to the need: Hextech Anomaly),
          'e2p' n energy -> n [A] (Ancient Henge, 'var'), 'kill' kill a friendly unit or gear -> outputs (Malzahar).
        The output pays part of this cost; any rest floats in the Rune Pool (restricted by `only` if given)."""
        for o, ad in s.adders(pid, ctx):
            conv = ad.get("conv")
            if conv is None or id(o) in skip:
                continue
            sk = tuple(skip) + (id(o),)
            if conv == "p2e":
                ns = range(1, e + 1) if ad.get("var") else [ad["n"]]
                for n in ns:
                    sub = s.plan_payment(pid, max(0, e - n), list(reqs) + [ANY] * n, ctx, sk)
                    if sub is not None:
                        sub.setdefault("conv", []).append(dict(o=o, ad=ad, n=n, left_e=max(0, n - e)))
                        return sub
            elif conv == "e2p":
                for n in range(1, len(reqs) + 1):
                    rest = sorted(reqs, key=len)[n:]
                    sub = s.plan_payment(pid, e + n, rest, ctx, sk)
                    if sub is not None:
                        sub.setdefault("conv", []).append(dict(o=o, ad=ad, n=n, left_e=0))
                        return sub
            elif conv == "kill":
                if not ad["victims"](s, pid, o):
                    continue
                k = len(ad.get("p", ()))
                rest = sorted(reqs, key=len)[k:]
                sub = s.plan_payment(pid, e, rest, ctx, sk)
                if sub is not None:
                    sub.setdefault("conv", []).append(dict(o=o, ad=ad, n=k, left_p=max(0, k - len(reqs))))
                    return sub
        return None

    def _plan_payment(s, pid, e, reqs, ctx=None, _skip=()):
        pl = s.p[pid]
        pool_e = pl.pool_e
        if pl.pool_r:
            pool_e += sum(n for n, only in pl.pool_r if only(s, pid, ctx))
        pool_p = Counter(pl.pool_p)
        ready = [r for r in pl.runes if not r.exhausted]
        exh = [r for r in pl.runes if r.exhausted]
        golds = s.golds(pid)
        recycle, use_gold = [], 0
        adders = [a for a in s.adders(pid, ctx) if a[1].get("conv") is None and id(a[0]) not in _skip]
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
        gx = s.gold_extra(pid) if golds else 0
        avail = pool_e + extra[0] + gx * use_gold
        take = min(avail, e_left)
        avail -= take
        e_left -= take
        took = take
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
        while e_left > 0 and gx and use_gold < len(golds):
            use_gold += 1                            # Gold used for its extra energy, its [A] floats
            pool_p["A"] += 1
            t = min(gx, e_left)
            avail += gx - t
            e_left -= t
        if e_left > 0:
            return None
        return dict(e=e, pool_e_after=avail, pool_p=pool_p, exhaust=exhaust, recycle=recycle, gold=use_gold,
                    adders=used, took=took)

    def can_pay(s, pid, e, reqs, ctx=None):
        return s.plan_payment(pid, e, reqs, ctx) is not None

    def pay(s, pid, e, reqs, ctx=None):
        plan = s.plan_payment(pid, e, reqs, ctx)
        if plan is None:
            return False
        pl = s.p[pid]
        # restricted energy is spent first when it may pay this cost (the plan counted it in its pool)
        took = plan.get("took", 0)
        used_r = 0
        if pl.pool_r:
            keep = []
            for ent in pl.pool_r:
                if took > used_r and ent[1](s, pid, ctx):
                    u = min(ent[0], took - used_r)
                    used_r += u
                    if ent[0] > u:
                        keep.append([ent[0] - u, ent[1]])
                elif ent[0] > 0:
                    keep.append(ent)
            pl.pool_r = keep
        # unrestricted pool after payment: what the plan left, minus the restricted energy still unspent
        pl.pool_e = plan["pool_e_after"] - sum(n for n, only in pl.pool_r if only(s, pid, ctx))
        pl.pool_p = +plan["pool_p"]
        s.hist["power_spent"][pid] += len(reqs)
        for cv in plan.get("conv", ()):
            o, ad = cv["o"], cv["ad"]
            if ad.get("exhaust", True):
                o.exhausted = True
            s.log(f"  P{pid} uses {o.cname}: Add ({ad['conv']} {cv['n']})")
            if ad["conv"] == "kill":
                vs = ad["victims"](s, pid, o)
                v = s.ask(pid, "add_kill", vs, src=o) if len(vs) > 1 else vs[0]
                s.kill([v], pid, cost=True)
                for _ in range(cv.get("left_p", 0)):
                    pl.pool_p["A"] += 1
            if cv.get("left_e"):
                if ad.get("only") is not None:
                    pl.pool_r.append([cv["left_e"], ad["only"]])
                else:
                    pl.pool_e += cv["left_e"]
            s.ability_used(pid, o, "Add")
        for o, ad in plan["adders"]:
            # [Add] abilities resolve immediately (rule 429.3.a); their resources are already in the plan
            if ad.get("exhaust", True):
                o.exhausted = True
            s.log(f"  P{pid} uses {o.cname}: Add")
            s.ability_used(pid, o, "Add")
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
            s.ability_used(pid, gobj, "Add")
            s.kill([gobj], pid, cost=True)
            pl.gold_spent += 1
        return True

    def ability_used(s, pid, o, name, cost_e=0, item=None):
        """An [Add] ability was used (resolves at once, rule 429.3.a): counts gear abilities this turn and emits
        'activated' (pid, obj, ab, cost_e, item). Abilities on the chain emit it as they resolve (resolve_top)."""
        if o is not None and getattr(o, "spec", None) is not None and o.spec["type"] == "Gear":
            s.hist["gear_abs"][pid] += 1
        s.emit("activated", pid=pid, obj=o, ab=name, cost_e=cost_e, item=item)

    def power_reqs(s, card_domains, n):
        doms = frozenset(card_domains) if card_domains else ANY
        return [doms] * n

    # ------------------------------------------------------------------ killing, damage, movement
    def deal(s, target, amount, src_kind="spell", src_pid=None, item=None):
        """Deal damage from a spell or ability (rule 417): Bonus Damage (rules 712-715: Void Gate, Impl.dmg_bonus of
        other objects, fx_dmg_bonus of attached Equipment, g.effects kind='dmg_bonus'), then the replacement
        effects on the target (prevention, doubling: see damage_replace). Returns the damage dealt."""
        if target is None or target not in s.board or amount <= 0:
            return 0
        if item is None:
            item = s.resolving
        if src_pid is None and item is not None:
            src_pid = item.ctrl
        ctx = dict(kind=src_kind, by=src_pid, item=item, target=target)
        if src_kind in ("spell", "ability"):
            if target.loc in (0, 1) and s.bfs[target.loc].name == "Void Gate":
                amount += 1
            amount += s.bonus_damage(ctx)
        amount = s.damage_replace(target, amount, ctx)
        if amount <= 0:
            return 0
        s.mark_damage(target, amount, src_pid, src_kind)
        s.log(f"  {target} takes {amount} ({s.might(target)} might)")
        s.emit("damaged", obj=target, amount=amount, kind=src_kind, by=src_pid)
        return amount

    def mark_damage(s, target, amount, pid, kind):
        target.damage += amount
        if pid is not None:
            target.dmg_by[pid] = target.dmg_by.get(pid, 0) + amount
            target.last_dmg = (pid, kind)
        s.need_cleanup = True

    def bonus_damage(s, ctx):
        """Bonus Damage of a spell or ability that deals damage (summed, rule 714)."""
        n = 0
        for src, im in s.providers("dmg_bonus"):
            n += im.dmg_bonus(s, src, ctx) or 0
        for go, u, im in s.fx_providers("fx_dmg_bonus"):
            n += im.fx_dmg_bonus(s, go, u, ctx) or 0
        for e in list(s.effects):
            if e.get("kind") == "dmg_bonus":
                n += e["fn"](s, e, ctx) or 0
        return n

    def damage_replace(s, target, amount, ctx):
        """Replacement effects on damage that would be dealt to target (rules 367, 437): "can't be dealt damage"
        and total prevention (Impl.dmg_prevent(g, o, ctx) of the target, Impl.aura_dmg_prevent(g, src, o, ctx),
        g.effects kind='prevent_damage' fn(g, eff, o, ctx)), "prevent the next time" (kind='prevent_next', consumed),
        the Prevent Value (Ki Barrier, o.prevent), then doubling (kind='double_damage'). The target's controller
        orders them (rule 372): every prevention is applied before doubling, which never gives more damage."""
        if amount <= 0:
            return 0
        im = s.impl(target)
        if im is not None and im.dmg_prevent is not None and im.dmg_prevent(s, target, ctx):
            return 0
        for src, ia in s.providers("aura_dmg_prevent"):
            if ia.aura_dmg_prevent(s, src, target, ctx):
                return 0
        if s.effects:
            for e in list(s.effects):
                if e.get("kind") == "prevent_damage" and e["fn"](s, e, target, ctx):
                    return 0
            for e in list(s.effects):
                if e.get("kind") == "prevent_next" and e["uid"] == target.uid and e["oid"] == target.oid:
                    s.effects.remove(e)                     # "the next time it would be dealt damage this turn"
                    s.log(f"  damage to {target} is prevented")
                    return 0
        if target.prevent:
            p = min(target.prevent, amount)
            target.prevent -= p
            amount -= p
        if amount > 0 and s.effects:
            for e in s.effects:
                if e.get("kind") == "double_damage" and e["uid"] == target.uid and e["oid"] == target.oid:
                    amount *= 2
        return amount

    def lethal(s, o):
        """Lethal damage (rule 142.4): damage >= Might, or any damage from a player whose damage is always enough
        (Impl.lethal_any: Elder Dragon, "Any amount of your damage is enough to kill enemy units")."""
        if o.damage <= 0:
            return False
        if o.damage >= s.might(o):
            return True
        if o.dmg_by:
            for src, im in s.providers("lethal_any"):
                if o.dmg_by.get(src.ctrl, 0) > 0 and im.lethal_any(s, src, o):
                    return True
        return False

    def kill(s, objs, by_pid=None, cost=False):
        """Kill permanents (rule 428), with replacement effects ("If ... would die ... instead", rules 370-373:
        Zhonya's Hourglass, Guardian Angel, Highlander, Soraka...) and death triggers.
        by_pid: player responsible (default: controller of the resolving item; lethal damage: the player whose
        source dealt the last damage, rule 428.5.c). Returns the list of objects actually killed."""
        objs = [o for o in objs if o is not None and o in s.board]
        if not objs:
            return []
        it = s.resolving
        if by_pid is None and it is not None:
            by_pid = it.ctrl
        saved = s.death_replacements(objs)
        objs = [o for o in objs if o not in saved and o in s.board]
        if not objs:
            return []
        # lookback info before leaving the board (rules 359.3.e.13, 808.1.d.3)
        infos = []
        for o in objs:
            inf = s.lookback(o)
            by, kind = by_pid, (it.kind if it is not None else None)
            if o.last_dmg is not None and s.lethal(o) and not cost:
                by, kind = o.last_dmg                 # dies of lethal damage: attributed to its source
            inf.update(by=by, by_kind=kind, stage=s.stage, phase=s.phase, tp=s.tp, turn=s.turn_no, as_cost=cost)
            infos.append(inf)
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
            s.hist["died"].append(inf)
            im = s.impl(inf["name"])
            dks = [(inf["name"], im.deathknell, im.dk_choose)] if im is not None and im.deathknell else []
            for n in inf.get("copied", ()):                 # copied text (Svellsongur): its Deathknell too
                ic = s.impl(n)
                if ic is not None and ic.deathknell:
                    dks.append((f"{n} (copy)", ic.deathknell, ic.dk_choose))
            for n in inf.get("att_names", ()):             # Deathknell in the Effect Text of attached Equipment
                ig = s.impl(n)
                if ig is not None and ig.fx_deathknell is not None:
                    dks.append((f"{inf['name']} ({n})", ig.fx_deathknell, None))
            if dks:
                k = 1 + sum(1 for u in s.units(inf["ctrl"]) if u.cname == "Karthus, Eternal")
                for name, fn, choose in dks:
                    for _ in range(k):
                        s.queue_trigger(inf["ctrl"], f"Deathknell {name}", fn, dict(info=inf), src=None,
                                        choose=choose)
            s.emit("die", info=inf)
            s.emit("leave", obj=inf["obj"], info=inf, dest=inf["obj"].zone, killed=True)
        return objs

    def death_replacements(s, objs):
        """Apply "would die ... instead" replacement effects to the permanents about to die (rules 370-373).
        Sources: Impl.death_rep(g, src, unit) of permanents/legends/battlefields, Impl.fx_death_rep(g, gear, unit)
        of attached Equipment (Effect Text), and g.effects entries kind='death_rep' with fn(g, eff, unit).
        Each returns None or a dict(name, apply(g, unit), may=False, cost=None|fn(g)->bool, key=, multi=False).
        The controller of the dying object chooses which replacement applies (rule 372); a replacement with a key
        is applied in one sequence only (rule 373.2: `multi` ones save every unit they qualify for at once).
        Returns the saved objects."""
        hooks = _cards().HOOKS
        if not (hooks.get("death_rep") or hooks.get("fx_death_rep")) and \
                not any(e.get("kind") == "death_rep" for e in s.effects):
            return []
        saved, used = [], set()
        prev, s._dying = s._dying, list(objs)
        try:
            s._replace_deaths(objs, saved, used)
        finally:
            s._dying = prev
        return saved

    def _replace_deaths(s, objs, saved, used):
        for pid in (s.tp, 1 - s.tp):                       # rule 373.1: turn order
            mine = [o for o in objs if o.ctrl == pid]
            while mine:
                pairs = []
                for u in mine:
                    for r in s._death_reps(u, objs):
                        if r.get("key") in used:
                            continue
                        pairs.append((u, r))
                if not pairs:
                    break
                mand = [p for p in pairs if not p[1].get("may")]
                if mand and all(p[1]["name"] == "Zhonya's Hourglass" for p in mand):
                    # Zhonya's Hourglass: the player picks which unit it saves (kept from the original engine)
                    us = []
                    for u, _ in mand:
                        if u not in us:
                            us.append(u)
                    u = s.ask(pid, "zhonya_save", us) if len(us) > 1 else us[0]
                    choice = next(p for p in mand if p[0] is u)
                else:
                    opts = [Opt(f"{r['name']} : sauver {u.cname}", (u, r), u) for u, r in (mand or pairs)]
                    if not mand:
                        opts.append(Opt("ne rien remplacer", None))
                    pick = s.ask(pid, "death_replace", opts)
                    if pick is None or pick.value is None:
                        break
                    choice = pick.value
                u, r = choice
                if r.get("cost") is not None and not r["cost"](s):
                    used.add(r.get("key") or id(r))
                    continue
                if r.get("key") is not None:
                    used.add(r["key"])
                targets = [u]
                if r.get("multi"):
                    targets += [x for x in mine if x is not u and any(rr.get("key") == r.get("key")
                                                                       for rr in s._death_reps(x, objs))]
                for x in targets:
                    s.log(f"  {r['name']} replaces the death of {x}")
                    s.stats[f"death_rep_{r['name']}"] += 1
                    r["apply"](s, x)
                    saved.append(x)
                    mine.remove(x)

    def _death_reps(s, u, objs):
        out = []
        for src, im in s.providers("death_rep"):
            r = im.death_rep(s, src, u)
            if r is not None:
                r.setdefault("key", ("src", getattr(src, "uid", src.idx if hasattr(src, "idx") else id(src)), r["name"]))
                out.append(r)
        if u.attached:
            for go, _, im in s.fx_providers("fx_death_rep", u):
                r = im.fx_death_rep(s, go, u)
                if r is not None:
                    r.setdefault("key", ("fx", go.uid, r["name"]))
                    out.append(r)
        for e in list(s.effects):
            if e.get("kind") == "death_rep":
                r = e["fn"](s, e, u)
                if r is not None:
                    r.setdefault("key", ("eff", id(e)))
                    out.append(r)
        return out

    def save_unit(s, u, heal=True, exhaust=True):
        """'Heal it, exhaust it, and recall it' (the usual death replacement)."""
        if heal:
            u.damage = 0
            u.dmg_by = {}
        if exhaust:
            u.exhausted = True
        s.recall(u)

    def replace_battlefield(s, b, name):
        """Replace a battlefield with a battlefield token (rule 438): the token keeps every status of the battlefield
        it replaces (control, scored, contested, hidden cards, the showdown: rule 438.1, same Battlefield object).
        The replaced card waits in Banishment as "replaced" (438.5.a; kept in b.replaced for a swap back, 438.7); a
        replaced token stops existing (438.6). Emits 'bf_replaced' (bf, old, new)."""
        old = b.name
        if b.replaced is None:
            b.replaced = old                 # the card; a token replacing a token keeps the original card
        b.name = name
        s.need_cleanup = True
        s.log(f"  {old} is replaced with {name}")
        s.emit("bf_replaced", bf=b.idx, old=old, new=name)

    def swap_back(s, b):
        """Swap back (rule 438.7.b): the battlefield token stops existing and the card it replaced returns."""
        if b.replaced is None:
            return False
        old, b.name, b.replaced = b.name, b.replaced, None
        s.need_cleanup = True
        s.log(f"  {old} is swapped back to {b.name}")
        s.emit("bf_replaced", bf=b.idx, old=old, new=b.name)
        return True

    def recall(s, o):
        """Recall (rule 455): to base, not a move (movement restrictions don't apply)."""
        o.loc = "base"
        o.desig = None
        for gu in o.attached:
            go = s.obj(gu)
            if go is not None:
                go.loc = "base"
        s.need_cleanup = True

    def can_move(s, o, dest, by_pid, standard=False):
        """Movement restrictions (rule 359.3.e.6: a forbidden move instruction is ignored): the unit's own text
        (Impl.cant_move(g, o, dest, by_pid, standard)), attached Equipment (Impl.fx_cant_move(g, gear, o, dest,
        by_pid, standard)), other objects (Impl.aura_cant_move(g, src, o, dest, by_pid, standard)) and effects
        (g.effects kind='cant_move' fn(g, eff, o, dest, by_pid, standard))."""
        im = s.impl(o)
        if im is not None and im.cant_move is not None and im.cant_move(s, o, dest, by_pid, standard):
            return False
        if o.attached:
            for go, _, ig in s.fx_providers("fx_cant_move", o):
                if ig.fx_cant_move(s, go, o, dest, by_pid, standard):
                    return False
        for src, ia in s.providers("aura_cant_move"):
            if ia.aura_cant_move(s, src, o, dest, by_pid, standard):
                return False
        for e in s.effects:
            if e.get("kind") == "cant_move" and e["fn"](s, e, o, dest, by_pid, standard):
                return False
        return True

    def move_tax(s, pid, units, dest):
        """Additional cost to move several units at the same time (Impl.move_tax(g, src, pid, units, dest) ->
        reqs: Mageseeker Investigator)."""
        reqs = []
        for src, im in s.providers("move_tax"):
            reqs += im.move_tax(s, src, pid, units, dest) or []
        return reqs

    def move(s, objs, dest, by_pid, standard=False):
        """Move units (rule 445). dest: 'base' or battlefield index. Forbidden moves are skipped (can_move); a
        move cost of several units (move_tax) is paid by the moving player for a move made by an effect, units
        beyond what they can pay for staying where they are."""
        objs = [o for o in objs if o in s.board and o.loc != dest and s.can_move(o, dest, by_pid, standard)]
        if not standard and len(objs) > 1 and _cards().HOOKS.get("move_tax"):
            mover = objs[0].ctrl
            while len(objs) > 1:
                reqs = s.move_tax(mover, objs, dest)
                if not reqs or s.pay(mover, 0, reqs):
                    break
                objs = objs[:-1]
        moved = []
        for o in objs:
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

    def replace_choice_effect(s, o, what, duration=None):
        """Gangplank, Naval: "If a spell or ability that chooses me would stun me, give me -might, or return me to
        hand, give me +3 might instead." (Impl.choice_rep(g, o, item, what, duration) -> True if it replaced the
        effect; duration: that of a replaced -might).
        Returns True if the effect happens normally."""
        it = s.resolving
        if it is None or o.uid not in it.chosen:
            return True
        im = s.impl(o)
        if im is not None and im.choice_rep is not None and im.choice_rep(s, o, it, what, duration):
            return False
        return True

    def stun(s, o, by_pid):
        if o is None or o not in s.board or o.spec["type"] != "Unit" or o.stunned:
            return
        if not s.replace_choice_effect(o, "stun"):
            return
        o.stunned = True
        s.log(f"  {o} is stunned")
        s.emit("stun", obj=o, by=by_pid)

    def buff(s, o, by=None):
        """Buff (rule 426): +1 Might counter if the unit has none (any number with Impl.multi_buff: Lee Sin,
        Ascetic). Returns True if it was buffed. Emits 'buff' (obj, by)."""
        if o is None or o not in s.board:
            return False
        im = s.impl(o)
        if o.buff == 0 or (im is not None and im.multi_buff):
            o.buff += 1
            if by is None:
                by = s.resolving.ctrl if s.resolving is not None else o.ctrl
            s.emit("buff", obj=o, by=by)
            return True
        return False

    def spend_buff(s, o, pid=None):
        """Spend a buff (rule 745): remove one buff counter; emits 'spend_buff' (obj, pid). Returns True if done."""
        if o is None or o.buff <= 0:
            return False
        o.buff -= 1
        s.need_cleanup = True
        s.emit("spend_buff", obj=o, pid=o.ctrl if pid is None else pid)
        return True

    def empower(s, o, by=None):
        """Empower (rule 441): returns True if o became Empowered (emits 'empowered' obj=, by= the player who
        empowers: by, else the controller of the resolving item, else o's controller)."""
        if o is None or o.empowered:
            return False
        o.empowered = True
        if by is None:
            by = s.resolving.ctrl if s.resolving is not None else o.ctrl
        s.log(f"  {o} is empowered")
        s.emit("empowered", obj=o, by=by)
        return True

    def disempower(s, o):
        if o is not None and o.empowered:
            o.empowered = False
            return True
        return False

    def can_ready(s, o, by=None, kind=None):
        """"I can't be readied" (Impl.no_ready(g, o)) and "spells and abilities can't ready ..."
        (Impl.aura_no_ready(g, src, o, by, kind); kind: 'awaken' or the kind of the resolving item)."""
        im = s.impl(o)
        if im is not None and im.no_ready is not None and im.no_ready(s, o):
            return False
        for src, ia in s.providers("aura_no_ready"):
            if ia.aura_no_ready(s, src, o, by, kind):
                return False
        return True

    def ready_obj(s, o, by=None, kind=None):
        """Ready an object; emits 'ready' (obj, by). by: the player who readies it (Awaken: the turn player; an
        effect: its controller), kind: 'awaken' or the resolving item's kind."""
        if o.exhausted:
            if by is None and s.resolving is not None:
                by = s.resolving.ctrl
            if kind is None and s.resolving is not None:
                kind = s.resolving.kind
            if not s.can_ready(o, by, kind):
                return
            o.exhausted = False
            s.emit("ready", obj=o, by=by)

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
                ch = it.data.get("_choose")
                if ch is not None and getattr(ch, "options", None) is not None and not ch.options(s, it):
                    # aucune cible légale : la capacité est retirée (402.4) ; on ne demande pas « utiliser l'effet ? »
                    s.log(f"  {it.name} : aucune cible possible, l'effet ne s'applique pas")
                    continue
                c = it.data.get("_cost")
                if c is not None and it.data.get("_may") and not s.cost_payable(it, c):
                    # coût optionnel impayable (Valley of Idols sans énergie) : ni question ni effet, et on le dit
                    s.log(f"  {it.name} : pas de quoi payer ({getattr(c, 'label', 'le coût')}), l'effet ne s'applique pas")
                    continue
                if it.data.get("_may") and not s.ask(pid, "may", [True, False], item=it):
                    continue
                if ch is not None and not ch(s, it):
                    continue
                if c is not None and not c(s, it):
                    s.log(f"  {it.name} : coût non payé, l'effet ne s'applique pas")
                    continue
                s.chain.append(it)
                s.log(f"  trigger on chain: {it.name}")
                s.on_finalize(it)
        if s.chain:
            s.priority = s.chain[-1].ctrl
            s.passes = 0
        s.need_cleanup = True

    def cost_payable(s, it, c):
        """Le coût optionnel c d'un déclenchement « may » peut-il être payé maintenant ? (retour utilisateur : Valley of
        Idols proposée sans énergie). c.can quand le coût le dit (cards.may_pay) ; sinon, pour un joueur humain
        seulement, essai sur une copie de la partie (premier choix à chaque question) ; l'IA garde son comportement."""
        if getattr(c, "can", None) is not None:
            return bool(c.can(s, it))
        ag = s.agents[it.ctrl] if s.agents else None
        if not (getattr(ag, "every_choice", False) or getattr(s, "every_choice", False)):
            return True
        g2 = s.clone()
        g2.agents = [_FirstChoice(), _FirstChoice()]
        try:
            return bool(c(g2, it))
        except Exception:                                  # noqa: BLE001 — dans le doute, on pose la question
            return True

    def on_finalize(s, item):
        """Targeting effects (Irelia: 'When you choose me')."""
        for uid in sorted(item.chosen):
            o = s.obj(uid)
            if o is not None:
                if o.ctrl != item.ctrl and o.spec["type"] == "Unit":
                    s.hist["chose_enemy"][item.ctrl] = True
                s.emit("chosen", obj=o, item=item)

    def emit(s, ev, **info):
        """Dispatch an event to every permanent, battlefield and active effect that listens to it. A permanent that
        leaves the board gets its own 'leave' event (Impl.on_leave(g, obj, info): "When this leaves the board")."""
        if ev == "leave":
            im = s.impl(info["info"]["name"])
            if im is not None and im.on_leave is not None:
                im.on_leave(s, info["obj"], info)
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
                if go is not None and go.copy_name and go.cname == "Svellsongur" and o in s.board:
                    ic = s.impl(go.copy_name)    # the copied text, appended to the unit
                    if ic is not None and ic.on_event is not None:
                        ic.on_event(s, o, ev, info)
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
        if s.zone_listen:
            names = _cards().HOOKS.get("zone_event")
            if names:
                found = False
                for pl in s.p:
                    for c in pl.hand + pl.trash:
                        if c.cname in names:
                            found = True
                            s.impl(c).zone_event(s, c, ev, info)
                if not found and s.stage != "setup" and not any(
                        c.cname in names for pl in s.p for c in pl.deck + pl.champ + pl.banish) and not any(
                        o.cname in names for o in s.board) and not any(
                        it.card is not None and it.card.cname in names for it in s.chain + [s.resolving]
                        if it is not None) and not any(
                        c.cname in names for b in s.bfs for c in b.facedowns):
                    s.zone_listen = False       # no card of the game can ever listen from a hand or a trash

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
        if s.shown_hand is not None and not s.chain:  # l'effet qui a révélé la main est résolu et on rejoue : fin
            s.shown_hand = None
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
        prev, s.resolving = s.resolving, it
        try:
            if it.kind == "spell":
                im = s.impl(it.card)
                im.resolve(s, it)
                s.repeat_resolution(it, im)
                if it.card.zone == "chain":
                    dest = it.data.get("after", "trash")
                    s.card_leaves_chain(it.card, dest, it.ctrl)
            else:
                it.fn(s, it)
        finally:
            s.resolving = prev
        if it.kind == "spell":
            s.played_event(it.data.get("played_by", it.ctrl), it.card, it)
        elif it.kind == "ability" and it.data.get("_ab") is not None:
            # "When you use/play an activated ability" triggers as it resolves (rule 377.2.a)
            s.emit("activated", pid=it.ctrl, obj=s.obj(it.src) or it.data.get("_src_obj"), ab=it.data["_ab"],
                   cost_e=it.data.get("_cost_e", 0), item=it)
        s.need_cleanup = True
        if s.chain:
            s.priority = s.chain[-1].ctrl

    def repeat_resolution(s, it, im):
        """Generic [Repeat] (rule 820): each Repeat cost paid (it.data['reps']: list of repetition choices, from
        printed or granted Repeat) executes the spell's effect again with that choice (new targets checked with the
        same predicates). Resolvers wrapped by cards.repeatable handle their printed Repeat themselves."""
        reps = it.data.get("reps")
        if not reps:
            return
        preds = im.preds or []
        for ch in reps:
            if it.card.zone != "chain":
                break
            it2 = Item("spell", it.ctrl, it.name, card=it.card, src=it.src,
                       data={**it.data, **ch, "rep": False, "reps": None, "tg2": ()})
            for i, uid in enumerate(ch.get("tg", ())):
                o = s.obj(uid) if isinstance(uid, int) else None
                pred = preds[min(i, len(preds) - 1)] if preds else None
                if o is not None and o.oid == ch.get("_oids", {}).get(uid, o.oid):
                    it2.targets.append((o.uid, ch.get("_oids", {}).get(uid, o.oid), pred))
                else:
                    it2.targets.append((uid if isinstance(uid, int) else None, -1, pred))
            it2.chosen = set(it.chosen)
            s.log(f"  {it.name} repeats")
            im.resolve(s, it2)

    def card_leaves_chain(s, c, dest, by=None):
        """A spell card leaves the chain (resolved or countered): trash, banish (emits 'banish'), or 'recycle'
        (bottom of the Main Deck: Kai'Sa, Evolutionary's "Then recycle it")."""
        dest = s.trash_dest(c, dest, "chain")
        c.zone = None
        if dest == "banish":
            c.zone = "banish"; s.p[c.owner].banish.append(c)
        elif dest == "recycle":
            c.zone = "deck"; s.p[c.owner].deck.append(c)
        else:
            c.zone = "trash"; s.p[c.owner].trash.append(c)
        c.oid += 1
        if dest == "banish":
            s.emit("banish", pid=c.owner if by is None else by, card=c)

    def counter(s, it):
        """Rule 425: a countered item does nothing and is cleared from the chain (cards to the trash)."""
        if it not in s.chain or not s.counterable(it):
            return False
        s.chain.remove(it)
        s.log(f"  {it.name} is countered")
        s.stats[f"countered_{it.name}"] += 1
        if it.kind == "spell":
            after = it.data.get("after", "trash")
            s.card_leaves_chain(it.card, after if (it.data.get("flow") or after == "recycle") else "trash", it.ctrl)
        s.need_cleanup = True
        return True

    def counterable(s, it):
        """"This can't be countered" (item.uncounterable) and "Your spells and abilities can't be countered"
        (Impl.no_counter(g, src, item) of other objects: Mel, Newly Awakened)."""
        if it.uncounterable:
            return False
        for src, im in s.providers("no_counter"):
            if im.no_counter(s, src, it):
                return False
        return True

    def played_event(s, pid, card, item=None):
        """A card's play completed (rule 419.4.a). Tokens are played too (rule 185.2.a) but are not cards: they
        emit 'played' with n=0 and token=True and are not added to g.played."""
        if card.token:
            s.hist["tokens"][pid] += 1
            s.emit("played", pid=pid, card=card, item=item, n=0, token=True)
            return
        s.played[pid].append(card.cname)
        s.emit("played", pid=pid, card=card, item=item, n=len(s.played[pid]), token=False)

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
        # "When my Might becomes N or more" (Impl.might_watch = N): emits 'might_reached' (obj, n)
        names = _cards().HOOKS.get("might_watch")
        if names:
            for u in s.units():
                if u.cname in names:
                    n = s.impl(u).might_watch
                    m = s.might(u)
                    if u.might_seen is not None and u.might_seen < n <= m:
                        s.emit("might_reached", obj=u, n=n)
                    u.might_seen = m
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
            for c in list(b.facedowns):
                if b.ctrl != c.owner:
                    s.log(f"  hidden {c} at {b.name} is trashed")
                    s.trash_facedown(b, c)
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

    def trash_facedown(s, b, c):
        """A hidden card removed from a battlefield goes to its owner's trash (rule 323.7.5)."""
        b.facedowns.remove(c)
        dest = s.trash_dest(c, "trash", "facedown")
        c.reset(); c.oid += 1; c.zone = dest
        (s.p[c.owner].banish if dest == "banish" else s.p[c.owner].trash).append(c)

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
                u.dmg_by = {}
            atk = [u for u in s.units(loc=b.idx) if u.desig == "att"]
            dfn = [u for u in s.units(loc=b.idx) if u.desig == "def"]
            if atk and dfn:
                # Symbol of the Solari: "If a combat where you are the attacker ends in a tie, recall ALL units
                # instead." (Impl.tie_recall_all on a permanent the attacker controls)
                if any(src.ctrl == sd.attacker and im.tie_recall_all for src, im in s.providers("tie_recall_all")):
                    atk = atk + dfn
                    s.log("  tie: every unit is recalled")
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
            for c in list(b.facedowns):
                if c.owner != b.ctrl:
                    s.trash_facedown(b, c)
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

    def deals_combat_damage(s, u):
        """Units that contribute their Might to combat damage (rule 465): not stunned, and not "I don't deal combat
        damage" (Impl.no_combat_damage(g, o)) nor excluded by another object (Impl.aura_no_combat(g, src, o))."""
        if u.stunned:
            return False
        im = s.impl(u)
        if im is not None and im.no_combat_damage is not None and im.no_combat_damage(s, u):
            return False
        for src, ia in s.providers("aura_no_combat"):
            if ia.aura_no_combat(s, src, u):
                return False
        return True

    def combat_damage(s, sd, atk, dfn):
        """Rule 465: sum Might of the units that deal combat damage, attacker assigns first, then dealt
        simultaneously (replacement effects on each unit: damage_replace)."""
        a_total = sum(max(0, s.might(u)) for u in atk if s.deals_combat_damage(u))
        d_total = sum(max(0, s.might(u)) for u in dfn if s.deals_combat_damage(u))
        s.log(f"  combat damage: attackers {a_total} vs defenders {d_total}")
        sd.excess = [0, 0]
        assign_a = s.assign_damage(sd.attacker, a_total, dfn)
        assign_d = s.assign_damage(sd.defender, d_total, atk)
        dealt = []
        sd.assigned = (assign_a, assign_d)       # excess damage checks ("if you assigned 3 or more excess damage")
        for u, n in list(assign_a.items()) + list(assign_d.items()):
            if n <= 0 or u not in s.board:
                continue
            n = s.damage_replace(u, n, dict(kind="combat", by=1 - u.ctrl, item=None, target=u))
            if n > 0:
                s.mark_damage(u, n, 1 - u.ctrl, "combat")
                dealt.append((u, n))
        for u, n in dealt:
            s.emit("damaged", obj=u, amount=n, kind="combat", by=1 - u.ctrl)
        s.need_cleanup = True

    def lethal_need(s, u, pid=None):
        """Lethal damage still needed by u (rule 465.2.c): 1 is enough when the assigning player's damage is always
        lethal for u (Impl.lethal_any)."""
        if pid is not None and pid != u.ctrl:
            for src, im in s.providers("lethal_any"):
                if src.ctrl == pid and im.lethal_any(s, src, u):
                    return 1 + u.prevent
        return max(1, s.might(u) - u.damage) + u.prevent

    def assign_damage(s, pid, total, targets):
        """Legal assignment orders (Tank first, Backline and "assigned combat damage last" last, lethal in full before
        the next unit, excess only on the last unit). The assigning player chooses the order of units within a tier.
        Tank is ignored by a player who "ignores [Tank] while assigning combat damage here" (Impl.ignore_tank).
        The excess damage (beyond lethal) is kept in g.sd.excess[pid]."""
        out = {}
        if total <= 0 or not targets:
            return out
        bf = targets[0].loc
        no_tank = any(im.ignore_tank(s, src, pid, bf) for src, im in s.providers("ignore_tank"))

        def tank(u):
            return not no_tank and s.has_kw(u, "Tank")

        def last(u):
            im = s.impl(u)
            return s.has_kw(u, "Backline") or (im is not None and im.assign_last)
        tiers = [[u for u in targets if tank(u)],
                 [u for u in targets if not tank(u) and not last(u)],
                 [u for u in targets if last(u) and not tank(u)]]
        from actions import full_choices
        if full_choices(s, pid):
            return s.assign_by_hand(pid, total, tiers)
        order = []
        for t in tiers:
            if len(t) > 1:
                t = s.ask(pid, "damage_order", [t], targets=t) or t
            order += t
        left = total
        excess = 0
        for i, u in enumerate(order):
            if left <= 0:
                break
            need = s.lethal_need(u, pid)
            n = left if i == len(order) - 1 else min(left, need)
            out[u] = n
            excess += max(0, n - need)
            left -= n
        if left > 0 and order:
            out[order[-1]] = out.get(order[-1], 0) + left
            excess += left
        if s.sd is not None:
            s.sd.excess[pid] = excess
        return out

    def assign_by_hand(s, pid, total, tiers):
        """Joueur humain : il choisit une à une l'unité qui reçoit ses dégâts mortels en entier (465.2.c.3), parmi
        celles que les règles permettent à cet instant (Tank d'abord 815.1.c.2, Backline en dernier 826.4.b).
        La dernière unité reçoit tout le reste, excédent compris (465.2.c.4)."""
        out, left, excess = {}, total, 0
        tiers = [list(t) for t in tiers if t]
        while left > 0 and tiers:
            t = tiers[0]
            if len(t) == 1:
                u = t[0]
            else:
                u = s.ask(pid, "damage_pick", list(t), left=left, need={x.uid: s.lethal_need(x, pid) for x in t})
            t.remove(u)
            if not t:
                tiers.pop(0)
            need = s.lethal_need(u, pid)
            n = left if not tiers else min(left, need)
            out[u] = n
            excess += max(0, n - need)
            left -= n
        if s.sd is not None:
            s.sd.excess[pid] = excess
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

    def score_point(s, pid, b, why):
        """The point of a Conquer or Hold (rules 469-471). Replacement: Impl.point_rep(g, src, pid, b, why) -> True
        if it replaced the point (Otterpus: "they draw 1 instead"). Returns True if a point was gained."""
        for src, im in s.providers("point_rep"):
            if im.point_rep(s, src, pid, b, why):
                return False
        return s.gain_point(pid, f"{why} {b.name}")

    def conquer(s, pid, b):
        """Rule 469.1, 471: conquer scores the battlefield (final point rule) and triggers conquer effects."""
        s.stats[f"conquer_P{pid}"] += 1
        if s.can_score(pid, b):
            b.scored.add(pid)
            pl = s.p[pid]
            if pl.points >= s.victory - 1:
                if all(pid in x.scored for x in s.bfs):
                    s.score_point(pid, b, "conquer")
                else:
                    s.log(f"  P{pid} draws instead of the final point")
                    s.stats["final_point_draw"] += 1
                    s.draw(pid, 1)
            else:
                s.score_point(pid, b, "conquer")
        else:
            b.scored.add(pid)
        units = [u for u in s.units(pid, b.idx)]
        s.hist["conquered"] += [(u.uid, u.oid) for u in units]
        s.hist["conq_bf"][pid].append(b.idx)
        s.scoring_event("conquer", pid, b, units)

    def scoring_event(s, ev, pid, b, units):
        """Conquer/hold effects (Hunt, 'conquer'/'hold' events). "Your conquer (hold) effects for conquering
        (holding) here trigger an additional time" (Impl.scoring_extra(g, src, pid, bf, ev) -> n): the triggered
        abilities of pid queued by this event are queued n more times. "My hold effects are also conquer effects,
        and vice versa" (Equipment with Impl.fx_swap_scoring): the unit's own handlers also get the other event."""
        n0 = len(s.trigq)
        s.hunt(pid, units)
        s.emit(ev, pid=pid, bf=b.idx, units=units)
        other = "hold" if ev == "conquer" else "conquer"
        for u in units:
            if u in s.board and u.attached and s.fx_providers("fx_swap_scoring", u):
                s.emit_self(u, other, dict(pid=pid, bf=b.idx, units=units))
        extra = 0
        for src, im in s.providers("scoring_extra"):
            extra += im.scoring_extra(s, src, pid, b.idx, ev) or 0
        if extra:
            new = [t for t in s.trigq[n0:] if t.ctrl == pid]
            for _ in range(extra):
                for t in new:
                    t2 = copy.copy(t)
                    Item._n += 1
                    t2.id = Item._n
                    t2.data = dict(t.data)
                    t2.targets, t2.chosen = list(t.targets), set(t.chosen)
                    s.trigq.append(t2)

    def emit_self(s, o, ev, info):
        """Deliver an event only to o's own triggered abilities (and the Effect Text of its Equipment)."""
        im = s.impl(o)
        if im is not None and im.on_event is not None:
            im.on_event(s, o, ev, info)
        for gu in list(o.attached):
            go = s.obj(gu)
            ig = s.impl(go) if go is not None else None
            if ig is not None and ig.effect_event is not None:
                ig.effect_event(s, go, o, ev, info)

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
            if s.score_point(pid, b, "hold"):
                s.hist["hold_pts"][pid] += 1
        else:
            b.scored.add(pid)
        units = [u for u in s.units(pid, b.idx)]
        s.scoring_event("hold", pid, b, units)

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
        s.reset_hist()
        for b in s.bfs:
            b.scored = set()
        s.log(f"===== turn {s.turn_no}: P{s.tp} ({s.p[s.tp].legend_name}) points {s.p[0].points}-{s.p[1].points}")

    def turn_step(s):
        st = s.stage
        s.phase = st
        tp = s.tp
        pl = s.p[tp]
        if st == "setup":
            s.setup()
            return
        if st == "awaken":
            for o in list(s.board):
                if o.ctrl == tp:
                    s.ready_obj(o, by=tp, kind="awaken")
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
            for src, im in s.providers("channel_mod"):            # Sandstone Chimera
                n = im.channel_mod(s, src, tp, n)
            s.channel(tp, n)
            s.stage = "draw"
        elif st == "draw":
            if not s.effects_of("skip_draw", tp):         # Endless Riches: "Skip your Draw Phase."
                s.draw(tp, 1)
            s.stage = "main_start"
        elif st == "main_start":
            for p_ in s.p:
                p_.pool_e = 0; p_.pool_p = Counter(); p_.pool_r = []
            s.emit("main_start", pid=tp)
            s.stage = "main"
        elif st == "end_step":
            s.emit("end_turn", pid=tp)
            s.stage = "expire"
        elif st == "expire":
            for u in s.units():
                u.damage = 0
                u.dmg_by = {}
                u.stunned = False
                u.prevent = 0
            for o in s.board:
                o.mods = [m for m in o.mods if m[1] != "turn"]
                o.grants = [g for g in o.grants if g[2] != "turn"]
                o.abs = [a for a in o.abs if a[1] != "turn"]
                o.xtags = [t for t in o.xtags if t[1] != "turn"]
                o.dmg_by = {}
            s.effects = [e for e in s.effects if e.get("dur") != "turn"]
            for p_ in s.p:
                p_.pool_e = 0; p_.pool_p = Counter(); p_.pool_r = []
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


class _FirstChoice:
    """Agent d'essai pour Game.cost_payable : prend le premier choix proposé."""
    def choose(self, g, pid, kind, options, ctx):
        return options[0]


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
