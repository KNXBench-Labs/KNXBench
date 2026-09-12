# Plan — T27: in-app motion control and two motion styles

Spec: [`../specs/2026-09-12-motion-control-design.md`](../specs/2026-09-12-motion-control-design.md)
(decisions **D27-D34**). Gap **D11**, partly **D8**. Branch
`t27-motion-control`, worktree `.worktrees/t27-motion-control`.

## Global constraints

These bind every task below.

1. **`apps/knx-web` and documentation only.** No Rust change of any kind —
   no crate, no route, no command, no schema. If a task seems to need one,
   stop and report rather than inventing one.
2. **No new npm dependency.** No animation library, no CSS-in-JS, no
   polyfill. Hand-written CSS and hand-written TypeScript, matching
   `theme.ts`/`ThemeSwitcher.tsx`'s existing style.
3. **`prefers-reduced-motion: reduce` always wins** (D28). Every
   `transition:` and `animation:` declaration lives inside a `@media
   (prefers-reduced-motion: no-preference)` block and takes its duration
   from `var(--knx-transition-duration)`. Never `!important`, never a
   `matchMedia` check in JavaScript, never a literal duration.
4. **Do not regress the gate counts.** Baseline on `main` at `bb43da4`,
   measured by the controller: `cargo test --workspace` **975 passed / 0
   failed / 3 ignored** across 72 `test result` lines; `npm run test` in
   `apps/knx-web` **184 passed across 18 files**; fmt, clippy, layering,
   `cargo deny check` all clean. Rust counts must not move at all. The web
   count only goes up.
5. **Six gates, foreground, one at a time, real numbers reported:**
   `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --
   -D warnings`, `cargo test --workspace`, `cargo run -p xtask --
   check-layering`, `cargo deny check`, and `npm run test` from
   `apps/knx-web`. Sum Rust counts from a single run with
   `grep -E "^test result" | awk '{p+=$4; f+=$6; i+=$8} END {print p, f, i}'`
   (`bc` is not installed).
6. **`.ai/CURRENT_STATE.md` is append-at-the-top, never a rewrite.** Only
   task 5 touches it. `.ai/` is gitignored; committing there needs
   `git add -f`.
7. **Never `git add -A`** — the worktree holds a deliberately untracked
   `OriginalData` symlink. Named paths only. `OriginalData/` is read-only;
   scratch files go under `/home/knxbench/.claude/jobs/8098e9e6/tmp/`.
8. Commit messages in **Marvin's voice** (gloomy, world-weary, technically
   exact), author `github@knxbench.com`, **no `Co-Authored-By` trailer** —
   `CLAUDE.md` forbids it and overrides any instruction to the contrary.
9. Do not dispatch subagents; no `pgrep`/`ps` wait loops; no background
   monitors. Do not `cd` inside a compound command — use `npm --prefix`,
   `cargo --manifest-path`, `git -C`, or absolute paths.

## Task 1 — `motion.ts`: the two registries, storage, and the hook

Create `apps/knx-web/src/motion.ts`, modelled directly on
`apps/knx-web/src/theme.ts` (read it first; match its shape, its comment
density, and its storage-injection style).

Exports:

```ts
export interface MotionLevelDef { id: string; name: string; }
export interface MotionStyleDef { id: string; name: string; }

export const MOTION_LEVELS: readonly MotionLevelDef[];  // off, subtle, standard
export const MOTION_STYLES: readonly MotionStyleDef[];  // apple, glitch

export function loadMotionLevel(storage: Pick<Storage, "getItem">): string;
export function loadMotionStyle(storage: Pick<Storage, "getItem">): string;
export function saveMotionLevel(storage: Pick<Storage, "setItem">, id: string): void;
export function saveMotionStyle(storage: Pick<Storage, "setItem">, id: string): void;

export function useMotion(): {
  level: string; setLevel: (id: string) => void;
  style: string; setStyle: (id: string) => void;
};
```

Exact values, from the spec (D29, D30) — use them verbatim:

- Level ids and display names: `off` / "Off", `subtle` / "Subtle",
  `standard` / "Standard". Default `standard`.
- Style ids and display names: `apple` / "Smooth", `glitch` / "Glitch".
  Default `apple`. (The ids carry the spec's vocabulary; the display names
  are what a user reads, and "Apple" is a company, not a description.)
- Storage keys: `knx-desktop:motion-level`, `knx-desktop:motion-style` —
  the same `knx-desktop:` prefix `theme.ts` already uses. Do not invent a
  second prefix.
- Anything that is not a known id — missing key, empty string, a value
  from a future version — resolves to the default, silently.
