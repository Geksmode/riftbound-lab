import asyncio, sys, random, json
from playwright.async_api import async_playwright
W, H = int(sys.argv[1]), int(sys.argv[2])
A, B = json.load(open("rdecks.json"))
def my(d, i):
    m = {}; r = {}
    for n in d["main"]: m[n] = m.get(n, 0) + 1
    for x in d["runes"]: k = x.replace(" Rune", ""); r[k] = r.get(k, 0) + 1
    return {"id": f"r{i}", "name": f"Test {d['legend'].split(',')[0]}", "legend": d["legend"], "champion": d["champion"], "main": m, "runes": r, "battlefields": d["battlefields"]}
async def waitr(pg):
    for _ in range(1200):
        if await pg.evaluate("!!(V && !working)"): return
        await asyncio.sleep(0.1)
    raise TimeoutError("table bloquée")
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(executable_path="/opt/pw-browsers/chromium-1194/chrome-linux/chrome")
        pg = await b.new_page(viewport={"width": W, "height": H})
        errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
        await pg.goto("http://localhost:8767/train.html")
        await pg.evaluate("d => localStorage.setItem('rbt-decks', JSON.stringify(d))", {"r0": my(A, 0), "r1": my(B, 1)})
        await pg.reload()
        await pg.wait_for_selector("#bStart", timeout=120000)
        print("jouables:", await pg.evaluate("CAT.cards.filter(c=>c.m).length"), "/", await pg.evaluate("CAT.cards.length"))
        await pg.select_option("#sMine", "my:r0"); await pg.select_option("#sOpp", "my:r1")
        print(await pg.inner_text("#sPlan"))
        await pg.fill("#sSeed", "5"); await pg.click("#bStart")
        rr = random.Random(2); asks = set()
        for it in range(250):
            await waitr(pg)
            if await pg.evaluate("V.winner !== null && V.winner !== undefined"): break
            if await pg.query_selector("#bKeep"): await pg.click("#bKeep"); continue
            if await pg.evaluate("!!V.ask"):
                t = await pg.evaluate("V.ask.title"); asks.add(t)
                if len(asks) <= 6: await pg.screenshot(path=f"dk2_ask{len(asks)}.png")
                n = await pg.evaluate("V.ask.options.length")
                await pg.evaluate(f"(async()=>{{ await pump(J(T.answer(JSON.stringify({rr.randrange(n)})))) }})()"); continue
            n = await pg.evaluate("V.dec ? V.dec.options.length : 0")
            if n: await pg.evaluate(f"(async()=>{{ await pump(J(T.act({rr.randrange(n)}))) }})()")
            if it == 40: await pg.screenshot(path=f"dk2_{W}.png")
        print("tour", await pg.evaluate("V.st.t"), await pg.evaluate("V.st.pts"), "gagnant", await pg.evaluate("V.winner"), "PN", await pg.evaluate("PN"))
        print("questions:", sorted(asks)); print("erreurs:", errs[:5])
        await b.close()
asyncio.run(main())
