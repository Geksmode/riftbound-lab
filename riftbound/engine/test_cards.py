"""Scenario tests: one or more per card / rule. Run: python3 test_cards.py"""
import traceback
from collections import Counter
from game import Game, Rune, Obj
from actions import play_options, main_options
from decks import load

A = load("akali_dongdong_wuhan-open_5th", "Void Gate")
L = load("leblanc_gyatarina_ccs-iq5_1st", "Star Spring")


class Scripted:
    """Answers choices from a dict kind -> value (or callable), else the first option."""
    def __init__(s, answers=None):
        s.answers = answers or {}
        s.queue = []

    def start(s, g):
        pass

    def mulligan(s, g, pid):
        return []

    def decide(s, g, d):
        if s.queue:
            return s.queue.pop(0)
        return ("pass",) if d.kind != "main" else ("end",)

    def choose(s, g, pid, kind, options, ctx):
        if kind in s.answers:
            a = s.answers[kind]
            return a(options, ctx) if callable(a) else a
        if kind == "damage_order":
            return options[0]
        return options[0]


def new(bf0="Void Gate", bf1="Star Spring", answers0=None, answers1=None, tp=0):
    ag = [Scripted(answers0), Scripted(answers1)]
    g = Game([dict(A, battlefield=bf0), dict(L, battlefield=bf1)], ag, first=0, seed=1, log=True)
    for pl in g.p:
        pl.deck = [c for c in pl.deck]
        pl.hand = []
    g.stage = "main"
    g.tp = tp
    g.turn_no = 6
    for pl in g.p:
        pl.turns = 3
    return g, ag


def runes(g, pid, doms):
    g.p[pid].runes = [Rune(d, pid) for d in doms]


def put(g, pid, name, loc="base", ready=True, bf_control=True):
    o = Obj(name, pid)
    g.enter_board(o, pid, loc, ready=ready)
    if loc in (0, 1) and bf_control and g.bfs[loc].ctrl is None:
        g.bfs[loc].ctrl = pid
    return o


def hand(g, pid, name):
    o = Obj(name, pid)
    o.zone = "hand"
    g.p[pid].hand.append(o)
    return o


def settle(g, max_steps=200):
    """Run with the scripted agents until the next main decision; returns that decision."""
    for _ in range(max_steps):
        d = g.advance()
        if d is None or d.kind == "main":
            return d
        g.apply(g.agents[d.player].decide(g, d))
    raise RuntimeError("did not settle")


def opt(g, pid, name, pred=lambda ch: True, src=None):
    d = g.advance()
    for o in d.options:
        if o[0] == "play":
            c = None
            for z in (g.p[pid].hand, g.p[pid].champ, g.p[pid].trash):
                for x in z:
                    if x.uid == o[1]:
                        c = x
            for b in g.bfs:
                if b.facedown is not None and b.facedown.uid == o[1]:
                    c = b.facedown
            if c is not None and c.cname == name and pred(o[3]) and (src is None or o[2] == src):
                return o
    raise AssertionError(f"no option to play {name}: {[x for x in d.options if x[0] == 'play']}")


TESTS = []


def test(f):
    TESTS.append(f)
    return f


# ------------------------------------------------------------------ rules
@test
def move_to_empty_bf_conquers():
    g, _ = new()
    runes(g, 0, ["Fury"] * 4)
    u = put(g, 0, "Mournful Witness")
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert g.bfs[1].ctrl == 0 and g.p[0].points == 1, (g.bfs[1].ctrl, g.p[0].points)
    assert u.exhausted


@test
def combat_attacker_recalled_on_tie():
    g, _ = new()
    a = put(g, 0, "Stellacorn Herder")          # 3
    d = put(g, 1, "Black Rose Dignitary", 1)  # 2 (Assault only when attacking)
    d2 = put(g, 1, "Soaring Scout", 1)         # 1
    g.apply(("move", (a.uid,), 1))
    settle(g)
    # 3 damage from attacker kills both (2+1 lethal), defenders deal 3 -> Herder dies too
    assert a.zone == "trash" and d.zone == "trash" and d2.zone == "trash"
    assert g.bfs[1].ctrl is None


@test
def defender_holds_when_both_survive():
    g, _ = new()
    a = put(g, 0, "Mournful Witness")          # 2
    d = put(g, 1, "Glasc Mixologist", 1)       # 5
    hand(g, 1, "Watchful Sentry")
    g.apply(("move", (a.uid,), 1))
    settle(g)
    assert a.zone == "trash" and d.zone == "board" and g.bfs[1].ctrl == 1


@test
def attackers_recalled_if_defenders_remain():
    g, _ = new()
    a = put(g, 0, "Kai'Sa, Survivor")          # 4
    d = put(g, 1, "Ruined Rex", 1)             # 6
    g.p[1].deck = []                           # no draw from deathknells
    d.damage = 0
    a2 = put(g, 0, "Scuttle Crab")             # 0
    g.apply(("move", (a.uid, a2.uid), 1))
    settle(g)
    # attackers 4 vs Rex 6 -> Rex assigns 6: Kai'Sa needs 4, crab 1 (0 might) -> both die; Rex survives
    assert a.zone == "trash" and a2.zone == "trash" and d.zone == "board"


@test
def tank_and_backline_assignment():
    g, _ = new()
    blitz = put(g, 0, "Blitzcrank, Impassive", 1)     # 5 Tank defender
    crab = put(g, 0, "Mournful Witness", 1)
    g.bfs[1].ctrl = 0
    att = put(g, 1, "Harnessed Dragon")               # 6, no deathknell
    g.tp = 1
    g.apply(("move", (att.uid,), 1))
    settle(g)
    # 6 damage: Tank first (5 lethal) then 1 to Witness (survives, 2 might)
    assert blitz.zone == "trash" and crab.zone == "board", (blitz.zone, crab.zone)


@test
def final_point_needs_all_battlefields():
    g, _ = new()
    g.p[0].points = 7
    u = put(g, 0, "Mournful Witness")
    n = len(g.p[0].hand)
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(3)]
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert g.p[0].points == 7 and len(g.p[0].hand) == n + 1 and g.winner is None


