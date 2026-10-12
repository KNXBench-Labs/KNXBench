/** Isolated fictional group-address study model; never a production KNX adapter. */
(() => {
  const mainNames = { 0: 'Zentral', 1: 'Beleuchtung', 2: 'Beschattung', 3: 'Heizung', 4: 'Reserve / ohne Bereich' };
  const middleNames = { 0: { 0: 'Zentralfunktionen' }, 1: { 0: 'Schalten', 1: 'Dimmen', 2: 'Rückmeldung' }, 2: { 0: 'Fahren', 1: 'Position' }, 3: { 0: 'Sollwert', 1: 'Istwert' }, 4: { 0: 'Ohne Bereich' } };
  const objects = [
    ['light-button', 'Bedienung Licht', 'Schalten', 'light'], ['light-actor', 'Aktor Licht', 'Schalten', 'light'],
    ['dim-button', 'Bedienung Licht', 'Dimmen', 'light'], ['dim-actor', 'Aktor Licht', 'Dimmwert', 'light'],
    ['light-status', 'Aktor Licht', 'Rückmeldung', 'light'], ['display', 'Visualisierung', 'Anzeige', 'light'],
    ['blind-button', 'Bedienung Rollo', 'Fahren', 'blind'], ['blind-actor', 'Aktor Rollo', 'Fahren', 'blind'],
    ['blind-position', 'Aktor Rollo', 'Position', 'blind'], ['heat-controller', 'Regler', 'Sollwert', 'heat'],
    ['heat-valve', 'Heizungsaktor', 'Sollwert', 'heat'], ['heat-status', 'Regler', 'Istwert', 'heat'],
  ].map(([id, device, name, family]) => ({ id, device, name, family }));
  const addresses = [];
  const add = (id, main, middle, sub, name, dpts, links, extra = {}) => addresses.push({
    id, raw: main * 2048 + middle * 256 + sub, name, dpts,
    range: `${mainNames[main]} / ${middleNames[main][middle]}`, linksKnown: true,
    links: links.map(([object, direction]) => ({ object, direction })), ...extra,
  });
  const rooms = [['kitchen', 'Küche'], ['living', 'Wohnzimmer'], ['hall', 'Flur'], ['office', 'Büro'], ['bed', 'Schlafzimmer'], ['terrace', 'Terrasse']];
  rooms.forEach(([slug, room], i) => {
    add(`light-${slug}-switch`, 1, 0, i + 1, `${room} · Licht schalten`, ['1.001'], [['light-button', 'send'], ['light-actor', 'receive']]);
    add(`light-${slug}-dim`, 1, 1, i + 1, `${room} · Helligkeit`, ['5.001'], [['dim-button', 'send'], ['dim-actor', 'receive']]);
    add(`light-${slug}-status`, 1, 2, i + 1, `${room} · Lichtstatus`, ['1.001'], [['light-status', 'send'], ['display', 'receive']]);
  });
  rooms.filter(([slug]) => ['kitchen', 'living', 'office', 'bed'].includes(slug)).forEach(([slug, room], i) => {
    add(`blind-${slug}-move`, 2, 0, i + 1, `${room} · Rollo fahren`, ['1.008'], [['blind-button', 'send'], ['blind-actor', 'receive']]);
    add(`blind-${slug}-position`, 2, 1, i + 1, `${room} · Rolloposition`, ['5.001'], [['blind-position', 'send']]);
  });
  [['kitchen', 'Küche'], ['living', 'Wohnzimmer'], ['bath', 'Bad']].forEach(([slug, room], i) => {
    add(`heat-${slug}-set`, 3, 0, i + 1, `${room} · Solltemperatur`, ['9.001'], [['heat-controller', 'send'], ['heat-valve', 'receive']]);
    add(`heat-${slug}-actual`, 3, 1, i + 1, `${room} · Isttemperatur`, ['9.001'], [['heat-status', 'send']]);
  });
  add('spare-unknown', 4, 0, 1, 'Reserve · Daten unvollständig', [], [], { range: null, linksKnown: false });
  add('spare-unlinked', 4, 0, 2, 'Reserve · Noch nicht verknüpft', [], [], { range: null });
  add('light-conflict', 1, 2, 30, 'Küche · Widersprüchlicher Status', ['1.001', '5.001'], [['light-status', 'send'], ['dim-actor', 'receive']]);
  add('central-off', 0, 0, 1, 'Haus · Zentral aus', ['1.001'], [['light-button', 'send'], ['light-actor', 'receive'], ['display', 'send'], ['display', 'receive']]);
  function freeze(value) { if (value && typeof value === 'object') { Object.values(value).forEach(freeze); Object.freeze(value); } return value; }
  freeze(addresses); freeze(objects);
  const levels = a => [a.raw >> 11, (a.raw >> 8) & 7, a.raw & 255];
  const formatAddress = (a, notation) => notation === 'two' ? `${a.raw >> 11}/${a.raw & 2047}` : levels(a).join('/');
  const createState = () => ({ view: 'tree', notation: 'three', query: '', filter: 'all', main: null, middle: null, selected: [], focus: null });
  function visibleAddresses(state) {
    const needle = state.query.trim().toLocaleLowerCase('de');
    return addresses.filter(a => {
      const [main, middle] = levels(a);
      if (state.main !== null && state.main !== main) return false;
      if (state.notation === 'three' && state.middle !== null && state.middle !== middle) return false;
      if (state.filter === 'conflict' && a.dpts.length < 2) return false;
      if (state.filter === 'unknown' && a.dpts.length !== 0) return false;
      if (state.filter === 'unlinked' && !(a.linksKnown && a.links.length === 0)) return false;
      const spellings = [formatAddress(a, 'two'), formatAddress(a, 'three')];
      return !needle || [a.name, a.range ?? 'ohne Bereich', ...a.dpts, ...spellings, ...spellings.map(v => v.replaceAll('/', '.'))].some(v => v.toLocaleLowerCase('de').includes(needle));
    }).sort((a, b) => a.raw - b.raw);
  }
  function buildGroups(input, notation) {
    const map = new Map();
    [...input].sort((a, b) => a.raw - b.raw).forEach(a => {
      const [main, middle] = levels(a);
      if (!map.has(main)) map.set(main, { main, name: mainNames[main] ?? `Hauptgruppe ${main}`, addresses: [], children: [] });
      const group = map.get(main); group.addresses.push(a);
      if (notation === 'three') {
        let child = group.children.find(c => c.middle === middle);
        if (!child) { child = { middle, name: middleNames[main]?.[middle] ?? `Mittelgruppe ${middle}`, addresses: [] }; group.children.push(child); }
        child.addresses.push(a);
      }
    });
    return [...map.values()];
  }
  function connectionStatus(a, object) {
    if (!a.linksKnown) return 'unknown';
    const links = a.links.filter(l => l.object === object);
    const send = links.some(l => l.direction === 'send'), receive = links.some(l => l.direction === 'receive');
    return send && receive ? 'both' : send ? 'send' : receive ? 'receive' : 'none';
  }
  function switchView(state, view) {
    if (!['tree', 'columns', 'atlas', 'matrix'].includes(view)) throw new Error('Unknown view');
    return { ...state, view };
  }
  function selectAddress(state, id, additive) {
    if (!addresses.some(a => a.id === id)) throw new Error('Unknown address');
    const selected = additive ? (state.selected.includes(id) ? state.selected.filter(i => i !== id) : [...state.selected, id]) : [id];
    return { ...state, selected, focus: id };
  }
  globalThis.GaStudy = { addresses, objects, levels, formatAddress, createState, visibleAddresses, buildGroups, connectionStatus, switchView, selectAddress };
})();
