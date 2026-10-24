# AppImage launcher contract (KL-158)

The post-alpha launcher change requested on 2026-10-07. It changes packaging,
not the KNX domain, project format, server API or commissioning permissions.
The published `v0.1.0-alpha.4` tag and its downloadable AppImage are **not**
replaced by this work. Publication status lives in
[the source-ID ledger](status/LEDGER.md).

## Backend selection

The generated GTK AppRun hook contains the policy from
`tools/appimage/display-backend.sh`, before GTK initialization:

| Caller/session environment | Effective `GDK_BACKEND` | Default `WEBKIT_DISABLE_DMABUF_RENDERER` |
| --- | --- | --- |
| Nonempty `GDK_BACKEND` | Preserved exactly, including ordering and `*` | `1` only if that list permits `wayland` or `*` |
| No backend; nonempty `WAYLAND_DISPLAY` or `XDG_SESSION_TYPE=wayland` | `wayland,x11` | `1` |
| No backend and no Wayland session hint | `x11` | Unset |

An empty `GDK_BACKEND` is treated as automatic selection. An explicitly set
`WEBKIT_DISABLE_DMABUF_RENDERER` is never overwritten, including `0` and the
empty string. GTK, not a second socket-probing implementation, tries the
backends in order. A session hint is not proof of a reachable compositor;
stale Wayland can fall back to X11, but a caller asking for `wayland` alone
gets no hidden X11 retry. Neither backend can start without an actual display.

The DMABUF default is the workaround measured in AR17/KL-158; it does **not**
claim universal GPU compatibility or that every renderer becomes software-only.
Performance of the alternate WebKit rendering path has not been benchmarked.
The opt-out for driver diagnostics is explicit, for example:

```sh
GDK_BACKEND=wayland WEBKIT_DISABLE_DMABUF_RENDERER=0 ./KNXBench.AppImage
GDK_BACKEND=x11 ./KNXBench.AppImage
```

