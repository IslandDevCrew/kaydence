const assert = require('node:assert/strict');
const fs = require('node:fs');
const zlib = require('node:zlib');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const { PNG } = require(require.resolve('pngjs', { paths: [process.env.PLAYWRIGHT_MODULE || process.cwd()] }));
const url = process.env.APP_URL || 'http://127.0.0.1:1442';
const results = [], errors = [];
const axes = [['mac', '#14a7a1'], ['windows', '#0067c0'], ['linux', '#1f9d63']];

function luminance(color) {
  return color.map(value => value / 255)
    .map(value => value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4)
    .reduce((sum, value, index) => sum + value * [.2126, .7152, .0722][index], 0);
}
function contrast(a, b) {
  return (Math.max(luminance(a), luminance(b)) + .05) / (Math.min(luminance(a), luminance(b)) + .05);
}
function near(a, b, tolerance = 2) { return a.every((value, index) => Math.abs(value - b[index]) <= tolerance); }
function pixel(png, [x, y]) {
  x = Math.floor(x); y = Math.floor(y);
  if (x < 0 || y < 0 || x >= png.width || y >= png.height) return null;
  return [...png.data.subarray((y * png.width + x) * 4, (y * png.width + x) * 4 + 3)];
}

async function focusState(page) {
  return page.evaluate(() => {
    const element = document.activeElement, style = getComputedStyle(element), rect = element.getBoundingClientRect();
    const rgba = color => {
      const srgb = color.match(/^color\(srgb ([\d.\s-]+?)(?: \/ ([\d.]+))?\)$/);
      if (srgb) return srgb[1].trim().split(/\s+/).map(value => +value * 255).concat(+(srgb[2] ?? 1));
      const rgb = color.match(/^rgba?\(([^)]+)\)$/);
      if (!rgb) throw new Error(`Unsupported color: ${color}`);
      const channels = rgb[1].split(',').map(Number);
      return channels.length === 3 ? channels.concat(1) : channels;
    };
    const backgroundAt = point => {
      const hit = document.elementFromPoint(...point), chain = [];
      if (!hit) return null;
      for (let parent = hit; parent; parent = parent.parentElement) chain.unshift(parent);
      let background = [255, 255, 255], unsupported = false;
      for (const parent of chain) {
        const css = getComputedStyle(parent), color = rgba(css.backgroundColor);
        unsupported ||= css.backgroundImage !== 'none' || +css.opacity !== 1 || css.filter !== 'none' || css.mixBlendMode !== 'normal';
        background = background.map((value, index) => color[index] * color[3] + value * (1 - color[3]));
      }
      return { color: background, unsupported, hit: hit.className || hit.tagName };
    };
    const width = parseFloat(style.outlineWidth), offset = parseFloat(style.outlineOffset);
    const inside = offset + width <= 0, probes = [];
    // Interior and exterior outlines use their respective adjacent painted surfaces.
    for (const side of ['top', 'bottom', 'left', 'right']) {
      for (const fraction of [.25, .5, .75]) for (let depth = .5; depth < width; depth += .5) {
        const distance = inside ? -offset + 2 : offset + width + 2;
        let point, adjacent;
        if (side === 'top' || side === 'bottom') {
          const x = rect.x + rect.width * fraction;
          point = [x, side === 'top' ? rect.top - offset - depth : rect.bottom + offset + depth];
          adjacent = [x, side === 'top' ? rect.top + (inside ? distance : -distance) : rect.bottom + (inside ? -distance : distance)];
        } else {
          const y = rect.y + rect.height * fraction;
          point = [side === 'left' ? rect.left - offset - depth : rect.right + offset + depth, y];
          adjacent = [side === 'left' ? rect.left + (inside ? distance : -distance) : rect.right + (inside ? -distance : distance), y];
        }
        probes.push({ side, point, adjacent, behind: backgroundAt(point), beside: backgroundAt(adjacent) });
      }
    }
    return {
      identity: [...document.querySelectorAll('*')].indexOf(element), tag: element.tagName,
      name: element.getAttribute('aria-label') || element.textContent.trim().slice(0, 90),
      selector: element.className || `${element.parentElement?.className} > ${element.tagName.toLowerCase()}`,
      outline: style.outlineColor, color: rgba(style.outlineColor), style: style.outlineStyle,
      interactive: element.matches('button, [href], input, select, textarea, summary, [tabindex]') && element.tabIndex >= 0,
      visible: element.matches(':focus-visible'), width, offset, inside, rect: rect.toJSON(),
      geometry: [rect.x, rect.y, rect.width, rect.height, width, offset, style.font, style.borderRadius], probes,
    };
  });
}

