// Browser plugin unavailable: Chromium fixtures, not native WebView/OS-scaling proof.
const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const results = [];
(async () => {
  const browser = await chromium.launch();
  try {
    for (const theme of ['light', 'dark']) for (const width of [450, 500, 501, 900]) for (const lane of ['macOS', 'Windows', 'Linux']) {
      const height = width === 900 ? 600 : 300, page = await browser.newPage({ viewport: { width, height }, colorScheme: theme, reducedMotion: 'reduce' }), errors = [];
      page.on('pageerror', e => errors.push(e.message));
      page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
      try {
        await page.goto(process.env.APP_URL || 'http://127.0.0.1:1434');
        assert.equal(await page.title(), 'Kaydence'); assert.equal(await page.locator('vite-error-overlay').count(), 0);
        await page.locator('.nav-list').getByRole('button', { name: 'Privacy', exact: true }).click();
        const board = page.locator('.privacy-board');
        await board.getByRole('button', { name: lane, exact: true }).click();
        assert.equal(await board.getByRole('button', { name: lane, exact: true }).getAttribute('aria-pressed'), 'true');
        assert.match(await board.innerText(), /Privacy & Context/);
        if (process.env.INJECT_HIDDEN) await page.addStyleTag({ content: '.privacy-permission-list { height: 0; overflow: hidden !important; }' });
        const b = await board.boundingBox(); await page.mouse.move(Math.min(width - 12, b.x + b.width / 2), height - 16);
        await page.keyboard.press('Control+Home'); await page.waitForTimeout(150);
        if (process.env.CAPTURE_DIR && lane === 'macOS' && theme === 'light' && width === 900) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/privacy-light-900-start.png`, animations: 'disabled' });
        const seen = new Set(), expected = new Set(), defects = new Set();
        for (let step = 0; step < 45; step++) {
          const scan = await board.evaluate(root => {
            const out = { seen: [], expected: [], defects: [] }, walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT); let node, i = 0;
            while ((node = walker.nextNode())) {
              if (!node.textContent.trim()) continue;
              const el = node.parentElement, style = getComputedStyle(el), range = document.createRange(); range.selectNodeContents(node);
              const rects = [...range.getClientRects()].filter(r => r.width && r.height); i++;
              if (!rects.length || style.visibility !== 'visible') continue;
              if (parseFloat(style.fontSize) < 10.99) out.defects.push(`font ${style.fontSize}: ${node.textContent.trim()}`);
              rects.forEach((r, j) => {
                const key = `${i}:${j}`; out.expected.push(key); let scrollable = false, visible = r.top >= 0 && r.bottom <= innerHeight && r.left >= 0 && r.right <= innerWidth;
                const panel = el.closest('.privacy-panel, .privacy-constitution, .privacy-heading, .privacy-footer');
                if (panel) { const box = panel.getBoundingClientRect(); if (r.left < box.left - 1 || r.right > box.right + 1 || r.top < box.top - 1 || r.bottom > box.bottom + 1) out.defects.push(`section escape: ${node.textContent.trim()}`); }
                for (let p = el; p && p !== document.body; p = p.parentElement) {
                  const s = getComputedStyle(p), box = p.getBoundingClientRect(), outside = r.left < box.left - 1 || r.right > box.right + 1 || r.top < box.top - 1 || r.bottom > box.bottom + 1;
                  if (!scrollable && (/(hidden|clip)/.test(s.overflowX) && (r.left < box.left - 1 || r.right > box.right + 1) || /(hidden|clip)/.test(s.overflowY) && (r.top < box.top - 1 || r.bottom > box.bottom + 1))) out.defects.push(`clipped: ${node.textContent.trim()}`);
                  if (/(auto|scroll|hidden|clip)/.test(`${s.overflowX} ${s.overflowY}`) && outside) visible = false;
                  if (/(auto|scroll)/.test(s.overflowY) && p.scrollHeight > p.clientHeight) scrollable = true;
                }
                if (visible) out.seen.push(key);
              });
            }
            return out;
          });
          for (const key of scan.expected) expected.add(key); for (const key of scan.seen) seen.add(key); for (const defect of scan.defects) defects.add(defect);
          if (defects.size || (step && seen.size === expected.size)) break;
          await page.mouse.wheel(0, Math.floor(height * .55)); await page.waitForTimeout(35);
        }
        if (process.env.CAPTURE_DIR && lane === 'macOS' && [501, 900].includes(width)) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/${process.env.CAPTURE_PREFIX || 'privacy'}-${theme}-${width}-end.png`, animations: 'disabled' });
        assert.deepEqual([...defects], [], 'No under-floor text or hidden-section clipping');
        assert.equal(seen.size, expected.size, 'Every complete text fragment reached by actual wheel input');
        assert(await board.locator('.privacy-main-grid').evaluate(el => el.getBoundingClientRect().height > 0), 'Main content cannot collapse');
        assert(await board.evaluate(el => [...el.children].every((child, i, all) => !i || all[i - 1].getBoundingClientRect().bottom <= child.getBoundingClientRect().top + 1)), 'Main sections must not overlap');
        for (const selector of ['.privacy-reader-row small', '.privacy-permission-row small', '.privacy-constitution article small']) assert(await board.locator(selector).first().isVisible(), 'Policy details stay rendered');
        assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'No horizontal document overflow');
        await board.getByRole('button', { name: lane, exact: true }).click();
        for (let i = 0; i < 3 - ['macOS', 'Windows', 'Linux'].indexOf(lane); i++) await page.keyboard.press('Tab');
        const opener = board.getByRole('button', { name: 'Privacy Constitution', exact: true });
        assert(await opener.evaluate(el => el === document.activeElement), 'Tab reaches Constitution'); await page.keyboard.press('Enter');
        const modal = page.getByRole('dialog'); assert(await modal.isVisible());
        assert(await modal.evaluate(el => [...el.querySelectorAll('*')].filter(n => [...n.childNodes].some(c => c.nodeType === Node.TEXT_NODE && c.textContent.trim())).every(n => parseFloat(getComputedStyle(n).fontSize) >= 10.99)), 'Dialog text floor');
        await page.keyboard.press('Tab'); assert(await modal.evaluate(el => el.contains(document.activeElement)));
        await page.keyboard.press('Escape'); assert.equal(await modal.count(), 0); assert(await opener.evaluate(el => el === document.activeElement));
        await page.keyboard.press('Tab'); const setup = board.getByRole('button', { name: 'Review in Setup' });
        assert(await setup.evaluate(el => el === document.activeElement));
        const focused = await setup.boundingBox(); assert(focused.y >= 0 && focused.y + focused.height <= height, 'Keyboard scroll exposes Setup action');
        await page.keyboard.press('Tab');
        const history = board.getByRole('button', { name: 'View History' });
        assert(await history.evaluate(el => el === document.activeElement), 'Tab reaches History');
        assert(await history.evaluate(el => {
          const r = el.getBoundingClientRect(), s = getComputedStyle(el);
          const ring = Math.max(0, parseFloat(s.outlineWidth) + parseFloat(s.outlineOffset));
          let top = 0, bottom = innerHeight, left = 0, right = innerWidth;
          for (let p = el.parentElement; p; p = p.parentElement) {
            const b = p.getBoundingClientRect(), style = getComputedStyle(p);
            if (/auto|scroll|hidden|clip/.test(style.overflowY)) { top = Math.max(top, b.top); bottom = Math.min(bottom, b.bottom); }
            if (/auto|scroll|hidden|clip/.test(style.overflowX)) { left = Math.max(left, b.left); right = Math.min(right, b.right); }
          }
          return s.outlineStyle !== 'none' && r.top - ring >= top && r.bottom + ring <= bottom && r.left - ring >= left && r.right + ring <= right;
        }), 'Complete History focus ring must stay visible');
        await page.keyboard.press('Shift+Tab');
        assert(await setup.evaluate(el => el === document.activeElement));
        await page.keyboard.press('Enter'); assert(await page.getByRole('button', { name: 'Cancel', exact: true }).isVisible());
        assert.deepEqual(errors, []); results.push({ theme, width, lane, status: 'PASS', textFragments: seen.size });
      } catch (e) { results.push({ theme, width, lane, status: 'FAIL', reason: e.message }); }
      finally { await page.close(); }
    }
  } finally { await browser.close(); }
  console.log(JSON.stringify({ scope: '3 OS reference presentations; 450x300 is CSS 200%-equivalent only', results }, null, 2));
  if (results.some(r => r.status === 'FAIL')) process.exitCode = 1;
})().catch(e => { console.error(e); process.exitCode = 1; });
