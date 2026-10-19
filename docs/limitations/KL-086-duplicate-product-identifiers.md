# Duplicate product identifiers

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-086)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## WP-17 — source provenance after Alpha 15 (19 October 2026)

Product DB v27 (delivered on `main` as `d18ce65e`, not an Alpha 15 artifact) adds source-addressable declaration evidence for
`datapoint_type` (both mains and subtypes), `function_type`, `function_point`
and `space_usage`. Fresh normalized winners name retained source bytes plus
an ordinal/start-tag byte range. Same-file and cross-file losing declarations
stay individually addressable. Missing Id and orphan parent are explicitly
reported, not disguised as identity collisions; empty Id retains its existing
separate behavior. The parser's current normalized winner/context rules remain
unchanged. [ADR-0147](../adr/0147-master-identity-provenance.md).

Migration/replay derives declarations without rewriting normalized values or
historical install reports. **Remaining boundary:** old winner/encounter/write
history cannot be recovered from retained bytes or inferred from blob order.
Those facts stay explicitly unavailable; failed/unexamined source inspections
are not clean or empty evidence. No version-pinning policy or unsupported
identifier family is implemented by this provenance change.

<!-- BEGIN retained catalogue record -->

## 86. Duplicate identifiers inside one file — recorded for normalized product identifiers; DPT provenance remains limited

**Original limitation (as filed).** `first_winner` — one copy, in
`crates/knx-productdb/src/parse/mod.rs`, called by `parse/hardware.rs` and
`parse/catalog.rs` since the two byte-identical copies were merged on
2026-09-13, plus the inline equivalent for `application_program` in
`crates/knx-productdb/src/parse/program.rs` — recorded an `IdConflict` only
when an id it had already seen belonged to a *different* file — its only
test was `kept != source_sha256`, comparing the existing row's stored
`source_sha256` against the `source_sha256` the current parse call was
handed. Because one `ingest_hardware`/`ingest_catalog` call always passes
the same `source_sha256` for every element in that file, two `Hardware` (or
`Product`, `Hardware2Program`, `CatalogSection`, `CatalogItem`,
`ApplicationProgram`) elements sharing an `@Id` **inside the same file**
always compared equal and never reached the `IdConflict` branch: the second
element was dropped, first-writer-wins, with nothing recorded anywhere.

**Initially fixed for `first_winner`'s two callers.** `first_winner`
(`crates/knx-productdb/src/parse/mod.rs`) now takes an extra
`seen_this_call: &mut HashMap<(String, String), u32>` parameter, freshly
created once per `ingest_hardware`/`ingest_catalog` call and threaded
through every `first_winner` invocation made while parsing that one file.
Each call increments the count for `(table, id)` and reports it back as
`occurrence`. A conflict is now recorded when *either* `kept !=
source_sha256` (the original cross-file check, unchanged) *or* `occurrence
> 1` (new: this exact id has already been seen earlier in this same parse
call). `source_sha256` itself, and its meaning as file provenance
elsewhere (idempotent re-parse detection, translation backfill), is
untouched — this is additive, not a reinterpretation of an existing column.

The new key costs one field, not a schema rewrite: `IdConflict`
(`crates/knx-productdb/src/report.rs`) gained `pub occurrence: u32` (`1`
means "first sighting this call, so any conflict is the old cross-file
kind"; `>1` means "the Nth same-file sighting"). It is persisted by
reusing the existing `ingest_unknown.occurrences` column (previously
always written as the literal `1` for `IdConflict` rows) and, for
installed-package reports, a new `package_conflict.occurrence` column
(schema v6 → v7, `migrate_v6_to_v7`, `DEFAULT 1` for rows written before
this change). First-writer-wins behaviour is unchanged: the second element
in a same-file collision is still not stored as a row, but the fact that
it existed and lost is now visible in the report, satisfying CLAUDE.md's
"never silently discard information" for this path.
Was pinned, now proven fixed **[V]**, by
`two_hardware_elements_sharing_an_id_in_one_file_record_the_collision`
(`parse/hardware.rs`) and
`two_catalog_items_sharing_an_id_in_one_file_record_the_collision`
(`parse/catalog.rs`) — both are the exact same synthetic same-file-duplicate
input as their now-retired `..._conflict_silently` predecessors, with the
assertion flipped from "conflicts is empty" to "one conflict, occurrence
2"; run against the pre-fix code both would fail (and did, verbatim,
before this fix, since they are literally the old pinning tests renamed
and re-asserted).

**Closed for `application_program`, 2026-09-17.**
`ingest_program` now uses the same `first_winner` helper and a fresh
`seen_this_call` map for each source file. Two `ApplicationProgram` elements
with the same `@Id` in one file retain the first declaration and record an
`IdConflict` with `occurrence = 2`; the existing cross-file behavior remains
unchanged. The regression test
`two_application_programs_sharing_an_id_in_one_file_record_the_collision`
failed against the old inline logic and passes with the shared helper.

