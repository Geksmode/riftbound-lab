import base64,csv,json,glob,re,os,urllib.parse,collections
import os
REPO=os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))   # riftbound/
SETS={0:'OGN',1:'OGS',3:'SFD',4:'UNL',5:'VEN',6:'RAD'}
P={}
for r in csv.DictReader(open(REPO+'/cards/cards_all_printings.csv',encoding='utf-8')):
    P[(r['set_code'],r['collector_number'].lstrip('0').lower())]=(r['name'],r['type'],r['supertype'])
def varints(b):
    out=[];i=0
    while i<len(b):
        v=0;s=0
        while True:
            x=b[i];i+=1;v|=(x&0x7f)<<s;s+=7
            if not x&0x80:break
        out.append(v)
    return out
def decode(code):
    v=varints(base64.b32decode(code+'='*(-len(code)%8)))
    i=1; cards=[]   # (section, qty, set, variant, num)
    def groups(q,sec):
        nonlocal i
        n=v[i];i+=1
        for _ in range(n):
            k,s,var=v[i],v[i+1],v[i+2];i+=3
            for _ in range(k):
                cards.append((sec,q,s,var,v[i]));i+=1
    for q in range(12,0,-1): groups(q,'main')
    for q in range(3,0,-1):
        if i<len(v): groups(q,'side')
    if i<len(v):
        if v[i]==1: cards.append(('champ',1,v[i+1],v[i+2],v[i+3])); i+=4
        else: i+=1
    assert i==len(v),(i,len(v))
    return cards
def name(s,var,num):
    k=(SETS.get(s),str(num)+('' if not var else '?'))
    return P.get((SETS.get(s),str(num))) if not var else None
def deck(code):
    """-> dict sections or raises"""
    d=dict(legend=[],champion=[],main=[],runes=[],battlefields=[],sideboard=[]); unk=[]
    cs=decode(code)
    champ=[c for c in cs if c[0]=='champ']
    for sec,q,s,var,num in cs:
        c=name(s,var,num)
        if not c: unk.append((sec,q,s,var,num)); continue
        n,t,sup=c
        if sec=='champ': d['champion'].append((1,n))
        elif sec=='side': d['sideboard'].append((q,n))
        elif t=='Legend': d['legend'].append((q,n))
        elif t=='Rune': d['runes'].append((q,n))
        elif t=='Battlefield': d['battlefields'].append((q,n))
        else: d['main'].append((q,n))
    # le champion désigné est compté dans le main : en retirer 1 exemplaire
    if d['champion']:
        cn=d['champion'][0][1]; out=[]; done=False
        for q,n in d['main']:
            if n==cn and not done:
                done=True
                if q>1: out.append((q-1,n))
            else: out.append((q,n))
        d['main']=out
        if not done: unk.append(('champ-absent',cn))
    return d,unk
if __name__=='__main__':
    c="CMAAAAAAAAAQCAAAFIAACAIAADLACAADAIAAALJ2AUBQAIJKTEA2CAOGAEAQIABHAQAQAAGVAEBAGAE2AGRQCAIEAAVACBQAEACAEAAAFOTAEBADACYQDRIB2IA5SAIBAUABWAYGAAOCGKQAAIBAAABL4AAQEBIAFCBQCAQBAQALAAIBAYACSAIDACYQC"
    d,u=deck(c); print(d,u)
    # stats sur tous les codes
    st=collections.Counter(); sets=collections.Counter(); vars_=collections.Counter()
    for f in glob.glob('ra/ev_*.json'):
        for r in json.load(open(f))['rows']:
            code=re.search(r'code=([A-Z0-9]+)',r['href']).group(1)
            try:
                for sec,q,s,var,num in decode(code): sets[s]+=1; vars_[var]+=1
                d,u=deck(code); st['ok' if not u else 'unk']+=1
                if u and st['unk']<8: print(u[:4])
            except Exception as e: st['err']+=1; print('ERR',f,repr(e)[:80])
    print(st,sets,vars_)
