# Saved folder currency — 7 November 2026

Scope: [saved folder currency contract](../contracts/saved-folder-currency.md),
WP-06 open item 3. Owner decision 7 November 2026: conservative marker against
the unchanged original XML, no new storage.

## Delivered

- `knx_etsproj::device_baseline::read_device_baseline`: one device's original
  `(RefId, Value)` parameters and `GroupObjectTree` object list, with the
  importer's schema-20/21/23 parser and normalisation.
- `knx_app::saved_folder_currency`: order-insensitive comparison against the
  checksum-verified retained `ContainerEntry`; unknown with a named reason when
  the original cannot be attributed or read.
- Server: `DeviceDetail.project_folder_currency`, computed only when a saved
  Folder is placed and set only when one is still shown after the inactive
  refusal. Folder placements are unchanged.
- Web: one notice above the object table for changed/unknown (`FolderCurrency.tsx`),
  English and German.

## Executed checks (author, before review)

- Focused Rust: device baseline 2, comparison 3, server folder lib 5,
  `http_com_object_folders` integration 9 (two new mounted tests: real import,
  description edit, `SetParameterValue` overwrite/undo, value restored to its
  original text, new entry, native save/reopen, missing and tampered original).
- Projection 86 with bindings export (two new generated-binding tests); clippy `-D warnings` for the four crates.
- Web: `tsc --noEmit` for all 29 configurations; Vitest four files, 171 passed.
- Author mutants, each killed with sources restored by SHA-256: no sorting, no
  checksum check, objects ignored, unknown reported as unchanged.
- Cost (release build, retained private corpus `0.xml` members, seven runs):
  checksum plus baseline read median 9.3 ms / max 11.6 ms for 806,451 bytes,
  4.0 ms for about 338 KB, 0.4 ms for 29,681 bytes. Paid once per device-detail
  request that shows saved folders.
- Real-data witness (temporary uncommitted probe, release build, offline
  namespace): six of seven private demo projects imported; directly after
  import all 185 devices of the five schema-21/23 projects report `Unchanged`,
  and all 36 devices of the schema-11 project report `UnsupportedSchema`
  (schema 11 has no saved folders, so no marker is shown there). This checks
  that the comparison agrees with the importer's own mapping on real exports;
  it is not an edit witness on real data.

## Independent review and corrections

First verdict on `1b85f4df1`: **BLOCK** for missing tests only; no product defect
found. The reviewer confirmed importer parity with a probe over schemas 20, 21 and
23 (duplicated parameter, missing `Value`, escaped value, module and override-only
objects, duplicated tree reference, two devices) and ran the eight object-table
browser specs: default 197, communication 104, table-columns 176, devices 33 and
issue09 14 passed. A temporary reviewer spec over three themes, two languages and
two widths showed the notice without horizontal overflow (36 passed); it was not
committed.

- IMPORTANT: mutants M4 (all devices' parameters) and M9 (first installation only)
  survived. New two-installation, two-device import test kills both.
- MINOR: M5 (no shown-folder gate) and M3 (shared source never detected) survived;
  dedicated tests now kill both.
- MINOR: each request hashed and re-parsed the whole member (reviewer: 23 / 214 /
  655 ms for 1.4 / 14.3 / 43 MB). A session-only memo of the last parsed member,
  keyed by the verified digest, source path and schema, now parses once. Release
  measurement on the same synthetic members: first request 24 / 242 / 708 ms,
  later requests 0.6 / 6.1 / 18.0 ms median (checksum only), one parse each. A
  tampered member under the old digest is still refused, and a changed member is
  re-parsed (tested, and mutant M10 "ignore the key" is killed).
- MINOR: contract wording (shown-folder condition, notice text), counts in this
  record, and a schema-20 line-device reader test.

## Acceptance and publication

Feature `1b85f4df1` plus review fix `424efce07` (focused re-review APPROVE:
earlier survivors M3/M4/M5/M9 and new M10–M12 killed), merged main-first as
`b33f95145` on `b111a7eed` (tree identical to the branch head). The maintained
change-scoped package gate ran on that merge: all 17 selected stages exited 0
with valid proofs, inputs frozen, no user data touched or unchecked. Counts:
Rust 4,470 passed / 0 failed / 206 ignored (32 binaries), bindings 86, private
corpus 149/149, web gate 34 stages including 1,120 browser cases. Pushed after
the gate with `origin/main` unchanged; local, tracking and live `main` read back
as `b33f95145`.

## Limits

No folder recomputation, hiding or parameter-level diff; ETS rebuild behaviour
is not established. Module-instance arguments are not compared. Writing an
explicit value equal to an omitted product default counts as changed. No ETS
execution, hardware or bus action.