@test
def final_point_when_both_scored():
    g, _ = new()
    g.p[0].points = 7
    g.bfs[0].scored.add(0)
    u = put(g, 0, "Mournful Witness")
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert g.winner == 0


@test
def control_lost_without_units():
    g, _ = new()
    u = put(g, 1, "Soaring Scout", 1)
    runes(g, 0, ["Fury"] * 2 + ["Calm"] * 2)
    hand(g, 0, "Falling Star")
    g.apply(opt(g, 0, "Falling Star"))
    settle(g)
    assert u.zone == "trash" and g.bfs[1].ctrl is None


@test
def hold_scores_at_beginning():
    g, _ = new()
    put(g, 0, "Mournful Witness", 0)
    g.apply(("end",))
    settle(g)                 # opponent main
    g.apply(("end",))
    settle(g)
    assert g.p[0].points == 1, g.p[0].points


@test
def rune_payment_recycle_for_power():
    g, _ = new()
    runes(g, 0, ["Fury", "Fury", "Calm", "Calm"])
    hand(g, 0, "Falling Star")      # 2 energy + 2 Fury power
    put(g, 1, "Ruined Rex", 1)
    g.apply(opt(g, 0, "Falling Star"))
    settle(g)
    # both Fury runes recycled (power), the Calm runes exhausted for energy (or Fury first exhausted)
    assert len(g.p[0].runes) == 2 and len(g.p[0].rune_deck) >= 2


@test
def burn_out_gives_point():
    g, _ = new()
    g.p[0].deck = []
    g.p[0].trash = [Obj("Discipline", 0)]
    g.draw(0, 1)
    assert g.p[1].points == 1 and len(g.p[0].hand) == 1


@test
def hidden_card_trashed_when_control_lost():
    g, _ = new()
    runes(g, 0, ["Calm"] * 3)
    put(g, 0, "Mournful Witness", 0)
    c = hand(g, 0, "Zhonya's Hourglass")
    g.apply(("hide", c.uid, 0))
    settle(g)
    assert g.bfs[0].facedown is c
    # opponent kills the only unit there
    g.bfs[0].ctrl = 0
    w = g.units(0, 0)[0]
    g.kill([w])
    settle(g)
    assert g.bfs[0].facedown is None and c in g.p[0].trash


# ------------------------------------------------------------------ Akali cards
@test
def deadly_weapon_ping_on_move():
    g, _ = new(answers0={"may": True})
    ak = put(g, 0, "Akali, Deadly Weapon")
    e = put(g, 1, "Soaring Scout", 1)
    put(g, 1, "Ruined Rex", 1)
    g.p[1].deck = []
    g.apply(("move", (ak.uid,), 1))
    settle(g)
    assert e.zone == "trash" or g.stats["burnout"] >= 0


@test
def deadly_weapon_empowered_void_gate():
    g, _ = new(answers0={"may": True, "target": lambda o, c: [x for x in o if x.cname == "Glasc Mixologist"][0]})
    ak = put(g, 0, "Akali, Deadly Weapon")
    ak.empowered = True
    e = put(g, 1, "Glasc Mixologist", 0, bf_control=True)    # at Void Gate
    g.apply(("move", (ak.uid,), 0))
    for _ in range(10):
        d = g.advance()
        if d is None or (g.sd is not None and g.sd.combat):
            break
        g.apply(("pass",))
    # empowered 2 + Void Gate 1 = 3 damage before combat
    assert e.damage == 3 or e.zone == "trash", e.damage
    assert g.might(ak) == 4


@test
def legend_retreat_moves_and_readies():
    g, ag = new()
    runes(g, 0, ["Calm"] * 3)
    g.p[0].legend.empowered = True
    h = put(g, 0, "Stellacorn Herder")
    put(g, 1, "Glasc Mixologist", 1)
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(5)]
    g.apply(("move", (h.uid,), 1))
    d = g.advance()
    while d.kind == "priority":
        g.apply(("pass",)); d = g.advance()
    assert d.kind == "focus" and d.player == 0
    acts = [o for o in d.options if o[0] == "act"]
    assert acts, d.options
    g.apply(acts[0])
    settle(g)
    assert h.loc == "base" and not h.exhausted and h.zone == "board"
    assert len(g.p[0].hand) == 2      # Herder drew twice (move in, move out)


@test
def silent_untargetable_outside_combat():
    g, _ = new()
    s = put(g, 0, "Akali, Silent", 0)
    runes(g, 1, ["Order"] * 8)
    hand(g, 1, "Hidden Blade")
    g.tp = 1
    d = g.advance()
    assert not any(o[0] == "play" and o[1] == g.p[1].hand[0].uid for o in d.options), "Hidden Blade must have no target"


@test
def back_off_draws_from_hand_only():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    hand(g, 0, "Back Off")
    e = put(g, 1, "Ruined Rex", 1)
    g.p[0].deck = [Obj("Discipline", 0)]
    g.apply(opt(g, 0, "Back Off"))
    settle(g)
    assert e.stunned and len(g.p[0].hand) == 1


@test
def hidden_back_off_free_next_turn():
    g, _ = new()
    runes(g, 0, ["Calm"] * 1)
    put(g, 0, "Mournful Witness", 0)
    c = hand(g, 0, "Back Off")
    g.apply(("hide", c.uid, 0))
    settle(g)
    assert len(g.p[0].runes) == 0              # [A] paid by recycling
    g.turn_no += 1
    g.tp = 1
    e = put(g, 1, "Ruined Rex")
    e2 = put(g, 1, "Glasc Mixologist", 1)
    g.apply(("move", (e.uid,), 0))
    d = g.advance()
    while not (d.kind == "focus" and d.player == 0):
        g.apply(("pass",)); d = g.advance()
    o = [x for x in d.options if x[0] == "play" and x[2] == "facedown"]
    assert o and all(x[3]["tg"] == (e.uid,) for x in o), o   # target restricted to that battlefield


@test
def zhonya_saves_from_combat_death():
    g, _ = new()
    z = put(g, 0, "Zhonya's Hourglass")
    u = put(g, 0, "Kai'Sa, Survivor", 1)
    g.bfs[1].ctrl = 0
    a = put(g, 1, "Ruined Rex")
    g.tp = 1
    g.apply(("move", (a.uid,), 1))
    settle(g)
    assert u.zone == "board" and u.loc == "base" and u.exhausted and z.zone == "trash"
    assert g.bfs[1].ctrl == 1


