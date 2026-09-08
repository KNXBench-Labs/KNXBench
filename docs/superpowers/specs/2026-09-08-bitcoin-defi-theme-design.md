# Session 5, cycle 13: Bitcoin DeFi theme — design

**Status.** Approved, ready for implementation planning.

## Context

Cycle 11 (no dedicated design spec — described inline in ROADMAP.md's
cycle-11 entry) shipped a four-token color override (`palette.ts`)
layered on cycle 7's System/Light/Dark cycle (`theme.ts`,
`docs/superpowers/specs/2026-09-04-dark-light-mode-design.md`). This cycle replaces both with a different concept: a
**named, selectable theme** — a complete visual package (colors,
typography, radii, shadows, texture), not a per-user color tweak on top
of a light/dark base. "Bitcoin DeFi" is the first theme and today's
default; more are expected later, so the registry is built for N even
though it holds one entry now.

`apps/knx-desktop` has no UI code of its own — `docs/ARCHITECTURE.md`
§3: it's a thin Tauri wrapper that spawns `knx-server` locally and
points a WebView at it, no `#[tauri::command]` handlers, no styling.
`apps/knx-web` is the one frontend, served to both the desktop WebView
and the browser/Docker deployment target identically. This cycle touches
only `apps/knx-web` — "native and web UI identical" falls out of the
existing architecture for free, not something this cycle has to
engineer.

**Current state** (`apps/knx-web/src`):

- `theme.ts` / `ThemeToggle.tsx`: System/Light/Dark cycle, `data-theme`
  attribute on `<html>`, `localStorage` key `knx-desktop:theme`.
- `palette.ts` / `ThemePanel.tsx`: four freely-colorable tokens
  (`accent`/`bg`/`surface`/`text`) plus a three-level motion setting,
  applied as inline CSS custom properties on `<html>`, `localStorage`
  key `knx-desktop:palette`. Opened from a gear button in the toolbar.
- `styles.css`: system color keywords (`AccentColor`/`Canvas`/
  `CanvasText`) as the unthemed base, three literal-color tokens
  (`--knx-error-color`, `--knx-overlay-backdrop`, `--knx-overlay-shadow`)
  themed per cycle 7, no typography/radius/shadow system at all —
  `font-family: system-ui, sans-serif` is the only font rule in the
  file.
- No Content-Security-Policy anywhere (`tauri.conf.json` sets none,
  `knx-server` sets no security-header middleware) — an external
  `<link>` to Google Fonts is unblocked in both deployment targets.
- `index.html` has a small inline script applying a persisted theme
  choice to `<html data-theme>` before React mounts, closing a
  one-frame flash-of-wrong-theme window.

**Scope decisions:**

- Theme replaces the System/Light/Dark cycle outright, not layered on
  top of it. Bitcoin DeFi is dark-only by design (the source design
  system is explicit: "Dark Mode Only") — a future light-mode-capable
  theme would express that as a second registry entry, or as its own
  internal light/dark pair, not by resurrecting the old cycle. User
  confirmed this is the intended trade — Bitcoin DeFi is optimized for
  dark, not built to also do light.
- `palette.ts`/`ThemePanel.tsx` are deleted outright, not migrated. A
  4-token override doesn't map onto an 12+-token theme package
  (background/surface/foreground/muted/border/three accents/two
  gradients/three font stacks/three radius scales/two glow shadows).
  User-facing customization within a theme is future scope, revisited
  once there's more than one theme to customize on top of.
- No Tailwind, no `cva`, no component-kit dependency. The source design
  system's "Implementation Notes" assume a Tailwind/shadcn stack; this
  project has plain CSS with a `--knx-*` custom-property convention
  (CLAUDE.md: avoid unnecessary dependencies/frameworks; match existing
  patterns). Every design-system token becomes a CSS custom property,
  every component recipe becomes a class in `styles.css`, same shape as
  every prior cycle's CSS.
