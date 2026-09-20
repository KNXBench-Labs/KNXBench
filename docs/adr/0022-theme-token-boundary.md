# ADR 0022: A theme owns the palette tokens; the settings own the rest

Date: 2026-09-19

Status: Accepted

## Context

`apps/knx-web/src/styles.css` carried three themes and 31 `--knx-*` custom
properties declared in total (26 once the five that turned out to belong to
the component layer are set aside), with no rule saying which of them a
theme is responsible for. The result was visible in the stylesheet before
this ADR was written:

- The light palette was defined **by negation** —
  `:root:not([data-theme="bitcoin-defi"])` — and so were all four accent
  overrides. Every theme added after Bitcoin DeFi would have been swept into
  the light palette silently, inheriting a stranger's background colour
  without anyone declaring it.
- `--knx-on-accent` existed in two themes out of three. The third compensated
  with a hard-coded `color: #ffffff` on every button: white on `#f7931a`, a
  contrast ratio of about 2.3:1.
- Graphite, the dark theme, inherited the light theme's shadows
  (`0 1px 2px #1721390a` over a `#171b22` canvas), which is a shadow nobody
  can see.
- Bitcoin DeFi declared `--knx-transition-duration` and `--knx-motion-easing`
  itself, duplicating the motion control that `motion.ts` already owns, so a
  theme could have silently overridden a user's accessibility setting.
- `--knx-accent-secondary` and `--knx-glow-gold` were declared by two themes
  each and read by nothing.
- The decorative backdrop was a rule only one theme could ever match:
  `:root[data-theme="bitcoin-defi"] body::before`.

Each of these is the same mistake: no one had said what a theme is, so a
theme was whatever the cascade left over.

## Decision

The `--knx-*` custom properties split into two layers, and a token belongs to
exactly one of them.

**The theme layer.** One block per theme, selected as
`:root[data-theme="<id>"]`, containing nothing but custom properties and
`color-scheme`. A theme block must define **every** token in the boundary.
Themes never style an element and never define themselves by negating another
theme — the two legal selector shapes are the base block above and the
accent variation `:root[data-theme="<id>"][data-accent="<name>"]`.

**The component layer.** `:root` plus the `[data-density]` and
`[data-motion-*]` setting blocks. These hold the tokens a *user setting*
owns: `--knx-control-height` and `--knx-cell-padding` (density),
`--knx-transition-duration`, `--knx-motion-easing` and
`--knx-feedback-duration` (motion, ADR-scope of T27). A theme that set one of
these would be rebuilding a settings mechanism inside a palette.

**Membership is derived, not curated.** A token is a theme's to set exactly
when the stylesheet reads it through `var()` and no user setting owns it.
`requiredThemeTokens()` in `apps/knx-web/src/themeTokens.ts` computes that set
from the stylesheet itself, and `themeTokens.test.ts` holds every theme to it.
Adding `var(--knx-whatever)` to a component rule therefore obliges every theme
to answer for it on the next test run — which no hand-maintained list would
have caught. The boundary is 27 tokens today: surfaces and ink, the accent
pair, status colours, overlay and elevation, the three type families, the
three radii, and the three backdrop tokens.

**Motion is not a theme's business.** The two registers the roadmap memo asked
for — Apple-subtle and cyberpunk-glitch — are split along this same line. The
*visual* half is a theme (`cupertino`, `neon-grid`). The *motion* half is the
existing motion setting's style axis (`apple` → a cubic-bézier ease,
`glitch` → `steps(4, end)`), reaching the stylesheet only through
`var(--knx-transition-duration)` and `var(--knx-motion-easing)` inside
`@media (prefers-reduced-motion: no-preference)`, exactly as
`motionGuard.test.ts` requires. The two axes compose: any theme can be run
with either motion style.

**Accent variations are opt-in.** A theme may re-point `--knx-accent` and
`--knx-on-accent` per accent preference; a theme whose accent is part of its
identity declares no variations and keeps its own accent whatever the accent
setting says. Porcelain and Graphite vary; Cupertino, Neon Grid and Bitcoin
DeFi do not.

