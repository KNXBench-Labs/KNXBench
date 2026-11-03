# WP-28 — group-address sub-range navigation acceptance

## Result — 3 November 2026

**Delivered on main:** `e1d6f916e`. A local range selection in Columns or Atlas
replaces the initial Explorer scope instead of intersecting unrelated ranges.
Every later Explorer range activation resets local navigation, including a fresh
activation of the same numeric range ID. Address selection does not reset it.
The existing stored-membership model, occurrence-safe keys, filtering, selection
and row windows remain unchanged; Atlas still renders direct members only after
local range selection, not every descendant or the all-ranges overview.

[Task and causal reproduction](../open-work/WP-28-ga-columns-atlas-empty-table.md).

## Discriminating evidence and review

- The original rendered-controller regressions failed in both views: a populated
  child received an empty list instead of the two expected stored address IDs.
- Independent review found same-ID Explorer reactivation could leave local
  navigation active. Two additional assertion-based REDs reproduced it; the
  guarded existing selection event now restores Explorer navigation without
  changing App or introducing another generation/API contract.
- Correction re-review approved. The final focused run passed 66 tests in eight
  files, including both selection states, empty/sibling isolation, search/filter,
  stored rather than numeric membership, selection continuity and 500-row windows.
- Eight strict full-App browser cases cover Columns/Atlas, EN/DE, 400/1440 pixels,
  the actual Explorer route, exact ordered rows, search, genuine empty children
  and same-ID return with/without an intervening address selection. Each passed
  under the default, group-addresses and table-columns configurations. All 24
  corresponding development/production frames were visually inspected; horizontal
  scrolling remains local to the narrow table, not page overflow.

## Integrated acceptance

Main-first `93873f86b` on published catalogue prerequisite `5296e56c5` has exactly
the feature candidate's tree. The four reviewed WP-28 files match the retained
focused source hashes. Its complete selected package gate passed all 11 stages,
with frozen inputs and checked, unchanged user data:

| Executed boundary | Result |
| --- | --- |
| Formatting, documentation, browser lanes, five xtask checks, tool tests, web gate, whitespace | 11/11 package stages |
| Web build, all selected fixture type checks and lanes | 39/39 web stages |
| Vitest | 5,043 passed; zero failed |
| Default Chromium functional / exclusive | 848 / 46 passed |
| Production group-addresses functional / exclusive | 29 / 1 passed |
| Production table-columns functional / exclusive | 164 / 4 passed |

The browser rows are 1,092 executions, not a claim of 1,092 distinct scenarios.
Inventory-only list results are not executions. Rust tests, Clippy, bindings and
private-corpus stages were **not selected** for this web-only change; their
absence is not reported as fresh Rust or corpus acceptance.

The conditional push correctly refused when main gained catalogue documentation.
Published successor `e1d6f916e` is the accepted tree plus that exact raw upstream
patch, independently composed in an alternate index with a mismatching-tree
negative control. Runtime sources, build and tool identities are unchanged.
Documentation, tool tests, whitespace and commit-local-info delta checks passed;
no user data was touched or left unchecked. Local/tracking/live publication refs
were read back equal. The browser results remain bound to `93873f86b`, not
mislabelled as fresh executions on its documentation successor.

## Retained failures and limits

The first branch gate remains failed, not waived: three byte-unchanged German
footer cases raced settings hydration while dismissing a late-night toast. A
local test readiness check waits for the actual UI language before using the
unchanged shared helper. Independent review approved; the three cases passed five
repetitions each in default and communication production configurations. That
identical prerequisite correction reached main in `f3366cfe9` and was dropped as
a duplicate when rebasing WP-28. No notification overlap or no-connect assertion
was removed. Earlier browser assertion/fixture mistakes remain separate failed
attempts, not product-regression REDs or accepted gate stages.

This is fictional-data software acceptance. The original private project,
native WebKitGTK/Tauri behavior, a live backend, real KNX installation, release
and deployment were not verified or authorised by this package. The outer
Explorer heading and the local range breadcrumb retain their existing separate
roles; this is not a heading or creation-target redesign. Broader WP-11/native
and all unrelated compatibility/hardware limitations remain open.

Private raw evidence and failed attempts are retained under
`<profile>/evidence/2026-11-03-wp28-group-address-subranges/`; no local paths or
private project bytes are required to reproduce the committed fictional fixtures.
