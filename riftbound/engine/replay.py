#!/usr/bin/env python3
"""Enregistreur de replays (additif : ne modifie ni game.py ni ai.py).

RecGame hérite de Game et prend une photo de l'état à chaque ligne de journal ; RecAgent hérite de SearchAgent,
fait exactement les mêmes calculs (mêmes appels au générateur aléatoire) et note le score de chaque option
envisagée. record(seed, deck_akali, deck_leblanc, ...) renvoie un dict JSON prêt pour le lecteur
(riftbound/replays/viewer.html).

  python3 replay.py <seed> [bf_akali] [bf_leblanc] [first]   -> replays/seed_<seed>.json
"""
import json, re, sys, random
from pathlib import Path
from game import Game, Obj, Item, SPEC, EQUIP_BONUS
from ai import SearchAgent, determinize, PolicyAgent, rollout_policy, evaluate
import game as _game

HERE = Path(__file__).resolve().parent
OUT = HERE.parent / "replays"
NAMES = ("Akali", "LeBlanc")
UID = re.compile(r"#(\d+)")


def short(x):
    return UID.sub("", str(x))


def fr_line(g, line):
    """Traduction française d'une ligne du journal du moteur."""
    s = line.strip()
    bf = lambda m: g.bfs[int(m)].name if m in ("0", "1") else ("la base" if m == "base" else m)
    rules = [
        (r"^P(\d) plays (.+?) from (hand|champ|facedown|trash) ?(.*)$",
         lambda m: f"{NAMES[int(m[1])]} joue {m[2]}" + {"hand": "", "champ": " (zone champion)",
                   "facedown": " (carte cachée révélée)", "trash": " depuis la défausse"}[m[3]] + fmt_args(g, m[4])),
        (r"^P(\d) activates (.+?) ?(\(.*\))?$", lambda m: f"{NAMES[int(m[1])]} active {m[2]}" + fmt_args(g, m[3] or "")),
        (r"^P(\d) moves \[(.*)\] -> (.+)$", lambda m: f"{NAMES[int(m[1])]} déplace {m[2]} vers {bf(m[3])}"),
        (r"^P(\d) hides a card at (.+)$", lambda m: f"{NAMES[int(m[1])]} cache une carte à {m[2]}"),
        (r"^P(\d) \+1 point \((.+)\) -> (\d+)$", lambda m: f"{NAMES[int(m[1])]} +1 point ({pt_why(m[2])}) → {m[3]}"),
        (r"^P(\d) pays Deflect \+(\d+) power$",
         lambda m: f"{NAMES[int(m[1])]} paie Deflect : +{m[2]} puissance de n'importe quel domaine (règle 809)"),
        (r"^P(\d) reveals their hand \[(.*)\]$",
         lambda m: f"{NAMES[int(m[1])]} révèle sa main : {m[2] or 'vide'}"),
        (r"^resolve (.+)$", lambda m: f"Résolution : {m[1]}"),
        (r"^(.+) takes (\d+) \((\d+) might\)$", lambda m: f"{m[1]} subit {m[2]} dégât(s) (might {m[3]})"),
        (r"^(.+) moves (\w+)->(\w+)$", lambda m: f"{m[1]} va de {bf(m[2])} à {bf(m[3])}"),
        (r"^(.+) is stunned$", lambda m: f"{m[1]} est étourdi(e)"),
        (r"^(.+) dies$", lambda m: f"{m[1]} meurt"),
        (r"^(.+) is countered$", lambda m: f"{m[1]} est contré(e)"),
        (r"^(.+) attached to (.+)$", lambda m: f"{m[1]} est attaché(e) à {m[2]}"),
        (r"^trigger on chain: (.+)$", lambda m: f"Déclenchement sur la chaîne : {m[1]}"),
        (r"^hidden (.+) at (.+) is trashed$", lambda m: f"La carte cachée {m[1]} à {m[2]} part à la défausse"),
        (r"^combat damage: attackers (\d+) vs defenders (\d+)$",
         lambda m: f"Dégâts de combat : attaquants {m[1]} contre défenseurs {m[2]}"),
        (r"^Zhonya's Hourglass saves (.+)$", lambda m: f"Zhonya's Hourglass sauve {m[1]}"),
        (r"^Reflection of (.+) at (.+)$", lambda m: f"Reflet de {m[1]} à {bf(m[2])}"),
        (r"^P(\d) takes control of (.+)$", lambda m: f"{NAMES[int(m[1])]} prend le contrôle de {m[2]}"),
        (r"^P(\d) draws instead of the final point$", lambda m: f"{NAMES[int(m[1])]} pioche au lieu du dernier point"),
        (r"^P(\d) loses control of (.+)$", lambda m: f"{NAMES[int(m[1])]} perd le contrôle de {m[2]}"),
        (r"^P(\d) draws (.+)$", lambda m: f"{NAMES[int(m[1])]} pioche {m[2]}"),
        (r"^Baited Hook finds (.+)$", lambda m: f"Baited Hook trouve {m[1]}"),
        (r"^COMBAT at (.+), attacker P(\d)$", lambda m: f"COMBAT à {m[1]}, attaquant {NAMES[int(m[2])]}"),
        (r"^showdown at (.+), attacker P(\d)$", lambda m: f"Showdown à {m[1]}, initiative {NAMES[int(m[2])]}"),
        (r"^showdown becomes a combat$", lambda m: "Le showdown devient un combat"),
        (r"^WINNER (\d)$", lambda m: f"VICTOIRE de {NAMES[int(m[1])]}"),
        (r"^BURN OUT (\d)$", lambda m: f"{NAMES[int(m[1])]} est à court de deck (Burn Out)"),
        (r"^Threshold of the Gray: both players add 1 energy$", lambda m: "Threshold of the Gray : chaque joueur ajoute 1 énergie"),
        (r"^===== turn (\d+): P(\d) .*points (\d+)-(\d+)$",
         lambda m: f"Tour {m[1]}, à {NAMES[int(m[2])]} de jouer (score {m[3]}-{m[4]})"),
    ]
    for pat, fn in rules:
        m = re.match(pat, s)
        if m:
            return short(fn(m))
    return short(s)