function measure(state, buffer) {
  const png = PNG.sync.read(buffer), samples = [];
  for (const probe of state.probes) {
    if (!probe.behind || !probe.beside || probe.behind.unsupported || probe.beside.unsupported) continue;
    const paint = pixel(png, probe.point), adjacent = pixel(png, probe.adjacent);
    if (!paint || !adjacent) continue;
    const expected = probe.behind.color.map((value, index) => state.color[index] * state.color[3] + value * (1 - state.color[3]));
    // Reject antialiasing fringes, text/border pixels, and unsupported paint stacks.
    if (!near(paint, expected) || !near(adjacent, probe.beside.color)) continue;
    samples.push({ side: probe.side, point: probe.point, adjacentPoint: probe.adjacent,
      paint, adjacent, expected, background: probe.beside.color,
      ratio: contrast(expected, probe.beside.color), pixelRatio: contrast(paint, adjacent) });
  }
  return samples;
}

async function seek(page, target) {
  for (let step = 0; step < 70; step++) {
    if (await target.evaluate(element => element === document.activeElement && element.matches(':focus-visible'))) return;
    await page.keyboard.press('Tab');
  }
  throw new Error(`Natural Tab did not reach ${await target.innerText()}`);
}

function continuousBands(state, samples, buffer) {
  const png = PNG.sync.read(buffer), bands = [];
  const padding = Math.max(4, parseFloat(state.geometry[7]) + 2);
  for (const side of ['top', 'bottom', 'left', 'right']) {
    const candidates = samples.filter(sample => sample.side === side && sample.ratio >= 3);
    const axis = side === 'top' || side === 'bottom' ? 0 : 1;
    const start = Math.ceil((axis === 0 ? state.rect.left : state.rect.top) + padding);
    const end = Math.floor((axis === 0 ? state.rect.right : state.rect.bottom) - padding);
    for (const sample of candidates) {
      const ratios = [];
      for (let position = start; position <= end; position++) {
        const point = [...sample.point], adjacentPoint = [...sample.adjacentPoint];
        point[axis] = adjacentPoint[axis] = position;
        const paint = pixel(png, point), adjacent = pixel(png, adjacentPoint);
        if (!paint || !adjacent || !near(paint, sample.expected) || !near(adjacent, sample.background)) break;
        ratios.push(contrast(paint, adjacent));
      }
      if (ratios.length >= 3 && ratios.length === end - start + 1 && ratios.every(ratio => ratio >= 3)) {
        bands.push({ side, start, end, point: sample.point, adjacentPoint: sample.adjacentPoint,
          pixels: ratios.length, ratio: sample.ratio, pixelMinimum: Math.min(...ratios) });
        break;
      }
    }
  }
  return bands;
}

async function walk(page, key, startWithTab = true) {
  const seen = new Set(), rows = [];
  if (startWithTab) await page.keyboard.press('Tab');
  for (let step = 0; step < 70; step++) {
    const state = await focusState(page);
    if (seen.has(state.identity)) break;
    seen.add(state.identity);
    const missing = state.interactive && (!state.visible || state.style === 'none' || state.width === 0);
    if (missing || (state.visible && state.style === 'solid')) {
      const buffer = await page.screenshot({ animations: 'disabled' });
      const samples = missing ? [] : measure(state, buffer);
      // A continuous flat side is stronger evidence than three isolated probes.
      // Independent visual review still decides whether that cue identifies focus.
      const qualifyingBands = continuousBands(state, samples, buffer);
      const qualifyingSides = qualifyingBands.map(band => band.side);
      const { probes, ...summary } = state;
      rows.push({ ...summary, samples, qualifyingSides, qualifyingBands, missing, unresolved: samples.length === 0,
        failed: missing || (samples.length > 0 && qualifyingSides.length === 0) });
      const capture = process.env.CAPTURE_DIR && [
        'mac-light-900-Dictate/Dictate', 'mac-light-450-Cleanup/raw',
        'mac-light-450-Privacy/Privacy Constitution', 'mac-dark-900-Dictate/Dictate',
        'windows-dark-900-Setup/Refresh proof', 'linux-dark-450-Setup/Evidence',
        'mac-light-900-Privacy-dialog/Close privacy constitution',
        'linux-dark-900-Setup-models/Close setup evidence',
      ].includes(`${key}/${state.name}`);
      if (capture) await fs.promises.writeFile(`${process.env.CAPTURE_DIR}/${key}-${state.name.replace(/\W+/g, '-')}.png`, buffer);
    }
    await page.keyboard.press('Tab');
  }
  assert(rows.length, `${key}: meaningful authored-focus controls`);
  results.push({ key, rows });
  console.log(`${key}: ${rows.length} states, ${rows.filter(row => row.failed).length} below3, ${rows.filter(row => row.unresolved).length} unresolved`);
}

