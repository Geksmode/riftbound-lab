#!/usr/bin/env python3
"""Partie humain contre IA, pilotée pas à pas (page d'entraînement riftbound/train/, moteur chargé dans Pyodide).

L'humain joue Akali (G2 de Gorica, joueur 0) contre l'IA LeBlanc IQ#5 (plan Hook tempo, IA tempo, joueur 1).
Le moteur demande certains choix en pleine résolution (g.ask, mulligan) : l'agent humain lève NeedChoice, l'état est
restauré depuis la copie prise avant l'opération, la page pose la question, puis l'opération est rejouée avec la
réponse (le moteur est déterministe : mêmes compteurs, mêmes générateurs aléatoires).

API (renvoie du JSON) : new(seed, bf, first, level) ; step() ; act(i) ; answer(x) ; hint().
  python3 train.py  -> partie de test où l'humain est remplacé par des choix au hasard."""
import copy, json, random
import game as _game
from game import Game, Obj, Item
from decks import load, edit, with_bf
import plans as P
from replay import fr_line, describe, snap, short

NAMES = ("Akali", "LeBlanc")
G = load("akali_gorica_rq-singapore")
GL = edit(G, out=[("Long Sword", 2)], add=[("Akali, Silent", 1), ("Ferrous Forerunner", 1)])
G2 = edit(GL, out=[("Defy", 3)], add=[("Ferrous Forerunner", 1), ("Lonely Poro", 1), ("Scuttle Crab", 1)])
L = load("leblanc_gyatarina_ccs-iq5_1st")
ME, AI = 0, 1


class NeedChoice(Exception):
    def __init__(s, kind, options, ctx):
        s.kind, s.options, s.ctx = kind, options, ctx


class Human:
    """Agent du joueur : ne décide rien lui-même, lit les réponses déjà données ou lève NeedChoice."""

    def __init__(s):
        s.answers, s.k = [], 0

    def start(s, g):
        pass

    def _next(s, kind, options, ctx):
        if s.k < len(s.answers):
            s.k += 1
            return s.answers[s.k - 1]
        raise NeedChoice(kind, options, ctx)

    def mulligan(s, g, pid):
        out = s._next("mulligan", list(g.p[pid].hand), {})
        return [g.p[pid].hand[i] for i in out]

    def choose(s, g, pid, kind, options, ctx):
        return options[s._next(kind, options, ctx)]


class TGame(Game):
    """Journal en français ; les pioches de l'humain sont notées, celles de l'IA restent cachées."""

    def __init__(s, *a, **k):
        s.buf = []
        k["log"] = True
        super().__init__(*a, **k)

    def log(s, *a):
        if not s.logging:
            return
        raw = " ".join(str(x) for x in a)
        s.buf.append([1 if raw.startswith("  ") else 0, fr_line(s, raw)])

    def draw(s, pid, n=1):
        for _ in range(n):
            before = list(s.p[pid].hand)
            super().draw(pid, 1)
            if s.winner is not None:
                return
            new = [c for c in s.p[pid].hand if c not in before]
            if new and s.logging and s.stage != "setup":
                s.buf.append([1, f"Tu pioches {new[0].cname}" if pid == ME else "LeBlanc pioche une carte"])

    def clone(s):
        b, s.buf = s.buf, []
        try:
            return super().clone()
        finally:
            s.buf = b


W = {}


def new(seed=None, bf=None, first=None, level=1):
    seed = random.randrange(10 ** 6) if seed is None else int(seed)
    Obj._n = 0
    Item._n = 0
    r = random.Random(seed * 7919)
    f = r.randrange(2) if first is None else int(first)
    pa, pl = P.PLANS["Gorica"](), P.PLANS["Hook tempo"]()
    lbf = pl.battlefield(L, f == 1) or r.choice(L["battlefields"])
    abf = bf or pa.battlefield(G2, f == 0, lbf) or r.choice(G2["battlefields"])
    A, B = with_bf(G2, abf), with_bf(L, lbf)
    hu = Human()
    ai = P.PlanAgent(seed + 500000, plan=pl, opp_plan=pa, samples=int(level))
    coach = P.PlanAgent(seed + 900000, plan=P.PLANS["Gorica"](), opp_plan=P.PLANS["Hook tempo"]())
    g = TGame([A, B], [hu, ai], seed=seed, first=f)
    W.clear()
    W.update(g=g, ag=[hu, ai], coach=coach, d=None, retry=None, seed=seed, decks=[A, B], last_ai=None)
    return json.dumps(dict(seed=seed, first=f, bf=[abf, lbf], battlefields=G2["battlefields"],
                           decks=[dict(legend=x["legend"], champion=x.get("champion"), main=sorted(set(x["main"])))
                                  for x in (A, B)]))


