import json,glob,collections,datetime,csv,sys
from decode import deck,REPO
BAN={r['name'] for r in csv.DictReader(open(REPO+'/cards/cards_unique.csv',encoding='utf-8')) if r['banned_standard']}
out=collections.defaultdict(list); st=collections.Counter()
for f in glob.glob('ra/com/*.json'):
    j=json.load(open(f))
    if not j['m']: st['sans code']+=1; continue
    code,pub,upd,likes,com,clones=j['m']
    try: d,u=deck(code)
    except Exception: st['err']+=1; continue
    flat=lambda s:[n for q,n in d[s] for _ in range(q)]
    if len(d['legend'])!=1 or len(d['champion'])!=1 or len(flat('main'))!=39 or len(flat('runes'))!=12 or len(flat('battlefields'))!=3 or u: st['incomplet']+=1; continue
    allc=set(flat('main'))|set(flat('sideboard'))|set(flat('battlefields'))|{d['champion'][0][1]}
    ban=sorted(allc&BAN)
    if ban: st['banni']+=1; continue
    st['ok']+=1
    out[d['legend'][0][1]].append(dict(url=j['url'],title=j['title'],code=code,deck=d,upd=str(datetime.datetime.utcfromtimestamp(int(upd)/1000))[:16],pub=str(datetime.datetime.utcfromtimestamp(int(pub)/1000))[:10],likes=int(likes),clones=int(clones),side=len(flat('sideboard'))))
print(st)
json.dump(out,open('com.json','w'))
for lg in ["Akali, Rogue Assassin","Lee Sin, Blind Monk","Leona, Radiant Dawn","Darius, Hand of Noxus","Yasuo, Unforgiven","Riven, The Exile","Rengar, Pridestalker","Master Yi, Wuju Master","Teemo, Swift Scout","Garen, Might of Demacia","Jinx, Loose Cannon","Annie, Dark Child","Ahri, Nine-Tailed Fox"]:
    print(lg,len(out[lg]))
    for x in sorted(out[lg],key=lambda x:(-x['clones'],-x['likes']))[:6]: print('   ',x['upd'],'pub',x['pub'],'♥',x['likes'],'clones',x['clones'],'sb',x['side'],'|',x['title'][:70])
