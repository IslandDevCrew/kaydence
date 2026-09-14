// Chromium frontend fixtures; CSS viewports are not native OS scaling proof.
const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
(async () => {
  const browser = await chromium.launch(), failures = [], measurements = [];
  try {
    for (const theme of ['light', 'dark']) for (const layout of ['miccapsule', 'pillbar', 'stackedpanel']) for (const [width, height] of [[900, 600], [450, 300], [501, 300], [700, 600], [761, 600], [800, 600], [880, 600], [899, 600], [900, 300]]) {
      const tag = `${theme}-${layout}-${width}x${height}`;
      const page = await browser.newPage({ viewport: { width, height }, colorScheme: theme, reducedMotion: 'reduce' });
      const check = (ok, message) => { if (!ok) failures.push(`${tag}: ${message}`); };
      page.on('pageerror', error => failures.push(`${tag}: ${error.message}`));
      await page.addInitScript(preset => localStorage.setItem('kaydence.cockpit.layoutPreset', preset), layout);
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1433');
      const board = page.locator('.dictate-board');
      const small = await board.evaluate(el => Array.from(el.querySelectorAll('*')).filter(node => node.getBoundingClientRect().height && Array.from(node.childNodes).some(child => child.nodeType === 3 && child.textContent.trim()) && parseFloat(getComputedStyle(node).fontSize) < 11).map(node => `${node.className || node.tagName}:${getComputedStyle(node).fontSize}`).slice(0, 8));
      check(!small.length, `11px floor: ${small.join(', ')}`);
      check(await board.evaluate(el => el.scrollWidth <= el.clientWidth + 1), 'board horizontal clipping');
      for (const panel of await page.locator('.cockpit-topband,.cockpit-capture-panel,.cockpit-history-panel,.cockpit-control-panel,.cockpit-transcript-panel').all()) {
        check(await panel.evaluate(el => el.clientHeight > 0 && el.scrollWidth <= el.clientWidth + 1), `panel dimensions: ${await panel.getAttribute('class')}`);
      }
      await page.getByRole('button', { name: 'Dictate', exact: true }).click();
      let historyTargets = 0, reachedFooter = false;
      for (let step = 0; step < 65; step++) {
        await page.keyboard.press('Tab');
        const target = await page.evaluate(() => {
          const el = document.activeElement, panel = el?.closest('.cockpit-history-panel');
          if (!panel) return null;
          const r = el.getBoundingClientRect(), b = panel.getBoundingClientRect();
          return { text: el.textContent.trim(), summary: el.tagName === 'SUMMARY',
            visible: r.top >= Math.max(0, b.top) && r.bottom <= Math.min(innerHeight, b.bottom) && r.left >= b.left && r.right <= b.right };
        });
        if (target) {
          historyTargets++; check(target.visible, `whole History keyboard target clipped: ${target.text}`);
          if (target.summary) await page.keyboard.press('Enter');
        }
        if (await page.evaluate(() => document.activeElement?.matches('.cockpit-sb-pill summary'))) { reachedFooter = true; break; }
      }
      check(reachedFooter && historyTargets >= 5, 'natural History traversal reaches footer without activating destructive controls');
      await page.reload();
      if (width === 900 && height === 600) {
        const [status, meter, pills] = await Promise.all(['.cockpit-sb-right > strong', '.cockpit-mic-meter', '.cockpit-sb-pills'].map(selector => page.locator(selector).boundingBox()));
        const left = meter.x - status.x - status.width, right = pills.x - meter.x - meter.width;
        measurements.push({ tag, left, right });
        check(left >= 0 && right >= 0 && Math.abs(left - right) < 0.05 && Math.abs(status.y + status.height / 2 - pills.y - pills.height / 2) < 1, 'meter remains centered on one row');
        if (process.env.CAPTURE_DIR) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/${tag}.png`, animations: 'disabled' });
      }
      if (width < 900) {
        const [meter, pills, right] = await Promise.all(['.cockpit-mic-meter', '.cockpit-sb-pills', '.cockpit-sb-right'].map(selector => page.locator(selector).boundingBox()));
        const padding = await page.locator('.cockpit-sb-right').evaluate(el => parseFloat(getComputedStyle(el).paddingRight));
        check(pills.y >= meter.y + meter.height && Math.abs(right.x + right.width - padding - meter.x - meter.width) < 1, 'wrapped pills keep meter gracefully docked at right edge');
        if (process.env.CAPTURE_DIR && width === 800 && layout === 'miccapsule') await page.screenshot({ path: `${process.env.CAPTURE_DIR}/docked-${tag}.png`, animations: 'disabled' });
      }
      for (const [index, summary] of (await page.locator('.cockpit-sb-pill summary').all()).entries()) {
        await summary.focus(); await page.keyboard.press('Enter');
        const button = page.locator('.cockpit-sb-pill').nth(index).getByRole('button', { name: 'Change in Setup', exact: true });
        await page.keyboard.press('Tab');
        check(await button.evaluate(el => el === document.activeElement), 'popover keyboard target');
        check(await button.evaluate(el => { const r = el.getBoundingClientRect(); const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return r.x >= 0 && r.y >= 0 && r.right <= innerWidth && r.bottom <= innerHeight && (hit === el || el.contains(hit)); }), 'popover action is visible and not clipped/occluded');
        const popup = page.locator('.cockpit-sb-pill').nth(index).locator('.cockpit-sb-pop');
        check(await popup.evaluate(el => { const r = el.getBoundingClientRect(); return r.x >= 0 && r.right <= innerWidth && el.scrollWidth <= el.clientWidth + 1; }), 'popover bounds');
        if (process.env.CAPTURE_DIR && index === 0 && layout === 'miccapsule' && [501, 900].includes(width)) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/popover-${tag}.png`, animations: 'disabled' });
        await summary.focus(); await page.keyboard.press('Enter');
      }
      const summaries = page.locator('.cockpit-sb-pill summary');
      await summaries.nth(1).focus(); await page.keyboard.press('Enter');
      await summaries.nth(0).focus(); await page.keyboard.press('Enter');
      check(await page.locator('.cockpit-sb-pill[open]').count() === 1, 'Reverse-order disclosures are mutually exclusive');
      await page.keyboard.press('Tab');
      const active = page.locator('.cockpit-sb-pill').first().getByRole('button', { name: 'Change in Setup' });
      check(await active.evaluate(el => { const r = el.getBoundingClientRect(); return document.activeElement === el && document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2) === el; }), 'Active disclosure action cannot be covered by stale popover');
      await page.keyboard.press('Escape');
      check(await page.locator('.cockpit-sb-pill[open]').count() === 0 && await summaries.first().evaluate(el => el === document.activeElement), 'Escape closes and restores summary focus');
      check(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'document overflow');
      console.log(`CHECK ${tag}`);
      await page.close();
    }
    console.log(JSON.stringify({ measurements, failures }, null, 2));
    assert.deepEqual(failures, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
