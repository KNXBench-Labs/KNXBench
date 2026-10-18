# Specialized parameter editors — evidence-first admission (AP3 / PDB-04)

This began as research/design on fetched main `f7a5d8dc`. The first read-only
source-evidence slice is now implemented on published AP2/native13 main; it is
not editor implementation, PDB-04 completion or compatibility acceptance.
AP2 was published separately; its earlier source-bound receipts stay historical.

## Packed time units fall back, 2026-10-18

Census G6 ([gap census](../research/2026-10-17-product-database-gap-census.md)):
`time-count` labelled every `TypeTime` as a decimal count of its declared unit,
also when the unit is packed. A read-only count over the local collection
(18 October 2026, member bodies deduplicated, values allowlisted, no names or
ids) found 1,456 `TypeTime` declarations, all with a `Unit`: plain `Seconds` 551,
`Milliseconds` 250, `HundredMilliseconds` 93, `Minutes` 90, `Hours` 10,
`TenMilliseconds` 10, and packed `PackedSecondsAndMilliseconds` 443,
`PackedMinutesSecondsAndMilliseconds` 5, `PackedDaysHoursMinutesAndSeconds` 4
(79 programs). A packed unit combines several fields in one integer whose layout
no local document defines.

`chooseParameterEditor` now admits `time-count` only for the six plain units
observed (and for an undeclared unit, shown as "not declared"); every other unit,
the packed ones included, gets the existing `unsupported-codec` fallback: the
original value is shown unconverted and no count input is offered. The server's
validation and write path are unchanged. Tests: `plan.test.ts` (six plain units,
three packed, an unknown and an empty unit, undeclared unit) and
`ParameterEditor.test.tsx` (packed value shown, no input, no write).

## Public contract follow-up, 2026-10-12

