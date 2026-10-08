"""Deflect (règle 809) sur TOUS les sorts modélisés : chaque fois qu'un sort choisit une unité ennemie avec Deflect N,
il coûte N puissance de n'importe quel domaine en plus (809.1.c), répétitions (Repeat, 820) comprises.
Méthode : même choix, une fois sur Pouty Poro (Deflect 1), une fois sur Mournful Witness (sans Deflect) ; le surcoût
doit être exactement 1 par fois où Poro est choisi. Run: python3 cardsets/test_deflect_auto.py"""
import csv, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from actions import card_choices, total_cost            # noqa: E402
from game import ANY                                    # noqa: E402

T = Suite("deflect_auto")
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ALL = ["Fury", "Calm", "Mind", "Body", "Chaos", "Order"] * 2
SPELLS = sorted(r["name"] for r in csv.DictReader(open(os.path.join(ROOT, "cards", "cards_unique.csv"), encoding="utf-8"))
                if r["type"] == "Spell" and r["name"] in IMPL)


def occ(x, uid):
    if isinstance(x, (list, tuple)):
        return sum(occ(y, uid) for y in x)
    if isinstance(x, dict):
        return sum(occ(v, uid) for k, v in x.items() if not str(k).startswith("_"))
    return 1 if x == uid else 0


def swap(x, a, b):
    if isinstance(x, tuple):
        return tuple(swap(y, a, b) for y in x)
    if isinstance(x, list):
        return [swap(y, a, b) for y in x]
    if isinstance(x, dict):
        return {k: swap(v, a, b) for k, v in x.items()}
    return b if x == a else x


@T.test
def every_spell_pays_deflect_each_time_it_chooses():
    bad, checked = [], 0
    for name in SPELLS:
        for loc in (0, "base"):
            g, _ = new()
            runes(g, 0, ALL)
            poro, wit = put(g, 1, "Pouty Poro", loc), put(g, 1, "Mournful Witness", loc)
            put(g, 0, "Mournful Witness", loc)
            c = hand(g, 0, name)
            for ch in card_choices(g, 0, c, "hand", False, False, every=True):
                n = occ(ch, poro.uid)
                if not n:
                    continue
                q1 = total_cost(g, 0, c, ch, "hand")[1]
                q2 = total_cost(g, 0, c, swap(ch, poro.uid, wit.uid), "hand")[1]
                checked += 1
                if q1.count(ANY) - q2.count(ANY) != n:
                    bad.append(f"{name} : Poro choisi {n} fois, surcoût {q1.count(ANY) - q2.count(ANY)} ({ch})")
                    break
    assert checked > 200 and not bad, f"{checked} choix\n" + "\n".join(bad)


@T.test
def repeat_target_with_deflect_recycles_one_more_rune():
    used = {}
    for target in ("Pouty Poro", "Mournful Witness"):
        g, _ = new()
        g.every_choice = True
        runes(g, 0, ALL)
        me, e = put(g, 0, "Mournful Witness"), put(g, 1, target)
        hand(g, 0, "Blood Rush")
        n0 = len(g.p[0].runes)
        g.apply(opt(g, 0, "Blood Rush", lambda x: x.get("rep") and tuple(x.get("tg2", ())) == (e.uid,)
                    and tuple(x.get("tg", ())) == (me.uid,)))
        settle(g)
        used[target] = n0 - len(g.p[0].runes)
    assert used["Pouty Poro"] == used["Mournful Witness"] + 1, used


if __name__ == "__main__":
    T.main()