(async () => {
  const browser = await chromium.launch();
  try {
    for (const [os, accent] of axes) for (const theme of ['light', 'dark']) for (const width of [900, 450]) {
      for (const view of ['Dictate', 'Cleanup', 'Privacy', 'Setup']) {
        const page = await browser.newPage({ viewport: { width, height: width === 900 ? 600 : 300 },
          colorScheme: theme, reducedMotion: 'reduce',
          userAgent: `Mozilla/5.0 (${os}) AppleWebKit/537.36 Chrome/145.0.0.0 Safari/537.36` });
        page.on('pageerror', error => errors.push(error.message));
        page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
        await page.goto(url);
        assert.equal(await page.title(), 'Kaydence');
        assert.equal(await page.locator('vite-error-overlay').count(), 0);
        if (view !== 'Dictate') await page.locator('.nav-list').getByRole('button', { name: view, exact: true }).click();
        await page.mouse.move(0, 0);
        assert.equal(await page.locator('.app-shell').evaluate(element => getComputedStyle(element).getPropertyValue('--accent').trim()), accent);
        const key = `${os}-${theme}-${width}-${view}`;
        await walk(page, key);
        if (view === 'Privacy' || view === 'Setup') {
          const opener = page.getByRole('button', { name: view === 'Privacy' ? 'Privacy Constitution' : 'Evidence', exact: true });
          await seek(page, opener);
          await page.keyboard.press('Enter');
          await page.getByRole('dialog').waitFor();
          if (view === 'Setup') {
            for (const section of ['models', 'permissions', 'proof']) {
              await seek(page, page.getByRole('dialog').getByRole('button', { name: section, exact: true }));
              await page.keyboard.press('Enter');
              await walk(page, `${key}-${section}`, false);
            }
          } else await walk(page, `${key}-dialog`, false);
          await page.keyboard.press('Escape');
        }
        await page.close();
      }
    }
    if (process.env.RESULT_PATH) fs.writeFileSync(process.env.RESULT_PATH, zlib.gzipSync(JSON.stringify({ engine: browser.version(), results, errors })));
    if (process.env.BASELINE_PATH) {
      const baseline = JSON.parse(zlib.gunzipSync(fs.readFileSync(process.env.BASELINE_PATH))).results;
      const geometry = data => data.map(scope => [scope.key, scope.rows.map(row => [row.name, row.geometry])]);
      assert.deepEqual(geometry(results), geometry(baseline), 'Color-only change preserves focused geometry');
    }
    const rows = results.flatMap(scope => scope.rows);
    assert.deepEqual(errors, [], 'No frontend errors');
    assert.equal(rows.filter(row => row.unresolved).length, 0, 'Unresolved paint is not a pass');
    assert.equal(rows.filter(row => row.failed).length, 0, 'Authored focus indicators need 3:1 against actual adjacent colors');
    const qualifying = rows.flatMap(row => row.samples.filter(sample => row.qualifyingSides.includes(sample.side)));
    console.log(`PASS ${results.length} scopes / ${rows.length} states; qualifying-side minimum ${Math.min(...qualifying.map(sample => sample.ratio))}`);
    console.log(`All-sample minimum ${Math.min(...rows.flatMap(row => row.samples.map(sample => sample.ratio)))}; adjacent-button overlaps retained, not an all-pixels or AAA-area claim`);
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
