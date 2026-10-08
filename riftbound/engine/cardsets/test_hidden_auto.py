"""Hidden (règles 421, 811) sur TOUTES les cartes : une carte qui a le mot-clé se cache (1 rune au choix, à un
battlefield que je contrôle et qui n'a pas déjà de carte face cachée, pendant mon tour en état ouvert) et se joue depuis
la face cachée à partir du tour suivant pour 0 énergie ; une carte qui ne fait que MENTIONNER [Hidden] (Ember Monk,
Ava Achiever, Pack of Wonders…) ne se cache pas. Plus les cibles de Back Off et Switcheroo pour un joueur humain.
Run: python3 cardsets/test_hidden_auto.py"""
import csv, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from actions import timed_options                       # noqa: E402

T = Suite("hidden_auto")
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ROWS = list(csv.DictReader(open(os.path.join(ROOT, "cards", "cards_unique.csv"), encoding="utf-8")))
# Le mot-clé est imprimé en tête du texte ; la colonne « keywords » du CSV range aussi les simples mentions.
HAS_KW = sorted(r["name"] for r in ROWS if r["name"] in IMPL and r["text"].startswith(("[Hidden]", "Hidden (")))
MENTION = sorted(r["name"] for r in ROWS if r["name"] in IMPL and r["name"] not in HAS_KW
                 and ("Hidden" in r["keywords"].split("|") or "[Hidden]" in r["text"]))


def hides(g, uid):
    return [o for o in options_of(g) if o[0] == "hide" and o[1] == uid]


def board(g):
    """Deux unités à moi au battlefield 0 (Might 5 et 3) que je contrôle, une à ma base, une ennemie au battlefield 0."""
    put(g, 0, "Brazen Buccaneer", 0)
    put(g, 0, "Arena Kingpin", 0)
    put(g, 0, "Arena Kingpin", "base")
    put(g, 1, "Ancient Warmonger", 0)
    g.bfs[0].ctrl = 0


def to_my_decision(g):
    d = g.advance()
    for _ in range(6):
        if d is None or d.player == 0:
            return d
        g.apply(("pass",) if ("pass",) in d.options else d.options[0])
        d = g.advance()
    return d


@T.test
def every_hidden_card_can_be_hidden():
    assert len(HAS_KW) >= 35, HAS_KW
    bad = []
    for n in HAS_KW:
        for exhausted in (False, True):            # payer [A] recycle une rune : elle peut être épuisée
            g, _ = new()
            runes(g, 0, ["Calm"])
            for r in g.p[0].runes:
                r.exhausted = exhausted
            c = hand(g, 0, n)
            g.bfs[0].ctrl = 0
            if hides(g, c.uid) != [("hide", c.uid, 0)]:
                bad.append((n, exhausted))
    assert not bad, bad


@T.test
def cards_that_only_mention_hidden_cannot_be_hidden():
    # 811.1 : Hidden est un mot-clé ; le texte « a card with [Hidden] » ne le donne pas (images OGN 107, 167, 181)
    for n in ("Ember Monk", "Ava Achiever", "Pack of Wonders"):
        assert n in MENTION, (n, MENTION)
    bad = []
    for n in MENTION:
        g, _ = new()
        runes(g, 0, ["Calm"] * 3)
        c = hand(g, 0, n)
        g.bfs[0].ctrl = 0
        if hides(g, c.uid):
            bad.append(n)
    assert not bad, bad


@T.test
def hide_needs_a_controlled_battlefield_a_free_slot_and_a_rune():
    g, _ = new()
    runes(g, 0, ["Calm"])
    c = hand(g, 0, "Back Off")
    assert not hides(g, c.uid)                     # 421.1 : aucun battlefield contrôlé
    g.bfs[1].ctrl = 1
    assert not hides(g, c.uid)                     # celui de l'adversaire non plus
    g.bfs[0].ctrl = 0
    assert hides(g, c.uid) == [("hide", c.uid, 0)]
    runes(g, 0, [])
    assert not hides(g, c.uid)                     # 811.1.b : il faut payer [A]
    runes(g, 0, ["Calm"])
    other = Obj("Block", 0)
    other.zone, other.hidden_turn, other.hidden_bf = "facedown", g.turn_no, 0
    g.bfs[0].facedowns.append(other)
    assert not hides(g, c.uid)                     # 811.1.b : déjà une carte face cachée là


@T.test
def no_hide_outside_my_open_main_turn():
    # 410.1.a / 421.2 : action discrétionnaire, seulement pendant mon tour en état ouvert neutre
    g, _ = new()
    runes(g, 0, ["Calm"])
    hand(g, 0, "Back Off")
    g.bfs[0].ctrl = 0
    assert not [o for o in timed_options(g, 0, closed=False, every=True) if o[0] == "hide"]


