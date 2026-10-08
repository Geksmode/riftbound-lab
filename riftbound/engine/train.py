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
# Duel entre deux joueurs (deux navigateurs reliés, voir duel_new) : les deux places sont humaines ; chaque navigateur
# applique la même suite d'entrées (act / answer, dans l'ordre) et ne montre que les décisions de sa place (ME).
DUEL = False


class NeedChoice(Exception):
    def __init__(s, kind, options, ctx, pid=None):
        s.kind, s.options, s.ctx, s.pid = kind, options, ctx, pid


class Human:
    """Agent du joueur : ne décide rien lui-même, lit les réponses déjà données ou lève NeedChoice.
    every_choice : le moteur lui propose tous les choix légaux (actions.full_choices), sans les plafonds de l'IA."""
    every_choice = True

    def __init__(s):
        s.answers, s.k = [], 0

    def start(s, g):
        pass

    def _next(s, kind, options, ctx, pid=None):
        if s.k < len(s.answers):
            s.k += 1
            return s.answers[s.k - 1]
        raise NeedChoice(kind, options, ctx, pid)

    def mulligan(s, g, pid):
        out = s._next("mulligan", list(g.p[pid].hand), {}, pid)
        return [g.p[pid].hand[i] for i in out]

    def choose(s, g, pid, kind, options, ctx):
        return options[s._next(kind, options, ctx, pid)]


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
                battlefield=d["battlefields"][0], sideboard=list(d.get("sideboard") or []), key=d.get("name") or "perso")


def new(seed=None, bf=None, first=None, level=1, mine=None, opp=None, obf=None):
    duel_off()
    seed = random.randrange(10 ** 6) if seed is None else int(seed)
    Obj._n = 0
    Item._n = 0
    r = random.Random(seed * 7919)
    f = r.randrange(2) if first is None else int(first)
    MD, OD = _deck(mine, G2), _deck(opp, L)
    pa, pl = _plan_for(MD), _plan_for(OD)
    lbf = obf if obf in OD["battlefields"] else (pl.battlefield(OD, f == 1) or r.choice(OD["battlefields"]))
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


# ------------------------------------------------------------------ duel entre deux joueurs
def duel_new(seed, bf0, bf1, first, deck0, deck1, me, names=None):
    """Partie à deux humains : place 0 = hôte, place 1 = invité. Les deux navigateurs appellent duel_new avec les MÊMES
    arguments sauf `me` (leur place), puis appliquent la même suite d'entrées act / answer. Aucune IA."""
    global ME, AI, DUEL
    ME, AI, DUEL = int(me), 1 - int(me), True
    seed = int(seed)
    Obj._n = 0
    Item._n = 0
    D = [_deck(deck0, G2), _deck(deck1, L)]
    A, B = with_bf(D[0], bf0 if bf0 in D[0]["battlefields"] else D[0]["battlefields"][0]), \
        with_bf(D[1], bf1 if bf1 in D[1]["battlefields"] else D[1]["battlefields"][0])
    nm = list(names) if names else [_short(A), _short(B)]
    if nm[0] == nm[1]:
        nm = [nm[0], nm[1] + " (2)"]
    _rp.NAMES = tuple(nm)
    ag = [Human(), Human()]
    coach = P.PlanAgent(seed + 900000, plan=_plan_for([A, B][ME]), opp_plan=_plan_for([A, B][AI]))
    g = TGame([A, B], ag, seed=seed, first=int(first))
    W.clear()
    W.update(g=g, ag=ag, coach=coach, d=None, retry=None, seed=seed, decks=[A, B], last_ai=None, names=nm)
    from version import engine_md5
    return json.dumps(dict(seed=seed, first=int(first), bf=[A["battlefield"], B["battlefield"]], names=nm, me=ME,
                           engine=engine_md5(), decks=[dict(legend=x["legend"], champion=x.get("champion"),
                                                            main=sorted(set(x["main"]))) for x in (A, B)]))


def duel_off():
    """Revenir aux parties contre l'IA (la place locale redevient 0)."""
    global ME, AI, DUEL
    ME, AI, DUEL = 0, 1, False
    return "{}"


