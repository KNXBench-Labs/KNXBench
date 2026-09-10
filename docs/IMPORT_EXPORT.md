# Import and export

The contract Session 3 implements. Evidence markers follow
[RESEARCH.md](RESEARCH.md): **[V]** verified against the reference project or
against source code, **[D]** documented elsewhere but not verified here, **[A]**
assumption.

## 1. Pipeline

Six stages. Each has its own error type, and no stage knows the next one.

```text
.knxproj (ZIP)
  → Container    unpack, decrypt (ZipCrypto < 21 / AES ≥ 21), entry inventory
  → Detect       schema version from the default namespace, never assumed
  → Parse        tolerant XML reader per schema version → SourceDocument
  → Validate     structural checks, reference resolution, conflicts
  → Map          SourceDocument → knx-core Project (with provenance)
  → Report       ImportReport
```

The separation matters in both directions. `Parse` produces a `SourceDocument`
that is still shaped like the source, so that parsing can be tested against raw
XML without a domain model. `Map` is the only stage that knows `knx-core`, so
that a schema change stops there instead of reaching the model.

Parsing streams; there is no DOM. One application program file reaches 5.7 MB,
and a project unpacks to about 22 MB (RESEARCH §4.1) [V].

## 2. Container handling

A `.knxproj` is a plain ZIP archive. The observed layout (RESEARCH §2.1) [V]:

```text
knx_master.xml                  KNX master data: DPTs, manufacturers, mask versions, media
<M-xxxx>.signature              one per referenced manufacturer
<P-xxxx>.signature              one for the project
<M-xxxx>/Catalog.xml            manufacturer product catalogue tree
<M-xxxx>/Hardware.xml           hardware, products, hardware-to-program mapping
<M-xxxx>/Baggages.xml           optional, references binary baggage
<M-xxxx>/<M-xxxx>_A-<app>.xml   application programs, one file each
<M-xxxx>/Baggages/*.dll         opaque manufacturer binaries
<P-xxxx>/Project.xml            project metadata only
<P-xxxx>/0.xml                  installation 0: topology, buildings, group addresses
<P-xxxx>/BinaryData/<guid>.dat  opaque per-device blobs
<P-xxxx>/ExtraData/*.azp, *.rbg opaque legacy ETS3-era plug-in data
```

The project identifier is recovered from the `P-*.signature` filename [V].

**Filename case differs by generation.** ETS4 writes `Project.xml`; ETS5 and
ETS6 write `project.xml` [V]. Entry lookup is therefore case-insensitive over
the inventory, not a hard-coded name.

**Password protection** (RESEARCH §2.3) [V, from `xknxproject` source]: a
protected project nests the payload as `<P-xxxx>.zip`.

- Schema < 21 (ETS4, ETS5): standard ZipCrypto, password taken as UTF-8 bytes.
- Schema ≥ 21 (ETS6): AES ZIP, with the password derived as
  `base64(PBKDF2-HMAC-SHA256(password = utf-16-le(user_password), salt =
  "21.project.ets.knx.org", iterations = 65536, dklen = 32))`.

The reference project is unprotected, so **both paths are unverified in
practice**. Neither may be described as supported before it has been tested
against a real protected project. Detection is implemented and tested
(`Container::open` refuses a nested `<P-xxxx>.zip` payload by name,
`ContainerError::PasswordProtected`); decryption is not.

**Container entry size guard (Session 3, malformed-input hardening):** an
entry whose declared uncompressed size exceeds 64 MB is refused before any
allocation for it, not after. Measured justification: the largest single
entry in either reference project is 5.7 MB (RESEARCH §4.1), and the whole
uncompressed reference container is 22 MB — 64 MB leaves ample headroom for
a legitimate project while refusing a zip-bomb-shaped entry that declares
gigabytes it does not contain. Without this guard, `Vec::with_capacity`
allocating a declared multi-gigabyte size can abort the process outright
(an allocation failure that large calls Rust's global allocator error
handler, not a recoverable `Result::Err`) rather than fail cleanly.

## 3. Schema detection

The schema version is the trailing integer of the default XML namespace, for
example `http://knx.org/xml/project/11`. It is **read from the file, never
assumed** — including never inferred from a filename or a ZIP entry layout.

| Namespace version | ETS generation | Evidence |
| --- | --- | --- |
| 11 | ETS 4.1 / 4.2 | **[V]** |
| 12 | ETS 4 | **[D]** |
| 13, 14 | ETS 5 up to 5.6 | **[D]** |
| 20 | ETS 5.7 | **[D]** |
| 21, 22, 23 … | ETS 6.x | **[D]** |