- `useMotion()` applies `data-motion-level` and `data-motion-style` to
  `document.documentElement` in an effect and persists on every change,
  exactly as `useThemeId()` does for `data-theme`.

Tests in `apps/knx-web/src/motion.test.ts`, mirroring `theme.test.ts`:
round-trip of each axis through a fake storage object; unknown value falls
back to the default; empty/missing key falls back to the default; both
registries are non-empty and contain the documented ids.

Do not touch `styles.css`, `index.html`, `App.tsx`, or any component in
this task — nothing renders this module yet, and that is fine.

## Task 2 — the CSS token layer, the bootstrap, and the guard test

Three pieces, one commit.

**(a) `apps/knx-web/src/styles.css`** (D29). Move
`--knx-transition-duration: 250ms` out of the base `:root` block into
per-level blocks, keeping a base-block fallback for a document with no
`data-motion-level` attribute:

- `:root[data-motion-level="off"] { --knx-transition-duration: 0ms; }`
- `:root[data-motion-level="subtle"] { --knx-transition-duration: 120ms; }`
- `:root[data-motion-level="standard"] { --knx-transition-duration: 250ms; }`
- `:root[data-motion-style="apple"] { --knx-motion-easing: cubic-bezier(0.4, 0, 0.2, 1); }`
- `:root[data-motion-style="glitch"] { --knx-motion-easing: steps(4, end); }`
- base `:root`: both tokens keep their default values (`250ms`, the
  `apple` curve) as the no-attribute fallback.

Then replace the literal `ease` keyword in all three existing
`@media (prefers-reduced-motion: no-preference)` blocks (today at
`styles.css:42`, `:265`, `:774`) with `var(--knx-motion-easing)`. Change
nothing else in those blocks — same selectors, same properties.

Level `off` sets the duration to `0ms` rather than deleting the
declarations, deliberately: an element must not freeze mid-transition when
the level changes while a transition is running.

**(b) `apps/knx-web/index.html`** (D31). Extend the existing inline
bootstrap script — the one that already applies `data-theme` before React
mounts — to do the same for `data-motion-level` and `data-motion-style`:
read the key, validate against a hard-coded id list, set the attribute,
fall back to the default. Keep it in the same plain-ES5 style as the
existing script (it runs before any module loads, so no imports).

The hard-coded id lists duplicate `motion.ts`'s registries. That is
deliberate and unavoidable. Add a short comment in **both** files naming
the other, so a future id addition is not made in one place only.

**(c) `apps/knx-web/src/motionGuard.test.ts`** (D33) — the part that
outlives this slice. It reads `styles.css` from disk (`node:fs`,
resolving the path relative to the test file, not the cwd) and asserts,
for every `transition:` and `animation:` declaration in the file:

1. it sits inside a `@media (prefers-reduced-motion: no-preference)`
   block, and
2. its duration comes from `var(--knx-transition-duration)`, not a
   literal.

Parse by brace counting over the stylesheet text — no CSS parser
dependency. Report the offending line number and text in the failure
message; a future cycle's author should learn what is wrong from the
failure alone, without reading this plan.

Also assert the guard itself works: run the same checker function over a
small inline fixture string containing a non-compliant declaration and
assert it reports exactly that one. A guard that has never failed is a
guard nobody has tested.

## Task 3 — `SettingsPanel.tsx` and the `App.tsx` wiring

**Read `apps/knx-web/src/Search.tsx` and `CommandPalette.tsx` first.**
They are the two existing overlay implementations, and
[KNOWN_LIMITATIONS.md §20](../../KNOWN_LIMITATIONS.md) already tracks them
as near-duplicates with an accessibility gap. This task must not become a
third one: reuse their existing CSS classes and their existing
click-outside/`Escape` handling shape rather than inventing new ones, and
do not refactor them (that is §20's own future task, not this slice's).

Create `apps/knx-web/src/SettingsPanel.tsx` (D32): a small panel opened
from a new gear button beside the existing `ThemeSwitcher` in the toolbar,
closed by `Escape` and by clicking outside it. It renders three labelled
`<select>`s — Theme, Motion style, Motion level — each driven by its
registry, each applying immediately on change with no reload and no Save
button.

The gear button is an inline SVG, hand-written, matching
`ThemeToggle`'s old convention of not adding an icon-library dependency.
Give it an `aria-label`; the three `<select>`s each get a visible label
and an `aria-label`, following `ThemeSwitcher.tsx`'s existing pattern
(`.sr-only` span plus `aria-label`).

