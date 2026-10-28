#!/usr/bin/env python3
"""Banc d'erreurs de cible et de timing de l'IA (2026-10-28).

    python3 banc/banc.py gen <parties> <graine0> <defaut|akali|miroir> [version] [processus]
    python3 banc/banc.py bilan <positions.jsonl> [...]
    python3 banc/banc.py rejuge <parties.jsonl> [processus]
    python3 banc/banc.py rejoue <positions.jsonl> <version> [processus] [play|ask]

Parties IA contre IA comme exp_search.play (même setup, mêmes graines d'agents), version « turn » par défaut (IA Normal
de la table : PlanAgent, search="turn", 1 tirage). À chaque choix de l'IA, un juge rejoue les autres choix possibles
sur une copie de la position et les note :
  - « play » : le coup joué est une carte ; autres choix = toutes les variantes de la MÊME carte (autres cibles, coûts
    en plus, Repeat), y compris celles que l'IA ne voit pas (every_choice : plafonds MAX_CHOICES / cards.cap levés) ;
  - « ask » : choix pendant la résolution (Game.ask : cible d'une capacité, défausse, « you may »…), chaque option
    jouée à sa place (les choix d'avant rejoués à l'identique, ceux d'après par Heuristics.choose).
Note d'un choix = ai.evaluate pour le joueur qui choisit, après la résolution : les deux joueurs passent jusqu'à la
prochaine décision « main » (chaîne résolue, combat du showdown compris). Deux notes : poids Texel (ai.EV) et anciens
poids (ai.EV_OLD) ; regret = écart à la meilleure option quand LES DEUX poids la préfèrent (regret2). Avec EV seul, toute
Accelerate ou tout paiement par rune plutôt que par Gold passait pour une erreur (rune 6,6 : poids corrélés, ia-texel). Le juge voit toute la position (pas de monde caché) et ne regarde pas plus loin que la résolution :
il trouve les erreurs « à coup sûr » selon la propre évaluation de l'IA (mauvais côté, mauvaise cible, dégâts
perdus), pas les erreurs de long terme.
Sorts : effet = note après résolution − note si le sort était contré sans effet (mêmes coûts payés, mêmes déclencheurs
« quand tu joues un sort »). Effet ≤ 0,25 pour les deux poids = « sans effet visible » : candidat au mauvais timing, pas
erreur sûre (le juge ne compare pas avec « garder la carte ») ; si une unité ciblée, ou une unité vers le battlefield
d'une cible, attaque plus tard dans le tour : « avant attaque », mis à part (dégâts et Might « ce tour » servent au
combat d'après, que la note immédiate ne voit pas).
Sorties : banc/data/pos-<graine0>.jsonl (une position jugée par ligne) et banc/data/parties-<graine0>.jsonl (script
complet de chaque partie : réponses à chaque décision et à chaque ask, pour rejouer les positions). Résumé :
banc/results_banc.json (version.stamp). Le juge ne change pas la partie (compteurs d'objets restaurés) : une partie
jugée est identique coup pour coup à la même partie sans juge (test_banc.py)."""
import json, math, os, sys, time, traceback
from multiprocessing import Pool
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
_argv, sys.argv = sys.argv, sys.argv[:1]      # exp.py (importé par exp_search) lit sys.argv
import exp_search as E
sys.argv = _argv
import ai
from ai import Heuristics, PolicyAgent, evaluate
from game import Game, Obj, Item
from version import engine_md5, stamp

REGRET = 2.0          # erreur nette : ≈ un tiers de point de victoire (EV["pts"] ≈ 6,9), plus qu'une unité 1/1 (unit0 + might)
NOEFFECT = 0.25
MAX_ALTS = 40
_BAD = object()
_POL = PolicyAgent()


