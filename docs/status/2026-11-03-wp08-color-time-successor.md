# Bounded Color/Time successor repair — 3 November 2026

## Accepted scope and source identity

Source `9ed8fca6aebecf77b739703864691edcc4b489b4` was published main-first
as `5c405bfde508a22e2e7001e88615faf4491ec2c6`. The feature and published
merge have identical trees. This is a main-only ProductDB v32 data repair,
not a release, deployment or broader [WP-08](../open-work/WP-08-parameter-value-codecs.md)
codec acceptance.

The [repair contract](../contracts/color-time-successor-repair.md) limits writes
to four existing type columns for authenticated, exact released-v15 bad tuples.
Original history, counters, unknown evidence, source bytes and released migration
bodies remain unchanged. Unsupported or uncertain sources are refused atomically;
the measured declaration index limits orphan candidates but never reconstructs
missing ownership. Builds capped at v31 refuse v32; downgrade is unsupported.

## Historical evidence and corrected-source artifacts

The separate [authentic witness](2026-11-03-wp08-authentic-color-time-upgrade.md)
uses original v14 code `91d319ea4` and the actual v31 chain `176a9b799`.
It is retained unchanged, not relabelled as a v32 execution. The corrected source
has separate invocations, source inventories and an actually executed retained
adapter (`dd46190b23465e8e4bd5771a612e24ff2f72d82551bf500f79f18a1f53813e27`).

Copies of the genuine original v14 and contaminated v31 databases were opened
through the corrected v32 public opener; identical fictional package bytes were
also installed fresh. Independent typed-row/schema comparisons confirmed:

- Exactly two winner type rows change, only within the four permitted columns.
- All 45 other tables and their history remain exact against the retained
  migrated-v31 baseline. Earlier released migrations still run on v14.
- Fresh31 and Fresh32 have identical logical tables, not identical version fields.
- Every recorded reopen is exact and does not append diagnostics.
- All six original input hashes remain unchanged. Original witness inputs were
  never opened writable. Historical encounter counts are preserved, not rewritten
  to fresh counters; whole upgraded-database/fresh-database equality is not claimed.

## Independent review and regression calibration

The initial candidate `fec74a9ea` was blocked for a global orphan-candidate leak:
an unrelated unmeasured source could acquire a refusal and its public diagnostic
reader changed from no measurement to an error. The corrective ten-test run was
RED (8 passed, 2 failed) before the source-local fix and GREEN (10 passed) after it.
A stale v30 comment was corrected to v31 separately within the reviewed delta.

The independent correction review approved source `9ed8fca6a`, closed both findings,
and authenticated the corrected parent's retained artifacts without claiming a
second reviewer execution of those database operations. Its focused pristine
controls passed 10/10 before and after mutation. All ten compiled semantic mutants
were detected: global orphan admission, omitted orphan handling, missing ambiguity
and measurement guards, no-op repair, missing hash authentication, foreign-owner
seeding, exact-tuple proof, source rollback and diagnostic-reader admission.
There were no compilation-error kills or zero-test runs.

Corrected owning-package checks passed: 800 library tests, 1,039 integration tests
(32 ignored), two example suites of 13 each and two doctests; the separate heavy
harness remained ignored. Clippy and formatting passed. These populations overlap
later selected tests and are not additive totals.

## Selected package acceptance

Both the feature and main-first merge completed the exact maintained 14-stage
change-scoped plan with every stage proved, inputs frozen and user data checked
and untouched. Each selected Rust run passed 5,306 tests across 19 test binaries
and five doctest suites, with 176 ignored; each selected private corpus passed 97/97. These are the complete
selected runs, not every workspace, ignored, private-corpus or browser test.
The package selected no web/browser suite and makes no new native-UI claim.
The standard conditional publisher checked the upstream base before pushing;
tracking/live Main was subsequently read back equal to `5c405bfde`.

Raw receipts, source inventories, corrected artifacts, reviews, mutation binaries
and an own-source Git bundle are retained privately. Existing cache executables
captured at closeout have retention-time identities only; they are not invented
historical execution hashes. The first author's original RED executable was not
retained and is not reconstructed. Its original logs/source/hash evidence remains
separate; the corrective RED/GREEN and correction-review binaries are retained.

## Remaining boundaries

Date checked editing/native/image encoding, Colour/Picture codecs, packed Time
semantics and the wider WP-08 evidence remain open. No real bus/device, VM,
manufacturer access, release or deployment was used or authorized by this repair.
Documentation-only successors carry this named source acceptance only while the
runtime, dependencies and gate inputs remain byte-identical; they are not new
Rust/corpus executions.
