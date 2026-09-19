# Linux AppImage Packaging Design

**Date:** 2026-09-17

**Status:** Proposed for implementation

## Purpose

KNXBench needs one supported Linux desktop delivery path that produces the
same Tauri application developers run from source. The first package is an
`x86_64` AppImage. It is an alpha artifact for direct download and local use,
not a distribution repository or an automatic update channel.

The package must remain a thin delivery layer. It bundles the existing
`knx-desktop` binary and built `knx-web` frontend without adding a second
backend, changing project storage, or weakening the current HTTP boundary
between frontend and `knx-server`.

## Decision and alternatives

### Selected: Tauri AppImage

Tauri already owns the native shell and supports AppImage as a native bundle
target. Using that support keeps application metadata, resources, executable,
and desktop entry in one build system. One downloadable file also gives the
project a useful first Linux artifact without choosing a particular package
manager.

The AppImage is built for `x86_64-unknown-linux-gnu`. It is expected to run on
compatible glibc based desktop systems with WebKitGTK 4.1 and GTK 3 available.
The documentation must state those runtime requirements rather than claiming
universal Linux portability.

### Rejected for this slice

- A Debian package has better dependency installation on Debian and Ubuntu,
  but narrows the first release to one distribution family.
- Flatpak provides a stronger runtime contract, but introduces a runtime,
  sandbox permissions, a manifest, and eventual repository ownership that the
  current alpha does not need.
- Building AppImage, Debian, and RPM packages together multiplies validation
  and support before one package path has been exercised by users.

## Bundle contract

`apps/knx-desktop/src-tauri/tauri.conf.json` is the source of truth for the
Linux desktop bundle:

- bundling is active and the configured target is `appimage`;
- the product name is `KNXBench` and the identifier remains
  `com.knxbench.knxbench-labs`;
- the existing `knx-web/dist` resource remains bundled as `frontend`, so the
  release executable serves the same built frontend over its loopback HTTP
  server;
- the bundle carries the canonical repository `LICENSE` file and suitable
  desktop metadata;
- icons are repository owned raster assets at the sizes Tauri needs for Linux
  desktop integration;
- the application version remains `0.1.0-alpha.1` and is checked against the
  Rust desktop package and `apps/knx-web/package.json` before release.

The package does not bundle project files, manufacturer databases, passwords,
gateway addresses, or development fixtures. User data continues to live below
Tauri's platform application data directory under `projects`.

## Release workflow

A dedicated GitHub Actions workflow owns package production. It runs on:

- `workflow_dispatch`, producing a retained Actions artifact for inspection;
- pushed tags matching `v*`, producing the same Actions artifact and attaching
  the AppImage to the corresponding GitHub release.

The job runs on Ubuntu, installs the same Tauri Linux prerequisites as the
normal CI job, installs Node.js 22 and the repository Rust toolchain, restores
dependencies with `npm ci`, and invokes Tauri with the AppImage bundle target.
It must not rely on a globally preinstalled Tauri CLI: the workflow installs a
locked Tauri CLI compatible with major version 2.

Before upload, a repository script verifies all of these conditions:

1. the desktop Cargo package, web package, and expected release version agree;
2. exactly one `.AppImage` exists in Tauri's AppImage output directory;
3. the file is nonempty and executable;
4. its filename contains the expected version;
5. the configured Tauri bundle target is AppImage and the root AGPL licence is
   configured for inclusion.

Tag builds additionally require the tag without its leading `v` to equal the
package version. A mismatch fails before publication. The workflow grants
`contents: write` only to the release job step that needs it. Manual builds do
not publish a GitHub release.

## Verification

The task closes only after a real AppImage has been produced from a clean
worktree. Static configuration checks alone are insufficient.

The proof consists of:

- focused tests for the package verification script, including multiple,
  missing, empty, non-executable, and version-mismatched artifacts;
- a production `cargo tauri build --bundles appimage`;
- inspection of the AppImage filename, executable bit, and bundled desktop
  metadata/licence;
- a bounded launch under a graphical session or Xvfb. Remaining alive until
  the timeout is success; an immediate loader, resource, or startup failure is
  failure;
- the existing frontend production build and relevant Rust checks.

The release documentation records the host distribution and versions used for
the verified build. It does not generalize that observation into support for
untested distributions.

## User documentation

The README gains an AppImage section containing exact commands to download or
locate the artifact, mark it executable, launch it, and remove it. It names the
WebKitGTK 4.1 and GTK 3 runtime requirement, the `x86_64` and glibc boundary,
the application data location, and the absence of automatic updates and
package signing in this alpha slice.

An architecture decision record explains why AppImage is the first package
format and what evidence would justify adding a native package or Flatpak.
Roadmap, implementation status, and the completion goal change only after the
real bundle and launch proof pass.

## Explicit non-goals

- automatic updates or an update server;
- package signing, attestations, or a stable release trust policy;
- ARM64 or musl builds;
- Debian, RPM, Arch, Flatpak, Snap, or app store publication;
- changing the Docker or CLI delivery paths;
- publishing an actual version tag or GitHub release during implementation.

## Stop condition

Stop when the repository can build one verified `x86_64` AppImage locally and
in the documented GitHub workflow, package validation rejects malformed output,
the bounded startup check passes, and the user facing documentation states the
tested compatibility boundary and remaining limitations accurately.
