// Actual browser bundle at rest; negative paint/spacing overrides are explicitly labeled.
const assert = require('node:assert/strict'), fs = require('node:fs'), crypto = require('node:crypto');
const pw = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const out = process.env.CAPTURE_DIR; assert(out); fs.mkdirSync(out, { recursive: true });
const { focused } = require('./2026-09-14-p1-setup-evidence-check.cjs');
const cssPath = 'apps/desktop/src/views/FirstRunView.css';
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
function textAtRest(root) {
  const rows = [], walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const node = walker.currentNode, el = node.parentElement;
    if (!node.textContent.trim() || el.closest('option')) continue;
    const required = [], visible = [], size = parseFloat(getComputedStyle(el).fontSize);
    for (let i = 0; i < node.length; i++) {
      if (!node.textContent[i].trim()) continue; required.push(i);
      const range = document.createRange(); range.setStart(node, i); range.setEnd(node, i + 1);
      for (const r of range.getClientRects()) {
        let good = r.width > 0 && r.height > 0 && r.left >= -.2 && r.top >= -.2 && r.right <= innerWidth + .2 && r.bottom <= innerHeight + .2;
        for (let p = el; good && p; p = p.parentElement) {
          const s = getComputedStyle(p), b = p.getBoundingClientRect();
          if (s.visibility !== 'visible' || s.display === 'none' || +s.opacity === 0) good = false;
          if (/(hidden|clip|auto|scroll)/.test(s.overflowX) && (r.left < b.left - .2 || r.right > b.right + .2)) good = false;
          if (/(hidden|clip|auto|scroll)/.test(s.overflowY) && (r.top < b.top - .2 || r.bottom > b.bottom + .2)) good = false;
          if (p.matches('.first-run-control-row,.first-run-license-card,.first-run-step-rail li,.first-run-next,.first-run-footer') && (r.left < b.left - .2 || r.right > b.right + .2 || r.top < b.top - .2 || r.bottom > b.bottom + .2)) good = false;
        }
        const hit = good && document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
        const owner = hit && hit.contains(el) && /auto|scroll/.test(getComputedStyle(hit).overflowY) && ['::before','::after'].every(p => ['none','normal'].includes(getComputedStyle(hit,p).content));
        if (good && hit && (hit === el || el.contains(hit) || owner)) { visible.push(i); break; }
      }
    }
    rows.push({ text: node.textContent, size, required, visible });
  }
  return rows;
}
function geometry(root) {
  const box = el => el.getBoundingClientRect().toJSON();
  const workspace = root.querySelector('.first-run-workspace'), s = getComputedStyle(workspace);
  return { workspace: { ...box(workspace), scrollTop: workspace.scrollTop, client: workspace.clientHeight, content: workspace.scrollHeight, paddingBottom: s.paddingBottom },
    actions: [...root.querySelectorAll('.first-run-footer > button')].map(el => ({ text: el.textContent, disabled: el.disabled, ...box(el) })),
    controls: [...root.querySelectorAll('button,input,select')].map(el => ({ text: el.textContent, ...box(el) })),
    spacing: ['.first-run-control-row','.first-run-next','.first-run-license-card'].map(selector => { const el=root.querySelector(selector), c=getComputedStyle(el); return { selector, padding: c.padding, height: el.getBoundingClientRect().height, minHeight: c.minHeight }; }),
    document: [document.documentElement.scrollWidth, document.documentElement.scrollHeight], rawAccent: getComputedStyle(root.closest('.app-shell')).getPropertyValue('--accent').trim() };
}
(async () => {
  const results = [], errors = [], versions = {}, source = fs.readFileSync(cssPath, 'utf8');
  for (const engine of process.env.ENGINE ? [process.env.ENGINE] : ['chromium','webkit']) {
    const browser = await pw[engine].launch(); versions[engine] = browser.version();
    try { for (const theme of process.env.THEME ? [process.env.THEME] : ['light','dark']) for (const lane of process.env.LANE ? [process.env.LANE] : ['macOS','Windows','Linux']) {
      const tag = `${engine}-${theme}-${lane}`, row = { tag, negative: process.env.NEGATIVE || null };
      const page = await browser.newPage({ viewport: { width: 900, height: 600 }, colorScheme: theme, reducedMotion: 'reduce' });
      page.setDefaultTimeout(5000); page.on('pageerror', e => errors.push({ tag, error: e.message }));
      page.on('console', m => { if (m.type() === 'error') errors.push({ tag, error: m.text() }); });
      try {
        await page.goto(process.env.APP_URL || 'http://127.0.0.1:1432');
        await page.locator('.nav-list').getByRole('button', { name: 'Setup', exact: true }).click();
        await page.locator('.first-run-lanes').getByRole('button', { name: lane, exact: true }).click();
        await page.evaluate(() => document.fonts.ready);
        const served = await page.locator('style[data-vite-dev-id$="/views/FirstRunView.css"]').textContent();
        assert.equal(served, source, 'Actual served CSS equals authored source'); row.sourceSHA256 = sha(served);
        if (process.env.NEGATIVE) {
          const css = { spacing: '.first-run-control-row{padding-block:3px!important}.first-run-next{padding-block:5px!important}.first-run-license-card{padding-block:6px!important}', hidden: '.first-run-local-note small{visibility:hidden!important}', font: '.first-run-local-note small{font-size:10px!important}', paint: '.first-run-step-rail::after{content:"";position:fixed;inset:0;background:white;z-index:9999}' }[process.env.NEGATIVE];
          assert(css); await page.addStyleTag({ content: css });
        }
        const board = page.locator('.first-run-board'); row.geometry = await board.evaluate(geometry); row.text = await board.evaluate(textAtRest);
        await page.screenshot({ path: `${out}/${tag}-initial.png` });
        assert.equal(row.geometry.workspace.scrollTop, 0, 'Initial fold was not scrolled');
        assert(row.geometry.workspace.content <= row.geometry.workspace.client, 'Default Setup has no initial vertical overflow');
        assert(row.geometry.actions.every(r => r.bottom + 3 <= 600 && r.top - 3 >= row.geometry.workspace.top), 'Both complete action targets and existing 2px+1px cue fit at rest');
        assert.deepEqual(row.geometry.document, [900,600], 'No document overflow');
        assert(row.text.length > 60, 'Meaningful complete board inventory');
        assert.deepEqual(row.text.filter(r => r.size < 11 || r.required.some(n => !r.visible.includes(n))), [], 'Every initial glyph is visible and >=11px');
        // Reference-lane primary remains disabled; do not manufacture its focusability.
        const expected = row.geometry.actions.filter(r => !r.disabled).map(r => r.text);
        assert.equal(expected.length, lane === 'macOS' ? 2 : 1, 'Runtime/reference action boundary unchanged');
        row.cues = []; const reached = new Set(), tab = engine === 'webkit' ? 'Alt+Tab' : 'Tab';
        for (let n = 0; n < 70 && reached.size < expected.length; n++) {
          await page.keyboard.press(tab); await page.waitForTimeout(140);
          if (await page.locator('.first-run-footer > button:focus').count()) {
            const data = await page.locator(':focus').evaluate(focused); assert(data.active && data.visible && data.indicator, 'Whole actual keyboard action cue');
            reached.add(data.text); row.cues.push(data); await page.screenshot({ path: `${out}/${tag}-cue-${reached.size}.png` });
          }
        }
        assert(expected.every(text => reached.has(text)), 'All enabled footer actions reached with native keyboard input'); row.pass = true;
      } catch (e) { row.pass = false; row.error = e.message; }
      results.push(row); console.log(`${row.pass ? 'PASS' : 'FAIL'} ${tag} ${row.error || ''}`); await page.close();
    } } finally { await browser.close(); }
  }
  fs.writeFileSync(`${out}/results.json`, JSON.stringify({ versions, results, errors, sourceSHA256: sha(source) }, null, 2));
  if (errors.length || results.some(r => !r.pass)) process.exitCode = 1;
})().catch(e => { console.error(e); process.exitCode = 1; });
