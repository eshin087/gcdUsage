/* Tests the display options in an isolated native app. Uses the same launch
 * environment as native-smoke.cjs. Captures only local screenshots. */
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { chromium } = require(process.env.GCD_PLAYWRIGHT_MODULE || 'playwright');

async function main() {
  const out = path.resolve(process.env.GCD_QA_OUTPUT_DIR || '.local-test/display-qa');
  await fs.mkdir(out, { recursive: true });
  const browser = await chromium.connectOverCDP('http://127.0.0.1:9223');
  let page;
  for (let i = 0; i < 150 && !page; i++) {
    page = browser.contexts().flatMap(context => context.pages()).find(candidate => candidate.url().startsWith('http://tauri.localhost'));
    if (!page) await new Promise(resolve => setTimeout(resolve, 100));
  }
  assert.ok(page, 'Expected native app dashboard');
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.waitForFunction(() => !!window.__TAURI_INTERNALS__?.invoke);
  await page.waitForFunction(async () => {
    const overview = await window.__TAURI_INTERNALS__.invoke('get_overview');
    return !overview.importing && overview.snapshots.length === 2;
  }, null, { timeout: 120000 });
  const overview = () => page.evaluate(() => window.__TAURI_INTERNALS__.invoke('get_overview'));
  const initial = await overview();
  assert.equal(initial.settings.theme, 'black');
  assert.equal(initial.settings.meterDisplay, 'remaining');
  assert.equal(initial.settings.stripLocked, false);
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'black');
  assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).backgroundColor), 'rgb(0, 0, 0)');

  async function checkMeters(mode) {
    await page.locator('nav').getByRole('button', { name: /^Overview/ }).click();
    const current = await overview();
    const spec = [['claude', 300], ['claude', 10080], ['codex', 10080]];
    for (let i = 0; i < spec.length; i++) {
      const [provider, duration] = spec[i];
      const snapshot = current.snapshots.find(s => s.provider === provider);
      const window = snapshot?.windows.find(w => w.id === provider + ':' + duration);
      const card = page.locator('.quota-card').nth(i);
      if (window) {
        const percent = mode === 'remaining' ? 100 - window.usedPercent : window.usedPercent;
        assert.equal(Number(await card.getByRole('progressbar').getAttribute('aria-valuenow')), percent);
        assert.match(await card.locator('.quota-value').innerText(), new RegExp(Math.round(percent) + '%\\s*' + (mode === 'remaining' ? 'left' : 'used')));
        if (window.resetsAt == null) assert.match(await card.locator('.quota-footer').innerText(), /Reset N\/A/);
      } else {
        assert.match(await card.locator('.quota-value').innerText(), /—/);
      }
    }
  }
  await checkMeters('remaining');
  await page.screenshot({ path: path.join(out, 'black-overview.png'), fullPage: false });

  async function save() {
    await page.getByRole('button', { name: /^(Save changes|Finish setup)$/ }).click();
    await page.getByText(/^(Settings saved\.|All set\. Your meters keep running when you close this window\.)$/).waitFor();
  }
  async function settingsPage() {
    await page.locator('nav').getByRole('button', { name: /^Settings/ }).click();
    await page.getByRole('combobox', { name: 'Color theme' }).waitFor();
  }
  const palettes = {};
  for (const theme of ['slate', 'midnight', 'light', 'system', 'black']) {
    await settingsPage();
    await page.getByRole('combobox', { name: 'Color theme' }).selectOption(theme);
    await save();
    assert.equal((await overview()).settings.theme, theme);
    await page.waitForFunction(theme => document.documentElement.dataset.theme === (theme === 'system' ? (matchMedia('(prefers-color-scheme: dark)').matches ? 'slate' : 'light') : theme), theme);
    palettes[theme] = await page.evaluate(() => ({ background: getComputedStyle(document.documentElement).backgroundColor, theme: document.documentElement.dataset.theme }));
    if (theme === 'midnight' || theme === 'light') {
      await settingsPage();
      await page.screenshot({ path: path.join(out, theme + '-settings.png'), fullPage: false });
    }
  }
  assert.equal(palettes.black.background, 'rgb(0, 0, 0)');
  assert.equal(new Set(['black', 'slate', 'midnight', 'light'].map(key => palettes[key].background)).size, 4);

  for (const mode of ['used', 'remaining']) {
    await settingsPage();
    await page.getByRole('combobox', { name: 'Meter percentages' }).selectOption(mode);
    await save();
    assert.equal((await overview()).settings.meterDisplay, mode);
    await checkMeters(mode);
  }
  for (const anchored of [true, false]) {
    await settingsPage();
    await page.getByRole('checkbox', { name: /^Lock strip position/ }).setChecked(anchored);
    await save();
    assert.equal((await overview()).settings.stripLocked, anchored);
  }
  await page.reload();
  await page.waitForFunction(() => !!window.__TAURI_INTERNALS__?.invoke);
  const final = await overview();
  assert.equal(final.settings.theme, 'black');
  assert.equal(final.settings.meterDisplay, 'remaining');
  assert.equal(final.settings.stripLocked, false);
  assert.equal(final.settings.deviceId, initial.settings.deviceId);
  await checkMeters('remaining');
  assert.deepEqual(errors, []);
  const report = { palettes, percentageModes: ['remaining', 'used'], positionLockModes: [true, false], persistedAfterReload: true, browserErrors: errors };
  await fs.writeFile(path.join(out, 'display-report.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
  const closed = page.waitForEvent('close', { timeout: 10000 });
  await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('plugin:window|close', { label: 'dashboard' })).catch(error => {
    if (!/closed|crashed|destroyed/i.test(String(error))) throw error;
  });
  await closed;
  await browser.close().catch(() => {});
}
main().catch(error => { console.error(error); process.exitCode = 1; });
