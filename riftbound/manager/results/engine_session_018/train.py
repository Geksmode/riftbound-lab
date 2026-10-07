#!/usr/bin/env python3
"""Partie humain contre IA, pilotée pas à pas (page d'entraînement riftbound/train/, moteur chargé dans Pyodide).

L'humain joue Akali (G2 de Gorica, joueur 0) contre l'IA LeBlanc IQ#5 (plan Hook tempo, IA tempo, joueur 1).
Le moteur demande certains choix en pleine résolution (g.ask, mulligan) : l'agent humain lève NeedChoice, l'état est
restauré depuis la copie prise avant l'opération, la page pose la question, puis l'opération est rejouée avec la
réponse (le moteur est déterministe : mêmes compteurs, mêmes générateurs aléatoires).

API (renvoie du JSON) : new(seed, bf, first, level) ; step() ; act(i) ; answer(x) ; hint().
  python3 train.py  -> partie de test où l'humain est remplacé par des choix au hasard."""
import copy, json, random, itertools
import actions as _act
import game as _game
from game import Game, Obj, Item
from decks import load, edit, with_bf, DECKS
import plans as P
import replay as _rp
from replay import fr_line, describe, snap, short, _find

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
                s.buf.append([1, f"Tu pioches {new[0].cname}" if pid == ME else f"{_rp.NAMES[AI]} pioche une carte"])

    def clone(s):
        b, s.buf = s.buf, []
        try:
            return super().clone()
        finally:
            s.buf = b


W = {}


PRESETS = {"akali-g2": ("Akali G2 (Gorica)", G2), "leblanc-iq5": ("LeBlanc IQ#5 (Gyatarina)", L)}


def _plan_for(deck):
    """Plan de jeu connu pour la légende du deck, sinon l'IA générique (aucun plan)."""
    lg = deck.get("legend") or ""
    if lg.startswith("Akali"):
        return P.PLANS["Gorica"]()
    if lg.startswith("LeBlanc"):
        return P.PLANS["Hook tempo"]()
    return P.Plan()


def _short(deck):
    return (deck.get("legend") or "?").split(",")[0]


def _deck(x, default):
    """Deck donné par la page : clé de PRESETS, clé de decks.json, ou dict {legend, champion, main, runes, battlefields}."""
    if x in (None, ""):
        return dict(default)
    if isinstance(x, str) and not x.lstrip().startswith("{"):
        if x in PRESETS:
            return dict(PRESETS[x][1])
        return load(x)
    d = json.loads(x) if isinstance(x, str) else dict(x)
    err = [e for e in validate(d) if not e.startswith("⚠")]
    if err:
        raise ValueError("Deck invalide : " + " ; ".join(err))
    return dict(legend=d["legend"], champion=d["champion"], main=list(d["main"]),
                runes=[r.replace(" Rune", "") for r in d["runes"]], battlefields=list(d["battlefields"]),
                battlefield=d["battlefields"][0], sideboard=[], key=d.get("name") or "perso")


def new(seed=None, bf=None, first=None, level=1, mine=None, opp=None):
    seed = random.randrange(10 ** 6) if seed is None else int(seed)
    Obj._n = 0
    Item._n = 0
    r = random.Random(seed * 7919)
    f = r.randrange(2) if first is None else int(first)
    MD, OD = _deck(mine, G2), _deck(opp, L)
    pa, pl = _plan_for(MD), _plan_for(OD)
    lbf = pl.battlefield(OD, f == 1) or r.choice(OD["battlefields"])
    abf = bf if bf in MD["battlefields"] else (pa.battlefield(MD, f == 0, lbf) or r.choice(MD["battlefields"]))
    A, B = with_bf(MD, abf), with_bf(OD, lbf)
    names = [_short(A), _short(B)]
    if names[0] == names[1]:
        names = [names[0], names[1] + " (IA)"]
    _rp.NAMES = tuple(names)
    hu = Human()
    ai = P.PlanAgent(seed + 500000, plan=pl, opp_plan=pa, samples=int(level))
    coach = P.PlanAgent(seed + 900000, plan=_plan_for(MD), opp_plan=_plan_for(OD))
    g = TGame([A, B], [hu, ai], seed=seed, first=f)
    W.clear()
    W.update(g=g, ag=[hu, ai], coach=coach, d=None, retry=None, seed=seed, decks=[A, B], last_ai=None, names=names)
    return json.dumps(dict(seed=seed, first=f, bf=[abf, lbf], battlefields=MD["battlefields"], names=names,
                           decks=[dict(legend=x["legend"], champion=x.get("champion"), main=sorted(set(x["main"])))
                                  for x in (A, B)]))


