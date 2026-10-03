# Modern Retro Green CRT

## Production animation integration (2026-10-03; integrated follow-up)

The previously proposed interaction effects now run in the real React `App`,
`ProjectExplorer` and `GroupAddressTable`, on the isolated branch
`feat/crt-interactions-20261003`. The authorized follow-up committed the feature
as `16c9d774` and integrated it onto current upstream main as `3d03aea5` in a
separate clean checkout. Publication is verified through the current Git refs
and the closure handover; the original local-only boundary below is historical.
The existing tree/workspace keyboard controllers are reused unchanged.

The integrated merge passed Web1739/98, build,12 production and16 reference
Chromium groups, workspace Clippy, Rust3009 passed/166 ignored (153 result blocks),
fmt and4 repository gates. The ignored tests/private corpus/live hardware were
not executed by this ordinary workspace gate; native/Orca acceptance is unclaimed.
See `.ai/logs/2026-10-03_codex_crt-merge-publication.md` and its retained receipt.

Select **Settings → Appearance → Motion style → CRT** separately from the theme:

- **Standard:** 250ms ease-out left-to-right hover/focus/selection fill, a bounded
  leading light and an activation glow capped at 120ms. Repeated bright feedback
  is bounded by a 600ms per-stream gap without throttling the actual actions.
- **Subtle:** existing 120ms fill only, no transient light or activation flash.
- **Off / OS reduced motion:** static feedback only; running CRT effects cancel.
- Manual **Save/Save As** glows when its existing request begins, not when it is
  assumed to succeed. Cancellation/stale-snapshot guards and errors remain intact;
  autosave does not glow. Save still uses the shared palette accent.

The palette remains **v1 / 1.1.0** and carries no motion or executable CSS.
Fill colors use the current shared accent/surface mixtures, preserving the
multi-selection rail. Save-only purple, exact `#003300`, additional input-focus
glow and broader component styling remain separate semantic-role proposals.
Importing the palette does not silently enable animations or override Motion Off.

Production evidence:

- `design/verify-crt-interactions.mjs` drives the actual application entry point,
  real browser file picker, mocked native-open projection and actual manual Save
  path. All API requests are intercepted or blocked; startup discovery is mocked.
  No project file, real backend/settings or KNX hardware is read/written.
- [Production browser receipt](../design/retro-green-crt.production.receipt.json)
  records native expiry, selection/checkbox independence, existing keyboard
  navigation, mid-effect Off/reduced-motion cancellation, zoom clipping, Save
  refusal/Enter behavior and separation from Smooth.
- [Production screenshot](../design/retro-green-crt.production.png) samples a real
  native 125ms frame at a fixed viewport with Playwright Clock paused. Informational
  toasts are dismissed through their real buttons for the capture. Expiry tests
  use real timers separately; the still image is not a timing test.
- `crtInteractions.test.ts`, `crtStyles.test.ts` and the strengthened motion tests
  cover admission, bounded effects, cancellation, disposal, clipping and bootstrap.
  Existing motion/token/selection/application regressions remain in the full suite.

Reproduce after `npm ci --ignore-scripts` in `apps/knx-web`, then start
`npx vite --config vite.fixtures.config.ts` there. From the worktree root run
`node --experimental-strip-types design/verify-crt-interactions.mjs`.
Use only that no-proxy local fixture server and `/usr/bin/chromium`. The Node flag
admits the repository's synthetic TypeScript fixture import. The script overwrites
only its own production screenshot/receipt. Native WebKitGTK/Orca and full
tree/grid accessibility remain unclaimed; see the ADR-0022 follow-up.

The following design-study/publication entries are historical. In particular,
their original “not integrated” statements describe that earlier delivery, not
the production animation follow-up above.

## Reference-driven revision — 1.1.0 (2026-10-03)

This revision develops the existing CRT design from the user's monitor photograph:
compact chrome, quiet green-black surfaces, softer phosphor text, luminous mint
action buttons, fine table rules, purple Save and a short white-green leading
edge during row interaction. These are deliberate design values, not claims to
recover calibrated colors from a photographed display. The private reference
photograph is not copied, embedded or re-published in these artifacts.

The palette keeps the same ID with version **1.1.0**; the schema and token version
remain v1. The application's replacement confirmation protects an installed 1.0
pack. No arbitrary styles or executable effects were added to the format.

## Delivered design artifacts

- Importable, complete v1 palette:
  [`modern-retro-green-crt.knx-theme.json`](../apps/knx-web/themes/modern-retro-green-crt.knx-theme.json).