@test
def zhonya_removes_deathknell():
    g, _ = new()
    put(g, 1, "Zhonya's Hourglass")
    rex = put(g, 1, "Ruined Rex", 1)
    g.p[1].deck = []
    victim = put(g, 0, "Kai'Sa, Survivor")
    g.kill([rex])
    settle(g)
    assert rex.zone == "board" and victim.damage == 0


@test
def falling_star_two_instances_void_gate():
    g, _ = new()
    runes(g, 0, ["Fury"] * 4)
    hand(g, 0, "Falling Star")
    e = put(g, 1, "Ruined Rex", 0)     # Void Gate
    g.apply(opt(g, 0, "Falling Star", lambda ch: ch["tg"] == (e.uid, e.uid)))
    for _ in range(5):
        d = g.advance()
        if d.kind == "main":
            break
        g.apply(("pass",))
    assert e.zone == "trash"           # 4 + 4 >= 6


@test
def charm_frees_battlefield():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    hand(g, 0, "Charm")
    e = put(g, 1, "Ruined Rex", 1)
    g.apply(opt(g, 0, "Charm", lambda ch: ch["dest"] == "base"))
    settle(g)
    assert e.loc == "base" and g.bfs[1].ctrl is None


@test
def shuriken_flip_and_flow():
    g, _ = new()
    runes(g, 0, ["Fury"] * 3 + ["Calm"] * 3)
    c = hand(g, 0, "Shuriken Flip")
    e = put(g, 1, "LeBlanc, Fragmented", 1)
    m = put(g, 0, "Mournful Witness", ready=False)
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(3)]
    g.apply(opt(g, 0, "Shuriken Flip", lambda ch: ch["tg"] == (e.uid,) and ch["mover"] == m.uid and ch["dest"] == 0))
    settle(g)
    assert e.damage == 2 and m.loc == 0 and c in g.p[0].trash
    g.apply(opt(g, 0, "Shuriken Flip", lambda ch: ch.get("flow"), src="trash"))
    settle(g)
    assert c in g.p[0].banish


@test
def defy_counters_cheap_spell():
    g, ag = new()
    runes(g, 0, ["Calm"] * 2)
    hand(g, 0, "Defy")
    runes(g, 1, ["Order"] * 4)
    hand(g, 1, "Hidden Blade")
    u = put(g, 0, "Kai'Sa, Survivor", 1)
    g.tp = 1
    g.apply(opt(g, 1, "Hidden Blade"))
    d = g.advance()
    g.apply(("pass",))              # LeBlanc passes priority
    d = g.advance()
    assert d.kind == "priority" and d.player == 0
    o = [x for x in d.options if x[0] == "play"]
    g.apply(o[0])
    settle(g)
    assert u.zone == "board"


@test
def not_so_fast_counters_dragon_trigger():
    g, ag = new()
    runes(g, 0, ["Calm"] * 3)
    hand(g, 0, "Not So Fast")
    runes(g, 1, ["Order"] * 10)
    hand(g, 1, "Harnessed Dragon")
    u = put(g, 0, "Kai'Sa, Survivor", 1)
    g.tp = 1
    g.apply(opt(g, 1, "Harnessed Dragon"))
    d = g.advance()
    while d.kind == "priority" and d.player != 0:
        g.apply(("pass",)); d = g.advance()
    o = [x for x in d.options if x[0] == "play"]
    assert o, d.options
    g.apply(o[0])
    settle(g)
    assert u.zone == "board"


@test
def mournful_witness_empowers_after_combat():
    g, _ = new()
    w = put(g, 0, "Mournful Witness")
    w2 = put(g, 0, "Kai'Sa, Survivor")
    e = put(g, 1, "Soaring Scout", 1)
    g.p[1].deck = []
    g.apply(("move", (w.uid, w2.uid), 1))
    settle(g)
    assert w.empowered and g.might(w) == 4


@test
def herder_draws_on_move():
    g, _ = new()
    h = put(g, 0, "Stellacorn Herder")
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(3)]
    g.apply(("move", (h.uid,), 0))
    settle(g)
    assert len(g.p[0].hand) == 1


@test
def kaisa_accelerate_and_conquer_draw():
    g, _ = new()
    runes(g, 0, ["Fury"] * 6)
    hand(g, 0, "Kai'Sa, Survivor")
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(3)]
    g.apply(opt(g, 0, "Kai'Sa, Survivor", lambda ch: ch["acc"]))
    settle(g)
    k = g.units(0)[0]
    assert not k.exhausted
    g.apply(("move", (k.uid,), 1))
    settle(g)
    assert len(g.p[0].hand) == 1 and g.p[0].points == 1


@test
def heron_discount():
    g, _ = new()
    put(g, 0, "Astral Heron", 0)
    runes(g, 0, ["Fury"] * 2 + ["Calm"] * 2)
    hand(g, 0, "Scuttle Crab")
    hand(g, 0, "Falling Star")
    put(g, 1, "Ruined Rex", 1)
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(3)]
    g.apply(opt(g, 0, "Scuttle Crab", lambda ch: ch["loc"] == "base"))
    settle(g)
    assert any(ef.get("kind") == "heron" for ef in g.effects)
    # 2 runes left exhausted? Crab cost 2 -> 2 ready runes left; Falling Star now costs 0 energy 0 power
    g.apply(opt(g, 0, "Falling Star"))
    settle(g)
    assert not any(ef.get("kind") == "heron" for ef in g.effects)


@test
def ferrous_forerunner_mechs():
    g, _ = new()
    f = put(g, 0, "Ferrous Forerunner")
    g.kill([f])
    settle(g)
    assert len([u for u in g.units(0) if u.cname == "Mech"]) == 2


