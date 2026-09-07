// Browser skill absent: regular Playwright; fixture presentation proof, never native runtime proof.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = process.env.APP_ROOT || process.cwd();
const source = fs.readFileSync(`${root}/apps/desktop/src/App.tsx`, 'utf8');
function literal(pattern) {
  const match = source.match(pattern);
  assert(match, 'Reviewed fixture literal must remain identifiable; do not evaluate application code');
  return JSON.parse(JSON.stringify(vm.runInNewContext(`(${match[1]})`, {}, { timeout: 100 })));
}
const snapshot = literal(/const previewSnapshot: AppSnapshot = ([\s\S]*?\n});/);
const samples = literal(/const previewHistory: HistorySession\[\] = ([\s\S]*?\n\]);/);
const sentinel = 'Native hydration sentinel: retain this genuine test-session payload.';
const history = [{ ...samples[0], id: 'hydration-sentinel', raw_text: sentinel, clean_text: sentinel }];
const results = [];
const options = { viewport: { width: Number(process.env.CHECK_WIDTH || 900), height: Number(process.env.CHECK_HEIGHT || 600) }, colorScheme: process.env.CHECK_THEME || 'light', reducedMotion: 'reduce' };
async function capture(page, name) { if (process.env.CAPTURE_DIR) await page.screenshot({ path: `${process.env.CAPTURE_DIR}/${name}.png`, animations: 'disabled' }); }
const shown = (scope, expression) => scope.getByText(expression).filter({ visible: true }).first().waitFor({ state: 'visible', timeout: 1800 });
const historyIssue = /history.{0,45}(unavailable|failed|could not|unable)|(?:unable|could not).{0,30}(load|refresh).{0,15}history/i;
async function noSamples(page) {
  const text = await page.locator('body').innerText();
  for (const sample of samples) {
    assert(!text.includes(sample.clean_text), 'Sample transcript must not appear in native mode');
    assert(!text.includes(sample.raw_text), 'Sample raw text must not appear in native mode');
  }
}
async function nativePage(browser, control) {
  const page = await browser.newPage(options);
  await page.exposeFunction('__kaydenceHydrationFixture', async command => {
    if (command === 'app_snapshot') {
      if (control.snapshot === 'pending') return new Promise(() => {});
      if (control.snapshot === 'error') throw new Error('Fixture: desktop snapshot unavailable');
      return snapshot;
    }
    if (command === 'recent_history') {
      if (control.history === 'error') throw new Error('Fixture: history read unavailable');
      return history;
    }
    throw new Error(`Unexpected mocked command: ${command}`);
  });
  await page.addInitScript(() => {
    window.isTauri = true;
    window.__TAURI_INTERNALS__ = { invoke: command => window.__kaydenceHydrationFixture(command) };
  });
  return page;
}
(async () => {
  const browser = await chromium.launch();
  async function check(name, control, run) {
    const page = control ? await nativePage(browser, control) : await browser.newPage(options);
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
      await page.addInitScript(preset => localStorage.setItem('kaydence.cockpit.layoutPreset', preset), process.env.COCKPIT_LAYOUT || 'miccapsule');
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1430');
      assert.equal(await page.title(), 'Kaydence');
      assert.equal(await page.locator('vite-error-overlay').count(), 0);
      await run(page);
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'No document overflow');
      await capture(page, `step-${results.length + 1}`);
      assert.deepEqual(errors, [], 'No uncaught page errors');
      results.push({ name, status: 'PASS' });
    } catch (error) {
      results.push({ name, status: 'FAIL', reason: error.message.split('\n')[0], body: (await page.locator('body').innerText()).slice(0, 280) });
    } finally { await page.close(); }
  }
  try {
    await check('browser visibly identifies its fixtures', null, page => shown(page, /preview fixture/i));
    await check('native pending has no sample or operational cockpit', { snapshot: 'pending', history: 'ok' }, async page => {
      await shown(page, /loading kaydence/i);
      await noSamples(page);
      assert.equal(await page.getByRole('region', { name: 'Capture', exact: true }).count(), 0);
      assert.equal(await page.getByText(/^(operational|recording|ready)$/i).count(), 0);
    });
    const retry = { snapshot: 'error', history: 'ok' };
    await check('native rejection is explicit and retry recovers', retry, async page => {
      await shown(page, /desktop state unavailable/i);
      await noSamples(page);
      await capture(page, 'connection-error');
      retry.snapshot = 'ok';
      await page.getByRole('button', { name: /retry connection/i }).click();
      await shown(page, sentinel);
      assert.equal(await page.getByText(/preview fixture/i).count(), 0);
    });
    const partialRetry = { snapshot: 'error', history: 'ok' };
    await check('retry partial failure marks retained history stale', partialRetry, async page => {
      await shown(page, /desktop state unavailable/i);
      partialRetry.snapshot = 'ok'; partialRetry.history = 'error';
      await page.getByRole('button', { name: /retry connection/i }).click();
      await shown(page, /last-loaded.*stale/i);
      await shown(page, sentinel);
      await noSamples(page);
    });
    await check('history rejection is not sample data or empty success', { snapshot: 'ok', history: 'error' }, async page => {
      await shown(page, historyIssue);
      await noSamples(page);
      assert.equal(await page.getByText(/no local sessions yet/i).count(), 0);
      await page.locator('.nav-list').getByRole('button', { name: 'Privacy', exact: true }).click();
      await shown(page, historyIssue);
      assert.equal(await page.getByText(/no local session metadata yet/i).count(), 0);
    });
    const stale = { snapshot: 'ok', history: 'ok' };
    await check('refresh failure retains last-loaded history and warns', stale, async page => {
      await shown(page, sentinel);
      stale.history = 'error';
      await page.getByRole('region', { name: 'History', exact: true }).getByRole('button', { name: /^refresh$/i }).click();
      await shown(page, /stale|last[ -](loaded|known)|previously loaded|showing.*previous/i);
      await shown(page, sentinel);
      await noSamples(page);
    });
    await check('native capture does not invent live status', { snapshot: 'ok', history: 'ok' }, async page => {
      await shown(page, sentinel);
      const capture = page.getByRole('region', { name: 'Capture', exact: true });
      await shown(capture, /live status unavailable/i);
      assert.equal(await capture.getByText(/^(ready|recording)$/i).count(), 0);
      assert.equal(await capture.getByLabel(/microphone idle/i).count(), 0);
    });
  } finally { await browser.close(); }
  console.log(JSON.stringify({ scope: 'Presentation fixtures only; no native calls', results }, null, 2));
  if (results.some(result => result.status === 'FAIL')) process.exitCode = 1;
})().catch(error => { console.error(error); process.exitCode = 1; });
