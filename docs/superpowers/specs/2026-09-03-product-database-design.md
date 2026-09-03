# Product Database Design — Session 4

Date: 2026-09-03
Session: 4 (Manufacturer databases)
Status: approved in brainstorming, pending implementation plan
Input: [RESEARCH.md](../../RESEARCH.md) §4, §5, §10; [ADR-0005](../../adr/0005-separate-product-database.md);
[IMPORT_EXPORT.md](../../IMPORT_EXPORT.md) §10; [KNOWN_LIMITATIONS.md](../../KNOWN_LIMITATIONS.md) §11, §12

This document defines the shared product database and its ingest path. It does
not implement it. Every non-obvious decision cites the finding or the earlier
decision that forced it.

---

## 1. Scope

Build `knx-productdb`: a shared, content-hashed SQLite database holding
manufacturer, hardware, product, application-program and translation data,
ingested from the `<M-xxxx>/*` entries of a `.knxproj`, plus the resolution
pass that fills the `Program` and `ProgramRef` layers of the domain model from
it.

**In scope.**

| Deliverable | Why |
| --- | --- |
| `products.sqlite` schema with its own migration chain | ADR-0005; project and product schemas have different lifetimes |
| Streaming ingest of `Catalog.xml`, `Hardware.xml`, application programs, `Baggages.xml` and the baggage blobs | RESEARCH §4 |
| Minimal `knx_master.xml` ingest: `Manufacturers`, `DatapointTypes` | RESEARCH §5; without it `M-0083` stays a number |
| Content-hash identity, existing entries skipped without re-parsing | ROADMAP Session 4; 22 MB per project otherwise re-parsed |
| Project-side manifest in `knx-store` schema v3 | So a project can name what it referenced even when the database is gone |
| Export path reconstructing `<M-xxxx>/*` container entries from the database | ADR-0007 guarantee 2 must survive the move |
| Enrichment of `ComObjectInstance` from the application program | Closes KNOWN_LIMITATIONS §12 for communication objects |
| Graceful degradation when the database or one program is missing | ADR-0005 |
| `knx products` CLI subcommand; `--product-db` / `--no-product-db` on `knx import` | Inspection, separate ingest, and a tested fallback path |

**Out of scope for this session**, each with its landing place:

| Excluded | Reason / lands in |
| --- | --- |
| Full entity persistence of `knx_core::Project` into SQLite | Independent subsystem; own cycle after this one |
| `when/@test` expression grammar spike | Research, not a feature; own cycle (RESEARCH R3) |
| `Dynamic` tree parsing (`choose`/`when`, visibility logic) | Depends on the unresolved grammar above; the raw bytes are retained regardless |
| Parameter *editor* / parameter value interpretation in the UI | Session 5 and later |
| `.knxprod` direct ingest for master data scheme ≥ 12 | KNOWN_LIMITATIONS §11, out of v1 scope |
| Schema 23 manufacturer data | Same blocker as schema 23 project data (RESEARCH §3.3, risk R1) |

---

## 2. Crate boundary

`knx-productdb` owns both the parser and the store, and depends on
`knx-core`, `rusqlite`, `quick-xml` and `sha2`. It depends on **neither**
`knx-etsproj` nor `knx-store`.

Two alternatives were considered and rejected:

**Parser in `knx-etsproj`, `knx-productdb` as storage only.** Reuses
`source.rs`, `known.rs` and `report.rs`, but welds the product model onto the
project-import crate — a later `.knxprod` ingest would have to travel through
the project importer — and grows the workspace's largest crate (~5,000 lines,
`map.rs` alone 1,337) further.

**A shared `knx-xml` primitives crate.** Extracting the tolerant reader,
`RetainedAttribute`/`RetainedElement` and `UnknownConstruct` would mean
refactoring tested, stable Session 3 code to serve a consumer that does not
need the retained-fragment machinery at all — see §4: because every source
file is stored whole as a blob, this parser's integrity guarantee comes from
the blob, not from retained fragments. The duplication that remains is roughly
100 lines of quick-xml streaming patterns.

`cargo run -p xtask -- check-layering` gains a third root: `knx-productdb`
must reach neither `knx-etsproj` nor `knx-store`. Like the existing two rules,
it is verified by observing it fail on a deliberate violation.

