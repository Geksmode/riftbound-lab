"""Helpers for the scenario tests of card modules (cardsets/test_<batch>.py). See cardsets/GUIDE.md.

    from testkit import *          # new, put, hand, runes, settle, opt, deck_top, Suite, temp_card, ...
    T = Suite("my_batch")

    @T.test
    def my_card_does_x():
        g, ag = new()
        ...

    if __name__ == "__main__":
        T.main()
"""
import os, sys, traceback
from contextlib import contextmanager

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_cards import Scripted, new, runes, put, hand, settle, opt, A, L     # noqa: E402,F401
from game import Game, Rune, Obj, SPEC, EQUIP_BONUS                              # noqa: E402,F401
import cards                                                                     # noqa: E402
from cards import IMPL                                                           # noqa: E402,F401
from actions import total_cost, card_choices, main_options, play_options, ability_options   # noqa: E402,F401


def deck_top(g, pid, names):
    """Put new cards with these names on top of pid's Main Deck (first name = top card). Returns them."""
    out = []
    for n in reversed(names):
        o = Obj(n, pid)
        o.zone = "deck"
        g.p[pid].deck.insert(0, o)
        out.append(o)
    return list(reversed(out))


def champ(g, pid, name):
    o = Obj(name, pid)
    o.zone = "champ"
    g.p[pid].champ.append(o)
    return o


def options_of(g, kind=None):
    """Options of the current top-level decision (optionally only those whose first element is kind)."""
    d = g.advance()
    return [o for o in d.options if kind is None or o[0] == kind]


def act_options(g, pid, cname=None):
    """Activated ability options of pid ('act' tuples), optionally only for a source named cname."""
    out = []
    for o in options_of(g, "act"):
        src = o[1]
        name = g.p[pid].legend_name if isinstance(src, tuple) else (g.obj(src).cname if g.obj(src) else None)
        if cname is None or name == cname:
            out.append(o)
    return out


@contextmanager
def temp_card(name, **kw):
    """Register a card Impl for the duration of a test (or reuse the real one if the name is already modelled
    and kw is empty). Restores IMPL / EQUIP_BONUS / AURA / ADDERS afterwards."""
    if name in IMPL and not kw:
        yield IMPL[name]
        return
    old = IMPL.pop(name, None)
    old_bonus = EQUIP_BONUS.get(name)
    aura, adders = set(cards.AURA), set(cards.ADDERS)
    tm = list(cards.TRACK_MIGHTY)
    hooks = {k: set(v) for k, v in cards.HOOKS.items()}
    conv = list(cards.CONVERTERS)
    try:
        yield cards.card(name, **kw)
    finally:
        IMPL.pop(name, None)
        if old is not None:
            IMPL[name] = old
        if old_bonus is None:
            EQUIP_BONUS.pop(name, None)
        else:
            EQUIP_BONUS[name] = old_bonus
        cards.AURA.clear(); cards.AURA.update(aura)
        cards.ADDERS.clear(); cards.ADDERS.update(adders)
        cards.TRACK_MIGHTY[:] = tm
        cards.HOOKS.clear(); cards.HOOKS.update(hooks)
        cards.CONVERTERS[:] = conv


class Suite:
    """A list of scenario tests with its own runner (prints 'N/M tests passed')."""
    def __init__(s, name):
        s.name = name
        s.tests = []

    def test(s, f):
        s.tests.append(f)
        return f

    def run(s):
        ok, fails = 0, []
        for t in s.tests:
            try:
                t()
                ok += 1
            except Exception as e:
                fails.append((t.__name__, e))
                traceback.print_exc()
        print(f"{ok}/{len(s.tests)} tests passed ({s.name})")
        for n, e in fails:
            print("FAIL", n, repr(e)[:200])
        return ok, fails

    def main(s):
        _, fails = s.run()
        sys.exit(1 if fails else 0)