# ---------------------------------------------------------------------- réponses aux asks : codage pour les rejouer
def enc(ans, options):
    for i, o in enumerate(options):
        if o is ans:
            return ["i", i]
    for i, o in enumerate(options):
        try:
            if type(o) is type(ans) and o == ans:
                return ["i", i]
        except Exception:
            pass
    if isinstance(ans, (list, tuple)) and all(any(x is o for o in options) for x in ans):
        return ["s", [next(i for i, o in enumerate(options) if o is x) for x in ans], isinstance(ans, tuple)]
    if ans is None or isinstance(ans, (bool, int, float, str)):
        return ["v", ans]
    return None


def dec(e, options):
    if e is None:
        return _BAD
    if e[0] == "i":
        return options[e[1]] if e[1] < len(options) else _BAD
    if e[0] == "s":
        if any(i >= len(options) for i in e[1]):
            return _BAD
        out = [options[i] for i in e[1]]
        return tuple(out) if e[2] else out
    return e[1]


def label(g, o, pid):
    if hasattr(o, "cname"):
        side = "" if getattr(o, "ctrl", getattr(o, "owner", pid)) == pid else " (adv.)"
        return f"{o.cname}{side}"
    return repr(o)[:60]


class Rec:
    """Agent réel enveloppé : note chaque ask (joueur, type, options, réponse) dans `log`."""

    def __init__(s, ag, log):
        s.ag, s.log = ag, log

    def __getattr__(s, k):
        return getattr(s.ag, k)

    def decide(s, g, d):
        return s.ag.decide(g, d)

    def choose(s, g, pid, kind, options, ctx):
        r = s.ag.choose(g, pid, kind, options, ctx)
        it = ctx.get("item")
        s.log.append(dict(pid=pid, kind=kind, n=len(options), enc=enc(r, options), src=getattr(it, "name", None),
                          labels=[label(g, o, pid) for o in options[:MAX_ALTS]]))
        return r


class Forced(Heuristics):
    """Juge : asks d'avant le j-ième rejoués comme dans la partie, le j-ième forcé sur `alt`, la suite par Heuristics.
    `seen` garde (type, nombre d'options) du j-ième pour vérifier que le rejeu suit la partie."""

    def __init__(s, seg, j=-1, alt=None):
        s.seg, s.j, s.alt, s.k, s.seen = seg, j, alt, 0, None

    def choose(s, g, pid, kind, options, ctx):
        k, s.k = s.k, s.k + 1
        if k == s.j:
            s.seen = (kind, len(options))
            return options[s.alt]
        if k < len(s.seg):
            r = dec(s.seg[k]["enc"], options)
            if r is not _BAD:
                return r
        return Heuristics.choose(s, g, pid, kind, options, ctx)


# ---------------------------------------------------------------------- juge
def settle(c):
    """Les deux joueurs passent jusqu'à la prochaine décision « main » (ou la fin de la partie)."""
    for _ in range(400):
        dd = c.advance()
        if dd is None or dd.kind == "main":
            return
        c.apply(("pass",) if ("pass",) in dd.options else dd.options[0])


def note(c, pid):
    """(note avec ai.EV, note avec ai.EV_OLD) : deux jeux de poids. Les poids Texel (EV) sont corrélés entre eux et ne
    se lisent pas un par un (rune 6,6, on_bf −4,6 : une Accelerate ou un paiement par rune plutôt que par Gold coûte
    « 6,6 » sans que le bénéfice ne se voie tout de suite) ; une erreur doit l'être pour les deux (regret2)."""
    if c.winner is not None:
        v = 1000.0 if c.winner == pid else -1000.0
        return (v, v)
    return (evaluate(c, pid), evaluate(c, pid, ai.EV_OLD))


def regret2(vals, i):
    """Regret de l'option i : (accord des deux poids, EV seul, EV_OLD seul). Accord = la meilleure option qui bat i
    à la fois avec EV et avec EV_OLD, de l'écart le plus petit des deux."""
    both = max(min(v[0] - vals[i][0], v[1] - vals[i][1]) for v in vals)
    return (round(max(0.0, both), 3), round(max(v[0] for v in vals) - vals[i][0], 3),
            round(max(v[1] for v in vals) - vals[i][1], 3))


