"""AI for both decks.

SearchAgent: for every top-level decision (main action, priority, focus) it tries each legal action on copies of
the game, plays the consequences out (chain, showdown, combat) with both players passing, and keeps the action
with the best evaluation. Hidden information is respected: in each copy the opponent's hand and facedown cards
are re-drawn at random from the cards it has not seen, and both decks are shuffled (determinization).
All choices made during resolution (targets, "you may", damage assignment...) use the heuristics of `choose`.
"""
import os
import random
from game import SPEC, Obj, Item
from cards import value as unit_value

REACTIVE = {"Discipline", "Defy", "Not So Fast", "Back Off", "Block", "En Garde", "Ki Barrier", "Deathgrip",
            "Hidden Blade", "Stupefy", "Crumbling Sands", "Against the Odds", "Decree of Focus", "Sacrifice",
            "Long Sword", "Sterak's Gage", "Zhonya's Hourglass", "Vi, Peacekeeper", "Chakram Dancer"}
# Tempo (2026-10-03, guides riftbound.gg « scoring » et tempo vs value) : à 6 points on est à portée de victoire,
# à 7 une tenue gagne. L'IA doit alors refuser la tenue adverse à tout prix. RB_TEMPO=0 rend l'ancienne IA.
TEMPO = os.environ.get("RB_TEMPO", "1") != "0"
URGENT_SAMPLES = 6
# Recherche (2026-10-09) : "old" = chaque option jugée sur son propre monde tiré au hasard (s.rng avance entre les
# options) ; "crn" = tirages communs (le k-ième monde est le même pour toutes les options) ; "sh" = tirages communs
# + élimination en plusieurs passes (successive halving) avec SH_EXTRA × (options × samples) rollouts en plus.
# RB_SEARCH=old rend l'ancienne recherche (comparaisons).
SEARCH = os.environ.get("RB_SEARCH", "sh")
SH_EXTRA = float(os.environ.get("RB_SH_EXTRA", "1.0"))

DK_FODDER = {"Soaring Scout", "Honest Broker", "Watchful Sentry", "Black Rose Dignitary", "LeBlanc, Fragmented",
             "Lonely Poro", "Scuttle Crab"}


# ---------------------------------------------------------------------- evaluation
def lasting_might(g, o):
    """Might without 'this turn' effects (they expire before the next turn)."""
    m = g.might(o)
    m -= sum(a for a, d in o.mods if d == "turn")
    return m


# Poids de l'évaluation (valeurs historiques). SearchAgent(cfg={"ev": {...}}) en remplace une partie (essais d'auto-jeu).
EV = dict(pts=7.0, pts_hi=4.0, bf=3.0, fd=1.8, hold_win=40.0, unit0=1.0, might=0.8, cost=0.12, on_bf=0.4,
          card0=1.4, card_e=0.05, react=2.0, react_kw=0.0, rune=0.9, leg_emp=2.0, xp=0.15, deck_low=3.0)


def point_value(p, victory, w=EV):
    return w["pts"] * p + w["pts_hi"] * max(0, p - (victory - 3))


def card_value(c, w=EV):
    n = c.cname
    if n in REACTIVE:
        return w["react"]
    sp = c.spec
    if w["react_kw"] and "Reaction" in sp["keywords"]:
        return w["react"]
    return w["card0"] + w["card_e"] * min(sp["e"], 8)


def unit_eval(g, u, w=EV):
    m = max(0, lasting_might(g, u))
    sp = u.spec
    v = w["unit0"] + w["might"] * m + w["cost"] * (sp["e"] + 1.5 * sp["p"])
    if u.token and u.cname == "Reflection" or g.has_kw(u, "Temporary"):
        v = 0.4 + 0.4 * m
    if u.cname == "Mech":
        v = 0.8 + 0.8 * m
    if u.cname == "Karthus, Eternal":
        v += 2.5
    if u.empowered:
        v += 0.5
    if u.loc in (0, 1):
        v += w["on_bf"]
    return v


GEAR_V = {"Zhonya's Hourglass": 2.5, "Baited Hook": 3.0, "Gold": 0.9, "Long Sword": 0.8, "Sterak's Gage": 1.0,
          "Pendulum Blade": 0.8}


def danger(g, me):
    """L'adversaire est à portée de victoire (6+ sur 8) : plus de long terme."""
    return g.p[1 - me].points >= g.victory - 2


