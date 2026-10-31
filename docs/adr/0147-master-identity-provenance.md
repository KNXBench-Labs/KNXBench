# ADR 0147: Master declarations have provenance; historical winners are not reconstructed

Date: 2026-10-19
Status: Accepted

## Evidence and boundary

`parse/master.rs` dispatches by local element name at **any depth**, not by
canonical ancestry. `DatapointType` sets the current main number (invalid or
missing Number becomes zero) even without an Id; a self-closing type clears
that context. Closing any nested type clears it rather than restoring the
outer context. `FunctionType` similarly carries its Id, including the empty
string; missing Id clears it. A rejected duplicate parent still supplies its
context to later children. Function points and DPT subtypes without their
respective carried context are dropped. This is the existing behavior, not a
new master-schema interpretation. All four normalized tables use global Id
and `INSERT OR IGNORE`; mains and subtypes share `datapoint_type`'s namespace.
Manufacturer names and translations have different existing policies and are
not identity families in this decision.

Official SQLite documentation, checked 2026-10-19:

- <https://www.sqlite.org/stricttables.html>, section 2: STRICT primary keys
  imply NOT NULL. A missing Id binds NULL, not the empty string.
- <https://www.sqlite.org/lang_conflict.html>: IGNORE skips uniqueness/NOT NULL
  violations; its zero-row result alone does not prove an identity collision.
- <https://www.sqlite.org/lang_savepoint.html>, section 2: releasing the outer
  savepoint commits; nested release does not commit the caller's transaction.
  Failure of final release must roll back the owned savepoint as well.

This extends [ADR-0043](0043-package-identity-is-recorded-per-candidate.md)
without expanding its six product kinds, scanner, CLI or UI, and follows
[ADR-0020](0020-migrations-may-rederive-from-stored-bytes.md): bytes determine
current declarations, not historical install order or writes.

## Schema and units (product database 26 → 27)

A separate `master_provenance` module owns three STRICT tables:

- `master_source`: source SHA-256, definition version 1, history availability
  (`observed` or `historical-unavailable`), inspection status (`complete`,
  `failed`, `unexamined`) and explicit failure reason. Complete with zero
  declarations is measured zero, never unavailable. Missing source ownership
  may be recorded here even when the `source_file` row is absent.
- `master_declaration`: `(source_sha256, ordinal)` primary key. One row per
  tracked start/empty event the **master parser itself** encounters. Ordinal
  starts at 1 across all four families (including both DPT forms), in event
  order, including malformed declarations. Store family, element local name,
  nullable Id, actual local-name XML path, start-tag half-open byte range
  `[byte_start, byte_end)` in the retained bytes, carried main number or
  function-type Id, missing-Id and orphan-parent flags, and nullable observed
  insert result. Empty Id is a valid existing normalized identity and stays
  distinct from absent Id. Byte ranges identify the exact declaration tag;
  they are not subtree digests or XML-schema validity claims.
- `master_winner`: `(family, logical_id)` primary key. A fresh successful
  normalized insert records its exact source/ordinal. Existing normalized rows
  are seeded with NULL source/ordinal: `historical-unavailable`. Never compare
  current values to candidates or sort blobs to guess who installed first.

Family names are `datapoint_type`, `function_type`, `function_point`, and
`space_usage`. A source's structural malformed flags are byte-determined.
An observed write is measured only at the normalized insert branch. An
eligible zero-row insert must resolve an existing normalized identity before
being called a collision; unexplained suppression fails atomically. Neither
missing Id nor orphan parent is a collision. A declaration can have both
malformed flags. A same-file duplicate and a cross-file duplicate remain
separate source/ordinal addresses even when their values happen to agree.

## Reports and API

