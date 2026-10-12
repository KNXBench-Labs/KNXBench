/** Four standalone read-only studies sharing one fictional model and selection state. */
(() => {
  'use strict';
  const m = globalThis.GaStudy;
  const $ = id => document.getElementById(id);
  const esc = value => String(value).replace(/[&<>"']/g, ch => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[ch]);
  let state = m.createState();
  const expanded = new Set(['m:1', 's:1:0']);
  let matrixFamily = 'light';
  let chosenObject = null;
  const descriptions = {
    tree: ['Struktur lesen, Details vergleichen', 'Bereiche aufklappen und Adressen mit DPTs und Verknüpfungen direkt vergleichen.', 'Klassisch', 'Checkboxen oder Strg/Cmd + Klick: mehrere Adressen auswählen.'],
    columns: ['Schritt für Schritt navigieren', 'Jede Ebene bekommt eine Spalte. Der gewählte Bereich gilt auch nach einem Ansichtswechsel.', 'Klassisch', 'Bereich links wählen; Adressen rechts auswählen. Jede Spalte scrollt unabhängig.'],
    atlas: ['Den Adressraum erkunden', 'Bereichsflächen öffnen und bis zur einzelnen Adresse hineinzoomen. Keine Gebäudekarte.', 'Experimentell', 'Fläche öffnen: nächste Ebene. Breadcrumb: zurück. Strg/Cmd + Klick: Mehrfachauswahl.'],
    matrix: ['Projektierte Beziehungen verstehen', 'Zeilen sind Adressen, Spalten Kommunikationsobjekte. Eine Zelle öffnet die Verknüpfungsdetails.', 'Experimentell', 'Objektspalten filtern oder horizontal scrollen. S/E zeigt Projektlinks, keine Busaktivität.'],
  };
  const directionNames = { send: 'Send-Verknüpfung', receive: 'Receive-Verknüpfung', both: 'Send und Receive', none: 'Keine Verknüpfung (bekannt)', unknown: 'Verknüpfungen unbekannt' };
  const directionSymbols = { send: 'S', receive: 'E', both: 'S/E', none: '—', unknown: '?' };
  const allGroups = () => m.buildGroups(m.addresses, state.notation);
  const selected = a => state.selected.includes(a.id);
  const addressLabel = a => `${m.formatAddress(a, state.notation)} ${a.name}`;
  function dptBadge(a) {
    if (a.dpts.length === 0) return '<span class="badge unknown">DPT unbekannt</span>';
    if (a.dpts.length > 1) return `<span class="badge conflict" title="${esc(a.dpts.join(' / '))}">DPT-Konflikt</span>`;
    return `<span class="mono">${esc(a.dpts[0])}</span>`;
  }
  function linkBadge(a) {
    if (!a.linksKnown) return '<span class="badge unknown">Unbekannt</span>';
    if (!a.links.length) return '<span class="badge">Keine</span>';
    const send = a.links.filter(l => l.direction === 'send').length;
    const receive = a.links.filter(l => l.direction === 'receive').length;
    return `<span class="mono" title="${send} Send- und ${receive} Receive-Verknüpfungen">S ${send} · E ${receive}</span>`;
  }
  function checkbox(a) { return `<input type="checkbox" data-action="check" data-id="${esc(a.id)}" data-key="check-${esc(a.id)}" aria-label="${esc(addressLabel(a))} zur Auswahl hinzufügen" ${selected(a) ? 'checked' : ''}>`; }
  function row(a, indent = false) {
    return `<tr class="${selected(a) ? 'selected' : ''}" data-address="${esc(a.id)}"><td class="check-cell">${checkbox(a)}</td><td class="address-cell ${indent ? 'indent' : ''}"><button class="address-button mono" data-action="select" data-id="${esc(a.id)}" data-key="select-${esc(a.id)}" aria-label="${esc(addressLabel(a))}">${m.formatAddress(a, state.notation)}</button></td><td>${esc(a.name)}</td><td>${dptBadge(a)}</td><td>${linkBadge(a)}</td></tr>`;
  }
  const empty = () => '<div class="empty-state"><strong>Keine passenden Adressen</strong>Ändere die Suche, wähle einen anderen Bereich oder setze die Filter zurück.</div>';
  function treeView(rows) {
    if (!rows.length) return empty();
    const forced = !!state.query.trim() || state.filter !== 'all';
    let body = '';
    for (const g of m.buildGroups(rows, state.notation)) {
      const key = `m:${g.main}`, open = forced || expanded.has(key);
      body += `<tr class="group-row"><td colspan="5"><button data-action="expand" data-id="${key}" data-key="${key}" aria-expanded="${open}" ${forced ? 'disabled' : ''}><span class="arrow" aria-hidden="true">${open ? '▾' : '▸'}</span><span class="mono">${g.main}</span>${esc(g.name)}<span class="count">${g.addresses.length} ${g.addresses.length === 1 ? 'Adresse' : 'Adressen'}</span></button></td></tr>`;
      if (!open) continue;
      if (state.notation === 'two') { body += g.addresses.map(a => row(a, true)).join(''); continue; }
      for (const c of g.children) {
        const childKey = `s:${g.main}:${c.middle}`, childOpen = forced || expanded.has(childKey);
        body += `<tr class="group-row level-two"><td colspan="5"><button data-action="expand" data-id="${childKey}" data-key="${childKey}" aria-expanded="${childOpen}" ${forced ? 'disabled' : ''}><span class="arrow" aria-hidden="true">${childOpen ? '▾' : '▸'}</span><span class="mono">${g.main}/${c.middle}</span>${esc(c.name)}<span class="count">${c.addresses.length}</span></button></td></tr>`;
        if (childOpen) body += c.addresses.map(a => row(a, true)).join('');
      }
    }
    return `<div class="table-tools"><button data-action="expand-all" data-key="expand-all" ${forced ? 'disabled' : ''}>Alle öffnen</button><button data-action="collapse-all" data-key="collapse-all" ${forced ? 'disabled' : ''}>Alle schließen</button><span class="caption">${forced ? 'Trefferzweige automatisch geöffnet' : 'Numerisch nach Adresse sortiert'}</span></div><table class="address-table"><thead><tr><th><span class="sr-only">Auswahl</span></th><th>Adresse</th><th>Name</th><th>DPT</th><th>Links</th></tr></thead><tbody>${body}</tbody></table>`;
  }
  function columnsView(rows) {
    const globalRows = m.visibleAddresses({ ...state, main: null, middle: null });
    const groups = m.buildGroups(globalRows, state.notation);
    const mainColumn = `<div class="browser-column"><h3 class="column-title">Hauptgruppen</h3><button class="browser-item" data-action="scope-main" data-id="all" data-key="main-all" aria-current="${state.main === null}"><span>Alle Bereiche</span><small>${globalRows.length}</small></button>${groups.map(g => `<button class="browser-item" data-action="scope-main" data-id="${g.main}" data-key="main-${g.main}" aria-current="${state.main === g.main}"><span><span class="number mono">${g.main}</span>${esc(g.name)}</span><small>${g.addresses.length} ›</small></button>`).join('')}</div>`;
    const currentGroup = groups.find(g => g.main === state.main);
    const middleColumn = state.notation === 'two' ? '' : `<div class="browser-column"><h3 class="column-title">Mittelgruppen</h3>${state.main === null ? '<p class="column-placeholder">Wähle eine Hauptgruppe. Rechts sind bereits alle passenden Adressen erreichbar.</p>' : `<button class="browser-item" data-action="scope-middle" data-id="all" data-key="middle-all" aria-current="${state.middle === null}">Alle Mittelgruppen</button>${(currentGroup?.children ?? []).map(c => `<button class="browser-item" data-action="scope-middle" data-id="${c.middle}" data-key="middle-${c.middle}" aria-current="${state.middle === c.middle}"><span><span class="number mono">${state.main}/${c.middle}</span>${esc(c.name)}</span><small>${c.addresses.length} ›</small></button>`).join('')}`}</div>`;
    const addressColumn = `<div class="browser-column"><h3 class="column-title">${rows.length} Gruppenadressen</h3>${rows.length ? rows.map(a => `<div class="browser-address ${selected(a) ? 'selected' : ''}" data-address="${esc(a.id)}">${checkbox(a)}<button data-action="select" data-id="${esc(a.id)}" data-key="select-${esc(a.id)}" aria-label="${esc(addressLabel(a))}"><span class="mono">${m.formatAddress(a, state.notation)}</span><small>${esc(a.name)}</small></button></div>`).join('') : empty()}</div>`;
    return `<div class="columns-browser ${state.notation === 'two' ? 'two-level' : ''}">${mainColumn}${middleColumn}${addressColumn}</div>`;
  }
  function atlasView(rows) {
    if (!rows.length) return empty();
    const groups = m.buildGroups(rows, state.notation);
    let title, level, tiles, help;
    const tile = (code, name, list, action, id) => `<button class="atlas-tile" data-action="${action}" data-id="${id}" data-key="atlas-${action}-${id}"><span class="tile-code">${esc(code)}</span><strong>${esc(name)}</strong><small>${list.length} ${list.length === 1 ? 'Adresse' : 'Adressen'}</small>${list.some(a => a.dpts.length > 1) ? '<span class="tile-state">! Konflikt</span>' : ''}<span class="tile-bottom">Bereich öffnen ↗</span></button>`;
    let cards = false;
    if (state.main === null) {
      title = 'Das ganze Projekt'; level = 'Hauptgruppen';
      tiles = groups.map(g => tile(g.main, g.name, g.addresses, 'scope-main', g.main)).join('');
      help = 'Flächen bleiben gleich groß: Anzahl und Warnungen sind beschriftet, nicht nur farbcodiert.';
    } else if (state.notation === 'three' && state.middle === null) {
      const group = groups.find(g => g.main === state.main);
      title = group?.name ?? 'Bereich'; level = 'Mittelgruppen';
      tiles = (group?.children ?? []).map(c => tile(`${state.main}/${c.middle}`, c.name, c.addresses, 'scope-middle', c.middle)).join('');
      help = 'Öffne einen Unterbereich. Der Breadcrumb führt dich jederzeit wieder zurück.';
    } else {
      title = 'Adressen im Fokus'; level = 'Gruppenadressen'; cards = true;
      tiles = rows.map(a => `<button class="atlas-tile ${selected(a) ? 'selected' : ''}" data-action="select" data-id="${esc(a.id)}" data-key="select-${esc(a.id)}" data-address="${esc(a.id)}" aria-label="${esc(addressLabel(a))}" aria-pressed="${selected(a)}"><span class="tile-code">${m.formatAddress(a, state.notation)}</span><strong>${esc(a.name)}</strong><small>${a.dpts.length > 1 ? 'DPT-Konflikt' : a.dpts[0] ? `DPT ${esc(a.dpts[0])}` : 'DPT unbekannt'}</small><span class="tile-bottom">${a.linksKnown ? `${a.links.length} Projektlinks` : 'Verknüpfungen unbekannt'}</span></button>`).join('');
      help = 'Adresse öffnen: Eigenschaften rechts. Strg/Cmd + Klick fügt sie zur Auswahl hinzu.';
    }
    return `<div class="atlas-intro"><div><h3>${esc(title)}</h3><p>Bereiche erkunden, Identitäten behalten.</p></div><span class="atlas-level">${level}</span></div><div class="atlas-grid ${cards ? 'addresses' : ''}">${tiles}</div><p class="atlas-help">${help} Die Flächen sind keine physische Topologie.</p>`;
  }
  function matrixView(rows) {
    const objects = m.objects.filter(o => matrixFamily === 'all' || o.family === matrixFamily);
    const toolbar = `<div class="matrix-toolbar"><label>Objektspalten<select id="matrix-family" aria-label="Objektspalten"><option value="light" ${matrixFamily === 'light' ? 'selected' : ''}>Beleuchtung</option><option value="blind" ${matrixFamily === 'blind' ? 'selected' : ''}>Beschattung</option><option value="heat" ${matrixFamily === 'heat' ? 'selected' : ''}>Heizung</option><option value="all" ${matrixFamily === 'all' ? 'selected' : ''}>Alle 12 Objekte</option></select></label><div class="legend"><span>S · Send</span><span>E · Receive</span><span>S/E · beide</span><span>— · keine</span><span>? · unbekannt</span></div></div>`;
    if (!rows.length) return toolbar + empty();
    return `${toolbar}<div class="matrix-scroll"><table class="matrix-table"><thead><tr><th>Gruppenadresse<small>${rows.length} Zeilen · ${objects.length} Objekte</small></th>${objects.map(o => `<th scope="col">${esc(o.device)}<small>${esc(o.name)}</small></th>`).join('')}</tr></thead><tbody>${rows.map(a => `<tr class="${selected(a) ? 'selected' : ''}" data-address="${esc(a.id)}"><th scope="row"><button class="address-button mono" data-action="select" data-id="${esc(a.id)}" data-key="select-${esc(a.id)}" aria-label="${esc(addressLabel(a))}">${m.formatAddress(a, state.notation)}</button><small title="${esc(a.name)}">${esc(a.name)}</small></th>${objects.map(o => {
      const status = m.connectionStatus(a, o.id);
      return `<td><button class="matrix-cell ${status}" data-action="cell" data-id="${esc(a.id)}" data-object="${esc(o.id)}" data-key="cell-${esc(a.id)}-${esc(o.id)}" aria-label="${esc(addressLabel(a))} · ${esc(o.device)} ${esc(o.name)} · ${directionNames[status]}">${directionSymbols[status]}</button></td>`;
    }).join('')}</tr>`).join('')}</tbody></table></div><p class="matrix-foot">Die Objektspalten filtern nur die Spalten, nicht die Adresszeilen. Angaben sind fiktive Projektlinks; keine Busbeobachtung.</p>`;
  }
  function renderInspector() {
    const a = m.addresses.find(a => a.id === state.focus);
    if (!a) { $('inspector').innerHTML = '<div class="inspector-empty"><div class="empty-icon" aria-hidden="true">▤</div><strong>Eine Auswahl, vier Ansichten</strong><p>Wähle eine Gruppenadresse.<br>Ihre Eigenschaften bleiben beim Ansichtswechsel erhalten.</p></div>'; return; }
    const alternate = m.formatAddress(a, state.notation === 'three' ? 'two' : 'three');
    const links = a.links.map(l => { const o = m.objects.find(o => o.id === l.object); return `<div class="link-item"><span class="badge">${l.direction === 'send' ? 'S' : 'E'}</span><div>${esc(o.device)}<small>${esc(o.name)} · ${directionNames[l.direction]}</small></div></div>`; }).join('');
    const object = m.objects.find(o => o.id === chosenObject);
    const cell = object ? `<div class="object-detail"><strong>Gewählte Matrixzelle</strong><p>${esc(object.device)} · ${esc(object.name)}</p><p>${directionNames[m.connectionStatus(a, object.id)]}</p></div>` : '';
    $('inspector').innerHTML = `<div data-inspected="${esc(a.id)}"><div class="inspector-address mono">${m.formatAddress(a, state.notation)}</div><h3 class="inspector-name">${esc(a.name)}</h3><dl><dt>Gespeicherter Beispielbereich</dt><dd>${esc(a.range ?? 'Keinem Bereich zugeordnet')}</dd><dt>Datapoint-Typ</dt><dd>${dptBadge(a)}${a.dpts.length > 1 ? `<p class="mono">${esc(a.dpts.join(' / '))}</p>` : ''}</dd><dt>Alternative Darstellung</dt><dd class="mono">${alternate}</dd><dt>Stabile Beispiel-ID</dt><dd class="mono">${esc(a.id)}</dd><dt>Auswahl</dt><dd>${state.selected.length} Adresse${state.selected.length === 1 ? '' : 'n'} ausgewählt</dd></dl>${cell}<h3>Projektierte Verknüpfungen</h3>${a.linksKnown ? links || '<p class="badge">Keine Verknüpfung (bekannt)</p>' : '<p class="badge unknown">Verknüpfungsdaten nicht vorhanden</p>'}<p class="disclaimer">Fiktive, unveränderliche Beispieldaten. „S“ und „E“ bezeichnen modellierte Send-/Receive-Verknüpfungen, nicht gemessene Busaktivität.</p></div>`;
  }
  function render() {
    const focusedKey = document.activeElement?.dataset?.key;
    const rows = m.visibleAddresses(state), groups = allGroups();
    const description = descriptions[state.view];
    document.querySelectorAll('[data-view]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.view === state.view)));
    $('concept-title').textContent = description[0]; $('concept-description').textContent = description[1]; $('concept-badge').textContent = description[2]; $('view-hint').textContent = description[3];
    $('notation').value = state.notation;
    $('explorer-ranges').innerHTML = `<button data-action="scope-main" data-id="all" data-key="explorer-all" aria-current="${state.main === null}">Alle Bereiche <span>36</span></button>${groups.map(g => `<button data-action="scope-main" data-id="${g.main}" data-key="explorer-${g.main}" aria-current="${state.main === g.main}">${esc(g.name)}<span>${g.addresses.length}</span></button>`).join('')}`;
    const main = groups.find(g => g.main === state.main), middle = main?.children.find(c => c.middle === state.middle);
    $('breadcrumbs').innerHTML = '<button data-action="scope-main" data-id="all" data-key="crumb-all">Alle Bereiche</button>' + (main ? `<span aria-hidden="true">/</span><button data-action="scope-main" data-id="${main.main}" data-key="crumb-main">${esc(main.name)}</button>` : '') + (middle && state.notation === 'three' ? `<span aria-hidden="true">/</span><span>${esc(middle.name)}</span>` : '');
    $('result-count').textContent = `${rows.length} von 36 Adressen`;
    const hiddenSelection = state.selected.filter(id => !rows.some(a => a.id === id)).length;
    $('selection-status').hidden = state.selected.length === 0;
    $('selection-status').innerHTML = `<span>${state.selected.length} ausgewählt${hiddenSelection ? ` · ${hiddenSelection} außerhalb des aktuellen Filters` : ''}</span><button data-action="clear-selection" data-key="clear-selection">Auswahl aufheben</button>`;
    $('workspace').className = `view-${state.view}`;
    $('workspace').innerHTML = ({ tree: treeView, columns: columnsView, atlas: atlasView, matrix: matrixView })[state.view](rows);
    renderInspector();
    if (focusedKey) { const next = document.querySelector(`[data-key="${CSS.escape(focusedKey)}"]`); (next ?? $('workspace-title')).focus({ preventScroll: true }); }
  }
  document.addEventListener('click', event => {
    const button = event.target.closest('button[data-view], [data-action]');
    if (!button || button.disabled) return;
    if (button.dataset.view) {
      state = m.switchView(state, button.dataset.view);
      try { localStorage.setItem('knxbench.ga-study.view.v1', state.view); } catch { /* Local files/private mode can refuse persistence. */ }
      render(); return;
    }
    const { action, id } = button.dataset;
    if (action === 'select' || action === 'check' || action === 'cell') {
      state = m.selectAddress(state, id, action === 'check' || event.ctrlKey || event.metaKey);
      chosenObject = action === 'cell' ? button.dataset.object : null;
    } else if (action === 'scope-main') {
      state = { ...state, main: id === 'all' ? null : Number(id), middle: null };
      if (state.main !== null) expanded.add(`m:${state.main}`);
    } else if (action === 'scope-middle') {
      state = { ...state, middle: id === 'all' ? null : Number(id) };
      if (state.middle !== null) expanded.add(`s:${state.main}:${state.middle}`);
    } else if (action === 'expand') {
      if (expanded.has(id)) expanded.delete(id); else expanded.add(id);
    } else if (action === 'expand-all') {
      groupsForExpansion().forEach(key => expanded.add(key));
    } else if (action === 'collapse-all') expanded.clear();
    else if (action === 'clear-selection') { state = { ...state, selected: [], focus: null }; chosenObject = null; }
    render();
  });
  function groupsForExpansion() { return allGroups().flatMap(g => [`m:${g.main}`, ...g.children.map(c => `s:${g.main}:${c.middle}`)]); }
  $('workspace').addEventListener('change', event => { if (event.target.id === 'matrix-family') { matrixFamily = event.target.value; render(); $('matrix-family').focus(); } });
  $('search').addEventListener('input', event => { state = { ...state, query: event.target.value }; render(); });
  $('filter').addEventListener('change', event => { state = { ...state, filter: event.target.value }; render(); });
  $('notation').addEventListener('change', event => { state = { ...state, notation: event.target.value, middle: null }; render(); });
  $('reset').addEventListener('click', () => { state = { ...state, query: '', filter: 'all', main: null, middle: null }; $('search').value = ''; $('filter').value = 'all'; render(); });
  $('tasks-toggle').addEventListener('click', () => { const open = $('tasks').hidden; $('tasks').hidden = !open; $('tasks-toggle').setAttribute('aria-expanded', String(open)); });
  $('theme').addEventListener('click', () => { const dark = document.documentElement.dataset.theme !== 'dark'; document.documentElement.dataset.theme = dark ? 'dark' : 'light'; $('theme').textContent = dark ? 'Hellmodus' : 'Dunkelmodus'; });
  try { const saved = localStorage.getItem('knxbench.ga-study.view.v1'); if (saved && Object.hasOwn(descriptions, saved)) state = m.switchView(state, saved); } catch { /* In-memory fallback works without browser storage. */ }
  render();
})();
