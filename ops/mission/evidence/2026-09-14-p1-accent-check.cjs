const assert = require('node:assert/strict'), fs = require('node:fs'), zlib = require('node:zlib'), crypto = require('node:crypto');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const axes = [['mac', '#14a7a1'], ['windows', '#0067c0'], ['linux', '#1f9d63']];
const allText = process.env.CHECK_ALL_TEXT === '1';
// Optional controlled IPC presentation states, never native runtime proof.
function statusFixture() {
  const source = fs.readFileSync('apps/desktop/src/App.tsx', 'utf8'), vm = require('node:vm');
  const literal = pattern => { const match = source.match(pattern); assert(match, 'Review changed fixture declaration'); return vm.runInNewContext(`(${match[1]})`, {}, { timeout: 1000 }); };
  const snapshot = literal(/const previewSnapshot: AppSnapshot = ([\s\S]*?\n});/);
  const history = literal(/const previewHistory: HistorySession\[\] = ([\s\S]*?\n\]);/).slice(0, 1);
  const state = process.env.STATUS_FIXTURE;
  assert(['ready', 'blocked', 'held', 'failed'].includes(state));
  snapshot.settings.first_run.permission_requirements = [{ id: 'contrast-fixture', label: 'Controlled permission', state: state === 'blocked' ? 'blocked' : 'ready', detail: 'Controlled fixture', action: 'None', action_label: 'Review' }];
  if (state === 'blocked') history.length = 0;
  if (state === 'held') history[0].held_reason = 'secure_field';
  if (state === 'failed') history[0].failure = { stage: 'inject', error: 'Controlled injection failure' };
  return { snapshot, history };
}
async function measure(root) {
  return root.evaluate(root => {
    const canvas = document.createElement('canvas'), ctx = canvas.getContext('2d', { willReadFrequently: true }); canvas.width = canvas.height = 1;
    const rgba = c => {
      const srgb = c.match(/^color\(srgb ([\d.\s-]+?)(?: \/ ([\d.]+))?\)$/);
      if (srgb) return srgb[1].trim().split(/\s+/).map(v => +v * 255).concat(+(srgb[2] ?? 1));
      const rgb = c.match(/^rgba?\(([^)]+)\)$/); if (rgb) { const channels = rgb[1].split(',').map(Number); return channels.length === 3 ? channels.concat(1) : channels; }
      ctx.clearRect(0, 0, 1, 1); ctx.fillStyle = c; ctx.fillRect(0, 0, 1, 1); return [...ctx.getImageData(0, 0, 1, 1).data].map((v, i) => i === 3 ? v / 255 : v);
    };
    const over = (f, b) => [0, 1, 2].map(i => f[i] * f[3] + b[i] * (1 - f[3])).concat(1);
    const lum = c => c.slice(0, 3).map(v => v / 255).map(v => v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4).reduce((s, v, i) => s + v * [.2126, .7152, .0722][i], 0);
    const expressions = new Set(['var(--accent)']);
    const rules = rs => { for (const r of rs) { if (r.cssRules) rules(r.cssRules); if (r.style?.color.includes('--accent')) expressions.add(r.style.color); } };
    for (const sheet of document.styleSheets) rules(sheet.cssRules);
    const shell = document.querySelector('.app-shell'), probe = document.createElement('i'); probe.hidden = true; shell.append(probe);
    const resolve = c => { probe.style.color = c; return rgba(getComputedStyle(probe).color).join(); };
    const palette = [...expressions].map(resolve), fills = ['--accent', '--accent-control'].filter(t => getComputedStyle(shell).getPropertyValue(t)).map(t => resolve(`var(${t})`)); probe.remove();
    const rows = [], walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const n = walker.currentNode, el = n.parentElement, text = n.textContent.trim();
      if (!text || !el.getClientRects().length || el.closest(':disabled,[aria-disabled="true"]')) continue;
      const s = getComputedStyle(el), fg = rgba(s.color), chain = []; let opacity = 1;
      for (let p = el; p; p = p.parentElement) { chain.unshift(p); opacity *= +getComputedStyle(p).opacity; }
      if (opacity === 0) continue;
      let bg = rgba(getComputedStyle(document.body).backgroundColor);
      for (const p of chain) bg = over(rgba(getComputedStyle(p).backgroundColor), bg);
      fg[3] *= opacity; const painted = over(fg, bg), ls = [lum(painted), lum(bg)].sort((a, b) => a - b), ratio = (ls[1] + .05) / (ls[0] + .05);
      const accent = palette.includes(rgba(s.color).join()) || chain.some(p => fills.includes(rgba(getComputedStyle(p).backgroundColor).join()));
      const unsupported = chain.some(p => { const c = getComputedStyle(p); return c.backgroundImage !== 'none' || +c.opacity !== 1 || c.filter !== 'none' || c.mixBlendMode !== 'normal'; });
      rows.push({ text, selector: el.className || el.tagName, fg: s.color, bg: bg.slice(0, 3), ratio, accent, unsupported });
    }
    return rows;
  });
}
(async () => {
  const browser = await chromium.launch(), results = [], errors = [];
  const baseline = process.env.BASELINE_METRICS ? JSON.parse(zlib.gunzipSync(fs.readFileSync(process.env.BASELINE_METRICS))) : null;
  try {
    for (const [os, accent] of axes) for (const theme of ['light', 'dark']) {
      const page = await browser.newPage({ viewport: { width: 900, height: 600 }, colorScheme: theme, reducedMotion: 'reduce', userAgent: `Mozilla/5.0 (${os}) AppleWebKit/537.36 Chrome/145.0.0.0 Safari/537.36` });
      page.on('pageerror', e => errors.push(e.message)); page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
      if (process.env.STATUS_FIXTURE) await page.addInitScript(({ snapshot, history }) => {
        window.isTauri = true;
        window.__TAURI_INTERNALS__ = { invoke: async command => {
          if (command === 'app_snapshot') return snapshot;
          if (command === 'recent_history') return history;
          throw new Error(`Unexpected contrast fixture IPC: ${command}`);
        } };
      }, statusFixture());
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1436'); assert.equal(await page.title(), 'Kaydence'); assert.equal(await page.locator('vite-error-overlay').count(), 0);
      const check = async (name, root = page.locator('.app-shell')) => {
        await page.mouse.move(899, 599); await page.evaluate(() => document.activeElement?.blur());
        const rows = (await measure(root)).map(r => ({ ...r, state: 'rest' })), blocked = [];
        for (const control of await root.locator('button:enabled, summary, select:enabled').all()) {
          if (!await control.isVisible()) continue;
          const settle = () => control.evaluate(async el => { await Promise.all(el.getAnimations({ subtree: true }).filter(a => a.effect.getTiming().iterations !== Infinity).map(a => a.finished.catch(() => {}))); });
          await control.scrollIntoViewIfNeeded();
          const hit = await control.evaluate(el => { const r = el.getBoundingClientRect(); return el.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)); });
          if (hit) { await control.hover(); await settle(); rows.push(...(await measure(control)).map(r => ({ ...r, state: 'hover' }))); }
          else blocked.push(await control.innerText());
          await page.mouse.move(899, 599); await page.keyboard.press('Tab'); await control.focus(); await settle(); rows.push(...(await measure(control)).map(r => ({ ...r, state: 'focus' })));
        }
        assert.equal(await page.locator('.app-shell').evaluate(el => getComputedStyle(el).getPropertyValue('--accent').trim()), accent, 'Raw OS accent unchanged');
        assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--on-accent').trim()), '#ffffff');
        const geometry = await root.evaluate(el => [el, ...el.querySelectorAll('*')].filter(n => n.getClientRects().length).map(n => { const r = n.getBoundingClientRect(); return [n.tagName, n.className, r.width, r.height, getComputedStyle(n).font]; }));
        const hash = crypto.createHash('sha256').update(JSON.stringify(geometry)).digest('hex'), key = `${os}/${theme}/${name}`;
        if (baseline) assert.equal(hash, baseline.find(r => r.key === key)?.geometry, `Element size/type parity: ${key}`);
        const checked = allText ? rows : rows.filter(r => r.accent), failed = checked.filter(r => r.ratio < 4.5 || r.unsupported), other = rows.filter(r => !r.accent && (r.ratio < 4.5 || r.unsupported));
        results.push({ key, geometry: hash, lowest: Math.min(...checked.map(r => r.ratio)), checked: checked.length, samples: checked, failed, other, blocked });
        console.log(`${failed.length ? 'FAIL' : blocked.length ? 'PARTIAL' : 'PASS'} ${key}: ${checked.length} ${allText ? 'sampled' : 'accent'} text checks; minimum ${Math.min(...checked.map(r => r.ratio)).toFixed(4)}; nonaccent findings ${other.length}; pointer-blocked ${blocked.length}`);
        if (process.env.CAPTURE_DIR && ['Dictate', 'Cleanup', 'Privacy', 'Setup'].includes(name)) {
          await page.mouse.move(899, 599); await page.evaluate(() => { document.activeElement?.blur(); for (const el of document.querySelectorAll('*')) el.scrollTop = 0; scrollTo(0, 0); });
          await page.screenshot({ path: `${process.env.CAPTURE_DIR}/${os === 'mac' ? '' : `${os}-`}${name.toLowerCase()}-${theme}.png`, animations: 'disabled' });
        }
      };
      for (const view of ['Dictate', 'Cleanup', 'Privacy', 'Setup']) {
        await page.locator('.nav-list').getByRole('button', { name: view, exact: true }).click(); await check(view);
        if (view === 'Dictate') for (const [index, pill] of (await page.locator('.cockpit-sb-pill').all()).entries()) {
          await pill.locator('summary').click(); await check(`Dictate-status-${index}`, pill); await pill.locator('summary').click();
        }
        if (view === 'Privacy' || view === 'Setup') {
          await page.getByRole('button', { name: view === 'Privacy' ? 'Privacy Constitution' : 'Evidence', exact: true }).click();
          if (view === 'Setup') for (const tab of ['models', 'permissions', 'proof']) { await page.getByRole('dialog').getByRole('button', { name: tab, exact: true }).click(); await check(`Setup-${tab}`, page.getByRole('dialog')); }
          else await check('Privacy-dialog', page.getByRole('dialog'));
          await page.getByRole('dialog').getByRole('button', { name: /Close/ }).click();
        }
      }
      await page.getByRole('button', { name: 'Cancel', exact: true }).click();
      for (const preset of ['pillbar', 'stackedpanel']) { await page.evaluate(p => localStorage.setItem('kaydence.cockpit.layoutPreset', p), preset); await page.reload(); await check(`Dictate-${preset}`); }
      await page.close();
    }
    if (process.env.RESULT_PATH) fs.writeFileSync(process.env.RESULT_PATH, zlib.gzipSync(JSON.stringify(results)));
    assert.deepEqual(errors, [], 'No frontend errors'); assert(results.every(r => r.checked > 0 && r.failed.length === 0), 'All sampled enabled text must reach 4.5:1; inspect compressed measured results');
    if (results.some(r => r.blocked.length)) { console.error('PARTIAL: pointer-obstructed enabled controls have unmeasured hover states; integrate Dictate fix and rerun.'); process.exitCode = 2; }
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