**Residue: `datapoint_type` still has no declaration provenance**, even
though collisions are counted: `knx_master.xml`'s `DatapointType`/
`DatapointSubtype` elements are written with a bare `INSERT OR IGNORE`
(`crates/knx-productdb/src/parse/master.rs`) into `datapoint_type`, whose
primary key is `id` alone with no `source_sha256` column to compare
against in the first place.

**AR05 audit, 2026-10-02 (candidate, offline).** New package regressions verify
same-file main/subtype collisions: four semantic declarations read, two stored,
two dropped, first normalized values unchanged, full master bytes retained and
historical facts stable after reopen/retry. An orphan subtype is read/dropped
but does not increment the legacy collision counter. These distinguish semantic
loss from retained bytes; they do not invent winner provenance or lift this
residue. Existing normalized product identity reporting is unchanged. Contract:
[MANUFACTURER_REPORT_CONTRACT](../contracts/MANUFACTURER_REPORT_CONTRACT.md).

**Measured against the real corpus.** Every `.knxprod` file under
`OriginalData/ProductDatabases/` was copied to a scratch directory outside
the repository (never modified in place) and installed with
`knx_productdb::install_package` into one shared database, in the order
`ls` returns them, via a throwaway test gated on `KNXBENCH_PRODUCT_CORPUS`
— command: `KNXBENCH_PRODUCT_CORPUS=<scratch dir> cargo test -p
knx-productdb --test tmp_collision_probe -- --nocapture`, deleted after
this measurement, not part of this commit **[V]**.

- `first_winner`-tracked tables (`hardware`, `product`, `hardware2program`,
  `catalog_item`, `application_program`): **0** cross-file `IdConflict`s
  across the corpus's 4 distinct packages. The corpus is small — one real
  vendor package, one test-fixture package and two near-duplicate
  fixtures — and no two files declare overlapping manufacturer/hardware
  ids, so this measures "never observed here", not "cannot happen"; the
  same-file case (now closed for `hardware`/`catalog_item`/etc., see
  above) is demonstrated by a synthetic test instead because no real file
  in this corpus happens to contain one either.
- `datapoint_type` (the untracked path): **routine, not rare.** Every
  package's `knx_master.xml` restates the *entire* KNX-standard DPT
  catalogue rather than only the DPTs its own products use. Installing the
  4 distinct packages in sequence: package 1 declares 234 `DatapointType`/
  `DatapointSubtype` elements and the table grows by 234 (nothing to
  collide with yet); package 2 declares 354 and the table grows by only
  120 (234 silently dropped); package 3 declares 383 and the table grows
  by 29 (354 dropped); package 4 declares 234 and the table grows by 0
  (all 234 dropped). Total: **822 silent, uncounted drops across 4
  packages**, every one of them after the first hitting the collision on
  effectively its whole DPT declaration.

**Fixed, in a small and contained way.** The `datapoint_type` path had no
counter at all, so one was added: `MasterIngest::dropped_datapoint_types`
counts every `INSERT OR IGNORE` that changed zero rows, `InstallReport`
carries the sum as `dropped_datapoint_types` (persisted in a new
`package.dropped_datapoint_type_count` column, schema v6,
`migrate_v5_to_v6`), and `knx products ingest` in `apps/knx-cli` prints it
alongside the existing conflict count. This is a
count of drops, not a full `IdConflict` — `datapoint_type` still has no
`source_sha256` to build one from, so it cannot say *which* file's id won,
only that one lost.

**Remaining limitation — `datapoint_type` provenance.** The current counter
shows how many declarations collided, but the `datapoint_type` table has no
`source_sha256` column. Reports therefore cannot identify which file supplied
the retained declaration. This needs a deliberate schema migration and remains
separate from the now-complete same-file detection for every `first_winner`
caller.

**Residue grows, 2026-09-14 (T13, branch `d10-language-data`).**
`function_type`, `function_point` and `space_usage` (new tables, schema
v9 → v10, closing §64's own residue) are filled by `parse/master.rs` the
same bare `INSERT OR IGNORE` way `datapoint_type` already was — no
`source_sha256` column, no occurrence counter, same restated-whole-catalogue
collision shape a second `knx_master.xml` produces. Not measured
separately against the corpus the way `datapoint_type` was above; the
shape of the gap is identical, so it is stated rather than re-argued. A
future fix for `datapoint_type`'s residue should cover these three tables
in the same pass rather than leaving them a second time. The same three
element families also inherit `datapoint_type`'s other silent-discard
shape: a `FunctionType`, `FunctionPoint` or `SpaceUsage` with no `@Id`
attribute at all binds `NULL` into a `TEXT PRIMARY KEY` column, and
`INSERT OR IGNORE` drops that row with no error, no counter and no
`ingest_unknown` entry — exactly as an id-less `DatapointType` already
did before this slice, and not a new gap this slice introduces, only one
it extends to three more tables. An id-less `FunctionType` compounds the
loss: it orphans every `FunctionPoint` nested inside it too, and those
are then silently dropped a second time by the parentless-`FunctionPoint`
guard at `parse/master.rs:213` (T13 fix round 2).

---
<!-- END retained catalogue record -->