# ------------------------------------------------------------------ match BO1 / BO3 (voir match.py)
def match_new(mode, seed=None, mine=None, opp=None):
    """Nouveau match : état JSON à garder par la page et à repasser aux fonctions suivantes."""
    import match as _m
    seed = random.randrange(10 ** 6) if seed in (None, "") else int(seed)
    MD, OD = _deck(mine, G2), _deck(opp, L)
    st = _m.new(mode, seed, [MD["battlefields"], OD["battlefields"]])
    return json.dumps(st)


def match_next(state, mine=None, opp=None, human_bf=None):
    """Prépare la manche suivante : qui choisit le premier joueur, choix de l'IA, battlefields permis, sideboard permis.
    Le battlefield de l'IA est choisi sans connaître celui du joueur ; la page ne l'affiche qu'au lancement."""
    import match as _m
    st = json.loads(state)
    MD, OD = _deck(mine, G2), _deck(opp, L)
    ch = _m.chooser(st)
    first = _m.forced_first(st)
    if first is None and ch == _m.AI:
        first = _m.ai_first(st)
    pl = _plan_for(OD)
    bfs = _m.pick_bfs(st, lambda ok: (lambda b: b if b in ok else None)(pl.battlefield(dict(OD, battlefields=ok), first == 1)),
                      human_bf)
    return json.dumps(dict(game=_m.game_no(st), chooser=ch, first=first, roll=st["roll"] if _m.game_no(st) == 1 else None,
                           allowed=_m.allowed_bfs(st, 0), ai_bf=bfs[1], human_bf=bfs[0] if st["mode"] == "bo1" else None,
                           sideboard=_m.can_sideboard(st), seed=_m.seed_of(st), wins=st["wins"], over=_m.over(st)))


def match_duel_next(state):
    """Duel entre deux joueurs : préparation de la manche sans aucun choix d'IA (les deux places sont humaines) :
    qui choisit le premier joueur (place 0 = hôte, 1 = invité, None = imposé après une nulle), battlefields permis
    pour chaque place (choix simultanés, ou tirés au hasard en BO1), sideboard permis, graine de la manche."""
    import match as _m
    st = json.loads(state)
    bo1 = _m.pick_bfs(st) if st["mode"] == "bo1" else None
    return json.dumps(dict(game=_m.game_no(st), chooser=_m.chooser(st), first=_m.forced_first(st),
                           roll=st["roll"] if _m.game_no(st) == 1 else None,
                           allowed=[_m.allowed_bfs(st, 0), _m.allowed_bfs(st, 1)], bo1_bfs=bo1,
                           sideboard=_m.can_sideboard(st), seed=_m.seed_of(st), wins=st["wins"], over=_m.over(st)))


def match_record(state, first, bfs, win):
    import match as _m
    st = _m.record(json.loads(state), int(first), json.loads(bfs) if isinstance(bfs, str) else bfs, int(win))
    return json.dumps(dict(st, over=_m.over(st), winner=_m.winner(st)))


def match_swap(deck, out_cards, in_cards, champion=None):
    """Sideboarding entre deux manches (403.4, 601.1.c.4) ; renvoie le deck modifié, validé."""
    import match as _m
    d = _m.sideboard_swap(json.loads(deck), json.loads(out_cards), json.loads(in_cards), champion or None)
    err = [e for e in validate(d) if not e.startswith("⚠")]
    if err:
        raise ValueError("Deck invalide après sideboard : " + " ; ".join(err))
    return json.dumps(d)


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
                runes=[r if r.endswith(" Rune") else r + " Rune" for r in d["runes"]], battlefields=list(d["battlefields"]),
                sideboard=list(d.get("sideboard") or []))


