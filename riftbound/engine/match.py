"""Match BO1 / BO3 entre le joueur (0) et l'IA (1), fidèle aux règles.

- Core Rules 485 (1v1 Duel, BO1) : chaque joueur tire AU HASARD un de ses 3 battlefields (485.5).
- Core Rules 486 (1v1 Match, BO3) : chaque joueur CHOISIT un de ses 3 battlefields, en même temps que l'autre (486.5) ;
  après une manche gagnée, les battlefields joués sont retirés pour le reste du match ; après une nulle ils peuvent
  resservir (486.5.a). Le match se gagne en 2 manches (486.6).
- Règles de tournoi 407 : manche 1, un joueur tiré au sort choisit de jouer en premier ou en second (407.1-407.2) ;
  ensuite le perdant de la manche précédente choisit (407.4) ; après une nulle, le même joueur recommence (407.4).
- Règles de tournoi 403 et 601.1.c : sideboard interdit en manche 1 (403.5) et après une nulle (403.10) ; échanges
  1 pour 1 (403.4), le Chosen Champion peut changer (601.1.c.4) ; runes, légende et battlefields fixes (403.4.b).

L'état est un dict JSON (la page le garde) ; ces fonctions sont pures. Choix de l'IA : jouer en premier ; battlefield
choisi par son plan (sinon au hasard) sans connaître celui du joueur (choix simultanés) ; l'IA ne sideboarde pas."""
import random

HUMAN, AI = 0, 1


def new(mode, seed, bfs):
    """bfs = [battlefields du joueur, battlefields de l'IA] (3 chacun)."""
    assert mode in ("bo1", "bo3")
    rng = random.Random(int(seed) * 7907 + 13)
    return dict(mode=mode, seed=int(seed), bfs=[list(bfs[0]), list(bfs[1])], wins=[0, 0], used=[[], []], history=[],
                roll=rng.randrange(2))          # gagnant du tirage au sort de la manche 1 (407.2)


def game_no(st):
    return len(st["history"]) + 1


def over(st):
    if st["mode"] == "bo1":
        return len(st["history"]) >= 1
    return max(st["wins"]) >= 2


def winner(st):
    if not over(st):
        return None
    if st["wins"][0] != st["wins"][1]:
        return 0 if st["wins"][0] > st["wins"][1] else 1
    return -1


def chooser(st):
    """Qui choisit de jouer en premier ou en second pour la manche à venir ; None = imposé (après une nulle)."""
    if not st["history"]:
        return st["roll"]
    w = st["history"][-1]["winner"]
    return (1 - w) if w in (0, 1) else None


def forced_first(st):
    """Premier joueur imposé après une nulle (407.4) ; None sinon."""
    if st["history"] and chooser(st) is None:
        return st["history"][-1]["first"]
    return None


def allowed_bfs(st, pid):
    return [b for b in st["bfs"][pid] if b not in st["used"][pid]]


def can_sideboard(st):
    return st["mode"] == "bo3" and bool(st["history"]) and st["history"][-1]["winner"] in (0, 1)


def seed_of(st):
    return st["seed"] * 10 + game_no(st)


def ai_first(st):
    """L'IA, quand elle choisit, joue en premier."""
    return AI


def pick_bfs(st, choose_ai_bf=None, human_bf=None):
    """Battlefields de la manche : BO1 au hasard pour les deux (485.5) ; BO3 choix (486.5), l'IA par choose_ai_bf(allowed)
    ou au hasard. Le choix de l'IA ne dépend pas de celui du joueur (simultanés)."""
    rng = random.Random(seed_of(st) * 31 + 5)
    if st["mode"] == "bo1":
        return [rng.choice(st["bfs"][0]), rng.choice(st["bfs"][1])]
    ok_ai = allowed_bfs(st, AI)
    b_ai = (choose_ai_bf(ok_ai) if choose_ai_bf else None) or rng.choice(ok_ai)
    if b_ai not in ok_ai:
        b_ai = rng.choice(ok_ai)
    ok_h = allowed_bfs(st, HUMAN)
    b_h = human_bf if human_bf in ok_h else rng.choice(ok_h)
    return [b_h, b_ai]


def record(st, first, bfs, win):
    """Enregistre une manche finie : premier joueur, battlefields joués, vainqueur (0, 1 ou -1 pour nulle)."""
    st = dict(st, wins=list(st["wins"]), used=[list(x) for x in st["used"]], history=list(st["history"]))
    st["history"].append(dict(game=game_no(st), first=int(first), bfs=list(bfs), winner=win))
    if win in (0, 1):
        st["wins"][win] += 1
        for p in (0, 1):
            st["used"][p].append(bfs[p])        # retirés pour le reste du match (486.5)
    return st


def sideboard_swap(deck, out_cards, in_cards, champion=None):
    """Échange 1 pour 1 (403.4) : out_cards quittent le Main Deck pour le sideboard, in_cards font l'inverse.
    Le Chosen Champion peut être changé pour une unité Champion du Main Deck ou du sideboard (601.1.c.4)."""
    if len(out_cards) != len(in_cards):
        raise ValueError("échange 1 pour 1 : autant de cartes sortent que de cartes entrent (règle 403.4)")
    main, side = list(deck["main"]), list(deck.get("sideboard") or [])
    for n in out_cards:
        main.remove(n)
    for n in in_cards:
        side.remove(n)
    main += list(in_cards)
    side += list(out_cards)
    d = dict(deck, main=main, sideboard=side)
    if champion and champion != deck.get("champion"):
        pool = main + side
        if champion not in pool:
            raise ValueError("le nouveau Chosen Champion doit venir du Main Deck ou du sideboard (601.1.c.4)")
        (main if champion in main else side).remove(champion)
        main.append(deck["champion"])               # l'ancien champion retourne dans le Main Deck
        d.update(main=main, sideboard=side, champion=champion)
    return d
