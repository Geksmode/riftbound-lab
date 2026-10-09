#!/usr/bin/env python3
"""Plans de jeu tirés des joueurs réels (additif : ne modifie ni game.py ni ai.py).

Sources résumées dans ../gameplans/ (transcriptions de vidéos, guide Metafy de Gorica, guides écrits).
Un plan agit à cinq endroits :
  - mulligan(g, pid)                : ce qu'on garde / renvoie ;
  - battlefield(deck, first, opp)   : battlefield présenté selon qui commence ;
  - prior(g, me, a)                 : bonus/malus (unités d'évaluation, 7 ~ 1 point) ajouté au score d'une action ;
  - shape(g, me)                    : termes ajoutés à l'évaluation en fin de simulation (ce que le deck valorise) ;
  - choose(...)                     : choix pendant la résolution (défausse, copie, « vous pouvez »).
PlanAgent = SearchAgent + plan ; mêmes simulations, seul le score change.
"""
import random
from ai import SearchAgent, PolicyAgent, Heuristics, determinize, rollout_policy, evaluate, card_value, danger
import ai as _ai
import os

# Vidéos LeBlanc (transcriptions collées par l'utilisateur, 3 oct.), actives par défaut (RB_VIDEO=0 les coupe) :
# copie de Ruined Rex / Watcher en priorité, défausse d'un Karthus en double si Glasc peut le ramener, Watcher closer ;
# vidéo Riftlab « Leblanc Deceiver guide » : le Reflet prêt va prendre l'autre battlefield (Windswept).
VIDEO = os.environ.get("RB_VIDEO", "1") != "0"
KEEP_REFLECTION = os.environ.get("RB_REFL", "1") != "0"   # RB_REFL=0 : ancien LeBlanc (rappelle Reflet / original, cache peu)
from cards import value as unit_value

DK = {"Soaring Scout", "Honest Broker", "Watchful Sentry", "Black Rose Dignitary", "LeBlanc, Fragmented",
      "Glasc Mixologist", "Ruined Rex", "Rift Herald"}


def card_of(g, uid):
    for pl in g.p:
        for z in (pl.hand, pl.champ, pl.trash):
            for c in z:
                if c.uid == uid:
                    return c
    for b in g.bfs:
        if b.facedown is not None and b.facedown.uid == uid:
            return b.facedown
    return g.obj(uid)


class Plan:
    name = "aucun"

    def mulligan(s, g, pid):
        return None                      # None = heuristique par défaut

    def battlefield(s, deck, first, opp_bf=None):
        return None                      # None = tirage au hasard (comme avant)

    def prior(s, g, me, a):
        return 0.0

    def shape(s, g, me):
        return 0.0

    def choose(s, g, pid, kind, options, ctx):
        return NotImplemented


