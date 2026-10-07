import asyncio, sys, json
from playwright.async_api import async_playwright
W = int(sys.argv[1]); H = int(sys.argv[2]); SEED = sys.argv[3] if len(sys.argv) > 3 else "42"
import os; TAP = os.environ.get("TAP") == "1"
async def drag(pg, a, b, shot=None):
    ra, rb = await a.bounding_box(), await b.bounding_box()
    x0, y0 = ra["x"] + ra["width"] / 2, ra["y"] + ra["height"] / 2
    x1, y1 = rb["x"] + rb["width"] / 2, rb["y"] + rb["height"] / 2
    await pg.mouse.move(x0, y0); await pg.mouse.down()
    for k in range(1, 11):
        await pg.mouse.move(x0 + (x1 - x0) * k / 10, y0 + (y1 - y0) * k / 10)
    if shot: await pg.screenshot(path=shot)
    await pg.mouse.up()
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(executable_path="/opt/pw-browsers/chromium-1194/chrome-linux/chrome")
        pg = await b.new_page(viewport={"width": W, "height": H}, is_mobile=W < 600, has_touch=W < 600)
        errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
        await pg.goto("http://localhost:8766/train.html")
        await pg.wait_for_selector("#bStart", timeout=60000)
        await pg.fill("#sSeed", SEED); await pg.click("#bStart")
        stats = {"drag": 0, "cible": 0, "liste": 0, "rate": 0, "move": 0}
        shots = set(); done = {}
        for it in range(300):
            await pg.wait_for_function("!working", timeout=60000)
            if await pg.query_selector("#bAgain"): break
            if await pg.query_selector("#bKeep"): await pg.click("#bKeep"); continue
            asks = await pg.query_selector_all("[data-a]")
            if asks: await asks[0].click(); continue
            if await pg.query_selector("#bCancel"):
                # cible ou précision demandée
                tgt = await pg.query_selector("#board .card.drop-ok")
                if tgt:
                    if "cible" not in shots: shots.add("cible"); await pg.screenshot(path=f"dnd_{W}_cible.png")
                    await tgt.click(); stats["cible"] += 1
                else:
                    stats["liste"] += 1; await (await pg.query_selector("#act .opt.go")).click()
                continue
            na = await pg.evaluate("document.querySelectorAll('#arrowsG path').length")
            if na and "fleche_ia" not in shots:
                shots.add("fleche_ia"); await pg.screenshot(path=f"dnd_{W}_fleche_ia.png"); print("flèches IA:", na, await pg.evaluate("JSON.stringify(lastAI)"))
            ops = await pg.evaluate("V.dec.options")
            pick = None
            for o in ops:
                if not done.get("2t") and o["k"] == "play" and len(o["t1"]) == 2 and o["t1"][0] != o["t1"][1] and o.get("from_") == "hand":
                    pick = ("2t", o); break
                if not done.get("grp") and o["k"] == "move" and len(o["us"]) == 2 and o["loc"] in (0, 1):
                    pick = ("grp", o); break
            if pick and pick[0] == "2t":
                o = pick[1]; done["2t"] = 1
                src = await pg.query_selector(f'#board [data-uid="{o["src"]}"]')
                t1 = await pg.query_selector(f'#board .bf [data-uid="{o["t1"][0]}"], #board .zone [data-uid="{o["t1"][0]}"]')
                await drag(pg, src, t1); await asyncio.sleep(0.2)
                await pg.screenshot(path=f"dnd_{W}_2t_a.png")
                t2 = await pg.query_selector(f'#board .bf [data-uid="{o["t1"][1]}"], #board .zone [data-uid="{o["t1"][1]}"]')
                await t2.click(); await pg.wait_for_function("!working", timeout=60000)
                print("2 cibles :", [l for l in await pg.evaluate("LOG.map(x=>x[1])") if "joue" in l][-2:]); continue
            if pick and pick[0] == "grp":
                o = pick[1]; done["grp"] = 1
                for u in o["us"]: await (await pg.query_selector(f'#board [data-uid="{u}"]')).click(); await asyncio.sleep(0.1)
                await pg.screenshot(path=f"dnd_{W}_grp.png")
                await (await pg.query_selector(f'#board [data-drop="{o["loc"]}"]')).click(position={"x": 8, "y": 8})
                await pg.wait_for_function("!working", timeout=60000)
                print("groupe :", [l for l in await pg.evaluate("LOG.map(x=>x[1])") if "déplace" in l and l.startswith("Akali")][-1:]); continue
            for o in ops:
                if o["k"] == "play" and o.get("from_") == "hand" and o["t1"]:
                    pick = ("t", o); break
            if not pick:
                for o in ops:
                    if o["k"] == "play" and o.get("from_") in ("hand", "champ") and o.get("loc") is not None and not o["t1"]:
                        pick = ("z", o); break
            if not pick:
                for o in ops:
                    if o["k"] == "move" and len(o["us"]) == 1 and o["loc"] in (0, 1): pick = ("m", o); break
            if not pick or stats["drag"] > 25:
                e = [o for o in ops if o["k"] in ("end", "pass")]
                await pg.click(f'#act .big [data-i="{e[0]["i"]}"]'); continue
            kind, o = pick
            src = await pg.query_selector(f'#board [data-uid="{o["src"] if kind != "m" else o["us"][0]}"]')
            if kind == "t": dst = await pg.query_selector(f'#board .bf [data-uid="{o["t1"][0]}"], #board .zone [data-uid="{o["t1"][0]}"]')
            else: dst = await pg.query_selector(f'#board [data-drop="{o["loc"]}"]')
            if not src or not dst: stats["rate"] += 1; e=[x for x in ops if x["k"] in ("end","pass")]; await pg.click(f'#act .big [data-i="{e[0]["i"]}"]'); continue
            n0 = len(await pg.evaluate("LOG"))
            sh = None
            if kind not in shots: shots.add(kind); sh = f"dnd_{W}_{kind}{'_tap' if TAP else ''}.png"
            if TAP:
                await src.click(); await asyncio.sleep(0.1)
                if sh: await pg.screenshot(path=sh)
                dst = await pg.query_selector(f'#board .bf [data-uid="{o["t1"][0]}"], #board .zone [data-uid="{o["t1"][0]}"]') if kind == "t" else await pg.query_selector(f'#board [data-drop="{o["loc"]}"]')
                await dst.scroll_into_view_if_needed(); await dst.click(position={"x": 8, "y": 8} if kind != "t" else None)
            else:
                await dst.scroll_into_view_if_needed(); await drag(pg, src, dst, sh)
            stats["drag"] += 1
            if kind == "m": stats["move"] += 1
            await asyncio.sleep(0.3)
        await pg.screenshot(path=f"dnd_{W}_fin.png")
        print(stats, await pg.inner_text("#actTitle"), "erreurs:", errs[:3])
        await b.close()
asyncio.run(main())
