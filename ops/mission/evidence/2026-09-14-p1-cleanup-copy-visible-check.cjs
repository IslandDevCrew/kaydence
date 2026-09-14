// Presentation fixtures; natural keyboard/wheel only, never forced focus/scroll offsets.
const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
async function tabTo(page, target) {
  for (let i = 0; i < 85; i++) {
    await page.keyboard.press('Tab'); await page.waitForTimeout(100);
    if (await target.evaluate(el => el === document.activeElement)) {
      const faults = await target.evaluate(el => {
        const r = el.getBoundingClientRect(), s = getComputedStyle(el);
        const ring = Math.max(0, parseFloat(s.outlineWidth) + parseFloat(s.outlineOffset));
        let top = 0, bottom = innerHeight, left = 0, right = innerWidth;
        for (let a = el.parentElement; a; a = a.parentElement) {
          const c = getComputedStyle(a), b = a.getBoundingClientRect();
          if (/auto|scroll|hidden|clip/.test(c.overflowY)) { top = Math.max(top, b.top); bottom = Math.min(bottom, b.bottom); }
          if (/auto|scroll|hidden|clip/.test(c.overflowX)) { left = Math.max(left, b.left); right = Math.min(right, b.right); }
        }
        const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
        return { text: el.textContent, outline: s.outlineStyle, ring, bounds: r.toJSON(), clip: { top, bottom, left, right }, fits: r.top - ring >= top && r.bottom + ring <= bottom && r.left - ring >= left && r.right + ring <= right && hit && el.contains(hit) };
      });
      if (!faults.fits && process.env.CAPTURE_DIR) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/focus-failure.png` });
      assert(faults.fits && faults.outline !== 'none', `Whole focus outline: ${JSON.stringify(faults)}`); return;
    }
  }
  assert.fail('Natural Tab did not reach requested control');
}
async function readCopy(page, selector) {
  const area = page.locator(selector), b = await area.boundingBox(), vp = page.viewportSize();
  await page.mouse.move(Math.max(5, Math.min(vp.width - 5, b.x + b.width / 2)), Math.max(5, Math.min(vp.height - 5, b.y + b.height / 2)));
  await page.mouse.wheel(0, -15000); await page.waitForTimeout(350);
  const seen = new Set(); let fragments = [];
  for (let step = 0; step < 100; step++) {
    fragments = await area.evaluate(el => {
      const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT), out = []; let node, id = 0;
      while ((node = walker.nextNode())) {
        if (!node.textContent.trim()) continue;
        const parent = node.parentElement, font = parseFloat(getComputedStyle(parent).fontSize);
        const range = document.createRange(); range.selectNodeContents(node); let line = 0;
        for (const r of range.getClientRects()) {
          if (!r.width || !r.height) continue;
          let top = 0, bottom = innerHeight, left = 0, right = innerWidth;
          for (let a = parent; a; a = a.parentElement) {
            const s = getComputedStyle(a), box = a.getBoundingClientRect();
            if (/auto|scroll|hidden|clip/.test(s.overflowY)) { top = Math.max(top, box.top); bottom = Math.min(bottom, box.bottom); }
            if (/auto|scroll|hidden|clip/.test(s.overflowX)) { left = Math.max(left, box.left); right = Math.min(right, box.right); }
          }
          const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
          out.push({ id: `${id}:${line++}`, text: node.textContent.trim(), font,
            visible: r.top >= top - 1 && r.bottom <= bottom + 1 && r.left >= left - 1 && r.right <= right + 1 && hit && parent.contains(hit) });
        }
        id++;
      }
      return out;
    });
    assert(fragments.length, 'Meaningful rendered copy');
    assert(fragments.every(f => f.font >= 11), `11px copy floor: ${JSON.stringify(fragments.filter(f => f.font < 11))}`);
    for (const f of fragments) if (f.visible) seen.add(f.id);
    if (seen.size === fragments.length) return { selector, fragments: seen.size };
    await page.mouse.wheel(0, Math.min(vp.height, b.height) / 4); await page.waitForTimeout(60);
  }
  assert.fail(`Unreachable full text in ${selector}: ${JSON.stringify(fragments.filter(f => !seen.has(f.id)))}`);
}
(async () => {
  const browser = await chromium.launch(), results = [], errors = [];
  try {
    for (const theme of process.env.THEME ? [process.env.THEME] : ['light', 'dark']) for (const width of process.env.WIDTH ? [+process.env.WIDTH] : [900, 450, 500, 501]) for (const layout of process.env.LAYOUT ? [process.env.LAYOUT] : ['miccapsule', 'pillbar', 'stackedpanel']) {
      const page = await browser.newPage({ viewport: { width, height: width === 900 ? 600 : 300 }, colorScheme: theme, reducedMotion: 'reduce' });
      const tag = `${theme}/${width}/${layout}`;
      page.on('pageerror', e => errors.push({ tag, error: e.message }));
      page.on('console', m => { if (m.type() === 'error') errors.push({ tag, error: m.text() }); });
      await page.addInitScript(p => localStorage.setItem('kaydence.cockpit.layoutPreset', p), layout);
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1437');
      assert.equal(await page.title(), 'Kaydence'); assert.equal(await page.locator('vite-error-overlay').count(), 0);
      for (const dial of ['raw', 'light', 'full']) {
        const panel = page.getByRole('region', { name: 'Cleanup', exact: true });
        await tabTo(page, panel.getByRole('button', { name: dial, exact: true })); await page.keyboard.press('Space');
        assert.equal(await panel.getByRole('button', { name: dial, exact: true }).getAttribute('aria-pressed'), 'true');
        const checks = [await readCopy(page, '.cockpit-control-panel:has(.cockpit-dial)')];
        await tabTo(page, panel.getByRole('button', { name: 'Configure cleanups', exact: true }));
        if (process.env.CAPTURE_DIR && dial === 'full') await page.screenshot({ path: `${process.env.CAPTURE_DIR}/dictate-${theme}-${width}-${layout}.png` });
        if (layout === 'miccapsule') {
          await page.keyboard.press('Enter');
          assert.equal(await page.locator('.cleanup-dial-section').getByRole('button', { name: dial, exact: true }).getAttribute('aria-pressed'), 'true');
          checks.push(await readCopy(page, '.cleanup-rules-panel'));
          if (process.env.CAPTURE_DIR && dial === 'full') await page.screenshot({ path: `${process.env.CAPTURE_DIR}/cleanup-${theme}-${width}-full-copy.png` });
          await tabTo(page, page.getByRole('button', { name: 'Save Changes', exact: true }));
          await tabTo(page, page.locator('.nav-list').getByRole('button', { name: 'Dictate', exact: true }));
          await page.keyboard.press('Enter');
        }
        results.push({ tag, dial, checks }); console.log(`PASS ${tag}/${dial}: complete copy and natural focus`);
      }
      await page.close();
    }
    assert.deepEqual(errors, []); console.log(JSON.stringify({ results, errors }, null, 2));
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