# ---------------------------------------------------------------------------------------- Akali (Gorica)
class AkaliGorica(Plan):
    """Gorica : « Akali n'est pas un midrange agressif, c'est un deck contrôle lent orienté valeur ; les points ne
    comptent pas, il faut juste ne pas perdre. » Moteur Stellacorn Herder + Astral Heron, légende non empowered
    utilisée pour rappeler Herder (pioche), s'installer sur UN battlefield avec Heron et une carte cachée."""
    name = "Gorica"
    ENGINE = {"Stellacorn Herder", "Astral Heron"}
    FOUR = {"Stellacorn Herder", "Kai'Sa, Survivor", "Noxus Hopeful", "Akali, Silent"}
    PROTECT = {"Zhonya's Hourglass", "Back Off", "Block"}
    TWO = {"Mischievous Marai", "Mournful Witness", "Lonely Poro", "Scuttle Crab"}

    def mulligan(s, g, pid):
        # Metafy (Gorica) : renvoyer 2 cartes sauf si la main a Stellacorn ET Heron ; ne jamais renvoyer d'unité.
        h = g.p[pid].hand
        names = {c.cname for c in h}
        if "Stellacorn Herder" in names and "Astral Heron" in names:
            return []
        return s._throw(g, pid, set())

    def _throw(s, g, pid, keep):
        # renvoyer les sorts/équipements les plus chers, en gardant une première protection
        cand = sorted((c for c in g.p[pid].hand if c.spec["type"] != "Unit" and c.cname not in keep),
                      key=lambda c: -c.spec["e"])
        prot = next((c for c in cand if c.cname in s.PROTECT), None)
        return [c for c in cand if c is not prot][:2]

    def battlefield(s, deck, first, opp_bf=None):
        # Metafy : Void Gate à l'aveugle ; contre LeBlanc garder Sigil pour ses parties Windswept (elle joue
        # Windswept quand elle commence, donc quand Akali est seconde).
        opts = deck["battlefields"]
        pick = "Sigil of the Storm" if (opp_bf == "Windswept Hillock" or (opp_bf is None and not first)) else "Void Gate"
        return pick if pick in opts else None

    def prior(s, g, me, a):
        pl = g.p[me]
        k = a[0]
        v = 0.0
        if k == "act" and isinstance(a[1], tuple):            # légende
            if a[2] == 0:                                     # Empower : rare, luxe de fin de partie
                v -= 8.0 if pl.turns < 6 else 1.0
            elif a[2] == 1:                                   # Retreat : la boucle Herder
                u = g.obj(a[3].get("tg", (None,))[0]) if a[3].get("tg") else None
                if u is not None and u.cname == "Stellacorn Herder":
                    v += 4.0
                elif u is not None and u.cname == "Astral Heron":
                    v -= 3.0                                  # Heron reste sur le battlefield
        elif k == "move" and a[2] in (0, 1):
            units = [g.obj(u) for u in a[1]]
            names = {u.cname for u in units if u is not None}
            enemies = g.units(1 - me, a[2])
            if "Stellacorn Herder" in names and not pl.legend.exhausted and not enemies:
                v += 3.0                                      # entrer pour piocher, puis rappeler
            # pas de course aux points en début de partie : ne pas engager seul contre un défenseur
            if enemies and pl.turns <= 3 and len(units) == 1:
                v -= 2.0
            if any(e.cname == "Karthus, Eternal" for e in enemies):
                v -= 1.5                                      # ne pas nourrir les Deathknell (BMU)
        elif k == "play":
            c = card_of(g, a[1])
            loc = dict(a[3]).get("loc")
            if c is not None and c.cname == "Astral Heron" and loc in (0, 1):
                # Metafy : Heron sur un battlefield seulement avec une carte cachée pour la déclencher
                v += 3.0 if g.bfs[loc].facedown is not None and g.bfs[loc].facedown.owner == me else -2.0
            if c is not None and c.cname == "Stellacorn Herder":
                v += 1.5
        elif k == "hide":
            if any(u.cname == "Astral Heron" and u.loc == a[2] for u in g.units(me)):
                v += 1.5                                      # carte cachée = réduction Heron gratuite
        return v

    def shape(s, g, me):
        if g.winner is not None:
            return 0.0
        v = 0.0
        for u in g.units(me):
            if u.cname == "Stellacorn Herder":
                v += 2.0
            elif u.cname == "Astral Heron":
                v += 3.0 if u.loc in (0, 1) else 1.0
        v += 0.4 * len(g.p[me].hand)                          # avance de ressources
        if not any(u.loc == "base" for u in g.units(me)) and any(u.loc in (0, 1) for u in g.units(me)):
            v -= 1.5                                          # ne jamais engager toutes ses unités
        return v


