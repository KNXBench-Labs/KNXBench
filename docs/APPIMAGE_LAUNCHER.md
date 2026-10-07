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

## Verification

Focused automated checks:

```sh
python3 -m unittest tools.tests.test_appimage_launcher -v
```

The suite exercises session hints, backend ordering, X11 defaults, wildcard
selection, user overrides (including an empty renderer value), repeated
sourcing, heredoc guards, verified/bounded downloads, offline repeat preparation,
corrupt-cache refusal, executable mode and Cargo metadata routing.

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

One x86_64 Arch/Omarchy machine, GTK3/WebKitGTK 4.1; Xvfb 21.1.24 and
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

- [Tauri CLI 2.11.4 bundle hook](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-cli/src/bundle.rs): the hook runs before bundler settings/tool preparation.
- [Tauri local-tool selection](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-cli/src/interface/mod.rs) and [Cargo metadata invocation](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-cli/src/interface/rust.rs): selected target directory, not a guessed `target/release`.
- [Tauri AppImage bundler](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle/linux/appimage/linuxdeploy.rs): project-local `.tauri`, existing-plugin reuse, GTK plugin invocation.
- [Pinned upstream GTK plugin](https://github.com/tauri-apps/linuxdeploy-plugin-gtk/blob/dda522bce37387f1b853d9095713bfaa924c8423/linuxdeploy-plugin-gtk.sh): the backend assignment is emitted inside a quoted heredoc.
- [GTK 3 runtime environment](https://docs.gtk.org/gtk3/running.html): comma-separated backend choices are tried in order; `*` admits remaining backends (supported since GTK 3.10).

These facts were inspected from the primary sources, not inferred from ETS
internals or a failed screenshot. Architectural rationale is in
[ADR-0021's launcher amendment](adr/0021-appimage-is-the-first-linux-package.md#2026-10-07-amendment-owned-display-policy-kl-158).
