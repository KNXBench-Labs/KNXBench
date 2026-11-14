# WP-41 — Consistent workspace density and an optional technical appearance

Owner request: 13 November 2026; target agreed after design questions Q1–Q10.
**Design revision 2 accepted and separate implementation Go granted on
14 November 2026.** Phases B–D are now authorised: shared density/layout,
technical light/dark presentation, independent review, relevant gates and source
publication. No release or deployment authority. The delivered package was
removed from [OPEN_WORK](../OPEN_WORK.md); the decisions remain recorded here.

**Priority:** Owner-authorised bounded implementation; no release target assigned.
**Closed 14 November 2026** (removed from [OPEN_WORK](../OPEN_WORK.md)): phases
A–D delivered at the bounded software/browser scope. Runtime `836824491` is
accepted and published on main after independent review and all twelve selected
package-gate stages. Main-only after Alpha 21, no release/deployment or native
acceptance. [Measured acceptance and limits](../status/2026-11-14-wp41-workspace-density.md).

### Accepted revision 2 geometry

Use a shared scalar value slot of **10 rem (160 CSS-px at the reviewed 100% UI
scale)** for numbers and ordinary choices, not content-based widths. Keep the
same width across densities and appearances; prefix/unit belongs to that slot.
Use one larger text class, up to **24 rem**, for names/descriptions, bounded by
the actual pane. Preserve long text access and constrained-width fallback.
Compact/Comfortable control targets remain distinct (reviewed 32/40 px), while
shared alignment, hierarchy and existing theme identities remain authoritative.

## Requested outcome and owner decisions

Improve space utilisation coherently across all themes, rather than shrinking
isolated controls. The existing web-like character may remain. Additionally offer
a technical appearance as a new built-in light/dark theme pair.

- Preserve the explorer/workspace/inspector foundation and familiar workflows;
  improve alignment, visual hierarchy and arrangement within those areas.
- Use consistent spacing, aligned label/value rows, content-appropriate field
  widths and fewer redundant containers. Related short fields may share a row
  when there is sufficient width; return to a single-column arrangement when
  needed. Preserve declared manufacturer groups and ordering; do not invent
  relationships from translated captions or fill columns arbitrarily.
- Gain useful space without indiscriminately reducing font sizes, concealing
  relevant information, hiding warnings or weakening keyboard/focus usability.
- Keep Compact/Comfortable density independent of theme. Apply a coherent
  density system to both and preserve saved theme/density choices.
- Add a calm IDE/CAD-like technical appearance: clear area boundaries, straight
  controls, restrained elevation and precise table/property presentation. Provide
  matching light and dark variants; keep Rail/Rail Dark and the other existing
  theme identities rather than repurposing them.
- Use the same improved arrangement, components, functions, data and workflows
  for both appearances. This is not a separately maintained second UI, an ETS
  copy, a navigation redesign or an animation feature.
- Judge the overall composition across the device list, parameter workspace and
  inspector, not an isolated sample card. Exact colours, dimensions and theme
  names are design work, not decisions already made by this package.

## Existing basis and gap

Source inspection was performed on `38d7ad5f5`; package intake is based on
`99a78fe35`. This is source evidence, not a measured screenshot comparison,
visual acceptance or a compatibility result. Recheck current source before any
future design or implementation; reuse the following mechanisms:

- [Appearance preferences](../../apps/knx-web/src/appearance.ts) already offer
  Compact/Comfortable, with Compact the default. In
  [styles.css](../../apps/knx-web/src/styles.css), shared control height and
  table padding follow density, but many toolbar/panel/form spacings are fixed.
- [Parameter workspace styles](../../apps/knx-web/src/parameterWorkspace.css)
  already provide aligned label/value rows and declared matrix layouts. Build
  on them; do not replace evidenced source layouts with a generic form.
- [ADR-0022](../adr/0022-theme-token-boundary.md) separates palette from
  setting-owned geometry. Imported v1 theme packs have a restricted token set,
  not arbitrary CSS/layout authority; a new palette file alone cannot deliver
  the requested density work.
- [Theme registry](../../apps/knx-web/src/theme.ts) and
  [ADR-0157](../adr/0157-rail-built-in-presentation.md) provide the built-in
  presentation mechanism used by Rail. Reuse that boundary for the optional
  appearance without silently overriding density or accessibility preferences.

At intake, the gap was a consistent composition and spacing system plus the
optional technical presentation. The accepted revision 2 and delivered source
now implement that bounded scope; no space-saving percentage is inferred.

## Bounded phases and actual status