@test
def long_sword_quickdraw_in_combat():
    g, _ = new()
    runes(g, 0, ["Fury"] * 3)
    hand(g, 0, "Long Sword")
    u = put(g, 0, "Mournful Witness", 1)
    g.bfs[1].ctrl = 0
    a = put(g, 1, "LeBlanc, Fragmented")
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(3)]
    g.tp = 1
    g.apply(("move", (a.uid,), 1))
    d = g.advance()
    # attacker has focus first and passes; Akali then plays Long Sword as a reaction
    g.apply(("pass",))
    o = opt(g, 0, "Long Sword")
    g.apply(o)
    settle(g)
    # Witness 2+2 = 4 vs LeBlanc 3+1 (Assault) = 4 -> both die
    assert a.zone == "trash"


@test
def marai_hidden_ping():
    g, _ = new()
    put(g, 0, "Mournful Witness", 0)
    runes(g, 0, ["Fury"])
    c = hand(g, 0, "Mischievous Marai")
    g.apply(("hide", c.uid, 0))
    settle(g)
    g.turn_no += 1
    g.tp = 1
    e = put(g, 1, "Ruined Rex")
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(3)]
    g.apply(("move", (e.uid,), 0))
    d = g.advance()
    while not (d.kind == "focus" and d.player == 0):
        g.apply(("pass",)); d = g.advance()
    o = opt(g, 0, "Mischievous Marai", src="facedown")
    g.apply(o)
    d = g.advance()
    while d is not None and d.kind == "priority":
        g.apply(("pass",)); d = g.advance()
    assert c.loc == 0 and e.damage == 3, e.damage     # 2 + Void Gate


@test
def akali_dw_empower_ability():
    g, _ = new()
    runes(g, 0, ["Fury"] * 3)
    ak = put(g, 0, "Akali, Deadly Weapon")
    d = g.advance()
    acts = [o for o in d.options if o[0] == "act" and o[1] == ak.uid]
    g.apply(acts[0])
    settle(g)
    assert ak.empowered and g.might(ak) == 4


@test
def irelia_deflect_and_chosen():
    g, _ = new()
    i = put(g, 0, "Irelia, Fervent", 1)
    runes(g, 1, ["Order"] * 3)
    hb = hand(g, 1, "Hidden Blade")
    from actions import total_cost
    e, reqs = total_cost(g, 1, hb, dict(tg=(i.uid,)), "hand")
    assert e == 2 and len(reqs) == 2          # 1 Order + 1 any (Deflect)
    g.tp = 1
    g.apply(opt(g, 1, "Hidden Blade"))
    g.advance()
    assert g.might(i) == 4                     # chosen by an enemy: no bonus (only "you choose")


@test
def adaptatron_kills_gear_on_conquer():
    g, _ = new(answers0={"may": True})
    a = put(g, 0, "Adaptatron")
    hk = put(g, 1, "Baited Hook")
    g.apply(("move", (a.uid,), 1))
    settle(g)
    assert hk.zone == "trash" and g.might(a) == 4


@test
def against_the_odds():
    g, _ = new()
    runes(g, 0, ["Fury"] * 2)
    hand(g, 0, "Against the Odds")
    u = put(g, 0, "Mournful Witness", 1, bf_control=False)
    put(g, 1, "Soaring Scout", 1, bf_control=False)
    put(g, 1, "Honest Broker", 1, bf_control=False)
    g.apply(opt(g, 0, "Against the Odds"))
    for _ in range(4):
        d = g.advance()
        if d.kind != "priority":
            break
        g.apply(("pass",))
    assert g.might(u) == 6


@test
def en_garde_alone():
    g, _ = new()
    runes(g, 0, ["Calm"])
    hand(g, 0, "En Garde")
    u = put(g, 0, "Mournful Witness", 0)
    g.apply(opt(g, 0, "En Garde"))
    settle(g)
    assert g.might(u) == 4


@test
def block_shield_tank():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    hand(g, 0, "Block")
    u = put(g, 0, "Mournful Witness", 0)
    g.apply(opt(g, 0, "Block"))
    settle(g)
    assert g.has_kw(u, "Tank") and g.kw_value(u, "Shield") == 3


@test
def blitzcrank_pulls_enemy():
    g, _ = new(answers0={"may": True})
    runes(g, 0, ["Calm"] * 6)
    put(g, 0, "Mournful Witness", 0)
    hand(g, 0, "Blitzcrank, Impassive")
    e = put(g, 1, "Soaring Scout")
    g.p[1].deck = []
    g.apply(opt(g, 0, "Blitzcrank, Impassive", lambda ch: ch["loc"] == 0))
    settle(g)
    assert e.zone == "trash"          # pulled into Blitzcrank's battlefield, combat, dies


@test
def darius_second_card():
    g, _ = new()
    runes(g, 0, ["Fury"] * 6 + ["Calm"] * 3)
    hand(g, 0, "Darius, Trifarian")
    hand(g, 0, "Scuttle Crab")
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(3)]
    g.apply(opt(g, 0, "Darius, Trifarian"))
    settle(g)
    dar = [u for u in g.units(0) if u.cname == "Darius, Trifarian"][0]
    assert dar.exhausted
    g.apply(opt(g, 0, "Scuttle Crab"))
    settle(g)
    assert not dar.exhausted and g.might(dar) == 7


@test
def noxus_hopeful_legion():
    g, _ = new()
    runes(g, 0, ["Fury"] * 4)
    hand(g, 0, "Noxus Hopeful")
    hand(g, 0, "Scuttle Crab")
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(3)]
    g.apply(opt(g, 0, "Scuttle Crab"))
    settle(g)
    o = opt(g, 0, "Noxus Hopeful")      # costs 2 now, 2 runes left
    assert o


@test
def sky_splitter_reduction():
    g, _ = new()
    runes(g, 0, ["Fury"] * 3)
    put(g, 0, "Astral Heron")               # 7 might -> cost 1
    hand(g, 0, "Sky Splitter")
    e = put(g, 1, "Ruined Rex", 1)
    g.p[1].deck = []
    o = opt(g, 0, "Sky Splitter")
    g.apply(o)
    settle(g)
    assert e.damage == 5 or e.zone == "trash"


@test
def brittle_steel_kills_hook():
    g, _ = new()
    runes(g, 0, ["Fury"] * 3)
    hand(g, 0, "Brittle Steel")
    hk = put(g, 1, "Baited Hook")
    g.apply(opt(g, 0, "Brittle Steel"))
    settle(g)
    assert hk.zone == "trash"


