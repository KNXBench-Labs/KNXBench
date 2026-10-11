# Specialized parameter editors — evidence-first admission (AP3 / PDB-04)

This began as research/design on fetched main `f7a5d8dc`. The first read-only
source-evidence slice is now implemented on published AP2/native13 main; it is
not editor implementation, PDB-04 completion or compatibility acceptance.
AP2 was published separately; its earlier source-bound receipts stay historical.

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
