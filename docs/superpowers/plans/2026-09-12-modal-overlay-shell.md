# T31 — implementation plan: shared modal overlay shell + accessibility pass

Spec: [`../specs/2026-09-12-modal-overlay-shell-design.md`](../specs/2026-09-12-modal-overlay-shell-design.md)

Branch `t31-overlay-shell`, worktree
`/mnt/daten-i/Sourcecode/KNXBench/.worktrees/t31-overlay-shell`.

## Global Constraints

1. **`apps/knx-web` only.** No Rust file, no `crates/`, no `apps/knx-cli`,
   no `apps/knx-server` source change. Docs under `docs/` and `.ai/` are
   in scope for the documentation task only.
2. **No visual redesign.** `styles.css`'s `.search-overlay`,
   `.search-panel`, `.search-results`, `.search-result`, `.search-empty`
   and `.search-group*` declarations stay byte-identical. If a task needs
   new CSS it must be additive and must sit inside an existing
   `@media (prefers-reduced-motion: no-preference)` block if it declares
   `transition:`/`animation:` — `motionGuard.test.ts` enforces this and
   must stay green.
3. **No new npm dependency.** No focus-trap library, no headless UI kit.
4. **Vitest does not process CSS or bundle `?raw`.** A test that needs to
   read source or stylesheet text uses `node:fs` plus the existing
   `apps/knx-web/src/node-builtins.d.ts` declarations. Never
   `import x from "./y?raw"` — it silently resolves to the empty string
   and produces a green test that checks nothing.
5. **`npm run test` does not build the application.** Every task ends with
   both `npx --prefix apps/knx-web tsc -p apps/knx-web/tsconfig.json --noEmit`
   and `npm --prefix apps/knx-web run test` green.
6. **Do not run `npm run build`.** Vite's `outDir` clean step deletes the
   tracked `apps/knx-web/dist/.gitkeep`. `tsc --noEmit` is the type gate.
7. **Never `git add -A`.** The worktree contains a deliberately untracked
   `OriginalData` symlink. Add named paths only.
8. **Never `git stash`.** The stash stack is shared across worktrees.
9. **Commit messages** are written in the voice of Marvin, the manically
   depressed robot from *The Hitchhiker's Guide to the Galaxy* — gloomy,
   world-weary, and technically accurate and complete. No
   `Co-Authored-By:` trailer of any kind (`CLAUDE.md` forbids it and
   overrides any session-level instruction to add one).
10. **Do not dispatch subagents.** Review arrives from the coordinator.

## Task 1 — `Overlay.tsx` and its tests

Create `apps/knx-web/src/Overlay.tsx`. Write it as follows; this is the
complete intended implementation, not a sketch.

```tsx
import { useEffect, useRef } from "react";
import type { KeyboardEvent, ReactNode, RefObject } from "react";

/**
 * The one modal overlay shell (T31, closing D9 / KNOWN_LIMITATIONS.md §20).
 *
 * Owns what every overlay in this application shares: the
 * `.search-overlay`/`.search-panel` structure, `role="dialog"`, dismissal
 * by backdrop click and by `Escape`, initial focus, a focus trap, and
 * focus restoration on close. It owns nothing about lists — highlight
 * state, arrow-key traversal and `Enter` activation genuinely differ
 * between the Search, Command Palette and Catalog Browser dialogs and
 * stay with them.
 */
export const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

function focusableIn(panel: HTMLElement): HTMLElement[] {
  return Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (el) => !el.hasAttribute("hidden") && el.getAttribute("aria-hidden") !== "true",
  );
}

export default function Overlay(props: {
  labelledBy?: string;
  label?: string;
  className?: string;
  initialFocusRef?: RefObject<HTMLElement | null>;
  onClose: () => void;
  children: ReactNode;
}) {
  const { labelledBy, label, className, initialFocusRef, onClose, children } = props;
  const panelRef = useRef<HTMLDivElement | null>(null);

  // Initial focus on mount, restoration on unmount. The dependency list is
  // deliberately empty: this runs once per open, and re-running it on a
  // prop change would yank focus out from under whatever the user is
  // typing into.
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const panel = panelRef.current;
    const target = initialFocusRef?.current ?? (panel ? (focusableIn(panel)[0] ?? panel) : null);
    target?.focus();
    return () => {
      previous?.focus?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
      return;
    }
    if (e.key !== "Tab") return;
    const panel = panelRef.current;
    if (!panel) return;
    const items = focusableIn(panel);
    if (items.length === 0) {
      // Nothing to move to, so Tab must not leave the dialog either.
      e.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement as HTMLElement | null;
    const inside = active !== null && panel.contains(active);
    if (e.shiftKey && (!inside || active === first)) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && (!inside || active === last)) {
      e.preventDefault();
      first.focus();
    }
  }

  return (
    <div className="search-overlay" onClick={onClose}>
      <div
        ref={panelRef}
        className={className ? `search-panel ${className}` : "search-panel"}
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        aria-label={label}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        {children}
      </div>
    </div>
  );
}
```

