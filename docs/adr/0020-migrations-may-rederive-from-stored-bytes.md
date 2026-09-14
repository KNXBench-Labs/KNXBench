# ADR 0020: A product-database migration may re-derive what the stored bytes determine, and must not invent what only the install knew

Date: 2026-09-14
Status: Accepted
Session: 7 (T9 of the goal-completion run)

## Context

[KNOWN_LIMITATIONS.md §87](../KNOWN_LIMITATIONS.md) records that
`application_program.linkable` stays `NULL` forever in any product database
built before 2026-09-13. `bool_flag` (`crates/knx-productdb/src/parse/mod.rs`)
accepted only `"1"` and `"0"` until that date, so every word-spelled
`Linkable` was read as "absent". The helper now takes all four canonical
`xs:boolean` spellings, which fixes every future ingest and no past one, and
two short-circuits keep the old `NULL` in place: `install_package`
(`package.rs:373-405`) returns `skipped: true` on a package whose sha256 is
already on record without re-reading a byte, and `migrate_v5_to_v6`
(`migration.rs:293`) added its column without re-deriving anything.

§87 asked for a v7 migration that re-parses, and named the reason it had not
been written: *"it would be the first migration in the chain to call the
parser, which is an architectural commitment"*. **That premise is false, and
checking it is what this ADR is for.**

### E1 — two migrations already call the parse layer, and have since 2026-09-11

| Migration | Parse-layer call | What it re-derives |
| --- | --- | --- |
| `migrate_v2_to_v3` → `backfill_dynamic_nodes` (`migration.rs:531`) | `dynamic::parse::parse_dynamic_trees` | every `dynamic_node` row, from every blob that `classify`s as `ApplicationProgram` |
| `migrate_v3_to_v4` → `backfill_shared_translations` (`migration.rs:410`) | `ingest::classify` + `parse::translation::ingest_translations` | `Catalog`/`Hardware`/`Master`-scoped `translation` rows, from every blob of those kinds |

Both read every `source_file` blob, filter it by `classify`, give each blob
its own `SAVEPOINT`, and record a failure as an `ingest_unknown` row through
the shared `record_backfill_failure` — whose own doc comment says "Shared by
every backfill in this file", in the plural, because there are two. The
commitment §87 was reluctant to make was made twice already, by
`migrate_v2_to_v3`, whose doc comment states the design intent outright:

> This is the first migration in this crate that runs Rust rather than plain
> SQL — see `backfill_dynamic_nodes` below, which is the payoff ADR-0011's
> blob store was designed for: a file's bytes are kept specifically so a
> later parser can read what an earlier one skipped, without asking the user
> to feed the file in again.

What was missing was not the decision. It was the *record* of the decision
and its boundary, which is why §87 could restate the opposite convention —
"a migration adds structure, never re-parses" — without contradicting
anything written down.

### E2 — why `migrate_v4_to_v5` and `migrate_v5_to_v6` were nonetheless right to default to `0`

Those two add per-package counters — four `translation_*_count` columns and
`dropped_datapoint_type_count` — and default them to `0` for packages
installed before the columns existed, on the stated grounds that re-deriving
them "would mean re-parsing bytes this migration has no access to". The
bytes are in fact right there in `source_file`; the real reason is better
than the one given, and it is the line this ADR draws.

Those counters are **not** functions of the file. They count what one
`INSERT OR IGNORE` actually changed at one moment in one database's history
— `TranslationCounts`' own doc comment says so: "a row already present under
the same key is ignored by SQLite and contributes nothing here, even though
it was seen". Replaying the bytes today would count what is *missing today*,
not what that install wrote. The answer depends on install order and on what
other packages had already claimed. It is unrecoverable, and `0` with the gap
named is the honest value.

`linkable` is the opposite kind of value: one attribute of one element of one
file, a pure function of bytes the database already holds, with no dependence
on when or in what order the file arrived.

### E3 — the corpus, which also shows §87's own claim is too narrow

Every `Linkable` in the repository's corpus, by producing tool rather than by
schema version (`grep -o 'Linkable="[^"]*"'` over the unpacked archives):

| Source | Namespace | `Linkable` spellings |
| --- | --- | --- |
| `Unser Zuhause ets4 - 2025-12-15.knxproj` (ETS4) | `project/11` | 9× `"0"`, 3× `"1"` |
| `KV v2.5 - demo.knxproj` (ETS6 6.0.5030.0) | `project/21` | 4× `"false"` |
| `Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj` (ETS6 6.3.7959.0) | `project/23` | 9× `"0"`, 3× `"1"` |
| `646704-04_ETS4_2012_47_DE_EN.knxprod` | `project/11` | 1× `"true"` |
| `Weinzierl_730_KNX_IP_Interface_ETS4.knxprod` (and `_v1`) | `project/11` | 1× `"false"` each |
| `Dummy_Applikation_Secure.knxprod` | `project/20` | 1× `"false"` |
| `MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod` | `project/20` | 3× `"false"` |

