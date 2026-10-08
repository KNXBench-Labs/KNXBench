# Modern LCARS presentation

Implemented locally on 2026-10-08 after visual approval of the
[interactive offline study](design-studies/lcars/README.md).
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
  transitions, driven by the existing duration/easing preference.
- **Subtle:** colour/border transitions without the Standard reveal.
- **Off / OS reduced motion:** no LCARS reveal or transitions; existing running
  effects are cancelled rather than left paused.
- Existing Smooth/Glitch/CRT settings remain independent; LCARS does not force
  another motion style. Existing application effects still follow their owner.

A reveal marks navigation, never saved data or connected hardware. Save and
bus state continue to follow actual application/API results. A synthetic Save As
refusal in the browser proof issues one request, shows its real error and leaves
"Not saved yet" unchanged. All fixture requests, including automatic discovery,
are intercepted; no real server, filesystem or bus is contacted by that proof.

## Verification

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
the repository cwd under `output/playwright/`. Exact-source evidence is in
`design-studies/lcars/application-verification.json`.

The offline study remains a separate memory-only demonstrator, not a production
save implementation. Native WebKitGTK/Orca, Firefox, browser-chrome zoom and a
complete WCAG audit are unverified. This is not an official Star Trek product.
No Docker rebuild, application release or live hardware operation is included.