def validate(d):
    """Règles de construction (103) et cartes non modélisées. Renvoie la liste des problèmes (vide = jouable)."""
    import cards as _cards
    from collections import Counter
    S = _game.SPEC
    err = []
    lg, ch = d.get("legend"), d.get("champion")
    main, runes, bfs = list(d.get("main") or []), list(d.get("runes") or []), list(d.get("battlefields") or [])
    side = list(d.get("sideboard") or [])
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
    # sideboard (règles de tournoi 601.1.c) : 10 cartes au plus, seulement des cartes valides pour le Main Deck, limites
    # d'exemplaires sur Main Deck + sideboard
    if len(side) > 10:
        err.append(f"sideboard : {len(side)}/10 cartes au plus")
    tot = cnt + Counter(side)
    for n, q in sorted(tot.items()):
        if q > 3:
            err.append(f"{n} : {q} exemplaires (3 au plus, sideboard compris)" if n in side else f"{n} : {q} exemplaires (3 au plus)")
        if "Unique" in (S.get(n, {}).get("text") or "") and q > 1:
            err.append(f"{n} est Unique : 1 exemplaire au plus")
    sig = sum(q for n, q in cnt.items() if S.get(n, {}).get("super") == "Signature")
    if sig > 3:
        err.append(f"{sig} cartes Signature (3 au plus)")
    if lg and ident:
        for n in sorted(set(cnt) | set(side)):
            if n not in S:
                err.append(f"carte inconnue : {n}")
                continue
            if S[n]["type"] in ("Rune", "Battlefield", "Legend"):
                err.append(f"{n} ne va pas dans le deck principal" + (" ni dans le sideboard" if n in side else ""))
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
    todo = sorted(n for n in set(cnt) | set(side) | set(bfs) | ({lg} if lg else set()) if n in S and n not in _cards.IMPL)
    if todo:
        err.append("pas encore modélisées : " + ", ".join(todo))
    banned = sorted(n for n in set(cnt) | set(side) | set(bfs) | ({lg} if lg else set()) if n in S and _BAN.get(n))
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
    hus = {i: a for i, a in enumerate(W["ag"]) if isinstance(a, Human)}
    for a in hus.values():
        a.k = 0
    W["g"].buf = []
    try:
        op()
    except NeedChoice as e:
        _restore(sv)
        for i, a in hus.items():                       # les réponses déjà données restent pour la reprise de op
            W["ag"][i].answers = a.answers
        W["retry"] = (op, e)
        W["g"].buf = []
        return _view(ask=e)
    for i in hus:
        W["ag"][i].answers = []
    W["retry"] = None
    return _view()


class _It:
    """Objet chaîne factice pour évaluer les prédicats de cible au moment du choix."""
    def __init__(s, pid, hb):
        s.ctrl, s.data = pid, {"hidden_bf": hb}
        s.targets, s.chosen, s.src, s.name = [], set(), None, ""


def _expand(g, d):
    """Sorts à cibles : le moteur ne propose que quelques combinaisons (ennemis les mieux classés, 8 au plus) pour
    l'IA. Pour le joueur humain, on ajoute toutes les combinaisons légales (prédicats de la carte, Deflect payé)."""
    pid = d.player                                    # le joueur qui décide (en duel : pas forcément ME)
    opts, seen, groups = list(d.options), {repr(a) for a in d.options}, {}
    for a in opts:
        if a[0] == "play" and "tg" in a[3] and set(a[3]) <= {"tg", "flow"}:
            groups.setdefault((a[1], a[2]), []).append(a)
    for (uid, src), lst in groups.items():
        n = len(lst[0][3]["tg"])
        if n == 0 or any(len(a[3]["tg"]) != n or not all(isinstance(x, int) for x in a[3]["tg"]) for a in lst):
            continue
        card = _find(g, pid, uid)
        im = g.impl(card) if card is not None else None
        if im is None or not im.preds:
            continue
        pr = [im.preds[min(i, len(im.preds) - 1)] for i in range(n)]
        it = _It(pid, card.hidden_bf if src == "facedown" else None)
        rep = any(len(set(a[3]["tg"])) < n for a in lst)
        if all(p is pr[0] for p in pr):
            slot = [o for o in g.board if g.targetable(o, pid) and pr[0](g, it, o)]
            combos = (itertools.combinations_with_replacement if rep else itertools.combinations)(slot, n)
        else:
            # prédicats dépendants (la 2e cible selon la 1re) : it.targets se remplit au fil des cibles
            def _walk(i, pre):
                if i == n:
                    yield tuple(pre)
                    return
                it.targets = [(o.uid, o.oid) for o in pre]
                it.chosen = {o.uid for o in pre}
                for o in [o for o in g.board if g.targetable(o, pid) and pr[i](g, it, o)]:
                    if rep or o not in pre:
                        yield from _walk(i + 1, pre + [o])
            combos = list(_walk(0, []))
        extra = {k: v for k, v in lst[0][3].items() if k != "tg"}
        for c in combos:
            ch = dict(extra, tg=tuple(o.uid for o in c))
            a = ("play", uid, src, ch)
            if repr(a) in seen or not _act.affordable(g, pid, card, ch, src):
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
        card = _find(g, pid, uid)
        im = g.impl(card) if card is not None else None
        if im is None:
            continue
        hb = card.hidden_bf if src == "facedown" else None
        it = _It(pid, hb)
        ens = [e for e in enemies(g, pid, False, hb) if not im.preds or im.preds[0](g, it, e)]
        fr = friends(g, pid)
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
                    if repr(a) in seen or not _act.affordable(g, pid, card, ch, src):
                        continue
                    seen.add(repr(a))
                    opts.append(a)
    d.options = opts