class AkaliGoricaAggro(AkaliGorica):
    """Mise à jour Metafy contre LeBlanc (Gorica le donne 45/55) : jouer AGRESSIF en milieu de partie,
    mulligan pour Kai'Sa, Darius, Blitzcrank, Shuriken Flip, Falling Star et l'interaction, empower Deadly Weapon
    pour tuer les unités que LeBlanc laisse sur les battlefields, tuer Karthus à vue, se méfier de Vi."""
    name = "Gorica agressif"
    KEEP = {"Kai'Sa, Survivor", "Darius, Trifarian", "Blitzcrank, Impassive", "Shuriken Flip", "Falling Star",
            "Stellacorn Herder", "Back Off", "Akali, Silent", "Ferrous Forerunner"}

    def mulligan(s, g, pid):
        h = g.p[pid].hand
        out = []
        herons = 0
        for c in sorted(h, key=lambda c: -c.spec["e"]):
            if c.cname == "Astral Heron":
                herons += 1
                if herons > 1:
                    out.append(c)                             # une seule Heron suffit dans ce plan
            elif c.cname not in s.KEEP and c.spec["type"] != "Unit":
                out.append(c)
        return out[:2]

    def prior(s, g, me, a):
        v = super().prior(g, me, a)
        pl = g.p[me]
        k = a[0]
        karthus = {u.uid for u in g.units(1 - me) if u.cname == "Karthus, Eternal"}
        ch = a[3] if len(a) > 3 and isinstance(a[3], dict) else {}
        if karthus and any(t in karthus for t in list(ch.get("tg", ())) + list(ch.get("tg2", ()))):
            v += 5.0                                          # tuer Karthus à vue
        temp = {u.uid for u in g.units(1 - me) if g.has_kw(u, "Temporary")}
        if temp and k == "play" and any(t in temp for t in ch.get("tg", ())):
            v -= 2.0                                          # viser la source, pas le Reflet qui meurt seul
        if k == "act" and not isinstance(a[1], tuple):
            o = g.obj(a[1])
            if o is not None and o.cname == "Akali, Deadly Weapon" and a[2] == 0:
                if any(u.loc in (0, 1) for u in g.units(1 - me)):
                    v += 4.0                                  # empower pour tuer les unités sur les battlefields
        elif k == "move" and a[2] in (0, 1) and 3 <= pl.turns:
            v += 1.5                                          # pression en milieu de partie
        return v

    def shape(s, g, me):
        if g.winner is not None:
            return 0.0
        v = super().shape(g, me)
        if any(u.cname == "Karthus, Eternal" for u in g.units(1 - me)):
            v -= 6.0
        v += 0.3 * sum(max(0, g.might(u)) for u in g.units(me) if u.loc in (0, 1))
        return v


