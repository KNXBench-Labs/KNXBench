/** Browser recipe for the highlights section and its real-app stills. */
async (page) => {
  const origin = new URL(page.url()).origin;
  const checks = [], errors = [], unexpected = [];
  const assert = (name, condition) => {
    if (!condition) throw new Error(name);
    checks.push(name);
  };
  const assets = ['highlight-import.webp', 'highlight-history.webp', 'highlight-wizard.webp',
    'highlight-parameters.webp', 'highlight-catalogue.webp', 'highlight-search.webp'];
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
          const section = tab.locator('#features');
          await section.scrollIntoViewIfNeeded();
          assert(`${prefix}: nine highlights`, await section.locator('article').count() === 9);
          assert(`${prefix}: numbered 01 to 09`, (await section.locator('.step').allTextContents()).join(',') ===
            '01,02,03,04,05,06,07,08,09');
          for (const asset of assets) {
            const image = section.locator(`img[src$="/${asset}"]`);
            await image.scrollIntoViewIfNeeded();
            await tab.waitForFunction(node => node.complete && node.naturalWidth > 0, await image.elementHandle());
            const data = await image.evaluate(node => ({
              alt: node.alt, width: node.naturalWidth, height: node.naturalHeight,
              declaredWidth: Number(node.getAttribute('width')), declaredHeight: Number(node.getAttribute('height')),
              link: node.parentElement.getAttribute('href'), shown: node.getBoundingClientRect().width
            }));
            assert(`${prefix}: ${asset} decodes with honest dimensions and enlarge link`,
              !!data.alt && data.width === data.declaredWidth && data.height === data.declaredHeight &&
              data.link.endsWith('/' + asset) && data.shown > 0);
          }
          assert(`${prefix}: no release-specific sections`, await tab.locator('#release-features, #release-history').count() === 0);
          const main = await tab.locator('main').textContent();
          assert(`${prefix}: no release numbers in the page`, !/Alpha[ -]?\d|alpha\.\d|System-2|#30/.test(main));
          assert(`${prefix}: alpha notice stays visible`, await tab.locator('.hero .alpha-tag').isVisible() &&
            /alpha/i.test(await tab.locator('#status h2').textContent()));
          assert(`${prefix}: no negative disclaimer tags`, !/no live bus|no hardware|kein live-bus|kein hardwarekontakt/i.test(main));
          assert(`${prefix}: no horizontal page overflow`, await tab.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
          assert(`${prefix}: Docker starts from ready-made image`, (await tab.locator('.docker-path ol').textContent()).includes(lang === 'de' ? 'fertige Image' : 'ready-made image'));
          if (javaScriptEnabled && [390, 1440].includes(width)) {
            await section.locator('article').first().scrollIntoViewIfNeeded();
            await tab.screenshot({ path: `website/output/playwright/highlights-${lang}-${width}-top.png` });
            await section.locator('article').last().scrollIntoViewIfNeeded();
            await tab.screenshot({ path: `website/output/playwright/highlights-${lang}-${width}-bottom.png` });
          }
        }
      }
    } finally { await context.close(); }
  }
  assert('no third-party requests', unexpected.length === 0);
  assert('no page or console errors', errors.length === 0);
  return { count: checks.length, checks, unexpected, errors, scope: 'Built static site in Chromium, DE/EN, 320–1440px, JavaScript on/off. No native GUI or universal compatibility acceptance.' };
}
