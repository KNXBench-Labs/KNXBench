# ADR 0157: Rail is a built-in light/dark pair with a controlled presentation

Date: 2026-11-02
Status: Accepted (owner request and choices of 2026-11-02)

## Context

The owner asked for a new theme: modern, sharp, fresh and precise, with fluid
animation, a modern typeface, square buttons and nothing generic. They chose a
light and a dark entry, Geist with Geist Mono, and a deepened KNX bus-cable green
accent with success set apart in turquoise. Square geometry and motion are not
expressible by a v1 theme pack (ADR-0060), whose tokens are colour, the font
allow-list and radii only, without CSS or animation. ADR-0092 already admits one
trusted built-in presentation (LCARS) behind a separate document marker. The
standing UI rule is that every newly requested animation is switchable through
its own motion category (ADR-0122).

## Decision

- Ship `rail` (**Rail**, light) and `rail-dark` (**Rail Dark**) as complete
  ADR-0022 palette blocks with fixed accents, Geist/Geist Mono and zero button,
  input and card radii. Selection uses the existing single Theme dropdown
  (ADR-0079); no new setting, schema migration or server write path.
- Both resolve to one presentation marker, `data-presentation="rail"`, set by
  `useThemeId` and mirrored by the pre-paint bootstrap only for these built-in
  ids, never for a pack. A small id-to-presentation map in `theme.ts` replaces
  the LCARS-only condition; LCARS behaviour is unchanged.
- The presentation squares shared controls and the common dialog/menu surfaces,
  sharpens hover edges to ink, marks selection as an edge, ends table headers in
  an ink rule and draws an inert measuring rail (18px pitch, major tick every
  90px) under the toolbar. It adds no panels, hit targets or status meaning.
  Its primary hover/press veil uses theme ink: darker in Rail light, lighter in
  Rail Dark, preserving text contrast while remaining presentation-scoped.
  Other themes keep their existing veil.
- Motion lives in a new reduction-only category `theme` (`motionTheme`,
  "Theme effects (Rail)"): a one-time rail draw at start-up, a 2px line that runs
  in under a hovered control (the primary veil sweeps instead), dialogs settling
  in, the File menu unrolling while open and the current-workspace bar handing
  over. Subtle keeps short fades only. All of it sits behind OS
  `no-preference` and the effective category level, uses multiples of
  `--knx-transition-duration` and the user's easing, is finite, never fills
  forwards and starts from a motion-free baseline. No ambient loops.
- Geist and Geist Mono come from `@fontsource/geist` and
  `@fontsource/geist-mono` (SIL OFL 1.1), bundled and served locally like the
  existing fonts; no external font request. They are not added to the pack font
  allow-list.

## Alternatives considered

- A v1 pack: cannot express square geometry, motion or a new font.
- Restyling Porcelain/Graphite: changes users' existing default look unasked.
- Riding the global motion level only: violates the per-animation category rule.
- A second presentation per palette: identical geometry; one marker is simpler.

## Consequences

Rail is not exportable as a lossless v1 pack and not removable through Theme
files. Components that paint their own corners or backgrounds outside the listed
surfaces keep them. The Rust diagnostic bundle lists both ids as shipped themes.
`railPresentation.test.ts` pins the bootstrap/runtime mapping, square tokens,
inert decoration, motion gating and keyframe properties; the Chromium theme
matrices (roles, focus, separators, theme state, dialogs) include both palettes.
Verification is Chromium with intercepted synthetic API replies; native
WebKitGTK/Orca, Firefox and full WCAG acceptance are not established. The
`rail-theme.e2e.ts` browser regression composites the actual constant-colour
primary layers at rest, hover and press, and checks live motion cancellation.
