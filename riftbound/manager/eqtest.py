import sys, json
sys.path.insert(0, sys.argv[1])
import run_session as R
d = json.load(open(__import__('os').path.join(__import__('os').path.dirname(__import__('os').path.abspath(__file__)), 'results/session_017.json')))
n = int(sys.argv[2])
same = diff = 0; dl = []
for r in d['results'][:2]:
    job = dict(r['job'], n=n)
    res = R.run(job, 206800)
    old = {s: w for s, w, *_ in r['per_seed']}
    for s, w, *_ in res['per_seed']:
        if old[s] == w: same += 1
        else: diff += 1; dl.append((r['label'][:20], s, old[s], w))
    print(job['label'], 'new wr', res['wr'], 'errors', res['errors'], res['err'], flush=True)
print('same', same, 'diff', diff, dl[:10])
