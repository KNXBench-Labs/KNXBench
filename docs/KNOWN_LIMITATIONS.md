# Known limitations

Each entry states the limitation, its cause, what it costs the user, and the
condition under which it would be lifted. Nothing here is a defect to be fixed
by trying harder — these are consequences of evidence we do not have or of
decisions recorded in [docs/adr/](adr/).

The companion document is [COMPATIBILITY.md](COMPATIBILITY.md), which states
what is verified. Nothing may appear as verified there and as a limitation
here.

## 1. Single-sample bias

**Limitation.** Everything verified about the `.knxproj` format comes from one
installation: schema 11 (ETS 4.1.8) and schema 23 (ETS 6.3.7959.0) — the same
project, exported twice (risk R1).

**Cause.** No independent ETS5 or ETS6 sample project has been available. The
second export of the ETS4 reference project (RESEARCH §2.4/§3.3) confirms the
schema-11→23 format diff for this one installation, but says nothing about
schema 12, 13, 14, 20, 21, 22, and nothing about a differently-structured
project on schema 23 (e.g. one using `Functions`, KNX Secure, or multiple
areas/lines for real).

**Impact.** Support for schema 12, 13, 14, 20, 21 and 22 is still derived from
documentation and from reading `xknxproject`, not from evidence. Schema 23
support is derived from evidence but only from one project shape; a first
import of a structurally different schema-23 project will still likely
produce unknown-construct entries.

**Session 3 status.** `knx-etsproj`'s known-element table covers schema 11
only, built from this one reference project. Schema 23 is **detected and
refused by name** (`ImportFailure::NoKnownSchemaTable { version: 23 }`) —
never silently misread through the schema-11 table, which would undercount
communication objects by about 24% and misread every boolean flag as false
(RESEARCH §3.3). Building the schema-23 known-element table is Session 4
work, not attempted here.

**Lifted when.** Real ETS5 projects and further, independent ETS6 projects
have been imported and their unknown-construct reports reconciled to empty.
This is a prerequisite for claiming more than schema 11 and this one
schema-23 shape.

## 2. No authoritative XSD is publicly available

**Limitation.** Imports are tolerant, not schema-validating (risk R2).

**Cause.** The official schemas ship with the Manufacturer Tool via the KNX
GitLab account, which requires KNX membership [D].

**Impact.** We cannot tell "this file is invalid" from "this file uses
something we do not know". A malformed file may be read as far as it parses,
with the rest reported rather than rejected.

**Lifted when.** Authoritative schemas become available to the project. Note
that the tolerant parser would still be worth keeping — it is what turns a new
schema version into a report instead of a crash.

## 3. Device parameters are preserved but not interpreted

**Limitation.** All 1390 `ParameterInstanceRef` values in the reference project
are imported, stored and exported unchanged, but their meaning is not
evaluated. There is no parameter editor in v1 (risk R3).

**Cause.** Parameter visibility and semantics are driven by the `Dynamic` tree,
whose `choose`/`when` expression grammar is unresearched.

**Impact.** Device configuration must still be done in ETS. This application
will not corrupt parameter data, but it will not let you change it either.

**Lifted when.** The Session 4 research spike establishes the `when/@test`
grammar, and a parameter editor is built on top of it.

## 4. Round trips are semantic, not byte-exact

**Limitation.** An exported file is not byte-identical to the imported one
(risk R4).

**Cause.** Signatures cannot be regenerated, attribute ordering is not
guaranteed stable, and ETS assigns internal identifiers.

**Impact.** Comparing an export against the original with `cmp` will show
differences. That is expected and is not evidence of data loss.

**Lifted when.** Never — this one is structural. What replaces it are the three
guarantees in [IMPORT_EXPORT.md](IMPORT_EXPORT.md): semantic model equality,
hash equality of all opaque bytes, and an explicit unsigned-export statement.
See [ADR-0007](adr/0007-roundtrip-fidelity.md).

## 5. Exports are unsigned, and ETS acceptance is untested

**Limitation.** Every file this application writes is unsigned, and whether ETS
re-imports it is unknown (risk R9).

**Cause.** Signatures are RSA over manufacturer and project data; the signing
keys are KNX's.

