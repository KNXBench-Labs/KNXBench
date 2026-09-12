# 2026-09-12 — T31: a shared modal overlay shell, and the accessibility pass its four consumers never got

Architecture log for the T31 cycle, branch `t31-overlay-shell`, four code
commits from `ea54b0c` (`bdba7cb`..`3eff155`) plus this docs-reconciliation
commit — 11 files under `apps/knx-web/src`, 875 insertions, 225 deletions
before this task's docs-only commit.

Design: `docs/superpowers/specs/2026-09-12-modal-overlay-shell-design.md`.
Plan: `docs/superpowers/plans/2026-09-12-modal-overlay-shell.md`.

## What changed architecturally

One new shell component owning exactly the behaviour that was identical
across four hand-rolled overlays, one semantics pass applied uniformly to
every result list, one correctness fix for the one list that had no
keyboard path at all, and one regression guard turning a missed lift
trigger into something that fails a build instead of sitting in prose.

### 1. `Overlay.tsx` — the shell, not a fifth implementation

`apps/knx-web/src/Overlay.tsx` (task 1, `bdba7cb`, 94 lines) renders
`div.search-overlay > div.search-panel[role="dialog"][aria-modal="true"][tabIndex=-1]`
and owns exactly what was the same in all four consumers and nothing
that differed: backdrop-click dismissal (`onClick` + `stopPropagation`
on the panel, as every hand-rolled copy already did it), `Escape` via a
single `keydown` listener on the panel element — not `window`, not an
input, so the key works wherever focus sits inside the dialog and stops
working the instant it unmounts — initial focus (`initialFocusRef`, else
the first focusable descendant via the exported `FOCUSABLE_SELECTOR`,
else the panel itself), a hand-written `Tab`/`Shift+Tab` focus trap, and
focus restoration to whatever had it before the dialog opened. No new
dependency: no focus-trap library, no `<dialog>` element, per the
design's own non-goals.

The explicit-ref case is not decoration. `CatalogBrowser.tsx`'s first
focusable descendant is the "Install product database" file input, not
its search field — "first focusable" alone would have silently
misdirected that dialog's opening focus onto the wrong control every
time it opened.

Deliberately *not* owned by the shell: list rendering, highlight state,
arrow-key traversal, `Enter` activation. The design spec rejected a
`useListboxNav` hook outright — three list behaviours differ in their
traversal rules (grouped vs. flat, skip-disabled vs. not, stop-at-end vs.
not) and a hook covering all three would need more configuration than it
would save. The lists stay hand-written; only their *semantics* are
unified (§3 below).

### 2. All four consumers migrated, one deletion outright

Task 2 (`a0c6dbe`) moves `Search.tsx` and `CommandPalette.tsx` onto
`Overlay`; task 3 (`8d6220d`) moves `CatalogBrowser.tsx` and
`SettingsPanel.tsx`. Each dropped its own overlay/panel divs, its own
`Escape` handling, and its `autoFocus`. `SettingsPanel.tsx`'s `window`
`keydown` listener — the odd one out among the four, since three
`<select>`s gave it no single field to hang `Escape` off of — is deleted
outright rather than left dead, now that the shell provides the same
behaviour structurally.

### 3. Listbox semantics, applied uniformly

Every result list in the three list-bearing overlays becomes a real
listbox: the text input is `role="combobox"` with
`aria-expanded`/`aria-controls`/`aria-activedescendant`; the `<ul>` is
`role="listbox"`; each row is `role="option"` with `aria-selected` and a
stable id. `CommandPalette.tsx`'s `aria-disabled="true"`, which sat on
rows with no role to qualify it before this slice, now has one.
`Search.tsx`'s kind-grouped `<li className="search-group">` wrappers
cannot sit inside a `role="listbox"` as bare items, so each group became
`role="group"`/`aria-label`, the inner `<ul>` dropped to
`role="presentation"`, and the visible `.search-group-label` gained
`aria-hidden="true"` since the group's own `aria-label` already
announces the same text.

### 4. `CatalogBrowser.tsx` gets the keyboard path it never had

