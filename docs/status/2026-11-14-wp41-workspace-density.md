# WP-41 — Workspace density and technical appearance

## Delivered scope and authority

**Source accepted and published on main 14 November 2026:**
`836824491428367a34d79577bcbd9dc34c42386e`, on base `475b3a793`.
The owner accepted static design revision 2 and separately authorised phases
B–D: implementation, independent review, relevant gates and source publication.
This is **main-only, after Alpha 21**, not a new release or deployment.
[Work-package decisions and ownership](../open-work/WP-41-workspace-density-and-technical-appearance.md).

- One shared scalar slot: **10 rem / 160 CSS-px at 100% UI scale**, including
  prefix/unit; one larger declared Text/name/description class up to **24 rem**,
  bounded by the pane. No content-based individual widths. Declared smaller
  matrix cells and constrained panes still limit available width; raw evidence
  and checked sliders keep their existing layout owners.
- Shared Compact/Comfortable spacing and 32/40 px control minima, independently
  of palette, across the device list, parameter workspace and Inspector.
- Optional **Technical Light / Technical Dark**, using the existing trusted
  built-in presentation and locally bundled fonts. No new component tree,
  preference/schema/storage model, animation or imported-pack layout authority.
  [ADR-0157 addendum](../adr/0157-rail-built-in-presentation.md#addendum-wp-41-technical-pair-14-november-2026).
- Existing parameter navigation, group identity/order, conditional captions,
  declared matrix placement, App/E4 focus ownership and domain mutations remain
  owned by their existing components. This does not close WP-05 or WP-11.
  Only actually missing source semantics remain deferred; WP-41 did not wait
  for all of WP-05 or duplicate its structure engine.
- Device columns revise initial defaults through the existing resizing hook;
  stored browser preferences are not rewritten. Address default is 82 px and
  Devices inline cell padding 10 px.
  [Independent geometry/resize contract](../contracts/table-column-widths.md).

## Independent review and corrections

Whole-feature Standards and Spec reviews approved the corrected source after
pressed-primary contrast, Inspector text-slot specificity, dropdown inventory,
density spacing and actual source-notice visibility were corrected. Bounded
follow-up review approved the ADR/header explanation and all-nine-palette matrix
population. Optional nonblocking CSS/naming/helper-style heuristics were not
turned into an unrelated production refactor.

The first integrated browser gate exposed twelve old test oracles, not a new
production change: the old address default, compact Devices totals and theme
inventory. `c251898ee` re-pins them to the accepted design without weakening the
per-column intrinsic oracle, inflated-floor rejection or tolerances. `721c52dcd`
updates the documented declared budget and measured/history table; independent
re-review closed both documentation findings. Both affected spec files then
passed in full: **62 cases**, no unexpected, skipped or flaky tests, on
`e2a79a83c`. That focused epoch is not the final package acceptance.

## Final integrated acceptance

The maintained change-scoped package gate ran on **`836824491`** against
**`475b3a793`**. It selected exactly twelve stages, all direct exit 0 with valid
owning proofs, complete plan and matching frozen inputs:

`fmt`, `documentation`, `browser-lanes`, `check-layering`, `check-headers`,
`check-anchors`, `check-ledger`, `check-corpus-gates`, `tool-tests`, `web-gate`,
`diagnostic-sections-tests`, `diff-check`.

The user-data guard checked its scope and reported no touched or unchecked paths.
This is a complete **selected** gate, not a full-workspace/release or private-corpus
run. The narrow server delta adds only the two shipped-theme IDs and their
positive diagnostic test tuples.

| Owning population | Actual final result |
| --- | --- |
| Nested web gate | 43 stages, all valid; fresh build and all 27 fixture TypeScript configurations |
| Vitest | 5,810 passed, no failed/pending/todo tests |
| Default browser | 1,284 functional + 97 exclusive passed |
| Production dialogs | 26 functional passed; no exclusive cases selected |
| Production devices | 237 functional + 1 exclusive passed |
| Production table columns | 243 functional + 4 exclusive passed |
| Python tool suite | 563 total: 547 passed, 16 optional skips |
| Entire diagnostic-section test module | 17 passed, 0 ignored; 1,005 other library tests filtered out |

Browser populations overlap across configurations; do not sum them as unique
cases. Every nonempty browser population reported zero skipped, unexpected or
flaky cases. The expanded declared-matrix cases cover all nine palettes,
both densities, EN/DE and 400/1440 px in their selecting configurations.

## Visual, focus and mutation evidence

Six actual Chromium/App captures over fictional intercepted data were inspected:
device list, Inspector and focused parameters in Technical Light/Compact/English
and Technical Dark/Comfortable/German. The Inspector obstruction was dismissed
through ordinary UI. List frames precede that dismissal and may contain a
notification in the empty Properties pane, not over the table; parameter frames
represent one local scroll position, not every row simultaneously. There is no
invented density-gain percentage or inference of native acceptance from these frames.

Settings remains normal page navigation: its round trip retains drafts/selection
but adds no automatic arbitrary inline-input return-focus policy. The separate
active-editor settings-refresh checks retain exact DOM identity, draft, visible
focus and continued real keyboard input across palette/density changes.
80/150% and constrained pane cases retain their actual consumer paths.
[Settings contract](../contracts/settings-workspace.md) ·
[E4 ownership](../contracts/editor-preview-focus.md).

Six isolated CSS mutants were assertion-killed; baseline, neutral and restored
controls were green (nine runs / 27 tests). These and the earlier focused/visual
receipts keep their original source bindings. They are not falsely relabelled as
fresh mutation or visual executions of the final integration. Final gate4 is the
current runtime acceptance; earlier red, queued and interrupted attempts remain
separate, including gate1's invalid snapshot-status refusal and interrupted gate3.

## Publication and remaining limits

The native gate finished 14 November at 16:00:52 CET. Its conditional publication
rechecked the released reservation and published Alpha 21 tag, pushed only after
acceptance, and verified local/tracking/live main equality at `836824491`.
The parent subsequently fetched main and confirmed the published head again.
Private receipts, original logs/traces, review reports, manifests, captures and
source overlays are retained under
`<profile>/evidence/wp41-implementation-20261114/`; `acceptance-summary.json`
indexes the authenticated final populations. Retention/owned resource cleanup is
tracked locally, not granted to foreign sessions by this source publication.

No private corpus, real backend/project installation, bus/device, VM, native
shell/Orca/Firefox, full Rust workspace, new ETS compatibility, release or
deployment was exercised or authorised here. Existing WP-05 semantic and WP-11
native boundaries stay open. The intentionally stale/dirty shared root was not
synchronised; private files and foreign work were preserved. A result-only
Markdown successor may carry this acceptance only with unchanged runtime inputs
and its own relevant documentation gate; it is not another runtime execution.