**Impact.** An export may or may not open in ETS. The application says so at
export time rather than implying it will work.

**Session 3 status.** Every export carries `ExportWarning::Unsigned` — always
constructed before anything else can fail, so no export is produced without
it. Every `*.signature` entry (one per manufacturer plus one for the
project — five in the reference project) is copied through unchanged and
reported as `ExportWarning::StaleSignature { source_path }` per entry: it
no longer matches the content it signs, since it cannot be regenerated
without KNX's signing keys.

**Lifted when.** Someone exports a project and opens it in a real ETS
installation. Either outcome is useful: if ETS rejects it, the export is
documented as one-way, which is a different product decision than a bug.

## 6. Devices behind vendor plug-in DLLs

**Limitation.** Devices whose configuration depends on a vendor plug-in DLL
cannot be configured by this application (risk R5).

**Cause.** The configuration logic lives inside a Windows PE binary shipped in
the project file — for example `econEts3.dll` (641 KB) and `FastDownload.dll`
(160 KB) in the reference project [V].

**Impact.** Affected devices are detected, marked read-only, and reported in
the import report's `unsupported` list. Their data round-trips unchanged; it
simply cannot be edited here.

**Lifted when.** Never by us. Executing vendor binaries is not something this
application does, on any platform.

## 7. Commissioning and device download are out of scope

**Limitation.** The application does not program devices (RESEARCH §8.3).

**Cause.** Bricking risk on real hardware, an undocumented `Legacy*`
compatibility matrix, and vendor DLL involvement in download procedures.

**Impact.** Planning and documentation happen here; downloading happens in ETS.

**Lifted when.** A deliberate decision to take it on, with hardware to test
against. The architecture does not block it: load procedures, memory layout and
mask data are all present in the product database.

## 8. KNX Secure is not implemented

**Limitation.** No Data Secure, no IP Secure, no keyring handling (RESEARCH
§9).

**Cause.** No sample key material was available in Session 0, so nothing about
it could be verified.

**Impact.** Secured installations cannot be fully represented or monitored.

**Lifted when.** Sample material and a real secured installation are available.
The isolation boundary already exists — `knx-secure` is a separate crate with
no dependency on `knx-core` — precisely so that this can be built without
retrofitting secret handling into the model
([ADR-0008](adr/0008-key-material-isolation.md)).

## 9. Project files are not diffable

**Limitation.** A project is a SQLite file, so version control tools cannot
show a meaningful diff of it.

**Cause.** A deliberate trade for transactional, incremental saving and indexed
access ([ADR-0003](adr/0003-sqlite-project-format.md)).

**Impact.** Projects can be versioned as binaries only. Reviewing what changed
between two versions requires the application.

**Lifted when.** A textual export and import format is added, if a demonstrated
need arises. It is deliberately not built speculatively.

## 10. The project licence is not decided

**Limitation.** The Cargo workspace declares `AGPL-3.0-or-later` as a
placeholder. This is not a decision.

**Cause.** The licence has not been chosen yet.

**Impact.** No practical impact today, since nothing is distributed. It must be
settled before any release.

**Lifted when.** The licence is chosen and the workspace `license` field is
updated. Whatever it becomes, it must remain consistent with the constraint
that no GPL crate enters the runtime graph — that constraint is about *incoming*
dependencies and is independent of our own licence
([ADR-0002](adr/0002-own-knxproj-parser.md)).

## 11. `.knxprod` files for master data scheme ≥ 12 cannot be imported directly

**Limitation.** Manufacturer product files in the `.knxprod` container are only
readable for master data scheme 11.

**Cause.** `.knxprod` is the same XML family as `.knxproj`, but newer master
data schemes add an encryption or obfuscation layer that Session 0 research did
not establish (RESEARCH §10) [D, open].

**Impact.** Product data for newer devices has to reach the product database by
another route — in practice, from a `.knxproj` that already contains the
application programs it references.

**Lifted when.** The container layer for scheme ≥ 12 is understood, or an
official route to that data becomes available. Out of v1 scope either way.

## 12. Manufacturer application program data lives in the opaque store, not the product database

