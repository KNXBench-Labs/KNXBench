# WP-50 — Windows 11 x64 desktop: feasibility and bounded port

Owner decision: 23 November 2026 ("Ja. Dokumentieren und starten"): Windows 11
x64, the same local Tauri app with the embedded server, KNXnet/IP only at first,
no USB/serial. This authorises a bounded feasibility spike with sequential
slices, not an unbounded port and not weaker security. No Windows support is
claimed anywhere; native acceptance has not happened.

## Current owner decision — 26 November 2026

**Slice 3a is docs only.** [ADR-0192](../adr/0192-productdb-windows-file-security.md)
is **Proposed**, not accepted: actual ProductDB Unix invariants, Windows candidates,
explicit unproved boundaries and future native acceptance cases. No production,
test or tool source changes; no ProductDB/xtask port, toolchain/SDK/binary
installation or Windows checks in this slice. Independent review and the parent's
package gate remain pending; the implementer performs documentation self-review
and `git diff --check` only.

The cargo-xwin/SDK route is **CLOSED**. At owner request the SDK/CRT cache
(~649 MiB) and task-local cargo-xwin/xwin tools were deleted on 23 November.
The conditional llvm-mingw alternative **STOPPED before download, target install,
porting edit or commit** at the unresolved Cephes FIXME in pinned mingw-w64.
The owner selected docs-only, not an exception to that stop. Primary notice and
open issue: [research record](../research/2026-11-23-windows-cross-build.md#llvm-mingw-route-stopped-before-setup-reconfirmed-26-november-2026).

Code porting waits for an **owner-provided Windows 11 x64 machine or new VM with
MSVC**; the existing analysis VM is excluded. That choice does not accept unread
terms or authorize VM creation/installation. Future native toolchain terms need
separate inspection/acceptance. No bus, hardware, release or deployment work.

Historical raw evidence was initially reported retained. The old slice 1/2/3a
evidence directories were absent on a read-only check on 26 November; possible
25 November auto-pruning is not a successful review. The independent factchecker
exited 1, **not approval**. No checks were rerun to reconstruct lost evidence.

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
explicitly proved design per mechanism; handle-based opens, ACLs, `LockFileEx`
and rename/replace APIs are candidates, not automatically race-safe equivalents
([ADR-0192](../adr/0192-productdb-windows-file-security.md)). Recovery publication
requires no replacement; atomic replacement for an inventory is a separate case.
Where a `cfg(not(unix))` branch exists today it refuses or degrades; it is not a
Windows implementation.

## Inventory of platform-critical mechanisms (source-verified at base b15363a25)

- `crates/knx-app/src/backup_directory.rs`: non-Unix branches (lines ~116, ~479) refuse descriptor-pinned backups; `device_backup.rs`, `service_control_backup.rs`, `project_history.rs` use `std::os::unix`/`rustix`.
- `crates/knx-productdb/src/cleanup.rs` (`cfg(not(unix))` at ~150), `managed_apply.rs`, `knx-store/src/activity_history.rs`.
- `apps/knx-server/src/catalogue_jobs/{artifacts,persistence,spool}.rs`, `online_catalogue/{vault,store,download}.rs`, `device_system2/storage.rs`, `tls_cert.rs`: Unix permissions, ownership, locks.
- `apps/knx-desktop/src-tauri/src/web_process.rs`: Linux WebKitGTK watcher (`webkit2gtk` is a `cfg(target_os = "linux")` dependency); a WebView2 equivalent is undecided.
- `crates/knx-net/src/tpuart.rs` and `apps/knx-server/src/serial_candidates.rs`: Linux-only; out of scope (no USB/serial).
- `apps/knx-desktop/src-tauri/tauri.conf.json`: bundle target is `appimage` only, `beforeBundleCommand` runs `tools/prepare_appimage_tools.py`; no Windows bundle target.
- At the slice 1 base, `apps/knx-desktop/src-tauri/icons/` had no `icon.ico`;
  slice 2 added it (see historical probe).
- `rustix` and `libc` appear in `knx-app`, `knx-server`, `knx-productdb`, `knx-net` manifests; their non-Unix use is not yet audited.

These are mechanisms to port, not permission to weaken them.

## Slices (sequential)

1. **Feasibility probe and documentation: published via PR #146, main `bce9f16f7`; independent review findings fixed, selected documentation gate accepted.**
2. Historical icon and cross typecheck slice: PR #149; ICO added, C build scripts reached. The cargo-xwin/SDK route is now closed and cache/tools deleted, not a continuing build environment.
3. **3a docs-only design:** ADR-0192 Proposed; route/chronology correction. **Later porting:** waits for the owner-provided native Windows 11 x64 MSVC environment and separately inspected/accepted terms; preserve Linux protection and prove Windows mechanisms before admission.
4. Desktop: WebView2 process-failure handling, `msi`/`nsis` bundle targets, paths policy.
5. Native acceptance on Windows 11 x64.

## Slice 1 status (23 November 2026; probe and dossier published)

PR #146 merged as `bce9f16f7`. The selected documentation gate passed all ten
stages on `b58f06daf`; its receipt was carried to the tree-identical main-first
merge after confirming the unchanged comparison base. Frozen inputs and checked
untouched user data were verified. No Rust, web or corpus test stage was selected
for this documentation-only package. (Historical status; see the subsequent
slice 2 and current owner decision.)

Probe: base `b15363a25`, rustc/cargo 1.98.0 (pinned), target
`x86_64-pc-windows-msvc` installed via `rustup target add`, host Linux.

Scope of every check: `cargo check` on the **default targets** of each package
(lib and bin) with the Windows `cfg` only. No `--all-targets`, no tests, no
build/link, no native or emulated execution. A pass therefore means only that
the Windows-cfg code type-checks; Linux-only implementations excluded by `cfg`
are not checked for Windows. The knx-net check reported 14 dead-code warnings
related to TP-UART; that is not evidence of Windows serial support. It is not a
linked Windows build.

### Evidence and exit codes

Two historical rounds, originally reported preserved. Their evidence directories
are now absent (see current disposition); the following describes the maintained
historical record, not a fresh raw-log verification:

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
original logs show; the original logs were the source for those historical
observations.

- Exit 0: knx-core, knx-devconf, knx-etsproj, knx-secure, knx-testsupport, knx-net, knx-report, knx-projection, knx-csv, knx-diff, knx-build-stamp.
- Exit 101: knx-app, knx-edit, knx-store, knx-productdb, knx-cli, knx-desktop, knx-server, knx-mcp, xtask. In every case the first error is a **toolchain** error in a C build script, not a Rust source error: `ring 0.17.14` ("GNU compiler is not supported for this target": no MSVC `cl`) and `libsqlite3-sys 0.38.2` (cc-rs: `failed to find tool "lib.exe"`). At that probe the host had no MSVC/Windows SDK, mingw, `cargo-xwin` or `lld-link`;
  this is not a current installation inventory.
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
- Project disposition, not a Tauri statement or legal conclusion: cargo-xwin
  fetched the Windows SDK with its own licence terms. That route is now closed;
  future native terms inspection/acceptance remains separate from environment
  selection.
- No signing cost or certificate choice is assumed.

## Slice 2 historical status (23 November 2026; icon and cross typechecks, route now closed)

PR #149 records the icon and cross-check frontier. Independent review findings
were corrected in `fccb8c354`; the Linux branch gate passed all 14 selected stages
with frozen inputs and checked untouched user data. The corpus selection was
empty. The earlier run using an unapproved system temp directory was rejected as
acceptance; the accepted run used private profile scratch. Positive icon tests
passed 6/6; targeted mutants prove rejection of bad PNG magic and unknown CLI
arguments. No further SDK commands were run during review fixes or Linux gates.

The owner chose an isolated cross-build and a general isolated SDK download first. The
worker afterwards found a terms caveat (VS2019 link vs VS2022 content, "solely with
Visual Studio" wording) and continued; the coordinator then paused all SDK use. No
consent for the caveat exists and applicability is unresolved (not legal advice).
Details, versions, excerpts, unread SDK EULA:
`docs/research/2026-11-23-windows-cross-build.md`.
Historically, cargo-xwin 0.24.1 / xwin 0.10.0 with host clang-cl provided a
`cargo check` cross toolchain (CRT 14.44.17.14, SDK 10.0.26100, task-local,
never committed; since deleted). `ring` and `libsqlite3-sys` C build scripts
passed. `icons/icon.ico` was generated from the project's own 128 and 256 px PNGs
(`tools/make_windows_icon.py`, no resampling) and `tauri-build` passed. Historical
result: 13/20 packages exited 0 (knx-edit and knx-store newly reached), workspace
and 7 packages exited 101 with Rust source errors only: knx-productdb
(43, Unix `std::os::unix`/`rustix::fs` in `managed_catalogue`, `catalogue_copy/destination`
and `managed_apply`) and xtask (2, AppImage). knx-app/cli/desktop/mcp/server were blocked
behind knx-productdb; their own source portability remains unknown. The C checks
ran with clang-cl; MSVC `cl` was not run. Still not done: link, run, resource tool,
native acceptance. The route is now closed and cache/tools deleted; these were
cross typechecks only, not Windows support, native acceptance or a release claim.

## Future native acceptance (not executed by Slice 3a)

1. `cargo check --workspace --target x86_64-pc-windows-msvc --locked` exits 0 in a documented environment.
2. On native Windows 11 x64: `cargo test` of the affected crates passes, with each Windows security mechanism covered by a test that fails when the mechanism is removed.
3. A built desktop app starts, opens a project from a user-chosen folder, edits, saves, closes and reopens it with identical content; recorded run, no bus.
4. KNXnet/IP discovery/tunnel behaviour checked against loopback or simulated transport only until separate hardware authorisation.
5. Docs state support only up to what 1–4 verified.

## Remaining decisions

- Owner-provided Windows 11 x64 machine or new VM (MSVC) must become available;
  applicable toolchain terms require separate inspection/acceptance. The old
  cargo-xwin/SDK route stays closed, llvm-mingw stays stopped; release/CRT
  redistribution questions are outside this slice.
- ADR-0192 stays Proposed: intermediate reparse traversal, ancestor/root and
  SQLite/VFS reopen binding, file IDs/link count, SID/DACL/privileged-principal
  policy, lease lifetime, no-replace visibility and directory-entry durability
  need native evidence. No Windows mechanism has been implemented or proved.
- Independent review and parent package gate of the docs-only package pending;
  future native tests/mutants in ADR-0192 are requirements, not executed results.
- WebView2 failure watcher design; installer format; signing (cost and certificate undecided).
- Whether `ring`/`libsqlite3-sys` stay as they are (checked with clang-cl only; MSVC `cl` untested).