| Phase | Outcome | Actual status / entry condition |
| --- | --- | --- |
| A — Static design comparison | Read-only current-state inspection and static before/after comparisons for device list, parameter workspace and inspector; shared representative data across variants; proposed baseline and technical light/dark composition | Accepted revision 2 on 14 November; fictional static dossier retained privately |
| B — Shared density/layout | Apply the accepted spacing, field-width and visual hierarchy system across affected existing themes and both densities using common components | Accepted and published on main `836824491`; shared components, existing themes and both densities verified |
| C — Technical theme pair | Add the accepted built-in light/dark technical presentation on the same components and improved layout, preserving other theme identities | `technical-light` / `technical-dark` accepted on the existing trusted presentation boundary; main-only |
| D — Integrated verification | Verify the complete affected UI and relevant regressions; retain visual evidence and the scoped acceptance record | Complete selected package gate 12/12 and nested web 43/43, source-bound review/visual/focus evidence; publication verified |

### Historical author-phase B–C boundary

The author-phase candidate starts from `d44a02d72`, retaining the published
navigation and E4 focus fix. It uses setting-owned panel/row spacing (12/4 px Compact, 16/8 px
Comfortable) and the existing 32/40 px control minima. Scalar slots stay 10 rem
in both densities; an explicit declared-Text/name/description class is bounded
at 24 rem. Units and address prefixes share the scalar slot. Narrow panes stack
labels above controls. Text rows reserve label space rather than consuming the
whole pane; checked sliders and raw evidence retain their own usable layouts.

Device columns use revised **initial** widths through the existing resizing
hook, not CSS overrides of stored preferences. The table remains native and
locally scrollable. No parameter navigation, evaluated identity, declared cell
placement, conditional caption or mutation handler is changed.

Technical Light and Technical Dark use `data-presentation="technical"`, complete
fixed-accent palettes, the existing Inter/JetBrains Mono fonts and restrained
2 px geometry. They add no animation and do not alter System, Rail or pack
admission. The diagnostics inventory names both built-in IDs.

Authored regressions extend the existing Devices/matrix browser fixtures and
theme/control/dialog inventories; unit coverage pins bootstrap/runtime identity,
the exact accepted colours, setting independence and metadata-owned width
classes. **No tests, builds, gates or browser servers were executed in the
author phase.** No measured density gain, screenshot review, publication or
native acceptance was claimed by that author handoff. The parent subsequently
completed phase D; the final source-bound result is recorded below.

First independent review identified pressed-primary contrast, an Inspector text-slot
cascade conflict and stale/incomplete regression oracles. The candidate corrects
these through the existing shared presentation boundary; later parent-owned
checks established integrated acceptance and publication on `836824491`.

The second independent Standards and Spec reviews approve the frozen source
candidate. Bounded follow-up documents the technical boundary in ADR-0157,
explains the scalar-anchored shared header and extends the declared-matrix
population to every existing palette and both densities, languages and viewport
widths. Their selecting configurations subsequently passed in the final package
gate. Source-review approval itself is not runtime acceptance or publication.

Focus coverage keeps the existing [Settings page contract](../contracts/settings-workspace.md):
its round trip preserves drafts, selection and settings, but adds no automatic
inline-input return-focus policy. A separate in-place server-settings refresh
checks the still-active editor's exact DOM identity, draft, keyboard focus and
continued typing across palette/density changes. Existing E4 preview/cancel
ownership stays unchanged; it is not extended into a global focus manager.
Scaled real-consumer checks cover 80%/150%, wide/narrow viewports and constrained
Inspector text/address fields; source warning/error checks require visible,
nonempty and locally reachable rendered text, not only DOM content.

Phase A is a design artifact, not product verification. Label the
before view with the inspected source/build and the after view as a proposal;
use public-safe representative data. Do not publish private project content or
manufacturer originals. A static after view does not establish interactive or
native-shell acceptance. Do not start a clickable prototype as part of Phase A.

## Acceptance contract

### Static design acceptance

- Present all three views together with consistent selection/data between
  variants; show the improved existing appearance and technical light/dark.
- Make label/value alignment, field-width policy, spacing hierarchy, related
  fields and constrained-width fallback inspectable. Include long labels,
  source-defined groups, warning/error states and enough rows to assess density.
- Record current and proposed dimensions at matching viewport/UI scale, and
  explain the useful space gained without inventing an improvement percentage.
  Static annotations are proposals, not browser measurements of implemented UI.
- Obtain owner acceptance of the complete composition before product changes.

### Product acceptance after separate implementation Go

- Exercise all affected existing themes and the new light/dark pair with both
  density choices. Preserve saved choices and verify theme/density switching
  without data, draft, selection or workflow changes caused by the appearance.
- Verify content-appropriate widths, long EN/DE labels, declared parameter
  row/matrix layouts and constrained panes/viewports/UI scales. Check usable
  table-body space and local scrolling, not only absence of outer overflow.
