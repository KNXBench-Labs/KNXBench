# Bitcoin DeFi Theme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `knx-web`'s System/Light/Dark theme cycle + 4-token color-override panel with a named, selectable theme registry, ship "Bitcoin DeFi" as its first and default entry, and apply it to every existing UI surface (buttons, overlays, cards, inputs, technical text, headings, background).

**Architecture:** Frontend-only (`apps/knx-web`), no backend/domain changes. A small TypeScript registry module (`theme.ts`) drives one `data-theme` attribute on `<html>`; every visual token lives in `styles.css` as a CSS custom property scoped under `:root[data-theme="bitcoin-defi"]`; every component recipe is a class added to the same file. `knx-desktop` (Tauri) and the Docker web target both serve this one frontend unchanged, so "native and web identical" requires no separate work.

**Tech Stack:** React 19 + TypeScript + Vite (existing), Vitest (existing), `@fontsource/*` (new deps, self-hosted fonts), Google Fonts `<link>` (new, CDN overlay).

**Spec:** `docs/superpowers/specs/2026-09-08-bitcoin-defi-theme-design.md`

## Global Constraints

- Storage key `knx-desktop:theme` is reused as-is (old `"system"`/`"light"`/`"dark"` values must silently fall back to the new default, not crash or throw).
- No Tailwind, no `cva`, no UI-kit dependency — plain CSS + the existing `--knx-*` custom-property convention only.
- Every theme token lives inside `:root[data-theme="bitcoin-defi"]` — no bare-`:root` fallback values (the inline `index.html` script guarantees the attribute is always set before paint).
- `prefers-reduced-motion: no-preference` continues to gate all transitions/animations — no new unconditional animation.
- `apps/knx-desktop` gets no changes — it has no UI code of its own (`docs/ARCHITECTURE.md` §3).
- Commit as `github@knxbench.com`, no co-author trailer (CLAUDE.md).

---

## File Structure

| File | Change |
|---|---|
| `apps/knx-web/src/theme.ts` | Rewritten: registry (`ThemeDef[]`), `loadThemeId`/`saveThemeId`/`useThemeId` replace `Theme`/`nextTheme`/`loadTheme`/`saveTheme`/`useTheme` |
| `apps/knx-web/src/theme.test.ts` | Rewritten for the id-based registry |
| `apps/knx-web/src/palette.ts` | Deleted |
| `apps/knx-web/src/palette.test.ts` | Deleted |
| `apps/knx-web/src/ThemePanel.tsx` | Deleted |
| `apps/knx-web/src/ThemeToggle.tsx` | Deleted |
| `apps/knx-web/src/ThemeSwitcher.tsx` | New — `<select>`-based theme picker |
| `apps/knx-web/src/App.tsx` | Wiring: drop palette/panel state and imports, swap `ThemeToggle` for `ThemeSwitcher` |
| `apps/knx-web/index.html` | Inline script rewritten (always-set attribute, known-id list); Google Fonts `<link>` tags added |
| `apps/knx-web/src/main.tsx` | `@fontsource/*` CSS imports added |
| `apps/knx-web/package.json` | `@fontsource/space-grotesk`, `@fontsource/inter`, `@fontsource/jetbrains-mono` added |
| `apps/knx-web/src/styles.css` | Token layer rewritten; component recipes (buttons, overlays, cards, inputs, mono text, headings, background texture, corner accents, toasts) added; dead `.theme-panel-*` rules removed |
| `docs/ROADMAP.md` | Cycle 13 entry added, stale "Cycle 12+ candidates" line resolved |
| `docs/IMPLEMENTATION_STATUS.md` | Session 5 row updated with cycle 13 |

---

## Task 1: `theme.ts` registry + tests

**Files:**
- Modify: `apps/knx-web/src/theme.ts` (full rewrite)
- Modify: `apps/knx-web/src/theme.test.ts` (full rewrite)

**Interfaces:**
- Produces: `interface ThemeDef { id: string; name: string }`, `THEMES: readonly ThemeDef[]`, `loadThemeId(storage: Pick<Storage, "getItem">): string`, `saveThemeId(storage: Pick<Storage, "setItem">, id: string): void`, `useThemeId(): [string, (id: string) => void]`.

