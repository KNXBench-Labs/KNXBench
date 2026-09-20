# Import and export

The contract Session 3 implements. Evidence markers follow
[RESEARCH.md](RESEARCH.md): **[V]** verified against the reference project or
against source code, **[D]** documented elsewhere but not verified here, **[A]**
assumption.

## 1. Pipeline

Six stages. Each has its own error type, and no stage knows the next one.

```text
.knxproj (ZIP)
  → Container    unpack, entry inventory (can decrypt ZipCrypto < 21 given a password; AES ≥ 21 refused — but no caller passes a password yet)
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

**Password protection** (RESEARCH §2.3): a protected project nests the
payload as `<P-xxxx>.zip`.

- Schema < 21 (ETS4, ETS5): standard ZipCrypto (PKWARE APPNOTE.TXT
  §6.0/§6.1 [D]), password taken as UTF-8 bytes — `[A]`, inferred from
  `xknxproject`'s source, which is evidence about `xknxproject`, not
  about ETS. APPNOTE itself leaves the password's byte encoding
  unspecified, and no real protected ETS export has been available to
  settle it. An ASCII password cannot tell the two readings apart, so the
  inference is untested where it matters.
- Schema ≥ 21 (ETS6): AES ZIP, with the password derived as
  `base64(PBKDF2-HMAC-SHA256(password = utf-16-le(user_password), salt =
  "21.project.ets.knx.org", iterations = 65536, dklen = 32))` — specified
  and verified against the KNX Standard's own test vectors (`knx-secure`).

The reference project is unprotected, so **neither path has been run
against a real protected project**. `Container::open` (no password) still
refuses a nested `<P-xxxx>.zip` payload by name
(`ContainerError::PasswordProtected`) — unchanged. Given a password,
`Container::open_with_password` now decrypts the ZipCrypto (schema < 21)
case, tested against synthetic fixtures built with an independent ZIP
tool (KNOWN_LIMITATIONS.md §13); ZipCrypto is not real security and this
exists only to read a file the caller already has the password to
(`knx_secure::zipcrypto`). The AES (schema ≥ 21) case is still refused by
name (`ContainerError::UnsupportedEncryption`), not attempted — neither
scheme may be described as *verified* before it has been tested against a
real protected project.

**Nothing in the import pipeline passes a password.** `import()` calls
`Container::open`, not `open_with_password`; the only callers of the
latter are its own tests. The decryption above is a capability of stage 1,
not of the pipeline, and a protected project is still an import failure
from the caller's point of view. It stays that way until a password
reaches `import()` together with an `ImportReport` entry for the
roundtrip gap a decrypted project carries (KNOWN_LIMITATIONS.md §13).

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
| A known-but-not-modelled attribute (`Installation/@BCUKey`, `@SplitType`, `ProjectInformation`'s tool-state attributes; `DeviceInstance`'s `LoadedImage`/`CheckSums`/`DownloadCounter`, ETS's differential-download state, `Project Schema23 v01.00.00.pdf` p. 44) | Name and value, keyed by the element's own ETS id (`knx_etsproj::xpath`) so export puts it back on the element it came from; where two source elements share one key the value is dropped and an export warning says so (`KNOWN_LIMITATIONS.md` [#34](KNOWN_LIMITATIONS.md#34-schema-21-export-drops-a-handful-of-known-but-unmapped-per-deviceper-line-attributes--resolved-2026-09-20)) | `RetainedAttribute` |
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

**Amendment (C14): `OpaqueSummary` also carries `xpath` and `name`.**
Without them, every `RetainedAttribute`/`RetainedElement` row reads
`reason: "known or unknown attribute the model does not carry"` and nothing
else — a preserved `LoadedImage` is indistinguishable from a preserved
`Comment`. Both are cheap (short strings, not the bytes the Session 3
amendment above was written to keep out) and both were already on the
underlying `OpaqueEntry`; they were just not copied into the summary. Empty
for a whole-file entry or a manufacturer file, same as on `OpaqueEntry`
itself.

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

A group address without a group range is not silently discarded. Both the
schema-11 and schema-21 writers return `ExportError::UnrangedGroupAddress`,
because the verified writer shape emits addresses only inside `GroupRange` and
KNXBench has no evidenced faithful external representation for a range-less
address. Native `.knxdb` persistence continues to preserve it.

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

### 9.1 Attribute-level fidelity, measured

The three guarantees above are about the model and the opaque bytes. They
say nothing about the `.knxproj` a user exports and opens elsewhere, so
that is measured separately:
`crates/knx-etsproj/tests/retained_v21_measurement.rs` imports each corpus
project, exports it, and compares every element of both documents keyed by
its ancestors' own ids — counting attributes that went in and did not come
out, and attributes that came out holding a different value.

As of 2026-09-20 the ETS4 (schema 11) project loses nothing; the KV
(schema 21) and ETS 6.3.0 (schema 23) projects lose only
`Installation/@Name` and `@DefaultLine`, both empty strings in the source,
which the domain model cannot tell apart from absent. Values that change:
`KNX/@CreatedBy` and `@ToolVersion`, deliberately, because this
application is not ETS; and `DeviceInstance/@LastDownload`/`@LastModified`,
reformatted to fewer fractional-second digits by the trip through a typed
timestamp.

Anything an export cannot put back is reported as an `ExportWarning` —
`RetainedAttributeNotExported` for an attribute, `RetainedElementNotExported`
for a whole element — one warning per `(element, attribute)` class with the
number of instances behind it, carrying a rendered `detail` sentence that
says which of three things happened: the writer had nowhere to put it, the
value's owning element could not be identified, or the project wrote a
different value than the import preserved. These travel the same route to
the UI as import diagnostics, through `ExportWarningDto` in
`apps/knx-server`.

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
*interpretation* (evaluating the `Dynamic` tree — its `when/@test` value
grammar is now documented, RESEARCH §4.3/R3, and its structural grammar is
corpus-observed; `knx-productdb` evaluates it headlessly as of T18 slice 1
(2026-09-11), but nothing outside that crate calls it — the raw bytes are
retained regardless, per ADR-0011); a layer stack in
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
later, a second, ETS-shaped column profile would fit alongside this one by
extending the importer's header matching (`map_headers` in
`crates/knx-csv/src/read.rs`) — one hard-coded `match` block, not a plug-in
seam, so adding a profile means editing that function rather than
registering with it, but the change stays contained to it. That is the
upgrade path, and it is the only claim of ETS interoperability this feature
is entitled to make.

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

## 12. Project documentation export

**This is KNXBench's own document, not an ETS report.** ETS can print
topology, building, device, and group-address reports to paper or PDF, but
no ETS-produced report sample — no PDF, no printout, no exported document
of any kind — exists anywhere in this repository, and `docs/RESEARCH.md`
has no section on ETS's report layout. So this feature is never called an
"ETS report" or a replacement for one, in code, docs, UI text, or CLI
output, and its content is chosen from what this domain model actually
holds, not from a remembered ETS layout. The same rule §11 states for the
CSV format applies here.

Implemented in `crates/knx-report`, a pure crate depending only on
`knx-core`, `knx-projection`, and `chrono` — no filesystem, HTTP, SQLite,
or system-clock access. Its one entry point:

```rust
pub fn render_html(project: &Project, options: &ReportOptions) -> HtmlReport;
```

`ReportOptions::generated_at` is the only source of "now" — server, CLI,
and tests each supply their own, so the same project and timestamp always
render to byte-identical HTML. `HtmlReport` carries the rendered `html`
`String` plus `warnings: Vec<ReportWarning>` — one entry per structural
oddity the walk finds (a device in no line, a group address in no range, a
building part with a dangling parent, an orphaned communication object, a
link to a group address that does not exist, or a malformed
`Override::Malformed` field). `render_html` cannot fail: a non-empty
`warnings` list describes problems in the *project*, never an error in
rendering, and every warning is rendered inline in the document itself as
well as returned to the caller — CLAUDE.md's "never silently discard
information" applied to an artifact that must carry its own caveats even
if a caller discards `warnings`.

**The document is one self-contained UTF-8 HTML file.** No JavaScript, no
external assets (no images, no web fonts, no network URLs of any kind),
one inline `<style>` block. It survives being emailed, copied to a USB
stick, or opened on a machine with no network. It contains no animation or
transition — a printed page cannot offer a toggle, so it has nothing that
would need one.

**Sections, in order:** Header (project name/number, group-address style,
completion, last modified, project start, ETS/domain schema versions, plus
the generation timestamp) — Contents (an anchor list) — Summary (counts:
installations, areas, lines, devices, communication objects, group ranges,
group addresses, building parts, parameter values) — Topology (area → line
→ device, plus devices assigned to no line) — Buildings (the flat
parent/children list resolved into its real nesting, each part's devices,
plus any part whose parent does not resolve) — Group addresses (ranges
nested main → middle, each address with its formatted form, name,
`Central`/`Unfiltered`, and every linked communication object's device,
object number, name, DPT and direction; plus addresses inside no range) —
Devices (name, individual address, description, commissioning state, and
the raw `product_ref`/`program_ref` identifiers, each device's
communication objects with number, name, description, DPT, the resolved
layer, the six flags, active state and links, plus any orphaned
communication object) — "What this report does not contain".

**A group address has no datapoint type of its own.** The DPT belongs to
its linked communication objects, which may disagree, so the document
lists each linked object's own DPT rather than printing one derived
consensus value — a deliberately different computation from `knx-csv`'s
`derive_dpt` (§11), answering a different question, so no logic is shared
between the two crates.

**The document states its own limits**, in its own last section, not only
in this file: manufacturer, product and application-program names are not
resolved (the identifiers are printed verbatim — resolving them needs
`knx-productdb`, which this crate must not reach); parameter values and
module-instance arguments are counted but not listed (they remain
uninterpreted raw data here — the `@test` value grammar that would let a
reader evaluate them is documented, RESEARCH §4.3/R3, and `knx-productdb`
now evaluates it headlessly (T18 slice 1), but `knx-report` does not reach
that crate either); binary data is referenced by name and id
only; text renders in the project's default language only; and the
document says plainly that it is not an ETS report and has not been
compared to one.

**PDF is produced by the browser's own print dialog**, not by KNXBench —
the document ships `@media print` rules (no page breaks inside a table
row, each top-level section starts a new page) for exactly that. There is
no Rust PDF renderer in this workspace and none is planned; a browser
already has one.

**Surfaces.** Server: `POST /api/project/documentation-export {path}` →
`{warnings}` (`apps/knx-server/src/routes.rs`), writing through the same
`resolve_new_project_path` helper the `.knxproj` and CSV exports use, and
logging one T11 session-log entry per warning under `source: "doc-export"`
without resetting the log. CLI: `knx doc-export <store.knxdb> <out.html>`
(`apps/knx-cli`), printing a summary and every warning, exiting `1` only
when no file could be produced at all (a report with warnings is still a
complete, correct report, so there is no separate warning exit code). Web:
an "Export documentation…" button (`DocumentationExportButton.tsx`) in the
same toolbar row as the `.knxproj` and CSV export controls.

Not implemented, and recorded here rather than only in
`KNOWN_LIMITATIONS.md`: PDF generation without a browser; an in-application
print preview; section selection or filtering; multiple languages in one
document; manufacturer/product/program name resolution; parameter and
module-argument listings.

Design record: `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`.
