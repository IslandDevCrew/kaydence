// Browser skill absent: Playwright fixture rendering, not native WebView or OS-scaling proof.
// 450x300 is the CSS viewport equivalent of a 900x600 window at 200%; DPR is not zoom.
const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const labels = ['Dictate', 'Whisper-Ahead', 'Cleanup', 'Relay', 'Voiceprint', 'Conductor', 'Privacy', 'Dictionary', 'Analytics', 'Setup'];
const live = ['Dictate', 'Cleanup', 'Privacy', 'Setup'];
const textSelector = '.brand-lockup strong, .nav-item-label > span, .nav-item small, .sidebar-card strong, .sidebar-card p';
const results = [];
(async () => {
  const browser = await chromium.launch();
  let markSource;
  try {
    for (const theme of ['light', 'dark']) for (const width of [375, 450, 500, 501, 900]) {
      const page = await browser.newPage({ viewport: { width, height: width === 900 ? 600 : 300 }, colorScheme: theme, reducedMotion: 'reduce' });
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      try {
        await page.goto(process.env.APP_URL || 'http://127.0.0.1:1430');
        assert.equal(await page.title(), 'Kaydence');
        assert.equal(await page.locator('vite-error-overlay').count(), 0);
        const rail = page.getByRole('complementary', { name: 'Kaydence navigation' });
        const nav = rail.getByRole('navigation');
        for (const view of live.slice(0, 3)) {
          await nav.getByRole('button', { name: view, exact: true }).click();
          assert.deepEqual(await nav.locator('.nav-item-label').allTextContents(), labels);
          assert.deepEqual(await nav.locator('button:enabled .nav-item-label').allTextContents(), live);
          assert.equal(await nav.locator('[aria-current="page"] .nav-item-label').textContent(), view);
          const logo = rail.locator('.brand-mark');
          if (width === 900) assert.equal((await rail.boundingBox()).width, 126, 'Default rail width is preserved');
          assert(await logo.isVisible() && await logo.evaluate(el => el.complete && el.naturalWidth > 0), 'Real mark loads');
          markSource ??= await logo.getAttribute('src');
          assert.equal(await logo.getAttribute('src'), markSource, 'One shared product mark');
          const tooSmall = await rail.evaluate(el => [...el.querySelectorAll('*')].filter(node => node.getClientRects().length && getComputedStyle(node).visibility === 'visible' && [...node.childNodes].some(child => child.nodeType === Node.TEXT_NODE && child.textContent.trim())).filter(node => parseFloat(getComputedStyle(node).fontSize) < 10.99).map(node => `${node.textContent.trim()}: ${getComputedStyle(node).fontSize}`));
          assert.deepEqual(tooSmall, [], `${view}: all rendered rail text must meet the locked 11px floor`);
          for (const item of await rail.locator(textSelector).all()) {
            await item.scrollIntoViewIfNeeded();
            const issue = await item.evaluate(el => {
              const range = document.createRange(); range.selectNodeContents(el);
              const fragments = [...range.getClientRects()].filter(rect => rect.width && rect.height);
              const clips = [el.getBoundingClientRect()];
              for (let parent = el.parentElement; parent; parent = parent.parentElement) {
                const style = getComputedStyle(parent);
                if (/(hidden|clip|auto|scroll)/.test(`${style.overflowX} ${style.overflowY}`)) clips.push(parent.getBoundingClientRect());
              }
              return fragments.some(rect => clips.some(clip => rect.left < clip.left - 1 || rect.right > clip.right + 1 || rect.top < clip.top - 1 || rect.bottom > clip.bottom + 1)) ? el.textContent.trim() : null;
            });
            assert.equal(issue, null, 'Whole text ranges must remain inside labels and clipping ancestors');
          }
          assert(await rail.evaluate(el => { const brand = el.querySelector('.brand-lockup').getBoundingClientRect(); const nav = el.querySelector('.nav-list').getBoundingClientRect(); const footer = el.querySelector('.sidebar-card').getBoundingClientRect(); return brand.bottom <= nav.top && nav.bottom <= footer.top; }), 'Brand, navigation and footer must not overlap');
          assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'No document horizontal overflow');
          if (process.env.CAPTURE_DIR && [450, 900].includes(width)) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/rail-${view.toLowerCase()}-${theme}-${width}.png`, fullPage: true, animations: 'disabled' });
        }
        await nav.getByRole('button', { name: 'Dictate', exact: true }).focus();
        for (const view of live.slice(1)) {
          await page.keyboard.press('Tab');
          const button = nav.getByRole('button', { name: view, exact: true });
          assert(await button.evaluate(el => el === document.activeElement), `Tab reaches ${view}`);
          const box = await button.boundingBox(); const height = page.viewportSize().height;
          assert(box && box.x >= 0 && box.y >= 0 && box.x + box.width <= width && box.y + box.height <= height, `${view} focus scroll is fully visible`);
          assert(await button.evaluate(el => getComputedStyle(el).outlineStyle !== 'none'), 'Visible keyboard focus');
          const ring = await button.evaluate(el => { const s = getComputedStyle(el); return parseFloat(s.outlineWidth) + parseFloat(s.outlineOffset); });
          assert(box.y - ring >= 0 && box.y + box.height + ring <= height, 'Entire focus ring is visible');
          await page.keyboard.press('Enter');
        }
        await page.getByRole('button', { name: 'Cancel', exact: true }).focus();
        await page.keyboard.press('Enter');
        assert.equal(await nav.locator('[aria-current="page"] .nav-item-label').textContent(), 'Dictate');
        assert.deepEqual(errors, [], 'No uncaught frontend errors');
        results.push({ theme, width, status: 'PASS' });
      } catch (error) { results.push({ theme, width, status: 'FAIL', reason: error.message }); }
      finally { await page.close(); }
    }
  } finally { await browser.close(); }
  console.log(JSON.stringify({ scope: 'CSS fixture rendering only; 450x300 is 200%-equivalent, not native scaling proof', results }, null, 2));
  if (results.some(result => result.status === 'FAIL')) process.exitCode = 1;
})().catch(error => { console.error(error); process.exitCode = 1; });
