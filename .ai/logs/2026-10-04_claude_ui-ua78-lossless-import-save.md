# 2026-10-04 — Claude (goal-ui owner) — UA7/UA8: no silent loss between import, memory and `.knxdb`

## Why
UA5 found that `.knxdb` save collapsed a doubly placed device. After the
user said "keep on going", a probe checked the same class of defect more
broadly: build a project state, save, reopen, compare.

## Probe result (before the fix)
| State | Result |
| --- | --- |
| Duplicate area / GA / group range / building part id | saved, reopened lossy |
| Orphaned line (no area lists it) | saved, line gone |
| Range child listed twice; parent pointer without child entry | saved, reopened different |
| Device twice in one building part | raw SQL `UNIQUE` error |
| Dangling GA range pointer | SQL FK error (acceptable) |
| Device in parts of two installations | exact (only the counters were repaired because the probe used ids above them) |

## The reachable path
`apps/knx-cli/tests/cli_import.rs::a_report_with_real_errors_exits_two_not_zero`
imports an ETS file with a repeated GA `@Id`. Both group addresses got
internal id 2, so the old save silently kept one. With only the store check,
the CLI then failed at save (exit 1). Root fix in the importer (ADR-0073):
every element keeps its own id; references to the repeated id are reported
as `AmbiguousReference`. Checking that path also showed that schema ≥21
short `Links` ids were resolved document-wide with last-wins. A synthetic
two-installation project proved a silent cross-installation link:
`[[None], [Some(4000)]]` against the old mapper. Short ids now resolve
per installation, the scope validation already used.

## Changes
- `knx-etsproj`: `src/id_table.rs` (`IdTable`, `Reference`), `map.rs` uses it
  for all six kinds, `resolve_one` + `AmbiguousReference`, per-installation
  `short_group_address_ids`.
- `knx-store`: `src/representable.rs` (`check_representable`,
  `RepresentationIssue`, `EntityKind`), `StoreError::Unrepresentable`; the
  ADR-0071 ambiguity check moved into the same module. The unit test that
  pinned a dangling building-part parent as an SQL error now expects the
  earlier, named `HierarchyMismatch`; the FK deferral stays pinned by the
  dangling-range test.
- Docs: ADR-0074, ADR-0073, IMPORT_EXPORT (mapping scopes), KNOWN_LIMITATIONS
  (§2 bullet, U12 scope), IMPLEMENTATION_STATUS.

## Evidence
- RED: probe (7 lossy/raw-error states); old mapper fails the new
  duplicate-id test and the scope test; mutants 10/10 caught (7 store,
  3 importer including the CLI exit code).
- Corpus: 3 `.knxproj`, one installation each, no repeated ids or short-id
  collisions → import output unchanged.
- Gates: fmt, clippy -D warnings, layering, headers, anchors, corpus gates,
  diff-check green; workspace tests 162 blocks, 3,088 passed, 1 failed —
  `http_project_diff::a_knxproj_with_an_error_diagnostic_is_refused_with_its_report`
  pinned "1 error diagnostic"; the import now also reports the link it no
  longer guesses (2). Assertion updated with that reason; the test file
  re-run 13/13 green.
- Corpus-gated `--ignored` runs: knx-etsproj 55 (incl. 5 xknxproject oracle
  tests with the local `project_dump.json` linked in), knx-store 2,
  `save_load_roundtrip` 1, `open_reference_project` 1, `cli_import` 8 — all
  green.

## Open
Duplicate-id renumbering and hierarchy repair commands do not exist; such a
project cannot be saved natively until fixed elsewhere. Whether ETS repeats
`GA-<n>` across installations is unverified (no multi-installation schema
≥21 sample).
