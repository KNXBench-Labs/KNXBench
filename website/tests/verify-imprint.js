/** Playwright CLI acceptance for the owner-supplied bilingual legal notice. */
async (page) => {
  const origin = 'http://127.0.0.1:4198';
  const checks = [], errors = [], unexpected = [];
  const assert = (name, condition) => {
    if (!condition) throw new Error(name);
    checks.push(name);
  };
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.route('**/*', route => {
    if (new URL(route.request().url()).origin === origin) return route.continue();
    unexpected.push(route.request().url());
    return route.abort();
  });
  for (const [lang, title] of [['de', 'Impressum'], ['en', 'Legal notice']]) {
    await page.goto(`${origin}/${lang}/`);
    const link = page.locator('.site-footer a').filter({ hasText: title });
    assert(`${lang} footer has exactly one legal-notice link`, await link.count() === 1);
    await link.focus();
    await page.keyboard.press('Enter');
    await page.waitForURL(`${origin}/${lang}/contact/`);
    assert(`${lang} keyboard footer link opens legal notice`, new URL(page.url()).pathname === `/${lang}/contact/`);
    assert(`${lang} heading identifies the legal notice`, await page.locator('h1').textContent() === title);
    assert(`${lang} provider details match supplied text exactly`, await page.locator('address').innerText() === 'Andre Becker\nSchusterstrasse 3\n48268 Greven');
    assert(`${lang} email link matches supplied address`, await page.locator('a[href^="mailto:"]').getAttribute('href') === 'mailto:contact@knxbench.com');
    assert(`${lang} no missing-provider placeholder remains`, await page.locator('.launch-note').count() === 0);
    assert(`${lang} address is not italicized`, await page.locator('address').evaluate(el => getComputedStyle(el).fontStyle === 'normal'));
    for (const width of [320, 390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      assert(`${lang} legal notice ${width}px has no horizontal overflow`, await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    }
    await page.screenshot({ path: `website/output/playwright/imprint-${lang}.png`, fullPage: true });
  }
  await page.goto(`${origin}/`);
  await page.locator('.site-footer a').filter({ hasText: 'Legal notice' }).click();
  assert('root English footer opens legal notice', new URL(page.url()).pathname === '/en/contact/');
  const basic = await page.context().browser().newContext({ javaScriptEnabled: false });
  try {
    const plain = await basic.newPage();
    await plain.goto(`${origin}/de/contact/`);
    assert('legal notice is readable without JavaScript', (await plain.locator('address').innerText()).includes('Andre Becker'));
  } finally { await basic.close(); }
  assert('no unexpected external requests', unexpected.length === 0);
  assert('no page or console errors', errors.length === 0);
  return { checks, count: checks.length, errors, unexpected, scope: 'Chromium local preview only; no email sent or legal compliance certification' };
}
