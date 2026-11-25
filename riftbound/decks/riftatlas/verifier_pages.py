import json,subprocess,re,html,os,hashlib,collections
pick=json.load(open('pick.json')); bad=0
for lg,x in pick.items():
    url='https://riftatlas.com'+html.unescape(x['href']) if x['src']=='tournoi' else x['url']
    f='ra/ver/'+hashlib.md5(url.encode()).hexdigest()+'.html'
    if not os.path.exists(f) or os.path.getsize(f)<50000:
        subprocess.run(['curl','-s','-m','60','-L',url,'-o',f])
    h=open(f,encoding='utf-8',errors='replace').read()
    # texte visible de la page : sections « N x Nom »
    t=re.sub(r'<script.*?</script>|<style.*?</style>','',h,flags=re.S)
    t=html.unescape(re.sub(r'\s+',' ',re.sub(r'<[^>]+>',' ',t)))
    d=x['deck']; miss=[]
    want=collections.Counter()
    for sec in ['legend','champion','main','runes','battlefields','sideboard']:
        for q,n in d[sec]:
            if f"{q} x {n} ".lower() not in t.lower(): miss.append((sec,q,n))
    tot=re.search(r'Main Deck (\d+)/39 .*?Sideboard (\d+)/',t) or re.search(r'Main Deck (\d+)/39',t)
    side=sum(q for q,n in d['sideboard']); main=sum(q for q,n in d['main'])
    ok=not miss and tot and int(tot.group(1))==main and (tot.lastindex<2 or int(tot.group(2))==side)
    x['url']=url; x['verified']=bool(ok)
    if not ok: bad+=1; print('ÉCART',lg,miss[:5],tot.groups() if tot else None,main,side)
print('vérifiées',len(pick)-bad,'/',len(pick))
json.dump(pick,open('pick.json','w'),ensure_ascii=False)
