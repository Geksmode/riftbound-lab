"""Fuzz the engine with random agents on decks that contain given cards (default: every auto-modelled card, or
the cards of a card module). Every legal action must be executable; the same seed must give the same game.

  python3 fuzz_cards.py                       # 50 games, decks with the auto-modelled keyword-only cards
  python3 fuzz_cards.py 100 --module my_batch # 100 games with the cards registered by cardsets/my_batch.py
  python3 fuzz_cards.py 20 "Sunlit Guardian" "Pouty Poro"
"""
import sys, re, traceback, hashlib
from game import Game, SPEC, DOMAINS, Obj, Item
from agents import RandomAgent
from decks import load, DECKS
import cards


def playable(name):
    sp = SPEC[name]
    return sp["type"] in ("Unit", "Spell", "Gear") and not cards.is_token(name) and name in cards.IMPL \
        and name not in cards.NOT_CARDS


def build(base_key, names, rng):
    """A tournament deck with up to 18 main cards replaced by copies of names, runes covering their domains."""
    d = load(base_key)
    names = [n for n in names if playable(n)]
    if not names:
        return d
    pick = [rng.choice(names) for _ in range(18)]
    main = list(d["main"])
    rng.shuffle(main)
    d["main"] = main[:len(main) - len(pick)] + pick
    doms = sorted(set(x for n in pick for x in SPEC[n]["domains"] if x in DOMAINS))
    if doms:
        runes = list(d["runes"])
        for i in range(min(6, len(runes))):
            runes[i] = doms[i % len(doms)]
        d["runes"] = runes
    return d


def play(seed, names):
    import random
    rng = random.Random(seed)
    keys = sorted(DECKS)
    decks = [build(keys[(seed + i * 5) % len(keys)], names, rng) for i in range(2)]
    ag = [RandomAgent(seed), RandomAgent(seed + 7919)]
    Obj._n = Item._n = 0                    # object numbers appear in logs: restart them for each game
    g = Game(decks, ag, seed=seed, log=True)
    n = 0
    while True:
        d = g.advance()
        if d is None or n > 4000:
            break
        g.apply(ag[d.player].decide(g, d))
        n += 1
    return g


def digest(g):
    """Hash of the game log, object numbers removed (they are global counters)."""
    return hashlib.md5("\n".join(re.sub(r"#\d+", "", l) for l in g.lines).encode()).hexdigest()


def main(argv):
    n = int(argv[0]) if argv and argv[0].isdigit() else 50
    rest = argv[1:] if argv and argv[0].isdigit() else argv
    if rest[:1] == ["--module"]:
        names = sorted(k for k, v in cards.IMPL.items() if v.module == "cardsets." + rest[1])
    elif rest:
        names = rest
    else:
        names = sorted(cards.AUTO)
    missing = [x for x in names if x not in cards.IMPL]
    if missing:
        sys.exit(f"not modelled: {missing}")
    errors = 0
    played = set()
    for seed in range(n):
        try:
            g = play(seed, names)
            h1 = digest(g)
            if seed < 5:                                  # determinism: same seed, same game
                h2 = digest(play(seed, names))
                assert h1 == h2, f"seed {seed}: two runs differ"
            played |= set(k[5:] for k in g.stats if isinstance(k, str) and k.startswith("play_"))
        except Exception:
            errors += 1
            print(f"seed {seed}: EXCEPTION")
            traceback.print_exc()
    hit = sorted(set(names) & played)
    print(f"{n} games, {errors} exceptions; {len(hit)}/{len(names)} of the cards were played at least once")
    never = sorted(set(n_ for n_ in names if playable(n_)) - played)
    if never:
        print("never played:", ", ".join(never))
    return errors


if __name__ == "__main__":
    sys.exit(1 if main(sys.argv[1:]) else 0)