def replay(pre, cnt, a, pid, agent, fizzle=False, ckpt=False):
    Obj._n, Item._n = cnt
    c = pre.clone()
    c.agents = [agent, agent]
    c.checkpoint = ckpt                       # point de reprise pour une version avec ai.ask_sim (rescore)
    c.apply(a)
    if fizzle:
        it = next((x for x in c.chain if x.kind == "spell" and x.card is not None and x.card.uid == a[1]), None)
        if it is None:
            return None
        c.chain.remove(it)
        c.card_leaves_chain(it.card, "trash", it.ctrl)
    settle(c)
    return note(c, pid)


def full_options(pre, d):
    c = pre.clone()
    c.every_choice = True
    if d.kind == "main":
        return c.options_main(d.player)
    if d.kind == "focus":
        return c.options_focus(d.player)
    return c.options_priority(d.player)


def play_alts(pre, d, a):
    """Variantes jugées du coup « play » a (a en tête), comme judge_play : (carte, variantes, variantes vues par l'IA),
    ou None si a n'est pas une carte jouée."""
    if a[0] != "play":
        return None
    card = _POL.card_of(pre, a)
    if card is None:
        return None
    try:
        alts = [o for o in full_options(pre, d) if o[0] == "play" and o[1] == a[1] and o[2] == a[2]]
    except Exception:
        alts = [o for o in d.options if o[0] == "play" and o[1] == a[1] and o[2] == a[2]]
    # même emplacement (base ou battlefield) que le coup de l'IA : où poser une unité est un choix de long terme que la
    # note immédiate ne sait pas juger (EV["on_bf"] < 0 à côté d'unit0 et might : poids corrélés, ia-texel)
    where = lambda o: (dict(o[3]).get("loc"), dict(o[3]).get("dest"))
    alts = [o for o in alts if where(o) == where(a)]
    seen = [o for o in d.options if o[0] == "play" and o[1] == a[1] and o[2] == a[2] and where(o) == where(a)]
    if a not in alts:
        alts.append(a)
    return card, [a] + [o for o in alts if o != a][:MAX_ALTS - 1], seen


def judge_play(pre, cnt, d, a):
    pa = play_alts(pre, d, a)
    if pa is None:
        return None
    card, alts, seen = pa
    spell = card.spec["type"] == "Spell"
    if len(alts) < 2 and not spell:
        return None
    from replay import describe
    pid = d.player
    vals = [replay(pre, cnt, o, pid, Forced([])) for o in alts]
    r = dict(typ="play", card=card.cname, ctype=card.spec["type"], phase=d.kind, pid=pid,
             alts=[[describe(pre, o, pid), round(v[0], 3), round(v[1], 3), o in seen] for o, v in zip(alts, vals)],
             n_seen=len(seen), n_all=len(alts))
    r["regret"], r["regret_ev"], r["regret_old"] = regret2(vals, 0)
    if spell:
        fz = replay(pre, cnt, a, pid, Forced([]), fizzle=True)
        if fz is not None:
            # sans effet seulement si sans effet pour les deux poids
            r["effect_ev"], r["effect_old"] = round(vals[0][0] - fz[0], 3), round(vals[0][1] - fz[1], 3)
            r["effect"] = max(r["effect_ev"], r["effect_old"])
        r["tg"] = [u for u in (dict(a[3]).get("tg", ()) or ()) if isinstance(u, int)]
        r["tg_loc"] = sorted({o.loc for o in (pre.obj(u) for u in r["tg"]) if o is not None and isinstance(o.loc, int)})
    return r


def judge_asks(pre, cnt, a, seg):
    out = []
    for j, s in enumerate(seg):
        if s["n"] < 2 or s["enc"] is None or s["enc"][0] != "i":
            continue
        vals, ok = [], True
        for alt in range(min(s["n"], MAX_ALTS)):
            f = Forced(seg, j, alt)
            v = replay(pre, cnt, a, s["pid"], f)
            if f.seen != (s["kind"], s["n"]):
                ok = False
                break
            vals.append(v)
        if not ok:
            continue
        ch = s["enc"][1]
        if ch >= len(vals):
            continue
        rg = regret2(vals, ch)
        out.append(dict(typ="ask", kind=s["kind"], src=s.get("src"), pid=s["pid"], j=j, chosen=ch,
                        alts=[[lb, round(v[0], 3), round(v[1], 3)] for lb, v in zip(s["labels"], vals)],
                        regret=rg[0], regret_ev=rg[1], regret_old=rg[2], n_all=s["n"]))
    return out