The one change in this slice that is a user-facing correctness fix, not
a maintainability one. Its result rows were `<li onClick>` with no
`tabIndex`, no key handler and no role: a keyboard-only user could not
reach the catalog list at all, meaning they could not create a device
from the catalog by keyboard, full stop. Task 3 adds `ArrowDown`/`ArrowUp`
highlight movement on the search input — stopping, not wrapping, at the
ends, matching `Search.tsx` — and `Enter`-to-pick: selects the
highlighted item and pre-fills the device-name field, exactly what
clicking the row already did. It deliberately does not create the
device; creation stays behind the name field's own `Enter` and the
Create button, since that field only renders once an item is selected.
Rows still carry no `tabIndex`: focus stays on the input, driving the
list through `aria-activedescendant`, the same combobox pattern
`Search.tsx`/`CommandPalette.tsx` already used — a roving tabindex
alongside it would be two competing keyboard models in one widget.

### 5. `overlayShell.test.ts` — the missed lift trigger becomes a test

`KNOWN_LIMITATIONS.md` §20 had said, since the CSS had two consumers,
that a shared shell should be reconsidered "when a third overlay is
added." That trigger fired twice — at `CatalogBrowser.tsx`, then at
`SettingsPanel.tsx` — and nobody noticed either time, because the
trigger lived in prose and prose does not fail a build. Task 4
(`3eff155`) adds a guard, modeled on `motionGuard.test.ts`'s approach:
it reads every non-test `.tsx` file under `apps/knx-web/src` with
`node:fs` and fails the suite, naming the offender, if the literal
`search-overlay` appears anywhere outside `Overlay.tsx`. Verified with
teeth, the same way `motionGuard.test.ts` was: the literal was injected
into a second file and the guard named it, then the injection was
reverted.

## What this slice deliberately does not do

The design spec's §5 non-goals, and they survive into
`KNOWN_LIMITATIONS.md` §20 rather than being quietly dropped:

- **No scroll-into-view.** A highlight moved past the panel's visible
  area by arrow keys still does not scroll into view, in any of the
  three lists. `Overlay.tsx` has no list knowledge to fix this with, and
  none of the three list owners gained it here — the same defect §19
  records for a different widget.
- **No inert background.** Content behind the overlay is not marked
  `inert`/`aria-hidden`. The focus trap stops `Tab` from leaving the
  dialog; it does not stop a screen reader's browse/virtual-cursor mode
  from still reaching content behind it.
- **No focus-visible styling pass.** The trap makes every control in the
  dialog keyboard-*reachable*; it says nothing about whether every
  reached control is *visibly* focused in every theme.
- **No screen-reader verification, anywhere.** `Overlay.test.tsx` and the
  extended `CatalogBrowser.test.tsx`/`SettingsPanel.test.tsx` run under
  jsdom, which asserts that focus moves, the trap cycles, and ARIA
  attributes point at the right elements. It says nothing about what
  NVDA, JAWS, Orca or VoiceOver actually announce. No conformance to
  WCAG or any other accessibility standard is claimed anywhere in this
  slice, and no audit of any kind has been performed.

No claim of ETS parity is made anywhere in this slice — ETS's own
dialogs are native Windows/Java widgets with their own accessibility
story, not a thing this slice's jsdom-verified custom-overlay work is
being compared against.

## Verification

Gates, foreground, one at a time, on this branch:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo run -p xtask -- check-layering`, `cargo deny
  check` — all unchanged by this slice; no Rust file was touched
  anywhere in it.
- `npm run test` (`apps/knx-web`, `vitest run`) — **235 passed across 25
  files** (up from 215/21 at the branch point, `ea54b0c`).
- `npx tsc -p apps/knx-web/tsconfig.json --noEmit` — clean.
- `npm run build` was **not** run in this task, by instruction — Vite's
  `outDir` clean step deletes the tracked
  `apps/knx-web/dist/.gitkeep`; `tsc --noEmit` above is the type gate.

Documentation reconciled in this same task (docs-only, no code touched):
`KNOWN_LIMITATIONS.md` §20 body rewritten to "resolved" (header line
left byte-identical — `GAP_ANALYSIS_ETS.md` links its anchor slug);
`GAP_ANALYSIS_ETS.md` row **D9** closed and a **T31** backlog entry added
(no existing tier fit, said so rather than inventing one); a dated entry
appended to `IMPLEMENTATION_STATUS.md` plus its `Last updated:` line
moved to 2026-09-12; `ROADMAP.md` checked and left untouched — nothing
there makes a claim this slice falsifies. Dated specs and plans under
`docs/superpowers/` — including this slice's own — were left untouched
as historical record, per project convention.
