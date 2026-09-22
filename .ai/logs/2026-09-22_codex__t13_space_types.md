# T13 Task 3 — complete documented Space types

Status: DONE. All task acceptance conditions implemented; controller review pending.

## Scope and evidence

Added exact `BuildingPartType` variants Stairway, RoomPart, Area, Ground and
Segment. Extended parser, native codec, projection, creation API and existing
UI kind maps/list with EN/DE translations. Mapper keeps its explicit unknown
ETS problem/fallback. Native unknown kinds now return
`StoreError::UnknownBuildingPartType(String)` rather than Building.
Schema remains v9; migration files are unchanged.

Personally examined the local PDF:
`/mnt/daten-i/Sourcecode/knx-spec-kb/sources/The KNX Standard v3.0.0/Project Schema23 v01.00.00.pdf`.
§1.1.2.3, page 7, lists ten values including Segment but not RoomPart.
§§1.2.6.3–1.2.6.4, pages 54–55, describe Space; the Type attribute table
explicitly names BOTH RoomPart and Segment (the latter continues on the next
line). Only RoomPart differs. The parent approved correcting the mistaken
design sentence. Spec, DATA_MODEL, IMPORT_EXPORT, COMPATIBILITY and §89 now
reflect that exact evidence. No new real-corpus compatibility is claimed.
ADR-0028's absence of ETS export is respected.

## TDD evidence

Environment for Rust commands:
`CARGO_TARGET_DIR=/var/tmp/knxbench-t13-target CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`.

RED, before production changes:

- `cargo test -p knx-etsproj building_part_type`: 1 pass / 1 fail,
  UnknownEnumValue for Stairway.
- `cargo test -p knx-etsproj --test golden_reference_project space_type`:
  0 pass / 1 fail, Stairway emitted type-related MapProblem.
- `cargo test -p knx-store building_unknown`: 0 pass / 1 fail,
  load unexpectedly succeeded with kind Building for stored FutureSpace.
- `cargo test -p knx-store building_documented`: 0 pass / 1 fail,
  additional type was rejected before native persistence.
- `cargo test -p knx-server --test http_edit_routes building_part_creation_preserves`:
  0 pass / 1 fail, HTTP 400 instead of 200 for Stairway.
- `npm test -- --configLoader runner src/ProjectExplorer.test.tsx`:
  32 pass / 5 fail, four raw German labels and absent Segment creation option.
- `npm test -- --configLoader runner src/Inspector.test.tsx`:
  6 pass / 4 fail, raw kinds instead of German labels (Segment is identical
  in both languages and was already a passing case).

GREEN and final verification:

- Focused building tests in knx-core/store/projection/report: 29/29 passed.
- Focused synthetic schema-23 import/map/report: 1/1 passed.
- Focused frontend Inspector/ProjectExplorer/i18n: 67/67 passed.
- `cargo test --workspace`: 1,969 passed, 0 failed, 5 ignored,
  92 result blocks; complete log `/var/tmp/knxbench-t13-task3-rust.log`.
- `npx tsc --noEmit`: passed.
- `npm test -- --configLoader runner`: 926/926 passed, 63 files;
  complete log `/var/tmp/knxbench-t13-task3-web.log`.
- `cargo clippy -p knx-core -p knx-etsproj -p knx-store -p knx-projection -p knx-server -p knx-report --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `cargo run -p xtask -- check-layering`, `check-headers`, and
  `check-anchors`: passed (194 headers / 162 grandfathered; 389 links across
  184 Markdown files, no dead links).

During test construction, corrected unresolved import/helper names, the
Inspector selection discriminant, and a missing filename argument on the
full-pipeline import test. The first full Rust attempt stopped at that compile
error; the subsequent complete run above passed. Early failing UI tests
printed act-environment warnings during failed-test cleanup; final runs did not.

## Coverage and self-review

Synthetic schema-23 ZIP import and direct mapper exercise all five values and
FutureSpace; report errors retain the unknown token. Native load/save/re-save
compares the whole project/hierarchy and literal SQLite kinds. API creation
returns each exact kind. Projection tests verify actual tree nodes. UI tests
select each localized option, submit it, and verify the exact raw creation
argument; Inspector and catalogues verify localization. Report HTML
distinguishes all five kinds from BuildingPart.

Reviewed the diff for exhaustive kind mappings, no silent storage default,
unchanged schema/migrations, no themes or group-address notation changes,
no ETS exporter revival, and no LIMITATION_TRIAGE edits.
Tasks 1/2 changes remain intact. No KNX/LAN/multicast/gateway/hardware traffic,
no new private LAN literals, no prohibited fixture, no subagents and no push.

Remaining evidence limits: none of the three local reference projects contains
these five types; RoomPart is absent from the documented enumeration despite
appearing in the Type attribute table. Native schema stays v9, so older
binaries with the six-kind decoder can still coarsen these values on reopening.
This pre-existing reader limitation is documented rather than hidden.

## Files changed

- Rust: knx-core building; knx-etsproj values/map and golden reference test;
  knx-store building/lib/project; knx-projection lib; knx-report render;
  knx-server domain and HTTP edit tests.
- Web: Inspector/ProjectExplorer and tests; i18n tests; EN/DE catalogues.
- Docs: DATA_MODEL, IMPORT_EXPORT, COMPATIBILITY, KNOWN_LIMITATIONS §89,
  IMPLEMENTATION_STATUS and the approved T13 design evidence correction.
- Handover: .ai/CURRENT_STATE.md and this durable log; detailed report retained
  under the ignored .superpowers task directory.
