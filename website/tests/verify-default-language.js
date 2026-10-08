/** Playwright CLI acceptance: English root, explicit DE/EN and no locale redirect. */
async (page) => {
  const origin = 'http://127.0.0.1:4198';
  const checks = [], errors = [], unexpected = [];
  const assert = (name, condition) => { if (!condition) throw new Error(name); checks.push(name); };
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.route('**/*', route => {
    if (new URL(route.request().url()).origin === origin) return route.continue();
    unexpected.push(route.request().url()); return route.abort();
  });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const width of [320, 390, 768, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(`${origin}/`);
    assert(`root ${width}px is English without redirect`, await page.locator('html').getAttribute('lang') === 'en' && new URL(page.url()).pathname === '/');
    assert(`root ${width}px has no horizontal overflow`, await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
  }
  assert('root hero has English copy and accessible name', await page.locator('h1').getAttribute('aria-label') === 'Your KNX project. Your workbench.');
  assert('English switch is marked as current', await page.locator('.language-nav a[lang="en"]').getAttribute('aria-current') === 'page');
  assert('root videos use English captions', await page.locator('video track').evaluateAll(tracks => tracks.length === 3 && tracks.every(t => t.srclang === 'en' && t.getAttribute('src').endsWith('-en.vtt'))));
  assert('root canonical identifies the English edition', await page.locator('link[rel="canonical"]').getAttribute('href') === 'https://knxbench.com/en/');
  await page.screenshot({ path: 'website/output/playwright/default-english.png' });
  await page.locator('.language-nav a[lang="de"]').click();
  await page.waitForURL(`${origin}/de/`);
  assert('German remains explicitly selectable', await page.locator('html').getAttribute('lang') === 'de' && (await page.locator('h1').textContent()).includes('Dein KNX-Projekt.'));
  await page.locator('.language-nav a[lang="en"]').click();
  await page.waitForURL(`${origin}/en/`);
  assert('English remains explicitly selectable', await page.locator('html').getAttribute('lang') === 'en');
  const basic = await page.context().browser().newContext({ locale: 'de-DE', javaScriptEnabled: false });
  try {
    const plain = await basic.newPage();
    await plain.goto(`${origin}/`);
    assert('German browser locale and no JS still get English root', await plain.locator('html').getAttribute('lang') === 'en' && (await plain.locator('h1').innerText()).includes('Your KNX project.'));
    await plain.locator('.site-footer a').filter({ hasText: 'Legal notice' }).click();
    await plain.waitForURL(`${origin}/en/contact/`);
    assert('root English legal-notice link works without JS', await plain.locator('h1').textContent() === 'Legal notice');
  } finally { await basic.close(); }
  assert('no external requests', unexpected.length === 0);
  assert('no page or console errors', errors.length === 0);
  return { checks, count: checks.length, errors, unexpected, scope: 'Chromium local static website, default locale only; application language unchanged' };
}