def pt_why(w):
    m = re.match(r"(conquer|hold) (.+)", w)
    if m:
        return ("conquête de " if m[1] == "conquer" else "tenue de ") + m[2]
    return w


def fmt_args(g, a):
    a = (a or "").strip().strip("()")
    if not a:
        return ""
    parts = []
    for k, v in re.findall(r"(\w+)=(.*?)(?= \w+=|$)", a):
        if k in ("tg", "tg2"):
            v = re.sub(r"(#\d+),", r"\1 et ", v.strip(","))
            if v:
                parts.append("cible " + v)
        elif k == "loc":
            parts.append("à " + (g.bfs[int(v)].name if v in ("0", "1") else "la base"))
        elif k == "dest":
            parts.append("vers " + (g.bfs[int(v)].name if v in ("0", "1") else "la base"))
        elif k in ("mover", "mover_oid", "kill", "xp"):
            continue
        else:
            parts.append(f"{k}={v}")
    return (" — " + ", ".join(parts)) if parts else ""


def describe(g, a, pid):
    """Texte français d'une action de haut niveau, avant qu'elle soit appliquée."""
    k = a[0]
    if k == "pass":
        return "passe"
    if k == "end":
        return "termine son tour"
    if k == "play":
        _, uid, src, ch = a
        c = _find(g, pid, uid)
        n = c.cname if c else "?"
        extra = []
        for kk, v in dict(ch).items():
            if kk in ("tg", "tg2") and v:
                extra.append("cible " + " et ".join(short(g.obj(u) or u) for u in v))
            elif kk == "loc":
                extra.append("à " + (g.bfs[v].name if v in (0, 1) else "la base"))
            elif kk == "dest" and v is not None:
                extra.append("vers " + (g.bfs[v].name if v in (0, 1) else "la base"))
            elif kk in ("accelerate", "repeat") and v:
                extra.append(kk)
        if c is not None:
            from actions import deflect_total
            nd = len(deflect_total(g, pid, c, dict(ch)))
            if nd:
                extra.append(f"Deflect +{nd} puissance")
        return f"joue {n}" + (" (" + ", ".join(extra) + ")" if extra else "")
    if k == "hide":
        c = _find(g, pid, a[1])
        return f"cache {c.cname if c else '?'} à {g.bfs[a[2]].name}"
    if k == "act":
        src = a[1]
        n = g.p[src[1]].legend_name if isinstance(src, tuple) else (g.obj(src).cname if g.obj(src) else "?")
        return f"active {n}"
    if k == "move":
        us = ", ".join(short(g.obj(u)) for u in a[1])
        d = a[2]
        return f"déplace {us} vers " + (g.bfs[d].name if d in (0, 1) else "la base")
    return str(a)


def _find(g, pid, uid):
    pl = g.p[pid]
    for c in pl.hand + pl.champ + pl.trash + [c for b in g.bfs for c in b.facedowns]:
        if c.uid == uid:
            return c
    return g.obj(uid)


