# Four production group-address presentations

## Authority and scope

Owner invocation: uploaded goal on 2026-10-12. All four production views are
mandatory: expandable tree table, column browser, semantic address atlas and
directional communication-object matrix. The global Explorer stays stable.
Editing remains in existing Inspector/command services. Tree table is the initial
engineering default; only the presentation preference is remembered outside
project data. No release, deployment, user-container or bus actions are authorized.

The [design record](../design-studies/group-addresses.md) includes the newer owner
planning addendum; exact local studies were preserved before isolated worktree
creation. The standalone study is inspiration, not a DTO or acceptance fixture.

## Ownership

Session `5f9107fd0635` owns this goal and new independent presentation modules in
`feature/group-address-four-5f9107fd`, initially based on upstream `f0de988b`.
Root HEAD/index/foreign files and all other worktrees remain untouched.

On 2026-10-12 the owner explicitly chose: **keep owners separate; build the
independent foundation now; integrate after the DPT owner publishes and hands
off**. DPT-presentation has unpublished changes in GroupAddressTable,
groupAddressView, translations and styles. This session does not modify, accept,
copy or publish those changes. Existing Functions/Sites and other shell owners
also remain separate. Recheck fresh upstream, handover and overlap before mounting.
No subagents or quota checks. Reviews are explicitly self-reviews.

## Packages and evidence gates

| ID | Work package | Current state | Required verification |
| --- | --- | --- | --- |
| GA1 | Data/hierarchy contract and shared typed foundation | Done | `groupAddressPresentation(.test|Scale.test).ts`: malformed/duplicate/cyclic/unassigned, immutability, 10,000-level chain |
| GA2 | Tree table and column browser | Done (integrated on DPT publish `fe09a41a`) | `GroupAddressWorkspace.test.tsx`, existing `GroupAddressTable.test.tsx`, browser gate |
| GA3 | Semantic address atlas | Done | Shared scope/filter/selection, drill-down, narrow geometry in browser gate |
| GA4 | Directional connection matrix | Done | `groupAddressMatrix.test.ts`; exact device/object IDs, bounded explicit column choice (≤12), Send/Receive/unknown/absence |
| GA5 | Integration and hardening | Done | `e2e/group-address-views.e2e.ts` on the production preview: EN/DE, light/dark, 1440/400 px, Three/Two-level/Free, project reset, stale-update pruning, 5,000-address timing; existing related e2e and native rename gate |
| GA6 | Matrix object inventory (unlinked objects) | Done 2026-10-12 | See below |
| GA7 | Derived 2-/3-level tree from address numbers | Parked: waiting for feedback on issue #4 | See below; ADR first |
| GA8 | Virtualization instead of the 500-row window | Parked: no feedback request, no failing measurement | See below |

## Follow-ups after the first publication (2026-10-12)

**Feedback status.** Issue
[#4](https://github.com/KNXBench-Labs/KNXBench/issues/4) was asked for short
feedback (tree fit, numeric fallback for projects without stored ranges, use of the
other views) on 2026-10-12 09:28 UTC. At the check before GA6 (same day) nobody had
answered, so GA7 and GA8 were not started. The issue stays open until the reporter
confirms.

**GA6 acceptance criteria (met).** Unlinked objects enter the matrix only through
the canonical per-device read `api.deviceDetail` (`GET /api/device/{id}`, same
product language as the Inspector); no UI object model, no invented objects.
Reads happen only on an explicit per-device **Load objects** action. The scope
note switches from "objects with project links" to "linked plus loaded devices",
and a status line counts loaded / not loaded / loading / unavailable devices.
Failures and replies naming another device are shown as unavailable with Retry,
never as absence. Replies are bound to request generation, project snapshot,
language and project; outdated replies are ignored, loaded devices are re-read
after updates (their chosen columns survive the refresh) and forgotten on project
switch. Linked objects missing from a loaded list are reported. The 12-column
limit and explicit selection remain; loaded objects are offered, never
auto-selected. Evidence: `groupAddressMatrix.test.ts`,
`groupAddressObjectInventory.test.ts`, `GroupAddressMatrixInventory.test.tsx`
(RED first, five guard mutations caught), production-preview browser gate with an
intercepted read and a failing read, and a narrow-layout negative control.

**GA7 preconditions (if feedback asks for it).** ADR before code. The derived
tree must be visibly marked as derived from address numbers (not stored data),
never mixed with stored ranges, never written to the project, and switchable
(off by default unless decided otherwise in the ADR). Acceptance criteria will be
derived from the reporter's answer.

**GA8 preconditions.** Only on a feedback request or a measurement showing the
render window is insufficient. Baseline: 0.6–1.5 s per view switch at 5,000
addresses (remeasured during GA6: 0.64–1.49 s). Measure first; no new dependency
without a reason.

## Completion

The goal is complete only when all four views are mounted in production, jointly
verified, documented and published on main with exact remote readback. A model,
plan, prototype or unmounted component never counts as completion.

Publish only reviewed green work packages at sensible boundaries; if upstream
changes integration inputs, rerun affected gates. Keep failed attempts separate
from their successors. Retain necessary evidence, clean only own scratch/runtime
artifacts after acceptance. No full accessibility, native GUI certification or
ETS compatibility claim without corresponding execution.

## Presentation invariants

- Stored GroupRange parent/membership is independent of numerical notation.
  Never invent functional names, change address style or rewrite project structure.
- Derive read-only occurrence rows, retain malformed and ambiguous data, and
  report problems rather than silently dropping them. Ambiguous stable IDs are
  visible but must not be offered as safe edit targets.
- One controller owns search, filter and local navigation scope; App remains
  selection/Inspector/command authority. All views pass their actual visible
  order to the existing shared selection handler.
- View preference is not project data. Project incarnation and installation
  scope invalidate local navigation; updates remove stale IDs. View switching
  retains common selection, query, filters and scope.
- Matrix reverse links prove only linked-object scope, not a complete object
  inventory or measured traffic. Unlinked columns come only from explicitly
  loaded canonical device-detail reads (GA6); failed/stale reads are unavailable,
  never absence. Unknown directions remain visible.
- No direct matrix writes, spreadsheet editor, geometric zoom/pan, live overlays,
  layout persistence, framework/plugin/schema additions.
