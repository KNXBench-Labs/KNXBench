# Motion controls and residual surfaces

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-043)

## Four motion categories (20 October 2026 reconciliation)

The three-category account in the retained record and 14 October update is
superseded: **Toasts**, **Achievements**, **Flow** and **Structure lines**
now have independent reduction-only preferences. The fourth is `lines`, stored
as `motionLines`; it applies to structure-connection feedback. Global Off and
the operating-system reduced-motion preference still win. Other effects retain
their documented global controls, and this does not widen the stylesheet
guard or establish native/accessibility conformance.

Evidence: [category list and reduction rule](../../apps/knx-web/src/motion.ts),
[settings consumer](../../apps/knx-web/src/MotionCategorySettings.tsx),
[structure-lines regression](../../apps/knx-web/src/motionCategories.test.ts).

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="43-animations-have-no-in-app-switch-only-the-os-reduced-motion-preference"></a>

## 43. Animation controls exist; some motion surfaces remain outside their guard

**Limitation.** Resolved for the two axes T27 (2026-09-12) shipped,
enforced structurally rather than by convention, and still limited in
five specific, deliberate ways below.

`apps/knx-web/src/motion.ts` exposes two independent, persisted settings —
a **level** (`off`/`subtle`/`standard`, default `standard`, driving
`--knx-transition-duration`: `0ms`/`120ms`/`250ms`) and a **style**
(`apple`/`glitch`, displayed as "Smooth"/"Glitch", default `apple`,
driving `--knx-motion-easing`: a cubic-bezier ease or `steps(4, end)`).
Both live as two `<select>`s in the gear-button `SettingsPanel.tsx`
alongside the theme picker, persist to `localStorage`
(`knx-desktop:motion-level`, `knx-desktop:motion-style`), and are applied
as `data-motion-level`/`data-motion-style` on `<html>` both by
`useMotion()` after React mounts and by a pre-mount bootstrap script in
`index.html` (so there is no flash of default motion before the first
render — the script hard-codes the same id lists as `motion.ts`,
cross-commented in both files as a duplication to keep in sync by hand).

The OS-wins rule is structural, not conventional: every
`transition:`/`animation:` declaration in `styles.css` sits inside a
`@media (prefers-reduced-motion: no-preference)` block, none uses
`!important`, and no `.ts`/`.tsx` file calls `window.matchMedia` at all —
so `prefers-reduced-motion: reduce` cannot be overridden from inside the
application, even by mistake. `motionGuard.test.ts` turns that rule into a
test: a brace-counting checker over `styles.css`'s text fails the suite if
any `transition:`/`animation:` declaration sits outside a
`no-preference` block, or uses a literal duration instead of
`var(--knx-transition-duration)`.

What remains limited, on purpose:

