#!/usr/bin/env python3
"""Puzzles de la table (menu « Puzzles ») : une position de départ montée dans le VRAI moteur (game.Game, toutes les
cartes, aucune règle simplifiée), un objectif vérifié par le moteur à la fin d'un tour, l'IA de la table joue
l'adversaire.

Positions reprises des puzzles Akali (Heron de Gorica) contre Master Yi, Wuju Bladesman de riftbound/puzzles/puzzles.js
(2026-10-04), qui tournaient sur un mini-moteur JS simplifié ; ici rien n'est simplifié, et chaque solution est rejouée
contre la vraie IA par cardsets/test_puzzles.py (et chaque piège perd).

Une entrée de PUZZLES :
  id, titre / tag / brief / but / solution / leçon : textes (fr, en) ;
  bfs : [(nom, contrôleur)] pour le battlefield 0 (le tien) et 1 (celui de Yi) ; contrôleur 0 = toi, 1 = Yi, None ;
  me / op : unités [(nom, lieu, prête, options)] (lieu "base" ou 0 / 1), main, runes [(domaine, prête)] ;
  hidden : cartes cachées [(joueur, nom, battlefield)] (cachées au tour précédent : jouables tout de suite, 421) ;
  start : joueur qui a le tour (0 = toi) ; fin : nombre de fins de tour avant de juger (1 = fin de ce tour, 2 = fin du
  tour suivant) ;
  op_runes : runes du dessus du deck de runes de Yi (canalisées à son tour, 430) ;
  forced : premiers déplacements imposés à Yi ([noms des unités], destination) : c'est la situation donnée par
  l'énoncé (« il attaque Sigil avec Master Yi »), pas une règle ;
  goal(g, start_uids) -> bool : l'objectif, lu sur l'état réel ;
  sol : la solution jouée par le test (suite de coups décrits par des fonctions de choix, voir test_puzzles) ;
  traps : coups qui doivent perdre contre l'IA.

Pioches : le dessus de chaque deck est rempli des cartes les plus chères du deck (on ne pioche pas une réponse gratuite
pendant le puzzle) ; le reste du deck est mélangé avec la graine du puzzle (même puzzle = même partie). Les champions
ne sont pas dans la zone des champions (positions d'origine sans champion)."""
from lang import tr

SIGIL, GROVE = "Sigil of the Storm", "Grove of the God-Willow"
ME, OP = 0, 1
DECKS = ("akali-g2", "yi-bladesman_swagalisk_rq-la_6th")


def _ctrl(g, b):
    return g.bfs[b].ctrl


def _alive(g, name, pid=ME):
    return [o for o in g.board if o.ctrl == pid and o.name == name]


def _heron_on_mine(g, _):
    return any(isinstance(o.loc, int) and g.bfs[o.loc].ctrl == ME for o in _alive(g, "Astral Heron"))


def _none_lost(g, start):
    on = {o.uid for o in g.board}
    return all(u in on for u in start)


