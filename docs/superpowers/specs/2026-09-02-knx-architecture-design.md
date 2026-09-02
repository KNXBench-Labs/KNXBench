# Architecture Design — KNX Engineering Application

Date: 2026-09-02
Session: 1 (Architecture)
Status: approved in brainstorming, pending implementation plan
Input: [RESEARCH.md](../../RESEARCH.md) (Session 0, complete)

This document defines the architecture. It does not implement it. Every
non-obvious decision cites the Session 0 finding that forced it, so that a
later session can re-open a decision by re-checking the evidence rather than
re-arguing the opinion.

---

## 1. Scope

Build a Linux-first, KNX-compatible engineering application as an independent
alternative to ETS.

**v1 target — project editor without device parameter configuration.** Import
`.knxproj`, inspect and edit group addresses, links, building structure,
topology, device names and addresses, export, and monitor the live bus against
the open project.

**Explicitly out of v1:**

| Excluded | Reason |
| --- | --- |
| Device parameter editing / `Dynamic` tree evaluation | RESEARCH R3 — `choose`/`when` grammar unresearched; needs the Session 4 spike |
| Commissioning / device download | RESEARCH §8.3 — bricking risk, undocumented `Legacy*` matrix, vendor DLLs |
| KNX Secure | RESEARCH §9 — no sample material; subsystem exists but stays empty |
| `.knxprod` direct import for master data scheme ≥ 12 | RESEARCH §10 — encryption layer unresolved |

The wording in all user-facing text is **"KNX-compatible"**. Never "KNX
certified", never "full ETS compatibility" (RESEARCH §7, §10).

## 2. Technology decisions

| Area | Decision |
| --- | --- |
| Core language | Rust |
| Desktop shell | Tauri |
| UI | React + TypeScript, Vite |
| Working project format | SQLite, one file per project |
| Product database | SQLite, separate file, shared across projects |
| Headless entry point | `knx-cli`, first-class, not a by-product |
| Documentation and code language | English |

Rationale for Rust: the parser must stream 22 MB of application program XML
(RESEARCH §4.1), the product database needs indexing and caching (R7), and the
provenance model of §5.2 is far safer expressed in a type system than by
convention. It also removes the GPL exposure of R6 at the root — there is no
Python KNX library in the runtime graph at all.

Rationale for a first-class CLI: it forces the core to stay UI-free and makes
roundtrip and regression tests runnable in CI without a display.

## 3. Component structure

One Cargo workspace. Dependencies point downward only.

```text
apps/
  knx-desktop/     Tauri shell (Rust) + ui/ (React + TS, Vite)
  knx-cli/         import | report | export | monitor | db

crates/
  knx-core/        Domain model, addresses, DPT, override resolution, validation.
                   No IO, no XML, no SQL, no UI.
  knx-app/         Application services: open/save, commands, undo/redo,
                   search, selection, reports
  knx-store/       SQLite project storage, schema migrations, opaque store
  knx-etsproj/     .knxproj read/write: ZIP, schema detection, tolerant XML
                   parser, mapping to/from knx-core, import report
  knx-productdb/   Product database (own SQLite), ingest of manufacturer data,
                   indexed access to application programs
  knx-net/         KNXnet/IP: discovery, tunnelling, routing, cEMI, telegrams
  knx-secure/      Isolated key material subsystem (empty for now, but present)

xtask/             Build and verification tasks, including the license gate
tests/fixtures/    Reference projects, golden files
```

```text
knx-desktop ─┐
knx-cli ─────┴─> knx-app ─> knx-core
                    ├─> knx-store ────> knx-core
                    ├─> knx-etsproj ──> knx-core
                    ├─> knx-productdb ─> knx-core
                    ├─> knx-net ──────> knx-core
                    └─> knx-secure
```

`knx-etsproj` and `knx-store` are separate because the import format and the
storage format evolve independently. An ETS6 schema delta must not touch the
project file schema, and a model migration must not break the importer.

### 3.1 Mechanically enforced rules

These are tests, not guidelines. Each fails the build.

1. `knx-core` must not reach `serde_json`, `quick-xml`, `rusqlite` or `tokio`
   in its dependency graph. Checked in `xtask` via `cargo metadata`.
2. No runtime crate may depend on a GPL-licensed crate (R6). Enforced with
   `cargo-deny` and an explicit allowlist. `xknxproject` stays in `.venv`,
   invoked only by test scripts, never by the Rust build.
3. The UI communicates only through Tauri commands into `knx-app`. It has no
   path to `knx-store` or `knx-etsproj`.

## 4. Core architecture approach

