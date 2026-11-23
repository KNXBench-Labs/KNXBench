# WP-50 — Windows 11 x64 desktop: feasibility and bounded port

Owner decision: 23 November 2026 ("Ja. Dokumentieren und starten"): Windows 11
x64, the same local Tauri app with the embedded server, KNXnet/IP only at first,
no USB/serial. This authorises a bounded feasibility spike with sequential
slices, not an unbounded port and not weaker security. No Windows support is
claimed anywhere; native acceptance has not happened.

## Outcome sought

A native Windows 11 x64 build of the desktop app that opens, edits, saves and
reopens a project, with the same safety guarantees as Linux (descriptor-pinned
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

1. **Feasibility probe and documentation (this slice, delivered below).**
2. Build environment: obtain a Windows-capable C toolchain/SDK in an isolated, authorised way, or a native Windows 11 x64 machine/CI runner; get `cargo check --workspace --target x86_64-pc-windows-msvc` past the C build scripts; add `icon.ico`.
3. Source portability: fix whatever rustc reports for the Unix-only crates, mechanism by mechanism, each with its own tests and a design note for the Windows security equivalent.
4. Desktop: WebView2 process-failure handling, `msi`/`nsis` bundle targets, paths policy.
5. Native acceptance on Windows 11 x64.

## Slice 1 status (23 November 2026)

Probe: base `b15363a25`, rustc/cargo 1.98.0 (pinned), target
`x86_64-pc-windows-msvc` installed via `rustup target add` (exit 0), host
Linux. Commands, with scratch `XDG_*` and `CARGO_TARGET_DIR`:

- `cargo check --workspace --target x86_64-pc-windows-msvc --locked` → **exit 101**.
- Same with `--keep-going --message-format short` → **exit 101**.
- Per package `cargo check -p <pkg> --target x86_64-pc-windows-msvc --locked`.

Results:

- Pass (exit 0): knx-core, knx-devconf, knx-etsproj, knx-secure, knx-testsupport, knx-net, knx-report, knx-projection, knx-csv, knx-diff, knx-build-stamp.
- Fail (exit 101): knx-app, knx-edit, knx-store, knx-productdb, knx-cli, knx-desktop, knx-server, knx-mcp, xtask. In every case the first error is a **toolchain** error in a C build script, not a Rust source error: `ring 0.17.14` ("GNU compiler is not supported for this target": no MSVC `cl`) and `libsqlite3-sys 0.38.2` (cc-rs: `failed to find tool "lib.exe"`). The host has no MSVC/Windows SDK, no mingw, no `cargo-xwin`, no `lld-link`.
- One **repo-level Windows blocker** found: the `knx-desktop` build script (tauri-build) fails with ``icons/icon.ico` not found; required for generating a Windows Resource file``.
- Tauri's Windows crates (`tao`, `webview2-com`, `wry`, `tauri-runtime-wry`) type-checked.
- Because the C build scripts stop the dependants, **no Rust source of knx-app, knx-store, knx-productdb, knx-server or knx-desktop has been compiled for Windows yet**. The portability of those crates is unknown; the inventory above is by reading, not by compiler diagnostics.

Logs (not tracked, local scratch, retained by the coordinator): `check-workspace.log`, `check-keep-going.log`, `pkg-<name>.log` in the WP-50 scratch directory.

Not done: no binary, no linking, no Wine/Windows execution, no native test.
A passing cross-target check would not be Windows acceptance.

## Verified framework facts (Tauri v2 documentation, inspected 23 November 2026)

- Tauri on Windows needs the Microsoft C++ Build Tools and Microsoft Edge WebView2; the supported Rust target is MSVC (`x86_64-pc-windows-msvc`). (<https://v2.tauri.app/start/prerequisites/>)
- WebView2 ships with Windows 10 (1803+) and 11; MSI packaging needs the VBSCRIPT feature. (same page, <https://v2.tauri.app/distribute/windows-installer/>)
- The installer page documents cross-building from Linux/macOS with `cargo-xwin` and NSIS as "a last resort"; it says signing cross-compiled installers needs an external tool. `cargo-xwin` downloads the Windows SDK, which has its own licence; using it is an owner/licence decision not taken here.
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