Only version 11 has been seen. Everything else in this table is documentation
we have not confirmed, and the tolerant parser is what turns that gap into a
report instead of a failure.

The version is read from `P-*/0.xml` rather than from `knx_master.xml`, which
avoids a dependency on where master data sits [A]. `xknxproject` reads it from
`knx_master.xml` instead [V]; the two are expected to agree, and a disagreement
is a report entry.

## 4. Tolerant parsing

No authoritative XSD is publicly available. The official schemas ship with the
Manufacturer Tool via the KNX GitLab account, which requires membership
(RESEARCH §2.2, risk R2) [D]. We therefore cannot validate against an
authoritative schema, and pretending otherwise would only move the failure
later.

The parser works against an **explicit list of known elements and attributes
per schema version**. Anything outside that list is not an error. It is:

1. stored verbatim in the opaque store, and
2. counted in the import report with its source path and XPath.

This is what makes the first ETS5 or ETS6 import produce a concrete list of
unknown constructs instead of a crash. It is the mitigation for the
single-sample bias of risk R1: we have one ETS 4.1 project and no ETS5/ETS6
sample, so the parser is built to report its own ignorance rather than to
assume it has none.

Streaming rather than DOM is forced by the same numbers as above: 5.7 MB in one
file, 22 MB unpacked, 5919 translation elements in a single application program
(RESEARCH §4.1) [V].

**Nesting depth is bounded by design, not by an explicit limit (Session 3,
malformed-input hardening).** The parser holds its own path/frame stacks on
the heap rather than recursing per element, so it cannot overflow the Rust
call stack regardless of document depth — verified with a 10,000-level
`<GroupRange>` document, which the existing truncation handling already
turns into a clean `ParseError::Xml`, not a crash, with no depth-specific
code added.

## 5. Opaque store

A table in the project file (`opaque_entry`, `knx-store` schema version 2):

```text
(source_path, xpath, kind, name, bytes, sha256)
```

`xpath`/`name` extend the four-column shape this section originally
sketched: an opaque entry is not always a whole file — a retained attribute
needs to say which element it belongs to and what it was called, so export
can put it back on the right element rather than just somewhere in the
archive. Both are empty for a whole container entry.

Contents (RESEARCH §7) [V], now with `OpaqueKind`, since the code exists:

