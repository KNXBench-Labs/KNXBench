/** Real-browser contracts for the fictional Phosphor Atlas marketing hero. */
async (page) => {
  const origin = new URL(page.url()).origin;
  const checks = [], errors = [], unexpected = [];
  const assert = (name, condition) => { if (!condition) throw new Error(name); checks.push(name); };
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.route('**/*', route => {
    if (new URL(route.request().url()).origin === origin) return route.continue();
    unexpected.push(route.request().url()); return route.abort();
  });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.goto(`${origin}/en/`);
  await page.locator('.atlas-diagram').scrollIntoViewIfNeeded();
  await page.waitForFunction(() => document.getAnimations().some(a => a.id === 'phosphor-path-draw'), null, { timeout: 4000 });
  assert('actual path reveal runs', await page.evaluate(() => document.getAnimations().some(a => a.id === 'phosphor-path-draw')));
  const toggle = page.locator('[data-motion-toggle]');
  // DOM activation avoids scrolling the diagram away before cancellation.
  await toggle.evaluate(button => button.click());
  assert('off cancels every active effect immediately', await page.evaluate(() => document.getAnimations().length === 0));
  assert('off leaves visible nodes and complete connections', await page.locator('[data-atlas-node]').evaluateAll(nodes => nodes.every(n => getComputedStyle(n).opacity === '1')) && await page.locator('.atlas-wide path').evaluateAll(paths => paths.every(p => getComputedStyle(p).strokeDashoffset === '0px')));
  assert('off state is explicit', await toggle.getAttribute('aria-pressed') === 'true');
  await toggle.evaluate(button => button.click());
  await page.waitForTimeout(2600);
  assert('diagram reveal never replays after resume', await page.evaluate(() => !document.getAnimations().some(a => a.id === 'phosphor-path-draw' || a.id === 'phosphor-node-appear')));
  for (const lang of ['en', 'de']) {
    for (const width of [320, 390, 768, 1440]) {
      await page.emulateMedia({ reducedMotion: 'reduce' });
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`${origin}/${lang}/`);
      await page.evaluate(() => document.fonts.ready);
      assert(`${lang}/${width} dark-green canvas`, await page.locator('body').evaluate(el => getComputedStyle(el).backgroundColor === 'rgb(3, 21, 12)'));
      assert(`${lang}/${width} no page overflow`, await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      assert(`${lang}/${width} every node is readable and bounded`, await page.locator('[data-atlas-node]').evaluateAll(nodes => nodes.every(n => { const r = n.getBoundingClientRect(); return r.width >= 140 && r.height >= 44 && r.left >= 0 && r.right <= innerWidth; })));
      assert(`${lang}/${width} logical dash gaps stay visibly distinct`, await page.locator('.atlas-paths').evaluateAll(svgs => svgs.filter(svg => svg.getBoundingClientRect().width > 0).every(svg => [...svg.querySelectorAll('[data-relation="logical"] path')].every(path => {
        const gap = Number.parseFloat(getComputedStyle(path).strokeDasharray.split(/[, ]+/)[1]);
        return path.getTotalLength() * gap >= 2.5;
      }))));
      const node = page.locator('[data-atlas-node="group"]');
      await node.focus(); await page.keyboard.press('Enter');
      assert(`${lang}/${width} keyboard selects a node`, await node.getAttribute('aria-current') === 'true');
      assert(`${lang}/${width} selected explanation opens`, await page.locator('#atlas-note-group').evaluate(el => el.open));
      assert(`${lang}/${width} node declares its expanded target`, await node.getAttribute('aria-controls') === 'atlas-note-group' && await node.getAttribute('aria-expanded') === 'true');
      assert(`${lang}/${width} logical links highlight`, await page.locator('[data-relation="logical"] .is-related').count() === 6);
      await page.locator('[data-atlas-node="switch"]').click();
      assert(`${lang}/${width} pointer selection updates explanation`, await page.locator('#atlas-note-switch').evaluate(el => el.open) && !await page.locator('#atlas-note-group').evaluate(el => el.open));
      await page.locator('#atlas-note-switch summary').click();
      await page.waitForFunction(() => document.querySelector('[data-atlas-node="switch"]').getAttribute('aria-expanded') === 'false');
      assert(`${lang}/${width} native closure updates expanded state`, await page.locator('[data-atlas-node="switch"]').getAttribute('aria-expanded') === 'false');
      assert(`${lang}/${width} OS reduction suppresses all effects`, await page.evaluate(() => document.getAnimations().length === 0));
      await page.locator('.hero-intro').scrollIntoViewIfNeeded();
      await page.screenshot({ path: `website/output/playwright/phosphor-${lang}-${width}.png` });
    }
  }
  // A new document starts a new one-shot sequence; reduce mid-path.
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.goto(`${origin}/en/`);
  await page.locator('.atlas-diagram').scrollIntoViewIfNeeded();
  await page.waitForFunction(() => document.getAnimations().some(a => a.id === 'phosphor-path-draw'));
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => document.getAnimations().length === 0);
  assert('live OS change settles the graph', await page.evaluate(() => document.getAnimations().length === 0));
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.goto(`${origin}/en/`);
  await page.locator('.atlas-diagram').scrollIntoViewIfNeeded();
  await page.waitForFunction(() => document.getAnimations().some(a => a.id === 'phosphor-path-draw'));
  await page.setViewportSize({ width: 390, height: 901 });
  await page.waitForFunction(() => !document.getAnimations().some(a => a.id === 'phosphor-path-draw' || a.id === 'phosphor-node-appear'), null, { timeout: 1000 });
  assert('responsive resize cancels graph effects', await page.evaluate(() => !document.getAnimations().some(a => a.id === 'phosphor-path-draw' || a.id === 'phosphor-node-appear')));
  await page.goto(`${origin}/en/`);
  const cta = page.locator('.hero-actions .button');
  await page.mouse.move(0, 0);
  await cta.hover();
  assert('actual hover transition starts when motion is enabled', await cta.evaluate(el => el.getAnimations().length > 0));
  await page.locator('[data-motion-toggle]').evaluate(button => button.click());
  assert('off cancels active CSS hover transitions too', await cta.evaluate(el => el.getAnimations().length === 0));
  const touch = await page.context().browser().newContext({ hasTouch: true, isMobile: true, viewport: { width: 320, height: 900 }, reducedMotion: 'reduce' });
  try {
    const mobile = await touch.newPage(); await mobile.goto(`${origin}/de/`);
    await mobile.locator('[data-atlas-node="actuator"]').tap();
    assert('touch tap selects a node and explanation', await mobile.locator('#atlas-note-actuator').evaluate(el => el.open));
  } finally { await touch.close(); }
  const nojs = await page.context().browser().newContext({ javaScriptEnabled: false, viewport: { width: 320, height: 900 } });
  try {
    const basic = await nojs.newPage();
    await basic.goto(`${origin}/de/`);
    assert('no-JS all five node links exist', await basic.locator('[data-atlas-node]').count() === 5);
    await basic.locator('[data-atlas-node="group"]').click();
    assert('no-JS native fragment reaches its explanation', new URL(basic.url()).hash === '#atlas-note-group');
    await basic.locator('#atlas-note-group summary').click();
    assert('no-JS explanation remains usable', await basic.locator('#atlas-note-group').evaluate(el => el.open));
    assert('no-JS diagram is fully visible', await basic.locator('[data-atlas-node]').evaluateAll(nodes => nodes.every(n => getComputedStyle(n).opacity === '1')));
    assert('no-JS motion control is hidden', await basic.locator('[data-motion-toggle]').isHidden());
  } finally { await nojs.close(); }
  const fallback = await page.context().browser().newContext({ viewport: { width: 320, height: 900 } });
  try {
    await fallback.addInitScript(() => { delete Element.prototype.animate; delete window.IntersectionObserver; });
    const basic = await fallback.newPage(); await basic.goto(`${origin}/en/`);
    await basic.locator('[data-atlas-node="sensor"]').click();
    assert('missing animation APIs retain diagram selection', await basic.locator('#atlas-note-sensor').evaluate(el => el.open));
    assert('missing APIs never hide artwork', await basic.locator('[data-atlas-node]').evaluateAll(nodes => nodes.every(n => getComputedStyle(n).opacity === '1')));
  } finally { await fallback.close(); }
  assert('no external requests', unexpected.length === 0);
  assert('no browser errors', errors.length === 0);
  return { count: checks.length, checks, errors, unexpected, scope: 'Chromium marketing preview, not live KNX or screen-reader certification' };
}
