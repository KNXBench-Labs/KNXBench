# Native import-diagnostic availability and durable original reports

Local server/CLI/application/UI implementation, 2026-10-11. No domain/native-schema migration.

## Availability and provenance

`ProjectTree.import_report_available` retains its optional application meaning;
`import_report_historical` additionally marks recorded original findings restored
from native metadata. Historical findings are not current project compatibility,
manufacturer calculation execution, signature verification or hardware evidence.

| State | Available | Historical | Interpretation |
| --- | --- | --- | --- |
| External server import in this session | true | false | Original report and import counts, unchanged by ordinary edits |
| Newly created project | true | false | Known initial zero; no external import or full report exists |
| Native open with admitted report | true | true | Exact recorded original report and original category-row counters |
| Legacy/native file without report | false | false | Historical findings unavailable; zero is not a pass |
| Pure projection / older response | omitted | omitted | Producer asserts neither availability nor provenance |

Failed opening does not replace project, counters, provenance or saved bytes.
Save As does not turn live original-import evidence into a new assessment.
Native opening does not reimport source files, execute plug-ins or contact a bus.

## Durable envelope

[ADR-0113](../adr/0113-durable-native-import-evidence.md) defines one primary,
version-1 application-owned passthrough record. Its reserved kind/identity is not
an ETS member; similarly named source `File` rows are not interpreted. Scoped
source reports do not become a primary merged-project report by basename.

- Complete original `ImportReport` JSON is captured after server/CLI import/enrichment
  reporting and before replacement. Existing source passthrough and manufacturer
  manifest data are retained unchanged; the native metadata is one extra row.
- Envelope and report serialization are bounded to 16 MiB. Failed serialization
  refuses the project load explicitly rather than truncating the report. Existing
  native context/history bounds still apply; this is not a peak-RSS guarantee.
- Reader admits identity, unique primary record, SHA-256, exact envelope version,
  valid JSON/report shape and recognized severity. Ambiguous interpreted counter
  keys are refused, even when the stored byte hash matches.
- Source/historical JSON remains byte-exact through native Save/Open/Edit/Save,
  restart and Undo/Redo. Counts use the original import units: error-severity rows,
  warning-severity rows plus unknown/conflict/unsupported **rows**, not distinct
  losses or occurrence totals. Categories can overlap.
- Missing evidence is not reconstructed. Invalid/unsupported evidence refuses
  native opening untouched; it is not silently treated as empty or compatible.
- Hashes detect corruption, not deliberate forgery. Producer and source metadata
  remain recorded claims; no independent authenticity is inferred.

## Explicit read and local download

`GET /api/project/import-report` uses the existing authenticated project routes.
The response includes `available`, `historical`, `server_incarnation`,
`project_incarnation` and nullable **`report_json`**. The original JSON is a string
on the wire so browser number conversion cannot alter large integers or unknown
fields. A client may derive a display object, but exports embed the original text
verbatim rather than reserializing that derived object.

The Dashboard provides an explicit read action; there is no automatic full-report
request. Dashboard/Explorer mark restored counters as historical in EN/DE. The
panel rejects another-project replies and discards late reads after replacement,
while retaining immutable original evidence across ordinary same-project refreshes.

The JSON preview is explicitly limited to 65,536 characters and may end within a
value. The **full local download** is not truncated. Original source names,
attributes, hashes or other private data can occur in a report; the UI warns before
sharing. No automatic external transmission or anonymization is provided.

## Verification scope

Public witnesses are fictional. App controls cover exact native persistence,
legacy absence, identity/hash/duplicate/version/shape/severity/budget refusals and
ambiguous counters. Productive HTTP controls cover import/save/restart/edit/undo/
redo, full original-text equality, legacy availability, authenticated access and
byte/state preservation after a corrupt native open.

Client/panel controls cover explicit request ownership, malformed responses,
source binding, stale responses, same-project refreshes, provenance and large
integer fidelity. Built-bundle Chromium cases use strict intercepted APIs at
400/1440 px in EN/DE, independently read the actual downloaded file, retain all
1,001 fictional records and check overflow, unexpected requests and page errors.
They are browser evidence, not an ETS or private-project replay.

## Remaining boundaries

- This package captures **primary server imports and CLI `import --store`**.
  Source-scoped selective-import reports and library-only reads do not become a
  primary destination report. CLI stdout retains the source-only opaque count;
  the added native report record is separate metadata. No output file is opened
  before report capture succeeds. Existing `--replace` consent remains required.
- Original reports are not fresh merged-project diagnostics. Current-session
  operational logs remain separately bounded and are not a lifetime audit.
- Old native files cannot recover findings that were never stored.
- The storage schema does not need migration for this generic row; older readers
  can retain it without understanding its provenance. Complete older-reader UI
  diagnostic behavior is not claimed.
- Browser local download is tested; an independent native-shell save-dialog
  workflow is not part of this package.
- No support status, protocol behavior or device compatibility is promoted merely
  because a report is stored and readable. Remaining functional matrix stays open.
