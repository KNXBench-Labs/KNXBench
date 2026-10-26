/** Real Chromium acceptance for the selective live-typing headlines (WP-21). */
async (page) => {
  const origin = new URL(page.url()).origin, checks = [], errors = [], unexpected = [];
  const browser = page.context().browser();
  const assert = (name, value) => { if (!value) throw new Error(name); checks.push(name); };
  const TYPED = ['hero-title', 'features-title'];
  // Visible glyph count plus cursor/last-glyph geometry for one heading, in one synchronous read.
  const probe = (id, on = page) => on.evaluate((id) => {
    const h = document.getElementById(id);
    const shown = [...h.querySelectorAll('.char')].filter(c => getComputedStyle(c.firstChild).visibility === 'visible' && !c.hasAttribute('data-untyped'));
    const cursors = h.querySelectorAll('.typing-cursor');
    const last = shown[shown.length - 1], cursor = cursors[0];
    const box = el => { const r = el.getBoundingClientRect(); return { left: r.left, right: r.right, top: r.top, bottom: r.bottom }; };
    // Anything animating inside the heading except the cursor blink: reveal or ambient glyph swaps.
    const foreign = h.getAnimations({ subtree: true }).filter(a => a.id !== 'headline-typing-cursor').length;
    return { total: h.querySelectorAll('.char').length, shown: shown.length, cursors: cursors.length, foreign,
      state: h.dataset.headlineState, hiddenAria: cursor ? cursor.getAttribute('aria-hidden') : null,
      opacity: cursor ? getComputedStyle(cursor).opacity : null, at: performance.now(),
      last: last && box(last), cursor: cursor && box(cursor), line: cursor && Math.round(cursor.getBoundingClientRect().top) };
  }, id);
  const sample = async (on, id, until, limit = 12000) => {
    const samples = [], start = Date.now();
    while (Date.now() - start < limit) {
      const s = await probe(id, on); samples.push(s);
      if (until(s)) break;
      await on.waitForTimeout(25);
    }
    return samples;
  };
  const context = async (options) => {
    const c = await browser.newContext({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'no-preference', ...options });
    await c.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : (unexpected.push(route.request().url()), route.abort()));
    const p = await c.newPage();
    p.on('pageerror', error => errors.push(error.message));
    return [c, p];
  };
  const adjacent = s => s.cursor && s.last && Math.abs(s.cursor.left - s.last.right) <= 4 &&
    s.cursor.top < s.last.bottom && s.cursor.bottom > s.last.top;
  const finished = (s) => s.shown === s.total && s.cursors === 0;

  // Exactly the selected headings carry the explicit template marker.
  for (const lang of ['en', 'de']) {
    await page.goto(`${origin}/${lang}/`);
    const marked = await page.$$eval('[data-headline-typing]', e => e.map(x => x.id));
    assert(`${lang}: exactly hero-title and features-title are marked for typing`, JSON.stringify(marked) === JSON.stringify(TYPED));
    assert(`${lang}: only marked headings are on-page headings with that marker`, await page.$$eval('main h1, main h2, main h3', hs => hs.filter(h => h.hasAttribute('data-headline-typing')).length === 2));
  }

  // Desktop hero: live typing with real intermediate glyph visibility and an inline cursor.
  const [c1, p1] = await context({});
  try {
    await p1.goto(`${origin}/en/`);
    const label = await p1.locator('#hero-title').getAttribute('aria-label');
    const size = async () => p1.locator('#hero-title').evaluate(h => [h.offsetWidth, h.offsetHeight]);
    const before = await size();
    const samples = await sample(p1, 'hero-title', finished);
    const mid = samples.filter(s => s.shown > 0 && s.shown < s.total);
    assert('hero glyphs become visible one after another (real intermediate states)', mid.length >= 5);
    assert('revealed glyph count never decreases', samples.every((s, i) => !i || s.shown >= samples[i - 1].shown));
    assert('exactly one cursor while typing', mid.every(s => s.cursors === 1));
    assert('cursor sits immediately after the last revealed glyph', mid.every(adjacent));
    // Cursor blink: two discrete opacity phases per cycle, finite, visibly alternating.
    const blinkS = samples.filter(s => s.shown === s.total && s.cursors === 1);
    const flips = blinkS.filter((s, i) => i && s.opacity !== blinkS[i - 1].opacity).length;
    assert('cursor must visibly alternate opacity after the last glyph', blinkS.length >= 20 && new Set(blinkS.map(s => s.opacity)).size >= 2 && flips >= 4);
    assert('cursor blink is discrete: only fully on or fully off', blinkS.every(s => s.opacity === '0' || s.opacity === '1'));
    assert('cursor blink lasts about four seconds then ends', blinkS[blinkS.length - 1].at - blinkS[0].at >= 3500);
    assert('cursor is aria-hidden while it exists', samples.filter(s => s.cursors).every(s => s.hiddenAria === 'true') && samples.some(s => s.cursors));
    assert('typed hero never gets reveal or ambient glyph animations, incl. after the first ambient tick', samples.every(s => s.foreign === 0 && s.state === 'shown') && samples[samples.length - 1].at - samples[0].at > 2500);
    assert('hero ends fully visible with the cursor gone after its finite blink', finished(samples[samples.length - 1]));
    const afterSize = await size();
    assert('hero layout box is stable through typing', JSON.stringify(before) === JSON.stringify(afterSize));
    assert('hero accessible name unchanged and keeps its line break boundary', label === 'Your KNX project. Your workbench.' && await p1.locator('#hero-title br').count() === 1 && await p1.locator('#hero-title').getAttribute('aria-label') === label);
    assert('no endless cursor animation remains', await p1.evaluate(() => document.getAnimations().filter(a => a.id === 'headline-typing-cursor').length === 0));
    // Features heading types on first visibility only, and other headings never get a cursor.
    assert('features heading has not typed before it is seen', (await probe('features-title', p1)).shown === 0);
    await p1.evaluate(() => { const h = document.getElementById('features-title'); window.scrollTo(0, h.getBoundingClientRect().top + scrollY - innerHeight / 2); });
    const feat = await sample(p1, 'features-title', finished);
    assert('features heading types on first visibility', feat.some(s => s.shown > 0 && s.shown < s.total) && finished(feat[feat.length - 1]));
    assert('typed features heading never gets reveal or ambient glyph animations', feat.every(s => s.foreign === 0 && s.state === 'shown'));
    assert('features cursor follows the revealed text', feat.filter(s => s.shown > 0 && s.shown < s.total).every(adjacent));
    await p1.evaluate(() => window.scrollTo(0, 0));
    await p1.waitForTimeout(400);
    await p1.evaluate(() => { const h = document.getElementById('features-title'); window.scrollTo(0, h.getBoundingClientRect().top + scrollY - innerHeight / 2); });
    const again = [];
    for (let i = 0; i < 12; i++) { again.push(await probe('features-title', p1)); await p1.waitForTimeout(40); }
    assert('no typing replay on re-entry or scroll', again.every(s => s.shown === s.total && s.cursors === 0));
    assert('no replay for hero either', finished(await probe('hero-title', p1)));
    const others = await p1.evaluate(async () => {
      const seen = [];
      for (const h of document.querySelectorAll('main h1, main h2, main h3')) {
        if (h.hasAttribute('data-headline-typing')) continue;
        h.scrollIntoView({ block: 'center' });
        for (let i = 0; i < 40 && h.dataset.headlineState !== 'shown'; i++) await new Promise(r => setTimeout(r, 50));
        seen.push({ cursor: !!h.querySelector('.typing-cursor'), untyped: !!h.querySelector('[data-untyped]'), state: h.dataset.headlineState });
      }
      return seen;
    });
    assert('other headings never get a typing cursor or typing hiding', others.length > 5 && others.every(o => !o.cursor && !o.untyped));
    assert('other headings keep the existing reveal state', others.every(o => o.state === 'shown'));
  } finally { await c1.close(); }

  // Mobile, wrapped headings (German is longer): the cursor follows onto later lines.
  const [c2, p2] = await context({ viewport: { width: 320, height: 900 } });
  try {
    await p2.goto(`${origin}/de/`);
    const s = await sample(p2, 'hero-title', finished);
    const mid = s.filter(x => x.shown > 0 && x.shown < x.total);
    assert('mobile hero types with the cursor beside the last glyph', mid.length >= 5 && mid.every(adjacent));
    assert('mobile hero wraps and the cursor reaches a later line', new Set(mid.map(x => Math.round(x.last.top))).size >= 2);
    assert('mobile hero ends complete without cursor', finished(s[s.length - 1]));
    await p2.evaluate(() => { const h = document.getElementById('features-title'); window.scrollTo(0, h.getBoundingClientRect().top + scrollY - innerHeight / 2); });
    const f = await sample(p2, 'features-title', finished);
    const fm = f.filter(x => x.shown > 0 && x.shown < x.total);
    assert('mobile features heading wraps with an inline cursor', fm.length >= 5 && fm.every(adjacent) && new Set(fm.map(x => Math.round(x.last.top))).size >= 2);
    assert('mobile has no horizontal overflow', await p2.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert('mobile accessible name keeps word boundary', await p2.locator('#hero-title').getAttribute('aria-label') === 'Dein KNX-Projekt. Deine Werkbank.');
  } finally { await c2.close(); }

  // Forced colors: the cursor must stay visible in a system colour.
  const [cf, pf] = await context({ forcedColors: 'active' });
  try {
    await pf.goto(`${origin}/en/`);
    await pf.waitForFunction(() => document.querySelector('#hero-title .typing-cursor'), null, { timeout: 4000, polling: 15 });
    const cc = await pf.evaluate(() => { const c = document.querySelector('#hero-title .typing-cursor'), a = getComputedStyle(c, '::after'); const probe = document.createElement('i'); probe.style.color = 'CanvasText'; document.body.append(probe); const text = getComputedStyle(probe).color; probe.remove(); const probe2 = document.createElement('i'); probe2.style.color = 'Canvas'; document.body.append(probe2); const canvas = getComputedStyle(probe2).color; probe2.remove(); return { bg: a.backgroundColor, text, canvas }; });
    assert('forced-colors cursor uses CanvasText, not the page background', cc.bg === cc.text && cc.bg !== cc.canvas);
  } finally { await cf.close(); }

  // Reduced motion on load: static, complete, no cursor.
  const [c3, p3] = await context({ reducedMotion: 'reduce' });
  try {
    await p3.goto(`${origin}/en/`);
    await p3.waitForTimeout(300);
    assert('initial reduced motion shows hero immediately without a cursor', finished(await probe('hero-title', p3)));
    await p3.evaluate(() => { const h = document.getElementById('features-title'); window.scrollTo(0, h.getBoundingClientRect().top + scrollY - innerHeight / 2); });
    await p3.waitForTimeout(300);
    assert('initial reduced motion never types features either', finished(await probe('features-title', p3)));
  } finally { await c3.close(); }

  // Live reduction, then pause/resume, cancel mid-typing and never replay.
  for (const mode of ['reduced-motion', 'pause']) {
    const [c, p] = await context({});
    try {
      await p.goto(`${origin}/en/`);
      const read = () => p.evaluate(() => { const h = document.getElementById('hero-title'); return { total: h.querySelectorAll('.char').length, hidden: [...h.querySelectorAll('.char')].filter(c => getComputedStyle(c.firstChild).visibility !== 'visible' || c.hasAttribute('data-untyped')).length, cursors: h.querySelectorAll('.typing-cursor').length, anim: document.getAnimations().length }; });
      await p.waitForFunction(() => { const h = document.getElementById('hero-title'); return h.querySelector('.typing-cursor') && h.querySelector('[data-untyped]'); }, null, { timeout: 4000, polling: 15 });
      if (mode === 'pause') await p.locator('[data-motion-toggle]').click();
      else await p.emulateMedia({ reducedMotion: 'reduce' });
      // The media-query change event is delivered asynchronously; allow a few frames, not a replay.
      await p.waitForFunction(() => !document.querySelector('#hero-title .typing-cursor'), null, { timeout: 300, polling: 'raf' }).catch(() => {});
      const cut = await read();
      assert(`${mode}: cancellation shows all text and removes the cursor at once`, cut.hidden === 0 && cut.cursors === 0);
      await p.waitForTimeout(300);
      if (mode === 'pause') await p.locator('[data-motion-toggle]').click();
      else await p.emulateMedia({ reducedMotion: 'no-preference' });
      await p.evaluate(() => window.scrollTo(0, 1500)); await p.waitForTimeout(300); await p.evaluate(() => window.scrollTo(0, 0));
      for (let i = 0; i < 8; i++) { const r = await read(); if (r.hidden || r.cursors) throw new Error(`${mode}: cancelled typing replayed`); await p.waitForTimeout(40); }
      checks.push(`${mode}: resume does not replay cancelled typing`);
    } finally { await c.close(); }
  }

  // JavaScript disabled: plain readable static heading.
  const [c4, p4] = await context({ javaScriptEnabled: false });
  try {
    await p4.goto(`${origin}/en/`);
    const text = await p4.locator('#hero-title').innerText();
    assert('JS-disabled hero is plain readable text', text.replace(/\s+/g, ' ') === 'Your KNX project. Your workbench.' && await p4.locator('#hero-title .char').count() === 0 && await p4.locator('#hero-title').isVisible());
    assert('JS-disabled features heading is readable', (await p4.locator('#features-title').innerText()).replace(/\s+/g, ' ') === 'From a project file to a clear picture.');
  } finally { await c4.close(); }

  assert('no page errors or foreign requests', errors.length === 0 && unexpected.length === 0);
  return { count: checks.length, checks, errors, unexpected };
}
