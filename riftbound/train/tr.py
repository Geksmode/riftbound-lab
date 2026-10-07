import asyncio, random, sys
from playwright.async_api import async_playwright
W = int(sys.argv[1]) if len(sys.argv) > 1 else 1400
H = int(sys.argv[2]) if len(sys.argv) > 2 else 900
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(executable_path="/opt/pw-browsers/chromium-1194/chrome-linux/chrome")
        pg = await b.new_page(viewport={"width": W, "height": H}, is_mobile=W < 600, has_touch=W < 600)
        errs = []
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.on("console", lambda m: m.type == "error" and errs.append(m.text))
        await pg.goto("http://localhost:8766/train.html")
        await pg.wait_for_selector("#bStart", timeout=60000)
        await pg.fill("#sSeed", "42")
        await pg.click("#bStart")
        rr = random.Random(1)
        shots = 0
        undone = False
        import time; t0 = time.time()
        for it in range(400):
            await pg.wait_for_function("!working", timeout=60000)
            if await pg.query_selector("#bAgain"):
                break
            if await pg.query_selector("#bKeep"):
                await pg.screenshot(path=f"tr_{W}_mull.png"); 
                await pg.click("[data-m='0']"); await pg.click("#bKeep"); continue
            asks = await pg.query_selector_all("[data-a]")
            if asks:
                await asks[0].click(); continue
            cans = await pg.query_selector_all(".card.can")
            if shots < 2 and it >= 3 and cans:
                await cans[0].click()
                shots += 1; await pg.screenshot(path=f"tr_{W}_sel{shots}.png")
                if shots == 2:
                    await pg.click("#bHint"); await pg.wait_for_selector(".hintrow", timeout=60000)
                    await pg.screenshot(path=f"tr_{W}_hint.png")
            opts = await pg.query_selector_all("#act [data-i]")
            if not opts:
                await asyncio.sleep(0.2); continue
            if it == 30 and not undone:
                undone = True
                await pg.click("#bUndo"); print("undo ok", await pg.inner_text("#aiLine")); continue
            opts = [x for x in opts if await x.is_visible()]
            labels = [await x.inner_text() for x in opts]
            good = [x for x, l in zip(opts, labels) if not l.startswith(("Terminer", "Passer", "Conseil"))]
            o = rr.choice(good) if good and rr.random() < 0.85 else opts[0]
            await o.click()
        await pg.screenshot(path=f"tr_{W}_end.png")
        print("fin", round(time.time()-t0), "s", it, "itérations", await pg.inner_text("#actTitle"), await pg.inner_text("#gmeta"), "erreurs:", errs[:5])
        await b.close()
asyncio.run(main())
