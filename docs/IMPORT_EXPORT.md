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
against a real protected project.

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

## 5. Opaque store

A table in the project file:

```text
(source_path, kind, bytes, sha256)
```

Contents (RESEARCH §7) [V]:

| Item | Content |
| --- | --- |
| `<M>/Baggages/*.dll` | Windows PE binaries — ETS plug-ins, e.g. `econEts3.dll` (641 KB), `FastDownload.dll` (160 KB) |
| `<P>/BinaryData/<guid>.dat` | 8-byte header, `<BlobInfo>`, CSV payload; keyed by `BinaryData/@Id` on `DeviceInstance` |
| `<P>/ExtraData/*.rbg`, `*.azp` | ISO-8859 CSV with CRLF, legacy ETS3-era plug-in data |
| `*.signature` | RSA signatures over manufacturer and project data |
| `Options/Legacy*` flags | Per-application compatibility switches |
| `RegistrationInfo` and `Hash` attributes | Certification metadata |
| Unknown XML fragments | Anything the known-element list does not cover |

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
    pub opaque: Vec<OpaqueEntry>,             // what was preserved verbatim, and why
    pub inferred: Vec<InferredValue>,         // e.g. DPT from linked objects
    pub conflicts: Vec<Conflict>,             // e.g. divergent DPTs on one GA
    pub unsupported: Vec<UnsupportedFeature>, // e.g. baggage DLL → device read-only
    pub errors: Vec<ImportError>,
}
```

`counts` carries read and mapped figures per entity type, so that a discrepancy
is visible as a number rather than as a suspicion.

**Rule:** if the importer drops information for which there is neither a model
representation nor an opaque entry, that is a bug in the importer — not a
report entry, and not an acceptable outcome. Silent discarding is not permitted
anywhere in this pipeline.

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

The comparison relation of guarantee 1 is part of the contract, not an
implementation detail: it is declared in code, and a change to it is a change
to what fidelity means here.

See [ADR-0007](adr/0007-roundtrip-fidelity.md).

## 10. Product database ingest

On project import, manufacturer data is **not** copied into the project. It is
ingested into the separate product database, keyed by manufacturer, application
program and version, with a content hash. Entries that already exist are
skipped.

The project holds references only. Consequences: 22 MB of application data is
stored once rather than once per project, and the licensing separation
(RESEARCH §10) is structural rather than a matter of discipline.

If the referenced product data is missing when a project is opened, **the
project still opens.** Communication objects then show the `Instance` layer
only, clearly marked incomplete. A project must never depend on the presence of
manufacturer data.

See [ADR-0005](adr/0005-separate-product-database.md).
