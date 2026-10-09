/** Playwright CLI checks for the static, bilingual offline-demo download handoff. */
async (page) => {
  const origin = new URL(page.url()).origin;
  const checks = [], errors = [], unexpected = [];
  const assert = (name, condition) => { if (!condition) throw new Error(name); checks.push(name); };
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.route('**/*', route => {
    if (new URL(route.request().url()).origin === origin) return route.continue();
    unexpected.push(route.request().url());
    return route.abort();
  });
  const prefix = 'https://github.com/KNXBench-Labs/KNXBench/raw/refs/heads/main/demos/1.0.0/';
  const expected = ['single-family-home-1.0.0.zip', 'multi-unit-residential-1.0.0.zip',
    'office-building-1.0.0.zip', 'knxbench-community-demos-1.0.0.zip', 'SHA256SUMS']
    .map(name => prefix + name);
  expected.push('https://github.com/KNXBench-Labs/KNXBench/blob/main/demos/README.md');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const path of ['/', '/de/', '/en/']) {
    for (const width of [320, 390, 768, 1024, 1440]) {
      await page.setViewportSize({ width, height: 1000 });
      await page.goto(origin + path);
      await page.evaluate(() => document.fonts.ready);
      const demos = page.locator('#demos');
      await demos.scrollIntoViewIfNeeded();
      assert(`${path} ${width}: three project choices`, await demos.locator('article').count() === 3);
      assert(`${path} ${width}: six exact handoff destinations`,
        JSON.stringify((await demos.locator('a[href]').evaluateAll(links => links.map(a => a.href))).sort()) === JSON.stringify([...expected].sort()));
      assert(`${path} ${width}: no page overflow`, await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      const linkGeometry = await demos.locator('a[href]').evaluateAll(links => links.map(a => {
        const box = a.getBoundingClientRect();
        return { text: a.textContent, left: box.left, right: box.right, scroll: a.scrollWidth, client: a.clientWidth, viewport: innerWidth };
      }));
      if (!linkGeometry.every(a => a.left >= 0 && a.right <= a.viewport && a.scroll <= a.client)) {
        throw new Error(`${path} ${width}: demo link overflow ${JSON.stringify(linkGeometry)}`);
      }
      assert(`${path} ${width}: all demo links fit`, true);
      const text = await demos.innerText();
      assert(`${path} ${width}: package counts visible`, ['32', '105', '101', '339', '157', '507'].every(n => text.includes(n)));
      assert(`${path} ${width}: setup and safety disclosed`, text.includes('fictional-demo-devices.knxprod') && text.includes('auto') && (path === '/de/' ? text.includes('niemals auf echte Geräte') : text.includes('never be downloaded to real devices')));
      assert(`${path} ${width}: English guides and frozen version disclosed`, text.includes('1.0.0') && text.includes('local review candidate') && (path === '/de/' ? text.includes('englische Tour') : text.includes('English tour')));
      const first = demos.locator('article a').first();
      await first.focus();
      assert(`${path} ${width}: download link keyboard focus`, await first.evaluate(a => a === document.activeElement));
      if (path === '/de/' && width === 390 || path === '/en/' && width === 1440) {
        await page.screenshot({ path: `website/output/playwright/demos-${width}-${path.slice(1, 3)}.png` });
      }
    }
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto(origin + path);
    const navigation = page.locator('.desktop-nav a[href="#demos"]');
    await navigation.focus(); await page.keyboard.press('Enter');
    assert(`${path}: desktop keyboard navigation reaches demos`, new URL(page.url()).hash === '#demos');
    await page.setViewportSize({ width: 390, height: 1000 });
    await page.goto(origin + path);
    await page.locator('.mobile-menu summary').click();
    await page.locator('.mobile-menu a[href="#demos"]').click();
    assert(`${path}: mobile navigation reaches demos and closes`, new URL(page.url()).hash === '#demos' && !await page.locator('.mobile-menu').evaluate(el => el.open));
  }
  const nojs = await page.context().browser().newContext({ javaScriptEnabled: false, viewport: { width: 390, height: 1000 } });
  try {
    for (const path of ['/', '/de/', '/en/']) {
      const basic = await nojs.newPage();
      await basic.goto(origin + path);
      assert(`${path}: no-JS downloads remain available`, await basic.locator('#demos a[href]').count() === 6);
      await basic.locator('.mobile-menu summary').click();
      await basic.locator('.mobile-menu a[href="#demos"]').click();
      assert(`${path}: no-JS navigation reaches demos`, new URL(basic.url()).hash === '#demos');
      await basic.close();
    }
  } finally { await nojs.close(); }
  assert('no automatic third-party request', unexpected.length === 0);
  assert('no page or console error', errors.length === 0);
  return { checks, count: checks.length, errors, unexpected, scope: 'Static handoff only; download bytes verified separately, not a new app or hardware acceptance.' };
}