def evaluate(g, me, w=None):
    if w is None:
        w = EV
    if g.winner is not None:
        t = 25.0 * g.turn_no if TEMPO else 0.0         # gagner tôt, perdre le plus tard possible
        if g.winner == me:
            return 10000.0 - t
        if g.winner == 1 - me:
            return -10000.0 + t
        return 0.0
    s = 0.0
    for pid, sign in ((me, 1.0), (1 - me, -1.0)):
        pl = g.p[pid]
        v = point_value(pl.points, g.victory, w)
        held = 0
        for b in g.bfs:
            if b.ctrl == pid:
                v += w["bf"]
                held += 1
                if b.facedown is not None and b.facedown.owner == pid:
                    v += w["fd"]
        if TEMPO and held and pl.points >= g.victory - 1:
            v += w["hold_win"]                         # tenue gagnante au début de son prochain tour
        for o in g.board:
            if o.ctrl != pid:
                continue
            if o.spec["type"] == "Unit":
                v += unit_eval(g, o, w)
            else:
                v += GEAR_V.get(o.cname, 1.0)
        v += sum(card_value(c, w) for c in pl.hand)
        v += w["rune"] * len(pl.runes)
        if pl.legend.empowered:
            v += w["leg_emp"]
        v += w["xp"] * pl.xp
        if len(pl.deck) < 3:
            v -= w["deck_low"]
        s += sign * v
    return s


# ---------------------------------------------------------------------- heuristic choices
class Heuristics:
    cfg = {}

    def mulligan(s, g, pid):
        mode = s.cfg.get("mulligan", "normal")
        h = g.p[pid].hand
        if mode == "aggro":
            # keep units costing 4 or less and Shuriken Flip, throw the rest (up to 2)
            keep = [c for c in h if (c.spec["type"] == "Unit" and c.spec["e"] <= 4) or c.cname == "Shuriken Flip"]
            rest = [c for c in h if c not in keep]
            rest.sort(key=lambda c: -(c.spec["e"] + 2 * c.spec["p"]))
            return rest[:2]
        if mode == "none":
            return []
        out = []
        cheap = [c for c in h if c.spec["type"] == "Unit" and c.spec["e"] <= 3]
        for c in sorted(h, key=lambda c: -(c.spec["e"] + 2 * c.spec["p"])):
            if len(out) >= 2:
                break
            if c.spec["e"] >= 6:
                out.append(c)
        if not cheap and len(out) < 2:
            rest = [c for c in sorted(h, key=lambda c: -c.spec["e"]) if c not in out and c.spec["e"] >= 4]
            out += rest[:2 - len(out)]
        return out

    def choose(s, g, pid, kind, options, ctx):
        if kind == "may":
            it = ctx.get("item")
            name = it.name if it is not None else ""
            if name.startswith("Star Spring"):
                return False
            if name.startswith("Dusk Rose Lab"):
                return any(u.ctrl == pid and (u.cname in DK_FODDER or g.has_kw(u, "Temporary")) for u in g.units(pid))
            return True
        if kind == "dusk_kill":
            fod = [u for u in options if u is not None and (g.has_kw(u, "Temporary") or u.cname in DK_FODDER)]
            return min(fod, key=lambda u: unit_value(g, u)) if fod else None
        if kind == "target":
            it = ctx.get("item")
            # prefer an enemy unit that this effect kills, then the most valuable enemy
            dmg = it.data.get("dmg", 0) if it is not None else 0
            enemies = [o for o in options if o.ctrl != pid]
            if dmg:
                kills = [o for o in enemies if g.might(o) - o.damage <= dmg + (1 if o.loc in (0, 1) and g.bfs[o.loc].name == "Void Gate" else 0)]
                if kills:
                    return max(kills, key=lambda o: unit_value(g, o))
            if enemies:
                return max(enemies, key=lambda o: unit_value(g, o))
            return options[0]
        if kind == "zhonya_save":
            return max(options, key=lambda u: unit_value(g, u))
        if kind == "damage_order":
            t = list(options[0])
            # kill the most valuable units per point of damage first
            t.sort(key=lambda u: -unit_value(g, u) / max(1, g.lethal_need(u)))
            return t
        if kind in ("hook_pick", "mixologist_pick", "herald_pick"):
            real = [o for o in options if o is not None]
            return real[0] if real else None
        if kind == "herald_dk_pick":
            return options[0]
        if kind == "sacrifice":
            return min(options, key=lambda u: unit_value(g, u))
        if kind == "discard":
            return min(options, key=lambda c: card_value(c) + 0.05 * c.spec["e"])
        if kind == "copy_target":
            return max(options, key=lambda u: unit_value(g, u))
        if kind == "play_location":
            return "base" if "base" in options else options[0]
        if kind == "recycle_rune":
            from collections import Counter
            cnt = Counter(r.domain for r in g.p[pid].runes)
            ex = [r for r in options if r.exhausted] or options
            return max(ex, key=lambda r: cnt[r.domain])
        if kind == "equip_target":
            return max(options, key=lambda u: (u.loc in (0, 1), g.might(u)))
        if kind == "star_spring":
            return options[0]
        if kind == "ashe_pick":
            return options[0]
        return options[0]


