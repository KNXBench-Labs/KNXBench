# ADR 0006: Opaque passthrough store

Date: 2026-09-02
Status: Accepted
Session: 1

## Context

A `.knxproj` contains material we cannot reproduce and should not interpret
(RESEARCH §7): signatures we have no key to regenerate, vendor plug-in DLLs
under `Baggages/`, per-device binary blobs, legacy ETS3 plug-in data, `Legacy*`
option flags and certification metadata.

There is also no authoritative public XSD for the format (RESEARCH §2.2, risk
R2). Unknown XML constructs are therefore the expected case when meeting an
unfamiliar schema version, not an exceptional one.

`CLAUDE.md` requires that information is never silently discarded.

## Decision

The project file carries an opaque store: `(source_path, kind, bytes, sha256)`.
Everything the model does not represent is retained there verbatim, including
unknown XML fragments, and written back unchanged on export.

Opaque bytes are never executed and never interpreted. Vendor DLLs in
particular are stored and copied, nothing more.

If the importer encounters something with neither a model representation nor an
opaque entry, that is a bug in the importer, and the import report is where it
becomes visible.

## Alternatives considered

**Discard what we do not model.** Violates the data integrity requirement, and
makes every round trip a slow leak of the user's project.

**Model everything.** Not possible without the specification and without
executing vendor code, and it would make an unknown construct a blocker rather
than a report entry.

## Consequences

Round trips survive constructs we do not understand.

The first ETS5 or ETS6 import produces a concrete list of unknowns instead of a
crash, which turns the missing-XSD risk into a work item.

Project files grow by the size of the retained data. Since the alternative is
losing it, this is the correct trade.
