# Controls and separators

UI-02 of the [UI feedback set](../goals/ui-feedback-2026-10-18.md) (owner
requests 5, 6 and 7, 18 October 2026). Presentation only: no API, schema,
stored-data or protocol change. Later UI packages (File submenus, topology)
build on these rules instead of styling their own buttons.

## One owner

`apps/knx-web/src/styles.css`, block **Controls**, is the only place that
decides how a button looks. Screens choose a variant by class; they do not
repaint a button with their own `background`/`border`/`color` triple.

| Variant | Class | Use |
| --- | --- | --- |
| Secondary | none (default), or `secondary-action` | Every ordinary action, Cancel, Back, "Show …" |
| Primary | `primary-action` | The one action a surface exists for: Create, Save, Apply, Export, Import, Confirm |
| Quiet | `quiet-action` (+ `icon-action` for a square icon button) | Toolbar and inline actions that should not draw a box until hovered |
| Destructive | `danger-action` | Delete, Discard, Replace, Reset: anything that destroys or throws work away |

A button is secondary unless it says otherwise. Before UI-02 every unclassed
button was accent-filled, so a device page showed eight equally loud actions and
the inspector's Delete looked like the primary action. Destructive actions now
read as warnings (error ink and edge, error-tinted hover), never as primaries.

The toolbar's quiet hover/press colours skip its `primary-action` Save. Before
UI-02 that hover repainted Save in a pale accent tint under its white (Porcelain)
or dark (Graphite) label, losing the primary look exactly when pointed at.

Three feature editors keep their own module-scoped secondary look for now
(`features/rtf`, `features/notes`, `features/parameter-editors`
`editor.module.css`): bordered, surface-filled buttons without the shared
hover/press feedback. Moving them onto the shared variants is open.

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
  below the 24px floor); 13px label, weight 500 (primary 600).
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

Table-column handles reuse the same separator paint; their interaction and
coverage are specified in [Table column widths](table-column-widths.md).

## Value source labels

The routine **Source: device instance / Quelle: Geräteinstanz** badge is no
longer shown in the communication-object DPT and description headings
(`ValueProvenance`, routine layer `Instance`). Every other layer (application
program, application reference, inferred value, project edit, unknown
verbatim identifiers) keeps its heading badge. The layer stays in the model,
the API DTOs (`dpt_layer`, `description_layer`), import/export and storage;
the DPT field's opened **technical reference** names it there instead. Each
layer is shown in exactly one place, so the reference never repeats a heading
badge.

## Tests

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
