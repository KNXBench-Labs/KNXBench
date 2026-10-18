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
| Address object matrix — `GroupAddressMatrix.tsx` | no; bounded window | no | no | **not converted**: project-object columns lack a stable presentation-only identity; see boundary below |
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
- Each column declares a default and minimum in pixels. The maximum is 1200 px
  unless declared otherwise. Stored values must be finite numbers in range;
  invalid values revert individually to defaults. Unknown columns are ignored,
  new columns use defaults and reordered columns keep their identity. An invalid
  root shape or corrupt JSON silently uses defaults. The next edit/reset writes
  a fresh validated record, dropping unknown columns. Storage denial retains
  live session operation; reload persistence requires working browser storage.
- Pointer drag captures the pointer and clamps the changed column. Other pointer
  IDs are ignored; pointer-up, cancellation and lost capture end the drag.
  Resizing stops click, double-click, drag-start and handled key propagation,
  so header sorting and row selection are not activated.
- The handle is a focusable vertical `role="separator"`, named by its header,
  exposing `aria-valuenow/min/max`. Left/Right changes by 8 px; Shift changes
  by 40 px; Home/End chooses minimum/maximum; Enter restores that column's
  default. Double-click also restores the default. Each table has a localized
  **Reset column widths** action restoring all its defaults.
- The handle reuses the [UI-02 separator paint](controls-and-separators.md):
  a border-colour hairline at rest and accent line/band on hover, focus and
  held drag. It introduces no animation or theme-pack tokens. Reset uses the
  shared quiet button class. All five themes and OS reduced motion are covered.
- Width overflow stays in the table viewport. Header/body share actual columns;
  device/object sticky headers retain their vertical viewport. Diff row
  measurements remain owned by the existing ResizeObserver/windowing code.

## Remaining boundary

17 of 18 native table surfaces are converted. **The address/object matrix is
not converted.** Its selectable object columns use project device/object keys.
Persisting those as preference keys would violate this package's prohibition on
project content in keys, while using the current ordinal would attach a width
to a different object after selection changes. A future matrix-specific UX
needs an explicit policy (for example, one common object-column width). This
package neither invents that policy nor claims complete table coverage.

The read-only flow flag property tables now have column headings (Field/Value)
so their two resize handles have accessible names; captions and flag values
are preserved. No manufacturer parameter-grid code is changed.

## Focused verification

| Requirement | Test |
| --- | --- |
| Persist/remount, types/ranges, corrupt JSON, changed/reordered columns, multiple instances, reset, window events, storage denial | `src/useTableColumnWidths.test.tsx` (8 tests) |
| ARIA, keyboard/Shift/bounds, pointer capture and IDs, no sorting/drag/selection, cancel/capture loss, double-click and table reset | `src/ResizableTable.test.tsx` (5 tests) |
| Actual production comparison exposes operable separators | `src/ComparisonTable.test.tsx`: offers keyboard resizing and reset on the production comparison table |
| Real device and comparison drag/keyboard, sort isolation, reset, reload, local narrow overflow, sticky headers, header/row alignment, zero API writes | `e2e/table-column-widths.e2e.ts`: five EN/DE theme/width cases |
| Nested virtual history tables share widths, retain alignment at end of window, remount/reload/reset, bounded DOM and narrow scrolling | `e2e/table-column-widths.e2e.ts`: virtualised history field tables share widths, align through remounts and persist |

Fixtures are fictional and intercept all API requests. No hardware, project
writes or external compatibility claims are part of this proof.
