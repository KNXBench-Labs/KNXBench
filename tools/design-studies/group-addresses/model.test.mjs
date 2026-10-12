/** Regression contracts for the isolated design study's shared fictional data. */
import test from 'node:test';
import assert from 'node:assert/strict';
import './model.js';
const m = globalThis.GaStudy;

test('fictional dataset has 36 unique addresses including an unassigned range', () => {
  assert.equal(m.addresses.length, 36);
  assert.equal(new Set(m.addresses.map(a => a.id)).size, 36);
  assert.equal(new Set(m.addresses.map(a => a.raw)).size, 36);
  assert.ok(m.addresses.some(a => a.range === null));
  assert.ok(Object.isFrozen(m.addresses));
});
test('two and three level notation preserve the same raw address identity', () => {
  const a = m.addresses.find(a => a.id === 'light-kitchen-switch');
  assert.ok(a);
  assert.equal(m.formatAddress(a, 'three'), '1/0/1');
  assert.equal(m.formatAddress(a, 'two'), '1/1');
  assert.equal(m.formatAddress({ raw: 2048 + 2 * 256 + 4 }, 'two'), '1/516');
});
test('search accepts both notations independent of displayed notation', () => {
  for (const query of ['1/0/1', '1.0.1', '1/1', 'Küche']) {
    assert.ok(m.visibleAddresses({ ...m.createState(), query }).some(a => a.id === 'light-kitchen-switch'), query);
  }
});
test('DPT search retains dotted type notation rather than treating every dot as an address', () => {
  const result = m.visibleAddresses({ ...m.createState(), query: '9.001' });
  assert.equal(result.length, 6);
  assert.ok(result.every(a => a.dpts.includes('9.001')));
});
test('filters distinguish conflict, unknown DPT and confirmed absent links', () => {
  const all = m.createState();
  const conflicts = m.visibleAddresses({ ...all, filter: 'conflict' });
  const unknown = m.visibleAddresses({ ...all, filter: 'unknown' });
  const none = m.visibleAddresses({ ...all, filter: 'unlinked' });
  assert.ok(conflicts.length > 0);
  assert.ok(unknown.length > 0);
  assert.ok(none.length > 0);
  assert.ok(none.every(a => a.linksKnown && a.links.length === 0));
  assert.ok(!none.some(a => a.id === 'spare-unknown'));
});
test('two-level projection flattens only presentation and does not mutate source ranges', () => {
  const before = JSON.stringify(m.addresses);
  const three = m.buildGroups(m.addresses, 'three');
  const two = m.buildGroups(m.addresses, 'two');
  assert.ok(three.find(g => g.main === 1).children.length > 1);
  assert.equal(two.find(g => g.main === 1).children.length, 0);
  assert.equal(two.reduce((sum, g) => sum + g.addresses.length, 0), 36);
  assert.equal(JSON.stringify(m.addresses), before);
});
test('view switch retains common search scope and selected identity', () => {
  const original = { ...m.createState(), query: 'Küche', main: 1, selected: ['light-kitchen-switch'], focus: 'light-kitchen-switch' };
  for (const view of ['tree', 'columns', 'atlas', 'matrix']) {
    const changed = m.switchView(original, view);
    assert.equal(changed.view, view);
    assert.equal(changed.query, original.query);
    assert.equal(changed.main, original.main);
    assert.deepEqual(changed.selected, original.selected);
    assert.equal(changed.focus, original.focus);
  }
  assert.throws(() => m.switchView(original, 'made-up'), /view/i);
});
test('selection is additive on demand and never modifies the source objects', () => {
  const a = m.selectAddress(m.createState(), 'light-kitchen-switch', false);
  const b = m.selectAddress(a, 'light-living-switch', true);
  assert.deepEqual(b.selected, ['light-kitchen-switch', 'light-living-switch']);
  assert.equal(b.focus, 'light-living-switch');
  assert.deepEqual(m.selectAddress(b, 'light-kitchen-switch', true).selected, ['light-living-switch']);
  assert.throws(() => m.selectAddress(b, 'missing', false), /address/i);
});
test('matrix cells distinguish send receive both absent and unavailable', () => {
  const a = { linksKnown: true, links: [{ object: 'x', direction: 'send' }, { object: 'x', direction: 'receive' }] };
  assert.equal(m.connectionStatus(a, 'x'), 'both');
  assert.equal(m.connectionStatus(a, 'y'), 'none');
  assert.equal(m.connectionStatus({ linksKnown: false, links: [] }, 'y'), 'unknown');
  assert.equal(m.connectionStatus({ linksKnown: true, links: [{ object: 'x', direction: 'send' }] }, 'x'), 'send');
  assert.equal(m.connectionStatus({ linksKnown: true, links: [{ object: 'x', direction: 'receive' }] }, 'x'), 'receive');
});
