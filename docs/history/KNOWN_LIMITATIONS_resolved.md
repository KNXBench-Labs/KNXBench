# Resolved limitations — historical descriptions

Moved verbatim from [KNOWN_LIMITATIONS](../KNOWN_LIMITATIONS.md) on 2026-10-04
(AR14D D5); only relative links changed. Each entry keeps a stub with its
heading, number and status line in the main file, so numbering, triage counts
and fragment links still work. These descriptions are history: the status
line says what holds today, and the [source-ID ledger](../status/LEDGER.md)
carries the row status.

## 18. `open_project` does not clear the previous `.knxdb` `store_path`

**Resolved; source reconciliation 2026-10-01 (AR00).** The text below is
historical, not the current behavior. `apps/knx-server/src/domain.rs::open_project`
passes `None` to `replace_project_state`; its shared replacement transaction
sets `store_path` together with the project, clean baseline, opaque data and
manufacturer references under the project lock. The desktop delegates to that
domain. `http_project_routes::importing_replaces_a_dirty_project_with_a_clean_baseline`
asserts the imported snapshot has no store path. Do not redispatch the old fix.

**Limitation.** `AppState.store_path` (the `.knxdb` file a subsequent plain
`save_project` writes to) is only ever set by `save_project_as` and
`open_native_project`. The Tauri `open_project` command — ETS `.knxproj`
import — loads a fresh in-memory project but never touches `store_path`.
If a `.knxdb` was open and the user then imports a `.knxproj`, `store_path`
still points at that old `.knxdb` file.

**Cause.** `open_project` and `open_native_project` were added in
different cycles (`.knxproj` import predates the native `.knxdb` format)
and were never made to share a single "what file, if any, backs the
in-memory project" invariant.

**Impact.** None reachable through the current UI: `apps/knx-web/src/
App.tsx` resets its own `hasStorePath` flag to `false` on ETS import, so
"Save" always falls back to "Save As…" in that state. But the backend has
no equivalent guard — `save_project` just writes wherever `store_path`
points, with no check that the loaded project actually originated from
that path — so a future UI change that calls `save_project` without first
re-deriving `hasStorePath` from a real backend query could silently
overwrite the old `.knxdb` with the newly-imported project's data.

**Lifted when.** Either `open_project` clears `store_path` to `None`, or
`save_project` verifies the in-memory project actually originated from
`store_path` before writing.

**Related (2026-09-10, T10).** `export_project` used to be a second
consumer of a stale `store_path`: it re-opened `store_path` off disk to
read the opaque passthrough table and manufacturer manifest, so the same
stale-pointer scenario above could attach one project's opaque/manifest
data to a different project's export. Closed for that one code path by
reading `AppState.opaque`/`AppState.manufacturer_refs` (the live,
in-memory copies) instead of re-opening the file — see
[GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/a584007fc05a/docs/GAP_ANALYSIS_ETS.md)'s C4 row. The underlying gap
above (`store_path` itself can point at the wrong file) is unchanged.

## 23. `/api/project/download` buffers the whole `.knxdb` file in memory

**Status, reconciled 2026-10-01 (AR00): resolved for whole-file buffering.**
The historical heading is retained for fragment links. Temporary SQLite
serialization remains; it is not a whole-file response buffer.

**Resolution.** The route freshly serializes the current in-memory project,
including unsaved edits, opaque entries, and manufacturer references, into
one temporary SQLite file. `tower_http::services::ServeFile` streams that
file in bounded 64 KiB chunks instead of copying it into a whole-file
`Vec<u8>`. Content type and attachment filename remain unchanged.

The response body owns the temporary path until it is dropped, including
after the HTTP response is split into its headers and body. Both completed
and abandoned bodies remove their temporary file; response extensions alone
would not guarantee this lifetime.

**Proof.** Unit tests consume a file larger than three small test chunks,
require multiple non-empty frames bounded by the configured chunk size,
compare every byte, and verify cleanup after completed and abandoned
downloads. The HTTP regression downloads an unsaved project and opens the
result as a KNX store, preserving its latest edit, installation, opaque
entries, and manufacturer references. Serialization still creates one
temporary SQLite file before streaming begins.

## 24. `FsPicker` has no drag-and-drop or multi-select

**Status, reconciled 2026-10-01 (AR00): resolved for multi-file upload/drop.**
The historical heading remains for fragment links. Single-project selection
is intentional and does not make the implemented upload gestures absent.

**Final-review hardening, 2026-09-22.** Duplicate basenames are explicit 409
conflicts, including pre-existing upload files; no destination is overwritten.
The earlier successful count and filename/error remain visible on partial
failure. Closing/selecting/unmounting stops the remaining queue. A reopened
picker waits for the previous in-flight request, which may still finish on the
server; closing is not a rollback of that request.

