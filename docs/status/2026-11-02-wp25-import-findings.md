# WP-25: import findings after reopening

Source delivered 2 November 2026, main-first merge `3455fdb9ca604be6141002172523cdcd889b2405`
(first parent `95d968f46`, feature `e91d232df`, guard correction `cbc480864`).
Main-only after Alpha 19; no new release, deployment or bus operation.

## Cause and smallest solution

Native opening already restored primary import counters from persisted evidence
but reset the session log without re-projecting the counted findings. A real
SQLite native-store regression observed `(1, 4)` counters with zero `import:*`
entries. Fresh import was the only original producer of those entries.

Re-project admitted stored primary errors/warnings after a successful native
replacement and reuse the existing log UI plus explicit full-report reader.
A new findings workspace would duplicate that navigation/filter/export surface.
The original JSON is never rewritten, selective-source reports are not mixed in,
missing fields are not invented zeros, grouping agrees with admitted counts,
and checked occurrence aggregation cannot wrap. Historical/open-time provenance
remains visible even without the informational restoration line. A capped or
missing list is explained and the complete report is explicitly accessible.
[Contract](../contracts/native-import-evidence.md#session-log-projection-on-native-open-wp-25-main-only-after-alpha-19) ·
[Task and remaining boundaries](../open-work/WP-25-import-findings-after-reopen.md).

## Review and regression proof

- Independent whole-feature review and focused guard-correction re-review:
  **APPROVE**, no blocking findings. Runtime content remained identical across
  the unpublished rebase; the two inventory corrections changed tests only.
- Original-source native-store RED: two expected failures plus the legacy control;
  repair RED additionally exposed sparse-field and overflowing-total defects.
- Independent isolated controls: 16 tests passed. Removing restoration, wrapping
  occurrence addition and bypassing count parity each failed the intended tests.
  An intermediate restored run used stale compiler output and remains uncredited;
  the final byte-restored, recompiled control passed all 16.
- EN/DE at 1440/400 px: eight focused production and eight focused default-dev
  browser cases passed. They exercised the actual full-report download, retained
  unknown report fields, refused unexpected APIs and checked narrow overflow.
  Four element-scoped screenshots were inspected. These are API-fixture browser
  proofs, not a live-server/native-shell or hardware acceptance.
- Companion graph/API inventories remain exact, project-mutation census unchanged;
  runtime assertions pin that the companion offers/requests no full report.

## Maintained change-scoped gate

The corrected feature and main-first integration have the same complete Git tree
`18f88a28f53a687f5fcf6198a033c9d596e41876`; one maintained gate on the merge
therefore covers both identical trees. The first local gate failed two Web guard
inventories and is superseded, not credited to the corrected tree.

Final gate: **15/15 stages**, every stage exit zero with valid proof, completed
plan, frozen inputs and empty touched/unchecked user-data lists. Execution was
unprivileged in a loopback-only network namespace. Selected scope:

| Check | Result |
| --- | --- |
| Web build and fixture typechecks | Passed |
| Vitest | 4,897 passed, zero failed |
| Default browser lanes | 969 functional + 67 exclusive passed |
| Production dialogs | 26 functional passed; zero exclusive selected |
| Rust (`knx-server`, `knx-desktop`) | 1,662 passed; 69 default-ignored; 7 harnesses |
| Selected corpus | 51/51 passed in loopback isolation |
| Python tools | 482 tests, including 6 optional skips |
| Formatting, docs, browser lanes, five xtask checks, Clippy, diff | Passed |

These overlapping populations are not a unique combined test count. This is
complete acceptance of the maintained **selected-package plan**, not release-full,
whole-workspace, all-private-corpus or hardware acceptance. No generated-binding
stage was selected. The real installation was not contacted.

## Publication

After the accepted gate, a fresh upstream/snapshot check found no movement or
publication reservation. A normal fast-forward push published the main-first
merge; local, tracking and live `main` all read back the exact delivered commit.
The deliberately stale/dirty root checkout was not synchronized or staged.
