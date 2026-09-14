const fs = require('node:fs'), vm = require('node:vm'), assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE);
const file = require('node:path').join(__dirname, '2026-09-14-p1-focus-check.cjs');
const source = fs.readFileSync(file, 'utf8');
const context = { require, process, console };
vm.runInNewContext(source.slice(0, source.indexOf('(async () => {')) + '\nglobalThis.api = { walk, results };', context);
(async () => {
  const browser = await chromium.launch();
  try {
    for (const [name, background, outline, expected] of [
      ['missing', 'white', 'none', 'missing'],
      ['weak', 'white', '3px solid rgba(20,167,161,.34)', 'failed'],
      ['unsupported', 'linear-gradient(white, #eee)', '3px solid #14716f', 'unresolved'],
      ['control', 'white', '3px solid #14716f', 'green'],
    ]) {
      const page = await browser.newPage({ viewport: { width: 500, height: 300 } });
      await page.setContent(`<style>body{background:${background}}button{margin:30px;width:120px;height:40px;background:white;outline-offset:2px}button:focus-visible{outline:${outline}}</style><button>Probe</button>`);
      await context.api.walk(page, `negative-${name}`);
      const row = context.api.results.at(-1).rows[0];
      if (expected === 'green') assert(!row.failed && !row.unresolved && !row.missing);
      else assert(row[expected], `Expected ${expected}`);
      console.log(`PASS ${name}: missing=${row.missing} failed=${row.failed} unresolved=${row.unresolved}`);
      await page.close();
    }
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
