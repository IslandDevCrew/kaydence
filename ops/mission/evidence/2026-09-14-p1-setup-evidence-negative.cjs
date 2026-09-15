// Deliberately broken browser-only counterfactuals; never used as positive evidence.
const assert = require('node:assert/strict'), fs = require('node:fs');
const engines = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const { fullRead, focused, fixture } = require('./2026-09-14-p1-setup-evidence-check.cjs');
const out = process.env.CAPTURE_DIR, rows = [];
(async () => {
  for (const engine of ['chromium', 'webkit']) {
    const browser = await engines[engine].launch();
    try { for (const fault of ['font', 'hidden', 'clipped', 'outline', 'content-row', 'eyebrow-rule']) {
      const page = await browser.newPage({ viewport: { width: 450, height: 300 } }), row = { engine, fault };
      try {
        if (fault === 'content-row') await fixture(page, true);
        await page.goto(process.env.APP_URL || 'http://127.0.0.1:1435');
        await page.locator('.nav-list').getByRole('button', { name: 'Setup', exact: true }).click();
        await page.getByRole('button', { name: 'Evidence', exact: true }).click();
        await page.getByRole('dialog').getByRole('button', { name: 'models', exact: true }).click();
        row.before = await fullRead(page); assert.deepEqual(row.before.issues, [], 'Negative starts with a passing actual surface');
        if (fault === 'outline') {
          await page.keyboard.press(engine === 'webkit' ? 'Alt+Tab' : 'Tab');
          const before = await page.locator(':focus').evaluate(focused); assert(before.indicator && before.visible);
          await page.addStyleTag({ content: 'dialog[open] button { outline: none !important; }' });
          row.after = await page.locator(':focus').evaluate(focused); assert(!row.after.indicator, 'Missing indicator must reject');
        } else if (fault === 'content-row' || fault === 'eyebrow-rule') {
          const selector = fault === 'content-row' ? '.first-run-evidence-body' : '.first-run-dialog header span';
          const property = fault === 'content-row' ? 'grid-auto-rows' : 'font-size';
          assert.equal(await page.evaluate(({ selector, property }) => {
            let changed = 0;
            for (const sheet of document.styleSheets) for (const rule of sheet.cssRules) if (rule.selectorText === selector && rule.style.getPropertyValue(property)) { rule.style.removeProperty(property); changed++; }
            return changed;
          }, { selector, property }), 1, 'Remove exactly the owned production declaration');
          row.after = await fullRead(page); assert(row.after.issues.some(s => s.startsWith(fault === 'content-row' ? 'unread text' : 'sub11')), 'Removing cure must reproduce original defect');
        } else {
          const css = fault === 'font' ? 'font-size:8px !important' : fault === 'hidden' ? 'visibility:hidden !important' : 'height:3px !important;overflow:hidden !important';
          await page.addStyleTag({ content: `.first-run-evidence-summary p { ${css} }` });
          row.after = await fullRead(page);
          assert(row.after.issues.some(s => s.startsWith(fault === 'font' ? 'sub11' : 'unread text')), 'Original text oracle must reject the fault');
        }
        await page.screenshot({ path: `${out}/${engine}-${fault}.png` }); row.pass = true;
      } catch (e) { row.pass = false; row.error = e.message; }
      rows.push(row); await page.close(); console.log(JSON.stringify({ engine, fault, pass: row.pass, error: row.error }));
    } } finally { await browser.close(); }
  }
  fs.writeFileSync(`${out}/negative.json`, JSON.stringify(rows, null, 2)); assert(rows.every(r => r.pass));
})().catch(e => { console.error(e); process.exitCode = 1; });
