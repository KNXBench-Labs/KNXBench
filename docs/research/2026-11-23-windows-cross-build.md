# Windows MSVC cross-build toolchain and compiler frontier (2026-11-23)

WP-50 slice 2. Date of source inspection: 2026-11-23. Host Linux x86_64, Rust
1.98.0 (pinned), target `x86_64-pc-windows-msvc`. This is a **cross code check**
(`cargo check`, no link, no binary, no execution, no Wine, no native Windows).

## Sources and terms

- cargo-xwin 0.24.1 (<https://github.com/rust-cross/cargo-xwin>, MIT): its README
  states that using it means accepting the licence at
  <https://go.microsoft.com/fwlink/?LinkId=2086102>; prerequisite is clang, optionally
  llvm-tools.
- xwin 0.10.0 (<https://github.com/Jake-Shadle/xwin>, MIT/Apache-2.0): downloads
  the MSVC CRT and Windows SDK from Microsoft's Visual Studio channel manifest
  (default manifest version 17) and prompts for the same licence (`--accept-license`
  / `XWIN_ACCEPT_LICENSE=1` skips the prompt).
- The fwlink redirects to <https://visualstudio.microsoft.com/license-terms/mlt031519/>,
  "Microsoft Visual Studio 2019 Remote Debugger, Stand-alone Profiler, Intellitrace,
  Snapshot Debugger, other Debuggers and Build Tools". Its document
  `VS_2019_DiagnosticBuildTools_ENU.1033.docx` (sha256
  `03c90f7595dd03827e3944af29c5647598a8ce3ef0c97d3e1f99342f73a786b1`) was read in full.
  Relevant text, quoted:
  - "You may install and use any number of copies of the software to use solely with
    Visual Studio Community, Visual Studio Professional, and Visual Studio Enterprise,
    to develop and test your applications."
  - "You may copy and install files from the software onto your build devices ...
    You and others in your organization may use these files on your Build Devices
    solely to compile, build, and verify applications developed by using the software ..."
  - Not permitted: "share, publish, rent, or lease the software; or provide the software
    as a stand-alone offering or combine it with any of your applications for others to
    use, or transfer the software or this agreement to any third party."
- **Open point, not a legal conclusion.** The terms linked by the tools are the
  VS 2019 Build Tools terms, while xwin fetches the VS 2022 (17.x) channel content.
  Whether using the downloaded CRT/SDK without an installed Visual Studio edition
  falls under "solely with Visual Studio ..." / "Build Devices" is not settled by the
  text; the owner authorised the isolated local setup knowing this. Consequences kept:
  the SDK/CRT stays in a task-local cache, is never committed, bundled or published,
  and no artifact built here is offered as a redistributable SDK component. Linking
  the CRT into a release binary is a separate question for any release slice.

## Installed tools (task-local, no system or global config change)

- `cargo install --locked --root <task>/tools cargo-xwin@0.24.1 xwin@0.10.0`
  (crates.io, built from source with Rust 1.98.0). Binary sha256: cargo-xwin
  `c05bee55654de62ae9c2eb54b0698897764514e07df4f88f8c589c8952b47de1`, xwin
  `1d2399d28feeb86f87cee6ab52d2c8fab66ab519bda04252fbe75a6a1ca87956`.
- Host clang / clang-cl 23.1.3 and `llvm-lib` (system packages already present, not installed here).
- `lld-link`: a symlink in the task tools directory to the `rust-lld` (LLD 22.1.8)
  shipped with the pinned Rust toolchain; no other linker was installed. The checks
  below do not link, so the linker was not exercised by them.
- SDK/CRT cache: `XWIN_CACHE_DIR=<task>/xwin-cache cargo xwin cache xwin` (`XWIN_ARCH=x86_64`,
  `XWIN_ACCEPT_LICENSE=1`), exit 0, about 649 MiB. Content: Visual Studio manifest
  `VisualStudio.17.Release/17.14.45+37906.3` (`manifest_17.json` sha256
  `44a3e7dc44486c945abdc1fae97e7e22de4d05e80f03b00e9302ba4b72c3e0e3`), MSVC CRT
  `14.44.17.14`, Windows SDK `10.0.26100`, UCRT.

## Compiler frontier

Every command ran under `cargo xwin` with a task-local `CARGO_TARGET_DIR`, scratch
`XDG_*`, `--locked`, via a Python subprocess controller that recorded argv and the exact
return code (not a shell wrapper). Scope: default targets (lib/bin), `cargo check` only.

- `cargo xwin check --workspace --target x86_64-pc-windows-msvc --locked --keep-going --message-format short`
  before the icon: **101**. The C build scripts (`ring 0.17.14`, `libsqlite3-sys 0.38.2`)
  now **pass** with clang-cl and the downloaded CRT/SDK headers; `knx-desktop`'s build
  script failed on the missing `icons/icon.ico`.
- After adding `icon.ico`: **101**; the `tauri-build` script passes. Remaining errors are
  only Rust source errors in two crates.
- Per package (20 from `cargo metadata`): 13 exit 0 (knx-core, knx-devconf, knx-etsproj,
  knx-secure, knx-testsupport, knx-net, knx-report, knx-projection, knx-csv, knx-diff,
  knx-build-stamp, plus the newly reached **knx-edit** and **knx-store**), 7 exit 101
  (knx-productdb, knx-app, knx-cli, knx-desktop, knx-mcp, knx-server, xtask).
  A pass is Windows-cfg type checking only; `cfg(unix)` implementations are not checked
  for Windows and knx-net passing does not mean serial/USB support.
- Own-source diagnostics, all portability class "Unix-only std/rustix API in code without
  a `cfg(unix)` gate or Windows equivalent":
  - `knx-productdb`: 43 errors (E0432, E0433 and mostly E0599):
    `managed_catalogue.rs` 18, `catalogue_copy/destination.rs` 16,
    `catalogue_copy/destination/publication.rs` 5 (`rustix::fs` unavailable),
    `managed_apply.rs` 4 (`std::os::unix`, `MetadataExt` dev/ino/nlink/mtime/ctime,
    `OpenOptionsExt::mode`, `DirBuilderExt::mode`). This is exactly the descriptor-pinned
    and owner-mode mechanism listed in the dossier.
  - `xtask`: 2 errors in `xtask/src/appimage.rs` (`std::os::unix`, `Permissions::mode`).
    AppImage-only tooling, not shipped on Windows.
  - knx-app, knx-cli, knx-desktop, knx-mcp, knx-server fail only because they depend on
    knx-productdb; their own source is still **not reached**.
- Toolchain errors remaining: none observed after the setup.

Evidence (task scratch, not tracked): logs and a JSON receipt with argv/return codes.

## Not done

No link, no Windows binary, no run (Wine or native), no resource-compiler/NSIS/MSI
exercise, no WebView2. `icon.ico` only lets `tauri-build` proceed; the native icon
appearance is unverified.