- **Per-category control covers three surfaces only (since 2026-10-13).**
  Toasts, Achievements and Flow have their own reduction-only setting
  ([ADR-0122](../adr/0122-category-motion-preferences.md)); everything else
  (the Group Monitor's new-row highlight, hover transitions, dialogs) still
  follows the one global level and easing. A category can never raise the
  global level or override the OS preference.
- **The guard reads `styles.css` only.** An inline `style={{ transition:
  ... }}` in a component, a second CSS file, or a stylesheet inside a
  future dependency would all escape it entirely.
- **The guard matches the `transition:`/`animation:` shorthands only.** A
  longhand — `animation-duration: 300ms`, `transition-delay: 400ms` — is
  not inspected and would pass. Widening the pattern to longhands would
  false-positive on `transition-property`, which carries no duration at
  all; closing the hole honestly needs a CSS value parser, and the slice
  forbids the dependency. Recorded rather than fixed, on purpose.
- **The test environment does not run CSS animations.** No test anywhere
  asserts that anything actually moves; the tests assert that the right
  class and the right attribute are applied (including
  `BusMonitorPanel.test.tsx`'s new-row highlight tests), and the
  stylesheet is trusted to do the rest. Nobody has verified the visual
  result in a browser.
- **The guard has to read the stylesheet from disk with `node:fs`, and the
  tidier-looking alternative silently disarms it.** Replacing the read
  with Vite's `import css from "./styles.css?raw"` type-checks, runs, and
  passes — against the **empty string**, because Vitest does not process
  CSS. This was tried on this branch and caught by injecting a literal
  `200ms` into `styles.css` and watching the suite stay green regardless.
  A future author tidying that import away would remove the guard without
  removing the test. The `node:fs` import in turn needed
  `apps/knx-web/src/node-builtins.d.ts` (added mid-slice, commit
  `056b4a0`), because `npm run build` is `tsc && vite build` over
  `include: ["src"]` and `@types/node` is deliberately not a dependency —
  without it, `npm run test` (Vitest) was passing on a `node:fs` import
  that broke the production build outright, caught once on this branch
  before it shipped anywhere.

**Cause.** The regression this section used to describe — cycle 11's
`off`/`subtle`/`standard` setting in `ThemePanel.tsx`, deleted without
replacement by cycle 13's theme rewrite — is fixed. What remains above is
scope, decided rather than missed: T27's design chose two orthogonal axes
(level, style) over a per-category switch because the 2026-09-10 style
memo asked for two independent visual directions, not finer-grained
animation targeting (see `ROADMAP.md`'s "Cross-cutting — Motion and
animation"); and the guard was built as a text checker over one known
file rather than a real CSS parser, because the slice's no-new-dependency
rule rules out pulling one in just for this.

**Impact.** Materially smaller than before T27. A user bothered by motion
can turn the level to `off`; a user who finds the default merely too much
can use `subtle`; a user with an opinion about *how* things move, not just
how fast, can pick between the two shipped styles independently of
intensity. What a user still cannot do is quiet one feature while keeping
another animated, and what nobody can do is rely on the test suite alone
to prove that nothing outside a `no-preference` block moves — the guard's
blind spots above are real, just narrow today because `styles.css` is
still the only stylesheet and every current declaration is a compliant
shorthand.

**Lifted when.** Partially lifted, 2026-09-12 (T27, closing
`GAP_ANALYSIS_ETS.md` gap D11): the in-app switch exists, is two axes
wide, persists across sessions, and is structurally bound to
`prefers-reduced-motion: reduce` always winning. The rule this section
used to ask for — every future animation switchable through it — is now
enforced by `motionGuard.test.ts` rather than by prose, and T15's Group
Monitor table (the first animated feature that shipped without the
switch) has been retrofitted with a guarded new-row highlight
(`BusMonitorPanel.tsx`). What is left open, and would need its own
design work rather than a bugfix: (1) a per-category control, if a future
feature ever needs quieter motion in one area while another stays
animated — not requested yet; and (2) a guard with real CSS-parser-backed
coverage (longhands, non-`styles.css` sources, inline `style=` motion) —
deliberately deferred, since it needs a dependency this slice was not
scoped to add.
<!-- END retained catalogue record -->

## Update 2026-10-14: per-category motion exists (Goal B8)

The record above still says a user "cannot quiet one feature while keeping
another animated" and lists a per-category control as open item (1). That was
delivered on 2026-10-13 ([ADR-0122](../adr/0122-category-motion-preferences.md),
ledger `KL-43` `DONE`): **Toasts**, **Achievements** and **Flow** each inherit,
stay at most subtle, or are off (`motionToasts`, `motionAchievements`,
`motionFlow`); a category only reduces, and global Off and the OS preference
always win (`apps/knx-web/src/motion.ts`). Open item (2) is unchanged: the guard
in `motionGuard.test.ts` is a text check over `styles.css`, not a CSS parser, and
no test proves that nothing moves outside its guard. Other animated surfaces have
no category of their own and follow the global level.
