# Group-address presentation studies

## Decision and scope — 2026-10-11

The tester reports that the central group-address list is difficult to navigate
and suggests expandable two-/three-level hierarchy. The owner explicitly wants
users to choose a presentation matching their workflow rather than enforcing a
single view. The owner accepted keeping the global Project Explorer stable and
limiting the presentation switch to the central group-address workspace, and
welcomed unconventional alternatives. The subsequent **yes authorizes isolated
clickable design studies**, not production integration or four shipped features.

The artifact is [the standalone group-address laboratory](../../tools/design-studies/group-addresses/README.md)
([openable HTML](../../tools/design-studies/group-addresses/index.html)).
It is separate from `apps/knx-web`, its dependency graph, server, domain and
project persistence. No production UI entry point or shared stylesheet changed.

## Four proposals

| Study | Interaction | Intended comparison | Principal risk |
| --- | --- | --- | --- |
| Hierarchical table | Expandable numeric groups plus native table | Structure and engineering details together | Long expanded trees; future sorting semantics |
| Column browser | One scrollable column per numeric level | Guided structural navigation | Width and cross-range comparisons |
| Address atlas | Semantic drill-down from labeled areas to address tiles | Spatial recognition of an address hierarchy | Hidden detail; does not establish improved usability |
| Connection matrix | Address rows against object columns, direction-aware cells | Modeled relationships | Width, size and evidence ambiguity |

All use the same immutable, original fictional dataset, stable IDs, shared
selection, search and Inspector. Choosing a view preserves common scope and
filters; selected addresses outside the filter are counted explicitly, not
silently cleared. Only the view preference is saved in a study-specific browser
key, with an in-memory fallback. There are no editing or bus actions.

## Representation and integrity boundaries

- The fixture's raw-address rendering matches the existing bit split in
  `crates/knx-core/src/address.rs`; 1/0/1 and 1/1 represent the same raw fixture
  address. Switching notation in this laboratory does not issue a project
  address-style mutation or rewrite stored example ranges.
- Numeric grouping is a **study projection with explicitly supplied fictional
  names**. It is not an importer or a contract for arbitrary real GroupRange
  hierarchies. A future production design must reconcile actual stored range
  structure, numeric address layout and Free-style notation without inventing
  missing labels or dropping unassigned addresses.
- DPT conflicts, unknown DPTs, confirmed absent links and unavailable link data
  have distinct states. Matrix `S`/`E`/`S/E` are modeled directional links, never
  evidence of measured sender/receiver behavior on a real bus.
- Atlas geometry is disposable presentation, not building geometry or physical
  topology. Its current zoom is drill-down, not a pan/gesture/animation engine.
- These proposals deliberately use fictional read-only objects, not generated
  production DTOs. No adapter compatibility or production feature admission is
  inferred from a successful standalone-page test.

## Verification and limitations

Nine Node model tests pass. The final private-profile Chromium direct-file
recipe executes 52 named assertions and captures 13 frames at desktop
1440×1000 and compact 400×900, including dark mode and both address notations.
The earlier HTTP recipe executes 50 assertions. Shared selection, scope,
filters, tree expansion, column traversal, atlas drill-down, matrix cell
meaning, unknown/absent data, preferred-view reload, no external requests and
source immutability are exercised. Native Enter/focus is checked for tree
expansion. Full screen-reader usability and all keyboard navigation are not
certified. Direct-file operation was actually exercised, not inferred from HTTP.

The initial model skeleton fails before implementation; the dotted-DPT search
case separately fails and passes after correction. Browser personal-profile
locking and a malformed CLI function are excluded tooling failures. Retained
source-bound receipts and screenshots are local evidence; self-review is not
independent acceptance. No app build/test, import/export, migration, vendor
package, performance or hardware result is claimed.

Wide matrix/table/column content scrolls internally on compact screens. The
Inspector moves below the workspace and remains reachable by page scrolling;
not every field fits within a single 400×900 viewport. No virtualization, large
project claim, writing workflow, localization beyond German, project persistence
or production address-style integration is implemented. Native desktop and
Firefox acceptance remain unexecuted.

## Next decision

Compare the four studies using their common tasks, then collect actual user
preferences and navigation friction. Decide which concepts to refine and how to
map the preserved real range structure before authorizing a separately scoped
production implementation. The stable Explorer and user-choice principle remain
settled; no ledger, roadmap or compatibility completion is promoted by this study.

## Owner planning decision — 2026-10-12

The owner likes the column browser, atlas and matrix, and accepts the tree as a
requested option. **Plan all four selectable presentations**, not a competition
with one surviving view. This updates the earlier open concept-selection step.
The owner asks for complexity assessment and planning, not immediate production
implementation, publication or release. The global Explorer remains stable;
preferred presentation is a user setting outside project data. A tree-table
initial default is still a proposal, not an owner-selected favorite.

### Assessed implementation basis

Read-only inspection compared the shared root with the locally available
`origin/main` at `f0de988b` (not a claim that this is the newest remote revision).
`GroupAddressNode`, `GroupAddressLinkNode`, `GroupRangeNode`, `DeviceNode` and
`groupAddressView.ts` match those upstream blobs. `GroupAddressTable.tsx` has
upstream changes; a future implementation must start from its then-current
integration base rather than transplanting the stale root. Scope, actual range
parents, effective/conflicting DPT information, reverse links with stable device
and object IDs, shared `useMultiSelection`/visible-order handling and Inspector
already exist. They are reusable implementation ingredients, not acceptance of
the new views. The fictional laboratory model is not production domain logic.

