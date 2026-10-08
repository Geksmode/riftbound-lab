#!/usr/bin/env python3
"""Compare deux résultats de simulation en appliquant les règles du projet, ou en regroupe plusieurs.

  python3 compare.py A.json:SEL B.json:SEL           écart A - B (± écart-type), apparié si mêmes graines
  python3 compare.py --pool A.json:SEL B.json:SEL... regroupe des blocs de la MÊME configuration

SEL = index dans la liste des résultats, ou un morceau de « label » (premier résultat qui le contient). Les fichiers
de session du manager ({"engine_md5", "results": [...]}) sont acceptés. Refus (code 3) si :
- un résultat n'a pas de version du moteur (engine_md5) : le remesurer ;
- les versions du moteur diffèrent ;
- les graines se recouvrent en partie (ni appariées, ni indépendantes) ;
- --pool sur des blocs qui partagent des graines (ce ne sont pas des parties indépendantes).
Verdict « on ne sait pas » sous 2 écarts-types."""
import json, math, sys
from version import scores

REFUS = 3


def load(spec):
    path, _, sel = spec.rpartition(":") if ":" in spec else (spec, "", "0")
    d = json.load(open(path))
    top = d.get("engine_md5") if isinstance(d, dict) else None
    items = d.get("results", []) if isinstance(d, dict) else d
    if sel.lstrip("-").isdigit():
        e = items[int(sel)]
    else:
        e = next((x for x in items if sel in str(x.get("label", x.get("A", "")))), None)
        if e is None:
            sys.exit(f"{path} : aucun résultat ne contient « {sel} »")
    e = dict(e)
    if top and not e.get("engine_md5"):
        e["engine_md5"] = top
    e["_nom"] = f"{path}:{e.get('label') or e.get('A', sel)}"
    return e


def refus(msg):
    print("REFUS :", msg)
    sys.exit(REFUS)


def check_version(es):
    for e in es:
        if not e.get("engine_md5"):
            refus(f"{e['_nom']} n'a pas de version du moteur (engine_md5) : le remesurer avec le moteur actuel.")
    v = {e["engine_md5"] for e in es}
    if len(v) > 1:
        refus("versions du moteur différentes (" + ", ".join(f"{e['_nom']} = {e['engine_md5']}" for e in es) +
              ") : ne jamais agréger ni comparer deux versions (sauf équivalence prouvée par manager/eqtest.py).")
    return v.pop()


def mean_se(xs):
    n = len(xs)
    m = sum(xs) / n
    sd = math.sqrt(sum((x - m) ** 2 for x in xs) / (n - 1)) if n > 1 else 0.0
    return m, sd / math.sqrt(n)


def verdict(d, se):
    z = d / se if se else float("inf") if d else 0.0
    txt = "écart mesurable (≥ 2 écarts-types)" if abs(z) >= 2 else "on ne sait pas (< 2 écarts-types)"
    return z, txt


def compare(a, b):
    ver = check_version([a, b])
    sa, sb = scores(a), scores(b)
    if not sa or not sb:
        refus("pas de résultat par graine (per_seed / per_pair) : impossible de vérifier les graines.")
    ka, kb = set(sa), set(sb)
    if ka == kb:
        ks = sorted(ka)
        d, se = mean_se([sa[k] - sb[k] for k in ks])
        kind = f"apparié sur {len(ks)} graines communes"
    elif not ka & kb:
        (ma, ea), (mb, eb) = mean_se(list(sa.values())), mean_se(list(sb.values()))
        d, se = ma - mb, math.sqrt(ea ** 2 + eb ** 2)
        kind = f"indépendant ({len(ka)} et {len(kb)} graines disjointes)"
    else:
        refus(f"graines en partie communes ({len(ka & kb)} sur {len(ka)} et {len(kb)}) : ni apparié ni indépendant.")
    z, txt = verdict(d, se)
    print(f"moteur {ver} · {kind}")
    print(f"A = {a['_nom']}\nB = {b['_nom']}")
    print(f"A - B = {100 * d:+.1f} ± {100 * se:.1f} points (z = {z:.1f}) : {txt}")
    return d, se


def pool(es):
    ver = check_version(es)
    seen = {}
    for e in es:
        for k in scores(e):
            if k in seen:
                refus(f"la graine {k} est dans {seen[k]} et {e['_nom']} : des blocs sur les mêmes graines ne sont pas indépendants.")
            seen[k] = e["_nom"]
    xs = [v for e in es for v in scores(e).values()]
    m, se = mean_se(xs)
    print(f"moteur {ver} · {len(es)} blocs, {len(xs)} graines distinctes")
    print(f"regroupé = {100 * m:.1f} % ± {100 * se:.1f}")
    return m, se


if __name__ == "__main__":
    args = sys.argv[1:]
    if args[:1] == ["--pool"]:
        pool([load(x) for x in args[1:]])
    elif len(args) == 2:
        compare(load(args[0]), load(args[1]))
    else:
        print(__doc__)
        sys.exit(2)
