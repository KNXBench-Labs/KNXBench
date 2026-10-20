# Alpha 16 successor: migration-repair preparation

Date: 2026-10-19.
Agent: codex.

## Authority and scope

The owner authorised the recommended successor order until redirected:
earlier column-promotion repairs, WP-04's ordered slices, WP-19, then admitted
WP-05 layout work; an independent native-binding build check is allowed.
The claim is published in `2ff5b370`. No release, deployment, desktop control
or real bus/device operation is authorised by it.

The selected checkout is intentionally left dirty and unsynchronised. The own
branch is `feat/alpha16-successor-sequence-20261019`.

## Verified source observations

A read-only worker inspected immutable `041bbd1d`; the later claim changes
only the handover. Controller inspection corroborated the following mechanisms:

- `backfill_text_parameter_refs` tracks current program but not first
  declaration, and traverses Dynamic although static ingest skips it;
- `backfill_placement_columns` skips Dynamic but selects each repeated program
  declaration as long as its source owns the program;
- `Retired::retires` relies on suffix matching, and the earlier promotion
  tables lack the context needed to retain unmodelled static/dynamic hosts;
- `backfill_suffix_text` already has first-declaration tracking and Dynamic
  exclusion for its own v26 promotion;
- `backfill_parameter_attribute_unknowns` skips Dynamic and returns only rows
  absent from the global ledger, so it cannot independently restore each
  shared-source package's missing report evidence;
- `package::add_report_unknowns` preserves non-unknown facts but omits existing
  keys, while `mark_report_unavailable` preserves a missing or unavailable
  report. Repair design must account for those exact helper boundaries.

The proposed contract is `docs/contracts/product-promotion-repair.md`.
This is source analysis, not observed runtime reproduction or acceptance.

## Work in flight

A bounded regression worker is assigned only
`crates/knx-productdb/tests/it/placement_columns.rs`, appending a self-contained
module. It may call existing rewind helpers but may not change source,
registration, rewind modules, docs or another worktree. It must record intended
RED assertions and an unchanged existing GREEN baseline using isolated test
homes and its own target directory. No heavy gate or corpus run is assigned.

WP-17 retains migration v27, master parsing/provenance/replay and report
integration. The source repair waits for its published delivery and claim
release. A notified publication watcher has been started; its dry run correctly
reported not ready. Live handles and retained evidence paths are in private
`<profile-scratch>/alpha16-sequence/continuation.json`.

## Controller correction and runtime handoff

The regression worker appended eight tests without changing production or the
existing test prefix. Controller inspection found its historical setup counted
unknown attributes only for the winning program declaration. Static ingest
reports those attributes before the normalized-row winner check, so a losing
declaration also contributes evidence. This was corrected for both nonempty
and winner-absent caption/placement cases, keeping the first observed sample
and exact occurrence count.

The original queued test attempt was stopped before execution and retained as
superseded with zero test stages. A separate corrected attempt acquired the
common lease and began the unchanged placement baseline at 22:27; no completed
stage was recorded at that checkpoint. The existing file prefix and frozen
test hash were independently verified. A read-only review of the historical
fixture and assertion validity is in flight. These observations establish
preparation and execution progress, not a GREEN baseline or an intended RED.

## Runtime reproduction verified

Corrected attempt completed at 22:28. The controller read the structured receipt
and every actual semantic assertion: unchanged module baselines passed 5 + 4 +
4 + 11 = 24 tests; all eight new regressions failed at their intended
post-migration value/evidence assertions. Observed wrong values are the second
program declaration's caption, placement and enumeration values, including
when the first declaration omitted them. The other failures show the exact
unsupported rows absent after migration. Production hashes and the test hash
were unchanged throughout the run.

This proves reproducible defects and discriminating early-schema regressions,
not repair implementation or package acceptance. The read-only preparation review accepted the corrected test module with no
blocking findings. Its acceptance is limited to the eight historical-entry
regressions, not every table/index or an already-upgraded database repair.
Controller source/receipt inspection corroborates that boundary. WP-17
publication remains pending; already-upgraded and wider failure cases are
required before repair acceptance.

## Pending

Consume and independently verify the regression review. After WP-17 delivery,
re-read its final schema/report contract, allocate the next actual schema
version, amend the narrow correction policy in an ADR, implement test-first,
independently review, gate and publish the first package. Then continue the
ordered goal without another go. No repair code, passing package gate or
migration compatibility acceptance is claimed yet.
