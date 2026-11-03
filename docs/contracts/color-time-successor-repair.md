# Source-authenticated Color/Time successor repair

Status: **implementation candidate, not independently accepted** (3 November 2026).
ProductDB v32 is a data-only successor to the unchanged released v15 promotion.
The [authentic historical witness](../status/2026-11-03-wp08-authentic-color-time-upgrade.md)
remains the independent original-producer reproduction; authored known-state
regressions and scratch replay are not historical producer executions.

## Repair boundary

The public migrating opener can correct existing non-NULL `Color`/`Time`
contamination when the retained source's first-authoritative type is `Other`
or `Date`. It writes only existing `parameter_type.kind`, `size_in_bit`,
`min_inclusive` and `max_inclusive`, restoring NULLs as well as values.
It never inserts or deletes catalogue rows, changes source bytes/keys,
rewrites program ownership, retires original unknown evidence or recomputes
package counters, conflicts or measured install history. Earlier migrations
still execute normally on older databases; this successor changes none of
their released bodies or the shared Color/Time helper.

Candidate sources come from normalized program owners and retained
`ApplicationProgram` package members. Sources without owned or orphan
Color/Time rows are outside this repair. Legacy payload sources remain outside
this XML proof. An orphan without an attributable retained program source is
not adopted or reconstructed. This is not a general catalogue rebuild.

## Authentication and proof

1. Admit actual retained BLOB length against the existing retained-source
   ceiling before loading it. Require declared/actual length equality and the
   exact SHA-256 before classification or parser materialization.
2. Require `ApplicationProgram` classification, complete XML and borrowed
   lexical depth/item/path/value work admission using the existing evidence
   ceilings. This scan makes no semantic type decisions.
3. Derive normalized types through the authoritative current program parser in
   disposable `promotion_scratch` schemas. Seed foreign program owners before
   parsing; compare owned program IDs/manufacturers and the full owned type key
   inventory. Duplicate type keys that the parser cannot admit are refused,
   not assigned a new winner rule.
4. In an independent scratch parser state, map current first Color/Time/Date
   kinds to the historical `Other`/NULL kind fields. These are the only later
   recognized kinds in this model. Call the **unchanged** released Color/Time
   helper once in that scratch, discarding every scratch evidence write.
5. Permit a difference only from stored Color/Time to authoritative Other/Date
   when the complete stored row equals that exact replayed bad row. Keys,
   `name`, `base`, `number_type` and every unaffected type row must agree with
   authoritative parsing. Already-correct rows are not rewritten.

Two scratch parser states model the known promotion, not lost encounter
history. One source is held at a time; foreign owners and live type rows are
streamed, with no retained cross-source catalogue inventory. Lexical limits
bound replay work, not SQLite heap, elapsed time or process RSS. The source
key cursor settles before a source savepoint can roll back.

## Refusal and compatibility

A source/hash/length/classification/parse/key/ownership/tuple uncertainty rolls
back the entire source savepoint, including earlier individually provable row
updates. Independent valid sources continue. A stable
`PromotionRepairError` / `repair_color_time_kinds` diagnostic is appended with
one occurrence and a nonempty cause; existing evidence IDs/rows stay intact.
The public diagnostic reader narrowly admits that name and validates its
shape. An unmeasured source remains unmeasured: its named refusal is an error,
not a new measured inventory. Storage errors remain fatal to the enclosing
migration transaction; deterministic scratch-parser key collisions are source
refusals rather than live storage failures.

Reopening v32 does not replay the migration or append another diagnostic.
Older builds capped at v31 refuse v32 with `FutureVersion`; there is no
supported downgrade. No DDL or dependency changes are needed. The pinned v28
fresh-chain schema shape remains unchanged, but admission deliberately requires
v32 and witnesses retain the actual version.

## Candidate verification and remaining acceptance

The registered `color_time_successor` module has seven leaf tests, including a
13-variant refusal table and ten measured/unmeasured diagnostic-shape controls.
The original-source semantic RED asserts wrong persisted winner rows before
checking schema version. Coverage includes Other and Date winners, NULL Time
restoration, legitimate and unique Color/Time, mixed own/foreign programs,
full unrelated table/history preservation, native live-write failure and
reopen/idempotence. Owning ProductDB lib/default integration checks are focused
candidate evidence; ignored/private corpus checks are not claimed.

Separate retained artifacts exercise copies of the genuine original v14 and
contaminated v31 databases, a fresh v32 install of the exact retained fictional
package, and every reopen. Typed table multisets, schema descriptors and
unrelated history are compared independently of the four permitted columns.
Original witness inputs are never opened writable or modified.

Parent acceptance still requires independent code/data-integrity review,
mutation calibration, complete selected gates, integration and publication.
Broader [WP-08](../open-work/WP-08-parameter-value-codecs.md) codec, checked date
editor and native/image evidence remain open; this migration establishes none
of those semantics and has no real-bus or hardware effect.