# ---------------------------------------------------------------------------------- state snapshot
def snap(g):
    def unit(o):
        d = dict(u=o.uid, n=o.cname, c=o.ctrl, m=g.might(o), b=SPEC[o.cname]["might"])
        if o.damage: d["d"] = o.damage
        if o.exhausted: d["x"] = 1
        if o.stunned: d["s"] = 1
        if o.buff: d["bf"] = o.buff
        if o.empowered: d["e"] = 1
        if o.token: d["t"] = 1
        if o.attached:
            d["g"] = [g.obj(x).cname for x in o.attached if g.obj(x) is not None]
        kws = [kw for kw in ("Temporary", "Tank", "Backline", "Ganking", "Deflect", "Assault", "Shield") if g.has_kw(o, kw)]
        if kws: d["k"] = kws
        return d

    st = dict(t=g.turn_no, tp=g.tp, pts=[p.points for p in g.p], v=g.victory)
    locs = {"base": [[], []], 0: [], 1: []}
    loose_gear = [[], []]
    for o in g.board:
        if o.spec["type"] == "Unit":
            if o.loc == "base":
                locs["base"][o.ctrl].append(unit(o))
            elif o.loc in (0, 1):
                locs[o.loc].append(unit(o))
        elif o.attached_to is None:
            loose_gear[o.ctrl].append(dict(u=o.uid, n=o.cname, x=1 if o.exhausted else 0))
    pl = []
    for i, p in enumerate(g.p):
        pl.append(dict(
            hand=[[c.uid, c.cname] for c in p.hand],
            champ=[c.cname for c in p.champ],
            deck=len(p.deck), trash=[c.cname for c in p.trash],
            runes=[(r.domain[:2] + ("x" if r.exhausted else "")) for r in p.runes], rdeck=len(p.rune_deck),
            leg=dict(n=p.legend_name, x=1 if p.legend.exhausted else 0, e=1 if p.legend.empowered else 0),
            base=locs["base"][i], gear=loose_gear[i], xp=p.xp, pe=p.pool_e,
            pp={d: n for d, n in sorted(p.pool_p.items()) if n}))   # Rune Pool : énergie et puissance flottantes
    st["p"] = pl
    st["bfs"] = [dict(n=b.name, c=b.ctrl, fd=(None if b.facedown is None else [b.facedown.owner, b.facedown.cname]),
                      u=locs[b.idx]) for b in g.bfs]
    st["chain"] = [dict(n=short(it.name), c=it.ctrl, k=it.kind) for it in g.chain]
    if g.sd is not None:
        st["sd"] = dict(bf=g.sd.bf.idx if hasattr(g.sd.bf, "idx") else g.sd.bf, combat=g.sd.combat, a=g.sd.attacker)
    return st


class RecGame(Game):
    """Game qui photographie l'état à chaque ligne du journal. Les copies faites par l'IA n'enregistrent rien."""

    def __init__(s, *a, **k):
        k["log"] = True
        s.frames = []
        s.pending = []
        s.pdec = None
        super().__init__(*a, **k)

    def log(s, *a):
        if not s.logging:
            return
        super().log(*a)
        raw = " ".join(str(x) for x in a)
        txt = fr_line(s, raw)
        st = snap(s)
        uids = [int(x) for x in UID.findall(raw)]
        ind = 1 if raw.startswith("  ") else 0
        if s.frames and s.frames[-1]["st"] == st and not s.pending and s.pdec is None \
                and not raw.startswith("====="):
            s.frames[-1]["ev"].append([ind, txt])
            s.frames[-1]["hl"] = sorted(set(s.frames[-1]["hl"]) | set(uids))
            return
        f = dict(ev=[[ind, txt]], st=st, hl=uids, ev0=round(max(-60, min(60, evaluate(s, 0))), 1))
        if s.pending:
            f["notes"] = s.pending
            s.pending = []
        if s.pdec is not None:
            f["dec"] = s.pdec
            s.pdec = None
        s.frames.append(f)

    def draw(s, pid, n=1):
        """Chaque pioche devient une ligne du journal : sinon une carte piochée puis jouée aussitôt
        n'apparaît jamais dans la main du replay."""
        for _ in range(n):
            before = list(s.p[pid].hand)
            super().draw(pid, 1)
            if s.winner is not None:
                return
            new = [c for c in s.p[pid].hand if c not in before]
            if new and s.logging and s.stage != "setup":
                s.log(f"  P{pid} draws {new[0].cname}#{new[0].uid}")

    def decision_frame(s, pid, kind, chosen, alts, n_opts):
        st = snap(s)
        f = dict(ev=[[0, f"{NAMES[pid]} décide : {chosen}"]], st=st, hl=[], dec=dict(p=pid, k=kind, n=n_opts, alts=alts),
                 ev0=round(max(-60, min(60, evaluate(s, 0))), 1))
        if s.pending:
            f["notes"] = s.pending
            s.pending = []
        s.frames.append(f)

    def clone(s):
        fr, pe, pd = s.frames, s.pending, s.pdec
        s.frames, s.pending, s.pdec = [], [], None
        try:
            return super().clone()
        finally:
            s.frames, s.pending, s.pdec = fr, pe, pd