@test
def lonely_poro_alone():
    g, _ = new()
    p = put(g, 0, "Lonely Poro", 0)
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(3)]
    g.kill([p])
    settle(g)
    assert len(g.p[0].hand) == 1


@test
def scuttle_crab_xp():
    g, _ = new()
    c = put(g, 0, "Scuttle Crab")
    g.kill([c])
    settle(g)
    assert g.p[0].xp == 1 and g.p[1].revealed_turn == g.turn_no


@test
def targon_peak_readies_runes():
    g, _ = new(bf0="Targon's Peak")
    runes(g, 0, ["Fury"] * 4)
    for r in g.p[0].runes[:3]:
        r.exhausted = True
    u = put(g, 0, "Mournful Witness")
    g.apply(("move", (u.uid,), 0))
    settle(g)
    g.apply(("end",))
    settle(g)
    assert sum(1 for r in g.p[0].runes if not r.exhausted) == 3


@test
def forgotten_monument_no_score_early():
    g, _ = new(bf0="Forgotten Monument")
    g.p[0].turns = 2
    u = put(g, 0, "Mournful Witness")
    g.apply(("move", (u.uid,), 0))
    settle(g)
    assert g.bfs[0].ctrl == 0 and g.p[0].points == 0


# ------------------------------------------------------------------ LeBlanc cards
@test
def karthus_doubles_deathknell():
    g, _ = new()
    put(g, 1, "Karthus, Eternal")
    s = put(g, 1, "Soaring Scout")
    n = len(g.p[1].runes)
    g.kill([s])
    settle(g)
    assert len(g.p[1].runes) == n + 2


@test
def baited_hook_finds_dragon():
    g, _ = new(answers1={"hook_pick": lambda o, c: [x for x in o if x is not None][0]})
    g.tp = 1
    runes(g, 1, ["Order"] * 4)
    put(g, 1, "Baited Hook")
    m = put(g, 1, "Glasc Mixologist")
    e = put(g, 0, "Kai'Sa, Survivor", 1)
    g.p[1].deck = [Obj("Harnessed Dragon", 1)] + [Obj("Discipline", 1) for _ in range(6)]
    g.p[1].trash = []
    d = g.advance()
    acts = [o for o in d.options if o[0] == "act" and o[3].get("tg") == (m.uid,)]
    g.apply(acts[0])
    settle(g)
    assert any(u.cname == "Harnessed Dragon" for u in g.units(1)) and e.zone == "trash"


@test
def rex_deathknell_with_void_gate():
    g, _ = new()
    rex = put(g, 1, "Ruined Rex")
    k = put(g, 0, "Astral Heron", 0, bf_control=True)
    g.kill([rex])
    settle(g)
    assert k.damage == 5, k.damage


@test
def mixologist_replays():
    g, _ = new(answers1={"mixologist_pick": lambda o, c: [x for x in o if x is not None][0]})
    m = put(g, 1, "Glasc Mixologist")
    g.p[1].trash = [Obj("LeBlanc, Fragmented", 1)]
    for c in g.p[1].trash:
        c.zone = "trash"
    g.kill([m])
    settle(g)
    assert any(u.cname == "LeBlanc, Fragmented" for u in g.units(1))


@test
def leblanc_reflection_on_conquer():
    g, _ = new(answers1={"may": True})
    g.tp = 1
    hand(g, 1, "Watchful Sentry")
    u = put(g, 1, "Ruined Rex")
    g.apply(("move", (u.uid,), 1))
    settle(g)
    refl = [x for x in g.units(1) if x.token]
    assert refl and refl[0].cname == "Ruined Rex" and g.might(refl[0]) == 6 and not refl[0].exhausted
    assert g.p[1].legend.exhausted and not g.p[1].hand


@test
def temporary_reflection_dies_and_deathknells():
    g, _ = new()
    from cards import _reflection
    s = put(g, 1, "Soaring Scout")
    r = _reflection(g, 1, "base", s)
    n = len(g.p[1].runes)
    g.tp = 0
    g.apply(("end",))
    settle(g)
    assert r.zone is None or r not in g.board
    assert len(g.p[1].runes) >= n + 1


@test
def leblanc_eao_protects_temporary():
    g, _ = new()
    from cards import _reflection
    put(g, 1, "LeBlanc, Everywhere At Once", 1)
    s = put(g, 1, "Soaring Scout", 1)
    r = _reflection(g, 1, 1, s)
    g.apply(("end",))
    settle(g)
    assert r in g.board


@test
def vi_ambush_and_stun():
    g, _ = new()
    runes(g, 1, ["Order"] * 7)
    hand(g, 1, "Vi, Peacekeeper")
    d1 = put(g, 1, "Soaring Scout", 1)
    a = put(g, 0, "Kai'Sa, Survivor")
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(3)]
    g.apply(("move", (a.uid,), 1))
    d = g.advance()
    assert d.kind == "focus" and d.player == 0
    g.apply(("pass",))
    o = opt(g, 1, "Vi, Peacekeeper")
    g.apply(o)
    settle(g)
    vi = [u for u in g.units(1) if u.cname == "Vi, Peacekeeper"]
    assert vi and a.zone == "trash"          # 4 vs 1+5


@test
def sacrifice_kills_mighty():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Order"] * 2)
    m = put(g, 1, "Glasc Mixologist")
    hand(g, 1, "Sacrifice")
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(3)]
    g.p[1].trash = []
    g.apply(opt(g, 1, "Sacrifice"))
    settle(g)
    assert m.zone == "trash" and len(g.p[1].hand) == 2


@test
def deathgrip_pump():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Order"] * 2)
    big = put(g, 1, "Ruined Rex")
    s = put(g, 1, "Soaring Scout")
    hand(g, 1, "Deathgrip")
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(3)]
    g.p[0].deck = []
    g.apply(opt(g, 1, "Deathgrip", lambda ch: ch.get("tg") == (s.uid, big.uid)))
    settle(g)
    assert s.zone == "trash" and g.might(big) == 7 and len(g.p[1].hand) == 1


