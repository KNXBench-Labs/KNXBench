/* Story/site/app.js headline treatment, scoped to the marketing page only. */
(() => {
  'use strict';
  if (!window.IntersectionObserver || !window.matchMedia ||
      !Element.prototype.animate || !Element.prototype.getAnimations) return;
  const headings = [...document.querySelectorAll('main h1, main h2, main h3')];
  const toggle = document.querySelector('[data-motion-toggle]');
  if (!headings.length || !toggle) return;
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)');
  const boxes = new Map(), reveals = new Map(), visible = new Set(), active = new Set();
  // Selected headings (data-headline-typing) type once per page load instead of the fade reveal.
  const typings = new Map();
  const TYPE_MS = 50, BLINKS = 4, BLINK_MS = 1000;
  const SWAP = { duration: 720, easing: 'cubic-bezier(.65,0,.35,1)', id: 'ambient-char-swap' };
  const CLIP = 'inset(-0.4em -0.06em)';
  let paused = false, unsupported = false, pageHidden = false, timer = 0, ticks = 0;

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

  const allowed = () => !paused && !unsupported && !pageHidden && !reduce.matches && !document.hidden;
  function settleReveal(heading, shown) {
    const state = reveals.get(heading);
    state.shown = shown;
    state.target = shown;
    heading.dataset.headlineState = shown ? 'shown' : 'hidden';
    const animations = state.animations;
    state.animations = [];
    animations.forEach(animation => animation.cancel());
  }
  function reveal(heading, shown) {
    const state = reveals.get(heading);
    if (state.animations.length) {
      if (state.target !== shown) {
        state.target = shown;
        // Keep the exact current time: fast scroll reversals must not jump.
        for (const animation of state.animations) {
          animation.playbackRate = shown ? 1 : -1;
          animation.play();
        }
      }
      return;
    }
    if (state.shown === shown) return;
    state.target = shown;
    const chars = boxes.get(heading);
    // SessionRadar's 650ms roll-in + 24ms stagger, capped for long headings.
    const stagger = Math.min(24, 600 / Math.max(1, chars.length - 1));
    const duration = 650 + stagger * Math.max(0, chars.length - 1);
    const animations = state.animations;
    try {
      chars.forEach((box, index) => {
        const hidden = { opacity: 0, transform: 'translateY(.3em) rotateX(-45deg)' };
        const shownFrame = { opacity: 1, transform: 'translateY(0) rotateX(0)' };
        const animation = box.animate([
          { ...hidden, offset: 0 },
          { ...hidden, offset: index * stagger / duration, easing: 'ease' },
          { ...shownFrame, offset: (index * stagger + 650) / duration },
          { ...shownFrame, offset: 1 }
        ], { duration, fill: 'both', id: 'headline-reveal' });
        animations.push(animation);
        animation.currentTime = state.shown ? duration : 0;
        animation.playbackRate = shown ? 1 : -1;
        animation.play();
      });
      Promise.all(animations.map(animation => animation.finished)).then(() => {
        if (state.animations === animations) settleReveal(heading, state.target);
      }, () => {});
    } catch (_) {
      unsupported = true;
      toggle.hidden = true;
      sync();
    }
  }
  function placeCursor(typing, index) {
    const { cursor, chars } = typing;
    // Zero-width and inside the nowrap word: layout and wrapping stay those of the full text.
    if (index < 0) chars[0].parentNode.insertBefore(cursor, chars[0]);
    else chars[index].parentNode.insertBefore(cursor, chars[index].nextSibling);
  }
  function finishTyping(heading, blink) {
    const typing = typings.get(heading);
    if (typing.status === 'done') return;
    window.clearTimeout(typing.timer);
    typing.status = 'done';
    typing.chars.forEach(box => box.removeAttribute('data-untyped'));
    heading.dataset.typingState = 'done';
    const { cursor } = typing;
    if (blink && cursor.isConnected) {
      try {
        // step-end per keyframe interval: on for the first half of a cycle, off for the second.
        const animation = cursor.animate([
          { opacity: 1, offset: 0, easing: 'step-end' },
          { opacity: 0, offset: .5, easing: 'step-end' },
          { opacity: 1, offset: 1 }
        ], { duration: BLINK_MS, iterations: BLINKS, id: 'headline-typing-cursor' });
        const remove = () => cursor.remove();
        animation.finished.then(remove, () => {});
        typing.blink = animation;
        return;
      } catch (_) { /* Falls through to removing the cursor. */ }
    }
    cursor.remove();
  }
  function stopTyping(heading) {
    const typing = typings.get(heading);
    if (typing.blink) typing.blink.cancel();
    typing.cursor.remove();
    finishTyping(heading, false);
  }
  function startTyping(heading) {
    const typing = typings.get(heading);
    typing.status = 'running';
    heading.dataset.typingState = 'typing';
    placeCursor(typing, -1);
    const step = () => {
      typing.index += 1;
      typing.chars[typing.index].removeAttribute('data-untyped');
      placeCursor(typing, typing.index);
      if (typing.index >= typing.chars.length - 1) finishTyping(heading, true);
      else typing.timer = window.setTimeout(step, TYPE_MS);
    };
    typing.timer = window.setTimeout(step, TYPE_MS);
  }
  function cancelWhere(predicate) {
    for (const animation of active) if (predicate(animation)) animation.cancel();
  }
  function track(animation) {
    active.add(animation);
    animation.finished.then(() => active.delete(animation), () => active.delete(animation));
  }
  function roll(box, delay) {
    const timing = { ...SWAP, delay };
    const [axis, sign] = [['X', 1], ['X', -1], ['Y', -1], ['Y', 1]][Math.floor(Math.random() * 4)];
    const outgoing = `translate${axis}(${sign * 105}%)`;
    const incoming = `translate${axis}(${-sign * 105}%)`;
    const clip = axis === 'Y' ? 'inset(0 -0.06em)' : CLIP;
    try {
      track(box.animate([{ clipPath: clip }, { clipPath: clip }], timing));
      track(box.firstChild.animate([{ transform: 'translate(0, 0)' }, { transform: outgoing }], timing));
      track(box.animate([{ transform: incoming, opacity: 1 }, { transform: 'translate(0, 0)', opacity: 1 }],
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
      if (typings.has(heading)) continue;
      const state = reveals.get(heading);
      if (!state.shown || state.animations.length) continue;
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
          { transform: 'translate(0, 0)' },
          { transform: 'translateX(.08em)', color: '#c1aff2', offset: .4 },
          { transform: 'translate(0, 0)' }
        ], { duration: 180, easing: 'steps(2, end)', id: 'phosphor-char-glitch' }));
      } catch (_) { /* The normal rolling treatment remains available. */ }
    }
    if (allowed() && visible.size) timer = window.setTimeout(tick, 2200 + Math.random() * 1600);
  }
  function sync() {
    document.documentElement.dataset.motion = allowed() ? 'on' : 'off';
    for (const heading of headings) {
      const typing = typings.get(heading);
      if (typing) {
        if (!allowed()) stopTyping(heading);
        else if (typing.status === 'idle' && visible.has(heading)) startTyping(heading);
        continue;
      }
      if (allowed()) reveal(heading, visible.has(heading));
      else settleReveal(heading, true);
    }
    if (!allowed() || !visible.size) {
      window.clearTimeout(timer);
      timer = 0;
      cancelWhere(() => true);
    } else if (!timer) timer = window.setTimeout(tick, 1200);
  }
  headings.forEach(heading => {
    const found = split(heading);
    boxes.set(heading, found);
    if (heading.hasAttribute('data-headline-typing') && found.length) {
      const cursor = document.createElement('span');
      cursor.className = 'typing-cursor';
      cursor.setAttribute('aria-hidden', 'true');
      found.forEach(box => box.setAttribute('data-untyped', ''));
      typings.set(heading, { chars: found, cursor, index: -1, status: 'idle', timer: 0, blink: null });
      heading.dataset.typingState = 'pending';
      reveals.set(heading, { shown: true, target: true, animations: [] });
      heading.dataset.headlineState = 'shown';
      return;
    }
    reveals.set(heading, { shown: false, target: false, animations: [] });
    heading.dataset.headlineState = 'hidden';
  });
  const observer = new IntersectionObserver(entries => {
    for (const entry of entries) {
      if (entry.isIntersecting && entry.intersectionRatio >= .15) visible.add(entry.target);
      else {
        visible.delete(entry.target);
        cancelWhere(animation => entry.target.contains(animation.effect.target));
      }
    }
    sync();
  }, { rootMargin: '-5% 0px -5% 0px', threshold: [0, .15] });
  headings.forEach(heading => observer.observe(heading));
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
    pageHidden = true;
    sync();
  });
  window.addEventListener('pageshow', () => {
    pageHidden = false;
    sync();
  });
})();
