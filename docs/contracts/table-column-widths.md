# Table column widths

UI-05 of the [UI feedback set](../goals/ui-feedback-2026-10-18.md).
Presentation only; widths never enter project data, commands, exports or the
server settings record. The existing `settingsStore` synchronizes with the
server and therefore cannot own these browser-local preferences.

## Coverage checklist (inventory taken before conversion)

Native data tables in `apps/knx-web/src`; custom role tables/grids were also
searched. V = virtualised, S = sortable, D = draggable header/rows.
No table below has draggable headers or rows (explorer trees own drag/drop).

| Surface / file | V | S | D | Coverage |
| --- | --- | --- | --- | --- |
| Devices — `DevicesWorkspace.tsx` | no | yes | no | converted |
| Communication objects — `ComObjectTable.tsx` | no | yes | no | converted |
| Address flat/tree table — `GroupAddressTreeTable.tsx` | no; bounded window | no | no | converted |
| Address object matrix — `GroupAddressMatrix.tsx` | no; bounded window | no | no | fixed select/address/name columns converted; per-object columns stay fixed, see boundary below |
| Address members — `GroupAddressLinks.tsx` | no | no | no | converted |
| Building device list — `StructureWorkspace.tsx` | no | no | no | converted |
| Monitor telegrams — `BusMonitorPanel.tsx` | no; bounded capture | no | no | converted |
| Flow current values — `TelegramFlowView.tsx` | no | no | no | converted |
| Flow object flags — `TelegramFlowView.tsx` | no | no | no | converted |
| Decoded comparison — `ComparisonTable.tsx` | no | no | no | converted |
| History/diff field changes — `ProjectDiffDetails.tsx` | nested in measured virtual list | no | no | converted |
| Readiness report — `DeviceInspectionPanel.tsx` | no | no | no | converted |
| Changed bytes report — `DeviceInspectionPanel.tsx` | no | no | no | converted |
| Line scan results — `LineScanPanel.tsx` | no | no | no | converted |
| Download unwritten ranges — `DeviceDownloadPanel.tsx` | no | no | no | converted |
| Download segments — `DeviceDownloadPanel.tsx` | no | no | no | converted |
| Download blocks — `DeviceDownloadPanel.tsx` | no | no | no | converted |
| Name recipe preview — `features/nameRecipe/NameRecipeDialog.tsx` | no | no | no | converted |

Not tables: `CatalogBrowser` (listbox/cards), `ProjectHistoryDialog` (version
list and definition list), `BusActivityHistory` (ordered activity list),
`SourceReportPanel`, `CatalogInstallReport`, `LegacyInstallReport` (forms/lists),
project-diff entity summaries (lists), address atlas/columns and topology/building
cards. Explorer trees, forms and manufacturer-declared parameter grids
(ADR-0137) are outside scope and remain unchanged.


## Width and interaction contract

- `ResizableTable.tsx` decorates native column headers with the single
  `ColumnResizeHandle`, applies an explicit `colgroup` and fixed table layout,
  and owns a local scroll viewport. Existing scroll classes/region labels move
  onto that viewport, avoiding nested horizontal scroll controls. Body elements, row keys, sorting,
  editors and commands remain owned by their original components.
- `useTableColumnWidths.ts` stores a numeric column-ID record at
  `knxbench:table-column-widths:v1:<table-id>`. Both table and column IDs are
  source-defined presentation identifiers. No translated labels, names,
  addresses, entity IDs, project identity or cell values enter the key/value.
  Instances of the same table kind share widths, including nested history field
  tables and tables remounted by virtualisation. Other table kinds are isolated.
- Each column declares a default and a lower minimum preference in pixels.
  The maximum is 1200 px
  unless declared otherwise. Stored values must be finite numbers in range;
  invalid values revert individually to defaults. Unknown columns are ignored,
  new columns use defaults and reordered columns keep their identity. An invalid
  root shape or corrupt JSON silently uses defaults. Edits merge valid stored keys, including columns temporarily absent in another
  view; absent keys outside 24–12000 px or with nonnumeric values are pruned.
  A table reset writes only the current schema defaults. Storage denial retains
  live session operation; reload persistence requires working browser storage.
- Pointer drag captures the pointer and clamps the changed column. Other pointer
  IDs are ignored. Drag previews update local state once per animation frame;
  pointer-up, cancellation, lost capture and unmount persist and broadcast once,
  including the last unpainted pointer move. Arrow/Home/End keys are ignored
  during a pointer drag. Escape restores the starting width without persistence. Keyboard steps persist immediately.
  Resizing stops click, double-click, drag-start and handled key propagation,
  so header sorting and row selection are not activated.
