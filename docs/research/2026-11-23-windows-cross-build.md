# Windows MSVC cross-build toolchain and compiler frontier (2026-11-23)

WP-50 slice 2. Date of source inspection: 2026-11-23. Host Linux x86_64, Rust
1.98.0 (pinned), target `x86_64-pc-windows-msvc`. This is a **cross code check**
(`cargo check`, no link, no binary, no execution, no Wine, no native Windows).

## Current disposition (26 November 2026)

The owner chose **docs-only Slice 3a**, with
[ADR-0192 Proposed](../adr/0192-productdb-windows-file-security.md). No ProductDB
or xtask port, source/test/tool edit, toolchain/SDK/binary installation, Windows
check or historical rerun is authorized by this slice. Public documentation/source
HTTP reads supply design evidence only.

The cargo-xwin/SDK route is **CLOSED**, not paused. At the owner's request on
23 November, the SDK/CRT cache (about 649 MiB) and task-local cargo-xwin/xwin
tools were deleted. The setup below is historical, not an installed environment.
Historical logs were initially reported retained; a read-only presence check on
26 November found the old slice 1, slice 2 and slice 3a evidence directories
absent. They may have been auto-pruned on 25 November. Do not claim currently
retained raw receipts or recreate them by rerunning checks. An independent
factchecker exited 1; that is **not an approval**. Compiler counts below remain
the historical maintained record, not fresh verification.

Porting waits for an owner-provided Windows 11 x64 machine or **new** VM using
MSVC. The existing analysis VM is excluded. This environment choice does not
accept unread terms or authorize creating a VM/installing Microsoft components;
future native toolchain terms need separate inspection/acceptance.

## Sources and terms (historical SDK probe)

Not legal advice and no judgement on what is prohibited. Whether and how the terms
apply to this use is **unresolved**.

- cargo-xwin 0.24.1 (crates.io source, upstream commit
  `6d4ebf45c9eb3a749dc097f702efcc5454b61c40` from the crate's `.cargo_vcs_info.json`;
  repository <https://github.com/rust-cross/cargo-xwin>, MIT): its README says using it
  means accepting the licence at <https://go.microsoft.com/fwlink/?LinkId=2086102>;
  prerequisite is clang, optionally llvm-tools.
- xwin 0.10.0 (crates.io source, upstream commit
  `251c324d7e662c384100c1adf9bd2c3364348966`; repository
  <https://github.com/Jake-Shadle/xwin>, MIT/Apache-2.0): downloads the MSVC CRT and
  Windows SDK from Microsoft's Visual Studio channel manifest (default manifest version
  17) and prompts for a licence (`--accept-license` / `XWIN_ACCEPT_LICENSE=1` skips it).
- The open-source licences of these two tools are separate from the proprietary
  Microsoft components (CRT, SDK) they fetch.
- The fwlink in the cargo-xwin README redirects to
  <https://visualstudio.microsoft.com/license-terms/mlt031519/> (Visual Studio 2019
  Remote Debugger, ... and Build Tools; document
  `VS_2019_DiagnosticBuildTools_ENU.1033.docx`, sha256
  `03c90f7595dd03827e3944af29c5647598a8ce3ef0c97d3e1f99342f73a786b1`). Its
  use grant is tied to Visual Studio Community, Professional or Enterprise.
- The manifest that xwin fetched lists `Microsoft.VisualStudio.Product.BuildTools` with
  licence link `LinkId=2179911`, which redirects to
  <https://visualstudio.microsoft.com/license-terms/vs2022-ga-diagnosticbuildtools/>
  and its document `Visual-Studio-2022-Diagnostic-Build-Tools-Agent-License_Update-March-2024_EN.docx`
  (sha256 `2f66b86a00e8d9833789897ce23d05a4a2dbea370cf39c8c1098dbc17d0e7bdc`, hash
  recomputed from the saved local copy). The following is a summary, not legal advice:
  - The tools supplement the named Visual Studio editions. The use grant requires
    "a valid license to one of the Visual Studio Products".
  - The Build Devices provision ties compilation, building and verification to
    applications developed using Visual Studio Products. A separate provision
    addresses open-source C++ dependencies without a Visual Studio licence and
    limits modifications to those dependencies.
  - Paraphrase: no sharing, publishing or renting the software, and no stand-alone
    offering or bundling with applications for others.
- So the matching 2022 terms do not close the question: it is not settled by the text
  whether using the downloaded CRT/SDK without an installed Visual Studio edition fits
  these terms.
- The manifest also lists a separate "Windows SDK EULA" package for the SDK
  (10.0.26100). Its terms were **not read** (reading needs a download) and are not
  covered by the Build Tools paragraphs above.
- Consent chronology: the owner first approved a general isolated SDK download. The
  worker only afterwards found the VS2019/VS2022 mismatch and the "solely with Visual
  Studio" wording and continued instead of stopping; the coordinator then paused all SDK
  use. The owner has not consented with knowledge of these caveats, and no applicability
  is presumed authorised. No vendor was contacted.
