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
- Projection 84 with bindings export; clippy `-D warnings` for the four crates.
- Web: `tsc --noEmit` for all 28 configurations; Vitest four files, 171 passed.
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

## Limits

No folder recomputation, hiding or parameter-level diff; ETS rebuild behaviour
is not established. Module-instance arguments are not compared. Writing an
explicit value equal to an omitted product default counts as changed. No ETS
execution, hardware or bus action.