def _tick():
    g, d = W["g"], W["d"]
    if d is None:
        W["d"] = g.advance()
        if W["d"] is not None and (W["d"].player == ME or DUEL):   # en duel : mêmes options chez les deux joueurs
            _expand(g, W["d"])
        return
    if (d.player == ME or DUEL) and len(d.options) == 1:   # rien à choisir (seulement passer) : on passe pour l'humain
        g.apply(d.options[0])
        W["d"] = None
        return
    if d.player == AI and not DUEL:
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
    if DUEL and d.player != ME:                         # coup de l'adversaire reçu : affiché comme les coups de l'IA
        W["last_ai"] = describe(W["g"], a, d.player) if a[0] not in ("pass",) else None
        W["last_ai_info"] = dict(_info(a), k=a[0], src=_src(W["g"], a)) if a[0] not in ("pass", "end") else None
    h = W.setdefault("hist", [])
    if not DUEL:                                        # pas de « Reprendre » en duel : l'adversaire a déjà vu le coup
        h.append(_save())
        del h[:-40]

    def op():
        W["g"].apply(a)
        W["d"] = None
    return _run(op)


def answer(x):
    op, e = W["retry"]
    W["ag"][e.pid if e.pid is not None else ME].answers.append(json.loads(x) if isinstance(x, str) else x)
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
    if hasattr(o, "label") and hasattr(o, "value"):       # game.Opt
        return str(o.label)
    if isinstance(o, int) and not isinstance(o, bool) and o in (0, 1):
        return g.bfs[o].name
    if o == "base":
        return "la base"
    if isinstance(o, int) and not isinstance(o, bool):
        u = g.obj(o)
        return _olabel(g, u) if u is not None else str(o)
    if isinstance(o, dict):
        try:
            from cards import _choice_label
            return _choice_label(g, o)
        except Exception:
            return ", ".join(f"{k} {_olabel(g, v)}" for k, v in sorted(o.items()))
    if isinstance(o, (tuple, list)):
        return " + ".join(_olabel(g, x) for x in o) if o else "aucun"
    if isinstance(o, str):
        return WORDS.get(o, o)
    return short(o)


# Options textuelles des questions (modes, positions) en français.
WORDS = dict(top="sur le dessus du deck", bottom="sous le deck", draw="piocher", discard="défausser",
             cards="des cartes", runes="des runes", buff="renforcer (buff)", ready="préparer", exhaust="épuiser",
             channel="canaliser 1 rune")


