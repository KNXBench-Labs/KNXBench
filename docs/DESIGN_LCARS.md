# Modern LCARS presentation

Implemented locally on 2026-10-08 after visual approval of the
[interactive offline study](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/design-studies/lcars/README.md).
Decision: [ADR-0092](adr/0092-lcars-built-in-presentation.md).

## Use

Settings → Appearance → Theme → **LCARS**. The choice is acknowledged through
the existing settings write and persists as `theme: "lcars"`. Switch to another
entry to restore its original presentation. LCARS ships with the application;
it is not an imported pack, cannot be removed/exported as one, and changes no
project data.

The dark palette uses warm orange/apricot framing and lavender navigation and
addresses. Space Grotesk headings, Inter content and JetBrains Mono technical
values use the existing fonts. Characteristic elbows and segmented bands frame
the existing toolbar/sidebar; the central engineering tables stay quiet.
Explorer, workspace, inspector, diagnostics, resizing, keyboard shortcuts,
selection and validation keep their existing workflows. No extra clicks,
decorative panels, sound or fake telemetry.

Accent is fixed; its disabled control preserves the user's accent preference
for other themes. Density remains independent: Compact and Comfortable still
change actual control/row spacing. At narrow sizes the decorative rail becomes
smaller or disappears; the existing shell stacks and scrolls normally.

## Movement and truthfulness

- **Standard:** finite selection-underline reveal and brief colour/border
  transitions, plus a gentle 10-second header-band pulse and 16-second
  apricot/lavender brand-emblem colour cycle. The band never disappears.
- **Subtle:** colour/border transitions without the Standard reveal; ambient
  cycles slow to 18/24 seconds, with less dimming and only a 20% colour mix.
- **Off / OS reduced motion:** static header/emblem and no LCARS reveal or
  transitions. Running effects are cancelled rather than left paused, also
  after a cold reload. Leaving LCARS cancels the ambient effects too.
- Existing Smooth/Glitch/CRT action styles remain independent. Ambient chrome
  deliberately uses smooth easing so it does not become stepped or hectic.

The two decorative loops were explicitly requested on 2026-10-08 after the
original finite-motion delivery. Only the header pseudo-element opacity and
small emblem background colour animate: no moving geometry, blinking content,
fake activity indicators, JavaScript timers or animation-triggered API writes.
The existing motion control is the opt-out; no new persisted setting is added.

A reveal marks navigation, never saved data or connected hardware. Save and
bus state continue to follow actual application/API results. A synthetic Save As
refusal in the browser proof issues one request, shows its real error and leaves
"Not saved yet" unchanged. All fixture requests, including automatic discovery,
are intercepted; no real server, filesystem or bus is contacted by that proof.

## Ambient follow-up verification (2026-10-08)

The requested decorative follow-up passed **2,332 frontend tests / 138 files**,
**158 Chromium tests**, build/type/flow checks and five repository gates.
The actual built-workbench CLI passed **54 named assertions**: genuine clock
progression and colour/opacity changes, stable rows/geometry, zero idle backend
mutations, live Off/OS/theme cancellation, cold startup, lower Subtle amplitude,
independent preferences, truthful refused save and small-window usability.
Warm/lavender header captures were inspected. This is local, offline synthetic
API/self-review evidence, not a new live deployment or native certification.
Exact final source, build and checks: [ambient receipt](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/design-studies/lcars/ambient-verification.json).

## Original integration verification

- Frontend: **2,286 tests / 136 files** passed; production TypeScript/Vite build
  and theme-fixture type check passed.
- Existing complete intercepted browser suite: **158 tests** passed, including
  new LCARS actual editor/table/inspector/validation/dialog/accent coverage and
  existing theme-manager/pane-splitter regressions.
- Production-workbench CLI proof: **33 named assertions** passed; no unexpected
  API/external requests or browser errors. It deliberately returns HTTP 500 for
  Save As; that known console resource error is not hidden as a successful save.
  Real OS-motion cancellation, Off/Subtle/Standard, independent density, theme
  switching/reload, actual application CSS zoom 1.5, and 720×620/480×900 layouts
  were exercised. Representative screenshots were visually inspected.
- Unit RED→GREEN: missing registry/dropdown/restoration and missing framing/
  density/motion-boundary assertions failed before their implementation.
- Review is **in-session source self-review**, not independent approval.

Reproduce the production proof by building `apps/knx-web`, serving its `dist/`
on isolated loopback port 4189 (no API proxy), opening an isolated Playwright
CLI Chromium session, then running
`docs/design-studies/lcars/verify-application.js` with CLI `run-code --filename`.
Retrieve `window.__lcarsApplicationVerification` with CLI `eval`; do not infer
assertion success solely from a wrapper exit. Screenshot paths are relative to
the repository cwd under `output/playwright/`. `application-verification.json`
binds the original finite-motion delivery; `ambient-verification.json` binds the
later ambient follow-up. Prior deployment evidence does not certify this new
local-only source.

The offline study remains a separate memory-only demonstrator, not a production
save implementation. Native WebKitGTK/Orca, Firefox, browser-chrome zoom and a
complete WCAG audit are unverified. This is not an official Star Trek product.
The original local integration did not include deployment. On 2026-10-08 the
user separately authorized publication and a Docker rollout, now verified on
HTTPS port 8484; see [deployment receipt](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/design-studies/lcars/deployment-verification.json).
No release tag or live hardware operation was performed.

## LCARS ambient timing reference and live rollout

The header/emblem cycles use the existing transition-duration token, not a
separate clock setting. Standard 10s/16s and Subtle 18s/24s are reference
periods at 200ms and 120ms respectively. Existing timing overrides scale the
periods: the verified live Standard token `.25s` gives 12.5s/20s. No saved
timing preference is overwritten. Off and OS reduced motion cancel the
effects immediately. Source/image/live proof is recorded in
`docs/design-studies/lcars/ambient-deployment-verification.json` relative to
the repository root.
