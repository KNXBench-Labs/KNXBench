/** Real-browser acceptance of the built Alpha 12 marketing section. */
async (page) => {
  const origin = new URL(page.url()).origin;
  const checks = [], errors = [], unexpected = [];
  const assert = (name, condition) => {
    if (!condition) throw new Error(name);
    checks.push(name);
  };
  const assets = ['alpha12-devices.webp', 'alpha12-addresses.webp', 'alpha12-instructions.webp', 'alpha12-support.webp'];
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
          assert(`${prefix}: Alpha 12 heading`, (await section.locator('h2').textContent()).includes('Alpha 12'));
          assert(`${prefix}: four feature cards`, await section.locator('article').count() === 4);
          for (const asset of assets) {
            const image = section.locator(`img[src$="/${asset}"]`);
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
          assert(`${prefix}: release notes link`, await section.locator('a[href="https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.12"]').count() === 1);
          const status = await tab.locator('#status').textContent();
          assert(`${prefix}: schema and downgrade boundaries`, status.includes('16') && status.includes('23') && status.includes('Alpha 11'));
          assert(`${prefix}: earlier videos remain separately labelled`, (await tab.locator('#watch').textContent()).includes(lang === 'de' ? 'Frühere Aufnahme' : 'Earlier recording'));
          assert(`${prefix}: no horizontal page overflow`, await tab.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
          assert(`${prefix}: Docker starts from ready-made image`, (await tab.locator('.docker-path ol').textContent()).includes(lang === 'de' ? 'fertige Image' : 'ready-made image'));
          if (javaScriptEnabled && [390, 1440].includes(width)) {
            await section.locator('article').first().scrollIntoViewIfNeeded();
            await tab.screenshot({ path: `website/output/playwright/alpha12-${lang}-${width}-top.png` });
            await section.locator('article').last().scrollIntoViewIfNeeded();
            await tab.screenshot({ path: `website/output/playwright/alpha12-${lang}-${width}-bottom.png` });
          }
        }
      }
    } finally { await context.close(); }
  }
  assert('no third-party requests', unexpected.length === 0);
  assert('no page or console errors', errors.length === 0);
  return { count: checks.length, checks, unexpected, errors, scope: 'Built static site in Chromium, DE/EN, 320–1440px, JavaScript on/off. No hardware, native GUI or universal compatibility acceptance.' };
}
