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
  nested inside one, is a violation. (Nesting was the only direction this
  bullet closed; the sibling direction is the selector-list gap below.)
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

Two more came out of the whole-branch review of 2026-09-20, both in what
those new guards could not see rather than in what they claimed:

- **The optional semicolon.** CSS lets the last declaration in a block go
  without one, and `parseRules` flushed only on `;`, so `.evil { color:
  #ff00aa }` — valid, browser-honoured CSS — was invisible to all three
  guards at once, this ADR's opening failure included. `parseRules` now
  flushes at `}` as well.
- **The selector list, and the spelling of an attribute name.** The
  violation check ran per comma-part while the three classifiers matched
  the whole collapsed selector, so `:root[data-theme="a"],
  :root[data-theme="b"] { … }` was a third shape belonging to neither
  set — two themes sharing one block, which is the sharing Consequences
  above gives up on purpose ("themes no longer share values …
  duplication is the price of the guarantee"), reintroduced through a
  comma. And because attribute *names* in a selector are ASCII
  case-insensitive while `data-*` *values* are not,
  `:root[DATA-THEME="a"]` selected a theme that no check recognised. All
  four functions now ask one question, over the whole selector, with the
  attribute name folded to lowercase and the theme id left alone.

## Contrast enforcement (T13)

The build-time gate now enforces the three role pairs already implied by the
component layer:

- `--knx-foreground` on `--knx-bg`;
- `--knx-foreground` on `--knx-surface`;
- `--knx-on-accent` on `--knx-accent`.

Every registered palette and every registered accent variation is evaluated.
Variations overlay only their accent pair on the base theme before evaluation.
The evaluator supports the concrete opaque hex and `rgb()`/`rgba(..., 1)` forms
used by these roles, plus recursive `var(--knx-...)` references. Unsupported
notation, unresolved references, cycles, and non-opaque alpha are named
violations containing the theme, pair, and offending value; they are never
skipped. The exact WCAG AA threshold is 4.5:1, with no rounded acceptance.

Duplicate `(theme, accent)` blocks fail rather than hiding a later CSS override
behind a first-match lookup; each variation test receives its actual block.
RGB channels must be finite and inside `[0,255]`, including numerical helper
inputs (valid fractional values remain supported). Relative luminance uses the
current WCAG sRGB breakpoint `0.04045`. An alias error retains the terminal
offending value, not only the name of the referring token.

This is a role-pair invariant, not an audit of every possible component
composition, browser rendering difference, or assistive technology.

## CRT interaction follow-up (2026-10-03)

The CRT motion style extends the application-owned style registry with `crt`.
It is independently selected in Settings, not encoded in a palette and not
implicitly enabled when a theme is imported. Existing defaults remain unchanged.
The pre-mount bootstrap and runtime registry admit the same third identifier;
an execution test covers the actual bootstrap, not only its literal array.

Standard CRT uses a 250ms ease-out fill. Subtle keeps the existing 120ms duration
and omits transient light/flash; Off and OS reduced motion use an explicitly
motion-free baseline. All animated rules retain the no-preference media guard.
Palette roles remain the existing 27-token v1 boundary. No arbitrary pack CSS,
new palette role, settings migration or project-model change is introduced.

Native table rows and tree labels paint their own fill with background-size.
One disposable UI controller per workspace owns the bounded activation flags,
timers, two observers and an inert, aria-hidden decorative span outside the
table. Its bounds are intersected with scroll clipping and the viewport, then
converted for the application's existing root zoom. It cancels on preference
changes, anchor retirement, scroll/resize, drag start, window blur and disposal.
It never prevents an action, synthesizes clicks, changes selection or calls an
application/backend service. The existing delegated tree/workspace keyboard
handlers are reused; no second keyboard controller is installed.

Manual Save/Save As emits only a presentation signal immediately before its
existing API request, after Save-As cancellation/stale-snapshot checks. The glow
is an activation cue, not a persistence acknowledgment; error handling and dirty
state are unchanged. Autosave does not emit decorative feedback. Each transient
effect stream has a 600ms admission gap; the native actions are not throttled.

Evidence and narrower compatibility claims are in
[the CRT guide](../DESIGN_RETRO_GREEN_CRT.md) and the production browser receipt.
The screenshot samples a paused native animation; normal expiry/cancellation is
verified separately with real timers. This is not WebKitGTK/Orca or broad WCAG
acceptance, and it does not supply Save-only purple or an exact selection color.
