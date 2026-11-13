# WP-41 — Consistent workspace density and an optional technical appearance

Owner request: 13 November 2026; target agreed after design questions Q1–Q10.
**Recorded only; not claimed. No design, prototype or implementation Go.**
The owner explicitly confirmed the target but requested saving a work package,
not starting it. Listed in [OPEN_WORK](../OPEN_WORK.md).

**Priority:** Unscheduled; no implementation priority or release target assigned.
**Status:** Planned only. The next possible phase is a separately authorised
static design comparison; product implementation requires later design acceptance
and its own explicit Go. This entry is not automatic goal-execution authority.

## Requested outcome and owner decisions

Improve space utilisation coherently across all themes, rather than shrinking
isolated controls. The existing web-like character may remain. Additionally offer
a technical appearance as a new built-in light/dark theme pair.

- Preserve the explorer/workspace/inspector foundation and familiar workflows;
  improve alignment, hierarchy and arrangement within those areas.
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

The remaining gap is a consistent, visually evaluated composition and spacing
system across the affected surfaces, plus the optional technical presentation.
No space-saving percentage, final pixel budget or finished design is claimed.

## Bounded phases and actual status

| Phase | Outcome | Actual status / entry condition |
| --- | --- | --- |
| A — Static design comparison | Read-only current-state inspection and static before/after comparisons for device list, parameter workspace and inspector; shared representative data across variants; proposed baseline and technical light/dark composition | Not started; requires explicit design Go |
| B — Shared density/layout | Apply the accepted spacing, field-width and visual hierarchy system across affected existing themes and both densities using common components | Not started; requires Phase A acceptance and explicit implementation Go |
| C — Technical theme pair | Add the accepted built-in light/dark technical presentation on the same components and improved layout, preserving other theme identities | Not started; requires Phase A acceptance and explicit implementation Go; integrate with B rather than duplicating its layout |
| D — Integrated verification | Verify the complete affected UI and relevant regressions; retain visual evidence and the scoped acceptance record | Not started; part of future authorised implementation, not performed by saving this package |

Phase A is a design artifact, not a claim that the proposed UI exists. Label the
before view with the inspected source/build and the after view as a proposal;
use public-safe representative data. Do not publish private project content or
manufacturer originals. A static after view does not establish interactive or
native-shell acceptance. Do not start a clickable prototype as part of Phase A.

## Acceptance for future authorised work

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

## Coordination, dependencies and exclusions

- [WP-05](WP-05-parameter-layout-fidelity.md) owns evidence-gated parameter
  layout/hierarchy semantics, including inline sections, nested blocks and
  `CollapseIfEmpty`. A separate parameter-grouping investigation is pending;
  reuse its conclusions and coordinate with its owner. This package neither
  claims that work nor guesses missing hierarchy or manufacturer behaviour.
  General spacing work is not blanket-blocked on those unrelated semantics;
  any dependent grouping slice waits for its evidenced contract.
- Reconcile shared parameter, inspector and table consumers with
  [WP-37](WP-37-pending-changes-across-ui.md), and native acceptance boundaries
  with [WP-11](WP-11-native-linux-acceptance.md), before future edits.
- Device-name/product-name fallback remains separate. Do not silently populate
  stored names or add name resolution as a side effect of theme work.
- No new domain/schema/storage model, import-format behaviour, bus/protocol
  behaviour, device action, release, deployment or current-installation change.
- No source ownership or execution lease is acquired by this planned entry.
  Re-read current coordination and obtain the applicable Go before each phase.
