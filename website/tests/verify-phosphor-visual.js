/** Raster and finite-effect evidence for the Phosphor Atlas private preview. */
async (page) => {
  const origin = new URL(page.url()).origin;
  const checks = [];
  const assert = (name, value) => { if (!value) throw new Error(name); checks.push(name); };
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.goto(`${origin}/en/`);
  await page.evaluate(() => document.fonts.ready);
  await page.locator('.atlas-diagram').scrollIntoViewIfNeeded();
  await page.waitForFunction(() => document.getAnimations().some(a => a.id === 'phosphor-path-draw'));
  await page.screenshot({ path: 'website/output/playwright/phosphor-draw-in-flight.png' });
  await page.waitForTimeout(2400);
  assert('finite diagram reveal settles without retained animation objects', await page.evaluate(() => !document.getAnimations().some(a => a.id === 'phosphor-path-draw' || a.id === 'phosphor-node-appear')));
  await page.locator('h1').scrollIntoViewIfNeeded();
  await page.waitForFunction(() => document.getAnimations().some(a => a.id === 'phosphor-char-glitch'), null, { timeout: 32000, polling: 20 });
  const glitch = await page.evaluate(() => document.getAnimations().filter(a => a.id === 'phosphor-char-glitch').map(a => ({ duration: a.effect.getTiming().duration, target: a.effect.target.closest('h1,h2,h3').tagName, frames: a.effect.getKeyframes() })));
  assert('rare glitch affects a single heading glyph for 180ms', glitch.length === 1 && glitch[0].duration === 180);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => document.getAnimations().length === 0);
  assert('reduction cancels the rare glyph glitch', await page.evaluate(() => document.getAnimations().length === 0));
  for (const lang of ['en', 'de']) {
    for (const width of [320, 390, 768, 1440]) {
      await page.setViewportSize({ width, height: 1000 });
      await page.goto(`${origin}/${lang}/`);
      await page.evaluate(() => document.fonts.ready);
      await page.locator('[data-atlas-node="group"]').click();
      assert(`${lang}/${width} unfocused skip link remains outside viewport`, await page.locator('.skip-link').evaluate(el => el !== document.activeElement && el.getBoundingClientRect().bottom <= 0));
      await page.locator('#installation-atlas').screenshot({ path: `website/output/playwright/atlas-${lang}-${width}.png` });
      await page.evaluate(() => scrollTo(0, 0));
      await page.screenshot({ path: `website/output/playwright/hero-${lang}-${width}.png` });
      assert(`${lang}/${width} node text fits its bounds`, await page.locator('[data-atlas-node]').evaluateAll(nodes => nodes.every(n => n.scrollWidth <= n.clientWidth)));
      if (width === 1440 || width === 320) {
        await page.screenshot({ path: `website/output/playwright/page-${lang}-${width}.png`, fullPage: true });
      }
    }
  }
  return { count: checks.length, checks, glitch, scope: 'Actual Chromium raster and WAAPI lifecycle evidence; no hardware claims' };
}