---

## 3. Data flow

Import:

```text
.knxproj
 └─ knx-etsproj  (container classification, as today)
     ├─ OpaqueKind::ManufacturerData + Baggage
     │     → ImportOutcome.manufacturer: Vec<ManufacturerFile>
     │       { source_path, bytes, sha256 }
     └─ everything else → ImportOutcome.opaque   (unchanged)
 └─ knx-app  (the only crate that sees both sides, as with OpaqueEntry today)
     ├─ manufacturer → knx_productdb::ingest_file(&product_conn, path, &bytes)
     ├─ knx_master.xml bytes → knx_productdb::ingest_master_data(&product_conn, &bytes)
     │     (a read of the opaque entry; the file itself stays in the opaque store)
     ├─ manifest (source_path, sha256, len, kind) → knx-store, schema v3
     ├─ opaque → knx-store  (unchanged)
     └─ knx_productdb::enrich(&mut project, &product_conn) → EnrichmentReport
```

Export:

```text
knx-app  loads the manifest
         → fetches each blob from products.sqlite by sha256
         → rebuilds OpaqueEntry { kind: ManufacturerData, bytes }
         → knx_etsproj::export_knxproj(project, &all_entries)   // signature unchanged
```

`export_knxproj`'s signature does not change: `knx-app` assembles the full
entry list before calling it, exactly as it already assembles stored opaque
entries.

**Warnings.** `ExportWarning::ManufacturerDataFromOpaqueStore` becomes
`ManufacturerDataFromProductDb { entries }` — the warning does not disappear,
it starts telling the truth about the source. A new
`ExportWarning::MissingManufacturerData { source_path, sha256 }` is emitted
once per file that the manifest names and the database cannot supply. The
container is then written without that entry, and the user is told which one,
rather than receiving a silently incomplete archive.

**Two decisions on what moves:**

- `M-xxxx.signature` entries stay in the project's opaque store. They are 175
  bytes each, they sign a container state rather than a product, and leaving
  them where they are keeps the export path for signatures unchanged.
- `Baggages/*` (a 641 KB and a 156 KB vendor DLL in the reference project)
  moves into the product database with the rest. It is manufacturer-scoped and
  identical across projects, so it deduplicates the same way. It continues to
  be stored and never executed.

Roundtrip guarantee 2 (every opaque entry returns with the same SHA-256) is
preserved: the bytes move to a different store, the hash is unchanged, and the
ingest verifies the computed hash against the manifest hash.

---

## 4. Database schema

`products.sqlite` carries **its own migration chain**, keyed off its own
`user_version`, with `knx_productdb::open_and_migrate` and
`CURRENT_PRODUCTDB_VERSION = 1`. It is deliberately not `knx-store`'s chain: a
project-schema bump must not force a product-database migration, or the other
way round.

All tables are `STRICT`. Version 1:

