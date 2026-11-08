# Release-only AppImage packaging and catalog startup

Packaging scope, 8 November 2026. No KNX protocol, project-storage format,
commissioning permission, release publication or replacement of existing assets.

## Implementation

- Automatic AppImage builds run on `v*` release tags only, not branch pushes,
  pull requests or schedules. Deliberate manual release preparation remains
  possible. The xtask workflow contract rejects additional automatic triggers.
- Build on `ubuntu-24.04`; reject a finished runtime or payload ELF requiring
  glibc above 2.39, including weak symbols and ELF version-needs entries.
- Normalize AppDir readability/searchability before packaging, explicitly make
  both launchers world-executable, then verify actual SquashFS inode modes.
- Start the root-owned extracted payload as another UID with private application
  data in a network namespace with loopback only. Require frontend readiness and
  a captured application window, not merely a surviving launcher process.
- Prefetch the three pinned deploy-tool assets through the official release API
  and verify their bytes before compilation, executable permission and cache
  promotion. Preserve failed transport attempts as failures.

See the [launcher contract](../APPIMAGE_LAUNCHER.md) and
[artifact investigation](../research/2026-11-08-appimage-catalog-packaging.md).

## Actual image acceptance

[Manual run 42471855386](https://github.com/KNXBench-Labs/KNXBench/actions/runs/42471855386)
completed successfully at source `76d697151126d11a67a73b21a8448ed9b0f3b4d3`:

- Unpublished test image `KNXBench_0.1.0-alpha.19_amd64.AppImage`, 94,452,216 bytes;
  SHA-256 `595af0863c287e8871b0f5678e0d9af2238858491b331303f93e6333b27ce1d1`.
- Both launchers owner/group 0/0, mode 0755; target UID 65534.
- Only `lo` visible in the private network namespace. Version, HTML and module
  requests returned HTTP 200; the version was `0.1.0-alpha.19+g76d6971`.
- Parent inspection of the downloaded, hash-matched evidence showed the real
  rendered workspace with New/Open/Import cards and no selected gateway.
- All 165 inspected ELF files, including the outer runtime, required at most
  glibc 2.39; no baseline violations.
- The release-publication job was skipped. This run did not publish a tag or
  release asset. Documentation-only successors retain this image's original
  source/version identity rather than relabelling it as a newer build.

Source acceptance uses the maintained change-scoped package gate. Rust scope is
`xtask`; this is not a full-workspace or private device-corpus acceptance.
The immutable-reader fixture correction changes tests only: production planner
rules and audit pins remain unchanged, including their conservative fallback.

## Boundaries

This is an ABI ceiling and an offline X11 cross-user startup check, not universal
Linux, graphics-driver or older-glibc compatibility. The image still uses host
libc and graphics services. No live bus or hardware was accessed.

The already published alpha.19 image remains unchanged and retains its old
permission/ABI defects. Catalog PR
[#11305](https://github.com/AppImage/appimage.github.io/pull/11305) can test a
corrected public asset only after a subsequent release; no `/retest`, replacement
asset or claim of a successful catalog retest is part of this source package.
