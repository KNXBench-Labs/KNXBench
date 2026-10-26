/** Browser recipe for Alpha 17 copy and retained Alpha 14 media. */
async (page) => {
  const origin = new URL(page.url()).origin;
  const checks = [], errors = [], unexpected = [];
  const assert = (name, condition) => {
    if (!condition) throw new Error(name);
    checks.push(name);
  };
  const assets = ['alpha14-parameters.webp', 'alpha14-vendor-zip.webp', 'alpha14-notes.webp', 'alpha14-diagnostics.webp'];
  for (const javaScriptEnabled of [true, false]) {
    const context = await page.context().browser().newContext({ javaScriptEnabled, reducedMotion: 'reduce' });
    try {
      await context.route('**/*', route => {
        if (new URL(route.request().url()).origin === origin) return route.continue();
        unexpected.push(route.request().url());
        return route.abort();
      });
      const tab = await context.newPage();
      tab.on('pageerror', error => errors.push(error.message));
      tab.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
      for (const lang of ['de', 'en']) {
        for (const width of [320, 390, 768, 1440]) {
          const prefix = `${lang}/${width}/${javaScriptEnabled ? 'js' : 'nojs'}`;
          await tab.setViewportSize({ width, height: 1080 });
          await tab.goto(`${origin}/${lang}/`);
          await tab.evaluate(() => document.fonts.ready);
          const section = tab.locator('#release-features');
          await section.scrollIntoViewIfNeeded();
          assert(`${prefix}: Alpha 17 heading`, (await section.locator('h2').textContent()).includes('Alpha 17'));
          assert(`${prefix}: four feature cards`, await section.locator('article').count() === 4);
          const history = tab.locator('#release-history');
          assert(`${prefix}: Alpha 14 history heading`, (await history.locator('h2').textContent()).includes('Alpha 14'));
          for (const asset of assets) {
            const image = history.locator(`img[src$="/${asset}"]`);
            await image.scrollIntoViewIfNeeded();
            await tab.waitForFunction(node => node.complete && node.naturalWidth > 0, await image.elementHandle());
            const data = await image.evaluate(node => ({
              alt: node.alt, width: node.naturalWidth, height: node.naturalHeight,
              declaredWidth: Number(node.getAttribute('width')), declaredHeight: Number(node.getAttribute('height')),
              link: node.parentElement.getAttribute('href')
            }));
            assert(`${prefix}: ${asset} decodes with honest dimensions and enlarge link`,
              !!data.alt && data.width === data.declaredWidth && data.height === data.declaredHeight && data.link.endsWith('/' + asset));
          }
          assert(`${prefix}: release notes link`, await section.locator('a[href="https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.17"]').count() === 1);
          assert(`${prefix}: history release links`, await history.locator('a[href*="/releases/tag/"]').count() === 3);
          const status = await tab.locator('#status').textContent();
          assert(`${prefix}: schema and downgrade boundaries`, status.includes('16') && status.includes('28') && status.includes('Alpha 16') && status.includes('Alpha 15'));
          const watch = await tab.locator('#watch').textContent();
          assert(`${prefix}: videos are labelled as Alpha 14 recordings`, (watch.match(/Alpha 14/g) || []).length >= 3);
          assert(`${prefix}: no negative disclaimer tags`, !/no live bus|no hardware|kein live-bus|kein hardwarekontakt/i.test(await tab.locator('main').textContent()));
          assert(`${prefix}: no horizontal page overflow`, await tab.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
          assert(`${prefix}: Docker starts from ready-made image`, (await tab.locator('.docker-path ol').textContent()).includes(lang === 'de' ? 'fertige Image' : 'ready-made image'));
          if (javaScriptEnabled && [390, 1440].includes(width)) {
            await section.locator('article').first().scrollIntoViewIfNeeded();
            await tab.screenshot({ path: `website/output/playwright/alpha17-${lang}-${width}-top.png` });
            await section.locator('article').last().scrollIntoViewIfNeeded();
            await tab.screenshot({ path: `website/output/playwright/alpha17-${lang}-${width}-bottom.png` });
          }
        }
      }
    } finally { await context.close(); }
  }
  assert('no third-party requests', unexpected.length === 0);
  assert('no page or console errors', errors.length === 0);
  return { count: checks.length, checks, unexpected, errors, scope: 'Built static site in Chromium, DE/EN, 320–1440px, JavaScript on/off. No hardware, native GUI or universal compatibility acceptance.' };
}
