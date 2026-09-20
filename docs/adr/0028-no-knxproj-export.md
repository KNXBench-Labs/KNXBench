# ADR 0028: KNXBench reads `.knxproj` and never writes one

Date: 2026-09-20

Status: Accepted

Session: 7 (T27 of the goal-completion run)

## Context

Session 3 built a `.knxproj` writer (`knx-etsproj::export`, schema 11 and
schema 21, retained-attribute replay, ~2160 lines) on top of the opaque
passthrough store (ADR-0006). ADR-0007 defined what a faithful round trip
would mean; ADR-0015 then dropped "ETS re-imports our output" as a goal but
explicitly declined to remove the writer, on the grounds that it was tested
code that cost nothing to keep.

That last part turned out to be untrue. The writer was never free: every
import-side change had to be mirrored in two schema writers, the retained-key
machinery existed to serve replay, KNOWN_LIMITATIONS.md carried export-only
gaps (§34 and relatives) that nobody was scheduled to close, and risk R9 —
whether ETS accepts an unsigned third-party file at all — stayed unanswered,
which meant the feature's value was never demonstrated in the first place.

The user settled it on 2026-09-20: *"drop export zu ets. das brauchen wir
nicht. einmal importiert bleibt es beim KNXBench file format."* Asked whether
to freeze the code or delete it: *"raus damit."* This is a product decision,
not a new technical finding.

## Decision

KNXBench imports `.knxproj` and never writes one. The project is imported
once and lives in KNXBench's own storage (SQLite, ADR-0003) from then on;
that store is the only project format the application writes.

No entry point may produce a `.knxproj`: no library function, no CLI
subcommand, no HTTP route, no UI control. `knx-etsproj::export`,
`knx-app::export_ets_project`, `POST /api/project/export`, the `knx export`
subcommand and the toolbar button are removed, along with `ExportWarning` and
its transport and UI representations.

Import is untouched by this decision. The opaque passthrough store keeps its
import side — its remaining value is as evidence of what the source file
said, which the import report and diagnostics use — and instance-exact
retained keys (`knx-etsproj::xpath`) stay for the same reason. Schema-≥21
communication-object flag resolution, which arrived with the export work, is
import-side and outlives it.

Other exports are unaffected because they are not `.knxproj` writing: CSV
group-address export (`/api/group-addresses/csv-export`), documentation
export (`/api/project/documentation-export`), the debug report and the
project diff all stay.

## Alternatives considered

**Keep the writer as a diagnostic convenience.** This was ADR-0015's choice
and it is what this ADR reverses. A feature nobody is allowed to verify, that
must be maintained in lockstep with the importer, and that ships with an
unsigned-output caveat and documented schema gaps, is a liability rather than
a convenience.

**Freeze the code but leave it compiled in.** Rejected by the user in as many
words. Frozen code is still code: it is still built, still tested, still in
the clippy gate, and still appears in the UI as a control someone will press.

**Move the exporter behind a feature flag.** Rejected as the worst of both —
the maintenance stays, the test coverage decays, and the documentation has to
describe a feature most builds do not have.

## Consequences

Import is one-way. There is no supported path from KNXBench back into ETS,
and the documentation says so plainly rather than describing a partial one.
Anyone who needs to hand a project to ETS keeps their original `.knxproj`;
KNXBench does not consume the original in place.

ADR-0007 is superseded: round-trip fidelity as defined there (semantic
equality of a re-imported export, opaque hash equality across a write,
exports are unsigned) describes an operation that no longer exists. Import
fidelity is now stated directly in `docs/IMPORT_EXPORT.md` in terms of what
import must preserve, with no write step in the loop.

ADR-0015 is superseded: its decision to retain the exporter is exactly what
changed. Its analysis of why ETS re-import was not worth chasing stands and
is the reasoning this ADR extends.

Risk R9 is closed as not applicable — no file is written, so no file has to
be accepted. Limitation entries that existed only because export was
incomplete are closed with the reason "export withdrawn 2026-09-20" rather
than deleted, so the record of what was once wrong survives the feature.

What has to be enforced: no new writer creeps back in under another name. A
search for `knxproj` in a write context, and for a `.knxproj` extension in a
file-save dialog, should find nothing. The file-open filter for `.knxproj`
stays, and only that.