If the `eslint-disable` comment is unnecessary in this repository (there
is no ESLint config in `apps/knx-web` — check before keeping it), drop the
comment and keep the explanatory comment above it.

Create `apps/knx-web/src/Overlay.test.tsx` covering, in this order:

1. structure and roles: `.search-overlay` > `.search-panel`, panel has
   `role="dialog"` and `aria-modal="true"`, children render inside it.
2. `aria-labelledby` when `labelledBy` is passed; `aria-label` when
   `label` is passed.
3. a click on the backdrop calls `onClose`; a click inside the panel does
   not.
4. `Escape` on an element inside the panel calls `onClose`.
5. initial focus: with `initialFocusRef` pointing at the *second* input,
   that input has focus after mount; without the ref, the first input has
   focus.
6. focus trap: `Tab` fired on the last focusable moves focus to the first;
   `Shift+Tab` on the first moves focus to the last.
7. focus restore: a button outside the overlay is focused, the overlay
   mounts and takes focus, and after unmount that button has focus again.
8. after unmount, `Escape` fired on `document.body` does not call
   `onClose`.

Follow the testing idiom already used in `SettingsPanel.test.tsx`
(`@testing-library/react`, `fireEvent`, `describe`/`it`/`expect` from
`vitest`). Read that file first.

Commit. Nothing else changes in this task — the four existing overlays are
untouched and still work.

## Task 2 — migrate `Search.tsx` and `CommandPalette.tsx`

Both files get the same three changes.

**Structure.** Replace the hand-written
`<div className="search-overlay" onClick={onClose}><div className="search-panel" onClick={stopPropagation}>`
wrapper with `<Overlay label="…" onClose={onClose} initialFocusRef={inputRef}>`.
Labels: `"Search"` for `Search.tsx`, `"Command palette"` for
`CommandPalette.tsx`. Add `const inputRef = useRef<HTMLInputElement>(null);`
and `ref={inputRef}` on the text input; remove that input's `autoFocus`.

**`Escape`.** Delete the `if (e.key === "Escape") { onClose(); }` branch
from each component's `handleKeyDown` — the shell handles it now. The
`ArrowDown`/`ArrowUp`/`Enter` branches stay exactly as they are; their
traversal rules are not changing in this task or any other.

**Listbox semantics.** In `Search.tsx`:

- the input gets `role="combobox"`, `aria-expanded={ordered.length > 0}`,
  `aria-controls="search-results"`, and
  `aria-activedescendant={ordered[highlight] ? \`search-option-${highlight}\` : undefined}`.
- `<ul className="search-results">` becomes
  `<ul className="search-results" id="search-results" role="listbox">`.