# ---------------------------------------------------------------------- une partie
def play(seed, mode, ver, judge=True, script0=None):
    """Une partie jouée par `ver` (ou rejouée d'après script0, sans recherche : pour rejuger une partie déjà jouée)."""
    Obj._n = 0
    Item._n = 0
    decks, plans, f = E.setup(seed, mode)
    ags = E.agents(seed, decks, plans, ver, ver)
    if script0 is not None:
        ptr = [0]
        ags = [Script(x, script0, ptr) for x in ags]
    asks = []
    rec = [Rec(x, asks) for x in ags]
    script = []                       # ["d", indice, type, n] ou ["a", codage, type, n], dans l'ordre de la partie
    pos, moved = [], set()
    g = Game(decks, rec, seed=seed, first=f)
    pend = None

    def flush(pend, a_end):
        pre, cnt, d, a, a0, step, turn = pend
        seg = asks[a0:a_end]
        if not judge:
            return
        now = (Obj._n, Item._n)
        try:
            items = []
            jp = judge_play(pre, cnt, d, a)
            if jp:
                items.append(jp)
            items += judge_asks(pre, cnt, a, seg)
        finally:
            Obj._n, Item._n = now
        for it in items:
            it.update(seed=seed, step=step, turn=turn)
            pos.append(it)

    def add_asks(k0):
        for s in asks[k0:]:
            script.append(["a", s["enc"], s["kind"], s["n"]])

    k_done = 0
    while True:
        d = g.advance()
        add_asks(k_done)
        k_done = len(asks)
        if pend is not None:
            flush(pend, len(asks))
            pend = None
        if d is None:
            break
        cnt = (Obj._n, Item._n)
        pre = g.clone() if judge else None
        a = rec[d.player].decide(g, d)
        idx = d.options.index(a)
        script.append(["d", idx, d.kind, len(d.options)])
        if a[0] == "move" and a[2] != "base":
            moved.update((g.turn_no, u) for u in a[1])
            moved.add((g.turn_no, "bf", a[2]))
        pend = (pre, cnt, d, a, len(asks), len(script) - 1, g.turn_no)
        g.apply(a)
        add_asks(k_done)
        k_done = len(asks)
    for p in pos:
        # une unité ciblée, ou une unité de n'importe qui vers le battlefield d'une cible, attaque plus tard dans le tour :
        # dégâts, Might « ce tour » et malus servent au combat d'après, que la note immédiate ne voit pas
        if "effect" in p and p["effect"] <= NOEFFECT and (any((p["turn"], u) in moved for u in p["tg"]) or
                                                          any((p["turn"], "bf", l) in moved for l in p.get("tg_loc", ()))):
            p["before_attack"] = True
    return dict(seed=seed, mode=mode, ver=ver, legends=[x["legend"] for x in decks], first=f, winner=g.winner,
                turns=g.turn_no, script=script), pos


def job(arg):
    seed, mode, ver = arg[:3]
    t0 = time.time()
    try:
        gm, pos = play(seed, mode, ver, script0=arg[3] if len(arg) > 3 else None)
    except Exception:
        return dict(seed=seed, err=traceback.format_exc()[-1200:]), []
    gm["sec"] = round(time.time() - t0, 1)
    return gm, pos


# ---------------------------------------------------------------------- rejouer le banc avec une autre version
class Desync(Exception):
    pass