[Inspected source/access and module-scope qualification](../research/product-database.md#2026-10-12--ap3-colourpicture-contract-and-module-scope-follow-up)
records actual public bodies and the bound type-definition login redirect.
No Color/Picture codec or module mapping is admitted; the explicit fallback and
existing source/validation/persistence authority remain unchanged. Missing
authorized type definitions are an external input, not a completed editor.

## Verified source boundary, 2026-10-11

The KNX Association's Project Schema23 v01.00.00, section1.1.3.19 on printed
pages30–31, specifies decimal-string numeric values and applies the same value
encoding to TypeTime. The table was read as text and visually checked. It does
not establish time units, a clock/duration presentation, conversion factors or
precision rules. Searches within this specific PDF did not find TypeColor,
TypePicture or Slider; that is not proof that their definitions do not exist in
manufacturer schemas or another document. Tag/@Color's hexadecimal pattern is
for project tags, not evidence for parameter-color encoding.

Primary source: [KNX Project Schema23](https://support.knx.org/hc/nl/article_attachments/17389755651474),
section1.1.3.19, pp.30–31; project-tag color is a separately scoped declaration.
No SDK/member access, manufacturer execution, hardware or private data required.

## Actual current implementation

The dedicated product database records parameter kind, size, number subtype and
bounds. `crates/knx-productdb/src/parse/program.rs` admits TypeTime with integer
bounds; Unit/UIHint remain reported unknown attributes, not editor metadata.
TypeColor/Space has no admitted value codec; Picture likewise has no type
attributes admitted by the current parser. Whole source blobs/unknown records
are retained, not dropped. A generic unknown sample is not unique per-type
editor authority.

`apps/knx-server/src/routes.rs` ParameterFieldDto exposes kind, bounds, options,
value and exact write authority, but no time-unit, slider, color-space or picture
contract. `ParameterPanel.tsx` is currently generic presentation on the existing
application validation/write response, and belongs to the shell owner for mount
changes. Source retention alone is not a typed value-validation contract.

## Smallest safe next design

1. Declare exact dedicated product-layer/source-evidence and isolated feature
   paths before coding. Do not add manufacturer-name heuristics or infer hints
   from parameter labels. A documented numeric Time subset can preserve decimal
   strings without guessing units; any conversion needs separately verified data.
2. Recover exact per-type attributes through the versioned product layer,
   preserving source identity and ambiguous/contradictory/unknown declarations.
   Never use ingest_unknown's aggregate sample as an individual field's truth.
3. Provide a safe explicit fallback when units, codec, options or numeric
   bounds/precision cannot be verified. Do not enable a color/image upload simply
   because a browser has that input control.
4. Reuse existing parameter validation/dependencies/write authority and native
   persistence. Isolated UI gets a host contract; no shell/global copy/style
   change without the single owner's integration.
5. Tests must cover exact attribute admission, conflicting hints, malformed
   bounds, lossless lexical values, rejected changes, dependency effects and
   Save/Reopen. Unknown evidence is not a successful specialized editor.

The initial reconnaissance changed no runtime or database. The subsequent
read-only helper changes neither schema, normalized/source data, catalogue nor
project mappings. PDB-04 remains undelivered; neither raw source attributes nor
these tests certify manufacturer memory-image encoding or live device behavior.

## First implementation slice: exact read-only source evidence

Owned paths before this slice: `crates/knx-productdb/src/parameter_editor.rs`,
`crates/knx-productdb/src/lib.rs` (one module export),
`crates/knx-productdb/tests/parameter_editor.rs`, this contract and owned handover.
No schema/parser/global UI/manifest/shared shell changes. Read metadata from the
exact program-owned blob and exact parameter-ref/type join, verifying identity
and model agreement; do not select from aggregate unknown samples or names.
Read byte length before allocating, then bounded UTF-8 XML with exact supported
namespace and structural owner, rejecting DTD/ambiguous declarations. Missing,
oversized, invalid or contradictory evidence is a named fallback; SQLite errors
remain real errors. Every raw type attribute is retained in the evidence result,
not reclassified as a validated encoding. Parent hints are separately preserved.
Source/normalized rows, language overlays and project mappings never change.

The public parameter reconstruction article is accessible anonymously through
the official Help Center API. A separately requested article returned401; its
body/title/semantics are not verified here. No account activation/login or
protected-document bypass attempted.
Schema23 only proves Time has Number-like values, not duration arithmetic.
Color Space/Picture RefId/Number Increment are raw evidence until their specific
use is documented and tested; no manufacturer/name heuristics.

## Initial slice acceptance and limitations

Frozen-input checks:17 synthetic exact-source tests within831 ProductDB passes,
25 ignored explicitly excluded; strict all-target Clippy/format/docs and five
explicit-root repository gates passed. Actual close/read-only-reopen preserves
source evidence and database file bytes. A named source-reference RED established
that a changed normalized ref must not borrow another valid source type's hints.
Initial compiler setup refusals remain separate from behavioral REDs.

Only direct program Static declarations in project namespaces20–23 are read.
Module-instance/definition scope needs separately audited mapping and falls back.
Limits:8MiB source (length queried before blob allocation),100000 events,64 levels,
64 attributes per element,4096 bytes per attribute name/value. DTD/non-UTF8/foreign
root namespaces, duplicate owners/types/refs or mismatched normalized/source
ref→parameter→type chains do not authorize specialized controls. Qualified raw
attributes retain their names and must not be mistaken for unqualified hints.
There is no resource loading, HTML rendering, codec, unit conversion, catalogue
write, parameter mutation, API DTO or production-shell mount in this slice.

## Next owned slice: isolated editor admission and controls

Before coding, own `apps/knx-web/src/features/parameter-editors/plan.ts`,
`plan.test.ts`, `ParameterEditor.tsx`, `ParameterEditor.test.tsx`, `copy.ts` and
`editor.module.css`, this contract and own handover. No ParameterPanel/api/global
styles/catalogues/manifests/shell mutation. Initially pass exact evidence from a
typed host adapter; producer HTTP adoption is a separate declared step.

The selector must not turn raw evidence into a guessed codec. Admitted Time is
a decimal integer count only, no h:m:s/date conversion; raw unit is visibly
verbatim/uninterpreted. Numeric Slider is a bounded safe integer presentation
only for an explicit unqualified Slider hint and valid integer subtype/bounds;
unsupported Increment or conflicting hints fall back, rather than rounding.
Source lexical strings are not rewritten on selection/render or a no-op.
Color/Picture receive their own visible unsupported-codec fallback, not an
enabled browser colour/file control. Current host write authority and existing
application validation/dependency/persistence remain authoritative.

Only explicit user Apply invokes the supplied existing-service callback. Draft
context changes revoke stale results; rejected writes retain a recoverable draft
and visible refusal. Host must refresh the actual panel/tree after success and
provide module/source/revision identity; no local reconstruction of dependencies.

## Read-only HTTP producer and isolated fixture ownership

Next exact owned paths: `apps/knx-server/src/parameter_editor_routes.rs`,
server `lib.rs` (one module/authorized-router merge),
`apps/knx-server/tests/http_parameter_editor_source.rs`,
`apps/knx-web/src/features/parameter-editors/fixture.html`, `fixture.tsx`,
`build-fixture.mjs`, plus this contract/owned handover/status. Existing parameter
routes/domain/write DTOs, global styles and shell remain unchanged.

GET `/api/device/{id}/parameter-editor-source?etsId=...` will resolve only the
device's existing Hardware2Program mapping; no client-supplied program authority.
The response binds device/ref/server/project/snapshot identity and named source
evidence. Project and ProductDB mutexes must never be held together: copy the
program reference and project identity, read ProductDB, then recheck project
identity/reference and refuse concurrent drift. Optional unavailable sources stay
200 with explicit status; actual SQL errors remain500. No new write endpoint or
revision-conditional mutation claim. The reused legacy parameter write endpoint
remains the authority; UI stale-result suppression cannot roll back a sent edit.

The isolated fixture may use clearly synthetic source/value/service results for
component browser acceptance. It must not be presented as real persistence or
API acceptance; HTTP tests separately exercise real handlers/native save/reopen.
No API/bus/network resources may escape the loopback request allowlist.

Owned documentation closure additionally includes `docs/contracts/parameter-editor-host.md`,
`docs/KNOWN_LIMITATIONS.md`, `docs/IMPLEMENTATION_STATUS.md`, and
`docs/evidence/parameter-editors-2026-10-11.json`. These record the bounded
controls and independent evidence scopes; PDB-04 is not closed by this subset.

Metadata-only closure also owns the exact PDB-04 row annotation in
`docs/status/LEDGER.md` and an additive AP3 boundary in `docs/OPEN_WORK.md`.
The PDB-04 status stays LATER; no other row/owner/status/count changes.