No unpack/source/re-run workaround is needed in an AppImage built with this
contract. Older released images still need the
[old-image workaround](manual/reference/03-troubleshooting.md#the-appimage-stops-with-failed-to-initialize-gtk).

## Host-provided libraries (issue #6)

`libwayland-client.so.0` is **not** bundled; the host provides it. Images up to
and including `v0.1.0-alpha.9` shipped the build runner's copy (Ubuntu 24.04,
libwayland 1.22). Newer Mesa EGL vendor libraries import symbols it lacks
(`wl_display_create_queue_with_name`, `wl_fixes_interface`), so glvnd cannot
load Mesa, no EGL display can be created, and the WebKit web process aborts
with `Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...`.
This happens under X11 and Wayland alike, before WebKit reads its renderer
variables, so the DMABUF default above does not help. The AppImage community
excludelist carries the same exclusion (AppImageCommunity/pkg2appimage#559;
upstream report tauri-apps/tauri#15665).

`tools/appimage/host-libraries.sh` is appended to the end of the pinned GTK
plugin and deletes `libwayland-client.so*` from the AppDir. linuxdeploy runs
input plugins after its dependency copy and copies nothing afterwards except
icons, desktop files and root files (checked in linuxdeploy `659c9db`
`src/main.cpp`), so the deletion reaches the packed image. The append refuses
a pinned plugin whose final command is not the known WebKit path patch. The
other `libwayland-*` libraries stay bundled: `libwebkit2gtk-4.1.so.0` links
`libwayland-server` directly, and only the client library was shown to
conflict.

`tools/appimage/check-host-libraries.sh <AppImage>` extracts an image, checks a
positive control (`usr/lib/libwebkit2gtk-4.1.so.0`) and fails if any
`libwayland-client.so*` is present. The release workflow runs it right after
bundling, and `xtask check-appimage` requires that workflow step.

**Evidence, 2026-10-12, one Arch/desktop host (NVIDIA RTX, Mesa 26.2.2,
libwayland 1.26).** The published alpha.9 AppImage (checksum verified) was
started from its extracted AppDir with isolated HOME/XDG directories. glvnd was
pinned to Mesa with
`__EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json`,
standing in for an Intel/AMD-only host. Unchanged, it printed the reporter's
exact abort line under X11 (Xwayland) and native Wayland. With only
`usr/lib/libwayland-client.so.0` removed, both stayed up for the full 12-second
window without the abort. With the NVIDIA vendor available, the unchanged image
did not abort on this host: the failure depends on the EGL vendor actually
used. A ctypes `eglGetDisplay(EGL_DEFAULT_DISPLAY)` probe gave the same
red/green split; `LD_DEBUG=libs` showed the undefined-symbol load failures.

The fix was then built by the release workflow itself: `workflow_dispatch` run
`38286151360` on `fix/issue6-appimage-egl` (commit `09229157`) was green,
including the new guard ("no libwayland-client"), `check-appimage`, and the
Xvfb and Weston startup smokes. Its artifact (88,603,128 bytes, SHA-256
`450215a1c6724407714a2456ecffddd8c9943d28af132910565a88c68afeeaa7`) still
bundles `libwayland-{cursor,egl,server}` and the display policy. On the same
host with glvnd pinned to Mesa, it printed no abort and no crash line under
X11 or Wayland, while a control rerun of the published alpha.9 printed two
aborts and `web process terminated (Crashed); recovery: Reload`. A
live-process count does not discriminate here (the recovery respawns the web
process), and no screenshot was taken. The reporter's own machine
(Kubuntu 24.04) has not been tested; details were requested in issue #6.

## Reproducible build integration

Tauri CLI **2.11.4**, the version pinned by the release workflow, runs
`build.beforeBundleCommand` before preparing the AppImage. With
`bundle.useLocalToolsDir=true`, its tools directory is the selected Cargo
`target_directory` plus `.tauri`. The preparer invokes Cargo metadata from
`apps/knx-desktop/src-tauri`, exactly as Tauri does, so Cargo config and a
relative/custom `CARGO_TARGET_DIR` are respected.

`tools/prepare_appimage_tools.py` prepares **only** that GTK deploy plugin:

1. Fetch the immutable upstream source on its first run; read at most 128 KiB
   plus one overflow-detection byte, with a 30-second request timeout.
2. Verify the complete source against its pinned SHA-256 before caching or
   executing anything. A corrupt cache fails closed, without silently repairing
   or replacing it over the network.
3. Replace exactly the forced-X11 assignment inside the quoted hook heredoc
   with the owned display policy. Refuse a missing/duplicate assignment, an
   unquoted/out-of-scope assignment or a policy that terminates the heredoc.
4. Atomically write the executable plugin. Future builds reuse the verified
   source offline and regenerate the policy, rather than trusting a previously
   patched output.

Pinned GTK deploy source:

- Revision: `dda522bce37387f1b853d9095713bfaa924c8423`.
- SHA-256: `7804c9eef13e59bf2783aad9882ef9db8f3f3f9e8d631874b1d348d550a3693f`.
- Size inspected: 14,622 bytes.

The user's global `~/.cache/tauri` is not patched. The inspected global cache
was an older plugin than this pin; consequently this is not a byte-for-byte
repack of the released alpha. Library/resource deployment stays upstream-owned
at the pinned revision; only the launcher assignment is changed relative to
that revision. Do not share one Cargo target/tool directory between independent
applications' bundlers. Tauri's other deploy tools retain its normal download
behavior; this change does not make the entire toolchain hermetic.

**Dependencies:** Python 3 standard library for preparation/tests; the existing
Tauri/linuxdeploy toolchain. Weston and Xvfb are smoke-test tools, not runtime
resources or new application dependencies. No Rust/npm dependency was added.

### Recovery from a tool-cache error

For a checksum mismatch, inspect/remove only the owned
`<target_directory>/.tauri/knxbench-gtk-upstream.sh` and retry preparation; do
not patch the global cache or weaken the checksum. To update the pin, inspect
upstream changes, update revision and digest together, re-run the mutation and
launcher tests, rebuild and smoke-test both backends. A Tauri CLI upgrade must
also re-verify the before-bundle hook and local-tool reuse mechanism.

## Save dialog start folder (24 October 2026)

**Finding (found by a native WP-11 run of the published `v0.1.0-alpha.16`
AppImage on a Wayland session with the in-process GTK 3 file chooser).** Every
native save dialog (project Save As and other in-app downloads, session log,
bus monitor capture) opened inside the image's read-only mount
(`/tmp/.mount_<id>/usr`). None of the three `tauri-plugin-dialog` save dialogs
set a folder, so GTK used the process working directory, which was
`$APPDIR/usr`. Tauri's bundler copies a prebuilt `apprun-old` `AppRun` into the
AppDir ([bundler source](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle/linux/appimage/linuxdeploy.rs));
the observed directory matches AppImageKit's `AppRun.c`, which changes into
`$APPDIR/usr` before executing the application. That the prebuilt binary is
built from exactly this source was not separately verified.

**Fix.** `apps/knx-desktop/src-tauri/src/save_directory.rs` chooses the start
folder for all three dialogs (`set_directory` only when a folder is found):

1. `OWD`, the launcher's original working directory, when `APPDIR` is set;
2. otherwise the current directory;
3. otherwise the home directory;
4. otherwise nothing, leaving GTK's default.

Each candidate must be an existing absolute directory and, after resolving
symbolic links, must not lie inside `APPDIR`. A launch from the image's own
mount therefore falls back to the home directory instead of reopening the mount.
Unit tests in that module cover these cases with temporary directories.

**What the runtime exports (verified in source, not guessed).** The type-2
runtime's `src/runtime/runtime.c` (AppImage/type2-runtime `main`, commit
`fedfad51`) exports `APPIMAGE`, `ARGV0` and `APPDIR` (the mount directory) on
the normal FUSE path. It then sets `OWD` from `getcwd()` when that call
succeeds (1024-byte buffer) and executes `$APPDIR/AppRun`. The
`--appimage-extract-and-run` path sets `APPIMAGE`, `ARGV0` and `APPDIR` (the
extraction directory) but **not** `OWD`. AppImageKit's older `runtime.c`
behaves the same way. `OWD` is therefore optional, and in extract-and-run mode
the policy falls through to the home directory: the working directory there is
also `$APPDIR/usr` and is rejected. Outside an AppImage (`APPDIR` unset) an
inherited `OWD` belongs to another program's launch and is ignored.