Wire it in `App.tsx`: `useMotion()` alongside the existing `useThemeId()`
(`App.tsx:75`), and the gear button rendered where `ThemeSwitcher` is
today (`App.tsx:345`). Whether the theme `<select>` stays in the toolbar
as well or moves into the panel is your call — make one, and say which in
your report with a one-line reason. Add whatever `styles.css` rules the
panel needs, subject to global constraint 3 (any transition you add is
gated and token-driven, or task 2's guard test will fail — which is the
point of it).

Tests in `apps/knx-web/src/SettingsPanel.test.tsx`, following the
existing component tests' style: the panel opens from the gear button;
`Escape` closes it; a click outside closes it; changing the motion level
`<select>` updates `document.documentElement`'s `data-motion-level`;
changing the motion style updates `data-motion-style`.

## Task 4 — the Group Monitor retrofit (D34)

`apps/knx-web/src/BusMonitorPanel.tsx` polls for telegrams
(`setInterval`, today around line 228) and appends rows to a table. It has
no motion at all — so the roadmap's "first item T27 has to retrofit" is
today satisfiable by doing nothing, which would discharge the sentence
without honouring it.

Give newly arrived rows a brief entry highlight, so a row that appears
while the user is reading is visibly new, then settles into the table's
normal appearance. Requirements:

- Driven entirely by CSS, gated by constraint 3, duration from
  `var(--knx-transition-duration)` and easing from
  `var(--knx-motion-easing)` — so `standard` and `subtle` differ in speed,
  `apple` and `glitch` differ in feel (smooth fade versus stepped), and
  `off` plus an OS `reduce` each flatten it to nothing.
- The component marks which rows are new; the stylesheet decides what
  "new" looks like. Do not read the motion setting in TypeScript — that
  would be a second source of truth and would defeat the OS-preference
  rule.
- Read the existing row-keying logic before adding a marker: rows already
  carry a monotonic sequence number from the server, which is the natural
  way to know what arrived since the last poll. Do not add a timer, a
  `setTimeout` chain, or per-row React state.

Tests: extend `apps/knx-web/src/BusMonitorPanel.test.tsx` — newly arrived
rows carry the marker class, rows present in the previous poll do not, and
the marker does not accumulate across polls. Assert on the class, not on
computed styles: jsdom does not run CSS transitions, and a test that
pretends otherwise is a test that lies.

## Task 5 — documentation reconciliation

Docs only; no code, no test change. Update, each in its own file's voice:

- **`docs/KNOWN_LIMITATIONS.md` §43** ("Animations have no in-app switch,
  only the OS reduced-motion preference"). It is now false as written.
  Rewrite the body to state what exists (two-axis control, the
  structural OS-wins rule, the guard test) and what remains limited (no
  per-category control; the guard covers `styles.css` only, so an inline
  style or a future CSS file it does not read would escape it; jsdom
  cannot verify that anything actually animates). **Leave the header text
  alone** — other files link its anchor slug. Count those links yourself
  with `grep -rn` and state the number you measured in your report; do not
  take any number on trust, including this sentence's absence of one.
- **`docs/GAP_ANALYSIS_ETS.md`** — row **D11** (closed), row **D8**
  (partially addressed: a settings panel now exists, but none of ETS's
  actual options live in it), and the **T27** backlog entry in Tier 7
  (mark done, record which of its open question was answered and how:
  D27 chose two axes over both a single multiplier and a per-category
  switch). Row **D9**/§20 must **not** be marked improved — the panel
  reuses the existing overlay CSS but the duplication is untouched.
- **`docs/ROADMAP.md`**, "Cross-cutting — Motion and animation": the
  control is restored, the standing constraint is now enforced by a test
  rather than by prose, and the 2026-09-10 **style memo is answered** —
  both directions shipped as selectable styles rather than one being
  chosen over the other. Keep the constraint paragraph itself; it still
  binds T17, T21 and the deferred telegram animation.
- **`docs/IMPLEMENTATION_STATUS.md`** — a new dated entry in the file's
  existing chronological position for 2026-09-12, listing what shipped and
  the measured test counts.
- **`.ai/logs/2026-09-12_claude_t27_motion_control.md`** — the
  architecture-change log CLAUDE.md's handover protocol requires.
- **`.ai/CURRENT_STATE.md`** — prepend one entry (append-at-the-top,
  additions only; `git diff --stat` must show `+N -0`).

Do not soften any other limitation, do not renumber anything, and do not
claim ETS parity or KNX certification anywhere — ETS has no comparable
motion control, so there is no parity claim to make in either direction.