### Proposed small production architecture

Keep one group-address workspace controller with common query/filter, actual
installation/range scope and existing selection/command handlers. Derive a typed,
read-only presentation index from the generated projections; render it through
four focused components. Reuse current DPT/direction/notation helpers and the
existing Inspector and bulk actions. View-local expansion, column path, atlas
navigation and matrix column filter stay separate. Define explicit navigation
and visible-order semantics so switching views cannot select hidden addresses
through a stale Shift-click anchor. Keep preferences scoped to presentation;
project changes, deleted IDs and new sessions must invalidate stale view context.
No plugin framework, chart dependency or domain/store schema migration is
proposed. Any missing projection must be justified at its actual service owner.

### Relative effort (engineering estimate, not measured hours)

| Work package | Relative effort | Main production concern |
| --- | --- | --- |
| Shared workspace/index/controller | Medium | Scope, typed selectors, selection/focus, preference invalidation |
| Hierarchical table | Medium | Existing range hierarchy, unassigned data, expansion and visible ordering |
| Column browser | Low–medium after shared foundation | Actual variable-depth ranges, narrow layouts and keyboard navigation |
| Semantic address atlas | Medium | Stable drill-down, grouping/counts, navigation and accessible tile layout |
| Connection matrix | Medium–high | Real object identity, sparse lookup, complete column scope, two-axis navigation |
| Integrated hardening | Medium–high | Four-view state transitions, installed native/web shell, large inputs and regression tests |

These estimates apply to four navigation/inspection views sharing existing
editing through Inspector and command services. Direct matrix link writes,
spreadsheet-style inline editing in every view, animated geometric zoom/pan,
custom layouts and live traffic overlays are additional scope, not silently
included. Semantic atlas drill-down is sufficient to preserve the accepted
study's interaction concept; no persisted geometry is needed.

### Difficult contracts to resolve before coding

1. **Stored hierarchy versus address notation:** real `GroupRange.parent` and
   address membership are authoritative project data. Numeric two-/three-level
   grouping is a distinct optional projection, not a rewrite of imported ranges.
   Never invent names from numbers, flatten stored structure due to a notation
   toggle, or lose unassigned/dangling addresses. Include Free-style and nested,
   missing-parent/cyclic defensive cases in the presentation contract.
2. **Matrix column completeness:** reverse group-address links provide linked
   object IDs and directional metadata; `DeviceNode` alone provides an object
   count, not an all-object inventory. A matrix inferred only from those reverse
   links must say "linked objects" and cannot claim to include unlinked objects.
   To show all objects for selected devices, inspect/reuse the current canonical
   device-detail/service projection before proposing a new endpoint. Keep
   missing/unavailable data distinct from confirmed absence and retain unknown
   direction values. Do not import the fixture's independent `linksKnown` flag
   as an invented production DTO field or infer completeness from an empty list.
3. **Scale:** index links by address/object ID rather than searching all links
   for each cell. Bound matrix columns by explicit device/object scope and
   measure actual row/column rendering at realistic sizes before choosing
   virtualization. Such UI limits must be visible and never hide canonical data
   as if absent. No large-project performance claim follows from the 36-row lab.
4. **Input/accessibility:** define selection separately from focus, reconcile
   with existing delegated keyboard handlers, and choose native table/button
   semantics or a fully implemented grid/treegrid contract deliberately. W3C
   APG [treegrid](https://www.w3.org/WAI/ARIA/apg/patterns/treegrid/) and
   [grid](https://www.w3.org/WAI/ARIA/apg/patterns/grid/) specify keyboard/focus
   behavior; adding ARIA roles alone does not implement that behavior. These
   primary guidance sources were consulted on 2026-10-12, not used as proof of
   this application's accessibility.

### Proposed delivery sequence and acceptance

1. Freeze the actual integration base; agree and test hierarchy/object-scope
   semantics; build the shared typed index/controller without altering core
   rules, project storage or the global Explorer.
2. Integrate the tree-table and column browser through existing selection and
   editing owners, keeping current group-address actions available.
3. Integrate the semantic atlas using that same index, scope and Inspector.
4. Integrate the matrix with explicit column inventory scope and no direct
   cell writes initially; preserve modeled versus actual-hardware distinctions.
5. Exercise the final combination across all four views: navigation, selection,
   filters, native/web keyboard/focus, project switch/update/delete, EN/DE,
   theme, 2-/3-/Free-style, unassigned/malformed/conflicting/unknown data, narrow
   geometry and realistic large projects. Existing editing/undo/save must not
   regress. Add focused pure selector, component and real integrated browser
   tests; run owning build/quality gates on the actual integrated sources.

Acceptance means all four are freely selectable and share one correct project
state. It does not require identical control layouts, but no presentation may
silently change project data or lose a supported action. The build sequence is
incremental; it is not a proposal to drop any of the four. Implementation awaits
separate authorization and then-current shared-surface ownership checks.