This change is unit-tested and gated.

**Native re-run and the frontend dialogs (24 October 2026).** A native re-run
of an AppImage built with this fix verified that the Rust-side session-log save
dialog now opens in the launch directory (`OWD`). The same run found that the
project **Save As** dialog still opened in `/tmp/.mount_<id>/usr`. That dialog,
like project Open and the other file pickers of the web UI (CSV import and
export, debug report, documentation, plugin data, project diff, selective
import, unresolved-program notice), is opened from JavaScript through
`@tauri-apps/plugin-dialog` in `apps/knx-web/src/filePicker.ts`, not in Rust,
so the fix above never reached it: it passed only a bare file name (Save) or no
folder at all (Open). The Finding's list above named project Save As among the
Rust-side dialogs; only the in-app download dialog in `download_handler.rs` is.

**Frontend fix.** The desktop shell now exposes the same policy as the app
command `default_dialog_directory` (main window only, permission
`allow-default-dialog-directory`), which returns the folder from
`save_directory.rs` or `null`; a folder that is not valid UTF-8 is reported as
`null`. `filePicker.ts` asks for it before each native dialog: Save receives
`defaultPath` = that folder joined with the default file name (`join` from
`@tauri-apps/api/path`), Open receives the folder as `defaultPath`. When the
command answers `null`, fails or the join fails, the dialog gets exactly the
arguments it had before. The plain web build's server-mount picker is
unchanged. Vitest covers the joined Save path, the Open folder, the `null` and
rejected-command fallbacks, a failed join and the untouched web build; Rust
tests cover the command's ACL grant (and its refusal for companion windows),
the `core:default` grant of `join`, and the UTF-8 conversion. This frontend
change is unit-tested and gated. A native Wayland/WebKitGTK re-run on 24 October
2026 of integration source `6f06d7e1a` verified frontend Save As and Open, and
Rust-side session-log export, all starting in the dedicated launch directory.
Save As produced a native project whose SQLite integrity check returned `ok`;
Open and log-export cancellation closed their dialogs. This is named native
workflow evidence, not Orca acceptance, every-picker coverage or a new release.