- Landing-page-only elements from the source design system — spinning
  orbital hero, pricing-tier cards, blockchain timeline, bouncing stat
  cards — have no target in this app (a data-dense project editor: tree
  views, forms, search, dashboard) and are not built. What *does* carry
  over: pill buttons with gradient/glow, glass-morphism overlays, card
  hover-lift with colored glow, monospace technical text (the app
  already sets addresses/DPTs in `monospace` — this cycle just points
  that at JetBrains Mono and gives it accent color), gradient text on
  the one heading that reads as a hero moment (`Dashboard.tsx`'s
  title), the fading grid-pattern background, corner-accent borders on
  the closest analog to the source's "How It Works" cards
  (`Dashboard.tsx`'s stat/issue blocks).
- Fonts: self-hosted (`@fontsource/space-grotesk`,
  `@fontsource/inter`, `@fontsource/jetbrains-mono`) is the guaranteed
  fallback — always available offline, one `import` per family in
  `main.tsx`. A Google Fonts `<link>` is added in `index.html`
  alongside it per the user's request for a CDN-first look. Whichever
  of the two stylesheets registers a given family/weight's `@font-face`
  last in the CSSOM is the one the browser actually uses to render —
  load order between a static `<head>` `<link>` and a JS-module-driven
  import isn't something this design pins down, so which of the two
  "wins" on any given load is not guaranteed. Not a real problem: it's
  the same typeface either way, imperceptible. True network-first-then-
  local-fallback would need hardcoded `fonts.gstatic.com` URLs inside a
  hand-written `@font-face` `src` list (the only mechanism with a
  defined fallback order) — rejected as fragile complexity for zero
  visible difference. Documented and confirmed with the user rather
  than silently simplified.
- Motion: no user-facing setting (that was `palette.ts`'s job, now
  gone). `prefers-reduced-motion: no-preference` continues to gate all
  transitions/animations, same as before — the OS setting is the only
  control, which is the existing pattern for `--knx-transition-duration`
  and is where the new CSS animations (button hover, card lift) hook in
  too.

## Backend

None. Frontend-only, like cycles 6, 7, 8, 10, 11.

## Frontend

### `theme.ts` (rewritten) — registry, not a cycle

```ts
export interface ThemeDef {
  id: string;
  name: string;
}

export const THEMES: readonly ThemeDef[] = [
  { id: "bitcoin-defi", name: "Bitcoin DeFi" },
];

const DEFAULT_THEME_ID = "bitcoin-defi";
const STORAGE_KEY = "knx-desktop:theme";

function isThemeId(id: string): boolean {
  return THEMES.some((t) => t.id === id);
}

/**
 * Reads the persisted theme id. Anything that isn't a known theme id —
 * missing key, or a value from a future/incompatible version, or one
 * whose theme has since been removed — resolves to the default, the
 * same always-safe-default philosophy as the old `loadTheme`.
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
 * data-theme>`, and persists on every change.
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

Reuses the `knx-desktop:theme` storage key outright — cycle 7's stored
values (`"system"`/`"light"`/`"dark"`) all fail `isThemeId`, so an
existing install falls through to the default `"bitcoin-defi"` cleanly,
no migration code needed. `palette.ts`'s `knx-desktop:palette` key is
simply never read again after this cycle — orphaned, harmless.

### `index.html`

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

Same duplication rationale as cycle 7's version: this runs before any
module graph is available, so the known-id list is a literal here, kept
in sync by hand with `theme.ts`'s `THEMES` array. Unlike cycle 7,
`data-theme` is now *always* set (never removed) — there is no
"system"-equivalent fallback state anymore, so every code path,
including first-ever load with nothing in storage, needs the attribute
present before paint to avoid a flash of unthemed (system-color)
content.

Google Fonts, added to `<head>`:

```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link
  href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap"
  rel="stylesheet"
/>
```

### `main.tsx` — self-hosted font imports

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

New `package.json` dependencies: `@fontsource/space-grotesk`,
`@fontsource/inter`, `@fontsource/jetbrains-mono`.

### `ThemeSwitcher.tsx` (new, replaces `ThemeToggle.tsx`)

A `<select>`-based switcher, not a cycle button — a cycle stops making
sense once the list can grow past 2-3 entries. Built against `THEMES`
so a second entry needs no UI change:

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

`ThemeToggle.tsx` and its three inline SVG icons are deleted — a
`<select>` needs no icon.

### `App.tsx` wiring

- `const [themeId, setThemeId] = useThemeId();` replaces `const [theme,
  cycleTheme] = useTheme();`.
- `const [paletteSettings, ...] = usePalette();` and `themePanelOpen`
  state are deleted, along with the gear button and the `<ThemePanel>`
  render block.
- `<ThemeSwitcher themes={THEMES} activeId={themeId}
  onSelect={setThemeId} />` replaces `<ThemeToggle theme={theme}
  onCycle={cycleTheme} />` in the toolbar, same position.

### `styles.css` — token layer

No bare-`:root` fallback tokens (unlike cycle 7's, which needed one for
the genuine "system/unthemed" state). There is no unthemed state
anymore — `index.html`'s inline script always sets `data-theme`
synchronously before paint — so every token lives inside the
`[data-theme="bitcoin-defi"]` block:

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

  /* Feedback (colored, not black — "no pure black shadows" per source spec) */
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

Every existing consumer of `--knx-accent`/`--knx-bg`/`--knx-surface`
(`color-mix(in srgb, var(--knx-accent) 20%, transparent)` for tree/
search selection highlights, `body { background/color }`) picks up the
new palette automatically, no per-rule change needed there.
`--knx-text`, the old palette.ts token name, is renamed
`--knx-foreground` to match the source design system's naming — every
`var(--knx-text)` reference in `styles.css` (search/fs-picker panels,
etc.) is updated to `var(--knx-foreground)` in the same pass.

### `styles.css` — component recipes

- **Buttons** (all bare `<button>` toolbar elements in `App.tsx`, plus
  panel action buttons): pill-shaped (`--knx-radius-button`),
  `background: var(--knx-gradient-primary)`, white bold uppercase text,
  `box-shadow: var(--knx-glow-orange)`, hover
  `transform: scale(1.05)` + `var(--knx-glow-orange-strong)`,
  `disabled` drops the gradient to flat `var(--knx-surface)` at reduced
  opacity (disabled buttons — Save/Undo/Redo when unavailable — must
  stay visibly inert, not just less shiny).
- **Overlays** (`.search-panel`, `.fs-picker`, `.theme-panel`'s
  successor if any float remains, `.search-overlay`/`.fs-picker-overlay`
  backdrops): glass morphism — `background: rgba(15, 17, 21, 0.6)`,
  `backdrop-filter: blur(12px)`, `border: 1px solid var(--knx-border)`,
  `box-shadow: 0 4px 24px var(--knx-overlay-shadow)`.
- **Cards** (`.dashboard-stats`, `.com-object-list li`,
  `.dashboard-issues`): `background: var(--knx-surface)`,
  `border: 1px solid var(--knx-border)`,
  `border-radius: var(--knx-radius-card)`, hover
  `translateY(-4px)` + `border-color: var(--knx-border-hover)` +
  `box-shadow: var(--knx-glow-orange)`, transition gated under
  `prefers-reduced-motion: no-preference` same as today.
- **Inputs** (`.search-panel input/select`, `.fs-picker` filter,
  `.inspector-field input`, `.tree-new-row input`,
  `.catalog-create-row input`, `.theme-switcher select`):
  `background: rgba(0, 0, 0, 0.5)`,
  `border: none`, `border-bottom: 2px solid var(--knx-border)`,
  `font-family: var(--knx-font-mono)`, focus
  `border-bottom-color: var(--knx-accent)` +
  `box-shadow: 0 10px 20px -10px rgba(247, 147, 26, 0.3)`, no
  `outline`.
- **Technical/mono text** (`.inspector-address`, `.com-object-label`
  addresses, `.provenance-badge`, DPT labels): `font-family:
  var(--knx-font-mono)`, colored `var(--knx-accent-tertiary)` (gold) —
  the app already sets these `monospace`, this cycle just points that
  family variable at JetBrains Mono and adds accent color, no structural
  change.
- **Headings** (`h1`-`h6` app-wide, `Dashboard.tsx`'s title
  specifically): `font-family: var(--knx-font-heading)`,
  `font-weight: 600`. `Dashboard.tsx`'s title only —
  `background: var(--knx-gradient-gold); -webkit-background-clip: text;
  color: transparent;` (gradient text), since it's the one heading in
  the app that reads as a "hero" moment (first thing shown once a
  project loads with nothing selected); every other heading (Inspector
  field labels, panel titles) stays plain `--knx-foreground` — gradient
  text on every heading would cheapen the one place it's meant to land.
- **Background texture**: `body` gets the source design system's
  signature fading grid pattern —
  ```css
  body {
    background-image:
      linear-gradient(to right, rgba(30, 41, 59, 0.5) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(30, 41, 59, 0.5) 1px, transparent 1px);
    background-size: 50px 50px;
    mask-image: radial-gradient(circle at center, black 40%, transparent 100%);
  }
  ```
  applied under `:root[data-theme="bitcoin-defi"] body` so a future
  non-grid theme doesn't inherit it.
- **Corner-accent borders**: `.dashboard-issues` (the closest analog to
  the source's "How It Works" cards — a block presenting a finding, one
  per issue category) gets `border-top: 1px solid var(--knx-accent);
  border-left: 1px solid var(--knx-accent);` on the block itself, no new
  markup.
- **Toasts** (`.toast--error`, `.toast--fun`): glass background matching
  the overlay recipe above, `border-color` swapped to
  `var(--knx-error-color)`/`var(--knx-accent)` respectively — otherwise
  unchanged from cycle 10's structure.
- **Cleanup**: `.theme-panel-overlay`, `.theme-panel`,
  `.theme-panel-tokens`, `.theme-panel-motion`, `.theme-panel-actions`
  rules are deleted from `styles.css` along with `ThemePanel.tsx` —
  dead CSS otherwise.

## Testing

`theme.test.ts` rewritten for the id-based registry (replaces the old
System/Light/Dark cases):

- `loadThemeId`: a fake storage returning `null` resolves to
  `"bitcoin-defi"`; returning `"bitcoin-defi"` resolves unchanged;
  returning a garbage/unknown id (including cycle 7's old
  `"system"`/`"light"`/`"dark"` values, proving the silent-fallback
  migration path) resolves to `"bitcoin-defi"`.
- `saveThemeId`: calls the fake storage's `setItem` with the exact key
  and value.

No test for `useThemeId`, `ThemeSwitcher.tsx`, or the `App.tsx`/
`index.html` wiring — same DOM-touching rationale as every prior cycle
touching `theme.ts`.

`palette.test.ts` is deleted outright (its module is deleted).

No test covers `styles.css` itself — same as every prior styling cycle,
this project has no visual-regression tooling. Manual smoke check (load
the app, confirm the grid background/gradient buttons/glass overlays
render, open Search and the command palette to confirm glass + glow,
confirm Dashboard's gradient-text title, resize to confirm nothing
breaks at the existing responsive breakpoints if any) is left
unperformed here — no display available in this environment — and
should be run before, or at, merge.
