# Declared product-ZIP size admission contracts

## Evidence and scope (AR06W)

Read the existing `install_package` implementation before designing fixtures:
`crates/knx-productdb/src/package.rs` first checks compressed input 256 MiB,
then structural/parser metadata binding, then all advertised member sizes 64 MiB
and their total 256 MiB, and only afterwards decodes payloads. These constants
are unchanged. A declared-size fixture is malformed payload metadata, not an
actual successfully installed 64/256 MiB payload and not a peak-memory benchmark.
Source-ID status belongs solely in [status/LEDGER](status/LEDGER.md).

The fresh standalone baseline on committed `44746183` passed 40, failed 0,
ignored 3; 121
native dependency hashes, original log and freshly compiled executable checked.
It does not count as new byte test coverage or private-corpus evidence.

## Format reference verified before implementation

Primary source: PKWARE APPNOTE 6.3.10, FINAL, revised November 1, 2022, consulted
October 4, 2026: <https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT>.
Sections 4.3.7, 4.3.12 and 4.4.1 describe the local/central fields and little-endian
numeric representation. Do not reproduce the specification; this is a reference
and a description of our fixture design. For an ordinary non-ZIP64, no-descriptor
entry, changing only its advertised uncompressed length requires updating both
local and central lengths consistently. Locate headers via the actual ZIP
reader's named-member offsets, validate signatures, flags and method, then
change the two fields. Do not search for arbitrary signature byte strings or
assume a footer position. Confirm declarations through a fresh reader afterward.

## Verified bounded contracts (native and semantic controls)

- Member declared exactly 64 MiB proceeds past size admission and later refuses
  at its named actual decoded-length mismatch.
- Declared 64 MiB + 1 byte refuses with the member's typed `SizeLimit`.
- Aggregate declared exactly 256 MiB (including both XML members) proceeds to
  the first named baggage decoded-length mismatch.
- Aggregate 256 MiB + 1 byte, with every member individually at/below 64 MiB, refuses
  with typed `SizeLimit` at the named member which exceeds the total.

Each starts by installing a real two-XML seed, proving its retained archive,
then snapshots every column/value of all non-system tables including BLOBs.
Both resource refusals and later malformed-payload refusals must preserve this
entire state and retained seed byte-exact. Use the already delivered test
snapshot helpers instead of duplicating a second SQL snapshot implementation.

Fixture budget: at most six entries, each actual baggage body one byte;
compressed archive under 8 KiB; no fixture allocation based on a declared length. Aggregate
arithmetic is checked and verified against the actual reader. Deflate is selected
explicitly from the installed workspace ZIP dependency. No ZIP64, encryption or
data-descriptor semantics are added. Payloads are deliberately malformed: the
assertion must identify the later failure precisely, not treat any error as an
inclusive-boundary pass.

## Execution evidence and remaining work

The four native declared-size contracts passed 4/0/0; two old count cases were
filtered, not reexecuted. 121 current input hashes, original log and newly
compiled executable were independently verified. Separate standalone baseline
40/0/3 is not added to these totals. All four isolated lower/upper member/total
controls now independently accepted: fresh compile 0, selected exact named
behavior 101, expected semantic assertion, original logs, binary and source
hashes all verified. These are expected control REDs, not product regressions.
Canonical production source unchanged. The scoped public package/docs sweep
also completed: all nine commands exit 0; ProductDB 644 passed, 0 failed,
25 ignored in 31 result blocks. These totals already include the four new
declared-size cases and the two existing count cases; do not add earlier
standalone/native totals or expected control failures. Meaningful policy scope:
474 headed files, 445 anchors across 275 Markdown files, 186 ledger rows and
369 Rust files for corpus policy. This is not full-workspace/UI acceptance.

The initial control sweep stopped before behavior: first compile 101 because
Cargo loads default target entrypoints for every workspace member, including
nondependency crates omitted by a narrow native-only source copy. This is
snapshot infrastructure failure, not a detected mutant. Original receipt/log
retained. Retry snapshots supply 378 actual tracked Rust files (never fake empty
entrypoints); actual `cargo metadata --offline --locked --no-deps` passed before
dispatch. Each 121 native dependency comparison still differs only at the intended
size constant. Separate retry/fresh-target acceptance succeeded for all four;
original failed attempt remains preserved and contributes zero detected controls.

Actual inclusive compressed/expanded payloads, encoded input 256 MiB boundaries,
real CLI/HTTP caller admission, caller latency/progress/cancellation and hostile
resource-budget acceptance remain separate. No production limit raise, streaming
rewrite, ETS/manufacturer parity, hardware behavior or Alpha closure is claimed.

## Publication scope

At the October 5 remote-tip audit (`b3c341d5`), native ProductDB/Core/testsupport
sources were unchanged from the baseline; only this uncommitted test fixture
differs in the 121-input comparison. The native runs therefore identify both
their old base and exact source hashes, not a fabricated committed revision.
The declaration-only leaf `a9aa77b9` was integrated with published owner
`b3c341d5` at `2e188226`. A fresh integrated public16 was independently
accepted on that exact candidate: all 16 commands exit 0; Rust 3200 passed,
0 failed, 177 ignored in 175 result blocks, Web 2001 and Chromium 131, plus
a separate one-case Chromium environment probe (not added). All 871
non-Markdown inputs, original logs, freshly built source-root checks and the
release CLI stamped `2e188226` were checked and archived. Earlier scoped
counts are not additive and are not relabelled as this execution.

The release CLI is `knx 0.1.0-alpha.1+g2e188226`; these public tests do not
resolve another owner’s AR21 performance/manual findings or establish
private-corpus, live-hardware or full Alpha acceptance. Final documentation gates6 passed on `425f3407`; that source/evidence
publication was pushed to main and remote-readback verified, including all
871 source hashes and the four-test fixture. Runtime evidence remains
identified by `2e188226`, not retagged to later metadata. KL-151 IN_PROGRESS.
Own-only hygiene and shutdown metadata follow this publication.

The original receipts, logs, six accepted binaries and intended control-source
deltas were archived and hash-checked in 70 files (345,219,799 bytes) at the
profile-local evidence directory `alpha-release/ar06w-declared-size-20261005`.
This archive preserves the original failed control setup; no private corpus
or hardware execution is introduced. Final integration/publication evidence
will be recorded separately, without relabelling historical runtime results.