## Verification

Focused automated checks:

```sh
python3 -m unittest tools.tests.test_appimage_launcher -v
```

The suite exercises session hints, backend ordering, X11 defaults, wildcard
selection, user overrides (including an empty renderer value), repeated
sourcing, heredoc guards, verified/bounded downloads, offline repeat preparation,
corrupt-cache refusal, executable mode and Cargo metadata routing. It also
covers the host-library policy (removal, retained sibling libraries, idempotence,
missing-AppDir refusal) and the anchored append. With
`KNXBENCH_GTK_PLUGIN_SOURCE` pointing at the pinned upstream plugin, an
additional test applies both patches to the real file and syntax-checks the
result.

The release workflow runs these tests before bundling and has separate startup
checks under Xvfb and native headless Weston. Each startup check requires the
application to remain alive until timeout; neither is a claim of full UI or
accessibility acceptance on Ubuntu. Its native Wayland check removes `DISPLAY`
and both launcher override variables, so it cannot accidentally pass via X11
or the old manual workaround.

## Local acceptance receipt — 2026-10-07

**Local development build, not a published replacement.** Base
`b54cd5a5e8fcf37d5fbc89212e4c4acb20bd25e0` (includes the other owner's published
first-run guide), plus the five-file launcher source fingerprint
`5852c1e91fab7ea51fd9054a63df2d7c35658f41427644c4acde61b7f734ceed`.
This is an uncommitted working-tree build, **not** a clean-tree release build;
the embedded Git stamp identifies the base, not the patch by itself.

- Artifact: `KNXBench_0.1.0-alpha.4_amd64.AppImage`, 110,041,592 bytes.
- SHA-256: `2d88a4f282254499253571f2c7f60ae12c05980c65d5c62e2d857ca6704b936d`.
- Build: actual `cargo tauri build --bundles appimage --ci`, `NO_STRIP=1`,
  remapped build paths; the before-bundle hook ran and the package contains
  the owned policy.
- Launcher tests: **19 passed**; eight isolated source mutants caught,
  with production sources unchanged.
- `xtask`: **97 passed** (85 unit + 12 scope tests); package validator
  passed against the completed artifact, not during bundling.
- Startup bodies from the workflow: **both replayed locally and passed**,
  using the real extracted Xvfb/Weston tools. No GitHub-hosted run was dispatched.

One x86_64 Arch/desktop machine, GTK3/WebKitGTK 4.1; Xvfb 21.1.24 and
Weston 15.0.1 were extracted for tests without installing them system-wide.
Their package hashes matched the local pacman repository metadata.

| Direct-image case | Result |
| --- | --- |
| X11, no Wayland hint or override | Starts; renderer override remains unset; screenshot |
| Hyprland, `DISPLAY` unset, no manual renderer/backend setting | Starts; `wayland,x11` / renderer flag `1` observed in the actual process |
| Private headless Weston, automatic backend selection | Starts natively, no X server variable |
| Private Weston, explicit `GDK_BACKEND=wayland` | Starts; caller selection retained |
| Stale Wayland socket, working Xvfb | Falls back to X11; screenshot |
| Valid Wayland hint, explicit X11 with working Xvfb | Starts on X11; renderer override remains unset |
| Private Weston framebuffer capture | Starts and **renders the real UI**, including the parallel owner's first-run guide; screenshot inspected |
| Neither display available | GTK refuses startup, exit 101 |
| Caller insists on X11 without `DISPLAY`, despite working Wayland | GTK refuses startup, exit 101; override is not ignored |
| Unchanged released-alpha artifact, same Wayland-only control | Original forced-X11 failure reproduced, exit 101 |

The seven positive cases above executed the real `.AppImage` entrypoint with
`APPIMAGE_EXTRACT_AND_RUN=1` for FUSE-independent execution; they did not
manually unpack the image, bypass its GTK hook or inject backend settings in
the automatic cases. FUSE mounting itself is not acceptance evidence here.
They exercised frontend delivery, new/save-as/open,
fictional sample import/save-as/open: **seven HTTP 200 checks each**, in fresh
network namespaces with **only loopback** and private HOME/XDG directories.
An empty project's initial `GET /api/project` correctly returns 400. The
actual desktop process environment was inspected for just the two launcher
variables. There was no live bus, real ETS corpus or user product-database access.

