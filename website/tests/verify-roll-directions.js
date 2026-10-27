/** Deterministic RNG inputs exercise real four-direction glyph rolls and cancellation. */
async (page) => {
  const origin = new URL(page.url()).origin, checks = [], directions = [];
  const assert = (name,value) => { if (!value) throw new Error(name); checks.push(name); };
  for (const [name,random,outgoing,incoming] of [
    ['right',.05,'translateX(105%)','translateX(-105%)'],
    ['left',.30,'translateX(-105%)','translateX(105%)'],
    ['up',.55,'translateY(-105%)','translateY(105%)'],
    ['down',.80,'translateY(105%)','translateY(-105%)']
  ]) {
    const context = await page.context().browser().newContext({viewport:{width:1440,height:1000},reducedMotion:'no-preference'});
    try {
      await context.addInitScript(value => { Math.random = () => value; },random);
      await context.route('**/*',route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
      const sample = await context.newPage();
      await sample.goto(`${origin}/en/`); await sample.evaluate(() => document.fonts.ready);
      await sample.locator('#features-title').scrollIntoViewIfNeeded(); // rolls begin once its typing has settled
      const before = await sample.locator('#features-title').boundingBox();
      await sample.waitForFunction(() => document.getAnimations().some(a => a.id === 'ambient-char-swap'),null,{timeout:15000,polling:20});
      const proof = await sample.evaluate(() => document.getAnimations().filter(a => a.id === 'ambient-char-swap').map(a => ({duration:a.effect.getTiming().duration,pseudo:a.effect.pseudoElement || '',frames:a.effect.getKeyframes().map(f => f.transform || '')})));
      assert(`${name}: actual outgoing direction`,proof.some(a => !a.pseudo && a.frames.includes(outgoing)));
      assert(`${name}: actual opposite incoming twin`,proof.some(a => a.pseudo === '::after' && a.frames.includes(incoming)));
      assert(`${name}: retained 720ms duration`,proof.length > 0 && proof.every(a => a.duration === 720));
      assert(`${name}: stable heading bounds`,JSON.stringify(before) === JSON.stringify(await sample.locator('#features-title').boundingBox()));
      await sample.evaluate(async () => {
        const animations = document.getAnimations().filter(a => a.id === 'ambient-char-swap');
        animations.forEach(a => a.pause()); await Promise.all(animations.map(a => a.ready));
        animations.forEach(a => { a.currentTime = a.effect.getTiming().delay + 360; });
        await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      });
      assert(`${name}: sampled glyph genuinely displaced`,await sample.evaluate(() => document.getAnimations().some(a => a.id === 'ambient-char-swap' && a.effect.target.classList.contains('char-glyph') && !['none','matrix(1, 0, 0, 1, 0, 0)'].includes(getComputedStyle(a.effect.target).transform))));
      await sample.screenshot({path:`website/output/playwright/roll-${name}.png`});
      await sample.locator('[data-motion-toggle]').focus(); await sample.keyboard.press('Enter');
      assert(`${name}: keyboard off cancels active roll`,await sample.evaluate(() => document.getAnimations().length === 0));
      await sample.keyboard.press('Enter');
      await sample.locator('#features-title').scrollIntoViewIfNeeded(); // focusing the toggle scrolled to the header
      await sample.waitForFunction(() => document.getAnimations().some(a => a.id === 'ambient-char-swap'));
      await sample.emulateMedia({reducedMotion:'reduce'});
      await sample.waitForFunction(() => document.getAnimations().length === 0);
      assert(`${name}: live reduction cancels roll`,await sample.evaluate(() => document.getAnimations().length === 0));
      directions.push({name,random,proof});
    } finally { await context.close(); }
  }
  return {count:checks.length,checks,directions,scope:'Real Chromium animations with controlled RNG branches, not natural-distribution evidence'};
}
