const assert = require('node:assert/strict');
const fs = require('node:fs');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const labels = ['Dictate', 'Whisper-Ahead', 'Cleanup', 'Relay', 'Voiceprint', 'Conductor', 'Privacy', 'Dictionary', 'Analytics', 'Setup'];
const live = ['Dictate', 'Cleanup', 'Privacy', 'Setup'];
(async () => {
  const browser = await chromium.launch();
  const errors = [];
  try {
    for (const theme of ['light', 'dark']) for (const width of [450, 375, 500, 501, 900]) {
      const page = await browser.newPage({ viewport: { width, height: width <= 501 ? 300 : 600 }, colorScheme: theme, reducedMotion: 'reduce' });
      page.on('pageerror', error => errors.push(error.message));
      page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1427');
      assert.equal(await page.title(), 'Kaydence');
      assert.equal(await page.locator('vite-error-overlay').count(), 0);
      const nav = page.locator('.nav-list');
      assert(await nav.isVisible(), `${theme} ${width}: navigation must remain available`);
      for (const view of live.slice(0, 3)) {
        await nav.getByRole('button', { name: view, exact: true }).click();
        assert.deepEqual(await nav.locator('.nav-item-label').allTextContents(), labels, 'Locked order');
        assert.deepEqual(await nav.locator('button:enabled .nav-item-label').allTextContents(), live, 'Future entries stay disabled');
        assert.equal(await nav.locator('[aria-current="page"] .nav-item-label').textContent(), view);
        assert(await nav.evaluate(el => el.lastElementChild.getBoundingClientRect().bottom <= el.closest('aside').querySelector('.sidebar-card').getBoundingClientRect().top), 'Footer must not overlap navigation');
        assert(await page.locator('.brand-mark').isVisible(), 'Real identity remains visible');
        assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${view}: no horizontal overflow`);
        if (width === 900 && process.env.BASELINE_RAIL_DIR) {
          const rail = page.locator('.compact-sidebar');
          assert.deepEqual(await rail.boundingBox(), { x: 0, y: 0, width: 126, height: 600 });
          assert((await rail.screenshot({ animations: 'disabled' })).equals(fs.readFileSync(`${process.env.BASELINE_RAIL_DIR}/${view}-${theme}-rail.png`)), 'Default rail pixels unchanged');
        }
        if (width <= 500) assert(await nav.evaluate(el => Array.from(el.querySelectorAll('.nav-item-label, small')).every(node => parseFloat(getComputedStyle(node).fontSize) >= 11)), 'Readable narrow navigation');
        if (process.env.CAPTURE_DIR && [450, 900].includes(width)) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/${view.toLowerCase()}-${theme}-${width}.png`, fullPage: true, animations: 'disabled' });
      }
      await nav.getByRole('button', { name: 'Dictate', exact: true }).focus();
      for (const view of live.slice(1)) {
        await page.keyboard.press('Tab');
        const button = nav.getByRole('button', { name: view, exact: true });
        assert(await button.evaluate(el => el === document.activeElement), `Tab reaches ${view}`);
        const box = await button.boundingBox();
        const height = page.viewportSize().height;
        assert(box && box.x >= 0 && box.y >= 0 && box.x + box.width <= width && box.y + box.height <= height, `Focused ${view} is not clipped`);
        assert(await button.evaluate(el => getComputedStyle(el).outlineStyle !== 'none'), 'Visible keyboard focus');
        const ring = await button.evaluate(el => { const s = getComputedStyle(el); return parseFloat(s.outlineWidth) + parseFloat(s.outlineOffset); });
        assert(box.y - ring >= 0 && box.y + box.height + ring <= height, `Entire ${view} focus ring is visible`);
        if (process.env.CAPTURE_DIR && width === 501 && view === 'Setup') await page.screenshot({ path: `${process.env.CAPTURE_DIR}/focus-${theme}-501.png`, animations: 'disabled' });
        await page.keyboard.press('Enter');
      }
      const cancel = page.getByRole('button', { name: 'Cancel', exact: true });
      await cancel.focus();
      await page.keyboard.press('Enter');
      assert.equal(await nav.locator('[aria-current="page"] .nav-item-label').textContent(), 'Dictate', 'Setup round trip');
      console.log(`PASS ${theme} ${width}: locked nav, disabled phases, four routes, keyboard and overflow`);
      await page.close();
    }
    assert.deepEqual(errors, [], 'No frontend errors');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