| Table | Columns (shape, not final DDL) | Purpose |
| --- | --- | --- |
| `schema_meta` | `key PK, value` | Version marker, as in `knx-store` |
| `source_file` | `sha256 PK, source_path, manufacturer_id, len, bytes BLOB` | The raw bytes of every ingested file, deduplicated by content hash |
| `manufacturer` | `id PK, name` | `M-0083` → display name, from `knx_master.xml` |
| `catalog_section` | `manufacturer_id, id PK, parent_id, name, number, visible_description, default_language` | Recursive catalog tree |
| `catalog_item` | `manufacturer_id, id PK, section_id, name, number, visible_description, product_ref_id, hardware2program_ref_id, default_language` | Leaf entries; the product picker's rows |
| `hardware` | `manufacturer_id, id PK, name, serial_number, version_number, bus_current, has_individual_address, has_application_program, is_accessory, is_coupler, is_power_supply, is_ip_enabled, is_power_line_repeater, original_manufacturer, source_sha256` | RESEARCH §4 |
| `product` | `manufacturer_id, id PK, hardware_id, text, order_number, is_rail_mounted, width_in_millimeter, default_language, hash, registration_status` | `DeviceInstance.product_ref` resolves here |
| `hardware2program` | `manufacturer_id, id PK, hardware_id, application_program_ref, medium_types, hash, registration_number, registration_status, registration_signature` | `DeviceInstance.program_ref` resolves here |
| `application_program` | `manufacturer_id, id PK, name, application_number, application_version, program_type, mask_version, pei_type, load_procedure_style, default_language, hash, dynamic_table_management, linkable, original_manufacturer, source_sha256, ingested_at` | Program head |
| `parameter_type` | `program_id, id PK, name, kind, size_in_bit, base, min_inclusive, max_inclusive, number_type` | `kind` is one of `Number`, `Restriction`, `Text`, `None`, `Float`, `IPAddress`, `Picture`, `Raw`, `Other` |
| `parameter_type_enum` | `program_id, parameter_type_id, id PK, value, text, display_order` | 1,096 rows for one reference device |
| `parameter` | `program_id, id PK, name, text, parameter_type_id, access, value, suffix, code_segment, offset, bit_offset, union_id, union_size_in_bit` | `union_id` is null for a standalone parameter |
| `parameter_ref` | `program_id, id PK, parameter_id, display_order, tag, text, value` | Nullable columns are per-variant overrides |
| `com_object` | `program_id, id PK, number, name, text, function_text, visible_description, object_size, priority, dpt_list, read_flag, write_flag, transmit_flag, update_flag, communication_flag, read_on_init_flag` | Program layer of the override chain |
| `com_object_ref` | `program_id, id PK, com_object_id, tag, text, function_text, visible_description, object_size, priority, dpt_list, read_flag, write_flag, transmit_flag, update_flag, communication_flag, read_on_init_flag` | ProgramRef layer; every override column nullable |
| `translation` | `program_id, language, ref_id, attribute_name, text`, PK over the first four | 5,919 rows for one reference program |
| `datapoint_type` | `id PK, main, sub, name, text` | From `knx_master.xml`; validates `DptRef` and names a type in the UI |
| `ingest_unknown` | `source_sha256, program_id, xpath, kind, name, occurrences, sample` | Unknown elements and attributes, counted, never fatal — the same rule `ImportReport` already follows |

Indexes: `source_file(manufacturer_id)`, `hardware2program(application_program_ref)`,
`product(hardware_id)`, `catalog_item(section_id)`, `com_object_ref(program_id)`,
`parameter_ref(program_id)`, `translation(program_id, language)`,
`parameter_type_enum(program_id, parameter_type_id)`.

**No separate blob column for the `Dynamic` subtree.** The whole file is
already in `source_file`; "Dynamic stays opaque" means the streaming parser
skips the subtree without materializing it. Nothing is lost, because nothing
was ever dropped: the bytes are the retention mechanism.

**`knx_master.xml` is ingested minimally** — `Manufacturers` (447 entries) and
`DatapointTypes` (46 main, 289 sub) only. The file itself stays in the
project's opaque store as `OpaqueKind::MasterData`, so the export path does not
change.

---

## 5. Ingest

For each manufacturer file handed in by `knx-app`:

1. Compute SHA-256. If `source_file` already holds that hash, **skip
   entirely — no parse.** This is what makes importing a second project with
   the same devices cheap, and it is the rule ROADMAP calls "skipping existing
   entries".
2. Otherwise store the blob, classify the file by the first element under
   `ManufacturerData` (`Catalog`, `Hardware`, `ApplicationPrograms`,
   `Baggages`), and run the matching parser. A baggage blob (`Baggages/*.dll`
   and similar) is stored without any parse at all.
3. One streaming pass, all inserts inside **one transaction per file**: a file
   is either wholly ingested or not at all. A failure mid-file leaves the
   database exactly as it was.
4. Unknown elements and attributes are counted into `ingest_unknown` with one
   sample value each, and never fail the ingest.

**Identity is the content hash, not a name.** The same program in a new
version has a different hash *and* a different id (the id encodes the version:
`M-0083_A-0019-13-A892` versus `M-0083_A-0019-16-ECA7`), so both revisions
coexist and nothing is overwritten. A file whose bytes changed but whose id did
not — a re-signed or re-exported program — produces a second `source_file` row
and an id collision on the entity tables. The ingest resolves this by keeping
the first ingested entity rows and writing an `ingest_unknown` row with
`kind = 'IdConflict'`, `name` = the colliding id and `sample` = the other
hash, so both hashes and the winning row stay on record. Existing rows are
never silently rewritten: product data is evidence, not a cache to be
invalidated by guesswork.