All 35 occurrences sit on `<ApplicationProgram>` and nowhere else. Namespace
`project/11` appears on both sides of the table, so the spelling is a
property of the tool that wrote the file, not of the schema version — §87's
"schema-20 or schema-21 package" understates the blast radius, and a
schema-11 `.knxprod` from 2012 is affected too. A fresh ingest of all eight
files yields 34 programs, 0 with `linkable IS NULL` (27 false, 7 true), 75
`source_file` blobs and 4 `package` rows at `user_version` 6 — four, not
five, because the two Weinzierl archives are byte-identical
(`15133a14784d…`), which is the content-hash identity of
[ADR-0011](0011-product-database-storage.md) doing its job.

### E4 — nothing reads the column yet, and nothing was silently discarded

`grep -rn linkable` over the whole workspace returns the schema, the write in
`parse/program.rs`, its own regression test, and prose. There is no reader in
`query.rs`, in `apps/knx-server`, or in the web app. Separately, the old
`bool_flag` did not swallow the value it failed to understand: its
`Some(other)` arm reported it, so a pre-fix database carries
`ingest_unknown` rows with `kind = 'Attribute'`, `name = 'Linkable'` and
`sample = 'true'` or `'false'`. The information was preserved and reported,
exactly as CLAUDE.md requires; it was the derived column that went wrong.

### E5 — what "just rebuild it" actually costs