# ---------------------------------------------------------------------------------------- LeBlanc (Deathknell)
class LeBlancDeathknell(Plan):
    """« Tu es le deck contrôle. Ne fais pas la course. Échange tes petites unités tôt, chaque mort te fait piocher
    ou rampe. » Corps Deathknell avant Karthus, Karthus à la base, Glasc cible de copie, échelle Baited Hook,
    puis un « omega hold » sur un battlefield."""
    name = "Deathknell"
    NEVER_DISCARD = {"Watchful Sentry", "Karthus, Eternal", "Glasc Mixologist", "Ruined Rex", "Mirror Image"}
    DISCARD_ORDER = ["Sacrifice", "Black Rose Dignitary", "Baited Hook", "Harnessed Dragon", "Rift Herald",
                     "Mirror Image", "Deathgrip", "Hidden Blade", "Honest Broker", "Soaring Scout"]
    COPY_PREF = ["Glasc Mixologist", "LeBlanc, Fragmented", "Vi, Peacekeeper", "Rift Herald", "Harnessed Dragon",
                 "Astral Heron", "Ruined Rex", "Thousand-Tailed Watcher"]

    def mulligan(s, g, pid):
        h = g.p[pid].hand
        first = g.first == pid
        out = []
        karthus = 0
        two = [c for c in h if c.spec["type"] == "Unit" and c.spec["e"] <= 3 and c.cname in DK]
        for c in sorted(h, key=lambda c: -(c.spec["e"] + c.spec["p"])):
            n = c.cname
            if n in ("Harnessed Dragon", "Rift Herald", "Thousand-Tailed Watcher"):
                out.append(c)
            elif n == "Karthus, Eternal":
                karthus += 1
                if karthus > 1:
                    out.append(c)
            elif n == "Hidden Blade" and first:
                out.append(c)
        if first and not two:
            out += [c for c in h if c not in out and c.spec["e"] >= 5][:2]
        return out[:2]

    def battlefield(s, deck, first, opp_bf=None):
        pick = "Windswept Hillock" if first else "Star Spring"
        return pick if pick in deck["battlefields"] else None

    def prior(s, g, me, a):
        pl = g.p[me]
        k = a[0]
        v = 0.0
        if k == "play":
            c = card_of(g, a[1])
            if c is None:
                return 0.0
            n = c.cname
            loc = dict(a[3]).get("loc")
            if pl.turns == 1 and n in ("Soaring Scout", "Black Rose Dignitary", "LeBlanc, Fragmented",
                                       "Honest Broker", "Watchful Sentry"):
                v += 3.0                                      # toujours jouer au tour 1
            if VIDEO and n == "Thousand-Tailed Watcher":
                opp_bf = sum(1 for u in g.units(1 - me) if u.loc in (0, 1))
                if opp_bf >= 2 and not any(b.ctrl == me for b in g.bfs):
                    v += 2.0                                  # closer : l'adversaire a tout poussé, on est dehors
            if n == "Karthus, Eternal":
                if not any(u.cname in DK or u.cname == "Reflection" for u in g.units(me)):
                    v -= 3.0                                  # corps Deathknell d'abord
                if loc in (0, 1):
                    v -= 3.0                                  # Karthus reste à la base
        elif k == "move" and a[2] in (0, 1):
            units = [g.obj(u) for u in a[1]]
            if any(u is not None and u.cname == "Karthus, Eternal" for u in units):
                v -= 4.0
            if VIDEO and g.bfs[a[2]].ctrl != me and any(u is not None and u.token and u.name == "Reflection"
                                                         and u.loc in (0, 1) and u.loc != a[2] for u in units):
                v += 2.0                                      # Riftlab : le Reflet prêt gank l'autre battlefield
        elif k == "hide" and KEEP_REFLECTION:
            # joueurs réels (utilisateur, 3 oct.) : LeBlanc cache une carte là où il tient, même avec un seul Reflet
            v += 2.5
            if all(u.token and u.name == "Reflection" for u in g.units(me, a[2])):
                v += 1.5
        elif k == "move" and a[2] == "base" and KEEP_REFLECTION:
            # joueurs réels (utilisateur, 3 oct.) : le Reflet et l'unité copiée restent sur le battlefield
            refl = [u for u in g.units(me) if u.token and u.name == "Reflection" and u.loc in (0, 1)]
            for u in (g.obj(x) for x in a[1]):
                if u is not None and any(u is r or (u.cname == r.copy and u.loc == r.loc) for r in refl):
                    v -= 8.0
                    break
        return v

    def shape(s, g, me):
        if g.winner is not None:
            return 0.0
        v = 0.0
        for u in g.units(me):
            if u.cname == "Karthus, Eternal":
                v += 1.5 if u.loc == "base" else -1.0
        # omega hold : de la might sur un battlefield qu'on tient
        for b in g.bfs:
            if b.ctrl == me:
                v += 0.25 * sum(max(0, g.might(u)) for u in g.units(me, b.idx))
        return v

    @staticmethod
    def _spare_karthus(g, pid):
        """Deux Karthus en main et un Glasc encore disponible (jeu, main ou deck) pour ramener celui qu'on jette."""
        pl = g.p[pid]
        if sum(c.cname == "Karthus, Eternal" for c in pl.hand) < 2:
            return False
        return (any(u.cname == "Glasc Mixologist" for u in g.units(pid))
                or any(c.cname == "Glasc Mixologist" for c in pl.hand + pl.deck))

    def choose(s, g, pid, kind, options, ctx):
        if kind == "may":
            it = ctx.get("item")
            if it is not None and it.name.startswith("LeBlanc, Deceiver"):
                # ne pas cloner si cela force à défausser une carte clé
                return (any(c.cname not in s.NEVER_DISCARD for c in g.p[pid].hand)
                        or (VIDEO and s._spare_karthus(g, pid)))
            return NotImplemented
        if kind == "discard" and ctx.get("reason") == "leblanc":
            big = any(g.might(u) >= 5 for u in g.units(pid))
            spare_k = VIDEO and s._spare_karthus(g, pid)
            def rank(c):
                n = c.cname
                if n == "Sacrifice" and big:
                    return 50
                if n == "Karthus, Eternal" and spare_k:
                    return 30                                 # « un Karthus est plus sûr en défausse » (Glasc le ramène)
                if n in s.NEVER_DISCARD:
                    return 100
                return s.DISCARD_ORDER.index(n) if n in s.DISCARD_ORDER else 40
            return min(options, key=rank)
        if kind == "copy_target":
            pref = (["Ruined Rex", "Thousand-Tailed Watcher"] + [n for n in s.COPY_PREF if n not in
                    ("Ruined Rex", "Thousand-Tailed Watcher")]) if VIDEO else s.COPY_PREF
            def rank(u):
                return pref.index(u.cname) if u.cname in pref else 50 - unit_value(g, u)
            return min(options, key=rank)
        return NotImplemented