The core is a **normalized domain model carrying provenance per value**
(brainstorming approach 1), with one borrowing from a source-faithful design:
an opaque store keyed by source path retains everything not modelled, verbatim,
including unknown XML constructs.

Two alternatives were considered and rejected:

- *Source-faithful document plus computed projection.* Rejected: the ETS schema
  would become the domain model, which CLAUDE.md forbids and which would make a
  second schema generation impossible to attach cleanly.
- *Event-sourced core.* Rejected for v1: every query needs materialization, and
  migrations would have to keep replaying historical commands — the most
  expensive form of schema versioning. Undo/redo is achieved with a command
  pattern instead, at a fraction of the cost.

## 5. Domain model (`knx-core`)

### 5.1 Identity

Two kinds of identifier, never conflated.

- **Internal IDs** (`DeviceId`, `GroupAddressId`, …): stable, project-unique,
  persisted. These are the primary keys.
- **`SourceRef { path, ets_id }`**: the original ETS identifier string, e.g.
  `M-006A_A-0001-22-26C0_O-0_R-10001`. Never used as a primary key — ETS ids
  collide across projects and change — but never discarded, because export and
  provenance need them.

### 5.2 The override chain as a type

RESEARCH §3.2 is the single most constraining finding: a communication object's
effective properties resolve through three layers, and 758 of 907 instances in
the reference project override the datapoint type at instance level. An import
that reads only the application program produces wrong data for the majority of
objects, and a model without provenance cannot decide what to write back.

Therefore no resolved scalar exists without its layer:

```rust
pub enum Layer { Program, ProgramRef, Instance, Inferred, UserEdit }

pub struct Resolved<T> { pub value: T, pub layer: Layer }

pub struct ComObjectInstance {
    pub id: ComObjectInstanceId,
    pub source: SourceRef,
    pub device: DeviceId,
    pub number: u16,                       // from _O-<n>
    pub text: Resolved<LocalizedString>,
    pub description: Option<Resolved<LocalizedString>>,
    pub dpt: Option<Resolved<DptRef>>,
    pub flags: Resolved<ComFlags>,
    pub size: Resolved<ObjectSize>,
    pub is_active: bool,
    pub links: Vec<GroupLink>,
}
```

Export semantics follow directly from the layer:

| Layer | Origin | Written to `0.xml` on export |
| --- | --- | --- |
| `Program`, `ProgramRef` | Product database | No |
| `Instance` | Present in the source project | Yes |
| `UserEdit` | Changed in this application | Yes |
| `Inferred` | Derived by us (e.g. DPT from linked objects) | No — shown in the UI as inferred |

### 5.3 Two orthogonal hierarchies

`Devices` is the sole owner of devices. `Topology` (Area → Line → devices) and
`Buildings` (recursive typed `BuildingPart` → devices) hold references only
(RESEARCH §3.1).

A device without a line is valid and lives in `Topology::unassigned`. The
reference project contains exactly one such device, and this is precisely where
`xknxproject` loses data (RESEARCH §7.1).

### 5.4 Directional links

`GroupLink { ga: GroupAddressId, direction: Send | Receive }`. Direction is
semantically meaningful — which object writes the group address versus which
listens — and is never flattened into an undirected association (RESEARCH
§3.1: 569 send links against 27 receive links in the reference project).

### 5.5 Commissioning state is domain data

Not import metadata. It describes the delta between the planned project and the
physical installation, which is engineering-critical (RESEARCH §3.1).

```rust
pub struct CommissioningState {
    pub completion: CompletionStatus,   // Undefined | Editing | FinishedDesign | Accepted
    pub individual_address_loaded: bool,
    pub application_program_loaded: bool,
    pub parameters_loaded: bool,
    pub communication_part_loaded: bool,
    pub medium_config_loaded: bool,
    pub last_modified: Option<DateTime<Utc>>,
    pub last_download: Option<DateTime<Utc>>,
    pub broken: bool,
}
```

### 5.6 Language-aware strings from the start

`LocalizedString` is a handle into a `StringTable` keyed by `(key, language)`,
not a `String`. Import populates it from the `TranslationUnit` trees — 5919
translation elements in a single application program (RESEARCH §4.1). Display
resolves against the active language with fallback to `DefaultLanguage`.

This is retrofit-hostile, which is why it is in the model from day one.

### 5.7 Addresses

Dedicated types, not integers. `IndividualAddress(u16)` exposing area/line/
device; `GroupAddress(u16)` rendered according to the project-wide
`GroupAddressStyle` (Free / TwoLevel / ThreeLevel). Parsing and formatting are
pure functions with typed errors.