- each group `<li className="search-group">` gains `role="group"` and
  `aria-label={KIND_LABELS[kind]}`; its inner `<ul>` gains
  `role="presentation"`; the `.search-group-label` div gains
  `aria-hidden="true"` (the group's `aria-label` already announces it).
- each result `<li>` gains `id={\`search-option-${position}\`}`,
  `role="option"` and `aria-selected={position === highlight}`.

In `CommandPalette.tsx`, the same with a flat list: input gets
`role="combobox"`, `aria-expanded={results.length > 0}`,
`aria-controls="palette-results"`,
`aria-activedescendant={results[highlight] ? \`palette-option-${highlight}\` : undefined}`;
`<ul>` gets `id="palette-results"` and `role="listbox"`; each row gets
`id={\`palette-option-${i}\`}`, `role="option"` and
`aria-selected={i === highlight}`, keeping its existing `aria-disabled`.

**Tests.** Existing suites that touch these components must stay green
(`App.test.tsx`, `commandRegistry.test.ts`, `searchMatch.test.ts` — check
for others). Add to the suite covering each component, creating a test
file if none exists for it:

- `Escape` closes the dialog (proving the shell's handler is wired, not
  just that the old one was deleted).
- the highlighted row carries `aria-selected="true"` and the input's
  `aria-activedescendant` names that row's `id`.
- arrow-key traversal still behaves as before: in `Search.tsx`,
  `ArrowDown` past the last result stays on the last; in
  `CommandPalette.tsx`, `ArrowDown` skips a disabled row.

Commit.

## Task 3 — migrate `CatalogBrowser.tsx` and `SettingsPanel.tsx`, and give the catalog list a keyboard path

**`SettingsPanel.tsx`** — the smaller half. Replace the overlay/panel
wrapper with
`<Overlay labelledBy="settings-panel-title" className="settings-panel" onClose={onClose}>`,
give the existing `<h2 className="settings-panel-title">` the matching
`id="settings-panel-title"`, and **delete the `useEffect` that adds the
`window` `keydown` listener** along with its now-unused `useEffect`
import. Rewrite the file's doc comment: it currently explains that the
component reuses the overlay shape by hand and "declines to widen" §20's
gap; it now uses the shell, and the comment should say what is true after
this slice.

**`CatalogBrowser.tsx`** — the substantive half.

- Wrap in `<Overlay label="Device catalog" onClose={onClose} initialFocusRef={searchRef}>`;
  add `const searchRef = useRef<HTMLInputElement>(null);`, put `ref={searchRef}`
  on the catalog search input, remove its `autoFocus`.
- Remove the `if (e.key === "Escape") onClose();` branches from **both**
  the search input and the device-name input — the shell owns `Escape`.
  The name input's `Enter` → `void create()` branch stays.
- Add `const [highlight, setHighlight] = useState(0);` and an effect
  resetting it to `0` whenever `items` changes.
- Give the search input an `onKeyDown` handler:
  `ArrowDown` moves the highlight to `Math.min(h + 1, items.length - 1)`,
  `ArrowUp` to `Math.max(h - 1, 0)` (both `preventDefault()`), and `Enter`
  calls `pick(items[highlight])` when that item exists. `Enter` **picks**
  — it selects the item and pre-fills the name field, exactly as clicking
  the row does. It does not create the device; creation stays behind the
  name field and the Create button.
- Listbox semantics as in task 2: input gets `role="combobox"`,
  `aria-expanded={items.length > 0}`, `aria-controls="catalog-results"`,
  `aria-activedescendant={items[highlight] ? \`catalog-option-${highlight}\` : undefined}`;
  `<ul>` gets `id="catalog-results"` and `role="listbox"`; each row gets
  `id={\`catalog-option-${i}\`}`, `role="option"` and
  `aria-selected={selected?.id === item.id}`. Note the deliberate
  asymmetry: this list's *selection* (`selected`) and its *highlight* are
  different things — `aria-selected` follows `selected`, and
  `aria-activedescendant` follows `highlight`. The `.selected` CSS class
  keeps following `selected`, unchanged.
- The rows keep `onClick` and get **no** `tabIndex`. The input keeps focus
  and drives the list; a roving tabindex on top of a combobox would be two
  keyboard models in one widget.
- Update the file's header comment: the sentence "Click-only selection —
  no arrow-key nav, a deliberate scope cut (unlike Search.tsx) since this
  modal's own text field already needs Enter for 'create'" is no longer
  true and its reasoning was wrong anyway (the name field only renders
  once an item is selected, so the *search* input's `Enter` was always
  free). Say what the component does now.

**Tests.** Extend `CatalogBrowser.test.tsx`:

- `ArrowDown` then `Enter` on the search input selects the second catalog
  item — assert the device-name input is pre-filled from it — using no
  mouse event at all. This is the regression test for the defect §20
  named: a keyboard-only user could not reach that list.
- `ArrowUp` at the top of the list stays at the top.
- the highlighted row is named by the input's `aria-activedescendant`.

Extend `SettingsPanel.test.tsx`: `Escape` still closes (now via the
shell), and a backdrop click still closes.

Keep every existing assertion in both suites passing; where one queried
the old DOM shape, update the query rather than deleting the assertion.

Commit.

## Task 4 — the copy-paste guard

Create `apps/knx-web/src/overlayShell.test.ts`. Using `node:fs` (never
`?raw` — see Global Constraint 4), read every `.tsx` file under
`apps/knx-web/src` and assert that `Overlay.tsx` is the only one whose
text contains `search-overlay`. On failure the message must name the
offending files, so the next agent to copy-paste a fifth overlay is told
what to do instead of guessing.

Model the file on `motionGuard.test.ts` — read it first — and reuse the
same `fileURLToPath`/`dirname`/`join` idiom for locating the source
directory. Exclude `.test.tsx` files from the scan: a test file may
legitimately mention the class name while asserting on it.

Add a short comment at the top of the file explaining why this guard
exists: §20's lift trigger was prose ("a third overlay is added"), prose
does not fail a build, and two further consumers were added before anyone
noticed.

Commit.

## Task 5 — documentation reconciliation (docs only, no code)

Goal.md rule 5: status, limitation, roadmap, compatibility and
architecture docs move in the same change as the code. This task writes
them. Read the design spec first; it is the source of truth for what was
and was not built.

- **`docs/KNOWN_LIMITATIONS.md` §20** — rewrite the body. The shell now
  exists and the keyboard defect is fixed, so the section is no longer
  "unaddressed". **Leave the header line byte-identical**, because
  `docs/GAP_ANALYSIS_ETS.md` links its anchor slug; the rename belongs to
  whoever updates both in one commit, and the documentation task is not
  the place to break an anchor. Instead, open the body with a dated
  resolution line in the style §17 and §27 already use ("— resolved"),
  state what shipped (`Overlay.tsx`, the listbox semantics, the catalog
  keyboard path, the copy-paste guard), and then state what did **not**
  ship, from the spec's section 5: no scroll-into-view for a highlight
  moved off-panel; no `inert`/`aria-hidden` on background content, so a
  screen reader's browse mode still reaches it; no focus-visible styling
  pass; and no verification against a real screen reader — jsdom asserts
  wiring, not assistive-technology behaviour. Do not claim accessibility
  conformance to any standard; nothing here has been audited.
- **`docs/GAP_ANALYSIS_ETS.md`** — row **D9** closes. Say what closed it
  and keep the residue visible: the four consumers now share one shell,
  the Catalog Browser's list is reachable by keyboard, and the remaining
  items above are tracked in §20, which stays open for them.
  Add **T31** to the backlog section as done, dated 2026-09-12, with a
  pointer to the design spec — place it where the tier structure puts UI
  work, and if no tier fits, say so in one line rather than inventing a
  tier.
- **`docs/IMPLEMENTATION_STATUS.md`** — move `Last updated:` to
  2026-09-12 and add a dated T31 entry in the existing entry style.
- **`docs/ROADMAP.md`** — only if it makes a claim this slice changes.
  Check; if nothing there is now false, change nothing and say so in the
  report.
- **`.ai/logs/2026-09-12_claude_t31_overlay_shell.md`** — a new log in the
  style of `.ai/logs/2026-09-12_claude_t27_motion_control.md`. Note that
  `.ai/` is gitignored: committing files there needs `git add -f`.
- **`.ai/CURRENT_STATE.md`** — **prepend** a new entry (newest first,
  `---` separated) in the documented format: Last Agent / Timestamp /
  Completed / Pending-Next Steps / Notes for Codex. Do not edit or
  reorder the existing entries.

Do not touch any `.ts`/`.tsx` file in this task. Report the exact test
counts you measured rather than copying them from an earlier task's
report.
