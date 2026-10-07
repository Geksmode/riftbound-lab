import asyncio, sys, os
from playwright.async_api import async_playwright
W = int(sys.argv[1]); H = int(sys.argv[2]); SEED = sys.argv[3]
async def drag(pg, a, b):
    await pg.evaluate("document.querySelector('.boardwrap').scrollIntoView()")
    ra, rb = await a.bounding_box(), await b.bounding_box()
    x0, y0 = ra["x"] + ra["width"] / 2, ra["y"] + ra["height"] / 2
    x1, y1 = rb["x"] + min(rb["width"] / 2, 30), rb["y"] + min(rb["height"] / 2, 30)
    await pg.mouse.move(x0, y0); await pg.mouse.down()
    for k in range(1, 11): await pg.mouse.move(x0 + (x1 - x0) * k / 10, y0 + (y1 - y0) * k / 10)
    await pg.mouse.up()
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(executable_path="/opt/pw-browsers/chromium-1194/chrome-linux/chrome")
        pg = await b.new_page(viewport={"width": W, "height": H}, is_mobile=W < 600, has_touch=W < 600)
        errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
        await pg.goto("http://localhost:8766/train.html")
        await pg.wait_for_selector("#bStart", timeout=60000)
        await pg.fill("#sSeed", SEED); await pg.click("#bStart")
        done, shots, n = {}, set(), 0
        async def shot(k):
            if k not in shots: shots.add(k); await pg.screenshot(path=f"v9_{W}_{SEED}_{k}.png", full_page=W < 600)
        async def lastlog(word):
            L = await pg.evaluate("LOG.map(x=>x[1])"); return [l for l in L if word in l][-1:]
        for it in range(400):
            await pg.wait_for_function("V && !working", timeout=60000)
            if await pg.query_selector("#bAgain"): await shot("fin"); break
            if await pg.query_selector("#bKeep"):
                await shot("mulligan")
                if not done.get("mullbug"):
                    done["mullbug"] = 1; await pg.click("#bNew"); await asyncio.sleep(0.2); await pg.click("#mClose"); await asyncio.sleep(0.2)
                    print("mulligan après fermeture :", await pg.is_visible("#bKeep"))
                    await pg.click("#bJournal"); await asyncio.sleep(0.4); await shot("journal"); await pg.click("#bJClose")
                await pg.click("#bKeep"); continue
            if await pg.evaluate("!!(mode && mode.type === 'ask')"):
                k = await pg.evaluate("mode.a.kind"); done["ask_" + k] = done.get("ask_" + k, 0) + 1
                t = await pg.query_selector("#board .drop-ok")
                if t:
                    await t.click(); await shot("ask_" + k); await pg.click("#fOk")
                else:
                    await shot("ask_" + k); await (await pg.query_selector("#fbar [data-a]")).click()
                print("choix", k, ":", await pg.evaluate("LOG.slice(-2).map(x=>x[1])")); continue
            if not done.get("trash") and await pg.evaluate("V.st.p[0].trash.length + V.st.p[1].trash.length > 1"):
                done["trash"] = 1; pid = await pg.evaluate("V.st.p[0].trash.length ? 0 : 1")
                await (await pg.query_selector(f'[data-trash="{pid}"]')).click(); await asyncio.sleep(0.2); await shot("trash")
                print("défausse :", (await pg.inner_text("#mbox"))[:80].replace("\n", " ")); await pg.keyboard.press("Escape"); continue
            sfs = await pg.evaluate("V.dec.options.filter(o => o.k === 'play' && o.mover !== null && o.mover !== undefined)")
            if sfs and not done.get("shuriken"):
                done["shuriken"] = 1; o = ([x for x in sfs if x["t1"]] or sfs)[0]
                await (await pg.query_selector(f'#board [data-uid="{o["src"]}"]')).click()
                await (await pg.query_selector('#pop button.tg')).click(); await shot("sf_1")
                if o["t1"]: await (await pg.query_selector(f'#board .bf [data-uid="{o["t1"][0]}"], #board .zone [data-uid="{o["t1"][0]}"]')).click()
                else: await pg.click("#fSkip")
                await shot("sf_2")
                await (await pg.query_selector(f'#board .bf [data-uid="{o["mover"]}"], #board .zone [data-uid="{o["mover"]}"]')).click(); await shot("sf_3")
                await (await pg.query_selector(f'#board [data-drop="{o["loc"]}"]')).click(position={"x": 8, "y": 8}); await shot("sf_4")
                print("shuriken étapes :", (await pg.inner_text("#fbar")).replace("\n", " | "))
                await pg.click("#fOk"); await pg.wait_for_function("!working")
                print("shuriken :", await lastlog("Shuriken")); continue
            accs = await pg.evaluate("V.dec.options.filter(o => o.k === 'play' && o.acc && !(o.t1||[]).length).map(o => o.src)")
            if accs and not done.get("acc"):
                done["acc"] = 1
                await (await pg.query_selector(f'#board [data-uid="{accs[0]}"]')).click(); await shot("accelerate")
                print("menu accelerate :", (await pg.inner_text("#pop")).replace("\n", " | "))
                await (await pg.query_selector('#pop button.acc')).click(); await pg.wait_for_function("!working")
                print("accelerate :", await lastlog("joue")); continue
            if await pg.evaluate("Object.keys(ALIAS).length > 0") and not done.get("alias"):
                k = await pg.evaluate("Object.keys(ALIAS)[0]")
                await (await pg.query_selector(f'#board [data-uid="{k}"]')).click(); await shot("alias")
                nb = len(await pg.query_selector_all("#pop button")); done["alias"] = nb
                print("copie en double, boutons du menu :", nb, await pg.inner_text("#pop"))
                await pg.keyboard.press("Escape"); continue
            asks = await pg.query_selector_all("#fbar [data-a]")
            if asks: await asks[0].click(); continue
            if await pg.is_visible("#fbar"):   # état inattendu : on annule
                li = await pg.query_selector("#fbar [data-i]")
                if li: await li.click(); done["liste"] = done.get("liste", 0) + 1; continue
                await pg.click("#fNo"); continue
            ops = await pg.evaluate("V.dec.options")
            end = [o for o in ops if o["k"] in ("end", "pass")][0]
            emp = [o for o in ops if o["k"] == "act" and o["label"].startswith("Empower") and o["src"] != "legend"]
            tg2 = [o for o in ops if o["k"] == "play" and len(o["t1"]) == 2 and o["t1"][0] != o["t1"][1] and o.get("from_") == "hand"]
            tg1 = [o for o in ops if o["k"] == "play" and len(o["t1"]) == 1 and o.get("from_") == "hand"]
            grp = [o for o in ops if o["k"] == "move" and len(o["us"]) == 2 and o["loc"] in (0, 1)]
            mv = [o for o in ops if o["k"] == "move" and len(o["us"]) == 1 and o["loc"] in (0, 1)]
            zp = [o for o in ops if o["k"] == "play" and o.get("from_") in ("hand", "champ") and o.get("loc") is not None and not o["t1"]]
            q = lambda u: pg.query_selector(f'#board [data-uid="{u}"]')
            qu = lambda u: pg.query_selector(f'#board .bf [data-uid="{u}"], #board .zone [data-uid="{u}"]')
            n += 1
            if emp and not done.get("emp"):
                done["emp"] = 1; o = emp[0]
                await (await q(o["src"])).click(); await shot("pop_empower")
                bt = await pg.query_selector('#pop button:has-text("Empower")')
                await bt.click(); await pg.wait_for_function("!working")
                print("empower :", await lastlog("Empower")); continue
            if tg2 and not done.get("tg2"):
                done["tg2"] = 1; o = tg2[0]
                await (await q(o["src"])).click(); await shot("pop_cible")
                await (await pg.query_selector('#pop button.tg')).click()
                await (await qu(o["t1"][0])).click(); await (await qu(o["t1"][1])).click()
                await shot("cibles_2"); await pg.click("#fOk"); await pg.wait_for_function("!working")
                print("2 cibles :", await lastlog("joue Falling")); continue
            if tg1 and not done.get("tg1"):
                done["tg1"] = 1; o = tg1[0]
                await drag(pg, await q(o["src"]), await qu(o["t1"][0])); await shot("cible_drag")
                if await pg.is_enabled("#fOk"): await pg.click("#fOk")
                await pg.wait_for_function("!working"); print("1 cible :", await lastlog("cible")); continue
            if grp and not done.get("grp"):
                done["grp"] = 1; o = grp[0]
                await (await q(o["us"][0])).click(); await (await pg.query_selector('#pop button[data-mv]')).click()
                await (await q(o["us"][1])).click()
                await (await pg.query_selector(f'#board [data-drop="{o["loc"]}"]')).click(position={"x": 8, "y": 8})
                await shot("groupe"); await pg.click("#fOk"); await pg.wait_for_function("!working")
                print("groupe :", await lastlog("Akali déplace")); continue
            if mv and done.get("mv", 0) < 3:
                done["mv"] = done.get("mv", 0) + 1; o = mv[0]
                await drag(pg, await q(o["us"][0]), await pg.query_selector(f'#board [data-drop="{o["loc"]}"]'))
                await shot("move_drag")
                if await pg.is_enabled("#fOk"): await pg.click("#fOk")
                else: print("move refusé", o["label"]); await pg.click("#fNo"); continue
                await pg.wait_for_function("!working"); print("move :", await lastlog("Akali déplace")); continue
            if zp and n < 60:
                o = zp[0]
                await drag(pg, await q(o["src"]), await pg.query_selector(f'#board [data-drop="{o["loc"]}"]')); await asyncio.sleep(0.3); continue
            await pg.click("#bMain")
        print(done, await pg.inner_text("#turnInfo"), "erreurs:", errs[:3])
        await b.close()
asyncio.run(main())
