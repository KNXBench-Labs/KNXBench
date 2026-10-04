# 2026-10-04 — UX-01 web half: group address onto communication object

Agent: Claude, goal-ui.md owner session. Web lock taken for this package
(`6abf557b`).

## Decision

GAP_ANALYSIS_ETS B10 kept this gesture out because `LinkComObject` needs an
explicit direction and a hidden default would not be honest. The drop target
is therefore the object's link row, which already shows the direction choice
(default Send since the keyboard path exists). A drop links in exactly the
direction shown there; nothing is chosen out of sight. The keyboard path is
unchanged.

## Change

- `groupAddressDrag.ts`: MIME `application/x-knxbench-group-address-id`,
  `writeDraggedGroupAddress`, `carriesGroupAddress` (type only, for dragover),
  `readDraggedGroupAddress` (strict `^[1-9]\d*$`, safe integer).
- `ProjectExplorer.tsx`: group-address items are draggable and write the id.
- `Inspector.tsx` `NewGroupLinkRow`: dragover accepts the type and marks
  `data-drop-ready`; drop parses, checks the device's linkable list, then
  `linkTo(id)` shares the button's request path and direction.
- Messages en/de: `inspector.dropNotLinkable`.
- `e2e/installations-fixture.tsx`: `?workspace` renders the device workspace.

## Evidence

- RED: the payload module missing, 4 drop-target cases, 1 Explorer source
  case; the foreign-drag guard already held.
- Browser: the first e2e run showed no drop at all. Measured, not guessed: a
  minimal probe proved `dragTo` delivers dragstart/dragover in this headless
  Chromium, while the real scenario logged only `mousedown`. The target row
  sat below the 720 px viewport, and Chromium drops a pending drag when the
  page scrolls while the button is held. Fixed by a 1280x1200 viewport plus an
  in-viewport assertion; no product change was needed.
- Controls: both e2e cases fail against the previous Explorer/Inspector.
  5 mutants caught by name: linkable check skipped, row direction ignored,
  dragover accepting anything, payload accepting a leading zero, Explorer
  writing no payload.
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,155 passed / 0 failed / 177 ignored in 169 blocks with 0 skip markers, five repository gates (headers 469 ok, ceiling 157; anchors 442 ok; ledger 185 rows), tsc, Vitest 1,833/101 files, complete intercepted Chromium suite 108/108, whitespace; source frozen.

## Boundaries

Native WebKitGTK drag is not verified. Structural drag gestures stay
select-only. Offline only, no KNX or bus contact.
