const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
(async () => {
  const browser = await chromium.launch();
  const failures = [], errors = [];
  try {
    for (const theme of ['light', 'dark']) for (const width of [900, 450, 500, 501]) {
      const height = width === 900 ? 600 : 300;
      const page = await browser.newPage({ viewport: { width, height }, colorScheme: theme, reducedMotion: 'reduce' });
      page.on('pageerror', error => errors.push(error.message));
      page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1432');
      assert.equal(await page.title(), 'Kaydence');
      assert.equal(await page.locator('vite-error-overlay').count(), 0);
      await page.locator('.nav-list').getByRole('button', { name: 'Cleanup', exact: true }).click();
      for (const lane of ['macOS', 'Windows', 'Linux']) {
        await page.locator('.cleanup-lane-switcher').getByRole('button', { name: lane, exact: true }).click();
        const issues = await page.locator('.cleanup-board').evaluate(board => {
          const issues = [], rectangles = [];
          const walker = document.createTreeWalker(board, NodeFilter.SHOW_TEXT);
          const clipped = rect => {
            for (let node = rect.node; node; node = node.parentElement) {
              const box = node.getBoundingClientRect(), style = getComputedStyle(node);
              if (/hidden|clip|auto|scroll/.test(style.overflowX) && (rect.left < box.left - 1 || rect.right > box.right + 1)) return true;
            }
            return rect.left < -1 || rect.right > innerWidth + 1;
          };
          while (walker.nextNode()) {
            const node = walker.currentNode, el = node.parentElement;
            if (!node.textContent.trim() || !el.getClientRects().length) continue;
            const font = parseFloat(getComputedStyle(el).fontSize);
            if (font < 11) issues.push(`font ${font}: ${node.textContent.trim()}`);
            el.scrollIntoView({ block: 'center', inline: 'nearest' });
            const range = document.createRange(); range.selectNodeContents(node);
            for (const rect of range.getClientRects()) {
              if (clipped({ ...rect.toJSON(), node: el })) issues.push(`horizontal text clipping: ${node.textContent.trim()}`);
              if (rect.top < -1 || rect.bottom > innerHeight + 1) issues.push(`unreachable text: ${node.textContent.trim()}`);
              for (let p = el.parentElement; p; p = p.parentElement) {
                const s = getComputedStyle(p), b = p.getBoundingClientRect();
                if (/hidden|clip|auto|scroll/.test(s.overflowY) && (rect.top < b.top - 1 || rect.bottom > b.bottom + 1)) issues.push(`vertical text clipping: ${node.textContent.trim()}`);
                if (/hidden|clip/.test(s.overflowY) && p.scrollHeight > p.clientHeight + 1) issues.push(`hidden overflow is not user-scrollable: ${node.textContent.trim()}`);
                if (p.matches('.cleanup-panel > section') && (rect.top < b.top - 1 || rect.bottom > b.bottom + 1)) issues.push(`text escapes its section: ${node.textContent.trim()}`);
              }
            }
          }
          board.querySelector('.cleanup-status-legend').scrollIntoView({ block: 'center' });
          for (const el of board.querySelectorAll('.cleanup-status-legend > span')) {
            const range = document.createRange(); range.selectNodeContents(el);
            rectangles.push(range.getBoundingClientRect().toJSON());
          }
          for (let i = 0; i < rectangles.length; i++) for (let j = i + 1; j < rectangles.length; j++) {
            const a = rectangles[i], b = rectangles[j];
            if (Math.min(a.right, b.right) - Math.max(a.left, b.left) > 1 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 1) issues.push('status legend overlap');
          }
          for (const panel of board.querySelectorAll('.cleanup-panel')) {
            const sections = [...panel.children].map(el => el.getBoundingClientRect());
            for (let i = 1; i < sections.length; i++) if (sections[i].top < sections[i - 1].bottom - 1) issues.push('panel sections overlap');
          }
          return [...new Set(issues)];
        });
        for (const name of ['Reset to Defaults', 'Save Changes', 'Check permissions']) {
          const button = page.locator('.cleanup-board').getByRole('button', { name, exact: true });
          await button.focus();
          assert(await button.evaluate(el => el === document.activeElement), `${name}: keyboard focus`);
          const b = await button.boundingBox();
          if (!b || b.x < 0 || b.y < 0 || b.x + b.width > width || b.y + b.height > height) issues.push(`${name}: clipped action`);
        }
        if (width === 900) for (const panel of ['.cleanup-rules-panel', '.cleanup-injection-panel']) {
          if (await page.locator(panel).evaluate(el => el.scrollHeight <= el.clientHeight)) continue;
          await page.locator(panel).evaluate(el => { el.scrollTop = 0; });
          await page.locator(panel).hover(); await page.mouse.wheel(0, 2000);
          await page.waitForFunction(selector => document.querySelector(selector).scrollTop > 0, panel);
        }
        if (issues.length) failures.push({ theme, width, lane, issues });
        else console.log(`PASS ${theme} ${width}x${height} ${lane}: 11px floor, full text ranges, legend separation, scroll and actions`);
        if (lane === 'macOS' && process.env.CAPTURE_DIR) {
          await page.locator('.cleanup-status-legend').scrollIntoViewIfNeeded();
          await page.screenshot({ path: `${process.env.CAPTURE_DIR}/cleanup-${theme}-${width}-detail.png`, animations: 'disabled' });
          await page.locator('.cleanup-board').evaluate(board => { for (const el of [board, ...board.querySelectorAll('*')]) el.scrollTop = 0; scrollTo(0, 0); });
          await page.screenshot({ path: `${process.env.CAPTURE_DIR}/cleanup-${theme}-${width}.png`, animations: 'disabled', fullPage: true });
        }
      }
      await page.close();
    }
    assert.deepEqual(errors, [], 'Frontend console and page errors');
    assert.deepEqual(failures, [], 'Cleanup readability findings');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
