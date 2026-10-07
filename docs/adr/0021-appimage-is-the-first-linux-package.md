# ADR 0021: AppImage is the first Linux package

Date: 2026-09-17

Status: Accepted

## Context

KNXBench needs one Linux desktop delivery path for its alpha release. The Tauri desktop shell already builds the KNXBench binary and bundles the web frontend, so the package format must deliver that application without introducing a second runtime or package-specific application layer. The first package is tested only on `x86_64-unknown-linux-gnu` systems with a compatible glibc-based desktop, WebKitGTK 4.1, and GTK 3. This is a tested boundary, not a claim of universal Linux portability.

## Decision

KNXBench ships AppImage as its first Linux package format. Tauri produces one AppImage containing the desktop binary, frontend, application metadata, desktop integration assets, and the root AGPL licence. A user receives one downloadable file that can be marked executable and launched, which fits the alpha release without choosing a distribution-specific repository or package manager.

The package has no automatic update channel. Users replace the AppImage manually when a newer release is available. This manual-update cost is accepted while the project establishes build, install, and launch evidence for one delivery format.

## Alternatives

A Debian package could integrate dependency installation for Debian and Ubuntu systems, but would narrow the first package to one distribution family. Flatpak could provide a stronger runtime contract, but requires a sandbox policy, manifest, runtime, and repository ownership that are outside this alpha package. Neither format is added until a real build, install, and launch has produced evidence for its supported boundary.

## 2026-10-07 amendment: owned display policy (KL-158)

The user requested the post-alpha removal of the AppImage's forced-X11
launcher boundary. Keep Tauri's AppImage format, library/resource deployment
and existing application executable; do not add a second application launcher
or change the KNX domain to accommodate a packaging problem.

Use Tauri CLI 2.11.4's verified `beforeBundleCommand` and
`bundle.useLocalToolsDir` extension points. Prepare a checksum-pinned GTK
deploy plugin in the selected Cargo target directory, replacing only its
quoted runtime hook's backend assignment with a small owned policy. Never
modify the user's shared/global Tauri cache. A changed upstream layout or
invalid checksum fails the build instead of silently shipping forced X11.

Prefer native Wayland, with GTK-managed X11 fallback, when the session gives a
Wayland hint. Preserve explicit caller backend/renderer settings. Default the
measured WebKit DMABUF workaround only when Wayland is permitted. The
[launcher contract](../APPIMAGE_LAUNCHER.md) records primary-source evidence,
tests, package verification and remaining platform/GPU limits.

Rejected: post-build unpack/repack (a second publication path and stale-artifact
risk), patching a mutable global tool cache (non-reproducible and affects other
projects), or retrying the application process after a display failure (unnecessary
lifecycle complexity). The existing published alpha tag/asset stays untouched;
this amendment does not authorize a new release.