- [ ] **Step 1: Write the failing tests**

Replace the full contents of `apps/knx-web/src/theme.test.ts` with:

```ts
import { describe, expect, it, vi } from "vitest";
import { THEMES, loadThemeId, saveThemeId } from "./theme";

function fakeStorage(initial: Record<string, string> = {}) {
  const store = { ...initial };
  return {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store[key] = value;
    }),
  };
}

describe("THEMES", () => {
  it("includes bitcoin-defi", () => {
    expect(THEMES.some((t) => t.id === "bitcoin-defi")).toBe(true);
  });
});

describe("loadThemeId", () => {
  it("defaults to bitcoin-defi when nothing is stored", () => {
    expect(loadThemeId(fakeStorage())).toBe("bitcoin-defi");
  });

  it("returns a known stored id unchanged", () => {
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "bitcoin-defi" }))).toBe("bitcoin-defi");
  });

  it("falls back to bitcoin-defi for an unknown id", () => {
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "solarized" }))).toBe("bitcoin-defi");
  });

  it("falls back to bitcoin-defi for cycle 7's old System/Light/Dark values", () => {
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "system" }))).toBe("bitcoin-defi");
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "light" }))).toBe("bitcoin-defi");
    expect(loadThemeId(fakeStorage({ "knx-desktop:theme": "dark" }))).toBe("bitcoin-defi");
  });
});

describe("saveThemeId", () => {
  it("stores the exact key and value", () => {
    const storage = fakeStorage();
    saveThemeId(storage, "bitcoin-defi");
    expect(storage.setItem).toHaveBeenCalledWith("knx-desktop:theme", "bitcoin-defi");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd apps/knx-web && npx vitest run theme.test.ts`
Expected: FAIL — `theme.ts` still exports the old `Theme`/`loadTheme`/`saveTheme` shape, not `THEMES`/`loadThemeId`/`saveThemeId`.

- [ ] **Step 3: Write the implementation**

Replace the full contents of `apps/knx-web/src/theme.ts` with:

```ts
import { useEffect, useState } from "react";

export interface ThemeDef {
  id: string;
  name: string;
}

export const THEMES: readonly ThemeDef[] = [{ id: "bitcoin-defi", name: "Bitcoin DeFi" }];

const DEFAULT_THEME_ID = "bitcoin-defi";
const STORAGE_KEY = "knx-desktop:theme";

function isThemeId(id: string): boolean {
  return THEMES.some((t) => t.id === id);
}

/**
 * Reads the persisted theme id. Anything that isn't a known theme id —
 * missing key, a value from a future/incompatible version, cycle 7's old
 * "system"/"light"/"dark" values, or a theme that's since been removed —
 * resolves to the default, the same always-safe-default philosophy as the
 * old `loadTheme`.
 */
export function loadThemeId(storage: Pick<Storage, "getItem">): string {
  const raw = storage.getItem(STORAGE_KEY);
  return raw && isThemeId(raw) ? raw : DEFAULT_THEME_ID;
}

export function saveThemeId(storage: Pick<Storage, "setItem">, id: string): void {
  storage.setItem(STORAGE_KEY, id);
}

/**
 * Reads the persisted theme id on mount, applies it to `<html
 * data-theme>`, and persists on every change. Unlike the old `useTheme`,
 * the attribute is always set — there is no "system"/unthemed state
 * anymore.
 */
export function useThemeId(): [string, (id: string) => void] {
  const [id, setId] = useState<string>(() => loadThemeId(window.localStorage));

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", id);
    saveThemeId(window.localStorage, id);
  }, [id]);

  return [id, setId];
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd apps/knx-web && npx vitest run theme.test.ts`
Expected: PASS, 6 tests.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-web/src/theme.ts apps/knx-web/src/theme.test.ts
git commit -m "feat(knx-web): theme.ts becomes a named-theme registry" --author="KNXBench-Labs <github@knxbench.com>"
```

---

## Task 2: `ThemeSwitcher.tsx`, delete `ThemeToggle.tsx`

**Files:**
- Create: `apps/knx-web/src/ThemeSwitcher.tsx`
- Delete: `apps/knx-web/src/ThemeToggle.tsx`

**Interfaces:**
- Consumes: `ThemeDef` from `./theme` (Task 1).
- Produces: `export default function ThemeSwitcher(props: { themes: readonly ThemeDef[]; activeId: string; onSelect: (id: string) => void }): JSX.Element`.

- [ ] **Step 1: Delete `ThemeToggle.tsx`**

```bash
git rm apps/knx-web/src/ThemeToggle.tsx
```

- [ ] **Step 2: Create `ThemeSwitcher.tsx`**

```tsx
import type { ThemeDef } from "./theme";

