"""Version anglaise de la table (lang.py) : chaque question a un titre anglais, une partie jouée en anglais ne montre
aucun texte français (questions, choix, coups, journal, conseil, erreurs de deck), et la langue ne change pas la partie
(même graine, mêmes choix : même partie en français et en anglais).
Run: python3 cardsets/test_lang.py"""
import json, os, random, re, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *                                   # noqa: E402,F403
import train                                            # noqa: E402
import cards as _cards                                  # noqa: E402
import lang                                             # noqa: E402
from game import SPEC                                   # noqa: E402

T = Suite("lang")
# Mots et lettres qui trahissent du français une fois les noms de cartes retirés (les cartes ont des noms anglais).
FR = re.compile(r"[àâçéèêëîïôûùœ«»]|\b(le|la|les|des|du|une|et|ou|ton|ta|tes|ses|pour|avec|sans|sur|sous|dans|vers|"
                r"cible|carte|cartes|unité|unités|joue|déplace|pioche|tour|défausse|aucun|aucune|quelle|quel|toi|"
                r"adverse|passe|termine|énergie|gratuit|oui)\b", re.I)
NAMES = sorted((n for n in SPEC), key=len, reverse=True)
NAME_RE = re.compile("|".join(re.escape(n) for n in NAMES))


def french(s):
    s = NAME_RE.sub("X", str(s))
    m = FR.search(s)
    return m.group(0) if m else None


def rdeck(rng, cat):
    L = [c for c in cat if c["t"] == "Legend" and c["m"]]
    BF = [c["n"] for c in cat if c["t"] == "Battlefield" and c["m"] and not c["ban"]]
    for _ in range(50):
        lg = rng.choice(L)
        dom = set(lg["d"])
        ch = [c for c in cat if c["t"] == "Unit" and c["s"] == "Champion" and c["m"] and set(c["tg"]) & set(lg["tg"])
              and set(c["d"]) <= dom]
        pool = [c["n"] for c in cat if c["t"] in ("Unit", "Spell", "Gear") and c["m"] and set(c["d"]) <= dom
                and c["s"] not in ("Champion", "Signature") and not c["ban"] and "Unique" not in c["tx"]]
        if not ch or len(pool) < 14:
            continue
        main = []
        while len(main) < 39:
            n = rng.choice(pool)
            if main.count(n) < 3:
                main.append(n)
        d = dict(name="r", legend=lg["n"], champion=rng.choice(ch)["n"], main=main,
                 runes=[lg["d"][0] + " Rune"] * 6 + [lg["d"][-1] + " Rune"] * 6, battlefields=rng.sample(BF, 3))
        if not [x for x in json.loads(train.check(json.dumps(d))) if not x.startswith("⚠")]:
            return d
    raise RuntimeError("pas de deck")


def play(seed, a, b, texts, max_steps=600):
    """Partie à choix tirés au hasard (graine seed) ; renvoie la suite des états vus, ajoute les textes montrés."""
    rng = random.Random(seed)
    json.loads(train.new(seed, None, None, 0, json.dumps(a), json.dumps(b)))
    v = json.loads(train.step())
    seen, n = [], 0
    while v.get("winner") is None and n < max_steps:
        n += 1
        seen.append(json.dumps(v["st"], sort_keys=True))
        texts += [t for _, t in v.get("log", [])] + ([v["ai"]] if v.get("ai") else [])
        if v.get("ask"):
            texts += [v["ask"]["title"]] + list(v["ask"]["options"])
            k = v["ask"]["kind"]
            x = [] if k == "mulligan" else rng.randrange(len(v["ask"]["options"]))
            v = json.loads(train.answer(json.dumps(x)))
        elif v.get("dec"):
            texts += [o["label"] for o in v["dec"]["options"]]
            if n % 25 == 0:
                texts += [h["label"] for h in json.loads(train.hint(3))] + [str(h["gap"]) for h in json.loads(train.hint(3))]
            v = json.loads(train.act(rng.randrange(len(v["dec"]["options"]))))
        else:
            v = json.loads(train.step())
    seen.append(json.dumps(v["st"], sort_keys=True))
    return seen


@T.test
def every_question_has_an_english_title():
    missing = sorted(k for k in set(_cards.ASK_TEXT) | set(train.ASKS) if k not in lang.ASK_EN)
    assert not missing, f"titres anglais manquants dans lang.ASK_EN : {missing}"
    bad = {k: v for k, v in lang.ASK_EN.items() if french(v)}
    assert not bad, bad


@T.test
def english_games_show_no_french_and_play_the_same_game():
    cat = json.loads(train.catalog())["cards"]
    try:
        for seed in (3, 8, 21):
            rng = random.Random(seed)
            a, b = rdeck(rng, cat), rdeck(rng, cat)
            lang.set_lang("fr")
            fr_states = play(seed, a, b, [])
            lang.set_lang("en")
            texts = []
            en_states = play(seed, a, b, texts)
            assert fr_states == en_states, f"graine {seed} : la langue a changé la partie"
            bad = sorted({(french(t), t) for t in texts if french(t)})
            assert not bad, f"graine {seed} ({a['legend']} / {b['legend']}) : {bad[:12]}"
    finally:
        lang.set_lang("fr")


@T.test
def deck_errors_in_english():
    try:
        lang.set_lang("en")
        errs = json.loads(train.check(json.dumps(dict(legend=None, champion=None, main=["Nope"] * 5, runes=[],
                                                      battlefields=["Nope"]))))
        assert errs and not [e for e in errs if french(e)], errs
    finally:
        lang.set_lang("fr")


if __name__ == "__main__":
    T.main()
