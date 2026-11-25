const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
const slugs = process.argv.slice(2);
const b = await chromium.launch();
const ctx = await b.newContext({ ignoreHTTPSErrors: false });
for (const slug of slugs) {
  const out = `ra/ev_${slug}.json`;
  if (fs.existsSync(out)) continue;
  const p = await ctx.newPage();
  const failed = [];
  p.on('requestfailed', r => failed.push(r.url().slice(0, 120)));
  try {
    await p.goto(`https://riftatlas.com/events/${slug}`, { waitUntil: 'domcontentloaded', timeout: 60000 });
    await p.waitForTimeout(2500);
    const btn = p.locator('button', { hasText: /All \d+ standings/ });
    if (await btn.count()) { await btn.first().click(); await p.waitForTimeout(4000); }
    const data = await p.evaluate(() => {
      const rows = [...document.querySelectorAll('li')].filter(li => li.querySelector('a[href*="/decks/view?code="]'));
      const head = document.querySelector('main')?.innerText.slice(0, 400);
      return { head, rows: rows.map(li => ({
        href: li.querySelector('a[href*="/decks/view?code="]').getAttribute('href'),
        legend: li.querySelector('[title]')?.getAttribute('title'),
        text: li.innerText.replace(/\s+/g, ' ').slice(0, 200) })) };
    });
    data.failed = failed.slice(0, 5);
    fs.writeFileSync(out, JSON.stringify(data));
    console.log(slug, data.rows.length, failed.length, (data.head || '').replace(/\s+/g, ' ').slice(0, 120));
  } catch (e) { console.log(slug, 'ERR', String(e).slice(0, 200)); }
  await p.close();
}
await b.close();
})();