| Item | Content | `OpaqueKind` |
| --- | --- | --- |
| `<M>/Baggages/*.dll` | Windows PE binaries — ETS plug-ins, e.g. `econEts3.dll` (641 KB), `FastDownload.dll` (160 KB) | `Baggage` |
| `<M>/*` (everything else under a manufacturer directory) | Catalog, hardware, application program XML — Session 3 arrangement, moves to the product database in Session 4 | `ManufacturerData` |
| `<P>/BinaryData/<guid>.dat` | 8-byte header, `<BlobInfo>`, CSV payload; keyed by `BinaryData/@Id` on `DeviceInstance` | `BinaryData` |
| `<P>/ExtraData/*.rbg`, `*.azp` | ISO-8859 CSV with CRLF, legacy ETS3-era plug-in data | `ExtraData` |
| `*.signature` | RSA signatures over manufacturer and project data | `Signature` |
| `knx_master.xml` | DPT/product master catalogue | `MasterData` |
| Any other container entry not regenerated on export | Copied through unchanged | `ContainerEntry` |
| A known-but-not-modelled attribute (`Installation/@BCUKey`, `@SplitType`, `ProjectInformation`'s tool-state attributes) | Name and value, matched back onto its element by `(xpath, name)` on export | `RetainedAttribute` |
| An unrecognized element, or a known-but-not-modelled element (`BusAccess`) | Raw bytes, tag included | `RetainedElement` |

**Fidelity by construct**, the promised column — modeled in the domain
model, retained opaquely, or a documented unsupported feature — measured
against the reference project, not intentions:

| Construct | Status |
| --- | --- |
| `Project`, `Installation`, `Area`, `Line`, `DeviceInstance`, `ComObjectInstanceRef`, `GroupRange`, `GroupAddress`, `BuildingPart`, `DeviceInstanceRef`, `Connectors/Send`/`Receive` | Modeled |
| `ParameterInstanceRef` | Modeled, retained uninterpreted (section 4/§10 of DATA_MODEL.md) |
| `Installation/@BCUKey`, `@SplitType`, `ProjectInformation`'s tool-state attributes (`ProjectTracingLevel`, `Hide16BitGroupsFromLegacyPlugins`) | Retained opaque (`RetainedAttribute`) |
| `BusAccess` | Retained opaque (`RetainedElement`) |
| `BinaryData` blobs, `ExtraData`, `*.signature`, `knx_master.xml` | Retained opaque (whole-file `OpaqueEntry`) |
| Manufacturer application program data (`<M-xxxx>/*`) | Retained opaque this session (`ManufacturerData`); moves to the shared product database in Session 4 |
| Vendor plug-in DLLs (`<M-xxxx>/Baggages/*`) | Retained opaque, never executed; reported as `UnsupportedFeature` — the device is read-only here |
| Any element or attribute outside the known-element table for the detected schema version | Retained opaque, reported in `unknown` |
| Schema versions other than 11 | Not read — `NoKnownSchemaTable`, a named import failure, not silent wrong data |

Three rules, without exception:

1. **Never execute.** Vendor DLLs are bytes we copy. They are never loaded.
2. **Never interpret.** Opaque content has no meaning inside our model.
3. **Write back unchanged.** Export reproduces the stored bytes exactly, and
   the SHA-256 proves it.

The vendor plug-in binaries are the concrete reason full ETS compatibility is
not achievable and is never claimed: part of some devices' configuration
behaviour lives inside a vendor DLL, and no independent tool can reproduce it.

See [ADR-0006](adr/0006-opaque-passthrough-store.md).

## 6. The import report

The report is a deliverable, not a log. It is structured, persisted in the
project file, displayed in both the UI and the CLI, and exportable as JSON.

```rust
pub struct ImportReport {
    pub source: SourceInfo,                   // file, size, schema version, ETS version
    pub counts: EntityCounts,                 // per entity type: read / mapped
    pub unknown: Vec<UnknownConstruct>,       // element/attribute, path, frequency
    pub opaque: Vec<OpaqueSummary>,           // what was preserved verbatim, and why — no bytes
    pub inferred: Vec<InferredValue>,         // e.g. DPT from linked objects
    pub conflicts: Vec<Conflict>,             // e.g. divergent DPTs on one GA
    pub unsupported: Vec<UnsupportedFeature>, // e.g. baggage DLL → device read-only
    pub errors: Vec<ImportError>,             // validation and mapping problems, tagged by stage and severity
}
```

**Amendment (Session 3): `opaque` holds `OpaqueSummary`, not `OpaqueEntry`.**
This section originally sketched the raw entry type directly in the report;
the implemented type omits `bytes` (`OpaqueSummary` carries `source_path`,
`kind`, `size`, `sha256`, `reason` instead) specifically so that importing
a project with 22 MB of manufacturer data does not produce a multi-megabyte
JSON report — the store holds the bytes, the report only says they exist
and why. `errors` is likewise not the raw `SourceProblem`/`MapProblem` enums
Stage 4/5 use internally: each is flattened to `{stage, severity, xpath,
detail}` so the report stays trivially serializable without coupling its
JSON shape to those enums' exact variants.

`counts` carries read and mapped figures per entity type, so that a discrepancy
is visible as a number rather than as a suspicion.

**Rule:** if the importer drops information for which there is neither a model
representation nor an opaque entry, that is a bug in the importer — not a
report entry, and not an acceptable outcome. Silent discarding is not permitted
anywhere in this pipeline.

**Unparsable values.** A present attribute whose value the importer cannot
parse is kept, not dropped. On a field modelled as `Override<T>` the raw
text is held in `Override::Malformed` (ADR-0010's amendment) and written
back verbatim on export, so a file that arrives with an unreadable value
leaves with the same one. A field modelled as a bare value or an `Option`
has nowhere to keep the raw text, so it falls back to the type's default
with the problem reported — the loss is visible in the report, never
silent.

### 6.1 CLI exit codes

`knx import` distinguishes three outcomes, so a script does not have to
parse the JSON report to learn which one it got:

| Code | Meaning |
| --- | --- |
| 0 | A project was produced and its report contains no `Severity::Error` entries. Warnings and unknown constructs do not change this. |
| 1 | No project was produced: bad usage, an unreadable file, an unsupported schema, or a store that could not be opened. |
| 2 | A project was produced, but the report contains at least one `Severity::Error` entry — data the mapper could not use, such as a dangling reference or a duplicate id. |

Code 2 exists because 0 and 1 alone force a choice between hiding real
structural errors and failing an import that did in fact produce a usable
project.

## 7. Inference and conflicts

A group address may carry no datapoint type of its own — 194 of 514 in the
reference project (RESEARCH §6.1) [V]. Where one can be derived from the
communication objects linked to it, the derived value is used for display and
is marked `Layer::Inferred`. Inferred values are shown as inferred in the UI
and are **never exported**: writing them back would silently promote our guess
into the user's project.

Where several linked objects declare **different** datapoint types on one group
address, that is a `Conflict`. The group address keeps no datapoint type, the
conflict appears in the report, and the UI shows it. There is no silent
majority vote and no first-wins rule — the disagreement is information, and
resolving it is the user's decision.

## 8. Export

Export writes:

- every value whose layer is `Instance` or `UserEdit`
  ([ADR-0004](adr/0004-provenance-model.md));
- every opaque entry, byte-identical;
- nothing derived from the product database, and nothing marked `Inferred`.

**Every export is unsigned.** Signatures are RSA over manufacturer and project
data and cannot be regenerated without KNX signing keys (RESEARCH §7) [V].

Whether ETS re-imports an unsigned file written by a third-party tool is
**untested** (risk R9). The export path states this to the user at export time,
and keeps stating it until someone has verified it against a real ETS
installation. No claim of ETS interoperability is made on the strength of the
file being well-formed.

## 9. Roundtrip fidelity

Byte equality is not attempted and is never claimed (risk R4). Three testable
guarantees replace it, each of which becomes a named test in Session 3:

| # | Guarantee | Test |
| --- | --- | --- |
| 1 | **Semantic equality** — import → export → import yields a model equal to the first, under a comparison relation with ordering normalized and internal IDs excluded | `roundtrip_model_is_semantically_equal` |
| 2 | **Opaque equality** — every opaque entry returns with the same SHA-256 | `roundtrip_opaque_bytes_are_hash_identical` |
| 3 | **Unsigned** — every export is unsigned, and says so | `export_is_unsigned_and_reports_it` |

The comparison relation of guarantee 1 is declared as `SemanticProject` in
`knx-etsproj/src/compare.rs` (`semantic_view`, `describe_difference`) — it
is part of the contract, not an implementation detail, and a change to it
is a change to what fidelity means here.

**A fourth check, convergence, was added during Session 3 implementation:**
`a_second_roundtrip_changes_nothing_further` asserts that exporting the
*re-imported* project a second time produces the same container-entry names
and the same `0.xml` bytes as the first export. Not one of the three
declared guarantees (it follows from them, rather than adding a new
dimension of fidelity), but worth stating explicitly: a pipeline that keeps
normalizing something differently on every pass would satisfy guarantees
1–3 on each individual roundtrip while still not being a roundtrip in the
ordinary sense of the word.

See [ADR-0007](adr/0007-roundtrip-fidelity.md).

## 10. Product database ingest

**Implemented (Session 4).** `<M-xxxx>/*` container entries (catalog,
hardware, application program XML, and `Baggages/*` — everything except
`.signature` entries) are no longer copied into the project's own opaque
store. `knx-etsproj`'s `collect_container_entries` hands them out
separately as `ManufacturerFile` values (`ImportOutcome.manufacturer`);
`knx-app`, the one crate that sees both `knx-etsproj` and `knx-store`,
routes them into `knx-productdb` and writes a manifest naming what the
project was imported with.

**Ingest.** Each file's SHA-256 is the identity ([ADR-0011](adr/0011-product-database-storage.md)).
A hash already present in `source_file` is skipped without being parsed at
all — measured directly by
`a_second_ingest_of_the_same_files_stores_nothing_new`
(`crates/knx-productdb/tests/golden_reference_products.rs`) and by
`a_second_import_into_the_same_product_db_skips_every_file`
(`crates/knx-app/tests/product_db.rs`). Otherwise the file is classified
by its first recognized element (`Catalog`, `Hardware`,
`ApplicationPrograms`, `Baggages`, or a non-XML blob), stored as a blob and
parsed into entity tables inside one transaction, so a failure partway
through leaves the database exactly as it was
(`a_failing_parse_leaves_no_partial_rows_and_no_blob`). `knx_master.xml`
contributes `Manufacturers` and `DatapointTypes` only
(`knx_productdb::ingest_master_data`); the file itself stays in the
project's own opaque store as `OpaqueKind::MasterData`.

**The project manifest.** `knx-store` schema v3 adds `manufacturer_ref`
(`source_path`, `sha256`, `len`, `kind`) — what a project was imported
with, independent of whether the product database that supplied the bytes
is still around. `M-xxxx.signature` entries are the one exception and stay
in the project's own opaque store: they sign a container state, not a
product, so their export path is unchanged.

**Export.** `knx-app`'s `export_ets_project` loads the manifest, fetches
each file back out of the product database by its SHA-256
(`knx_productdb::load_source_file`), and reassembles the full
`OpaqueEntry` list `knx_etsproj::export::export_knxproj` needs — whose own
signature does not change. A manifest entry the database cannot supply
produces `ExportWarning::MissingManufacturerData { source_path, sha256 }`
naming exactly which file, and the container is written without it rather
than silently incomplete
(`a_project_opens_and_names_its_gap_when_the_product_database_is_gone`).
`export_is_byte_identical_with_and_without_the_product_database` is the
proof that routing manufacturer data through the shared database changes
nothing about what gets written back.

**Degradation.** `--no-product-db` (or `ImportOptions { product_db: None
}`) runs the Session 3 path unchanged: manufacturer bytes go into the
project's own opaque store, no ingest, no enrichment — a tested fallback,
not an assertion. Without any product database at all, a project still
opens; communication objects show the `Instance` layer only, and the
degradation is reported once per affected device rather than failing the
import (`EnrichmentReport`, [ADR-0012](adr/0012-enrichment-into-absent-slots.md)).
A project must never depend on the presence of manufacturer data.

**Enrichment.** `knx_productdb::enrich` resolves each device's
`Hardware2ProgramRefId` to an application program and each communication
object's `ComObjectInstanceRef` source id to a `ComObjectRef`/`ComObject`
pair, then fills `Override::Absent` slots only — see ADR-0012 for why
`Empty`, `Malformed` and instance-level `Value`s are never touched, and why
an ambiguous, space-separated `DatapointType` list fills nothing and is
reported (`EnrichmentIssue::AmbiguousDpt`) instead of guessed.

**Not part of this session's delivery**, carried forward: parameter
*interpretation* (the `Dynamic` tree and the `when/@test` grammar, RESEARCH
R3 — the raw bytes are retained regardless, per ADR-0011); a layer stack in
`Override<T>` that would make an `Empty`-slot program value visible without
risking the export change ADR-0012 rules out; `.knxprod` direct ingest for
master data scheme ≥ 12 (KNOWN_LIMITATIONS §11); schema 23 manufacturer
data (same blocker as schema 23 project data).

## 11. Group-address CSV exchange

**This is a format KNXBench defines and owns, not an ETS one.** ETS has its
own "Export Group Addresses" feature, but no sample of what it actually
writes exists in this repository, and none of the 179 documents in the
extracted KNX Standard v3.0.0 corpus specifies a group-address CSV/Excel
exchange format — group-address CSV export is an ETS *application* feature,
not something the KNX Association standardizes. So this document does not,
and cannot, claim that "KNXBench group-address CSV v1" round-trips through
ETS, and nothing in the UI, CLI, or server API may say "ETS CSV" or imply
that interoperability. If a genuine ETS-produced CSV sample is obtained
later, the importer's column-mapping layer (§ below) is built to take a
second, ETS-shaped column profile alongside this one without a rewrite —
that is the upgrade path, and it is the only claim of ETS interoperability
this feature is entitled to make.

Implemented in `crates/knx-csv`, a pure crate depending only on `knx-core`,
`csv`, and `serde` — it knows nothing of SQLite, HTTP, or the CLI. Its three
entry points: `export_group_addresses(&Project) -> CsvExport`,
`parse_group_addresses(text, style) -> ParsedCsv`, and
`plan_import(&Project, &ParsedCsv) -> ImportPlan`. Orchestration (reading a
path, applying the resulting command, saving the store) lives in
`apps/knx-server` and `apps/knx-cli`, both calling the same three functions.

**Encoding and framing.** Written UTF-8 with a leading BOM (Excel opens
BOM-less UTF-8 as the system code page and mangles non-ASCII names), read
with or without one. Written with CRLF line endings, read with either CRLF
or LF, including a file that mixes the two. Written with `,` as the
separator; read with `,` or `;`, auto-detected per file — a German-locale
Excel writes `;` and reads `,` depending on the OS list separator, so both
are accepted rather than picking one. Quoting follows RFC 4180: a field
containing the separator, a quote character, or a line break is
double-quote-quoted, with embedded quotes doubled.

**Header row.** Required. Columns are matched by name, case-insensitively,
with surrounding whitespace ignored; column order does not matter.

| Column | Written on export | Read on import |
| --- | --- | --- |
| `Address` | the address in the project's own group-address style (`Free`, `TwoLevel`, or `ThreeLevel` — a project-wide setting, never per-address) | **required**; this is the row's identity, matched against existing entries |
| `Name` | the entry's name | **required**, must be non-empty |
| `Central` | `true`/`false` | optional; accepted spellings (case-insensitive): `true`/`false`/`1`/`0`/`yes`/`no`; empty means "false" when creating a new address and "leave unchanged" when updating an existing one |
| `Unfiltered` | `true`/`false` | same rules as `Central` |
| `DatapointType` | derived from the linked communication objects' DPTs: unanimous → that DPT, none linked → empty, disagreement → empty plus one export warning naming the address | accepted but **never applied** — a group address itself carries no DPT in this domain model (only its linked communication objects do), so this column is recognized and counted in the import report as ignored, never rejected and never silently dropped |
| `MainGroup` | the name of the containing main group range, if any | accepted but **never applied**, same reporting treatment as `DatapointType` |
| `MiddleGroup` | the name of the containing middle group range, if any | accepted but **never applied**, same reporting treatment as `DatapointType` |

Unknown columns (anything not in the table above) are collected and
reported by name once per file, never silently ignored.

**Per row, import produces exactly one of four outcomes:** *create* (no
existing entry has that address), *update* (an entry has that address and
at least one applied column — `Name`, `Central`, or `Unfiltered` — differs),
*unchanged* (an entry has that address and nothing differs), or *error*. A
row is an error when: the address column is missing, unparseable, out of
range for the project's address style, or is `0` (reserved for broadcast,
never a valid group address); the name is missing or blank; a `Central`/
`Unfiltered` cell is present but not a recognized boolean spelling; or an
address appears more than once in the same file.

**All-or-nothing.** If any row in the file is an error, nothing from the
file is applied — the report names every offending row (1-based, counting
the header, so it matches what a spreadsheet shows) so the user can fix the
file and retry. A hand-edited spreadsheet that is half-good and half-broken
either applies in full or not at all; there is no partial apply.

**What import never does**, stated as plainly as what it does:

- **It never deletes.** An address present in the project but absent from
  the file is left alone. A CSV is an edit against the current project, not
  a replacement of it.
- **It never re-addresses.** The address is the match key, so changing an
  address in the spreadsheet is read as "create a new entry at the new
  address," not "move this entry." Deleting the old entry and creating the
  new one — already possible from the group-address view — is the supported
  way to re-address. See `KNOWN_LIMITATIONS.md`.
- **It never applies `DatapointType`, `MainGroup`, or `MiddleGroup`.** These
  three columns exist so an exported spreadsheet shows what each address is
  for, not just its bare address and name; they are read back and reported
  as recognized-but-ignored rather than rejected (which would make this
  tool's own export un-importable) or silently dropped.
- **It never creates or renames group ranges.** A newly created address is
  placed into the innermost existing group range whose bounds already
  contain it, if any; if none contains it, the address is created without a
  range and the report says so. No range is ever created by a CSV import.
- **There are no `Description`/`Comment` columns**, in either direction.
  The `.knxproj` schema has `GroupAddress/@Description` and `@Comment`
  attributes, but this domain model does not carry them yet
  (`GroupAddressEntry`, `crates/knx-core/src/group.rs`), so the CSV format
  cannot round-trip fields that do not exist here.

**One undo step.** A successful import applies as a single
`Command::Batch`, so the whole import is one undo, the same as T9's bulk
operations.

**Surfaces.** Server: `POST /api/group-addresses/csv-export` and
`POST /api/group-addresses/csv-import` (`apps/knx-server`), both path-based
like the existing `.knxproj` export/import routes, and both logging to the
T11 session log. CLI: `knx ga-export <store.knxdb> <out.csv>` and
`knx ga-import <store.knxdb> <in.csv> [--dry-run]` (`apps/knx-cli`) —
`--dry-run` runs the identical plan and prints a byte-identical report body
to a real import, then a trailing `store written: yes`/`no (…)` line makes
explicit whether anything was actually saved. Web: two toolbar buttons in
the group-address view (`GroupAddressCsvButtons.tsx`).

Design record: `docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`.
