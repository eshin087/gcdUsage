/*
 * Optional smoke test for an already running isolated Windows app.
 * Launch with GCD_USAGE_TEST_MODE=1, GCD_USAGE_DATA_DIR=<isolated folder>, and
 * WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223.
 * Requires Playwright (or GCD_PLAYWRIGHT_MODULE pointing to its local package).
 * Example: GCD_QA_OUTPUT_DIR=<private folder> node scripts/native-smoke.cjs
 * Screenshots may contain local prompt previews. Keep the output directory private.
 * The test edits only the isolated app's settings; it does not reconnect accounts.
 */
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const { chromium } = require(process.env.GCD_PLAYWRIGHT_MODULE || 'playwright');

async function main() {
  const output = path.resolve(process.env.GCD_QA_OUTPUT_DIR || path.join(os.tmpdir(), 'gcd-usage-native-qa'));
  await fs.mkdir(output, { recursive: true });
  const browser = await chromium.connectOverCDP(process.env.GCD_QA_ENDPOINT || 'http://127.0.0.1:9223');
  let page;
  for (let attempt = 0; attempt < 150 && !page; attempt++) {
    page = browser.contexts().flatMap(context => context.pages()).find(candidate => candidate.url().startsWith('http://tauri.localhost'));
    if (!page) await new Promise(resolve => setTimeout(resolve, 100));
  }
  assert.ok(page, 'Expected the native GCD Usage dashboard WebView');
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.waitForFunction(() => !!window.__TAURI_INTERNALS__?.invoke);
  const invoke = (command, args = {}) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args });

  await page.waitForFunction(async (expectHistory) => {
    const current = await window.__TAURI_INTERNALS__.invoke('get_overview');
    return !current.importing && (!expectHistory || current.stats.promptCount > 0);
  }, process.env.GCD_QA_EXPECT_HISTORY === '1', { timeout: 120000, polling: 1000 });
  const overview = await invoke('get_overview');
  assert.ok(overview.settings.deviceId);
  assert.equal(await page.getByText('Design preview', { exact: false }).count(), 0);
  await page.getByRole('heading', { name: 'Your usage, at a glance.' }).waitFor();
  if (process.env.GCD_QA_EXPECT_HISTORY === '1') {
    await page.waitForFunction(() => parseFloat(document.querySelector('.stat-item strong')?.textContent || '') > 0, null, { timeout: 30000 });
  }
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.screenshot({ path: path.join(output, 'native-overview.png'), fullPage: true });

  const history = await invoke('get_history', { filter: { limit: 50, offset: 0 } });
  assert.ok(history.items.length <= 50);
  assert.ok(history.total >= history.items.length);
  await page.locator('nav').getByRole('button', { name: /^History/ }).click();
  await page.getByRole('textbox', { name: 'Search prompt previews' }).waitFor();
  await page.screenshot({ path: path.join(output, 'native-history.png'), fullPage: false });
  if (history.items.length) {
    await page.getByRole('button', { name: 'Show prompt details' }).first().click();
    await page.getByText('PROMPT DETAILS', { exact: true }).waitFor();
    await page.getByRole('textbox', { name: 'Search prompt previews' }).fill('gcd-smoke-no-match-2939160');
    await page.getByText('0 matching prompts', { exact: false }).waitFor();
    await page.getByRole('button', { name: 'Clear filters', exact: true }).click();
  }

  await page.getByRole('button', { name: 'Model advice', exact: true }).click();
  const advice = {};
  for (const [task, label] of [['quick', 'Quick work'], ['everyday', 'Everyday work'], ['complex', 'Complex analysis']]) {
    await page.getByRole('button', { name: label, exact: false }).click();
    const result = await invoke('get_recommendations', { task });
    assert.ok(Array.isArray(result.recommendations));
    advice[task] = result.recommendations.map(item => ({ provider: item.provider, confidence: item.confidence, fitsBudget: item.fitsBudget }));
  }
  await page.screenshot({ path: path.join(output, 'native-advice.png'), fullPage: true });

  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('textbox', { name: 'Computer name', exact: true }).fill('GCD Usage QA');
  await page.getByRole('textbox', { name: 'Computer name', exact: true }).press('Tab');
  await page.getByRole('button', { name: overview.settings.setupComplete ? 'Save changes' : 'Finish setup', exact: true }).click();
  await page.getByText(overview.settings.setupComplete ? 'Settings saved.' : 'All set. Your meters keep running when you close this window.', { exact: true }).waitFor();
  const saved = await invoke('get_overview');
  assert.equal(saved.settings.deviceName, 'GCD Usage QA');
  assert.equal(saved.settings.setupComplete, true);
  assert.equal(saved.settings.deviceId, overview.settings.deviceId);
  await page.reload();
  await page.waitForFunction(() => !!window.__TAURI_INTERNALS__?.invoke);
  const restored = await invoke('get_overview');
  assert.equal(restored.settings.deviceName, 'GCD Usage QA');
  assert.equal(restored.settings.setupComplete, true);

  const report = {
    snapshots: restored.snapshots.map(snapshot => ({ provider: snapshot.provider, status: snapshot.status, message: snapshot.message, windowCount: snapshot.windows.length })),
    history: { prompts: history.total, pageSize: history.items.length, conversations: restored.stats.conversationCount, requests: restored.stats.requestCount, totalTokens: restored.stats.totalTokens },
    importing: restored.importing,
    importFiles: restored.importReport.files,
    importWarnings: restored.importReport.warnings.length,
    advice,
    settingsPersisted: true,
    browserErrors: errors,
  };
  assert.deepEqual(errors, []);
  await fs.writeFile(path.join(output, 'native-report.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
  console.log('Requesting dashboard close; background process should remain.');
  await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('plugin:window|close', { label: 'dashboard' })).catch(error => {
    if (!/closed|Target page|Target crashed|destroyed/i.test(String(error))) throw error;
  });
  console.log('Native smoke passed. Measure process memory after the dashboard closes.');
  await browser.close().catch(() => {});
}
main().catch(error => { console.error(error); process.exitCode = 1; });