- Self-contained interactive **target-design study**, not application integration:
  [`modern-retro-green-crt.preview.html`](../design/modern-retro-green-crt.preview.html).
  Open it in a browser. It uses synthetic data, does not fetch, does not write
  files/settings and cannot contact KNX hardware. It now uses a native table with
  twelve synthetic records, separate checkboxes and address activation buttons.
  This is not proof of production table/tree accessibility or native WebKitGTK.
- Admission, exact palette, contrast, export/reimport and reversible DOM tests:
  `apps/knx-web/src/retroGreenTheme.test.ts`.
- Reproducible browser verification:
  [`verify-crt-reference.mjs`](../design/verify-crt-reference.mjs) and
  [`browser receipt`](../design/retro-green-crt-reference.receipt.json).
  The screenshot samples an actual browser-rendered animation paused at 125ms;
  lifetime and cancellation checks run separately with real timers.

The study demonstrates the complete requested appearance, including purple Save,
left-to-right fill with a bounded leading light, mouse/keyboard activation glow,
violet Save flare, input focus glow, native-row navigation, independent checkbox
selection, filtering and an animation switch. It is **plain HTML/CSS/JavaScript**,
not a second React application or a Tailwind/Tauri implementation. The importable
palette is consumed by the existing React/Vite frontend and its existing Tauri
shell. No framework or dependency was added. The preview embeds the unmodified
installed JetBrains Mono Latin regular WOFF2, with its full SIL OFL notice/license
in the HTML; it requires no external font request.

## Design direction

Engineering workbench, not a pretend terminal: left Project Explorer, central
address workspace and right Properties Inspector. Left-aligned content, compact
monospaced typography, hairline green boundaries and small 2px token radii.
No decorative boot sequence, permanent flicker, fake connectivity or scanline
veil over engineering data. Use glow for interaction, not every text glyph.

| Role | Requested target | v1 palette |
| --- | --- | --- |
| Canvas / panels | Black with subtle green-black panels | `#050505` / `#050b06` |
| Phosphor text | Softer green, not neon on every glyph | `#bedbbb` |
| Quiet rules / interactive accent | Fine green borders; neon for interaction | `#31573b` / `#39ff14`; components decide where borders occur |
| Typography | JetBrains Mono | All three font tokens use the installed font |
| Muted ink | Readable, softer green | `#91b48b` |
| Luminous action fill | Mint phosphor | Primary gradient `#89ff9b` → `#b2ffc0`, existing gradient consumers only |
| Selected/hover fill | `#003300` | Not independently expressible; current components mix the shared accent |
| Save fill | `#6b21a8` | Not independently expressible; Save uses the shared accent, so remains green |
| Elevation/hover glow | Restrained green | Existing shadow tokens use green alpha |
| Error/warning | Distinct, readable status | Coral `#ff7373` / amber `#f4cc70`; not disguised as success |

A purple `--knx-accent` would turn selected tree text, focus, navigation and other
primary controls purple too. The palette deliberately keeps this shared role
neon green rather than shipping low-contrast selected text on black. No requested
unsupported value is hidden in an ignored JSON field or a token with another role.

## Use in the application

### Publication checkpoint

After explicit user authorization, source commit
`879c69b2f11c825c1d4f10b5409e5a5148f06e67` was pushed to
`design-retro-green-crt-20261003`; local, fetched and live remote source refs were
equal on2026-10-03 at20:19 CEST. Fresh Web1712/96 files and TypeScript/Vite build
passed. Review was in-session, not independent-agent approval. The branch is
published, **not merged into main**; no dirty-root product synchronization.

### Import

Requires the U14–U18 theme integration (verified baseline `e7f9db8e`, fetched
`origin/main`), not the older dirty local root checkout inspected at task start.
Open **Settings → Appearance**, import the `.knx-theme.json` file, inspect the
preview, then **Apply theme**. Installation/selection use the existing guarded
settings acknowledgment path. No settings file was modified by creating these
artifacts; no root synchronization, commit or push is implied.
If version 1.0 with the same ID is already installed, explicitly confirm
**Replace and apply**. Cancel keeps its contents and selection unchanged.

### Verified revision and reproduction

Ten design/palette regressions pass, including native markup and palette
agreement; focused suite **329**, complete Web suite **1,712 in 96 files**, and
TypeScript/Vite build pass. **16 Chromium groups** pass, including an actual
manager preview, refused replacement, confirmed guarded replacement, cold reload
and exact exported v1.1 values. All settings are intercepted/synthetic; one mock
conditional write, zero page errors and zero unexpected requests. No real API,
project, settings or hardware write is performed.

