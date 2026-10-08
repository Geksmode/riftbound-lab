"""Outils de résultats : version du moteur (version.py) et règles de comparaison (compare.py).
Run: python3 cardsets/test_resultats.py"""
import hashlib, io, json, os, sys, tempfile, contextlib
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import Suite                                # noqa: E402
import version, compare                                  # noqa: E402

T = Suite("resultats")


def res(seeds, scores, md5="v1", label="x"):
    return dict(label=label, engine_md5=md5, per_seed=[[s, w] for s, w in zip(seeds, scores)], _nom=label)


def code(fn, *a):
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        try:
            fn(*a)
            return 0, out.getvalue()
        except SystemExit as e:
            return e.code, out.getvalue()


@T.test
def engine_md5_same_algorithm_as_manager():
    h = hashlib.md5()
    root = version.HERE
    for f in sorted(root.glob("*.py")) + sorted((root / "cardsets").glob("*.py")):
        h.update(f.relative_to(root).as_posix().encode() + f.read_text(encoding="utf-8").encode())
    assert version.engine_md5() == h.hexdigest()[:10]


@T.test
def stamp_adds_version_and_seed_summary():
    e = version.stamp(dict(per_seed=[[7, 1.0], [5, 0.0], [6, 0.5]]))
    assert e["engine_md5"] == version.engine_md5() and e["seeds"] == dict(first=5, last=7, n=3)
    e = version.stamp(dict(per_pair=[[100, 0.5], [101, 1.0]]))
    assert e["seeds"]["n"] == 2


@T.test
def paired_when_same_seeds():
    c, out = code(compare.compare, res([1, 2, 3, 4], [1, 1, 1, 0]), res([1, 2, 3, 4], [0, 1, 0, 0]))
    assert c == 0 and "apparié sur 4" in out and "+50.0" in out


@T.test
def independent_when_disjoint_seeds():
    c, out = code(compare.compare, res([1, 2], [1, 0]), res([3, 4], [0, 0]))
    assert c == 0 and "indépendant" in out


@T.test
def refuses_partial_overlap():
    c, out = code(compare.compare, res([1, 2, 3], [1, 0, 1]), res([3, 4, 5], [0, 0, 1]))
    assert c == compare.REFUS and "en partie communes" in out


@T.test
def refuses_two_engine_versions():
    c, out = code(compare.compare, res([1, 2], [1, 0], "v1"), res([1, 2], [0, 0], "v2"))
    assert c == compare.REFUS and "versions du moteur" in out


@T.test
def refuses_result_without_version():
    a = res([1, 2], [1, 0]); a.pop("engine_md5")
    c, out = code(compare.compare, a, res([1, 2], [0, 0]))
    assert c == compare.REFUS and "remesurer" in out


@T.test
def pool_refuses_shared_seeds_and_accepts_disjoint_blocks():
    c, out = code(compare.pool, [res([1, 2], [1, 0], label="a"), res([2, 3], [0, 1], label="b")])
    assert c == compare.REFUS and "pas indépendants" in out
    c, out = code(compare.pool, [res([1, 2], [1, 0], label="a"), res([3, 4], [1, 1], label="b")])
    assert c == 0 and "75.0 %" in out


@T.test
def says_we_do_not_know_under_two_se():
    c, out = code(compare.compare, res(list(range(20)), [1, 0] * 10), res(list(range(20)), [0, 1] * 10))
    assert "on ne sait pas" in out


@T.test
def load_reads_manager_session_files():
    d = dict(engine_md5="abc", results=[dict(label="moteur vs Hook", per_seed=[[1, 1.0]])])
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(d, f)
    try:
        e = compare.load(f.name + ":Hook")
        assert e["engine_md5"] == "abc" and version.scores(e) == {1: 1.0}
    finally:
        os.unlink(f.name)


if __name__ == "__main__":
    T.main()