Measured on this machine, debug build, `knx products ingest` over all five
`.knxprod` packages and all three `.knxproj` projects into an empty file:
**17.0 s wall, 128 MB**. Cheap — for eight files. The number that matters is
not this one, though: a rebuild needs the *original files*, and the blob
store exists precisely so that a later parser need not ask for them again
(E1's quotation). A user who ingested a manufacturer's `.knxprod` eight
months ago and deleted it still has its bytes in the database and no way to
feed them back in.

## Decision

**A product-database migration may re-derive a value that is a pure function
of bytes already in `source_file`, by calling the current parse layer. It
must not manufacture a value whose true answer depended on the install
event.** That is the line: bytes, yes; history, no. `migrate_v2_to_v3` and
`migrate_v3_to_v4` are on the permitted side and stay; `migrate_v4_to_v5`'s
and `migrate_v5_to_v6`'s counters are on the forbidden side and keep their
honest `0`.

A backfill of the permitted kind carries five obligations. Rules 1, 4 and 5
are what the two existing backfills already do, promoted from habit to
requirement; rules 2 and 3 are new, because those two backfills only ever
*insert* rows into tables that were empty, and this one is the first to
`UPDATE` a column that another writer may have set:

1. **It calls the parse layer, not a copy of it.** A frozen private
   re-implementation inside `migration.rs` would trade a visible coupling for
   an invisible one and duplicate logic CLAUDE.md forbids duplicating.
2. **It writes into absent slots only.** The `UPDATE` is guarded by
   `IS NULL`, so a value an ingest positively determined is never overwritten
   — [ADR-0012](0012-enrichment-into-absent-slots.md)'s rule, applied to a
   migration.
3. **It is scoped by `source_sha256`.** A blob that lost an id conflict
   ([ADR-0011](0011-product-database-storage.md)) must not write over the
   winning row, so the `UPDATE` matches the blob the row came from.
4. **One `SAVEPOINT` per blob, failures into `ingest_unknown`, never a
   refused open.** A database that will not open is worse than one with a
   gap.
5. **`user_version` is the record that it ran.** A backfill happens once per
   database, on the open that crosses its version boundary.

**Concretely, product-database schema v7 is that backfill for `linkable`, and
adds no column.** `migrate_v6_to_v7` re-reads only the blobs that actually
have an affected row — `SELECT DISTINCT source_sha256 FROM
application_program WHERE linkable IS NULL` — reads
`ApplicationProgram/@Linkable` through the same `bool_flag` the ingest path
uses, and fills the column. Where it fills one, it also deletes the now-false
`ingest_unknown` row that said the attribute was not understood; that row was
a report about a parse gap that no longer exists, and the bytes behind it are
untouched in `source_file`.

**This fixes one generation of staleness, not the class.** If the parse layer
ever changes its answer for bytes it has already read, a database that is
already at v7 will keep the older answer and no migration will re-run.
§87 therefore stays open, rewritten to describe that class instead of the
`linkable` instance it used to describe.

## Alternatives considered

**Leave `linkable` NULL and document it.** The status quo, and cheap: nothing
reads the column (E4), so nobody is getting a wrong answer today. Rejected
because "no reader yet" is the weakest reason there is to keep a column
wrong. The first reader inherits a silent falsehood — `linkable IS NULL`
means "not stated in the file" everywhere else — and the value is recoverable
from bytes we already hold, which is the whole point of holding them.

**Re-derive on read, or lazily on first use.** Rejected on three counts.
There is no reader to hook (E4), so it would mean writing the consumer first,
inside a task about a migration. A read that writes needs a write transaction
on a database a caller may have opened to query, and the product database is
shared across projects ([ADR-0005](0005-separate-product-database.md)). And
it makes the same query's cost and the same database's contents depend on who
asked first, which is the determinism CLAUDE.md asks for, lost — the same
objection [ADR-0011](0011-product-database-storage.md) already used to reject
"blob only, no parsed tables" for making lookups re-parse.

**Force a package reinstall: drop `install_package`'s sha256 short-circuit,
or add a `--force` to it.** Rejected on data integrity. Ingest is
first-writer-wins with the loser recorded in `package_conflict`, so a
package's rows are not cleanly its own: deleting them to re-insert them can
leave a hole that a *different* package would have filled, and filling it
back requires re-ingesting that one too. The short-circuit is also not the
only one — `ingest.rs`'s `source_parse_evidence` content-hash skip and
`program.rs`'s `already_present` flag would each have to be defeated as well,
which is three loosened safety properties to reach a place the bytes in
`source_file` already reach with none.

**Rebuild the database from the original packages.** Kept as the fallback,
rejected as the answer. It needs files the user is not obliged to still have
(E5), and the blob store was designed so that they would not be (E1). Its
measured 17.0 s for this eight-file corpus says nothing about a full
manufacturer catalogue, which was not measured.

**Freeze a private copy of the `Linkable` rule inside `migration.rs` so the
migration touches no parse code.** Rejected. It duplicates `bool_flag`, and a
frozen copy drifts silently: the day the parse layer learns a fifth spelling,
the migration keeps the old four and nothing fails. The coupling is real
either way; this version of it is merely harder to see.

**Record a parse-generation number per `source_file` row and report a stale
database instead of repairing it.** This addresses the *class* E1's rule
leaves open, and it is the shape a future answer would take if the class
recurs — a counter bumped whenever parse semantics change, `0` for rows
ingested before the counter existed, and a line in `knx products verify`.
Rejected here for two reasons: it reports a problem it could instead fix,
which is the wrong trade when the fix is available; and its `0` is
false-positive by construction, since "ingested before the marker existed"
includes every database built between 2026-09-13's parse fix and the marker
— databases that are in fact current. Deferred rather than dismissed, and
deliberately not bundled into a task about one column.

## Consequences

**`CURRENT_PRODUCTDB_VERSION` becomes 7, with no DDL in the step.** A
migration that only moves data is a first for this chain; `user_version` is
what makes it happen exactly once, so the version bump is the entire
mechanism and not bookkeeping around one.

**The cost is bounded by the defect, not by the database.** The backfill
selects blobs through `application_program.linkable IS NULL`, so a database
with no affected rows — every database built after 2026-09-13, and every
fresh one — pays one query and reads no blob. Measured on E5's 128 MB
corpus database, debug build, as the wall time of a `knx products list` that
has to open it: **1.14 s** when all 34 programs need filling and all 34 of
their blobs are read, **0.038 s** when the same file is rolled back to v6 with
its values intact, which is to the millisecond the same as opening an
already-migrated one. The worst case is paid once, by the databases that have
the defect.

**A frozen fixture is now owed, and paid.** `migration.rs`'s tests build a
v6 database with a `NULL` `linkable` and a stale `ingest_unknown` row, open
it, and prove the value appears and the stale row is gone; a corpus test does
the same with the real `.knxprod` bytes. A test that only exercised the new
code path on fresh input would prove nothing about the databases this exists
for.

**Two existing migrations gain the justification they were running without.**
`backfill_dynamic_nodes` and `backfill_shared_translations` are no longer
undocumented precedents that a future reviewer might read as accidents. A
reviewer now has a name for the question "may this migration parse?" and a
two-word test for it: bytes, or history.

**The class of defect stays open and stays named.** KNOWN_LIMITATIONS.md §87
survives as "a parse fix does not reach rows already ingested", with
`linkable` recorded as the one instance that was repaired and how. The
parse-generation marker above is the thing to reach for if it happens twice
more.

**No compatibility claim changes.** This is entirely internal to
`knx-productdb`: the same bytes, the same exports, no ETS behaviour verified
or claimed either way.