**The document always carries a resolved `data-theme`.** `index.html`'s
pre-mount bootstrap sets it before the first paint and `useThemeId` maintains
it, so no theme has to double as the no-attribute default. The theme selection
persists in `localStorage["knx-desktop:theme"]` and nowhere else.

## Alternatives considered

**A base theme that others extend.** Porcelain defines everything, other
themes override what differs. Rejected: it is precisely the arrangement that
produced Graphite's invisible shadows. An incomplete palette has to *fail*,
not fall back, or it ships.

**A hand-written list of theme tokens.** Simpler to read, but it goes stale
the first time someone adds a `var()` to a component rule and forgets the
list. Deriving the list from the stylesheet removes the chance to forget.

**Themes carrying their own motion.** A cyberpunk theme that shipped its own
durations would be a second settings mechanism, unreachable from the Settings
panel and invisible to the reduced-motion guard. Rejected outright.

**Renaming every token to a fully semantic scheme.** Tempting —
`--knx-accent-tertiary` still says nothing about its role. Only the three
tokens whose names hard-coded a colour were renamed (`--knx-glow-orange` →
`--knx-shadow-raised`, `--knx-glow-orange-strong` → `--knx-shadow-hover`,
`--knx-gradient-gold` → `--knx-gradient-display`), because a theme layer whose
token names presume a palette cannot hold more than one palette. The rest is
left for a task that is about naming.

## Consequences

Easier: adding a theme is filling in one block and one registry entry, with a
test that says exactly which tokens are still missing. The accent-by-negation
trap cannot come back — a selector outside the two legal shapes fails the
suite.

Harder: themes no longer share values, so a change to the shared geometry
(the radius scale, the type stack) has to be made in every block. That
duplication is the price of the guarantee, and it is a price paid in a data
section, not in logic.

Enforced by `apps/knx-web/src/themeTokens.test.ts`: one block per registered
theme; every block complete; no component-layer token inside a theme; no
plain property but `color-scheme`; no negation selector and no theme-scoped
element rule; no token declared and never read; accent variations limited to
the accent pair of a registered theme and a registered accent; and
`index.html`'s duplicated bootstrap id list identical to `THEMES`.

Four gaps in that enforcement were found by the pre-merge review of
2026-09-19 and closed afterwards, each with a test that fails against the
unfixed behaviour:

- **Nesting.** The checks above collected depth-0 blocks only, so a theme
  block written inside an `@media` query was neither held to the boundary
  nor reported — including one redefining `--knx-transition-duration`, the
  exact override this ADR exists to prevent. `parseRules` now carries each
  rule's ancestor chain, and a theme selector that is nested, or any rule
  nested inside one, is a violation.
- **The component layer's literal colours.** The boundary proves a theme
  block is complete; nothing proved a component rule goes through the
  tokens at all. A `color: #ff00aa` in a component rule passed the whole
  suite. `componentColourLiterals()` now rejects hex literals, colour
  functions and named colours outside the theme layer; `transparent`,
  `currentColor` and `url()` contents pass. The stylesheet needed no
  allow-list — it had no such literal already.
- **One stylesheet.** The test read a single hard-coded path, so a second
  `.css` file would have sat outside this ADR entirely. It now walks
  `apps/knx-web` for every `.css` file and fails if it finds none.
- **Token names.** `var(--knx-fooBar)` was truncated to `--knx-foo` and
  handed on as a real token, failing every theme for a token nobody wrote.
  Names are read whole and illegal ones rejected by name.

Not covered: nothing checks that a palette is *legible*. Contrast ratios were
chosen by hand and no automated check enforces them. A contrast test over the
theme layer is the obvious next tightening, and `themeTokens.ts` already
parses everything such a test would need. Recorded as
[KNOWN_LIMITATIONS.md §119](../KNOWN_LIMITATIONS.md).