class Script:
    """Rejoue une partie enregistrée : décisions et asks dans l'ordre du script ; mulligan par l'agent d'origine."""

    def __init__(s, ag, script, ptr):
        s.ag, s.script, s.ptr = ag, script, ptr

    def __getattr__(s, k):
        return getattr(s.ag, k)

    def _next(s, typ, kind, n):
        i = s.ptr[0]
        if i >= len(s.script) or s.script[i][0] != typ or s.script[i][2] != kind or s.script[i][3] != n:
            raise Desync(f"pas {i} : attendu {s.script[i] if i < len(s.script) else None}, vu {typ} {kind} {n}")
        s.ptr[0] += 1
        return s.script[i]

    def decide(s, g, d):
        e = s._next("d", d.kind, len(d.options))
        return d.options[e[1]]

    def choose(s, g, pid, kind, options, ctx):
        e = s._next("a", kind, len(options))
        r = dec(e[1], options)
        return s.ag.choose(g, pid, kind, options, ctx) if r is _BAD else r


class Ask(Forced):
    """Le j-ième ask est posé à l'agent `cand` (sa réponse dans s.got), les autres comme Forced."""

    def __init__(s, seg, j, cand):
        super().__init__(seg)
        s.jj, s.cand, s.got = j, cand, None

    def choose(s, g, pid, kind, options, ctx):
        if s.k == s.jj:
            s.k += 1
            r = s.cand.choose(g, pid, kind, options, ctx)
            s.got = next((i for i, o in enumerate(options) if o is r), None)
            return r
        return super().choose(g, pid, kind, options, ctx)


def rescore(game, pos, ver, only=None):
    """Positions `pos` (d'une même partie) rejouées : choix de la version `ver` à la même position, noté par le juge.
    Retourne une liste de dicts (typ, regret de ver, regret d'origine, même coup, autre coup)."""
    seed, mode = game["seed"], game["mode"]
    Obj._n = 0
    Item._n = 0
    decks, plans, f = E.setup(seed, mode)
    orig = E.agents(seed, decks, plans, game["ver"], game["ver"])
    d2, p2, _ = E.setup(seed, mode)
    cand = E.agents(seed, d2, p2, ver, ver)
    ptr = [0]
    sa = [Script(x, game["script"], ptr) for x in orig]
    g = Game(decks, sa, seed=seed, first=f)
    want = {}
    for p in pos:
        if only is None or p["typ"] == only:
            want.setdefault(p["step"], []).append(p)
    out = []
    while want:
        d = g.advance()
        if d is None:
            break
        step = ptr[0]
        ps = want.pop(step, None)
        if ps:
            cnt = (Obj._n, Item._n)
            pre = g.clone()
            a0 = d.options[game["script"][step][1]]
            # asks du segment d'origine (réponses enregistrées), pour Forced
            seg, k = [], step + 1
            while k < len(game["script"]) and game["script"][k][0] == "a":
                e = game["script"][k]
                seg.append(dict(enc=e[1], kind=e[2], n=e[3]))
                k += 1
            for p in ps:
                now = (Obj._n, Item._n)
                try:
                    if p["typ"] == "play":
                        a = cand[d.player].decide(pre.clone(), d)
                        Obj._n, Item._n = cnt
                        same = a[0] == "play" and a[1] == a0[1] and a[2] == a0[2]
                        r = judge_play(pre, cnt, d, a) if same else None
                        out.append(dict(typ="play", card=p["card"], old=p["regret"], new=r["regret"] if r else None,
                                        same_card=same, changed=a != a0, old_eff=p.get("effect"),
                                        new_eff=r.get("effect") if r else None, before_attack=p.get("before_attack"),
                                        seed=p["seed"], step=p["step"]))
                    else:
                        f_ = Ask(seg, p["j"], cand[p["pid"]])
                        replay(pre, cnt, a0, p["pid"], f_, ckpt=True)
                        vals = [(x[1], x[2]) for x in p["alts"]]
                        new = None if f_.got is None or f_.got >= len(vals) else regret2(vals, f_.got)[0]
                        out.append(dict(typ="ask", kind=p["kind"], old=p["regret"], new=new,
                                        changed=f_.got != p["chosen"], seed=p["seed"], step=p["step"], j=p["j"]))
                finally:
                    Obj._n, Item._n = now
        g.apply(sa[d.player].decide(g, d))
    return out


