# Import and export

## PDB-3 product-install evidence projection (schema v12)

Product-package installs expose the core encounter ledger through the server and
CLI. A measured report contains deterministic count rows for every encountered
category/disposition (including `read`, `stored`, `deduplicated`,
`retained-but-uninterpreted`, `unsupported`, and `dropped`), distinct unknown
constructs with occurrence totals, and bounded unsupported diagnostics. The
report preserves relative archive/XML paths only; it never exposes the host
`source_name`.

`facts: null` means the package was installed before schema v12 recorded the
ledger and is therefore historical/unavailable. It is not the same as measured
zero. The projection is reporting only: signature members are stored but not
verified, and no ETS parity claim is made. PDB-8 is the future typed master-data
coverage slice. PDB-10 (schema v16, ADR-0042) inventories baggage: every `Baggages.xml` declaration typed as raw lexemes and resolved exactly to its member, every payload classified by content, nested ZIPs measured from their directory only. The `baggage_index` count is `stored`, and
`unsupported-baggage-index` gave way to `unresolved-baggage-declaration`
(per index and reason) and `undeclared-baggage-payload` (per member).

**Space types (T13, 2026-09-22).** Import preserves `Stairway`, `RoomPart`,
`Area`, `Ground` and `Segment` alongside the six previously supported types.
Schema23 §1.1.2.3 omits `RoomPart`, while §1.2.6.4 lists both `RoomPart` and
`Segment`; see [DATA_MODEL §5](DATA_MODEL.md#5-two-orthogonal-hierarchies).
Synthetic schema-23 tests prove mapping, not real-project compatibility for
these five values. Unknown ETS values remain reported mapping errors.
Native schema-v9 save/load/re-save preserves exact kinds; unknown stored kinds
fail with `StoreError::UnknownBuildingPartType`, preventing silent rewriting.
There is no ETS project exporter (ADR-0028).

The contract Session 3 implements. Evidence markers follow
[RESEARCH.md](RESEARCH.md): **[V]** verified against the reference project or
against source code, **[D]** documented elsewhere but not verified here, **[A]**
assumption.

**`.knxproj` is read-only.** KNXBench imports it and never writes it
([ADR-0028](adr/0028-no-knxproj-export.md), 2026-09-20); the project then
lives in KNXBench's own `.knxdb` store. The "export" in this document's
title is the group-address CSV (section 11) and the project documentation
(section 12), neither of which is an ETS format. Being able to read a
`.knxproj` is not the same as being able to replace ETS, and nothing here
claims otherwise.

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

### 4.1 Structural validation (stage 4)

Tolerant parsing is not the same as unchecked import. `knx_etsproj::validate`
runs between parse and mapping — the position `CLAUDE.md`'s data flow
prescribes — and classifies structural problems without ever modifying the
document or aborting the import. An **error** is data the mapper cannot use;
a **warning** is data it can use but should not trust. Both reach the user
through `ImportReport::errors`, tagged `stage: "validate"`.

| Check | Severity | Scope |
| --- | --- | --- |
| Duplicate `@Id` (`Area`, `Line`, `DeviceInstance`, `BinaryData`, `GroupRange`, `GroupAddress`, `BuildingPart`) | Error | Document-wide — ETS ids embed their installation |
| Dangling `Connectors/Send\|Receive/@GroupAddressRefId` (schema 11) | Error | Document-wide, same reason |
| Dangling `Links` target (schema ≥21, short ids such as `GA-3`) | Error | Per installation — a short id embeds no installation |
| Two devices on one individual address | Warning | Per installation |
| Two group addresses on one address | Warning | Per installation |
| Group address outside its enclosing `GroupRange`'s bounds | Warning | Per range |

The set is bounded by what the corpus can attest rather than by an
authoritative schema, because there is none
([KNOWN_LIMITATIONS §2](KNOWN_LIMITATIONS.md#2-no-authoritative-xsd-is-publicly-available)).
Two things follow, both documented in `validate.rs`'s own header with the
measurement behind them: `ModuleInstance/@Id` is not checked for uniqueness
(it is device-scoped — the KV schema-21 project repeats 16 of its 32 ids
across devices sharing one application program), and the reference checks
that need either the mapper's id tables (`@DefaultLine`, `DeviceInstanceRef`)
or the product database (`ProductRefId`) stay with those stages, so each
class of problem has exactly one reporting channel.

## 5. Opaque store

A table in the project file (`opaque_entry`, `knx-store` schema version 2):

```text
(source_path, xpath, kind, name, bytes, sha256)
```

`xpath`/`name` extend the four-column shape this section originally
sketched: an opaque entry is not always a whole file — a retained attribute
needs to say which element it belongs to and what it was called, so the
import report can name *where in the source file* the preserved value was
found rather than just pointing at the archive. Both are empty for a whole
container entry. (The keys were written for a writer that replayed them;
the writer is gone since [ADR-0028](adr/0028-no-knxproj-export.md), the
keys are not — locating the evidence is the point now.)

Contents (RESEARCH §7) [V], now with `OpaqueKind`, since the code exists:

| Item | Content | `OpaqueKind` |
| --- | --- | --- |
| `<M>/Baggages/*.dll` | Windows PE binaries — ETS plug-ins, e.g. `econEts3.dll` (641 KB), `FastDownload.dll` (160 KB) | `Baggage` |
| `<M>/*` (everything else under a manufacturer directory) | Catalog, hardware, application program XML — Session 3 arrangement, moves to the product database in Session 4 | `ManufacturerData` |
| `<P>/BinaryData/<guid>.dat` | 8-byte header, `<BlobInfo>`, CSV payload; keyed by `BinaryData/@Id` on `DeviceInstance` | `BinaryData` |
| `<P>/ExtraData/*.rbg`, `*.azp` | ISO-8859 CSV with CRLF, legacy ETS3-era plug-in data | `ExtraData` |
| `*.signature` | RSA signatures over manufacturer and project data | `Signature` |
| `knx_master.xml` | DPT/product master catalogue | `MasterData` |
| Any other container entry | Copied through unchanged | `ContainerEntry` |
| A known-but-not-modelled attribute (`Installation/@BCUKey`, `@SplitType`, `ProjectInformation`'s tool-state attributes; `DeviceInstance`'s `LoadedImage`/`CheckSums`/`DownloadCounter`, ETS's differential-download state, `Project Schema23 v01.00.00.pdf` p. 44) | Name and value, keyed by the element's own ETS id (`knx_etsproj::xpath`), one key per source element instance, so the report can say which element the value came from | `RetainedAttribute` |
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
3. **Never alter.** The stored bytes are the source's bytes, and the
   SHA-256 proves it. Nothing in KNXBench rewrites them — since
   [ADR-0028](adr/0028-no-knxproj-export.md) nothing writes them back out
   at all, so they are evidence of what the source file said and nothing
   else.

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
text is held in `Override::Malformed` (ADR-0010's amendment), so the exact
text the file carried stays visible in the project and in diagnostics
instead of being replaced by a guess. A field modelled as a bare value or
an `Option` has nowhere to keep the raw text, so it falls back to the
type's default with the problem reported — the loss is visible in the
report, never silent.

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
and stay distinguishable from what the file actually said: a guess is never
allowed to lose its label and pass itself off as the user's own data.

Where several linked objects declare **different** datapoint types on one group
address, that is a `Conflict`. The group address keeps no datapoint type, the
conflict appears in the report, and the UI shows it. There is no silent
majority vote and no first-wins rule — the disagreement is information, and
resolving it is the user's decision.

## 8. Writing `.knxproj` — withdrawn 2026-09-20

KNXBench reads `.knxproj` and does not write one. A project is imported
once and lives in KNXBench's own `.knxdb` store (ADR-0003) from then on.

The writer existed: `knx-etsproj::export` produced schema-11 and schema-21
documents, replayed retained attributes and elements from the opaque store,
and reported through `ExportWarning` what it could not put back. It was
deleted on 2026-09-20 by [ADR-0028](adr/0028-no-knxproj-export.md), with the
CLI subcommand, the HTTP route and the toolbar button that reached it. No
library function, no CLI subcommand, no HTTP route and no UI control writes
a `.knxproj` any more.

Import is unaffected — nothing about what KNXBench *understands* when it
reads a file changed. The opaque store, the retained keys, the schema-≥21
communication-object flag resolution and every import report entry stay
exactly as they were.

The other exports are not `.knxproj` writing and are untouched: group-address
CSV exchange (section 11), project documentation export (section 12), the
debug report and the project diff.

**Consequence for the user.** There is no supported path from KNXBench back
into ETS. Anyone who needs one keeps their original `.knxproj` — KNXBench
reads it without consuming or modifying it.

## 9. Import fidelity — what reading a file has to preserve

Round-trip fidelity as ADR-0007 defined it (import → export → import,
opaque hashes equal across a write, every export unsigned) described an
operation that no longer exists. What survives is the half that was always
about reading:

| # | Guarantee | Where it is tested |
| --- | --- | --- |
| 1 | **Nothing is silently discarded.** Anything the model cannot represent is either an opaque entry or a report entry, and a value that is neither is a bug in the importer | the import test suites in `crates/knx-etsproj/tests/` |
| 2 | **Opaque bytes are the source's bytes.** Every opaque entry is stored with the SHA-256 it arrived with, whole-file or retained attribute | the `opaque` unit tests in `crates/knx-etsproj/src/opaque.rs`, plus `crates/knx-etsproj/tests/download_state.rs` |
| 3 | **Retained values keep their address.** Each retained attribute is keyed to the single source element instance it came from (`knx_etsproj::xpath`), so the report can say where it was | `download_state_attributes_are_preserved_and_reported_by_name` |

Byte equality of anything KNXBench produces is not attempted and is never
claimed (risk R4), which is now trivially true: KNXBench produces no
`.knxproj` at all. Risk R9 — whether ETS re-imports an unsigned
third-party file — is closed as not applicable for the same reason.

`SemanticProject` in `knx-etsproj/src/compare.rs` (`semantic_view`,
`describe_difference`) still declares what "these two imports produced the
same project" means. It has no caller since the round-trip test was deleted;
it is kept because the relation is about the model, not about the writer.

See [ADR-0028](adr/0028-no-knxproj-export.md), which supersedes
[ADR-0007](adr/0007-roundtrip-fidelity.md).

### 9.1 Attribute-level fidelity, as last measured

Until 2026-09-20 `crates/knx-etsproj/tests/retained_v21_measurement.rs`
imported each corpus project, exported it, and compared both documents
element by element, counting attributes that went in and did not come out.
The test went with the exporter. Its final result is recorded here because
it is the sharpest statement of how much of a source file the importer
actually holds on to:

> ETS4 (schema 11) lost nothing. KV (schema 21) and ETS 6.3.0 (schema 23)
> lost only `Installation/@Name` and `@DefaultLine`, both empty strings in
> the source, which the domain model cannot tell apart from absent.

That is a measurement of the import side seen through a writer, not a
promise about any file KNXBench produces, and no equivalent measurement
exists now that there is no second document to compare against. The
empty-string gap is a real import-side limitation and is recorded as such
in `KNOWN_LIMITATIONS.md`.

### 9.2 Communication-object `RefId` shapes, and which one a schema writes

A `ComObjectInstanceRef/@RefId` — and, at schema ≥21, every id in
`GroupObjectTree/@GroupObjectInstances` — comes in three shapes. The
importer reads all three, and reads nothing else.

| Shape | Example | Where seen | How the object number is obtained |
| --- | --- | --- | --- |
| Fully qualified | `M-006A_A-0001-22-26C0-O0079_O-0_R-10001` | ETS4, schema 11 | `values::com_object_number` — the `O-<n>` of the final two segments |
| Module-based | `MD-2_M-1_MI-1_O-2-0_R-4` | KV demo, schema 21 | `values::module_com_object_ref` — the *second* numeral of `O-<a>-<b>` |
| Device-local | `O-3_R-10005` | ETS 6.3.0, schema 23 | `values::device_local_com_object_number` — the `O-<n>` head |

The device-local shape carries no application-program prefix because it does
not need one: the `DeviceInstance` that holds it already names its own
`Hardware2ProgramRefId`, and the container's own `M-<n>/Hardware.xml`
resolves that to exactly one `ApplicationProgramRef`. The prefix is implied
by position, not missing. Measured over the whole ETS 6.3.0 reference
project: all 867 of its `GroupObjectTree` ids and all 691 of its
`ComObjectInstanceRef/@RefId`s are two-segment, and reattaching each one's
own device program names an existing `ComObjectRef/@Id` 867 times out of
867, with the `ComObject/@Number` behind it equal to the id's own `O-<n>`
digits every time.

**Where the reconstruction happens, and where it does not.** The project
model keeps the id exactly as the file wrote it
(`ComObjectInstance::source::ets_id`); nothing is rewritten on the way in.
The fully-qualified form is rebuilt only at the moment of a product-database
lookup (`knx_productdb::com_object_lookup_id`), using the program *that
device* resolved to. An id whose shape is none of the three above is
reported as `MalformedRefId` and mapped to object number `0`; a rebuilt id
the database does not know is reported as
`EnrichmentIssue::ComObjectRefMissing`. Neither is ever attached to a
different device's object — the same rule `KNOWN_LIMITATIONS.md` §34 sets,
applied on the read side. Evidence base and its limits:
`KNOWN_LIMITATIONS.md` §125.

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

Standalone-package ZIP names are decoded before any path decision
([ADR-0034](adr/0034-zip-member-names-follow-declared-encoding.md)): bit-11
names must be valid UTF-8; unflagged names use the ZIP-defined CP437 path. A
version-1 Unicode-path field overrides that path only when its raw-name CRC and
UTF-8 are valid; stale CRCs/unknown versions fall back, while duplicate or
conflicting declarations fail. Local and central names, flags and compression
methods, CRCs and sizes must agree; local ranges may not alias or overlap, data
descriptors are verified, and ZIP64 member metadata is rejected in this bounded
slice. Parser-effective offsets, CRCs and sizes must still match that preflight
record, so an earlier EOCD cannot become an unchecked fallback. The
resulting name is then checked for traversal, absolute/drive-like paths,
NUL/backslash, symlinks and normalized file/prefix collisions, and that exact
checked value becomes `PackageMember.path`. This admits the measured
eleven-package legacy-name aggregate without publishing its composition,
relaxing the archive boundary or implying broader ETS compatibility.

The opt-in compatibility-matrix regression
(`crates/knx-productdb/tests/corpus_compatibility_matrix.rs`) measures a
configured private product corpus in two modes: each package against an empty
isolated database, then all packages against one shared database in stable hash
order. `KNXBENCH_PRODUCT_CORPUS` is the mandatory read-only confinement root;
`KNXBENCH_PRODUCT_CORPUS_SCOPES` is a mandatory OS path list selecting exact
directories below it. `KNXBENCH_PRODUCT_MATRIX_OUTPUT` must be outside that
root and receives JSON containing only ordinals, scheme numbers, per-instance
`{status}` outcomes for success/deduplication and `{status, category}` for
rejection, aggregate report totals, final table counts, and one
aggregate identity/outcome commitment. Detailed per-instance report counts stay
inside that commitment because their stable vectors can fingerprint individual
packages; the JSON never publishes those vectors, individual hashes, source
paths, filenames, or manufacturer labels. The gated implementation is Linux-only: descriptor confinement uses
`openat2`, and descriptor-backed directory enumeration requires mounted
`/proc`. PDB-8 (schema v14) re-pinned the aggregate commitment after a
main-vs-branch run showed exactly two of 31 final table counts changed,
`package_install_count` 3,277 → 3,390 and `package_install_diagnostic`
645 → 880, both now pinned explicitly; outcomes and report totals were equal.
PDB-9 (schema v15) re-pinned it again: exactly two of 31 final table counts
changed, `ingest_unknown` 23,040 → 23,051 and `package_install_unknown`
9,245 → 9,251, because 75 `Element TypeColor`/`TypeTime` report rows became
86 unread-attribute rows (26 → 32 distinct per package). The unknown report
totals moved by the same +11 (22,758 → 22,769 per instance, 22,642 →
22,653 shared); an independent Python recount of the same package instances
predicts every one of these deltas, and no product table changed.
PDB-10 (schema v16) re-pinned it a third time: the new
`package_baggage_inventory`/`_payload`/`_declaration` tables hold 113 / 789 /
776 rows, and the only existing count that moved is
`package_install_diagnostic` 880 → 859 — 34 `unsupported-baggage-index` rows
(one per non-empty index; 3 of 37 indexes are empty) replaced by 13
`undeclared-baggage-payload` rows and no unresolved declaration. Outcomes and
report totals were equal, and an independent Python recount predicts each
number.
The 2026-09-24
gate bound 115 instances / 113 unique hashes: isolated 104 installed and 11
unsupported; shared 102 installed, 2 already present, and 11 unsupported.
`isolation_report_totals` sums only the 104 successful isolated installs.
`shared_installed_report_totals` sums only the 102 attempts that inserted new
rows; `shared_successful_attempt_report_totals` additionally includes both
deduplicated attempts, whose installer reports replay the already stored
package's report even though they add no rows. Final table counts remain a
fourth, distinct measurement. The output directory is held by descriptor for
the full run and must be on a different filesystem from the corpus, preventing
a concurrent rename from turning publication into a corpus write.
This is a repeatable compatibility baseline, not a claim that preserved unknown
XML is understood or that ETS would produce the same database.

**The project manifest.** `knx-store` schema v3 adds `manufacturer_ref`
(`source_path`, `sha256`, `len`, `kind`) — what a project was imported
with, independent of whether the product database that supplied the bytes
is still around. `M-xxxx.signature` entries are the one exception and stay
in the project's own opaque store: they sign a container state, not a
product, so they are kept with the project rather than with the products.

**Resolving the manifest.** The manifest is what makes the split
recoverable: each entry's SHA-256 fetches the original bytes back out of
the product database (`knx_productdb::load_source_file`), so a project can
still say what it was imported with. A manifest entry the database cannot
supply is a named gap, not a silent one — the project still opens and the
missing file is reported by `source_path` and `sha256`
(`a_project_names_its_manufacturer_gap_when_the_product_database_is_gone`,
`crates/knx-app/tests/product_db.rs`). Before 2026-09-20 this path also fed
the `.knxproj` writer; the writer is gone (ADR-0028) and the manifest is
not, because knowing the provenance of a device's application program is an
import property.

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
| `Action` | `upsert` | optional; empty/`upsert` creates or updates, `readdress` moves the existing entry named by `Address`, and `delete` removes that existing entry |
| `NewAddress` | empty | required and non-zero for `readdress`; forbidden for `upsert` and `delete` |
| `Name` | the entry's name | **required**, must be non-empty |
| `Central` | `true`/`false` | optional; accepted spellings (case-insensitive): `true`/`false`/`1`/`0`/`yes`/`no`; empty means "false" when creating a new address and "leave unchanged" when updating an existing one |
| `Unfiltered` | `true`/`false` | same rules as `Central` |
| `DatapointType (read-only)` | derived from the linked communication objects' DPTs: unanimous → that DPT, none linked → empty, disagreement → empty plus one export warning naming the address | recognized and validated against the project, but **never applied**; changing the cell is an explicit row error rather than a silently ignored edit. The legacy `DatapointType` header remains accepted with the same semantics |
| `MainGroup (read-only)` | the name of the containing main group range, if any | recognized and validated, never applied; legacy `MainGroup` remains accepted |
| `MiddleGroup (read-only)` | the name of the containing middle group range, if any | recognized and validated, never applied; legacy `MiddleGroup` remains accepted |

Unknown columns (anything not in the table above) are collected and
reported by name once per file, never silently ignored.

**Per row, import produces one of six outcomes:** *create* (no existing entry
has that address), *update* (an entry has that address and at least one
applied column differs), *readdress*, *delete*, *unchanged*, or *error*. A
row is an error when: the address column is missing, unparseable, out of
range for the project's address style, or is `0` (reserved for broadcast,
never a valid group address); the name is missing or blank; a `Central`/
`Unfiltered` cell is present but not a recognized boolean spelling; or an
address appears more than once in the same file. Unknown actions, malformed
shapes, duplicate target claims and targets that already
exist are also errors. Duplicate recognized headers are file-level errors;
the first value is retained for diagnostics, never silently overwritten.
Readdress swaps and moves into an address another row
deletes are intentionally rejected: targets must be free in the project being
previewed, independent of CSV row order.

**All-or-nothing.** If any row in the file is an error, nothing from the
file is applied — the report names every offending row (1-based, counting
the header, so it matches what a spreadsheet shows) so the user can fix the
file and retry. A hand-edited spreadsheet that is half-good and half-broken
either applies in full or not at all; there is no partial apply.

**Destructive operations are explicit and two-phase.** Omitting a project
address from the file still never deletes it, and changing only `Address`
still creates a new entry. Readdressing requires `Action=readdress` with the
old `Address` and a separate `NewAddress`; deletion requires
`Action=delete`. The plan lists every move/delete and the communication-object
ids linked to its stable group-address id. Readdress preserves that id, so
all links remain attached. Delete is rejected while any link still refers to
the id.

Server and web return this plan without applying it and issue an opaque
confirmation token bound to the exact CSV bytes, server incarnation and
project revision. Only a second request with that token applies the batch;
a changed CSV or project is rejected and must be previewed again. CLI prints
a SHA-256 token bound to the exact CSV and semantic project snapshot and
requires a second `--confirm <token>` invocation; it reloads and rechecks the
project after acquiring SQLite's write lock, then saves in that same
transaction. Both
paths remain all-or-nothing, and the server records the applied batch as one
undo step.

**What import still never does**, stated as plainly as what it does:

- **It never applies `DatapointType`, `MainGroup`, or `MiddleGroup`.** These
  three read-only columns show derived context. Their values are validated;
  edits are rejected instead of ignored or silently dropped.
- **It never creates or renames group ranges.** A newly created address is
  placed into the innermost existing group range whose bounds already
  contain it, if any; if none contains it, the address is created without a
  range and the report says so. No range is ever created by a CSV import.
- **There are no `Description`/`Comment` columns**, in either direction.
  The `.knxproj` schema has `GroupAddress/@Description` and `@Comment`
  attributes, but this domain model does not carry them yet
  (`GroupAddressEntry`, `crates/knx-core/src/group.rs`), so the CSV format
  cannot round-trip fields that do not exist here.

**One server undo step.** A successful server/web import applies as one
`Command::Batch`, so the whole import is one undo. The standalone CLI writes
the resulting project file and has no cross-process undo history.

**Surfaces.** Server: `POST /api/group-addresses/csv-export` and
`POST /api/group-addresses/csv-import` (`apps/knx-server`), both path-based
like the project import route, and both logging to the
T11 session log. CLI: `knx ga-export <store.knxdb> <out.csv>` and
`knx ga-import <store.knxdb> <in.csv> [--dry-run] [--confirm <token>]`
(`apps/knx-cli`) —
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
`Override::Malformed` field) and per caller-supplied product projection that
cannot be rendered faithfully. `render_html` cannot fail: a non-empty
`warnings` list describes source/project/product-data limitations, never an
I/O error in rendering, and every warning is rendered inline in the document
itself as well as returned to the caller — CLAUDE.md's "never silently discard
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
Devices (name, individual address, description, commissioning state, raw
`product_ref`/`program_ref` identifiers, hardware-consistent resolved
manufacturer/product/program names, stored parameters and module arguments,
each device's
communication objects with number, name, description, DPT, the resolved
layer, the six flags, active state and links, plus any orphaned
communication object) — "Limits and warnings". Header, contents, and this
last section remain mandatory even when an API caller selects only a subset
of the five content sections.

**A group address has no datapoint type of its own.** The DPT belongs to
its linked communication objects, which may disagree, so the document
lists each linked object's own DPT rather than printing one derived
consensus value — a deliberately different computation from `knx-csv`'s
`derive_dpt` (§11), answering a different question, so no logic is shared
between the two crates.

**The document states its own limits**, in its own last section, not only
in this file. `knx-app` resolves display-only product data outside the pure
renderer. A product/program pair must share hardware; mismatches retain both
raw references, attach no foreign program metadata, and warn. Missing or
whitespace-only identity/field names fall back to raw identifiers and warn.
Restriction enum labels are formatted when declared; unknown restriction
values and every parameter kind without a report formatter remain raw and
warn. Module arguments remain raw bindings: `AllocatorRef`, unknown kinds,
allocation metadata and repeat semantics are not interpreted. Binary data is
referenced by name and id only, and the document says plainly that it is not
an ETS report and has not been compared to one.

**PDF is produced by the browser's own print dialog**, not by KNXBench —
the document ships `@media print` rules (no page breaks inside a table
row, each top-level section starts a new page) for exactly that. There is
no Rust PDF renderer in this workspace and none is planned; a browser
already has one.

**Surfaces.** Server: `POST /api/project/documentation-export` accepts `path`
plus optional `language` and `sections`, returning `{warnings}`
(`apps/knx-server/src/routes.rs`), writing through the same
`resolve_new_project_path` helper the CSV export uses, and
logging one T11 session-log entry per warning under `source: "doc-export"`
without resetting the log. `POST /api/project/documentation-preview` accepts
the same language/section selection and returns `{html, warnings}` without
creating a file or changing the session log. CLI:
`knx doc-export <store.knxdb> <out.html>`
(`apps/knx-cli`), printing a summary and every warning, exiting `1` only
when no file could be produced at all (a report with warnings is still a
complete, correct report, so there is no separate warning exit code). Web:
an "Export documentation…" button (`DocumentationExportButton.tsx`) in the
same toolbar row as the CSV export controls.

Not implemented, and recorded here rather than only in
`KNOWN_LIMITATIONS.md`: PDF generation without a browser; a frontend
preview/print action or section/language controls; multiple languages in one
document; a third/report-pack language; formatters for non-restriction
parameter kinds; and interpretation of the unsupported module semantics named
above.

Design record: `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`.
