/** Playwright CLI acceptance for Story-style ambient heading characters. */
async (page) => {
  const origin = new URL(page.url()).origin;
  const checks = [], errors = [], unexpected = [];
  const assert = (name, condition) => { if (!condition) throw new Error(name); checks.push(name); };
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.route('**/*', route => {
    if (new URL(route.request().url()).origin === origin) return route.continue();
    unexpected.push(route.request().url());
    return route.abort();
  });
  const active = () => page.evaluate(() => document.getAnimations().filter(a => a.id === 'ambient-char-swap' && a.playState === 'running').length);
  const waitForRoll = () => page.waitForFunction(() => document.getAnimations().some(a => a.id === 'ambient-char-swap' && a.playState === 'running'), null, { timeout: 7000, polling: 30 });
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto(`${origin}/de/`);
  assert('marketing headings are split into Story-style character boxes', await page.locator('main h1 .char-glyph').count() > 0);
  assert('hero keeps its explicit line break', await page.locator('h1 br').count() === 1);
  assert('hero accessible name retains the word boundary across its line break', await page.locator('h1').getAttribute('aria-label') === 'Dein KNX-Projekt. Deine Werkbank.');
  assert('decorative words are hidden from assistive technology', await page.locator('main .char-word').evaluateAll(words => words.every(word => word.getAttribute('aria-hidden') === 'true')));
  const before = await page.locator('h1').boundingBox();
  await waitForRoll();
  const proof = await page.evaluate(() => {
    const animations = document.getAnimations().filter(a => a.id === 'ambient-char-swap' && a.playState === 'running');
    return animations.map(a => ({ duration: a.effect.getTiming().duration, easing: a.effect.getTiming().easing, target: a.effect.target.closest('h1,h2,h3').tagName, pseudo: a.effect.pseudoElement || '', frames: a.effect.getKeyframes().map(f => ({ transform: f.transform, opacity: f.opacity })) }));
  });
  assert('actual Story swaps use 720ms easing', proof.length > 0 && proof.every(a => a.duration === 720 && a.easing === 'cubic-bezier(0.65, 0, 0.35, 1)'));
  assert('actual glyph slides out and its pseudo-element twin slides in', proof.some(a => a.frames.some(f => f.transform === 'translateX(105%)')) && proof.some(a => a.pseudo === '::after'));
  assert('ambient rolls preserve the heading box', JSON.stringify(await page.locator('h1').boundingBox()) === JSON.stringify(before));
  await waitForRoll();
  await page.evaluate(async () => {
    const animations = document.getAnimations().filter(a => a.id === 'ambient-char-swap' && a.playState === 'running');
    for (const animation of animations) animation.pause();
    await Promise.all(animations.map(a => a.ready));
    for (const animation of animations) animation.currentTime = animation.effect.getTiming().delay + 360;
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  });
  const snapshot = await page.evaluate(() => document.getAnimations().filter(a => a.id === 'ambient-char-swap').map(a => ({ state: a.playState, currentTime: a.currentTime, transform: getComputedStyle(a.effect.target, a.effect.pseudoElement || null).transform })));
  assert('in-flight snapshot contains a paused displaced glyph', snapshot.some(a => a.state === 'paused' && a.transform !== 'none' && a.transform !== 'matrix(1, 0, 0, 1, 0, 0)'));
  await page.screenshot({ path: 'website/output/playwright/headline-motion-de.png' });
  await page.evaluate(() => document.getAnimations().filter(a => a.id === 'ambient-char-swap').forEach(a => a.play()));
  assert('sampled glyph resumes before live cancellation check', await active() > 0);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => !document.getAnimations().some(a => a.id === 'ambient-char-swap'));
  assert('live OS reduced motion cancels an in-flight swap', await active() === 0);
  await page.waitForTimeout(4200);
  assert('OS reduction prevents subsequent swaps', await active() === 0);
  const toggle = page.locator('[data-motion-toggle]');
  assert('manual pause control remains reachable under OS reduction', await toggle.isVisible());
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.locator('h1').scrollIntoViewIfNeeded();
  await waitForRoll();
  assert('clearing OS reduction resumes ambient rolls', await active() > 0);
  await toggle.focus();
  await page.keyboard.press('Enter');
  assert('keyboard pause sets the accessible pressed state', await toggle.getAttribute('aria-pressed') === 'true');
  assert('manual pause cancels running swaps', await active() === 0);
  await page.waitForTimeout(4200);
  assert('manual pause prevents new swaps', await active() === 0);
  await page.keyboard.press('Enter');
  assert('keyboard resume clears the pressed state', await toggle.getAttribute('aria-pressed') === 'false');
  await page.locator('h1').scrollIntoViewIfNeeded();
  await waitForRoll();
  assert('manual resume restarts ambient rolls', await active() > 0);
  // Explicitly synthetic visibility state: exercise cleanup, never claim a real tab suspension.
  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => true });
    document.dispatchEvent(new Event('visibilitychange'));
  });
  assert('hidden-document lifecycle cancels ambient rolls', await active() === 0);
  await page.waitForTimeout(4200);
  assert('hidden-document lifecycle suppresses later rolls', await active() === 0);
  await page.evaluate(() => { delete document.hidden; document.dispatchEvent(new Event('visibilitychange')); });
  await waitForRoll();
  assert('visible-document lifecycle resumes ambient rolls', await active() > 0);
  await page.locator('#start').scrollIntoViewIfNeeded();
  await page.waitForFunction(() => !document.querySelector('h1').getAnimations({ subtree: true }).some(a => a.id === 'ambient-char-swap'));
  assert('offscreen hero stops animating', await page.locator('h1').evaluate(h => h.getAnimations({ subtree: true }).length === 0));
  for (const lang of ['de', 'en']) {
    for (const width of [320, 390, 768, 1440]) {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`${origin}/${lang}/`);
      assert(`${lang} ${width}px every marketing heading is enhanced`, await page.locator('main h1, main h2, main h3').evaluateAll(headings => headings.every(h => h.querySelector('.char-glyph') && h.getAttribute('aria-label'))));
      assert(`${lang} ${width}px does not overflow after splitting`, await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    }
    await page.goto(`${origin}/${lang}/contact/`);
    assert(`${lang} legal notice stays static`, await page.locator('.char').count() === 0 && await active() === 0);
    await page.goto(`${origin}/${lang}/privacy/`);
    assert(`${lang} privacy notice stays static`, await page.locator('.char').count() === 0 && await active() === 0);
  }
  const nojs = await page.context().browser().newContext({ javaScriptEnabled: false });
  try {
    const basic = await nojs.newPage();
    await basic.goto(`${origin}/en/`);
    assert('without JavaScript the original heading remains readable', (await basic.locator('h1').innerText()) === 'Your KNX project.\nYour workbench.');
    assert('without JavaScript the nonfunctional motion button stays hidden', await basic.locator('[data-motion-toggle]').isHidden());
  } finally { await nojs.close(); }
  const fallback = await page.context().browser().newContext();
  try {
    await fallback.addInitScript(() => { delete window.IntersectionObserver; delete Element.prototype.animate; });
    const basic = await fallback.newPage();
    await basic.goto(`${origin}/en/`);
    assert('missing motion APIs leave the original heading intact', await basic.locator('.char').count() === 0 && (await basic.locator('h1').innerText()).includes('Your KNX project.'));
    assert('missing motion APIs keep the pause button hidden', await basic.locator('[data-motion-toggle]').isHidden());
  } finally { await fallback.close(); }
  assert('no external requests', unexpected.length === 0);
  assert('no console or page errors', errors.length === 0);
  return { checks, count: checks.length, errors, unexpected, animationProof: proof, snapshot, scope: 'Chromium local preview; document visibility lifecycle exercised synthetically; no native screen-reader certification' };
}