Public Rust queries expose source status and declarations (with retained-byte
addresses), current normalized winner availability, and all source-resolvable
candidates for a family/Id. Queries authenticate retained source bytes and
revalidate complete declaration inventories through the same parser, once per
distinct source rather than once per duplicate. Wrong positions, contexts,
ordinals, closed vocabularies and winner pointers fail closed. Source reports
separate authentic retained-byte availability from the inspection status; a
failed inspection can retain authentic but malformed XML. Missing/hash-invalid
sources and their known winner addresses remain explicit, with unavailable
retained-byte flags, never presented as verified source-resolvable evidence.

Observed results are immutable facts from the first dedicated semantic parse
actually observed by v27, not recomputed writes on retry and **not** claims
about the source's original installation. A declaration-only historical replay
leaves them NULL; a later real dedicated ingest may fill those absent facts
and mark source history `observed`. That transition records the newly observed
parse, never reconstructs an old encounter or rewrites an install snapshot. Current candidate membership is not a
historical encounter count. A candidate from an unobserved historical source
has unavailable install outcome even if a same-source duplicate is provable.

`MasterIngest` returns its source hash and per-declaration malformed/collision
diagnostics. The package's existing diagnostics channel uses exactly
`master-identity-collision` and `malformed-master-declaration`, one occurrence
per affected declaration. Detail identifies family, source/ordinal, byte range,
malformed flags, and, where known, the kept source/ordinal. Historical winner
unavailability is explicit. Existing DPT read/stored/dropped and legacy
zero-write counter semantics are preserved; this does not add install count
categories for the other families. Missing Id plus missing parent is one
malformed declaration diagnostic, with both reasons, not two declarations.

These are additive v27 diagnostic vocabulary entries in the existing report
format, not evidence that an older report measured master provenance. Historical
package snapshots/counters and pre-ledger unavailable facts are never rewritten.
A new package reusing retained master bytes still obtains its own diagnostics
from the actual current parse/write decisions. An exact package retry reloads
its original immutable report. No HTTP route or master-identity UI is required.

## Ingest and replay

Dedicated public `ingest_master_data` retains/verifies exact source bytes and
performs normalized writes, provenance and report evidence inside one owned
savepoint, including rollback on final release failure. This covers direct
project/CLI callers as well as package installs. Generic `ingest_file` remains
opaque for MasterData. The old v9 → v10 migration uses a private legacy entry
point of the same parser that requires no v27 tables.

Migration v26 → v27 creates the tables unconditionally, seeds unavailable
normalized winners **only at schema creation**, and invokes the public atomic
replay. Ordinary v27 replay first validates that every normalized identity has
an explicit, well-paired winner marker; a missing or half-NULL marker aborts
without seeding a replacement or turning lost evidence into historical
unavailability. Existing paired-NULL markers remain legitimate history.
Replay calls the
same parser in declaration-only mode: no normalized writes, manufacturer-name
updates, translations, unknown-evidence retirement or historical report edits.
Sources are examined deterministically by SHA-256, one bounded payload at a
time. Check actual length before materializing, declared length and SHA-256
before parsing, complete XML and parser resource limits before accepting rows.
Known master ownership (package members, previous provenance) names missing or
unreadable sources. Unclassifiable oversized/damaged sources receive explicit
unexamined markers; opaque baggage is never semantically parsed. Classification
is inspection, not semantic import.

Replay reconciles the complete byte-derived inventory at stable source/ordinal
keys. It repairs ranges, paths and carried context in place, inserts missing
unobserved declarations and removes surplus unobserved, unreferenced rows.
Observed identity/eligibility changes, lost eligible outcomes in an observed
source, damaged/missing observed keys and surplus winner-referenced keys are
ambiguous: replay refuses the entire operation atomically instead of guessing
history or rebinding a winner. Invalid definition/history headers also refuse.
Trustworthy first-observed outcomes, winner addresses and immutable package
snapshots remain unchanged. The result is idempotent and never publishes a
complete inventory that its own source reader cannot admit.
A bad source is a per-source inspection failure; evidence-budget exhaustion,
ambiguous reconciliation and SQLite/write failures abort all owned changes,
including failures at outer RELEASE. Nested operations never
commit the caller's outer transaction. No historical order is inferred from
rowid, filenames, hash order or current normalized values.

