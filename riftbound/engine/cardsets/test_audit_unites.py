"""Audit des unités et capacités à choix facultatif ("up to", "any number"), suite de test_audit_texte.py.
Règle 355.13 : zéro est un choix permis; le maximum est respecté. Run: python3 cardsets/test_audit_unites.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from cards import make_token                            # noqa: E402
from cardsets.test_audit_texte import play, rich, choices   # noqa: E402

T = Suite("audit_unites")


def named(g, pid, name):
    return [u for u in g.units(pid) if u.cname == name]


def none_answer(opts, ctx):
    return None


def body_rich(g, pid=0):
    runes(g, pid, ["Body"] * 12 + ["Order"] * 4)


def play_unit(g, name, loc="base", pid=0):
    hand(g, pid, name)
    play(g, pid, name, lambda ch: ch["loc"] == loc and not ch.get("acc") and not ch.get("spend")
         and not ch.get("kills") and not ch.get("bard"))
    return named(g, pid, name)[0]


def to_trash(g, pid, name):
    o = Obj(name, pid)
    o.zone = "trash"
    g.p[pid].trash.append(o)
    return o


# Kinkou Monk (body): "When you play me, buff up to two other friendly units."
@T.test
def monk_zero_other_units():
    g, _ = new(answers0={"buff_target": none_answer})
    rich(g)
    a = put(g, 0, "Legion Rearguard")
    m = play_unit(g, "Kinkou Monk")
    assert m.zone == "board" and a.buff == 0


@T.test
def monk_alone_still_playable():
    g, _ = new()
    rich(g)
    m = play_unit(g, "Kinkou Monk")
    assert m.zone == "board" and m.buff == 0


@T.test
def monk_max_two_others_not_self():
    g, _ = new()
    rich(g)
    us = [put(g, 0, "Legion Rearguard") for _ in range(3)]
    m = play_unit(g, "Kinkou Monk")
    assert sum(u.buff for u in us) == 2 and m.buff == 0


# Fae Dragon (body): "When you play me, buff up to four friendly units."
@T.test
def fae_dragon_zero():
    g, _ = new(answers0={"fae_dragon_buff": none_answer})
    rich(g)
    us = [put(g, 0, "Legion Rearguard") for _ in range(5)]
    f = play_unit(g, "Fae Dragon")
    assert sum(u.buff for u in us) + f.buff == 0


@T.test
def fae_dragon_max_four():
    g, _ = new()
    rich(g)
    us = [put(g, 0, "Legion Rearguard") for _ in range(5)]
    f = play_unit(g, "Fae Dragon")
    assert sum(u.buff for u in us) + f.buff == 4


# Elder Dragon (body): "choose up to one enemy unit at each location. Deal 1 to them."
@T.test
def elder_dragon_one_per_location():
    g, _ = new()
    body_rich(g)
    a1, a2 = put(g, 1, "Glasc Mixologist", "base"), put(g, 1, "Glasc Mixologist", "base")
    b = put(g, 1, "Glasc Mixologist", 1)
    play_unit(g, "Elder Dragon")
    # any amount of damage kills enemy units (Elder Dragon): exactly one unit at base, and the one at bf 1
    assert (a1.zone == "trash") + (a2.zone == "trash") == 1 and b.zone == "trash"


@T.test
def elder_dragon_zero():
    g, _ = new(answers0={"elder_dragon": none_answer})
    body_rich(g)
    a1 = put(g, 1, "Glasc Mixologist", "base")
    e = play_unit(g, "Elder Dragon")
    assert a1.zone == "board" and a1.damage == 0 and e.zone == "board"


# Corrupted Dragon (body): "When I attack, you may move any number of enemy units here each with 5 might or less
# to their base."
@T.test
def corrupted_dragon_zero():
    g, _ = new(answers0={"corrupted_dragon": none_answer})
    d = put(g, 0, "Corrupted Dragon")
    p = put(g, 1, "Legion Rearguard", 1)
    g.apply(("move", (d.uid,), 1))
    settle(g)
    assert p.loc != "base"


@T.test
def corrupted_dragon_several_small_not_big():
    g, _ = new()
    d = put(g, 0, "Corrupted Dragon")
    small = [put(g, 1, "Legion Rearguard", 1), put(g, 1, "Shipyard Skulker", 1)]
    big = put(g, 1, "Mega-Mech", 1)                         # 6 might: not a legal choice
    g.apply(("move", (d.uid,), 1))
    settle(g)
    assert all(u.loc == "base" for u in small), [u.loc for u in small]
    assert big.loc != "base"


# Gentle Gemdragon (body): "When you play me or another Dragon, ready up to 2 runes."
@T.test
def gemdragon_ready_at_most_two_runes():
    g, _ = new()
    runes(g, 0, ["Body"] * 12)
    play_unit(g, "Gentle Gemdragon")
    ex = sum(1 for r in g.p[0].runes if r.exhausted)
    spec = SPEC["Gentle Gemdragon"]
    assert ex == max(0, spec["e"] + spec["p"] - 2), (ex, spec["e"], spec["p"])



# Décision de l'utilisateur (2026-10-08) pour Gentle Gemdragon et Hwei : « ready up to 2 runes » = on prépare le maximum
# possible jusqu'à 2 ; si toutes les runes sont déjà prêtes, on ne prépare rien (et la partie continue).
@T.test
def gemdragon_ready_runes_edge_cases():
    from cardsets.body import _ready_runes
    for exhausted, expected in ((0, 0), (1, 1), (3, 2)):
        g, _ = new()
        runes(g, 0, ["Body"] * 6)
        for r in g.p[0].runes[:exhausted]:
            r.exhausted = True
        assert _ready_runes(g, 0, 2) == expected, (exhausted, expected)
        assert sum(1 for r in g.p[0].runes if r.exhausted) == exhausted - expected


@T.test
def gemdragon_triggers_on_another_dragon():
    g, _ = new()
    runes(g, 0, ["Body"] * 12)
    put(g, 0, "Gentle Gemdragon")
    play_unit(g, "Dune Drake")
    spec = SPEC["Dune Drake"]
    assert sum(1 for r in g.p[0].runes if r.exhausted) == max(0, spec["e"] + spec["p"] - 2)

# Albus Ferros (order): "spend any number of buffs. For each buff spent, channel 1 rune exhausted."
@T.test
def albus_zero_buffs():
    g, _ = new(answers0={"spend_buff": none_answer})
    rich(g)
    a = put(g, 0, "Legion Rearguard")
    g.buff(a)
    n0 = len(g.p[0].runes)
    play_unit(g, "Albus Ferros")
    assert a.buff == 1 and len(g.p[0].runes) == n0


@T.test
def albus_n_buffs_n_runes():
    g, _ = new()
    rich(g)
    a, b = put(g, 0, "Legion Rearguard"), put(g, 0, "Pouty Poro")
    g.buff(a)
    g.buff(b)
    n0 = len(g.p[0].runes)
    play_unit(g, "Albus Ferros")
    assert a.buff == 0 and b.buff == 0 and len(g.p[0].runes) == n0 + 2


# Kraken Hunter (body): "you may spend any number of buffs as an additional cost. Reduce my cost by 1 body rune for
# each buff you spend."
@T.test
def kraken_hunter_spend_zero_one_two():
    g, _ = new()
    rich(g)
    a, b = put(g, 0, "Legion Rearguard"), put(g, 0, "Pouty Poro")
    g.buff(a)
    g.buff(b)
    k = hand(g, 0, "Kraken Hunter")
    spends = sorted({len(x.get("spend", ())) for x in choices(g, "Kraken Hunter")})
    assert spends == [0, 1, 2], spends
    base = total_cost(g, 0, k, dict(loc="base"), "hand")
    two = total_cost(g, 0, k, dict(loc="base", spend=(a.uid, b.uid)), "hand")
    assert len(two[1]) == len(base[1]) - 2 and two[0] == base[0]


# Commander Ledros (order): "you may kill any number of friendly units as an additional cost. Reduce my cost by
# 1 order rune for each killed this way."
@T.test
def ledros_kill_zero_and_n():
    g, _ = new()
    rich(g)
    runes(g, 0, ["Order"] * 6 + ["Body"] * 8)
    us = [put(g, 0, "Pouty Poro") for _ in range(3)]
    k = hand(g, 0, "Commander Ledros")
    chs = choices(g, "Commander Ledros")
    assert {len(x.get("kills", ())) for x in chs} >= {0, 1, 2, 3}
    base = total_cost(g, 0, k, dict(loc="base"), "hand")
    two = total_cost(g, 0, k, dict(loc="base", kills=(us[0].uid, us[1].uid)), "hand")
    assert len(two[1]) == len(base[1]) - 2


# Forge of the Future (order): "Kill this: Recycle up to 4 cards from trashes."
def _forge_setup(answers=None):
    g, _ = new(answers0=answers)
    rich(g)
    for i in range(6):
        to_trash(g, i % 2, "Pouty Poro")
    f = put(g, 0, "Forge of the Future")
    return g, f, len(g.p[0].trash) + len(g.p[1].trash)


@T.test
def forge_recycle_zero():
    g, f, n = _forge_setup({"forge_recycle": none_answer})
    g.apply(act_options(g, 0, "Forge of the Future")[0])
    settle(g)
    assert f.zone == "trash" and len(g.p[0].trash) + len(g.p[1].trash) == n + 1


@T.test
def forge_recycle_max_four():
    g, f, n = _forge_setup()
    g.apply(act_options(g, 0, "Forge of the Future")[0])
    settle(g)
    assert len(g.p[0].trash) + len(g.p[1].trash) == n + 1 - 4


# Azir, Sovereign (order): "When I attack, you may move any number of your token units to this battlefield."
@T.test
def azir_zero_tokens_moved():
    g, _ = new(answers0={"azir_move": lambda opts, ctx: False})
    az = put(g, 0, "Azir, Sovereign")
    t = make_token(g, "Recruit", 0, "base")
    put(g, 1, "Shipyard Skulker", 1)
    g.apply(("move", (az.uid,), 1))
    settle(g)
    assert t.loc == "base"


@T.test
def azir_moves_all_tokens_only_tokens():
    g, _ = new()
    az = put(g, 0, "Azir, Sovereign")
    t1, t2 = make_token(g, "Recruit", 0, "base"), make_token(g, "Recruit", 0, "base")
    p = put(g, 0, "Pouty Poro", "base")
    put(g, 1, "Shipyard Skulker", 1)
    g.apply(("move", (az.uid,), 1))
    settle(g)
    assert t1.loc == 1 and t2.loc == 1 and p.loc == "base"


# Bard, Mercurial (mind): "move any number of your units to an open battlefield" (zero allowed)
@T.test
def bard_zero_units_moved():
    g, _ = new(answers0={"bard_move": lambda opts, ctx: next(o for o in opts if not o[1])})
    rich(g)
    a = put(g, 0, "Pouty Poro", "base")
    hand(g, 0, "Bard, Mercurial")
    play(g, 0, "Bard, Mercurial", lambda ch: ch.get("bard") and ch["loc"] == "base")
    assert a.loc == "base"


# Lee Sin, Ascetic (calm): "I can have any number of buffs."
@T.test
def lee_sin_ascetic_stacks_buffs():
    g, _ = new()
    l = put(g, 0, "Lee Sin, Ascetic")
    for _ in range(4):
        g.buff(l)
    assert l.buff == 4
    p = put(g, 0, "Pouty Poro")
    g.buff(p)
    g.buff(p)
    assert p.buff == 1


# Kayle, Justified (order): "I can be [Empowered] up to three times."
@T.test
def kayle_empower_max_three():
    g, _ = new()
    rich(g)
    k = put(g, 0, "Kayle, Justified")
    for _ in range(3):
        g.apply(act_options(g, 0, "Kayle, Justified")[0])
        settle(g)
    assert not act_options(g, 0, "Kayle, Justified")
    assert g.has_kw(k, "Ganking") and g.kw_value(k, "Deflect") == 3


# Spiderling (chaos): "Your deck can have any number of cards named Spiderling."
@T.test
def spiderling_any_number_in_deck():
    import cards
    assert cards.deck_problems(["Spiderling"] * 12) == []


# Volibear, Furious (fury): "When I attack, deal 5 damage split among any number of enemy units here."
def _voli(answer):
    seen = []

    def ans(opts, ctx):
        seen.append(list(opts))
        return answer(opts)
    g, _ = new(answers0={"split_damage": ans})
    v = put(g, 0, "Volibear, Furious")
    es = [make_token(g, "Recruit", 1, 1) for _ in range(6)]
    g.bfs[1].ctrl = 1
    return g, v, es, seen


@T.test
def volibear_zero_targets_is_an_option():
    g, v, es, seen = _voli(lambda opts: opts[-1])
    kinds = []
    g.effects.append(dict(on="damaged", fn=lambda g_, e, info: kinds.append(info["kind"])))
    g.apply(("move", (v.uid,), 1))
    d = g.advance()
    while not seen and d is not None and d.kind != "main":
        g.apply(g.agents[d.player].decide(g, d))
        d = g.advance()
    assert seen and () in seen[0], seen[0][-3:]
    settle(g)
    assert "ability" not in kinds                                           # no ability damage was dealt


@T.test
def volibear_at_most_five_targets_each_at_least_one():
    g, v, es, seen = _voli(lambda opts: opts[0])
    g.apply(("move", (v.uid,), 1))
    d = g.advance()
    while not seen and d is not None and d.kind != "main":
        g.apply(g.agents[d.player].decide(g, d))
        d = g.advance()
    for a in seen[0]:
        assert len(a) <= 5 and all(n >= 1 for _, n in a) and sum(n for _, n in a) <= 5, a


# Nasus (legend): "you may exhaust me to ready up to 2 runes": choosing none readies none
@T.test
def nasus_may_ready_zero_runes():
    from cardsets.test_legends import leg
    g, _ = new(answers0={"nasus_ready_rune": none_answer})
    L_ = leg(g, 0, "Nasus, Curator of the Sands")
    runes(g, 0, ["Calm"] * 7)
    hand(g, 0, "Astral Heron")
    g.apply(opt(g, 0, "Astral Heron", lambda ch: ch["loc"] == "base" and not ch.get("acc")))
    settle(g)
    assert sum(1 for r in g.p[0].runes if not r.exhausted) == 0


# Baited Hook (LeBlanc): "banish a unit from among them that has Might up to 1 more than the killed unit"
@T.test
def baited_hook_might_limit():
    from cards import _hook
    g, _ = new(answers0={"hook_pick": lambda opts, ctx: opts[1] if len(opts) > 1 else None})
    k = put(g, 0, "Shipyard Skulker")                  # 3 might
    deck_top(g, 0, ["Mountain Drake", "Vanguard Sergeant", "Pouty Poro", "Pouty Poro", "Pouty Poro"])
    offered = []
    g.agents[0].answers["hook_pick"] = lambda opts, ctx: (offered.extend(c.cname for c in opts if c), None)[1]
    hk = put(g, 0, "Baited Hook")
    runes(g, 0, ["Order"] * 2)
    g.apply(act_options(g, 0, "Baited Hook")[0])
    settle(g)
    assert "Mountain Drake" not in offered and offered, offered


# Fae Dragon: the up-to-four friendly units are chosen as targets (rule 355.10): "chosen" is emitted for each
# (Spirit Wheel, Irelia...), not at resolution without choosing
@T.test
def fae_dragon_buffed_units_are_chosen_as_targets():
    g, _ = new()
    rich(g)
    us = [put(g, 0, "Legion Rearguard") for _ in range(2)]
    seen = []
    g.effects.append(dict(on="chosen", fn=lambda g_, e, info: seen.append(info["obj"])))
    play_unit(g, "Fae Dragon")
    assert all(u in seen for u in us if u.buff), seen
    assert len([u for u in seen]) == sum(1 for u in us + named(g, 0, "Fae Dragon") if u.buff)


if __name__ == "__main__":
    T.main()
