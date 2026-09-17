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