class FastAgent(Heuristics):
    """Rollout policy: passes every window, ends the turn, heuristic choices."""

    def start(s, g):
        pass

    def decide(s, g, d):
        if d.kind == "main":
            return ("end",)
        return ("pass",)


def determinize(g, me, rng):
    """Re-draw what `me` cannot see: opponent hand and facedown cards, order of both decks."""
    opp = 1 - me
    po = g.p[opp]
    if po.revealed_turn != g.turn_no:
        hidden_fd = [b for b in g.bfs if b.facedown is not None and b.facedown.owner == opp]
        pool = po.deck + po.hand
        rng.shuffle(pool)
        nh = len(po.hand)
        po.hand = pool[:nh]
        po.deck = pool[nh:]
        for c in po.hand:
            c.zone = "hand"
        for c in po.deck:
            c.zone = "deck"
        for b in hidden_fd:
            cands = [c for c in po.deck if g.impl(c) is not None and g.impl(c).hidden]
            if cands and rng.random() < 0.85:
                new = rng.choice(cands)
                old = b.facedown
                po.deck.remove(new)
                new.zone, new.hidden_turn, new.hidden_bf = "facedown", old.hidden_turn, old.hidden_bf
                old.zone, old.hidden_turn, old.hidden_bf = "deck", None, None
                po.deck.append(old)
                b.facedown = new
        rng.shuffle(po.deck)
    else:
        rng.shuffle(po.deck)
    rng.shuffle(g.p[me].deck)


def rollout(g, max_steps=300):
    fa = g.agents[0]
    for _ in range(max_steps):
        d = g.advance()
        if d is None or d.kind == "main":
            return
        g.apply(fa.decide(g, d))


