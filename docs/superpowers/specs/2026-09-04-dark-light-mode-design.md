# Session 5, cycle 7: dark/light mode — design

**Status.** Approved, ready for implementation planning.

## Context

Cycle 6 (`docs/superpowers/specs/2026-09-04-command-palette-design.md`)
delivered the command palette. Dark/light mode is the last UI/UX
deliverable CLAUDE.md and ROADMAP.md's Session 5 list that isn't shipped
yet. This cycle is that: an explicit theme toggle, on top of the
system-following behavior the app already has by accident.

**Current state** (`apps/knx-desktop`):

- `styles.css:1-2` already declares `:root { color-scheme: light dark; }`
  — the app currently follows the OS/webview color-scheme preference
  automatically, with no way to override it. Elements using system color
  keywords (`Canvas`/`CanvasText`, `search-panel`) and `currentColor`-
  relative rules (`color-mix(in srgb, currentColor 15%, transparent)`,
  used for selection highlights and borders) already adapt to whichever
  scheme is in effect.
- Three rules use literal, theme-blind colors that do **not** adapt:
  `.error-banner`/`.field-error`/`.import-errors` (`#b00020`),
  `.search-overlay` (`rgba(0, 0, 0, 0.4)` backdrop), and `.search-panel`
  (`rgba(0, 0, 0, 0.3)` box-shadow).
- No persistence mechanism exists anywhere in the frontend — no
  `localStorage` use, no Tauri store plugin, no settings file. No theme
  state, no toggle UI.
- `apps/knx-desktop/src-tauri/tauri.conf.json` sets no Content-Security-
  Policy — Tauri's default is unrestricted, so an inline `<script>` in
  `index.html` is unblocked.
- `vitest.config.ts` runs tests under `environment: "node"` — no `window`/
  `document`/`localStorage` globals available in tests, matching the
  project's existing split between pure-logic unit tests
  (`searchMatch.test.ts`, `commandRegistry.test.ts`) and untested
  DOM-touching components (`Search.tsx`, `CommandPalette.tsx`).

**Scope decisions:**

- Three theme states — System, Light, Dark — cycled by repeated clicks on
  one toggle button, not two independent controls. System stays reachable
  after an explicit choice (clicking around the cycle), so a user who
  picks Light or Dark isn't permanently opted out of following the OS.
- Persistence is `localStorage`, not a new Tauri plugin or a field on
  `.knxdb`/`AppState`. Theme is a UI preference, not project/domain data
  — it does not belong in the KNX domain model (CLAUDE.md: keep the
  domain core free of UI concerns), and the project has no existing
  persistence plugin to build on, so `localStorage` is the minimal
  dependency-free option.
- The toggle is an icon button (sun/moon/monitor), not a text label —
  the user explicitly asked for a themeable UI with icons. The three
  icons are inline SVG in `ThemeToggle.tsx`, not an icon library
  dependency: three fixed glyphs don't justify a new package (CLAUDE.md:
  avoid unnecessary dependencies).
- Only the three literal, non-adapting colors get CSS custom properties.
  Everything already using `currentColor`/system color keywords needs no
  change — it already reacts to `color-scheme`. This is not a full design-
  token system; it's the minimum that makes "themeable" true across the
  whole UI rather than just around the toggle button itself.
- No flash-of-wrong-theme handling beyond a small inline script in
  `index.html`'s `<head>` that reads `localStorage` synchronously before
  React mounts. No hydration mismatch is possible (this is a client-only
  SPA, not SSR), so the risk is purely a same-session flash on load,
  which the inline script closes.

## Backend

None. This cycle is frontend-only, like cycle 6.

## Frontend

### `theme.ts` (new) — state, persistence, pure cycling logic

