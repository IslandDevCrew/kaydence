// Read-only targeted contrast evidence; browser OS references are not native OS proof.
const assert = require('node:assert/strict'), fs = require('node:fs');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const luminance = value => {
  const rgb = value.match(/[\d.]+/g).slice(0, 3).map(Number).map(x => {
    x /= 255; return x <= .04045 ? x / 12.92 : ((x + .055) / 1.055) ** 2.4;
  });
  return .2126 * rgb[0] + .7152 * rgb[1] + .0722 * rgb[2];
};
(async () => {
  const browser = await chromium.launch(), results = [];
  try {
    for (const theme of ['light', 'dark']) {
      const page = await browser.newPage({ viewport: { width: 900, height: 600 }, colorScheme: theme, reducedMotion: 'reduce' });
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1431');
      await page.locator('.nav-list').getByRole('button', { name: 'Cleanup', exact: true }).click();
      for (const lane of ['macOS', 'Windows', 'Linux']) {
        await page.locator('.cleanup-lane-switcher').getByRole('button', { name: lane, exact: true }).click();
        const button = page.locator('.cleanup-actions button:last-child');
        const colors = await button.evaluate(el => { const s = getComputedStyle(el); return { text: el.textContent, foreground: s.color, background: s.backgroundColor, font: s.fontSize, weight: s.fontWeight, disabled: el.disabled }; });
        assert.equal(colors.disabled, false, 'Enabled action, not inactive-control exception');
        const f = luminance(colors.foreground), b = luminance(colors.background);
        const ratio = (Math.max(f, b) + .05) / (Math.min(f, b) + .05);
        results.push({ theme, lane, ...colors, ratio, minimum: 4.5, pass: ratio >= 4.5 });
        if (process.env.CAPTURE_DIR) await button.screenshot({ path: process.env.CAPTURE_DIR + '/' + theme + '-' + lane + '.png' });
      }
      await page.close();
    }
    console.log(JSON.stringify({ sourceHead: 'd21ace91ea9f50ff4a0c1c492a1eaea7e6a17721', results }, null, 2));
    if (process.env.RESULT_PATH) fs.writeFileSync(process.env.RESULT_PATH, JSON.stringify(results, null, 2) + '\n');
    assert(results.every(x => x.pass), 'Locked accent treatment fails normal-text contrast; design ruling required');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