@T.test
def hide_pays_one_rune_and_puts_the_card_facedown():
    g, _ = new()
    runes(g, 0, ["Calm", "Fury"])
    c = hand(g, 0, "Back Off")
    g.bfs[0].ctrl = 0
    g.apply(("hide", c.uid, 0))
    assert c.zone == "facedown" and g.bfs[0].facedowns == [c] and c not in g.p[0].hand
    assert len(g.p[0].runes) == 1                  # une rune recyclée


@T.test
def every_hidden_card_plays_from_facedown_next_turn_for_free():
    bad = []
    for n in HAS_KW:
        for same_turn in (False, True):
            g, _ = new()
            g.every_choice = True
            runes(g, 0, [])                        # 0 énergie : le coût de base est ignoré (811.1.b)
            board(g)
            c = Obj(n, 0)
            c.zone, c.hidden_bf = "facedown", 0
            c.hidden_turn = g.turn_no if same_turn else g.turn_no - 1
            g.bfs[0].facedowns.append(c)
            d = to_my_decision(g)
            ok = bool(d) and any(a[0] == "play" and a[1] == c.uid for a in d.options)
            if ok == same_turn:                    # jouable seulement « beginning on the next turn »
                bad.append((n, "même tour" if same_turn else "tour suivant"))
    assert not bad, bad


@T.test
def back_off_targets_any_unit_for_a_human():
    # « [Stun] a unit » : une unité amie ou déjà étourdie est une cible légale
    g, _ = new()
    g.every_choice = True
    runes(g, 0, ["Calm"] * 4)
    hand(g, 0, "Back Off")
    mine = put(g, 0, "Arena Kingpin", "base")
    foe = put(g, 1, "Ancient Warmonger", "base")
    foe.stunned = True
    tgs = {dict(o[3]).get("tg", (None,))[0] for o in options_of(g, "play")}
    assert mine.uid in tgs and foe.uid in tgs, tgs


@T.test
def back_off_from_facedown_only_targets_units_there():
    g, _ = new()
    g.every_choice = True
    runes(g, 0, [])
    board(g)
    far = put(g, 1, "Arena Kingpin", "base")
    c = Obj("Back Off", 0)
    c.zone, c.hidden_bf, c.hidden_turn = "facedown", 0, g.turn_no - 1
    g.bfs[0].facedowns.append(c)
    d = to_my_decision(g)
    tgs = {dict(a[3])["tg"][0] for a in d.options if a[0] == "play" and a[1] == c.uid}
    assert tgs and far.uid not in tgs and all(g.obj(u).loc == 0 for u in tgs), tgs


@T.test
def switcheroo_two_units_with_the_same_might_for_a_human():
    g, _ = new()
    g.every_choice = True
    runes(g, 0, ["Chaos"] * 4)
    hand(g, 0, "Switcheroo")
    a = put(g, 0, "Arena Kingpin", 0)
    b = put(g, 1, "Arena Kingpin", 1)
    b2 = put(g, 0, "Crowd Favorite", 1)           # Might 3 aussi
    pairs = {tuple(sorted(dict(o[3])["tg"])) for o in options_of(g, "play")}
    assert tuple(sorted((b.uid, b2.uid))) in pairs, pairs
    assert a.uid not in {u for p in pairs for u in p}   # seule au battlefield 0


@T.test
def the_table_never_names_the_card_the_opponent_hides():
    # 421/811 : une carte cachée est face cachée ; l'annonce du coup de l'IA dit « cache une carte à … », sans son nom
    import json, train
    seen = 0
    for seed in range(1, 40):
        train.new(seed, None, 0)
        v = json.loads(train.step())
        for _ in range(600):
            while v.get("busy"):
                v = json.loads(train.step())
                ai = v.get("ai") or ""
                if ai.startswith("cache "):
                    seen += 1
                    assert ai in [f"cache une carte à {b.name}" for b in train.W["g"].bfs], ai
                    assert (v.get("aii") or {}).get("src") is None, v.get("aii")
            if v.get("winner") is not None or seen >= 2:
                break
            if v.get("ask"):
                v = json.loads(train.answer(json.dumps([] if v["ask"]["kind"] == "mulligan" else 0)))
            elif v.get("dec"):
                ops = v["dec"]["options"]
                i = next((o["i"] for o in ops if o["k"] in ("end", "pass")), 0)
                v = json.loads(train.act(i))
            else:
                v = json.loads(train.step())
        if seen >= 2:
            break
    assert seen >= 1, "l'IA n'a caché aucune carte sur ces graines"


if __name__ == "__main__":
    T.main()
