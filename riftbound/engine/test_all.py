"""Run every scenario test file: test_cards.py and each cardsets/test_*.py, each in its own process.
Usage: python3 test_all.py [-v]     (exit code 1 if any test fails)
The files run side by side, one per core (RB_JOBS=n to choose, RB_JOBS=1 one after another); the report keeps
the file order."""
import glob, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))


def run(f):
    return subprocess.run([sys.executable, f], cwd=HERE, capture_output=True, text=True)


def main():
    files = ["test_cards.py"] + sorted(os.path.relpath(f, HERE) for f in glob.glob(os.path.join(HERE, "cardsets", "test_*.py")))
    total_ok = total = 0
    bad = []
    jobs = int(os.environ.get("RB_JOBS") or os.cpu_count() or 1)
    with ThreadPoolExecutor(max(1, jobs)) as ex:
        procs = list(ex.map(run, files))
    for f, p in zip(files, procs):
        out = p.stdout + p.stderr
        m = re.findall(r"(\d+)/(\d+) tests passed", out)
        if m:
            ok, n = map(int, m[-1])
        else:
            ok, n = 0, 1
        total_ok += ok
        total += n
        status = "ok" if (ok == n and p.returncode == 0) else "FAIL"
        print(f"{status:4} {ok:4}/{n:<4} {f}")
        if status != "ok":
            bad.append(f)
            if "-v" in sys.argv or not m:
                print(out[-3000:])
            else:
                print("\n".join(l for l in out.splitlines() if l.startswith("FAIL")))
    print(f"{total_ok}/{total} tests passed in {len(files)} files")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
