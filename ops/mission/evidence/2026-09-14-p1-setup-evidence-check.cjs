// Browser-only fixtures; real wheel/Tab (WebKit Option+Tab), no positive style/focus/scroll overrides.
const assert = require('node:assert/strict'), fs = require('node:fs'), vm = require('node:vm');
const engines = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const out = process.env.CAPTURE_DIR; assert(out); fs.mkdirSync(out, { recursive: true });
async function settle(page) {
  await page.waitForTimeout(80); // Let the native input task start before measuring quiescence.
  await page.evaluate(async () => {
    let last = '', since = performance.now();
    for (let n = 0; n < 180; n++) {
      await new Promise(requestAnimationFrame);
      const now = [...document.querySelectorAll('*')].filter(e => e.scrollHeight > e.clientHeight || e.scrollWidth > e.clientWidth).map(e => `${e.scrollTop},${e.scrollLeft}`).join(';');
      if (now !== last) since = performance.now(); last = now;
      if (performance.now() - since >= 300) return;
    }
    throw Error('Scrolling did not settle');
  });
}
async function wheel(page, root, x, y) {
  const box = await root.boundingBox(); assert(box && box.height > 0);
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  const handle = await root.elementHandle();
  await handle.evaluate(el => { el.__receipt = false; el.__listener = () => { el.__receipt = true; }; el.addEventListener('wheel', el.__listener, { once: true }); });
  try { await page.mouse.wheel(x, y); await page.waitForFunction(el => el.__receipt, handle, { timeout: 2000 }); }
  finally { await handle.evaluate(el => { el.removeEventListener('wheel', el.__listener); delete el.__listener; delete el.__receipt; }); await handle.dispose(); }
  await settle(page);
}
function readText(root) {
  const rows = [], walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const node = walker.currentNode, el = node.parentElement;
    if (!node.textContent.trim()) continue;
    const style = getComputedStyle(el), visible = [], required = [];
    for (let i = 0; i < node.length; i++) {
      if (!node.textContent[i].trim()) continue; required.push(i);
      const range = document.createRange(); range.setStart(node, i); range.setEnd(node, i + 1);
      let shown = false;
      for (const r of range.getClientRects()) {
        let good = r.width > 0 && r.height > 0 && r.left >= -.2 && r.top >= -.2 && r.right <= innerWidth + .2 && r.bottom <= innerHeight + .2;
        for (let p = el; good && p; p = p.parentElement) {
          const s = getComputedStyle(p), b = p.getBoundingClientRect();
          if (s.visibility !== 'visible' || s.display === 'none') good = false;
          if (/(hidden|clip|auto|scroll)/.test(s.overflowX) && (r.left < b.left - .2 || r.right > b.right + .2)) good = false;
          if (/(hidden|clip|auto|scroll)/.test(s.overflowY) && (r.top < b.top - .2 || r.bottom > b.bottom + .2)) good = false;
          if (p.matches('.first-run-evidence-body > *, .first-run-proof-grid > *, .first-run-permission-row, .first-run-model-row') && (r.left < b.left - .2 || r.right > b.right + .2 || r.top < b.top - .2 || r.bottom > b.bottom + .2)) good = false;
          if (p === root) break;
        }
        const hit = good && document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
        if (good && hit && (hit === el || el.contains(hit))) shown = true;
      }
      if (shown) visible.push(i);
    }
    rows.push({ text: node.textContent, size: parseFloat(style.fontSize), required, visible });
  }
  return rows;
}
async function fullRead(page) {
  const root = page.locator('.first-run-dialog'), scroller = page.locator('.first-run-evidence-body');
  await wheel(page, scroller, 0, -10000);
  const initial = await root.evaluate(readText), seen = initial.map(() => new Set()), positions = [];
  const sample = async () => {
    const rows = await root.evaluate(readText); assert.deepEqual(rows.map(r => r.text), initial.map(r => r.text), 'Stable actual text inventory');
    rows.forEach((r, i) => r.visible.forEach(n => seen[i].add(n)));
  };
  for (let step = 0; step < 70; step++) {
    await sample();
    for (const code of await scroller.locator('code').all()) {
      const visible = await code.evaluate(el => { const r = el.getBoundingClientRect(), p = el.closest('.first-run-evidence-body').getBoundingClientRect(); return r.top >= p.top && r.bottom <= p.bottom && r.top >= 0 && r.bottom <= innerHeight && el.scrollWidth > el.clientWidth; });
      if (!visible) continue;
      await wheel(page, code, -10000, 0);
      for (let n = 0; n < 20; n++) {
        await sample(); const x = await code.evaluate(el => [el.scrollLeft, el.scrollWidth - el.clientWidth, el.clientWidth]);
        if (x[0] >= x[1] - 1) break;
        await wheel(page, code, x[2] * .6, 0); assert((await code.evaluate(el => el.scrollLeft)) > x[0], 'Horizontal wheel advances code');
      }
    }
    const pos = await scroller.evaluate(el => ({ top: el.scrollTop, max: el.scrollHeight - el.clientHeight, height: el.clientHeight, overflow: getComputedStyle(el).overflowY }));
    positions.push(pos); if (pos.max <= 0 || pos.top >= pos.max - .5) break;
    assert(/auto|scroll/.test(pos.overflow), 'Overflowing text needs a real user-scroll owner');
    await wheel(page, scroller, 0, Math.max(30, Math.floor(pos.height * .6)));
    const after = await scroller.evaluate(el => ({ top: el.scrollTop, max: el.scrollHeight - el.clientHeight }));
    assert(after.top > pos.top, `Real wheel must advance: ${JSON.stringify({ pos, after })}`);
  }
  const issues = initial.flatMap((r, i) => [r.size < 11 ? `sub11 ${r.size}: ${r.text}` : null, r.required.some(n => !seen[i].has(n)) ? `unread text: ${r.text}` : null].filter(Boolean));
  return { issues, nodes: initial.length, characters: initial.reduce((n, r) => n + r.required.length, 0), minimum: Math.min(...initial.map(r => r.size)), positions };
}
function focused(el) {
  const r = el.getBoundingClientRect(), s = getComputedStyle(el), edge = Math.max(0, parseFloat(s.outlineWidth) + parseFloat(s.outlineOffset));
  let l = 0, t = 0, right = innerWidth, bottom = innerHeight;
  for (let p = el.parentElement; p; p = p.parentElement) {
    const a = p.getBoundingClientRect(), c = getComputedStyle(p);
    if (/auto|scroll|hidden|clip/.test(c.overflowX)) { l = Math.max(l, a.left); right = Math.min(right, a.right); }
    if (/auto|scroll|hidden|clip/.test(c.overflowY)) { t = Math.max(t, a.top); bottom = Math.min(bottom, a.bottom); }
    if (p.matches('.first-run-permission-row')) { l = Math.max(l, a.left); right = Math.min(right, a.right); t = Math.max(t, a.top); bottom = Math.min(bottom, a.bottom); }
    if (p.matches('dialog[open]')) break;
  }
  const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
  return { text: el.textContent.trim(), inside: !!el.closest('dialog[open]'), active: document.activeElement === el,
    permission: !!el.closest('.first-run-permission-row'), native: el.matches(':focus-visible'), marked: el.hasAttribute('data-modal-keyboard-focus'),
    visible: r.width > 0 && r.height > 0 && r.left - edge >= l - .2 && r.top - edge >= t - .2 && r.right + edge <= right + .2 && r.bottom + edge <= bottom + .2 && !!hit && (el === hit || el.contains(hit)),
    indicator: s.outlineStyle !== 'none' && parseFloat(s.outlineWidth) >= 1, outline: [s.outlineColor, s.outlineWidth, s.outlineOffset], margin: s.scrollMarginTop, rect: r.toJSON() };
}
async function cue(page) {
  await settle(page); const row = await page.locator(':focus').evaluate(focused);
  assert(row.active && row.visible && row.indicator, JSON.stringify(row));
  if (row.permission) { assert.equal(row.outline[2], '-2px', 'Nested permission inset applies to actual keyboard focus'); assert.equal(row.margin, '8px', 'Keep inherited Modal scroll margin'); }
  return row;
}
module.exports = { fullRead, focused, cue, wheel, fixture };
async function fixture(page, force = false) {
  if (!process.env.DETAILS && !force) return;
  const source = fs.readFileSync('apps/desktop/src/App.tsx', 'utf8').match(/const previewSnapshot: AppSnapshot = ([\s\S]*?\n});/); assert(source);
  const snapshot = JSON.parse(JSON.stringify(vm.runInNewContext(`(${source[1]})`, {}, { timeout: 1000 })));
  const model = { id: 'whisper-base-en-q5_1', task: 'asr', lane: 'gpu', runtime: 'whisper_cpp', file: 'ggml-base.en-q5_1.bin', state: 'missing', detail: 'Fixture only: local model is not installed.', download_available: false, license: 'MIT', selected: true, min_hw: 'Fixture reference hardware', size_mb: 60 };
  Object.assign(snapshot.settings.first_run, { required_models: [model], asr_candidates: [model], model_ready: false });
  await page.exposeFunction('__evidenceFixture', (command) => {
    if (command === 'app_snapshot') return snapshot; if (command === 'recent_history') return [];
    if (command === 'first_run_model_download_preflight') return { ...model, model_id: model.id, available: false, expected_sha256: '0123456789abcdef'.repeat(4), blocked_reason: 'Fixture only: downloads remain gated.' };
    if (command === 'first_run_permission_action') return { requirement_id: 'microphone', label: 'Fixture microphone guidance', manual_step: 'Read this fixture only; no operating-system permission has changed.', proof_requirement: 'Native capture remains unverified.', expected_evidence: 'A separately approved physical-device record.', ready_boundary: 'No fixture grants readiness.', proof_command: 'fixture-only-do-not-execute '.repeat(4), settings_target: null };
    throw Error(`Unexpected fixture command: ${command}`);
  });
  await page.addInitScript(() => { window.isTauri = true; window.__TAURI_INTERNALS__ = { invoke: command => window.__evidenceFixture(command) }; });
}
if (require.main === module) (async () => {
  const results = [], errors = [], versions = {};
  for (const engine of process.env.ENGINE ? [process.env.ENGINE] : ['chromium', 'webkit']) {
    const browser = await engines[engine].launch(); versions[engine] = browser.version();
    const tab = engine === 'webkit' ? 'Alt+Tab' : 'Tab', back = engine === 'webkit' ? 'Alt+Shift+Tab' : 'Shift+Tab';
    try { for (const theme of process.env.THEME ? [process.env.THEME] : ['light', 'dark']) for (const width of process.env.WIDTH ? [+process.env.WIDTH] : [900, 450, 501]) for (const section of process.env.SECTION ? [process.env.SECTION] : ['models', 'permissions', 'proof']) {
      const tag = `${engine}-${theme}-${width}-${section}`, page = await browser.newPage({ viewport: { width, height: width === 900 ? 600 : 300 }, colorScheme: theme, reducedMotion: 'reduce' });
      page.setDefaultTimeout(5000); page.on('pageerror', e => errors.push({ tag, error: e.message })); page.on('console', m => { if (m.type() === 'error') errors.push({ tag, error: m.text() }); });
      const row = { tag, cues: [] }; let stage = 'entry';
      try {
        await fixture(page);
        await page.goto(process.env.APP_URL || 'http://127.0.0.1:1435'); assert.equal(await page.title(), 'Kaydence'); assert.equal(await page.locator('vite-error-overlay').count(), 0);
        await page.locator('.nav-list').getByRole('button', { name: 'Setup', exact: true }).click();
        const opener = page.getByRole('button', { name: 'Evidence', exact: true }), actual = await opener.elementHandle();
        await opener.click(); const dialog = page.getByRole('dialog'); await dialog.getByRole('button', { name: section, exact: true }).click();
        if (process.env.DETAILS && section !== 'proof') {
          await dialog.getByRole('button', { name: section === 'models' ? 'Review' : 'Show microphone step', exact: true }).click();
          await dialog.locator(section === 'models' ? '.first-run-preflight' : '.first-run-permission-proof').waitFor();
          if (section === 'models') assert(await dialog.getByRole('button', { name: 'Download', exact: true }).isDisabled(), 'Review cannot unlock gated download');
        }
        stage = 'real wheel reading'; row.read = await fullRead(page);
        if (width !== 501) await page.screenshot({ path: `${out}/${tag}-read.png` });
        stage = 'natural keyboard'; const count = await dialog.locator('button:enabled').count();
        for (const key of [tab, back]) for (let n = 0; n < count + 2; n++) {
          await page.keyboard.press(key); const c = await cue(page); assert(c.inside); row.cues.push(c);
          if (c.permission && width !== 501 && !row.cueCapture) {
            row.cueCapture = `${tag}-cue.png`; await page.screenshot({ path: `${out}/${row.cueCapture}` });
          }
        }
        stage = 'Escape actual opener'; await page.keyboard.press('Escape'); await dialog.waitFor({ state: 'detached' });
        row.returned = await cue(page); assert(await actual.evaluate(el => document.activeElement === el));
        stage = 'keyboard Close'; await opener.click(); await page.keyboard.press(tab);
        for (let n = 0; n < 40 && !await page.getByRole('button', { name: 'Close setup evidence' }).evaluate(el => document.activeElement === el); n++) await page.keyboard.press(tab);
        assert(await page.getByRole('button', { name: 'Close setup evidence' }).evaluate(el => document.activeElement === el)); row.close = await cue(page);
        await page.keyboard.press('Enter'); await dialog.waitFor({ state: 'detached' }); row.closeReturn = await cue(page); assert(await actual.evaluate(el => document.activeElement === el));
        assert.deepEqual(row.read.issues, [], 'All dialog text >=11px and fully wheel-readable'); row.pass = true;
      } catch (e) { row.pass = false; row.stage = stage; row.error = e.message; }
      results.push(row); console.log(`${row.pass ? 'PASS' : 'FAIL'} ${tag} ${row.error || ''}`); await page.close();
    } } finally { await browser.close(); }
  }
  fs.writeFileSync(`${out}/results.json`, JSON.stringify({ versions, results, errors }, null, 2));
  console.log(JSON.stringify({ cases: results.length, passed: results.filter(r => r.pass).length, errors }));
  if (errors.length || results.some(r => !r.pass)) process.exitCode = 1;
})().catch(e => { console.error(e); process.exitCode = 1; });