## Bounded evidence output

`MAX_MASTER_EVIDENCE_BYTES` is **16 × 1024 × 1024 bytes**, a conservative fixed
policy, not a corpus- or machine-tuned allowance. It bounds one source operation
(ingest or a replay source); report and identity reads aggregate all of their
referenced inventories under one allowance. A small shared monotonic meter
starts with 1,024 bytes of header headroom. Each declaration charges 256 bytes
of record overhead plus its UTF-8 source-hash, family, element, Id, absolute
path and carried function-type-Id lengths. Path length and every copied field
are admitted **before** their output allocations. Dedicated ingest and replay
reserve a second inventory's charge before accepting complete evidence, so the
subsequent stored-versus-authoritative comparison remains admissible.

Before formatting each emitted diagnostic, the same meter reserves two copies
of `256 + path_bytes + 32 + 10 * id_bytes + 512`: record overhead, ordinal
suffix, conservative debug-escaped Id and fixed source/hash/range/winner text.
The second copy covers persisted reading/validation. This intentionally bounds
repeated IDs, contexts and source hashes as well as repeated long ancestry;
input size, depth and declaration count alone cannot bound that amplification.

Persisted readers charge stored text lengths before decoding/copying strings,
then charge the authoritative parser's inventory against the same meter. A
package-report read preflights the stored master-diagnostic byte total before
materializing its rows; report validation shares a meter across those strings,
each distinct cached source inventory (both losing and kept sources) and
diagnostic-prefix construction; the cache is shared for the complete report.
Identity queries likewise share one meter across their distinct source reads.
Source absence or invalid length/hash keeps retained-byte availability false,
without rewriting historical snapshots or pretending the metadata was verified.
An exhausted read returns no partial report; an exhausted ingest or replay rolls
back its owned savepoint. No declarations or diagnostics are truncated.

## Acceptance and exclusions

Independent literal fixtures cover all four families and both DPT forms,
same/cross-file collisions, empty versus absent Id, orphan/both malformed
flags, unusual parser contexts, exact positions, retained direct ingest,
retry/reopen, migration preserving complete normalized/report values,
historical-unavailable versus measured zero, bad/missing/hash-invalid sources,
repair/replay determinism, opaque preservation, late SQL and outer-release
rollback. Real failing and passing focused tests and owning-crate Clippy logs
are retained outside Git.

No new winner policy, master schema/ETS compatibility claim, data download,
live bus operation, or WP-02 historical column-promotion repair is included.

## Amendment: UTF-8 BOM byte ranges (product database 30 → 31, 31 October 2026)

quick-xml strips a leading UTF-8 byte-order mark but does not count it in
`buffer_position`. Master sources with a BOM therefore got every
`[byte_start, byte_end)` three bytes too early. That contradicted "in the
retained bytes". 92 of 142 `knx_master.xml` files in the private corpus start
with a BOM. The parser now adds the stripped prefix back.

The data-only v31 step changes two things. It moves `byte_start`/`byte_end`.
It also rewrites the collision and malformed-declaration install diagnostics
whose text quotes exactly the old range. Writer, reader and migration share one
text builder. The step touches authenticated BOM sources only, and only when the
corrected parser reproduces every stored declaration with a range exactly three
bytes later. Everything else stays untouched, so the authoritative reader keeps
reporting it rather than the migration guessing a repair. That covers damaged,
diverging, incomplete, unreadable, unparsable or already-correct sources.

- **Why the step is needed.** Without it, that reader would refuse the shifted
  rows, because it re-parses the retained bytes.
- **Why it also runs early.** The v27 → v28 and v28 → v29 steps reload install
  reports through that reader. The same idempotent step therefore runs first
  when a v27–v29 catalogue is opened. Otherwise those steps would downgrade
  every report of a BOM master.
- **Rejected alternative.** A full provenance replay was rejected: it also
  writes status rows for unrelated damaged sources.
- **Older builds.** They refuse product31 by their version check.