PUZZLES = [
    dict(
        id="flip", seed=11,
        title=("Tuer avant le combat", "Kill before combat"),
        tag=("Prendre un battlefield", "Take a battlefield"),
        brief=("Prends Grove of the God-Willow ce tour-ci. Tu sais que Yi a Punch First en main.",
               "Take Grove of the God-Willow this turn. You know Yi has Punch First in hand."),
        goal_text=("Tu contrôles Grove of the God-Willow à la fin de ton tour.",
                   "You control Grove of the God-Willow at the end of your turn."),
        solution=("Shuriken Flip sur le Pit Rookie (2 dégâts), et le déplacement du sort envoie Kai'Sa sur Grove.",
                  "Shuriken Flip on Pit Rookie (2 damage), and the spell's move sends Kai'Sa to Grove."),
        lesson=("Hors combat, le Rookie n'a que 2 de puissance : la légende de Yi ne compte qu'en défense. En réponse à "
                "ton sort, Yi ne peut jouer que des Reactions : Punch First est une Action, elle ne sort qu'à son tour ou "
                "pendant un showdown. Le Rookie meurt, Grove est vide, Kai'Sa conquiert et pioche. Si tu attaques "
                "directement, le Rookie vaut 4 en défense et Punch First le monte à 9.",
                "Outside combat the Rookie only has 2 might: Yi's legend only counts on defense. In response to your "
                "spell Yi can only play Reactions: Punch First is an Action, it only comes out on his turn or during a "
                "showdown. The Rookie dies, Grove is empty, Kai'Sa conquers and draws. If you attack directly, the "
                "Rookie is worth 4 on defense and Punch First makes it 9."),
        bfs=[(SIGIL, ME), (GROVE, OP)],
        me=dict(units=[("Mournful Witness", 0, False), ("Kai'Sa, Survivor", "base", True)], hand=["Shuriken Flip"],
                runes=[("Fury", True), ("Calm", True)]),
        op=dict(units=[("Pit Rookie", 1, True)], hand=["Punch First"],
                runes=[("Body", True), ("Body", True), ("Calm", True)]),
        start=ME, fin=1,
        goal=lambda g, s: _ctrl(g, 1) == ME,
    ),
    dict(
        id="count", seed=12,
        title=("Compter son défenseur", "Count the defender"),
        tag=("Compter", "Counting"),
        brief=("Prends Grove of the God-Willow ce tour-ci. Yi a En Garde en main et une rune Calm ouverte.",
               "Take Grove of the God-Willow this turn. Yi has En Garde in hand and one Calm rune open."),
        goal_text=("Tu contrôles Grove of the God-Willow à la fin de ton tour.",
                   "You control Grove of the God-Willow at the end of your turn."),
        solution=("Attaquer avec Kai'Sa et Mournful Witness ensemble, et garder Discipline pour répondre à En Garde.",
                  "Attack with Kai'Sa and Mournful Witness together, and keep Discipline to answer En Garde."),
        lesson=("First Mate défend seul : 3 + 2 = 5. Avec En Garde (+1, +1 s'il est seul) il monte à 7. Kai'Sa + Witness "
                "font 6 : il faut Discipline pour passer à 8. Avant chaque attaque, compte : puissance imprimée + 2 + le "
                "meilleur tour qu'il peut payer.",
                "First Mate defends alone: 3 + 2 = 5. With En Garde (+1, +1 more if alone) it reaches 7. Kai'Sa + Witness "
                "make 6: you need Discipline to reach 8. Before every attack, count: printed might + 2 + the best trick "
                "he can pay for."),
        bfs=[(SIGIL, None), (GROVE, OP)],
        me=dict(units=[("Kai'Sa, Survivor", "base", True), ("Mournful Witness", "base", True)], hand=["Discipline"],
                runes=[("Calm", True), ("Calm", True)]),
        op=dict(units=[("First Mate", 1, True)], hand=["En Garde"], runes=[("Calm", True)]),
        start=ME, fin=1,
        goal=lambda g, s: _ctrl(g, 1) == ME,
    ),
    dict(
        id="heron", seed=13,
        title=("Installer le Héron", "Set up the Heron"),
        tag=("Plan Héron", "Heron plan"),
        brief=("Pose l'Astral Heron pour qu'il soit encore sur un battlefield à toi à la fin du tour de Yi. À son tour, "
               "Yi aura 2 runes (Calm et Body) et Charm en main.",
               "Put down the Astral Heron so that it is still on a battlefield you control at the end of Yi's turn. On "
               "his turn Yi will have 2 runes (Calm and Body) and Charm in hand."),
        goal_text=("À la fin du tour de Yi, l'Astral Heron est vivant, sur un battlefield que tu contrôles.",
                   "At the end of Yi's turn, the Astral Heron is alive, on a battlefield you control."),
        solution=("Jouer l'Astral Heron directement sur Sigil of the Storm (tu le contrôles), puis finir le tour avec "
                  "Defy et une rune Calm ouverte. À son tour, Defy contre Charm.",
                  "Play the Astral Heron straight onto Sigil of the Storm (you control it), then end your turn with Defy "
                  "and a Calm rune open. On his turn, Defy counters Charm."),
        lesson=("Règle 355.2 : une unité se joue dans ta base ou sur un battlefield que tu contrôles. Le Héron ne marche "
                "pas vers un battlefield, il s'y pose. Charm ne se joue qu'au tour de Yi, et il passe sous Defy "
                "(1 énergie, 1 rune) : garde Defy ouvert à la fin de ton tour.",
                "Rule 355.2: a unit is played to your base or to a battlefield you control. The Heron does not walk to a "
                "battlefield, it lands there. Charm is only played on Yi's turn, and it falls under Defy (1 energy, "
                "1 rune): keep Defy open at the end of your turn."),
        bfs=[(SIGIL, ME), (GROVE, OP)],
        me=dict(units=[("Mournful Witness", 0, False)], hand=["Astral Heron", "Mournful Witness", "Defy"],
                runes=[("Calm", True)] * 4 + [("Fury", True)] * 4),
        op=dict(units=[("Master Yi, Tempered", 1, True)], hand=["Charm"], runes=[]),
        op_runes=["Calm", "Body"],
        start=ME, fin=2,
        goal=_heron_on_mine,
    ),
    dict(
        id="block", seed=14,
        title=("Il attaque ton Héron", "He attacks your Heron"),
        tag=("Défendre", "Defending"),
        brief=("C'est le tour de Yi. Il attaque Sigil of the Storm avec Master Yi. Garde ton Astral Heron en vie. Tu as "
               "Block caché sur Sigil.",
               "It is Yi's turn. He attacks Sigil of the Storm with Master Yi. Keep your Astral Heron alive. You have "
               "Block hidden at Sigil."),
        goal_text=("L'Astral Heron est encore en vie à la fin du tour de Yi.",
                   "The Astral Heron is still alive at the end of Yi's turn."),
        solution=("Quand Yi joue Punch First (Yi à 9), révéler Block sur le Héron : Shield 3 le monte à 10. Il survit et "
                  "tue Yi.",
                  "When Yi plays Punch First (Yi at 9), reveal Block on the Heron: Shield 3 takes it to 10. It survives "
                  "and kills Yi."),
        lesson=("Punch First coûte 2 runes : Defy ne peut pas le contrer. Discipline ne suffit pas (9 contre 9, le Héron "
                "meurt). Block se joue depuis la face cachée sans énergie. Et la légende de Yi ne l'aide pas quand c'est "
                "lui qui attaque.",
                "Punch First costs 2 runes: Defy cannot counter it. Discipline is not enough (9 against 9, the Heron "
                "dies). Block is played from facedown for no energy. And Yi's legend does not help him when he is the "
                "one attacking."),
        bfs=[(SIGIL, ME), (GROVE, None)],
        hidden=[(ME, "Block", 0)],
        me=dict(units=[("Astral Heron", 0, True)], hand=["Discipline", "Defy"], runes=[("Calm", True), ("Calm", True)]),
        op=dict(units=[("Master Yi, Tempered", "base", True)], hand=["Punch First"],
                runes=[("Body", True), ("Body", True), ("Calm", True)]),
        forced=[(["Master Yi, Tempered"], 0)],
        start=OP, fin=1,
        goal=lambda g, s: bool(_alive(g, "Astral Heron")),
    ),
    dict(
        id="runner", seed=15,
        title=("Ruin Runner", "Ruin Runner"),
        tag=("Connaître sa liste", "Know your list"),
        brief=("Prends Grove of the God-Willow sans perdre ton Astral Heron.",
               "Take Grove of the God-Willow without losing your Astral Heron."),
        goal_text=("Tu contrôles Grove of the God-Willow et ton Astral Heron est vivant à la fin de ton tour.",
                   "You control Grove of the God-Willow and your Astral Heron is alive at the end of your turn."),
        solution=("Block sur Mournful Witness (Tank), puis attaquer avec Witness et le Héron. Ruin Runner doit mettre ses "
                  "dégâts sur Witness d'abord : 2 sur Witness, 5 sur le Héron qui survit.",
                  "Block on Mournful Witness (Tank), then attack with Witness and the Heron. Ruin Runner must put its "
                  "damage on Witness first: 2 on Witness, 5 on the Heron, which survives."),
        lesson=("Ruin Runner ne peut pas être choisi par tes sorts : Falling Star et Back Off ne le touchent pas. Seul le "
                "combat le tue, et il vaut 7 en défense seul. Tank oblige l'adversaire à mettre ses dégâts sur cette "
                "unité en premier : c'est comme ça qu'on protège une grosse unité dans un combat.",
                "Ruin Runner can't be chosen by your spells: Falling Star and Back Off don't touch it. Only combat kills "
                "it, and it is worth 7 defending alone. Tank forces the opponent to put damage on that unit first: "
                "that is how you protect a big unit in a combat."),
        bfs=[(SIGIL, None), (GROVE, OP)],
        me=dict(units=[("Astral Heron", "base", True), ("Mournful Witness", "base", True)],
                hand=["Falling Star", "Back Off", "Block"],
                runes=[("Fury", True), ("Fury", True), ("Calm", True), ("Calm", True)]),
        op=dict(units=[("Ruin Runner", 1, True)], hand=[], runes=[]),
        start=ME, fin=1,
        goal=lambda g, s: _ctrl(g, 1) == ME and bool(_alive(g, "Astral Heron")),
    ),
    dict(
        id="decree", seed=16,
        title=("Manche 2 : Decree of Focus", "Game 2: Decree of Focus"),
        tag=("Sideboard", "Sideboard"),
        brief=("Manche 2, Yi a sidé Decree of Focus. Prends Grove of the God-Willow sans perdre d'unité. Yi a une rune "
               "Calm ouverte, Decree of Focus et En Garde en main.",
               "Game 2, Yi sided in Decree of Focus. Take Grove of the God-Willow without losing a unit. Yi has one Calm "
               "rune open, Decree of Focus and En Garde in hand."),
        goal_text=("Tu contrôles Grove of the God-Willow et tu n'as perdu aucune unité à la fin de ton tour.",
                   "You control Grove of the God-Willow and you have lost no unit at the end of your turn."),
        solution=("Attaquer avec l'Astral Heron seul. Autre ligne : attaquer avec le Héron et Witness, puis ramener "
                  "Witness en base avec la légende d'Akali avant les dégâts.",
                  "Attack with the Astral Heron alone. Other line: attack with the Heron and Witness, then bring Witness "
                  "back to base with Akali's legend before damage."),
        lesson=("Decree of Focus donne +4 à son unité en combat contre une unité Fury (Kai'Sa). Contre le Héron seul "
                "(Calm), son meilleur tour est En Garde : Rookie 2 + 2 + 2 = 6, le Héron (7) survit et le tue. Dès que "
                "Kai'Sa vient, le Rookie passe à 8. Avec Witness en plus, ses 6 dégâts tuent Witness, sauf si tu la sors "
                "du combat avec ta légende (une fois par tour, pendant un showdown à ton tour).",
                "Decree of Focus gives +4 to its unit in combat against a Fury unit (Kai'Sa). Against the Heron alone "
                "(Calm), his best trick is En Garde: Rookie 2 + 2 + 2 = 6, the Heron (7) survives and kills it. As soon "
                "as Kai'Sa comes, the Rookie reaches 8. With Witness too, its 6 damage kill Witness, unless you take her "
                "out of the combat with your legend (once per turn, during a showdown on your turn)."),
        bfs=[(SIGIL, None), (GROVE, OP)],
        me=dict(units=[("Astral Heron", "base", True), ("Mournful Witness", "base", True, dict(emp=True)),
                       ("Kai'Sa, Survivor", "base", True)], hand=[], runes=[]),
        op=dict(units=[("Pit Rookie", 1, True)], hand=["Decree of Focus", "En Garde"], runes=[("Calm", True)]),
        start=ME, fin=1,
        goal=lambda g, s: _ctrl(g, 1) == ME and _none_lost(g, s),
    ),
]
BY_ID = {p["id"]: p for p in PUZZLES}