# ------------------------------------------------------------------ éditeur de deck
def _spec_rows():
    import csv
    return list(csv.DictReader(open(_game.ROOT / "cards" / "cards_unique.csv", encoding="utf-8")))


def catalog():
    """Toutes les cartes du jeu pour l'éditeur : m=1 si la carte est modélisée (jouable)."""
    import cards as _cards
    out = []
    for x in _spec_rows():
        if x["supertype"] == "Token":
            continue
        n = x["name"]
        out.append(dict(id=x["id"], n=n, t=x["type"], s=x["supertype"], d=[d for d in x["domain"].split("|") if d],
                        e=x["energy_cost"], p=x["power_cost"], mi=x["might"], tg=[t for t in x["tags"].split("|") if t],
                        k=x["keywords"], tx=x["text"], set=x["set_code"], r=x["rarity"], ban=x["banned_standard"],
                        m=1 if (n in _cards.IMPL or x["type"] == "Rune") else 0))
    pre = [dict(key=k, name=v[0], deck=_export(v[1])) for k, v in PRESETS.items()]
    for k in sorted(DECKS):
        d = load(k)
        pre.append(dict(key=k, name=k.replace("_", " · "), deck=_export(d)))
    return json.dumps(dict(cards=out, presets=pre), ensure_ascii=False)


def _export(d):
    return dict(legend=d["legend"], champion=d.get("champion"), main=list(d["main"]),
                runes=[r if r.endswith(" Rune") else r + " Rune" for r in d["runes"]], battlefields=list(d["battlefields"]))


def validate(d):
    """Règles de construction (103) et cartes non modélisées. Renvoie la liste des problèmes (vide = jouable)."""
    import cards as _cards
    from collections import Counter
    S = _game.SPEC
    err = []
    lg, ch = d.get("legend"), d.get("champion")
    main, runes, bfs = list(d.get("main") or []), list(d.get("runes") or []), list(d.get("battlefields") or [])
    if not lg or S.get(lg, {}).get("type") != "Legend":
        err.append("choisis une légende")
        ident = set()
    else:
        ident = set(S[lg]["domains"])
        ltags = S[lg]["tags"]
    if not ch or S.get(ch, {}).get("super") != "Champion" or S[ch]["type"] != "Unit":
        err.append("choisis un champion (unité Champion)")
    elif lg and S.get(lg, {}).get("type") == "Legend" and not (S[ch]["tags"] & ltags):
        err.append(f"{ch} n'a pas le tag de la légende ({', '.join(sorted(ltags))})")
    if len(main) + (1 if ch else 0) < 40:
        err.append(f"deck principal : {len(main) + (1 if ch else 0)}/40 cartes (champion compris)")
    cnt = Counter(main + ([ch] if ch else []))
    for n, q in sorted(cnt.items()):
        if q > 3:
            err.append(f"{n} : {q} exemplaires (3 au plus)")
        if "Unique" in (S.get(n, {}).get("text") or "") and q > 1:
            err.append(f"{n} est Unique : 1 exemplaire au plus")
    sig = sum(q for n, q in cnt.items() if S.get(n, {}).get("super") == "Signature")
    if sig > 3:
        err.append(f"{sig} cartes Signature (3 au plus)")
    if lg and ident:
        for n in sorted(cnt):
            if n not in S:
                err.append(f"carte inconnue : {n}")
                continue
            if S[n]["type"] in ("Rune", "Battlefield", "Legend"):
                err.append(f"{n} ne va pas dans le deck principal")
            if not set(S[n]["domains"]) <= ident:
                err.append(f"{n} est hors des domaines de la légende")
            if S[n]["super"] == "Signature" and not (S[n]["tags"] & ltags):
                err.append(f"{n} est une Signature d'un autre champion")
    if len(runes) != 12:
        err.append(f"runes : {len(runes)}/12")
    for r_ in sorted(set(runes)):
        dom = r_.replace(" Rune", "")
        if ident and dom not in ident:
            err.append(f"{r_} hors des domaines de la légende")
    if len(bfs) != 3 or len(set(bfs)) != 3:
        err.append(f"battlefields : {len(set(bfs))}/3 différents")
    for b in bfs:
        if S.get(b, {}).get("type") != "Battlefield":
            err.append(f"{b} n'est pas un battlefield")
    todo = sorted(n for n in set(cnt) | set(bfs) | ({lg} if lg else set()) if n in S and n not in _cards.IMPL)
    if todo:
        err.append("pas encore modélisées : " + ", ".join(todo))
    banned = sorted(n for n in set(cnt) | set(bfs) | ({lg} if lg else set()) if n in S and _BAN.get(n))
    if banned:
        err.append("⚠ bannies en Standard : " + ", ".join(banned))
    return err


