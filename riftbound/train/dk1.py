import asyncio, sys, random
from playwright.async_api import async_playwright
W = int(sys.argv[1]); H = int(sys.argv[2])
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(executable_path="/opt/pw-browsers/chromium-1194/chrome-linux/chrome")
        pg = await b.new_page(viewport={"width": W, "height": H}, is_mobile=W < 600, has_touch=W < 600)
        errs = []; pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("console", lambda m: m.type == "error" and errs.append(m.text)); pg.on("response", lambda r: r.status >= 400 and errs.append(r.url))
        await pg.goto("http://localhost:8767/train.html")
        await pg.wait_for_selector("#bStart", timeout=90000)
        print("ton deck:", await pg.evaluate("[...$('sMine').options].map(o=>o.textContent).slice(0,4)"))
        await pg.click("#bOpenDecks"); await pg.wait_for_selector(".dkgrid .dkc")
        print("cartes affichées:", await pg.evaluate("document.querySelectorAll('.dkc').length"), await pg.inner_text(".dkcat > .small"))
        await pg.screenshot(path=f"dk_{W}_vide.png")
        await pg.select_option("#dkType", "Legend"); await asyncio.sleep(0.2)
        await pg.screenshot(path=f"dk_{W}_legendes.png")
        await pg.click('.dkc[data-add="Akali, Rogue Assassin"]')
        await pg.select_option("#dkType", ""); await pg.fill("#dkQ", "Falling"); await asyncio.sleep(0.3)
        await pg.click('.dkc[data-add="Falling Star"]'); await pg.click('.dkc[data-add="Falling Star"]')
        print("statut:", await pg.inner_text(".dkst"), (await pg.inner_text(".dkerr"))[:200].replace("\n", " | "))
        # copie d'un deck de référence, modification, enregistrement
        await pg.select_option("#dkLoad", "akali_dongdong_wuhan-open_5th"); await asyncio.sleep(0.3)
        print("copie:", await pg.inner_text(".dkst"), await pg.input_value("#dkName"))
        await pg.fill("#dkName", "Akali test")
        await pg.fill("#dkQ", ""); await asyncio.sleep(0.2)
        await pg.screenshot(path=f"dk_{W}_deck.png")
        # importer
        txt = await pg.evaluate("exportText(DK)")
        print("export lignes:", len(txt.splitlines()))
        await pg.click("#dkPlay"); await pg.wait_for_selector("#bStart")
        print("deck choisi:", await pg.evaluate("$('sMine').value"), await pg.inner_text("#sPlan"))
        await pg.select_option("#sOpp", "akali-g2")
        await pg.fill("#sSeed", "77"); await pg.click("#bStart")
        rr = random.Random(1)
        for it in range(120):
            await pg.wait_for_function("V && !working", timeout=90000)
            if await pg.evaluate("V.winner !== null && V.winner !== undefined"): break
            if await pg.query_selector("#bKeep"): await pg.click("#bKeep"); continue
            ok = await pg.evaluate("""(() => { if (V.ask) { const x = V.ask.kind === 'mulligan' ? [] : 0; answerX(x); return 'ask'; } return null; })()""") if False else None
            n = await pg.evaluate("V.dec ? V.dec.options.length : 0")
            if not n:
                if await pg.evaluate("!!V.ask"):
                    await pg.evaluate("(async()=>{ await pump(J(T.answer(JSON.stringify(V.ask.kind==='mulligan'?[]:0)))) })()")
                continue
            i = rr.randrange(n)
            await pg.evaluate(f"(async()=>{{ recMove(['act',{i},'',V.st.t]); await pump(J(T.act({i}))) }})()")
            if it == 30: await pg.screenshot(path=f"dk_{W}_partie.png")
        print("tour", await pg.evaluate("V.st.t"), "noms", await pg.evaluate("PN"), "gmeta", await pg.inner_text("#gmeta"))
        print("rec deck:", await pg.evaluate("REC && REC.mine ? REC.mine.slice(0,60) : null"))
        await pg.screenshot(path=f"dk_{W}_fin.png")
        print("erreurs:", errs[:5])
        await b.close()
asyncio.run(main())
