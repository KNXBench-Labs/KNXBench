# Limitations: current-summary reconciliation (20 October 2026)

## Scope and evidence

The 19 October reassessment read all 195 catalogue dossiers at `2ff5b370`
and traced relevant production consumers and test sources. This documentation
package starts from `dc83e939`, after WP-17, and rechecks the affected paths.
It does not implement another feature or change formal ledger dispositions.
The canonical [ledger](LEDGER.md) remains the sole status record.

Eight current summaries needed correction; older retained accounts are not
new implementation tasks. Their hash-pinned blocks, source IDs and existing
anchors remain intact. The overview is regenerated from the authored
[catalogue](../limitations/catalogue.json), not hand-edited.

## Corrections

| Current-reading route | Correct boundary and evidence |
| --- | --- |
| [Parameter presentation](../limitations/KL-003-parameter-evaluation-and-editing-boundaries.md) | Supported block/separator captions resolve; row/column references remain raw. The server's `parameter_captions` and `parameter_grid` paths are distinct. `SuffixText` is delivered, not a missing column/display task. |
| [Build identity](../limitations/KL-065-version-identity-and-dirty-builds.md) | Development stamping has no dirty suffix. `knx-build-stamp` rejects reported changes/commit mismatch when the clean-tree release mode is enabled; a version string does not prove that mode was used. |
| [Archive budgets](../limitations/KL-159-project-archive-budgets-and-duplicate-names.md) | Shared limits are 1,000,000,000 bytes per member and 4 GiB expanded total, not 64/1024 MiB. Integrity checks and nested accounting remain; the bounds are not a peak-memory guarantee. |
| [Shutdown](../limitations/KL-163-server-shutdown-and-active-writes.md) | `StopAdmission` refuses new tunnel openers; existing-session group writes and discovery are outside that check. Bounded draining is not safe device-side abort/recovery. |
| [Desktop downloads](../limitations/KL-182-desktop-silent-downloads.md) | The staged-download/save-dialog repair shipped in Alpha 12, as its release notes/publication receipt record. Older installed assets and untested native paths are not repaired by this prose. |
| [Native portability](../limitations/KL-183-knxdb-manufacturer-files.md) | Available hash-matched saved-manifest files are embedded, not automatically every referenced application. Save can succeed without unavailable bytes. The additional catalogue-created-device case is source-traced, not runtime-reproduced. |
| [Diagnostic profiles](../limitations/KL-184-device-diagnostic-export.md) | Generic/Web/HTTP remains identity-only; CLI also has the ProductDB-derived, identity-gated format-2 memory profile. Its offline/simulator evidence does not inherit the generic profile's 46-device hardware observations or establish backup/restore authority. |
| [Installation rename](../limitations/installation-node-editing.md) | The actual inspector uses `RenameNameField` → `api.renameEntity` with expected name, snapshot revision and server/project incarnations. This does not remove legacy unbound API support or provide general installation management. |

The [motion dossier](../limitations/KL-043-motion-controls-and-residual-surfaces.md)
also names the fourth category, structure lines (`motionLines`), alongside
Toasts, Achievements and Flow. Its broad current summary remains accurate.

## Portability: confirmed scope versus follow-up hypothesis

`manufacturer_files::embed_into` visits saved manifest entries only. Missing
bytes are an explicit unavailable result; the existing HTTP test source
`a_save_that_cannot_carry_a_file_says_so` expects successful Save with zero
carried files. Consequently, "fixed for new saves" was overbroad.

The additional catalogue path was independently source-traced: New starts
with an empty manufacturer manifest, catalogue creation updates the project
and command stack without extending it, and the native snapshot copies that
manifest. It is **not a newly executed reproduction**. The remaining witness
is catalogue device create → Save → reopen with an empty product database.
No repair or runtime-confirmed outcome for that scenario is claimed here.

The device/product guide, Docker/data-volume chapter, capability table and
practical known-issues page now keep this scope instead of promising universal
self-contained saves. Original packages and independent backups remain needed.
README and FAQ have no matching self-contained-save promise to change; the
technical import/export page is clarified without rewriting its dated history.

## Validation boundary

The preceding audit actually ran 385 targeted Web unit tests in 12 files on
`2ff5b370` (296 + 89, both exit 0). That is retained evidence, not a new run on
this documentation candidate. Its other cited Rust/test paths were inspected,
not executed. Fresh acceptance on main-first merge `52a42218`: all 13 planned
scoped-gate stages passed with frozen inputs, including 317 tool tests, 103
xtask tests, Clippy, all 1,636 anchors across 680 Markdown files and 208 ledger
rows. The separate 14-test catalogue/preservation preflight passed too. The
xtask-only corpus stage had no selected targets and is not a corpus rerun.
No real user data was touched or unchecked. Publication is a separate readback
step; these checks do not establish new application or hardware acceptance.

No source code, schema, programming admission, hardware operation, native
acceptance, format interpretation or ETS-compatibility claim changes.