**Memory.** The parser streams; the largest file in the reference project is
5.7 MB and the whole manufacturer set 22 MB. The bytes are in memory anyway,
because they arrive from the container reader, so the blob insert adds no new
peak. Nothing accumulates across files.

---

## 6. Project manifest and `knx-store` schema v3

`knx-store` gains schema version 3 with one table:

```sql
CREATE TABLE manufacturer_ref (
    id          INTEGER PRIMARY KEY,
    source_path TEXT NOT NULL,
    sha256      TEXT NOT NULL,
    len         INTEGER NOT NULL,
    kind        TEXT NOT NULL
) STRICT;
CREATE INDEX manufacturer_ref_sha256 ON manufacturer_ref (sha256);
```

This is what lets a project state which manufacturer files it was imported
with, even when the product database is absent — the difference between a
nameable gap and silent data loss. A frozen fixture `fixtures/v3-empty.sqlite`
is generated with `cargo run -p xtask -- freeze-fixture`, matching the existing
v1 and v2 fixtures. `knx_core::CURRENT_SCHEMA_VERSION` moves to 3 in lockstep,
as it did in Session 3.

---

## 7. Enrichment

`knx_productdb::enrich(&mut Project, &Connection) -> EnrichmentReport` runs
after import and after opening a project.

**Resolution chain.** `DeviceInstance.program_ref` (`Hardware2ProgramRefId`) →
`hardware2program` → `application_program_ref` → `application_program`. For
each of the device's communication objects, `ComObjectInstance.source.ets_id`
already holds the full `ComObjectRef` id (`M-006A_A-0001-22-26C0-O0079_O-1_R-10003`;
set in `map.rs:660`), so `ets_id` → `com_object_ref` → `com_object` resolves
both program layers without any change to `knx-core`.

**Enrichment fills `Override::Absent` slots only.** `Empty`, `Malformed` and
`Value(Instance)` are never touched. The reason is concrete: 497 of the
reference project's 907 `ComObjectInstanceRef` elements carry
`DatapointType=""`. If a program value overwrote such a slot, the writer —
which emits `Override::Empty` as an empty attribute and skips a
non-`is_exported()` `Value` entirely — would export the attribute as *absent*
instead of *empty*, changing the file. For `Empty` slots the program value
remains available by querying the product database; it is not baked into the
model. Extending `Override<T>` into a layer stack is the alternative, and it is
a domain-model change with a migration for a small gain: rejected here,
recorded as a limitation.

`ComObjectInstance.size` (`Option<Resolved<ObjectSize>>`) is not an `Override`
and is filled freely from `ComObject/@ObjectSize`, overridden by
`ComObjectRef/@ObjectSize` where present — precisely what the field's own doc
comment says it exists for. It is never exported, since
`Layer::is_exported()` excludes `Program` and `ProgramRef`.

**Datapoint type alternatives.** `ComObjectRef/@DatapointType` can hold a
space-separated list (RESEARCH §4.2, e.g. `"DPST-9-21 DPST-9-21"`). The list is
stored verbatim in `dpt_list`. When it holds more than one alternative,
enrichment fills **nothing** and records `EnrichmentIssue::AmbiguousDpt`.
Which alternative applies cannot be decided from one sample, and guessing would
be invented compatibility.

**Report.** `EnrichmentReport` carries, per entry, the device or communication
object it concerns and one of: `Unavailable` (no product database),
`ProgramMissing { program_ref }`, `ComObjectRefMissing { ref_id }`,
`AmbiguousDpt { ref_id, alternatives }`. Counts are summarized the way
`ImportReport` already summarizes its own entries.

**Degradation.** No product database: the project opens, every communication
object shows the `Instance` layer only, the report says so once rather than per
object, and export names each file it cannot supply. One program missing: one
report entry per affected device, everything else is enriched. A project never
depends on the presence of manufacturer data — ADR-0005's rule, now with a code
path that proves it.

---

## 8. CLI

```text
knx import <file.knxproj> [--store <path>] [--report-json <path>]
                          [--product-db <path>] [--no-product-db]
knx products list [--manufacturer M-xxxx]
knx products ingest <file.knxproj>
knx products show <program-id>
knx products verify
```