def rjob(arg):
    game, pos, ver, only = arg
    try:
        return rescore(game, pos, ver, only)
    except Exception:
        return [dict(err=traceback.format_exc()[-800:], seed=game["seed"])]


# ---------------------------------------------------------------------- bilan
def is_error(p):
    return p["regret"] >= REGRET


def ratio_se(per_game):
    """Taux (erreurs / positions) et écart-type par grappes (les positions d'une même partie ne sont pas indépendantes)."""
    e = [x for x, _ in per_game]
    m = [y for _, y in per_game]
    M = sum(m)
    if not M:
        return 0.0, 0.0
    r = sum(e) / M
    k = len(per_game)
    if k < 2:
        return r, 0.0
    v = k / (k - 1) * sum((ei - r * mi) ** 2 for ei, mi in per_game) / M ** 2
    return r, math.sqrt(v)


def bilan(pos, games):
    seeds = sorted({g["seed"] for g in games})
    out = dict(games=len(seeds), positions=len(pos))
    for name, sel in (("play", lambda p: p["typ"] == "play" and p["n_all"] > 1),
                      ("ask", lambda p: p["typ"] == "ask"),
                      ("tout", lambda p: p["typ"] == "ask" or p["n_all"] > 1)):
        ps = [p for p in pos if sel(p)]
        pg = [(sum(1 for p in ps if p["seed"] == s and is_error(p)), sum(1 for p in ps if p["seed"] == s)) for s in seeds]
        r, se = ratio_se(pg)
        out[name] = dict(n=len(ps), errors=sum(1 for p in ps if is_error(p)), rate=round(r, 4), se=round(se, 4))
    # cible hors de la liste de l'IA : la meilleure variante n'était pas proposée à l'IA (plafonds)
    pl = [p for p in pos if p["typ"] == "play" and p["n_all"] > 1 and is_error(p)]
    out["play_best_unseen"] = sum(1 for p in pl if not max(p["alts"], key=lambda x: min(x[1], x[2]))[3])
    sp = [p for p in pos if p["typ"] == "play" and "effect" in p]
    noe = [p for p in sp if p["effect"] <= NOEFFECT]
    pg = [(sum(1 for p in noe if p["seed"] == s and not p.get("before_attack")), sum(1 for p in sp if p["seed"] == s))
          for s in seeds]
    r, se = ratio_se(pg)
    out["spells"] = dict(n=len(sp), no_effect=len(noe), before_attack=sum(1 for p in noe if p.get("before_attack")),
                         rate_no_effect=round(r, 4), se=round(se, 4))
    by = {}
    for p in pos:
        if is_error(p):
            k = p["card"] if p["typ"] == "play" else "ask:" + p["kind"]
            by[k] = by.get(k, 0) + 1
    out["errors_by"] = sorted(by.items(), key=lambda x: -x[1])[:25]
    by = {}
    for p in noe:
        if not p.get("before_attack"):
            by[p["card"]] = by.get(p["card"], 0) + 1
    out["no_effect_by"] = sorted(by.items(), key=lambda x: -x[1])[:25]
    return out


def rsummary(res):
    """Mêmes positions : erreurs d'origine contre erreurs de la version rejouée (appariées par position)."""
    out = {}
    for typ in ("play", "ask"):
        rs = [x for x in res if x["typ"] == typ]
        sc = [x for x in rs if x["new"] is not None]
        out[typ] = dict(n=len(rs), scored=len(sc), old_errors=sum(1 for x in rs if x["old"] >= REGRET),
                        new_errors=sum(1 for x in sc if x["new"] >= REGRET),
                        fixed=sum(1 for x in sc if x["old"] >= REGRET > x["new"]),
                        broken=sum(1 for x in sc if x["new"] >= REGRET > x["old"]),
                        changed=sum(1 for x in rs if x["changed"]),
                        other_move=sum(1 for x in rs if typ == "play" and not x["same_card"]))
    sp = [x for x in res if x["typ"] == "play" and x.get("old_eff") is not None]
    ne = lambda e, x: e is not None and e <= NOEFFECT and not x.get("before_attack")
    out["spells"] = dict(n=len(sp), old_no_effect=sum(1 for x in sp if ne(x["old_eff"], x)),
                         new_no_effect=sum(1 for x in sp if ne(x["new_eff"], x)),
                         new_not_played=sum(1 for x in sp if not x["same_card"]))
    return out