class SearchAgent(Heuristics):
    def __init__(s, seed=0, samples=1, max_cands=40, name="", horizon=2, cfg=None, search=None, sh_extra=None):
        s.cfg = dict(cfg or {})
        s.rng = random.Random(seed)
        s.samples = samples
        s.max_cands = max_cands
        s.name = name
        s.horizon = horizon
        s.fast = FastAgent()
        s.search = search or SEARCH
        s.ev = dict(EV, **s.cfg["ev"]) if s.cfg.get("ev") else None
        s.sh_extra = SH_EXTRA if sh_extra is None else sh_extra

    def start(s, g):
        pass

    def risky_attack(s, g, me, a):
        """Attacking a battlefield where the opponent has units and enough resources for an Ambush unit."""
        if a[0] != "move" or a[2] == "base":
            return False
        opp = 1 - me
        if not g.units(opp, a[2]):
            return False
        ready = sum(1 for r in g.p[opp].runes if not r.exhausted)
        return ready >= 5 and len(g.p[opp].hand) >= 1

    # -------- un rollout : surchargé par PlanAgent (politiques du plan, valeur + forme, a priori)
    def policies(s, me):
        pol = PolicyAgent()
        pol.cfg = s.cfg
        return [pol, pol]

    def value(s, c, me):
        return evaluate(c, me, s.ev)

    def prior_of(s, g, me, a):
        return 0.0

    def one(s, g, me, a, rng, world=None):
        """Valeur d'un rollout. `world` (graine) : le monde caché et l'aléa du jeu sont fixés par la graine, et les
        compteurs d'identifiants remis à l'identique, pour que toutes les options voient exactement le même monde."""
        if world is not None:
            n0, i0 = Obj._n, Item._n
        try:
            c = g.clone()
            determinize(c, me, rng)
            if world is not None:
                c.rng = random.Random(world * 2 + 1)
            c.agents = s.policies(me)
            c.apply(a)
            rollout_policy(c, s.horizon, cfg=s.cfg)
            return s.value(c, me)
        finally:
            if world is not None:
                Obj._n, Item._n = n0, i0

    def score(s, g, me, a, base_now=None):
        """Ancienne recherche : s.samples mondes tirés avec s.rng (différents d'une option à l'autre)."""
        tot = 0.0
        for _ in range(s.samples):
            tot += s.one(g, me, a, s.rng)
        return tot / s.samples + s.prior_of(g, me, a)

    def decide(s, g, d):
        opts = d.options
        if len(opts) == 1:
            return opts[0]
        me = d.player
        base_now = None
        if s.cfg.get("respect_ambush") and d.kind == "main":
            opts = [a for a in opts if not s.risky_attack(g, me, a)] or opts
        if len(opts) > s.max_cands:
            opts = opts[:1] + s.rng.sample(opts[1:], s.max_cands - 1)
        return s.pick(g, d, opts)[0]

    def urgent(s, g, d, me, best_v):
        return TEMPO and d.kind == "main" and s.samples < URGENT_SAMPLES and (best_v < -5000 or danger(g, me))

    def pick(s, g, d, opts):
        """Meilleure option et scores [(v, a)] ; en danger ou si tout perd, on refait avec plus de tirages."""
        if s.search != "old" and type(s).score is SearchAgent.score:
            # (une sous-classe qui redéfinit encore score(), ex. plans.py figé d'une session du manager : ancienne recherche)
            return s.pick_crn(g, d, opts)
        me = d.player
        scored = [(s.score(g, me, a), a) for a in opts]
        best_v = max(v for v, _ in scored)
        if s.urgent(g, d, me, best_v):
            # gagner à tout prix : un seul tirage dit « tout perd » au hasard ; on mesure la chance de survie
            n0, s.samples = s.samples, URGENT_SAMPLES
            try:
                scored = [(s.score(g, me, a), a) for a in opts]
            finally:
                s.samples = n0
        best, best_v = None, -1e18
        for v, a in scored:
            if v > best_v + 1e-9:
                best, best_v = a, v
        return best, best_v, scored

    def pick_crn(s, g, d, opts):
        """Tirages communs : une graine par indice de tirage, tirée au début de la décision ; l'option i au tirage k
        est jouée dans le monde k (même main et cartes cachées adverses, même ordre des decks, même aléa).
        Mode "sh" : après la première passe, on garde la meilleure moitié et on lui ajoute des tirages, jusqu'à 2
        options ; budget en plus = sh_extra × options × samples, réparti également entre les passes."""
        me = d.player
        n = len(opts)
        worlds = []
        tot = [0.0] * n
        cnt = [0] * n
        pri = [s.prior_of(g, me, a) for a in opts]

        def run(i, k):
            while len(worlds) < k:
                worlds.append(s.rng.getrandbits(30))
            for j in range(cnt[i], k):
                tot[i] += s.one(g, me, opts[i], random.Random(worlds[j]), worlds[j])
            cnt[i] = max(cnt[i], k)

        def val(i):
            return tot[i] / cnt[i] + pri[i]

        def ranked(idx):
            return sorted(idx, key=lambda i: (-val(i), i))

        k0 = s.samples
        for i in range(n):
            run(i, k0)
        if s.urgent(g, d, me, max(val(i) for i in range(n))):
            k0 = URGENT_SAMPLES                     # mêmes premiers mondes, on complète jusqu'à 6
            for i in range(n):
                run(i, k0)
        surv = ranked(range(n))
        if s.search == "sh" and n > 1:
            sizes, m = [], n
            while m > 2:
                m = max(2, m // 2)
                sizes.append(m)
            sizes = sizes or [2]
            budget = int(s.sh_extra * n * s.samples)
            per, carry = budget // len(sizes), budget % len(sizes)
            k = k0
            for m in sizes:
                surv = surv[:m]
                carry += per
                add = carry // m
                if add:
                    k += add
                    carry -= add * m
                    for i in surv:
                        run(i, k)
                surv = ranked(surv)
        scored = [(val(i), opts[i]) for i in range(n)]
        b = surv[0]
        return opts[b], val(b), scored


# ---------------------------------------------------------------------- cheap policy (rollouts, baseline)
def best_target_score(g, pid):
    return 0


class PolicyAgent(Heuristics):
    """Cheap rule-based play, no cloning. Used as the rollout policy and as a baseline opponent."""

    def start(s, g):
        pass

    def decide(s, g, d):
        if d.kind != "main":
            return s.react(g, d)
        return s.main(g, d)

    # -------- helpers
    def combat_margin(s, g, pid, bf, extra=()):
        mine = sum(max(0, g.might(u)) for u in g.units(pid, bf) if not u.stunned)
        mine += sum(max(0, g.might(u)) for u in extra)
        theirs = sum(max(0, g.might(u)) for u in g.units(1 - pid, bf) if not u.stunned)
        return mine, theirs

    def main(s, g, d):
        pid = d.player
        opts = d.options
        plays = [o for o in opts if o[0] == "play"]
        moves = [o for o in opts if o[0] == "move"]
        acts = [o for o in opts if o[0] == "act"]
        # 1. free value: abilities that draw/ramp (Hook), equip
        for o in acts:
            if isinstance(o[1], int):
                src = g.obj(o[1])
                if src is not None and src.cname == "Baited Hook":
                    return o
        # 2. attack or conquer
        best, bv = None, 0.0
        for o in moves:
            units = [g.obj(u) for u in o[1]]
            dest = o[2]
            if dest == "base":
                continue
            b = g.bfs[dest]
            mine, theirs = s.combat_margin(g, pid, dest, units)
            if theirs == 0:
                v = 5.0 if b.ctrl != pid else 0.0
            elif mine > theirs:
                v = 3.0 + 0.2 * (mine - theirs)
            else:
                v = -1.0
            if TEMPO and b.ctrl == 1 - pid and g.p[1 - pid].points >= g.victory - 1 and mine > theirs:
                v += 4.0                               # casser la tenue gagnante avant de conquérir ailleurs
            v -= 0.1 * len(units)
            if v > bv:
                best, bv = o, v
        if best is not None:
            return best
        # 3. play the biggest unit we can
        units = [o for o in plays if s.is_unit(g, o)]
        if units:
            units.sort(key=lambda o: -(s.card_of(g, o).spec["might"] or 0))
            return units[0]
        # 4. gear / spells that remove a blocker
        if plays and s.cfg.get("pol_keep"):
            # (essai) garder les sorts [Reaction] pour les fenêtres de réaction au lieu de les jeter dans son tour
            plays = [o for o in plays if not s.reaction_card(g, o)]
        if plays:
            return plays[0]
        for o in acts:
            return o
        return ("end",)

    def reaction_card(s, g, o):
        c = s.card_of(g, o)
        return c is not None and c.spec["type"] == "Spell" and "Reaction" in c.spec["keywords"]

    def is_unit(s, g, o):
        c = s.card_of(g, o)
        return c is not None and c.spec["type"] == "Unit"

    def card_of(s, g, o):
        for pl in g.p:
            for z in (pl.hand, pl.champ, pl.trash):
                for c in z:
                    if c.uid == o[1]:
                        return c
        for b in g.bfs:
            for c in b.facedowns:
                if c.uid == o[1]:
                    return c
        return None

    def react(s, g, d):
        pid = d.player
        if g.sd is None or g.sd.stage != "open":
            return ("pass",)
        bf = g.sd.bf
        mine, theirs = s.combat_margin(g, pid, bf)
        plays = [o for o in d.options if o[0] == "play"]
        if not plays:
            return ("pass",)
        losing = mine <= theirs and g.units(pid, bf)
        if losing and g.rng.random() < 0.8:
            return plays[0]
        return ("pass",)


def rollout_policy(g, horizon=2, max_steps=600, cfg=None):
    """Play on with the cheap policy until `horizon` turns have passed (or the game ends)."""
    pol = PolicyAgent()
    if cfg:
        pol.cfg = cfg
    stop = g.turn_no + horizon
    for _ in range(max_steps):
        d = g.advance()
        if d is None:
            return
        if d.kind == "main" and g.turn_no > stop:
            return
        g.apply(pol.decide(g, d))
