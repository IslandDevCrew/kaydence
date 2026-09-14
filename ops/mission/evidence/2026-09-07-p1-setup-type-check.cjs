const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
async function naturalFocusWalk(page, scope, issues) {
  for (let step = 0; step < 40; step++) {
    await page.keyboard.press('Tab');
    const failure = await page.evaluate(selector => {
      const el = document.activeElement, root = document.querySelector(selector);
      if (!root?.contains(el)) return selector.startsWith('dialog') ? 'focus escaped scope' : 'page cycle complete';
      const r = el.getBoundingClientRect(), s = getComputedStyle(el);
      const ring = Math.max(0, parseFloat(s.outlineWidth) + parseFloat(s.outlineOffset));
      let top = 0, bottom = innerHeight, left = 0, right = innerWidth;
      for (let p = el.parentElement; p; p = p.parentElement) {
        const b = p.getBoundingClientRect(), css = getComputedStyle(p);
        if (/auto|scroll|hidden|clip/.test(css.overflowY)) { top = Math.max(top, b.top); bottom = Math.min(bottom, b.bottom); }
        if (/auto|scroll|hidden|clip/.test(css.overflowX)) { left = Math.max(left, b.left); right = Math.min(right, b.right); }
        if (p.matches('dialog[open]')) break;
      }
      return s.outlineStyle === 'none' || r.top - ring < top || r.bottom + ring > bottom || r.left - ring < left || r.right + ring > right ? `natural focus/ring clipped: ${el.getAttribute('aria-label') || el.className || el.textContent} ${JSON.stringify({ rect: [r.top, r.bottom, r.left, r.right], clip: [top, bottom, left, right], ring, outline: s.outlineStyle })}` : null;
    }, scope);
    if (failure === 'page cycle complete') break;
    if (failure) issues.push(failure);
  }
}
async function inspect(root) {
  return root.evaluate(board => {
    const issues = [], walker = document.createTreeWalker(board, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const node = walker.currentNode, el = node.parentElement, text = node.textContent.trim();
      if (!text || !el.getClientRects().length) continue;
      if (parseFloat(getComputedStyle(el).fontSize) < 11) issues.push(`sub-11px: ${text}`);
      el.scrollIntoView({ block: 'center', inline: 'nearest' });
      const range = document.createRange(); range.selectNodeContents(node);
      for (const r of range.getClientRects()) {
        if (r.left < -1 || r.right > innerWidth + 1 || r.top < -1 || r.bottom > innerHeight + 1) issues.push(`unreachable text: ${text}`);
        for (let p = el; p; p = p.parentElement) {
          const s = getComputedStyle(p), b = p.getBoundingClientRect();
          if (/hidden|clip|auto|scroll/.test(s.overflowX) && (r.left < b.left - 1 || r.right > b.right + 1)) issues.push(`horizontal clipping: ${text}`);
          if (/hidden|clip|auto|scroll/.test(s.overflowY) && (r.top < b.top - 1 || r.bottom > b.bottom + 1)) issues.push(`vertical clipping: ${text}`);
          if (/hidden|clip/.test(s.overflowY) && p.scrollHeight > p.clientHeight + 1) issues.push(`hidden overflow: ${text}`);
          if (p.matches('.first-run-control-row, .first-run-license-card, .first-run-step-rail li, .first-run-evidence-body > *') && (r.top < b.top - 1 || r.bottom > b.bottom + 1 || r.left < b.left - 1 || r.right > b.right + 1)) issues.push(`text escapes row/card: ${text}`);
        }
      }
    }
    for (const parent of board.querySelectorAll('.first-run-controls, .first-run-step-rail, .first-run-evidence-body')) {
      const boxes = [...parent.children].map(el => el.getBoundingClientRect());
      for (let i = 1; i < boxes.length; i++) if (boxes[i].top < boxes[i - 1].bottom - 1) issues.push('section overlap');
    }
    const ctx = document.createElement('canvas').getContext('2d');
    for (const select of board.querySelectorAll('select')) {
      const s = getComputedStyle(select); ctx.font = s.font;
      if (ctx.measureText(select.selectedOptions[0].textContent).width > select.clientWidth - parseFloat(s.paddingLeft) - parseFloat(s.paddingRight)) issues.push(`selected value clipped: ${select.getAttribute('aria-label')}`);
    }
    if (document.documentElement.scrollHeight > innerHeight + 1 || document.documentElement.scrollWidth > innerWidth + 1) issues.push('Setup leaks document overflow');
    return [...new Set(issues)];
  });
}
(async () => {
  const browser = await chromium.launch(), failures = [], errors = [];
  try {
    for (const theme of ['light', 'dark']) for (const width of [900, 450, 500, 501]) {
      const height = width === 900 ? 600 : 300;
      const page = await browser.newPage({ viewport: { width, height }, colorScheme: theme, reducedMotion: 'reduce' });
      page.on('pageerror', e => errors.push(e.message));
      page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1435');
      assert.equal(await page.title(), 'Kaydence'); assert.equal(await page.locator('vite-error-overlay').count(), 0);
      await page.locator('.nav-list').getByRole('button', { name: 'Setup', exact: true }).click();
      for (const lane of ['macOS', 'Windows', 'Linux']) {
        await page.locator('.first-run-lanes').getByRole('button', { name: lane, exact: true }).click();
        const issues = await inspect(page.locator('.first-run-board'));
        await page.locator('.first-run-lanes').getByRole('button', { name: lane, exact: true }).click();
        await naturalFocusWalk(page, '.first-run-board', issues);
        await page.keyboard.press('Tab');
        for (const control of await page.locator('.first-run-board button:enabled, .first-run-board select:enabled, .first-run-board input:enabled').all()) {
          await control.focus();
          if (!await control.evaluate(el => { const s = getComputedStyle(el), b = el.getBoundingClientRect(), ring = parseFloat(s.outlineWidth) + parseFloat(s.outlineOffset); return el === document.activeElement && s.outlineStyle !== 'none' && b.left - ring >= 0 && b.right + ring <= innerWidth && b.top - ring >= 0 && b.bottom + ring <= innerHeight; })) issues.push(`Control focus/ring clipped: ${await control.getAttribute('aria-label') || await control.textContent()}`);
        }
        for (const selector of ['.first-run-next p', '.first-run-next > span', '.first-run-hotkey-control > div', '.first-run-step-rail li small']) if (!await page.locator(selector).isVisible()) issues.push(`hidden required content: ${selector}`);
        for (const selector of ['.first-run-workspace', '.first-run-step-rail']) {
          const area = page.locator(selector);
          if (await area.evaluate(el => el.scrollHeight > el.clientHeight)) {
            if (!await area.evaluate(el => /auto|scroll/.test(getComputedStyle(el).overflowY))) { issues.push(`not user-scrollable: ${selector}`); continue; }
            await area.evaluate(el => { el.scrollTop = 0; }); await area.hover(); await page.mouse.wheel(0, 4000);
            await page.waitForFunction(s => document.querySelector(s).scrollTop > 0, selector);
          }
        }
        const cancel = page.getByRole('button', { name: 'Cancel', exact: true }); await cancel.focus();
        const box = await cancel.boundingBox();
        if (!box || box.y < 0 || box.y + box.height > height) issues.push('Cancel clipped');
        if (lane === 'macOS' && process.env.CAPTURE_DIR) {
          if ([900, 450].includes(width)) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/setup-${theme}-${width}-bottom.png`, animations: 'disabled' });
          await page.locator('.first-run-board').evaluate(el => { for (const node of [el, ...el.querySelectorAll('*')]) node.scrollTop = 0; scrollTo(0, 0); });
          await page.screenshot({ path: `${process.env.CAPTURE_DIR}/setup-${theme}-${width}.png`, animations: 'disabled' });
        }
        const evidence = page.getByRole('button', { name: 'Evidence', exact: true }); await evidence.click();
        const modal = page.getByRole('dialog');
        for (const section of ['permissions', 'models', 'proof']) {
          await modal.getByRole('button', { name: section, exact: true }).click();
          issues.push(...(await inspect(modal.locator('.first-run-dialog'))).map(s => `${section}: ${s}`));
          await modal.getByRole('button', { name: section, exact: true }).click();
          await naturalFocusWalk(page, 'dialog[open]', issues);
        }
        for (const key of ['Shift+Tab', ...Array(10).fill('Tab')]) { await page.keyboard.press(key); assert(await modal.evaluate(el => el.contains(document.activeElement)), 'Modal focus containment'); }
        const close = modal.getByRole('button', { name: 'Close setup evidence' }); await close.focus();
        const closeBox = await close.boundingBox();
        assert(closeBox && closeBox.y >= 0 && closeBox.y + closeBox.height <= height, 'Visible Close');
        if (lane === 'macOS' && [900, 450].includes(width) && process.env.CAPTURE_DIR) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/setup-${theme}-${width}-proof.png`, animations: 'disabled' });
        await page.keyboard.press('Escape'); assert.equal(await page.getByRole('dialog').count(), 0);
        assert(await evidence.evaluate(el => document.activeElement === el), 'Focus returned to Evidence');
        if (issues.length) failures.push({ theme, width, lane, issues: [...new Set(issues)] });
        else console.log(`PASS ${theme} ${width}x${height} ${lane}: 11px body/dialog, full text, real scrolling, modal and actions`);
      }
      await page.getByRole('button', { name: 'Cancel', exact: true }).focus(); await page.keyboard.press('Enter');
      assert.equal(await page.locator('.nav-list [aria-current="page"] .nav-item-label').textContent(), 'Dictate', 'Cancel returns to Dictate');
      await page.close();
    }
    assert.deepEqual(errors, [], 'No frontend errors'); assert.deepEqual(failures, [], 'Setup readability');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
