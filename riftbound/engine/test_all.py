"""Run every scenario test file: test_cards.py and each cardsets/test_*.py, each in its own process.
Usage: python3 test_all.py [-v]     (exit code 1 if any test fails)"""
import glob, os, re, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))


def main():
    files = ["test_cards.py"] + sorted(os.path.relpath(f, HERE) for f in glob.glob(os.path.join(HERE, "cardsets", "test_*.py")))
    total_ok = total = 0
    bad = []
    for f in files:
        p = subprocess.run([sys.executable, f], cwd=HERE, capture_output=True, text=True)
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
