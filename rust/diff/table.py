"""Test différentiel de la table : les mêmes appels `train.<fn>(...)` au module Python et au moteur Rust
(`rust/target/release/table`), réponses JSON comparées octet pour octet (sauf `cards()`, dont l'ordre des clés vient
d'un ensemble Python : comparé comme objet).

Les coups du joueur sont tirés au hasard (graine du scénario) parmi les choix que la réponse propose : comme les deux
moteurs rendent le même JSON, ils reçoivent la même suite d'appels. Parties complètes, « Reprendre », questions en
cours de résolution, drapeau, journal de l'IA, match BO3, sideboard, éditeur de deck, mode replay, version anglaise.

    python3 rust/diff/table.py [graine_début] [nombre] [--ia random]

`--ia random` (défaut tant que l'IA n'est pas portée en Rust) : l'IA de la table est un RandomAgent des deux côtés et
aucun plan n'est utilisé (train.P.PlanAgent et train._plan_for remplacés ici, RB_TABLE_AI=random côté Rust).
Sans cette option : la vraie IA (PlanAgent, recherche du tour) des deux côtés.
"""
import json, os, random, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..")
sys.path.insert(0, os.path.join(ROOT, "riftbound", "engine"))
import train  # noqa: E402
import plans as P  # noqa: E402
import agents  # noqa: E402

args = [a for a in sys.argv[1:] if not a.startswith("--")]
START = int(args[0]) if args else 1
N = int(args[1]) if len(args) > 1 else 5
STUB = "--ia" not in sys.argv or sys.argv[sys.argv.index("--ia") + 1] == "random"
MAXC = int(os.environ.get("RB_DIFF_MAXC", "4000"))
env = dict(os.environ)
if STUB:
    train.P.PlanAgent = lambda seed, **kw: agents.RandomAgent(seed)
    train._plan_for = lambda deck: P.Plan()
    env["RB_TABLE_AI"] = "random"
RS = subprocess.Popen([os.path.join(ROOT, "rust", "target", "release", "table")], stdin=subprocess.PIPE,
                      stdout=subprocess.PIPE, env=env, text=True, encoding="utf-8")
STATS = dict(calls=0, diffs=0, errors_same=0)
KEYS = sorted(train.DECKS)


def py_call(f, a):
    try:
        r = getattr(train, f)(*a)
        return dict(r=r if isinstance(r, str) else json.dumps(r))
    except Exception as e:  # noqa: BLE001
        return dict(err=f"{type(e).__name__}: {e}")


def rs_call(f, a):
    RS.stdin.write(json.dumps([f, a], ensure_ascii=False) + "\n")
    RS.stdin.flush()
    return json.loads(RS.stdout.readline())


class Diff(Exception):
    pass


def call(f, *a, loose=False):
    """Même appel aux deux moteurs ; la réponse Python (texte JSON) si elles sont identiques."""
    STATS["calls"] += 1
    p, r = py_call(f, list(a)), rs_call(f, list(a))
    if "err" in p or "err" in r:
        if "err" in p and "err" in r:
            STATS["errors_same"] += 1
            return None
        raise Diff(f"{f}{list(a)!r:.200} : Python {p!r:.600}\n   Rust {r!r:.600}")
    if p["r"] != r["r"]:
        if loose:
            jp, jr = json.loads(p["r"]), json.loads(r["r"])
            if jp == jr:
                return p["r"]
            ks = sorted(set(jp) ^ set(jr)) or [k for k in jp if jp[k] != jr.get(k)][:3]
            raise Diff(f"{f} : clés différentes {ks} : Python {[jp.get(k) for k in ks]!r:.800} Rust {[jr.get(k) for k in ks]!r:.800}")
        a_, b_ = p["r"], r["r"]
        i = next((k for k in range(min(len(a_), len(b_))) if a_[k] != b_[k]), min(len(a_), len(b_)))
        raise Diff(f"{f}{list(a)!r:.200} : première différence au caractère {i}\n   Python …{a_[max(0, i - 300):i + 200]}\n"
                   f"   Rust   …{b_[max(0, i - 300):i + 200]}")
    return p["r"]


