# T31 — A shared modal overlay shell, and the accessibility pass its four consumers never got

- **Date:** 2026-09-12
- **Status:** design, ready for implementation
- **Closes:** **D9** in [GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md);
  [KNOWN_LIMITATIONS.md §20](../../KNOWN_LIMITATIONS.md#20-command-palette-and-search-share-overlay-css-and-an-accessibility-gap-unaddressed)
- **Scope:** `apps/knx-web` only. No Rust file is touched.

## Why now

§20 has a lift trigger, written when the overlay CSS had two consumers:
*"a third overlay is added"*. On 2026-09-12 it was discovered that the
trigger had already fired — twice, unnoticed. Four components render
`.search-overlay` wrapping `.search-panel` today:

| Component | Overlay | Panel | Click-outside | `Escape` | Result list |
| --- | --- | --- | --- | --- | --- |
| `Search.tsx` | yes | yes | `onClick` + `stopPropagation` | on its `<input>` | grouped, arrow-key nav |
| `CommandPalette.tsx` | yes | yes | `onClick` + `stopPropagation` | on its `<input>` | flat, arrow-key nav, skips disabled |
| `CatalogBrowser.tsx` | yes | yes | `onClick` + `stopPropagation` | on two `<input>`s | flat, **mouse only** |
| `SettingsPanel.tsx` | yes | yes | `onClick` + `stopPropagation` | `window` listener | none |

Three of the four duplicate the whole modal shape by hand. The fourth
duplicates the two divs and invents a different `Escape` mechanism because
it has no input to hang one off.

The accessibility side is worse than "inconsistent". `CatalogBrowser.tsx`'s
result rows are `<li onClick>` with no `tabIndex`, no key handler and no
role: **there is no keyboard path into that list at all**, so a keyboard-only
user cannot create a device from the catalog. That is a user-facing
correctness defect, not a maintainability complaint. No overlay is a
`role="dialog"`; none traps focus; none restores focus on close; no result
list is a `role="listbox"`; `CommandPalette.tsx`'s `aria-disabled="true"`
sits on rows that carry no role for it to qualify.

## What gets built

### 1. `Overlay.tsx` — the shell

One component owning exactly the behaviour that is the same in all four
consumers, and nothing that differs between them.

```tsx
export default function Overlay(props: {
  labelledBy?: string;        // id of the element naming the dialog
  label?: string;             // or a literal name, when there is no heading
  className?: string;         // extra classes for the panel (e.g. "settings-panel")
  initialFocusRef?: RefObject<HTMLElement | null>;
  onClose: () => void;
  children: ReactNode;
}): JSX.Element;
```

It owns:

- **Structure** — `div.search-overlay` > `div.search-panel`, with
  `role="dialog"`, `aria-modal="true"` and `aria-labelledby`/`aria-label`
  on the panel.
- **Backdrop dismissal** — click on the overlay closes; click inside the
  panel does not. Implemented as today: `onClick` on the overlay,
  `stopPropagation` on the panel.
- **`Escape`** — one `keydown` listener on the panel element (not on
  `window`, not on an input), so the key works wherever focus sits inside
  the dialog and is not stolen from the page when the dialog is closed.
- **Initial focus** — on mount, focus `initialFocusRef.current` if the
  consumer named one, else the first focusable descendant of the panel,
  else the panel itself (which carries `tabIndex={-1}` for exactly that
  case). This subsumes every consumer's `autoFocus`, which is removed from
  the consumers. The explicit ref is not decoration: `CatalogBrowser.tsx`'s
  first focusable descendant is the *"Install product database"* file
  input, not its search field, so "first focusable" alone would silently
  move that dialog's focus.
- **Focus trap** — `Tab`/`Shift+Tab` cycle within the panel's focusable
  descendants. Focus cannot leave a modal dialog by keyboard.
- **Focus restore** — on unmount, return focus to the element that had it
  before the dialog opened.

It does **not** own: list rendering, highlight state, arrow-key traversal,
`Enter` activation, or anything about the payload. Those genuinely differ
per consumer (§20's cause (a) is correct on that point and stays correct);
the shell exists so that the parts which *are* the same stop being four
copies.

**Focusable-descendant query.** One exported constant, used by both the
initial-focus and trap logic:

```ts
const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
```

Elements are filtered to those currently rendered (`offsetParent !== null`
is unreliable under jsdom, so filter on `disabled`/`hidden` attributes and
`aria-hidden="true"` only — the query above already excludes disabled
controls).

### 2. `useListboxNav` — no

Rejected. Three list behaviours differ in their traversal rules (grouped
vs flat, skip-disabled vs not, stop-at-end vs not) and a hook abstracting
all three would take more configuration than it saves. The lists stay
hand-written; only their *semantics* are unified, which is section 3.

### 3. Listbox semantics, applied uniformly

Every result list in the three list-bearing overlays becomes a real
listbox:

- `<ul className="search-results" role="listbox" id="<overlay>-results">`
- each row: `role="option"`, `aria-selected={isHighlighted}`, a stable
  `id` (`<overlay>-option-<n>`), and `aria-disabled` where the concept
  exists (Command Palette only).
- the overlay's text input gets `role="combobox"`, `aria-controls` naming
  the list, `aria-expanded`, and `aria-activedescendant` pointing at the
  highlighted row's id — which is how a screen reader is told which row
  the arrow keys are on without focus ever leaving the input.
- `Search.tsx`'s grouping stays visible. Its outer `<li className="search-group">`
  wrappers cannot sit inside a `role="listbox"` as bare list items, so each
  group becomes `role="group"` with `aria-label` set to the group's label,
  and the inner `<ul>` loses its implicit role (`role="presentation"`),
  leaving the `role="option"` rows as the listbox's children. The visible
  `.search-group-label` div is marked `aria-hidden="true"` because the
  group's `aria-label` already announces it.

### 4. `CatalogBrowser.tsx` gets a keyboard path

The gap that makes this slice user-facing rather than cosmetic. The
catalog list gains the same highlight-and-arrow-keys behaviour
`Search.tsx` has:

- `highlight` state, reset to `0` whenever the item list changes.
- `ArrowDown`/`ArrowUp` on the search input move the highlight, stopping
  at the ends (matching `Search.tsx`, not wrapping).
- `Enter` on the search input picks the highlighted item — *picks*, i.e.
  does what clicking the row does (selects it and fills the name field).
  It does **not** create the device; creation stays behind the name field
  and the Create button, which is what the existing comment in that file
  means by "this modal's own text field already needs Enter for create".
  That comment described a field that does not exist yet at that point:
  the name input only renders once an item is selected. The search input's
  `Enter` is free, and this spec claims it.
- Rows keep `onClick` and gain `role="option"`/`aria-selected` per
  section 3. They do not get `tabIndex` — the input keeps focus and drives
  the list through `aria-activedescendant`, exactly as Search and the
  palette do. Adding both a roving tabindex and a combobox pattern would
  be two competing keyboard models in one widget.

### 5. What the shell does not fix

Stated plainly so the limitation text after this slice is honest:

- **No scroll-into-view.** A highlight moved past the panel's visible area
  by arrow keys still does not scroll into view in any overlay. `Overlay`
  has no business knowing about lists, and none of the three list owners
  gains it here. This is the same defect as §19 in a different widget.
- **No inert background.** Content behind the overlay is not marked
  `inert`/`aria-hidden`, so a screen reader's virtual cursor (as opposed
  to Tab) can still reach it. A focus trap stops `Tab`, not browse mode.
- **No focus-visible styling pass.** The trap makes focus reachable; it
  does not make every focused control visibly focused in every theme.
- **jsdom is not a browser.** The tests below assert focus moves, trap
  cycling and ARIA wiring in jsdom. Real assistive-technology behaviour
  has never been verified against a screen reader, and this slice does not
  claim it.

## Testing

New `Overlay.test.tsx`:

1. renders its children inside `.search-overlay` > `.search-panel`, with
   `role="dialog"` and `aria-modal="true"`.
2. `aria-labelledby` is applied when `labelledBy` is given; `aria-label`
   when `label` is.
3. clicking the backdrop calls `onClose`; clicking the panel does not.
4. `Escape` inside the panel calls `onClose`.
5. focus lands on `initialFocusRef` when given, and on the first
   focusable descendant when not.
6. `Tab` from the last focusable descendant wraps to the first;
   `Shift+Tab` from the first wraps to the last.
7. focus returns to the previously focused element on unmount.
8. `Escape` after unmount does not call `onClose` (listener removed).

Extensions to existing suites:

- `CatalogBrowser.test.tsx`: `ArrowDown` then `Enter` on the search input
  selects the second catalog item (name field pre-filled from it) without
  any mouse event; `ArrowUp` at the top stays at the top.
- `SettingsPanel.test.tsx`: still closes on `Escape` (now via the shell,
  not a `window` listener) and on a backdrop click.
- A shared-shell regression guard, `overlayShell.test.ts`: reads every
  `.tsx` file under `apps/knx-web/src` with `node:fs` and asserts that
  `Overlay.tsx` is the only one containing the literal `search-overlay`
  — i.e. that a fifth overlay cannot be added by copy-paste without the
  guard failing, naming the offending file. This mirrors
  `motionGuard.test.ts`'s approach and exists for the same reason: §20's
  lift trigger was prose, prose does not fail a build, and it was missed
  for two consumers running.

**Note for the implementer:** Vitest does not process CSS, so
`import src from "./Search.tsx?raw"` is not a way to read source. Use
`node:fs` and the `node-builtins.d.ts` declarations that already exist —
and run `npx tsc -p apps/knx-web/tsconfig.json --noEmit`, because
`npm run test` does not build the application.

## Non-goals

- No visual redesign. The overlay and panel CSS are unchanged; this is a
  behaviour and semantics change, and a diff that alters `.search-overlay`
  or `.search-panel` declarations has exceeded its brief.
- No new dependency. No focus-trap library, no headless-UI package. The
  behaviour above is ~80 lines.
- No `<dialog>` element. Native `<dialog>` + `showModal()` would give the
  trap and the backdrop for free, but it also brings its own top-layer
  stacking, its own `::backdrop` styling model and a jsdom implementation
  that does not support `showModal`. Not worth re-testing four working
  overlays against, today.
