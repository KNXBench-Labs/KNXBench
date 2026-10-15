/* Story/site/app.js headline treatment, scoped to the marketing page only. */
(() => {
  'use strict';
  if (!window.IntersectionObserver || !window.matchMedia ||
      !Element.prototype.animate || !Element.prototype.getAnimations) return;
  const headings = [...document.querySelectorAll('main h1, main h2, main h3')];
  const toggle = document.querySelector('[data-motion-toggle]');
  if (!headings.length || !toggle) return;
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)');
  const boxes = new Map(), visible = new Set(), active = new Set();
  const SWAP = { duration: 720, easing: 'cubic-bezier(.65,0,.35,1)', id: 'ambient-char-swap' };
  const CLIP = 'inset(-0.4em -0.06em)';
  let paused = false, unsupported = false, timer = 0, ticks = 0;
  const atlas = document.querySelector('.atlas-diagram');
  let atlasVisible = false, atlasStarted = false;

  function revealAtlas() {
    if (!atlas || !atlasVisible || atlasStarted || !allowed()) return;
    atlasStarted = true;
    try {
      const paths = [...atlas.querySelectorAll('path')].filter(path => path.closest('svg').getBoundingClientRect().width > 0);
      paths.forEach((path, index) => track(path.animate([
        { strokeDasharray: '1 1', strokeDashoffset: 1 },
        { strokeDasharray: '1 1', strokeDashoffset: 0 }
      ], { duration: 850, delay: index * 180, fill: 'backwards', easing: 'ease-out', id: 'phosphor-path-draw' })));
      atlas.querySelectorAll('[data-atlas-node]').forEach((node, index) => track(node.animate(
        [{ opacity: 0 }, { opacity: 1 }],
        { duration: 380, delay: index * 180, fill: 'backwards', easing: 'ease-out', id: 'phosphor-node-appear' }
      )));
    } catch (_) {
      // A partial/missing WAAPI implementation must leave the whole diagram visible.
      cancelWhere(animation => animation.id.startsWith('phosphor-'));
    }
  }

  function split(heading) {
    const label = heading.cloneNode(true);
    label.querySelectorAll('br').forEach(br => br.replaceWith(' '));
    heading.setAttribute('aria-label', label.textContent.replace(/\s+/g, ' ').trim());
    const walker = document.createTreeWalker(heading, NodeFilter.SHOW_TEXT);
    const texts = [], found = [];
    while (walker.nextNode()) texts.push(walker.currentNode);
    for (const text of texts) {
      const fragment = document.createDocumentFragment();
      for (const part of text.data.split(/([ \t\n\r\f]+)/)) {
        if (!part) continue;
        if (/^[ \t\n\r\f]+$/.test(part)) {
          fragment.appendChild(document.createTextNode(part));
          continue;
        }
        const word = document.createElement('span');
        word.className = 'char-word';
        word.setAttribute('aria-hidden', 'true');
        for (const character of Array.from(part)) {
          const box = document.createElement('span');
          box.className = 'char';
          box.dataset.char = character;
          const glyph = document.createElement('span');
          glyph.className = 'char-glyph';
          glyph.textContent = character;
          box.appendChild(glyph);
          word.appendChild(box);
          if (/\S/.test(character)) found.push(box);
        }
        fragment.appendChild(word);
      }
      text.replaceWith(fragment);
    }
    return found;
  }

  const allowed = () => !paused && !unsupported && !reduce.matches && !document.hidden;
  function cancelWhere(predicate) {
    for (const animation of active) if (predicate(animation)) animation.cancel();
  }
  function track(animation) {
    active.add(animation);
    animation.finished.then(() => active.delete(animation), () => active.delete(animation));
  }
  function roll(box, delay) {
    const timing = { ...SWAP, delay };
    try {
      track(box.animate([{ clipPath: CLIP }, { clipPath: CLIP }], timing));
      track(box.firstChild.animate([{ transform: 'translateX(0)' }, { transform: 'translateX(105%)' }], timing));
      track(box.animate([{ transform: 'translateX(-105%)', opacity: 1 }, { transform: 'translateX(0)', opacity: 1 }],
        { ...timing, pseudoElement: '::after' }));
    } catch (_) {
      unsupported = true;
      toggle.hidden = true;
      sync();
    }
  }
  function tick() {
    timer = 0;
    if (!allowed() || !visible.size) return;
    const pool = [];
    for (const heading of visible) {
      for (const box of boxes.get(heading)) if (!box.getAnimations({ subtree: true }).length) pool.push(box);
    }
    ticks += 1;
    for (let picked = 0; picked < 2 && pool.length && allowed(); picked += 1) {
      const [box] = pool.splice(Math.floor(Math.random() * pool.length), 1);
      roll(box, picked ? 60 + Math.random() * 160 : 0);
    }
    // An occasional single-glyph displacement, never flashing prose or whole headings.
    if (ticks % 7 === 0 && pool.length) {
      const box = pool[0];
      try {
        track(box.firstChild.animate([
          { transform: 'translateX(0)' },
          { transform: 'translateX(.08em)', color: '#c1aff2', offset: .4 },
          { transform: 'translateX(0)' }
        ], { duration: 180, easing: 'steps(2, end)', id: 'phosphor-char-glitch' }));
      } catch (_) { /* The normal rolling treatment remains available. */ }
    }
    if (allowed() && visible.size) timer = window.setTimeout(tick, 2200 + Math.random() * 1600);
  }
  function sync() {
    document.documentElement.dataset.motion = allowed() ? 'on' : 'off';
    if (!allowed() || !atlasVisible) cancelWhere(animation => animation.id.startsWith('phosphor-'));
    revealAtlas();
    if (!allowed() || !visible.size) {
      window.clearTimeout(timer);
      timer = 0;
      cancelWhere(animation => !allowed() || animation.id === 'ambient-char-swap' || animation.id === 'phosphor-char-glitch');
    } else if (!timer) timer = window.setTimeout(tick, 1200);
  }
  headings.forEach(heading => boxes.set(heading, split(heading)));
  const observer = new IntersectionObserver(entries => {
    for (const entry of entries) {
      if (entry.target === atlas) {
        atlasVisible = entry.isIntersecting;
        continue;
      }
      if (entry.isIntersecting) visible.add(entry.target);
      else {
        visible.delete(entry.target);
        cancelWhere(animation => entry.target.contains(animation.effect.target));
      }
    }
    sync();
  });
  headings.forEach(heading => observer.observe(heading));
  if (atlas) observer.observe(atlas);
  // Responsive art uses a different path geometry; cancel, never replay mid-resize.
  window.addEventListener('resize', () => cancelWhere(animation => animation.id.startsWith('phosphor-')));
  toggle.hidden = false;
  sync();
  toggle.addEventListener('click', () => {
    paused = !paused;
    toggle.setAttribute('aria-pressed', String(paused));
    toggle.textContent = paused ? toggle.dataset.motionResume : toggle.dataset.motionPause;
    sync();
  });
  if (reduce.addEventListener) reduce.addEventListener('change', sync);
  else reduce.addListener(sync);
  document.addEventListener('visibilitychange', sync);
  window.addEventListener('pagehide', () => {
    window.clearTimeout(timer);
    timer = 0;
    cancelWhere(() => true);
    document.documentElement.dataset.motion = 'off';
  });
  window.addEventListener('pageshow', sync);
})();
