# Local Linux alpha candidate (AR17)

Record of the **local** AppImage candidate built and inspected on 2026-10-06.
It is a candidate, not a release: no tag, no upload, no claim beyond the one
machine named below. Publication stays the user's decision (`RELEASE-04`,
AR19).

## 1. Artifact

| Field | Value |
| --- | --- |
| Built revision | `6b9b681877c3fd8eb4e805f8315dfa2c2d8ea858` (clean tree, `KNX_REQUIRE_CLEAN_TREE=1`; the hash is embedded in the binary) |
| Version | `0.1.0-alpha.4` (desktop crate and bundled `knx-web` frontend) |
| File | `KNXBench_0.1.0-alpha.4_amd64.AppImage`, 107,829,752 bytes |
| SHA-256 | `4c778104a11112f233dca2b82fb03111e259127b8f7b93c4357ccbf77e480ef2` |
| Validator | `cargo run -p xtask -- check-appimage --artifact-dir …/bundle/appimage` → `AppImage ok: version 0.1.0-alpha.4`, exit 0 |
| Build command | `cargo tauri build --bundles appimage --ci` in `apps/knx-desktop` after `npm ci --prefix apps/knx-web` (the commands of `.github/workflows/linux-appimage.yml`, ADR-0021), with `NO_STRIP=1` and `APPIMAGE_EXTRACT_AND_RUN=1` as in that workflow |
| Toolchain | rustc 1.98.0, tauri-cli 2.11.4, Node 26.8.1, npm 11.19.0 |

The artifact itself is not committed. It and its file manifest are kept in
the maintainer's evidence store (`ar17-candidate-20261006`).

### Version fix found on the way

The first build failed in `check-appimage`: `knx-desktop` was still
`0.1.0-alpha.1` while `knx-web` had been `0.1.0-alpha.4` since 2026-10-04.
The validator requires both to match (ADR-0018: the desktop ships that
frontend). `6b9b6818` raises the desktop crate to `0.1.0-alpha.4`; the second
build passed.

## 2. Environment tested

One machine: x86_64, Omarchy (Arch-based), kernel 7.2.5, glibc 2.44, host
WebKitGTK 4.1 2.52.6 and GTK 3.24.52 (the AppImage brings its own copies of
both), Hyprland with Xwayland. Nothing here extends to another distribution,
architecture or desktop.

## 3. Offline smoke check

Every run: a fresh network namespace with only `lo` up, private
`XDG_DATA_HOME`/`XDG_CONFIG_HOME`/`XDG_CACHE_HOME`/`TMPDIR`, the real server
API of the running application on its loopback port, and a fictional sample
project (invented manufacturer `M-7FF0`, eight devices, fifteen group
addresses; built by a script, no real data).

| Step | X11 run (unmodified AppImage, private Xvfb 21.1.24-1, signature-checked) | Wayland run (extracted contents, see §4) |
| --- | --- | --- |
| Launch, window rendered | yes (screenshot), version `v0.1.0-alpha.4` in the status bar | yes (screenshot) |
| New project → save as `smoke.knxdb` | 200 / 200 | 200 / 200 |
| Import sample `.knxproj` → save as `sample.knxdb` | 200, 0 errors, 0 warnings / 200 | same |
| Reopen both files | `Smoke`; `Sample house`, 15 group addresses, 2 lines | same |
| Open a missing file | 400 `path does not exist` (409 `projectUnsavedChanges` over unsaved edits, otherwise 422 `projectNotOpenable` since 2026-10-06, see below) | same |
| Import a non-ZIP file | 500 `not a zip archive: invalid Zip archive: Could not find EOCD` (422 `projectNotImportable` since 2026-10-06, see below) | same |
| Non-loopback sockets after the steps | none | none |
| Application still running | yes | yes |

The same sample through the release CLI (`knx import --report-json`): 8
devices, 15 group addresses, 39 communication objects, 0 errors; the report
lists the unknown attributes it met instead of dropping them. Clear messages
for bad input hold; the status code of a malformed file is 500, which the
server tests pin today (`http_project_routes.rs`) — a classification wart,
not a lost message.

**Update 2026-10-06 (AR18 re-check N5):** the classification wart is
removed. An import that fails because of the file itself answers `422`
with kind `projectNotImportable`: not a ZIP, a refused archive, or not a
KNX project. This is pinned by
`a_file_that_is_not_an_importable_project_is_refused_as_the_callers_to_fix`.
A missing file answers `422` too, since re-check round 2 (N8).

Native UI checks (keyboard, screen reader, every dialog) belong to the UI
owner and were not claimed here.

## 4. Findings

1. **The AppImage starts only with an X server** —
   [KNOWN_LIMITATIONS §158](KNOWN_LIMITATIONS.md#158-the-appimage-starts-only-with-an-x-server).
   The bundled GTK hook sets `GDK_BACKEND=x11` unconditionally. On this
   machine Xwayland was running but its socket refused every connection
   (also for `xprop`), so the unmodified AppImage stopped with
   `Failed to initialize GTK` (exit 101). Started natively on Wayland, it
   needs `WEBKIT_DISABLE_DMABUF_RENDERER=1`, otherwise Wayland protocol error
   71 ends it. The tested workaround is in the
   [troubleshooting chapter](manual/reference/03-troubleshooting.md#the-appimage-stops-with-failed-to-initialize-gtk).
2. **Build paths in the binary.** The release binary contains about 540
   strings with the builder's home directory (`~/.cargo` and `~/.rustup`
   source paths in panic locations). Not a secret, but the account name of
   whoever builds it. A CI-built artifact would carry the runner's path
   instead. Option for AR18/AR19: build releases with
   `--remap-path-prefix`. Added to the privacy checklist in
   [ALPHA_READINESS](ALPHA_READINESS.md#deployment-and-privacy-checklist-for-ar15ar17-and-release-notes).
   *Update 2026-10-06 (AR18):* the AR18 candidate was built with
   `--remap-path-prefix` and holds no such string;
   [ALPHA_FINAL_GATES §5](ALPHA_FINAL_GATES.md#5-artifact-and-offline-start).
3. **Version drift** between desktop and web — fixed (§1).

## 5. Packaging contents

337 files, 305 MiB unpacked; the file list and every byte of the extracted
AppImage match the build's `AppDir`.

| Part | Files |
| --- | --- |
| `usr/bin/knx-desktop` | 1 (31 MiB, PIE x86-64) |
| Shared libraries (`usr/lib/*.so*`, GTK 3, WebKitGTK 4.1, JavaScriptCore, Wayland client) | 155 |
| WebKit helper processes and injected bundle (`usr/lib64/webkit2gtk-4.1`, `usr/lib/webkit2gtk-4.1`) | 6 |
| GTK/GLib/pixbuf/GIO modules and schemas | 77 |
| Frontend (`usr/lib/KNXBench/frontend`): `index.html`, 3 JS, 1 CSS, 84 fonts | 89 |
| `LICENSE`, icons, `.desktop` file, `AppRun`, GTK hook | 9 |

Searched and not found: `.knxproj`, `.knxprod`, `.knxdb`, `.zip`, `.sqlite`,
`.md`, key/certificate files, `.env` files, the string `OriginalData`, the
project's source-tree path, private-key headers and GitHub token prefixes.
No maintainer documentation is bundled (ADR-0024).

## 6. Not covered

Other distributions and desktops; any network or KNX contact (excluded by
design); AppImage update/signing; the UI owner's native checks. (This list
once named the 500 status for malformed files; since 2026-10-06 they answer
422 `projectNotImportable`, see §3.)
