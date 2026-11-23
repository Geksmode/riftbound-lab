"""Pilote des puzzles pour les tests : joue une ligne (suite d'étapes) pour le joueur contre la vraie IA de la table.

Une étape : dict(on=..., pick=...) :
  on : "main" | "prio" (priorité ou focus, chaîne ou showdown) | "ask:<kind>" ;
  when(g) : condition facultative (ex. Punch First est sur la chaîne) ;
  pick(g, a) : vrai pour l'action brute (ou l'option de la question) à prendre.
Hors étape : on passe (priorité, focus), on termine son tour (main, quand toutes les étapes sont jouées) et on répond
la première option aux questions. Renvoie (ok, journal)."""
import json


def nm(g, u):
    o = g.obj(u)
    return o.name if o is not None else None


def run(pid, steps, level=1, limit=400):
    import train
    train.puzzle_new(pid, level)
    steps = list(steps)
    v = json.loads(train.step())
    log = []
    for _ in range(limit):
        g = train.W["g"]
        if v.get("winner") is not None:
            return v["puz"]["ok"], log, steps
        if v.get("busy"):
            v = json.loads(train.step())
            continue
        if v.get("ask"):
            kind = v["ask"]["kind"]
            e = train.W["retry"][1]
            k = 0
            if steps and steps[0]["on"] == "ask:" + kind and steps[0].get("when", lambda g: True)(g):
                k = next(i for i, o in enumerate(e.options) if steps[0]["pick"](g, o))
                steps.pop(0)
            log.append(("ask", kind, v["ask"]["options"][k]))
            v = json.loads(train.answer(json.dumps([] if kind == "mulligan" else k)))
            continue
        d = train.W["d"]
        on = "main" if d.kind == "main" else "prio"
        i = None
        if steps and steps[0]["on"] == on and steps[0].get("when", lambda g: True)(g):
            m = [j for j, a in enumerate(d.options) if steps[0]["pick"](g, a)]
            if m:
                i = m[0]
                steps.pop(0)
        if i is None:
            if on == "main" and steps and steps[0]["on"] == "main":
                raise AssertionError("étape introuvable : " + repr([train.describe(g, a, 0) for a in d.options]))
            want = "end" if on == "main" else "pass"
            i = next(j for j, a in enumerate(d.options) if a[0] == want)
        log.append((on, train.describe(g, d.options[i], 0)))
        v = json.loads(train.act(i))
    raise AssertionError("puzzle sans fin")


def play(name, dest=None, tg=None, src=None):
    """Action « jouer la carte name » (vers dest pour une unité ; tg = noms des cibles)."""
    def f(g, a):
        if a[0] != "play":
            return False
        from replay import _find
        o = _find(g, 0, a[1])
        if o is None or o.name != name or (src is not None and a[2] != src):
            return False
        ch = a[3] if len(a) > 3 and isinstance(a[3], dict) else {}
        if dest is not None and ch.get("loc", ch.get("dest")) != dest:
            return False
        if tg is not None and sorted(nm(g, u) for u in ch.get("tg", ())) != sorted(tg):
            return False
        return True
    return f


def move(names, dest):
    return lambda g, a: a[0] == "move" and a[2] == dest and sorted(nm(g, u) for u in a[1]) == sorted(names)