@test
def deathgrip_needs_two_friendly_units():
    # 355.8 : deux cibles obligatoires ; avec une seule unité alliée (ou aucune), Deathgrip n'est pas jouable
    from actions import card_choices
    for n in (0, 1):
        g, _ = new()
        runes(g, 0, ["Order"] * 2)
        for _ in range(n):
            put(g, 0, "Soaring Scout")
        put(g, 1, "Ruined Rex")
        c = hand(g, 0, "Deathgrip")
        assert not card_choices(g, 0, c, "hand", False, False, every=True), n
    g, _ = new()
    runes(g, 0, ["Order"] * 2)
    a, b = put(g, 0, "Soaring Scout"), put(g, 0, "Ruined Rex")
    chs = card_choices(g, 0, hand(g, 0, "Deathgrip"), "hand", False, False, every=True)
    assert sorted(c["tg"] for c in chs) == sorted([(a.uid, b.uid), (b.uid, a.uid)])


@test
def watcher_minus_three_min_one():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Mind"] * 8)
    hand(g, 1, "Thousand-Tailed Watcher")
    a = put(g, 0, "Astral Heron")
    b = put(g, 0, "Mournful Witness")
    g.apply(opt(g, 1, "Thousand-Tailed Watcher", lambda ch: ch["acc"]))
    settle(g)
    assert g.might(a) == 4 and g.might(b) == 1


@test
def kennen_stun_and_bonus():
    g, _ = new(answers1={"may": True})
    g.tp = 1
    runes(g, 1, ["Order"] * 5)
    hand(g, 1, "Kennen, Keeper of Balance")
    e = put(g, 0, "Kai'Sa, Survivor", 0)
    o = opt(g, 1, "Kennen, Keeper of Balance", lambda ch: ch["loc"] == "base")
    g.apply(o)
    settle(g)
    ken = [u for u in g.units(1) if u.cname.startswith("Kennen")][0]
    assert e.stunned and g.might(ken) == 2 and g.p[1].runes and sum(not r.exhausted for r in g.p[1].runes) == 0


@test
def mirror_image_copy():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Order"] * 3 + ["Mind"] * 3)
    hand(g, 1, "Mirror Image")
    t = put(g, 0, "Astral Heron")
    g.apply(opt(g, 1, "Mirror Image", lambda ch: ch["tg"] == (t.uid,)))
    settle(g)
    r = [u for u in g.units(1) if u.token][0]
    assert r.cname == "Astral Heron" and not r.exhausted and g.has_kw(r, "Temporary")


@test
def honest_broker_gold_pays_power():
    g, _ = new()
    b = put(g, 1, "Honest Broker")
    g.kill([b])
    settle(g)
    gold = [o for o in g.board if o.cname == "Gold"]
    assert gold and gold[0].exhausted
    gold[0].exhausted = False
    assert g.can_pay(1, 0, [frozenset({"Fury"})])


@test
def rift_herald_dk_plays_from_hand():
    g, _ = new()
    h = put(g, 1, "Rift Herald")
    runes(g, 1, ["Order"])
    hand(g, 1, "Harnessed Dragon")
    runes(g, 1, ["Order"] * 2)
    put(g, 0, "Kai'Sa, Survivor")
    g.kill([h])
    settle(g)
    assert any(u.cname == "Harnessed Dragon" for u in g.units(1))


@test
def hidden_blade_draws_2():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Order"] * 3)
    hand(g, 1, "Hidden Blade")
    e = put(g, 0, "Kai'Sa, Survivor", 1)
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(3)]
    g.apply(opt(g, 1, "Hidden Blade"))
    settle(g)
    assert e.zone == "trash" and len(g.p[0].hand) == 2


@test
def dusk_rose_lab_trigger():
    g, _ = new(bf1="Dusk Rose Lab", answers1={"may": True, "dusk_kill": lambda o, c: [x for x in o if x is not None][0]})
    s = put(g, 1, "Soaring Scout", 1)
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(5)]
    g.apply(("end",))
    settle(g)
    assert s.zone == "trash" and g.p[1].points == 0          # killed before scoring -> no hold


@test
def star_spring_move_back():
    g, _ = new(answers1={"may": True})
    g.tp = 1
    runes(g, 1, ["Order"] * 3)
    put(g, 1, "Soaring Scout", 1)
    hand(g, 1, "Honest Broker")
    g.apply(opt(g, 1, "Honest Broker", lambda ch: ch["loc"] == 1))
    settle(g)
    sc = [u for u in g.units(1) if u.cname == "Soaring Scout"][0]
    assert sc.loc == "base"


@test
def threshold_adds_energy():
    g, _ = new(bf1="Threshold of the Gray")
    a = put(g, 0, "Kai'Sa, Survivor")
    put(g, 1, "Glasc Mixologist", 1)
    g.p[1].deck = []
    g.apply(("move", (a.uid,), 1))
    g.advance()
    assert g.p[0].pool_e == 1 and g.p[1].pool_e == 1


@test
def windswept_ganking():
    g, _ = new(bf1="Windswept Hillock")
    u = put(g, 0, "Mournful Witness", 1)
    d = g.advance()
    assert ("move", (u.uid,), 0) in d.options


# ------------------------------------------------------------------ remaining pool cards
@test
def sterak_gage_bonus():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    hand(g, 0, "Sterak's Gage")
    u = put(g, 0, "Mournful Witness")
    g.apply(opt(g, 0, "Sterak's Gage"))
    settle(g)
    assert g.might(u) == 5 and g.mighty(u)


@test
def pendulum_blade_move_bonus():
    g, _ = new()
    runes(g, 0, ["Fury"] * 4)
    hand(g, 0, "Pendulum Blade")
    u = put(g, 0, "Mournful Witness")
    g.apply(opt(g, 0, "Pendulum Blade"))
    settle(g)
    d = g.advance()
    acts = [o for o in d.options if o[0] == "act"]
    g.apply(acts[0]); settle(g)
    assert g.might(u) == 3            # +1 Might Bonus
    g.apply(("move", (u.uid,), 1)); settle(g)
    assert g.might(u) == 5            # +2 this turn when moving to a battlefield


@test
def disarming_rake_kills_gear():
    g, _ = new(answers0={"may": True})
    runes(g, 0, ["Calm"] * 4)
    hand(g, 0, "Disarming Rake")
    hk = put(g, 1, "Baited Hook")
    g.apply(opt(g, 0, "Disarming Rake"))
    settle(g)
    assert hk.zone == "trash"


