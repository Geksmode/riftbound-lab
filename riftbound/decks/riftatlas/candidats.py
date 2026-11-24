import json,glob,re,collections,datetime,sys,csv
from decode import deck,REPO
BAN={r['name'] for r in csv.DictReader(open(REPO+'/cards/cards_unique.csv',encoding='utf-8')) if r['banned_standard']}
RAD=datetime.date(2026,10,23)
cands=collections.defaultdict(list); seen=set(); stats=collections.Counter()
for f in sorted(glob.glob('ra/ev_*.json')):
    slug=f[6:-5]; j=json.load(open(f))
    m=re.search(r'Events\s+(\w+ \d+, \d{4})\s+(.*?)\s+(\d+) players',j['head'],flags=re.S)
    date=datetime.datetime.strptime(m.group(1).title(),'%b %d, %Y').date(); players=int(m.group(3))
    ename=None
    for r in j['rows']:
        code=re.search(r'code=([A-Z0-9]+)',r['href']).group(1); rank=int(re.search(r'rank=(\d+)',r['href']).group(1))
        if (slug,rank) in seen: continue
        seen.add((slug,rank))
        nm=re.search(r'name=([^&]*)',r['href']).group(1)
        import urllib.parse
        nm=urllib.parse.unquote_plus(nm); player,_,ename=nm.partition(' · ')
        rec=re.search(r'(\d+-\d+(?:-\d+)?)\s*$',r['text'])
        try: d,u=deck(code)
        except Exception: stats['decode-err']+=1; continue
        flat=lambda s:[n for q,n in d[s] for _ in range(q)]
        probs=[]
        if len(d['legend'])!=1 or len(d['champion'])!=1: probs.append('legend/champion')
        if len(flat('main'))+1!=40: probs.append(f"main {len(flat('main'))+1}")
        if len(flat('runes'))!=12: probs.append('runes')
        if len(flat('battlefields'))!=3: probs.append('bf')
        if u: probs.append('unk')
        allc=set(flat('main'))|set(flat('sideboard'))|set(flat('battlefields'))|{n for q,n in d['champion']}
        ban=sorted(allc&BAN)
        if probs: stats['invalide']+=1; continue
        lg=d['legend'][0][1]
        cands[lg].append(dict(legend=lg,slug=slug,event=ename,date=str(date),players=players,rank=rank,player=player,record=rec.group(1) if rec else '',
            code=code,href=r['href'],deck=d,ban=ban,side=len(flat('sideboard')),tier=0 if date>=RAD else 1,pct=(rank-0.5)/players))
        stats['ok']+=1
print(stats,len(cands))
def key(c): return (bool(c["ban"]), c["side"]<8, c["players"]<16, (0 if c["tier"]==0 and c["pct"]<=0.5 else 1 if c["pct"]<=0.5 else 2), c["pct"], -c["players"])
out={}
for lg in sorted(cands):
    cs=sorted(cands[lg],key=key); out[lg]=[c for c in cs if not c["ban"]][:15]
    legal=[c for c in cs if not c['ban']]
    t0=[c for c in legal if c['tier']==0]
    b=cs[0]
    print(f"{lg[:34]:34} n={len(cs):4} légales={len(legal):4} rad={len(t0):4} | {'BAN ' if b['ban'] else ''}{b['date']} {b['event'][:32]:32} {b['rank']}/{b['players']} {b['player'][:18]} sb{b['side']} {b['record']}")
json.dump(out,open('cands.json','w'))
allleg={r['name'] for r in csv.DictReader(open(REPO+'/cards/cards_unique.csv',encoding='utf-8')) if r['type']=='Legend'}
print('légendes sans liste :',sorted(allleg-set(cands)))
