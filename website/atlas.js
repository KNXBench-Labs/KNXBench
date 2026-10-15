/** Progressive selection for the fictional marketing installation diagram. */
(() => {
  'use strict';
  const atlas = document.querySelector('#installation-atlas');
  if (!atlas) return;
  const nodes = [...atlas.querySelectorAll('[data-atlas-node]')];
  const notes = [...atlas.querySelectorAll('.atlas-notes details')];
  const paths = [...atlas.querySelectorAll('[data-links]')];
  function select(node) {
    const id = node.dataset.atlasNode;
    nodes.forEach(item => {
      if (item === node) item.setAttribute('aria-current', 'true');
      else item.removeAttribute('aria-current');
    });
    paths.forEach(path => path.classList.toggle('is-related', path.dataset.links.split(' ').includes(id)));
    notes.forEach(note => { note.open = note.id === node.hash.slice(1); });
    nodes.forEach(item => item.setAttribute('aria-expanded', String(item === node)));
  }
  nodes.forEach(node => {
    const note = notes.find(item => item.id === node.hash.slice(1));
    node.setAttribute('aria-controls', note.id);
    node.setAttribute('aria-expanded', String(note.open));
    note.addEventListener('toggle', () => node.setAttribute('aria-expanded', String(note.open)));
  });
  nodes.forEach(node => node.addEventListener('click', event => {
    // Keep modified clicks and native fragment navigation usable without JS.
    if (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    select(node);
  }));
  // Native disclosure summaries remain independent keyboard/touch controls.
  // A fragment opened directly still reveals its explanation without JS.
  const fragment = nodes.find(node => node.hash === location.hash);
  select(fragment || nodes[0]);
})();
