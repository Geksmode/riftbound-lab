import json,subprocess
import os
CHECK=os.path.join(os.path.dirname(os.path.abspath(__file__)),'..','..','..','rust','target','release','check_decks')
c=json.load(open('cands.json')); com=json.load(open('com.json'))
def js(d,key):
    flat=lambda s:[n for q,n in d[s] for _ in range(q)]
    return dict(key=key,legend=d['legend'][0][1],champion=d['champion'][0][1],main=flat('main'),runes=[r.replace(' Rune','') for r in flat('runes')],battlefields=flat('battlefields'),sideboard=flat('sideboard'))
allc=[]
for lg,cs in c.items():
    for i,x in enumerate(cs): x['src']='tournoi'; allc.append((f't|{lg}|{i}',x))
for lg,cs in com.items():
    for i,x in enumerate(sorted(cs,key=lambda x:(x['side']<8,-x['clones'],-x['likes']))): x['src']='communauté'; allc.append((f'c|{lg}|{i:03}',x))
json.dump([js(x['deck'],k) for k,x in allc],open('tocheck.json','w'),ensure_ascii=False)
res=dict(l.split('\t',1) for l in subprocess.run([CHECK,'tocheck.json'],capture_output=True,text=True).stdout.splitlines())
import csv
legs=sorted(r['name'] for r in csv.DictReader(open(os.path.join(os.path.dirname(CHECK),'..','..','..','riftbound','cards','cards_unique.csv'),encoding='utf-8')) if r['type']=='Legend')
pick={}
for lg in legs:
    for k,x in allc:
        if k.split('|')[1]!=lg: continue
        if res[k]=='':
            pick[lg]=x; break
        else: print('  rejet',k,x.get('player') or x.get('title','')[:40],'→',res[k][:140])
    x=pick.get(lg)
    if not x: print(f'{lg:36} AUCUNE'); continue
    if x['src']=='tournoi': print(f"{lg[:34]:34} {x['date']} {x['event'][:34]:34} {x['rank']:>3}/{x['players']:<3} {x['record']:7} {x['player'][:20]} sb{x['side']}")
    else: print(f"{lg[:34]:34} COMMUNAUTÉ maj {x['upd']} clones {x['clones']} ♥{x['likes']} sb{x['side']} {x['title'][:60]}")
json.dump(pick,open('pick.json','w'),ensure_ascii=False)
