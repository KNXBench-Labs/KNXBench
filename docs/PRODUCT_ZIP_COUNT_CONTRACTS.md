# Existing product-ZIP entry-count contracts

## Scope

AR06V verifies the currently enforced entry-count boundary in
`crates/knx-productdb/src/package.rs`, not a proposed limit change.
`MAX_MEMBERS` remains 4096. This is evidence for part of KL-151, not its
byte-size, caller or resource-policy decision. Per-ID status belongs only in
[the source-ID ledger](status/LEDGER.md); this dossier is evidence, not a
second status table.

No production code, manufacturer grammar, hardware behavior or Web source was
changed by this slice. Fixtures are public synthetic Scheme11 XML and safe
empty directories; no new private corpus access or import matrix was run.

## Native contracts

`crates/knx-productdb/tests/zip_cap_boundaries.rs` registers:

- `exact_member_count_cap_installs_and_retains_original_archive_on_replay`:
  exactly 4096 entries install. The complete original archive is retained
  byte-for-byte; same-name replay preserves all persisted values.
- `one_entry_over_member_count_cap_preserves_every_seeded_database_value`:
  exactly 4097 entries receive `PackageError::SizeLimit`. A real seed package
  is installed and its retained archive checked before taking the snapshot.
  Every column/value of every non-system table, including BLOBs, is compared
  after refusal; the retained seed remains byte-exact.

The fixtures assert counts using the actual ZIP reader rather than guessed
raw-header offsets. Empty directories are entries for preflight purposes but
are not counted as imported XML payloads. The native target passed two tests
with no failures or ignored tests; 120 dependency inputs, log and freshly
compiled test binary were independently hash-checked.

## Compiled semantic controls

Only isolated public-source snapshots were mutated. Each used a separate
fresh Cargo target and the identical test file; the canonical source was
never mutated.

- A limit of 4095 fails the exact-4096 contract with its named inclusive
  `SizeLimit` failure.
- A limit of 4097 admits the over-4097 fixture and fails the required-refusal
  contract at `unwrap_err`.

Both controls compiled successfully and each selected test returned 101 with
zero passed / one failed. Logs, source/test/binary hashes and all 120 dependency
inputs per snapshot were verified: the only delta was the intended constant.
These are accepted negative controls, not production test failures.

## Executed regression checkpoints

- Initial six-path feature commit: `65d7cf5c`. Branch public8 passed all eight
  commands: ProductDB640 passed / 0 failed / 25 ignored across31 result blocks,
  strict package Clippy, fmt, four root-bound doc/policy gates and whitespace.
  All784 public source/config hashes were verified. Native2 is part of640,
  not two extra workspace passes; ignored tests are not passes.
- Actual integrated `4dbca7e5` public15: all15 commands0; Rust3147/0/177 over170
  result blocks, Web1761, Chromium90. The single browser environment probe is
  separate, not an additional full-suite case.793 source hashes, all logs and
  the release CLI hash/version were independently verified.
- Actual integrated `0e3e90b2` public15 after owner0028: all15 commands0;
  Rust3147/0/177 over170 blocks, Web1769, Chromium98 plus separate probe1.
  All797 source/config hashes and logs verified; fresh changed-crate/root-bound
  xtask compilation and strict Clippy/CI binding policy passed. Release CLI
  hash/version is bound to this HEAD, not the prior binary. No ignored/private
  or hardware tests are represented as passes.

Separate in-session whole-slice review found no blocking findings; it is not an
independent-model approval. Subsequent published UI/ledger changes require their
own actual integrated gates before publication; these checkpoints retain their
exact execution identities.

## Preserved failed attempts

The initial full-gate wrapper assumed a tracked `dist/.gitkeep` even though
none existed and refused before any repository command stage. The corrected
wrapper queries the committed inventory before optional-artifact restoration.

The next attempt completed Web install/build/unit commands but all90 registered
browser cases failed before DOM assertions: its deeply nested TMPDIR made
Chromium's `SingletonSocket` path too long. Subsequent Rust/doc stages were not
started. A short task-owned directory inside the authorized scratch root and
one real named browser probe resolved that setup issue before full reruns.
Both failed attempts, logs and browser traces remain separate from later green
runs; they were not reclassified as successful or product assertion failures.
Delayed historical Scheme23 survivors and verifier refusals are likewise retained
in their own producer-bound evidence and are not new AR06V controls.

## Remaining boundaries

This dossier does not verify compressed/member/expanded byte boundaries,
CLI/HTTP source-admission contracts, hostile-byte memory/time budgets or the
synchronous product-database mutex's caller latency/progress/cancellation policy.
It neither raises limits nor establishes streaming, ETS parity, complete
manufacturer semantics or live-bus behavior. Scheme10 remains an explicit
refusal pending grammar evidence. KL-151's resource/caller decision and complete
Alpha acceptance remain separate.

Public receipts/logs and the tested CLI are archived before task-owned scratch
cleanup at a verified publication boundary; foreign root work and locks stay
untouched. The AR14D status-docs lock was respected until its published release at18:45;
subsequent evidence goes into the single ledger without changing the unresolved
resource decision.