class RecAgent(SearchAgent):
    """SearchAgent identique (mêmes tirages aléatoires), qui note les scores des options."""

    def decide(s, g, d):
        opts = d.options
        if len(opts) == 1:
            return opts[0]
        me = d.player
        full = opts
        if s.cfg.get("respect_ambush") and d.kind == "main":
            opts = [a for a in opts if not s.risky_attack(g, me, a)] or opts
        if len(opts) > s.max_cands:
            opts = opts[:1] + s.rng.sample(opts[1:], s.max_cands - 1)
        best, best_v, scored = s.pick(g, d, opts)
        if isinstance(g, RecGame):
            scored.sort(key=lambda x: -x[0])
            alts = [[describe(g, a, me), round(v - best_v, 1), 1 if a == best else 0] for v, a in scored[:6]]
            if best is not None and best not in [a for _, a in scored[:6]]:
                alts.append([describe(g, best, me), 0.0, 1])
            if best[0] == "pass":
                # une fenêtre de réaction laissée passer : simple note sur l'image suivante
                if len(scored) > 1:
                    second = next((x for x in scored if x[1][0] != "pass"), None)
                    if second is not None:
                        gap = second[0] - best_v
                        gap = "perdait la partie" if gap < -1000 else f"{gap:.1f}"
                        g.pending.append(f"{NAMES[me]} pouvait réagir ({describe(g, second[1], me)}, {gap}) mais passe")
            elif best[0] == "end":
                g.decision_frame(me, d.kind, describe(g, best, me), alts, len(full))
            else:
                g.pdec = dict(p=me, k=d.kind, n=len(full), alts=alts)
        return best

    def mulligan(s, g, pid):
        out = super().mulligan(g, pid)
        if isinstance(g, RecGame):
            g.mull = getattr(g, "mull", {})
            g.mull[pid] = dict(hand=[c.cname for c in g.p[pid].hand], out=[c.cname for c in out])
        return out


def card_db(names):
    db = {}
    for n in names:
        sp = SPEC.get(n)
        if sp is None:
            continue
        db[n] = dict(t=sp["type"], s=sp["super"], d=list(sp["domains"]), e=sp["e"], p=sp["p"], m=sp["might"],
                     tx=sp["text"], tg=sorted(sp["tags"]))
        if n in EQUIP_BONUS:
            db[n]["eb"] = EQUIP_BONUS[n]
    return db


def record(seed, a_deck, l_deck, a_bf=None, l_bf=None, first=None, cfg=None, reset_ids=True):
    """Joue la partie `seed` comme exp.one (mêmes tirages de battlefield / premier joueur) et l'enregistre."""
    from decks import with_bf
    if reset_ids:
        Obj._n = 0
        Item._n = 0
    r = random.Random(seed * 7919)
    abf = r.choice(a_deck["battlefields"]) if a_bf is None else a_bf
    lbf = r.choice(l_deck["battlefields"]) if l_bf is None else l_bf
    f = r.randrange(2) if first is None else first
    A, L = with_bf(a_deck, abf), with_bf(l_deck, lbf)
    ag = [RecAgent(seed, cfg=cfg), RecAgent(seed + 500000)]
    g = RecGame([A, L], ag, seed=seed, first=f)
    while True:
        d = g.advance()
        if d is None:
            break
        g.apply(ag[d.player].decide(g, d))
    names = set(A["main"]) | set(L["main"]) | {A["legend"], L["legend"], A.get("champion"), L.get("champion"),
                                                 "Mech", "Reflection", "Gold"}
    for fr in g.frames:
        for p in fr["st"]["p"]:
            names |= {c[1] for c in p["hand"]}
    return dict(seed=seed, first=f, winner=g.winner, turns=g.turn_no, pts=[p.points for p in g.p],
                players=[dict(name=NAMES[i], legend=d["legend"], champion=d.get("champion"), bf=d["battlefield"],
                              deck=d.get("key", ""), main=sorted(set(d["main"]), key=d["main"].index))
                         for i, d in enumerate((A, L))],
                mulligan=getattr(g, "mull", {}), frames=g.frames, cards=card_db(n for n in names if n))


