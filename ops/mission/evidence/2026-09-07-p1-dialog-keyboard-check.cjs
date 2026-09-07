// Run against the built frontend, with an already-installed Playwright module.
const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => {
      if (message.type() === 'error') errors.push(message.text());
    });
    for (const viewport of [{ width: 900, height: 600 }, { width: 450, height: 300 }]) {
      for (const view of ['Privacy', 'Setup']) {
        await page.setViewportSize({ width: 900, height: 600 });
        await page.goto(process.env.APP_URL || 'http://127.0.0.1:1427');
        assert.equal(await page.title(), 'Kaydence');
        assert.equal(await page.locator('vite-error-overlay').count(), 0, 'No framework overlay');
        await page.locator('.nav-list').getByRole('button', { name: view, exact: true }).click();
        const trigger = page.getByRole('button', {
          name: view === 'Privacy' ? 'Privacy Constitution' : 'Evidence', exact: true,
        });
        await trigger.click();
        await page.setViewportSize(viewport);
        const modal = page.getByRole('dialog');
        assert(await modal.isVisible(), `${view}: nonblank modal`);
        const focusInside = () => modal.evaluate(el => el.contains(document.activeElement));
        assert(await focusInside(), `${view}: initial focus`);
        for (const key of ['Shift+Tab', ...Array(18).fill('Tab')]) {
          await page.keyboard.press(key);
          assert(await focusInside(), `${view}: ${key} must stay inside modal`);
        }
        const close = modal.getByRole('button', { name: /Close/ });
        const box = await close.boundingBox();
        assert(box && box.x >= 0 && box.y >= 0 && box.x + box.width <= viewport.width
          && box.y + box.height <= viewport.height, `${view}: Close remains in viewport`);
        if (process.env.CAPTURE_DIR) await page.screenshot({
          path: `${process.env.CAPTURE_DIR}/${view.toLowerCase()}-dialog-${viewport.width}x${viewport.height}.png`,
        });
        await page.keyboard.press('Escape');
        assert.equal(await page.getByRole('dialog').count(), 0, `${view}: Escape closes`);
        assert(await trigger.evaluate(el => el === document.activeElement), `${view}: focus restored`);
        await trigger.click();
        await page.getByRole('dialog').getByRole('button', { name: /Close/ }).click();
        assert(await trigger.evaluate(el => el === document.activeElement), `${view}: Close restores focus`);
        console.log(`PASS ${view} ${viewport.width}x${viewport.height}: keyboard loop, visible Close, Escape, focus return`);
      }
    }
    assert.deepEqual(errors, [], 'No frontend console errors');
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
