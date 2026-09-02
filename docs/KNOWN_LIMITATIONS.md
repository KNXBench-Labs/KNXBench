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
project: schema 11, ETS 4.1.8 (risk R1).

**Cause.** No ETS5 or ETS6 sample project has been available.

**Impact.** Support for schema 12, 13, 14, 20 and 21+ is derived from
documentation and from reading `xknxproject`, not from evidence. A first import
of such a project will very likely produce unknown-construct entries.

**Lifted when.** Real ETS5 and ETS6 projects have been imported and their
unknown-construct reports reconciled to empty. This is a prerequisite for
Session 3 being able to claim more than schema 11.

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