def summary(rep):
    """Indicateurs pour choisir les parties intéressantes : écart max de score, retournements, cartes jouées."""
    max_def = [0, 0]
    played = [set(), set()]
    lead_changes, last = 0, 0
    for fr in rep["frames"]:
        a, l = fr["st"]["pts"]
        max_def[0] = max(max_def[0], l - a)
        max_def[1] = max(max_def[1], a - l)
        lead = (a > l) - (a < l)
        if lead and last and lead != last:
            lead_changes += 1
        if lead:
            last = lead
        for _, t in fr["ev"]:
            for i, n in enumerate(NAMES):
                if t.startswith(n + " joue "):
                    played[i].add(t[len(n) + 6:].split(" (")[0].split(" — ")[0].split(" depuis")[0])
    return dict(seed=rep["seed"], first=rep["first"], winner=rep["winner"], turns=rep["turns"], pts=rep["pts"],
                bf=[p["bf"] for p in rep["players"]], akali_max_behind=max_def[0], akali_max_ahead=max_def[1],
                lead_changes=lead_changes, played=[sorted(x) for x in played], frames=len(rep["frames"]))


# ---------------------------------------------------------------------------------------- parties avec plans de jeu
def _rec_plan_agent():
    from plans import PlanAgent

    class RecPlanAgent(RecAgent, PlanAgent):
        """PlanAgent (scores du plan) qui note aussi les options comme RecAgent."""
    return RecPlanAgent


def record_plan(seed, a_deck, l_deck, a_plan=None, l_plan=None, a_bfs=None, l_bfs=None, first=None, agent_kw=None):
    """Rejoue la partie `seed` exactement comme exp_plans.one et manager/run_session.one (mêmes tirages, mêmes plans,
    battlefields ou premier joueur éventuellement forcés) et l'enregistre. `agent_kw` : [dict Akali, dict LeBlanc]
    passés aux agents (ex. recherche search/samples/sh_extra d'exp_search.py)."""
    import plans as P
    from decks import with_bf
    Obj._n = 0
    Item._n = 0
    r = random.Random(seed * 7919)
    f = r.randrange(2) if first is None else first
    pa = P.PLANS[a_plan]() if a_plan else None
    pl = P.PLANS[l_plan]() if l_plan else None
    lbf = (pl.battlefield(l_deck, f == 1) if pl and not l_bfs else None) or r.choice(l_bfs or l_deck["battlefields"])
    abf = (pa.battlefield(a_deck, f == 0, lbf) if pa and not a_bfs else None) or r.choice(a_bfs or a_deck["battlefields"])
    A, L = with_bf(a_deck, abf), with_bf(l_deck, lbf)
    RA = _rec_plan_agent()
    kw = agent_kw or [{}, {}]
    ag = [RA(seed, plan=pa, opp_plan=pl, **kw[0]), RA(seed + 500000, plan=pl, opp_plan=pa, **kw[1])]
    g = RecGame([A, L], ag, seed=seed, first=f)
    while True:
        d = g.advance()
        if d is None:
            break
        g.apply(ag[d.player].decide(g, d))
    names = set(A["main"]) | set(L["main"]) | {A["legend"], L["legend"], A.get("champion"), L.get("champion"),
                                                 "Mech", "Reflection", "Gold"}
    for fr in g.frames:
        for p in fr["st"]["p"]:
            names |= {c[1] for c in p["hand"]}
    return dict(seed=seed, first=f, winner=g.winner, turns=g.turn_no, pts=[p.points for p in g.p],
                plans=[a_plan or "aucun", l_plan or "aucun"],
                players=[dict(name=NAMES[i], legend=d["legend"], champion=d.get("champion"), bf=d["battlefield"],
                              deck=d.get("key", ""), main=sorted(set(d["main"]), key=d["main"].index))
                         for i, d in enumerate((A, L))],
                mulligan=getattr(g, "mull", {}), frames=g.frames, cards=card_db(n for n in names if n))


if __name__ == "__main__":
    from exp_gorica import G2
    from exp import L
    seed = int(sys.argv[1])
    abf = sys.argv[2] if len(sys.argv) > 2 and sys.argv[2] != "-" else None
    lbf = sys.argv[3] if len(sys.argv) > 3 and sys.argv[3] != "-" else None
    first = int(sys.argv[4]) if len(sys.argv) > 4 else None
    rep = record(seed, G2, L, abf, lbf, first)
    OUT.mkdir(exist_ok=True)
    p = OUT / f"seed_{seed}.json"
    json.dump(rep, open(p, "w"), ensure_ascii=False, separators=(",", ":"))
    print(p, summary(rep))

