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
  Instances of the same table kind share stored preferences, including nested
  history field tables and tables remounted by virtualisation. Measured floors
  belong to each mount; differing content can produce different rendered widths.
  Other table kinds are isolated.
- All width calculations and saved preferences use CSS layout pixels at every
  application scale (80–150%). Token and flex-icon DOMRect widths, measured
  header announcements and pointer deltas are divided by the root
  `--app-ui-scale`, using the same finite-positive fallback as `Overlay`.
  Computed-style insets, col/style widths and client/scroll dimensions already
  use layout pixels. Keyboard steps stay 8/40 layout px; ARIA current/text values
  announce rounded layout widths. A 60 visual px pointer movement changes the
  preference by `60 / scale` layout px before integer preference rounding;
  rendered movement can still be constrained by the fitting boundaries below.
  Changing the root style invalidates token/icon caches through the existing
  ancestor observer and refreshes geometry even when normalized floors stay
  unchanged. Stored widths do not change units when application scale changes.
  This unit split assumes Chromium's standardized CSS `zoom` (DOMRect and
  pointer coordinates zoomed, `client*`/`scroll*` and computed lengths not). It
  is verified in system Chromium only; WebKitGTK/Tauri behaviour under `zoom`
  is unverified ([KL-130](../limitations/KL-130-ZOOM-application-zoom.md#update-18-october-2026-resizable-tables-under-application-zoom)).
- Each column declares a default and a lower minimum preference in layout pixels.
  The stored-preference maximum is 1200 px
  unless declared otherwise. Stored values must be finite numbers in range;
  invalid values revert individually to defaults. Unknown columns are ignored,
  new columns use defaults and reordered columns keep their identity. An invalid
  root shape or corrupt JSON silently uses defaults. Edits merge valid stored keys, including columns temporarily absent in another
  view; absent keys outside 24–12000 px or with nonnumeric values are pruned.
  A table reset writes only the current schema defaults. Storage denial retains
  live session operation; reload persistence requires working browser storage.
- Pointer drag captures the pointer and clamps the changed preference. Other pointer
  IDs are ignored. Drag previews update local state once per animation frame;
  pointer-up, cancellation, lost capture and unmount persist and broadcast once,
  including the last unpainted pointer move. Arrow/Home/End/Enter keys are ignored
  during a pointer drag. Escape restores the starting width without persistence. Keyboard steps persist immediately.
  Resizing stops click, double-click, drag-start and handled key propagation,
  so header sorting and row selection are not activated.
- The handle is a focusable vertical `role="separator"`, named by its header,
  exposing `aria-valuenow/min/max`, pixel-valued `aria-valuetext` and a
  reference to keyboard help. The current value announces the rounded measured
  header width, including fitting and surplus distribution; the saved preference
  stays internal. Left/Right adjusts that preference by 8 px; Shift adjusts it
  by 40 px; Home/End chooses its minimum/maximum; Enter restores that column's
  default. Double-click also restores the default. Single-instance tables have a localized
  **Reset column widths** action named after their label, caption or localized
  table kind. Repeated field tables in comparison/history details and object-flag
  tables across a flow inspector's connections, and address/tree/matrix tables
  repeated across installations omit per-instance reset buttons;
  the owning surface renders one reset above the list, with the localized
  table kind in both its visible label and accessible name. Comparison/history
  renders this owner only if the report contains field changes (project info,
  installation info or any entity, including nested entries). A table reset
  restores shared defaults and clears the measured session floors of every mounted instance with that table ID in the current window.
  The existing width-change event carries this reset notification; listeners are
  removed on unmount, with no global content cache or reset registry.
  A viewport is a focusable region only
  when explicitly labelled or overflowing.
- The handle reuses the [UI-02 separator paint](controls-and-separators.md):
  a border-colour hairline at rest and accent line/band on hover, focus and
  held drag. It introduces no animation or theme-pack tokens. Reset uses the
  shared quiet button class. All five themes and OS reduced motion are covered.
- Every adjustable column uses the same measured word floor. Header and cell
  text tokens are measured in hidden spans using their computed typography,
  including nested bold/small/monospace text. Padding, borders, nested control
  margins/minima and adjacent sort icons contribute to the floor; ignored
  fixed-layout cell `min-width` declarations do not. The widest
  requirement is rounded up and capped at 320 px per column. The header target
  footprint remains at least 48 px (or the smaller declared preference minimum).
  Spanning group headings/editors do not belong to a single column and are
  excluded; mounted hidden object rows participate so expanding a group does
  not first compress its words. No per-table DPT/number fit constants remain.
- Token measurements are cached by text and typography per mounted table.
  MutationRecord targets invalidate only affected native cells; unchanged cells
  reuse their measurements without full-cell HTML serialization or text walks.
  Added cells are measured; moved rows retain their cached measurements. Relevant
  row/section style changes invalidate their descendant cells. A separate,
  attribute-only observer watches the table and its mounted ancestor chain for
  class/style and appearance changes, clearing this table's presentation cache.
  Style relevance uses a property-level comparison of old/current declarations:
  only `font*`, `letter-spacing`, `word-spacing`, `text-transform`, `line-height`
  and custom properties (`--*`) invalidate. Class and data-theme/density/presentation
  changes also invalidate. Ancestor box geometry (padding, margin, width/height,
  min/max sizes, positioning, transforms and scroll properties) does not invalidate
  words, including virtual-list spacer padding during scrolling. This comparison
  does not read computed styles. Direct cell/row inset edits still use the targeted
  content observer. In-place `colspan` changes trigger column remapping.
  Both observers disconnect on unmount; neither watches a document subtree.
  Resize-handle changes,
  hidden-row toggles and the monitor's arrival-background class do not invalidate
  words. Aggregating cached floors still visits mounted cells; this is not table
  virtualization. Theme/root appearance changes and font loading invalidate the
  measurement cache. Measurement spans are inserted together before geometry is
  read and then removed. Hidden panes defer measurement until their viewport has
  a width; mounted hidden rows in a visible pane still participate. Width edits
  and pane resizes do not remeasure tokens per frame.
- Measured column floors only grow during a mounted table session, including
  through filtering, content replacement and appearance changes. Longer words
  may increase a floor. **Reset column widths** clears that mount's maxima and
  measures its current content again in all mounted instances of that table ID;
  remounting also starts anew. Per-column
  preference resets do not clear the session's measured floors. Sort buttons
  always reserve a one-em indicator slot, including when unsorted.
- The global header/body rule uses `--knx-cell-padding`: compact inline padding
  is 12 px and comfortable is 14 px; theme packs can override the shorthand.
  More-specific surface rules retain their existing spacing, including project
  diff (0.1 rem block), name recipe (6 px block) and flow inspector cells.
  These actual insets contribute to the measured floors. Sortable
  header buttons and inline device/flow links avoid duplicate horizontal padding.
  Grouped object controls retain their nested tree indentation.
  The thirteen-column device table (System added by WP-20) uses 12 px compact
  EN/DE headings, including Location/Ort, Topology/Topologie and Prog.; full
  sort names remain accessible and appear in title tooltips, including
  separator context.
- Declared default budgets are devices 1180 px (the sum of the thirteen declared
  column defaults; 1100 px before WP-20 added the 80 px System default),
  communication objects 800/804 px and monitor 1024 px. Each default baseline
  is raised to its measured floor when necessary, then compressed
  proportionally toward that floor to fit
  the actual pane, including navigation/inspector space. A pane narrower than
  the sum of floors scrolls within its table. The table fills surplus space.
  User resizing applies `stored preference - declared default` pixels to each
  fitted baseline, with a final header-footprint clamp. Thus the DPT **stored
  preference** minimum remains 100 px; Home can render below 100 px or below
  the measured floor in a fitted pane. Rendered widths can also exceed the
  preference when floors or surplus space require it. The automatic baseline
  never shrinks below its measured floor; explicit user shrinking can split
  words. Flow current-values fixture floors can exceed its narrower inspector pane. Widening beyond
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
The 320 px cap deliberately permits character-level wrapping for pathological
tokens wider than that budget. Word safety applies to the mounted single-column
text considered above, not arbitrary spanning editors or every possible imported
project. Synthetic browser proof covers the labels and widths listed below;
wide-screen fitting is content- and pane-dependent, even for fixtures.

Production Chromium checks use compact Porcelain, synthetic data and navigation
present. The independent test oracle copies computed typography/insets and
control styles into isolated auto-layout, min-content single-cell tables. It
retains asymmetric horizontal border, margin and padding longhands, removes
component widths/colgroups and resize noise, preserves complete punctuated tokens, and includes mounted hidden rows and retained grouped maxima.
Oracle DOMRect measurements are independently converted to layout pixels at the
application root scale. It does not shrink the production viewport or read its columns as a minimum.

| Window | Surface | EN intrinsic / pane | DE intrinsic / pane |
| --- | --- | --- | --- |
| 1440 px | Devices | 1005 / 780 px: local scroll | 1042 / 780 px: local scroll |
| 1440 px | Grouped objects | 599 / 738 px: fits | 683 / 738 px: fits |
| 1440 px | Flat objects after grouped view | 690 / 738 px: fits | 800 / 738 px: local scroll |
| 1920 px | Devices | 1005 / 1260 px: fits | 1042 / 1260 px: fits |
| 1920 px | Grouped objects | 599 / 1218 px: fits | 683 / 1218 px: fits |
| 1920 px | Flat objects after grouped view | 690 / 1218 px: fits | 800 / 1218 px: fits |

Intrinsic and production token floors are different measurements. Native
min-content includes a 44 px checkbox footprint while the declared text-free
selection column renders at 32 px; shared inline branches also differ by up to
8 px from separately summed token insets. Tests bound per-column differences
(12 px for selection, 8 px elsewhere, plus one Chromium geometry subpixel
`1 / 64 / scale` for zoom rounding) and additionally assert reviewed rendered
fixture totals within 2 px: devices EN 1000 / DE 1037 px (919 / 955 px before
WP-20 added the System column, re-measured 19 October 2026); German flat objects
800 px. Device text columns alone require 961 / 998 px (previously 880 / 917;
DE 999 px while the fixtures still omitted `system`),
independently exceeding the 780 px pane. These fixture bounds reject inflated floors; they are not a
proof of globally optimal layout or arbitrary imported-project fit.

The flat-view measurements include the grouped mount's retained number floor.
The EN/DE monitor fixture fits at both widths. Flow current-values now has a
319 px intrinsic budget in its 340 px inspector pane and explicitly must fit
at both widths; its flag table requires 176 px and also fits. The earlier
373 px flow overflow came from the shared padding override, which incorrectly
replaced its specialized spacing. Restoring the CSS cascade removes that
artificial overflow. Device/German-flat 1440 px cases still scroll locally
with intact words; meaningful fields remain available.

The oracle has browser bounds for a monospace token, changed padding/font,
a nested control minimum, punctuation, the 320 px cap and column-width
independence. A behavioral negative control inflates a rendered column, requires
the assertion to reject it, then restores and rechecks it. This test helper is
bounded to the listed fixtures/styles; it is not a general CSS layout engine.
History instances share preferences but independently retain their own maxima;
stacked tables with different content can therefore have different widths,
including after a shared reset remeasures their different current content or
one instance remounts. No global content cache is kept.

## Remaining boundary

Observer ownership follows the ancestor chain captured at mount. Shipped React
panes keep mounted tables in that chain. Moving a live table into a different
DOM ancestry without remounting is unsupported; remount after such a move.
Dynamically replacing stylesheets, or using width/height container queries to
change typography, is outside this invalidation proof. Relevant class/style changes on
the captured chain, root appearance changes and font-load completion are covered.

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
| Application scale 0.8/1.5: layout floors and flex icons (fresh mount per scale, so retained session floors cannot mask an unnormalized measurement), root-style observer firing, geometry refresh without a resize event, measured ARIA, pointer deltas and unchanged keyboard steps | `src/ResizableTable.test.tsx`: scale stores pointer deltas; root scale mutations remeasure tokens, flex icons and announcements; a table first mounted at scale 0.8/1.5 measures layout-pixel floors; a root scale change re-reads header geometry without a resize event |
| Application scale 0.8/1.5: independent layout-pixel oracle, intact device words, real 60 visual px drag, keyboard, column/table reset and reload at 100% | `e2e/table-column-widths.e2e.ts`: scale layout-pixel fit, intact words, visual pointer tracking, keyboard, reset and restore |
| Each zoom path is load-bearing on its own (manual negative controls, 18 October 2026, not automated) | Removing the scale division from only the word floors fails the independent-oracle comparison (10 px and 22 px over the allowance); only the pointer delta fails the stored drag width (160 instead of 175/140); only the header measurement fails the announced width (14 px and 35 px off). Each file restored byte-exact. |
| Overlay stacking with background inert removed solely for synchronous hit testing | `e2e/table-column-widths.e2e.ts`: overlay paints above device headers (a negative control that raised the table viewport above the overlay was run once during UISET-05; it is not automated) |
| Persist/remount, types/ranges, corrupt JSON, changed/reordered columns, multiple instances, reset, window events, storage denial | `src/useTableColumnWidths.test.tsx` |
| Measured header versus styled cell floors, 320 px cap, cache reuse, targeted mutation invalidation in 1,000 rows, session maxima through filtering/reset/remount, hidden-pane deferral | `src/ResizableTable.test.tsx`: fits to the widest header or cell token; caps pathological tokens and caches measurement; invalidates only a changed cell; keeps session floors after filtering; defers hidden-pane word measurement |
| Colspan remapping; table class/style, viewport/pane inherited typography and padding invalidation; bounded ownership/cleanup and geometry-only cache reuse | `e2e/table-column-widths.e2e.ts`: colspan mutation, inherited typography, inherited padding; `src/ResizableTable.test.tsx`: bounded ancestor subscriptions, ancestor pane width and height; ancestor geometry noise and presentation edits/removals |
| Independent min-content oracle waits for fonts requested by the mounted table (layout flush, current `document.fonts.ready`); Devices measurements, also after a reload, start only after the product catalogue answered (`openResolvedList`, `waitForCatalogue`; held-catalogue regression); delayed-font regression, reviewed overflow totals, oracle bounds and inflated-column rejection/restoration; specialized block padding | `e2e/table-column-widths.e2e.ts`: intrinsic oracle bounds, independent fit assertion, specialized project diff |
| Accurate compact EN/DE location, topology and programming headings preserve full sort names and tooltips | `src/DevicesWorkspace.test.tsx`: `%s compact headings retain full sort names and tooltips`; `%s compact location, topology and programming labels keep their meaning` |
| Grouped/flat measured DPT floor in a 300 px pane, local table overflow, stored 100 px minimum, exact rendered delta and reset | `src/ComObjectTable.test.tsx`: `%s measures its DPT fit floor while preserving the stored minimum and rendered delta` |
| EN/DE grouped and flat object cell/header words intact at 360 px, actual local scrolling; status intact at 640 px | `e2e/table-column-widths.e2e.ts`: objects keep every word intact at 360 and status at 640 (long DE DPT/master text, EN function and undetermined status) |
| DE `Beleuchtungsstärke (9.004)` and decoded value intact at 400 px, actual local scrolling and no page overflow | `e2e/group-address-views.e2e.ts`: German address tree preserves the long DPT label; `e2e/monitor-control.e2e.ts`: German monitor decoded DPT words stay intact |
| EN/DE DPT/master-text words remain on one line per token at 360/1440 px; document overflow remains bounded | Unchanged `e2e/device-editor-layout.e2e.ts`: evaluated channel evidence stays keyboard-accessible |
| Measured ARIA pixels, keyboard/Shift/bounds, pointer capture and IDs, no sorting/drag/selection, cancel/capture loss, unmount commit, keys ignored during pointer drag, caller-defined trailing widths, double-click and table reset | `src/ResizableTable.test.tsx` |
| Every converted surface (including both object views and three fixed matrix columns) mounts with separators aligned to the declared columns and each column shrinks with ArrowLeft | Parameterized `column smoke: <table-id> exposes <count> resize separators` tests using `tableColumnSmoke.fixture.ts` |
| Actual production comparison exposes operable separators | `src/ComparisonTable.test.tsx`: offers keyboard resizing and reset on the production comparison table |
| Real device and comparison drag/keyboard, sort isolation, reset, reload, local narrow overflow, sticky headers, header/row alignment, measured fit or local scroll in EN/DE at 1440/1920, 24 px targets, zero API writes; shrinking/redistribution with local scroll at 1440, isolated headers under overlays, narrow German sort indicator and measured announcements | `e2e/table-column-widths.e2e.ts`: five EN/DE theme/width cases plus focused regression cases |
| Nested virtual history tables share widths, retain alignment at end of window, remount/reload/reset, bounded DOM and narrow scrolling | `e2e/table-column-widths.e2e.ts`: virtualised history field tables share widths, align through remounts and persist |
| One owner reset for repeated comparison fields and flow flags; reset clears every mounted field table's preferences and session floors; double-click preserves floors; devices keep their own reset | `src/ProjectDiffDetails.test.tsx`: one shared field reset; `src/TelegramFlowView.test.tsx`: `%s inspector owns one reset`; `e2e/table-column-widths.e2e.ts`: comparison owns exactly one field reset |
| Virtual history scrolling does not remeasure retained field tables (fixture-only measurement-pass counter, no timing threshold); asymmetric horizontal insets survive oracle cloning | `e2e/table-column-widths.e2e.ts`: scrolling virtualised history; intrinsic oracle preserves asymmetric borders |
| Added/removed-only reports omit the owner; info, installation, entity and recursive nested field changes retain one; owner labels visibly match EN/DE accessible names while per-table labels stay short | `src/ProjectDiffDetails.test.tsx`: omits the reset, one reset exists; `src/ResizableTable.test.tsx`: owner reset visibly names; `src/TelegramFlowView.test.tsx`: inspector owns one reset |
| Multiple installations share one address-table reset in tree/columns/matrix views; atlas has no reset; a single installation retains its per-table control | `src/StructureWorkspace.test.tsx`: `%s address tables across installations have one owner reset` |

Run the focused production browser selection with
`npx playwright test --config playwright.table-columns.config.ts`; it builds
fresh app and fixture pages before serving them. This includes token-driven
compact/comfortable/custom padding, constant sort footprints and the EN/DE
1440/1920 word-safe fit-budget checks above. The repository web gate runs it as
the `production-table-columns` stage, and `tsconfig.table-columns-fixtures.json`
typechecks its spec, fixtures, oracle and Vite config. The same spec also runs
in the default fixture lane. Device editor layout continues to use the unchanged
`playwright.issue09.config.ts` regression, which is outside the web gate.

Fixtures are fictional and intercept all API requests. No hardware, project
writes or external compatibility claims are part of this proof.
