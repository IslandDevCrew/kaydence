// Presentation fixtures only; the Rust cleanup engine is not executed in the browser.
const assert = require('node:assert/strict'), fs = require('node:fs');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const examples = [
  ['hello comma world', 'Hello, world.'],
  ['um send send the build comma you know then ship it period', 'Send the build, then ship it.'],
  ['schedule it for Tuesday no wait Friday', 'Schedule it for Friday.'],
];
const rust = fs.readFileSync('apps/desktop/src-tauri/src/cleanup/mod.rs', 'utf8');
for (const [before, after] of examples) for (const text of [before, after]) assert(rust.includes(`"${text}"`), 'Example remains bound to an existing Rust fixture');
(async () => {
  const browser = await chromium.launch(), errors = [];
  try {
    for (const theme of ['light', 'dark']) for (const layout of ['miccapsule', 'pillbar', 'stackedpanel']) {
      const page = await browser.newPage({ viewport: { width: 900, height: 600 }, colorScheme: theme });
      page.on('pageerror', e => errors.push(e.message));
      page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
      await page.addInitScript(p => localStorage.setItem('kaydence.cockpit.layoutPreset', p), layout);
      await page.goto(process.env.APP_URL || 'http://127.0.0.1:1437');
      assert.equal(await page.title(), 'Kaydence'); assert.equal(await page.locator('vite-error-overlay').count(), 0);
      for (const dial of ['Raw', 'Light', 'Full']) {
        const cockpit = page.locator('.cockpit-control-panel').filter({ has: page.locator('.cockpit-dial') });
        await cockpit.getByRole('button', { name: dial.toLowerCase(), exact: true }).click();
        assert.equal(await cockpit.getByRole('button', { name: dial.toLowerCase(), exact: true }).getAttribute('aria-pressed'), 'true');
        assert.equal(await cockpit.locator('ul li').count(), dial === 'Raw' ? 0 : 5);
        if (dial === 'Full') assert((await cockpit.innerText()).includes('Full currently uses Light rules; no profile rewrite.'));
        await page.locator('.nav-list').getByRole('button', { name: 'Cleanup', exact: true }).click();
        const rules = page.locator('.cleanup-rule-list');
        assert.equal(await rules.locator('input:checked').count(), dial === 'Raw' ? 0 : 6);
        const dictionary = rules.locator('label').filter({ hasText: 'Apply custom dictionary' });
        assert.equal(await dictionary.locator('input').isChecked(), false);
        assert((await dictionary.innerText()).includes('Not connected'));
        assert(!(await rules.innerText()).includes('Profile rule'));
        const rows = page.locator('.cleanup-examples > div');
        for (let i = 0; i < examples.length; i++) {
          assert.equal(await rows.nth(i).locator('p').nth(0).textContent(), `Before${examples[i][0]}`);
          assert.equal(await rows.nth(i).locator('p').nth(1).textContent(), `After${examples[i][dial === 'Raw' ? 0 : 1]}`);
        }
        assert((await page.locator('.cleanup-example-section h3').innerText()).toLowerCase().includes('illustrative'));
        if (process.env.CAPTURE_DIR && layout === 'miccapsule') await page.screenshot({ path: `${process.env.CAPTURE_DIR}/cleanup-${theme}-${dial.toLowerCase()}.png` });
        await page.locator('.nav-list').getByRole('button', { name: 'Dictate', exact: true }).click();
        assert.equal(await cockpit.getByRole('button', { name: dial.toLowerCase(), exact: true }).getAttribute('aria-pressed'), 'true', 'Navigation preserves selected dial');
        console.log(`PASS ${theme}/${layout}/${dial}: truthful rules, dictionary disconnected, test-bound examples`);
      }
      await page.close();
    }
    assert.deepEqual(errors, []);
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