export default function ThemeSwitcher(props: {
  themes: readonly ThemeDef[];
  activeId: string;
  onSelect: (id: string) => void;
}) {
  const { themes, activeId, onSelect } = props;
  return (
    <label className="theme-switcher">
      <span className="sr-only">Theme</span>
      <select value={activeId} onChange={(e) => onSelect(e.target.value)} aria-label="Theme">
        {themes.map((t) => (
          <option key={t.id} value={t.id}>
            {t.name}
          </option>
        ))}
      </select>
    </label>
  );
}
```

- [ ] **Step 3: Type-check**

Run: `cd apps/knx-web && npx tsc --noEmit`
Expected: `App.tsx` now errors on the removed `ThemeToggle` import (fixed in Task 3) — `ThemeSwitcher.tsx` itself must show no new errors. If `tsc` reports anything inside `ThemeSwitcher.tsx`, fix before continuing.

- [ ] **Step 4: Commit**

```bash
git add apps/knx-web/src/ThemeSwitcher.tsx apps/knx-web/src/ThemeToggle.tsx
git commit -m "feat(knx-web): ThemeSwitcher replaces ThemeToggle" --author="KNXBench-Labs <github@knxbench.com>"
```

---

## Task 3: `App.tsx` wiring — drop palette/panel, wire the switcher

**Files:**
- Modify: `apps/knx-web/src/App.tsx`
- Delete: `apps/knx-web/src/palette.ts`
- Delete: `apps/knx-web/src/palette.test.ts`
- Delete: `apps/knx-web/src/ThemePanel.tsx`

**Interfaces:**
- Consumes: `useThemeId` and `THEMES` from `./theme` (Task 1), `ThemeSwitcher` from `./ThemeSwitcher` (Task 2).

- [ ] **Step 1: Delete the palette module and panel**

```bash
git rm apps/knx-web/src/palette.ts apps/knx-web/src/palette.test.ts apps/knx-web/src/ThemePanel.tsx
```

- [ ] **Step 2: Edit `App.tsx` imports**

In `apps/knx-web/src/App.tsx`, replace:

```tsx
import ThemeToggle from "./ThemeToggle";
import ThemePanel from "./ThemePanel";
import Dashboard from "./Dashboard";
import { useTheme } from "./theme";
import { usePalette } from "./palette";
```

with:

```tsx
import ThemeSwitcher from "./ThemeSwitcher";
import Dashboard from "./Dashboard";
import { THEMES, useThemeId } from "./theme";
```

- [ ] **Step 3: Edit `App.tsx` state**

Replace:

```tsx
  const [theme, cycleTheme] = useTheme();
  const [paletteSettings, setPaletteColor, setPaletteMotion, resetPalette] = usePalette();
  const [themePanelOpen, setThemePanelOpen] = useState(false);
```

with:

```tsx
  const [themeId, setThemeId] = useThemeId();
```

- [ ] **Step 4: Edit `App.tsx` JSX**

Replace:

```tsx
      <ThemeToggle theme={theme} onCycle={cycleTheme} />
      <button onClick={() => setThemePanelOpen(true)} title="Customize theme" aria-label="Customize theme">
        ⚙
      </button>
      {themePanelOpen && (
        <ThemePanel
          settings={paletteSettings}
          onSetColor={setPaletteColor}
          onSetMotion={setPaletteMotion}
          onResetAll={resetPalette}
          onClose={() => setThemePanelOpen(false)}
        />
      )}
