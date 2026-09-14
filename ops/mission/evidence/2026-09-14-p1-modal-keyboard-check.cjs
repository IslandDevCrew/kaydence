// Actual pointer/keyboard input. WebKit uses explicit Option+Tab; no settings change.
const assert = require('node:assert/strict'), fs = require('node:fs'), vm = require('node:vm');
const engines = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const out = process.env.CAPTURE_DIR; assert(out, 'Set a fresh output directory'); fs.mkdirSync(out, { recursive: true });
const marker = '[data-modal-keyboard-focus]';
function inspect(el) {
  const s = getComputedStyle(el), r = el.getBoundingClientRect(), extra = s.outlineStyle === 'none' ? 0 : Math.max(0, parseFloat(s.outlineWidth) + parseFloat(s.outlineOffset));
  const b = { left: 0, top: 0, right: innerWidth, bottom: innerHeight };
  for (let p = el.parentElement; p; p = p.parentElement) {
    const c = getComputedStyle(p), a = p.getBoundingClientRect();
    if (/(auto|scroll|hidden|clip)/.test(c.overflowY)) { b.top = Math.max(b.top, a.top); b.bottom = Math.min(b.bottom, a.bottom); }
    if (/(auto|scroll|hidden|clip)/.test(c.overflowX)) { b.left = Math.max(b.left, a.left); b.right = Math.min(b.right, a.right); }
    if (p.tagName === 'DIALOG' && p.open) break;
  }
  const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
  return { text: el.tagName === 'BODY' ? 'BODY' : el.textContent.trim(), tag: el.tagName, focused: document.activeElement === el,
    inside: !!el.closest('dialog[open]'), marked: el.hasAttribute('data-modal-keyboard-focus'), focusVisible: el.matches(':focus-visible'),
    ring: { style: s.outlineStyle, width: s.outlineWidth, offset: s.outlineOffset, color: s.outlineColor, radius: s.borderRadius },
    indicator: s.outlineStyle !== 'none' && parseFloat(s.outlineWidth) > 0, rect: r.toJSON(), bounds: b,
    visible: r.width > 0 && r.height > 0 && r.left - extra >= b.left - .1 && r.right + extra <= b.right + .1 && r.top - extra >= b.top - .1 && r.bottom + extra <= b.bottom + .1 && !!hit && (hit === el || el.contains(hit)) };
}
async function active(page, inside = true) {
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const data = await page.evaluate(inspect => {
    const fn = new Function(`return (${inspect})`)(); return fn(document.activeElement);
  }, inspect.toString());
  assert.equal(data.inside, inside, JSON.stringify(data)); assert(data.indicator && data.visible, JSON.stringify(data)); return data;
}
const source = fs.readFileSync('apps/desktop/src/App.tsx', 'utf8');
const match = source.match(/const previewSnapshot: AppSnapshot = ([\s\S]*?\n});/); assert(match);
const snapshot = JSON.parse(JSON.stringify(vm.runInNewContext(`(${match[1]})`, {}, { timeout: 1000 })));
(async () => {
  const results = [], errors = [], versions = {};
  for (const engine of ['chromium', 'webkit']) {
    const browser = await engines[engine].launch(); versions[engine] = browser.version();
    const tab = engine === 'webkit' ? 'Alt+Tab' : 'Tab', reverse = engine === 'webkit' ? 'Alt+Shift+Tab' : 'Shift+Tab';
    try { for (const theme of ['light', 'dark']) for (const width of [900, 450, 501]) for (const section of ['privacy', 'models', 'permissions', 'proof']) {
      const tag = `${engine}-${theme}-${width}-${section}`, page = await browser.newPage({ viewport: { width, height: width === 900 ? 600 : 300 }, colorScheme: theme, reducedMotion: 'reduce' });
      page.setDefaultTimeout(5000); page.on('pageerror', e => errors.push({ tag, message: e.message }));
      page.on('console', m => { if (m.type() === 'error') errors.push({ tag, message: m.text() }); });
      const row = { tag, steps: [] }; let stage = 'open';
      try {
        await page.goto(process.env.APP_URL || 'http://127.0.0.1:1444');
        await page.locator('.nav-list').getByRole('button', { name: section === 'privacy' ? 'Privacy' : 'Setup', exact: true }).click();
        const opener = page.getByRole('button', { name: section === 'privacy' ? 'Privacy Constitution' : 'Evidence', exact: true });
        const actualOpener = await opener.elementHandle(); await opener.click(); const dialog = page.getByRole('dialog');
        if (section !== 'privacy') await dialog.getByRole('button', { name: section, exact: true }).click();
        const text = page.locator(section === 'privacy' ? '.privacy-dialog dt' : section === 'permissions' ? '.first-run-permission-row strong' : '.first-run-evidence-summary strong').first();
        for (const key of [reverse, tab]) {
          stage = `text pointer then ${key}`; await text.click();
          assert.equal(await page.locator(marker).count(), 0, 'Pointer clears explicit keyboard intent');
          await page.keyboard.press(key); row.steps.push(await active(page));
          const count = await dialog.locator('button:enabled').count();
          for (let n = 0; n < count + 2; n++) { await page.keyboard.press(key); row.steps.push(await active(page)); }
        }
        stage = 'Escape return'; await page.keyboard.press('Escape'); await dialog.waitFor({ state: 'detached' });
        const returned = await actualOpener.evaluate(inspect); assert(returned.focused && returned.indicator && returned.visible, JSON.stringify(returned)); row.returned = returned;
        stage = 'outside pointer clears'; await page.locator(section === 'privacy' ? '.privacy-heading h1' : '.first-run-heading h1').click();
        assert.equal(await page.locator(marker).count(), 0, 'Outside pointer clears returned-opener marker');
        stage = 'keyboard Close return'; await opener.click(); await dialog.getByRole('button', { name: /Close/ }).click();
        assert.equal(await page.locator(marker).count(), 0, 'Pointer Close does not retain a keyboard marker');
        assert(await actualOpener.evaluate(el => document.activeElement === el), 'Pointer Close returns actual opener');
        await opener.click();
        for (let n = 0; n < 40 && !await page.evaluate(() => document.activeElement.textContent.trim() === 'Close'); n++) await page.keyboard.press(tab);
        assert.equal(await page.evaluate(() => document.activeElement.textContent.trim()), 'Close');
        await page.keyboard.press('Enter'); await dialog.waitFor({ state: 'detached' });
        const closeReturn = await actualOpener.evaluate(inspect); assert(closeReturn.focused && closeReturn.indicator && closeReturn.visible, JSON.stringify(closeReturn));
        await page.keyboard.press(tab); assert.equal(await page.locator(marker).count(), 0, 'Blur releases returned marker');
        row.pass = true;
      } catch (e) { row.pass = false; row.stage = stage; row.error = e.message; }
      if (width === 900 && theme === 'light') await page.screenshot({ path: `${out}/${tag}.png` });
      results.push(row); console.log(`${row.pass ? 'PASS' : 'FAIL'} ${tag} ${row.error || ''}`); await page.close();
    }
    // Hold the existing permission IPC; keyboard dismissal uses existing Evidence fallback.
    for (const width of [900, 450]) for (const key of ['Escape', 'Enter']) {
      const tag = `${engine}-pending-${width}-${key}`, page = await browser.newPage({ viewport: { width, height: width === 900 ? 600 : 300 } }), calls = [];
      const fixture = structuredClone(snapshot); fixture.settings.first_run.next_step = { ...fixture.settings.first_run.next_step, kind: 'permission', target_id: 'microphone' };
      let release; const pending = new Promise(resolve => { release = resolve; }); const row = { tag }; let stage = 'pending';
      await page.exposeFunction('__keyboardFixture', async (command, args) => {
        if (command === 'app_snapshot') return fixture; if (command === 'recent_history') return [];
        calls.push({ command, args }); assert.equal(command, 'first_run_permission_action'); await pending;
        return { requirement_id: 'microphone', label: 'Microphone', state: 'needs_review', settings_target: null };
      });
      await page.addInitScript(() => { window.isTauri = true; window.__TAURI_INTERNALS__ = { invoke: (command, args) => window.__keyboardFixture(command, args) }; });
      try {
        await page.goto(process.env.APP_URL || 'http://127.0.0.1:1444'); await page.locator('.nav-list').getByRole('button', { name: 'Setup', exact: true }).click();
        await page.locator('.first-run-footer > button.primary').click(); const dialog = page.getByRole('dialog'); await dialog.waitFor();
        assert(await page.locator('.first-run-footer > button.primary').isDisabled());
        for (let n = 0; key === 'Enter' && n < 30 && !await page.evaluate(() => document.activeElement.textContent.trim() === 'Close'); n++) await page.keyboard.press(tab);
        await page.keyboard.press(key); await dialog.waitFor({ state: 'detached' });
        const fallback = page.locator('.first-run-next > button'), returned = await fallback.evaluate(inspect);
        assert(returned.focused && returned.indicator && returned.visible, JSON.stringify(returned)); row.returned = returned;
        stage = 'late completion after user focus moves'; const select = page.getByRole('combobox', { name: 'Local ASR engine' }); await select.click();
        assert.equal(await page.locator(marker).count(), 0); release(); await page.waitForFunction(() => !document.querySelector('.first-run-footer > button.primary').disabled);
        assert(await select.evaluate(el => document.activeElement === el), 'Late response must not steal user-selected focus');
        assert.equal(await page.locator(marker).count(), 0, 'Late response cannot recreate marker');
        assert.deepEqual(calls, [{ command: 'first_run_permission_action', args: { requirementId: 'microphone' } }]); row.pass = true;
      } catch (e) { row.pass = false; row.stage = stage; row.error = e.message; release(); }
      results.push(row); console.log(`${row.pass ? 'PASS' : 'FAIL'} ${tag} ${row.error || ''}`); await page.close();
    }
    if (engine === 'webkit') for (const fault of ['marker', 'recipe', 'margin']) {
      const row = { tag: `negative-${fault}` }, width = fault === 'margin' ? 450 : 900;
      const page = await browser.newPage({ viewport: { width, height: width === 900 ? 600 : 300 } });
      try {
        await page.goto(process.env.APP_URL || 'http://127.0.0.1:1444');
        await page.locator('.nav-list').getByRole('button', { name: 'Setup', exact: true }).click();
        await page.getByRole('button', { name: 'Evidence', exact: true }).click();
        await page.getByRole('dialog').getByRole('button', { name: fault === 'margin' ? 'proof' : 'models', exact: true }).click();
        const text = page.locator('.first-run-evidence-summary strong').first(); await text.click(); await page.keyboard.press(reverse);
        row.before = await active(page);
        if (fault === 'marker') await page.evaluate(() => document.activeElement.removeAttribute('data-modal-keyboard-focus'));
        else if (fault === 'recipe') assert(await page.evaluate(() => {
          let changed = 0;
          for (const sheet of document.styleSheets) for (const rule of sheet.cssRules) if (rule.selectorText?.includes('[data-modal-keyboard-focus]')) {
            rule.selectorText = rule.selectorText.replaceAll(':is(:focus-visible, :focus:where([data-modal-keyboard-focus]))', ':focus-visible'); changed++;
          }
          return changed;
        }), 'Negative must remove an actual authored selector');
        else {
          await page.addStyleTag({ content: 'dialog.modal-backdrop :is(button,[href],input,select,textarea,[tabindex]) { scroll-margin: 0 !important; }' });
          await page.getByRole('dialog').getByRole('button', { name: /Close/ }).click();
          await page.getByRole('button', { name: 'Evidence', exact: true }).click();
          await page.getByRole('dialog').getByRole('button', { name: 'proof', exact: true }).click();
          await text.click(); await page.keyboard.press(reverse);
        }
        row.after = await page.evaluate(inspect => new Function(`return (${inspect})`)()(document.activeElement), inspect.toString());
        assert(fault === 'margin' ? !row.after.visible : !row.after.indicator, 'Negative must fail the original visibility oracle'); row.pass = true;
      } catch (e) { row.pass = false; row.error = e.message; }
      results.push(row); console.log(`${row.pass ? 'PASS' : 'FAIL'} ${row.tag} ${row.error || ''}`); await page.close();
    } } finally { await browser.close(); }
  }
  fs.writeFileSync(`${out}/results.json`, JSON.stringify({ versions, results, errors }, null, 2));
  console.log(JSON.stringify({ cases: results.length, passed: results.filter(r => r.pass).length, errors }));
  if (errors.length || results.some(r => !r.pass)) process.exitCode = 1;
})().catch(e => { console.error(e); process.exitCode = 1; });