class LeBlancHookTempo(LeBlancDeathknell):
    """Liste IQ #5 = version occidentale « tempo » construite autour de Baited Hook (riftbound.gg : « deck très
    proactif »). Hextech : faire marquer la première vraie unité, puis utiliser la copie pour un travail que
    l'adversaire peut difficilement gérer ; garder une main qui conteste tôt. Le moteur du Deceiver ne tourne
    que sur conquête ou tenue d'un battlefield, donc LeBlanc doit occuper les battlefields."""
    name = "Hook tempo"

    def mulligan(s, g, pid):
        out = super().mulligan(g, pid)
        h = g.p[pid].hand
        if not any(c.spec["type"] == "Unit" and c.spec["e"] <= 3 for c in h if c not in out):
            out += [c for c in sorted(h, key=lambda c: -c.spec["e"]) if c not in out and c.spec["e"] >= 5]
        return out[:2]

    def prior(s, g, me, a):
        v = super().prior(g, me, a)
        k = a[0]
        if k == "move" and a[2] in (0, 1):
            units = [g.obj(u) for u in a[1]]
            if all(u is not None and u.cname != "Karthus, Eternal" for u in units):
                v += 2.0 if g.p[me].turns <= 6 else 1.0       # conquérir pour déclencher la légende
        elif k == "act" and not isinstance(a[1], tuple):
            o = g.obj(a[1])
            t = g.obj(a[3].get("tg", (None,))[0]) if isinstance(a[3], dict) and a[3].get("tg") else None
            if o is not None and o.cname == "Baited Hook" and t is not None:
                if t.cname in DK or g.has_kw(t, "Temporary"):
                    v += 2.5                                  # échelle Hook sur un corps Deathknell ou un Reflet
                elif t.cname == "Karthus, Eternal":
                    v -= 6.0
        return v

    def shape(s, g, me):
        if g.winner is not None:
            return 0.0
        v = super().shape(g, me)
        for b in g.bfs:
            if b.ctrl == me and any(not g.has_kw(u, "Temporary") for u in g.units(me, b.idx)):
                v += 1.5                                      # tenue = nouveau Reflet au prochain tour
        return v


class LeBlancHookWindswept(LeBlancHookTempo):
    """Hook tempo qui présente toujours Windswept Hillock contre Akali (mesuré : −8 ± 3 pts pour Akali par rapport à
    Star Spring, surtout quand Akali commence ; fil « Agent manager », session 1)."""
    name = "Hook tempo Windswept"

    def battlefield(s, deck, first, opp_bf=None):
        return "Windswept Hillock" if "Windswept Hillock" in deck["battlefields"] else None


PLANS = {"Gorica": AkaliGorica, "Hook tempo": LeBlancHookTempo, "Hook tempo Windswept": LeBlancHookWindswept, "Gorica agressif": AkaliGoricaAggro, "Deathknell": LeBlancDeathknell}


# ---------------------------------------------------------------------------------------- agents
class PlanPolicy(PolicyAgent):
    """Politique de simulation qui fait les choix de résolution du plan (défausse, copie...)."""

    def __init__(s, plan=None):
        s.plan = plan

    def choose(s, g, pid, kind, options, ctx):
        if s.plan is not None:
            r = s.plan.choose(g, pid, kind, options, ctx)
            if r is not NotImplemented:
                return r
        return Heuristics.choose(s, g, pid, kind, options, ctx)


class PlanAgent(SearchAgent):
    def __init__(s, seed=0, plan=None, opp_plan=None, **kw):
        super().__init__(seed, **kw)
        s.plan = plan or Plan()
        s.opp_plan = opp_plan

    def mulligan(s, g, pid):
        r = s.plan.mulligan(g, pid)
        return super().mulligan(g, pid) if r is None else r

    def choose(s, g, pid, kind, options, ctx):
        r = s.plan.choose(g, pid, kind, options, ctx)
        if r is not NotImplemented:
            return r
        return super().choose(g, pid, kind, options, ctx)

    # un rollout de SearchAgent (ancienne recherche ou tirages communs) : politiques et valeur du plan
    def policies(s, me):
        ag = [None, None]
        ag[me] = PlanPolicy(s.plan)
        ag[1 - me] = PlanPolicy(s.opp_plan)
        for x in ag:
            x.cfg = s.cfg
        return ag

    def value(s, c, me):
        return evaluate(c, me, s.ev) + s.plan.shape(c, me)

    def prior_of(s, g, me, a):
        pr = s.plan.prior(g, me, a)
        if _ai.TEMPO and pr < 0 and danger(g, me):
            pr = 0.0                                   # à portée de défaite, les principes du plan ne tiennent plus
        return pr