def listing():
    """Liste pour le menu (textes dans la langue de la table)."""
    return [dict(id=p["id"], title=tr(*p["title"]), tag=tr(*p["tag"]), brief=tr(*p["brief"]),
                 goal=tr(*p["goal_text"])) for p in PUZZLES]


def texts(p):
    return dict(id=p["id"], title=tr(*p["title"]), tag=tr(*p["tag"]), brief=tr(*p["brief"]),
                goal=tr(*p["goal_text"]), solution=tr(*p["solution"]), lesson=tr(*p["lesson"]))


def card_names(p):
    """Toutes les cartes nommées par la position (pour les images de la table)."""
    out = {n for n, _ in p["bfs"]} | {n for _, n, _ in p.get("hidden", ())}
    for side in (p["me"], p["op"]):
        out |= {u[0] for u in side["units"]} | set(side["hand"])
    return out


def _top_expensive(deck, n=6):
    """Les n cartes les plus chères du deck (énergie puis puissance, puis nom) : posées sur le dessus."""
    from game import SPEC
    return sorted(deck, key=lambda o: (-SPEC[o.name]["e"], -(SPEC[o.name].get("p") or 0), o.name))[:n]


def setup(g, p):
    """Monte la position du puzzle p dans la partie g (créée mais pas commencée). Renvoie les uids des unités de départ
    du joueur (pour « aucune unité perdue »)."""
    from game import Obj, Rune
    g.first = p["start"]
    g.tp = p["start"]
    g.turn_no = 6
    for pl in g.p:
        pl.turns = 3
        pl.points = 2
        pl.champ = []
        pl.hand = []
        g.rng.shuffle(pl.deck)
        top = _top_expensive(pl.deck)
        pl.deck = top + [c for c in pl.deck if c not in top]
    g.reset_hist()
    for b, (name, ctrl) in zip(g.bfs, p["bfs"]):
        assert b.name == name, (b.name, name)
        b.ctrl = ctrl
    for pid, side in ((ME, p["me"]), (OP, p["op"])):
        pl = g.p[pid]
        for u in side["units"]:
            name, loc, ready = u[:3]
            opt = u[3] if len(u) > 3 else {}
            o = Obj(name, pid)
            g.enter_board(o, pid, loc, ready=ready)
            o.entered_turn = g.turn_no - 1
            if opt.get("emp"):
                o.empowered = True
        for n in side["hand"]:
            c = Obj(n, pid)
            c.zone = "hand"
            pl.hand.append(c)
        rest = list(pl.rune_deck)
        pl.runes = []
        for dom, ready in side["runes"]:
            r = next((x for x in rest if x.domain == dom), None)
            if r is None:
                r = Rune(dom, pid)
            else:
                rest.remove(r)
            r.exhausted = not ready
            pl.runes.append(r)
        if pid == OP and p.get("op_runes"):
            first = []
            for dom in p["op_runes"]:
                r = next((x for x in rest if x.domain == dom), None) or Rune(dom, pid)
                if r in rest:
                    rest.remove(r)
                first.append(r)
            rest = first + rest
        pl.rune_deck = rest
    for pid, name, bf in p.get("hidden", ()):
        c = Obj(name, pid)
        c.zone = "facedown"
        c.hidden_turn = g.turn_no - 1
        c.hidden_bf = bf
        g.bfs[bf].facedowns.append(c)
    g.stage = "main_start"                    # vide les réserves et émet main_start, puis phase principale
    g.puz = dict(id=p["id"], end=g.turn_no + p["fin"] - 1)
    return [o.uid for o in g.board if o.ctrl == ME]


def judge(g, start):
    p = BY_ID[g.puz["id"]]
    return bool(p["goal"](g, start))
