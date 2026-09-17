# Linux AppImage handover

## Decision and scope

ADR 0021 selects an x86_64 AppImage as KNXBench's first Linux desktop package.
It keeps the existing Tauri desktop shell and bundled web frontend; no second
runtime or package-specific application layer was added. The bounded support
statement is compatible glibc, GTK 3, and WebKitGTK 4.1 on x86_64 Linux.

The checked-in GitHub Actions workflow is configured for manual dispatch and
pushed `v*` tags. A manual run uploads an Actions artifact; a tag run is
configured to create or update a GitHub release. It was not dispatched or run
in this work, and no push, tag, or release was created.

## Local build evidence

Host: `big-omarchy`, Linux `7.2.5-3-omarchy`, x86_64. Tooling: Rust
`1.98.0 (88d9e12ae 2026-08-18)`, Cargo `1.98.0 (797e8a9bc 2026-08-05)`,
Node `v26.8.1`, npm `11.19.0`, Tauri CLI `2.11.4`, and gdk-pixbuf `2.44.7`.
`npm ci --prefix apps/knx-web` installed 63 locked packages. The final build
used an isolated Tauri cache:

```bash
NO_STRIP=1 \
XDG_CACHE_HOME=/tmp/knxbench-tauri-cache.TkesPq \
APPIMAGE_EXTRACT_AND_RUN=1 \
cargo tauri build --bundles appimage --ci
```

Arch gdk-pixbuf advertises an absent legacy loader directory. The upstream GTK
deployment plugin expects it, so an isolated copy in `/tmp` was adjusted to
create the AppDir cache directory. That uncommitted, host-only adjustment had
SHA-256 `7829d7081db747ce260ee64c2f6b294bf8d4570998f631e6bdc1ea91a77a41a7`;
the final AppImage contains no `/tmp` tree. The committed `NO_STRIP: "1"`
workflow setting addresses linuxdeploy's inability to strip modern Arch
`.relr.dyn` sections and is semantically checked by `xtask`.

## Artifact and inspection

`target/release/bundle/appimage/KNXBench_0.1.0-alpha.1_amd64.AppImage` was the
only artifact in its directory. It is executable, 105839096 bytes, and has
SHA-256 `b0ec49aebcec984ffdce21639306713862fc6c7baf15f7247f060ac314cdaef7`.
`file` identified an ELF 64-bit x86-64 static-PIE AppImage runtime.

`cargo run -p xtask -- check-appimage` passed. Extraction from an empty
`/tmp/knxbench-appimage-inspect` directory with `--appimage-extract` passed and
verified `usr/bin/knx-desktop` (25,444,888 bytes), `KNXBench.desktop` with
`Exec=knx-desktop`, `Icon=knx-desktop`, and `Type=Application`, the root icon,
both 128/256 hicolor icons, `frontend/index.html` plus 88 frontend files, and
the 34,523-byte bundled `LICENSE`. Its SHA-256 was
`0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0`, identical
to repository `LICENSE`.

## Startup proof and checks

`xvfb-run`/`Xvfb` were unavailable. The bounded startup used the active
XWayland display:

```bash
timeout 15s env APPIMAGE_EXTRACT_AND_RUN=1 \
  /mnt/daten-i/Sourcecode/KNXBench/.worktrees/linux-appimage/target/release/bundle/appimage/KNXBench_0.1.0-alpha.1_amd64.AppImage
```

It ran 15,003 ms and ended with status 124 from `timeout`. At three seconds,
the `KNXBench` window (`Knx-desktop`) was mapped, visible, and 984 x 1098.
The 126-byte log had only two XWayland/GBM allocation warnings; it had no
library/resource, panic, or server-start failure. No KNX discovery, gateway
contact, bus read, or bus write occurred.

Task 4 checks passed: `cargo fmt --all --check`; `cargo test -p xtask` (46
passed, 0 failed); `cargo clippy -p xtask --all-targets -- -D warnings`;
`cargo deny check` (existing duplicate-version warnings only);
`cargo run -p xtask -- check-appimage`; and `git diff --check`.

## Limitations

This local Arch/XWayland run does not establish Ubuntu CI success or general
Linux distribution compatibility. The checked-in workflow remains unexecuted.
`NO_STRIP=1` may increase the package size by retaining debug/symbol data in
linuxdeploy-copied libraries. The GBM warnings leave other GPU/display stacks
untested. There is no updater, signature, ARM64 build, or native dependency
management in this alpha package.

## Final verification and scope review

Task 6 reviewed the 11 commits and all 20 changed files from `f7893ec` through
`23b27a4`. `cargo fmt --all --check`, xtask clippy, 46 xtask tests, layering,
471 web tests across 42 files, the web production build (81 modules),
`check-appimage`, and `git diff --check f7893ec..HEAD` passed. `check-headers`
failed exactly at the known pre-existing baseline: 170 files without a header
against ceiling 168. No source-header cleanup was added to this packaging
branch.

The final artifact identity was unchanged: 105839096 bytes, executable, and
SHA-256 `b0ec49aebcec984ffdce21639306713862fc6c7baf15f7247f060ac314cdaef7`.
A fresh empty-directory extraction again verified the 25,444,888-byte desktop
binary, expected desktop metadata, root and 128/256 icons, 88 frontend files,
and the 34,523-byte canonical licence with SHA-256
`0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0`.
A second active-XWayland launch mapped a visible `Knx-desktop` window and ran
15002 ms until timeout status 124. Its 126-byte log again contained only the
two GBM allocation warnings. No KNX discovery, gateway action, bus read, or
bus write was invoked.

The full branch review confirmed exactly one `appimage` bundle target, no
updater, signing, ARM64, native package format, runtime, storage, or bus-layer
change, and no packaged project, manufacturer database, keyring, password, or
gateway data. Release publication is conditioned on both a push event and tag
ref; manual dispatch can only upload an Actions artifact. Documentation keeps
the local Arch/XWayland evidence separate from the configured but unexecuted
Ubuntu workflow. No push, tag, release, workflow dispatch, merge, or
publication occurred.
