import sys,re,json,subprocess,os,html
u=sys.argv[1]; i=u.rsplit('/',1)[1]; out=f'com/{i}.json'
if os.path.exists(out): sys.exit()
h=subprocess.run(['curl','-s','-m','60','-L',u],capture_output=True).stdout.decode('utf-8','replace')
h2=h.replace('\\"','"')
m=re.search(r'"([A-Z2-7]{60,})"\},"publishedAt":(\d+),"updatedAt":(\d+),"thumbsUpCount":(\d+),"commentCount":(\d+),"cloneCount":(\d+)',h2)
t=re.search(r'<title>(.*?)</title>',h)
if not t: sys.exit()
json.dump(dict(url=u,title=html.unescape(t.group(1)),m=m.groups() if m else None,size=len(h)),open(out,'w'))
