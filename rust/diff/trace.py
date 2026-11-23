"""Test différentiel Python / Rust : journal d'une partie du moteur Python de référence.

Deux RandomAgent (agents.py) jouent ; chaque question (g.ask) et chaque décision de haut niveau est écrite avec ses
options (écriture canonique, la même que rust/src/trace.rs) et l'indice choisi, plus un résumé de l'état à chaque
décision. Le moteur Rust doit écrire exactement le même journal (rust/src/bin/trace.rs).

    python3 rust/diff/trace.py <graine> <paquet0> <paquet1> [graine_agent0 graine_agent1]  > journal.txt
"""
import json, os, sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "riftbound", "engine"))
import game  # noqa: E402
from game import Game, Obj, Item, Rune, Opt, Battlefield  # noqa: E402
import cards  # noqa: E402,F401  (charge le registre des cartes)
from agents import RandomAgent  # noqa: E402
import decks  # noqa: E402


def canon(v):
    """Écriture canonique d'une valeur (options de g.ask, actions)."""
    if v is None:
        return "N"
    if v is True:
        return "T"
    if v is False:
        return "F"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, float):
        return repr(v)
    if isinstance(v, str):
        return json.dumps(v, ensure_ascii=False)
    if isinstance(v, Obj):
        return f"O{v.uid}"
    if isinstance(v, Rune):
        return f"R{v.uid}"
    if isinstance(v, Item):
        return f"I{v.id}"
    if isinstance(v, Battlefield):
        return f"B{v.idx}"
    if isinstance(v, Opt):
        return f"Opt({json.dumps(v.label, ensure_ascii=False)}|{canon(v.value)})"
    if isinstance(v, (list, tuple)):
        return "[" + ",".join(canon(x) for x in v) + "]"
    if isinstance(v, (set, frozenset)):
        return "[" + ",".join(sorted(canon(x) for x in v)) + "]"
    if isinstance(v, dict):
        return "{" + ",".join(f"{canon(k) if not isinstance(k, str) else json.dumps(k)}:{canon(x)}"
                              for k, x in sorted(v.items(), key=lambda kv: str(kv[0]))) + "}"
    if callable(v):
        return "F"
    return "?" + type(v).__name__


def loc(l):
    return "N" if l is None else ("base" if l == "base" else str(l))


def state(g):
    """Résumé de l'état (écrit à chaque décision de haut niveau)."""
    parts = [f"t{g.turn_no} tp{g.tp} st={g.stage} pts={g.p[0].points}-{g.p[1].points} xp={g.p[0].xp}-{g.p[1].xp}"]
    for pl in g.p:
        parts.append(f"P{pl.pid} h={len(pl.hand)} d={len(pl.deck)} tr={len(pl.trash)} b={len(pl.banish)} "
                     f"r={''.join(repr(r) for r in pl.runes)} rd={len(pl.rune_deck)} e={pl.pool_e}")
    for o in g.board:
        parts.append(f"{o.cname}#{o.uid}@{loc(o.loc)} c{o.ctrl} m{g.might(o)} d{o.damage} "
                     f"{'x' if o.exhausted else ''}{'s' if o.stunned else ''}{'b' if o.buff else ''}")
    for b in g.bfs:
        parts.append(f"bf{b.idx}:{b.name} c={b.ctrl} k={b.contested}")
    return " | ".join(parts)


class Trace:
    def __init__(s, inner, out):
        s.inner, s.out = inner, out

    def start(s, g):
        s.inner.start(g)

    def mulligan(s, g, pid):
        r = s.inner.mulligan(g, pid)
        s.out.append(f"mul P{pid} {canon(r)}")
        return r

    def decide(s, g, d):
        a = s.inner.decide(g, d)
        s.out.append(f"st {state(g)}")
        i = next(i for i, o in enumerate(d.options) if o is a or o == a)
        s.out.append(f"dec {d.kind} P{d.player} n={len(d.options)} -> {i} {canon(a)} :: "
                     + " ".join(canon(o) for o in d.options))
        return a

    def choose(s, g, pid, kind, options, ctx):
        r = s.inner.choose(g, pid, kind, options, ctx)
        i = next((i for i, o in enumerate(options) if o is r), None)
        s.out.append(f"ask {kind} P{pid} n={len(options)} -> {i} :: " + " ".join(canon(o) for o in options))
        return r


def run(seed, k0, k1, a0=1, a1=2, max_decisions=5000):
    Obj._n = 0
    Item._n = 0
    out = []
    d0, d1 = decks.load(k0), decks.load(k1)
    agents = [Trace(RandomAgent(a0), out), Trace(RandomAgent(a1), out)]
    g = game.play_game([d0, d1], agents, seed=seed, max_decisions=max_decisions)
    out.append(f"fin winner={g.winner} t{g.turn_no} pts={g.p[0].points}-{g.p[1].points}")
    return out


if __name__ == "__main__":
    a = sys.argv[1:]
    seed, k0, k1 = int(a[0]), a[1], a[2]
    a0, a1 = (int(a[3]), int(a[4])) if len(a) >= 5 else (1, 2)
    print("\n".join(run(seed, k0, k1, a0, a1)))