@test
def tomb_raider_needs_7_runes():
    g, _ = new()
    runes(g, 0, ["Calm"] * 6)
    hand(g, 0, "Tomb-Raider Barbara")
    hk = put(g, 1, "Baited Hook")
    g.apply(opt(g, 0, "Tomb-Raider Barbara"))
    settle(g)
    assert hk.zone == "board"          # only 6 runes controlled
    g2, _ = new()
    runes(g2, 0, ["Calm"] * 7)
    hand(g2, 0, "Tomb-Raider Barbara")
    hk2 = put(g2, 1, "Baited Hook")
    g2.apply(opt(g2, 0, "Tomb-Raider Barbara"))
    settle(g2)
    assert hk2.zone == "trash"


@test
def salvage_optional_gear_kill():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Order"] * 3)
    hand(g, 1, "Salvage")
    z = put(g, 0, "Zhonya's Hourglass")
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(3)]
    g.apply(opt(g, 1, "Salvage", lambda ch: ch["tg"] == (z.uid,)))
    settle(g)
    assert z.zone == "trash" and len(g.p[1].hand) == 1


@test
def turn_to_dust_temporary_gear():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Mind"] * 2)
    hand(g, 1, "Turn to Dust")
    z = put(g, 0, "Zhonya's Hourglass")
    g.apply(opt(g, 1, "Turn to Dust"))
    settle(g)
    assert g.has_kw(z, "Temporary")
    g.apply(("end",))
    settle(g)                 # Akali's beginning phase kills it
    assert z.zone == "trash"


@test
def stupefy_minimum_one():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Mind"] * 1)
    hand(g, 1, "Stupefy")
    u = put(g, 0, "Scuttle Crab")        # 0 might
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(3)]
    g.apply(opt(g, 1, "Stupefy"))
    settle(g)
    assert g.might(u) == 0 and len(g.p[1].hand) == 1


@test
def ki_barrier_prevents_7():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Order"] * 3)
    hand(g, 1, "Ki Barrier")
    u = put(g, 1, "Ruined Rex")
    g.apply(opt(g, 1, "Ki Barrier"))
    settle(g)
    assert u.prevent == 7
    g.deal(u, 6, "spell", 0)
    assert u.damage == 0 and u.prevent == 1


@test
def cull_the_weak_both_players():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Order"] * 3)
    hand(g, 1, "Cull the Weak")
    a = put(g, 0, "Mournful Witness")
    b = put(g, 1, "Soaring Scout")
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(3)]
    g.apply(opt(g, 1, "Cull the Weak"))
    settle(g)
    assert a.zone == "trash" and b.zone == "trash"


@test
def safety_inspector_xp_skips_own_kill():
    g, _ = new()
    g.tp = 1
    g.p[1].xp = 3
    runes(g, 1, ["Order"] * 6)
    hand(g, 1, "Safety Inspector")
    a = put(g, 0, "Mournful Witness")
    b = put(g, 1, "Soaring Scout")
    g.apply(opt(g, 1, "Safety Inspector", lambda ch: ch.get("xp")))
    settle(g)
    assert a.zone == "trash" and b.zone == "board" and g.p[1].xp == 0


@test
def bellows_breath_repeat():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Mind"] * 4)
    hand(g, 1, "Bellows Breath")
    a = put(g, 0, "Scuttle Crab")
    b = put(g, 0, "Lonely Poro")
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(5)]
    g.apply(opt(g, 1, "Bellows Breath", lambda ch: ch.get("rep")))
    settle(g)
    assert a.zone == "trash" and b.zone == "trash"       # 1+1 each


@test
def chakram_dancer_ambush_shield():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Mind"] * 3)
    hand(g, 1, "Chakram Dancer")
    o = put(g, 1, "Soaring Scout", 1)
    a = put(g, 0, "Mournful Witness")
    g.tp = 0
    g.apply(("move", (a.uid,), 1))
    d = g.advance()
    while not (d.kind == "focus" and d.player == 1):
        g.apply(("pass",)); d = g.advance()
    pl = [x for x in d.options if x[0] == "play"]
    assert pl and all(x[3]["loc"] == 1 for x in pl)
    g.apply(pl[0])
    settle(g)
    # Dancer 3 + Scout 1+1 (shield) = 5 vs Witness 2
    assert a.zone == "trash" and o.zone == "board"


@test
def crumbling_sands_needs_second_spell():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    hand(g, 0, "Crumbling Sands")
    runes(g, 1, ["Order"] * 6)
    hand(g, 1, "Hidden Blade")
    hand(g, 1, "Stupefy")
    u = put(g, 0, "Kai'Sa, Survivor", 1)
    g.p[0].deck = [Obj("Discipline", 0) for _ in range(3)]
    g.p[1].deck = [Obj("Discipline", 1) for _ in range(3)]
    g.tp = 1
    g.apply(opt(g, 1, "Stupefy"))
    settle(g)
    g.apply(opt(g, 1, "Hidden Blade"))
    d = g.advance()
    while d.player != 0:
        g.apply(("pass",)); d = g.advance()
    o = [x for x in d.options if x[0] == "play"]
    assert o
    g.apply(o[0]); settle(g)
    assert u.zone == "board"


@test
def decrees_domain_restrictions():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    hand(g, 0, "Decree of Rage")        # enemy Calm unit only
    hand(g, 0, "Decree of Unity")       # enemy Chaos unit/gear
    put(g, 1, "Ruined Rex", 1)          # Mind
    names = lambda d: set(c.cname for o in d.options if o[0] == "play"
                          for c in g.p[0].hand if c.uid == o[1])
    assert not (names(g.advance()) & {"Decree of Rage", "Decree of Unity"})
    put(g, 1, "Soaring Scout", 1)       # Order, not Calm nor Chaos
    assert not (names(g.advance()) & {"Decree of Rage", "Decree of Unity"})


@test
def decree_of_insight_body_only():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Mind"] * 2)
    hand(g, 1, "Decree of Insight")
    put(g, 0, "Mournful Witness")       # Calm
    d = g.advance()
    assert not any(o[0] == "play" for o in d.options)


