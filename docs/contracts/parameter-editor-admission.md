# Specialized parameter editors — evidence-first admission (AP3 / PDB-04)

This is research/design, not editor implementation or compatibility acceptance.
Worktree `parameter-editors-20261011` starts from fetched main `f7a5d8dc`.
The AP2 assembled candidate stays frozen in its separate worktree.

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

No runtime changes, database migration, catalogue install or test execution by
this reconnaissance. PDB-04 remains undelivered; format facts above do not certify
manufacturer memory-image encoding or live device behavior.
