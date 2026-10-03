# AR06P KL-150 lexical ModuleDef storage checkpoint — 2026-10-03

## Scope and confirmed cause

Storage-only correction in `crates/knx-productdb/src/dynamic/parse.rs`.
The original single identity and argument-position counter lose the enclosing
scope when a nested definition closes; the empty-element arm also clears that
scope. A definition stack now restores both identity and argument position.
The existing `(program_id, module_def_id, node_id)` keys, transactions,
first-source ownership and supported namespace/size limits are unchanged.
No evaluator, allocation, parameter-writing, UI or protocol behavior changed.

## Evidence accepted at 20:56 CEST

- Initial exact synthetic package RED: compile0/inventory1/exact101, named
  0 passed/1 failed/0 ignored and the actual dynamic-node UNIQUE constraint.
- Expanded unchanged-producer REDs separately prove empty nested definitions
  and argument resumption fail. The nonempty-seeded whole-package rollback
  control passes after correcting its test-only table name to the real
  `package` table. Original rejection and logs retained, not a producer bug.
- Initial focused5/public ProductDB621/0/24 and strict lint/format passed.
- Final expanded public8-command receipt independently reconciled: nine
  ordinary focused tests pass, one explicitly ignored private case; full
  ProductDB625/0/25, compiled ignored inventory25, Clippy `-D warnings`,
  format and whitespace pass. All706 source/config inputs exact.
- Three separately compiled706-input mutants produce five exact named
  behavioral failures: lost enclosing identity at End, clearing at Empty,
  resetting enclosing argument position. A fourth snapshot is byte-exact
  committed pre-fix parser and has its own named synthetic RED. Four compile
  and four inventory stages pass; six named101 failures make14 stages.
  Canonical source was never mutated.
- One explicitly authorized private nested package: same test-profile baseline
  and candidate binaries, compiled ignored inventory1 each, baseline101/0-1-0
  versus candidate0/1-0-0. Candidate's independent lexical scope census agrees
  with durable dynamic-node scope counts; every reported member matches the
  original ZIP bytes and retained blob. Retry succeeds. Original package and
 853-entry manifest unchanged, zero private temporary directories remain.
- Private execution is offline in a user/network namespace. Output is RAM-only;
  only closed execution counts, source/binary identities and the whole-manifest
  aggregate commitment survive. No private item records, raw diagnostics,
  identifiers, paths or payloads are copied into this log/receipt.
- Nine public synthetic descriptor-relative confinement refusals pass.

## Review and corrected preflights

Separate in-session whole-feature review traced both event arms, lexical scopes,
argument positions, static ownership, migration replay, transaction rollback,
source retention, private failure privacy and downstream loader boundaries.
No remaining blocking finding; this is not independent-model approval.
The added private helper initially failed compilation on an actual `BytesEnd`
local-name string type and SQLite's unsupported `usize` conversion; corrected
using the actual APIs and `i64`. A later strict Clippy refusal correctly rejects
`ok().expect()`. A coarse error-unwrapper now discards error payloads explicitly
instead of accepting Clippy's privacy-unsafe raw-error suggestion. Rejected
receipts remain preserved. No lint allowance or workspace trust change.

## Open boundaries

Fresh broad/current-upstream acceptance, the complete original-filename853
CLI measurement, final documentation/publication/readback and owned cleanup
remain pending. KL-150 is not checked or lifted yet. The single private case is
not a replacement for a full853 run or the separately pinned115/113 matrix.
R-MODULE-04 runtime/allocation semantics remain AR07 work. Existing catalogs
with previously admitted mis-scoped rows are not automatically rebuilt;
first-source/idempotence policy remains intact. Migration-style replay is
synthetically verified for eligible retained sources, not a new migration.
Dynamic unknown-diagnostic paths still use their existing templates; retained
bytes, not a new full nested-XPath guarantee, preserve the original XML.
KL-149 cb5781c7 is fully delivered and cleaned; shared root/statistics owner,
foreign worktrees/processes, original data and hardware remain untouched.
