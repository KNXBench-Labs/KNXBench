/** Chromium CLI check of real device tabs using only local synthetic API fixtures. */
async (page) => {
  const checks = [];
  const check = (condition, label) => { if (!condition) throw new Error(label); checks.push(label); };
  const errors = [];
  const unexpected = [];
  const requests = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
  const field = (etsId, access, value = '5') => ({
    etsId, name: etsId, text: null, nameLanguage: null, textLanguage: null,
    kind: 'Number', value, valueSource: 'Stored', editable: access === null,
    writeEtsId: access === null ? etsId : null, access, min: '0', max: '10',
    enumOptions: [], displayOrder: null,
  });
  const response = (value = '5') => ({
    programId: 'EXAMPLE', sourceLanguage: null,
    tree: { schema_version: 11, errors: 0, warnings: 0, can_undo: true, can_redo: false, is_modified: true, group_address_style: 'ThreeLevel', installations: [] },
    sections: [{ scope: null, fields: [field('Delay', null, value), field('Manufacturer internal value', 'None'), field('Manufacturer reference value', 'Read')] }],
    stale: [{ etsId: 'Legacy retained value', raw: '99' }],
    diagnostics: [
      { scope: null, kind: 'parameterAccessReadOnly', severity: 'warning', message: 'Manufacturer access restriction', detail: 'Access Read: manufacturer reference; Access None: internal value' },
      ...Array.from({ length: 6 }, (_, i) => ({ scope: null, kind: 'noBranchMatched', severity: 'info', message: 'A choice did not match any of its options.', detail: `NoBranchMatched { choose_node: ${i}, retained_detail: "${'x'.repeat(100)}" }` })),
    ],
  });
  await page.route('**/*', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.origin !== 'http://127.0.0.1:4194') { unexpected.push(request.url()); return route.abort(); }
    if (!url.pathname.startsWith('/api/')) return route.continue();
    requests.push({ method: request.method(), path: url.pathname });
    if (url.pathname === '/api/device/42/parameters' && request.method() === 'GET') {
      return route.fulfill({ contentType: 'application/json', body: JSON.stringify(response()) });
    }
    if (url.pathname === '/api/device/42/parameters' && request.method() === 'POST') {
      const body = request.postDataJSON();
      check(body.etsId === 'Delay' && body.raw === '6', 'write names only the user-accessible parameter');
      return route.fulfill({ contentType: 'application/json', body: JSON.stringify(response('6')) });
    }
    unexpected.push(request.method() + ' ' + url.pathname);
    return route.abort();
  });
  for (const lang of ['en', 'de']) for (const width of [1440, 400]) for (const theme of ['porcelain', 'graphite', 'lcars']) {
    const prefix = `${lang}/${width}/${theme}`;
    await page.setViewportSize({ width, height: 900 });
    await page.goto(`http://127.0.0.1:4194/e2e/device-editor-fixture.html?lang=${lang}`);
    await page.evaluate((theme) => { document.documentElement.dataset.theme = theme; if (theme === 'lcars') document.documentElement.dataset.presentation = 'lcars'; }, theme);
    const tabs = page.getByRole('tab');
    await tabs.nth(1).click();
    const visible = page.locator('[role="tabpanel"]:visible');
    await visible.locator('[data-ets-id="Delay"] input').waitFor();
    check(await tabs.count() === 5, `${prefix}: five tabs`);
    check(await tabs.nth(3).textContent() === (lang === 'de' ? 'Diagnose' : 'Diagnostics'), `${prefix}: translated Diagnostics tab`);
    check(await tabs.nth(4).textContent() === (lang === 'de' ? 'Herstellerfelder' : 'Manufacturer fields'), `${prefix}: translated Manufacturer fields tab`);
    check(await visible.locator('.parameter-field').count() === 1, `${prefix}: only user field in editor`);
    check(await visible.locator('.parameter-diagnostics-banner, .parameter-hidden-toggle, .parameter-stale-section').count() === 0, `${prefix}: clean editor`);
    const getCount = requests.filter((request) => request.method === 'GET').length;
    await tabs.nth(3).click();
    check(await visible.locator('li[data-severity]').count() === 2, `${prefix}: repeated notes grouped into one cause`);
    const info = visible.locator('li[data-severity="info"]');
    check((await info.innerText()).includes(lang === 'de' ? '6 Vorkommen' : '6 occurrences'), `${prefix}: occurrence count preserved`);
    check((await info.innerText()).includes(lang === 'de' ? 'Steuerwert' : 'current controlling value'), `${prefix}: concise cause`);
    check(await info.locator('pre:visible').count() === 0, `${prefix}: raw details initially collapsed`);
    await info.locator('.parameter-diagnostic-details > summary').click();
    check(await info.locator('pre:visible').count() === 6, `${prefix}: all six technical records inspectable`);
    check((await visible.innerText()).includes('Legacy retained value: 99'), `${prefix}: unmatched stored value preserved`);
    const layout = await page.evaluate(() => ({ width: innerWidth, document: document.documentElement.scrollWidth,
      clipped: [...document.querySelectorAll('.device-tabs button, .parameter-diagnostics-banner, .parameter-diagnostic-details pre')]
        .filter((element) => element.getBoundingClientRect().width > 0 && element.scrollWidth > element.clientWidth + 1).map((element) => element.className) }));
    check(layout.document <= width + 1 && layout.clipped.length === 0, `${prefix}: no clipped tabs or technical details (${JSON.stringify(layout)})`);
    await tabs.nth(4).click();
    check(await visible.locator('.parameter-field').count() === 2, `${prefix}: both restricted fields preserved`);
    check(await visible.locator('input:disabled').count() === 2, `${prefix}: restricted fields cannot be edited`);
    check((await visible.innerText()).includes('Access None') && (await visible.innerText()).includes('Access Read'), `${prefix}: exact access reasons`);
    check(requests.filter((request) => request.method === 'GET').length === getCount, `${prefix}: tab switches share one fetch`);
    await tabs.nth(1).click();
    await visible.locator('input').fill('6');
    const applied = page.waitForResponse((reply) => reply.url().includes('/api/device/42/parameters') && reply.request().method() === 'POST');
    await visible.locator('input').press('Tab');
    check((await applied).ok(), `${prefix}: edit response accepted`);
    await page.waitForFunction(() => document.querySelector('[data-ets-id="Delay"] input')?.value === '6');
    await tabs.nth(3).click();
    check(await visible.locator('li[data-severity]').count() === 2, `${prefix}: shared updated response retains diagnostics`);
    await tabs.nth(4).click();
    await tabs.nth(4).focus();
    check(await tabs.nth(4).getAttribute('aria-selected') === 'true', `${prefix}: Manufacturer fields selected before keyboard navigation`);
    await page.keyboard.press('ArrowRight');
    check(await tabs.nth(0).getAttribute('aria-selected') === 'true', `${prefix}: keyboard wraps across all tabs`);
    await page.keyboard.press('End');
    check(await tabs.nth(4).getAttribute('aria-selected') === 'true', `${prefix}: End reaches Manufacturer fields`);
    check(await page.locator('[role="tabpanel"]:visible').count() === 1, `${prefix}: one visible panel`);
  }
  check(errors.length === 0, `zero browser errors (${JSON.stringify(errors)})`);
  check(unexpected.length === 0, `zero unexpected or external requests (${JSON.stringify(unexpected)})`);
  const result = { status: 'passed', count: checks.length, checks, requests, unexpected, errors,
    scope: 'Actual Inspector/DeviceWorkspace components and shipped palette CSS, synthetic intercepted API; no hardware or production writes.' };
  await page.evaluate((result) => { window.__parameterVerification = result; }, result);
  return { status: result.status, count: result.count, requests: requests.length, unexpected, errors };
}
