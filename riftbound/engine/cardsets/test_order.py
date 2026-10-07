"""Tests of batch order. Run: RB_CARDSETS=order python3 cardsets/test_order.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from cards import make_token                            # noqa: E402
from game import Item                                   # noqa: E402
ORDER = frozenset({"Order"})

T = Suite("order")
SK, VS, PP, MD = "Shipyard Skulker", "Vanguard Sergeant", "Playful Phantom", "Mountain Drake"   # vanilla 3/4/5/10


def units_named(g, pid, name):
    return [u for u in g.units(pid) if u.cname == name]


def to_player(g, pid):
    """Let the other player pass until pid has the decision (showdowns)."""
    for _ in range(50):
        d = g.advance()
        if d is None or d.player == pid:
            return d
        g.apply(("pass",))
    raise RuntimeError("no decision for P%d" % pid)


def play(g, pid, name, pred=lambda ch: True):
    g.apply(opt(g, pid, name, pred))
    settle(g)


# ------------------------------------------------------------------ A-C
@T.test
def albus_ferros_spends_buffs_for_runes():
    g, _ = new()
    runes(g, 0, ["Order"] * 4)
    a, b = put(g, 0, SK), put(g, 0, VS)
    a.buff = b.buff = 1
    hand(g, 0, "Albus Ferros")
    play(g, 0, "Albus Ferros", lambda ch: ch["loc"] == "base")
    assert a.buff == 0 and b.buff == 0
    pl = g.p[0]
    assert len(pl.runes) == 6 and sum(r.exhausted for r in pl.runes) == 6


@T.test
def albus_ferros_may_spend_none():
    g, _ = new(answers0={"spend_buff": None})
    runes(g, 0, ["Order"] * 4)
    a = put(g, 0, SK)
    a.buff = 1
    hand(g, 0, "Albus Ferros")
    play(g, 0, "Albus Ferros", lambda ch: ch["loc"] == "base")
    assert a.buff == 1 and len(g.p[0].runes) == 4


@T.test
def altar_of_memories_draws_and_puts_back():
    g, _ = new()
    alt = put(g, 0, "Altar of Memories")
    u = put(g, 0, SK)
    top = deck_top(g, 0, ["Watchful Sentry"])
    g.kill([u])
    settle(g)
    pl = g.p[0]
    assert alt.exhausted and not pl.hand and pl.deck[-1] is top[0], (alt.exhausted, pl.hand)
    # exhausted: no second trigger
    u2 = put(g, 0, SK)
    g.kill([u2]); settle(g)
    assert not pl.hand and pl.deck[-1] is top[0]


@T.test
def ambessa_empowered_assault_and_attack_kill():
    g, _ = new()
    runes(g, 0, ["Order"] * 3)
    amb = put(g, 0, "Ambessa, Respected and Feared")
    acts = act_options(g, 0, "Ambessa, Respected and Feared")
    assert acts
    g.apply(acts[0]); settle(g)
    assert amb.empowered and g.has_kw(amb, "Assault") and g.kw_value(amb, "Assault") == 2
    small = put(g, 1, SK, 1)
    big = put(g, 1, MD, 1)
    g.apply(("move", (amb.uid,), 1))
    settle(g)
    assert small.zone == "trash" and big.zone == "board", (small.zone, big.zone)


@T.test
def ambessa_not_empowered_no_kill():
    g, _ = new()
    amb = put(g, 0, "Ambessa, Respected and Feared")
    small = put(g, 1, "Soaring Scout", 1)
    put(g, 1, MD, 1)
    g.apply(("move", (amb.uid,), 1))
    settle(g)
    assert g.kw_value(amb, "Assault") == 0
    assert small.zone == "trash"               # killed in combat only (Drake takes the rest), not by the trigger
    assert amb.zone == "trash"


@T.test
def aurok_general_boosts_empowered_units():
    g, _ = new()
    a = put(g, 0, "Aurok General")
    o = put(g, 0, "Escaped Grayback")
    plain = put(g, 0, SK)
    g.empower(o)
    assert g.might(a) == 5 and g.might(o) == 5            # Aurok not empowered: nothing (Grayback +2 itself)
    g.empower(a)
    assert g.might(a) == 7 and g.might(o) == 7 and g.might(plain) == 3
    e = put(g, 1, "Escaped Grayback"); g.empower(e)
    assert g.might(e) == 5                                # not your unit


@T.test
def azir_moves_tokens_on_attack():
    g, _ = new()
    az = put(g, 0, "Azir, Sovereign")
    t = make_token(g, "Recruit", 0, "base", ready=False)
    other = put(g, 0, SK)                       # not a token: stays
    put(g, 1, SK, 1)
    seen = []
    g.effects.append(dict(on="attack", fn=lambda g_, e, info: seen.append(info["obj"])))
    g.apply(("move", (az.uid,), 1))
    settle(g)
    assert seen == [az, t] and other.loc == "base"     # the Recruit joined the attack


@T.test
def bf_sword_equip_plus_3():
    g, _ = new()
    runes(g, 0, ["Order"])
    s = put(g, 0, "B.F. Sword")
    u = put(g, 0, SK)
    g.apply(act_options(g, 0, "B.F. Sword")[0]); settle(g)
    assert s.attached_to == u.uid and g.might(u) == 6


@T.test
def back_to_back_two_friends():
    g, _ = new()
    runes(g, 0, ["Order"] * 3)
    a, b = put(g, 0, SK), put(g, 0, VS)
    hand(g, 0, "Back to Back")
    c = hand(g, 0, "Back to Back")
    play(g, 0, "Back to Back")
    assert g.might(a) == 5 and g.might(b) == 6
    g.kill([b]); settle(g)
    assert card_choices(g, 0, c, "hand", False, False) == []      # two targets are needed (rule 355.8)


@T.test
def bandle_soldier_level3_ready():
    g, _ = new()
    runes(g, 0, ["Order"] * 5)
    hand(g, 0, "Bandle Soldier")
    g.gain_xp(0, 3)
    play(g, 0, "Bandle Soldier", lambda ch: ch["loc"] == "base")
    assert not units_named(g, 0, "Bandle Soldier")[0].exhausted
    g2, _ = new()
    runes(g2, 0, ["Order"] * 5)
    hand(g2, 0, "Bandle Soldier")
    play(g2, 0, "Bandle Soldier", lambda ch: ch["loc"] == "base")
    assert units_named(g2, 0, "Bandle Soldier")[0].exhausted


@T.test
def blade_of_the_ruined_king_kills_to_equip():
    g, _ = new()
    runes(g, 0, ["Order"])
    bl = put(g, 0, "Blade of the Ruined King")
    big = put(g, 0, VS)
    fod = put(g, 0, "Soaring Scout")
    acts = act_options(g, 0, "Blade of the Ruined King")
    assert all(a[3]["victim"] != a[3]["tg"][0] for a in acts)
    a = [a for a in acts if a[3]["tg"] == (big.uid,) and a[3]["victim"] == fod.uid][0]
    g.apply(a); settle(g)
    assert fod.zone == "trash" and bl.attached_to == big.uid and g.might(big) == 8
    g2, _ = new()
    runes(g2, 0, ["Order"])
    put(g2, 0, "Blade of the Ruined King"); put(g2, 0, VS)
    assert not act_options(g2, 0, "Blade of the Ruined King")      # no other unit to kill


@T.test
def blast_of_power_kills_at_battlefield():
    g, _ = new()
    runes(g, 0, ["Order"] * 7)
    e = put(g, 1, MD, 1)
    hand(g, 0, "Blast of Power")
    play(g, 0, "Blast of Power", lambda ch: ch["tg"] == (e.uid,))
    assert e.zone == "trash"


@T.test
def blood_money_golds():
    g, _ = new()
    runes(g, 0, ["Order"] * 4)
    e = put(g, 1, "Soaring Scout", 1)
    f = put(g, 0, "Pouty Poro", 1)
    hand(g, 0, "Blood Money"); hand(g, 0, "Blood Money")
    play(g, 0, "Blood Money", lambda ch: ch["tg"] == (e.uid,))
    assert e.zone == "trash" and len(units_named(g, 0, "Gold")) == 0
    golds = [x for x in g.gear(0) if x.cname == "Gold"]
    assert len(golds) == 1 and golds[0].exhausted
    play(g, 0, "Blood Money", lambda ch: ch["tg"] == (f.uid,))
    assert f.zone == "trash" and len([x for x in g.gear(0) if x.cname == "Gold"]) == 3


@T.test
def bonds_of_strength_repeat():
    g, _ = new()
    runes(g, 0, ["Order"] * 4)
    a, b = put(g, 0, SK), put(g, 0, VS)
    hand(g, 0, "Bonds of Strength")
    play(g, 0, "Bonds of Strength", lambda ch: ch.get("rep"))
    assert g.might(a) == 5 and g.might(b) == 6


@T.test
def call_to_glory_spend_buff_is_free():
    g, _ = new()
    runes(g, 0, ["Order"] * 1)
    a = put(g, 0, SK)
    b = put(g, 0, VS)
    b.buff = 1
    c = hand(g, 0, "Call to Glory")
    chs = card_choices(g, 0, c, "hand", False, False)
    assert chs and all(ch.get("free") for ch in chs)            # 1 rune: only the free version is affordable
    play(g, 0, "Call to Glory", lambda ch: ch["tg"] == (a.uid,))
    assert b.buff == 0 and g.might(a) == 6 and not g.p[0].runes[0].exhausted


@T.test
def carrion_dredger_bird():
    g, _ = new()
    d = put(g, 0, "Carrion Dredger")
    g.kill([d]); settle(g)
    birds = units_named(g, 0, "Bird")
    assert len(birds) == 1 and birds[0].loc == "base" and g.has_kw(birds[0], "Deflect") and birds[0].token


@T.test
def commander_ledros_kills_to_reduce_cost():
    g, _ = new()
    runes(g, 0, ["Order"] * 6)                  # 6 energy + [Order] x4, minus one per killed unit
    a, b = put(g, 0, "Soaring Scout"), put(g, 0, SK)
    c = hand(g, 0, "Commander Ledros")
    chs = card_choices(g, 0, c, "hand", False, False)
    assert sorted(len(ch.get("kills", ())) for ch in chs) == [0, 1, 1, 2], chs
    assert total_cost(g, 0, c, chs[2], "hand")[1] == [frozenset({"Order"})] * 2
    play(g, 0, "Commander Ledros", lambda ch: ch["loc"] == "base" and len(ch.get("kills", ())) == 2)
    led = units_named(g, 0, "Commander Ledros")[0]
    assert a.zone == "trash" and b.zone == "trash" and g.has_kw(led, "Ganking") and g.has_kw(led, "Deflect")
    assert len(g.p[0].runes) == 6 - 2 + 1      # Soaring Scout's Deathknell channels 1 rune


@T.test
def commander_ledros_every_kill_set_for_a_human():
    g, _ = new()
    runes(g, 0, ["Order"] * 10)
    us = [put(g, 0, SK) for _ in range(5)]
    c = hand(g, 0, "Commander Ledros")
    ai = {ch.get("kills", ()) for ch in card_choices(g, 0, c, "hand", False, False) if ch["loc"] == "base"}
    full = {ch.get("kills", ()) for ch in card_choices(g, 0, c, "hand", False, False, every=True) if ch["loc"] == "base"}
    assert len(ai) < 32 and len(full) == 32 and ai <= full          # every subset of the 5 units
    assert not getattr(g, "every_choice", False)                    # restored after the call
    g.agents[0].every_choice = True                                 # a human agent (train.Human)
    assert {ch.get("kills", ()) for ch in card_choices(g, 0, c, "hand", False, False) if ch["loc"] == "base"} == full
    assert len(us) == 5


@T.test
def corina_moves_and_makes_recruits():
    g, _ = new()
    c = put(g, 0, "Corina Veraza")
    g.apply(("move", (c.uid,), 1)); settle(g)
    rs = [u for u in g.units(0) if u.cname == "Recruit"]
    assert len(rs) == 3 and all(u.loc == 1 for u in rs)


@T.test
def crimson_pigeons_attacking_with_another():
    g, _ = new()
    p = put(g, 0, "Crimson Pigeons")
    assert g.might(p) == 3
    o = put(g, 0, SK)
    put(g, 1, MD, 1)
    seen = []
    g.effects.append(dict(on="attack", fn=lambda g_, e, info: seen.append(g_.might(p))))
    g.apply(("move", (p.uid, o.uid), 1)); settle(g)
    assert seen and seen[-1] == 5
    g2, _ = new()
    p2 = put(g2, 0, "Crimson Pigeons")
    put(g2, 1, MD, 1)
    seen2 = []
    g2.effects.append(dict(on="attack", fn=lambda g_, e, info: seen2.append(g_.might(p2))))
    g2.apply(("move", (p2.uid,), 1)); settle(g2)
    assert seen2 == [3]


@T.test
def cruel_patron_needs_a_kill():
    g, _ = new()
    runes(g, 0, ["Order"] * 4)
    c = hand(g, 0, "Cruel Patron")
    assert card_choices(g, 0, c, "hand", False, False) == []
    u = put(g, 0, SK)
    play(g, 0, "Cruel Patron", lambda ch: ch["loc"] == "base")
    assert u.zone == "trash" and units_named(g, 0, "Cruel Patron")


@T.test
def darius_executioner_legion_and_aura():
    g, _ = new()
    runes(g, 0, ["Order"] * 9)
    o = put(g, 0, SK)
    hand(g, 0, "Darius, Executioner")
    hand(g, 0, "Back to Back")
    put(g, 0, VS)
    play(g, 0, "Back to Back")
    play(g, 0, "Darius, Executioner", lambda ch: ch["loc"] == "base")
    d = units_named(g, 0, "Darius, Executioner")[0]
    assert not d.exhausted
    assert g.might(o) == 3 + 2 + 1 and g.might(d) == 6
    e = put(g, 1, SK)
    assert g.might(e) == 3
    g.move([o], 1, 0)
    assert g.might(o) == 5


@T.test
def darius_without_legion_stays_exhausted():
    g, _ = new()
    runes(g, 0, ["Order"] * 7)
    hand(g, 0, "Darius, Executioner")
    play(g, 0, "Darius, Executioner", lambda ch: ch["loc"] == "base")
    assert units_named(g, 0, "Darius, Executioner")[0].exhausted


@T.test
def disciple_of_shen_shield_3_with_exactly_one_other():
    g, _ = new()
    d = put(g, 0, "Disciple of Shen", 1)
    assert g.kw_value(d, "Shield") == 0
    put(g, 0, SK, 1)
    assert g.kw_value(d, "Shield") == 3
    put(g, 0, SK, 1)
    assert g.kw_value(d, "Shield") == 0


@T.test
def divine_judgment_recycles_the_rest():
    g, _ = new()
    runes(g, 0, ["Order"] * 9)
    us0 = [put(g, 0, n) for n in (SK, VS, PP)]
    us1 = [put(g, 1, n) for n in (SK, VS, PP)]
    gs = [put(g, 0, "B.F. Sword"), put(g, 0, "Eye of the Herald"), put(g, 0, "Glowstone")]
    for n in ("Blast of Power", "Vengeance", "Back to Back"):
        hand(g, 1, n)
    runes(g, 1, ["Order"] * 3)
    hand(g, 0, "Divine Judgment")
    play(g, 0, "Divine Judgment")
    assert len(g.units(0)) == 2 and len(g.units(1)) == 2 and len(g.gear(0)) == 2
    assert us0[0].zone == "deck" and us1[0].zone == "deck"          # lowest value recycled (AI order)
    assert len(g.p[0].runes) == 2 and len(g.p[1].runes) == 2 and len(g.p[1].hand) == 2
    assert g.p[0].deck[-1].cname in (SK, "Glowstone") or g.p[0].deck[-2].cname in (SK, "Glowstone")


@T.test
def divining_shells_ability():
    g, _ = new()
    sh = put(g, 0, "Divining Shells")
    u = put(g, 0, SK)
    acts = act_options(g, 0, "Divining Shells")
    g.apply([a for a in acts if a[3]["tg"] == (u.uid,)][0]); settle(g)
    assert sh.zone == "trash" and g.might(u) == 5


@T.test
def divining_shells_vision():
    g, _ = new(answers0={"predict_recycle": True})
    runes(g, 0, ["Order"] * 2)
    top = deck_top(g, 0, ["Watchful Sentry"])
    hand(g, 0, "Divining Shells")
    play(g, 0, "Divining Shells")
    assert g.p[0].deck[-1] is top[0]


@T.test
def drag_under_costs_less_outside_hand():
    g, _ = new()
    c = hand(g, 0, "Drag Under")
    e, _ = total_cost(g, 0, c, {}, "hand")
    c.zone = "deck"
    e2, _ = total_cost(g, 0, c, {}, "deck")
    assert e == 5 and e2 == 3
    c.zone = "hand"
    runes(g, 0, ["Order"] * 6)
    t = put(g, 1, MD, 1)
    play(g, 0, "Drag Under", lambda ch: ch["tg"] == (t.uid,))
    assert t.zone == "trash"


@T.test
def dragon_form_base_might_5():
    g, _ = new()
    runes(g, 0, ["Order"] * 6)
    u = put(g, 0, SK)
    g.mod(u, 1)
    hand(g, 0, "Dragon Form")
    play(g, 0, "Dragon Form", lambda ch: ch["tg"] == (u.uid,))
    assert g.might(u) == 6 and g.mighty(u)
    # flow from trash (banished afterwards); base stays 5
    c = [x for x in g.p[0].trash if x.cname == "Dragon Form"][0]
    play(g, 0, "Dragon Form", lambda ch: ch["tg"] == (u.uid,) and ch.get("flow"))
    assert g.might(u) == 6 and c.zone == "banish"
    d = put(g, 1, MD, 1)
    hand(g, 0, "Dragon Form")
    runes(g, 0, ["Order"] * 3)
    play(g, 0, "Dragon Form", lambda ch: ch["tg"] == (d.uid,))
    assert g.might(d) == 5
    g.apply(("end",)); settle(g)
    assert g.might(d) == 10 and g.might(u) == 3


@T.test
def eminent_benefactor_hold_golds():
    g, _ = new()
    put(g, 0, "Eminent Benefactor", 1)
    g.apply(("end",)); settle(g)
    g.apply(("end",)); settle(g)
    golds = [x for x in g.gear(0) if x.cname == "Gold"]
    assert len(golds) == 2 and all(x.exhausted for x in golds)


@T.test
def enthralling_protector_spend_xp_buff():
    g, _ = new()
    p = put(g, 0, "Enthralling Protector")
    assert not act_options(g, 0, "Enthralling Protector")
    g.gain_xp(0, 2)
    g.apply(act_options(g, 0, "Enthralling Protector")[0]); settle(g)
    assert p.buff == 1 and g.might(p) == 3 and g.p[0].xp == 0
    assert g.kw_value(p, "Hunt") == 1


@T.test
def escaped_grayback_empower_by_killing():
    g, _ = new()
    gb = put(g, 0, "Escaped Grayback")
    assert not act_options(g, 0, "Escaped Grayback")
    f = put(g, 0, "Soaring Scout")
    g.apply(act_options(g, 0, "Escaped Grayback")[0]); settle(g)
    assert f.zone == "trash" and gb.empowered and g.might(gb) == 5
    assert not act_options(g, 0, "Escaped Grayback")


@T.test
def eye_of_the_herald_recruit_on_move():
    g, _ = new()
    runes(g, 0, ["Order"])
    eye = put(g, 0, "Eye of the Herald")
    u = put(g, 0, SK)
    g.apply(act_options(g, 0, "Eye of the Herald")[0]); settle(g)
    assert eye.attached_to == u.uid and g.might(u) == 3
    g.apply(("move", (u.uid,), 1)); settle(g)
    rs = units_named(g, 0, "Recruit")
    assert len(rs) == 1 and rs[0].loc == 1


@T.test
def facebreaker_stuns_both():
    g, _ = new()
    runes(g, 0, ["Order"] * 2)
    f = put(g, 0, SK, 1)
    e = put(g, 1, VS, 1, bf_control=False)
    hand(g, 0, "Facebreaker")
    to_player(g, 0)
    play(g, 0, "Facebreaker", lambda ch: ch["tg"] == (f.uid, e.uid))
    assert f.stunned and e.stunned


@T.test
def faithful_manufactor_recruit_here():
    g, _ = new()
    runes(g, 0, ["Order"] * 3)
    put(g, 0, SK, 1)
    hand(g, 0, "Faithful Manufactor")
    play(g, 0, "Faithful Manufactor", lambda ch: ch["loc"] == 1)
    rs = units_named(g, 0, "Recruit")
    assert len(rs) == 1 and rs[0].loc == 1 and rs[0].token


@T.test
def fiora_victorious_mighty_keywords():
    g, _ = new()
    f = put(g, 0, "Fiora, Victorious")
    assert not g.has_kw(f, "Ganking")
    g.mod(f, 1)
    assert g.has_kw(f, "Ganking") and g.has_kw(f, "Deflect") and g.has_kw(f, "Shield")


@T.test
def fiora_worthy_ready_on_mighty():
    g, _ = new()
    runes(g, 0, ["Order"] * 4)
    put(g, 0, "Fiora, Worthy")
    u = put(g, 0, SK)
    g.need_cleanup = True; settle(g)
    u.exhausted = True
    hand(g, 0, "Back to Back")
    put(g, 0, VS)
    play(g, 0, "Back to Back", lambda ch: u.uid in ch["tg"])
    assert g.might(u) == 5 and not u.exhausted
    assert len(g.p[0].runes) == 2              # both targets became Mighty: [Order] paid twice


@T.test
def forge_of_the_future_recruit_and_recycle():
    g, _ = new()
    runes(g, 0, ["Order"] * 2)
    hand(g, 0, "Forge of the Future")
    play(g, 0, "Forge of the Future")
    rs = units_named(g, 0, "Recruit")
    assert len(rs) == 1 and rs[0].loc == "base"
    for n in ("Vengeance", "Blast of Power"):
        c = Obj(n, 1); c.zone = "trash"; g.p[1].trash.append(c)
    for n in ("Back to Back", "Guards!", "Dragon Form"):
        c = Obj(n, 0); c.zone = "trash"; g.p[0].trash.append(c)
    g.apply(act_options(g, 0, "Forge of the Future")[0]); settle(g)
    forge = [x for x in g.p[0].trash if x.cname == "Forge of the Future"]
    assert forge and len(g.p[1].trash) == 0 and len(g.p[0].trash) == 2      # Forge + 1 card left
    assert {c.cname for c in g.p[1].deck[-2:]} == {"Vengeance", "Blast of Power"}


@T.test
def garen_commander_aura():
    g, _ = new()
    gar = put(g, 0, "Garen, Commander", 1)
    a = put(g, 0, SK, 1)
    b = put(g, 0, SK)
    assert g.might(a) == 4 and g.might(b) == 3 and g.might(gar) == 5


@T.test
def glowstone_given_away_kills_their_units():
    g, _ = new()
    runes(g, 0, ["Order"] * 2)
    gl = put(g, 0, "Glowstone")
    mine = put(g, 0, VS)
    e = put(g, 1, SK)
    e2 = put(g, 1, MD)
    g.apply([a for a in act_options(g, 0, "Glowstone") if a[3] == {}][0]); settle(g)   # Empower
    assert gl.empowered
    g.apply([a for a in act_options(g, 0, "Glowstone") if a[3].get("player") == 1][0]); settle(g)
    assert gl.ctrl == 1 and not gl.empowered and gl.exhausted
    g.apply(("end",)); settle(g)          # end of P0's turn: nothing (P1 controls it)
    assert gl in g.board and mine.zone == "board"
    g.apply(("end",)); settle(g)          # end of P1's turn: kill it, deal 5 to P1's units
    assert gl.zone == "trash" and e.zone == "trash" and e2.zone == "board" and mine.zone == "board"


@T.test
def glowstone_hurts_its_controller():
    g, _ = new()
    gl = put(g, 0, "Glowstone")
    mine = put(g, 0, VS)
    g.apply(("end",)); settle(g)
    assert gl.zone == "trash" and mine.zone == "trash"


@T.test
def grand_strategem_plus_5():
    g, _ = new()
    runes(g, 0, ["Order"] * 9)
    a, b = put(g, 0, SK), put(g, 0, VS, 1)
    e = put(g, 1, SK)
    hand(g, 0, "Grand Strategem")
    play(g, 0, "Grand Strategem")
    assert g.might(a) == 8 and g.might(b) == 9 and g.might(e) == 3


@T.test
def guards_sand_soldier_ready():
    g, _ = new()
    runes(g, 0, ["Order"] * 4)
    hand(g, 0, "Guards!")
    play(g, 0, "Guards!")
    s = units_named(g, 0, "Sand Soldier")
    assert len(s) == 1 and not s[0].exhausted and s[0].loc == "base" and len(g.p[0].runes) == 3


@T.test
def guards_from_hidden_at_that_battlefield():
    g, _ = new(answers0={"may": False})
    runes(g, 0, ["Order"] * 2)
    put(g, 0, SK, 1)
    c = hand(g, 0, "Guards!")
    g.apply(("hide", c.uid, 1)); settle(g)
    g.turn_no += 1
    play(g, 0, "Guards!")
    s = units_named(g, 0, "Sand Soldier")
    assert len(s) == 1 and s[0].loc == 1 and s[0].exhausted


@T.test
def heroic_charge():
    g, _ = new()
    runes(g, 0, ["Order"] * 3)
    f = put(g, 0, SK, 1)
    e = put(g, 1, VS, 1, bf_control=False)
    hand(g, 0, "Heroic Charge")
    to_player(g, 0)
    play(g, 0, "Heroic Charge", lambda ch: ch["tg"] == (f.uid, e.uid))
    # the stunned attacker deals no combat damage; the defender (3+1) kills it
    assert g.might(f) == 4 and f.damage == 0 and f.zone == "board" and e.zone == "trash"


@T.test
def imperial_decree_kills_damaged_units():
    g, _ = new()
    runes(g, 0, ["Order"] * 7)
    a = put(g, 0, MD)
    e = put(g, 1, MD, 1)
    hand(g, 0, "Imperial Decree")
    play(g, 0, "Imperial Decree")
    g.apply(("move", (a.uid,), 1)); settle(g)
    assert a.zone == "trash" and e.zone == "trash"


# ------------------------------------------------------------------ K-Z
@T.test
def karma_buffs_on_recycle():
    g, _ = new(answers0={"predict_recycle": True})
    runes(g, 0, ["Order"] * 7)
    u = put(g, 0, SK)
    deck_top(g, 0, ["Watchful Sentry"])
    hand(g, 0, "Karma, Channeler")
    play(g, 0, "Karma, Channeler", lambda ch: ch["loc"] == "base")    # Vision recycles -> buff
    k = units_named(g, 0, "Karma, Channeler")[0]
    assert u.buff == 1 or k.buff == 1
    n = sum(x.buff for x in g.units(0))
    g.recycle_rune(0, g.p[0].runes[0]); settle(g)        # a rune is not a card
    assert sum(x.buff for x in g.units(0)) == n
    g.recycle_cards(1, [g.p[1].deck[0]]); settle(g)      # the opponent recycling: no trigger
    assert sum(x.buff for x in g.units(0)) == n


@T.test
def kayle_empowered_three_times():
    g, _ = new()
    runes(g, 0, ["Order"] * 9)
    k = put(g, 0, "Kayle, Justified")
    for i in range(3):
        acts = act_options(g, 0, "Kayle, Justified")
        assert acts, i
        g.apply(acts[0]); settle(g)
        assert g.might(k) == 3 + 2 * (i + 1)
        assert g.has_kw(k, "Ganking") == (i == 2)
    assert not act_options(g, 0, "Kayle, Justified") and g.kw_value(k, "Deflect") == 3
    g.disempower(k)
    assert g.might(k) == 3 and not g.has_kw(k, "Ganking")


@T.test
def keeper_of_law_discount():
    g, _ = new()
    c = hand(g, 0, "Keeper of Law")
    assert total_cost(g, 0, c, dict(loc="base"), "hand")[0] == 5
    put(g, 0, SK, 1); put(g, 0, SK, 1)
    e, reqs = total_cost(g, 0, c, dict(loc="base"), "hand")
    assert e == 3 and reqs == []
    put(g, 0, SK, 1)
    assert total_cost(g, 0, c, dict(loc="base"), "hand")[0] == 5


@T.test
def kings_edict_opponent_chooses():
    g, _ = new()
    runes(g, 0, ["Order"] * 8)
    mine = put(g, 0, "Soaring Scout")
    small, big = put(g, 1, "Soaring Scout"), put(g, 1, MD)
    hand(g, 0, "King's Edict")
    play(g, 0, "King's Edict")
    assert small.zone == "trash" and big.zone == "board" and mine.zone == "board"


@T.test
def lacerate_disempower_then_kill():
    g, _ = new()
    runes(g, 0, ["Order"] * 6)
    e = put(g, 1, "Solari Sunhawk")
    g.empower(e)
    assert g.might(e) == 4
    hand(g, 0, "Lacerate")
    play(g, 0, "Lacerate", lambda ch: ch["tg"] == (e.uid,))
    assert e.zone == "trash"
    big = put(g, 1, VS)
    c = [x for x in g.p[0].trash if x.cname == "Lacerate"][0]
    play(g, 0, "Lacerate", lambda ch: ch["tg"] == (big.uid,) and ch.get("flow"))
    assert big.zone == "board" and c.zone == "banish"


@T.test
def leona_stuns_on_attack():
    g, _ = new()
    le = put(g, 0, "Leona, Determined")
    e = put(g, 1, MD, 1)
    seen = []
    g.effects.append(dict(on="stun", fn=lambda g_, ef, info: seen.append(info["obj"])))
    g.apply(("move", (le.uid,), 1)); settle(g)
    assert seen == [e]
    d = put(g, 0, "Leona, Determined")
    d.desig = "def"
    assert g.might(d) == 5


@T.test
def lightning_rush_draw_one_trash_rest():
    g, _ = new()
    runes(g, 0, ["Order"] * 3)
    top = deck_top(g, 0, ["Vengeance", "Watchful Sentry", "Soaring Scout"])
    hand(g, 0, "Lightning Rush")
    play(g, 0, "Lightning Rush")
    pl = g.p[0]
    assert pl.hand == [top[0]] and top[1] in pl.trash and top[2] in pl.trash
    play(g, 0, "Lightning Rush", lambda ch: ch.get("flow"))          # 2 energy + [A] from the trash
    assert len(pl.hand) == 2 and [c for c in pl.banish if c.cname == "Lightning Rush"]


@T.test
def loyal_poro_not_alone():
    g, _ = new()
    p = put(g, 0, "Loyal Poro", 1)
    put(g, 0, SK, 1)
    deck_top(g, 0, ["Vengeance"])
    g.kill([p]); settle(g)
    assert len(g.p[0].hand) == 1
    p2 = put(g, 0, "Loyal Poro", 0)
    g.kill([p2]); settle(g)
    assert len(g.p[0].hand) == 1


@T.test
def lux_adds_energy_for_spells_only():
    g, _ = new()
    runes(g, 0, ["Order"] * 1)
    lux = put(g, 0, "Lux, Crownguard")
    sp = hand(g, 0, "Back to Back")             # 3 energy
    un = hand(g, 0, "Soaring Scout")            # 2 energy
    put(g, 0, SK); put(g, 0, VS)
    assert card_choices(g, 0, sp, "hand", False, False) and not card_choices(g, 0, un, "hand", False, False)
    play(g, 0, "Back to Back")
    assert lux.exhausted


@T.test
def machine_evangel_three_recruits():
    g, _ = new()
    m = put(g, 0, "Machine Evangel", 1)
    g.kill([m]); settle(g)
    rs = units_named(g, 0, "Recruit")
    assert len(rs) == 3 and all(r.loc == "base" for r in rs)


@T.test
def masa_optional_cost_stuns():
    g, _ = new()
    runes(g, 0, ["Order"] * 5)
    e = put(g, 1, MD, 1)
    c = hand(g, 0, "Masa, Crashing Thunder")
    chs = card_choices(g, 0, c, "hand", False, False)
    assert any(ch.get("masa") for ch in chs) and any(not ch.get("masa") for ch in chs)
    play(g, 0, "Masa, Crashing Thunder", lambda ch: ch.get("masa") and ch["loc"] == "base")
    assert e.stunned and len(g.p[0].runes) == 4
    g2, _ = new()
    runes(g2, 0, ["Order"] * 5)
    e2 = put(g2, 1, MD, 1)
    hand(g2, 0, "Masa, Crashing Thunder")
    play(g2, 0, "Masa, Crashing Thunder", lambda ch: not ch.get("masa") and ch["loc"] == "base")
    assert not e2.stunned and len(g2.p[0].runes) == 5


@T.test
def noxian_drummer_recruit_on_move():
    g, _ = new()
    d = put(g, 0, "Noxian Drummer")
    g.apply(("move", (d.uid,), 1)); settle(g)
    rs = units_named(g, 0, "Recruit")
    assert len(rs) == 1 and rs[0].loc == 1


@T.test
def noxian_emissary_empowered_deathknell():
    g, _ = new()
    runes(g, 0, ["Order"] * 2)
    e = put(g, 0, "Noxian Emissary")
    g.kill([e]); settle(g)
    assert not units_named(g, 0, "Recruit")
    e2 = put(g, 0, "Noxian Emissary")
    g.apply(act_options(g, 0, "Noxian Emissary")[0]); settle(g)
    assert e2.empowered
    g.kill([e2]); settle(g)
    assert len(units_named(g, 0, "Recruit")) == 2


@T.test
def peak_guardian_buffs():
    g, _ = new()
    runes(g, 0, ["Order"] * 7)
    a = put(g, 0, SK, 1)
    b = put(g, 0, SK)
    hand(g, 0, "Peak Guardian")
    play(g, 0, "Peak Guardian", lambda ch: ch["loc"] == 1)
    pg = units_named(g, 0, "Peak Guardian")[0]
    assert pg.buff == 1 and a.buff == 1 and b.buff == 0


@T.test
def poppy_spend_xp_discount():
    g, _ = new()
    runes(g, 0, ["Order"] * 4)
    c = hand(g, 0, "Poppy, Defender of the Meek")
    assert not card_choices(g, 0, c, "hand", False, False)
    g.gain_xp(0, 3)
    play(g, 0, "Poppy, Defender of the Meek", lambda ch: ch.get("xp") and ch["loc"] == "base")
    p = units_named(g, 0, "Poppy, Defender of the Meek")[0]
    assert g.p[0].xp == 0 and g.has_kw(p, "Tank")


@T.test
def recruit_the_vanguard_four_tokens():
    g, _ = new(answers0={"token_location": lambda opts, ctx: opts[-1]})
    runes(g, 0, ["Order"] * 6)
    put(g, 0, SK, 1)
    hand(g, 0, "Recruit the Vanguard")
    play(g, 0, "Recruit the Vanguard")
    rs = units_named(g, 0, "Recruit")
    assert len(rs) == 4 and all(r.loc == 1 for r in rs)


@T.test
def reksai_plays_revealed_unit_here():
    g, _ = new()
    runes(g, 0, ["Order"] * 4)
    rk = put(g, 0, "Rek'Sai, Swarm Queen")
    put(g, 1, MD, 1)
    top = deck_top(g, 0, [VS, "Vengeance"])
    g.apply(("move", (rk.uid,), 1)); settle(g)
    assert any("plays Vanguard Sergeant from deck (loc=1)" in l for l in g.lines)    # played here (then died)
    assert top[0].zone in ("board", "trash") and top[1].zone == "deck" and g.p[0].deck[-1] is top[1]
    assert len([r for r in g.p[0].runes if r.exhausted]) == 4


@T.test
def reksai_cannot_pay_recycles_both():
    g, _ = new()
    rk = put(g, 0, "Rek'Sai, Swarm Queen")
    put(g, 1, MD, 1)
    top = deck_top(g, 0, [VS, "Vengeance"])
    g.apply(("move", (rk.uid,), 1)); settle(g)
    assert top[0].zone == "deck" and top[1].zone == "deck" and set(g.p[0].deck[-2:]) == set(top)


@T.test
def royal_guard_sand_soldier():
    g, _ = new()
    runes(g, 0, ["Order"] * 4)
    hand(g, 0, "Royal Guard")
    play(g, 0, "Royal Guard", lambda ch: ch["loc"] == "base")
    assert len(units_named(g, 0, "Sand Soldier")) == 1


@T.test
def sacred_shears_deathknell():
    g, _ = new()
    runes(g, 0, ["Order"])
    sh = put(g, 0, "Sacred Shears")
    u = put(g, 0, SK)
    g.apply(act_options(g, 0, "Sacred Shears")[0]); settle(g)
    assert g.might(u) == 4
    deck_top(g, 0, ["Vengeance"])
    g.kill([u]); settle(g)
    assert len(g.p[0].hand) == 1 and sh.zone == "board" and sh.attached_to is None
    u2 = put(g, 0, SK)
    g.kill([u2]); settle(g)
    assert len(g.p[0].hand) == 1                         # not attached: nothing


@T.test
def sandshifter_kills_small_enemy():
    g, _ = new()
    runes(g, 0, ["Order"] * 7)
    s, b = put(g, 1, SK), put(g, 1, VS)
    hand(g, 0, "Sandshifter")
    play(g, 0, "Sandshifter", lambda ch: ch["loc"] == "base")
    assert s.zone == "trash" and b.zone == "board"


@T.test
def scrutinizing_sergeant_xp():
    g, _ = new()
    runes(g, 0, ["Order"] * 6)
    put(g, 0, SK); put(g, 0, SK, 1)
    put(g, 1, SK)
    hand(g, 0, "Scrutinizing Sergeant")
    play(g, 0, "Scrutinizing Sergeant", lambda ch: ch["loc"] == "base")
    assert g.p[0].xp == 3


@T.test
def seal_of_unity_adds_order():
    g, _ = new()
    runes(g, 0, ["Fury"] * 3)
    seal = put(g, 0, "Seal of Unity")
    assert g.pay(0, 1, [ORDER])
    assert seal.exhausted and len(g.p[0].runes) == 3


@T.test
def sett_buffed_units_here():
    g, _ = new()
    s = put(g, 0, "Sett, Kingpin", 1)
    a, b = put(g, 0, SK, 1), put(g, 0, SK)
    a.buff = b.buff = 1
    assert g.might(s) == 6
    s.buff = 1
    assert g.might(s) == 8 and g.has_kw(s, "Tank")


@T.test
def shadows_call_temporary_draw_2():
    g, _ = new()
    runes(g, 0, ["Order"] * 4)
    u = put(g, 0, SK)
    c = hand(g, 0, "Shadow's Call")
    play(g, 0, "Shadow's Call", lambda ch: ch["tg"] == (u.uid,))
    assert g.has_kw(u, "Temporary") and len(g.p[0].hand) == 2
    c2 = hand(g, 0, "Shadow's Call")
    assert not [ch for ch in card_choices(g, 0, c2, "hand", False, False) if ch["tg"] == (u.uid,)]


@T.test
def shard_of_undoing_with_temporary():
    g, _ = new()
    put(g, 0, "Shard of Undoing")
    t = put(g, 0, SK)
    g.grant(t, "Temporary", 1, None)
    t2 = put(g, 0, SK)
    g.grant(t2, "Temporary", 1, None)
    e1, e2 = put(g, 1, "Soaring Scout"), put(g, 1, VS)
    g.apply(("end",)); settle(g)
    g.apply(("end",)); settle(g)               # P0's Beginning Phase: both Temporary units die, first time only
    assert t.zone == "trash" and t2.zone == "trash"
    assert e1.zone == "trash" and e2.zone == "board"


@T.test
def shard_of_undoing_not_in_main_phase():
    g, _ = new()
    put(g, 0, "Shard of Undoing")
    u = put(g, 0, SK)
    e = put(g, 1, SK)
    g.kill([u]); settle(g)
    assert e.zone == "board"


@T.test
def shen_leader_hold_scores():
    g, _ = new()
    put(g, 0, "Shen, Leader of the Kinkou Order", 1)
    put(g, 0, SK, 1)
    pts = g.p[0].points
    g.apply(("end",)); settle(g)
    g.apply(("end",)); settle(g)
    assert g.p[0].points == pts + 2                    # hold + Shen
    g2, _ = new()
    put(g2, 0, "Shen, Leader of the Kinkou Order", 1)
    g2.apply(("end",)); settle(g2)
    g2.apply(("end",)); settle(g2)
    assert g2.p[0].points == 1


@T.test
def shepherds_heirloom_xp_and_equip():
    g, _ = new()
    runes(g, 0, ["Order"] * 2)
    hand(g, 0, "Shepherd's Heirloom")
    u = put(g, 0, SK)
    play(g, 0, "Shepherd's Heirloom")
    assert g.p[0].xp == 1
    h = [x for x in g.gear(0) if x.cname == "Shepherd's Heirloom"][0]
    g.apply(act_options(g, 0, "Shepherd's Heirloom")[0]); settle(g)
    assert h.attached_to == u.uid and g.might(u) == 5 and g.p[0].xp == 0
    assert not act_options(g, 0, "Shepherd's Heirloom")


@T.test
def solari_chief_stun_or_kill():
    g, _ = new()
    runes(g, 0, ["Order"] * 12)
    e = put(g, 1, MD)
    hand(g, 0, "Solari Chief"); hand(g, 0, "Solari Chief")
    play(g, 0, "Solari Chief", lambda ch: ch["loc"] == "base")
    assert e.stunned and e.zone == "board"
    play(g, 0, "Solari Chief", lambda ch: ch["loc"] == "base")
    assert e.zone == "trash"


@T.test
def solari_sunhawk_empowered():
    g, _ = new()
    runes(g, 0, ["Order"] * 2)
    s = put(g, 0, "Solari Sunhawk")
    g.apply(act_options(g, 0, "Solari Sunhawk")[0]); settle(g)
    assert g.might(s) == 4 and g.kw_value(s, "Deflect") == 2


@T.test
def soul_harvest():
    g, _ = new()
    runes(g, 0, ["Order"] * 3)
    s, b = put(g, 1, SK, 1), put(g, 1, VS, 1)
    c = hand(g, 0, "Soul Harvest")
    assert [ch["tg"] for ch in card_choices(g, 0, c, "hand", False, False)] == [(s.uid,)]
    play(g, 0, "Soul Harvest")
    assert s.zone == "trash" and b.zone == "board"


@T.test
def spectral_matron_plays_from_trash():
    g, _ = new()
    runes(g, 0, ["Order"] * 6)
    c = Obj(SK, 0); c.zone = "trash"; g.p[0].trash.append(c)
    big = Obj(VS, 0); big.zone = "trash"; g.p[0].trash.append(big)        # 4 energy: not eligible
    hand(g, 0, "Spectral Matron")
    play(g, 0, "Spectral Matron", lambda ch: ch["loc"] == "base")
    assert c.zone == "board" and big.zone == "trash"


@T.test
def stalking_wolf_kills_pet_and_ambushes():
    g, _ = new()
    runes(g, 0, ["Order"] * 6)
    pet = put(g, 0, "Pouty Poro", 1)
    put(g, 0, SK)
    w = hand(g, 0, "Stalking Wolf")
    chs = card_choices(g, 0, w, "hand", True, False)       # Closed state: Ambush to the pet's battlefield
    assert chs and all(ch["kill"] == pet.uid and ch["loc"] == 1 for ch in chs), chs
    play(g, 0, "Stalking Wolf", lambda ch: ch["loc"] == 1)
    assert pet.zone == "trash" and units_named(g, 0, "Stalking Wolf")[0].loc == 1
    g2, _ = new()
    runes(g2, 0, ["Order"] * 6)
    put(g2, 0, SK)
    w2 = hand(g2, 0, "Stalking Wolf")
    assert card_choices(g2, 0, w2, "hand", False, False) == []


@T.test
def starhound_returns_pet():
    g, _ = new()
    runes(g, 0, ["Order"] * 6)
    c = Obj("Pouty Poro", 0); c.zone = "trash"; g.p[0].trash.append(c)
    n = Obj(SK, 0); n.zone = "trash"; g.p[0].trash.append(n)
    hand(g, 0, "Starhound")
    play(g, 0, "Starhound", lambda ch: ch["loc"] == "base")
    assert c.zone == "hand" and n.zone == "trash"


@T.test
def the_ruination():
    g, _ = new()
    runes(g, 0, ["Order"] * 12)
    us = [put(g, 0, SK), put(g, 1, MD, 1), put(g, 1, VS)]
    gear = put(g, 0, "B.F. Sword")
    hand(g, 0, "The Ruination")
    play(g, 0, "The Ruination")
    assert all(u.zone == "trash" for u in us) and gear.zone == "board"


@T.test
def trifarian_gloryseeker_legion():
    g, _ = new()
    runes(g, 0, ["Order"] * 7)
    hand(g, 0, "Trifarian Gloryseeker"); hand(g, 0, "Trifarian Gloryseeker")
    play(g, 0, "Trifarian Gloryseeker", lambda ch: ch["loc"] == "base")
    play(g, 0, "Trifarian Gloryseeker", lambda ch: ch["loc"] == "base")
    gs = units_named(g, 0, "Trifarian Gloryseeker")
    assert sorted(u.buff for u in gs) == [0, 1]


@T.test
def trove_golem_four_golds():
    g, _ = new()
    runes(g, 0, ["Order"] * 10)
    hand(g, 0, "Trove Golem")
    play(g, 0, "Trove Golem", lambda ch: ch["loc"] == "base")
    golds = [x for x in g.gear(0) if x.cname == "Gold"]
    assert len(golds) == 4 and all(x.exhausted for x in golds)


@T.test
def trusty_ramhound():
    g, _ = new()
    r = put(g, 0, "Trusty Ramhound", 1)
    assert g.might(r) == 2
    put(g, 0, SK, 1)
    assert g.might(r) == 3


@T.test
def ultrasoft_poro_birds_at_battlefield():
    g, _ = new()
    p = put(g, 0, "Ultrasoft Poro")
    assert not act_options(g, 0, "Ultrasoft Poro")
    p.loc = 1; g.bfs[1].ctrl = 0
    g.apply(act_options(g, 0, "Ultrasoft Poro")[0]); settle(g)
    bs = units_named(g, 0, "Bird")
    assert len(bs) == 2 and all(g.has_kw(b, "Deflect") for b in bs) and p.exhausted


@T.test
def undying_loyalty_pet_discount():
    g, _ = new()
    runes(g, 0, ["Order"] * 1)
    c = Obj("Pouty Poro", 0); c.zone = "trash"; g.p[0].trash.append(c)
    sk = Obj("Soaring Scout", 0); sk.zone = "trash"; g.p[0].trash.append(sk)
    ul = hand(g, 0, "Undying Loyalty")
    chs = card_choices(g, 0, ul, "hand", False, False)
    assert [ch["pick"][0] for ch in chs] == [c.uid, sk.uid]          # both pets (Scout is a Bird)
    play(g, 0, "Undying Loyalty", lambda ch: ch["pick"][0] == c.uid)
    assert c.zone == "board" and c.loc == "base"
    g2, _ = new()
    runes(g2, 0, ["Order"] * 1)
    x = Obj("Watchful Sentry", 0); x.zone = "trash"; g2.p[0].trash.append(x)
    assert card_choices(g2, 0, hand(g2, 0, "Undying Loyalty"), "hand", False, False) == []    # 2 energy + [Order]


@T.test
def unsung_hero_mighty_draws_2():
    g, _ = new()
    h = put(g, 0, "Unsung Hero")
    g.kill([h]); settle(g)
    assert not g.p[0].hand
    h2 = put(g, 0, "Unsung Hero")
    g.mod(h2, 3)
    g.kill([h2]); settle(g)
    assert len(g.p[0].hand) == 2


@T.test
def vanguard_armory_three_recruits():
    g, _ = new()
    a = put(g, 0, "Vanguard Armory")
    g.apply(act_options(g, 0, "Vanguard Armory")[0]); settle(g)
    assert len(units_named(g, 0, "Recruit")) == 3 and a.exhausted


@T.test
def vanguard_attendant_enters_ready():
    g, _ = new()
    runes(g, 0, ["Order"] * 7)
    hand(g, 0, "Vanguard Attendant")
    play(g, 0, "Vanguard Attendant", lambda ch: ch["loc"] == "base")
    assert not units_named(g, 0, "Vanguard Attendant")[0].exhausted


@T.test
def vanguard_captain_legion():
    g, _ = new()
    runes(g, 0, ["Order"] * 9)
    hand(g, 0, "Vanguard Captain"); hand(g, 0, "Vanguard Captain")
    play(g, 0, "Vanguard Captain", lambda ch: ch["loc"] == "base")
    assert not units_named(g, 0, "Recruit")
    play(g, 0, "Vanguard Captain", lambda ch: ch["loc"] == "base")
    assert len(units_named(g, 0, "Recruit")) == 2


@T.test
def vengeance_kills():
    g, _ = new()
    runes(g, 0, ["Order"] * 6)
    e = put(g, 1, MD)
    hand(g, 0, "Vengeance")
    play(g, 0, "Vengeance")
    assert e.zone == "trash"


@T.test
def viktor_recruit_on_death():
    g, _ = new()
    v = put(g, 0, "Viktor, Leader")
    u = put(g, 0, SK)
    g.kill([u]); settle(g)
    rs = units_named(g, 0, "Recruit")
    assert len(rs) == 1 and rs[0].loc == "base"
    g.kill(rs); settle(g)
    assert not units_named(g, 0, "Recruit")              # a Recruit dying doesn't trigger
    u2 = put(g, 0, SK)
    g.kill([v, u2]); settle(g)                           # same action: Viktor doesn't see it (383.2.c.2)
    assert not units_named(g, 0, "Recruit")


@T.test
def xin_zhao_enters_ready_with_two_in_base():
    g, _ = new()
    runes(g, 0, ["Order"] * 8)
    put(g, 0, SK)
    hand(g, 0, "Xin Zhao, Vigilant"); hand(g, 0, "Xin Zhao, Vigilant")
    play(g, 0, "Xin Zhao, Vigilant", lambda ch: ch["loc"] == "base")
    x1 = units_named(g, 0, "Xin Zhao, Vigilant")[0]
    assert x1.exhausted
    play(g, 0, "Xin Zhao, Vigilant", lambda ch: ch["loc"] == "base")
    x2 = [u for u in units_named(g, 0, "Xin Zhao, Vigilant") if u is not x1][0]
    assert not x2.exhausted


@T.test
def zaun_punk_kills_gear():
    g, _ = new()
    runes(g, 0, ["Order"] * 3)
    mine = put(g, 0, "Eye of the Herald")
    theirs = put(g, 1, "B.F. Sword")
    hand(g, 0, "Zaun Punk")
    play(g, 0, "Zaun Punk", lambda ch: ch.get("kill") == mine.uid and ch["loc"] == "base")
    assert mine.zone == "trash" and theirs.zone == "trash"
    g2, _ = new()
    runes(g2, 0, ["Order"] * 3)
    t2 = put(g2, 1, "B.F. Sword")
    hand(g2, 0, "Zaun Punk")
    play(g2, 0, "Zaun Punk", lambda ch: not ch.get("kill") and ch["loc"] == "base")
    assert t2.zone == "board"


# ------------------------------------------------------------------ cards using the engine hooks (integration)
@T.test
def sacred_shears_deathknell_when_dying_with_the_unit():
    g, _ = new()
    runes(g, 0, ["Order"])
    sh = put(g, 0, "Sacred Shears")
    u = put(g, 0, SK)
    g.apply(act_options(g, 0, "Sacred Shears")[0]); settle(g)
    deck_top(g, 0, ["Vengeance"])
    g.kill([u, sh]); settle(g)                         # the Effect Text was the unit's when it died
    assert len(g.p[0].hand) == 1 and sh.zone == "trash"


def _attack(g, units, bf=1):
    g.apply(("move", tuple(u.uid for u in units), bf))
    return settle(g)


@T.test
def galio_deals_no_combat_damage():
    g, _ = new()
    ga = put(g, 0, "Galio, Indefatigable")
    d = put(g, 1, SK, 1)
    assert g.kw_value(ga, "Deflect") == 1 and g.has_kw(ga, "Tank")
    _attack(g, [ga])
    assert d.zone == "board" and d.loc == 1 and ga.zone == "board" and ga.loc == "base"


@T.test
def sacred_protector_needs_exactly_one_other_unit():
    g, _ = new()
    sp = put(g, 0, "Sacred Protector")
    d = put(g, 1, VS, 1)
    _attack(g, [sp])
    assert d.zone == "board"                           # alone: no combat damage
    g2, _ = new()
    sp2, sk = put(g2, 0, "Sacred Protector"), put(g2, 0, SK)
    d2 = put(g2, 1, PP, 1)
    _attack(g2, [sp2, sk])
    assert d2.zone == "trash"                          # 6 + 3 >= 5


@T.test
def tactical_retreat_saves_once():
    g, _ = new()
    runes(g, 0, ["Order"] * 2)
    u = put(g, 0, VS, 0)
    u.damage = 2
    hand(g, 0, "Tactical Retreat")
    play(g, 0, "Tactical Retreat", lambda ch: ch["tg"] == (u.uid,))
    g.kill([u]); settle(g)
    assert u.zone == "board" and u.loc == "base" and u.exhausted and u.damage == 0
    g.kill([u]); settle(g)
    assert u.zone == "trash"


@T.test
def soraka_saves_weaker_units_here():
    g, _ = new()
    so = put(g, 0, "Soraka, Wanderer", 0)
    a, b, big = put(g, 0, SK, 0), put(g, 0, SK, 0), put(g, 0, PP, 0)
    far = put(g, 0, SK, "base")
    g.kill([a, b, big, far]); settle(g)
    assert a.zone == b.zone == "board" and a.loc == b.loc == "base" and a.exhausted
    assert big.zone == "trash" and far.zone == "trash" and so.loc == 0


@T.test
def soraka_saves_units_dying_with_her_and_is_assigned_last():
    g, _ = new()
    so = put(g, 0, "Soraka, Wanderer", 0)
    a = put(g, 0, SK, 0)
    g.kill([so, a]); settle(g)
    assert so.zone == "trash" and a.zone == "board" and a.loc == "base"      # rule 370.4
    g2, _ = new()
    so2, sk = put(g2, 0, "Soraka, Wanderer", 0), put(g2, 0, SK, 0)
    out = g2.assign_damage(1, 3, [so2, sk])
    assert out == {sk: 3}


@T.test
def symbol_of_the_solari_recalls_all_on_tie():
    for with_symbol in (False, True):
        g, _ = new()
        if with_symbol:
            put(g, 0, "Symbol of the Solari")
        ga = put(g, 0, "Galio, Indefatigable")
        d = put(g, 1, SK, 1)
        _attack(g, [ga])
        assert ga.loc == "base" and d.zone == "board"
        assert d.loc == ("base" if with_symbol else 1)


@T.test
def renata_industrialist_tokens_enter_ready():
    g, _ = new()
    put(g, 0, "Renata Glasc, Industrialist")
    r = make_token(g, "Recruit", 0)
    gold = make_token(g, "Gold", 0, "base", ready=False)
    theirs = make_token(g, "Recruit", 1)
    assert not r.exhausted and not gold.exhausted and theirs.exhausted


@T.test
def rally_the_troops_buffs_units_played_this_turn():
    g, _ = new()
    runes(g, 0, ["Order"] * 5)
    deck_top(g, 0, ["Vengeance"])
    hand(g, 0, "Rally the Troops")
    play(g, 0, "Rally the Troops")
    assert [c.cname for c in g.p[0].hand] == ["Vengeance"]
    hand(g, 0, SK)
    play(g, 0, SK, lambda ch: ch["loc"] == "base")
    t = make_token(g, "Recruit", 0); settle(g)
    old = put(g, 1, SK)
    assert units_named(g, 0, SK)[0].buff == 1 and t.buff == 1 and old.buff == 0


@T.test
def reluctant_leader_grows_with_units_played():
    g, _ = new()
    runes(g, 0, ["Order"] * 3)
    rl = put(g, 0, "Reluctant Leader")
    hand(g, 0, SK)
    play(g, 0, SK, lambda ch: ch["loc"] == "base")
    assert g.might(rl) == 5
    make_token(g, "Recruit", 0); settle(g)
    assert g.might(rl) == 7
    make_token(g, "Recruit", 1); settle(g)
    assert g.might(rl) == 7


@T.test
def fallen_feline_forbids_the_named_spell_at_a_battlefield():
    g, _ = new(tp=0)
    runes(g, 0, ["Order"] * 2)
    put(g, 0, SK, 0)
    tr = Obj("Tactical Retreat", 1); tr.zone = "trash"; g.p[1].trash.append(tr)
    hand(g, 0, "Fallen Feline")
    play(g, 0, "Fallen Feline", lambda ch: ch["loc"] == 0)
    ff = units_named(g, 0, "Fallen Feline")[0]
    runes(g, 1, ["Order"] * 2)
    c = hand(g, 1, "Tactical Retreat")
    put(g, 1, SK)
    assert card_choices(g, 1, c, "hand", False, False) == []
    ff.loc = "base"
    assert card_choices(g, 1, c, "hand", False, False)
    ff.loc = 0
    runes(g, 0, ["Order"] * 2)
    assert card_choices(g, 0, hand(g, 0, "Tactical Retreat"), "hand", False, False)     # only opponents


@T.test
def mageseeker_investigator_taxes_group_moves():
    g, _ = new()
    put(g, 1, "Mageseeker Investigator", 1)
    a, b = put(g, 0, SK), put(g, 0, VS)
    groups = [o[1] for o in options_of(g, "move") if o[2] == 1]
    assert (a.uid,) in groups and (b.uid,) in groups and tuple(sorted((a.uid, b.uid))) not in groups
    runes(g, 0, ["Order"])
    groups = [o[1] for o in options_of(g, "move") if o[2] == 1]
    assert tuple(sorted((a.uid, b.uid))) in groups
    g.apply(("move", tuple(sorted((a.uid, b.uid))), 1))
    assert len(g.p[0].runes) == 0 and a.loc == b.loc == 1


@T.test
def hungry_wolf_needs_an_enemy_chosen_and_once_per_turn():
    g, _ = new()
    runes(g, 0, ["Order"] * 3)
    w = put(g, 0, "Hungry Wolf", ready=False)
    assert not act_options(g, 0, "Hungry Wolf")
    e = put(g, 1, SK)
    it = Item("ability", 0, "test")
    g.add_target(it, e)
    g.on_finalize(it)
    assert g.hist["chose_enemy"][0]
    g.apply(act_options(g, 0, "Hungry Wolf")[0]); settle(g)
    assert not w.exhausted and g.might(w) == 5
    assert not act_options(g, 0, "Hungry Wolf")


@T.test
def ivern_friend_to_all_gains_a_tag_and_scores():
    for tag, pts in (("Poro", 2), ("Bird", 1)):
        g, _ = new()
        runes(g, 0, ["Order"] * 6)
        put(g, 0, "Bird"); put(g, 0, "Fallen Feline"); put(g, 0, "Hungry Wolf")
        hand(g, 0, "Ivern, Friend to All")
        play(g, 0, "Ivern, Friend to All", lambda ch: ch["loc"] == "base" and ch["tag"] == tag)
        iv = units_named(g, 0, "Ivern, Friend to All")[0]
        assert tag in g.tags(iv)
        iv.loc = 1
        g.conquer(0, g.bfs[1]); settle(g)
        assert g.p[0].points == pts


@T.test
def shady_spectacles_copy_lasts_while_attached():
    g, _ = new()
    runes(g, 0, ["Order"] * 2)
    sp = put(g, 0, "Shady Spectacles")
    u, md = put(g, 0, SK), put(g, 0, MD)
    g.apply([o for o in act_options(g, 0, "Shady Spectacles") if o[3]["tg"] == (u.uid,)][0]); settle(g)
    assert u.cname == MD and g.might(u) == 10 and md.cname == MD
    g.kill([sp]); settle(g)
    assert u.cname == SK and g.might(u) == 3


@T.test
def undertitan_adds_energy_when_revealed_and_pumps_on_play():
    g, _ = new()
    deck_top(g, 0, ["Undertitan"])
    g.reveal(1, 1, owner=0)
    assert g.p[0].pool_e == 2 and g.p[1].pool_e == 0
    g.p[0].pool_e = 0
    runes(g, 0, ["Order"] * 7)
    a = put(g, 0, SK)
    hand(g, 0, "Undertitan")
    play(g, 0, "Undertitan", lambda ch: ch["loc"] == "base")
    assert g.might(a) == 5 and g.might(units_named(g, 0, "Undertitan")[0]) == 5


@T.test
def vanguard_helm_moves_a_buff_on_death():
    g, _ = new()
    put(g, 0, "Vanguard Helm")
    a, b = put(g, 0, SK), put(g, 0, VS)
    a.buff = 1
    g.kill([a]); settle(g)
    assert b.buff == 1
    c = put(g, 0, SK)
    g.kill([c]); settle(g)
    assert b.buff == 1


if __name__ == "__main__":
    T.main()
