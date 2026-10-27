#!/usr/bin/env python3
"""Empreinte du jeu de l'IA : garde-fou des optimisations du moteur (porte d'intégration, étape « empreinte »).

    python3 empreinte.py          compare à empreinte.json : code 1 si une décision a changé
    python3 empreinte.py --maj    réécrit empreinte.json (changement voulu du jeu : carte, règle, IA)

Parties à graines fixes (exp_search.setup « defaut » : deux decks par défaut tirés au hasard), jouées par la recherche
de l'IA : « old » (rapide) et « turn » (celle de la table). Chaque décision prise (joueur, type, coup choisi) et le
résultat final entrent dans un MD5 par partie. Une optimisation pure (vitesse) ne doit changer AUCUNE empreinte ; si
une change alors que le jeu ne devait pas changer, l'optimisation a un effet de bord. Une partie par processus : vérifie
aussi que même graine = même partie d'un processus à l'autre (ordre des set, hachage des textes).
Le temps est donné à titre indicatif (il dépend de la machine) : comparer avant / après sur la même machine."""
import sys, json, time, hashlib
from multiprocessing import Pool
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
REF = HERE / "empreinte.json"
# (graine, recherche) : quatre parties « old » et une « turn » (moins de deux minutes sur 4 cœurs)
PARTIES = [(910001, "old"), (910002, "old"), (910003, "old"), (910004, "old"), (910005, "turn")]


def partie(arg):
    seed, search = arg
    _argv, sys.argv = sys.argv, sys.argv[:1]      # exp.py (importé par exp_search) lit sys.argv
    import exp_search as E
    sys.argv = _argv
    import plans as P
    from game import Game, Obj, Item
    Obj._n = 0
    Item._n = 0
    t0 = time.perf_counter()
    decks, pls, f = E.setup(seed, "defaut")
    ag = [P.PlanAgent(seed + 500000 * i, plan=pls[i], opp_plan=pls[1 - i], search=search) for i in range(2)]
    g = Game(decks, ag, seed=seed, first=f)
    h, n = hashlib.md5(), 0
    while True:
        d = g.advance()
        if d is None:
            break
        a = ag[d.player].decide(g, d)
        h.update(repr((d.player, d.kind, a)).encode())
        g.apply(a)
        n += 1
    h.update(repr((g.winner, g.turn_no, [p.points for p in g.p])).encode())
    return dict(seed=seed, search=search, legends=[x["legend"] for x in decks], decisions=n, winner=g.winner,
                turns=g.turn_no, md5=h.hexdigest(), s=round(time.perf_counter() - t0, 1))


def main(maj=False):
    t0 = time.time()
    with Pool(min(4, len(PARTIES))) as p:
        out = p.map(partie, PARTIES, chunksize=1)
    total = round(time.time() - t0, 1)
    if maj:
        from version import engine_md5
        json.dump(dict(engine_md5=engine_md5(), python=sys.version.split()[0], seconds=total, games=out),
                  open(REF, "w"), ensure_ascii=False, indent=1)
        print(f"empreinte.json réécrit : {len(out)} parties, {total} s")
        return 0
    ref = {(r["seed"], r["search"]): r for r in json.load(open(REF))["games"]} if REF.exists() else {}
    bad = 0
    for r in out:
        x = ref.get((r["seed"], r["search"]))
        ok = x is not None and x["md5"] == r["md5"]
        bad += not ok
        why = "" if ok else (" (pas de référence)" if x is None else
                             f" : décisions changées (référence {x['decisions']} décisions, gagnant {x['winner']}, "
                             f"{x['turns']} tours ; maintenant {r['decisions']}, {r['winner']}, {r['turns']})")
        print(f"{'OK' if ok else 'ÉCHEC'}  graine {r['seed']} {r['search']:4} {' / '.join(r['legends'])} : "
              f"{r['decisions']} décisions, {r['s']} s{why}")
    if bad:
        print("ÉCHEC : le jeu de l'IA a changé. Si c'est voulu (carte, règle, réglage de l'IA), lance "
              "« python3 riftbound/engine/empreinte.py --maj » et committe empreinte.json avec le changement ; "
              "sinon, une optimisation a un effet de bord.")
        return 1
    print(f"empreinte : {len(out)} parties identiques à la référence, {total} s")
    return 0


if __name__ == "__main__":
    sys.exit(main("--maj" in sys.argv[1:]))