def _save():
    return copy.deepcopy(dict(g=W["g"], ag=W["ag"], d=W["d"])), Obj._n, Item._n


def _restore(sv):
    st, Obj._n, Item._n = copy.deepcopy(sv[0]), sv[1], sv[2]
    W.update(g=st["g"], ag=st["ag"], d=st["d"])
    W["g"].agents = W["ag"]


def _run(op):
    """Exécute op ; si l'humain doit répondre à une question en cours de résolution, tout est annulé jusqu'à la réponse."""
    sv = _save()
    hu = W["ag"][ME]
    hu.k = 0
    W["g"].buf = []
    try:
        op()
    except NeedChoice as e:
        _restore(sv)
        W["ag"][ME].answers = hu.answers
        W["retry"] = (op, e)
        W["g"].buf = []
        return _view(ask=e)
    W["ag"][ME].answers = []
    W["retry"] = None
    return _view()


def _tick():
    g, d = W["g"], W["d"]
    if d is None:
        W["d"] = g.advance()
        return
    if d.player == ME and len(d.options) == 1:       # rien à choisir (seulement passer) : on passe pour l'humain
        g.apply(d.options[0])
        W["d"] = None
        return
    if d.player == AI:
        a = W["ag"][AI].decide(g, d)
        W["last_ai"] = describe(g, a, AI) if a[0] not in ("pass",) else None
        W["last_ai_info"] = dict(_info(a), k=a[0], src=_src(g, a)) if a[0] not in ("pass", "end") else None
        g.apply(a)
        W["d"] = None


def step():
    """Avance d'un cran (une décision de l'IA au plus). La page rappelle step() tant que busy est vrai."""
    return _run(_tick)


def act(i):
    d = W["d"]
    a = d.options[int(i)]
    h = W.setdefault("hist", [])
    h.append(_save())
    del h[:-40]

    def op():
        W["g"].apply(a)
        W["d"] = None
    return _run(op)


def answer(x):
    op, e = W["retry"]
    W["ag"][ME].answers.append(json.loads(x) if isinstance(x, str) else x)
    return _run(op)


def undo():
    """Reprend le dernier coup du joueur (état juste avant ce coup, y compris les réponses de l'IA qui ont suivi)."""
    if W.get("hist"):
        _restore(W["hist"].pop())
        W["ag"][ME].answers = []
        W["retry"] = None
    W["g"].buf = []
    return _view()


def cards():
    from replay import card_db
    names = set()
    for d in W["decks"]:
        names |= set(d["main"]) | {d["legend"], d.get("champion"), d["battlefield"]} | set(d.get("battlefields", []))
    names |= set(G2["battlefields"]) | set(L["battlefields"]) | {"Mech", "Reflection", "Gold"}
    return json.dumps(card_db(n for n in names if n), ensure_ascii=False)


def hint(n=4):
    """Ce que l'IA Akali (plan Gorica) jouerait ici : les meilleures options et l'écart de score."""
    g, d = W["g"], W["d"]
    if d is None or d.player != ME or len(d.options) < 2:
        return json.dumps([])
    sv = _save()
    try:
        c = W["coach"]
        opts = list(d.options)
        if len(opts) > c.max_cands:
            opts = opts[:1] + c.rng.sample(opts[1:], c.max_cands - 1)
        best, best_v, scored = c.pick(g, d, opts)
    finally:
        _restore(sv)
    scored.sort(key=lambda x: -x[0])
    idx = {repr(a): i for i, a in enumerate(W["d"].options)}
    out = []
    for v, a in scored[:n]:
        gap = v - best_v
        out.append(dict(i=idx.get(repr(a)), label=describe(W["g"], a, ME),
                        gap=("perd la partie" if gap < -1000 else round(gap, 1))))
    return json.dumps(out, ensure_ascii=False)


def _olabel(g, o):
    if o is None:
        return "aucun"
    if o is True:
        return "oui"
    if o is False:
        return "non"
    if hasattr(o, "cname"):
        loc = getattr(o, "loc", None)
        where = "" if loc is None else " (" + (g.bfs[loc].name if loc in (0, 1) else "base") + ")" if o in g.board else ""
        mine = "" if getattr(o, "ctrl", ME) == ME else " adverse"
        return f"{o.cname}{mine}{where}"
    if hasattr(o, "domain"):
        return f"rune {o.domain}"
    if o in (0, 1) and not isinstance(o, bool):
        return g.bfs[o].name
    if o == "base":
        return "la base"
    return short(o)


