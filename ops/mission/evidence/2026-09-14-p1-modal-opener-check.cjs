// Real pointer activation; source-derived IPC fixtures never call a native backend.
const assert = require('node:assert/strict'), fs = require('node:fs'), vm = require('node:vm');
const { chromium, webkit } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const source = fs.readFileSync('apps/desktop/src/App.tsx', 'utf8');
const match = source.match(/const previewSnapshot: AppSnapshot = ([\s\S]*?\n});/);
assert(match, 'Review changed fixture declaration');
const base = JSON.parse(JSON.stringify(vm.runInNewContext(`(${match[1]})`, {}, { timeout: 1000 })));
const out = process.env.CAPTURE_DIR; assert(out, 'Set a fresh CAPTURE_DIR'); fs.mkdirSync(out, { recursive: true });
const core = ['privacy', 'evidence', 'models', 'permissions', 'hotkey'];
const kinds = ['model_metadata', 'model_install', 'asr_runtime', 'permission'];
const routes = ['setup', 'dictation', 'complete'];
function intent(kind) {
  const review = { command: 'first_run_model_download_preflight', args: { modelId: 'fixture-model' } };
  const refresh = { command: 'refresh_model_readiness', args: {} };
  if (kind === 'model_metadata') return [review, refresh];
  if (kind === 'model_install') return [review];
  if (kind === 'asr_runtime') return [refresh];
  if (kind === 'permission') return [{ command: 'first_run_permission_action', args: { requirementId: 'microphone' } }];
  return [];
}
function shape(el) {
  const r = el.getBoundingClientRect(), style = getComputedStyle(el);
  const ring = style.outlineStyle === 'none' ? 0 : Math.max(0, parseFloat(style.outlineWidth) + parseFloat(style.outlineOffset));
  const bounds = { left: 0, top: 0, right: innerWidth, bottom: innerHeight };
  for (let p = el.parentElement; p; p = p.parentElement) {
    const c = getComputedStyle(p), box = p.getBoundingClientRect();
    if (/(auto|scroll|hidden|clip)/.test(c.overflowX)) { bounds.left = Math.max(bounds.left, box.left); bounds.right = Math.min(bounds.right, box.right); }
    if (/(auto|scroll|hidden|clip)/.test(c.overflowY)) { bounds.top = Math.max(bounds.top, box.top); bounds.bottom = Math.min(bounds.bottom, box.bottom); }
  }
  return { focused: document.activeElement === el, active: document.activeElement.tagName,
    rect: r.toJSON(), ring, bounds, visible: r.width > 0 && r.height > 0 && r.left - ring >= bounds.left - .1 && r.right + ring <= bounds.right + .1 && r.top - ring >= bounds.top - .1 && r.bottom + ring <= bounds.bottom + .1 };
}
(async () => {
  const results = [], versions = {}, errors = [];
  for (const [engine, launcher] of Object.entries({ chromium, webkit })) {
    if (process.env.ENGINE && process.env.ENGINE !== engine) continue;
    const browser = await launcher.launch(); versions[engine] = browser.version();
    try {
      for (const theme of ['light', 'dark']) for (const width of [900, 450, 501]) {
        const cases = [...core, ...(width === 900 ? [...kinds, ...routes] : [])];
        for (const entry of cases) for (const dismissal of ['Escape', 'Close']) {
          if (routes.includes(entry) && dismissal === 'Close') continue;
          const tag = `${engine}-${theme}-${width}-${entry}-${dismissal}`, row = { tag };
          const page = await browser.newPage({ viewport: { width, height: width === 900 ? 600 : 300 }, colorScheme: theme, reducedMotion: 'reduce' });
          page.setDefaultTimeout(5000); page.on('pageerror', error => errors.push({ tag, message: error.message }));
          page.on('console', message => { if (message.type() === 'error') errors.push({ tag, message: message.text() }); });
          const snapshot = structuredClone(base), calls = [], kind = core.includes(entry) ? 'hotkey' : entry;
          snapshot.settings.first_run.next_step = { ...snapshot.settings.first_run.next_step, kind,
            target_id: kind === 'permission' ? 'microphone' : 'fixture-model', action_label: 'Review next step' };
          await page.exposeFunction('__openerFixture', async (command, args) => {
            if (command === 'app_snapshot') return snapshot;
            if (command === 'recent_history') return [];
            calls.push({ command, ...(args === undefined ? {} : { args }) });
            if (command === 'refresh_model_readiness') return snapshot;
            if (command === 'first_run_model_download_preflight') return { model_id: 'fixture-model', task: 'asr', runtime: 'fixture', file: 'fixture.bin', state: 'missing', available: false, sources: [], source_count: 0, license: 'Fixture', license_review_required: true, detail: 'Controlled fixture', operator_action: 'No download', proof_requirement: 'Native proof remains separate' };
            if (command === 'first_run_permission_action') return { requirement_id: 'microphone', label: 'Microphone', state: 'needs_review', action_label: 'Review', settings_target: null, settings_open_label: null, manual_step: 'Controlled fixture', proof_requirement: 'Native proof remains separate', proof_command: null, expected_evidence: 'None', ready_boundary: 'Not OS proof' };
            throw new Error(`Unexpected fixture IPC: ${command}`);
          });
          await page.addInitScript(() => { window.isTauri = true; window.__TAURI_INTERNALS__ = { invoke: (command, args) => window.__openerFixture(command, args) }; });
          try {
            await page.goto(process.env.APP_URL || 'http://127.0.0.1:1440');
            await page.locator('.nav-list').getByRole('button', { name: entry === 'privacy' ? 'Privacy' : 'Setup', exact: true }).click();
            const selector = entry === 'privacy' ? '.privacy-heading > button' : entry === 'evidence' ? '.first-run-next > button' : entry === 'models' ? '.first-run-status-button' : entry === 'permissions' ? '.first-run-inline-action' : '.first-run-footer > button.primary';
            const opener = page.locator(selector), actual = await opener.elementHandle();
            row.before = await opener.evaluate(shape);
            if (entry === 'setup') assert(await opener.isDisabled(), 'Setup gate must stay disabled');
            else await opener.click();
            if (routes.includes(entry)) {
              if (entry !== 'setup') await page.locator('.dictate-shell').waitFor();
              assert.equal(await page.getByRole('dialog').count(), 0, 'Non-modal action must not open evidence');
              assert.deepEqual(calls, []); row.pass = true;
            } else {
            await page.getByRole('dialog').waitFor(); row.initial = await page.evaluate(() => document.activeElement.tagName);
            if (entry !== 'privacy') {
              const section = ['models', ...kinds.slice(0, 3)].includes(entry) ? 'models' : ['permissions', 'permission'].includes(entry) ? 'permissions' : 'proof';
              assert.equal(await page.getByRole('dialog').locator('nav button[aria-pressed="true"]').textContent(), section);
            }
            // Settle the existing action, without altering focus, scrolling or native commands.
            await page.waitForFunction(el => !el.disabled, actual);
            assert.deepEqual(calls, core.includes(entry) ? [] : intent(kind)); row.calls = calls;
            if (dismissal === 'Escape') await page.keyboard.press('Escape');
            else await page.getByRole('dialog').getByRole('button', { name: /Close/ }).click();
            await page.getByRole('dialog').waitFor({ state: 'detached' });
            await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
            row.returned = await actual.evaluate(shape);
            assert(row.returned.focused, `Actual clicked opener lost: ${JSON.stringify(row.returned)}`);
            assert(row.returned.visible, `Returned opener is clipped: ${JSON.stringify(row.returned)}`);
            row.pass = true;
            }
          } catch (error) { row.pass = false; row.failure = String(error); }
          if (theme === 'light' && width === 450 && dismissal === 'Escape') await page.screenshot({ path: `${out}/${tag}.png` });
          results.push(row); console.log(`${row.pass ? 'PASS' : 'FAIL'} ${tag}${row.failure ? ` ${row.failure}` : ''}`); await page.close();
        }
      }
    } finally { await browser.close(); }
  }
  fs.writeFileSync(`${out}/results.json`, JSON.stringify({ versions, results, errors }, null, 2));
  console.log(JSON.stringify({ versions, cases: results.length, passed: results.filter(row => row.pass).length, errors }));
  if (errors.length || results.some(row => !row.pass)) process.exitCode = 1;
})().catch(error => { console.error(error); process.exitCode = 1; });
