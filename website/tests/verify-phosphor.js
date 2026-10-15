/** Original screenshot-first layout, green Story headings and static fallbacks. */
async (page) => {
  const origin = new URL(page.url()).origin, checks = [], errors = [], unexpected = [];
  const assert = (name,value) => { if (!value) throw new Error(name); checks.push(name); };
  page.on('pageerror',error => errors.push(error.message));
  page.on('console',message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.route('**/*',route => {
    if (new URL(route.request().url()).origin === origin) return route.continue();
    unexpected.push(route.request().url()); return route.abort();
  });
  for (const lang of ['en','de']) {
    for (const width of [320,390,768,1440]) {
      await page.setViewportSize({width,height:1000});
      await page.emulateMedia({reducedMotion:'reduce'});
      await page.goto(`${origin}/${lang}/`); await page.evaluate(() => document.fonts.ready);
      assert(`${lang}/${width} headings are Story phosphor green`,await page.locator('h1,h2,h3').evaluateAll(headings => headings.length > 0 && headings.every(h => getComputedStyle(h).color === 'rgb(136, 255, 173)')));
      assert(`${lang}/${width} installation diagram absent`,await page.locator('#installation-atlas,[data-atlas-node]').count() === 0);
      assert(`${lang}/${width} original screenshot-first hierarchy`,await page.locator('.hero').evaluate(el => [...el.children].map(c => c.className).join('|') === 'hero-intro|hero-media|benefit-strip'));
      assert(`${lang}/${width} green-black canvas`,await page.locator('body').evaluate(el => getComputedStyle(el).backgroundColor === 'rgb(3, 21, 12)'));
      assert(`${lang}/${width} no page overflow`,await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      assert(`${lang}/${width} reduction suppresses effects`,await page.evaluate(() => document.getAnimations().length === 0));
      assert(`${lang}/${width} prose has no glow or blur`,await page.locator('.lede').evaluate(el => getComputedStyle(el).textShadow === 'none' && getComputedStyle(el).filter === 'none'));
      for (const image of await page.locator('img').all()) {
        if (await image.isVisible()) await image.scrollIntoViewIfNeeded();
        await image.evaluate(el => el.decode());
      }
      assert(`${lang}/${width} media decoded before capture`,await page.locator('img').evaluateAll(images => images.every(image => image.complete && image.naturalWidth > 0)));
      await page.evaluate(() => scrollTo(0,0));
      await page.screenshot({path:`website/output/playwright/crt-${lang}-${width}.png`});
      if ([320,1440].includes(width)) await page.screenshot({path:`website/output/playwright/complete-crt-${lang}-${width}.png`,fullPage:true});
    }
    for (const kind of ['contact','privacy']) {
      await page.goto(`${origin}/${lang}/${kind}/`);
      assert(`${lang}/${kind} green static heading`,await page.locator('h1').evaluate(h => getComputedStyle(h).color === 'rgb(136, 255, 173)') && await page.locator('.char').count() === 0);
    }
  }
  await page.setViewportSize({width:1440,height:1000}); await page.emulateMedia({reducedMotion:'no-preference'});
  await page.goto(`${origin}/en/`); await page.mouse.move(0,0);
  const cta = page.locator('.hero-actions .button'); await cta.hover();
  assert('permitted hover transition actually starts',await cta.evaluate(el => el.getAnimations().length > 0));
  await page.locator('[data-motion-toggle]').evaluate(el => el.click());
  assert('Motion off cancels active CSS transitions too',await cta.evaluate(el => el.getAnimations().length === 0));
  const nojs = await page.context().browser().newContext({javaScriptEnabled:false,viewport:{width:320,height:900}});
  try {
    const basic = await nojs.newPage(); await basic.goto(`${origin}/de/`);
    assert('no-JS original screenshot remains',await basic.locator('.hero-media img').count() === 1);
    assert('no-JS heading and motion fallback intact',(await basic.locator('h1').textContent()).includes('Dein KNX-Projekt.') && await basic.locator('[data-motion-toggle]').isHidden());
  } finally { await nojs.close(); }
  assert('no external requests',unexpected.length === 0); assert('no browser errors',errors.length === 0);
  return {count:checks.length,checks,errors,unexpected,scope:'Actual Chromium original-layout CRT preview/public-build variant; no Story or application changes'};
}
