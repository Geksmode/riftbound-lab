"""Tests générés depuis le texte imprimé de TOUS les sorts modélisés (riftbound/cards/cards_unique.csv, colonne text).

Classement automatique du texte (sans les rappels entre parenthèses) :
- « obligatoire » : un verbe d'effet suivi de « a / an / another ... unit » (Kill a unit, Deal 3 to an enemy unit...) ;
- « facultatif » : « up to N » ou « any number of » (règle 355.13).
Contrôles, pour chaque sort :
1. plateau vide : un sort à cible obligatoire n'est PAS jouable (355.8 : il faut un choix valide pour toutes les cibles) ;
   un sort à cible facultative EST jouable ;
2. avec des unités partout : un sort « up to / any number » propose toujours le choix sans cible (355.13) ;
3. le choix sans cible se résout sans erreur et la carte quitte la main (la suite de l'effet s'applique).
Un sort que le classement lit mal va dans EXCEPTIONS avec la raison : une nouvelle carte mal classée fait échouer le
test tant que personne n'a vérifié son texte. Run: python3 cardsets/test_texte_auto.py"""
import csv, os, re, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
from actions import card_choices                        # noqa: E402

T = Suite("texte_auto")
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ALL = ["Fury", "Calm", "Mind", "Body", "Chaos", "Order"] * 2
VERB = r"(stun|kill|give|move|ready|exhaust|return|recall|buff|banish|choose|heal|copy|deal \d+ (?:damage )?to)"
MAND = re.compile(VERB + r"\s+(a|an|another)\s+(?:[\w-]+\s+){0,3}?units?\b")
OPT = re.compile(r"up to (?:one|two|three|four|\d+)|any number of")

# Sorts que le classement lit mal : le moteur suit le texte, vérifié à la main (2026-10-08).
EXCEPTIONS = {
    "Meditation": "« you may exhaust a friendly unit » est un coût additionnel facultatif : jouable sans unité",
    "Reinforce": "« banish a unit from among them » vise les cartes regardées, pas le plateau",
    "Wild Claw": "« banish a unit or gear from among them » vise les cartes regardées, pas le plateau",
    "Whirlwind": "« each player may return a unit » : facultatif pour chaque joueur",
    "Bone Skewer": "cible un battlefield ; « choose a unit from it » vise la main adverse",
    "Curtain Call": "sort modal : le mode « Draw 1 » n'a pas de cible",
    "Piercing Light": "« Deal 2 to a unit at a battlefield » est obligatoire ; seule la 2e cible est « up to »",
    "Moonfall": "« Choose a battlefield where you have units » : injouable sans unité, malgré « up to one enemy unit »",
    "Deathgrip": "À TRANCHER par l'utilisateur : le moteur le laisse jouer sans unité (juste « Draw 1 ») ; lecture stricte de "
                 "355.7-355.8 : « Kill a friendly unit » et « another friendly unit » sont deux cibles obligatoires",
}


def spells():
    rows = csv.DictReader(open(os.path.join(ROOT, "cards", "cards_unique.csv"), encoding="utf-8"))
    return sorted((r["name"], r["text"]) for r in rows if r["type"] == "Spell" and r["name"] in IMPL)


def classify(text):
    t = re.sub(r"\[[^\]]*\]", "", re.sub(r"\([^)]*\)", "", text.lower()))
    return bool(MAND.search(t)), bool(OPT.search(t))


def board(units):
    g, _ = new()
    runes(g, 0, ALL)
    if units:
        for pid in (0, 1):
            for loc in ("base", 0, 1):
                put(g, pid, "Mournful Witness", loc)
    return g


def chs(g, name):
    c = hand(g, 0, name)
    return c, card_choices(g, 0, c, "hand", False, False, every=True)


SPELLS = spells()


@T.test
def text_and_engine_agree_on_an_empty_board():
    bad = []
    for name, text in SPELLS:
        mand, opt_ = classify(text)
        if name in EXCEPTIONS or not (mand or opt_):
            continue
        _, ch = chs(board(False), name)
        if opt_ and not ch:
            bad.append(f"{name} : « up to / any number » mais injouable sur plateau vide")
        elif mand and not opt_ and ch:
            bad.append(f"{name} : cible obligatoire mais jouable sur plateau vide ({len(ch)} choix)")
    assert not bad, "\n" + "\n".join(bad)


@T.test
def optional_targets_always_offer_zero_targets():
    bad = []
    for name, text in SPELLS:
        mand, opt_ = classify(text)
        if not opt_ or name in EXCEPTIONS:
            continue
        _, ch = chs(board(True), name)
        if ch and not any(not c.get("tg") and not c.get("tg2") for c in ch):
            bad.append(f"{name} : aucun choix sans cible alors que des cibles existent (355.13)")
    assert not bad, "\n" + "\n".join(bad)


@T.test
def zero_target_choice_resolves():
    bad, n = [], 0
    for name, text in SPELLS:
        mand, opt_ = classify(text)
        if not opt_ or name in EXCEPTIONS:
            continue
        g = board(False)
        c, ch = chs(g, name)
        zero = [x for x in ch if not x.get("tg")]
        if not zero:
            continue
        try:
            g.apply(opt(g, 0, name, lambda x: x == zero[0]))
            settle(g)
        except Exception as e:                            # noqa: BLE001
            bad.append(f"{name} : {type(e).__name__} {e}")
            continue
        n += 1
        if c in g.p[0].hand:
            bad.append(f"{name} : la carte est restée en main")
    assert n > 0 and not bad, "\n" + "\n".join(bad)


UNITS = sorted((r["name"], r["text"]) for r in csv.DictReader(open(os.path.join(ROOT, "cards", "cards_unique.csv"), encoding="utf-8"))
               if r["type"] == "Unit" and r["name"] in IMPL and OPT.search(re.sub(r"\([^)]*\)", "", r["text"].lower())))


@T.test
def optional_unit_effects_resolve_with_no_target():
    """Unités « up to / any number » jouées sur un plateau vide : l'effet se résout sans erreur, l'unité est en jeu."""
    bad, n = [], 0
    for name, _ in UNITS:
        g = board(False)
        c = hand(g, 0, name)
        ch = [x for x in card_choices(g, 0, c, "hand", False, False, every=True) if x.get("loc") == "base"]
        if not ch:
            continue
        try:
            g.apply(opt(g, 0, name, lambda x: x == ch[0]))
            settle(g)
        except Exception as e:                            # noqa: BLE001
            bad.append(f"{name} : {type(e).__name__} {e}")
            continue
        n += 1
        if not any(u.cname == name for u in g.units(0)):
            bad.append(f"{name} : l'unité n'est pas en jeu")
    assert n >= len(UNITS) // 2 and not bad, f"{n}/{len(UNITS)} jouées\n" + "\n".join(bad)


@T.test
def every_exception_is_a_real_modelled_spell():
    names = {n for n, _ in SPELLS}
    assert set(EXCEPTIONS) <= names, set(EXCEPTIONS) - names


if __name__ == "__main__":
    T.main()
