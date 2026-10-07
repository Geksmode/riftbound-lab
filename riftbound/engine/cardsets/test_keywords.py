"""Scenario tests of the generic keyword mechanics, the auto-modelled keyword-only cards and the card-module loader.
Run: python3 cardsets/test_keywords.py   (or python3 test_all.py)"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
import cards                                            # noqa: E402
from cards import (ability, add_ability, when_mighty, repeatable, make_token, P_unit, tg_choices, enemies,  # noqa
                   friends, deck_problems, parse_cost, repeat_cost, flow_cost)
from game import ANY                                    # noqa: E402

T = Suite("keywords")


# ------------------------------------------------------------------ auto-modelled keyword-only cards
@T.test
def auto_registered_cards():
    for n in ("Blazing Scorcher", "Pouty Poro", "Sunlit Guardian", "Laurent Duelist", "Mega-Mech", "Mountain Drake",
              "Recruit (271) // Buff", "Sprite (274) // Buff", "Shen, Kinkou", "Voracious Gromp", "Sentinel Adept"):
        assert n in IMPL and IMPL[n].auto, n
    assert "Buff" not in IMPL and "XP Tracker" not in IMPL
    # reminder text does not count: Inferna's "[Reaction]" is inside Ambush reminder text
    assert IMPL["Inferna"].timing is None and IMPL["Inferna"].ambush and IMPL["Inferna"].kw == {"Assault": 2}
    assert IMPL["Shen, Kinkou"].timing == "reaction" and IMPL["Shen, Kinkou"].kw == {"Shield": 2, "Tank": 1}
    assert IMPL["Rengar, Unseen"].accelerate and IMPL["Rengar, Unseen"].kw == {"Assault": 2, "Deflect": 1, "Ganking": 1}
    # cards with other abilities are not auto-modelled, equipment neither (Might Bonus not in the database)
    # (card modules may register them by hand: then the entry is theirs, not auto_keywords')
    for n in ("Gemhand Hunter", "Seal of Rage", "Serrated Dirk", "Fiora, Victorious"):
        assert n not in IMPL or not IMPL[n].auto, n
        assert cards.auto_keywords(n, register=False) is None, n


@T.test
def shield_tank_defender_might():
    g, _ = new()
    a = put(g, 0, "Laurent Duelist")                 # 3 might, Assault 2
    d = put(g, 1, "Sunlit Guardian", 1)              # 3 might, Shield, Tank
    d2 = put(g, 1, "Pouty Poro", 1)                  # 2 might
    g.p[1].deck = []
    g.apply(("move", (a.uid,), 1))
    settle(g)
    # attacker 5 might: Tank first -> Guardian (4 as defender) gets lethal, 1 left on Poro; defenders deal 4+2
    assert d.zone == "trash" and d2.zone == "board" and a.zone == "trash", (d.zone, d2.zone, a.zone)


@T.test
def deflect_on_auto_unit_taxes_enemy_spell():
    g, _ = new()
    poro = put(g, 0, "Pouty Poro")
    other = put(g, 0, "Mountain Drake")
    c = hand(g, 1, "Stupefy")
    _, r1 = total_cost(g, 1, c, dict(tg=(poro.uid,)), "hand")
    _, r2 = total_cost(g, 1, c, dict(tg=(other.uid,)), "hand")
    assert len(r1) == len(r2) + 1 and r1[-1] == ANY


@T.test
def accelerate_auto_enters_ready():
    g, _ = new()
    runes(g, 0, ["Fury"] * 7)
    hand(g, 0, "Blazing Scorcher")
    g.apply(opt(g, 0, "Blazing Scorcher", lambda ch: ch.get("acc") and ch["loc"] == "base"))
    settle(g)
    u = [x for x in g.units(0) if x.cname == "Blazing Scorcher"][0]
    assert not u.exhausted and len(g.p[0].runes) == 6        # 6 energy + [Fury] (one rune recycled)


@T.test
def reaction_unit_in_closed_state():
    g, ag = new()
    runes(g, 0, ["Calm"] * 2 + ["Order"] * 4)
    shen = hand(g, 0, "Shen, Kinkou")
    hand(g, 0, "Discipline")
    me = put(g, 0, "Mountain Drake")
    # a non-Reaction unit cannot be played in a Closed state, Shen can (to base or a battlefield you control)
    drake2 = hand(g, 0, "Laurent Duelist")
    assert card_choices(g, 0, drake2, "hand", True, False) == []
    assert any(ch["loc"] == "base" for ch in card_choices(g, 0, shen, "hand", True, False))
    g.apply(opt(g, 0, "Discipline", lambda ch: ch.get("tg") == (me.uid,)))
    d = g.advance()
    assert d.kind == "priority" and d.player == 0
    o = [x for x in d.options if x[0] == "play" and x[1] == shen.uid]
    assert o, d.options
    g.apply(o[0])
    settle(g)
    assert shen.zone == "board" and shen.loc == "base"


@T.test
def ambush_auto_unit_in_closed_state():
    g, _ = new()
    runes(g, 0, ["Calm"] * 2 + ["Order"] * 3)
    me = put(g, 0, "Mountain Drake", 0)
    sp = hand(g, 0, "Soulspinner")
    chs = card_choices(g, 0, sp, "hand", True, False)
    assert [ch["loc"] for ch in chs] == [0], chs


@T.test
def hidden_auto_unit_can_hide():
    g, _ = new()
    runes(g, 0, ["Body"])
    put(g, 0, "Mountain Drake", 0)
    c = hand(g, 0, "Pakaa Cub")
    assert ("hide", c.uid, 0) in options_of(g, "hide")


@T.test
def temporary_sprite_token_dies():
    g, _ = new()
    t = make_token(g, "Sprite (274) // Buff", 0)
    assert g.has_kw(t, "Temporary")
    t2 = make_token(g, "Sprite", 0)
    g.apply(("end",))
    settle(g)                                       # P1's turn
    assert t in g.board and t2 in g.board
    g.apply(("end",))
    settle(g)                                       # P0's turn: Temporary kills them before scoring
    assert t not in g.board and t2 not in g.board


@T.test
def recruit_tokens_are_units():
    g, _ = new()
    for n in ("Recruit", "Recruit (271) // Buff", "Recruit (NX)", "Sand Soldier", "Tentacle", "Bird"):
        t = make_token(g, n, 0)
        assert t.spec["type"] == "Unit" and n in IMPL, n
    assert g.might(t) == 1 and g.has_kw(t, "Deflect")      # Bird


# ------------------------------------------------------------------ Vision / Predict / Burn
@T.test
def vision_predict_recycles_top_card():
    g, _ = new(answers0={"predict_recycle": True})
    runes(g, 0, ["Chaos"] * 2)
    top = deck_top(g, 0, ["Watchful Sentry", "Soaring Scout"])
    hand(g, 0, "Mystic Poro")
    g.apply(opt(g, 0, "Mystic Poro"))
    settle(g)
    pl = g.p[0]
    assert pl.deck[0] is top[1] and pl.deck[-1] is top[0], (pl.deck[:2], pl.deck[-1])


@T.test
def vision_keep_top_card():
    g, _ = new()                                    # default answer: keep (False)
    runes(g, 0, ["Chaos"] * 2)
    top = deck_top(g, 0, ["Watchful Sentry"])
    hand(g, 0, "Mystic Poro")
    g.apply(opt(g, 0, "Mystic Poro"))
    settle(g)
    assert g.p[0].deck[0] is top[0]


@T.test
def predict_two_reorders():
    g, _ = new(answers0={"predict_recycle": False, "predict_top": lambda opts, ctx: opts[-1]})
    top = deck_top(g, 0, ["Watchful Sentry", "Soaring Scout"])
    rec = g.predict(0, 2)
    assert rec == [] and g.p[0].deck[:2] == [top[1], top[0]]


@T.test
def burn_and_burn_out():
    g, _ = new()
    top = deck_top(g, 1, ["Watchful Sentry", "Soaring Scout"])
    seen = []
    g.effects.append(dict(on="burn", fn=lambda g_, eff, info: seen.append(len(info["cards"]))))
    burned = g.burn(1, 2)
    assert burned == top and top[0] in g.p[1].trash and seen == [2]
    g.p[1].deck = []
    pts = g.p[0].points
    g.burn(1, 1)                                    # empty deck: burn out, opponent gains 1 (rule 440.4)
    assert g.p[0].points == pts + 1


# ------------------------------------------------------------------ Hunt / Level / XP
@T.test
def hunt_gains_xp_on_conquer():
    g, _ = new()
    u = put(g, 0, "Voracious Gromp")
    g.apply(("move", (u.uid,), 1))
    settle(g)
    assert g.bfs[1].ctrl == 0 and g.p[0].xp == 3


@T.test
def hunt_on_hold():
    g, _ = new()
    put(g, 0, "Voracious Gromp", 1)
    g.apply(("end",)); settle(g)
    g.apply(("end",)); settle(g)                    # P0 holds at its Beginning Phase
    assert g.p[0].xp == 3, g.p[0].xp


@T.test
def level_might_keywords_and_ready():
    with temp_card("Gustwalker", kw={"Hunt": 2}, levels=[(3, dict(might=1, kw={"Ganking": 1}))]), \
            temp_card("Bandle Soldier", levels=[(3, dict(ready=True))]):
        g, _ = new()
        u = put(g, 0, "Gustwalker", 0)
        assert g.might(u) == 3 and not g.has_kw(u, "Ganking")
        g.gain_xp(0, 3)
        assert g.might(u) == 4 and g.has_kw(u, "Ganking")
        assert any(o[0] == "move" and o[2] == 1 for o in options_of(g, "move"))
        runes(g, 0, ["Order"] * 6)
        hand(g, 0, "Bandle Soldier")
        g.apply(opt(g, 0, "Bandle Soldier", lambda ch: ch["loc"] == "base"))
        settle(g)
        b = [x for x in g.units(0) if x.cname == "Bandle Soldier"][0]
        assert not b.exhausted
        assert g.spend_xp(0, 2) and not g.spend_xp(0, 5) and g.p[0].xp == 1


@T.test
def level_highest_wins():
    with temp_card("Targonian Visionary", levels=[(3, dict(might=1)), (11, dict(might=4))]):
        g, _ = new()
        u = put(g, 0, "Targonian Visionary")
        base = u.spec["might"]
        g.gain_xp(0, 12)
        assert g.might(u) == base + 4


# ------------------------------------------------------------------ Weaponmaster / Equip
@T.test
def weaponmaster_attaches_with_discount():
    with temp_card("Forgefire Cape", equip="1 rune of any type", bonus=1):
        g, _ = new()
        runes(g, 0, ["Fury"] * 3)
        cape = put(g, 0, "Forgefire Cape")
        old = put(g, 0, "Mountain Drake")
        from actions import attach
        attach(g, cape, old)
        hand(g, 0, "Sentinel Adept")
        g.apply(opt(g, 0, "Sentinel Adept", lambda ch: ch["loc"] == "base"))
        settle(g)
        adept = [x for x in g.units(0) if x.cname == "Sentinel Adept"][0]
        # [Equip] 1 rune of any type reduced by [A] -> free; moved from the other unit (rule 821)
        assert cape.attached_to == adept.uid and g.might(adept) == 4 and not old.attached
        assert len(g.p[0].runes) == 3


@T.test
def weaponmaster_pays_equip_cost():
    g, _ = new()
    runes(g, 0, ["Fury"] * 4)
    sword = put(g, 0, "Long Sword")
    hand(g, 0, "Sentinel Adept")
    g.apply(opt(g, 0, "Sentinel Adept", lambda ch: ch["loc"] == "base"))
    settle(g)
    adept = [x for x in g.units(0) if x.cname == "Sentinel Adept"][0]
    assert sword.attached_to == adept.uid and len(g.p[0].runes) == 3     # [Equip] [Fury] is not reduced


@T.test
def equip_field_builds_ability():
    with temp_card("Serrated Dirk", equip="1 fury rune", bonus=2):
        g, _ = new()
        runes(g, 0, ["Fury"])
        dirk = put(g, 0, "Serrated Dirk")
        u = put(g, 0, "Mountain Drake")
        acts = act_options(g, 0, "Serrated Dirk")
        assert acts
        g.apply(acts[0]); settle(g)
        assert dirk.attached_to == u.uid and g.might(u) == u.spec["might"] + 2


# ------------------------------------------------------------------ Add / Empower / Mighty / auras
@T.test
def add_ability_pays_power():
    with temp_card("Seal of Rage", add=[add_ability("1 fury rune")]):
        g, _ = new()
        runes(g, 0, ["Calm"] * 6)
        seal = put(g, 0, "Seal of Rage")
        assert g.can_pay(0, 1, [frozenset({"Fury"})])
        assert g.pay(0, 1, [frozenset({"Fury"})])
        assert seal.exhausted and len(g.p[0].runes) == 6
        assert not g.can_pay(0, 0, [frozenset({"Fury"})])


@T.test
def add_ability_restricted_to_spells():
    spells_only = lambda g, pid, o, ctx: ctx is not None and ctx.get("kind") == "spell"
    with temp_card("Lux, Crownguard", add=[add_ability("2 energy", can=spells_only)]):
        g, _ = new()
        runes(g, 0, ["Calm"] * 1)
        put(g, 0, "Lux, Crownguard")
        unit = hand(g, 0, "Stalwart Poro")           # 2 energy: not payable with Lux
        spell = hand(g, 0, "Discipline")             # 2 energy
        assert not card_choices(g, 0, unit, "hand", False, False)
        assert card_choices(g, 0, spell, "hand", False, False)


@T.test
def empower_field_and_event():
    seen = []
    with temp_card("Apprentice Mage", empower="2 energy", might_if=[(cards.when_empowered, 1)],
                   on_event=lambda g, o, ev, info: ev == "empowered" and info["obj"] is o and seen.append(o)):
        g, _ = new()
        runes(g, 0, ["Mind"] * 2)
        m = put(g, 0, "Apprentice Mage")
        acts = act_options(g, 0, "Apprentice Mage")
        assert acts
        g.apply(acts[0]); settle(g)
        assert m.empowered and seen == [m] and g.might(m) == 4
        assert not act_options(g, 0, "Apprentice Mage")


@T.test
def mighty_dependent_keywords_no_loop():
    with temp_card("Fiora, Victorious", kw_if=[(when_mighty, {"Deflect": 1, "Ganking": 1, "Shield": 1})]):
        g, _ = new()
        f = put(g, 0, "Fiora, Victorious")           # 4 might
        assert not g.has_kw(f, "Ganking")
        g.mod(f, 1)
        assert g.has_kw(f, "Ganking") and g.kw_value(f, "Deflect") == 1
        f.desig = "def"
        assert g.might(f) == 6


@T.test
def becomes_mighty_event():
    seen = []
    with temp_card("Fiora, Worthy", track_mighty=True,
                   on_event=lambda g, o, ev, info: ev == "becomes_mighty" and seen.append(info["obj"])):
        g, _ = new(answers0={})
        runes(g, 0, ["Calm"] * 2)
        put(g, 0, "Fiora, Worthy")
        u = put(g, 0, "Mountain Drake")
        hand(g, 0, "Discipline")
        g.need_cleanup = True; settle(g)
        assert not seen
        g.mod(u, 5 - g.might(u) - 2)
        g.apply(opt(g, 0, "Discipline", lambda ch: ch.get("tg") == (u.uid,)))
        settle(g)
        assert seen == [u], seen


@T.test
def aura_grants_vision_to_other_units():
    with temp_card("Gemcraft Seer", kw={"Vision": 1},
                   aura_kw=lambda g, src, o: {"Vision": 1} if o is not src and o.ctrl == src.ctrl
                   and o.spec["type"] == "Unit" else None):
        g, _ = new(answers0={"predict_recycle": True})
        runes(g, 0, ["Calm"] * 2)
        put(g, 0, "Gemcraft Seer")
        top = deck_top(g, 0, ["Watchful Sentry"])
        hand(g, 0, "Stalwart Poro")
        g.apply(opt(g, 0, "Stalwart Poro"))
        settle(g)
        assert g.p[0].deck[-1] is top[0]


@T.test
def aura_might():
    with temp_card("Master Yi, Wuju Master",
                   aura_might=lambda g, src, o: 1 if o.ctrl == src.owner and g.level(src.owner, 6) else 0):
        g, _ = new()
        g.p[0].legend_name = "Master Yi, Wuju Master"
        g.p[0].legend = Obj("Master Yi, Wuju Master", 0)
        u = put(g, 0, "Mountain Drake")
        e = put(g, 1, "Mountain Drake")
        b = g.might(u)
        g.gain_xp(0, 6)
        assert g.might(u) == b + 1 and g.might(e) == b


# ------------------------------------------------------------------ other helpers
@T.test
def deflect_applies_to_activated_abilities():
    stun_ab = ability("Stun", "1 energy and 1 fury rune", timing="action", exhaust=True, preds=[P_unit],
                      choices=lambda g, pid, o: tg_choices(enemies(g, pid)),
                      resolve=lambda g, it: g.stun(g.legal(it, 0), it.ctrl))
    with temp_card("Shadow", abilities=[stun_ab]):
        g, _ = new()
        runes(g, 0, ["Fury"])                        # 1 rune: 1 energy + [Fury], not the Deflect [A]
        put(g, 0, "Shadow")
        put(g, 1, "Pouty Poro")
        assert not act_options(g, 0, "Shadow")
        runes(g, 0, ["Fury"] * 2)
        acts = act_options(g, 0, "Shadow")
        assert len(g.p[0].runes) == 2
        assert acts
        g.apply(acts[0]); settle(g)
        assert g.units(1)[0].stunned


@T.test
def stun_and_buff_events():
    seen = []
    g, _ = new()
    g.effects.append(dict(on="stun", fn=lambda g_, e, info: seen.append(("stun", info["obj"]))))
    g.effects.append(dict(on="buff", fn=lambda g_, e, info: seen.append(("buff", info["obj"]))))
    u = put(g, 1, "Mountain Drake")
    g.stun(u, 0); g.stun(u, 0)
    assert g.buff(u) and not g.buff(u)
    assert seen == [("stun", u), ("buff", u)] and g.might(u) == u.spec["might"] + 1


@T.test
def legion_helper():
    g, _ = new()
    runes(g, 0, ["Calm"] * 4)
    c = hand(g, 0, "Stalwart Poro")
    assert not g.legion(0, c)
    hand(g, 0, "Discipline")
    me = put(g, 0, "Mountain Drake")
    g.apply(opt(g, 0, "Discipline", lambda ch: ch.get("tg") == (me.uid,)))
    settle(g)
    assert g.legion(0, c)
    g.apply(opt(g, 0, "Stalwart Poro"))
    settle(g)
    assert g.legion(0, c)                            # another card (Discipline) was finalized


@T.test
def repeatable_runs_twice():
    def one(g, it):
        make_token(g, "Sand Soldier", it.ctrl)
    with temp_card("Desert's Call", repeat=repeat_cost("2 energy"), resolve=repeatable(one)):
        g, _ = new()
        runes(g, 0, ["Calm"] * 4)
        hand(g, 0, "Desert's Call")
        g.apply(opt(g, 0, "Desert's Call", lambda ch: ch.get("rep")))
        settle(g)
        assert len([u for u in g.units(0) if u.cname == "Sand Soldier"]) == 2


@T.test
def cost_parsers():
    assert parse_cost("2 energy and 1 fury rune") == (2, [frozenset({"Fury"})])
    assert parse_cost("1 energy and 2 runes of any type") == (1, [ANY, ANY])
    assert parse_cost("2 body runes") == (0, [frozenset({"Body"})] * 2)
    assert repeat_cost("2 energy")[0] == 2 and repeat_cost("2 energy")[2] == 0
    assert repeat_cost("1 chaos rune") == (0, ("Chaos",), 1)
    assert flow_cost("4 energy and 1 fury rune") == (4, 1, ("Fury",))


@T.test
def unique_deck_constraint():
    # Forgefire Cape is [Unique]; take it out of IMPL for the test so that "not modelled" is checked too
    old = IMPL.pop("Forgefire Cape", None)
    try:
        probs = deck_problems(["Sunlit Guardian", "Sunlit Guardian", "Forgefire Cape", "Forgefire Cape"])
    finally:
        if old is not None:
            IMPL["Forgefire Cape"] = old
    assert "Forgefire Cape: [Unique], 2 copies" in probs and "Forgefire Cape: not modelled" in probs
    assert not [p for p in probs if p.startswith("Sunlit")]
    assert deck_problems(dict(A)) == [], deck_problems(dict(A))


@T.test
def duplicate_registration_is_an_error():
    try:
        cards.card("Sunlit Guardian")
    except ValueError as e:
        assert "already registered" in str(e)
    else:
        raise AssertionError("no error")
    try:
        cards.card("Not A Card")
    except KeyError:
        pass
    else:
        raise AssertionError("no error")
    try:
        cards.Impl("X", tming="action")
    except TypeError:
        pass
    else:
        raise AssertionError("typo in a field name must fail")


@T.test
def loader_imports_listed_modules_and_refuses_duplicates():
    import tempfile, shutil
    d = tempfile.mkdtemp()
    old = IMPL.pop("Gemhand Hunter", None)              # a module may have registered it: restore it afterwards
    env = os.environ.pop("RB_CARDSETS", None)           # the module filter would skip alpha/beta
    try:
        os.makedirs(os.path.join(d, "tmpsets_kw"))
        open(os.path.join(d, "tmpsets_kw", "__init__.py"), "w").write('MODULES = ["alpha", "beta"]\n')
        open(os.path.join(d, "tmpsets_kw", "alpha.py"), "w").write(
            "from cards import *\n"
            "card('Gemhand Hunter', kw={'Hunt': 1}, levels=[(6, dict(might=1))])\n")
        open(os.path.join(d, "tmpsets_kw", "beta.py"), "w").write(
            "from cards import *\ncard('Gemhand Hunter')\n")
        sys.path.insert(0, d)
        try:
            cards.load_cardsets("tmpsets_kw")
        except ValueError as e:
            assert "tmpsets_kw.alpha" in str(e), e          # the error names the first module
        else:
            raise AssertionError("duplicate not detected")
        assert IMPL["Gemhand Hunter"].module == "tmpsets_kw.alpha" and IMPL["Gemhand Hunter"].levels
    finally:
        sys.path.remove(d)
        IMPL.pop("Gemhand Hunter", None)
        if old is not None:
            IMPL["Gemhand Hunter"] = old
        if env is not None:
            os.environ["RB_CARDSETS"] = env
        for m in ("tmpsets_kw", "tmpsets_kw.alpha", "tmpsets_kw.beta"):
            sys.modules.pop(m, None)
        shutil.rmtree(d)


@T.test
def module_list_is_explicit_and_sorted():
    import cardsets
    assert cardsets.MODULES == sorted(cardsets.MODULES)
    for m in cardsets.MODULES:
        assert os.path.exists(os.path.join(os.path.dirname(cardsets.__file__), m + ".py")), m


if __name__ == "__main__":
    T.main()