```

with:

```tsx
      <ThemeSwitcher themes={THEMES} activeId={themeId} onSelect={setThemeId} />
```

- [ ] **Step 5: Type-check and run the full test suite**

Run: `cd apps/knx-web && npx tsc --noEmit && npx vitest run`
Expected: `tsc` clean; vitest passes (palette.test.ts's tests are gone from the count, theme.test.ts's 6 new tests present, no other suite touches the deleted modules — confirm with `grep -rn "palette\|ThemeToggle\|ThemePanel" apps/knx-web/src` if anything unexpected still references them).

- [ ] **Step 6: Commit**

```bash
git add apps/knx-web/src/App.tsx apps/knx-web/src/palette.ts apps/knx-web/src/palette.test.ts apps/knx-web/src/ThemePanel.tsx
git commit -m "feat(knx-web): wire ThemeSwitcher into App, drop palette overrides" --author="KNXBench-Labs <github@knxbench.com>"
```

---

## Task 4: `index.html` — always-set attribute + Google Fonts

**Files:**
- Modify: `apps/knx-web/index.html`

- [ ] **Step 1: Replace the inline script**

Replace:

```html
    <script>
      (function () {
        var t = localStorage.getItem("knx-desktop:theme");
        if (t === "light" || t === "dark") {
          document.documentElement.setAttribute("data-theme", t);
        }
      })();
    </script>
```

with:

```html
    <script>
      (function () {
        var t = localStorage.getItem("knx-desktop:theme");
        var known = ["bitcoin-defi"];
        document.documentElement.setAttribute(
          "data-theme",
          known.indexOf(t) !== -1 ? t : "bitcoin-defi",
        );
      })();
    </script>
```

- [ ] **Step 2: Add Google Fonts links, right after the inline script**

```html
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link
      href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap"
      rel="stylesheet"
    />
