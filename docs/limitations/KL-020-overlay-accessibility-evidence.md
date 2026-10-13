# Overlay accessibility evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-020)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 20. Command palette and search share overlay CSS and an accessibility gap — partially resolved

**Resolved (2026-09-12, T31)** — for the shell and the keyboard defect;
not for accessibility conformance in general, which is not a thing this
entry can ever claim closed by fiat. `apps/knx-web/src/Overlay.tsx` is
now the one component behind
`.search-overlay`/`.search-panel`, and all four former hand-rolled
copies — `Search.tsx`, `CommandPalette.tsx`, `CatalogBrowser.tsx` (T2)
and `SettingsPanel.tsx` (T27) — render it instead of their own overlay
divs. It owns `role="dialog"`/`aria-modal="true"` on the panel,
backdrop-click and single-`keydown`-on-the-panel `Escape` dismissal
(replacing `SettingsPanel.tsx`'s old `window` listener outright),
initial focus (`initialFocusRef`, else the first focusable descendant,
else the panel itself via `tabIndex={-1}`), a `Tab`/`Shift+Tab` focus
trap, and focus restoration to whatever had focus before the dialog
opened. `overlayShell.test.ts` enforces the "one shell" half
structurally: it scans every `.tsx` file under `apps/knx-web/src` with
`node:fs` and fails, naming the offender, if the literal `search-overlay`
appears anywhere outside `Overlay.tsx` — so a fifth hand-rolled copy
cannot slip in by copy-paste the way the second, third and fourth did.

The keyboard defect that made this a user-facing bug rather than a
cosmetic one is also fixed: `CatalogBrowser.tsx`'s result rows, `<li
onClick>` with no `tabIndex`, no key handler and no role, gained
`ArrowDown`/`ArrowUp` highlight movement (stopping, not wrapping, at the
ends, matching `Search.tsx`) and `Enter`-to-pick, matching what clicking
a row already did — pre-filling the device-name field, not creating the
device, which still needs the name field's own `Enter` or the Create
button. A keyboard-only user can now reach and choose a catalog item;
previously they could not reach the list at all.

Listbox semantics are now applied uniformly across all three
list-bearing overlays: each text input is `role="combobox"` with
`aria-expanded`/`aria-controls`/`aria-activedescendant`; each `<ul>` is
`role="listbox"`; each row is `role="option"` with `aria-selected` and a
stable id — so `CommandPalette.tsx`'s `aria-disabled="true"` now sits on
a row that carries a role for it to qualify, and the highlighted row in
every list is announced via `aria-activedescendant` rather than not at
all. `Search.tsx`'s kind groups became `role="group"`/`aria-label`
wrappers around `role="presentation"` `<ul>`s, with the visible
`.search-group-label` marked `aria-hidden="true"` since the group's
`aria-label` already says the same thing.

**Keyboard alpha follow-up (2026-10-02, delivered at 2e57f8e5).** Search, Command Palette
and Catalog Browser now scroll the active option with nearest-edge alignment
without moving combobox focus. Catalog keyboard highlight remains distinct
from a picked product and never creates a device by itself. The shared shell
leases background `inert`/`aria-hidden` state per document, restores prior
attribute values, excludes newly added background content and handles nested
or out-of-order modal closes. Initial/Tab-wrap filtering skips controls below
hidden/inert/aria-hidden ancestors. Final close restores the original connected,
non-inert focus target. List logic still stays outside `Overlay.tsx`.

HelpTip keeps a permanent local, visually clipped description and paints a
separate decorative body portal. Fixed placement escapes clipped/transformed
ancestors, bounds the popup to the viewport at application zoom, repositions
on resize/captured scroll and cleans up listeners. Closed painted bubbles no
longer extend the document at their old trigger coordinates.

Focused tests and 17 fully intercepted Chromium cases cover list visibility,
modal focus/background accessibility-tree exclusion, nested close, and tooltip
geometry/description in DE/EN at 360/640/1440 px and 100/150% zoom. Eighteen
behavioral controls are rejected with exact source restoration. All twelve
candidate/integrated gates pass: Web 1,357, Chromium 52, Rust 2,890 / zero failed /
163 ignored, 576-source freeze. Exact source ref/tree/all 28 artifacts were read
back after publication; package log records failed predecessors separately.

**Still open.** No whole-application focus-visible styling audit across every
theme, full Tauri/native interaction acceptance, or actual screenreader run.
Chromium DOM/accessibility-tree evidence does not prove what NVDA, JAWS, Orca
or VoiceOver announces. No WCAG conformance, universal virtual-cursor guarantee
or alpha-release exception is claimed. Design:
`docs/superpowers/specs/2026-09-12-modal-overlay-shell-design.md`.

**U8 update (2026-09-29).** The shared shell now offers opt-in viewport-bounded
pointer and keyboard resizing. Settings and Debug report use it; the other
overlays retain their existing sizes. Focus containment, Escape and backdrop
dismissal are covered by regression tests, including a pointer drag that ends
on the backdrop. Headless Chromium exercised the Settings layout, arrow-key
resize and internal scroll at 640 px, including 150% application zoom; the
zoomed dialog's rendered dimensions must be converted back to layout pixels
before a keyboard step. Native WebKitGTK sizing and real screen reader
announcements remain unverified; this is not an accessibility audit.

**Originally.** Four components shared `styles.css`'s
`.search-overlay`/`.search-panel` shape with no shared component behind
it, three of them duplicating the whole modal shape by hand (overlay
div, click-outside `stopPropagation` panel, autofocused input, `Escape`
handling); no result list anywhere carried `role="listbox"`/`role="option"`;
`CommandPalette.tsx`'s disabled rows carried `aria-disabled="true"` with
nothing backing it; and `CatalogBrowser.tsx`'s rows had no keyboard path
into the list at all.
<!-- END retained catalogue record -->
