// Browser fixture only. Exercise real keyboard/wheel; never force focus or scroll in tests.
const assert = require('node:assert/strict'), fs = require('node:fs'), cp = require('node:child_process');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const styleRef = process.env.PRIVACY_STYLE_REF;
const privacyStyle = styleRef ? cp.execFileSync('git', ['show', `${styleRef}:apps/desktop/src/views/PrivacyView.css`], { encoding: 'utf8' }) : null;
async function reach(page, name) {
  for (let i = 0; i < 50; i++) {
    await page.keyboard.press('Tab');
    await settle(page, page.locator('body'), true);
    if (await page.evaluate(n => document.activeElement?.textContent.trim() === n, name)) return;
  }
  assert.fail(`Keyboard did not reach ${name}`);
}
async function visibleFocus(page, inside) {
  await settle(page, page.locator('body'), true);
  const result = await page.evaluate(inside => {
    const el = document.activeElement, r = el.getBoundingClientRect(), s = getComputedStyle(el);
    const extra = Math.max(0, parseFloat(s.outlineWidth) + parseFloat(s.outlineOffset));
    const faults = [];
    if (inside && !el.closest('dialog[open]')) faults.push('outside modal');
    if (s.outlineStyle === 'none' || parseFloat(s.outlineWidth) < 1) faults.push('missing outline');
    if (r.left - extra < 0 || r.top - extra < 0 || r.right + extra > innerWidth || r.bottom + extra > innerHeight) faults.push('viewport clips outline');
    for (let p = el.parentElement; p; p = p.parentElement) {
      const c = getComputedStyle(p), b = p.getBoundingClientRect();
      if (/(auto|scroll|hidden|clip)/.test(c.overflowY) && (r.top - extra < b.top - .5 || r.bottom + extra > b.bottom + .5)) faults.push(`vertical clip: ${p.className}`);
      if (/(auto|scroll|hidden|clip)/.test(c.overflowX) && (r.left - extra < b.left - .5 || r.right + extra > b.right + .5)) faults.push(`horizontal clip: ${p.className}`);
      if (p.tagName === 'DIALOG' && p.open) break;
    }
    const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
    if (!hit || !(el === hit || el.contains(hit))) faults.push('focus occluded');
    return { name: el.textContent.trim(), faults, bounds: r.toJSON() };
  }, inside);
  assert.deepEqual(result.faults, [], JSON.stringify(result));
  return result.name;
}
async function settle(page, root, active = false, quietMs = 0) {
  await page.waitForTimeout(80); // Allow native wheel/focus scrolling before observing settlement.
  await root.evaluate(async (el, { active, quietMs }) => {
    const parents = []; for (let p = active ? document.activeElement : el; p; p = p.parentElement) parents.push(p);
    let previous = '', stable = 0, changed = performance.now();
    for (let i = 0; i < 120; i++) {
      await new Promise(requestAnimationFrame);
      const position = parents.map(p => `${p.scrollTop},${p.scrollLeft}`).join(';');
      if (position !== previous) changed = performance.now();
      stable = position === previous ? stable + 1 : 0; previous = position;
      if (stable >= 4 && performance.now() - changed >= quietMs) return;
    }
    throw Error('Wheel scrolling did not settle');
  }, { active, quietMs });
}
async function wheelBottom(page, root, delta = 10000) {
  const b = await root.boundingBox();
  await page.mouse.move(Math.min(page.viewportSize().width - 20, b.x + b.width / 2), Math.min(page.viewportSize().height - 25, b.y + b.height / 2));
  const target = await root.elementHandle();
  await target.evaluate(el => {
    el.__modalWheelProbe = { received: false, listener: () => { el.__modalWheelProbe.received = true; } };
    el.addEventListener('wheel', el.__modalWheelProbe.listener, { once: true });
  });
  try {
    await page.mouse.wheel(0, delta);
    await page.waitForFunction(el => el.__modalWheelProbe.received, target, { timeout: 2000 });
  } finally {
    await target.evaluate(el => { el.removeEventListener('wheel', el.__modalWheelProbe.listener); delete el.__modalWheelProbe; });
    await target.dispose();
  }
  // Separate opposite synthetic gestures; four unchanged frames can retain the prior wheel target.
  await settle(page, root, false, 300);
  return root.evaluate(el => ({ top: el.scrollTop, max: el.scrollHeight - el.clientHeight }));
}
(async () => {
  const browser = await chromium.launch(), results = [], errors = [];
  try {
    for (const theme of process.env.THEME ? [process.env.THEME] : ['light', 'dark']) for (const width of process.env.WIDTH ? [+process.env.WIDTH] : [450, 501, 900]) for (const view of process.env.VIEW ? [process.env.VIEW] : ['Privacy', 'Setup']) {
      const page = await browser.newPage({ viewport: { width, height: width === 900 ? 600 : 300 }, colorScheme: theme, reducedMotion: 'reduce' });
      page.on('pageerror', e => errors.push(e.message)); page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
      let stage = 'entry';
      try {
        await page.goto(process.env.APP_URL || 'http://127.0.0.1:1438');
        assert.equal(await page.title(), 'Kaydence'); assert.equal(await page.locator('vite-error-overlay').count(), 0);
        if (privacyStyle && view === 'Privacy') await page.evaluate(bodyStyle => {
          const sheet = [...document.querySelectorAll('style[data-vite-dev-id]')].find(s => s.dataset.viteDevId.endsWith('/views/PrivacyView.css'));
          if (!sheet) throw Error('Expected served Privacy stylesheet');
          const margin = sheet.textContent.match(/\.privacy-heading > button \{[^}]*\}/)?.[0] || '';
          const boundary = '\n.privacy-board {';
          // Replace body declarations (including deletions); preserve actual rail and opener fix.
          sheet.textContent = sheet.textContent.slice(0, sheet.textContent.indexOf(boundary))
            + bodyStyle.slice(bodyStyle.indexOf(boundary)) + '\n' + margin;
        }, privacyStyle);
        await reach(page, view); await page.keyboard.press('Enter');
        const opener = view === 'Privacy' ? 'Privacy Constitution' : 'Evidence';
        await wheelBottom(page, page.locator(view === 'Privacy' ? '.privacy-board' : '.first-run-board'));
        await reach(page, opener); stage = 'opener'; await visibleFocus(page, false);
        await page.keyboard.press('Enter'); stage = 'initial focus';
        const modal = page.getByRole('dialog'); assert(await modal.isVisible());
        assert.equal(await visibleFocus(page, true), 'Close');
        const scroll = page.locator(view === 'Privacy' ? '.privacy-dialog' : '.first-run-evidence-body');
        const background = () => modal.evaluate(el => {
          const positions = []; for (let p = el.parentElement; p; p = p.parentElement) positions.push([p.scrollTop, p.scrollLeft]); return positions;
        });
        const behind = await background(); stage = 'wheel at upper boundary';
        await wheelBottom(page, scroll, -10000);
        assert.deepEqual(await background(), behind, 'Modal wheel must not scroll inert background');
        stage = 'wheel then Tab'; const wheel = await wheelBottom(page, scroll);
        assert.deepEqual(await background(), behind, 'Modal lower boundary must not scroll inert background');
        assert(wheel.max <= 0 || wheel.top > 0, 'Real wheel scrolls overflowing modal content');
        await page.keyboard.press('Tab'); await visibleFocus(page, true);
        if (process.env.CAPTURE_DIR && theme === 'light' && width === 450 && (!privacyStyle || view === 'Privacy')) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/${view.toLowerCase()}-after-wheel.png` });
        await page.locator(view === 'Privacy' ? '.privacy-dialog dt' : '.first-run-evidence-summary strong').first().click();
        stage = 'content click then reverse'; await page.keyboard.press('Shift+Tab'); await visibleFocus(page, true);
        for (const key of ['Shift+Tab', ...Array(18).fill('Tab'), ...Array(18).fill('Shift+Tab')]) {
          stage = key; await page.keyboard.press(key); await visibleFocus(page, true);
        }
        if (view === 'Setup') for (const section of ['models', 'permissions', 'proof']) {
          await modal.getByRole('button', { name: section, exact: true }).click();
          await wheelBottom(page, page.locator('.first-run-evidence-body'));
          for (let i = 0; i < 18; i++) { await page.keyboard.press(i % 2 ? 'Shift+Tab' : 'Tab'); await visibleFocus(page, true); }
        }
        stage = 'Escape restore'; await page.keyboard.press('Escape');
        assert.equal(await page.getByRole('dialog').count(), 0); assert.equal(await visibleFocus(page, false), opener);
        if (process.env.CAPTURE_DIR && (!privacyStyle || view === 'Privacy')) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/${view.toLowerCase()}-${theme}-${width}.png` });
        stage = 'reopen reverse'; await page.keyboard.press('Enter'); await page.keyboard.press('Shift+Tab'); await visibleFocus(page, true);
        await reach(page, 'Close'); stage = 'Close restore'; await page.keyboard.press('Enter');
        assert.equal(await page.getByRole('dialog').count(), 0); assert.equal(await visibleFocus(page, false), opener);
        stage = 'backdrop'; await page.keyboard.press('Enter'); await page.mouse.click(2, 2);
        assert.equal(await page.getByRole('dialog').count(), 0); assert.equal(await page.evaluate(() => document.activeElement.textContent.trim()), opener);
        results.push({ theme, width, view, status: 'PASS', wheel });
      } catch (e) {
        results.push({ theme, width, view, status: 'FAIL', stage, error: e.message });
        if (process.env.CAPTURE_DIR) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/${view.toLowerCase()}-${theme}-${width}-failure.png` });
      } finally { await page.close(); }
    }
  } finally { await browser.close(); }
  console.log(JSON.stringify({ scope: 'Fixture keyboard/wheel only', privacyStyleRef: styleRef || null, results, errors }, null, 2));
  if (process.env.RESULT_PATH) fs.writeFileSync(process.env.RESULT_PATH, JSON.stringify({ results, errors }, null, 2));
  assert.deepEqual(errors, []); assert(results.every(r => r.status === 'PASS'));
})().catch(e => { console.error(e); process.exitCode = 1; });