To reproduce in the isolated worktree, install the existing Web dependencies with
`npm ci --ignore-scripts` in `apps/knx-web`, start
`npx vite --config vite.fixtures.config.ts` there, then in a second terminal run
`node design/verify-crt-reference.mjs` from the worktree root. The script requires
`/usr/bin/chromium`, overwrites its own screenshot/receipt and intercepts every
settings request. Do not point it at a real backend. Owned installed dependencies,
build outputs and scratch logs are removed after delivery; these source artifacts
and the receipt are retained for reproduction.

## Explicit v1 limitations

[ADR-0060](adr/0060-versioned-declarative-theme-packs.md) permits exactly 27
palette tokens. Packs cannot contain selectors, pseudo-elements, keyframes,
motion/density preferences, external font URLs or arbitrary CSS. The token-only
palette therefore cannot implement the requested 250ms fill/flash, dedicated
`#003300` selection, Save-only purple, additional panel/table borders or a new
input-focus glow. Existing hover/elevation consumers can use its green shadows,
but several workbench controls explicitly set `box-shadow: none`.

The standalone study is a visual/interaction reference for those changes, not
proof that they are enabled in the production UI. It does not replace native
WebKitGTK, real screen-reader, all-component contrast or broad WCAG acceptance.

## Original production proposals — design-study delivery, before the follow-up

The application-owned animation and existing keyboard paths are implemented/
verified above. The semantic-token additions and broader accessibility proposals
below are still proposals; this section preserves the original design rationale.

### 1. Separate semantic palette roles, with an explicit token-version policy

Do not special-case `user-modern-retro-green-crt` in component selectors. Add
semantic roles shared by every palette, with CRT values:

| Proposed token | CRT value | Consumer |
| --- | --- | --- |
| `--knx-selection-bg` | `#003300` | Row/tree/topology selection fill |
| `--knx-save-bg` | `#6b21a8` | Save action only |
| `--knx-on-save` | `#e6d9ff` | Save foreground |
| `--knx-focus-shadow` | `0px 0px 10px 0px #39ff1460` | Input/control focus |
| `--knx-activation-shadow` | `0px 0px 16px 0px #39ff1473` | Short confirmation feedback |

Text glow can derive from `var(--knx-accent)` in the component layer; do not
add a permissive free-form CSS token just for text-shadow. Extend contrast tests
for ink on selection and Save. Define values for **all** built-ins. Extend the
validated grammar/DOM lease coherently in `themePack.ts`, `themePackDom.ts` and
`themeTokens.test.ts`; do not quietly make v1's exact token boundary larger.
An explicit v2 admission/compatibility design must keep existing v1 packs usable
through documented derived defaults, retain unsupported packs and diagnose them.
Do not merely accept `tokenVersion: 2` in one frontend function.

Give the Save control in `App.tsx` a semantic class such as `save-action`; the
current toolbar `.primary-action` selector uses `--knx-accent` for all primary
actions. A generic class that recolours every primary button is not Save-only.
The purple-on-green role pair must be evaluated, not assumed accessible.

### 2. Cell-fill behavior belongs to components, not theme-pack executable data

`ProjectExplorer.tsx`'s `TreeNode` already renders a native `.tree-label` button;
`GroupAddressTable.tsx` renders native `tr` rows with a `.table-select` button.
Use a component-owned fill wrapper for button/tree/topology targets. Example
**proposal**, with the prospective semantic token:

```css
.cell-fill {
  position: relative;
  isolation: isolate;
  overflow: hidden;
}
.cell-fill::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  background: var(--knx-selection-bg);
  transform: scaleX(0);
  transform-origin: left;
  transition-property: none;
}
.cell-fill:hover::before,
.cell-fill:focus-visible::before,
.cell-fill[aria-pressed="true"]::before {
  transform: scaleX(1);
}
@media (prefers-reduced-motion: no-preference) {
  .cell-fill::before {
    transition: transform var(--knx-transition-duration) var(--knx-motion-easing);
  }
}
```

Keep existing selection and multiselection semantics; focus/hover must not
silently mutate domain selection. Selected state applies immediately, including
when motion is off. For rows, include `:focus-within` and `aria-selected` in the
visual state mapping. **Do not copy a button wrapper/pseudo-element blindly onto
native table rows** or switch the engineering table to layout buttons. Prototype
the row-background renderer against the existing table's widths, backgrounds,
checkboxes, scrolling and both Chromium/WebKitGTK before selecting a technique.
The **1.1 study** verifies a native-row background gradient whose size grows from
0% to 100%, without generating pseudo-cells on `tr`. Its noninteractive light
overlay is a sibling of the native table, aligned from the row/container bounds;
scroll, resize, completion and motion changes retire the pulse. This supplies
Chromium study evidence only. Adapt the renderer through component refs/lifecycle
and check WebKitGTK and the real engineering table before production adoption.
The `::before`/scale technique above remains suitable for tree/button surfaces.
The desktop study uses a viewport-bounded grid/flex shell and local table scroll,
so the footer stays visible. Sample transient effects at a fixed viewport:
full-page screenshot capture can temporarily resize the page and correctly
trigger the overlay's resize-cancellation handler. The final capture waits for
native pause readiness/paint and asserts that its overlay remains visible.