_BAN = {}
try:
    _BAN = {x["name"]: x["banned_standard"] for x in _spec_rows() if x["banned_standard"]}
except Exception:
    pass


def check(x):
    return json.dumps(validate(json.loads(x) if isinstance(x, str) else x), ensure_ascii=False)


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


class _It:
    """Objet chaîne factice pour évaluer les prédicats de cible au moment du choix."""
    def __init__(s, pid, hb):
        s.ctrl, s.data = pid, {"hidden_bf": hb}


def _expand(g, d):
    """Sorts à cibles : le moteur ne propose que quelques combinaisons (ennemis les mieux classés, 8 au plus) pour
    l'IA. Pour le joueur humain, on ajoute toutes les combinaisons légales (prédicats de la carte, Deflect payé)."""
    opts, seen, groups = list(d.options), {repr(a) for a in d.options}, {}
    for a in opts:
        if a[0] == "play" and "tg" in a[3] and set(a[3]) <= {"tg", "flow"}:
            groups.setdefault((a[1], a[2]), []).append(a)
    for (uid, src), lst in groups.items():
        n = len(lst[0][3]["tg"])
        if n == 0 or any(len(a[3]["tg"]) != n or not all(isinstance(x, int) for x in a[3]["tg"]) for a in lst):
            continue
        card = _find(g, ME, uid)
        im = g.impl(card) if card is not None else None
        if im is None or not im.preds:
            continue
        pr = [im.preds[min(i, len(im.preds) - 1)] for i in range(n)]
        it = _It(ME, card.hidden_bf if src == "facedown" else None)
        slots = [[o for o in g.board if g.targetable(o, ME) and p(g, it, o)] for p in pr]
        rep = any(len(set(a[3]["tg"])) < n for a in lst)
        if all(p is pr[0] for p in pr):
            combos = (itertools.combinations_with_replacement if rep else itertools.combinations)(slots[0], n)
        else:
            combos = (c for c in itertools.product(*slots) if rep or len(set(c)) == n)
        extra = {k: v for k, v in lst[0][3].items() if k != "tg"}
        for c in combos:
            ch = dict(extra, tg=tuple(o.uid for o in c))
            a = ("play", uid, src, ch)
            if repr(a) in seen or not _act.affordable(g, ME, card, ch, src):
                continue
            seen.add(repr(a))
            opts.append(a)
    # Sorts « cible puis déplacement » (Shuriken Flip : 2 dégâts à un ennemi au plus, puis un allié va vers une zone) :
    # le moteur ne garde que 12 combinaisons pour l'IA ; l'humain a droit à toutes (ennemi ou aucun × allié × destination).
    mgroups = {}
    for a in d.options:
        if a[0] == "play" and isinstance(a[3], dict) and "mover" in a[3]:
            mgroups.setdefault((a[1], a[2]), []).append(a)
    from cards import enemies, friends
    for (uid, src), lst in mgroups.items():
        card = _find(g, ME, uid)
        im = g.impl(card) if card is not None else None
        if im is None:
            continue
        hb = card.hidden_bf if src == "facedown" else None
        it = _It(ME, hb)
        ens = [e for e in enemies(g, ME, False, hb) if not im.preds or im.preds[0](g, it, e)]
        fr = friends(g, ME)
        if hb is not None:
            fr = [u for u in fr if u.loc == hb] or fr
        extra = {k: v for k, v in lst[0][3].items() if k not in ("tg", "mover", "mover_oid", "dest")}
        for t in [(e.uid,) for e in ens] + [()]:
            for u in fr:
                for dst in ("base", 0, 1):
                    if dst == u.loc:
                        continue
                    ch = dict(extra, tg=t, mover=u.uid, mover_oid=u.oid, dest=dst)
                    a = ("play", uid, src, ch)
                    if repr(a) in seen or not _act.affordable(g, ME, card, ch, src):
                        continue
                    seen.add(repr(a))
                    opts.append(a)
    d.options = opts


def _tick():
    g, d = W["g"], W["d"]
    if d is None:
        W["d"] = g.advance()
        if W["d"] is not None and W["d"].player == ME:
            _expand(g, W["d"])
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
    names |= {n for n, sp in _game.SPEC.items() if sp.get("super") == "Token"} | {"Mech", "Reflection", "Gold"}
    return json.dumps(card_db(n for n in names if n), ensure_ascii=False)


def hint(n=4):
    """Ce que l'IA (plan de ta légende) jouerait ici : les meilleures options et l'écart de score."""
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
        where = "" if loc is None else " (" + (g.bfs[loc].name if loc in (0, 1) else "base") + ")" if hasattr(o, "uid") and g.obj(o.uid) is not None else ""
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
            mixologist_pick="Quelle unité ?", ashe_pick="Quelle carte ?", target="Cible :",
            damage_order="Ordre des dégâts :")


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