```ts
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

`nextTheme`/`loadTheme`/`saveTheme` are pure and take an injected
storage-like object rather than reading `window.localStorage` directly —
this is what makes them testable under `vitest`'s `environment: "node"`
without a `localStorage` global or a DOM. `useTheme` itself is not unit
tested, for the same reason `Search.tsx`/`CommandPalette.tsx` aren't: it
touches `window`/`document`, and this project has no component-testing
library.

### `index.html` — inline script, before React mounts

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

Placed in `<head>`, before `styles.css` would otherwise paint with the
wrong scheme for one frame. Deliberately duplicates `loadTheme`'s literal
strings (`STORAGE_KEY`, the two valid values) rather than importing from
`theme.ts` — this script runs before any module graph is available, and
it is intentionally the only place these three literals appear twice;
`theme.ts`'s `STORAGE_KEY` constant and this script's string must be kept
in sync by hand if either ever changes.

### `ThemeToggle.tsx` (new)

```tsx
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

All three icons use `currentColor` for their strokes/fills — no
theme-specific icon variant is needed, they inherit the button's text
color the same way every other icon-less button already inherits
`CanvasText`/`currentColor`. `ThemeToggle.tsx` needs
`import type { Theme } from "./theme";`; `React.ReactElement` needs no
import of its own — `ProjectExplorer.tsx:12` already references
`React.ReactNode` the same way, unimported, via the global `React`
namespace `@types/react` provides.

### `App.tsx` wiring

- `const [theme, cycleTheme] = useTheme();` alongside the other top-level
  state.
- `<ThemeToggle theme={theme} onCycle={cycleTheme} />` added to the
  toolbar, after the existing "Commands… (Ctrl+Shift+P)" button.
- No keyboard shortcut for this cycle — the toggle is click-only. (Cycle
  6 added `Ctrl+Shift+P`; a shortcut for theme-cycling isn't requested
  and isn't an obvious keystroke to reserve, so it's left out rather than
  guessed at.)

### `styles.css`

Three new custom properties at `:root` (light defaults, matching today's
literal values exactly, so System/unthemed behavior is visually
unchanged), a `prefers-color-scheme: dark` override block, and two
`:root[data-theme="..."]` override blocks that also set `color-scheme`
explicitly (so native form controls and `Canvas`/`CanvasText` follow the
explicit override too, not just these three tokens):

```css
:root {
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

`:root[data-theme="dark"]` outranks the plain `@media` block regardless
of source order (attribute selector on `:root` is more specific than a
bare `:root` inside a media query), so an explicit override always wins
over the OS preference, which is the required behavior.

`.error-banner`, `.field-error`, `.project-explorer footer
.import-errors` switch from `#b00020` to `var(--knx-error-color)`.
`.search-overlay`'s `background` switches from `rgba(0, 0, 0, 0.4)` to
`var(--knx-overlay-backdrop)`. `.search-panel`'s `box-shadow` switches
its color component from `rgba(0, 0, 0, 0.3)` to `var(--knx-overlay-
shadow)`. The exact dark-mode values (`#ff6b6b`, `0.6`, `0.5`) are a
starting point, not a measured contrast-ratio result — cheap to retune
later, not an architectural decision.

## Testing

`theme.test.ts` (Vitest, alongside `commandRegistry.test.ts`):

- `nextTheme`: `"system" -> "light" -> "dark" -> "system"`, verified as a
  full cycle back to the start.
- `loadTheme`: a fake storage returning `null` (nothing stored) resolves
  to `"system"`; returning `"light"`/`"dark"` resolves to that value
  unchanged; returning an arbitrary/garbage string (e.g. `"solarized"`)
  resolves to `"system"`.
- `saveTheme`: calls the fake storage's `setItem` with the exact key and
  value.

No test for `useTheme`, `ThemeToggle.tsx`, the `App.tsx` wiring, or the
`index.html` inline script — all touch `window`/`document`/the DOM, and
this project has no component-testing library (see Context). Manual
smoke check (open the app, click through all three theme states, confirm
the error banner/search overlay are legible in both Light and Dark,
restart the app and confirm the persisted choice survives) is left
unperformed here for the same reason cycles 4-6 left theirs unperformed
— no display available in this environment. Should be run before, or at,
merge.
