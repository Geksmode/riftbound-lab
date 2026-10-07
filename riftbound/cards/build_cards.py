import json,csv,re,html,collections
# curl -o bulk.json https://api.rifthunt.com/bulk/cards
SRC='bulk.json'; import os; OUT=os.path.dirname(os.path.abspath(__file__)) + '/'
d=json.load(open(SRC))
SETS={'OGN':'Origins','OGS':'Origins: Proving Grounds','SFD':'Spiritforged','UNL':'Unleashed','VEN':'Vendetta'}
ORDER=['OGN','SFD','UNL','VEN','OGS']
# bans (Standard) : ancienne base + table du résumé des règles
bans={}
for o in csv.DictReader(open(OUT+'cards_all_printings.csv')):
    if o['banned_standard']: bans[o['name']]=o['banned_standard']
for line in open(OUT + '../rules/tournament-rules-resume.md'):
    m=re.match(r'\| (.+?) \| \w+ \| (\d{4}-\d\d-\d\d) \|',line)
    if m: bans[m.group(1)]=m.group(2)
def tok(m):
    parts=re.findall(r':rb_([a-z0-9_]+):',m.group(0)); out=[]
    runes=collections.Counter()
    for p in parts:
        if p.startswith('energy_'): out.append(f"{p[7:]} energy")
        elif p.startswith('rune_'): runes[p[5:]]+=1
        elif p=='might': out.append('might')
        elif p=='exhaust': out.append('exhaust')
    for r,n in runes.items():
        out.append(f"{n} rune{'s' if n>1 else ''} of any type" if r=='rainbow' else f"{n} {r} rune{'s' if n>1 else ''}")
    return ' and '.join(out)
def clean(x):
    t=x.get('rich') or x.get('text') or ''
    t=re.sub(r'<br\s*/?>|</p>\s*<p>',' ',t); t=re.sub(r'<[^>]+>','',t); t=html.unescape(t)
    t=re.sub(r'(?::rb_[a-z0-9_]+:)+',tok,t)
    return re.sub(r'\s+',' ',t).strip()
def kws(t):
    ks=[]
    for k in re.findall(r'\[([^\]]+)\]',t):
        k=re.sub(r'\s*\d+$','',k).strip()
        if k and k[0].isalpha() and k not in ks: ks.append(k)
    return '|'.join(ks)
rows=[]
for x in d['cards']:
    if x['set'] not in SETS: continue
    rid=x['riftboundId']; parts=rid.split('-')
    cid='-'.join(parts[:2]); num=parts[1]
    variant=bool(x['alt'] or x['over'] or x['sig'] or x['rarity']=='Showcase' or not num.isdigit())
    text=clean(x)
    name=re.sub(r'\s*\((Alternate Art|Overnumbered|Signature|Metal|Champion|Top 8|Starter|GG EZ|Launch Exclusive|Ultimate)\)$','',x['name'])
    pw=x['power']
    if pw is None and x['type'] in ('Unit','Spell','Gear'): pw=0
    rows.append(dict(id=cid,set=SETS[x['set']],set_code=x['set'],collector_number=num,name=name,type=x['type'],
        rarity=x['rarity'],domain='|'.join(x['domains'] or []),energy_cost='' if x['energy'] is None else x['energy'],
        power_cost='' if pw is None else pw,might='' if x['might'] is None else x['might'],tags='|'.join(x['tags'] or []),
        keywords=kws(text),is_variant=variant,banned_standard=bans.get(name,''),supertype=x['supertype'] or '',
        text=text,image_url=(x['imgUrl'] or '').split('?')[0]))
rows.sort(key=lambda r:(ORDER.index(r['set_code']),r['collector_number'].zfill(4),r['id']))
assert len({r['id'] for r in rows})==len(rows)
COLS=['id','set','set_code','collector_number','name','type','rarity','domain','energy_cost','power_cost','might','tags','keywords','is_variant','banned_standard','supertype','text']
def w(path,rs,cols):
    with open(path,'w',newline='') as f:
        wr=csv.DictWriter(f,cols,extrasaction='ignore'); wr.writeheader(); wr.writerows(rs)
w(OUT+'cards_all_printings.csv',rows,COLS+['image_url'])
json.dump(rows,open(OUT+'cards_all_printings.json','w'),ensure_ascii=False,indent=1)
uniq={}
for r in sorted(rows,key=lambda r:(r['is_variant'],r['set_code']=='OGS',ORDER.index(r['set_code']),r['collector_number'].zfill(4))):
    uniq.setdefault(r['name'],r)
u=sorted(uniq.values(),key=lambda r:(ORDER.index(r['set_code']),r['collector_number'].zfill(4)))
w(OUT+'cards_unique.csv',u,COLS)
print(len(rows),collections.Counter(r['set_code'] for r in rows))
print(len(u),collections.Counter(r['set_code'] for r in u))
print('banned',sorted((r['name'],r['banned_standard']) for r in u if r['banned_standard']))
print('missing bans',set(bans)-{r['name'] for r in u})
