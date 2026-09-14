// Dismiss while permission IPC is pending: existing Evidence is the logical fallback.
const fs = require('node:fs'), vm = require('node:vm'), assert = require('node:assert/strict');
const { chromium, webkit } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const source = fs.readFileSync('apps/desktop/src/App.tsx', 'utf8');
const match = source.match(/const previewSnapshot: AppSnapshot = ([\s\S]*?\n});/); assert(match);
const base = JSON.parse(JSON.stringify(vm.runInNewContext(`(${match[1]})`, {}, { timeout: 1000 })));
const out = process.env.CAPTURE_DIR; assert(out); fs.mkdirSync(out, { recursive: true });
function focusBounds(el) {
  const r = el.getBoundingClientRect(), c = getComputedStyle(el), ring = c.outlineStyle === 'none' ? 0 : Math.max(0, parseFloat(c.outlineWidth) + parseFloat(c.outlineOffset));
  const b = { left: 0, top: 0, right: innerWidth, bottom: innerHeight };
  for (let p = el.parentElement; p; p = p.parentElement) {
    const s = getComputedStyle(p), a = p.getBoundingClientRect();
    if (/(auto|scroll|hidden|clip)/.test(s.overflowY)) { b.top = Math.max(b.top, a.top); b.bottom = Math.min(b.bottom, a.bottom); }
    if (/(auto|scroll|hidden|clip)/.test(s.overflowX)) { b.left = Math.max(b.left, a.left); b.right = Math.min(b.right, a.right); }
  }
  return { focused: document.activeElement === el, active: document.activeElement.tagName, rect: r.toJSON(), ring, bounds: b,
    visible: r.width > 0 && r.height > 0 && r.top - ring >= b.top - .1 && r.bottom + ring <= b.bottom + .1 && r.left - ring >= b.left - .1 && r.right + ring <= b.right + .1 };
}
(async () => {
  const results = [];
  for (const [engine, launcher] of Object.entries({ chromium, webkit })) {
    const browser = await launcher.launch();
    try { for (const theme of ['light', 'dark']) for (const width of [900, 450, 501]) for (const method of ['Escape', 'Close']) {
      const page = await browser.newPage({ viewport: { width, height: width === 900 ? 600 : 300 }, colorScheme: theme, reducedMotion: 'reduce' }), snapshot = structuredClone(base), calls = [], errors = [];
      page.setDefaultTimeout(5000); page.on('pageerror', e => errors.push(e.message));
      page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
      snapshot.settings.first_run.next_step = { ...snapshot.settings.first_run.next_step, kind: 'permission', target_id: 'microphone', action_label: 'Review next step' };
      let release; const pending = new Promise(resolve => { release = resolve; });
      await page.exposeFunction('__pendingFixture', async (command, args) => {
        if (command === 'app_snapshot') return snapshot;
        if (command === 'recent_history') return [];
        calls.push({ command, args }); assert.equal(command, 'first_run_permission_action');
        await pending; return { requirement_id: 'microphone', label: 'Microphone', state: 'needs_review', settings_target: null, manual_step: 'Controlled fixture', proof_requirement: 'Not native proof' };
      });
      await page.addInitScript(() => { window.isTauri = true; window.__TAURI_INTERNALS__ = { invoke: (command, args) => window.__pendingFixture(command, args) }; });
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1440');
      await page.locator('.nav-list').getByRole('button', { name: 'Setup', exact: true }).click();
      const opener = page.locator('.first-run-footer > button.primary'); await opener.click();
      const fallback = await page.locator('.first-run-next > button').elementHandle();
      await page.getByRole('dialog').waitFor(); assert(await opener.isDisabled());
      if (method === 'Escape') await page.keyboard.press('Escape'); else await page.getByRole('dialog').getByRole('button', { name: /Close/ }).click();
      await page.getByRole('dialog').waitFor({ state: 'detached' });
      const returned = await opener.evaluate(el => ({ openerFocused: document.activeElement === el, disabled: el.disabled, active: document.activeElement.tagName }));
      await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      const fallbackReturn = await fallback.evaluate(focusBounds);
      release(); await page.waitForFunction(() => !document.querySelector('.first-run-footer > button.primary').disabled);
      const settled = await opener.evaluate(el => ({ openerFocused: document.activeElement === el, active: document.activeElement.tagName }));
      const fallbackSettled = await fallback.evaluate(focusBounds);
      const pass = returned.disabled && !returned.openerFocused && !settled.openerFocused && fallbackReturn.focused && fallbackReturn.visible && fallbackSettled.focused && fallbackSettled.visible && !errors.length;
      assert.deepEqual(calls, [{ command: 'first_run_permission_action', args: { requirementId: 'microphone' } }]);
      results.push({ engine, version: browser.version(), theme, width, method, calls, returned, settled, fallbackReturn, fallbackSettled, pass, errors });
      if (theme === 'dark' && width === 450 && method === 'Escape') await page.screenshot({ path: `${out}/${engine}-fallback.png` });
      console.log(`${pass ? 'PASS' : 'FAIL'} ${engine} ${theme} ${width} ${method}`); await page.close();
    } } finally { await browser.close(); }
  }
  fs.writeFileSync(`${out}/results.json`, JSON.stringify(results, null, 2)); console.log(JSON.stringify({ cases: results.length, passed: results.filter(row => row.pass).length }));
  if (results.some(row => !row.pass)) process.exitCode = 1;
})().catch(error => { console.error(error); process.exitCode = 1; });