def play(rng, v, moves, limit):
    """Joue au hasard jusqu'à la fin (ou limit appels) ; moves : coups enregistrés comme la page (act / ans / undo)."""
    n = 0
    while v.get("winner") is None and n < limit:
        n += 1
        if v.get("ask"):
            a = v["ask"]
            k = len(a["options"])
            x = rng.sample(range(k), rng.randint(0, min(2, k))) if a["kind"] == "mulligan" else rng.randrange(k)
            moves.append(["ans", x])
            v = json.loads(call("answer", json.dumps(x)))
        elif v.get("dec"):
            r = rng.random()
            if r < 0.04 and v.get("undo"):
                moves.append(["undo"])
                v = json.loads(call("undo"))
                continue
            if r < 0.08:
                call("flag")
                call("ai_script")
            ops = v["dec"]["options"]
            i = rng.randrange(len(ops))
            moves.append(["act", i])
            v = json.loads(call("act", i))
        elif v.get("busy"):
            v = json.loads(call("step"))
        else:
            break
    return v


def scenario(seed):
    rng = random.Random(seed)
    lang = "en" if seed % 3 == 0 else "fr"
    call("set_lang", lang)
    mine = rng.choice(KEYS + ["akali-g2", None])
    opp = rng.choice(KEYS + ["leblanc-iq5", None])
    call("check", json.dumps(train._export(train._deck(mine, train.G2))))
    level = rng.choice([1, 3, 6])
    first = rng.choice([None, 0, 1])
    meta = json.loads(call("new", seed, None, first, level, mine, opp, None))
    call("cards", loose=True)
    moves = []
    v = json.loads(call("step"))
    v = play(rng, v, moves, MAXC)
    script = json.loads(call("ai_script"))
    # mode replay de la partie jouée
    game = dict(seed=seed, first=meta["first"], bf=meta["bf"][0], obf=meta["bf"][1], level=level, mine=mine, opp=opp,
                moves=moves, ai=script["ai"], result=dict(winner=v.get("winner"), pts=v["st"]["pts"]))
    rl = call("replay_load", json.dumps(game))
    if rl is not None:
        n = json.loads(rl)["n"]
        for k in sorted(set([0, n - 1] + [rng.randrange(n) for _ in range(5)])):
            call("replay_frame", k)
        call("replay_exit")
    # match BO3 : préparation des manches, enregistrement, sideboard
    st = call("match_new", rng.choice(["bo1", "bo3"]), seed, mine, opp)
    while st is not None and not json.loads(st).get("over"):
        nx = json.loads(call("match_next", st, mine, opp, None))
        call("match_duel_next", st)
        bfs = [nx["human_bf"] or json.loads(st)["bfs"][0][0], nx["ai_bf"]]
        st = call("match_record", st, nx["first"] if nx["first"] is not None else 0, json.dumps(bfs), rng.choice([0, 1, -1]))
    d = train._export(train._deck(mine, train.G2))
    if d["sideboard"]:
        call("match_swap", json.dumps(d), json.dumps(d["main"][:1]), json.dumps(d["sideboard"][:1]), None)
    call("duel_off")
    return v


def main():
    call("catalog")
    call("set_lang", "fr")
    ok = bad = 0
    for s in range(START, START + N):
        try:
            v = scenario(s)
            ok += 1
            print(f"graine {s} : identique (vainqueur {v.get('winner')}, tour {v['st']['t']})", flush=True)
        except Diff as e:
            bad += 1
            print(f"graine {s} : DIFFÉRENCE {e}", flush=True)
            RS.kill()
            return ok, bad
    return ok, bad


if __name__ == "__main__":
    ok, bad = main()
    print(f"scénarios identiques={ok} différents={bad} appels={STATS['calls']} erreurs identiques={STATS['errors_same']}")
    sys.exit(1 if bad else 0)
