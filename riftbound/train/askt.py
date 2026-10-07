import asyncio
from playwright.async_api import async_playwright
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(executable_path="/opt/pw-browsers/chromium-1194/chrome-linux/chrome")
        pg = await b.new_page(viewport={"width": 1400, "height": 900})
        await pg.goto("http://localhost:8766/train.html"); await pg.wait_for_selector("#bStart", timeout=60000)
        await pg.fill("#sSeed", "44"); await pg.click("#bStart")
        await pg.wait_for_function("!working && V", timeout=60000)
        while not await pg.evaluate("!!(V.dec && V.st.bfs.some(b => b.u.length))"):
            await pg.wait_for_function("!working", timeout=60000)
            if await pg.query_selector("#bKeep"): await pg.click("#bKeep")
            else:
                e = await pg.query_selector("#act .big [data-i]"); await e.click()
            await pg.wait_for_function("!working", timeout=60000)
        us = await pg.evaluate("V.st.bfs.flatMap(b => b.u.map(u => u.u)).slice(0, 2)")
        await pg.evaluate(f"""(() => {{ const v = JSON.parse(JSON.stringify(V)); delete v.dec; v.log = [];
            v.ask = {{ kind: "target", title: "Akali, Deadly Weapon : infliger 1 à quelle unité ?", options: ["unité A", "unité B", "aucun"], uids: [{us[0]}, {us[1] if len(us) > 1 else 'null'}, null], item: "Akali, Deadly Weapon" }};
            show(v); }})()""")
        print("mode:", await pg.evaluate("mode && mode.type"), "surbrillance:", len(await pg.query_selector_all("#board .drop-ok")))
        await (await pg.query_selector(f'#board [data-uid="{us[0]}"]')).click()
        print("choix:", await pg.evaluate("mode.pick"), "bouton actif:", await pg.is_enabled("#fOk"), await pg.inner_text("#fbar"))
        await pg.screenshot(path="ask_test.png")
        await b.close()
asyncio.run(main())
