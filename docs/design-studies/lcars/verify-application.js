/** Playwright CLI function: real production workbench, fully intercepted synthetic API.
 * Run against an isolated preview built from the candidate, never a live KNX server.
 * Retrieve window.__lcarsApplicationVerification after run-code.
 */
async (page) => {
  const origin = 'http://127.0.0.1:4189';
  const checks = [], unexpected = [], errors = [], requests = [], writes = [];
  const assert = (name, condition) => { if (!condition) throw new Error(name); checks.push(name); };
  const settle = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  let settings = { theme: 'lcars', uiLanguage: 'en', accent: 'mint', density: 'compact', motionLevel: 'standard', motionStyle: 'apple',
    uiThemePacks: {}, onboardingGuide: { seenStage: 'alpha' }, autosaveEnabled: 'false', achievementsEnabled: 'false', foreign: 'preserved' };
  let tree = { schema_version: 11, errors: 0, warnings: 1, can_undo: false, can_redo: false, is_modified: true,
    server_incarnation: 'lcars-offline-fixture', snapshot_revision: 1, group_address_style: 'ThreeLevel', has_store_path: false,
    installations: [{ id: 1, name: 'LCARS offline fixture', topology: [], buildings: [], unassigned: [], group_ranges: [],
      group_addresses: Array.from({ length: 30 }, (_, i) => ({ id: 100 + i, name: `Lighting ${i + 1}`, address: `1/0/${i + 1}`, range: null, dpts: ['DPST-1-1'], links: [] })) }] };
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error' && !message.text().includes('500 (Internal Server Error)')) errors.push(message.text()); });
  await page.route('**/*', async route => {
    const request = route.request(), url = new URL(request.url()), method = request.method();
    if (url.origin !== origin) { unexpected.push(`${method} ${url.origin}`); await route.abort(); return; }
    if (!url.pathname.startsWith('/api/')) { await route.continue(); return; }
    requests.push(`${method} ${url.pathname}`);
    const reply = (json, status = 200) => route.fulfill({ json, status });
    if (url.pathname === '/api/auth/status' && method === 'GET') return reply({ required: false, authenticated: true });
    if (url.pathname === '/api/settings' && ['GET', 'PUT'].includes(method)) {
      if (method === 'PUT') {
        const payload = request.postDataJSON();
        writes.push(payload.settings);
        if (payload.expectedSettings && Object.entries(payload.expectedSettings).some(([key, value]) => JSON.stringify(settings[key] ?? null) !== JSON.stringify(value))) {
          unexpected.push('conditional precondition mismatch'); return reply({ error: 'fixture conflict' }, 409);
        }
        settings = { ...settings, ...payload.settings };
      }
      return reply({ schemaVersion: 1, conditionalPatchVersion: 1, status: 'ok', settings });
    }
    if (url.pathname === '/api/project' && method === 'GET') return reply(tree);
    if (url.pathname === '/api/project/new' && method === 'POST') return reply(tree);
    if (url.pathname === '/api/bus/discover' && method === 'POST') return reply({ interfaces: [] });
    if (['/api/product-languages', '/api/catalog/manufacturers', '/api/catalog/items', '/api/fs/list'].includes(url.pathname) && method === 'GET') return reply([]);
    if (url.pathname === '/api/version' && method === 'GET') return reply({ version: '0.1.0-alpha.5+gfixture' });
    if (url.pathname === '/api/achievements/record' && method === 'POST') return reply({ schemaVersion: 1, status: 'ok', unlocked: {}, progress: {} });
    if (url.pathname === '/api/achievements' && method === 'GET') return reply({ schemaVersion: 1, status: 'absent', unlocked: {}, progress: {} });
    if (url.pathname === '/api/project/save-as' && method === 'POST') return reply({ error: 'Synthetic save refusal: no file was written' }, 500);
    unexpected.push(`${method} ${url.pathname}`); return reply({ error: 'unhandled offline fixture request' }, 404);
  });
  await page.setViewportSize({ width: 1600, height: 1000 });
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  const create = async () => {
    await page.goto(origin);
    await page.getByRole('button', { name: 'New project…', exact: true }).click();
    await page.getByRole('dialog', { name: 'New project', exact: true }).getByRole('button', { name: 'Create project', exact: true }).click();
    await page.locator('.project-explorer').waitFor();
  };
  const nav = () => page.getByRole('navigation', { name: 'Navigation', exact: true });
  const toolbar = () => page.locator('.workbench-toolbar');
  const openSettings = () => toolbar().getByRole('button', { name: 'Settings', exact: true }).click();
  const setPreference = async (name, value) => {
    await openSettings();
    await page.getByRole('combobox', { name, exact: true }).selectOption(value);
    await settle();
    // selectOption is not a keyboard-focus gesture; focus the actual control
    // after the conditional acknowledgment before sending Escape to its dialog.
    await page.getByRole('combobox', { name, exact: true }).focus();
    await page.keyboard.press('Escape');
    await page.locator('.settings-panel').waitFor({ state: 'detached' });
  };
  const animations = () => page.evaluate(() => document.getAnimations().filter(a => a.animationName === 'lcars-navigation-reveal').length);
  const rows = () => page.locator('.address-table tbody > tr');
  try {
  await create();
  await settle();
  assert('saved LCARS resolves on the actual application', await page.locator('html').getAttribute('data-theme') === 'lcars');
  assert('built-in presentation is active', await page.locator('html').getAttribute('data-presentation') === 'lcars');
  assert('theme does not install packs or rewrite independent settings', writes.length === 0 && settings.density === 'compact' && settings.motionLevel === 'standard' && settings.accent === 'mint');
  const geometry = await toolbar().evaluate(el => ({ radius: getComputedStyle(el).borderTopLeftRadius,
    band: getComputedStyle(el, '::before').backgroundImage, hit: getComputedStyle(el, '::before').pointerEvents }));
  assert('characteristic elbow and segmented band render', geometry.radius === '44px' && geometry.band.includes('linear-gradient') && geometry.hit === 'none');
  await nav().getByRole('button', { name: 'Group addresses', exact: true }).click();
  assert('navigation feedback is a real finite animation', await animations() > 0);
  await page.waitForTimeout(750);
  assert('navigation feedback finishes without an idle loop', await animations() === 0);
  assert('native table has 30 data rows', await rows().count() === 30);
  assert('checkbox and address activation remain separate native controls', await rows().first().getByRole('checkbox').count() === 1 && await rows().first().getByRole('button', { name: '1/0/1', exact: true }).count() === 1);
  await rows().first().getByRole('button', { name: '1/0/1', exact: true }).click();
  assert('row activation selects the actual inspector', await rows().first().getAttribute('aria-selected') === 'true' && await page.locator('.workbench-pane-right').getByRole('heading', { name: 'Lighting 1', exact: true }).count() === 1);
  await rows().nth(1).getByRole('checkbox').check();
  assert('marking another row does not erase first selection', await rows().nth(1).getByRole('checkbox').isChecked() && await rows().first().getAttribute('aria-selected') === 'true');
  await page.locator('.address-filter').fill('1/0/30');
  assert('existing filter is usable', await rows().count() === 1 && (await rows().textContent()).includes('Lighting 30'));
  await page.locator('.address-filter').fill('');
  const rowHeight = await rows().first().evaluate(el => el.getBoundingClientRect().height);
  await setPreference('Density', 'comfortable');
  assert('comfortable density remains independent and changes real rows', await rows().first().evaluate(el => el.getBoundingClientRect().height) > rowHeight && settings.theme === 'lcars');
  await setPreference('Density', 'compact');
  await rows().first().getByRole('button', { name: '1/0/1', exact: true }).focus();
  await page.keyboard.press('Tab'); await page.keyboard.press('Shift+Tab');
  assert('actual table keyboard focus stays visible', await rows().first().getByRole('button', { name: '1/0/1', exact: true }).evaluate(el => el === document.activeElement && getComputedStyle(el).outlineStyle !== 'none'));
  await page.keyboard.press('Control+k');
  assert('existing search shortcut still opens', await page.locator('.search-panel').isVisible());
  await page.keyboard.press('Escape');
  await page.keyboard.press('Control+Shift+p');
  assert('existing command palette shortcut still opens', await page.getByRole('dialog').isVisible());
  await page.keyboard.press('Escape');
  await page.screenshot({ path: 'output/playwright/lcars-application-desktop.png' });

  await nav().getByRole('button', { name: 'Topology', exact: true }).click();
  await page.evaluate(() => document.getAnimations().filter(a => a.animationName === 'lcars-navigation-reveal').forEach(a => a.pause()));
  assert('motion cancellation starts with a live effect', await animations() > 0);
  await setPreference('Motion level', 'off'); await settle();
  assert('motion Off cancels an already running navigation effect', await animations() === 0);
  await nav().getByRole('button', { name: 'Group addresses', exact: true }).click();
  assert('motion Off prevents new feedback', await animations() === 0);
  await setPreference('Motion level', 'standard');
  await nav().getByRole('button', { name: 'Topology', exact: true }).click();
  await page.evaluate(() => document.getAnimations().filter(a => a.animationName === 'lcars-navigation-reveal').forEach(a => a.pause()));
  assert('OS cancellation starts with a live effect', await animations() > 0);
  await page.emulateMedia({ reducedMotion: 'reduce' }); await settle();
  assert('OS reduction cancels the running effect', await animations() === 0);
  await nav().getByRole('button', { name: 'Group addresses', exact: true }).click();
  assert('OS reduction prevents new feedback', await animations() === 0);
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await setPreference('Motion level', 'subtle');
  await nav().getByRole('button', { name: 'Topology', exact: true }).click();
  assert('subtle uses transitions without the standard reveal', await animations() === 0);
  await nav().getByRole('button', { name: 'Group addresses', exact: true }).click();

  const beforeSave = requests.length;
  await toolbar().getByRole('button', { name: 'Save', exact: true }).click();
  await page.getByRole('dialog', { name: 'Save as', exact: true }).getByRole('button', { name: 'Save', exact: true }).click();
  await page.getByRole('alert').filter({ hasText: 'Synthetic save refusal: no file was written' }).waitFor();
  assert('a refused save cannot become a decorative success', tree.is_modified && !(await page.locator('.workbench-status').textContent()).includes('Last saved') && requests.slice(beforeSave).filter(r => r === 'POST /api/project/save-as').length === 1);
  assert('save failure never causes a bus write', !requests.some(r => /\/api\/bus\/(send|write|connect)/.test(r)));
  await page.screenshot({ path: 'output/playwright/lcars-application-error.png' });

  await setPreference('Theme', 'graphite');
  assert('switching away removes the presentation and restores the original shell', await page.locator('html').getAttribute('data-presentation') === null && await toolbar().evaluate(el => getComputedStyle(el).borderTopLeftRadius) === '0px');
  await setPreference('Theme', 'user-modern-retro-green-crt');
  assert('shipped imported-format palette cannot acquire LCARS geometry', await page.locator('html').getAttribute('data-presentation') === null);
  await setPreference('Theme', 'lcars');
  await create(); await nav().getByRole('button', { name: 'Group addresses', exact: true }).click();
  assert('LCARS persists after cold reload', await page.locator('html').getAttribute('data-presentation') === 'lcars' && settings.foreign === 'preserved');
  await page.keyboard.press('Control+='); await page.keyboard.press('Control+='); await page.keyboard.press('Control+='); await page.keyboard.press('Control+='); await page.keyboard.press('Control+=');
  await settle();
  assert('actual application UI scale reaches 1.5 without global horizontal overflow', await page.evaluate(() => getComputedStyle(document.documentElement).zoom === '1.5' && document.documentElement.scrollWidth <= innerWidth));
  await page.screenshot({ path: 'output/playwright/lcars-application-scale.png' });
  await page.keyboard.press('Control+0');
  for (const [width, height] of [[720, 620], [480, 900]]) {
    await page.setViewportSize({ width, height });
    await create();
    await nav().getByRole('button', { name: 'Group addresses', exact: true }).click();
    await rows().first().getByRole('button', { name: '1/0/1', exact: true }).scrollIntoViewIfNeeded();
    const sizes = await page.evaluate(() => {
      const center = document.querySelector('.workbench-center').getBoundingClientRect();
      const row = document.querySelector('.address-table tbody tr').getBoundingClientRect();
      return { overflow: document.documentElement.scrollWidth > innerWidth, visibleRow: row.bottom > Math.max(0, center.top) && row.top < Math.min(innerHeight, center.bottom), centerHeight: center.height };
    });
    assert(`${width}px preserves usable table rows and no global horizontal overflow`, !sizes.overflow && sizes.visibleRow && sizes.centerHeight >= 100);
    await page.screenshot({ path: `output/playwright/lcars-application-${width}.png` });
  }
  assert('all API traffic stayed inside the explicitly intercepted fixture', unexpected.length === 0);
  assert('no unexpected browser errors', errors.length === 0);
  assert('independent preferences and foreign settings survived theme switches', settings.accent === 'mint' && settings.density === 'compact' && settings.motionLevel === 'subtle' && settings.motionStyle === 'apple' && settings.foreign === 'preserved');
  const result = { checks, count: checks.length, unexpected, errors, requests, writes, fixtureOnly: true, productionBuild: true };
  await page.evaluate(value => { window.__lcarsApplicationVerification = value; }, result);
  return result;
  } catch (error) {
    await page.evaluate(value => { window.__lcarsApplicationFailure = value; }, { checks, unexpected, errors, requests, writes, failure: String(error) });
    throw error;
  }
}
