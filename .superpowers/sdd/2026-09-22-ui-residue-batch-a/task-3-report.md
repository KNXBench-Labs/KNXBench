# Task 3 report: bounded project downloads

Status: complete. Scope: T12 Task 3 / `KNOWN_LIMITATIONS.md` §23 only.

## Implementation

- `apps/knx-server/src/fs_routes.rs`: preserves fresh serialization of the live project, opaque entries, and manufacturer references. Mutex guards end before the async file-service call. The temporary SQLite file is served by `ServeFile` with a 64 KiB buffer; no whole-file `std::fs::read` remains in this route.
- A private `TemporaryFileBody` delegates frames, end-of-stream, and size hints to the Axum body and owns `Arc<TempPath>`. The body field precedes the path guard so its file handle drops first. The guard removes the temporary file when the body is dropped, including abandoned downloads.
- The content type remains `application/octet-stream`; disposition remains `attachment; filename="project.knxdb"`.
- `apps/knx-server/Cargo.toml` and `Cargo.lock`: direct production `http-body = "1"` names the trait's frame/size-hint API; test-only `http-body-util = "0.1"` exposes frame-by-frame assertions. Both packages were already present transitively; no new package version was added.
- `apps/knx-server/tests/http_fs_routes.rs`: a real POST creates an unsaved project; the fixture then changes its in-memory name and adds opaque/manufacturer entries. GET download must return the expected headers and a readable KNX store preserving the latest name, installation, opaque bytes, and manufacturer references.
- Updated only limitation §23 plus `.ai/CURRENT_STATE.md` and this report. No frontend, KNX protocol, or other limitation work.

## Body ownership refinement

The brief proposed retaining the path only in a response extension. HTTP response/body separation can discard extensions while the body is still being sent. The parent explicitly approved retaining the Arc in the private body wrapper instead, with an extension optional. The implementation needs no redundant response extension. The parent also approved the direct production `http-body` dependency. Tests call `into_body()` explicitly and verify cleanup for consumed and unconsumed bodies.

## TDD evidence

Tests and the requested dev dependency were added first. To obtain a runtime RED rather than an undefined-helper compiler error, the existing buffered implementation was extracted unchanged into private `stream_temp_file`; production still performed `std::fs::read` and returned the whole `Vec` at that stage.

All Cargo gate commands used:

```text
CARGO_TARGET_DIR=/var/tmp/knxbench-t12-target
CARGO_INCREMENTAL=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0
CARGO_BUILD_JOBS=2
```

RED command: `cargo test -p knx-server fs_routes::tests --lib` — exit 101, 0 passed / 2 failed / 153 filtered out.

- `temporary_download_streams_bounded_frames_and_cleans_up_after_body_drop`: expected failure `oversized body frame: 785`, with a configured limit of 256 bytes.
- `dropping_an_unconsumed_download_body_removes_its_temporary_file`: expected failure `response extraction must not remove the temporary file`.
- The HTTP success regression passed against the original buffered implementation, as expected for a compatibility regression; it was then strengthened to cover the latest in-memory edit and opaque/manufacturer preservation before the final focused GREEN run.

Minimal GREEN changed the helper to `ServeFile` plus the private body/path guard. The streaming regression consumes all frames, requires at least two non-empty data frames, asserts every frame is at most 256 bytes, and compares all 785 source bytes. A whole-file `Vec` response cannot pass it. It also proves the file remains present after consumption until body drop, then disappears. The abandoned-body test proves response extraction does not drop the file and unconsumed body drop does.

## Final gates

| Command | Result |
| --- | --- |
| `cargo test -p knx-server fs_routes::tests --lib` | exit 0; 2 passed, 153 filtered out |
| `cargo test -p knx-server --test http_fs_routes` | exit 0; 7 passed |
| `cargo test --workspace` | exit 0; 1,951 passed, 0 failed, 5 ignored, 92 result blocks; one full run |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy -p knx-server --all-targets -- -D warnings` | exit 0; no warnings |
| `git diff --check` | exit 0 |

Full workspace output: `/var/tmp/knxbench-t12-task3-workspace.log`. Clippy output: `/var/tmp/knxbench-t12-task3-clippy.log`. These are local gate artifacts, not repository files. No test failures were omitted. All commands were local; no KNX, multicast, LAN, gateway, or hardware traffic was initiated.

## Self-review

- Body memory is bounded by the file-service chunk size; the test never collects the body before inspecting individual frames.
- The download still serializes the current in-memory project regardless of `store_path`, including passthrough data. Header and readable-store regressions cover the route end to end.
- The path remains owned across `Response::into_body()`, and RAII cleanup covers both success and cancellation. The private wrapper introduces no public API or generalized file service.
- State locks are confined to the existing synchronous serialization phase and are not held across streaming awaits.
- Only the permitted server files, lockfile dependency edges, §23, and handover/report changed. No unrelated refactor.
- Remaining limitation: serialization still creates one complete temporary SQLite file before the response starts. This change bounds the response-body buffer, not serialization work or the already resident domain model.

Concerns: none outstanding within Task 3 scope. No push or reviewer/subagent dispatch performed.