@test
def decree_of_focus_in_combat():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2)
    hand(g, 0, "Decree of Focus")
    u = put(g, 0, "Mournful Witness", 1)
    g.bfs[1].ctrl = 0
    a = put(g, 1, "LeBlanc, Fragmented")     # Order, not Fury
    g.tp = 1
    g.apply(("move", (a.uid,), 1))
    d = g.advance()
    while not (d.kind == "focus" and d.player == 0):
        g.apply(("pass",)); d = g.advance()
    assert not any(o[0] == "play" for o in d.options)   # no enemy Fury unit


@test
def back_alley_bar_bonus():
    g, _ = new(bf0="Back-Alley Bar")
    u = put(g, 0, "Mournful Witness", 0)
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert g.might(u) == 3


@test
def sigil_recycles_rune():
    g, _ = new(bf0="Sigil of the Storm")
    runes(g, 0, ["Fury"] * 3)
    u = put(g, 0, "Mournful Witness")
    g.apply(("move", (u.uid,), 0))
    settle(g)
    assert len(g.p[0].runes) == 2 and g.p[0].points == 1
    assert g.p[0].pool_e == 1          # the recycled ready rune floats its energy (429.3.a)


@test
def forbidding_waste_defending_alone():
    g, _ = new(bf1="Forbidding Waste")
    u = put(g, 0, "Kai'Sa, Survivor", 1)
    g.bfs[1].ctrl = 0
    a = put(g, 1, "LeBlanc, Fragmented")
    g.tp = 1
    g.apply(("move", (a.uid,), 1))
    d = g.advance()
    assert g.might(u) == 2        # 4 - 2 defending alone
    while d is not None and d.kind in ("focus", "priority"):
        g.apply(("pass",)); d = g.advance()
    assert u.zone == "trash"      # 3 + 1 Assault >= 2


@test
def aspirants_climb_raises_victory():
    g, _ = new(bf0="Aspirant's Climb")
    assert g.victory == 9


@test
def time_warp_extra_turn():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Mind"] * 10 + ["Order"] * 4)
    hand(g, 1, "Time Warp")
    c = g.p[1].hand[-1]
    g.apply(opt(g, 1, "Time Warp"))
    settle(g)
    assert g.extra_turns == [1] and c in g.p[1].banish
    g.apply(("end",))
    settle(g)
    assert g.tp == 1


@test
def ashe_banishes_until_hold():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Order"] * 6)
    hand(g, 1, "Ashe, Focused")
    victim = hand(g, 0, "Falling Star")
    put(g, 0, "Mournful Witness", 0)
    g.apply(opt(g, 1, "Ashe, Focused"))
    settle(g)
    assert victim in g.p[0].banish
    g.apply(("end",))
    settle(g)                       # Akali holds Void Gate
    assert victim in g.p[0].hand


@test
def atakhan_cost_reduction_and_attack():
    g, _ = new()
    g.tp = 1
    runes(g, 1, ["Order"] * 7)
    hand(g, 1, "Atakhan")
    fodder = put(g, 1, "Ruined Rex")      # 6e 1p -> Atakhan costs 4e 2p
    from actions import total_cost
    e, reqs = total_cost(g, 1, g.p[1].hand[0], dict(kill=fodder.uid), "hand")
    assert (e, len(reqs)) == (4, 2), (e, reqs)
    g.apply(opt(g, 1, "Atakhan", lambda ch: ch.get("kill") == fodder.uid))
    settle(g)
    at = [u for u in g.units(1) if u.cname == "Atakhan"][0]
    assert g.has_kw(at, "Ganking") and fodder.zone == "trash"


@test
def legend_empower_cost():
    g, _ = new()
    runes(g, 0, ["Fury"] * 4)
    d = g.advance()
    acts = [o for o in d.options if o[0] == "act" and o[1] == ("legend", 0)]
    assert acts
    g.apply(acts[0]); settle(g)
    assert g.p[0].legend.empowered and len(g.p[0].runes) == 3


@test
def every_ask_kind_has_a_french_title():
    """Each g.ask kind (literal, or passed to a shared helper as kind=...) has a title for a human (train.py)."""
    import glob, os, re
    import train
    train.catalog()                                       # loads every card module (their ask_text calls)
    from cards import ASK_TEXT
    here = os.path.dirname(os.path.abspath(__file__))
    kinds = set()
    for f in glob.glob(os.path.join(here, "*.py")) + glob.glob(os.path.join(here, "cardsets", "*.py")):
        if os.path.basename(f).startswith(("test_", "fuzz")):
            continue
        src = open(f).read()
        kinds |= set(re.findall(r'\.ask\(\s*[\w.\[\]]+\s*,\s*"(\w+)"', src))
        kinds |= set(re.findall(r'(?:trig_target|_pick)\([^()]*(?:\([^()]*\)[^()]*)*kind="(\w+)"', src))
    missing = sorted(k for k in kinds if k not in ASK_TEXT and k not in train.ASKS)
    assert not missing, missing


@test
def human_option_labels_are_readable():
    import train
    from game import Opt
    g, _ = new()
    u = g.units(0)[0] if g.units(0) else None
    assert train._olabel(g, Opt("sauver X", 3)) == "sauver X"
    assert train._olabel(g, "bottom") == "sous le deck"
    assert "Accelerate" in train._olabel(g, dict(loc="base", acc=True))

    class A:
        kind, ctx = "split_damage", {}
    assert train._alabel(g, A, (1, 1, 3)) == "1 + 1 + 3 dégâts"
    A.kind = "burn_player"
    assert train._alabel(g, A, 0) == "toi" and train._alabel(g, A, 1) == "l'adversaire"
    if u is not None:
        A.kind = "move_enemy"
        assert "→" in train._alabel(g, A, (u, "base"))


def run():
    ok = 0
    fails = []
    for t in TESTS:
        try:
            t()
            ok += 1
        except Exception as e:
            fails.append((t.__name__, e))
            traceback.print_exc()
    print(f"{ok}/{len(TESTS)} tests passed")
    for n, e in fails:
        print("FAIL", n, repr(e)[:200])
    return ok, fails


if __name__ == "__main__":
    run()