def show(b):
    def pc(x):
        return f"{100 * x['rate']:.1f} % ± {100 * x['se']:.1f}"
    print(f"{b['games']} parties, {b['positions']} positions jugées")
    print(f"  cibles des cartes jouées : {b['play']['errors']}/{b['play']['n']} erreurs (regret ≥ {REGRET}) = {pc(b['play'])}"
          f" ; meilleure cible absente de la liste de l'IA : {b['play_best_unseen']}")
    print(f"  choix pendant la résolution : {b['ask']['errors']}/{b['ask']['n']} = {pc(b['ask'])}")
    print(f"  ensemble : {pc(b['tout'])}")
    s = b["spells"]
    print(f"  sorts sans effet visible : {s['no_effect']}/{s['n']} (dont {s['before_attack']} avant une attaque de la cible) ;"
          f" hors « avant attaque » {100 * s['rate_no_effect']:.1f} % ± {100 * s['se']:.1f}")
    print("  erreurs par carte / type :", ", ".join(f"{k} {v}" for k, v in b["errors_by"]))
    print("  sans effet par carte :", ", ".join(f"{k} {v}" for k, v in b["no_effect_by"]))


def load(paths):
    pos, games = [], []
    for p in paths:
        for line in open(p, encoding="utf-8"):
            pos.append(json.loads(line))
        gp = Path(p).with_name(Path(p).name.replace("pos-", "parties-"))
        if gp.exists():
            games += [json.loads(x) for x in open(gp, encoding="utf-8")]
    if not games:
        games = [dict(seed=s) for s in sorted({p["seed"] for p in pos})]
    return pos, games


