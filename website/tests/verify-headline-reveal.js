/** Real viewport entry/exit, interrupted reversal and readable motion fallbacks. */
async (page) => {
  const origin = new URL(page.url()).origin, checks = [], errors = [], unexpected = [];
  const assert = (name, value) => { if (!value) throw new Error(name); checks.push(name); };
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    if (new URL(route.request().url()).origin === origin) return route.continue();
    unexpected.push(route.request().url()); return route.abort();
  });
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto(`${origin}/en/`);
  const hero = page.locator('h1');
  const revealing = () => hero.evaluate(h => h.getAnimations({ subtree: true }).filter(a => a.id === 'headline-reveal'));
  await page.waitForFunction(() => document.querySelector('h1').getAnimations({ subtree: true }).some(a => a.id === 'headline-reveal'), null, { timeout: 3000 });
  assert('page-load hero has real character entrance animations', (await revealing()).length > 0);
  const proof = await hero.evaluate(h => h.getAnimations({ subtree: true }).filter(a => a.id === 'headline-reveal').map(a => ({ frames: a.effect.getKeyframes().map(f => ({ transform: f.transform, opacity: f.opacity, offset: f.offset })), duration: a.effect.getTiming().duration })));
  assert('entrance retains SessionRadar translation, rotation and fade', proof.every(a => a.frames.some(f => f.transform === 'translateY(0.3em) rotateX(-45deg)' && f.opacity === '0')));
  assert('characters arrive in a stagger', new Set(proof.map(a => a.frames[1].offset)).size > 1);
  await page.waitForFunction(() => !document.querySelector('h1').getAnimations({ subtree: true }).some(a => a.id === 'headline-reveal'));
  const bounds = await hero.boundingBox();
  assert('settled hero is completely readable', await hero.evaluate(h => [...h.querySelectorAll('.char')].every(c => getComputedStyle(c).opacity === '1')));
  // Put most of the hero just outside the top inset, then interrupt its exit.
  await hero.evaluate(h => window.scrollTo(0, h.getBoundingClientRect().bottom + scrollY - innerHeight * .05 - 1));
  await page.waitForFunction(() => document.querySelector('h1').getAnimations({ subtree: true }).some(a => a.id === 'headline-reveal' && a.playbackRate < 0), null, { timeout: 3000 });
  assert('leaving viewport plays the entrance timeline backwards', await hero.evaluate(h => h.getAnimations({ subtree: true }).filter(a => a.id === 'headline-reveal').every(a => a.playbackRate < 0)));
  // Freeze a real native timeline mid-flight, then let viewport re-entry reverse it.
  await hero.evaluate(async h => {
    const animations = h.getAnimations({ subtree: true }).filter(a => a.id === 'headline-reveal');
    animations.forEach(a => a.pause()); await Promise.all(animations.map(a => a.ready));
    animations.forEach(a => { a.currentTime = a.effect.getTiming().duration / 2; });
    window.__headlineExitAnimations = animations;
    window.scrollTo(0, 0);
  });
  await page.waitForFunction(() => window.__headlineExitAnimations.every(a => a.playbackRate > 0 && a.playState === 'running'));
  assert('rapid re-entry reverses the same animations, without replacing them', await hero.evaluate(h => window.__headlineExitAnimations.every(a => h.getAnimations({ subtree: true }).includes(a))));
  await page.screenshot({ path: 'website/output/playwright/headline-reveal-en.png' });
  await page.waitForFunction(() => !document.querySelector('h1').getAnimations({ subtree: true }).some(a => a.id === 'headline-reveal'));
  assert('entrance preserves layout bounds', JSON.stringify(bounds) === JSON.stringify(await hero.boundingBox()));
  await hero.evaluate(h => window.scrollTo(0, h.getBoundingClientRect().bottom + scrollY - innerHeight * .05 - 1));
  await page.waitForFunction(() => document.querySelector('h1').getAnimations({ subtree: true }).some(a => a.id === 'headline-reveal' && a.playbackRate < 0));
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => !document.getAnimations().length);
  assert('live reduced motion cancels exit and restores readable letters', await hero.evaluate(h => [...h.querySelectorAll('.char')].every(c => getComputedStyle(c).opacity === '1')));
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.locator('[data-motion-toggle]').click();
  assert('manual pause leaves every headline readable', await page.locator('main .char').evaluateAll(chars => chars.every(c => getComputedStyle(c).opacity === '1')));
  assert('manual pause cancels both animation layers', await page.evaluate(() => document.getAnimations().length === 0));
  await page.locator('[data-motion-toggle]').click();
  // Every marketing heading, not just the hero, repeats the entrance on re-entry.
  for (const lang of ['en', 'de']) {
    await page.goto(`${origin}/${lang}/`);
    const headings = page.locator('main h1, main h2, main h3');
    const count = await headings.count();
    for (let index = 0; index < count; index++) {
      const heading = headings.nth(index);
      await heading.evaluate(h => window.scrollTo(0, h.getBoundingClientRect().top + scrollY - innerHeight / 2));
      await page.waitForFunction(i => document.querySelectorAll('main h1, main h2, main h3')[i].dataset.headlineState === 'shown', index);
      assert(`${lang} heading ${index + 1} settles readable on entry`, await heading.evaluate(h => [...h.querySelectorAll('.char')].every(c => getComputedStyle(c).opacity === '1')));
    }
  }
  // The lower viewport edge must reverse too, not only the hero's upper exit.
  const section = page.locator('main h2').first();
  await section.evaluate(h => {
    window.__headlineBoundary = h;
    window.scrollTo(0, h.getBoundingClientRect().top + scrollY - innerHeight / 2);
  });
  await page.waitForFunction(() => window.__headlineBoundary.dataset.headlineState === 'shown' && !window.__headlineBoundary.getAnimations({ subtree: true }).some(a => a.id === 'headline-reveal'));
  await section.evaluate(h => window.scrollTo(0, h.getBoundingClientRect().top + scrollY - innerHeight * .95 + 1));
  await page.waitForFunction(() => window.__headlineBoundary.getAnimations({ subtree: true }).some(a => a.id === 'headline-reveal' && a.playbackRate < 0));
  assert('scrolling upwards reverses a heading at the lower viewport edge', true);
  await page.waitForFunction(() => window.__headlineBoundary.dataset.headlineState === 'hidden' && window.__headlineBoundary.getAnimations({ subtree: true }).length === 0);
  assert('completed exit leaves no retained or running animations', true);
  await section.evaluate(h => window.scrollTo(0, h.getBoundingClientRect().top + scrollY - innerHeight / 2));
  await page.waitForFunction(() => window.__headlineBoundary.getAnimations({ subtree: true }).some(a => a.id === 'headline-reveal' && a.playbackRate > 0));
  // Explicit synthetic pagehide/pageshow: lifecycle cleanup, not browser-cache proof.
  await page.evaluate(() => window.dispatchEvent(new Event('pagehide')));
  assert('pagehide cancels both motion layers and restores readable text', await page.evaluate(() => !document.getAnimations().length && [...document.querySelectorAll('main .char')].every(c => getComputedStyle(c).opacity === '1')));
  await page.evaluate(() => window.dispatchEvent(new Event('pageshow')));
  assert('pageshow restores the motion owner', await page.evaluate(() => document.documentElement.dataset.motion === 'on'));
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 320, height: 900 });
  await page.goto(`${origin}/de/`);
  assert('reduced-motion load has no hidden headings or animations', await page.locator('main .char').evaluateAll(chars => chars.every(c => getComputedStyle(c).opacity === '1')) && await page.evaluate(() => !document.getAnimations().length));
  assert('mobile heading layout has no horizontal overflow', await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
  const fallback = await page.context().browser().newContext({ reducedMotion: 'no-preference' });
  try {
    await fallback.addInitScript(() => { Element.prototype.animate = () => { throw new Error('animation refused'); }; });
    await fallback.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
    const basic = await fallback.newPage();
    await basic.goto(`${origin}/en/`);
    await basic.waitForFunction(() => document.documentElement.dataset.motion === 'off');
    assert('animation failure restores all text and hides the nonfunctional control', await basic.locator('main .char').evaluateAll(chars => chars.length > 0 && chars.every(c => getComputedStyle(c).opacity === '1')) && await basic.locator('[data-motion-toggle]').isHidden());
  } finally { await fallback.close(); }
  assert('no page errors or foreign requests', errors.length === 0 && unexpected.length === 0);
  return { count: checks.length, checks, errors, unexpected, proof, scope: 'Real Chromium viewport and native animation timelines; paused midpoint and page lifecycle explicitly instrumented' };
}
