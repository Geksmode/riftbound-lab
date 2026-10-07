import csv, json, io, os, sys, urllib.request
from concurrent.futures import ThreadPoolExecutor
from PIL import Image
R = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "cards") + "/"
OUT = os.path.join(os.path.dirname(__file__), "thumbs")
urls = {x["id"]: x.get("image_url") for x in json.load(open(R + "cards_all_printings.json"))}
rows = list(csv.DictReader(open(R + "cards_unique.csv")))
def one(x):
    p = f"{OUT}/{x['id']}.jpg"
    if os.path.exists(p): return 0
    u = urls.get(x["id"])
    if not u: return x["id"]
    for k in range(3):
        try:
            b = urllib.request.urlopen(u, timeout=60).read()
            im = Image.open(io.BytesIO(b)).convert("RGB")
            if im.width > im.height: im = im.rotate(90, expand=True)  # battlefields are landscape
            im.resize((200, 279), Image.LANCZOS).save(p, quality=80, optimize=True)
            return 0
        except Exception as e:
            err = e
    return f"{x['id']} {err}"
with ThreadPoolExecutor(12) as ex:
    bad = [r for r in ex.map(one, rows) if r]
print("missing", len(bad), bad[:10])