```

- [ ] **Step 3: Verify manually**

Run: `cat apps/knx-web/index.html` — confirm `<html lang="en">` still has no `data-theme` attribute in the static markup (it's set at runtime by the script) and the three new `<link>` tags sit inside `<head>`, after the script, before `</head>`.

- [ ] **Step 4: Commit**

```bash
git add apps/knx-web/index.html
git commit -m "feat(knx-web): index.html always sets data-theme, adds Google Fonts" --author="KNXBench-Labs <github@knxbench.com>"
```

---

## Task 5: Self-hosted fonts (`@fontsource`)

**Files:**
- Modify: `apps/knx-web/package.json`
- Modify: `apps/knx-web/src/main.tsx`

- [ ] **Step 1: Install the font packages**

```bash
cd apps/knx-web && npm install @fontsource/space-grotesk @fontsource/inter @fontsource/jetbrains-mono
```

Expected: `package.json`'s `dependencies` gains all three; `package-lock.json` updates.

- [ ] **Step 2: Import them in `main.tsx`**

Read `apps/knx-web/src/main.tsx` first to see its exact current import order, then add these imports at the top of the file, before the existing `styles.css` import if one exists there (otherwise before the first local import):

```ts
import "@fontsource/space-grotesk/400.css";
import "@fontsource/space-grotesk/500.css";
import "@fontsource/space-grotesk/600.css";
import "@fontsource/space-grotesk/700.css";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
```

- [ ] **Step 3: Build to verify the imports resolve**

Run: `cd apps/knx-web && npm run build`
Expected: builds clean, no "module not found" errors for any `@fontsource/*` path.

- [ ] **Step 4: Commit**

```bash
git add apps/knx-web/package.json apps/knx-web/package-lock.json apps/knx-web/src/main.tsx
git commit -m "feat(knx-web): self-host Space Grotesk/Inter/JetBrains Mono" --author="KNXBench-Labs <github@knxbench.com>"
```

---

## Task 6: `styles.css` — token layer

**Files:**
- Modify: `apps/knx-web/src/styles.css:1-40` (the old `:root`, `@media (prefers-color-scheme: dark)`, `:root[data-theme="light"]`, `:root[data-theme="dark"]` blocks)

**Interfaces:**
- Produces: every `--knx-*` custom property later tasks' recipes read: `--knx-bg`, `--knx-surface`, `--knx-foreground`, `--knx-muted`, `--knx-border`, `--knx-border-hover`, `--knx-accent`, `--knx-accent-secondary`, `--knx-accent-tertiary`, `--knx-gradient-primary`, `--knx-gradient-gold`, `--knx-error-color`, `--knx-overlay-backdrop`, `--knx-overlay-shadow`, `--knx-glow-orange`, `--knx-glow-orange-strong`, `--knx-glow-gold`, `--knx-font-heading`, `--knx-font-body`, `--knx-font-mono`, `--knx-radius-card`, `--knx-radius-button`, `--knx-radius-input`, `--knx-transition-duration`.

- [ ] **Step 1: Read the current top of the file**

Read `apps/knx-web/src/styles.css` lines 1-53 to confirm the exact current text before editing (it may have shifted slightly from what's quoted in the spec).

- [ ] **Step 2: Replace everything from the top of the file through the end of the `:root[data-theme="dark"]` block — i.e. the `:root { ... }` block, the `@media (prefers-color-scheme: dark) { ... }` block, and the `:root[data-theme="light"] { ... }`/`:root[data-theme="dark"] { ... }` blocks, stopping right before the `@media (prefers-reduced-motion: no-preference)` rule — with:**

```css
:root[data-theme="bitcoin-defi"] {
  color-scheme: dark;
  font-family: var(--knx-font-body);

  /* Surfaces */
  --knx-bg: #030304;
  --knx-surface: #0f1115;
  --knx-foreground: #ffffff;
  --knx-muted: #94a3b8;
  --knx-border: rgba(255, 255, 255, 0.1);
  --knx-border-hover: rgba(247, 147, 26, 0.5);

  /* Accents */
  --knx-accent: #f7931a;
  --knx-accent-secondary: #ea580c;
  --knx-accent-tertiary: #ffd600;
  --knx-gradient-primary: linear-gradient(to right, #ea580c, #f7931a);
  --knx-gradient-gold: linear-gradient(to right, #f7931a, #ffd600);

  /* Feedback (colored, never pure black) */
  --knx-error-color: #ff6b6b;
  --knx-overlay-backdrop: rgba(3, 3, 4, 0.7);
  --knx-overlay-shadow: rgba(234, 88, 12, 0.25);
  --knx-glow-orange: 0 0 20px -5px rgba(234, 88, 12, 0.5);
  --knx-glow-orange-strong: 0 0 30px -5px rgba(247, 147, 26, 0.6);
  --knx-glow-gold: 0 0 20px rgba(255, 214, 0, 0.3);

  /* Typography */
  --knx-font-heading: "Space Grotesk", sans-serif;
  --knx-font-body: "Inter", sans-serif;
  --knx-font-mono: "JetBrains Mono", monospace;

  /* Radius */
  --knx-radius-card: 16px;
  --knx-radius-button: 999px;
  --knx-radius-input: 8px;

  --knx-transition-duration: 250ms;
}
```

Keep the `@media (prefers-reduced-motion: no-preference) { ... }` block and everything after it in the file unchanged for now (later tasks edit those rules).

- [ ] **Step 3: Fix `--knx-text` → `--knx-foreground` renames**

Run: `grep -n "knx-text" apps/knx-web/src/styles.css` — for every match, replace `var(--knx-text)` with `var(--knx-foreground)`.

- [ ] **Step 4: Build and run tests**

Run: `cd apps/knx-web && npx tsc --noEmit && npx vitest run && npm run build`
Expected: all clean — this task is CSS-only, so no test exercises the visual result, but the build must still succeed (a malformed CSS block can still fail Vite's CSS parser).

- [ ] **Step 5: Commit**

```bash
git add apps/knx-web/src/styles.css
git commit -m "feat(knx-web): Bitcoin DeFi theme token layer" --author="KNXBench-Labs <github@knxbench.com>"
```

---

## Task 7: `styles.css` — buttons, inputs, technical text, headings

**Files:**
- Modify: `apps/knx-web/src/styles.css`

- [ ] **Step 1: Add button styling**

Append to `styles.css`:

```css
button {
  font-family: var(--knx-font-body);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border: none;
  border-radius: var(--knx-radius-button);
  padding: 0.5rem 1.25rem;
  min-height: 44px;
  color: #ffffff;
  background: var(--knx-gradient-primary);
  box-shadow: var(--knx-glow-orange);
  cursor: pointer;
}

@media (prefers-reduced-motion: no-preference) {
  button {
    transition: transform var(--knx-transition-duration) ease,
      box-shadow var(--knx-transition-duration) ease;
  }
}

button:hover:not(:disabled) {
  transform: scale(1.05);
  box-shadow: var(--knx-glow-orange-strong);
}

button:disabled {
  background: var(--knx-surface);
  color: var(--knx-muted);
  box-shadow: none;
  opacity: 0.6;
  cursor: not-allowed;
}
```

- [ ] **Step 2: Add input/select styling**

Append:

```css
input,
select {
  font-family: var(--knx-font-mono);
  color: var(--knx-foreground);
  background: rgba(0, 0, 0, 0.5);
  border: none;
  border-bottom: 2px solid var(--knx-border);
  border-radius: var(--knx-radius-input) var(--knx-radius-input) 0 0;
  padding: 0.5rem 0.75rem;
}

input:focus-visible,
select:focus-visible {
  outline: none;
  border-bottom-color: var(--knx-accent);
  box-shadow: 0 10px 20px -10px rgba(247, 147, 26, 0.3);
}
```

- [ ] **Step 3: Point technical/mono text at the theme's mono font and gold accent**

Find `.inspector-address` in `styles.css` (currently `font-family: monospace;`) and replace its rule with:

```css
.inspector-address {
  font-family: var(--knx-font-mono);
  color: var(--knx-accent-tertiary);
}
```

Find `.provenance-badge` and add `font-family: var(--knx-font-mono);` to its existing declaration block.

Find `.com-object-label` and add `font-family: var(--knx-font-mono);` to its existing declaration block.

- [ ] **Step 4: Heading font, gradient text on the Dashboard title**

Append:

```css
h1,
h2,
h3,
h4,
h5,
h6 {
  font-family: var(--knx-font-heading);
  font-weight: 600;
}

.dashboard > h2:first-child {
  background: var(--knx-gradient-gold);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}
```

Read `apps/knx-web/src/Dashboard.tsx` first to confirm its title is in fact the first `<h2>` child of the `.dashboard` container — if the actual markup differs, adjust the selector to match instead of leaving it wrong.

- [ ] **Step 5: Build and run tests**

Run: `cd apps/knx-web && npx tsc --noEmit && npx vitest run && npm run build`
Expected: all clean.

- [ ] **Step 6: Commit**

```bash
git add apps/knx-web/src/styles.css
git commit -m "feat(knx-web): themed buttons, inputs, mono text, headings" --author="KNXBench-Labs <github@knxbench.com>"
```

---

## Task 8: `styles.css` — overlays (glass morphism), cards, corner accents

**Files:**
- Modify: `apps/knx-web/src/styles.css`

- [ ] **Step 1: Glass-morphism overlays**

Find `.search-panel` and `.fs-picker` rule blocks. Both currently set `background: var(--knx-surface);` and `box-shadow: 0 4px 24px var(--knx-overlay-shadow);` — replace those two declarations (keep every other declaration in each block unchanged) with:

```css
  background: rgba(15, 17, 21, 0.6);
  backdrop-filter: blur(12px);
  border: 1px solid var(--knx-border);
  box-shadow: 0 4px 24px var(--knx-overlay-shadow);
```

- [ ] **Step 2: Cards — dashboard stats, com-object list items**

Find `.dashboard-stats` — add to its existing block:

```css
  background: var(--knx-surface);
  border: 1px solid var(--knx-border);
  border-radius: var(--knx-radius-card);
  padding: 1rem 1.25rem;
```

Find `.com-object-list li` — add to its existing block:

```css
  background: var(--knx-surface);
  border-radius: var(--knx-radius-card);
  padding: 0.75rem 1rem;
```

(This replaces its existing `border-top: 1px solid color-mix(...)` — remove that declaration since the card now has its own full border via a new hover rule below, not a top-only divider.)

Append hover behavior for both:

```css
@media (prefers-reduced-motion: no-preference) {
  .dashboard-stats,
  .com-object-list li {
    transition: transform var(--knx-transition-duration) ease,
      border-color var(--knx-transition-duration) ease,
      box-shadow var(--knx-transition-duration) ease;
  }
}

.com-object-list li:hover {
  transform: translateY(-4px);
  border-color: var(--knx-border-hover);
  box-shadow: var(--knx-glow-orange);
}
```

- [ ] **Step 3: Corner-accent border on `.dashboard-issues`**

Find `.dashboard-issues` — add to its existing block:

```css
  border-top: 1px solid var(--knx-accent);
  border-left: 1px solid var(--knx-accent);
  padding: 0.75rem 1rem;
```

- [ ] **Step 4: Toasts**

Find `.toast--error` and `.toast--fun`. In each, replace `background: var(--knx-surface);` with:

```css
  background: rgba(15, 17, 21, 0.6);
  backdrop-filter: blur(12px);
```

`.toast--error`'s existing `border: 1px solid var(--knx-error-color);` and `.toast--fun`'s existing `border: 1px solid currentColor;` stay as-is (already theme-reactive via the token/currentColor).

- [ ] **Step 5: Build and run tests**

Run: `cd apps/knx-web && npx tsc --noEmit && npx vitest run && npm run build`
Expected: all clean.

- [ ] **Step 6: Commit**

```bash
git add apps/knx-web/src/styles.css
git commit -m "feat(knx-web): glass overlays, card hover-lift, corner accents" --author="KNXBench-Labs <github@knxbench.com>"
```

---

## Task 9: `styles.css` — background texture, theme-switcher styling, dead-CSS cleanup

**Files:**
- Modify: `apps/knx-web/src/styles.css`

- [ ] **Step 1: Grid-pattern background**

Find the `body { margin: 0; background: var(--knx-bg); color: var(--knx-foreground); }` rule (Task 6 renamed `--knx-text` to `--knx-foreground` here). Append immediately after it:

```css
:root[data-theme="bitcoin-defi"] body {
  background-color: var(--knx-bg);
  background-image:
    linear-gradient(to right, rgba(30, 41, 59, 0.5) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(30, 41, 59, 0.5) 1px, transparent 1px);
  background-size: 50px 50px;
  mask-image: radial-gradient(circle at center, black 40%, transparent 100%);
}
```

- [ ] **Step 2: `.theme-switcher` styling**

Append:

```css
.theme-switcher select {
  font-family: var(--knx-font-mono);
  text-transform: none;
  letter-spacing: normal;
  min-width: 10rem;
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
```

- [ ] **Step 3: Remove dead `.theme-panel-*` rules**

Run: `grep -n "theme-panel" apps/knx-web/src/styles.css` — delete every rule block found (`.theme-panel-overlay`, `.theme-panel`, `.theme-panel-tokens`, `.theme-panel-tokens li`, `.theme-panel-motion`, `.theme-panel-actions`). `ThemePanel.tsx` was deleted in Task 3; these classes have no remaining consumer.

- [ ] **Step 4: Build and run tests**

Run: `cd apps/knx-web && npx tsc --noEmit && npx vitest run && npm run build`
Expected: all clean.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-web/src/styles.css
git commit -m "feat(knx-web): grid-pattern background, switcher styling, dead-CSS cleanup" --author="KNXBench-Labs <github@knxbench.com>"
```

---

## Task 10: Docs — ROADMAP.md, IMPLEMENTATION_STATUS.md

**Files:**
- Modify: `docs/ROADMAP.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`

- [ ] **Step 1: Read the exact current text to edit**

Read `docs/ROADMAP.md` around its "Cycle 11 ... Cycle 12+ candidates" text (Session 5 subsection) and `docs/IMPLEMENTATION_STATUS.md`'s Session 5 summary row, to quote the precise current strings before editing (both may have shifted since this plan was written, if any other cycle landed first).

- [ ] **Step 2: Add the cycle 13 entry to `docs/ROADMAP.md`**

Replace the line `Cycle 12+ candidates (from `ideas.md`, not yet scheduled).` with a paragraph describing this cycle (mirroring the style of cycles 7/11's paragraphs already in the file — named theme registry replacing the System/Light/Dark cycle and the 4-token palette override, Bitcoin DeFi as first/default theme, self-hosted + Google Fonts, component recipes applied app-wide), followed by a new `Cycle 14+ candidates (from `ideas.md`, not yet scheduled).` line.

- [ ] **Step 3: Update `docs/IMPLEMENTATION_STATUS.md`'s Session 5 row**

Append a clause for cycle 13 to the existing Session 5 summary sentence, in the same style as the existing cycle 7/8/9/10/11/12 clauses, linking `docs/superpowers/specs/2026-09-08-bitcoin-defi-theme-design.md`, and noting the file-level changes: `theme.ts` rewritten as a named-theme registry, `palette.ts`/`ThemePanel.tsx`/`ThemeToggle.tsx` deleted, `ThemeSwitcher.tsx` new, full `styles.css` restyle.

Also update the `apps/knx-web/` row's file list further down (currently lists `ThemeToggle.tsx`, references `palette.test.ts`) to match the new file set (`ThemeSwitcher.tsx`, no `palette.test.ts`), and the "`Last updated:`" line at the top of the file.

- [ ] **Step 4: Commit**

```bash
git add docs/ROADMAP.md docs/IMPLEMENTATION_STATUS.md
git commit -m "docs: cycle 13 done — Bitcoin DeFi theme" --author="KNXBench-Labs <github@knxbench.com>"
```

---

## Task 11: Manual smoke check

**Files:** none (verification only)

- [ ] **Step 1: Build and serve**

Run: `cd apps/knx-web && npm run build && npm run preview` (or `npm run dev`), open the printed URL in a browser.

- [ ] **Step 2: Verify visually**

Confirm: dark void background with visible fading grid pattern; toolbar buttons are orange-gradient pills that scale and glow brighter on hover; disabled buttons (Save/Undo/Redo with no project loaded) look visibly inert, not just dimmed-gradient; the theme `<select>` shows "Bitcoin DeFi" selected; opening Search (`Ctrl+K`) or the command palette (`Ctrl+Shift+P`) shows a blurred glass panel; with a project loaded and nothing selected, the Dashboard title renders in a gold gradient and its stat blocks look like cards with hover-lift; an inspector's address/DPT fields render in JetBrains Mono, gold-colored; reload the page and confirm the theme persists with no flash of unthemed content.

- [ ] **Step 3: Record the result**

If this environment has no display available to run this check, say so explicitly in the final report rather than claiming it passed — same as every prior UI cycle's plan (cycles 6/7/8/10/11 all left this step unperformed for the same reason).

---

## Self-Review Notes

- **Spec coverage:** registry replaces cycle (Task 1-3), palette/panel deleted (Task 3), index.html always-set attribute + Google Fonts (Task 4), self-hosted fonts (Task 5), token layer (Task 6), buttons/inputs/mono/headings (Task 7), overlays/cards/corner-accents/toasts (Task 8), background texture + switcher + dead-CSS cleanup (Task 9), docs (Task 10), manual check (Task 11) — every spec section has a task.
- **Placeholder scan:** no TBD/TODO; every CSS/TS step shows literal code, not a description of code.
- **Type consistency:** `ThemeDef`/`THEMES`/`loadThemeId`/`saveThemeId`/`useThemeId` names match exactly between Task 1's produced interface and every later task's usage (Task 2's `ThemeSwitcher` props, Task 3's `App.tsx` wiring). `--knx-*` token names introduced in Task 6 match exactly what Tasks 7-9 reference (`--knx-foreground`, `--knx-accent-tertiary`, `--knx-radius-card`, `--knx-glow-orange`, etc. — cross-checked against the Task 6 token list).