- Preserve all relevant values, original grouping/order, labels, diagnostics,
  pending marks, accessible names, keyboard paths and visible focus. Theme
  changes must not gain mutation authority or conceal warnings/errors.
- Verify computed geometry and actual rendered views, including focus and
  disabled/error states and reduced motion. Record browser/native-shell
  coverage separately; do not infer native acceptance from static drafts.
- Review common component consumers and run the relevant scoped package gate.
  Saving this plan does not implement or execute these acceptance cases.

## Final acceptance and publication

Runtime `836824491` passed the complete selected twelve-stage gate on base
`475b3a793`, with frozen inputs, valid direct stage proofs and checked user data
untouched. Nested web: 43 stages, all 27 fixture TypeScript configurations,
5,810 Vitest cases; default browser 1,284 functional + 97 exclusive, production
dialogs 26, devices 237 + 1, table columns 243 + 4. Browser populations overlap;
none reported skipped, unexpected or flaky cases. Tool suite: 547 passed and
16 optional skips; all 17 diagnostic-section tests passed (other library tests
not selected). Earlier failed/queued/interrupted epochs remain separate.

The conditional push and fresh main readback confirmed source publication.
No private corpus, full Rust workspace, native shell, VM, hardware/bus, release
or deployment was run. Broader WP-05 and WP-11 remain separate. Full evidence
boundaries: [acceptance record](../status/2026-11-14-wp41-workspace-density.md).

## Coordination, dependencies and exclusions

### Boundary with WP-05 and parameter navigation

Owner decision, 14 November 2026: keep the packages separate and coordinate the
shared parameter workspace; shared files do not imply duplicate implementation.
[WP-05](WP-05-parameter-layout-fidelity.md) includes UI work, not just backend
preparation. Its evidence-gated source-layout semantics remain authoritative.

| Concern | Responsibility |
| --- | --- |
| Declared parameter groups, nesting, inline sections, conditional visibility and `CollapseIfEmpty` | WP-05 or its coordinated parameter-navigation slice, under that slice's evidenced contract |
| Declared table/grid placement, axis captions and layout/control-hint semantics | WP-05; retain its delivered behavior and explicit unsupported boundaries |
| Consistent spacing, field widths, control sizes and visual hierarchy across themes | WP-41, within the existing declared structure |
| Additional technical light/dark appearance | WP-41, using the same components and workflows |
| Compact visual treatment of parameter groups and navigation | WP-41, consuming the relevant delivered parameter structure rather than rebuilding it |

WP-41 must not flatten nested groups, reorder declared cells, change conditional
sections/captions or infer missing hierarchy from names to achieve a denser
presentation. Unsupported source semantics stay explicit; a theme is not a
substitute for the missing domain/presentation contract.

### Delivery order and shared edit surfaces

1. WP-05 and the coordinated parameter-navigation owner deliver and accept the
   relevant evidenced hierarchy/layout slice first. Reuse their existing
   captions, declared grids and regression coverage rather than a second engine.
2. Parameter-specific WP-41 design/layout work then builds on that delivered
   structure. Its final design comparison and future product acceptance must
   use the actual accepted structure, preserving group identity, nesting, order,
   captions, conditional visibility and declared row/matrix placement in both
   existing and technical appearances.
3. This is a slice dependency, not a requirement to finish all of WP-05. General
   shell spacing, device-list/inspector composition and theme-palette work are
   not blanket-blocked by unrelated WP-05 features. They still require WP-41's
   own design/implementation Go; the three-view design acceptance remains intact.
4. The parameter workspace, its styles and server/caption/grid projections are
   shared edit surfaces. Before future edits, re-read current coordination,
   agree revision and ownership with the active WP-05/parameter-navigation
   owner, and preserve its frozen inputs. A disjoint worktree grants neither
   source ownership nor permission to refactor another track's contracts.

The boundary clarification itself granted no execution authority. The subsequent
14 November owner decisions accepted static design revision 2 and separately
authorised phases B–D. Generic parameter navigation2867ee1b5/af05c8e57 is already
published and reused; only missing semantic slices remain deferred. This does
not grant VM/bus access, release/deployment authority or product acceptance.

### Other dependencies and exclusions

- Reconcile shared parameter, inspector and table consumers with
  [WP-37](WP-37-pending-changes-across-ui.md), and native acceptance boundaries
  with [WP-11](WP-11-native-linux-acceptance.md), before future edits.
- Device-name/product-name fallback remains separate. Do not silently populate
  stored names or add name resolution as a side effect of theme work.
- No new domain/schema/storage model, import-format behaviour, bus/protocol
  behaviour, device action, release, deployment or current-installation change.
- No source ownership or execution lease is acquired by this planned entry.
  Re-read current coordination and obtain the applicable Go before each phase.
