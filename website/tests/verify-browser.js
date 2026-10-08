/** Plain Playwright CLI function: verify the actual static marketing artifact. */
async (page) => {
  const origin = 'http://127.0.0.1:4198';
  const checks = [], requests = [], unexpected = [], errors = [], media = [];
  const assert = (name, condition) => { if (!condition) throw new Error(name); checks.push(name); };
  page.on('request', request => {
    requests.push(new URL(request.url()).pathname);
    if (new URL(request.url()).origin !== origin) unexpected.push(request.url());
  });
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/de/`);
  await page.locator('.hero-media img').waitFor();
  await page.evaluate(() => document.fonts.ready);
  assert('German hero renders actual marketing copy', (await page.locator('h1').textContent()).includes('Dein KNX-Projekt.'));
  assert('private preview and alpha are visible', await page.locator('.preview-note').isVisible() && await page.locator('.hero .alpha-tag').isVisible());
  assert('no video request before deliberate playback', !requests.some(path => path.endsWith('.mp4')));
  assert('both locally hosted font families load', await page.evaluate(() => document.fonts.check('600 20px "Bench Display"') && document.fonts.check('400 20px "Bench Text"')));
  await page.screenshot({ path: 'website/output/playwright/desktop-de.png' });
  await page.screenshot({ path: 'website/output/playwright/full-de.png', fullPage: true });
  await page.keyboard.press('Tab');
  assert('keyboard skip link receives first focus', await page.locator('.skip-link').evaluate(el => el === document.activeElement));
  await page.keyboard.press('Enter');
  assert('skip link reaches the main landmark', await page.locator('main').evaluate(el => el === document.activeElement));
  await page.locator('.hero-actions a').first().click();
  assert('primary CTA reaches the real-product installation section', new URL(page.url()).hash === '#start');
  assert('Docker is the prominent first path and AppImage is separate', await page.locator('.docker-path').isVisible() && (await page.locator('.install-paths article').first().textContent()).includes('Docker'));
  assert('public installation targets and the site preview are disclosed', (await page.locator('.launch-note').first().textContent()).includes('öffentlich auf GitHub') && (await page.locator('.launch-note').first().textContent()).includes('Vorschau'));
  const links = await page.locator('a[href]').evaluateAll(elements => [...new Set(elements.map(el => el.href))]);
  for (const href of links) {
    const url = new URL(href);
    if (url.origin !== origin || (url.pathname === '/de/' && url.hash)) continue;
    const response = await page.request.get(url.href);
    assert(`local link answers 200: ${url.pathname}`, response.status() === 200);
  }
  for (const image of await page.locator('img').all()) {
    await image.scrollIntoViewIfNeeded();
    await page.waitForFunction(el => el.complete && el.naturalWidth > 0, await image.elementHandle());
  }
  assert('all product/logo/gallery images decode', await page.locator('img').evaluateAll(images => images.every(el => el.complete && el.naturalWidth > 0)));
  const videos = page.locator('video');
  assert('three real-app demo videos with no autoplay or loop', await videos.count() === 3 && await videos.evaluateAll(elements => elements.every(v => v.paused && !v.autoplay && !v.loop && v.controls)));
  for (let i = 0; i < await videos.count(); i++) {
    const video = videos.nth(i);
    await video.scrollIntoViewIfNeeded();
    await video.evaluate(v => v.play());
    await page.waitForFunction(index => { const v = document.querySelectorAll('video')[index]; return v.currentTime > .3 && v.videoWidth > 0 && v.getVideoPlaybackQuality().totalVideoFrames > 0; }, i);
    const result = await video.evaluate(v => ({ name: v.getAttribute('aria-label'), width: v.videoWidth, height: v.videoHeight, duration: v.duration, time: v.currentTime, decodedFrames: v.getVideoPlaybackQuality().totalVideoFrames, paused: v.paused }));
    media.push(result);
    assert(`video ${i + 1} actually decodes frames`, result.decodedFrames > 0 && result.time > .3 && result.duration > 1);
    assert(`video ${i + 1} pauses other players`, await videos.evaluateAll((elements, active) => elements.every((v, index) => index === active || v.paused), i));
    await page.waitForFunction(index => { const track = document.querySelectorAll('video')[index].textTracks[0]; return track && track.cues && track.cues.length > 0; }, i);
    assert(`video ${i + 1} descriptive captions load`, await video.evaluate(v => v.textTracks[0].cues.length > 0));
  }
  await videos.last().evaluate(v => v.pause());
  assert('motion reduction produces no active page animations', await page.evaluate(() => document.getAnimations().length === 0));
  assert('no cookies are set by the static site', (await page.context().cookies()).length === 0);
  await page.locator('.language-nav a[lang="en"]').click();
  assert('English language URL and content are complete', new URL(page.url()).pathname === '/en/' && await page.locator('html').getAttribute('lang') === 'en' && (await page.locator('h1').textContent()).includes('Your KNX project.'));
  await page.screenshot({ path: 'website/output/playwright/desktop-en.png' });
  for (const lang of ['de', 'en']) {
    for (const width of [320, 390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`${origin}/${lang}/`);
      assert(`${lang} ${width}px has no horizontal overflow`, await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      assert(`${lang} ${width}px primary CTA remains visible`, await page.locator('.hero-actions a').first().isVisible());
    }
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`${origin}/de/`);
  const summary = page.locator('.mobile-menu > summary');
  await summary.focus(); await page.keyboard.press('Enter');
  assert('mobile navigation opens with keyboard', await page.locator('.mobile-menu').evaluate(el => el.open));
  await page.keyboard.press('Escape');
  assert('mobile Escape closes and restores focus', await page.locator('.mobile-menu').evaluate(el => !el.open && el.querySelector('summary') === document.activeElement));
  await summary.click(); await page.locator('.mobile-menu a[href="#start"]').click();
  assert('mobile internal navigation closes the disclosure', !await page.locator('.mobile-menu').evaluate(el => el.open) && new URL(page.url()).hash === '#start');
  await page.goto(`${origin}/de/`);
  await page.screenshot({ path: 'website/output/playwright/mobile-de.png', fullPage: true });
  await page.locator('.site-footer a[href="../story/"]').click();
  assert('Evolution link opens the existing story companion', new URL(page.url()).pathname === '/story/' && await page.locator('.chapter-section').count() > 0 && await page.locator('.chapter-section').count() === await page.evaluate(() => JSON.parse(document.getElementById('story-data').textContent).chapters.length));
  assert('story still displays its private-preview notice', await page.locator('.preview-banner').isVisible());
  const nojs = await page.context().browser().newContext({ javaScriptEnabled: false, viewport: { width: 390, height: 844 } });
  try {
    const basic = await nojs.newPage();
    await basic.goto(`${origin}/en/`);
    assert('no-JavaScript English hero remains readable', (await basic.locator('h1').textContent()).includes('Your KNX project.'));
    await basic.locator('.hero-actions a').first().click();
    assert('no-JavaScript primary CTA still works', new URL(basic.url()).hash === '#start');
    await basic.locator('#status summary').click();
    assert('no-JavaScript compatibility disclosure still works', await basic.locator('#status details').evaluate(el => el.open));
    await basic.screenshot({ path: 'website/output/playwright/nojs-mobile-en.png', fullPage: true });
  } finally { await nojs.close(); }
  assert('no third-party request at any tested stage', unexpected.length === 0);
  assert('no page or console errors', errors.length === 0);
  return { checks, count: checks.length, media, unexpected, errors, requestPaths: [...new Set(requests)].sort(), externalLinks: 'not followed by this recipe; checked signed out separately', scope: 'Chromium; static preview; native screen readers and public hosting not verified' };
}