DOM_FR = {"Fury": "Fury", "Calm": "Calm", "Mind": "Mind", "Body": "Body", "Chaos": "Chaos", "Order": "Order"}


def _cost_txt(e, reqs):
    from game import ANY
    parts = [f"{e} énergie"] if e else []
    cnt = {}
    for r in reqs:
        k = "rune au choix" if r == ANY else " ou ".join(sorted(r))
        cnt[k] = cnt.get(k, 0) + 1
    parts += [f"{n} {k}" for k, n in cnt.items()]
    return " + ".join(parts) or "gratuit"


def _act_label(g, a):
    """Nom et coût d'une capacité activée : « Empower Akali, Deadly Weapon (2 énergie + 1 Fury) »."""
    src, i, ch = a[1], a[2], (a[3] if len(a) > 3 and isinstance(a[3], dict) else {})
    if isinstance(src, tuple):
        obj, name = g.p[src[1]].legend, g.p[src[1]].legend_name
    else:
        obj = g.obj(src)
        name = obj.cname if obj is not None else "?"
    im = g.impl(name if isinstance(src, tuple) else obj)
    try:
        ab = im.abilities[i]
        e, reqs = ab["cost"](g, ME, obj, ch)
        extra = ""
        if ch.get("tg"):
            extra = ", cible " + " et ".join(short(g.obj(u) or u) for u in ch["tg"])
        return f"{ab['name']} : {name} ({_cost_txt(e, reqs)}{', épuise' if ab.get('exhaust') else ''}{extra})"
    except Exception:
        return describe(g, a, ME)


def _ask_uid(g, o):
    """Objet d'un choix cliquable sur la table : unité ou équipement en jeu, ou carte de ta main."""
    if not hasattr(o, "uid") or isinstance(o, bool):
        return None
    # La question vient d'une copie de l'état (rejouée ensuite) : on compare par uid, pas par identité.
    if g.obj(o.uid) is not None or any(c.uid == o.uid for c in g.p[ME].hand):
        return o.uid
    return None


def _ask_src(g, ask):
    it = ask.ctx.get("item")
    src = getattr(it, "src", None)
    if isinstance(src, int) and g.obj(src) is not None:
        return src
    c = ask.ctx.get("card")
    return getattr(c, "uid", None)


def _ask_title(g, ask):
    """Titre de la question : connu (ASKS), sinon le nom de la carte qui demande."""
    if ask.kind in ASKS:
        return ASKS[ask.kind]
    it = ask.ctx.get("item") if isinstance(ask.ctx, dict) else None
    nm = getattr(it, "name", "") or ""
    if not nm:
        u = _ask_src(g, ask)
        o = g.obj(u) if isinstance(u, int) else None
        nm = o.cname if o is not None else ""
    return f"{short(nm)} : fais ton choix" if nm else "Fais ton choix"


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
        out["ask"] = dict(kind=ask.kind, title=_ask_title(g, ask),
                          options=[_olabel(g, o) for o in ask.options],
                          uids=[_ask_uid(g, o) for o in ask.options],
                          zs=[("base" if o == "base" else str(o)) if (o == "base" or (isinstance(o, int) and not isinstance(o, bool) and o in (0, 1))) else None
                              for o in ask.options],
                          src=_ask_src(g, ask),
                          item=short(getattr(ask.ctx.get("item"), "name", "") or "") or None)
    elif g.winner is not None:
        pass
    elif d is not None and d.player == ME:
        # Le moteur ne donne des options qu'à un exemplaire par nom de carte (les copies sont identiques) :
        # les autres exemplaires en main renvoient vers celui-là.
        have = {}
        for a in d.options:
            if a[0] in ("play", "hide"):
                c = _find(g, ME, a[1])
                if c is not None:
                    have.setdefault((c.cname, a[2] if a[0] == "play" else "hand"), a[1])
        out["alias"] = {str(c.uid): have[(c.cname, "hand")] for c in g.p[ME].hand
                        if (c.cname, "hand") in have and have[(c.cname, "hand")] != c.uid}
        out["dec"] = dict(kind=d.kind, options=[dict(i=i, k=a[0], src=_src(g, a), us=list(a[1]) if a[0] == "move" else None,
                                                     label=_act_label(g, a) if a[0] == "act" else describe(g, a, ME), **_info(a))
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
    print("gagnant", _rp.NAMES[v["winner"]] if v["winner"] in (0, 1) else v["winner"], v["st"]["pts"],
          f"{time.time() - t0:.1f}s, {n_me} décisions humaines, {n_ask} questions, {n_ai} pas")