ASKS = dict(mulligan="Mulligan : choisis jusqu'à 2 cartes à remettre", may="Utiliser cet effet optionnel ?",
            play_location="Où poser l'unité ?", equip_target="Équiper quelle unité ?", discard="Défausser quelle carte ?",
            copy_target="Copier quelle unité ?", sacrifice="Sacrifier quelle unité ?", recycle_rune="Recycler quelle rune ?",
            hook_pick="Baited Hook : quelle unité prendre ?", zhonya_save="Zhonya : sauver quelle unité ?",
            star_spring="Star Spring : quelle unité ?", dusk_kill="Détruire quelle unité ?",
            deathgrip_victim="Quelle unité sacrifier ?", herald_pick="Quelle unité ?", herald_dk_pick="Quelle carte ?",
            mixologist_pick="Quelle unité ?", ashe_pick="Quelle carte ?")


def _src(g, a):
    k = a[0]
    if k in ("play", "hide"):
        return a[1]
    if k == "act":
        return "legend" if isinstance(a[1], tuple) else a[1]
    if k == "move":
        return a[1][0]
    return None


def _info(a):
    """Données structurées d'une action pour le glisser-déposer et les flèches : cibles, lieu, unité déplacée."""
    k = a[0]
    out = {}
    if k == "play":
        ch = dict(a[3])
        out = dict(t1=list(ch.get("tg", ()) or ()), t2=list(ch.get("tg2", ()) or ()),
                   loc=ch.get("loc", ch.get("dest")), mover=ch.get("mover"), acc=bool(ch.get("acc")),
                   rep=bool(ch.get("rep")), from_=a[2])
    elif k == "act":
        ch = dict(a[3]) if len(a) > 3 and isinstance(a[3], dict) else {}
        out = dict(t1=list(ch.get("tg", ()) or ()), loc=ch.get("dest"))
    elif k == "hide":
        out = dict(loc=a[2])
    elif k == "move":
        out = dict(loc=a[2])
    return out


def _view(ask=None):
    g = W["g"]
    st = snap(g)
    opp = st["p"][AI]
    opp["hand"] = [[0, "?"] for _ in opp["hand"]]
    for b in st["bfs"]:
        if b["fd"] is not None and b["fd"][0] == AI:
            b["fd"] = [AI, "?"]
    out = dict(st=st, log=g.buf, winner=g.winner, ai=W.pop("last_ai", None), aii=W.pop("last_ai_info", None))
    W["last_ai"] = None
    W["last_ai_info"] = None
    for c, it in zip(st["chain"], g.chain):
        c["tg"] = list((it.data or {}).get("tg", ()) or ()) + list((it.data or {}).get("tg2", ()) or ())
        c["src"] = it.src if isinstance(it.src, int) else None
    st["p"][ME]["champu"] = [c.uid for c in g.p[ME].champ]
    for b, gb in zip(st["bfs"], g.bfs):
        if gb.facedown is not None and gb.facedown.owner == ME:
            b["fdu"] = gb.facedown.uid
    out["undo"] = len(W.get("hist", []))
    d = W["d"]
    if ask is not None:
        out["ask"] = dict(kind=ask.kind, title=ASKS.get(ask.kind, ask.kind),
                          options=[_olabel(g, o) for o in ask.options],
                          item=short(getattr(ask.ctx.get("item"), "name", "") or "") or None)
    elif g.winner is not None:
        pass
    elif d is not None and d.player == ME:
        out["dec"] = dict(kind=d.kind, options=[dict(i=i, k=a[0], src=_src(g, a), us=list(a[1]) if a[0] == "move" else None,
                                                     label=describe(g, a, ME), **_info(a))
                                                for i, a in enumerate(d.options)])
    else:
        out["busy"] = True
    return json.dumps(out, ensure_ascii=False)


if __name__ == "__main__":
    import sys, time
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 1
    rr = random.Random(seed)
    print(new(seed))
    v = json.loads(step())
    t0, n_ai, n_me, n_ask = time.time(), 0, 0, 0
    while v.get("winner") is None:
        for ind, t in v["log"]:
            if not ind and t.startswith("Tour"):
                print(t)
        if v.get("ask"):
            n_ask += 1
            k = v["ask"]["kind"]
            x = [] if k == "mulligan" else rr.randrange(len(v["ask"]["options"]))
            v = json.loads(answer(json.dumps(x)))
        elif v.get("dec"):
            n_me += 1
            if n_me % 15 == 1:
                h = json.loads(hint())
                print("  conseil :", h[:2])
            ops = v["dec"]["options"]
            v = json.loads(act(rr.randrange(len(ops))))
        else:
            t1 = time.time()
            v = json.loads(step())
            n_ai += 1
    print("gagnant", NAMES[v["winner"]] if v["winner"] in (0, 1) else v["winner"], v["st"]["pts"],
          f"{time.time() - t0:.1f}s, {n_me} décisions humaines, {n_ask} questions, {n_ai} pas")