- The handle is a focusable vertical `role="separator"`, named by its header,
  exposing `aria-valuenow/min/max`, pixel-valued `aria-valuetext` and a
  reference to keyboard help. The current value announces the rounded measured
  header width, including fitting and surplus distribution; the saved preference
  stays internal. Left/Right adjusts that preference by 8 px; Shift adjusts it
  by 40 px; Home/End chooses its minimum/maximum; Enter restores that column's
  default. Double-click also restores the default. Each table has a localized
  **Reset column widths** action named after its label, caption or localized
  table kind, restoring all its defaults. A viewport is a focusable region only
  when explicitly labelled or overflowing.
- The handle reuses the [UI-02 separator paint](controls-and-separators.md):
  a border-colour hairline at rest and accent line/band on hover, focus and
  held drag. It introduces no animation or theme-pack tokens. Reset uses the
  shared quiet button class. All five themes and OS reduced motion are covered.
- Default widths follow the previous layout budgets (devices 1100 px,
  communication objects 800/804 px, monitor 1024 px); the table fills any surplus
  container space. Defaults compress proportionally to the actual available
  pane width, including navigation and inspector space; user pixel adjustments
  apply on top of that baseline. A 48 px header footprint (32 px for defaults of
  48 px) retains separate sort and resize targets; the communication-object number column reserves 120 px
  for its existing indented disclosure at the default; explicit resizing can
  shrink it below that footprint. Very narrow panes below that combined minimum scroll
  locally. Flow inspector defaults fit its narrower pane. Widening beyond
  the available space scrolls locally. Download blocks use the adapter viewport
  for both axes, capped at 320 px height; the table retains native table display.
- Resize targets are 24 px wide, centred on interior column boundaries; the
  final target sits inside the viewport edge to avoid creating overflow. Header
  stacking stays local to the isolated table scroll container, below menus and
  overlays; padding keeps sort buttons clickable. Fitted headings show at
  most three label lines, with sort indicators outside the clamped label. They
  retain the full accessible label and a title tooltip so
  sticky headers cannot obscure the first data rows. Development builds warn
  if native heading and declared column counts diverge.
- Width overflow stays in the table viewport. Header/body share actual columns;
  device/object sticky headers retain their vertical viewport. Diff row
  measurements remain owned by the existing ResizeObserver/windowing code.

## Known behaviour

Pane fitting uses ResizeObserver updates. While dragging a pane splitter, the
fitted layout can settle one frame after the pane and briefly show a horizontal
scrollbar. This accepted cosmetic behaviour does not change saved preferences.
Rendered movement can differ from a preference step when the table distributes
surplus space or reaches its header footprint.

## Remaining boundary

All 18 native table surfaces use the adapter. The address/object matrix's three
fixed columns (select, address, name) have stable presentation IDs and persist
widths. The matrix passes its 180 px object width into the adapter; fitting is
disabled when fixed trailing columns are present. Its per-object columns retain 180 px preferred widths
without resize controls and do not produce preference keys. Their selectable project device/object keys cannot become
browser preference IDs; ordinal persistence would follow a different object
after selection changes. A future matrix-specific policy (for example, one
common object-column width) remains open in OPEN_WORK. Complete per-column
coverage is not claimed.

The read-only flow flag property tables now have column headings (Flag/Value)
so their two resize handles have accessible names; captions and flag values
are preserved. No manufacturer parameter-grid code is changed.

## Focused verification

| Requirement | Test |
| --- | --- |
| Persist/remount, types/ranges, corrupt JSON, changed/reordered columns, multiple instances, reset, window events, storage denial | `src/useTableColumnWidths.test.tsx` |
| Measured ARIA pixels, keyboard/Shift/bounds, pointer capture and IDs, no sorting/drag/selection, cancel/capture loss, unmount commit, keys ignored during pointer drag, caller-defined trailing widths, double-click and table reset | `src/ResizableTable.test.tsx` |
| Every converted surface (including both object views and three fixed matrix columns) mounts with separators aligned to the declared columns and each column shrinks with ArrowLeft | Parameterized `column smoke: <table-id> exposes <count> resize separators` tests using `tableColumnSmoke.fixture.ts` |
| Actual production comparison exposes operable separators | `src/ComparisonTable.test.tsx`: offers keyboard resizing and reset on the production comparison table |
| Real device and comparison drag/keyboard, sort isolation, reset, reload, local narrow overflow, sticky headers, header/row alignment, default fit at 1440/1920, 24 px targets, zero API writes; shrinking/redistribution at 1440, isolated headers under overlays, narrow German sort indicator and measured announcements | `e2e/table-column-widths.e2e.ts`: five EN/DE theme/width cases plus focused regression cases |
| Nested virtual history tables share widths, retain alignment at end of window, remount/reload/reset, bounded DOM and narrow scrolling | `e2e/table-column-widths.e2e.ts`: virtualised history field tables share widths, align through remounts and persist |

Fixtures are fictional and intercept all API requests. No hardware, project
writes or external compatibility claims are part of this proof.
