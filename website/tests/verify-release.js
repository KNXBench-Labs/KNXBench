/** Playwright CLI acceptance for the public release build served on 127.0.0.1:4198. */
async (page) => {
  const origin = 'http://127.0.0.1:4198';
  const checks = [], unexpected = [], errors = [];
  const assert = (name, condition) => { if (!condition) throw new Error(name); checks.push(name); };
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.route('**/*', route => {
    if (new URL(route.request().url()).origin === origin) return route.continue();
    unexpected.push(route.request().url());
    return route.abort();
  });
  for (const path of ['/', '/de/', '/en/']) {
    await page.goto(`${origin}${path}`);
    assert(`${path} has no preview banner`, await page.locator('.preview-note').count() === 0);
    assert(`${path} has no launch note`, await page.locator('.launch-note').count() === 0);
    assert(`${path} is indexable`, await page.locator('meta[name="robots"]').count() === 0);
    assert(`${path} still renders the hero`, await page.locator('h1').isVisible());
  }
  await page.goto(`${origin}/story/`);
  assert('story is the published variant', (await page.locator('.masthead-note').textContent()).includes('Published edition'));
  assert('story has no preview banner', await page.locator('.preview-banner').count() === 0);
  assert('story renders its chapters', await page.locator('.chapter-section').count() > 0);
  for (const [lang, title] of [['de', 'Datenschutz'], ['en', 'Privacy']]) {
    await page.goto(`${origin}/${lang}/privacy/`);
    assert(`${lang} privacy page has its release title`, (await page.locator('h1').textContent()) === title);
    assert(`${lang} privacy page names the host`, (await page.locator('main').textContent()).includes('GitHub Pages'));
    assert(`${lang} privacy page links GitHub's statement`, await page.locator('a[href^="https://docs.github.com/"]').count() === 1);
  }
  const robots = await (await page.request.get(`${origin}/robots.txt`)).text();
  assert('robots.txt allows crawling', robots === 'User-agent: *\nAllow: /\n');
  return { checks, count: checks.length, unexpected, errors, scope: 'Chromium; release artifact on loopback; not the deployed host' };
}
