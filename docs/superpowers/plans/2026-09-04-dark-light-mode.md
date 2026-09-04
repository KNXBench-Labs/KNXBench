# Dark/Light Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a three-state (System/Light/Dark) theme toggle to
`knx-desktop`, persisted across restarts, with an icon button in the
toolbar and no flash of the wrong theme on launch.

**Architecture:** Frontend-only. `theme.ts` holds a pure cycling function,
pure `localStorage`-backed load/save functions (storage injected, not
read globally, so they're unit-testable under Node), and a `useTheme()`
hook that applies the current theme to `<html data-theme>` and persists
on change. `ThemeToggle.tsx` is a small presentational icon button.
`styles.css` gains three CSS custom properties for the only three colors
that don't already adapt to `color-scheme`, with a `prefers-color-scheme`
media-query default and `:root[data-theme]` overrides that outrank it.
`index.html` gets a tiny inline script to apply a persisted explicit
choice before React mounts, avoiding a one-frame flash of the wrong
theme.

**Tech Stack:** React 19, TypeScript, Vitest (`environment: "node"`, no
DOM — matches existing project convention: pure logic gets unit tests,
DOM-touching components don't).

**Spec:** `docs/superpowers/specs/2026-09-04-dark-light-mode-design.md`

## Global Constraints

- No backend change. Frontend-only, like cycles 6 and (partially) 5.
- Three theme states: `"system"`, `"light"`, `"dark"`, cycled in that
  order by one toggle button (not two independent controls).
- Persistence is `localStorage` under the key `"knx-desktop:theme"` — not
  a new Tauri plugin, not a field on `.knxdb`/`AppState`.
- No new npm dependency. The three toggle icons (sun/moon/monitor) are
  inline SVG written by hand, not an icon library.
- Only three existing CSS rules get new custom properties: the error
  color (`#b00020`, used in three selectors), the search-overlay backdrop
  (`rgba(0, 0, 0, 0.4)`), and the search-panel box-shadow color component
  (`rgba(0, 0, 0, 0.3)`). Everything else already adapts via
  `currentColor`/system color keywords and needs no change.
- `:root[data-theme="light"]`/`:root[data-theme="dark"]` must outrank the
  `@media (prefers-color-scheme: dark)` block regardless of source
  order — verified in the spec: an attribute selector on `:root` has
  higher specificity than a bare `:root` inside a media query.
- No new keyboard shortcut for this cycle — the toggle is click-only.

---

## File Structure

- Create: `apps/knx-desktop/src/theme.ts` — `Theme` type, `nextTheme`,
  `loadTheme`, `saveTheme`, `useTheme`.
- Create: `apps/knx-desktop/src/theme.test.ts` — Vitest coverage for the
  three pure functions.
- Create: `apps/knx-desktop/src/ThemeToggle.tsx` — the icon button.
- Modify: `apps/knx-desktop/src/styles.css` — three new custom
  properties, media-query default, two `[data-theme]` override blocks,
  and the three consuming rules switched to `var(...)`.
- Modify: `apps/knx-desktop/index.html` — inline script in `<head>`.
- Modify: `apps/knx-desktop/src/App.tsx` — wire `useTheme()` and render
  `<ThemeToggle>`.

---

### Task 1: `theme.ts` — state, persistence, pure cycling logic

**Files:**
- Create: `apps/knx-desktop/src/theme.ts`
- Test: `apps/knx-desktop/src/theme.test.ts`

**Interfaces:**
- Produces: `export type Theme = "system" | "light" | "dark";`,
  `nextTheme(current: Theme): Theme`,
  `loadTheme(storage: Pick<Storage, "getItem">): Theme`,
  `saveTheme(storage: Pick<Storage, "setItem">, theme: Theme): void`,
  `useTheme(): [Theme, () => void]`. Task 3 (`App.tsx`) calls `useTheme()`
  by this exact name and destructures its exact two-element tuple shape.
  Task 2 (`ThemeToggle.tsx`) imports the `Theme` type by this exact name.

- [ ] **Step 1: Write the failing test**

```ts
// apps/knx-desktop/src/theme.test.ts
import { describe, expect, it } from "vitest";
import { loadTheme, nextTheme, saveTheme } from "./theme";

function fakeStorage(initial: Record<string, string> = {}) {
  const store = { ...initial };
  return {
    getItem: (key: string) => (key in store ? store[key] : null),
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    _store: store,
  };
}

describe("nextTheme", () => {
  it("cycles system -> light -> dark -> system", () => {
    expect(nextTheme("system")).toBe("light");
    expect(nextTheme("light")).toBe("dark");
    expect(nextTheme("dark")).toBe("system");
  });
});

describe("loadTheme", () => {
  it("resolves to system when nothing is stored", () => {
    expect(loadTheme(fakeStorage())).toBe("system");
  });

  it("resolves to the stored value for light or dark", () => {
    expect(loadTheme(fakeStorage({ "knx-desktop:theme": "light" }))).toBe("light");
    expect(loadTheme(fakeStorage({ "knx-desktop:theme": "dark" }))).toBe("dark");
  });

  it("resolves to system for an unrecognized stored value", () => {
    expect(loadTheme(fakeStorage({ "knx-desktop:theme": "solarized" }))).toBe("system");
  });
});

describe("saveTheme", () => {
  it("writes the theme under the expected key", () => {
    const storage = fakeStorage();
    saveTheme(storage, "dark");
    expect(storage._store["knx-desktop:theme"]).toBe("dark");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/knx-desktop && npm test -- theme`
Expected: FAIL — `Cannot find module './theme'`

- [ ] **Step 3: Write minimal implementation**

```ts
// apps/knx-desktop/src/theme.ts
import { useEffect, useState } from "react";

export type Theme = "system" | "light" | "dark";

const ORDER: Theme[] = ["system", "light", "dark"];
const STORAGE_KEY = "knx-desktop:theme";

/** Cycles System -> Light -> Dark -> System. */
export function nextTheme(current: Theme): Theme {
  return ORDER[(ORDER.indexOf(current) + 1) % ORDER.length];
}

/**
 * Reads the persisted theme. Any stored value other than "light"/"dark"
 * (missing key, or a value from a future/incompatible version) resolves
 * to "system" — the always-safe default, never a hard failure.
 */
export function loadTheme(storage: Pick<Storage, "getItem">): Theme {
  const raw = storage.getItem(STORAGE_KEY);
  return raw === "light" || raw === "dark" ? raw : "system";
}

export function saveTheme(storage: Pick<Storage, "setItem">, theme: Theme): void {
  storage.setItem(STORAGE_KEY, theme);
}

/**
 * Reads the persisted theme on mount, applies it to `<html data-theme>`
 * (removed entirely for "system", so the `prefers-color-scheme` media
 * query in styles.css governs), and persists on every change. The
 * returned setter is a cycle-to-next function, not an arbitrary setter —
 * the toggle button is the only caller and only ever advances the cycle.
 */
export function useTheme(): [Theme, () => void] {
  const [theme, setTheme] = useState<Theme>(() => loadTheme(window.localStorage));

  useEffect(() => {
    if (theme === "system") {
      document.documentElement.removeAttribute("data-theme");
    } else {
      document.documentElement.setAttribute("data-theme", theme);
    }
    saveTheme(window.localStorage, theme);
  }, [theme]);

  return [theme, () => setTheme(nextTheme)];
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd apps/knx-desktop && npm test -- theme`
Expected: PASS, 5 tests

- [ ] **Step 5: Commit**

```bash
git add apps/knx-desktop/src/theme.ts apps/knx-desktop/src/theme.test.ts
git commit -m "feat(knx-desktop): add theme state, persistence, cycling logic"
```

---

### Task 2: `ThemeToggle.tsx` — icon button

**Files:**
- Create: `apps/knx-desktop/src/ThemeToggle.tsx`

**Interfaces:**
- Consumes: `Theme` type from `./theme` (Task 1).
- Produces: default export `ThemeToggle(props: { theme: Theme; onCycle:
  () => void })`. Task 3 (`App.tsx`) renders
  `<ThemeToggle theme={theme} onCycle={cycleTheme} />`.

No component-testing library exists in this project (`vitest.config.ts`
sets `environment: "node"`) — this task is verified by type-checking, not
an automated test, matching `Search.tsx`/`CommandPalette.tsx`'s
precedent.

- [ ] **Step 1: Write the component**

```tsx
// apps/knx-desktop/src/ThemeToggle.tsx
import type { Theme } from "./theme";

const LABELS: Record<Theme, string> = {
  system: "Following system theme",
  light: "Light theme",
  dark: "Dark theme",
};

const ICONS: Record<Theme, React.ReactElement> = {
  system: <MonitorIcon />,
  light: <SunIcon />,
  dark: <MoonIcon />,
};

export default function ThemeToggle(props: { theme: Theme; onCycle: () => void }) {
  const { theme, onCycle } = props;
  return (
    <button onClick={onCycle} title={LABELS[theme]} aria-label={LABELS[theme]}>
      {ICONS[theme]}
    </button>
  );
}

function SunIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <circle cx="12" cy="12" r="4" />
      <line x1="12" y1="2" x2="12" y2="4" />
      <line x1="12" y1="20" x2="12" y2="22" />
      <line x1="4.2" y1="4.2" x2="5.6" y2="5.6" />
      <line x1="18.4" y1="18.4" x2="19.8" y2="19.8" />
      <line x1="2" y1="12" x2="4" y2="12" />
      <line x1="20" y1="12" x2="22" y2="12" />
      <line x1="4.2" y1="19.8" x2="5.6" y2="18.4" />
      <line x1="18.4" y1="5.6" x2="19.8" y2="4.2" />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
      <path d="M20 14.5A8.5 8.5 0 0 1 9.5 4 8.5 8.5 0 1 0 20 14.5Z" />
    </svg>
  );
}

function MonitorIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="4" width="18" height="12" rx="1.5" />
      <line x1="8" y1="20" x2="16" y2="20" />
      <line x1="12" y1="16" x2="12" y2="20" />
    </svg>
  );
}
```

`React.ReactElement` needs no import of its own —
`apps/knx-desktop/src/ProjectExplorer.tsx:12` already references
`React.ReactNode` the same way, unimported, via the global `React`
namespace `@types/react` provides.

- [ ] **Step 2: Type-check**

Run: `cd apps/knx-desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add apps/knx-desktop/src/ThemeToggle.tsx
git commit -m "feat(knx-desktop): add theme toggle icon button"
```

---

### Task 3: Wire theme into the app (CSS, inline script, `App.tsx`)

**Files:**
- Modify: `apps/knx-desktop/src/styles.css`
- Modify: `apps/knx-desktop/index.html`
- Modify: `apps/knx-desktop/src/App.tsx`

**Interfaces:**
- Consumes: `useTheme` from `./theme` (Task 1), `ThemeToggle` default
  export from `./ThemeToggle` (Task 2).
- Produces: nothing for later tasks — this is the integration point and
  the plan's final task.

- [ ] **Step 1: Add the CSS custom properties**

In `apps/knx-desktop/src/styles.css`, replace the `:root` block at the
top of the file:

```css
/* apps/knx-desktop/src/styles.css — old */
:root {
  color-scheme: light dark;
  font-family: system-ui, sans-serif;
}
```

with:

```css
/* apps/knx-desktop/src/styles.css — new */
:root {
  color-scheme: light dark;
  font-family: system-ui, sans-serif;
  --knx-error-color: #b00020;
  --knx-overlay-backdrop: rgba(0, 0, 0, 0.4);
  --knx-overlay-shadow: rgba(0, 0, 0, 0.3);
}

@media (prefers-color-scheme: dark) {
  :root {
    --knx-error-color: #ff6b6b;
    --knx-overlay-backdrop: rgba(0, 0, 0, 0.6);
    --knx-overlay-shadow: rgba(0, 0, 0, 0.5);
  }
}

:root[data-theme="light"] {
  --knx-error-color: #b00020;
  --knx-overlay-backdrop: rgba(0, 0, 0, 0.4);
  --knx-overlay-shadow: rgba(0, 0, 0, 0.3);
  color-scheme: light;
}

:root[data-theme="dark"] {
  --knx-error-color: #ff6b6b;
  --knx-overlay-backdrop: rgba(0, 0, 0, 0.6);
  --knx-overlay-shadow: rgba(0, 0, 0, 0.5);
  color-scheme: dark;
}
```

- [ ] **Step 2: Switch the three consuming rules to `var(...)`**

Replace:

```css
.error-banner {
  color: #b00020;
  border: 1px solid #b00020;
  padding: 0.5rem;
  border-radius: 4px;
}
```

with:

```css
.error-banner {
  color: var(--knx-error-color);
  border: 1px solid var(--knx-error-color);
  padding: 0.5rem;
  border-radius: 4px;
}
```

Replace:

```css
.project-explorer footer .import-errors {
  color: #b00020;
  font-weight: bold;
}
```

with:

```css
.project-explorer footer .import-errors {
  color: var(--knx-error-color);
  font-weight: bold;
}
```

Replace:

```css
.field-error {
  display: block;
  color: #b00020;
  font-size: 0.85em;
}
```

with:

```css
.field-error {
  display: block;
  color: var(--knx-error-color);
  font-size: 0.85em;
}
```

Replace:

```css
.search-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  justify-content: center;
  padding-top: 10vh;
  z-index: 10;
}
```

with:

```css
.search-overlay {
  position: fixed;
  inset: 0;
  background: var(--knx-overlay-backdrop);
  display: flex;
  justify-content: center;
  padding-top: 10vh;
  z-index: 10;
}
```

Replace:

```css
.search-panel {
  background: Canvas;
  color: CanvasText;
  width: min(32rem, 90vw);
  max-height: 70vh;
  overflow-y: auto;
  border-radius: 6px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.3);
  padding: 0.75rem;
}
```

with:

```css
.search-panel {
  background: Canvas;
  color: CanvasText;
  width: min(32rem, 90vw);
  max-height: 70vh;
  overflow-y: auto;
  border-radius: 6px;
  box-shadow: 0 4px 24px var(--knx-overlay-shadow);
  padding: 0.75rem;
}
```

- [ ] **Step 3: Add the inline script to `index.html`**

Replace:

```html
  <head>
    <meta charset="UTF-8" />
    <title>KNX</title>
  </head>
```

with:

```html
  <head>
    <meta charset="UTF-8" />
    <title>KNX</title>
    <script>
      (function () {
        var t = localStorage.getItem("knx-desktop:theme");
        if (t === "light" || t === "dark") {
          document.documentElement.setAttribute("data-theme", t);
        }
      })();
    </script>
  </head>
```

This deliberately duplicates `theme.ts`'s `STORAGE_KEY` string and its
two valid values literally — this script runs before any module graph is
available, so it cannot import from `theme.ts`. If `STORAGE_KEY` in
`theme.ts` (Task 1) or the set of valid theme values ever changes, this
literal must be updated by hand to match.

- [ ] **Step 4: Wire `App.tsx`**

Add to the import block (after the existing `import type { CommandContext }
from "./commandRegistry";` line):

```ts
import ThemeToggle from "./ThemeToggle";
import { useTheme } from "./theme";
```

Add inside the `App` function body, alongside the other top-level state
(after the existing `const [paletteOpen, setPaletteOpen] = useState(false);`
line):

```ts
const [theme, cycleTheme] = useTheme();
```

Add the toggle to the toolbar. Replace:

```tsx
      <button
        onClick={() => {
          setSearchOpen(false);
          setPaletteOpen(true);
        }}
      >
        Commands… (Ctrl+Shift+P)
      </button>
      {error && (
```

with:

```tsx
      <button
        onClick={() => {
          setSearchOpen(false);
          setPaletteOpen(true);
        }}
      >
        Commands… (Ctrl+Shift+P)
      </button>
      <ThemeToggle theme={theme} onCycle={cycleTheme} />
      {error && (
```

- [ ] **Step 5: Type-check and run the full test suite**

Run: `cd apps/knx-desktop && npx tsc --noEmit && npm test`
Expected: no type errors; all existing tests plus Task 1's
`theme.test.ts` pass (28 total: 23 existing + 5 new).

- [ ] **Step 6: Manual smoke check (leave documented, not performed here)**

This step cannot be executed in this environment — no display is
available for a Tauri GUI session, the same limitation cycles 4-6
recorded in `docs/IMPLEMENTATION_STATUS.md`. Before or at merge, a
session with a display should:

1. Launch the app, confirm the theme icon matches the OS's current
   color scheme (System state, monitor icon).
2. Click the toggle: icon changes to sun, UI switches to (or stays in,
   if the OS is already light) light colors. Click again: moon icon,
   dark colors. Click again: back to monitor/System.
3. Trigger an error banner (e.g. open a nonexistent `.knxproj` path if
   possible, or any existing error path) in both Light and Dark: confirm
   the error text/border stays legible in both.
4. Open the search overlay (`Ctrl+K`) in both Light and Dark: confirm the
   backdrop and panel shadow both still read as "a modal is open" rather
   than disappearing into the background.
5. Restart the app after picking an explicit Light or Dark: confirm the
   choice survives (no flash of the wrong theme, no reset to System).

- [ ] **Step 7: Commit**

```bash
git add apps/knx-desktop/src/styles.css apps/knx-desktop/index.html apps/knx-desktop/src/App.tsx
git commit -m "feat(knx-desktop): wire theme toggle into styles.css, index.html, App.tsx"
```

---

## Post-implementation documentation

After Task 3 is committed and the manual smoke check (Step 6) has been
run on a machine with a display:

- Update `docs/ROADMAP.md`'s Session 5 section: add a "Cycle 7" paragraph
  in the same style as cycles 1-6, and remove the "dark/light mode
  remains not yet scheduled" trailer sentences (currently in the cycle
  1-6 paragraphs and the "Cycle 7+ candidates" paragraph, which should be
  renumbered to "Cycle 8+" now that cycle 7 is taken). Note that Session
  5's full CLAUDE.md/ROADMAP UI/UX deliverable list is now complete —
  Project Explorer, properties inspector, search, command palette, and
  dark/light mode have all shipped.
- Update `docs/IMPLEMENTATION_STATUS.md`: record dark/light mode as
  shipped (cycle 7), bump the vitest count (23 → 28), and note whether
  Step 6's manual smoke check was actually performed, following cycles
  4-6's template for recording this explicitly either way.
- If Step 6 surfaces a real bug (not a docs gap) — e.g. a contrast
  problem in the placeholder dark-mode color values — fix it under
  `superpowers:systematic-debugging` before writing these docs updates,
  not after.