A group address without a datapoint type is normal, not an error — 194 of 514
in the reference project (RESEARCH §6.1). So is a group address with no linked
communication object at all — 110 of 514. Both must survive import and export
unchanged.

### 5.8 Retained but uninterpreted

Kept in the model so that the commissioning path stays open (RESEARCH §8.3,
recommendation 11), but not interpreted in v1:

- `ParameterInstance { ref: SourceRef, raw: String }` — 1390 values in the
  reference project, dropped entirely by `xknxproject`.
- `Memory`, `AbsoluteSegment`, `LoadProcedures`, mask and resource data — held
  in the product database.

### 5.9 Versioning

`Project { schema_version: u32, .. }`. Migration is an ordered chain
`v_n → v_n+1` implemented in `knx-store`. Every version has a frozen fixture
that must keep loading.

## 6. Import and export

### 6.1 Pipeline

Each stage has its own error type. No stage knows the next one.

```text
.knxproj (ZIP)
  → Container    unpack, decrypt (ZipCrypto < 21 / AES ≥ 21), entry inventory
  → Detect       schema version from the default namespace, never assumed
  → Parse        tolerant XML reader per schema version → SourceDocument
  → Validate     structural checks, reference resolution, conflicts
  → Map          SourceDocument → knx-core Project (with provenance)
  → Report       ImportReport
```

Parsing streams with `quick-xml`; no DOM. A single application program file
reaches 5.7 MB, 22 MB unpacked in total (RESEARCH §4.1).

### 6.2 Tolerant parsing

No authoritative XSD is publicly available (RESEARCH §2.2, R2). The parser
therefore works against an explicit list of known elements and attributes per
schema version. Anything outside that list is not an error but a finding: it is
stored verbatim in the opaque store **and** counted in the report with its
source path and XPath.

This is what makes the first ETS5/ETS6 import produce a concrete list of
unknown constructs instead of a crash — the mitigation for the single-sample
bias of R1, given that no ETS5/ETS6 sample project is available yet.

### 6.3 Opaque store

A table in the project file: `(source_path, kind, bytes, sha256)`.

Contents: `*.signature`, `Baggages/*.dll`, `BinaryData/*.dat`,
`ExtraData/*.azp|*.rbg`, `Options/Legacy*` flags, `RegistrationInfo` and `Hash`
attributes, and every unknown XML fragment (RESEARCH §7).

Rules: never execute, never interpret, write back unchanged on export.

### 6.4 The import report is a deliverable

Structured, persisted in the project file, displayed in UI and CLI, exportable
as JSON. Not a log.

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

**Rule:** if the importer drops information for which there is neither a model
representation nor an opaque entry, that is a bug in the importer, not a report
entry. Silent discarding is not permitted anywhere.

### 6.5 Conflicts are reported, not resolved

The case left open in RESEARCH §6.1 — several linked objects declaring
different datapoint types on one group address — is recorded as a `Conflict`.
The group address stays without a datapoint type and the UI shows the conflict.
No silent majority vote.

### 6.6 Roundtrip fidelity, defined

Byte equality is not attempted and is never claimed (R4). Three testable
guarantees replace it:

1. **Semantic equality** — import → export → import yields a model equal to the
   first under a declared comparison relation (ordering normalized, internal
   IDs excluded).
2. **Opaque equality** — all opaque bytes are hash-identical.
3. **Unsigned** — every export is unsigned. Whether ETS re-imports it is
   untested (R9) and is stated to the user at export time until it has been
   verified.

### 6.7 Product database ingest

On project import, manufacturer data is **not** copied into the project. It is
ingested into the separate product database, keyed by
`(manufacturer, application_program, version)` with a hash. Existing entries are
skipped. The project holds references only.

Consequences: 22 MB of application data is stored once rather than per project,
and the licensing separation demanded by RESEARCH §10 is structural rather than
a matter of discipline.

If the referenced product data is missing when a project is opened, the project
still opens. Communication objects then show only the `Instance` layer, clearly
marked incomplete. A project must never depend on the presence of manufacturer
data.

## 7. Application layer (`knx-app`)

### 7.1 Commands, not setters

Every mutation is a `Command` with `apply(&mut Project) -> Result<Inverse>`.
Undo/redo is a stack of inverses. The UI never holds `&mut` on the model.

Commands are where validation lives — duplicate individual address, group
address outside its `GroupRange`, link to a deleted object. Not the UI, not the
store.

Any command that changes a `Resolved<T>` sets its layer to `UserEdit`. The
exporter therefore knows what to write with no additional bookkeeping.

### 7.2 Saving

`knx-store` writes inside a SQLite transaction. Saving is incremental at entity
granularity. A crash leaves either the old or the new state, never a half-written
project. This is the reason the working file is SQLite rather than a directory
tree.