- Subsequent disposition: the owner closed the route and requested deletion;
  cache and task-local tools were deleted on 23 November (see above). No further
  SDK use or redistribution follows. The unread SDK terms and unresolved
  applicability were not settled by deleting the cache or choosing a future
  native environment.

## Historical task-local setup (deleted; no current installation claim)

- `cargo install --locked --root <task>/tools cargo-xwin@0.24.1 xwin@0.10.0`
  (crates.io, built from source with Rust 1.98.0). Binary sha256: cargo-xwin
  `c05bee55654de62ae9c2eb54b0698897764514e07df4f88f8c589c8952b47de1`, xwin
  `1d2399d28feeb86f87cee6ab52d2c8fab66ab519bda04252fbe75a6a1ca87956`.
- At the time: host clang / clang-cl 23.1.3 and `llvm-lib` (already present;
  this record makes no current tool-availability claim).
- `lld-link`: a symlink in the task tools directory to the `rust-lld` (LLD 22.1.8)
  shipped with the pinned Rust toolchain; no other linker was installed. The checks
  below do not link, so the linker was not exercised by them.
- SDK/CRT cache: `XWIN_CACHE_DIR=<task>/xwin-cache cargo xwin cache xwin` (`XWIN_ARCH=x86_64`,
  `XWIN_ACCEPT_LICENSE=1`), exit 0, about 649 MiB. Content: Visual Studio manifest
  `VisualStudio.17.Release/17.14.45+37906.3` (`manifest_17.json` sha256
  `44a3e7dc44486c945abdc1fae97e7e22de4d05e80f03b00e9302ba4b72c3e0e3`), MSVC CRT
  `14.44.17.14`, Windows SDK `10.0.26100`, UCRT.

## Historical compiler frontier (23 November; not rerun)

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

Historical evidence description (task scratch, not tracked): logs and a JSON
receipt with argv/return codes were reported. That receipt contained 22 entries:
2 workspace runs plus 20 per-package checks. Tool installation and cache setup
(both reported exit 0) had logs with an `exit=0` trailer, not receipt entries.
Those evidence directories are now absent; no setup was rerun to reconstruct them.

## Not done

No link, no Windows binary, no run (Wine or native), no resource-compiler/NSIS/MSI
exercise, no WebView2. `icon.ico` only lets `tauri-build` proceed; the native icon
appearance is unverified.

## llvm-mingw route stopped before setup (reconfirmed 26 November 2026)

The conditional alternative stopped before any toolchain download, Rust target
installation, porting edit or commit. The stop was the unresolved literal notice
in `COPYING.MinGW-w64-runtime/COPYING.MinGW-w64-runtime.txt`, pinned mingw-w64
commit `41eb042358a52011b35350392130dcdc94a8ead8`, lines 209–220:

```text
FIXME: Cephes math lib
Copyright (C) 1984-1998 Stephen L. Moshier

It sounds vague, but as to be found at
<http://lists.debian.org/debian-legal/2004/12/msg00295.html>, it gives an
impression that the author could be willing to give an explicit
permission to distribute those files e.g. under a BSD style license. So
probably there is no problem here, although it could be good to get a
permission from the author and then add a license into the Cephes files
in MinGW runtime. At least on follow-up it is marked that debian sees the
version a-like BSD one. As MinGW.org (where those cephes parts are coming
from) distributes them now over 6 years, it should be fine.
```

Primary sources read again on 26 November (public text only):

- [Pinned upstream notice, lines 209–220](https://github.com/mingw-w64/mingw-w64/blob/41eb042358a52011b35350392130dcdc94a8ead8/COPYING.MinGW-w64-runtime/COPYING.MinGW-w64-runtime.txt#L209-L220),
  also fetched as [raw pinned text](https://raw.githubusercontent.com/mingw-w64/mingw-w64/41eb042358a52011b35350392130dcdc94a8ead8/COPYING.MinGW-w64-runtime/COPYING.MinGW-w64-runtime.txt).
  The FIXME paragraph is present literally.
- [Upstream issue mingw-w64/mingw-w64#64](https://github.com/mingw-w64/mingw-w64/issues/64),
  state `open` in the [upstream API response](https://api.github.com/repos/mingw-w64/mingw-w64/issues/64).
  The issue raises questions about this notice; its title and allegations are
  not findings of this project.

This records an unresolved prerequisite under the owner's stop rule, not a legal
conclusion that the toolchain is unlawful. Another project's licence statement
does not resolve the pinned runtime notice for this route. The owner selected
docs-only on 26 November, **not** an exception authorizing llvm-mingw. No setup
or use was performed in this slice.

## ProductDB design evidence (26 November 2026)

[ADR-0192](../adr/0192-productdb-windows-file-security.md) inventories source at
`fe6b30dc3`, separates ProductDB from the server's lease/ownership responsibilities,
and cites primary Microsoft docs for candidate APIs. Handle-relative traversal,
ancestor binding, SID/DACL and privileged-principal policy, SQLite/VFS reopen
identity, and publication durability remain unproved. Proposed native adversarial
and mutant acceptance cases have not been run. No Windows implementation or
native proof is claimed; Linux protection must be preserved.