def _alabel(g, ask, o):
    """Libellé d'une option selon la question : joueurs, X, répartitions de dégâts, paires (carte, choix de jeu)."""
    k = ask.kind
    try:
        if k == "damage_pick":
            return f"{_olabel(g, o)} (mortel : {ask.ctx.get('need', {}).get(o.uid, '?')} dégâts)"
        if k in ("burn_player", "choose_player") and isinstance(o, int):
            return "toi" if o == ME else "l'adversaire"
        if k == "bullet_time_x":
            return f"X = {o}"
        if k == "ava_play":
            c, ch = ask.ctx["plays"][o]
            return f"{c.cname} ({_olabel(g, ch)})"
        if k == "entourage":
            q, a = o
            return f"{WORDS.get(a, a)} {'ta légende' if q == ME else 'la légende adverse'}"
        if k == "split_damage":
            if not o:
                return "aucune cible"
            if all(isinstance(x, int) for x in o):
                return " + ".join(str(x) for x in o) + " dégâts"
            return ", ".join(f"{n} à {_olabel(g, u)}" for u, n in o)
        if k in ("mf_ready", "ready_pick") and isinstance(o, tuple):
            return _olabel(g, o[1])
        if k == "bard_move":
            bf, grp = o
            return f"vers {_olabel(g, bf)} : " + (" + ".join(_olabel(g, u) for u in grp) or "aucune unité")
        if k in ("move_enemy", "profiteer") and isinstance(o, tuple):
            return f"{_olabel(g, o[0])} → {_olabel(g, o[1])}"
        if isinstance(o, tuple) and len(o) == 2 and hasattr(o[0], "cname") and isinstance(o[1], dict):
            return f"{o[0].cname} ({_olabel(g, o[1])})"
    except Exception:
        pass
    return _olabel(g, o)


ASKS = dict(mulligan="Mulligan : choisis jusqu'à 2 cartes à remettre", may="Utiliser cet effet optionnel ?",
            play_location="Où poser l'unité ?", equip_target="Équiper quelle unité ?", discard="Défausser quelle carte ?",
            copy_target="Copier quelle unité ?", sacrifice="Sacrifier quelle unité ?", recycle_rune="Recycler quelle rune ?",
            hook_pick="Baited Hook : quelle unité prendre ?", zhonya_save="Zhonya : sauver quelle unité ?",
            star_spring="Star Spring : quelle unité ?", dusk_kill="Détruire quelle unité ?",
            herald_pick="Quelle unité ?", herald_dk_pick="Quelle carte ?",
            mixologist_pick="Quelle unité ?", ashe_pick="Quelle carte ?", target="Cible :",
            damage_order="Ordre des dégâts :",
            damage_pick="Combat : quelle unité reçoit d'abord ses dégâts mortels ?")


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
    """Titre de la question : connu (ASKS, puis cards.ASK_TEXT que chaque module remplit), précédé du nom de la carte
    qui demande quand il n'y figure pas déjà."""
    from cards import ASK_TEXT
    if ask.kind == "damage_pick":
        return (f"Combat : il te reste {ask.ctx.get('left')} dégâts à assigner. Quelle unité reçoit d'abord ses dégâts "
                "mortels ? (règle 465.2.c : mortel en entier avant la suivante, l'excédent va à la dernière)")
    t = ASKS.get(ask.kind) or ASK_TEXT.get(ask.kind)
    it = ask.ctx.get("item") if isinstance(ask.ctx, dict) else None
    nm = getattr(it, "name", "") or ""
    if not nm:
        u = _ask_src(g, ask)
        o = g.obj(u) if isinstance(u, int) else None
        nm = o.cname if o is not None else ""
    if t is None:
        t = "Choisis une option :"
    if nm and short(nm) not in t and ask.kind not in ("mulligan",):
        return f"{short(nm)} — {t}"
    return t


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
    if ask is not None and DUEL and ask.pid is not None and ask.pid != ME:
        out["wait"] = "adversaire"                      # question posée à l'adversaire : on attend sa réponse
    elif ask is not None:
        out["ask"] = dict(kind=ask.kind, title=_ask_title(g, ask),
                          options=[_alabel(g, ask, o) for o in ask.options],
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
    elif DUEL and d is not None and d.player != ME:
        out["wait"] = "adversaire"                      # décision de l'adversaire : on attend son coup
    else:
        out["busy"] = True
    out["me"] = ME
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