## 8. UI boundary

Tauri commands form a narrow, explicitly typed API. The UI requests projections
(`ProjectTree`, `DeviceList`, `GroupAddressTable`, `Inspector<T>`) and sends
`Command` values back.

Domain types are not mirrored one-to-one into TypeScript. Projections are shaped
for display and generated from Rust with `ts-rs` so they cannot drift. Large
tables are paginated and filtered on the Rust side rather than shipped whole
into the browser.

UI workarounds for domain problems are not acceptable; the fix belongs in the
layer that owns the problem.

## 9. KNXnet/IP (`knx-net`)

Own implementation against ISO 22510. Not a port of `xknx`.

Session 6 work, but the interface is fixed now: a `BusConnection` trait with
`discover`, `connect_tunnel`, `send`, `subscribe`. The bus monitor is a consumer
that resolves telegrams against the open project; the connection itself knows
nothing about projects.

`BusAccess` from `0.xml` — the ETS commissioning interface connection string —
is preserved verbatim and **not** translated into our own connection model
(RESEARCH §3.1). It is ETS tool configuration, not domain data.

Commissioning and download stay out of scope (RESEARCH §8.3). The architecture
does not block that path: load procedures, memory layout and mask data are
present in the product database.

## 10. Key material (`knx-secure`)

The crate exists from the first commit, with its own storage, even while empty.

Rules in force from now on: key material never enters the `Project` model, never
enters an `ImportReport`, never enters an export, never enters a log, and is
omitted by default from diagnostic dumps (RESEARCH §9). A test asserts that
`knx-secure` types do not implement `Serialize` toward report or export paths.

Retrofitting isolation is how secrets leak, which is why the boundary exists
before the feature does.

## 11. Test strategy

| Level | Content |
| --- | --- |
| Unit | Addresses, DPT parsing, override resolution, validation rules |
| Golden | Import of the reference project against the entity counts from RESEARCH §3: 36 devices including the unassigned one, 514 group addresses, 907 `ComObjectInstanceRef`, 1390 parameter values, 569/27 send/receive links |
| Oracle | Comparison against `xknxproject` output where it is not known to be lossy; every deviation must be explained |
| Roundtrip | The three guarantees of §6.6 |
| Migration | Every schema version has a frozen fixture that must keep loading |
| Malformed input | Broken ZIP, truncated XML, unknown schema, duplicate IDs, invalid addresses, dangling references, password-protected without password |
| License / layering | Dependency graph reaches no GPL crate; `knx-core` stays IO-free |

The golden numbers are reproducible independently via
`tools/inspect_knxproj.py`, which makes them a genuine oracle rather than a
self-generated expectation.

## 12. Session 1 deliverables

Documents and scaffolding only. No feature code.

- `docs/ARCHITECTURE.md`, `docs/DATA_MODEL.md`, `docs/IMPORT_EXPORT.md`,
  `docs/COMPATIBILITY.md`, `docs/KNOWN_LIMITATIONS.md`, `docs/ROADMAP.md`
- ADRs in `docs/adr/`:

  | ADR | Subject |
  | --- | --- |
  | 0001 | Technology stack: Rust core, Tauri, React, SQLite |
  | 0002 | Own `.knxproj` parser; `xknxproject` as test oracle only |
  | 0003 | SQLite as the native project format |
  | 0004 | Provenance and override-chain model |
  | 0005 | Separate, shared product database |
  | 0006 | Opaque passthrough store |
  | 0007 | Roundtrip fidelity definition |
  | 0008 | Key material isolation |
  | 0009 | UI boundary via generated projections |

- Cargo workspace with empty crates, `cargo-deny`, the `xtask` license and
  layering gate, and CI — so that the repository stays buildable from the start.
- `docs/IMPLEMENTATION_STATUS.md` updated.

## 13. Carried-forward open questions

Unchanged from RESEARCH §12; none of them block this architecture, and each has
a defined landing place in it.

| Question | Where it lands |
| --- | --- |
| ETS5/ETS6 schema deltas (13, 14, 20, 21+) | Tolerant parser reports unknown constructs (§6.2); per-version known-element lists |
| `Functions` element semantics | Domain model addition; absent from the reference sample |
| `when/@test` expression grammar | Session 4 spike, prerequisite for the parameter editor |
| Whether ETS re-imports an unsigned third-party `.knxproj` | Export warning stays until tested (§6.6) |
| Whether Data Secure runtime keys are readable from `.knxproj` | `knx-secure`, Session 7 or later |
| `.knxprod` encryption for master data scheme ≥ 12 | Product database ingest; out of v1 scope |
