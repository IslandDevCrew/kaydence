// Controlled IPC fixtures exercise presentation branches, never native OS proof.
const assert = require('node:assert/strict'), fs = require('node:fs'), vm = require('node:vm');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const source = fs.readFileSync('apps/desktop/src/App.tsx', 'utf8');
function literal(pattern) { const m = source.match(pattern); assert(m, 'Review changed fixture declaration'); return vm.runInNewContext(`(${m[1]})`, {}, { timeout: 1000 }); }
const base = literal(/const previewSnapshot: AppSnapshot = ([\s\S]*?\n});/);
const historyBase = literal(/const previewHistory: HistorySession\[\] = ([\s\S]*?\n\]);/);
const lanes = [
  { id: 'mac', label: 'macOS', accent: '#14a7a1', method: 'AX native insert', permission: 'Accessibility' },
  { id: 'windows', label: 'Windows', accent: '#0067c0', method: 'UI Automation + SendInput', permission: 'UI Automation' },
  { id: 'linux', label: 'Linux', accent: '#1f9d63', method: 'AT-SPI detect + uinput', permission: 'AT-SPI / X11' },
];
(async () => {
  const browser = await chromium.launch(), errors = [];
  try {
    for (const os of lanes) for (const state of ['ready', 'blocked', 'held', 'failed', 'failed-empty']) {
      const snapshot = structuredClone(base), history = structuredClone(historyBase.slice(0, 1));
      snapshot.settings.first_run.permission_requirements = [{ id: 'test-permission', label: os.permission, state: state === 'blocked' ? 'blocked' : 'ready', detail: 'Controlled fixture', action: 'None', action_label: 'Review' }];
      if (state === 'blocked') history.length = 0;
      if (state === 'held') history[0].held_reason = 'secure_field';
      if (state === 'failed') history[0].failure = { stage: 'inject', error: 'Controlled injection failure' };
      if (state === 'failed-empty') history[0].failure = { stage: 'inject', error: '' };
      const page = await browser.newPage({ viewport: { width: 900, height: 600 }, reducedMotion: 'reduce', userAgent: `Mozilla/5.0 (${os.id}) AppleWebKit/537.36 Chrome/145.0.0.0 Safari/537.36` });
      page.on('pageerror', e => errors.push(e.message));
      page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
      await page.addInitScript(({ snapshot, history }) => {
        window.isTauri = true;
        window.__TAURI_INTERNALS__ = { invoke: async command => {
          if (command === 'app_snapshot') return snapshot;
          if (command === 'recent_history') return history;
          throw new Error(`Unexpected fixture IPC: ${command}`);
        } };
      }, { snapshot, history });
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1428');
      const nav = page.locator('.nav-list');
      await nav.getByRole('button', { name: 'Cleanup', exact: true }).click();
      const rows = page.locator('.cleanup-gates-section > div:not(.cleanup-status-legend)');
      assert.equal(await rows.count(), 4);
      for (const index of [1, 3]) {
        assert.equal(await rows.nth(index).locator('.ready').count(), 0, 'Secure/round-trip proof must not be invented');
        assert.match(await rows.nth(index).textContent(), /not measured|pending/i);
      }
      if (state !== 'ready') assert.equal(await rows.nth(2).locator('.ready').count(), 0, 'Hold/failure/empty history is not delivered');
      if (state === 'blocked') assert.equal(await rows.locator('.ready').count(), 0, 'Blocked runtime has no passing gate');
      if (state === 'failed-empty') assert.match(await page.locator('.cleanup-health-section').textContent(), /Issue.*Delivery failed/s);
      if (state === 'ready') {
        assert.match(await rows.nth(2).textContent(), /Recorded/);
        for (const view of ['Cleanup', 'Privacy', 'Setup']) for (const ref of lanes.filter(lane => lane.id !== os.id)) {
          await nav.getByRole('button', { name: view, exact: true }).click();
          await page.locator('[aria-label="Operating system lane"]').getByRole('button', { name: ref.label, exact: true }).click();
          if (view === 'Cleanup') {
            assert.equal(await rows.locator('.ready').count(), 0, 'Reference lane cannot inherit host proof');
            assert.match(await page.locator('.cleanup-method-section').textContent(), /Reference/);
            assert.doesNotMatch(await page.locator('.cleanup-health-section').textContent(), /proved|recorded/i);
            if (process.env.CAPTURE_DIR) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/${os.id}-reference-${ref.id}.png`, animations: 'disabled' });
          }
          if (view === 'Setup') await page.getByRole('button', { name: 'Cancel', exact: true }).click();
          else await nav.getByRole('button', { name: 'Dictate', exact: true }).click();
          assert.equal(await page.locator('.dictate-shell').evaluate(el => el.style.getPropertyValue('--accent')), os.accent, 'Runtime accent cannot inherit reference selection');
          assert.match(await page.locator('.cockpit-sb-left').textContent(), new RegExp(os.label));
          assert((await page.locator('.cockpit-output-panel').textContent()).includes(os.method), 'Runtime injection description');
        }
      }
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      console.log(`PASS ${os.id} ${state}: runtime/reference proof boundaries`);
      await page.close();
    }
    const fixture = await browser.newPage({ viewport: { width: 450, height: 300 }, colorScheme: 'dark' });
    await fixture.goto(process.env.APP_URL || 'http://127.0.0.1:1428');
    await fixture.locator('.nav-list').getByRole('button', { name: 'Cleanup', exact: true }).click();
    assert.equal(await fixture.locator('.cleanup-gates-section .ready').count(), 0, 'Browser fixture is not OS proof');
    assert.match(await fixture.locator('.cleanup-health-section').textContent(), /Sample data/);
    assert(await fixture.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    if (process.env.CAPTURE_DIR) await fixture.screenshot({ path: `${process.env.CAPTURE_DIR}/fixture-dark-450.png`, animations: 'disabled' });
    await fixture.close();
    console.log('PASS dark 450x300: browser fixture is not OS proof');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