A complementary check as the normal desktop UID 1000 (no network namespace)
verified a mapped **native, not Xwayland** Hyprland window, its backend/renderer
settings and its own frontend/version loopback endpoints. It made no KNX or
project-mutating API requests. Its host-desktop pixel capture was occluded,
rejected and deleted; **pixel acceptance is the private Weston capture**, not
that host image. Neither check claims full native-UI/accessibility acceptance.

### Non-green attempts and remaining limits

- The complete Python tools suite reports **56 tests, one failure**:
  `test_project_files_define_shared_memory_contract` expects the generated-memory
  marker in `CLAUDE.md`. The identical failure was reproduced in the untouched
  root checkout; no agent-instruction repair is bundled into KL-158.
- A namespace preserving UID 1000 with ambient capabilities triggered an
  sandboxed SVG-loader (`glycin`/`bwrap`) failure with this artifact on this host. The
  root-mapped network-isolated tests and normal-user window-mapping check worked.
  No production sandbox-disable flag was added to hide that test-envelope issue.
- Early harness attempts incorrectly expected 200 for an empty project,
  used a window-focus/capture path that was not valid, or inventoried a Weston
  capture under the wrong directory. They are retained as non-green attempts;
  they are not counted as accepted smoke runs.
- One package-gate attempt ran before the final bundle existed and correctly
  failed with zero artifacts. Acceptance uses the later completed-artifact gate.
- Local CI replay setup initially lacked the external `xvfb-run` on PATH and
  then exceeded the Unix-socket path limit with a deeply nested test runtime
  directory. Corrected owned wrappers/shorter scratch paths ran the **unchanged**
  workflow startup bodies successfully. These were not product-source fixes.

Other distributions, GPUs, compositors, dialogs and assistive technologies
remain unverified. The global Tauri cache and published alpha.4 tag/asset remain
unchanged. Evidence and the source/artifact manifest are kept locally as
`kl-158-appimage-launcher-20261007` under the maintainer's post-alpha evidence
store; the implementation is not yet committed, merged or published.

## Primary-source findings (2026-10-07)

- [type2-runtime `runtime.c` at `fedfad51`](https://github.com/AppImage/type2-runtime/blob/fedfad51acc2860b2916a1eef32ba4a500ff31ef/src/runtime/runtime.c): `APPIMAGE`/`ARGV0`/`APPDIR` on both launch paths, `OWD` only on the mounted path (added 2026-10-24).
- [AppImageKit `AppRun.c`](https://github.com/AppImage/AppImageKit/blob/939ff4d4c033caa0170d0cac643098b5b5664528/src/AppRun.c): changes into `$APPDIR/usr` before executing the application (added 2026-10-24).

- [Tauri CLI 2.11.4 bundle hook](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-cli/src/bundle.rs): the hook runs before bundler settings/tool preparation.
- [Tauri local-tool selection](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-cli/src/interface/mod.rs) and [Cargo metadata invocation](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-cli/src/interface/rust.rs): selected target directory, not a guessed `target/release`.
- [Tauri AppImage bundler](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle/linux/appimage/linuxdeploy.rs): project-local `.tauri`, existing-plugin reuse, GTK plugin invocation.
- [Pinned upstream GTK plugin](https://github.com/tauri-apps/linuxdeploy-plugin-gtk/blob/dda522bce37387f1b853d9095713bfaa924c8423/linuxdeploy-plugin-gtk.sh): the backend assignment is emitted inside a quoted heredoc.
- [GTK 3 runtime environment](https://docs.gtk.org/gtk3/running.html): comma-separated backend choices are tried in order; `*` admits remaining backends (supported since GTK 3.10).

These facts were inspected from the primary sources, not inferred from ETS
internals or a failed screenshot. Architectural rationale is in
[ADR-0021's launcher amendment](adr/0021-appimage-is-the-first-linux-package.md#2026-10-07-amendment-owned-display-policy-kl-158).
