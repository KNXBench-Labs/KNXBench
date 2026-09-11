# Design — T27: in-app motion control, two motion styles, and the rule that binds every future animation (gap D11, part of D8)

## Status and scope

**Status:** design, 2026-09-12. Decision numbering continues **D27** — the
parameter-editor design (`2026-09-11-parameter-editor-design.md`) ended at
D26. A code comment citing "design D30" means this document.

**In scope.** A user-facing motion control in `apps/knx-web` with two
independent axes — an intensity *level* and a visual *style* — its
persistence, its bootstrap, the settings surface it lives on, a mechanical
guard that keeps every future animation bound to it, and the retrofit of
the one shipped feature the roadmap names (T15's Group Monitor table).
Closes **D11**, partially addresses **D8**, and discharges the roadmap's
standing motion constraint by making it enforceable rather than merely
written down.

**Deliberately out of scope**, argued in §4: a full ETS-style options
dialog, a per-component motion category switch, any animation library
dependency, and any motion in the Tauri desktop shell beyond what it
inherits from the same `apps/knx-web` build.

## 1. The problem, stated in the code as it is today

Measured on `main` at `6112d3d`, not remembered:

- `apps/knx-web/src/styles.css` is 846 lines and contains **exactly one**
  motion token — `--knx-transition-duration: 250ms` (line 39) — and
  **exactly three** `@media (prefers-reduced-motion: no-preference)` blocks
  (lines 42, 265, 774), covering: background/color/border transitions on
  `body`, `.search-panel`, `.fs-picker`, `.tree-label.selected`,
  `.search-result.selected`; transform/border/shadow transitions on
  `.dashboard-stats` and `.com-object-list li`; and transform/shadow
  transitions on `button`.
- There are **zero** `@keyframes` rules and **zero** `animation:`
  declarations anywhere in the stylesheet.
- **No** `.ts`/`.tsx` file references motion at all: `grep -rn
  "transition\|animate\|keyframes" apps/knx-web/src/*.ts{,x}` returns
  nothing outside the stylesheet.
- Therefore the OS `prefers-reduced-motion` setting is the only control a
  user has over motion, and it is all-or-nothing. That is exactly what
  [KNOWN_LIMITATIONS.md §43](../../KNOWN_LIMITATIONS.md) and gap **D11**
  say, and it is worse than cycle 11, which shipped an `off`/`subtle`/
  `standard` setting before cycle 13's theme rewrite deleted the panel it
  lived on (`2026-09-08-bitcoin-defi-theme-design.md`: "Motion: no
  user-facing setting (that was `palette.ts`'s job, now gone)").

What already exists and should be copied rather than reinvented:

- `apps/knx-web/src/theme.ts` — a `ThemeDef`/`THEMES` registry plus
  `loadThemeId`/`saveThemeId`/`useThemeId`, storage injected as a
  parameter (`Pick<Storage, "getItem">`), unknown values falling back to a
  default, and an effect that writes `data-theme` onto
  `document.documentElement`. This is the shape the motion module copies.
- `apps/knx-web/index.html` — an inline bootstrap script that applies the
  stored theme before React mounts, avoiding a one-frame flash of the
  wrong theme. Motion needs the same treatment for the same reason.
- `apps/knx-web/src/ThemeSwitcher.tsx` — a 20-line `<label>` + `<select>`
  rendered from `App.tsx:345`. It is the entire settings surface of the
  application.

## 2. Decisions

### D27. Two axes — level and style — not one

The T27 backlog entry poses the open question as "a global duration
multiplier or a per-category switch". The answer is neither alone: one
axis for **how much** motion (`off` / `subtle` / `standard`) and one for
**what kind** (`apple` / `glitch`), because the user's recorded style memo
(ROADMAP, "Memo (2026-09-10), style direction") names two visual
directions — an Apple-like sleek/subtle/clean direction and a techy
glitch/cyberpunk-OS direction that is still clean rather than noisy — and
those are orthogonal to intensity. A user who wants the cyberpunk look at
low intensity must be able to say so.

Per-*category* switching (separate toggles for overlays, hovers, the
telegram table) is rejected for this slice: there are three transition
sites in the whole stylesheet, so a category axis would be a taxonomy
invented ahead of the thing it classifies. §4 records it as the natural
next step once animations mean "a telegram flying along a bus line".

Both axes are registry-backed, exactly like `THEMES`, so a third style —
or a fourth level — needs no UI change:

```ts
export interface MotionStyleDef { id: string; name: string; }
export const MOTION_STYLES: readonly MotionStyleDef[];   // apple, glitch
export interface MotionLevelDef { id: string; name: string; }
export const MOTION_LEVELS: readonly MotionLevelDef[];   // off, subtle, standard
```

### D28. The OS preference wins structurally, not by cascade arithmetic

`prefers-reduced-motion: reduce` must always beat the in-app choice. The
enforcement is not an `!important`, not a specificity trick, and not a
JavaScript check of `window.matchMedia` — all three can be defeated by the
next stylesheet edit.

Instead: **every motion declaration in the application stays inside a
`@media (prefers-reduced-motion: no-preference)` block**, which is already
the convention all three existing blocks follow. When the OS says
"reduce", no `transition` or `animation` property is ever emitted at all,
so there is nothing for a stored preference to override. The in-app level
only selects *which duration token applies inside that block*.

Level `off` sets the duration tokens to `0ms` rather than removing the
declarations, so an element never gets stuck mid-transition when the user
switches level while a transition is running.

### D29. Attributes and tokens

`useMotion()` applies two attributes to `<html>`, mirroring `data-theme`:

```
<html data-theme="bitcoin-defi" data-motion-level="subtle" data-motion-style="apple">
```

`styles.css` gains a token block per combination axis:

- `:root[data-motion-level="off"] { --knx-transition-duration: 0ms; }`
- `:root[data-motion-level="subtle"] { --knx-transition-duration: 120ms; }`
- `:root[data-motion-level="standard"] { --knx-transition-duration: 250ms; }`
  — 250ms stays the standard value, so today's look is the default and this
  slice changes nothing for a user who never opens the setting.
- `:root[data-motion-style="apple"] { --knx-motion-easing: cubic-bezier(0.4, 0, 0.2, 1); }`
  — a single smooth ease-out curve, no overshoot.
- `:root[data-motion-style="glitch"] { --knx-motion-easing: steps(4, end); }`
  — stepped rather than continuous, which reads as technical/digital
  without adding noise, per the memo's "still clean and sleek".

The literal `ease` keyword in the three existing blocks is replaced by
`var(--knx-motion-easing)`. `--knx-transition-duration: 250ms` moves out
of the base `:root` block into the level blocks, with the base block
keeping it as the fallback for a document that somehow has no
`data-motion-level` attribute (a stale cached `index.html`, a test
rendering without the bootstrap).

### D30. Storage, defaults, and the legacy-value rule

Two `localStorage` keys, following `theme.ts`'s existing
`knx-desktop:`-prefixed convention rather than inventing a second one:
`knx-desktop:motion-level` and `knx-desktop:motion-style`. Defaults:
`standard` and `apple` — today's behaviour, and the less surprising of the
two styles.

Anything that is not a known id resolves to the default, silently: a
missing key, a value from a future version, and cycle 11's old values. The
old setting used the same three level ids (`off`/`subtle`/`standard`), so
a user who still has cycle 11's key would get their old choice back if we
read it; we do **not** read it, because cycle 11 stored it inside a
different key that cycle 13 deleted along with `palette.ts`, and resurrecting
a two-cycle-old key to recover a setting nobody has had since is not worth
the code.

### D31. Bootstrap before React mounts

`index.html`'s existing inline script gains the same treatment for both
motion attributes: read, validate against a hard-coded id list, set the
attribute, fall back to the default. Without it, the first frame renders
at `standard` and then jumps — which is precisely the flash the theme
bootstrap exists to prevent, and worse here, since the user who set `off`
is the user least willing to watch something move.

The hard-coded id list in `index.html` duplicates the registry in
`motion.ts`. That duplication is deliberate and is the same trade
`data-theme`'s bootstrap already makes: the script must run before any
module loads. A comment in both files names the other.

### D32. The control lives on a new `SettingsPanel`, not a fourth bare `<select>`

`ThemeSwitcher.tsx`'s bare `<select>` in the toolbar cannot absorb two
more axes without the toolbar becoming a settings bar. A new
`SettingsPanel.tsx` — opened from a gear button beside the theme switcher,
dismissed by click-outside and `Escape` — hosts Theme, Motion style, and
Motion level as three labelled `<select>`s.

This is deliberately **not** D8's options dialog: no group-address style,
no backup behaviour, no language (T25's job), no persistence beyond
`localStorage`. It is the surface those settings will move into when they
exist, which is why it is a panel rather than a third inline control.

It reuses the existing overlay CSS classes rather than adding a fourth
near-duplicate overlay implementation — [KNOWN_LIMITATIONS.md §20](../../KNOWN_LIMITATIONS.md)
and gap **D9** already track Search and Command Palette being two
near-duplicates, and this slice must not make that three. It does not fix
§20 either; that stays open and is named in the slice's own limitation
text.

### D33. The constraint becomes a test, not a paragraph

The roadmap's standing rule — every animation is switchable off from
inside the application — is worth exactly as much as its enforcement. This
slice adds `apps/knx-web/src/motionGuard.test.ts`: it reads `styles.css`
from disk and asserts that every `transition:` and `animation:`
declaration in the file

1. sits inside a `@media (prefers-reduced-motion: no-preference)` block,
   and
2. takes its duration from `var(--knx-transition-duration)` rather than a
   literal.

A new animation that ignores the control then fails the frontend suite in
the same commit that introduces it, which is the only version of this rule
that survives contact with a future cycle. The test parses by brace
counting over the stylesheet text — no CSS parser dependency, and the
stylesheet is 846 lines of hand-written CSS, not generated output.

### D34. The T15 retrofit is a real animation, gated

ROADMAP names T15's Group Monitor table as "the first item T27 has to
retrofit rather than merely constrain". Measured: that table has no
animation at all today, so a literal retrofit would be a no-op and the
roadmap sentence would be discharged by doing nothing.

Instead, the telegram table gains the one piece of motion it actually
wants — a brief entry highlight on a newly arrived row, so a row that
appears while the user is reading is visibly new — implemented per style:
`apple` fades the highlight out smoothly, `glitch` steps it out. Both use
`var(--knx-transition-duration)` inside a `no-preference` block, so level
`off` and an OS `reduce` each flatten it to nothing. This makes the two
styles concretely different somewhere a user actually looks, and gives
D33's guard a second real subject.

## 3. Acceptance criteria

1. With `data-motion-level="off"`, no element transitions: the computed
   `--knx-transition-duration` is `0ms`, asserted in a test.
2. With an OS `prefers-reduced-motion: reduce`, no `transition` or
   `animation` declaration applies regardless of the stored level or style
   — guaranteed by D28's structure and asserted by D33's guard test.
3. The three settings round-trip through `localStorage`: chosen, reloaded,
   still chosen; unknown/absent values fall back to
   `bitcoin-defi`/`standard`/`apple`.
4. The bootstrap script sets both motion attributes before React mounts,
   for a stored value and for a garbage value.
5. `SettingsPanel` opens from the gear button, closes on `Escape` and on
   click-outside, and changing any of its three `<select>`s applies
   immediately with no reload.
6. A newly arrived telegram row is visually marked as new under both
   styles at levels `subtle`/`standard`, and not marked at level `off`.
7. `motionGuard.test.ts` fails if a `transition:` is added outside a
   `no-preference` block or with a literal duration — demonstrated by the
   test's own fixture, not merely asserted in prose.
8. Both gate suites stay green with no count regression: Rust **975
   passed / 0 failed / 3 ignored**, web **184 passed** before this slice.

## 4. What this slice deliberately does not do

- **No options dialog (D8 stays open).** Group-address style, backup
  behaviour and language are not settings yet, and inventing storage for
  them here would pre-empt T25's own design question about where a
  language setting lives.
- **No per-category motion switch.** Named in D27 as the natural next step
  once T17/T21's motion-heavy views exist. Three transition sites do not
  justify a taxonomy.
- **No animation library.** Same reasoning as cycle 13's hand-written SVG
  icons: the dependency would exceed the feature.
- **No fix for §20/D9's duplicate overlay implementations.** The new panel
  reuses the existing overlay CSS so the count of near-duplicate
  implementations does not grow, but the underlying duplication and its
  accessibility gap stay open and stay documented.
- **No motion in server, CLI or Rust code of any kind.** This slice is
  `apps/knx-web` plus documentation.

## 5. Task cut

Five tasks, each independently reviewable:

1. `motion.ts` — registries, `loadMotion*`/`saveMotion*`, `useMotion()`,
   unit tests mirroring `theme.test.ts`.
2. `styles.css` token layer (D29) + `index.html` bootstrap (D31) +
   `motionGuard.test.ts` (D33).
3. `SettingsPanel.tsx` (D32) + its tests + `App.tsx` wiring.
4. Telegram-table new-row retrofit (D34) + tests.
5. Documentation reconciliation: `KNOWN_LIMITATIONS.md` §43,
   `GAP_ANALYSIS_ETS.md` D11/D8/T27, `ROADMAP.md`'s motion section and its
   style memo, `IMPLEMENTATION_STATUS.md`, and the `.ai/logs/` entry.
