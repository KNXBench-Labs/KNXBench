# 2026-09-28 — Claude — K5: download to a device from the web UI

## What changed

- **ADR-0045** settles ADR-0040's open point. The start route demands the
  plan's confirmation phrase (`WriteAuthorisation::for_hardware`, the same
  gate as the CLI) and writes only the exact plan shown. It never trusts
  the UI's remembered consent. Stated plainly: the phrase proves that the
  request means *this* device, not that a person read a dialog.
- **Server** (`apps/knx-server`):
  - `device_download.rs`: one background run, a progress log (`stepStarted`,
    `dataWritten`, `stepDone`) and a terminal status (`finished` with
    `written` and `restart`, or `failed` with `written`, `stoppedInStep`,
    `error`).
  - `device_download_routes.rs`: `plan` → `start` → `status?since=`.
  - Monitor and scan starts refuse while a download runs.
  - Lock order: `start` holds download → monitor → scan until the run is
    registered. Monitor/scan take their own lock first, then only
    `try_lock` the download lock.
- **UI** (`apps/knx-web`): `DeviceDownloadPanel` as a third tab in
  `BusDiagnosticsPanel`, "Download to device" / "In Gerät laden".
  `collectDevices` is exported from `treeUtils` (it existed already).
- **Core/net**: `MemoryDownloadStep::changes_device` moved from the CLI to
  `knx-core`. `SimulatedDevice::with_config_at`.
- **Docs**: IMPLEMENTATION_STATUS "K5", manual `07-bus-and-interfaces.md`
  gets a new "Downloading to a device" section, and
  `manual/implementation-status.md` no longer claims "no button, no route
  and no command" (stale since K4). KNOWN_LIMITATIONS §101 gets its bound.
  GLOSSARY lists the new entry points. goal-commission K5 status added.

## Evidence

- HTTP end-to-end tests against the simulator: 6/6. One is a full write in
  which every reported block is compared with the simulator's memory.
- 7 server mutants and 8 UI mutants, all caught. Two of them (monitor
  check, monitor ignores download) were repeated after the lock-order
  change and caught again.
- UI: 10 panel tests. The whole web suite: 71 files, 1105 tests green
  before the gate.
- Full gate (fresh `CARGO_TARGET_DIR`): fmt, clippy `-D warnings`, 2350/0
  workspace, K5 HTTP 6, K4 CLI 11, K3 1, web 71 files and 1105 tests,
  web build, bindings clean, layering, headers (after 3 headers were
  shortened), anchors, corpus-gates, `diff --check`.

## Pitfalls met

- The simulator answers at `1.1.25`; the project targets `1.1.67`, hence
  `with_config_at`.
- A parameter edit that deactivates the linked object makes `plan` fail
  with 422, not 409. The "project edited" test therefore adds a second
  group link instead, so the configuration stays valid and the plan
  differs.
- The worktree has no corpus and no `node_modules`. Point
  `KNXBENCH_PRODUCT_CORPUS` / `KNXBENCH_K3_PROJECT` at the root checkout's
  `OriginalData`, and run `npm ci`.
- vitest output needs `NO_COLOR=1`, or grep misses the summary line.

## Not done (fence)

- The File menu's "Download project" → save/export rename is `goal-ui.md`
  U3 (the UI session, after this lock release).
- No real device was written. The UI's **[W]** run is K7 and needs the
  user's explicit, device-specific go.
