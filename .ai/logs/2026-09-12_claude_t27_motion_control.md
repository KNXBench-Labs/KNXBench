# 2026-09-12 — T27: an in-app motion control, and the rule it now enforces

Architecture log for the T27 cycle, branch `t27-motion-control`, seven
commits from `bb43da4` (`de78253`..`b2aac7d`) plus this docs-reconciliation
commit — 12 files under `apps/knx-web/`, 905 insertions, 62 deletions
before this task's docs-only commit.

Design: `docs/superpowers/specs/2026-09-12-motion-control-design.md`
(decisions D27-D34).
Plan: `docs/superpowers/plans/2026-09-12-motion-control.md`.

## What changed architecturally

One new state module, one CSS token layer it drives, one guard test that
turns a standing constraint into an enforceable one, one settings surface
that absorbs a component it replaces, and one retrofit of a feature that
shipped ahead of all of it.

### 1. `motion.ts` — two independent axes, not one

`apps/knx-web/src/motion.ts` (task 1, `96eeff6`) is modeled on `theme.ts`'s
`Pick<Storage, ...>`-injection style and always-safe-default philosophy,
but exposes two registries instead of one: `MOTION_LEVELS`
(`off`/`subtle`/`standard`) and `MOTION_STYLES` (`apple`/`glitch`). The
design's own open question — a global duration multiplier or a
per-category switch — was answered with neither: two orthogonal axes,
intensity and feel, because the 2026-09-10 style memo named two
independent visual directions rather than asking for finer targeting.
`useMotion()` returns `{level, setLevel, style, setStyle}` and deliberately
never calls `window.matchMedia` — a comment in the file explains why:
`prefers-reduced-motion` stays enforced in CSS alone, so nothing in
TypeScript can ever be tempted to override it.

### 2. `styles.css` — a token layer, and a structural OS-wins rule

Task 2 (`2313c8c`) adds `--knx-transition-duration`
(`0ms`/`120ms`/`250ms` per level) and `--knx-motion-easing` (a
cubic-bezier ease for `apple`, `steps(4, end)` for `glitch`) to the
existing `:root[data-theme="bitcoin-defi"]` block, selected by
`data-motion-level`/`data-motion-style` attributes that `index.html`'s
pre-mount bootstrap script applies before React ever renders (the script
hard-codes the same id lists as `motion.ts`, since it runs before any
module loads — cross-commented in both files as a duplication kept in
sync by hand, not an oversight).

### 3. `motionGuard.test.ts` — the constraint becomes a test

`docs/ROADMAP.md`'s "Cross-cutting — Motion and animation" section has
said, since 2026-09-10, that every animation this application ships must
be switchable off in-app with the OS always winning. Task 2 also adds a
brace-counting checker over `styles.css`'s text that fails the suite if a
`transition:`/`animation:` declaration sits outside a
`@media (prefers-reduced-motion: no-preference)` block or uses a literal
duration. Verified with teeth: a literal `200ms ease` injected into
`styles.css` made it fail, naming the line.

That guard nearly shipped disarmed twice, both caught on this branch
rather than after:

- **The `?raw` trap.** The tidier-looking alternative to reading
  `styles.css` with `node:fs` — Vite's `import css from
  "./styles.css?raw"` — type-checks, runs, and passes, against the
  **empty string**, because Vitest does not process CSS by default. Tried
  deliberately, caught by injecting the same literal `200ms` and watching
  the suite stay green regardless. Recorded in
  `KNOWN_LIMITATIONS.md` §43 so a future "cleanup" doesn't reintroduce it.
- **The production-build break.** `node:fs`/`node:url`/`node:path` type-
  check fine under Vitest but fail `npm run build` (`tsc && vite build`,
  `include: ["src"]`, no `@types/node`) with TS2591 — six green gates
  including `npm run test`, and the one thing anybody would actually ship
  was broken. Caught (`056b4a0`) only because this slice added a seventh
  gate, `tsc --noEmit`, specifically because `npm run test` alone would
  not have caught it. Fixed with a 27-line `node-builtins.d.ts` declaring
  exactly the four functions used, rather than adding `@types/node` (the
  slice's no-new-dependency rule, and a wider type surface would have let
  a component import `node:fs` unnoticed).

### 4. `SettingsPanel.tsx` replaces `ThemeSwitcher.tsx`, not beside it

Task 3 (`0dd8029`) adds a gear-button overlay reusing
`Search.tsx`/`CommandPalette.tsx`'s `.search-overlay`/`.search-panel`
shape and click-outside pattern verbatim — no fourth overlay
*implementation*, but a fourth *consumer* of that shared CSS with no
shared component behind it (`CatalogBrowser.tsx`, from T2, was already a
third, independent of this slice — `GAP_ANALYSIS_ETS.md` row **D9**
undercounted before T27 even started, now corrected). `ThemeSwitcher.tsx`
is deleted outright, its one consumer (the toolbar's always-visible theme
`<select>`) folded into the panel rather than kept alongside it — one
settings entry point, not two.

A coordinator follow-up (`0604180`) removed the now-orphaned
`.theme-switcher select` CSS rule and left a comment recording that
`.sr-only` survives despite also losing its only consumer: it is the
standard visually-hidden-label utility, and `KNOWN_LIMITATIONS.md` §20's
still-open accessibility work will want it rather than reinvent it.

### 5. The retrofit the roadmap's own text predicted

`docs/ROADMAP.md` named T15's Group Monitor table, shipped 2026-09-11
ahead of this control, as "the first item T27 has to retrofit rather than
merely constrain." Task 4 (`b2aac7d`) does that: a `newRowThreshold` state
in `BusMonitorPanel.tsx` marks rows from the most recent poll with a
`bus-monitor-row-new` class, paired with a new
`@keyframes bus-monitor-row-new-highlight` inside the same
`no-preference` block. The threshold resets on *every* poll tick,
including one that returns zero telegrams, so the highlight lasts exactly
one poll interval and never accumulates — no timer, no per-row React
state, no reading of `data-motion-*`/`matchMedia` in TypeScript.

## What this slice deliberately does not do

`KNOWN_LIMITATIONS.md` §43, rewritten rather than closed outright, is the
full accounting. The headline five: no per-category control (one
duration/easing pair for the whole application); the guard reads
`styles.css` only, nothing else; the guard matches the `transition:`/
`animation:` shorthands only, not longhands like `animation-duration`;
jsdom runs no test that actually observes motion, only class/attribute
assertions; and the `node:fs`/`?raw` trap above, which a future "cleanup"
could still reintroduce if this log and §43 are both forgotten.

No claim of ETS parity or KNX certification is made anywhere in this
slice — ETS has no comparable motion control, so there is no parity claim
to make in either direction.

## Verification

Gates, all seven, foreground, one at a time, on this branch:

- `cargo fmt --all --check` — clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo test --workspace` — **975 passed / 0 failed / 3 ignored** across
  72 `test result` lines (unchanged from the branch's pre-T27 baseline —
  no Rust file touched anywhere in this slice)
- `cargo run -p xtask -- check-layering` — ok
- `cargo deny check` — advisories ok, bans ok, licenses ok, sources ok
- `npm run test` (`apps/knx-web`, `vitest run`) — **215 passed across 21
  files** (up from 184/18 before this slice)
- `npx tsc -p apps/knx-web/tsconfig.json --noEmit` — clean (the seventh
  gate, added mid-slice after the production-build break above)

Documentation reconciled in this same task (docs-only, no code touched):
`KNOWN_LIMITATIONS.md` §43 rewritten in full; `GAP_ANALYSIS_ETS.md` rows
D8 (partially addressed), D9 (corrected to four overlay-CSS consumers,
stays open), D11 (closed), and the Tier 7 T27 backlog entry (done);
`ROADMAP.md`'s "Cross-cutting — Motion and animation" section and its
2026-09-10 style memo (answered: both directions shipped); a new dated
entry in `IMPLEMENTATION_STATUS.md`; and every `ThemeSwitcher.tsx`
mention in a current-state document corrected to say it is gone. Dated
specs and plans under `docs/superpowers/` — including this slice's own —
were left untouched as historical record, per project convention.
