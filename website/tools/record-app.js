/** CLI-only real-app recording. API answers are fictional; every request is intercepted. */
async (page) => {
  const origin = 'http://127.0.0.1:4197';
  const out = 'website/assets';
  const frames = 'website/output/playwright/group-frames';
  const requests = [], unexpected = [], errors = [], checks = [];
  const assert = (name, ok) => { if (!ok) throw new Error(name); checks.push(name); };
  const names = ['Living room ceiling light', 'Kitchen light', 'Hall light', 'Bedroom light', 'Living room ceiling light status', 'Kitchen light status', 'Hall light status', 'Bedroom light status', 'Living room blind up/down', 'Kitchen blind up/down', 'Living room temperature', 'Bedroom temperature', 'Living room setpoint', 'Bedroom setpoint', 'Heating mode'];
  let settings = { theme: 'graphite', uiLanguage: 'en', accent: 'mint', density: 'comfortable', motionLevel: 'off', motionStyle: 'apple', uiThemePacks: {}, onboardingGuide: { seenStage: 'alpha' }, autosaveEnabled: 'false', achievementsEnabled: 'false' };
  const tree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, server_incarnation: 'website-offline-demo', snapshot_revision: 1, group_address_style: 'ThreeLevel', has_store_path: false,
    installations: [{ id: 1, name: 'Sample house (fictional)', topology: [], buildings: [], unassigned: [], group_ranges: [], group_addresses: names.map((name, i) => ({ id: 100 + i, name, address: `0/${i < 8 ? 0 : 1}/${i < 8 ? i + 1 : i - 7}`, range: null, dpts: [i < 10 ? 'DPST-1-1' : 'DPST-9-1'], links: [] })) }] };
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', async route => {
    const url = new URL(route.request().url()), method = route.request().method();
    if (url.origin !== origin) { unexpected.push(`${method} external`); return route.abort(); }
    if (!url.pathname.startsWith('/api/')) return route.continue();
    requests.push(`${method} ${url.pathname}`);
    const reply = json => route.fulfill({ json });
    if (url.pathname === '/api/auth/status' && method === 'GET') return reply({ required: false, authenticated: true });
    if (url.pathname === '/api/settings' && ['GET', 'PUT'].includes(method)) {
      if (method === 'PUT') settings = { ...settings, ...route.request().postDataJSON().settings };
      return reply({ schemaVersion: 1, conditionalPatchVersion: 1, status: 'ok', settings });
    }
    if (url.pathname === '/api/project' && method === 'GET') return reply(tree);
    if (url.pathname === '/api/project/new' && method === 'POST') return reply(tree);
    if (url.pathname === '/api/bus/discover' && method === 'POST') return reply({ interfaces: [] });
    if (['/api/product-languages', '/api/catalog/manufacturers', '/api/catalog/items', '/api/fs/list'].includes(url.pathname) && method === 'GET') return reply([]);
    if (url.pathname === '/api/version' && method === 'GET') return reply({ version: '0.1.0-alpha.5+goffline-demo' });
    if (url.pathname === '/api/achievements' && method === 'GET') return reply({ schemaVersion: 1, status: 'absent', unlocked: {}, progress: {} });
    if (url.pathname === '/api/achievements/record' && method === 'POST') return reply({ schemaVersion: 1, status: 'ok', unlocked: {}, progress: {} });
    unexpected.push(`${method} ${url.pathname}`); return route.fulfill({ status: 404, json: { error: 'not admitted by offline media fixture' } });
  });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const nav = () => page.getByRole('navigation', { name: 'Navigation', exact: true });
  await page.goto(origin);
  await page.getByRole('button', { name: 'New project…', exact: true }).click();
  await page.getByRole('dialog', { name: 'New project', exact: true }).getByRole('button', { name: 'Create project', exact: true }).click();
  await page.getByRole('button', { name: 'Done', exact: true }).click();
  await nav().getByRole('button', { name: 'Group addresses', exact: true }).click();
  const rows = () => page.locator('.address-table tbody > tr');
  await rows().first().waitFor();
  assert('real production workbench renders 15 fictional addresses', await rows().count() === 15);
  await rows().nth(1).getByRole('button', { name: '0/0/2', exact: true }).click();
  assert('real inspector selects the kitchen light', (await page.locator('.workbench-pane-right').textContent()).includes('Kitchen light'));
  await page.screenshot({ path: `${out}/graphite.png` });
  let frame = 0;
  const record = async count => {
    for (let i = 0; i < count; i++) {
      await page.screenshot({ path: `${frames}/${String(frame++).padStart(4, '0')}.png` });
      await page.waitForTimeout(180);
    }
  };
  await record(10);
  await page.locator('.address-filter').fill('Kitchen');
  assert('actual application filter narrows the demo project', await rows().count() === 3);
  await record(12);
  await rows().first().getByRole('button', { name: '0/0/2', exact: true }).click();
  await record(12);
  await page.locator('.address-filter').fill('');
  await record(10);
  const settingsButton = () => page.locator('.workbench-toolbar').getByRole('button', { name: 'Settings', exact: true });
  for (const [theme, filename] of [['user-modern-retro-green-crt', 'crt'], ['lcars', 'lcars']]) {
    await settingsButton().click();
    const selector = page.getByRole('combobox', { name: 'Theme', exact: true });
    await selector.selectOption(theme);
    await page.waitForTimeout(250);
    await selector.focus();
    await page.keyboard.press('Escape');
    await page.locator('.settings-panel').waitFor({ state: 'detached' });
    assert(`${filename} uses the app's real theme`, await page.locator('html').getAttribute('data-theme') === theme);
    await page.screenshot({ path: `${out}/${filename}.png` });
  }
  assert('no unhandled/external requests escaped the fixture', unexpected.length === 0);
  assert('no page script errors', errors.length === 0);
  return { checks, count: checks.length, requests, unexpected, errors, frames: frame, themeWrites: 'synthetic only', source: 'production frontend, fictional API answers; no live KNX server' };
}
