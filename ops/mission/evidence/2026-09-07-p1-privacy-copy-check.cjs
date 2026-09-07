const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 900, height: 600 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
    for (const theme of ['light', 'dark']) {
      await page.emulateMedia({ colorScheme: theme, reducedMotion: 'reduce' });
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1427');
      await page.locator('.nav-list').getByRole('button', { name: 'Privacy', exact: true }).click();
      assert.equal(await page.title(), 'Kaydence');
      assert.equal(await page.locator('vite-error-overlay').count(), 0);
      const summary = await page.getByRole('region', { name: 'Privacy constitution summary' }).innerText();
      assert(summary.includes('Audio and transcripts stay local.'), 'Acknowledge retained audio');
      assert(summary.includes('30-day retention policy.'), 'Render fixture retention setting');
      const audit = page.getByRole('complementary', { name: 'Network audit' });
      const copy = await audit.innerText();
      assert(copy.toLowerCase().includes('source-level'), 'Identify the source-level audit');
      assert(copy.includes('Live connections not measured here.'), 'Disclose measurement boundary');
      assert(!/Verified|outbound connections|since launch/i.test(copy), 'No invented live proof');
      if (process.env.CAPTURE_DIR) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/privacy-${theme}-900x600.png` });
      await page.getByRole('button', { name: 'Privacy Constitution', exact: true }).click();
      const modalCopy = await page.getByRole('dialog').innerText();
      assert(modalCopy.includes('Audio and transcripts are retained locally.'));
      assert(modalCopy.includes('History refresh applies your 30-day retention policy'));
      assert(!modalCopy.includes('Audio never persists'));
      await page.keyboard.press('Escape');
      assert.equal(await page.getByRole('dialog').count(), 0);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      console.log(`PASS ${theme}: retention truth, source-audit boundary, modal access, no overflow`);
    }
    assert.deepEqual(errors, [], 'No frontend errors');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