Default database path is `$XDG_DATA_HOME/knx/products.sqlite`, falling back to
`~/.local/share/knx/products.sqlite` when `XDG_DATA_HOME` is unset. Creating
it is part of the first ingest, not a separate setup step.

`--no-product-db` runs exactly the Session 3 path: manufacturer bytes into the
project's opaque store, no ingest, no enrichment. It exists so that the
fallback is a tested path rather than an assertion, and so a user who does not
want a shared database still gets a complete, exportable project.

`knx products verify` re-hashes every blob and reports any row whose stored
bytes no longer match its `sha256` key.

Exit codes follow IMPORT_EXPORT §6.1 unchanged: 0 on success, 1 when no project
could be produced, 2 when a project was produced but its report carries
`Severity::Error` entries. Enrichment issues are warnings, not errors — a
device whose application program is absent is an incomplete project, not a
failed import.

---

## 9. Testing

| Test | What it pins down |
| --- | --- |
| Parser units against small XML excerpts | Catalog, hardware, program head, `TypeRestriction` + `Enumeration`, `Union`/`Memory`, translations |
| `ingest_is_idempotent` | The same file twice yields one `source_file` row, and the second run measurably does not parse |
| Golden ingest of the reference project | Manufacturer, program, communication-object, parameter, enumeration and translation counts — **measured during implementation, never guessed** |
| `export_with_product_db_matches_export_without` | Byte-identical export whether manufacturer data came from the opaque store or the product database. This is the proof that the detour changes nothing |
| `roundtrip_opaque_bytes_are_hash_identical` (existing, extended) | ADR-0007 guarantee 2 still holds across the new storage split |
| `project_opens_without_product_db` | Removing the database after import: the project opens, names what is missing, and exports with warnings |
| Malformed suite | Truncated program XML, unknown elements, a manifest hash that does not match its blob, an id collision across two different hashes |
| Migration v2 → v3 with the frozen fixture | The chain, as for v1 and v2 |
| Oracle against `xknxproject` | Communication-object texts and datapoint types where that tool is not known to be lossy (RESEARCH §7.1). Test path only; it is GPL-2.0-only and must never enter the runtime graph (ADR-0002) |

---

## 10. Documentation to update

| Document | Change |
| --- | --- |
| `IMPORT_EXPORT.md` §10 | Target design becomes implemented behavior; the manifest and the export reconstruction path are described |
| `KNOWN_LIMITATIONS.md` §12 | Lifted for communication objects; restated for what remains (parameter interpretation, `Empty`-slot program values, ambiguous DPT lists) |
| `ARCHITECTURE.md` | `knx-productdb`'s responsibility; the third `check-layering` root |
| `DATA_MODEL.md` §3 | `Program` / `ProgramRef` layers are now populated, and by what |
| `COMPATIBILITY.md` | New verified rows, each naming its test |
| `IMPLEMENTATION_STATUS.md`, `ROADMAP.md` | Session 4 status, and what was deliberately deferred |
| `adr/0011-product-database-storage.md` (new) | Blobs *and* parsed tables; content hash as identity |
| `adr/0012-enrichment-into-absent-slots.md` (new) | Why enrichment never touches `Empty` / `Malformed` / `Value(Instance)` |

---

## 11. Risks

| Risk | Handling |
| --- | --- |
| Manufacturer XML shapes beyond the four manufacturers in the reference project | The parser is tolerant by construction and counts what it does not know; the blob keeps everything regardless. Single-sample bias stays recorded (KNOWN_LIMITATIONS §1) |
| Ingest performance on 22 MB across 12 programs | Streaming, one pass, one transaction per file, and the content-hash skip on re-import. Measured in the golden test rather than asserted |
| Product database grows without bound across many projects | Out of scope this session; `knx products verify` and the per-file provenance in `source_file` are the groundwork for a later prune command |
| An `Empty`-slot program value being invisible in the model | Recorded as a limitation with its lift condition (a layer stack in `Override<T>`), not hidden |
| Licensing of redistributed manufacturer data | Unchanged by this session and structurally improved by it: product data now lives in a store that can be distributed, imported or withheld independently (RESEARCH §10, ADR-0005) |