### 3. Flash on activation, without delaying selection or writes

Attach feedback at the existing **one activation callback**. A native button's
Enter/Space click should not also fire a duplicate flash in `onKeyDown`. Do not
use CSS `:active` alone: it reflects a held input, not a bounded post-confirmation
pulse. Maintain a short-lived presentation-only generation/attribute, retire
its timer/animation on completion, unmount, motion-disable or OS preference
change, and throttle rapid repeat activation across the participating surface.
The revised study uses a shared 600ms activation guard and explicit timer cleanup;
this is not a flash-safety or WCAG certificate. Select immediately and paint the pulse over the selected
state. Save activation feedback is **not** a successful-save acknowledgment;
only the existing successful API result may display “Saved”.

### 4. Add an independent CRT interaction/motion choice

Keep motion owned by `motion.ts`, `index.html` pre-mount preference bootstrap,
`SettingsPanel.tsx` and the component setting blocks. A new explicit CRT style
can map Standard to `--knx-transition-duration: 250ms` and `ease-out`, while
preserving Off/Subtle and the existing Smooth/Glitch behavior. Model any added
interaction-effect choice explicitly; do not infer animation permission merely
from the selected theme ID. Keep both user Off and OS reduced motion effective.
Use the feedback duration through the existing motion ceiling for flash.

The strengthened study test caught an important cancellation case: removing the
guarded transition declaration restored the default transition property `all`,
which let already-running input box-shadow and row-transform transitions finish
after Off. The study now uses motion-free baseline `transition-property: none`
and `animation-name: none`, overridden only by enabled/no-preference rules;
the immediate live-animation assertion passes. Test disabling **mid-transition**,
not only mounting with Off. This finding is about the study, not a verified
production-runtime defect or permission to refactor the existing motion engine.

Avoid `transition-all`: enumerate transform, background, border and shadow
properties actually involved, so geometry/drag layout do not animate accidentally.
Tailwind utilities may express these component rules if Tailwind is adopted in a
separate build decision; Tailwind is absent from the current manifest and is not
required for these effects. A literal `duration-[250ms]` utility must not bypass
the existing `motionGuard.test.ts` setting/OS guard.

### 5. Keyboard navigation is functionality, not decoration

The current `TreeNode` buttons implement native activation and independent
expansion; they do not expose a complete ARIA tree/arrow-navigation contract.
Add visible-node Up/Down/Home/End and parent/child Left/Right navigation with an
explicit focus model, collapsed-node exclusion and preserved multiselection.
Do not hijack arrow keys from editable inputs, selects or checkboxes. Keep focus
and selection distinguishable. Consult the WAI-ARIA tree guidance below; adopt
roles only alongside the complete associated behavior, not for visual styling.

### Acceptance for that follow-up

- Existing v1 packs still admit/roundtrip; unknown/newer content is retained and
  diagnosed. All built-ins and new semantic role pairs pass contrast tests.
- Hover/focus fill starts left at zero, finishes at full width and persists only
  for selected/hovered/focused state; unrelated rows are unaffected.
- Click/Enter/Space activate once; repeated confirmation feedback is bounded;
  unmount/theme switch/motion-off leaves no stale animation or timer.
- Off and OS reduced-motion keep static selected/focus feedback without flash
  or animated fill. User preferences remain unchanged by theme import.
- Native table semantics, checkbox selection, Shift/Ctrl selection, editing,
  collapsed-tree traversal and focus restoration survive the changes.
- Browser proof on the real components, plus separate WebKitGTK/manual evidence
  before native claims. No KNX project operation or hardware write for a style.

## Primary references checked for the proposal

- W3C WAI-ARIA APG, Tree View Pattern:
  <https://www.w3.org/WAI/ARIA/apg/patterns/treeview/> — focus navigation versus
  Enter activation and supported tree roles.
- W3C WAI-ARIA APG, Developing a Keyboard Interface:
  <https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/> — intra-component
  arrows, inter-component Tab, visible focus and selection distinction.
- MDN, `prefers-reduced-motion`:
  <https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion>
  — OS preference conveyed to CSS. The repository's stricter motion rule is
  local product policy, enforced by `motionGuard.test.ts` and ADR-0022.