**Limitation.** Parameter and communication-object semantics (type, range,
unit, translations) that would come from an application program are not
yet available. `ProductRefId` and `Hardware2ProgramRefId` are preserved as
opaque strings on every `DeviceInstance`; nothing resolves them.

**Cause.** [ADR-0005](adr/0005-separate-product-database.md)'s shared product
database is Session 4 work. Until it exists, `<M-xxxx>/*` (catalog,
hardware, application program XML, and vendor baggage) is retained in the
per-project opaque store instead — one full copy per project, not the
target design's one copy shared across every project that references it.

**Impact.** A project opens completely and round-trips its manufacturer
data byte-for-byte, but nothing in it is interpreted: no parameter
metadata, no DPT catalogue resolution beyond what a `DptRef` string already
encodes, no application-program-level defaults for communication object
properties. `ComObjectInstance` values in v1 carry only the `Instance`
layer; `Program`/`ProgramRef` never appear.

**Lifted when.** Session 4 builds the product database and its ingest path
(IMPORT_EXPORT §10). Existing projects keep opening in the meantime — a
project must never depend on the presence of manufacturer data.

## 13. Password-protected projects are refused, not decrypted

**Limitation.** A `.knxproj` whose project part is nested as `<P-xxxx>.zip`
(IMPORT_EXPORT §2) is detected and named
(`ContainerError::PasswordProtected`), but the file is never opened.

**Cause.** Both decryption schemes (ZipCrypto for schema < 21, AES/PBKDF2
for schema ≥ 21) are documented from `xknxproject` source but unverified
against a real protected project — the reference project is unprotected.
Shipping an untested decryption path would claim support this repository
cannot demonstrate.

**Impact.** A protected project cannot be imported at all today, by design
rather than by omission: refusing cleanly is preferred over a decryption
path nobody has run against a real encrypted file.

**Lifted when.** A real password-protected ETS4/5 project (ZipCrypto) and a
real password-protected ETS6 project (AES) are available to verify each
scheme against.

## 14. The project's default language is a placeholder

**Limitation.** Every imported project is created with
`Language("en")` as its `StringTable`'s default language, regardless of the
language the project was actually authored in.

**Cause.** A schema-11 project file carries no project-wide language tag:
`DefaultLanguage` belongs to an application program's `RegistrationInfo`
(RESEARCH §4.1), not to `ProjectInformation`. Session 3 imports no
application program, so there is nothing in the imported data to derive a
real value from, and inventing one from, say, the project name would be a
guess presented as a fact.

**Impact.** Nothing observable in Session 3: instance-level `@Text` and
`@Description` are literal, not translated, so every `Text` this importer
produces is `Text::Literal` and resolves without consulting the table at
all. The default language only starts to matter once localized program
text exists. Export and semantic comparison both ask the project's own
`StringTable::default_language()` rather than naming a language
themselves, so when a real value arrives there is exactly one place that
sets it.

**Lifted when.** Session 4 ingests application programs and their
`RegistrationInfo`, giving the importer a measured language to set instead
of a placeholder.

## 15. Unparsable values survive only on `Override` fields

**Limitation.** A present attribute whose value cannot be parsed keeps its
raw text — and is written back verbatim on export — only where the field
is modelled as `Override<T>` (`Override::Malformed`, ADR-0010's
amendment). On a field modelled as a bare value or an `Option<T>`, an
unparsable value falls back to the type's default and only the report
records what the source actually said.

**Cause.** `Override<T>` exists to carry an attribute's presence state, so
a fourth state costs nothing structurally. A bare `u8` or an
`Option<DateTime<Utc>>` has nowhere to put a string, and widening every
such field would push presence bookkeeping into parts of the model that do
not otherwise need it.

**Impact.** For the affected fields (timestamps, individual addresses,
numeric ids, enums such as `CompletionStatus`) a malformed source value is
reported but not written back: the export is a correct file that differs
from the original in exactly that attribute. No such value occurs in the
reference project — the case is reachable only with hand-broken input.

**Lifted when.** A real project is found that carries unparsable values on
those fields, making the added model surface worth its cost. Until then
the asymmetry is deliberate, documented, and reported at import time.