**Resolution, 2026-09-22.** The browser picker still returns one
`Promise<string | null>` path because project open/import remains a singular
human choice. Its local file input now accepts multiple files, and its upload
label accepts native file drops. A shared routine sends each file to the
existing one-file `/api/fs/upload` route sequentially, refreshes the
`uploads` listing after successful requests, and announces a completed batch
as a polite status. It never auto-selects an uploaded project.

**Failure handling.** The first failed request stops that batch, keeps earlier
successful uploads intact, and names the failed file plus server error and
completed count. It does not announce batch success. During protected-mode
dragover the picker inspects only `DataTransfer.types`; it reads dropped files
only at drop time.

## 42. `command_sync.rs`'s module doc overstates its own role — pre-existing, not introduced by T12

**Resolved by AR04.** The exported compatibility helper explicitly uses the
existing transactional whole-project-save fallback. Its former successful no-op
arms and row-only persistence claims are removed; adjacent low-level helper
comments and tests no longer imply a live incremental engine. Current production
save paths remain complete saves and do not call this helper.

**Evidence.** A previously unsupported parameter edit failed behaviorally before
the fix. File-backed regressions verify nested structural batches, exact reopened
models/high-water marks, undo/redo, middle-sibling order, unchanged opaque and
manufacturer rows, and a late SQL failure retaining the prior durable state.
Three compiled behavioral mutants were caught and all touched source hashes
restored. See [the storage contract](../contracts/STORAGE_COMMAND_CONTRACT.md) and the AR04
receipt in `.ai/logs/2026-10-01_codex_alpha-storage-contract.md` for final gates
and publication; a planned gate is not a passing result.

**Retained boundary, not this former defect.** No incremental-performance claim,
automatic in-memory/history rollback, combined opaque/model transaction or new
multi-user conflict policy is introduced. Use `save_project_if_unchanged` for
the existing expected-state contract. UI/editor and commissioning scopes remain
with their original owners. This heading/fragment is a historical waypoint.

## §130 A gate binary can verify a directory that no longer exists

**Status.** Resolved by AR01 (2026-10-01); historical heading/anchor retained.

Before AR01, `xtask`'s checks derived their repository root at **compile time** from
`env!("CARGO_MANIFEST_DIR")` (`xtask/src/main.rs:41`, `:234`, `:283`), not from
the working directory at run time. A cached `xtask` binary built inside a
different worktree therefore keeps checking *that* worktree's path. When the
worktree is deleted, `check-anchors` fails with `cannot read .../DIN-3`, while
`check-headers` reports `0 files with a well-formed header ... 0 without one`
and still **exits 0** — a gate that inspected nothing and called it success.

**Evidence.** After the `din-3-goal-migration` worktree was removed,
`strings target/debug/xtask` still contained
`/mnt/daten-i/Sourcecode/.paperclip-worktrees/KNXBench/DIN-3`. `git worktree
prune` did not help (the path is in the binary, not in git metadata), and
`touch xtask/src/main.rs && cargo build -p xtask` did **not** rebuild it on
this ntfs3 mount. A build with a fresh `CARGO_TARGET_DIR` produced a binary
carrying `/mnt/daten-i/Sourcecode/KNXBench`, after which the same three gates
reported real magnitudes: anchors **382 links / 214 files**, headers **215**,
layering ok, all exit 0 **[V]**.

**Cost.** Any documentation gate run from a stale binary is worthless but
looks green. This is the skip-vs-pass failure of §129's corpus tests one layer
up: exit code 0 is not evidence that work happened.

**Resolution.** Current gate binaries select the exact runtime workspace root
from CWD or an explicit leading `--root PATH`, validate its Cargo workspace and
named members, and print that canonical target. They never climb to a parent
or fall back to the build tree. Required source/documentation scan roots must
be nonempty; layering requires every checked policy root as a workspace member
and resolved node. Corpus-gate output now includes actual Rust-file coverage.
See [verification targets](../VERIFICATION.md) and `xtask/tests/gate_scope.rs`.

AR01 reproduced the old success over zero sources after deleting its own build
worktree, then verified new deleted-target refusal, valid/wrong targets,
intentional fixtures and behavioral guard mutations. A pre-AR01 executable
still has the old bug and must be rebuilt. The caller must still verify that
the emitted target/revision is the intended candidate; nonempty coverage does
not certify completeness or concurrent-tree stability. No native UI or bus
claim is added; the separate zoom entry numbered 130 remains open.