def main(args):
    if args[0] == "bilan":
        pos, games = load(args[1:])
        show(bilan(pos, games))
        return
    if args[0] == "rejuge":
        # rejuger des parties déjà jouées (juge modifié) : rejouées d'après leur script, sans recherche
        fg = Path(args[1])
        procs = int(args[2]) if len(args) > 2 else 4
        games = [json.loads(x) for x in open(fg, encoding="utf-8")]
        md5 = engine_md5(HERE.parent)
        # Autre version du moteur : le script se rejoue s'il retombe sur chaque décision (type et nombre d'options,
        # sinon Desync et la partie est écartée) ; la partie est alors gardée sous la nouvelle version (engine), la
        # version où elle a été jouée reste dans engine_played. Les notes du juge sont toutes recalculées.
        by = {x["seed"]: x for x in games}
        fp = fg.with_name(fg.name.replace("parties-", "pos-"))
        kept = []
        with Pool(procs) as p, open(fp, "w", encoding="utf-8") as op:
            for gm, pos in p.imap_unordered(job, [(x["seed"], x["mode"], x["ver"], x["script"]) for x in games]):
                if "err" in gm:
                    print("ERREUR", gm["seed"], gm["err"][-400:], flush=True)
                    continue
                old = by[gm["seed"]]
                if old.get("engine") != md5:
                    old["engine_played"] = old.get("engine_played") or old.get("engine")
                    old["engine"] = md5
                kept.append(old)
                for x in pos:
                    op.write(json.dumps(x, ensure_ascii=False) + "\n")
        with open(fg, "w", encoding="utf-8") as og:
            for x in sorted(kept, key=lambda x: x["seed"]):
                og.write(json.dumps(x, ensure_ascii=False) + "\n")
        print(f"{len(kept)}/{len(games)} parties rejouées")
        pos, games = load([fp])
        b = bilan(pos, games)
        show(b)
        save(dict(label=f"banc {games[0]['ver']} · {games[0]['mode']} (rejugé)", mode=games[0]["mode"],
                  ver=games[0]["ver"], file=fg.name, regret=REGRET, **b), games, md5)
        return
    if args[0] == "rejoue":
        fp, ver = args[1], args[2]
        procs = int(args[3]) if len(args) > 3 else 4
        only = args[4] if len(args) > 4 and args[4] in ("play", "ask") else None
        pos, games = load([fp])
        md5 = engine_md5(HERE.parent)
        jobs = [(gm, [p for p in pos if p["seed"] == gm["seed"]], ver, only) for gm in games]
        t0 = time.time()
        with Pool(procs) as p:
            res = [x for r in p.imap_unordered(rjob, jobs) for x in r]
        errs = [x for x in res if "err" in x]
        res = [x for x in res if "err" not in x]
        for e in errs[:3]:
            print("ERREUR", e["seed"], e["err"][-400:])
        with open(Path(fp).with_name(f"rejoue-{ver.replace('@', '_').replace(',', '_').replace('=', '-')}-{Path(fp).name}"),
                  "w", encoding="utf-8") as o:
            for x in res:
                o.write(json.dumps(x, ensure_ascii=False) + "\n")
        out = rsummary(res)
        out.update(label=f"banc rejoué {ver} sur {Path(fp).name}", ver=ver, file=Path(fp).name, errors=len(errs),
                   engine_md5=md5, games_engine=sorted({g.get("engine") for g in games}), seconds=round(time.time() - t0))
        print(json.dumps(out, ensure_ascii=False, indent=1))
        f = HERE / "results_banc.json"
        allr = json.load(open(f, encoding="utf-8")) if f.exists() else []
        allr.append(out)
        json.dump(allr, open(f, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
        return
    n, seed0, mode = int(args[1]), int(args[2]), args[3]
    ver = args[4] if len(args) > 4 else "turn"
    procs = int(args[5]) if len(args) > 5 else 4
    dd = HERE / "data"
    dd.mkdir(exist_ok=True)
    fp, fg = dd / f"pos-{seed0}.jsonl", dd / f"parties-{seed0}.jsonl"
    md5 = engine_md5(HERE.parent)
    done = set()
    if fg.exists():
        for line in open(fg, encoding="utf-8"):
            x = json.loads(line)
            if x.get("engine") != md5:
                sys.exit(f"{fg} : autre version du moteur ({x.get('engine')} au lieu de {md5}) ; changer de graine0")
            done.add(x["seed"])
    todo = [(seed0 + k, mode, ver) for k in range(n) if seed0 + k not in done]
    t0, errs = time.time(), []
    with Pool(procs) as p, open(fp, "a", encoding="utf-8") as op, open(fg, "a", encoding="utf-8") as og:
        for gm, pos in p.imap_unordered(job, todo):
            if "err" in gm:
                errs.append(gm)
                print("ERREUR", gm["seed"], gm["err"][-400:], flush=True)
                continue
            gm["engine"] = md5
            og.write(json.dumps(gm, ensure_ascii=False) + "\n")
            for x in pos:
                op.write(json.dumps(x, ensure_ascii=False) + "\n")
            og.flush()
            op.flush()
            print(f"graine {gm['seed']} : {len(pos)} positions, {gm['sec']} s", flush=True)
    pos, games = load([fp])
    b = bilan(pos, games)
    show(b)
    save(dict(label=f"banc {ver} · {mode}", mode=mode, ver=ver, file=fg.name, regret=REGRET, errors=len(errs),
              seconds=round(time.time() - t0), **b), games, md5)


def save(res, games, md5):
    res["seeds"] = dict(first=min(x["seed"] for x in games), last=max(x["seed"] for x in games), n=len(games))
    res["engine_md5"] = md5
    f = HERE / "results_banc.json"
    allr = json.load(open(f, encoding="utf-8")) if f.exists() else []
    allr.append(res)
    json.dump(allr, open(f, "w", encoding="utf-8"), ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main(sys.argv[1:])
