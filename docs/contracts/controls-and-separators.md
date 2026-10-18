# Controls and separators

UI-02 of the [UI feedback set](../goals/ui-feedback-2026-10-18.md) (owner
requests 5, 6 and 7, 18 October 2026). Presentation only: no API, schema,
stored-data or protocol change. Later UI packages (File submenus, topology)
build on these rules instead of styling their own buttons.

## Shared variants and their owner

`apps/knx-web/src/styles.css`, block **Controls**, owns the four button
variants and every state rule (rest, hover, press, focus, disabled, pending).
An ordinary action, a dialog answer or a form button takes its look from there
by class and does not repaint itself with its own `background`/`border`/`color`
triple. A small, listed set of surfaces is not an ordinary button and keeps a
local paint on purpose; see [Local paint](#local-paint-the-complete-opt-out-list).

| Variant | Class | Use |
| --- | --- | --- |
| Secondary | none (default), or `secondary-action` | Every ordinary action, Cancel, Back, "Show …" |
| Primary | `primary-action` | The one action a surface exists for: Create, Save, Apply, Export, Import, Confirm |
| Quiet | `quiet-action` (+ `icon-action` for a square icon button) | Toolbar and inline actions that should not draw a box until hovered |
| Destructive | `danger-action` | Delete, Discard, Replace, Reset: anything that destroys or throws work away |

`quiet-action`/`icon-action` are available for new toolbar-like surfaces; the
workbench toolbar family predates them and keeps its own quiet paint (listed
below), which reads the same at rest: no fill, no edge.

A button is secondary unless it says otherwise. Before UI-02 every unclassed
button was accent-filled, so a device page showed eight equally loud actions and
the inspector's Delete looked like the primary action. Destructive actions now
read as warnings (error ink and edge, error-tinted hover), never as primaries.

The toolbar's quiet hover/press colours skip its `primary-action` Save. Before
UI-02 that hover repainted Save in a pale accent tint under its white (Porcelain)
or dark (Graphite) label, losing the primary look exactly when pointed at.

The three feature editors (`features/rtf`, `features/notes`,
`features/parameter-editors` `editor.module.css`) set only layout (padding,
radius, minimum height) on their buttons; paint and states come from the shared
variants, so a destructive answer there reads as destructive too.

### Destructive mapping

A Cancel, Keep or Close answer is never `primary-action`. A confirmation that
deletes, discards, overwrites, clears or replaces data is always
`danger-action`:

| Surface | Destructive answer |
| --- | --- |
| New project wizard / device wizard | Discard (abandon entries), Discard changes and create |
| Unsaved changes (quit, open other project) | Quit without saving, Discard changes and open |
| Note draft guard | Discard draft and continue |
| Overwrite confirmation | Replace |
| Catalogue update | Restore backup, Delete backup |
| Theme pack manager | Replace and apply, Remove pack |
| Achievements | Reset |
| Project history | Restore, Delete selected version, Clear undo and redo |
| Notes need a decision (note-aware delete) | Delete and apply choices |
| Plug-in data adoption | Adopt plug-in data (replaces bytes; Undo does not reverse it) |
| Functions | Delete (inline confirmation) |
| Project notes editor | Discard draft; Confirm change when the reviewed change removes a note |
| Installation instructions editor | Apply change when it removes or resets the override; Discard draft and reload |
| Line-scan exclusions | Confirm removal (the armed second click) |
| Inspector, bulk selection toolbar | Delete |

Not mapped to destructive, on purpose: buttons that only open one of the
confirmations above (Restore version…, Delete version…, Remove…, Reset
achievements…); a note's Confirm change for create, edit or move; programming,
download and catalogue installation (their own consent dialogs or a backup
guard them, and they are the action the surface exists for); Rename names
(undoable); the single-link Unlink and wizard row × buttons (row-level,
undoable or not yet stored); the native `window.confirm` of a destructive CSV
import.

### Local paint: the complete opt-out list

These surfaces are not ordinary buttons and keep their own paint. Each sets its
look directly, so the shared hooks do not apply to it; the shared focus ring
and the 24px floor still do.

| Surface (selector) | File | Why it stays local |
| --- | --- | --- |
| Workbench toolbar, navigation and panel-control buttons, diagram nodes, `.table-select` | `styles.css` | Chrome rows: quiet at rest with an accent-tinted hover and colour-only press for full-width rows; the toolbar's Save keeps the primary paint |
| LCARS presentation of navigation and toolbar | `styles.css` | The LCARS theme's own segment shapes and colours |
| `.navigation-section-toggle`, `.parameter-group-toggle`, tree rows (`.tree-label`, `.tree-toggle`), `.explorer-tool` | `styles.css` | Disclosure headings and tree rows, not actions |
| Tabs: `.device-tabs`, `.companion-tabs`, `.bus-diagnostics-tabs`, `.help-panel-topics` | `styles.css` | Selected-tab state (accent ink/underline) |
| Table header sort buttons (`.devices-table th`, `.com-table thead`) | `styles.css` | Header cells |
| Segmented and pressed toggles: `.com-table-views`, `.onboarding-language`, `.project-diff-status-filter`, `.flow-toolbar` (pressed state only) | `styles.css` | Their rest look is the shared secondary; only the pressed or selected segment is accent-filled locally |
| Link-like text buttons: `.import-diagnostic-link`, `.log-entry-open`, `.dashboard-row dd > button`, `.dashboard-finding-link`, `.building-overview-button`, `.address-overview-button`, `.companion-open`, `.project-wizard-step-link`, `.log-panel-import-scope-clear` | `styles.css`, `Dashboard.css` | Read as text links inside prose or tables |
| Selection lists and cards: `.welcome-card`, `.bus-discovery-option`, `.fs-picker-list`, `.project-history-versions`, `.parameter-workspace-nav`, `.comparison-results`, Group address workspace rows, columns, atlas tiles and matrix cells (`groupAddressWorkspace.css`), the notes list | `styles.css`, `projectHistory.css`, `parameterWorkspace.css`, `deviceComparisons.css`, `groupAddressWorkspace.css`, `features/notes/editor.module.css` | Full-width rows or tiles with a selected state |
| Small chips: `.help-tip-trigger`, `.toast` buttons, `.autosave-countdown` button, `.overlay-resize-key`, `.project-diff-toggle`, `.import-evidence-notice`, `.ga-range-add`, `.project-wizard-remove`, `.comparison-tag` | `styles.css`, `ImportDiagnosticsNotice.css`, `groupAddressWorkspace.css`, `deviceComparisons.css` | Glyph or chip inside another component, inheriting its ink |

Removed in the review round because they only re-created the shared secondary
look: `.log-export-actions button`, `.ga-new-in-range-actions button`,
`.about-dialog .release-update-panel button`, the rest paint of
`.com-table-views button` and `.onboarding-language button`, the
`.project-history-panel button` override (which had also hidden the history
confirmations' destructive paint), and the paint of the three feature editors.

Existing layout classes stay (`dialog-action` in the unsaved-changes rows, the
explorer, tab and table families); they now carry only layout. The former
dialog-local modifiers `dialog-action--secondary/--danger/--primary` are gone.

## Tokens

Component-layer tokens (ADR-0022; listed in `COMPONENT_LAYER_TOKENS`, never set
by a theme):

| Token | Compact | Comfortable | Owner |
| --- | --- | --- | --- |
| `--knx-control-height` | 32px | 40px | density (unchanged) |
| `--knx-control-padding-inline` | 12px | 16px | density |
| `--knx-control-gap` | 6px | 8px | density |
| `--knx-control-press-scale` | 0.97 | 0.97 | motion |

Colours are derived from theme tokens only (`--knx-surface`, `--knx-border`,
`--knx-accent`, `--knx-on-accent`, `--knx-error-color`, `--knx-gradient-primary`,
`--knx-shadow-raised`, `--knx-shadow-hover`), so the four built-in themes and the
bundled CRT pack need no new token and theme-pack format v1 is unchanged.
Primary keeps painting with the theme's `--knx-gradient-primary`.

Variants set private hooks that the shared state rules read: `--button-bg`,
`--button-ink`, `--button-edge`, `--button-hover-bg`, `--button-hover-edge`,
`--button-press-bg`, `--button-veil` (primary's hover/press overlay over its
gradient) and `--button-press-scale`. A new variant sets these hooks and
nothing else.

## States

- **Rest:** height is at least `--knx-control-height` (32px compact, so never
  below the 24px floor); 13px label, weight 500 (primary 600). Controls whose
  glyph is deliberately small keep a 24px interactive box around it: the help
  tip's 18px ring sits inside a 24×24 button, the toast's × button and the
  explorer's import-finding links are at least 24px high (and wide).
- **Hover:** colour only (fill and edge; primary adds `--knx-shadow-hover`).
  No hover rule may scale or move a button (issue #14).
- **Pressed:** a stronger fill plus `scale(var(--knx-control-press-scale))`.
  The scale is a transform, so the button's layout box and its neighbours do
  not move. Full-width rows (navigation entries, explorer rows, table selects,
  file-menu and picker entries, tabs, section toggles) set
  `--button-press-scale: 1` and answer with colour only. A surface that pins
  `transform` itself keeps it: the press rule has zero specificity.
- **Keyboard focus:** a 2px accent outline, 2px offset; outlines take no space.
- **Disabled:** surface fill, muted ink, 60% opacity, `not-allowed` cursor;
  hover and press change nothing.
- **Pending:** `aria-busy="true"` shows the `progress` cursor; a busy disabled
  button keeps 80% opacity so "working" does not read as "unavailable". The
  New project Create and catalogue-update Apply buttons set it.
- **Motion:** transitions run for `min(--knx-feedback-duration,
  --knx-transition-duration)` inside the `prefers-reduced-motion:
  no-preference` guard. Motion level Off and OS reduced motion remove the
  transition and the press scale; the colour feedback stays, instantly.

## Navigation separators

- The left column's two horizontal splitters draw a 1px hairline in
  `--knx-border` at rest, centred in their unchanged 6px hit area. Hover,
  keyboard focus and a held drag (`:active`, kept by pointer capture) turn it
  into a 2px accent rule over a faint accent band.
- Without a project there are no splitters; consecutive navigation sections
  carry a 1px `--knx-border` top rule instead. At narrow widths, where the
  splitters are hidden, the section boundary keeps that rule.
- The rules that used to sit under a section heading (explorer and diagnostics
  `border-top`) were moved to the section boundary.
- The vertical pane resizers keep the pane's own border as their resting line
  and show the same 2px accent rule when highlighted.
- Pointer and keyboard resizing (16px arrow steps, Home/End), ARIA values,
  collapse state and the first resize right after unfolding are unchanged
  and covered by `e2e/controls-and-separators.e2e.ts` and
  `e2e/workbench-splitters.e2e.ts`.

## Value source labels

The routine **Source: device instance / Quelle: Geräteinstanz** badge is no
longer shown in the communication-object DPT and description headings
(`ValueProvenance`, routine layer `Instance`). Every other layer (application
program, application reference, inferred value, project edit, unknown
verbatim identifiers) keeps its heading badge. The layer stays in the model,
the API DTOs (`dpt_layer`, `description_layer`), import/export and storage;
the field's own opened **technical reference** names it there instead: the DPT
field's reference, and for the description a **Technical reference** disclosure
that appears under the description only when its layer is routine. No layer
becomes unreachable, and each layer is shown in exactly one place, so a
reference never repeats a heading badge.

## Tests

- `e2e/controls-theme-matrix.e2e.ts`: Porcelain, Graphite, Cupertino, LCARS
  and CRT at 600 and 1440 px, one page load each: dialog Cancel/Keep secondary,
  Create primary, Discard destructive; toolbar Commands and Undo icon quiet,
  Save primary; inspector Delete destructive with the form's 16px spacing to the
  next field; keyboard focus ring on dialog, toolbar, table-row and inspector
  buttons; every checked control, the toast's ×, the import-finding links and
  the help tip at least 24×24 px; splitter hairlines (wide) or section rules
  (narrow) visible at rest. Roles are read from computed paint against the
  active theme's tokens (`e2e/control-roles.ts`).
- `e2e/project-history.e2e.ts`: restore, delete and clear confirmations paint
  destructive, Cancel secondary (Porcelain, Graphite).
- `e2e/note-disposition.e2e.ts`: Delete and apply choices destructive, Cancel
  secondary.
- Unit: the destructive mapping in `NoteDispositionDialog`, `PluginDataDialog`,
  `FunctionsPanel`, `LineScanExclusionsEditor`, `GroupAddressTable` (bulk
  delete), `ProjectNotesEditor` and `InstructionEditor` tests; description and
  DPT Instance layers independently in `DptSelection.test.tsx`.

- `e2e/controls-and-separators.e2e.ts`: separators visible at rest and
  highlighted on hover/focus in Porcelain, Graphite, Cupertino, LCARS and
  CRT; first resize after unfolding; secondary/primary/destructive rest,
  hover, press, focus, disabled and pending at 1440 and 400 px in EN and DE
  without layout shift; no press animation with motion off or reduced motion;
  instance label absent in headings, present in the technical reference,
  API data unchanged.
- `src/DptSelection.test.tsx`: the same label rule at component level, and
  every non-routine layer still shown.
- `src/UnsavedChangesDialog.test.tsx`: the three answers use the shared
  variants.
- `e2e/dialogs-overview.e2e.ts`: its contrast helper now judges a gradient
  fill by every colour stop (worst case) instead of skipping it, because a
  primary answer paints `--knx-gradient-primary`; the 4.5:1 bar is unchanged.
