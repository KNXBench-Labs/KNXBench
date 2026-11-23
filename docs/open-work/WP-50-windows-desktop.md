# WP-50 — Windows 11 x64 desktop: feasibility and bounded port

Owner decision: 23 November 2026 ("Ja. Dokumentieren und starten"): Windows 11
x64, the same local Tauri app with the embedded server, KNXnet/IP only at first,
no USB/serial. This authorises a bounded feasibility spike with sequential
slices, not an unbounded port and not weaker security. No Windows support is
claimed anywhere; native acceptance has not happened.

## Outcome sought

A native Windows 11 x64 build of the desktop app that opens, edits, saves and
reopens a project, with the goal of the same safety guarantees as Linux (descriptor-pinned
writes, owner-only file modes, atomic commits) implemented with Windows
mechanisms, not skipped.

## Paths versus filesystem security

Two separate questions. (1) Where data lives: per-user AppData Local/Roaming for
application data and user-chosen Documents/project locations. This is a path
policy (`apps/knx-server/src/paths.rs`, `knx-desktop` save/download
directories). (2) How files are written safely: Unix descriptor-pinned
directories, `0o600/0o700` modes, `flock`-style locks and ownership checks via
`rustix`/`std::os::unix`. These are OS-specific mechanisms. Windows needs an
equivalent design (handle-based opens, ACLs, `LockFileEx`, atomic replace),
decided per mechanism. Where a `cfg(not(unix))` branch exists today it refuses
or degrades; it is not a Windows implementation.

## Inventory of platform-critical mechanisms (source-verified at base b15363a25)

- `crates/knx-app/src/backup_directory.rs`: non-Unix branches (lines ~116, ~479) refuse descriptor-pinned backups; `device_backup.rs`, `service_control_backup.rs`, `project_history.rs` use `std::os::unix`/`rustix`.
- `crates/knx-productdb/src/cleanup.rs` (`cfg(not(unix))` at ~150), `managed_apply.rs`, `knx-store/src/activity_history.rs`.
- `apps/knx-server/src/catalogue_jobs/{artifacts,persistence,spool}.rs`, `online_catalogue/{vault,store,download}.rs`, `device_system2/storage.rs`, `tls_cert.rs`: Unix permissions, ownership, locks.
- `apps/knx-desktop/src-tauri/src/web_process.rs`: Linux WebKitGTK watcher (`webkit2gtk` is a `cfg(target_os = "linux")` dependency); a WebView2 equivalent is undecided.
- `crates/knx-net/src/tpuart.rs` and `apps/knx-server/src/serial_candidates.rs`: Linux-only; out of scope (no USB/serial).
- `apps/knx-desktop/src-tauri/tauri.conf.json`: bundle target is `appimage` only, `beforeBundleCommand` runs `tools/prepare_appimage_tools.py`; no Windows bundle target.
- `apps/knx-desktop/src-tauri/icons/` has no `icon.ico` (see probe).
- `rustix` and `libc` appear in `knx-app`, `knx-server`, `knx-productdb`, `knx-net` manifests; their non-Unix use is not yet audited.

These are mechanisms to port, not permission to weaken them.

## Slices (sequential)

1. **Feasibility probe and documentation (this slice: probe done, pending review/gate and publication).**
2. Build environment: obtain a Windows-capable C toolchain/SDK in an isolated, authorised way, or a native Windows 11 x64 machine/CI runner; get `cargo check --workspace --target x86_64-pc-windows-msvc` past the C build scripts; add `icon.ico`.
3. Source portability: fix whatever rustc reports for the Unix-only crates, mechanism by mechanism, each with its own tests and a design note for the Windows security equivalent.
4. Desktop: WebView2 process-failure handling, `msi`/`nsis` bundle targets, paths policy.
5. Native acceptance on Windows 11 x64.

## Slice 1 status (23 November 2026; probe done, pending final gate and publication)

Probe: base `b15363a25`, rustc/cargo 1.98.0 (pinned), target
`x86_64-pc-windows-msvc` installed via `rustup target add`, host Linux.

Scope of every check: `cargo check` on the **default targets** of each package
(lib and bin) with the Windows `cfg` only. No `--all-targets`, no tests, no
build/link, no native or emulated execution. A pass therefore means only that
the Windows-cfg code type-checks; Linux-only code stays compiled under its own
`cfg` (knx-net passed with 14 `tpuart.rs`-related warnings, for example). It is
not a Windows build.

### Evidence and exit codes

Two rounds, both preserved:

- **Original run** (logs `check-workspace.log`, `check-keep-going.log`,
  `pkg-<name>.log`, `target-add.log` in the WP-50 scratch directory). These
  logs contain no exit lines; the exit codes (`rustup target add` 0, both
  workspace commands 101) were reported by the worker, and the pass/fail split
  per package was derived from log content. The worker's first wrapper
  returned exit 0 while cargo itself exited 101: the wrapper exit was **not** a
  successful check and is not used as evidence.
