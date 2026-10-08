"""Tests of batch chaos. Run: RB_CARDSETS=chaos python3 cardsets/test_chaos.py
(Zed's Shadow Clone token is registered by cardsets/fury: its own test only checks the token is played.)"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from cards import make_token                            # noqa: E402
from game import ANY                                    # noqa: E402

T = Suite("chaos")

V3, V4, V5, V8 = "Shipyard Skulker", "Vanguard Sergeant", "Playful Phantom", "Mega-Mech"     # vanilla units


def N(**kw):
    """new() with two battlefields without abilities (no Void Gate bonus damage, no Star Spring trigger)."""
    return new(bf0="Test Field A", bf1="Test Field B", **kw)


def R(g, pid, n=12, dom="Chaos"):
    runes(g, pid, [dom] * n)


def play(g, pid, name, pred=lambda ch: True, src=None):
    g.apply(opt(g, pid, name, pred, src))
    return settle(g)


def base(ch):
    return ch.get("loc") == "base" and not ch.get("acc")


def unit(g, name, pid=None):
    us = [u for u in g.units(pid) if u.cname == name]
    assert us, (name, g.board)
    return us[-1]


def facedown(g, pid, name, bf):
    """A card hidden last turn at bf (bf controlled by pid, with one of pid's units there)."""
    if not g.units(pid, bf):
        put(g, pid, V3, bf)
    g.bfs[bf].ctrl = pid
    o = Obj(name, pid)
    o.zone = "facedown"
    o.hidden_turn = g.turn_no - 1
    o.hidden_bf = bf
    g.bfs[bf].facedown = o
    return o


def trash(g, pid, name):
    o = Obj(name, pid)
    o.zone = "trash"
    g.p[pid].trash.append(o)
    return o


def respond(g, pid, name, pred=lambda ch: True, src=None):
    """P(1-pid) has just played something: it passes priority, then pid plays name in reaction."""
    d = g.advance()
    assert d.kind == "priority" and d.player == 1 - pid, d
    g.apply(("pass",))
    g.apply(opt(g, pid, name, pred, src))
    return settle(g)


# ====================================================================== A
@T.test
def abandon_returns_countered_spell_to_hand():
    g, _ = N()
    R(g, 0, 4); R(g, 1, 2)
    e = put(g, 1, V3, 0)
    rb = hand(g, 0, "Rebuke")
    hand(g, 1, "Abandon")
    deck_top(g, 1, [V4])
    g.apply(opt(g, 0, "Rebuke", lambda ch: ch.get("tg") == (e.uid,)))
    respond(g, 1, "Abandon")
    assert e.zone == "board" and rb in g.p[0].hand and rb not in g.p[0].trash


@T.test
def acceptable_losses_each_player_kills_a_gear():
    g, _ = N()
    R(g, 0, 2)
    s0 = put(g, 0, "Seal of Discord")
    s1 = put(g, 1, "Cull")
    hand(g, 0, "Acceptable Losses")
    play(g, 0, "Acceptable Losses")
    assert s0.zone == "trash" and s1.zone == "trash"


@T.test
def ancient_warmonger_assault_per_enemy_here():
    g, _ = N()
    w = put(g, 0, "Ancient Warmonger", 0)
    put(g, 1, V3, 0); put(g, 1, V3, 0); put(g, 1, V3, 1)
    assert g.kw_value(w, "Assault") == 2
    w.loc = "base"
    assert g.kw_value(w, "Assault") == 0


@T.test
def angler_beast_bounces_small_units():
    g, _ = N()
    R(g, 0)
    small = put(g, 1, "Recruit", 1)
    big = put(g, 1, V3, 1)
    mine = put(g, 0, "Sand Soldier")
    hand(g, 0, "Angler Beast")
    play(g, 0, "Angler Beast", base)
    assert small.zone == "hand" and mine.zone == "hand" and big.zone == "board"
    assert unit(g, "Angler Beast").zone == "board"


@T.test
def annie_returns_spell_from_trash():
    g, _ = N()
    R(g, 0)
    s = trash(g, 0, "Gust")
    trash(g, 0, V3)
    hand(g, 0, "Annie, Stubborn")
    play(g, 0, "Annie, Stubborn", base)
    assert s in g.p[0].hand


# ====================================================================== B
@T.test
def beast_below_returns_friend_and_enemy():
    g, _ = N()
    R(g, 0)
    f = put(g, 0, V3)
    e = put(g, 1, V4, 1)
    hand(g, 0, "Beast Below")
    play(g, 0, "Beast Below", base)
    assert f.zone == "hand" and e.zone == "hand" and unit(g, "Beast Below").zone == "board"


@T.test
def bewitching_spirit_opponent_discards():
    g, _ = N()
    R(g, 0)
    c = hand(g, 1, V3)
    hand(g, 0, "Bewitching Spirit")
    play(g, 0, "Bewitching Spirit", base)
    assert c in g.p[1].trash


@T.test
def black_market_broker_gold_from_facedown():
    g, _ = N()
    put(g, 0, "Black Market Broker")
    facedown(g, 0, "Teemo, Scout", 0)
    play(g, 0, "Teemo, Scout", src="facedown")
    golds = [o for o in g.gear(0) if o.cname == "Gold"]
    assert len(golds) == 1 and golds[0].exhausted


@T.test
def blast_cone_moves_and_stuns():
    g, _ = N()
    R(g, 0)
    e = put(g, 1, V4, 1)
    hand(g, 0, "Blast Cone")
    play(g, 0, "Blast Cone")
    cone = [o for o in g.gear(0) if o.cname == "Blast Cone"][0]
    assert e.loc == "base" and e.stunned and cone.exhausted


@T.test
def bone_skewer_opponent_plays_unit_stunned():
    g, _ = N()
    R(g, 0)
    put(g, 0, V8, 0)
    c = hand(g, 1, V3)
    hand(g, 0, "Bone Skewer")
    play(g, 0, "Bone Skewer", lambda ch: ch["bf"] == 0)
    log = "\n".join(g.lines)
    assert "P1 plays Shipyard Skulker from hand (loc=0 free=True)" in log and f"{c} is stunned" in log
    assert "attackers 0 vs defenders 8" in log and c.zone == "trash"


@T.test
def boots_of_swiftness_bonus_and_ganking():
    g, _ = N()
    R(g, 0, 4)
    u = put(g, 0, V3, 0)
    hand(g, 0, "Boots of Swiftness")
    play(g, 0, "Boots of Swiftness")
    boots = [o for o in g.gear(0) if o.cname == "Boots of Swiftness"][0]
    g.apply([o for o in act_options(g, 0, "Boots of Swiftness") if o[3].get("tg") == (u.uid,)][0])
    settle(g)
    assert boots.attached_to == u.uid and g.might(u) == 5 and g.has_kw(u, "Ganking")


# ====================================================================== C
@T.test
def called_shot_draws_one_recycles_other_with_repeat():
    g, _ = N()
    R(g, 0, 2)
    top = deck_top(g, 0, [V8, V3, V5, V4])
    hand(g, 0, "Called Shot")
    play(g, 0, "Called Shot", lambda ch: ch.get("rep"))
    pl = g.p[0]
    assert top[0] in pl.hand and top[2] in pl.hand          # Mega-Mech, then Playful Phantom
    assert top[1] in pl.deck[-2:] and top[3] in pl.deck[-2:]


@T.test
def cemetery_attendant_returns_unit():
    g, _ = N()
    R(g, 0)
    t = trash(g, 0, V4)
    hand(g, 0, "Cemetery Attendant")
    play(g, 0, "Cemetery Attendant", base)
    assert t in g.p[0].hand


@T.test
def conscription_small_and_with_xp():
    g, _ = N()
    R(g, 0, 14)
    small = put(g, 1, V3, 1)
    big = put(g, 1, V5, 1)
    c = hand(g, 0, "Conscription")
    chs = card_choices(g, 0, c, "hand", False, False)
    assert [ch["tg"] for ch in chs] == [(small.uid,)]
    g.p[0].xp = 5
    play(g, 0, "Conscription", lambda ch: ch.get("tg") == (big.uid,))
    assert big.ctrl == 0 and big.loc == "base" and big.exhausted and g.p[0].xp == 0


@T.test
def corrupt_enforcer_discards_on_move_draws_on_win():
    g, _ = N()
    u = put(g, 0, "Corrupt Enforcer")
    put(g, 1, "Recruit", 1)
    c = hand(g, 0, V3)
    n = len(g.p[0].hand)
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert c in g.p[0].trash and g.bfs[1].ctrl == 0
    assert len(g.p[0].hand) == n - 1 + 1             # discarded 1, drew 1 for the win


@T.test
def crescent_guardian_enters_ready_after_spell():
    g, _ = N()
    R(g, 0, 12)
    c = hand(g, 0, "Crescent Guardian")
    assert all(not ch.get("cg") for ch in card_choices(g, 0, c, "hand", False, False))
    g.spells_played[0] = 1
    play(g, 0, "Crescent Guardian", lambda ch: ch.get("cg") and ch["loc"] == "base")
    assert not c.exhausted and len(g.p[0].runes) == 11


@T.test
def cull_gold_on_conquer():
    g, _ = N()
    R(g, 0, 2)
    u = put(g, 0, V3)
    cull = put(g, 0, "Cull")
    from actions import attach as _att
    _att(g, cull, u)
    assert g.might(u) == 4
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert any(o.cname == "Gold" for o in g.gear(0))


@T.test
def cursed_sarcophagus_banishes_and_replays():
    g, _ = N()
    R(g, 0, 12)
    a = trash(g, 0, V3)
    b = trash(g, 0, "Gust")
    hand(g, 0, "Cursed Sarcophagus")
    play(g, 0, "Cursed Sarcophagus")
    assert a in g.p[0].banish and b in g.p[0].trash
    n = sum(1 for r in g.p[0].runes if r.exhausted)
    g.apply(act_options(g, 0, "Cursed Sarcophagus")[0])
    settle(g)
    assert a.zone == "board" and sum(1 for r in g.p[0].runes if r.exhausted) == n + 3


# ====================================================================== D
@T.test
def decree_of_discord_returns_order_units():
    g, _ = N()
    R(g, 0, 2)
    a = put(g, 1, V4, 1)              # Order, 4
    b = put(g, 1, V4, 1)
    c = hand(g, 0, "Decree of Discord")
    chs = card_choices(g, 0, c, "hand", False, False)
    assert all(len(ch["tg"]) <= 1 for ch in chs)       # 4 + 4 > 5
    play(g, 0, "Decree of Discord", lambda ch: ch["tg"] == (a.uid,))
    assert a.zone == "hand" and b.zone == "board"


@T.test
def diana_gets_might_when_spell_played():
    g, _ = N()
    R(g, 0, 4)
    d = put(g, 0, "Diana, No Longer Human")
    hand(g, 0, "Lunar Boon")
    play(g, 0, "Lunar Boon")
    assert g.might(d) == 5


@T.test
def dorans_ring_loot_on_conquer():
    g, _ = N()
    u = put(g, 0, V3)
    r = put(g, 0, "Doran's Ring")
    from actions import attach as _att
    _att(g, r, u)
    c = hand(g, 0, V8)
    n = len(g.p[0].hand)
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert c in g.p[0].trash and len(g.p[0].hand) == n and g.might(u) == 4


@T.test
def downwell_returns_everything():
    g, _ = N()
    R(g, 0, 12)
    a = put(g, 0, V3); b = put(g, 1, V4, 1); gg = put(g, 1, "Cull")
    t = make_token(g, "Recruit", 0)
    hand(g, 0, "Downwell")
    play(g, 0, "Downwell")
    assert a.zone == "hand" and b.zone == "hand" and gg.zone == "hand" and t not in g.board
    assert b in g.p[1].hand and t not in g.p[0].hand


@T.test
def draven_scores_on_win_and_on_combat_death():
    g, _ = N()
    d = put(g, 0, "Draven, Audacious")
    put(g, 1, V3, 1)
    g.apply(("move", (d.uid,), 1))
    settle(g)
    assert g.p[0].points == 2, g.p[0].points          # conquer + Draven
    g2, _ = N()
    d2 = put(g2, 0, "Draven, Audacious")
    put(g2, 1, V8, 1)
    g2.apply(("move", (d2.uid,), 1))
    settle(g2)
    assert d2.zone == "trash" and g2.p[1].points == 1 and g2.p[0].points == 0


# ====================================================================== E
@T.test
def edge_of_night_attaches_from_facedown():
    g, _ = N()
    fd = facedown(g, 0, "Edge of Night", 0)
    u = unit(g, V3, 0)
    play(g, 0, "Edge of Night", src="facedown")
    assert fd.attached_to == u.uid and g.might(u) == 5


@T.test
def ember_monk_buff_on_hidden_play():
    g, _ = N()
    m = put(g, 0, "Ember Monk")
    facedown(g, 0, "Fight or Flight", 0)
    play(g, 0, "Fight or Flight", src="facedown")
    assert g.might(m) == 6


@T.test
def evelynn_pulls_enemy_from_facedown():
    g, _ = N()
    facedown(g, 0, "Evelynn, Entrancing", 0)
    e = put(g, 1, V3, 1)
    play(g, 0, "Evelynn, Entrancing", src="facedown")
    ev = unit(g, "Evelynn, Entrancing")
    assert ev.loc == 0 and g.has_kw(ev, "Backline")
    assert f"{e} moves 1->0" in "\n".join(g.lines)


@T.test
def evershade_stalker_loots():
    g, _ = N()
    R(g, 0)
    c = hand(g, 0, V8)
    hand(g, 0, "Evershade Stalker")
    n = len(g.p[0].hand)
    play(g, 0, "Evershade Stalker", base)
    assert c in g.p[0].trash and len(g.p[0].hand) == n - 1


@T.test
def existential_dread_stun_then_bounce_with_repeat():
    g, _ = N()
    R(g, 1, 4)
    a = put(g, 1, V3)
    put(g, 0, V8, 1)
    hand(g, 0, "Existential Dread")
    R(g, 0, 3)
    g.apply(("end",)); settle(g)                    # P1's turn
    g.apply(("move", (a.uid,), 1))
    d = g.advance()                                 # combat: attacker P1 has focus
    g.apply(("pass",))
    g.apply(opt(g, 0, "Existential Dread", lambda ch: ch.get("rep") and ch.get("tg2") == (a.uid,)))
    settle(g)
    assert a.zone == "hand"


# ====================================================================== F
@T.test
def factory_recall_returns_gear():
    g, _ = N()
    R(g, 0, 2)
    x = put(g, 1, "Cull")
    hand(g, 0, "Factory Recall")
    play(g, 0, "Factory Recall")
    assert x in g.p[1].hand


@T.test
def fading_memories_gives_temporary():
    g, _ = N()
    R(g, 0, 6)
    e = put(g, 1, V5, 1)
    hand(g, 0, "Fading Memories")
    play(g, 0, "Fading Memories", lambda ch: ch["tg"] == (e.uid,))
    assert g.has_kw(e, "Temporary")
    g.apply(("end",)); settle(g)
    assert e.zone == "trash"


@T.test
def fae_porter_brings_a_friend():
    g, _ = N()
    R(g, 0, 2)
    p = put(g, 0, "Fae Porter")
    f = put(g, 0, V3)
    g.apply(("move", (p.uid,), 1))
    settle(g)
    assert p.loc == 1 and f.loc == 1 and len(g.p[0].runes) == 1


@T.test
def fight_or_flight_moves_unit_to_base():
    g, _ = N()
    R(g, 0, 2)
    e = put(g, 1, V3, 1)
    hand(g, 0, "Fight or Flight")
    play(g, 0, "Fight or Flight")
    assert e.loc == "base"


@T.test
def fizz_replays_spell_from_trash_and_recycles_it():
    g, _ = N()
    R(g, 0, 12)
    s = trash(g, 0, "Rebuke")                     # 2 energy, 2 chaos
    e = put(g, 1, V5, 1)
    hand(g, 0, "Fizz, Trickster")
    play(g, 0, "Fizz, Trickster", base)
    assert e.zone == "hand" and s in g.p[0].deck and s not in g.p[0].trash
    assert len(g.p[0].runes) == 12 - 1 - 2         # Fizz's power + Rebuke's power only


@T.test
def flash_moves_two_friends_to_base():
    g, _ = N()
    R(g, 0, 2)
    a = put(g, 0, V3, 1); b = put(g, 0, V4, 1)
    hand(g, 0, "Flash")
    play(g, 0, "Flash", lambda ch: len(ch["tg"]) == 2)
    assert a.loc == "base" and b.loc == "base"


@T.test
def forgotten_relic_burns_unit_and_buffs():
    g, _ = N()
    R(g, 0, 5)
    u = put(g, 0, V3)
    deck_top(g, 0, [V8])
    hand(g, 0, "Forgotten Relic")
    play(g, 0, "Forgotten Relic")
    assert g.p[0].trash[-1].cname == V8 and g.might(u) == 11


# ====================================================================== G
@T.test
def gust_returns_small_unit():
    g, _ = N()
    R(g, 0, 1)
    e = put(g, 1, V3, 1)
    big = put(g, 1, V5, 1)
    c = hand(g, 0, "Gust")
    assert [ch["tg"] for ch in card_choices(g, 0, c, "hand", False, False)] == [(e.uid,)]
    play(g, 0, "Gust")
    assert e.zone == "hand" and big.zone == "board"


@T.test
def gust_monk_paid_cost_gives_assault():
    g, _ = N()
    R(g, 0, 3)
    t = trash(g, 1, V3)
    f = put(g, 0, V4)
    hand(g, 0, "Gust Monk")
    play(g, 0, "Gust Monk", lambda ch: ch.get("gm") and ch["loc"] == "base")
    assert t in g.p[1].banish
    assert any(g.kw_value(u, "Assault") == 2 for u in g.units(0))


@T.test
def hard_bargain_counters_unless_paid():
    g, _ = N(answers0={"pay_or_countered": False})
    R(g, 0, 4); R(g, 1, 2)
    e = put(g, 1, V3, 0)
    rb = hand(g, 0, "Rebuke")
    hand(g, 1, "Hard Bargain")
    g.apply(opt(g, 0, "Rebuke", lambda ch: ch.get("tg") == (e.uid,)))
    respond(g, 1, "Hard Bargain", lambda ch: not ch.get("rep2"))
    assert e.zone == "board" and rb in g.p[0].trash
    g, _ = N()                                      # default: P0 pays 2
    R(g, 0, 6); R(g, 1, 2)
    e = put(g, 1, V3, 0)
    hand(g, 0, "Rebuke")
    hand(g, 1, "Hard Bargain")
    g.apply(opt(g, 0, "Rebuke", lambda ch: ch.get("tg") == (e.uid,)))
    respond(g, 1, "Hard Bargain", lambda ch: not ch.get("rep2"))
    assert e.zone == "hand"


@T.test
def harpoon_squad_might_when_leaving_battlefield():
    g, _ = N()
    h = put(g, 0, "Harpoon Squad", 1)
    g.apply(("move", (h.uid,), "base"))
    settle(g)
    assert g.might(h) == 6


@T.test
def heedless_resurrection_kills_and_replays():
    g, _ = N()
    R(g, 0, 3)
    v = put(g, 0, V4)
    t = trash(g, 0, V3)                             # costs 3, no more than Vanguard Sergeant (4)
    hand(g, 0, "Heedless Resurrection")
    play(g, 0, "Heedless Resurrection", lambda ch: ch["kill"] == v.uid)
    played = g.units(0)                              # the killed Sergeant itself is in the trash: replayed
    assert len(played) == 1 and played[0] is v and t.zone == "trash"


# ====================================================================== I
@T.test
def illaoi_tentacles_and_might():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Illaoi, Prophet of the Great Kraken")
    play(g, 0, "Illaoi, Prophet of the Great Kraken", base)
    il = unit(g, "Illaoi, Prophet of the Great Kraken")
    assert len([u for u in g.units(0) if u.cname == "Tentacle"]) == 1 and g.might(il) == 5
    il.exhausted = False
    g.apply(("move", (il.uid,), 1))
    settle(g)
    assert len([u for u in g.units(0) if u.cname == "Tentacle"]) == 2 and g.might(il) == 6


@T.test
def insightful_investigator_pays_xp():
    g, _ = N()
    R(g, 0)
    g.p[0].xp = 2
    c = hand(g, 1, V8)
    hand(g, 0, "Insightful Investigator")
    play(g, 0, "Insightful Investigator", base)
    assert c in g.p[1].trash and len(g.p[1].hand) == 1 and g.p[0].xp == 0


@T.test
def invert_timelines_discards_and_draws_4():
    g, _ = N()
    R(g, 0)
    a = hand(g, 0, V3); b = hand(g, 1, V4)
    hand(g, 0, "Invert Timelines")
    play(g, 0, "Invert Timelines")
    assert a in g.p[0].trash and b in g.p[1].trash and len(g.p[0].hand) == 4 and len(g.p[1].hand) == 4


@T.test
def isolate_moves_and_draws():
    g, _ = N()
    R(g, 0, 2)
    a = put(g, 1, V3, 1); b = put(g, 1, V4, 1)
    hand(g, 0, "Isolate")
    n = len(g.p[0].hand)
    play(g, 0, "Isolate", lambda ch: ch["tg"] == (a.uid,))
    assert a.loc == "base" and b.loc == 1 and len(g.p[0].hand) == n       # -Isolate +1 draw


# ====================================================================== J / K
@T.test
def jae_medarda_draws_when_chosen():
    g, _ = N()
    R(g, 0, 4)
    j = put(g, 0, "Jae Medarda", 1)
    hand(g, 0, "Ride The Wind")
    n = len(g.p[0].hand)
    play(g, 0, "Ride The Wind", lambda ch: ch["tg"] == (j.uid,) and ch["dest"] == "base")
    assert len(g.p[0].hand) == n and j.loc == "base"


@T.test
def jinx_once_per_discard_action():
    g, _ = N()
    R(g, 0, 4)
    j = put(g, 0, "Jinx, Rebel", ready=False)
    hand(g, 0, V3); hand(g, 0, V4)
    hand(g, 0, "Invert Timelines")
    play(g, 0, "Invert Timelines")
    assert not j.exhausted and g.might(j) == 6


@T.test
def kayn_no_damage_after_two_moves():
    g, _ = N()
    k = put(g, 0, "Kayn, Unleashed")
    g.move([k], 0, 0)
    g.deal(k, 2, "spell", 1)
    assert k.damage == 2
    g.move([k], 1, 0)
    g.deal(k, 3, "spell", 1)
    assert k.damage == 2


@T.test
def khazix_alone_enemy_buff_and_xp():
    g, _ = N()
    k = put(g, 0, "Kha'Zix, Mutating Horror")
    put(g, 1, V3, 1)
    g.apply(("move", (k.uid,), 1))
    settle(g)
    assert g.p[0].xp == 2 and k.zone == "board"


@T.test
def kharox_empower_burns_and_steals():
    g, _ = N()
    R(g, 0, 8)
    k = put(g, 0, "Kharox")
    deck_top(g, 1, [V8, "Gust", V3])
    g.apply([o for o in act_options(g, 0, "Kharox")][0])
    settle(g)
    assert k.empowered
    mm = unit(g, V8)
    assert mm.ctrl == 0 and mm.owner == 1


@T.test
def kinkou_lifeblade_empowered():
    g, _ = N()
    R(g, 0, 2)
    k = put(g, 0, "Kinkou Lifeblade")
    assert not g.has_kw(k, "Ganking")
    g.apply(act_options(g, 0, "Kinkou Lifeblade")[0])
    settle(g)
    assert k.empowered and g.might(k) == 5 and g.has_kw(k, "Ganking")


@T.test
def kogmaw_deathknell_deals_4_here():
    g, _ = N()
    k = put(g, 0, "Kog'Maw, Caustic", 1)
    e = put(g, 1, V4, 1, bf_control=False)
    other = put(g, 1, V4, 0)
    g.kill([k], 1)
    settle(g)
    assert e.zone == "trash" and other.zone == "board"


# ====================================================================== L
@T.test
def last_rites_equip_cost_and_replay():
    g, _ = N(answers0={"recycle_from_trash": lambda opts, ctx: [c for c in opts if c.spec["type"] != "Unit"][0]})
    R(g, 0, 6)
    u = put(g, 0, V4)
    lr = put(g, 0, "Last Rites")
    assert not act_options(g, 0, "Last Rites")            # needs 2 cards in the trash
    trash(g, 0, "Gust"); trash(g, 0, "Rebuke")
    t = trash(g, 0, V3)
    g.apply([o for o in act_options(g, 0, "Last Rites") if o[3]["tg"] == (u.uid,)][0])
    settle(g)
    assert lr.attached_to == u.uid and g.might(u) == 6 and len(g.p[0].trash) == 1
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert t.zone == "board"


@T.test
def loyal_pup_joins_defense():
    g, _ = N()
    pup = put(g, 1, "Loyal Pup")
    put(g, 1, V3, 1)
    a = put(g, 0, V4)
    g.apply(("move", (a.uid,), 1))
    settle(g)
    assert a.zone == "trash"                         # 3 + 3 defenders: the 4 might attacker dies"


@T.test
def lunar_boon_loot():
    g, _ = N()
    R(g, 0, 3)
    c = hand(g, 0, V3)
    hand(g, 0, "Lunar Boon")
    play(g, 0, "Lunar Boon")
    assert c in g.p[0].trash and len(g.p[0].hand) == 2


# ====================================================================== M
@T.test
def maddened_marauder_moves_unit_to_base():
    g, _ = N()
    R(g, 0)
    e = put(g, 1, V5, 1)
    hand(g, 0, "Maddened Marauder")
    play(g, 0, "Maddened Marauder", base)
    assert e.loc == "base" and g.has_kw(unit(g, "Maddened Marauder"), "Tank")


@T.test
def megatusk_spends_xp_for_ganking():
    g, _ = N()
    m = put(g, 0, "Megatusk", 0)
    f = put(g, 0, V3, 0)
    assert not act_options(g, 0, "Megatusk")
    g.p[0].xp = 3
    g.apply(act_options(g, 0, "Megatusk")[0])
    settle(g)
    assert g.has_kw(f, "Ganking") and g.p[0].xp == 0


@T.test
def mel_empower_by_discarding_spell_banishes():
    g, _ = N()
    m = put(g, 0, "Mel, Defiant Soul")
    s = hand(g, 0, "Gust")
    e = put(g, 1, V3, 1)
    g.apply(act_options(g, 0, "Mel, Defiant Soul")[0])
    settle(g)
    assert m.empowered and s in g.p[0].trash and e in g.p[1].banish


@T.test
def minah_each_player_draws():
    g, _ = N()
    m = put(g, 0, "Minah Swiftfoot")
    g.apply(("move", (m.uid,), 0))
    settle(g)
    assert len(g.p[0].hand) == 1 and len(g.p[1].hand) == 1
    g2, _ = N(answers0={"minah_mode": "discard"})
    m = put(g2, 0, "Minah Swiftfoot")
    hand(g2, 0, V3); hand(g2, 1, V3)
    g2.apply(("move", (m.uid,), 0))
    settle(g2)
    assert not g2.p[0].hand and not g2.p[1].hand


@T.test
def mindsplitter_discards_chosen_card():
    g, _ = N()
    R(g, 0)
    a = hand(g, 1, V3); b = hand(g, 1, V8)
    hand(g, 0, "Mindsplitter")
    play(g, 0, "Mindsplitter", base)
    assert b in g.p[1].trash and a in g.p[1].hand


@T.test
def mister_root_xp_on_move():
    g, _ = N()
    m = put(g, 0, "Mister Root")
    g.apply(("move", (m.uid,), 0))
    settle(g)
    assert g.p[0].xp == 2


@T.test
def morbid_return_returns_unit():
    g, _ = N()
    R(g, 0, 2)
    t = trash(g, 0, V8)
    hand(g, 0, "Morbid Return")
    play(g, 0, "Morbid Return")
    assert t in g.p[0].hand


# ====================================================================== O / P
@T.test
def overzealous_fan_sacrifices_to_repel():
    g, _ = N()
    fan = put(g, 1, "Overzealous Fan", 1)
    a = put(g, 0, V5)
    g.apply(("move", (a.uid,), 1))
    settle(g)
    assert fan.zone == "trash" and a.loc == "base" and a.zone == "board"


@T.test
def pack_of_wonders_returns_friendly_unit():
    g, _ = N()
    put(g, 0, "Pack of Wonders")
    u = put(g, 0, V8)
    g.apply(act_options(g, 0, "Pack of Wonders")[0])
    settle(g)
    assert u in g.p[0].hand
    g, _ = N()
    put(g, 0, "Pack of Wonders")
    fd = facedown(g, 0, "Gust", 0)
    g.apply([o for o in act_options(g, 0, "Pack of Wonders") if o[3].get("fd") == 0][0])
    settle(g)
    assert fd in g.p[0].hand and g.bfs[0].facedown is None


@T.test
def possession_steals_unit():
    g, _ = N()
    R(g, 0, 11)
    e = put(g, 1, V8, 1)
    hand(g, 0, "Possession")
    play(g, 0, "Possession")
    assert e.ctrl == 0 and e.loc == "base" and e.owner == 1


@T.test
def pyke_gold_once_per_turn():
    g, _ = N()
    put(g, 0, "Pyke, Returned", 0)
    a = put(g, 1, V3, 1); b = put(g, 1, V3, 1)
    g.kill([a], 0); g.kill([b], 0)
    settle(g)
    assert len([o for o in g.gear(0) if o.cname == "Gold"]) == 1


# ====================================================================== R
@T.test
def ravenbloom_banishes_opponent_gear():
    g, _ = N()
    rv = put(g, 0, "Ravenbloom Prefect")
    R(g, 1, 2)
    hand(g, 1, "Cull")
    g.apply(("end",)); settle(g)
    play(g, 1, "Cull")
    assert rv in g.p[0].banish
    assert any(c.cname == "Cull" for c in g.p[1].banish)


@T.test
def rebuke_returns_unit():
    g, _ = N()
    R(g, 0, 4)
    e = put(g, 1, V8, 1)
    hand(g, 0, "Rebuke")
    play(g, 0, "Rebuke")
    assert e in g.p[1].hand


@T.test
def rhasa_and_shadowblade_cost_reduction():
    g, _ = N()
    for _ in range(4):
        trash(g, 0, V3)
    trash(g, 0, "Shadowblade Lurker")
    r = hand(g, 0, "Rhasa the Sunderer")
    s = hand(g, 0, "Shadowblade Lurker")
    assert total_cost(g, 0, r, dict(loc="base"), "hand")[0] == 5
    assert total_cost(g, 0, s, dict(loc="base"), "hand")[0] == 3


@T.test
def ride_the_wind_moves_and_readies():
    g, _ = N()
    R(g, 0, 3)
    u = put(g, 0, V3, ready=False)
    hand(g, 0, "Ride The Wind")
    play(g, 0, "Ride The Wind", lambda ch: ch["dest"] == 1)
    assert u.loc == 1 and not u.exhausted


# ====================================================================== S
@T.test
def scryers_bloom_predict_draw_xp():
    g, _ = N()
    R(g, 0, 2)
    hand(g, 0, "Scryer's Bloom")
    play(g, 0, "Scryer's Bloom")
    b = [o for o in g.gear(0) if o.cname == "Scryer's Bloom"][0]
    assert b.exhausted and not act_options(g, 0, "Scryer's Bloom")
    b.exhausted = False
    g.apply(act_options(g, 0, "Scryer's Bloom")[0])
    settle(g)
    assert b.zone == "trash" and len(g.p[0].hand) == 1 and g.p[0].xp == 1


@T.test
def seal_of_discord_adds_chaos():
    g, _ = N()
    runes(g, 0, ["Fury"] * 2)
    put(g, 0, "Seal of Discord")
    c = hand(g, 0, "Gust Monk")
    hand(g, 0, "Morbid Return")
    trash(g, 0, V3)
    play(g, 0, "Morbid Return")                      # 2 energy, no power: the Seal is not needed
    assert any(c_.cname == V3 for c_ in g.p[0].hand)
    g, _ = N()
    runes(g, 0, ["Fury"] * 1)
    put(g, 0, "Seal of Discord")
    hand(g, 0, "Called Shot")                        # [Chaos]: payable only with the Seal
    play(g, 0, "Called Shot", lambda ch: not ch.get("rep"))
    assert [o for o in g.gear(0) if o.cname == "Seal of Discord"][0].exhausted


@T.test
def shadow_order_disciple_burns_for_might():
    g, _ = N()
    d = put(g, 0, "Shadow Order Disciple")
    top = deck_top(g, 0, [V3])
    g.apply(("move", (d.uid,), 0))
    settle(g)
    assert top[0] in g.p[0].trash and g.might(d) == 3


@T.test
def shadows_of_the_past_returns_two():
    g, _ = N()
    R(g, 0, 4)
    a = trash(g, 0, V8); b = trash(g, 1, V5)
    hand(g, 0, "Shadows of the Past")
    play(g, 0, "Shadows of the Past", lambda ch: set(ch["cs"]) == {a.uid, b.uid})
    assert a in g.p[0].hand and b in g.p[1].hand


@T.test
def sinister_poro_pays_to_repel():
    g, _ = N()
    R(g, 0, 1)
    p = put(g, 0, "Sinister Poro")
    e = put(g, 1, V3, 1)
    g.apply(("move", (p.uid,), 1))
    settle(g)
    assert e.loc == "base" and g.bfs[1].ctrl == 0


@T.test
def soulgorger_and_harrowing_play_from_trash():
    g, _ = N()
    R(g, 0, 12)
    t = trash(g, 0, V8)
    hand(g, 0, "Soulgorger")
    play(g, 0, "Soulgorger", base)
    assert t.zone == "board" and len(g.p[0].runes) == 10
    g, _ = N()
    R(g, 0, 8)
    t = trash(g, 0, V5)
    hand(g, 0, "The Harrowing")
    play(g, 0, "The Harrowing")
    assert t.zone == "board" and len(g.p[0].runes) == 6


@T.test
def spiderling_might_per_spiderling():
    g, _ = N()
    a = put(g, 0, "Spiderling", 0); b = put(g, 0, "Spiderling", 0); c = put(g, 0, "Spiderling", 1)
    assert g.might(a) == 2 and g.might(c) == 1


@T.test
def spirit_wheel_draws_on_choosing_friend():
    g, _ = N()
    R(g, 0, 5)
    w = put(g, 0, "Spirit Wheel")
    u = put(g, 0, V3)
    hand(g, 0, "Ride The Wind")
    play(g, 0, "Ride The Wind", lambda ch: ch["tg"] == (u.uid,))
    assert w.exhausted and len(g.p[0].hand) == 1


@T.test
def stacked_deck_takes_best():
    g, _ = N()
    R(g, 0, 1)
    top = deck_top(g, 0, [V3, V8, V4])
    hand(g, 0, "Stacked Deck")
    play(g, 0, "Stacked Deck")
    assert top[1] in g.p[0].hand and top[0] in g.p[0].deck[-2:] and top[2] in g.p[0].deck[-2:]


@T.test
def star_crossed_returns_both():
    g, _ = N()
    R(g, 0, 4)
    f = put(g, 0, V3); e = put(g, 1, V8, 1)
    hand(g, 0, "Star-Crossed")
    play(g, 0, "Star-Crossed")
    assert f.zone == "hand" and e.zone == "hand"


@T.test
def stealthy_pursuer_follows():
    g, _ = N()
    sp = put(g, 0, "Stealthy Pursuer")
    f = put(g, 0, V3)
    g.apply(("move", (f.uid,), 1))
    settle(g)
    assert sp.loc == 1


@T.test
def switcheroo_swaps_might():
    g, _ = N()
    R(g, 0, 4)
    a = put(g, 0, V3, 1); b = put(g, 0, V8, 1)
    hand(g, 0, "Switcheroo")
    play(g, 0, "Switcheroo")
    assert g.might(a) == 8 and g.might(b) == 3


# ====================================================================== T
@T.test
def tail_cloaked_matriarch_reanimates():
    g, _ = N()
    R(g, 0, 3)
    put(g, 0, "Tail-Cloaked Matriarch")
    t = trash(g, 0, V3)
    trash(g, 0, V8)                                  # too expensive
    g.apply(act_options(g, 0, "Tail-Cloaked Matriarch")[0])
    settle(g)
    assert t.zone == "board" and t.loc == "base"


@T.test
def teemo_plus_three():
    g, _ = N()
    R(g, 0, 2)
    hand(g, 0, "Teemo, Scout")
    play(g, 0, "Teemo, Scout", base)
    assert g.might(unit(g, "Teemo, Scout")) == 4


@T.test
def temptation_moves_enemy_next_to_friend_with_repeat():
    g, _ = N()
    R(g, 0, 4)
    a = put(g, 1, V3, 0); b = put(g, 1, V4)
    c = put(g, 1, V5, 1)
    hand(g, 0, "Temptation")
    o = opt(g, 0, "Temptation", lambda ch: ch.get("rep2") and ch["tg"] != ch["tg2"])
    ch = o[3]
    g.apply(o)
    settle(g)
    assert g.obj(ch["tg"][0]).loc == ch["dest"] and g.obj(ch["tg2"][0]).loc == ch["dest2"]


@T.test
def the_list_names_tag():
    g, _ = N(answers0={"name_tag": "Pirate"})
    R(g, 0, 1)
    e = put(g, 1, V3, 1)                             # Pirate
    hand(g, 0, "The List")
    play(g, 0, "The List")
    g.apply(act_options(g, 0, "The List")[0])
    settle(g)
    assert g.might(e) == 1


@T.test
def the_syren_moves_friend_to_base():
    g, _ = N()
    R(g, 0, 1)
    put(g, 0, "The Syren")
    u = put(g, 0, V3, 1)
    g.apply(act_options(g, 0, "The Syren")[0])
    settle(g)
    assert u.loc == "base"


@T.test
def tideturner_swaps_places():
    g, _ = N()
    R(g, 0, 2)
    f = put(g, 0, V3, 1)
    hand(g, 0, "Tideturner")
    play(g, 0, "Tideturner", base)
    t = unit(g, "Tideturner")
    assert t.loc == 1 and f.loc == "base"


@T.test
def tornado_warrior_empowers_until_end_of_turn():
    g, _ = N()
    facedown(g, 0, "Tornado Warrior", 0)
    k = put(g, 0, "Kinkou Lifeblade", 0)
    play(g, 0, "Tornado Warrior", src="facedown")
    assert k.empowered and g.has_kw(k, "Ganking")
    g.apply(("end",)); settle(g)
    assert not k.empowered


@T.test
def traveling_merchant_and_treasure_hunter_on_move():
    g, _ = N()
    m = put(g, 0, "Traveling Merchant")
    h = put(g, 0, "Treasure Hunter")
    c = hand(g, 0, V8)
    g.apply(("move", (m.uid, h.uid), 0))
    settle(g)
    assert c in g.p[0].trash and len(g.p[0].hand) == 1
    assert len([o for o in g.gear(0) if o.cname == "Gold"]) == 1


@T.test
def twilight_step_and_flow():
    g, _ = N()
    R(g, 0, 3)
    e = put(g, 1, V3, 1)
    big = put(g, 1, V8, 1)
    c = hand(g, 0, "Twilight Step")
    assert all(ch["tg"] != (big.uid,) for ch in card_choices(g, 0, c, "hand", False, False))
    play(g, 0, "Twilight Step", lambda ch: ch["tg"] == (e.uid,) and ch["dest"] == "base")
    assert e.loc == "base" and c in g.p[0].trash
    R(g, 0, 5)
    e.loc = 1
    play(g, 0, "Twilight Step", lambda ch: ch["tg"] == (e.uid,) and ch["dest"] == "base", src="trash")
    assert e.loc == "base" and c in g.p[0].banish


@T.test
def twisted_fate_reveals_rune():
    from game import Rune
    for dom, check in (("Mind", "draw"), ("Fury", "dmg"), ("Order", "stun")):
        g, _ = N()
        tf = put(g, 0, "Twisted Fate, Gambler")
        a = put(g, 1, V5, 1); b = put(g, 1, V5, 1)
        g.p[0].rune_deck.insert(0, Rune(dom, 0))
        r = g.p[0].rune_deck[0]
        g.apply(("move", (tf.uid,), 1))
        d = g.advance()
        while g.chain:
            g.apply(("pass",)); g.advance()
        assert g.p[0].rune_deck[-1] is r
        if check == "draw":
            assert len(g.p[0].hand) == 1
        elif check == "dmg":
            assert sorted([a.damage, b.damage]) == [1, 2]
        else:
            assert a.stunned or b.stunned


# ====================================================================== U / V / W
@T.test
def undercover_agent_deathknell():
    g, _ = N()
    u = put(g, 0, "Undercover Agent")
    c = hand(g, 0, V3)
    g.kill([u], 1)
    settle(g)
    assert c in g.p[0].trash and len(g.p[0].hand) == 2


@T.test
def up_from_the_deep_two_tentacles_and_flow():
    g, _ = N()
    R(g, 0, 6)
    c = hand(g, 0, "Up from the Deep")
    play(g, 0, "Up from the Deep")
    assert len([u for u in g.units(0) if u.cname == "Tentacle"]) == 2 and c in g.p[0].trash
    play(g, 0, "Up from the Deep", src="trash")
    assert len([u for u in g.units(0) if u.cname == "Tentacle"]) == 4 and c in g.p[0].banish


@T.test
def vicious_snapjaws_xp_when_friend_dies():
    g, _ = N()
    put(g, 0, "Vicious Snapjaws")
    a = put(g, 0, V3)
    put(g, 1, V3)
    g.kill([a], 1)
    g.kill([g.units(1)[0]], 0)
    settle(g)
    assert g.p[0].xp == 1


@T.test
def walking_roost_gives_opponent_a_bird():
    g, _ = N()
    R(g, 0)
    hand(g, 0, "Walking Roost")
    play(g, 0, "Walking Roost", base)
    birds = [u for u in g.units(1) if u.cname == "Bird"]
    assert len(birds) == 1 and g.has_kw(birds[0], "Deflect") and g.has_kw(unit(g, "Walking Roost"), "Deflect")


@T.test
def whirlwind_each_player_returns():
    g, _ = N()
    R(g, 0, 4)
    mine = put(g, 0, V8); theirs = put(g, 1, V8, 1)
    hand(g, 0, "Whirlwind")
    play(g, 0, "Whirlwind")
    assert mine.zone == "hand" and theirs.zone == "hand"     # P1 returns P0's unit, P0 returns P1's


@T.test
def wind_and_ghosts_banish_or_bounce():
    g, _ = N()
    R(g, 0, 8)
    small = put(g, 1, V3, 1); big = put(g, 1, V8, 1)
    hand(g, 0, "Wind and Ghosts"); hand(g, 0, "Wind and Ghosts")
    play(g, 0, "Wind and Ghosts", lambda ch: ch["tg"] == (small.uid,))
    play(g, 0, "Wind and Ghosts", lambda ch: ch["tg"] == (big.uid,))
    assert small in g.p[1].banish and big in g.p[1].hand


@T.test
def windsinger_hidden_restricted_to_its_battlefield():
    g, _ = N()
    facedown(g, 0, "Windsinger", 0)
    far = put(g, 1, V3, 1)
    near = unit(g, V3, 0)                            # the only unit with 3 might or less at that battlefield
    assert IMPL["Windsinger"].hidden
    play(g, 0, "Windsinger", src="facedown")
    assert near.zone == "hand" and far.zone == "board"


# ====================================================================== Y / Z
@T.test
def yasuo_scores_on_third_move():
    g, _ = N()
    y = put(g, 0, "Yasuo, Windrider")
    g.move([y], 0, 0); g.move([y], 1, 0)
    p = g.p[0].points
    settle(g)
    p = g.p[0].points
    g.move([y], 0, 0)
    settle(g)
    assert g.p[0].points >= p + 1 and g.stats.get("yasuo_moves") is None


@T.test
def zaunite_bouncer_returns_unit():
    g, _ = N()
    R(g, 0, 6)
    e = put(g, 1, V8, 1)
    hand(g, 0, "Zaunite Bouncer")
    play(g, 0, "Zaunite Bouncer", base)
    assert e in g.p[1].hand


@T.test
def zed_conquer_clone_and_swap():
    g, _ = N()
    R(g, 0, 2)
    z = put(g, 0, "Zed, Without a Sound")
    g.apply(("move", (z.uid,), 1))
    settle(g)
    clones = [u for u in g.units(0) if u.cname == "Shadow Clone"]
    assert len(clones) == 1 and clones[0].loc == "base"
    g.apply(act_options(g, 0, "Zed, Without a Sound")[0])
    settle(g)
    assert z.loc == "base" and clones[0].loc == 1


# ====================================================================== rules decided
@T.test
def abandon_flow_spell_stays_banished():
    g, _ = N()
    R(g, 0, 5); R(g, 1, 2)
    e = put(g, 1, V3, 1)
    ts = trash(g, 0, "Twilight Step")
    hand(g, 1, "Abandon")
    g.apply(opt(g, 0, "Twilight Step", lambda ch: ch["tg"] == (e.uid,), src="trash"))
    respond(g, 1, "Abandon")
    assert ts in g.p[0].banish and e.loc == 1


@T.test
def draven_death_not_doubled_by_karthus():
    g, _ = N()
    d = put(g, 0, "Draven, Audacious")
    put(g, 0, "Karthus, Eternal")
    put(g, 1, V8, 1)
    g.apply(("move", (d.uid,), 1))
    settle(g)
    assert g.p[1].points == 1


@T.test
def random_games_with_chaos_cards_are_deterministic():
    import fuzz_cards
    names = sorted(k for k, v in IMPL.items() if v.module == "cardsets.chaos")
    for seed in (3, 11):
        a = fuzz_cards.digest(fuzz_cards.play(seed, names))
        b = fuzz_cards.digest(fuzz_cards.play(seed, names))
        assert a == b


# ====================================================================== cards using the engine hooks (integration)
CH = frozenset({"Chaos"})


def _locs(g, pid, c):
    return sorted(str(ch["loc"]) for ch in card_choices(g, pid, c, "hand", False, False) if not ch.get("acc"))


@T.test
def open_battlefield_permissions():
    g, _ = N()
    R(g, 0)
    put(g, 1, V3, 1)
    for n in ("Miss Fortune, Buccaneer", "Ocean Drake", "Sai Scout", "Sneaky Deckhand"):
        assert _locs(g, 0, hand(g, 0, n)) == ["0", "base"], n
    sk = hand(g, 0, V3)
    R(g, 1)
    assert _locs(g, 0, sk) == ["base"]
    put(g, 0, "Miss Fortune, Buccaneer")
    assert _locs(g, 0, sk) == ["0", "base"] and _locs(g, 1, hand(g, 1, V3)) == ["1", "base"]
    assert g.kw_value(put(g, 0, "Sai Scout"), "Vision") == 1


@T.test
def ocean_drake_returns_a_non_dragon():
    g, _ = N()
    R(g, 0)
    e = put(g, 1, V5)
    hand(g, 0, "Ocean Drake")
    play(g, 0, "Ocean Drake", base)
    assert e.zone == "hand"


@T.test
def irelia_graceful_discounts_spells_choosing_her():
    g, _ = N()
    ir = put(g, 0, "Irelia, Graceful")
    c = hand(g, 0, "Temptation")
    assert total_cost(g, 0, c, dict(tg=(ir.uid,)), "hand") == (1, [])
    assert total_cost(g, 0, c, dict(tg=(put(g, 1, V3).uid,)), "hand") == (2, [])
    exhaust = g.p[0].runes = []
    assert total_cost(g, 0, c, dict(tg=(ir.uid,)), "hand") == (1, [])


@T.test
def stargazer_discounts_flow_from_trash():
    g, _ = N()
    put(g, 0, "Stargazer")
    c = trash(g, 0, "Twilight Step")
    assert total_cost(g, 0, c, dict(flow=True), "trash") == (2, [CH])
    assert total_cost(g, 0, hand(g, 0, "Twilight Step"), {}, "hand")[0] == SPEC["Twilight Step"]["e"]


@T.test
def vex_cheerless_changes_spell_costs_in_combat():
    from game import Showdown
    g, _ = N()
    v = put(g, 0, "Vex, Cheerless", 1)
    put(g, 1, V3, 1)
    c0, c1 = hand(g, 0, "Hard Bargain"), hand(g, 1, "Hard Bargain")
    assert total_cost(g, 0, c0, {}, "hand") == (2, [])
    g.sd = Showdown(1, True, 0)
    v.desig = "att"
    assert total_cost(g, 0, c0, {}, "hand") == (1, []) and total_cost(g, 1, c1, {}, "hand") == (3, [ANY])


@T.test
def ezreal_prodigy_discounts_each_optional_cost():
    g, _ = N()
    R(g, 0, 3)
    deck_top(g, 0, [V3, V3])
    keep = hand(g, 0, V4)
    hand(g, 0, "Ezreal, Prodigy")
    play(g, 0, "Ezreal, Prodigy", base)
    assert keep.zone == "trash" and len(g.p[0].hand) == 2
    hb = hand(g, 0, "Hard Bargain")
    assert total_cost(g, 0, hb, dict(rep2=True), "hand") == (3, [])
    cc = hand(g, 0, "Sivir, Mercenary")
    assert total_cost(g, 0, cc, dict(acc=True, loc="base"), "hand") == (4, [CH, CH])     # Accelerate: 0 + [C]
    e = put(g, 1, "Vex, Apathetic")
    t = hand(g, 0, "Temptation")
    assert total_cost(g, 0, t, dict(tg=(put(g, 1, V3).uid,), rep2=True, tg2=(e.uid,)), "hand") == (3, [ANY])


@T.test
def syndra_transcendent_grants_repeat_in_showdowns():
    from game import Showdown
    g, _ = N()
    R(g, 0, 8)
    sy = put(g, 0, "Syndra, Transcendent", 1)
    put(g, 1, V3, 1); put(g, 1, V4)
    c = hand(g, 0, "Called Shot")
    assert not [ch for ch in card_choices(g, 0, c, "hand", False, False) if ch.get("reps")]
    g.sd = Showdown(1, False, 1)
    chs = [ch for ch in card_choices(g, 0, c, "hand", False, True) if ch.get("reps")]
    e0, r0 = total_cost(g, 0, c, {}, "hand")
    assert chs and total_cost(g, 0, c, chs[0], "hand") == (e0 + 2, r0 + [CH])



# Repeat accordé (Syndra) sur une carte qui code son propre Repeat dans ses choix (Temptation : rep2 / tg2) :
# la répétition reprend un choix de base, jamais une variante qui contient déjà une répétition (son coût n'a pas
# été payé, règle 820.1.c.2). Avant correctif : TypeError « multiple values for keyword argument 'tg2' ».
@T.test
def granted_repeat_never_reuses_printed_repeat_variant():
    g, _ = N()
    R(g, 0, 12)
    put(g, 1, V3, 1); put(g, 1, V4); put(g, 1, V3)
    g.effects.append(dict(kind="grant_repeat", pid=0, cost=lambda g_, card: (1, [])))   # un Repeat [1] accordé
    c = hand(g, 0, "Temptation")
    g.every_choice = True                                    # tous les choix, comme pour un joueur humain
    chs = [ch for ch in card_choices(g, 0, c, "hand", False, False) if ch.get("reps")]
    assert chs
    for ch in chs:
        for rc in ch["reps"]:
            assert "tg2" not in rc and not rc.get("rep2") and not rc.get("rep"), rc
    ch = next(ch for ch in chs if ch.get("rep2"))
    g.apply(opt(g, 0, "Temptation", lambda x: x == ch))
    settle(g)

@T.test
def kennen_storm_gives_flow_on_conquer():
    g, _ = N()
    R(g, 0)
    deck_top(g, 0, [V3, V3])
    hand(g, 0, "Kennen, Storm of Shuriken")
    play(g, 0, "Kennen, Storm of Shuriken", base)
    assert len(g.p[0].trash) == 2
    k = unit(g, "Kennen, Storm of Shuriken", 0)
    sp = trash(g, 0, "Called Shot")
    assert not card_choices(g, 0, sp, "trash", False, False)
    k.loc = 0
    g.conquer(0, g.bfs[0]); settle(g)
    chs = [ch for ch in card_choices(g, 0, sp, "trash", False, False) if not ch.get("rep")]
    assert chs and total_cost(g, 0, sp, chs[0], "trash") == total_cost(g, 0, hand(g, 0, "Called Shot"), {}, "hand")


@T.test
def mask_mother_on_discard():
    g, _ = N()
    R(g, 0, 1)
    u = put(g, 0, V3)
    mm = hand(g, 0, "Mask Mother")
    g.discard(0, mm); settle(g)
    assert g.might(u) == 5


@T.test
def scrapheap_draws_when_played_discarded_or_killed():
    g, _ = N()
    R(g, 0, 2)
    deck_top(g, 0, [V3] * 4)
    hand(g, 0, "Scrapheap")
    play(g, 0, "Scrapheap")
    assert len(g.p[0].hand) == 1
    sh = [x for x in g.gear(0) if x.cname == "Scrapheap"][0]
    g.kill([sh]); settle(g)
    assert len(g.p[0].hand) == 2
    g.discard(0, hand(g, 0, "Scrapheap")); settle(g)
    assert len(g.p[0].hand) == 3
    sh2 = put(g, 0, "Scrapheap")
    g.to_zone(sh2, "hand"); settle(g)
    assert len(g.p[0].hand) == 4                       # returned: no draw (only the card itself came back)


@T.test
def treasure_trove_on_leaving():
    g, _ = N()
    R(g, 0, 1)
    deck_top(g, 0, [V3, V3])
    tt = put(g, 0, "Treasure Trove")
    n = len(g.p[0].runes)
    g.apply(act_options(g, 0, "Treasure Trove")[0]); settle(g)
    assert tt.zone == "trash" and len(g.p[0].hand) == 1 and len(g.p[0].runes) == n      # 1 recycled, 1 channeled
    assert sum(r.exhausted for r in g.p[0].runes) == 1
    tt2 = put(g, 0, "Treasure Trove")
    g.to_zone(tt2, "hand"); settle(g)
    assert len(g.p[0].hand) == 3


@T.test
def maduli_cant_be_readied_and_moves_over_weaker_enemies():
    g, _ = N()
    R(g, 0, 2)
    m = put(g, 0, "Maduli the Gatekeeper", ready=False)
    g.ready_obj(m, by=0, kind="awaken")
    assert m.exhausted
    put(g, 1, V3, 1)
    put(g, 1, V8, 0)
    opts = act_options(g, 0, "Maduli the Gatekeeper")
    assert [o[3]["bf"] for o in opts] == [1]
    g.apply(opts[0]); settle(g)
    assert m.loc == 1


@T.test
def vex_apathetic_stuns_and_pins_played_units():
    g, _ = N(tp=1)
    put(g, 0, "Vex, Apathetic", 0)
    R(g, 1, 4)
    hand(g, 1, V3)
    play(g, 1, V3, base)
    u = unit(g, V3, 1)
    assert u.stunned
    u.exhausted = False
    assert not [o for o in options_of(g, "move") if u.uid in o[1]]


@T.test
def sivir_mercenary_after_two_runes():
    g, _ = N()
    s_ = put(g, 0, "Sivir, Mercenary")
    assert g.might(s_) == 4
    R(g, 0, 2)
    g.pay(0, 0, [ANY, ANY])
    assert g.might(s_) == 6 and g.has_kw(s_, "Ganking")


@T.test
def nocturne_played_when_seen_on_top():
    g, _ = N()
    R(g, 0, 1)
    deck_top(g, 0, ["Nocturne, Horrifying"])
    g.predict(0, 1); settle(g)
    n = unit(g, "Nocturne, Horrifying", 0)
    assert n.zone == "board" and len(g.p[0].runes) == 0
    g2, _ = N()
    R(g2, 0, 1)
    deck_top(g2, 0, ["Nocturne, Horrifying"])
    g2.look(0, g2.p[0].deck[:1])
    g2.draw(0, 1); settle(g2)                        # drawn before the trigger resolves: not played
    assert [c.cname for c in g2.p[0].hand] == ["Nocturne, Horrifying"]


if __name__ == "__main__":
    T.main()