- **Recheck** (cached target directory, same commands and scratch `XDG_*`),
  driven by a small private Python subprocess controller that recorded argv,
  cwd, exact return code per command and the source HEAD/base in
  `probe-exits.json` (logs `recheck-*.log`; the 20 packages come from
  `cargo metadata`, not a hand list). Source HEAD `395cc25af`, base
  `b15363a25`.

Recheck exit codes: `rustup target add x86_64-pc-windows-msvc` **0**;
`cargo check --workspace --target x86_64-pc-windows-msvc --locked` **101**;
same with `--keep-going --message-format short` **101**; per-package
`cargo check -p <pkg> --target x86_64-pc-windows-msvc --locked` over 20
packages: **11 exit 0, 9 exit 101**. The result is identical to the original
run. Recheck logs replay cached output, so they omit `Checking` lines that the
original logs show; the original logs remain the source for those.

- Exit 0: knx-core, knx-devconf, knx-etsproj, knx-secure, knx-testsupport, knx-net, knx-report, knx-projection, knx-csv, knx-diff, knx-build-stamp.
- Exit 101: knx-app, knx-edit, knx-store, knx-productdb, knx-cli, knx-desktop, knx-server, knx-mcp, xtask. In every case the first error is a **toolchain** error in a C build script, not a Rust source error: `ring 0.17.14` ("GNU compiler is not supported for this target": no MSVC `cl`) and `libsqlite3-sys 0.38.2` (cc-rs: `failed to find tool "lib.exe"`). The host has no MSVC/Windows SDK, no mingw, no `cargo-xwin`, no `lld-link`.
- `knx-web` is an npm project, not a Cargo package, and is not part of the 20.
- One **repo-level Windows blocker** found: the `knx-desktop` build script (tauri-build) fails with ``icons/icon.ico` not found; required for generating a Windows Resource file``. It appears in the `--keep-going` workspace run only (original and recheck); the per-package `knx-desktop` log stops earlier at the ring/libsqlite3-sys build scripts.
- Tauri's Windows crates (`tao`, `webview2-com`, `wry`, `tauri-runtime-wry`) show `Checking` in the original `--keep-going` log; no error is reported for them.
- Because the C build scripts stop the dependants, **none of the own workspace source of knx-app, knx-store, knx-productdb, knx-server or knx-desktop was reached** (their dependencies were checked). Their portability is unknown; the inventory above is by reading, not by compiler diagnostics.

Not done: no binary, no linking, no Wine/Windows execution, no native test.
A passing cross-target check would not be Windows acceptance.

## Framework statements (Tauri v2 documentation, read 23 November 2026)

Quoted or closely paraphrased from the fetched pages; not independently verified on Windows.

- Prerequisites page (<https://v2.tauri.app/start/prerequisites/>): Microsoft C++ Build Tools and Microsoft Edge WebView2 are required; the page names the MSVC toolchain as needed for full Tauri support. Choosing `x86_64-pc-windows-msvc` as our target follows from that.
- The page states WebView2 is installed on Windows 10 from version 1803 onward and later versions; it does not name Windows 11 explicitly.
- Installer page (<https://v2.tauri.app/distribute/windows-installer/>): MSI packaging requires the VBSCRIPT optional feature, which the page says is enabled by default but being deprecated. Cross-building from Linux/macOS (`cargo-xwin`, NSIS only) is described as "a last resort", and signing a cross-compiled installer needs an external tool.
- Our own open point, not a Tauri statement and not a legal conclusion: `cargo-xwin` downloads the Windows SDK, which carries its own licence terms; whether and by whom they are accepted is a pending owner decision.
- No signing cost or certificate choice is assumed.

## Acceptance (measurable)

1. `cargo check --workspace --target x86_64-pc-windows-msvc --locked` exits 0 in a documented environment.
2. On native Windows 11 x64: `cargo test` of the affected crates passes, with each Windows security mechanism covered by a test that fails when the mechanism is removed.
3. A built desktop app starts, opens a project from a user-chosen folder, edits, saves, closes and reopens it with identical content; recorded run, no bus.
4. KNXnet/IP discovery/tunnel behaviour checked against loopback or simulated transport only until separate hardware authorisation.
5. Docs state support only up to what 1–4 verified.

## Remaining decisions

- Toolchain for slice 2: native Windows 11 machine or CI runner, or isolated SDK download (licence acceptance) — owner.
- Windows equivalents for descriptor-pinned directory writes, owner-only ACLs and locks (ADR needed).
- WebView2 failure watcher design; installer format; signing (cost and certificate undecided).
- Whether `ring`/`libsqlite3-sys` stay as they are (expected, not verified, to build with MSVC `cl`).
